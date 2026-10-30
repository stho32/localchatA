//! R00047 F0157 „Leistung“ (ADR-0053 P9, QZ3, Pre-Mortem 3): ein Channel mit 100.000 Nachrichten über 7 Tage, jede
//! zweite eine Antwort auf eine zufällige ältere (fester Startwert), 10 % der Originale bearbeitet, 5 % gelöscht. Q1:
//! die neuesten 100 unter 50 ms (Median aus 20, gemessen vom Senden bis zum vollständig gelesenen Inhalt, also eine
//! obere Schranke der Server-Zeit); Q6: Laden beim Start unter 2 s (`ladezeit_ms`). Die Grenzen sind die der
//! Anforderung, nie schärfer. Im Gate im Debug-Build (Q1 und die Zitate), die Messung für den Bericht im
//! Release-Build: `cargo test --release --test leistung_antworten -- --nocapture`.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::{Absender, Instanz, gesundheit, starten_mit};
use serde_json::{Value, json};
use std::fmt::Write as _;
use std::path::Path;
use std::time::{Duration, Instant};

const JETZT: &str = "2026-10-28T15:00:00.000+01:00";
const NACHRICHTEN: usize = 100_000;
const TAGE: usize = 7;
const JE_TAG: usize = NACHRICHTEN / TAGE + 1;
/// Q1 (ADR-0006, ADR-0025): neueste 100 unter 50 ms.
const GRENZE_Q1: Duration = Duration::from_millis(50);
/// Q6: Start unter 2 s.
const GRENZE_Q6_MS: u64 = 2_000;
const ORIGINAL: &str = "claude@10.0.0.5";
const ANTWORTENDER: &str = "codex@10.0.0.6";

/// Nachricht `nummer` ist eine Antwort, wenn ungerade; sonst ein Original.
fn ist_antwort(nummer: usize) -> bool {
    nummer % 2 == 1
}

/// Jedes zehnte Original wird bearbeitet, jedes zwanzigste (andere) gelöscht.
fn bearbeitet(nummer: usize) -> bool {
    nummer.is_multiple_of(20)
}

fn geloescht(nummer: usize) -> bool {
    nummer % 40 == 2
}

/// Die Zeit der Nachricht `nummer` (3 s Abstand ab Mitternacht je Tag) plus `ms`.
fn zeit(nummer: usize, ms: usize) -> String {
    let datum = format!("2026-10-{:02}", 22 + nummer / JE_TAG);
    let sekunden = nummer % JE_TAG * 3;
    format!(
        "{datum}T{:02}:{:02}:{:02}.{ms:03}+01:00",
        sekunden / 3600,
        sekunden / 60 % 60,
        sekunden % 60
    )
}

/// Ein fester Zufall (lineare Kongruenz), damit jeder Lauf denselben Bestand hat.
struct Zufall(u64);

impl Zufall {
    fn unter(&mut self, grenze: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) as usize % grenze
    }
}

/// Erzeugt `channels/gross/` mit dem Bestand der Probe; liefert (Antworten, Änderungssätze).
fn bestand_erzeugen(ordner: &Path) -> (usize, usize) {
    let pfad = ordner.join("channels/gross");
    std::fs::create_dir_all(&pfad).expect("Channel");
    let mut zufall = Zufall(4711);
    let mut antworten = 0;
    let mut aenderungen = 0;
    for tag in 0..TAGE {
        let mut rohtext = String::new();
        for nummer in tag * JE_TAG..((tag + 1) * JE_TAG).min(NACHRICHTEN) {
            if ist_antwort(nummer) {
                let bezug = zufall.unter(nummer);
                let von = if ist_antwort(bezug) {
                    ANTWORTENDER
                } else {
                    ORIGINAL
                };
                let _ = write!(
                    rohtext,
                    "@ {} {ANTWORTENDER}\n+ art: agent\n+ antwort_auf: {} {von}\n  Antwort auf {bezug}\n\n",
                    zeit(nummer, 0),
                    zeit(bezug, 0)
                );
                antworten += 1;
                continue;
            }
            let _ = write!(
                rohtext,
                "@ {} {ORIGINAL}\n+ art: agent\n  ",
                zeit(nummer, 0)
            );
            for wort in 0..(10 + nummer % 81) {
                rohtext.push_str(if wort % 9 == 8 {
                    "Größe\n  "
                } else {
                    "Nachricht "
                });
            }
            rohtext.push_str("\n\n");
            if bearbeitet(nummer) {
                let _ = write!(
                    rohtext,
                    "@ {} {ORIGINAL}\n+ bearbeitet: {}\n  Neue Fassung von {nummer}\n\n",
                    zeit(nummer, 1),
                    zeit(nummer, 0)
                );
                aenderungen += 1;
            }
            if geloescht(nummer) {
                let _ = write!(
                    rohtext,
                    "@ {} {ORIGINAL}\n+ geloescht: {}\n\n",
                    zeit(nummer, 1),
                    zeit(nummer, 0)
                );
                aenderungen += 1;
            }
        }
        std::fs::write(pfad.join(format!("2026-10-{:02}.log", 22 + tag)), rohtext).expect("Tag");
    }
    (antworten, aenderungen)
}

/// Die neuesten 100 und die Dauer vom Senden bis zum vollständig gelesenen Inhalt.
async fn neueste(beta: &Absender, instanz: &Instanz, token: &str) -> (Value, Duration) {
    let url = format!("{}api/channels/gross/nachrichten", instanz.adresse);
    let beginn = Instant::now();
    let antwort = beta
        .anfrage(reqwest::Method::GET, url)
        .header("X-LocalchatA-Sitzung", token)
        .send()
        .await
        .expect("Antwort");
    let status = antwort.status();
    let text = antwort.text().await.expect("Inhalt");
    let dauer = beginn.elapsed();
    assert_eq!(status, 200, "{text}");
    (serde_json::from_str(&text).expect("JSON"), dauer)
}

/// Ein Lauf: Start, `ladezeit_ms`, zwanzigmal die neuesten 100 (sortiert), der letzte Block, die Gesundheit.
struct Messung {
    ladezeit_ms: u64,
    dauern: Vec<Duration>,
    block: Value,
    gesundheit: Value,
}

async fn messen() -> Messung {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let (antworten, aenderungen) = bestand_erzeugen(wurzel.path());
    let mut instanz = starten_mit(
        wurzel.path(),
        &["--port", "0", "--host", "127.0.0.1", "--kein-browser"],
        &[("LOCALCHATA_JETZT", JETZT), ("TZ", "Europe/Berlin")],
    );
    let (_, gesundheit) = gesundheit(&instanz.adresse).await;
    let beta = Absender::von("127.0.0.3");
    let beitritt = beta
        .beitreten(&instanz.adresse, json!({ "name": "beta" }), None)
        .await;
    let token = beitritt.json["token"].as_str().expect("Token").to_string();
    let mut dauern = Vec::new();
    let mut block = Value::Null;
    for _ in 0..20 {
        let (antwort, dauer) = neueste(&beta, &instanz, &token).await;
        dauern.push(dauer);
        block = antwort;
    }
    dauern.sort();
    assert!(instanz.signal_und_warten("TERM").success());
    let messung = Messung {
        ladezeit_ms: gesundheit["ladezeit_ms"].as_u64().expect("ladezeit_ms"),
        dauern,
        block,
        gesundheit,
    };
    println!(
        "P9 {}: {antworten} Antworten, {aenderungen} Änderungssätze, ladezeit_ms {}, neueste 100 min {:?} median {:?} \
         max {:?}",
        if cfg!(debug_assertions) {
            "Debug"
        } else {
            "Release"
        },
        messung.ladezeit_ms,
        messung.dauern[0],
        messung.dauern[10],
        messung.dauern[19]
    );
    messung
}

/// ADR-0053 P9 (Gate, Debug-Build): der Block liefert die Zitate im Zustand des Originals (`da`, `bearbeitet` mit der
/// neuen Fassung, `geloescht` ohne Text), keine Defekte, und die neuesten 100 kommen im Median unter Q1.
#[tokio::test]
async fn p9_viele_antworten_bremsen_den_verlauf_nicht() {
    let messung = messen().await;

    assert_eq!(messung.gesundheit["nachrichten"], NACHRICHTEN);
    assert_eq!(messung.gesundheit["defekte_zeilen"], 0);
    let nachrichten = messung.block["nachrichten"]
        .as_array()
        .expect("nachrichten");
    assert_eq!(nachrichten.len(), 100);
    let mut zustaende = std::collections::BTreeSet::new();
    for nachricht in nachrichten.iter().filter(|n| !n["antwort_auf"].is_null()) {
        let zitat = &nachricht["antwort_auf"];
        let zustand = zitat["zustand"].as_str().expect("zustand");
        zustaende.insert(zustand.to_string());
        let textanfang = zitat["textanfang"].as_str().expect("textanfang");
        match zustand {
            "bearbeitet" => assert!(textanfang.starts_with("Neue Fassung von "), "{zitat}"),
            "geloescht" => assert_eq!(textanfang, "", "{zitat}"),
            "da" => assert!(
                textanfang.starts_with("Nachricht ") || textanfang.starts_with("Antwort auf "),
                "{zitat}"
            ),
            _ => panic!("unerwarteter Zustand: {zitat}"),
        }
    }
    assert_eq!(
        nachrichten
            .iter()
            .filter(|n| !n["antwort_auf"].is_null())
            .count(),
        50
    );
    assert!(zustaende.contains("da"), "{zustaende:?}");
    assert!(
        messung.dauern[10] < GRENZE_Q1,
        "Q1: Median {:?} ≥ {GRENZE_Q1:?}",
        messung.dauern[10]
    );
}

/// ADR-0053 P9 (Release): Start unter 2 s und die neuesten 100 im Median unter 50 ms — Q6 gilt im Release-Build
/// (der Debug-Build parst etwa neunmal langsamer, eine Debug-Grenze wäre erfunden).
#[tokio::test]
#[cfg_attr(
    debug_assertions,
    ignore = "Q6 gilt im Release-Build: cargo test --release --test leistung_antworten"
)]
async fn p9_viele_antworten_bremsen_den_start_nicht() {
    let messung = messen().await;

    assert_eq!(messung.gesundheit["nachrichten"], NACHRICHTEN);
    assert!(
        messung.ladezeit_ms < GRENZE_Q6_MS,
        "Q6: ladezeit_ms {} ≥ {GRENZE_Q6_MS}",
        messung.ladezeit_ms
    );
    assert!(
        messung.dauern[10] < GRENZE_Q1,
        "Q1: Median {:?} ≥ {GRENZE_Q1:?}",
        messung.dauern[10]
    );
}
