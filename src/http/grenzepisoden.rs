//! Code-Review #24 B1, Abnahme-Proben 1 und 2: die Episoden an der Sitzungs- und an der Channel-Grenze, über
//! den Router und dieselbe Annahmeschleife wie das Programm, mit einem Protokoll in der Speicher-Senke und
//! gestellter Uhr (das Ende der Episode ohne Warten). Die Grenzen stehen als Zahl aus der Anforderung im Test
//! (R00015, R00018 F0081).

use super::testserver;
use crate::gespraeche::Direktgrenzen;
use crate::protokoll::senke::Speicher;
use crate::protokoll::testhilfe::{im_speicher, zeit};
use crate::protokoll::{Protokoll, Stufe};
use crate::uhr::GestellteUhr;
use serde_json::{Value, json};
use std::sync::Arc;

const SITZUNG: &str = "X-LocalchatA-Sitzung";

struct Instanz {
    basis: String,
    protokoll: Arc<Protokoll>,
    speicher: Speicher,
    uhr: Arc<GestellteUhr>,
    _ordner: tempfile::TempDir,
}

/// Router wie im Programm, Gespräche in einem eigenen Wegwerf-Ordner, Protokoll auf Information.
async fn starten() -> Instanz {
    starten_mit(Direktgrenzen::default()).await
}

/// Wie `starten`, mit eingestellten Grenzen neuer Direktgespräche (ADR-0055 Regel 4: kleine Testkonstanten statt
/// einer Flut).
async fn starten_mit(grenzen: Direktgrenzen) -> Instanz {
    let ordner = tempfile::tempdir().expect("Wegwerf-Ordner");
    let mut zustand = testserver::zustand(ordner.path());
    zustand.gespraeche = Arc::new(
        crate::gespraeche::Gespraeche::laden(
            Arc::new(crate::gespraeche::Dateiablage::neu(
                ordner.path(),
                Arc::new(crate::protokoll::Protokoll::ohne_ablage()),
            )),
            Arc::new(crate::uhr::Systemuhr),
        )
        .expect("leerer Ordner")
        .mit_direkt_grenzen(grenzen),
    );
    let (protokoll, speicher, uhr, _) =
        im_speicher("2026-10-28T12:00:00+01:00", Stufe::Information);
    zustand.protokoll = Arc::clone(&protokoll);
    let basis = testserver::app_starten(super::router(zustand)).await;
    Instanz {
        basis,
        protokoll,
        speicher,
        uhr,
        _ordner: ordner,
    }
}

impl Instanz {
    async fn post(&self, pfad: &str, koerper: Value, token: Option<&str>) -> (u16, Value) {
        self.post_von("127.0.0.1", pfad, koerper, token).await
    }

    /// Wie `post`, von der Herkunft `von` (Linux-Loopback 127.0.0.0/8).
    async fn post_von(
        &self,
        von: &str,
        pfad: &str,
        koerper: Value,
        token: Option<&str>,
    ) -> (u16, Value) {
        let client = reqwest::Client::builder()
            .local_address(von.parse::<std::net::IpAddr>().expect("Herkunft"))
            .build()
            .expect("Client");
        let mut anfrage = client.post(format!("{}{pfad}", self.basis)).json(&koerper);
        if let Some(token) = token {
            anfrage = anfrage.header(SITZUNG, token);
        }
        let antwort = anfrage.send().await.expect("Antwort");
        let status = antwort.status().as_u16();
        (status, antwort.json().await.unwrap_or(Value::Null))
    }

    /// Alle Zeilen `WARNING <bereich>` in Schreibreihenfolge, nachdem die Ruhe (60 s) vorbei ist.
    fn warnungen_nach_der_ruhe(&self, bereich: &str) -> Vec<String> {
        self.uhr.stellen(zeit("2026-10-28T12:01:01+01:00"));
        self.protokoll.episoden_schliessen();
        let teil = format!(" WARNING {bereich} ");
        self.speicher
            .zeilen()
            .into_iter()
            .filter(|zeile| zeile.contains(&teil))
            .collect()
    }
}

/// Probe 1: `JE_HERKUNFT` Beitritte von 127.0.0.1 gehen, die nächsten 6 sind 429 — genau eine Zeile „Episode
/// beginnt“ mit Herkunft und Grenze, nach der Ruhe genau eine „Episode endet: 6“.
#[tokio::test]
async fn sechs_beitritte_ueber_der_sitzungsgrenze_sind_eine_episode_mit_herkunft_und_grenze() {
    let instanz = starten().await;
    for nummer in 1..=128 {
        let (status, _) = instanz
            .post("api/beitreten", json!({"name": format!("n{nummer}")}), None)
            .await;
        assert_eq!(status, 201, "n{nummer}");
    }

    let mut ueber = Vec::new();
    for nummer in 1..=6 {
        let name = format!("z{nummer}");
        ueber.push(
            instanz
                .post("api/beitreten", json!({"name": name}), None)
                .await
                .0,
        );
    }
    let warnungen = instanz.warnungen_nach_der_ruhe("sitzung");

    assert_eq!(ueber, [429; 6]);
    assert_eq!(warnungen.len(), 2, "{warnungen:#?}");
    assert!(
        warnungen[0].ends_with(
            " WARNING sitzung Beitritt an der Sitzungsgrenze abgewiesen — Episode beginnt | herkunft=127.0.0.1 grenze=herkunft hoechstens=128"
        ),
        "{warnungen:#?}"
    );
    assert!(
        warnungen[1].contains(
            " WARNING sitzung Beitritt an der Sitzungsgrenze abgewiesen — Episode endet: 6 in 0 s (herkunft 6)"
        ),
        "{warnungen:#?}"
    );
}

/// R00018 F0081, Abnahme-Probe des Observability-Reviews #25 S1 (wörtlich: „Gesamtgrenze Sitzungen füllen, Beitritt
/// von frischer Herkunft → `grenze=512 art=gesamt`, Gegenprobe 129. von 127.0.0.1 → `grenze=128 art=herkunft`“)
/// mit den entschiedenen Namen (ADR-0042 Ergänzung Karte #26): 4 × 128 Sitzungen von 127.0.0.1–127.0.0.4, der
/// Beitritt von 127.0.0.5 → 429 und `grenze=gesamt hoechstens=512`; die Gegenprobe steht im Test darüber.
#[cfg(target_os = "linux")]
#[tokio::test]
async fn ein_beitritt_ueber_der_gesamtgrenze_nennt_grenze_gesamt_und_512() {
    let instanz = starten().await;
    for herkunft in 1..=4 {
        let von = format!("127.0.0.{herkunft}");
        for nummer in 1..=128 {
            let name = format!("h{herkunft}n{nummer}");
            let (status, antwort) = instanz
                .post_von(&von, "api/beitreten", json!({ "name": name }), None)
                .await;
            assert_eq!(status, 201, "{name}: {antwort}");
        }
    }

    let (ueber, antwort) = instanz
        .post_von(
            "127.0.0.5",
            "api/beitreten",
            json!({"name": "frisch"}),
            None,
        )
        .await;
    let warnungen = instanz.warnungen_nach_der_ruhe("sitzung");

    assert_eq!(ueber, 429, "{antwort}");
    assert_eq!(warnungen.len(), 2, "{warnungen:#?}");
    assert!(
        warnungen[0].ends_with(
            " WARNING sitzung Beitritt an der Sitzungsgrenze abgewiesen — Episode beginnt | herkunft=127.0.0.5 grenze=gesamt hoechstens=512"
        ),
        "{warnungen:#?}"
    );
    assert!(
        warnungen[1].contains(" — Episode endet: 1 in 0 s (gesamt 1)"),
        "{warnungen:#?}"
    );
}

/// Probe 2: `JE_HERKUNFT` Channels einer Herkunft gehen, der nächste ist 429 — genau eine Zeile „Episode
/// beginnt“ mit Herkunft und Grenze.
#[tokio::test]
async fn ein_anlegen_ueber_der_channel_grenze_ist_eine_episode_mit_herkunft_und_grenze() {
    let instanz = starten().await;
    let (status, sitzung) = instanz
        .post("api/beitreten", json!({"name": "alpha"}), None)
        .await;
    assert_eq!(status, 201);
    let token = sitzung["token"].as_str().expect("Token").to_string();
    for nummer in 1..=128 {
        let name = format!("c{nummer:03}");
        let (status, antwort) = instanz
            .post("api/channels", json!({"name": name}), Some(&token))
            .await;
        assert_eq!(status, 201, "{name}: {antwort}");
    }

    let (ueber, antwort) = instanz
        .post("api/channels", json!({"name": "ueber"}), Some(&token))
        .await;
    let warnungen = instanz.warnungen_nach_der_ruhe("channels");

    assert_eq!(ueber, 429, "{antwort}");
    assert_eq!(warnungen.len(), 2, "{warnungen:#?}");
    assert!(
        warnungen[0].ends_with(
            " WARNING channels Anlegen an der Channel-Grenze abgewiesen — Episode beginnt | herkunft=127.0.0.1 grenze=herkunft hoechstens=128"
        ),
        "{warnungen:#?}"
    );
    assert!(
        warnungen[1].contains(" Episode endet: 1 in 0 s (herkunft 1)"),
        "{warnungen:#?}"
    );
}

impl Instanz {
    /// Tritt als `name` von `von` bei und liefert das Token.
    async fn beitreten(&self, von: &str, name: &str) -> String {
        let (status, antwort) = self
            .post_von(von, "api/beitreten", json!({ "name": name }), None)
            .await;
        assert_eq!(status, 201, "{name}: {antwort}");
        antwort["token"].as_str().expect("Token").to_string()
    }

    /// `POST /api/direkt/{gegenueber}/nachrichten`.
    async fn direkt_senden(
        &self,
        von: &str,
        token: &str,
        gegenueber: &str,
        text: &str,
    ) -> (u16, Value) {
        self.post_von(
            von,
            &format!("api/direkt/{gegenueber}/nachrichten"),
            json!({ "text": text }),
            Some(token),
        )
        .await
    }
}

/// B0779 F1 (wörtlich, kleine Testkonstante): das N+1-te **neue** Paar einer Herkunft → 429
/// `{grenze: "direktgespraeche", art: "herkunft", hoechstens: 3}` wie die Channel-Grenze (`grenze` benennt die
/// Sache, `art` die Stufe); ein bestehendes Gespräch sendet weiter, ein Paar von einer anderen Herkunft geht, und
/// die Abweisung legt nichts an.
#[tokio::test]
async fn das_vierte_neue_direktgespraech_einer_herkunft_ist_429_direktgespraeche_herkunft() {
    let instanz = starten_mit(Direktgrenzen {
        gesamt: 100,
        je_herkunft: 3,
    })
    .await;
    let anna = instanz.beitreten("127.0.0.2", "anna").await;
    let ben = instanz.beitreten("127.0.0.3", "ben").await;
    for nummer in 1..=4 {
        instanz.beitreten("127.0.0.9", &format!("p{nummer}")).await;
    }
    for nummer in 1..=3 {
        let (status, antwort) = instanz
            .direkt_senden("127.0.0.2", &anna, &format!("p{nummer}@127.0.0.9"), "hallo")
            .await;
        assert_eq!(status, 201, "p{nummer}: {antwort}");
    }

    let (ueber, antwort) = instanz
        .direkt_senden("127.0.0.2", &anna, "p4@127.0.0.9", "hallo")
        .await;
    let (bestehend, _) = instanz
        .direkt_senden("127.0.0.2", &anna, "p1@127.0.0.9", "noch eins")
        .await;
    let (andere_herkunft, _) = instanz
        .direkt_senden("127.0.0.3", &ben, "p4@127.0.0.9", "hallo")
        .await;

    assert_eq!(ueber, 429, "{antwort}");
    assert_eq!(antwort["grenze"], "direktgespraeche", "{antwort}");
    assert_eq!(antwort["art"], "herkunft", "{antwort}");
    assert_eq!(antwort["hoechstens"], 3, "{antwort}");
    assert!(
        antwort["fehler"]
            .as_str()
            .is_some_and(|text| !text.is_empty() && !text.contains("p4@")),
        "ein Satz, ohne die Kennung des Gegenübers: {antwort}"
    );
    assert_eq!(bestehend, 201);
    assert_eq!(andere_herkunft, 201);
}

/// Gesamtgrenze: `art: "gesamt"` mit `hoechstens` aus der Gesamtgrenze, vor der Herkunftsgrenze.
#[tokio::test]
async fn das_neue_direktgespraech_ueber_der_gesamtgrenze_ist_429_direktgespraeche_gesamt() {
    let instanz = starten_mit(Direktgrenzen {
        gesamt: 2,
        je_herkunft: 1,
    })
    .await;
    let anna = instanz.beitreten("127.0.0.2", "anna").await;
    let ben = instanz.beitreten("127.0.0.3", "ben").await;
    for nummer in 1..=3 {
        instanz.beitreten("127.0.0.9", &format!("p{nummer}")).await;
    }
    for (von, token, nummer) in [("127.0.0.2", &anna, 1), ("127.0.0.3", &ben, 2)] {
        let (status, antwort) = instanz
            .direkt_senden(von, token, &format!("p{nummer}@127.0.0.9"), "hallo")
            .await;
        assert_eq!(status, 201, "p{nummer}: {antwort}");
    }

    let (ueber, antwort) = instanz
        .direkt_senden("127.0.0.2", &anna, "p3@127.0.0.9", "hallo")
        .await;

    assert_eq!(ueber, 429, "{antwort}");
    assert_eq!(antwort["grenze"], "direktgespraeche", "{antwort}");
    assert_eq!(antwort["art"], "gesamt", "{antwort}");
    assert_eq!(antwort["hoechstens"], 2, "{antwort}");
}
