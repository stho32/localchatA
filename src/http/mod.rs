//! HTTP-Schnittstelle: Router an der Systemgrenze (A00004).

mod beitreten;
mod eingebettet;
mod gesundheit;
mod grenzschutz;
mod ich;
mod oberflaeche;
mod teilnehmer;
mod teilnehmerliste;

pub use grenzschutz::Grenzschutz;

use crate::anwesenheit::Anwesenheit;
use crate::gespraeche::Gespraeche;
use axum::Router;
use axum::middleware;
use axum::routing::{get, post};
use std::path::PathBuf;
use std::sync::Arc;

/// Was die Handler über die laufende Instanz wissen: ihren Ordner, die geladenen Gespräche, die
/// Anwesenheit und die Namen, unter denen sie antworten darf.
#[derive(Clone)]
pub struct Zustand {
    pub ordner: PathBuf,
    pub gespraeche: Arc<Gespraeche>,
    pub anwesenheit: Arc<Anwesenheit>,
    pub grenzschutz: Grenzschutz,
}

/// Alle Routen unter der Schicht `grenzschutz` (ADR-0014) — sie ist die äußerste, auch für 404 und 405.
pub fn router(zustand: Zustand) -> Router {
    let grenzschutz = Arc::new(zustand.grenzschutz.clone());
    Router::new()
        .route("/", get(oberflaeche::wurzel))
        .route("/web/{datei}", get(oberflaeche::datei))
        .route("/api/gesundheit", get(gesundheit::beantworten))
        .route("/api/beitreten", post(beitreten::beantworten))
        .route("/api/ich", get(ich::beantworten))
        .route("/api/teilnehmer", get(teilnehmerliste::beantworten))
        .with_state(Arc::new(zustand))
        .layer(middleware::from_fn_with_state(
            grenzschutz,
            grenzschutz::schicht,
        ))
}

#[cfg(test)]
pub(crate) mod testserver {
    use super::{Grenzschutz, Zustand, router};
    use crate::anwesenheit::Anwesenheit;
    use crate::gespraeche::{Dateiablage, Gespraeche};
    use crate::start::ausliefern::ausliefern;
    use crate::start::ende::Ende;
    use crate::start::verbindungen::Verbindungsgrenzen;
    use crate::uhr::Systemuhr;
    use crate::zufall::Betriebssystemzufall;
    use axum::Router;
    use std::path::Path;
    use std::sync::Arc;
    use std::time::Duration;

    /// Startet den Router im Testprozess auf einem freien Port und liefert die Basisadresse.
    pub async fn starten(ordner: &Path) -> String {
        app_starten(router(zustand(ordner))).await
    }

    /// Ein Zustand wie im Programm: Gespräche aus einem leeren Wegwerf-Ordner (`ordner` steht nur in den
    /// Antworten), Anwesenheit mit Systemuhr, Betriebssystem-Zufall und 60 s Gnadenfrist.
    pub fn zustand(ordner: &Path) -> Zustand {
        let leer = tempfile::tempdir().expect("Wegwerf-Ordner");
        let gespraeche =
            Gespraeche::laden(Arc::new(Dateiablage::neu(leer.path())), Arc::new(Systemuhr))
                .expect("leerer Ordner geladen");
        Zustand {
            ordner: ordner.to_path_buf(),
            gespraeche: Arc::new(gespraeche),
            anwesenheit: Arc::new(Anwesenheit::neu(
                Arc::new(Systemuhr),
                Arc::new(Betriebssystemzufall),
                Duration::from_secs(60),
            )),
            grenzschutz: Grenzschutz::neu(None, Vec::new()),
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
        tokio::spawn(ausliefern(listener, app, Ende::nie(), grenzen));
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
