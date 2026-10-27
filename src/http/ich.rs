//! Ich beantworten (B0054): `GET /api/ich` → 200 `{kennung, name, herkunft, art}` mit gültiger Sitzung,
//! sonst 401 aus dem Extraktor `Teilnehmer` — die Oberfläche prüft damit beim Laden ihr gemerktes Token.

use super::teilnehmer::Teilnehmer;
use axum::Json;
use serde_json::{Value, json};

pub async fn beantworten(teilnehmer: Teilnehmer) -> Json<Value> {
    Json(json!({
        "kennung": teilnehmer.kennung.als_text(),
        "name": teilnehmer.name.als_text(),
        "herkunft": teilnehmer.herkunft.to_string(),
        "art": teilnehmer.art,
    }))
}
