//! Anhänge im Test (R00029): Instanz mit `channels/allgemein/` und `channels/technik/`, `--plattenquote 10MiB
//! --anhangquote 10MiB`, Anmelden, Hochladen und Senden mit Anhängen von einer festen Absenderadresse.

use super::{Absender, ApiAntwort, Instanz, api_antwort, starten_mit};
use serde_json::{Value, json};
use std::path::Path;

pub const MIB: u64 = 1_048_576;
/// Anhang-Quote der gemeinsamen Gegebenheiten: 10 MiB.
pub const Q: u64 = 10 * MIB;

const LOKAL: [&str; 5] = ["--port", "0", "--host", "127.0.0.1", "--kein-browser"];

/// Startet mit `TZ=Europe/Berlin`, `--plattenquote 10MiB --anhangquote 10MiB` und weiteren Optionen (die
/// Anhang-Quote einer weiteren Option gewinnt nicht — `weitere` hängt hinten an, die letzte Angabe zählt).
pub fn starten(ordner: &Path, weitere: &[&str], umgebung: &[(&str, &str)]) -> Instanz {
    let mut argumente = LOKAL.to_vec();
    argumente.extend_from_slice(&["--plattenquote", "10MiB"]);
    if !weitere.contains(&"--anhangquote") {
        argumente.extend_from_slice(&["--anhangquote", "10MiB"]);
    }
    argumente.extend_from_slice(weitere);
    let mut alle = vec![("TZ", "Europe/Berlin")];
    alle.extend_from_slice(umgebung);
    starten_mit(ordner, &argumente, &alle)
}

/// `POST <basis>api/anhaenge` mit `{"name": name, "bytes": bytes}`.
pub async fn anmelden(
    absender: &Absender,
    basis: &str,
    token: &str,
    name: &str,
    bytes: u64,
) -> ApiAntwort {
    anmelden_roh(
        absender,
        basis,
        Some(token),
        json!({ "name": name, "bytes": bytes }).to_string(),
    )
    .await
}

/// `POST <basis>api/anhaenge` mit beliebigem JSON-Text.
pub async fn anmelden_roh(
    absender: &Absender,
    basis: &str,
    token: Option<&str>,
    koerper: String,
) -> ApiAntwort {
    let mut anfrage = absender
        .anfrage(reqwest::Method::POST, format!("{basis}api/anhaenge"))
        .header("Content-Type", "application/json")
        .body(koerper);
    if let Some(token) = token {
        anfrage = anfrage.header("X-LocalchatA-Sitzung", token);
    }
    api_antwort(anfrage).await
}

/// Die ID aus einer 201 der Anmeldung.
pub fn id(antwort: &ApiAntwort) -> String {
    assert_eq!(antwort.status, 201, "{}", antwort.json);
    antwort.json["anhang"]["id"]
        .as_str()
        .expect("id")
        .to_string()
}

/// `GET <basis>api/gesundheit` als JSON.
pub async fn gesundheit(basis: &str) -> Value {
    let (status, json) = super::gesundheit(basis).await;
    assert_eq!(status, 200);
    json
}
