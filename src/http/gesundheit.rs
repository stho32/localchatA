//! Health-Check: `GET /api/gesundheit` meldet Bereitschaft und Kennzahlen des Ordners (ADR-0009).

use super::Zustand;
use axum::Json;
use axum::extract::State;
use serde::Serialize;
use std::sync::Arc;

#[derive(Debug, Serialize)]
pub struct Gesundheit {
    status: &'static str,
    ordner: String,
    nachrichten: u64,
    log_bytes: u64,
    ladezeit_ms: u64,
    defekte_zeilen: u64,
    sitzungen: usize,
    halter: usize,
}

/// Antwortet der Port, ist die Instanz bereit: geladen wird vor dem Binden (A00003).
/// Die Kennzahlen kommen aus Laden und Verlaufsindex (R00002) und der Anwesenheit: lebende Sitzungen und
/// offene Ströme (R00004) — Zahlen ohne Namen, ohne Sitzung lesbar.
pub async fn beantworten(State(zustand): State<Arc<Zustand>>) -> Json<Gesundheit> {
    let kennzahlen = zustand.gespraeche.kennzahlen();
    let betrieb = zustand.anwesenheit.betrieb();
    Json(Gesundheit {
        status: "bereit",
        ordner: zustand.ordner.display().to_string(),
        nachrichten: kennzahlen.nachrichten,
        log_bytes: kennzahlen.log_bytes,
        ladezeit_ms: kennzahlen.ladezeit_ms,
        defekte_zeilen: kennzahlen.defekte_zeilen,
        sitzungen: betrieb.sitzungen,
        halter: betrieb.halter,
    })
}

#[cfg(test)]
mod tests {
    use crate::http::testserver;
    use serde_json::json;
    use std::path::Path;

    async fn gesundheit_fuer(ordner: &str) -> (u16, String, serde_json::Value) {
        let adresse = testserver::starten(Path::new(ordner)).await;
        let antwort = reqwest::get(format!("{adresse}api/gesundheit"))
            .await
            .expect("Antwort");
        let status = antwort.status().as_u16();
        let typ = antwort.headers()["content-type"]
            .to_str()
            .expect("Content-Type")
            .to_string();
        (status, typ, antwort.json().await.expect("JSON"))
    }

    #[tokio::test]
    async fn leerer_ordner_ist_bereit_mit_kennzahlen_null() {
        let (status, typ, inhalt) = gesundheit_fuer("/tmp/lc-r1").await;

        assert_eq!(status, 200);
        assert_eq!(typ, "application/json");
        assert_eq!(
            inhalt,
            json!({
                "status": "bereit",
                "ordner": "/tmp/lc-r1",
                "nachrichten": 0,
                "log_bytes": 0,
                "ladezeit_ms": 0,
                "defekte_zeilen": 0,
                "sitzungen": 0,
                "halter": 0
            })
        );
    }

    #[tokio::test]
    async fn der_ordner_der_instanz_steht_in_der_antwort() {
        let (_, _, inhalt) = gesundheit_fuer("/srv/anderer ordner").await;
        assert_eq!(inhalt["ordner"], "/srv/anderer ordner");
    }
}
