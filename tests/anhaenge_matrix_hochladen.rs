//! R00029 B0468 — Angriffsmatrix Hochladen über das Programm (Linie L2): echtes Programm, `--plattenquote 10MiB
//! --anhangquote 10MiB`, Herkünfte 127.0.0.1–127.0.0.5, Rohanfragen über TCP. Kontrollpaar K1, K2; M5/M32, M6,
//! M12/M34, M20, M21, M23, M33, M41′ und M42′ ohne Löschen (den Gate-Wortlaut mit Löschen fährt B0512).

mod gemeinsam;

use gemeinsam::Absender;
use gemeinsam::anhaenge::{
    Q, anmelden, gesundheit, hochladen, id, im_eingang, roh_antwort, roh_oeffnen, starten,
};
use gemeinsam::dateien::png;
use gemeinsam::senden::{beitreten, ordner, senden};
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::time::Instant;

fn put_kopf(id: &str, token: &str, laenge: u64) -> String {
    format!(
        "PUT /api/anhaenge/{id} HTTP/1.1\r\nHost: 127.0.0.1\r\nX-LocalchatA-Sitzung: {token}\r\nContent-Type: application/octet-stream\r\nContent-Length: {laenge}"
    )
}

/// K1: Anmelden 300 KiB PNG → 201, `PUT` → 200 — färbt sie rot, misst die Probe nichts.
async fn k1(basis: &str, absender: &Absender, token: &str) {
    let mut bild = png(64, 64);
    bild.resize(307_200, 0);
    let angemeldet = id(&anmelden(absender, basis, token, "k1.png", bild.len() as u64).await);
    let antwort = hochladen(absender, basis, token, &angemeldet, bild).await;
    assert_eq!(antwort.status, 200, "K1: {}", antwort.json);
}

/// K1, K2 und M21: 127.0.0.2 und .3 je 4 Kennungen × 4 Anmeldungen à 1 Byte → die 17. von 127.0.0.2 429
/// `offen_herkunft` (K2); `beta@127.0.0.1` meldet an und lädt hoch (K1) — der Eingang ist nicht sperrbar.
#[tokio::test]
async fn kontrollpaar_und_eingang_nicht_sperrbar() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let basis = instanz.adresse.as_str();
    for herkunft in ["127.0.0.2", "127.0.0.3"] {
        let absender = Absender::von(herkunft);
        for name in ["a", "b", "c", "d"] {
            let name = format!("{name}{}", &herkunft[8..]);
            let token = beitreten(&absender, basis, &name, Some("agent")).await;
            for _ in 0..4 {
                id(&anmelden(&absender, basis, &token, "x", 1).await);
            }
        }
    }
    let zweite = Absender::von("127.0.0.2");
    let fuenfte = beitreten(&zweite, basis, "e", Some("agent")).await;
    let k2 = anmelden(&zweite, basis, &fuenfte, "x", 1).await;
    assert_eq!(
        (k2.status, &k2.json["grenze"]),
        (429, &serde_json::json!("offen_herkunft")),
        "K2"
    );

    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, basis, "beta", Some("agent")).await;
    k1(basis, &beta, &token).await;
}

/// M6: 127.0.0.2–.5 halten je 16 laufende `PUT` (4 Kennungen × 4, 131.072 angemeldet, 65.536 gesendet) → ein
/// weiterer `PUT` einer vorher angemeldeten ID von 127.0.0.1 → 429 `offen_gesamt` 64 mit `Retry-After: 5` und
/// `Connection: close`; Senden und Gesundheit von 127.0.0.1 wie ohne.
#[tokio::test]
async fn vierundsechzig_laufende_sind_die_grenze() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let basis = instanz.adresse.clone();
    let beta = Absender::von("127.0.0.1");
    let token_beta = beitreten(&beta, &basis, "beta", Some("agent")).await;
    let vorher = id(&anmelden(&beta, &basis, &token_beta, "x.bin", 1).await);
    let mut offen = Vec::new();
    for herkunft in ["127.0.0.2", "127.0.0.3", "127.0.0.4", "127.0.0.5"] {
        let absender = Absender::von(herkunft);
        for name in ["a", "b", "c", "d"] {
            let name = format!("{name}{}", &herkunft[8..]);
            let token = beitreten(&absender, &basis, &name, Some("agent")).await;
            for _ in 0..4 {
                let angemeldet = id(&anmelden(&absender, &basis, &token, "y", 131_072).await);
                let mut verbindung = roh_oeffnen(
                    instanz.port(),
                    herkunft,
                    &put_kopf(&angemeldet, &token, 131_072),
                )
                .await;
                verbindung.write_all(&[0u8; 65_536]).await.expect("Hälfte");
                offen.push(verbindung);
            }
        }
    }
    let beginn = Instant::now();
    while gesundheit(&basis).await["hochladungen"] != 64
        && beginn.elapsed() < Duration::from_secs(5)
    {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    let mut weitere = roh_oeffnen(
        instanz.port(),
        "127.0.0.1",
        &put_kopf(&vorher, &token_beta, 1),
    )
    .await;
    let antwort = roh_antwort(&mut weitere, Duration::from_secs(5)).await;

    assert!(antwort.starts_with("HTTP/1.1 429"), "{antwort}");
    let klein = antwort.to_ascii_lowercase();
    assert!(
        klein.contains("retry-after: 5") && klein.contains("connection: close"),
        "{antwort}"
    );
    assert!(
        antwort.contains("\"grenze\":\"offen_gesamt\"") && antwort.contains("\"hoechstens\":64"),
        "{antwort}"
    );
    let gesendet = senden(&beta, &basis, &token_beta, "allgemein", "trotzdem").await;
    assert_eq!(gesendet.status, 201);
    assert_eq!(gesundheit(&basis).await["status"], "bereit");
    drop(offen);
}

/// M23: zwei gleichzeitige `PUT` auf eine ID → einmal 200, einmal 409 `laeuft`.
#[tokio::test]
async fn zwei_put_auf_eine_id() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, basis, "beta", Some("agent")).await;
    let angemeldet = id(&anmelden(&beta, basis, &token, "a.bin", 100_000).await);
    let mut erster = roh_oeffnen(
        instanz.port(),
        "127.0.0.1",
        &put_kopf(&angemeldet, &token, 100_000),
    )
    .await;
    erster.write_all(&[1u8; 50_000]).await.expect("Hälfte");
    tokio::time::sleep(Duration::from_millis(200)).await;

    let zweiter = hochladen(&beta, basis, &token, &angemeldet, vec![2u8; 100_000]).await;
    erster.write_all(&[1u8; 50_000]).await.expect("Rest");
    let antwort = roh_antwort(&mut erster, Duration::from_secs(5)).await;

    assert_eq!(
        (zweiter.status, &zweiter.json["regel"]),
        (409, &serde_json::json!("laeuft")),
        "{}",
        zweiter.json
    );
    assert!(antwort.starts_with("HTTP/1.1 200"), "{antwort}");
    let datei = std::fs::read(wurzel.path().join(".eingang").join(&angemeldet)).expect("Datei");
    assert!(datei.iter().all(|byte| *byte == 1));
}

/// Eine Rohanfrage mit beliebigem Kopf und Körper; liefert die ganze Antwort (bis die Verbindung zu ist oder 5 s).
async fn roh(port: u16, kopf: &str, koerper: &[u8]) -> String {
    let mut verbindung = roh_oeffnen(port, "127.0.0.1", kopf).await;
    let _ = verbindung.write_all(koerper).await;
    roh_antwort(&mut verbindung, Duration::from_secs(5)).await
}

/// M5/M32: Rohkörper — doppelte und ungleiche `Content-Length`, `+5`, `-5`, `Transfer-Encoding: chunked` mit
/// `Content-Length`, `Transfer-Encoding: identity`, HTTP/1.0, `Expect: 100-continue` bei Abweisung → 400/411/422
/// ohne `100 Continue`; Bytes nach dem Körper werden nie Teil der Datei und nie eine zweite Anfrage — auch die 200
/// eines `PUT` schließt die Verbindung;
/// zu wenig Bytes → Stückfrist 408.
#[tokio::test]
async fn rohkoerper() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let basis = instanz.adresse.as_str();
    let port = instanz.port();
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, basis, "beta", Some("agent")).await;
    let angemeldet = id(&anmelden(&beta, basis, &token, "a.bin", 10).await);
    let anfang = format!(
        "PUT /api/anhaenge/{angemeldet} HTTP/1.1\r\nHost: 127.0.0.1\r\nX-LocalchatA-Sitzung: {token}\r\nContent-Type: application/octet-stream"
    );

    for (fall, kopf) in [
        (
            "doppelt ungleich",
            format!("{anfang}\r\nContent-Length: 10\r\nContent-Length: 11"),
        ),
        ("plus", format!("{anfang}\r\nContent-Length: +5")),
        ("minus", format!("{anfang}\r\nContent-Length: -5")),
        (
            "chunked und Länge",
            format!("{anfang}\r\nTransfer-Encoding: chunked\r\nContent-Length: 10"),
        ),
        (
            "identity",
            format!("{anfang}\r\nTransfer-Encoding: identity\r\nContent-Length: 10"),
        ),
        (
            "expect bei Abweisung",
            format!("{anfang}\r\nContent-Length: 9\r\nExpect: 100-continue"),
        ),
    ] {
        let antwort = roh(port, &kopf, b"").await;
        let status: u16 = antwort
            .get(9..12)
            .and_then(|code| code.parse().ok())
            .unwrap_or(0);
        assert!([400, 411, 422].contains(&status), "{fall}: {antwort}");
        assert!(!antwort.contains("100 Continue"), "{fall}: {antwort}");
    }
    assert!(im_eingang(wurzel.path()).is_empty());

    let alt = roh(
        port,
        &format!(
            "PUT /api/anhaenge/{angemeldet} HTTP/1.0\r\nHost: 127.0.0.1\r\nX-LocalchatA-Sitzung: {token}\r\nContent-Type: application/octet-stream"
        ),
        b"0123456789",
    )
    .await;
    assert!(
        alt.starts_with("HTTP/1.0 411"),
        "HTTP/1.0 ohne Länge: {alt}"
    );

    // Zwei gleiche `Content-Length` fasst hyper zu einer zusammen (RFC 9110 §8.6 erlaubt es); die Länge ist dann
    // eindeutig — kein Schmuggel, die Datei hat genau die Bytes des Körpers.
    let gleich = id(&anmelden(&beta, basis, &token, "c.bin", 10).await);
    let doppelt = roh(
        port,
        &format!(
            "PUT /api/anhaenge/{gleich} HTTP/1.1\r\nHost: 127.0.0.1\r\nX-LocalchatA-Sitzung: {token}\r\nContent-Type: application/octet-stream\r\nContent-Length: 10\r\nContent-Length: 10"
        ),
        b"0123456789",
    )
    .await;
    assert!(
        doppelt.starts_with("HTTP/1.1 200") || doppelt.starts_with("HTTP/1.1 400"),
        "doppelt gleich: {doppelt}"
    );
    if doppelt.starts_with("HTTP/1.1 200") {
        assert_eq!(
            std::fs::read(wurzel.path().join(".eingang").join(&gleich)).expect("Datei"),
            b"0123456789"
        );
    }

    let geschmuggelt = "0123456789GET /api/gesundheit HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n";
    let antwort = roh(
        port,
        &format!("{anfang}\r\nContent-Length: 10"),
        geschmuggelt.as_bytes(),
    )
    .await;
    assert!(antwort.starts_with("HTTP/1.1 200"), "{antwort}");
    assert_eq!(
        antwort.matches("HTTP/1.1 ").count(),
        1,
        "keine zweite Anfrage aus Körperbytes: {antwort}"
    );
    assert_eq!(
        std::fs::read(wurzel.path().join(".eingang").join(&angemeldet)).expect("Datei"),
        b"0123456789"
    );

    let wenig = id(&anmelden(&beta, basis, &token, "b.bin", 100).await);
    let beginn = Instant::now();
    let mut verbindung = roh_oeffnen(port, "127.0.0.1", &put_kopf(&wenig, &token, 100)).await;
    verbindung.write_all(&[0u8; 50]).await.expect("Hälfte");
    let antwort = roh_antwort(&mut verbindung, Duration::from_secs(15)).await;
    assert!(antwort.starts_with("HTTP/1.1 408"), "{antwort}");
    assert!(beginn.elapsed() <= Duration::from_secs(30));
}

/// M33: `X-HTTP-Method-Override`, Methode klein, `..%2f`, `%00`, ID in Großbuchstaben → 405 bzw. 404.
#[tokio::test]
async fn methode_und_pfad() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let basis = instanz.adresse.as_str();
    let port = instanz.port();
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, basis, "beta", Some("agent")).await;
    let angemeldet = id(&anmelden(&beta, basis, &token, "a.bin", 10).await);
    let sitzung = format!(
        "Host: 127.0.0.1\r\nX-LocalchatA-Sitzung: {token}\r\nContent-Type: application/octet-stream\r\nContent-Length: 10"
    );

    let ueberschrieben = roh(
        port,
        &format!(
            "POST /api/anhaenge/{angemeldet} HTTP/1.1\r\nX-HTTP-Method-Override: PUT\r\n{sitzung}"
        ),
        b"0123456789",
    )
    .await;
    assert!(
        ueberschrieben.starts_with("HTTP/1.1 405"),
        "{ueberschrieben}"
    );
    let klein = roh(
        port,
        &format!("put /api/anhaenge/{angemeldet} HTTP/1.1\r\n{sitzung}"),
        b"0123456789",
    )
    .await;
    assert!(!klein.starts_with("HTTP/1.1 200"), "{klein}");
    for pfad in [
        format!("/api/anhaenge/..%2f{angemeldet}"),
        format!("/api/anhaenge/{angemeldet}%00"),
        format!("/api/anhaenge/{}", angemeldet.to_uppercase()),
    ] {
        let antwort = roh(
            port,
            &format!("PUT {pfad} HTTP/1.1\r\n{sitzung}"),
            b"0123456789",
        )
        .await;
        assert!(antwort.starts_with("HTTP/1.1 404"), "{pfad}: {antwort}");
    }
    assert!(im_eingang(wurzel.path()).is_empty());
}

/// M12/M34: `POST`/`PUT`/`DELETE /api/anhaenge` mit fremdem `Origin`, `Sec-Fetch-Site: cross-site`, `text/plain`-
/// Formular, fremdem Host → 403 (bzw. 415) ohne Wirkung; `OPTIONS` ohne `Access-Control-Allow-*`.
#[tokio::test]
async fn ursprung_und_rebinding() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let basis = instanz.adresse.as_str();
    let port = instanz.port();
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, basis, "beta", Some("agent")).await;
    let angemeldet = id(&anmelden(&beta, basis, &token, "a.bin", 10).await);
    let vorher = gesundheit(basis).await["reserviert"].clone();

    for (methode, pfad, typ, koerper) in [
        (
            "POST",
            "/api/anhaenge".to_string(),
            "application/json",
            "{\"name\":\"x\",\"bytes\":1}".to_string(),
        ),
        (
            "PUT",
            format!("/api/anhaenge/{angemeldet}"),
            "application/octet-stream",
            "0123456789".to_string(),
        ),
        (
            "DELETE",
            format!("/api/anhaenge/{angemeldet}"),
            "application/json",
            String::new(),
        ),
    ] {
        for zusatz in [
            "Host: 127.0.0.1\r\nOrigin: http://evil.example".to_string(),
            "Host: 127.0.0.1\r\nSec-Fetch-Site: cross-site".to_string(),
            "Host: evil.example".to_string(),
        ] {
            let antwort = roh(
                port,
                &format!(
                    "{methode} {pfad} HTTP/1.1\r\n{zusatz}\r\nX-LocalchatA-Sitzung: {token}\r\nContent-Type: {typ}\r\nContent-Length: {}",
                    koerper.len()
                ),
                koerper.as_bytes(),
            )
            .await;
            assert!(
                antwort.starts_with("HTTP/1.1 403"),
                "{methode} {zusatz}: {antwort}"
            );
        }
        if methode == "DELETE" {
            continue;
        }
        let formular = roh(
            port,
            &format!(
                "{methode} {pfad} HTTP/1.1\r\nHost: 127.0.0.1\r\nX-LocalchatA-Sitzung: {token}\r\nContent-Type: text/plain\r\nContent-Length: {}",
                koerper.len()
            ),
            koerper.as_bytes(),
        )
        .await;
        assert!(
            formular.starts_with("HTTP/1.1 415") || formular.starts_with("HTTP/1.1 403"),
            "{methode} text/plain: {formular}"
        );
    }
    let optionen = roh(port, "OPTIONS /api/anhaenge HTTP/1.1\r\nHost: 127.0.0.1\r\nOrigin: http://evil.example\r\nAccess-Control-Request-Method: POST", b"").await;
    assert!(
        !optionen
            .to_ascii_lowercase()
            .contains("access-control-allow-"),
        "{optionen}"
    );
    assert!(im_eingang(wurzel.path()).is_empty());
    assert_eq!(gesundheit(basis).await["reserviert"], vorher);
    assert_eq!(
        hochladen(&beta, basis, &token, &angemeldet, b"0123456789".to_vec())
            .await
            .status,
        200,
        "Anmeldung unverändert"
    );
}

/// Ein `PUT` auf `id` mit `Content-Length: laenge`, nach einem Byte geschlossen.
async fn ein_byte_und_zu(port: u16, herkunft: &str, id: &str, token: &str, laenge: u64) {
    let mut verbindung = roh_oeffnen(port, herkunft, &put_kopf(id, token, laenge)).await;
    let _ = verbindung.write_all(&[0]).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    drop(verbindung);
}

/// Hält eine Anmeldung von `bytes` (Absender von 127.0.0.2) mit einem `PUT` alle 50 s nach t0 (nach 1 Byte geschlossen), misst über Health
/// ab t0 + 60 s alle 500 ms: bei t0 + 70 s noch reserviert (Kontrolle), 0 spätestens bei `frei_bis`; der erste
/// `PUT` danach antwortet 404 `abgelaufen`. Liefert die Zeit, zu der `reserviert` 0 wurde.
async fn halten_bis_zum_lebensende(
    instanz: &gemeinsam::Instanz,
    absender: &Absender,
    token: &str,
    bytes: u64,
    frei_bis: Duration,
    waehrend: impl AsyncFn(),
) -> Duration {
    let basis = instanz.adresse.as_str();
    let angemeldet = id(&anmelden(absender, basis, token, "halten.bin", bytes).await);
    let t0 = Instant::now();
    let port = instanz.port();
    let takt = {
        let (angemeldet, token) = (angemeldet.clone(), token.to_string());
        tokio::spawn(async move {
            for runde in 1u32.. {
                tokio::time::sleep_until(t0 + Duration::from_secs(50) * runde).await;
                ein_byte_und_zu(port, "127.0.0.2", &angemeldet, &token, bytes).await;
            }
        })
    };
    tokio::time::sleep_until(t0 + Duration::from_secs(60)).await;
    waehrend().await;
    let mut kontrolle = false;
    let frei = loop {
        let reserviert = gesundheit(basis).await["reserviert"]
            .as_u64()
            .expect("reserviert");
        let jetzt = t0.elapsed();
        if jetzt >= Duration::from_secs(70)
            && jetzt < Duration::from_secs(71)
            && reserviert == bytes
        {
            kontrolle = true;
        }
        if reserviert == 0 {
            break jetzt;
        }
        assert!(jetzt <= frei_bis, "reserviert {reserviert} bei {jetzt:?}");
        tokio::time::sleep(Duration::from_millis(500)).await;
    };
    takt.abort();
    assert!(
        kontrolle,
        "bei t0 + 70 s war die Reservierung nicht gehalten — die Probe misst nichts"
    );
    let mut danach = roh_oeffnen(port, "127.0.0.2", &put_kopf(&angemeldet, token, bytes)).await;
    let antwort = roh_antwort(&mut danach, Duration::from_secs(5)).await;
    assert!(
        antwort.starts_with("HTTP/1.1 404") && antwort.contains("\"grund\":\"abgelaufen\""),
        "{antwort}"
    );
    frei
}

/// M20 (wörtlich): „Anmelden 9.437.184, 20 min alle 50 s PUT nach 1 Byte geschlossen → spätestens nach der absoluten
/// Lebensdauer 404 `abgelaufen`, reserviert 0.“ — Lebensende t0 + 78 s; 0 spätestens t0 + 103 s.
#[tokio::test]
async fn m20_reservierung_halten_endet_am_lebensende() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let beta = Absender::von("127.0.0.2");
    let token = beitreten(&beta, &instanz.adresse, "beta", Some("agent")).await;

    let frei = halten_bis_zum_lebensende(
        &instanz,
        &beta,
        &token,
        9_437_184,
        Duration::from_secs(103),
        async || {},
    )
    .await;

    println!("M20: reserviert 0 nach {frei:?}");
}

/// M42′ ohne Löschen: `beta@127.0.0.2` hält 10.485.760; währenddessen `alpha` Anmelden 1 → 507 `hochladungen` mit
/// `Retry-After: 30`, Senden 201, Health `reserviert` 10.485.760 und `hochladungen` 1; frei spätestens t0 + 106 s;
/// danach `alpha` 201.
#[tokio::test]
async fn m42_anspruch_haelt_platz_bis_zum_lebensende() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let basis = instanz.adresse.clone();
    let beta = Absender::von("127.0.0.2");
    let alpha = Absender::von("127.0.0.1");
    let token = beitreten(&beta, &basis, "beta", Some("agent")).await;

    let waehrend = async || {
        let token_alpha = beitreten(&alpha, &basis, "alpha", Some("agent")).await;
        let abgewiesen = anmelden(&alpha, &basis, &token_alpha, "a.bin", 1).await;
        assert_eq!(
            (abgewiesen.status, &abgewiesen.json["grund"]),
            (507, &serde_json::json!("hochladungen"))
        );
        assert_eq!(
            abgewiesen
                .kopf
                .get("retry-after")
                .and_then(|w| w.to_str().ok()),
            Some("30")
        );
        assert_eq!(
            senden(&alpha, &basis, &token_alpha, "allgemein", "Text geht")
                .await
                .status,
            201
        );
        let stand = gesundheit(&basis).await;
        assert_eq!(
            (stand["reserviert"].as_u64(), stand["hochladungen"].as_u64()),
            (Some(Q), Some(1))
        );
    };
    let frei = halten_bis_zum_lebensende(
        &instanz,
        &beta,
        &token,
        Q,
        Duration::from_secs(106),
        waehrend,
    )
    .await;

    println!("M42′: reserviert 0 nach {frei:?}");
    let token_alpha = beitreten(&alpha, &basis, "alpha-danach", Some("agent")).await;
    assert_eq!(
        anmelden(&alpha, &basis, &token_alpha, "a.bin", Q)
            .await
            .status,
        201
    );
}

/// M41′ ohne Löschen: leere Instanz, Anmelden 10.485.760, nach 5 MiB Verbindung geschlossen → kein `.teil`,
/// `reserviert` 0 spätestens 80 s nach dem Abbruch; danach `alpha` über 10.485.760 → 201.
#[tokio::test]
async fn m41_abbruch_bei_ganzer_quote() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let alpha = Absender::von("127.0.0.1");
    let token = beitreten(&beta, basis, "beta", Some("agent")).await;
    let angemeldet = id(&anmelden(&beta, basis, &token, "gross.bin", Q).await);
    let mut verbindung = roh_oeffnen(
        instanz.port(),
        "127.0.0.1",
        &put_kopf(&angemeldet, &token, Q),
    )
    .await;
    verbindung
        .write_all(&vec![0u8; 5 * 1_048_576])
        .await
        .expect("5 MiB");
    drop(verbindung);
    let abbruch = Instant::now();

    let beginn = Instant::now();
    while !im_eingang(wurzel.path()).is_empty() && beginn.elapsed() < Duration::from_secs(5) {
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(im_eingang(wurzel.path()).is_empty(), "kein .teil");
    while gesundheit(basis).await["reserviert"] != 0 {
        assert!(
            abbruch.elapsed() <= Duration::from_secs(80),
            "reserviert noch nach 80 s"
        );
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    println!("M41′: reserviert 0 nach {:?}", abbruch.elapsed());
    let token_alpha = beitreten(&alpha, basis, "alpha", Some("agent")).await;
    assert_eq!(
        anmelden(&alpha, basis, &token_alpha, "b.bin", Q)
            .await
            .status,
        201
    );
}
