//! Eigene Nachricht löschen (R00040 F0136; ADR-0048 Regeln 5, 9, 10, 12, 13): `POST
//! /api/channels/{name}/nachrichten/loeschen {zeit}` mit Sitzung, Mensch und Agent gleich, in einem Schritt → 200
//! `{channel, nachricht, ergebnis, anhaenge}`. Prüfreihenfolge (abgewiesen, nie umgeformt): Grenzschutz 403 ·
//! Sitzung 401 · Vorab-Buchung 1 Einheit 429 · Körper über 1.024 Bytes 413 · kein JSON-Typ 415 · kein JSON 400 ·
//! doppeltes oder weiteres Feld (auch `bestaetigt`) 422 `feld` · Channel 404 · 422 `zeit` · unter der Sperre 404
//! `nachricht` · 403 `fremd` · schon gelöscht 200 · 507 `quote` (mit Reserve) · 500. Kein Wiederholschlüssel, keine
//! Restbuchung — Löschen ist von sich aus wiederholbar.

use super::Zustand;
use super::anfrageprotokoll::mit_regel;
use super::bearbeiten::{
    fremd_abgewiesen, fremd_auftreten, nachricht_fehlt, quote_erreicht, schreibfehler_auftreten,
    zeit_abgewiesen, zeit_lesen,
};
use super::bremse::{Gegenstand, gebremst_melden};
use super::koerper::{Koerperfehler, Koerperort, json_lesen, koerperfehler_beantworten};
use super::nachrichten::NachrichtAntwort;
use super::senden::{nicht_gespeichert_antwort, quote_auftreten, unbekannt};
use super::teilnehmer::Teilnehmer;
use crate::gespraeche::nachricht_loeschen::{Loeschfehler, NachrichtGeloescht};
use crate::gespraeche::{Gespraech, Nachricht};
use crate::protokoll::vorlauf;
use crate::protokoll::{Bereich, Kontext, Protokoll, Schluessel, Stufe};
use axum::Json;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Instant;

const ROUTE: &str = "POST /api/channels/{name}/nachrichten/loeschen";

/// Körpergrenze des Löschens (ADR-0048 Regel 5): `{"zeit":"…"}` in der längsten Schreibweise ≈ 190 Bytes.
pub const LOESCHEN_GRENZE: usize = 1_024;

/// Kosten am Schreibkonto: eine Einheit, nur vorab (ADR-0048 Regel 5).
const VORAB_LOESCHEN: u32 = 1;

pub async fn loeschen(
    teilnehmer: Teilnehmer,
    State(zustand): State<Arc<Zustand>>,
    Path(name): Path<String>,
    kopf: HeaderMap,
    koerper: Body,
) -> Response {
    let eintritt = Instant::now();
    if let Err(gebremst) =
        zustand
            .schreibkonto
            .vorab(&teilnehmer.kennung, teilnehmer.herkunft, VORAB_LOESCHEN)
    {
        return gebremst_melden(
            &zustand.protokoll,
            gebremst,
            Gegenstand::Nachrichten,
            &teilnehmer,
        );
    }
    let wer = teilnehmer.kennung.als_text().to_string();
    let ort = Koerperort {
        protokoll: &zustand.protokoll,
        route: ROUTE,
        herkunft: teilnehmer.herkunft,
        kennung: Some(&wer),
    };
    let wert = match json_lesen(&kopf, koerper, LOESCHEN_GRENZE, &ort).await {
        Ok(wert) => wert,
        Err(fehler) => return koerper_abgewiesen(fehler),
    };
    if let Some(feld) = weiteres_feld(&wert) {
        return mit_regel(feld_abgewiesen(&feld), "feld_unbekannt", Some(&wer));
    }
    if !zustand.gespraeche.channel_bekannt(&name) {
        return mit_regel(unbekannt(&name), "channel_unbekannt", Some(&wer));
    }
    let Some(ziel) = zeit_lesen(&wert) else {
        return mit_regel(zeit_abgewiesen(), "zeit", Some(&wer));
    };
    let gespraeche = Arc::clone(&zustand.gespraeche);
    let gespraech = Gespraech::channel(&name);
    let von = teilnehmer.kennung.clone();
    let ergebnis =
        tokio::task::spawn_blocking(move || gespraeche.nachricht_loeschen(&gespraech, &von, ziel))
            .await;
    let melden = |grund| geloescht_melden(&zustand.protokoll, &name, &wer, 0, eintritt, grund);
    match ergebnis {
        Ok(Ok(NachrichtGeloescht::Neu(nachricht))) => {
            vorlauf::eintragen(&zustand.protokoll, zustand.gespraeche.zeit_vorlauf_ms());
            melden(None);
            geloescht(&name, &nachricht, "geloescht")
        }
        Ok(Ok(NachrichtGeloescht::Schon(nachricht))) => {
            melden(Some("schon_geloescht"));
            geloescht(&name, &nachricht, "schon_geloescht")
        }
        Ok(Err(fehler)) => {
            abweisung_melden(&zustand.protokoll, &fehler, &name, &wer);
            abgewiesen(fehler, &name, &wer)
        }
        Err(_) => super::abbruch::interner_fehler("Löschen"),
    }
}

/// `DEBUG aendern Nachricht gelöscht` (ADR-0048 Regel 13): Channel, Kennung, Zahl der mitgelöschten Anhänge, die Zeit
/// vom Eintritt in den Handler bis zum Ende; ohne Satz `grund=schon_geloescht` — nie Text.
fn geloescht_melden(
    protokoll: &Protokoll,
    channel: &str,
    wer: &str,
    anhaenge: u64,
    eintritt: Instant,
    grund: Option<&str>,
) {
    let kontext = Kontext::neu()
        .mit(Schluessel::Channel, channel)
        .mit(Schluessel::Kennung, wer)
        .mit(Schluessel::Anhaenge, anhaenge)
        .mit(Schluessel::DauerMs, eintritt.elapsed().as_millis());
    let kontext = match grund {
        Some(grund) => kontext.mit(Schluessel::Grund, grund),
        None => kontext,
    };
    protokoll.eintragen(
        Stufe::Debug,
        Bereich::Aendern,
        "Nachricht gelöscht",
        kontext,
    );
}

/// Abweisungen unter der Sperre mit eigener Episode wie beim Bearbeiten: 403 `fremd` je Kennung, 500 je Datei und
/// Grund, 507 in der Quote-Episode der Kennung mit `route=loeschen`.
fn abweisung_melden(protokoll: &Protokoll, fehler: &Loeschfehler, channel: &str, wer: &str) {
    match fehler {
        Loeschfehler::Fremd { .. } => protokoll.episode(fremd_auftreten(channel, wer)),
        Loeschfehler::Ablage { datei, grund } => {
            protokoll.episode(schreibfehler_auftreten(channel, wer, datei, grund));
        }
        Loeschfehler::Quote { belegt, hoechstens } => {
            let mut auftreten = quote_auftreten(channel, wer, *belegt, *hoechstens);
            auftreten.kontext = auftreten
                .kontext
                .mit_nur_beginn(Schluessel::Route, "loeschen");
            protokoll.episode(auftreten);
        }
        Loeschfehler::Unbekannt | Loeschfehler::Nachricht => {}
    }
}

/// Die Antworten der Prüfungen unter der Sperre.
fn abgewiesen(fehler: Loeschfehler, channel: &str, wer: &str) -> Response {
    match fehler {
        Loeschfehler::Unbekannt => mit_regel(unbekannt(channel), "channel_unbekannt", Some(wer)),
        Loeschfehler::Nachricht => mit_regel(nachricht_fehlt(), "nachricht", Some(wer)),
        Loeschfehler::Fremd { absender } => {
            mit_regel(fremd_abgewiesen(&absender), "fremd", Some(wer))
        }
        Loeschfehler::Quote { belegt, hoechstens } => mit_regel(
            quote_erreicht("Nicht gelöscht", belegt, hoechstens),
            "quote",
            Some(wer),
        ),
        Loeschfehler::Ablage { .. } => nicht_gespeichert_antwort(),
    }
}

fn geloescht(channel: &str, nachricht: &Nachricht, ergebnis: &str) -> Response {
    let antwort = json!({
        "channel": channel,
        "nachricht": NachrichtAntwort::aus(nachricht),
        "ergebnis": ergebnis,
        "anhaenge": {"geloescht": 0, "bytes": 0, "fehler": 0},
    });
    (StatusCode::OK, Json(antwort)).into_response()
}

/// Das erste Feld neben `zeit` (ADR-0048 Regel 5, Regel 12: auch `bestaetigt`); ein Nicht-Objekt hat keines.
fn weiteres_feld(wert: &Value) -> Option<String> {
    wert.as_object()?
        .keys()
        .find(|feld| *feld != "zeit")
        .cloned()
}

fn feld_abgewiesen(feld: &str) -> Response {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(json!({
            "fehler": format!("Unbekanntes Feld {feld} — erlaubt ist nur zeit."),
            "regel": "feld",
            "feld": feld,
        })),
    )
        .into_response()
}

const BEISPIEL: &str = r#"{"zeit":"2026-10-30T14:07:12.345+01:00"}"#;

fn koerper_abgewiesen(fehler: Koerperfehler) -> Response {
    koerperfehler_beantworten(
        fehler,
        "Der Körper ist zu groß — höchstens 1.024 Byte; er braucht nur zeit.",
        BEISPIEL,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ein weiteres Feld wird genannt, auch `bestaetigt` (ADR-0048 Regel 12); nur `zeit` ist erlaubt.
    #[test]
    fn nur_zeit_ist_ein_erlaubtes_feld() {
        assert_eq!(
            weiteres_feld(&json!({"zeit": "x", "bestaetigt": true})),
            Some("bestaetigt".to_string())
        );
        assert_eq!(weiteres_feld(&json!({"zeit": "x"})), None);
        assert_eq!(weiteres_feld(&json!([1])), None);
    }
}
