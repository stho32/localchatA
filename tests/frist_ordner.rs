//! R00019 F0063 Ordner nicht lesbar — über das echte Programm. Nach der Startzeile nimmt `chmod a-r` dem Ordner
//! eines Channels das Leserecht (Auflisten scheitert, Dateien mit bekanntem Namen bleiben erreichbar). Läuft nicht
//! als root (dort wirkt `chmod` nicht).
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::frist::{KURZ_VOR_MITTERNACHT, Rechte, starten_um, tag_anlegen, tag_da};
use gemeinsam::protokoll::zeilen;
use std::time::{Duration, Instant};

/// Z3 und Z4 aus R00008.
const Z3: Duration = Duration::from_secs(4);
const Z4: Duration = Duration::from_secs(3);

const TAGE_NICHT_ERMITTELT: &str = "ERROR frist Tage nicht ermittelt — Ordner nicht lesbar; erneuter Versuch beim nächsten Anlass | channel=allgemein datei=channels/allgemein";

fn frist_zeilen(ordner: &std::path::Path, tag: &str) -> Vec<String> {
    zeilen(ordner, tag)
        .into_iter()
        .filter(|zeile| zeile.contains(" frist "))
        .map(|zeile| zeile.split_once(' ').expect("Zeitstempel").1.to_string())
        .collect()
}

/// R00019 F0063 „Ruhe am Tageswechsel“: `allgemein` mit 20.(1), 28.(1), `zweiter` mit 20.(1), 22.(1); sofort nach
/// der Startzeile `chmod a-r channels/allgemein` → innerhalb Z3 genau eine Zeile „Tage nicht ermittelt“ mit
/// `grund=tageswechsel` und für `zweiter` das Entfallen des 22.; in Z4 danach keine weitere Frist-Zeile.
#[test]
fn am_tageswechsel_meldet_ein_nicht_lesbarer_ordner_sich_einmal_die_uebrigen_werden_gepflegt() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let ordner = wurzel.path();
    tag_anlegen(ordner, "allgemein", "2026-10-20", 1);
    tag_anlegen(ordner, "allgemein", "2026-10-28", 1);
    tag_anlegen(ordner, "zweiter", "2026-10-20", 1);
    tag_anlegen(ordner, "zweiter", "2026-10-22", 1);

    let mut instanz = starten_um(ordner, KURZ_VOR_MITTERNACHT);
    let startzeile = Instant::now();
    let _leseschutz = Rechte::setzen(&ordner.join("channels/allgemein"), 0o311);
    let stderr = instanz.stderr_mitschreiben();

    let tageswechsel = || -> Vec<String> {
        frist_zeilen(ordner, "2026-10-29")
            .into_iter()
            .filter(|zeile| zeile.contains("grund=tageswechsel"))
            .collect()
    };
    while tageswechsel().len() < 2 && startzeile.elapsed() < Duration::from_secs(10) {
        std::thread::sleep(Duration::from_millis(25));
    }
    let gemessen = startzeile.elapsed();
    println!("Z3 gemessen: {} ms", gemessen.as_millis());
    let mut neuer_tag = frist_zeilen(ordner, "2026-10-29");
    neuer_tag.sort();

    assert!(gemessen <= Z3, "{gemessen:?} > Z3, {neuer_tag:#?}");
    assert_eq!(neuer_tag.len(), 2, "{neuer_tag:#?}");
    assert!(
        neuer_tag[0].starts_with(&format!(
            "{TAGE_NICHT_ERMITTELT} grund=tageswechsel ursache=\""
        )) && neuer_tag[0].ends_with('"'),
        "{neuer_tag:#?}"
    );
    assert!(
        neuer_tag[1].starts_with("INFORMATION frist 1 Tag entfallen (2026-10-22;")
            && neuer_tag[1].ends_with("| channel=zweiter anzahl=1 grund=tageswechsel"),
        "{neuer_tag:#?}"
    );
    assert!(!tag_da(ordner, "zweiter", "2026-10-22"));
    assert!(
        !frist_zeilen(ordner, "2026-10-28")
            .iter()
            .chain(&neuer_tag)
            .any(|zeile| zeile.contains("Tag nicht entfallen")),
        "{neuer_tag:#?}"
    );

    std::thread::sleep(Z4);

    assert_eq!(frist_zeilen(ordner, "2026-10-29").len(), 2);
    let auf_stderr: Vec<String> = stderr
        .lock()
        .expect("stderr")
        .iter()
        .filter(|zeile| zeile.contains(" frist "))
        .cloned()
        .collect();
    assert_eq!(auf_stderr.len(), 1, "{auf_stderr:#?}");
    assert!(
        auf_stderr[0].contains(TAGE_NICHT_ERMITTELT),
        "{auf_stderr:#?}"
    );
}
