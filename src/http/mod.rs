//! HTTP-Schnittstelle: Router an der Systemgrenze (A00004).

mod abbruch;
mod abweisungsseite;
mod anfrageprotokoll;
mod auskunft;
mod beitreten;
mod channels;
mod eingebettet;
mod ereignisse;
mod frist;
mod gesundheit;
#[cfg(test)]
mod grenzepisoden;
mod grenzschutz;
mod hostnamen;
mod ich;
mod koerper;
mod nachrichten;
mod namensauskunft;
mod oberflaeche;
mod senden;
mod teilnehmer;
mod teilnehmerliste;
#[cfg(test)]
mod testauskunft;
mod verlassen;
mod werkbank;

pub use auskunft::Auskunft;
pub use grenzschutz::Grenzschutz;
pub use namensauskunft::{Echt, Namensauskunft, Tabelle};
pub use werkbank::Werkbank;

use crate::anwesenheit::Anwesenheit;
use crate::gespraeche::Gespraeche;
use crate::live::bus::Bus;
use crate::protokoll::Protokoll;
use crate::protokoll::testpanik::Testpanik;
use crate::schreibkonto::Schreibkonto;
use crate::start::ende::Ende;
use axum::Router;
use axum::middleware;
use axum::routing::{get, post, put};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

/// Was die Handler über die laufende Instanz wissen: ihren Ordner, die geladenen Gespräche, die
/// Anwesenheit, den Ereignisbus und die Namen, unter denen sie antworten darf.
#[derive(Clone)]
pub struct Zustand {
    pub ordner: PathBuf,
    pub gespraeche: Arc<Gespraeche>,
    pub anwesenheit: Arc<Anwesenheit>,
    pub bus: Arc<Bus>,
    /// Abstand der Herzschläge im Ereignisstrom.
    pub herzschlag: Duration,
    /// Ende des Programms — jeder Strom hört es und endet selbst.
    pub ende: Ende,
    pub grenzschutz: Grenzschutz,
    /// Plätze für schwere Arbeit (ADR-0039 Regel 4).
    pub werkbank: Werkbank,
    /// Token-Eimer gegen Flut beim Senden (A00014).
    pub schreibkonto: Arc<Schreibkonto>,
    /// Betriebsprotokoll der Instanz (A00015).
    pub protokoll: Arc<Protokoll>,
    /// Testknopf `LOCALCHATA_TESTPANIK` (nur Debug-Build wirksam).
    pub testpanik: Testpanik,
}

/// Alle Routen unter der Schicht `grenzschutz` (ADR-0014) — sie ist die äußerste, auch für 404 und 405.
pub fn router(zustand: Zustand) -> Router {
    let grenzschutz = (
        Arc::new(zustand.grenzschutz.clone()),
        Arc::clone(&zustand.protokoll),
    );
    let protokoll = Arc::clone(&zustand.protokoll);
    Router::new()
        .route("/", get(oberflaeche::wurzel))
        .route("/web/{datei}", get(oberflaeche::datei))
        .route("/api/gesundheit", get(gesundheit::beantworten))
        .route("/api/beitreten", post(beitreten::beantworten))
        .route(
            "/api/channels",
            get(channels::auflisten).post(channels::anlegen),
        )
        .route(
            "/api/channels/{name}/nachrichten",
            get(nachrichten::beantworten).post(senden::senden),
        )
        .route("/api/channels/{name}/frist", put(frist::setzen))
        .route("/api/ich", get(ich::beantworten))
        .route("/api/teilnehmer", get(teilnehmerliste::beantworten))
        .route("/api/ereignisse", get(ereignisse::beantworten))
        .route("/api/verlassen", post(verlassen::beantworten))
        .with_state(Arc::new(zustand))
        .layer(middleware::from_fn_with_state(
            protokoll,
            anfrageprotokoll::schicht,
        ))
        .layer(middleware::from_fn_with_state(
            grenzschutz,
            grenzschutz::schicht,
        ))
}

#[cfg(test)]
pub(crate) mod testserver {
    use super::{Grenzschutz, Werkbank, Zustand};
    use crate::anwesenheit::Anwesenheit;
    use crate::gespraeche::{Dateiablage, Gespraeche};
    use crate::start::abrisspflege::{Pflege, pflege_aus};
    use crate::start::ausliefern::ausliefern;
    use crate::start::ende::Ende;
    use crate::start::verbindungen::Verbindungsgrenzen;
    use crate::uhr::Systemuhr;
    use crate::zufall::Betriebssystemzufall;
    use axum::Router;
    use std::path::Path;
    use std::sync::Arc;
    use std::time::Duration;

    /// Ein Zustand wie im Programm: Gespräche aus einem leeren Wegwerf-Ordner (`ordner` steht nur in den
    /// Antworten), Anwesenheit mit Systemuhr, Betriebssystem-Zufall und 60 s Gnadenfrist, Herzschlag 5 s wie das
    /// Programm ohne Knopf.
    pub fn zustand(ordner: &Path) -> Zustand {
        let leer = tempfile::tempdir().expect("Wegwerf-Ordner");
        let bus = Arc::new(crate::live::bus::Bus::neu(crate::live::KAPAZITAET));
        let gespraeche = Gespraeche::laden(
            Arc::new(Dateiablage::neu(
                leer.path(),
                Arc::new(crate::protokoll::Protokoll::ohne_ablage()),
            )),
            Arc::new(Systemuhr),
        )
        .expect("leerer Ordner geladen")
        .mit_veroeffentlichen(crate::live::bus::channels_veroeffentlichen(&bus))
        .mit_nachricht_veroeffentlichen(crate::live::bus::nachrichten_veroeffentlichen(&bus));
        let anwesenheit = Anwesenheit::neu(
            Arc::new(Systemuhr),
            Arc::new(Betriebssystemzufall),
            Duration::from_secs(60),
        )
        .mit_veroeffentlichen(crate::live::bus::anwesenheit_veroeffentlichen(&bus));
        Zustand {
            ordner: ordner.to_path_buf(),
            gespraeche: Arc::new(gespraeche),
            anwesenheit: Arc::new(anwesenheit),
            bus,
            herzschlag: Duration::from_secs(5),
            ende: Ende::nie(),
            grenzschutz: Grenzschutz::neu(None, Vec::new()),
            werkbank: Werkbank::aus_kernen(),
            schreibkonto: Arc::new(crate::schreibkonto::Schreibkonto::neu(Arc::new(Systemuhr))),
            protokoll: Arc::new(crate::protokoll::Protokoll::ohne_ablage()),
            testpanik: crate::protokoll::testpanik::Testpanik::keine(),
        }
    }

    /// Liefert `app` im Testprozess auf einem freien Port aus — über dieselbe Annahmeschleife wie das
    /// Programm (ADR-0010, ein Server-Stack), damit Handler die Herkunft sehen — und liefert die Basisadresse.
    pub async fn app_starten(app: Router) -> String {
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("Testport gebunden");
        let adresse = listener.local_addr().expect("Adresse");
        let grenzen = Verbindungsgrenzen::fuer_deskriptoren(None);
        let pflege = Pflege::kernel(
            pflege_aus(Duration::from_secs(5)),
            Arc::new(crate::protokoll::Protokoll::ohne_ablage()),
        );
        tokio::spawn(ausliefern(
            listener,
            app,
            Ende::nie(),
            grenzen,
            pflege,
            Arc::new(crate::protokoll::Protokoll::ohne_ablage()),
        ));
        format!("http://{adresse}/")
    }

    #[tokio::test]
    async fn der_testserver_liefert_ueber_denselben_stack_wie_das_programm_mit_herkunft() {
        use axum::extract::ConnectInfo;
        use axum::routing::get;
        use std::net::SocketAddr;
        let app = Router::new().route(
            "/",
            get(
                |ConnectInfo(gegenstelle): ConnectInfo<SocketAddr>| async move {
                    gegenstelle.ip().to_string()
                },
            ),
        );

        let adresse = app_starten(app).await;
        let antwort = reqwest::get(adresse).await.expect("Antwort");

        assert_eq!(antwort.status(), 200);
        assert_eq!(antwort.text().await.expect("Text"), "127.0.0.1");
    }
}
