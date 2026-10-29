//! R00014 US-17, Kürzen an der Quelle (Security-Review #24, O1, CWE-770; ADR-0040, Ergänzung): riesige
//! `Host:`-Köpfe von vielen Herkünften blähen die Instanz nicht auf — eine offene Episode hält den Wert nur
//! gekürzt. Über das echte Programm; RSS aus `/proc/<pid>/status`.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::protokoll::wert;
use gemeinsam::wegwerf_starten;
use std::net::Ipv4Addr;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpSocket;

/// So viele Herkünfte schicken je einen riesigen Kopf (Abnahme-Probe des security-reviewer).
const HERKUENFTE: u32 = 300;
/// So groß ist der `Host:`-Wert je Anfrage.
const KOPF_BYTES: usize = 400_000;
/// Höchstens so viel darf der RSS wachsen (gemessen vor dem Fix: 309 MB).
const HOECHSTENS_ZUWACHS_KB: u64 = 50 * 1024;

fn rss_kb(pid: u32) -> u64 {
    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).expect("status");
    status
        .lines()
        .find_map(|zeile| zeile.strip_prefix("VmRSS:"))
        .and_then(|rest| rest.trim().trim_end_matches("kB").trim().parse().ok())
        .expect("VmRSS")
}

/// `127.0.0.2` … `127.0.0.255`, dann weiter in `127.0.1.0/24`.
fn herkunft(nummer: u32) -> Ipv4Addr {
    Ipv4Addr::from(u32::from(Ipv4Addr::new(127, 0, 0, 2)) + nummer)
}

/// Eine Anfrage `GET /` mit riesigem Host von `von`; liefert den Status.
async fn riesiger_host(port: u16, von: Ipv4Addr, kopf: &str) -> u16 {
    let socket = TcpSocket::new_v4().expect("Socket");
    socket
        .bind((von, 0).into())
        .expect("an die Herkunft gebunden");
    let mut strom = socket
        .connect((Ipv4Addr::LOCALHOST, port).into())
        .await
        .expect("verbunden");
    let anfrage = format!("GET / HTTP/1.1\r\nHost: {kopf}\r\nConnection: close\r\n\r\n");
    strom.write_all(anfrage.as_bytes()).await.expect("gesendet");
    let mut antwort = Vec::new();
    tokio::time::timeout(Duration::from_secs(10), strom.read_to_end(&mut antwort))
        .await
        .expect("Antwort binnen 10 s")
        .expect("gelesen");
    let text = String::from_utf8_lossy(&antwort);
    text.split(' ')
        .nth(1)
        .and_then(|status| status.parse().ok())
        .unwrap_or_else(|| panic!("keine Statuszeile: {}", text.lines().next().unwrap_or("")))
}

#[tokio::test]
async fn us17_dreihundert_riesige_host_koepfe_lassen_den_rss_um_hoechstens_50_mb_wachsen() {
    let instanz = wegwerf_starten();
    let pid = instanz.kind.id();
    let kopf = "a".repeat(KOPF_BYTES);
    let vorher = rss_kb(pid);

    let mut status = Vec::new();
    for nummer in 0..HERKUENFTE {
        status.push(riesiger_host(instanz.port(), herkunft(nummer), &kopf).await);
    }
    let nachher = rss_kb(pid);

    assert!(status.iter().all(|&s| s == 403), "{status:?}");
    let zuwachs = nachher.saturating_sub(vorher);
    eprintln!("messung us17_rss_zuwachs_kb={zuwachs} vorher_kb={vorher} nachher_kb={nachher}");
    assert!(
        zuwachs <= HOECHSTENS_ZUWACHS_KB,
        "RSS um {zuwachs} kB gewachsen (vorher {vorher} kB, nachher {nachher} kB)"
    );
    let grenzschutz: Vec<String> = gemeinsam::protokoll::alle_zeilen(&instanz.ordner)
        .into_iter()
        .filter(|zeile| zeile.contains(" WARNING grenzschutz "))
        .collect();
    assert_eq!(
        grenzschutz.len(),
        HERKUENFTE as usize,
        "eine Episode je Herkunft"
    );
    for zeile in &grenzschutz {
        let grund = wert(zeile, "grund").expect("grund=");
        assert!(grund.chars().count() <= 256, "{}", grund.len());
        assert!(grund.ends_with('…'), "{grund}");
    }
}
