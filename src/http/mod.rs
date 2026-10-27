//! HTTP-Schnittstelle: Router an der Systemgrenze (A00004).

mod eingebettet;
mod gesundheit;
mod oberflaeche;

use crate::gespraeche::Gespraeche;
use axum::Router;
use axum::routing::get;
use std::path::PathBuf;
use std::sync::Arc;

/// Was die Handler über die laufende Instanz wissen: ihren Ordner und die geladenen Gespräche.
#[derive(Clone)]
pub struct Zustand {
    pub ordner: PathBuf,
    pub gespraeche: Arc<Gespraeche>,
}

pub fn router(zustand: Zustand) -> Router {
    Router::new()
        .route("/", get(oberflaeche::wurzel))
        .route("/web/{datei}", get(oberflaeche::datei))
        .route("/api/gesundheit", get(gesundheit::beantworten))
        .with_state(Arc::new(zustand))
}

#[cfg(test)]
pub(crate) mod testserver {
    use super::{Zustand, router};
    use crate::gespraeche::{Dateiablage, Gespraeche};
    use crate::uhr::Systemuhr;
    use std::path::Path;
    use std::sync::Arc;

    /// Startet den Router im Testprozess auf einem freien Port und liefert die Basisadresse;
    /// die Gespräche kommen aus einem leeren Wegwerf-Ordner, `ordner` steht nur in den Antworten.
    pub async fn starten(ordner: &Path) -> String {
        let leer = tempfile::tempdir().expect("Wegwerf-Ordner");
        let gespraeche =
            Gespraeche::laden(Box::new(Dateiablage::neu(leer.path())), Box::new(Systemuhr))
                .expect("leerer Ordner geladen");
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("Testport gebunden");
        let adresse = listener.local_addr().expect("Adresse");
        let app = router(Zustand {
            ordner: ordner.to_path_buf(),
            gespraeche: Arc::new(gespraeche),
        });
        tokio::spawn(async move { axum::serve(listener, app).await });
        format!("http://{adresse}/")
    }
}
