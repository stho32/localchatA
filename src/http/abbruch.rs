//! Abbruch an der Grenze (ADR-0042 Regel 4 und Ergänzung 6): ein Blocking-Teil lief nicht zu Ende. Eine Hilfe für
//! alle Handler: 500 mit deutschem Satz ohne Fremdtext; Panik → `ERROR programmfehler … grund=panik` (die
//! EXCEPTION mit `ort` schreibt der Haken), Abbruch ohne Panik → `EXCEPTION programmfehler … grund=abgebrochen`,
//! geschlossene Werkbank → `EXCEPTION programmfehler … grund=werkbank_geschlossen`. Der Text des Abbruchs
//! (englisch, von tokio) steht nur im Protokoll, im Wert `ursache` (ADR-0042 Ergänzung #24 Punkte 5 und 6).

use super::ziel::Ziel;
use crate::protokoll::{Bereich, Kontext, Protokoll, Schluessel, Stufe};
use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use tokio::task::JoinError;

/// Wie ein Blocking-Teil endete, ohne ein Ergebnis zu liefern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Abbruchart {
    /// Der Blocking-Teil geriet in Panik.
    Panik,
    /// Er wurde abgebrochen, ohne Panik.
    Abgebrochen,
    /// Die Werkbank war geschlossen; die Arbeit begann nie.
    Geschlossen,
}

/// Ein Abbruch mit seiner Art und dem Text der Ursache (nur fürs Protokoll).
#[derive(Debug)]
pub struct Abgebrochen {
    pub art: Abbruchart,
    pub ursache: String,
}

impl From<JoinError> for Abgebrochen {
    fn from(fehler: JoinError) -> Abgebrochen {
        Abgebrochen {
            art: if fehler.is_panic() {
                Abbruchart::Panik
            } else {
                Abbruchart::Abgebrochen
            },
            ursache: fehler.to_string(),
        }
    }
}

/// Wo der Abbruch geschah: Routenmuster, Fachoperation, Kennung und Gespräch (Channel oder Direktgespräch), falls bekannt.
pub struct Grenzort<'a> {
    pub route: &'static str,
    pub operation: &'static str,
    pub kennung: Option<&'a str>,
    pub ziel: Option<&'a Ziel>,
}

/// 500 `{fehler: "Interner Fehler — <was>; bitte erneut versuchen"}` und genau ein Eintrag.
pub fn abbruch_beantworten(
    protokoll: &Protokoll,
    abbruch: &Abgebrochen,
    ort: &Grenzort<'_>,
    was: &str,
) -> Response {
    eintrag_schreiben(
        protokoll,
        abbruch,
        ort,
        format!("Anfrage an einem Programmfehler gescheitert — {was}"),
        Some(500),
    );
    interner_fehler(was)
}

/// Nur der Eintrag, wo keine Antwort mehr möglich ist (im Strom): `was` ist der ganze Text, ohne `status`.
pub fn abbruch_eintragen(
    protokoll: &Protokoll,
    abbruch: &Abgebrochen,
    ort: &Grenzort<'_>,
    was: &str,
) {
    eintrag_schreiben(protokoll, abbruch, ort, was.to_string(), None);
}

/// Panik → ERROR `grund=panik`; abgebrochen → EXCEPTION `grund=abgebrochen`; Werkbank geschlossen → EXCEPTION
/// `grund=werkbank_geschlossen` (ADR-0042 Ergänzung #24 Punkt 6). Der Fremdtext steht in `ursache` (Punkt 5).
fn eintrag_schreiben(
    protokoll: &Protokoll,
    abbruch: &Abgebrochen,
    ort: &Grenzort<'_>,
    text: String,
    status: Option<u16>,
) {
    let (stufe, grund) = match abbruch.art {
        Abbruchart::Panik => (Stufe::Error, "panik"),
        Abbruchart::Abgebrochen => (Stufe::Exception, "abgebrochen"),
        Abbruchart::Geschlossen => (Stufe::Exception, "werkbank_geschlossen"),
    };
    let mut kontext = Kontext::neu()
        .mit(Schluessel::Route, ort.route)
        .mit(Schluessel::Operation, ort.operation);
    if let Some(status) = status {
        kontext = kontext.mit(Schluessel::Status, status);
    }
    if let Some(kennung) = ort.kennung {
        kontext = kontext.mit(Schluessel::Kennung, kennung);
    }
    if let Some(ziel) = ort.ziel {
        kontext = kontext.mit(ziel.bezug(), ziel.name());
    }
    protokoll.eintragen(
        stufe,
        Bereich::Programmfehler,
        text,
        kontext
            .mit(Schluessel::Grund, grund)
            .mit(Schluessel::Ursache, &abbruch.ursache),
    );
}

/// 500 mit deutschem Satz ohne Fremdtext.
pub fn interner_fehler(was: &str) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "fehler": format!("Interner Fehler — {was}; bitte erneut versuchen") })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protokoll::testhilfe::im_speicher;

    const CHANNELS: Grenzort<'static> = Grenzort {
        route: "GET /api/channels",
        operation: "channels",
        kennung: None,
        ziel: None,
    };

    async fn antwort_als_json(antwort: Response) -> serde_json::Value {
        let koerper = axum::body::to_bytes(antwort.into_body(), 4096)
            .await
            .expect("Körper");
        serde_json::from_slice(&koerper).expect("JSON")
    }

    #[tokio::test]
    async fn abgebrochen_ohne_panik_ist_exception_mit_grund_abgebrochen_und_500_ohne_fremdtext() {
        let (protokoll, speicher, _, _) =
            im_speicher("2026-10-28T12:00:00+01:00", Stufe::Information);
        let handle = tokio::spawn(std::future::pending::<()>());
        handle.abort();
        let fehler = handle.await.expect_err("abgebrochen");

        let antwort = abbruch_beantworten(
            &protokoll,
            &Abgebrochen::from(fehler),
            &CHANNELS,
            "Channels nicht gelesen",
        );

        assert_eq!(antwort.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let json = antwort_als_json(antwort).await;
        assert_eq!(
            json["fehler"],
            "Interner Fehler — Channels nicht gelesen; bitte erneut versuchen"
        );
        let text = json.to_string();
        assert!(!text.contains("task") && !text.contains("cancel"), "{text}");
        let zeilen = speicher.zeilen();
        assert_eq!(zeilen.len(), 1, "{zeilen:#?}");
        assert!(
            zeilen[0].contains(" EXCEPTION programmfehler "),
            "{zeilen:#?}"
        );
        assert!(
            zeilen[0].contains(
                "route=\"GET /api/channels\" operation=channels status=500 grund=abgebrochen ursache="
            ),
            "{zeilen:#?}"
        );
        assert!(!zeilen[0].contains(" art="), "{zeilen:#?}");
        assert!(
            zeilen[0].contains("cancelled"),
            "der Fremdtext steht im Protokoll: {zeilen:#?}"
        );
    }

    #[tokio::test]
    async fn panik_ist_error_mit_grund_panik_kennung_und_channel() {
        let (protokoll, speicher, _, _) =
            im_speicher("2026-10-28T12:00:00+01:00", Stufe::Information);
        let fehler = tokio::task::spawn_blocking(|| panic!("Probe-Panik"))
            .await
            .expect_err("Panik");

        let antwort = abbruch_beantworten(
            &protokoll,
            &Abgebrochen::from(fehler),
            &Grenzort {
                route: "GET /api/channels/{name}/nachrichten",
                operation: "verlauf",
                kennung: Some("beta@127.0.0.1"),
                ziel: Some(&Ziel::Channel("allgemein".to_string())),
            },
            "Verlauf nicht gelesen",
        );

        let json = antwort_als_json(antwort).await;
        assert_eq!(
            json["fehler"],
            "Interner Fehler — Verlauf nicht gelesen; bitte erneut versuchen"
        );
        assert!(!json.to_string().contains("panicked"));
        let zeilen = speicher.zeilen();
        assert_eq!(zeilen.len(), 1, "{zeilen:#?}");
        assert!(zeilen[0].contains(" ERROR programmfehler "), "{zeilen:#?}");
        assert!(!zeilen[0].contains(" art="), "{zeilen:#?}");
        assert!(
            zeilen[0].contains("route=\"GET /api/channels/{name}/nachrichten\" operation=verlauf status=500 kennung=beta@127.0.0.1 channel=allgemein grund=panik"),
            "{zeilen:#?}"
        );
    }

    /// R00018 F0059 (ADR-0042 Ergänzung #24 Punkt 6): die geschlossene Werkbank ist ein eigener Grund, kein
    /// Abbruch — EXCEPTION mit `grund=werkbank_geschlossen`, der Fremdtext in `ursache`.
    #[tokio::test]
    async fn geschlossene_werkbank_ist_exception_mit_eigenem_grund() {
        let (protokoll, speicher, _, _) =
            im_speicher("2026-10-28T12:00:00+01:00", Stufe::Information);
        let abbruch = Abgebrochen {
            art: Abbruchart::Geschlossen,
            ursache: "Werkbank geschlossen — semaphore closed".to_string(),
        };

        let antwort =
            abbruch_beantworten(&protokoll, &abbruch, &CHANNELS, "Channels nicht gelesen");

        assert_eq!(antwort.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let zeilen = speicher.zeilen();
        assert_eq!(zeilen.len(), 1, "{zeilen:#?}");
        assert!(
            zeilen[0].contains(" EXCEPTION programmfehler "),
            "{zeilen:#?}"
        );
        assert!(
            zeilen[0].contains(
                "status=500 grund=werkbank_geschlossen ursache=\"Werkbank geschlossen — semaphore closed\""
            ),
            "{zeilen:#?}"
        );
        assert!(!zeilen[0].contains(" art="), "{zeilen:#?}");
    }

    /// R00018 F0059 (ADR-0042 Ergänzung #24 Punkt 1): wo keine Antwort mehr möglich ist (im Strom), schreibt
    /// `abbruch_eintragen` genau den Eintrag — ohne `status`, denn es geht keine 500 hinaus.
    #[tokio::test]
    async fn abbruch_eintragen_schreibt_nur_den_eintrag() {
        let (protokoll, speicher, _, _) =
            im_speicher("2026-10-28T12:00:00+01:00", Stufe::Information);
        let handle = tokio::spawn(std::future::pending::<()>());
        handle.abort();
        let fehler = handle.await.expect_err("abgebrochen");

        abbruch_eintragen(
            &protokoll,
            &Abgebrochen::from(fehler),
            &Grenzort {
                route: "GET /api/ereignisse",
                operation: "channels_stand",
                kennung: Some("beta@127.0.0.1"),
                ziel: None,
            },
            "Ereignisstrom beendet — Stand der Channels nicht geholt; der Client verbindet neu",
        );

        let zeilen = speicher.zeilen();
        assert_eq!(zeilen.len(), 1, "{zeilen:#?}");
        assert!(
            zeilen[0].contains(" EXCEPTION programmfehler Ereignisstrom beendet — Stand der Channels nicht geholt; der Client verbindet neu | route=\"GET /api/ereignisse\" operation=channels_stand kennung=beta@127.0.0.1 grund=abgebrochen ursache="),
            "{zeilen:#?}"
        );
    }
}
