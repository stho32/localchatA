//! Teilnehmer beantworten (B0077): `GET /api/teilnehmer` mit Sitzung → 200 `{stand, liste}`, je Eintrag
//! `{kennung, name, herkunft, art, status, seit}` (ADR-0016 Regel 7); ohne gültige Sitzung 401 aus dem
//! Extraktor `Teilnehmer`.

use super::Zustand;
use super::teilnehmer::Teilnehmer;
use crate::anwesenheit::{Stand, Teilnehmerzeile};
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
    json!({ "stand": stand.stand, "liste": liste })
}

pub fn zeile_als_json(zeile: &Teilnehmerzeile) -> Value {
    json!({
        "kennung": zeile.kennung.als_text(),
        "name": zeile.name.als_text(),
        "herkunft": zeile.herkunft.to_string(),
        "art": zeile.art,
        "status": zeile.status,
        "seit": zeile.seit.to_rfc3339_opts(SecondsFormat::Millis, false),
    })
}
