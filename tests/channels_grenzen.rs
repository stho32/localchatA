//! F0022 Grenzen beim Anlegen — über das echte Programm (R00006, US-9 bis US-11 und Kriterien).
//! Wertprüfung als Spanne: der Wert an der Grenze geht, der nächste nicht.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::{Absender, Instanz, gesundheit, lokal_starten, wegwerf_starten};
use serde_json::json;

async fn beitreten(absender: &Absender, instanz: &Instanz, name: &str) -> String {
    let antwort = absender
        .beitreten(&instanz.adresse, json!({ "name": name }), None)
        .await;
    assert_eq!(antwort.status, 201, "{}", antwort.json);
    antwort.json["token"].as_str().expect("Token").to_string()
}

async fn anlegen(absender: &Absender, instanz: &Instanz, token: &str, name: &str) -> u16 {
    absender
        .channel_anlegen(&instanz.adresse, token, json!({ "name": name }))
        .await
        .status
}

/// Legt `<vorsilbe>01` … `<vorsilbe><bis>` an und liefert die Status in dieser Reihenfolge.
async fn reihe(
    absender: &Absender,
    instanz: &Instanz,
    token: &str,
    vorsilbe: &str,
    bis: usize,
) -> Vec<u16> {
    let mut status = Vec::new();
    for nummer in 1..=bis {
        status.push(anlegen(absender, instanz, token, &format!("{vorsilbe}{nummer:02}")).await);
    }
    status
}

#[tokio::test]
async fn us9_us10_hoechstens_32_je_herkunft_auch_nach_einem_neustart() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let instanz = lokal_starten(wurzel.path());
    let zwei = Absender::von("127.0.0.2");
    let drei = Absender::von("127.0.0.3");
    let alpha = beitreten(&zwei, &instanz, "alpha").await;

    let erste_32 = reihe(&zwei, &instanz, &alpha, "c", 32).await;
    let c33 = zwei
        .channel_anlegen(&instanz.adresse, &alpha, json!({ "name": "c33" }))
        .await;
    let beta = beitreten(&drei, &instanz, "beta").await;
    let d01 = anlegen(&drei, &instanz, &beta, "d01").await;

    assert_eq!(erste_32, vec![201; 32]);
    assert_eq!(c33.status, 429, "{}", c33.json);
    assert_eq!(c33.json["grenze"], "herkunft");
    assert_eq!(c33.json["hoechstens"], 32);
    assert!(c33.json["fehler"].is_string(), "{}", c33.json);
    assert_eq!(d01, 201);

    drop(instanz);
    let instanz = lokal_starten(wurzel.path());
    let alpha = beitreten(&zwei, &instanz, "alpha").await;
    let nach_neustart = zwei
        .channel_anlegen(&instanz.adresse, &alpha, json!({ "name": "c33" }))
        .await;

    assert_eq!(nach_neustart.status, 429, "{}", nach_neustart.json);
    assert_eq!(nach_neustart.json["grenze"], "herkunft");
    assert!(!instanz.ordner.join("channels/c33").exists());
}

#[tokio::test]
async fn us11_hoechstens_256_insgesamt_ein_vorhandener_name_bleibt_409() {
    let instanz = wegwerf_starten();
    let mut status = Vec::new();
    for nummer in 2..=9 {
        let absender = Absender::von(&format!("127.0.0.{nummer}"));
        let token = beitreten(&absender, &instanz, &format!("agent{nummer}")).await;
        status.extend(reihe(&absender, &instanz, &token, &format!("h{nummer}-"), 32).await);
    }
    let eins = Absender::von("127.0.0.1");
    let token = beitreten(&eins, &instanz, "noch-einer").await;

    let ueber = eins
        .channel_anlegen(&instanz.adresse, &token, json!({ "name": "zu-viel" }))
        .await;
    let vorhanden = anlegen(&eins, &instanz, &token, "h2-01").await;

    assert_eq!(status, vec![201; 256]);
    assert_eq!(ueber.status, 429, "{}", ueber.json);
    assert_eq!(ueber.json["grenze"], "gesamt");
    assert_eq!(ueber.json["hoechstens"], 256);
    assert_eq!(vorhanden, 409);
    assert_eq!(gesundheit(&instanz.adresse).await.1["channels"], 256);
}

#[tokio::test]
async fn die_gesundheit_nennt_die_zahl_der_channels() {
    let instanz = wegwerf_starten();
    let eins = Absender::von("127.0.0.1");
    let token = beitreten(&eins, &instanz, "shoff").await;

    let vorher = gesundheit(&instanz.adresse).await.1;
    assert_eq!(anlegen(&eins, &instanz, &token, "allgemein").await, 201);
    let nachher = gesundheit(&instanz.adresse).await.1;

    assert_eq!(
        (&vorher["channels"], &nachher["channels"]),
        (&json!(0), &json!(1))
    );
}
