//! Aktivität setzen und löschen (R00045 F0161, ADR-0051 Regel 4): `PUT /api/channels/{name}/aktivitaet
//! {"zustand":"tippt"}` → 200 `{zustand, bis, jetzt}`, `DELETE` → 204 (auch ohne aktiven Zustand). Die Kennung kommt nur
//! aus der Sitzung. Prüfreihenfolge: Grenzschutz 403 → Sitzung 401 → Körper 415/413/400 → Gespräch 404 (nicht sichtbar
//! wie unbekannt) → Wert 422 (`zustand`, `feld`, `aktivitaeten`) → Wirkung. Stufe I0040: nur `tippt`, ohne `gueltig_s`
//! (B0695 schaltet `denkt_nach` und `gueltig_s` frei). Kein Eintrag im Betriebsprotokoll, wenn es gelingt (Regel 11).

use super::Zustand;
use super::anfrageprotokoll::mit_regel;
use super::koerper::{Koerperfehler, Koerperort, json_lesen, koerperfehler_beantworten};
use super::senden::unbekannt;
use super::teilnehmer::Teilnehmer;
use super::teilnehmerliste::zeit_als_text;
use crate::aktivitaet::{self, Abweisung, Gesetzt};
use crate::gespraeche::Gespraech;
use axum::Json;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};
use std::sync::Arc;

const ROUTE: &str = "PUT /api/channels/{name}/aktivitaet";
const BEISPIEL: &str = r#"{"zustand":"tippt"}"#;
/// Körpergrenze (ADR-0051 Regel 4): der größte gültige Körper `{"zustand":"denkt_nach","gueltig_s":120}` hat 40 Bytes;
/// mit Raum für Leerraum und Einrückung 1 KiB.
const GRENZE: usize = 1_024;

pub async fn setzen(
    teilnehmer: Teilnehmer,
    State(zustand): State<Arc<Zustand>>,
    Path(name): Path<String>,
    kopf: HeaderMap,
    koerper: Body,
) -> Response {
    let wer = teilnehmer.kennung.als_text();
    let ort = Koerperort {
        protokoll: &zustand.protokoll,
        route: ROUTE,
        herkunft: teilnehmer.herkunft,
        kennung: Some(wer),
    };
    let koerper = match json_lesen(&kopf, koerper, GRENZE, &ort).await {
        Ok(koerper) => koerper,
        Err(fehler) => return koerper_abgewiesen(fehler),
    };
    let Some(gespraech) = sichtbares_gespraech(&zustand, &name, &teilnehmer) else {
        return mit_regel(unbekannt(&name), "channel_unbekannt", Some(wer));
    };
    let gewuenscht = match wunsch_lesen(&koerper) {
        Ok(gewuenscht) => gewuenscht,
        Err(wertfehler) => {
            return mit_regel(wertfehler.beantworten(), wertfehler.regel(), Some(wer));
        }
    };
    match zustand
        .aktivitaet
        .setzen(&teilnehmer.kennung, &gespraech, gewuenscht, None)
    {
        Ok(gesetzt) => gesetzt_beantworten(gesetzt),
        Err(abweisung) => {
            let wertfehler = Wertfehler::Abgewiesen(abweisung);
            mit_regel(wertfehler.beantworten(), wertfehler.regel(), Some(wer))
        }
    }
}

pub async fn loeschen(
    teilnehmer: Teilnehmer,
    State(zustand): State<Arc<Zustand>>,
    Path(name): Path<String>,
) -> Response {
    let Some(gespraech) = sichtbares_gespraech(&zustand, &name, &teilnehmer) else {
        return mit_regel(
            unbekannt(&name),
            "channel_unbekannt",
            Some(teilnehmer.kennung.als_text()),
        );
    };
    zustand.aktivitaet.beenden(&teilnehmer.kennung, &gespraech);
    StatusCode::NO_CONTENT.into_response()
}

/// Das Gespräch des Pfads, wenn es bekannt und für den Aufrufer sichtbar ist — sonst keines (404 wie unbekannt, kein
/// Orakel). Archivierte Channels gibt es erst mit I0022.
fn sichtbares_gespraech(
    zustand: &Zustand,
    name: &str,
    teilnehmer: &Teilnehmer,
) -> Option<Gespraech> {
    let gespraech = Gespraech::channel(name);
    let sichtbar = zustand.gespraeche.channel_bekannt(name)
        && aktivitaet::sichtbar(&gespraech, &teilnehmer.kennung, false);
    sichtbar.then_some(gespraech)
}

/// Warum ein Körper keinen Zustand ergibt.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Wertfehler {
    /// Ein anderes Feld als `zustand` — auch `kennung` und (bis I0041) `gueltig_s`.
    Feld(String),
    /// `zustand` fehlt, ist kein Text oder nicht der feste Wortlaut.
    Zustand,
    Abgewiesen(Abweisung),
}

impl Wertfehler {
    fn regel(&self) -> &'static str {
        match self {
            Wertfehler::Feld(_) => "feld",
            Wertfehler::Zustand => "zustand",
            Wertfehler::Abgewiesen(Abweisung::Gueltigkeit) => "gueltigkeit",
            Wertfehler::Abgewiesen(Abweisung::ZuViele) => "aktivitaeten",
        }
    }

    fn beantworten(&self) -> Response {
        let fehler = match self {
            Wertfehler::Feld(feld) => {
                format!("Unbekanntes Feld {feld} — erlaubt ist zustand, zum Beispiel {BEISPIEL}.")
            }
            Wertfehler::Zustand => {
                format!("zustand ist \"tippt\", zum Beispiel {BEISPIEL}.")
            }
            Wertfehler::Abgewiesen(Abweisung::Gueltigkeit) => {
                "gueltig_s ist eine ganze Zahl von 1 bis 120.".to_string()
            }
            Wertfehler::Abgewiesen(Abweisung::ZuViele) => format!(
                "Höchstens {} Gespräche gleichzeitig — erst einen Zustand löschen.",
                aktivitaet::JE_KENNUNG_HOECHSTENS
            ),
        };
        let mut antwort = json!({ "fehler": fehler, "regel": self.regel() });
        if let Wertfehler::Feld(feld) = self {
            antwort["feld"] = json!(feld);
        }
        (StatusCode::UNPROCESSABLE_ENTITY, Json(antwort)).into_response()
    }
}

/// Der gewünschte Zustand: genau das Feld `zustand` mit dem Wortlaut `tippt` (Stufe I0040).
fn wunsch_lesen(koerper: &Value) -> Result<aktivitaet::Zustand, Wertfehler> {
    let objekt = koerper.as_object().ok_or(Wertfehler::Zustand)?;
    if let Some(feld) = objekt.keys().find(|feld| *feld != "zustand") {
        return Err(Wertfehler::Feld(feld.clone()));
    }
    objekt
        .get("zustand")
        .and_then(Value::as_str)
        .and_then(aktivitaet::Zustand::lesen)
        .filter(|zustand| *zustand == aktivitaet::Zustand::Tippt)
        .ok_or(Wertfehler::Zustand)
}

fn gesetzt_beantworten(gesetzt: Gesetzt) -> Response {
    Json(json!({
        "zustand": gesetzt.zustand.als_text(),
        "bis": zeit_als_text(&gesetzt.bis),
        "jetzt": zeit_als_text(&gesetzt.jetzt),
    }))
    .into_response()
}

/// Die eine Antwort für Körperfehler mit dem 413-Satz und dem Beispiel der Route.
fn koerper_abgewiesen(fehler: Koerperfehler) -> Response {
    koerperfehler_beantworten(
        fehler,
        &format!(
            "Der Körper ist zu groß — höchstens {GRENZE} Bytes, nur den Zustand schicken, zum Beispiel {BEISPIEL}."
        ),
        BEISPIEL,
    )
}
