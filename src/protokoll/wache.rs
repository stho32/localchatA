//! Wache über Hintergrundaufgaben (ADR-0042 Regel 4): endet eine Aufgabe vor dem Ende-Signal — durch Panik
//! oder Rückkehr —, steht `ERROR programmfehler Hintergrundaufgabe <name> beendet — <folge>` im Protokoll und ihr
//! Name in den ausgefallenen.

use super::Protokoll;
use crate::start::ende::Ende;
use std::future::Future;
use std::sync::Arc;
use tokio::task::JoinHandle;

/// Startet `aufgabe` bewacht; das Ergebnis endet, wenn die Aufgabe endet.
pub fn bewacht<F>(
    protokoll: Arc<Protokoll>,
    name: &'static str,
    folge: &'static str,
    ende: Ende,
    aufgabe: F,
) -> JoinHandle<()>
where
    F: Future<Output = ()> + Send + 'static,
{
    tokio::spawn(async move {
        let _ = tokio::spawn(aufgabe).await;
        if !ende.gemeldet() {
            protokoll.aufgabe_ausgefallen(name, folge);
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protokoll::Stufe;
    use crate::protokoll::testhilfe::im_speicher;

    #[tokio::test]
    async fn eine_panik_vor_dem_ende_meldet_die_aufgabe_als_ausgefallen() {
        let (protokoll, speicher, _, _) =
            im_speicher("2026-10-28T12:00:00+01:00", Stufe::Information);
        let (_melden, ende) = Ende::neu();

        bewacht(
            Arc::clone(&protokoll),
            "takt",
            "Anwesenheit wird nicht mehr gepflegt",
            ende,
            async { panic!("Takt in Panik (Probe)") },
        )
        .await
        .expect("Wache endet");

        assert_eq!(protokoll.ausgefallen(), ["takt"]);
        let zeilen = speicher.zeilen();
        assert_eq!(zeilen.len(), 1, "{zeilen:#?}");
        assert!(
            zeilen[0].ends_with(" ERROR programmfehler Hintergrundaufgabe takt beendet — Anwesenheit wird nicht mehr gepflegt"),
            "{zeilen:#?}"
        );
    }

    #[tokio::test]
    async fn eine_rueckkehr_vor_dem_ende_ist_ebenso_ein_ausfall() {
        let (protokoll, _, _, _) = im_speicher("2026-10-28T12:00:00+01:00", Stufe::Information);
        let (_melden, ende) = Ende::neu();

        bewacht(Arc::clone(&protokoll), "tagestakt", "—", ende, async {})
            .await
            .expect("Wache endet");

        assert_eq!(protokoll.ausgefallen(), ["tagestakt"]);
    }

    #[tokio::test]
    async fn eine_rueckkehr_nach_dem_ende_signal_ist_kein_ausfall() {
        let (protokoll, speicher, _, _) =
            im_speicher("2026-10-28T12:00:00+01:00", Stufe::Information);
        let (melden, ende) = Ende::neu();
        let mut hoeren = ende.clone();

        let wache = bewacht(Arc::clone(&protokoll), "takt", "—", ende, async move {
            hoeren.abwarten().await;
        });
        melden.melden();
        wache.await.expect("Wache endet");

        assert!(protokoll.ausgefallen().is_empty());
        assert!(speicher.zeilen().is_empty());
    }
}
