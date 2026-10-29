//! R00029 F0097 — Anmelden über das echte Programm (B0427): Hauptweg, Abweisungen der Anmeldung, Namensregel,
//! zu groß, offene und laufende Hochladungen je Kennung, laufende Hochladungen belegen den Platz, Platte.

mod gemeinsam;

use gemeinsam::Absender;
use gemeinsam::anhaenge::{
    MIB, Q, ablegen, anmelden, anmelden_roh, gesundheit, hochladen, id, roh_antwort, roh_oeffnen,
    starten,
};
use gemeinsam::senden::{beitreten, ordner, roh_senden};
use serde_json::json;
use std::time::Duration;
use tokio::io::AsyncWriteExt;

fn ist_id(text: &str) -> bool {
    text.len() == 26
        && text
            .bytes()
            .all(|z| z.is_ascii_lowercase() || (b'2'..=b'7').contains(&z))
}

fn zeit(text: &serde_json::Value) -> chrono::DateTime<chrono::FixedOffset> {
    chrono::DateTime::parse_from_rfc3339(text.as_str().expect("Zeit")).expect("RFC 3339")
}

/// Hauptweg: 307.200 → 201 mit ID, `hochladen`, `anlauf_bis` ≈ + 60 s, `hochladen_bis` ≈ + 60,586 s (± 1 s gegen
/// die Uhr des Prüfers).
#[tokio::test]
async fn hauptweg_meldet_an_mit_id_hochladen_und_fristen() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, &instanz.adresse, "beta", Some("agent")).await;

    let vorher = chrono::Utc::now();
    let antwort = anmelden(&beta, &instanz.adresse, &token, "bericht.pdf", 307_200).await;

    assert_eq!(antwort.status, 201, "{}", antwort.json);
    let anhang = &antwort.json["anhang"];
    let id = anhang["id"].as_str().expect("id");
    assert!(ist_id(id), "{id}");
    assert_eq!(anhang["name"], "bericht.pdf");
    assert_eq!(anhang["bytes"], 307_200);
    assert_eq!(anhang["hochladen"], format!("/api/anhaenge/{id}"));
    let bis = (zeit(&anhang["hochladen_bis"]).to_utc() - vorher).num_milliseconds();
    assert!(
        (59_586..=61_586).contains(&bis),
        "hochladen_bis {bis} ms nach der Anfrage"
    );
    let anlauf = (zeit(&anhang["anlauf_bis"]).to_utc() - vorher).num_milliseconds();
    assert!(
        (59_000..=61_000).contains(&anlauf),
        "anlauf_bis {anlauf} ms"
    );
    assert_eq!(antwort.json.as_object().map(|o| o.len()), Some(1));
}

/// R00029 „Abweisungen der Anmeldung“ und Namensregel über das Programm: jede Antwort JSON.
#[tokio::test]
async fn abweisungen_der_anmeldung_sind_json_in_der_reihenfolge() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, &instanz.adresse, "beta", Some("agent")).await;
    let basis = instanz.adresse.as_str();

    let ohne = anmelden_roh(&beta, basis, None, r#"{"name":"a","bytes":1}"#.into()).await;
    assert_eq!(ohne.status, 401);
    let fremd = gemeinsam::api_antwort(
        beta.anfrage(reqwest::Method::POST, format!("{basis}api/anhaenge"))
            .header("Content-Type", "application/json")
            .header("Origin", "http://evil.example")
            .header("X-LocalchatA-Sitzung", &token)
            .body(r#"{"name":"a","bytes":1}"#),
    )
    .await;
    assert_eq!(fremd.status, 403);
    let regel = |roh: &'static str| {
        let (beta, token) = (&beta, &token);
        async move { anmelden_roh(beta, basis, Some(token), roh.into()).await }
    };
    let feld = regel(r#"{"name":"a","bytes":1,"x":1}"#).await;
    assert_eq!(
        (feld.status, feld.json["regel"].as_str()),
        (422, Some("feld"))
    );
    for falsch in [
        r#"{"name":"a","bytes":-1}"#,
        r#"{"name":"a","bytes":1.5}"#,
        r#"{"name":"a","bytes":"1"}"#,
        r#"{"name":"a","bytes":18446744073709551616}"#,
    ] {
        let antwort = regel(falsch).await;
        assert_eq!(
            (antwort.status, antwort.json["regel"].as_str()),
            (422, Some("bytes")),
            "{falsch}"
        );
    }
    let leer = regel(r#"{"name":"a","bytes":0}"#).await;
    assert_eq!(
        (leer.status, leer.json["regel"].as_str()),
        (422, Some("leer"))
    );
    let doppelt = regel(r#"{"name":"a","bytes":1,"bytes":2}"#).await;
    assert_eq!(doppelt.status, 422);
    assert_eq!(doppelt.json["regel"], "feld");
    assert_eq!(doppelt.json["feld"], "bytes");
    assert_eq!(doppelt.json["grund"], "doppelt");
    let gross = anmelden_roh(
        &beta,
        basis,
        Some(&token),
        format!(r#"{{"name":"{}","bytes":1}}"#, "a".repeat(2_100)),
    )
    .await;
    assert_eq!(gross.status, 413);
    assert!(gross.json["fehler"].is_string());
    let name = anmelden(&beta, basis, &token, "../x", 1).await;
    assert_eq!(
        (
            name.status,
            name.json["regel"].as_str(),
            name.json["grund"].as_str()
        ),
        (422, Some("name"), Some("zeichen"))
    );
    let leerer_name = anmelden(&beta, basis, &token, "", 1).await;
    assert_eq!(leerer_name.json["grund"], "leer");
}

/// R00029 „Größer als die ganze Quote“: 10.485.761 → 422 `zu_gross` mit Text; 10.485.760 → 201.
#[tokio::test]
async fn groesser_als_die_ganze_anhang_quote_ist_422_zu_gross() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, &instanz.adresse, "beta", Some("agent")).await;

    let zu_gross = anmelden(&beta, &instanz.adresse, &token, "a.bin", Q + 1).await;

    assert_eq!(zu_gross.status, 422);
    assert_eq!(
        zu_gross.json,
        json!({
            "fehler": "Nicht hochgeladen: die Datei ist größer als der ganze Platz für Anhänge dieser Instanz (10,0 MiB).",
            "regel": "zu_gross",
            "hoechstens": 10_485_760,
        })
    );
    id(&anmelden(&beta, &instanz.adresse, &token, "a.bin", Q).await);
}

/// Q ganz beansprucht → die nächste Anmeldung 507 `hochladungen` mit `Retry-After: 30` und den Zahlen.
#[tokio::test]
async fn laufende_hochladungen_belegen_den_platz_507_mit_retry_after_30() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let beta = Absender::von("127.0.0.1");
    let alpha = Absender::von("127.0.0.1");
    let token_beta = beitreten(&beta, &instanz.adresse, "beta", Some("agent")).await;
    let token_alpha = beitreten(&alpha, &instanz.adresse, "alpha", Some("agent")).await;
    id(&anmelden(&beta, &instanz.adresse, &token_beta, "a.bin", Q).await);

    let antwort = anmelden(&alpha, &instanz.adresse, &token_alpha, "b.bin", 1).await;

    assert_eq!(antwort.status, 507);
    assert_eq!(
        antwort.kopf.get("retry-after").map(|w| w.to_str().ok()),
        Some(Some("30"))
    );
    assert_eq!(
        antwort.json,
        json!({
            "fehler": "Nicht hochgeladen: laufende Hochladungen belegen gerade den Platz für Anhänge — noch 0,0 MiB frei. Gleich noch einmal versuchen.",
            "regel": "quote",
            "grund": "hochladungen",
            "belegt": 10_485_760,
            "gebunden": 0,
            "reserviert": 10_485_760,
            "hoechstens": 10_485_760,
            "frei": 0,
            "braucht": 65_536,
        })
    );
}

/// Neun Anhänge zu 1 MiB gebunden (G = 9.437.184).
async fn neun_gebunden(absender: &Absender, basis: &str, token: &str) {
    for nummer in 0..9 {
        let id = ablegen(
            absender,
            basis,
            token,
            &format!("n{nummer}.bin"),
            vec![1u8; MIB as usize],
        )
        .await;
        let koerper = json!({"text": "", "anhaenge": [id]})
            .to_string()
            .into_bytes();
        let gesendet = roh_senden(
            absender,
            basis,
            Some(token),
            "allgemein",
            "application/json",
            koerper,
        )
        .await;
        assert_eq!(gesendet.status, 201, "{}", gesendet.json);
    }
}

/// Befund #9 B1, R00029 „Volle Anhang-Quote, Rechenbeispiel“ (QA6′): neun gebunden, Anmelden 2.097.152 → 507 ohne
/// `Retry-After` mit dem Körper wörtlich; nichts beansprucht, `anhang_abgewiesen` 1. Variante mit einem
/// angemeldeten Byte (`reserviert` 65.536): `belegt` = G + A, `gebunden` = G, `frei` = Q − G − A. Mutanten „`frei` und
/// `braucht` vertauscht“ und „`belegt` = `gebunden`“ werden rot.
#[tokio::test]
async fn volle_anhang_quote_507_voll_woertlich() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, basis, "beta", Some("agent")).await;
    neun_gebunden(&beta, basis, &token).await;

    let voll = anmelden(&beta, basis, &token, "g.bin", 2 * MIB).await;

    assert_eq!(voll.status, 507);
    assert_eq!(voll.kopf.get("retry-after"), None);
    assert_eq!(
        voll.json,
        json!({
            "fehler": "Nicht hochgeladen: der Platz für Anhänge ist voll — frei sind 1,0 MiB, die Datei braucht 2,0 MiB. Platz entsteht, wenn Anhänge gelöscht werden oder ältere Tage entfallen; Text kann weiter gesendet werden.",
            "regel": "quote",
            "grund": "voll",
            "belegt": 9_437_184,
            "gebunden": 9_437_184,
            "reserviert": 0,
            "hoechstens": 10_485_760,
            "frei": 1_048_576,
            "braucht": 2_097_152,
        })
    );
    let stand = gesundheit(basis).await;
    assert_eq!(
        (&stand["reserviert"], &stand["anhang_abgewiesen"]),
        (&json!(0), &json!(1))
    );

    id(&anmelden(&beta, basis, &token, "eins.bin", 1).await);
    let mit_anspruch = anmelden(&beta, basis, &token, "g.bin", 2 * MIB).await;

    assert_eq!(mit_anspruch.status, 507);
    assert_eq!(mit_anspruch.kopf.get("retry-after"), None);
    assert_eq!(
        mit_anspruch.json,
        json!({
            "fehler": "Nicht hochgeladen: der Platz für Anhänge ist voll — frei sind 0,9 MiB, die Datei braucht 2,0 MiB. Platz entsteht, wenn Anhänge gelöscht werden oder ältere Tage entfallen; Text kann weiter gesendet werden.",
            "regel": "quote",
            "grund": "voll",
            "belegt": 9_502_720,
            "gebunden": 9_437_184,
            "reserviert": 65_536,
            "hoechstens": 10_485_760,
            "frei": 983_040,
            "braucht": 2_097_152,
        })
    );
    let stand = gesundheit(basis).await;
    assert_eq!(
        (&stand["reserviert"], &stand["anhang_abgewiesen"]),
        (&json!(65_536), &json!(2))
    );
}

/// ADR-0046 Ergänzung „Karte #9 Nacharbeit F1“, Erste Tests (Integration, wörtlich) und R00029 „Text mit zehn
/// Anhängen“: zehn Dateien zu je 1 KiB anmelden → zehnmal 201, je `PUT` 200; eine elfte Anmeldung vor dem Senden →
/// 429 `offen_kennung` 10 mit `Retry-After: 5`; Senden mit zehn IDs → 201 mit zehn Metazeilen in Sendefolge. Fünf
/// `PUT` derselben Kennung gleichzeitig (Körper verzögert) → genau einer 429 `laufend_kennung` 4 mit
/// `Connection: close`, danach derselbe `PUT` → 200.
#[tokio::test]
async fn zehn_offen_und_vier_laufend_je_kennung() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &["--anhangquote", "64MiB"], &[]);
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, basis, "beta", Some("agent")).await;
    let mut ids = Vec::new();
    for nummer in 0..10 {
        let angemeldet =
            id(&anmelden(&beta, basis, &token, &format!("d{nummer}.bin"), 1_024).await);
        let antwort = hochladen(&beta, basis, &token, &angemeldet, vec![nummer; 1_024]).await;
        assert_eq!(antwort.status, 200, "{}", antwort.json);
        ids.push(angemeldet);
    }
    let elfte = anmelden(&beta, basis, &token, "elf.bin", 1_024).await;
    assert_eq!(
        (
            elfte.status,
            &elfte.json["grenze"],
            &elfte.json["hoechstens"]
        ),
        (429, &json!("offen_kennung"), &json!(10)),
        "{}",
        elfte.json
    );
    assert_eq!(
        elfte.json["fehler"],
        "Nicht angemeldet: höchstens 10 Dateien gleichzeitig offen — erst senden oder abbrechen."
    );
    assert_eq!(
        elfte.kopf.get("retry-after").map(|w| w.to_str().ok()),
        Some(Some("5"))
    );
    let gesendet = roh_senden(
        &beta,
        basis,
        Some(&token),
        "allgemein",
        "application/json",
        json!({"text": "zehn", "anhaenge": ids})
            .to_string()
            .into_bytes(),
    )
    .await;
    assert_eq!(gesendet.status, 201, "{}", gesendet.json);
    let reihenfolge: Vec<&str> = gesendet.json["nachricht"]["anhaenge"]
        .as_array()
        .expect("Liste")
        .iter()
        .map(|anhang| anhang["id"].as_str().expect("id"))
        .collect();
    assert_eq!(
        reihenfolge,
        ids.iter().map(String::as_str).collect::<Vec<_>>()
    );

    let mut fuenf = Vec::new();
    for _ in 0..5 {
        let angemeldet = id(&anmelden(&beta, basis, &token, "f.bin", 1_024).await);
        let kopf = format!(
            "PUT /api/anhaenge/{angemeldet} HTTP/1.1\r\nHost: 127.0.0.1\r\nX-LocalchatA-Sitzung: {token}\r\nContent-Type: application/octet-stream\r\nContent-Length: 1024"
        );
        fuenf.push((
            angemeldet,
            roh_oeffnen(instanz.port(), "127.0.0.1", &kopf).await,
        ));
    }
    let mut abgewiesen = Vec::new();
    let mut laufend = Vec::new();
    for (angemeldet, mut verbindung) in fuenf {
        let antwort = roh_antwort(&mut verbindung, Duration::from_millis(1_500)).await;
        if antwort.is_empty() {
            laufend.push(verbindung);
        } else {
            abgewiesen.push((angemeldet, antwort));
        }
    }
    assert_eq!(abgewiesen.len(), 1, "{abgewiesen:?}");
    let (wieder, antwort) = abgewiesen.remove(0);
    assert!(antwort.starts_with("HTTP/1.1 429"), "{antwort}");
    let klein = antwort.to_ascii_lowercase();
    assert!(
        klein.contains("connection: close") && klein.contains("retry-after: 5"),
        "{antwort}"
    );
    assert!(
        antwort.contains("\"grenze\":\"laufend_kennung\"") && antwort.contains("\"hoechstens\":4"),
        "{antwort}"
    );
    assert!(
        antwort.contains("Gerade zu viele Hochladungen — gleich noch einmal versuchen."),
        "{antwort}"
    );
    for mut verbindung in laufend {
        verbindung.write_all(&[1u8; 1_024]).await.expect("Körper");
        let antwort = roh_antwort(&mut verbindung, Duration::from_secs(5)).await;
        assert!(antwort.starts_with("HTTP/1.1 200"), "{antwort}");
    }
    let nochmal = hochladen(&beta, basis, &token, &wieder, vec![2u8; 1_024]).await;
    assert_eq!(nochmal.status, 200, "{}", nochmal.json);
}

/// R00029 „Platte“: mit `--anhangquote aus` → 1 PiB → 507 `platte` mit `frei` > 0, nichts reserviert.
#[tokio::test]
async fn ein_pebibyte_ist_507_platte() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &["--anhangquote", "aus"], &[]);
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, &instanz.adresse, "beta", Some("agent")).await;

    let antwort = anmelden(
        &beta,
        &instanz.adresse,
        &token,
        "riesig.img",
        1_125_899_906_842_624,
    )
    .await;

    assert_eq!(antwort.status, 507, "{}", antwort.json);
    assert_eq!(antwort.json["regel"], "platte");
    assert!(antwort.json["frei"].as_u64().is_some_and(|frei| frei > 0));
    assert!(antwort.kopf.get("retry-after").is_none());
    let gross = anmelden(&beta, &instanz.adresse, &token, "a.bin", 10 * MIB + 1).await;
    assert_eq!(
        gross.status, 201,
        "ohne Anhang-Quote kein zu_gross: {}",
        gross.json
    );
}
