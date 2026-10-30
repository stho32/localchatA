//! R00039 F0133 — Änderungen live im Strom (ADR-0048 Regel 11): das echte Programm, `alpha` liest den Strom, `beta`
//! bearbeitet seine Nachricht `t1`.

mod gemeinsam;

use gemeinsam::senden::{beitreten, ordner, senden};
use gemeinsam::strom::{Empfangen, Strom};
use gemeinsam::{Absender, ApiAntwort, Instanz, api_antwort};
use serde_json::{Value, json};
use std::path::Path;
use std::time::Duration;
use tokio::time::Instant;

const JETZT: &str = "2026-10-28T12:00:00.000+01:00";
/// Zeitkriterium (`AI-Team/planer.md`): Ereignis weniger als 1 s nach der 200.
const GRENZE: Duration = Duration::from_secs(1);
/// Abwesenheit über die doppelte Grenze beobachtet.
const STILLE: Duration = Duration::from_secs(2);

fn starten(ordner: &Path) -> Instanz {
    gemeinsam::starten_mit(
        ordner,
        &[
            "--port",
            "0",
            "--host",
            "127.0.0.1",
            "--kein-browser",
            "--plattenquote",
            "10MiB",
        ],
        &[("TZ", "Europe/Berlin"), ("LOCALCHATA_JETZT", JETZT)],
    )
}

fn ist_aenderung(e: &Empfangen) -> bool {
    e.data["art"] == "aenderung"
}

async fn bearbeiten(
    absender: &Absender,
    basis: &str,
    token: &str,
    channel: &str,
    zeit: &str,
    text: &str,
    schluessel: Option<&str>,
) -> ApiAntwort {
    let mut anfrage = absender
        .anfrage(
            reqwest::Method::POST,
            format!("{basis}api/channels/{channel}/nachrichten/bearbeiten"),
        )
        .header("X-LocalchatA-Sitzung", token)
        .json(&json!({ "zeit": zeit, "text": text }));
    if let Some(schluessel) = schluessel {
        anfrage = anfrage.header("Idempotency-Key", schluessel);
    }
    api_antwort(anfrage).await
}

/// `alpha` liest den Strom, `beta` hat `Hallo` gesendet: Instanz, Strom, Token von `beta`, `t1`.
async fn gegeben(ordner: &Path) -> (Instanz, Strom, Absender, String, String) {
    let instanz = starten(ordner);
    let absender = Absender::von("127.0.0.1");
    let alpha = beitreten(&absender, &instanz.adresse, "alpha", None).await;
    let beta = beitreten(&absender, &instanz.adresse, "beta", Some("agent")).await;
    let strom = Strom::oeffnen(&absender, &instanz.adresse, &alpha).await;
    let gesendet = senden(&absender, &instanz.adresse, &beta, "allgemein", "Hallo").await;
    assert_eq!(gesendet.status, 201, "{}", gesendet.json);
    let t1 = gesendet.json["nachricht"]["zeit"]
        .as_str()
        .expect("zeit")
        .to_string();
    (instanz, strom, absender, beta, t1)
}

/// R00039 F0133 „Ereignis“: genau ein `aenderung` mit Channel und der Nachricht wie in der 200, weniger als 1 s nach
/// der 200.
#[tokio::test]
async fn die_aenderung_kommt_live_an() {
    let wurzel = ordner();
    let (instanz, mut strom, absender, beta, t1) = gegeben(wurzel.path()).await;

    let antwort = bearbeiten(
        &absender,
        &instanz.adresse,
        &beta,
        "allgemein",
        &t1,
        "Hallo zusammen",
        None,
    )
    .await;
    let erhalten = Instant::now();
    let ereignis = strom.bis(Duration::from_secs(10), ist_aenderung).await;
    strom.mitlesen(STILLE).await;

    assert_eq!(antwort.status, 200, "{}", antwort.json);
    let ereignis = ereignis.expect("aenderung");
    assert_eq!(
        ereignis.data,
        json!({"art": "aenderung", "channel": "allgemein", "nachricht": {
            "zeit": t1, "von": "beta@127.0.0.1", "art": "agent", "text": "Hallo zusammen",
            "zustand": "bearbeitet", "geaendert": antwort.json["nachricht"]["geaendert"], "anhaenge": [], "antwort_auf": null}})
    );
    let dauer = ereignis.ankunft.saturating_duration_since(erhalten);
    println!("Ereignis nach der 200: {dauer:?} (Grenze {GRENZE:?})");
    assert!(dauer < GRENZE, "{dauer:?}");
    assert_eq!(
        strom.ereignisse.iter().filter(|e| ist_aenderung(e)).count(),
        1
    );
}

/// R00039 F0133 „Kein Ereignis ohne Änderung“: Wiederholung mit gleichem Schlüssel, unveränderter Text, 403, 404,
/// 422 und 507 → binnen 2 s nach der letzten kein weiteres `aenderung`.
#[tokio::test]
async fn ohne_aenderung_kein_ereignis() {
    let wurzel = ordner();
    let jetzt = localchata::uhr::Zeit::parse_from_rfc3339(JETZT).expect("jetzt");
    let berlin = |zeitpunkt: chrono::DateTime<chrono::Utc>| {
        zeitpunkt.with_timezone(&chrono::FixedOffset::east_opt(3600).expect("Versatz"))
    };
    localchata::testdaten::quote_ordner(
        wurzel.path(),
        "allgemein",
        10_485_760 - 300,
        jetzt,
        berlin,
    )
    .expect("Quote-Ordner");
    let (instanz, mut strom, absender, beta, t1) = gegeben(wurzel.path()).await;
    let basis = instanz.adresse.as_str();
    let gamma = beitreten(&absender, basis, "gamma", None).await;
    let ohne = (chrono::DateTime::parse_from_rfc3339(&t1).expect("t1")
        + chrono::Duration::milliseconds(1))
    .to_rfc3339_opts(chrono::SecondsFormat::Millis, false);

    let erste = bearbeiten(
        &absender,
        basis,
        &beta,
        "allgemein",
        &t1,
        "Hallo!",
        Some("k1"),
    )
    .await;
    let mut antworten: Vec<(u16, ApiAntwort)> = vec![
        (
            200,
            bearbeiten(
                &absender,
                basis,
                &beta,
                "allgemein",
                &t1,
                "Hallo!",
                Some("k1"),
            )
            .await,
        ),
        (
            200,
            bearbeiten(&absender, basis, &beta, "allgemein", &t1, "Hallo!", None).await,
        ),
        (
            403,
            bearbeiten(&absender, basis, &gamma, "allgemein", &t1, "x", None).await,
        ),
        (
            404,
            bearbeiten(&absender, basis, &beta, "allgemein", &ohne, "x", None).await,
        ),
        (
            404,
            bearbeiten(&absender, basis, &beta, "technik", &t1, "x", None).await,
        ),
        (
            422,
            bearbeiten(&absender, basis, &beta, "allgemein", &t1, " ", None).await,
        ),
    ];
    antworten.push((
        507,
        bearbeiten(
            &absender,
            basis,
            &beta,
            "allgemein",
            &t1,
            &"q".repeat(400),
            None,
        )
        .await,
    ));
    strom.mitlesen(STILLE).await;

    assert_eq!(erste.status, 200, "{}", erste.json);
    for (status, antwort) in &antworten {
        assert_eq!(antwort.status, *status, "{}", antwort.json);
    }
    let aenderungen: Vec<&Value> = strom
        .ereignisse
        .iter()
        .filter(|e| ist_aenderung(e))
        .map(|e| &e.data)
        .collect();
    assert_eq!(aenderungen.len(), 1, "{aenderungen:#?}");
    assert_eq!(aenderungen[0]["nachricht"], erste.json["nachricht"]);
}

/// R00039 F0133 „Neuere Änderung trägt den größeren Zeitstempel“: `Eins`, dann `Zwei` → zwei Ereignisse in dieser
/// Folge, `geaendert` steigt, das zweite trägt `Zwei`.
#[tokio::test]
async fn zwei_aenderungen_kommen_in_ihrer_folge() {
    let wurzel = ordner();
    let (instanz, mut strom, absender, beta, t1) = gegeben(wurzel.path()).await;
    let basis = instanz.adresse.as_str();

    for text in ["Eins", "Zwei"] {
        let antwort = bearbeiten(&absender, basis, &beta, "allgemein", &t1, text, None).await;
        assert_eq!(antwort.status, 200, "{}", antwort.json);
    }
    let zweite = strom
        .bis(Duration::from_secs(10), |e| {
            ist_aenderung(e) && e.data["nachricht"]["text"] == "Zwei"
        })
        .await;

    assert!(zweite.is_some(), "{:#?}", strom.ereignisse);
    let aenderungen: Vec<&Value> = strom
        .ereignisse
        .iter()
        .filter(|e| ist_aenderung(e))
        .map(|e| &e.data["nachricht"])
        .collect();
    assert_eq!(aenderungen.len(), 2, "{aenderungen:#?}");
    assert_eq!(aenderungen[0]["text"], "Eins");
    assert_eq!(aenderungen[1]["text"], "Zwei");
    let zeitpunkt = |wert: &Value| {
        chrono::DateTime::parse_from_rfc3339(wert["geaendert"].as_str().expect("geaendert"))
            .expect("Zeit")
    };
    assert!(zeitpunkt(aenderungen[1]) > zeitpunkt(aenderungen[0]));
}
