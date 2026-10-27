//! HTTP-Schnittstelle: Router an der Systemgrenze (A00004).

mod eingebettet;
mod gesundheit;
mod grenzschutz;
mod oberflaeche;

pub use grenzschutz::Grenzschutz;

use crate::gespraeche::Gespraeche;
use axum::Router;
use axum::middleware;
use axum::routing::get;
use std::path::PathBuf;
use std::sync::Arc;

/// Was die Handler über die laufende Instanz wissen: ihren Ordner, die geladenen Gespräche und die
/// Namen, unter denen sie antworten darf.
#[derive(Clone)]
pub struct Zustand {
    pub ordner: PathBuf,
    pub gespraeche: Arc<Gespraeche>,
    pub grenzschutz: Grenzschutz,
}

/// Alle Routen unter der Schicht `grenzschutz` (ADR-0014) — sie ist die äußerste, auch für 404 und 405.
pub fn router(zustand: Zustand) -> Router {
    let grenzschutz = Arc::new(zustand.grenzschutz.clone());
    Router::new()
        .route("/", get(oberflaeche::wurzel))
        .route("/web/{datei}", get(oberflaeche::datei))
        .route("/api/gesundheit", get(gesundheit::beantworten))
        .with_state(Arc::new(zustand))
        .layer(middleware::from_fn_with_state(
            grenzschutz,
            grenzschutz::schicht,
        ))
}

#[cfg(test)]
pub(crate) mod testserver {
    use super::{Grenzschutz, Zustand, router};
    use crate::gespraeche::{Dateiablage, Gespraeche};
    use crate::uhr::Systemuhr;
    use std::path::Path;
    use std::sync::Arc;

    /// Startet den Router im Testprozess auf einem freien Port und liefert die Basisadresse;
    /// die Gespräche kommen aus einem leeren Wegwerf-Ordner, `ordner` steht nur in den Antworten.
    pub async fn starten(ordner: &Path) -> String {
        let leer = tempfile::tempdir().expect("Wegwerf-Ordner");
        let gespraeche =
            Gespraeche::laden(Arc::new(Dateiablage::neu(leer.path())), Arc::new(Systemuhr))
                .expect("leerer Ordner geladen");
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("Testport gebunden");
        let adresse = listener.local_addr().expect("Adresse");
        let app = router(Zustand {
            ordner: ordner.to_path_buf(),
            gespraeche: Arc::new(gespraeche),
            grenzschutz: Grenzschutz::neu(None, Vec::new()),
        });
        tokio::spawn(async move { axum::serve(listener, app).await });
        format!("http://{adresse}/")
    }
}
