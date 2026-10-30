//! R00050 F0174 B0775 — Anhang-IDs eines Direktgesprächs bleiben Dritten verborgen (ADR-0055 Regel 5, ADR-0047 B4;
//! Matrix A1, A3, W7; Linie L2, echtes Programm): `cora` erhält aus Verlauf, Liste, Strom und Warten keine ID aus
//! `anna` ↔ `ben`; mit dem Link öffnet sie (gewollt, B4); Anmelden bleibt gesprächsfrei; Senden mit der fremden ID
//! endet wie im Channel für eine gebundene ID.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::anhaenge::{ablegen, anmelden, id, starten as starten_anhaenge};
use gemeinsam::direkt::{ANNA, ANNA_BEN, BEN, Runde, direkt_eintraege, runde_mit};
use gemeinsam::senden::{JETZT, ordner, senden};
use gemeinsam::strom::Strom;
use gemeinsam::{ApiAntwort, api_antwort};
use serde_json::json;
use std::time::Duration;

const GEHEIM: &[u8] = b"nur fuer anna und ben";

async fn runde_anhaenge() -> Runde {
    let wurzel = ordner();
    let instanz = starten_anhaenge(wurzel.path(), &[], &[("LOCALCHATA_JETZT", JETZT)]);
    runde_mit(wurzel, instanz).await
}

/// `anna` legt `geheim.txt` ab und sendet sie an `ben`; liefert die ID.
async fn anna_sendet_anhang(r: &Runde) -> String {
    let basis = &r.instanz.adresse;
    let id = ablegen(
        &r.anna.absender,
        basis,
        &r.anna.token,
        "geheim.txt",
        GEHEIM.to_vec(),
    )
    .await;
    let gesendet = r
        .anna
        .senden(basis, BEN, &json!({"text": "für dich", "anhaenge": [id]}))
        .await;
    assert_eq!(gesendet.status, 201, "{}", gesendet.json);
    id
}

async fn get(r: &Runde, wer: &gemeinsam::direkt::Teilnehmer, pfad: &str) -> ApiAntwort {
    api_antwort(
        wer.absender
            .anfrage(reqwest::Method::GET, format!("{}{pfad}", r.instanz.adresse))
            .header("X-LocalchatA-Sitzung", &wer.token),
    )
    .await
}

/// A1: `GET /anhaenge/<id>` ohne Sitzung → 200 mit den Bytes (B4, gewollt); die ID und der Name tauchen in keiner
/// Antwort an `cora` auf — Verlauf des Channels, Verlauf mit `anna`, Liste der Direktgespräche, Warten, Strom. Als
/// Kontrolle sieht `ben` die ID im Verlauf.
#[tokio::test]
async fn mit_dem_link_oeffnet_cora_ohne_die_id_je_zu_sehen() {
    let r = runde_anhaenge().await;
    let basis = r.instanz.adresse.clone();
    let mut strom = Strom::oeffnen(&r.cora.absender, &basis, &r.cora.token).await;
    let t0 = senden(&r.cora.absender, &basis, &r.cora.token, "allgemein", "eins").await;
    let seit = t0.json["nachricht"]["zeit"]
        .as_str()
        .expect("zeit")
        .to_string();

    let id = anna_sendet_anhang(&r).await;
    senden(&r.anna.absender, &basis, &r.anna.token, "allgemein", "zwei").await;

    let kontrolle = get(&r, &r.ben, &format!("api/direkt/{ANNA}/nachrichten")).await;
    let abruf = reqwest::get(format!("{basis}anhaenge/{id}"))
        .await
        .expect("Abruf");
    let abruf_status = abruf.status().as_u16();
    let abruf_bytes = abruf.bytes().await.expect("Körper");
    let nachholen = get(
        &r,
        &r.cora,
        &format!("api/warten?bis=0&seit={}", seit.replace('+', "%2B")),
    )
    .await;
    let an_cora = [
        get(&r, &r.cora, "api/channels/allgemein/nachrichten").await,
        get(&r, &r.cora, &format!("api/direkt/{ANNA}/nachrichten")).await,
        get(&r, &r.cora, "api/direkt").await,
        nachholen,
    ];
    let im_strom = strom
        .bis(Duration::from_millis(1500), |e| {
            e.data.to_string().contains("geheim")
        })
        .await;

    assert!(
        kontrolle.json.to_string().contains(&id),
        "Kontrolle: Beteiligte sehen die ID: {}",
        kontrolle.json
    );
    assert_eq!((abruf_status, abruf_bytes.as_ref()), (200, GEHEIM));
    for antwort in &an_cora {
        assert_eq!(antwort.status, 200, "{}", antwort.json);
        let text = antwort.json.to_string();
        assert!(!text.contains(&id), "ID in einer Antwort an cora: {text}");
        assert!(!text.contains("geheim.txt"), "Name an cora: {text}");
    }
    let gesehen: Vec<String> = strom
        .ereignisse
        .iter()
        .map(|e| e.data.to_string())
        .collect();
    assert!(
        im_strom.is_none() && gesehen.iter().all(|e| !e.contains(&id)),
        "Strom von cora: {gesehen:?}"
    );
}

/// A3: Anmelden ist gesprächsfrei wie heute — dieselbe Antwortform für `cora` und `anna`, kein Hinweis auf ein
/// Gespräch, und eine ID aus `anna` ↔ `ben` ändert daran nichts.
#[tokio::test]
async fn anmelden_bleibt_gespraechsfrei() {
    let r = runde_anhaenge().await;
    let basis = r.instanz.adresse.clone();
    anna_sendet_anhang(&r).await;

    let von_cora = anmelden(&r.cora.absender, &basis, &r.cora.token, "c.txt", 3).await;
    let von_anna = anmelden(&r.anna.absender, &basis, &r.anna.token, "a.txt", 3).await;

    for antwort in [&von_cora, &von_anna] {
        assert_eq!(antwort.status, 201, "{}", antwort.json);
        let text = antwort.json.to_string();
        assert!(
            !text.contains("direkt") && !text.contains("gespraech"),
            "{text}"
        );
    }
    let felder = |antwort: &ApiAntwort| -> Vec<String> {
        antwort.json["anhang"]
            .as_object()
            .expect("anhang")
            .keys()
            .cloned()
            .collect()
    };
    assert_eq!(felder(&von_cora), felder(&von_anna));
    id(&von_cora);
}

/// W7: `cora` sendet mit der ID aus `anna` ↔ `ben` (per Link bekannt) in einen Channel und ins eigene Direktgespräch
/// mit `anna` → wie im Channel für eine gebundene ID (M38): 422 `anhang`, Grund `unbekannt`, gleiche Antwort wie für
/// die gebundene ID eines Channels; kein Bezug, nichts geschrieben, kein neues Paar.
#[tokio::test]
async fn senden_mit_der_fremden_id_endet_wie_im_channel() {
    let r = runde_anhaenge().await;
    let basis = r.instanz.adresse.clone();
    let id = anna_sendet_anhang(&r).await;
    let channel_id = ablegen(
        &r.anna.absender,
        &basis,
        &r.anna.token,
        "c.txt",
        b"c".to_vec(),
    )
    .await;
    let gebunden = api_antwort(
        r.anna
            .absender
            .anfrage(
                reqwest::Method::POST,
                format!("{basis}api/channels/allgemein/nachrichten"),
            )
            .header("X-LocalchatA-Sitzung", &r.anna.token)
            .json(&json!({"text": "x", "anhaenge": [channel_id]})),
    )
    .await;
    assert_eq!(gebunden.status, 201, "{}", gebunden.json);
    let vorher = std::fs::read(
        r.wurzel
            .path()
            .join("direkt")
            .join(ANNA_BEN)
            .join(format!("{}.log", gemeinsam::senden::HEUTE)),
    )
    .expect("Tag");

    let in_den_channel = api_antwort(
        r.cora
            .absender
            .anfrage(
                reqwest::Method::POST,
                format!("{basis}api/channels/allgemein/nachrichten"),
            )
            .header("X-LocalchatA-Sitzung", &r.cora.token)
            .json(&json!({"text": "x", "anhaenge": [id]})),
    )
    .await;
    let ins_direkte = r
        .cora
        .senden(&basis, ANNA, &json!({"text": "x", "anhaenge": [id]}))
        .await;
    let kontrolle = api_antwort(
        r.cora
            .absender
            .anfrage(
                reqwest::Method::POST,
                format!("{basis}api/channels/allgemein/nachrichten"),
            )
            .header("X-LocalchatA-Sitzung", &r.cora.token)
            .json(&json!({"text": "x", "anhaenge": [channel_id]})),
    )
    .await;

    // Die Antwort nennt die gesendete ID im Text; sonst muss sie gleich sein.
    let ohne_id = |antwort: &ApiAntwort, gesendet: &str| -> String {
        antwort.json.to_string().replace(gesendet, "<id>")
    };
    assert_eq!(kontrolle.status, 422, "{}", kontrolle.json);
    for antwort in [&in_den_channel, &ins_direkte] {
        assert_eq!(antwort.status, 422, "{}", antwort.json);
        assert_eq!(antwort.json["regel"], "anhang", "{}", antwort.json);
        assert_eq!(antwort.json["grund"], "unbekannt", "{}", antwort.json);
        assert_eq!(ohne_id(antwort, &id), ohne_id(&kontrolle, &channel_id));
    }
    assert_eq!(
        direkt_eintraege(r.wurzel.path()),
        [ANNA_BEN],
        "kein Paar mit cora"
    );
    let nachher = std::fs::read(
        r.wurzel
            .path()
            .join("direkt")
            .join(ANNA_BEN)
            .join(format!("{}.log", gemeinsam::senden::HEUTE)),
    )
    .expect("Tag");
    assert_eq!(nachher, vorher, "nichts in das Gespräch geschrieben");
}
