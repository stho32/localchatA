//! Testdateien für Anhänge, im Test gebaut (R00029 „Testdateien entstehen im Test“): Köpfe der Arten-Tabelle, ein
//! Hochformatfoto mit EXIF und Ort, ein Video mit Ortsbox.

/// PNG mit `IHDR` (CRC wie im Format) und einem `IDAT`.
pub fn png(breite: u32, hoehe: u32) -> Vec<u8> {
    let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = breite.to_be_bytes().to_vec();
    ihdr.extend(hoehe.to_be_bytes());
    ihdr.extend([8, 2, 0, 0, 0]);
    png_chunk(&mut bytes, b"IHDR", &ihdr);
    png_chunk(&mut bytes, b"IDAT", &[0x78, 0x9c, 0x63, 0, 0, 0, 1, 0, 1]);
    png_chunk(&mut bytes, b"IEND", &[]);
    bytes
}

pub fn png_chunk(bytes: &mut Vec<u8>, typ: &[u8; 4], daten: &[u8]) {
    bytes.extend((daten.len() as u32).to_be_bytes());
    bytes.extend(typ);
    bytes.extend(daten);
    let mut pruef = typ.to_vec();
    pruef.extend(daten);
    bytes.extend(crc32(&pruef).to_be_bytes());
}

/// CRC-32 wie PNG (Polynom 0xEDB88320).
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// TIFF (Motorola) mit IFD0-Einträgen (Tag, Typ, Zahl, Wert bzw. Versatz) und angehängten Daten.
pub fn tiff(ifd0: &[(u16, u16, u32, u32)], anhang: &[u8]) -> Vec<u8> {
    let mut bytes = b"MM\0*".to_vec();
    bytes.extend(8u32.to_be_bytes());
    bytes.extend((ifd0.len() as u16).to_be_bytes());
    for (tag, typ, zahl, wert) in ifd0 {
        bytes.extend(tag.to_be_bytes());
        bytes.extend(typ.to_be_bytes());
        bytes.extend(zahl.to_be_bytes());
        if *typ == 3 && *zahl == 1 {
            bytes.extend((*wert as u16).to_be_bytes());
            bytes.extend([0, 0]);
        } else {
            bytes.extend(wert.to_be_bytes());
        }
    }
    bytes.extend(0u32.to_be_bytes());
    bytes.extend(anhang);
    bytes
}

/// JPEG `SOI`, Segmente, `SOF0` (Breite × Höhe), `SOS`, Scan-Daten, `EOI`.
pub fn jpeg(segmente: &[(u8, Vec<u8>)], breite: u16, hoehe: u16) -> Vec<u8> {
    let mut bytes = vec![0xff, 0xd8];
    for (marke, nutzlast) in segmente {
        jpeg_segment(&mut bytes, *marke, nutzlast);
    }
    let mut sof = vec![8];
    sof.extend(hoehe.to_be_bytes());
    sof.extend(breite.to_be_bytes());
    sof.extend([1, 1, 0x11, 0]);
    jpeg_segment(&mut bytes, 0xc0, &sof);
    jpeg_segment(&mut bytes, 0xda, &[1, 1, 0, 0, 0x3f, 0]);
    bytes.extend(b"BILDDATEN\xff\x00BILDDATEN");
    bytes.extend([0xff, 0xd9]);
    bytes
}

pub fn jpeg_segment(bytes: &mut Vec<u8>, marke: u8, nutzlast: &[u8]) {
    bytes.extend([0xff, marke]);
    bytes.extend(((nutzlast.len() + 2) as u16).to_be_bytes());
    bytes.extend(nutzlast);
}

/// Das Hochformatfoto aus R00029: JPEG 4000 × 3000 mit EXIF (`Orientation` 6, `Model`, `DateTimeOriginal`,
/// GPS-IFD mit 52.5200), dazu XMP mit `exif:GPSLatitude`.
pub fn hochkant_jpg() -> Vec<u8> {
    let modell = b"Telefon X1\0";
    let zeit = b"2026:10:29 07:12:00\0";
    let gps_ifd: Vec<u8> = {
        let mut ifd = 1u16.to_be_bytes().to_vec();
        ifd.extend(2u16.to_be_bytes());
        ifd.extend(2u16.to_be_bytes());
        ifd.extend(8u32.to_be_bytes());
        ifd.extend(0u32.to_be_bytes());
        ifd.extend(0u32.to_be_bytes());
        ifd.extend(b"GPS52.5200N13.4050E");
        ifd
    };
    let eintraege = 4u32;
    let daten_ab = 8 + 2 + 12 * eintraege + 4;
    let modell_ab = daten_ab;
    let zeit_ab = modell_ab + modell.len() as u32;
    let gps_ab = zeit_ab + zeit.len() as u32;
    let mut daten = modell.to_vec();
    daten.extend(zeit);
    daten.extend(&gps_ifd);
    let exif = tiff(
        &[
            (0x0110, 2, modell.len() as u32, modell_ab),
            (0x0112, 3, 1, 6),
            (0x8825, 4, 1, gps_ab),
            (0x9003, 2, zeit.len() as u32, zeit_ab),
        ],
        &daten,
    );
    let mut app1 = b"Exif\0\0".to_vec();
    app1.extend(exif);
    let mut xmp = b"http://ns.adobe.com/xap/1.0/\0".to_vec();
    xmp.extend(br#"<x:xmpmeta><rdf:Description exif:GPSLatitude="52,31.2N" exif:GPSLongitude="13,24.3E"/></x:xmpmeta>"#);
    jpeg(
        &[
            (0xe0, b"JFIF\0\x01\x01\0\0\x01\0\x01\0\0".to_vec()),
            (0xe1, app1),
            (0xe1, xmp),
        ],
        4000,
        3000,
    )
}

pub fn gif(breite: u16, hoehe: u16) -> Vec<u8> {
    let mut bytes = b"GIF89a".to_vec();
    bytes.extend(breite.to_le_bytes());
    bytes.extend(hoehe.to_le_bytes());
    bytes.extend([0, 0, 0]);
    bytes.extend([0x2c, 0, 0, 0, 0, 1, 0, 1, 0, 0, 2, 2, 0x44, 1, 0, 0x3b]);
    bytes
}

pub fn webp_vp8x(breite: u32, hoehe: u32) -> Vec<u8> {
    let mut daten = vec![0, 0, 0, 0];
    daten.extend(&(breite - 1).to_le_bytes()[..3]);
    daten.extend(&(hoehe - 1).to_le_bytes()[..3]);
    let mut bytes = b"RIFF".to_vec();
    bytes.extend((4 + 8 + daten.len() as u32).to_le_bytes());
    bytes.extend(b"WEBPVP8X");
    bytes.extend((daten.len() as u32).to_le_bytes());
    bytes.extend(daten);
    bytes
}

pub fn pdf() -> Vec<u8> {
    b"%PDF-1.7\n1 0 obj << /Type /Catalog >> endobj\ntrailer << /Root 1 0 R >>\n%%EOF\n".to_vec()
}

/// ISOBMFF: `ftyp` mit Marke, `moov` mit `udta` (optional mit `©xyz`), `mdat`.
pub fn mp4(marke: &[u8; 4], ort: Option<&[u8]>) -> Vec<u8> {
    let boxe = |typ: &[u8], inhalt: &[u8]| {
        let mut bytes = ((8 + inhalt.len()) as u32).to_be_bytes().to_vec();
        bytes.extend(typ);
        bytes.extend(inhalt);
        bytes
    };
    let mut ftyp = marke.to_vec();
    ftyp.extend([0, 0, 2, 0]);
    ftyp.extend(b"isommp41");
    let mut udta = Vec::new();
    if let Some(ort) = ort {
        let mut wert = (ort.len() as u16).to_be_bytes().to_vec();
        wert.extend([0x15, 0xc7]);
        wert.extend(ort);
        udta.extend(boxe(b"\xa9xyz", &wert));
    }
    let mut moov = boxe(b"mvhd", &[0; 100]);
    moov.extend(boxe(b"udta", &udta));
    let mut bytes = boxe(b"ftyp", &ftyp);
    bytes.extend(boxe(b"moov", &moov));
    bytes.extend(boxe(b"mdat", b"MEDIENDATEN-MEDIENDATEN"));
    bytes
}

pub fn webm() -> Vec<u8> {
    let mut kopf = vec![0x42, 0x86, 0x81, 0x01, 0x42, 0x82, 0x84];
    kopf.extend(b"webm");
    kopf.extend([0x42, 0x87, 0x81, 0x04]);
    let mut bytes = vec![0x1a, 0x45, 0xdf, 0xa3, 0x80 | kopf.len() as u8];
    bytes.extend(kopf);
    bytes.extend([0x18, 0x53, 0x80, 0x67, 0x01, 0, 0, 0, 0, 0, 0, 0x40]);
    bytes.extend((0..4_000u32).map(|n| (n % 251) as u8));
    bytes
}

pub fn ogg(paket: &[u8]) -> Vec<u8> {
    let mut bytes = b"OggS".to_vec();
    bytes.extend([0, 2]);
    bytes.extend([0; 20]);
    bytes.push(1);
    bytes.push(paket.len() as u8);
    bytes.extend(paket);
    bytes
}

pub fn mp3() -> Vec<u8> {
    let mut rahmen = vec![0xff, 0xfb, 0x90, 0x00];
    rahmen.resize(417, 0x55);
    let mut bytes = rahmen.clone();
    bytes.extend(&rahmen);
    bytes
}
