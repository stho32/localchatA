//! Plattenauskunft (A00008 Ergänzung Karte #9, ADR-0046 Regel 1.5): wie viele Bytes das Dateisystem des Ordners
//! noch frei hat (`f_bavail × f_frsize` aus `statvfs` über `rustix`, Merkmal `fs`). Port mit Attrappe für Tests.

use std::io;
use std::path::PathBuf;

pub trait Plattenauskunft: Send + Sync {
    fn frei(&self) -> io::Result<u64>;
}

/// Das Dateisystem des Instanzordners.
pub struct Statvfs {
    ordner: PathBuf,
}

impl Statvfs {
    pub fn neu(ordner: PathBuf) -> Statvfs {
        Statvfs { ordner }
    }
}

impl Plattenauskunft for Statvfs {
    fn frei(&self) -> io::Result<u64> {
        let stand = rustix::fs::statvfs(&self.ordner)?;
        Ok(stand.f_bavail.saturating_mul(stand.f_frsize))
    }
}

/// Ohne Auskunft gilt die Platte als groß genug — nur, solange keine Instanz Anhänge schreibt (Tests ohne Platte).
pub struct Unbegrenzt;

impl Plattenauskunft for Unbegrenzt {
    fn frei(&self) -> io::Result<u64> {
        Ok(u64::MAX)
    }
}

/// Gestellte Auskunft für Tests.
#[cfg(test)]
pub struct Gestellt(pub std::sync::atomic::AtomicU64);

#[cfg(test)]
impl Plattenauskunft for Gestellt {
    fn frei(&self) -> io::Result<u64> {
        Ok(self.0.load(std::sync::atomic::Ordering::SeqCst))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Erster Test der Ergänzung A00008: `platz_frei` gegen einen Wegwerf-Ordner > 0; ein fehlender Ordner ist ein
    /// Fehler, keine Null.
    #[test]
    fn statvfs_liefert_freien_platz_des_ordners() {
        let ordner = tempfile::tempdir().expect("Wegwerf-Ordner");
        let frei = Statvfs::neu(ordner.path().to_path_buf())
            .frei()
            .expect("Auskunft");
        assert!(frei > 0);
        assert!(Statvfs::neu(ordner.path().join("fehlt")).frei().is_err());
    }
}
