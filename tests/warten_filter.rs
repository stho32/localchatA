//! F0166 Nacharbeit B1 (code-reviewer #17): der Filter `channel` der Route wirkt beim Warten und beim Nachholen
//! (R00049 US-5 bis US-7; ADR-0054 Regel 2). Channels `allgemein`, `technik`, `zufall`; `claude` wartet, `anna` sendet.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::senden::{beitreten, ordner, senden, starten};
use gemeinsam::{Absender, ApiAntwort, Instanz, api_antwort};
use serde_json::json;
use std::time::{Duration, Instant};

async fn gegeben() -> (tempfile::TempDir, Instanz, Absender, String, String) {
    let wurzel = ordner();
    std::fs::create_dir_all(wurzel.path().join("channels/zufall")).expect("Channel zufall");
    let instanz = starten(wurzel.path(), &[]);
    let absender = Absender::von("127.0.0.1");
    let anna = beitreten(&absender, &instanz.adresse, "anna", Some("mensch")).await;
    let claude = beitreten(&absender, &instanz.adresse, "claude", None).await;
    (wurzel, instanz, absender, anna, claude)
}

async fn gesendet(
    absender: &Absender,
    instanz: &Instanz,
    anna: &str,
    channel: &str,
    text: &str,
) -> String {
    let antwort = senden(absender, &instanz.adresse, anna, channel, text).await;
    assert_eq!(antwort.status, 201, "{}", antwort.json);
    antwort.json["nachricht"]["zeit"]
        .as_str()
        .expect("zeit")
        .to_string()
}

/// Öffnet `GET api/warten?<query>&seit=<seit>` im Hintergrund; liefert Antwort und Ankunftszeit.
fn aufruf(
    instanz: &Instanz,
    claude: &str,
    query: &str,
    seit: &str,
) -> tokio::task::JoinHandle<(ApiAntwort, Instant)> {
    let url = format!(
        "{}api/warten?{query}&seit={}",
        instanz.adresse,
        seit.replace('+', "%2B")
    );
    let claude = claude.to_string();
    tokio::spawn(async move {
        let antwort = api_antwort(
            Absender::von("127.0.0.1")
                .anfrage(reqwest::Method::GET, url)
                .header("X-LocalchatA-Sitzung", claude),
        )
        .await;
        (antwort, Instant::now())
    })
}

fn channels(antwort: &ApiAntwort) -> Vec<String> {
    antwort.json["nachrichten"]
        .as_array()
        .expect("nachrichten")
        .iter()
        .map(|eintrag| eintrag["channel"].as_str().expect("channel").to_string())
        .collect()
}

/// Eine Nachricht in einem fremden Channel weckt nicht: `bis=3&channel=technik` bleibt leer, `ende: "frist"` nach
/// 3,0 bis 4,0 s.
#[tokio::test]
async fn eine_nachricht_in_einem_anderen_channel_weckt_nicht() {
    let (_wurzel, instanz, absender, anna, claude) = gegeben().await;
    let t0 = gesendet(&absender, &instanz, &anna, "technik", "t0").await;
    let beginn = Instant::now();
    let offen = aufruf(&instanz, &claude, "bis=3&channel=technik", &t0);
    tokio::time::sleep(Duration::from_millis(300)).await;
    gesendet(&absender, &instanz, &anna, "allgemein", "fremd").await;

    let (antwort, angekommen) = offen.await.expect("Aufruf");

    assert_eq!(antwort.status, 200, "{}", antwort.json);
    assert_eq!(antwort.json["nachrichten"], json!([]), "{}", antwort.json);
    assert_eq!(antwort.json["ende"], "frist");
    let dauer = angekommen.duration_since(beginn);
    assert!(
        dauer >= Duration::from_millis(3000) && dauer < Duration::from_millis(4000),
        "{dauer:?}"
    );
}

/// Dieselbe Frage mit einer Nachricht im genannten Channel: genau diese, binnen 1 s nach der 201.
#[tokio::test]
async fn eine_nachricht_im_genannten_channel_weckt_binnen_einer_sekunde() {
    let (_wurzel, instanz, absender, anna, claude) = gegeben().await;
    let t0 = gesendet(&absender, &instanz, &anna, "technik", "t0").await;
    let offen = aufruf(&instanz, &claude, "bis=30&channel=technik", &t0);
    tokio::time::sleep(Duration::from_millis(300)).await;

    let t1 = gesendet(&absender, &instanz, &anna, "technik", "treffer").await;
    let nach_der_201 = Instant::now();
    let (antwort, angekommen) = offen.await.expect("Aufruf");

    assert!(angekommen.saturating_duration_since(nach_der_201) < Duration::from_secs(1));
    assert_eq!(channels(&antwort), ["technik"]);
    assert_eq!(antwort.json["nachrichten"][0]["nachricht"]["zeit"], t1);
}

/// Zwei `channel` sind eine Vereinigung: `zufall` weckt nicht, `allgemein` weckt.
#[tokio::test]
async fn zwei_channels_sind_eine_vereinigung() {
    let (_wurzel, instanz, absender, anna, claude) = gegeben().await;
    let t0 = gesendet(&absender, &instanz, &anna, "technik", "t0").await;
    let query = "bis=4&channel=technik&channel=allgemein";
    let offen = aufruf(&instanz, &claude, query, &t0);
    tokio::time::sleep(Duration::from_millis(300)).await;
    gesendet(&absender, &instanz, &anna, "zufall", "fremd").await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(!offen.is_finished(), "zufall weckt nicht");

    let t1 = gesendet(&absender, &instanz, &anna, "allgemein", "treffer").await;
    let (antwort, _) = offen.await.expect("Aufruf");

    assert_eq!(channels(&antwort), ["allgemein"]);
    assert_eq!(antwort.json["nachrichten"][0]["nachricht"]["zeit"], t1);
}

/// Nachholen: je eine Nachricht nach `t0` in allgemein, technik und zufall; `bis=0&channel=technik` liefert genau die
/// aus technik.
#[tokio::test]
async fn nachholen_liefert_nur_den_genannten_channel() {
    let (_wurzel, instanz, absender, anna, claude) = gegeben().await;
    let t0 = gesendet(&absender, &instanz, &anna, "technik", "t0").await;
    gesendet(&absender, &instanz, &anna, "allgemein", "a").await;
    let t2 = gesendet(&absender, &instanz, &anna, "technik", "t").await;
    gesendet(&absender, &instanz, &anna, "zufall", "z").await;

    let (antwort, _) = aufruf(&instanz, &claude, "bis=0&channel=technik", &t0)
        .await
        .expect("Aufruf");

    assert_eq!(channels(&antwort), ["technik"]);
    assert_eq!(antwort.json["nachrichten"][0]["nachricht"]["zeit"], t2);
    assert_eq!(antwort.json["ende"], "treffer");
}
