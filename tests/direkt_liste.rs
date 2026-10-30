//! R00050 F0172 B0771 — die Liste der eigenen Direktgespräche über das echte Programm (ADR-0055 Regel 3 letzter Punkt,
//! Matrix R7): `GET /api/direkt` → `{direktgespraeche: [{kennung, letzte, frist_tage}]}`, nur die eigenen mit
//! mindestens einer Nachricht in der Frist, jüngste zuerst.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::direkt::{ANNA, ANNA_BEN, BEN, CORA, Teilnehmer, runde, runde_mit};
use gemeinsam::senden::{ordner, starten};
use gemeinsam::{ApiAntwort, api_antwort};
use serde_json::{Value, json};

async fn liste(t: &Teilnehmer, basis: &str) -> ApiAntwort {
    api_antwort(
        t.absender
            .anfrage(reqwest::Method::GET, format!("{basis}api/direkt"))
            .header("X-LocalchatA-Sitzung", &t.token),
    )
    .await
}

fn kennungen(antwort: &ApiAntwort) -> Vec<Value> {
    antwort.json["direktgespraeche"]
        .as_array()
        .expect("Liste")
        .iter()
        .map(|eintrag| eintrag["kennung"].clone())
        .collect()
}

fn zeit(antwort: &ApiAntwort) -> String {
    assert_eq!(antwort.status, 201, "{}", antwort.json);
    antwort.json["nachricht"]["zeit"]
        .as_str()
        .expect("zeit")
        .to_string()
}

/// D11, erster Test (wörtlich): `GET /api/direkt` als `anna` → `[{kennung: ben@…, letzte, frist_tage: 7}]`, als `cora`
/// → `[]`; `Cache-Control: no-store`. Mutante „fremde Gespräche in der Liste“.
#[tokio::test]
async fn jeder_sieht_nur_seine_gespraeche_mit_letzter_zeit_und_sieben_tagen() {
    let r = runde(&[]).await;
    let basis = r.instanz.adresse.clone();
    let letzte = zeit(&r.anna.senden(&basis, BEN, &json!({"text": "eins"})).await);
    let juengste = zeit(&r.ben.senden(&basis, ANNA, &json!({"text": "zwei"})).await);

    let annas = liste(&r.anna, &basis).await;
    let bens = liste(&r.ben, &basis).await;
    let coras = liste(&r.cora, &basis).await;

    assert_eq!(annas.status, 200, "{}", annas.json);
    assert_eq!(
        annas.json,
        json!({"direktgespraeche": [{"kennung": BEN, "letzte": juengste, "frist_tage": 7}]})
    );
    assert_eq!(
        bens.json,
        json!({"direktgespraeche": [{"kennung": ANNA, "letzte": juengste, "frist_tage": 7}]})
    );
    assert_ne!(letzte, juengste);
    assert_eq!(
        coras.json,
        json!({"direktgespraeche": []}),
        "cora sieht nichts"
    );
    assert_eq!(
        annas
            .kopf
            .get("cache-control")
            .and_then(|w| w.to_str().ok()),
        Some("no-store")
    );
}

/// Mehrere Gespräche: das mit der jüngsten Nachricht zuerst; ein Dritter erscheint nur im eigenen Paar.
#[tokio::test]
async fn die_liste_ist_nach_der_juengsten_nachricht_absteigend() {
    let r = runde(&[]).await;
    let basis = r.instanz.adresse.clone();
    zeit(
        &r.anna
            .senden(&basis, BEN, &json!({"text": "zuerst mit ben"}))
            .await,
    );
    zeit(
        &r.anna
            .senden(&basis, CORA, &json!({"text": "dann mit cora"}))
            .await,
    );

    let annas = liste(&r.anna, &basis).await;
    let bens = liste(&r.ben, &basis).await;
    let coras = liste(&r.cora, &basis).await;

    assert_eq!(kennungen(&annas), [json!(CORA), json!(BEN)]);
    assert_eq!(kennungen(&bens), [json!(ANNA)]);
    assert_eq!(kennungen(&coras), [json!(ANNA)]);
}

/// Ein Paarordner aus einem früheren Lauf erscheint mit seiner letzten Zeit; ein leerer Ordner (ohne Nachricht in der
/// Frist) nicht; ohne Sitzung ist es 401.
#[tokio::test]
async fn ein_geladenes_gespraech_erscheint_ein_leerer_ordner_und_ohne_sitzung_nichts() {
    let wurzel = ordner();
    let paar = wurzel.path().join("direkt").join(ANNA_BEN);
    std::fs::create_dir_all(&paar).expect("Paar");
    std::fs::write(
        paar.join("2026-10-27.log"),
        "@ 2026-10-27T15:00:00.000+01:00 anna@127.0.0.2\n+ art: agent\n  von gestern\n\n",
    )
    .expect("Tag");
    let leer = wurzel
        .path()
        .join("direkt")
        .join("anna@127.0.0.2+cora@127.0.0.1");
    std::fs::create_dir_all(&leer).expect("leer");
    let instanz = starten(wurzel.path(), &[]);
    let r = runde_mit(wurzel, instanz).await;
    let basis = r.instanz.adresse.clone();

    let annas = liste(&r.anna, &basis).await;
    let ohne = api_antwort(
        r.anna
            .absender
            .anfrage(reqwest::Method::GET, format!("{basis}api/direkt")),
    )
    .await;

    assert_eq!(
        annas.json,
        json!({"direktgespraeche": [{"kennung": BEN, "letzte": "2026-10-27T15:00:00.000+01:00", "frist_tage": 7}]})
    );
    assert_eq!(ohne.status, 401, "{}", ohne.json);
}

/// R7 (wörtlich): `Host: evil.example`; `Origin: http://evil.example` mit Sitzungskopf → 403 vor allem (Host); bei
/// fremdem Origin auf GET kein `Access-Control-Allow-*`; `Cache-Control: no-store` auf `GET /api/direkt`.
#[tokio::test]
async fn r7_fremder_host_ist_403_fremder_origin_bekommt_keine_cors_freigabe() {
    let r = runde(&[]).await;
    let basis = r.instanz.adresse.clone();
    zeit(&r.anna.senden(&basis, BEN, &json!({"text": "privat"})).await);

    let fremder_host = api_antwort(
        r.anna
            .absender
            .anfrage(reqwest::Method::GET, format!("{basis}api/direkt"))
            .header("Host", "evil.example")
            .header("X-LocalchatA-Sitzung", &r.anna.token),
    )
    .await;
    let fremder_origin = api_antwort(
        r.anna
            .absender
            .anfrage(reqwest::Method::GET, format!("{basis}api/direkt"))
            .header("Origin", "http://evil.example")
            .header("X-LocalchatA-Sitzung", &r.anna.token),
    )
    .await;

    assert_eq!(fremder_host.status, 403, "{}", fremder_host.json);
    assert!(!fremder_host.json.to_string().contains(BEN));
    assert!(
        fremder_origin
            .kopf
            .keys()
            .all(|name| !name.as_str().starts_with("access-control-allow")),
        "{:?}",
        fremder_origin.kopf
    );
    assert_eq!(
        fremder_origin
            .kopf
            .get("cache-control")
            .and_then(|w| w.to_str().ok()),
        Some("no-store")
    );
}
