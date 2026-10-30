//! R00045 F0161 „Tippen sehen“, Serverteil (ADR-0051 P4, P5, P6, P8 ohne `gueltig_s`) — über das echte Programm:
//! `PUT`/`DELETE /api/channels/{name}/aktivitaet`, Ereignis `aktivitaet` im Strom der anderen, nie im eigenen.
//! Bis zum Lesen (B0696) prüft `ben` über seinen Strom, ob etwas gewirkt hat.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::protokoll::zeilen;
use gemeinsam::senden::{HEUTE, beitreten, ordner, senden, starten};
use gemeinsam::strom::{Empfangen, Strom};
use gemeinsam::{Absender, ApiAntwort, Instanz, api_antwort, gesundheit};
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
    ben: String,
    cora: String,
    strom_ben: Strom,
}

async fn runde() -> Runde {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[("LOCALCHATA_PROTOKOLL_STUFE", "debug")]);
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
        ben,
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

/// `anna` setzt `tippt` in `channel`, und `ben` hat es gelesen.
async fn anna_tippt_bei_ben(r: &mut Runde, channel: &str) {
    let gesetzt = setzen(
        &r.absender,
        &r.instanz,
        Some(&r.anna),
        channel,
        json!({"zustand": "tippt"}),
    )
    .await;
    assert_eq!(gesetzt.status, 200, "{}", gesetzt.json);
    r.strom_ben
        .bis(Duration::from_secs(5), aktivitaet_von("anna@127.0.0.1"))
        .await
        .expect("gesetzt bei ben");
}

/// Die Arten, die `ben` ab `ab` gelesen hat, ohne Herzschläge; `aktivitaet` mit Channel und Zustand.
fn gelesen_ab(strom: &Strom, ab: usize) -> Vec<String> {
    strom.ereignisse[ab..]
        .iter()
        .filter(|e| e.event != "herzschlag")
        .map(|e| match e.data["art"].as_str() {
            Some("aktivitaet") => format!("aktivitaet {} {}", e.data["channel"], e.data["zustand"]),
            Some("nachricht") => format!("nachricht {}", e.data["channel"]),
            art => format!("{art:?}"),
        })
        .collect()
}

/// B0687, P4 zweiter Satz (Mutante M7): `anna` sendet → `ben` liest `nachricht`, danach `aktivitaet` mit
/// `zustand: null`, in dieser Reihenfolge; eine Nachricht in `# technik` beendet den Zustand in `# allgemein` nicht.
#[tokio::test]
async fn p4_senden_beendet_den_zustand_nach_der_nachricht_nur_im_gespraech() {
    let mut r = runde().await;
    anna_tippt_bei_ben(&mut r, "allgemein").await;
    let ab = r.strom_ben.ereignisse.len();

    let gesendet = senden(
        &r.absender,
        &r.instanz.adresse,
        &r.anna,
        "allgemein",
        "fertig",
    )
    .await;
    r.strom_ben
        .bis(Duration::from_secs(5), ist_aktivitaet)
        .await
        .expect("Ende bei ben");
    let in_allgemein = gelesen_ab(&r.strom_ben, ab);
    anna_tippt_bei_ben(&mut r, "allgemein").await;
    let ab = r.strom_ben.ereignisse.len();
    let in_technik = senden(
        &r.absender,
        &r.instanz.adresse,
        &r.anna,
        "technik",
        "anderswo",
    )
    .await;
    r.strom_ben
        .bis(Duration::from_secs(5), |e| e.data["art"] == "nachricht")
        .await
        .expect("Nachricht in technik");
    let nach_technik = r.strom_ben.bis(STILLE, ist_aktivitaet).await;

    assert_eq!((gesendet.status, in_technik.status), (201, 201));
    assert_eq!(
        in_allgemein,
        ["nachricht \"allgemein\"", "aktivitaet \"allgemein\" null"]
    );
    assert!(
        nach_technik.is_none(),
        "{nach_technik:?} nach {:?}",
        gelesen_ab(&r.strom_ben, ab)
    );
}

/// Alle Dateien unter `channels/` mit Inhalt — gleich heißt: dieselben Dateien mit denselben Bytes (wie `sha256sum`).
fn channels_abbild(ordner: &std::path::Path) -> Vec<(std::path::PathBuf, Vec<u8>)> {
    let mut offen = vec![ordner.join("channels")];
    let mut dateien = Vec::new();
    while let Some(verzeichnis) = offen.pop() {
        for eintrag in std::fs::read_dir(&verzeichnis)
            .expect("Verzeichnis")
            .flatten()
        {
            let pfad = eintrag.path();
            if pfad.is_dir() {
                offen.push(pfad);
            } else {
                let inhalt = std::fs::read(&pfad).expect("Datei");
                dateien.push((pfad, inhalt));
            }
        }
    }
    dateien.sort();
    dateien
}

/// B0687, P6 Flüchtig: 30 Aufrufe Setzen und Löschen über 10 s → `channels/` Byte für Byte gleich, im
/// Betriebsprotokoll (Stufe Debug) keine Zeile mit `aktivitaet`; `ben` verbindet neu → erstes Ereignis `teilnehmer`,
/// kein `aktivitaet` vor dem nächsten Setzen, obwohl `anna` gerade tippt.
#[tokio::test]
async fn p6_nichts_bleibt_zurueck_und_ein_neuer_strom_holt_nichts_nach() {
    let mut r = runde().await;
    senden(
        &r.absender,
        &r.instanz.adresse,
        &r.ben,
        "allgemein",
        "vorher",
    )
    .await;
    let vorher = channels_abbild(&r.instanz.ordner);
    let takt = Duration::from_millis(667);

    let mut staende = Vec::new();
    for _ in 0..15 {
        let gesetzt = setzen(
            &r.absender,
            &r.instanz,
            Some(&r.anna),
            "allgemein",
            json!({"zustand": "tippt"}),
        )
        .await;
        let geloescht = loeschen(&r.absender, &r.instanz, &r.anna, "allgemein").await;
        staende.push((gesetzt.status, geloescht.status));
        tokio::time::sleep(takt).await;
    }
    anna_tippt_bei_ben(&mut r, "allgemein").await;
    drop(std::mem::replace(
        &mut r.strom_ben,
        Strom::oeffnen(&r.absender, &r.instanz.adresse, &r.ben).await,
    ));
    let erstes = r
        .strom_ben
        .bis(Duration::from_secs(5), |e| e.event != "herzschlag")
        .await
        .expect("erstes Ereignis");
    let vor_dem_setzen = r.strom_ben.bis(STILLE, ist_aktivitaet).await;
    anna_tippt_bei_ben(&mut r, "allgemein").await;
    let nachher = channels_abbild(&r.instanz.ordner);
    let mit_aktivitaet: Vec<String> = zeilen(&r.instanz.ordner, HEUTE)
        .into_iter()
        .filter(|zeile| zeile.contains("aktivitaet"))
        .collect();

    assert!(
        staende.iter().all(|stand| *stand == (200, 204)),
        "{staende:?}"
    );
    assert_eq!(nachher, vorher, "channels/ unverändert");
    assert!(
        !zeilen(&r.instanz.ordner, HEUTE).is_empty(),
        "Protokoll auf Stufe Debug geschrieben"
    );
    assert!(mit_aktivitaet.is_empty(), "{mit_aktivitaet:#?}");
    assert_eq!(erstes.data["art"], "teilnehmer");
    assert!(vor_dem_setzen.is_none(), "{vor_dem_setzen:?}");
}

/// B0693, P9 Bremse an der Route (ADR-0051 Regeln 9, 11; Mutanten M9, M10): 11 × `PUT` von `anna` in `# allgemein` in
/// unter 1 s → zehnmal 200, einmal 429 `grenze: "gespraech"` mit `Retry-After: 1` und dem Satz der ADR; die Anzeige
/// bei `ben` bleibt (kein Ende); nach 1 s wieder 200; zehn Meldungen in `# technik` gehen durch; die Plattenquote ist
/// unberührt; danach 30 Nachrichten von `anna` in einem Zug → alle 201 (Schreibkonto unberührt, Vorrat 30); im
/// Protokoll genau eine Beginn-Zeile `WARNING aktivitaet` mit `kennung`, `channel`, `grenze`.
#[tokio::test]
async fn p9_die_elfte_meldung_ist_429_die_anzeige_bleibt_senden_bleibt_frei() {
    let mut r = runde().await;
    let quote_vorher = gesundheit(&r.instanz.adresse).await.1["plattenquote"].clone();
    let tippt = || json!({"zustand": "tippt"});

    let mut zehn = Vec::new();
    for _ in 0..10 {
        zehn.push(setzen(&r.absender, &r.instanz, Some(&r.anna), "allgemein", tippt()).await);
    }
    let elfte = setzen(&r.absender, &r.instanz, Some(&r.anna), "allgemein", tippt()).await;
    let gebremst_um = Instant::now();
    r.strom_ben
        .bis(Duration::from_secs(5), aktivitaet_von("anna@127.0.0.1"))
        .await
        .expect("gesetzt bei ben");
    while r.strom_ben.bis(STILLE, ist_aktivitaet).await.is_some() {}
    let ende_bei_ben: Vec<_> = r
        .strom_ben
        .ereignisse
        .iter()
        .filter(|e| ist_aktivitaet(e) && e.data["zustand"].is_null())
        .map(|e| e.data.clone())
        .collect();
    tokio::time::sleep_until(gebremst_um + Duration::from_secs(1)).await;
    let nach_einer_sekunde =
        setzen(&r.absender, &r.instanz, Some(&r.anna), "allgemein", tippt()).await;
    let mut in_technik = Vec::new();
    for _ in 0..10 {
        in_technik.push(
            setzen(&r.absender, &r.instanz, Some(&r.anna), "technik", tippt())
                .await
                .status,
        );
    }
    let quote_nachher = gesundheit(&r.instanz.adresse).await.1["plattenquote"].clone();
    let mut gesendet = Vec::new();
    for n in 0..30 {
        gesendet.push(
            senden(
                &r.absender,
                &r.instanz.adresse,
                &r.anna,
                "allgemein",
                &format!("n{n}"),
            )
            .await
            .status,
        );
    }
    let beginn: Vec<String> = zeilen(&r.instanz.ordner, HEUTE)
        .into_iter()
        .filter(|zeile| zeile.contains(" WARNING aktivitaet ") && zeile.contains("Episode beginnt"))
        .collect();

    assert!(zehn.iter().all(|a| a.status == 200), "{zehn:?}");
    assert_eq!(elfte.status, 429, "{}", elfte.json);
    assert_eq!(
        elfte
            .kopf
            .get("retry-after")
            .and_then(|wert| wert.to_str().ok()),
        Some("1")
    );
    assert_eq!(
        elfte.json,
        json!({
            "fehler": "Zu viele Aktivitätsmeldungen von anna@127.0.0.1 in # allgemein — wieder möglich in 1 s.",
            "grenze": "gespraech",
            "wieder_in_ms": elfte.json["wieder_in_ms"]
        })
    );
    let wieder_in_ms = elfte.json["wieder_in_ms"].as_u64().expect("wieder_in_ms");
    assert!((1..=1_000).contains(&wieder_in_ms), "{wieder_in_ms}");
    assert!(ende_bei_ben.is_empty(), "Anzeige bleibt: {ende_bei_ben:?}");
    assert_eq!(
        nach_einer_sekunde.status, 200,
        "{}",
        nach_einer_sekunde.json
    );
    assert_eq!(in_technik, [200; 10]);
    assert_eq!(quote_nachher, quote_vorher, "Plattenquote unberührt");
    assert_eq!(gesendet, [201; 30], "Schreibkonto unberührt");
    assert_eq!(beginn.len(), 1, "{beginn:#?}");
    assert!(
        beginn[0].ends_with(
            "Aktivität gebremst: zu viele Meldungen — Episode beginnt | kennung=anna@127.0.0.1 channel=allgemein grenze=gespraech"
        ),
        "{beginn:#?}"
    );
}

/// B0693, R00045 „Bremse“ zweiter Satz (Mutante M9 „Eimer je Herkunft aus“): 120 Meldungen einer Adresse über 12
/// Kennungen × 10 Gespräche gehen; die nächste wird spätestens kurz danach mit `grenze: "herkunft"` und dem Satz für
/// die Adresse abgewiesen (während der Folge fließen 12 je Sekunde nach — gezählt wird bis zur ersten 429).
#[tokio::test]
async fn bremse_je_herkunft_nach_120_meldungen_einer_adresse() {
    let wurzel = ordner();
    let gespraeche: Vec<String> = (0..10).map(|n| format!("c{n}")).collect();
    for name in &gespraeche {
        std::fs::create_dir_all(wurzel.path().join("channels").join(name)).expect("Channel");
    }
    let instanz = starten(wurzel.path(), &[]);
    let absender = Absender::von("127.0.0.1");
    let mut kennungen = Vec::new();
    for n in 0..13 {
        kennungen.push(beitreten(&absender, &instanz.adresse, &format!("k{n}"), None).await);
    }
    let tippt = || json!({"zustand": "tippt"});

    let mut erste = Vec::new();
    for token in &kennungen[..12] {
        for gespraech in &gespraeche {
            erste.push(
                setzen(&absender, &instanz, Some(token), gespraech, tippt())
                    .await
                    .status,
            );
        }
    }
    let mut weitere = Vec::new();
    let gebremst = loop {
        let antwort = setzen(
            &absender,
            &instanz,
            Some(&kennungen[12]),
            &gespraeche[weitere.len() % 10],
            tippt(),
        )
        .await;
        if antwort.status != 200 || weitere.len() >= 40 {
            break antwort;
        }
        weitere.push(antwort.status);
    };

    assert_eq!(erste, [200; 120]);
    assert_eq!(
        gebremst.status,
        429,
        "nach {} weiteren: {}",
        weitere.len(),
        gebremst.json
    );
    assert!(
        weitere.len() < 12,
        "höchstens der Nachschub einer Sekunde: {}",
        weitere.len()
    );
    assert_eq!(gebremst.json["grenze"], "herkunft");
    let gespraech = &gespraeche[weitere.len() % 10];
    let wieder_in_ms = gebremst.json["wieder_in_ms"]
        .as_u64()
        .expect("wieder_in_ms");
    assert!((1..=84).contains(&wieder_in_ms), "{wieder_in_ms}");
    assert_eq!(
        gebremst.json["fehler"],
        format!(
            "Zu viele Aktivitätsmeldungen von 127.0.0.1 (alle Teilnehmer dieser Adresse zusammen) in # {gespraech} — wieder möglich in 1 s."
        )
    );
}
