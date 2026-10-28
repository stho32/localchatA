//! F0046 Wiederholschlüssel an der Grenze (R00012, US-1; ADR-0034 Regeln 3.7, 3.9, 6): das echte Programm,
//! `alpha` liest den Strom, `beta` sendet mit `Idempotency-Key`.

mod gemeinsam;

use gemeinsam::senden::{beitreten, ordner, senden, starten, tagesinhalt};
use gemeinsam::strom::{Empfangen, Strom};
use gemeinsam::{Absender, ApiAntwort, api_antwort};
use serde_json::json;
use std::time::Duration;

/// `POST …/nachrichten` mit `{"text": text}` und dem Kopf `Idempotency-Key: schluessel`.
async fn mit_schluessel(
    absender: &Absender,
    basis: &str,
    token: &str,
    channel: &str,
    text: &str,
    schluessel: &str,
) -> ApiAntwort {
    api_antwort(
        absender
            .anfrage(
                reqwest::Method::POST,
                format!("{basis}api/channels/{channel}/nachrichten"),
            )
            .header("X-LocalchatA-Sitzung", token)
            .header("Idempotency-Key", schluessel)
            .json(&json!({ "text": text })),
    )
    .await
}

fn ist_nachricht(e: &Empfangen) -> bool {
    e.data["art"] == "nachricht"
}

fn saetze_mit(inhalt: &[u8], text: &str) -> usize {
    String::from_utf8_lossy(inhalt)
        .lines()
        .filter(|zeile| *zeile == format!("  {text}"))
        .count()
}

/// R00012 F0046 „Format“: `a`, `A-z_0-9` und 64 × `a` → 201; leer, 65 × `a`, `a b`, `ä`, `a.b` → je 422
/// `regel: "schluessel"`; dazu zwei Köpfe `a` und `b` → 422; nichts geschrieben.
#[tokio::test]
async fn das_format_des_schluessels_wird_geprueft_nichts_geschrieben() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[]);
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, &instanz.adresse, "beta", None).await;
    let basis = &instanz.adresse;
    let lang = "a".repeat(64);
    let zu_lang = "a".repeat(65);

    let mut gueltig = Vec::new();
    for (nummer, schluessel) in ["a", "A-z_0-9", lang.as_str()].iter().enumerate() {
        let text = format!("gut{nummer}");
        gueltig.push(mit_schluessel(&beta, basis, &token, "allgemein", &text, schluessel).await);
    }
    let vorher = tagesinhalt(wurzel.path(), "allgemein");
    let mut ungueltig = Vec::new();
    for schluessel in ["", zu_lang.as_str(), "a b", "ä", "a.b"] {
        let kopf = reqwest::header::HeaderValue::from_bytes(schluessel.as_bytes()).expect("Kopf");
        let antwort = api_antwort(
            beta.anfrage(
                reqwest::Method::POST,
                format!("{basis}api/channels/allgemein/nachrichten"),
            )
            .header("X-LocalchatA-Sitzung", &token)
            .header("Idempotency-Key", kopf)
            .json(&json!({ "text": "schlecht" })),
        )
        .await;
        ungueltig.push((schluessel.to_string(), antwort));
    }
    // Zwei Kopfzeilen roh über TCP — ein HTTP-Client könnte sie zusammenfassen.
    let doppelt = gemeinsam::roh_anfragen(
        std::net::SocketAddr::from(([127, 0, 0, 1], instanz.port())),
        &format!(
            "POST /api/channels/allgemein/nachrichten HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nX-LocalchatA-Sitzung: {token}\r\nContent-Type: application/json\r\nIdempotency-Key: a\r\nIdempotency-Key: b",
            instanz.port()
        ),
        r#"{"text":"schlecht"}"#,
    );
    let doppelt = ApiAntwort {
        status: doppelt.status,
        kopf: reqwest::header::HeaderMap::new(),
        json: serde_json::from_str(&doppelt.inhalt).unwrap_or_default(),
    };
    ungueltig.push(("a, b (zwei Köpfe)".to_string(), doppelt));

    assert_eq!(
        gueltig.iter().map(|a| a.status).collect::<Vec<_>>(),
        [201, 201, 201]
    );
    for (schluessel, antwort) in &ungueltig {
        assert_eq!(antwort.status, 422, "{schluessel:?}: {}", antwort.json);
        assert_eq!(antwort.json["regel"], "schluessel", "{schluessel:?}");
        assert!(antwort.json["fehler"].is_string(), "{}", antwort.json);
    }
    assert_eq!(tagesinhalt(wurzel.path(), "allgemein"), vorher);
    assert_eq!(saetze_mit(&vorher, "schlecht"), 0);
}

/// R00012 F0046 „Wiederholung“ (US-1): 201 mit Zeit t; dieselbe noch einmal → 200 mit derselben Nachricht;
/// `Hallo!` mit `k1` → 422 `schluessel` mit dem Text; das nächste Ereignis nach `Hallo` ist `drei`; `Hallo`
/// genau einmal in der Tagesdatei; `k1` weder in Datei, Antworten noch Ereignissen.
#[tokio::test]
async fn us1_dieselbe_nachricht_mit_demselben_schluessel_ist_200_ohne_satz_und_ohne_ereignis() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[]);
    let alpha = Absender::von("127.0.0.1");
    let beta = Absender::von("127.0.0.1");
    let token_alpha = beitreten(&alpha, &instanz.adresse, "alpha", None).await;
    let token = beitreten(&beta, &instanz.adresse, "beta", None).await;
    let mut strom = Strom::oeffnen(&alpha, &instanz.adresse, &token_alpha).await;
    let basis = &instanz.adresse;

    let erste = mit_schluessel(&beta, basis, &token, "allgemein", "Hallo", "k1").await;
    let zweite = mit_schluessel(&beta, basis, &token, "allgemein", "Hallo", "k1").await;
    let andere = mit_schluessel(&beta, basis, &token, "allgemein", "Hallo!", "k1").await;
    let drei = senden(&beta, basis, &token, "allgemein", "drei").await;
    let hallo = strom
        .bis(Duration::from_secs(10), ist_nachricht)
        .await
        .expect("Ereignis Hallo");
    let danach = strom
        .bis(Duration::from_secs(10), |e| {
            ist_nachricht(e) && e.data["nachricht"]["text"] != "Hallo"
        })
        .await
        .expect("Ereignis danach");
    let inhalt = tagesinhalt(wurzel.path(), "allgemein");

    assert_eq!(erste.status, 201, "{}", erste.json);
    assert_eq!(zweite.status, 200, "{}", zweite.json);
    assert_eq!(zweite.json, erste.json);
    assert_eq!(andere.status, 422, "{}", andere.json);
    assert_eq!(
        andere.json,
        json!({"fehler": "Dieser Schlüssel gehört zu einer anderen Nachricht.", "regel": "schluessel"})
    );
    assert_eq!(drei.status, 201);
    assert_eq!(hallo.data["nachricht"]["text"], "Hallo");
    assert_eq!(danach.data["nachricht"]["text"], "drei");
    let ereignisse_hallo = strom
        .ereignisse
        .iter()
        .filter(|e| ist_nachricht(e) && e.data["nachricht"]["text"] == "Hallo")
        .count();
    assert_eq!(ereignisse_hallo, 1);
    assert_eq!(saetze_mit(&inhalt, "Hallo"), 1);
    assert_eq!(saetze_mit(&inhalt, "Hallo!"), 0);
    assert!(!String::from_utf8_lossy(&inhalt).contains("k1"));
    for antwort in [&erste, &zweite, &andere, &drei] {
        assert!(!antwort.json.to_string().contains("k1"), "{}", antwort.json);
    }
    for ereignis in &strom.ereignisse {
        assert!(
            !ereignis.data.to_string().contains("k1"),
            "{}",
            ereignis.data
        );
    }
}

/// R00012 F0046 „Wiederholung“: derselbe Schlüssel an einen anderen Channel → 422 `schluessel`; ein Schlüssel
/// einer anderen Kennung ist ein anderer Schlüssel → 201.
#[tokio::test]
async fn ein_schluessel_gilt_je_kennung_und_je_channel() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[]);
    let beta = Absender::von("127.0.0.1");
    let gamma = Absender::von("127.0.0.1");
    let token_beta = beitreten(&beta, &instanz.adresse, "beta", None).await;
    let token_gamma = beitreten(&gamma, &instanz.adresse, "gamma", None).await;
    let basis = &instanz.adresse;

    let erste = mit_schluessel(&beta, basis, &token_beta, "allgemein", "Hallo", "k1").await;
    let technik = mit_schluessel(&beta, basis, &token_beta, "technik", "Hallo", "k1").await;
    let fremd = mit_schluessel(&gamma, basis, &token_gamma, "allgemein", "Hallo", "k1").await;

    assert_eq!(erste.status, 201);
    assert_eq!(technik.status, 422, "{}", technik.json);
    assert_eq!(technik.json["regel"], "schluessel");
    assert_eq!(fremd.status, 201, "{}", fremd.json);
    assert_ne!(
        fremd.json["nachricht"]["zeit"],
        erste.json["nachricht"]["zeit"]
    );
    assert_eq!(
        saetze_mit(&tagesinhalt(wurzel.path(), "allgemein"), "Hallo"),
        2
    );
}

/// R00012 F0046 „Gleichzeitig“: 20 gleichzeitige `POST` von `beta` mit `k2` → genau eine 201, 19 × 200, alle mit
/// derselben Zeit; ein Satz im Log; `alpha` liest genau ein Ereignis dazu.
#[tokio::test]
async fn zwanzig_gleichzeitige_wiederholungen_ergeben_einen_satz() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[]);
    let alpha = Absender::von("127.0.0.1");
    let beta = std::sync::Arc::new(Absender::von("127.0.0.1"));
    let token_alpha = beitreten(&alpha, &instanz.adresse, "alpha", None).await;
    let token = beitreten(&beta, &instanz.adresse, "beta", None).await;
    let mut strom = Strom::oeffnen(&alpha, &instanz.adresse, &token_alpha).await;

    let laeufe: Vec<_> = (0..20)
        .map(|_| {
            let beta = std::sync::Arc::clone(&beta);
            let basis = instanz.adresse.clone();
            let token = token.clone();
            tokio::spawn(async move {
                mit_schluessel(&beta, &basis, &token, "allgemein", "Hallo", "k2").await
            })
        })
        .collect();
    let mut antworten = Vec::new();
    for lauf in laeufe {
        antworten.push(lauf.await.expect("Lauf"));
    }
    strom.bis(Duration::from_secs(10), ist_nachricht).await;
    strom.mitlesen(Duration::from_millis(500)).await;

    let mut status: Vec<_> = antworten.iter().map(|a| a.status).collect();
    status.sort_unstable();
    assert_eq!(status[0], 200);
    assert_eq!(
        status.iter().filter(|s| **s == 201).count(),
        1,
        "{status:?}"
    );
    assert_eq!(
        status.iter().filter(|s| **s == 200).count(),
        19,
        "{status:?}"
    );
    let zeiten: std::collections::BTreeSet<_> = antworten
        .iter()
        .map(|a| a.json["nachricht"]["zeit"].to_string())
        .collect();
    assert_eq!(zeiten.len(), 1, "{zeiten:?}");
    assert_eq!(
        saetze_mit(&tagesinhalt(wurzel.path(), "allgemein"), "Hallo"),
        1
    );
    assert_eq!(
        strom.ereignisse.iter().filter(|e| ist_nachricht(e)).count(),
        1
    );
}

/// R00012 F0046 „Wiederholung bucht“: `beta` sendet mit Schlüsseln, bis der Eimer leer ist (mindestens die 30 des
/// Vorrats); die Wiederholung der letzten durchgegangenen unmittelbar danach → 429 (keine Ausnahme vom Konto);
/// nach `Retry-After` dieselbe Wiederholung → 200 mit der ursprünglichen Zeit.
#[tokio::test]
async fn eine_wiederholung_bucht_am_schreibkonto_wie_ein_erster_versuch() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[]);
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, &instanz.adresse, "beta", None).await;
    let basis = &instanz.adresse;

    let mut durch = Vec::new();
    for nummer in 0..60 {
        let schluessel = format!("s{nummer}");
        let antwort = mit_schluessel(
            &beta,
            basis,
            &token,
            "allgemein",
            &format!("n{nummer}"),
            &schluessel,
        )
        .await;
        if antwort.status == 429 {
            break;
        }
        assert_eq!(antwort.status, 201, "{}", antwort.json);
        durch.push((nummer, antwort));
    }
    let (letzte, ursprung) = durch.last().expect("durchgegangen");
    let text = format!("n{letzte}");
    let schluessel = format!("s{letzte}");
    let gebremst = mit_schluessel(&beta, basis, &token, "allgemein", &text, &schluessel).await;
    let warten: u64 = gebremst
        .kopf
        .get("retry-after")
        .and_then(|wert| wert.to_str().ok())
        .and_then(|wert| wert.parse().ok())
        .expect("Retry-After");
    tokio::time::sleep(Duration::from_secs(warten)).await;
    let spaeter = mit_schluessel(&beta, basis, &token, "allgemein", &text, &schluessel).await;

    assert!(durch.len() >= 30, "{}", durch.len());
    assert_eq!(gebremst.status, 429, "{}", gebremst.json);
    assert_eq!(spaeter.status, 200, "{}", spaeter.json);
    assert_eq!(
        spaeter.json["nachricht"]["zeit"],
        ursprung.json["nachricht"]["zeit"]
    );
    assert_eq!(
        saetze_mit(&tagesinhalt(wurzel.path(), "allgemein"), &text),
        1
    );
}
