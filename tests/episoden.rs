//! F0055 Grenzen, Episoden und Tagesbudget — über das echte Programm (R00014, US-13 bis US-15 und Kriterien).

mod gemeinsam;

use gemeinsam::protokoll::{mit, wert, zeilen};
use gemeinsam::{Absender, Instanz, aufrufen_mit, gesundheit, roh_anfragen, starten_mit};
use serde_json::json;
use std::io::{Read, Write};
use std::path::Path;
use std::time::{Duration, Instant};

#[test]
fn ruhe_null_ist_exit_2_und_nennt_die_variable() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let ordner = wurzel.path().display().to_string();

    let ende = aufrufen_mit(
        &[
            &ordner,
            "--port",
            "0",
            "--host",
            "127.0.0.1",
            "--kein-browser",
        ],
        &[("LOCALCHATA_EPISODE_RUHE_MS", "0")],
    );

    assert_eq!(ende.exit, Some(2), "{}", ende.stderr);
    assert!(
        ende.stderr.contains("LOCALCHATA_EPISODE_RUHE_MS"),
        "{}",
        ende.stderr
    );
    assert!(!wurzel.path().join("logs").exists());
}

const LOKAL: [&str; 5] = ["--port", "0", "--host", "127.0.0.1", "--kein-browser"];

fn starten_um(ordner: &Path, weitere: &[&str], umgebung: &[(&str, &str)]) -> Instanz {
    let mut argumente = LOKAL.to_vec();
    argumente.extend_from_slice(weitere);
    let mut alle = vec![
        ("LOCALCHATA_JETZT", "2026-10-28T12:00:00+01:00"),
        ("TZ", "Europe/Berlin"),
    ];
    alle.extend_from_slice(umgebung);
    starten_mit(ordner, &argumente, &alle)
}

fn alle_protokolle(ordner: &Path) -> Vec<u8> {
    let mut alles = Vec::new();
    for eintrag in std::fs::read_dir(ordner.join("logs")).expect("logs/") {
        alles.extend(std::fs::read(eintrag.expect("Eintrag").path()).expect("lesbar"));
    }
    alles
}

fn enthaelt(heuhaufen: &[u8], nadel: &[u8]) -> bool {
    heuhaufen
        .windows(nadel.len())
        .any(|fenster| fenster == nadel)
}

/// Wartet höchstens 10 s, bis `bedingung` auf den Zeilen von heute gilt; Abfrage alle 100 ms.
async fn abwarten(ordner: &Path, bedingung: impl Fn(&[String]) -> bool) -> Vec<String> {
    let ende = Instant::now() + Duration::from_secs(10);
    loop {
        let datei = zeilen(ordner, "2026-10-28");
        if bedingung(&datei) || Instant::now() > ende {
            return datei;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// Gegenprobe des observability-reviewer (ADR-0042, Ergänzung 8), wörtlich: 200 × 5 KB mit Marker von 127.0.0.1,
/// vorher eine echte Sitzung; Ruhe 1 s.
async fn gegenprobe(stufe: &[&str]) {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let mut instanz = starten_um(
        wurzel.path(),
        stufe,
        &[("LOCALCHATA_EPISODE_RUHE_MS", "1000")],
    );
    let token = Absender::von("127.0.0.1")
        .beitreten(&instanz.adresse, json!({"name": "beta"}), None)
        .await
        .json["token"]
        .as_str()
        .expect("Token")
        .to_string();
    let koerper = format!(
        "{{\"name\":\"KOERPER-MARKER-7f3a\",\"rest\":\"{}\"}}",
        "x".repeat(5_120 - 42)
    );
    let client = reqwest::Client::new();
    for _ in 0..200 {
        let antwort = client
            .post(format!("{}api/beitreten", instanz.adresse))
            .header("Content-Type", "application/json")
            .body(koerper.clone())
            .send()
            .await
            .expect("Antwort");
        assert_eq!(antwort.status(), 413);
    }
    let quelle = |zeile: &String| {
        zeile.contains(" WARNING ")
            && wert(zeile, "route").as_deref() == Some("POST /api/beitreten")
            && wert(zeile, "herkunft").as_deref() == Some("127.0.0.1")
    };
    tokio::time::sleep(Duration::from_secs(2)).await;
    let datei = abwarten(wurzel.path(), |d| {
        d.iter().any(|z| quelle(z) && z.contains("Episode endet"))
    })
    .await;
    let (_, kennzahlen) = gesundheit(&instanz.adresse).await;
    assert!(instanz.signal_und_warten("TERM").success());

    let warnungen: Vec<&String> = datei.iter().filter(|z| quelle(z)).collect();
    assert_eq!(warnungen.len(), 2, "{datei:#?}");
    assert!(warnungen[0].contains("— Episode beginnt"), "{warnungen:#?}");
    assert!(
        warnungen[1].contains("— Episode endet: 200 "),
        "{warnungen:#?}"
    );
    assert!(
        warnungen[1].contains("(zu groß 200; verworfen vollständig 200)"),
        "{warnungen:#?}"
    );
    assert!(
        warnungen[1].contains("verworfen vollständig 200"),
        "{warnungen:#?}"
    );
    assert_eq!(kennzahlen["abgewiesen"]["zu_gross"], 200, "{kennzahlen}");
    let alles = alle_protokolle(wurzel.path());
    assert!(!enthaelt(&alles, b"KOERPER-MARKER-7f3a"));
    assert!(!enthaelt(&alles, token.as_bytes()));
}

#[tokio::test]
async fn us13_gegenprobe_200_zu_grosse_koerper_ergeben_zwei_zeilen() {
    gegenprobe(&[]).await;
}

#[tokio::test]
async fn us13_gegenprobe_auf_warning_enthaelt_dieselben_zwei_zeilen() {
    gegenprobe(&["--protokoll-stufe", "warning"]).await;
}

#[tokio::test]
async fn us15_grenzschutz_flut_ist_eine_episode() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let instanz = starten_um(wurzel.path(), &[], &[]);
    let ziel = format!("127.0.0.1:{}", instanz.port())
        .parse()
        .expect("Ziel");

    for _ in 0..50 {
        let antwort = roh_anfragen(ziel, "GET / HTTP/1.1\r\nHost: boese.example", "");
        assert_eq!(antwort.status, 403);
    }

    let datei = zeilen(wurzel.path(), "2026-10-28");
    let grenzschutz = mit(&datei, "WARNING", "grenzschutz");
    assert_eq!(grenzschutz.len(), 1, "{datei:#?}");
    assert!(
        grenzschutz[0].contains("— Episode beginnt"),
        "{}",
        grenzschutz[0]
    );
    let grund = wert(&grenzschutz[0], "grund").expect("grund");
    assert!(
        grund.contains("boese.example") && grund.chars().count() <= 256,
        "{grund}"
    );
    assert_eq!(
        wert(&grenzschutz[0], "herkunft").as_deref(),
        Some("127.0.0.1")
    );
}

#[tokio::test]
async fn us2_ein_richtungszeichen_im_host_faelscht_nichts() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let instanz = starten_um(wurzel.path(), &[], &[]);
    let mut verbindung =
        std::net::TcpStream::connect(("127.0.0.1", instanz.port())).expect("Verbindung");
    verbindung
        .write_all(
            "GET / HTTP/1.1\r\nHost: gut\u{202e}elif.example\r\nConnection: close\r\n\r\n"
                .as_bytes(),
        )
        .expect("gesendet");
    let mut antwort = String::new();
    let _ = verbindung.read_to_string(&mut antwort);

    assert!(antwort.starts_with("HTTP/1.1 403"), "{antwort}");
    let roh = alle_protokolle(wurzel.path());
    assert!(!enthaelt(&roh, "\u{202e}".as_bytes()));
    let datei = zeilen(wurzel.path(), "2026-10-28");
    let grenzschutz = mit(&datei, "WARNING", "grenzschutz");
    assert_eq!(grenzschutz.len(), 1, "{datei:#?}");
    assert!(
        grenzschutz[0].contains("gut\\u{202e}elif.example"),
        "{}",
        grenzschutz[0]
    );
}

#[tokio::test]
async fn us14_wer_an_die_verbindungsgrenze_stoesst_wird_einmal_gemeldet() {
    let grenze = localchata::start::verbindungen::JE_GEGENSTELLE;
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let mut instanz = starten_um(
        wurzel.path(),
        &[],
        &[("LOCALCHATA_EPISODE_RUHE_MS", "1000")],
    );
    let ziel = ("127.0.0.1", instanz.port());

    let mut offen = Vec::new();
    for _ in 0..grenze {
        offen.push(std::net::TcpStream::connect(ziel).expect("Verbindung"));
    }
    for _ in 0..31 {
        let abgewiesen = std::net::TcpStream::connect(ziel).expect("Verbindung");
        let mut puffer = [0u8; 1];
        abgewiesen
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("Lesegrenze");
        let _ = (&abgewiesen).read(&mut puffer);
    }
    let datei = abwarten(wurzel.path(), |d| {
        !mit(d, "WARNING", "verbindung").is_empty()
    })
    .await;
    let warnungen = mit(&datei, "WARNING", "verbindung");
    assert_eq!(warnungen.len(), 1, "{warnungen:#?}");
    assert!(
        warnungen[0].contains("— Episode beginnt"),
        "{}",
        warnungen[0]
    );
    assert_eq!(
        wert(&warnungen[0], "herkunft").as_deref(),
        Some("127.0.0.1")
    );
    assert_eq!(wert(&warnungen[0], "grenze").as_deref(), Some("herkunft"));
    assert_eq!(wert(&warnungen[0], "hoechstens"), Some(grenze.to_string()));

    drop(offen);
    let datei = abwarten(wurzel.path(), |d| {
        mit(d, "WARNING", "verbindung")
            .iter()
            .any(|z| z.contains("Episode endet"))
    })
    .await;
    assert!(instanz.signal_und_warten("TERM").success());
    let warnungen = mit(&datei, "WARNING", "verbindung");
    assert_eq!(warnungen.len(), 2, "{warnungen:#?}");
    assert!(
        warnungen[1].contains("— Episode endet: 31 in "),
        "{}",
        warnungen[1]
    );
    let stderr = instanz.stderr_bis_zum_ende();
    assert_eq!(
        stderr
            .lines()
            .filter(|z| z.contains(" WARNING verbindung ") && z.contains("Episode beginnt"))
            .count(),
        1,
        "{stderr}"
    );
}
