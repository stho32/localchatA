//! F0035 Tageswechsel (R00008, US-12) und „Mitternacht“ (R00017): die laufende Instanz pflegt nach Mitternacht
//! ohne Neustart — über den Tagestakt (Tagesaufgabe `fristpflege`). Start 2 s vor Mitternacht; Grenze Z3 = 4 s
//! nach der Startzeile, geprüft im Takt von 25 ms (R00008: Prüfintervall ≤ 50 ms). Die Regel selbst (Grenze,
//! rückwärts, Startwert) prüft der Tagestakt-Test aus R00014 mit gestellter Uhr.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::frist::{KURZ_VOR_MITTERNACHT, starten_um, tag_anlegen, tag_da};
use gemeinsam::protokoll::zeilen;
use std::time::{Duration, Instant};

/// Z3 aus R00008.
const Z3: Duration = Duration::from_secs(4);

fn frist_zeilen(ordner: &std::path::Path, tag: &str) -> Vec<String> {
    zeilen(ordner, tag)
        .into_iter()
        .filter(|zeile| zeile.contains(" frist "))
        .map(|zeile| zeile.split_once(' ').expect("Zeitstempel").1.to_string())
        .collect()
}

#[test]
fn us12_um_mitternacht_entfaellt_der_naechste_tag_und_steht_in_der_datei_des_neuen_tages() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let ordner = wurzel.path();
    for tag in ["2026-10-20", "2026-10-21", "2026-10-22", "2026-10-28"] {
        tag_anlegen(ordner, "allgemein", tag, 1);
    }

    let mut instanz = starten_um(ordner, KURZ_VOR_MITTERNACHT);
    let startzeile = Instant::now();

    assert!(!tag_da(ordner, "allgemein", "2026-10-20"));
    assert!(!tag_da(ordner, "allgemein", "2026-10-21"));
    assert!(tag_da(ordner, "allgemein", "2026-10-22"));
    assert!(tag_da(ordner, "allgemein", "2026-10-28"));
    assert_eq!(
        frist_zeilen(ordner, "2026-10-28"),
        vec![
            "INFORMATION frist 2 Tage entfallen (2026-10-20 bis 2026-10-21; Frist 7 Tage) | channel=allgemein anzahl=2 grund=start"
        ]
    );
    while tag_da(ordner, "allgemein", "2026-10-22")
        && startzeile.elapsed() < Duration::from_secs(10)
    {
        std::thread::sleep(Duration::from_millis(25));
    }
    let gemessen = startzeile.elapsed();
    println!("Z3 gemessen: {} ms", gemessen.as_millis());

    assert!(
        !tag_da(ordner, "allgemein", "2026-10-22"),
        "nach {gemessen:?} noch da"
    );
    assert!(gemessen <= Z3, "{gemessen:?} > Z3");
    assert!(tag_da(ordner, "allgemein", "2026-10-28"));
    let neuer_tag = frist_zeilen(ordner, "2026-10-29");
    assert_eq!(neuer_tag.len(), 1, "{neuer_tag:#?}");
    assert!(
        neuer_tag[0].starts_with("INFORMATION frist ")
            && neuer_tag[0].contains("2026-10-22")
            && neuer_tag[0].ends_with("| channel=allgemein anzahl=1 grund=tageswechsel"),
        "{neuer_tag:#?}"
    );
    assert!(
        !frist_zeilen(ordner, "2026-10-28")
            .iter()
            .any(|zeile| zeile.contains("grund=tageswechsel"))
    );
    instanz.signal_und_warten("TERM");
    let stderr = instanz.stderr_bis_zum_ende();
    assert!(!stderr.contains("entfallen"), "{stderr}");
}
