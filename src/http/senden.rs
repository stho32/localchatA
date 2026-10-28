//! Nachricht senden (F0041; ADR-0034): `POST /api/channels/{name}/nachrichten {text}` mit Sitzung → 201
//! `{channel, nachricht}`. Prüfreihenfolge (abgewiesen, nie umgeformt): Grenzschutz 403 · Sitzung 401 · Körper zu
//! groß 413 · kein JSON-Typ 415 · kein JSON 400 · Channel 404 · Sendekörper 422 `text`/`feld` · Textregel 422
//! `leer`/`laenge`/`zeichen` · Plattenfehler 500. Der Körper wird auf der Werkbank geparst (Grenze über 16 KiB).

use super::Zustand;
use super::abbruch::{Abgebrochen, Grenzort, abbruch_beantworten};
use super::anfrageprotokoll::Eingetragen;
use super::koerper::{Koerperfehler, Koerperort, ZU_LANGSAM, json_lesen_auf_werkbank};
use super::nachrichten::NachrichtAntwort;
use super::teilnehmer::Teilnehmer;
use crate::gespraeche::textregel::{TEXT_HOECHSTENS_BYTES, Textfehler, text_pruefen};
use crate::gespraeche::{Gespraech, Kennung, Nachricht, Sendefehler};
use crate::protokoll::episoden::Auftreten;
use crate::protokoll::{Bereich, Kontext, Schluessel, Stufe};
use crate::schreibkonto::{Gebremst, Grenze};
use crate::uhr::{vorlauf_melden, vorlauf_text};
use axum::Json;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::header::{CONNECTION, RETRY_AFTER};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};
use std::net::IpAddr;
use std::sync::Arc;

const ROUTE: &str = "POST /api/channels/{name}/nachrichten";

/// Körpergrenze des Sendens (ADR-0034 Regel 2): der größte gültige Körper in der längsten Schreibweise hat
/// 98.315 Bytes (`{"text":"` + 16.384 × `\u0061` + `"}`); mit 30 % Reserve 127.810, gerundet auf 128 KiB.
pub const SENDEN_GRENZE: usize = 131_072;

pub async fn senden(
    teilnehmer: Teilnehmer,
    State(zustand): State<Arc<Zustand>>,
    Path(name): Path<String>,
    kopf: HeaderMap,
    koerper: Body,
) -> Response {
    let wer = teilnehmer.kennung.als_text().to_string();
    let ort = Koerperort {
        protokoll: &zustand.protokoll,
        route: ROUTE,
        herkunft: teilnehmer.herkunft,
        kennung: Some(&wer),
    };
    let wert = match json_lesen_auf_werkbank(&kopf, koerper, SENDEN_GRENZE, &ort, &zustand.werkbank)
        .await
    {
        Ok(Ok(wert)) => wert,
        Ok(Err(fehler)) => return kein_json(fehler),
        Err(abbruch) => return abgebrochen(&zustand, &abbruch, &wer, &name),
    };
    if !zustand.gespraeche.channel_bekannt(&name) {
        return unbekannt(&name);
    }
    let text = match sendekoerper_lesen(&wert) {
        Ok(text) => text.to_string(),
        Err(fehler) => return sendekoerper_abgewiesen(fehler),
    };
    if let Err(fehler) = text_pruefen(&text) {
        return text_abgewiesen(fehler);
    }
    let gespraeche = Arc::clone(&zustand.gespraeche);
    let gespraech = Gespraech::channel(&name);
    let von = teilnehmer.kennung.clone();
    let art = teilnehmer.art;
    let ergebnis =
        tokio::task::spawn_blocking(move || gespraeche.senden(&gespraech, &von, art, &text)).await;
    match ergebnis {
        Ok(Ok(nachricht)) => {
            vorlauf_eintragen(&zustand.protokoll, zustand.gespraeche.zeit_vorlauf_ms());
            gesendet(&name, &nachricht)
        }
        Ok(Err(Sendefehler::Unbekannt)) => unbekannt(&name),
        Ok(Err(Sendefehler::Ablage { datei, grund })) => {
            nicht_gespeichert(&zustand, &name, &wer, &datei, &grund)
        }
        Err(abbruch) => abgebrochen(&zustand, &Abgebrochen::from(abbruch), &wer, &name),
    }
}

/// 404 „# name gibt es nicht.“ — auch für ungültige Namen (R00011 F0041); der Name steht nur im JSON-Text.
fn unbekannt(name: &str) -> Response {
    let fehler = format!("# {name} gibt es nicht.");
    (StatusCode::NOT_FOUND, Json(json!({ "fehler": fehler }))).into_response()
}

fn gesendet(channel: &str, nachricht: &Nachricht) -> Response {
    let antwort = json!({ "channel": channel, "nachricht": NachrichtAntwort::aus(nachricht) });
    (StatusCode::CREATED, Json(antwort)).into_response()
}

/// 500 mit deutschem Satz; Datei und Grund stehen im Betriebsprotokoll (ab ERROR auch auf stderr), nie der Text.
fn nicht_gespeichert(
    zustand: &Zustand,
    channel: &str,
    wer: &str,
    datei: &str,
    grund: &str,
) -> Response {
    zustand.protokoll.eintragen(
        Stufe::Error,
        Bereich::Senden,
        "Nachricht nicht gespeichert",
        Kontext::neu()
            .mit(Schluessel::Channel, channel)
            .mit(Schluessel::Kennung, wer)
            .mit(Schluessel::Datei, datei)
            .mit(Schluessel::Grund, grund),
    );
    let fehler = "Nicht gespeichert — der Server konnte nicht schreiben.";
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "fehler": fehler })),
    )
        .into_response()
}

fn abgebrochen(zustand: &Zustand, abbruch: &Abgebrochen, wer: &str, channel: &str) -> Response {
    abbruch_beantworten(
        &zustand.protokoll,
        abbruch,
        &Grenzort {
            route: ROUTE,
            operation: "senden",
            kennung: Some(wer),
            channel: Some(channel),
        },
        "Nachricht nicht gesendet",
    )
}

const BEISPIEL: &str = r#"{"text":"Hallo"}"#;

/// Wie beim Anlegen: deutsche `{fehler}`-Antwort mit Grund und Ausweg; nach 413 und 408 schließt die Verbindung.
fn kein_json(fehler: Koerperfehler) -> Response {
    let mit_beispiel = |grund: &str| format!("{grund}, zum Beispiel {BEISPIEL}.");
    let (status, text) = match fehler {
        Koerperfehler::KeinJson => (
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            mit_beispiel("Der Körper muss JSON sein — mit Content-Type: application/json schicken"),
        ),
        Koerperfehler::Kaputt | Koerperfehler::Abgebrochen => (
            StatusCode::BAD_REQUEST,
            mit_beispiel("Der Körper ist kein gültiges JSON — ein JSON-Objekt schicken"),
        ),
        Koerperfehler::ZuGross => (
            StatusCode::PAYLOAD_TOO_LARGE,
            mit_beispiel(&format!(
                "Der Körper ist zu groß — höchstens {SENDEN_GRENZE} Bytes, längere Texte teilen"
            )),
        ),
        Koerperfehler::ZuLangsam => (StatusCode::REQUEST_TIMEOUT, ZU_LANGSAM.to_string()),
    };
    let antwort = (status, Json(json!({ "fehler": text })));
    let schliessen = matches!(fehler, Koerperfehler::ZuGross | Koerperfehler::ZuLangsam);
    let mut antwort = if schliessen {
        ([(CONNECTION, "close")], antwort).into_response()
    } else {
        antwort.into_response()
    };
    antwort.extensions_mut().insert(Eingetragen);
    antwort
}

fn sendekoerper_abgewiesen(fehler: Sendekoerperfehler) -> Response {
    let antwort = match fehler {
        Sendekoerperfehler::Text => json!({
            "fehler": "Der Körper braucht text als Zeichenkette.",
            "regel": "text",
        }),
        Sendekoerperfehler::Feld { feld } => json!({
            "fehler": format!("Unbekanntes Feld {feld} — erlaubt ist nur text."),
            "regel": "feld",
            "feld": feld,
        }),
    };
    (StatusCode::UNPROCESSABLE_ENTITY, Json(antwort)).into_response()
}

/// Texte der Abweisung, die der Browser wörtlich zeigt (R00011 F0041); die Stelle für Menschen ab 1.
fn text_abgewiesen(fehler: Textfehler) -> Response {
    let antwort = match fehler {
        Textfehler::Leer => json!({ "fehler": "Die Nachricht ist leer.", "regel": "leer" }),
        Textfehler::Laenge { bytes } => json!({
            "fehler": format!(
                "Zu lang: {} von höchstens {} Byte — bitte teilen.",
                mit_tausenderpunkt(bytes),
                mit_tausenderpunkt(TEXT_HOECHSTENS_BYTES)
            ),
            "regel": "laenge",
            "hoechstens": TEXT_HOECHSTENS_BYTES,
            "bytes": bytes,
        }),
        Textfehler::Zeichen { stelle, zeichen } => json!({
            "fehler": format!("Steuerzeichen {zeichen} an Stelle {} — bitte entfernen.", stelle + 1),
            "regel": "zeichen",
            "stelle": stelle,
            "zeichen": zeichen,
        }),
    };
    (StatusCode::UNPROCESSABLE_ENTITY, Json(antwort)).into_response()
}

/// Meldet einen Vorlauf über 60 s als Episode `WARNING zeit` der Instanz (R00016, ADR-0035 Regel 4 mit Ergänzung).
fn vorlauf_eintragen(protokoll: &crate::protokoll::Protokoll, vorlauf_ms: u64) {
    if !vorlauf_melden(vorlauf_ms) {
        return;
    }
    protokoll.episode(Auftreten {
        stufe: Stufe::Warning,
        bereich: Bereich::Zeit,
        text: "Nachrichtenzeit läuft der Systemuhr voraus",
        quelle: "instanz".to_string(),
        kontext: Kontext::neu().mit(Schluessel::DauerMs, vorlauf_ms),
        art: None,
        verworfen: None,
        bytes: None,
        anzeige: Some(vorlauf_text(vorlauf_ms)),
    });
}

/// 429 `{fehler, grenze, wieder_in_ms}` mit `Retry-After` (B0234, ADR-0036 Regel 4).
#[allow(dead_code)] // bis B0233 das Senden bucht
fn gebremst_beantworten(gebremst: Gebremst, kennung: &Kennung, herkunft: IpAddr) -> Response {
    let wieder_in_ms = u64::try_from(gebremst.wieder_in.as_millis()).unwrap_or(u64::MAX);
    let sekunden = wieder_in_ms.div_ceil(1_000).max(1);
    let (grenze, wer) = match gebremst.grenze {
        Grenze::Kennung => ("kennung", kennung.als_text().to_string()),
        Grenze::Herkunft => (
            "herkunft",
            format!("{herkunft} (alle Teilnehmer dieser Adresse zusammen)"),
        ),
    };
    let fehler = format!("Zu viele Nachrichten von {wer} — wieder möglich in {sekunden} s.");
    (
        StatusCode::TOO_MANY_REQUESTS,
        [(RETRY_AFTER, sekunden.to_string())],
        Json(json!({ "fehler": fehler, "grenze": grenze, "wieder_in_ms": wieder_in_ms })),
    )
        .into_response()
}

/// `17020` → `17.020`.
fn mit_tausenderpunkt(zahl: usize) -> String {
    let ziffern = zahl.to_string();
    let mut text = String::new();
    for (stelle, ziffer) in ziffern.chars().enumerate() {
        if stelle > 0 && (ziffern.len() - stelle).is_multiple_of(3) {
            text.push('.');
        }
        text.push(ziffer);
    }
    text
}

/// Warum ein Sendekörper keinen Text liefert (422 `text` bzw. `feld`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sendekoerperfehler {
    /// Kein Objekt, `text` fehlt oder ist keine Zeichenkette.
    Text,
    /// Ein weiteres Feld neben `text`.
    Feld { feld: String },
}

/// Der Text aus dem Körper `{"text": "…"}` (B0207, ADR-0034 Regel 3.3); jedes weitere Feld wird abgewiesen.
pub fn sendekoerper_lesen(wert: &Value) -> Result<&str, Sendekoerperfehler> {
    let objekt = wert.as_object().ok_or(Sendekoerperfehler::Text)?;
    let text = objekt
        .get("text")
        .and_then(Value::as_str)
        .ok_or(Sendekoerperfehler::Text)?;
    if let Some(feld) = objekt.keys().find(|feld| *feld != "text") {
        return Err(Sendekoerperfehler::Feld { feld: feld.clone() });
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// R00011 F0042 „Vorlauf“ mit der Episode des Betriebsprotokolls (R00016): über 60 s eine Zeile `WARNING zeit`
    /// mit Stunden und Minuten und `dauer_ms`; die zweite Nachricht keine; nach der Ruhe endet die Episode; ein neuer
    /// Vorlauf beginnt eine neue. 60.000 ms gibt keine Zeile.
    #[test]
    fn ein_vorlauf_ueber_60_s_ist_eine_episode_des_protokolls() {
        use crate::protokoll::Stufe;
        use crate::protokoll::testhilfe::{im_speicher, zeit};
        let (protokoll, speicher, uhr, stderr) =
            im_speicher("2026-10-28T11:58:59+01:00", Stufe::Information);
        let zeit_zeilen = || {
            speicher
                .zeilen()
                .into_iter()
                .filter(|zeile| zeile.contains(" WARNING zeit "))
                .collect::<Vec<_>>()
        };

        vorlauf_eintragen(&protokoll, 60_000);
        let bei_60_000 = zeit_zeilen().len();
        vorlauf_eintragen(&protokoll, 61_001);
        vorlauf_eintragen(&protokoll, 61_002);
        let nach_zweien = zeit_zeilen();
        uhr.stellen(zeit("2026-10-28T12:01:00+01:00"));
        protokoll.episoden_schliessen();
        let nach_der_ruhe = zeit_zeilen().len();
        vorlauf_eintragen(&protokoll, 7_410_002);

        assert_eq!(bei_60_000, 0);
        assert_eq!(nach_zweien.len(), 1, "{nach_zweien:#?}");
        assert!(
            nach_zweien[0].contains(
                "Nachrichtenzeit läuft der Systemuhr 0 h 01 min voraus (Uhr zurückgestellt?) — Zeiten bleiben streng steigend, bis die Uhr aufholt. — Episode beginnt | "
            ),
            "{nach_zweien:#?}"
        );
        assert!(
            nach_zweien[0].contains("dauer_ms=61001"),
            "{nach_zweien:#?}"
        );
        assert_eq!(nach_der_ruhe, 2, "Ende nach der Ruhe");
        let alle = zeit_zeilen();
        assert_eq!(alle.len(), 3);
        assert!(alle[2].contains(" 2 h 03 min voraus") && alle[2].contains("Episode beginnt"));
        assert!(
            stderr
                .zeilen()
                .iter()
                .any(|zeile| zeile.contains(" 2 h 03 min voraus"))
        );
    }

    async fn teile(antwort: Response) -> (u16, Option<String>, serde_json::Value) {
        let status = antwort.status().as_u16();
        let retry = antwort
            .headers()
            .get("retry-after")
            .map(|wert| wert.to_str().expect("Text").to_string());
        let bytes = axum::body::to_bytes(antwort.into_body(), usize::MAX)
            .await
            .expect("Körper");
        (status, retry, serde_json::from_slice(&bytes).expect("JSON"))
    }

    /// B0234 (ADR-0036 Regel 4): 429 mit Grenze, `wieder_in_ms`, `Retry-After` = ⌈ms / 1000⌉, mindestens 1, und
    /// dem Text je Eimer.
    #[tokio::test]
    async fn gebremst_ist_429_mit_retry_after_und_text_je_eimer() {
        use std::time::Duration;
        let beta = Kennung::bilden("beta@127.0.0.1").expect("Kennung");
        let antwort = |grenze, ms| {
            gebremst_beantworten(
                Gebremst {
                    grenze,
                    wieder_in: Duration::from_millis(ms),
                },
                &beta,
                "127.0.0.2".parse().expect("IP"),
            )
        };

        let kennung = teile(antwort(Grenze::Kennung, 2_500)).await;
        let herkunft = teile(antwort(Grenze::Herkunft, 125)).await;
        let genau = teile(antwort(Grenze::Kennung, 3_000)).await;
        let null = teile(antwort(Grenze::Kennung, 0)).await;

        assert_eq!(
            kennung,
            (
                429,
                Some("3".to_string()),
                json!({"fehler": "Zu viele Nachrichten von beta@127.0.0.1 — wieder möglich in 3 s.", "grenze": "kennung", "wieder_in_ms": 2_500})
            )
        );
        assert_eq!(
            herkunft,
            (
                429,
                Some("1".to_string()),
                json!({"fehler": "Zu viele Nachrichten von 127.0.0.2 (alle Teilnehmer dieser Adresse zusammen) — wieder möglich in 1 s.", "grenze": "herkunft", "wieder_in_ms": 125})
            )
        );
        assert_eq!(genau.1.as_deref(), Some("3"));
        assert_eq!(null.1.as_deref(), Some("1"));
    }

    #[test]
    fn zahlen_mit_tausenderpunkt() {
        assert_eq!(mit_tausenderpunkt(5), "5");
        assert_eq!(mit_tausenderpunkt(999), "999");
        assert_eq!(mit_tausenderpunkt(16_384), "16.384");
        assert_eq!(mit_tausenderpunkt(1_234_567), "1.234.567");
    }

    #[test]
    fn ein_objekt_mit_text_liefert_den_text() {
        assert_eq!(sendekoerper_lesen(&json!({"text": "Hallo"})), Ok("Hallo"));
        assert_eq!(sendekoerper_lesen(&json!({"text": "  "})), Ok("  "));
    }

    #[test]
    fn ohne_text_als_zeichenkette_ist_es_regel_text() {
        for wert in [
            json!({}),
            json!({"text": 5}),
            json!({"text": null}),
            json!([1]),
            json!("Hallo"),
        ] {
            assert_eq!(
                sendekoerper_lesen(&wert),
                Err(Sendekoerperfehler::Text),
                "{wert}"
            );
        }
    }

    #[test]
    fn ein_weiteres_feld_wird_mit_seinem_namen_abgewiesen() {
        assert_eq!(
            sendekoerper_lesen(&json!({"text": "Hallo", "antwort": "x"})),
            Err(Sendekoerperfehler::Feld {
                feld: "antwort".into()
            })
        );
    }
}
