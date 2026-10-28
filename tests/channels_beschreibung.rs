//! F0026 Beschreibung über die API (R00007, US-21, US-22): über das echte Programm. Absender `127.0.0.2` und
//! `127.0.0.3` (Linux-Loopback); Beitritte per API ohne `art` sind Agenten.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::strom::{Empfangen, Strom};
use gemeinsam::{Absender, Instanz, lokal_starten};
use serde_json::{Value, json};
use std::fs;
use std::time::Duration;

const SATZ: &str = "Absprachen zwischen Security- und Entwicklungs-Factory";

async fn beitreten(absender: &Absender, instanz: &Instanz, name: &str) -> String {
    let antwort = absender
        .beitreten(&instanz.adresse, json!({ "name": name }), None)
        .await;
    assert_eq!(antwort.status, 201, "{}", antwort.json);
    antwort.json["token"].as_str().expect("Token").to_string()
}

fn ist_art(art: &'static str) -> impl Fn(&Empfangen) -> bool {
    move |e| e.data["art"] == art
}

fn beschreibung_in_liste(liste: &Value, name: &str) -> Value {
    liste
        .as_array()
        .expect("Liste")
        .iter()
        .find(|channel| channel["name"] == name)
        .map(|channel| channel["beschreibung"].clone())
        .expect("Channel in der Liste")
}

#[tokio::test]
async fn us21_die_beschreibung_steht_in_antwort_datei_liste_stand_ereignis_und_nach_dem_neustart() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let instanz = lokal_starten(wurzel.path());
    let zwei = Absender::von("127.0.0.2");
    let drei = Absender::von("127.0.0.3");
    let alpha = beitreten(&zwei, &instanz, "alpha").await;
    let beta = beitreten(&drei, &instanz, "beta").await;
    let mut laufend = Strom::oeffnen(&drei, &instanz.adresse, &beta).await;
    laufend
        .bis(Duration::from_secs(5), ist_art("channels"))
        .await
        .expect("Stand der Channels");

    let angelegt = zwei
        .channel_anlegen(
            &instanz.adresse,
            &alpha,
            json!({ "name": "security-entwicklung", "beschreibung": SATZ }),
        )
        .await;

    assert_eq!(angelegt.status, 201, "{}", angelegt.json);
    assert_eq!(angelegt.json["channel"]["beschreibung"], SATZ);
    let datei: toml::Table = fs::read_to_string(
        wurzel
            .path()
            .join("channels/security-entwicklung/einstellungen.toml"),
    )
    .expect("einstellungen.toml")
    .parse()
    .expect("TOML");
    assert_eq!(datei["beschreibung"].as_str(), Some(SATZ));
    let liste = zwei.channels(&instanz.adresse, Some(&alpha)).await;
    assert_eq!(
        beschreibung_in_liste(&liste.json["liste"], "security-entwicklung"),
        SATZ
    );
    let ereignis = laufend
        .bis(Duration::from_secs(5), ist_art("channel"))
        .await
        .expect("Ereignis channel");
    assert_eq!(ereignis.data["channel"]["beschreibung"], SATZ);
    let mut neu = Strom::oeffnen(&drei, &instanz.adresse, &beta).await;
    let stand = neu
        .bis(Duration::from_secs(5), ist_art("channels"))
        .await
        .expect("Stand der Channels");
    assert_eq!(
        beschreibung_in_liste(&stand.data["liste"], "security-entwicklung"),
        SATZ
    );
    drop(laufend);
    drop(neu);
    drop(instanz);

    let neugestartet = lokal_starten(wurzel.path());
    let gamma = beitreten(&drei, &neugestartet, "gamma").await;
    let liste = drei.channels(&neugestartet.adresse, Some(&gamma)).await;
    assert_eq!(
        beschreibung_in_liste(&liste.json["liste"], "security-entwicklung"),
        SATZ
    );
}

#[tokio::test]
async fn us22_zu_lang_oder_mit_umbruch_wird_abgewiesen_ohne_ordner_zweihundert_zeichen_gehen() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let instanz = lokal_starten(wurzel.path());
    let zwei = Absender::von("127.0.0.2");
    let alpha = beitreten(&zwei, &instanz, "alpha").await;
    let anlegen = |name: &'static str, beschreibung: Value| {
        zwei.channel_anlegen(
            &instanz.adresse,
            &alpha,
            json!({ "name": name, "beschreibung": beschreibung }),
        )
    };

    let zu_lang = anlegen("lang", json!("ä".repeat(201))).await;
    let umbruch = anlegen("umbruch", json!("a\nb")).await;
    let tab = anlegen("tab", json!("a\tb")).await;
    let genau = anlegen("genau", json!("ä".repeat(200))).await;

    for abgewiesen in [&zu_lang, &umbruch, &tab] {
        assert_eq!(abgewiesen.status, 422, "{}", abgewiesen.json);
        assert_eq!(abgewiesen.json["regel"], "beschreibung");
    }
    assert_eq!(genau.status, 201, "{}", genau.json);
    assert_eq!(genau.json["channel"]["beschreibung"], "ä".repeat(200));
    let mut ordner: Vec<String> = fs::read_dir(wurzel.path().join("channels"))
        .expect("channels")
        .map(|eintrag| {
            eintrag
                .expect("Eintrag")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    ordner.sort();
    assert_eq!(ordner, vec!["genau"]);
}
