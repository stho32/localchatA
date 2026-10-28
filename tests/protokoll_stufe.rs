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
