//! R00024 Instanz über ihren Netznamen öffnen (ADR-0045) über das echte Programm, Namensauskunft über den
//! Testknopf `LOCALCHATA_TESTNAMEN` (Debug-Build): Kriterien je Feature und die Zeilen der Linie L2 aus dem
//! Abschnitt „Angriffsmatrix“. Anfragen gehen roh über TCP, damit `Host`, `Origin` und `Accept` genau so
//! ankommen, wie Kriterium und Zeile sie nennen.
#![cfg(unix)]

mod gemeinsam;

use gemeinsam::protokoll::{mit, wert, zeilen};
use gemeinsam::{Instanz, RohAntwort, aufrufen_mit, roh_anfragen, starten_mit};
use std::net::{IpAddr, SocketAddr};
use std::path::Path;

/// Testnamen aus den Akzeptanzkriterien von R00024.
const TESTNAMEN: &str = "127.0.0.1>nuc5.fritz.box;chat.local=127.0.0.1;drucker.local=192.0.2.1;boese.example=127.0.0.1;chat=127.0.0.1;nas.home.arpa=127.0.0.1;dienst.internal=127.0.0.1;chat.fritz.box=127.0.0.1";
/// Startzeit der Testuhr; die Tagesdatei des Protokolls heißt danach.
const JETZT: &str = "2026-10-29T12:00:00+01:00";
const HEUTE: &str = "2026-10-29";

struct Lauf {
    instanz: Instanz,
    _wurzel: tempfile::TempDir,
    /// Die Adresse, über die angefragt wird (lokale Adresse der Verbindung).
    ueber: IpAddr,
    /// stderr des Programms, zeilenweise mitgelesen.
    stderr: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
}

/// Startet das Programm mit `argumente` auf Port 0, Testuhr, `TZ=Europe/Berlin` und `umgebung`.
fn starten(argumente: &[&str], umgebung: &[(&str, &str)]) -> Lauf {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let mut alle = vec!["--port", "0", "--kein-browser"];
    if !argumente.contains(&"--host") {
        alle.extend(["--host", "127.0.0.1"]);
    }
    alle.extend(argumente);
    let mut umgebung_gesamt = vec![("LOCALCHATA_JETZT", JETZT), ("TZ", "Europe/Berlin")];
    umgebung_gesamt.extend(umgebung);
    let mut instanz = starten_mit(wurzel.path(), &alle, &umgebung_gesamt);
    let stderr = instanz.stderr_mitschreiben();
    Lauf {
        instanz,
        _wurzel: wurzel,
        ueber: IpAddr::from([127, 0, 0, 1]),
        stderr,
    }
}

/// Mit den Testnamen, Protokoll auf Stufe debug.
fn mit_testnamen(argumente: &[&str]) -> Lauf {
    starten(
        argumente,
        &[
            ("LOCALCHATA_TESTNAMEN", TESTNAMEN),
            ("LOCALCHATA_PROTOKOLL_STUFE", "debug"),
        ],
    )
}

impl Lauf {
    fn port(&self) -> u16 {
        self.instanz.port()
    }

    fn ueber(mut self, adresse: &str) -> Lauf {
        self.ueber = adresse.parse().expect("Adresse");
        self
    }

    /// Roh: `<zeile>` mit `Host: <host>` (ohne Port: `:<p>` wird angehängt, wenn `host` keinen hat) und `kopf`.
    fn anfragen(&self, zeile: &str, host: &str, kopf: &[&str], koerper: &str) -> RohAntwort {
        let mut anfrage = vec![format!("{zeile} HTTP/1.1"), format!("Host: {host}")];
        anfrage.extend(kopf.iter().map(|zeile| zeile.to_string()));
        roh_anfragen(
            SocketAddr::new(self.ueber, self.port()),
            &anfrage.join("\r\n"),
            koerper,
        )
    }

    /// `GET /api/gesundheit` mit dem Host-Namen `name:<p>`.
    fn gesundheit(&self, name: &str) -> RohAntwort {
        self.anfragen(
            "GET /api/gesundheit",
            &format!("{name}:{}", self.port()),
            &[],
            "",
        )
    }

    fn protokoll(&self) -> Vec<String> {
        zeilen(self.instanz.ordner.as_path(), HEUTE)
    }

    /// Zeilen „Namensauskunft gefragt“ mit `art` und `schluessel`.
    fn gefragt(&self, art: &str, schluessel: &str) -> usize {
        mit(&self.protokoll(), "DEBUG", "grenzschutz")
            .iter()
            .filter(|zeile| zeile.contains("Namensauskunft gefragt"))
            .filter(|zeile| wert(zeile, "art").as_deref() == Some(art))
            .filter(|zeile| wert(zeile, "schluessel").as_deref() == Some(schluessel))
            .count()
    }

    /// Zeilen `INFORMATION grenzschutz <text>` mit `name`.
    fn erlaubt_gemeldet(&self, text: &str, name: &str) -> Vec<String> {
        mit(&self.protokoll(), "INFORMATION", "grenzschutz")
            .into_iter()
            .filter(|zeile| zeile.contains(text))
            .filter(|zeile| wert(zeile, "name").as_deref() == Some(name))
            .collect()
    }
}

fn erlaubt(antwort: &RohAntwort) -> bool {
    antwort.status == 200 && antwort.inhalt.contains("\"status\":\"bereit\"")
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

fn exit_code(ordner: &Path, umgebung: &[(&str, &str)], argumente: &[&str]) -> gemeinsam::Ende {
    let ordner = ordner.to_str().expect("UTF-8");
    let mut alle = vec![
        ordner,
        "--port",
        "0",
        "--host",
        "127.0.0.1",
        "--kein-browser",
    ];
    alle.extend(argumente);
    aufrufen_mit(&alle, umgebung)
}

// ---------------------------------------------------------------------------------------------------------
// F0077 Über den Rückwärtsnamen öffnen
// ---------------------------------------------------------------------------------------------------------

/// Kriterium „Rückwärtsname über das echte Programm“, erlaubt (Matrix B1, B3 über Loopback).
#[test]
fn f0077_der_rueckwaertsname_oeffnet_in_jeder_schreibweise() {
    let lauf = mit_testnamen(&[]);

    for name in ["nuc5.fritz.box", "NUC5.Fritz.Box", "nuc5.fritz.box."] {
        let antwort = lauf.gesundheit(name);
        assert!(
            erlaubt(&antwort),
            "{name}: {} {}",
            antwort.status,
            antwort.inhalt
        );
    }
}

/// Kriterium „Rückwärtsname über das echte Programm“, abgewiesen und weiter erlaubt (Matrix A1, D2, K1, K2, K6).
#[test]
fn f0077_oeffentliche_und_kaputte_namen_bleiben_zu_feste_namen_offen() {
    let lauf = mit_testnamen(&["--erreichbar-als", "chat.firma.de"]);

    assert_eq!(lauf.gesundheit("boese.example").status, 403);
    assert_eq!(lauf.gesundheit("a_b.example").status, 403);
    for name in ["127.0.0.1", "localhost", "chat.firma.de"] {
        assert!(erlaubt(&lauf.gesundheit(name)), "{name}");
    }
}

/// US-3, Matrix A1 und H1 (E6, E8): ein öffentlicher Name auf die eigene Adresse wird nie vorwärts gefragt.
#[test]
fn f0077_ein_oeffentlicher_name_wird_nie_vorwaerts_gefragt() {
    let lauf = mit_testnamen(&[]);

    assert_eq!(lauf.gesundheit("boese.example").status, 403);
    assert_eq!(lauf.gesundheit("boese.example").status, 403);

    assert_eq!(lauf.gefragt("vorwaerts", "boese.example"), 0);
    assert!(lauf.erlaubt_gemeldet("erlaubt", "boese.example").is_empty());
    assert_eq!(lauf.gefragt("rueckwaerts", "127.0.0.1"), 1);
}

/// Kriterium „Gefragt / nicht gefragt“ und „Protokoll“ (Matrix B1): zweimal derselbe Name → eine Rückwärtsfrage,
/// eine Information mit Name und Adresse.
#[test]
fn f0077_zweimal_der_rueckwaertsname_eine_frage_eine_information() {
    let lauf = mit_testnamen(&[]);

    assert!(erlaubt(&lauf.gesundheit("nuc5.fritz.box")));
    assert!(erlaubt(&lauf.gesundheit("nuc5.fritz.box")));

    assert_eq!(lauf.gefragt("rueckwaerts", "127.0.0.1"), 1);
    let gemeldet = lauf.erlaubt_gemeldet(
        "Host-Name über Rückwärtsauflösung erlaubt",
        "nuc5.fritz.box",
    );
    assert_eq!(gemeldet.len(), 1, "{gemeldet:?}");
    assert_eq!(wert(&gemeldet[0], "adresse").as_deref(), Some("127.0.0.1"));
}

/// Matrix B1, B2 (L2): der Rückwärtsname gilt nur über die Adresse, zu der er gehört — `--host 0.0.0.0`, über
/// 127.0.0.2 erlaubt, über 127.0.0.1 (Rückwärtsname `localhost`) abgewiesen.
#[test]
fn b1_b2_der_rueckwaertsname_gilt_nur_ueber_seine_adresse() {
    let umgebung = [
        (
            "LOCALCHATA_TESTNAMEN",
            "127.0.0.2>chat.fritz.box;127.0.0.1>localhost",
        ),
        ("LOCALCHATA_PROTOKOLL_STUFE", "debug"),
    ];
    let zweite = starten(&["--host", "0.0.0.0"], &umgebung).ueber("127.0.0.2");
    assert!(erlaubt(&zweite.gesundheit("chat.fritz.box")));
    assert!(erlaubt(&zweite.gesundheit("chat.fritz.box")));
    assert_eq!(zweite.gefragt("rueckwaerts", "127.0.0.2"), 1);
    let gemeldet = zweite.erlaubt_gemeldet(
        "Host-Name über Rückwärtsauflösung erlaubt",
        "chat.fritz.box",
    );
    assert_eq!(gemeldet.len(), 1, "{gemeldet:?}");
    assert_eq!(wert(&gemeldet[0], "adresse").as_deref(), Some("127.0.0.2"));

    let erste = starten(&["--host", "0.0.0.0"], &umgebung);
    assert_eq!(erste.gesundheit("chat.fritz.box").status, 403);
    assert_eq!(erste.gefragt("vorwaerts", "chat.fritz.box"), 0);
}

/// Kriterium „Origin wörtlich“ (E5, Matrix D11, D12).
#[test]
fn f0077_origin_wird_woertlich_mit_dem_host_kopf_verglichen() {
    let lauf = mit_testnamen(&[]);
    let host = format!("{}.local.:{}", rechnername(), lauf.port());
    let koerper = r#"{"name":"beta"}"#;
    let json = "Content-Type: application/json";

    let gleich = lauf.anfragen(
        "POST /api/beitreten",
        &host,
        &[json, &format!("Origin: http://{host}")],
        koerper,
    );
    let ohne_punkt = lauf.anfragen(
        "POST /api/beitreten",
        &host,
        &[
            json,
            &format!("Origin: http://{}.local:{}", rechnername(), lauf.port()),
        ],
        r#"{"name":"gamma"}"#,
    );

    assert_eq!(gleich.status, 201, "{}", gleich.inhalt);
    assert_eq!(ohne_punkt.status, 403, "{}", ohne_punkt.inhalt);
    assert!(
        ohne_punkt.inhalt.contains("\"fehler\""),
        "{}",
        ohne_punkt.inhalt
    );
}

/// Kriterium „Testknopf nur im Debug-Build“: ein falscher Wert ist Exit 2 und nennt die Variable (Matrix H2).
#[test]
#[cfg_attr(
    not(debug_assertions),
    ignore = "Debug-Probe: cargo test --test netznamen"
)]
fn f0077_falsche_testnamen_sind_exit_2() {
    for falsch in ["kaputt", "quatsch", "x.local=", "x.local=999.1.1.1", ">x"] {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let ende = exit_code(wurzel.path(), &[("LOCALCHATA_TESTNAMEN", falsch)], &[]);

        assert_eq!(ende.exit, Some(2), "{falsch}: {}", ende.stderr);
        assert!(
            ende.stderr.contains("LOCALCHATA_TESTNAMEN"),
            "{}",
            ende.stderr
        );
        assert!(ende.stdout.is_empty(), "kein Listener: {}", ende.stdout);
    }
}

/// Kriterium „Testknopf nur im Debug-Build“, Release (Matrix H3, H4): die Variable wird nicht gelesen —
/// `nuc5.fritz.box` ist über 127.0.0.1 abgewiesen (echt: `localhost`), ein falscher Wert hält den Start nicht auf.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "Release-Probe: cargo test --release --test netznamen -- release"
)]
fn release_testnamen_werden_nicht_gelesen() {
    let lauf = mit_testnamen(&[]);
    assert_eq!(lauf.gesundheit("nuc5.fritz.box").status, 403);
    assert_eq!(lauf.gesundheit("chat.local").status, 403);

    let kaputt = starten(&[], &[("LOCALCHATA_TESTNAMEN", "quatsch")]);
    assert!(erlaubt(&kaputt.gesundheit("127.0.0.1")));
}

// ---------------------------------------------------------------------------------------------------------
// F0078 Name im lokalen Namensraum
// ---------------------------------------------------------------------------------------------------------

/// Kriterium „Über das echte Programm“ (Matrix C1, C3, C4, A6, D1, C11, C12, K6).
#[test]
fn f0078_lokale_namen_auf_eigene_adressen_oeffnen_andere_nicht() {
    let lauf = mit_testnamen(&[]);

    for name in ["chat.local", "chat", "nas.home.arpa", "dienst.internal"] {
        let antwort = lauf.gesundheit(name);
        assert!(
            erlaubt(&antwort),
            "{name}: {} {}",
            antwort.status,
            antwort.inhalt
        );
    }
    for name in [
        "drucker.local",
        "local.boese.example",
        "evil-local",
        "chat.fritz.box",
        "x.localhost",
    ] {
        assert_eq!(lauf.gesundheit(name).status, 403, "{name}");
    }

    let freigegeben = mit_testnamen(&["--erreichbar-als", "chat.fritz.box"]);
    assert!(erlaubt(&freigegeben.gesundheit("chat.fritz.box")));
}

/// Kriterium „Gefragt / nicht gefragt“ (E6; Matrix C1, C2, D1, C12, C13).
#[test]
fn f0078_ein_lokaler_name_wird_einmal_gefragt_andere_nie_vorwaerts() {
    let lauf = mit_testnamen(&[]);

    for name in ["chat.local", "chat.local", "CHAT.LOCAL", "chat.local."] {
        assert!(erlaubt(&lauf.gesundheit(name)), "{name}");
    }
    for name in [
        "local.boese.example",
        "x.localhost",
        "2130706433",
        "0x7f000001",
    ] {
        assert_eq!(lauf.gesundheit(name).status, 403, "{name}");
    }

    assert_eq!(lauf.gefragt("vorwaerts", "chat.local"), 1);
    for name in [
        "local.boese.example",
        "x.localhost",
        "2130706433",
        "0x7f000001",
    ] {
        assert_eq!(lauf.gefragt("vorwaerts", name), 0, "{name}");
    }
}

/// Kriterium „Protokoll“ (Matrix C1): zwei Anfragen → genau eine Information mit Name und Adresse.
#[test]
fn f0078_zweimal_chat_local_eine_information() {
    let lauf = mit_testnamen(&[]);

    assert!(erlaubt(&lauf.gesundheit("chat.local")));
    assert!(erlaubt(&lauf.gesundheit("chat.local")));

    let gemeldet = lauf.erlaubt_gemeldet("Host-Name über lokalen Namensraum erlaubt", "chat.local");
    assert_eq!(gemeldet.len(), 1, "{gemeldet:?}");
    assert_eq!(wert(&gemeldet[0], "adresse").as_deref(), Some("127.0.0.1"));
}

/// Matrix C5–C7, C10 (L2; E1): gemischte Antwort erlaubt; unspezifiziert, Multicast, Broadcast abgewiesen, ohne
/// „eigen?“ zu fragen; ein Name ohne Antwort abgewiesen.
#[test]
fn c5_c7_c10_gemischt_erlaubt_unspezifiziert_multicast_broadcast_nie() {
    let lauf = starten(
        &[],
        &[
            (
                "LOCALCHATA_TESTNAMEN",
                "mix.local=192.0.2.1,127.0.0.1;null.local=0.0.0.0;null6.local=::;mc.local=224.0.0.251;bc.local=255.255.255.255",
            ),
            ("LOCALCHATA_PROTOKOLL_STUFE", "debug"),
        ],
    );

    assert!(erlaubt(&lauf.gesundheit("mix.local")));
    for name in [
        "null.local",
        "null6.local",
        "mc.local",
        "bc.local",
        "gibtsnicht.local",
    ] {
        assert_eq!(lauf.gesundheit(name).status, 403, "{name}");
        assert_eq!(lauf.gefragt("vorwaerts", name), 1, "{name}");
    }
    for adresse in ["0.0.0.0", "::", "224.0.0.251", "255.255.255.255"] {
        assert_eq!(lauf.gefragt("eigen", adresse), 0, "{adresse}");
    }
    assert_eq!(lauf.gefragt("eigen", "127.0.0.1"), 1);
}

// ---------------------------------------------------------------------------------------------------------
// F0079 Abweisung als verständliche Seite
// ---------------------------------------------------------------------------------------------------------

const CSP: &str = "default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self' data:; font-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'";
const UEBERSCHRIFT: &str = "Diese Adresse ist für localchatA nicht freigegeben";

impl Lauf {
    /// Roh, Zeilen wie gegeben (ohne automatischen Host).
    fn roh(&self, zeilen: &[&str]) -> RohAntwort {
        roh_anfragen(
            SocketAddr::new(self.ueber, self.port()),
            &zeilen.join("\r\n"),
            "",
        )
    }

    fn eigene_adresse(&self) -> String {
        format!("http://{}/", SocketAddr::new(self.ueber, self.port()))
    }
}

/// ABW-H: 403, HTML-Seite mit Link auf die lokale Adresse, Header-Satz (CSP unverändert), kein `Location`.
fn ist_seite(antwort: &RohAntwort, link: &str) {
    assert_eq!(antwort.status, 403, "{}", antwort.inhalt);
    assert_eq!(
        antwort.header("content-type"),
        Some("text/html; charset=utf-8")
    );
    assert!(antwort.inhalt.contains(UEBERSCHRIFT), "{}", antwort.inhalt);
    assert!(
        antwort
            .inhalt
            .contains(&format!("<a href=\"{link}\">{link}</a>")),
        "{}",
        antwort.inhalt
    );
    assert_eq!(antwort.header("content-security-policy"), Some(CSP));
    assert_eq!(antwort.header("x-content-type-options"), Some("nosniff"));
    assert_eq!(antwort.header("x-frame-options"), Some("DENY"));
    assert_eq!(antwort.header("location"), None);
}

/// ABW-T: 403, Text mit Name und Adresse, kein HTML.
fn ist_text(antwort: &RohAntwort, name: &str, link: &str) {
    assert_eq!(antwort.status, 403, "{}", antwort.inhalt);
    assert_eq!(
        antwort.header("content-type"),
        Some("text/plain; charset=utf-8")
    );
    for teil in [
        name.to_string(),
        link.to_string(),
        format!("--erreichbar-als {name}"),
        "Namen des eigenen Netzes gelten automatisch".to_string(),
    ] {
        assert!(antwort.inhalt.contains(&teil), "{teil}: {}", antwort.inhalt);
    }
    assert!(!antwort.inhalt.contains("<html"), "{}", antwort.inhalt);
}

/// Kriterium „Browser bekommt die Seite“ (US-6, Matrix A5, G1, G3 a/b).
#[test]
fn f0079_ein_browser_dokument_bekommt_die_seite() {
    let lauf = mit_testnamen(&[]);
    let host = format!("boese.example:{}", lauf.port());
    let link = lauf.eigene_adresse();

    let accept = lauf.anfragen("GET /", &host, &["Accept: text/html"], "");
    let dest = lauf.anfragen(
        "GET /",
        &host,
        &["Sec-Fetch-Dest: document", "Accept: */*"],
        "",
    );
    let beides = lauf.anfragen(
        "GET /api/gesundheit",
        &host,
        &["Accept: text/html,*/*", "Sec-Fetch-Dest: document"],
        "",
    );

    ist_seite(&accept, &link);
    assert!(accept.inhalt.contains(
        "Der Betreiber kann den Namen boese.example mit --erreichbar-als boese.example beim Start freigeben."
    ));
    ist_seite(&dest, &link);
    ist_seite(&beides, &link);
    assert_eq!(beides.header("cache-control"), Some("no-store"));
}

/// Matrix G1: der Link nennt den Port der lokalen Adresse, nicht den des Host-Kopfs.
#[test]
fn g1_der_link_nennt_den_port_der_lokalen_adresse() {
    let lauf = mit_testnamen(&[]);

    let antwort = lauf.anfragen("GET /", "boese.example:9999", &["Accept: text/html"], "");

    ist_seite(&antwort, &lauf.eigene_adresse());
    assert!(!antwort.inhalt.contains(":9999/"), "{}", antwort.inhalt);
}

/// Kriterium „Agent bekommt Text“ (US-8, Matrix G3 c/d).
#[test]
fn f0079_ein_agent_bekommt_text_mit_der_adresse() {
    let lauf = mit_testnamen(&[]);
    let host = format!("boese.example:{}", lauf.port());
    let link = lauf.eigene_adresse();

    ist_text(
        &lauf.anfragen(
            "GET /api/gesundheit",
            &host,
            &["Accept: application/json"],
            "",
        ),
        "boese.example",
        &link,
    );
    ist_text(
        &lauf.anfragen("GET /api/gesundheit", &host, &[], ""),
        "boese.example",
        &link,
    );
    ist_text(
        &lauf.anfragen("GET /", &host, &["Accept: */*"], ""),
        "boese.example",
        &link,
    );
}

/// Matrix G4: die Seite auch auf einer Schreibroute — ohne Wirkung.
#[test]
fn g4_seite_auf_der_schreibroute_ohne_beitritt() {
    let lauf = mit_testnamen(&[]);
    let host = format!("boese.example:{}", lauf.port());

    let antwort = lauf.anfragen(
        "POST /api/beitreten",
        &host,
        &[
            "Accept: text/html",
            "Content-Type: application/json",
            &format!("Origin: http://{host}"),
        ],
        r#"{"name":"beta"}"#,
    );

    ist_seite(&antwort, &lauf.eigene_adresse());
    let gesundheit = lauf.gesundheit("127.0.0.1");
    assert!(
        gesundheit.inhalt.contains("\"sitzungen\":0"),
        "{}",
        gesundheit.inhalt
    );
}

/// Matrix G2 (roh): der Name erscheint nur maskiert.
#[test]
fn g2_der_name_wird_in_der_seite_maskiert() {
    let lauf = mit_testnamen(&[]);
    for (host, maskiert) in [
        (
            "<img src=x onerror=alert(1)>",
            "&lt;img src=x onerror=alert(1)&gt;",
        ),
        ("a\"b'c&d.local", "a&quot;b&#39;c&amp;d.local"),
        (
            "</h1><script>x</script>",
            "&lt;/h1&gt;&lt;script&gt;x&lt;/script&gt;",
        ),
    ] {
        let antwort = lauf.roh(&[
            "GET / HTTP/1.1",
            &format!("Host: {host}"),
            "Accept: text/html",
        ]);
        ist_seite(&antwort, &lauf.eigene_adresse());
        assert!(
            antwort.inhalt.contains(maskiert),
            "{host}: {}",
            antwort.inhalt
        );
        assert!(!antwort.inhalt.contains("<img"), "{}", antwort.inhalt);
        assert!(!antwort.inhalt.contains("<script"), "{}", antwort.inhalt);
    }
}

/// Kriterium „Ohne Host“ (E4, Matrix G5, D9, D10).
#[test]
fn f0079_ohne_host_seite_ohne_namen_text_doppelt_ohne_seite() {
    let lauf = mit_testnamen(&[]);
    let link = lauf.eigene_adresse();

    let seite = lauf.roh(&["GET / HTTP/1.0", "Accept: text/html"]);
    ist_seite(&seite, &link);
    assert!(
        !seite.inhalt.contains("--erreichbar-als"),
        "{}",
        seite.inhalt
    );

    let leer = lauf.roh(&["GET / HTTP/1.1", "Host: ", "Accept: text/html"]);
    ist_seite(&leer, &link);
    assert!(!leer.inhalt.contains("--erreichbar-als"), "{}", leer.inhalt);

    let text = lauf.roh(&["GET / HTTP/1.0", "Accept: */*"]);
    assert_eq!(text.status, 403);
    assert_eq!(
        text.header("content-type"),
        Some("text/plain; charset=utf-8")
    );
    assert!(text.inhalt.contains("nennt keinen Host"), "{}", text.inhalt);
    assert!(text.inhalt.contains(&link), "{}", text.inhalt);

    let host = format!("Host: 127.0.0.1:{}", lauf.port());
    for zweiter in [host.as_str(), "Host: boese.example"] {
        let doppelt = lauf.roh(&["GET / HTTP/1.1", &host, zweiter, "Accept: text/html"]);
        assert_eq!(doppelt.status, 403, "{}", doppelt.inhalt);
        assert_ne!(
            doppelt.header("content-type"),
            Some("text/html; charset=utf-8")
        );
        assert!(!doppelt.inhalt.contains("<html"), "{}", doppelt.inhalt);
    }
}

/// Matrix E4, E5: der Link bei IPv6 in Klammern, bei Dual-Stack über IPv4 ohne `::ffff:`.
#[test]
fn e4_e5_der_link_bei_ipv6_und_dual_stack() {
    let v6 = starten(&["--host", "::"], &[]).ueber("::1");
    let v4 = starten(&["--host", "::"], &[]);

    let ueber_v6 = v6.anfragen("GET /", "boese.example", &["Accept: text/html"], "");
    let ueber_v4 = v4.anfragen("GET /", "boese.example", &["Accept: text/html"], "");

    ist_seite(&ueber_v6, &format!("http://[::1]:{}/", v6.port()));
    ist_seite(&ueber_v4, &format!("http://127.0.0.1:{}/", v4.port()));
    assert!(!ueber_v4.inhalt.contains("::ffff:"), "{}", ueber_v4.inhalt);
}

// ---------------------------------------------------------------------------------------------------------
// F0080 Schalter --alle-namen
// ---------------------------------------------------------------------------------------------------------

/// US-11, Matrix I12: ohne den Schalter ist ein falscher Wert Exit 2, nennt die Variable, kein Listener.
#[test]
fn f0080_ein_falscher_wert_ohne_schalter_ist_exit_2() {
    for falsch in ["ja", "true", "2", " 1"] {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let ende = exit_code(wurzel.path(), &[("LOCALCHATA_ALLE_NAMEN", falsch)], &[]);

        assert_eq!(ende.exit, Some(2), "{falsch:?}: {}", ende.stderr);
        assert!(
            ende.stderr.contains("LOCALCHATA_ALLE_NAMEN"),
            "{}",
            ende.stderr
        );
        assert!(ende.stdout.is_empty(), "{}", ende.stdout);
    }
}

/// US-12, Matrix I14 (E7): der Schalter im Aufruf gilt neben einer falschen Umgebung.
#[test]
fn f0080_der_schalter_gilt_neben_einer_falschen_umgebung() {
    let lauf = starten(&["--alle-namen"], &[("LOCALCHATA_ALLE_NAMEN", "ja")]);

    assert!(erlaubt(&lauf.gesundheit("127.0.0.1")));
}

/// Alle Zeilen „Namensauskunft gefragt“, gleich welche Art.
fn alle_fragen(lauf: &Lauf) -> usize {
    mit(&lauf.protokoll(), "DEBUG", "grenzschutz")
        .iter()
        .filter(|zeile| zeile.contains("Namensauskunft gefragt"))
        .count()
}

fn beitreten(lauf: &Lauf, host: &str, kopf: &[&str], name: &str) -> RohAntwort {
    let mut alle = vec!["Content-Type: application/json"];
    alle.extend(kopf);
    lauf.anfragen(
        "POST /api/beitreten",
        host,
        &alle,
        &format!(r#"{{"name":"{name}"}}"#),
    )
}

/// Kriterium F0080 „Jeder Name“ (Matrix I1, I5): jeder Name in Normalform erlaubt, nie gefragt; Normalform und
/// fehlender Host bleiben abgewiesen; mit `Accept: text/html` die Seite.
#[test]
fn f0080_mit_alle_namen_gilt_jeder_name_ohne_frage() {
    let lauf = starten(
        &["--alle-namen"],
        &[
            ("LOCALCHATA_TESTNAMEN", TESTNAMEN),
            ("LOCALCHATA_PROTOKOLL_STUFE", "debug"),
        ],
    );

    for name in ["boese.example", "chat.lan", "x.local", "drucker.local"] {
        assert!(erlaubt(&lauf.gesundheit(name)), "{name}");
    }
    for name in ["a_b.example", "bücher.local", "2130706433"] {
        assert_eq!(lauf.gesundheit(name).status, 403, "{name}");
    }
    let lang = format!("{}.local", "a".repeat(64));
    assert_eq!(lauf.gesundheit(&lang).status, 403);
    assert_eq!(lauf.roh(&["GET / HTTP/1.0", "Accept: */*"]).status, 403);
    ist_seite(
        &lauf.roh(&["GET / HTTP/1.0", "Accept: text/html"]),
        &lauf.eigene_adresse(),
    );
    let host = format!("Host: 127.0.0.1:{}", lauf.port());
    assert_eq!(lauf.roh(&["GET / HTTP/1.1", &host, &host]).status, 403);

    assert_eq!(alle_fragen(&lauf), 0);
}

/// Kriterium F0080 „Jeder Name“ und US-10 (Matrix I2–I4, I6, I7): CSRF bleibt zu, Rebinding ist bewusst offen.
#[test]
fn f0080_die_ursprungsregel_bleibt_rebinding_ist_offen() {
    let lauf = starten(&["--alle-namen"], &[]);
    let boese = format!("boese.example:{}", lauf.port());
    let eigen = format!("127.0.0.1:{}", lauf.port());

    let fremder_origin = beitreten(&lauf, &boese, &["Origin: http://andere.example"], "beta");
    assert_eq!(fremder_origin.status, 403, "{}", fremder_origin.inhalt);
    assert!(fremder_origin.inhalt.contains("\"fehler\""));
    for origin in ["Origin: http://boese.example", "Origin: null"] {
        let csrf = beitreten(&lauf, &eigen, &[origin], "beta");
        assert_eq!(csrf.status, 403, "{origin}: {}", csrf.inhalt);
    }
    for site in ["Sec-Fetch-Site: same-site", "Sec-Fetch-Site: cross-site"] {
        let origin = format!("Origin: http://{boese}");
        let antwort = beitreten(&lauf, &boese, &[&origin, site], "beta");
        assert_eq!(antwort.status, 403, "{site}");
    }
    let vorflug = lauf.anfragen(
        "OPTIONS /api/beitreten",
        &boese,
        &[
            "Origin: http://boese.example",
            "Access-Control-Request-Method: POST",
        ],
        "",
    );
    assert_eq!(vorflug.status, 405);
    assert!(
        !vorflug
            .kopf
            .iter()
            .any(|(name, _)| name.starts_with("access-control-allow-")),
        "{:?}",
        vorflug.kopf
    );
    assert!(
        lauf.gesundheit("127.0.0.1")
            .inhalt
            .contains("\"sitzungen\":0")
    );

    let origin = format!("Origin: http://{boese}");
    let rebinding = beitreten(
        &lauf,
        &boese,
        &[&origin, "Sec-Fetch-Site: same-origin"],
        "beta",
    );
    assert_eq!(rebinding.status, 201, "{}", rebinding.inhalt);

    assert_eq!(
        lauf.gesundheit("boese.example")
            .header("cross-origin-opener-policy"),
        None
    );
    assert_eq!(
        lauf.gesundheit("127.0.0.1")
            .header("cross-origin-opener-policy"),
        Some("same-origin")
    );
}

const RISIKO: &str = "--alle-namen ist aktiv: Jeder Host-Name wird angenommen — eine fremde Website, die ein Teilnehmer öffnet, kann über DNS-Rebinding Channels mitlesen, Nachrichten schreiben und über die Frist Verläufe löschen.";

/// Kriterium F0080 „Sichtbar“ (US-9, Matrix I8–I11): WARNING vor der Startzeile in der Datei und auf stderr,
/// `alle_namen` im Health-Check; ohne den Schalter `false` und keine solche Zeile.
#[test]
fn f0080_der_schalter_ist_sichtbar_in_protokoll_stderr_und_gesundheit() {
    let mit_option = starten(&["--alle-namen"], &[]);
    let mit_umgebung = starten(&[], &[("LOCALCHATA_ALLE_NAMEN", "1")]);
    for lauf in [&mit_option, &mit_umgebung] {
        assert!(
            lauf.gesundheit("127.0.0.1")
                .inhalt
                .contains("\"alle_namen\":true")
        );
        let protokoll = lauf.protokoll();
        let warnung = protokoll
            .iter()
            .position(|zeile| zeile.contains(" WARNING grenzschutz ") && zeile.contains(RISIKO))
            .unwrap_or_else(|| panic!("WARNING fehlt: {protokoll:?}"));
        let bereit = protokoll
            .iter()
            .position(|zeile| zeile.contains("Instanz bereit"))
            .expect("Instanz bereit");
        assert!(warnung < bereit, "{protokoll:?}");
        let stderr = lauf.stderr.lock().expect("stderr").clone();
        assert!(
            stderr
                .iter()
                .any(|zeile| zeile.contains(" WARNING grenzschutz ") && zeile.contains(RISIKO)),
            "{stderr:?}"
        );
    }

    for umgebung in [
        &[][..],
        &[("LOCALCHATA_ALLE_NAMEN", "0")][..],
        &[("LOCALCHATA_ALLE_NAMEN", "")][..],
    ] {
        let ohne = starten(&[], umgebung);
        assert!(
            ohne.gesundheit("127.0.0.1")
                .inhalt
                .contains("\"alle_namen\":false"),
            "{umgebung:?}"
        );
        assert!(
            !ohne
                .protokoll()
                .iter()
                .any(|zeile| zeile.contains("--alle-namen ist aktiv")),
            "{umgebung:?}"
        );
        assert_eq!(ohne.gesundheit("boese.example").status, 403);
    }
}

/// Matrix I13: Option geht vor `LOCALCHATA_ALLE_NAMEN=0`.
#[test]
fn i13_die_option_geht_der_umgebung_vor() {
    let lauf = starten(&["--alle-namen"], &[("LOCALCHATA_ALLE_NAMEN", "0")]);

    assert!(
        lauf.gesundheit("127.0.0.1")
            .inhalt
            .contains("\"alle_namen\":true")
    );
    assert!(erlaubt(&lauf.gesundheit("boese.example")));
}

/// Matrix F8 (L2): Abweisungen unter Flut sind eine Episode je Herkunft, keine Zeile je Anfrage; „erlaubt“
/// höchstens einmal je Cache-Eintrag.
#[test]
fn f8_protokoll_unter_flut_eine_episode_und_eine_information() {
    let lauf = mit_testnamen(&[]);

    for nummer in 0..300 {
        assert_eq!(lauf.gesundheit(&format!("r{nummer}.local")).status, 403);
    }
    // Nacharbeit #35 (ADR-0045 Ergänzung „Karte #35 Nacharbeit“ Punkt 2, Rate): die Flut hat den Eimer geleert —
    // ein neuer Name kommt nach der Flut herein, sobald wieder ein Zeichen da ist (10 je Sekunde).
    let ende = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !erlaubt(&lauf.gesundheit("chat.local")) {
        assert!(
            std::time::Instant::now() < ende,
            "chat.local nach der Flut nicht erlaubt"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    for _ in 0..20 {
        assert!(erlaubt(&lauf.gesundheit("chat.local")));
    }

    let warnungen = mit(&lauf.protokoll(), "WARNING", "grenzschutz");
    let abweisungen: Vec<&String> = warnungen
        .iter()
        .filter(|zeile| zeile.contains("Anfrage mit fremdem Host abgewiesen"))
        .collect();
    assert_eq!(abweisungen.len(), 1, "{abweisungen:?}");
    assert!(
        abweisungen[0].contains("Episode beginnt"),
        "{abweisungen:?}"
    );
    let gemeldet = lauf.erlaubt_gemeldet("Host-Name über lokalen Namensraum erlaubt", "chat.local");
    assert_eq!(gemeldet.len(), 1, "{gemeldet:?}");
}

/// Matrix I15, I16 (L2): `--alle-namen=1` ist eine unbekannte Option; `--hilfe` nennt den Schalter mit dem
/// Risikosatz auf eigener Zeile.
#[test]
fn i15_i16_schreibweise_mit_wert_und_hilfe() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let ende = exit_code(wurzel.path(), &[], &["--alle-namen=1"]);
    assert_eq!(ende.exit, Some(2), "{}", ende.stderr);
    assert!(
        ende.stderr.contains("Unbekannte Option --alle-namen=1"),
        "{}",
        ende.stderr
    );

    let hilfe = gemeinsam::aufrufen(&["--hilfe"]);
    assert_eq!(hilfe.exit, Some(0));
    assert!(
        hilfe
            .stdout
            .lines()
            .next()
            .unwrap_or_default()
            .contains("[--alle-namen]")
    );
    assert!(
        hilfe
            .stdout
            .lines()
            .any(|zeile| zeile.trim_start().starts_with("--alle-namen")
                && zeile.contains("LOCALCHATA_ALLE_NAMEN")),
        "{}",
        hilfe.stdout
    );
    // Probe des rechtschreib-reviewer (Nacharbeit #35, ADR-0045 Ergänzung Punkt 5): „Risiko: eine fremde“ 0-mal,
    // der vollständige Satz mit „Eine“ genau einmal, auf eigener Zeile.
    let satz = "Risiko: Eine fremde Website, die ein Teilnehmer öffnet, kann über DNS-Rebinding Channels mitlesen, Nachrichten schreiben und über die Frist Verläufe löschen.";
    let zeilen_mit = |text: &str| {
        hilfe
            .stdout
            .lines()
            .filter(|zeile| zeile.contains(text))
            .count()
    };
    assert_eq!(zeilen_mit("Risiko: eine fremde"), 0, "{}", hilfe.stdout);
    assert_eq!(zeilen_mit(satz), 1, "{}", hilfe.stdout);
    assert!(
        hilfe.stdout.lines().any(|zeile| zeile.trim_start() == satz),
        "{}",
        hilfe.stdout
    );
    assert!(
        hilfe.stdout.contains("gelten automatisch"),
        "{}",
        hilfe.stdout
    );
}

/// Kriterium „Rückwärtsname im echten Netz“ (echtes DNS, ohne Testnamen; Matrix B7 automatisch, soweit der
/// Rechner es hergibt): nur mit einer LAN-Adresse, für die `getent hosts` einen Namen liefert — sonst als
/// übersprungen gemeldet (stderr des Tests).
#[test]
fn f0077_rueckwaertsname_im_echten_netz() {
    let Some((lan, name)) = lan_adresse_mit_namen() else {
        eprintln!("übersprungen: keine LAN-Adresse mit Rückwärtsnamen (getent hosts)");
        return;
    };
    let lauf = starten(&["--host", "0.0.0.0"], &[]).ueber(&lan.to_string());

    let antwort = lauf.gesundheit(&name);

    assert!(
        erlaubt(&antwort),
        "{name} über {lan}: {} {}",
        antwort.status,
        antwort.inhalt
    );
}

fn lan_adresse_mit_namen() -> Option<(IpAddr, String)> {
    let sonde = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    sonde.connect("192.0.2.1:9").ok()?;
    let lan = sonde.local_addr().ok()?.ip();
    if lan.is_loopback() || lan.is_unspecified() {
        return None;
    }
    let ausgabe = std::process::Command::new("getent")
        .args(["hosts", &lan.to_string()])
        .output()
        .ok()?;
    let text = String::from_utf8(ausgabe.stdout).ok()?;
    let name = text.split_whitespace().nth(1)?.to_string();
    Some((lan, name))
}
