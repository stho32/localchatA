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
