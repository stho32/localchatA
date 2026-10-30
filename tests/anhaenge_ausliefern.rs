//! R00029 F0100 — Ausliefern mit Art und Teilabruf über das echte Programm (B0446, B0447, B0467): Öffnen ohne
//! Sitzung, Download unter dem Anzeigenamen, Kopfzeilen überall (M14), Teilabruf (M4, M31), nicht gefunden (M30,
//! M33), Grenzschutz (M12, M34), kein Ort im Anhang (QS7), Deskriptoren nach 1.000 Bereichsanfragen.

mod gemeinsam;

use gemeinsam::anhaenge::{ablegen, starten};
use gemeinsam::dateien::{hochkant_jpg, mp4, pdf, webm};
use gemeinsam::protokoll::alle_zeilen;
use gemeinsam::senden::{beitreten, ordner, roh_senden};
use gemeinsam::{Absender, RohAntwort, roh_anfragen};
use reqwest::Method;
use serde_json::json;
use std::net::SocketAddr;

const SANDBOX: &str = "sandbox; default-src 'none'";
const NICHT_DA: &str = "Diesen Anhang gibt es nicht (mehr).";

struct Bereit {
    _wurzel: tempfile::TempDir,
    instanz: gemeinsam::Instanz,
    absender: Absender,
}

impl Bereit {
    fn basis(&self) -> &str {
        &self.instanz.adresse
    }

    fn ziel(&self) -> SocketAddr {
        let ohne = self
            .basis()
            .trim_start_matches("http://")
            .trim_end_matches('/');
        ohne.parse().expect("Adresse")
    }

    fn pfad(&self) -> &std::path::Path {
        self._wurzel.path()
    }

    async fn abrufen(
        &self,
        methode: Method,
        pfad: &str,
        kopf: &[(&str, &str)],
    ) -> reqwest::Response {
        let mut anfrage = self.absender.anfrage(
            methode,
            format!("{}{}", self.basis(), pfad.trim_start_matches('/')),
        );
        for (name, wert) in kopf {
            anfrage = anfrage.header(*name, *wert);
        }
        anfrage.send().await.expect("Antwort")
    }

    fn roh(&self, zeile: &str) -> RohAntwort {
        roh_anfragen(
            self.ziel(),
            &format!("{zeile}\r\nHost: {}", self.ziel()),
            "",
        )
    }
}

/// Eine Instanz, `beta` sendet die Dateien als Anhänge in `allgemein`; liefert die IDs in Reihenfolge.
async fn mit_anhaengen(dateien: &[(&str, Vec<u8>)]) -> (Bereit, Vec<String>) {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let absender = Absender::von("127.0.0.1");
    let token = beitreten(&absender, &instanz.adresse, "beta", Some("agent")).await;
    let mut ids = Vec::new();
    for (name, inhalt) in dateien {
        let id = ablegen(&absender, &instanz.adresse, &token, name, inhalt.clone()).await;
        let koerper = json!({"text": "", "anhaenge": [id]})
            .to_string()
            .into_bytes();
        let antwort = roh_senden(
            &absender,
            &instanz.adresse,
            Some(&token),
            "allgemein",
            "application/json",
            koerper,
        )
        .await;
        assert_eq!(antwort.status, 201, "{}", antwort.json);
        ids.push(id);
    }
    (
        Bereit {
            _wurzel: wurzel,
            instanz,
            absender,
        },
        ids,
    )
}

fn kopf(antwort: &reqwest::Response, name: &str) -> String {
    antwort
        .headers()
        .get(name)
        .map(|wert| wert.to_str().unwrap_or("?").to_string())
        .unwrap_or_default()
}

fn route_kopf_pruefen(antwort: &reqwest::Response, was: &str) {
    assert_eq!(kopf(antwort, "content-security-policy"), SANDBOX, "{was}");
    assert_eq!(kopf(antwort, "x-content-type-options"), "nosniff", "{was}");
    assert_eq!(
        kopf(antwort, "cross-origin-resource-policy"),
        "same-origin",
        "{was}"
    );
    assert_eq!(kopf(antwort, "referrer-policy"), "no-referrer", "{was}");
    assert_eq!(kopf(antwort, "x-frame-options"), "DENY", "{was}");
    let freigaben: Vec<&str> = antwort
        .headers()
        .keys()
        .map(|name| name.as_str())
        .filter(|name| {
            name.starts_with("access-control-") || *name == "cross-origin-embedder-policy"
        })
        .collect();
    assert_eq!(freigaben, Vec::<&str>::new(), "{was}");
}

const HTML: &[u8] = b"<html><script>localStorage.t</script></html>";

/// Öffnen ohne Sitzung: `x.html` → 200 `text/plain; charset=utf-8`, inline unter `x.html`, CSP `sandbox`, Körper
/// byte-gleich.
#[tokio::test]
async fn oeffnen_ohne_sitzung() {
    let (bereit, ids) = mit_anhaengen(&[("x.html", HTML.to_vec())]).await;

    let antwort = bereit
        .abrufen(Method::GET, &format!("/anhaenge/{}", ids[0]), &[])
        .await;

    assert_eq!(antwort.status(), 200);
    route_kopf_pruefen(&antwort, "200");
    assert_eq!(kopf(&antwort, "content-type"), "text/plain; charset=utf-8");
    assert_eq!(
        kopf(&antwort, "content-disposition"),
        "inline; filename=\"x.html\"; filename*=UTF-8''x.html"
    );
    assert_eq!(kopf(&antwort, "accept-ranges"), "bytes");
    assert_eq!(kopf(&antwort, "etag"), format!("\"{}\"", ids[0]));
    assert_eq!(kopf(&antwort, "cache-control"), "private, no-cache");
    assert_eq!(antwort.bytes().await.expect("Körper").as_ref(), HTML);
}

/// Download unter dem Anzeigenamen; Sonderzeichen (M28).
#[tokio::test]
async fn download_unter_dem_anzeigenamen() {
    let (bereit, ids) = mit_anhaengen(&[
        ("Bericht März.bin", vec![0, 1, 0, 2]),
        ("a;b'c%d*e f.txt", vec![0, 9]),
    ])
    .await;

    let bericht = bereit
        .abrufen(Method::GET, &format!("/anhaenge/{}", ids[0]), &[])
        .await;
    let sonder = bereit
        .abrufen(Method::GET, &format!("/anhaenge/{}", ids[1]), &[])
        .await;

    assert_eq!(kopf(&bericht, "content-type"), "application/octet-stream");
    assert_eq!(
        kopf(&bericht, "content-disposition"),
        "attachment; filename=\"Bericht_M_rz.bin\"; filename*=UTF-8''Bericht%20M%C3%A4rz.bin"
    );
    assert!(
        kopf(&sonder, "content-disposition")
            .ends_with("filename*=UTF-8''a%3Bb%27c%25d%2Ae%20f.txt")
    );
}

/// Kopfzeilen überall (M14): 200, 206, 304, 404 aus dem Handler und dem Router, 416, `HEAD`, 403, 405 → CSP
/// `sandbox` und `nosniff`; `/%61nhaenge/x` → `sandbox`; `/anhaengex`, `/api/anhaenge`, `/` → App-CSP. Mit einer PDF,
/// nicht mit einem WebM: das Kriterium nimmt die fünf Medienarten aus (Medien-CSP, geprüft je Art und Status in
/// `tests/anhaenge_medien_csp.rs`); CORP `same-origin`, kein `Access-Control-*`, kein COEP gelten überall.
#[tokio::test]
async fn kopfzeilen_ueberall() {
    let (bereit, ids) = mit_anhaengen(&[("dok.pdf", pdf())]).await;
    let pfad = format!("/anhaenge/{}", ids[0]);
    let etag = format!("\"{}\"", ids[0]);

    let faelle = [
        (Method::GET, pfad.clone(), vec![], 200),
        (Method::GET, pfad.clone(), vec![("range", "bytes=0-9")], 206),
        (
            Method::GET,
            pfad.clone(),
            vec![("if-none-match", etag.as_str())],
            304,
        ),
        (Method::GET, "/anhaenge/abc".to_string(), vec![], 404),
        (
            Method::GET,
            pfad.clone(),
            vec![("range", "bytes=999999-")],
            416,
        ),
        (Method::HEAD, pfad.clone(), vec![], 200),
    ];
    for (methode, ziel, kopfzeilen, status) in faelle {
        let antwort = bereit.abrufen(methode.clone(), &ziel, &kopfzeilen).await;
        assert_eq!(antwort.status(), status, "{methode} {ziel} {kopfzeilen:?}");
        route_kopf_pruefen(&antwort, &format!("{methode} {status}"));
        if status == 200 || status == 206 {
            assert_eq!(kopf(&antwort, "etag"), etag);
            assert_eq!(kopf(&antwort, "accept-ranges"), "bytes");
            assert_eq!(kopf(&antwort, "cache-control"), "private, no-cache");
        }
    }
    let router = bereit.abrufen(Method::GET, "/anhaenge/x/y", &[]).await;
    assert_eq!(router.status(), 404);
    assert_eq!(kopf(&router, "content-security-policy"), SANDBOX);
    assert_eq!(kopf(&router, "x-content-type-options"), "nosniff");
    let falsch = bereit.abrufen(Method::POST, &pfad, &[]).await;
    assert_eq!(falsch.status(), 405);
    assert_eq!(kopf(&falsch, "content-security-policy"), SANDBOX);
    let fremd = bereit.roh(&format!("GET {pfad} HTTP/1.1\r\nHost: evil.example"));
    assert_eq!(fremd.status, 403);
    assert_eq!(fremd.header("content-security-policy"), Some(SANDBOX));
    assert_eq!(fremd.header("x-content-type-options"), Some("nosniff"));
    let kodiert = bereit.roh("GET /%61nhaenge/x HTTP/1.1");
    assert_eq!(kodiert.header("content-security-policy"), Some(SANDBOX));
    for anders in ["/anhaengex", "/api/anhaenge", "/"] {
        let antwort = bereit.abrufen(Method::GET, anders, &[]).await;
        let csp = kopf(&antwort, "content-security-policy");
        assert!(
            csp.starts_with("default-src 'none'; script-src 'self'"),
            "{anders}: {csp}"
        );
    }
}

/// Teilabruf über das Programm (WebM bleibt byte-gleich).
#[tokio::test]
async fn teilabruf_ueber_das_programm() {
    let film = webm();
    let n = film.len();
    let (bereit, ids) = mit_anhaengen(&[("film.webm", film.clone())]).await;
    let pfad = format!("/anhaenge/{}", ids[0]);
    let etag = format!("\"{}\"", ids[0]);
    assert!(n > 200, "{n}");

    let teil = bereit
        .abrufen(Method::GET, &pfad, &[("range", "bytes=100-199")])
        .await;
    assert_eq!(teil.status(), 206);
    assert_eq!(kopf(&teil, "content-range"), format!("bytes 100-199/{n}"));
    assert_eq!(kopf(&teil, "content-length"), "100");
    assert_eq!(teil.bytes().await.expect("Teil").as_ref(), &film[100..200]);

    let zu_weit = bereit
        .abrufen(Method::GET, &pfad, &[("range", &format!("bytes={n}-"))])
        .await;
    assert_eq!(
        (zu_weit.status().as_u16(), kopf(&zu_weit, "content-range")),
        (416, format!("bytes */{n}"))
    );
    let mehrere = bereit
        .abrufen(Method::GET, &pfad, &[("range", "bytes=0-1,5-6")])
        .await;
    assert_eq!(mehrere.status(), 200);
    assert_eq!(mehrere.bytes().await.expect("ganz").len(), n);
    let passend = bereit
        .abrufen(
            Method::GET,
            &pfad,
            &[("range", "bytes=0-9"), ("if-range", &etag)],
        )
        .await;
    assert_eq!(passend.status(), 206);
    let anders = bereit
        .abrufen(
            Method::GET,
            &pfad,
            &[("range", "bytes=0-9"), ("if-range", "\"anders\"")],
        )
        .await;
    assert_eq!(anders.status(), 200);
    let gleich = bereit
        .abrufen(
            Method::GET,
            &pfad,
            &[("if-none-match", &etag), ("range", "bytes=0-9")],
        )
        .await;
    assert_eq!(gleich.status(), 304);
    assert!(gleich.bytes().await.expect("leer").is_empty());
    let alles = bereit
        .abrufen(
            Method::GET,
            &pfad,
            &[("range", &format!("bytes=-{}", n + 10))],
        )
        .await;
    assert_eq!(
        (alles.status().as_u16(), kopf(&alles, "content-range")),
        (206, format!("bytes 0-{}/{n}", n - 1))
    );
    let kopfabruf = bereit
        .abrufen(Method::HEAD, &pfad, &[("range", "bytes=0-9")])
        .await;
    assert_eq!(
        (
            kopfabruf.status().as_u16(),
            kopf(&kopfabruf, "content-length")
        ),
        (200, n.to_string())
    );
}

fn offene_deskriptoren(pid: u32) -> usize {
    std::fs::read_dir(format!("/proc/{pid}/fd"))
        .expect("fd")
        .count()
}

/// M31: 1.000 Bereichsanfragen nacheinander auf einer Verbindung → offene Deskriptoren danach wie davor.
#[tokio::test]
async fn tausend_bereichsanfragen_lassen_keinen_deskriptor_offen() {
    let (bereit, ids) = mit_anhaengen(&[("film.webm", webm())]).await;
    let pfad = format!("/anhaenge/{}", ids[0]);
    let pid = bereit.instanz.kind.id();
    let client = reqwest::Client::builder()
        .pool_max_idle_per_host(1)
        .build()
        .expect("Client");
    let url = format!("{}{}", bereit.basis(), pfad.trim_start_matches('/'));
    let erste = client
        .get(&url)
        .header("range", "bytes=0-0")
        .send()
        .await
        .expect("erste");
    let _ = erste.bytes().await;
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let vorher = offene_deskriptoren(pid);

    for nummer in 0..1_000u64 {
        let antwort = client
            .get(&url)
            .header(
                "range",
                format!("bytes={}-{}", nummer % 50, nummer % 50 + 3),
            )
            .send()
            .await
            .expect("Antwort");
        assert_eq!(antwort.status(), 206);
        assert_eq!(antwort.bytes().await.expect("Körper").len(), 4);
    }

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let nachher = offene_deskriptoren(pid);
    assert!(
        nachher <= vorher,
        "offene Deskriptoren {vorher} → {nachher}"
    );
}

/// Nicht gefunden: `abc`, ausgedachte gültige ID, Datei auf der Platte gelöscht (mit `WARNING anhang` nur mit
/// Präfix), Symlink statt Datei, Pfade mit `..%2f`, `%00`, Großbuchstaben → 404 mit dem Text.
#[tokio::test]
async fn nicht_gefunden() {
    let (bereit, ids) =
        mit_anhaengen(&[("geheim.bin", vec![1; 10]), ("zweite.bin", vec![2; 10])]).await;
    let anhaenge = std::fs::read_dir(bereit.pfad().join("channels/allgemein/anhaenge"))
        .expect("anhaenge")
        .next()
        .expect("Tag")
        .expect("Tag")
        .path();
    let datei = |ascii: &str| {
        std::fs::read_dir(&anhaenge)
            .expect("Tag")
            .map(|e| e.expect("Eintrag").path())
            .find(|p| p.to_string_lossy().ends_with(ascii))
            .expect("Datei")
    };
    std::fs::remove_file(datei("geheim.bin")).expect("gelöscht");
    let zweite = datei("zweite.bin");
    let fremd = tempfile::NamedTempFile::new().expect("fremd");
    std::fs::write(fremd.path(), vec![9; 10]).expect("fremde Bytes");
    std::fs::remove_file(&zweite).expect("weg");
    std::os::unix::fs::symlink(fremd.path(), &zweite).expect("Symlink");

    for pfad in [
        "/anhaenge/abc".to_string(),
        format!("/anhaenge/{}", "a".repeat(26)),
        format!("/anhaenge/{}", ids[0]),
        format!("/anhaenge/{}", ids[1]),
        format!("/anhaenge/{}", ids[0].to_uppercase()),
        "/anhaenge/..%2fanhaenge".to_string(),
        format!("/anhaenge/{}%00", ids[0]),
    ] {
        let antwort = bereit.abrufen(Method::GET, &pfad, &[]).await;
        assert_eq!(antwort.status(), 404, "{pfad}");
        assert_eq!(kopf(&antwort, "content-security-policy"), SANDBOX);
        assert_eq!(antwort.text().await.expect("Text"), NICHT_DA, "{pfad}");
    }
    let protokoll = alle_zeilen(bereit.pfad());
    let fehlt: Vec<&String> = protokoll
        .iter()
        .filter(|z| z.contains("WARNING anhang"))
        .collect();
    assert_eq!(fehlt.len(), 2, "{protokoll:#?}");
    for zeile in fehlt {
        assert!(
            zeile.contains("channels/allgemein")
                && zeile.contains("tag=")
                && zeile.contains("anhang="),
            "{zeile}"
        );
        for verboten in [ids[0].as_str(), ids[1].as_str(), "geheim", "zweite"] {
            assert!(!zeile.contains(verboten), "{verboten} in {zeile}");
        }
    }
}

/// Grenzschutz: fremder Host 403; eine mitgeschickte Sitzung ändert die Antwort nicht; `OPTIONS` ohne
/// `Access-Control-Allow-*`; kleine Methode und Override → nicht ausgeliefert (M33, M34).
#[tokio::test]
async fn grenzschutz_und_methoden() {
    let (bereit, ids) = mit_anhaengen(&[("a.txt", b"Hallo".to_vec())]).await;
    let pfad = format!("/anhaenge/{}", ids[0]);

    let mit_sitzung = bereit
        .abrufen(Method::GET, &pfad, &[("x-localchata-sitzung", "erfunden")])
        .await;
    assert_eq!(mit_sitzung.status(), 200);
    let optionen = bereit
        .abrufen(Method::OPTIONS, &pfad, &[("origin", "http://evil.example")])
        .await;
    assert!(
        optionen
            .headers()
            .keys()
            .all(|name| !name.as_str().starts_with("access-control-allow-"))
    );
    let klein = bereit.roh(&format!("get {pfad} HTTP/1.1"));
    assert_ne!(klein.status, 200, "{}", klein.status);
    let override_ = bereit.roh(&format!(
        "POST {pfad} HTTP/1.1\r\nX-HTTP-Method-Override: GET"
    ));
    assert_eq!(override_.status, 405);
}

/// Kein Ort im Anhang (QS7): Handyfoto und Handyvideo hochladen, senden, abrufen → ohne GPS, Modell und Zeit;
/// `Content-Length` = Anmeldung; `Range: bytes=0-99` → 206 aus der getilgten Datei; Bilddaten byte-gleich.
#[tokio::test]
async fn kein_ort_im_anhang() {
    let foto = hochkant_jpg();
    let video = mp4(b"isom", Some(b"+52.5200+013.4050/"));
    let (bereit, ids) =
        mit_anhaengen(&[("hochkant.jpg", foto.clone()), ("weg.mp4", video.clone())]).await;

    for (id, original) in ids.iter().zip([&foto, &video]) {
        let antwort = bereit
            .abrufen(Method::GET, &format!("/anhaenge/{id}"), &[])
            .await;
        assert_eq!(kopf(&antwort, "content-length"), original.len().to_string());
        let bytes = antwort.bytes().await.expect("Körper");
        for spur in [&b"GPS"[..], b"52.5200", b"Telefon", b"2026:10:29"] {
            assert!(
                !bytes.windows(spur.len()).any(|f| f == spur),
                "{}",
                String::from_utf8_lossy(spur)
            );
        }
        let teil = bereit
            .abrufen(
                Method::GET,
                &format!("/anhaenge/{id}"),
                &[("range", "bytes=0-99")],
            )
            .await;
        assert_eq!(teil.status(), 206);
        assert_eq!(teil.bytes().await.expect("Teil").as_ref(), &bytes[..100]);
    }
    let bild = bereit
        .abrufen(Method::GET, &format!("/anhaenge/{}", ids[0]), &[])
        .await
        .bytes()
        .await
        .expect("Bild");
    let sos = foto
        .windows(2)
        .position(|p| p == [0xff, 0xda])
        .expect("SOS");
    assert_eq!(&bild[sos..], &foto[sos..]);
    // Befund #53 B6: `Orientation` 6 steht genau einmal (im minimalen EXIF), in keiner Byte-Ordnung doppelt.
    let orientierung = |muster: &[u8]| bild.windows(muster.len()).filter(|f| *f == muster).count();
    assert_eq!(
        orientierung(b"\x01\x12\x00\x03\x00\x00\x00\x01\x00\x06")
            + orientierung(b"\x12\x01\x03\x00\x01\x00\x00\x00\x06\x00"),
        1
    );
    // Befund #53 B6: das Video ist ab `mdat` byte-gleich.
    let film = bereit
        .abrufen(Method::GET, &format!("/anhaenge/{}", ids[1]), &[])
        .await
        .bytes()
        .await
        .expect("Video");
    let mdat = video.windows(4).position(|p| p == b"mdat").expect("mdat") - 4;
    assert_eq!(&film[mdat..], &video[mdat..]);
}

/// Befund #53 B6, R00029 „Text über 16 MiB“ an der Route (F3): 16.777.216 Bytes `a` → `text/plain`, `inline`;
/// 16.777.217 → `text/plain`, `attachment`.
#[tokio::test]
async fn text_ueber_16_mib_nur_als_download() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &["--anhangquote", "64MiB"], &[]);
    let absender = Absender::von("127.0.0.1");
    let token = beitreten(&absender, &instanz.adresse, "beta", Some("agent")).await;
    for (laenge, erwartet) in [(16_777_216usize, "inline"), (16_777_217, "attachment")] {
        let id = ablegen(
            &absender,
            &instanz.adresse,
            &token,
            "gross.txt",
            vec![b'a'; laenge],
        )
        .await;
        let antwort = roh_senden(
            &absender,
            &instanz.adresse,
            Some(&token),
            "allgemein",
            "application/json",
            json!({"text": "", "anhaenge": [id]})
                .to_string()
                .into_bytes(),
        )
        .await;
        assert_eq!(antwort.status, 201, "{}", antwort.json);

        let abgerufen = absender
            .anfrage(Method::HEAD, format!("{}anhaenge/{id}", instanz.adresse))
            .send()
            .await
            .expect("Antwort");

        assert_eq!(abgerufen.status(), 200);
        assert!(
            kopf(&abgerufen, "content-type").starts_with("text/plain"),
            "{laenge}"
        );
        let disposition = kopf(&abgerufen, "content-disposition");
        assert!(
            disposition.starts_with(&format!("{erwartet};")),
            "{laenge}: {disposition}"
        );
    }
}

/// Befund #53 B6, R00029 „Grenzschutz“ mit echter Sitzung: `Host: evil.example` mit der Sitzung von `beta` → 403,
/// und die Sitzung wird nicht verlängert (`weg_um` von `beta` unverändert); Kontrolle: ein Aufruf der API mit
/// derselben Sitzung verlängert sie.
#[tokio::test]
async fn grenzschutz_verlaengert_keine_sitzung() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let beta = Absender::von("127.0.0.1");
    let gamma = Absender::von("127.0.0.3");
    let token = beitreten(&beta, &instanz.adresse, "beta", Some("agent")).await;
    let token_gamma = beitreten(&gamma, &instanz.adresse, "gamma", Some("agent")).await;
    let id = ablegen(&beta, &instanz.adresse, &token, "a.txt", b"Hallo".to_vec()).await;
    let gesendet = roh_senden(
        &beta,
        &instanz.adresse,
        Some(&token),
        "allgemein",
        "application/json",
        json!({"text": "", "anhaenge": [id]})
            .to_string()
            .into_bytes(),
    )
    .await;
    assert_eq!(gesendet.status, 201);
    let weg_um = || {
        let (gamma, token_gamma, basis) = (&gamma, &token_gamma, &instanz.adresse);
        async move {
            let liste = gemeinsam::api_antwort(
                gamma
                    .anfrage(Method::GET, format!("{basis}api/teilnehmer"))
                    .header("X-LocalchatA-Sitzung", token_gamma),
            )
            .await;
            liste.json["liste"]
                .as_array()
                .expect("liste")
                .iter()
                .find(|zeile| zeile["kennung"] == "beta@127.0.0.1")
                .expect("beta")["weg_um"]
                .clone()
        }
    };
    // Ohne Strom ist `beta` 1 s nach der letzten Anfrage `offline` und hat ein `weg_um`.
    tokio::time::sleep(std::time::Duration::from_millis(1_200)).await;
    let vorher = weg_um().await;
    assert!(vorher.is_string(), "{vorher}");
    tokio::time::sleep(std::time::Duration::from_millis(1_100)).await;
    let ziel: SocketAddr = instanz
        .adresse
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .parse()
        .expect("Adresse");

    let fremd = roh_anfragen(
        ziel,
        &format!(
            "GET /anhaenge/{id} HTTP/1.1\r\nHost: evil.example\r\nX-LocalchatA-Sitzung: {token}"
        ),
        "",
    );

    assert_eq!(fremd.status, 403);
    assert_eq!(weg_um().await, vorher, "Sitzung nicht verlängert");
    let kontrolle = gemeinsam::api_antwort(
        beta.anfrage(Method::GET, format!("{}api/teilnehmer", instanz.adresse))
            .header("X-LocalchatA-Sitzung", &token),
    )
    .await;
    assert_eq!(kontrolle.status, 200);
    let zeit = |wert: &serde_json::Value| {
        chrono::DateTime::parse_from_rfc3339(wert.as_str().expect("weg_um")).expect("RFC 3339")
    };
    let nachher = weg_um().await;
    assert!(
        nachher.is_string() && zeit(&nachher) > zeit(&vorher),
        "Kontrolle: die API verlängert — {vorher} → {nachher}"
    );
}

/// M30: nach dem Entfallen des Tags (Neustart 8 Tage später) → 404, auch mit `If-None-Match: "<id>"` (Existenz vor
/// Cache; Mutante „ETag vor `finden`“).
#[tokio::test]
async fn nach_dem_entfallen_404_auch_mit_passendem_etag() {
    let wurzel = ordner();
    let absender = Absender::von("127.0.0.1");
    let id = {
        let instanz = starten(
            wurzel.path(),
            &[],
            &[("LOCALCHATA_JETZT", "2026-10-20T12:00:00+02:00")],
        );
        let token = beitreten(&absender, &instanz.adresse, "beta", Some("agent")).await;
        let id = ablegen(&absender, &instanz.adresse, &token, "a.bin", vec![1; 10]).await;
        let koerper = json!({"text": "", "anhaenge": [id]})
            .to_string()
            .into_bytes();
        let antwort = roh_senden(
            &absender,
            &instanz.adresse,
            Some(&token),
            "allgemein",
            "application/json",
            koerper,
        )
        .await;
        assert_eq!(antwort.status, 201);
        let vorher = absender
            .anfrage(Method::GET, format!("{}anhaenge/{id}", instanz.adresse))
            .header("if-none-match", format!("\"{id}\""))
            .send()
            .await
            .expect("Antwort");
        assert_eq!(vorher.status(), 304);
        id
    };
    let instanz = starten(
        wurzel.path(),
        &[],
        &[("LOCALCHATA_JETZT", "2026-10-28T12:00:00+01:00")],
    );

    let antwort = absender
        .anfrage(Method::GET, format!("{}anhaenge/{id}", instanz.adresse))
        .header("if-none-match", format!("\"{id}\""))
        .send()
        .await
        .expect("Antwort");

    assert_eq!(antwort.status(), 404);
    assert_eq!(antwort.text().await.expect("Text"), NICHT_DA);
}
