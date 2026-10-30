//! R00043 F0144 (ADR-0048 K2): das Löschen überlebt den Abbruch des Clients — das echte Programm mit
//! `--protokoll-stufe debug`: 40-mal wird eine Nachricht mit Anhang angelegt, der Löschauftrag gesendet und die
//! Verbindung sofort zurückgesetzt (RST, Antwort nie gelesen). Nach 2 s stehen alle Zeilen, alle Ereignisse und die
//! Kennzahl; der Messwert der Reviewer auf Karte #67 vorher: 29 von 40.

mod gemeinsam;

use gemeinsam::anhaenge::{ablegen, gesundheit};
use gemeinsam::herkunft::verbinden;
use gemeinsam::protokoll::zeilen;
use gemeinsam::senden::{HEUTE, beitreten, ordner, roh_senden, starten, verlauf};
use gemeinsam::strom::Strom;
use gemeinsam::{Absender, Instanz};
use serde_json::json;
use std::time::Duration;
use tokio::io::AsyncWriteExt;

const ANZAHL: usize = 40;
/// So viele Agenten mit eigener Herkunft teilen sich die 40 Löschungen (das Schreibkonto gilt je Kennung).
const AGENTEN: usize = 8;

struct Agent {
    absender: Absender,
    herkunft: String,
    token: String,
}

async fn agenten(instanz: &Instanz) -> Vec<Agent> {
    let mut alle = Vec::new();
    for nummer in 0..AGENTEN {
        let herkunft = format!("127.0.0.{}", nummer + 1);
        let absender = Absender::von(&herkunft);
        let token = beitreten(
            &absender,
            &instanz.adresse,
            &format!("agent{nummer}"),
            Some("agent"),
        )
        .await;
        alle.push(Agent {
            absender,
            herkunft,
            token,
        });
    }
    alle
}

/// Sendet eine Nachricht mit einem Anhang; liefert ihre Zeit.
async fn mit_anhang_senden(instanz: &Instanz, agent: &Agent, nummer: usize) -> String {
    let id = ablegen(
        &agent.absender,
        &instanz.adresse,
        &agent.token,
        &format!("bericht{nummer}.bin"),
        vec![7; 1_000],
    )
    .await;
    let antwort = roh_senden(
        &agent.absender,
        &instanz.adresse,
        Some(&agent.token),
        "allgemein",
        "application/json",
        json!({"text": format!("Bericht {nummer}"), "anhaenge": [id]})
            .to_string()
            .into_bytes(),
    )
    .await;
    assert_eq!(antwort.status, 201, "{}", antwort.json);
    antwort.json["nachricht"]["zeit"]
        .as_str()
        .expect("zeit")
        .to_string()
}

/// Sendet die Löschanfrage roh und setzt die Verbindung sofort zurück (`SO_LINGER` 0), ohne die Antwort zu lesen.
async fn loeschen_und_zuruecksetzen(
    instanz: &Instanz,
    agent: &Agent,
    zeit: &str,
    warten: Duration,
) {
    let koerper = json!({ "zeit": zeit }).to_string();
    let anfrage = format!(
        "POST /api/channels/allgemein/nachrichten/loeschen HTTP/1.1\r\nHost: 127.0.0.1\r\n\
         X-LocalchatA-Sitzung: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{koerper}",
        agent.token,
        koerper.len()
    );
    let mut verbindung = verbinden(instanz.port(), &agent.herkunft).await;
    verbindung
        .write_all(anfrage.as_bytes())
        .await
        .expect("Anfrage gesendet");
    tokio::time::sleep(warten).await;
    rustix::net::sockopt::set_socket_linger(&verbindung, Some(Duration::ZERO))
        .expect("SO_LINGER 0");
    drop(verbindung);
}

#[tokio::test(flavor = "multi_thread")]
async fn vierzig_abgebrochene_loeschungen_hinterlassen_alle_zeilen_ereignisse_und_die_kennzahl() {
    let wurzel = ordner();
    let mut instanz = starten(wurzel.path(), &[("LOCALCHATA_PROTOKOLL_STUFE", "debug")]);
    let agenten = agenten(&instanz).await;
    let mut zeiten = Vec::new();
    for nummer in 0..ANZAHL {
        let agent = &agenten[nummer % AGENTEN];
        zeiten.push(mit_anhang_senden(&instanz, agent, nummer).await);
    }
    let beobachter = Absender::von("127.0.0.9");
    let token = beitreten(&beobachter, &instanz.adresse, "beobachter", Some("agent")).await;
    let mut strom = Strom::oeffnen(&beobachter, &instanz.adresse, &token).await;

    for (nummer, zeit) in zeiten.iter().enumerate() {
        let warten = Duration::from_micros(500 * (nummer % 8) as u64);
        loeschen_und_zuruecksetzen(&instanz, &agenten[nummer % AGENTEN], zeit, warten).await;
    }
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Ein RST, der vor dem ersten Lesen der Anfrage eintrifft, löscht nichts — das ist kein Verlust. Ein Löschsatz
    // im Verlauf aber heißt: Protokoll, Ereignis und Kennzahl müssen da sein, gleich wann der Client abbrach.
    let verlauf = verlauf(&beobachter, &instanz.adresse, &token, "allgemein").await;
    let gestartet = verlauf["nachrichten"]
        .as_array()
        .expect("nachrichten")
        .iter()
        .filter(|nachricht| nachricht["zustand"] == "geloescht")
        .count();
    assert!(
        gestartet >= ANZAHL / 2,
        "die Probe hat Zähne nur, wenn die meisten Anfragen gelesen wurden: {gestartet}"
    );
    let protokoll = zeilen(wurzel.path(), HEUTE);
    let zahl = |teil: &str| protokoll.iter().filter(|z| z.contains(teil)).count();
    assert_eq!(zahl(" DEBUG aendern Nachricht gelöscht"), gestartet);
    assert_eq!(zahl(" INFORMATION anhang Anhänge gelöscht"), gestartet);
    strom.mitlesen(Duration::from_millis(300)).await;
    let ereignisse = strom
        .ereignisse
        .iter()
        .filter(|e| e.data["art"] == "anhaenge")
        .count();
    assert_eq!(ereignisse, gestartet);
    let kennzahlen = gesundheit(&instanz.adresse).await;
    assert_eq!(kennzahlen["anhaenge_geloescht"], gestartet);
    assert!(instanz.signal_und_warten("TERM").success());
}
