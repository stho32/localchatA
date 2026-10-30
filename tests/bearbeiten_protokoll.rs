//! R00039 F0132 „Betriebsprotokoll“ (ADR-0048 Regel 13, Bereich `aendern`): das echte Programm mit
//! `LOCALCHATA_PROTOKOLL_STUFE=debug` und `LOCALCHATA_EPISODE_RUHE_MS=200`; nie Text.

mod gemeinsam;

use gemeinsam::protokoll::{alle_zeilen, mit, zeilen};
use gemeinsam::senden::{HEUTE, beitreten, ordner, senden, starten, tagesinhalt};
use gemeinsam::strom::Strom;
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

/// R00039 B2 (code-reviewer #48; Akzeptanzkriterium „Schreibfehler“, nicht als root): Channel-Ordner und Tagesdatei
/// nur lesbar (`chmod 555` auf den Ordner UND `444` auf die Datei) → zweimal 500 „Nicht gespeichert — der Server
/// konnte nicht schreiben.“, Tageslog byte-gleich, kein `aenderung` im Strom, genau eine ERROR-Episode „Änderung nicht
/// geschrieben“ mit Datei und Grund (Ende mit 2); danach geht Bearbeiten wieder und erzeugt das erste Ereignis.
#[cfg(unix)]
#[tokio::test]
async fn ein_schreibfehler_beim_bearbeiten_ist_500_und_eine_episode() {
    use std::os::unix::fs::PermissionsExt;
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &UMGEBUNG);
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, basis, "beta", Some("agent")).await;
    let alpha = beitreten(&beta, basis, "alpha", None).await;
    let mut strom = Strom::oeffnen(&beta, basis, &alpha).await;
    let gesendet = senden(&beta, basis, &token, "allgemein", "Hallo").await;
    let t1 = gesendet.json["nachricht"]["zeit"]
        .as_str()
        .expect("zeit")
        .to_string();
    strom
        .bis(Duration::from_secs(10), |e| e.data["art"] == "nachricht")
        .await
        .expect("Ereignis der Nachricht");
    let channel = wurzel.path().join("channels/allgemein");
    let datei = gemeinsam::senden::tagesdatei(wurzel.path(), "allgemein");
    let vorher = tagesinhalt(wurzel.path(), "allgemein");
    let rechte = |pfad: &Path, modus| {
        std::fs::set_permissions(pfad, std::fs::Permissions::from_mode(modus)).expect("Rechte");
    };
    rechte(&datei, 0o444);
    rechte(&channel, 0o555);

    let erste = bearbeiten(&beta, basis, &token, &t1, "Hallo zusammen", None).await;
    let zweite = bearbeiten(&beta, basis, &token, &t1, "Hallo zusammen", None).await;
    let waehrend = tagesinhalt(wurzel.path(), "allgemein");
    rechte(&channel, 0o755);
    rechte(&datei, 0o644);
    let ende = warten_auf(wurzel.path(), 1, |zeile| {
        zeile.contains(" ERROR aendern ") && zeile.contains("Episode endet")
    })
    .await;
    let danach = bearbeiten(&beta, basis, &token, &t1, "Hallo wieder", None).await;
    let ereignis = strom
        .bis(Duration::from_secs(10), |e| e.data["art"] == "aenderung")
        .await
        .expect("aenderung nach dem Schreibfehler");

    for antwort in [&erste, &zweite] {
        assert_eq!(antwort.status, 500, "{}", antwort.json);
        assert_eq!(
            antwort.json["fehler"],
            "Nicht gespeichert — der Server konnte nicht schreiben."
        );
    }
    assert_eq!(waehrend, vorher);
    assert_eq!(danach.status, 200, "{}", danach.json);
    assert_eq!(
        ereignis.data["nachricht"]["text"], "Hallo wieder",
        "das erste aenderung-Ereignis gehört zur geglückten Änderung"
    );
    let fehler = mit(&zeilen(wurzel.path(), HEUTE), "ERROR", "aendern");
    let beginn: Vec<_> = fehler
        .iter()
        .filter(|zeile| zeile.contains("Episode beginnt"))
        .collect();
    assert_eq!(beginn.len(), 1, "{fehler:#?}");
    let datei = format!("datei=channels/allgemein/{HEUTE}.log");
    for teil in [
        "Änderung nicht geschrieben",
        "channel=allgemein",
        "kennung=beta@127.0.0.1",
        datei.as_str(),
        "grund=",
    ] {
        assert!(beginn[0].contains(teil), "{teil}: {}", beginn[0]);
    }
    assert_eq!(ende.len(), 1, "{fehler:#?}");
    assert!(ende[0].contains("Episode endet: 2 "), "{}", ende[0]);
}
