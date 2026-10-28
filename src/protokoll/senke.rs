//! Wohin die Zeilen gehen: die Tagesdateien unter `logs/` (ADR-0040 Regel 2 und 7, ADR-0041 Regel 3, je mit
//! Ergänzung M1) oder im Test eine Speicher-Senke.

use chrono::NaiveDate;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

/// Nimmt fertige Zeilen für einen lokalen Tag entgegen.
pub trait Senke: Send {
    /// Hängt `zeile` an die Datei des Tages `tag` an; liefert die Größe dieser Datei danach.
    fn anhaengen(&mut self, tag: NaiveDate, zeile: &str) -> io::Result<u64>;
    /// Die Datei, in die ein Eintrag des Tages gehen würde (für Meldungen, relativ zum Ordner).
    fn datei(&self, tag: NaiveDate) -> String;
    /// Der Ordner `logs/` der Tagesdateien; eine Speicher-Senke hat keinen.
    fn ordner(&self) -> Option<PathBuf> {
        None
    }
}

/// Die Tagesdateien `logs/JJJJ-MM-TT.log`; jeder Eintrag öffnet seine Datei, schreibt und schließt sie — keine
/// Datei bleibt über einen Eintrag hinaus offen, das Datum bestimmt den Pfad, sonst gibt es keinen Zustand
/// (ADR-0040, Ergänzung „gelöschte Tagesdatei“, Option C).
pub struct Tagesdateien {
    logs: PathBuf,
}

/// Name des Ordners im Instanzordner.
pub const LOGS: &str = "logs";

impl Tagesdateien {
    pub fn neu(ordner: &Path) -> Tagesdateien {
        Tagesdateien {
            logs: ordner.join(LOGS),
        }
    }

    /// Legt `logs/` an, falls es fehlt.
    pub fn ordner_anlegen(&self) -> io::Result<()> {
        std::fs::create_dir_all(&self.logs)
    }

    /// Öffnet die Datei des Tages (append, create); fehlt `logs/`, wird es einmal angelegt und erneut versucht
    /// (ADR-0040, Ergänzung Festlegung (4)).
    fn oeffnen(&self, tag: NaiveDate) -> io::Result<File> {
        let pfad = self.logs.join(dateiname(tag));
        match oeffnen_zum_anhaengen(&pfad) {
            Err(fehler) if fehler.kind() == io::ErrorKind::NotFound => {
                self.ordner_anlegen()?;
                oeffnen_zum_anhaengen(&pfad)
            }
            ergebnis => ergebnis,
        }
    }
}

fn oeffnen_zum_anhaengen(pfad: &Path) -> io::Result<File> {
    OpenOptions::new().append(true).create(true).open(pfad)
}

/// `JJJJ-MM-TT.log`
pub fn dateiname(tag: NaiveDate) -> String {
    format!("{}.log", tag.format("%Y-%m-%d"))
}

impl Senke for Tagesdateien {
    /// Öffnen (append, create) → ein `write_all` der Zeile → Größe aus den Metadaten des Handles → schließen
    /// (mit dem Ende des Aufrufs).
    fn anhaengen(&mut self, tag: NaiveDate, zeile: &str) -> io::Result<u64> {
        let mut datei = self.oeffnen(tag)?;
        datei.write_all(zeile.as_bytes())?;
        Ok(datei.metadata()?.len())
    }

    fn datei(&self, tag: NaiveDate) -> String {
        format!("{LOGS}/{}", dateiname(tag))
    }

    fn ordner(&self) -> Option<PathBuf> {
        Some(self.logs.clone())
    }
}

/// Speicher-Senke für Tests: hält je Zeile Tag und Text; `scheitern` lässt jedes Anhängen scheitern.
#[derive(Clone, Default)]
pub struct Speicher {
    zeilen: Arc<Mutex<Vec<(NaiveDate, String)>>>,
    scheitern: Arc<Mutex<bool>>,
}

impl Speicher {
    pub fn neu() -> Speicher {
        Speicher::default()
    }

    /// Alle angehängten Zeilen ohne Zeilenende, in Schreibreihenfolge.
    pub fn zeilen(&self) -> Vec<String> {
        self.zeilen
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .map(|(_, zeile)| zeile.trim_end_matches('\n').to_string())
            .collect()
    }

    pub fn scheitern_lassen(&self, scheitern: bool) {
        *self
            .scheitern
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = scheitern;
    }
}

impl Senke for Speicher {
    fn anhaengen(&mut self, tag: NaiveDate, zeile: &str) -> io::Result<u64> {
        if *self
            .scheitern
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
        {
            return Err(io::Error::other("Speicher-Senke scheitert"));
        }
        let mut zeilen = self.zeilen.lock().unwrap_or_else(PoisonError::into_inner);
        zeilen.push((tag, zeile.to_string()));
        Ok(zeilen
            .iter()
            .filter(|(gesetzt, _)| *gesetzt == tag)
            .map(|(_, zeile)| zeile.len() as u64)
            .sum())
    }

    fn datei(&self, tag: NaiveDate) -> String {
        format!("{LOGS}/{}", dateiname(tag))
    }
}

/// ADR-0040, Ergänzung „gelöschte Tagesdatei“ (Observability-Review #24, M1; Code-Review #24, B2), am selben Tag:
/// was die Betreiberin im laufenden Tag mit `heute.log` oder `logs/` tut, wirkt beim nächsten Eintrag.
#[cfg(test)]
mod tests {
    use super::*;

    const ZEILE_A: &str = "a\n";

    struct Probe {
        _wurzel: tempfile::TempDir,
        logs: PathBuf,
        senke: Tagesdateien,
        tag: NaiveDate,
    }

    /// Ein erster Eintrag am 2026-10-28 steht schon in `logs/2026-10-28.log`.
    fn nach_dem_ersten_eintrag() -> Probe {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let mut senke = Tagesdateien::neu(wurzel.path());
        let tag = NaiveDate::from_ymd_opt(2026, 10, 28).expect("Tag");
        senke.anhaengen(tag, ZEILE_A).expect("erster Eintrag");
        Probe {
            logs: wurzel.path().join(LOGS),
            _wurzel: wurzel,
            senke,
            tag,
        }
    }

    impl Probe {
        fn heute(&self) -> PathBuf {
            self.logs.join("2026-10-28.log")
        }
    }

    #[test]
    fn eine_geloeschte_tagesdatei_legt_der_naechste_eintrag_desselben_tages_neu_an() {
        let mut p = nach_dem_ersten_eintrag();
        std::fs::remove_file(p.heute()).expect("gelöscht");

        let groesse = p.senke.anhaengen(p.tag, "b\n").expect("Eintrag");

        assert_eq!(std::fs::read_to_string(p.heute()).expect("neu da"), "b\n");
        assert_eq!(groesse, 2);
    }

    #[test]
    fn ein_geloeschter_logs_ordner_entsteht_beim_naechsten_eintrag_desselben_tages_neu() {
        let mut p = nach_dem_ersten_eintrag();
        std::fs::remove_dir_all(&p.logs).expect("logs weg");

        let groesse = p.senke.anhaengen(p.tag, "c\n").expect("Eintrag");

        assert_eq!(std::fs::read_to_string(p.heute()).expect("neu da"), "c\n");
        assert_eq!(groesse, 2);
    }

    #[test]
    fn nach_dem_verschieben_geht_der_naechste_eintrag_in_eine_neue_tagesdatei() {
        let mut p = nach_dem_ersten_eintrag();
        let alt = p.logs.join("alt.log");
        std::fs::rename(p.heute(), &alt).expect("verschoben");

        p.senke.anhaengen(p.tag, "d\n").expect("Eintrag");

        assert_eq!(std::fs::read_to_string(p.heute()).expect("neu da"), "d\n");
        assert_eq!(std::fs::read_to_string(&alt).expect("alt"), ZEILE_A);
    }

    #[test]
    fn nach_dem_abschneiden_ist_die_groesse_die_der_neuen_zeile() {
        let mut p = nach_dem_ersten_eintrag();
        std::fs::File::create(p.heute()).expect("auf 0 Bytes abgeschnitten");

        let groesse = p.senke.anhaengen(p.tag, "efg\n").expect("Eintrag");

        assert_eq!(groesse, 4);
        assert_eq!(std::fs::read_to_string(p.heute()).expect("da"), "efg\n");
    }
}
