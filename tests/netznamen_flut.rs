//! Nacharbeit Karte #35, Security-Review W1, Linie L2 (R00024 US-13): der Rückwärtsname der angesprochenen
//! Adresse bleibt erlaubt, während 20 parallele Anfragen ohne Konto mit immer neuen Namen fluten.
#![cfg(unix)]

mod gemeinsam;

use gemeinsam::starten_mit;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// `GET /api/gesundheit` über `ziel` mit `Host: <host>`; der Status oder 0, wenn keine Antwort kam.
fn status(ziel: SocketAddr, host: &str) -> u16 {
    let Ok(mut verbindung) = TcpStream::connect_timeout(&ziel, Duration::from_secs(5)) else {
        return 0;
    };
    let _ = verbindung.set_read_timeout(Some(Duration::from_secs(5)));
    let anfrage =
        format!("GET /api/gesundheit HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n");
    if verbindung.write_all(anfrage.as_bytes()).is_err() {
        return 0;
    }
    let mut antwort = Vec::new();
    let _ = verbindung.read_to_end(&mut antwort);
    String::from_utf8_lossy(&antwort)
        .split(' ')
        .nth(1)
        .and_then(|status| status.parse().ok())
        .unwrap_or(0)
}

/// Ein neuer Name je Aufruf, ohne Muster, das ein Cache-Treffer sein könnte.
fn zufallsname(faden: usize, nummer: usize) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|seit| seit.subsec_nanos())
        .unwrap_or_default();
    format!("r{faden}x{nummer}x{nanos}")
}

/// Abnahme-Probe W1 (L2): „chat.fritz.box über 127.0.0.2 → 200; 10 s lang 20 parallele Anfragen über 127.0.0.1
/// mit Host r<zufall>:P; gleichzeitig alle 100 ms chat.fritz.box über 127.0.0.2 → jede 200.“
#[test]
fn der_router_name_bleibt_offen_waehrend_jemand_mit_namen_flutet() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let instanz = starten_mit(
        wurzel.path(),
        &["--port", "0", "--host", "0.0.0.0", "--kein-browser"],
        &[("LOCALCHATA_TESTNAMEN", "127.0.0.2>chat.fritz.box")],
    );
    let p = instanz.port();
    let router: SocketAddr = format!("127.0.0.2:{p}").parse().expect("Adresse");
    let flutziel: SocketAddr = format!("127.0.0.1:{p}").parse().expect("Adresse");
    let router_host = format!("chat.fritz.box:{p}");
    assert_eq!(status(router, &router_host), 200, "vor der Flut");

    let ende = Arc::new(AtomicBool::new(false));
    let geflutet = Arc::new(AtomicUsize::new(0));
    let fluter: Vec<_> = (0..20)
        .map(|faden| {
            let ende = Arc::clone(&ende);
            let geflutet = Arc::clone(&geflutet);
            std::thread::spawn(move || {
                let mut nummer = 0;
                while !ende.load(Ordering::SeqCst) {
                    nummer += 1;
                    status(flutziel, &format!("{}:{p}", zufallsname(faden, nummer)));
                    geflutet.fetch_add(1, Ordering::SeqCst);
                }
            })
        })
        .collect();

    let beginn = Instant::now();
    let mut antworten = Vec::new();
    while beginn.elapsed() < Duration::from_secs(10) {
        let takt = Instant::now();
        antworten.push(status(router, &router_host));
        std::thread::sleep(Duration::from_millis(100).saturating_sub(takt.elapsed()));
    }
    ende.store(true, Ordering::SeqCst);
    for faden in fluter {
        faden.join().expect("Fluter");
    }

    let abgewiesen = antworten.iter().filter(|status| **status != 200).count();
    assert!(antworten.len() >= 50, "{} Anfragen", antworten.len());
    assert!(geflutet.load(Ordering::SeqCst) > 1000, "Flut zu schwach");
    assert_eq!(
        abgewiesen,
        0,
        "{abgewiesen} von {} Anfragen an chat.fritz.box nicht 200: {antworten:?}",
        antworten.len()
    );
}
