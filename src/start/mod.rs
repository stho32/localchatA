//! Start: vom Aufruf bis zum lauschenden Server (A00003).
//!
//! Reihenfolge: Aufruf lesen → Uhr → Gnadenfrist → Herzschlag → Ordner vorbereiten → sperren → Verlauf laden → Anwesenheit → Bus → binden → Ende-Signal → Takt → Startzeile → Browser → ausliefern bis zum Signal.

pub(crate) mod abrisspflege;
mod aufruf;
pub(crate) mod ausliefern;
mod browser;
pub(crate) mod ende;
mod ordner;
mod port;
mod rechnername;
mod sperre;
mod startfehler;
pub(crate) mod verbindungen;

use abrisspflege::{Pflege, pflege_aus};
use aufruf::{Aufruf, Portwunsch, Startauftrag};
use ende::Ende;
use startfehler::Startfehler;
use verbindungen::Verbindungsgrenzen;

use crate::anwesenheit::{Anwesenheit, gnadenfrist};
use crate::gespraeche::{Dateiablage, Gespraeche};
use crate::http::{self, Grenzschutz, Zustand};
use crate::live::bus::{self, Bus};
use crate::live::{self, herzschlag, takt};
use crate::uhr::{self, Uhr};
use crate::zufall::Betriebssystemzufall;
use aufruf::HILFE;
use std::net::SocketAddr;
use std::path::Path;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

/// Führt einen Aufruf (Argumente ohne Programmnamen) bis zum Ende aus und liefert den Exit-Code:
/// 0 geordnet beendet oder Hilfe, 1 Ordner, Port oder Sperre scheitert, 2 falscher Aufruf oder
/// unlesbare Startzeit in `LOCALCHATA_JETZT`, Gnadenfrist in `LOCALCHATA_GNADENFRIST_MS` bzw. Herzschlag in
/// `LOCALCHATA_HERZSCHLAG_MS`.
pub async fn ausfuehren<I: IntoIterator<Item = String>>(argumente: I) -> ExitCode {
    match Startauftrag::aus_argumenten(argumente) {
        Ok(Aufruf::Hilfe) => {
            println!("{HILFE}");
            ExitCode::SUCCESS
        }
        Ok(Aufruf::Starten(auftrag)) => {
            let startzeit = std::env::var_os(uhr::STARTZEIT_VARIABLE);
            let uhr = match uhr::startzeit_lesen(startzeit.as_deref()).and_then(uhr::aus_umgebung) {
                Ok(uhr) => uhr,
                Err(fehler) => {
                    eprintln!("{fehler}");
                    return ExitCode::from(2);
                }
            };
            let gnadenfrist_wert = std::env::var_os(gnadenfrist::VARIABLE);
            let gnadenfrist = match gnadenfrist::aus_umgebung(gnadenfrist_wert.as_deref()) {
                Ok(gnadenfrist) => gnadenfrist,
                Err(fehler) => {
                    eprintln!("{fehler}");
                    return ExitCode::from(2);
                }
            };
            let herzschlag_wert = std::env::var_os(herzschlag::VARIABLE);
            let herzschlag = match herzschlag::aus_umgebung(herzschlag_wert.as_deref()) {
                Ok(herzschlag) => herzschlag,
                Err(fehler) => {
                    eprintln!("{fehler}");
                    return ExitCode::from(2);
                }
            };
            match starten(auftrag, uhr, gnadenfrist, herzschlag).await {
                Ok(()) => ExitCode::SUCCESS,
                Err(fehler) => {
                    eprintln!("{fehler}");
                    ExitCode::FAILURE
                }
            }
        }
        Err(fehler) => {
            eprintln!("{fehler}");
            ExitCode::from(2)
        }
    }
}

async fn starten(
    auftrag: Startauftrag,
    uhr: Box<dyn Uhr>,
    gnadenfrist: Duration,
    herzschlag: Duration,
) -> Result<(), Startfehler> {
    let uhr: Arc<dyn Uhr> = Arc::from(uhr);
    let ordner = ordner::ordner_vorbereiten(&auftrag.ordner)?;
    let _sperre = sperre::ordner_sperren(&ordner)?;
    let gespraeche = verlauf_laden(&ordner, Arc::clone(&uhr))?;
    let bus = Arc::new(Bus::neu(live::KAPAZITAET));
    let anwesenheit = Arc::new(
        Anwesenheit::neu(
            Arc::clone(&uhr),
            Arc::new(Betriebssystemzufall),
            gnadenfrist,
        )
        .mit_veroeffentlichen(bus::anwesenheit_veroeffentlichen(&bus)),
    );
    let listener = port::listener_binden(&auftrag.host, auftrag.port).await?;
    let adresse = listener.local_addr().map_err(|fehler| Startfehler::Port {
        host: auftrag.host.clone(),
        port: 0,
        grund: fehler.to_string(),
    })?;
    let ende = ende_einrichten();
    tokio::spawn(takt::takten(
        Arc::clone(&anwesenheit),
        uhr,
        ende.clone(),
        takt::PERIODE,
    ));
    startzeile_schreiben(&ordner, adresse);
    if auftrag.browser_oeffnen {
        browser::oeffnen(adresse);
    }
    let app = http::router(Zustand {
        ordner,
        gespraeche: Arc::new(gespraeche),
        anwesenheit,
        bus,
        herzschlag,
        ende: ende.clone(),
        grenzschutz: Grenzschutz::neu(rechnername::lesen(), auftrag.erreichbar_als),
    });
    ausliefern::ausliefern(
        listener,
        app,
        ende,
        Verbindungsgrenzen::fuer_diesen_prozess(),
        Pflege::kernel(pflege_aus(herzschlag)),
    )
    .await;
    Ok(())
}

/// Lädt den Verlauf vor dem Binden — ab der Startzeile ist er vollständig da (A00003).
/// Jeder Defekt steht als eine Zeile `<datei>:<zeile>` auf stderr (X00004).
fn verlauf_laden(ordner: &Path, uhr: Arc<dyn Uhr>) -> Result<Gespraeche, Startfehler> {
    let gespraeche =
        Gespraeche::laden(Arc::new(Dateiablage::neu(ordner)), uhr).map_err(|fehler| {
            Startfehler::Laden {
                ordner: ordner.to_path_buf(),
                grund: fehler.to_string(),
            }
        })?;
    for defekt in gespraeche.defekte() {
        eprintln!("{defekt}");
    }
    Ok(gespraeche)
}

/// Die einzige Zeile auf stdout; Tests und Director lesen die Adresse daraus (X00001).
fn startzeile_schreiben(ordner: &Path, adresse: SocketAddr) {
    println!(
        "localchatA läuft für {} unter http://{adresse}/",
        ordner.display()
    );
}

/// Das Ende-Signal für alle: trifft Strg+C oder SIGTERM ein, wird das Ende gemeldet.
fn ende_einrichten() -> Ende {
    let (melden, ende) = Ende::neu();
    let beenden = beenden_einrichten();
    tokio::spawn(async move {
        beenden.await;
        melden.melden();
    });
    ende
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
