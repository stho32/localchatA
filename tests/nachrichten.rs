//! F0027 Verlauf über die API — über das echte Programm (R00007, US-1 bis US-5). Absender `127.0.0.2` und
//! `127.0.0.3` (Linux-Loopback); Beitritte per API ohne `art` sind Agenten. Uhr und Zeitzone wie in R00007.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::{Absender, ApiAntwort, Instanz, api_antwort, starten_mit};
use serde_json::{Value, json};
use std::fs;
use std::path::Path;

pub const JETZT: &str = "2026-10-28T15:00:00.000+01:00";

const ALLGEMEIN: &str = "@ 2026-10-28T14:00:00.000+01:00 alpha@127.0.0.2\n  eins\n\n@ 2026-10-28T14:01:00.000+01:00 alpha@127.0.0.2\n  zwei\n  drei\n\n@ 2026-10-28T14:02:00.000+01:00 alpha@127.0.0.2\n  vier\n\n";

fn datei(ordner: &Path, pfad: &str, inhalt: &str) {
    let ziel = ordner.join(pfad);
    fs::create_dir_all(ziel.parent().expect("Elternordner")).expect("Ordner");
    fs::write(ziel, inhalt).expect("Datei");
}

fn starten(ordner: &Path) -> Instanz {
    starten_mit(
        ordner,
        &["--port", "0", "--host", "127.0.0.1", "--kein-browser"],
        &[("LOCALCHATA_JETZT", JETZT), ("TZ", "Europe/Berlin")],
    )
}

/// `allgemein` mit drei Sätzen von `alpha` und der leere Channel `leer`; `beta` ist von `127.0.0.3` beigetreten.
async fn us1_gegeben(ordner: &Path) -> (Instanz, Absender, String) {
    datei(ordner, "channels/allgemein/2026-10-28.log", ALLGEMEIN);
    fs::create_dir_all(ordner.join("channels/leer")).expect("leer");
    let instanz = starten(ordner);
    let drei = Absender::von("127.0.0.3");
    let beitritt = drei
        .beitreten(&instanz.adresse, json!({ "name": "beta" }), None)
        .await;
    assert_eq!(beitritt.status, 201, "{}", beitritt.json);
    let token = beitritt.json["token"].as_str().expect("Token").to_string();
    (instanz, drei, token)
}

async fn lesen(
    absender: &Absender,
    instanz: &Instanz,
    token: Option<&str>,
    pfad: &str,
) -> ApiAntwort {
    let mut anfrage = absender.anfrage(reqwest::Method::GET, format!("{}{pfad}", instanz.adresse));
    if let Some(token) = token {
        anfrage = anfrage.header("X-LocalchatA-Sitzung", token);
    }
    api_antwort(anfrage).await
}

fn zeiten(antwort: &ApiAntwort) -> Vec<&str> {
    antwort.json["nachrichten"]
        .as_array()
        .expect("nachrichten")
        .iter()
        .map(|nachricht| nachricht["zeit"].as_str().expect("zeit"))
        .collect()
}

#[tokio::test]
async fn us1_die_neuesten_nachrichten_mit_zeit_absender_und_text_aelteste_zuerst() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let (instanz, beta, token) = us1_gegeben(wurzel.path()).await;

    let antwort = lesen(
        &beta,
        &instanz,
        Some(&token),
        "api/channels/allgemein/nachrichten",
    )
    .await;
    let leer = lesen(
        &beta,
        &instanz,
        Some(&token),
        "api/channels/leer/nachrichten",
    )
    .await;

    assert_eq!(antwort.status, 200, "{}", antwort.json);
    assert_eq!(
        antwort.json,
        json!({
            "channel": "allgemein",
            "nachrichten": [
                { "zeit": "2026-10-28T14:00:00.000+01:00", "von": "alpha@127.0.0.2", "art": null, "text": "eins", "anhaenge": [], "zustand": null, "geaendert": null, "antwort_auf": null },
                { "zeit": "2026-10-28T14:01:00.000+01:00", "von": "alpha@127.0.0.2", "art": null, "text": "zwei\ndrei", "anhaenge": [], "zustand": null, "geaendert": null, "antwort_auf": null },
                { "zeit": "2026-10-28T14:02:00.000+01:00", "von": "alpha@127.0.0.2", "art": null, "text": "vier", "anhaenge": [], "zustand": null, "geaendert": null, "antwort_auf": null },
            ],
            "aelter": false,
            "neuer": false,
        })
    );
    assert_eq!(leer.status, 200, "{}", leer.json);
    assert_eq!(
        leer.json,
        json!({ "channel": "leer", "nachrichten": [], "aelter": false, "neuer": false })
    );
}

#[tokio::test]
async fn us2_nach_einer_zeit_kommt_genau_was_danach_geschah() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let (instanz, beta, token) = us1_gegeben(wurzel.path()).await;

    let nach = lesen(
        &beta,
        &instanz,
        Some(&token),
        "api/channels/allgemein/nachrichten?nach=2026-10-28T14:00:00.000%2B01:00",
    )
    .await;
    let vor = lesen(
        &beta,
        &instanz,
        Some(&token),
        "api/channels/allgemein/nachrichten?vor=2026-10-28T14:02:00.000%2B01:00&n=1",
    )
    .await;

    assert_eq!(nach.status, 200, "{}", nach.json);
    assert_eq!(
        zeiten(&nach),
        vec![
            "2026-10-28T14:01:00.000+01:00",
            "2026-10-28T14:02:00.000+01:00"
        ]
    );
    assert_eq!(
        (nach.json["aelter"].clone(), nach.json["neuer"].clone()),
        (json!(true), json!(false))
    );
    assert_eq!(zeiten(&vor), vec!["2026-10-28T14:01:00.000+01:00"]);
    assert_eq!(
        (vor.json["aelter"].clone(), vor.json["neuer"].clone()),
        (json!(true), json!(true))
    );
}

#[tokio::test]
async fn us4_falsche_abfragen_werden_mit_ihrer_regel_abgewiesen_eins_und_zweihundert_gehen() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let (instanz, beta, token) = us1_gegeben(wurzel.path()).await;
    let pfad = "api/channels/allgemein/nachrichten?";

    for (abfrage, regel) in [
        ("n=0", "n"),
        ("n=201", "n"),
        ("n=abc", "n"),
        ("n=1.5", "n"),
        ("n=-1", "n"),
        ("vor=2026-10-28T14:00:00.000+01:00", "cursor"),
        (
            "vor=2026-10-28T14:00:00.000%2B01:00&nach=2026-10-28T14:00:00.000%2B01:00",
            "cursor",
        ),
        ("vor=gestern", "cursor"),
    ] {
        let antwort = lesen(&beta, &instanz, Some(&token), &format!("{pfad}{abfrage}")).await;
        assert_eq!(antwort.status, 422, "{abfrage}: {}", antwort.json);
        assert_eq!(antwort.json["regel"], regel, "{abfrage}");
        assert!(antwort.json["fehler"].is_string(), "{abfrage}");
    }
    let eins = lesen(&beta, &instanz, Some(&token), &format!("{pfad}n=1")).await;
    let zweihundert = lesen(&beta, &instanz, Some(&token), &format!("{pfad}n=200")).await;
    let kodiert = lesen(
        &beta,
        &instanz,
        Some(&token),
        &format!("{pfad}vor=2026-10-28T14:00:00.000%2B01:00"),
    )
    .await;
    assert_eq!(eins.status, 200, "{}", eins.json);
    assert_eq!(zeiten(&eins), vec!["2026-10-28T14:02:00.000+01:00"]);
    assert_eq!(zweihundert.status, 200, "{}", zweihundert.json);
    assert_eq!(zeiten(&zweihundert).len(), 3);
    assert_eq!(kodiert.status, 200, "{}", kodiert.json);
    assert_eq!(kodiert.json["nachrichten"], json!([]));
}

#[tokio::test]
async fn us5_unbekannter_oder_ungueltiger_channel_404_ohne_sitzung_401() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let (instanz, beta, token) = us1_gegeben(wurzel.path()).await;

    let fehlt = lesen(
        &beta,
        &instanz,
        Some(&token),
        "api/channels/fehlt/nachrichten",
    )
    .await;
    let gross = lesen(
        &beta,
        &instanz,
        Some(&token),
        "api/channels/Allgemein/nachrichten",
    )
    .await;
    let ohne = lesen(&beta, &instanz, None, "api/channels/allgemein/nachrichten").await;

    assert_eq!(fehlt.status, 404, "{}", fehlt.json);
    assert_eq!(fehlt.json["fehler"], "# fehlt gibt es nicht.");
    assert_eq!(gross.status, 404, "{}", gross.json);
    assert!(gross.json["fehler"].is_string());
    assert_eq!(ohne.status, 401, "{}", ohne.json);
    assert_eq!(ohne.json["nachrichten"], Value::Null);
}
