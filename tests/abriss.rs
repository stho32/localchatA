//! F0016 Stiller Abriss wird erkannt — die Glieder, die sich ohne Paketverlust prüfen lassen, über das echte
//! Programm (R00005, US-1, US-2; ADR-0017).
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::strom::{Strom, ist_wechsel};
use gemeinsam::{Absender, Instanz, wegwerf_starten};
use serde_json::json;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpSocket, TcpStream};
use tokio::time::Instant;

async fn beitreten(absender: &Absender, instanz: &Instanz, name: &str) -> String {
    let antwort = absender
        .beitreten(&instanz.adresse, json!({ "name": name }), None)
        .await;
    antwort.json["token"].as_str().expect("Token").to_string()
}

/// Ein Strom über einen rohen Socket von `absender` — wie das Python-Skript der User Story; liefert den
/// Socket, nachdem der Kopf der Antwort gelesen ist.
async fn roher_strom(instanz: &Instanz, absender: &str, token: &str) -> TcpStream {
    let socket = TcpSocket::new_v4().expect("Socket");
    socket
        .bind(format!("{absender}:0").parse().expect("Absender"))
        .expect("gebunden");
    let ziel = format!("127.0.0.1:{}", instanz.port())
        .parse()
        .expect("Ziel");
    let mut strom = socket.connect(ziel).await.expect("verbunden");
    let anfrage = format!(
        "GET /api/ereignisse HTTP/1.1\r\nHost: 127.0.0.1\r\nX-LocalchatA-Sitzung: {token}\r\n\r\n"
    );
    strom.write_all(anfrage.as_bytes()).await.expect("gesendet");
    let mut puffer = vec![0u8; 65536];
    let gelesen = strom.read(&mut puffer).await.expect("Antwort");
    let kopf = String::from_utf8_lossy(&puffer[..gelesen]);
    assert!(kopf.starts_with("HTTP/1.1 200"), "{kopf}");
    strom
}

/// US-2 (Kriterium „Harter Abbruch“): `alpha` bricht seinen Strom hart ab (`SO_LINGER 0`, RST) → `beta` liest
/// `alpha` `offline` spätestens 2 s nach der Rückkehr von `close` (erwartet ≤ 1,25 s: Nachlauf 1 s plus
/// höchstens ein Takt 0,25 s).
#[tokio::test]
async fn us2_ein_hart_abgebrochener_strom_steht_binnen_2_s_nach_close_offline() {
    let instanz = wegwerf_starten();
    let (zwei, drei) = (Absender::von("127.0.0.2"), Absender::von("127.0.0.3"));
    let beta = beitreten(&drei, &instanz, "beta").await;
    let mut strom_beta = Strom::oeffnen(&drei, &instanz.adresse, &beta).await;
    let alpha = beitreten(&zwei, &instanz, "alpha").await;
    let roh = roher_strom(&instanz, "127.0.0.2", &alpha).await;
    let online = ist_wechsel("alpha@127.0.0.2", "online");
    strom_beta
        .bis(Duration::from_secs(5), online)
        .await
        .expect("beta hat alpha online gelesen");

    rustix::net::sockopt::set_socket_linger(&roh, Some(Duration::ZERO)).expect("SO_LINGER 0");
    drop(roh);
    let close_zurueck = Instant::now();
    let offline = strom_beta
        .bis(
            Duration::from_secs(10),
            ist_wechsel("alpha@127.0.0.2", "offline"),
        )
        .await
        .expect("beta liest alpha offline");

    let dauer = offline.ankunft - close_zurueck;
    eprintln!("messung us2_rst_offline_ms={}", dauer.as_millis());
    assert!(dauer <= Duration::from_secs(2), "{dauer:?}");
}

/// Kriterium „Herzschlag ohne Knopf“: ein Strom liest nach dem Stand spätestens 7 s später ein
/// `event: herzschlag` (erwartet ≤ 5 s, der Standard-Herzschlag).
#[tokio::test]
async fn herzschlag_ohne_knopf_kommt_spaetestens_7_s_nach_dem_stand() {
    let instanz = wegwerf_starten();
    let zwei = Absender::von("127.0.0.2");
    let alpha = beitreten(&zwei, &instanz, "alpha").await;
    let mut strom = Strom::oeffnen(&zwei, &instanz.adresse, &alpha).await;
    let stand = strom
        .bis(Duration::from_secs(5), |e| e.data["art"] == "teilnehmer")
        .await
        .expect("Stand");

    let herzschlag = strom
        .bis(Duration::from_secs(10), |e| e.event == "herzschlag")
        .await
        .expect("Herzschlag");

    let dauer = herzschlag.ankunft - stand.ankunft;
    eprintln!("messung herzschlag_ms={}", dauer.as_millis());
    assert!(dauer <= Duration::from_secs(7), "{dauer:?}");
}
