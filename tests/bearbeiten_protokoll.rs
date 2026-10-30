//! R00039 F0132 „Betriebsprotokoll“ (ADR-0048 Regel 13, Bereich `aendern`): das echte Programm mit
//! `LOCALCHATA_PROTOKOLL_STUFE=debug` und `LOCALCHATA_EPISODE_RUHE_MS=200`; nie Text.

mod gemeinsam;

use gemeinsam::protokoll::{alle_zeilen, mit, zeilen};
use gemeinsam::senden::{HEUTE, beitreten, ordner, senden, starten};
use gemeinsam::{Absender, ApiAntwort, api_antwort};
use serde_json::json;
use std::path::Path;
use std::time::{Duration, Instant};

const UMGEBUNG: [(&str, &str); 2] = [
    ("LOCALCHATA_PROTOKOLL_STUFE", "debug"),
    ("LOCALCHATA_EPISODE_RUHE_MS", "200"),
];

async fn bearbeiten(
    absender: &Absender,
    basis: &str,
    token: &str,
    zeit: &str,
    text: &str,
    schluessel: Option<&str>,
) -> ApiAntwort {
    let mut anfrage = absender
        .anfrage(
            reqwest::Method::POST,
            format!("{basis}api/channels/allgemein/nachrichten/bearbeiten"),
        )
        .header("X-LocalchatA-Sitzung", token)
        .json(&json!({ "zeit": zeit, "text": text }));
    if let Some(schluessel) = schluessel {
        anfrage = anfrage.header("Idempotency-Key", schluessel);
    }
    api_antwort(anfrage).await
}

/// Wartet höchstens 5 s, bis `zahl` Zeilen von `logs/<heute>.log` `passen`.
async fn warten_auf(ordner: &Path, zahl: usize, passt: impl Fn(&str) -> bool) -> Vec<String> {
    let beginn = Instant::now();
    loop {
        let treffer: Vec<String> = zeilen(ordner, HEUTE)
            .into_iter()
            .filter(|zeile| passt(zeile))
            .collect();
        if treffer.len() >= zahl || beginn.elapsed() > Duration::from_secs(5) {
            return treffer;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

/// R00039 „Betriebsprotokoll“: der Rundweg schreibt `DEBUG aendern Nachricht bearbeitet` mit Channel, Kennung,
/// `anzahl=14` und `dauer_ms`; unverändert und Wiederholung tragen `grund=`; drei 403 von `gamma` → eine Beginn- und
/// eine Ende-Zeile mit 3; fünf Kennungen mit je einem 403 → fünf Episoden (M16); der Text steht nirgends.
#[tokio::test]
async fn bearbeiten_im_betriebsprotokoll() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &UMGEBUNG);
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, basis, "beta", Some("agent")).await;
    let gesendet = senden(&beta, basis, &token, "allgemein", "Hallo").await;
    let t1 = gesendet.json["nachricht"]["zeit"]
        .as_str()
        .expect("zeit")
        .to_string();

    let rundweg = bearbeiten(&beta, basis, &token, &t1, "Hallo zusammen", Some("k1")).await;
    let wiederholt = bearbeiten(&beta, basis, &token, &t1, "Hallo zusammen", Some("k1")).await;
    let unveraendert = bearbeiten(&beta, basis, &token, &t1, "Hallo zusammen", None).await;
    let gamma = beitreten(&beta, basis, "gamma", None).await;
    for _ in 0..3 {
        let antwort = bearbeiten(&beta, basis, &gamma, &t1, "gefälscht", None).await;
        assert_eq!(antwort.status, 403, "{}", antwort.json);
    }
    for nummer in 1..=5 {
        let fremd = beitreten(&beta, basis, &format!("fremd{nummer}"), None).await;
        let antwort = bearbeiten(&beta, basis, &fremd, &t1, "gefälscht", None).await;
        assert_eq!(antwort.status, 403, "{}", antwort.json);
    }
    let fremd_episoden = warten_auf(wurzel.path(), 12, |zeile| {
        zeile.contains(" WARNING aendern Ändern fremder Nachricht abgewiesen — Episode ")
    })
    .await;

    for antwort in [&rundweg, &wiederholt, &unveraendert] {
        assert_eq!(antwort.status, 200, "{}", antwort.json);
    }
    let debug = mit(&zeilen(wurzel.path(), HEUTE), "DEBUG", "aendern");
    assert_eq!(debug.len(), 3, "{debug:#?}");
    let vorne = " DEBUG aendern Nachricht bearbeitet | channel=allgemein kennung=beta@127.0.0.1 anzahl=14 dauer_ms=";
    assert!(
        debug.iter().all(|zeile| zeile.contains(vorne)),
        "{debug:#?}"
    );
    assert!(!debug[0].contains("grund="), "{debug:#?}");
    assert!(debug[1].ends_with(" grund=wiederholung"), "{debug:#?}");
    assert!(debug[2].ends_with(" grund=unveraendert"), "{debug:#?}");
    let von_gamma: Vec<&String> = fremd_episoden
        .iter()
        .filter(|zeile| zeile.contains("kennung=gamma@127.0.0.1"))
        .collect();
    assert_eq!(von_gamma.len(), 2, "{fremd_episoden:#?}");
    assert!(
        von_gamma[0].ends_with(
            " WARNING aendern Ändern fremder Nachricht abgewiesen — Episode beginnt | kennung=gamma@127.0.0.1 channel=allgemein"
        ),
        "{von_gamma:#?}"
    );
    assert!(
        von_gamma[1].contains("Episode endet: 3 in "),
        "{von_gamma:#?}"
    );
    for nummer in 1..=5 {
        let kennung = format!("kennung=fremd{nummer}@127.0.0.1");
        let beginn = fremd_episoden
            .iter()
            .filter(|zeile| zeile.contains("Episode beginnt") && zeile.contains(&kennung))
            .count();
        assert_eq!(beginn, 1, "{kennung}: {fremd_episoden:#?}");
    }
    assert!(
        alle_zeilen(wurzel.path())
            .iter()
            .all(|zeile| !zeile.contains("Hallo zusammen") && !zeile.contains("gefälscht")),
        "Text im Protokoll"
    );
}
