//! F0052 Bereinigung nach 14 Tagen — über das echte Programm (R00014, US-6, US-7, Kriterium „Tageswechsel“).

mod gemeinsam;

use gemeinsam::protokoll::{mit, zeilen};
use gemeinsam::starten_mit;
use std::path::Path;
use std::time::{Duration, Instant};

const ARGUMENTE: [&str; 5] = ["--port", "0", "--host", "127.0.0.1", "--kein-browser"];

fn protokolldateien(ordner: &Path, namen: &[&str]) {
    let logs = ordner.join("logs");
    std::fs::create_dir_all(&logs).expect("logs");
    for name in namen {
        std::fs::write(logs.join(name), "alt\n").expect("Datei");
    }
}

#[cfg(unix)]
#[tokio::test]
async fn us6_beim_start_entfaellt_was_aelter_als_14_tage_ist() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    protokolldateien(
        wurzel.path(),
        &[
            "2026-10-13.log",
            "2026-10-14.log",
            "2026-10-15.log",
            "notizen.txt",
        ],
    );
    std::fs::write(wurzel.path().join("ziel.log"), "Ziel").expect("Ziel");
    std::os::unix::fs::symlink(
        wurzel.path().join("ziel.log"),
        wurzel.path().join("logs/2026-10-01.log"),
    )
    .expect("Symlink");

    let _instanz = starten_mit(
        wurzel.path(),
        &ARGUMENTE,
        &[
            ("LOCALCHATA_JETZT", "2026-10-28T12:00:00+01:00"),
            ("TZ", "Europe/Berlin"),
        ],
    );

    let logs = wurzel.path().join("logs");
    assert!(!logs.join("2026-10-13.log").exists());
    assert!(!logs.join("2026-10-14.log").exists());
    assert!(logs.join("2026-10-15.log").exists());
    assert!(logs.join("notizen.txt").exists());
    assert!(logs.join("2026-10-01.log").symlink_metadata().is_ok());
    assert!(wurzel.path().join("ziel.log").exists());
    let protokoll = mit(
        &zeilen(wurzel.path(), "2026-10-28"),
        "INFORMATION",
        "protokoll",
    );
    assert_eq!(protokoll.len(), 1, "{protokoll:#?}");
    assert!(
        protokoll[0].ends_with(
            "2 Protokolldateien entfallen (2026-10-13 bis 2026-10-14; Aufbewahrung 14 Tage; Start)"
        ),
        "{protokoll:#?}"
    );
}

/// Zweck: der Tagestakt ist im laufenden Programm verdrahtet, Bereinigung und Rotation greifen ohne Neustart.
/// Uhr ab dem Lesen der Startzeile; erwartet ≈ 2–3 s (höchstens 2 s bis Mitternacht, höchstens 1 s bis zum
/// nächsten Takt), Grenze 4,0 s (33 % über 3 s); Abfrage alle 100 ms.
#[tokio::test]
async fn us7_um_mitternacht_entfaellt_der_naechste_tag_ohne_neustart() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    protokolldateien(wurzel.path(), &["2026-10-14.log", "2026-10-15.log"]);

    let _instanz = starten_mit(
        wurzel.path(),
        &ARGUMENTE,
        &[
            ("LOCALCHATA_JETZT", "2026-10-28T23:59:58+01:00"),
            ("TZ", "Europe/Berlin"),
        ],
    );
    let startzeile = Instant::now();
    let logs = wurzel.path().join("logs");
    assert!(!logs.join("2026-10-14.log").exists(), "Start-Bereinigung");
    assert!(logs.join("2026-10-15.log").exists(), "15 ≥ 28 − 13");

    while logs.join("2026-10-15.log").exists() && startzeile.elapsed() < Duration::from_secs(10) {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let dauer = startzeile.elapsed();
    eprintln!("messung us7_tageswechsel_ms={}", dauer.as_millis());

    assert!(
        !logs.join("2026-10-15.log").exists(),
        "nach {dauer:?} noch da"
    );
    assert!(dauer <= Duration::from_millis(4_000), "{dauer:?}");
    let neu = mit(
        &zeilen(wurzel.path(), "2026-10-29"),
        "INFORMATION",
        "protokoll",
    );
    assert!(
        neu.iter().any(|zeile| zeile.ends_with(
            "1 Protokolldatei entfällt (2026-10-15 bis 2026-10-15; Aufbewahrung 14 Tage; Tageswechsel)"
        )),
        "{neu:#?}"
    );
    let alt = mit(
        &zeilen(wurzel.path(), "2026-10-28"),
        "INFORMATION",
        "protokoll",
    );
    assert!(
        alt.iter().any(|zeile| zeile.contains("(2026-10-14 bis 2026-10-14; Aufbewahrung 14 Tage; Start)")),
        "{alt:#?}"
    );
}
