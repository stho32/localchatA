//! F0002 Ordner gegen zweite Instanz sperren — über das echte Programm (US-6, US-7).

mod gemeinsam;

use gemeinsam::{PROGRAMM, gesundheit, lokal_starten, warten};
use std::process::{Command, Stdio};
use std::time::Duration;

fn zweiter_start(ordner: &std::path::Path) -> (Option<i32>, String) {
    let mut kind = Command::new(PROGRAMM)
        .arg(ordner)
        .args(["--port", "0", "--host", "127.0.0.1", "--kein-browser"])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("zweiter Start");
    let ende = warten(&mut kind, Duration::from_secs(5));
    if ende.is_none() {
        let _ = kind.kill();
    }
    let ausgabe = kind.wait_with_output().expect("Ausgabe");
    (
        ende.and_then(|status| status.code()),
        String::from_utf8_lossy(&ausgabe.stderr).into_owned(),
    )
}

#[tokio::test]
async fn us6_waehrend_eine_instanz_laeuft_liegt_die_sperre_und_ein_zweiter_start_endet_mit_exit_eins()
 {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let erste = lokal_starten(wurzel.path());
    assert!(wurzel.path().join(".localchatA.sperre").is_file());

    let (exit, _) = zweiter_start(wurzel.path());

    assert_eq!(exit, Some(1));
    assert_eq!(
        gesundheit(&erste.adresse).await.0,
        200,
        "die erste Instanz lebt weiter"
    );
}

#[tokio::test]
async fn us7_nach_dem_ende_der_ersten_startet_eine_neue_trotz_liegender_sperrdatei() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let mut erste = lokal_starten(wurzel.path());
    assert_eq!(erste.signal_und_warten("INT").code(), Some(0));
    assert!(wurzel.path().join(".localchatA.sperre").is_file());

    let neue = lokal_starten(wurzel.path());

    assert_eq!(gesundheit(&neue.adresse).await.0, 200);
}
