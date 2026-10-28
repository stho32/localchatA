//! F0056 Grenzen je Herkunft über das echte Programm (R00015 G1, Lastprofil „Team-LAN“): 160 offene
//! Verbindungen einer Herkunft werden bedient, die 161. nicht, eine andere Herkunft weiter. Sitzungen (G2) und
//! Channels (G3) prüfen `tests/beitreten.rs` und `tests/channels_grenzen.rs` mit derselben Spanne.
//!
//! Alles geschieht innerhalb der Kopf-Zeitgrenze (10 s, zugleich Leerlaufgrenze): wartete der Test länger,
//! wären die gehaltenen Verbindungen schon geschlossen und die 161. bekäme einen Platz.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::wegwerf_starten;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpSocket, TcpStream};

/// Die Gesamtgrenze ist `min(512, Deskriptoren / 2)`; unter 1.024 Deskriptoren prüfte der Test eine andere
/// Grenze — dann Abbruch statt Befund (R00015, „Gemeinsam“).
fn deskriptoren_reichen() {
    let weich = rustix::process::getrlimit(rustix::process::Resource::Nofile).current;
    assert!(
        weich.is_none_or(|grenze| grenze >= 1024),
        "Abbruch, kein Befund: ulimit -n ist {weich:?}, der Test braucht mindestens 1024"
    );
}

/// Eine Rohverbindung, gebunden an die Herkunft `herkunft` (Linux-Loopback 127.0.0.0/8).
async fn verbinden(port: u16, herkunft: &str) -> TcpStream {
    let socket = TcpSocket::new_v4().expect("Socket");
    socket
        .bind(format!("{herkunft}:0").parse().expect("Herkunft"))
        .expect("gebunden");
    socket
        .connect(format!("127.0.0.1:{port}").parse().expect("Ziel"))
        .await
        .expect("verbunden")
}

/// `GET /api/gesundheit` auf dieser Verbindung (sie bleibt offen) — der Status, oder `None`, wenn die Instanz
/// die Verbindung ohne Antwort schließt.
async fn gesundheit_auf(strom: &mut TcpStream) -> Option<u16> {
    let anfrage = b"GET /api/gesundheit HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n";
    strom.write_all(anfrage).await.ok()?;
    let mut gelesen = Vec::new();
    let mut puffer = [0u8; 4096];
    let ende = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let n = tokio::time::timeout_at(ende, strom.read(&mut puffer))
            .await
            .ok()?
            .ok()?;
        if n == 0 {
            return None;
        }
        gelesen.extend_from_slice(&puffer[..n]);
        if let Some(status) = vollstaendige_antwort(&gelesen) {
            return Some(status);
        }
    }
}

/// Status, sobald Kopf und Körper (nach `Content-Length`) ganz gelesen sind.
fn vollstaendige_antwort(gelesen: &[u8]) -> Option<u16> {
    let text = String::from_utf8_lossy(gelesen);
    let (kopf, inhalt) = text.split_once("\r\n\r\n")?;
    let laenge: usize = kopf
        .lines()
        .find_map(|zeile| {
            let (name, wert) = zeile.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| wert.trim().parse().ok())?
        })
        .expect("Content-Length");
    (inhalt.len() >= laenge).then(|| {
        kopf.split(' ')
            .nth(1)
            .and_then(|code| code.parse().ok())
            .expect("Statuszeile")
    })
}

#[tokio::test]
async fn g1_160_verbindungen_einer_herkunft_werden_bedient_die_161_nicht_eine_andere_herkunft_schon()
 {
    deskriptoren_reichen();
    let instanz = wegwerf_starten();
    let port = instanz.port();

    let mut gehalten = Vec::new();
    let mut status = Vec::new();
    for _ in 0..160 {
        let mut strom = verbinden(port, "127.0.0.1").await;
        status.push(gesundheit_auf(&mut strom).await);
        gehalten.push(strom);
    }
    let mut zu_viel = verbinden(port, "127.0.0.1").await;
    let die_161 = gesundheit_auf(&mut zu_viel).await;
    let mut anderswo = verbinden(port, "127.0.0.2").await;
    let andere_herkunft = gesundheit_auf(&mut anderswo).await;

    let bedient = status.iter().filter(|s| **s == Some(200)).count();
    assert_eq!(bedient, 160, "{status:?}");
    assert_eq!(die_161, None, "die 161. Verbindung wird abgewiesen");
    assert_eq!(andere_herkunft, Some(200));
    drop(gehalten);
}
