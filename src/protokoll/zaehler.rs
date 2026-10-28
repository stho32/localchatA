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
