//! R00022 F0073 B0346 „Vorlauf beim Start prüfen“ (ADR-0042 Ergänzung Karte #31 Punkt 3): das echte Programm mit
//! `LOCALCHATA_JETZT`; der Vorlauf der zuletzt vergebenen Nachrichtenzeit steht im Protokoll, bevor jemand sendet.

mod gemeinsam;

use gemeinsam::protokoll::{mit, zeilen};
use gemeinsam::senden::{HEUTE, ordner, starten, tagesdatei};
use std::path::Path;
use std::time::Duration;

fn satz_um(ordner: &Path, zeit: &str) {
    std::fs::write(
        tagesdatei(ordner, "allgemein"),
        format!("@ {zeit} claude@10.0.0.1\n  aus der Zukunft\n\n"),
    )
    .expect("Tagesdatei");
}

fn zeit_zeilen(ordner: &Path) -> Vec<String> {
    mit(&zeilen(ordner, HEUTE), "WARNING", "zeit")
}

/// Nachricht von 12:00:30, Uhr 10:00: binnen 1 s nach der Startzeile steht `WARNING zeit … 2 h 00 min voraus …
/// Episode beginnt | dauer_ms=…` im Protokoll, ohne dass gesendet wurde. Mutante „nur beim Senden“ → rot.
/// Die Abnahme-Probe nennt 12:00:00; die Testuhr läuft ab `LOCALCHATA_JETZT` aber in Echtzeit weiter, der Vorlauf
/// wäre 7.199.99x ms und der abgerundete Text „1 h 59 min“ — mit 30 s mehr ist „2 h 00 min“ stabil.
#[test]
fn ein_vorlauf_steht_beim_start_im_protokoll_bevor_gesendet_wird() {
    let wurzel = ordner();
    satz_um(wurzel.path(), "2026-10-28T12:00:30.000+01:00");

    let _instanz = starten(
        wurzel.path(),
        &[("LOCALCHATA_JETZT", "2026-10-28T10:00:00.000+01:00")],
    );
    let startzeile = std::time::Instant::now();
    let mut gefunden = zeit_zeilen(wurzel.path());
    while gefunden.is_empty() && startzeile.elapsed() < Duration::from_secs(1) {
        std::thread::sleep(Duration::from_millis(20));
        gefunden = zeit_zeilen(wurzel.path());
    }

    assert_eq!(gefunden.len(), 1, "{gefunden:#?}");
    assert!(
        gefunden[0].contains("Nachrichtenzeit läuft der Systemuhr 2 h 00 min voraus"),
        "{gefunden:#?}"
    );
    assert!(
        gefunden[0].contains("Episode beginnt | dauer_ms="),
        "{gefunden:#?}"
    );
    let dauer = gemeinsam::protokoll::wert(&gefunden[0], "dauer_ms").expect("dauer_ms");
    assert!(
        dauer.parse::<u64>().expect("Zahl") >= 7_200_000,
        "{gefunden:#?}"
    );
}

/// Gegenprobe: eine Nachricht 30 s voraus ist kein Vorlauf — nach 1 s keine `zeit`-Zeile.
#[test]
fn dreissig_sekunden_voraus_sind_beim_start_kein_vorlauf() {
    let wurzel = ordner();
    satz_um(wurzel.path(), "2026-10-28T10:00:30.000+01:00");

    let _instanz = starten(
        wurzel.path(),
        &[("LOCALCHATA_JETZT", "2026-10-28T10:00:00.000+01:00")],
    );
    std::thread::sleep(Duration::from_secs(1));

    assert_eq!(zeit_zeilen(wurzel.path()), Vec::<String>::new());
}
