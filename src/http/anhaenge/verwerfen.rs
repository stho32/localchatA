//! Verwerfen (B0430, ADR-0046 Regel 3 und Ergänzung „Karte #9 Gate“ Regel 5): `DELETE /api/anhaenge/{id}` mit
//! Sitzung → immer 204. Die eigene ID: ein laufender `PUT` endet als Abbruch (er räumt seine Teildatei selbst),
//! Teil- bzw. fertige Datei weg, Reservierung sofort frei. Fremde, unbekannte und schon gebundene IDs bleiben
//! unberührt — keine Antwort verrät etwas über sie (M9).

use super::super::Zustand;
use super::super::teilnehmer::Teilnehmer;
use crate::gespraeche::anhangid::Anhangid;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use std::sync::Arc;

pub async fn verwerfen(
    teilnehmer: Teilnehmer,
    State(zustand): State<Arc<Zustand>>,
    Path(id): Path<String>,
) -> Response {
    if let Some(id) = Anhangid::lesen(&id) {
        let gespraeche = Arc::clone(&zustand.gespraeche);
        let kennung = teilnehmer.kennung;
        let _ =
            tokio::task::spawn_blocking(move || gespraeche.anhang_verwerfen(&kennung, &id)).await;
    }
    StatusCode::NO_CONTENT.into_response()
}
