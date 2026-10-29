//! Medienart (B0434, B0584, ADR-0047 Frage 3): PDF, ISOBMFF (`ftyp` ab Byte 4, Boxgröße 16–4.096; nach allen
//! Marken AVIF, HEIC, HEIF, sonst nach der Hauptmarke `M4A `/`M4B ` Audio, `qt  ` keine, jede andere MP4), WebM (EBML, `DocType` `webm`; `matroska` keine), Ogg (Version 0; Opus und Vorbis Audio, Theora
//! Video), MP3 (`ID3` v2–4 oder zwei gültige Rahmenköpfe Layer III). Rein, auf den ersten höchstens 64 KiB; jede
//! Länge mit Überlaufprüfung, jede Schleife schreitet fort oder endet.

/// Die Medienart der ersten Bytes, sonst keine.
pub fn medien(kopf: &[u8]) -> Option<&'static str> {
    if kopf.starts_with(b"%PDF-") {
        return Some("application/pdf");
    }
    if let Some(art) = mp4(kopf) {
        return art;
    }
    if kopf.starts_with(&[0x1a, 0x45, 0xdf, 0xa3]) {
        return webm(kopf);
    }
    if kopf.starts_with(b"OggS") {
        return ogg(kopf);
    }
    mp3(kopf)
}

/// Marken der HEIF-Familie (ADR-0047 Ergänzung „Karte #9 Nacharbeit F3 und F4“ Regel 2).
const AVIF: [&[u8; 4]; 2] = [b"avif", b"avis"];
const HEIC: [&[u8; 4]; 6] = [b"heic", b"heix", b"heim", b"heis", b"hevc", b"hevx"];
const HEIF: [&[u8; 4]; 2] = [b"mif1", b"msf1"];

/// `Some(Some(art))` ISOBMFF mit Art, `Some(None)` ein `ftyp`, das keine Art hat (`qt  `), `None` kein `ftyp`.
/// Alle Marken zählen — Hauptmarke und kompatible —, zuerst AVIF, dann HEIC, dann HEIF, dann MP4 nach der Hauptmarke.
fn mp4(kopf: &[u8]) -> Option<Option<&'static str>> {
    let groesse = u32::from_be_bytes(kopf.get(0..4)?.try_into().ok()?);
    if kopf.get(4..8)? != b"ftyp" || !(16..=4_096).contains(&groesse) {
        return None;
    }
    let haupt = kopf.get(8..12)?;
    let kompatibel = kopf.get(16..(groesse as usize).min(kopf.len()))?;
    let marken: Vec<&[u8]> = std::iter::once(haupt)
        .chain(kompatibel.chunks_exact(4))
        .collect();
    let unter = |familie: &[&[u8; 4]]| {
        marken
            .iter()
            .any(|marke| familie.iter().any(|f| f.as_slice() == *marke))
    };
    Some(if unter(&AVIF) {
        Some(AVIF_TYP)
    } else if unter(&HEIC) {
        Some(HEIC_TYP)
    } else if unter(&HEIF) {
        Some(HEIF_TYP)
    } else {
        match haupt {
            b"M4A " | b"M4B " => Some("audio/mp4"),
            b"qt  " => None,
            _ => Some("video/mp4"),
        }
    })
}

pub const AVIF_TYP: &str = "image/avif";
pub const HEIC_TYP: &str = "image/heic";
pub const HEIF_TYP: &str = "image/heif";

/// Eine EBML-Zahl variabler Länge ab `stelle`: (Wert ohne Marke, Länge); `mit_marke` für IDs.
fn vint(bytes: &[u8], stelle: usize, mit_marke: bool) -> Option<(u64, usize)> {
    let erstes = *bytes.get(stelle)?;
    let laenge = erstes.leading_zeros() as usize + 1;
    if laenge > 8 {
        return None;
    }
    let mut wert = if mit_marke {
        u64::from(erstes)
    } else {
        u64::from(erstes) & (0xff >> laenge)
    };
    for byte in bytes.get(stelle + 1..stelle + laenge)? {
        wert = (wert << 8) | u64::from(*byte);
    }
    Some((wert, laenge))
}

/// Der EBML-Kopf; darin `DocType` (0x4282): `webm` → Video, alles andere keine Art.
fn webm(kopf: &[u8]) -> Option<&'static str> {
    let (groesse, laenge) = vint(kopf, 4, false)?;
    let anfang = 4 + laenge;
    let ende = anfang
        .checked_add(usize::try_from(groesse).ok()?)?
        .min(kopf.len());
    let mut stelle = anfang;
    while stelle < ende {
        let (id, id_laenge) = vint(kopf, stelle, true)?;
        let (wert_laenge, laenge_laenge) = vint(kopf, stelle + id_laenge, false)?;
        let wert_anfang = stelle + id_laenge + laenge_laenge;
        let wert_ende = wert_anfang.checked_add(usize::try_from(wert_laenge).ok()?)?;
        if id == 0x4282 {
            let wert = kopf.get(wert_anfang..wert_ende)?;
            let ohne_nul = wert.split(|byte| *byte == 0).next().unwrap_or_default();
            return (ohne_nul == b"webm").then_some("video/webm");
        }
        stelle = wert_ende;
    }
    None
}

/// Erste Seite: Version 0, Segmenttabelle, das erste Paket entscheidet.
fn ogg(kopf: &[u8]) -> Option<&'static str> {
    if *kopf.get(4)? != 0 {
        return None;
    }
    let segmente = usize::from(*kopf.get(26)?);
    let paket = kopf.get(27 + segmente..)?;
    if paket.starts_with(b"\x01vorbis") || paket.starts_with(b"OpusHead") {
        Some("audio/ogg")
    } else if paket.starts_with(b"\x80theora") {
        Some("video/ogg")
    } else {
        None
    }
}

/// `ID3` mit Version 2–4, oder ein Rahmenkopf Layer III und ein zweiter an der berechneten Stelle.
fn mp3(kopf: &[u8]) -> Option<&'static str> {
    if kopf.starts_with(b"ID3") && matches!(kopf.get(3), Some(2..=4)) {
        return Some("audio/mpeg");
    }
    let laenge = rahmenlaenge(kopf.get(0..4)?)?;
    rahmenlaenge(kopf.get(laenge..laenge.checked_add(4)?)?)?;
    Some("audio/mpeg")
}

/// Länge eines MPEG-Audio-Rahmens Layer III aus seinem Kopf; ein ungültiger Kopf hat keine.
fn rahmenlaenge(kopf: &[u8]) -> Option<usize> {
    let [a, b, c, _] = <[u8; 4]>::try_from(kopf).ok()?;
    if a != 0xff || b & 0xe0 != 0xe0 {
        return None;
    }
    let version = (b >> 3) & 0b11;
    let schicht = (b >> 1) & 0b11;
    if version == 0b01 || schicht != 0b01 {
        return None;
    }
    let bitraten_index = usize::from(c >> 4);
    let rate_index = usize::from((c >> 2) & 0b11);
    if bitraten_index == 0 || bitraten_index == 15 || rate_index == 3 {
        return None;
    }
    const MPEG1: [u32; 14] = [
        32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320,
    ];
    const MPEG2: [u32; 14] = [8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160];
    let (bitrate, abtastrate, faktor) = match version {
        0b11 => (
            MPEG1[bitraten_index - 1],
            [44_100, 48_000, 32_000][rate_index],
            144,
        ),
        0b10 => (
            MPEG2[bitraten_index - 1],
            [22_050, 24_000, 16_000][rate_index],
            72,
        ),
        _ => (
            MPEG2[bitraten_index - 1],
            [11_025, 12_000, 8_000][rate_index],
            72,
        ),
    };
    let fuellung = u32::from((c >> 1) & 1);
    let laenge = faktor * bitrate * 1_000 / abtastrate + fuellung;
    (laenge >= 4).then_some(laenge as usize)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub fn ftyp(marke: &[u8; 4]) -> Vec<u8> {
        let mut bytes = 24u32.to_be_bytes().to_vec();
        bytes.extend(b"ftyp");
        bytes.extend(marke);
        bytes.extend([0, 0, 2, 0]);
        bytes.extend(b"isommp41");
        bytes
    }

    pub fn ebml(doctype: &[u8]) -> Vec<u8> {
        let mut kopf = vec![
            0x42,
            0x86,
            0x81,
            0x01,
            0x42,
            0x82,
            0x80 | doctype.len() as u8,
        ];
        kopf.extend(doctype);
        kopf.extend([0x42, 0x87, 0x81, 0x04]);
        let mut bytes = vec![0x1a, 0x45, 0xdf, 0xa3, 0x80 | kopf.len() as u8];
        bytes.extend(kopf);
        bytes.extend([0x18, 0x53, 0x80, 0x67, 0x01, 0, 0, 0, 0, 0, 0, 0]);
        bytes
    }

    pub fn ogg_mit(paket: &[u8]) -> Vec<u8> {
        let mut bytes = b"OggS".to_vec();
        bytes.extend([0, 2]);
        bytes.extend([0; 20]);
        bytes.push(1);
        bytes.push(paket.len() as u8);
        bytes.extend(paket);
        bytes
    }

    /// MPEG1 Layer III, 128 kbit/s, 44,1 kHz, ohne Füllung: 417 Bytes je Rahmen.
    pub fn mp3_rahmen() -> Vec<u8> {
        let mut rahmen = vec![0xff, 0xfb, 0x90, 0x00];
        rahmen.resize(417, 0);
        rahmen
    }

    #[test]
    fn die_tabelle_der_medienarten() {
        assert_eq!(medien(b"%PDF-1.7\n"), Some("application/pdf"));
        assert_eq!(medien(&ftyp(b"isom")), Some("video/mp4"));
        assert_eq!(medien(&ftyp(b"M4A ")), Some("audio/mp4"));
        assert_eq!(medien(&ftyp(b"M4B ")), Some("audio/mp4"));
        assert_eq!(medien(&ftyp(b"qt  ")), None);
        assert_eq!(medien(&ebml(b"webm")), Some("video/webm"));
        assert_eq!(medien(&ebml(b"webm\0\0")), Some("video/webm"));
        assert_eq!(medien(&ebml(b"matroska")), None);
        assert_eq!(medien(&ogg_mit(b"OpusHead\x01\x02")), Some("audio/ogg"));
        assert_eq!(medien(&ogg_mit(b"\x01vorbis\0\0")), Some("audio/ogg"));
        assert_eq!(medien(&ogg_mit(b"\x80theora\x03")), Some("video/ogg"));
        assert_eq!(medien(&ogg_mit(b"\x7fFLAC")), None);
        assert_eq!(medien(b"ID3\x04\0\0\0\0\0\0"), Some("audio/mpeg"));
        assert_eq!(medien(b"ID3\x05\0\0\0\0\0\0"), None);
        let mut zwei = mp3_rahmen();
        zwei.extend(mp3_rahmen());
        assert_eq!(medien(&zwei), Some("audio/mpeg"));
    }

    /// Ein einzelner Rahmenkopf ohne zweiten ist kein MP3 (Mutante „MP3 ohne zweiten Rahmenkopf“); ungültige Köpfe
    /// (Bitrate 0 oder 15, Abtastrate 3, Layer I) auch nicht.
    #[test]
    fn ein_rahmenkopf_allein_ist_kein_mp3() {
        assert_eq!(medien(&mp3_rahmen()), None);
        let mut falsch_dahinter = mp3_rahmen();
        falsch_dahinter.extend([0x12, 0x34, 0x56, 0x78]);
        assert_eq!(medien(&falsch_dahinter), None);
        for kopf in [
            [0xff, 0xfb, 0x00, 0x00],
            [0xff, 0xfb, 0xf0, 0x00],
            [0xff, 0xfb, 0x9c, 0x00],
            [0xff, 0xff, 0x90, 0x00],
        ] {
            let mut bytes = kopf.to_vec();
            bytes.resize(417, 0);
            bytes.extend(kopf);
            assert_eq!(medien(&bytes), None, "{kopf:x?}");
        }
    }

    /// Ogg nur in Version 0; `ftyp` außerhalb 16–4.096 ist kein MP4.
    #[test]
    fn ogg_version_und_ftyp_groesse() {
        let mut version1 = ogg_mit(b"OpusHead");
        version1[4] = 1;
        assert_eq!(medien(&version1), None);
        let mut klein = ftyp(b"isom");
        klein[0..4].copy_from_slice(&8u32.to_be_bytes());
        assert_eq!(medien(&klein), None);
        let mut gross = ftyp(b"isom");
        gross[0..4].copy_from_slice(&4_097u32.to_be_bytes());
        assert_eq!(medien(&gross), None);
    }

    /// M3: jede Signatur abgeschnitten nach 1, 4, 8, 16 und 32 Bytes, Längenfelder 0 und 2⁶⁴ − 1 → nie Panik.
    #[test]
    fn kaputte_koepfe_enden_ohne_panik() {
        let mut zwei = mp3_rahmen();
        zwei.extend(mp3_rahmen());
        for ganz in [
            ftyp(b"isom"),
            ebml(b"webm"),
            ogg_mit(b"OpusHead"),
            zwei,
            b"ID3\x04".to_vec(),
        ] {
            for laenge in [1, 4, 8, 16, 32].into_iter().chain(0..ganz.len().min(600)) {
                let _ = medien(&ganz[..laenge.min(ganz.len())]);
            }
        }
        let mut riesig = ebml(b"webm");
        riesig[4] = 0x01;
        riesig.splice(5..5, [0xff; 7]);
        let _ = medien(&riesig);
        let mut null = ebml(b"webm");
        null[4] = 0x80;
        assert_eq!(medien(&null), None);
        let _ = medien(&[0x1a, 0x45, 0xdf, 0xa3, 0x00]);
        let mut ogg = ogg_mit(b"OpusHead");
        ogg[26] = 255;
        let _ = medien(&ogg);
    }
}
