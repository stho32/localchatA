//! F0169 Warten endet mit Grund — Ende der Instanz und Verlassen über das echte Programm (R00049 US-27, US-28;
//! ADR-0054 W6b, B0750). Alle Teilnehmer treten von `127.0.0.1` bei.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::protokoll::alle_zeilen;
use gemeinsam::senden::{beitreten, ordner, starten};
use gemeinsam::{Absender, ApiAntwort, api_antwort};
use serde_json::json;
use std::time::{Duration, Instant};
use tokio::task::JoinHandle;

/// Öffnet `GET api/warten?bis=60` mit dem Token; liefert die Antwort und den Augenblick ihrer Ankunft.
fn aufruf_oeffnen(adresse: &str, token: &str) -> JoinHandle<(ApiAntwort, Instant)> {
    let adresse = adresse.to_string();
    let token = token.to_string();
    tokio::spawn(async move {
        let antwort = api_antwort(
            Absender::von("127.0.0.1")
                .anfrage(reqwest::Method::GET, format!("{adresse}api/warten?bis=60"))
                .header("X-LocalchatA-Sitzung", token),
        )
        .await;
        (antwort, Instant::now())
    })
}

/// W6b / QW5: `anzahl` offene Aufrufe von `anzahl` Kennungen, SIGTERM → alle 200 mit leerer Liste und
/// `ende: "programmende"` binnen 1 s nach dem Signal, Exit 0 binnen 1 s, keine Zeile der Stufe EXCEPTION.
async fn programmende_pruefen(anzahl: usize) {
    let wurzel = ordner();
    let mut instanz = starten(wurzel.path(), &[]);
    let absender = Absender::von("127.0.0.1");
    let mut aufrufe = Vec::new();
    for nummer in 0..anzahl {
        let token = beitreten(&absender, &instanz.adresse, &format!("agent{nummer}"), None).await;
        aufrufe.push(aufruf_oeffnen(&instanz.adresse, &token));
    }
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert!(
        aufrufe.iter().all(|aufruf| !aufruf.is_finished()),
        "alle Aufrufe warten"
    );

    let signal = Instant::now();
    let (status, ende) = tokio::task::spawn_blocking(move || {
        let status = instanz.signal_und_warten("TERM");
        (status, Instant::now())
    })
    .await
    .expect("Signal");
    let mut spaetester = signal;
    for aufruf in aufrufe {
        let (antwort, angekommen) = aufruf.await.expect("Aufruf");
        assert_eq!(antwort.status, 200, "{}", antwort.json);
        assert_eq!(antwort.json["nachrichten"], json!([]), "{}", antwort.json);
        assert_eq!(antwort.json["ende"], "programmende", "{}", antwort.json);
        spaetester = spaetester.max(angekommen);
    }

    assert_eq!(status.code(), Some(0), "Exit 0");
    assert!(
        spaetester.saturating_duration_since(signal) < Duration::from_secs(1),
        "alle Antworten binnen 1 s: {:?}",
        spaetester.saturating_duration_since(signal)
    );
    assert!(
        ende.saturating_duration_since(signal) < Duration::from_secs(1),
        "Exit binnen 1 s: {:?}",
        ende.saturating_duration_since(signal)
    );
    let zeilen = alle_zeilen(wurzel.path());
    assert!(
        zeilen.iter().all(|zeile| !zeile.contains(" EXCEPTION ")),
        "keine EXCEPTION: {zeilen:?}"
    );
}

/// US-27 (W6b): drei offene Aufrufe, SIGTERM.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn drei_offene_aufrufe_enden_beim_signal_mit_programmende() {
    programmende_pruefen(3).await;
}

/// US-27 (QW5): 50 offene Aufrufe von 50 Kennungen, SIGTERM.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn fuenfzig_offene_aufrufe_enden_beim_signal_mit_programmende() {
    programmende_pruefen(50).await;
}

/// US-28 (W6b): `POST /api/verlassen` beendet den offenen Aufruf binnen 1 s mit `ende: "verlassen"`; der nächste
/// Aufruf mit dem Token ist 401. Ein zweiter Wartender (`mia`) bleibt unberührt.
#[tokio::test]
async fn verlassen_beendet_den_offenen_aufruf_mit_verlassen() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[]);
    let absender = Absender::von("127.0.0.1");
    let claude = beitreten(&absender, &instanz.adresse, "claude", None).await;
    let mia = beitreten(&absender, &instanz.adresse, "mia", None).await;
    let aufruf = aufruf_oeffnen(&instanz.adresse, &claude);
    let fremder = aufruf_oeffnen(&instanz.adresse, &mia);
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert!(!aufruf.is_finished(), "der Aufruf wartet");

    let verlassen = api_antwort(
        absender
            .anfrage(
                reqwest::Method::POST,
                format!("{}api/verlassen", instanz.adresse),
            )
            .header("X-LocalchatA-Sitzung", &claude),
    )
    .await;
    let nach_dem_verlassen = Instant::now();
    let (antwort, angekommen) = aufruf.await.expect("Aufruf");
    let danach = api_antwort(
        absender
            .anfrage(
                reqwest::Method::GET,
                format!("{}api/warten?bis=0", instanz.adresse),
            )
            .header("X-LocalchatA-Sitzung", &claude),
    )
    .await;

    assert_eq!(verlassen.status, 204);
    assert_eq!(antwort.status, 200, "{}", antwort.json);
    assert_eq!(antwort.json["nachrichten"], json!([]));
    assert_eq!(antwort.json["ende"], "verlassen", "{}", antwort.json);
    assert!(
        angekommen.saturating_duration_since(nach_dem_verlassen) < Duration::from_secs(1),
        "binnen 1 s"
    );
    assert_eq!(danach.status, 401, "{}", danach.json);
    assert!(!fremder.is_finished(), "mia wartet weiter");
    fremder.abort();
}
