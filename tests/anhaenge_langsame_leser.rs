//! R00029 F0100 „Langsame Leser“ (QS4; B0447, B0586): 160 Verbindungen einer Herkunft fordern je einen Anhang (1 MiB
//! bzw. 1 GiB) an und lesen nicht → der RSS der Instanz wächst um höchstens 160 × 64 KiB + 50 MB; jede Verbindung
//! ist nach der Abrissgrenze server-seitig zu; die Gesundheit antwortet währenddessen 200. Dazu der Speicher
//! unvollständiger Köpfe (ADR-0010 Ergänzung „Karte #9 Nacharbeit“). Leistungstests, nicht im Gate:
//! `cargo test --release --test anhaenge_langsame_leser -- --ignored --test-threads=1`.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::Absender;
use gemeinsam::anhaenge::{ablegen, starten};
use gemeinsam::langsam::{nicht_lesend_oeffnen, server_seitig_offen};
use gemeinsam::senden::{beitreten, ordner, roh_senden};
use std::time::Duration;

fn rss(pid: u32) -> u64 {
    std::fs::read_to_string(format!("/proc/{pid}/status"))
        .expect("status")
        .lines()
        .find_map(|zeile| zeile.strip_prefix("VmRSS:"))
        .and_then(|wert| {
            wert.trim()
                .trim_end_matches(" kB")
                .trim()
                .parse::<u64>()
                .ok()
        })
        .expect("VmRSS")
        * 1024
}

/// Höchstens: 160 Verbindungen × Puffer je Verbindung + 50 MB (QS4).
const HOECHSTENS: u64 = 160 * 65_536 + 50_000_000;

/// QS4 für einen Anhang von `bytes`: Zuwachs des RSS nach 3 s, alle Verbindungen nach der Abrissgrenze zu.
async fn langsame_leser(bytes: usize) {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &["--anhangquote", "2GiB"], &[]);
    let basis = instanz.adresse.clone();
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, &basis, "beta", Some("agent")).await;
    let id = ablegen(&beta, &basis, &token, "film.bin", vec![0u8; bytes]).await;
    let koerper = serde_json::json!({"text": "", "anhaenge": [id]})
        .to_string()
        .into_bytes();
    assert_eq!(
        roh_senden(
            &beta,
            &basis,
            Some(&token),
            "allgemein",
            "application/json",
            koerper
        )
        .await
        .status,
        201
    );
    let leser = Absender::von("127.0.0.2");
    let token_leser = beitreten(&leser, &basis, "leser", None).await;
    let pid = instanz.kind.id();
    let vorher = rss(pid);

    let beginn = std::time::Instant::now();
    let mut offen = Vec::new();
    for _ in 0..160 {
        offen.push(
            nicht_lesend_oeffnen(
                instanz.port(),
                "127.0.0.2",
                &format!("/anhaenge/{id}"),
                &token_leser,
            )
            .await,
        );
    }
    let mut spitze = vorher;
    while beginn.elapsed() < Duration::from_secs(3) {
        spitze = spitze.max(rss(pid));
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let nach_3_s = rss(pid);
    let (status, _) = gemeinsam::gesundheit(&basis).await;
    assert_eq!(status, 200, "Gesundheit während der langsamen Leser");
    let gegenstellen: Vec<_> = offen.iter().map(|verbindung| verbindung.eigene).collect();
    let mut noch_offen = server_seitig_offen(instanz.port(), &gegenstellen);
    while !noch_offen.is_empty() && beginn.elapsed() < Duration::from_secs(40) {
        tokio::time::sleep(Duration::from_millis(250)).await;
        noch_offen = server_seitig_offen(instanz.port(), &gegenstellen);
    }

    let zuwachs = nach_3_s.saturating_sub(vorher);
    println!(
        "QS4 {bytes} Bytes: RSS-Zuwachs nach 3 s {zuwachs} Bytes (Spitze {} Bytes, Grenze {HOECHSTENS}), alle zu nach {:?}, noch offen {}",
        spitze.saturating_sub(vorher),
        beginn.elapsed(),
        noch_offen.len()
    );
    assert!(zuwachs <= HOECHSTENS, "RSS-Zuwachs {zuwachs}");
    assert!(noch_offen.is_empty(), "{noch_offen:?}");
}

#[tokio::test]
#[ignore = "Leistungstest, Release-Build: cargo test --release --test anhaenge_langsame_leser -- --ignored"]
async fn langsame_leser_eines_anhangs_von_1_mib() {
    langsame_leser(1_048_576).await;
}

#[tokio::test]
#[ignore = "Leistungstest, Release-Build: cargo test --release --test anhaenge_langsame_leser -- --ignored"]
async fn langsame_leser_eines_anhangs_von_1_gib() {
    langsame_leser(1_073_741_824).await;
}

/// Speicher unvollständiger Köpfe (Release, Rohsocket): 160 Verbindungen einer Herkunft senden je 15.000 Bytes Kopf
/// ohne Leerzeile und schweigen → RSS-Zuwachs ≤ 160 × 65.536 + 50 MB; nach der Kopf-Zeitgrenze (10 s) alle zu.
#[tokio::test]
#[ignore = "Leistungstest, Release-Build: cargo test --release --test anhaenge_langsame_leser -- --ignored"]
async fn unvollstaendige_koepfe_halten_wenig_speicher() {
    use tokio::io::AsyncWriteExt;
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let pid = instanz.kind.id();
    let vorher = rss(pid);
    let kopf = format!(
        "GET /api/gesundheit HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Probe: {}",
        "a".repeat(15_000 - 57)
    );
    let beginn = std::time::Instant::now();
    let mut offen = Vec::new();
    for _ in 0..160 {
        let mut verbindung = gemeinsam::herkunft::verbinden(instanz.port(), "127.0.0.2").await;
        verbindung.write_all(kopf.as_bytes()).await.expect("Kopf");
        offen.push(verbindung);
    }
    tokio::time::sleep(Duration::from_secs(3)).await;
    let zuwachs = rss(pid).saturating_sub(vorher);
    let gegenstellen: Vec<_> = offen
        .iter()
        .filter_map(|verbindung| match verbindung.local_addr() {
            Ok(std::net::SocketAddr::V4(eigene)) => Some(eigene),
            _ => None,
        })
        .collect();
    let mut noch_offen = server_seitig_offen(instanz.port(), &gegenstellen);
    while !noch_offen.is_empty() && beginn.elapsed() < Duration::from_secs(20) {
        tokio::time::sleep(Duration::from_millis(250)).await;
        noch_offen = server_seitig_offen(instanz.port(), &gegenstellen);
    }
    println!(
        "Köpfe: RSS-Zuwachs {zuwachs} Bytes (Grenze {HOECHSTENS}), alle zu nach {:?}",
        beginn.elapsed()
    );
    assert!(zuwachs <= HOECHSTENS, "RSS-Zuwachs {zuwachs}");
    assert!(noch_offen.is_empty(), "{noch_offen:?}");
}
