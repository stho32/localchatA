//! Beitreten beantworten (B0049, B0055): `POST /api/beitreten {name, art?}` → 201 mit Token und Kennung,
//! 409 Name vergeben, 422 Regelverstoß, 429 zu viele Sitzungen; ohne JSON 415 (axum `Json`). Mit gültigem
//! Token derselben Herkunft im Header ist es eine Wiederkehr: 200 dieselbe Sitzung, anderer Name 422.

use super::Zustand;
use super::teilnehmer::gemeldete_sitzung;
use crate::anwesenheit::{
    Abweisung, Beitrittsfehler, Beitrittswunsch, GESAMT, JE_HERKUNFT, Sitzung,
};
use axum::Json;
use axum::extract::{ConnectInfo, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use chrono::SecondsFormat;
use serde_json::{Value, json};
use std::net::SocketAddr;
use std::sync::Arc;

pub async fn beantworten(
    State(zustand): State<Arc<Zustand>>,
    ConnectInfo(gegenstelle): ConnectInfo<SocketAddr>,
    kopf: HeaderMap,
    Json(koerper): Json<Value>,
) -> Response {
    let wunsch = match Beitrittswunsch::pruefen(&koerper) {
        Ok(wunsch) => wunsch,
        Err(abweisung) => return regelverstoss(abweisung),
    };
    let anwesenheit = &zustand.anwesenheit;
    if let Some(sitzung) = gemeldete_sitzung(&kopf, gegenstelle.ip(), anwesenheit) {
        return wiederkehr(&sitzung, &wunsch, anwesenheit.gnadenfrist().as_secs());
    }
    match anwesenheit.beitreten(&wunsch, gegenstelle.ip()) {
        Ok(sitzung) => {
            let gnadenfrist_s = anwesenheit.gnadenfrist().as_secs();
            (
                StatusCode::CREATED,
                sitzung_als_json(&sitzung, gnadenfrist_s),
            )
                .into_response()
        }
        Err(fehler) => beitrittsfehler(fehler, &wunsch, gegenstelle),
    }
}

/// Ein Name je Sitzung: denselben Namen gibt es zurück, einen Namenswechsel gibt es nicht — auch nicht still.
fn wiederkehr(sitzung: &Sitzung, wunsch: &Beitrittswunsch, gnadenfrist_s: u64) -> Response {
    if sitzung.name == wunsch.name {
        return (StatusCode::OK, sitzung_als_json(sitzung, gnadenfrist_s)).into_response();
    }
    regelverstoss(Abweisung {
        fehler: format!(
            "Diese Sitzung trägt den Namen {} — einen Namenswechsel gibt es nicht.",
            sitzung.name
        ),
        regel: REGEL_EIN_NAME,
        vorschlag: None,
    })
}

const REGEL_EIN_NAME: &str = "eine Sitzung trägt genau einen Namen";

/// Die Antwort mit Token — nur hier verlässt es den Server (ADR-0012).
fn sitzung_als_json(sitzung: &Sitzung, gnadenfrist_s: u64) -> Json<Value> {
    Json(json!({
        "token": sitzung.token.als_text(),
        "kennung": sitzung.kennung.als_text(),
        "name": sitzung.name.als_text(),
        "herkunft": sitzung.herkunft.to_string(),
        "art": sitzung.art,
        "gnadenfrist_s": gnadenfrist_s,
    }))
}

fn regelverstoss(abweisung: Abweisung) -> Response {
    let mut antwort = json!({ "fehler": abweisung.fehler, "regel": abweisung.regel });
    if let Some(vorschlag) = abweisung.vorschlag {
        antwort["vorschlag"] = json!(vorschlag.als_text());
    }
    (StatusCode::UNPROCESSABLE_ENTITY, Json(antwort)).into_response()
}

fn beitrittsfehler(
    fehler: Beitrittsfehler,
    wunsch: &Beitrittswunsch,
    gegenstelle: SocketAddr,
) -> Response {
    match fehler {
        Beitrittsfehler::NameVergeben {
            belegt_von,
            seit,
            vorschlag,
        } => {
            let text = format!(
                "Der Name {} ist vergeben — {belegt_von} ist seit {} dabei. Wie wäre {vorschlag}?",
                wunsch.name,
                seit.format("%H:%M")
            );
            let antwort = json!({
                "fehler": text,
                "belegt_von": belegt_von.als_text(),
                "seit": seit.to_rfc3339_opts(SecondsFormat::Millis, false),
                "vorschlag": vorschlag.als_text(),
            });
            (StatusCode::CONFLICT, Json(antwort)).into_response()
        }
        Beitrittsfehler::ZuVieleSitzungen => {
            let text = format!(
                "Zu viele Sitzungen — höchstens {JE_HERKUNFT} je Herkunft ({}) und {GESAMT} insgesamt.",
                gegenstelle.ip()
            );
            (
                StatusCode::TOO_MANY_REQUESTS,
                Json(json!({ "fehler": text })),
            )
                .into_response()
        }
        Beitrittsfehler::KeinZufall(fehler) => {
            let text = format!("Kein Zufall für ein Sitzungs-Token: {}", fehler.0);
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "fehler": text })),
            )
                .into_response()
        }
    }
}
