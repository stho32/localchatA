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

/// US-1 (Kriterium „Gesundheit“): der Betrieb sieht die wirksamen Werte — ohne Knopf 5000 / 10 000, mit
/// `LOCALCHATA_HERZSCHLAG_MS=500` 500 / 1000.
#[tokio::test]
async fn us1_gesundheit_nennt_herzschlag_und_abrissgrenze() {
    let ohne_knopf = wegwerf_starten();
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let mit_knopf = gemeinsam::starten_mit(
        wurzel.path(),
        &["--port", "0", "--host", "127.0.0.1", "--kein-browser"],
        &[("LOCALCHATA_HERZSCHLAG_MS", "500")],
    );

    let (_, standard) = gemeinsam::gesundheit(&ohne_knopf.adresse).await;
    let (_, gestellt) = gemeinsam::gesundheit(&mit_knopf.adresse).await;

    assert_eq!(
        (&standard["herzschlag_ms"], &standard["abrissgrenze_ms"]),
        (&json!(5000), &json!(10_000)),
        "{standard}"
    );
    assert_eq!(
        (&gestellt["herzschlag_ms"], &gestellt["abrissgrenze_ms"]),
        (&json!(500), &json!(1000)),
        "{gestellt}"
    );
}

/// Der Kernel-Timer einer angenommenen Verbindung, wie `/proc/net/tcp` ihn zeigt: (`timer_active`, `when` in
/// Ticks zu 1/100 s) der Zeile im Zustand ESTABLISHED (`01`) mit lokalem Port `instanzport` und Gegenport
/// `clientport`.
fn kernel_timer(instanzport: u16, clientport: u16) -> Option<(u32, u64)> {
    let tabelle = std::fs::read_to_string("/proc/net/tcp").expect("/proc/net/tcp");
    tabelle.lines().skip(1).find_map(|zeile| {
        let felder: Vec<&str> = zeile.split_whitespace().collect();
        let port = |adresse: &str| u16::from_str_radix(adresse.rsplit(':').next()?, 16).ok();
        let verbunden = felder[3] == "01";
        let passt = verbunden && port(felder[1])? == instanzport && port(felder[2])? == clientport;
        if !passt {
            return None;
        }
        let (aktiv, wann) = felder[5].split_once(':')?;
        Some((
            u32::from_str_radix(aktiv, 16).ok()?,
            u64::from_str_radix(wann, 16).ok()?,
        ))
    })
}

/// Hält von 127.0.0.2 eine Keep-alive-Verbindung mit einer beantworteten Anfrage offen und liest dann, bis der
/// Keepalive-Timer (`timer_active` = 2) der angenommenen Seite läuft, höchstens 3 s: (`timer_active`, `when`).
async fn timer_nach_einer_anfrage(instanz: &Instanz) -> (u32, u64) {
    let socket = TcpSocket::new_v4().expect("Socket");
    socket
        .bind("127.0.0.2:0".parse().expect("Absender"))
        .expect("gebunden");
    let ziel = format!("127.0.0.1:{}", instanz.port())
        .parse()
        .expect("Ziel");
    let mut strom = socket.connect(ziel).await.expect("verbunden");
    let clientport = strom.local_addr().expect("Adresse").port();
    strom
        .write_all(b"GET /api/gesundheit HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
        .await
        .expect("gesendet");
    let mut puffer = vec![0u8; 4096];
    let gelesen = strom.read(&mut puffer).await.expect("Antwort");
    assert!(String::from_utf8_lossy(&puffer[..gelesen]).starts_with("HTTP/1.1 200"));
    let ende = Instant::now() + Duration::from_secs(3);
    let mut zuletzt = None;
    while Instant::now() < ende {
        zuletzt = kernel_timer(instanz.port(), clientport);
        if matches!(zuletzt, Some((2, _))) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    drop(strom);
    zuletzt.expect("Zeile der angenommenen Verbindung in /proc/net/tcp")
}

/// Nacharbeit 1, Code-Review B1: der Start setzt die Kernel-Pflege mit dem Herzschlag ein —
/// `LOCALCHATA_HERZSCHLAG_MS=2000` → Keepalive-Timer aktiv, 100 < `when` ≤ 200 Ticks (Keepidle 2 s; die
/// Untergrenze fängt eine Pflege mit falschem Wert, Nacharbeit 2).
#[tokio::test]
async fn nacharbeit1_b1_der_start_pflegt_mit_dem_herzschlag_aus_dem_knopf() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let instanz = gemeinsam::starten_mit(
        wurzel.path(),
        &["--port", "0", "--host", "127.0.0.1", "--kein-browser"],
        &[("LOCALCHATA_HERZSCHLAG_MS", "2000")],
    );

    let (aktiv, wann) = timer_nach_einer_anfrage(&instanz).await;

    eprintln!("messung b1_when_2000_ticks={wann}");
    assert_eq!(aktiv, 2, "Keepalive-Timer, when {wann}");
    assert!(
        (101..=200).contains(&wann),
        "when {wann} Ticks, erwartet 100 < when ≤ 200 (Keepidle 2 s)"
    );
}

/// Variante ohne Knopf: Herzschlag 5 s → Keepalive-Timer aktiv, 400 < `when` ≤ 500 Ticks.
#[tokio::test]
async fn nacharbeit1_b1_ohne_knopf_pflegt_der_start_mit_5_s() {
    let instanz = wegwerf_starten();

    let (aktiv, wann) = timer_nach_einer_anfrage(&instanz).await;

    eprintln!("messung b1_when_standard_ticks={wann}");
    assert_eq!(aktiv, 2, "Keepalive-Timer, when {wann}");
    assert!(
        (401..=500).contains(&wann),
        "when {wann} Ticks, erwartet 400 < when ≤ 500 (Keepidle 5 s)"
    );
}
