//! F0021 Channels live im Ereignisstrom — über das echte Programm (R00006, US-7, US-8 und Kriterien).
//! Zeitgrenzen sind die des Kriteriums (2 s ab Erhalt der 201), gemessen als Ankunft beim selben Beobachter.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::strom::{Empfangen, Strom, Thema};
use gemeinsam::{Absender, Instanz, wegwerf_starten};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::time::Duration;
use tokio::time::Instant;

/// Grenze aus R00006 F0021: spätestens 2 s nach Erhalt der 201.
const ZUSTELLGRENZE: Duration = Duration::from_secs(2);

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

/// Die Channel-Namen, die ein Strom kennt: sein Stand `channels` vereinigt mit seinen Wechseln `channel`.
fn bekannte_channels(strom: &Strom) -> BTreeSet<String> {
    let mut namen = BTreeSet::new();
    for ereignis in &strom.ereignisse {
        let data = &ereignis.data;
        let neue: Vec<&Value> = match data["art"].as_str() {
            Some("channels") => data["liste"]
                .as_array()
                .map(|l| l.iter().collect())
                .unwrap_or_default(),
            Some("channel") => vec![&data["channel"]],
            _ => Vec::new(),
        };
        namen.extend(
            neue.iter()
                .filter_map(|c| c["name"].as_str().map(str::to_string)),
        );
    }
    namen
}

#[tokio::test]
async fn us7_der_strom_nennt_nach_den_teilnehmern_die_channels() {
    let instanz = wegwerf_starten();
    let drei = Absender::von("127.0.0.3");
    let beta = beitreten(&drei, &instanz, "beta").await;

    let mut strom = Strom::oeffnen(&drei, &instanz.adresse, &beta).await;
    strom
        .bis(Duration::from_secs(5), ist_art("channels"))
        .await
        .expect("Stand der Channels");

    assert_eq!(strom.ereignisse[0].data["art"], "teilnehmer");
    assert_eq!(
        strom.ereignisse[1].data,
        json!({ "art": "channels", "stand": 0, "liste": [] })
    );
}

#[tokio::test]
async fn us8_ein_neuer_channel_erscheint_spaetestens_2_s_nach_der_201_im_strom() {
    let instanz = wegwerf_starten();
    let zwei = Absender::von("127.0.0.2");
    let drei = Absender::von("127.0.0.3");
    let beta = beitreten(&drei, &instanz, "beta").await;
    let mut strom = Strom::oeffnen(&drei, &instanz.adresse, &beta).await;
    strom
        .bis(Duration::from_secs(5), ist_art("channels"))
        .await
        .expect("Stand der Channels");
    let alpha = beitreten(&zwei, &instanz, "alpha").await;

    let angelegt = zwei
        .channel_anlegen(&instanz.adresse, &alpha, json!({ "name": "allgemein" }))
        .await;
    let erhalt_der_201 = Instant::now();
    let wechsel = strom
        .bis(Duration::from_secs(10), ist_art("channel"))
        .await
        .expect("Wechsel channel");

    assert_eq!(angelegt.status, 201, "{}", angelegt.json);
    assert_eq!(
        wechsel.data,
        json!({ "art": "channel", "stand": 1, "channel": angelegt.json["channel"] })
    );
    let dauer = wechsel.ankunft.saturating_duration_since(erhalt_der_201);
    eprintln!("messung us8_zustellung_ms={}", dauer.as_millis());
    assert!(dauer <= ZUSTELLGRENZE, "{dauer:?} nach der 201");
}

#[tokio::test]
async fn dreissig_anlagen_parallel_zu_zehn_strom_oeffnungen_jeder_strom_kennt_alle_einmal() {
    let instanz = wegwerf_starten();
    let zwei = Absender::von("127.0.0.2");
    let drei = Absender::von("127.0.0.3");
    let alpha = beitreten(&zwei, &instanz, "alpha").await;
    let beta = beitreten(&drei, &instanz, "beta").await;

    let anlagen = futures_util::future::join_all((0..30).map(|nummer| {
        zwei.channel_anlegen(
            &instanz.adresse,
            &alpha,
            json!({ "name": format!("c{nummer:02}") }),
        )
    }));
    let oeffnungen = futures_util::future::join_all(
        (0..10).map(|_| Strom::oeffnen(&drei, &instanz.adresse, &beta)),
    );
    let (antworten, mut stroeme) = tokio::join!(anlagen, oeffnungen);
    let letzte_201 = Instant::now();

    assert!(antworten.iter().all(|a| a.status == 201));
    let erwartet: BTreeSet<String> = (0..30).map(|nummer| format!("c{nummer:02}")).collect();
    for (nummer, strom) in stroeme.iter_mut().enumerate() {
        while bekannte_channels(strom) != erwartet && letzte_201.elapsed() < ZUSTELLGRENZE {
            strom.mitlesen(Duration::from_millis(25)).await;
        }
        assert_eq!(bekannte_channels(strom), erwartet, "Strom {nummer}");
        let staende = strom.staende(Thema::Channels);
        assert!(
            staende.windows(2).all(|paar| paar[0] < paar[1]),
            "Strom {nummer}: {staende:?}"
        );
    }
}
