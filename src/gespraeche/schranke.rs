//! Haken für feste Verschränkungen im Testbau (ADR-0048 Ergänzung „Karte #73“ K8): ein Vorgang wartet **vor seinen
//! Sperren** an einer Schranke mit zwei Parteien, zweimal — das erste Warten meldet dem Test, dass der Vorgang dort
//! steht, das zweite hält ihn, bis der Test ihn freigibt. Dazwischen läuft, was der Test verschränken will. Die
//! Schranke gilt für den nächsten Durchgang des Vorgangs und ist danach weg. Gibt es diese Datei nur im Testbau; im
//! Betrieb steht kein Haken im Kern, und kein Knopf der Umgebung setzt einen.

use std::sync::{Arc, Barrier, Mutex};

/// Die Vorgänge, vor deren Sperren eine Schranke stehen kann.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Vorgang {
    Bearbeiten,
    Loeschen,
    /// Je entfallendem Tag der Pflege, vor der Sperre des Gesprächs.
    Pflegen,
    /// Das Bereitstellen eines Direktgesprächs, vor der Sperre des Verzeichnisses (ADR-0055 Regel 4, W5).
    DirektAnlegen,
}

#[derive(Default)]
pub(super) struct Schranken {
    bearbeiten: Mutex<Option<Arc<Barrier>>>,
    loeschen: Mutex<Option<Arc<Barrier>>>,
    pflegen: Mutex<Option<Arc<Barrier>>>,
    direkt_anlegen: Mutex<Option<Arc<Barrier>>>,
}

impl Schranken {
    fn platz(&self, vorgang: Vorgang) -> &Mutex<Option<Arc<Barrier>>> {
        match vorgang {
            Vorgang::Bearbeiten => &self.bearbeiten,
            Vorgang::Loeschen => &self.loeschen,
            Vorgang::Pflegen => &self.pflegen,
            Vorgang::DirektAnlegen => &self.direkt_anlegen,
        }
    }

    /// Stellt die Schranke vor den nächsten Durchgang des Vorgangs; der Test wartet zweimal an ihr.
    pub(super) fn aufstellen(&self, vorgang: Vorgang) -> Arc<Barrier> {
        let schranke = Arc::new(Barrier::new(2));
        *super::sperren(self.platz(vorgang)) = Some(Arc::clone(&schranke));
        schranke
    }

    /// Vom Vorgang vor seinen Sperren gerufen: steht eine Schranke, zweimal warten; sonst nichts.
    pub(super) fn passieren(&self, vorgang: Vorgang) {
        let schranke = super::sperren(self.platz(vorgang)).take();
        if let Some(schranke) = schranke {
            schranke.wait();
            schranke.wait();
        }
    }
}
