//! Werkbank (B0254, ADR-0039 Regel 4): eine gemeinsame Semaphore für schwere Arbeit an der HTTP-Grenze —
//! Verlaufsblöcke, später große Körper (B0211), Suche und Anhang-Metadaten. Erlaubnisse = Zahl der Kerne,
//! mindestens 2. Die Erlaubnis wird vor `spawn_blocking` erworben und lebt bis zum Ende des Blocking-Teils;
//! so entstehen höchstens so viele Arbeitsspitzen (und Arenen des Allokators) zugleich, wie es Plätze gibt.

use super::abbruch::{Abbruchart, Abgebrochen};
use std::sync::Arc;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

/// Wenigstens so viele Erlaubnisse, damit ein langer Block die Werkbank nicht allein belegt.
const MINDESTENS: usize = 2;

/// Erlaubnisse der Werkbank aus der Zahl der Kerne, mindestens 2 (rein).
pub fn erlaubnisse_aus_kernen(kerne: usize) -> usize {
    kerne.max(MINDESTENS)
}

/// Die Werkbank; Klone teilen dieselben Erlaubnisse.
#[derive(Debug, Clone)]
pub struct Werkbank {
    erlaubnisse: Arc<Semaphore>,
}

impl Werkbank {
    /// So viele Erlaubnisse, wie der Prozess Kerne nutzen darf (`available_parallelism`), mindestens 2.
    pub fn aus_kernen() -> Werkbank {
        let kerne = std::thread::available_parallelism().map_or(1, usize::from);
        Werkbank::mit(erlaubnisse_aus_kernen(kerne))
    }

    /// Genau so viele Erlaubnisse (Tests).
    pub fn mit(erlaubnisse: usize) -> Werkbank {
        Werkbank {
            erlaubnisse: Arc::new(Semaphore::new(erlaubnisse)),
        }
    }

    /// Wartet auf eine Erlaubnis; sie gilt, bis sie fallen gelassen wird.
    pub async fn erwerben(&self) -> Result<OwnedSemaphorePermit, Abgebrochen> {
        Arc::clone(&self.erlaubnisse)
            .acquire_owned()
            .await
            .map_err(|fehler| Abgebrochen {
                art: Abbruchart::Geschlossen,
                ursache: format!("Werkbank geschlossen — {fehler}"),
            })
    }

    /// Erwirbt eine Erlaubnis und führt `arbeit` mit ihr auf einem Blocking-Thread aus.
    pub async fn ausfuehren<T, F>(&self, arbeit: F) -> Result<T, Abgebrochen>
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        let erlaubnis = self.erwerben().await?;
        tokio::task::spawn_blocking(move || {
            let ergebnis = arbeit();
            drop(erlaubnis);
            ergebnis
        })
        .await
        .map_err(Abgebrochen::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn erlaubnisse_sind_die_kerne_mindestens_zwei() {
        assert_eq!(erlaubnisse_aus_kernen(1), 2);
        assert_eq!(erlaubnisse_aus_kernen(2), 2);
        assert_eq!(erlaubnisse_aus_kernen(8), 8);
    }

    #[test]
    fn aus_kernen_hat_mindestens_zwei_erlaubnisse() {
        let werkbank = Werkbank::aus_kernen();

        assert!(werkbank.erlaubnisse.available_permits() >= 2);
    }

    #[tokio::test]
    async fn die_erlaubnis_lebt_bis_zum_ende_der_arbeit() {
        let werkbank = Werkbank::mit(1);

        let frei_waehrend = {
            let innen = werkbank.clone();
            werkbank
                .ausfuehren(move || innen.erlaubnisse.available_permits())
                .await
                .expect("gelaufen")
        };

        assert_eq!(frei_waehrend, 0);
        assert_eq!(werkbank.erlaubnisse.available_permits(), 1);
    }
}
