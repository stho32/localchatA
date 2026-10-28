//! Takt (B0083, ADR-0016 Regel 4): alle 250 ms `pflegen(jetzt)` und die Wechsel veröffentlichen — nach dem
//! Freigeben der Sperre; endet mit dem Ende-Signal.

use super::bus::{Bus, Ereignis};
use crate::anwesenheit::Anwesenheit;
use crate::start::ende::Ende;
use crate::uhr::Uhr;
use std::sync::Arc;
use std::time::Duration;

pub const PERIODE: Duration = Duration::from_millis(250);

/// Pflegt die Anwesenheit im Takt `periode`, bis das Ende gemeldet ist.
pub async fn takten(
    anwesenheit: Arc<Anwesenheit>,
    uhr: Arc<dyn Uhr>,
    bus: Arc<Bus>,
    mut ende: Ende,
    periode: Duration,
) {
    let mut takt = tokio::time::interval(periode);
    takt.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            () = ende.abwarten() => return,
            _ = takt.tick() => {
                for wechsel in anwesenheit.pflegen(uhr.jetzt()) {
                    bus.veroeffentlichen(Ereignis::Anwesenheit(wechsel));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anwesenheit::Beitrittswunsch;
    use crate::anwesenheit::status::Status;
    use crate::live::bus::Empfang;
    use crate::uhr::{GestellteUhr, Zeit};
    use crate::zufall::FesteFolge;
    use serde_json::json;

    #[tokio::test]
    async fn der_takt_veroeffentlicht_offline_nach_dem_nachlauf_und_endet_mit_dem_ende() {
        let t0 = Zeit::parse_from_rfc3339("2026-10-28T09:00:00+01:00").expect("Zeit");
        let uhr = Arc::new(GestellteUhr::neu(t0));
        let anwesenheit = Arc::new(Anwesenheit::neu(
            Arc::clone(&uhr) as Arc<dyn Uhr>,
            Arc::new(FesteFolge::neu([1])),
            Duration::from_secs(60),
        ));
        let bus = Arc::new(Bus::neu(16));
        let mut abonnement = bus.abonnieren();
        let wunsch = Beitrittswunsch::pruefen(&json!({"name": "alpha"})).expect("Wunsch");
        anwesenheit
            .beitreten(&wunsch, "127.0.0.2".parse().expect("IP"))
            .expect("beigetreten");
        let (melden, ende) = Ende::neu();
        let aufgabe = tokio::spawn(takten(
            Arc::clone(&anwesenheit),
            Arc::clone(&uhr) as Arc<dyn Uhr>,
            Arc::clone(&bus),
            ende,
            Duration::from_millis(10),
        ));

        uhr.stellen(t0 + chrono::Duration::seconds(1));
        let empfangen = tokio::time::timeout(Duration::from_secs(1), abonnement.naechstes())
            .await
            .expect("Wechsel binnen 1 s");
        melden.melden();
        tokio::time::timeout(Duration::from_secs(1), aufgabe)
            .await
            .expect("Takt endet mit dem Ende")
            .expect("Aufgabe");

        let Some(Empfang::Ereignis(Ereignis::Anwesenheit(wechsel))) = empfangen else {
            panic!("{empfangen:?}");
        };
        assert_eq!(wechsel.teilnehmer.status, Status::Offline);
        assert_eq!(wechsel.stand, 2);
    }
}
