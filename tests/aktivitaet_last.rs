//! B0698 Lastprobe P11 (R00045 I0041 „Last“, US-12; ADR-0051 Lastnachweis; Lastprofil Team-LAN): 20 Menschen mit
//! Strom setzen `tippt` alle 3 s, 50 Agenten ohne Strom setzen `denkt_nach` und erneuern alle 20 s, 30 Nachrichten/s
//! über 60 s. Gemessen wie Z10 in R00011: von der 201 im Testprozess bis zum Ereignis `nachricht` beim Strom eines
//! Menschen (zügiger Leser). Davor 3 s Leerlauf mit derselben Messung; Leerlauf-p90 über 0,05 s macht den Lauf ungültig
//! (wiederholen, nicht rot).
//!
//! Gate von I0041, nicht im Standardlauf (70 Teilnehmer, 60 s):
//! `cargo test --release --test aktivitaet_last -- --ignored --nocapture` (im Debug-Build ohne `--release` ebenso fahrbar).
//! Die Grenze ist die der Anforderung (Zustellung unter 1 s, Maximum), nie schärfer.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::protokoll::zeilen;
use gemeinsam::senden::{HEUTE, beitreten, ordner, senden, starten};
use gemeinsam::strom::Strom;
use gemeinsam::{Absender, api_antwort};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::Instant;

const MENSCHEN: usize = 20;
const AGENTEN: usize = 50;
const NACHRICHTEN_JE_S: u64 = 30;
const DAUER: Duration = Duration::from_secs(60);
const TIPPT_ALLE: Duration = Duration::from_secs(3);
const ERNEUERN_ALLE: Duration = Duration::from_secs(20);
const LEERLAUF: Duration = Duration::from_secs(3);
/// R00045 „Last“: Zustellung jeder Nachricht unter 1 s.
const GRENZE: Duration = Duration::from_secs(1);
/// R00010/R00011: darüber ist der Rechner zu ausgelastet, der Lauf ungültig.
const LEERLAUF_HOECHSTENS: Duration = Duration::from_millis(50);

/// Agenten verteilt auf 10 Adressen (je 5; 3 Nachrichten/s je Adresse, unter 8/s des Schreibkontos), Menschen auf 5
/// Adressen (je 4 Ströme).
fn agent_adresse(n: usize) -> String {
    format!("127.0.1.{}", n % 10 + 1)
}

fn mensch_adresse(n: usize) -> String {
    format!("127.0.2.{}", n % 5 + 1)
}

async fn setzen(absender: &Absender, basis: &str, token: &str, koerper: serde_json::Value) -> u16 {
    api_antwort(
        absender
            .anfrage(
                reqwest::Method::PUT,
                format!("{basis}api/channels/allgemein/aktivitaet"),
            )
            .header("X-LocalchatA-Sitzung", token)
            .header("Content-Type", "application/json")
            .body(koerper.to_string()),
    )
    .await
    .status
}

/// Perzentil über sortierte Dauern (nächster Rang).
fn perzentil(sortiert: &[Duration], anteil: f64) -> Duration {
    let rang = ((sortiert.len() as f64) * anteil).ceil() as usize;
    sortiert[rang.clamp(1, sortiert.len()) - 1]
}

/// Sendet `anzahl` Nachrichten im Takt `1 / je_s` reihum über die Agenten; je Nachricht (Text, Zeitpunkt der 201,
/// Status).
async fn senden_im_takt(
    basis: &str,
    agenten: &[(Arc<Absender>, String)],
    je_s: u64,
    anzahl: usize,
    praefix: &str,
) -> Vec<(String, Instant, u16)> {
    let takt = Duration::from_micros(1_000_000 / je_s);
    let beginn = Instant::now();
    let mut laufend = Vec::new();
    for n in 0..anzahl {
        tokio::time::sleep_until(beginn + takt * u32::try_from(n).expect("Zahl")).await;
        let (absender, token) = agenten[n % agenten.len()].clone();
        let (basis, text) = (basis.to_string(), format!("{praefix}{n}"));
        laufend.push(tokio::spawn(async move {
            let antwort = senden(&absender, &basis, &token, "allgemein", &text).await;
            (text, Instant::now(), antwort.status)
        }));
    }
    let mut gesendet = Vec::new();
    for aufgabe in laufend {
        gesendet.push(aufgabe.await.expect("Senden"));
    }
    gesendet
}

/// Zustellung je gesendeter Nachricht: Ankunft beim Leser − 201 (früher als die 201 zählt 0).
fn zustellungen(
    gesendet: &[(String, Instant, u16)],
    angekommen: &HashMap<String, Instant>,
) -> (Vec<Duration>, usize) {
    let mut dauern = Vec::new();
    let mut fehlend = 0;
    for (text, antwort, status) in gesendet {
        if *status != 201 {
            continue;
        }
        match angekommen.get(text) {
            Some(ankunft) => dauern.push(ankunft.saturating_duration_since(*antwort)),
            None => fehlend += 1,
        }
    }
    dauern.sort();
    (dauern, fehlend)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "Lastprobe P11 (I0041): cargo test --release --test aktivitaet_last -- --ignored --nocapture"]
async fn p11_zustellung_unter_last_von_tippen_und_nachdenken() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[]);
    let basis = instanz.adresse.clone();
    let mut menschen = Vec::new();
    for n in 0..MENSCHEN {
        let absender = Arc::new(Absender::von(&mensch_adresse(n)));
        let token = beitreten(&absender, &basis, &format!("mensch{n}"), Some("mensch")).await;
        menschen.push((absender, token));
    }
    let mut agenten = Vec::new();
    for n in 0..AGENTEN {
        let absender = Arc::new(Absender::von(&agent_adresse(n)));
        let token = beitreten(&absender, &basis, &format!("agent{n}"), Some("agent")).await;
        agenten.push((absender, token));
    }
    let ende = Instant::now() + LEERLAUF + DAUER + Duration::from_secs(3);
    let mut stroeme = Vec::new();
    for (absender, token) in &menschen {
        let mut strom = Strom::oeffnen(absender, &basis, token).await;
        stroeme.push(tokio::spawn(async move {
            while Instant::now() < ende && !strom.mitlesen(Duration::from_millis(500)).await {}
            strom
        }));
    }
    tokio::time::sleep(Duration::from_millis(500)).await;

    let leerlauf = senden_im_takt(&basis, &agenten[..1], 10, 30, "leer").await;
    let last_beginn = Instant::now();
    let last_ende = last_beginn + DAUER;
    let mut signale = Vec::new();
    for (n, (absender, token)) in menschen.iter().enumerate() {
        let (absender, token, basis) = (Arc::clone(absender), token.clone(), basis.clone());
        let versatz =
            TIPPT_ALLE * u32::try_from(n).expect("Zahl") / u32::try_from(MENSCHEN).expect("Zahl");
        signale.push(tokio::spawn(async move {
            let mut status = Vec::new();
            let mut naechste = last_beginn + versatz;
            while naechste < last_ende {
                tokio::time::sleep_until(naechste).await;
                status.push(setzen(&absender, &basis, &token, json!({"zustand": "tippt"})).await);
                naechste += TIPPT_ALLE;
            }
            status
        }));
    }
    for (n, (absender, token)) in agenten.iter().enumerate() {
        let (absender, token, basis) = (Arc::clone(absender), token.clone(), basis.clone());
        let versatz =
            ERNEUERN_ALLE * u32::try_from(n).expect("Zahl") / u32::try_from(AGENTEN).expect("Zahl");
        signale.push(tokio::spawn(async move {
            let mut status = Vec::new();
            let mut naechste = last_beginn + versatz;
            while naechste < last_ende {
                tokio::time::sleep_until(naechste).await;
                status.push(
                    setzen(
                        &absender,
                        &basis,
                        &token,
                        json!({"zustand": "denkt_nach", "gueltig_s": 30}),
                    )
                    .await,
                );
                naechste += ERNEUERN_ALLE;
            }
            status
        }));
    }
    let anzahl = usize::try_from(NACHRICHTEN_JE_S * DAUER.as_secs()).expect("Zahl");
    let unter_last = senden_im_takt(&basis, &agenten, NACHRICHTEN_JE_S, anzahl, "last").await;
    let mut signal_status = Vec::new();
    for aufgabe in signale {
        signal_status.extend(aufgabe.await.expect("Signale"));
    }
    let mut gelesen = Vec::new();
    for aufgabe in stroeme {
        gelesen.push(aufgabe.await.expect("Strom"));
    }

    let leser = &gelesen[0];
    let angekommen: HashMap<String, Instant> = leser
        .ereignisse
        .iter()
        .filter(|e| e.data["art"] == "nachricht")
        .filter_map(|e| Some((e.data["nachricht"]["text"].as_str()?.to_string(), e.ankunft)))
        .collect();
    let (leer, leer_fehlend) = zustellungen(&leerlauf, &angekommen);
    let (last, last_fehlend) = zustellungen(&unter_last, &angekommen);
    let aktivitaet_beim_leser = leser
        .ereignisse
        .iter()
        .filter(|e| e.data["art"] == "aktivitaet")
        .count();
    let luecken: Vec<usize> = gelesen
        .iter()
        .map(|s| {
            s.ereignisse
                .iter()
                .filter(|e| e.data["art"] == "aktivitaet_luecke")
                .count()
        })
        .collect();
    let mut nachrichten_status: HashMap<u16, usize> = HashMap::new();
    for (_, _, status) in &unter_last {
        *nachrichten_status.entry(*status).or_default() += 1;
    }
    let mut setzen_status: HashMap<u16, usize> = HashMap::new();
    for status in &signal_status {
        *setzen_status.entry(*status).or_default() += 1;
    }
    let episoden_live: Vec<String> = zeilen(&instanz.ordner, HEUTE)
        .into_iter()
        .filter(|z| z.contains(" WARNING live "))
        .collect();
    let leerlauf_p90 = perzentil(&leer, 0.9);
    println!(
        "P11: Leerlauf-p90 {:.1} ms; unter Last n={} p50 {:.1} ms, p90 {:.1} ms, max {:.1} ms (Grenze {} ms); \
         Nachrichten je Status {nachrichten_status:?}; Setzen je Status {setzen_status:?}; aktivitaet beim Leser \
         {aktivitaet_beim_leser}; Lücken je Strom {luecken:?}; Episoden live {}",
        leerlauf_p90.as_secs_f64() * 1e3,
        last.len(),
        perzentil(&last, 0.5).as_secs_f64() * 1e3,
        perzentil(&last, 0.9).as_secs_f64() * 1e3,
        last.last().expect("gemessen").as_secs_f64() * 1e3,
        GRENZE.as_millis(),
        episoden_live.len(),
    );

    assert_eq!(
        (leer_fehlend, last_fehlend),
        (0, 0),
        "jede Nachricht kam an"
    );
    assert!(
        leerlauf_p90 <= LEERLAUF_HOECHSTENS,
        "Lauf ungültig (Rechner ausgelastet), wiederholen: Leerlauf-p90 {leerlauf_p90:?}"
    );
    assert_eq!(
        nachrichten_status.get(&201).copied(),
        Some(anzahl),
        "kein 429 auf Nachrichten: {nachrichten_status:?}"
    );
    assert_eq!(
        setzen_status.get(&200).copied(),
        Some(signal_status.len()),
        "kein Setzen gebremst: {setzen_status:?}"
    );
    assert!(aktivitaet_beim_leser > 0, "Signale sind geflossen");
    let max = *last.last().expect("gemessen");
    assert!(max < GRENZE, "Zustellung max {max:?}");
    assert!(
        luecken.iter().all(|&n| n == 0),
        "aktivitaet_luecke: {luecken:?}"
    );
    assert!(episoden_live.is_empty(), "{episoden_live:#?}");
}
