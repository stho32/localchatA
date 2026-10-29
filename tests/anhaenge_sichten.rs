//! R00029 F0117 — Gelöschte Anhänge in offenen Sichten (B0513, B0514): `zustand` je Anhang in Verlauf und
//! Senden-Antwort; Ereignis `anhaenge` je Channel im Strom.

mod gemeinsam;

use gemeinsam::Absender;
use gemeinsam::anhaenge::{MIB, ablegen, starten};
use gemeinsam::senden::{beitreten, ordner, roh_senden, verlauf};
use gemeinsam::strom::Strom;
use gemeinsam::{ApiAntwort, api_antwort};
use serde_json::{Value, json};
use std::time::Duration;
use tokio::time::Instant;

async fn senden_mit(
    absender: &Absender,
    basis: &str,
    token: &str,
    channel: &str,
    id: &str,
) -> ApiAntwort {
    let koerper = json!({"text": "", "anhaenge": [id]})
        .to_string()
        .into_bytes();
    roh_senden(
        absender,
        basis,
        Some(token),
        channel,
        "application/json",
        koerper,
    )
    .await
}

async fn loeschen(absender: &Absender, basis: &str, token: &str, koerper: Value) -> ApiAntwort {
    api_antwort(
        absender
            .anfrage(
                reqwest::Method::POST,
                format!("{basis}api/anhaenge/loeschen"),
            )
            .header("Content-Type", "application/json")
            .header("X-LocalchatA-Sitzung", token)
            .body(koerper.to_string()),
    )
    .await
}

/// Zustand überall (B0513): nach dem Löschen nennt der Verlauf `a` mit `zustand: "geloescht"` (Name, Bytes, Art,
/// URL bleiben) und `c` mit `"vorhanden"`; die Senden-Antwort einer neuen Nachricht mit Anhang trägt `"vorhanden"`.
#[tokio::test]
async fn zustand_in_verlauf_und_senden() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, basis, "beta", Some("agent")).await;
    let a = ablegen(&beta, basis, &token, "a.bin", vec![0; 100]).await;
    let erste = senden_mit(&beta, basis, &token, "allgemein", &a).await;
    assert_eq!(
        erste.json["nachricht"]["anhaenge"][0]["zustand"],
        "vorhanden"
    );
    let c = ablegen(&beta, basis, &token, "c.bin", vec![0; 100]).await;
    assert_eq!(
        senden_mit(&beta, basis, &token, "allgemein", &c)
            .await
            .status,
        201
    );

    assert_eq!(
        loeschen(
            &beta,
            basis,
            &token,
            json!({"ids": [a], "bestaetigt": true})
        )
        .await
        .status,
        200
    );

    let gelesen = verlauf(&beta, basis, &token, "allgemein").await;
    let anhang = &gelesen["nachrichten"][0]["anhaenge"][0];
    assert_eq!(
        anhang,
        &json!({
            "id": a, "name": "a.bin", "bytes": 100, "art": "application/octet-stream", "breite": null,
            "hoehe": null, "url": format!("/anhaenge/{a}"), "zustand": "geloescht",
        })
    );
    assert_eq!(
        gelesen["nachrichten"][1]["anhaenge"][0]["zustand"],
        "vorhanden"
    );
}
/// Befund #55 S2: die Wiederholung eines Sendens (gleicher Schlüssel, gleicher Körper) antwortet 200 mit dem
/// heutigen `zustand` je Anhang — vor dem Löschen `vorhanden`, danach `geloescht` (Name, Bytes, URL bleiben).
#[tokio::test]
async fn zustand_nach_einer_wiederholung() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, basis, "beta", Some("agent")).await;
    let a = ablegen(&beta, basis, &token, "a.bin", vec![0; 100]).await;
    let senden = || {
        let (beta, token, a) = (&beta, &token, &a);
        async move {
            api_antwort(
                beta.anfrage(
                    reqwest::Method::POST,
                    format!("{basis}api/channels/allgemein/nachrichten"),
                )
                .header("X-LocalchatA-Sitzung", token)
                .header("Idempotency-Key", "w1")
                .json(&json!({"text": "x", "anhaenge": [a]})),
            )
            .await
        }
    };
    assert_eq!(senden().await.status, 201);
    let vorher = senden().await;
    assert_eq!(vorher.status, 200, "{}", vorher.json);
    assert_eq!(
        vorher.json["nachricht"]["anhaenge"][0]["zustand"],
        "vorhanden"
    );
    assert_eq!(
        loeschen(
            &beta,
            basis,
            &token,
            json!({"ids": [a], "bestaetigt": true})
        )
        .await
        .status,
        200
    );

    let danach = senden().await;

    assert_eq!(danach.status, 200, "{}", danach.json);
    assert_eq!(
        danach.json["nachricht"]["anhaenge"][0],
        json!({
            "id": a, "name": "a.bin", "bytes": 100, "art": "application/octet-stream", "breite": null,
            "hoehe": null, "url": format!("/anhaenge/{a}"), "zustand": "geloescht",
        })
    );
}

/// Ereignis live (B0514, QA7′): `alpha` liest den Strom; `gamma` löscht `a` (allgemein) und `b` (technik) bestätigt
/// → genau zwei Ereignisse `anhaenge` mit ID und Satzzeit, je < 1 s nach der 200; die 409-Vorschau und ein
/// `schon_geloescht` erzeugen keines.
#[tokio::test]
async fn ereignis_anhaenge_je_channel() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let gamma = Absender::von("127.0.0.1");
    let alpha = Absender::von("127.0.0.2");
    let token_beta = beitreten(&beta, basis, "beta", Some("agent")).await;
    let token_gamma = beitreten(&gamma, basis, "gamma", Some("agent")).await;
    let token_alpha = beitreten(&alpha, basis, "alpha", None).await;
    let a = ablegen(&beta, basis, &token_beta, "a.bin", vec![1; MIB as usize]).await;
    let zeit_a = senden_mit(&beta, basis, &token_beta, "allgemein", &a)
        .await
        .json["nachricht"]["zeit"]
        .clone();
    let b = ablegen(&beta, basis, &token_beta, "b.bin", vec![1; MIB as usize]).await;
    let zeit_b = senden_mit(&beta, basis, &token_beta, "technik", &b)
        .await
        .json["nachricht"]["zeit"]
        .clone();
    let mut strom = Strom::oeffnen(&alpha, basis, &token_alpha).await;

    assert_eq!(
        loeschen(&gamma, basis, &token_gamma, json!({"ids": [a, b]}))
            .await
            .status,
        409
    );
    let antwort = loeschen(
        &gamma,
        basis,
        &token_gamma,
        json!({"ids": [a, b], "bestaetigt": true}),
    )
    .await;
    let nach_der_200 = Instant::now();
    assert_eq!(antwort.status, 200);
    let ist_anhaenge = |e: &gemeinsam::strom::Empfangen| e.data["art"] == "anhaenge";
    let anzahl = |strom: &Strom| strom.ereignisse.iter().filter(|e| ist_anhaenge(e)).count();
    while anzahl(&strom) < 2 && nach_der_200.elapsed() < Duration::from_secs(2) {
        strom.bis(Duration::from_millis(100), ist_anhaenge).await;
    }
    let angekommen: Vec<gemeinsam::strom::Empfangen> = strom
        .ereignisse
        .iter()
        .filter(|e| ist_anhaenge(e))
        .cloned()
        .collect();
    assert_eq!(angekommen.len(), 2);
    for ereignis in &angekommen {
        assert!(ereignis.ankunft - nach_der_200 < Duration::from_secs(1));
    }
    let (erstes, zweites) = (angekommen[0].clone(), angekommen[1].clone());
    let mut ereignisse = vec![erstes.data, zweites.data];
    ereignisse.sort_by_key(|e| e["channel"].as_str().unwrap_or_default().to_string());
    assert_eq!(
        ereignisse,
        vec![
            json!({"art": "anhaenge", "channel": "allgemein", "geloescht": [{"id": a, "zeit": zeit_a}]}),
            json!({"art": "anhaenge", "channel": "technik", "geloescht": [{"id": b, "zeit": zeit_b}]}),
        ]
    );

    assert_eq!(
        loeschen(
            &gamma,
            basis,
            &token_gamma,
            json!({"ids": [a], "bestaetigt": true})
        )
        .await
        .status,
        200
    );
    assert!(
        strom
            .bis(Duration::from_millis(500), ist_anhaenge)
            .await
            .is_none(),
        "schon_geloescht ohne Ereignis"
    );
    let vorschau = strom
        .ereignisse
        .iter()
        .filter(|e| e.data["art"] == "anhaenge")
        .count();
    assert_eq!(vorschau, 2, "die 409-Vorschau erzeugt keines");
}
