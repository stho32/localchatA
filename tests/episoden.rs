//! F0055 Grenzen, Episoden und Tagesbudget — über das echte Programm (R00014, US-13 bis US-15 und Kriterien).

mod gemeinsam;

use gemeinsam::aufrufen_mit;

#[test]
fn ruhe_null_ist_exit_2_und_nennt_die_variable() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let ordner = wurzel.path().display().to_string();

    let ende = aufrufen_mit(
        &[
            &ordner,
            "--port",
            "0",
            "--host",
            "127.0.0.1",
            "--kein-browser",
        ],
        &[("LOCALCHATA_EPISODE_RUHE_MS", "0")],
    );

    assert_eq!(ende.exit, Some(2), "{}", ende.stderr);
    assert!(
        ende.stderr.contains("LOCALCHATA_EPISODE_RUHE_MS"),
        "{}",
        ende.stderr
    );
    assert!(!wurzel.path().join("logs").exists());
}
