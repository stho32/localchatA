//! R00029 F0097 — Hochladen über das echte Programm (B0429): Hauptweg, Abweisungen ohne Lesen, Abbruch und Erneut,
//! Fristen, nicht blockierend.

mod gemeinsam;

use gemeinsam::Absender;
use gemeinsam::anhaenge::{
    MIB, anmelden, hochladen, id, im_eingang, roh_antwort, roh_oeffnen, starten,
};
use gemeinsam::senden::{beitreten, ordner, senden};
use std::time::{Duration, Instant};
use tokio::io::AsyncWriteExt;

fn put_kopf(id: &str, token: &str, laenge: u64) -> String {
    format!(
        "PUT /api/anhaenge/{id} HTTP/1.1\r\nHost: 127.0.0.1\r\nX-LocalchatA-Sitzung: {token}\r\nContent-Type: application/octet-stream\r\nContent-Length: {laenge}"
    )
}

/// Hauptweg: 307.200 Bytes → 200 mit `gueltig_bis` ≈ + 15 min, Art `application/octet-stream` ohne Maße;
/// `.eingang/<id>` hat 307.200 Bytes, keine `.teil`.
#[tokio::test]
async fn hochladen_legt_die_datei_in_den_eingang() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, &instanz.adresse, "beta", Some("agent")).await;
    let id = id(&anmelden(&beta, &instanz.adresse, &token, "bericht.pdf", 307_200).await);

    let vorher = chrono::Utc::now();
    let antwort = hochladen(&beta, &instanz.adresse, &token, &id, vec![0x42; 307_200]).await;

    assert_eq!(antwort.status, 200, "{}", antwort.json);
    let anhang = &antwort.json["anhang"];
    assert_eq!(anhang["id"], id.as_str());
    assert_eq!(anhang["name"], "bericht.pdf");
    assert_eq!(anhang["bytes"], 307_200);
    assert_eq!(anhang["art"], "application/octet-stream");
    assert!(anhang["breite"].is_null() && anhang["hoehe"].is_null());
    let bis = chrono::DateTime::parse_from_rfc3339(anhang["gueltig_bis"].as_str().expect("Zeit"))
        .expect("RFC 3339");
    let ms = (bis.to_utc() - vorher).num_milliseconds();
    assert!((899_000..=901_000).contains(&ms), "gueltig_bis {ms} ms");
    assert_eq!(im_eingang(wurzel.path()), vec![id.clone()]);
    assert_eq!(
        std::fs::metadata(wurzel.path().join(".eingang").join(&id))
            .expect("Datei")
            .len(),
        307_200
    );
}

/// R00029 „Hochladen abgewiesen ohne Lesen“: falsche Länge 422 `laenge` mit `erwartet` und `Connection: close`;
/// chunked 411; `text/plain` 415; zweiter `PUT` 409 `fertig`; fremde und ausgedachte ID 404 `unbekannt`; nie eine
/// `.teil`.
#[tokio::test]
async fn abweisungen_vor_dem_ersten_byte() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let gamma = Absender::von("127.0.0.1");
    let token = beitreten(&beta, basis, "beta", Some("agent")).await;
    let token_gamma = beitreten(&gamma, basis, "gamma", Some("agent")).await;
    let id = id(&anmelden(&beta, basis, &token, "bericht.pdf", 307_200).await);
    let id_gamma =
        gemeinsam::anhaenge::id(&anmelden(&gamma, basis, &token_gamma, "g.pdf", 10).await);

    let mut kurz = roh_oeffnen(instanz.port(), "127.0.0.1", &put_kopf(&id, &token, 307_199)).await;
    let antwort = roh_antwort(&mut kurz, Duration::from_secs(5)).await;
    assert!(antwort.starts_with("HTTP/1.1 422"), "{antwort}");
    assert!(
        antwort.to_ascii_lowercase().contains("connection: close"),
        "{antwort}"
    );
    assert!(
        antwort.contains("\"erwartet\":307200") && antwort.contains("\"regel\":\"laenge\""),
        "{antwort}"
    );
    assert!(!antwort.contains("100 Continue"), "{antwort}");

    let chunked = format!(
        "PUT /api/anhaenge/{id} HTTP/1.1\r\nHost: 127.0.0.1\r\nX-LocalchatA-Sitzung: {token}\r\nContent-Type: application/octet-stream\r\nTransfer-Encoding: chunked"
    );
    let mut verbindung = roh_oeffnen(instanz.port(), "127.0.0.1", &chunked).await;
    let antwort = roh_antwort(&mut verbindung, Duration::from_secs(5)).await;
    assert!(antwort.starts_with("HTTP/1.1 411"), "{antwort}");

    let text = beta
        .anfrage(reqwest::Method::PUT, format!("{basis}api/anhaenge/{id}"))
        .header("Content-Type", "text/plain")
        .header("X-LocalchatA-Sitzung", &token)
        .body(vec![0u8; 307_200]);
    assert_eq!(gemeinsam::api_antwort(text).await.status, 415);

    let fremd = hochladen(&beta, basis, &token, &id_gamma, vec![0; 10]).await;
    assert_eq!(
        (fremd.status, fremd.json["grund"].as_str()),
        (404, Some("unbekannt"))
    );
    let ausgedacht = hochladen(
        &beta,
        basis,
        &token,
        "aaaaaaaaaaaaaaaaaaaaaaaaaa",
        vec![0; 10],
    )
    .await;
    assert_eq!(
        (ausgedacht.status, ausgedacht.json["grund"].as_str()),
        (404, Some("unbekannt"))
    );
    assert!(
        im_eingang(wurzel.path()).is_empty(),
        "{:?}",
        im_eingang(wurzel.path())
    );

    assert_eq!(
        hochladen(&beta, basis, &token, &id, vec![1; 307_200])
            .await
            .status,
        200
    );
    let zweiter = hochladen(&beta, basis, &token, &id, vec![1; 307_200]).await;
    assert_eq!(
        (zweiter.status, zweiter.json["regel"].as_str()),
        (409, Some("fertig"))
    );
    assert_eq!(im_eingang(wurzel.path()), vec![id]);
}

/// R00029 „Abbruch und Erneut“: nach 100 KiB Verbindung geschlossen → keine `.teil`; ein neuer `PUT` auf dieselbe
/// ID mit allen Bytes → 200.
#[tokio::test]
async fn nach_dem_abbruch_laedt_ein_neuer_put_hoch() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, &instanz.adresse, "beta", Some("agent")).await;
    let id = id(&anmelden(&beta, &instanz.adresse, &token, "bericht.pdf", 307_200).await);

    let mut verbindung =
        roh_oeffnen(instanz.port(), "127.0.0.1", &put_kopf(&id, &token, 307_200)).await;
    verbindung
        .write_all(&vec![7u8; 100 * 1024])
        .await
        .expect("100 KiB");
    drop(verbindung);
    let bis = Instant::now() + Duration::from_secs(5);
    while !im_eingang(wurzel.path()).is_empty() && Instant::now() < bis {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(
        im_eingang(wurzel.path()).is_empty(),
        "{:?}",
        im_eingang(wurzel.path())
    );

    // Der Abbruch wird erkannt, sobald die Instanz das Ende der Verbindung liest; bis dahin läuft der alte `PUT`
    // (409 `laeuft`) — gewartet wird auf das Merkmal, nicht auf eine Zeit.
    let mut erneut = hochladen(&beta, &instanz.adresse, &token, &id, vec![7; 307_200]).await;
    let bis = Instant::now() + Duration::from_secs(5);
    while erneut.status == 409 && Instant::now() < bis {
        tokio::time::sleep(Duration::from_millis(20)).await;
        erneut = hochladen(&beta, &instanz.adresse, &token, &id, vec![7; 307_200]).await;
    }
    assert_eq!(erneut.status, 200, "{}", erneut.json);
}

/// R00029 „Fristen“ über das Programm: 1 MiB angemeldet, 64 KiB, dann Stille → 408 spätestens 13 s nach dem
/// letzten Byte, keine `.teil`.
#[tokio::test]
async fn stille_nach_64_kib_ist_408_nach_der_stueckfrist() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, &instanz.adresse, "beta", Some("agent")).await;
    let id = id(&anmelden(&beta, &instanz.adresse, &token, "x.bin", MIB).await);

    let mut verbindung =
        roh_oeffnen(instanz.port(), "127.0.0.1", &put_kopf(&id, &token, MIB)).await;
    verbindung
        .write_all(&vec![1u8; 65_536])
        .await
        .expect("64 KiB");
    let letztes_byte = Instant::now();
    let antwort = roh_antwort(&mut verbindung, Duration::from_secs(20)).await;
    let dauer = letztes_byte.elapsed();

    assert!(antwort.starts_with("HTTP/1.1 408"), "{antwort}");
    assert!(dauer <= Duration::from_secs(13), "{dauer:?}");
    assert!(dauer >= Duration::from_secs(9), "{dauer:?}");
    assert!(im_eingang(wurzel.path()).is_empty());
}

/// R00029 „Nicht blockierend, schnell geprüft“: während ein `PUT` über 64 MiB mit 4 MiB/s läuft, antwortet Senden
/// einer Textnachricht 201, und ein zweiter Teilnehmer meldet an und lädt 1 KiB hoch.
#[tokio::test]
async fn ein_langsames_grosses_hochladen_blockiert_nichts() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &["--anhangquote", "1GiB"], &[]);
    let basis = instanz.adresse.clone();
    let beta = Absender::von("127.0.0.1");
    let alpha = Absender::von("127.0.0.2");
    let token = beitreten(&beta, &basis, "beta", Some("agent")).await;
    let token_alpha = beitreten(&alpha, &basis, "alpha", Some("agent")).await;
    let gross = 64 * MIB;
    let id = id(&anmelden(&beta, &basis, &token, "abbild.img", gross).await);
    let mut verbindung =
        roh_oeffnen(instanz.port(), "127.0.0.1", &put_kopf(&id, &token, gross)).await;
    let schreiber = tokio::spawn(async move {
        let stueck = vec![3u8; 256 * 1024];
        for _ in 0..(gross / (256 * 1024)) {
            verbindung.write_all(&stueck).await.expect("Stück");
            tokio::time::sleep(Duration::from_millis(62)).await;
        }
        roh_antwort(&mut verbindung, Duration::from_secs(30)).await
    });
    tokio::time::sleep(Duration::from_secs(1)).await;

    let gesendet = senden(&alpha, &basis, &token_alpha, "allgemein", "Hallo").await;
    assert_eq!(gesendet.status, 201, "{}", gesendet.json);
    let klein =
        gemeinsam::anhaenge::ablegen(&alpha, &basis, &token_alpha, "klein.txt", vec![b'k'; 1024])
            .await;
    assert!(!klein.is_empty());
    assert!(!schreiber.is_finished(), "das große Hochladen läuft noch");

    let antwort = schreiber.await.expect("Schreiber");
    assert!(
        antwort.starts_with("HTTP/1.1 200"),
        "{}",
        &antwort[..antwort.len().min(200)]
    );
}
