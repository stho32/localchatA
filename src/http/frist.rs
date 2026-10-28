//! Frist setzen (R00008 F0033, ADR-0031): `PUT /api/channels/{name}/frist {tage}` mit Sitzung. Prüfreihenfolge
//! (Regel 2): Grenzschutz 403, Sitzung 401, Körper 415/413/400 → unbekannter Channel 404 → Wert 422 `frist` →
//! defekte Einstellungen 409 `einstellungen-defekt` → 200 `{channel, entfallen}`. Die Arbeit am Verzeichnis läuft
//! über `spawn_blocking`.

use super::Zustand;
use super::abbruch::{Abgebrochen, Grenzort, abbruch_beantworten};
use super::anfrageprotokoll::Eingetragen;
use super::channels::channel_als_json;
use super::koerper::{Koerperfehler, Koerperort, ZU_LANGSAM, json_lesen};
use super::teilnehmer::Teilnehmer;
use crate::gespraeche::{FristGesetzt, Fristfehler, Gespraeche, frist_in_spanne};
use axum::Json;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::header::CONNECTION;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};
use std::sync::Arc;

const ROUTE: &str = "PUT /api/channels/{name}/frist";
const BEISPIEL: &str = r#"{"tage":30}"#;
/// Körpergrenze (R00010, ADR-0010 Ergänzung): der größte sinnvolle Körper `{"tage":3650,"bestaetigt":false}` hat
/// 33 Bytes; mit Raum für Leerraum und Einrückung auf 1 KiB.
const FRIST_GRENZE: usize = 1024;
const REGEL_FRIST: &str = "frist";
const REGEL_EINSTELLUNGEN_DEFEKT: &str = "einstellungen-defekt";

pub async fn setzen(
    teilnehmer: Teilnehmer,
    State(zustand): State<Arc<Zustand>>,
    Path(name): Path<String>,
    kopf: HeaderMap,
    koerper: Body,
) -> Response {
    let ort = Koerperort {
        protokoll: &zustand.protokoll,
        route: ROUTE,
        herkunft: teilnehmer.herkunft,
        kennung: Some(teilnehmer.kennung.als_text()),
    };
    let koerper = match json_lesen(&kopf, koerper, FRIST_GRENZE, &ort).await {
        Ok(koerper) => koerper,
        Err(fehler) => return kein_json(fehler),
    };
    let gespraeche = Arc::clone(&zustand.gespraeche);
    let gewuenscht = name.clone();
    let ergebnis =
        tokio::task::spawn_blocking(move || pruefen_und_setzen(&gespraeche, &gewuenscht, &koerper))
            .await;
    match ergebnis {
        Ok(Ok(gesetzt)) => gesetzt_beantworten(&gesetzt),
        Ok(Err(abweisung)) => abweisung.beantworten(&name),
        Err(abgebrochen) => abbruch_beantworten(
            &zustand.protokoll,
            &Abgebrochen::from(abgebrochen),
            &Grenzort {
                route: ROUTE,
                operation: "frist_setzen",
                kennung: Some(teilnehmer.kennung.als_text()),
                channel: Some(&name),
            },
            "Frist nicht gesetzt",
        ),
    }
}

/// Warum eine Anfrage nach dem Körper abgewiesen wurde.
enum Abweisung {
    Unbekannt,
    Regel(&'static str),
    Frist(Fristfehler),
}

/// Channel vor Wert vor Defekt (ADR-0031 Regel 2.2–2.4).
fn pruefen_und_setzen(
    gespraeche: &Gespraeche,
    name: &str,
    koerper: &Value,
) -> Result<FristGesetzt, Abweisung> {
    if !gespraeche.channel_bekannt(name) {
        return Err(Abweisung::Unbekannt);
    }
    let tage = frist_bilden(koerper.get("tage")).map_err(|_| Abweisung::Regel(REGEL_FRIST))?;
    gespraeche
        .frist_setzen(name, tage)
        .map_err(Abweisung::Frist)
}

impl Abweisung {
    fn beantworten(self, name: &str) -> Response {
        match self {
            Abweisung::Unbekannt | Abweisung::Frist(Fristfehler::Unbekannt) => (
                StatusCode::NOT_FOUND,
                Json(json!({ "fehler": format!("# {name} gibt es nicht.") })),
            )
                .into_response(),
            Abweisung::Regel(regel) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(json!({
                    "fehler": "Die Frist ist eine ganze Zahl von 1 bis 3650 Tagen, zum Beispiel {\"tage\":30}.",
                    "regel": regel,
                })),
            )
                .into_response(),
            Abweisung::Frist(Fristfehler::EinstellungenDefekt { datei }) => (
                StatusCode::CONFLICT,
                Json(json!({
                    "fehler": format!("{datei} ist defekt; von Hand reparieren und neu starten."),
                    "regel": REGEL_EINSTELLUNGEN_DEFEKT,
                    "datei": datei,
                })),
            )
                .into_response(),
            Abweisung::Frist(Fristfehler::Ablage { datei, grund }) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "fehler": format!("Frist nicht gesetzt — {datei} ließ sich nicht schreiben ({grund})."),
                })),
            )
                .into_response(),
        }
    }
}

fn gesetzt_beantworten(gesetzt: &FristGesetzt) -> Response {
    Json(json!({
        "channel": channel_als_json(&gesetzt.channel),
        "entfallen": { "tage": 0, "nachrichten": 0 },
    }))
    .into_response()
}

/// Wie beim Anlegen: deutsche `{fehler}`-Antwort mit Grund und Beispiel; nach 413 und 408 schließt die Verbindung.
fn kein_json(fehler: Koerperfehler) -> Response {
    let mit_beispiel = |grund: &str| format!("{grund}, zum Beispiel {BEISPIEL}.");
    let (status, text) = match fehler {
        Koerperfehler::KeinJson => (
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            mit_beispiel("Der Körper muss JSON sein — mit Content-Type: application/json schicken"),
        ),
        Koerperfehler::Kaputt | Koerperfehler::Abgebrochen => (
            StatusCode::BAD_REQUEST,
            mit_beispiel("Der Körper ist kein gültiges JSON — ein JSON-Objekt schicken"),
        ),
        Koerperfehler::ZuGross => (
            StatusCode::PAYLOAD_TOO_LARGE,
            mit_beispiel(&format!(
                "Der Körper ist zu groß — höchstens {FRIST_GRENZE} Bytes, nur die Tage schicken"
            )),
        ),
        Koerperfehler::ZuLangsam => (StatusCode::REQUEST_TIMEOUT, ZU_LANGSAM.to_string()),
    };
    let antwort = (status, Json(json!({ "fehler": text })));
    let schliessen = matches!(fehler, Koerperfehler::ZuGross | Koerperfehler::ZuLangsam);
    let mut antwort = if schliessen {
        ([(CONNECTION, "close")], antwort).into_response()
    } else {
        antwort.into_response()
    };
    antwort.extensions_mut().insert(Eingetragen);
    antwort
}

/// `tage` ist keine ganze Zahl von 1 bis 3650 — abgewiesen, nie umgeformt (ADR-0031 Regel 2.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Regelfehler;

/// Der JSON-Wert `tage` als Frist: nur eine ganze Zahl (`"7"`, `7.0`, `7.5`, `null` und Fehlen nicht) in der Spanne.
pub fn frist_bilden(wert: Option<&Value>) -> Result<u32, Regelfehler> {
    wert.and_then(Value::as_i64)
        .and_then(frist_in_spanne)
        .ok_or(Regelfehler)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn gebildet(koerper: Value) -> Result<u32, Regelfehler> {
        frist_bilden(koerper.get("tage"))
    }

    #[test]
    fn eins_und_3650_gelten_als_spanne() {
        assert_eq!(gebildet(json!({ "tage": 1 })), Ok(1));
        assert_eq!(gebildet(json!({ "tage": 7 })), Ok(7));
        assert_eq!(gebildet(json!({ "tage": 3650 })), Ok(3650));
    }

    #[test]
    fn null_3651_und_negative_liegen_ausserhalb() {
        for tage in [
            json!(0),
            json!(3651),
            json!(-1),
            json!(i64::MAX),
            json!(u64::MAX),
        ] {
            assert_eq!(
                gebildet(json!({ "tage": tage })),
                Err(Regelfehler),
                "{tage}"
            );
        }
    }

    #[test]
    fn keine_ganze_zahl_ist_ein_regelfehler_und_wird_nicht_umgeformt() {
        let koerper: Value = serde_json::from_str(r#"{"tage":7.0}"#).expect("JSON");
        assert_eq!(gebildet(koerper), Err(Regelfehler));
        for tage in [
            json!("7"),
            json!(7.5),
            Value::Null,
            json!([7]),
            json!({ "n": 7 }),
            json!(true),
        ] {
            assert_eq!(
                gebildet(json!({ "tage": tage })),
                Err(Regelfehler),
                "{tage}"
            );
        }
        assert_eq!(gebildet(json!({})), Err(Regelfehler));
    }
}
