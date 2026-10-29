//! R00018 F0061 Tagesbudget über Neustarts — über das echte Programm: eine schon volle Klasse in der vorhandenen
//! Tagesdatei verwirft nach dem Start ihre Einträge, die Meldung steht einmal neu in der Datei.

mod gemeinsam;

use gemeinsam::{Absender, starten_mit};
use serde_json::json;

/// Budget der Klasse INFORMATION+WARNING: 32 MiB; vorhanden sind 33 MiB.
const VORHANDEN: usize = 33 * 1024 * 1024;

#[tokio::test]
async fn ein_neustart_mit_voller_informationsklasse_schreibt_keine_information_mehr() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let logs = wurzel.path().join("logs");
    std::fs::create_dir_all(&logs).expect("logs");
    let zeile = format!(
        "2026-10-28T11:00:00.000+01:00 INFORMATION start Vorlauf {}\n",
        "x".repeat(60)
    );
    let vorhanden = zeile.repeat(VORHANDEN / zeile.len() + 1);
    assert!(vorhanden.len() >= VORHANDEN);
    std::fs::write(logs.join("2026-10-28.log"), &vorhanden).expect("Tagesdatei");
    let instanz = starten_mit(
        wurzel.path(),
        &["--port", "0", "--host", "127.0.0.1", "--kein-browser"],
        &[
            ("LOCALCHATA_JETZT", "2026-10-28T12:00:00+01:00"),
            ("TZ", "Europe/Berlin"),
        ],
    );

    let beitritt = Absender::von("127.0.0.1")
        .beitreten(&instanz.adresse, json!({ "name": "beta" }), None)
        .await;
    let (status, gesundheit) = gemeinsam::gesundheit(&instanz.adresse).await;

    assert_eq!(beitritt.status, 201, "{}", beitritt.json);
    let inhalt = std::fs::read_to_string(logs.join("2026-10-28.log")).expect("Tagesdatei");
    let neu: Vec<&str> = inhalt[vorhanden.len()..].lines().collect();
    assert!(
        !neu.iter().any(|zeile| zeile.contains(" INFORMATION ")),
        "{neu:#?}"
    );
    let meldungen: Vec<&&str> = neu
        .iter()
        .filter(|zeile| {
            zeile.contains(" WARNING protokoll Tagesbudget INFORMATION+WARNING erreicht — ")
        })
        .collect();
    assert_eq!(meldungen.len(), 1, "{neu:#?}");
    assert_eq!(status, 200);
    assert_eq!(gesundheit["status"], "bereit");
    let verworfen = gesundheit["protokoll_verworfen"].as_u64().expect("Zahl");
    assert!(verworfen >= 2, "Start und Beitritt verworfen: {verworfen}");
}
