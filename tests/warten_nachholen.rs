//! F0170 Nachholen bleibt begrenzt über das echte Programm (R00049, ADR-0054 Ergänzung „Nachholen von `warten` …“
//! Regel 1 und 8, B0758): `/api/gesundheit` nennt `nachholen {saetze, gebremst, bank_plaetze, bank_belegt}`, die
//! Nachholbank hat die Hälfte der Kerne (mindestens 1) neben der Werkbank, und ein Nachholen zählt seine Sätze.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::senden::{beitreten, ordner, senden, starten};
use gemeinsam::{Absender, api_antwort, gesundheit};

/// Health vor und nach einem Nachholen über drei eigene Nachrichten von `anna`: `claude` holt mit altem `seit` nach
/// → `saetze` 3, `gebremst` 0, `bank_belegt` 0; `bank_plaetze` ist die Hälfte der Werkbank-Kerne, mindestens 1.
#[tokio::test]
async fn die_gesundheit_nennt_das_nachholen_und_eine_eigene_bank() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[]);
    let absender = Absender::von("127.0.0.1");
    let anna = beitreten(&absender, &instanz.adresse, "anna", None).await;
    let claude = beitreten(&absender, &instanz.adresse, "claude", None).await;
    for nummer in 1..=3 {
        let antwort = senden(
            &absender,
            &instanz.adresse,
            &anna,
            "allgemein",
            &format!("n{nummer}"),
        )
        .await;
        assert_eq!(antwort.status, 201, "{}", antwort.json);
    }
    let vorher = gesundheit(&instanz.adresse).await.1;

    let gewartet = api_antwort(
        absender
            .anfrage(
                reqwest::Method::GET,
                format!(
                    "{}api/warten?bis=0&seit=2000-01-01T00:00:00.000Z",
                    instanz.adresse
                ),
            )
            .header("X-LocalchatA-Sitzung", claude),
    )
    .await;
    let nachher = gesundheit(&instanz.adresse).await.1;

    assert_eq!(gewartet.status, 200, "{}", gewartet.json);
    assert_eq!(vorher["nachholen"]["saetze"], 0, "{vorher}");
    assert_eq!(nachher["nachholen"]["saetze"], 3, "{nachher}");
    assert_eq!(nachher["nachholen"]["gebremst"], 0);
    assert_eq!(nachher["nachholen"]["bank_belegt"], 0);
    let werkbank = nachher["werkbank_plaetze"]
        .as_u64()
        .expect("werkbank_plaetze");
    assert_eq!(
        nachher["nachholen"]["bank_plaetze"].as_u64(),
        Some((werkbank / 2).max(1)),
        "{nachher}"
    );
}
