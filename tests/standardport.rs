//! Standard-Port 7430 mit Ausweichen über das echte Programm (F0001, US-2).
//! Die Tests belegen und befreien die Ports selbst; ein Mutex hält sie nacheinander.

mod gemeinsam;

use gemeinsam::{gesundheit, starten};
use std::net::TcpListener;
use std::sync::Mutex;

static STANDARDPORTS: Mutex<()> = Mutex::new(());

fn belegen(port: u16) -> TcpListener {
    TcpListener::bind(("0.0.0.0", port))
        .unwrap_or_else(|fehler| panic!("Vorbedingung: Port {port} muss frei sein ({fehler})"))
}

fn ist_frei(port: u16) {
    drop(belegen(port));
}

#[tokio::test]
async fn ohne_port_und_mit_freiem_7430_lauscht_die_instanz_auf_0_0_0_0_7430() {
    let _nacheinander = STANDARDPORTS
        .lock()
        .unwrap_or_else(|vergiftet| vergiftet.into_inner());
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    ist_frei(7430);

    let instanz = starten(wurzel.path(), &["--kein-browser"]);

    assert!(
        instanz.startzeile.ends_with(" unter http://0.0.0.0:7430/"),
        "{}",
        instanz.startzeile
    );
    assert_eq!(gesundheit("http://127.0.0.1:7430/").await.0, 200);
}

#[tokio::test]
async fn us2_ohne_port_und_mit_belegten_7430_und_7431_nimmt_die_instanz_7432() {
    let _nacheinander = STANDARDPORTS
        .lock()
        .unwrap_or_else(|vergiftet| vergiftet.into_inner());
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let _erster = belegen(7430);
    let _zweiter = belegen(7431);
    ist_frei(7432);

    let instanz = starten(wurzel.path(), &["--kein-browser"]);

    assert!(
        instanz.startzeile.ends_with(" unter http://0.0.0.0:7432/"),
        "{}",
        instanz.startzeile
    );
    assert_eq!(gesundheit("http://127.0.0.1:7432/").await.0, 200);
}
