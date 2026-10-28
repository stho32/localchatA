//! F0023 Ordner von Hand und Plattenfehler — über das echte Programm (R00006, US-12 bis US-14 und Kriterien).
//! Ausnahmewege: nichts geht verloren, nichts wird überschrieben, der Betreiber erfährt es auf stderr.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::{Absender, Instanz, gesundheit, starten_mit};
use serde_json::{Value, json};
use std::fs;
use std::path::Path;

const LOKAL: &[&str] = &["--port", "0", "--host", "127.0.0.1", "--kein-browser"];

async fn beitreten(absender: &Absender, instanz: &Instanz, name: &str) -> String {
    let antwort = absender
        .beitreten(&instanz.adresse, json!({ "name": name }), None)
        .await;
    assert_eq!(antwort.status, 201, "{}", antwort.json);
    antwort.json["token"].as_str().expect("Token").to_string()
}

async fn liste(instanz: &Instanz) -> Value {
    let eins = Absender::von("127.0.0.1");
    let token = beitreten(&eins, instanz, "leser").await;
    let antwort = eins.channels(&instanz.adresse, Some(&token)).await;
    assert_eq!(antwort.status, 200, "{}", antwort.json);
    antwort.json["liste"].clone()
}

fn datei(ordner: &Path, pfad: &str, inhalt: &[u8]) {
    let ziel = ordner.join(pfad);
    fs::create_dir_all(ziel.parent().expect("Elternordner")).expect("Ordner");
    fs::write(ziel, inhalt).expect("Datei");
}

/// Zeilen auf stderr, die `teil` enthalten — gelesen bis zum Ende des Programms.
fn stderr_zeilen(mut instanz: Instanz, teil: &str) -> Vec<String> {
    instanz.signal_und_warten("TERM");
    instanz
        .stderr_bis_zum_ende()
        .lines()
        .filter(|zeile| zeile.contains(teil))
        .map(str::to_string)
        .collect()
}

#[tokio::test]
async fn us14_ordner_mit_ungueltigem_namen_werden_beim_start_uebergangen_und_bleiben() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let ordner = wurzel.path();
    datei(
        ordner,
        "channels/allgemein/einstellungen.toml",
        b"angelegt = \"2026-10-28T10:15:00.123+01:00\"\nangelegt_von = \"alpha@127.0.0.2\"\n",
    );
    datei(ordner, "channels/Test/zettel.txt", b"von Hand");
    datei(ordner, "channels/.neu-x/zettel.txt", b"Arbeitsordner");
    let instanz = starten_mit(ordner, LOKAL, &[]);

    let namen = liste(&instanz).await;
    let gesundheit = gesundheit(&instanz.adresse).await.1;

    assert_eq!(namen.as_array().map(Vec::len), Some(1), "{namen}");
    assert_eq!(namen[0]["name"], "allgemein");
    assert_eq!(namen[0]["angelegt_von"], "alpha@127.0.0.2");
    assert_eq!(gesundheit["defekte_zeilen"], 1);
    let zu_test = stderr_zeilen(instanz, "channels/");
    assert_eq!(
        zu_test,
        vec!["channels/Test: kein gültiger Channelname, übergangen"]
    );
    assert_eq!(
        fs::read(ordner.join("channels/Test/zettel.txt")).expect("Test"),
        b"von Hand"
    );
    assert_eq!(
        fs::read(ordner.join("channels/.neu-x/zettel.txt")).expect(".neu-x"),
        b"Arbeitsordner"
    );
}

#[tokio::test]
async fn us14_eine_defekte_einstellungsdatei_wird_als_unbekannt_gelesen_und_nie_geaendert() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let ordner = wurzel.path();
    datei(
        ordner,
        "channels/kaputt/einstellungen.toml",
        b"angelegt = \"",
    );
    let instanz = starten_mit(ordner, LOKAL, &[]);

    let namen = liste(&instanz).await;

    assert_eq!(
        namen,
        json!([{ "name": "kaputt", "angelegt": null, "angelegt_von": null, "beschreibung": null }])
    );
    let zeilen = stderr_zeilen(instanz, "channels/kaputt/einstellungen.toml");
    assert_eq!(zeilen.len(), 1, "{zeilen:?}");
    assert!(
        zeilen[0].starts_with("channels/kaputt/einstellungen.toml:1: "),
        "{zeilen:?}"
    );
    assert_eq!(
        fs::read(ordner.join("channels/kaputt/einstellungen.toml")).expect("Datei"),
        b"angelegt = \""
    );
}

#[tokio::test]
async fn us14_ungueltiger_ordner_und_defekte_datei_zaehlen_zusammen_zwei_defekte() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let ordner = wurzel.path();
    datei(ordner, "channels/Test/zettel.txt", b"von Hand");
    datei(ordner, "channels/.neu-x/zettel.txt", b"Arbeitsordner");
    datei(
        ordner,
        "channels/kaputt/einstellungen.toml",
        b"angelegt = \"",
    );
    let instanz = starten_mit(ordner, LOKAL, &[]);

    let namen = liste(&instanz).await;

    assert_eq!(gesundheit(&instanz.adresse).await.1["defekte_zeilen"], 2);
    assert_eq!(namen.as_array().map(Vec::len), Some(1), "{namen}");
    assert_eq!(namen[0]["name"], "kaputt");
}
