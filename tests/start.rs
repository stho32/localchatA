//! F0001 Starten mit Adresse — über das echte Programm (US-1, US-3, Kriterium LAN).

mod gemeinsam;

use gemeinsam::{gesundheit, lan_adresse, lokal_starten, starten};
use serde_json::json;
use std::net::{TcpListener, TcpStream};

#[tokio::test]
async fn us1_neuer_ordner_wird_angelegt_und_die_startzeile_nennt_die_tatsaechliche_adresse() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let ordner = wurzel.path().join("lc-r1");
    assert!(!ordner.exists());

    let mut instanz = lokal_starten(&ordner);

    assert!(ordner.is_dir());
    let absolut = ordner.canonicalize().expect("absoluter Ordner");
    let port = instanz.port();
    assert_ne!(port, 0);
    assert_eq!(
        instanz.startzeile,
        format!(
            "localchatA läuft für {} unter http://127.0.0.1:{port}/",
            absolut.display()
        )
    );
    let (status, inhalt) = gesundheit(&instanz.adresse).await;
    assert_eq!(status, 200);
    assert_eq!(inhalt["status"], "bereit");
    assert_eq!(inhalt["ordner"], json!(absolut.display().to_string()));
    assert_eq!(inhalt["nachrichten"], 0);
    assert_eq!(inhalt["log_bytes"], 0);
    assert_eq!(inhalt["defekte_zeilen"], 0);

    assert!(instanz.signal_und_warten("TERM").success());
    assert_eq!(
        instanz.restliche_ausgabe(),
        "",
        "die Startzeile ist die einzige Zeile auf stdout"
    );
}

#[tokio::test]
async fn us3_sigterm_beendet_geordnet_mit_exit_null_und_gibt_den_port_frei() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let mut instanz = lokal_starten(wurzel.path());
    let port = instanz.port();
    assert_eq!(gesundheit(&instanz.adresse).await.0, 200);

    let ende = instanz.signal_und_warten("TERM");

    assert_eq!(ende.code(), Some(0));
    assert!(
        TcpStream::connect(("127.0.0.1", port)).is_err(),
        "Port {port} antwortet noch"
    );
    assert!(
        TcpListener::bind(("127.0.0.1", port)).is_ok(),
        "Port {port} nicht frei"
    );
}

#[tokio::test]
async fn us3_strg_c_beendet_geordnet_mit_exit_null() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let mut instanz = lokal_starten(wurzel.path());
    assert_eq!(gesundheit(&instanz.adresse).await.0, 200);

    let ende = instanz.signal_und_warten("INT");

    assert_eq!(ende.code(), Some(0));
}

#[tokio::test]
async fn ohne_host_lauscht_die_instanz_auf_allen_schnittstellen_und_ist_im_lan_erreichbar() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let mut instanz = starten(wurzel.path(), &["--port", "0", "--kein-browser"]);
    let port = instanz.port();
    assert!(
        instanz
            .startzeile
            .ends_with(&format!(" unter http://0.0.0.0:{port}/")),
        "{}",
        instanz.startzeile
    );

    let (status, inhalt) = gesundheit(&format!("http://{}:{port}/", lan_adresse())).await;

    assert_eq!(status, 200);
    assert_eq!(inhalt["status"], "bereit");
    instanz.signal_und_warten("TERM");
}
