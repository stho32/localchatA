//! R00029 B0512 — Angriffsmatrix Löschen über das Programm (Linie L2): echtes Programm, `--plattenquote 10MiB
//! --anhangquote 10MiB` (M51: `--anhangquote 64MiB`), Rohanfragen; Kontrollpaar K1. M40′, M44′, M45′, M51, M52
//! (M53 und der Neustart stehen in `anhaenge_loeschen.rs`).

mod gemeinsam;

use gemeinsam::anhaenge::{
    MIB, ablegen, anmelden, gesundheit, hochladen, id, roh_antwort, roh_oeffnen, starten,
};
use gemeinsam::dateien::png;
use gemeinsam::senden::{beitreten, ordner, roh_senden, senden};
use gemeinsam::{Absender, ApiAntwort, api_antwort};
use serde_json::{Value, json};
use std::path::Path;
use std::time::Duration;

async fn senden_mit(
    absender: &Absender,
    basis: &str,
    token: &str,
    channel: &str,
    ids: &[String],
) -> ApiAntwort {
    let koerper = json!({"text": "", "anhaenge": ids})
        .to_string()
        .into_bytes();
    roh_senden(
        absender,
        basis,
        Some(token),
        channel,
        "application/json",
        koerper,
    )
    .await
}

async fn loeschen(absender: &Absender, basis: &str, token: &str, koerper: Value) -> ApiAntwort {
    api_antwort(
        absender
            .anfrage(
                reqwest::Method::POST,
                format!("{basis}api/anhaenge/loeschen"),
            )
            .header("Content-Type", "application/json")
            .header("X-LocalchatA-Sitzung", token)
            .body(koerper.to_string()),
    )
    .await
}

async fn abrufen(basis: &str, id: &str) -> (u16, Vec<u8>) {
    let antwort = reqwest::get(format!("{basis}anhaenge/{id}"))
        .await
        .expect("Antwort");
    (
        antwort.status().as_u16(),
        antwort.bytes().await.expect("Körper").to_vec(),
    )
}

/// K1: Anmelden 300 KiB PNG → 201, `PUT` → 200.
async fn k1(basis: &str, absender: &Absender, token: &str) {
    let mut bild = png(64, 64);
    bild.resize(307_200, 0);
    let angemeldet = id(&anmelden(absender, basis, token, "k1.png", bild.len() as u64).await);
    assert_eq!(
        hochladen(absender, basis, token, &angemeldet, bild)
            .await
            .status,
        200,
        "K1"
    );
}

/// Neun Anhänge à 1 MiB von `beta` in `allgemein`.
async fn neun(basis: &str, beta: &Absender, token: &str) -> Vec<String> {
    let mut ids = Vec::new();
    for nummer in 0..9 {
        let id = ablegen(
            beta,
            basis,
            token,
            &format!("n{nummer}.bin"),
            vec![nummer as u8; MIB as usize],
        )
        .await;
        assert_eq!(
            senden_mit(beta, basis, token, "allgemein", std::slice::from_ref(&id))
                .await
                .status,
            201
        );
        ids.push(id);
    }
    ids
}

/// M40′: Quote füllen — neun gebunden, Anmelden 2 MiB → 507 `voll` ohne `Retry-After`, Text 201, kein gebundener
/// Anhang verändert; nach bestätigtem Löschen von zweien nimmt die Anmeldung an. M44′: ausgedachte ID und ID des
/// Eingangs → `unbekannt` mit gleich langer Antwort; die 410 nennt kein Gespräch, keinen Tag, keinen Namen.
#[tokio::test]
async fn quote_fuellen_und_kein_orakel() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, basis, "beta", Some("agent")).await;
    let ids = neun(basis, &beta, &token).await;

    let voll = anmelden(&beta, basis, &token, "gross.bin", 2 * MIB).await;
    assert_eq!(
        (voll.status, &voll.json["grund"]),
        (507, &json!("voll")),
        "{}",
        voll.json
    );
    assert!(voll.kopf.get("retry-after").is_none());
    assert_eq!(
        senden(&beta, basis, &token, "allgemein", "Text geht")
            .await
            .status,
        201
    );
    for (nummer, id) in ids.iter().enumerate() {
        assert_eq!(
            abrufen(basis, id).await,
            (200, vec![nummer as u8; MIB as usize])
        );
    }

    let ausgedacht = "c".repeat(26);
    let angemeldet = id(&anmelden(&beta, basis, &token, "klein.bin", 10).await);
    let erfunden = loeschen(
        &beta,
        basis,
        &token,
        json!({"ids": [ausgedacht], "bestaetigt": true}),
    )
    .await;
    let eingang = loeschen(
        &beta,
        basis,
        &token,
        json!({"ids": [angemeldet], "bestaetigt": true}),
    )
    .await;
    assert_eq!(
        erfunden.json.to_string().len(),
        eingang.json.to_string().len()
    );
    assert_eq!(
        (
            &erfunden.json["ergebnisse"][0]["ergebnis"],
            &eingang.json["ergebnisse"][0]["ergebnis"]
        ),
        (&json!("unbekannt"), &json!("unbekannt"))
    );

    let geloescht = loeschen(
        &beta,
        basis,
        &token,
        json!({"ids": [ids[0], ids[1]], "bestaetigt": true}),
    )
    .await;
    assert_eq!(geloescht.status, 200);
    assert_eq!(
        anmelden(&beta, basis, &token, "gross.bin", 2 * MIB)
            .await
            .status,
        201
    );
    let (status, koerper) = abrufen(basis, &ids[0]).await;
    let text = String::from_utf8(koerper).expect("Text");
    assert_eq!(status, 410);
    assert!(
        !text.contains("allgemein") && !text.contains("n0.bin") && !text.contains("2026"),
        "{text}"
    );
    k1(basis, &beta, &token).await;
}

/// M45′: der Tag-Ordner bzw. `anhaenge/` als Symlink auf einen fremden Ordner mit gleichnamiger Datei gleicher
/// Länge → Löschen `fehler`, fremde Datei byte-gleich; `GET` liefert nie die fremden Bytes.
#[tokio::test]
async fn kuerzen_folgt_keinem_symlink_im_pfad() {
    for ebene in ["tag", "anhaenge"] {
        let wurzel = ordner();
        let instanz = starten(wurzel.path(), &[], &[]);
        let basis = instanz.adresse.as_str();
        let beta = Absender::von("127.0.0.1");
        let token = beitreten(&beta, basis, "beta", Some("agent")).await;
        let a = ablegen(&beta, basis, &token, "a.bin", vec![1u8; 1000]).await;
        assert_eq!(
            senden_mit(&beta, basis, &token, "allgemein", std::slice::from_ref(&a))
                .await
                .status,
            201
        );
        let anhaenge = wurzel.path().join("channels/allgemein/anhaenge");
        let tag = std::fs::read_dir(&anhaenge)
            .expect("anhaenge")
            .next()
            .expect("Tag")
            .expect("Tag")
            .path();
        let fremd = tempfile::tempdir().expect("fremd");
        let (ersetzt, fremder_tag) = match ebene {
            "tag" => (tag.clone(), fremd.path().to_path_buf()),
            _ => (
                anhaenge.clone(),
                fremd.path().join(tag.file_name().expect("Name")),
            ),
        };
        std::fs::create_dir_all(&fremder_tag).expect("fremder Tag");
        for eintrag in std::fs::read_dir(&tag).expect("Tag") {
            let eintrag = eintrag.expect("Eintrag");
            std::fs::write(fremder_tag.join(eintrag.file_name()), vec![9u8; 1000])
                .expect("fremde Datei");
        }
        std::fs::rename(&ersetzt, wurzel.path().join("beiseite")).expect("beiseite");
        std::os::unix::fs::symlink(fremd.path(), &ersetzt).expect("Symlink");

        let antwort = loeschen(
            &beta,
            basis,
            &token,
            json!({"ids": [a], "bestaetigt": true}),
        )
        .await;

        assert_eq!(
            antwort.json["ergebnisse"][0]["ergebnis"], "fehler",
            "{ebene}: {}",
            antwort.json
        );
        for eintrag in std::fs::read_dir(&fremder_tag).expect("fremd") {
            assert_eq!(
                std::fs::read(eintrag.expect("Eintrag").path()).expect("fremd"),
                vec![9u8; 1000],
                "{ebene}"
            );
        }
        let (status, koerper) = abrufen(basis, &a).await;
        assert_eq!(status, 404, "{ebene}");
        assert!(!koerper.contains(&9u8), "{ebene}");
    }
}

fn zeilen_mit(ordner: &Path, text: &str) -> Vec<String> {
    gemeinsam::protokoll::alle_zeilen(ordner)
        .into_iter()
        .filter(|zeile| zeile.contains(text))
        .collect()
}

/// M51: eigene Instanz `--anhangquote 64MiB`, 501 Anhänge, gelöscht in Anfragen zu 500 und 1 mit Bestätigung →
/// beide 200, zwei `INFORMATION`-Zeilen, Health `anhaenge_geloescht` 501, keine ID im Protokoll; 1.000 bestätigte
/// Anfragen nur mit ausgedachten IDs → keine weitere `INFORMATION`, Health unverändert.
#[tokio::test]
async fn loeschflut() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &["--anhangquote", "64MiB"], &[]);
    let basis = instanz.adresse.clone();
    let mut ids: Vec<String> = Vec::new();
    let mut runde = 0;
    while ids.len() < 501 {
        for herkunft in 2..10 {
            let absender = Absender::von(&format!("127.0.0.{herkunft}"));
            for kennung in 0..4 {
                if ids.len() >= 501 {
                    break;
                }
                let token = beitreten(
                    &absender,
                    &basis,
                    &format!("k{runde}-{herkunft}-{kennung}"),
                    Some("agent"),
                )
                .await;
                let mut eigene = Vec::new();
                for _ in 0..4 {
                    if ids.len() + eigene.len() >= 501 {
                        break;
                    }
                    eigene.push(ablegen(&absender, &basis, &token, "x.bin", vec![1]).await);
                }
                let gesendet = senden_mit(&absender, &basis, &token, "allgemein", &eigene).await;
                assert_eq!(gesendet.status, 201, "{}", gesendet.json);
                ids.extend(eigene);
            }
        }
        runde += 1;
    }
    let loeschende = Absender::von("127.0.0.1");
    let token = beitreten(&loeschende, &basis, "putz", Some("mensch")).await;

    let erste = loeschen(
        &loeschende,
        &basis,
        &token,
        json!({"ids": &ids[..500], "bestaetigt": true}),
    )
    .await;
    let zweite = loeschen(
        &loeschende,
        &basis,
        &token,
        json!({"ids": &ids[500..], "bestaetigt": true}),
    )
    .await;

    assert_eq!(
        (erste.status, erste.json["geloescht"].as_u64()),
        (200, Some(500)),
        "{}",
        erste.json
    );
    assert_eq!(
        (zweite.status, zweite.json["geloescht"].as_u64()),
        (200, Some(1))
    );
    assert_eq!(gesundheit(&basis).await["anhaenge_geloescht"], 501);
    for nummer in 0..1_000u32 {
        let ausgedacht = format!(
            "{:a>26}",
            format!("{nummer}")
                .replace('0', "q")
                .replace('1', "r")
                .replace('8', "s")
                .replace('9', "t")
        );
        loeschen(
            &loeschende,
            &basis,
            &token,
            json!({"ids": [ausgedacht], "bestaetigt": true}),
        )
        .await;
    }
    assert_eq!(gesundheit(&basis).await["anhaenge_geloescht"], 501);
    drop(instanz);
    assert_eq!(
        zeilen_mit(wurzel.path(), "INFORMATION anhang Anhänge gelöscht").len(),
        2
    );
    let protokoll = gemeinsam::protokoll::alle_zeilen(wurzel.path()).join("\n");
    for id in &ids {
        assert!(!protokoll.contains(id.as_str()), "{id} im Protokoll");
    }
}

/// M52: fremder `Origin` (auch `null`), `Sec-Fetch-Site: cross-site`, `text/plain`-Formular, fremder Host → 403
/// bzw. 415, nichts gelöscht; `OPTIONS` ohne `Access-Control-Allow-*`.
#[tokio::test]
async fn ursprung_und_rebinding_beim_loeschen() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let basis = instanz.adresse.as_str();
    let port = instanz.port();
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, basis, "beta", Some("agent")).await;
    let a = ablegen(&beta, basis, &token, "a.bin", vec![1u8; 10]).await;
    assert_eq!(
        senden_mit(&beta, basis, &token, "allgemein", std::slice::from_ref(&a))
            .await
            .status,
        201
    );
    let koerper = json!({"ids": [a], "bestaetigt": true}).to_string();

    for (zusatz, typ, erwartet) in [
        (
            "Host: 127.0.0.1\r\nOrigin: http://evil.example",
            "application/json",
            "403",
        ),
        ("Host: 127.0.0.1\r\nOrigin: null", "application/json", "403"),
        (
            "Host: 127.0.0.1\r\nSec-Fetch-Site: cross-site",
            "application/json",
            "403",
        ),
        ("Host: evil.example", "application/json", "403"),
        ("Host: 127.0.0.1", "text/plain", "415"),
    ] {
        let mut verbindung = roh_oeffnen(
            port,
            "127.0.0.1",
            &format!(
                "POST /api/anhaenge/loeschen HTTP/1.1\r\n{zusatz}\r\nX-LocalchatA-Sitzung: {token}\r\nContent-Type: {typ}\r\nContent-Length: {}\r\nConnection: close",
                koerper.len()
            ),
        )
        .await;
        tokio::io::AsyncWriteExt::write_all(&mut verbindung, koerper.as_bytes())
            .await
            .expect("Körper");
        let antwort = roh_antwort(&mut verbindung, Duration::from_secs(5)).await;
        assert!(
            antwort.starts_with(&format!("HTTP/1.1 {erwartet}")),
            "{zusatz} {typ}: {antwort}"
        );
    }
    let mut optionen = roh_oeffnen(
        port,
        "127.0.0.1",
        "OPTIONS /api/anhaenge/loeschen HTTP/1.1\r\nHost: 127.0.0.1\r\nOrigin: http://evil.example\r\nAccess-Control-Request-Method: POST\r\nConnection: close",
    )
    .await;
    let antwort = roh_antwort(&mut optionen, Duration::from_secs(5)).await;
    assert!(
        !antwort
            .to_ascii_lowercase()
            .contains("access-control-allow-"),
        "{antwort}"
    );
    assert_eq!(abrufen(basis, &a).await.0, 200, "nichts gelöscht");
}
