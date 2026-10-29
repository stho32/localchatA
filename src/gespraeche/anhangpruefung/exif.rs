//! Orientierung aus IFD0 (B0523, ADR-0047 Ergänzung „Karte #9 Nachfragen planer“ Punkt 1, „Ein Leser“ für Erkenner
//! und Tilger): Tag `0x0112` nur aus IFD0 einer EXIF-Struktur ab dem TIFF-Kopf, beide Byte-Ordnungen, höchstens
//! die ersten 1 MiB. Rein.

/// Höchstens so viele Bytes ab dem TIFF-Kopf werden gelesen (ADR-0047 Ergänzung „Karte #9 Security-Gate“ Regel 1).
pub const HOECHSTENS: usize = 1_048_576;

const TAG_ORIENTATION: u16 = 0x0112;
const TYP_SHORT: u16 = 3;

/// Byte-Ordnung des TIFF-Kopfs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ordnung {
    Intel,
    Motorola,
}

impl Ordnung {
    pub fn u16(self, bytes: &[u8], stelle: usize) -> Option<u16> {
        let paar: [u8; 2] = bytes.get(stelle..stelle.checked_add(2)?)?.try_into().ok()?;
        Some(match self {
            Ordnung::Intel => u16::from_le_bytes(paar),
            Ordnung::Motorola => u16::from_be_bytes(paar),
        })
    }

    pub fn u32(self, bytes: &[u8], stelle: usize) -> Option<u32> {
        let vier: [u8; 4] = bytes.get(stelle..stelle.checked_add(4)?)?.try_into().ok()?;
        Some(match self {
            Ordnung::Intel => u32::from_le_bytes(vier),
            Ordnung::Motorola => u32::from_be_bytes(vier),
        })
    }
}

/// Die Byte-Ordnung aus `II*\0` bzw. `MM\0*`, sonst keine.
pub fn ordnung(tiff: &[u8]) -> Option<Ordnung> {
    match tiff.get(..4)? {
        [b'I', b'I', 42, 0] => Some(Ordnung::Intel),
        [b'M', b'M', 0, 42] => Some(Ordnung::Motorola),
        _ => None,
    }
}

/// `Orientation` 1–8 aus IFD0; Typ falsch, Wert 0 oder ab 9, Eintrag oder IFD0 außerhalb der Bytes, nur in IFD1 →
/// keine.
pub fn orientierung(tiff: &[u8]) -> Option<u16> {
    let tiff = &tiff[..tiff.len().min(HOECHSTENS)];
    let ordnung = ordnung(tiff)?;
    let ifd0 = usize::try_from(ordnung.u32(tiff, 4)?).ok()?;
    let anzahl = usize::from(ordnung.u16(tiff, ifd0)?);
    for nummer in 0..anzahl {
        let eintrag = ifd0.checked_add(2)?.checked_add(nummer.checked_mul(12)?)?;
        let tag = ordnung.u16(tiff, eintrag)?;
        if tag != TAG_ORIENTATION {
            continue;
        }
        let typ = ordnung.u16(tiff, eintrag + 2)?;
        let zahl = ordnung.u32(tiff, eintrag + 4)?;
        if typ != TYP_SHORT || zahl != 1 {
            return None;
        }
        let wert = ordnung.u16(tiff, eintrag + 8)?;
        return (1..=8).contains(&wert).then_some(wert);
    }
    None
}

/// Bei 5–8 sind Breite und Höhe der angezeigten Fassung getauscht.
pub fn tauscht(orientierung: Option<u16>) -> bool {
    matches!(orientierung, Some(5..=8))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Eine TIFF-Struktur mit den Einträgen von IFD0 (Tag, Typ, Zahl, Wert) und optional einem IFD1 mit einem
    /// Eintrag.
    pub fn tiff(
        ordnung: Ordnung,
        ifd0: &[(u16, u16, u32, u16)],
        ifd1: Option<(u16, u16)>,
    ) -> Vec<u8> {
        let u16b = |wert: u16| match ordnung {
            Ordnung::Intel => wert.to_le_bytes().to_vec(),
            Ordnung::Motorola => wert.to_be_bytes().to_vec(),
        };
        let u32b = |wert: u32| match ordnung {
            Ordnung::Intel => wert.to_le_bytes().to_vec(),
            Ordnung::Motorola => wert.to_be_bytes().to_vec(),
        };
        let mut bytes = match ordnung {
            Ordnung::Intel => b"II*\0".to_vec(),
            Ordnung::Motorola => b"MM\0*".to_vec(),
        };
        bytes.extend(u32b(8));
        bytes.extend(u16b(ifd0.len() as u16));
        for (tag, typ, zahl, wert) in ifd0 {
            bytes.extend(u16b(*tag));
            bytes.extend(u16b(*typ));
            bytes.extend(u32b(*zahl));
            bytes.extend(u16b(*wert));
            bytes.extend([0, 0]);
        }
        let naechstes = bytes.len() as u32 + 4;
        bytes.extend(u32b(if ifd1.is_some() { naechstes } else { 0 }));
        if let Some((tag, wert)) = ifd1 {
            bytes.extend(u16b(1));
            bytes.extend(u16b(tag));
            bytes.extend(u16b(TYP_SHORT));
            bytes.extend(u32b(1));
            bytes.extend(u16b(wert));
            bytes.extend([0, 0]);
            bytes.extend(u32b(0));
        }
        bytes
    }

    /// Motorola- und Intel-Kopf je mit 6 → 6; ein Eintrag davor stört nicht.
    #[test]
    fn beide_byte_ordnungen_liefern_die_orientierung() {
        for ordnung in [Ordnung::Intel, Ordnung::Motorola] {
            let bytes = tiff(
                ordnung,
                &[(0x010f, 2, 4, 0), (TAG_ORIENTATION, TYP_SHORT, 1, 6)],
                None,
            );
            assert_eq!(orientierung(&bytes), Some(6), "{ordnung:?}");
        }
    }

    /// Nur in IFD1 → keine; Wert 9 und 0 → keine; Typ falsch → keine.
    #[test]
    fn nur_ifd0_mit_gueltigem_typ_und_wert_zaehlt() {
        assert_eq!(
            orientierung(&tiff(
                Ordnung::Intel,
                &[(0x010f, 2, 4, 0)],
                Some((TAG_ORIENTATION, 6))
            )),
            None
        );
        for falsch in [0, 9, 255] {
            assert_eq!(
                orientierung(&tiff(
                    Ordnung::Intel,
                    &[(TAG_ORIENTATION, TYP_SHORT, 1, falsch)],
                    None
                )),
                None
            );
        }
        assert_eq!(
            orientierung(&tiff(Ordnung::Intel, &[(TAG_ORIENTATION, 4, 1, 6)], None)),
            None
        );
        assert_eq!(
            orientierung(&tiff(
                Ordnung::Intel,
                &[(TAG_ORIENTATION, TYP_SHORT, 2, 6)],
                None
            )),
            None
        );
        for gut in 1..=8 {
            assert_eq!(
                orientierung(&tiff(
                    Ordnung::Motorola,
                    &[(TAG_ORIENTATION, TYP_SHORT, 1, gut)],
                    None
                )),
                Some(gut)
            );
        }
    }

    /// IFD0-Offset hinter dem Ende, abgeschnittener Eintrag, kein TIFF-Kopf → keine, nie Panik.
    #[test]
    fn ausserhalb_der_bytes_ist_keine_orientierung() {
        let mut hinten = tiff(Ordnung::Intel, &[(TAG_ORIENTATION, TYP_SHORT, 1, 6)], None);
        hinten[4..8].copy_from_slice(&1_000u32.to_le_bytes());
        assert_eq!(orientierung(&hinten), None);
        let ganz = tiff(Ordnung::Intel, &[(TAG_ORIENTATION, TYP_SHORT, 1, 6)], None);
        for laenge in 0..ganz.len() {
            let _ = orientierung(&ganz[..laenge]);
        }
        assert_eq!(orientierung(&ganz[..18]), None);
        assert_eq!(orientierung(b"XX*\0\x08\0\0\0"), None);
        let mut gross = tiff(Ordnung::Intel, &[(TAG_ORIENTATION, TYP_SHORT, 1, 6)], None);
        gross[4..8].copy_from_slice(&(u32::MAX).to_le_bytes());
        assert_eq!(orientierung(&gross), None);
    }

    /// IFD0 hinter der ersten 1 MiB → wie ohne `Orientation`.
    #[test]
    fn ifd0_hinter_einem_mebibyte_zaehlt_nicht() {
        let mut bytes = b"II*\0".to_vec();
        bytes.extend((HOECHSTENS as u32).to_le_bytes());
        bytes.resize(HOECHSTENS, 0);
        bytes.extend(1u16.to_le_bytes());
        bytes.extend(TAG_ORIENTATION.to_le_bytes());
        bytes.extend(TYP_SHORT.to_le_bytes());
        bytes.extend(1u32.to_le_bytes());
        bytes.extend(6u16.to_le_bytes());
        bytes.extend([0, 0, 0, 0, 0, 0]);
        assert_eq!(orientierung(&bytes), None);
    }

    #[test]
    fn fuenf_bis_acht_tauschen() {
        assert!(tauscht(Some(5)) && tauscht(Some(6)) && tauscht(Some(7)) && tauscht(Some(8)));
        assert!(!tauscht(Some(1)) && !tauscht(Some(3)) && !tauscht(Some(4)) && !tauscht(None));
    }
}
