//! R00040 F0136 „Betriebsprotokoll“ (ADR-0048 Regel 13, Bereich `aendern`): das echte Programm mit
//! `LOCALCHATA_PROTOKOLL_STUFE=debug` und `LOCALCHATA_EPISODE_RUHE_MS=200`; nie Text.

mod gemeinsam;

use gemeinsam::protokoll::{alle_zeilen, mit, zeilen};
use gemeinsam::senden::{HEUTE, beitreten, ordner, senden, starten, tagesinhalt};
use gemeinsam::{Absender, ApiAntwort, api_antwort};
use serde_json::json;
use std::path::Path;
use std::time::{Duration, Instant};

const UMGEBUNG: [(&str, &str); 2] = [
    ("LOCALCHATA_PROTOKOLL_STUFE", "debug"),
    ("LOCALCHATA_EPISODE_RUHE_MS", "200"),
];

async fn loeschen(absender: &Absender, basis: &str, token: &str, zeit: &str) -> ApiAntwort {
    api_antwort(
        absender
            .anfrage(
                reqwest::Method::POST,
                format!("{basis}api/channels/allgemein/nachrichten/loeschen"),
            )
            .header("X-LocalchatA-Sitzung", token)
            .json(&json!({ "zeit": zeit })),
    )
    .await
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

async fn beta_mit_hallo(beta: &Absender, basis: &str) -> (String, String) {
    let token = beitreten(beta, basis, "beta", Some("agent")).await;
    let gesendet = senden(beta, basis, &token, "allgemein", "Hallo").await;
    assert_eq!(gesendet.status, 201, "{}", gesendet.json);
    let t1 = gesendet.json["nachricht"]["zeit"]
        .as_str()
        .expect("zeit")
        .to_string();
    (token, t1)
}

/// R00040 „Betriebsprotokoll“: der Rundweg schreibt `DEBUG aendern Nachricht gelöscht | channel=allgemein
/// kennung=beta@127.0.0.1 anhaenge=0 dauer_ms=…`; das zweite Löschen trägt `grund=schon_geloescht`; zwei 403 von
/// `gamma` → eine Episode „Ändern fremder Nachricht abgewiesen“ mit Ende 2; `Hallo` steht nirgends in `logs/`.
#[tokio::test]
async fn loeschen_im_betriebsprotokoll() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &UMGEBUNG);
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let (token, t1) = beta_mit_hallo(&beta, basis).await;
    let gamma = beitreten(&beta, basis, "gamma", None).await;

    let fremd = [
        loeschen(&beta, basis, &gamma, &t1).await,
        loeschen(&beta, basis, &gamma, &t1).await,
    ];
    let rundweg = loeschen(&beta, basis, &token, &t1).await;
    let zweites = loeschen(&beta, basis, &token, &t1).await;
    let episode = warten_auf(wurzel.path(), 2, |zeile| {
        zeile.contains(" WARNING aendern Ändern fremder Nachricht abgewiesen — Episode ")
    })
    .await;

    for antwort in &fremd {
        assert_eq!(antwort.status, 403, "{}", antwort.json);
    }
    assert_eq!(rundweg.status, 200, "{}", rundweg.json);
    assert_eq!(zweites.status, 200, "{}", zweites.json);
    let debug = mit(&zeilen(wurzel.path(), HEUTE), "DEBUG", "aendern");
    assert_eq!(debug.len(), 2, "{debug:#?}");
    let vorne = " DEBUG aendern Nachricht gelöscht | channel=allgemein kennung=beta@127.0.0.1 anhaenge=0 dauer_ms=";
    assert!(
        debug.iter().all(|zeile| zeile.contains(vorne)),
        "{debug:#?}"
    );
    assert!(!debug[0].contains("grund="), "{debug:#?}");
    assert!(debug[1].ends_with(" grund=schon_geloescht"), "{debug:#?}");
    assert!(
        episode[0].ends_with("Episode beginnt | kennung=gamma@127.0.0.1 channel=allgemein"),
        "{episode:#?}"
    );
    assert!(episode[1].contains("Episode endet: 2 in "), "{episode:#?}");
    assert!(
        alle_zeilen(wurzel.path())
            .iter()
            .all(|zeile| !zeile.contains("Hallo")),
        "Text im Protokoll"
    );
}

/// R00040 „Schreibfehler“ (nicht als root): Channel-Ordner und Tagesdatei nur lesbar → 500 „Nicht gespeichert — der
/// Server konnte nicht schreiben.“, Tageslog byte-gleich, eine ERROR-Episode „Änderung nicht geschrieben“ mit Datei;
/// danach löscht es wieder.
#[cfg(unix)]
#[tokio::test]
async fn ein_schreibfehler_beim_loeschen_ist_500_und_eine_episode() {
    use std::os::unix::fs::PermissionsExt;
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &UMGEBUNG);
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let (token, t1) = beta_mit_hallo(&beta, basis).await;
    let channel = wurzel.path().join("channels/allgemein");
    let datei = gemeinsam::senden::tagesdatei(wurzel.path(), "allgemein");
    let vorher = tagesinhalt(wurzel.path(), "allgemein");
    let rechte = |pfad: &Path, modus| {
        std::fs::set_permissions(pfad, std::fs::Permissions::from_mode(modus)).expect("Rechte");
    };
    rechte(&datei, 0o444);
    rechte(&channel, 0o555);

    let gescheitert = loeschen(&beta, basis, &token, &t1).await;
    let waehrend = tagesinhalt(wurzel.path(), "allgemein");
    rechte(&channel, 0o755);
    rechte(&datei, 0o644);
    let fehler = warten_auf(wurzel.path(), 1, |zeile| {
        zeile.contains(" ERROR aendern Änderung nicht geschrieben — Episode beginnt")
    })
    .await;
    let danach = loeschen(&beta, basis, &token, &t1).await;

    assert_eq!(gescheitert.status, 500, "{}", gescheitert.json);
    assert_eq!(
        gescheitert.json["fehler"],
        "Nicht gespeichert — der Server konnte nicht schreiben."
    );
    assert_eq!(waehrend, vorher);
    assert_eq!(fehler.len(), 1, "{fehler:#?}");
    assert!(
        fehler[0].contains(&format!("datei=channels/allgemein/{HEUTE}.log"))
            && fehler[0].contains("kennung=beta@127.0.0.1"),
        "{fehler:#?}"
    );
    assert_eq!(danach.status, 200, "{}", danach.json);
    assert_eq!(danach.json["ergebnis"], "geloescht");
}
