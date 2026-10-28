//! Aufbewahrungsfrist im Test (R00008, R00017): Instanz mit gestellter Uhr, Tagesdateien vor dem Start, Frist
//! setzen per `PUT`, Frist in der Liste lesen.

use super::{Absender, ApiAntwort, Instanz, api_antwort, starten_mit};
use serde_json::{Value, json};
use std::path::Path;

/// Heute ist der 28. Oktober 2026, 12:00 (R00008, User Stories).
pub const MITTAG: &str = "2026-10-28T12:00:00.000+01:00";
/// Zwei Sekunden vor Mitternacht (R00008 F0035, Z3).
pub const KURZ_VOR_MITTERNACHT: &str = "2026-10-28T23:59:58+01:00";

/// Startet auf Port 0 mit `TZ=Europe/Berlin` und `LOCALCHATA_JETZT=<jetzt>`; stderr wird mitgelesen.
pub fn starten_um(ordner: &Path, jetzt: &str) -> Instanz {
    starten_mit(
        ordner,
        &["--port", "0", "--host", "127.0.0.1", "--kein-browser"],
        &[("TZ", "Europe/Berlin"), ("LOCALCHATA_JETZT", jetzt)],
    )
}

/// `channels/<channel>/<tag>.log` mit `anzahl` Nachrichten (09:00 im Minutentakt, Versatz des Tages).
pub fn tag_anlegen(ordner: &Path, channel: &str, tag: &str, anzahl: usize) {
    let versatz = if tag < "2026-10-25" {
        "+02:00"
    } else {
        "+01:00"
    };
    let text: String = (0..anzahl)
        .map(|nummer| {
            format!("@ {tag}T09:{nummer:02}:00.000{versatz} alpha@127.0.0.2\n  n{nummer}\n\n")
        })
        .collect();
    datei_anlegen(
        ordner,
        &format!("channels/{channel}/{tag}.log"),
        text.as_bytes(),
    );
}

/// Legt eine Datei samt Elternordnern an.
pub fn datei_anlegen(ordner: &Path, pfad: &str, inhalt: &[u8]) {
    let ziel = ordner.join(pfad);
    std::fs::create_dir_all(ziel.parent().expect("Elternordner")).expect("Ordner");
    std::fs::write(ziel, inhalt).expect("Datei");
}

/// Ob `channels/<channel>/<tag>.log` da ist.
pub fn tag_da(ordner: &Path, channel: &str, tag: &str) -> bool {
    ordner
        .join(format!("channels/{channel}/{tag}.log"))
        .exists()
}

pub async fn beitreten(absender: &Absender, instanz: &Instanz, name: &str) -> String {
    let antwort = absender
        .beitreten(&instanz.adresse, json!({ "name": name }), None)
        .await;
    assert_eq!(antwort.status, 201, "{}", antwort.json);
    antwort.json["token"].as_str().expect("Token").to_string()
}

/// `PUT <basis>api/channels/<name>/frist` mit JSON-Körper und Sitzung.
pub async fn frist_setzen(
    absender: &Absender,
    instanz: &Instanz,
    token: &str,
    name: &str,
    koerper: Value,
) -> ApiAntwort {
    let anfrage = absender
        .anfrage(
            reqwest::Method::PUT,
            format!("{}api/channels/{name}/frist", instanz.adresse),
        )
        .header("X-LocalchatA-Sitzung", token)
        .json(&koerper);
    api_antwort(anfrage).await
}

/// Der Channel `name` aus `GET /api/channels`, mit `stand` der Liste.
pub async fn channel_in_liste(
    absender: &Absender,
    instanz: &Instanz,
    token: &str,
    name: &str,
) -> (Value, u64) {
    let liste = absender.channels(&instanz.adresse, Some(token)).await;
    assert_eq!(liste.status, 200, "{}", liste.json);
    let channel = liste.json["liste"]
        .as_array()
        .expect("Liste")
        .iter()
        .find(|channel| channel["name"] == name)
        .cloned()
        .unwrap_or_else(|| panic!("{name} nicht in {}", liste.json));
    (channel, liste.json["stand"].as_u64().expect("stand"))
}

pub async fn nachrichten(instanz: &Instanz) -> u64 {
    let (status, json) = super::gesundheit(&instanz.adresse).await;
    assert_eq!(status, 200);
    json["nachrichten"].as_u64().expect("nachrichten")
}
