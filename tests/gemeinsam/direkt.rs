//! Direktgespräche im Test (R00050, ADR-0055): `anna@127.0.0.2`, `ben@127.0.0.3` und `cora@127.0.0.1` als die drei
//! Akteure der Angriffsmatrix (K, K, A1), die Routen unter `/api/direkt/{kennung}/…` und die Ordner unter `direkt/`.

use super::senden::{beitreten, ordner, starten};
use super::{Absender, ApiAntwort, Instanz, api_antwort};
use serde_json::Value;
use std::path::Path;

pub const ANNA: &str = "anna@127.0.0.2";
pub const BEN: &str = "ben@127.0.0.3";
pub const CORA: &str = "cora@127.0.0.1";
/// Der Ordner des Paars anna ↔ ben (sortiert).
pub const ANNA_BEN: &str = "anna@127.0.0.2+ben@127.0.0.3";

/// Die drei Teilnehmer mit Token und eigener Absenderadresse.
pub struct Runde {
    pub instanz: Instanz,
    pub wurzel: tempfile::TempDir,
    pub anna: Teilnehmer,
    pub ben: Teilnehmer,
    pub cora: Teilnehmer,
}

pub struct Teilnehmer {
    pub absender: Absender,
    pub token: String,
}

/// Instanz mit `channels/allgemein` und `channels/technik`, dazu die drei Teilnehmer beigetreten.
pub async fn runde(umgebung: &[(&str, &str)]) -> Runde {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), umgebung);
    let beitritt = |name: &'static str, adresse: &'static str| {
        let absender = Absender::von(adresse);
        let basis = instanz.adresse.clone();
        async move {
            let token = beitreten(&absender, &basis, name, None).await;
            Teilnehmer { absender, token }
        }
    };
    let anna = beitritt("anna", "127.0.0.2").await;
    let ben = beitritt("ben", "127.0.0.3").await;
    let cora = beitritt("cora", "127.0.0.1").await;
    Runde {
        instanz,
        wurzel,
        anna,
        ben,
        cora,
    }
}

impl Teilnehmer {
    /// `POST /api/direkt/<segment>/nachrichten` mit JSON-Körper; `segment` wird nicht kodiert.
    pub async fn senden(&self, basis: &str, segment: &str, koerper: &Value) -> ApiAntwort {
        self.post(basis, segment, "nachrichten", koerper).await
    }

    /// `POST /api/direkt/<segment>/<unterpfad>` mit JSON-Körper.
    pub async fn post(
        &self,
        basis: &str,
        segment: &str,
        unterpfad: &str,
        koerper: &Value,
    ) -> ApiAntwort {
        api_antwort(
            self.absender
                .anfrage(
                    reqwest::Method::POST,
                    format!("{basis}api/direkt/{segment}/{unterpfad}"),
                )
                .header("X-LocalchatA-Sitzung", &self.token)
                .json(koerper),
        )
        .await
    }

    /// `GET /api/direkt/<segment>/nachrichten`.
    pub async fn lesen(&self, basis: &str, segment: &str) -> ApiAntwort {
        api_antwort(
            self.absender
                .anfrage(
                    reqwest::Method::GET,
                    format!("{basis}api/direkt/{segment}/nachrichten"),
                )
                .header("X-LocalchatA-Sitzung", &self.token),
        )
        .await
    }
}

/// `GET /api/direkt/<segment>/nachrichten` ohne Sitzung.
pub async fn lesen_ohne_sitzung(absender: &Absender, basis: &str, segment: &str) -> ApiAntwort {
    api_antwort(absender.anfrage(
        reqwest::Method::GET,
        format!("{basis}api/direkt/{segment}/nachrichten"),
    ))
    .await
}

/// Die Namen unter `direkt/` (leer, wenn es den Ordner nicht gibt).
pub fn direkt_eintraege(ordner: &Path) -> Vec<String> {
    let mut namen: Vec<String> = std::fs::read_dir(ordner.join("direkt"))
        .map(|liste| {
            liste
                .flatten()
                .map(|eintrag| eintrag.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    namen.sort();
    namen
}

/// Die heutige Tagesdatei eines Paars (leer, wenn es sie nicht gibt).
pub fn paarinhalt(ordner: &Path, paarordner: &str) -> Vec<u8> {
    std::fs::read(
        ordner
            .join("direkt")
            .join(paarordner)
            .join(format!("{}.log", super::senden::HEUTE)),
    )
    .unwrap_or_default()
}
