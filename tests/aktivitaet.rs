//! R00045 F0161 „Tippen sehen“, Serverteil (ADR-0051 P4, P5, P6, P8 ohne `gueltig_s`) — über das echte Programm:
//! `PUT`/`DELETE /api/channels/{name}/aktivitaet`, Ereignis `aktivitaet` im Strom der anderen, nie im eigenen.
//! Bis zum Lesen (B0696) prüft `ben` über seinen Strom, ob etwas gewirkt hat.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::senden::{beitreten, ordner, starten};
use gemeinsam::strom::{Empfangen, Strom};
use gemeinsam::{Absender, ApiAntwort, Instanz, api_antwort};
use serde_json::{Value, json};
use std::time::Duration;
use tokio::time::Instant;

/// Erscheinen binnen 1 s nach der 200 (R00045 „Erscheinen“, wie Z10 in R00011).
const ERSCHEINEN: Duration = Duration::from_secs(1);
/// So lange wird auf etwas gewartet, das nicht kommen darf (ADR-0051 P4: „binnen 500 ms kein `aktivitaet`“).
const STILLE: Duration = Duration::from_millis(500);

async fn setzen(
    absender: &Absender,
    instanz: &Instanz,
    token: Option<&str>,
    channel: &str,
    koerper: Value,
) -> ApiAntwort {
    aufrufen(
        absender,
        instanz,
        reqwest::Method::PUT,
        token,
        channel,
        Some(koerper),
        &[],
    )
    .await
}

async fn loeschen(
    absender: &Absender,
    instanz: &Instanz,
    token: &str,
    channel: &str,
) -> ApiAntwort {
    aufrufen(
        absender,
        instanz,
        reqwest::Method::DELETE,
        Some(token),
        channel,
        None,
        &[],
    )
    .await
}

async fn aufrufen(
    absender: &Absender,
    instanz: &Instanz,
    methode: reqwest::Method,
    token: Option<&str>,
    channel: &str,
    koerper: Option<Value>,
    kopf: &[(&str, &str)],
) -> ApiAntwort {
    let mut anfrage = absender.anfrage(
        methode,
        format!("{}api/channels/{channel}/aktivitaet", instanz.adresse),
    );
    if let Some(token) = token {
        anfrage = anfrage.header("X-LocalchatA-Sitzung", token);
    }
    if let Some(koerper) = koerper {
        anfrage = anfrage
            .header("Content-Type", "application/json")
            .body(koerper.to_string());
    }
    for (name, wert) in kopf {
        anfrage = anfrage.header(*name, *wert);
    }
    api_antwort(anfrage).await
}

fn ist_aktivitaet(e: &Empfangen) -> bool {
    e.data["art"] == "aktivitaet"
}

fn aktivitaet_von(kennung: &'static str) -> impl Fn(&Empfangen) -> bool {
    move |e| ist_aktivitaet(e) && e.data["kennung"] == kennung
}

/// `anna`, `ben`, `cora` von 127.0.0.1 in einer Instanz mit `# allgemein` und `# technik`; `ben` liest den Strom.
struct Runde {
    _ordner: tempfile::TempDir,
    instanz: Instanz,
    absender: Absender,
    anna: String,
    cora: String,
    strom_ben: Strom,
}

async fn runde() -> Runde {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[]);
    let absender = Absender::von("127.0.0.1");
    let anna = beitreten(&absender, &instanz.adresse, "anna", None).await;
    let ben = beitreten(&absender, &instanz.adresse, "ben", None).await;
    let cora = beitreten(&absender, &instanz.adresse, "cora", None).await;
    let mut strom_ben = Strom::oeffnen(&absender, &instanz.adresse, &ben).await;
    strom_ben
        .bis(Duration::from_secs(5), |e| e.data["art"] == "channels")
        .await
        .expect("Stand der Channels");
    Runde {
        _ordner: wurzel,
        instanz,
        absender,
        anna,
        cora,
        strom_ben,
    }
}

/// P4 erster Satz (Mutante M1): `anna` setzt `tippt` → 200 mit `zustand`, `bis` = `jetzt` + 6 s; `ben` liest
/// `aktivitaet` binnen 1 s nach der 200; `anna`s eigener Strom liest binnen 500 ms keines.
#[tokio::test]
async fn p4_anna_tippt_ben_sieht_es_binnen_einer_sekunde_anna_selbst_nie() {
    let mut r = runde().await;
    let mut strom_anna = Strom::oeffnen(&r.absender, &r.instanz.adresse, &r.anna).await;
    strom_anna
        .bis(Duration::from_secs(5), |e| e.data["art"] == "channels")
        .await
        .expect("Stand der Channels");

    let gesetzt = setzen(
        &r.absender,
        &r.instanz,
        Some(&r.anna),
        "allgemein",
        json!({"zustand": "tippt"}),
    )
    .await;
    let antwort_da = Instant::now();
    let bei_ben = r
        .strom_ben
        .bis(Duration::from_secs(5), aktivitaet_von("anna@127.0.0.1"))
        .await
        .expect("aktivitaet bei ben");
    let bei_anna = strom_anna.bis(STILLE, ist_aktivitaet).await;

    assert_eq!(gesetzt.status, 200, "{}", gesetzt.json);
    assert_eq!(gesetzt.json["zustand"], "tippt");
    let zeit = |wert: &Value| {
        chrono::DateTime::parse_from_rfc3339(wert.as_str().expect("Zeit")).expect("RFC 3339")
    };
    assert_eq!(
        zeit(&gesetzt.json["bis"]) - zeit(&gesetzt.json["jetzt"]),
        chrono::TimeDelta::seconds(6)
    );
    let warten = bei_ben.ankunft.saturating_duration_since(antwort_da);
    println!("Erscheinen gemessen: {} µs", warten.as_micros());
    assert!(warten <= ERSCHEINEN, "{warten:?}");
    assert_eq!(
        bei_ben.data,
        json!({
            "art": "aktivitaet", "channel": "allgemein", "kennung": "anna@127.0.0.1",
            "zustand": "tippt", "bis": gesetzt.json["bis"], "jetzt": gesetzt.json["jetzt"]
        })
    );
    assert!(bei_anna.is_none(), "{bei_anna:?}");
}

/// P5 Inhaber (Mutante M8): eine Kennung im Körper ist 422 `feld` ohne Wirkung; `cora` setzt nur für sich; `cora`
/// löscht nur ihren eigenen Zustand — `anna`s bleibt.
#[tokio::test]
async fn p5_nur_die_eigene_kennung_setzt_und_loescht_den_eigenen_zustand() {
    let mut r = runde().await;

    let fremd = setzen(
        &r.absender,
        &r.instanz,
        Some(&r.cora),
        "allgemein",
        json!({"zustand": "tippt", "kennung": "anna@127.0.0.1"}),
    )
    .await;
    let nach_fremd = r.strom_ben.bis(STILLE, ist_aktivitaet).await;
    let cora = setzen(
        &r.absender,
        &r.instanz,
        Some(&r.cora),
        "allgemein",
        json!({"zustand": "tippt"}),
    )
    .await;
    let cora_bei_ben = r
        .strom_ben
        .bis(Duration::from_secs(5), ist_aktivitaet)
        .await
        .expect("cora");
    let anna = setzen(
        &r.absender,
        &r.instanz,
        Some(&r.anna),
        "allgemein",
        json!({"zustand": "tippt"}),
    )
    .await;
    let anna_bei_ben = r
        .strom_ben
        .bis(Duration::from_secs(5), ist_aktivitaet)
        .await
        .expect("anna");
    let geloescht = loeschen(&r.absender, &r.instanz, &r.cora, "allgemein").await;
    let ende_bei_ben = r
        .strom_ben
        .bis(Duration::from_secs(5), ist_aktivitaet)
        .await
        .expect("Ende");
    let danach = r.strom_ben.bis(STILLE, ist_aktivitaet).await;

    assert_eq!(fremd.status, 422, "{}", fremd.json);
    assert_eq!(fremd.json["regel"], "feld");
    assert_eq!(fremd.json["feld"], "kennung");
    assert!(nach_fremd.is_none(), "{nach_fremd:?}");
    assert_eq!((cora.status, anna.status), (200, 200));
    assert_eq!(
        (&cora_bei_ben.data["kennung"], &cora_bei_ben.data["zustand"]),
        (&json!("cora@127.0.0.1"), &json!("tippt"))
    );
    assert_eq!(anna_bei_ben.data["kennung"], "anna@127.0.0.1");
    assert_eq!(geloescht.status, 204);
    assert_eq!(
        ende_bei_ben.data,
        json!({"art": "aktivitaet", "channel": "allgemein", "kennung": "cora@127.0.0.1", "zustand": null})
    );
    assert!(danach.is_none(), "anna bleibt: {danach:?}");
}

/// P8 ohne `gueltig_s` (Stufe I0040): falscher Wortlaut 422 `zustand`, `denkt_nach` und `gueltig_s` erst mit I0041
/// (422 `zustand` bzw. `feld`), fremder `Origin` 403, unbekannter Channel 404, ohne Sitzung 401 — jeweils ohne
/// Wirkung; `DELETE` ohne aktiven Zustand 204 ohne Ereignis.
#[tokio::test]
async fn p8_abweisungen_ohne_wirkung() {
    let mut r = runde().await;
    let mut antworten = Vec::new();

    for zustand in ["schreibt", "denkt nach", "Tippt", "denkt_nach"] {
        let antwort = setzen(
            &r.absender,
            &r.instanz,
            Some(&r.anna),
            "allgemein",
            json!({ "zustand": zustand }),
        )
        .await;
        antworten.push((
            zustand.to_string(),
            antwort.status,
            antwort.json["regel"].clone(),
        ));
    }
    for koerper in [json!({}), json!({"zustand": 1}), json!(["tippt"])] {
        let antwort = setzen(
            &r.absender,
            &r.instanz,
            Some(&r.anna),
            "allgemein",
            koerper.clone(),
        )
        .await;
        antworten.push((
            koerper.to_string(),
            antwort.status,
            antwort.json["regel"].clone(),
        ));
    }
    let mit_gueltigkeit = setzen(
        &r.absender,
        &r.instanz,
        Some(&r.anna),
        "allgemein",
        json!({"zustand": "tippt", "gueltig_s": 30}),
    )
    .await;
    antworten.push((
        "gueltig_s".into(),
        mit_gueltigkeit.status,
        mit_gueltigkeit.json["regel"].clone(),
    ));
    for methode in [reqwest::Method::PUT, reqwest::Method::DELETE] {
        let antwort = aufrufen(
            &r.absender,
            &r.instanz,
            methode.clone(),
            Some(&r.anna),
            "allgemein",
            Some(json!({"zustand": "tippt"})),
            &[("Origin", "http://fremd.example")],
        )
        .await;
        antworten.push((format!("Origin {methode}"), antwort.status, Value::Null));
    }
    let unbekannt = setzen(
        &r.absender,
        &r.instanz,
        Some(&r.anna),
        "gibt-es-nicht",
        json!({"zustand": "tippt"}),
    )
    .await;
    antworten.push((
        "unbekannt".into(),
        unbekannt.status,
        unbekannt.json["fehler"].clone(),
    ));
    let ohne_sitzung = setzen(
        &r.absender,
        &r.instanz,
        None,
        "allgemein",
        json!({"zustand": "tippt"}),
    )
    .await;
    antworten.push(("ohne Sitzung".into(), ohne_sitzung.status, Value::Null));
    let geloescht = loeschen(&r.absender, &r.instanz, &r.anna, "allgemein").await;
    antworten.push(("DELETE".into(), geloescht.status, Value::Null));
    let unbekannt_loeschen = loeschen(&r.absender, &r.instanz, &r.anna, "gibt-es-nicht").await;
    antworten.push((
        "DELETE unbekannt".into(),
        unbekannt_loeschen.status,
        Value::Null,
    ));
    let wirkung = r.strom_ben.bis(STILLE, ist_aktivitaet).await;

    let regel = |r: &str| json!(r);
    assert_eq!(
        antworten,
        [
            ("schreibt".to_string(), 422, regel("zustand")),
            ("denkt nach".to_string(), 422, regel("zustand")),
            ("Tippt".to_string(), 422, regel("zustand")),
            ("denkt_nach".to_string(), 422, regel("zustand")),
            ("{}".to_string(), 422, regel("zustand")),
            (r#"{"zustand":1}"#.to_string(), 422, regel("zustand")),
            (r#"["tippt"]"#.to_string(), 422, regel("zustand")),
            ("gueltig_s".to_string(), 422, regel("feld")),
            ("Origin PUT".to_string(), 403, Value::Null),
            ("Origin DELETE".to_string(), 403, Value::Null),
            (
                "unbekannt".to_string(),
                404,
                json!("# gibt-es-nicht gibt es nicht.")
            ),
            ("ohne Sitzung".to_string(), 401, Value::Null),
            ("DELETE".to_string(), 204, Value::Null),
            ("DELETE unbekannt".to_string(), 404, Value::Null),
        ]
    );
    assert!(wirkung.is_none(), "{wirkung:?}");
}
