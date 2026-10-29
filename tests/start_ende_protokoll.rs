//! R00018 F0082 Start, Ende und Sitzungswechsel lückenlos im Protokoll — über das echte Programm.
#![cfg(unix)]

mod gemeinsam;

use gemeinsam::starten_mit;
use std::path::Path;

const LOKAL: [&str; 5] = ["--port", "0", "--host", "127.0.0.1", "--kein-browser"];

/// Alle Zeilen aller Tagesdateien unter `logs/`, nach Dateinamen (Datum) geordnet.
fn alle_zeilen(ordner: &Path) -> Vec<String> {
    let mut dateien: Vec<_> = std::fs::read_dir(ordner.join("logs"))
        .expect("logs/")
        .flatten()
        .map(|eintrag| eintrag.path())
        .collect();
    dateien.sort();
    dateien
        .iter()
        .flat_map(|datei| {
            std::fs::read_to_string(datei)
                .unwrap_or_default()
                .lines()
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .collect()
}

/// B0383 (ADR-0042 Ergänzung Karte #26 Punkt 2): eine beim Ende offene Episode schreibt ihre Ende-Zeile nach der
/// ersten `INFORMATION ende` und vor „beendet“ — die Ruhe von 60 s ist nicht abgelaufen.
#[tokio::test]
async fn eine_beim_ende_offene_episode_endet_vor_beendet() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let mut instanz = starten_mit(
        wurzel.path(),
        &LOKAL,
        &[("LOCALCHATA_EPISODE_RUHE_MS", "60000")],
    );
    let antwort = reqwest::Client::new()
        .post(format!("{}api/beitreten", instanz.adresse))
        .header("Origin", "http://fremd.example")
        .json(&serde_json::json!({"name": "beta"}))
        .send()
        .await
        .expect("Antwort");
    assert_eq!(antwort.status().as_u16(), 403);

    let ende = instanz.signal_und_warten("TERM");

    assert_eq!(ende.code(), Some(0));
    let zeilen = alle_zeilen(wurzel.path());
    let stelle = |teil: &str| zeilen.iter().position(|zeile| zeile.contains(teil));
    let beginn = stelle(" WARNING grenzschutz ").expect("Episode beginnt");
    assert!(zeilen[beginn].contains("— Episode beginnt"), "{zeilen:#?}");
    let signal = stelle(" INFORMATION ende Ende-Signal empfangen").expect("erste ende-Zeile");
    assert!(zeilen[signal].contains("grund=SIGTERM"), "{zeilen:#?}");
    let enden: Vec<usize> = zeilen
        .iter()
        .enumerate()
        .filter(|(_, zeile)| {
            zeile.contains(" WARNING grenzschutz ") && zeile.contains("— Episode endet: 1 in ")
        })
        .map(|(nummer, _)| nummer)
        .collect();
    assert_eq!(enden.len(), 1, "{zeilen:#?}");
    let beendet = zeilen.len() - 1;
    assert!(
        zeilen[beendet].contains(" INFORMATION ende Instanz beendet"),
        "„beendet“ ist die letzte Zeile: {zeilen:#?}"
    );
    assert!(signal < enden[0] && enden[0] < beendet, "{zeilen:#?}");
}

/// B0384 (R00014 „Ort“): `logs/` liegt nach dem Start da, auch wenn die Mindeststufe noch nichts schreibt;
/// gegengeprüft: eine falsche Stufe aus der Umgebung ist Exit 2 ohne `logs/`.
#[test]
fn logs_liegt_nach_dem_start_auch_auf_stufe_warning_da() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let mut argumente = LOKAL.to_vec();
    argumente.extend_from_slice(&["--protokoll-stufe", "warning"]);

    let instanz = starten_mit(wurzel.path(), &argumente, &[]);

    assert!(
        wurzel.path().join("logs").is_dir(),
        "nach der Startzeile {}",
        instanz.startzeile
    );

    let falsch = tempfile::tempdir().expect("Wegwerf-Ordner");
    let ordner = falsch.path().display().to_string();
    let mut argumente = vec![ordner.as_str()];
    argumente.extend_from_slice(&LOKAL);
    let ende = gemeinsam::aufrufen_mit(&argumente, &[("LOCALCHATA_PROTOKOLL_STUFE", "laut")]);
    assert_eq!(ende.exit, Some(2), "{}", ende.stderr);
    assert!(!falsch.path().join("logs").exists());
}
