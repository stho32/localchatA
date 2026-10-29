//! Frist setzen (R00008 F0033, ADR-0031): `PUT /api/channels/{name}/frist {tage}` mit Sitzung. Prüfreihenfolge
//! (Regel 2): Grenzschutz 403, Sitzung 401, Vorab-Buchung 429 → Körper 415/413/400 → unbekannter Channel 404 → Wert
//! 422 `frist` → Rest-Buchung 429 → defekte Einstellungen 409 `einstellungen-defekt` → beim Verkürzen ohne
//! Bestätigung Ordner nicht lesbar 500 (Ergänzung R00019) oder Rückfrage 409 `bestaetigen` → 200 `{channel,
//! entfallen}`. Am Schreibkonto kostet eine Änderung 10 Einheiten, 1 vorab und 9 vor dem Setzen; der unbekannte
//! Channel bucht den Rest vor der 404 (R00021, ADR-0036 Ergänzung Karte #31 (b)). Die Arbeit am Verzeichnis läuft
//! über `spawn_blocking`.

use super::Zustand;
use super::abbruch::{Abgebrochen, Grenzort, abbruch_beantworten};
use super::anfrageprotokoll::mit_regel;
use super::bremse::{Gegenstand, gebremst_melden};
use super::channels::channel_als_json;
use super::koerper::{Koerperfehler, Koerperort, json_lesen, koerperfehler_beantworten};
use super::teilnehmer::Teilnehmer;
use crate::gespraeche::fristeintrag::{
    frist_gesetzt_eintrag, frist_nicht_gesetzt_eintrag, pflege_eintrag,
    tage_nicht_ermittelt_eintrag,
};
use crate::gespraeche::{
    Entfaellt, Frist, FristGesetzt, Fristfehler, Gespraeche, Kennung, frist_in_spanne,
};
use crate::protokoll::{Eintrag, Protokoll};
use axum::Json;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};
use std::sync::Arc;

const ROUTE: &str = "PUT /api/channels/{name}/frist";
const BEISPIEL: &str = r#"{"tage":30}"#;
/// Körpergrenze (R00010, ADR-0010 Ergänzung): der größte sinnvolle Körper `{"tage":3650,"bestaetigt":false}` hat
/// 33 Bytes; mit Raum für Leerraum und Einrückung auf 1 KiB.
const FRIST_GRENZE: usize = 1024;
const REGEL_FRIST: &str = "frist";
const REGEL_BESTAETIGT: &str = "bestaetigt";
const REGEL_BESTAETIGEN: &str = "bestaetigen";
const REGEL_EINSTELLUNGEN_DEFEKT: &str = "einstellungen-defekt";
/// Volle Kosten einer Friständerung am Schreibkonto (ADR-0036 Ergänzung Karte #31 (b) Regel 1).
const KOSTEN: u32 = 10;

pub async fn setzen(
    teilnehmer: Teilnehmer,
    State(zustand): State<Arc<Zustand>>,
    Path(name): Path<String>,
    kopf: HeaderMap,
    koerper: Body,
) -> Response {
    let konto = &zustand.schreibkonto;
    if let Err(gebremst) = konto.vorab(&teilnehmer.kennung, teilnehmer.herkunft, KOSTEN) {
        return gebremst_melden(
            &zustand.protokoll,
            gebremst,
            Gegenstand::Aenderungen,
            &teilnehmer,
        );
    }
    let ort = Koerperort {
        protokoll: &zustand.protokoll,
        route: ROUTE,
        herkunft: teilnehmer.herkunft,
        kennung: Some(teilnehmer.kennung.als_text()),
    };
    let koerper = match json_lesen(&kopf, koerper, FRIST_GRENZE, &ort).await {
        Ok(koerper) => koerper,
        Err(fehler) => return koerper_abgewiesen(fehler),
    };
    let wunsch = wunsch_lesen(&zustand.gespraeche, &name, &koerper);
    if kostet_den_rest(&wunsch)
        && let Err(gebremst) = konto.rest(&teilnehmer.kennung, teilnehmer.herkunft, KOSTEN)
    {
        return gebremst_melden(
            &zustand.protokoll,
            gebremst,
            Gegenstand::Aenderungen,
            &teilnehmer,
        );
    }
    let Wunsch { tage, bestaetigt } = match wunsch {
        Ok(wunsch) => wunsch,
        Err(abweisung) => return abgewiesen(abweisung, &name, &teilnehmer.kennung),
    };
    let gespraeche = Arc::clone(&zustand.gespraeche);
    let gewuenscht = name.clone();
    let kennung = teilnehmer.kennung.clone();
    let ergebnis = tokio::task::spawn_blocking(move || {
        gespraeche
            .frist_setzen(&gewuenscht, tage, bestaetigt, &kennung)
            .map_err(Abweisung::Frist)
    })
    .await;
    match ergebnis {
        Ok(Ok(gesetzt)) => {
            gesetzt_eintragen(&zustand.protokoll, &gesetzt, &teilnehmer.kennung);
            gesetzt_beantworten(&gesetzt)
        }
        Ok(Err(Abweisung::Frist(Fristfehler::Ablage { datei, ursache }))) => {
            eintragen(
                &zustand.protokoll,
                frist_nicht_gesetzt_eintrag(&name, &teilnehmer.kennung, &datei, &ursache),
            );
            Abweisung::Frist(Fristfehler::Ablage { datei, ursache }).beantworten(&name)
        }
        Ok(Err(Abweisung::Frist(Fristfehler::TageNichtErmittelt { ordner, ursache }))) => {
            eintragen(
                &zustand.protokoll,
                tage_nicht_ermittelt_eintrag(&name, &teilnehmer.kennung, &ordner, &ursache),
            );
            Abweisung::Frist(Fristfehler::TageNichtErmittelt { ordner, ursache }).beantworten(&name)
        }
        Ok(Err(abweisung)) => abgewiesen(abweisung, &name, &teilnehmer.kennung),
        Err(abgebrochen) => abbruch_beantworten(
            &zustand.protokoll,
            &Abgebrochen::from(abgebrochen),
            &Grenzort {
                route: ROUTE,
                operation: "frist_setzen",
                kennung: Some(teilnehmer.kennung.als_text()),
                channel: Some(&name),
            },
            "Frist nicht gesetzt",
        ),
    }
}

/// Warum eine Anfrage nach dem Körper abgewiesen wurde.
enum Abweisung {
    Unbekannt,
    Regel(&'static str),
    Frist(Fristfehler),
}

/// Was die Anfrage setzen will, nachdem Channel und Wert geprüft sind.
struct Wunsch {
    tage: u32,
    bestaetigt: bool,
}

/// Channel vor Wert (ADR-0031 Regel 2.2, 2.3); `bestaetigt` fehlt = `false`, sonst ein Wahrheitswert. Defekte
/// Einstellungen und die Rückfrage prüft erst `frist_setzen`.
fn wunsch_lesen(gespraeche: &Gespraeche, name: &str, koerper: &Value) -> Result<Wunsch, Abweisung> {
    if !gespraeche.channel_bekannt(name) {
        return Err(Abweisung::Unbekannt);
    }
    let tage = frist_bilden(koerper.get("tage")).map_err(|_| Abweisung::Regel(REGEL_FRIST))?;
    let bestaetigt = match koerper.get("bestaetigt") {
        None => false,
        Some(wert) => wert.as_bool().ok_or(Abweisung::Regel(REGEL_BESTAETIGT))?,
    };
    Ok(Wunsch { tage, bestaetigt })
}

/// Ein ungültiger Wert (422) kostet nur die Vorab-Einheit; alles andere, auch der unbekannte Channel, die vollen
/// Kosten (R00021 „Was eine Anfrage kostet“).
fn kostet_den_rest(wunsch: &Result<Wunsch, Abweisung>) -> bool {
    !matches!(wunsch, Err(Abweisung::Regel(_)))
}

/// Eine Abweisung mit ihrem Regelschlüssel fürs Protokoll (ADR-0042 Ergänzung #24 Punkt 3).
fn abgewiesen(abweisung: Abweisung, name: &str, von: &Kennung) -> Response {
    let regel = abweisung.regelschluessel();
    mit_regel(abweisung.beantworten(name), regel, Some(von.als_text()))
}

/// Eine wirksame Änderung, dann was die Pflege tat — derselbe Wert schreibt nichts (R00017).
fn gesetzt_eintragen(protokoll: &Protokoll, gesetzt: &FristGesetzt, von: &Kennung) {
    let Some(alt) = gesetzt.geaendert_von else {
        return;
    };
    let Frist::Tage(neu) = gesetzt.channel.frist else {
        return;
    };
    eintragen(
        protokoll,
        frist_gesetzt_eintrag(gesetzt.channel.name.als_text(), alt, neu, von),
    );
    for bericht in &gesetzt.berichte {
        eintragen(protokoll, pflege_eintrag(bericht));
    }
}

fn eintragen(protokoll: &Protokoll, eintrag: Eintrag) {
    protokoll.eintragen(
        eintrag.stufe,
        eintrag.bereich,
        eintrag.text,
        eintrag.kontext,
    );
}

impl Abweisung {
    /// Regelschlüssel fürs Protokoll (ADR-0042 Ergänzung #24 Punkt 3).
    fn regelschluessel(&self) -> &'static str {
        match self {
            Abweisung::Unbekannt | Abweisung::Frist(Fristfehler::Unbekannt) => "channel_unbekannt",
            Abweisung::Regel(REGEL_BESTAETIGT) => "bestaetigt_ungueltig",
            Abweisung::Regel(_) => "frist_ungueltig",
            Abweisung::Frist(Fristfehler::Bestaetigen(_)) => "bestaetigen",
            Abweisung::Frist(Fristfehler::EinstellungenDefekt { .. }) => "einstellungen_defekt",
            Abweisung::Frist(Fristfehler::Ablage { .. }) => "ablage",
            Abweisung::Frist(Fristfehler::TageNichtErmittelt { .. }) => "tage_nicht_ermittelt",
        }
    }

    fn beantworten(self, name: &str) -> Response {
        match self {
            Abweisung::Unbekannt | Abweisung::Frist(Fristfehler::Unbekannt) => (
                StatusCode::NOT_FOUND,
                Json(json!({ "fehler": format!("# {name} gibt es nicht.") })),
            )
                .into_response(),
            Abweisung::Regel(REGEL_BESTAETIGT) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(json!({
                    "fehler": "„bestaetigt“ ist true oder false, zum Beispiel {\"tage\":7,\"bestaetigt\":true}.",
                    "regel": REGEL_BESTAETIGT,
                })),
            )
                .into_response(),
            Abweisung::Regel(regel) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(json!({
                    "fehler": "Die Frist ist eine ganze Zahl von 1 bis 3650 Tagen, zum Beispiel {\"tage\":30}.",
                    "regel": regel,
                })),
            )
                .into_response(),
            Abweisung::Frist(Fristfehler::Bestaetigen(entfaellt)) => bestaetigen(&entfaellt),
            Abweisung::Frist(Fristfehler::EinstellungenDefekt { datei }) => (
                StatusCode::CONFLICT,
                Json(json!({
                    "fehler": format!("{datei} ist defekt; von Hand reparieren und neu starten."),
                    "regel": REGEL_EINSTELLUNGEN_DEFEKT,
                    "datei": datei,
                })),
            )
                .into_response(),
            Abweisung::Frist(Fristfehler::TageNichtErmittelt { ordner, .. }) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "fehler": format!(
                        "Frist nicht gesetzt — die Tage von # {name} ließen sich nicht ermitteln (Ordner nicht lesbar). Leserecht von {ordner} prüfen und erneut versuchen."
                    ),
                })),
            )
                .into_response(),
            Abweisung::Frist(Fristfehler::Ablage { datei, ursache }) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "fehler": format!("Frist nicht gesetzt — {datei} ließ sich nicht schreiben ({ursache})."),
                })),
            )
                .into_response(),
        }
    }
}

fn gesetzt_beantworten(gesetzt: &FristGesetzt) -> Response {
    let (tage, nachrichten) = gesetzt.entfallen();
    Json(json!({
        "channel": channel_als_json(&gesetzt.channel),
        "entfallen": { "tage": tage, "nachrichten": nachrichten },
    }))
    .into_response()
}

/// 409 mit dem, was entfiele (ADR-0031 Regel 2.6) — die Oberfläche fragt mit diesen Zahlen zurück.
fn bestaetigen(entfaellt: &Entfaellt) -> Response {
    (
        StatusCode::CONFLICT,
        Json(json!({
            "fehler": bestaetigen_text(entfaellt),
            "regel": REGEL_BESTAETIGEN,
            "entfaellt": {
                "tage": entfaellt.tage,
                "nachrichten": entfaellt.nachrichten,
                "bis": entfaellt.bis.format("%Y-%m-%d").to_string(),
            },
        })),
    )
        .into_response()
}

/// Der Satz der 409 für Agenten: Verb, Tagesdatei und Nachricht je nach ihrer Zahl gebeugt (R00008, Nacharbeit #12).
fn bestaetigen_text(entfaellt: &Entfaellt) -> String {
    let (verb, tagesdateien) = match entfaellt.tage {
        1 => ("entfällt", "1 Tagesdatei".to_string()),
        n => ("entfallen", format!("{n} Tagesdateien")),
    };
    let nachrichten = match entfaellt.nachrichten {
        1 => "1 Nachricht".to_string(),
        n => format!("{n} Nachrichten"),
    };
    format!(
        "Mit dieser Frist {verb} sofort {tagesdateien} mit {nachrichten} bis {} — mit \"bestaetigt\":true erneut senden.",
        entfaellt.bis
    )
}

/// Die eine Antwort für Körperfehler (`koerper::koerperfehler_beantworten`) mit dem vollständigen 413-Satz und
/// dem Beispiel der Route.
fn koerper_abgewiesen(fehler: Koerperfehler) -> Response {
    koerperfehler_beantworten(
        fehler,
        &format!(
            "Der Körper ist zu groß — höchstens {FRIST_GRENZE} Bytes, nur die Tage schicken, zum Beispiel {BEISPIEL}."
        ),
        BEISPIEL,
    )
}

/// `tage` ist keine ganze Zahl von 1 bis 3650 — abgewiesen, nie umgeformt (ADR-0031 Regel 2.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Regelfehler;

/// Der JSON-Wert `tage` als Frist: nur eine ganze Zahl (`"7"`, `7.0`, `7.5`, `null` und Fehlen nicht) in der Spanne.
pub fn frist_bilden(wert: Option<&Value>) -> Result<u32, Regelfehler> {
    wert.and_then(Value::as_i64)
        .and_then(frist_in_spanne)
        .ok_or(Regelfehler)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn rueckfrage(tage: usize, nachrichten: usize) -> String {
        bestaetigen_text(&Entfaellt {
            tage,
            nachrichten,
            bis: chrono::NaiveDate::from_ymd_opt(2026, 10, 21).expect("Datum"),
        })
    }

    /// Abnahme-Probe Rechtschreib-Review #12, Muss 3: Verb, Tagesdatei und Nachricht folgen je ihrer Zahl.
    #[test]
    fn die_rueckfrage_beugt_verb_tagesdatei_und_nachricht_nach_ihrer_zahl() {
        let nachsatz = "bis 2026-10-21 — mit \"bestaetigt\":true erneut senden.";
        for (tage, nachrichten, erwartet) in [
            (1, 1, "entfällt sofort 1 Tagesdatei mit 1 Nachricht"),
            (1, 2, "entfällt sofort 1 Tagesdatei mit 2 Nachrichten"),
            (2, 1, "entfallen sofort 2 Tagesdateien mit 1 Nachricht"),
            (2, 5, "entfallen sofort 2 Tagesdateien mit 5 Nachrichten"),
        ] {
            assert_eq!(
                rueckfrage(tage, nachrichten),
                format!("Mit dieser Frist {erwartet} {nachsatz}"),
                "{tage}/{nachrichten}"
            );
        }
    }

    fn gebildet(koerper: Value) -> Result<u32, Regelfehler> {
        frist_bilden(koerper.get("tage"))
    }

    #[test]
    fn eins_und_3650_gelten_als_spanne() {
        assert_eq!(gebildet(json!({ "tage": 1 })), Ok(1));
        assert_eq!(gebildet(json!({ "tage": 7 })), Ok(7));
        assert_eq!(gebildet(json!({ "tage": 3650 })), Ok(3650));
    }

    #[test]
    fn null_3651_und_negative_liegen_ausserhalb() {
        for tage in [
            json!(0),
            json!(3651),
            json!(-1),
            json!(i64::MAX),
            json!(u64::MAX),
        ] {
            assert_eq!(
                gebildet(json!({ "tage": tage })),
                Err(Regelfehler),
                "{tage}"
            );
        }
    }

    #[test]
    fn keine_ganze_zahl_ist_ein_regelfehler_und_wird_nicht_umgeformt() {
        let koerper: Value = serde_json::from_str(r#"{"tage":7.0}"#).expect("JSON");
        assert_eq!(gebildet(koerper), Err(Regelfehler));
        for tage in [
            json!("7"),
            json!(7.5),
            Value::Null,
            json!([7]),
            json!({ "n": 7 }),
            json!(true),
        ] {
            assert_eq!(
                gebildet(json!({ "tage": tage })),
                Err(Regelfehler),
                "{tage}"
            );
        }
        assert_eq!(gebildet(json!({})), Err(Regelfehler));
    }
}
