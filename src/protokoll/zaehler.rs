//! Zähler für den Health-Check (ADR-0043 Regel 5 mit Ergänzung): Warnings, Errors und Exceptions seit dem Start,
//! gezählt beim Entstehen — unabhängig von Mindeststufe, Budget und Rückfall — und die Zeit des jüngsten Errors
//! und der jüngsten Exception.

use super::Stufe;
use crate::uhr::Zeit;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};

#[derive(Default)]
pub struct Zaehler {
    warnings: AtomicU64,
    errors: AtomicU64,
    exceptions: AtomicU64,
    letzter_error: Mutex<Option<Zeit>>,
    letzte_exception: Mutex<Option<Zeit>>,
}

/// Stand der Zähler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Zaehlerstand {
    pub warnings: u64,
    pub errors: u64,
    pub exceptions: u64,
    pub letzter_error: Option<Zeit>,
    pub letzte_exception: Option<Zeit>,
}

impl Zaehler {
    /// Zählt einen Eintrag der Stufe `stufe`, der um `jetzt` entsteht; Debug und Information zählen nicht.
    pub fn zaehlen(&self, stufe: Stufe, jetzt: Zeit) {
        match stufe {
            Stufe::Debug | Stufe::Information => {}
            Stufe::Warning => {
                self.warnings.fetch_add(1, Ordering::SeqCst);
            }
            Stufe::Error => {
                self.errors.fetch_add(1, Ordering::SeqCst);
                *self
                    .letzter_error
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner) = Some(jetzt);
            }
            Stufe::Exception => {
                self.exceptions.fetch_add(1, Ordering::SeqCst);
                *self
                    .letzte_exception
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner) = Some(jetzt);
            }
        }
    }

    pub fn stand(&self) -> Zaehlerstand {
        Zaehlerstand {
            warnings: self.warnings.load(Ordering::SeqCst),
            errors: self.errors.load(Ordering::SeqCst),
            exceptions: self.exceptions.load(Ordering::SeqCst),
            letzter_error: *self
                .letzter_error
                .lock()
                .unwrap_or_else(PoisonError::into_inner),
            letzte_exception: *self
                .letzte_exception
                .lock()
                .unwrap_or_else(PoisonError::into_inner),
        }
    }
}

/// Arten abgewiesener JSON-Körper (ADR-0043, Ergänzung).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Abweisungsart {
    ZuGross,
    KeinJson,
    Kaputt,
    Abgebrochen,
}

/// Abgewiesene Körper aller JSON-Routen seit dem Start, gezählt beim Entstehen.
#[derive(Default)]
pub struct Abgewiesen {
    zu_gross: AtomicU64,
    kein_json: AtomicU64,
    kaputt: AtomicU64,
    abgebrochen: AtomicU64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct AbgewiesenStand {
    pub zu_gross: u64,
    pub kein_json: u64,
    pub kaputt: u64,
    pub abgebrochen: u64,
}

impl Abgewiesen {
    pub fn zaehlen(&self, art: Abweisungsart) {
        let zaehler = match art {
            Abweisungsart::ZuGross => &self.zu_gross,
            Abweisungsart::KeinJson => &self.kein_json,
            Abweisungsart::Kaputt => &self.kaputt,
            Abweisungsart::Abgebrochen => &self.abgebrochen,
        };
        zaehler.fetch_add(1, Ordering::SeqCst);
    }

    pub fn stand(&self) -> AbgewiesenStand {
        AbgewiesenStand {
            zu_gross: self.zu_gross.load(Ordering::SeqCst),
            kein_json: self.kein_json.load(Ordering::SeqCst),
            kaputt: self.kaputt.load(Ordering::SeqCst),
            abgebrochen: self.abgebrochen.load(Ordering::SeqCst),
        }
    }
}
