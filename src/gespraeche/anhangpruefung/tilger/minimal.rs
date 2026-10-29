//! Minimales EXIF (B0501, A00018 Ergänzung F4): ein TIFF-Kopf mit einem IFD, darin höchstens der Eintrag
//! `Orientation`, wenn er ≠ 1 war; der Rest des alten Blocks wird Nullen. Nie wird über die Blockgrenze geschrieben.

use crate::gespraeche::anhangpruefung::exif::{HOECHSTENS, ordnung};

/// Länge des minimalen EXIF mit `Orientation` samt Kennung `Exif\0\0` eines JPEG-`APP1` (6 + 26 Bytes).
pub const MINIMAL: usize = 32;

/// TIFF-Kopf (8) + Zahl der Einträge (2) + ein Eintrag (12) + Versatz des nächsten IFD (4).
const TIFF_MIT_EINTRAG: usize = 26;

/// Der neue Inhalt eines Exif-Blocks von `laenge` Bytes: `praefix` (z. B. `Exif\0\0`, sonst leer), dann ein TIFF
/// (Motorola) mit IFD0 und — wenn `orientierung` ≠ 1 war — dem einen Eintrag `0x0112`; Rest Nullen. Ist der Block
/// kürzer als `praefix` + 26 Bytes (das minimale EXIF), wird er nur genullt, auch ohne `Orientation`.
pub fn minimales_exif(orientierung: Option<u16>, laenge: usize, praefix: &[u8]) -> Vec<u8> {
    let mut block = vec![0u8; laenge];
    if laenge < praefix.len() + TIFF_MIT_EINTRAG {
        return block;
    }
    let mut tiff = Vec::with_capacity(TIFF_MIT_EINTRAG);
    tiff.extend_from_slice(b"MM\0\x2a\0\0\0\x08");
    match orientierung.filter(|wert| (2..=8).contains(wert)) {
        Some(wert) => {
            tiff.extend_from_slice(&1u16.to_be_bytes());
            tiff.extend_from_slice(&0x0112u16.to_be_bytes());
            tiff.extend_from_slice(&3u16.to_be_bytes());
            tiff.extend_from_slice(&1u32.to_be_bytes());
            tiff.extend_from_slice(&wert.to_be_bytes());
            tiff.extend_from_slice(&[0, 0]);
        }
        None => tiff.extend_from_slice(&0u16.to_be_bytes()),
    }
    tiff.extend_from_slice(&0u32.to_be_bytes());
    block[..praefix.len()].copy_from_slice(praefix);
    block[praefix.len()..praefix.len() + tiff.len()].copy_from_slice(&tiff);
    block
}

/// Ob IFD0 auf ein GPS-IFD zeigt (Tag `0x8825`) — für `ort` im Protokoll, nie mit Inhalt.
pub fn traegt_ort(tiff: &[u8]) -> bool {
    let tiff = &tiff[..tiff.len().min(HOECHSTENS)];
    let Some(ordnung) = ordnung(tiff) else {
        return false;
    };
    let Some(ifd0) = ordnung
        .u32(tiff, 4)
        .and_then(|wert| usize::try_from(wert).ok())
    else {
        return false;
    };
    let anzahl = usize::from(ordnung.u16(tiff, ifd0).unwrap_or(0));
    (0..anzahl).any(|nummer| {
        ifd0.checked_add(2 + nummer * 12)
            .and_then(|eintrag| ordnung.u16(tiff, eintrag))
            == Some(TAG_GPS)
    })
}

const TAG_GPS: u16 = 0x8825;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gespraeche::anhangpruefung::exif::orientierung;

    #[test]
    fn die_orientierung_bleibt_und_der_rest_sind_nullen() {
        let block = minimales_exif(Some(6), 200, b"Exif\0\0");

        assert_eq!(block.len(), 200);
        assert_eq!(&block[..6], b"Exif\0\0");
        assert_eq!(orientierung(&block[6..]), Some(6));
        assert!(block[MINIMAL..].iter().all(|byte| *byte == 0));
    }

    #[test]
    fn orientierung_1_oder_keine_ergibt_ein_leeres_ifd() {
        for wert in [Some(1), None, Some(9)] {
            let block = minimales_exif(wert, 64, b"");
            assert_eq!(orientierung(&block), None, "{wert:?}");
            assert_eq!(&block[..10], b"MM\0\x2a\0\0\0\x08\0\0");
        }
    }

    #[test]
    fn ein_gps_ifd_in_ifd0_ist_ein_ort() {
        use crate::gespraeche::anhangpruefung::exif::{Ordnung, tests::tiff};
        assert!(traegt_ort(&tiff(
            Ordnung::Intel,
            &[(0x0112, 3, 1, 1), (0x8825, 4, 1, 40)],
            None
        )));
        assert!(!traegt_ort(&tiff(
            Ordnung::Motorola,
            &[(0x0112, 3, 1, 1)],
            Some((0x8825, 1))
        )));
        assert!(!traegt_ort(&minimales_exif(Some(6), 64, b"")));
    }

    #[test]
    fn ein_block_kuerzer_als_das_minimale_exif_wird_nur_genullt() {
        assert_eq!(
            minimales_exif(Some(6), MINIMAL - 1, b"Exif\0\0"),
            vec![0; MINIMAL - 1]
        );
        assert_eq!(minimales_exif(Some(6), 25, b""), vec![0; 25]);
        assert_eq!(orientierung(&minimales_exif(Some(6), 26, b"")), Some(6));
        assert_eq!(minimales_exif(Some(6), MINIMAL, b"Exif\0\0").len(), MINIMAL);
    }
}
