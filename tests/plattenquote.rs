//! R00023 Plattenquote — über das echte Programm (F0074 Quote einstellen, F0075 Senden an der Quote, F0076 Quote im
//! Betriebsprotokoll).

mod gemeinsam;

use gemeinsam::protokoll::alle_zeilen;
use gemeinsam::{Instanz, aufrufen, aufrufen_mit, starten_mit};
use std::path::Path;

const LOKAL: [&str; 5] = ["--port", "0", "--host", "127.0.0.1", "--kein-browser"];
const FALSCH: &str = "--plattenquote braucht eine Größe wie 4GiB oder 512MiB, oder aus";

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

/// Ruft mit dem Ordner und den lokalen Optionen auf, ohne auf eine Startzeile zu warten.
fn aufrufen_in(
    ordner: &Path,
    weitere_argumente: &[&str],
    umgebung: &[(&str, &str)],
) -> gemeinsam::Ende {
    let ordner = ordner.display().to_string();
    let mut argumente = vec![ordner.as_str()];
    argumente.extend_from_slice(&LOKAL);
    argumente.extend_from_slice(weitere_argumente);
    aufrufen_mit(&argumente, umgebung)
}

/// Der eine Start-Eintrag `INFORMATION start Instanz bereit`.
fn start_eintrag(ordner: &Path) -> String {
    let start: Vec<String> = alle_zeilen(ordner)
        .into_iter()
        .filter(|zeile| zeile.contains(" INFORMATION start Instanz bereit"))
        .collect();
    assert_eq!(start.len(), 1, "{start:#?}");
    start[0].clone()
}

/// F0074 „Falscher Wert beim Start“: `LOCALCHATA_PLATTENQUOTE=4` → Exit 2 mit Beispiel, kein `logs/`; dazu die
/// übrigen falschen Schreibweisen aus US-2 und ADR-0044 Regel 3.
#[test]
fn eine_falsche_quote_in_der_umgebung_ist_exit_2_ohne_logs() {
    for falsch in ["4", "4GB", "0MiB", "1025GiB", "-1GiB", ""] {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");

        let ende = aufrufen_in(wurzel.path(), &[], &[("LOCALCHATA_PLATTENQUOTE", falsch)]);

        assert_eq!(ende.exit, Some(2), "{falsch:?}: {}", ende.stderr);
        assert!(ende.stderr.contains(FALSCH), "{falsch:?}: {}", ende.stderr);
        assert!(!wurzel.path().join("logs").exists(), "{falsch:?}");
    }
}

#[test]
fn eine_falsche_quote_als_option_ist_exit_2_ohne_logs() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");

    let ende = aufrufen_in(wurzel.path(), &["--plattenquote", "4GB"], &[]);

    assert_eq!(ende.exit, Some(2), "{}", ende.stderr);
    assert!(ende.stderr.contains(FALSCH), "{}", ende.stderr);
    assert!(!wurzel.path().join("logs").exists());
}

/// `--plattenquote` als letztes Argument → Exit 2 mit „braucht einen Wert“.
#[test]
fn die_option_ohne_wert_ist_exit_2() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");

    let ende = aufrufen_in(wurzel.path(), &["--plattenquote"], &[]);

    assert_eq!(ende.exit, Some(2), "{}", ende.stderr);
    assert!(
        ende.stderr
            .contains("--plattenquote braucht einen Wert — Aufruf siehe localchatA --hilfe"),
        "{}",
        ende.stderr
    );
    assert!(!wurzel.path().join("logs").exists());
}

/// ADR-0044 Ergänzung Karte #26: eine gültige Option übersteuert eine falsche Umgebung — sie wird nicht gelesen.
/// (R00023 F0074 nennt hier noch Exit 2; die spätere Ergänzung der ADR geht vor, siehe Bericht Karte #34.)
#[test]
fn eine_gueltige_option_uebersteuert_eine_falsche_umgebung() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");

    let _instanz = starten_um(
        wurzel.path(),
        &["--plattenquote", "2MiB"],
        &[("LOCALCHATA_PLATTENQUOTE", "4")],
    );

    let start = start_eintrag(wurzel.path());
    assert!(
        start.contains(" plattenquote=2097152 belegt=0 option=--plattenquote "),
        "{start}"
    );
}

/// F0074 „Hilfe“: Exit 0, die Zeile zu `--plattenquote` wörtlich.
#[test]
fn die_hilfe_nennt_die_plattenquote() {
    let ende = aufrufen(&["--hilfe"]);

    assert_eq!(ende.exit, Some(0), "{}", ende.stderr);
    assert!(
        ende.stdout.contains("--plattenquote <größe>  Höchstgröße aller Gespräche (Tageslogs der Channels und Direktgespräche) wie 4GiB oder 512MiB, aus für keine Grenze; Standard 4GiB; auch über LOCALCHATA_PLATTENQUOTE"),
        "{}",
        ende.stdout
    );
}

/// F0074 „Wirksame Quote sichtbar“ (Start-Eintrag): Option, Umgebung, Standard mit Herkunft, leerer Ordner →
/// `belegt=0`; die Mutanten „Umgebung schlägt Option“ und „Herkunft immer `standard`“ werden rot. Geprüft wird die
/// Folge `plattenquote= belegt= option=`, denn `option=standard` der Protokollstufe steht schon davor.
#[test]
fn der_start_eintrag_nennt_quote_und_herkunft() {
    for (argumente, umgebung, erwartet) in [
        (
            &["--plattenquote", "512MiB"][..],
            &[("LOCALCHATA_PLATTENQUOTE", "1GiB")][..],
            " plattenquote=536870912 ",
        ),
        (
            &[][..],
            &[("LOCALCHATA_PLATTENQUOTE", "aus")][..],
            " plattenquote=aus ",
        ),
        (&[][..], &[][..], " plattenquote=4294967296 "),
    ] {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let _instanz = starten_um(wurzel.path(), argumente, umgebung);

        let start = start_eintrag(wurzel.path());
        let herkunft = match (argumente.is_empty(), umgebung.is_empty()) {
            (false, _) => "--plattenquote",
            (true, false) => "LOCALCHATA_PLATTENQUOTE",
            (true, true) => "standard",
        };
        assert!(
            start.contains(&format!("{erwartet}belegt=0 option={herkunft} ")),
            "{herkunft}: {start}"
        );
    }
}
