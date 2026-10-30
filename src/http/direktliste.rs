//! Die eigenen Direktgespräche (ADR-0055 Regel 3 letzter Punkt): `GET /api/direkt` mit Sitzung → 200
//! `{direktgespraeche: [{kennung, letzte, frist_tage}]}` — nur die eigenen mit mindestens einer Nachricht in der
//! Frist, `kennung` das Gegenüber, `letzte` die Zeit der jüngsten Nachricht, jüngste zuerst, `frist_tage` fest 7 bis
//! zur Ausbaustufe „Frist wählen“. `Cache-Control: no-store` setzt die Schicht des Grenzschutzes für jede `/api/`-Antwort.
//! Die Oberfläche füllt daraus die Seitenleiste „Direktnachrichten“.

use super::Zustand;
use super::teilnehmer::Teilnehmer;
use super::teilnehmerliste::zeit_als_text;
use crate::gespraeche::DIREKT_FRIST_TAGE;
use axum::Json;
use axum::extract::State;
use serde_json::{Value, json};
use std::sync::Arc;

pub async fn auflisten(teilnehmer: Teilnehmer, State(zustand): State<Arc<Zustand>>) -> Json<Value> {
    let eintraege: Vec<Value> = zustand
        .gespraeche
        .direktgespraeche_von(&teilnehmer.kennung)
        .iter()
        .map(|uebersicht| {
            json!({
                "kennung": uebersicht.gegenueber.als_text(),
                "letzte": zeit_als_text(&uebersicht.letzte),
                "frist_tage": DIREKT_FRIST_TAGE,
            })
        })
        .collect();
    Json(json!({ "direktgespraeche": eintraege }))
}
