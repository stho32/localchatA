//! B0258 Langsame Leser messen (R00013 K4, US-1, US-2; Abnahme der Befundkarte #23): nicht lesende Abrufe aus
//! `riesig` treiben den Speicher der Instanz nicht, und jede dieser Verbindungen ist nach der Sendefrist
//! server-seitig zu. Leistungstest im Release-Build, nicht im Gate:
//! `cargo test --release --test langsame_leser -- --ignored langsame_leser`.
//!
//! Je Lauf eine frische Instanz mit Standard-Herzschlag (Sendefrist 10 s). `r0` und die Threads `t0` nach den
//! Beitritten und einem ersten gelesenen Abruf `?n=1` von `127.0.0.9`; die Uhr startet, wenn alle Anfragen der
//! Rohverbindungen gesendet sind. Die Grenzen stehen in R00013 und werden hier übernommen, nie verschärft.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::langsam::{NichtLesend, nicht_lesend_oeffnen, riesig_anlegen, server_seitig_offen};
use gemeinsam::{Absender, Instanz, starten_mit};
use serde_json::json;
use std::time::Duration;
use tokio::time::Instant;

const MB: u64 = 1_000_000;
/// R00013 K4: 64 × 2 × 256 KiB + 50 MB (ADR-0039 „Rechnung Q8“).
const GRENZE_EINE_HERKUNFT: u64 = 84 * MB;
const GRENZE_ZWEI_HERKUENFTE: u64 = 117 * MB;
/// Nach dem Schließen, 15 s später.
const GRENZE_RUECKGABE: u64 = 50 * MB;
const THREADS_HOECHSTENS_MEHR: u64 = 64;
const ABTASTUNG: Duration = Duration::from_millis(250);
const SPITZE_BIS: Duration = Duration::from_secs(5);
const ZU_BEI: Duration = Duration::from_secs(15);
const GESUNDHEIT_GRENZE: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, Copy)]
struct Lauf {
    name: &'static str,
    abfrage: &'static str,
    herkuenfte: &'static [&'static str],
}

const LAEUFE: [Lauf; 6] = [
    Lauf {
        name: "n=200, 1 Herkunft (Hauptlauf)",
        abfrage: "?n=200",
        herkuenfte: &["127.0.0.1"],
    },
    Lauf {
        name: "n=200, 2 Herkünfte",
        abfrage: "?n=200",
        herkuenfte: &["127.0.0.1", "127.0.0.2"],
    },
    Lauf {
        name: "n=1 (Kontrolle)",
        abfrage: "?n=1",
        herkuenfte: &["127.0.0.1"],
    },
    Lauf {
        name: "n=100",
        abfrage: "?n=100",
        herkuenfte: &["127.0.0.1"],
    },
    Lauf {
        name: "vor 09:00",
        abfrage: "?n=200&vor=2026-10-28T09:00:00.000%2B01:00",
        herkuenfte: &["127.0.0.1"],
    },
    Lauf {
        name: "nach 07:59:59.999",
        abfrage: "?n=200&nach=2026-10-28T07:59:59.999%2B01:00",
        herkuenfte: &["127.0.0.1"],
    },
];

#[derive(Debug)]
struct Ergebnis {
    lauf: Lauf,
    r0: u64,
    spitze: u64,
    threads0: u64,
    threads_spitze: u64,
    zu_ab: Option<Duration>,
    offen_bei_15_s: usize,
    gesundheit: Vec<(u16, Duration)>,
    nach_dem_schliessen: u64,
}

impl Ergebnis {
    fn grenze(&self) -> u64 {
        if self.lauf.herkuenfte.len() == 2 {
            GRENZE_ZWEI_HERKUENFTE
        } else {
            GRENZE_EINE_HERKUNFT
        }
    }

    fn zeile(&self) -> String {
        format!(
            "{:<32} r0 {:>6.1} MB · Zuwachs {:>6.1} MB (≤ {} MB) · Threads {} → {} · server-seitig zu ab {} · offen bei 15 s {} · Gesundheit {:?} · nach dem Schließen {:>+6.1} MB",
            self.lauf.name,
            mb(self.r0),
            mb(self.spitze.saturating_sub(self.r0)),
            self.grenze() / MB,
            self.threads0,
            self.threads_spitze,
            self.zu_ab
                .map_or("—".to_string(), |d| format!("{:.2} s", d.as_secs_f64())),
            self.offen_bei_15_s,
            self.gesundheit
                .iter()
                .map(|(status, dauer)| format!("{status} in {} ms", dauer.as_millis()))
                .collect::<Vec<_>>(),
            (self.nach_dem_schliessen as f64 - self.r0 as f64) / MB as f64,
        )
    }

    fn verstoesse(&self) -> Vec<String> {
        let mut verstoesse = Vec::new();
        let zuwachs = self.spitze.saturating_sub(self.r0);
        if zuwachs > self.grenze() {
            verstoesse.push(format!("RSS-Zuwachs {:.1} MB", mb(zuwachs)));
        }
        if self.threads_spitze > self.threads0 + THREADS_HOECHSTENS_MEHR {
            verstoesse.push(format!("Threads {}", self.threads_spitze));
        }
        if self.offen_bei_15_s > 0 {
            verstoesse.push(format!("{} offen bei t0 + 15 s", self.offen_bei_15_s));
        }
        let gesund = self.gesundheit.len() == 2
            && self
                .gesundheit
                .iter()
                .all(|(status, dauer)| *status == 200 && *dauer <= GESUNDHEIT_GRENZE);
        if !gesund {
            verstoesse.push(format!("Gesundheit {:?}", self.gesundheit));
        }
        if self.nach_dem_schliessen.saturating_sub(self.r0) > GRENZE_RUECKGABE {
            verstoesse.push(format!(
                "nach dem Schließen {:.1} MB",
                mb(self.nach_dem_schliessen.saturating_sub(self.r0))
            ));
        }
        verstoesse
    }
}

fn mb(bytes: u64) -> f64 {
    bytes as f64 / MB as f64
}

/// `VmRSS` (Bytes) und `Threads` aus `/proc/<pid>/status`.
fn speicher_und_threads(instanz: &Instanz) -> (u64, u64) {
    let status =
        std::fs::read_to_string(format!("/proc/{}/status", instanz.kind.id())).expect("status");
    let wert = |schluessel: &str| -> u64 {
        status
            .lines()
            .find_map(|zeile| zeile.strip_prefix(schluessel))
            .and_then(|rest| rest.split_whitespace().next())
            .and_then(|zahl| zahl.parse().ok())
            .unwrap_or_else(|| panic!("{schluessel} in /proc/<pid>/status"))
    };
    (wert("VmRSS:") * 1024, wert("Threads:"))
}

async fn token_von(instanz: &Instanz, herkunft: &str, name: &str) -> String {
    let absender = Absender::von(herkunft);
    let beitritt = absender
        .beitreten(&instanz.adresse, json!({ "name": name }), None)
        .await;
    assert_eq!(beitritt.status, 201, "{}", beitritt.json);
    beitritt.json["token"].as_str().expect("Token").to_string()
}

/// `GET /api/gesundheit` von `127.0.0.9` — Status und Dauer bis zur vollständigen Antwort.
async fn gesundheit_messen(client: reqwest::Client, adresse: String) -> (u16, Duration) {
    let beginn = Instant::now();
    let antwort = tokio::time::timeout(
        Duration::from_secs(10),
        client.get(format!("{adresse}api/gesundheit")).send(),
    )
    .await;
    match antwort {
        Ok(Ok(antwort)) => {
            let status = antwort.status().as_u16();
            let _ = antwort.bytes().await;
            (status, beginn.elapsed())
        }
        _ => (0, beginn.elapsed()),
    }
}

async fn fahren(lauf: Lauf) -> Ergebnis {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    riesig_anlegen(wurzel.path());
    let instanz = starten_mit(
        wurzel.path(),
        &["--port", "0", "--host", "127.0.0.1", "--kein-browser"],
        &[],
    );
    let port = instanz.port();
    let mut tokens = Vec::new();
    for (nummer, herkunft) in lauf.herkuenfte.iter().enumerate() {
        tokens.push(token_von(&instanz, herkunft, &format!("leser{nummer}")).await);
    }
    let dritte = token_von(&instanz, "127.0.0.9", "dritte").await;
    let client = reqwest::Client::builder()
        .local_address("127.0.0.9".parse::<std::net::IpAddr>().expect("IP"))
        .build()
        .expect("Client");
    let erster = client
        .get(format!(
            "{}api/channels/riesig/nachrichten?n=1",
            instanz.adresse
        ))
        .header("X-LocalchatA-Sitzung", &dritte)
        .send()
        .await
        .expect("erster Abruf");
    assert_eq!(erster.status(), 200);
    let _ = erster.bytes().await;
    let (r0, threads0) = speicher_und_threads(&instanz);

    let pfad = format!("/api/channels/riesig/nachrichten{}", lauf.abfrage);
    let mut offen: Vec<NichtLesend> = Vec::new();
    for (herkunft, token) in lauf.herkuenfte.iter().zip(&tokens) {
        for _ in 0..64 {
            offen.push(nicht_lesend_oeffnen(port, herkunft, &pfad, token).await);
        }
    }
    let t0 = Instant::now();
    let gegenstellen: Vec<_> = offen.iter().map(|v| v.eigene).collect();

    let mut gesundheit = Vec::new();
    for nach in [Duration::from_secs(2), Duration::from_secs(5)] {
        let (client, adresse) = (client.clone(), instanz.adresse.clone());
        gesundheit.push(tokio::spawn(async move {
            tokio::time::sleep_until(t0 + nach).await;
            gesundheit_messen(client, adresse).await
        }));
    }
    let (mut spitze, mut threads_spitze, mut zu_ab) = (r0, threads0, None);
    let mut abtastung = tokio::time::interval_at(t0, ABTASTUNG);
    loop {
        abtastung.tick().await;
        let seit = t0.elapsed();
        if seit <= SPITZE_BIS {
            let (rss, threads) = speicher_und_threads(&instanz);
            spitze = spitze.max(rss);
            threads_spitze = threads_spitze.max(threads);
        }
        if zu_ab.is_none() && server_seitig_offen(port, &gegenstellen).is_empty() {
            zu_ab = Some(seit);
        }
        if seit >= ZU_BEI {
            break;
        }
    }
    let offen_bei_15_s = server_seitig_offen(port, &gegenstellen).len();
    let mut gemessen = Vec::new();
    for messung in gesundheit {
        gemessen.push(messung.await.expect("Gesundheit"));
    }
    drop(offen);
    tokio::time::sleep(Duration::from_secs(15)).await;
    let (nach_dem_schliessen, _) = speicher_und_threads(&instanz);
    Ergebnis {
        lauf,
        r0,
        spitze,
        threads0,
        threads_spitze,
        zu_ab,
        offen_bei_15_s,
        gesundheit: gemessen,
        nach_dem_schliessen,
    }
}

/// Reihenfolge der Läufe zufällig (gemeinsames Gelerntes: Timing-Proben nur mit zufälliger Reihenfolge und
/// Kontrollpaar); die Reihenfolge steht in der Ausgabe.
fn gemischt() -> Vec<Lauf> {
    let mut saat = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(1, |d| d.as_nanos() as u64)
        | 1;
    let mut laeufe = LAEUFE.to_vec();
    for index in (1..laeufe.len()).rev() {
        saat ^= saat << 13;
        saat ^= saat >> 7;
        saat ^= saat << 17;
        laeufe.swap(index, (saat % (index as u64 + 1)) as usize);
    }
    laeufe
}

/// K4 — Abnahme der Befundkarte #23: bei 64 nicht lesenden Verbindungen einer Herkunft (`?n=200` aus `riesig`)
/// ist der RSS-Zuwachs ≤ 84 MB (2 Herkünfte: ≤ 117 MB) **und** jede dieser Verbindungen ist bei `t0 + 15 s`
/// server-seitig zu; Threads ≤ `t0` + 64, Gesundheit von `127.0.0.9` 200 innerhalb 1 s bei `t0 + 2 s` und
/// `t0 + 5 s`, 15 s nach dem Schließen ≤ 50 MB über `r0`. Alle Läufe, dann ein Urteil.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "Leistungstest im Release-Build: cargo test --release --test langsame_leser -- --ignored langsame_leser"]
async fn langsame_leser_treiben_den_speicher_nicht_und_sind_nach_der_sendefrist_zu() {
    let mut ergebnisse = Vec::new();
    for lauf in gemischt() {
        let ergebnis = fahren(lauf).await;
        eprintln!("messung k4 {}", ergebnis.zeile());
        ergebnisse.push(ergebnis);
    }
    let verstoesse: Vec<String> = ergebnisse
        .iter()
        .flat_map(|ergebnis| {
            ergebnis
                .verstoesse()
                .into_iter()
                .map(move |verstoss| format!("{}: {verstoss}", ergebnis.lauf.name))
        })
        .collect();
    assert!(verstoesse.is_empty(), "{verstoesse:#?}");
}
