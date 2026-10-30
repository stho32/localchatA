//! Aktivität setzen, löschen und lesen (R00045 F0161, I0041; ADR-0051 Regel 4): `PUT /api/channels/{name}/aktivitaet
//! {"zustand":"tippt"|"denkt_nach","gueltig_s"?:1..120}` → 200 `{zustand, bis, jetzt}`, `DELETE` → 204 (auch ohne
//! aktiven Zustand). Die Kennung kommt nur aus der Sitzung. Prüfreihenfolge: Grenzschutz 403 → Sitzung 401 → Körper
//! 415/413/400 → Gespräch 404 (nicht sichtbar wie unbekannt) → Wert 422 (`zustand`, `gueltigkeit`, `feld`,
//! `aktivitaeten`) → Bremse 429 (Episode `WARNING aktivitaet`) → Wirkung. Kein Eintrag im Betriebsprotokoll, wenn es
//! gelingt (Regel 11).
//!
//! Dieselben Handler bedienen `PUT|DELETE|GET /api/direkt/{kennung}/aktivitaet` (ADR-0055 Regel 3, ADR-0051 Regel
//! 10): das Paar entsteht aus Sitzung und Pfad (nach der Sitzung 401: Kennung 422, `selbst` 422), und ohne
//! bestehendes Gespräch ist die Antwort ein 404 wie für ein nie vergebenes.

use super::Zustand;
use super::anfrageprotokoll::mit_regel;
use super::bearbeiten::{unbekannt_regel, unbekannt_wie};
use super::koerper::{Koerperfehler, Koerperort, json_lesen, koerperfehler_beantworten};
use super::teilnehmer::Teilnehmer;
use super::teilnehmerliste::zeit_als_text;
use super::ziel::{Segment, Ziel, bezug, ziel_aus_pfad};
use crate::aktivitaet::{self, Abweisung, Gebremst, Gesetzt, Grenze};
use crate::gespraeche::{Gespraech, sichtbar_fuer};
use crate::protokoll::episoden::{Auftreten, Quelle};
use crate::protokoll::{Bereich, Protokoll, Schluessel, Stufe, episodentexte};
use axum::Json;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::header::RETRY_AFTER;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};
use std::sync::Arc;

const ROUTE: &str = "PUT /api/channels/{name}/aktivitaet";
const ROUTE_DIREKT: &str = "PUT /api/direkt/{kennung}/aktivitaet";
const BEISPIEL: &str = r#"{"zustand":"denkt_nach","gueltig_s":60}"#;
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
    setzen_in(teilnehmer, zustand, Ziel::Channel(name), kopf, koerper).await
}

pub(super) async fn setzen_direkt(
    teilnehmer: Teilnehmer,
    State(zustand): State<Arc<Zustand>>,
    segment: Segment,
    kopf: HeaderMap,
    koerper: Body,
) -> Response {
    match ziel_aus_pfad(&teilnehmer, &segment) {
        Ok(ziel) => setzen_in(teilnehmer, zustand, ziel, kopf, koerper).await,
        Err(fehler) => fehler.antwort(&teilnehmer.kennung),
    }
}

async fn setzen_in(
    teilnehmer: Teilnehmer,
    zustand: Arc<Zustand>,
    ziel: Ziel,
    kopf: HeaderMap,
    koerper: Body,
) -> Response {
    let wer = teilnehmer.kennung.als_text();
    let ort = Koerperort {
        protokoll: &zustand.protokoll,
        route: match ziel {
            Ziel::Channel(_) => ROUTE,
            Ziel::Direkt { .. } => ROUTE_DIREKT,
        },
        herkunft: teilnehmer.herkunft,
        kennung: Some(wer),
    };
    let koerper = match json_lesen(&kopf, koerper, GRENZE, &ort).await {
        Ok(koerper) => koerper,
        Err(fehler) => return koerper_abgewiesen(fehler),
    };
    let Some(gespraech) = bestehendes_gespraech(&zustand, &ziel, &teilnehmer) else {
        return unbekannt_beantworten(&ziel, wer);
    };
    let wunsch = match wunsch_lesen(&koerper) {
        Ok(wunsch) => wunsch,
        Err(wertfehler) => return abgewiesen(&wertfehler, wer),
    };
    match zustand.aktivitaet.setzen(
        &teilnehmer.kennung,
        teilnehmer.herkunft,
        &gespraech,
        wunsch.zustand,
        wunsch.gueltig_s,
    ) {
        Ok(gesetzt) => gesetzt_beantworten(gesetzt),
        Err(Abweisung::Gebremst(gebremst)) => {
            gebremst_melden(&zustand.protokoll, gebremst, &teilnehmer, &ziel)
        }
        Err(Abweisung::Gueltigkeit) => abgewiesen(&Wertfehler::Gueltigkeit, wer),
        Err(Abweisung::ZuViele) => abgewiesen(&Wertfehler::ZuViele, wer),
    }
}

pub async fn loeschen(
    teilnehmer: Teilnehmer,
    State(zustand): State<Arc<Zustand>>,
    Path(name): Path<String>,
) -> Response {
    loeschen_in(&teilnehmer, &zustand, &Ziel::Channel(name))
}

pub(super) async fn loeschen_direkt(
    teilnehmer: Teilnehmer,
    State(zustand): State<Arc<Zustand>>,
    segment: Segment,
) -> Response {
    match ziel_aus_pfad(&teilnehmer, &segment) {
        Ok(ziel) => loeschen_in(&teilnehmer, &zustand, &ziel),
        Err(fehler) => fehler.antwort(&teilnehmer.kennung),
    }
}

fn loeschen_in(teilnehmer: &Teilnehmer, zustand: &Zustand, ziel: &Ziel) -> Response {
    let Some(gespraech) = bestehendes_gespraech(zustand, ziel, teilnehmer) else {
        return unbekannt_beantworten(ziel, teilnehmer.kennung.als_text());
    };
    zustand.aktivitaet.beenden(&teilnehmer.kennung, &gespraech);
    StatusCode::NO_CONTENT.into_response()
}

/// `GET`: die gültigen Zustände des Gesprächs ohne den des Lesenden, nach Kennung sortiert, mit `jetzt` (Regel 4);
/// kostet nichts an der Bremse und erzeugt kein Signal.
pub async fn lesen(
    teilnehmer: Teilnehmer,
    State(zustand): State<Arc<Zustand>>,
    Path(name): Path<String>,
) -> Response {
    lesen_in(&teilnehmer, &zustand, &Ziel::Channel(name))
}

pub(super) async fn lesen_direkt(
    teilnehmer: Teilnehmer,
    State(zustand): State<Arc<Zustand>>,
    segment: Segment,
) -> Response {
    match ziel_aus_pfad(&teilnehmer, &segment) {
        Ok(ziel) => lesen_in(&teilnehmer, &zustand, &ziel),
        Err(fehler) => fehler.antwort(&teilnehmer.kennung),
    }
}

fn lesen_in(teilnehmer: &Teilnehmer, zustand: &Zustand, ziel: &Ziel) -> Response {
    let Some(gespraech) = bestehendes_gespraech(zustand, ziel, teilnehmer) else {
        return unbekannt_beantworten(ziel, teilnehmer.kennung.als_text());
    };
    let gelesen = zustand.aktivitaet.lesen(&gespraech, &teilnehmer.kennung);
    let aktivitaeten: Vec<Value> = gelesen
        .aktivitaeten
        .iter()
        .map(|aktiv| {
            json!({
                "kennung": aktiv.kennung.als_text(),
                "zustand": aktiv.zustand.als_text(),
                "bis": zeit_als_text(&aktiv.bis),
            })
        })
        .collect();
    Json(json!({ "jetzt": zeit_als_text(&gelesen.jetzt), "aktivitaeten": aktivitaeten }))
        .into_response()
}

/// Das Gespräch der Route, wenn es besteht — sonst keines (404 wie unbekannt, kein Orakel). Ein Channel muss dem
/// Aufrufer sichtbar sein (archivierte gibt es erst mit I0022); ein Direktgespräch ist durch den Bau sichtbar: das
/// Paar entstand aus der Sitzung und dem Pfad, es besteht mit mindestens einer Nachricht in der Frist (ADR-0055
/// Regel 3). Ohne Gespräch wird nichts angelegt.
fn bestehendes_gespraech(
    zustand: &Zustand,
    ziel: &Ziel,
    teilnehmer: &Teilnehmer,
) -> Option<Gespraech> {
    match ziel {
        Ziel::Channel(name) => {
            let gespraech = Gespraech::channel(name);
            let sichtbar = zustand.gespraeche.channel_bekannt(name)
                && sichtbar_fuer(&gespraech, &teilnehmer.kennung, false);
            sichtbar.then_some(gespraech)
        }
        Ziel::Direkt { paar, .. } => zustand
            .gespraeche
            .direkt_besteht(paar)
            .then(|| Gespraech::Direkt(paar.clone())),
    }
}

/// 404 für ein Gespräch, das es für den Aufrufer nicht gibt: „# name gibt es nicht.“ im Channel, im Direktgespräch
/// dieselbe Antwort wie für eine nie vergebene Zeit.
fn unbekannt_beantworten(ziel: &Ziel, wer: &str) -> Response {
    mit_regel(unbekannt_wie(ziel), unbekannt_regel(ziel), Some(wer))
}

/// Warum ein Körper keinen Zustand ergibt.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Wertfehler {
    /// Ein anderes Feld als `zustand` und `gueltig_s` — auch `kennung`.
    Feld(String),
    /// `zustand` fehlt, ist kein Text oder nicht der feste Wortlaut.
    Zustand,
    /// `gueltig_s` 0 oder über 120.
    Gueltigkeit,
    /// Ein 17. Gespräch derselben Kennung.
    ZuViele,
}

impl Wertfehler {
    fn regel(&self) -> &'static str {
        match self {
            Wertfehler::Feld(_) => "feld",
            Wertfehler::Zustand => "zustand",
            Wertfehler::Gueltigkeit => "gueltigkeit",
            Wertfehler::ZuViele => "aktivitaeten",
        }
    }

    fn beantworten(&self) -> Response {
        let fehler = match self {
            Wertfehler::Feld(feld) => {
                format!(
                    "Unbekanntes Feld {feld} — erlaubt sind zustand und gueltig_s, zum Beispiel {BEISPIEL}."
                )
            }
            Wertfehler::Zustand => {
                format!("zustand ist \"tippt\" oder \"denkt_nach\", zum Beispiel {BEISPIEL}.")
            }
            Wertfehler::Gueltigkeit => "gueltig_s ist eine ganze Zahl von 1 bis 120.".to_string(),
            Wertfehler::ZuViele => format!(
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

/// Eine 422 mit Regelschlüssel; die Middleware schreibt sie als `DEBUG` (ADR-0051 Regel 11).
fn abgewiesen(wertfehler: &Wertfehler, wer: &str) -> Response {
    mit_regel(wertfehler.beantworten(), wertfehler.regel(), Some(wer))
}

/// Jede 429 der Bremse zählt in die Episode `WARNING aktivitaet` je Kennung (ADR-0042 Ergänzung „Karte #77“); die
/// Antwort hat die Form der Bremse des Schreibkontos (ADR-0036 Regel 4) mit eigenem Satz (ADR-0051 Regel 9).
fn gebremst_melden(
    protokoll: &Protokoll,
    gebremst: Gebremst,
    teilnehmer: &Teilnehmer,
    ziel: &Ziel,
) -> Response {
    let wer = teilnehmer.kennung.als_text();
    let name = ziel.name();
    protokoll.episode(gebremst_auftreten(wer, &name, gebremst.grenze));
    let wo = match ziel {
        Ziel::Channel(name) => format!("# {name}"),
        Ziel::Direkt { .. } => "dem Direktgespräch".to_string(),
    };
    let wieder_in_ms = u64::try_from(gebremst.wieder_in.as_millis()).unwrap_or(u64::MAX);
    let sekunden = wieder_in_ms.div_ceil(1_000).max(1);
    let von = match gebremst.grenze {
        Grenze::Gespraech => wer.to_string(),
        Grenze::Herkunft => format!(
            "{} (alle Teilnehmer dieser Adresse zusammen)",
            teilnehmer.herkunft
        ),
    };
    let fehler =
        format!("Zu viele Aktivitätsmeldungen von {von} in {wo} — wieder möglich in {sekunden} s.");
    (
        StatusCode::TOO_MANY_REQUESTS,
        [(RETRY_AFTER, sekunden.to_string())],
        Json(json!({
            "fehler": fehler,
            "grenze": gebremst.grenze.als_text(),
            "wieder_in_ms": wieder_in_ms
        })),
    )
        .into_response()
}

/// Das Auftreten einer 429 der Aktivität: Quelle ist die Kennung; Channel und Grenze stehen nur in der Beginn-Zeile,
/// die Grenze zählt als Art für die Ende-Zeile `(gespraech <g>; herkunft <h>)`.
pub(crate) fn gebremst_auftreten(wer: &str, channel: &str, grenze: Grenze) -> Auftreten {
    Auftreten::neu(
        Stufe::Warning,
        Bereich::Aktivitaet,
        episodentexte::AKTIVITAET_GEBREMST,
        Quelle::neu().mit(Schluessel::Kennung, wer),
    )
    .einzeln(bezug(channel), channel)
    .einzeln(Schluessel::Grenze, grenze.als_text())
    .art(grenze.als_text())
}

/// Was der Körper verlangt: der Zustand und, wenn angegeben, die Gültigkeit in Sekunden.
struct Wunsch {
    zustand: aktivitaet::Zustand,
    gueltig_s: Option<u32>,
}

/// Genau `zustand` mit festem Wortlaut und höchstens `gueltig_s` als ganze Zahl; ob sie in 1 bis 120 liegt, prüft die
/// Aktivität (abweisen statt begrenzen, F2). Jede andere Form der Gültigkeit — Bruch, Text, negativ, `null` — ist
/// `gueltigkeit`.
fn wunsch_lesen(koerper: &Value) -> Result<Wunsch, Wertfehler> {
    let objekt = koerper.as_object().ok_or(Wertfehler::Zustand)?;
    if let Some(feld) = objekt
        .keys()
        .find(|feld| *feld != "zustand" && *feld != "gueltig_s")
    {
        return Err(Wertfehler::Feld(feld.clone()));
    }
    let zustand = objekt
        .get("zustand")
        .and_then(Value::as_str)
        .and_then(aktivitaet::Zustand::lesen)
        .ok_or(Wertfehler::Zustand)?;
    let gueltig_s = match objekt.get("gueltig_s") {
        None => None,
        Some(wert) => Some(
            wert.as_u64()
                .map(|zahl| u32::try_from(zahl).unwrap_or(u32::MAX))
                .ok_or(Wertfehler::Gueltigkeit)?,
        ),
    };
    Ok(Wunsch { zustand, gueltig_s })
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
