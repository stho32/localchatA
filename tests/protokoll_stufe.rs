//! F0054 Stufe und Kennzahlen — über das echte Programm (R00014, US-10 bis US-12 und Kriterien).

mod gemeinsam;

use gemeinsam::protokoll::{mit, wert, zeilen};
use gemeinsam::{Instanz, aufrufen, aufrufen_mit, starten_mit};
use std::path::Path;

const LOKAL: [&str; 5] = ["--port", "0", "--host", "127.0.0.1", "--kein-browser"];

fn starten_um(ordner: &Path, weitere_argumente: &[&str], umgebung: &[(&str, &str)]) -> Instanz {
    let mut argumente = LOKAL.to_vec();
    argumente.extend_from_slice(weitere_argumente);
    let mut alle = vec![
        ("LOCALCHATA_JETZT", "2026-10-28T12:00:00+01:00"),
        ("TZ", "Europe/Berlin"),
    ];
    alle.extend_from_slice(umgebung);
    starten_mit(ordner, &argumente, &alle)
}

#[test]
fn eine_falsche_stufe_in_der_umgebung_ist_exit_2_ohne_logs() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let ordner = wurzel.path().display().to_string();
    let mut argumente = vec![ordner.as_str()];
    argumente.extend_from_slice(&LOKAL);

    let ende = aufrufen_mit(&argumente, &[("LOCALCHATA_PROTOKOLL_STUFE", "laut")]);

    assert_eq!(ende.exit, Some(2), "{}", ende.stderr);
    assert!(
        ende.stderr.contains("debug, information, warning, error"),
        "{}",
        ende.stderr
    );
    assert!(!wurzel.path().join("logs").exists());
}

#[test]
fn us12_eine_falsche_stufe_als_option_ist_exit_2_ohne_logs() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let ordner = wurzel.path().display().to_string();
    let mut argumente = vec![ordner.as_str()];
    argumente.extend_from_slice(&LOKAL);
    argumente.extend_from_slice(&["--protokoll-stufe", "laut"]);

    let ende = aufrufen(&argumente);

    assert_eq!(ende.exit, Some(2), "{}", ende.stderr);
    assert!(
        ende.stderr.contains("debug, information, warning, error"),
        "{}",
        ende.stderr
    );
    assert!(!wurzel.path().join("logs").exists());
}

#[test]
fn die_hilfe_nennt_option_und_umgebung() {
    let ende = aufrufen(&["--hilfe"]);
    assert!(ende.stdout.contains("--protokoll-stufe"), "{}", ende.stdout);
    assert!(
        ende.stdout.contains("LOCALCHATA_PROTOKOLL_STUFE"),
        "{}",
        ende.stdout
    );
}

#[tokio::test]
async fn die_option_schlaegt_die_umgebung_auch_der_start_eintrag_faellt_unter_warning() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let mut instanz = starten_um(
        wurzel.path(),
        &["--protokoll-stufe", "warning"],
        &[("LOCALCHATA_PROTOKOLL_STUFE", "debug")],
    );
    assert!(instanz.signal_und_warten("TERM").success());

    let datei = zeilen(wurzel.path(), "2026-10-28");
    assert!(
        datei
            .iter()
            .all(|zeile| !zeile.contains(" INFORMATION ") && !zeile.contains(" DEBUG ")),
        "{datei:#?}"
    );
}

#[tokio::test]
async fn der_start_eintrag_nennt_stufe_und_herkunft_aus_der_umgebung() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let mut instanz = starten_um(
        wurzel.path(),
        &[],
        &[("LOCALCHATA_PROTOKOLL_STUFE", "Debug")],
    );
    assert!(instanz.signal_und_warten("TERM").success());
    let stderr = instanz.stderr_bis_zum_ende();

    let start = mit(&zeilen(wurzel.path(), "2026-10-28"), "INFORMATION", "start");
    assert_eq!(start.len(), 1, "{start:#?}");
    assert_eq!(wert(&start[0], "stufe").as_deref(), Some("debug"));
    assert_eq!(
        wert(&start[0], "option").as_deref(),
        Some("LOCALCHATA_PROTOKOLL_STUFE")
    );
    assert!(
        stderr
            .lines()
            .all(|zeile| !zeile.contains(" INFORMATION ") && !zeile.contains(" DEBUG ")),
        "{stderr}"
    );
}

#[tokio::test]
async fn us10_frische_instanz_meldet_null_und_die_bytes_der_heutigen_datei() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let instanz = starten_um(wurzel.path(), &[], &[]);

    let (status, inhalt) = gemeinsam::gesundheit(&instanz.adresse).await;
    let groesse = std::fs::metadata(wurzel.path().join("logs/2026-10-28.log"))
        .expect("heute.log")
        .len();

    assert_eq!(status, 200);
    for (feld, erwartet) in [
        ("status", serde_json::json!("bereit")),
        ("warnings", serde_json::json!(0)),
        ("errors", serde_json::json!(0)),
        ("exceptions", serde_json::json!(0)),
        ("letzter_error", serde_json::Value::Null),
        ("letzte_exception", serde_json::Value::Null),
        ("protokoll_ok", serde_json::json!(true)),
        ("protokoll_stufe", serde_json::json!("information")),
        ("protokoll_verworfen", serde_json::json!(0)),
        ("aufgaben_ausgefallen", serde_json::json!([])),
        (
            "abgewiesen",
            serde_json::json!({"zu_gross":0,"kein_json":0,"kaputt":0,"abgebrochen":0,"zu_langsam":0}),
        ),
    ] {
        assert_eq!(inhalt[feld], erwartet, "{feld}: {inhalt}");
    }
    let bytes = inhalt["protokoll_bytes_heute"].as_u64().expect("Zahl");
    let eine_zeile = zeilen(wurzel.path(), "2026-10-28")
        .iter()
        .map(|zeile| zeile.len() as u64 + 1)
        .max()
        .unwrap_or(0);
    assert!(
        bytes.abs_diff(groesse) <= eine_zeile,
        "{bytes} gegen {groesse}"
    );
    assert!(bytes > 0);
}

#[tokio::test]
async fn zaehler_zaehlen_auch_unter_der_mindeststufe() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let ziel = wurzel.path().join("channels/allgemein/2026-10-28.log");
    std::fs::create_dir_all(ziel.parent().expect("Eltern")).expect("Ordner");
    std::fs::write(&ziel, "@ kaputt\n\n@ kaputt\n\n").expect("Datei");
    let mut instanz = starten_um(wurzel.path(), &["--protokoll-stufe", "error"], &[]);

    let (_, inhalt) = gemeinsam::gesundheit(&instanz.adresse).await;
    assert!(instanz.signal_und_warten("TERM").success());
    let stderr = instanz.stderr_bis_zum_ende();

    assert_eq!(inhalt["warnings"], 2, "{inhalt}");
    assert_eq!(inhalt["protokoll_stufe"], "error");
    assert!(mit(&zeilen(wurzel.path(), "2026-10-28"), "WARNING", "laden").is_empty());
    assert!(!stderr.contains(" WARNING "), "{stderr}");
}

/// Tritt bei, ruft den Verlauf von `allgemein` ab, beendet geordnet.
async fn verlauf_abrufen(instanz: &mut Instanz) {
    let absender = gemeinsam::Absender::von("127.0.0.1");
    let token = absender
        .beitreten(&instanz.adresse, serde_json::json!({"name": "beta"}), None)
        .await
        .json["token"]
        .as_str()
        .expect("Token")
        .to_string();
    let antwort = absender
        .anfrage(
            reqwest::Method::GET,
            format!("{}api/channels/allgemein/nachrichten", instanz.adresse),
        )
        .header("X-LocalchatA-Sitzung", token)
        .send()
        .await
        .expect("Antwort");
    assert_eq!(antwort.status(), 200);
    assert!(instanz.signal_und_warten("TERM").success());
}

#[tokio::test]
async fn ohne_angabe_kein_debug_mit_debug_eine_verlaufszeile() {
    for (stufe, debug_erwartet) in [(None, false), (Some("debug"), true)] {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        std::fs::create_dir_all(wurzel.path().join("channels/allgemein")).expect("Channel");
        let umgebung: Vec<(&str, &str)> = stufe
            .map(|s| vec![("LOCALCHATA_PROTOKOLL_STUFE", s)])
            .unwrap_or_default();
        let mut instanz = starten_um(wurzel.path(), &[], &umgebung);

        verlauf_abrufen(&mut instanz).await;

        let datei = zeilen(wurzel.path(), "2026-10-28");
        let verlauf = mit(&datei, "DEBUG", "verlauf");
        if debug_erwartet {
            assert_eq!(verlauf.len(), 1, "{datei:#?}");
            assert_eq!(wert(&verlauf[0], "channel").as_deref(), Some("allgemein"));
            assert_eq!(wert(&verlauf[0], "anzahl").as_deref(), Some("0"));
            assert!(wert(&verlauf[0], "dauer_ms").is_some(), "{}", verlauf[0]);
        } else {
            assert!(datei.iter().all(|z| !z.contains(" DEBUG ")), "{datei:#?}");
        }
    }
}
