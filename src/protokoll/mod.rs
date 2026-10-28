//! Betriebsprotokoll (A00015, ADR-0040 bis ADR-0043): eine escapte Zeile je Eintrag in `logs/<lokales Datum>.log`,
//! ab Warning dieselbe Zeile auf stderr.

pub mod senke;
pub mod zeile;

pub use zeile::{Bereich, Eintrag, Kontext, Schluessel, Stufe};

use crate::uhr::Uhr;
use senke::{Senke, Tagesdateien};
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};

/// Wohin Zeilen auf stderr gehen — im Programm stderr, im Test eine Aufzeichnung.
pub type Spiegel = Arc<dyn Fn(&str) + Send + Sync>;

/// Das Betriebsprotokoll einer Instanz; geteilt als `Arc<Protokoll>`, nie global (außer im Panik-Haken).
pub struct Protokoll {
    uhr: Arc<dyn Uhr>,
    mindeststufe: Stufe,
    spiegel: Spiegel,
    inneres: Mutex<Inneres>,
}

struct Inneres {
    senke: Box<dyn Senke>,
}

impl Protokoll {
    /// Das Protokoll unter `<ordner>/logs/`; scheitert nie.
    pub fn oeffnen(ordner: &Path, uhr: Arc<dyn Uhr>, mindeststufe: Stufe) -> Protokoll {
        let dateien = Tagesdateien::neu(ordner);
        let _ = dateien.ordner_anlegen();
        Protokoll::mit(Box::new(dateien), stderr_spiegel(), uhr, mindeststufe)
    }

    /// Ein Protokoll über eine beliebige Senke und einen beliebigen Spiegel (Tests).
    pub fn mit(
        senke: Box<dyn Senke>,
        spiegel: Spiegel,
        uhr: Arc<dyn Uhr>,
        mindeststufe: Stufe,
    ) -> Protokoll {
        Protokoll {
            uhr,
            mindeststufe,
            spiegel,
            inneres: Mutex::new(Inneres { senke }),
        }
    }

    pub fn mindeststufe(&self) -> Stufe {
        self.mindeststufe
    }

    /// Schreibt einen Eintrag in die Datei des lokalen Datums seiner Zeit; unter der Mindeststufe nichts;
    /// ab Warning zusätzlich auf stderr.
    pub fn eintragen(
        &self,
        stufe: Stufe,
        bereich: Bereich,
        text: impl Into<String>,
        kontext: Kontext,
    ) {
        if stufe < self.mindeststufe {
            return;
        }
        let jetzt = self.uhr.jetzt();
        let zeile = zeile::zeile(&Eintrag::neu(stufe, bereich, text, kontext), jetzt);
        let tag = jetzt.date_naive();
        {
            let mut inneres = self.inneres.lock().unwrap_or_else(PoisonError::into_inner);
            let _ = inneres.senke.anhaengen(tag, &zeile);
        }
        if stufe >= Stufe::Warning {
            (self.spiegel)(&zeile);
        }
    }
}

/// stderr des Prozesses; eine Zeile je Aufruf in einem `write_all`.
fn stderr_spiegel() -> Spiegel {
    Arc::new(|zeile: &str| {
        let _ = std::io::stderr().lock().write_all(zeile.as_bytes());
    })
}

#[cfg(test)]
pub(crate) mod testhilfe {
    use super::*;
    use crate::uhr::Zeit;

    pub fn zeit(text: &str) -> Zeit {
        Zeit::parse_from_rfc3339(text).expect("Testzeit")
    }

    /// Aufgezeichnete stderr-Zeilen (ohne Zeilenende).
    #[derive(Clone, Default)]
    pub struct Aufzeichnung(pub Arc<Mutex<Vec<String>>>);

    impl Aufzeichnung {
        pub fn spiegel(&self) -> Spiegel {
            let zeilen = Arc::clone(&self.0);
            Arc::new(move |zeile: &str| {
                zeilen
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .push(zeile.trim_end_matches('\n').to_string());
            })
        }

        pub fn zeilen(&self) -> Vec<String> {
            self.0
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::testhilfe::*;
    use super::*;
    use crate::uhr::GestellteUhr;

    struct InDateien {
        _wurzel: tempfile::TempDir,
        ordner: std::path::PathBuf,
        protokoll: Protokoll,
        uhr: Arc<GestellteUhr>,
        stderr: Aufzeichnung,
    }

    fn in_dateien(start: &str, mindeststufe: Stufe) -> InDateien {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let ordner = wurzel.path().to_path_buf();
        let uhr = Arc::new(GestellteUhr::neu(zeit(start)));
        let stderr = Aufzeichnung::default();
        let protokoll = Protokoll::mit(
            Box::new(Tagesdateien::neu(&ordner)),
            stderr.spiegel(),
            Arc::clone(&uhr) as Arc<dyn Uhr>,
            mindeststufe,
        );
        InDateien {
            _wurzel: wurzel,
            ordner,
            protokoll,
            uhr,
            stderr,
        }
    }

    impl InDateien {
        fn um(&self, zeitpunkt: &str, text: &str) {
            self.uhr.stellen(zeit(zeitpunkt));
            self.protokoll
                .eintragen(Stufe::Information, Bereich::Start, text, Kontext::neu());
        }

        fn datei(&self, name: &str) -> Vec<String> {
            std::fs::read_to_string(self.ordner.join("logs").join(name))
                .unwrap_or_default()
                .lines()
                .map(str::to_string)
                .collect()
        }
    }

    #[test]
    fn eintraege_um_mitternacht_liegen_in_der_datei_ihres_lokalen_tages() {
        let p = in_dateien("2026-10-28T12:00:00+01:00", Stufe::Information);
        p.um("2026-10-28T23:59:59.999+01:00", "vorher");
        p.um("2026-10-29T00:00:00.000+01:00", "nachher");

        assert_eq!(
            p.datei("2026-10-28.log"),
            ["2026-10-28T23:59:59.999+01:00 INFORMATION start vorher"]
        );
        assert_eq!(
            p.datei("2026-10-29.log"),
            ["2026-10-29T00:00:00.000+01:00 INFORMATION start nachher"]
        );
    }

    #[test]
    fn rueckstellung_und_vorstellung_bleiben_im_selben_tag_in_schreibreihenfolge() {
        let p = in_dateien("2026-10-25T00:00:00+02:00", Stufe::Information);
        p.um("2026-10-25T02:30:00.000+02:00", "a");
        p.um("2026-10-25T02:10:00.000+01:00", "b");
        p.um("2026-03-29T01:59:59.999+01:00", "c");
        p.um("2026-03-29T03:00:00.000+02:00", "d");

        let texte = |name| {
            p.datei(name)
                .iter()
                .map(|zeile| zeile.rsplit(' ').next().unwrap_or_default().to_string())
                .collect::<Vec<_>>()
        };
        assert_eq!(texte("2026-10-25.log"), ["a", "b"]);
        assert_eq!(texte("2026-03-29.log"), ["c", "d"]);
    }

    #[test]
    fn rueckwaerts_ueber_mitternacht_haengt_an_die_aeltere_datei_an() {
        let p = in_dateien("2026-10-28T12:00:00+01:00", Stufe::Information);
        p.um("2026-10-28T23:00:00.000+01:00", "erst");
        p.um("2026-10-29T00:00:01.000+01:00", "neu");
        p.um("2026-10-28T23:59:59.000+01:00", "zurück");

        assert_eq!(p.datei("2026-10-28.log").len(), 2);
        assert!(p.datei("2026-10-28.log")[1].ends_with("zurück"));
        assert_eq!(p.datei("2026-10-29.log").len(), 1);
    }

    #[test]
    fn das_datum_ist_das_lokale_nicht_das_utc_datum() {
        let p = in_dateien("2026-10-29T00:30:00.000+01:00", Stufe::Information);
        p.um("2026-10-29T00:30:00.000+01:00", "früh");

        assert_eq!(p.datei("2026-10-29.log").len(), 1);
        assert!(p.datei("2026-10-28.log").is_empty());
    }

    #[test]
    fn ab_warning_steht_dieselbe_zeile_auf_stderr_darunter_nie() {
        let p = in_dateien("2026-10-28T12:00:00+01:00", Stufe::Debug);
        for stufe in [
            Stufe::Debug,
            Stufe::Information,
            Stufe::Warning,
            Stufe::Error,
            Stufe::Exception,
        ] {
            p.protokoll
                .eintragen(stufe, Bereich::Laden, stufe.als_text(), Kontext::neu());
        }

        let datei = p.datei("2026-10-28.log");
        assert_eq!(datei.len(), 5);
        assert_eq!(p.stderr.zeilen(), datei[2..].to_vec());
    }

    #[test]
    fn unter_der_mindeststufe_wird_nichts_geschrieben() {
        let p = in_dateien("2026-10-28T12:00:00+01:00", Stufe::Warning);
        p.protokoll
            .eintragen(Stufe::Information, Bereich::Start, "leise", Kontext::neu());
        p.protokoll
            .eintragen(Stufe::Warning, Bereich::Start, "laut", Kontext::neu());

        assert_eq!(p.datei("2026-10-28.log").len(), 1);
    }

    #[test]
    fn einen_vom_betreiber_geloeschten_logs_ordner_legt_der_naechste_eintrag_neu_an() {
        let p = in_dateien("2026-10-28T12:00:00+01:00", Stufe::Information);
        p.um("2026-10-28T12:00:00+01:00", "eins");
        std::fs::remove_dir_all(p.ordner.join("logs")).expect("logs weg");
        p.um("2026-10-29T12:00:00+01:00", "zwei");

        assert_eq!(p.datei("2026-10-29.log").len(), 1);
    }
}
