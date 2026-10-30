//! R00050 F0172 B0768 — Senden und Lesen im Direktgespräch über das echte Programm (ADR-0055 Regel 3, Matrix K1):
//! `anna@127.0.0.2` ↔ `ben@127.0.0.3`, das Gegenüber im Pfad, das Paar aus Sitzung und Pfad.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::direkt::{ANNA, ANNA_BEN, BEN, direkt_eintraege, paarinhalt, runde};
use serde_json::json;

/// K1 (wörtlich): anna `POST /api/direkt/ben@127.0.0.3/nachrichten {"text":"k1"}`; ben `GET
/// /api/direkt/anna@127.0.0.2/nachrichten` → 201 `{channel:null, direkt:"ben@127.0.0.3", nachricht}`; 200 mit k1;
/// Ordner `direkt/anna@127.0.0.2+ben@127.0.0.3/<tag>.log`. Mutante „Paar ohne Sortierung“: ben → anna legte einen
/// zweiten Ordner an.
#[tokio::test]
async fn k1_anna_sendet_ben_ben_liest_ein_ordner_in_beide_richtungen() {
    let r = runde(&[]).await;
    let basis = r.instanz.adresse.clone();

    let gesendet = r.anna.senden(&basis, BEN, &json!({"text": "k1"})).await;
    let gelesen = r.ben.lesen(&basis, ANNA).await;
    let antwort = r
        .ben
        .senden(&basis, ANNA, &json!({"text": "k1 zurück"}))
        .await;
    let annas_sicht = r.anna.lesen(&basis, BEN).await;

    assert_eq!(gesendet.status, 201, "{}", gesendet.json);
    assert_eq!(gesendet.json["channel"], json!(null), "{}", gesendet.json);
    assert_eq!(gesendet.json["direkt"], BEN, "{}", gesendet.json);
    assert_eq!(gesendet.json["nachricht"]["text"], "k1");
    assert_eq!(gesendet.json["nachricht"]["von"], ANNA);
    assert_eq!(gelesen.status, 200, "{}", gelesen.json);
    assert_eq!(gelesen.json["channel"], json!(null));
    assert_eq!(gelesen.json["direkt"], ANNA);
    assert_eq!(gelesen.json["nachrichten"][0], gesendet.json["nachricht"]);
    assert_eq!(
        gelesen.json["nachrichten"].as_array().map(Vec::len),
        Some(1)
    );
    assert_eq!(antwort.status, 201, "{}", antwort.json);
    assert_eq!(antwort.json["direkt"], ANNA);
    let beide = annas_sicht.json["nachrichten"].as_array().expect("Liste");
    assert_eq!(
        beide.iter().map(|n| n["text"].clone()).collect::<Vec<_>>(),
        [json!("k1"), json!("k1 zurück")]
    );
    assert_eq!(annas_sicht.json["direkt"], BEN);
    assert_eq!(direkt_eintraege(r.wurzel.path()), [ANNA_BEN]);
    let log = String::from_utf8(paarinhalt(r.wurzel.path(), ANNA_BEN)).expect("UTF-8");
    assert!(log.contains("k1") && log.contains("k1 zurück"), "{log}");
}

/// Der Channel bleibt, wie er war: dieselbe Form ohne `direkt`, `channel` ein String.
#[tokio::test]
async fn die_antworten_der_channels_tragen_kein_direkt() {
    let r = runde(&[]).await;
    let basis = r.instanz.adresse.clone();

    let gesendet =
        gemeinsam::senden::senden(&r.anna.absender, &basis, &r.anna.token, "allgemein", "hi").await;
    let verlauf =
        gemeinsam::senden::verlauf(&r.anna.absender, &basis, &r.anna.token, "allgemein").await;

    assert_eq!(gesendet.json["channel"], "allgemein");
    assert!(gesendet.json.get("direkt").is_none(), "{}", gesendet.json);
    assert_eq!(verlauf["channel"], "allgemein");
    assert!(verlauf.get("direkt").is_none(), "{verlauf}");
}
