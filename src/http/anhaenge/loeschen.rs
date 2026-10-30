//! Route Löschen beantworten (B0508, ADR-0046 Ergänzung „Karte #9 Betreiber F1′“ Regeln 4, 5, 9, 12): `POST
//! /api/anhaenge/loeschen {ids, bestaetigt?}` mit Sitzung, Mensch und Agent gleich. Prüfreihenfolge: 403 · 401 ·
//! 429 (1 Einheit vorab) · 415/400/413/408 · 422 (B0505) · 409 `bestaetigen` · 429 (Rest-Buchung) · 200.

use super::super::Zustand;
use super::super::anfrageprotokoll::{mit_regel, mit_regel_und};
use super::super::bremse::{Gegenstand, gebremst_melden};
use super::super::koerper::{Koerperfehler, Koerperort, felder_lesen, koerperfehler_beantworten};
use super::super::senden::groesse_in_worten;
use super::super::teilnehmer::Teilnehmer;
use super::loeschkoerper::auftrag_pruefen;
use crate::gespraeche::Gespraech;
use crate::gespraeche::loeschen::{Einzelergebnis, Geloescht, Loeschen, Umfang};
use crate::live::bus::{Bus, Ereignis};
use crate::protokoll::{Bereich, Kontext, Protokoll, Schluessel, Stufe};
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
            mit_regel_und(
                antwort,
                "bestaetigen",
                Some(&wer),
                vec![(Schluessel::Anhaenge, umfang.anhaenge.to_string())],
            )
        }
        Ok((Loeschen::Gebremst, Some(gebremst))) => gebremst_melden(
            &zustand.protokoll,
            gebremst,
            Gegenstand::Loeschungen,
            &teilnehmer,
        ),
        Ok((Loeschen::Geloescht(bericht), _)) => {
            protokollieren(&zustand.protokoll, &teilnehmer, &bericht, Anlass::Laufwerk);
            veroeffentlichen(&zustand.bus, &bericht);
            (StatusCode::OK, Json(ergebnis(&bericht))).into_response()
        }
        Ok((Loeschen::Gebremst, None)) | Err(_) => {
            super::super::abbruch::interner_fehler("Löschen")
        }
    }
}

/// Wer Anhänge löscht: das Laufwerk (die Route hier) oder das Löschen ihrer Nachricht (ADR-0048 Regel 9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::http) enum Anlass {
    Laufwerk,
    Nachricht,
}

/// Betrieb (B0511, ADR-0042 Ergänzung „F1′“ und „Karte #9 Security-Gate“ P-a): mit Wirkung genau eine
/// `INFORMATION anhang Anhänge gelöscht` (Textanfang für das Eingriffsbudget, ADR-0049), ohne Wirkung im Laufwerk nur
/// `DEBUG`, beim Löschen einer Nachricht nichts; je Datei `DEBUG` bzw. `ERROR` — nie Anhang-ID, Anzeigename oder
/// ASCII-Teil. Beim Löschen einer Nachricht trägt die Zeile `grund=nachricht` (ADR-0048 Regel 13), und ein gescheitertes
/// Kürzen lässt den Anhang gelöscht.
pub(in crate::http) fn protokollieren(
    protokoll: &Protokoll,
    teilnehmer: &Teilnehmer,
    bericht: &Geloescht,
    anlass: Anlass,
) {
    let wer = teilnehmer.kennung.als_text();
    for datei in &bericht.dateien {
        let ort = Kontext::neu()
            .mit(Schluessel::Ordner, datei.gespraech.pfad())
            .mit(Schluessel::Tag, datei.tag.format("%Y-%m-%d"))
            .mit(Schluessel::Anhang, datei.datei.as_str())
            .mit(Schluessel::Bytes, datei.bytes);
        match &datei.grund {
            None => protokoll.eintragen(Stufe::Debug, Bereich::Anhang, "Anhang gelöscht", ort),
            Some(grund) => protokoll.eintragen(
                Stufe::Error,
                Bereich::Anhang,
                match anlass {
                    Anlass::Laufwerk => "Löschen gescheitert — der Anhang bleibt",
                    Anlass::Nachricht => {
                        "Löschen gescheitert — der Anhang bleibt gelöscht, der nächste Start kürzt die Datei"
                    }
                },
                ort.mit(Schluessel::Grund, grund.as_str()),
            ),
        }
    }
    if bericht.umfang.anhaenge + bericht.fehler > 0 {
        let kontext = Kontext::neu()
            .mit(Schluessel::Kennung, wer)
            .mit(Schluessel::Herkunft, teilnehmer.herkunft)
            .mit(Schluessel::Anhaenge, bericht.umfang.anhaenge)
            .mit(Schluessel::Bytes, bericht.umfang.bytes)
            .mit(Schluessel::Gespraeche, bericht.umfang.gespraeche)
            .mit(Schluessel::Fremde, bericht.umfang.fremde)
            .mit(Schluessel::Fehler, bericht.fehler);
        let kontext = match anlass {
            Anlass::Laufwerk => kontext,
            Anlass::Nachricht => kontext.mit(Schluessel::Grund, "nachricht"),
        };
        protokoll.eintragen(
            Stufe::Information,
            Bereich::Anhang,
            "Anhänge gelöscht",
            kontext,
        );
    } else if anlass == Anlass::Laufwerk {
        let zahl = |gesucht: Einzelergebnis| {
            bericht
                .ergebnisse
                .iter()
                .filter(|(_, ergebnis)| *ergebnis == gesucht)
                .count()
        };
        protokoll.eintragen(
            Stufe::Debug,
            Bereich::Anhang,
            "Löschen ohne Wirkung",
            Kontext::neu()
                .mit(Schluessel::Kennung, wer)
                .mit(Schluessel::Herkunft, teilnehmer.herkunft)
                .mit(Schluessel::Ids, bericht.ergebnisse.len())
                .mit(Schluessel::Unbekannt, zahl(Einzelergebnis::Unbekannt))
                .mit(
                    Schluessel::SchonGeloescht,
                    zahl(Einzelergebnis::SchonGeloescht),
                ),
        );
    }
}

/// Je betroffenem Channel ein Ereignis `anhaenge` mit (ID, Satzzeit) — nach Schritt (c), nie für die Vorschau oder
/// `schon_geloescht` (B0514, ADR-0047 Ergänzung „F1′“ Regel 3). Direktgespräche erst mit I0012.
pub(in crate::http) fn veroeffentlichen(bus: &Bus, bericht: &Geloescht) {
    for (gespraech, geloescht) in &bericht.je_gespraech {
        let Gespraech::Channel(channel) = gespraech else {
            continue;
        };
        bus.veroeffentlichen(Ereignis::Anhaenge {
            channel: channel.clone(),
            geloescht: geloescht
                .iter()
                .map(|(id, zeit)| (id.als_text().to_string(), *zeit))
                .collect(),
        });
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
