//! F0003 Browser öffnen — über das echte Programm; `LOCALCHATA_BROWSER` ersetzt den Standardbrowser (US-8, US-9).
#![cfg(unix)]

mod gemeinsam;

use gemeinsam::{gesundheit, starten_mit};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Ein Ersatzbrowser, der die aufgerufene Adresse in eine Datei schreibt.
fn ersatzbrowser(ordner: &Path) -> (PathBuf, PathBuf) {
    let mitschrift = ordner.join("aufgerufen.txt");
    let programm = ordner.join("browser.sh");
    std::fs::write(
        &programm,
        format!("#!/bin/sh\necho \"$1\" > '{}'\n", mitschrift.display()),
    )
    .expect("Ersatzbrowser geschrieben");
    std::fs::set_permissions(&programm, std::fs::Permissions::from_mode(0o755))
        .expect("ausführbar");
    (programm, mitschrift)
}

fn aufgerufene_adresse(mitschrift: &Path) -> String {
    let ende = Instant::now() + Duration::from_secs(5);
    while Instant::now() < ende {
        if let Ok(inhalt) = std::fs::read_to_string(mitschrift)
            && inhalt.ends_with('\n')
        {
            return inhalt.trim_end().to_string();
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("der Browser wurde nicht geöffnet");
}

#[tokio::test]
async fn us8_bei_allen_schnittstellen_oeffnet_der_browser_die_eigene_maschine() {
    let werkzeug = tempfile::tempdir().expect("Werkzeug-Ordner");
    let ordner = tempfile::tempdir().expect("Wegwerf-Ordner");
    let (programm, mitschrift) = ersatzbrowser(werkzeug.path());
    let programm = programm.display().to_string();

    let instanz = starten_mit(
        ordner.path(),
        &["--port", "0"],
        &[("LOCALCHATA_BROWSER", &programm)],
    );

    assert_eq!(
        aufgerufene_adresse(&mitschrift),
        format!("http://127.0.0.1:{}/", instanz.port())
    );
}

#[tokio::test]
async fn bei_bestimmtem_host_oeffnet_der_browser_die_adresse_der_startzeile() {
    let werkzeug = tempfile::tempdir().expect("Werkzeug-Ordner");
    let ordner = tempfile::tempdir().expect("Wegwerf-Ordner");
    let (programm, mitschrift) = ersatzbrowser(werkzeug.path());
    let programm = programm.display().to_string();

    let instanz = starten_mit(
        ordner.path(),
        &["--port", "0", "--host", "127.0.0.1"],
        &[("LOCALCHATA_BROWSER", &programm)],
    );

    assert_eq!(aufgerufene_adresse(&mitschrift), instanz.adresse);
}

#[tokio::test]
async fn laesst_sich_kein_browser_starten_steht_ein_hinweis_auf_stderr_und_die_instanz_laeuft() {
    for ersatz in ["/gibt/es/nicht/browser", "/bin/false"] {
        let ordner = tempfile::tempdir().expect("Wegwerf-Ordner");
        let mut instanz = starten_mit(
            ordner.path(),
            &["--port", "0", "--host", "127.0.0.1"],
            &[("LOCALCHATA_BROWSER", ersatz)],
        );

        let hinweis = instanz
            .stderr_zeile_abwarten(Duration::from_secs(5))
            .unwrap_or_else(|| panic!("{ersatz}: kein Hinweis auf stderr"));

        assert!(hinweis.contains("Browser"), "{ersatz}: {hinweis:?}");
        assert!(hinweis.contains(&instanz.adresse), "{ersatz}: {hinweis:?}");
        assert_eq!(gesundheit(&instanz.adresse).await.0, 200, "{ersatz}");
    }
}

#[tokio::test]
async fn us9_mit_kein_browser_oeffnet_sich_kein_browser() {
    let werkzeug = tempfile::tempdir().expect("Werkzeug-Ordner");
    let ordner = tempfile::tempdir().expect("Wegwerf-Ordner");
    let (programm, mitschrift) = ersatzbrowser(werkzeug.path());
    let programm = programm.display().to_string();

    let mut instanz = starten_mit(
        ordner.path(),
        &["--port", "0", "--host", "127.0.0.1", "--kein-browser"],
        &[("LOCALCHATA_BROWSER", &programm)],
    );
    assert_eq!(gesundheit(&instanz.adresse).await.0, 200);
    assert_eq!(instanz.signal_und_warten("TERM").code(), Some(0));
    // Der Gegenfall (us8) schreibt binnen Millisekunden; eine Sekunde nach dem Ende ist nichts gekommen.
    std::thread::sleep(Duration::from_secs(1));

    assert!(
        !mitschrift.exists(),
        "Browser wurde trotz --kein-browser geöffnet"
    );
}
