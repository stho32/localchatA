//! Ordner vorbereiten: anlegen, wenn er fehlt, und als absoluten Pfad liefern.

use super::Startfehler;
use std::path::{Path, PathBuf};

pub fn ordner_vorbereiten(ordner: &Path) -> Result<PathBuf, Startfehler> {
    let ordnerfehler = |fehler: std::io::Error| Startfehler::Ordner {
        ordner: ordner.to_path_buf(),
        grund: fehler.to_string(),
    };
    std::fs::create_dir_all(ordner).map_err(ordnerfehler)?;
    ordner.canonicalize().map_err(ordnerfehler)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fehlender_ordner_wird_samt_eltern_angelegt_und_absolut_geliefert() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let ordner = wurzel.path().join("neu").join("lc-r1");

        let vorbereitet = ordner_vorbereiten(&ordner).expect("Ordner vorbereitet");

        assert!(ordner.is_dir());
        assert!(vorbereitet.is_absolute());
        assert_eq!(
            vorbereitet,
            ordner.canonicalize().expect("kanonischer Pfad")
        );
    }

    #[test]
    fn relativer_ordner_wird_zum_absoluten_pfad() {
        let vorbereitet = ordner_vorbereiten(Path::new(".")).expect("Ordner vorbereitet");
        let erwartet = std::env::current_dir().expect("Arbeitsverzeichnis");
        assert_eq!(
            vorbereitet,
            erwartet.canonicalize().expect("kanonischer Pfad")
        );
    }

    #[test]
    fn datei_im_weg_ergibt_einen_ordnerfehler_der_den_ordner_nennt() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let datei = wurzel.path().join("lc-datei");
        std::fs::write(&datei, "").expect("Datei angelegt");
        let ordner = datei.join("unter");

        let fehler = ordner_vorbereiten(&ordner).expect_err("Ordner darf nicht entstehen");

        assert!(matches!(&fehler, Startfehler::Ordner { ordner: o, .. } if *o == ordner));
        assert!(fehler.to_string().contains(&ordner.display().to_string()));
    }
}
