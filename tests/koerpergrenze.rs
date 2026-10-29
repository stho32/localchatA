//! R00010 Körpergrenze je Route (B0204, Befundkarte #21) — über das echte Programm. `POST /api/beitreten` und
//! `POST /api/channels` nehmen Körper bis einschließlich 4 096 Bytes an; ab 4 097 Bytes antworten sie 413
//! `{fehler}` mit „zu groß“ und der Grenze, ohne zu parsen (ungültiges JSON → 413, nicht 400). Die Spanne
//! steht je Route einzeln, damit eine Grenze, die nur an einer Route sitzt, auffällt. Den ganzen Satz der 413
//! samt Beispiel je Route (Beitreten, Channel anlegen, Frist setzen) prüfen die Tests `b1_…` (R00020 B0334,
//! Befund #31 B1).
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::frist::channel_in_liste;
use gemeinsam::{Absender, ApiAntwort, Instanz, api_antwort, wegwerf_starten};
use serde_json::{Value, json};
use std::fs;

const GRENZE: usize = 4096;

/// Ein gültiger Körper, mit Leerzeichen auf genau `laenge` Bytes aufgefüllt (`k4096.json`, `k4097.json`).
fn aufgefuellt(koerper: &Value, laenge: usize) -> String {
    let text = koerper.to_string();
    assert!(text.len() <= laenge, "{text} ist länger als {laenge}");
    format!("{text:<laenge$}")
}

/// `{` und lauter `x` — kein JSON (`kaputt4097.json`).
fn kaputt(laenge: usize) -> String {
    format!("{{{}", "x".repeat(laenge - 1))
}

async fn senden(
    absender: &Absender,
    instanz: &Instanz,
    pfad: &str,
    token: Option<&str>,
    content_type: Option<&str>,
    koerper: String,
) -> ApiAntwort {
    let mut anfrage = absender
        .anfrage(
            reqwest::Method::POST,
            format!("{}api/{pfad}", instanz.adresse),
        )
        .body(koerper);
    if let Some(content_type) = content_type {
        anfrage = anfrage.header("Content-Type", content_type);
    }
    if let Some(token) = token {
        anfrage = anfrage.header("X-LocalchatA-Sitzung", token);
    }
    api_antwort(anfrage).await
}

async fn beitreten_json(absender: &Absender, instanz: &Instanz, koerper: String) -> ApiAntwort {
    senden(
        absender,
        instanz,
        "beitreten",
        None,
        Some("application/json"),
        koerper,
    )
    .await
}

async fn token(absender: &Absender, instanz: &Instanz, name: &str) -> String {
    let antwort = absender
        .beitreten(&instanz.adresse, json!({ "name": name }), None)
        .await;
    assert_eq!(antwort.status, 201, "{}", antwort.json);
    antwort.json["token"].as_str().expect("Token").to_string()
}

fn zu_gross_mit_grenze(antwort: &ApiAntwort, fall: &str) {
    assert_eq!(antwort.status, 413, "{fall}: {}", antwort.json);
    let fehler = antwort.json["fehler"].as_str().unwrap_or_default();
    assert!(fehler.contains("zu groß"), "{fall}: {fehler:?}");
    assert!(fehler.contains("4096 Bytes"), "{fall}: {fehler:?}");
}

/// Die 413 einer Route mit genau ihrem Satz (Befund #31 B1): der Satzschluss mit dem Beispiel schreibt jede Route
/// selbst, deshalb steht der Wortlaut je Route vollständig im Test. Die Verbindung endet danach (`Connection: close`).
fn zu_gross_genau(antwort: &ApiAntwort, erwartet: &str) {
    assert_eq!(antwort.status, 413, "{}", antwort.json);
    assert_eq!(antwort.json["fehler"], erwartet);
    assert_eq!(
        antwort.kopf.get("connection").map(|wert| wert.as_bytes()),
        Some(&b"close"[..]),
        "{erwartet}"
    );
}

fn channel_ordner(instanz: &Instanz) -> Vec<String> {
    let Ok(eintraege) = fs::read_dir(instanz.ordner.join("channels")) else {
        return Vec::new();
    };
    let mut namen: Vec<String> = eintraege
        .map(|eintrag| {
            eintrag
                .expect("Eintrag")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    namen.sort();
    namen
}

#[tokio::test]
async fn us1_beitreten_mit_4097_bytes_ungueltigem_json_ergibt_413_statt_400() {
    let instanz = wegwerf_starten();
    let absender = Absender::von("127.0.0.1");

    let antwort = beitreten_json(&absender, &instanz, kaputt(GRENZE + 1)).await;

    zu_gross_mit_grenze(&antwort, "kaputt 4097");
}

#[tokio::test]
async fn us2_beitreten_nimmt_4096_bytes_an_und_weist_4097_ab() {
    let instanz = wegwerf_starten();
    let absender = Absender::von("127.0.0.1");
    let koerper = json!({ "name": "spanne" });

    let genau = beitreten_json(&absender, &instanz, aufgefuellt(&koerper, GRENZE)).await;
    let drueber = beitreten_json(&absender, &instanz, aufgefuellt(&koerper, GRENZE + 1)).await;
    let ohne_typ = senden(
        &absender,
        &instanz,
        "beitreten",
        None,
        None,
        aufgefuellt(&koerper, GRENZE + 1),
    )
    .await;

    assert_eq!(genau.status, 201, "{}", genau.json);
    assert_eq!(genau.json["kennung"], "spanne@127.0.0.1");
    zu_gross_mit_grenze(&drueber, "Beitritt 4097");
    zu_gross_mit_grenze(&ohne_typ, "Beitritt 4097 ohne Content-Type");
}

#[tokio::test]
async fn us3_channel_anlegen_nimmt_4096_bytes_an_und_weist_4097_ab_ohne_zweiten_ordner() {
    let instanz = wegwerf_starten();
    let absender = Absender::von("127.0.0.2");
    let alpha = token(&absender, &instanz, "alpha").await;
    let koerper = json!({ "name": "security-entwicklung" });
    let anlegen = |koerper: String, content_type: Option<&'static str>| {
        senden(
            &absender,
            &instanz,
            "channels",
            Some(&alpha),
            content_type,
            koerper,
        )
    };

    let genau = anlegen(aufgefuellt(&koerper, GRENZE), Some("application/json")).await;
    let drueber = anlegen(aufgefuellt(&koerper, GRENZE + 1), Some("application/json")).await;
    let kaputt_drueber = anlegen(kaputt(GRENZE + 1), Some("application/json")).await;
    let ohne_typ = anlegen(aufgefuellt(&koerper, GRENZE + 1), None).await;

    assert_eq!(genau.status, 201, "{}", genau.json);
    assert_eq!(genau.json["channel"]["name"], "security-entwicklung");
    zu_gross_mit_grenze(&drueber, "Anlage 4097");
    zu_gross_mit_grenze(&kaputt_drueber, "Anlage kaputt 4097");
    zu_gross_mit_grenze(&ohne_typ, "Anlage 4097 ohne Content-Type");
    assert_eq!(channel_ordner(&instanz), vec!["security-entwicklung"]);
}

#[tokio::test]
async fn us4_gewoehnliche_beitritte_und_anlagen_bleiben_201() {
    let instanz = wegwerf_starten();
    let beta = Absender::von("127.0.0.3");
    let zwei = Absender::von("127.0.0.2");

    let beitritt = beta
        .beitreten(&instanz.adresse, json!({ "name": "beta" }), None)
        .await;
    let token_beta = beitritt.json["token"].as_str().unwrap_or_default();
    let allgemein = beta
        .channel_anlegen(&instanz.adresse, token_beta, json!({ "name": "allgemein" }))
        .await;
    let spanne = zwei
        .beitreten(&instanz.adresse, json!({ "name": "spanne" }), None)
        .await;
    let token_spanne = spanne.json["token"].as_str().unwrap_or_default();
    let security = zwei
        .channel_anlegen(
            &instanz.adresse,
            token_spanne,
            json!({ "name": "security-entwicklung" }),
        )
        .await;

    assert_eq!(beitritt.status, 201, "{}", beitritt.json);
    assert_eq!(allgemein.status, 201, "{}", allgemein.json);
    assert_eq!(spanne.status, 201, "{}", spanne.json);
    assert_eq!(security.status, 201, "{}", security.json);
}

/// Herleitung der Grenze (R00010): Name `a` × 32 als `a`, Beschreibung `😀` × 200 als Ersatzpaar —
/// 2 621 Bytes, der größte gültige Körper einer Anlage. Er ist nie 413.
#[tokio::test]
async fn kontrolle_der_laengste_gueltige_channel_koerper_wird_angelegt() {
    let instanz = wegwerf_starten();
    let absender = Absender::von("127.0.0.2");
    let alpha = token(&absender, &instanz, "alpha").await;
    let koerper = format!(
        "{{\"name\":\"{}\",\"beschreibung\":\"{}\"}}",
        "\\u0061".repeat(32),
        "\\ud83d\\ude00".repeat(200)
    );
    assert_eq!(koerper.len(), 2621);

    let antwort = senden(
        &absender,
        &instanz,
        "channels",
        Some(&alpha),
        Some("application/json"),
        koerper,
    )
    .await;

    assert_eq!(antwort.status, 201, "{}", antwort.json);
    assert_eq!(antwort.json["channel"]["name"], "a".repeat(32));
    assert_eq!(antwort.json["channel"]["beschreibung"], "😀".repeat(200));
}

/// Befund #31 B1, Abnahme-Probe „Channel anlegen“: ein gültiger Körper mit 4 097 Bytes → 413 mit dem ganzen Satz
/// samt Beispiel, kein Channel-Ordner entsteht.
#[tokio::test]
async fn b1_channel_anlegen_413_nennt_den_ganzen_satz_mit_beispiel_und_legt_nichts_an() {
    let instanz = wegwerf_starten();
    let absender = Absender::von("127.0.0.2");
    let alpha = token(&absender, &instanz, "alpha").await;
    let koerper = aufgefuellt(&json!({ "name": "allgemein" }), GRENZE + 1);
    assert_eq!(koerper.len(), 4097);

    let antwort = senden(
        &absender,
        &instanz,
        "channels",
        Some(&alpha),
        Some("application/json"),
        koerper,
    )
    .await;

    zu_gross_genau(
        &antwort,
        "Der Körper ist zu groß — höchstens 4096 Bytes, nur Name und Beschreibung schicken, zum Beispiel {\"name\":\"allgemein\"}.",
    );
    assert_eq!(channel_ordner(&instanz), Vec::<String>::new());
}

/// Befund #31 B1, Abnahme-Probe „Frist setzen“: ein gültiger Körper mit 1 025 Bytes → 413 mit dem ganzen Satz
/// samt Beispiel; die Frist bleibt in der Liste und in `einstellungen.toml`, wie sie war.
#[tokio::test]
async fn b1_frist_setzen_413_nennt_den_ganzen_satz_mit_beispiel_und_laesst_die_frist() {
    let instanz = wegwerf_starten();
    let absender = Absender::von("127.0.0.2");
    let alpha = token(&absender, &instanz, "alpha").await;
    let angelegt = absender
        .channel_anlegen(&instanz.adresse, &alpha, json!({ "name": "allgemein" }))
        .await;
    assert_eq!(angelegt.status, 201, "{}", angelegt.json);
    let einstellungen = instanz.ordner.join("channels/allgemein/einstellungen.toml");
    let datei_vorher = fs::read(&einstellungen).ok();
    let (vorher, _) = channel_in_liste(&absender, &instanz, &alpha, "allgemein").await;
    let koerper = aufgefuellt(&json!({ "tage": 30 }), 1025);
    assert_eq!(koerper.len(), 1025);

    let antwort = api_antwort(
        absender
            .anfrage(
                reqwest::Method::PUT,
                format!("{}api/channels/allgemein/frist", instanz.adresse),
            )
            .header("X-LocalchatA-Sitzung", &alpha)
            .header("Content-Type", "application/json")
            .body(koerper),
    )
    .await;

    zu_gross_genau(
        &antwort,
        "Der Körper ist zu groß — höchstens 1024 Bytes, nur die Tage schicken, zum Beispiel {\"tage\":30}.",
    );
    let (danach, _) = channel_in_liste(&absender, &instanz, &alpha, "allgemein").await;
    assert_eq!(danach["frist_tage"], vorher["frist_tage"]);
    assert_eq!(danach["frist_tage"], 7);
    assert_eq!(fs::read(&einstellungen).ok(), datei_vorher);
}

/// Befund #31 B1, Variante „Beitreten“: ein gültiger Körper mit 4 097 Bytes → 413 mit dem ganzen Satz samt Beispiel.
#[tokio::test]
async fn b1_beitreten_413_nennt_den_ganzen_satz_mit_beispiel() {
    let instanz = wegwerf_starten();
    let absender = Absender::von("127.0.0.1");
    let koerper = aufgefuellt(&json!({ "name": "shoff" }), GRENZE + 1);
    assert_eq!(koerper.len(), 4097);

    let antwort = beitreten_json(&absender, &instanz, koerper).await;

    zu_gross_genau(
        &antwort,
        "Der Körper ist zu groß — höchstens 4096 Bytes, nur Name und Art schicken, zum Beispiel {\"name\":\"shoff\"}.",
    );
}
