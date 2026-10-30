//! Eigene Nachricht bearbeiten (F0132; ADR-0048 Regel 5): `POST /api/channels/{name}/nachrichten/bearbeiten
//! {zeit, text}` mit Sitzung → 200 `{channel, nachricht}` in der neuen Fassung. Prüfreihenfolge (abgewiesen, nie
//! umgeformt): Grenzschutz 403 · Sitzung 401 · Vorab-Buchung 429 · Körper zu groß 413 · kein JSON-Typ 415 · kein JSON
//! 400 · doppeltes oder weiteres Feld 422 `feld` · Channel 404 · 422 `zeit` · 422 `text`/`laenge`/`zeichen` · 422
//! `schluessel` · Restbuchung 429 · unter der Sperre 404 `nachricht` · 403 `fremd` · 409 `geloescht` · 422 `leer` ·
//! unverändert 200 · 507 `quote` · 500. Der Körper wird auf der Werkbank geparst (Grenze über 16 KiB).

use super::Zustand;
use super::abbruch::{Abgebrochen, Grenzort, abbruch_beantworten};
use super::anfrageprotokoll::mit_regel;
use super::bremse::{Gegenstand, gebremst_melden};
use super::koerper::{
    Koerperfehler, Koerperort, json_lesen_auf_werkbank, koerperfehler_beantworten,
};
use super::nachrichten::NachrichtAntwort;
use super::senden::{
    SCHLUESSEL_ANDERS, SCHLUESSEL_FORMAT, groesse_in_worten, mit_tausenderpunkt,
    nicht_gespeichert_antwort, quote_auftreten, schluessel_abgewiesen, schluessel_lesen,
    text_abgewiesen, unbekannt,
};
use super::teilnehmer::Teilnehmer;
use crate::gespraeche::textregel::{Textfehler, laenge_und_zeichen_pruefen};
use crate::gespraeche::{Bearbeitet, Bearbeitfehler, Gespraech, Kennung, Nachricht};
use crate::protokoll::episoden::{Auftreten, Quelle};
use crate::protokoll::vorlauf;
use crate::protokoll::{Bereich, Kontext, Protokoll, Schluessel, Stufe};
use crate::schreibkonto::eimer::kosten_senden;
use crate::uhr::Zeit;
use axum::Json;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Instant;

const ROUTE: &str = "POST /api/channels/{name}/nachrichten/bearbeiten";

/// Körpergrenze des Bearbeitens (ADR-0048 Regel 5): 98.315 Bytes für `text` in der längsten Schreibweise plus
/// `"zeit":"…",` mit 29 Zeichen als `\u`-Folgen ≈ 190 Bytes — innerhalb der Reserve der 128 KiB des Sendens.
pub const BEARBEITEN_GRENZE: usize = 131_072;

/// Wie beim Senden: vorab die kleinste Buchung, der Rest nach allen 4xx-Prüfungen (ADR-0036 Regel 3).
const VORAB_BEARBEITEN: u32 = 1;

pub async fn bearbeiten(
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
            .vorab(&teilnehmer.kennung, teilnehmer.herkunft, VORAB_BEARBEITEN)
    {
        return gebremst_melden(
            &zustand.protokoll,
            gebremst,
            Gegenstand::Bearbeitungen,
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
    let wert =
        match json_lesen_auf_werkbank(&kopf, koerper, BEARBEITEN_GRENZE, &ort, &zustand.werkbank)
            .await
        {
            Ok(Ok(wert)) => wert,
            Ok(Err(fehler)) => return koerper_abgewiesen(fehler),
            Err(abbruch) => return abgebrochen(&zustand, &abbruch, &wer, &name),
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
    let Some(text) = wert.get("text").and_then(Value::as_str) else {
        return mit_regel(text_fehlt(), "text_fehlt", Some(&wer));
    };
    if let Err(fehler) = laenge_und_zeichen_pruefen(text) {
        let regel = match fehler {
            Textfehler::Laenge { .. } => "text_zu_lang",
            Textfehler::Zeichen { .. } | Textfehler::Leer => "text_zeichen",
        };
        return mit_regel(text_abgewiesen(fehler), regel, Some(&wer));
    }
    let Ok(schluessel) = schluessel_lesen(&kopf) else {
        return mit_regel(
            schluessel_abgewiesen(SCHLUESSEL_FORMAT),
            "schluessel_ungueltig",
            Some(&wer),
        );
    };
    if let Err(gebremst) = zustand.schreibkonto.rest(
        &teilnehmer.kennung,
        teilnehmer.herkunft,
        kosten_senden(text.len()),
    ) {
        return gebremst_melden(
            &zustand.protokoll,
            gebremst,
            Gegenstand::Bearbeitungen,
            &teilnehmer,
        );
    }
    let text_bytes = text.len();
    let gespraeche = Arc::clone(&zustand.gespraeche);
    let gespraech = Gespraech::channel(&name);
    let von = teilnehmer.kennung.clone();
    let text = text.to_string();
    let ergebnis = tokio::task::spawn_blocking(move || {
        gespraeche.bearbeiten_mit(&gespraech, &von, ziel, &text, schluessel.as_deref())
    })
    .await;
    let melden = |grund| {
        bearbeitet_melden(&zustand.protokoll, &name, &wer, text_bytes, eintritt, grund);
    };
    match ergebnis {
        Ok(Ok(Bearbeitet::Neu(nachricht))) => {
            vorlauf::eintragen(&zustand.protokoll, zustand.gespraeche.zeit_vorlauf_ms());
            melden(None);
            bearbeitet(&name, &nachricht)
        }
        Ok(Ok(Bearbeitet::Wiederholt(nachricht))) => {
            melden(Some("wiederholung"));
            bearbeitet(&name, &nachricht)
        }
        Ok(Ok(Bearbeitet::Unveraendert(nachricht))) => {
            melden(Some("unveraendert"));
            bearbeitet(&name, &nachricht)
        }
        Ok(Err(fehler)) => {
            abweisung_melden(&zustand.protokoll, &fehler, &name, &wer);
            abgewiesen(fehler, &name, &wer)
        }
        Err(abbruch) => abgebrochen(&zustand, &Abgebrochen::from(abbruch), &wer, &name),
    }
}

/// `DEBUG aendern Nachricht bearbeitet` (ADR-0048 Regel 13): Channel, Kennung, Bytes des neuen Texts als `anzahl`, die
/// Zeit vom Eintritt in den Handler bis zum Schreiben; ohne Satz `grund=wiederholung|unveraendert` — nie der Text.
fn bearbeitet_melden(
    protokoll: &Protokoll,
    channel: &str,
    wer: &str,
    text_bytes: usize,
    eintritt: Instant,
    grund: Option<&str>,
) {
    let kontext = Kontext::neu()
        .mit(Schluessel::Channel, channel)
        .mit(Schluessel::Kennung, wer)
        .mit(Schluessel::Anzahl, text_bytes)
        .mit(Schluessel::DauerMs, eintritt.elapsed().as_millis());
    let kontext = match grund {
        Some(grund) => kontext.mit(Schluessel::Grund, grund),
        None => kontext,
    };
    protokoll.eintragen(
        Stufe::Debug,
        Bereich::Aendern,
        "Nachricht bearbeitet",
        kontext,
    );
}

/// Abweisungen unter der Sperre mit eigener Episode: 403 `fremd` je Kennung (WARNING), 500 je Datei und Grund
/// (ERROR), 507 in der Quote-Episode der Kennung mit `route=bearbeiten`.
fn abweisung_melden(protokoll: &Protokoll, fehler: &Bearbeitfehler, channel: &str, wer: &str) {
    match fehler {
        Bearbeitfehler::Fremd { .. } => protokoll.episode(fremd_auftreten(channel, wer)),
        Bearbeitfehler::Ablage { datei, grund } => {
            protokoll.episode(schreibfehler_auftreten(channel, wer, datei, grund));
        }
        Bearbeitfehler::Quote { belegt, hoechstens } => {
            protokoll.episode(
                quote_auftreten(channel, wer, *belegt, *hoechstens)
                    .einzeln(Schluessel::Route, "bearbeiten"),
            );
        }
        _ => {}
    }
}

/// Quelle der Episode „fremd“ ist die Kennung (M16: je Kennung eine Episode); der Channel steht nur am Beginn.
pub(crate) fn fremd_auftreten(channel: &str, wer: &str) -> Auftreten {
    Auftreten::neu(
        Stufe::Warning,
        Bereich::Aendern,
        crate::protokoll::episodentexte::FREMD_ABGEWIESEN,
        Quelle::neu().mit(Schluessel::Kennung, wer),
    )
    .einzeln(Schluessel::Channel, channel)
}

/// Quelle des Schreibfehlers sind Datei und Grund; die Kennung gehört dem einzelnen Auftreten.
pub(crate) fn schreibfehler_auftreten(
    channel: &str,
    wer: &str,
    datei: &str,
    grund: &str,
) -> Auftreten {
    Auftreten::neu(
        Stufe::Error,
        Bereich::Aendern,
        crate::protokoll::episodentexte::AENDERUNG_NICHT_GESCHRIEBEN,
        Quelle::neu()
            .mit(Schluessel::Channel, channel)
            .mit(Schluessel::Datei, datei)
            .mit(Schluessel::Grund, grund),
    )
    .einzeln(Schluessel::Kennung, wer)
}

/// Die Antworten der Prüfungen unter der Sperre.
fn abgewiesen(fehler: Bearbeitfehler, channel: &str, wer: &str) -> Response {
    match fehler {
        Bearbeitfehler::Unbekannt => mit_regel(unbekannt(channel), "channel_unbekannt", Some(wer)),
        Bearbeitfehler::Nachricht => mit_regel(nachricht_fehlt(), "nachricht", Some(wer)),
        Bearbeitfehler::Fremd { absender } => {
            mit_regel(fremd_abgewiesen(&absender), "fremd", Some(wer))
        }
        Bearbeitfehler::Geloescht => mit_regel(
            regel_antwort(
                StatusCode::CONFLICT,
                "Diese Nachricht ist gelöscht.",
                "geloescht",
            ),
            "geloescht",
            Some(wer),
        ),
        Bearbeitfehler::Leer => {
            mit_regel(text_abgewiesen(Textfehler::Leer), "text_leer", Some(wer))
        }
        Bearbeitfehler::Quote { belegt, hoechstens } => mit_regel(
            quote_erreicht("Nicht bearbeitet", belegt, hoechstens),
            "quote",
            Some(wer),
        ),
        Bearbeitfehler::Ablage { .. } => nicht_gespeichert_antwort(),
        Bearbeitfehler::SchluesselAnders => mit_regel(
            schluessel_abgewiesen(SCHLUESSEL_ANDERS),
            "schluessel_anders",
            Some(wer),
        ),
    }
}

/// 404 `nachricht` (ADR-0048 Regel 5): nie da, entfallen oder ein anderer Channel — kein Orakel.
pub(super) fn nachricht_fehlt() -> Response {
    regel_antwort(
        StatusCode::NOT_FOUND,
        "Diese Nachricht gibt es nicht (mehr).",
        "nachricht",
    )
}

/// 403 `fremd` mit der Kennung des Absenders (ADR-0048 Regel 5).
pub(super) fn fremd_abgewiesen(absender: &Kennung) -> Response {
    regel_antwort(
        StatusCode::FORBIDDEN,
        &format!("Nur der Absender {absender} kann diese Nachricht ändern."),
        "fremd",
    )
}

fn bearbeitet(channel: &str, nachricht: &Nachricht) -> Response {
    let antwort = json!({ "channel": channel, "nachricht": NachrichtAntwort::aus(nachricht) });
    (StatusCode::OK, Json(antwort)).into_response()
}

pub(super) fn regel_antwort(status: StatusCode, fehler: &str, regel: &str) -> Response {
    (status, Json(json!({ "fehler": fehler, "regel": regel }))).into_response()
}

/// Das erste Feld neben `zeit` und `text` (ADR-0048 Regel 5); ein Körper, der kein Objekt ist, hat keines.
fn weiteres_feld(wert: &Value) -> Option<String> {
    wert.as_object()?
        .keys()
        .find(|feld| *feld != "zeit" && *feld != "text")
        .cloned()
}

/// `zeit` als Zeitpunkt nach RFC 3339 — verglichen wird später als Zeitpunkt, gleich in welcher Schreibweise.
pub(super) fn zeit_lesen(wert: &Value) -> Option<Zeit> {
    let text = wert.get("zeit")?.as_str()?;
    Zeit::parse_from_rfc3339(text).ok()
}

fn feld_abgewiesen(feld: &str) -> Response {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(json!({
            "fehler": format!("Unbekanntes Feld {feld} — erlaubt sind zeit und text."),
            "regel": "feld",
            "feld": feld,
        })),
    )
        .into_response()
}

pub(super) fn zeit_abgewiesen() -> Response {
    regel_antwort(
        StatusCode::UNPROCESSABLE_ENTITY,
        "Der Körper braucht zeit als Zeitpunkt nach RFC 3339 — die zeit der Nachricht, zum Beispiel \
         2026-10-30T14:07:12.345+01:00.",
        "zeit",
    )
}

fn text_fehlt() -> Response {
    regel_antwort(
        StatusCode::UNPROCESSABLE_ENTITY,
        "Der Körper braucht text als Zeichenkette.",
        "text",
    )
}

/// 507 an der Plattenquote (ADR-0048 Regel 13, Form nach ADR-0044 Regel 4), ohne `Retry-After`; `was` beginnt den
/// Text („Nicht bearbeitet“, „Nicht gelöscht“).
pub(super) fn quote_erreicht(was: &str, belegt: u64, hoechstens: u64) -> Response {
    let fehler = format!(
        "{was}: die Gespräche belegen {}, die Quote dieser Instanz ist {}. Platz entsteht, wenn alte \
         Tage nach ihrer Aufbewahrungsfrist entfallen (beim nächsten Tageswechsel) oder eine Frist verkürzt wird; \
         die Quote stellt der Betreiber beim Start mit --plattenquote ein.",
        groesse_in_worten(belegt),
        groesse_in_worten(hoechstens)
    );
    (
        StatusCode::INSUFFICIENT_STORAGE,
        Json(json!({
            "fehler": fehler,
            "regel": "quote",
            "belegt": belegt,
            "hoechstens": hoechstens,
        })),
    )
        .into_response()
}

fn abgebrochen(zustand: &Zustand, abbruch: &Abgebrochen, wer: &str, channel: &str) -> Response {
    abbruch_beantworten(
        &zustand.protokoll,
        abbruch,
        &Grenzort {
            route: ROUTE,
            operation: "bearbeiten",
            kennung: Some(wer),
            channel: Some(channel),
        },
        "Nachricht nicht bearbeitet",
    )
}

const BEISPIEL: &str = r#"{"zeit":"2026-10-30T14:07:12.345+01:00","text":"Hallo zusammen"}"#;

fn koerper_abgewiesen(fehler: Koerperfehler) -> Response {
    koerperfehler_beantworten(
        fehler,
        &format!(
            "Der Körper ist zu groß — höchstens {} Byte; längere Texte teilen.",
            mit_tausenderpunkt(BEARBEITEN_GRENZE)
        ),
        BEISPIEL,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Feld und Zeit des Körpers: ein weiteres Feld wird genannt; `zeit` gilt nur als RFC 3339 in einer Zeichenkette.
    #[test]
    fn felder_und_zeit_des_koerpers() {
        assert_eq!(
            weiteres_feld(&json!({"zeit": "x", "text": "y", "anhaenge": []})),
            Some("anhaenge".to_string())
        );
        assert_eq!(weiteres_feld(&json!({"zeit": "x", "text": "y"})), None);
        assert_eq!(weiteres_feld(&json!([1])), None);
        assert!(zeit_lesen(&json!({"zeit": "2026-10-30T13:07:12.345Z"})).is_some());
        for falsch in [
            json!({"zeit": "gestern"}),
            json!({"zeit": 5}),
            json!({}),
            json!("x"),
        ] {
            assert_eq!(zeit_lesen(&falsch), None, "{falsch}");
        }
    }

    /// R00039 B0601 (ADR-0048 Regel 13): 403 `fremd` ist eine WARNING-Episode je Kennung mit Channel am Beginn; der
    /// Schreibfehler eine ERROR-Episode je Datei und Grund, die Ende-Zeile ohne Kennung; die Quote-Episode trägt
    /// `route=bearbeiten`.
    #[test]
    fn abweisungen_des_bearbeitens_sind_episoden() {
        use crate::protokoll::testhilfe::{im_speicher, zeit};
        let (protokoll, speicher, uhr, _) =
            im_speicher("2026-10-28T12:00:00+01:00", Stufe::Information);
        let datei = "channels/allgemein/2026-10-28.log";
        let fremd = |wer: &str| Bearbeitfehler::Fremd {
            absender: crate::gespraeche::Kennung::bilden(wer).expect("Kennung"),
        };
        let ablage = Bearbeitfehler::Ablage {
            datei: datei.to_string(),
            grund: "Kein Platz mehr".to_string(),
        };

        for wer in ["gamma@127.0.0.1", "gamma@127.0.0.1", "delta@127.0.0.1"] {
            abweisung_melden(&protokoll, &fremd("beta@127.0.0.1"), "allgemein", wer);
        }
        abweisung_melden(&protokoll, &ablage, "allgemein", "beta@127.0.0.1");
        abweisung_melden(&protokoll, &ablage, "allgemein", "gamma@127.0.0.1");
        abweisung_melden(
            &protokoll,
            &Bearbeitfehler::Quote {
                belegt: 100,
                hoechstens: 1000,
            },
            "allgemein",
            "beta@127.0.0.1",
        );
        uhr.stellen(zeit("2026-10-28T12:01:00+01:00"));
        protokoll.episoden_schliessen();

        let zeilen = speicher.zeilen();
        let von = |teil: &str| -> Vec<&String> {
            zeilen.iter().filter(|zeile| zeile.contains(teil)).collect()
        };
        let fremde = von(" WARNING aendern Ändern fremder Nachricht abgewiesen — Episode ");
        assert_eq!(fremde.len(), 4, "{zeilen:#?}");
        assert!(fremde[0].ends_with("Episode beginnt | kennung=gamma@127.0.0.1 channel=allgemein"));
        assert!(fremde[1].ends_with("Episode beginnt | kennung=delta@127.0.0.1 channel=allgemein"));
        let ende_gamma = fremde
            .iter()
            .find(|zeile| zeile.contains("Episode endet") && zeile.contains("kennung=gamma@"))
            .expect("Ende gamma");
        assert!(
            ende_gamma.contains("Episode endet: 2 in ") && !ende_gamma.contains("channel="),
            "{fremde:#?}"
        );
        let fehler = von(" ERROR aendern Änderung nicht geschrieben — Episode ");
        assert_eq!(fehler.len(), 2, "{zeilen:#?}");
        assert!(
            fehler[0].contains("kennung=beta@127.0.0.1")
                && fehler[0].contains(&format!("datei={datei}"))
        );
        assert!(
            fehler[1].contains("Episode endet: 2 in ") && !fehler[1].contains("kennung="),
            "{fehler:#?}"
        );
        let quote = von(" WARNING quote ");
        assert!(quote[0].contains("route=bearbeiten"), "{quote:#?}");
    }
}
