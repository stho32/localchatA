//! Verwerfen (B0430, ADR-0046 Regel 3 und Ergänzung „Karte #9 Gate“ Regel 5): `DELETE /api/anhaenge/{id}` mit
//! Sitzung → immer 204. Die eigene ID: ein laufender `PUT` endet als Abbruch (er räumt seine Teildatei selbst),
//! Teil- bzw. fertige Datei weg, Reservierung sofort frei. Fremde, unbekannte und schon gebundene IDs bleiben
//! unberührt — keine Antwort verrät etwas über sie (M9).

use super::super::Zustand;
use super::super::abbruch::{Abgebrochen, Grenzort, abbruch_beantworten};
use super::super::teilnehmer::Teilnehmer;
use crate::gespraeche::anhangid::Anhangid;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use std::sync::Arc;

const ROUTE: &str = "DELETE /api/anhaenge/{id}";

pub async fn verwerfen(
    teilnehmer: Teilnehmer,
    State(zustand): State<Arc<Zustand>>,
    Path(id): Path<String>,
) -> Response {
    let Some(id) = Anhangid::lesen(&id) else {
        return StatusCode::NO_CONTENT.into_response();
    };
    let gespraeche = Arc::clone(&zustand.gespraeche);
    let testpanik = zustand.testpanik.clone();
    let kennung = teilnehmer.kennung.clone();
    let verworfen = tokio::task::spawn_blocking(move || {
        testpanik.loeschen();
        gespraeche.anhang_verwerfen(&kennung, &id)
    })
    .await;
    match verworfen {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(fehler) => abbruch_beantworten(
            &zustand.protokoll,
            &Abgebrochen::from(fehler),
            &Grenzort {
                route: ROUTE,
                operation: "anhaenge_verwerfen",
                kennung: Some(teilnehmer.kennung.als_text()),
                channel: None,
            },
            "Anhang nicht verworfen",
        ),
    }
}

#[cfg(test)]
mod tests {
    use crate::http::loeschaufbau::{aufbau, aufbau_mit};
    use crate::protokoll::testpanik::Testpanik;
    use crate::zufall::FesteFolge;
    use axum::http::StatusCode;
    use std::ffi::OsStr;
    use std::sync::Arc;

    /// R00043 B0667 (ADR-0048 K3): eine Panik im Blocking-Teil des Verwerfens ist kein stilles 204, sondern 500 ohne
    /// Fremdtext und genau ein `ERROR programmfehler … operation=anhaenge_verwerfen grund=panik`.
    #[tokio::test(flavor = "multi_thread")]
    async fn eine_panik_beim_verwerfen_wird_beantwortet_und_protokolliert() {
        let panik = Testpanik::aus_wert(Some(OsStr::new("loeschen"))).expect("gültig");
        let aufbau = aufbau_mit(Arc::new(FesteFolge::neu(1..=100)), panik);
        let (_, id) = aufbau.nachricht_mit_anhang("beta@127.0.0.1");

        let antwort = aufbau.anhaenge_verwerfen("beta@127.0.0.1", &id).await;

        assert_eq!(antwort.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let eintraege = aufbau.zeilen_mit(" programmfehler ");
        assert_eq!(eintraege.len(), 1, "{eintraege:#?}");
        assert!(
            eintraege[0].contains(" ERROR programmfehler "),
            "{eintraege:#?}"
        );
        assert!(
            eintraege[0].contains("route=\"DELETE /api/anhaenge/{id}\" operation=anhaenge_verwerfen status=500 kennung=beta@127.0.0.1 grund=panik"),
            "{eintraege:#?}"
        );
    }

    /// Ohne Panik bleibt es bei 204 — auch für unbekannte IDs (M9).
    #[tokio::test(flavor = "multi_thread")]
    async fn ohne_programmfehler_antwortet_das_verwerfen_weiter_mit_204_und_ohne_eintrag() {
        let aufbau = aufbau();
        let (_, id) = aufbau.nachricht_mit_anhang("beta@127.0.0.1");

        let antwort = aufbau.anhaenge_verwerfen("beta@127.0.0.1", &id).await;

        assert_eq!(antwort.status(), StatusCode::NO_CONTENT);
        assert!(aufbau.zeilen_mit(" programmfehler ").is_empty());
    }
}
