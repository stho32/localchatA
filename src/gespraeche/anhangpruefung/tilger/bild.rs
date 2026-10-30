//! Bildmetadaten tilgen (B0502, A00018 Ergänzung F4; ADR-0047 Ergänzung F4 Regeln 2–4): JPEG, PNG, WebP und GIF über
//! ihre Längenketten, gepuffert über den Positionsport; die Länge der Datei ändert sich nie.

use super::Zaehler;
use super::minimal::{minimales_exif, traegt_ort};
use super::port::{Abbruch, Ergebnis, Kette, Port, STUECK};
use crate::gespraeche::anhangpruefung::exif::orientierung;

const EXIF: &[u8] = b"Exif\0\0";
const MPF: &[u8] = b"MPF\0";

/// Erlaubnisliste JPEG (ADR-0047 Ergänzung „Karte #9 Security-Review B2“ Regel 1): je `APPn` die Kennungen, die
/// bleiben — Farbe (ICC, Adobe), HDR (MPF, ISO 21496-1), JFIF. `APP1` Exif wird minimal; jedes andere `APPn` und
/// jeder `COM` wird mit Kennung und Inhalt genullt.
const JPEG_ERLAUBT: &[(u8, &[u8])] = &[
    (0xE0, b"JFIF\0"),
    (0xE2, b"ICC_PROFILE\0"),
    (0xE2, MPF),
    (0xE2, b"urn:iso:std:iso:ts:21496:-1\0"),
    (0xEE, b"Adobe"),
];

/// So viele Bytes eines Segments reichen, um seine Kennung zu erkennen.
const KENNUNG_HOECHSTENS: u64 = 32;
const PNG_SIGNATUR: &[u8] = b"\x89PNG\r\n\x1a\n";

fn metadaten<T>(grund: &'static str) -> Ergebnis<T> {
    Err(Abbruch::Metadaten(grund))
}

/// Ein Exif-Block in der Kette: Inhalt lesen (≤ 1 MiB), Orientierung und Ort merken, minimales EXIF schreiben.
fn exif_ersetzen(
    kette: &mut Kette<'_, '_>,
    pos: u64,
    laenge: u64,
    praefix: &[u8],
    zaehler: &mut Zaehler,
) -> Ergebnis<()> {
    if laenge > STUECK as u64 {
        return metadaten("Leseziel über 1 MiB");
    }
    let alt = kette.bytes(pos, laenge as usize)?;
    let tiff = alt.get(praefix.len()..).unwrap_or_default();
    let ausrichtung = orientierung(tiff);
    zaehler.ort |= traegt_ort(tiff);
    let neu = minimales_exif(ausrichtung, laenge as usize, praefix);
    kette.schreiben(pos, &neu)
}

fn jpeg_erlaubt(marker: u8, kopf: &[u8]) -> bool {
    JPEG_ERLAUBT
        .iter()
        .any(|(erlaubt, kennung)| *erlaubt == marker && kopf.starts_with(kennung))
}

/// Wo im JPEG die Segmente enden und ob das erste Bild ein `APP2` `MPF` trägt (Inhalt ohne `MPF\0`).
pub struct JpegBild {
    pub sos: u64,
    pub mpf: Option<(u64, u64)>,
}

/// Ein JPEG-Bild in `[start, ende)` (`FF D8 FF`), gelesen nur in diesem Bereich: Segmente bis `SOS` über die Längenkette — `APP1` Exif → minimales EXIF,
/// `APPn` nach der Erlaubnisliste, jedes andere `APPn` und `COM` genullt; Nicht-`APPn` und alles ab `SOS` unverändert.
pub fn jpeg(port: &Port<'_>, start: u64, ende: u64, zaehler: &mut Zaehler) -> Ergebnis<JpegBild> {
    let mut kette = port.kette_bis(ende);
    if kette.bytes(start, 3)? != [0xFF, 0xD8, 0xFF] {
        return metadaten("JPEG ohne FF D8 FF");
    }
    let mut pos = start + 2;
    let mut mpf = None;
    let sos = loop {
        if kette.byte(pos)? != 0xFF {
            return metadaten("JPEG-Segment ohne Marker");
        }
        while kette.byte(pos + 1)? == 0xFF {
            pos += 1;
        }
        let marker = kette.byte(pos + 1)?;
        let segment = pos;
        pos += 2;
        match marker {
            0xDA => break segment,
            0xD9 => break segment,
            0x01 | 0xD0..=0xD7 => continue,
            0xD8 | 0x00 => return metadaten("JPEG-Marker an falscher Stelle"),
            _ => {}
        }
        let laenge = u64::from(u16::from_be_bytes(
            kette.bytes(pos, 2)?.try_into().unwrap_or([0, 0]),
        ));
        if laenge < 2 || !kette.enthaelt(pos, laenge) {
            return metadaten("Längen widersprechen sich");
        }
        zaehler.block()?;
        let inhalt = pos + 2;
        let n = laenge - 2;
        let kopf = kette
            .bytes(inhalt, n.min(KENNUNG_HOECHSTENS) as usize)?
            .to_vec();
        match marker {
            0xE1 if kopf.starts_with(EXIF) => exif_ersetzen(&mut kette, inhalt, n, EXIF, zaehler)?,
            0xE2 if kopf.starts_with(MPF) && mpf.is_none() => mpf = Some((inhalt + 4, n - 4)),
            0xE0..=0xEF if jpeg_erlaubt(marker, &kopf) => {}
            0xE0..=0xEF | 0xFE => kette.nullen(inhalt, n)?,
            _ => {}
        }
        pos = inhalt + n;
    };
    kette.fertig()?;
    Ok(JpegBild { sos, mpf })
}

/// Erlaubnisliste PNG (ADR-0047 Ergänzung „Karte #9 Security-Review B2“ Regel 2): Struktur, Farbe, HDR, Animation.
const PNG_ERLAUBT: &[&[u8; 4]] = &[
    b"IHDR", b"PLTE", b"IDAT", b"IEND", b"tRNS", b"cHRM", b"gAMA", b"iCCP", b"sBIT", b"sRGB",
    b"cICP", b"mDCV", b"cLLI", b"bKGD", b"hIST", b"pHYs", b"acTL", b"fcTL", b"fdAT",
];

/// PNG bis `IEND`: `eXIf` → minimales EXIF; jeder Chunk außerhalb der Erlaubnisliste → Typ `lcAx`, Inhalt genullt
/// (nie gelesen), CRC neu.
/// `IDAT`/`fdAT` zählen nicht als Block und werden über ihren Kopf übersprungen.
pub fn png(port: &Port<'_>, zaehler: &mut Zaehler) -> Ergebnis<()> {
    let mut kette = port.kette();
    if kette.bytes(0, 8)? != PNG_SIGNATUR {
        return metadaten("PNG ohne Signatur");
    }
    let mut pos = 8u64;
    loop {
        let kopf = kette.bytes(pos, 8)?.to_vec();
        let laenge = u64::from(u32::from_be_bytes([kopf[0], kopf[1], kopf[2], kopf[3]]));
        let typ: [u8; 4] = [kopf[4], kopf[5], kopf[6], kopf[7]];
        let daten = pos + 8;
        if laenge > 0x7FFF_FFFF || !port.enthaelt(daten, laenge + 4) {
            return metadaten("Längen widersprechen sich");
        }
        if &typ != b"IDAT" && &typ != b"fdAT" {
            zaehler.block()?;
        }
        match &typ {
            b"eXIf" => {
                exif_ersetzen(&mut kette, daten, laenge, b"", zaehler)?;
                let neu = kette.bytes(daten, laenge as usize)?.to_vec();
                let pruefsumme = crc32(&[&typ[..], &neu[..]], 0);
                kette.schreiben(daten + laenge, &pruefsumme.to_be_bytes())?;
            }
            b"IEND" => break,
            typ if PNG_ERLAUBT.contains(&typ) => {}
            _ => {
                kette.schreiben(pos + 4, b"lcAx")?;
                kette.nullen(daten, laenge)?;
                let pruefsumme = crc32(&[b"lcAx"], laenge);
                kette.schreiben(daten + laenge, &pruefsumme.to_be_bytes())?;
            }
        }
        pos = daten + laenge + 4;
    }
    kette.fertig()
}

/// Erlaubnisliste WebP (ADR-0047 Ergänzung „Karte #9 Security-Review B2“ Regel 3); `EXIF` wird minimal.
const WEBP_ERLAUBT: &[&[u8; 4]] = &[
    b"VP8 ", b"VP8L", b"VP8X", b"ALPH", b"ANIM", b"ANMF", b"ICCP",
];

/// Im Rahmen eines `ANMF` bleiben nach dem Rahmenkopf nur diese Chunks.
const ANMF_ERLAUBT: &[&[u8; 4]] = &[b"ALPH", b"VP8 ", b"VP8L"];

/// Länge des Rahmenkopfs eines `ANMF` (Lage, Größe, Dauer, Flaggen).
const ANMF_KOPF: u64 = 16;

/// WebP (`RIFF`…`WEBP`): `EXIF` → minimales EXIF; jeder Chunk außerhalb der Erlaubnisliste → `JUNK`, genullt; in
/// `ANMF` nach dem Rahmenkopf dasselbe mit der Liste des Rahmens (eine Ebene); wurde ein `XMP ` getilgt, wird das
/// XMP-Bit in `VP8X` gelöscht.
pub fn webp(port: &Port<'_>, zaehler: &mut Zaehler) -> Ergebnis<()> {
    let mut kette = port.kette();
    let kopf = kette.bytes(0, 12)?.to_vec();
    if &kopf[..4] != b"RIFF" || &kopf[8..12] != b"WEBP" {
        return metadaten("WebP ohne RIFF-Kopf");
    }
    let riff = u64::from(u32::from_le_bytes([kopf[4], kopf[5], kopf[6], kopf[7]])) + 8;
    if riff > port.laenge() {
        return metadaten("Längen widersprechen sich");
    }
    let mut pos = 12u64;
    let mut vp8x = None;
    let mut xmp_entfernt = false;
    while pos + 8 <= riff {
        let (typ, daten, laenge) = webp_chunk(&mut kette, pos, riff, zaehler)?;
        match &typ {
            b"VP8X" if laenge >= 1 => vp8x = Some(daten),
            b"EXIF" => {
                let praefix: &[u8] = if kette.bytes(daten, laenge.min(6) as usize)? == EXIF {
                    EXIF
                } else {
                    b""
                };
                exif_ersetzen(&mut kette, daten, laenge, praefix, zaehler)?;
            }
            b"ANMF" if laenge >= ANMF_KOPF => {
                let ende = daten + laenge;
                let mut innen = daten + ANMF_KOPF;
                while innen + 8 <= ende {
                    let (typ, inhalt, n) = webp_chunk(&mut kette, innen, ende, zaehler)?;
                    if !ANMF_ERLAUBT.contains(&&typ) {
                        webp_unkenntlich(&mut kette, innen, n)?;
                    }
                    innen = inhalt + n + (n & 1);
                }
            }
            typ if WEBP_ERLAUBT.contains(&typ) => {}
            _ => {
                xmp_entfernt |= &typ == b"XMP ";
                webp_unkenntlich(&mut kette, pos, laenge)?;
            }
        }
        pos = daten + laenge + (laenge & 1);
    }
    if let (Some(flaggen), true) = (vp8x, xmp_entfernt) {
        let alt = kette.byte(flaggen)?;
        kette.schreiben(flaggen, &[alt & !0x04])?;
    }
    kette.fertig()
}

/// Kopf eines WebP-Chunks bei `pos`, ganz vor `ende`: Typ, Anfang der Daten, Länge; zählt als Block.
fn webp_chunk(
    kette: &mut Kette<'_, '_>,
    pos: u64,
    ende: u64,
    zaehler: &mut Zaehler,
) -> Ergebnis<([u8; 4], u64, u64)> {
    let kopf = kette.bytes(pos, 8)?;
    let typ: [u8; 4] = [kopf[0], kopf[1], kopf[2], kopf[3]];
    let laenge = u64::from(u32::from_le_bytes([kopf[4], kopf[5], kopf[6], kopf[7]]));
    let daten = pos + 8;
    if !kette.enthaelt(daten, laenge) || daten + laenge > ende {
        return metadaten("Längen widersprechen sich");
    }
    zaehler.block()?;
    Ok((typ, daten, laenge))
}

/// Ein WebP-Chunk wird `JUNK` mit Nullen (nie gelesen).
fn webp_unkenntlich(kette: &mut Kette<'_, '_>, pos: u64, laenge: u64) -> Ergebnis<()> {
    kette.schreiben(pos, b"JUNK")?;
    kette.nullen(pos + 8, laenge)
}

/// Erlaubnisliste GIF (ADR-0047 Ergänzung „Karte #9 Security-Review B2“ Regel 4): Anwendungen, deren Kennung bleibt
/// (Wiederholung, Farbprofil); Grafiksteuerung `F9` und Klartext `01` bleiben immer.
const GIF_ANWENDUNGEN: &[&[u8]] = &[b"NETSCAPE2.0", b"ANIMEXTS1.0", b"ICCRGBG1012"];

/// GIF: Bildbeschreibungen, Grafiksteuerung, Klartext und erlaubte Anwendungen bleiben; jede andere Erweiterung —
/// alle Unterblöcke samt Kennung genullt, Längenbytes bleiben; Unterblöcke zählen nie als Block.
pub fn gif(port: &Port<'_>, zaehler: &mut Zaehler) -> Ergebnis<()> {
    let mut kette = port.kette();
    let kopf = kette.bytes(0, 13)?.to_vec();
    if &kopf[..6] != b"GIF87a" && &kopf[..6] != b"GIF89a" {
        return metadaten("GIF ohne Kopf");
    }
    let mut pos = 13 + farbtabelle(kopf[10]);
    loop {
        match kette.byte(pos)? {
            0x3B => break,
            0x21 => {
                zaehler.block()?;
                let marke = kette.byte(pos + 1)?;
                pos += 2;
                let nullen = match marke {
                    0xF9 | 0x01 => false,
                    0xFF => {
                        let erster = kette.byte(pos)?;
                        let kennung = kette.bytes(pos + 1, usize::from(erster))?;
                        !GIF_ANWENDUNGEN.contains(&kennung)
                    }
                    _ => true,
                };
                pos = unterbloecke(&mut kette, pos, nullen)?;
            }
            0x2C => {
                zaehler.block()?;
                let gepackt = kette.byte(pos + 9)?;
                pos += 10 + farbtabelle(gepackt) + 1;
                pos = unterbloecke(&mut kette, pos, false)?;
            }
            _ => return metadaten("GIF-Block unbekannt"),
        }
    }
    kette.fertig()
}

fn farbtabelle(gepackt: u8) -> u64 {
    if gepackt & 0x80 == 0 {
        0
    } else {
        3 << (1 + u64::from(gepackt & 0x07))
    }
}

/// Unterblöcke bis zum Nullblock, gepuffert; liefert die Stelle dahinter.
fn unterbloecke(kette: &mut Kette<'_, '_>, mut pos: u64, nullen: bool) -> Ergebnis<u64> {
    loop {
        let laenge = u64::from(kette.byte(pos)?);
        pos += 1;
        if laenge == 0 {
            return Ok(pos);
        }
        if nullen {
            kette.nullen(pos, laenge)?;
        }
        pos += laenge;
    }
}

/// CRC-32 (PNG) über die Teile, gefolgt von `nullen` Nullbytes.
pub fn crc32(teile: &[&[u8]], nullen: u64) -> u32 {
    let schritt = |crc: u32, byte: u8| {
        (0..8).fold(crc ^ u32::from(byte), |crc, _| {
            if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            }
        })
    };
    let crc = teile
        .iter()
        .flat_map(|teil| teil.iter())
        .fold(0xFFFF_FFFFu32, |crc, byte| schritt(crc, *byte));
    !(0..nullen).fold(crc, |crc, _| schritt(crc, 0))
}

#[cfg(test)]
mod tests {
    use super::super::port::attrappe::Datei;
    use super::*;
    use crate::gespraeche::anhangpruefung::exif::Ordnung;
    use crate::gespraeche::anhangpruefung::exif::tests::tiff;

    fn segment(marker: u8, inhalt: &[u8]) -> Vec<u8> {
        let mut bytes = vec![0xFF, marker];
        bytes.extend_from_slice(&((inhalt.len() + 2) as u16).to_be_bytes());
        bytes.extend_from_slice(inhalt);
        bytes
    }

    /// Ein TIFF mit `Orientation` 6, `Model`, GPS-IFD und Text dahinter.
    fn handy_tiff() -> Vec<u8> {
        let mut tiff = tiff(
            Ordnung::Motorola,
            &[
                (0x0110, 2, 4, 0x4b41),
                (0x0112, 3, 1, 6),
                (0x8825, 4, 1, 60),
            ],
            None,
        );
        tiff.resize(60, 0);
        tiff.extend_from_slice(b"\0\x01GPS-52.5200N-13.4050E 2026:10:29 12:00:00");
        tiff
    }

    /// Handy-JPEG: `APP0` JFIF, `APP1` Exif (GPS, `Model`, `DateTimeOriginal`, `Orientation` 6), `APP1` XMP, `APP13`,
    /// `COM`, `APP2` ICC, dann `SOS` und Bilddaten.
    pub fn handy_jpeg() -> Vec<u8> {
        let mut bytes = vec![0xFF, 0xD8];
        bytes.extend(segment(0xE0, b"JFIF\0\x01\x01\0\0\x01\0\x01\0\0"));
        bytes.extend(segment(0xE1, &[EXIF, &handy_tiff()[..]].concat()));
        bytes.extend(segment(
            0xE1,
            &[
                &b"http://ns.adobe.com/xap/1.0/\0"[..],
                b"<x:xmpmeta><exif:GPSLatitude>52,31N</exif:GPSLatitude>",
            ]
            .concat(),
        ));
        bytes.extend(segment(0xED, b"Photoshop 3.0\08BIM\x04\x04 Ort: Berlin"));
        bytes.extend(segment(0xFE, b"Aufgenommen in Berlin"));
        bytes.extend(segment(0xE2, b"ICC_PROFILE\0\x01\x01Farbprofil"));
        bytes.extend(segment(0xDA, b"\x01\x01\0\0?\0"));
        bytes.extend_from_slice(b"Bilddaten \xFF\0 mehr\xFF\xD9");
        bytes
    }

    fn enthaelt(heu: &[u8], nadel: &[u8]) -> bool {
        heu.windows(nadel.len()).any(|fenster| fenster == nadel)
    }

    fn tilgen(
        daten: Vec<u8>,
        leser: fn(&Port<'_>, &mut Zaehler) -> Ergebnis<()>,
    ) -> (Vec<u8>, Zaehler, Datei) {
        let datei = Datei::mit(daten);
        let mut zaehler = Zaehler::default();
        {
            let port = Port::neu(&datei).expect("Port");
            leser(&port, &mut zaehler).expect("getilgt");
        }
        (datei.inhalt(), zaehler, datei)
    }

    fn jpeg_erstes(port: &Port<'_>, zaehler: &mut Zaehler) -> Ergebnis<()> {
        jpeg(port, 0, port.laenge(), zaehler).map(|_| ())
    }

    #[test]
    fn handy_jpeg_verliert_ort_kamera_zeit_xmp_iptc_und_kommentar_behaelt_orientierung_und_farbprofil()
     {
        let original = handy_jpeg();

        let (getilgt, zaehler, _) = tilgen(original.clone(), jpeg_erstes);

        assert_eq!(getilgt.len(), original.len(), "Länge geändert");
        for spur in [
            &b"GPS"[..],
            b"Kamera",
            b"2026:10:29",
            b"GPSLatitude",
            b"Berlin",
            b"Photoshop",
            b"xap",
        ] {
            assert!(
                !enthaelt(&getilgt, spur),
                "{}",
                String::from_utf8_lossy(spur)
            );
        }
        assert!(
            enthaelt(&getilgt, b"ICC_PROFILE\0\x01\x01Farbprofil"),
            "ICC getilgt"
        );
        assert!(enthaelt(&getilgt, b"JFIF"));
        let sos = original
            .windows(2)
            .position(|paar| paar == [0xFF, 0xDA])
            .expect("SOS");
        assert_eq!(getilgt[sos..], original[sos..]);
        let exif = getilgt
            .windows(6)
            .position(|fenster| fenster == EXIF)
            .expect("Exif bleibt");
        assert_eq!(orientierung(&getilgt[exif + 6..]), Some(6));
        assert!(zaehler.ort);
        assert_eq!(zaehler.bloecke, 6);
    }

    #[test]
    fn ein_kurzes_exif_wird_nur_genullt() {
        let mut bytes = vec![0xFF, 0xD8];
        bytes.extend(segment(0xE1, b"Exif\0\0MM\0*\0\0\0\x08GPS"));
        bytes.extend(segment(0xDA, b"\x01"));

        let (getilgt, _, _) = tilgen(bytes.clone(), jpeg_erstes);

        assert_eq!(&getilgt[6..6 + 17], &[0u8; 17]);
        assert_eq!(getilgt[23..], bytes[23..], "nichts über die Segmentgrenze");
    }

    #[test]
    fn ein_jpeg_mit_widerspruechlicher_laenge_ist_nicht_tilgbar() {
        let mut bytes = vec![0xFF, 0xD8, 0xFF, 0xE1, 0xFF, 0xFF];
        bytes.extend_from_slice(b"Exif\0\0");
        let datei = Datei::mit(bytes);
        let port = Port::neu(&datei).expect("Port");

        assert!(matches!(
            jpeg(&port, 0, port.laenge(), &mut Zaehler::default()),
            Err(Abbruch::Metadaten(_))
        ));
    }

    /// GPSLatitude 52° 31′ 12″ als drei RATIONAL (Motorola) — der binäre Wert von 52.52.
    const ORT_BINAER: [u8; 24] = [
        0, 0, 0, 52, 0, 0, 0, 1, 0, 0, 0, 31, 0, 0, 0, 1, 0, 0, 0, 12, 0, 0, 0, 1,
    ];

    /// Ortsangabe als Text und binär (ADR-0047 B2 P1–P6: `GPSLatitude` 52.52, `GPSLongitude` 13.405).
    fn ort() -> Vec<u8> {
        [
            &b"GPSLatitude 52.52 +52.5200+013.4050/ GPSLongitude 13.405 "[..],
            &ORT_BINAER[..],
        ]
        .concat()
    }

    /// Kein Ort in `bytes`: weder `GPS`, noch „52.52“, „+52.5200“, noch der binäre Wert.
    fn ohne_ort(bytes: &[u8]) -> bool {
        !enthaelt(bytes, b"GPS")
            && !enthaelt(bytes, b"52.52")
            && !enthaelt(bytes, b"+52.5200")
            && !enthaelt(bytes, &ORT_BINAER)
    }

    fn jpeg_aus(segmente: &[Vec<u8>]) -> Vec<u8> {
        [
            vec![0xFF, 0xD8],
            segmente.concat(),
            segment(0xC0, b"\x08\0\x10\0\x20\x01\x01\x11\0"),
            segment(0xDA, b"\x01\x01\0\0?\0"),
            b"Bilddaten \xFF\0 mehr\xFF\xD9".to_vec(),
        ]
        .concat()
    }

    /// JUMBF mit C2PA-Manifest und Zusicherung `stds.exif`, darin der Ort.
    fn jumbf_c2pa() -> Vec<u8> {
        [
            &b"JP\0\x01\0\0\0\x01"[..],
            b"\0\0\0\x80jumb\0\0\0\x1Ejumdc2pa\0\x11\0\x10\x80\0\xAA\08\x9Bq\x03c2pa\0",
            b"\0\0\0\x40jumb\0\0\0\x1Djumdjson\0\x11\0\x10\x80\0\xAA\0stds.exif\0",
            &ort(),
        ]
        .concat()
    }

    /// Vorschau-JPEG mit Exif-GPS (für `APP0` `JFXX`).
    fn vorschau_mit_gps() -> Vec<u8> {
        let mut jfxx = b"JFXX\0\x10".to_vec();
        jfxx.extend(jpeg_aus(&[segment(0xE1, &[EXIF, &ort()[..]].concat())]));
        jfxx
    }

    /// ADR-0047 B2 P1, P2, P7 (JPEG-Teil): `APP11` JUMBF/C2PA, `APP3`, `APP0` `JFXX` mit Vorschau, `APP2` FlashPix,
    /// `APP15` → Kennung und Inhalt genullt, Marker und Längen bleiben, ab `SOS` byte-gleich.
    #[test]
    fn jedes_nicht_erlaubte_app_segment_verliert_kennung_und_inhalt() {
        let segmente = vec![
            segment(0xE0, b"JFIF\0\x01\x01\0\0\x01\0\x01\0\0"),
            segment(0xEB, &jumbf_c2pa()),
            segment(0xE3, &[&b"META\0\0"[..], &ort()[..]].concat()),
            segment(0xE0, &vorschau_mit_gps()),
            segment(0xE2, &[&b"FPXR\0"[..], &ort()[..]].concat()),
            segment(0xEF, &[&b"Hersteller\0"[..], &ort()[..]].concat()),
        ];
        let original = jpeg_aus(&segmente);

        let (getilgt, zaehler, _) = tilgen(original.clone(), jpeg_erstes);

        assert_eq!(getilgt.len(), original.len());
        assert!(ohne_ort(&getilgt), "Ort bleibt");
        let mut pos = 2 + segmente[0].len();
        for segment in &segmente[1..] {
            assert_eq!(getilgt[pos..pos + 4], segment[..4], "Marker und Länge");
            assert!(
                getilgt[pos + 4..pos + segment.len()]
                    .iter()
                    .all(|b| *b == 0),
                "Kennung und Inhalt genullt bei FF {:02X}",
                segment[1]
            );
            pos += segment.len();
        }
        assert_eq!(
            getilgt[..2 + segmente[0].len()],
            original[..2 + segmente[0].len()]
        );
        let sos = original.len() - 26;
        assert_eq!(getilgt[sos..], original[sos..]);
        assert_eq!(zaehler.bloecke, 7);
    }

    /// ADR-0047 B2 P8 (JPEG-Teil): ICC über drei `APP2`, `APP14` Adobe, Orientierung 6, ISO-21496-1-`APP2` und
    /// `APP2` `MPF` bleiben byte-gleich, `SOF` und alles ab `SOS` auch.
    #[test]
    fn farbe_hdr_und_orientierung_bleiben_byte_gleich() {
        let bleiben = vec![
            segment(0xE0, b"JFIF\0\x01\x01\0\0\x01\0\x01\0\0"),
            segment(0xE2, b"ICC_PROFILE\0\x01\x03Teil eins"),
            segment(0xE2, b"ICC_PROFILE\0\x02\x03Teil zwei"),
            segment(0xE2, b"ICC_PROFILE\0\x03\x03Teil drei"),
            segment(0xEE, b"Adobe\0\x64\0\0\0\0\x02"),
            segment(0xE2, b"urn:iso:std:iso:ts:21496:-1\0\0\0\0\0"),
            segment(0xE2, b"MPF\0MM\0*\0\0\0\x08\0\0"),
        ];
        let exif = segment(0xE1, &[EXIF, &handy_tiff()[..]].concat());
        let original = jpeg_aus(&[bleiben.clone(), vec![exif]].concat());

        let (getilgt, _, _) = tilgen(original.clone(), jpeg_erstes);

        let erlaubt = 2 + bleiben.concat().len();
        assert_eq!(getilgt[..erlaubt], original[..erlaubt]);
        let exif_ab = getilgt.windows(6).position(|f| f == EXIF).expect("Exif");
        assert_eq!(orientierung(&getilgt[exif_ab + 6..]), Some(6));
        let sof = original.len() - 26 - 13;
        assert_eq!(getilgt[sof..], original[sof..]);
    }

    fn chunk(typ: &[u8], daten: &[u8]) -> Vec<u8> {
        let mut bytes = (daten.len() as u32).to_be_bytes().to_vec();
        bytes.extend_from_slice(typ);
        bytes.extend_from_slice(daten);
        bytes.extend_from_slice(&crc32(&[typ, daten], 0).to_be_bytes());
        bytes
    }

    pub fn handy_png(idat: usize) -> Vec<u8> {
        let mut bytes = PNG_SIGNATUR.to_vec();
        bytes.extend(chunk(b"IHDR", &[0, 0, 0, 2, 0, 0, 0, 1, 8, 2, 0, 0, 0]));
        bytes.extend(chunk(b"iCCP", b"Profil\0\0Farbe"));
        bytes.extend(chunk(b"eXIf", &handy_tiff()));
        bytes.extend(chunk(
            b"iTXt",
            b"XML:com.adobe.xmp\0\0\0\0\0<exif:GPSLatitude>52</exif:GPSLatitude>",
        ));
        bytes.extend(chunk(b"tIME", &[7, 234, 10, 29, 12, 0, 0]));
        for _ in 0..idat {
            bytes.extend(chunk(b"IDAT", b"x"));
        }
        bytes.extend(chunk(b"IEND", b""));
        bytes
    }

    #[test]
    fn png_verliert_exif_und_texte_crc_stimmt_farbprofil_bleibt() {
        let original = handy_png(1);

        let (getilgt, zaehler, _) = tilgen(original.clone(), png);

        assert_eq!(getilgt.len(), original.len());
        assert!(
            !enthaelt(&getilgt, b"GPS")
                && !enthaelt(&getilgt, b"Kamera")
                && !enthaelt(&getilgt, b"xmp")
        );
        assert!(enthaelt(&getilgt, b"Profil\0\0Farbe"));
        assert!(!enthaelt(&getilgt, b"tIME") && !enthaelt(&getilgt, b"iTXt"));
        let mut pos = 8;
        while pos < getilgt.len() {
            let laenge =
                u32::from_be_bytes(getilgt[pos..pos + 4].try_into().expect("Länge")) as usize;
            let typ = &getilgt[pos + 4..pos + 8];
            let daten = &getilgt[pos + 8..pos + 8 + laenge];
            let crc = u32::from_be_bytes(
                getilgt[pos + 8 + laenge..pos + 12 + laenge]
                    .try_into()
                    .expect("CRC"),
            );
            assert_eq!(
                crc,
                crc32(&[typ, daten], 0),
                "{}",
                String::from_utf8_lossy(typ)
            );
            if typ == b"eXIf" {
                assert_eq!(orientierung(daten), Some(6));
            }
            pos += 12 + laenge;
        }
        assert!(zaehler.ort);
    }

    /// Die Chunks eines PNG als `(Typ, ganzer Chunk)`.
    fn chunks(bytes: &[u8]) -> Vec<([u8; 4], Vec<u8>)> {
        let mut pos = 8;
        let mut alle = Vec::new();
        while pos + 12 <= bytes.len() {
            let laenge =
                u32::from_be_bytes(bytes[pos..pos + 4].try_into().expect("Länge")) as usize;
            let typ: [u8; 4] = bytes[pos + 4..pos + 8].try_into().expect("Typ");
            alle.push((typ, bytes[pos..pos + 12 + laenge].to_vec()));
            pos += 12 + laenge;
        }
        alle
    }

    /// ADR-0047 B2 P3 (B0642): Ort nur in `caBX`, dazu `tEXt`, privater `iDOT`, unbekannter kritischer `ABCD` → je
    /// `lcAx`, Inhalt genullt, CRC stimmt; `IDAT` byte-gleich.
    #[test]
    fn jeder_nicht_erlaubte_png_chunk_wird_lcax_mit_nullen() {
        let ihdr = chunk(b"IHDR", &[0, 0, 0, 2, 0, 0, 0, 1, 8, 2, 0, 0, 0]);
        let idat = chunk(b"IDAT", b"Bilddaten");
        let original = [
            PNG_SIGNATUR.to_vec(),
            ihdr.clone(),
            chunk(b"caBX", &[&b"jumb c2pa stds.exif "[..], &ort()].concat()),
            chunk(b"iDOT", &ort()),
            chunk(b"ABCD", &ort()),
            idat.clone(),
            chunk(b"IEND", b""),
        ]
        .concat();

        let (getilgt, _, _) = tilgen(original.clone(), png);

        assert_eq!(getilgt.len(), original.len());
        assert!(ohne_ort(&getilgt), "Ort bleibt");
        let typen: Vec<[u8; 4]> = chunks(&getilgt).iter().map(|(typ, _)| *typ).collect();
        assert_eq!(
            typen,
            vec![*b"IHDR", *b"lcAx", *b"lcAx", *b"lcAx", *b"IDAT", *b"IEND"]
        );
        for (typ, ganz) in chunks(&getilgt) {
            let daten = &ganz[8..ganz.len() - 4];
            let crc = u32::from_be_bytes(ganz[ganz.len() - 4..].try_into().expect("CRC"));
            assert_eq!(crc, crc32(&[&typ, daten], 0));
            if &typ == b"lcAx" {
                assert!(daten.iter().all(|b| *b == 0));
            }
        }
        assert!(enthaelt(&getilgt, &idat) && enthaelt(&getilgt, &ihdr));
    }

    /// ADR-0047 B2 P8 (PNG-Teil): `iCCP`, `cICP`, `mDCV`, `cLLI`, `pHYs`, APNG (`acTL`/`fcTL`/`fdAT`) und die übrigen
    /// erlaubten Chunks bleiben byte-gleich.
    #[test]
    fn erlaubte_png_chunks_bleiben_byte_gleich() {
        let erlaubt: Vec<Vec<u8>> = [
            &b"IHDR"[..],
            b"PLTE",
            b"tRNS",
            b"cHRM",
            b"gAMA",
            b"iCCP",
            b"sBIT",
            b"sRGB",
            b"cICP",
            b"mDCV",
            b"cLLI",
            b"bKGD",
            b"hIST",
            b"pHYs",
            b"acTL",
            b"fcTL",
            b"IDAT",
            b"fdAT",
        ]
        .iter()
        .map(|typ| chunk(typ, &[&typ[..], b" Inhalt"].concat()))
        .collect();
        let original = [PNG_SIGNATUR.to_vec(), erlaubt.concat(), chunk(b"IEND", b"")].concat();

        let (getilgt, _, _) = tilgen(original.clone(), png);

        assert_eq!(getilgt, original);
    }

    #[test]
    fn png_mit_200_000_idat_ist_tilgbar_weil_idat_nicht_zaehlt() {
        let (_, zaehler, _) = tilgen(handy_png(200_000), png);

        assert!(zaehler.bloecke < 10, "{}", zaehler.bloecke);
    }

    #[test]
    fn webp_verliert_exif_und_xmp_samt_flagge() {
        let mut inhalt = b"WEBP".to_vec();
        let teil = |typ: &[u8], daten: &[u8]| {
            let mut bytes = typ.to_vec();
            bytes.extend_from_slice(&(daten.len() as u32).to_le_bytes());
            bytes.extend_from_slice(daten);
            if daten.len() % 2 == 1 {
                bytes.push(0);
            }
            bytes
        };
        inhalt.extend(teil(b"VP8X", &[0x0C, 0, 0, 0, 1, 0, 0, 1, 0, 0]));
        inhalt.extend(teil(b"VP8 ", b"Bilddaten"));
        inhalt.extend(teil(b"EXIF", &handy_tiff()));
        inhalt.extend(teil(b"XMP ", b"<exif:GPSLatitude>52</exif:GPSLatitude>"));
        let mut original = b"RIFF".to_vec();
        original.extend_from_slice(&(inhalt.len() as u32).to_le_bytes());
        original.extend(inhalt);

        let (getilgt, zaehler, _) = tilgen(original.clone(), webp);

        assert_eq!(getilgt.len(), original.len());
        assert!(!enthaelt(&getilgt, b"GPS") && !enthaelt(&getilgt, b"XMP "));
        assert!(enthaelt(&getilgt, b"JUNK") && enthaelt(&getilgt, b"Bilddaten"));
        assert_eq!(getilgt[20], 0x08, "XMP-Bit gelöscht, EXIF-Bit bleibt");
        let exif = getilgt.windows(4).position(|f| f == b"EXIF").expect("EXIF");
        assert_eq!(orientierung(&getilgt[exif + 8..]), Some(6));
        assert_eq!(zaehler.bloecke, 4);
    }

    fn webp_teil(typ: &[u8], daten: &[u8]) -> Vec<u8> {
        let mut bytes = typ.to_vec();
        bytes.extend_from_slice(&(daten.len() as u32).to_le_bytes());
        bytes.extend_from_slice(daten);
        if daten.len() % 2 == 1 {
            bytes.push(0);
        }
        bytes
    }

    fn riff(teile: &[Vec<u8>]) -> Vec<u8> {
        let inhalt = [&b"WEBP"[..], &teile.concat()].concat();
        [&b"RIFF"[..], &(inhalt.len() as u32).to_le_bytes(), &inhalt].concat()
    }

    /// ADR-0047 B2 P7 und P8 (WebP-Teil) (B0643): `C2PA`, unbekanntes `ABCD` und im Rahmen eines `ANMF` ein
    /// unbekanntes `XYZW` → `JUNK`, Inhalt genullt; `VP8X`, `ICCP`, `ANIM`, Rahmenkopf, `ALPH`, `VP8 ` byte-gleich.
    #[test]
    fn webp_behaelt_nur_erlaubte_chunks_auch_im_animationsrahmen() {
        let vp8x = webp_teil(b"VP8X", &[0x32, 0, 0, 0, 1, 0, 0, 1, 0, 0]);
        let iccp = webp_teil(b"ICCP", b"Farbprofil");
        let anim = webp_teil(b"ANIM", &[0, 0, 0, 0, 0, 0]);
        let rahmenkopf = [0u8; 16];
        let alph = webp_teil(b"ALPH", b"Alpha");
        let vp8 = webp_teil(b"VP8 ", b"Bilddaten");
        let fremd_im_rahmen = webp_teil(b"XYZW", &ort());
        let anmf = webp_teil(
            b"ANMF",
            &[&rahmenkopf[..], &alph, &fremd_im_rahmen, &vp8].concat(),
        );
        let c2pa = webp_teil(b"C2PA", &ort());
        let abcd = webp_teil(b"ABCD", &ort());
        let original = riff(&[
            vp8x.clone(),
            iccp.clone(),
            anim.clone(),
            anmf,
            c2pa.clone(),
            abcd.clone(),
        ]);

        let (getilgt, _, _) = tilgen(original.clone(), webp);

        assert_eq!(getilgt.len(), original.len());
        assert!(ohne_ort(&getilgt), "Ort bleibt");
        for bleibt in [&vp8x, &iccp, &anim, &alph, &vp8] {
            assert!(
                enthaelt(&getilgt, bleibt),
                "{}",
                String::from_utf8_lossy(&bleibt[..4])
            );
        }
        assert!(
            !enthaelt(&getilgt, b"C2PA")
                && !enthaelt(&getilgt, b"ABCD")
                && !enthaelt(&getilgt, b"XYZW")
        );
        let junk: Vec<usize> = getilgt
            .windows(4)
            .enumerate()
            .filter(|(_, f)| f == b"JUNK")
            .map(|(stelle, _)| stelle)
            .collect();
        assert_eq!(junk.len(), 3);
        for stelle in junk {
            let laenge =
                u32::from_le_bytes(getilgt[stelle + 4..stelle + 8].try_into().expect("Länge"));
            assert!(
                getilgt[stelle + 8..stelle + 8 + laenge as usize]
                    .iter()
                    .all(|b| *b == 0)
            );
        }
    }

    /// ADR-0047 B2 P7 und P8 (GIF-Teil) (B0643): Anwendung `ABCDEFGH123` mit Ort → alle Unterblöcke samt Kennung
    /// Nullen, Längenbytes bleiben; `NETSCAPE2.0`, `ICCRGBG1012`, `ANIMEXTS1.0`, Grafiksteuerung und Klartext
    /// bleiben byte-gleich.
    #[test]
    fn gif_behaelt_nur_erlaubte_erweiterungen() {
        let kopf = b"GIF89a\x01\0\x01\0\x80\0\0\0\0\0\xFF\xFF\xFF".to_vec();
        let bleiben = [
            &b"\x21\xFF\x0bNETSCAPE2.0\x03\x01\0\0\0"[..],
            b"\x21\xFF\x0bANIMEXTS1.0\x03\x01\0\0\0",
            b"\x21\xFF\x0bICCRGBG1012\x05Farbe\0",
            b"\x21\xF9\x04\x04\x0a\0\0\0",
            b"\x21\x01\x0c\0\0\0\0\x01\0\x01\0\x08\x08\0\0\x04Text\0",
        ]
        .concat();
        let mut fremd = b"\x21\xFF\x0bABCDEFGH123".to_vec();
        fremd.push(ort().len() as u8);
        fremd.extend(ort());
        fremd.push(0);
        let bild = b"\x2C\0\0\0\0\x01\0\x01\0\0\x02\x01\x44\0\x3B".to_vec();
        let original = [kopf.clone(), bleiben.clone(), fremd.clone(), bild.clone()].concat();

        let (getilgt, _, _) = tilgen(original.clone(), super::gif);

        assert!(ohne_ort(&getilgt) && !enthaelt(&getilgt, b"ABCDEFGH"));
        let bis = kopf.len() + bleiben.len();
        assert_eq!(getilgt[..bis], original[..bis]);
        let mut erwartet = vec![0x21, 0xFF, 0x0b];
        erwartet.extend([0; 11]);
        erwartet.push(ort().len() as u8);
        erwartet.extend(vec![0; ort().len()]);
        erwartet.push(0);
        assert_eq!(getilgt[bis..bis + fremd.len()], erwartet[..]);
        assert_eq!(getilgt[bis + fremd.len()..], bild[..]);
    }

    /// GIF mit 300.000 Unterblöcken in den Bilddaten und einem Kommentar im letzten Bild.
    fn gif(unterbloecke: usize) -> Vec<u8> {
        let mut bytes = b"GIF89a\x01\0\x01\0\x80\0\0".to_vec();
        bytes.extend_from_slice(&[0, 0, 0, 255, 255, 255]);
        bytes.extend_from_slice(b"\x21\xFF\x0bXMP DataXMP");
        bytes.extend_from_slice(b"\x05GPS52\x03ort\0");
        bytes.extend_from_slice(b"\x2C\0\0\0\0\x01\0\x01\0\0\x02");
        for _ in 0..unterbloecke {
            bytes.extend_from_slice(b"\x01\x44");
        }
        bytes.push(0);
        bytes.extend_from_slice(b"\x21\xFE\x06Berlin\x02Ja\0");
        bytes.extend_from_slice(b"\x2C\0\0\0\0\x01\0\x01\0\0\x02\x01\x44\0");
        bytes.push(0x3B);
        bytes
    }

    #[test]
    fn gif_kommentar_und_xmp_genullt_unterbloecke_zaehlen_nicht_und_werden_gepuffert_gelesen() {
        let original = gif(300_000);

        let (getilgt, zaehler, datei) = tilgen(original.clone(), super::gif);

        assert_eq!(getilgt.len(), original.len());
        assert!(
            !enthaelt(&getilgt, b"Berlin")
                && !enthaelt(&getilgt, b"GPS52")
                && !enthaelt(&getilgt, b"ort")
        );
        assert!(
            enthaelt(&getilgt, b"\x21\xFE\x06\0\0\0\0\0\0\x02\0\0\0"),
            "Längenbytes bleiben"
        );
        assert!(zaehler.bloecke < 100_000, "{}", zaehler.bloecke);
        assert!(datei.anzahl_lesungen() < 300, "{}", datei.anzahl_lesungen());
        assert!(datei.groesste_lesung() <= STUECK);
    }

    /// Befund #51 B2 (d), R00029 „Robust“: ein animiertes GIF von 64 MiB (≈ 263.000 Unterblöcke zu 255 Bytes) mit
    /// Kommentar im letzten Frame → getilgt, kein `metadaten`, ≤ 2 s; die Bilddaten bleiben byte-gleich.
    #[test]
    fn ein_gif_von_64_mib_mit_kommentar_im_letzten_frame_ist_in_2_s_getilgt() {
        let mut original = b"GIF89a\x01\0\x01\0\x80\0\0".to_vec();
        original.extend_from_slice(&[0, 0, 0, 255, 255, 255]);
        original.extend_from_slice(b"\x2C\0\0\0\0\x01\0\x01\0\0\x02");
        let unterblock = [&[255u8][..], &[0x44; 255][..]].concat();
        while original.len() < 64 * 1_048_576 {
            original.extend_from_slice(&unterblock);
        }
        original.push(0);
        let daten_ende = original.len();
        original.extend_from_slice(b"\x21\xFE\x06Berlin\0");
        original.extend_from_slice(b"\x2C\0\0\0\0\x01\0\x01\0\0\x02\x01\x44\0");
        original.push(0x3B);

        let beginn = std::time::Instant::now();
        let (getilgt, zaehler, _) = tilgen(original.clone(), super::gif);
        let dauer = beginn.elapsed();

        assert!(dauer.as_secs_f64() <= 2.0, "{dauer:?}");
        assert!(zaehler.bloecke < 10, "{}", zaehler.bloecke);
        assert!(!enthaelt(&getilgt[daten_ende..], b"Berlin"));
        assert!(
            getilgt[..daten_ende] == original[..daten_ende],
            "Bilddaten gleich"
        );
    }

    /// Befund #51 B2 (d), R00029 „Robust“: Metadaten **nach** `IEND` sind Daten, kein Block — sie bleiben
    /// byte-gleich, die Chunks davor werden getilgt.
    #[test]
    fn metadaten_nach_iend_bleiben_byte_gleich() {
        let png_teil = handy_png(1);
        let dahinter = [
            chunk(b"tEXt", b"GPS\0 52.5200 13.4050"),
            chunk(b"eXIf", &handy_tiff()),
        ]
        .concat();
        let original = [png_teil.clone(), dahinter.clone()].concat();

        let (getilgt, _, _) = tilgen(original, png);

        assert_eq!(&getilgt[png_teil.len()..], &dahinter[..]);
        assert!(!enthaelt(&getilgt[..png_teil.len()], b"GPS"));
    }

    #[test]
    fn mehr_als_100_000_gedeutete_bloecke_sind_nicht_tilgbar() {
        let mut bytes = b"GIF89a\x01\0\x01\0\0\0\0".to_vec();
        for _ in 0..100_001 {
            bytes.extend_from_slice(b"\x21\xF9\0");
        }
        bytes.push(0x3B);
        let datei = Datei::mit(bytes);
        let port = Port::neu(&datei).expect("Port");

        assert!(matches!(
            super::gif(&port, &mut Zaehler::default()),
            Err(Abbruch::Metadaten(_))
        ));
    }
}
