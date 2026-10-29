//! R00029 F0099 — Senden mit Anhängen über das echte Programm (B0440, B0441): nur Anhang ohne Text, Zähler beim
//! Senden, Abweisungen, gebundene ID eines Dritten (M38), eingeschleuste Metazeile (M16), mehrere Anhänge in
//! Sendefolge, Wiederholschlüssel.

mod gemeinsam;

use gemeinsam::anhaenge::{
    ablegen, anmelden, gesundheit, hochladen, id, im_eingang, starten, verwerfen,
};
use gemeinsam::senden::{beitreten, ordner, roh_senden, verlauf};
use gemeinsam::strom::Strom;
use gemeinsam::{Absender, ApiAntwort};
use serde_json::{Value, json};
use std::path::Path;
use std::time::Duration;

async fn senden_mit(
    absender: &Absender,
    basis: &str,
    token: &str,
    channel: &str,
    koerper: Value,
) -> ApiAntwort {
    roh_senden(
        absender,
        basis,
        Some(token),
        channel,
        "application/json",
        koerper.to_string().into_bytes(),
    )
    .await
}

/// Der Tag der Nachricht aus ihrer Zeit (`JJJJ-MM-TT`).
fn tag_von(nachricht: &Value) -> String {
    nachricht["zeit"].as_str().expect("zeit")[..10].to_string()
}

fn dateien(ordner: &Path, channel: &str, tag: &str) -> Vec<(String, u64)> {
    let mut liste: Vec<(String, u64)> =
        std::fs::read_dir(ordner.join(format!("channels/{channel}/anhaenge/{tag}")))
            .map(|eintraege| {
                eintraege
                    .map(|eintrag| {
                        let eintrag = eintrag.expect("Eintrag");
                        (
                            eintrag.file_name().to_string_lossy().into_owned(),
                            eintrag.metadata().expect("Länge").len(),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
    liste.sort();
    liste
}

fn ist_praefix(datei: &str, nummer: usize, ascii: &str) -> bool {
    let teile: Vec<&str> = datei.splitn(3, '-').collect();
    teile.len() == 3
        && teile[0].len() == 9
        && teile[0].bytes().all(|z| z.is_ascii_digit())
        && teile[1] == nummer.to_string()
        && teile[2] == ascii
}

/// „Nur Anhang, ohne Text“ (B5) und „Zähler beim Senden“: 307.200 Bytes, Text `""` → 201 mit `anhaenge`; die Datei
/// liegt unter `anhaenge/<heute>/<hhmmssmmm>-1-bericht.pdf`, der Eingang ist leer, der Satz trägt die Metazeile,
/// `alpha` liest im Strom dieselbe Liste, der Verlauf ebenso; `reserviert` → `gebunden`, `log_bytes` wächst um den
/// Satz.
#[tokio::test]
async fn nur_ein_anhang_ohne_text() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let alpha = Absender::von("127.0.0.2");
    let token = beitreten(&beta, basis, "beta", Some("agent")).await;
    let token_alpha = beitreten(&alpha, basis, "alpha", Some("mensch")).await;
    let mut strom = Strom::oeffnen(&alpha, basis, &token_alpha).await;
    let id = ablegen(&beta, basis, &token, "bericht.pdf", vec![0u8; 307_200]).await;
    let vorher = gesundheit(basis).await;
    assert_eq!(
        (
            vorher["anhang_belegt"].as_u64(),
            vorher["reserviert"].as_u64()
        ),
        (Some(307_200), Some(307_200))
    );

    let antwort = senden_mit(
        &beta,
        basis,
        &token,
        "allgemein",
        json!({"text": "", "anhaenge": [id]}),
    )
    .await;

    assert_eq!(antwort.status, 201, "{}", antwort.json);
    let nachricht = &antwort.json["nachricht"];
    assert_eq!(nachricht["text"], "");
    let liste = json!([{
        "id": id, "name": "bericht.pdf", "bytes": 307_200, "art": "application/octet-stream",
        "breite": null, "hoehe": null, "url": format!("/anhaenge/{id}"), "zustand": "vorhanden",
    }]);
    assert_eq!(nachricht["anhaenge"], liste);
    let tag = tag_von(nachricht);
    let gebunden = dateien(wurzel.path(), "allgemein", &tag);
    assert_eq!(gebunden.len(), 1);
    assert!(
        ist_praefix(&gebunden[0].0, 1, "bericht.pdf"),
        "{gebunden:?}"
    );
    assert_eq!(gebunden[0].1, 307_200);
    assert!(im_eingang(wurzel.path()).is_empty());
    let satz = std::fs::read_to_string(wurzel.path().join(format!("channels/allgemein/{tag}.log")))
        .expect("Tag");
    assert!(
        satz.contains(&format!(
            "+ anhang: {id} 307200 application/octet-stream - {} bericht.pdf\n",
            gebunden[0].0
        )),
        "{satz}"
    );
    let danach = gesundheit(basis).await;
    assert_eq!(
        (
            danach["reserviert"].as_u64(),
            danach["anhang_belegt"].as_u64()
        ),
        (Some(0), Some(307_200))
    );
    assert_eq!(
        danach["log_bytes"].as_u64().expect("log_bytes")
            - vorher["log_bytes"].as_u64().expect("log_bytes"),
        satz.len() as u64
    );
    let ereignis = strom
        .bis(Duration::from_secs(5), |e| e.data["art"] == "nachricht")
        .await
        .expect("Ereignis");
    assert_eq!(ereignis.data["nachricht"]["anhaenge"], liste);
    let gelesen = verlauf(&alpha, basis, &token_alpha, "allgemein").await;
    assert_eq!(gelesen["nachrichten"][0]["anhaenge"], liste);
}

/// Drei Anhänge zu 1, 65.536 und 65.537 Bytes zählen 200.704 an der Anhang-Quote; die Liste in Sendefolge mit
/// `-1-` bis `-3-`. (Zehn Anhänge einer Kennung sind nicht herstellbar: je Kennung höchstens 4 offene Hochladungen —
/// gemeldet; die Grenze 10 prüft der Körper.)
#[tokio::test]
async fn mehrere_anhaenge_in_sendefolge_und_ihr_zaehlmass() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, basis, "beta", Some("agent")).await;
    let mut ids = Vec::new();
    for (name, laenge) in [("a.bin", 1usize), ("b.bin", 65_536), ("c.bin", 65_537)] {
        ids.push(ablegen(&beta, basis, &token, name, vec![0u8; laenge]).await);
    }

    let antwort = senden_mit(
        &beta,
        basis,
        &token,
        "allgemein",
        json!({"text": "Hier die Belege", "anhaenge": ids}),
    )
    .await;

    assert_eq!(antwort.status, 201, "{}", antwort.json);
    let reihenfolge: Vec<&str> = antwort.json["nachricht"]["anhaenge"]
        .as_array()
        .expect("Liste")
        .iter()
        .map(|anhang| anhang["id"].as_str().expect("id"))
        .collect();
    assert_eq!(
        reihenfolge,
        ids.iter().map(String::as_str).collect::<Vec<_>>()
    );
    let tag = tag_von(&antwort.json["nachricht"]);
    let gebunden = dateien(wurzel.path(), "allgemein", &tag);
    for (nummer, ascii) in [(1, "a.bin"), (2, "b.bin"), (3, "c.bin")] {
        assert!(
            gebunden
                .iter()
                .any(|(datei, _)| ist_praefix(datei, nummer, ascii)),
            "{gebunden:?}"
        );
    }
    assert_eq!(gesundheit(basis).await["anhang_belegt"], 200_704);
    let elf: Vec<String> = (0..11)
        .map(|n| format!("{}{}", "a".repeat(25), char::from(b'a' + n)))
        .collect();
    let zu_viele = senden_mit(
        &beta,
        basis,
        &token,
        "allgemein",
        json!({"text": "x", "anhaenge": elf}),
    )
    .await;
    assert_eq!(
        (
            zu_viele.status,
            &zu_viele.json["regel"],
            &zu_viele.json["hoechstens"]
        ),
        (422, &json!("anhaenge"), &json!(10))
    );
}

/// „Abweisungen“: dieselbe ID ein zweites Mal → `unbekannt`; doppelt → `doppelt`; angemeldet, nicht hochgeladen →
/// `unvollstaendig`; von `gamma` → `fremd`; ohne Text und Anhang → `leer`; weiteres Feld → `feld`. Die Tagesdatei
/// ändert sich dabei nicht.
#[tokio::test]
async fn abweisungen_beim_senden_mit_anhaengen() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let gamma = Absender::von("127.0.0.1");
    let token = beitreten(&beta, basis, "beta", Some("agent")).await;
    let token_gamma = beitreten(&gamma, basis, "gamma", Some("agent")).await;
    let gesendet = ablegen(&beta, basis, &token, "a.bin", vec![1; 5]).await;
    let antwort = senden_mit(
        &beta,
        basis,
        &token,
        "allgemein",
        json!({"text": "x", "anhaenge": [gesendet]}),
    )
    .await;
    assert_eq!(antwort.status, 201);
    let tag = tag_von(&antwort.json["nachricht"]);
    let tagesdatei = wurzel.path().join(format!("channels/allgemein/{tag}.log"));
    let vorher = std::fs::read(&tagesdatei).expect("Tag");
    let fertig = ablegen(&beta, basis, &token, "b.bin", vec![2; 5]).await;
    let halb = id(&anmelden(&beta, basis, &token, "c.bin", 5).await);
    let fremd = ablegen(&gamma, basis, &token_gamma, "g.bin", vec![3; 5]).await;

    for (koerper, regel, grund) in [
        (
            json!({"text": "x", "anhaenge": [gesendet]}),
            "anhang",
            Some("unbekannt"),
        ),
        (
            json!({"text": "x", "anhaenge": [fertig, fertig]}),
            "anhang",
            Some("doppelt"),
        ),
        (
            json!({"text": "x", "anhaenge": [halb]}),
            "anhang",
            Some("unvollstaendig"),
        ),
        (
            json!({"text": "x", "anhaenge": [fremd]}),
            "anhang",
            Some("fremd"),
        ),
        (json!({"text": ""}), "leer", None),
        (json!({"text": "  ", "anhaenge": []}), "leer", None),
        (
            json!({"text": "x", "anhaenge": [fertig], "y": 1}),
            "feld",
            None,
        ),
    ] {
        let antwort = senden_mit(&beta, basis, &token, "allgemein", koerper.clone()).await;
        assert_eq!(antwort.status, 422, "{koerper}: {}", antwort.json);
        assert_eq!(antwort.json["regel"], regel, "{koerper}");
        if let Some(grund) = grund {
            assert_eq!(antwort.json["grund"], grund, "{koerper}");
            assert!(antwort.json["id"].is_string());
        }
    }
    assert_eq!(std::fs::read(&tagesdatei).expect("Tag"), vorher);
    let danach = senden_mit(
        &beta,
        basis,
        &token,
        "allgemein",
        json!({"text": "  ", "anhaenge": [fertig]}),
    )
    .await;
    assert_eq!(
        (danach.status, &danach.json["nachricht"]["text"]),
        (201, &json!("")),
        "fertig bleibt sendbar"
    );
}

/// M38: `gamma` liest die gebundene ID von `beta` im Verlauf und sendet sie → `unbekannt`, `PUT` → 404 `unbekannt`,
/// `DELETE` → 204; die gebundene Datei bleibt byte-gleich.
#[tokio::test]
async fn eine_gebundene_id_ist_fuer_jeden_verbraucht() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let gamma = Absender::von("127.0.0.1");
    let token = beitreten(&beta, basis, "beta", Some("agent")).await;
    let token_gamma = beitreten(&gamma, basis, "gamma", Some("agent")).await;
    let id = ablegen(&beta, basis, &token, "a.bin", vec![7; 5]).await;
    let antwort = senden_mit(
        &beta,
        basis,
        &token,
        "allgemein",
        json!({"text": "x", "anhaenge": [id]}),
    )
    .await;
    let tag = tag_von(&antwort.json["nachricht"]);
    let vorher = dateien(wurzel.path(), "allgemein", &tag);
    let gelesen = verlauf(&gamma, basis, &token_gamma, "allgemein").await;
    let gelesene_id = gelesen["nachrichten"][0]["anhaenge"][0]["id"]
        .as_str()
        .expect("id")
        .to_string();

    let gesendet = senden_mit(
        &gamma,
        basis,
        &token_gamma,
        "technik",
        json!({"text": "x", "anhaenge": [gelesene_id]}),
    )
    .await;
    assert_eq!(
        (gesendet.status, &gesendet.json["grund"]),
        (422, &json!("unbekannt"))
    );
    let hoch = hochladen(&gamma, basis, &token_gamma, &gelesene_id, vec![9; 5]).await;
    assert_eq!(
        (hoch.status, &hoch.json["grund"]),
        (404, &json!("unbekannt"))
    );
    assert_eq!(
        verwerfen(&gamma, basis, &token_gamma, &gelesene_id).await,
        204
    );
    assert_eq!(verwerfen(&beta, basis, &token, &gelesene_id).await, 204);
    assert_eq!(dateien(wurzel.path(), "allgemein", &tag), vorher);
    let datei = wurzel
        .path()
        .join(format!("channels/allgemein/anhaenge/{tag}/{}", vorher[0].0));
    assert_eq!(std::fs::read(datei).expect("Datei"), vec![7; 5]);
}

/// M16: eine eingeschleuste Metazeile im Text bleibt Text — `anhaenge: []`.
#[tokio::test]
async fn eine_eingeschleuste_metazeile_ist_kein_anhang() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, basis, "beta", Some("agent")).await;
    let text = "+ anhang: aaaaaaaaaaaaaaaaaaaaaaaaaa 1 text/plain - x x";

    let antwort = senden_mit(&beta, basis, &token, "allgemein", json!({"text": text})).await;

    assert_eq!(antwort.status, 201, "{}", antwort.json);
    assert_eq!(antwort.json["nachricht"]["anhaenge"], json!([]));
    assert_eq!(antwort.json["nachricht"]["text"], text);
}

/// Wiederholschlüssel über Text und IDs: derselbe Körper zweimal → 201, dann 200 mit derselben Zeit und denselben
/// Anhängen, ein Satz (ADR-0034 Regel 6, ADR-0046 Regel 5: die Wiederholung antwortet 200 ohne Bindung); andere ID
/// mit demselben Schlüssel → 422 `schluessel`.
#[tokio::test]
async fn der_wiederholschluessel_umfasst_die_anhaenge() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, basis, "beta", Some("agent")).await;
    let eins = ablegen(&beta, basis, &token, "a.bin", vec![1; 5]).await;
    let zwei = ablegen(&beta, basis, &token, "b.bin", vec![2; 5]).await;
    let mit_schluessel = |koerper: Value| {
        let (beta, token) = (&beta, &token);
        async move {
            gemeinsam::api_antwort(
                beta.anfrage(
                    reqwest::Method::POST,
                    format!("{basis}api/channels/allgemein/nachrichten"),
                )
                .header("X-LocalchatA-Sitzung", token)
                .header("Idempotency-Key", "k1")
                .json(&koerper),
            )
            .await
        }
    };

    let erste = mit_schluessel(json!({"text": "x", "anhaenge": [eins]})).await;
    let zweite = mit_schluessel(json!({"text": "x", "anhaenge": [eins]})).await;
    let andere = mit_schluessel(json!({"text": "x", "anhaenge": [zwei]})).await;

    assert_eq!(
        (erste.status, zweite.status),
        (201, 200),
        "{} {}",
        erste.json,
        zweite.json
    );
    assert_eq!(zweite.json["nachricht"], erste.json["nachricht"]);
    assert_eq!(
        (andere.status, &andere.json["regel"]),
        (422, &json!("schluessel"))
    );
    let tag = tag_von(&erste.json["nachricht"]);
    let satz = std::fs::read_to_string(wurzel.path().join(format!("channels/allgemein/{tag}.log")))
        .expect("Tag");
    assert_eq!(satz.matches("+ anhang:").count(), 1);
}
