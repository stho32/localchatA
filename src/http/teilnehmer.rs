//! Teilnehmer an der Grenze erkennen (B0053): Extraktor für jede Route, die eine Sitzung verlangt —
//! Header `X-LocalchatA-Sitzung` mit gültigem Token derselben Herkunft, sonst 401 `{fehler}` (ADR-0012).

use super::Zustand;
use super::grenzschutz::einziger_wert;
use crate::anwesenheit::{Anwesenheit, Art, Sitzung};
use crate::gespraeche::Kennung;
use crate::name::Name;
use crate::zufall::Token;
use axum::Json;
use axum::extract::{ConnectInfo, FromRequestParts};
use axum::http::request::Parts;
use axum::http::{HeaderMap, HeaderName, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::json;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

/// Wer eine Anfrage stellt — geprüft; das Token bleibt in der Anwesenheit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Teilnehmer {
    pub kennung: Kennung,
    pub name: Name,
    pub herkunft: IpAddr,
    pub art: Art,
}

impl FromRequestParts<Arc<Zustand>> for Teilnehmer {
    type Rejection = Response;

    async fn from_request_parts(
        teile: &mut Parts,
        zustand: &Arc<Zustand>,
    ) -> Result<Teilnehmer, Response> {
        let herkunft = teile
            .extensions
            .get::<ConnectInfo<SocketAddr>>()
            .map(|ConnectInfo(gegenstelle)| gegenstelle.ip());
        let sitzung = herkunft
            .and_then(|herkunft| gemeldete_sitzung(&teile.headers, herkunft, &zustand.anwesenheit));
        let Some(sitzung) = sitzung else {
            return Err(nicht_erkannt());
        };
        Ok(Teilnehmer {
            kennung: sitzung.kennung,
            name: sitzung.name,
            herkunft: sitzung.herkunft,
            art: sitzung.art,
        })
    }
}

/// Die Sitzung, deren Token die Anfrage im Header nennt — nur von derselben Herkunft; zählt als
/// Lebenszeichen. Fehlt der Header, ist er mehrfach oder unlesbar, gibt es keine.
pub fn gemeldete_sitzung(
    kopf: &HeaderMap,
    herkunft: IpAddr,
    anwesenheit: &Anwesenheit,
) -> Option<Sitzung> {
    let sitzung_header = HeaderName::from_static(SITZUNG_HEADER);
    let token = einziger_wert(kopf, &sitzung_header).and_then(Token::lesen)?;
    anwesenheit.wiederkehren(&token, herkunft)
}

/// Der Nachweis einer Sitzung bei jedem Aufruf (ADR-0012).
pub const SITZUNG_HEADER: &str = "x-localchata-sitzung";

fn nicht_erkannt() -> Response {
    let fehler = "Keine gültige Sitzung — mit POST /api/beitreten beitreten und das Token im Header X-LocalchatA-Sitzung senden (gültig nur von derselben Herkunft).";
    (StatusCode::UNAUTHORIZED, Json(json!({ "fehler": fehler }))).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anwesenheit::Beitrittswunsch;
    use crate::http::testserver;
    use axum::Router;
    use axum::routing::get;
    use std::path::Path;

    /// Testserver mit einer Route, die die Kennung des erkannten Teilnehmers nennt; `claude` ist von
    /// 127.0.0.1 beigetreten. Liefert Basisadresse und Token.
    async fn mit_claude() -> (String, String) {
        let zustand = testserver::zustand(Path::new("/tmp/lc-teilnehmer"));
        let wunsch = Beitrittswunsch::pruefen(&json!({"name": "claude"})).expect("Wunsch");
        let sitzung = zustand
            .anwesenheit
            .beitreten(&wunsch, "127.0.0.1".parse().expect("IP"))
            .expect("beigetreten");
        let app = Router::new()
            .route(
                "/wer",
                get(|teilnehmer: Teilnehmer| async move { teilnehmer.kennung.to_string() }),
            )
            .with_state(Arc::new(zustand));
        let basis = testserver::app_starten(app).await;
        (basis, sitzung.token.als_text().to_string())
    }

    async fn fragen(basis: &str, von: &str, sitzung: &[&str]) -> (u16, String) {
        let client = reqwest::Client::builder()
            .local_address(von.parse::<IpAddr>().expect("IP"))
            .build()
            .expect("Client");
        let mut anfrage = client.get(format!("{basis}wer"));
        for wert in sitzung {
            anfrage = anfrage.header("X-LocalchatA-Sitzung", *wert);
        }
        let antwort = anfrage.send().await.expect("Antwort");
        (
            antwort.status().as_u16(),
            antwort.text().await.expect("Text"),
        )
    }

    fn mit_fehler(antwort: (u16, String)) {
        assert_eq!(antwort.0, 401, "{}", antwort.1);
        let json: serde_json::Value = serde_json::from_str(&antwort.1).expect("JSON");
        assert!(json["fehler"].is_string(), "{json}");
    }

    #[tokio::test]
    async fn gueltiges_token_derselben_herkunft_ergibt_den_teilnehmer() {
        let (basis, token) = mit_claude().await;

        assert_eq!(
            fragen(&basis, "127.0.0.1", &[&token]).await,
            (200, "claude@127.0.0.1".into())
        );
    }

    #[tokio::test]
    async fn ohne_unbekanntes_oder_unlesbares_token_ist_es_401() {
        let (basis, token) = mit_claude().await;
        let gross = token.to_uppercase();

        mit_fehler(fragen(&basis, "127.0.0.1", &[]).await);
        mit_fehler(fragen(&basis, "127.0.0.1", &["00000000000000000000000000000000"]).await);
        mit_fehler(fragen(&basis, "127.0.0.1", &["kein-token"]).await);
        mit_fehler(fragen(&basis, "127.0.0.1", &[&gross]).await);
        mit_fehler(fragen(&basis, "127.0.0.1", &[&token, &token]).await);
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn dasselbe_token_von_einer_anderen_herkunft_ist_401() {
        let (basis, token) = mit_claude().await;

        mit_fehler(fragen(&basis, "127.0.0.2", &[&token]).await);
    }
}
