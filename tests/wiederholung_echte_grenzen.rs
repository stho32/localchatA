//! R00025 F0087 B0494 „Echte Grenzen“ (ADR-0034 Ergänzung „Karte #36 Angriffsmatrix“ E4, Zeilen 3 L2r und 12): das
//! echte Programm **ohne** Testknopf; viele Loopback-Herkünfte `127.0.1.x` füllen mehr als 36.000 Einträge in etwa
//! einer Minute; ein Opfer mit einem Eintrag wiederholt → 200, ein Satz; `schluessel_verdraengt` ≥ 1; jede Episode
//! nennt einen Fluter, nie das Opfer. Der RSS-Zuwachs der Instanz ist nur **Grobwächter** gegen ungebremstes Wachsen
//! unter Flut (≤ 64 MB, ADR-0034 Ergänzung „Karte #36 Speicher“, S3), nicht Beleg des Speichers — den misst S2
//! (`tests/wiederholung_speicher.rs`).
//!
//! **Optional, `#[ignore]`**: nicht im Standard-Gate (≈ 1 min, 512 Sitzungen), im Entwicklertest ausdrücklich mit
//! `cargo test --test wiederholung_echte_grenzen -- --ignored`. Lesart: Opfer und Fluter teilen die Sitzungsgrenze
//! 512 (ADR-0012) — 511 Fluter auf 128 Herkünften (4 je Herkunft, die letzte 3) und das Opfer.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::senden::{beitreten, ordner, starten, tagesinhalt};
use gemeinsam::{Absender, ApiAntwort, api_antwort};
use serde_json::json;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// Mehr als die Gesamtgrenze, damit sicher verdrängt wird.
const ZIEL: usize = 36_000 + 2_000;
/// Grobwächter Flut (ADR-0034 Ergänzung „Karte #36 Speicher“, S3): Release-Messung 41,3 MB + ≈ 50 % Reserve.
const RSS_ZUWACHS_HOECHSTENS_KB: u64 = 64 * 1_000;
/// Profil des gemessenen Programms — es kommt aus demselben Bau wie dieser Test.
const PROFIL: &str = if cfg!(debug_assertions) {
    "Debug"
} else {
    "Release"
};
const FLUTER: usize = 511;
const HOECHSTENS_DAUER: Duration = Duration::from_secs(150);

async fn mit_schluessel(
    absender: &Absender,
    basis: &str,
    token: &str,
    text: &str,
    schluessel: &str,
) -> ApiAntwort {
    api_antwort(
        absender
            .anfrage(
                reqwest::Method::POST,
                format!("{basis}api/channels/allgemein/nachrichten"),
            )
            .header("X-LocalchatA-Sitzung", token)
            .header("Idempotency-Key", schluessel)
            .json(&json!({ "text": text })),
    )
    .await
}

fn vorbedingung_loopback() {
    let probe = std::net::UdpSocket::bind("127.0.1.2:0");
    assert!(
        probe.is_ok(),
        "Vorbedingung fehlt: Loopback über 127.0.0.1 hinaus (127.0.1.2) nicht nutzbar: {probe:?}"
    );
}

fn rss_kb(pid: u32) -> u64 {
    let status =
        std::fs::read_to_string(format!("/proc/{pid}/status")).expect("/proc/<pid>/status");
    status
        .lines()
        .find_map(|zeile| zeile.strip_prefix("VmRSS:"))
        .and_then(|wert| wert.trim().trim_end_matches("kB").trim().parse().ok())
        .expect("VmRSS")
}

/// Ein Fluter sendet mit eigenen Schlüsseln, bis zusammen `ZIEL` Einträge angenommen sind; bei 429 wartet er.
async fn fluten(
    absender: Absender,
    basis: String,
    token: String,
    name: String,
    angenommen: Arc<AtomicUsize>,
) {
    let beginn = Instant::now();
    let mut nummer = 0;
    while angenommen.load(Ordering::SeqCst) < ZIEL && beginn.elapsed() < HOECHSTENS_DAUER {
        let schluessel = format!("{name}-{nummer}");
        let antwort = mit_schluessel(&absender, &basis, &token, "Flut", &schluessel).await;
        match antwort.status {
            201 => {
                angenommen.fetch_add(1, Ordering::SeqCst);
                nummer += 1;
            }
            429 => {
                let warten = antwort.json["wieder_in_ms"].as_u64().unwrap_or(1_000);
                tokio::time::sleep(Duration::from_millis(warten)).await;
            }
            anderer => panic!("{name}: {anderer} {}", antwort.json),
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "optional, ≈ 1 min: cargo test --test wiederholung_echte_grenzen -- --ignored"]
async fn echte_grenzen_halten_den_schluessel_des_opfers_und_den_speicher() {
    vorbedingung_loopback();
    let wurzel = ordner();
    let mut instanz = starten(wurzel.path(), &[]);
    // Die Flut erzeugt Hunderte WARNING-Episoden, die auf stderr gespiegelt werden: mitlesen, sonst hält die volle
    // Pipe das Protokoll an.
    let _stderr = instanz.stderr_mitschreiben();
    let basis = instanz.adresse.clone();
    let opfer = Absender::von("127.0.0.1");
    let token = beitreten(&opfer, &basis, "opfer", None).await;
    let erste = mit_schluessel(&opfer, &basis, &token, "Wichtig", "v1").await;
    assert_eq!(erste.status, 201, "{}", erste.json);
    let mut fluter = Vec::new();
    for nummer in 0..FLUTER {
        let absender = Absender::von(&format!("127.0.1.{}", 2 + nummer / 4));
        let name = format!("f{nummer}");
        let token = beitreten(&absender, &basis, &name, Some("agent")).await;
        fluter.push((absender, token, name));
    }
    let rss_vorher = rss_kb(instanz.kind.id());
    let angenommen = Arc::new(AtomicUsize::new(0));
    let beginn = Instant::now();

    let laeufe: Vec<_> = fluter
        .into_iter()
        .map(|(absender, token, name)| {
            tokio::spawn(fluten(
                absender,
                basis.clone(),
                token,
                name,
                Arc::clone(&angenommen),
            ))
        })
        .collect();
    for lauf in laeufe {
        lauf.await.expect("Fluter");
    }
    let dauer = beginn.elapsed();
    let rss_nachher = rss_kb(instanz.kind.id());
    let wiederholt = mit_schluessel(&opfer, &basis, &token, "Wichtig", "v1").await;
    let (_, gesundheit) = gemeinsam::gesundheit(&basis).await;

    eprintln!(
        "echte Grenzen ({PROFIL}): {} angenommen in {dauer:.1?}, RSS {rss_vorher} → {rss_nachher} kB (+{} kB, \
         Grobwächter ≤ {RSS_ZUWACHS_HOECHSTENS_KB} kB), verdrängt {}",
        angenommen.load(Ordering::SeqCst),
        rss_nachher.saturating_sub(rss_vorher),
        gesundheit["schluessel_verdraengt"]
    );
    assert!(
        angenommen.load(Ordering::SeqCst) >= ZIEL,
        "Flut zu langsam: {dauer:?}"
    );
    assert_eq!(wiederholt.status, 200, "{}", wiederholt.json);
    let saetze = String::from_utf8_lossy(&tagesinhalt(wurzel.path(), "allgemein"))
        .lines()
        .filter(|zeile| *zeile == "  Wichtig")
        .count();
    assert_eq!(saetze, 1);
    assert!(
        gesundheit["schluessel_verdraengt"]
            .as_u64()
            .expect("Zähler")
            >= 1
    );
    let beginne: Vec<String> = gemeinsam::protokoll::alle_zeilen(wurzel.path())
        .into_iter()
        .filter(|zeile| zeile.contains("Wiederholschlüssel vor Ablauf verdrängt — eine Wiederholung kann doppelt ankommen — Episode beginnt"))
        .collect();
    assert!(!beginne.is_empty());
    assert!(
        beginne
            .iter()
            .all(|zeile| zeile.contains(" kennung=f") && !zeile.contains("opfer")),
        "{beginne:?}"
    );
    assert!(
        rss_nachher.saturating_sub(rss_vorher) <= RSS_ZUWACHS_HOECHSTENS_KB,
        "Grobwächter Flut ({PROFIL}): RSS-Zuwachs {} kB über {RSS_ZUWACHS_HOECHSTENS_KB} kB — die Instanz wächst \
         unter Flut ungebremst (ADR-0034 Ergänzung „Karte #36 Speicher“, S3); den Speicher des Wiederholspeichers \
         misst S2: cargo test --test wiederholung_speicher -- --nocapture",
        rss_nachher.saturating_sub(rss_vorher)
    );
}
