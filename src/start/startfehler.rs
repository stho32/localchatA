//! Was den Start scheitern lässt: Ordner, Port, Sperre — je mit dem Ort, der gemeint ist.

use std::fmt;
use std::path::PathBuf;

#[derive(Debug, PartialEq, Eq)]
pub enum Startfehler {
    Ordner { ordner: PathBuf, grund: String },
}

impl fmt::Display for Startfehler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Startfehler::Ordner { ordner, grund } => {
                write!(
                    f,
                    "Ordner {} lässt sich nicht anlegen: {grund}",
                    ordner.display()
                )
            }
        }
    }
}
