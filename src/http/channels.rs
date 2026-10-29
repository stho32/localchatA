//! Channels beantworten (B0121, B0122; ADR-0021 Regeln 1, 2, 7): `POST /api/channels {name}` mit Sitzung →
//! 201 `{channel}` (Beschreibung optional, R00007 F0026) | 409 `{fehler, channel}` oder `{fehler, regel: "ordner-vorhanden"}` | 422 `{fehler, regel, vorschlag?}` | 429 `{fehler, grenze,
//! hoechstens}`; `GET /api/channels` → 200
//! `{stand, liste}` nach Name. Ohne Sitzung 401 (Extraktor `Teilnehmer`), fremder `Origin` 403 (Grenzschutz),
//! ohne JSON 415, kaputtes JSON 400, zu großer Körper 413; Plattenfehler 500 `{fehler}` und eine Zeile auf
//! stderr mit Pfad und Grund. Die Arbeit am Verzeichnis läuft über `spawn_blocking`.

use super::Zustand;
use super::abbruch::{Abgebrochen, Grenzort, abbruch_beantworten};
use super::anfrageprotokoll::mit_regel;
use super::koerper::{Koerperfehler, Koerperort, json_lesen, koerperfehler_beantworten};
use super::teilnehmer::Teilnehmer;
use super::teilnehmerliste::zeit_als_text;
use crate::gespraeche::einstellungen::{Beschreibung, Beschreibungsfehler};
use crate::gespraeche::{Anlegefehler, Channel, ChannelStand, Frist, Grenzart, Grenze};
use crate::name::{NAMENSREGEL, Namensfehler, RESERVIERT};
use crate::protokoll::{Bereich, Kontext, Schluessel, Stufe};
use axum::Json;
use axum::body::Body;
use axum::extract::State;
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
    let ort = Koerperort {
        protokoll: &zustand.protokoll,
        route: "POST /api/channels",
        herkunft: teilnehmer.herkunft,
        kennung: Some(teilnehmer.kennung.als_text()),
    };
    let koerper = match json_lesen(&kopf, koerper, CHANNEL_ANLEGEN_GRENZE, &ort).await {
        Ok(koerper) => koerper,
        Err(fehler) => return kein_json(fehler),
    };
    let (name, beschreibung) = match gewuenscht(&koerper) {
        Ok(gewuenscht) => gewuenscht,
        Err((abweisung, regel)) => {
            return mit_regel(
                unverarbeitbar(abweisung),
                regel,
                Some(teilnehmer.kennung.als_text()),
            );
        }
    };
    let gespraeche = Arc::clone(&zustand.gespraeche);
    let herkunft = teilnehmer.herkunft;
    let kennung = teilnehmer.kennung;
    let wer = kennung.als_text().to_string();
    let gewuenscht = name.clone();
    let ergebnis = tokio::task::spawn_blocking(move || {
        gespraeche.channel_anlegen(&gewuenscht, beschreibung, &kennung)
    })
    .await;
    match ergebnis {
        Ok(Ok(channel)) => {
            zustand.protokoll.eintragen(
                Stufe::Information,
                Bereich::Channels,
                "Channel angelegt",
                Kontext::neu()
                    .mit(Schluessel::Channel, channel.name.als_text())
                    .mit(Schluessel::Kennung, &wer),
            );
            (
                StatusCode::CREATED,
                Json(json!({ "channel": channel_als_json(&channel) })),
            )
                .into_response()
        }
        Ok(Err(Anlegefehler::Ablage(grund))) => {
            zustand.protokoll.eintragen(
                Stufe::Error,
                Bereich::Channels,
                "Channel nicht angelegt — Ordner oder Einstellungen ließen sich nicht schreiben",
                Kontext::neu()
                    .mit(Schluessel::Channel, &name)
                    .mit(Schluessel::Kennung, &wer)
                    .mit(Schluessel::Grund, &grund),
            );
            ablagefehler(&grund)
        }
        Ok(Err(Anlegefehler::Grenze(grenze))) => {
            let art = match grenze.art {
                Grenzart::Gesamt => "gesamt",
                Grenzart::Herkunft => "herkunft",
            };
            super::beitreten::grenze_eintragen(
                &zustand.protokoll,
                Bereich::Channels,
                "Anlegen an der Channel-Grenze abgewiesen",
                herkunft,
                format!("{} {art}", grenze.hoechstens),
            );
            grenze_erreicht(grenze)
        }
        Ok(Err(fehler)) => {
            let regel = anlegefehler_regel(&fehler);
            mit_regel(anlegefehler(fehler, &name), regel, Some(&wer))
        }
        Err(abgebrochen) => abbruch_beantworten(
            &zustand.protokoll,
            &Abgebrochen::from(abgebrochen),
            &Grenzort {
                route: "POST /api/channels",
                operation: "channel_anlegen",
                kennung: Some(&wer),
                channel: Some(&name),
            },
            "Channel nicht angelegt",
        ),
    }
}

pub async fn auflisten(teilnehmer: Teilnehmer, State(zustand): State<Arc<Zustand>>) -> Response {
    let gespraeche = Arc::clone(&zustand.gespraeche);
    match tokio::task::spawn_blocking(move || gespraeche.channels()).await {
        Ok(stand) => Json(stand_als_json(&stand)).into_response(),
        Err(abgebrochen) => abbruch_beantworten(
            &zustand.protokoll,
            &Abgebrochen::from(abgebrochen),
            &Grenzort {
                route: "GET /api/channels",
                operation: "channels",
                kennung: Some(teilnehmer.kennung.als_text()),
                channel: None,
            },
            "Channels nicht gelesen",
        ),
    }
}

/// `{stand, liste}` — dieselbe Form in `GET /api/channels` und im Ereignis `channels` (ADR-0023 Regel 3).
pub fn stand_als_json(stand: &ChannelStand) -> Value {
    let liste: Vec<Value> = stand.liste.iter().map(channel_als_json).collect();
    json!({ "stand": stand.stand, "liste": liste })
}

/// `{name, angelegt, angelegt_von, beschreibung, frist_tage}`; unbekannte Werte und keine Beschreibung `null`,
/// `frist_tage` `null` bei defekten Einstellungen — dann entfällt nichts (ADR-0031 Regel 3).
pub fn channel_als_json(channel: &Channel) -> Value {
    let frist_tage = match channel.frist {
        Frist::Tage(tage) => Some(tage),
        Frist::Gesperrt { .. } => None,
    };
    json!({
        "name": channel.name.als_text(),
        "angelegt": channel.angelegt.as_ref().map(zeit_als_text),
        "angelegt_von": channel.angelegt_von.as_ref().map(|kennung| kennung.als_text()),
        "beschreibung": channel.beschreibung.as_ref().map(Beschreibung::als_text),
        "frist_tage": frist_tage,
    })
}

/// Name und Beschreibung aus dem Körper; die Beschreibung fehlt, ist `null` oder `""` (keine) oder ein Text nach
/// der Regel (ADR-0022 Regel 5) — sonst abgewiesen mit dem Inhalt einer 422, nichts angelegt.
/// Im Fehler mit dem Regelschlüssel fürs Protokoll.
fn gewuenscht(koerper: &Value) -> Result<(String, Option<Beschreibung>), (Value, &'static str)> {
    let beschreibung = beschreibung_lesen(koerper.get("beschreibung")).map_err(|_| {
        (
            json!({
                "fehler": "Eine Beschreibung ist ein Satz aus 1 bis 200 Zeichen ohne Zeilenumbruch und Tabulator.",
                "regel": REGEL_BESCHREIBUNG,
            }),
            "beschreibung_ungueltig",
        )
    })?;
    match koerper.get("name").and_then(Value::as_str) {
        Some(name) => Ok((name.to_string(), beschreibung)),
        None => Err((
            json!({
                "fehler": format!("Der Name fehlt oder ist kein Text — zum Beispiel {BEISPIEL} schicken."),
                "regel": NAMENSREGEL,
            }),
            "name_fehlt",
        )),
    }
}

/// Fehlt, `null` oder `""` → keine; ein Text → nach der Regel; alles andere ist kein Text und verstößt.
fn beschreibung_lesen(wert: Option<&Value>) -> Result<Option<Beschreibung>, Beschreibungsfehler> {
    match wert {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => Beschreibung::bilden(text),
        Some(_) => Err(Beschreibungsfehler),
    }
}

const REGEL_BESCHREIBUNG: &str = "beschreibung";
const BEISPIEL: &str = r#"{"name":"allgemein"}"#;

const REGEL_ORDNER_VORHANDEN: &str = "ordner-vorhanden";

/// Regelschlüssel einer abgewiesenen Anlage (ADR-0042 Ergänzung #24 Punkt 3).
fn anlegefehler_regel(fehler: &Anlegefehler) -> &'static str {
    match fehler {
        Anlegefehler::Vorhanden(_) => "channel_existiert",
        Anlegefehler::OrdnerVorhanden => "ordner_vorhanden",
        Anlegefehler::Regel(_) => "name_ungueltig",
        Anlegefehler::Grenze(_) => "channel_grenze",
        Anlegefehler::Ablage(_) => "ablage",
    }
}

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
        Anlegefehler::Ablage(grund) => ablagefehler(&grund),
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

/// Körpergrenze der Anlage (R00010): der größte gültige Körper hat 2 621 Bytes (Name 32 × `\u0061`,
/// Beschreibung 200 Ersatzpaare je 12 Bytes); mit 30 % Reserve 3 408, auf 4 KiB gerundet.
const CHANNEL_ANLEGEN_GRENZE: usize = 4096;

/// Die eine Antwort für Körperfehler (`koerper::koerperfehler_beantworten`) mit dem 413-Grund und Beispiel der Route.
fn kein_json(fehler: Koerperfehler) -> Response {
    koerperfehler_beantworten(
        fehler,
        &format!(
            "Der Körper ist zu groß — höchstens {CHANNEL_ANLEGEN_GRENZE} Bytes, nur Name und Beschreibung schicken"
        ),
        BEISPIEL,
    )
}
