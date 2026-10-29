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

/// `PUT <basis>api/anhaenge/<id>` mit allen Bytes als `application/octet-stream`.
pub async fn hochladen(
    absender: &Absender,
    basis: &str,
    token: &str,
    id: &str,
    bytes: Vec<u8>,
) -> ApiAntwort {
    api_antwort(
        absender
            .anfrage(reqwest::Method::PUT, format!("{basis}api/anhaenge/{id}"))
            .header("Content-Type", "application/octet-stream")
            .header("X-LocalchatA-Sitzung", token)
            .body(bytes),
    )
    .await
}

/// Anmelden und hochladen in einem; liefert die ID.
pub async fn ablegen(
    absender: &Absender,
    basis: &str,
    token: &str,
    name: &str,
    bytes: Vec<u8>,
) -> String {
    let id = id(&anmelden(absender, basis, token, name, bytes.len() as u64).await);
    let antwort = hochladen(absender, basis, token, &id, bytes).await;
    assert_eq!(antwort.status, 200, "{}", antwort.json);
    id
}

/// Eine Rohverbindung von `herkunft` zur Instanz; `kopf` sind die Kopfzeilen ohne Leerzeile.
pub async fn roh_oeffnen(port: u16, herkunft: &str, kopf: &str) -> tokio::net::TcpStream {
    use tokio::io::AsyncWriteExt;
    let mut verbindung = super::herkunft::verbinden(port, herkunft).await;
    verbindung
        .write_all(format!("{kopf}\r\n\r\n").as_bytes())
        .await
        .expect("Kopf gesendet");
    verbindung
}

/// Liest die Antwort einer Rohverbindung bis zum Ende oder höchstens `hoechstens`; Statuszeile und Rest.
pub async fn roh_antwort(
    verbindung: &mut tokio::net::TcpStream,
    hoechstens: std::time::Duration,
) -> String {
    use tokio::io::AsyncReadExt;
    let mut roh = Vec::new();
    let _ = tokio::time::timeout(hoechstens, verbindung.read_to_end(&mut roh)).await;
    String::from_utf8_lossy(&roh).into_owned()
}

/// `PUT /api/anhaenge/<id>` roh von 127.0.0.1 mit `Content-Length: laenge`, für Abweisungen vor dem ersten
/// Körperbyte (404, 409, 411, 415, 422, 429): die Antwort wird bis zum Schließen gelesen, danach — frühestens
/// `koerper_nach` nach dem Kopf — 10 Bytes Körper geschrieben, Schreibfehler zählen nicht. Die Instanz antwortet vor
/// dem Körper und schließt; ein Client, der den Körper vor dem Lesen schreibt, kann `EPIPE`/`ECONNRESET` sehen und
/// die Antwort verlieren (Befund #55 B1) — darum erst lesen. Liefert Status und JSON des Körpers.
pub async fn hochladen_abgewiesen(
    instanz: &Instanz,
    token: &str,
    id: &str,
    laenge: u64,
    koerper_nach: Option<std::time::Duration>,
) -> ApiAntwort {
    use tokio::io::AsyncWriteExt;
    let kopf = format!(
        "PUT /api/anhaenge/{id} HTTP/1.1\r\nHost: 127.0.0.1\r\nX-LocalchatA-Sitzung: {token}\r\nContent-Type: application/octet-stream\r\nContent-Length: {laenge}"
    );
    let beginn = tokio::time::Instant::now();
    let mut verbindung = roh_oeffnen(instanz.port(), "127.0.0.1", &kopf).await;
    let antwort = roh_antwort(&mut verbindung, std::time::Duration::from_secs(5)).await;
    if let Some(warten) = koerper_nach {
        tokio::time::sleep_until(beginn + warten).await;
        let _ = verbindung.write_all(&[7u8; 10]).await;
    }
    let status = antwort
        .strip_prefix("HTTP/1.1 ")
        .and_then(|rest| rest.get(..3))
        .and_then(|zahl| zahl.parse().ok())
        .unwrap_or_else(|| panic!("keine Antwort vor dem Körper: {antwort:?}"));
    let koerper = antwort
        .split_once("\r\n\r\n")
        .map_or("", |(_, koerper)| koerper);
    ApiAntwort {
        status,
        kopf: reqwest::header::HeaderMap::new(),
        json: serde_json::from_str(koerper).unwrap_or(Value::Null),
    }
}

/// Dateien im Eingang des Ordners (Namen).
pub fn im_eingang(ordner: &Path) -> Vec<String> {
    let mut namen: Vec<String> = std::fs::read_dir(ordner.join(".eingang"))
        .map(|eintraege| {
            eintraege
                .map(|eintrag| {
                    eintrag
                        .expect("Eintrag")
                        .file_name()
                        .to_string_lossy()
                        .into_owned()
                })
                .collect()
        })
        .unwrap_or_default();
    namen.sort();
    namen
}

/// `DELETE <basis>api/anhaenge/<id>`; liefert den Status.
pub async fn verwerfen(absender: &Absender, basis: &str, token: &str, id: &str) -> u16 {
    api_antwort(
        absender
            .anfrage(reqwest::Method::DELETE, format!("{basis}api/anhaenge/{id}"))
            .header("X-LocalchatA-Sitzung", token),
    )
    .await
    .status
}
