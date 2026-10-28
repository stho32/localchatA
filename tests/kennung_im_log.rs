//! F0007 Namensregel in der Kennung — über das echte Programm (R00003, US-5 und Kriterium mit 17 Sätzen):
//! eine Kopfzeile, deren Kennung die Regel aus ADR-0013 verletzt, ist beim Laden eine defekte Zeile.

mod gemeinsam;

use gemeinsam::protokoll::ort;
use gemeinsam::{gesundheit, starten_mit};
use std::fs;
use std::path::Path;

const TAG: &str = "channels/allgemein/2026-10-27.log";

/// Je Kennung ein Satz mit einer Textzeile; Satz `k` (ab 0) hat seine Kopfzeile in Zeile `3k + 1`.
fn log_schreiben(ordner: &Path, kennungen: &[String]) {
    let saetze: String = kennungen
        .iter()
        .enumerate()
        .map(|(k, kennung)| {
            format!("@ 2026-10-27T10:00:{k:02}.000+01:00 {kennung}\n  Satz {k}\n\n")
        })
        .collect();
    let ziel = ordner.join(TAG);
    fs::create_dir_all(ziel.parent().expect("Elternordner")).expect("Ordner");
    fs::write(ziel, saetze).expect("Datei");
}

/// Startet mit der Testuhr, liest den Health-Check, beendet geordnet; liefert Antwort und stderr.
async fn laden(ordner: &Path) -> (serde_json::Value, String) {
    let mut instanz = starten_mit(
        ordner,
        &["--port", "0", "--host", "127.0.0.1", "--kein-browser"],
        &[
            ("LOCALCHATA_JETZT", "2026-10-27T12:00:00+01:00"),
            ("TZ", "Europe/Berlin"),
        ],
    );
    let (status, inhalt) = gesundheit(&instanz.adresse).await;
    assert_eq!(status, 200);
    assert!(instanz.signal_und_warten("TERM").success());
    (inhalt, instanz.stderr_bis_zum_ende())
}

/// Die einzeln gemeldeten Defekte als `<datei>:<zeile>`, in der Reihenfolge der Datei (R00014: Protokollzeilen).
fn defekte_zeilen(stderr: &str) -> Vec<String> {
    stderr
        .lines()
        .filter(|zeile| zeile.contains(" zeile="))
        .map(ort)
        .collect()
}

fn erwartete_defekte(saetze: std::ops::Range<usize>) -> Vec<String> {
    saetze.map(|k| format!("{TAG}:{}", 3 * k + 1)).collect()
}

#[tokio::test]
async fn siebzehn_saetze_drei_gueltige_kennungen_vierzehn_defekte_mit_zeile() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let kennungen: Vec<String> = [
        "claude-2@127.0.0.1".to_string(),
        format!("{}@10.0.0.5", "a".repeat(32)),
        "shoff@fe80::1".to_string(),
        "Claude@127.0.0.1".to_string(),
        "jörg@127.0.0.1".to_string(),
        "chef\u{200B}@127.0.0.1".to_string(),
        "\u{202E}abc@127.0.0.1".to_string(),
        "\u{FEFF}abc@127.0.0.1".to_string(),
        "a_b@127.0.0.1".to_string(),
        "a.b@127.0.0.1".to_string(),
        "a/b@127.0.0.1".to_string(),
        "a\\b@127.0.0.1".to_string(),
        "a+b@127.0.0.1".to_string(),
        "-a@127.0.0.1".to_string(),
        "a-@127.0.0.1".to_string(),
        format!("{}@127.0.0.1", "a".repeat(33)),
        "claude@localhost".to_string(),
    ]
    .into();
    log_schreiben(wurzel.path(), &kennungen);

    let (inhalt, stderr) = laden(wurzel.path()).await;

    assert_eq!(inhalt["nachrichten"], 3, "{inhalt}");
    assert_eq!(inhalt["defekte_zeilen"], 14, "{inhalt}");
    // R00014: je Datei höchstens 10 einzeln, die übrigen 4 als eine Zeile „und 4 weitere“.
    assert_eq!(
        defekte_zeilen(&stderr),
        erwartete_defekte(3..13),
        "{stderr:?}"
    );
    assert_eq!(
        stderr
            .lines()
            .filter(|zeile| zeile.contains("und 4 weitere"))
            .count(),
        1,
        "{stderr:?}"
    );
}

#[tokio::test]
async fn us5_verwechselbare_kennungen_im_log_sind_defekt() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let kennungen: Vec<String> = [
        "claude-2@127.0.0.1",
        "shoff@fe80::1",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa@10.0.0.5",
        "Claude@127.0.0.1",
        "jörg@127.0.0.1",
        "a_b@127.0.0.1",
        "claude@localhost",
    ]
    .iter()
    .map(|kennung| kennung.to_string())
    .collect();
    log_schreiben(wurzel.path(), &kennungen);

    let (inhalt, stderr) = laden(wurzel.path()).await;

    assert_eq!(inhalt["nachrichten"], 3, "{inhalt}");
    assert_eq!(inhalt["defekte_zeilen"], 4, "{inhalt}");
    assert_eq!(
        defekte_zeilen(&stderr),
        erwartete_defekte(3..7),
        "{stderr:?}"
    );
}
