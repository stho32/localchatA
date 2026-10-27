//! Fehler der Fachbausteine (Aspekt X00004): ein Typ, übersetzt erst an der Grenze.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fehler {
    /// Eine Eingabe verletzt eine Regel; der Text nennt Wert und Regel.
    Ungueltig(String),
}

impl fmt::Display for Fehler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Fehler::Ungueltig(text) => write!(f, "{text}"),
        }
    }
}
