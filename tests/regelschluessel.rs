//! R00018 F0059 Regelschlüssel als Grund (ADR-0042 Ergänzung #24 Punkt 3) — über das echte Programm auf Stufe
//! debug: jede abweisende 4xx trägt einen festen Regelschlüssel als `grund`, bei bekannter Sitzung `kennung=`.

mod gemeinsam;

use gemeinsam::protokoll::{wert, zeilen};
use gemeinsam::{Absender, starten_mit};
use serde_json::json;

/// Die DEBUG-Zeilen mit `status=4…` in der Reihenfolge der Datei.
fn abweisungen(ordner: &std::path::Path) -> Vec<String> {
    zeilen(ordner, "2026-10-28")
        .into_iter()
        .filter(|zeile| zeile.contains(" DEBUG "))
        .filter(|zeile| wert(zeile, "status").is_some_and(|status| status.starts_with('4')))
        .collect()
}

#[tokio::test]
async fn jede_abweisung_nennt_ihren_regelschluessel() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let instanz = starten_mit(
        wurzel.path(),
        &["--port", "0", "--host", "127.0.0.1", "--kein-browser"],
        &[
            ("LOCALCHATA_JETZT", "2026-10-28T12:00:00+01:00"),
            ("TZ", "Europe/Berlin"),
            ("LOCALCHATA_PROTOKOLL_STUFE", "debug"),
        ],
    );
    let basis = &instanz.adresse;
    let beta = Absender::von("127.0.0.1");
    let client = reqwest::Client::new();
    let status = |antwort: reqwest::Response| antwort.status().as_u16();

    let falscher_name = beta.beitreten(basis, json!({"name": "a b"}), None).await;
    let token = beta
        .beitreten(basis, json!({"name": "beta"}), None)
        .await
        .json["token"]
        .as_str()
        .expect("Token")
        .to_string();
    let vergeben = Absender::von("127.0.0.1")
        .beitreten(basis, json!({"name": "beta"}), None)
        .await;
    let unbekannt = status(
        client
            .get(format!("{basis}api/gibtsnicht"))
            .send()
            .await
            .expect("404"),
    );
    let ohne_sitzung = status(
        client
            .get(format!("{basis}api/channels"))
            .send()
            .await
            .expect("401"),
    );
    let fremde_sitzung = status(
        client
            .get(format!("{basis}api/channels"))
            .header("X-LocalchatA-Sitzung", "x".repeat(43))
            .send()
            .await
            .expect("401"),
    );
    let anlegen = |name: &str| {
        client
            .post(format!("{basis}api/channels"))
            .header("X-LocalchatA-Sitzung", &token)
            .json(&json!({ "name": name }))
            .send()
    };
    let channel_name = status(anlegen("A B").await.expect("422"));
    assert_eq!(status(anlegen("allgemein").await.expect("201")), 201);
    let channel_da = status(anlegen("allgemein").await.expect("409"));
    let verlauf_fehlt = status(
        client
            .get(format!("{basis}api/channels/gibtsnicht/nachrichten"))
            .header("X-LocalchatA-Sitzung", &token)
            .send()
            .await
            .expect("404"),
    );
    let senden = |text: &str, schluessel: Option<&str>| {
        let mut anfrage = client
            .post(format!("{basis}api/channels/allgemein/nachrichten"))
            .header("X-LocalchatA-Sitzung", &token)
            .json(&json!({ "text": text }));
        if let Some(schluessel) = schluessel {
            anfrage = anfrage.header("Idempotency-Key", schluessel);
        }
        anfrage.send()
    };
    let leer = status(senden("", None).await.expect("422"));
    let schluessel_falsch = status(senden("Hallo", Some("ä")).await.expect("422"));
    assert_eq!(status(senden("Hallo", Some("k1")).await.expect("201")), 201);
    let schluessel_anders = status(senden("Anders", Some("k1")).await.expect("422"));
    let frist = status(
        client
            .put(format!("{basis}api/channels/allgemein/frist"))
            .header("X-LocalchatA-Sitzung", &token)
            .json(&json!({ "tage": 0 }))
            .send()
            .await
            .expect("422"),
    );

    assert_eq!(
        [
            falscher_name.status,
            vergeben.status,
            unbekannt,
            ohne_sitzung,
            fremde_sitzung,
            channel_name,
            channel_da,
            verlauf_fehlt,
            leer,
            schluessel_falsch,
            schluessel_anders,
            frist,
        ],
        [422, 409, 404, 401, 401, 422, 409, 404, 422, 422, 422, 422]
    );
    let zeilen = abweisungen(wurzel.path());
    let gruende: Vec<String> = zeilen
        .iter()
        .map(|zeile| wert(zeile, "grund").unwrap_or_default())
        .collect();
    assert_eq!(
        gruende,
        [
            "name_ungueltig",
            "name_vergeben",
            "unbekannt",
            "sitzung_fehlt",
            "sitzung_unbekannt",
            "name_ungueltig",
            "channel_existiert",
            "channel_unbekannt",
            "text_leer",
            "schluessel_ungueltig",
            "schluessel_anders",
            "frist_ungueltig",
        ],
        "{zeilen:#?}"
    );
    for zeile in &zeilen {
        let grund = wert(zeile, "grund").unwrap_or_default();
        assert!(
            !grund.is_empty() && grund.bytes().all(|z| z.is_ascii_lowercase() || z == b'_'),
            "{zeile}"
        );
    }
    assert_eq!(wert(&zeilen[0], "status").as_deref(), Some("422"));
    assert_eq!(wert(&zeilen[0], "herkunft").as_deref(), Some("127.0.0.1"));
    for zeile in &zeilen[5..] {
        assert_eq!(
            wert(zeile, "kennung").as_deref(),
            Some("beta@127.0.0.1"),
            "{zeile}"
        );
    }
}
