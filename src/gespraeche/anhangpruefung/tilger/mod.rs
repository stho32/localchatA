//! Ortsangaben beim Hochladen tilgen (F0115, A00018 Ergänzung F4, ADR-0047 Ergänzung „Karte #9 Betreiber F4“): eigene
//! Leser nach Struktur, an Ort und Stelle bei gleicher Länge, über den Positionsport.

pub mod bild;
pub mod isobmff;
pub mod minimal;
pub mod mpf;
pub mod port;

use crate::gespraeche::anhangablage::Teildatei;
use port::{Abbruch, Ergebnis, Port};

/// Höchstens so viele gedeutete Blöcke je Datei (Unterblöcke eines GIF und `IDAT`/`fdAT` zählen nie).
pub const HOECHSTENS_BLOECKE: u64 = 100_000;

/// Was der Tilger gedeutet hat: Zahl der Blöcke und ob ein Ort (GPS-IFD, Ortsbox) dabei war.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Zaehler {
    pub bloecke: u64,
    pub ort: bool,
}

/// Was getilgt wurde — fürs Protokoll, nie mit Inhalt; `arbeit` sind die gelesenen und geschriebenen Bytes
/// (ADR-0047 B1 Regel 4, Kontrollen gegen das Budget).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Getilgt {
    pub art: &'static str,
    pub bloecke: u64,
    pub ort: bool,
    pub arbeit: u64,
}

/// Tilgt eine fertige Teildatei nach ihrer Struktur (nicht nach der Art des Erkenners): JPEG samt MPF-Bildern, PNG,
/// WebP, GIF, ISOBMFF; alles andere bleibt unverändert (`None`). Die Länge wird danach geprüft.
pub fn tilgen(datei: &dyn Teildatei) -> Ergebnis<Option<Getilgt>> {
    let port = Port::neu(datei)?;
    let anfang = port.lesen(0, port.laenge().min(16))?;
    let mut zaehler = Zaehler::default();
    let art = if anfang.starts_with(&[0xFF, 0xD8, 0xFF]) {
        mpf::jpeg_mit_mpf(&port, &mut zaehler)?;
        "jpeg"
    } else if anfang.starts_with(b"\x89PNG\r\n\x1a\n") {
        bild::png(&port, &mut zaehler)?;
        "png"
    } else if anfang.starts_with(b"RIFF") && anfang.get(8..12) == Some(b"WEBP") {
        bild::webp(&port, &mut zaehler)?;
        "webp"
    } else if anfang.starts_with(b"GIF87a") || anfang.starts_with(b"GIF89a") {
        bild::gif(&port, &mut zaehler)?;
        "gif"
    } else if isobmff::ist_isobmff(&anfang) {
        isobmff::isobmff(&port, &mut zaehler)?;
        "isobmff"
    } else {
        return Ok(None);
    };
    if datei.laenge()? != port.laenge() {
        return Err(Abbruch::Platte(std::io::Error::other(
            "Länge beim Tilgen geändert",
        )));
    }
    Ok(Some(Getilgt {
        art,
        bloecke: zaehler.bloecke,
        ort: zaehler.ort,
        arbeit: port.verbraucht(),
    }))
}

impl Zaehler {
    /// Ein gedeuteter Block mehr; über 100.000 → `metadaten`.
    pub fn block(&mut self) -> Ergebnis<()> {
        self.bloecke += 1;
        if self.bloecke > HOECHSTENS_BLOECKE {
            return Err(Abbruch::Metadaten("über 100.000 Blöcke"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::port::attrappe::Datei;
    use super::*;

    #[test]
    fn andere_formate_bleiben_unveraendert() {
        for inhalt in [
            &b"%PDF-1.7 GPS"[..],
            b"\x1aE\xdf\xa3webm",
            b"OggS",
            b"ID3",
            b"Text mit GPS",
            b"",
        ] {
            let datei = Datei::mit(inhalt.to_vec());
            assert_eq!(tilgen(&datei).expect("kein Fehler"), None);
            assert_eq!(datei.inhalt(), inhalt);
        }
    }

    /// B0640: die Arbeit im Ergebnis ist die Summe aller Lese- und Schreiblängen am Port.
    #[test]
    fn die_arbeit_ist_die_summe_aller_zugriffe() {
        let datei = Datei::mit(super::bild::tests::handy_jpeg());

        let getilgt = tilgen(&datei).expect("getilgt").expect("JPEG");

        let gelesen: usize = datei.lesungen.lock().expect("Lesungen").iter().sum();
        let geschrieben: usize = datei
            .schreibungen
            .lock()
            .expect("Schreibungen")
            .iter()
            .sum();
        assert!(getilgt.arbeit > 0);
        assert_eq!(getilgt.arbeit, (gelesen + geschrieben) as u64);
    }

    #[test]
    fn ein_beschaedigtes_jpeg_ist_nicht_tilgbar() {
        let datei = Datei::mit(b"\xFF\xD8\xFF\xE1\xFF\xFFExif".to_vec());
        assert!(matches!(tilgen(&datei), Err(Abbruch::Metadaten(_))));
    }

    fn ergebnis(daten: Vec<u8>) -> (Ergebnis<Option<Getilgt>>, std::time::Duration, usize) {
        let laenge = daten.len();
        let datei = Datei::mit(daten);
        let beginn = std::time::Instant::now();
        let ergebnis = tilgen(&datei);
        assert_eq!(datei.inhalt().len(), laenge, "die Länge ändert sich nie");
        (ergebnis, beginn.elapsed(), datei.groesste_lesung())
    }

    fn kiste(typ: &[u8], inhalt: &[u8]) -> Vec<u8> {
        [&((inhalt.len() + 8) as u32).to_be_bytes()[..], typ, inhalt].concat()
    }

    /// Robust (M48, M49): kaputte Längen, Riesenlängen, tiefe und viele Blöcke, falsche CRC → keine Panik, keine
    /// Schleife, nichts außerhalb der Datei, Ergebnis getilgt oder `metadaten`, je Fall ≤ 2 s.
    #[test]
    fn robust_gegen_kaputte_strukturen() {
        let ftyp = kiste(b"ftyp", b"isom\0\0\x02\0isommp41");
        let mut tief = Vec::new();
        for _ in 0..10_000 {
            tief = kiste(b"udta", &tief);
        }
        let viele_chunks: Vec<u8> = [
            &b"\x89PNG\r\n\x1a\n"[..],
            &[0u8, 0, 0, 0, b'x', b'x', b'x', b'x', 0, 0, 0, 0].repeat(1_000_000),
        ]
        .concat();
        let faelle: Vec<(&str, Vec<u8>)> = vec![
            ("Segmentlänge 0", vec![0xFF, 0xD8, 0xFF, 0xE1, 0, 0]),
            ("Segmentlänge 1", vec![0xFF, 0xD8, 0xFF, 0xE1, 0, 1, 9]),
            (
                "Segment über das Ende",
                vec![0xFF, 0xD8, 0xFF, 0xE1, 0x40, 0, 1, 2, 3],
            ),
            (
                "Box size 0",
                [ftyp.clone(), vec![0, 0, 0, 0, b'm', b'o', b'o', b'v']].concat(),
            ),
            (
                "Box size 1 mit 2^64 − 1",
                [
                    ftyp.clone(),
                    vec![0, 0, 0, 1, b'm', b'o', b'o', b'v'],
                    u64::MAX.to_be_bytes().to_vec(),
                ]
                .concat(),
            ),
            (
                "Box size 8 mit Kindern",
                [
                    ftyp.clone(),
                    vec![
                        0, 0, 0, 8, b'm', b'o', b'o', b'v', 0, 0, 0, 9, b'u', b'd', b't', b'a',
                    ],
                ]
                .concat(),
            ),
            (
                "Verschachtelung 10.000",
                [ftyp.clone(), kiste(b"moov", &tief)].concat(),
            ),
            ("10⁶ leere Chunks", viele_chunks),
            (
                "falsche CRC",
                [
                    &b"\x89PNG\r\n\x1a\n"[..],
                    &[0, 0, 0, 1, b't', b'E', b'X', b't', b'x', 1, 2, 3, 4],
                    &[0, 0, 0, 0, b'I', b'E', b'N', b'D', 0, 0, 0, 0],
                ]
                .concat(),
            ),
        ];
        for (fall, daten) in faelle {
            let (ergebnis, dauer, lesung) = ergebnis(daten);
            assert!(
                dauer < std::time::Duration::from_secs(2),
                "{fall}: {dauer:?}"
            );
            assert!(lesung <= port::STUECK, "{fall}");
            assert!(
                matches!(ergebnis, Ok(_) | Err(Abbruch::Metadaten(_))),
                "{fall}: {ergebnis:?}"
            );
        }
    }

    /// Metadaten nach `SOS` bzw. nach `IEND` und ein Polyglot mit zweitem `ftyp` sind Daten, kein Block.
    #[test]
    fn nach_dem_ende_ist_nichts_metadatum() {
        let nach_sos = [
            &[0xFF, 0xD8, 0xFF, 0xDA, 0, 2][..],
            b"\xFF\xE1\x00\x10Exif\0\0GPS 52 13",
        ]
        .concat();
        let (getilgt, _, _) = ergebnis(nach_sos.clone());
        assert!(getilgt.is_ok());
        let datei = Datei::mit(nach_sos.clone());
        tilgen(&datei).expect("getilgt");
        assert_eq!(datei.inhalt(), nach_sos, "nach SOS unverändert");
        let zweites = [
            kiste(b"ftyp", b"isom\0\0\x02\0isom"),
            kiste(b"mdat", b"Medien"),
            kiste(b"ftyp", b"heic"),
            kiste(b"meta", b"\0\0\0\0"),
        ]
        .concat();
        let datei = Datei::mit(zweites.clone());
        tilgen(&datei).expect("getilgt");
        assert_eq!(datei.inhalt(), zweites);
    }

    /// Präparierte Metadaten (M47): GPS nur in XMP, nur in der MakerNote, nur in IPTC, zwei `APP1`-Exif, Extended XMP
    /// über mehrere Segmente, komprimiertes PNG `iTXt` → danach keine Ortsangabe in den Bytes, Bilddaten gleich.
    #[test]
    fn praeparierte_metadaten() {
        let segment = |marke: u8, inhalt: &[u8]| {
            [
                &[0xFF, marke][..],
                &((inhalt.len() + 2) as u16).to_be_bytes(),
                inhalt,
            ]
            .concat()
        };
        let jpeg = |segmente: Vec<Vec<u8>>| {
            [
                vec![0xFF, 0xD8],
                segmente.concat(),
                segment(0xDA, b"\x01"),
                b"Bilddaten".to_vec(),
            ]
            .concat()
        };
        let exif = |text: &[u8]| [&b"Exif\0\0MM\0*\0\0\0\x08\0\0\0\0\0\0"[..], text].concat();
        let ext = b"http://ns.adobe.com/xmp/extension/\0";
        let faelle = vec![
            jpeg(vec![segment(
                0xE1,
                b"http://ns.adobe.com/xap/1.0/\0<GPSLatitude>52</GPSLatitude>",
            )]),
            jpeg(vec![segment(0xE1, &exif(b"MakerNote GPS 52.52 13.40"))]),
            jpeg(vec![segment(0xED, b"Photoshop 3.0\08BIM GPS 52.52")]),
            jpeg(vec![
                segment(0xE1, &exif(b"GPS 52.52")),
                segment(0xE1, &exif(b"GPS 13.40")),
            ]),
            jpeg(vec![
                segment(0xE1, &[&ext[..], b"Teil1 GPS"].concat()),
                segment(0xE1, &[&ext[..], b"Teil2 GPS"].concat()),
            ]),
        ];
        for daten in faelle {
            let datei = Datei::mit(daten.clone());
            tilgen(&datei).expect("getilgt");
            let inhalt = datei.inhalt();
            assert!(
                !inhalt.windows(3).any(|f| f == b"GPS"),
                "{}",
                String::from_utf8_lossy(&daten)
            );
            assert!(inhalt.ends_with(b"Bilddaten"));
        }
        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        for (typ, daten) in [
            (&b"zTXt"[..], &b"Location\0\0x\x9cGPS"[..]),
            (b"iTXt", b"XML:com.adobe.xmp\0\x01\0\0\0x\x9cGPS"),
            (b"IEND", b""),
        ] {
            png.extend_from_slice(&(daten.len() as u32).to_be_bytes());
            png.extend_from_slice(typ);
            png.extend_from_slice(daten);
            png.extend_from_slice(&bild::crc32(&[typ, daten], 0).to_be_bytes());
        }
        let datei = Datei::mit(png);
        tilgen(&datei).expect("getilgt");
        assert!(!datei.inhalt().windows(3).any(|f| f == b"GPS"));
    }
}
