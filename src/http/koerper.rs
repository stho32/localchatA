//! JSON-Körper lesen (Gate-Stabilität, Befund aus R00003): höchstens `GRENZE` Bytes. Ist der Körper größer,
//! liest der Server den Rest noch bis `VERWERFEN_HOECHSTENS` bzw. `VERWERFEN_FRIST` und verwirft ihn, bevor
//! er antwortet — schlösse er mit ungelesenen Bytes, setzte das Betriebssystem die Verbindung zurück, und der
//! Client sähe einen Abbruch statt der 413. Die Obergrenzen halten Großlast und Tröpfeln aus (ADR-0010).

use axum::body::Body;
use axum::http::HeaderMap;
use axum::http::header::CONTENT_TYPE;
use futures_util::StreamExt;
use serde_json::Value;
use std::time::Duration;

/// Größter angenommener Körper (wie die Grenze von axum zuvor).
pub const GRENZE: usize = 2 * 1024 * 1024;
const VERWERFEN_HOECHSTENS: usize = 16 * 1024 * 1024;
const VERWERFEN_FRIST: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Koerperfehler {
    /// Kein `Content-Type: application/json` (oder `…+json`).
    KeinJson,
    /// Kein gültiges JSON oder ein Lesefehler.
    Kaputt,
    /// Größer als `GRENZE`.
    ZuGross,
}

/// Erst den Körper lesen (begrenzt), dann Typ und Inhalt prüfen — so ist er in jedem Fall gelesen, bevor
/// die Antwort geht.
pub async fn json_lesen(kopf: &HeaderMap, koerper: Body) -> Result<Value, Koerperfehler> {
    let bytes = begrenzt_lesen(koerper).await?;
    if !json_angekuendigt(kopf) {
        return Err(Koerperfehler::KeinJson);
    }
    serde_json::from_slice(&bytes).map_err(|_| Koerperfehler::Kaputt)
}

async fn begrenzt_lesen(koerper: Body) -> Result<Vec<u8>, Koerperfehler> {
    let mut strom = koerper.into_data_stream();
    let mut gelesen = Vec::new();
    while let Some(stueck) = strom.next().await {
        let stueck = stueck.map_err(|_| Koerperfehler::Kaputt)?;
        let gesamt = gelesen.len() + stueck.len();
        if gesamt > GRENZE {
            let _ = tokio::time::timeout(VERWERFEN_FRIST, rest_verwerfen(&mut strom, gesamt)).await;
            return Err(Koerperfehler::ZuGross);
        }
        gelesen.extend_from_slice(&stueck);
    }
    Ok(gelesen)
}

async fn rest_verwerfen(strom: &mut axum::body::BodyDataStream, mut gesamt: usize) {
    while gesamt <= VERWERFEN_HOECHSTENS {
        match strom.next().await {
            Some(Ok(stueck)) => gesamt += stueck.len(),
            _ => return,
        }
    }
}

/// Wie axum: `application/json` oder ein Untertyp mit Endung `+json`, Parameter wie `charset` erlaubt.
fn json_angekuendigt(kopf: &HeaderMap) -> bool {
    let Some(wert) = kopf.get(CONTENT_TYPE).and_then(|wert| wert.to_str().ok()) else {
        return false;
    };
    let typ = wert
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    let Some(untertyp) = typ.strip_prefix("application/") else {
        return false;
    };
    untertyp == "json" || untertyp.ends_with("+json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn mit_typ(typ: &str) -> HeaderMap {
        let mut kopf = HeaderMap::new();
        kopf.insert(CONTENT_TYPE, HeaderValue::from_str(typ).expect("Typ"));
        kopf
    }

    #[test]
    fn json_ist_application_json_oder_plus_json_mit_parametern() {
        for typ in [
            "application/json",
            "application/json; charset=utf-8",
            "Application/JSON",
            "application/problem+json",
        ] {
            assert!(json_angekuendigt(&mit_typ(typ)), "{typ}");
        }
        for typ in [
            "text/plain",
            "application/xml",
            "multipart/form-data",
            "text/json",
            "application/jsonx",
        ] {
            assert!(!json_angekuendigt(&mit_typ(typ)), "{typ}");
        }
        assert!(!json_angekuendigt(&HeaderMap::new()));
    }

    #[tokio::test]
    async fn zu_gross_ab_einem_byte_ueber_der_grenze() {
        let kopf = mit_typ("application/json");
        let genau = format!("\"{}\"", "x".repeat(GRENZE - 2));
        let drueber = format!("\"{}\"", "x".repeat(GRENZE - 1));

        assert!(json_lesen(&kopf, Body::from(genau)).await.is_ok());
        assert_eq!(
            json_lesen(&kopf, Body::from(drueber)).await,
            Err(Koerperfehler::ZuGross)
        );
    }
}
