//! Erkenner (A00018, B0436, ADR-0047 Frage 3): Stücke des `PUT` → Art. Signaturen der Tabelle von oben nach unten
//! aus den ersten höchstens 64 KiB, dann die Text-Prüfung über die ganze Datei, sonst `application/octet-stream`.
//! Name und `Content-Type` des Senders zählen nie. Rein.

use super::art::{Art, DATEI, inline};
use super::bild::bild;
use super::medien::medien;
use super::text::Textpruefer;

/// Für Signaturen werden höchstens so viele Bytes gelesen.
pub const KOPF_HOECHSTENS: usize = 64 * 1024;

pub const TEXT: &str = "text/plain; charset=utf-8";

#[derive(Debug, Clone, Default)]
pub struct Erkenner {
    kopf: Vec<u8>,
    text: Textpruefer,
    bytes: u64,
}

impl Erkenner {
    pub fn neu() -> Erkenner {
        Erkenner::default()
    }

    pub fn stueck(&mut self, bytes: &[u8]) {
        let platz = KOPF_HOECHSTENS - self.kopf.len();
        self.kopf
            .extend_from_slice(&bytes[..bytes.len().min(platz)]);
        self.text.stueck(bytes);
        self.bytes = self.bytes.saturating_add(bytes.len() as u64);
    }

    pub fn ergebnis(&self) -> Art {
        if let Some(bild) = bild(&self.kopf) {
            let (breite, hoehe) = bild.masse.unzip();
            return Art {
                typ: bild.typ,
                inline: inline(bild.typ, self.bytes),
                breite,
                hoehe,
            };
        }
        if let Some(typ) = medien(&self.kopf) {
            return Art {
                typ,
                inline: inline(typ, self.bytes),
                breite: None,
                hoehe: None,
            };
        }
        if self.text.ist_text() {
            return Art {
                typ: TEXT,
                inline: inline(TEXT, self.bytes),
                breite: None,
                hoehe: None,
            };
        }
        Art::datei()
    }
}

/// Die Arten der Tabelle — gegen sie prüft das Laden die Metazeile (M17).
pub const ARTEN: [&str; 16] = [
    "image/png",
    "image/jpeg",
    "image/gif",
    "image/webp",
    super::medien::AVIF_TYP,
    super::medien::HEIC_TYP,
    super::medien::HEIF_TYP,
    "application/pdf",
    "video/mp4",
    "audio/mp4",
    "video/webm",
    "audio/ogg",
    "video/ogg",
    "audio/mpeg",
    TEXT,
    DATEI,
];

#[cfg(test)]
mod tests {
    use super::super::bild::tests::{jpeg_mit, png_mit};
    use super::super::medien::tests::{ebml, ftyp, mp3_rahmen, ogg_mit};
    use super::*;

    fn art(bytes: &[u8]) -> Art {
        art_in(bytes, 65_536)
    }

    fn art_in(bytes: &[u8], stueck: usize) -> Art {
        let mut erkenner = Erkenner::neu();
        for teil in bytes.chunks(stueck.max(1)) {
            erkenner.stueck(teil);
        }
        if bytes.is_empty() {
            erkenner.stueck(&[]);
        }
        erkenner.ergebnis()
    }

    /// ADR-0047 „Erster Unit-Test“ und R00029 „Tabelle“: jede Zeile der Tabelle samt Disposition.
    #[test]
    fn die_tabelle_der_arten() {
        let png = art(&png_mit(1280, 800, &[], &[]));
        assert_eq!(
            (png.typ, png.inline, png.breite, png.hoehe),
            ("image/png", true, Some(1280), Some(800))
        );
        assert_eq!(art(&jpeg_mit(&[], 640, 480)).typ, "image/jpeg");
        assert_eq!(art(b"%PDF-1.7\n...").typ, "application/pdf");
        assert_eq!(art(&ftyp(b"isom")).typ, "video/mp4");
        assert_eq!(art(&ftyp(b"M4A ")).typ, "audio/mp4");
        let qt = art(&ftyp(b"qt  "));
        assert_eq!((qt.typ, qt.inline), (DATEI, false));
        assert_eq!(art(&ebml(b"webm")).typ, "video/webm");
        assert_eq!(art(&ebml(b"matroska")).typ, DATEI);
        assert_eq!(art(&ogg_mit(b"OpusHead")).typ, "audio/ogg");
        assert_eq!(art(&ogg_mit(b"\x80theora")).typ, "video/ogg");
        assert_eq!(art(b"ID3\x04\0\0\0\0\0\0").typ, "audio/mpeg");
        assert_eq!(
            art(&mp3_rahmen()).typ,
            DATEI,
            "ein Rahmenkopf allein ist kein MP3, NUL → kein Text"
        );
        let svg = art(b"<svg xmlns='http://www.w3.org/2000/svg'><script>alert(1)</script></svg>");
        assert_eq!((svg.typ, svg.inline, svg.breite), (TEXT, true, None));
        let mit_nul = art(b"<svg><script>\0</script></svg>");
        assert_eq!((mit_nul.typ, mit_nul.inline), (DATEI, false));
    }

    /// `ftyp` mit Hauptmarke, Version 0 und den kompatiblen Marken.
    fn ftyp_mit(haupt: &[u8; 4], kompatibel: &[&[u8; 4]]) -> Vec<u8> {
        let mut bytes = ((16 + 4 * kompatibel.len()) as u32).to_be_bytes().to_vec();
        bytes.extend(b"ftyp");
        bytes.extend(haupt);
        bytes.extend([0, 0, 0, 0]);
        for marke in kompatibel {
            bytes.extend(*marke);
        }
        bytes
    }

    /// ADR-0047 Ergänzung „Karte #9 Nacharbeit F3 und F4“, Erste Tests (Unit Erkenner, wörtlich): alle Marken der
    /// `ftyp`-Box, zuerst AVIF, dann HEIC, dann HEIF, dann MP4 wie bisher; `image/avif` inline, `image/heic` und
    /// `image/heif` attachment (Mutanten „nur Hauptmarke“, „`mif1` vor `avif` geprüft“, „HEIC inline“).
    #[test]
    fn die_marken_der_ftyp_box() {
        for (haupt, kompatibel, typ, inline) in [
            (b"avif", &[b"mif1", b"miaf"][..], "image/avif", true),
            (b"mif1", &[b"avif"][..], "image/avif", true),
            (b"avis", &[][..], "image/avif", true),
            (b"heic", &[b"mif1"][..], "image/heic", false),
            (b"mif1", &[][..], "image/heif", false),
            (b"msf1", &[b"hevc"][..], "image/heic", false),
            (b"isom", &[b"heic"][..], "image/heic", false),
            (b"isom", &[b"mp42"][..], "video/mp4", true),
            (b"mp42", &[b"isom"][..], "video/mp4", true),
            (b"M4A ", &[b"isom"][..], "audio/mp4", true),
            (b"qt  ", &[][..], DATEI, false),
        ] {
            let art = art(&ftyp_mit(haupt, kompatibel));
            assert_eq!(
                (art.typ, art.inline),
                (typ, inline),
                "{} {kompatibel:?}",
                String::from_utf8_lossy(haupt)
            );
        }
        let mut ungerade = ftyp_mit(b"isom", &[b"heic"]);
        ungerade.extend([b'a', b'v']);
        ungerade[3] += 2;
        assert_eq!(
            art(&ungerade).typ,
            "image/heic",
            "Rest hinter der letzten vollen Marke übergangen"
        );
        assert!(
            ARTEN.contains(&"image/avif")
                && ARTEN.contains(&"image/heic")
                && ARTEN.contains(&"image/heif")
        );
    }

    /// Text über 16 MiB → `attachment` (Betreiber F3; Mutante „Text über 16 MiB inline“).
    #[test]
    fn text_ueber_16_mib_ist_ein_download() {
        let mut genau = Erkenner::neu();
        let mut darueber = Erkenner::neu();
        let block = vec![b'a'; 1_048_576];
        for _ in 0..16 {
            genau.stueck(&block);
            darueber.stueck(&block);
        }
        darueber.stueck(b"a");
        assert_eq!(
            (genau.ergebnis().typ, genau.ergebnis().inline),
            (TEXT, true)
        );
        assert_eq!(
            (darueber.ergebnis().typ, darueber.ergebnis().inline),
            (TEXT, false)
        );
    }

    /// Stückelung zu 1, 3 und 65.536 Bytes → gleiches Urteil, auch für Bilder (der Kopf sammelt über die Stücke).
    #[test]
    fn das_urteil_haengt_nicht_an_der_stueckelung() {
        let bytes = [
            png_mit(1280, 800, &[], &[]),
            "Grüße 😀".repeat(50).into_bytes(),
            ebml(b"webm"),
            vec![0xc3; 3],
        ];
        for datei in &bytes {
            let ganz = art(datei);
            for groesse in [1, 3, 65_536] {
                assert_eq!(art_in(datei, groesse), ganz, "{groesse}");
            }
        }
    }

    /// Höchstens 64 KiB für Signaturen: eine Signatur hinter 64 KiB zählt nicht (der Rest ist Text-Prüfung).
    #[test]
    fn signaturen_nur_aus_den_ersten_64_kib() {
        let mut spaet = vec![b' '; KOPF_HOECHSTENS];
        spaet.extend(b"%PDF-1.7");
        assert_eq!(art(&spaet).typ, TEXT);
        let mut erkenner = Erkenner::neu();
        erkenner.stueck(&vec![0u8; 3 * KOPF_HOECHSTENS]);
        assert!(erkenner.kopf.len() <= KOPF_HOECHSTENS);
    }

    /// Name und Sender zählen nie: der Erkenner kennt keinen Namen — HTML bleibt Text, PNG bleibt PNG (M29).
    #[test]
    fn der_inhalt_entscheidet() {
        assert_eq!(art(b"<!doctype html><script>x</script>").typ, TEXT);
        assert_eq!(art(&png_mit(1, 1, &[], &[])).typ, "image/png");
        let mut polyglot = png_mit(1, 1, &[], &[]);
        polyglot.extend(b"<html><script>alert(1)</script></html>");
        assert_eq!(art(&polyglot).typ, "image/png");
        let mut id3_html = b"ID3\x03".to_vec();
        id3_html.extend(b"<html><script>alert(1)</script>");
        assert_eq!(art(&id3_html).typ, "audio/mpeg");
    }
}
