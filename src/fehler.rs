//! Fehler der Fachbausteine (Aspekt X00004): ein Typ, übersetzt erst an der Grenze.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fehler {
    /// Eine Eingabe verletzt eine Regel; der Text nennt Wert und Regel.
    Ungueltig(String),
    /// Die Ablage im Ordner lässt sich nicht lesen oder schreiben; der Text nennt Ort und Grund.
    Ablage(String),
}

impl fmt::Display for Fehler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Fehler::Ungueltig(text) | Fehler::Ablage(text) => write!(f, "{text}"),
        }
    }
}
