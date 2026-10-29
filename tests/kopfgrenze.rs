//! R00029 F0100 B0586 — Kopfgrenze der Instanz (ADR-0010 Ergänzung „Karte #9 Nacharbeit“ Regeln 1–4, K2): ab 16 KiB
//! eines noch unvollständigen Kopfes antwortet hyper 431 und schließt; ein Kopf von 16.000 Bytes geht durch; ein großer
//! Körper mit kleinem Kopf ist nie 431. Echtes Programm, Rohsocket.

mod gemeinsam;

use gemeinsam::Absender;
use gemeinsam::anhaenge::starten;
use gemeinsam::senden::{beitreten, ordner};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// `GET /api/gesundheit` (mit `Connection: close`, damit auch die 200 mit EOF endet) und einem Feld `X-Probe`, aufgefüllt, bis der Kopf samt Leerzeile `laenge` Bytes hat.
fn kopf_mit_laenge(laenge: usize) -> Vec<u8> {
    let anfang =
        "GET /api/gesundheit HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\nX-Probe: ";
    let ende = "\r\n\r\n";
    let fuellung = laenge - anfang.len() - ende.len();
    format!("{anfang}{}{ende}", "a".repeat(fuellung)).into_bytes()
}

/// Schreibt `kopf` in Stücken zu `stueck` Bytes je `pause` (Schreibfehler zählen nicht — die Instanz darf früh
/// schließen), liest bis EOF; `None`, wenn die Instanz die Verbindung nach 5 s nicht beendet hat.
async fn senden_und_lesen(
    port: u16,
    kopf: &[u8],
    stueck: usize,
    pause: Duration,
) -> Option<String> {
    let mut verbindung = gemeinsam::herkunft::verbinden(port, "127.0.0.1").await;
    for teil in kopf.chunks(stueck) {
        if verbindung.write_all(teil).await.is_err() {
            break;
        }
        if !pause.is_zero() {
            tokio::time::sleep(pause).await;
        }
    }
    let mut gelesen = Vec::new();
    let ende = tokio::time::timeout(Duration::from_secs(5), async {
        let mut puffer = [0u8; 4_096];
        loop {
            match verbindung.read(&mut puffer).await {
                Ok(0) | Err(_) => break,
                Ok(n) => gelesen.extend_from_slice(&puffer[..n]),
            }
        }
    })
    .await;
    ende.ok()
        .map(|()| String::from_utf8_lossy(&gelesen).into_owned())
}

/// Erste Tests (Kopfgrenze, wörtlich): Kopf 16.000 Bytes → 200; Kopf 100.000 Bytes in einem `write` → 431, danach
/// EOF; Kopf 20.000 Bytes in Stücken zu 1.000 Bytes je 10 ms → 431 (Mutante „`max_header_size` fehlt“).
#[tokio::test]
async fn ab_16_kib_kopf_ist_431() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let port = instanz.port();

    let klein = senden_und_lesen(port, &kopf_mit_laenge(16_000), 16_000, Duration::ZERO)
        .await
        .expect("Ende");
    assert!(
        klein.starts_with("HTTP/1.1 200"),
        "{}",
        &klein[..klein.len().min(200)]
    );

    let gross = senden_und_lesen(port, &kopf_mit_laenge(100_000), 100_000, Duration::ZERO)
        .await
        .expect("die Instanz schließt nach dem 431");
    assert!(gross.starts_with("HTTP/1.1 431"), "{gross}");

    let stueckweise = senden_und_lesen(
        port,
        &kopf_mit_laenge(20_000),
        1_000,
        Duration::from_millis(10),
    )
    .await
    .expect("die Instanz schließt nach dem 431");
    assert!(stueckweise.starts_with("HTTP/1.1 431"), "{stueckweise}");
}

/// Kontrolle: Senden mit 131.072 Bytes Körper und kleinem Kopf → wie bisher (die Abweisung der Route), nicht 431.
#[tokio::test]
async fn ein_grosser_koerper_mit_kleinem_kopf_ist_nie_431() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, basis, "beta", Some("agent")).await;
    let koerper = gemeinsam::senden::aufgefuellt(
        &format!("{{\"text\":\"{}\"}}", "a".repeat(16_384)),
        131_072,
    );
    let antwort = gemeinsam::senden::roh_senden(
        &beta,
        basis,
        Some(&token),
        "allgemein",
        "application/json",
        koerper,
    )
    .await;
    assert_ne!(antwort.status, 431, "{}", antwort.json);
    assert_eq!(antwort.status, 201, "{}", antwort.json);
}
