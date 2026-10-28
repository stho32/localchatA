//! Langsame Leser (R00013): der Channel `riesig`, nicht lesende Abrufe über Rohverbindungen und die Frage, ob
//! eine Verbindung auf der Seite der Instanz noch offen ist — beobachtet in `/proc/net/tcp`, ohne selbst Daten
//! an den Server zu senden.

use std::fs;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
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
    /// Eigene Adresse dieser Verbindung (Herkunft und Port).
    pub eigene: SocketAddrV4,
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
    let SocketAddr::V4(eigene) = strom.local_addr().expect("Adresse") else {
        panic!("IPv4 erwartet");
    };
    NichtLesend { strom, eigene }
}

/// Die eigenen Gegenstellen (IP und Port), zu denen die Instanz auf `server_port` noch einen Socket hat — aus
/// `/proc/net/tcp`.
pub fn server_seitig_offen(server_port: u16, gegenstellen: &[SocketAddrV4]) -> Vec<SocketAddrV4> {
    let tabelle = fs::read_to_string("/proc/net/tcp").expect("/proc/net/tcp");
    offen_in(&tabelle, server_port, gegenstellen)
}

/// Rein: Zeilen der Tabelle mit lokalem Port `server_port` und entfernter Adresse aus `gegenstellen`.
fn offen_in(tabelle: &str, server_port: u16, gegenstellen: &[SocketAddrV4]) -> Vec<SocketAddrV4> {
    let mut offen: Vec<SocketAddrV4> = tabelle
        .lines()
        .skip(1)
        .filter_map(|zeile| {
            let mut spalten = zeile.split_whitespace().skip(1);
            let lokal = adresse_aus(spalten.next()?)?;
            let entfernt = adresse_aus(spalten.next()?)?;
            (lokal.port() == server_port && gegenstellen.contains(&entfernt)).then_some(entfernt)
        })
        .collect();
    offen.sort_unstable();
    offen.dedup();
    offen
}

/// `0100007F:1F90` → 127.0.0.1:8080 (IPv4 in der Byte-Reihenfolge des Rechners, hier little-endian).
fn adresse_aus(adresse: &str) -> Option<SocketAddrV4> {
    let (ip, port) = adresse.split_once(':')?;
    let ip = u32::from_str_radix(ip, 16).ok()?;
    let port = u16::from_str_radix(port, 16).ok()?;
    Some(SocketAddrV4::new(Ipv4Addr::from(ip.swap_bytes()), port))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TABELLE: &str = "  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode
   0: 0100007F:1F90 0200007F:C350 01 00000000:00000000 00:00000000 00000000  1000        0 1
   1: 0100007F:1F90 0900007F:C351 01 00000000:00000000 00:00000000 00000000  1000        0 2
   2: 0100007F:1F91 0200007F:C352 01 00000000:00000000 00:00000000 00000000  1000        0 3
";

    #[test]
    fn eine_fremde_verbindung_mit_gleicher_portnummer_von_anderer_ip_zaehlt_nicht() {
        let eigene: SocketAddrV4 = "127.0.0.2:50001".parse().expect("Adresse");

        assert_eq!(offen_in(TABELLE, 0x1F90, &[eigene]), []);
    }

    #[test]
    fn die_eigene_gegenstelle_am_server_port_zaehlt() {
        let eigene: SocketAddrV4 = "127.0.0.2:50000".parse().expect("Adresse");
        let am_anderen_port: SocketAddrV4 = "127.0.0.2:50002".parse().expect("Adresse");

        assert_eq!(
            offen_in(TABELLE, 0x1F90, &[eigene, am_anderen_port]),
            [eigene]
        );
    }
}
