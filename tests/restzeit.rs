//! F0017 Restzeit über API und Strom — über das echte Programm (R00005, US-5 bis US-9; ADR-0019 Regeln 1–3).
//! Regeln werden über die Serverzeiten `seit`, `weg_um` und `jetzt` geprüft, Zustellung als obere Grenze
//! zwischen zwei Ankünften beim selben Beobachter (X00007).
#![cfg(target_os = "linux")]

mod gemeinsam;

use chrono::{DateTime, FixedOffset};
use gemeinsam::strom::{Empfangen, Strom, ist_wechsel};
use gemeinsam::{Absender, Instanz, starten_mit, wegwerf_starten};
use serde_json::{Value, json};
use std::time::Duration;

const LOKAL: &[&str] = &["--port", "0", "--host", "127.0.0.1", "--kein-browser"];

async fn beitreten(absender: &Absender, instanz: &Instanz, name: &str) -> String {
    let antwort = absender
        .beitreten(&instanz.adresse, json!({ "name": name }), None)
        .await;
    assert_eq!(antwort.status, 201, "{}", antwort.json);
    antwort.json["token"].as_str().expect("Token").to_string()
}

async fn teilnehmer(absender: &Absender, instanz: &Instanz, token: &str) -> Value {
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
    antwort.json
}

fn zeit(wert: &Value) -> DateTime<FixedOffset> {
    let text = wert
        .as_str()
        .unwrap_or_else(|| panic!("Zeit erwartet: {wert}"));
    DateTime::parse_from_rfc3339(text).unwrap_or_else(|_| panic!("kein RFC 3339: {text}"))
}

fn millisekunden_zwischen(frueher: &Value, spaeter: &Value) -> i64 {
    (zeit(spaeter) - zeit(frueher)).num_milliseconds()
}

fn zeile_von<'a>(liste: &'a Value, kennung: &str) -> &'a Value {
    liste
        .as_array()
        .expect("liste")
        .iter()
        .find(|zeile| zeile["kennung"] == kennung)
        .unwrap_or_else(|| panic!("{kennung} fehlt in {liste}"))
}

/// US-5 (Kriterium „Liste“): `alpha` schweigt, `beta` fragt 2 s nach dem Beitritt von `alpha` die Liste ab →
/// `jetzt` in der Antwort; `alpha` `offline` mit `weg_um − seit` = 59 000 ms genau; online-Zeilen `weg_um` null.
#[tokio::test]
async fn us5_die_liste_nennt_jetzt_und_wann_alpha_weg_ist() {
    let instanz = wegwerf_starten();
    let (zwei, drei) = (Absender::von("127.0.0.2"), Absender::von("127.0.0.3"));
    let beta = beitreten(&drei, &instanz, "beta").await;
    let _strom_beta = Strom::oeffnen(&drei, &instanz.adresse, &beta).await;
    beitreten(&zwei, &instanz, "alpha").await;
    tokio::time::sleep(Duration::from_secs(2)).await;

    let antwort = teilnehmer(&drei, &instanz, &beta).await;

    zeit(&antwort["jetzt"]);
    let alpha = zeile_von(&antwort["liste"], "alpha@127.0.0.2");
    assert_eq!(alpha["status"], "offline", "{antwort}");
    assert_eq!(
        millisekunden_zwischen(&alpha["seit"], &alpha["weg_um"]),
        59_000,
        "{antwort}"
    );
    let online: Vec<&Value> = antwort["liste"]
        .as_array()
        .expect("liste")
        .iter()
        .filter(|zeile| zeile["status"] == "online")
        .collect();
    assert!(!online.is_empty(), "beta hält einen Strom: {antwort}");
    for zeile in online {
        assert_eq!(zeile.get("weg_um"), Some(&Value::Null), "{zeile}");
    }
}

/// US-6 (Kriterium „Strom“): `alpha` schließt seinen Strom → der Wechsel `offline` trägt `jetzt` und `weg_um`,
/// `weg_um − seit` = 59 000 ms genau; Stand und jeder Wechsel tragen `jetzt`.
#[tokio::test]
async fn us6_der_wechsel_im_strom_traegt_jetzt_und_die_restzeit() {
    let instanz = wegwerf_starten();
    let (zwei, drei) = (Absender::von("127.0.0.2"), Absender::von("127.0.0.3"));
    let beta = beitreten(&drei, &instanz, "beta").await;
    let mut strom_beta = Strom::oeffnen(&drei, &instanz.adresse, &beta).await;
    let alpha = beitreten(&zwei, &instanz, "alpha").await;
    let strom_alpha = Strom::oeffnen(&zwei, &instanz.adresse, &alpha).await;
    strom_beta
        .bis(
            Duration::from_secs(5),
            ist_wechsel("alpha@127.0.0.2", "online"),
        )
        .await
        .expect("alpha online");

    drop(strom_alpha);
    let offline = strom_beta
        .bis(
            Duration::from_secs(5),
            ist_wechsel("alpha@127.0.0.2", "offline"),
        )
        .await
        .expect("alpha offline");

    let zeile = &offline.data["teilnehmer"];
    zeit(&offline.data["jetzt"]);
    assert_eq!(
        millisekunden_zwischen(&zeile["seit"], &zeile["weg_um"]),
        59_000,
        "{}",
        offline.data
    );
    let gelesen: Vec<&Empfangen> = strom_beta
        .ereignisse
        .iter()
        .filter(|e| e.event.is_empty())
        .collect();
    assert_eq!(gelesen[0].data["art"], "teilnehmer");
    for ereignis in gelesen {
        zeit(&ereignis.data["jetzt"]);
    }
}

/// US-7 (Kriterium „Weg nach der Restzeit“, `LOCALCHATA_GNADENFRIST_MS=3000`): `offline` mit
/// `weg_um − seit` = 2000 ms, danach `weg` mit `seit` > `weg_um`; Ankunft(weg) − Ankunft(offline) bei `beta`
/// ≤ 3 s (Regel 2 s plus 1 s Zustellung; erwartet etwa 2,1 s).
#[tokio::test]
async fn us7_weg_genau_nach_der_restzeit() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let instanz = starten_mit(
        wurzel.path(),
        LOKAL,
        &[("LOCALCHATA_GNADENFRIST_MS", "3000")],
    );
    let (zwei, drei) = (Absender::von("127.0.0.2"), Absender::von("127.0.0.3"));
    let beta = beitreten(&drei, &instanz, "beta").await;
    let mut strom_beta = Strom::oeffnen(&drei, &instanz.adresse, &beta).await;
    let alpha = beitreten(&zwei, &instanz, "alpha").await;
    let strom_alpha = Strom::oeffnen(&zwei, &instanz.adresse, &alpha).await;
    strom_beta
        .bis(
            Duration::from_secs(5),
            ist_wechsel("alpha@127.0.0.2", "online"),
        )
        .await
        .expect("alpha online");

    drop(strom_alpha);
    let offline = strom_beta
        .bis(
            Duration::from_secs(5),
            ist_wechsel("alpha@127.0.0.2", "offline"),
        )
        .await
        .expect("alpha offline");
    let weg = strom_beta
        .bis(
            Duration::from_secs(10),
            ist_wechsel("alpha@127.0.0.2", "weg"),
        )
        .await
        .expect("alpha weg");

    let offline_zeile = &offline.data["teilnehmer"];
    assert_eq!(
        millisekunden_zwischen(&offline_zeile["seit"], &offline_zeile["weg_um"]),
        2000,
        "{}",
        offline.data
    );
    assert!(
        zeit(&weg.data["teilnehmer"]["seit"]) > zeit(&offline_zeile["weg_um"]),
        "{} nach {}",
        weg.data,
        offline.data
    );
    let zustellung = weg.ankunft - offline.ankunft;
    eprintln!("messung us7_offline_bis_weg_ms={}", zustellung.as_millis());
    assert!(zustellung <= Duration::from_secs(3), "{zustellung:?}");
}

/// US-8 (Kriterium „Herzschlag“, `LOCALCHATA_HERZSCHLAG_MS=500`): in 2 s mindestens 2 Herzschläge (erwartet 4),
/// jeder mit `data: {"jetzt": …}`, kein `jetzt` kleiner als das vorige.
#[tokio::test]
async fn us8_der_herzschlag_traegt_die_serverzeit() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let instanz = starten_mit(wurzel.path(), LOKAL, &[("LOCALCHATA_HERZSCHLAG_MS", "500")]);
    let zwei = Absender::von("127.0.0.2");
    let alpha = beitreten(&zwei, &instanz, "alpha").await;
    let mut strom = Strom::oeffnen(&zwei, &instanz.adresse, &alpha).await;

    strom.mitlesen(Duration::from_secs(2)).await;

    let zeiten: Vec<DateTime<FixedOffset>> = strom
        .ereignisse
        .iter()
        .filter(|e| e.event == "herzschlag")
        .map(|e| zeit(&e.data["jetzt"]))
        .collect();
    assert!(zeiten.len() >= 2, "{:?}", strom.ereignisse);
    assert!(
        zeiten.windows(2).all(|paar| paar[0] <= paar[1]),
        "{zeiten:?}"
    );
}
