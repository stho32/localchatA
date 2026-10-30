//! N4 Verkabelung des Nachholens (ADR-0054 Ergänzung „Nachholen von `warten` …“ Regel 5 bis 8, R00049 F0170,
//! B0758): über den Router und dieselbe Annahmeschleife wie das Programm, mit eigener gestellter Uhr für das
//! Nachholkonto (steht, bis der Test sie stellt) und für die Anwesenheit, Protokoll in der Speicher-Senke. Die
//! Zahlen kommen aus der Anforderung (K3 50 × 2.000, K4 250 × 2.000, K5 200 leichte Aufrufe).

use super::testserver;
use crate::anwesenheit::{Anwesenheit, Beitrittswunsch};
use crate::gespraeche::{Art, Gespraech, Kennung};
use crate::protokoll::senke::Speicher;
use crate::protokoll::testhilfe::{im_speicher, zeit};
use crate::protokoll::{Protokoll, Stufe};
use crate::schreibkonto::nachholkonto::Nachholkonto;
use crate::uhr::{GestellteUhr, Zeit};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

const SITZUNG: &str = "X-LocalchatA-Sitzung";
/// Ein schwerer Aufruf: altes `seit`, `erwaehnung` (kein Treffer) — er prüft das volle Budget von 2.000 Sätzen.
const SCHWER: &str = "api/warten?bis=0&seit=2000-01-01T00:00:00.000Z&erwaehnung";
/// So viele Sätze stehen im Channel — mehr als ein Budget, damit jeder schwere Aufruf 2.000 prüft.
const SAETZE: usize = 2_500;

struct Aufbau {
    basis: String,
    tokens: HashMap<String, String>,
    konto: Arc<Nachholkonto>,
    konto_uhr: Arc<GestellteUhr>,
    anwesenheit_uhr: Arc<GestellteUhr>,
    protokoll: Arc<Protokoll>,
    protokoll_uhr: Arc<GestellteUhr>,
    speicher: Speicher,
    _ordner: tempfile::TempDir,
}

fn t(text: &str) -> Zeit {
    zeit(text)
}

/// Router wie im Programm; `# allgemein` mit `SAETZE` Nachrichten von `anna`; `namen` treten von 127.0.0.1 bei.
async fn aufbau(namen: &[&str]) -> Aufbau {
    let ordner = tempfile::tempdir().expect("Wegwerf-Ordner");
    let mut zustand = testserver::zustand(ordner.path());
    let anwesenheit_uhr = Arc::new(GestellteUhr::neu(t("2026-10-30T12:00:00+01:00")));
    zustand.anwesenheit = Arc::new(
        Anwesenheit::neu(
            Arc::clone(&anwesenheit_uhr) as Arc<dyn crate::uhr::Uhr>,
            Arc::new(crate::zufall::Betriebssystemzufall),
            Duration::from_secs(60),
        )
        .mit_veroeffentlichen(
            crate::live::bus::anwesenheit_mit_aktivitaet_veroeffentlichen(
                &zustand.bus,
                &zustand.aktivitaet,
            ),
        ),
    );
    let konto_uhr = Arc::new(GestellteUhr::neu(t("2026-10-30T12:00:00+01:00")));
    let konto = Arc::new(Nachholkonto::neu(
        Arc::clone(&konto_uhr) as Arc<dyn crate::uhr::Uhr>
    ));
    zustand.nachholkonto = Arc::clone(&konto);
    let (protokoll, speicher, protokoll_uhr, _) =
        im_speicher("2026-10-30T12:00:00+01:00", Stufe::Information);
    zustand.protokoll = Arc::clone(&protokoll);
    let anna = Kennung::bilden("anna@127.0.0.1").expect("Kennung");
    zustand
        .gespraeche
        .channel_anlegen("allgemein", None, &anna)
        .expect("angelegt");
    for nummer in 1..=SAETZE {
        zustand
            .gespraeche
            .senden(
                &Gespraech::channel("allgemein"),
                &anna,
                Art::Mensch,
                &format!("Nachricht {nummer}"),
            )
            .expect("gesendet");
    }
    let tokens = namen
        .iter()
        .map(|name| {
            let wunsch = Beitrittswunsch::pruefen(&json!({ "name": name })).expect("Wunsch");
            let token = zustand
                .anwesenheit
                .beitreten(&wunsch, "127.0.0.1".parse().expect("IP"))
                .expect("beigetreten")
                .sitzung
                .token
                .als_text()
                .to_string();
            ((*name).to_string(), token)
        })
        .collect();
    let basis = testserver::app_starten(super::router(zustand)).await;
    Aufbau {
        basis,
        tokens,
        konto,
        konto_uhr,
        anwesenheit_uhr,
        protokoll,
        protokoll_uhr,
        speicher,
        _ordner: ordner,
    }
}

/// Eine Antwort: Status, `Retry-After` und Körper.
#[derive(Debug, Clone, PartialEq)]
struct Antwort {
    status: u16,
    retry_after: Option<String>,
    koerper: Value,
}

impl Aufbau {
    async fn holen(&self, name: Option<&str>, pfad: &str) -> Antwort {
        let mut anfrage = reqwest::Client::new().get(format!("{}{pfad}", self.basis));
        if let Some(name) = name {
            anfrage = anfrage.header(SITZUNG, &self.tokens[name]);
        }
        let antwort = anfrage.send().await.expect("Antwort");
        let status = antwort.status().as_u16();
        let retry_after = antwort
            .headers()
            .get("retry-after")
            .and_then(|wert| wert.to_str().ok())
            .map(str::to_string);
        Antwort {
            status,
            retry_after,
            koerper: antwort.json().await.unwrap_or(Value::Null),
        }
    }

    async fn schwer(&self, name: &str) -> Antwort {
        self.holen(Some(name), SCHWER).await
    }

    async fn gesundheit(&self) -> Value {
        self.holen(None, "api/gesundheit").await.koerper
    }

    fn konto_stellen(&self, ms: i64) {
        self.konto_uhr
            .stellen(t("2026-10-30T12:00:00+01:00") + chrono::Duration::milliseconds(ms));
    }

    /// Alle Zeilen `WARNING nachholen`, nachdem die Ruhe (60 s) vorbei ist.
    fn warnungen_nach_der_ruhe(&self) -> Vec<String> {
        self.protokoll_uhr.stellen(t("2026-10-30T12:01:01+01:00"));
        self.protokoll.episoden_schliessen();
        self.speicher
            .zeilen()
            .into_iter()
            .filter(|zeile| zeile.contains(" WARNING nachholen "))
            .collect()
    }
}

/// K3 und 429-Antwort: 50 schwere Aufrufe von `claude` → 200 `weitere`; der 51. → 429 `{fehler, grenze:
/// "nachholen", von: "kennung", wieder_in_ms: 100}` mit `Retry-After: 1`; der Text nennt die Kennung und sagt, dass
/// `seit` gültig bleibt. Nach 100 ms liefert derselbe Aufruf dieselbe Antwort wie der erste.
#[tokio::test]
async fn der_einundfuenfzigste_schwere_aufruf_einer_kennung_ist_429_nachholen() {
    let aufbau = aufbau(&["claude"]).await;
    let mut antworten = Vec::new();
    for _ in 0..50 {
        antworten.push(aufbau.schwer("claude").await);
    }

    let gebremst = aufbau.schwer("claude").await;
    aufbau.konto_stellen(100);
    let danach = aufbau.schwer("claude").await;
    let gesundheit = aufbau.gesundheit().await;

    assert!(
        antworten
            .iter()
            .all(|antwort| antwort.status == 200 && antwort.koerper["ende"] == "weitere"),
        "{:?}",
        antworten.iter().find(|antwort| antwort.status != 200)
    );
    assert_eq!(gebremst.status, 429, "{}", gebremst.koerper);
    assert_eq!(gebremst.retry_after.as_deref(), Some("1"));
    assert_eq!(gebremst.koerper["grenze"], "nachholen");
    assert_eq!(gebremst.koerper["von"], "kennung");
    assert_eq!(gebremst.koerper["wieder_in_ms"], 100);
    let fehler = gebremst.koerper["fehler"].as_str().expect("fehler");
    assert!(fehler.contains("claude@127.0.0.1"), "{fehler}");
    assert!(fehler.contains("seit"), "{fehler}");
    assert_eq!(danach, antworten[0]);
    assert_eq!(gesundheit["nachholen"]["gebremst"], 1);
    assert_eq!(gesundheit["nachholen"]["saetze"], 51 * 2_000);
    assert_eq!(
        gesundheit["nachholen"]["kosten"],
        51 * 2_000,
        "kurze Sätze: Kosten gleich Sätze"
    );
    assert_eq!(gesundheit["nachholen"]["bank_belegt"], 0);
    assert!(gesundheit["nachholen"]["bank_plaetze"].as_u64() >= Some(1));
}

/// K4: 16 Kennungen von 127.0.0.1 buchen zusammen 250 schwere Aufrufe → alle 200; der 251. → 429 `von:
/// "herkunft"`, der Text nennt die Adresse („alle Teilnehmer dieser Adresse zusammen“).
#[tokio::test]
async fn der_zweihunderteinundfuenfzigste_schwere_aufruf_einer_herkunft_ist_429() {
    let namen: Vec<String> = (0..16).map(|nummer| format!("k{nummer}")).collect();
    let aufbau = aufbau(&namen.iter().map(String::as_str).collect::<Vec<_>>()).await;
    let mut status = Vec::new();
    for nummer in 0..250 {
        status.push(aufbau.schwer(&namen[nummer % 16]).await.status);
    }

    let gebremst = aufbau.schwer(&namen[250 % 16]).await;

    assert!(status.iter().all(|status| *status == 200), "{status:?}");
    assert_eq!(gebremst.status, 429, "{}", gebremst.koerper);
    assert_eq!(gebremst.koerper["von"], "herkunft");
    let fehler = gebremst.koerper["fehler"].as_str().expect("fehler");
    assert!(
        fehler.contains("127.0.0.1 (alle Teilnehmer dieser Adresse zusammen)"),
        "{fehler}"
    );
}

/// K5: 200 leichte Aufrufe in Folge (`seit` = Marke, nichts Neues) → kein 429; die Erstattung gilt für jeden
/// Ausgang, auch `frist` ohne Treffer.
#[tokio::test]
async fn zweihundert_leichte_aufrufe_in_folge_sind_nie_gebremst() {
    let aufbau = aufbau(&["claude"]).await;
    let mut seit = aufbau
        .holen(Some("claude"), "api/warten?bis=0")
        .await
        .koerper["seit"]
        .as_str()
        .expect("seit")
        .replace('+', "%2B");
    let mut status = Vec::new();
    for _ in 0..200 {
        let antwort = aufbau
            .holen(Some("claude"), &format!("api/warten?bis=0&seit={seit}"))
            .await;
        status.push(antwort.status);
        seit = antwort.koerper["seit"]
            .as_str()
            .unwrap_or_default()
            .replace('+', "%2B");
    }

    assert!(status.iter().all(|status| *status == 200), "{status:?}");
    assert_eq!(aufbau.gesundheit().await["nachholen"]["gebremst"], 0);
}

/// Reihenfolge und Nebenwirkungen (A5 Code-Review #17 F0169): ein 429 `nachholen` hinterlässt keinen Halter und
/// kein Lebenszeichen — `wartend` bleibt 0 und die Zeile von `claude` in `/api/teilnehmer` ist unverändert, auch
/// wenn die Uhr der Anwesenheit inzwischen 10 s weiter ist.
#[tokio::test]
async fn ein_429_nachholen_hinterlaesst_keinen_halter_und_kein_lebenszeichen() {
    let aufbau = aufbau(&["claude", "beob"]).await;
    aufbau
        .konto
        .reservieren(
            &Kennung::bilden("claude@127.0.0.1").expect("Kennung"),
            "127.0.0.1".parse().expect("IP"),
            100_000,
        )
        .expect("Vorrat");
    aufbau
        .anwesenheit_uhr
        .stellen(t("2026-10-30T12:00:10+01:00"));
    let vorher = zeile_von(&aufbau, "claude").await;

    let gebremst = aufbau.schwer("claude").await;
    let wartend = aufbau.gesundheit().await["wartend"].clone();
    aufbau
        .anwesenheit_uhr
        .stellen(t("2026-10-30T12:00:12+01:00"));
    let nachher = zeile_von(&aufbau, "claude").await;

    assert_eq!(gebremst.status, 429, "{}", gebremst.koerper);
    assert_eq!(wartend, 0);
    assert_eq!(nachher, vorher);
}

/// Die Zeile `name` in `GET /api/teilnehmer`, gelesen von `beob`.
async fn zeile_von(aufbau: &Aufbau, name: &str) -> Value {
    let liste = aufbau.holen(Some("beob"), "api/teilnehmer").await.koerper;
    liste["liste"]
        .as_array()
        .expect("liste")
        .iter()
        .find(|zeile| zeile["name"] == name)
        .cloned()
        .expect("Zeile")
}

/// Reihenfolge 401 → 422 → 404 → Grenze 4 → reservieren: bei leerem Konto antworten ohne Sitzung 401, ein falscher
/// Parameter 422, ein unbekannter Channel 404 und der fünfte gleichzeitige Aufruf 429 `warten` — nie `nachholen`.
#[tokio::test]
async fn die_pruefungen_vor_dem_reservieren_gehen_vor() {
    let aufbau = aufbau(&["claude"]).await;
    let offen: Vec<_> = (0..4)
        .map(|_| {
            let basis = aufbau.basis.clone();
            let token = aufbau.tokens["claude"].clone();
            tokio::spawn(async move {
                reqwest::Client::new()
                    .get(format!("{basis}api/warten?bis=60"))
                    .header(SITZUNG, token)
                    .send()
                    .await
            })
        })
        .collect();
    while aufbau.gesundheit().await["wartend"] != 4 {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    // `wartend` zählt ab dem Halter, die vier Aufrufe erstatten ihre Reservierung erst nach dem Nachholen — das Konto
    // ist leer, sobald der ganze Vorrat genommen ist, nicht schon bei `wartend` 4 (wackelte 2 von 30).
    let kennung = Kennung::bilden("claude@127.0.0.1").expect("Kennung");
    let herkunft = "127.0.0.1".parse().expect("IP");
    let frist = std::time::Instant::now() + Duration::from_secs(5);
    while aufbau
        .konto
        .reservieren(&kennung, herkunft, 100_000)
        .is_err()
    {
        assert!(std::time::Instant::now() < frist, "Vorrat wird nicht frei");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    let ohne_sitzung = aufbau.holen(None, SCHWER).await;
    let falsch = aufbau.holen(Some("claude"), "api/warten?bis=abc").await;
    let unbekannt = aufbau
        .holen(Some("claude"), "api/warten?bis=0&channel=gibtesnicht")
        .await;
    let fuenfter = aufbau.schwer("claude").await;
    for aufruf in offen {
        aufruf.abort();
    }

    assert_eq!(ohne_sitzung.status, 401);
    assert_eq!(falsch.status, 422);
    assert_eq!(unbekannt.status, 404);
    assert_eq!(fuenfter.status, 429);
    assert_eq!(fuenfter.koerper["grenze"], "warten");
    assert_eq!(aufbau.gesundheit().await["nachholen"]["gebremst"], 0);
}

/// Betrieb: drei 429 `nachholen` von `claude` sind **eine** Episode — genau eine Zeile „Episode beginnt“ mit
/// Kennung und Grenze, nach der Ruhe eine Ende-Zeile; nie Token oder Nachrichtentext.
#[tokio::test]
async fn drei_429_nachholen_sind_eine_episode_ohne_token_und_text() {
    let aufbau = aufbau(&["claude"]).await;
    aufbau
        .konto
        .reservieren(
            &Kennung::bilden("claude@127.0.0.1").expect("Kennung"),
            "127.0.0.1".parse().expect("IP"),
            100_000,
        )
        .expect("Vorrat");
    for _ in 0..3 {
        assert_eq!(aufbau.schwer("claude").await.status, 429);
    }

    let warnungen = aufbau.warnungen_nach_der_ruhe();

    assert_eq!(warnungen.len(), 2, "{warnungen:#?}");
    assert!(
        warnungen[0].ends_with(
            " WARNING nachholen Nachholen gebremst: zu viele geprüfte Sätze — Episode beginnt | kennung=claude@127.0.0.1 grenze=kennung"
        ),
        "{warnungen:#?}"
    );
    assert!(
        warnungen[1].contains(" — Episode endet: 3 in 0 s (kennung 3)"),
        "{warnungen:#?}"
    );
    let token = &aufbau.tokens["claude"];
    assert!(
        warnungen
            .iter()
            .all(|zeile| !zeile.contains(token.as_str()) && !zeile.contains("Nachricht"))
    );
}
