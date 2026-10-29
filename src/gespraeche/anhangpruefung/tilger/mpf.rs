//! Weitere JPEG-Bilder über MPF tilgen (B0522, ADR-0047 Ergänzung „Karte #9 Security-Gate“ L12 V-a, Regel 4): trägt
//! das erste Bild ein `APP2` mit `MPF\0`, gilt die JPEG-Regel für jedes weitere Bild, das der MP-Index nennt — Tiefe
//! 1; das `APP2` `MPF` bleibt.

use super::Zaehler;
use super::bild::{JpegBild, jpeg};
use super::port::{Abbruch, Ergebnis, Port};
use crate::gespraeche::anhangpruefung::exif::ordnung;

const TAG_MP_ENTRY: u16 = 0xB002;
const EINTRAG: usize = 16;

fn metadaten<T>(grund: &'static str) -> Ergebnis<T> {
    Err(Abbruch::Metadaten(grund))
}

/// Ein JPEG samt der Bilder, die sein MP-Index nennt. Zwei Durchgänge über den Index (≤ 64 KiB) statt einer Liste:
/// erst prüfen, dass jeder Eintrag ganz in der Datei liegt, dann jedes Bild tilgen — der Speicher bleibt konstant,
/// auch wenn viele gleiche `MP_ENTRY`-Tags Millionen Nennungen ergeben (die Blockgrenze beendet die Arbeit).
pub fn jpeg_mit_mpf(port: &Port<'_>, zaehler: &mut Zaehler) -> Ergebnis<()> {
    let erstes = jpeg(port, 0, zaehler)?;
    let JpegBild {
        sos,
        mpf: Some((tiff, laenge)),
    } = erstes
    else {
        return Ok(());
    };
    let index = port.lesen(tiff, laenge)?;
    weitere_bilder(port, &index, tiff, |_| Ok(()))?;
    weitere_bilder(port, &index, tiff, |anfang| {
        if anfang <= sos
            || !port.enthaelt(anfang, 3)
            || port.lesen(anfang, 3)? != [0xFF, 0xD8, 0xFF]
        {
            return metadaten("MPF-Eintrag nicht ganz in der Datei");
        }
        zaehler.block()?;
        jpeg(port, anfang, zaehler).map(|_| ())
    })
}

/// Die Anfänge der weiteren Bilder aus dem MP-Index (Versätze ab dem TIFF-Kopf des MPF-Segments), je an `je_bild`
/// in der Reihenfolge des Index; ein Eintrag, der nicht ganz in der Datei liegt, beendet mit `metadaten`.
fn weitere_bilder(
    port: &Port<'_>,
    bytes: &[u8],
    tiff: u64,
    mut je_bild: impl FnMut(u64) -> Ergebnis<()>,
) -> Ergebnis<()> {
    let Some(ordnung) = ordnung(bytes) else {
        return metadaten("MPF ohne TIFF-Kopf");
    };
    let falsch = || Abbruch::Metadaten("MPF-Index unlesbar");
    let ifd = ordnung.u32(bytes, 4).ok_or_else(falsch)? as usize;
    let anzahl = usize::from(ordnung.u16(bytes, ifd).ok_or_else(falsch)?);
    for nummer in 0..anzahl {
        let eintrag = ifd + 2 + nummer * 12;
        if ordnung.u16(bytes, eintrag).ok_or_else(falsch)? != TAG_MP_ENTRY {
            continue;
        }
        let zahl = ordnung.u32(bytes, eintrag + 4).ok_or_else(falsch)? as usize;
        let versatz = ordnung.u32(bytes, eintrag + 8).ok_or_else(falsch)? as usize;
        let ende = versatz.checked_add(zahl).ok_or_else(falsch)?;
        if !zahl.is_multiple_of(EINTRAG) || ende > bytes.len() {
            return metadaten("MPF-Eintrag nicht ganz in der Datei");
        }
        for bild in (versatz..ende).step_by(EINTRAG).skip(1) {
            let groesse = u64::from(ordnung.u32(bytes, bild + 4).ok_or_else(falsch)?);
            let ab = u64::from(ordnung.u32(bytes, bild + 8).ok_or_else(falsch)?);
            let anfang = tiff.checked_add(ab).ok_or_else(falsch)?;
            if !port.enthaelt(anfang, groesse) {
                return metadaten("MPF-Eintrag nicht ganz in der Datei");
            }
            je_bild(anfang)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::port::attrappe::Datei;
    use super::*;
    use crate::gespraeche::anhangpruefung::exif::orientierung;
    use crate::gespraeche::anhangpruefung::exif::{Ordnung, tests::tiff};

    fn segment(marker: u8, inhalt: &[u8]) -> Vec<u8> {
        let mut bytes = vec![0xFF, marker];
        bytes.extend_from_slice(&((inhalt.len() + 2) as u16).to_be_bytes());
        bytes.extend_from_slice(inhalt);
        bytes
    }

    fn exif_mit_gps() -> Vec<u8> {
        let mut exif = b"Exif\0\0".to_vec();
        exif.extend(tiff(
            Ordnung::Motorola,
            &[(0x0112, 3, 1, 8), (0x8825, 4, 1, 40)],
            None,
        ));
        exif.extend_from_slice(b"GPS 48.1 11.5");
        exif
    }

    /// Erstes Bild mit `APP2` `MPF` (zwei Einträge), zweites Bild mit Exif-GPS dahinter.
    fn mit_mpf(versatz_aendern: u32) -> Vec<u8> {
        let zweites = [
            vec![0xFF, 0xD8],
            segment(0xE1, &exif_mit_gps()),
            segment(0xDA, b"\x01"),
            b"Tiefe\xFF\xD9".to_vec(),
        ]
        .concat();
        // MPF: TIFF (MM) mit IFD0 = 1 Eintrag MPEntry (UNDEFINED, 32 Bytes ab Versatz 26).
        let mpf_laenge = 4 + 26 + 32;
        let erstes_ohne_mpf = 2 + segment(0xE1, &exif_mit_gps()).len();
        let vor_dem_zweiten = erstes_ohne_mpf + 4 + mpf_laenge + segment(0xDA, b"\x01").len() + 6;
        let tiff_anfang = erstes_ohne_mpf + 4 + 4;
        let mut mpf = b"MPF\0MM\0*\0\0\0\x08\0\x01".to_vec();
        mpf.extend_from_slice(&0xB002u16.to_be_bytes());
        mpf.extend_from_slice(&7u16.to_be_bytes());
        mpf.extend_from_slice(&32u32.to_be_bytes());
        mpf.extend_from_slice(&26u32.to_be_bytes());
        mpf.extend_from_slice(&0u32.to_be_bytes());
        mpf.extend_from_slice(&[0; 16]);
        mpf.extend_from_slice(&[0, 0, 0, 0]);
        mpf.extend_from_slice(&(zweites.len() as u32).to_be_bytes());
        mpf.extend_from_slice(
            &((vor_dem_zweiten - tiff_anfang) as u32 + versatz_aendern).to_be_bytes(),
        );
        mpf.extend_from_slice(&[0, 0, 0, 0]);
        assert_eq!(mpf.len(), mpf_laenge);
        let erstes = [
            vec![0xFF, 0xD8],
            segment(0xE1, &exif_mit_gps()),
            segment(0xE2, &mpf),
            segment(0xDA, b"\x01"),
            b"Bild\xFF\xD9".to_vec(),
        ]
        .concat();
        assert_eq!(erstes.len(), vor_dem_zweiten);
        [erstes, zweites].concat()
    }

    fn enthaelt(heu: &[u8], nadel: &[u8]) -> bool {
        heu.windows(nadel.len()).any(|fenster| fenster == nadel)
    }

    #[test]
    fn das_zweite_bild_verliert_sein_gps_beide_behalten_die_orientierung() {
        let datei = Datei::mit(mit_mpf(0));
        let mut zaehler = Zaehler::default();

        jpeg_mit_mpf(&Port::neu(&datei).expect("Port"), &mut zaehler).expect("getilgt");

        let getilgt = datei.inhalt();
        assert!(!enthaelt(&getilgt, b"GPS"));
        assert!(enthaelt(&getilgt, b"MPF\0MM"), "MPF bleibt");
        let exifs: Vec<usize> = getilgt
            .windows(6)
            .enumerate()
            .filter(|(_, f)| f == b"Exif\0\0")
            .map(|(stelle, _)| stelle)
            .collect();
        assert_eq!(exifs.len(), 2);
        for stelle in exifs {
            assert_eq!(orientierung(&getilgt[stelle + 6..]), Some(8));
        }
    }

    #[test]
    fn ein_mpf_eintrag_hinter_dem_dateiende_ist_nicht_tilgbar() {
        let datei = Datei::mit(mit_mpf(1_000_000));

        let ergebnis = jpeg_mit_mpf(&Port::neu(&datei).expect("Port"), &mut Zaehler::default());

        assert!(matches!(ergebnis, Err(Abbruch::Metadaten(_))));
    }

    #[test]
    fn ein_mpf_eintrag_vor_dem_sos_des_ersten_bildes_ist_nicht_tilgbar() {
        let mut bytes = mit_mpf(0);
        let mpf = bytes.windows(4).position(|f| f == b"MPF\0").expect("MPF");
        let feld = mpf + 4 + 26 + 16 + 8;
        bytes[feld..feld + 4].copy_from_slice(&0u32.to_be_bytes());
        let datei = Datei::mit(bytes);

        let ergebnis = jpeg_mit_mpf(&Port::neu(&datei).expect("Port"), &mut Zaehler::default());

        assert!(matches!(ergebnis, Err(Abbruch::Metadaten(_))));
    }
}
