//! Route Löschen beantworten (B0508, ADR-0046 Ergänzung „Karte #9 Betreiber F1′“ Regeln 4, 5, 9, 12): `POST
//! /api/anhaenge/loeschen {ids, bestaetigt?}` mit Sitzung, Mensch und Agent gleich. Prüfreihenfolge: 403 · 401 ·
//! 429 (1 Einheit vorab) · 415/400/413/408 · 422 (B0505) · 409 `bestaetigen` · 429 (Rest-Buchung) · 200.

use super::super::Zustand;
use super::super::anfrageprotokoll::mit_regel;
use super::super::bremse::{Gegenstand, gebremst_melden};
use super::super::koerper::{Koerperfehler, Koerperort, felder_lesen, koerperfehler_beantworten};
use super::super::senden::groesse_in_worten;
use super::super::teilnehmer::Teilnehmer;
use super::loeschkoerper::auftrag_pruefen;
use crate::gespraeche::loeschen::{Geloescht, Loeschen, Umfang};
use axum::Json;
use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};
use std::sync::Arc;

const ROUTE: &str = "POST /api/anhaenge/loeschen";

/// Körpergrenze: 500 IDs × 29 Zeichen (`"<26>",`) + Hülle `{"ids":[],"bestaetigt":false}` ≈ 14.530 → 16.384.
pub const LOESCHEN_GRENZE: usize = 16_384;

/// Vorab-Buchung am Schreibkonto (ADR-0036 Ergänzung Karte #9, ADR-0046 Ergänzung „F1′“ Regel 5).
const VORAB_LOESCHEN: u32 = 1;

pub async fn loeschen(
    teilnehmer: Teilnehmer,
    State(zustand): State<Arc<Zustand>>,
    kopf: HeaderMap,
    koerper: Body,
) -> Response {
    if let Err(gebremst) =
        zustand
            .schreibkonto
            .vorab(&teilnehmer.kennung, teilnehmer.herkunft, VORAB_LOESCHEN)
    {
        return gebremst_melden(
            &zustand.protokoll,
            gebremst,
            Gegenstand::Loeschungen,
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
    let felder = match felder_lesen(&kopf, koerper, LOESCHEN_GRENZE, &ort).await {
        Ok(felder) => felder,
        Err(fehler) => return koerper_abgewiesen(fehler),
    };
    let auftrag = match auftrag_pruefen(felder.as_ref()) {
        Ok(auftrag) => auftrag,
        Err(abweisung) => {
            let antwort =
                (StatusCode::UNPROCESSABLE_ENTITY, Json(abweisung.koerper)).into_response();
            return mit_regel(antwort, abweisung.regel, Some(&wer));
        }
    };
    let Ok(erlaubnis) = Arc::clone(&zustand.hochladebank).acquire_owned().await else {
        return super::super::abbruch::interner_fehler("Löschen");
    };
    let gespraeche = Arc::clone(&zustand.gespraeche);
    let konto = Arc::clone(&zustand.schreibkonto);
    let kennung = teilnehmer.kennung.clone();
    let herkunft = teilnehmer.herkunft;
    let geloescht = tokio::task::spawn_blocking(move || {
        let _erlaubnis = erlaubnis;
        let mut gebremst = None;
        let loeschen =
            gespraeche.anhaenge_loeschen(&kennung, &auftrag.ids, auftrag.bestaetigt, |einheiten| {
                match konto.rest(&kennung, herkunft, VORAB_LOESCHEN + einheiten) {
                    Ok(()) => true,
                    Err(bremse) => {
                        gebremst = Some(bremse);
                        false
                    }
                }
            });
        (loeschen, gebremst)
    })
    .await;
    match geloescht {
        Ok((Loeschen::Bestaetigen(umfang), _)) => {
            let antwort = (StatusCode::CONFLICT, Json(rueckfrage(umfang))).into_response();
            mit_regel(antwort, "bestaetigen", Some(&wer))
        }
        Ok((Loeschen::Gebremst, Some(gebremst))) => gebremst_melden(
            &zustand.protokoll,
            gebremst,
            Gegenstand::Loeschungen,
            &teilnehmer,
        ),
        Ok((Loeschen::Geloescht(bericht), _)) => {
            (StatusCode::OK, Json(ergebnis(&bericht))).into_response()
        }
        Ok((Loeschen::Gebremst, None)) | Err(_) => {
            super::super::abbruch::interner_fehler("Löschen")
        }
    }
}

/// 409 mit dem Umfang und der Frage (K-a).
pub fn rueckfrage(umfang: Umfang) -> Value {
    json!({
        "fehler": format!(
            "Wirklich {} Anhänge ({}) aus {} Gesprächen löschen, davon {} von anderen? Die Nachrichten bleiben.",
            umfang.anhaenge,
            groesse_in_worten(umfang.bytes),
            umfang.gespraeche,
            umfang.fremde
        ),
        "regel": "bestaetigen",
        "loeschen": {
            "anhaenge": umfang.anhaenge,
            "bytes": umfang.bytes,
            "gespraeche": umfang.gespraeche,
            "fremde": umfang.fremde,
        },
    })
}

/// 200 mit dem Ergebnis je ID und dem Platz.
pub fn ergebnis(bericht: &Geloescht) -> Value {
    json!({
        "geloescht": bericht.umfang.anhaenge,
        "bytes": bericht.umfang.bytes,
        "ergebnisse": bericht
            .ergebnisse
            .iter()
            .map(|(id, ergebnis)| json!({"id": id.als_text(), "ergebnis": ergebnis.als_text()}))
            .collect::<Vec<_>>(),
        "platz": {
            "hoechstens": bericht.platz.hoechstens,
            "belegt": bericht.platz.belegt,
            "frei": bericht.platz.frei,
        },
    })
}

fn koerper_abgewiesen(fehler: Koerperfehler) -> Response {
    koerperfehler_beantworten(
        fehler,
        "Der Körper ist zu groß — höchstens 16.384 Byte; höchstens 500 IDs je Anfrage.",
        r#"{"ids":["<id>"],"bestaetigt":true}"#,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn der_text_der_rueckfrage() {
        let koerper = rueckfrage(Umfang {
            anhaenge: 2,
            bytes: 2_097_152,
            gespraeche: 2,
            fremde: 2,
        });
        assert_eq!(
            koerper,
            json!({
                "fehler": "Wirklich 2 Anhänge (2,0 MiB) aus 2 Gesprächen löschen, davon 2 von anderen? Die Nachrichten bleiben.",
                "regel": "bestaetigen",
                "loeschen": {"anhaenge": 2, "bytes": 2_097_152, "gespraeche": 2, "fremde": 2},
            })
        );
    }
}
