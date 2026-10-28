//! Langsame Leser (R00013): der Channel `riesig`, nicht lesende Abrufe über Rohverbindungen und die Frage, ob
//! eine Verbindung auf der Seite der Instanz noch offen ist — beobachtet in `/proc/net/tcp`, ohne selbst Daten
//! an den Server zu senden.

use std::fs;
use std::path::Path;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpSocket, TcpStream};

pub const RIESIG: &str = "channels/riesig/2026-10-28.log";

/// `riesig` wie in R00013: 200 Sätze von `alpha@127.0.0.2` im Sekundentakt ab 08:00, je 16 384 Bytes `x` in
/// einer Zeile, an einem Tag.
pub fn riesig_anlegen(ordner: &Path) {
    let text = "x".repeat(16_384);
    let mut inhalt = String::new();
    for index in 0..200 {
        inhalt.push_str(&format!(
            "@ 2026-10-28T08:{:02}:{:02}.000+01:00 alpha@127.0.0.2\n  {text}\n\n",
            index / 60,
            index % 60
        ));
    }
    let ziel = ordner.join(RIESIG);
    fs::create_dir_all(ziel.parent().expect("Elternordner")).expect("Ordner");
    fs::write(ziel, inhalt).expect("Datei");
}

/// Ein nicht lesender Abruf: TCP-Socket mit 4 KiB Empfangspuffer (vor dem Verbinden gesetzt), gebunden an die
/// Herkunft, der die Anfrage mit Sitzung sendet und danach nie liest.
pub struct NichtLesend {
    pub strom: TcpStream,
    pub eigener_port: u16,
}

pub async fn nicht_lesend_oeffnen(
    server_port: u16,
    herkunft: &str,
    pfad: &str,
    token: &str,
) -> NichtLesend {
    let socket = TcpSocket::new_v4().expect("Socket");
    socket.set_recv_buffer_size(4096).expect("SO_RCVBUF");
    socket
        .bind(format!("{herkunft}:0").parse().expect("Herkunft"))
        .expect("gebunden");
    let ziel = format!("127.0.0.1:{server_port}").parse().expect("Ziel");
    let mut strom = socket.connect(ziel).await.expect("verbunden");
    let anfrage =
        format!("GET {pfad} HTTP/1.1\r\nHost: 127.0.0.1\r\nX-LocalchatA-Sitzung: {token}\r\n\r\n");
    strom
        .write_all(anfrage.as_bytes())
        .await
        .expect("Anfrage gesendet");
    let eigener_port = strom.local_addr().expect("Adresse").port();
    NichtLesend {
        strom,
        eigener_port,
    }
}

/// Die eigenen Ports (Gegenstellen), zu denen die Instanz auf `server_port` noch einen Socket hat — aus
/// `/proc/net/tcp`: Zeilen mit lokalem Port `server_port` und entferntem Port aus `gegenstellen`.
pub fn server_seitig_offen(server_port: u16, gegenstellen: &[u16]) -> Vec<u16> {
    let tabelle = fs::read_to_string("/proc/net/tcp").expect("/proc/net/tcp");
    let mut offen: Vec<u16> = tabelle
        .lines()
        .skip(1)
        .filter_map(|zeile| {
            let mut spalten = zeile.split_whitespace().skip(1);
            let lokal = port_aus(spalten.next()?)?;
            let entfernt = port_aus(spalten.next()?)?;
            (lokal == server_port && gegenstellen.contains(&entfernt)).then_some(entfernt)
        })
        .collect();
    offen.sort_unstable();
    offen.dedup();
    offen
}

/// `0100007F:1F90` → 8080.
fn port_aus(adresse: &str) -> Option<u16> {
    let (_, port) = adresse.split_once(':')?;
    u16::from_str_radix(port, 16).ok()
}
