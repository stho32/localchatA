//! JSON-Körper lesen (Gate-Stabilität, Befund aus R00003): höchstens so viele Bytes, wie die Route als Grenze
//! nennt (R00010, ADR-0010: jede JSON-Route nennt ihre Grenze mit Herleitung). Ist der Körper größer,
//! liest der Server den Rest noch bis `VERWERFEN_HOECHSTENS` bzw. `VERWERFEN_FRIST` und verwirft ihn, bevor
//! er antwortet — schlösse er mit ungelesenen Bytes, setzte das Betriebssystem die Verbindung zurück, und der
//! Client sähe einen Abbruch statt der 413. Die Obergrenzen halten Großlast und Tröpfeln aus (ADR-0010).

use crate::protokoll::Protokoll;
use crate::protokoll::zaehler::Abweisungsart;
use axum::body::Body;
use axum::http::HeaderMap;
use axum::http::header::CONTENT_TYPE;
use futures_util::StreamExt;
use serde_json::Value;
use std::time::Duration;

const VERWERFEN_HOECHSTENS: usize = 16 * 1024 * 1024;
const VERWERFEN_FRIST: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Koerperfehler {
    /// Kein `Content-Type: application/json` (oder `…+json`).
    KeinJson,
    /// Gelesen, aber kein gültiges JSON.
    Kaputt,
    /// Lesefehler im Strom: der Client ist weg (ADR-0042, Ergänzung 2).
    Abgebrochen,
    /// Größer als die Grenze der Route — erkannt vor jedem Parse.
    ZuGross,
}

/// Wo ein Körper gelesen wird: das Protokoll, das Abweisungen zählt.
pub struct Koerperort<'a> {
    pub protokoll: &'a Protokoll,
}

/// Liest den JSON-Körper wie `lesen`; jede Abweisung zählt in `abgewiesen` (ADR-0043, Ergänzung) — zentral
/// hier, nicht je Handler.
pub async fn json_lesen(
    kopf: &HeaderMap,
    koerper: Body,
    grenze: usize,
    ort: &Koerperort<'_>,
) -> Result<Value, Koerperfehler> {
    let ergebnis = lesen(kopf, koerper, grenze).await;
    if let Err(fehler) = ergebnis {
        ort.protokoll.abgewiesen(match fehler {
            Koerperfehler::ZuGross => Abweisungsart::ZuGross,
            Koerperfehler::KeinJson => Abweisungsart::KeinJson,
            Koerperfehler::Kaputt => Abweisungsart::Kaputt,
            Koerperfehler::Abgebrochen => Abweisungsart::Abgebrochen,
        });
    }
    ergebnis
}

/// Erst den Körper lesen (höchstens `grenze` Bytes), dann Typ und Inhalt prüfen — so ist er in jedem Fall
/// gelesen, bevor die Antwort geht, und über der Grenze wird nichts geparst.
async fn lesen(kopf: &HeaderMap, koerper: Body, grenze: usize) -> Result<Value, Koerperfehler> {
    let bytes = begrenzt_lesen(koerper, grenze).await?;
    if !json_angekuendigt(kopf) {
        return Err(Koerperfehler::KeinJson);
    }
    serde_json::from_slice(&bytes).map_err(|_| Koerperfehler::Kaputt)
}

async fn begrenzt_lesen(koerper: Body, grenze: usize) -> Result<Vec<u8>, Koerperfehler> {
    let mut strom = koerper.into_data_stream();
    let mut gelesen = Vec::new();
    while let Some(stueck) = strom.next().await {
        let stueck = stueck.map_err(|_| Koerperfehler::Abgebrochen)?;
        let gesamt = gelesen.len() + stueck.len();
        if gesamt > grenze {
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

    /// R00010: genau an der Grenze angenommen, ein Byte darüber `ZuGross` — auch ungültiges JSON, denn die
    /// Grenze greift vor dem Parse. Zwei Grenzen, damit keine feste Zahl im Code den Test besteht.
    #[tokio::test]
    async fn grenze_je_aufruf_als_spanne() {
        let kopf = mit_typ("application/json");
        let gueltig = |laenge: usize| format!("{:<laenge$}", "{\"name\":\"spanne\"}");
        let kaputt = |laenge: usize| format!("{{{}", "x".repeat(laenge - 1));

        for grenze in [4096, 64] {
            assert!(
                lesen(&kopf, Body::from(gueltig(grenze)), grenze)
                    .await
                    .is_ok(),
                "{grenze} Bytes an der Grenze {grenze}"
            );
            for koerper in [gueltig(grenze + 1), kaputt(grenze + 1)] {
                assert_eq!(
                    lesen(&kopf, Body::from(koerper), grenze).await,
                    Err(Koerperfehler::ZuGross),
                    "{} Bytes an der Grenze {grenze}",
                    grenze + 1
                );
            }
        }
        assert_eq!(
            lesen(&kopf, Body::from(kaputt(4096)), 4096).await,
            Err(Koerperfehler::Kaputt)
        );
    }
}
