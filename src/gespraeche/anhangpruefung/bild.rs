//! Bildart und Maße (B0433, ADR-0047 Frage 3 und Ergänzungen Karte #9 Gate Regel 4, „Karte #9 Nachfragen planer“
//! Punkt 1): PNG (`IHDR` als erster Chunk), JPEG (erstes über die Segmentlängen ab `SOI` erreichte `SOFn`, nie eine
//! Suche nach Markerbytes), GIF (Kopf), WebP (der Chunk, den der Kopf nennt) — Maße nur aus dieser Struktur;
//! Breite oder Höhe 0 → keine Maße; `Orientation` 5–8 aus IFD0 → angezeigte Maße (getauscht), GIF nie. Rein, auf den
//! ersten höchstens 64 KiB.

use super::exif;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bild {
    pub typ: &'static str,
    /// Angezeigte Maße — bei `Orientation` 5–8 getauscht.
    pub masse: Option<(u32, u32)>,
}

/// Die Bildart der ersten Bytes, sonst keine.
pub fn bild(kopf: &[u8]) -> Option<Bild> {
    if kopf.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some(Bild {
            typ: "image/png",
            masse: png(kopf),
        });
    }
    if kopf.starts_with(&[0xff, 0xd8, 0xff]) {
        return Some(Bild {
            typ: "image/jpeg",
            masse: jpeg(kopf),
        });
    }
    if kopf.starts_with(b"GIF87a") || kopf.starts_with(b"GIF89a") {
        return Some(Bild {
            typ: "image/gif",
            masse: gif(kopf),
        });
    }
    if kopf.len() >= 12 && &kopf[..4] == b"RIFF" && &kopf[8..12] == b"WEBP" {
        return Some(Bild {
            typ: "image/webp",
            masse: webp(kopf),
        });
    }
    None
}

fn u16_be(bytes: &[u8], stelle: usize) -> Option<u16> {
    Some(u16::from_be_bytes(
        bytes.get(stelle..stelle.checked_add(2)?)?.try_into().ok()?,
    ))
}

fn u32_be(bytes: &[u8], stelle: usize) -> Option<u32> {
    Some(u32::from_be_bytes(
        bytes.get(stelle..stelle.checked_add(4)?)?.try_into().ok()?,
    ))
}

fn u32_le(bytes: &[u8], stelle: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(stelle..stelle.checked_add(4)?)?.try_into().ok()?,
    ))
}

fn u24_le(bytes: &[u8], stelle: usize) -> Option<u32> {
    let drei = bytes.get(stelle..stelle.checked_add(3)?)?;
    Some(u32::from(drei[0]) | u32::from(drei[1]) << 8 | u32::from(drei[2]) << 16)
}

/// Maße ohne Null, bei `Orientation` 5–8 getauscht.
fn angezeigt(breite: u32, hoehe: u32, orientierung: Option<u16>) -> Option<(u32, u32)> {
    if breite == 0 || hoehe == 0 {
        return None;
    }
    Some(if exif::tauscht(orientierung) {
        (hoehe, breite)
    } else {
        (breite, hoehe)
    })
}

/// `IHDR` als erster Chunk; `eXIf` vor `IDAT` liefert die Orientierung.
fn png(kopf: &[u8]) -> Option<(u32, u32)> {
    if u32_be(kopf, 8)? != 13 || kopf.get(12..16)? != b"IHDR" {
        return None;
    }
    let breite = u32_be(kopf, 16)?;
    let hoehe = u32_be(kopf, 20)?;
    let mut stelle: usize = 8;
    let mut orientierung = None;
    while let (Some(laenge), Some(typ)) = (u32_be(kopf, stelle), kopf.get(stelle + 4..stelle + 8)) {
        let daten = stelle + 8;
        let ende = daten.checked_add(usize::try_from(laenge).ok()?)?;
        if typ == b"IDAT" || typ == b"IEND" {
            break;
        }
        if typ == b"eXIf" {
            orientierung = kopf.get(daten..ende).and_then(exif::orientierung);
            break;
        }
        stelle = ende.checked_add(4)?;
    }
    angezeigt(breite, hoehe, orientierung)
}

/// Segmente ab `SOI` über ihre Längen bis `SOS`: das erste `SOFn` liefert Höhe und Breite, `APP1` mit `Exif\0\0` die
/// Orientierung. Eine Längenkette, die nicht trägt, endet ohne Maße.
fn jpeg(kopf: &[u8]) -> Option<(u32, u32)> {
    let mut stelle = 2;
    let mut masse = None;
    let mut orientierung = None;
    loop {
        if *kopf.get(stelle)? != 0xff {
            return None;
        }
        let mut marke_an = stelle + 1;
        while *kopf.get(marke_an)? == 0xff {
            marke_an += 1;
        }
        let marke = *kopf.get(marke_an)?;
        if matches!(marke, 0xd0..=0xd7 | 0x01) {
            stelle = marke_an + 1;
            continue;
        }
        if matches!(marke, 0xda | 0xd9) {
            break;
        }
        let laenge = usize::from(u16_be(kopf, marke_an + 1)?);
        if laenge < 2 {
            return None;
        }
        let nutzlast = kopf.get(marke_an + 3..marke_an + 1 + laenge);
        if masse.is_none() && ist_sof(marke) {
            let nutzlast = nutzlast?;
            masse = Some((
                u32::from(u16_be(nutzlast, 3)?),
                u32::from(u16_be(nutzlast, 1)?),
            ));
        }
        if marke == 0xe1 && orientierung.is_none() {
            orientierung = nutzlast
                .and_then(|nutzlast| nutzlast.strip_prefix(b"Exif\0\0"))
                .and_then(exif::orientierung);
        }
        stelle = marke_an + 1 + laenge;
    }
    let (breite, hoehe) = masse?;
    angezeigt(breite, hoehe, orientierung)
}

/// `SOF0`–`SOF15` ohne `DHT` (C4), `JPG` (C8), `DAC` (CC).
fn ist_sof(marke: u8) -> bool {
    matches!(marke, 0xc0..=0xcf) && !matches!(marke, 0xc4 | 0xc8 | 0xcc)
}

fn gif(kopf: &[u8]) -> Option<(u32, u32)> {
    let breite = u32::from(u16::from_le_bytes(kopf.get(6..8)?.try_into().ok()?));
    let hoehe = u32::from(u16::from_le_bytes(kopf.get(8..10)?.try_into().ok()?));
    angezeigt(breite, hoehe, None)
}

/// Der Chunk, den der Kopf nennt (`VP8 `, `VP8L`, `VP8X`); bei `VP8X` liefert ein `EXIF`-Chunk die Orientierung.
fn webp(kopf: &[u8]) -> Option<(u32, u32)> {
    let chunk = kopf.get(12..16)?;
    let daten = 20;
    match chunk {
        b"VP8 " => {
            if kopf.get(daten + 3..daten + 6)? != [0x9d, 0x01, 0x2a] {
                return None;
            }
            let breite = u32::from(
                u16::from_le_bytes(kopf.get(daten + 6..daten + 8)?.try_into().ok()?) & 0x3fff,
            );
            let hoehe = u32::from(
                u16::from_le_bytes(kopf.get(daten + 8..daten + 10)?.try_into().ok()?) & 0x3fff,
            );
            angezeigt(breite, hoehe, None)
        }
        b"VP8L" => {
            if *kopf.get(daten)? != 0x2f {
                return None;
            }
            let bits = u32_le(kopf, daten + 1)?;
            angezeigt((bits & 0x3fff) + 1, ((bits >> 14) & 0x3fff) + 1, None)
        }
        b"VP8X" => {
            let breite = u24_le(kopf, daten + 4)?.checked_add(1)?;
            let hoehe = u24_le(kopf, daten + 7)?.checked_add(1)?;
            angezeigt(breite, hoehe, webp_orientierung(kopf))
        }
        _ => None,
    }
}

/// Der Chunk `EXIF` hinter dem Kopf, über die Chunk-Längen gesucht (gerade aufgerundet).
fn webp_orientierung(kopf: &[u8]) -> Option<u16> {
    let mut stelle: usize = 12;
    loop {
        let typ = kopf.get(stelle..stelle + 4)?;
        let laenge = usize::try_from(u32_le(kopf, stelle + 4)?).ok()?;
        let daten = stelle + 8;
        if typ == b"EXIF" {
            let exif = kopf.get(daten..daten.checked_add(laenge)?)?;
            let tiff = exif.strip_prefix(b"Exif\0\0").unwrap_or(exif);
            return exif::orientierung(tiff);
        }
        stelle = daten.checked_add(laenge)?.checked_add(laenge % 2)?;
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::super::exif::Ordnung;
    use super::super::exif::tests::tiff;
    use super::*;

    pub fn png_mit(
        breite: u32,
        hoehe: u32,
        vorher: &[(&[u8; 4], Vec<u8>)],
        nachher: &[(&[u8; 4], Vec<u8>)],
    ) -> Vec<u8> {
        let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
        let chunk = |bytes: &mut Vec<u8>, typ: &[u8; 4], daten: &[u8]| {
            bytes.extend((daten.len() as u32).to_be_bytes());
            bytes.extend(typ);
            bytes.extend(daten);
            bytes.extend([0, 0, 0, 0]);
        };
        for (typ, daten) in vorher {
            chunk(&mut bytes, typ, daten);
        }
        let mut ihdr = breite.to_be_bytes().to_vec();
        ihdr.extend(hoehe.to_be_bytes());
        ihdr.extend([8, 6, 0, 0, 0]);
        chunk(&mut bytes, b"IHDR", &ihdr);
        for (typ, daten) in nachher {
            chunk(&mut bytes, typ, daten);
        }
        chunk(&mut bytes, b"IDAT", &[1, 2, 3]);
        chunk(&mut bytes, b"IEND", &[]);
        bytes
    }

    /// Ein JPEG: `SOI`, Segmente (Marke, Nutzlast), `SOF0` mit Höhe und Breite, `SOS`, Scan-Daten, `EOI`.
    pub fn jpeg_mit(segmente: &[(u8, Vec<u8>)], breite: u16, hoehe: u16) -> Vec<u8> {
        let mut bytes = vec![0xff, 0xd8];
        let segment = |bytes: &mut Vec<u8>, marke: u8, nutzlast: &[u8]| {
            bytes.extend([0xff, marke]);
            bytes.extend(((nutzlast.len() + 2) as u16).to_be_bytes());
            bytes.extend(nutzlast);
        };
        for (marke, nutzlast) in segmente {
            segment(&mut bytes, *marke, nutzlast);
        }
        let mut sof = vec![8];
        sof.extend(hoehe.to_be_bytes());
        sof.extend(breite.to_be_bytes());
        sof.extend([3, 1, 0x22, 0, 2, 0x11, 1, 3, 0x11, 1]);
        segment(&mut bytes, 0xc0, &sof);
        segment(&mut bytes, 0xda, &[1, 1, 0, 0, 0x3f, 0]);
        bytes.extend([0x12, 0x34, 0xff, 0x00, 0x56]);
        bytes.extend([0xff, 0xd9]);
        bytes
    }

    pub fn exif_app1(orientierung: u16, ordnung: Ordnung) -> (u8, Vec<u8>) {
        let mut nutzlast = b"Exif\0\0".to_vec();
        nutzlast.extend(tiff(ordnung, &[(0x0112, 3, 1, orientierung)], None));
        (0xe1, nutzlast)
    }

    fn masse(bytes: &[u8]) -> Option<(u32, u32)> {
        bild(bytes).and_then(|bild| bild.masse)
    }

    /// Tabelle: PNG-Kopf mit `IHDR` 1280 × 800; JPEG, GIF, WebP (`VP8 `, `VP8L`, `VP8X`) je mit ihren Maßen.
    #[test]
    fn jede_bildart_mit_ihren_massen() {
        assert_eq!(
            bild(&png_mit(1280, 800, &[], &[])),
            Some(Bild {
                typ: "image/png",
                masse: Some((1280, 800))
            })
        );
        assert_eq!(
            bild(&jpeg_mit(&[], 640, 480)),
            Some(Bild {
                typ: "image/jpeg",
                masse: Some((640, 480))
            })
        );
        let mut gif = b"GIF89a".to_vec();
        gif.extend([0x40, 0x01, 0xf0, 0x00, 0, 0, 0]);
        assert_eq!(
            bild(&gif),
            Some(Bild {
                typ: "image/gif",
                masse: Some((320, 240))
            })
        );
        let webp = |chunk: &[u8; 4], daten: Vec<u8>| {
            let mut bytes = b"RIFF\0\0\0\0WEBP".to_vec();
            bytes.extend(chunk);
            bytes.extend((daten.len() as u32).to_le_bytes());
            bytes.extend(daten);
            bytes
        };
        let mut vp8 = vec![0, 0, 0, 0x9d, 0x01, 0x2a];
        vp8.extend(200u16.to_le_bytes());
        vp8.extend(100u16.to_le_bytes());
        assert_eq!(masse(&webp(b"VP8 ", vp8)), Some((200, 100)));
        let bits: u32 = 199 | (99 << 14);
        let mut vp8l = vec![0x2f];
        vp8l.extend(bits.to_le_bytes());
        assert_eq!(masse(&webp(b"VP8L", vp8l)), Some((200, 100)));
        let mut vp8x = vec![0, 0, 0, 0];
        vp8x.extend(&1279u32.to_le_bytes()[..3]);
        vp8x.extend(&799u32.to_le_bytes()[..3]);
        assert_eq!(masse(&webp(b"VP8X", vp8x)), Some((1280, 800)));
        assert_eq!(bild(b"%PDF-1.7"), None);
    }

    /// M27: `FF C0` mit falschen Maßen in einem APP1 vor dem echten `SOF0` 640 × 480 → 640 × 480; `SOF0` hinter einer
    /// falschen Segmentlänge → keine Maße; PNG ohne `IHDR` als ersten Chunk → keine Maße; 0 → Art ohne Maße;
    /// 65.535 × 65.535 wie angegeben (Mutante „Markersuche statt Längenkette“).
    #[test]
    fn masse_nur_aus_der_laengenkette() {
        let falle = (
            0xe1,
            vec![b'x', 0xff, 0xc0, 0, 17, 8, 0x27, 0x0f, 0x27, 0x0f],
        );
        assert_eq!(masse(&jpeg_mit(&[falle], 640, 480)), Some((640, 480)));
        let mut kaputt = jpeg_mit(&[(0xe0, vec![0; 10])], 640, 480);
        kaputt[4..6].copy_from_slice(&40u16.to_be_bytes());
        assert_eq!(
            bild(&kaputt),
            Some(Bild {
                typ: "image/jpeg",
                masse: None
            })
        );
        assert_eq!(
            bild(&png_mit(1280, 800, &[(b"tEXt", b"a\0b".to_vec())], &[])),
            Some(Bild {
                typ: "image/png",
                masse: None
            })
        );
        assert_eq!(
            bild(&png_mit(0, 800, &[], &[])),
            Some(Bild {
                typ: "image/png",
                masse: None
            })
        );
        assert_eq!(masse(&jpeg_mit(&[], 640, 0)), None);
        assert_eq!(
            masse(&jpeg_mit(&[], 65_535, 65_535)),
            Some((65_535, 65_535))
        );
    }

    /// Angezeigte Maße: JPEG 4000 × 3000 mit 6 und 8 → 3000 × 4000; mit 3 und 9 → 4000 × 3000; 6 nur in IFD1 → nicht;
    /// Motorola und Intel je getauscht; IFD0 hinter dem Segmentende → nicht; PNG `eXIf` 5 → 800 × 1280; GIF nie
    /// (Mutante „Tausch weg“).
    #[test]
    fn orientierung_fuenf_bis_acht_tauscht_die_masse() {
        for (wert, erwartet) in [
            (6, (3000, 4000)),
            (8, (3000, 4000)),
            (3, (4000, 3000)),
            (9, (4000, 3000)),
        ] {
            assert_eq!(
                masse(&jpeg_mit(&[exif_app1(wert, Ordnung::Motorola)], 4000, 3000)),
                Some(erwartet),
                "{wert}"
            );
        }
        assert_eq!(
            masse(&jpeg_mit(&[exif_app1(6, Ordnung::Intel)], 4000, 3000)),
            Some((3000, 4000))
        );
        let mut nur_ifd1 = b"Exif\0\0".to_vec();
        nur_ifd1.extend(tiff(
            Ordnung::Intel,
            &[(0x010f, 2, 4, 0)],
            Some((0x0112, 6)),
        ));
        assert_eq!(
            masse(&jpeg_mit(&[(0xe1, nur_ifd1)], 4000, 3000)),
            Some((4000, 3000))
        );
        let mut hinten = exif_app1(6, Ordnung::Intel);
        hinten.1[10..14].copy_from_slice(&500u32.to_le_bytes());
        assert_eq!(masse(&jpeg_mit(&[hinten], 4000, 3000)), Some((4000, 3000)));
        let mut exif_png = tiff(Ordnung::Motorola, &[(0x0112, 3, 1, 5)], None);
        let png = png_mit(1280, 800, &[], &[(b"eXIf", std::mem::take(&mut exif_png))]);
        assert_eq!(masse(&png), Some((800, 1280)));
    }

    /// Kaputte Köpfe: abgeschnitten nach 1–32 Bytes, Längen 0 und 0xFFFF → nie Panik, keine Schleife.
    #[test]
    fn abgeschnittene_koepfe_enden_ohne_panik() {
        let ganz = [
            png_mit(1280, 800, &[], &[]),
            jpeg_mit(&[exif_app1(6, Ordnung::Intel)], 640, 480),
            b"GIF89a\x40\x01\xf0\x00".to_vec(),
            b"RIFF\0\0\0\0WEBPVP8X\x0a\0\0\0\0\0\0\0\x01\0\0\x01\0\0".to_vec(),
        ];
        for bytes in &ganz {
            for laenge in [1, 4, 8, 16, 32].into_iter().chain(0..bytes.len()) {
                let _ = bild(&bytes[..laenge.min(bytes.len())]);
            }
        }
        let mut null = jpeg_mit(&[(0xe0, vec![0; 4])], 1, 1);
        null[4..6].copy_from_slice(&0u16.to_be_bytes());
        assert_eq!(masse(&null), None);
        let mut riesig = png_mit(1, 1, &[], &[(b"eXIf", vec![0; 4])]);
        riesig[33..37].copy_from_slice(&u32::MAX.to_be_bytes());
        let _ = bild(&riesig);
        let mut webp = b"RIFF\0\0\0\0WEBPVP8X\x0a\0\0\0\0\0\0\0\x01\0\0\x01\0\0".to_vec();
        webp.extend(b"ABCD\xff\xff\xff\xff");
        let _ = bild(&webp);
    }
}
