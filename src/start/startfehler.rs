//! Was den Start scheitern lässt: Ordner, Port, Sperre — je mit dem Ort, der gemeint ist.

use std::fmt;
use std::path::PathBuf;

#[derive(Debug, PartialEq, Eq)]
pub enum Startfehler {
    Ordner {
        ordner: PathBuf,
        grund: String,
    },
    Port {
        host: String,
        port: u16,
        grund: String,
    },
    Sperre {
        ordner: PathBuf,
        grund: String,
    },
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
            Startfehler::Port { host, port, grund } => {
                write!(f, "Port {port} auf {host} lässt sich nicht binden: {grund}")
            }
            Startfehler::Sperre { ordner, grund } => {
                write!(
                    f,
                    "Ordner {} lässt sich nicht sperren: {grund}",
                    ordner.display()
                )
            }
        }
    }
}
