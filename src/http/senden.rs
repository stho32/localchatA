//! Nachricht senden (F0041; ADR-0034): `POST /api/channels/{name}/nachrichten {text}` mit Sitzung → 201
//! `{channel, nachricht}`. Prüfreihenfolge (abgewiesen, nie umgeformt): Grenzschutz 403 · Sitzung 401 · Vorab-Buchung
//! 429 · Körper zu
//! groß 413 · kein JSON-Typ 415 · kein JSON 400 · Channel 404 · Sendekörper 422 `text`/`feld` · Textregel 422
//! `leer`/`laenge`/`zeichen` · Wiederholschlüssel 422 `schluessel` · Restbuchung 429 · Plattenfehler 500. Mit bekanntem
//! `Idempotency-Key` (R00012 F0046, ADR-0034 Regel 6) → 200 mit der ursprünglichen Nachricht, ohne Satz und Ereignis. Der Körper wird auf der Werkbank geparst (Grenze über 16 KiB).

use super::Zustand;
use super::abbruch::{Abgebrochen, Grenzort, abbruch_beantworten};
use super::anfrageprotokoll::mit_regel;
use super::bearbeiten::{unbekannt_regel, unbekannt_wie};
use super::bremse::{Gegenstand, gebremst_melden};
use super::koerper::{
    Koerperfehler, Koerperort, json_lesen_auf_werkbank, koerperfehler_beantworten,
};
use super::nachrichten::NachrichtAntwort;
use super::teilnehmer::Teilnehmer;
use super::ziel::{Segment, Ziel, in_teilnehmerliste, ziel_aus_pfad};
use crate::gespraeche::anhangid::Anhangid;
use crate::gespraeche::textregel::{TEXT_HOECHSTENS_BYTES, Textfehler, text_pruefen};
use crate::gespraeche::{
    Antwortfehler, Gesendet, Gespraech, Grenzart, Grenze, Nachricht, Sendefehler, Sendung,
};
use crate::protokoll::episoden::{Auftreten, Quelle};
use crate::protokoll::grenze::{Gegriffen, grenzauftreten};
use crate::protokoll::vorlauf;
use crate::protokoll::{Bereich, Kontext, Schluessel, Stufe};
use crate::schreibkonto::eimer::kosten_senden;
use axum::Json;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Instant;

const ROUTE: &str = "POST /api/channels/{name}/nachrichten";

/// Körpergrenze des Sendens (ADR-0034 Regel 2): der größte gültige Körper in der längsten Schreibweise hat
/// 98.315 Bytes (`{"text":"` + 16.384 × `\u0061` + `"}`); mit 30 % Reserve 127.810, gerundet auf 128 KiB.
pub const SENDEN_GRENZE: usize = 131_072;

/// Die Kosten einer Nachricht stehen erst nach dem Lesen fest; der Vorab-Schritt rechnet mit der kleinsten (1),
/// R00011 und ADR-0036 Regel 3 unverändert (R00021 „Senden unverändert“).
const VORAB_SENDEN: u32 = 1;

/// `POST /api/channels/{name}/nachrichten`.
pub async fn senden(
    teilnehmer: Teilnehmer,
    State(zustand): State<Arc<Zustand>>,
    Path(name): Path<String>,
    kopf: HeaderMap,
    koerper: Body,
) -> Response {
    senden_in(teilnehmer, zustand, Ziel::Channel(name), kopf, koerper).await
}

/// `POST /api/direkt/{kennung}/nachrichten` (ADR-0055 Regel 3): die Sitzung vor dem Pfad (`Teilnehmer` zuerst), das
/// Segment roh, dann Kanonprüfung und `selbst`; ab da dieselbe Folge wie im Channel.
pub(super) async fn senden_direkt(
    teilnehmer: Teilnehmer,
    State(zustand): State<Arc<Zustand>>,
    segment: Segment,
    kopf: HeaderMap,
    koerper: Body,
) -> Response {
    match ziel_aus_pfad(&teilnehmer, &segment) {
        Ok(ziel) => senden_in(teilnehmer, zustand, ziel, kopf, koerper).await,
        Err(fehler) => fehler.antwort(&teilnehmer.kennung),
    }
}

async fn senden_in(
    teilnehmer: Teilnehmer,
    zustand: Arc<Zustand>,
    ziel: Ziel,
    kopf: HeaderMap,
    koerper: Body,
) -> Response {
    let eintritt = Instant::now();
    if let Err(gebremst) =
        zustand
            .schreibkonto
            .vorab(&teilnehmer.kennung, teilnehmer.herkunft, VORAB_SENDEN)
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
        route: ziel.route(ROUTE),
        herkunft: teilnehmer.herkunft,
        kennung: Some(&wer),
    };
    let wert = match json_lesen_auf_werkbank(&kopf, koerper, SENDEN_GRENZE, &ort, &zustand.werkbank)
        .await
    {
        Ok(Ok(wert)) => wert,
        Ok(Err(fehler)) => return koerper_abgewiesen(fehler),
        Err(abbruch) => return abgebrochen(&zustand, &abbruch, &wer, &ziel),
    };
    if let Ziel::Channel(name) = &ziel
        && !zustand.gespraeche.channel_bekannt(name)
    {
        return mit_regel(unbekannt(name), "channel_unbekannt", Some(&wer));
    }
    let (text, anhaenge) = match sendekoerper_lesen(&wert) {
        Ok((text, anhaenge)) => (text.to_string(), anhaenge),
        Err(fehler) => {
            let regel = match fehler {
                Sendekoerperfehler::Text => "text_fehlt",
                Sendekoerperfehler::Feld { .. } => "feld_unbekannt",
                Sendekoerperfehler::Anhaenge => "anhaenge",
                Sendekoerperfehler::Anhang { .. } => "anhang",
                Sendekoerperfehler::AntwortAuf => "antwort_auf_format",
            };
            return mit_regel(sendekoerper_abgewiesen(fehler), regel, Some(&wer));
        }
    };
    let antwort_auf = match antwort_auf_lesen(&wert) {
        Ok(antwort_auf) => antwort_auf,
        Err(fehler) => {
            return mit_regel(
                sendekoerper_abgewiesen(fehler),
                "antwort_auf_format",
                Some(&wer),
            );
        }
    };
    let text = match text_mit_anhaengen(&text, anhaenge.len()) {
        Ok(text) => text.to_string(),
        Err(fehler) => {
            let regel = match fehler {
                Textfehler::Leer => "text_leer",
                Textfehler::Laenge { .. } => "text_zu_lang",
                Textfehler::Zeichen { .. } => "text_zeichen",
            };
            return mit_regel(text_abgewiesen(fehler), regel, Some(&wer));
        }
    };
    let Ok(schluessel) = schluessel_lesen(&kopf) else {
        return mit_regel(
            schluessel_abgewiesen(SCHLUESSEL_FORMAT),
            "schluessel_ungueltig",
            Some(&wer),
        );
    };
    if let Ziel::Direkt { paar, gegenueber } = &ziel
        && !zustand.gespraeche.direkt_besteht(paar)
        && !in_teilnehmerliste(&zustand, gegenueber)
    {
        return mit_regel(empfaenger_unbekannt(), "empfaenger_unbekannt", Some(&wer));
    }
    let text_bytes = text.len();
    let kosten = kosten_senden(text.len());
    if let Err(gebremst) =
        zustand
            .schreibkonto
            .rest(&teilnehmer.kennung, teilnehmer.herkunft, kosten)
    {
        return gebremst_melden(
            &zustand.protokoll,
            gebremst,
            Gegenstand::Nachrichten,
            &teilnehmer,
        );
    }
    let gespraeche = Arc::clone(&zustand.gespraeche);
    let senden_an = ziel.clone();
    let von = teilnehmer.kennung.clone();
    let art = teilnehmer.art;
    let ergebnis = tokio::task::spawn_blocking(move || {
        let sendung = Sendung {
            text: &text,
            anhaenge: &anhaenge,
            schluessel: schluessel.as_deref(),
            antwort_auf,
        };
        match &senden_an {
            Ziel::Channel(name) => {
                gespraeche.senden_sendung(&Gespraech::channel(name), &von, art, sendung)
            }
            Ziel::Direkt { paar, .. } => gespraeche.direkt_senden(paar, &von, art, sendung),
        }
    })
    .await;
    match ergebnis {
        Ok(Ok(Gesendet::Neu(nachricht))) => {
            // Nach dem Veröffentlichen der Nachricht, außerhalb der Schreibsperre (ADR-0051 Regel 7 b).
            zustand
                .aktivitaet
                .beenden(&teilnehmer.kennung, &ziel.gespraech());
            vorlauf::eintragen(&zustand.protokoll, zustand.gespraeche.zeit_vorlauf_ms());
            gesendet_melden(&zustand, &ziel, &wer, text_bytes, eintritt, None);
            gesendet(StatusCode::CREATED, &ziel, &nachricht)
        }
        Ok(Ok(Gesendet::Wiederholt(nachricht))) => {
            gesendet_melden(
                &zustand,
                &ziel,
                &wer,
                text_bytes,
                eintritt,
                Some("wiederholung"),
            );
            gesendet(StatusCode::OK, &ziel, &nachricht)
        }
        Ok(Err(Sendefehler::SchluesselAnders)) => mit_regel(
            schluessel_abgewiesen(SCHLUESSEL_ANDERS),
            "schluessel_anders",
            Some(&wer),
        ),
        // Ein Direktgespräch, das zwischen Bereitstellen und Senden verschwand, antwortet wie eine nie vergebene
        // Nachricht — ohne Ordnername und ohne Channel-Regel (ADR-0048 M3).
        Ok(Err(Sendefehler::Unbekannt)) => {
            mit_regel(unbekannt_wie(&ziel), unbekannt_regel(&ziel), Some(&wer))
        }
        Ok(Err(Sendefehler::DirektGrenze(grenze))) => {
            direkt_grenze_melden(&zustand, teilnehmer.herkunft, grenze);
            mit_regel(direkt_grenze_erreicht(grenze), "direkt_grenze", Some(&wer))
        }
        Ok(Err(Sendefehler::Ablage { datei, grund })) => {
            nicht_gespeichert(&zustand, &ziel, &wer, &datei, &grund)
        }
        Ok(Err(Sendefehler::Quote { belegt, hoechstens })) => {
            quote_melden(&zustand, &ziel, &wer, belegt, hoechstens);
            mit_regel(quote_erreicht(belegt, hoechstens), "quote", Some(&wer))
        }
        Ok(Err(Sendefehler::Anhang { id, grund })) => mit_regel(
            sendekoerper_abgewiesen(Sendekoerperfehler::Anhang {
                id: id.als_text().to_string(),
                grund: grund.als_text(),
            }),
            "anhang",
            Some(&wer),
        ),
        Ok(Err(Sendefehler::Antwort(fehler))) => {
            let regel = match fehler {
                Antwortfehler::Unbekannt => "antwort_auf_unbekannt",
                Antwortfehler::Geloescht => "antwort_auf_geloescht",
            };
            mit_regel(antwort_abgewiesen(fehler), regel, Some(&wer))
        }
        Ok(Err(Sendefehler::Binden {
            tag,
            datei,
            grund,
            waisen,
        })) => {
            nicht_gebunden(&zustand, &ziel, &wer, tag, &datei, &grund, &waisen);
            nicht_gespeichert_antwort()
        }
        Err(abbruch) => abgebrochen(&zustand, &Abgebrochen::from(abbruch), &wer, &ziel),
    }
}

/// 404 „# name gibt es nicht.“ — auch für ungültige Namen (R00011 F0041); der Name steht nur im JSON-Text.
pub(super) fn unbekannt(name: &str) -> Response {
    let fehler = format!("# {name} gibt es nicht.");
    (StatusCode::NOT_FOUND, Json(json!({ "fehler": fehler }))).into_response()
}

/// `DEBUG senden Nachricht gesendet` (R00022 F0073): Channel, Kennung, Bytes des Texts als `anzahl` und die Zeit vom
/// Eintritt in den Handler bis zum Schreiben; bei einer Wiederholung mit bekanntem Schlüssel `grund=wiederholung` —
/// nie der Text.
fn gesendet_melden(
    zustand: &Zustand,
    ziel: &Ziel,
    wer: &str,
    text_bytes: usize,
    eintritt: Instant,
    grund: Option<&str>,
) {
    let kontext = Kontext::neu()
        .mit(ziel.bezug(), ziel.name())
        .mit(Schluessel::Kennung, wer)
        .mit(Schluessel::Anzahl, text_bytes)
        .mit(Schluessel::DauerMs, eintritt.elapsed().as_millis());
    let kontext = match grund {
        Some(grund) => kontext.mit(Schluessel::Grund, grund),
        None => kontext,
    };
    zustand
        .protokoll
        .eintragen(Stufe::Debug, Bereich::Senden, "Nachricht gesendet", kontext);
}

fn gesendet(status: StatusCode, ziel: &Ziel, nachricht: &Nachricht) -> Response {
    let antwort = ziel.antwort_mit(json!(NachrichtAntwort::aus(nachricht)));
    (status, Json(antwort)).into_response()
}

/// Jede Abweisung an der Grenze zählt in eine Episode `WARNING direkt` je Herkunft (ADR-0055 Regel 11, Muster
/// `channels`): Herkunft, Grenze und Zahl — nie die Kennung des Gegenübers, nie ein Ordner.
fn direkt_grenze_melden(zustand: &Zustand, herkunft: std::net::IpAddr, grenze: Grenze) {
    let art = match grenze.art {
        Grenzart::Gesamt => crate::protokoll::grenze::Grenzart::Gesamt,
        Grenzart::Herkunft => crate::protokoll::grenze::Grenzart::Herkunft,
    };
    zustand.protokoll.episode(grenzauftreten(
        Bereich::Direkt,
        crate::protokoll::episodentexte::DIREKT_GRENZE,
        herkunft,
        Gegriffen {
            art,
            hoechstens: grenze.hoechstens,
        },
    ));
}

/// 429 an der Grenze neuer Direktgespräche (ADR-0055 Regel 4, E16): `{fehler, grenze: "direktgespraeche", art,
/// hoechstens}` wie die Channel-Grenze; der Satz nennt weder ein Gegenüber noch einen Ordner.
fn direkt_grenze_erreicht(grenze: Grenze) -> Response {
    let (art, wessen) = match grenze.art {
        Grenzart::Gesamt => ("gesamt", "insgesamt"),
        Grenzart::Herkunft => ("herkunft", "je Rechner"),
    };
    let text = format!(
        "Zu viele neue Direktgespräche: höchstens {} {wessen} — bestehende Gespräche nutzen; Platz entsteht, wenn \
         ein Gespräch nach seiner Frist entfällt.",
        grenze.hoechstens
    );
    (
        StatusCode::TOO_MANY_REQUESTS,
        Json(json!({
            "fehler": text,
            "grenze": "direktgespraeche",
            "art": art,
            "hoechstens": grenze.hoechstens,
        })),
    )
        .into_response()
}

/// 404 `empfaenger` (ADR-0055 Regel 3, PO 170): ein neues Gespräch beginnt nur mit jemandem aus der Teilnehmerliste;
/// dieselbe Antwort für einen, den es nie gab, und einen, der gegangen ist — und kein Ordner.
fn empfaenger_unbekannt() -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(json!({
            "fehler": "Diesen Teilnehmer gibt es gerade nicht — ein neues Gespräch beginnt nur mit jemandem aus der Teilnehmerliste.",
            "regel": "empfaenger",
        })),
    )
        .into_response()
}

pub(super) const SCHLUESSEL_FORMAT: &str =
    "Idempotency-Key: 1 bis 64 Zeichen aus A–Z, a–z, 0–9, _ und - — zum Beispiel 7f3a-k1.";
pub(super) const SCHLUESSEL_ANDERS: &str = "Dieser Schlüssel gehört zu einer anderen Nachricht.";

pub(super) fn schluessel_abgewiesen(fehler: &str) -> Response {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(json!({ "fehler": fehler, "regel": "schluessel" })),
    )
        .into_response()
}

/// Der Kopf `Idempotency-Key` (ADR-0034 Regel 3.7): fehlt er, kein Schlüssel; genau einmal mit 1–64 Zeichen aus
/// `[A-Za-z0-9_-]` → der Schlüssel; sonst (auch mehrfach) abgewiesen, nie umgeformt.
pub(super) fn schluessel_lesen(kopf: &HeaderMap) -> Result<Option<String>, ()> {
    let mut werte = kopf.get_all("idempotency-key").iter();
    let Some(wert) = werte.next() else {
        return Ok(None);
    };
    if werte.next().is_some() {
        return Err(());
    }
    let bytes = wert.as_bytes();
    let erlaubt = |byte: &u8| byte.is_ascii_alphanumeric() || *byte == b'_' || *byte == b'-';
    if bytes.is_empty() || bytes.len() > 64 || !bytes.iter().all(erlaubt) {
        return Err(());
    }
    Ok(Some(String::from_utf8_lossy(bytes).into_owned()))
}

/// 500 mit deutschem Satz; der Schreibfehler zählt in eine Episode `ERROR senden` je Datei und Grund (R00016), ab
/// ERROR auch auf stderr — nie der Nachrichtentext.
fn nicht_gespeichert(
    zustand: &Zustand,
    ziel: &Ziel,
    wer: &str,
    datei: &str,
    grund: &str,
) -> Response {
    zustand
        .protokoll
        .episode(schreibfehler_auftreten(ziel, wer, datei, grund));
    let fehler = "Nicht gespeichert — der Server konnte nicht schreiben.";
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "fehler": fehler })),
    )
        .into_response()
}

/// Das Auftreten eines Schreibfehlers: die Quelle ist Datei und Grund, die Kennung gehört dem einzelnen Auftreten.
pub(crate) fn schreibfehler_auftreten(
    ziel: &Ziel,
    wer: &str,
    datei: &str,
    grund: &str,
) -> Auftreten {
    Auftreten::neu(
        Stufe::Error,
        Bereich::Senden,
        crate::protokoll::episodentexte::NICHT_GESPEICHERT,
        Quelle::neu()
            .mit(ziel.bezug(), ziel.name())
            .mit(Schluessel::Datei, datei)
            .mit(Schluessel::Grund, grund),
    )
    .einzeln(Schluessel::Kennung, wer)
}

/// 500 nach gescheitertem Binden (ADR-0046 Regel 6): `ERROR senden` mit Gespräch, Tag und Datei nur als Präfix;
/// je Waise eine weitere Zeile — nie Anhang-ID oder Anzeigename (M19).
fn nicht_gebunden(
    zustand: &Zustand,
    ziel: &Ziel,
    wer: &str,
    tag: chrono::NaiveDate,
    datei: &str,
    grund: &str,
    waisen: &[String],
) {
    zustand.protokoll.eintragen(
        Stufe::Error,
        Bereich::Senden,
        "Anhang nicht gebunden — die Nachricht ist nicht gesendet, die Anhänge liegen wieder im Eingang",
        Kontext::neu()
            .mit(ziel.bezug(), ziel.name())
            .mit(Schluessel::Kennung, wer)
            .mit(Schluessel::Tag, tag.format("%Y-%m-%d"))
            .mit(Schluessel::Datei, datei)
            .mit(Schluessel::Grund, grund),
    );
    for waise in waisen {
        zustand.protokoll.eintragen(
            Stufe::Error,
            Bereich::Senden,
            "Zurücklegen gescheitert — die Datei bleibt als Waise im Tag und zählt an der Anhang-Quote",
            Kontext::neu()
                .mit(ziel.bezug(), ziel.name())
                .mit(Schluessel::Tag, tag.format("%Y-%m-%d"))
                .mit(Schluessel::Datei, waise.as_str()),
        );
    }
}

pub(super) fn nicht_gespeichert_antwort() -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "fehler": "Nicht gespeichert — der Server konnte nicht schreiben." })),
    )
        .into_response()
}

/// Jede 507 der Quote zählt in eine Episode `WARNING quote` je Kennung (ADR-0044 Regel 8); die einzelne Abweisung
/// trägt die Schicht des Anfrageprotokolls als DEBUG ein.
fn quote_melden(zustand: &Zustand, ziel: &Ziel, wer: &str, belegt: u64, hoechstens: u64) {
    zustand
        .protokoll
        .episode(quote_auftreten(ziel, wer, belegt, hoechstens));
}

/// Quelle der Quote-Episode ist die Kennung (mit der Grenze); Channel und Belegung sind Werte der einzelnen
/// Abweisung und stehen nur in der Beginn-Zeile (ADR-0044 Ergänzung Karte #66).
pub(crate) fn quote_auftreten(ziel: &Ziel, wer: &str, belegt: u64, hoechstens: u64) -> Auftreten {
    Auftreten::neu(
        Stufe::Warning,
        Bereich::Quote,
        crate::protokoll::episodentexte::QUOTE_ERREICHT,
        Quelle::neu()
            .mit(Schluessel::Kennung, wer)
            .mit(Schluessel::Hoechstens, hoechstens),
    )
    .einzeln(ziel.bezug(), ziel.name())
    .einzeln(Schluessel::Belegt, belegt)
}

/// Bytes in Worten für den 507-Text.
/// Bytes in Worten für den 507-Text (R00023 F0075): ab 1 GiB in GiB, sonst in MiB, eine Nachkommastelle, halbe
/// nach oben, ganzzahlig gerechnet; deutsches Komma, Tausenderpunkt.
pub(super) fn groesse_in_worten(bytes: u64) -> String {
    const MIB: u128 = 1024 * 1024;
    const GIB: u128 = 1024 * MIB;
    let (einheit, name) = if u128::from(bytes) >= GIB {
        (GIB, "GiB")
    } else {
        (MIB, "MiB")
    };
    let zehntel = (u128::from(bytes) * 20 + einheit) / (2 * einheit);
    let ganz = usize::try_from(zehntel / 10).unwrap_or(usize::MAX);
    format!("{},{} {name}", mit_tausenderpunkt(ganz), zehntel % 10)
}

/// 507 an der Plattenquote (ADR-0044 Regel 4), ohne `Retry-After`.
fn quote_erreicht(belegt: u64, hoechstens: u64) -> Response {
    let fehler = format!(
        "Nicht gesendet: Die Gespräche belegen {}, die Quote dieser Instanz ist {}. Platz entsteht, wenn alte \
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

fn abgebrochen(zustand: &Zustand, abbruch: &Abgebrochen, wer: &str, ziel: &Ziel) -> Response {
    abbruch_beantworten(
        &zustand.protokoll,
        abbruch,
        &Grenzort {
            route: ziel.route(ROUTE),
            operation: "senden",
            kennung: Some(wer),
            ziel: Some(ziel),
        },
        "Nachricht nicht gesendet",
    )
}

const BEISPIEL: &str = r#"{"text":"Hallo"}"#;

/// Die eine Antwort für Körperfehler (`koerper::koerperfehler_beantworten`) mit dem vollständigen 413-Satz und
/// dem Beispiel der Route; die 413 schreibt die Grenze wie die Oberfläche, ohne Beispiel (R00020 F0069).
fn koerper_abgewiesen(fehler: Koerperfehler) -> Response {
    koerperfehler_beantworten(
        fehler,
        &format!(
            "Der Körper ist zu groß — höchstens {} Byte; längere Texte teilen.",
            mit_tausenderpunkt(SENDEN_GRENZE)
        ),
        BEISPIEL,
    )
}

fn sendekoerper_abgewiesen(fehler: Sendekoerperfehler) -> Response {
    let antwort = match fehler {
        Sendekoerperfehler::Text => json!({
            "fehler": "Der Körper braucht text als Zeichenkette.",
            "regel": "text",
        }),
        Sendekoerperfehler::Feld { feld } => json!({
            "fehler": format!("Unbekanntes Feld {feld} — erlaubt sind text, anhaenge und antwort_auf."),
            "regel": "feld",
            "feld": feld,
        }),
        Sendekoerperfehler::Anhaenge => json!({
            "fehler": format!("anhaenge braucht eine Liste von höchstens {ANHAENGE_HOECHSTENS} Anhang-IDs."),
            "regel": "anhaenge",
            "hoechstens": ANHAENGE_HOECHSTENS,
        }),
        Sendekoerperfehler::Anhang { id, grund } => json!({
            "fehler": anhangtext(&id, grund),
            "regel": "anhang",
            "id": id,
            "grund": grund,
        }),
        Sendekoerperfehler::AntwortAuf => json!({
            "fehler": ANTWORT_AUF_FORMAT,
            "regel": "antwort_auf",
            "grund": "format",
        }),
    };
    (StatusCode::UNPROCESSABLE_ENTITY, Json(antwort)).into_response()
}

/// Texte der Abweisung eines Bezugs (ADR-0053 Regel 2), wörtlich.
const ANTWORT_AUF_FORMAT: &str = "antwort_auf muss die Zeit einer Nachricht sein, wie sie im Verlauf steht (RFC 3339, z. B. 2026-10-30T09:10:03.004+01:00).";
const ANTWORT_AUF_UNBEKANNT: &str =
    "Die Nachricht, auf die du antwortest, gibt es in diesem Gespräch nicht (mehr).";
const ANTWORT_AUF_GELOESCHT: &str = "Die Nachricht, auf die du antwortest, ist gelöscht.";

/// 422 `{fehler, regel: "antwort_auf", grund}` für einen Bezug, der unter der Sperre nicht gilt; eine Zeit aus einem
/// anderen Gespräch antwortet wie eine nie dagewesene (ADR-0053 Regel 10).
fn antwort_abgewiesen(fehler: Antwortfehler) -> Response {
    let (text, grund) = match fehler {
        Antwortfehler::Unbekannt => (ANTWORT_AUF_UNBEKANNT, "unbekannt"),
        Antwortfehler::Geloescht => (ANTWORT_AUF_GELOESCHT, "geloescht"),
    };
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(json!({ "fehler": text, "regel": "antwort_auf", "grund": grund })),
    )
        .into_response()
}

/// Der Satz zu einem Anhang, der nicht gesendet werden kann.
pub(super) fn anhangtext(id: &str, grund: &str) -> String {
    match grund {
        "doppelt" => format!("Der Anhang {id} steht zweimal in der Liste."),
        "abgelaufen" => format!("Der Anhang {id} ist abgelaufen — neu anmelden und hochladen."),
        "fremd" => format!("Der Anhang {id} gehört einem anderen Teilnehmer."),
        "unvollstaendig" => format!("Der Anhang {id} ist noch nicht vollständig hochgeladen."),
        _ => format!("Den Anhang {id} gibt es nicht — erst anmelden und hochladen."),
    }
}

/// Texte der Abweisung, die der Browser wörtlich zeigt (R00011 F0041); die Stelle für Menschen ab 1.
pub(super) fn text_abgewiesen(fehler: Textfehler) -> Response {
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

/// `17020` → `17.020`.
pub(super) fn mit_tausenderpunkt(zahl: usize) -> String {
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

/// Höchstens so viele Anhänge trägt eine Nachricht (ADR-0046 Frage 3).
pub const ANHAENGE_HOECHSTENS: usize = 10;

/// Warum ein Sendekörper keinen Auftrag liefert (422 `text`, `feld`, `anhaenge` bzw. `anhang`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sendekoerperfehler {
    /// Kein Objekt, `text` fehlt oder ist keine Zeichenkette.
    Text,
    /// Ein weiteres Feld neben `text` und `anhaenge`.
    Feld { feld: String },
    /// `anhaenge` ist keine Liste oder hat mehr als 10 Einträge.
    Anhaenge,
    /// Ein Eintrag ist keine ID (`unbekannt`) oder steht zweimal (`doppelt`).
    Anhang { id: String, grund: &'static str },
    /// `antwort_auf` ist weder `null` noch eine Zeit nach RFC 3339 (ADR-0053 Regel 2).
    AntwortAuf,
}

/// Text und Anhänge aus dem Körper `{"text": "…", "anhaenge"?: ["<id>", …]}` (B0439, ADR-0034 Regel 3.3 mit
/// Ergänzung Karte #9, Reihenfolge nach Karte #47): erst jedes weitere Feld, dann der Typ von `text`; `anhaenge` höchstens 10, keine doppelt.
pub fn sendekoerper_lesen(wert: &Value) -> Result<(&str, Vec<Anhangid>), Sendekoerperfehler> {
    let objekt = wert.as_object().ok_or(Sendekoerperfehler::Text)?;
    if let Some(feld) = objekt
        .keys()
        .find(|feld| !["text", "anhaenge", "antwort_auf"].contains(&feld.as_str()))
    {
        return Err(Sendekoerperfehler::Feld { feld: feld.clone() });
    }
    let text = objekt
        .get("text")
        .and_then(Value::as_str)
        .ok_or(Sendekoerperfehler::Text)?;
    let liste = match objekt.get("anhaenge") {
        None => return Ok((text, Vec::new())),
        Some(Value::Array(liste)) if liste.len() <= ANHAENGE_HOECHSTENS => liste,
        Some(_) => return Err(Sendekoerperfehler::Anhaenge),
    };
    let mut ids: Vec<Anhangid> = Vec::with_capacity(liste.len());
    for eintrag in liste {
        let text_der_id = eintrag
            .as_str()
            .map_or_else(|| eintrag.to_string(), str::to_string);
        let id = eintrag.as_str().and_then(Anhangid::lesen).ok_or_else(|| {
            Sendekoerperfehler::Anhang {
                id: text_der_id.clone(),
                grund: "unbekannt",
            }
        })?;
        if ids.contains(&id) {
            return Err(Sendekoerperfehler::Anhang {
                id: text_der_id,
                grund: "doppelt",
            });
        }
        ids.push(id);
    }
    Ok((text, ids))
}

/// Der Bezug aus dem Körper (ADR-0053 Regel 2, vor der Sperre): fehlt `antwort_auf` oder ist es `null`, keiner; eine
/// Zeichenkette in RFC 3339 ist die Zeit der Nachricht, auf die geantwortet wird (verglichen als Zeitpunkt); alles
/// andere ist `AntwortAuf`.
pub fn antwort_auf_lesen(wert: &Value) -> Result<Option<crate::uhr::Zeit>, Sendekoerperfehler> {
    match wert.get("antwort_auf") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(zeit)) => crate::uhr::Zeit::parse_from_rfc3339(zeit)
            .map(Some)
            .map_err(|_| Sendekoerperfehler::AntwortAuf),
        Some(_) => Err(Sendekoerperfehler::AntwortAuf),
    }
}

/// Mit mindestens einem Anhang ist ein Text nur aus Leerraum erlaubt und wird als `""` gespeichert (Betreiber B5);
/// sonst gilt die Textregel unverändert.
pub fn text_mit_anhaengen(text: &str, anhaenge: usize) -> Result<&str, Textfehler> {
    if anhaenge > 0 && text.chars().all(char::is_whitespace) {
        return Ok("");
    }
    text_pruefen(text).map(|()| text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn allgemein() -> Ziel {
        Ziel::Channel("allgemein".to_string())
    }

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

        vorlauf::eintragen(&protokoll, 60_000);
        let bei_60_000 = zeit_zeilen().len();
        vorlauf::eintragen(&protokoll, 61_001);
        vorlauf::eintragen(&protokoll, 61_002);
        let nach_zweien = zeit_zeilen();
        uhr.stellen(zeit("2026-10-28T12:01:00+01:00"));
        protokoll.episoden_schliessen();
        let nach_der_ruhe = zeit_zeilen().len();
        vorlauf::eintragen(&protokoll, 7_410_002);

        assert_eq!(bei_60_000, 0);
        assert_eq!(nach_zweien.len(), 1, "{nach_zweien:#?}");
        assert!(
            nach_zweien[0].contains(
                "Nachrichtenzeit läuft der Systemuhr 0 h 01 min voraus (Uhr zurückgestellt?) — Zeiten bleiben streng steigend, bis die Uhr aufholt — Episode beginnt | "
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

    /// R00022 B0345 (ADR-0042 Ergänzung Karte #31 Punkt 7, erster Test): Schreibfehler-Episode mit zwei Kennungen →
    /// Beginn mit `kennung=` der ersten, Ende ohne `kennung=`, mit `datei=` und `channel=`. Mutante „Kontext des
    /// ersten Auftretens“ → rot.
    #[test]
    fn die_ende_zeile_des_schreibfehlers_nennt_die_datei_und_keine_kennung() {
        use crate::protokoll::Stufe;
        use crate::protokoll::testhilfe::{im_speicher, zeit};
        let (protokoll, speicher, uhr, _) =
            im_speicher("2026-10-28T12:00:00+01:00", Stufe::Information);
        let datei = "channels/allgemein/2026-10-28.log";
        protokoll.episode(schreibfehler_auftreten(
            &allgemein(),
            "beta@127.0.0.1",
            datei,
            "Kein Platz mehr",
        ));
        protokoll.episode(schreibfehler_auftreten(
            &allgemein(),
            "gamma@127.0.0.1",
            datei,
            "Kein Platz mehr",
        ));
        uhr.stellen(zeit("2026-10-28T12:01:00+01:00"));
        protokoll.episoden_schliessen();

        let zeilen: Vec<String> = speicher
            .zeilen()
            .into_iter()
            .filter(|zeile| zeile.contains(" ERROR senden "))
            .collect();
        assert_eq!(zeilen.len(), 2, "{zeilen:#?}");
        assert!(zeilen[0].contains("Episode beginnt"), "{zeilen:#?}");
        assert!(zeilen[0].contains("kennung=beta@127.0.0.1"), "{zeilen:#?}");
        assert!(zeilen[1].contains("Episode endet: 2 in "), "{zeilen:#?}");
        assert!(!zeilen[1].contains("kennung="), "{zeilen:#?}");
        assert!(zeilen[1].contains(&format!("datei={datei}")), "{zeilen:#?}");
        assert!(zeilen[1].contains("channel=allgemein"), "{zeilen:#?}");
    }

    /// Nacharbeit code-reviewer #13 F0172, Sollte 4 (wörtlich): verschwindet das eben bereitgestellte Direktgespräch
    /// zweimal (ein Ablage-Stumpf, dessen Verlauf weg ist), endet das Senden mit `Unbekannt` — die Antwort nennt
    /// weder `channel_unbekannt` noch den Ordnernamen, sie ist dieselbe wie für eine nie vergebene Nachricht (gleiche
    /// Wahl wie bei Bearbeiten, ADR-0048 M3).
    #[tokio::test]
    async fn ein_verschwundenes_direktgespraech_nennt_weder_channel_noch_ordner() {
        use crate::anwesenheit::Beitrittswunsch;
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let mut zustand = super::super::testserver::zustand(wurzel.path());
        zustand.gespraeche = Arc::new(
            crate::gespraeche::Gespraeche::laden(
                Arc::new(crate::gespraeche::Dateiablage::neu(
                    wurzel.path(),
                    Arc::new(crate::protokoll::Protokoll::ohne_ablage()),
                )),
                Arc::new(crate::uhr::Systemuhr),
            )
            .expect("geladen"),
        );
        let herkunft = "127.0.0.1".parse().expect("IP");
        let beitreten = |name: &str| {
            let wunsch = Beitrittswunsch::pruefen(&json!({"name": name})).expect("Wunsch");
            zustand
                .anwesenheit
                .beitreten(&wunsch, herkunft)
                .expect("beigetreten")
                .sitzung
                .token
                .als_text()
                .to_string()
        };
        let token = beitreten("anna");
        beitreten("ben");
        zustand.gespraeche.direkt_verschwinden_lassen(2);
        let basis = super::super::testserver::app_starten(super::super::router(zustand)).await;

        let antwort = reqwest::Client::new()
            .post(format!("{basis}api/direkt/ben@127.0.0.1/nachrichten"))
            .header("X-LocalchatA-Sitzung", token)
            .json(&json!({"text": "hallo"}))
            .send()
            .await
            .expect("HTTP");

        let status = antwort.status();
        let koerper: serde_json::Value = antwort.json().await.expect("JSON");
        assert_eq!(status, 404, "{koerper}");
        let text = koerper.to_string();
        assert!(!text.contains("channel_unbekannt"), "{text}");
        assert!(
            !text.contains("direkt/") && !text.contains("ben@") && !text.contains("anna@"),
            "{text}"
        );
        assert_eq!(koerper["regel"], "nachricht", "{text}");
    }

    /// R00022 B0345 (zweiter Test, wörtlich): `zeit` mit 61.001 ms, dann 7.410.002 ms → Ende mit
    /// `dauer_ms=7410002`; die Beginn-Zeile trägt 61001. Mutante „Ende mit dem ersten `dauer_ms`“ → rot.
    #[test]
    fn die_ende_zeile_des_vorlaufs_nennt_die_letzte_dauer() {
        use crate::protokoll::Stufe;
        use crate::protokoll::testhilfe::{im_speicher, zeit};
        let (protokoll, speicher, uhr, _) =
            im_speicher("2026-10-28T12:00:00+01:00", Stufe::Information);

        vorlauf::eintragen(&protokoll, 61_001);
        vorlauf::eintragen(&protokoll, 7_410_002);
        uhr.stellen(zeit("2026-10-28T12:01:00+01:00"));
        protokoll.episoden_schliessen();

        let zeilen: Vec<String> = speicher
            .zeilen()
            .into_iter()
            .filter(|zeile| zeile.contains(" WARNING zeit "))
            .collect();
        assert_eq!(zeilen.len(), 2, "{zeilen:#?}");
        assert!(zeilen[0].contains("dauer_ms=61001"), "{zeilen:#?}");
        assert!(zeilen[1].contains("Episode endet: 2 in "), "{zeilen:#?}");
        assert!(zeilen[1].ends_with("dauer_ms=7410002"), "{zeilen:#?}");
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

    /// R00023 F0075 „Größen in Worten“ (Rechenbeispiele): ab 1 GiB in GiB, sonst MiB, eine Nachkommastelle,
    /// halbe nach oben, deutsches Komma, Tausenderpunkt.
    #[test]
    fn groessen_in_worten() {
        for (bytes, text) in [
            (4_294_967_296, "4,0 GiB"),
            (3_865_470_567, "3,6 GiB"),
            (1_073_741_824, "1,0 GiB"),
            (1_073_741_823, "1.024,0 MiB"),
            (536_870_912, "512,0 MiB"),
            (1_048_576, "1,0 MiB"),
            (1_048_519, "1,0 MiB"),
            (1_101_005, "1,1 MiB"),
            (1_101_004, "1,0 MiB"),
            (1_099_511_627_776, "1.024,0 GiB"),
            (0, "0,0 MiB"),
        ] {
            assert_eq!(groesse_in_worten(bytes), text, "{bytes}");
        }
    }

    /// B0358 (ADR-0044 Regel 4): 507 ohne `Retry-After`, `regel: "quote"`, `belegt`, `hoechstens` und der Text mit
    /// Grund und Ausweg.
    #[tokio::test]
    async fn an_der_quote_ist_507_mit_grund_und_ausweg() {
        let antwort = teile(quote_erreicht(1_048_519, 1_048_576)).await;

        assert_eq!(
            antwort,
            (
                507,
                None,
                json!({
                    "fehler": "Nicht gesendet: Die Gespräche belegen 1,0 MiB, die Quote dieser Instanz ist 1,0 MiB. Platz entsteht, wenn alte Tage nach ihrer Aufbewahrungsfrist entfallen (beim nächsten Tageswechsel) oder eine Frist verkürzt wird; die Quote stellt der Betreiber beim Start mit --plattenquote ein.",
                    "regel": "quote",
                    "belegt": 1_048_519,
                    "hoechstens": 1_048_576
                })
            )
        );
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
        assert_eq!(
            sendekoerper_lesen(&json!({"text": "Hallo"})),
            Ok(("Hallo", vec![]))
        );
        assert_eq!(
            sendekoerper_lesen(&json!({"text": "  "})),
            Ok(("  ", vec![]))
        );
    }

    fn id(text: &str) -> Anhangid {
        Anhangid::lesen(text).expect("ID")
    }

    /// B0439: `anhaenge` mit IDs in Reihenfolge; `[]` heißt keine; über 10 → `anhaenge`; eine ID zweimal → `anhang`
    /// `doppelt`; kein ID-Text, Zahl oder `null` → `anhang` `unbekannt`; keine Liste → `anhaenge`; weiteres Feld →
    /// `feld`.
    #[test]
    fn der_sendekoerper_mit_anhaengen() {
        let a = "aaaaaaaaaaaaaaaaaaaaaaaaaa";
        let b = "bbbbbbbbbbbbbbbbbbbbbbbbbb";
        assert_eq!(
            sendekoerper_lesen(&json!({"text": "", "anhaenge": [a, b]})),
            Ok(("", vec![id(a), id(b)]))
        );
        assert_eq!(
            sendekoerper_lesen(&json!({"text": "x", "anhaenge": []})),
            Ok(("x", vec![]))
        );
        let elf: Vec<String> = (0..11)
            .map(|n| format!("{}{}", "a".repeat(25), char::from(b'a' + n)))
            .collect();
        assert_eq!(
            sendekoerper_lesen(&json!({"text": "x", "anhaenge": elf})),
            Err(Sendekoerperfehler::Anhaenge)
        );
        assert_eq!(
            sendekoerper_lesen(&json!({"text": "x", "anhaenge": [a, a]})),
            Err(Sendekoerperfehler::Anhang {
                id: a.to_string(),
                grund: "doppelt"
            })
        );
        for falsch in [
            json!("kurz"),
            json!(5),
            json!(null),
            json!(a.to_uppercase()),
        ] {
            let erwartet = falsch
                .as_str()
                .map_or_else(|| falsch.to_string(), str::to_string);
            assert_eq!(
                sendekoerper_lesen(&json!({"text": "x", "anhaenge": [falsch]})),
                Err(Sendekoerperfehler::Anhang {
                    id: erwartet,
                    grund: "unbekannt"
                })
            );
        }
        assert_eq!(
            sendekoerper_lesen(&json!({"text": "x", "anhaenge": a})),
            Err(Sendekoerperfehler::Anhaenge)
        );
        assert_eq!(
            sendekoerper_lesen(&json!({"text": "x", "anhaenge": [a], "y": 1})),
            Err(Sendekoerperfehler::Feld { feld: "y".into() })
        );
    }

    /// B5: Text leer oder nur Leerraum mit mindestens einem Anhang → `""`; ohne Anhang `leer` wie bisher; die übrigen
    /// Regeln gelten weiter.
    #[test]
    fn leerer_text_ist_mit_anhang_erlaubt() {
        assert_eq!(text_mit_anhaengen("", 1), Ok(""));
        assert_eq!(text_mit_anhaengen("  \n\t", 1), Ok(""));
        assert_eq!(text_mit_anhaengen("Hallo", 1), Ok("Hallo"));
        assert_eq!(text_mit_anhaengen("", 0), Err(Textfehler::Leer));
        assert_eq!(text_mit_anhaengen("  ", 0), Err(Textfehler::Leer));
        assert!(matches!(
            text_mit_anhaengen("a\u{1b}", 1),
            Err(Textfehler::Zeichen { .. })
        ));
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

    /// B0548 (ADR-0034 Ergänzung Karte #47): das weitere Feld vor dem Typfehler von `text` — `{"x":1}` nennt `x`,
    /// `{"text":1,"x":1}` ebenso; `{"text":1}` bleibt `Text`.
    #[test]
    fn ein_weiteres_feld_kommt_vor_dem_typfehler_von_text() {
        let feld = |name: &str| Err(Sendekoerperfehler::Feld { feld: name.into() });
        assert_eq!(sendekoerper_lesen(&json!({"x": 1})), feld("x"));
        assert_eq!(sendekoerper_lesen(&json!({"text": 1, "x": 1})), feld("x"));
        assert_eq!(sendekoerper_lesen(&json!({"y": 1})), feld("y"));
        assert_eq!(
            sendekoerper_lesen(&json!({"text": 1})),
            Err(Sendekoerperfehler::Text)
        );
    }
}
