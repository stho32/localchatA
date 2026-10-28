//! Channels beantworten (B0121, B0122; ADR-0021 Regeln 1, 2, 7): `POST /api/channels {name}` mit Sitzung →
//! 201 `{channel}` | 409 `{fehler, channel}` oder `{fehler, regel: "ordner-vorhanden"}` | 422 `{fehler, regel, vorschlag?}` | 429 `{fehler, grenze,
//! hoechstens}`; `GET /api/channels` → 200
//! `{stand, liste}` nach Name. Ohne Sitzung 401 (Extraktor `Teilnehmer`), fremder `Origin` 403 (Grenzschutz),
//! ohne JSON 415, kaputtes JSON 400, zu großer Körper 413; Plattenfehler 500 `{fehler}` und eine Zeile auf
//! stderr mit Pfad und Grund. Die Arbeit am Verzeichnis läuft über `spawn_blocking`.

use super::Zustand;
use super::koerper::{Koerperfehler, json_lesen};
use super::teilnehmer::Teilnehmer;
use super::teilnehmerliste::zeit_als_text;
use crate::gespraeche::{Anlegefehler, Channel, ChannelStand, Grenzart, Grenze};
use crate::name::{NAMENSREGEL, Namensfehler, RESERVIERT};
use axum::Json;
use axum::body::Body;
use axum::extract::State;
use axum::http::header::CONNECTION;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};
use std::sync::Arc;

pub async fn anlegen(
    teilnehmer: Teilnehmer,
    State(zustand): State<Arc<Zustand>>,
    kopf: HeaderMap,
    koerper: Body,
) -> Response {
    let koerper = match json_lesen(&kopf, koerper).await {
        Ok(koerper) => koerper,
        Err(fehler) => return kein_json(fehler),
    };
    let name = match gewuenschter_name(&koerper) {
        Ok(name) => name,
        Err(abweisung) => return unverarbeitbar(abweisung),
    };
    let gespraeche = Arc::clone(&zustand.gespraeche);
    let kennung = teilnehmer.kennung;
    let gewuenscht = name.clone();
    let ergebnis =
        tokio::task::spawn_blocking(move || gespraeche.channel_anlegen(&gewuenscht, &kennung))
            .await;
    match ergebnis {
        Ok(Ok(channel)) => (
            StatusCode::CREATED,
            Json(json!({ "channel": channel_als_json(&channel) })),
        )
            .into_response(),
        Ok(Err(fehler)) => anlegefehler(fehler, &name),
        Err(abgebrochen) => ablagefehler(&format!("Anlegen abgebrochen: {abgebrochen}")),
    }
}

pub async fn auflisten(_: Teilnehmer, State(zustand): State<Arc<Zustand>>) -> Response {
    let gespraeche = Arc::clone(&zustand.gespraeche);
    match tokio::task::spawn_blocking(move || gespraeche.channels()).await {
        Ok(stand) => Json(stand_als_json(&stand)).into_response(),
        Err(abgebrochen) => ablagefehler(&format!("Liste abgebrochen: {abgebrochen}")),
    }
}

/// `{stand, liste}` — dieselbe Form in `GET /api/channels` und im Ereignis `channels` (ADR-0023 Regel 3).
pub fn stand_als_json(stand: &ChannelStand) -> Value {
    let liste: Vec<Value> = stand.liste.iter().map(channel_als_json).collect();
    json!({ "stand": stand.stand, "liste": liste })
}

/// `{name, angelegt, angelegt_von, beschreibung}`; unbekannte Werte `null`, die Beschreibung bis F0026 immer.
pub fn channel_als_json(channel: &Channel) -> Value {
    json!({
        "name": channel.name.als_text(),
        "angelegt": channel.angelegt.as_ref().map(zeit_als_text),
        "angelegt_von": channel.angelegt_von.as_ref().map(|kennung| kennung.als_text()),
        "beschreibung": null,
    })
}

/// Der Name aus dem Körper; eine Beschreibung, die weder fehlt noch `null` noch `""` ist, wird abgewiesen
/// statt still verworfen (R00006, Ausbaustufe 2 bringt sie). Abgewiesen wird mit dem Inhalt einer 422.
fn gewuenschter_name(koerper: &Value) -> Result<String, Value> {
    let beschreibung = koerper.get("beschreibung").unwrap_or(&Value::Null);
    if !(beschreibung.is_null() || beschreibung == "") {
        return Err(json!({
            "fehler": "Eine Beschreibung gibt es noch nicht — den Channel ohne beschreibung anlegen.",
            "regel": REGEL_BESCHREIBUNG,
        }));
    }
    match koerper.get("name").and_then(Value::as_str) {
        Some(name) => Ok(name.to_string()),
        None => Err(json!({
            "fehler": format!("Der Name fehlt oder ist kein Text — zum Beispiel {BEISPIEL} schicken."),
            "regel": NAMENSREGEL,
        })),
    }
}

const REGEL_BESCHREIBUNG: &str = "beschreibung";
const BEISPIEL: &str = r#"{"name":"allgemein"}"#;

const REGEL_ORDNER_VORHANDEN: &str = "ordner-vorhanden";

fn anlegefehler(fehler: Anlegefehler, name: &str) -> Response {
    match fehler {
        Anlegefehler::Vorhanden(channel) => {
            let text = format!("# {} gibt es schon.", channel.name);
            (
                StatusCode::CONFLICT,
                Json(json!({ "fehler": text, "channel": channel_als_json(&channel) })),
            )
                .into_response()
        }
        Anlegefehler::OrdnerVorhanden => {
            let text = format!(
                "channels/{name} besteht im Ordner schon; localchatA übernimmt ihn beim nächsten Start."
            );
            (
                StatusCode::CONFLICT,
                Json(json!({ "fehler": text, "regel": REGEL_ORDNER_VORHANDEN })),
            )
                .into_response()
        }
        Anlegefehler::Regel(fehler) => regelverstoss(&fehler),
        Anlegefehler::Grenze(grenze) => grenze_erreicht(grenze),
        Anlegefehler::Ablage(grund) => {
            eprintln!("Channel nicht angelegt: {grund}");
            ablagefehler(&grund)
        }
    }
}

/// 429 mit Art und Wert der Grenze; ein Ausweg nur von Hand — es gibt kein Löschen (ADR-0021).
fn grenze_erreicht(grenze: Grenze) -> Response {
    let (art, wessen) = match grenze.art {
        Grenzart::Gesamt => ("gesamt", "insgesamt"),
        Grenzart::Herkunft => ("herkunft", "je Rechner"),
    };
    let text = format!(
        "Zu viele Channels: höchstens {} {wessen} — vorhandene Channels nutzen.",
        grenze.hoechstens
    );
    (
        StatusCode::TOO_MANY_REQUESTS,
        Json(json!({ "fehler": text, "grenze": art, "hoechstens": grenze.hoechstens })),
    )
        .into_response()
}

fn regelverstoss(fehler: &Namensfehler) -> Response {
    let mut antwort = json!({ "regel": fehler.regel });
    antwort["fehler"] = if fehler.regel == RESERVIERT {
        json!(
            "Der Name ist ein Gerätename, den Windows nicht als Ordner zulässt — einen anderen wählen."
        )
    } else {
        json!(format!("Ein Channelname hat {NAMENSREGEL}."))
    };
    if let Some(vorschlag) = &fehler.vorschlag {
        antwort["vorschlag"] = json!(vorschlag.als_text());
    }
    unverarbeitbar(antwort)
}

fn unverarbeitbar(antwort: Value) -> Response {
    (StatusCode::UNPROCESSABLE_ENTITY, Json(antwort)).into_response()
}

fn ablagefehler(grund: &str) -> Response {
    let text = format!("Channel nicht angelegt — {grund}");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "fehler": text })),
    )
        .into_response()
}

/// Wie beim Beitreten: deutsche `{fehler}`-Antwort mit Grund und Ausweg; nach 413 schließt die Verbindung.
fn kein_json(fehler: Koerperfehler) -> Response {
    let (status, grund) = match fehler {
        Koerperfehler::KeinJson => (
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Der Körper muss JSON sein — mit Content-Type: application/json schicken",
        ),
        Koerperfehler::Kaputt => (
            StatusCode::BAD_REQUEST,
            "Der Körper ist kein gültiges JSON — ein JSON-Objekt schicken",
        ),
        Koerperfehler::ZuGross => (
            StatusCode::PAYLOAD_TOO_LARGE,
            "Der Körper ist zu groß — nur den Namen schicken",
        ),
    };
    let antwort = (
        status,
        Json(json!({ "fehler": format!("{grund}, zum Beispiel {BEISPIEL}.") })),
    );
    if fehler == Koerperfehler::ZuGross {
        return ([(CONNECTION, "close")], antwort).into_response();
    }
    antwort.into_response()
}
