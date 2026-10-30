//! Route Löschen beantworten (B0508, ADR-0046 Ergänzung „Karte #9 Betreiber F1′“ Regeln 4, 5, 9, 12): `POST
//! /api/anhaenge/loeschen {ids, bestaetigt?}` mit Sitzung, Mensch und Agent gleich. Prüfreihenfolge: 403 · 401 ·
//! 429 (1 Einheit vorab) · 415/400/413/408 · 422 (B0505) · 409 `bestaetigen` · 429 (Rest-Buchung) · 200.

use super::super::Zustand;
use super::super::anfrageprotokoll::{mit_regel, mit_regel_und};
use super::super::bremse::{Gegenstand, gebremst_melden};
use super::super::koerper::{Koerperfehler, Koerperort, felder_lesen, koerperfehler_beantworten};
use super::super::senden::groesse_in_worten;
use super::super::teilnehmer::Teilnehmer;
use super::loeschkoerper::{self, auftrag_pruefen};
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
    let auftrag = Loeschauftrag {
        gespraeche: Arc::clone(&zustand.gespraeche),
        konto: Arc::clone(&zustand.schreibkonto),
        protokoll: Arc::clone(&zustand.protokoll),
        bus: Arc::clone(&zustand.bus),
        teilnehmer,
        auftrag,
    };
    let fertig = tokio::task::spawn_blocking(move || {
        let _erlaubnis = erlaubnis;
        auftrag.ausfuehren()
    })
    .await;
    match fertig {
        Ok(antwort) => antwort,
        Err(_) => super::super::abbruch::interner_fehler("Löschen"),
    }
}

/// Alles nach dem Start des Löschens in einem Blocking-Teil (ADR-0048 K2, gleiche Klasse wie beim Löschen einer
/// Nachricht): Wirkung, Protokollzeilen, Ereignis und Bau der Antwort, ohne `await` dazwischen — bricht der Client ab,
/// läuft der Teil zu Ende.
struct Loeschauftrag {
    gespraeche: Arc<crate::gespraeche::Gespraeche>,
    konto: Arc<crate::schreibkonto::Schreibkonto>,
    protokoll: Arc<Protokoll>,
    bus: Arc<Bus>,
    teilnehmer: Teilnehmer,
    auftrag: loeschkoerper::Auftrag,
}

impl Loeschauftrag {
    fn ausfuehren(&self) -> Response {
        let kennung = &self.teilnehmer.kennung;
        let herkunft = self.teilnehmer.herkunft;
        let mut gebremst = None;
        let loeschen = self.gespraeche.anhaenge_loeschen(
            kennung,
            &self.auftrag.ids,
            self.auftrag.bestaetigt,
            |einheiten| match self
                .konto
                .rest(kennung, herkunft, VORAB_LOESCHEN + einheiten)
            {
                Ok(()) => true,
                Err(bremse) => {
                    gebremst = Some(bremse);
                    false
                }
            },
        );
        match (loeschen, gebremst) {
            (Loeschen::Bestaetigen(umfang), _) => {
                let antwort = (StatusCode::CONFLICT, Json(rueckfrage(umfang))).into_response();
                mit_regel_und(
                    antwort,
                    "bestaetigen",
                    Some(kennung.als_text()),
                    vec![(Schluessel::Anhaenge, umfang.anhaenge.to_string())],
                )
            }
            (Loeschen::Gebremst, Some(gebremst)) => gebremst_melden(
                &self.protokoll,
                gebremst,
                Gegenstand::Loeschungen,
                &self.teilnehmer,
            ),
            (Loeschen::Geloescht(bericht), _) => {
                protokollieren(
                    &self.protokoll,
                    &self.teilnehmer,
                    &bericht,
                    Anlass::Laufwerk,
                );
                veroeffentlichen(&self.bus, &bericht);
                (StatusCode::OK, Json(ergebnis(&bericht))).into_response()
            }
            (Loeschen::Gebremst, None) => super::super::abbruch::interner_fehler("Löschen"),
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
    use crate::http::loeschaufbau::{aufbau, naechstes_anhaenge_ereignis};

    /// R00043 B0666 (ADR-0048 K2, gleiche Klasse beim Löschen im Laufwerk): bricht der Client nach dem Start des
    /// Löschens ab, stehen trotzdem die Zeile `INFORMATION anhang Anhänge gelöscht` (ohne `grund`) und das Ereignis
    /// `anhaenge` (Mutante „Protokoll nach dem await“ rot).
    #[tokio::test(flavor = "multi_thread")]
    async fn der_abbruch_des_clients_verliert_beim_loeschen_im_laufwerk_weder_protokoll_noch_ereignis()
     {
        let aufbau = aufbau();
        let (zeit, id) = aufbau.nachricht_mit_anhang("beta@127.0.0.1");
        let mut abonnement = aufbau.zustand.bus.abonnieren();

        aufbau
            .abbrechen_am_kuerzen(aufbau.anhaenge_loeschen("beta@127.0.0.1", &[&id]))
            .await;
        let ereignis = naechstes_anhaenge_ereignis(&mut abonnement).await;

        assert_eq!(ereignis, Some(vec![(id.als_text().to_string(), zeit)]));
        let information = aufbau.zeilen_mit(" INFORMATION anhang Anhänge gelöscht");
        assert_eq!(information.len(), 1, "{:#?}", aufbau.zeilen.zeilen());
        assert!(!information[0].contains("grund="), "{information:#?}");
    }

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
