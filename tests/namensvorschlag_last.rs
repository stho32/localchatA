//! Nacharbeit 1 zu R00006 (Security, CWE-407/400): ein langer, ungültiger Name bindet den Server nicht. Der
//! Vorschlag entsteht aus höchstens 4 × 32 Zeichen, die Akzent-Abfrage ist O(1) — gegen das echte Programm.
//! Grenze 0,15 s aus der Abnahme-Probe des Security-Reviews (Antwortzeit je Anfrage, p90 der Gesundheit
//! unter Last); gemessen ab dem Senden des fertig kodierten Körpers bis zur vollständigen Antwort.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::{Absender, api_antwort, wegwerf_starten};
use reqwest::Method;
use serde_json::{Value, json};
use std::time::{Duration, Instant};

const GRENZE: Duration = Duration::from_millis(150);

/// Körper an der Grenze von 4 096 Bytes (R00010; bis dahin knapp unter 2 MB), mit dem Vorschlag, den der
/// Server macht.
fn lange_namen() -> Vec<(&'static str, Vec<u8>, Option<String>)> {
    let koerper = |name: String| serde_json::to_vec(&json!({ "name": name })).expect("JSON");
    vec![
        ("ÿ×2042", koerper("ÿ".repeat(2_042)), Some("y".repeat(32))),
        ("日×1361", koerper("日".repeat(1_361)), None),
        ("a×4085", koerper("a".repeat(4_085)), Some("a".repeat(32))),
    ]
}

async fn senden(
    absender: &Absender,
    url: &str,
    token: Option<&str>,
    koerper: &[u8],
) -> (u16, Value, Duration) {
    let mut anfrage = absender
        .anfrage(Method::POST, url.to_string())
        .header("Content-Type", "application/json")
        .body(koerper.to_vec());
    if let Some(token) = token {
        anfrage = anfrage.header("X-LocalchatA-Sitzung", token);
    }
    let beginn = Instant::now();
    let antwort = api_antwort(anfrage).await;
    (antwort.status, antwort.json, beginn.elapsed())
}

fn verteilung(dauern: &mut [Duration]) -> String {
    dauern.sort();
    format!(
        "min {:?}, Median {:?}, max {:?}",
        dauern[0],
        dauern[dauern.len() / 2],
        dauern[dauern.len() - 1]
    )
}

#[tokio::test]
async fn lange_namen_beantwortet_der_server_mit_422_und_vorschlag_in_hoechstens_150_ms() {
    let instanz = wegwerf_starten();
    let fremder = Absender::von("127.0.0.2");
    let mitglied = Absender::von("127.0.0.3");
    let beitritt = mitglied
        .beitreten(&instanz.adresse, json!({ "name": "mitglied" }), None)
        .await;
    assert_eq!(beitritt.status, 201, "{}", beitritt.json);
    let token = beitritt.json["token"].as_str().expect("Token").to_string();
    let wege = [
        ("beitreten", &fremder, None, "api/beitreten"),
        ("channels", &mitglied, Some(token.as_str()), "api/channels"),
    ];

    for (weg, absender, token, pfad) in wege {
        let url = format!("{}{pfad}", instanz.adresse);
        for (name, koerper, erwartet) in lange_namen() {
            let mut dauern = Vec::new();
            for _ in 0..3 {
                let (status, inhalt, dauer) = senden(absender, &url, token, &koerper).await;
                assert_eq!(status, 422, "{weg} {name}: {inhalt}");
                assert_eq!(
                    inhalt["vorschlag"].as_str(),
                    erwartet.as_deref(),
                    "{weg} {name}"
                );
                dauern.push(dauer);
            }
            let hoechstens = *dauern.iter().max().expect("Messung");
            println!("{weg} {name}: {}", verteilung(&mut dauern));
            assert!(
                hoechstens <= GRENZE,
                "{weg} {name}: {hoechstens:?} > {GRENZE:?}"
            );
        }
    }
}

/// 24 Schleifen von vier Herkünften schicken den teuersten Körper; eine fünfte Herkunft fragt die Gesundheit.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn unter_last_langer_namen_antwortet_die_gesundheit_im_p90_in_hoechstens_150_ms() {
    let instanz = wegwerf_starten();
    let url = format!("{}api/beitreten", instanz.adresse);
    let (_, koerper, _) = lange_namen().remove(0);
    let ende = Instant::now() + Duration::from_secs(4);
    let mut schleifen = Vec::new();
    for nummer in 0..24 {
        let absender = Absender::von(&format!("127.0.0.{}", 2 + nummer % 4));
        let url = url.clone();
        let koerper = koerper.clone();
        schleifen.push(tokio::spawn(async move {
            let mut gesendet = 0;
            while Instant::now() < ende {
                let (status, _, _) = senden(&absender, &url, None, &koerper).await;
                assert_eq!(status, 422);
                gesendet += 1;
            }
            gesendet
        }));
    }

    tokio::time::sleep(Duration::from_millis(500)).await;
    let beobachter = Absender::von("127.0.0.6");
    let gesundheit = format!("{}api/gesundheit", instanz.adresse);
    let mut dauern = Vec::new();
    while Instant::now() < ende {
        let beginn = Instant::now();
        let antwort = api_antwort(beobachter.anfrage(Method::GET, gesundheit.clone())).await;
        dauern.push(beginn.elapsed());
        assert_eq!(antwort.status, 200);
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let mut gesendet = 0;
    for schleife in schleifen {
        gesendet += schleife.await.expect("Schleife");
    }

    dauern.sort();
    let p90 = dauern[dauern.len() * 9 / 10 - 1];
    println!(
        "Last: {gesendet} lange Namen, {} Gesundheit, p90 {p90:?}, {}",
        dauern.len(),
        verteilung(&mut dauern)
    );
    assert!(p90 <= GRENZE, "p90 der Gesundheit {p90:?} > {GRENZE:?}");
}
