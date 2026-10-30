//! R00047 F0157, Nacharbeit B1 (Security-Review #79, ADR-0053 Regel 3): ein Original aus Leerraum darf die Auflösung
//! der Zitate nicht verlangsamen. Abnahme-Probe wörtlich: (a) Release, Original "x" + 15.000 × "\n" + "y" (und
//! die " \n"-Variante, und eine bearbeitete Fassung), 200 Antworten, GET n=200 höchstens das Dreifache der Kontrolle
//! und unter 5 ms, Kontrollpaar (gleicher Bau, Original "x y") und zufällige Reihenfolge; (b) Semantik "a" +
//! 100 × "\n" → "a" ohne Panik (Unit `gespraeche::textanfang`, im Gate); (c) 20 parallele Leser: n=20 und
//! Senden unter 100 ms; (d) eine Mutante ohne Obergrenze (`LESEGRENZE_ZEICHEN` unendlich) macht (a) rot.
//! Messung nur im Release-Build: `cargo test --release --test leistung_zitat_leerraum -- --nocapture`.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::{Absender, Instanz, starten_mit};
use serde_json::{Value, json};
use std::fmt::Write as _;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

const JETZT: &str = "2026-10-28T15:00:00.000+01:00";
const ORIGINAL: &str = "claude@10.0.0.5";
const ANTWORTENDER: &str = "codex@10.0.0.6";
const T0: &str = "2026-10-28T10:00:00.000+01:00";
const ANTWORTEN: usize = 200;
const RUNDEN: usize = 30;
/// Abnahme (a): höchstens das Dreifache der Kontrolle und unter 5 ms.
const FAKTOR: u32 = 3;
const GRENZE_GET: Duration = Duration::from_millis(5);
/// Abnahme (c): Senden unter 100 ms.
const GRENZE_SENDEN: Duration = Duration::from_millis(100);

/// Ein Leerraum-Original: erste und letzte Zeile sichtbar, dazwischen Leerraumzeilen bis etwa 15 KiB Satz.
#[derive(Clone, Copy)]
struct Fuellung {
    channel: &'static str,
    zeile: &'static str,
    anzahl: usize,
    bearbeitet: bool,
}

const VARIANTEN: [Fuellung; 3] = [
    Fuellung {
        channel: "nl",
        zeile: "",
        anzahl: 15_000,
        bearbeitet: false,
    },
    Fuellung {
        channel: "sp",
        zeile: " ",
        anzahl: 3_900,
        bearbeitet: false,
    },
    Fuellung {
        channel: "ed",
        zeile: "",
        anzahl: 15_000,
        bearbeitet: true,
    },
];

fn text(fuellung: Option<Fuellung>) -> String {
    let Some(fuellung) = fuellung else {
        return "  x y\n".to_string();
    };
    let mut text = String::from("  x\n");
    for _ in 0..fuellung.anzahl {
        if !fuellung.zeile.is_empty() {
            text.push_str("  ");
            text.push_str(fuellung.zeile);
        }
        text.push('\n');
    }
    text.push_str("  y\n");
    text
}

/// Ein Channel mit einem Original (bei `bearbeitet` mit Änderungssatz) und `antworten` Antworten darauf; `leer`
/// bestimmt, ob der Text Leerraum ist oder die Kontrolle "x y".
fn channel_anlegen(ordner: &Path, name: &str, fuellung: Fuellung, leer: bool, antworten: usize) {
    let pfad = ordner.join("channels").join(name);
    std::fs::create_dir_all(&pfad).expect("Channel");
    let text = text(leer.then_some(fuellung));
    let mut log = String::new();
    if fuellung.bearbeitet {
        let _ = write!(
            log,
            "@ {T0} {ORIGINAL}\n+ art: agent\n  Erste Fassung\n\n@ 2026-10-28T10:00:30.000+01:00 {ORIGINAL}\n+ bearbeitet: {T0}\n{text}\n"
        );
    } else {
        let _ = write!(log, "@ {T0} {ORIGINAL}\n+ art: agent\n{text}\n");
    }
    for nummer in 0..antworten {
        let _ = write!(
            log,
            "@ 2026-10-28T11:{:02}:{:02}.000+01:00 {ANTWORTENDER}\n+ art: agent\n+ antwort_auf: {T0} {ORIGINAL}\n  Antwort\n\n",
            nummer / 60,
            nummer % 60
        );
    }
    std::fs::write(pfad.join("2026-10-28.log"), log).expect("Tag");
}

fn bestand_anlegen(ordner: &Path, antworten: usize) {
    for fuellung in VARIANTEN {
        channel_anlegen(ordner, fuellung.channel, fuellung, true, antworten);
        let kontrolle = format!("k{}", fuellung.channel);
        channel_anlegen(ordner, &kontrolle, fuellung, false, antworten);
    }
}

async fn instanz_mit_bestand(
    antworten: usize,
) -> (tempfile::TempDir, Instanz, Arc<Absender>, String) {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    bestand_anlegen(wurzel.path(), antworten);
    let instanz = starten_mit(
        wurzel.path(),
        &["--port", "0", "--host", "127.0.0.1", "--kein-browser"],
        &[("LOCALCHATA_JETZT", JETZT), ("TZ", "Europe/Berlin")],
    );
    let beta = Arc::new(Absender::von("127.0.0.3"));
    let beitritt = beta
        .beitreten(&instanz.adresse, json!({ "name": "beta" }), None)
        .await;
    let token = beitritt.json["token"].as_str().expect("Token").to_string();
    (wurzel, instanz, beta, token)
}

/// `GET …/nachrichten?n=<n>`: der Block und die Dauer bis zum vollständig gelesenen Inhalt.
async fn lesen(
    beta: &Absender,
    basis: &str,
    token: &str,
    channel: &str,
    n: usize,
) -> (Value, Duration) {
    let beginn = Instant::now();
    let antwort = beta
        .anfrage(
            reqwest::Method::GET,
            format!("{basis}api/channels/{channel}/nachrichten?n={n}"),
        )
        .header("X-LocalchatA-Sitzung", token)
        .send()
        .await
        .expect("Antwort");
    let status = antwort.status();
    let inhalt = antwort.text().await.expect("Inhalt");
    let dauer = beginn.elapsed();
    assert_eq!(status, 200, "{inhalt}");
    (serde_json::from_str(&inhalt).expect("JSON"), dauer)
}

fn median(dauern: &mut [Duration]) -> Duration {
    dauern.sort();
    dauern[dauern.len() / 2]
}

/// Ein fester Zufall für die Reihenfolge der Paare.
struct Zufall(u64);

impl Zufall {
    fn muenze(&mut self) -> bool {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 40) & 1 == 1
    }
}

/// Abnahme (a): je Variante Median aus `RUNDEN` Lesungen des Leerraum-Channels gegen seine Kontrolle; je Runde
/// entscheidet der Zufall, wer zuerst liest.
#[tokio::test]
#[cfg_attr(
    debug_assertions,
    ignore = "Abnahme (a) gilt im Release-Build: cargo test --release --test leistung_zitat_leerraum"
)]
async fn b1_leerraum_originale_bremsen_das_lesen_nicht() {
    let (_wurzel, instanz, beta, token) = instanz_mit_bestand(ANTWORTEN).await;
    let mut zufall = Zufall(4711);
    for fuellung in VARIANTEN {
        let kontrolle = format!("k{}", fuellung.channel);
        let (mut leer, mut kontr) = (Vec::new(), Vec::new());
        for runde in 0..RUNDEN + 3 {
            let leer_zuerst = zufall.muenze();
            let mut dauern = [Duration::ZERO; 2];
            let mut bloecke = [Value::Null, Value::Null];
            for schritt in 0..2 {
                let leer_dran = (schritt == 0) == leer_zuerst;
                let channel = if leer_dran {
                    fuellung.channel
                } else {
                    &kontrolle
                };
                let (block, dauer) =
                    lesen(&beta, &instanz.adresse, &token, channel, ANTWORTEN).await;
                dauern[usize::from(!leer_dran)] = dauer;
                bloecke[usize::from(!leer_dran)] = block;
            }
            assert_eq!(
                bloecke[0]["nachrichten"].as_array().map(Vec::len),
                Some(ANTWORTEN)
            );
            let zitat = &bloecke[0]["nachrichten"][0]["antwort_auf"];
            let erwartet = if fuellung.bearbeitet {
                "bearbeitet"
            } else {
                "da"
            };
            assert_eq!(zitat["zustand"], erwartet, "{}", fuellung.channel);
            assert!(
                zitat["textanfang"]
                    .as_str()
                    .is_some_and(|t| t.starts_with('x')),
                "{zitat}"
            );
            if runde >= 3 {
                leer.push(dauern[0]);
                kontr.push(dauern[1]);
            }
        }
        let (leer, kontr) = (median(&mut leer), median(&mut kontr));
        println!(
            "B1 {} {}: GET n={ANTWORTEN} Leerraum {leer:?} Kontrolle {kontr:?}",
            if cfg!(debug_assertions) {
                "Debug"
            } else {
                "Release"
            },
            fuellung.channel
        );
        assert!(
            leer < GRENZE_GET,
            "{}: {leer:?} ≥ {GRENZE_GET:?}",
            fuellung.channel
        );
        assert!(
            leer <= kontr * FAKTOR,
            "{}: {leer:?} > {FAKTOR} × Kontrolle {kontr:?}",
            fuellung.channel
        );
    }
}

/// Abnahme (c): 20 Leser holen gleichzeitig n=20 aus dem Leerraum-Channel; währenddessen sendet ein Teilnehmer mit
/// Bezug auf das Original — jedes Senden unter 100 ms (es löst das Zitat unter derselben Gesprächssperre auf).
#[tokio::test]
#[cfg_attr(
    debug_assertions,
    ignore = "Abnahme (c) gilt im Release-Build: cargo test --release --test leistung_zitat_leerraum"
)]
async fn b1_zwanzig_leser_halten_das_senden_nicht_auf() {
    let (_wurzel, instanz, beta, token) = instanz_mit_bestand(2_000).await;
    let leser: Vec<_> = (0..20)
        .map(|_| {
            let (beta, basis, token) = (Arc::clone(&beta), instanz.adresse.clone(), token.clone());
            tokio::spawn(async move {
                for _ in 0..40 {
                    let (block, _) = lesen(&beta, &basis, &token, "nl", 20).await;
                    assert_eq!(block["nachrichten"].as_array().map(Vec::len), Some(20));
                }
            })
        })
        .collect();
    tokio::time::sleep(Duration::from_millis(20)).await;
    let mut laengste = Duration::ZERO;
    for nummer in 0..10 {
        let beginn = Instant::now();
        let antwort = gemeinsam::senden::roh_senden(
            &beta,
            &instanz.adresse,
            Some(&token),
            "nl",
            "application/json",
            json!({ "text": format!("Neu {nummer}"), "antwort_auf": T0 })
                .to_string()
                .into_bytes(),
        )
        .await;
        laengste = laengste.max(beginn.elapsed());
        assert_eq!(antwort.status, 201, "{}", antwort.json);
        assert!(
            antwort.json["nachricht"]["antwort_auf"]["textanfang"]
                .as_str()
                .is_some_and(|t| t.starts_with('x'))
        );
    }
    for leser in leser {
        leser.await.expect("Leser");
    }
    println!("B1 (c): längstes Senden {laengste:?}");
    assert!(
        laengste < GRENZE_SENDEN,
        "Senden {laengste:?} ≥ {GRENZE_SENDEN:?}"
    );
}
