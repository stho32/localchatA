//! Abbruch an der Grenze (ADR-0042 Regel 4 und Ergänzung 6): ein Blocking-Teil lief nicht zu Ende. Eine Hilfe für
//! alle Handler: 500 mit deutschem Satz ohne Fremdtext; Panik → `ERROR programmfehler … grund=panik` (die
//! EXCEPTION mit `ort` schreibt der Haken), Abbruch ohne Panik → `EXCEPTION programmfehler … grund=abgebrochen`.
//! Der Text des Abbruchs (englisch, von tokio) steht nur im Protokoll, im Wert `art`.

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

/// Wo der Abbruch geschah: Routenmuster, Fachoperation, Kennung und Channel, falls bekannt.
pub struct Grenzort<'a> {
    pub route: &'static str,
    pub operation: &'static str,
    pub kennung: Option<&'a str>,
    pub channel: Option<&'a str>,
}

/// 500 `{fehler: "Interner Fehler — <was>; bitte erneut versuchen"}` und genau ein Eintrag.
pub fn abbruch_beantworten(
    protokoll: &Protokoll,
    abbruch: &Abgebrochen,
    ort: &Grenzort<'_>,
    was: &str,
) -> Response {
    let (stufe, grund) = match abbruch.art {
        Abbruchart::Panik => (Stufe::Error, "panik"),
        Abbruchart::Abgebrochen | Abbruchart::Geschlossen => (Stufe::Exception, "abgebrochen"),
    };
    let mut kontext = Kontext::neu()
        .mit(Schluessel::Route, ort.route)
        .mit(Schluessel::Operation, ort.operation)
        .mit(Schluessel::Status, 500);
    if let Some(kennung) = ort.kennung {
        kontext = kontext.mit(Schluessel::Kennung, kennung);
    }
    if let Some(channel) = ort.channel {
        kontext = kontext.mit(Schluessel::Channel, channel);
    }
    protokoll.eintragen(
        stufe,
        Bereich::Programmfehler,
        format!("Anfrage an einem Programmfehler gescheitert — {was}"),
        kontext
            .mit(Schluessel::Grund, grund)
            .mit(Schluessel::Art, &abbruch.ursache),
    );
    interner_fehler(was)
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
        channel: None,
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
                "route=\"GET /api/channels\" operation=channels status=500 grund=abgebrochen art="
            ),
            "{zeilen:#?}"
        );
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
                channel: Some("allgemein"),
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
        assert!(
            zeilen[0].contains("route=\"GET /api/channels/{name}/nachrichten\" operation=verlauf status=500 kennung=beta@127.0.0.1 channel=allgemein grund=panik"),
            "{zeilen:#?}"
        );
    }
}
