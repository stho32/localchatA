//! F0053 Programmfehler — über das echte Programm im Debug-Build (R00014, US-8, US-9, Kriterien
//! „Exception über LOCALCHATA_TESTPANIK“, „Hintergrundaufgabe“, „Testknopf“).

mod gemeinsam;

use gemeinsam::protokoll::{ist_protokollzeile, mit, wert, zeilen};
use gemeinsam::{Absender, Instanz, aufrufen_mit, gesundheit, starten_mit};
use serde_json::json;
use std::path::Path;
use std::time::{Duration, Instant};

const ARGUMENTE: [&str; 5] = ["--port", "0", "--host", "127.0.0.1", "--kein-browser"];

fn starten_um(ordner: &Path, weitere: &[(&str, &str)]) -> Instanz {
    let mut umgebung = vec![
        ("LOCALCHATA_JETZT", "2026-10-28T12:00:00+01:00"),
        ("TZ", "Europe/Berlin"),
    ];
    umgebung.extend_from_slice(weitere);
    starten_mit(ordner, &ARGUMENTE, &umgebung)
}

fn alle_protokolle(ordner: &Path) -> String {
    let mut text = String::new();
    for eintrag in std::fs::read_dir(ordner.join("logs")).expect("logs/") {
        text.push_str(&std::fs::read_to_string(eintrag.expect("Eintrag").path()).expect("lesbar"));
    }
    text
}

#[tokio::test]
async fn us8_panik_im_verlauf_wird_exception_und_error_an_der_grenze_die_instanz_laeuft_weiter() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    std::fs::create_dir_all(wurzel.path().join("channels/allgemein")).expect("Channel");
    let mut instanz = starten_um(wurzel.path(), &[("LOCALCHATA_TESTPANIK", "verlauf")]);
    let absender = Absender::von("127.0.0.1");
    let token = absender
        .beitreten(&instanz.adresse, json!({"name": "beta"}), None)
        .await
        .json["token"]
        .as_str()
        .expect("Token")
        .to_string();
    let verlauf = || {
        absender
            .anfrage(
                reqwest::Method::GET,
                format!("{}api/channels/allgemein/nachrichten", instanz.adresse),
            )
            .header("X-LocalchatA-Sitzung", &token)
            .send()
    };

    let erste = verlauf().await.expect("Antwort");
    let erster_status = erste.status().as_u16();
    let erster_text = erste.text().await.expect("Text");
    let zweite = verlauf().await.expect("Antwort").status().as_u16();
    let (gesund, kennzahlen) = gesundheit(&instanz.adresse).await;
    assert!(instanz.signal_und_warten("TERM").success());
    let stderr = instanz.stderr_bis_zum_ende();

    assert_eq!(erster_status, 500);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&erster_text).expect("JSON"),
        json!({"fehler": "Interner Fehler — Verlauf nicht gelesen; bitte erneut versuchen"})
    );
    for fremd in ["panicked", "task", "JoinError"] {
        assert!(!erster_text.contains(fremd), "{erster_text}");
    }
    let datei = zeilen(wurzel.path(), "2026-10-28");
    let exceptions = mit(&datei, "EXCEPTION", "programmfehler");
    let errors = mit(&datei, "ERROR", "programmfehler");
    assert_eq!(exceptions.len(), 1, "{datei:#?}");
    assert_eq!(errors.len(), 1, "{datei:#?}");
    let stelle = |zeile: &String| datei.iter().position(|z| z == zeile);
    assert!(stelle(&exceptions[0]) < stelle(&errors[0]), "{datei:#?}");
    assert!(
        wert(&exceptions[0], "ort").is_some_and(|ort| ort.starts_with("src/")),
        "{}",
        exceptions[0]
    );
    assert!(
        wert(&exceptions[0], "thread").is_some(),
        "{}",
        exceptions[0]
    );
    let error = &errors[0];
    assert_eq!(
        wert(error, "route").as_deref(),
        Some("GET /api/channels/{name}/nachrichten")
    );
    assert_eq!(wert(error, "operation").as_deref(), Some("verlauf"));
    assert_eq!(wert(error, "status").as_deref(), Some("500"));
    assert_eq!(wert(error, "grund").as_deref(), Some("panik"));
    assert_eq!(wert(error, "kennung").as_deref(), Some("beta@127.0.0.1"));
    assert!(stderr.lines().any(|z| z == exceptions[0]), "{stderr}");
    assert!(stderr.lines().any(|z| z == errors[0]), "{stderr}");
    assert!(!stderr.contains("panicked at"), "{stderr}");
    for zeile in stderr.lines() {
        assert!(ist_protokollzeile(zeile), "{zeile}");
    }
    assert_eq!(zweite, 200);
    assert_eq!(gesund, 200);
    // Health-Check nach Exception (F0054): Zeiten der Zeilen im Protokoll.
    let zeit = |zeile: &str| zeile.split(' ').next().unwrap_or_default().to_string();
    assert_eq!(kennzahlen["exceptions"], 1, "{kennzahlen}");
    assert_eq!(kennzahlen["errors"], 1, "{kennzahlen}");
    assert_eq!(kennzahlen["letzte_exception"], json!(zeit(&exceptions[0])));
    assert_eq!(kennzahlen["letzter_error"], json!(zeit(&errors[0])));
    assert_eq!(alle_protokolle(wurzel.path()).matches(&token).count(), 0);
}

/// Erwartet ≤ 1 s nach der Startzeile (ein Takt), Grenze 2 s; Abfrage alle 100 ms.
#[tokio::test]
async fn us9_ein_ausgefallener_takt_wird_sichtbar() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let instanz = starten_um(
        wurzel.path(),
        &[
            ("LOCALCHATA_TESTPANIK", "takt"),
            ("LOCALCHATA_HERZSCHLAG_MS", "1000"),
        ],
    );
    let startzeile = Instant::now();
    let ausgefallen = |datei: &[String]| {
        mit(datei, "ERROR", "programmfehler")
            .iter()
            .any(|z| z.contains("Hintergrundaufgabe takt beendet — "))
    };
    while !ausgefallen(&zeilen(wurzel.path(), "2026-10-28"))
        && startzeile.elapsed() < Duration::from_secs(10)
    {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let dauer = startzeile.elapsed();
    let (status, inhalt) = gesundheit(&instanz.adresse).await;

    assert!(dauer <= Duration::from_secs(2), "{dauer:?}");
    let datei = zeilen(wurzel.path(), "2026-10-28");
    assert_eq!(
        mit(&datei, "EXCEPTION", "programmfehler").len(),
        1,
        "{datei:#?}"
    );
    assert_eq!(
        mit(&datei, "ERROR", "programmfehler").len(),
        1,
        "{datei:#?}"
    );
    assert_eq!(status, 200);
    assert_eq!(inhalt["aufgaben_ausgefallen"], json!(["takt"]), "{inhalt}");
    assert_eq!(inhalt["exceptions"], 1, "{inhalt}");
}

#[test]
fn ein_unbekannter_testknopf_ist_exit_2_und_nennt_die_variable() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let ordner = wurzel.path().display().to_string();

    let ende = aufrufen_mit(
        &[
            &ordner,
            "--port",
            "0",
            "--host",
            "127.0.0.1",
            "--kein-browser",
        ],
        &[("LOCALCHATA_TESTPANIK", "laut")],
    );

    assert_eq!(ende.exit, Some(2), "{}", ende.stderr);
    assert!(
        ende.stderr.contains("LOCALCHATA_TESTPANIK"),
        "{}",
        ende.stderr
    );
    assert!(!wurzel.path().join("logs").exists());
}
