//! F0166 Warten auf Neues in Channels — Route, Halter und Antwort über das echte Programm (R00049 US-1 bis US-4,
//! US-11, US-12; ADR-0054 W6a, B0739). Alle Teilnehmer treten von `127.0.0.1` bei.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::senden::{beitreten, ordner, senden, starten};
use gemeinsam::{Absender, ApiAntwort, Instanz, api_antwort, gesundheit};
use serde_json::{Value, json};
use std::time::{Duration, Instant};

/// `GET api/warten?<query>` mit optionalem Token; `seit` wird kodiert angehängt, wenn es da ist.
async fn warten(
    absender: &Absender,
    instanz: &Instanz,
    token: Option<&str>,
    query: &str,
    seit: Option<&str>,
) -> ApiAntwort {
    let mut url = format!("{}api/warten?{query}", instanz.adresse);
    if let Some(seit) = seit {
        url.push_str(&format!("&seit={}", seit.replace('+', "%2B")));
    }
    let mut anfrage = absender.anfrage(reqwest::Method::GET, url);
    if let Some(token) = token {
        anfrage = anfrage.header("X-LocalchatA-Sitzung", token);
    }
    api_antwort(anfrage).await
}

/// Instanz mit `allgemein` und `technik`; `anna` (Mensch) und `claude` (Agent) sind beigetreten.
async fn gegeben() -> (tempfile::TempDir, Instanz, Absender, String, String) {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[]);
    let absender = Absender::von("127.0.0.1");
    let anna = beitreten(&absender, &instanz.adresse, "anna", Some("mensch")).await;
    let claude = beitreten(&absender, &instanz.adresse, "claude", None).await;
    (wurzel, instanz, absender, anna, claude)
}

async fn gesendet(
    absender: &Absender,
    instanz: &Instanz,
    token: &str,
    channel: &str,
    text: &str,
) -> String {
    let antwort = senden(absender, &instanz.adresse, token, channel, text).await;
    assert_eq!(antwort.status, 201, "{}", antwort.json);
    antwort.json["nachricht"]["zeit"]
        .as_str()
        .expect("zeit")
        .to_string()
}

fn zeiten(antwort: &ApiAntwort) -> Vec<&str> {
    antwort.json["nachrichten"]
        .as_array()
        .expect("nachrichten")
        .iter()
        .map(|eintrag| eintrag["nachricht"]["zeit"].as_str().expect("zeit"))
        .collect()
}

/// US-4 (l), (m): ohne Header und mit unbekanntem Token 401.
#[tokio::test]
async fn ohne_sitzung_401() {
    let (_wurzel, instanz, absender, _anna, _claude) = gegeben().await;

    let ohne = warten(&absender, &instanz, None, "bis=0", None).await;
    let unbekannt = warten(
        &absender,
        &instanz,
        Some("00000000000000000000000000000000"),
        "bis=0",
        None,
    )
    .await;

    assert_eq!(ohne.status, 401, "{}", ohne.json);
    assert_eq!(unbekannt.status, 401, "{}", unbekannt.json);
}

/// US-4 (a)–(k) über die Route: je Regel eine Abweisung mit `regel`, der Tippfehler mit seinem Namen, der unbekannte
/// Channel 404 mit `channel`.
#[tokio::test]
async fn falsche_anfragen_werden_abgewiesen() {
    let (_wurzel, instanz, absender, _anna, claude) = gegeben().await;
    let fuenfundsechzig = vec!["channel=a"; 65].join("&");
    let faelle = [
        ("bis=3601", "bis"),
        ("bis=-1", "bis"),
        ("bis=1.5", "bis"),
        ("bis=abc", "bis"),
        ("seit=gestern", "cursor"),
        ("seit=2026-10-30T11:18:00.123+01:00", "cursor"),
        ("erwaehnung=1", "filter"),
        (fuenfundsechzig.as_str(), "filter"),
        (
            "seit=2026-10-30T11:18:00.123%2B01:00&seit=2026-10-30T11:18:00.124%2B01:00",
            "filter",
        ),
        ("erwahnung", "parameter"),
    ];

    for (query, regel) in faelle {
        let antwort = warten(&absender, &instanz, Some(&claude), query, None).await;
        assert_eq!(antwort.status, 422, "{query}: {}", antwort.json);
        assert_eq!(antwort.json["regel"], regel, "{query}: {}", antwort.json);
        assert!(
            antwort.json["fehler"].is_string(),
            "{query}: {}",
            antwort.json
        );
    }
    let tippfehler = warten(&absender, &instanz, Some(&claude), "erwahnung", None).await;
    let plus = warten(
        &absender,
        &instanz,
        Some(&claude),
        "seit=2026-10-30T11:18:00.123+01:00",
        None,
    )
    .await;
    let unbekannt = warten(
        &absender,
        &instanz,
        Some(&claude),
        "channel=gibtesnicht",
        None,
    )
    .await;

    assert_eq!(tippfehler.json["parameter"], "erwahnung");
    assert!(
        plus.json["fehler"]
            .as_str()
            .expect("fehler")
            .contains("%2B"),
        "{}",
        plus.json
    );
    assert_eq!(unbekannt.status, 404, "{}", unbekannt.json);
    assert_eq!(unbekannt.json["channel"], "gibtesnicht");
}

/// US-1: `bis=0` holt über alle Channels in Zeitfolge nach — jeder Eintrag `{channel, direkt, anlass, nachricht}`,
/// `seit` die letzte, `ende: "treffer"`; mit `seit=<t4>` leer, `seit` bleibt, `ende: "frist"`; nicht zwischenspeicherbar.
#[tokio::test]
async fn us1_bis_null_holt_ueber_alle_channels_in_zeitfolge_nach() {
    let (_wurzel, instanz, absender, anna, claude) = gegeben().await;
    let ben = beitreten(&absender, &instanz.adresse, "ben", Some("mensch")).await;
    let t0 = gesendet(&absender, &instanz, &claude, "technik", "t0").await;
    let t1 = gesendet(&absender, &instanz, &anna, "allgemein", "t1").await;
    let t2 = gesendet(&absender, &instanz, &anna, "allgemein", "t2").await;
    let t3 = gesendet(&absender, &instanz, &anna, "technik", "t3").await;
    let t4 = gesendet(&absender, &instanz, &ben, "allgemein", "t4").await;
    let beginn = Instant::now();

    let erste = warten(&absender, &instanz, Some(&claude), "bis=0", Some(&t0)).await;
    let zweite = warten(&absender, &instanz, Some(&claude), "bis=0", Some(&t4)).await;

    assert!(beginn.elapsed() < Duration::from_secs(1), "sofort");
    assert_eq!(erste.status, 200, "{}", erste.json);
    assert_eq!(zeiten(&erste), vec![t1.as_str(), &t2, &t3, &t4]);
    let channels: Vec<&Value> = erste.json["nachrichten"]
        .as_array()
        .expect("nachrichten")
        .iter()
        .map(|eintrag| &eintrag["channel"])
        .collect();
    assert_eq!(
        channels,
        [
            &json!("allgemein"),
            &json!("allgemein"),
            &json!("technik"),
            &json!("allgemein")
        ]
    );
    let erster = &erste.json["nachrichten"][0];
    assert_eq!(erster["direkt"], Value::Null);
    assert_eq!(erster["anlass"], json!([]));
    assert_eq!(erster["nachricht"]["von"], "anna@127.0.0.1");
    assert_eq!(erster["nachricht"]["text"], "t1");
    assert_eq!(erster["nachricht"]["zustand"], Value::Null);
    assert_eq!(
        (&erste.json["seit"], &erste.json["ende"]),
        (&json!(t4), &json!("treffer"))
    );
    assert_eq!(
        erste
            .kopf
            .get("cache-control")
            .and_then(|wert| wert.to_str().ok()),
        Some("no-store")
    );
    assert_eq!(zweite.status, 200, "{}", zweite.json);
    assert_eq!(zweite.json["nachrichten"], json!([]));
    assert_eq!(
        (&zweite.json["seit"], &zweite.json["ende"]),
        (&json!(t4), &json!("frist"))
    );
}

/// US-2: ein offener Aufruf kehrt binnen 1 s nach der 201 mit genau der neuen Nachricht zurück; `seit` ist ihre
/// Zeit (nicht die Marke beim Eingang — Mutante „`seit` = `H`“).
#[tokio::test]
async fn us2_ein_offener_aufruf_kehrt_binnen_einer_sekunde_nach_der_201_zurueck() {
    let (_wurzel, instanz, absender, anna, claude) = gegeben().await;
    let t4 = gesendet(&absender, &instanz, &anna, "allgemein", "t4").await;
    let aufruf = {
        let adresse = instanz.adresse.clone();
        let claude = claude.clone();
        let t4 = t4.clone();
        tokio::spawn(async move {
            let antwort = api_antwort(
                Absender::von("127.0.0.1")
                    .anfrage(
                        reqwest::Method::GET,
                        format!("{adresse}api/warten?bis=30&seit={}", t4.replace('+', "%2B")),
                    )
                    .header("X-LocalchatA-Sitzung", claude),
            )
            .await;
            (antwort, Instant::now())
        })
    };
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert!(!aufruf.is_finished(), "der Aufruf wartet");

    let t5 = gesendet(&absender, &instanz, &anna, "technik", "t5").await;
    let nach_der_201 = Instant::now();
    let (antwort, angekommen) = aufruf.await.expect("Aufruf");

    assert!(
        angekommen.saturating_duration_since(nach_der_201) < Duration::from_secs(1),
        "{:?}",
        angekommen.saturating_duration_since(nach_der_201)
    );
    assert_eq!(zeiten(&antwort), vec![t5.as_str()]);
    assert_eq!(
        (&antwort.json["seit"], &antwort.json["ende"]),
        (&json!(t5), &json!("treffer"))
    );
}

/// Ohne `seit` gilt „ab jetzt“; eine eigene Nachricht während des Wartens weckt nicht, rückt aber `seit` vor
/// (verarbeiteter Stand, nicht die Marke).
#[tokio::test]
async fn ohne_seit_ab_jetzt_und_der_verarbeitete_stand_rueckt_vor() {
    let (_wurzel, instanz, absender, anna, claude) = gegeben().await;
    let vorher = gesendet(&absender, &instanz, &anna, "allgemein", "vorher").await;
    let aufruf = {
        let adresse = instanz.adresse.clone();
        let claude = claude.clone();
        tokio::spawn(async move {
            api_antwort(
                Absender::von("127.0.0.1")
                    .anfrage(reqwest::Method::GET, format!("{adresse}api/warten?bis=2"))
                    .header("X-LocalchatA-Sitzung", claude),
            )
            .await
        })
    };
    tokio::time::sleep(Duration::from_millis(500)).await;
    let eigene = gesendet(&absender, &instanz, &claude, "allgemein", "eigene").await;

    let antwort = aufruf.await.expect("Aufruf");

    assert_eq!(antwort.status, 200, "{}", antwort.json);
    assert_eq!(
        antwort.json["nachrichten"],
        json!([]),
        "weder vorher noch eigene"
    );
    assert_ne!(antwort.json["seit"], json!(vorher));
    assert_eq!(
        (&antwort.json["seit"], &antwort.json["ende"]),
        (&json!(eigene), &json!("frist"))
    );
}

/// US-11: während des Wartens ist `claude` online und `wartend` 1; danach `wartend` 0, nach dem Nachlauf offline.
#[tokio::test]
async fn us11_der_wartende_aufruf_haelt_die_sitzung_online() {
    let (_wurzel, instanz, absender, _anna, claude) = gegeben().await;
    let status = |liste: &ApiAntwort| {
        liste.json["liste"]
            .as_array()
            .expect("liste")
            .iter()
            .find(|zeile| zeile["kennung"] == "claude@127.0.0.1")
            .map(|zeile| zeile["status"].clone())
    };
    let liste = |absender: &Absender, token: String, adresse: String| {
        api_antwort(
            absender
                .anfrage(reqwest::Method::GET, format!("{adresse}api/teilnehmer"))
                .header("X-LocalchatA-Sitzung", token),
        )
    };
    let leser = Absender::von("127.0.0.2");
    let mia = beitreten(&leser, &instanz.adresse, "mia", None).await;
    let vorher = gesundheit(&instanz.adresse).await.1;
    let aufruf = {
        let adresse = instanz.adresse.clone();
        let claude = claude.clone();
        tokio::spawn(async move {
            api_antwort(
                Absender::von("127.0.0.1")
                    .anfrage(reqwest::Method::GET, format!("{adresse}api/warten?bis=3"))
                    .header("X-LocalchatA-Sitzung", claude),
            )
            .await
        })
    };
    tokio::time::sleep(Duration::from_secs(1)).await;

    let waehrend = liste(&leser, mia.clone(), instanz.adresse.clone()).await;
    let gesund_waehrend = gesundheit(&instanz.adresse).await.1;
    let antwort = aufruf.await.expect("Aufruf");
    let gesund_danach = gesundheit(&instanz.adresse).await.1;
    tokio::time::sleep(Duration::from_secs(2)).await;
    let spaeter = liste(&leser, mia, instanz.adresse.clone()).await;

    assert_eq!(antwort.status, 200, "{}", antwort.json);
    assert_eq!(vorher["wartend"], 0);
    assert_eq!(status(&waehrend), Some(json!("online")));
    assert_eq!(gesund_waehrend["wartend"], 1);
    assert_eq!(gesund_danach["wartend"], 0);
    assert_eq!(status(&spaeter), Some(json!("offline")));
    let _ = absender;
}

/// US-12: zwei Aufrufe derselben Kennung bekommen beide dieselbe Nachricht.
#[tokio::test]
async fn us12_zwei_aufrufe_derselben_kennung_bekommen_beide_alles() {
    let (_wurzel, instanz, absender, anna, claude) = gegeben().await;
    let t0 = gesendet(&absender, &instanz, &anna, "allgemein", "t0").await;
    let aufruf = || {
        let adresse = instanz.adresse.clone();
        let claude = claude.clone();
        let t0 = t0.clone();
        tokio::spawn(async move {
            api_antwort(
                Absender::von("127.0.0.1")
                    .anfrage(
                        reqwest::Method::GET,
                        format!("{adresse}api/warten?bis=30&seit={}", t0.replace('+', "%2B")),
                    )
                    .header("X-LocalchatA-Sitzung", claude),
            )
            .await
        })
    };
    let (eins, zwei) = (aufruf(), aufruf());
    tokio::time::sleep(Duration::from_millis(500)).await;

    let t1 = gesendet(&absender, &instanz, &anna, "allgemein", "t1").await;
    let eins = eins.await.expect("eins");
    let zwei = zwei.await.expect("zwei");

    assert_eq!(zeiten(&eins), vec![t1.as_str()]);
    assert_eq!(zeiten(&zwei), vec![t1.as_str()]);
}

/// `warten` bucht nichts am Schreibkonto: nach 40 Aufrufen (mehr als der Vorrat von 30 je Kennung) sendet `claude`
/// weiter mit 201, und `gebremst` bleibt 0.
#[tokio::test]
async fn warten_bucht_nichts_am_schreibkonto() {
    let (_wurzel, instanz, absender, _anna, claude) = gegeben().await;

    for _ in 0..40 {
        let antwort = warten(&absender, &instanz, Some(&claude), "bis=0", None).await;
        assert_eq!(antwort.status, 200, "{}", antwort.json);
    }
    let gesendet = senden(&absender, &instanz.adresse, &claude, "allgemein", "danach").await;

    assert_eq!(gesendet.status, 201, "{}", gesendet.json);
    assert_eq!(gesundheit(&instanz.adresse).await.1["gebremst"], 0);
}
