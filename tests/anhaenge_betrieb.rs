//! R00029 F0103 — Anhänge im Betrieb über das echte Programm: Großer Anspruch (B0525), Episoden (B0463),
//! Ereignisse (B0462), Warnschwelle (B0515), Start (B0516), keine ID und kein Name im Protokoll (B0464).

mod gemeinsam;

use gemeinsam::Absender;
use gemeinsam::anhaenge::{
    ablegen, anmelden, gesundheit, id, roh_antwort, roh_oeffnen, starten, verwerfen,
};
use gemeinsam::senden::{beitreten, ordner, roh_senden};
use std::path::Path;

fn zeilen_mit(ordner: &Path, text: &str) -> Vec<String> {
    gemeinsam::protokoll::alle_zeilen(ordner)
        .into_iter()
        .filter(|zeile| zeile.contains(text))
        .collect()
}

/// Großer Anspruch (M42′ Bedingung 2): `beta` meldet 3 MiB an (Q 10 MiB) → eine Episodenzeile mit
/// `kennung=beta@127.0.0.1` und `hochladen_bis`; zehnmal anmelden und verwerfen → eine Episode.
#[tokio::test]
async fn grosser_anspruch_als_episode_je_kennung() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, basis, "beta", Some("agent")).await;

    for _ in 0..10 {
        let angemeldet = id(&anmelden(&beta, basis, &token, "gross.bin", 3 * 1_048_576).await);
        assert_eq!(verwerfen(&beta, basis, &token, &angemeldet).await, 204);
    }
    id(&anmelden(&beta, basis, &token, "klein.bin", 1).await);
    drop(instanz);

    let beginn = zeilen_mit(
        wurzel.path(),
        "INFORMATION anhang Großer Anspruch — Episode beginnt",
    );
    assert_eq!(beginn.len(), 1, "{beginn:#?}");
    assert!(
        beginn[0].contains("kennung=beta@127.0.0.1") && beginn[0].contains("hochladen_bis="),
        "{}",
        beginn[0]
    );
    assert!(beginn[0].contains("bytes=3145728"), "{}", beginn[0]);
}

fn put_kopf(id: &str, token: &str, laenge: u64) -> String {
    format!(
        "PUT /api/anhaenge/{id} HTTP/1.1\r\nHost: 127.0.0.1\r\nX-LocalchatA-Sitzung: {token}\r\nContent-Type: application/octet-stream\r\nContent-Length: {laenge}"
    )
}

/// Episoden (B0463): drei 408 aus derselben Herkunft → eine Episode `Hochladen zu langsam abgebrochen`,
/// `abgewiesen.zu_langsam` + 3; neun gebunden, drei 507 `voll` → eine Episode mit `kennung`, `bytes`, `belegt`,
/// `hoechstens`; drei 507 `hochladungen` → eine Episode; Health `anhang_abgewiesen` + 6; keine Zeile unter `quote`.
#[tokio::test]
async fn episoden_der_abweisungen() {
    use tokio::io::AsyncWriteExt;
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let basis = instanz.adresse.clone();
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, &basis, "beta", Some("agent")).await;

    let mut langsam = Vec::new();
    let mut langsame_ids = Vec::new();
    for _ in 0..3 {
        let angemeldet = id(&anmelden(&beta, &basis, &token, "l.bin", 100).await);
        let mut verbindung = roh_oeffnen(
            instanz.port(),
            "127.0.0.1",
            &put_kopf(&angemeldet, &token, 100),
        )
        .await;
        verbindung.write_all(&[0u8; 10]).await.expect("Anfang");
        langsam.push(verbindung);
        langsame_ids.push(angemeldet);
    }
    for mut verbindung in langsam {
        let antwort = roh_antwort(&mut verbindung, std::time::Duration::from_secs(20)).await;
        assert!(antwort.starts_with("HTTP/1.1 408"), "{antwort}");
    }
    assert_eq!(gesundheit(&basis).await["abgewiesen"]["zu_langsam"], 3);
    for angemeldet in &langsame_ids {
        assert_eq!(verwerfen(&beta, &basis, &token, angemeldet).await, 204);
    }

    for nummer in 0..9 {
        let id = ablegen(
            &beta,
            &basis,
            &token,
            &format!("n{nummer}.bin"),
            vec![1u8; 1_048_576],
        )
        .await;
        let koerper = serde_json::json!({"text": "", "anhaenge": [id]})
            .to_string()
            .into_bytes();
        assert_eq!(
            roh_senden(
                &beta,
                &basis,
                Some(&token),
                "allgemein",
                "application/json",
                koerper
            )
            .await
            .status,
            201
        );
    }
    for _ in 0..3 {
        assert_eq!(
            anmelden(&beta, &basis, &token, "g.bin", 2 * 1_048_576)
                .await
                .json["grund"],
            "voll"
        );
    }
    let gamma = Absender::von("127.0.0.2");
    let token_gamma = beitreten(&gamma, &basis, "gamma", Some("agent")).await;
    id(&anmelden(&gamma, &basis, &token_gamma, "rest.bin", 1_048_576).await);
    for _ in 0..3 {
        assert_eq!(
            anmelden(&beta, &basis, &token, "h.bin", 1).await.json["grund"],
            "hochladungen"
        );
    }
    assert_eq!(gesundheit(&basis).await["anhang_abgewiesen"], 6);
    drop(instanz);

    let langsam = zeilen_mit(
        wurzel.path(),
        "WARNING anhang Hochladen zu langsam abgebrochen — Episode beginnt",
    );
    assert_eq!(langsam.len(), 1, "{langsam:#?}");
    assert!(langsam[0].contains("herkunft=127.0.0.1"), "{}", langsam[0]);
    let voll = zeilen_mit(
        wurzel.path(),
        "WARNING anhang Hochladen abgewiesen: Platz für Anhänge voll — Episode beginnt",
    );
    assert_eq!(voll.len(), 1, "{voll:#?}");
    assert!(
        voll[0].contains("kennung=beta@127.0.0.1 bytes=2097152 belegt=9437184 hoechstens=10485760"),
        "{}",
        voll[0]
    );
    let belegen = zeilen_mit(
        wurzel.path(),
        "WARNING anhang Hochladen abgewiesen: laufende Hochladungen belegen den Platz — Episode beginnt",
    );
    assert_eq!(belegen.len(), 1, "{belegen:#?}");
    assert!(
        belegen[0].contains("kennung=beta@127.0.0.1") && belegen[0].contains("reserviert=1048576"),
        "{}",
        belegen[0]
    );
    assert!(zeilen_mit(wurzel.path(), " quote ").is_empty());
}

/// 507 `platte` (mit `--anhangquote aus`, 1 PiB) → Episode je Herkunft `Platte fast voll` mit `herkunft`, `bytes`,
/// `frei`.
#[tokio::test]
async fn platte_fast_voll_als_episode() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &["--anhangquote", "aus"], &[]);
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, &instanz.adresse, "beta", Some("agent")).await;

    let antwort = anmelden(&beta, &instanz.adresse, &token, "riesig.bin", 1 << 50).await;
    drop(instanz);

    assert_eq!(
        (antwort.status, &antwort.json["regel"]),
        (507, &serde_json::json!("platte"))
    );
    let zeilen = zeilen_mit(
        wurzel.path(),
        "WARNING anhang Hochladen abgewiesen: Platte fast voll — Episode beginnt",
    );
    assert_eq!(zeilen.len(), 1, "{zeilen:#?}");
    assert!(
        zeilen[0].contains("herkunft=127.0.0.1 bytes=1125899906842624 frei="),
        "{}",
        zeilen[0]
    );
}

/// M22: `beta` meldet dreimal 1 Byte an und lädt nichts hoch → spätestens 80 s nach der letzten 201 eine Episode
/// `Reservierung verfallen ohne Hochladen` mit `kennung=beta@127.0.0.1` und `bytes=1`; je Ablauf eine `DEBUG`-Zeile.
#[tokio::test]
async fn reservierung_verfallen_ohne_hochladen() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &["--protokoll-stufe", "debug"], &[]);
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, &instanz.adresse, "beta", Some("agent")).await;
    for _ in 0..3 {
        id(&anmelden(&beta, &instanz.adresse, &token, "x.bin", 1).await);
    }
    let letzte = std::time::Instant::now();

    while gesundheit(&instanz.adresse).await["hochladungen"] != 0 {
        assert!(letzte.elapsed() < std::time::Duration::from_secs(80));
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    drop(instanz);

    let zeilen = zeilen_mit(
        wurzel.path(),
        "WARNING anhang Reservierung verfallen ohne Hochladen — Episode beginnt",
    );
    assert_eq!(zeilen.len(), 1, "{zeilen:#?}");
    assert!(
        zeilen[0].contains("kennung=beta@127.0.0.1") && zeilen[0].contains("bytes=1"),
        "{}",
        zeilen[0]
    );
    assert_eq!(
        zeilen_mit(wurzel.path(), "DEBUG anhang Anmeldung verfallen").len(),
        3
    );
}
