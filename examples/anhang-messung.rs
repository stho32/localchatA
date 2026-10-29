//! Messwerkzeug für R00029 F0103 (B0465, B0466; nicht im Gate, Entwicklertest auf dem Zielrechner, Release-Build):
//!
//! - `anhang-messung qa3 <basis> [<pid>]` — zwei `PUT` à 1 GiB gleichzeitig, Gesundheit alle 100 ms, 5 Textnachrichten
//!   je Sekunde; berichtet p90 von Gesundheit und Senden und (mit `<pid>`) den RSS-Zuwachs der Instanz. Die Instanz
//!   braucht `--anhangquote` ≥ 2 GiB. Ohne Hochladungen (Vergleichslauf): `qa3-vergleich <basis>`.
//! - `anhang-messung qa8 <basis> [<laeufe>]` — je Lauf 500 Anhänge à 64 KiB ablegen und senden, dann in einer
//!   bestätigten Anfrage löschen, währenddessen 5 Textnachrichten je Sekunde; berichtet p90 der Löschantwort und des
//!   Sendens. Die Instanz braucht `--anhangquote` ≥ 64 MiB.
//! - `anhang-messung qa5-ordner <ordner> <anzahl>` — legt einen Ordner mit `<anzahl>` Anhängen à 1 KiB über 7 Tage an
//!   (Tagesdateien mit Metazeilen und Dateien unter `anhaenge/<tag>/`); danach die Instanz dreimal mit und ohne starten
//!   und `ladezeit_ms` aus `/api/gesundheit` vergleichen (QA5: Zuwachs ≤ 50 ms).
//!
//! QS6 (Sprung ans Ende eines 2-GiB-Videos) misst der Entwicklertest im Browser; dieses Werkzeug legt die Datei mit
//! `qa3` nicht an.

use serde_json::{Value, json};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const MIB: u64 = 1_048_576;

#[tokio::main]
async fn main() {
    let argumente: Vec<String> = std::env::args().skip(1).collect();
    match argumente
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["qa3", basis, rest @ ..] => {
            qa3(basis, rest.first().and_then(|pid| pid.parse().ok()), true).await
        }
        ["qa3-vergleich", basis] => qa3(basis, None, false).await,
        ["qa8", basis, rest @ ..] => {
            qa8(
                basis,
                rest.first().and_then(|n| n.parse().ok()).unwrap_or(10),
            )
            .await
        }
        ["qa5-ordner", ordner, anzahl] => qa5_ordner(ordner, anzahl.parse().unwrap_or(1_000)),
        _ => eprintln!(
            "Aufruf: anhang-messung qa3 <basis> [<pid>] | qa3-vergleich <basis> | qa8 <basis> [<laeufe>] | qa5-ordner <ordner> <anzahl>"
        ),
    }
}

fn p90(mut werte: Vec<Duration>) -> Duration {
    werte.sort();
    werte.get(werte.len() * 9 / 10).copied().unwrap_or_default()
}

async fn beitreten(client: &reqwest::Client, basis: &str, name: &str) -> String {
    let antwort: Value = client
        .post(format!("{basis}api/beitreten"))
        .json(&json!({"name": name, "art": "agent"}))
        .send()
        .await
        .expect("Beitritt")
        .json()
        .await
        .expect("JSON");
    antwort["token"].as_str().expect("Token").to_string()
}

async fn anmelden(client: &reqwest::Client, basis: &str, token: &str, bytes: u64) -> String {
    let antwort: Value = client
        .post(format!("{basis}api/anhaenge"))
        .header("X-LocalchatA-Sitzung", token)
        .json(&json!({"name": "messung.bin", "bytes": bytes}))
        .send()
        .await
        .expect("Anmelden")
        .json()
        .await
        .expect("JSON");
    antwort["anhang"]["id"]
        .as_str()
        .unwrap_or_else(|| panic!("Anmelden: {antwort}"))
        .to_string()
}

/// Ein `PUT` mit `bytes` Nullbytes über eine Rohverbindung in Stücken zu 1 MiB.
async fn hochladen(basis: &str, token: &str, id: &str, bytes: u64) -> u16 {
    let adresse = basis
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .to_string();
    let mut verbindung = tokio::net::TcpStream::connect(&adresse)
        .await
        .expect("Verbindung");
    let kopf = format!(
        "PUT /api/anhaenge/{id} HTTP/1.1\r\nHost: {adresse}\r\nX-LocalchatA-Sitzung: {token}\r\nContent-Type: application/octet-stream\r\nContent-Length: {bytes}\r\nConnection: close\r\n\r\n"
    );
    verbindung.write_all(kopf.as_bytes()).await.expect("Kopf");
    let stueck = vec![0u8; MIB as usize];
    let mut gesendet = 0;
    while gesendet < bytes {
        let n = (bytes - gesendet).min(MIB) as usize;
        verbindung.write_all(&stueck[..n]).await.expect("Körper");
        gesendet += n as u64;
    }
    let mut antwort = Vec::new();
    let _ = verbindung.read_to_end(&mut antwort).await;
    String::from_utf8_lossy(&antwort)
        .get(9..12)
        .and_then(|code| code.parse().ok())
        .unwrap_or(0)
}

fn rss(pid: u32) -> u64 {
    std::fs::read_to_string(format!("/proc/{pid}/status"))
        .ok()
        .and_then(|status| {
            status
                .lines()
                .find_map(|zeile| zeile.strip_prefix("VmRSS:"))
                .and_then(|wert| {
                    wert.trim()
                        .trim_end_matches(" kB")
                        .trim()
                        .parse::<u64>()
                        .ok()
                })
        })
        .unwrap_or(0)
        * 1024
}

/// Misst Gesundheit (alle 100 ms) und Senden (5/s), bis `fertig` gesetzt ist.
async fn nebenher(
    basis: String,
    fertig: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> (Vec<Duration>, Vec<Duration>) {
    let client = reqwest::Client::new();
    let token = beitreten(&client, &basis, &format!("mess{}", std::process::id())).await;
    let (mut gesundheit, mut senden) = (Vec::new(), Vec::new());
    let mut takt = tokio::time::interval(Duration::from_millis(100));
    let mut runde = 0u64;
    while !fertig.load(std::sync::atomic::Ordering::SeqCst) {
        takt.tick().await;
        let beginn = Instant::now();
        let _ = client.get(format!("{basis}api/gesundheit")).send().await;
        gesundheit.push(beginn.elapsed());
        if runde.is_multiple_of(2) {
            let beginn = Instant::now();
            let _ = client
                .post(format!("{basis}api/channels/allgemein/nachrichten"))
                .header("X-LocalchatA-Sitzung", &token)
                .json(&json!({"text": format!("Messung {runde}")}))
                .send()
                .await;
            senden.push(beginn.elapsed());
        }
        runde += 1;
    }
    (gesundheit, senden)
}

async fn qa3(basis: &str, pid: Option<u32>, mit_hochladen: bool) {
    let client = reqwest::Client::new();
    let rss_vorher = pid.map(rss);
    let fertig = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let messung = tokio::spawn(nebenher(basis.to_string(), std::sync::Arc::clone(&fertig)));
    let mut rss_hoechst = rss_vorher.unwrap_or(0);
    if mit_hochladen {
        let mut laeufe = Vec::new();
        for nummer in 0..2 {
            let token = beitreten(&client, basis, &format!("gross{nummer}")).await;
            let id = anmelden(&client, basis, &token, 1024 * MIB).await;
            let basis = basis.to_string();
            laeufe.push(tokio::spawn(async move {
                hochladen(&basis, &token, &id, 1024 * MIB).await
            }));
        }
        while laeufe.iter().any(|lauf| !lauf.is_finished()) {
            tokio::time::sleep(Duration::from_millis(200)).await;
            rss_hoechst = rss_hoechst.max(pid.map(rss).unwrap_or(0));
        }
        for lauf in laeufe {
            println!("PUT 1 GiB: {}", lauf.await.expect("Lauf"));
        }
    } else {
        tokio::time::sleep(Duration::from_secs(20)).await;
    }
    fertig.store(true, std::sync::atomic::Ordering::SeqCst);
    let (gesundheit, senden) = messung.await.expect("Messung");
    println!(
        "Gesundheit p90: {:?} ({} Proben)",
        p90(gesundheit.clone()),
        gesundheit.len()
    );
    println!(
        "Senden p90: {:?} ({} Proben)",
        p90(senden.clone()),
        senden.len()
    );
    if let Some(vorher) = rss_vorher {
        println!("RSS-Zuwachs: {} Bytes", rss_hoechst.saturating_sub(vorher));
    }
}

async fn qa8(basis: &str, laeufe: usize) {
    let client = reqwest::Client::new();
    let mut loeschen = Vec::new();
    let fertig = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let messung = tokio::spawn(nebenher(basis.to_string(), std::sync::Arc::clone(&fertig)));
    for lauf in 0..laeufe {
        let mut ids = Vec::new();
        for gruppe in 0..125 {
            let token = beitreten(&client, basis, &format!("q{lauf}-{gruppe}")).await;
            let mut eigene = Vec::new();
            for _ in 0..4 {
                let id = anmelden(&client, basis, &token, 64 * 1024).await;
                hochladen(basis, &token, &id, 64 * 1024).await;
                eigene.push(id);
            }
            let _ = client
                .post(format!("{basis}api/channels/allgemein/nachrichten"))
                .header("X-LocalchatA-Sitzung", &token)
                .json(&json!({"text": "", "anhaenge": eigene}))
                .send()
                .await;
            ids.extend(eigene);
        }
        let token = beitreten(&client, basis, &format!("putz{lauf}")).await;
        let beginn = Instant::now();
        let antwort = client
            .post(format!("{basis}api/anhaenge/loeschen"))
            .header("X-LocalchatA-Sitzung", &token)
            .json(&json!({"ids": ids, "bestaetigt": true}))
            .send()
            .await
            .expect("Löschen");
        loeschen.push(beginn.elapsed());
        println!(
            "Lauf {lauf}: {} in {:?}",
            antwort.status(),
            beginn.elapsed()
        );
    }
    fertig.store(true, std::sync::atomic::Ordering::SeqCst);
    let (_, senden) = messung.await.expect("Messung");
    println!("Löschen von 500 p90: {:?}", p90(loeschen));
    println!("Senden p90: {:?}", p90(senden));
}

/// Ordner mit `anzahl` Anhängen à 1 KiB über 7 Tage in `channels/allgemein` (QA5).
fn qa5_ordner(ordner: &str, anzahl: usize) {
    let kanal = std::path::Path::new(ordner).join("channels/allgemein");
    for nummer in 0..anzahl {
        let tagnummer = 22 + nummer % 7;
        let tag = format!("2026-10-{tagnummer:02}");
        let versatz = if tagnummer < 25 { "+02:00" } else { "+01:00" };
        let folge = nummer / 7;
        let id: String = format!("{nummer:0>26}")
            .chars()
            .map(|z| (b'a' + (z as u8 - b'0')) as char)
            .collect();
        let datei = format!("120000000-{}-a.bin", nummer + 1);
        let ordner_tag = kanal.join("anhaenge").join(&tag);
        std::fs::create_dir_all(&ordner_tag).expect("Tag");
        std::fs::write(ordner_tag.join(&datei), vec![1u8; 1024]).expect("Datei");
        let satz = format!(
            "@ {tag}T12:{:02}:{:02}.000{versatz} beta@127.0.0.1\n+ art: agent\n+ anhang: {id} 1024 application/octet-stream - {datei} a.bin\n\n",
            folge / 60,
            folge % 60
        );
        let mut tagesdatei = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(kanal.join(format!("{tag}.log")))
            .expect("Tagesdatei");
        std::io::Write::write_all(&mut tagesdatei, satz.as_bytes()).expect("Satz");
    }
    println!("{anzahl} Anhänge in {}", kanal.display());
}
