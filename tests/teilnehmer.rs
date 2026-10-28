//! F0011 Teilnehmerliste über die API abfragen — über das echte Programm (R00004, US-1 bis US-4).
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::{Absender, ApiAntwort, api_antwort, wegwerf_starten};
use serde_json::json;

fn token_aus(antwort: &ApiAntwort) -> String {
    antwort.json["token"].as_str().expect("Token").to_string()
}

async fn liste(absender: &Absender, basis: &str, token: Option<&str>) -> ApiAntwort {
    let mut anfrage = absender.anfrage(reqwest::Method::GET, format!("{basis}api/teilnehmer"));
    if let Some(token) = token {
        anfrage = anfrage.header("X-LocalchatA-Sitzung", token);
    }
    api_antwort(anfrage).await
}

#[tokio::test]
async fn us1_liste_mit_allen_teilnehmern_nach_kennung() {
    let instanz = wegwerf_starten();
    let (zwei, drei) = (Absender::von("127.0.0.2"), Absender::von("127.0.0.3"));
    zwei.beitreten(&instanz.adresse, json!({"name": "alpha"}), None)
        .await;
    let beta = token_aus(
        &drei
            .beitreten(&instanz.adresse, json!({"name": "beta"}), None)
            .await,
    );

    let antwort = liste(&drei, &instanz.adresse, Some(&beta)).await;

    assert_eq!(antwort.status, 200, "{}", antwort.json);
    assert_eq!(antwort.kopf["cache-control"], "no-store");
    assert_eq!(antwort.kopf["x-content-type-options"], "nosniff");
    assert!(antwort.json["stand"].as_u64().expect("stand") >= 2);
    let eintraege = antwort.json["liste"].as_array().expect("liste");
    let kennungen: Vec<&str> = eintraege
        .iter()
        .map(|eintrag| eintrag["kennung"].as_str().expect("kennung"))
        .collect();
    assert_eq!(kennungen, ["alpha@127.0.0.2", "beta@127.0.0.3"]);
    for (eintrag, (name, herkunft)) in eintraege
        .iter()
        .zip([("alpha", "127.0.0.2"), ("beta", "127.0.0.3")])
    {
        assert_eq!(eintrag["name"], name);
        assert_eq!(eintrag["herkunft"], herkunft);
        assert_eq!(eintrag["art"], "agent");
        assert!(
            ["online", "offline"].contains(&eintrag["status"].as_str().expect("status")),
            "{eintrag}"
        );
        let seit = eintrag["seit"].as_str().expect("seit");
        chrono::DateTime::parse_from_rfc3339(seit).expect("seit nach RFC 3339");
    }
}

#[tokio::test]
async fn us3_ohne_gueltige_sitzung_keine_liste() {
    let instanz = wegwerf_starten();
    let (zwei, drei) = (Absender::von("127.0.0.2"), Absender::von("127.0.0.3"));
    let alpha = token_aus(
        &zwei
            .beitreten(&instanz.adresse, json!({"name": "alpha"}), None)
            .await,
    );

    let ohne = liste(&drei, &instanz.adresse, None).await;
    let null = liste(
        &drei,
        &instanz.adresse,
        Some("00000000000000000000000000000000"),
    )
    .await;
    let fremd = liste(&drei, &instanz.adresse, Some(&alpha)).await;

    for antwort in [ohne, null, fremd] {
        assert_eq!(antwort.status, 401, "{}", antwort.json);
        assert!(antwort.json["fehler"].is_string(), "{}", antwort.json);
    }
}

fn eintrag<'a>(antwort: &'a ApiAntwort, kennung: &str) -> Option<&'a serde_json::Value> {
    antwort.json["liste"]
        .as_array()
        .expect("liste")
        .iter()
        .find(|eintrag| eintrag["kennung"] == kennung)
}

fn zeit(eintrag: &serde_json::Value) -> chrono::DateTime<chrono::FixedOffset> {
    chrono::DateTime::parse_from_rfc3339(eintrag["seit"].as_str().expect("seit")).expect("RFC 3339")
}

/// US-2, Rechenbeispiel mit echter Zeit: Beitritt bei t0 → bei t0 + 0,5 s online seit t0, bei t0 + 1,5 s
/// offline seit t0 + 1 s, und der Stand ist gewachsen.
#[tokio::test]
async fn us2_wer_keinen_strom_offen_hat_steht_nach_einer_sekunde_als_offline() {
    let instanz = wegwerf_starten();
    let (zwei, drei) = (Absender::von("127.0.0.2"), Absender::von("127.0.0.3"));
    let beta = token_aus(
        &drei
            .beitreten(&instanz.adresse, json!({"name": "beta"}), None)
            .await,
    );
    let start = tokio::time::Instant::now();
    zwei.beitreten(&instanz.adresse, json!({"name": "alpha"}), None)
        .await;

    tokio::time::sleep_until(start + std::time::Duration::from_millis(500)).await;
    let frueh = liste(&drei, &instanz.adresse, Some(&beta)).await;
    tokio::time::sleep_until(start + std::time::Duration::from_millis(1500)).await;
    let spaet = liste(&drei, &instanz.adresse, Some(&beta)).await;

    let alpha_frueh = eintrag(&frueh, "alpha@127.0.0.2").expect("alpha bei 0,5 s");
    let alpha_spaet = eintrag(&spaet, "alpha@127.0.0.2").expect("alpha bei 1,5 s");
    assert_eq!(alpha_frueh["status"], "online", "{alpha_frueh}");
    assert_eq!(alpha_spaet["status"], "offline", "{alpha_spaet}");
    assert_eq!(
        zeit(alpha_spaet) - zeit(alpha_frueh),
        chrono::Duration::seconds(1),
        "offline seit Beitritt + 1 s"
    );
    assert!(spaet.json["stand"].as_u64() > frueh.json["stand"].as_u64());
}

/// US-4 mit `LOCALCHATA_GNADENFRIST_MS=3000`: bei 1 s steht alpha offline, bei 4 s fehlt alpha
/// (4 s > 3 s), und der Name ist frei.
#[tokio::test]
async fn us4_wer_sich_nicht_mehr_meldet_verschwindet_aus_der_liste() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let instanz = gemeinsam::starten_mit(
        wurzel.path(),
        &["--port", "0", "--host", "127.0.0.1", "--kein-browser"],
        &[("LOCALCHATA_GNADENFRIST_MS", "3000")],
    );
    let (zwei, drei) = (Absender::von("127.0.0.2"), Absender::von("127.0.0.3"));
    let start = tokio::time::Instant::now();
    zwei.beitreten(&instanz.adresse, json!({"name": "alpha"}), None)
        .await;
    let beta = token_aus(
        &drei
            .beitreten(&instanz.adresse, json!({"name": "beta"}), None)
            .await,
    );

    tokio::time::sleep_until(start + std::time::Duration::from_millis(1200)).await;
    let bei_1 = liste(&drei, &instanz.adresse, Some(&beta)).await;
    tokio::time::sleep_until(start + std::time::Duration::from_secs(4)).await;
    let bei_4 = liste(&drei, &instanz.adresse, Some(&beta)).await;
    let neu = drei
        .beitreten(&instanz.adresse, json!({"name": "alpha"}), None)
        .await;

    let alpha = eintrag(&bei_1, "alpha@127.0.0.2").expect("alpha bei 1 s");
    assert_eq!(alpha["status"], "offline", "{alpha}");
    assert!(
        eintrag(&bei_4, "alpha@127.0.0.2").is_none(),
        "{}",
        bei_4.json
    );
    assert_eq!(neu.status, 201, "{}", neu.json);
}
