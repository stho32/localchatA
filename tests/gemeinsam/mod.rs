//! Startet das echte Programm `localchatA` und liest die Adresse aus der Startzeile (ADR-0009).
#![allow(dead_code)]

pub mod frist;
#[cfg(target_os = "linux")]
pub mod herkunft;
pub mod langsam;
pub mod protokoll;
pub mod senden;
pub mod strom;

use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStderr, ChildStdout, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

pub const PROGRAMM: &str = env!("CARGO_BIN_EXE_localchatA");

pub struct Instanz {
    pub ordner: PathBuf,
    wegwerf: Option<tempfile::TempDir>,
    pub kind: Child,
    pub startzeile: String,
    pub adresse: String,
    stdout: BufReader<ChildStdout>,
    stderr: Option<ChildStderr>,
}

/// Endet mit dem Drop: SIGTERM, damit das Programm geordnet endet; notfalls hart. Danach erst den Ordner weg.
impl Drop for Instanz {
    fn drop(&mut self) {
        if let Ok(None) = self.kind.try_wait() {
            let _ = signal_zustellen(&self.kind, "TERM");
            if warten(&mut self.kind, Duration::from_secs(5)).is_none() {
                let _ = self.kind.kill();
                let _ = self.kind.wait();
            }
        }
        self.wegwerf.take();
    }
}

impl Instanz {
    pub fn port(&self) -> u16 {
        let ohne_schraegstrich = self.adresse.trim_end_matches('/');
        let port = ohne_schraegstrich
            .rsplit(':')
            .next()
            .expect("Port in der Adresse");
        port.parse().expect("Port ist eine Zahl")
    }

    /// Schickt ein Signal (`TERM`, `INT`) und wartet höchstens fünf Sekunden auf das Ende.
    pub fn signal_und_warten(&mut self, signal: &str) -> ExitStatus {
        signal_senden(&self.kind, signal);
        warten(&mut self.kind, Duration::from_secs(5)).expect("Programm endet nach dem Signal")
    }

    /// Wartet höchstens `hoechstens` auf die nächste Zeile auf stderr (nur bei `starten_mit`).
    pub fn stderr_zeile_abwarten(&mut self, hoechstens: Duration) -> Option<String> {
        let stderr = self.stderr.take()?;
        let (senden, empfangen) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut zeile = String::new();
            let _ = BufReader::new(stderr).read_line(&mut zeile);
            let _ = senden.send(zeile);
        });
        empfangen
            .recv_timeout(hoechstens)
            .ok()
            .filter(|zeile| !zeile.is_empty())
    }

    /// Liest stderr ab jetzt in einem Thread zeilenweise mit (nur bei `starten_mit`); die Zeilen stehen in der
    /// gelieferten Liste, sobald sie gelesen sind.
    pub fn stderr_mitschreiben(&mut self) -> std::sync::Arc<std::sync::Mutex<Vec<String>>> {
        let zeilen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        if let Some(stderr) = self.stderr.take() {
            let ziel = std::sync::Arc::clone(&zeilen);
            std::thread::spawn(move || {
                for zeile in BufReader::new(stderr).lines().map_while(Result::ok) {
                    ziel.lock().expect("Zeilen").push(zeile);
                }
            });
        }
        zeilen
    }

    /// Alles, was auf stderr kam (nur bei `starten_mit`; liest bis zum Ende des Programms).
    pub fn stderr_bis_zum_ende(&mut self) -> String {
        let mut text = String::new();
        if let Some(mut stderr) = self.stderr.take() {
            stderr.read_to_string(&mut text).expect("stderr lesen");
        }
        text
    }

    /// Was nach der Startzeile noch auf stdout kam (liest bis zum Ende des Programms).
    pub fn restliche_ausgabe(&mut self) -> String {
        let mut rest = String::new();
        self.stdout.read_to_string(&mut rest).expect("stdout lesen");
        rest
    }
}

/// Schickt ein Signal (`TERM`, `INT`) per Systemaufruf — ohne den Umweg über einen `kill`-Prozess.
pub fn signal_senden(kind: &Child, signal: &str) {
    if let Err(fehler) = signal_zustellen(kind, signal) {
        panic!("Signal {signal} nicht zugestellt: {fehler}");
    }
}

fn signal_zustellen(kind: &Child, signal: &str) -> std::io::Result<()> {
    use rustix::process::{Pid, Signal, kill_process};
    let signal = match signal {
        "TERM" => Signal::TERM,
        "INT" => Signal::INT,
        anderes => panic!("unbekanntes Signal {anderes}"),
    };
    kill_process(Pid::from_child(kind), signal).map_err(std::io::Error::from)
}

pub fn warten(kind: &mut Child, hoechstens: Duration) -> Option<ExitStatus> {
    let ende = Instant::now() + hoechstens;
    while Instant::now() < ende {
        if let Some(status) = kind.try_wait().expect("Prozessstatus") {
            return Some(status);
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    None
}

/// Startet das Programm mit `<ordner> <argumente…>` und wartet auf die Startzeile.
pub fn starten(ordner: &Path, argumente: &[&str]) -> Instanz {
    let kind = Command::new(PROGRAMM)
        .arg(ordner)
        .args(argumente)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("localchatA startet");
    auf_startzeile_warten(ordner, kind)
}

/// Wie `starten`, mit zusätzlichen Umgebungsvariablen und mitgelesenem stderr.
pub fn starten_mit(ordner: &Path, argumente: &[&str], umgebung: &[(&str, &str)]) -> Instanz {
    let kind = Command::new(PROGRAMM)
        .arg(ordner)
        .args(argumente)
        .envs(umgebung.iter().copied())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("localchatA startet");
    let mut instanz = auf_startzeile_warten(ordner, kind);
    instanz.stderr = instanz.kind.stderr.take();
    instanz
}

fn auf_startzeile_warten(ordner: &Path, kind: Child) -> Instanz {
    auf_startzeile_warten_hoechstens(ordner, kind, Duration::from_secs(30))
}

/// Wartet höchstens `hoechstens` auf die Startzeile; sonst ist der Prozess beendet und der Test rot.
pub fn auf_startzeile_warten_hoechstens(
    ordner: &Path,
    mut kind: Child,
    hoechstens: Duration,
) -> Instanz {
    let mut stdout = BufReader::new(kind.stdout.take().expect("stdout"));
    let (senden, empfangen) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut startzeile = String::new();
        let gelesen = stdout.read_line(&mut startzeile);
        let _ = senden.send((stdout, startzeile, gelesen));
    });
    let Ok((stdout, startzeile, gelesen)) = empfangen.recv_timeout(hoechstens) else {
        let _ = kind.kill();
        let _ = kind.wait();
        panic!("keine Startzeile binnen {hoechstens:?} — Programm beendet");
    };
    gelesen.expect("Startzeile lesen");
    assert!(
        !startzeile.is_empty(),
        "keine Startzeile — Programm beendet: {:?}",
        kind.try_wait()
    );
    let startzeile = startzeile.trim_end_matches('\n').to_string();
    let adresse = startzeile
        .rsplit(' ')
        .next()
        .expect("Adresse am Ende der Startzeile")
        .to_string();
    Instanz {
        ordner: ordner.to_path_buf(),
        wegwerf: None,
        kind,
        startzeile,
        adresse,
        stdout,
        stderr: None,
    }
}

/// Eine lokale Instanz in einem eigenen Wegwerf-Ordner.
pub fn wegwerf_starten() -> Instanz {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let mut instanz = lokal_starten(wurzel.path());
    instanz.wegwerf = Some(wurzel);
    instanz
}

/// Wie `wegwerf_starten`; `vorbereiten` legt vor dem Start Dateien in den Ordner.
pub fn wegwerf_starten_mit(vorbereiten: impl FnOnce(&Path)) -> Instanz {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    vorbereiten(wurzel.path());
    let mut instanz = lokal_starten(wurzel.path());
    instanz.wegwerf = Some(wurzel);
    instanz
}

/// Wie `starten`, auf Port 0, nur 127.0.0.1, ohne Browser.
pub fn lokal_starten(ordner: &Path) -> Instanz {
    starten(
        ordner,
        &["--port", "0", "--host", "127.0.0.1", "--kein-browser"],
    )
}

pub async fn gesundheit(basis: &str) -> (u16, serde_json::Value) {
    let antwort = reqwest::get(format!("{basis}api/gesundheit"))
        .await
        .expect("Health-Check beantwortet");
    let status = antwort.status().as_u16();
    (status, antwort.json().await.expect("JSON"))
}

/// Die Adresse, über die dieser Rechner im LAN erreichbar ist (UDP-connect sendet nichts).
pub fn lan_adresse() -> std::net::IpAddr {
    let sonde = std::net::UdpSocket::bind("0.0.0.0:0").expect("UDP-Sonde");
    sonde.connect("192.0.2.1:9").expect("Route ins LAN");
    let ip = sonde.local_addr().expect("lokale Adresse").ip();
    assert!(
        !ip.is_loopback() && !ip.is_unspecified(),
        "keine LAN-Adresse: {ip}"
    );
    ip
}

/// Ergebnis eines Aufrufs, der von selbst endet: Exit-Code, stdout, stderr.
pub struct Ende {
    pub exit: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

/// Ruft das Programm auf und wartet höchstens fünf Sekunden auf sein Ende (sonst `exit: None`).
pub fn aufrufen(argumente: &[&str]) -> Ende {
    aufrufen_mit(argumente, &[])
}

/// Wie `aufrufen`, mit zusätzlichen Umgebungsvariablen.
pub fn aufrufen_mit(argumente: &[&str], umgebung: &[(&str, &str)]) -> Ende {
    let mut kind = Command::new(PROGRAMM)
        .args(argumente)
        .envs(umgebung.iter().copied())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("localchatA startet");
    let status = warten(&mut kind, Duration::from_secs(5));
    if status.is_none() {
        let _ = kind.kill();
    }
    let ausgabe = kind.wait_with_output().expect("Ausgabe");
    Ende {
        exit: status.and_then(|s| s.code()),
        stdout: String::from_utf8_lossy(&ausgabe.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&ausgabe.stderr).into_owned(),
    }
}

/// Antwort auf eine Anfrage, die Byte für Byte so gesendet wurde, wie der Test sie schreibt.
#[derive(Debug)]
pub struct RohAntwort {
    pub status: u16,
    /// Header-Namen kleingeschrieben, in der Reihenfolge der Antwort.
    pub kopf: Vec<(String, String)>,
    pub inhalt: String,
}

impl RohAntwort {
    /// Der Wert eines Headers (Name klein), wenn er genau einmal vorkommt; mehrfach ist ein Fehler im Test.
    pub fn header(&self, name: &str) -> Option<&str> {
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

/// Sendet `anfrage` (Kopfzeilen ohne Leerzeile am Ende; `Connection: close` hängt die Hilfe an) roh an
/// `ziel` und liest die Antwort bis zum Ende der Verbindung.
pub fn roh_anfragen(ziel: std::net::SocketAddr, anfrage: &str, koerper: &str) -> RohAntwort {
    use std::io::Write;
    let mut verbindung = std::net::TcpStream::connect(ziel).expect("Verbindung");
    verbindung
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("Lesegrenze");
    let text = format!(
        "{anfrage}\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{koerper}",
        koerper.len()
    );
    verbindung.write_all(text.as_bytes()).expect("Anfrage");
    let mut roh = Vec::new();
    verbindung.read_to_end(&mut roh).expect("Antwort");
    let roh = String::from_utf8_lossy(&roh).into_owned();
    let (kopfteil, inhalt) = roh.split_once("\r\n\r\n").expect("Kopf und Inhalt");
    let mut zeilen = kopfteil.split("\r\n");
    let status = zeilen
        .next()
        .and_then(|statuszeile| statuszeile.split(' ').nth(1))
        .and_then(|code| code.parse().ok())
        .unwrap_or_else(|| panic!("keine Statuszeile: {roh:?}"));
    let kopf = zeilen
        .filter_map(|zeile| zeile.split_once(':'))
        .map(|(name, wert)| (name.trim().to_lowercase(), wert.trim().to_string()))
        .collect();
    RohAntwort {
        status,
        kopf,
        inhalt: inhalt.to_string(),
    }
}

/// Antwort einer API-Anfrage: Status, Kopf, Inhalt als JSON (`Null`, wenn er keines ist).
#[derive(Debug)]
pub struct ApiAntwort {
    pub status: u16,
    pub kopf: reqwest::header::HeaderMap,
    pub json: serde_json::Value,
}

/// Ein Teilnehmer im Test: sendet von einer festen Absenderadresse (`127.0.0.2` per Linux-Loopback).
pub struct Absender {
    client: reqwest::Client,
}

impl Absender {
    pub fn von(adresse: &str) -> Absender {
        let ip: std::net::IpAddr = adresse.parse().expect("Absenderadresse");
        Absender {
            client: reqwest::Client::builder()
                .local_address(ip)
                .build()
                .expect("Client"),
        }
    }

    /// `POST <basis>api/beitreten` mit JSON und optional dem Sitzungs-Header.
    pub async fn beitreten(
        &self,
        basis: &str,
        koerper: serde_json::Value,
        token: Option<&str>,
    ) -> ApiAntwort {
        let mut anfrage = self
            .client
            .post(format!("{basis}api/beitreten"))
            .json(&koerper);
        if let Some(token) = token {
            anfrage = anfrage.header("X-LocalchatA-Sitzung", token);
        }
        api_antwort(anfrage).await
    }

    /// `GET <basis>api/ich` mit optional dem Sitzungs-Header.
    pub async fn ich(&self, basis: &str, token: Option<&str>) -> ApiAntwort {
        let mut anfrage = self.client.get(format!("{basis}api/ich"));
        if let Some(token) = token {
            anfrage = anfrage.header("X-LocalchatA-Sitzung", token);
        }
        api_antwort(anfrage).await
    }

    /// `POST <basis>api/channels` mit JSON-Körper und dem Sitzungs-Header (R00006).
    pub async fn channel_anlegen(
        &self,
        basis: &str,
        token: &str,
        koerper: serde_json::Value,
    ) -> ApiAntwort {
        let anfrage = self
            .client
            .post(format!("{basis}api/channels"))
            .header("X-LocalchatA-Sitzung", token)
            .json(&koerper);
        api_antwort(anfrage).await
    }

    /// `GET <basis>api/channels` mit optional dem Sitzungs-Header.
    pub async fn channels(&self, basis: &str, token: Option<&str>) -> ApiAntwort {
        let mut anfrage = self.client.get(format!("{basis}api/channels"));
        if let Some(token) = token {
            anfrage = anfrage.header("X-LocalchatA-Sitzung", token);
        }
        api_antwort(anfrage).await
    }

    /// Eine beliebig zusammengesetzte Anfrage dieses Absenders.
    pub fn anfrage(&self, methode: reqwest::Method, url: String) -> reqwest::RequestBuilder {
        self.client.request(methode, url)
    }
}

pub async fn api_antwort(anfrage: reqwest::RequestBuilder) -> ApiAntwort {
    let antwort = anfrage.send().await.expect("Antwort");
    let status = antwort.status().as_u16();
    let kopf = antwort.headers().clone();
    let text = antwort.text().await.expect("Inhalt");
    ApiAntwort {
        status,
        kopf,
        json: serde_json::from_str(&text).unwrap_or(serde_json::Value::Null),
    }
}
