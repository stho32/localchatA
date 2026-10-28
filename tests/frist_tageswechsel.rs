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

/// Die Frist-Zeilen `grund=tageswechsel` in der Datei des neuen Tages, abgewartet im Takt von 25 ms höchstens bis
/// 10 s nach `startzeile`, bis es `erwartet` sind — das Programm trägt erst nach der Pflege aller Channels ein
/// (Code-Review #12, Sollte 1). Liefert die Zeilen und die Zeit von der Startzeile bis zu ihrem Erscheinen.
fn tageswechsel_abwarten(
    ordner: &std::path::Path,
    startzeile: Instant,
    erwartet: usize,
) -> (Vec<String>, Duration) {
    let tageswechsel = || -> Vec<String> {
        frist_zeilen(ordner, "2026-10-29")
            .into_iter()
            .filter(|zeile| zeile.contains("grund=tageswechsel"))
            .collect()
    };
    while tageswechsel().len() < erwartet && startzeile.elapsed() < Duration::from_secs(10) {
        std::thread::sleep(Duration::from_millis(25));
    }
    let gemessen = startzeile.elapsed();
    println!("Z3 gemessen: {} ms", gemessen.as_millis());
    (tageswechsel(), gemessen)
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
    let (neuer_tag, gemessen) = tageswechsel_abwarten(ordner, startzeile, 1);

    assert!(gemessen <= Z3, "{gemessen:?} > Z3, {neuer_tag:#?}");
    assert_eq!(frist_zeilen(ordner, "2026-10-29"), neuer_tag);
    assert_eq!(neuer_tag.len(), 1, "{neuer_tag:#?}");
    assert!(
        neuer_tag[0].starts_with("INFORMATION frist ")
            && neuer_tag[0].contains("2026-10-22")
            && neuer_tag[0].ends_with("| channel=allgemein anzahl=1 grund=tageswechsel"),
        "{neuer_tag:#?}"
    );
    assert!(!tag_da(ordner, "allgemein", "2026-10-22"));
    assert!(tag_da(ordner, "allgemein", "2026-10-28"));
    assert!(
        !frist_zeilen(ordner, "2026-10-28")
            .iter()
            .any(|zeile| zeile.contains("grund=tageswechsel"))
    );
    instanz.signal_und_warten("TERM");
    let stderr = instanz.stderr_bis_zum_ende();
    assert!(!stderr.contains("entfallen"), "{stderr}");
}

/// Abnahme-Probe Code-Review #12, Sollte 1 (Variante zwei Channels): die Pflege beider Channels endet vor dem
/// Eintragen; je Channel eine Zeile in der Datei des neuen Tages, danach sind beide 22. weg.
#[test]
fn us12_mit_zwei_channels_steht_je_channel_eine_zeile_im_neuen_tag_und_beide_tage_sind_weg() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let ordner = wurzel.path();
    for channel in ["allgemein", "zweiter"] {
        tag_anlegen(ordner, channel, "2026-10-22", 1);
        tag_anlegen(ordner, channel, "2026-10-28", 1);
    }

    let mut instanz = starten_um(ordner, KURZ_VOR_MITTERNACHT);
    let startzeile = Instant::now();

    assert!(frist_zeilen(ordner, "2026-10-28").is_empty());
    let (mut neuer_tag, gemessen) = tageswechsel_abwarten(ordner, startzeile, 2);

    assert!(gemessen <= Z3, "{gemessen:?} > Z3, {neuer_tag:#?}");
    neuer_tag.sort();
    assert_eq!(neuer_tag.len(), 2, "{neuer_tag:#?}");
    for (zeile, channel) in neuer_tag.iter().zip(["allgemein", "zweiter"]) {
        assert!(
            zeile.starts_with("INFORMATION frist ")
                && zeile.contains("2026-10-22")
                && zeile.ends_with(&format!("| channel={channel} anzahl=1 grund=tageswechsel")),
            "{neuer_tag:#?}"
        );
        assert!(!tag_da(ordner, channel, "2026-10-22"), "{channel}");
        assert!(tag_da(ordner, channel, "2026-10-28"), "{channel}");
    }
    instanz.signal_und_warten("TERM");
    let stderr = instanz.stderr_bis_zum_ende();
    assert!(!stderr.contains("entfallen"), "{stderr}");
}
