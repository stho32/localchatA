//! Angriffsmatrix ADR-0045 (Karte #35), Linie L2: jede Zeile über das echte Programm auf allen acht Endpunkten
//! E1–E8, mit dem Token einer Kontrollsitzung derselben Herkunft auf E3–E7. Erwartet wird OK, ABW-T, ABW-H oder
//! CSRF wie im Abschnitt „Erwartete Antworten“; bei ABW und CSRF zusätzlich „keine Wirkung“ (Sitzungen,
//! Nachrichten, Frist unverändert) und, wo die Zeile „V nein“ sagt, keine Vorwärtsfrage (Stufe debug, E6).
//! Zeilen nur der Linie L1 stehen als Unit-Tests in `src/http/{hostnamen,auskunft,abweisungsseite}.rs`.
#![cfg(unix)]

mod gemeinsam;

use gemeinsam::protokoll::{mit, wert, zeilen};
use gemeinsam::{Instanz, starten_mit};
use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::time::Duration;

const JETZT: &str = "2026-10-29T12:00:00+01:00";
const HEUTE: &str = "2026-10-29";

/// Testnamen der Matrix (lokale Adresse 127.0.0.1).
const NAMEN_LOOPBACK: &str = concat!(
    "boese.example=127.0.0.1;rebind.boese.example=127.0.0.1;localtest.me=127.0.0.1;127.0.0.1.nip.io=127.0.0.1;",
    "chat.lan=127.0.0.1;x.local=127.0.0.1;x.home.arpa=127.0.0.1;x.internal=127.0.0.1;chat=127.0.0.1;",
    "drucker.local=192.0.2.1;mix.local=192.0.2.1,127.0.0.1;null.local=0.0.0.0;null6.local=::;",
    "mc.local=224.0.0.251;bc.local=255.255.255.255;boese-local=192.0.2.1;",
    "local.boese.example=127.0.0.1;nuc5.local.boese.example=127.0.0.1;boese.local.example=127.0.0.1;",
    "x_y.local=127.0.0.1;xn--bcher-kva.local=127.0.0.1",
);
/// Testnamen für den Aufbau `--host 0.0.0.0`: zwei eigene Adressen mit verschiedenem Rückwärtsnamen.
const NAMEN_ZWEI_ADRESSEN: &str = "127.0.0.2>chat.fritz.box;127.0.0.1>localhost;boese.example=127.0.0.2;andere.fritz.box=127.0.0.2";

#[derive(Clone, Copy, Debug, PartialEq)]
enum Erwartet {
    Ok,
    AbwT,
    AbwH,
    Csrf,
}

/// Was auf den schreibenden Endpunkten E2, E5, E6 als `Origin` und `Sec-Fetch-Site` kommt.
#[derive(Clone, Copy)]
enum Ursprung<'a> {
    /// `Origin: http://<Host wörtlich>`, `Sec-Fetch-Site: same-origin` — die Lage nach einem Rebinding.
    Standard,
    /// Weder `Origin` noch `Sec-Fetch-Site` (Agent).
    Keiner,
    /// `Origin` wie angegeben, `Sec-Fetch-Site: same-origin`.
    Origin(&'a str),
    /// Eigener Origin mit diesem `Sec-Fetch-Site`.
    Site(&'a str),
}

struct Lauf {
    instanz: Instanz,
    _wurzel: tempfile::TempDir,
    ueber: IpAddr,
    token: String,
    namen: usize,
}

fn starten(argumente: &[&str], testnamen: &str, ueber: &str) -> Lauf {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let mut alle = vec!["--port", "0", "--kein-browser"];
    if !argumente.contains(&"--host") {
        alle.extend(["--host", "127.0.0.1"]);
    }
    alle.extend(argumente);
    let instanz = starten_mit(
        wurzel.path(),
        &alle,
        &[
            ("LOCALCHATA_JETZT", JETZT),
            ("TZ", "Europe/Berlin"),
            ("LOCALCHATA_TESTNAMEN", testnamen),
            ("LOCALCHATA_PROTOKOLL_STUFE", "debug"),
        ],
    );
    let mut lauf = Lauf {
        instanz,
        _wurzel: wurzel,
        ueber: ueber.parse().expect("Adresse"),
        token: String::new(),
        namen: 0,
    };
    lauf.kontrolle_einrichten();
    lauf
}

/// Antwort mit Status, Kopf (klein) und Inhalt; bei `text/event-stream` nur der Kopf.
struct Antwort {
    status: u16,
    kopf: Vec<(String, String)>,
    inhalt: String,
}

impl Antwort {
    fn header(&self, name: &str) -> Option<&str> {
        let werte: Vec<&str> = self
            .kopf
            .iter()
            .filter(|(kopfname, _)| kopfname == name)
            .map(|(_, wert)| wert.as_str())
            .collect();
        assert!(werte.len() <= 1, "{name} mehrfach: {werte:?}");
        werte.first().copied()
    }
}

/// Roh über TCP; liest bis zum Ende — beim Ereignisstrom nur bis zum Ende des Kopfs.
fn roh(ziel: SocketAddr, kopfzeilen: &[String], koerper: &str) -> Antwort {
    let mut verbindung = TcpStream::connect(ziel).expect("Verbindung");
    verbindung
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("Lesegrenze");
    let text = format!(
        "{}\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{koerper}",
        kopfzeilen.join("\r\n"),
        koerper.len()
    );
    verbindung.write_all(text.as_bytes()).expect("Anfrage");
    let mut roh = Vec::new();
    let mut puffer = [0u8; 4096];
    loop {
        let gelesen = verbindung.read(&mut puffer).expect("Antwort");
        if gelesen == 0 {
            break;
        }
        roh.extend_from_slice(&puffer[..gelesen]);
        let text = String::from_utf8_lossy(&roh).to_ascii_lowercase();
        if let Some(ende) = text.find("\r\n\r\n")
            && text[..ende].contains("content-type: text/event-stream")
        {
            break;
        }
    }
    let roh = String::from_utf8_lossy(&roh).into_owned();
    let (kopfteil, inhalt) = roh.split_once("\r\n\r\n").expect("Kopf und Inhalt");
    let mut zeilen = kopfteil.split("\r\n");
    let status = zeilen
        .next()
        .and_then(|zeile| zeile.split(' ').nth(1))
        .and_then(|code| code.parse().ok())
        .unwrap_or_else(|| panic!("keine Statuszeile: {roh:?}"));
    let kopf = zeilen
        .filter_map(|zeile| zeile.split_once(':'))
        .map(|(name, wert)| (name.trim().to_lowercase(), wert.trim().to_string()))
        .collect();
    Antwort {
        status,
        kopf,
        inhalt: inhalt.to_string(),
    }
}

const SITZUNG: &str = "X-LocalchatA-Sitzung";

impl Lauf {
    fn port(&self) -> u16 {
        self.instanz.port()
    }

    fn ziel(&self) -> SocketAddr {
        SocketAddr::new(self.ueber, self.port())
    }

    fn ip_host(&self) -> String {
        SocketAddr::new(self.ueber, self.port()).to_string()
    }

    fn eigene_adresse(&self) -> String {
        format!("http://{}/", self.ip_host())
    }

    fn anfrage(&self, zeile: &str, host: &str, kopf: &[String], koerper: &str) -> Antwort {
        let mut alle = vec![format!("{zeile} HTTP/1.1"), format!("Host: {host}")];
        alle.extend(kopf.iter().cloned());
        roh(self.ziel(), &alle, koerper)
    }

    /// Kontrollsitzung derselben Herkunft über das IP-Literal und der Channel `srev`.
    fn kontrolle_einrichten(&mut self) {
        let host = self.ip_host();
        let json = "Content-Type: application/json".to_string();
        let beitritt = self.anfrage(
            "POST /api/beitreten",
            &host,
            std::slice::from_ref(&json),
            r#"{"name":"kontrolle"}"#,
        );
        assert_eq!(beitritt.status, 201, "{}", beitritt.inhalt);
        let wert: serde_json::Value = serde_json::from_str(&beitritt.inhalt).expect("JSON");
        self.token = wert["token"].as_str().expect("Token").to_string();
        let sitzung = format!("{SITZUNG}: {}", self.token);
        let channel = self.anfrage(
            "POST /api/channels",
            &host,
            &[json, sitzung],
            r#"{"name":"srev"}"#,
        );
        assert_eq!(channel.status, 201, "{}", channel.inhalt);
    }

    /// Eine neue Sitzung derselben Herkunft über das IP-Literal, nur für E6 einer Zeile.
    fn frist_token(&self) -> String {
        let beitritt = self.anfrage(
            "POST /api/beitreten",
            &self.ip_host(),
            &["Content-Type: application/json".to_string()],
            &format!(r#"{{"name":"f{}"}}"#, self.namen),
        );
        assert_eq!(beitritt.status, 201, "{}", beitritt.inhalt);
        let wert: serde_json::Value = serde_json::from_str(&beitritt.inhalt).expect("JSON");
        wert["token"].as_str().expect("Token").to_string()
    }

    /// Sitzungen, Nachrichten und die Liste der Channels (mit Frist) — „keine Wirkung“ heißt: gleich.
    fn stand(&self) -> (String, String, String) {
        let host = self.ip_host();
        let gesundheit = self.anfrage("GET /api/gesundheit", &host, &[], "");
        let wert: serde_json::Value = serde_json::from_str(&gesundheit.inhalt).expect("JSON");
        let sitzung = format!("{SITZUNG}: {}", self.token);
        let channels = self.anfrage("GET /api/channels", &host, &[sitzung], "");
        (
            wert["sitzungen"].to_string(),
            wert["nachrichten"].to_string(),
            channels.inhalt,
        )
    }

    fn vorwaerts_gefragt(&self, name: &str) -> usize {
        self.gefragt("vorwaerts", name)
    }

    fn rueckwaerts_gefragt(&self, adresse: &str) -> usize {
        self.gefragt("rueckwaerts", adresse)
    }

    fn gefragt(&self, art: &str, schluessel: &str) -> usize {
        mit(
            &zeilen(self.instanz.ordner.as_path(), HEUTE),
            "DEBUG",
            "grenzschutz",
        )
        .iter()
        .filter(|zeile| zeile.contains("Namensauskunft gefragt"))
        .filter(|zeile| wert(zeile, "art").as_deref() == Some(art))
        .filter(|zeile| wert(zeile, "schluessel").as_deref() == Some(schluessel))
        .count()
    }

    /// Eine Zeile der Matrix auf E1–E8.
    fn zeile(
        &mut self,
        nummer: &str,
        host: &str,
        ursprung: Ursprung,
        extra: &[&str],
        erwartet: Erwartet,
    ) {
        let vorher = self.stand();
        self.namen += 1;
        let sitzung = format!("{SITZUNG}: {}", self.token);
        // Eine Friständerung kostet 10 von 30 Einheiten der Kennung (R00021): wo E6 durchgeht, setzt eine eigene
        // Sitzung je Zeile die Frist, damit die Kontrollsitzung nicht nach drei Zeilen gebremst wird.
        let frist_sitzung = if erwartet == Erwartet::Ok {
            format!("{SITZUNG}: {}", self.frist_token())
        } else {
            sitzung.clone()
        };
        let json = "Content-Type: application/json".to_string();
        let schreibend: Vec<String> = match ursprung {
            Ursprung::Standard => vec![
                format!("Origin: http://{host}"),
                "Sec-Fetch-Site: same-origin".into(),
            ],
            Ursprung::Keiner => vec![],
            Ursprung::Origin(origin) => {
                vec![
                    format!("Origin: {origin}"),
                    "Sec-Fetch-Site: same-origin".into(),
                ]
            }
            Ursprung::Site(site) => vec![
                format!("Origin: http://{host}"),
                format!("Sec-Fetch-Site: {site}"),
            ],
        };
        let mit = |grund: &[String]| -> Vec<String> {
            let mut alle: Vec<String> = grund.to_vec();
            alle.extend(extra.iter().map(|zeile| zeile.to_string()));
            alle
        };
        let lesend_mit_token = mit(std::slice::from_ref(&sitzung));
        let mut schreibend_mit_token = schreibend.clone();
        schreibend_mit_token.extend([json.clone(), sitzung.clone()]);
        let mut frist_kopf = schreibend.clone();
        frist_kopf.extend([json.clone(), frist_sitzung]);
        let mut beitreten_kopf = schreibend.clone();
        beitreten_kopf.push(json.clone());
        let name = format!(r#"{{"name":"m{}"}}"#, self.namen);
        let endpunkte: Vec<(&str, &str, Vec<String>, String, u16)> = vec![
            ("E1", "GET /", mit(&[]), String::new(), 200),
            ("E2", "POST /api/beitreten", mit(&beitreten_kopf), name, 201),
            (
                "E3",
                "GET /api/channels",
                lesend_mit_token.clone(),
                String::new(),
                200,
            ),
            (
                "E4",
                "GET /api/channels/srev/nachrichten",
                lesend_mit_token.clone(),
                String::new(),
                200,
            ),
            (
                "E5",
                "POST /api/channels/srev/nachrichten",
                mit(&schreibend_mit_token),
                format!(r#"{{"text":"Zeile {nummer}"}}"#),
                201,
            ),
            (
                "E6",
                "PUT /api/channels/srev/frist",
                mit(&frist_kopf),
                r#"{"tage":30}"#.to_string(),
                200,
            ),
            (
                "E7",
                "GET /api/ereignisse",
                lesend_mit_token,
                String::new(),
                200,
            ),
            ("E8", "GET /api/gesundheit", mit(&[]), String::new(), 200),
        ];
        for (endpunkt, anfrage, kopf, koerper, ok) in endpunkte {
            let antwort = self.anfrage(anfrage, host, &kopf, &koerper);
            let ort = format!("{nummer} {endpunkt} Host {host:?}");
            let schreibt = ["E2", "E5", "E6"].contains(&endpunkt);
            match erwartet {
                Erwartet::Ok => {
                    assert_eq!(antwort.status, ok, "{ort}: {}", antwort.inhalt);
                    if endpunkt == "E7" {
                        assert_eq!(
                            antwort.header("content-type"),
                            Some("text/event-stream"),
                            "{ort}"
                        );
                    }
                }
                Erwartet::Csrf if !schreibt => {
                    assert_eq!(antwort.status, ok, "{ort}: {}", antwort.inhalt);
                }
                Erwartet::Csrf => {
                    assert_eq!(antwort.status, 403, "{ort}: {}", antwort.inhalt);
                    assert_eq!(
                        antwort.header("content-type"),
                        Some("application/json"),
                        "{ort}"
                    );
                    assert!(
                        antwort.inhalt.contains("\"fehler\""),
                        "{ort}: {}",
                        antwort.inhalt
                    );
                }
                Erwartet::AbwT | Erwartet::AbwH => {
                    self.abgewiesen(&antwort, &ort, anfrage, erwartet)
                }
            }
            if erwartet != Erwartet::Ok {
                assert!(
                    !antwort
                        .kopf
                        .iter()
                        .any(|(kopf, _)| kopf.starts_with("access-control-allow-")),
                    "{ort}"
                );
            }
        }
        if erwartet != Erwartet::Ok {
            assert_eq!(self.stand(), vorher, "{nummer}: keine Wirkung");
        }
    }

    fn abgewiesen(&self, antwort: &Antwort, ort: &str, anfrage: &str, erwartet: Erwartet) {
        assert_eq!(antwort.status, 403, "{ort}: {}", antwort.inhalt);
        let typ = if erwartet == Erwartet::AbwH {
            "text/html; charset=utf-8"
        } else {
            "text/plain; charset=utf-8"
        };
        assert_eq!(antwort.header("content-type"), Some(typ), "{ort}");
        assert!(
            antwort.inhalt.contains(&self.eigene_adresse()),
            "{ort}: {}",
            antwort.inhalt
        );
        assert_eq!(
            antwort.header("x-content-type-options"),
            Some("nosniff"),
            "{ort}"
        );
        assert_eq!(antwort.header("x-frame-options"), Some("DENY"), "{ort}");
        assert!(antwort.header("content-security-policy").is_some(), "{ort}");
        let unter_api = anfrage.contains(" /api/");
        assert_eq!(
            antwort.header("cache-control") == Some("no-store"),
            unter_api,
            "{ort}"
        );
    }
}

// --- K: Kontrollen ---------------------------------------------------------------------------------------

#[test]
fn k1_k2_k3_k5_k7_kontrollen_ueber_loopback() {
    let mut lauf = starten(&[], NAMEN_LOOPBACK, "127.0.0.1");
    let p = lauf.port();
    let rechner = rechnername();
    for (nummer, host) in [
        ("K1", format!("127.0.0.1:{p}")),
        ("K2", format!("localhost:{p}")),
        ("K3", format!("{rechner}:{p}")),
        ("K3", format!("{rechner}.local:{p}")),
        ("K3", format!("{}.LOCAL:{p}", rechner.to_uppercase())),
        ("K5", format!("[::ffff:127.0.0.1]:{p}")),
    ] {
        lauf.zeile(nummer, &host, Ursprung::Standard, &[], Erwartet::Ok);
    }
    lauf.zeile(
        "K7",
        &format!("127.0.0.1:{p}"),
        Ursprung::Keiner,
        &[],
        Erwartet::Ok,
    );
    // Kontrollpaar: A1 auf derselben Instanz bleibt abgewiesen.
    lauf.zeile(
        "A1",
        &format!("boese.example:{p}"),
        Ursprung::Standard,
        &[],
        Erwartet::AbwT,
    );
    assert_eq!(lauf.vorwaerts_gefragt("boese.example"), 0);
}

#[test]
fn k4_a3_ipv6() {
    let mut lauf = starten(&["--host", "::"], "boese.example=::1", "::1");
    let p = lauf.port();
    lauf.zeile(
        "K4",
        &format!("[::1]:{p}"),
        Ursprung::Standard,
        &[],
        Erwartet::Ok,
    );
    lauf.zeile(
        "A3",
        &format!("boese.example:{p}"),
        Ursprung::Standard,
        &[],
        Erwartet::AbwT,
    );
    assert_eq!(lauf.vorwaerts_gefragt("boese.example"), 0);
}

#[test]
fn k6_erreichbar_als() {
    let mut lauf = starten(
        &["--erreichbar-als", "chat.lan"],
        NAMEN_LOOPBACK,
        "127.0.0.1",
    );
    let p = lauf.port();
    lauf.zeile(
        "K6",
        &format!("chat.lan:{p}"),
        Ursprung::Standard,
        &[],
        Erwartet::Ok,
    );
}

// --- A: Rebinding über einen öffentlichen Namen ----------------------------------------------------------

#[test]
fn a1_a4_a5_a8_oeffentliche_namen_nie_erlaubt_nie_vorwaerts() {
    let mut lauf = starten(&[], NAMEN_LOOPBACK, "127.0.0.1");
    let p = lauf.port();
    for (nummer, name) in [
        ("A1", "boese.example"),
        ("A4", "rebind.boese.example"),
        ("A4", "localtest.me"),
        ("A4", "127.0.0.1.nip.io"),
        ("A8", "chat.lan"),
    ] {
        lauf.zeile(
            nummer,
            &format!("{name}:{p}"),
            Ursprung::Standard,
            &[],
            Erwartet::AbwT,
        );
        assert_eq!(lauf.vorwaerts_gefragt(name), 0, "{nummer} {name}");
    }
    lauf.zeile(
        "A5",
        &format!("boese.example:{p}"),
        Ursprung::Standard,
        &["Accept: text/html", "Sec-Fetch-Dest: document"],
        Erwartet::AbwH,
    );
}

/// Aufbau `--host 0.0.0.0`: über 127.0.0.2 (Rückwärtsname `chat.fritz.box`) und 127.0.0.1 (`localhost`).
#[test]
fn a2_a6_a7_b1_b3_b4_b5_j1_rueckwaertsname_ueber_zweite_adresse() {
    let mut lauf = starten(&["--host", "0.0.0.0"], NAMEN_ZWEI_ADRESSEN, "127.0.0.2");
    let p = lauf.port();
    lauf.zeile(
        "B1",
        &format!("chat.fritz.box:{p}"),
        Ursprung::Standard,
        &[],
        Erwartet::Ok,
    );
    lauf.zeile(
        "B3",
        &format!("Chat.Fritz.Box.:{p}"),
        Ursprung::Standard,
        &[],
        Erwartet::Ok,
    );
    for (nummer, name) in [
        ("A2", "boese.example"),
        ("A6", "andere.fritz.box"),
        ("A7", "fritz.box"),
        ("B4", "chat.fritz.box.boese.example"),
        ("B4", "chat.fritz"),
    ] {
        lauf.zeile(
            nummer,
            &format!("{name}:{p}"),
            Ursprung::Standard,
            &[],
            Erwartet::AbwT,
        );
        assert_eq!(lauf.vorwaerts_gefragt(name), 0, "{nummer} {name}");
    }
    lauf.zeile(
        "J1",
        &format!("chat.fritz.box:{p}"),
        Ursprung::Origin(&format!("http://boese.example:{p}")),
        &[],
        Erwartet::Csrf,
    );
    let vorflug = lauf.anfrage(
        "OPTIONS /api/beitreten",
        &format!("chat.fritz.box:{p}"),
        &[
            "Origin: http://boese.example".into(),
            "Access-Control-Request-Method: POST".into(),
        ],
        "",
    );
    assert_eq!(vorflug.status, 405, "J6");
    assert!(
        !vorflug
            .kopf
            .iter()
            .any(|(kopf, _)| kopf.starts_with("access-control-allow-")),
        "J6"
    );
}

#[test]
fn b2_derselbe_name_ueber_eine_andere_eigene_adresse() {
    let mut lauf = starten(&["--host", "0.0.0.0"], NAMEN_ZWEI_ADRESSEN, "127.0.0.1");
    let p = lauf.port();
    lauf.zeile(
        "B2",
        &format!("chat.fritz.box:{p}"),
        Ursprung::Standard,
        &[],
        Erwartet::AbwT,
    );
}

#[test]
fn b5_adresse_ohne_rueckwaertsnamen() {
    let mut lauf = starten(&["--host", "0.0.0.0"], NAMEN_ZWEI_ADRESSEN, "127.0.0.3");
    let p = lauf.port();
    lauf.zeile(
        "B5",
        &format!("chat.fritz.box:{p}"),
        Ursprung::Standard,
        &[],
        Erwartet::AbwT,
    );
    let rueckwaerts = mit(
        &zeilen(lauf.instanz.ordner.as_path(), HEUTE),
        "DEBUG",
        "grenzschutz",
    )
    .iter()
    .filter(|zeile| wert(zeile, "art").as_deref() == Some("rueckwaerts"))
    .filter(|zeile| wert(zeile, "schluessel").as_deref() == Some("127.0.0.3"))
    .count();
    assert_eq!(
        rueckwaerts, 1,
        "B5: zweite Anfrage innerhalb von 60 s ohne neue Frage"
    );
}

/// B6: Dual-Stack, die lokale Adresse kommt als `::ffff:127.0.0.2` und wird kanonisch.
#[test]
fn b6_dual_stack() {
    let mut lauf = starten(&["--host", "::"], NAMEN_ZWEI_ADRESSEN, "127.0.0.2");
    let p = lauf.port();
    lauf.zeile(
        "B6",
        &format!("chat.fritz.box:{p}"),
        Ursprung::Standard,
        &[],
        Erwartet::Ok,
    );
}

// --- C: Lokaler Namensraum -------------------------------------------------------------------------------

#[test]
fn c1_c7_c10_c13_lokaler_namensraum() {
    let mut lauf = starten(&[], NAMEN_LOOPBACK, "127.0.0.1");
    let p = lauf.port();
    for (nummer, host) in [
        ("C1", format!("x.local:{p}")),
        ("C2", format!("x.local.:{p}")),
        ("C2", format!("X.LOCAL:{p}")),
        ("C3", format!("x.home.arpa:{p}")),
        ("C3", format!("x.internal:{p}")),
        ("C3", format!("chat:{p}")),
        ("C5", format!("mix.local:{p}")),
        ("D4", format!("xn--bcher-kva.local:{p}")),
    ] {
        lauf.zeile(nummer, &host, Ursprung::Standard, &[], Erwartet::Ok);
    }
    assert_eq!(
        lauf.vorwaerts_gefragt("x.local"),
        1,
        "C2: ein Cache-Eintrag für alle Schreibweisen"
    );
    for (nummer, name) in [
        ("C4", "drucker.local"),
        ("C6", "null.local"),
        ("C6", "null6.local"),
        ("C7", "mc.local"),
        ("C7", "bc.local"),
        ("C10", "gibtsnicht.local"),
        ("C11", "boese-local"),
    ] {
        lauf.zeile(
            nummer,
            &format!("{name}:{p}"),
            Ursprung::Standard,
            &[],
            Erwartet::AbwT,
        );
        lauf.zeile(
            nummer,
            &format!("{name}:{p}"),
            Ursprung::Standard,
            &[],
            Erwartet::AbwT,
        );
        assert_eq!(
            lauf.vorwaerts_gefragt(name),
            1,
            "{nummer} {name}: einmal, dann Cache"
        );
    }
    // Nacharbeit #35 (ADR-0045 Ergänzung Punkt 1): in den C-Zeilen gilt „R nein“ — ein Name im lokalen
    // Namensraum wird nur vorwärts geprüft. C12 und D1 liegen außerhalb und fragen danach rückwärts („R: C“).
    assert_eq!(lauf.rueckwaerts_gefragt("127.0.0.1"), 0, "C1–C11: R nein");
    for (nummer, name) in [
        ("C12", "x.localhost"),
        ("C13", "2130706433"),
        ("C13", "0x7f000001"),
        ("D1", "local.boese.example"),
        ("D1", "nuc5.local.boese.example"),
        ("D1", "boese.local.example"),
    ] {
        lauf.zeile(
            nummer,
            &format!("{name}:{p}"),
            Ursprung::Standard,
            &[],
            Erwartet::AbwT,
        );
        assert_eq!(lauf.vorwaerts_gefragt(name), 0, "{nummer} {name}");
    }
}

// --- D: Normalform ---------------------------------------------------------------------------------------

#[test]
fn d2_d3_d6_d8_normalform_ohne_aufloesung() {
    let mut lauf = starten(&[], NAMEN_LOOPBACK, "127.0.0.1");
    let p = lauf.port();
    let lang = format!("{}.local", "a".repeat(64));
    for (nummer, host) in [
        ("D2", "x_y.local".to_string()),
        ("D2", "x%2elocal".to_string()),
        ("D2", "a@nuc5.local".to_string()),
        ("D2", "nuc5 .local".to_string()),
        ("D2", "nuc5\t.local".to_string()),
        ("D2", "<img src=x onerror=alert(1)>".to_string()),
        ("D3", "bücher.local".to_string()),
        ("D6", format!("{lang}:{p}")),
        ("D8", format!("a..local:{p}")),
        ("D8", format!(".local:{p}")),
        ("D8", format!("nuc5.local..:{p}")),
        ("D8", format!(".:{p}")),
    ] {
        lauf.zeile(nummer, &host, Ursprung::Standard, &[], Erwartet::AbwT);
    }
    for name in ["x_y.local", "bücher.local", lang.as_str()] {
        assert_eq!(lauf.vorwaerts_gefragt(name), 0, "{name}");
    }
}

#[test]
fn d11_d12_punkt_am_ende_origin_woertlich() {
    let mut lauf = starten(&[], NAMEN_LOOPBACK, "127.0.0.1");
    let p = lauf.port();
    let rechner = rechnername();
    let host = format!("{rechner}.local.:{p}");
    lauf.zeile(
        "D11",
        &host,
        Ursprung::Origin(&format!("http://{host}")),
        &[],
        Erwartet::Ok,
    );
    lauf.zeile(
        "D12",
        &host,
        Ursprung::Origin(&format!("http://{rechner}.local:{p}")),
        &[],
        Erwartet::Csrf,
    );
}

// --- E: IPv6-Literal und Zone ----------------------------------------------------------------------------

#[test]
fn e1_zone_im_literal() {
    let mut lauf = starten(&[], NAMEN_LOOPBACK, "127.0.0.1");
    let p = lauf.port();
    for host in [
        format!("[fe80::1%25eth0]:{p}"),
        format!("[fe80::1%eth0]:{p}"),
    ] {
        lauf.zeile("E1", &host, Ursprung::Standard, &[], Erwartet::AbwT);
    }
}

// --- J: Origin-Regeln unter automatisch erlaubten Namen --------------------------------------------------

#[test]
fn j2_j5_origin_regeln_unter_einem_lokalen_namen() {
    let mut lauf = starten(&[], NAMEN_LOOPBACK, "127.0.0.1");
    let p = lauf.port();
    let host = format!("x.local:{p}");
    lauf.zeile("J2", &host, Ursprung::Origin("null"), &[], Erwartet::Csrf);
    lauf.zeile(
        "J3",
        &host,
        Ursprung::Origin(&format!("http://localhost:{p}")),
        &[],
        Erwartet::Csrf,
    );
    lauf.zeile(
        "J3",
        &host,
        Ursprung::Origin(&format!("http://chat.fritz.box:{p}")),
        &[],
        Erwartet::Csrf,
    );
    lauf.zeile(
        "J4",
        &host,
        Ursprung::Site("same-site"),
        &[],
        Erwartet::Csrf,
    );
    lauf.zeile(
        "J4",
        &host,
        Ursprung::Site("cross-site"),
        &[],
        Erwartet::Csrf,
    );
    lauf.zeile("J5", &host, Ursprung::Keiner, &[], Erwartet::Ok);
}

// --- I: --alle-namen ---------------------------------------------------------------------------------------

#[test]
fn i1_i4_alle_namen_rebinding_offen_csrf_zu() {
    let mut lauf = starten(&["--alle-namen"], NAMEN_LOOPBACK, "127.0.0.1");
    let p = lauf.port();
    for name in ["boese.example", "chat.lan", "x.local"] {
        lauf.zeile(
            "I1",
            &format!("{name}:{p}"),
            Ursprung::Standard,
            &[],
            Erwartet::Ok,
        );
    }
    let gefragt = mit(
        &zeilen(lauf.instanz.ordner.as_path(), HEUTE),
        "DEBUG",
        "grenzschutz",
    )
    .iter()
    .filter(|zeile| zeile.contains("Namensauskunft gefragt"))
    .count();
    assert_eq!(gefragt, 0, "I1: V nein · R nein");
    lauf.zeile(
        "I2",
        &format!("boese.example:{p}"),
        Ursprung::Origin("http://andere.example"),
        &[],
        Erwartet::Csrf,
    );
    lauf.zeile(
        "I3",
        &format!("127.0.0.1:{p}"),
        Ursprung::Origin("http://boese.example"),
        &[],
        Erwartet::Csrf,
    );
    lauf.zeile(
        "I3",
        &format!("127.0.0.1:{p}"),
        Ursprung::Origin("null"),
        &[],
        Erwartet::Csrf,
    );
    lauf.zeile(
        "I4",
        &format!("boese.example:{p}"),
        Ursprung::Site("same-site"),
        &[],
        Erwartet::Csrf,
    );
    lauf.zeile(
        "I4",
        &format!("boese.example:{p}"),
        Ursprung::Site("cross-site"),
        &[],
        Erwartet::Csrf,
    );
    lauf.zeile("I5", "x_y.local", Ursprung::Standard, &[], Erwartet::AbwT);
    lauf.zeile(
        "I5",
        "bücher.local",
        Ursprung::Standard,
        &[],
        Erwartet::AbwT,
    );
    lauf.zeile(
        "I5",
        "x_y.local",
        Ursprung::Standard,
        &["Accept: text/html"],
        Erwartet::AbwH,
    );
}

fn rechnername() -> String {
    let ausgabe = std::process::Command::new("uname")
        .arg("-n")
        .output()
        .expect("uname -n");
    String::from_utf8(ausgabe.stdout)
        .expect("UTF-8")
        .trim_end()
        .to_string()
}
