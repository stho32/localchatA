//! R00045 F0161 „Tippen sehen“, Serverteil (ADR-0051 P4, P5, P6, P8 ohne `gueltig_s`) — über das echte Programm:
//! `PUT`/`DELETE /api/channels/{name}/aktivitaet`, Ereignis `aktivitaet` im Strom der anderen, nie im eigenen.
//! Bis zum Lesen (B0696) prüft `ben` über seinen Strom, ob etwas gewirkt hat.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::protokoll::zeilen;
use gemeinsam::senden::{HEUTE, beitreten, ordner, senden, starten};
use gemeinsam::strom::{Empfangen, Strom, ist_wechsel};
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

/// P8 (B0695 schaltet `denkt_nach` und `gueltig_s` frei): falscher Wortlaut 422 `zustand`, `gueltig_s` 0, −1, 1.5,
/// `"30"`, „bald“, `null`, 121 → 422 `gueltigkeit`, ein weiteres Feld 422 `feld`, fremder `Origin` 403, unbekannter
/// Channel 404, ohne Sitzung 401 — jeweils ohne Wirkung; `DELETE` ohne aktiven Zustand 204 ohne Ereignis.
#[tokio::test]
async fn p8_abweisungen_ohne_wirkung() {
    let mut r = runde().await;
    let mut antworten = Vec::new();

    for zustand in ["schreibt", "denkt nach", "Tippt"] {
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
    for gueltig_s in [
        json!(0),
        json!(-1),
        json!(1.5),
        json!("30"),
        json!("bald"),
        Value::Null,
        json!(121),
        json!(500),
    ] {
        let antwort = setzen(
            &r.absender,
            &r.instanz,
            Some(&r.anna),
            "allgemein",
            json!({"zustand": "denkt_nach", "gueltig_s": gueltig_s}),
        )
        .await;
        antworten.push((
            format!("gueltig_s {gueltig_s}"),
            antwort.status,
            antwort.json["regel"].clone(),
        ));
    }
    let weiteres_feld = setzen(
        &r.absender,
        &r.instanz,
        Some(&r.anna),
        "allgemein",
        json!({"zustand": "tippt", "gueltig_s": 30, "text": "x"}),
    )
    .await;
    antworten.push((
        "weiteres Feld".into(),
        weiteres_feld.status,
        weiteres_feld.json["regel"].clone(),
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
            ("{}".to_string(), 422, regel("zustand")),
            (r#"{"zustand":1}"#.to_string(), 422, regel("zustand")),
            (r#"["tippt"]"#.to_string(), 422, regel("zustand")),
            ("gueltig_s 0".to_string(), 422, regel("gueltigkeit")),
            ("gueltig_s -1".to_string(), 422, regel("gueltigkeit")),
            ("gueltig_s 1.5".to_string(), 422, regel("gueltigkeit")),
            ("gueltig_s \"30\"".to_string(), 422, regel("gueltigkeit")),
            ("gueltig_s \"bald\"".to_string(), 422, regel("gueltigkeit")),
            ("gueltig_s null".to_string(), 422, regel("gueltigkeit")),
            ("gueltig_s 121".to_string(), 422, regel("gueltigkeit")),
            ("gueltig_s 500".to_string(), 422, regel("gueltigkeit")),
            ("weiteres Feld".to_string(), 422, regel("feld")),
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

/// Erst der Wechsel der Anwesenheit, dann das Ende der Aktivität von `kennung` (beide können im selben Stück kommen) —
/// mit der Zeit vom Auslöser bis zum Ende.
async fn ende_nach_wechsel(
    strom: &mut Strom,
    kennung: &'static str,
    status: &'static str,
    ausgeloest: Instant,
) -> (Option<Empfangen>, Option<Duration>) {
    let ab = strom.ereignisse.len();
    let ist_ende = move |e: &Empfangen| aktivitaet_von(kennung)(e) && e.data["zustand"].is_null();
    let wechsel = strom
        .bis(Duration::from_secs(5), ist_wechsel(kennung, status))
        .await;
    let schon = strom.ereignisse[ab..].iter().find(|e| ist_ende(e)).cloned();
    let ende = match schon {
        Some(ende) => Some(ende),
        None => strom.bis(Duration::from_secs(5), ist_ende).await,
    };
    let reihenfolge = ende
        .as_ref()
        .zip(wechsel.as_ref())
        .map(|(e, w)| e.ankunft >= w.ankunft);
    assert_ne!(reihenfolge, Some(false), "Ende nach dem Wechsel");
    (
        wechsel,
        ende.map(|e| e.ankunft.saturating_duration_since(ausgeloest)),
    )
}

/// B0694, P7 Mensch (ADR-0051 Regel 7 d, e; R00045 „Verlassen und offline eines Menschen“): `anna` (`art: mensch`, mit
/// Strom) tippt; ihr Strom wird geschlossen → `ben` liest erst `offline`, dann `aktivitaet` mit `zustand: null`, binnen
/// 2 s nach dem Schließen. `anna` verbindet neu, tippt und verlässt → `weg`, dann das Ende, binnen 1 s.
#[tokio::test]
async fn p7_offline_und_verlassen_eines_menschen_beenden_seinen_zustand() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[]);
    let absender = Absender::von("127.0.0.1");
    let anna = beitreten(&absender, &instanz.adresse, "anna", Some("mensch")).await;
    let ben = beitreten(&absender, &instanz.adresse, "ben", Some("mensch")).await;
    let mut strom_ben = Strom::oeffnen(&absender, &instanz.adresse, &ben).await;
    let strom_anna = Strom::oeffnen(&absender, &instanz.adresse, &anna).await;
    let anna_tippt = async |strom_ben: &mut Strom| {
        let gesetzt = setzen(
            &absender,
            &instanz,
            Some(&anna),
            "allgemein",
            json!({"zustand": "tippt"}),
        )
        .await;
        assert_eq!(gesetzt.status, 200, "{}", gesetzt.json);
        strom_ben
            .bis(Duration::from_secs(5), aktivitaet_von("anna@127.0.0.1"))
            .await
            .expect("gesetzt bei ben");
    };
    // Anlauf vorbei: ein `offline` aus der Zeit vor dem Strom ist gelesen, bevor der Strom schließt.
    strom_ben.mitlesen(Duration::from_millis(1_200)).await;
    anna_tippt(&mut strom_ben).await;

    drop(strom_anna);
    let geschlossen = Instant::now();
    let (offline, nach_offline) =
        ende_nach_wechsel(&mut strom_ben, "anna@127.0.0.1", "offline", geschlossen).await;
    let _strom_anna = Strom::oeffnen(&absender, &instanz.adresse, &anna).await;
    anna_tippt(&mut strom_ben).await;
    let verlassen = api_antwort(
        absender
            .anfrage(
                reqwest::Method::POST,
                format!("{}api/verlassen", instanz.adresse),
            )
            .header("X-LocalchatA-Sitzung", &anna),
    )
    .await;
    let verlassen_um = Instant::now();
    let (weg, nach_weg) =
        ende_nach_wechsel(&mut strom_ben, "anna@127.0.0.1", "weg", verlassen_um).await;

    assert!(offline.is_some(), "offline bei ben");
    let nach_offline = nach_offline.expect("Ende nach dem Schließen");
    println!("Ende nach Schließen: {} ms", nach_offline.as_millis());
    assert!(nach_offline <= Duration::from_secs(2), "{nach_offline:?}");
    assert!(
        (200..300).contains(&verlassen.status),
        "{} {}",
        verlassen.status,
        verlassen.json
    );
    assert!(weg.is_some(), "weg bei ben");
    let nach_weg = nach_weg.expect("Ende nach dem Verlassen");
    println!("Ende nach Verlassen: {} ms", nach_weg.as_millis());
    assert!(nach_weg <= Duration::from_secs(1), "{nach_weg:?}");
}

/// B0694 Sperrfolge Anwesenheit → Aktivität ohne Verklemmung (Gnadenfrist 200 ms): acht Menschen öffnen je einen
/// Strom und setzen in Schleife; mittendrin schließt ihr Strom (offline beendet unter der Sperre der Anwesenheit,
/// während die anderen weiter setzen), später schweigen sie 1,5 s (weg nach der Gnadenfrist) und setzen danach wieder.
/// Alles endet binnen 20 s, jede Antwort ist 200, 401 (Sitzung weg) oder 429, und die Instanz antwortet danach.
#[tokio::test]
async fn sperrfolge_offline_weg_und_setzen_gleichzeitig_ohne_verklemmung() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[("LOCALCHATA_GNADENFRIST_MS", "200")]);
    let absender = std::sync::Arc::new(Absender::von("127.0.0.1"));
    let mut aufgaben = Vec::new();
    for n in 0..8_u64 {
        let token = beitreten(
            &absender,
            &instanz.adresse,
            &format!("m{n}"),
            Some("mensch"),
        )
        .await;
        let (absender, adresse) = (std::sync::Arc::clone(&absender), instanz.adresse.clone());
        aufgaben.push(tokio::spawn(async move {
            let mut strom = Some(Strom::oeffnen(&absender, &adresse, &token).await);
            let mut status = Vec::new();
            for runde in 0..30 {
                if runde == 5 + n {
                    strom.take();
                }
                if runde == 20 {
                    tokio::time::sleep(Duration::from_millis(1_500)).await;
                }
                let antwort = api_antwort(
                    absender
                        .anfrage(
                            reqwest::Method::PUT,
                            format!("{adresse}api/channels/allgemein/aktivitaet"),
                        )
                        .header("X-LocalchatA-Sitzung", &token)
                        .header("Content-Type", "application/json")
                        .body(r#"{"zustand":"tippt"}"#),
                )
                .await;
                status.push(antwort.status);
                tokio::time::sleep(Duration::from_millis(40 * (n % 3))).await;
            }
            status
        }));
    }

    let alle = tokio::time::timeout(Duration::from_secs(20), async {
        let mut alle = Vec::new();
        for aufgabe in aufgaben {
            alle.extend(aufgabe.await.expect("Aufgabe"));
        }
        alle
    })
    .await
    .expect("keine Verklemmung");
    let (gesund, _) = gesundheit(&instanz.adresse).await;

    assert_eq!(alle.len(), 240);
    assert!(alle.iter().all(|s| [200, 401, 429].contains(s)), "{alle:?}");
    assert!(alle.contains(&401), "Gnadenfrist abgelaufen: {alle:?}");
    assert_eq!(gesund, 200);
}

/// Die Dauer zwischen `jetzt` und `bis` einer Antwort auf das Setzen.
fn gueltig(antwort: &ApiAntwort) -> chrono::TimeDelta {
    let zeit = |wert: &Value| {
        chrono::DateTime::parse_from_rfc3339(wert.as_str().expect("Zeit")).expect("RFC 3339")
    };
    zeit(&antwort.json["bis"]) - zeit(&antwort.json["jetzt"])
}

/// B0695 (ADR-0051 Regeln 2, 3; R00045 I0041 „Setzen und Anzeige“, „Ablauf“; Mutante M5): `denkt_nach` ohne Angabe
/// gilt 30 s, `tippt` ohne Angabe 6 s, `gueltig_s` 10 → 10 s, 120 → 120 s; 121 wird nicht begrenzt, sondern ohne Wirkung
/// abgewiesen; das Ereignis bei `ben` nennt `denkt_nach` mit demselben `bis`.
#[tokio::test]
async fn i0041_denkt_nach_und_gueltigkeit_an_der_route() {
    let mut r = runde().await;
    let setzen_mit =
        |koerper: Value| setzen(&r.absender, &r.instanz, Some(&r.anna), "allgemein", koerper);

    let denkt_nach = setzen_mit(json!({"zustand": "denkt_nach"})).await;
    let bei_ben = r
        .strom_ben
        .bis(Duration::from_secs(5), aktivitaet_von("anna@127.0.0.1"))
        .await
        .expect("bei ben");
    let tippt = setzen_mit(json!({"zustand": "tippt"})).await;
    let zehn = setzen_mit(json!({"zustand": "denkt_nach", "gueltig_s": 10})).await;
    let hundertzwanzig = setzen_mit(json!({"zustand": "denkt_nach", "gueltig_s": 120})).await;
    while r.strom_ben.bis(STILLE, ist_aktivitaet).await.is_some() {}
    let zu_lang = setzen_mit(json!({"zustand": "tippt", "gueltig_s": 121})).await;
    let nach_zu_lang = r.strom_ben.bis(STILLE, ist_aktivitaet).await;

    assert_eq!(denkt_nach.status, 200, "{}", denkt_nach.json);
    assert_eq!(denkt_nach.json["zustand"], "denkt_nach");
    assert_eq!(gueltig(&denkt_nach), chrono::TimeDelta::seconds(30));
    assert_eq!(
        (&bei_ben.data["zustand"], &bei_ben.data["bis"]),
        (&json!("denkt_nach"), &denkt_nach.json["bis"])
    );
    assert_eq!(gueltig(&tippt), chrono::TimeDelta::seconds(6));
    assert_eq!(gueltig(&zehn), chrono::TimeDelta::seconds(10));
    assert_eq!(gueltig(&hundertzwanzig), chrono::TimeDelta::seconds(120));
    assert_eq!(
        (zu_lang.status, &zu_lang.json["regel"]),
        (422, &json!("gueltigkeit")),
        "nicht begrenzt: {}",
        zu_lang.json
    );
    assert!(nach_zu_lang.is_none(), "ohne Wirkung: {nach_zu_lang:?}");
}

/// B0695, P7 Agent (ADR-0051 Regel 7 e, F5; R00045 I0041 US-9; Mutante M6): `claude` tritt als Agent bei, hat keinen
/// Strom und setzt `denkt_nach` → `ben` liest `offline` von `claude`, aber kein Ende — auch nach 5 s nicht; `claude`
/// verlässt → `weg`, dann `zustand: null` binnen 1 s.
#[tokio::test]
async fn p7_ein_agent_ohne_strom_denkt_weiter_nach_bis_er_verlaesst() {
    let mut r = runde().await;
    let claude = beitreten(&r.absender, &r.instanz.adresse, "claude", Some("agent")).await;
    let gesetzt = setzen(
        &r.absender,
        &r.instanz,
        Some(&claude),
        "allgemein",
        json!({"zustand": "denkt_nach"}),
    )
    .await;
    r.strom_ben
        .bis(Duration::from_secs(5), aktivitaet_von("claude@127.0.0.1"))
        .await
        .expect("gesetzt bei ben");
    let gesetzt_um = Instant::now();

    let offline = r
        .strom_ben
        .bis(
            Duration::from_secs(5),
            ist_wechsel("claude@127.0.0.1", "offline"),
        )
        .await;
    tokio::time::sleep_until(gesetzt_um + Duration::from_secs(5)).await;
    r.strom_ben.mitlesen(Duration::from_millis(100)).await;
    let ende_vor_dem_verlassen: Vec<_> = r
        .strom_ben
        .ereignisse
        .iter()
        .filter(|e| aktivitaet_von("claude@127.0.0.1")(e) && e.data["zustand"].is_null())
        .cloned()
        .collect();
    let verlassen = api_antwort(
        r.absender
            .anfrage(
                reqwest::Method::POST,
                format!("{}api/verlassen", r.instanz.adresse),
            )
            .header("X-LocalchatA-Sitzung", &claude),
    )
    .await;
    let verlassen_um = Instant::now();
    let (weg, nach_weg) =
        ende_nach_wechsel(&mut r.strom_ben, "claude@127.0.0.1", "weg", verlassen_um).await;

    assert_eq!(gesetzt.status, 200, "{}", gesetzt.json);
    assert!(offline.is_some(), "claude ohne Strom ist offline");
    assert!(
        ende_vor_dem_verlassen.is_empty(),
        "{ende_vor_dem_verlassen:?}"
    );
    assert!(
        (200..300).contains(&verlassen.status),
        "{}",
        verlassen.status
    );
    assert!(weg.is_some(), "weg bei ben");
    let nach_weg = nach_weg.expect("Ende nach dem Verlassen");
    assert!(nach_weg <= Duration::from_secs(1), "{nach_weg:?}");
}

/// B0695 (ADR-0051 Regel 2): höchstens 16 Zustände je Kennung — das 17. Gespräch ist 422 `aktivitaeten` ohne Wirkung.
#[tokio::test]
async fn ein_siebzehntes_gespraech_ist_422_aktivitaeten() {
    let wurzel = ordner();
    let gespraeche: Vec<String> = (0..17).map(|n| format!("g{n}")).collect();
    for name in &gespraeche {
        std::fs::create_dir_all(wurzel.path().join("channels").join(name)).expect("Channel");
    }
    let instanz = starten(wurzel.path(), &[]);
    let absender = Absender::von("127.0.0.1");
    let claude = beitreten(&absender, &instanz.adresse, "claude", None).await;
    let mut status = Vec::new();
    for gespraech in &gespraeche {
        let antwort = setzen(
            &absender,
            &instanz,
            Some(&claude),
            gespraech,
            json!({"zustand": "denkt_nach", "gueltig_s": 60}),
        )
        .await;
        status.push((antwort.status, antwort.json["regel"].clone()));
    }

    assert!(status[..16].iter().all(|(s, _)| *s == 200), "{status:?}");
    assert_eq!(status[16], (422, json!("aktivitaeten")));
}
