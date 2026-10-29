//! R00029 F0117 — Gelöschte Anhänge in offenen Sichten (B0513, B0514): `zustand` je Anhang in Verlauf und
//! Senden-Antwort; Ereignis `anhaenge` je Channel im Strom.

mod gemeinsam;

use gemeinsam::Absender;
use gemeinsam::anhaenge::{ablegen, starten};
use gemeinsam::senden::{beitreten, ordner, roh_senden, verlauf};
use gemeinsam::{ApiAntwort, api_antwort};
use serde_json::{Value, json};

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
