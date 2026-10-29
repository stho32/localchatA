//! Die Antwort der Bremse (ADR-0036 Regel 4, Ergänzung Karte #31 (b) Regel 3): 429 `{fehler, grenze,
//! wieder_in_ms}` mit `Retry-After` für jede Route, die am Schreibkonto bucht; der Text nennt, wovon es zu viele gab.

use crate::gespraeche::Kennung;
use crate::schreibkonto::{Gebremst, Grenze};
use axum::Json;
use axum::http::StatusCode;
use axum::http::header::RETRY_AFTER;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use std::net::IpAddr;

/// Wovon eine Kennung zu viele schickte: Nachrichten (Senden) oder Änderungen (Frist).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gegenstand {
    Nachrichten,
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "die Frist bucht erst mit B0343")
    )]
    Aenderungen,
}

impl Gegenstand {
    fn als_text(self) -> &'static str {
        match self {
            Gegenstand::Nachrichten => "Nachrichten",
            Gegenstand::Aenderungen => "Änderungen",
        }
    }
}

/// 429 mit `Retry-After` = ⌈`wieder_in_ms` / 1.000⌉, mindestens 1; die Sekunden im Text sind `Retry-After`.
pub fn gebremst_beantworten(
    gebremst: Gebremst,
    gegenstand: Gegenstand,
    kennung: &Kennung,
    herkunft: IpAddr,
) -> Response {
    let wieder_in_ms = u64::try_from(gebremst.wieder_in.as_millis()).unwrap_or(u64::MAX);
    let sekunden = wieder_in_ms.div_ceil(1_000).max(1);
    let (grenze, wer) = match gebremst.grenze {
        Grenze::Kennung => ("kennung", kennung.als_text().to_string()),
        Grenze::Herkunft => (
            "herkunft",
            format!("{herkunft} (alle Teilnehmer dieser Adresse zusammen)"),
        ),
    };
    let fehler = format!(
        "Zu viele {} von {wer} — wieder möglich in {sekunden} s.",
        gegenstand.als_text()
    );
    (
        StatusCode::TOO_MANY_REQUESTS,
        [(RETRY_AFTER, sekunden.to_string())],
        Json(json!({ "fehler": fehler, "grenze": grenze, "wieder_in_ms": wieder_in_ms })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::time::Duration;

    async fn teile(antwort: Response) -> (u16, Option<String>, Value) {
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

    async fn antwort(
        gegenstand: Gegenstand,
        grenze: Grenze,
        ms: u64,
        herkunft: &str,
    ) -> (u16, Option<String>, Value) {
        let beta = Kennung::bilden("beta@127.0.0.1").expect("Kennung");
        teile(gebremst_beantworten(
            Gebremst {
                grenze,
                wieder_in: Duration::from_millis(ms),
            },
            gegenstand,
            &beta,
            herkunft.parse().expect("IP"),
        ))
        .await
    }

    /// B0234 (ADR-0036 Regel 4, R00011): 429 mit Grenze, `wieder_in_ms`, `Retry-After` = ⌈ms / 1000⌉, mindestens 1,
    /// und dem Text des Sendens je Eimer — wörtlich unverändert (R00021 „Text je Grenze“).
    #[tokio::test]
    async fn gebremst_ist_429_mit_retry_after_und_text_je_eimer() {
        let n = Gegenstand::Nachrichten;
        let kennung = antwort(n, Grenze::Kennung, 2_500, "127.0.0.2").await;
        let herkunft = antwort(n, Grenze::Herkunft, 125, "127.0.0.2").await;
        let genau = antwort(n, Grenze::Kennung, 3_000, "127.0.0.2").await;
        let null = antwort(n, Grenze::Kennung, 0, "127.0.0.2").await;

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

    /// R00021 F0072 „Text je Grenze“: `Gebremst { kennung, 10.000 ms }` der Frist → „Zu viele Änderungen von
    /// beta@127.0.0.1 — wieder möglich in 10 s.“, `Retry-After: 10`; `Gebremst { herkunft, 1.250 ms }` → „Zu viele
    /// Änderungen von 127.0.0.1 (alle Teilnehmer dieser Adresse zusammen) — wieder möglich in 2 s.“, `Retry-After: 2`.
    /// Mutante „Frist sagt Nachrichten“ wird rot.
    #[tokio::test]
    async fn die_frist_nennt_zu_viele_aenderungen_je_grenze() {
        let a = Gegenstand::Aenderungen;
        let kennung = antwort(a, Grenze::Kennung, 10_000, "127.0.0.1").await;
        let herkunft = antwort(a, Grenze::Herkunft, 1_250, "127.0.0.1").await;

        assert_eq!(
            kennung,
            (
                429,
                Some("10".to_string()),
                json!({"fehler": "Zu viele Änderungen von beta@127.0.0.1 — wieder möglich in 10 s.", "grenze": "kennung", "wieder_in_ms": 10_000})
            )
        );
        assert_eq!(
            herkunft,
            (
                429,
                Some("2".to_string()),
                json!({"fehler": "Zu viele Änderungen von 127.0.0.1 (alle Teilnehmer dieser Adresse zusammen) — wieder möglich in 2 s.", "grenze": "herkunft", "wieder_in_ms": 1_250})
            )
        );
    }
}
