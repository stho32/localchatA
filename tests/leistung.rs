//! Leistungsziel aus ADR-0006 als Früh-Indikator: 100.000 Nachrichten (≈ 53 MB) in der Frist,
//! Laden beim Start unter einer Sekunde, gemessen über `ladezeit_ms` des echten Programms (auch im
//! Debug-Build: gemessen ≈ 0,3 s für ≈ 59 MB; der Test läuft im Gate, Wegwerf-Ordner ≈ 59 MB).

mod gemeinsam;

use gemeinsam::{gesundheit, starten_mit};
use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::time::Instant;

const CHANNELS: usize = 10;
const TAGE: usize = 7;
const JE_TAG: usize = 100_000 / (CHANNELS * TAGE) + 1;

/// 10 Channels × 7 Tage, Sätze mit 10–90 Wörtern, jeder zwanzigste mit Codeblock (wie die Probe der ADR).
fn bestand_erzeugen(ordner: &Path) -> (usize, u64) {
    let mut saetze = 0;
    let mut bytes = 0;
    for channel in 0..CHANNELS {
        let pfad = ordner.join(format!("channels/c{channel}"));
        fs::create_dir_all(&pfad).expect("Channel");
        for tag in 0..TAGE {
            let mut rohtext = String::new();
            for nummer in 0..JE_TAG {
                let sekunde = nummer * 60 / JE_TAG;
                let _ = writeln!(
                    rohtext,
                    "@ 2026-10-{:02}T10:{sekunde:02}:{:02}.{:03}+01:00 claude@10.0.0.{channel}",
                    21 + tag,
                    nummer % 60,
                    nummer % 1000
                );
                let woerter = 10 + (nummer * 7 + channel) % 81;
                rohtext.push_str("  ");
                for wort in 0..woerter {
                    rohtext.push_str(if wort % 9 == 8 {
                        "Größe\n  "
                    } else {
                        "Nachricht "
                    });
                }
                rohtext.push('\n');
                if nummer % 20 == 0 {
                    for zeile in 0..45 {
                        let _ = writeln!(rohtext, "  let x{zeile} = {zeile};");
                    }
                }
                rohtext.push('\n');
                saetze += 1;
            }
            bytes += rohtext.len() as u64;
            fs::write(pfad.join(format!("2026-10-{:02}.log", 21 + tag)), rohtext).expect("Tag");
        }
    }
    (saetze, bytes)
}

#[tokio::test]
async fn leistung_100000_nachrichten_laden_unter_einer_sekunde() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let (saetze, bytes) = bestand_erzeugen(wurzel.path());

    let beginn = Instant::now();
    let mut instanz = starten_mit(
        wurzel.path(),
        &["--port", "0", "--host", "127.0.0.1", "--kein-browser"],
        &[
            ("LOCALCHATA_JETZT", "2026-10-27T12:00:00+01:00"),
            ("TZ", "Europe/Berlin"),
        ],
    );
    let bis_startzeile = beginn.elapsed();
    let (_, inhalt) = gesundheit(&instanz.adresse).await;
    assert!(instanz.signal_und_warten("TERM").success());

    println!(
        "Leistung: {saetze} Sätze, {bytes} Bytes, ladezeit_ms {}, bis zur Startzeile {bis_startzeile:?}",
        inhalt["ladezeit_ms"]
    );
    assert_eq!(inhalt["nachrichten"], saetze);
    assert_eq!(inhalt["log_bytes"], bytes);
    assert_eq!(inhalt["defekte_zeilen"], 0);
    assert!(saetze >= 100_000);
    assert!(
        inhalt["ladezeit_ms"].as_u64().expect("Zahl") < 1000,
        "{inhalt}"
    );
}
