//! F0034 Entfallen beim Start und beim Verkürzen (R00008, US-7 bis US-11) mit den Frist-Zeilen im
//! Betriebsprotokoll (R00017) — über das echte Programm mit gestellter Uhr (heute 28.10.2026).
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::Absender;
use gemeinsam::frist::{
    MITTAG, beitreten, channel_in_liste, datei_anlegen, nachrichten, starten_um, tag_anlegen,
    tag_da,
};
use gemeinsam::protokoll::{mit, zeilen};
use serde_json::Value;
use std::fs;

const HEUTE: &str = "2026-10-28";

/// Die Frist-Zeilen ab der Stufe, ohne Zeitstempel.
fn frist_zeilen(zeilen: &[String]) -> Vec<String> {
    zeilen
        .iter()
        .filter(|zeile| zeile.contains(" frist "))
        .map(|zeile| zeile.split_once(' ').expect("Zeitstempel").1.to_string())
        .collect()
}

/// US-7, Kriterien „Start“ (R00008) und „Start“ (R00017): weg, sobald die Startzeile da ist; die Zeile steht vor
/// `INFORMATION start`, stderr schweigt.
#[tokio::test]
async fn us7_der_start_raeumt_alte_tage_weg_und_schreibt_es_vor_der_startmeldung() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    for tag in ["2026-10-20", "2026-10-21", "2026-10-22", "2026-10-28"] {
        tag_anlegen(wurzel.path(), "allgemein", tag, 1);
    }

    let mut instanz = starten_um(wurzel.path(), MITTAG);

    assert!(!tag_da(wurzel.path(), "allgemein", "2026-10-20"));
    assert!(!tag_da(wurzel.path(), "allgemein", "2026-10-21"));
    assert!(tag_da(wurzel.path(), "allgemein", "2026-10-22"));
    assert!(tag_da(wurzel.path(), "allgemein", "2026-10-28"));
    let protokoll = zeilen(wurzel.path(), HEUTE);
    let erwartet = "INFORMATION frist 2 Tage entfallen (2026-10-20 bis 2026-10-21; Frist 7 Tage) | channel=allgemein anzahl=2 grund=start";
    assert_eq!(frist_zeilen(&protokoll), vec![erwartet]);
    let frist_an = protokoll.iter().position(|zeile| zeile.ends_with(erwartet));
    let start_an = protokoll
        .iter()
        .position(|zeile| zeile.contains(" INFORMATION start Instanz bereit"));
    assert!(frist_an < start_an && start_an.is_some(), "{protokoll:#?}");
    assert_eq!(nachrichten(&instanz).await, 2);
    instanz.signal_und_warten("TERM");
    let stderr = instanz.stderr_bis_zum_ende();
    assert!(!stderr.contains("entfallen"), "{stderr}");
}

/// US-11, Kriterien „Defekt heißt nichts“ (R00008) und „Einstellungen defekt bleibt laut“ (R00017).
#[tokio::test]
async fn us11_kaputte_einstellungen_sperren_das_loeschen_und_melden_sich_in_datei_und_stderr() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    datei_anlegen(
        wurzel.path(),
        "channels/kaputt/einstellungen.toml",
        b"frist_tage = \"sieben\"\n",
    );
    datei_anlegen(
        wurzel.path(),
        "channels/zu-klein/einstellungen.toml",
        b"frist_tage = 0\n",
    );
    tag_anlegen(wurzel.path(), "kaputt", "2026-01-01", 1);
    tag_anlegen(wurzel.path(), "zu-klein", "2026-01-01", 1);

    let mut instanz = starten_um(wurzel.path(), MITTAG);

    assert!(tag_da(wurzel.path(), "kaputt", "2026-01-01"));
    assert!(tag_da(wurzel.path(), "zu-klein", "2026-01-01"));
    assert_eq!(
        fs::read(wurzel.path().join("channels/kaputt/einstellungen.toml")).expect("Datei"),
        b"frist_tage = \"sieben\"\n"
    );
    assert_eq!(
        fs::read(wurzel.path().join("channels/zu-klein/einstellungen.toml")).expect("Datei"),
        b"frist_tage = 0\n"
    );
    let eins = Absender::von("127.0.0.1");
    let beta = beitreten(&eins, &instanz, "beta").await;
    for name in ["kaputt", "zu-klein"] {
        let (channel, _) = channel_in_liste(&eins, &instanz, &beta, name).await;
        assert_eq!(channel["frist_tage"], Value::Null, "{name}");
    }
    let erwartet = [
        "WARNING frist Nichts entfällt — Einstellungen defekt | channel=kaputt datei=channels/kaputt/einstellungen.toml zeile=1",
        "WARNING frist Nichts entfällt — Einstellungen defekt | channel=zu-klein datei=channels/zu-klein/einstellungen.toml zeile=1",
    ];
    assert_eq!(frist_zeilen(&zeilen(wurzel.path(), HEUTE)), erwartet);
    instanz.signal_und_warten("TERM");
    let stderr: Vec<String> = instanz
        .stderr_bis_zum_ende()
        .lines()
        .map(str::to_string)
        .collect();
    assert_eq!(frist_zeilen(&stderr), erwartet, "{stderr:#?}");
    assert_eq!(mit(&stderr, "WARNING", "laden").len(), 2, "{stderr:#?}");
}
