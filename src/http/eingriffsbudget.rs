//! R00034 F0125 „Beitrittsflut lässt die Fristzeile stehen“ (ADR-0049, Abnahme-Probe wörtlich): über den Router und
//! dieselbe Annahmeschleife wie das Programm, mit dem Protokoll in der Speicher-Senke und den Tagesbudgets
//! `INFORMATION+WARNING` und `EINGRIFF` je 2 KiB. Eine Flut aus `beitreten`/`verlassen` leert das allgemeine
//! Budget; die Fristzeile, die danach eine Sitzung erzeugt, steht trotzdem im Protokoll.

use super::testserver;
use crate::gespraeche::{Gespraech, Gespraeche, Speicherablage};
use crate::protokoll::budget::{BUDGETS, Budgets};
use crate::protokoll::senke::Speicher;
use crate::protokoll::testhilfe::{Aufzeichnung, zeit};
use crate::protokoll::{Protokoll, Stufe};
use crate::uhr::{GestellteUhr, Uhr};
use serde_json::{Value, json};
use std::sync::Arc;

const SITZUNG: &str = "X-LocalchatA-Sitzung";
const BUDGETZEILE_INFORMATION: &str = " WARNING protokoll Tagesbudget INFORMATION+WARNING erreicht — weitere Einträge bis Mitternacht nur gezählt";

struct Instanz {
    basis: String,
    speicher: Speicher,
    protokoll: Arc<Protokoll>,
    _ordner: tempfile::TempDir,
}

/// `allgemein` mit Frist 7; Protokoll in der Speicher-Senke mit zwei KiB für `INFORMATION+WARNING` und `EINGRIFF`.
async fn starten() -> Instanz {
    let ordner = tempfile::tempdir().expect("Wegwerf-Ordner");
    let ablage = Arc::new(Speicherablage::neu());
    let allgemein = Gespraech::channel("allgemein");
    ablage.gespraech_anlegen(&allgemein);
    ablage.einstellungen_setzen(&allgemein, b"frist_tage = 7\n");
    let uhr = Arc::new(GestellteUhr::neu(zeit("2026-10-29T12:00:00.000+01:00")));
    let gespraeche = Gespraeche::laden(ablage, uhr.clone()).expect("geladen");
    let speicher = Speicher::neu();
    let protokoll = Arc::new(
        Protokoll::mit(
            Box::new(speicher.clone()),
            Aufzeichnung::default().spiegel(),
            uhr as Arc<dyn Uhr>,
            Stufe::Information,
        )
        .mit_budgets(Budgets {
            information_warning: 2048,
            eingriff: 2048,
            ..BUDGETS
        }),
    );
    let mut zustand = testserver::zustand(ordner.path());
    zustand.gespraeche = Arc::new(gespraeche);
    zustand.protokoll = protokoll.clone();
    let basis = testserver::app_starten(super::router(zustand)).await;
    Instanz {
        basis,
        speicher,
        protokoll,
        _ordner: ordner,
    }
}

impl Instanz {
    async fn anfrage(
        &self,
        methode: reqwest::Method,
        pfad: &str,
        koerper: Option<Value>,
        token: Option<&str>,
    ) -> (u16, Value) {
        let mut anfrage = reqwest::Client::new().request(methode, format!("{}{pfad}", self.basis));
        if let Some(koerper) = koerper {
            anfrage = anfrage.json(&koerper);
        }
        if let Some(token) = token {
            anfrage = anfrage.header(SITZUNG, token);
        }
        let antwort = anfrage.send().await.expect("Antwort");
        let status = antwort.status().as_u16();
        (status, antwort.json().await.unwrap_or(Value::Null))
    }

    async fn beitreten(&self, name: &str) -> String {
        let (status, sitzung) = self
            .anfrage(
                reqwest::Method::POST,
                "api/beitreten",
                Some(json!({"name": name})),
                None,
            )
            .await;
        assert_eq!(status, 201, "{sitzung}");
        sitzung["token"].as_str().expect("Token").to_string()
    }

    /// 20-mal beitreten mit neuem Namen und mit dem erhaltenen Token wieder verlassen.
    async fn flut(&self) {
        for nummer in 1..=20 {
            let token = self.beitreten(&format!("gast{nummer:02}")).await;
            let (status, _) = self
                .anfrage(reqwest::Method::POST, "api/verlassen", None, Some(&token))
                .await;
            assert_eq!(status, 204);
        }
    }

    fn zeilen_mit(&self, teil: &str) -> Vec<String> {
        self.speicher
            .zeilen()
            .into_iter()
            .filter(|zeile| zeile.contains(teil))
            .collect()
    }
}

/// Abnahme-Probe der ADR-0049 wörtlich: Flut, danach Frist 7 → 30 durch eine Sitzung von `127.0.0.1` (200).
#[tokio::test]
async fn nach_einer_beitrittsflut_steht_die_fristzeile_im_protokoll() {
    let instanz = starten().await;
    instanz.flut().await;
    let token = instanz.beitreten("ziel").await;

    let (status, antwort) = instanz
        .anfrage(
            reqwest::Method::PUT,
            "api/channels/allgemein/frist",
            Some(json!({"tage": 30})),
            Some(&token),
        )
        .await;

    assert_eq!(status, 200, "{antwort}");
    let alle = instanz.speicher.zeilen();
    let budgetzeilen = instanz.zeilen_mit(BUDGETZEILE_INFORMATION);
    assert_eq!(budgetzeilen.len(), 1, "{alle:#?}");
    assert!(instanz.protokoll.kennzahlen().verworfen >= 1);
    assert!(
        instanz.zeilen_mit(" INFORMATION sitzung ").len() < 41,
        "mindestens eine der Sitzungszeilen fehlt (40 Flut + 1 Beitritt der Sitzung)"
    );
    let frist = instanz.zeilen_mit(" INFORMATION frist Frist 7 → 30 Tage gesetzt");
    assert_eq!(frist.len(), 1, "{alle:#?}");
    assert!(
        frist[0].contains(" | channel=allgemein ") && frist[0].contains("kennung=ziel@127.0.0.1"),
        "{frist:#?}"
    );
    let stelle = |zeile: &String| alle.iter().position(|z| z == zeile).expect("Zeile");
    assert!(
        stelle(&frist[0]) > stelle(&budgetzeilen[0]),
        "Fristzeile steht danach"
    );
    assert!(
        instanz
            .zeilen_mit("Tagesbudget EINGRIFF erreicht")
            .is_empty(),
        "{alle:#?}"
    );
}
