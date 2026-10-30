//! Der Nachholplatz eines Warte-Aufrufs (ADR-0054 Ergänzung „Nachholen von `warten` …“ Regel 1 und 9, B0756): das
//! Nachholen läuft nur auf der Nachholbank, nie auf der Werkbank; `live::warten` sieht nur den Port.

use super::Werkbank;
use super::abbruch::Abgebrochen;
use crate::gespraeche::nachholen::Nachgeholt;
use crate::live::warten::{Nacharbeit, Nachholplatz};

/// Der Platz eines Aufrufs auf der Nachholbank.
pub struct Platz {
    bank: Werkbank,
}

impl Platz {
    pub fn auf(bank: &Werkbank) -> Platz {
        Platz { bank: bank.clone() }
    }
}

impl Nachholplatz for Platz {
    type Fehler = Abgebrochen;

    fn ausfuehren<F>(&self, arbeit: F) -> Nacharbeit<Abgebrochen>
    where
        F: FnOnce() -> Option<Nachgeholt> + Send + 'static,
    {
        let bank = self.bank.clone();
        Box::pin(async move { bank.ausfuehren(arbeit).await })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Die Erlaubnis der Bank lebt bis zum Ende der Arbeit (Mutante „Erlaubnis vor dem Ende frei“).
    #[tokio::test]
    async fn der_platz_lebt_bis_zum_ende_der_arbeit() {
        let bank = Werkbank::mit(1);
        let innen = bank.clone();

        let belegt_waehrend = Platz::auf(&bank)
            .ausfuehren(move || {
                assert_eq!(innen.belegt(), 1);
                None
            })
            .await;

        assert!(belegt_waehrend.is_ok());
        assert_eq!(bank.belegt(), 0);
    }
}
