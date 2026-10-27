//! F0009 Sitzung wiedererkennen und nach der Gnadenfrist freigeben — über das echte Programm
//! (R00003, US-11 bis US-14 und Kriterien).
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::{Absender, aufrufen_mit, starten_mit};
use serde_json::json;

#[test]
fn us14_unlesbare_gnadenfrist_endet_mit_exit_zwei_und_nennt_die_variable() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let ordner = wurzel.path().display().to_string();

    let ende = aufrufen_mit(
        &[&ordner, "--port", "0", "--kein-browser"],
        &[("LOCALCHATA_GNADENFRIST_MS", "bald")],
    );

    assert_eq!(ende.exit, Some(2));
    assert!(
        ende.stderr.contains("LOCALCHATA_GNADENFRIST_MS"),
        "{}",
        ende.stderr
    );
    assert_eq!(ende.stdout, "", "keine Startzeile");
}

#[tokio::test]
async fn die_gnadenfrist_aus_der_umgebung_steht_in_der_beitrittsantwort() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let instanz = starten_mit(
        wurzel.path(),
        &["--port", "0", "--host", "127.0.0.1", "--kein-browser"],
        &[("LOCALCHATA_GNADENFRIST_MS", "3000")],
    );

    let antwort = Absender::von("127.0.0.1")
        .beitreten(&instanz.adresse, json!({"name": "claude"}), None)
        .await;

    assert_eq!(antwort.json["gnadenfrist_s"], 3, "{}", antwort.json);
}
