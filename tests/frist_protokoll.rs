//! F0058 Frist im Betriebsprotokoll (R00017, US-1): eine Friständerung steht als `INFORMATION frist` in
//! `logs/<heute>.log`, nicht auf stderr; derselbe Wert und abgewiesene Werte schreiben nichts. Die übrigen
//! Kriterien stehen bei ihren Abläufen: Start und Bestätigt verkürzen in `frist_entfallen.rs`, Mitternacht in
//! `frist_tageswechsel.rs`, Nicht löschbar und Schreibfehler in `frist_schutz.rs`.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::Absender;
use gemeinsam::frist::{MITTAG, beitreten, frist_setzen, starten_um};
use gemeinsam::protokoll::zeilen;
use serde_json::json;

fn frist_zeilen(ordner: &std::path::Path) -> Vec<String> {
    zeilen(ordner, "2026-10-28")
        .into_iter()
        .filter(|zeile| zeile.contains(" frist "))
        .map(|zeile| zeile.split_once(' ').expect("Zeitstempel").1.to_string())
        .collect()
}

/// Kriterien „Setzen“ und „Derselbe Wert und abgewiesene Werte“ (Mutanten M1, M2, M9).
#[tokio::test]
async fn us1_eine_fristaenderung_steht_in_der_datei_nicht_im_terminal() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    std::fs::create_dir_all(wurzel.path().join("channels/allgemein")).expect("Ordner");
    let mut instanz = starten_um(wurzel.path(), MITTAG);
    let eins = Absender::von("127.0.0.1");
    let beta = beitreten(&eins, &instanz, "beta").await;

    let gesetzt = frist_setzen(&eins, &instanz, &beta, "allgemein", json!({ "tage": 30 })).await;

    assert_eq!(gesetzt.status, 200, "{}", gesetzt.json);
    let erwartet = vec![
        "INFORMATION frist Frist 7 → 30 Tage gesetzt | channel=allgemein kennung=beta@127.0.0.1",
    ];
    assert_eq!(frist_zeilen(wurzel.path()), erwartet);

    let derselbe = frist_setzen(&eins, &instanz, &beta, "allgemein", json!({ "tage": 30 })).await;
    let abgewiesen = frist_setzen(&eins, &instanz, &beta, "allgemein", json!({ "tage": 0 })).await;

    assert_eq!((derselbe.status, abgewiesen.status), (200, 422));
    assert_eq!(frist_zeilen(wurzel.path()), erwartet);
    instanz.signal_und_warten("TERM");
    let stderr = instanz.stderr_bis_zum_ende();
    assert!(!stderr.contains("Frist 7 → 30"), "{stderr}");
    assert!(!stderr.contains("INFORMATION"), "{stderr}");
}
