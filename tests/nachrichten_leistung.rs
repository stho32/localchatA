//! F0027 mit 100.000 Nachrichten in einem Channel über 7 Tage (R00007, US-3, Kriterien „100.000 lückenlos“ und
//! L1): Daten aus dem Testdaten-Erzeuger, echtes Programm im Debug-Build. Die Grenze L1 ist die der Anforderung
//! (50 ms, erwartet 2–10 ms); gemessen wird je Anfrage vom Senden bis zum vollständig gelesenen Inhalt.
#![cfg(target_os = "linux")]

mod gemeinsam;

use chrono::{DateTime, Local, Utc};
use gemeinsam::{Absender, Instanz, starten_mit};
use localchata::testdaten::verlauf_erzeugen;
use localchata::uhr::Zeit;
use serde_json::{Value, json};
use std::time::{Duration, Instant};

const JETZT: &str = "2026-10-28T15:00:00.000+01:00";
const GRENZE_L1: Duration = Duration::from_millis(50);

fn ortszeit(zeitpunkt: DateTime<Utc>) -> Zeit {
    zeitpunkt.with_timezone(&Local).fixed_offset()
}

async fn gross_gegeben() -> (tempfile::TempDir, Instanz, Absender, String) {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let jetzt = Zeit::parse_from_rfc3339(JETZT).expect("jetzt");
    verlauf_erzeugen(wurzel.path(), "gross", 100_000, 7, jetzt, ortszeit).expect("erzeugt");
    let instanz = starten_mit(
        wurzel.path(),
        &["--port", "0", "--host", "127.0.0.1", "--kein-browser"],
        &[("LOCALCHATA_JETZT", JETZT), ("TZ", "Europe/Berlin")],
    );
    let beta = Absender::von("127.0.0.3");
    let beitritt = beta
        .beitreten(&instanz.adresse, json!({ "name": "beta" }), None)
        .await;
    let token = beitritt.json["token"].as_str().expect("Token").to_string();
    (wurzel, instanz, beta, token)
}

/// Ein Block und die Dauer vom Senden bis zum vollständig gelesenen Inhalt.
async fn block(
    beta: &Absender,
    instanz: &Instanz,
    token: &str,
    abfrage: &str,
) -> (Value, Duration) {
    let url = format!("{}api/channels/gross/nachrichten{abfrage}", instanz.adresse);
    let beginn = Instant::now();
    let antwort = beta
        .anfrage(reqwest::Method::GET, url)
        .header("X-LocalchatA-Sitzung", token)
        .send()
        .await
        .expect("Antwort");
    let status = antwort.status();
    let text = antwort.text().await.expect("Inhalt");
    let dauer = beginn.elapsed();
    assert_eq!(status, 200, "{text}");
    (serde_json::from_str(&text).expect("JSON"), dauer)
}

fn zeiten(block: &Value) -> Vec<String> {
    block["nachrichten"]
        .as_array()
        .expect("nachrichten")
        .iter()
        .map(|nachricht| nachricht["zeit"].as_str().expect("zeit").to_string())
        .collect()
}

fn kodiert(zeit: &str) -> String {
    zeit.replace('+', "%2B")
}

/// Alle Zeiten des Channels, rückwärts in Blöcken zu 100 gelesen, älteste zuerst; dazu die Zahl der Abrufe.
async fn alle_rueckwaerts(
    beta: &Absender,
    instanz: &Instanz,
    token: &str,
) -> (Vec<String>, usize, Value) {
    let (mut letzter, _) = block(beta, instanz, token, "").await;
    let mut bloecke = vec![zeiten(&letzter)];
    let mut abrufe = 1;
    while letzter["aelter"] == true {
        let erste = bloecke.last().expect("Block")[0].clone();
        letzter = block(beta, instanz, token, &format!("?vor={}", kodiert(&erste)))
            .await
            .0;
        bloecke.push(zeiten(&letzter));
        abrufe += 1;
        assert!(abrufe <= 1_001, "endet nicht");
    }
    let alle = bloecke.into_iter().rev().flatten().collect();
    (alle, abrufe, letzter)
}

#[tokio::test]
async fn us3_tausend_bloecke_rueckwaerts_ergeben_jede_zeit_genau_einmal() {
    let (_wurzel, instanz, beta, token) = gross_gegeben().await;

    let (neueste, _) = block(&beta, &instanz, &token, "").await;
    let (alle, abrufe, letzter) = alle_rueckwaerts(&beta, &instanz, &token).await;

    assert_eq!(zeiten(&neueste).len(), 100);
    assert_eq!(neueste["aelter"], true);
    assert_eq!(neueste["neuer"], false);
    assert_eq!(abrufe, 1_000);
    assert_eq!(letzter["aelter"], false);
    assert_eq!(alle.len(), 100_000);
    let zeitpunkte: Vec<Zeit> = alle
        .iter()
        .map(|zeit| Zeit::parse_from_rfc3339(zeit).expect("Zeit"))
        .collect();
    assert!(
        zeitpunkte.windows(2).all(|paar| paar[0] < paar[1]),
        "streng steigend, ohne Doppel"
    );
    assert_eq!(
        alle.last(),
        zeiten(&neueste).last(),
        "die letzte ist die jüngste"
    );
}

#[tokio::test]
async fn l1_ein_block_dauert_unter_der_grenze_neueste_und_mitten_im_verlauf() {
    let (_wurzel, instanz, beta, token) = gross_gegeben().await;
    let (alle, _, _) = alle_rueckwaerts(&beta, &instanz, &token).await;
    let mitte = format!("?vor={}", kodiert(&alle[49_999]));

    let mut groesste = Vec::new();
    for abfrage in ["", mitte.as_str()] {
        let mut dauern = Vec::new();
        for _ in 0..20 {
            let (antwort, dauer) = block(&beta, &instanz, &token, abfrage).await;
            assert_eq!(zeiten(&antwort).len(), 100);
            dauern.push(dauer);
        }
        dauern.sort();
        println!(
            "messung l1{abfrage:.4}: min {:?} median {:?} max {:?}",
            dauern[0], dauern[10], dauern[19]
        );
        groesste.push(dauern[19]);
    }

    for dauer in groesste {
        assert!(dauer < GRENZE_L1, "L1: {dauer:?} ≥ {GRENZE_L1:?}");
    }
}
