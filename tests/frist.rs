//! F0033 Frist über die API setzen und lesen (R00008, US-1 bis US-6) — über das echte Programm mit gestellter Uhr
//! (heute 28.10.2026). Beitritte per `curl`-gleicher API von `127.0.0.1` sind Agenten (`beta@127.0.0.1`).
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::frist::{
    MITTAG, beitreten, channel_in_liste, datei_anlegen, frist_setzen, nachrichten, starten_um,
    tag_anlegen,
};
use gemeinsam::{Absender, api_antwort};
use serde_json::{Value, json};
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

const EINSTELLUNGEN: &str = "angelegt = \"2026-10-27T09:00:00.000+01:00\"\nangelegt_von = \"alpha@127.0.0.2\"\nfarbe = \"blau\"\n";

fn einstellungen(ordner: &Path, channel: &str) -> Vec<u8> {
    fs::read(ordner.join(format!("channels/{channel}/einstellungen.toml"))).expect("Einstellungen")
}

fn als_tabelle(bytes: &[u8]) -> toml::Table {
    String::from_utf8(bytes.to_vec())
        .expect("UTF-8")
        .parse()
        .expect("TOML")
}

/// US-1, Kriterium „Setzen“ (die Zeile im Betriebsprotokoll prüft R00017 in `tests/frist_protokoll.rs`).
#[tokio::test]
async fn us1_frist_setzen_die_anderen_einstellungen_bleiben() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    datei_anlegen(
        wurzel.path(),
        "channels/allgemein/einstellungen.toml",
        EINSTELLUNGEN.as_bytes(),
    );
    let instanz = starten_um(wurzel.path(), MITTAG);
    let eins = Absender::von("127.0.0.1");
    let beta = beitreten(&eins, &instanz, "beta").await;
    let (vorher, _) = channel_in_liste(&eins, &instanz, &beta, "allgemein").await;
    assert_eq!(vorher["frist_tage"], 7);

    let gesetzt = frist_setzen(&eins, &instanz, &beta, "allgemein", json!({ "tage": 30 })).await;

    assert_eq!(gesetzt.status, 200, "{}", gesetzt.json);
    assert_eq!(gesetzt.json["channel"]["name"], "allgemein");
    assert_eq!(gesetzt.json["channel"]["frist_tage"], 30);
    assert_eq!(gesetzt.json["channel"]["angelegt_von"], "alpha@127.0.0.2");
    assert_eq!(
        gesetzt.json["entfallen"],
        json!({ "tage": 0, "nachrichten": 0 })
    );
    let tabelle = als_tabelle(&einstellungen(wurzel.path(), "allgemein"));
    assert_eq!(tabelle["frist_tage"].as_integer(), Some(30));
    assert_eq!(tabelle["farbe"].as_str(), Some("blau"));
    assert_eq!(
        tabelle["angelegt"].as_str(),
        Some("2026-10-27T09:00:00.000+01:00")
    );
    assert_eq!(tabelle["angelegt_von"].as_str(), Some("alpha@127.0.0.2"));
    let (danach, _) = channel_in_liste(&eins, &instanz, &beta, "allgemein").await;
    assert_eq!(danach["frist_tage"], 30);
}

/// Kriterium „Ohne Datei“ und US-2 / „Neustart“.
#[tokio::test]
async fn us2_ohne_datei_entsteht_sie_und_die_frist_ueberlebt_den_neustart() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    fs::create_dir_all(wurzel.path().join("channels/leer")).expect("Ordner");
    let eins = Absender::von("127.0.0.1");
    {
        let instanz = starten_um(wurzel.path(), MITTAG);
        let beta = beitreten(&eins, &instanz, "beta").await;
        let (vorher, _) = channel_in_liste(&eins, &instanz, &beta, "leer").await;
        assert_eq!(vorher["frist_tage"], 7);

        let gesetzt = frist_setzen(&eins, &instanz, &beta, "leer", json!({ "tage": 30 })).await;

        assert_eq!(gesetzt.status, 200, "{}", gesetzt.json);
        let tabelle = als_tabelle(&einstellungen(wurzel.path(), "leer"));
        assert_eq!(tabelle["frist_tage"].as_integer(), Some(30));
    }
    let neu = starten_um(wurzel.path(), MITTAG);
    let beta = beitreten(&eins, &neu, "beta").await;

    let (channel, _) = channel_in_liste(&eins, &neu, &beta, "leer").await;

    assert_eq!(channel["frist_tage"], 30);
}

/// Kriterium „Derselbe Wert“: 200 mit 0/0, Datei byte-gleich und nicht neu geschrieben (Inode bleibt).
#[tokio::test]
async fn derselbe_wert_schreibt_nicht() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let text = format!("{EINSTELLUNGEN}frist_tage = 30\n");
    datei_anlegen(
        wurzel.path(),
        "channels/allgemein/einstellungen.toml",
        text.as_bytes(),
    );
    let pfad = wurzel.path().join("channels/allgemein/einstellungen.toml");
    let inode = fs::metadata(&pfad).expect("Datei").ino();
    let instanz = starten_um(wurzel.path(), MITTAG);
    let eins = Absender::von("127.0.0.1");
    let beta = beitreten(&eins, &instanz, "beta").await;
    let (_, stand) = channel_in_liste(&eins, &instanz, &beta, "allgemein").await;

    let gesetzt = frist_setzen(&eins, &instanz, &beta, "allgemein", json!({ "tage": 30 })).await;

    assert_eq!(gesetzt.status, 200, "{}", gesetzt.json);
    assert_eq!(
        gesetzt.json["entfallen"],
        json!({ "tage": 0, "nachrichten": 0 })
    );
    assert_eq!(fs::read(&pfad).expect("Datei"), text.as_bytes());
    assert_eq!(fs::metadata(&pfad).expect("Datei").ino(), inode);
    assert_eq!(
        channel_in_liste(&eins, &instanz, &beta, "allgemein")
            .await
            .1,
        stand
    );
}

/// US-3, Kriterium „Wertprüfung als Spanne“ (Mutanten 0/2 und 3649/3651).
#[tokio::test]
async fn us3_falsche_werte_werden_abgewiesen_1_und_3650_gehen() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    datei_anlegen(
        wurzel.path(),
        "channels/allgemein/einstellungen.toml",
        EINSTELLUNGEN.as_bytes(),
    );
    let instanz = starten_um(wurzel.path(), MITTAG);
    let eins = Absender::von("127.0.0.1");
    let beta = beitreten(&eins, &instanz, "beta").await;
    let mit_sieben_komma_null: Value = serde_json::from_str(r#"{"tage":7.0}"#).expect("JSON");

    for koerper in [
        json!({ "tage": 0 }),
        json!({ "tage": 3651 }),
        json!({ "tage": -1 }),
        json!({ "tage": "7" }),
        mit_sieben_komma_null,
        json!({ "tage": 7.5 }),
        json!({ "tage": null }),
        json!({}),
    ] {
        let abgewiesen = frist_setzen(&eins, &instanz, &beta, "allgemein", koerper.clone()).await;

        assert_eq!(abgewiesen.status, 422, "{koerper} → {}", abgewiesen.json);
        assert_eq!(abgewiesen.json["regel"], "frist", "{koerper}");
        assert!(abgewiesen.json["fehler"].is_string(), "{koerper}");
        assert_eq!(
            einstellungen(wurzel.path(), "allgemein"),
            EINSTELLUNGEN.as_bytes()
        );
    }
    for tage in [1, 3650] {
        let gesetzt =
            frist_setzen(&eins, &instanz, &beta, "allgemein", json!({ "tage": tage })).await;
        assert_eq!(gesetzt.status, 200, "{tage} → {}", gesetzt.json);
        assert_eq!(gesetzt.json["channel"]["frist_tage"], tage);
    }
}

/// US-4, Kriterium „Grenzschutz und Fehler“: 401, 403, 415, 400, 404 — und der Channel vor dem Wert.
#[tokio::test]
async fn us4_grenzschutz_unbekannter_channel_und_keine_sitzung() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    datei_anlegen(
        wurzel.path(),
        "channels/allgemein/einstellungen.toml",
        EINSTELLUNGEN.as_bytes(),
    );
    let instanz = starten_um(wurzel.path(), MITTAG);
    let eins = Absender::von("127.0.0.1");
    let beta = beitreten(&eins, &instanz, "beta").await;
    let ziel = format!("{}api/channels/allgemein/frist", instanz.adresse);
    let put = || eins.anfrage(reqwest::Method::PUT, ziel.clone());

    let ohne_sitzung = api_antwort(put().json(&json!({ "tage": 30 }))).await;
    let fremd = api_antwort(
        put()
            .header("X-LocalchatA-Sitzung", &beta)
            .header("Origin", "http://fremd.example")
            .json(&json!({ "tage": 30 })),
    )
    .await;
    let kein_json = api_antwort(
        put()
            .header("X-LocalchatA-Sitzung", &beta)
            .header("Content-Type", "text/plain")
            .body(r#"{"tage":30}"#),
    )
    .await;
    let kaputt = api_antwort(
        put()
            .header("X-LocalchatA-Sitzung", &beta)
            .header("Content-Type", "application/json")
            .body("{"),
    )
    .await;

    assert_eq!(ohne_sitzung.status, 401, "{}", ohne_sitzung.json);
    assert_eq!(fremd.status, 403, "{}", fremd.json);
    assert_eq!(kein_json.status, 415, "{}", kein_json.json);
    assert_eq!(kaputt.status, 400, "{}", kaputt.json);
    for (name, koerper) in [
        ("fehlt", json!({ "tage": 30 })),
        ("Allgemein", json!({ "tage": 30 })),
        ("fehlt", json!({ "tage": 0 })),
    ] {
        let unbekannt = frist_setzen(&eins, &instanz, &beta, name, koerper).await;
        assert_eq!(unbekannt.status, 404, "{name} → {}", unbekannt.json);
        assert!(unbekannt.json["fehler"].is_string(), "{}", unbekannt.json);
    }
    assert_eq!(
        einstellungen(wurzel.path(), "allgemein"),
        EINSTELLUNGEN.as_bytes()
    );
}

/// US-5, Kriterium „Defekte Einstellungen“: `null` in der Liste, 409 mit Datei, Wert vor Defekt.
#[tokio::test]
async fn us5_kaputte_einstellungen_werden_nicht_ueberschrieben() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    datei_anlegen(
        wurzel.path(),
        "channels/kaputt/einstellungen.toml",
        b"frist_tage = \"sieben\"\n",
    );
    let mut instanz = starten_um(wurzel.path(), MITTAG);
    let eins = Absender::von("127.0.0.1");
    let beta = beitreten(&eins, &instanz, "beta").await;
    let (gesundheit_status, gesundheit) = gemeinsam::gesundheit(&instanz.adresse).await;
    assert_eq!(gesundheit_status, 200);
    assert_eq!(gesundheit["defekte_zeilen"], 1);
    let (kaputt, _) = channel_in_liste(&eins, &instanz, &beta, "kaputt").await;
    assert_eq!(kaputt["frist_tage"], Value::Null);

    let abgewiesen = frist_setzen(&eins, &instanz, &beta, "kaputt", json!({ "tage": 30 })).await;
    let wert_zuerst = frist_setzen(&eins, &instanz, &beta, "kaputt", json!({ "tage": 0 })).await;

    assert_eq!(abgewiesen.status, 409, "{}", abgewiesen.json);
    assert_eq!(abgewiesen.json["regel"], "einstellungen-defekt");
    assert_eq!(
        abgewiesen.json["datei"],
        "channels/kaputt/einstellungen.toml"
    );
    assert!(abgewiesen.json["fehler"].is_string());
    assert_eq!(wert_zuerst.status, 422, "{}", wert_zuerst.json);
    assert_eq!(
        einstellungen(wurzel.path(), "kaputt"),
        b"frist_tage = \"sieben\"\n"
    );
    instanz.signal_und_warten("TERM");
    let stderr = instanz.stderr_bis_zum_ende();
    assert!(
        stderr.contains("WARNING laden")
            && stderr.contains("datei=channels/kaputt/einstellungen.toml zeile=1"),
        "{stderr}"
    );
}

/// US-6, Kriterium „Laden mit der Frist des Channels“: 28.10. − 29 Tage = 29.09. (Mutanten Frist ± 1, immer 7).
#[tokio::test]
async fn us6_der_start_laedt_die_tage_der_eigenen_frist() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    datei_anlegen(
        wurzel.path(),
        "channels/allgemein/einstellungen.toml",
        b"farbe = \"blau\"\nfrist_tage = 30\n",
    );
    tag_anlegen(wurzel.path(), "allgemein", "2026-09-28", 3);
    tag_anlegen(wurzel.path(), "allgemein", "2026-09-29", 2);
    tag_anlegen(wurzel.path(), "allgemein", "2026-10-28", 1);

    let instanz = starten_um(wurzel.path(), MITTAG);

    assert_eq!(nachrichten(&instanz).await, 3);
}
