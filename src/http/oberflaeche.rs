//! Oberfläche ausliefern: `/` und `/web/<datei>` aus dem Programm, nie zwischengespeichert (ADR-0002).

use super::eingebettet;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::response::{IntoResponse, Response};

/// Die Seite nennt die Programmversion (`<meta name="lc-version">`), damit die Oberfläche ihren Worker mit
/// der Version in der Adresse anlegt (ADR-0015 Regel 3).
pub async fn wurzel() -> Response {
    let Some(seite) = eingebettet::nachschlagen("index.html") else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let html = String::from_utf8_lossy(seite.inhalt)
        .replace(VERSION_PLATZHALTER, env!("CARGO_PKG_VERSION"));
    (
        [(CONTENT_TYPE, seite.typ), (CACHE_CONTROL, "no-cache")],
        html,
    )
        .into_response()
}

const VERSION_PLATZHALTER: &str = "@VERSION@";

pub async fn datei(Path(name): Path<String>) -> Response {
    ausliefern(&name)
}

fn ausliefern(name: &str) -> Response {
    match eingebettet::nachschlagen(name) {
        Some(datei) => (
            [(CONTENT_TYPE, datei.typ), (CACHE_CONTROL, "no-cache")],
            datei.inhalt,
        )
            .into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
