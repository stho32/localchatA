//! R00025 F0087 B0495 — Regressionen der Geltung und des Protokolls über das echte Programm (Debug-Build;
//! Angriffsmatrix ADR-0034 Ergänzung „Karte #36 Angriffsmatrix“ Zeilen 17, 23–26, E2): was der security-reviewer im
//! Release-Build so geprobt hat, halten diese Tests fest. Kontrollpaar K1 hält
//! `senden_wiederholung::us1_dieselbe_nachricht_mit_demselben_schluessel_ist_200_ohne_satz_und_ohne_ereignis`.

mod gemeinsam;

use gemeinsam::senden::{beitreten, ordner, starten, tagesinhalt};
use gemeinsam::{Absender, ApiAntwort, api_antwort};
use serde_json::json;
use std::time::Duration;

/// `POST …/nachrichten` mit Typ, Körper, optional Token und dem Kopf `Idempotency-Key`.
async fn roh_mit_schluessel(
    absender: &Absender,
    basis: &str,
    token: Option<&str>,
    channel: &str,
    typ: &str,
    koerper: Vec<u8>,
    schluessel: &str,
) -> ApiAntwort {
    let mut anfrage = absender
        .anfrage(
            reqwest::Method::POST,
            format!("{basis}api/channels/{channel}/nachrichten"),
        )
        .header("Content-Type", typ)
        .header("Idempotency-Key", schluessel)
        .body(koerper);
    if let Some(token) = token {
        anfrage = anfrage.header("X-LocalchatA-Sitzung", token);
    }
    api_antwort(anfrage).await
}

async fn mit_schluessel(
    absender: &Absender,
    basis: &str,
    token: &str,
    channel: &str,
    text: &str,
    schluessel: &str,
) -> ApiAntwort {
    let koerper = json!({ "text": text }).to_string().into_bytes();
    roh_mit_schluessel(
        absender,
        basis,
        Some(token),
        channel,
        "application/json",
        koerper,
        schluessel,
    )
    .await
}

async fn verlassen(absender: &Absender, basis: &str, token: &str) -> u16 {
    api_antwort(
        absender
            .anfrage(reqwest::Method::POST, format!("{basis}api/verlassen"))
            .header("X-LocalchatA-Sitzung", token),
    )
    .await
    .status
}

fn saetze_mit(inhalt: &[u8], text: &str) -> usize {
    String::from_utf8_lossy(inhalt)
        .lines()
        .filter(|zeile| *zeile == format!("  {text}"))
        .count()
}

async fn verdraengt(basis: &str) -> serde_json::Value {
    gemeinsam::gesundheit(basis).await.1["schluessel_verdraengt"].clone()
}

/// Angriffsmatrix Zeilen 23 und 24 (E2 Geltung je Kennung): der Nachfolger gleichen Namens und gleicher Herkunft
/// erbt `k1` — gleicher Text 200 mit der alten `zeit`, anderer Text oder Channel 422 `schluessel_anders`; eine andere
/// Herkunft hat ihre eigene Geltung (201), ihr Beitritt als `bot` ist 409, solange der Name belegt ist; Zähler 0.
#[tokio::test]
async fn der_nachfolger_gleicher_kennung_erbt_die_schluessel_eine_andere_herkunft_nicht() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[("LOCALCHATA_PROTOKOLL_STUFE", "debug")]);
    let basis = &instanz.adresse;
    let hier = Absender::von("127.0.0.1");
    let dort = Absender::von("127.0.0.2");
    let vorgaenger = beitreten(&hier, basis, "bot", Some("agent")).await;
    let erste = mit_schluessel(&hier, basis, &vorgaenger, "allgemein", "Bericht X", "k1").await;
    let weg = verlassen(&hier, basis, &vorgaenger).await;
    let nachfolger = beitreten(&hier, basis, "bot", Some("agent")).await;

    let gleich = mit_schluessel(&hier, basis, &nachfolger, "allgemein", "Bericht X", "k1").await;
    let anderer_text =
        mit_schluessel(&hier, basis, &nachfolger, "allgemein", "Bericht Y", "k1").await;
    let anderer_channel =
        mit_schluessel(&hier, basis, &nachfolger, "technik", "Bericht X", "k1").await;
    let bot2 = beitreten(&dort, basis, "bot2", Some("agent")).await;
    let fremd = mit_schluessel(&dort, basis, &bot2, "allgemein", "Bericht X", "k1").await;
    let belegt = dort.beitreten(basis, json!({ "name": "bot" }), None).await;

    assert_eq!(erste.status, 201, "{}", erste.json);
    assert_eq!(weg, 204);
    assert_eq!(gleich.status, 200, "{}", gleich.json);
    assert_eq!(
        gleich.json["nachricht"]["zeit"],
        erste.json["nachricht"]["zeit"]
    );
    for antwort in [&anderer_text, &anderer_channel] {
        assert_eq!(antwort.status, 422, "{}", antwort.json);
        assert_eq!(antwort.json["regel"], "schluessel");
    }
    assert_eq!(fremd.status, 201, "{}", fremd.json);
    assert_eq!(belegt.status, 409, "{}", belegt.json);
    assert_eq!(
        saetze_mit(&tagesinhalt(wurzel.path(), "allgemein"), "Bericht X"),
        2
    );
    let anders = gemeinsam::protokoll::alle_zeilen(wurzel.path())
        .into_iter()
        .filter(|zeile| zeile.contains("grund=schluessel_anders"))
        .count();
    assert_eq!(anders, 2);
    assert_eq!(verdraengt(basis).await, 0);
}

/// Angriffsmatrix Zeile 25: `Idempotency-Key` ohne Token, mit erfundenem, mit abgelaufenem (verlassenem) und mit dem
/// Token einer anderen Herkunft → 401; der Speicher bleibt unberührt — derselbe Schlüssel ist danach neu (201).
#[tokio::test]
async fn ohne_gueltige_sitzung_401_und_nichts_gemerkt() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[]);
    let basis = &instanz.adresse;
    let hier = Absender::von("127.0.0.1");
    let dort = Absender::von("127.0.0.2");
    let alt = beitreten(&hier, basis, "alt", None).await;
    assert_eq!(verlassen(&hier, basis, &alt).await, 204);
    let gueltig = beitreten(&hier, basis, "gueltig", None).await;
    let koerper = || json!({ "text": "Versuch" }).to_string().into_bytes();
    let typ = "application/json";

    let ohne = roh_mit_schluessel(&hier, basis, None, "allgemein", typ, koerper(), "z1").await;
    let erfunden = roh_mit_schluessel(
        &hier,
        basis,
        Some(&"0".repeat(64)),
        "allgemein",
        typ,
        koerper(),
        "z1",
    )
    .await;
    let abgelaufen =
        roh_mit_schluessel(&hier, basis, Some(&alt), "allgemein", typ, koerper(), "z1").await;
    let fremde_herkunft = roh_mit_schluessel(
        &dort,
        basis,
        Some(&gueltig),
        "allgemein",
        typ,
        koerper(),
        "z1",
    )
    .await;
    let danach = mit_schluessel(&hier, basis, &gueltig, "allgemein", "Versuch", "z1").await;

    for antwort in [&ohne, &erfunden, &abgelaufen, &fremde_herkunft] {
        assert_eq!(antwort.status, 401, "{}", antwort.json);
    }
    assert_eq!(danach.status, 201, "{}", danach.json);
    assert_eq!(verdraengt(basis).await, 0);
}

/// Angriffsmatrix Zeile 26: Sendungen, die mit 413, 415, 400, 404, 422 `text`, `feld`, `zeichen` oder 429
/// abgewiesen werden, merken nichts — nach Behebung ergibt die Wiederholung mit demselben Schlüssel genau eine 201;
/// keine Verdrängung. (500 bei Plattenfehler ist ohne Eingriff nicht herstellbar: `merken` erst nach dem Anhängen.)
#[tokio::test]
async fn abgewiesene_sendungen_merken_keinen_schluessel() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[]);
    let basis = &instanz.adresse;
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, basis, "beta", None).await;
    let json_typ = "application/json";
    let gross = format!(r#"{{"text":"{}"}}"#, "a".repeat(140_000)).into_bytes();
    let faelle: Vec<(&str, u16, &str, &str, Vec<u8>)> = vec![
        ("r413", 413, "allgemein", json_typ, gross),
        (
            "r415",
            415,
            "allgemein",
            "text/plain",
            br#"{"text":"t"}"#.to_vec(),
        ),
        ("r400", 400, "allgemein", json_typ, b"{".to_vec()),
        (
            "r404",
            404,
            "gibtsnicht",
            json_typ,
            br#"{"text":"t"}"#.to_vec(),
        ),
        (
            "r422text",
            422,
            "allgemein",
            json_typ,
            br#"{"text":5}"#.to_vec(),
        ),
        (
            "r422feld",
            422,
            "allgemein",
            json_typ,
            br#"{"text":"t","x":1}"#.to_vec(),
        ),
        (
            "r422zeichen",
            422,
            "allgemein",
            json_typ,
            br#"{"text":"a\u001b"}"#.to_vec(),
        ),
    ];

    let mut abgewiesen = Vec::new();
    for (schluessel, erwartet, channel, typ, koerper) in faelle {
        let antwort = roh_mit_schluessel(
            &beta,
            basis,
            Some(&token),
            channel,
            typ,
            koerper,
            schluessel,
        )
        .await;
        abgewiesen.push((schluessel, erwartet, antwort.status));
    }
    let mut behoben = Vec::new();
    for (schluessel, _, _) in &abgewiesen {
        let text = format!("behoben {schluessel}");
        behoben.push(
            mit_schluessel(&beta, basis, &token, "allgemein", &text, schluessel)
                .await
                .status,
        );
    }

    for (schluessel, erwartet, status) in &abgewiesen {
        assert_eq!(status, erwartet, "{schluessel}");
    }
    assert_eq!(behoben, [201; 7]);
    assert_eq!(verdraengt(basis).await, 0);
}

/// Angriffsmatrix Zeile 26, 429: ist das Schreibkonto der Kennung leer, merkt die abgewiesene Sendung nichts; nach
/// dem Nachfüllen ergibt derselbe Schlüssel genau eine 201.
#[tokio::test]
async fn eine_gebremste_sendung_merkt_keinen_schluessel() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[]);
    let basis = &instanz.adresse;
    let eilig = Absender::von("127.0.0.1");
    let token = beitreten(&eilig, basis, "eilig", None).await;
    let mut fuellung = Vec::new();
    for nummer in 0..40 {
        let schluessel = format!("f{nummer}");
        let antwort =
            mit_schluessel(&eilig, basis, &token, "allgemein", &schluessel, &schluessel).await;
        fuellung.push(antwort.status);
        if antwort.status == 429 {
            break;
        }
    }
    assert_eq!(fuellung.last(), Some(&429), "{fuellung:?}");
    let gebremst = Some(mit_schluessel(&eilig, basis, &token, "allgemein", "Brems", "r429").await)
        .filter(|antwort| antwort.status == 429);
    let gebremst = gebremst.expect("eine 429");
    let warten = gebremst.json["wieder_in_ms"]
        .as_u64()
        .expect("wieder_in_ms");
    tokio::time::sleep(Duration::from_millis(warten + 200)).await;

    let wieder = mit_schluessel(&eilig, basis, &token, "allgemein", "Brems", "r429").await;

    assert_eq!(wieder.status, 201, "{}", wieder.json);
    assert_eq!(
        saetze_mit(&tagesinhalt(wurzel.path(), "allgemein"), "Brems"),
        1
    );
    assert_eq!(verdraengt(basis).await, 0);
}

/// Angriffsmatrix Zeile 26, 507: an der Quote abgewiesen merkt nichts — nach dem Verkürzen der Frist ergibt derselbe
/// Schlüssel genau eine 201.
#[tokio::test]
async fn eine_sendung_an_der_quote_merkt_keinen_schluessel() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let jetzt = localchata::uhr::Zeit::parse_from_rfc3339(gemeinsam::senden::JETZT).expect("jetzt");
    let berlin = |zeitpunkt: chrono::DateTime<chrono::Utc>| {
        zeitpunkt.with_timezone(&chrono::FixedOffset::east_opt(3600).expect("Versatz"))
    };
    localchata::testdaten::quote_ordner(wurzel.path(), "allgemein", 1_048_576 - 300, jetzt, berlin)
        .expect("Quote-Ordner");
    let instanz = gemeinsam::starten_mit(
        wurzel.path(),
        &[
            "--port",
            "0",
            "--host",
            "127.0.0.1",
            "--kein-browser",
            "--plattenquote",
            "1MiB",
        ],
        &[
            ("LOCALCHATA_JETZT", gemeinsam::senden::JETZT),
            ("TZ", "Europe/Berlin"),
        ],
    );
    let absender = Absender::von("127.0.0.1");
    let basis = &instanz.adresse;
    let token = beitreten(&absender, basis, "beta", None).await;
    let mut status = Vec::new();
    for nummer in 0..4 {
        let schluessel = format!("vorher{nummer}");
        status.push(
            mit_schluessel(
                &absender,
                basis,
                &token,
                "allgemein",
                "Quote-Probe 17 Z.",
                &schluessel,
            )
            .await
            .status,
        );
        if status.last() == Some(&507) {
            break;
        }
    }
    status.push(
        mit_schluessel(
            &absender,
            basis,
            &token,
            "allgemein",
            "Quote-Probe 17 Z.",
            "q1",
        )
        .await
        .status,
    );
    assert_eq!(status.last(), Some(&507), "{status:?}");
    let bestaetigt = gemeinsam::frist::frist_setzen(
        &absender,
        &instanz,
        &token,
        "allgemein",
        json!({ "tage": 1, "bestaetigt": true }),
    )
    .await;
    assert_eq!(bestaetigt.status, 200, "{}", bestaetigt.json);

    let wieder = mit_schluessel(
        &absender,
        basis,
        &token,
        "allgemein",
        "Quote-Probe 17 Z.",
        "q1",
    )
    .await;

    assert_eq!(wieder.status, 201, "{}", wieder.json);
    assert_eq!(verdraengt(basis).await, 0);
}

/// Angriffsmatrix Zeile 17 (Stufe Debug, Testknopf `=5`): Schlüssel `GEHEIM…`, Text `GEHEIMTEXT` — 201, 200, 422
/// Format (Leerzeichen, 65 Zeichen, Umlaut, leer, doppelt), 422 anders und eine Verdrängung mit Episode bis zum
/// Ende → kein `GEHEIM` in `logs/`, auch nicht in Episode-Beginn und -Ende.
#[tokio::test]
async fn weder_schluessel_noch_text_stehen_im_protokoll() {
    let wurzel = ordner();
    let mut instanz = starten(
        wurzel.path(),
        &[
            ("LOCALCHATA_PROTOKOLL_STUFE", "debug"),
            ("LOCALCHATA_TESTGRENZE_SCHLUESSEL", "5"),
        ],
    );
    let basis = instanz.adresse.clone();
    let basis = &basis;
    let geheim = Absender::von("127.0.0.1");
    let token = beitreten(&geheim, basis, "geheim", None).await;
    let lang = format!("GEHEIM{}", "x".repeat(59));
    let mut status = vec![
        mit_schluessel(&geheim, basis, &token, "allgemein", "GEHEIMTEXT", "GEHEIM1")
            .await
            .status,
        mit_schluessel(&geheim, basis, &token, "allgemein", "GEHEIMTEXT", "GEHEIM1")
            .await
            .status,
        mit_schluessel(
            &geheim,
            basis,
            &token,
            "allgemein",
            "GEHEIMTEXT2",
            "GEHEIM1",
        )
        .await
        .status,
    ];
    for kopf in [
        b"GEHEIM a".to_vec(),
        lang.into_bytes(),
        "GEHEIMä".as_bytes().to_vec(),
        Vec::new(),
    ] {
        let antwort = api_antwort(
            geheim
                .anfrage(
                    reqwest::Method::POST,
                    format!("{basis}api/channels/allgemein/nachrichten"),
                )
                .header("X-LocalchatA-Sitzung", &token)
                .header(
                    "Idempotency-Key",
                    reqwest::header::HeaderValue::from_bytes(&kopf).expect("Kopf"),
                )
                .json(&json!({ "text": "GEHEIMTEXT" })),
        )
        .await;
        status.push(antwort.status);
    }
    let doppelt = gemeinsam::roh_anfragen(
        std::net::SocketAddr::from(([127, 0, 0, 1], instanz.port())),
        &format!(
            "POST /api/channels/allgemein/nachrichten HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nX-LocalchatA-Sitzung: {token}\r\nContent-Type: application/json\r\nIdempotency-Key: GEHEIMa\r\nIdempotency-Key: GEHEIMb",
            instanz.port()
        ),
        r#"{"text":"GEHEIMTEXT"}"#,
    );
    status.push(doppelt.status);
    let andere = Absender::von("127.0.0.2");
    for nummer in 0..5 {
        let wer = beitreten(&andere, basis, &format!("v{nummer}"), None).await;
        let text = format!("GEHEIMTEXT{nummer}");
        status.push(
            mit_schluessel(
                &andere,
                basis,
                &wer,
                "allgemein",
                &text,
                &format!("GEHEIMv{nummer}"),
            )
            .await
            .status,
        );
    }
    assert_eq!(verdraengt(basis).await, 1);
    assert_eq!(instanz.signal_und_warten("TERM").code(), Some(0));

    assert_eq!(
        status,
        [
            201, 200, 422, 422, 422, 422, 422, 422, 201, 201, 201, 201, 201
        ]
    );
    let zeilen = gemeinsam::protokoll::alle_zeilen(wurzel.path());
    let episode: Vec<&String> = zeilen
        .iter()
        .filter(|zeile| zeile.contains("Wiederholschlüssel vor Ablauf verdrängt"))
        .collect();
    assert!(
        episode
            .iter()
            .any(|zeile| zeile.contains("Episode beginnt")),
        "{episode:?}"
    );
    assert!(
        episode.iter().any(|zeile| zeile.contains("Episode endet")),
        "{episode:?}"
    );
    let treffer: Vec<&String> = zeilen
        .iter()
        .filter(|zeile| zeile.contains("GEHEIM"))
        .collect();
    assert_eq!(treffer, Vec::<&String>::new());
}
