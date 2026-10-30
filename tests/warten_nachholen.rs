//! F0170 Nachholen bleibt begrenzt über das echte Programm (R00049, ADR-0054 Ergänzung „Nachholen von `warten` …“
//! Regel 1 und 8, B0758): `/api/gesundheit` nennt `nachholen {saetze, kosten, gebremst, bank_plaetze, bank_belegt}`,
//! die Nachholbank hat die Hälfte der Kerne (mindestens 1) neben der Werkbank, und ein Nachholen zählt seine Sätze
//! und seit F0171 (B0762) deren Kosten.
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
    assert_eq!(
        nachher["nachholen"]["kosten"], 3,
        "kurze Sätze: Kosten gleich Sätze: {nachher}"
    );
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

/// K3 (B0762, K6): ein Satz zu 16 KiB (Text 16.384 Byte ohne Zeilenumbruch) kostet 17 — `nachholen.saetze` zählt
/// den einen Satz, `nachholen.kosten` die 17, und `kosten` ≥ `saetze`.
#[tokio::test]
async fn die_gesundheit_zaehlt_die_kosten_eines_langen_satzes_getrennt_von_den_saetzen() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[]);
    let absender = Absender::von("127.0.0.1");
    let anna = beitreten(&absender, &instanz.adresse, "anna", None).await;
    let claude = beitreten(&absender, &instanz.adresse, "claude", None).await;
    let lang = "x".repeat(16_384);
    let antwort = senden(&absender, &instanz.adresse, &anna, "allgemein", &lang).await;
    assert_eq!(antwort.status, 201, "{}", antwort.json);

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
    assert_eq!(nachher["nachholen"]["saetze"], 1, "{nachher}");
    assert_eq!(nachher["nachholen"]["kosten"], 17, "{nachher}");
}

/// Nacharbeit security-reviewer #17 Befund 1, Probe (3): sechsmal kurz senden und auf 16.384 Byte bearbeiten, dann
/// `warten?bis=0&seit=2000-01-01T00:00:00Z&channel=<c>` → `nachholen.kosten` steigt um 102 (6 × 17), nicht um 6 —
/// gezählt wird der Änderungssatz, dessen Text ausgewertet wird.
#[tokio::test]
async fn bearbeitete_nachrichten_kosten_nach_ihrem_aenderungssatz() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[]);
    let absender = Absender::von("127.0.0.1");
    let anna = beitreten(&absender, &instanz.adresse, "anna", None).await;
    let claude = beitreten(&absender, &instanz.adresse, "claude", None).await;
    let lang = "x".repeat(16_384);
    for nummer in 1..=6 {
        let gesendet = senden(
            &absender,
            &instanz.adresse,
            &anna,
            "allgemein",
            &format!("k{nummer}"),
        )
        .await;
        assert_eq!(gesendet.status, 201, "{}", gesendet.json);
        let zeit = gesendet.json["nachricht"]["zeit"].as_str().expect("zeit");
        let bearbeitet = api_antwort(
            absender
                .anfrage(
                    reqwest::Method::POST,
                    format!(
                        "{}api/channels/allgemein/nachrichten/bearbeiten",
                        instanz.adresse
                    ),
                )
                .header("X-LocalchatA-Sitzung", &anna)
                .json(&serde_json::json!({ "zeit": zeit, "text": lang })),
        )
        .await;
        assert_eq!(bearbeitet.status, 200, "{}", bearbeitet.json);
    }
    let vorher = gesundheit(&instanz.adresse).await.1;

    let gewartet = api_antwort(
        absender
            .anfrage(
                reqwest::Method::GET,
                format!(
                    "{}api/warten?bis=0&seit=2000-01-01T00:00:00Z&channel=allgemein",
                    instanz.adresse
                ),
            )
            .header("X-LocalchatA-Sitzung", claude),
    )
    .await;
    let nachher = gesundheit(&instanz.adresse).await.1;

    assert_eq!(gewartet.status, 200, "{}", gewartet.json);
    let gestiegen = |feld: &str| {
        nachher["nachholen"][feld].as_u64().expect(feld)
            - vorher["nachholen"][feld].as_u64().expect(feld)
    };
    assert_eq!(gestiegen("saetze"), 6, "{vorher} → {nachher}");
    assert_eq!(gestiegen("kosten"), 102, "{vorher} → {nachher}");
}
