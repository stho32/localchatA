//! Start: vom Aufruf bis zum lauschenden Server (A00003).
//!
//! Reihenfolge: Aufruf lesen → Ordner vorbereiten → sperren → binden → Startzeile → Browser → ausliefern bis zum Signal.

mod aufruf;
pub mod browser;
pub mod ordner;
pub mod port;
pub mod sperre;
mod startfehler;

pub use aufruf::{Aufruf, Aufruffehler, Portwunsch, Startauftrag};
pub use startfehler::Startfehler;

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
    startzeile_schreiben(&ordner, adresse);
    if auftrag.browser_oeffnen {
        browser::oeffnen(adresse);
    }
    let app = http::router(Zustand { ordner });
    axum::serve(listener, app)
        .with_graceful_shutdown(beenden_verlangt())
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

/// Strg+C oder SIGTERM verlangen ein geordnetes Ende.
async fn beenden_verlangt() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        match signal(SignalKind::terminate()) {
            Ok(mut sigterm) => {
                tokio::select! {
                    _ = strg_c() => {}
                    _ = sigterm.recv() => {}
                }
            }
            Err(fehler) => {
                eprintln!("SIGTERM lässt sich nicht abfangen ({fehler}) — Ende nur mit Strg+C");
                strg_c().await;
            }
        }
    }
    #[cfg(not(unix))]
    strg_c().await;
}

/// Wartet auf Strg+C; lässt es sich nicht abfangen, wird nie beendet statt sofort.
async fn strg_c() {
    if let Err(fehler) = tokio::signal::ctrl_c().await {
        eprintln!("Strg+C lässt sich nicht abfangen ({fehler})");
        std::future::pending::<()>().await;
    }
}
