//! F0036 Frist live im Strom (R00008, US-13, US-14) — über das echte Programm; „kein Ereignis“ exakt über
//! `stand`, die Zustellung nur als Grenze Z1 (2 s nach Erhalt der 200).
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::Absender;
use gemeinsam::frist::{MITTAG, beitreten, datei_anlegen, frist_setzen, starten_um};
use gemeinsam::strom::{Empfangen, Strom};
use serde_json::{Value, json};
use std::time::Duration;
use tokio::time::Instant;

/// Z1 aus R00008.
const Z1: Duration = Duration::from_secs(2);

fn ist_art(art: &'static str) -> impl Fn(&Empfangen) -> bool {
    move |e| e.data["art"] == art
}

fn frist_im_stand(stand: &Value, name: &str) -> Value {
    stand["liste"]
        .as_array()
        .expect("Liste")
        .iter()
        .find(|channel| channel["name"] == name)
        .map(|channel| channel["frist_tage"].clone())
        .expect("Channel im Stand")
}

/// US-13, US-14, Kriterien „Ereignis“, „Kein Ereignis ohne Änderung“ und „Stand“.
#[tokio::test]
async fn us13_us14_die_neue_frist_kommt_im_strom_an_derselbe_wert_nicht() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    datei_anlegen(
        wurzel.path(),
        "channels/allgemein/einstellungen.toml",
        b"farbe = \"blau\"\n",
    );
    std::fs::create_dir_all(wurzel.path().join("channels/leer")).expect("Ordner");
    let instanz = starten_um(wurzel.path(), MITTAG);
    let zwei = Absender::von("127.0.0.2");
    let eins = Absender::von("127.0.0.1");
    let alpha = beitreten(&zwei, &instanz, "alpha").await;
    let beta = beitreten(&eins, &instanz, "beta").await;
    let mut strom = Strom::oeffnen(&zwei, &instanz.adresse, &alpha).await;
    let stand = strom
        .bis(Duration::from_secs(5), ist_art("channels"))
        .await
        .expect("Stand der Channels");
    let s = stand.data["stand"].as_u64().expect("stand");
    assert_eq!(frist_im_stand(&stand.data, "allgemein"), 7);

    let gesetzt = frist_setzen(&eins, &instanz, &beta, "allgemein", json!({ "tage": 30 })).await;
    let antwort_da = Instant::now();
    assert_eq!(gesetzt.status, 200, "{}", gesetzt.json);
    let ereignis = strom
        .bis(Duration::from_secs(10), ist_art("channel"))
        .await
        .expect("Ereignis channel");

    assert_eq!(ereignis.data["stand"], s + 1);
    assert_eq!(ereignis.data["channel"]["name"], "allgemein");
    assert_eq!(ereignis.data["channel"]["frist_tage"], 30);
    assert!(
        ereignis.ankunft.saturating_duration_since(antwort_da) <= Z1,
        "{:?}",
        ereignis.ankunft.saturating_duration_since(antwort_da)
    );

    let derselbe = frist_setzen(&eins, &instanz, &beta, "allgemein", json!({ "tage": 30 })).await;
    let abgewiesen = frist_setzen(&eins, &instanz, &beta, "allgemein", json!({ "tage": 0 })).await;
    let einunddreissig =
        frist_setzen(&eins, &instanz, &beta, "allgemein", json!({ "tage": 31 })).await;
    assert_eq!(
        (derselbe.status, abgewiesen.status, einunddreissig.status),
        (200, 422, 200)
    );
    let naechstes = strom
        .bis(Duration::from_secs(10), ist_art("channel"))
        .await
        .expect("nächstes Ereignis channel");

    assert_eq!(naechstes.data["stand"], s + 2);
    assert_eq!(naechstes.data["channel"]["frist_tage"], 31);
    let mut neu = Strom::oeffnen(&zwei, &instanz.adresse, &alpha).await;
    let neuer_stand = neu
        .bis(Duration::from_secs(5), ist_art("channels"))
        .await
        .expect("Stand");
    assert_eq!(frist_im_stand(&neuer_stand.data, "allgemein"), 31);
    assert_eq!(frist_im_stand(&neuer_stand.data, "leer"), 7);
}
