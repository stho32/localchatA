//! F0169 Warten bleibt begrenzt — höchstens 4 gleichzeitige Aufrufe je Kennung über das echte Programm
//! (R00049 US-29, US-30; ADR-0054 Regel 8 und 10, W6c, B0751). Alle Teilnehmer treten von `127.0.0.1` bei.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::protokoll::{alle_zeilen, mit};
use gemeinsam::senden::{beitreten, ordner, starten};
use gemeinsam::{Absender, ApiAntwort, api_antwort, gesundheit};
use std::time::{Duration, Instant};
use tokio::task::JoinHandle;

/// Öffnet `GET api/warten?bis=<bis>` mit dem Token; der Aufruf läuft im Hintergrund.
fn aufruf_oeffnen(adresse: &str, token: &str, bis: u32) -> JoinHandle<ApiAntwort> {
    let adresse = adresse.to_string();
    let token = token.to_string();
    tokio::spawn(async move {
        api_antwort(
            Absender::von("127.0.0.1")
                .anfrage(
                    reqwest::Method::GET,
                    format!("{adresse}api/warten?bis={bis}"),
                )
                .header("X-LocalchatA-Sitzung", token),
        )
        .await
    })
}

/// Wartet höchstens 5 s, bis `wartend` im Health-Check `soll` zeigt; liefert den letzten Wert.
async fn wartend_abwarten(adresse: &str, soll: u64) -> u64 {
    let ende = Instant::now() + Duration::from_secs(5);
    loop {
        let wert = gesundheit(adresse).await.1["wartend"]
            .as_u64()
            .expect("wartend");
        if wert == soll || Instant::now() > ende {
            return wert;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

/// US-29 (W6c): vier Aufrufe von `claude` laufen, der fünfte ist 429 `{fehler, grenze: "warten", hoechstens: 4}` mit
/// `Retry-After: 5`; `mia` (andere Kennung, dieselbe Herkunft) läuft zugleich. `wartend` zählt die fünf laufenden.
#[tokio::test]
async fn der_fuenfte_aufruf_einer_kennung_ist_429_eine_andere_kennung_laeuft() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[]);
    let absender = Absender::von("127.0.0.1");
    let claude = beitreten(&absender, &instanz.adresse, "claude", None).await;
    let mia = beitreten(&absender, &instanz.adresse, "mia", None).await;
    let vier: Vec<_> = (0..4)
        .map(|_| aufruf_oeffnen(&instanz.adresse, &claude, 60))
        .collect();
    assert_eq!(wartend_abwarten(&instanz.adresse, 4).await, 4);

    let fuenfter = aufruf_oeffnen(&instanz.adresse, &claude, 5)
        .await
        .expect("fünfter");
    let andere = aufruf_oeffnen(&instanz.adresse, &mia, 60);
    assert_eq!(wartend_abwarten(&instanz.adresse, 5).await, 5);

    assert_eq!(fuenfter.status, 429, "{}", fuenter_text(&fuenfter));
    assert_eq!(fuenfter.json["grenze"], "warten");
    assert_eq!(fuenfter.json["hoechstens"], 4);
    assert!(fuenfter.json["fehler"].is_string(), "{}", fuenfter.json);
    assert_eq!(
        fuenfter
            .kopf
            .get("retry-after")
            .and_then(|wert| wert.to_str().ok()),
        Some("5")
    );
    assert!(
        vier.iter().all(|aufruf| !aufruf.is_finished()),
        "vier laufen"
    );
    assert!(!andere.is_finished(), "mia läuft");
    for aufruf in vier {
        aufruf.abort();
    }
    andere.abort();
}

fn fuenter_text(antwort: &ApiAntwort) -> String {
    antwort.json.to_string()
}

/// US-30: bricht `claude` einen der 4 ab, nimmt ein neuer Aufruf den Platz; nach vier abgebrochenen Aufrufen
/// entsteht für einen fünften kein 429, und `wartend` ist 1.
#[tokio::test]
async fn abgebrochene_aufrufe_geben_ihren_platz_frei() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[]);
    let absender = Absender::von("127.0.0.1");
    let claude = beitreten(&absender, &instanz.adresse, "claude", None).await;
    let vier: Vec<_> = (0..4)
        .map(|_| aufruf_oeffnen(&instanz.adresse, &claude, 60))
        .collect();
    assert_eq!(wartend_abwarten(&instanz.adresse, 4).await, 4);

    vier[0].abort();
    assert_eq!(wartend_abwarten(&instanz.adresse, 3).await, 3);
    let ersatz = aufruf_oeffnen(&instanz.adresse, &claude, 60);
    assert_eq!(wartend_abwarten(&instanz.adresse, 4).await, 4);
    for aufruf in &vier[1..] {
        aufruf.abort();
    }
    ersatz.abort();
    assert_eq!(wartend_abwarten(&instanz.adresse, 0).await, 0);
    let neu = aufruf_oeffnen(&instanz.adresse, &claude, 60);
    let weiterer = aufruf_oeffnen(&instanz.adresse, &claude, 60);
    let dritter = aufruf_oeffnen(&instanz.adresse, &claude, 60);
    let vierter = aufruf_oeffnen(&instanz.adresse, &claude, 60);
    assert_eq!(wartend_abwarten(&instanz.adresse, 4).await, 4);
    let fuenfter = aufruf_oeffnen(&instanz.adresse, &claude, 1)
        .await
        .expect("fünfter");

    assert_eq!(fuenfter.status, 429, "voll: {}", fuenfter.json);
    for aufruf in [neu, weiterer, dritter, vierter] {
        aufruf.abort();
    }
    assert_eq!(wartend_abwarten(&instanz.adresse, 0).await, 0);
    let wieder = aufruf_oeffnen(&instanz.adresse, &claude, 0)
        .await
        .expect("wieder");
    assert_eq!(
        wieder.status, 200,
        "nach den Abbrüchen frei: {}",
        wieder.json
    );
}

/// US-29: mehrere 429 in Folge sind genau eine Grenz-Episode im Betriebsprotokoll (Bereich `warten`, Quelle die
/// Kennung, `hoechstens=4`), nach der Ruhe genau eine Ende-Zeile mit der Zahl; keine Zeile je Aufruf auf
/// INFORMATION und kein Nachrichtentext im Protokoll.
#[tokio::test]
async fn das_429_ist_genau_eine_grenz_episode_keine_zeile_je_aufruf() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[("LOCALCHATA_EPISODE_RUHE_MS", "800")]);
    let absender = Absender::von("127.0.0.1");
    let claude = beitreten(&absender, &instanz.adresse, "claude", None).await;
    let vier: Vec<_> = (0..4)
        .map(|_| aufruf_oeffnen(&instanz.adresse, &claude, 60))
        .collect();
    assert_eq!(wartend_abwarten(&instanz.adresse, 4).await, 4);

    for _ in 0..3 {
        let antwort = aufruf_oeffnen(&instanz.adresse, &claude, 1)
            .await
            .expect("über der Grenze");
        assert_eq!(antwort.status, 429, "{}", antwort.json);
    }
    tokio::time::sleep(Duration::from_millis(2500)).await;
    let zeilen = alle_zeilen(wurzel.path());

    let warnungen = mit(&zeilen, "WARNING", "warten");
    assert_eq!(warnungen.len(), 2, "{warnungen:#?}");
    assert!(
        warnungen[0].contains("Episode beginnt")
            && warnungen[0].contains("kennung=claude@127.0.0.1")
            && warnungen[0].contains("hoechstens=4"),
        "{warnungen:#?}"
    );
    assert!(warnungen[1].contains("Episode endet: 3"), "{warnungen:#?}");
    assert!(
        mit(&zeilen, "INFORMATION", "warten").is_empty(),
        "keine Zeile je Aufruf auf INFORMATION: {zeilen:#?}"
    );
    for aufruf in vier {
        aufruf.abort();
    }
}
