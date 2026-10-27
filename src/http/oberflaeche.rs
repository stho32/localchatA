//! Oberfläche ausliefern: `/` und `/web/<datei>` aus dem Programm, nie zwischengespeichert (ADR-0002).

use super::eingebettet;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::response::{IntoResponse, Response};

pub async fn wurzel() -> Response {
    ausliefern("index.html")
}

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
