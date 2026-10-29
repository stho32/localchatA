//! R00019 F0064 „Löschfehler nach dem Setzen“: über den Router und dieselbe Annahmeschleife wie das Programm, mit
//! Gesprächen auf der Speicher-Logablage (das Entfernen eines Tages scheitert, ohne dass das Schreiben der
//! Einstellungen scheitert — über die Platte nicht herstellbar) und dem Protokoll in der Speicher-Senke.

use super::testserver;
use crate::gespraeche::{Gespraech, Gespraeche, Speicherablage};
use crate::protokoll::Stufe;
use crate::protokoll::senke::Speicher;
use crate::protokoll::testhilfe::{im_speicher, zeit};
use crate::uhr::GestellteUhr;
use chrono::NaiveDate;
use serde_json::{Value, json};
use std::sync::Arc;

const SITZUNG: &str = "X-LocalchatA-Sitzung";

struct Instanz {
    basis: String,
    speicher: Speicher,
    _ordner: tempfile::TempDir,
}

fn tag(text: &str) -> NaiveDate {
    NaiveDate::parse_from_str(text, "%Y-%m-%d").expect("Testtag")
}

/// `allgemein` mit Frist 30 und 20.(2), 21.(3), 22.(1), 28.(1); das Entfernen des 21. scheitert.
async fn starten() -> Instanz {
    let ordner = tempfile::tempdir().expect("Wegwerf-Ordner");
    let ablage = Arc::new(Speicherablage::neu());
    let allgemein = Gespraech::channel("allgemein");
    ablage.einstellungen_setzen(&allgemein, b"frist_tage = 30\n");
    for (tag_text, anzahl) in [
        ("2026-10-20", 2),
        ("2026-10-21", 3),
        ("2026-10-22", 1),
        ("2026-10-28", 1),
    ] {
        let satz = format!("@ {tag_text}T09:00:00.000+02:00 a@127.0.0.9\n  x\n\n");
        ablage.tag_setzen(&allgemein, tag(tag_text), &satz.repeat(anzahl));
    }
    let uhr = Arc::new(GestellteUhr::neu(zeit("2026-10-28T12:00:00.000+01:00")));
    let gespraeche = Gespraeche::laden(ablage.clone(), uhr).expect("geladen");
    ablage.entfernen_scheitern(&allgemein, tag("2026-10-21"));
    let mut zustand = testserver::zustand(ordner.path());
    zustand.gespraeche = Arc::new(gespraeche);
    let (protokoll, speicher, _, _) = im_speicher("2026-10-28T12:00:00+01:00", Stufe::Information);
    zustand.protokoll = protokoll;
    let basis = testserver::app_starten(super::router(zustand)).await;
    Instanz {
        basis,
        speicher,
        _ordner: ordner,
    }
}

impl Instanz {
    async fn anfrage(
        &self,
        methode: reqwest::Method,
        pfad: &str,
        koerper: Value,
        token: Option<&str>,
    ) -> (u16, Value) {
        let mut anfrage = reqwest::Client::new()
            .request(methode, format!("{}{pfad}", self.basis))
            .json(&koerper);
        if let Some(token) = token {
            anfrage = anfrage.header(SITZUNG, token);
        }
        let antwort = anfrage.send().await.expect("Antwort");
        let status = antwort.status().as_u16();
        (status, antwort.json().await.unwrap_or(Value::Null))
    }

    fn frist_zeilen(&self) -> Vec<String> {
        self.speicher
            .zeilen()
            .into_iter()
            .filter(|zeile| zeile.contains(" frist "))
            .collect()
    }
}

/// R00019 F0064 „Löschfehler nach dem Setzen“: `beta` verkürzt bestätigt auf 7 → 200; die Senke hält genau eine
/// Zeile „Tag nicht entfallen“ für den 21. mit `grund=frist_gesetzt`, der Kennung und dem Plattenfehler in
/// `ursache`.
#[tokio::test]
async fn ein_loeschfehler_nach_dem_setzen_nennt_anlass_und_kennung_dessen_der_verkuerzte() {
    let instanz = starten().await;
    let (status, sitzung) = instanz
        .anfrage(
            reqwest::Method::POST,
            "api/beitreten",
            json!({"name": "beta"}),
            None,
        )
        .await;
    assert_eq!(status, 201, "{sitzung}");
    let token = sitzung["token"].as_str().expect("Token");

    let (status, antwort) = instanz
        .anfrage(
            reqwest::Method::PUT,
            "api/channels/allgemein/frist",
            json!({"tage": 7, "bestaetigt": true}),
            Some(token),
        )
        .await;

    assert_eq!(status, 200, "{antwort}");
    let nicht_entfallen: Vec<String> = instanz
        .frist_zeilen()
        .into_iter()
        .filter(|zeile| zeile.contains(" ERROR frist Tag nicht entfallen"))
        .collect();
    assert_eq!(nicht_entfallen.len(), 1, "{:#?}", instanz.frist_zeilen());
    assert!(
        nicht_entfallen[0].ends_with(
            " ERROR frist Tag nicht entfallen — erneuter Versuch beim nächsten Anlass | channel=allgemein datei=channels/allgemein/2026-10-21.log grund=frist_gesetzt kennung=beta@127.0.0.1 ursache=\"Entfernen scheitert (Speicher-Logablage)\""
        ),
        "{nicht_entfallen:#?}"
    );
}
