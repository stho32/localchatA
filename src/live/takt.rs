//! Takt (B0083, ADR-0016 Regel 4): alle 250 ms `pflegen(jetzt)`; die Wechsel veröffentlicht die Anwesenheit
//! unter ihrer Sperre (Nacharbeit 1, B1). Endet mit dem Ende-Signal.

use crate::anwesenheit::Anwesenheit;
use crate::anwesenheit::status::Status;
use crate::protokoll::testpanik::Testpanik;
use crate::protokoll::{Bereich, Kontext, Protokoll, Schluessel, Stufe};
use crate::start::ende::Ende;
use crate::uhr::Uhr;
use std::sync::Arc;
use std::time::Duration;

pub const PERIODE: Duration = Duration::from_millis(250);

/// Pflegt die Anwesenheit im Takt `periode`, bis das Ende gemeldet ist.
pub async fn takten(
    anwesenheit: Arc<Anwesenheit>,
    uhr: Arc<dyn Uhr>,
    protokoll: Arc<Protokoll>,
    testpanik: Testpanik,
    mut ende: Ende,
    periode: Duration,
) {
    let mut takt = tokio::time::interval(periode);
    takt.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            () = ende.abwarten() => return,
            _ = takt.tick() => {
                testpanik.takt();
                for wechsel in anwesenheit.pflegen(uhr.jetzt()) {
                    if wechsel.teilnehmer.status == Status::Weg {
                        protokoll.eintragen(
                            Stufe::Information,
                            Bereich::Sitzung,
                            "Sitzung weg — Gnadenfrist ohne Lebenszeichen abgelaufen, der Name ist frei",
                            Kontext::neu().mit(Schluessel::Kennung, wechsel.teilnehmer.kennung.als_text()),
                        );
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anwesenheit::Beitrittswunsch;
    use crate::live::bus::{Bus, Empfang, Ereignis};
    use crate::uhr::{GestellteUhr, Zeit};
    use crate::zufall::FesteFolge;
    use serde_json::json;

    #[tokio::test]
    async fn der_takt_veroeffentlicht_offline_nach_dem_nachlauf_und_endet_mit_dem_ende() {
        let t0 = Zeit::parse_from_rfc3339("2026-10-28T09:00:00+01:00").expect("Zeit");
        let uhr = Arc::new(GestellteUhr::neu(t0));
        let bus = Arc::new(Bus::neu(16));
        let an_den_bus = Arc::clone(&bus);
        let anwesenheit = Arc::new(
            Anwesenheit::neu(
                Arc::clone(&uhr) as Arc<dyn Uhr>,
                Arc::new(FesteFolge::neu([1])),
                Duration::from_secs(60),
            )
            .mit_veroeffentlichen(Arc::new(move |wechsel| {
                an_den_bus.veroeffentlichen(Ereignis::Anwesenheit(wechsel.clone()));
            })),
        );
        let wunsch = Beitrittswunsch::pruefen(&json!({"name": "alpha"})).expect("Wunsch");
        anwesenheit
            .beitreten(&wunsch, "127.0.0.2".parse().expect("IP"))
            .expect("beigetreten");
        let mut abonnement = bus.abonnieren();
        let (melden, ende) = Ende::neu();
        let aufgabe = tokio::spawn(takten(
            Arc::clone(&anwesenheit),
            Arc::clone(&uhr) as Arc<dyn Uhr>,
            Arc::new(Protokoll::ohne_ablage()),
            Testpanik::keine(),
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

    #[tokio::test]
    async fn eine_sitzung_nach_der_gnadenfrist_steht_als_weg_im_protokoll() {
        let (protokoll, speicher, _, _) = crate::protokoll::testhilfe::im_speicher(
            "2026-10-28T09:00:00+01:00",
            Stufe::Information,
        );
        let t0 = Zeit::parse_from_rfc3339("2026-10-28T09:00:00+01:00").expect("Zeit");
        let uhr = Arc::new(GestellteUhr::neu(t0));
        let anwesenheit = Arc::new(Anwesenheit::neu(
            Arc::clone(&uhr) as Arc<dyn Uhr>,
            Arc::new(FesteFolge::neu([1])),
            Duration::from_secs(60),
        ));
        let wunsch = Beitrittswunsch::pruefen(&json!({"name": "alpha"})).expect("Wunsch");
        anwesenheit
            .beitreten(&wunsch, "127.0.0.2".parse().expect("IP"))
            .expect("beigetreten");
        let (melden, ende) = Ende::neu();
        let aufgabe = tokio::spawn(takten(
            Arc::clone(&anwesenheit),
            Arc::clone(&uhr) as Arc<dyn Uhr>,
            protokoll,
            Testpanik::keine(),
            ende,
            Duration::from_millis(10),
        ));

        uhr.stellen(t0 + chrono::Duration::seconds(1));
        tokio::time::sleep(Duration::from_millis(50)).await;
        let vor_der_frist = speicher.zeilen().len();
        uhr.stellen(t0 + chrono::Duration::seconds(62));
        let ende_warten = tokio::time::Instant::now() + Duration::from_secs(2);
        while speicher.zeilen().is_empty() && tokio::time::Instant::now() < ende_warten {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        melden.melden();
        let _ = aufgabe.await;

        assert_eq!(vor_der_frist, 0, "offline ist keine Information");
        let zeilen = speicher.zeilen();
        assert_eq!(zeilen.len(), 1, "{zeilen:#?}");
        assert!(
            zeilen[0].contains(" INFORMATION sitzung Sitzung weg"),
            "{zeilen:#?}"
        );
        assert!(
            zeilen[0].ends_with("kennung=alpha@127.0.0.2"),
            "{zeilen:#?}"
        );
    }
}
