//! Rohverbindungen von einer bestimmten Herkunft (Linux-Loopback 127.0.0.0/8) und die Vorbedingung der Tests an
//! den Verbindungsgrenzen (R00015, „Gemeinsam“).

use tokio::net::{TcpSocket, TcpStream};

/// Die Gesamtgrenze ist `min(512, Deskriptoren / 2)`; unter 1.024 Deskriptoren prüfte ein Test eine andere
/// Grenze — dann Abbruch statt Befund.
pub fn deskriptoren_reichen() {
    let weich = rustix::process::getrlimit(rustix::process::Resource::Nofile).current;
    assert!(
        weich.is_none_or(|grenze| grenze >= 1024),
        "Abbruch, kein Befund: ulimit -n ist {weich:?}, der Test braucht mindestens 1024"
    );
}

/// Eine Rohverbindung zu `127.0.0.1:<port>`, gebunden an die Herkunft `herkunft`.
pub async fn verbinden(port: u16, herkunft: &str) -> TcpStream {
    let socket = TcpSocket::new_v4().expect("Socket");
    socket
        .bind(format!("{herkunft}:0").parse().expect("Herkunft"))
        .expect("gebunden");
    socket
        .connect(format!("127.0.0.1:{port}").parse().expect("Ziel"))
        .await
        .expect("verbunden")
}
