//! Start: vom Aufruf bis zum lauschenden Server (A00003).
//!
//! Reihenfolge: Aufruf lesen → Ordner vorbereiten → sperren → binden → Startzeile → Browser → ausliefern bis zum Signal.

mod aufruf;
mod browser;
mod ordner;
mod port;
mod sperre;
mod startfehler;

use aufruf::{Aufruf, Portwunsch, Startauftrag};
use startfehler::Startfehler;

use crate::http::{self, Zustand};
use aufruf::HILFE;
use std::net::SocketAddr;
use std::path::Path;
use std::process::ExitCode;

/// Führt einen Aufruf (Argumente ohne Programmnamen) bis zum Ende aus und liefert den Exit-Code:
/// 0 geordnet beendet oder Hilfe, 1 Ordner, Port oder Sperre scheitert, 2 falscher Aufruf.
pub async fn ausfuehren<I: IntoIterator<Item = String>>(argumente: I) -> ExitCode {
    match Startauftrag::aus_argumenten(argumente) {
        Ok(Aufruf::Hilfe) => {
            println!("{HILFE}");
            ExitCode::SUCCESS
        }
        Ok(Aufruf::Starten(auftrag)) => match starten(auftrag).await {
            Ok(()) => ExitCode::SUCCESS,
            Err(fehler) => {
                eprintln!("{fehler}");
                ExitCode::FAILURE
            }
        },
        Err(fehler) => {
            eprintln!("{fehler}");
            ExitCode::from(2)
        }
    }
}

async fn starten(auftrag: Startauftrag) -> Result<(), Startfehler> {
    let ordner = ordner::ordner_vorbereiten(&auftrag.ordner)?;
    let _sperre = sperre::ordner_sperren(&ordner)?;
    let listener = port::listener_binden(&auftrag.host, auftrag.port).await?;
    let adresse = listener.local_addr().map_err(|fehler| Startfehler::Port {
        host: auftrag.host.clone(),
        port: 0,
        grund: fehler.to_string(),
    })?;
    let beenden = beenden_einrichten();
    startzeile_schreiben(&ordner, adresse);
    if auftrag.browser_oeffnen {
        browser::oeffnen(adresse);
    }
    let app = http::router(Zustand { ordner });
    axum::serve(listener, app)
        .with_graceful_shutdown(beenden)
        .await
        .map_err(|fehler| Startfehler::Port {
            host: auftrag.host,
            port: adresse.port(),
            grund: fehler.to_string(),
        })
}

/// Die einzige Zeile auf stdout; Tests und Director lesen die Adresse daraus (X00001).
fn startzeile_schreiben(ordner: &Path, adresse: SocketAddr) {
    println!(
        "localchatA läuft für {} unter http://{adresse}/",
        ordner.display()
    );
}

/// Richtet den Empfang von Strg+C und SIGTERM sofort ein — vor der Startzeile, denn ab ihr gilt die Instanz
/// als bereit, und ein Signal ab da muss geordnet beenden. Das Ergebnis endet, sobald eines eintrifft.
#[cfg(unix)]
fn beenden_einrichten() -> impl Future<Output = ()> + Send + 'static {
    use tokio::signal::unix::SignalKind;
    let strg_c = signal_einrichten(SignalKind::interrupt(), "Strg+C");
    let sigterm = signal_einrichten(SignalKind::terminate(), "SIGTERM");
    async move {
        tokio::select! {
            _ = eintreffen(strg_c) => {}
            _ = eintreffen(sigterm) => {}
        }
    }
}

/// Lässt sich ein Signal nicht abfangen, steht ein Hinweis auf stderr; beendet wird dann nur über das andere.
#[cfg(unix)]
fn signal_einrichten(
    art: tokio::signal::unix::SignalKind,
    name: &str,
) -> Option<tokio::signal::unix::Signal> {
    match tokio::signal::unix::signal(art) {
        Ok(signal) => Some(signal),
        Err(fehler) => {
            eprintln!("{name} lässt sich nicht abfangen ({fehler})");
            None
        }
    }
}

#[cfg(unix)]
async fn eintreffen(signal: Option<tokio::signal::unix::Signal>) {
    match signal {
        Some(mut signal) => {
            signal.recv().await;
        }
        None => std::future::pending::<()>().await,
    }
}

/// Wie unter Unix, nur mit Strg+C; lässt es sich nicht abfangen, wird nie beendet statt sofort.
#[cfg(windows)]
fn beenden_einrichten() -> impl Future<Output = ()> + Send + 'static {
    let strg_c = tokio::signal::windows::ctrl_c();
    async move {
        match strg_c {
            Ok(mut strg_c) => {
                strg_c.recv().await;
            }
            Err(fehler) => {
                eprintln!("Strg+C lässt sich nicht abfangen ({fehler})");
                std::future::pending::<()>().await;
            }
        }
    }
}
