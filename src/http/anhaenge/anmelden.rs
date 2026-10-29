//! Anmelden beantworten (B0427, ADR-0046 Regel 1 mit Ergänzungen „Karte #9 Gate“, „F1′“, „Karte #9
//! Security-Gate“ L9): `POST /api/anhaenge {name, bytes}` mit Sitzung. Prüfreihenfolge, abgewiesen, nie umgeformt:
//! 403 · 401 · 429 (1 Einheit vorab) · 415/400/413/408 · 422 `feld` (doppelt, dann weiteres Feld) · 422 `bytes` ·
//! 422 `leer` · 422 `name` · 422 `zu_gross` · 429 offen · 507 `platte` · 507 `quote` (`voll` | `hochladungen`) · 201.

use super::super::Zustand;
use super::super::anfrageprotokoll::mit_regel;
use super::super::bremse::{Gegenstand, gebremst_melden};
use super::super::felder::{Felder, doppelt, wert};
use super::super::koerper::{Koerperfehler, Koerperort, felder_lesen, koerperfehler_beantworten};
use super::super::senden::groesse_in_worten;
use super::super::teilnehmer::Teilnehmer;
use super::super::teilnehmerliste::zeit_als_text;
use crate::gespraeche::Anmeldefehler;
use crate::gespraeche::anhangeingang::{Anmeldung, Offen};
use crate::gespraeche::anhangpruefung::Namensfehler;
use crate::gespraeche::anhangquote::{Platzfehler, Quotestand, zaehlmass};
use crate::protokoll::episoden::Auftreten;
use crate::protokoll::episodentexte;
use crate::protokoll::{Bereich, Kontext, Schluessel, Stufe};
use axum::Json;
use axum::body::Body;
use axum::extract::State;
use axum::http::header::RETRY_AFTER;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};
use std::sync::Arc;

const ROUTE: &str = "POST /api/anhaenge";

/// Körpergrenze (ADR-0046 Regel 1): Name ≤ 255 Zeichen in der längsten Schreibweise `\uXXXX` = 1.530, dazu
/// `{"name":"","bytes":18446744073709551615}` = 40, plus 30 % ≈ 2.041 → 2.048.
pub const ANMELDEN_GRENZE: usize = 2_048;

/// Vorab-Buchung am Schreibkonto (ADR-0036 Ergänzung Karte #9).
const VORAB_ANMELDEN: u32 = 1;

const BEISPIEL: &str = r#"{"name":"bericht.pdf","bytes":2457600}"#;

pub async fn anmelden(
    teilnehmer: Teilnehmer,
    State(zustand): State<Arc<Zustand>>,
    kopf: HeaderMap,
    koerper: Body,
) -> Response {
    if let Err(gebremst) =
        zustand
            .schreibkonto
            .vorab(&teilnehmer.kennung, teilnehmer.herkunft, VORAB_ANMELDEN)
    {
        return gebremst_melden(
            &zustand.protokoll,
            gebremst,
            Gegenstand::Anmeldungen,
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
    let felder = match felder_lesen(&kopf, koerper, ANMELDEN_GRENZE, &ort).await {
        Ok(felder) => felder,
        Err(fehler) => return koerper_abgewiesen(fehler),
    };
    let (name, bytes) = match koerper_pruefen(felder.as_ref()) {
        Ok(gelesen) => gelesen,
        Err(abweisung) => {
            let antwort =
                (StatusCode::UNPROCESSABLE_ENTITY, Json(abweisung.koerper)).into_response();
            return mit_regel(antwort, abweisung.regel, Some(&wer));
        }
    };
    let gespraeche = Arc::clone(&zustand.gespraeche);
    let kennung = teilnehmer.kennung.clone();
    let herkunft = teilnehmer.herkunft;
    let anzeigename = name.clone();
    let angemeldet = tokio::task::spawn_blocking(move || {
        gespraeche.anhang_anmelden(&kennung, herkunft, &anzeigename, bytes)
    })
    .await;
    match angemeldet {
        Ok(Ok(anmeldung)) => {
            anspruch_melden(&zustand, &teilnehmer, bytes, &anmeldung);
            angenommen(&anmeldung, &name, bytes)
        }
        Ok(Err(fehler)) => {
            let (antwort, regel) = abgewiesen(&fehler);
            mit_regel(antwort, regel, Some(&wer))
        }
        Err(_) => super::super::abbruch::interner_fehler("Anmelden"),
    }
}

/// Eine 422 des Körpers mit ihrem Regelschlüssel für das Anfrageprotokoll.
#[derive(Debug)]
struct Abweisung {
    koerper: Value,
    regel: &'static str,
}

fn unprocessable(regel: &'static str, koerper: Value) -> Abweisung {
    Abweisung { koerper, regel }
}

/// Feld doppelt → weiteres Feld → `bytes` → `leer` (B0427); `name` als Zeichenkette prüft die Namensregel danach.
fn koerper_pruefen(felder: Option<&Felder>) -> Result<(String, u64), Abweisung> {
    let Some(felder) = felder else {
        return Err(unprocessable(
            "feld_unbekannt",
            json!({
                "fehler": format!("Der Körper braucht ein JSON-Objekt, zum Beispiel {BEISPIEL}."),
                "regel": "feld",
            }),
        ));
    };
    if let Some(feld) = doppelt(felder) {
        return Err(unprocessable(
            "feld_doppelt",
            json!({
                "fehler": format!("Das Feld {feld} steht zweimal im Körper — jedes Feld nur einmal schicken."),
                "regel": "feld",
                "feld": feld,
                "grund": "doppelt",
            }),
        ));
    }
    if let Some((feld, _)) = felder
        .iter()
        .find(|(feld, _)| feld != "name" && feld != "bytes")
    {
        return Err(unprocessable(
            "feld_unbekannt",
            json!({
                "fehler": format!("Unbekanntes Feld {feld} — erlaubt sind name und bytes."),
                "regel": "feld",
                "feld": feld,
            }),
        ));
    }
    let Some(bytes) = wert(felder, "bytes").and_then(Value::as_u64) else {
        return Err(unprocessable(
            "bytes",
            json!({
                "fehler": "bytes braucht die Länge der Datei als ganze Zahl in Bytes.",
                "regel": "bytes",
            }),
        ));
    };
    if bytes == 0 {
        return Err(unprocessable(
            "leer",
            json!({ "fehler": "Die Datei ist leer — es gibt nichts hochzuladen.", "regel": "leer" }),
        ));
    }
    let Some(name) = wert(felder, "name").and_then(Value::as_str) else {
        return Err(unprocessable(
            "name",
            json!({
                "fehler": "name braucht den Dateinamen als Zeichenkette.",
                "regel": "name",
                "grund": "fehlt",
            }),
        ));
    };
    Ok((name.to_string(), bytes))
}

/// Ob eine angenommene Anmeldung ein großer Anspruch ist: `z(bytes) ≥ Q/4` bei gesetzter Anhang-Quote (B0525).
pub fn grosser_anspruch(bytes: u64, anhangquote: Option<u64>) -> bool {
    anhangquote.is_some_and(|quote| zaehlmass(bytes).saturating_mul(4) >= quote)
}

/// Episode je Kennung `INFORMATION anhang Großer Anspruch` (ADR-0042 und ADR-0046 Ergänzungen „Karte #9
/// Security-Gate“, M42′ Bedingung 2): wer so viel beansprucht, kann mit leeren `PUT`s das Hochladen aller sperren.
fn anspruch_melden(zustand: &Zustand, teilnehmer: &Teilnehmer, bytes: u64, anmeldung: &Anmeldung) {
    if !grosser_anspruch(bytes, zustand.gespraeche.anhangkennzahlen().anhangquote) {
        return;
    }
    let wer = teilnehmer.kennung.als_text();
    zustand.protokoll.episode(Auftreten {
        stufe: Stufe::Information,
        bereich: Bereich::Anhang,
        text: episodentexte::GROSSER_ANSPRUCH,
        quelle: wer.to_string(),
        kontext: Kontext::neu()
            .mit(Schluessel::Kennung, wer)
            .mit(Schluessel::Herkunft, teilnehmer.herkunft)
            .mit(Schluessel::Bytes, bytes)
            .mit(
                Schluessel::HochladenBis,
                zeit_als_text(&anmeldung.hochladen_bis),
            ),
        art: None,
        verworfen: None,
        bytes: None,
        anzeige: None,
    });
}

/// 201 `{anhang: {id, name, bytes, hochladen, anlauf_bis, hochladen_bis}}`.
fn angenommen(anmeldung: &Anmeldung, name: &str, bytes: u64) -> Response {
    let id = anmeldung.id.als_text();
    (
        StatusCode::CREATED,
        Json(json!({
            "anhang": {
                "id": id,
                "name": name,
                "bytes": bytes,
                "hochladen": format!("/api/anhaenge/{id}"),
                "anlauf_bis": zeit_als_text(&anmeldung.anlauf_bis),
                "hochladen_bis": zeit_als_text(&anmeldung.hochladen_bis),
            }
        })),
    )
        .into_response()
}

/// Die Antwort auf eine abgewiesene Anmeldung und ihr Regelschlüssel.
fn abgewiesen(fehler: &Anmeldefehler) -> (Response, &'static str) {
    match fehler {
        Anmeldefehler::Name(fehler) => (name_abgewiesen(fehler), "name"),
        Anmeldefehler::Platz(Platzfehler::ZuGross { hoechstens }) => (
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(json!({
                    "fehler": format!(
                        "Nicht hochgeladen: die Datei ist größer als der ganze Platz für Anhänge dieser Instanz ({}).",
                        groesse_in_worten(*hoechstens)
                    ),
                    "regel": "zu_gross",
                    "hoechstens": hoechstens,
                })),
            )
                .into_response(),
            "zu_gross",
        ),
        Anmeldefehler::Offen(offen) => (offen_abgewiesen(*offen), offen.grenze()),
        Anmeldefehler::Platz(Platzfehler::Platte { frei }) => (
            (
                StatusCode::INSUFFICIENT_STORAGE,
                Json(json!({
                    "fehler": format!(
                        "Nicht hochgeladen: auf der Platte ist nicht genug Platz — frei sind {}, 1,0 GiB bleiben immer frei.",
                        groesse_in_worten(*frei)
                    ),
                    "regel": "platte",
                    "frei": frei,
                })),
            )
                .into_response(),
            "platte",
        ),
        Anmeldefehler::Platz(Platzfehler::Voll(stand)) => (
            quote_abgewiesen(
                stand,
                "voll",
                format!(
                    "Nicht hochgeladen: der Platz für Anhänge ist voll — frei sind {}, die Datei braucht {}. Platz entsteht, wenn Anhänge gelöscht werden oder ältere Tage entfallen; Text kann weiter gesendet werden.",
                    groesse_in_worten(stand.frei),
                    groesse_in_worten(stand.braucht)
                ),
            )
            .into_response(),
            "quote_voll",
        ),
        Anmeldefehler::Platz(Platzfehler::Hochladungen(stand)) => (
            (
                [(RETRY_AFTER, "30")],
                quote_abgewiesen(
                    stand,
                    "hochladungen",
                    format!(
                        "Nicht hochgeladen: laufende Hochladungen belegen gerade den Platz für Anhänge — noch {} frei. Gleich noch einmal versuchen.",
                        groesse_in_worten(stand.frei)
                    ),
                ),
            )
                .into_response(),
            "quote_hochladungen",
        ),
        Anmeldefehler::Zufall(_) => (
            super::super::abbruch::interner_fehler("Anmelden"),
            "zufall",
        ),
    }
}

fn quote_abgewiesen(stand: &Quotestand, grund: &str, fehler: String) -> impl IntoResponse {
    (
        StatusCode::INSUFFICIENT_STORAGE,
        Json(json!({
            "fehler": fehler,
            "regel": "quote",
            "grund": grund,
            "belegt": stand.belegt,
            "gebunden": stand.gebunden,
            "reserviert": stand.reserviert,
            "hoechstens": stand.hoechstens,
            "frei": stand.frei,
            "braucht": stand.braucht,
        })),
    )
}

/// 429 `{fehler, grenze, hoechstens}` mit `Retry-After: 5`.
fn offen_abgewiesen(offen: Offen) -> Response {
    let wer = match offen {
        Offen::Kennung => "je Teilnehmer",
        Offen::Herkunft => "je Adresse (alle Teilnehmer dieser Adresse zusammen)",
        Offen::Gesamt => "laufende in dieser Instanz",
    };
    (
        StatusCode::TOO_MANY_REQUESTS,
        [(RETRY_AFTER, "5")],
        Json(json!({
            "fehler": format!(
                "Zu viele offene Hochladungen — höchstens {} {wer}; erst hochladen, senden oder abbrechen.",
                offen.hoechstens()
            ),
            "grenze": offen.grenze(),
            "hoechstens": offen.hoechstens(),
        })),
    )
        .into_response()
}

/// 422 `name` mit `grund` (`leer` | `laenge` | `zeichen` | `punkte`).
fn name_abgewiesen(fehler: &Namensfehler) -> Response {
    let (grund, text) = match fehler {
        Namensfehler::Leer => ("leer", "Der Dateiname ist leer.".to_string()),
        Namensfehler::Laenge { bytes } => (
            "laenge",
            format!("Der Dateiname ist zu lang: {bytes} von höchstens 255 Byte."),
        ),
        Namensfehler::Zeichen { stelle, zeichen } => (
            "zeichen",
            format!(
                "Der Dateiname enthält das Zeichen {zeichen} an Stelle {} — bitte umbenennen.",
                stelle + 1
            ),
        ),
        Namensfehler::Punkte => (
            "punkte",
            "Der Dateiname darf nicht „.“ oder „..“ sein.".to_string(),
        ),
    };
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(json!({ "fehler": text, "regel": "name", "grund": grund })),
    )
        .into_response()
}

/// Die eine Antwort für Körperfehler mit dem 413-Satz und dem Beispiel der Route.
fn koerper_abgewiesen(fehler: Koerperfehler) -> Response {
    koerperfehler_beantworten(
        fehler,
        "Der Körper ist zu groß — höchstens 2.048 Byte; nur name und bytes schicken.",
        BEISPIEL,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// B0525 an der Grenze im Zählmaß (Q = 10 MiB): 2.617.345 Bytes (z = 640 × 4.096 = Q/4) → großer Anspruch,
    /// 2.617.344 (z = 639 × 4.096) → nicht; ohne Anhang-Quote nie (Mutanten „> statt ≥“, „bytes statt z“).
    #[test]
    fn grosser_anspruch_an_der_grenze() {
        let quote = Some(10 * 1_048_576);
        assert!(grosser_anspruch(2_617_345, quote));
        assert!(!grosser_anspruch(2_617_344, quote));
        assert!(grosser_anspruch(3 * 1_048_576, quote));
        assert!(!grosser_anspruch(u64::MAX / 8, None));
    }

    fn felder(roh: &str) -> Felder {
        super::super::super::felder::felder_lesen(roh.as_bytes())
            .expect("JSON")
            .expect("Objekt")
    }

    async fn regel(felder: Option<&Felder>) -> (String, Value) {
        let abweisung = koerper_pruefen(felder).expect_err("abgewiesen");
        (abweisung.regel.to_string(), abweisung.koerper)
    }

    /// R00029 „Abweisungen der Anmeldung“: weiteres Feld → `feld`; `bytes` −1, 1.5, "1", 2⁶⁴ → `bytes`; 0 → `leer`;
    /// `bytes` zweimal → `feld`/`doppelt`, nie die letzte Fassung.
    #[tokio::test]
    async fn der_koerper_wird_in_der_reihenfolge_geprueft() {
        let (regel_x, x) = regel(Some(&felder(r#"{"name":"a","bytes":1,"x":1}"#))).await;
        assert_eq!(
            (regel_x.as_str(), x["regel"].as_str(), x["feld"].as_str()),
            ("feld_unbekannt", Some("feld"), Some("x"))
        );
        for falsch in ["-1", "1.5", "\"1\"", "18446744073709551616", "null"] {
            let (_, antwort) = regel(Some(&felder(&format!(
                r#"{{"name":"a","bytes":{falsch}}}"#
            ))))
            .await;
            assert_eq!(antwort["regel"], "bytes", "{falsch}");
        }
        let (_, leer) = regel(Some(&felder(r#"{"name":"a","bytes":0}"#))).await;
        assert_eq!(leer["regel"], "leer");
        let (_, doppelt) = regel(Some(&felder(r#"{"name":"a","bytes":1,"bytes":2}"#))).await;
        assert_eq!(
            (
                doppelt["regel"].as_str(),
                doppelt["feld"].as_str(),
                doppelt["grund"].as_str()
            ),
            (Some("feld"), Some("bytes"), Some("doppelt"))
        );
        let (_, doppelt_vor_fremd) =
            regel(Some(&felder(r#"{"x":1,"name":"a","name":"b","bytes":1}"#))).await;
        assert_eq!(doppelt_vor_fremd["grund"], "doppelt");
        let (_, ohne_name) = regel(Some(&felder(r#"{"bytes":1}"#))).await;
        assert_eq!(
            (ohne_name["regel"].as_str(), ohne_name["grund"].as_str()),
            (Some("name"), Some("fehlt"))
        );
        let (_, kein_objekt) = regel(None).await;
        assert_eq!(kein_objekt["regel"], "feld");
        assert_eq!(
            koerper_pruefen(Some(&felder(
                r#"{"name":"a","bytes":18446744073709551615}"#
            )))
            .ok(),
            Some(("a".to_string(), u64::MAX))
        );
    }
}
