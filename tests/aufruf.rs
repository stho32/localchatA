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
    for option in [
        "<ordner>",
        "--port",
        "--host",
        "--kein-browser",
        "--erreichbar-als",
        "--hilfe",
    ] {
        assert!(
            aufrufzeile.contains(option),
            "{option} fehlt in {aufrufzeile}"
        );
    }
    assert_eq!(ende.stderr, "");
}

/// R00018 F0062: `--protokoll-stufe <stufe>` steht auf eigener Zeile, die Erklärung darunter eingerückt wie bei
/// `--erreichbar-als`; keine ihrer Zeilen ist länger als die längste übrige Zeile der Hilfe.
#[test]
fn die_hilfe_zeigt_die_protokollstufe_auf_eigener_zeile_wie_erreichbar_als() {
    let ende = aufrufen(&["--hilfe"]);
    let zeilen: Vec<&str> = ende.stdout.lines().collect();
    let einzug = |zeile: &str| zeile.len() - zeile.trim_start().len();
    let stelle = |option: &str| {
        zeilen
            .iter()
            .position(|zeile| zeile.trim_start() == option)
            .unwrap_or_else(|| panic!("keine eigene Zeile {option}: {}", ende.stdout))
    };

    let stufe = stelle("--protokoll-stufe <stufe>");
    let erreichbar = stelle("--erreichbar-als <name>");

    assert_eq!(ende.exit, Some(0));
    assert_eq!(einzug(zeilen[stufe]), einzug(zeilen[erreichbar]));
    assert_eq!(
        einzug(zeilen[stufe + 1]),
        einzug(zeilen[erreichbar + 1]),
        "{}",
        ende.stdout
    );
    let block: Vec<usize> = (stufe..zeilen.len())
        .take_while(|&nummer| {
            nummer == stufe || einzug(zeilen[nummer]) == einzug(zeilen[stufe + 1])
        })
        .collect();
    let laengste_uebrige = (0..zeilen.len())
        .filter(|nummer| !block.contains(nummer))
        .map(|nummer| zeilen[nummer].chars().count())
        .max()
        .unwrap_or(0);
    for nummer in block {
        assert!(
            zeilen[nummer].chars().count() <= laengste_uebrige,
            "{}",
            zeilen[nummer]
        );
    }
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
fn erreichbar_als_ohne_wert_endet_mit_exit_zwei_und_nennt_die_option() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let ordner = wurzel.path().display().to_string();

    let ende = aufrufen(&[&ordner, "--erreichbar-als"]);

    assert_eq!(ende.exit, Some(2));
    assert!(
        genau_eine_zeile(&ende.stderr).contains("--erreichbar-als"),
        "{}",
        ende.stderr
    );
    assert_eq!(ende.stdout, "");
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
