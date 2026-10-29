//! R00018 F0082 Start, Ende und Sitzungswechsel lückenlos im Protokoll — über das echte Programm.
#![cfg(unix)]

mod gemeinsam;

use gemeinsam::starten_mit;

const LOKAL: [&str; 5] = ["--port", "0", "--host", "127.0.0.1", "--kein-browser"];

/// B0383 (ADR-0042 Ergänzung Karte #26 Punkt 2): eine beim Ende offene Episode schreibt ihre Ende-Zeile nach der
/// ersten `INFORMATION ende` und vor „beendet“ — die Ruhe von 60 s ist nicht abgelaufen.
#[tokio::test]
async fn eine_beim_ende_offene_episode_endet_vor_beendet() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let mut instanz = starten_mit(
        wurzel.path(),
        &LOKAL,
        &[("LOCALCHATA_EPISODE_RUHE_MS", "60000")],
    );
    let antwort = reqwest::Client::new()
        .post(format!("{}api/beitreten", instanz.adresse))
        .header("Origin", "http://fremd.example")
        .json(&serde_json::json!({"name": "beta"}))
        .send()
        .await
        .expect("Antwort");
    assert_eq!(antwort.status().as_u16(), 403);

    let ende = instanz.signal_und_warten("TERM");

    assert_eq!(ende.code(), Some(0));
    let zeilen = gemeinsam::protokoll::alle_zeilen(wurzel.path());
    let stelle = |teil: &str| zeilen.iter().position(|zeile| zeile.contains(teil));
    let beginn = stelle(" WARNING grenzschutz ").expect("Episode beginnt");
    assert!(zeilen[beginn].contains("— Episode beginnt"), "{zeilen:#?}");
    let signal = stelle(" INFORMATION ende Ende-Signal empfangen").expect("erste ende-Zeile");
    assert!(zeilen[signal].contains("grund=SIGTERM"), "{zeilen:#?}");
    let enden: Vec<usize> = zeilen
        .iter()
        .enumerate()
        .filter(|(_, zeile)| {
            zeile.contains(" WARNING grenzschutz ") && zeile.contains("— Episode endet: 1 in ")
        })
        .map(|(nummer, _)| nummer)
        .collect();
    assert_eq!(enden.len(), 1, "{zeilen:#?}");
    let beendet = zeilen.len() - 1;
    assert!(
        zeilen[beendet].contains(" INFORMATION ende Instanz beendet"),
        "„beendet“ ist die letzte Zeile: {zeilen:#?}"
    );
    assert!(signal < enden[0] && enden[0] < beendet, "{zeilen:#?}");
}

/// B0384 (R00014 „Ort“): `logs/` liegt nach dem Start da, auch wenn die Mindeststufe noch nichts schreibt;
/// gegengeprüft: eine falsche Stufe aus der Umgebung ist Exit 2 ohne `logs/`.
#[test]
fn logs_liegt_nach_dem_start_auch_auf_stufe_warning_da() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let mut argumente = LOKAL.to_vec();
    argumente.extend_from_slice(&["--protokoll-stufe", "warning"]);

    let instanz = starten_mit(wurzel.path(), &argumente, &[]);

    assert!(
        wurzel.path().join("logs").is_dir(),
        "nach der Startzeile {}",
        instanz.startzeile
    );

    let falsch = tempfile::tempdir().expect("Wegwerf-Ordner");
    let ordner = falsch.path().display().to_string();
    let mut argumente = vec![ordner.as_str()];
    argumente.extend_from_slice(&LOKAL);
    let ende = gemeinsam::aufrufen_mit(&argumente, &[("LOCALCHATA_PROTOKOLL_STUFE", "laut")]);
    assert_eq!(ende.exit, Some(2), "{}", ende.stderr);
    assert!(!falsch.path().join("logs").exists());
}

/// B0385 (ADR-0043 Ergänzung Karte #26): `--protokoll-stufe debug` übersteuert `LOCALCHATA_PROTOKOLL_STUFE=laut`;
/// ohne die Option bleibt die falsche Umgebung Exit 2 ohne `logs/`.
#[tokio::test]
async fn eine_gueltige_option_uebersteuert_eine_falsche_umgebung() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let mut argumente = LOKAL.to_vec();
    argumente.extend_from_slice(&["--protokoll-stufe", "debug"]);

    let instanz = starten_mit(
        wurzel.path(),
        &argumente,
        &[("LOCALCHATA_PROTOKOLL_STUFE", "laut")],
    );
    let (status, gesundheit) = gemeinsam::gesundheit(&instanz.adresse).await;

    assert_eq!(status, 200);
    assert_eq!(gesundheit["protokoll_stufe"], "debug");
    let start: Vec<String> = gemeinsam::protokoll::alle_zeilen(wurzel.path())
        .into_iter()
        .filter(|zeile| zeile.contains(" INFORMATION start Instanz bereit"))
        .collect();
    assert_eq!(start.len(), 1, "{start:#?}");
    assert!(
        start[0].contains(" stufe=debug option=--protokoll-stufe "),
        "{start:#?}"
    );

    let ohne = tempfile::tempdir().expect("Wegwerf-Ordner");
    let ordner = ohne.path().display().to_string();
    let mut argumente = vec![ordner.as_str()];
    argumente.extend_from_slice(&LOKAL);
    let ende = gemeinsam::aufrufen_mit(&argumente, &[("LOCALCHATA_PROTOKOLL_STUFE", "laut")]);
    assert_eq!(ende.exit, Some(2), "{}", ende.stderr);
    assert!(!ohne.path().join("logs").exists());
}

/// B0386, Abnahme-Probe der engen Wiederholung des Code-Reviews #24 (A8, wörtlich): „Integrationstest
/// online/offline mit LOCALCHATA_PROTOKOLL_STUFE=debug und kleiner LOCALCHATA_GNADENFRIST_MS → je eine „DEBUG
/// sitzung Sitzung online“/„… offline“ mit kennung=; Mutante start/mod.rs:271 mit_veroeffentlichen ohne Protokoll
/// muss rot werden.“ — `beta` tritt bei und öffnet den Ereignisstrom (online), schließt ihn und schweigt (offline).
#[tokio::test]
async fn online_und_offline_sind_je_eine_debug_zeile_mit_kennung() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let instanz = starten_mit(
        wurzel.path(),
        &LOKAL,
        &[
            ("LOCALCHATA_PROTOKOLL_STUFE", "debug"),
            ("LOCALCHATA_GNADENFRIST_MS", "3000"),
        ],
    );
    let beta = gemeinsam::Absender::von("127.0.0.1");
    let token = beta
        .beitreten(&instanz.adresse, serde_json::json!({"name": "beta"}), None)
        .await
        .json["token"]
        .as_str()
        .expect("Token")
        .to_string();
    let mut strom = gemeinsam::strom::Strom::oeffnen(&beta, &instanz.adresse, &token).await;
    strom.bis(std::time::Duration::from_secs(2), |_| true).await;
    drop(strom);

    let sitzung = |text: &str| {
        gemeinsam::protokoll::alle_zeilen(wurzel.path())
            .into_iter()
            .filter(|zeile| zeile.contains(&format!(" DEBUG sitzung {text} | ")))
            .collect::<Vec<_>>()
    };
    let bis = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while sitzung("Sitzung offline").is_empty() && std::time::Instant::now() < bis {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    let online = sitzung("Sitzung online");
    let offline = sitzung("Sitzung offline");
    assert_eq!(online.len(), 1, "{online:#?}");
    assert_eq!(offline.len(), 1, "{offline:#?}");
    for zeile in online.iter().chain(&offline) {
        assert!(zeile.ends_with(" | kennung=beta@127.0.0.1"), "{zeile}");
    }
}
