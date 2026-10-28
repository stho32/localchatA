//! F0019 Verlassen über die API — über das echte Programm (R00005, US-15 bis US-17; ADR-0018).
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::strom::{Strom, ist_wechsel};
use gemeinsam::{Absender, ApiAntwort, Instanz, wegwerf_starten};
use serde_json::{Value, json};
use std::time::Duration;
use tokio::time::Instant;

const NULL_TOKEN: &str = "00000000000000000000000000000000";

async fn beitreten(absender: &Absender, instanz: &Instanz, name: &str) -> ApiAntwort {
    absender
        .beitreten(&instanz.adresse, json!({ "name": name }), None)
        .await
}

async fn token(absender: &Absender, instanz: &Instanz, name: &str) -> String {
    let antwort = beitreten(absender, instanz, name).await;
    assert_eq!(antwort.status, 201, "{}", antwort.json);
    antwort.json["token"].as_str().expect("Token").to_string()
}

/// `POST /api/verlassen` von `absender`, mit dem Sitzungs-Header (wenn gegeben) und weiteren Kopfzeilen.
async fn verlassen(
    absender: &Absender,
    instanz: &Instanz,
    sitzung: Option<&str>,
    kopf: &[(&str, &str)],
) -> ApiAntwort {
    let mut anfrage = absender.anfrage(
        reqwest::Method::POST,
        format!("{}api/verlassen", instanz.adresse),
    );
    if let Some(sitzung) = sitzung {
        anfrage = anfrage.header("X-LocalchatA-Sitzung", sitzung);
    }
    for (name, wert) in kopf {
        anfrage = anfrage.header(*name, *wert);
    }
    gemeinsam::api_antwort(anfrage).await
}

async fn kennungen(absender: &Absender, instanz: &Instanz, token: &str) -> Vec<Value> {
    let antwort = gemeinsam::api_antwort(
        absender
            .anfrage(
                reqwest::Method::GET,
                format!("{}api/teilnehmer", instanz.adresse),
            )
            .header("X-LocalchatA-Sitzung", token),
    )
    .await;
    assert_eq!(antwort.status, 200, "{}", antwort.json);
    antwort.json["liste"]
        .as_array()
        .expect("liste")
        .iter()
        .map(|zeile| zeile["kennung"].clone())
        .collect()
}

fn kopfwert<'a>(antwort: &'a ApiAntwort, name: &str) -> Option<&'a str> {
    antwort.kopf.get(name).and_then(|wert| wert.to_str().ok())
}

/// US-15: `alpha` hält einen Strom, `beta` liest und hat `alpha` online gelesen; `alpha` verlässt → 204 mit
/// Header-Satz und `no-store`; `beta` liest `weg` spätestens 1 s nach der 204, ohne `offline` davor; ein
/// Beitritt `alpha` von `127.0.0.4` direkt danach ergibt 201.
#[tokio::test]
async fn us15_verlassen_gibt_den_namen_sofort_frei() {
    let instanz = wegwerf_starten();
    let (zwei, drei, vier) = (
        Absender::von("127.0.0.2"),
        Absender::von("127.0.0.3"),
        Absender::von("127.0.0.4"),
    );
    let beta = token(&drei, &instanz, "beta").await;
    let mut strom_beta = Strom::oeffnen(&drei, &instanz.adresse, &beta).await;
    let alpha = token(&zwei, &instanz, "alpha").await;
    let _strom_alpha = Strom::oeffnen(&zwei, &instanz.adresse, &alpha).await;
    strom_beta
        .bis(
            Duration::from_secs(5),
            ist_wechsel("alpha@127.0.0.2", "online"),
        )
        .await
        .expect("beta hat alpha online gelesen");

    let antwort = verlassen(&zwei, &instanz, Some(&alpha), &[]).await;
    let erhalten = Instant::now();
    let neu = beitreten(&vier, &instanz, "alpha").await;
    let weg = strom_beta
        .bis(
            Duration::from_secs(5),
            ist_wechsel("alpha@127.0.0.2", "weg"),
        )
        .await
        .expect("beta liest alpha weg");

    assert_eq!(antwort.status, 204, "{}", antwort.json);
    assert_eq!(kopfwert(&antwort, "cache-control"), Some("no-store"));
    assert_eq!(
        kopfwert(&antwort, "x-content-type-options"),
        Some("nosniff")
    );
    assert!(kopfwert(&antwort, "content-security-policy").is_some());
    assert_eq!(neu.status, 201, "{}", neu.json);
    let zustellung = weg.ankunft.saturating_duration_since(erhalten);
    eprintln!("messung us15_weg_nach_204_ms={}", zustellung.as_millis());
    assert!(zustellung <= Duration::from_secs(1), "{zustellung:?}");
    let alpha_wechsel: Vec<String> = strom_beta
        .wechsel_von("alpha@127.0.0.2")
        .iter()
        .map(|e| {
            e.data["teilnehmer"]["status"]
                .as_str()
                .unwrap_or_default()
                .to_string()
        })
        .collect();
    assert_eq!(alpha_wechsel, ["online", "weg"], "ohne offline davor");
}

/// US-16: ohne Header → 401 `{fehler}`; mit dem Header von `alpha` und fremdem `Origin` → 403; `alpha` lebt.
#[tokio::test]
async fn us16_ohne_header_oder_von_fremder_seite_kein_verlassen() {
    let instanz = wegwerf_starten();
    let (zwei, drei) = (Absender::von("127.0.0.2"), Absender::von("127.0.0.3"));
    let alpha = token(&zwei, &instanz, "alpha").await;
    let beta = token(&drei, &instanz, "beta").await;

    let ohne = verlassen(&zwei, &instanz, None, &[]).await;
    let fremd = verlassen(
        &zwei,
        &instanz,
        Some(&alpha),
        &[("Origin", "http://evil.example")],
    )
    .await;

    assert_eq!(ohne.status, 401);
    assert!(ohne.json["fehler"].is_string(), "{}", ohne.json);
    assert_eq!(fremd.status, 403);
    assert!(
        kennungen(&drei, &instanz, &beta)
            .await
            .contains(&json!("alpha@127.0.0.2"))
    );
}

/// US-17: das Token von `alpha` von `127.0.0.3` und ein unbekanntes Token → je 204 ohne Wirkung; `alpha` lebt,
/// ein Beitritt `alpha` von `127.0.0.4` ergibt 409.
#[tokio::test]
async fn us17_fremde_und_unbekannte_tokens_bewirken_nichts_und_verraten_nichts() {
    let instanz = wegwerf_starten();
    let (zwei, drei, vier) = (
        Absender::von("127.0.0.2"),
        Absender::von("127.0.0.3"),
        Absender::von("127.0.0.4"),
    );
    let alpha = token(&zwei, &instanz, "alpha").await;
    let beta = token(&drei, &instanz, "beta").await;

    let fremd = verlassen(&drei, &instanz, Some(&alpha), &[]).await;
    let unbekannt = verlassen(&zwei, &instanz, Some(NULL_TOKEN), &[]).await;
    let unlesbar = verlassen(&zwei, &instanz, Some("kein-token"), &[]).await;
    let zweiter = beitreten(&vier, &instanz, "alpha").await;

    assert_eq!(
        (fremd.status, unbekannt.status, unlesbar.status),
        (204, 204, 204)
    );
    assert!(
        kennungen(&drei, &instanz, &beta)
            .await
            .contains(&json!("alpha@127.0.0.2"))
    );
    assert_eq!(zweiter.status, 409, "{}", zweiter.json);
}

/// Kriterium „Ohne Wirkung“: nach einem erfolgreichen `verlassen` ein zweites mit demselben Token → 204, kein
/// weiterer Wechsel.
#[tokio::test]
async fn ein_zweites_verlassen_ist_204_ohne_weiteren_wechsel() {
    let instanz = wegwerf_starten();
    let (zwei, drei) = (Absender::von("127.0.0.2"), Absender::von("127.0.0.3"));
    let beta = token(&drei, &instanz, "beta").await;
    let mut strom_beta = Strom::oeffnen(&drei, &instanz.adresse, &beta).await;
    let alpha = token(&zwei, &instanz, "alpha").await;

    let erstes = verlassen(&zwei, &instanz, Some(&alpha), &[]).await;
    strom_beta
        .bis(
            Duration::from_secs(5),
            ist_wechsel("alpha@127.0.0.2", "weg"),
        )
        .await
        .expect("weg");
    let staende_davor = strom_beta.staende();
    let zweites = verlassen(&zwei, &instanz, Some(&alpha), &[]).await;
    strom_beta.mitlesen(Duration::from_millis(1500)).await;

    assert_eq!((erstes.status, zweites.status), (204, 204));
    let neu: Vec<u64> = strom_beta.staende()[staende_davor.len()..].to_vec();
    assert_eq!(neu, Vec::<u64>::new(), "kein weiterer Wechsel");
}
