//! R00040 F0140 „Verlauf bleibt schnell mit vielen Änderungen“ (ADR-0048 QE2, Pre-Mortem 2): ein Channel mit 100.000
//! Nachrichten über 7 Tage, davon 10.000 je dreimal bearbeitet und 5.000 gelöscht — 35.000 Änderungssätze, jeder
//! direkt hinter seiner Nachricht im Tag. Q1: die neuesten 100 in weniger als 50 ms (gemessen vom Senden bis zum
//! vollständig gelesenen Inhalt, also eine obere Schranke der Server-Zeit); Q6: Laden beim Start unter 2 s
//! (`ladezeit_ms`). Die Grenzen sind die der Anforderung, nie schärfer; derselbe Lauf ohne Änderungssätze steht zum
//! Vergleich in der Ausgabe (`--nocapture`). Im Gate im Debug-Build, die Messung für den Bericht im Release-Build.
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
/// Q1 (ADR-0006, ADR-0025): neueste 100 unter 50 ms.
const GRENZE_Q1: Duration = Duration::from_millis(50);
/// Q6: Start unter 2 s.
const GRENZE_Q6_MS: u64 = 2_000;

/// Nachricht `nummer` (über alle Tage): jede zehnte dreimal bearbeitet, jede zwanzigste ab der fünften gelöscht.
fn bearbeitet(nummer: usize) -> bool {
    nummer % 10 == 0
}

fn geloescht(nummer: usize) -> bool {
    nummer % 20 == 5
}

/// Erzeugt `channels/gross/` mit 100.000 Nachrichten (3 s Abstand ab Mitternacht) und, wenn `mit_aenderungen`,
/// hinter jeder betroffenen Nachricht ihre Änderungssätze (+1, +2, +3 ms). Liefert die Zahl der Änderungssätze.
fn bestand_erzeugen(ordner: &Path, mit_aenderungen: bool) -> usize {
    let pfad = ordner.join("channels/gross");
    std::fs::create_dir_all(&pfad).expect("Channel");
    let je_tag = NACHRICHTEN / TAGE + 1;
    let mut nummer = 0;
    let mut aenderungen = 0;
    for tag in 0..TAGE {
        let datum = format!("2026-10-{:02}", 22 + tag);
        let mut rohtext = String::new();
        for im_tag in 0..je_tag {
            if nummer == NACHRICHTEN {
                break;
            }
            let sekunden = im_tag * 3;
            let zeit = |ms: usize| {
                format!(
                    "{datum}T{:02}:{:02}:{:02}.{ms:03}+01:00",
                    sekunden / 3600,
                    sekunden / 60 % 60,
                    sekunden % 60
                )
            };
            let _ = writeln!(rohtext, "@ {} claude@10.0.0.5", zeit(0));
            rohtext.push_str("  ");
            for wort in 0..(10 + nummer % 81) {
                rohtext.push_str(if wort % 9 == 8 {
                    "Größe\n  "
                } else {
                    "Nachricht "
                });
            }
            rohtext.push_str("\n\n");
            if mit_aenderungen && bearbeitet(nummer) {
                for fassung in 1..=3 {
                    let _ = write!(
                        rohtext,
                        "@ {} claude@10.0.0.5\n+ bearbeitet: {}\n  Fassung {fassung} der Nachricht {nummer}\n\n",
                        zeit(fassung),
                        zeit(0)
                    );
                    aenderungen += 1;
                }
            }
            if mit_aenderungen && geloescht(nummer) {
                let _ = write!(
                    rohtext,
                    "@ {} claude@10.0.0.5\n+ geloescht: {}\n\n",
                    zeit(1),
                    zeit(0)
                );
                aenderungen += 1;
            }
            nummer += 1;
        }
        std::fs::write(pfad.join(format!("{datum}.log")), rohtext).expect("Tag");
    }
    aenderungen
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

/// Ein Lauf: Start, `ladezeit_ms`, zwanzigmal die neuesten 100 (min, Median, max), der letzte Block.
struct Messung {
    ladezeit_ms: u64,
    dauern: Vec<Duration>,
    block: Value,
    gesundheit: Value,
}

async fn messen(mit_aenderungen: bool) -> (Messung, usize) {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let aenderungen = bestand_erzeugen(wurzel.path(), mit_aenderungen);
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
        "QE2 {}: {aenderungen} Änderungssätze, ladezeit_ms {}, neueste 100 min {:?} median {:?} max {:?}",
        if mit_aenderungen {
            "mit Änderungen"
        } else {
            "ohne Änderungen (Vergleich)"
        },
        messung.ladezeit_ms,
        messung.dauern[0],
        messung.dauern[10],
        messung.dauern[19]
    );
    (messung, aenderungen)
}

/// R00040 „Messung QE2“, Teil Verlauf (Gate): mit 35.000 Änderungssätzen liefert der Block die Fassungen (bearbeitete
/// mit dem Text der dritten Fassung, gelöschte leer, die übrigen unverändert), `nachrichten` zählt nur Nachrichten,
/// keine Defekte, und die neuesten 100 kommen unter Q1 — auch im Debug-Build (gemessen ≈ 8 ms).
#[tokio::test]
async fn qe2_viele_aenderungen_bremsen_den_verlauf_nicht() {
    let (mit, aenderungen) = messen(true).await;

    assert_eq!(aenderungen, 35_000);
    assert_eq!(mit.gesundheit["nachrichten"], NACHRICHTEN);
    assert_eq!(mit.gesundheit["defekte_zeilen"], 0);
    let nachrichten = mit.block["nachrichten"].as_array().expect("nachrichten");
    assert_eq!(nachrichten.len(), 100);
    let zustaende: Vec<&Value> = nachrichten.iter().map(|n| &n["zustand"]).collect();
    assert!(zustaende.contains(&&json!("bearbeitet")), "{zustaende:?}");
    assert!(zustaende.contains(&&json!("geloescht")), "{zustaende:?}");
    for nachricht in nachrichten {
        match nachricht["zustand"].as_str() {
            Some("bearbeitet") => assert!(
                nachricht["text"]
                    .as_str()
                    .is_some_and(|text| text.starts_with("Fassung 3 der Nachricht ")),
                "{nachricht}"
            ),
            Some("geloescht") => assert_eq!(nachricht["text"], "", "{nachricht}"),
            _ => assert!(
                nachricht["text"]
                    .as_str()
                    .is_some_and(|text| text.starts_with("Nachricht ")),
                "{nachricht}"
            ),
        }
    }
    assert!(
        mit.dauern[19] < GRENZE_Q1,
        "Q1: {:?} ≥ {GRENZE_Q1:?}",
        mit.dauern[19]
    );
}

/// R00040 „Messung QE2“, Teil Start: Q6 gilt im Release-Build (Kriterium) und wird nur dort geprüft — der Debug-Build
/// parst Zeiten und Sätze etwa neunmal langsamer (gemessen 2,2 s gegen 0,25 s für denselben Ordner), eine Debug-Grenze
/// wäre erfunden. Derselbe Lauf ohne Änderungssätze steht zum Vergleich in der Ausgabe:
/// `cargo test --release --test leistung_aenderungen -- --nocapture`.
#[tokio::test]
#[cfg_attr(
    debug_assertions,
    ignore = "Q6 gilt im Release-Build: cargo test --release --test leistung_aenderungen"
)]
async fn qe2_viele_aenderungen_bremsen_den_start_nicht() {
    let (ohne, _) = messen(false).await;
    let (mit, _) = messen(true).await;

    assert_eq!(ohne.gesundheit["nachrichten"], NACHRICHTEN);
    assert!(
        mit.ladezeit_ms < GRENZE_Q6_MS,
        "Q6: ladezeit_ms {} ≥ {GRENZE_Q6_MS}",
        mit.ladezeit_ms
    );
    assert!(
        mit.dauern[19] < GRENZE_Q1,
        "Q1: {:?} ≥ {GRENZE_Q1:?}",
        mit.dauern[19]
    );
}
