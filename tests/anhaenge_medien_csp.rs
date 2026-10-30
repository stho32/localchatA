//! R00029 F0100 Karte #71 (B0664, B0726; ADR-0014 Ergänzung „Karte #71 (Chrome 154)“ Regeln 1–5, Bedingung C1,
//! Proben S1, S5; Kriterien „Medien-CSP nach Art, nicht nach Anfrage“, „Kopfzeilen überall“) über das echte
//! Programm: je Art der Tabelle und je Status trägt nur die Antwort auf die Datei einer der fünf Medienarten die
//! Medien-CSP; alles andere bleibt bei `sandbox; default-src 'none'`. CORP ist überall `same-origin`. Was die Anfrage
//! mitbringt (Inhaltstyp, Name, eigene Kopfzeilen, `Origin`), ändert daran nichts.
//!
//! Die Wache (C1): die Werte stehen hier als Literal, nie als Konstante des Programms — sonst mutierten Konstante und
//! Test zusammen. Ein `allow-scripts` in der Medien-CSP fängt kein E2E, nur dieser Vergleich.

mod gemeinsam;

use gemeinsam::Absender;
use gemeinsam::anhaenge::{ablegen, starten};
use gemeinsam::dateien;
use gemeinsam::senden::{beitreten, ordner, roh_senden};
use reqwest::Method;
use serde_json::json;

const ANHANG_CSP: &str = "sandbox; default-src 'none'";
const ANHANG_CSP_MEDIUM: &str = "sandbox allow-same-origin; default-src 'none'; media-src 'self'";

/// Die Arten, deren Direktlink abspielt (Regel 5) — `video/ogg` gehört nicht dazu, es wird geladen.
const MEDIENARTEN: [&str; 5] = [
    "video/mp4",
    "audio/mp4",
    "video/webm",
    "audio/ogg",
    "audio/mpeg",
];

/// Je Art der Tabelle (ADR-0047 Frage 3) eine Datei: Name, Inhalt, Art, die der Server daraus bestimmt.
fn je_art() -> Vec<(&'static str, Vec<u8>, &'static str)> {
    vec![
        ("bild.png", dateien::png(4, 3), "image/png"),
        ("hochkant.jpg", dateien::hochkant_jpg(), "image/jpeg"),
        ("bild.gif", dateien::gif(4, 3), "image/gif"),
        ("bild.webp", dateien::webp_vp8x(4, 3), "image/webp"),
        ("foto.avif", dateien::avif_foto(), "image/avif"),
        ("foto.heic", dateien::mp4(b"heic", None), "image/heic"),
        ("foto.heif", dateien::mp4(b"mif1", None), "image/heif"),
        ("doku.pdf", dateien::pdf(), "application/pdf"),
        ("film.mp4", dateien::mp4(b"isom", None), "video/mp4"),
        ("lied.m4a", dateien::mp4(b"M4A ", None), "audio/mp4"),
        ("film.webm", dateien::webm(), "video/webm"),
        ("film.ogv", dateien::ogg(b"\x80theora\x03"), "video/ogg"),
        ("ton.ogg", dateien::ogg(b"OpusHead\x01\x02"), "audio/ogg"),
        ("ton.mp3", dateien::mp3(), "audio/mpeg"),
        (
            "notiz.txt",
            b"Hallo, Welt".to_vec(),
            "text/plain; charset=utf-8",
        ),
        (
            "daten.bin",
            vec![0, 1, 2, 3, 0xff, 0xfe],
            "application/octet-stream",
        ),
    ]
}

fn ist_medienart(art: &str) -> bool {
    MEDIENARTEN.contains(&art)
}

struct Bereit {
    _wurzel: tempfile::TempDir,
    instanz: gemeinsam::Instanz,
    absender: Absender,
    token: String,
}

impl Bereit {
    async fn neu() -> Bereit {
        let wurzel = ordner();
        let instanz = starten(wurzel.path(), &[], &[]);
        let absender = Absender::von("127.0.0.1");
        let token = beitreten(&absender, &instanz.adresse, "beta", Some("agent")).await;
        Bereit {
            _wurzel: wurzel,
            instanz,
            absender,
            token,
        }
    }

    /// Ablegen und als Nachricht ohne Text in `allgemein` senden; liefert die ID. Je Datei eine eigene Kennung
    /// (`nummer`), damit die Bremse der Anmeldungen je Kennung nicht greift.
    async fn senden(&self, nummer: usize, name: &str, inhalt: Vec<u8>) -> String {
        let basis = &self.instanz.adresse;
        let kennung = format!("sender{nummer}");
        let token = beitreten(&self.absender, basis, &kennung, Some("agent")).await;
        let id = ablegen(&self.absender, basis, &token, name, inhalt).await;
        let koerper = json!({"text": "", "anhaenge": [id]})
            .to_string()
            .into_bytes();
        let antwort = roh_senden(
            &self.absender,
            basis,
            Some(&token),
            "allgemein",
            "application/json",
            koerper,
        )
        .await;
        assert_eq!(antwort.status, 201, "{name}: {}", antwort.json);
        id
    }

    async fn abrufen(&self, methode: Method, pfad: &str, kopf: &[(&str, &str)]) -> Kopf {
        let mut anfrage = self
            .absender
            .anfrage(methode, format!("{}{pfad}", self.instanz.adresse));
        for (name, wert) in kopf {
            anfrage = anfrage.header(*name, *wert);
        }
        Kopf::von(anfrage.send().await.expect("Antwort"))
    }

    async fn loeschen(&self, ids: &[&str]) {
        let antwort = self
            .absender
            .anfrage(
                Method::POST,
                format!("{}api/anhaenge/loeschen", self.instanz.adresse),
            )
            .header("Content-Type", "application/json")
            .header("X-LocalchatA-Sitzung", &self.token)
            .body(json!({"ids": ids, "bestaetigt": true}).to_string())
            .send()
            .await
            .expect("Löschen");
        assert_eq!(antwort.status(), 200);
    }
}

/// Status und die Kopfzeilen, um die es geht.
#[derive(Debug, PartialEq, Eq)]
struct Kopf {
    status: u16,
    art: String,
    disposition: String,
    csp: String,
    corp: String,
    nosniff: String,
    frame: String,
    referrer: String,
    cors: Vec<String>,
    coep: Option<String>,
}

impl Kopf {
    fn von(antwort: reqwest::Response) -> Kopf {
        let wert = |name: &str| {
            antwort
                .headers()
                .get(name)
                .map(|wert| wert.to_str().unwrap_or("?").to_string())
        };
        Kopf {
            status: antwort.status().as_u16(),
            art: wert("content-type").unwrap_or_default(),
            disposition: wert("content-disposition").unwrap_or_default(),
            csp: wert("content-security-policy").unwrap_or_default(),
            corp: wert("cross-origin-resource-policy").unwrap_or_default(),
            nosniff: wert("x-content-type-options").unwrap_or_default(),
            frame: wert("x-frame-options").unwrap_or_default(),
            referrer: wert("referrer-policy").unwrap_or_default(),
            cors: antwort
                .headers()
                .keys()
                .filter(|name| name.as_str().starts_with("access-control-"))
                .map(|name| name.to_string())
                .collect(),
            coep: wert("cross-origin-embedder-policy"),
        }
    }

    /// Medien-CSP genau für eine Medienart, sonst Anhang-CSP; CORP `same-origin` und der Rest des Satzes in jedem
    /// Fall (Regeln 1–4, B4, B5).
    fn pruefen(&self, medium: bool, fall: &str) {
        if medium {
            assert_eq!(self.csp, ANHANG_CSP_MEDIUM, "{fall}");
        } else {
            assert_eq!(self.csp, ANHANG_CSP, "{fall}");
        }
        assert_eq!(self.corp, "same-origin", "{fall}");
        assert_eq!(self.nosniff, "nosniff", "{fall}");
        assert_eq!(self.frame, "DENY", "{fall}");
        assert_eq!(self.referrer, "no-referrer", "{fall}");
        assert_eq!(self.cors, Vec::<String>::new(), "{fall}");
        assert_eq!(self.coep, None, "{fall}");
    }
}

/// M-4, S1: je Art der Tabelle und je Status 200, 206, 304, 416, `HEAD` → Medien-CSP genau bei den fünf
/// Medienarten; 404 (unbekannte ID) und 410 (gelöscht, auch für Medienarten) → `ANHANG_CSP`; CORP überall
/// `same-origin`. `video/ogg` (Theora) wird geladen: `attachment`, `ANHANG_CSP`.
#[tokio::test]
async fn kopfzeilen_je_art_und_status() {
    let bereit = Bereit::neu().await;
    let mut abgelegt = Vec::new();
    for (nummer, (name, inhalt, art)) in je_art().into_iter().enumerate() {
        let laenge = inhalt.len();
        abgelegt.push((bereit.senden(nummer, name, inhalt).await, art, laenge));
    }

    for (id, art, laenge) in &abgelegt {
        let pfad = format!("anhaenge/{id}");
        let etag = format!("\"{id}\"");
        let zu_weit = format!("bytes={laenge}-");
        let faelle = [
            (Method::GET, vec![], 200),
            (Method::GET, vec![("range", "bytes=0-3")], 206),
            (Method::GET, vec![("if-none-match", etag.as_str())], 304),
            (Method::GET, vec![("range", zu_weit.as_str())], 416),
            (Method::HEAD, vec![], 200),
        ];
        for (methode, kopf, status) in faelle {
            let antwort = bereit.abrufen(methode.clone(), &pfad, &kopf).await;
            let fall = format!("{art} {methode} {kopf:?}");
            assert_eq!(antwort.status, status, "{fall}");
            if status == 200 || status == 206 {
                assert_eq!(antwort.art, *art, "{fall}");
            }
            if *art == "video/ogg" && status == 200 {
                assert!(
                    antwort.disposition.starts_with("attachment;"),
                    "{fall}: {}",
                    antwort.disposition
                );
            }
            antwort.pruefen(ist_medienart(art), &fall);
        }
    }
    let medienarten = abgelegt.iter().filter(|(_, art, _)| ist_medienart(art));
    assert_eq!(medienarten.clone().count(), 5);

    let unbekannt = bereit
        .abrufen(Method::GET, "anhaenge/abcdefghijklmnopqrstuv", &[])
        .await;
    assert_eq!(unbekannt.status, 404);
    unbekannt.pruefen(false, "404 unbekannt");
    let ids: Vec<&str> = abgelegt.iter().map(|(id, _, _)| id.as_str()).collect();
    bereit.loeschen(&ids).await;
    for (id, art, _) in &abgelegt {
        for methode in [Method::GET, Method::HEAD] {
            let antwort = bereit
                .abrufen(methode.clone(), &format!("anhaenge/{id}"), &[])
                .await;
            let fall = format!("410 {art} {methode}");
            assert_eq!(antwort.status, 410, "{fall}");
            antwort.pruefen(false, &fall);
        }
    }
}

/// M-5: eine Anfrage mit `Content-Type: video/mp4`, mit dem Namen `x.mp4` (beim Hochladen und in der Adresse), mit
/// `X-Medium: 1` oder `Accept: video/mp4` auf eine PDF-, Text- und Bilddatei → `ANHANG_CSP` und `same-origin`.
/// Die `video/ogg`-Datei bleibt bei `ANHANG_CSP`, auch wenn die Anfrage sich als Medienabruf ausgibt.
#[tokio::test]
async fn der_marker_folgt_nie_der_anfrage() {
    let bereit = Bereit::neu().await;
    let ids = [
        bereit.senden(0, "x.mp4", dateien::pdf()).await,
        bereit.senden(1, "x.mp4", b"Hallo, Welt".to_vec()).await,
        bereit.senden(2, "x.mp4", dateien::png(4, 3)).await,
        bereit
            .senden(3, "x.mp4", dateien::ogg(b"\x80theora\x03"))
            .await,
    ];

    for id in &ids {
        for (pfad, kopf) in [
            (format!("anhaenge/{id}"), vec![]),
            (
                format!("anhaenge/{id}"),
                vec![("content-type", "video/mp4")],
            ),
            (format!("anhaenge/{id}"), vec![("x-medium", "1")]),
            (format!("anhaenge/{id}"), vec![("accept", "video/mp4")]),
            (format!("anhaenge/{id}?name=x.mp4"), vec![]),
            (
                format!("anhaenge/{id}"),
                vec![("range", "bytes=0-3"), ("content-type", "audio/mpeg")],
            ),
        ] {
            for methode in [Method::GET, Method::HEAD] {
                let antwort = bereit.abrufen(methode.clone(), &pfad, &kopf).await;
                let fall = format!("{methode} {pfad} {kopf:?}");
                assert!(antwort.status == 200 || antwort.status == 206, "{fall}");
                assert!(!ist_medienart(&antwort.art), "{fall}: {}", antwort.art);
                antwort.pruefen(false, &fall);
            }
        }
    }
}

/// S5: ein Medienabruf, wie ihn ein sandboxed Medien-Dokument oder eine fremde Seite stellt (`Origin: null` bzw.
/// `Origin: http://evil.example` bei gültigem `Host`, `Sec-Fetch-Mode: cors`, `Sec-Fetch-Dest: video`), bekommt keine
/// `Access-Control-*`-Kopfzeile und CORP `same-origin` — gespiegelt wird nichts.
#[tokio::test]
async fn kein_access_control_auf_medienabrufe() {
    let bereit = Bereit::neu().await;
    let ids = [
        bereit.senden(0, "film.webm", dateien::webm()).await,
        bereit.senden(1, "ton.mp3", dateien::mp3()).await,
    ];

    for id in &ids {
        for origin in ["null", "http://evil.example"] {
            for bereich in [None, Some("bytes=0-3")] {
                let mut kopf = vec![
                    ("origin", origin),
                    ("sec-fetch-mode", "cors"),
                    ("sec-fetch-dest", "video"),
                    ("sec-fetch-site", "cross-site"),
                ];
                kopf.extend(bereich.map(|bereich| ("range", bereich)));
                for methode in [Method::GET, Method::HEAD] {
                    let antwort = bereit
                        .abrufen(methode.clone(), &format!("anhaenge/{id}"), &kopf)
                        .await;
                    let fall = format!("{methode} {id} {kopf:?}");
                    assert!(antwort.status == 200 || antwort.status == 206, "{fall}");
                    antwort.pruefen(true, &fall);
                }
            }
        }
    }
}
