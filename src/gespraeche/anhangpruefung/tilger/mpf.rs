//! Weitere JPEG-Bilder über MPF tilgen (B0522, ADR-0047 Ergänzung „Karte #9 Security-Gate“ L12 V-a, Regel 4): trägt
//! das erste Bild ein `APP2` mit `MPF\0`, gilt die JPEG-Regel für jedes weitere Bild, das der MP-Index nennt — Tiefe
//! 1; das `APP2` `MPF` bleibt. Verweise eindeutig und überschneidungsfrei, jedes Bild nur in seinem Bereich gelesen
//! (ADR-0047 Ergänzung „Karte #9 Security-Review B1“, B0639).

use super::Zaehler;
use super::bild::{JpegBild, jpeg};
use super::port::{Abbruch, Ergebnis, Port};
use crate::gespraeche::anhangpruefung::exif::ordnung;

const TAG_MP_ENTRY: u16 = 0xB002;
const EINTRAG: usize = 16;

fn metadaten<T>(grund: &'static str) -> Ergebnis<T> {
    Err(Abbruch::Metadaten(grund))
}

/// Ein weiteres Bild aus dem MP-Index: `[anfang, anfang + groesse)`.
type Bild = (u64, u64);

/// Ein JPEG samt der Bilder, die sein MP-Index nennt (ADR-0047 B1 Regeln 2, 3): der Index trägt genau ein
/// `MP_ENTRY`; die weiteren Bilder (≤ 4.096, der Index hat ≤ 64 KiB) werden im ersten Durchgang nach Anfang sortiert
/// und geprüft — je ganz in der Datei, hinter dem `SOS` des ersten Bildes, mit `FF D8 FF`, paarweise
/// überschneidungsfrei —, bevor ein Bild getilgt wird; jedes Bild wird nur in seinem Bereich gelesen.
pub fn jpeg_mit_mpf(port: &Port<'_>, zaehler: &mut Zaehler) -> Ergebnis<()> {
    let erstes = jpeg(port, 0, port.laenge(), zaehler)?;
    let JpegBild {
        sos,
        mpf: Some((tiff, laenge)),
    } = erstes
    else {
        return Ok(());
    };
    let index = port.lesen(tiff, laenge)?;
    let mut bilder = weitere_bilder(port, &index, tiff)?;
    bilder.sort_unstable();
    pruefen(port, &bilder, sos)?;
    for (anfang, groesse) in bilder {
        zaehler.block()?;
        jpeg(port, anfang, anfang + groesse, zaehler)?;
    }
    Ok(())
}

/// Die nach Anfang sortierten Bilder: hinter dem `SOS`, mit `FF D8 FF`, keines überschneidet das nächste.
fn pruefen(port: &Port<'_>, bilder: &[Bild], sos: u64) -> Ergebnis<()> {
    if bilder
        .windows(2)
        .any(|paar| paar[0].0 + paar[0].1 > paar[1].0)
    {
        return metadaten("Verweise überschneiden sich");
    }
    for &(anfang, groesse) in bilder {
        if anfang <= sos || groesse < 3 || port.lesen(anfang, 3)? != [0xFF, 0xD8, 0xFF] {
            return metadaten("MPF-Eintrag nicht ganz in der Datei");
        }
    }
    Ok(())
}

/// Die weiteren Bilder aus dem MP-Index (Versätze ab dem TIFF-Kopf des MPF-Segments): genau ein `MP_ENTRY`, sonst
/// `Verweise doppelt`; ein Eintrag, der nicht ganz in der Datei liegt, beendet mit `metadaten`.
fn weitere_bilder(port: &Port<'_>, bytes: &[u8], tiff: u64) -> Ergebnis<Vec<Bild>> {
    let Some(ordnung) = ordnung(bytes) else {
        return metadaten("MPF ohne TIFF-Kopf");
    };
    let falsch = || Abbruch::Metadaten("MPF-Index unlesbar");
    let ifd = ordnung.u32(bytes, 4).ok_or_else(falsch)? as usize;
    let anzahl = usize::from(ordnung.u16(bytes, ifd).ok_or_else(falsch)?);
    let mut eintraege = None;
    for nummer in 0..anzahl {
        let eintrag = ifd + 2 + nummer * 12;
        if ordnung.u16(bytes, eintrag).ok_or_else(falsch)? != TAG_MP_ENTRY {
            continue;
        }
        if eintraege.is_some() {
            return metadaten("Verweise doppelt");
        }
        let zahl = ordnung.u32(bytes, eintrag + 4).ok_or_else(falsch)? as usize;
        let versatz = ordnung.u32(bytes, eintrag + 8).ok_or_else(falsch)? as usize;
        let ende = versatz.checked_add(zahl).ok_or_else(falsch)?;
        if !zahl.is_multiple_of(EINTRAG) || ende > bytes.len() {
            return metadaten("MPF-Eintrag nicht ganz in der Datei");
        }
        eintraege = Some(versatz..ende);
    }
    let mut bilder = Vec::new();
    for bild in eintraege.unwrap_or_default().step_by(EINTRAG).skip(1) {
        let groesse = u64::from(ordnung.u32(bytes, bild + 4).ok_or_else(falsch)?);
        let ab = u64::from(ordnung.u32(bytes, bild + 8).ok_or_else(falsch)?);
        let anfang = tiff.checked_add(ab).ok_or_else(falsch)?;
        if !port.enthaelt(anfang, groesse) {
            return metadaten("MPF-Eintrag nicht ganz in der Datei");
        }
        bilder.push((anfang, groesse));
    }
    Ok(bilder)
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

    /// Erstes Bild mit `APP2` `MPF`: `tags` gleiche `MP_ENTRY`-Felder über einen Index mit dem ersten Bild und den
    /// `weitere` (Größe, Versatz ab dem TIFF-Kopf); der TIFF-Kopf steht bei Byte 10.
    fn erstes_bild(tags: usize, weitere: &[(u32, u32)]) -> Vec<u8> {
        let daten_ab = (8 + 2 + tags * 12 + 4) as u32;
        let zahl = ((weitere.len() + 1) * EINTRAG) as u32;
        let mut mpf = b"MPF\0MM\0*\0\0\0\x08".to_vec();
        mpf.extend_from_slice(&(tags as u16).to_be_bytes());
        for _ in 0..tags {
            mpf.extend_from_slice(&TAG_MP_ENTRY.to_be_bytes());
            mpf.extend_from_slice(&7u16.to_be_bytes());
            mpf.extend_from_slice(&zahl.to_be_bytes());
            mpf.extend_from_slice(&daten_ab.to_be_bytes());
        }
        mpf.extend_from_slice(&0u32.to_be_bytes());
        mpf.extend_from_slice(&[0; 16]);
        for (groesse, versatz) in weitere {
            mpf.extend_from_slice(&[0; 4]);
            mpf.extend_from_slice(&groesse.to_be_bytes());
            mpf.extend_from_slice(&versatz.to_be_bytes());
            mpf.extend_from_slice(&[0; 4]);
        }
        [
            vec![0xFF, 0xD8],
            segment(0xE2, &mpf),
            segment(0xDA, b"\x01"),
            b"Bild\xFF\xD9".to_vec(),
        ]
        .concat()
    }

    /// Ein winziges JPEG von 64 Bytes mit Ortstext im `COM`.
    fn winzig() -> Vec<u8> {
        let mut kommentar = b"GPS 48.1 11.5".to_vec();
        kommentar.resize(50, b'.');
        let bild = [
            vec![0xFF, 0xD8],
            segment(0xFE, &kommentar),
            segment(0xDA, b"\x01"),
            vec![0xAB, 0xFF, 0xD9],
        ]
        .concat();
        assert_eq!(bild.len(), 64);
        bild
    }

    /// Erstes Bild, dahinter `bilder`; der Index nennt je `nennungen` die Bilder in dieser Reihenfolge.
    fn mit_bildern(tags: usize, bilder: &[Vec<u8>], nennungen: &[usize]) -> Vec<u8> {
        let laenge = erstes_bild(tags, &vec![(0, 0); nennungen.len()]).len();
        let mut anfaenge = Vec::new();
        let mut stelle = laenge;
        for bild in bilder {
            anfaenge.push(stelle);
            stelle += bild.len();
        }
        let weitere: Vec<(u32, u32)> = nennungen
            .iter()
            .map(|&nummer| (bilder[nummer].len() as u32, (anfaenge[nummer] - 10) as u32))
            .collect();
        [vec![erstes_bild(tags, &weitere)], bilder.to_vec()]
            .concat()
            .concat()
    }

    fn tilgen_in_2_s(bytes: Vec<u8>) -> (Ergebnis<()>, Datei, u64) {
        let datei = Datei::mit(bytes);
        let beginn = std::time::Instant::now();
        let (ergebnis, verbraucht) = {
            let port = Port::neu(&datei).expect("Port");
            (
                jpeg_mit_mpf(&port, &mut Zaehler::default()),
                port.verbraucht(),
            )
        };
        assert!(
            beginn.elapsed().as_secs_f64() <= 2.0,
            "{:?}",
            beginn.elapsed()
        );
        (ergebnis, datei, verbraucht)
    }

    fn grund(ergebnis: &Ergebnis<()>) -> Option<&'static str> {
        match ergebnis {
            Err(Abbruch::Metadaten(grund)) => Some(grund),
            _ => None,
        }
    }

    /// ADR-0047 B1 P3: 4.000 Einträge auf dasselbe Bild → 422 binnen 2 s.
    #[test]
    fn viertausend_eintraege_auf_dasselbe_bild_ueberschneiden_sich() {
        let (ergebnis, datei, _) = tilgen_in_2_s(mit_bildern(1, &[winzig()], &[0; 4_000]));

        assert_eq!(grund(&ergebnis), Some("Verweise überschneiden sich"));
        assert!(enthaelt(&datei.inhalt(), b"GPS 48.1"), "nichts geschrieben");
    }

    /// ADR-0047 B1 P3: 4.000 Einträge auf verschiedene Bilder je 64 Bytes → getilgt binnen 2 s, gezählte Bytes
    /// ≤ 2 × Länge + 8 MiB.
    #[test]
    fn viertausend_verschiedene_winzige_bilder_werden_im_budget_getilgt() {
        let bilder = vec![winzig(); 4_000];
        let nennungen: Vec<usize> = (0..4_000).rev().collect();
        let bytes = mit_bildern(1, &bilder, &nennungen);
        let laenge = bytes.len() as u64;

        let (ergebnis, datei, verbraucht) = tilgen_in_2_s(bytes);

        assert!(ergebnis.is_ok(), "{:?}", grund(&ergebnis));
        assert!(verbraucht <= 2 * laenge + 8 * 1_048_576, "{verbraucht}");
        assert!(!enthaelt(&datei.inhalt(), b"GPS"));
    }

    /// ADR-0047 B1 P3: zwei `MP_ENTRY`-Felder → 422.
    #[test]
    fn zwei_mp_entry_felder_sind_doppelte_verweise() {
        let (ergebnis, datei, _) = tilgen_in_2_s(mit_bildern(2, &[winzig()], &[0]));

        assert_eq!(grund(&ergebnis), Some("Verweise doppelt"));
        assert!(enthaelt(&datei.inhalt(), b"GPS 48.1"), "nichts geschrieben");
    }

    /// Zwei Bilder, die sich um ein Byte überschneiden → 422, bevor ein Bild getilgt wird.
    #[test]
    fn um_ein_byte_ueberschneidende_bilder_sind_nicht_tilgbar() {
        let mut bytes = mit_bildern(1, &[winzig(), winzig()], &[0, 1]);
        let mpf = bytes.windows(4).position(|f| f == b"MPF\0").expect("MPF");
        let groesse = mpf + 4 + 26 + 16 + 4;
        bytes[groesse..groesse + 4].copy_from_slice(&65u32.to_be_bytes());

        let (ergebnis, datei, _) = tilgen_in_2_s(bytes);

        assert_eq!(grund(&ergebnis), Some("Verweise überschneiden sich"));
        assert!(enthaelt(&datei.inhalt(), b"GPS 48.1"), "nichts geschrieben");
    }

    /// ADR-0047 B1 P4 zweiter Teil: ein Leser im MPF-Bild mit Größe 64 liest nie über Anfang + 64 hinaus — auch
    /// nicht, wenn eine Segmentlänge darüber hinaus zeigt.
    #[test]
    fn ein_bild_mit_groesse_64_wird_nur_in_seinem_bereich_gelesen() {
        let mut bytes = vec![0x55; 2 * 1_048_576];
        bytes[1_000..1_064].copy_from_slice(&winzig());
        let datei = Datei::mit(bytes.clone());
        let port = Port::neu(&datei).expect("Port");

        jpeg(&port, 1_000, 1_064, &mut Zaehler::default()).expect("getilgt");

        assert!(datei.groesste_lesung() <= 64, "{}", datei.groesste_lesung());
        bytes[1_004..1_006].copy_from_slice(&200u16.to_be_bytes());
        let datei = Datei::mit(bytes);
        let port = Port::neu(&datei).expect("Port");

        let ergebnis = jpeg(&port, 1_000, 1_064, &mut Zaehler::default());

        assert!(matches!(ergebnis, Err(Abbruch::Metadaten(_))));
        assert!(datei.groesste_lesung() <= 64, "{}", datei.groesste_lesung());
    }
}
