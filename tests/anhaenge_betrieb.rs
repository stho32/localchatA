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

/// R00038 F0129 (B0590, Integration): zwei Anmeldungen derselben Kennung über der Grenze mit verschiedenen `bytes`
/// (3 MiB, 4 MiB) bei Ruhe 200 ms → eine Beginn-Zeile mit `bytes=` der ersten, eine Ende-Zeile mit `kennung` und
/// `herkunft`, ohne `bytes=` und `hochladen_bis=`.
#[tokio::test]
async fn grosser_anspruch_ende_zeile_ohne_werte_des_ersten_auftretens() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[("LOCALCHATA_EPISODE_RUHE_MS", "200")]);
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, basis, "beta", Some("agent")).await;

    for megabyte in [3, 4] {
        let angemeldet =
            id(&anmelden(&beta, basis, &token, "gross.bin", megabyte * 1_048_576).await);
        assert_eq!(verwerfen(&beta, basis, &token, &angemeldet).await, 204);
    }
    let ende = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while zeilen_mit(wurzel.path(), "Großer Anspruch — Episode endet").is_empty()
        && std::time::Instant::now() < ende
    {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    drop(instanz);

    let beginn = zeilen_mit(wurzel.path(), "Großer Anspruch — Episode beginnt");
    assert_eq!(beginn.len(), 1, "{beginn:#?}");
    assert!(
        beginn[0].contains("bytes=3145728 hochladen_bis="),
        "{}",
        beginn[0]
    );
    let ende = zeilen_mit(wurzel.path(), "Großer Anspruch — Episode endet: 2 in ");
    assert_eq!(ende.len(), 1, "{ende:#?}");
    assert!(
        ende[0].ends_with("| kennung=beta@127.0.0.1 herkunft=127.0.0.1"),
        "{}",
        ende[0]
    );
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
        voll[0].contains("kennung=beta@127.0.0.1 hoechstens=10485760 bytes=2097152 belegt=9437184"),
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

/// Großes Hochladen (B0462): 64 MiB → genau eine `INFORMATION anhang Großes Hochladen abgeschlossen` mit
/// `bytes=67108864`, `dauer_ms`, `mib_s`; 64 MiB − 1 Byte → keine.
#[tokio::test]
async fn grosses_hochladen() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &["--anhangquote", "1GiB"], &[]);
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, &instanz.adresse, "beta", Some("agent")).await;

    for laenge in [64 * 1_048_576, 64 * 1_048_576 - 1] {
        ablegen(
            &beta,
            &instanz.adresse,
            &token,
            "gross.bin",
            vec![0u8; laenge],
        )
        .await;
    }
    drop(instanz);

    let zeilen = zeilen_mit(
        wurzel.path(),
        "INFORMATION anhang Großes Hochladen abgeschlossen",
    );
    assert_eq!(zeilen.len(), 1, "{zeilen:#?}");
    assert!(
        zeilen[0].contains("bytes=67108864 dauer_ms=") && zeilen[0].contains(" mib_s="),
        "{}",
        zeilen[0]
    );
}

/// Debug (B0462): je eine `DEBUG anhang`-Zeile für Anmeldung, Hochladen-Ende, Abbruch und Abruf (mit Range
/// `bereich` als Länge) mit `kennung`, `herkunft`, `bytes`, `art`, `dauer_ms` — die Datei nur als Präfix.
#[tokio::test]
async fn debug_ereignisse() {
    use tokio::io::AsyncWriteExt;
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &["--protokoll-stufe", "debug"], &[]);
    let basis = instanz.adresse.clone();
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, &basis, "beta", Some("agent")).await;
    let id = ablegen(&beta, &basis, &token, "a.bin", vec![0u8; 1000]).await;
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
    let teil = beta
        .anfrage(reqwest::Method::GET, format!("{basis}anhaenge/{id}"))
        .header("range", "bytes=0-99")
        .send()
        .await
        .expect("Abruf");
    assert_eq!(teil.status(), 206);
    let abbruch = gemeinsam::anhaenge::id(&anmelden(&beta, &basis, &token, "b.bin", 1000).await);
    let mut verbindung = roh_oeffnen(
        instanz.port(),
        "127.0.0.1",
        &put_kopf(&abbruch, &token, 1000),
    )
    .await;
    verbindung.write_all(&[0u8; 10]).await.expect("Anfang");
    drop(verbindung);
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    drop(instanz);

    let anmeldung = zeilen_mit(wurzel.path(), "DEBUG anhang Anmeldung angenommen");
    assert!(
        anmeldung[0]
            .contains("kennung=beta@127.0.0.1 herkunft=127.0.0.1 bytes=1000 datei=.eingang/"),
        "{}",
        anmeldung[0]
    );
    let ende = zeilen_mit(wurzel.path(), "DEBUG anhang Hochladen abgeschlossen");
    assert!(
        ende[0].contains("bytes=1000 art=application/octet-stream dauer_ms="),
        "{}",
        ende[0]
    );
    let abgebrochen = zeilen_mit(wurzel.path(), "DEBUG anhang Hochladen abgebrochen");
    assert_eq!(abgebrochen.len(), 1, "{abgebrochen:#?}");
    let abruf = zeilen_mit(wurzel.path(), "DEBUG anhang Anhang abgerufen");
    assert!(
        abruf[0].contains("herkunft=127.0.0.1")
            && abruf[0].contains("bytes=1000")
            && abruf[0].contains("bereich=100"),
        "{}",
        abruf[0]
    );
    for zeile in gemeinsam::protokoll::alle_zeilen(wurzel.path()) {
        assert!(
            !zeile.contains(id.as_str())
                && !zeile.contains(abbruch.as_str())
                && !zeile.contains("a.bin"),
            "{zeile}"
        );
    }
}

/// Warnschwelle (B0515): neun gebunden (G = 90 %) → keine Warnung; ein weiterer Anhang von 1 Byte → genau eine
/// `WARNING anhang Anhänge belegen 90 % der Anhangquote` mit `gebunden=9502720 hoechstens=10485760`; eine
/// Anmeldung ändert nichts; bestätigtes Löschen von 1 MiB → genau eine `INFORMATION … wieder unter 90 %` mit
/// `grund=geloescht`.
#[tokio::test]
async fn warnschwelle_der_anhang_quote() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let basis = instanz.adresse.clone();
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, &basis, "beta", Some("agent")).await;
    let mut ids = Vec::new();
    for nummer in 0..10 {
        let laenge = if nummer < 9 { 1_048_576 } else { 1 };
        let id = ablegen(
            &beta,
            &basis,
            &token,
            &format!("n{nummer}.bin"),
            vec![1u8; laenge],
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
        ids.push(id);
        if nummer == 8 {
            assert!(
                zeilen_mit(wurzel.path(), "Anhänge belegen 90 %").is_empty(),
                "genau 90 % warnt nicht"
            );
        }
    }
    id(&anmelden(&beta, &basis, &token, "a.bin", 65_536).await);
    let geloescht = beta
        .anfrage(
            reqwest::Method::POST,
            format!("{basis}api/anhaenge/loeschen"),
        )
        .header("Content-Type", "application/json")
        .header("X-LocalchatA-Sitzung", &token)
        .body(serde_json::json!({"ids": [ids[0]], "bestaetigt": true}).to_string())
        .send()
        .await
        .expect("Löschen");
    assert_eq!(geloescht.status(), 200);
    drop(instanz);

    let ueber = zeilen_mit(
        wurzel.path(),
        "WARNING anhang Anhänge belegen 90 % der Anhangquote",
    );
    assert_eq!(ueber.len(), 1, "{ueber:#?}");
    assert!(
        ueber[0].ends_with("| gebunden=9502720 hoechstens=10485760"),
        "{}",
        ueber[0]
    );
    let unter = zeilen_mit(
        wurzel.path(),
        "INFORMATION anhang Anhänge wieder unter 90 % der Anhangquote",
    );
    assert_eq!(unter.len(), 1, "{unter:#?}");
    assert!(
        unter[0].ends_with("| gebunden=8454144 hoechstens=10485760 grund=geloescht"),
        "{}",
        unter[0]
    );
}

/// Start (B0516, M46′): G = 8.454.144, Neustart mit `--anhangquote 8MiB` → `WARNING start Anhänge belegen mehr als
/// die Anhangquote`, keine Datei verändert; `INFORMATION start` nennt `anhangquote=8388608`,
/// `anhang_belegt=8454144`, `anhang_option=--anhangquote`; Anmelden 1 → 507 `voll`; nach bestätigtem Löschen von
/// zwei Anhängen zu 1 MiB → 201.
#[tokio::test]
async fn start_mit_gesenkter_anhangquote() {
    let wurzel = ordner();
    let mut ids = Vec::new();
    {
        let instanz = starten(wurzel.path(), &[], &[]);
        let beta = Absender::von("127.0.0.1");
        let token = beitreten(&beta, &instanz.adresse, "beta", Some("agent")).await;
        for nummer in 0..9 {
            let laenge = if nummer < 8 { 1_048_576 } else { 1 };
            let id = ablegen(
                &beta,
                &instanz.adresse,
                &token,
                &format!("n{nummer}.bin"),
                vec![1u8; laenge],
            )
            .await;
            let koerper = serde_json::json!({"text": "", "anhaenge": [id]})
                .to_string()
                .into_bytes();
            assert_eq!(
                roh_senden(
                    &beta,
                    &instanz.adresse,
                    Some(&token),
                    "allgemein",
                    "application/json",
                    koerper
                )
                .await
                .status,
                201
            );
            ids.push(id);
        }
        assert_eq!(
            gesundheit(&instanz.adresse).await["anhang_belegt"],
            8_454_144
        );
    }
    let vorher: Vec<(String, u64)> = dateien(wurzel.path());

    let instanz = starten(wurzel.path(), &["--anhangquote", "8MiB"], &[]);
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, &instanz.adresse, "beta2", Some("agent")).await;
    let voll = anmelden(&beta, &instanz.adresse, &token, "x.bin", 1).await;
    assert_eq!(
        (voll.status, &voll.json["grund"]),
        (507, &serde_json::json!("voll"))
    );
    assert_eq!(dateien(wurzel.path()), vorher, "keine Datei verändert");
    let antwort = beta
        .anfrage(
            reqwest::Method::POST,
            format!("{}api/anhaenge/loeschen", instanz.adresse),
        )
        .header("Content-Type", "application/json")
        .header("X-LocalchatA-Sitzung", &token)
        .body(serde_json::json!({"ids": [ids[0], ids[1]], "bestaetigt": true}).to_string())
        .send()
        .await
        .expect("Löschen");
    assert_eq!(antwort.status(), 200);
    assert_eq!(
        anmelden(&beta, &instanz.adresse, &token, "x.bin", 1)
            .await
            .status,
        201
    );
    drop(instanz);

    let warnung = zeilen_mit(
        wurzel.path(),
        "WARNING start Anhänge belegen mehr als die Anhangquote",
    );
    assert_eq!(warnung.len(), 1, "{warnung:#?}");
    let bereit: Vec<String> = zeilen_mit(wurzel.path(), "INFORMATION start Instanz bereit");
    assert!(
        bereit[1].contains("anhangquote=8388608 anhang_belegt=8454144 anhang_option=--anhangquote"),
        "{}",
        bereit[1]
    );
}

fn dateien(ordner: &Path) -> Vec<(String, u64)> {
    let anhaenge = ordner.join("channels/allgemein/anhaenge");
    let tag = std::fs::read_dir(&anhaenge)
        .expect("anhaenge")
        .next()
        .expect("Tag")
        .expect("Tag")
        .path();
    let mut liste: Vec<(String, u64)> = std::fs::read_dir(tag)
        .expect("Tag")
        .map(|e| {
            let e = e.expect("Eintrag");
            (
                e.file_name().to_string_lossy().into_owned(),
                e.metadata().expect("Länge").len(),
            )
        })
        .collect();
    liste.sort();
    liste
}

/// Keine ID, kein Name (B0464; M11, M18, M19): auf Debug Anmelden, Hochladen, Senden, Abruf mit und ohne Range,
/// 404, Abbruch, verfehlte Routen mit der ID, eine auf der Platte gelöschte Datei, eine unplausible Metazeile,
/// 422 `metadaten`, Löschen (409, 200, `fehler`) mit dem Anzeigenamen `Geheimplan Q4.pdf` → keine ID und weder
/// `Geheimplan` noch der ASCII-Teil im Protokoll; eine Datei nur als `<hhmmssmmm>-<n>`.
#[tokio::test]
async fn keine_id_und_kein_name_im_protokoll() {
    use tokio::io::AsyncWriteExt;
    let wurzel = ordner();
    let mut ids: Vec<String> = Vec::new();
    {
        let instanz = starten(wurzel.path(), &["--protokoll-stufe", "debug"], &[]);
        let basis = instanz.adresse.clone();
        let beta = Absender::von("127.0.0.1");
        let token = beitreten(&beta, &basis, "beta", Some("agent")).await;
        let senden = async |id: &str| {
            let koerper = serde_json::json!({"text": "", "anhaenge": [id]})
                .to_string()
                .into_bytes();
            roh_senden(
                &beta,
                &basis,
                Some(&token),
                "allgemein",
                "application/json",
                koerper,
            )
            .await
            .status
        };
        let a = ablegen(
            &beta,
            &basis,
            &token,
            "Geheimplan Q4.pdf",
            b"%PDF-1.7 Geheimplan".to_vec(),
        )
        .await;
        assert_eq!(senden(&a).await, 201);
        let b = ablegen(&beta, &basis, &token, "Geheimplan Q4.pdf", vec![0u8; 100]).await;
        assert_eq!(senden(&b).await, 201);
        let c = ablegen(&beta, &basis, &token, "Geheimplan Q4.pdf", vec![0u8; 100]).await;
        assert_eq!(senden(&c).await, 201);
        let client = reqwest::Client::new();
        for (pfad, range) in [
            (format!("anhaenge/{a}"), None),
            (format!("anhaenge/{a}"), Some("bytes=0-3")),
            (format!("anhaenge/{a}/x"), None),
            (format!("api/anhaenge/{a}/x"), None),
            (format!("anhaenge/{}", a.to_uppercase()), None),
        ] {
            let mut anfrage = client.get(format!("{basis}{pfad}"));
            if let Some(range) = range {
                anfrage = anfrage.header("range", range);
            }
            let _ = anfrage.send().await.expect("Abruf");
        }
        let abbruch = id(&anmelden(&beta, &basis, &token, "Geheimplan Q4.pdf", 1000).await);
        let mut verbindung = roh_oeffnen(
            instanz.port(),
            "127.0.0.1",
            &put_kopf(&abbruch, &token, 1000),
        )
        .await;
        verbindung.write_all(&[0u8; 10]).await.expect("Anfang");
        drop(verbindung);
        let kaputt = id(&anmelden(&beta, &basis, &token, "Geheimplan Q4.jpg", 20).await);
        let mut jpeg = vec![0xFF, 0xD8, 0xFF, 0xE1, 0xFF, 0xF0];
        jpeg.resize(20, 0);
        let mut verbindung =
            roh_oeffnen(instanz.port(), "127.0.0.1", &put_kopf(&kaputt, &token, 20)).await;
        verbindung.write_all(&jpeg).await.expect("JPEG");
        let antwort = roh_antwort(&mut verbindung, std::time::Duration::from_secs(5)).await;
        assert!(antwort.starts_with("HTTP/1.1 422"), "{antwort}");
        let anhaenge = wurzel.path().join("channels/allgemein/anhaenge");
        let tag = std::fs::read_dir(&anhaenge)
            .expect("anhaenge")
            .next()
            .expect("Tag")
            .expect("Tag")
            .path();
        let mut namen: Vec<std::path::PathBuf> = std::fs::read_dir(&tag)
            .expect("Tag")
            .map(|e| e.expect("Eintrag").path())
            .collect();
        namen.sort();
        std::fs::remove_file(&namen[1]).expect("Datei weg");
        let _ = client.get(format!("{basis}anhaenge/{b}")).send().await;
        let fremd = tempfile::NamedTempFile::new().expect("fremd");
        std::fs::remove_file(&namen[2]).expect("weg");
        std::os::unix::fs::symlink(fremd.path(), &namen[2]).expect("Symlink");
        let loeschen = async |koerper: serde_json::Value| {
            beta.anfrage(
                reqwest::Method::POST,
                format!("{basis}api/anhaenge/loeschen"),
            )
            .header("Content-Type", "application/json")
            .header("X-LocalchatA-Sitzung", &token)
            .body(koerper.to_string())
            .send()
            .await
            .expect("Löschen")
            .status()
            .as_u16()
        };
        assert_eq!(loeschen(serde_json::json!({"ids": [a]})).await, 409);
        assert_eq!(
            loeschen(serde_json::json!({"ids": [a, c], "bestaetigt": true})).await,
            200
        );
        ids.extend([a, b, c, abbruch, kaputt]);
    }
    let tagesdatei = std::fs::read_dir(wurzel.path().join("channels/allgemein"))
        .expect("Channel")
        .map(|e| e.expect("Eintrag").path())
        .find(|p| p.extension().is_some_and(|e| e == "log"))
        .expect("Tagesdatei");
    let mut inhalt = std::fs::read_to_string(&tagesdatei).expect("Tag");
    inhalt.push_str("@ 2026-10-29T23:59:00.000+01:00 beta@127.0.0.1\n+ anhang: zzzzzzzzzzzzzzzzzzzzzzzzzz 5 text/plain - ../Geheimplan_Q4.pdf Geheimplan Q4.pdf\n  x\n\n");
    std::fs::write(&tagesdatei, inhalt).expect("unplausibel");
    drop(starten(wurzel.path(), &["--protokoll-stufe", "debug"], &[]));

    let zeilen = gemeinsam::protokoll::alle_zeilen(wurzel.path());
    assert!(
        zeilen.iter().any(|z| z.contains("WARNING laden")),
        "unplausible Metazeile gemeldet"
    );
    assert!(
        zeilen
            .iter()
            .any(|z| z.contains("WARNING anhang Anhang fehlt")),
        "fehlende Datei gemeldet"
    );
    assert!(
        zeilen
            .iter()
            .any(|z| z.contains("ERROR anhang Löschen gescheitert")),
        "Löschen gescheitert gemeldet"
    );
    for zeile in &zeilen {
        for id in &ids {
            assert!(!zeile.contains(id.as_str()), "{id} in {zeile}");
        }
        assert!(!zeile.contains("Geheimplan"), "{zeile}");
    }
}
