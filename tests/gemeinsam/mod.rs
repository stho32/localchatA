//! Startet das echte Programm `localchatA` und liest die Adresse aus der Startzeile (ADR-0009).
#![allow(dead_code)]

use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

pub const PROGRAMM: &str = env!("CARGO_BIN_EXE_localchatA");

pub struct Instanz {
    pub ordner: PathBuf,
    wegwerf: Option<tempfile::TempDir>,
    pub kind: Child,
    pub startzeile: String,
    pub adresse: String,
    stdout: BufReader<ChildStdout>,
}

/// Endet mit dem Drop: SIGTERM, damit das Programm geordnet endet; notfalls hart. Danach erst den Ordner weg.
impl Drop for Instanz {
    fn drop(&mut self) {
        if let Ok(None) = self.kind.try_wait() {
            let _ = Command::new("kill")
                .arg("-TERM")
                .arg(self.kind.id().to_string())
                .status();
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

    /// Was nach der Startzeile noch auf stdout kam (liest bis zum Ende des Programms).
    pub fn restliche_ausgabe(&mut self) -> String {
        let mut rest = String::new();
        self.stdout.read_to_string(&mut rest).expect("stdout lesen");
        rest
    }
}

pub fn signal_senden(kind: &Child, signal: &str) {
    let gesendet = Command::new("kill")
        .arg(format!("-{signal}"))
        .arg(kind.id().to_string())
        .status()
        .expect("kill aufgerufen");
    assert!(gesendet.success(), "Signal {signal} nicht zugestellt");
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
    let mut kind = Command::new(PROGRAMM)
        .arg(ordner)
        .args(argumente)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("localchatA startet");
    let mut stdout = BufReader::new(kind.stdout.take().expect("stdout"));
    let mut startzeile = String::new();
    stdout.read_line(&mut startzeile).expect("Startzeile lesen");
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
    }
}

/// Eine lokale Instanz in einem eigenen Wegwerf-Ordner.
pub fn wegwerf_starten() -> Instanz {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
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
    let mut kind = Command::new(PROGRAMM)
        .args(argumente)
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
