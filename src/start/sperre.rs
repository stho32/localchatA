//! Ordner sperren: genau eine Instanz je Ordner über `<ordner>/.localchatA.sperre` (ADR-0005).

use super::Startfehler;
use std::fs::{File, TryLockError};
use std::path::Path;

pub const SPERRDATEI: &str = ".localchatA.sperre";

/// Hält die Sperre, solange sie lebt; das Betriebssystem gibt sie mit dem Prozess frei.
#[derive(Debug)]
pub struct Sperre {
    _datei: File,
}

pub fn ordner_sperren(ordner: &Path) -> Result<Sperre, Startfehler> {
    let sperrfehler = |grund: String| Startfehler::Sperre {
        ordner: ordner.to_path_buf(),
        grund,
    };
    let datei = File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(ordner.join(SPERRDATEI))
        .map_err(|fehler| sperrfehler(fehler.to_string()))?;
    match datei.try_lock() {
        Ok(()) => Ok(Sperre { _datei: datei }),
        Err(TryLockError::WouldBlock) => Err(sperrfehler(
            "bereits von einer Instanz gesperrt".to_string(),
        )),
        Err(TryLockError::Error(fehler)) => Err(sperrfehler(fehler.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn die_erste_sperre_gelingt_und_legt_die_sperrdatei_an() {
        let ordner = tempfile::tempdir().expect("Wegwerf-Ordner");

        let _sperre = ordner_sperren(ordner.path()).expect("gesperrt");

        assert!(ordner.path().join(SPERRDATEI).is_file());
    }

    #[test]
    fn eine_zweite_sperre_auf_demselben_ordner_scheitert_und_nennt_ordner_und_sperre() {
        let ordner = tempfile::tempdir().expect("Wegwerf-Ordner");
        let _erste = ordner_sperren(ordner.path()).expect("gesperrt");

        let fehler = ordner_sperren(ordner.path()).expect_err("zweite Sperre");

        assert!(matches!(&fehler, Startfehler::Sperre { ordner: o, .. } if o == ordner.path()));
        let text = fehler.to_string();
        assert!(
            text.contains(&ordner.path().display().to_string()),
            "{text}"
        );
        assert!(
            text.contains("bereits von einer Instanz gesperrt"),
            "{text}"
        );
    }

    #[test]
    fn nach_dem_freigeben_gelingt_die_sperre_trotz_liegender_sperrdatei() {
        let ordner = tempfile::tempdir().expect("Wegwerf-Ordner");
        drop(ordner_sperren(ordner.path()).expect("gesperrt"));
        assert!(ordner.path().join(SPERRDATEI).is_file());

        assert!(ordner_sperren(ordner.path()).is_ok());
    }
}
