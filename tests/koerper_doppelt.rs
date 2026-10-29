//! R00032 F0123 „Doppelte Schlüssel im Körper abgewiesen“ (B0549; ADR-0034 Ergänzung „Karte #47“) — das echte
//! Programm auf Stufe debug: Senden, Frist setzen, Beitreten und Channel anlegen weisen einen doppelten
//! Schlüssel der obersten Ebene mit 422 `feld`/`doppelt` ab, nichts ändert sich, nichts wird gemerkt.

mod gemeinsam;

use gemeinsam::frist::channel_in_liste;
use gemeinsam::protokoll::{alle_zeilen, wert};
use gemeinsam::senden::{beitreten, ordner, senden, starten, tagesinhalt, verlauf};
use gemeinsam::strom::Strom;
use gemeinsam::{Absender, ApiAntwort, api_antwort, gesundheit};
use serde_json::{Value, json};
use std::time::{Duration, Instant};

const DOPPELT_TEXT: &str = r#"{"text":"a","text":"b"}"#;

/// Beliebige Methode und Pfad mit rohem JSON-Körper, wie ihn ein fremdes Werkzeug schickt.
async fn roh(
    absender: &Absender,
    methode: reqwest::Method,
    basis: &str,
    pfad: &str,
    token: Option<&str>,
    koerper: &str,
) -> ApiAntwort {
    let mut anfrage = absender
        .anfrage(methode, format!("{basis}{pfad}"))
        .header("Content-Type", "application/json")
        .body(koerper.to_string());
    if let Some(token) = token {
        anfrage = anfrage.header("X-LocalchatA-Sitzung", token);
    }
    api_antwort(anfrage).await
}

fn ist_doppelt(antwort: &ApiAntwort, feld: &str) {
    assert_eq!(antwort.status, 422, "{}", antwort.json);
    assert_eq!(antwort.json["regel"], "feld", "{}", antwort.json);
    assert_eq!(antwort.json["feld"], feld, "{}", antwort.json);
    assert_eq!(antwort.json["grund"], "doppelt", "{}", antwort.json);
    assert_eq!(
        antwort.json["fehler"],
        format!("Das Feld {feld} steht zweimal im Körper — jedes Feld nur einmal schicken.")
    );
}

fn sitzungen(json: &Value) -> u64 {
    json["sitzungen"].as_u64().expect("sitzungen")
}

/// Abnahme-Probe der ADR über alle vier Routen, wörtlich: 422 `feld`/`doppelt` (beim Senden auch vor der 404 und
/// bei dekodiert gleichem Schlüssel), `{"x":1}` nennt `x` ohne `grund`, nichts geändert, je Abweisung genau eine
/// DEBUG-Zeile `feld_doppelt` ohne `anna`, `bob`, `neu2`; danach antworten gültige Körper wie vorher.
#[tokio::test]
async fn doppelte_schluessel_werden_auf_allen_vier_routen_abgewiesen_und_nichts_aendert_sich() {
    const POST: reqwest::Method = reqwest::Method::POST;
    const PUT: reqwest::Method = reqwest::Method::PUT;
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[("LOCALCHATA_PROTOKOLL_STUFE", "debug")]);
    let (basis, dora) = (&instanz.adresse, Absender::von("127.0.0.1"));
    let token = beitreten(&dora, basis, "dora", None).await;
    let mut strom = Strom::oeffnen(&dora, basis, &token).await;
    let frist_vorher = channel_in_liste(&dora, &instanz, &token, "allgemein")
        .await
        .0;
    let sitzungen_vorher = sitzungen(&gesundheit(basis).await.1);
    let nachrichten = "api/channels/allgemein/nachrichten";

    let doppelt = roh(&dora, POST, basis, nachrichten, Some(&token), DOPPELT_TEXT).await;
    let dekodiert = roh(
        &dora,
        POST,
        basis,
        nachrichten,
        Some(&token),
        r#"{"text":"a","text":"b"}"#,
    )
    .await;
    let zweites = roh(
        &dora,
        POST,
        basis,
        nachrichten,
        Some(&token),
        r#"{"text":"a","x":1,"x":2}"#,
    )
    .await;
    let unbekannt = roh(&dora, POST, basis, nachrichten, Some(&token), r#"{"x":1}"#).await;
    let ohne_channel = roh(
        &dora,
        POST,
        basis,
        "api/channels/gibtsnicht/nachrichten",
        Some(&token),
        DOPPELT_TEXT,
    )
    .await;
    let frist = roh(
        &dora,
        PUT,
        basis,
        "api/channels/allgemein/frist",
        Some(&token),
        r#"{"tage":1,"tage":7}"#,
    )
    .await;
    let beitritt = roh(
        &dora,
        POST,
        basis,
        "api/beitreten",
        None,
        r#"{"name":"anna","name":"bob"}"#,
    )
    .await;
    let anlegen = roh(
        &dora,
        POST,
        basis,
        "api/channels",
        Some(&token),
        r#"{"name":"neu","name":"neu2"}"#,
    )
    .await;

    ist_doppelt(&doppelt, "text");
    ist_doppelt(&dekodiert, "text");
    ist_doppelt(&zweites, "x");
    assert_eq!(unbekannt.status, 422, "{}", unbekannt.json);
    assert_eq!(unbekannt.json["regel"], "feld");
    assert_eq!(unbekannt.json["feld"], "x");
    assert!(unbekannt.json.get("grund").is_none(), "{}", unbekannt.json);
    ist_doppelt(&ohne_channel, "text");
    ist_doppelt(&frist, "tage");
    ist_doppelt(&beitritt, "name");
    ist_doppelt(&anlegen, "name");
    assert!(tagesinhalt(wurzel.path(), "allgemein").is_empty());
    assert_eq!(
        verlauf(&dora, basis, &token, "allgemein").await["nachrichten"],
        json!([])
    );
    assert!(
        strom
            .bis(Duration::from_millis(400), |e| e.data["art"] == "nachricht")
            .await
            .is_none(),
        "eine abgewiesene Nachricht erreicht den Strom"
    );
    assert_eq!(
        channel_in_liste(&dora, &instanz, &token, "allgemein")
            .await
            .0,
        frist_vorher
    );
    assert_eq!(sitzungen(&gesundheit(basis).await.1), sitzungen_vorher);
    assert!(beitritt.json.get("token").is_none(), "{}", beitritt.json);
    for ordnername in ["neu", "neu2"] {
        assert!(!wurzel.path().join("channels").join(ordnername).exists());
    }
    let zeilen = alle_zeilen(wurzel.path());
    let gemeldet: Vec<&String> = zeilen
        .iter()
        .filter(|zeile| wert(zeile, "grund").as_deref() == Some("feld_doppelt"))
        .collect();
    assert_eq!(gemeldet.len(), 7, "{gemeldet:#?}");
    for zeile in &gemeldet {
        assert!(zeile.contains(" DEBUG "), "{zeile}");
        assert_eq!(wert(zeile, "status").as_deref(), Some("422"), "{zeile}");
    }
    for fremd in ["anna", "bob", "neu2"] {
        assert!(
            zeilen.iter().all(|zeile| !zeile.contains(fremd)),
            "{fremd} steht im Protokoll"
        );
    }

    let gueltig = senden(&dora, basis, &token, "allgemein", "ok").await;
    let fest = roh(
        &dora,
        PUT,
        basis,
        "api/channels/allgemein/frist",
        Some(&token),
        r#"{"tage":7}"#,
    )
    .await;
    let neu = roh(
        &dora,
        POST,
        basis,
        "api/beitreten",
        None,
        r#"{"name":"anna"}"#,
    )
    .await;
    let channel = roh(
        &dora,
        POST,
        basis,
        "api/channels",
        Some(&token),
        r#"{"name":"neu"}"#,
    )
    .await;
    assert_eq!(
        [gueltig.status, fest.status, neu.status, channel.status],
        [201, 200, 201, 201],
        "{} {} {} {}",
        gueltig.json,
        fest.json,
        neu.json,
        channel.json
    );
}

/// Kosten wie jeder Körperfehler: 40 doppelte Körper nacheinander → zuerst nur 422 (Vorrat 30 + Nachschub 1 je s),
/// danach nur 429 `grenze: kennung`. Mutante „doppelt kostet 0“ → 40 × 422.
#[tokio::test]
async fn jede_doppelt_abweisung_kostet_eine_einheit_am_schreibkonto() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[]);
    let (basis, dora) = (&instanz.adresse, Absender::von("127.0.0.1"));
    let token = beitreten(&dora, basis, "dora", None).await;

    let beginn = Instant::now();
    let mut status = Vec::new();
    let mut letzte = None;
    for _ in 0..40 {
        let antwort = roh(
            &dora,
            reqwest::Method::POST,
            basis,
            "api/channels/allgemein/nachrichten",
            Some(&token),
            DOPPELT_TEXT,
        )
        .await;
        status.push(antwort.status);
        letzte = Some(antwort);
    }
    let dauer = usize::try_from(beginn.elapsed().as_millis().div_ceil(1_000)).expect("klein");

    let zahl_422 = status.iter().filter(|s| **s == 422).count();
    assert!(
        (30..=30 + dauer).contains(&zahl_422),
        "{zahl_422} × 422: {status:?}"
    );
    assert!(status[..zahl_422].iter().all(|s| *s == 422), "{status:?}");
    assert!(status[zahl_422..].iter().all(|s| *s == 429), "{status:?}");
    assert_eq!(letzte.expect("Antwort").json["grenze"], "kennung");
}

/// Nichts merkt sich: nach der 422 auf den doppelten Körper mit `Idempotency-Key: k47` antwortet `{"text":"a"}` mit
/// demselben Schlüssel 201, nicht 422 `schluessel_anders`.
#[tokio::test]
async fn eine_doppelt_abweisung_verbraucht_den_wiederholschluessel_nicht() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[]);
    let (basis, dora) = (&instanz.adresse, Absender::von("127.0.0.1"));
    let token = beitreten(&dora, basis, "dora", None).await;
    let mit_schluessel = |koerper: &'static str| {
        api_antwort(
            dora.anfrage(
                reqwest::Method::POST,
                format!("{basis}api/channels/allgemein/nachrichten"),
            )
            .header("X-LocalchatA-Sitzung", &token)
            .header("Content-Type", "application/json")
            .header("Idempotency-Key", "k47")
            .body(koerper),
        )
    };

    let abgewiesen = mit_schluessel(DOPPELT_TEXT).await;
    let gesendet = mit_schluessel(r#"{"text":"a"}"#).await;

    assert_eq!(abgewiesen.status, 422, "{}", abgewiesen.json);
    assert_eq!(gesendet.status, 201, "{}", gesendet.json);
}
