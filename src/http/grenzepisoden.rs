//! Code-Review #24 B1, Abnahme-Proben 1 und 2: die Episoden an der Sitzungs- und an der Channel-Grenze, über
//! den Router und dieselbe Annahmeschleife wie das Programm, mit einem Protokoll in der Speicher-Senke und
//! gestellter Uhr (das Ende der Episode ohne Warten). Die Grenzen liest der Test aus den Konstanten.

use super::testserver;
use crate::protokoll::senke::Speicher;
use crate::protokoll::testhilfe::{im_speicher, zeit};
use crate::protokoll::{Protokoll, Stufe};
use crate::uhr::GestellteUhr;
use serde_json::{Value, json};
use std::sync::Arc;

const SITZUNG: &str = "X-LocalchatA-Sitzung";

struct Instanz {
    basis: String,
    protokoll: Arc<Protokoll>,
    speicher: Speicher,
    uhr: Arc<GestellteUhr>,
    _ordner: tempfile::TempDir,
}

/// Router wie im Programm, Gespräche in einem eigenen Wegwerf-Ordner, Protokoll auf Information.
async fn starten() -> Instanz {
    let ordner = tempfile::tempdir().expect("Wegwerf-Ordner");
    let mut zustand = testserver::zustand(ordner.path());
    zustand.gespraeche = Arc::new(
        crate::gespraeche::Gespraeche::laden(
            Arc::new(crate::gespraeche::Dateiablage::neu(ordner.path())),
            Arc::new(crate::uhr::Systemuhr),
        )
        .expect("leerer Ordner"),
    );
    let (protokoll, speicher, uhr, _) =
        im_speicher("2026-10-28T12:00:00+01:00", Stufe::Information);
    zustand.protokoll = Arc::clone(&protokoll);
    let basis = testserver::app_starten(super::router(zustand)).await;
    Instanz {
        basis,
        protokoll,
        speicher,
        uhr,
        _ordner: ordner,
    }
}

impl Instanz {
    async fn post(&self, pfad: &str, koerper: Value, token: Option<&str>) -> (u16, Value) {
        let mut anfrage = reqwest::Client::new()
            .post(format!("{}{pfad}", self.basis))
            .json(&koerper);
        if let Some(token) = token {
            anfrage = anfrage.header(SITZUNG, token);
        }
        let antwort = anfrage.send().await.expect("Antwort");
        let status = antwort.status().as_u16();
        (status, antwort.json().await.unwrap_or(Value::Null))
    }

    /// Alle Zeilen `WARNING <bereich>` in Schreibreihenfolge, nachdem die Ruhe (60 s) vorbei ist.
    fn warnungen_nach_der_ruhe(&self, bereich: &str) -> Vec<String> {
        self.uhr.stellen(zeit("2026-10-28T12:01:01+01:00"));
        self.protokoll.episoden_schliessen();
        let teil = format!(" WARNING {bereich} ");
        self.speicher
            .zeilen()
            .into_iter()
            .filter(|zeile| zeile.contains(&teil))
            .collect()
    }
}

/// Probe 1: `JE_HERKUNFT` Beitritte von 127.0.0.1 gehen, die nächsten 6 sind 429 — genau eine Zeile „Episode
/// beginnt“ mit Herkunft und Grenze, nach der Ruhe genau eine „Episode endet: 6“.
#[tokio::test]
async fn sechs_beitritte_ueber_der_sitzungsgrenze_sind_eine_episode_mit_herkunft_und_grenze() {
    use crate::anwesenheit::{GESAMT, JE_HERKUNFT};
    let instanz = starten().await;
    for nummer in 1..=JE_HERKUNFT {
        let (status, _) = instanz
            .post("api/beitreten", json!({"name": format!("n{nummer}")}), None)
            .await;
        assert_eq!(status, 201, "n{nummer}");
    }

    let mut ueber = Vec::new();
    for nummer in 1..=6 {
        let name = format!("z{nummer}");
        ueber.push(
            instanz
                .post("api/beitreten", json!({"name": name}), None)
                .await
                .0,
        );
    }
    let warnungen = instanz.warnungen_nach_der_ruhe("sitzung");

    assert_eq!(ueber, [429; 6]);
    assert_eq!(warnungen.len(), 2, "{warnungen:#?}");
    assert!(
        warnungen[0].ends_with(&format!(
            " WARNING sitzung Beitritt an der Sitzungsgrenze abgewiesen — Episode beginnt | herkunft=127.0.0.1 grenze=\"{JE_HERKUNFT} je Herkunft, {GESAMT} gesamt\""
        )),
        "{warnungen:#?}"
    );
    assert!(
        warnungen[1].contains(
            " WARNING sitzung Beitritt an der Sitzungsgrenze abgewiesen — Episode endet: 6 in "
        ),
        "{warnungen:#?}"
    );
}

/// Probe 2: `JE_HERKUNFT` Channels einer Herkunft gehen, der nächste ist 429 — genau eine Zeile „Episode
/// beginnt“ mit Herkunft und Grenze.
#[tokio::test]
async fn ein_anlegen_ueber_der_channel_grenze_ist_eine_episode_mit_herkunft_und_grenze() {
    use crate::gespraeche::JE_HERKUNFT;
    let instanz = starten().await;
    let (status, sitzung) = instanz
        .post("api/beitreten", json!({"name": "alpha"}), None)
        .await;
    assert_eq!(status, 201);
    let token = sitzung["token"].as_str().expect("Token").to_string();
    for nummer in 1..=JE_HERKUNFT {
        let name = format!("c{nummer:03}");
        let (status, antwort) = instanz
            .post("api/channels", json!({"name": name}), Some(&token))
            .await;
        assert_eq!(status, 201, "{name}: {antwort}");
    }

    let (ueber, antwort) = instanz
        .post("api/channels", json!({"name": "ueber"}), Some(&token))
        .await;
    let warnungen = instanz.warnungen_nach_der_ruhe("channels");

    assert_eq!(ueber, 429, "{antwort}");
    assert_eq!(warnungen.len(), 2, "{warnungen:#?}");
    assert!(
        warnungen[0].ends_with(&format!(
            " WARNING channels Anlegen an der Channel-Grenze abgewiesen — Episode beginnt | herkunft=127.0.0.1 grenze=\"{JE_HERKUNFT} herkunft\""
        )),
        "{warnungen:#?}"
    );
    assert!(
        warnungen[1].contains(" Episode endet: 1 in "),
        "{warnungen:#?}"
    );
}
