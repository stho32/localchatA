//! Health-Check: `GET /api/gesundheit` meldet Bereitschaft und Kennzahlen des Ordners (ADR-0009).

use super::Zustand;
use super::teilnehmerliste::zeit_als_text;
use crate::protokoll::zaehler::AbgewiesenStand;
use crate::start::abrisspflege::pflege_aus;
use axum::Json;
use axum::extract::State;
use serde::Serialize;
use std::sync::Arc;

#[derive(Debug, Serialize)]
pub struct Gesundheit {
    status: &'static str,
    version: &'static str,
    ordner: String,
    nachrichten: u64,
    log_bytes: u64,
    ladezeit_ms: u64,
    defekte_zeilen: u64,
    channels: u64,
    /// Vorlauf der Nachrichtenzeit vor der Uhr (ADR-0035 Regel 4), 0 ohne.
    zeit_vorlauf_ms: u64,
    sitzungen: usize,
    halter: usize,
    herzschlag_ms: u128,
    abrissgrenze_ms: u128,
    protokoll_stufe: &'static str,
    protokoll_ok: bool,
    protokoll_bytes_heute: u64,
    protokoll_verworfen: u64,
    warnings: u64,
    errors: u64,
    exceptions: u64,
    letzter_error: Option<String>,
    letzte_exception: Option<String>,
    aufgaben_ausgefallen: Vec<&'static str>,
    abgewiesen: AbgewiesenStand,
}

/// Antwortet der Port, ist die Instanz bereit: geladen wird vor dem Binden (A00003).
/// Die Kennzahlen kommen aus Laden und Verlaufsindex (R00002), dem Channelverzeichnis (Anzahl, ADR-0021) und der Anwesenheit: lebende Sitzungen und
/// offene Ströme (R00004) — Zahlen ohne Namen, ohne Sitzung lesbar; dazu die Programmversion, mit der die
/// Oberfläche ihren Worker anlegt (ADR-0015 Regel 3), und die wirksamen Werte von Herzschlag und Abrissgrenze
/// (ADR-0017, B0103).
pub async fn beantworten(State(zustand): State<Arc<Zustand>>) -> Json<Gesundheit> {
    let kennzahlen = zustand.gespraeche.kennzahlen();
    let betrieb = zustand.anwesenheit.betrieb();
    let protokoll = zustand.protokoll.kennzahlen();
    Json(Gesundheit {
        status: "bereit",
        version: env!("CARGO_PKG_VERSION"),
        ordner: zustand.ordner.display().to_string(),
        nachrichten: kennzahlen.nachrichten,
        log_bytes: kennzahlen.log_bytes,
        ladezeit_ms: kennzahlen.ladezeit_ms,
        defekte_zeilen: kennzahlen.defekte_zeilen,
        channels: kennzahlen.channels,
        zeit_vorlauf_ms: zustand.gespraeche.zeit_vorlauf_ms(),
        sitzungen: betrieb.sitzungen,
        halter: betrieb.halter,
        herzschlag_ms: zustand.herzschlag.as_millis(),
        abrissgrenze_ms: pflege_aus(zustand.herzschlag).user_timeout.as_millis(),
        protokoll_stufe: protokoll.stufe.als_wert(),
        protokoll_ok: protokoll.ok,
        protokoll_bytes_heute: protokoll.bytes_heute,
        protokoll_verworfen: protokoll.verworfen,
        warnings: protokoll.zaehler.warnings,
        errors: protokoll.zaehler.errors,
        exceptions: protokoll.zaehler.exceptions,
        letzter_error: protokoll.zaehler.letzter_error.as_ref().map(zeit_als_text),
        letzte_exception: protokoll
            .zaehler
            .letzte_exception
            .as_ref()
            .map(zeit_als_text),
        aufgaben_ausgefallen: protokoll.ausgefallen,
        abgewiesen: protokoll.abgewiesen,
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
        let anfang = std::time::Instant::now();
        let (status, typ, mut inhalt) = gesundheit_fuer("/tmp/lc-r1").await;
        let gemessen = anfang.elapsed();
        let ladezeit_ms = inhalt["ladezeit_ms"]
            .take()
            .as_u64()
            .expect("ladezeit_ms als Zahl");

        assert!(
            u128::from(ladezeit_ms) <= gemessen.as_millis(),
            "{ladezeit_ms} ms"
        );
        assert_eq!(status, 200);
        assert_eq!(typ, "application/json");
        assert_eq!(
            inhalt,
            json!({
                "status": "bereit",
                "version": env!("CARGO_PKG_VERSION"),
                "ordner": "/tmp/lc-r1",
                "nachrichten": 0,
                "log_bytes": 0,
                "ladezeit_ms": null,
                "defekte_zeilen": 0,
                "channels": 0,
                "zeit_vorlauf_ms": 0,
                "sitzungen": 0,
                "halter": 0,
                "herzschlag_ms": 5000,
                "abrissgrenze_ms": 10_000,
                "protokoll_stufe": "information",
                "protokoll_ok": true,
                "protokoll_bytes_heute": 0,
                "protokoll_verworfen": 0,
                "warnings": 0,
                "errors": 0,
                "exceptions": 0,
                "letzter_error": null,
                "letzte_exception": null,
                "aufgaben_ausgefallen": [],
                "abgewiesen": {"zu_gross": 0, "kein_json": 0, "kaputt": 0, "abgebrochen": 0, "zu_langsam": 0}
            })
        );
    }

    #[tokio::test]
    async fn der_ordner_der_instanz_steht_in_der_antwort() {
        let (_, _, inhalt) = gesundheit_fuer("/srv/anderer ordner").await;
        assert_eq!(inhalt["ordner"], "/srv/anderer ordner");
    }
}
