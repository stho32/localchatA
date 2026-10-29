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
    instanz.stderr_mitschreiben();
    Lauf {
        instanz,
        _wurzel: wurzel,
        ueber: IpAddr::from([127, 0, 0, 1]),
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
