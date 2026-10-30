//! Aufbau für die Tests der Löschwege und der Anhang-Handler (Karte #73): ein Zustand mit Gesprächen auf der
//! Speicherablage, der Anhang-Attrappe, einer Hochladebank, dem Protokoll in der Speicher-Senke und den Handlern
//! als Aufruf ohne HTTP — so lässt sich die Zukunft eines Handlers an jedem `await` fallen lassen.

use super::testserver;
use super::{Zustand, anhaenge, nachricht_loeschen};
use crate::anwesenheit::Art;
use crate::gespraeche::anhangeingang::Kopf;
use crate::gespraeche::anhangid::Anhangid;
use crate::gespraeche::anhangspeicher::Anhangspeicher;
use crate::gespraeche::{Gesendet, Gespraech, Gespraeche, Kennung, Speicherablage};
use crate::http::teilnehmer::Teilnehmer;
use crate::live::bus::{Abonnement, Empfang, Ereignis};
use crate::name::Name;
use crate::protokoll::Stufe;
use crate::protokoll::senke::Speicher;
use crate::protokoll::testhilfe::{im_speicher, zeit};
use crate::protokoll::testpanik::Testpanik;
use crate::uhr::{GestellteUhr, Zeit};
use crate::zufall::{FesteFolge, Zufall};
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::http::header::CONTENT_TYPE;
use axum::response::Response;
use std::sync::{Arc, Barrier};
use std::time::Duration;

pub const MIB: u64 = 1_048_576;

pub struct Aufbau {
    pub zustand: Arc<Zustand>,
    pub zeilen: Speicher,
    pub anhaenge: Arc<Anhangspeicher>,
    _ordner: tempfile::TempDir,
}

/// Channel `allgemein`, Anhang-Quote 10 MiB, Uhr 2026-10-29 12:00, Stufe `Debug`.
pub fn aufbau() -> Aufbau {
    aufbau_mit(Arc::new(FesteFolge::neu(1..=100)), Testpanik::keine())
}

/// Wie `aufbau`, mit eigenem Zufall (ein Zufall, der in Panik gerät, bringt `anmelden` zum Abbruch) und Testknopf.
pub fn aufbau_mit(zufall: Arc<dyn Zufall>, testpanik: Testpanik) -> Aufbau {
    let ordner = tempfile::tempdir().expect("Wegwerf-Ordner");
    let ablage = Arc::new(Speicherablage::neu());
    ablage.gespraech_anlegen(&Gespraech::channel("allgemein"));
    let anhaenge = Arc::new(Anhangspeicher::neu());
    let uhr = Arc::new(GestellteUhr::neu(zeit("2026-10-29T12:00:00.000+01:00")));
    let gespraeche = Gespraeche::laden(ablage, uhr)
        .expect("geladen")
        .mit_anhangquote(Some(10 * MIB))
        .mit_anhangablage(anhaenge.clone())
        .mit_zufall(zufall);
    let mut zustand = testserver::zustand(ordner.path());
    zustand.gespraeche = Arc::new(gespraeche);
    let (protokoll, zeilen, _, _) = im_speicher("2026-10-29T12:00:00+01:00", Stufe::Debug);
    zustand.protokoll = protokoll;
    zustand.testpanik = testpanik;
    Aufbau {
        zustand: Arc::new(zustand),
        zeilen,
        anhaenge,
        _ordner: ordner,
    }
}

pub fn teilnehmer(kennung: &str) -> Teilnehmer {
    let name = kennung.split('@').next().expect("Name");
    Teilnehmer {
        kennung: Kennung::bilden(kennung).expect("Kennung"),
        name: Name::bilden(name).expect("Name"),
        herkunft: "127.0.0.1".parse().expect("IP"),
        art: Art::Agent,
    }
}

impl Aufbau {
    /// Eine fertig hochgeladene ID (angemeldet, Bytes abgelegt, abgeschlossen).
    pub fn hochgeladen(&self, wer: &str, name: &str, bytes: &[u8]) -> Anhangid {
        let gespraeche = &self.zustand.gespraeche;
        let wer = Kennung::bilden(wer).expect("Kennung");
        let anmeldung = gespraeche
            .anhang_anmelden(
                &wer,
                wer.herkunft().expect("Herkunft"),
                name,
                bytes.len() as u64,
            )
            .expect("angemeldet");
        let kopf = Kopf {
            typ_passt: true,
            laenge: Some(bytes.len() as u64),
        };
        let platz = gespraeche
            .hochladen_beginnen(&wer, &anmeldung.id, kopf)
            .expect("läuft");
        let mut teil = gespraeche
            .anhangablage()
            .teil_anlegen(&anmeldung.id)
            .expect("Teil");
        teil.anhaengen(bytes).expect("geschrieben");
        gespraeche
            .anhangablage()
            .teil_abschliessen(&anmeldung.id)
            .expect("fertig");
        gespraeche
            .hochladen_abschliessen(
                &anmeldung.id,
                platz.generation,
                crate::gespraeche::anhangpruefung::Art::datei(),
            )
            .expect("abgeschlossen");
        anmeldung.id
    }

    /// Eine Nachricht von `wer` in `allgemein` mit einem Anhang von 1 MiB; liefert Zeit und ID.
    pub fn nachricht_mit_anhang(&self, wer: &str) -> (Zeit, Anhangid) {
        let id = self.hochgeladen(wer, "bericht.bin", &vec![1; MIB as usize]);
        let Ok(Gesendet::Neu(nachricht)) = self.zustand.gespraeche.senden_mit_anhaengen(
            &Gespraech::channel("allgemein"),
            &Kennung::bilden(wer).expect("Kennung"),
            Art::Agent,
            "Bericht",
            std::slice::from_ref(&id),
            None,
        ) else {
            panic!("gesendet")
        };
        (nachricht.zeit, id)
    }

    /// Die Zeilen des Protokolls, die diesen Teil enthalten.
    pub fn zeilen_mit(&self, teil: &str) -> Vec<String> {
        self.zeilen
            .zeilen()
            .into_iter()
            .filter(|zeile| zeile.contains(teil))
            .collect()
    }

    /// Wartet höchstens zwei Sekunden, bis eine Zeile mit diesem Teil im Protokoll steht.
    pub async fn warten_auf_zeile(&self, teil: &str) {
        for _ in 0..200 {
            if !self.zeilen_mit(teil).is_empty() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    /// Der Blocking-Teil steht an der Schranke vor dem Kürzen; dann bricht der Client ab (die Zukunft des Handlers
    /// fällt), danach läuft der Blocking-Teil zu Ende.
    pub async fn abbrechen_am_kuerzen(
        &self,
        aufruf: impl Future<Output = Response> + Send + 'static,
    ) {
        let schranke = Arc::new(Barrier::new(2));
        self.anhaenge.vor_kuerzen_warten(Arc::clone(&schranke));
        let handler = tokio::spawn(aufruf);
        let an_der_schranke = Arc::clone(&schranke);
        tokio::task::spawn_blocking(move || an_der_schranke.wait())
            .await
            .expect("Schranke erreicht");
        handler.abort();
        assert!(handler.await.expect_err("abgebrochen").is_cancelled());
        tokio::task::spawn_blocking(move || schranke.wait())
            .await
            .expect("Schranke freigegeben");
    }

    /// Der Handler `POST …/nachrichten/loeschen` als Aufruf mit JSON-Körper `{"zeit": …}`.
    pub fn nachricht_loeschen(
        &self,
        wer: &str,
        zeit: Zeit,
    ) -> impl Future<Output = Response> + Send + use<> {
        let koerper =
            serde_json::json!({"zeit": zeit.to_rfc3339_opts(chrono::SecondsFormat::Millis, false)});
        nachricht_loeschen::loeschen(
            teilnehmer(wer),
            State(Arc::clone(&self.zustand)),
            Path("allgemein".to_string()),
            json_kopf(),
            Body::from(koerper.to_string()),
        )
    }

    /// Der Handler `POST /api/anhaenge/loeschen` als Aufruf mit `{"ids":[…],"bestaetigt":true}`.
    pub fn anhaenge_loeschen(
        &self,
        wer: &str,
        ids: &[&Anhangid],
    ) -> impl Future<Output = Response> + Send + use<> {
        let ids: Vec<&str> = ids.iter().map(|id| id.als_text()).collect();
        let koerper = serde_json::json!({"ids": ids, "bestaetigt": true});
        anhaenge::loeschen(
            teilnehmer(wer),
            State(Arc::clone(&self.zustand)),
            json_kopf(),
            Body::from(koerper.to_string()),
        )
    }

    /// Der Handler `POST /api/anhaenge` als Aufruf mit `{"name": …, "bytes": …}`.
    pub fn anhaenge_anmelden(&self, wer: &str) -> impl Future<Output = Response> + Send + use<> {
        let koerper = serde_json::json!({"name": "a.bin", "bytes": 10});
        anhaenge::anmelden(
            teilnehmer(wer),
            State(Arc::clone(&self.zustand)),
            json_kopf(),
            Body::from(koerper.to_string()),
        )
    }

    /// Der Handler `DELETE /api/anhaenge/{id}`.
    pub fn anhaenge_verwerfen(
        &self,
        wer: &str,
        id: &Anhangid,
    ) -> impl Future<Output = Response> + Send + use<> {
        anhaenge::verwerfen(
            teilnehmer(wer),
            State(Arc::clone(&self.zustand)),
            Path(id.als_text().to_string()),
        )
    }
}

/// Wartet auf das nächste Ereignis `anhaenge` des Busses; `None` nach zwei Sekunden.
pub async fn naechstes_anhaenge_ereignis(
    abonnement: &mut Abonnement,
) -> Option<Vec<(String, Zeit)>> {
    let warten = async {
        loop {
            if let Some(Empfang::Ereignis(Ereignis::Anhaenge { geloescht, .. })) =
                abonnement.naechstes().await
            {
                return geloescht;
            }
        }
    };
    tokio::time::timeout(Duration::from_secs(2), warten)
        .await
        .ok()
}

fn json_kopf() -> HeaderMap {
    let mut kopf = HeaderMap::new();
    kopf.insert(CONTENT_TYPE, "application/json".parse().expect("Kopf"));
    kopf
}
