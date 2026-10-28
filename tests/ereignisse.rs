//! F0012 Anwesenheit live im Ereignisstrom — über das echte Programm (R00004, US-5 bis US-13).
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::aufrufen_mit;

#[test]
fn us11_falscher_herzschlag_wert_verhindert_den_start() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let ordner = wurzel.path().display().to_string();

    let ende = aufrufen_mit(
        &[&ordner, "--port", "0", "--kein-browser"],
        &[("LOCALCHATA_HERZSCHLAG_MS", "bald")],
    );

    assert_eq!(ende.exit, Some(2));
    assert!(
        ende.stderr.contains("LOCALCHATA_HERZSCHLAG_MS"),
        "{}",
        ende.stderr
    );
    assert_eq!(ende.stdout, "", "keine Startzeile");
}
