//! Teilnehmer beantworten (B0077, B0106): `GET /api/teilnehmer` mit Sitzung → 200 `{stand, jetzt, liste}`, je
//! Eintrag `{kennung, name, herkunft, art, status, seit, weg_um}` (ADR-0016 Regel 7, ADR-0019 Regeln 1–2);
//! Zeiten RFC 3339 mit Millisekunden; ohne gültige Sitzung 401 aus dem Extraktor `Teilnehmer`.

use super::Zustand;
use super::teilnehmer::Teilnehmer;
use crate::anwesenheit::{Stand, Teilnehmerzeile};
use crate::uhr::Zeit;
use axum::Json;
use axum::extract::State;
use chrono::SecondsFormat;
use serde_json::{Value, json};
use std::sync::Arc;

pub async fn beantworten(_: Teilnehmer, State(zustand): State<Arc<Zustand>>) -> Json<Value> {
    Json(stand_als_json(&zustand.anwesenheit.teilnehmer()))
}

pub fn stand_als_json(stand: &Stand) -> Value {
    let liste: Vec<Value> = stand.liste.iter().map(zeile_als_json).collect();
    json!({ "stand": stand.stand, "jetzt": zeit_als_text(&stand.jetzt), "liste": liste })
}

pub fn zeile_als_json(zeile: &Teilnehmerzeile) -> Value {
    json!({
        "kennung": zeile.kennung.als_text(),
        "name": zeile.name.als_text(),
        "herkunft": zeile.herkunft.to_string(),
        "art": zeile.art,
        "status": zeile.status,
        "seit": zeit_als_text(&zeile.seit),
        "weg_um": zeile.weg_um.as_ref().map(zeit_als_text),
    })
}

/// Serverzeit an der Grenze: RFC 3339 mit Millisekunden und Versatz.
pub fn zeit_als_text(zeit: &Zeit) -> String {
    zeit.to_rfc3339_opts(SecondsFormat::Millis, false)
}
