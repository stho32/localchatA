//! Verlassen beantworten (B0114, ADR-0018 Regeln 1–3): `POST /api/verlassen` ohne Körper, mit dem Header
//! `X-LocalchatA-Sitzung`. Gültiges Token derselben Herkunft → die Sitzung ist beendet, `weg` ist unter der
//! Sperre veröffentlicht, bevor die 204 geht; jedes andere Token → 204 ohne Wirkung (kein Orakel, keine
//! Fremdbeendigung); ohne Header 401. Bewusst nicht über den Extraktor `Teilnehmer`: der antwortete auf ein
//! ungültiges Token mit 401 und verriete so, ob es galt. Fremder `Origin` → 403 aus `grenzschutz`.

use super::Zustand;
use super::teilnehmer::{gemeldetes_token, nicht_erkannt, sitzung_gemeldet};
use axum::extract::{ConnectInfo, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use std::net::SocketAddr;
use std::sync::Arc;

pub async fn beantworten(
    State(zustand): State<Arc<Zustand>>,
    ConnectInfo(gegenstelle): ConnectInfo<SocketAddr>,
    kopf: HeaderMap,
) -> Response {
    if !sitzung_gemeldet(&kopf) {
        return nicht_erkannt();
    }
    if let Some(token) = gemeldetes_token(&kopf) {
        zustand.anwesenheit.verlassen(&token, gegenstelle.ip());
    }
    StatusCode::NO_CONTENT.into_response()
}
