//! Verlauf beantworten (B0144, B0146; ADR-0025): `GET /api/channels/{name}/nachrichten[?vor=<zeit> | ?nach=<zeit>]
//! [&n=1..200]` mit Sitzung → 200 `{channel, nachrichten, aelter, neuer}` | 401 | 404 `{fehler}` | 422 `{fehler,
//! regel}`. Die Abfrage wird vor jeder Arbeit begrenzt und geprüft, abgewiesen statt umgeformt.

use super::Zustand;
use super::abbruch::{Grenzort, abbruch_beantworten, interner_fehler};
use super::anfrageprotokoll::mit_regel;
use super::teilnehmer::Teilnehmer;
use super::teilnehmerliste::zeit_als_text;
use super::ziel::{Segment, Ziel, bezug, ziel_aus_pfad};
use crate::gespraeche::abfrage::{Abfrage, Abschnitt, Blockgroesse};
use crate::gespraeche::zitat::Zitat;
use crate::gespraeche::{Aenderungszustand, Art, Nachricht, Nachrichtenblock, Unbekannt};
use crate::name::Channelname;
use crate::protokoll::{Bereich, Kontext, Schluessel, Stufe};
use crate::uhr::Zeit;
use axum::Json;
use axum::extract::{Path, RawQuery, State};
use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use serde_json::json;
use std::sync::Arc;

/// Höchstens so viele Satzbytes je Verlaufsblock (ADR-0039 Regel 1, R00013) — 256 KiB. Herleitung:
/// - Kontrollpaar: der größte realistische Block zu 200 hat 80,7 KiB (100.000 Nachrichten aus dem
///   Testdaten-Erzeuger) — das Budget liegt 3,2-fach darüber, realistische Blöcke bleiben vollständig.
/// - Größte Nachricht: 16 384 Bytes Text (ADR-0034); ein Satz ist höchstens ≈ 32 768 Bytes + Kopf (eine Zeile
///   je Zeichen) — also mindestens 7 Höchstgrößen je Block, bei einzeiligen 16-KiB-Texten 15.
/// - Speicher: JSON ≤ 2 × Budget + 60 Bytes je Nachricht ≈ 0,5 MiB je Antwort; 160 Verbindungen einer Herkunft
///   ≤ 80 MiB (Rechnung Q8 in ADR-0039, Ergänzung R00015).
pub const VERLAUF_BUDGET: usize = 262_144;

/// `GET /api/channels/{name}/nachrichten`.
pub async fn beantworten(
    teilnehmer: Teilnehmer,
    State(zustand): State<Arc<Zustand>>,
    Path(name): Path<String>,
    RawQuery(query): RawQuery,
) -> Response {
    verlauf_in(teilnehmer, zustand, Ziel::Channel(name), query).await
}

/// `GET /api/direkt/{kennung}/nachrichten` (ADR-0055 Regel 3): die Sitzung vor dem Pfad; ohne Gespräch 200 leer —
/// gelesen wird nie angelegt (Matrix R3).
pub(super) async fn beantworten_direkt(
    teilnehmer: Teilnehmer,
    State(zustand): State<Arc<Zustand>>,
    segment: Segment,
    RawQuery(query): RawQuery,
) -> Response {
    match ziel_aus_pfad(&teilnehmer, &segment) {
        Ok(ziel) => verlauf_in(teilnehmer, zustand, ziel, query).await,
        Err(fehler) => fehler.antwort(&teilnehmer.kennung),
    }
}

async fn verlauf_in(
    teilnehmer: Teilnehmer,
    zustand: Arc<Zustand>,
    ziel: Ziel,
    query: Option<String>,
) -> Response {
    let name = ziel.name();
    let abfrage = match abfrage_lesen(query.as_deref()) {
        Ok(abfrage) => abfrage,
        Err(fehler) => {
            return mit_regel(
                abgewiesen(fehler),
                "abfrage_ungueltig",
                Some(teilnehmer.kennung.als_text()),
            );
        }
    };
    let gespraeche = Arc::clone(&zustand.gespraeche);
    let gefragt = ziel.clone();
    let testpanik = zustand.testpanik.clone();
    let beginn = std::time::Instant::now();
    let ergebnis = zustand
        .werkbank
        .ausfuehren(move || {
            testpanik.verlauf();
            let block = match &gefragt {
                Ziel::Channel(name) => gespraeche.verlauf(name, &abfrage, VERLAUF_BUDGET),
                // Ohne Gespräch ein leerer Verlauf, kein Fehler und kein Ordner (ADR-0055 E5).
                Ziel::Direkt { .. } => Ok(gespraeche
                    .verlauf_von(&gefragt.gespraech(), &abfrage, VERLAUF_BUDGET)
                    .unwrap_or_default()),
            };
            block.map(|block| (block.nachrichten.len(), block_als_bytes(&gefragt, &block)))
        })
        .await;
    match ergebnis {
        Ok(Ok((anzahl, Ok(bytes)))) => {
            zustand.protokoll.eintragen(
                Stufe::Debug,
                Bereich::Verlauf,
                "Verlaufsblock gelesen",
                Kontext::neu()
                    .mit(bezug(&name), &name)
                    .mit(Schluessel::Anzahl, anzahl)
                    .mit(Schluessel::DauerMs, beginn.elapsed().as_millis()),
            );
            ([(CONTENT_TYPE, "application/json")], bytes).into_response()
        }
        Ok(Err(Unbekannt)) => mit_regel(
            unbekannt(&name),
            "channel_unbekannt",
            Some(teilnehmer.kennung.als_text()),
        ),
        Ok(Ok((_, Err(fehler)))) => nicht_gelesen(
            &zustand,
            teilnehmer.kennung.als_text(),
            &name,
            &fehler.to_string(),
        ),
        Err(abbruch) => abbruch_beantworten(
            &zustand.protokoll,
            &abbruch,
            &Grenzort {
                route: ROUTE,
                operation: "verlauf",
                kennung: Some(teilnehmer.kennung.als_text()),
                channel: Some(&name),
            },
            NICHT_GELESEN,
        ),
    }
}

const NICHT_GELESEN: &str = "Verlauf nicht gelesen";
const ROUTE: &str = "GET /api/channels/{name}/nachrichten";

/// Ein Block ließ sich nicht serialisieren — eine verletzte Annahme (ADR-0042 Ergänzung #24 Punkt 4): EXCEPTION
/// mit Ort und `grund=serialisierung`, der Fremdtext von `serde_json` nur in `ursache`; 500 mit deutschem Satz.
fn nicht_gelesen(zustand: &Zustand, kennung: &str, channel: &str, ursache: &str) -> Response {
    zustand.protokoll.exception(
        Bereich::Verlauf,
        "Verlauf nicht gelesen — Block ließ sich nicht serialisieren",
        Kontext::neu()
            .mit(Schluessel::Route, ROUTE)
            .mit(Schluessel::Operation, "verlauf")
            .mit(Schluessel::Status, 500)
            .mit(bezug(channel), channel)
            .mit(Schluessel::Kennung, kennung)
            .mit(Schluessel::Grund, "serialisierung")
            .mit(Schluessel::Ursache, ursache),
    );
    interner_fehler(NICHT_GELESEN)
}

/// `{channel, nachrichten, aelter, neuer}`, Nachrichten älteste zuerst (ADR-0025 Regel 3); ein Direktgespräch trägt
/// `channel: null` und `direkt` (die Kennung des Gegenübers), ein Channel kein `direkt`.
#[derive(Serialize)]
struct BlockAntwort<'a> {
    channel: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    direkt: Option<&'a str>,
    nachrichten: Vec<NachrichtAntwort<'a>>,
    aelter: bool,
    neuer: bool,
}

/// `{zeit, von, art, text, anhaenge}` — die eine Form einer Nachricht an der Grenze: Verlauf, 201 des Sendens und
/// Ereignis im Strom (ADR-0034 Regel 4, ADR-0047 Frage 5); `art` ist `null` ohne Metazeile; `anhaenge` ist immer eine
/// Liste in Sendefolge.
#[derive(Serialize)]
pub struct NachrichtAntwort<'a> {
    zeit: String,
    von: &'a str,
    art: Option<Art>,
    text: &'a str,
    anhaenge: Vec<AnhangAntwort<'a>>,
    /// `null` | `bearbeitet` | `geloescht` (ADR-0048 Regel 4).
    zustand: Option<&'static str>,
    /// Zeit des letzten Änderungssatzes oder `null`.
    geaendert: Option<String>,
    /// Das Zitat einer Antwort oder `null` (ADR-0053 Regel 3).
    antwort_auf: Option<ZitatAntwort<'a>>,
}

/// `{zeit, von, zustand, textanfang, anhang, geaendert}` — der Bezug einer Antwort, aufgelöst (ADR-0053 F2).
#[derive(Serialize)]
pub struct ZitatAntwort<'a> {
    zeit: String,
    von: &'a str,
    /// `da` | `bearbeitet` | `geloescht` | `entfallen`.
    zustand: &'static str,
    textanfang: &'a str,
    anhang: Option<&'a str>,
    geaendert: Option<String>,
}

impl<'a> ZitatAntwort<'a> {
    pub fn aus(zitat: &'a Zitat) -> ZitatAntwort<'a> {
        ZitatAntwort {
            zeit: zeit_als_text(&zitat.zeit),
            von: zitat.von.als_text(),
            zustand: zitat.zustand.als_text(),
            textanfang: &zitat.textanfang,
            anhang: zitat.anhang.as_deref(),
            geaendert: zitat.geaendert.as_ref().map(zeit_als_text),
        }
    }
}

/// `{id, name, bytes, art, breite, hoehe, url, zustand}` — Maße nur bei Bildern, sonst `null`; `url` relativ.
#[derive(Serialize)]
pub struct AnhangAntwort<'a> {
    id: &'a str,
    name: &'a str,
    bytes: u64,
    art: &'a str,
    breite: Option<u32>,
    hoehe: Option<u32>,
    url: String,
    /// `vorhanden` | `geloescht` (ADR-0047 Ergänzung „F1′“ Regel 2).
    zustand: &'static str,
}

impl<'a> NachrichtAntwort<'a> {
    pub fn aus(nachricht: &'a Nachricht) -> NachrichtAntwort<'a> {
        NachrichtAntwort {
            zeit: zeit_als_text(&nachricht.zeit),
            von: nachricht.von.als_text(),
            art: nachricht.art,
            text: &nachricht.text,
            anhaenge: nachricht
                .anhaenge
                .iter()
                .map(|anhang| AnhangAntwort {
                    id: anhang.id.als_text(),
                    name: &anhang.name,
                    bytes: anhang.bytes,
                    art: anhang.art,
                    breite: anhang.masse.map(|(breite, _)| breite),
                    hoehe: anhang.masse.map(|(_, hoehe)| hoehe),
                    url: format!("/anhaenge/{}", anhang.id.als_text()),
                    zustand: if anhang.geloescht {
                        "geloescht"
                    } else {
                        "vorhanden"
                    },
                })
                .collect(),
            zustand: nachricht
                .geaendert
                .map(|geaendert| match geaendert.zustand {
                    Aenderungszustand::Bearbeitet => "bearbeitet",
                    Aenderungszustand::Geloescht => "geloescht",
                }),
            geaendert: nachricht
                .geaendert
                .map(|geaendert| zeit_als_text(&geaendert.zeit)),
            antwort_auf: nachricht.antwort_auf.as_deref().map(ZitatAntwort::aus),
        }
    }
}

/// Die fertigen JSON-Bytes eines Blocks — im Blocking-Teil, ohne `Value`-Baum (ADR-0039 Regel 4).
fn block_als_bytes(ziel: &Ziel, block: &Nachrichtenblock) -> serde_json::Result<Vec<u8>> {
    let (channel, direkt) = match ziel {
        Ziel::Channel(name) => (Some(name.as_str()), None),
        Ziel::Direkt { gegenueber, .. } => (None, Some(gegenueber.als_text())),
    };
    serde_json::to_vec(&BlockAntwort {
        channel,
        direkt,
        nachrichten: block
            .nachrichten
            .iter()
            .map(NachrichtAntwort::aus)
            .collect(),
        aelter: block.aelter,
        neuer: block.neuer,
    })
}

/// 404 — den Namen nennt die Antwort nur, wenn er ein gültiger Channelname ist.
fn unbekannt(name: &str) -> Response {
    let fehler = match Channelname::bilden(name) {
        Ok(gueltig) => format!("# {} gibt es nicht.", gueltig.als_text()),
        Err(_) => {
            "Diesen Channel gibt es nicht — ein Channelname hat nur a–z, 0–9 und -.".to_string()
        }
    };
    (StatusCode::NOT_FOUND, Json(json!({ "fehler": fehler }))).into_response()
}

fn abgewiesen(fehler: Abfragefehler) -> Response {
    let (regel, text) = match fehler {
        Abfragefehler::Cursor => (
            "cursor",
            "Höchstens einer von vor und nach, als Zeit nach RFC 3339 mit Versatz; + als %2B kodieren — zum Beispiel vor=2026-10-28T14:00:00.000%2B01:00.",
        ),
        Abfragefehler::N => (
            "n",
            "n ist eine ganze Zahl von 1 bis 200 — zum Beispiel n=100.",
        ),
    };
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(json!({ "fehler": text, "regel": regel })),
    )
        .into_response()
}

/// Welche Regel eine Abfrage verletzt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Abfragefehler {
    Cursor,
    N,
}

/// Längere Abfragen liest niemand: zwei Cursor mit Kodierung und `n` brauchen unter 100 Zeichen.
const ABFRAGE_HOECHSTENS: usize = 256;

/// Der Query-Text (ohne `?`) als Abfrage: höchstens einer von `vor` und `nach` als Zeit nach RFC 3339 mit Versatz
/// (ein unkodiertes `+` ist ein Leerzeichen und damit unlesbar), `n` als ganze Zahl 1–200, Standard 100.
/// Unbekannte Schlüssel werden übergangen.
pub fn abfrage_lesen(query: Option<&str>) -> Result<Abfrage, Abfragefehler> {
    let query = query.unwrap_or_default();
    if query.len() > ABFRAGE_HOECHSTENS {
        return Err(Abfragefehler::Cursor);
    }
    let mut abschnitt = None;
    let mut n = None;
    for paar in query.split('&').filter(|paar| !paar.is_empty()) {
        let (schluessel, wert) = paar.split_once('=').unwrap_or((paar, ""));
        match dekodieren(schluessel).as_deref() {
            Some("vor") if abschnitt.is_none() => {
                abschnitt = Some(Abschnitt::Vor(cursor_lesen(wert)?))
            }
            Some("nach") if abschnitt.is_none() => {
                abschnitt = Some(Abschnitt::Nach(cursor_lesen(wert)?))
            }
            Some("vor" | "nach") => return Err(Abfragefehler::Cursor),
            Some("n") if n.is_none() => n = Some(n_lesen(wert)?),
            Some("n") => return Err(Abfragefehler::N),
            _ => {}
        }
    }
    Ok(Abfrage {
        abschnitt: abschnitt.unwrap_or(Abschnitt::Neueste),
        n: n.unwrap_or_else(Blockgroesse::standard),
    })
}

fn cursor_lesen(wert: &str) -> Result<Zeit, Abfragefehler> {
    dekodieren(wert)
        .and_then(|text| Zeit::parse_from_rfc3339(&text).ok())
        .ok_or(Abfragefehler::Cursor)
}

/// Eine bis drei Ziffern, sonst keine ganze Zahl im Sinn der Regel (kein Vorzeichen, kein Exponent).
fn n_lesen(wert: &str) -> Result<Blockgroesse, Abfragefehler> {
    let text = dekodieren(wert).ok_or(Abfragefehler::N)?;
    let nur_ziffern =
        (1..=3).contains(&text.len()) && text.bytes().all(|byte| byte.is_ascii_digit());
    if !nur_ziffern {
        return Err(Abfragefehler::N);
    }
    text.parse()
        .ok()
        .and_then(Blockgroesse::bilden)
        .ok_or(Abfragefehler::N)
}

/// Formular-Kodierung: `+` ist ein Leerzeichen, `%XX` ein Byte; eine kaputte Sequenz oder kein UTF-8 heißt
/// unlesbar.
pub(super) fn dekodieren(text: &str) -> Option<String> {
    let mut bytes = Vec::with_capacity(text.len());
    let mut rest = text.as_bytes();
    while let Some((&erstes, danach)) = rest.split_first() {
        rest = danach;
        match erstes {
            b'+' => bytes.push(b' '),
            b'%' => {
                let hex = rest
                    .get(..2)
                    .filter(|hex| hex.iter().all(u8::is_ascii_hexdigit))?;
                bytes.push(u8::from_str_radix(std::str::from_utf8(hex).ok()?, 16).ok()?);
                rest = &rest[2..];
            }
            anderes => bytes.push(anderes),
        }
    }
    String::from_utf8(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zeit(text: &str) -> Zeit {
        Zeit::parse_from_rfc3339(text).expect("Testzeit")
    }

    fn gelesen(query: &str) -> Result<(Abschnitt, usize), Abfragefehler> {
        abfrage_lesen(Some(query)).map(|abfrage| (abfrage.abschnitt, abfrage.n.wert()))
    }

    /// K7 (ADR-0039 Regel 4): eine Werkbank mit genau 1 Erlaubnis, die der Test hält — der Verlauf antwortet
    /// nicht innerhalb 1 s; nach der Freigabe kommt 200 innerhalb 1 s (Startpunkt: Freigabe).
    #[tokio::test]
    async fn k7_der_verlauf_wartet_auf_eine_erlaubnis_der_werkbank() {
        use super::super::Werkbank;
        use super::super::testserver;
        use crate::anwesenheit::Beitrittswunsch;
        use crate::gespraeche::Kennung;
        use std::path::Path;
        use std::time::{Duration, Instant};
        let mut zustand = testserver::zustand(Path::new("/tmp/lc-werkbank"));
        zustand.werkbank = Werkbank::mit(1);
        let wunsch = Beitrittswunsch::pruefen(&json!({"name": "claude"})).expect("Wunsch");
        let token = zustand
            .anwesenheit
            .beitreten(&wunsch, "127.0.0.1".parse().expect("IP"))
            .expect("beigetreten")
            .sitzung
            .token
            .als_text()
            .to_string();
        let von = Kennung::bilden("claude@127.0.0.1").expect("Kennung");
        zustand
            .gespraeche
            .channel_anlegen("allgemein", None, &von)
            .expect("angelegt");
        let gehalten = zustand.werkbank.erwerben().await.expect("Erlaubnis");
        let basis = testserver::app_starten(super::super::router(zustand)).await;
        let anfrage = reqwest::Client::new()
            .get(format!("{basis}api/channels/allgemein/nachrichten"))
            .header("X-LocalchatA-Sitzung", token)
            .send();
        let mut anfrage = std::pin::pin!(anfrage);

        let solange_gehalten = tokio::time::timeout(Duration::from_secs(1), &mut anfrage).await;
        assert!(
            solange_gehalten.is_err(),
            "Antwort trotz gehaltener Erlaubnis"
        );
        drop(gehalten);
        let freigabe = Instant::now();
        let antwort = tokio::time::timeout(Duration::from_secs(10), anfrage)
            .await
            .expect("Antwort nach der Freigabe")
            .expect("Antwort");
        let dauer = freigabe.elapsed();

        assert_eq!(antwort.status(), 200);
        assert!(dauer <= Duration::from_secs(1), "{dauer:?}");
    }

    #[tokio::test]
    async fn ein_abgebrochener_verlauf_ist_500_mit_grund() {
        let zustand = crate::http::testserver::zustand(std::path::Path::new("/tmp"));
        let antwort = nicht_gelesen(&zustand, "beta@127.0.0.1", "allgemein", "Probe");

        assert_eq!(antwort.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let koerper = axum::body::to_bytes(antwort.into_body(), 1024)
            .await
            .expect("Körper");
        let json: serde_json::Value = serde_json::from_slice(&koerper).expect("JSON");
        assert_eq!(
            json["fehler"],
            "Interner Fehler — Verlauf nicht gelesen; bitte erneut versuchen"
        );
    }

    /// R00018 F0059 (ADR-0042 Ergänzung #24 Punkt 4; ersetzt Code-Review #24 B1, Abnahme-Probe 7): ein
    /// Serialisierungsfehler ist eine verletzte Annahme — genau eine EXCEPTION mit Ort, festem `grund` und dem
    /// Fremdtext in `ursache`; `exceptions` steigt, `errors` nicht; die Antwort nennt den Fremdtext nicht.
    #[tokio::test]
    async fn nicht_serialisierbar_ist_genau_eine_exception_mit_ursache_den_die_antwort_nicht_nennt()
    {
        use crate::protokoll::testhilfe::im_speicher;
        let mut zustand = crate::http::testserver::zustand(std::path::Path::new("/tmp"));
        let (protokoll, speicher, _, _) =
            im_speicher("2026-10-28T12:00:00+01:00", Stufe::Information);
        zustand.protokoll = protokoll;

        let antwort = nicht_gelesen(
            &zustand,
            "beta@127.0.0.1",
            "allgemein",
            "key must be a string",
        );

        assert_eq!(antwort.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            speicher.zeilen(),
            [
                "2026-10-28T12:00:00.000+01:00 EXCEPTION verlauf Verlauf nicht gelesen — Block ließ sich nicht serialisieren | route=\"GET /api/channels/{name}/nachrichten\" operation=verlauf status=500 channel=allgemein kennung=beta@127.0.0.1 grund=serialisierung ursache=\"key must be a string\""
            ]
        );
        let zaehler = zustand.protokoll.zaehlerstand();
        assert_eq!((zaehler.exceptions, zaehler.errors), (1, 0));
        let koerper = axum::body::to_bytes(antwort.into_body(), 1024)
            .await
            .expect("Körper");
        let json: serde_json::Value = serde_json::from_slice(&koerper).expect("JSON");
        assert_eq!(
            json["fehler"],
            "Interner Fehler — Verlauf nicht gelesen; bitte erneut versuchen"
        );
    }

    /// B0210: Verlauf, Antwort und Ereignis serialisieren dieselbe Form `{zeit, von, art, text}`; ohne Metazeile
    /// ist `art` `null`.
    #[test]
    fn die_eine_form_einer_nachricht_traegt_die_art_ohne_metazeile_null() {
        use crate::gespraeche::Kennung;
        let mut nachricht = Nachricht {
            zeit: crate::uhr::Zeit::parse_from_rfc3339("2026-10-28T12:00:00.001+01:00")
                .expect("Zeit"),
            von: Kennung::bilden("beta@127.0.0.1").expect("Kennung"),
            art: Some(Art::Agent),
            text: "Hallo\nWelt".into(),
            anhaenge: Vec::new(),
            geaendert: None,
            bezug: None,
            antwort_auf: None,
        };

        let agent = serde_json::to_value(NachrichtAntwort::aus(&nachricht)).expect("JSON");
        nachricht.art = None;
        let ohne = serde_json::to_value(NachrichtAntwort::aus(&nachricht)).expect("JSON");

        assert_eq!(
            agent,
            json!({"zeit": "2026-10-28T12:00:00.001+01:00", "von": "beta@127.0.0.1", "art": "agent", "text": "Hallo\nWelt", "anhaenge": [], "zustand": null, "geaendert": null, "antwort_auf": null})
        );
        assert_eq!(ohne["art"], serde_json::Value::Null);
    }

    /// R00039 B0597 (ADR-0048 Regel 4): eine geänderte Nachricht trägt `zustand` und `geaendert`; nur Felder kommen
    /// hinzu.
    #[test]
    fn eine_geaenderte_nachricht_traegt_zustand_und_geaendert() {
        use crate::gespraeche::{Aenderungszustand, Geaendert, Kennung};
        let zeit = |text| crate::uhr::Zeit::parse_from_rfc3339(text).expect("Zeit");
        let mut nachricht = Nachricht {
            zeit: zeit("2026-10-28T12:00:00.001+01:00"),
            von: Kennung::bilden("beta@127.0.0.1").expect("Kennung"),
            art: Some(Art::Agent),
            text: "Hallo zusammen".into(),
            anhaenge: Vec::new(),
            geaendert: Some(Geaendert {
                zeit: zeit("2026-10-28T12:05:00.000+01:00"),
                zustand: Aenderungszustand::Bearbeitet,
            }),
            bezug: None,
            antwort_auf: None,
        };

        let bearbeitet = serde_json::to_value(NachrichtAntwort::aus(&nachricht)).expect("JSON");
        nachricht.geaendert = Some(Geaendert {
            zeit: zeit("2026-10-28T12:06:00.000+01:00"),
            zustand: Aenderungszustand::Geloescht,
        });
        let geloescht = serde_json::to_value(NachrichtAntwort::aus(&nachricht)).expect("JSON");

        assert_eq!(
            bearbeitet,
            json!({"zeit": "2026-10-28T12:00:00.001+01:00", "von": "beta@127.0.0.1", "art": "agent", "text": "Hallo zusammen", "anhaenge": [], "zustand": "bearbeitet", "geaendert": "2026-10-28T12:05:00.000+01:00", "antwort_auf": null})
        );
        assert_eq!(geloescht["zustand"], "geloescht");
        assert_eq!(geloescht["geaendert"], "2026-10-28T12:06:00.000+01:00");
    }

    #[test]
    fn das_budget_je_block_ist_256_kib() {
        assert_eq!(VERLAUF_BUDGET, 262_144);
    }

    const VIERZEHN_UHR: &str = "2026-10-28T14:00:00.000+01:00";

    #[test]
    fn ohne_abfrage_die_neuesten_hundert() {
        assert_eq!(
            abfrage_lesen(None).map(|abfrage| (abfrage.abschnitt, abfrage.n.wert())),
            Ok((Abschnitt::Neueste, 100))
        );
        assert_eq!(gelesen(""), Ok((Abschnitt::Neueste, 100)));
        assert_eq!(gelesen("farbe=rot"), Ok((Abschnitt::Neueste, 100)));
    }

    #[test]
    fn n_als_spanne_eins_und_zweihundert_gehen() {
        assert_eq!(gelesen("n=1"), Ok((Abschnitt::Neueste, 1)));
        assert_eq!(gelesen("n=200"), Ok((Abschnitt::Neueste, 200)));
    }

    #[test]
    fn n_ausserhalb_oder_keine_ganze_zahl_ist_regel_n() {
        for query in [
            "n=0", "n=201", "n=abc", "n=1.5", "n=-1", "n=", "n=%2B1", "n=+1", "n=1&n=2", "n=0100",
            "n=1e2",
        ] {
            assert_eq!(gelesen(query), Err(Abfragefehler::N), "{query}");
        }
    }

    #[test]
    fn cursor_kodiert_wird_als_zeitpunkt_gelesen() {
        assert_eq!(
            gelesen("vor=2026-10-28T14:00:00.000%2B01:00"),
            Ok((Abschnitt::Vor(zeit(VIERZEHN_UHR)), 100))
        );
        assert_eq!(
            gelesen("nach=2026-10-28T14:00:00.000%2B01:00&n=7"),
            Ok((Abschnitt::Nach(zeit(VIERZEHN_UHR)), 7))
        );
        assert_eq!(
            gelesen("n=7&vor=2026-10-28T13:00:00.000Z"),
            Ok((Abschnitt::Vor(zeit("2026-10-28T14:00:00.000+01:00")), 7))
        );
        assert_eq!(
            gelesen("%76or=2026-10-28T14%3A00%3A00.000%2B01%3A00"),
            Ok((Abschnitt::Vor(zeit(VIERZEHN_UHR)), 100))
        );
    }

    #[test]
    fn unkodiertes_plus_zwei_cursor_und_unlesbares_sind_regel_cursor() {
        for query in [
            "vor=2026-10-28T14:00:00.000+01:00",
            "vor=2026-10-28T14:00:00.000%2B01:00&nach=2026-10-28T14:00:00.000%2B01:00",
            "vor=2026-10-28T14:00:00.000%2B01:00&vor=2026-10-28T14:00:00.000%2B01:00",
            "vor=gestern",
            "nach=",
            "vor=2026-10-28T14:00:00.000",
            "vor=2026-10-28T14:00:00.000%2",
            "vor=%FF",
            "vor=%+1",
        ] {
            assert_eq!(gelesen(query), Err(Abfragefehler::Cursor), "{query}");
        }
    }

    #[test]
    fn eine_zu_lange_abfrage_wird_nicht_gelesen() {
        let lang = format!("farbe={}&n=5", "x".repeat(300));

        assert_eq!(gelesen(&lang), Err(Abfragefehler::Cursor));
        assert_eq!(
            gelesen(&format!("farbe={}&n=5", "x".repeat(200))),
            Ok((Abschnitt::Neueste, 5))
        );
    }
}
