//! Ende-Signal für alle (B0078, ADR-0010 Folgeaufgabe): der Start meldet das Ende einmal (SIGTERM, Strg+C),
//! Annahmeschleife, Takt und jeder Strom hören es und enden selbst.

use tokio::sync::watch;

/// Hört das Ende des Programms; beliebig oft klonbar.
#[derive(Debug, Clone)]
pub struct Ende(watch::Receiver<bool>);

/// Meldet das Ende allen, die es hören.
#[derive(Debug)]
pub struct EndeMelden(watch::Sender<bool>);

impl Ende {
    pub fn neu() -> (EndeMelden, Ende) {
        let (melden, hoeren) = watch::channel(false);
        (EndeMelden(melden), Ende(hoeren))
    }

    /// Ein Ende, das nie gemeldet wird (Testserver): der Melder lebt bis zum Ende des Testprozesses.
    #[cfg(test)]
    pub fn nie() -> Ende {
        let (melden, ende) = Ende::neu();
        std::mem::forget(melden);
        ende
    }

    /// Ist das Ende schon gemeldet?
    pub fn gemeldet(&self) -> bool {
        *self.0.borrow()
    }

    /// Kehrt zurück, sobald das Ende gemeldet ist. Ist der Melder fort, ohne gemeldet zu haben, kommt das
    /// Ende nie — dann wartet es für immer statt still „beendet“ zu melden.
    pub async fn abwarten(&mut self) {
        if self.0.wait_for(|beendet| *beendet).await.is_err() {
            std::future::pending::<()>().await;
        }
    }
}

impl EndeMelden {
    pub fn melden(&self) {
        let _ = self.0.send(true);
    }
}
