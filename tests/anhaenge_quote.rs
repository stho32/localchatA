//! R00029 F0097 „Anhang-Quote einstellen“ — über das echte Programm: falsche Werte enden mit Exit 2 und einer
//! stderr-Zeile, die Option bzw. Variable, Wert und Spanne nennt; kein `logs/`. Die Werte im Health-Check prüft
//! `anhaenge_kennzahlen.rs`.

mod gemeinsam;

use gemeinsam::{aufrufen, aufrufen_mit};
use std::path::Path;

const LOKAL: [&str; 5] = ["--port", "0", "--host", "127.0.0.1", "--kein-browser"];
const SPANNE: &str = "braucht eine Größe von 1 MiB bis 16.384 GiB wie 20GiB oder 512MiB, oder aus";

fn aufrufen_in(ordner: &Path, weitere: &[&str], umgebung: &[(&str, &str)]) -> gemeinsam::Ende {
    let ordner = ordner.display().to_string();
    let mut argumente = vec![ordner.as_str()];
    argumente.extend_from_slice(&LOKAL);
    argumente.extend_from_slice(weitere);
    aufrufen_mit(&argumente, umgebung)
}

#[test]
fn eine_falsche_anhangquote_als_option_ist_exit_2_mit_option_wert_und_spanne() {
    for falsch in ["0MiB", "20000", "20 GiB", "16385GiB", "512KiB", ""] {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");

        let ende = aufrufen_in(wurzel.path(), &["--anhangquote", falsch], &[]);

        assert_eq!(ende.exit, Some(2), "{falsch:?}: {}", ende.stderr);
        let erwartet = format!("--anhangquote {SPANNE} — nicht „{falsch}“");
        assert!(
            ende.stderr.contains(&erwartet),
            "{falsch:?}: {}",
            ende.stderr
        );
        assert_eq!(ende.stderr.trim_end().lines().count(), 1, "{}", ende.stderr);
        assert!(!wurzel.path().join("logs").exists(), "{falsch:?}");
    }
}

#[test]
fn eine_falsche_anhangquote_in_der_umgebung_ist_exit_2_ohne_logs() {
    for falsch in ["0GiB", "kaputt", ""] {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");

        let ende = aufrufen_in(wurzel.path(), &[], &[("LOCALCHATA_ANHANGQUOTE", falsch)]);

        assert_eq!(ende.exit, Some(2), "{falsch:?}: {}", ende.stderr);
        let erwartet = format!("LOCALCHATA_ANHANGQUOTE {SPANNE} — nicht „{falsch}“");
        assert!(
            ende.stderr.contains(&erwartet),
            "{falsch:?}: {}",
            ende.stderr
        );
        assert!(!wurzel.path().join("logs").exists(), "{falsch:?}");
    }
}

#[test]
fn die_hilfe_nennt_die_anhangquote() {
    let ende = aufrufen(&["--hilfe"]);
    assert_eq!(ende.exit, Some(0));
    let woerter = |text: &str| text.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        woerter(&ende.stdout).contains("--anhangquote <größe> Höchstgröße aller Anhänge zusammen wie 20GiB oder 512MiB, aus für keine Grenze; ist sie voll, wird Hochladen abgewiesen, bis Anhänge gelöscht werden oder entfallen; Standard 20GiB; auch über LOCALCHATA_ANHANGQUOTE"),
        "{}",
        ende.stdout
    );
}
