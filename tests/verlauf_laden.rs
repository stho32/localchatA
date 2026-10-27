//! F0004 Verlauf beim Start laden — über das echte Programm (R00002, US-1 bis US-3).

mod gemeinsam;

use gemeinsam::aufrufen_mit;

#[test]
fn us3_unlesbare_startzeit_endet_mit_exit_zwei_und_nennt_die_variable() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let ordner = wurzel.path().display().to_string();

    let ende = aufrufen_mit(
        &[&ordner, "--port", "0", "--kein-browser"],
        &[("LOCALCHATA_JETZT", "gestern"), ("TZ", "Europe/Berlin")],
    );

    assert_eq!(ende.exit, Some(2), "stderr: {}", ende.stderr);
    let zeilen: Vec<&str> = ende.stderr.lines().collect();
    assert_eq!(zeilen.len(), 1, "{:?}", ende.stderr);
    assert!(zeilen[0].contains("LOCALCHATA_JETZT"), "{}", zeilen[0]);
    assert_eq!(ende.stdout, "");
}
