//! B0029 Teilschreiben an einer echten Datei: `RLIMIT_FSIZE` im Kindprozess lässt `write_all` nach einem
//! Teil der Bytes scheitern; danach hat die Tagesdatei die Länge vor dem Schreiben (R00002, ADR-0011).
#![cfg(unix)]

use chrono::NaiveDate;
use localchata::gespraeche::{Dateiablage, Gespraech, Logablage};
use std::fs;
use std::process::Command;

const TAG: &str = "2026-10-27";
const VORHER: &str = "@ 2026-10-27T14:00:00.000+01:00 a@127.0.0.1\n  eins\n\n";
const SATZ: &str =
    "@ 2026-10-27T14:00:00.001+01:00 c@127.0.0.1\n  zwei, länger als zwanzig Bytes\n\n";

fn tag() -> NaiveDate {
    NaiveDate::parse_from_str(TAG, "%Y-%m-%d").expect("Tag")
}

/// Kindprozess: Grenze der Dateigröße auf Länge + 20 Bytes, SIGXFSZ abgefangen, dann anhängen.
#[tokio::test]
#[ignore = "nur als Kindprozess von teilschreiben_*"]
async fn hilfslauf_anhaengen_mit_dateigroessengrenze() {
    use rustix::process::{Resource, Rlimit, Signal, getrlimit, setrlimit};
    use tokio::signal::unix::{SignalKind, signal};

    let ordner = std::env::var("TEILSCHREIBEN_ORDNER").expect("TEILSCHREIBEN_ORDNER");
    let _xfsz = signal(SignalKind::from_raw(Signal::XFSZ.as_raw())).expect("SIGXFSZ abfangen");
    let laenge = VORHER.len() as u64;
    setrlimit(
        Resource::Fsize,
        Rlimit {
            current: Some(laenge + 20),
            maximum: getrlimit(Resource::Fsize).maximum,
        },
    )
    .expect("Grenze gesetzt");

    let ergebnis =
        Dateiablage::neu(ordner.as_ref()).anhaengen(&Gespraech::channel("allgemein"), tag(), SATZ);

    println!(
        "ERGEBNIS {}",
        if ergebnis.is_err() { "fehler" } else { "ok" }
    );
}

#[test]
fn teilschreiben_an_einer_echten_datei_hinterlaesst_die_laenge_vor_dem_schreiben() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let datei = wurzel.path().join(format!("channels/allgemein/{TAG}.log"));
    fs::create_dir_all(datei.parent().expect("Ordner")).expect("Ordner");
    fs::write(&datei, VORHER).expect("erster Satz");

    let ausgabe = Command::new(std::env::current_exe().expect("Testprogramm"))
        .args([
            "--exact",
            "hilfslauf_anhaengen_mit_dateigroessengrenze",
            "--ignored",
            "--nocapture",
        ])
        .env("TEILSCHREIBEN_ORDNER", wurzel.path())
        .output()
        .expect("Kindprozess");
    let stdout = String::from_utf8_lossy(&ausgabe.stdout);

    assert!(stdout.contains("ERGEBNIS fehler"), "{stdout}");
    assert_eq!(fs::read_to_string(&datei).expect("Tagesdatei"), VORHER);
}
