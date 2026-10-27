//! Exit-Codes und Meldungen über das echte Programm (US-4, US-5, US-6, Kriterium belegter Port).

mod gemeinsam;

use gemeinsam::{aufrufen, lokal_starten};

fn genau_eine_zeile(text: &str) -> &str {
    let zeilen: Vec<&str> = text.lines().collect();
    assert_eq!(zeilen.len(), 1, "genau eine Zeile erwartet: {text:?}");
    zeilen[0]
}

#[test]
fn us4_hilfe_zeigt_die_aufrufzeile_mit_allen_optionen_und_endet_mit_exit_null() {
    let ende = aufrufen(&["--hilfe"]);

    assert_eq!(ende.exit, Some(0));
    let aufrufzeile = ende.stdout.lines().next().unwrap_or_default();
    assert!(
        aufrufzeile.starts_with("Aufruf: localchatA"),
        "{aufrufzeile}"
    );
    for option in ["<ordner>", "--port", "--host", "--kein-browser", "--hilfe"] {
        assert!(
            aufrufzeile.contains(option),
            "{option} fehlt in {aufrufzeile}"
        );
    }
    assert_eq!(ende.stderr, "");
}

#[test]
fn us4_unbekannte_option_endet_mit_exit_zwei_und_verweist_auf_die_hilfe() {
    let ende = aufrufen(&["--gibtsnicht"]);

    assert_eq!(ende.exit, Some(2));
    let zeile = genau_eine_zeile(&ende.stderr);
    assert!(zeile.contains("--gibtsnicht"), "{zeile}");
    assert!(zeile.contains("--hilfe"), "{zeile}");
    assert_eq!(ende.stdout, "");
}

#[test]
fn fehlender_portwert_ist_ebenfalls_ein_falscher_aufruf_mit_exit_zwei() {
    assert_eq!(aufrufen(&["--port"]).exit, Some(2));
}

#[test]
fn us5_nicht_anlegbarer_ordner_endet_mit_exit_eins_und_nennt_den_ordner() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let datei = wurzel.path().join("lc-datei");
    std::fs::write(&datei, "").expect("Datei angelegt");
    let ordner = datei.join("unter");
    let ordner_text = ordner.display().to_string();

    let ende = aufrufen(&[&ordner_text, "--port", "0", "--kein-browser"]);

    assert_eq!(ende.exit, Some(1));
    assert!(
        genau_eine_zeile(&ende.stderr).contains(&ordner_text),
        "{}",
        ende.stderr
    );
    assert_eq!(ende.stdout, "");
}

#[test]
fn ausdruecklich_genannter_belegter_port_endet_mit_exit_eins_ohne_ausweichen() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let belegt = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("belegt");
    let port = belegt.local_addr().expect("Adresse").port().to_string();
    let ordner = wurzel.path().display().to_string();

    let ende = aufrufen(&[
        &ordner,
        "--port",
        &port,
        "--host",
        "127.0.0.1",
        "--kein-browser",
    ]);

    assert_eq!(ende.exit, Some(1));
    assert!(
        genau_eine_zeile(&ende.stderr).contains(&port),
        "{}",
        ende.stderr
    );
    assert_eq!(ende.stdout, "", "keine Startzeile, also kein Ausweichen");
}

#[test]
fn us6_zweiter_start_nennt_ordner_und_sperre_in_einer_zeile() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let _erste = lokal_starten(wurzel.path());
    let ordner = wurzel
        .path()
        .canonicalize()
        .expect("absolut")
        .display()
        .to_string();

    let ende = aufrufen(&[
        &ordner,
        "--port",
        "0",
        "--host",
        "127.0.0.1",
        "--kein-browser",
    ]);

    assert_eq!(ende.exit, Some(1));
    let zeile = genau_eine_zeile(&ende.stderr);
    assert!(zeile.contains(&ordner), "{zeile}");
    assert!(
        zeile.contains("bereits von einer Instanz gesperrt"),
        "{zeile}"
    );
}
