//! Bildmetadaten tilgen (B0502, A00018 Ergänzung F4; ADR-0047 Ergänzung F4 Regeln 2–4): JPEG, PNG, WebP und GIF über
//! ihre Längenketten, gepuffert über den Positionsport; die Länge der Datei ändert sich nie.

use super::Zaehler;
use super::minimal::{minimales_exif, traegt_ort};
use super::port::{Abbruch, Ergebnis, Kette, Port, STUECK};
use crate::gespraeche::anhangpruefung::exif::orientierung;

const EXIF: &[u8] = b"Exif\0\0";
const XMP: &[u8] = b"http://ns.adobe.com/xap/1.0/\0";
const XMP_EXTENDED: &[u8] = b"http://ns.adobe.com/xmp/extension/\0";
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

/// Wo im JPEG die Segmente enden und ob das erste Bild ein `APP2` `MPF` trägt (Inhalt ohne `MPF\0`).
pub struct JpegBild {
    pub sos: u64,
    pub mpf: Option<(u64, u64)>,
}

/// Ein JPEG-Bild ab `start` (`FF D8 FF`): Segmente bis `SOS` über die Längenkette — `APP1` Exif → minimales EXIF,
/// `APP1` XMP (auch Extended), `APP13` und `COM` genullt; `APP0`, `APP2`, `APP14` und alles ab `SOS` unverändert.
pub fn jpeg(port: &Port<'_>, start: u64, zaehler: &mut Zaehler) -> Ergebnis<JpegBild> {
    let mut kette = port.kette();
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
        if laenge < 2 || !port.enthaelt(pos, laenge) {
            return metadaten("Längen widersprechen sich");
        }
        zaehler.block()?;
        let inhalt = pos + 2;
        let n = laenge - 2;
        let kopf = kette
            .bytes(inhalt, n.min(XMP_EXTENDED.len() as u64) as usize)?
            .to_vec();
        match marker {
            0xE1 if kopf.starts_with(EXIF) => exif_ersetzen(&mut kette, inhalt, n, EXIF, zaehler)?,
            0xE1 if kopf.starts_with(XMP) || kopf.starts_with(XMP_EXTENDED) => {
                kette.nullen(inhalt, n)?
            }
            0xED | 0xFE => kette.nullen(inhalt, n)?,
            0xE2 if kopf.starts_with(b"MPF\0") && mpf.is_none() => mpf = Some((inhalt + 4, n - 4)),
            _ => {}
        }
        pos = inhalt + n;
    };
    kette.fertig()?;
    Ok(JpegBild { sos, mpf })
}

/// PNG bis `IEND`: `eXIf` → minimales EXIF; `tEXt`, `zTXt`, `iTXt`, `tIME` → Typ `lcAx`, Inhalt genullt; CRC neu.
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
            b"tEXt" | b"zTXt" | b"iTXt" | b"tIME" => {
                kette.schreiben(pos + 4, b"lcAx")?;
                kette.nullen(daten, laenge)?;
                let pruefsumme = crc32(&[b"lcAx"], laenge);
                kette.schreiben(daten + laenge, &pruefsumme.to_be_bytes())?;
            }
            b"IEND" => break,
            _ => {}
        }
        pos = daten + laenge + 4;
    }
    kette.fertig()
}

/// WebP (`RIFF`…`WEBP`): `EXIF` → minimales EXIF; `XMP ` → `JUNK`, genullt, XMP-Bit in `VP8X` gelöscht.
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
        let kopf = kette.bytes(pos, 8)?.to_vec();
        let typ: [u8; 4] = [kopf[0], kopf[1], kopf[2], kopf[3]];
        let laenge = u64::from(u32::from_le_bytes([kopf[4], kopf[5], kopf[6], kopf[7]]));
        let daten = pos + 8;
        if !port.enthaelt(daten, laenge) || daten + laenge > riff {
            return metadaten("Längen widersprechen sich");
        }
        zaehler.block()?;
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
            b"XMP " => {
                kette.schreiben(pos, b"JUNK")?;
                kette.nullen(daten, laenge)?;
                xmp_entfernt = true;
            }
            _ => {}
        }
        pos = daten + laenge + (laenge & 1);
    }
    if let (Some(flaggen), true) = (vp8x, xmp_entfernt) {
        let alt = kette.byte(flaggen)?;
        kette.schreiben(flaggen, &[alt & !0x04])?;
    }
    kette.fertig()
}

/// GIF: Kommentar- und `XMP DataXMP`-Erweiterung — ihre Unterblöcke genullt, Längenbytes bleiben; Unterblöcke zählen
/// nie als Block.
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
                    0xFE => true,
                    0xFF => {
                        let erster = u64::from(kette.byte(pos)?);
                        let kennung = kette.bytes(pos + 1, erster as usize)?;
                        let xmp = kennung == b"XMP DataXMP";
                        pos += 1 + erster;
                        xmp
                    }
                    _ => false,
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
                XMP,
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
        jpeg(port, 0, zaehler).map(|_| ())
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
            jpeg(&port, 0, &mut Zaehler::default()),
            Err(Abbruch::Metadaten(_))
        ));
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
            chunk(b"tEXt", b"GPS\052.5200 13.4050"),
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
