//! Nachricht senden im Test (R00011): Instanz mit `channels/allgemein/` und `channels/technik/`, Uhr ab
//! 2026-10-28 12:00, Beitritt und `POST …/nachrichten` von einer festen Absenderadresse.

use super::{Absender, ApiAntwort, Instanz, api_antwort, starten_mit};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// Startzeit der Instanz; ihr Tag ist die Tagesdatei der Nachrichten.
pub const JETZT: &str = "2026-10-28T12:00:00.000+01:00";
pub const HEUTE: &str = "2026-10-28";

/// Wegwerf-Ordner mit den leeren Channels `allgemein` und `technik`.
pub fn ordner() -> tempfile::TempDir {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    for channel in ["allgemein", "technik"] {
        std::fs::create_dir_all(wurzel.path().join(format!("channels/{channel}")))
            .expect("Channel-Ordner");
    }
    wurzel
}

/// Startet auf Port 0 mit `TZ=Europe/Berlin`, `LOCALCHATA_JETZT=<JETZT>` und weiteren Variablen; stderr mitgelesen.
pub fn starten(ordner: &Path, umgebung: &[(&str, &str)]) -> Instanz {
    let mut alle = vec![("TZ", "Europe/Berlin"), ("LOCALCHATA_JETZT", JETZT)];
    alle.extend_from_slice(umgebung);
    starten_mit(
        ordner,
        &["--port", "0", "--host", "127.0.0.1", "--kein-browser"],
        &alle,
    )
}

/// Tritt bei (`art` optional) und liefert das Token.
pub async fn beitreten(absender: &Absender, basis: &str, name: &str, art: Option<&str>) -> String {
    let mut koerper = json!({ "name": name });
    if let Some(art) = art {
        koerper["art"] = json!(art);
    }
    let antwort = absender.beitreten(basis, koerper, None).await;
    assert_eq!(antwort.status, 201, "{}", antwort.json);
    antwort.json["token"].as_str().expect("Token").to_string()
}

/// `POST <basis>api/channels/<channel>/nachrichten` mit `{"text": text}`.
pub async fn senden(
    absender: &Absender,
    basis: &str,
    token: &str,
    channel: &str,
    text: &str,
) -> ApiAntwort {
    roh_senden(
        absender,
        basis,
        Some(token),
        channel,
        "application/json",
        json!({ "text": text }).to_string().into_bytes(),
    )
    .await
}

/// Wie `senden` mit beliebigem Körper, Typ und optionalem Token.
pub async fn roh_senden(
    absender: &Absender,
    basis: &str,
    token: Option<&str>,
    channel: &str,
    typ: &str,
    koerper: Vec<u8>,
) -> ApiAntwort {
    let mut anfrage = absender
        .anfrage(
            reqwest::Method::POST,
            format!("{basis}api/channels/{channel}/nachrichten"),
        )
        .header("Content-Type", typ)
        .body(koerper);
    if let Some(token) = token {
        anfrage = anfrage.header("X-LocalchatA-Sitzung", token);
    }
    api_antwort(anfrage).await
}

/// Der Verlauf (neueste 100) eines Channels.
pub async fn verlauf(absender: &Absender, basis: &str, token: &str, channel: &str) -> Value {
    let antwort = api_antwort(
        absender
            .anfrage(
                reqwest::Method::GET,
                format!("{basis}api/channels/{channel}/nachrichten"),
            )
            .header("X-LocalchatA-Sitzung", token),
    )
    .await;
    assert_eq!(antwort.status, 200, "{}", antwort.json);
    antwort.json
}

pub fn tagesdatei(ordner: &Path, channel: &str) -> PathBuf {
    ordner.join(format!("channels/{channel}/{HEUTE}.log"))
}

/// Inhalt der heutigen Tagesdatei, leer, wenn es sie nicht gibt.
pub fn tagesinhalt(ordner: &Path, channel: &str) -> Vec<u8> {
    std::fs::read(tagesdatei(ordner, channel)).unwrap_or_default()
}

/// `json` mit Leerzeichen auf genau `laenge` Bytes aufgefüllt.
pub fn aufgefuellt(json: &str, laenge: usize) -> Vec<u8> {
    format!("{json}{}", " ".repeat(laenge - json.len())).into_bytes()
}
