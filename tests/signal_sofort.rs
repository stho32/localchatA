//! B0014 Signale vor der Startzeile abfangen — Abnahme-Probe B1 des Code-Reviews.
//! Die Startzeile ist der Vertrag „läuft und ist bereit“: ein Signal unmittelbar danach endet geordnet mit Exit 0,
//! danach ist der Port frei und ein Neustart auf demselben Ordner gelingt (Sperre frei).
#![cfg(unix)]

mod gemeinsam;

use gemeinsam::{Instanz, signal_senden, starten, starten_mit, warten};
use std::net::TcpListener;
use std::path::Path;
use std::time::Duration;

const LOKAL_OHNE_BROWSER: &[&str] = &["--port", "0", "--host", "127.0.0.1", "--kein-browser"];
const LOKAL_MIT_BROWSER: &[&str] = &["--port", "0", "--host", "127.0.0.1"];

/// Signal ohne Verzögerung nach der Startzeile; liefert, wie der Lauf endete und ob der Port danach frei ist.
fn sofort_signalisieren(mut instanz: Instanz, signal: &str) -> Result<(), String> {
    let port = instanz.port();
    signal_senden(&instanz.kind, signal);
    let ende = warten(&mut instanz.kind, Duration::from_secs(5))
        .ok_or_else(|| format!("{signal}: Programm endet nicht"))?;
    if ende.code() != Some(0) {
        return Err(format!("{signal}: endete mit {ende}, erwartet Exit 0"));
    }
    TcpListener::bind(("127.0.0.1", port))
        .map(drop)
        .map_err(|fehler| format!("{signal}: Port {port} nicht frei ({fehler})"))
}

/// Startet `wiederholungen` Mal nacheinander auf demselben Ordner — jeder Start beweist, dass die Sperre frei ist.
fn wiederholt(
    signal: &str,
    wiederholungen: usize,
    starten: impl Fn(&Path) -> Instanz,
) -> Vec<String> {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let mut fehlschlaege: Vec<String> = (1..=wiederholungen)
        .filter_map(|lauf| {
            sofort_signalisieren(starten(wurzel.path()), signal)
                .err()
                .map(|grund| format!("Lauf {lauf}: {grund}"))
        })
        .collect();
    let neustart = starten(wurzel.path());
    if neustart.startzeile.is_empty() {
        fehlschlaege.push("Neustart nach dem letzten Lauf ohne Startzeile".into());
    }
    fehlschlaege
}

fn bericht(fehlschlaege: &[String], wiederholungen: usize) -> String {
    format!(
        "{} von {wiederholungen} Läufen nicht geordnet beendet:\n{}",
        fehlschlaege.len(),
        fehlschlaege.join("\n")
    )
}

#[test]
fn sigterm_sofort_nach_der_startzeile_endet_fuenfzigmal_geordnet() {
    let fehlschlaege = wiederholt("TERM", 50, |ordner| starten(ordner, LOKAL_OHNE_BROWSER));
    assert!(fehlschlaege.is_empty(), "{}", bericht(&fehlschlaege, 50));
}

#[test]
fn sigint_sofort_nach_der_startzeile_endet_fuenfzigmal_geordnet() {
    let fehlschlaege = wiederholt("INT", 50, |ordner| starten(ordner, LOKAL_OHNE_BROWSER));
    assert!(fehlschlaege.is_empty(), "{}", bericht(&fehlschlaege, 50));
}

#[test]
fn sigterm_sofort_nach_der_startzeile_endet_auch_beim_browserstart_zwanzigmal_geordnet() {
    let fehlschlaege = wiederholt("TERM", 20, |ordner| {
        starten_mit(
            ordner,
            LOKAL_MIT_BROWSER,
            &[("LOCALCHATA_BROWSER", "/bin/true")],
        )
    });
    assert!(fehlschlaege.is_empty(), "{}", bericht(&fehlschlaege, 20));
}
