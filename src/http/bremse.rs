//! Die Bremse an jeder Route, die am Schreibkonto bucht (ADR-0036 Regeln 4, 7, Ergänzung Karte #31 (b) Regel 3):
//! eine Episode `WARNING schreibkonto` je Kennung und Gegenstand, dazu 429 `{fehler, grenze, wieder_in_ms}` mit
//! `Retry-After`; Text und Episode nennen, wovon es zu viele gab.

use super::teilnehmer::Teilnehmer;
use crate::gespraeche::Kennung;
use crate::protokoll::episoden::{Auftreten, Quelle};
use crate::protokoll::episodentexte;
use crate::protokoll::grenze::Grenzart;
use crate::protokoll::{Bereich, Protokoll, Schluessel, Stufe};
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
    Aenderungen,
    /// Anmeldungen von Anhängen (ADR-0036 Ergänzung Karte #9: 1 Einheit je Anmeldung).
    Anmeldungen,
    /// Löschanfragen für Anhänge (1 Einheit vorab, ⌈n / 100⌉ als Rest).
    Loeschungen,
}

impl Gegenstand {
    fn als_text(self) -> &'static str {
        match self {
            Gegenstand::Nachrichten => "Nachrichten",
            Gegenstand::Aenderungen => "Änderungen",
            Gegenstand::Anmeldungen => "Anmeldungen",
            Gegenstand::Loeschungen => "Löschanfragen",
        }
    }

    /// Der Text ist Teil des Episodenschlüssels: Senden und Frist derselben Kennung sind zwei Episoden.
    fn episodentext(self) -> &'static str {
        match self {
            Gegenstand::Nachrichten => episodentexte::GEBREMST,
            Gegenstand::Aenderungen => episodentexte::FRIST_GEBREMST,
            Gegenstand::Anmeldungen => episodentexte::ANHAENGE_GEBREMST,
            Gegenstand::Loeschungen => episodentexte::LOESCHEN_GEBREMST,
        }
    }
}

/// Jede 429 der Bremse zählt in die Episode ihres Gegenstands je Kennung (R00016, R00021).
pub fn gebremst_melden(
    protokoll: &Protokoll,
    gebremst: Gebremst,
    gegenstand: Gegenstand,
    teilnehmer: &Teilnehmer,
) -> Response {
    protokoll.episode(gebremst_auftreten(
        teilnehmer.kennung.als_text(),
        gebremst.grenze,
        gegenstand,
    ));
    gebremst_beantworten(
        gebremst,
        gegenstand,
        &teilnehmer.kennung,
        teilnehmer.herkunft,
    )
}

/// Das Auftreten einer 429 der Bremse: Quelle ist die Kennung; die Grenze zählt als Art, damit die Ende-Zeile je
/// Grenze zählt (ADR-0042 Ergänzung Karte #31 Punkt 2), und steht als `grenze=` nur in der Beginn-Zeile (Punkt 4).
pub(crate) fn gebremst_auftreten(wer: &str, grenze: Grenze, gegenstand: Gegenstand) -> Auftreten {
    let grenze = match grenze {
        Grenze::Kennung => Grenzart::Kennung,
        Grenze::Herkunft => Grenzart::Herkunft,
    };
    Auftreten::neu(
        Stufe::Warning,
        Bereich::Schreibkonto,
        gegenstand.episodentext(),
        Quelle::neu().mit(Schluessel::Kennung, wer),
    )
    .einzeln(Schluessel::Grenze, grenze)
    .art(grenze.als_text())
}

/// 429 mit `Retry-After` = ⌈`wieder_in_ms` / 1.000⌉, mindestens 1; die Sekunden im Text sind `Retry-After`.
fn gebremst_beantworten(
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

    /// R00020 F0070 „Ende-Zeile rein“ (ADR-0042 Ergänzung Karte #31 Punkt 7): 6 × `kennung` und 2 × `herkunft`
    /// in einer Episode → die Ende-Zeile zählt je Grenze, trägt `kennung=` und kein `grenze=`; nur `kennung` →
    /// „(kennung 6)“ ohne „herkunft 0“. Die Beginn-Zeile behält `grenze=` des ersten Auftretens.
    #[test]
    fn die_ende_zeile_der_bremse_zaehlt_je_grenze_mit_dem_kontext_der_quelle() {
        use crate::protokoll::testhilfe::{im_speicher, zeit};
        let (protokoll, speicher, uhr, _) =
            im_speicher("2026-10-28T12:00:00+01:00", Stufe::Information);
        let folge = [
            Grenze::Herkunft,
            Grenze::Kennung,
            Grenze::Kennung,
            Grenze::Kennung,
            Grenze::Herkunft,
            Grenze::Kennung,
            Grenze::Kennung,
            Grenze::Kennung,
        ];
        for (sekunde, grenze) in folge.into_iter().enumerate() {
            uhr.stellen(zeit(&format!("2026-10-28T12:00:0{sekunde}+01:00")));
            protokoll.episode(gebremst_auftreten(
                "beta@127.0.0.1",
                grenze,
                Gegenstand::Nachrichten,
            ));
            protokoll.episode(gebremst_auftreten(
                "gamma@127.0.0.1",
                Grenze::Kennung,
                Gegenstand::Nachrichten,
            ));
        }
        uhr.stellen(zeit("2026-10-28T12:02:00+01:00"));
        protokoll.episoden_schliessen();

        let konto: Vec<String> = speicher
            .zeilen()
            .into_iter()
            .filter(|zeile| zeile.contains(" WARNING schreibkonto "))
            .collect();
        let von = |wer: &str, art: &str| -> Vec<String> {
            konto
                .iter()
                .filter(|z| z.contains(wer) && z.contains(art))
                .cloned()
                .collect()
        };
        assert_eq!(konto.len(), 4, "{konto:#?}");
        assert_eq!(
            von("beta@", "Episode beginnt"),
            [konto[0].clone()],
            "{konto:#?}"
        );
        assert!(
            konto[0].ends_with(" — Episode beginnt | kennung=beta@127.0.0.1 grenze=herkunft"),
            "{konto:#?}"
        );
        let beta_ende = von("beta@", "Episode endet");
        assert_eq!(beta_ende.len(), 1, "{konto:#?}");
        assert!(
            beta_ende[0].ends_with(
                " — Episode endet: 8 in 7 s (herkunft 2; kennung 6) | kennung=beta@127.0.0.1"
            ),
            "{konto:#?}"
        );
        let gamma_ende = von("gamma@", "Episode endet");
        assert_eq!(gamma_ende.len(), 1, "{konto:#?}");
        assert!(
            gamma_ende[0]
                .ends_with(" — Episode endet: 8 in 7 s (kennung 8) | kennung=gamma@127.0.0.1"),
            "{konto:#?}"
        );
    }

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
