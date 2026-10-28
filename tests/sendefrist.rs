//! B0256 Sendefrist als Vertrag (R00013 K5, K6, Betrieb; US-3, US-4; ADR-0039 Regel 6, ADR-0017): wer 2 × Herzschlag
//! lang nichts abnimmt, dessen Verbindung schließt die Instanz (Abrissgrenze des Kernels); ein lesender
//! Ereignisstrom bleibt offen. Über das echte Programm, Linux.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::langsam::{nicht_lesend_oeffnen, riesig_anlegen, server_seitig_offen};
use gemeinsam::strom::Strom;
use gemeinsam::{Absender, Instanz, aufrufen, starten_mit};
use serde_json::json;
use std::time::Duration;
use tokio::time::Instant;

/// Herzschlag 2 s → Abrissgrenze (Sendefrist) 4 s.
const HERZSCHLAG_MS: &str = "2000";
/// K5: erwartet ≈ 4,3 s (Grenze 4 s + ≈ 0,3 s Kernel, ADR-0039 Probe 2) plus höchstens 0,1 s Abtastung.
const FRIST_UNTEN: Duration = Duration::from_millis(3600);
const FRIST_OBEN: Duration = Duration::from_millis(5800);
/// Abtastung von `/proc/net/tcp`, höchstens alle 100 ms (K5).
const ABTASTUNG: Duration = Duration::from_millis(50);

fn mit_herzschlag_2_s(ordner: &std::path::Path) -> Instanz {
    starten_mit(
        ordner,
        &["--port", "0", "--host", "127.0.0.1", "--kein-browser"],
        &[("LOCALCHATA_HERZSCHLAG_MS", HERZSCHLAG_MS)],
    )
}

/// Tritt von `herkunft` als `name` bei — der Client fällt danach weg und mit ihm seine Verbindung.
async fn beitreten(instanz: &Instanz, herkunft: &str, name: &str) -> String {
    let absender = Absender::von(herkunft);
    let beitritt = absender
        .beitreten(&instanz.adresse, json!({ "name": name }), None)
        .await;
    assert_eq!(beitritt.status, 201, "{}", beitritt.json);
    beitritt.json["token"].as_str().expect("Token").to_string()
}

/// K5 (US-3): 4 nicht lesende Abrufe `?n=200` aus `riesig` bei Herzschlag 2 s — je Verbindung liegt die Zeit vom
/// vollständigen Senden der Anfrage bis „server-seitig zu“ in [3,6 s; 5,8 s].
#[tokio::test]
async fn k5_nicht_lesende_verbindungen_schliesst_die_instanz_nach_der_sendefrist() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    riesig_anlegen(wurzel.path());
    let instanz = mit_herzschlag_2_s(wurzel.path());
    let token = beitreten(&instanz, "127.0.0.2", "beta").await;
    let port = instanz.port();
    let mut offen = Vec::new();
    for _ in 0..4 {
        let verbindung = nicht_lesend_oeffnen(
            port,
            "127.0.0.2",
            "/api/channels/riesig/nachrichten?n=200",
            &token,
        )
        .await;
        offen.push((verbindung, Instant::now()));
    }
    let gegenstellen: Vec<u16> = offen.iter().map(|(v, _)| v.eigener_port).collect();

    let mut dauern: Vec<Option<Duration>> = vec![None; offen.len()];
    let beobachten_bis = Instant::now() + Duration::from_secs(15);
    while dauern.iter().any(Option::is_none) && Instant::now() < beobachten_bis {
        let noch_offen = server_seitig_offen(port, &gegenstellen);
        let jetzt = Instant::now();
        for (index, (verbindung, gesendet)) in offen.iter().enumerate() {
            if dauern[index].is_none() && !noch_offen.contains(&verbindung.eigener_port) {
                dauern[index] = Some(jetzt - *gesendet);
            }
        }
        tokio::time::sleep(ABTASTUNG).await;
    }

    eprintln!(
        "messung k5_sendefrist_ms={:?}",
        dauern
            .iter()
            .map(|d| d.map(|d| d.as_millis()))
            .collect::<Vec<_>>()
    );
    for dauer in &dauern {
        let dauer = dauer.expect("Verbindung nach 15 s noch server-seitig offen");
        assert!(
            (FRIST_UNTEN..=FRIST_OBEN).contains(&dauer),
            "server-seitig zu nach {dauer:?}, erlaubt [3,6 s; 5,8 s] — alle: {dauern:?}"
        );
    }
    drop(offen);
}

/// K6 (US-4): ein lesender Ereignisstrom bei Herzschlag 2 s ist 12 s (3 × Abrissgrenze) nach dem Antwortkopf
/// noch offen, hat in dieser Zeit mindestens 4 Herzschläge geliefert (erwartet 6) und liefert danach noch eines.
#[tokio::test]
async fn k6_ein_lesender_ereignisstrom_bleibt_ueber_die_sendefrist_offen() {
    let instanz = mit_herzschlag_2_s(tempfile::tempdir().expect("Ordner").path());
    let zwei = Absender::von("127.0.0.2");
    let beitritt = zwei
        .beitreten(&instanz.adresse, json!({ "name": "beta" }), None)
        .await;
    let token = beitritt.json["token"].as_str().expect("Token").to_string();
    let mut strom = Strom::oeffnen(&zwei, &instanz.adresse, &token).await;
    let kopf = Instant::now();

    let geendet = strom.mitlesen(Duration::from_secs(12)).await;
    let herzschlaege = strom
        .ereignisse
        .iter()
        .filter(|ereignis| {
            ereignis.event == "herzschlag" && ereignis.ankunft - kopf <= Duration::from_secs(12)
        })
        .count();
    let danach = strom.bis(Duration::from_secs(5), |_| true).await;

    eprintln!("messung k6_herzschlaege_in_12_s={herzschlaege}");
    assert!(!geendet, "Strom vor 12 s geendet");
    assert!(herzschlaege >= 4, "nur {herzschlaege} Herzschläge in 12 s");
    assert!(danach.is_some(), "nach 12 s kein Ereignis mehr");
}

/// Betrieb: `--hilfe` nennt die Sendefrist in einem Satz.
#[test]
fn die_hilfe_nennt_die_sendefrist() {
    let ende = aufrufen(&["--hilfe"]);

    let text = ende.stdout.split_whitespace().collect::<Vec<_>>().join(" ");

    assert_eq!(ende.exit, Some(0));
    assert!(
        text.contains(
            "Sendefrist: eine Verbindung, die 2 × Herzschlag (Standard 10 s) lang nichts abnimmt oder bestätigt, schließt das System."
        ),
        "{}",
        ende.stdout
    );
}
