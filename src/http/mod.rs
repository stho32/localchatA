//! HTTP-Schnittstelle: Router an der Systemgrenze (A00004).

mod gesundheit;

use axum::Router;
use axum::routing::get;
use std::path::PathBuf;
use std::sync::Arc;

/// Was die Handler über die laufende Instanz wissen.
#[derive(Debug, Clone)]
pub struct Zustand {
    pub ordner: PathBuf,
}

pub fn router(zustand: Zustand) -> Router {
    Router::new()
        .route("/api/gesundheit", get(gesundheit::beantworten))
        .with_state(Arc::new(zustand))
}

#[cfg(test)]
pub(crate) mod testserver {
    use super::{Zustand, router};
    use std::path::Path;

    /// Startet den Router im Testprozess auf einem freien Port und liefert die Basisadresse.
    pub async fn starten(ordner: &Path) -> String {
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("Testport gebunden");
        let adresse = listener.local_addr().expect("Adresse");
        let app = router(Zustand {
            ordner: ordner.to_path_buf(),
        });
        tokio::spawn(async move { axum::serve(listener, app).await });
        format!("http://{adresse}/")
    }
}
