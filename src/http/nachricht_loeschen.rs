//! Eigene Nachricht löschen (R00040 F0136; ADR-0048 Regeln 5, 9, 10, 12, 13): `POST
//! /api/channels/{name}/nachrichten/loeschen {zeit}` mit Sitzung, Mensch und Agent gleich, in einem Schritt → 200
//! `{channel, nachricht, ergebnis, anhaenge}`. Prüfreihenfolge (abgewiesen, nie umgeformt): Grenzschutz 403 ·
//! Sitzung 401 · Vorab-Buchung 1 Einheit 429 · Körper über 1.024 Bytes 413 · kein JSON-Typ 415 · kein JSON 400 ·
//! doppeltes oder weiteres Feld (auch `bestaetigt`) 422 `feld` · Channel 404 · 422 `zeit` · unter der Sperre 404
//! `nachricht` · 403 `fremd` · schon gelöscht 200 · 507 `quote` (mit Reserve) · 500. Kein Wiederholschlüssel, keine
//! Restbuchung — Löschen ist von sich aus wiederholbar.

use super::Zustand;
use super::anfrageprotokoll::mit_regel;
use super::anhaenge::loeschen::{Anlass, protokollieren, veroeffentlichen};
use super::bearbeiten::{
    fremd_abgewiesen, fremd_auftreten, nachricht_fehlt, quote_erreicht, schreibfehler_auftreten,
    zeit_abgewiesen, zeit_lesen,
};
use super::bremse::{Gegenstand, gebremst_melden};
use super::koerper::{Koerperfehler, Koerperort, json_lesen, koerperfehler_beantworten};
use super::nachrichten::NachrichtAntwort;
use super::senden::{nicht_gespeichert_antwort, quote_auftreten, unbekannt};
use super::teilnehmer::Teilnehmer;
use crate::gespraeche::loeschen::Geloescht;
use crate::gespraeche::nachricht_loeschen::{Loeschfehler, NachrichtGeloescht};
use crate::gespraeche::{Gespraech, Gespraeche, Nachricht};
use crate::live::bus::Bus;
use crate::protokoll::vorlauf;
use crate::protokoll::{Bereich, Kontext, Protokoll, Schluessel, Stufe};
use crate::uhr::Zeit;
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
    let Ok(erlaubnis) = Arc::clone(&zustand.hochladebank).acquire_owned().await else {
        return super::abbruch::interner_fehler("Löschen");
    };
    let auftrag = Loeschauftrag {
        gespraeche: Arc::clone(&zustand.gespraeche),
        protokoll: Arc::clone(&zustand.protokoll),
        bus: Arc::clone(&zustand.bus),
        teilnehmer,
        channel: name,
        ziel,
        eintritt,
    };
    let fertig = tokio::task::spawn_blocking(move || {
        let _erlaubnis = erlaubnis;
        auftrag.ausfuehren()
    })
    .await;
    match fertig {
        Ok(antwort) => antwort,
        Err(_) => super::abbruch::interner_fehler("Löschen"),
    }
}

/// Alles, was nach dem Commit-Punkt geschieht, in einem Blocking-Teil (ADR-0048 K2): die Wirkung, die Protokollzeilen,
/// die Episoden, das Ereignis und der Bau der Antwort. Fällt die Zukunft des Handlers, weil der Client abbricht,
/// läuft dieser Teil zu Ende — ein `await` dazwischen verlöre Spur und Ereignis.
struct Loeschauftrag {
    gespraeche: Arc<Gespraeche>,
    protokoll: Arc<Protokoll>,
    bus: Arc<Bus>,
    teilnehmer: Teilnehmer,
    channel: String,
    ziel: Zeit,
    eintritt: Instant,
}

impl Loeschauftrag {
    fn ausfuehren(&self) -> Response {
        let wer = self.teilnehmer.kennung.als_text();
        let ergebnis = self.gespraeche.nachricht_loeschen(
            &Gespraech::channel(&self.channel),
            &self.teilnehmer.kennung,
            self.ziel,
        );
        match ergebnis {
            Ok(NachrichtGeloescht::Neu {
                nachricht,
                anhaenge,
            }) => {
                vorlauf::eintragen(&self.protokoll, self.gespraeche.zeit_vorlauf_ms());
                self.melden(anhaenge.umfang.anhaenge, None);
                protokollieren(
                    &self.protokoll,
                    &self.teilnehmer,
                    &anhaenge,
                    Anlass::Nachricht,
                );
                veroeffentlichen(&self.bus, &anhaenge);
                geloescht(&self.channel, &nachricht, "geloescht", Some(&anhaenge))
            }
            Ok(NachrichtGeloescht::Schon(nachricht)) => {
                self.melden(0, Some("schon_geloescht"));
                geloescht(&self.channel, &nachricht, "schon_geloescht", None)
            }
            Err(fehler) => {
                abweisung_melden(&self.protokoll, &fehler, &self.channel, wer);
                abgewiesen(fehler, &self.channel, wer)
            }
        }
    }

    fn melden(&self, anhaenge: u64, grund: Option<&str>) {
        geloescht_melden(
            &self.protokoll,
            &self.channel,
            self.teilnehmer.kennung.als_text(),
            anhaenge,
            self.eintritt,
            grund,
        );
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
            protokoll.episode(
                quote_auftreten(channel, wer, *belegt, *hoechstens)
                    .einzeln(Schluessel::Route, "loeschen"),
            );
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

/// 200 mit der gelöschten Fassung und der Bilanz der mitgelöschten Anhänge `{geloescht, bytes, fehler}` — schon
/// gelöscht: nichts mitgelöscht.
fn geloescht(
    channel: &str,
    nachricht: &Nachricht,
    ergebnis: &str,
    anhaenge: Option<&Geloescht>,
) -> Response {
    let (zahl, bytes, fehler) = anhaenge.map_or((0, 0, 0), |anhaenge| {
        (
            anhaenge.umfang.anhaenge,
            anhaenge.umfang.bytes,
            anhaenge.fehler,
        )
    });
    let antwort = json!({
        "channel": channel,
        "nachricht": NachrichtAntwort::aus(nachricht),
        "ergebnis": ergebnis,
        "anhaenge": {"geloescht": zahl, "bytes": bytes, "fehler": fehler},
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
    use crate::http::loeschaufbau::{aufbau, naechstes_anhaenge_ereignis};

    /// R00043 B0666 (ADR-0048 K2): bricht der Client nach dem Start des Löschens ab, stehen trotzdem genau eine Zeile
    /// `DEBUG aendern Nachricht gelöscht`, eine `INFORMATION anhang Anhänge gelöscht … grund=nachricht` und ein
    /// Ereignis `anhaenge` (Mutante „Protokoll nach dem await“ rot).
    #[tokio::test(flavor = "multi_thread")]
    async fn der_abbruch_des_clients_nach_dem_loeschsatz_verliert_weder_protokoll_noch_ereignis() {
        let aufbau = aufbau();
        let (zeit, id) = aufbau.nachricht_mit_anhang("beta@127.0.0.1");
        let mut abonnement = aufbau.zustand.bus.abonnieren();

        aufbau
            .abbrechen_am_kuerzen(aufbau.nachricht_loeschen("beta@127.0.0.1", zeit))
            .await;
        let ereignis = naechstes_anhaenge_ereignis(&mut abonnement).await;

        assert_eq!(ereignis, Some(vec![(id.als_text().to_string(), zeit)]));
        let zeilen = aufbau.zeilen.zeilen();
        assert_eq!(
            aufbau.zeilen_mit(" DEBUG aendern Nachricht gelöscht").len(),
            1,
            "{zeilen:#?}"
        );
        let information = aufbau.zeilen_mit(" INFORMATION anhang Anhänge gelöscht");
        assert_eq!(information.len(), 1, "{zeilen:#?}");
        assert!(
            information[0].contains("grund=nachricht"),
            "{information:#?}"
        );
    }

    /// R00043 B0666: scheitert das Kürzen, steht trotz Abbruch des Clients die `ERROR anhang`-Zeile.
    #[tokio::test(flavor = "multi_thread")]
    async fn der_abbruch_des_clients_verliert_auch_den_fehler_beim_kuerzen_nicht() {
        let aufbau = aufbau();
        let (zeit, _) = aufbau.nachricht_mit_anhang("beta@127.0.0.1");
        let datei = aufbau
            .anhaenge
            .anhaenge()
            .into_keys()
            .next()
            .expect("gebunden")
            .2;
        aufbau.anhaenge.kuerzen_scheitert(&datei);

        aufbau
            .abbrechen_am_kuerzen(aufbau.nachricht_loeschen("beta@127.0.0.1", zeit))
            .await;
        aufbau
            .warten_auf_zeile(" INFORMATION anhang Anhänge gelöscht")
            .await;

        let fehler = aufbau.zeilen_mit(" ERROR anhang Löschen gescheitert");
        assert_eq!(fehler.len(), 1, "{:#?}", aufbau.zeilen.zeilen());
    }

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
