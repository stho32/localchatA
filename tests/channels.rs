//! F0020 Channel über die API anlegen und listen — über das echte Programm (R00006, US-1 bis US-6).
//! Absender `127.0.0.1` … `127.0.0.3` (Linux-Loopback); Beitritte per API ohne `art` sind Agenten.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::{Absender, ApiAntwort, Instanz, api_antwort, gesundheit, lokal_starten};
use serde_json::{Value, json};
use std::fs;
use std::path::Path;

async fn beitreten(absender: &Absender, instanz: &Instanz, name: &str) -> String {
    let antwort = absender
        .beitreten(&instanz.adresse, json!({ "name": name }), None)
        .await;
    assert_eq!(antwort.status, 201, "{}", antwort.json);
    antwort.json["token"].as_str().expect("Token").to_string()
}

async fn anlegen(absender: &Absender, instanz: &Instanz, token: &str, name: &str) -> ApiAntwort {
    absender
        .channel_anlegen(&instanz.adresse, token, json!({ "name": name }))
        .await
}

/// Die Unterordner von `channels/`, sortiert; fehlt `channels/`, keine.
fn channel_ordner(ordner: &Path) -> Vec<String> {
    let Ok(eintraege) = fs::read_dir(ordner.join("channels")) else {
        return Vec::new();
    };
    let mut namen: Vec<String> = eintraege
        .map(|eintrag| {
            eintrag
                .expect("Eintrag")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    namen.sort();
    namen
}

/// RFC 3339 mit genau drei Nachkommastellen und Versatz, z. B. `2026-10-28T10:15:00.123+01:00`.
fn mit_millisekunden(zeit: &Value) -> bool {
    zeit.as_str().is_some_and(|text| {
        chrono::DateTime::parse_from_rfc3339(text).is_ok()
            && text.len() == 29
            && text.as_bytes()[19] == b'.'
    })
}

#[tokio::test]
async fn us1_ein_agent_legt_einen_channel_mit_ordner_und_einstellungsdatei_an() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let instanz = lokal_starten(wurzel.path());
    let zwei = Absender::von("127.0.0.2");
    let alpha = beitreten(&zwei, &instanz, "alpha").await;

    let antwort = anlegen(&zwei, &instanz, &alpha, "allgemein").await;

    assert_eq!(antwort.status, 201, "{}", antwort.json);
    let channel = &antwort.json["channel"];
    assert_eq!(channel["name"], "allgemein");
    assert_eq!(channel["angelegt_von"], "alpha@127.0.0.2");
    assert_eq!(channel["beschreibung"], Value::Null);
    assert!(mit_millisekunden(&channel["angelegt"]), "{channel}");
    assert_eq!(channel.as_object().expect("Objekt").len(), 4, "{channel}");
    let ordner = instanz.ordner.join("channels/allgemein");
    assert!(!ordner.join("einstellungen.toml.neu").exists());
    let datei: toml::Table = fs::read_to_string(ordner.join("einstellungen.toml"))
        .expect("einstellungen.toml")
        .parse()
        .expect("TOML");
    assert_eq!(datei["angelegt"].as_str(), channel["angelegt"].as_str());
    assert_eq!(datei["angelegt_von"].as_str(), Some("alpha@127.0.0.2"));
    assert!(!datei.contains_key("name"), "{datei}");
}

#[tokio::test]
async fn us3_ungueltige_namen_werden_mit_vorschlag_abgewiesen_und_legen_keinen_ordner_an() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let instanz = lokal_starten(wurzel.path());
    let zwei = Absender::von("127.0.0.2");
    let alpha = beitreten(&zwei, &instanz, "alpha").await;

    let gross = anlegen(&zwei, &instanz, &alpha, "Allgemein").await;
    let akzent = anlegen(&zwei, &instanz, &alpha, "José").await;
    let reserviert = anlegen(&zwei, &instanz, &alpha, "con").await;
    let ohne_namen = zwei
        .channel_anlegen(&instanz.adresse, &alpha, json!({ "name": 5 }))
        .await;

    for antwort in [&gross, &akzent, &reserviert, &ohne_namen] {
        assert_eq!(antwort.status, 422, "{}", antwort.json);
        assert!(antwort.json["fehler"].is_string(), "{}", antwort.json);
        assert!(antwort.json["regel"].is_string(), "{}", antwort.json);
    }
    assert_eq!(gross.json["vorschlag"], "allgemein");
    assert_eq!(akzent.json["vorschlag"], "jose");
    assert_eq!(reserviert.json["regel"], "reserviert");
    assert!(
        reserviert.json.get("vorschlag").is_none(),
        "{}",
        reserviert.json
    );
    assert!(channel_ordner(&instanz.ordner).is_empty());
}

#[tokio::test]
async fn us4_einen_vorhandenen_channel_gibt_es_nur_einmal_die_datei_bleibt_byte_gleich() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let instanz = lokal_starten(wurzel.path());
    let zwei = Absender::von("127.0.0.2");
    let drei = Absender::von("127.0.0.3");
    let alpha = beitreten(&zwei, &instanz, "alpha").await;
    let beta = beitreten(&drei, &instanz, "beta").await;
    let erste = anlegen(&zwei, &instanz, &alpha, "allgemein").await;
    let datei = instanz.ordner.join("channels/allgemein/einstellungen.toml");
    let vorher = fs::read(&datei).expect("Datei");

    let zweite = anlegen(&drei, &instanz, &beta, "allgemein").await;

    assert_eq!(zweite.status, 409, "{}", zweite.json);
    assert!(zweite.json["fehler"].is_string());
    assert_eq!(zweite.json["channel"], erste.json["channel"]);
    assert_eq!(zweite.json["channel"]["angelegt_von"], "alpha@127.0.0.2");
    assert_eq!(fs::read(&datei).expect("Datei"), vorher);
}

#[tokio::test]
async fn us5_zwanzig_gleichzeitige_anlagen_ergeben_genau_einen_channel() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let instanz = lokal_starten(wurzel.path());
    let zwei = Absender::von("127.0.0.2");
    let alpha = beitreten(&zwei, &instanz, "alpha").await;

    let antworten =
        futures_util::future::join_all((0..20).map(|_| anlegen(&zwei, &instanz, &alpha, "sync")))
            .await;

    let angelegt: Vec<&ApiAntwort> = antworten.iter().filter(|a| a.status == 201).collect();
    let vorhanden: Vec<&ApiAntwort> = antworten.iter().filter(|a| a.status == 409).collect();
    assert_eq!((angelegt.len(), vorhanden.len()), (1, 19));
    let zeit = &angelegt[0].json["channel"]["angelegt"];
    assert!(
        vorhanden
            .iter()
            .all(|a| &a.json["channel"]["angelegt"] == zeit)
    );
    assert_eq!(channel_ordner(&instanz.ordner), vec!["sync"]);
}

#[tokio::test]
async fn us6_eine_beschreibung_wird_abgewiesen_statt_still_verworfen_null_und_leer_gehen() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let instanz = lokal_starten(wurzel.path());
    let zwei = Absender::von("127.0.0.2");
    let alpha = beitreten(&zwei, &instanz, "alpha").await;
    let mit =
        |name: &str, beschreibung: Value| json!({ "name": name, "beschreibung": beschreibung });

    let gesetzt = zwei
        .channel_anlegen(&instanz.adresse, &alpha, mit("x1", json!("Absprachen")))
        .await;
    let zahl = zwei
        .channel_anlegen(&instanz.adresse, &alpha, mit("x4", json!(5)))
        .await;
    let null = zwei
        .channel_anlegen(&instanz.adresse, &alpha, mit("x2", Value::Null))
        .await;
    let leer = zwei
        .channel_anlegen(&instanz.adresse, &alpha, mit("x3", json!("")))
        .await;

    for abgewiesen in [&gesetzt, &zahl] {
        assert_eq!(abgewiesen.status, 422, "{}", abgewiesen.json);
        assert_eq!(abgewiesen.json["regel"], "beschreibung");
        assert!(abgewiesen.json["fehler"].is_string());
    }
    for angelegt in [&null, &leer] {
        assert_eq!(angelegt.status, 201, "{}", angelegt.json);
        assert_eq!(angelegt.json["channel"]["beschreibung"], Value::Null);
    }
    assert_eq!(channel_ordner(&instanz.ordner), vec!["x2", "x3"]);
}

#[tokio::test]
async fn us6_ohne_sitzung_fremder_ursprung_ohne_json_und_zu_gross_legen_nichts_an() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let instanz = lokal_starten(wurzel.path());
    let zwei = Absender::von("127.0.0.2");
    let alpha = beitreten(&zwei, &instanz, "alpha").await;
    let url = format!("{}api/channels", instanz.adresse);
    let post = || zwei.anfrage(reqwest::Method::POST, url.clone());
    let koerper = json!({ "name": "allgemein" });

    let ohne_sitzung = api_antwort(post().json(&koerper)).await;
    let fremd = api_antwort(
        post()
            .header("X-LocalchatA-Sitzung", &alpha)
            .header("Origin", "http://evil.example")
            .json(&koerper),
    )
    .await;
    let ohne_json = api_antwort(
        post()
            .header("X-LocalchatA-Sitzung", &alpha)
            .body(koerper.to_string()),
    )
    .await;
    let zu_gross = api_antwort(
        post()
            .header("X-LocalchatA-Sitzung", &alpha)
            .header("Content-Type", "application/json")
            .body(format!(
                "{{\"name\":\"allgemein\",\"x\":\"{}\"}}",
                "x".repeat(2 * 1024 * 1024)
            )),
    )
    .await;

    assert_eq!(
        [
            ohne_sitzung.status,
            fremd.status,
            ohne_json.status,
            zu_gross.status
        ],
        [401, 403, 415, 413]
    );
    for antwort in [&ohne_sitzung, &fremd, &ohne_json, &zu_gross] {
        assert!(antwort.json["fehler"].is_string(), "{}", antwort.json);
    }
    assert!(channel_ordner(&instanz.ordner).is_empty());
}

#[tokio::test]
async fn us2_die_liste_steht_nach_name_und_uebersteht_einen_neustart() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let zwei = Absender::von("127.0.0.2");
    let drei = Absender::von("127.0.0.3");
    let instanz = lokal_starten(wurzel.path());
    let alpha = beitreten(&zwei, &instanz, "alpha").await;
    let mut angelegt = Vec::new();
    for name in ["security-entwicklung", "allgemein"] {
        let antwort = anlegen(&zwei, &instanz, &alpha, name).await;
        assert_eq!(antwort.status, 201, "{}", antwort.json);
        angelegt.push(antwort.json["channel"].clone());
    }
    let beta = beitreten(&drei, &instanz, "beta").await;

    let vorher = drei.channels(&instanz.adresse, Some(&beta)).await;
    let ohne = drei.channels(&instanz.adresse, None).await;

    assert_eq!(vorher.status, 200, "{}", vorher.json);
    assert_eq!(vorher.json["stand"], 2);
    assert_eq!(
        vorher.json["liste"],
        json!([angelegt[1].clone(), angelegt[0].clone()])
    );
    assert_eq!(ohne.status, 401, "{}", ohne.json);
    assert!(ohne.json["fehler"].is_string());

    drop(instanz);
    let instanz = lokal_starten(wurzel.path());
    let beta = beitreten(&drei, &instanz, "beta").await;
    let nachher = drei.channels(&instanz.adresse, Some(&beta)).await;

    assert_eq!(nachher.status, 200, "{}", nachher.json);
    assert_eq!(nachher.json["stand"], 0);
    assert_eq!(nachher.json["liste"], vorher.json["liste"]);
}

#[tokio::test]
async fn ein_ordner_ohne_einstellungsdatei_steht_mit_unbekannten_werten_in_der_liste() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    fs::create_dir_all(wurzel.path().join("channels/notizen")).expect("Ordner von Hand");
    let instanz = lokal_starten(wurzel.path());
    let eins = Absender::von("127.0.0.1");
    let token = beitreten(&eins, &instanz, "shoff").await;

    let liste = eins.channels(&instanz.adresse, Some(&token)).await;

    assert_eq!(
        liste.json,
        json!({ "stand": 0, "liste": [
            { "name": "notizen", "angelegt": null, "angelegt_von": null, "beschreibung": null }
        ] })
    );
    assert_eq!(gesundheit(&instanz.adresse).await.1["defekte_zeilen"], 0);
}
