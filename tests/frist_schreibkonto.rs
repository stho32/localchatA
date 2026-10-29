//! F0072 Frist bucht am Schreibkonto (R00021, US-1, US-4): das echte Programm; jede Friständerung kostet 10
//! Einheiten (1 vorab, 9 nach den Prüfungen von Körper und Wert), eine Kennung hat 30. Jedes Kriterium startet eine
//! eigene Instanz; alle Kennungen kommen von `127.0.0.1`.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::frist::{frist_setzen, tag_anlegen};
use gemeinsam::protokoll::{mit, zeilen};
use gemeinsam::senden::{HEUTE, beitreten, ordner, senden, starten};
use gemeinsam::strom::{Empfangen, Strom};
use gemeinsam::{Absender, ApiAntwort, Instanz};
use serde_json::{Value, json};
use std::path::Path;
use std::time::{Duration, Instant};

const BEGINN_FRIST: &str =
    "WARNING schreibkonto Frist gebremst: zu viele Änderungen — Episode beginnt";
const BEGINN_SENDEN: &str =
    "WARNING schreibkonto Senden gebremst: zu viele Nachrichten — Episode beginnt";

fn retry_after(antwort: &ApiAntwort) -> u64 {
    antwort
        .kopf
        .get("retry-after")
        .and_then(|wert| wert.to_str().ok())
        .and_then(|wert| wert.parse().ok())
        .expect("Retry-After")
}

fn tage(n: u32) -> Value {
    json!({ "tage": n })
}

fn ist_channel_allgemein(e: &Empfangen) -> bool {
    e.data["art"] == "channel" && e.data["channel"]["name"] == "allgemein"
}

async fn gebremst_gesamt(instanz: &Instanz) -> u64 {
    let (status, json) = gemeinsam::gesundheit(&instanz.adresse).await;
    assert_eq!(status, 200);
    json["gebremst"].as_u64().expect("gebremst")
}

/// Setzt die Frist von `allgemein` nacheinander auf die `werte`; liefert die Antworten.
async fn nacheinander(
    absender: &Absender,
    instanz: &Instanz,
    token: &str,
    channel: &str,
    werte: &[Value],
) -> Vec<ApiAntwort> {
    let mut antworten = Vec::new();
    for wert in werte {
        antworten.push(frist_setzen(absender, instanz, token, channel, wert.clone()).await);
    }
    antworten
}

fn status(antworten: &[ApiAntwort]) -> Vec<u16> {
    antworten.iter().map(|antwort| antwort.status).collect()
}

/// Wartet höchstens `hoechstens`, bis `logs/<heute>.log` eine Zeile hat, die `passt`.
async fn warten_auf(ordner: &Path, hoechstens: Duration, passt: impl Fn(&str) -> bool) -> bool {
    let beginn = Instant::now();
    while beginn.elapsed() < hoechstens {
        if zeilen(ordner, HEUTE).iter().any(|zeile| passt(zeile)) {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    false
}

/// R00021 „Kennung im echten Programm“ (ADR-0036 Ergänzung Karte #31 (b), erster Integrationstest) mit
/// „`gebremst` zählt mit“, US-1 und US-4: `beta` setzt 8, 9, 8, 9 → 200, 200, 200, 429 mit `grenze: kennung`,
/// `Retry-After` ≥ 9 und dem Satz mit diesen Sekunden; `einstellungen.toml` trägt 8; `gamma` sieht genau drei
/// Ereignisse `channel`; genau eine Beginn-Zeile in `heute.log` und auf stderr; `gebremst` g → g + 1. Mutante
/// „Frist bucht nicht“ → rot.
#[tokio::test]
async fn us1_die_vierte_aenderung_einer_kennung_wird_gebremst_und_aendert_nichts() {
    let wurzel = ordner();
    let mut instanz = starten(wurzel.path(), &[]);
    let stderr = instanz.stderr_mitschreiben();
    let eins = Absender::von("127.0.0.1");
    let beta = beitreten(&eins, &instanz.adresse, "beta", None).await;
    let gamma = beitreten(&eins, &instanz.adresse, "gamma", None).await;
    let mut strom = Strom::oeffnen(&eins, &instanz.adresse, &gamma).await;
    strom.mitlesen(Duration::from_millis(200)).await;
    let vorher = gebremst_gesamt(&instanz).await;

    let antworten = nacheinander(
        &eins,
        &instanz,
        &beta,
        "allgemein",
        &[tage(8), tage(9), tage(8), tage(9)],
    )
    .await;
    strom.mitlesen(Duration::from_millis(1_000)).await;
    let nachher = gebremst_gesamt(&instanz).await;

    assert_eq!(status(&antworten), [200, 200, 200, 429]);
    let vierte = &antworten[3];
    let sekunden = retry_after(vierte);
    assert!(sekunden >= 9, "Retry-After {sekunden}");
    assert_eq!(vierte.json["grenze"], "kennung");
    assert_eq!(
        vierte.json["fehler"],
        format!("Zu viele Änderungen von beta@127.0.0.1 — wieder möglich in {sekunden} s.")
    );
    assert!(vierte.json["wieder_in_ms"].as_u64().expect("ms") > 8_000);
    let einstellungen =
        std::fs::read_to_string(wurzel.path().join("channels/allgemein/einstellungen.toml"))
            .expect("einstellungen.toml");
    assert!(einstellungen.contains("frist_tage = 8"), "{einstellungen}");
    let gesehen: Vec<_> = strom
        .ereignisse
        .iter()
        .filter(|e| ist_channel_allgemein(e))
        .map(|e| e.data["channel"]["frist_tage"].clone())
        .collect();
    assert_eq!(gesehen, [json!(8), json!(9), json!(8)]);
    assert_eq!(nachher, vorher + 1);
    let beginn: Vec<_> = zeilen(wurzel.path(), HEUTE)
        .into_iter()
        .filter(|zeile| zeile.contains(BEGINN_FRIST))
        .collect();
    assert_eq!(beginn.len(), 1, "{beginn:#?}");
    assert!(
        beginn[0].ends_with(&format!(
            "{BEGINN_FRIST} | kennung=beta@127.0.0.1 grenze=kennung"
        )),
        "{beginn:#?}"
    );
    let auf_stderr = stderr
        .lock()
        .expect("stderr")
        .iter()
        .filter(|zeile| zeile.contains(BEGINN_FRIST))
        .count();
    assert_eq!(auf_stderr, 1);
}

/// R00021 „Kosten je Antwort“ (a): 25 × `{"tage":0}` → 25 × 422 (je 1 Einheit); die folgende Änderung → 429
/// (26 von 30, der Rest 9 fehlt). Mutante „422 kostet 10“ → die 4. Anfrage wäre 429.
#[tokio::test]
async fn kosten_je_antwort_eine_422_kostet_nur_die_vorab_einheit() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[]);
    let eins = Absender::von("127.0.0.1");
    let beta = beitreten(&eins, &instanz.adresse, "beta", None).await;
    let beginn = Instant::now();

    let ungueltig = nacheinander(&eins, &instanz, &beta, "allgemein", &vec![tage(0); 25]).await;
    let dauer = beginn.elapsed();
    let danach = frist_setzen(&eins, &instanz, &beta, "allgemein", tage(8)).await;

    assert!(dauer < Duration::from_secs(4), "Folge dauerte {dauer:?}");
    assert_eq!(status(&ungueltig), vec![422; 25]);
    assert_eq!(danach.status, 429, "{}", danach.json);
}

/// R00021 „Kosten je Antwort“ (b): `delta` setzt die Frist von `gibtsnicht` dreimal → 3 × 404 (je 10); die vierte
/// Anfrage an `allgemein` → 429. Mutante „404 kostet 1“ → die vierte wäre 200.
#[tokio::test]
async fn kosten_je_antwort_eine_404_kostet_die_vollen_zehn() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[]);
    let eins = Absender::von("127.0.0.1");
    let delta = beitreten(&eins, &instanz.adresse, "delta", None).await;

    let unbekannt = nacheinander(&eins, &instanz, &delta, "gibtsnicht", &vec![tage(8); 3]).await;
    let vierte = frist_setzen(&eins, &instanz, &delta, "allgemein", tage(8)).await;

    assert_eq!(status(&unbekannt), [404, 404, 404]);
    assert_eq!(vierte.status, 429, "{}", vierte.json);
}

/// R00021 „Kosten je Antwort“ (c): `epsilon` setzt denselben Wert dreimal (7, 7, 7) → 3 × 200 ohne Ereignis
/// `channel`; die vierte → 429.
#[tokio::test]
async fn kosten_je_antwort_derselbe_wert_kostet_die_vollen_zehn() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[]);
    let eins = Absender::von("127.0.0.1");
    let epsilon = beitreten(&eins, &instanz.adresse, "epsilon", None).await;
    let gamma = beitreten(&eins, &instanz.adresse, "gamma", None).await;
    let mut strom = Strom::oeffnen(&eins, &instanz.adresse, &gamma).await;

    let gleich = nacheinander(&eins, &instanz, &epsilon, "allgemein", &vec![tage(7); 3]).await;
    let vierte = frist_setzen(&eins, &instanz, &epsilon, "allgemein", tage(8)).await;
    strom.mitlesen(Duration::from_millis(500)).await;

    assert_eq!(status(&gleich), [200, 200, 200]);
    assert_eq!(vierte.status, 429, "{}", vierte.json);
    assert!(
        !strom.ereignisse.iter().any(ist_channel_allgemein),
        "{:#?}",
        strom.ereignisse
    );
}

/// R00021 „Verkürzen bleibt ungebremst“, US-2 über die API: `allgemein` hat Tage, die bei 1 entfielen; `beta` setzt
/// 1 → 409; bestätigt → 200; 5 → 200; 6 → 429.
#[tokio::test]
async fn verkuerzen_in_zwei_schritten_bleibt_ungebremst() {
    let wurzel = ordner();
    tag_anlegen(wurzel.path(), "allgemein", "2026-10-26", 2);
    tag_anlegen(wurzel.path(), "allgemein", "2026-10-28", 1);
    let instanz = starten(wurzel.path(), &[]);
    let eins = Absender::von("127.0.0.1");
    let beta = beitreten(&eins, &instanz.adresse, "beta", None).await;

    let antworten = nacheinander(
        &eins,
        &instanz,
        &beta,
        "allgemein",
        &[
            tage(1),
            json!({ "tage": 1, "bestaetigt": true }),
            tage(5),
            tage(6),
        ],
    )
    .await;

    assert_eq!(status(&antworten), [409, 200, 200, 429]);
}

/// R00021 „Episode getrennt vom Senden“, US-4 (`LOCALCHATA_EPISODE_RUHE_MS=2000`): `beta` sendet, bis das Senden
/// gebremst wird (Eimer leer), und setzt gleich danach die Frist → 429 „Zu viele Änderungen …“ mit `Retry-After` 9
/// oder 10. `heute.log` hat für `beta` je genau eine Beginn-Zeile des Sendens und der Frist; nach der Ruhe je eine
/// Ende-Zeile, die der Frist mit `Episode endet: 1 in 0 s`. Mutante „Frist im selben Schlüssel wie das Senden“ →
/// nur eine Beginn-Zeile → rot.
#[tokio::test]
async fn us4_die_episode_der_frist_ist_getrennt_vom_senden() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[("LOCALCHATA_EPISODE_RUHE_MS", "2000")]);
    let eins = Absender::von("127.0.0.1");
    let beta = beitreten(&eins, &instanz.adresse, "beta", None).await;

    let mut senden_gebremst = false;
    for nummer in 0..60 {
        let antwort = senden(
            &eins,
            &instanz.adresse,
            &beta,
            "allgemein",
            &format!("n{nummer}"),
        )
        .await;
        if antwort.status == 429 {
            senden_gebremst = true;
            break;
        }
    }
    let frist = frist_setzen(&eins, &instanz, &beta, "allgemein", tage(8)).await;
    let enden = |zeilen: &[String]| -> Vec<String> {
        mit(zeilen, "WARNING", "schreibkonto")
            .into_iter()
            .filter(|z| z.contains("Episode endet") && z.contains("kennung=beta@127.0.0.1"))
            .collect()
    };
    let beide_zu = warten_auf(wurzel.path(), Duration::from_secs(15), |_| {
        enden(&zeilen(wurzel.path(), HEUTE)).len() >= 2
    })
    .await;
    let alle = zeilen(wurzel.path(), HEUTE);

    assert!(senden_gebremst, "das Senden wurde nie gebremst");
    assert_eq!(frist.status, 429, "{}", frist.json);
    let sekunden = retry_after(&frist);
    assert!((9..=10).contains(&sekunden), "Retry-After {sekunden}");
    assert_eq!(
        frist.json["fehler"],
        format!("Zu viele Änderungen von beta@127.0.0.1 — wieder möglich in {sekunden} s.")
    );
    let zaehle = |teil: &str| alle.iter().filter(|z| z.contains(teil)).count();
    assert_eq!(zaehle(BEGINN_SENDEN), 1, "{alle:#?}");
    assert_eq!(zaehle(BEGINN_FRIST), 1, "{alle:#?}");
    assert!(beide_zu, "keine zwei Ende-Zeilen: {alle:#?}");
    let enden = enden(&alle);
    assert_eq!(enden.len(), 2, "{enden:#?}");
    assert!(
        enden
            .iter()
            .any(|z| z.contains("Frist gebremst: zu viele Änderungen — Episode endet: 1 in 0 s")),
        "{enden:#?}"
    );
    assert!(
        enden
            .iter()
            .any(|z| z.contains("Senden gebremst: zu viele Nachrichten — Episode endet: ")),
        "{enden:#?}"
    );
}
