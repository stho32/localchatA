//! Ausliefern bis zum Signal (B0017): eigene Annahmeschleife statt `axum::serve`, weil diese keine
//! Kopf-Zeitgrenze, keine Verbindungsgrenze und kein Ende nach Frist kennt (Security-Review, CWE-400).

use super::verbindungen::{Belegung, Platz, Verbindungsgrenzen};
use axum::Router;
use hyper::server::conn::http1;
use hyper_util::rt::{TokioIo, TokioTimer};
use hyper_util::service::TowerToHyperService;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;
use tokio::task::JoinSet;

/// So lange darf ein Request-Kopf vom ersten Byte bis zur Leerzeile brauchen (gesamt, nicht je Lesevorgang).
/// Hyper startet die Uhr auch nach jeder Antwort neu: sie ist zugleich die Leerlaufgrenze zwischen Requests.
/// Laufende Antworten (Live-Kanal) zählen nicht, die Uhr läuft erst, wenn der nächste Kopf erwartet wird.
pub const KOPF_ZEITGRENZE: Duration = Duration::from_secs(10);
/// So lange bekommen offene Verbindungen nach dem Signal, dann werden sie abgebrochen.
pub const ENDE_FRIST: Duration = Duration::from_secs(5);
/// Pause nach einem Fehler der Annahme, damit ein dauerhafter Fehler die Schleife nicht heiß laufen lässt.
const ANNAHMEPAUSE: Duration = Duration::from_millis(50);

/// Nimmt Verbindungen an und bedient sie, bis `beenden` endet; danach endet jede offene Verbindung
/// geordnet oder spätestens nach `ENDE_FRIST`.
pub async fn ausliefern(
    listener: TcpListener,
    app: Router,
    beenden: impl Future<Output = ()>,
    grenzen: Verbindungsgrenzen,
) {
    let belegung = Belegung::neu(grenzen);
    let (ende_melden, ende) = watch::channel(false);
    let mut verbindungen = JoinSet::new();
    tokio::pin!(beenden);
    loop {
        tokio::select! {
            () = &mut beenden => break,
            Some(_) = verbindungen.join_next(), if !verbindungen.is_empty() => {}
            angenommen = listener.accept() => match angenommen {
                Ok((strom, gegenstelle)) => {
                    // Ohne Platz fällt der Strom hier und die Verbindung ist sofort geschlossen.
                    if let Some(platz) = belegung.zulassen(gegenstelle.ip()) {
                        verbindungen.spawn(bedienen(strom, app.clone(), platz, ende.clone()));
                    }
                }
                Err(_) => tokio::time::sleep(ANNAHMEPAUSE).await,
            },
        }
    }
    drop(listener);
    let _ = ende_melden.send(true);
    let alle_beendet = async { while verbindungen.join_next().await.is_some() {} };
    if tokio::time::timeout(ENDE_FRIST, alle_beendet)
        .await
        .is_err()
    {
        verbindungen.shutdown().await;
    }
}

/// Bedient eine Verbindung mit HTTP/1.1; nach dem Ende-Signal keine neuen Requests mehr.
/// Das Ergebnis der Verbindung (abgelaufene Kopf-Zeitgrenze, Abbruch der Gegenstelle) betrifft nur sie
/// selbst und wird bewusst nicht gemeldet — eine Zeile je Fremdverbindung würde stderr fluten.
async fn bedienen(strom: TcpStream, app: Router, _platz: Platz, mut ende: watch::Receiver<bool>) {
    let verbindung = http1::Builder::new()
        .timer(TokioTimer::new())
        .header_read_timeout(KOPF_ZEITGRENZE)
        .serve_connection(TokioIo::new(strom), TowerToHyperService::new(app));
    tokio::pin!(verbindung);
    tokio::select! {
        _ = verbindung.as_mut() => return,
        _ = ende.wait_for(|beendet| *beendet) => {}
    }
    verbindung.as_mut().graceful_shutdown();
    let _ = verbindung.await;
}
