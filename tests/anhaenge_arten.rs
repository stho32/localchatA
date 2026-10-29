//! R00029 F0098 „Über das Programm“ (B0436): je eine Datei jeder Zeile der Arten-Tabelle anmelden und hochladen →
//! die 200 nennt `art`, `breite`, `hoehe` wie die Tabelle; `hochkant.jpg` (4000 × 3000, `Orientation` 6) →
//! 3000 × 4000. Name und Sender zählen nie (M29).

mod gemeinsam;

use gemeinsam::Absender;
use gemeinsam::anhaenge::{anmelden, hochladen, id, starten, verwerfen};
use gemeinsam::dateien;
use gemeinsam::senden::{beitreten, ordner};
use serde_json::{Value, json};

#[tokio::test]
async fn jede_zeile_der_tabelle_ueber_das_programm() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, basis, "beta", Some("agent")).await;
    let faelle: Vec<(&str, Vec<u8>, Value, Value, Value)> = vec![
        (
            "bild.png",
            dateien::png(1280, 800),
            json!("image/png"),
            json!(1280),
            json!(800),
        ),
        (
            "hochkant.jpg",
            dateien::hochkant_jpg(),
            json!("image/jpeg"),
            json!(3000),
            json!(4000),
        ),
        (
            "bild.gif",
            dateien::gif(320, 240),
            json!("image/gif"),
            json!(320),
            json!(240),
        ),
        (
            "bild.webp",
            dateien::webp_vp8x(640, 480),
            json!("image/webp"),
            json!(640),
            json!(480),
        ),
        (
            "doku.pdf",
            dateien::pdf(),
            json!("application/pdf"),
            Value::Null,
            Value::Null,
        ),
        (
            "film.mp4",
            dateien::mp4(b"isom", None),
            json!("video/mp4"),
            Value::Null,
            Value::Null,
        ),
        (
            "lied.m4a",
            dateien::mp4(b"M4A ", None),
            json!("audio/mp4"),
            Value::Null,
            Value::Null,
        ),
        (
            "film.mov",
            dateien::mp4(b"qt  ", None),
            json!("application/octet-stream"),
            Value::Null,
            Value::Null,
        ),
        (
            "film.webm",
            dateien::webm(),
            json!("video/webm"),
            Value::Null,
            Value::Null,
        ),
        (
            "ton.ogg",
            dateien::ogg(b"OpusHead\x01\x02"),
            json!("audio/ogg"),
            Value::Null,
            Value::Null,
        ),
        (
            "film.ogv",
            dateien::ogg(b"\x80theora\x03"),
            json!("video/ogg"),
            Value::Null,
            Value::Null,
        ),
        (
            "ton.mp3",
            dateien::mp3(),
            json!("audio/mpeg"),
            Value::Null,
            Value::Null,
        ),
        (
            "x.html",
            b"<html><script>alert(1)</script></html>".to_vec(),
            json!("text/plain; charset=utf-8"),
            Value::Null,
            Value::Null,
        ),
        (
            "bild.png",
            b"<svg><script>alert(1)</script></svg>".to_vec(),
            json!("text/plain; charset=utf-8"),
            Value::Null,
            Value::Null,
        ),
        (
            "x.txt",
            dateien::png(2, 3),
            json!("image/png"),
            json!(2),
            json!(3),
        ),
        (
            "roh.bin",
            vec![0, 1, 2, 3, 0, 255],
            json!("application/octet-stream"),
            Value::Null,
            Value::Null,
        ),
    ];
    for (name, bytes, art, breite, hoehe) in faelle {
        let id = id(&anmelden(&beta, basis, &token, name, bytes.len() as u64).await);
        let antwort = hochladen(&beta, basis, &token, &id, bytes).await;
        assert_eq!(antwort.status, 200, "{name}: {}", antwort.json);
        let anhang = &antwort.json["anhang"];
        assert_eq!(
            (&anhang["art"], &anhang["breite"], &anhang["hoehe"]),
            (&art, &breite, &hoehe),
            "{name}"
        );
        assert_eq!(verwerfen(&beta, basis, &token, &id).await, 204);
    }
}

/// Befund #51 B3, R00029 F0098 „Über das Programm“: `hochkant.jpg` (4000 × 3000, `Orientation` 6) gesendet → die
/// angezeigten Maße 3000 × 4000 stehen gleich in der Senden-Antwort, im Ereignis des Stroms, im Verlauf und als
/// `3000x4000` in der Metazeile der Tagesdatei.
#[tokio::test]
async fn nach_dem_senden_stehen_ueberall_die_angezeigten_masse() {
    use gemeinsam::senden::{roh_senden, verlauf};
    use gemeinsam::strom::Strom;
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let alpha = Absender::von("127.0.0.2");
    let token = beitreten(&beta, basis, "beta", Some("agent")).await;
    let token_alpha = beitreten(&alpha, basis, "alpha", Some("mensch")).await;
    let mut strom = Strom::oeffnen(&alpha, basis, &token_alpha).await;
    let foto = dateien::hochkant_jpg();
    let angemeldet = id(&anmelden(&beta, basis, &token, "hochkant.jpg", foto.len() as u64).await);
    assert_eq!(
        hochladen(&beta, basis, &token, &angemeldet, foto)
            .await
            .status,
        200
    );

    let gesendet = roh_senden(
        &beta,
        basis,
        Some(&token),
        "allgemein",
        "application/json",
        json!({"text": "", "anhaenge": [angemeldet]})
            .to_string()
            .into_bytes(),
    )
    .await;

    assert_eq!(gesendet.status, 201, "{}", gesendet.json);
    let masse = |anhang: &Value| {
        (
            anhang["art"].clone(),
            anhang["breite"].clone(),
            anhang["hoehe"].clone(),
        )
    };
    let erwartet = (json!("image/jpeg"), json!(3000), json!(4000));
    assert_eq!(masse(&gesendet.json["nachricht"]["anhaenge"][0]), erwartet);
    let ereignis = strom
        .bis(std::time::Duration::from_secs(5), |e| {
            e.data["art"] == "nachricht"
        })
        .await
        .expect("Ereignis");
    assert_eq!(masse(&ereignis.data["nachricht"]["anhaenge"][0]), erwartet);
    let gelesen = verlauf(&alpha, basis, &token_alpha, "allgemein").await;
    assert_eq!(masse(&gelesen["nachrichten"][0]["anhaenge"][0]), erwartet);
    let tag = &gesendet.json["nachricht"]["zeit"].as_str().expect("zeit")[..10];
    let satz = std::fs::read_to_string(wurzel.path().join(format!("channels/allgemein/{tag}.log")))
        .expect("Tag");
    let metazeile = satz
        .lines()
        .find(|zeile| zeile.starts_with(&format!("+ anhang: {angemeldet} ")))
        .expect("Metazeile");
    assert!(metazeile.contains(" image/jpeg 3000x4000 "), "{metazeile}");
}
