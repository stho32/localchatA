//! F0023 Ordner von Hand und Plattenfehler — über das echte Programm (R00006, US-12 bis US-14 und Kriterien).
//! Ausnahmewege: nichts geht verloren, nichts wird überschrieben, der Betreiber erfährt es auf stderr.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::protokoll::ort;
use gemeinsam::strom::Strom;
use gemeinsam::{Absender, Instanz, gesundheit, starten_mit};
use serde_json::{Value, json};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::time::Duration;

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

/// Nur die `laden`-Zeilen — die `WARNING frist … Einstellungen defekt` daneben (R00017) zählt hier nicht.
fn laden_zeilen(zeilen: Vec<String>) -> Vec<String> {
    zeilen
        .into_iter()
        .filter(|zeile| zeile.contains(" laden "))
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
        zu_test.iter().map(|z| ort(z)).collect::<Vec<_>>(),
        ["channels/Test"]
    );
    assert!(
        zu_test[0].contains("WARNING laden Kein gültiger Channelname"),
        "{zu_test:?}"
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
        json!([{ "name": "kaputt", "angelegt": null, "angelegt_von": null, "beschreibung": null, "frist_tage": null }])
    );
    let zeilen = laden_zeilen(stderr_zeilen(instanz, "channels/kaputt/einstellungen.toml"));
    assert_eq!(zeilen.len(), 1, "{zeilen:?}");
    assert!(
        ort(&zeilen[0]) == "channels/kaputt/einstellungen.toml:1",
        "{zeilen:?}"
    );
    assert_eq!(
        fs::read(ordner.join("channels/kaputt/einstellungen.toml")).expect("Datei"),
        b"angelegt = \""
    );
}

#[tokio::test]
async fn r00007_eine_beschreibung_gegen_die_regel_macht_die_einstellungen_defekt_die_datei_bleibt()
{
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let ordner = wurzel.path();
    let inhalt = b"angelegt_von = \"alpha@127.0.0.2\"\nbeschreibung = \"a\\nb\"\n";
    datei(ordner, "channels/kaputt/einstellungen.toml", inhalt);
    let instanz = starten_mit(ordner, LOKAL, &[]);

    let namen = liste(&instanz).await;
    let (_, gesundheit) = gemeinsam::gesundheit(&instanz.adresse).await;

    assert_eq!(
        namen,
        json!([{ "name": "kaputt", "angelegt": null, "angelegt_von": null, "beschreibung": null, "frist_tage": null }])
    );
    assert_eq!(gesundheit["defekte_zeilen"], 1);
    let zeilen = laden_zeilen(stderr_zeilen(instanz, "channels/kaputt/einstellungen.toml"));
    assert_eq!(zeilen.len(), 1, "{zeilen:?}");
    assert!(
        ort(&zeilen[0]) == "channels/kaputt/einstellungen.toml:2",
        "{zeilen:?}"
    );
    assert_eq!(
        fs::read(ordner.join("channels/kaputt/einstellungen.toml")).expect("Datei"),
        inhalt
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

#[tokio::test]
async fn us12_ein_von_hand_angelegter_ordner_wird_nicht_ueberschrieben_und_beim_neustart_uebernommen()
 {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let ordner = wurzel.path();
    let instanz = starten_mit(ordner, LOKAL, &[]);
    let zwei = Absender::von("127.0.0.2");
    let alpha = beitreten(&zwei, &instanz, "alpha").await;
    datei(ordner, "channels/notizen/zettel.txt", b"hallo\n");

    let antwort = zwei
        .channel_anlegen(&instanz.adresse, &alpha, json!({ "name": "notizen" }))
        .await;
    let vorher = liste(&instanz).await;

    assert_eq!(antwort.status, 409, "{}", antwort.json);
    assert_eq!(antwort.json["regel"], "ordner-vorhanden");
    assert_eq!(
        antwort.json["fehler"],
        "channels/notizen besteht im Ordner schon; localchatA übernimmt ihn beim nächsten Start."
    );
    let inhalt: Vec<String> = fs::read_dir(ordner.join("channels/notizen"))
        .expect("Ordner")
        .map(|eintrag| {
            eintrag
                .expect("Eintrag")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    assert_eq!(inhalt, vec!["zettel.txt"]);
    assert_eq!(
        fs::read(ordner.join("channels/notizen/zettel.txt")).expect("Zettel"),
        b"hallo\n"
    );
    assert_eq!(vorher, json!([]));

    drop(instanz);
    let instanz = starten_mit(ordner, LOKAL, &[]);
    let nachher = liste(&instanz).await;

    assert_eq!(
        nachher,
        json!([{ "name": "notizen", "angelegt": null, "angelegt_von": null, "beschreibung": null, "frist_tage": 7 }])
    );
}

/// Nur als normaler Benutzer aussagekräftig: unter root wirkt `chmod a-w` nicht (R00006, Vorbereitung).
#[tokio::test]
async fn us13_ein_plattenfehler_hinterlaesst_keinen_halben_channel_und_kein_ereignis() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let ordner = wurzel.path();
    let instanz = starten_mit(ordner, LOKAL, &[]);
    let zwei = Absender::von("127.0.0.2");
    let drei = Absender::von("127.0.0.3");
    let alpha = beitreten(&zwei, &instanz, "alpha").await;
    let beta = beitreten(&drei, &instanz, "beta").await;
    let anlegen = |name: &'static str| {
        zwei.channel_anlegen(&instanz.adresse, &alpha, json!({ "name": name }))
    };
    assert_eq!(anlegen("allgemein").await.status, 201);
    let mut strom = Strom::oeffnen(&drei, &instanz.adresse, &beta).await;
    strom
        .bis(Duration::from_secs(5), |e| e.data["art"] == "channels")
        .await
        .expect("Stand der Channels");
    let channels = ordner.join("channels");
    fs::set_permissions(&channels, fs::Permissions::from_mode(0o555)).expect("chmod a-w");

    let gescheitert = anlegen("neu").await;
    let ereignis = strom
        .bis(Duration::from_millis(500), |e| e.data["art"] == "channel")
        .await;
    fs::set_permissions(&channels, fs::Permissions::from_mode(0o755)).expect("chmod u+w");
    let danach = anlegen("neu").await;

    assert_eq!(gescheitert.status, 500, "{}", gescheitert.json);
    assert!(
        gescheitert.json["fehler"].is_string(),
        "{}",
        gescheitert.json
    );
    assert!(ereignis.is_none(), "{ereignis:?}");
    assert_eq!(danach.status, 201, "{}", danach.json);
    let zeilen = stderr_zeilen(instanz, "channels/neu");
    assert_eq!(zeilen.len(), 1, "{zeilen:?}");
    assert!(zeilen[0].contains("Permission denied"), "{zeilen:?}");
}

#[tokio::test]
async fn us13_ohne_rechte_entsteht_kein_ordner() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let ordner = wurzel.path();
    fs::create_dir_all(ordner.join("channels")).expect("channels");
    let instanz = starten_mit(ordner, LOKAL, &[]);
    let zwei = Absender::von("127.0.0.2");
    let alpha = beitreten(&zwei, &instanz, "alpha").await;
    let channels = ordner.join("channels");
    fs::set_permissions(&channels, fs::Permissions::from_mode(0o555)).expect("chmod a-w");

    let gescheitert = zwei
        .channel_anlegen(&instanz.adresse, &alpha, json!({ "name": "neu" }))
        .await;
    let gibt_es = ordner.join("channels/neu").exists();
    fs::set_permissions(&channels, fs::Permissions::from_mode(0o755)).expect("chmod u+w");

    assert_eq!(gescheitert.status, 500, "{}", gescheitert.json);
    assert!(!gibt_es);
}
