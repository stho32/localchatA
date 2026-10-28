//! Wohin die Zeilen gehen: die Tagesdateien unter `logs/` (ADR-0040 Regel 2, ADR-0041 Regeln 2–3) oder im Test
//! eine Speicher-Senke.

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

/// Die Tagesdateien `logs/JJJJ-MM-TT.log`; die Datei des zuletzt geschriebenen Tages bleibt offen.
pub struct Tagesdateien {
    logs: PathBuf,
    offen: Option<OffeneDatei>,
}

struct OffeneDatei {
    tag: NaiveDate,
    datei: File,
    groesse: u64,
}

/// Name des Ordners im Instanzordner.
pub const LOGS: &str = "logs";

impl Tagesdateien {
    pub fn neu(ordner: &Path) -> Tagesdateien {
        Tagesdateien {
            logs: ordner.join(LOGS),
            offen: None,
        }
    }

    /// Legt `logs/` an, falls es fehlt.
    pub fn ordner_anlegen(&self) -> io::Result<()> {
        std::fs::create_dir_all(&self.logs)
    }

    /// Öffnet die Datei des Tages (append, create); fehlt `logs/`, wird es einmal angelegt und erneut versucht
    /// (ADR-0040, Ergänzung Festlegung (4)).
    fn oeffnen(&self, tag: NaiveDate) -> io::Result<OffeneDatei> {
        let pfad = self.logs.join(dateiname(tag));
        let datei = match oeffnen_zum_anhaengen(&pfad) {
            Err(fehler) if fehler.kind() == io::ErrorKind::NotFound => {
                self.ordner_anlegen()?;
                oeffnen_zum_anhaengen(&pfad)?
            }
            ergebnis => ergebnis?,
        };
        let groesse = datei.metadata()?.len();
        Ok(OffeneDatei {
            tag,
            datei,
            groesse,
        })
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
    fn anhaengen(&mut self, tag: NaiveDate, zeile: &str) -> io::Result<u64> {
        let passend = self.offen.as_ref().is_some_and(|offen| offen.tag == tag);
        if !passend {
            self.offen = None;
            self.offen = Some(self.oeffnen(tag)?);
        }
        let Some(offen) = self.offen.as_mut() else {
            return Err(io::Error::other("keine Tagesdatei offen"));
        };
        if let Err(fehler) = offen.datei.write_all(zeile.as_bytes()) {
            self.offen = None;
            return Err(fehler);
        }
        offen.groesse += zeile.len() as u64;
        Ok(offen.groesse)
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
