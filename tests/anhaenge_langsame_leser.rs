//! R00029 F0100 „Langsame Leser“ (QS4; B0447): 160 Verbindungen einer Herkunft fordern je eine Datei von 64 MiB an
//! und lesen nicht → der RSS der Instanz wächst um höchstens 160 × 64 KiB + 50 MB; jede Verbindung ist nach der
//! Abrissgrenze (10 s) server-seitig zu. Leistungstest wie `langsame_leser.rs`, nicht im Gate:
//! `cargo test --release --test anhaenge_langsame_leser -- --ignored`. Stand 2026-10-29 (Debug-Build): alle
//! Verbindungen nach 20 s zu, aber RSS-Zuwachs 82 MB statt höchstens 60,5 MB — offener Befund (hyper puffert je
//! Verbindung mehrere Stücke), im Bericht an den Director.
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

#[tokio::test]
#[ignore = "Leistungstest, Release-Build: cargo test --release --test anhaenge_langsame_leser -- --ignored"]
async fn langsame_leser_einer_datei_von_64_mib() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &["--anhangquote", "1GiB"], &[]);
    let basis = instanz.adresse.clone();
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, &basis, "beta", Some("agent")).await;
    let id = ablegen(&beta, &basis, &token, "film.bin", vec![0u8; 64 * 1_048_576]).await;
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
    let mut hoechst = vorher;
    let beginn = std::time::Instant::now();
    while beginn.elapsed() < Duration::from_secs(5) {
        hoechst = hoechst.max(rss(pid));
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    tokio::time::sleep(Duration::from_secs(15)).await;
    let gegenstellen: Vec<_> = offen.iter().map(|verbindung| verbindung.eigene).collect();
    let noch_offen = server_seitig_offen(instanz.port(), &gegenstellen);

    let zuwachs = hoechst.saturating_sub(vorher);
    println!(
        "QS4: RSS-Zuwachs {zuwachs} Bytes, nach 20 s noch offen {}",
        noch_offen.len()
    );
    assert!(
        zuwachs <= 160 * 65_536 + 50_000_000,
        "RSS-Zuwachs {zuwachs}"
    );
    assert!(noch_offen.is_empty(), "{noch_offen:?}");
}
