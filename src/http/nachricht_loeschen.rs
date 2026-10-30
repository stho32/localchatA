//! Eigene Nachricht löschen (R00040 F0136; ADR-0048 Regeln 5, 9, 10, 12, 13): `POST
//! /api/channels/{name}/nachrichten/loeschen {zeit}` mit Sitzung, Mensch und Agent gleich, in einem Schritt → 200
//! `{channel, nachricht, ergebnis, anhaenge}`. Prüfreihenfolge (abgewiesen, nie umgeformt): Grenzschutz 403 ·
//! Sitzung 401 · Vorab-Buchung 1 Einheit 429 · Körper über 1.024 Bytes 413 · kein JSON-Typ 415 · kein JSON 400 ·
//! doppeltes oder weiteres Feld (auch `bestaetigt`) 422 `feld` · Channel 404 · 422 `zeit` · unter der Sperre 404
//! `nachricht` · 403 `fremd` · schon gelöscht 200 · 507 `quote` (mit Reserve) · 500. Kein Wiederholschlüssel, keine
//! Restbuchung — Löschen ist von sich aus wiederholbar.

use super::Zustand;
use super::abbruch::{Abgebrochen, Grenzort, abbruch_beantworten};
use super::anfrageprotokoll::mit_regel;
use super::anhaenge::loeschen::{Anlass, protokollieren, veroeffentlichen};
use super::bearbeiten::{
    fremd_abgewiesen, fremd_auftreten, gespraech_bekannt, nachricht_fehlt, quote_erreicht,
    schreibfehler_auftreten, unbekannt_regel, unbekannt_wie, zeit_abgewiesen, zeit_lesen,
};
use super::bremse::{Gegenstand, gebremst_melden};
use super::koerper::{Koerperfehler, Koerperort, json_lesen, koerperfehler_beantworten};
use super::nachrichten::NachrichtAntwort;
use super::senden::{nicht_gespeichert_antwort, quote_auftreten};
use super::teilnehmer::Teilnehmer;
use super::werkbank::bank_erwerben;
use super::ziel::{Segment, Ziel, bezug, ziel_aus_pfad};
use crate::gespraeche::loeschen::Geloescht;
use crate::gespraeche::nachricht_loeschen::{Loeschfehler, NachrichtGeloescht};
use crate::gespraeche::{Gespraeche, Nachricht};
use crate::live::bus::Bus;
use crate::protokoll::vorlauf;
use crate::protokoll::{Bereich, Kontext, Protokoll, Schluessel, Stufe};
use crate::uhr::Zeit;
use axum::Json;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Instant;

const ROUTE: &str = "POST /api/channels/{name}/nachrichten/loeschen";

/// Körpergrenze des Löschens (ADR-0048 Regel 5): `{"zeit":"…"}` in der längsten Schreibweise ≈ 190 Bytes.
pub const LOESCHEN_GRENZE: usize = 1_024;

/// Kosten am Schreibkonto: eine Einheit, nur vorab (ADR-0048 Regel 5).
const VORAB_LOESCHEN: u32 = 1;

/// `POST /api/channels/{name}/nachrichten/loeschen`.
pub async fn loeschen(
    teilnehmer: Teilnehmer,
    State(zustand): State<Arc<Zustand>>,
    Path(name): Path<String>,
    kopf: HeaderMap,
    koerper: Body,
) -> Response {
    loeschen_in(teilnehmer, zustand, Ziel::Channel(name), kopf, koerper).await
}

/// `POST /api/direkt/{kennung}/nachrichten/loeschen` (ADR-0055 Regel 3): dieselbe Folge, das Paar aus Sitzung und
/// Pfad; ohne Gespräch 404 `nachricht` wie für eine nie vergebene Zeit (ADR-0048 M3).
pub(super) async fn loeschen_direkt(
    teilnehmer: Teilnehmer,
    State(zustand): State<Arc<Zustand>>,
    segment: Segment,
    kopf: HeaderMap,
    koerper: Body,
) -> Response {
    match ziel_aus_pfad(&teilnehmer, &segment) {
        Ok(ziel) => loeschen_in(teilnehmer, zustand, ziel, kopf, koerper).await,
        Err(fehler) => fehler.antwort(&teilnehmer.kennung),
    }
}

async fn loeschen_in(
    teilnehmer: Teilnehmer,
    zustand: Arc<Zustand>,
    gespraech_ziel: Ziel,
    kopf: HeaderMap,
    koerper: Body,
) -> Response {
    let name = gespraech_ziel.name();
    let eintritt = Instant::now();
    if let Err(gebremst) =
        zustand
            .schreibkonto
            .vorab(&teilnehmer.kennung, teilnehmer.herkunft, VORAB_LOESCHEN)
    {
        return gebremst_melden(
            &zustand.protokoll,
            gebremst,
            Gegenstand::Loeschungen,
            &teilnehmer,
        );
    }
    let wer = teilnehmer.kennung.als_text().to_string();
    let ort = Koerperort {
        protokoll: &zustand.protokoll,
        route: ROUTE,
        herkunft: teilnehmer.herkunft,
        kennung: Some(&wer),
    };
    let wert = match json_lesen(&kopf, koerper, LOESCHEN_GRENZE, &ort).await {
        Ok(wert) => wert,
        Err(fehler) => return koerper_abgewiesen(fehler),
    };
    if let Some(feld) = weiteres_feld(&wert) {
        return mit_regel(feld_abgewiesen(&feld), "feld_unbekannt", Some(&wer));
    }
    if !gespraech_bekannt(&zustand, &gespraech_ziel) {
        return mit_regel(
            unbekannt_wie(&gespraech_ziel),
            unbekannt_regel(&gespraech_ziel),
            Some(&wer),
        );
    }
    let Some(ziel) = zeit_lesen(&wert) else {
        return mit_regel(zeit_abgewiesen(), "zeit", Some(&wer));
    };
    let ort = Grenzort {
        route: ROUTE,
        operation: "nachricht_loeschen",
        kennung: Some(&wer),
        channel: Some(&name),
    };
    let erlaubnis =
        if zustand
            .gespraeche
            .anhangzahl(&gespraech_ziel.gespraech(), &teilnehmer.kennung, ziel)
            > 0
        {
            match bank_erwerben(&zustand.hochladebank).await {
                Ok(erlaubnis) => Some(erlaubnis),
                Err(abbruch) => {
                    return abbruch_beantworten(
                        &zustand.protokoll,
                        &abbruch,
                        &ort,
                        NICHT_GELOESCHT,
                    );
                }
            }
        } else {
            None
        };
    let testpanik = zustand.testpanik.clone();
    let auftrag = Loeschauftrag {
        gespraeche: Arc::clone(&zustand.gespraeche),
        protokoll: Arc::clone(&zustand.protokoll),
        bus: Arc::clone(&zustand.bus),
        teilnehmer: teilnehmer.clone(),
        gespraech: gespraech_ziel,
        ziel,
        eintritt,
    };
    let fertig = tokio::task::spawn_blocking(move || {
        let _erlaubnis = erlaubnis;
        testpanik.loeschen();
        auftrag.ausfuehren()
    })
    .await;
    match fertig {
        Ok(antwort) => antwort,
        Err(fehler) => abbruch_beantworten(
            &zustand.protokoll,
            &Abgebrochen::from(fehler),
            &ort,
            NICHT_GELOESCHT,
        ),
    }
}

const NICHT_GELOESCHT: &str = "Nachricht nicht gelöscht";

/// Alles, was nach dem Commit-Punkt geschieht, in einem Blocking-Teil (ADR-0048 K2): die Wirkung, die Protokollzeilen,
/// die Episoden, das Ereignis und der Bau der Antwort. Fällt die Zukunft des Handlers, weil der Client abbricht,
/// läuft dieser Teil zu Ende — ein `await` dazwischen verlöre Spur und Ereignis.
struct Loeschauftrag {
    gespraeche: Arc<Gespraeche>,
    protokoll: Arc<Protokoll>,
    bus: Arc<Bus>,
    teilnehmer: Teilnehmer,
    gespraech: Ziel,
    ziel: Zeit,
    eintritt: Instant,
}

impl Loeschauftrag {
    fn ausfuehren(&self) -> Response {
        let wer = self.teilnehmer.kennung.als_text();
        let channel = self.gespraech.name();
        let ergebnis = self.gespraeche.nachricht_loeschen(
            &self.gespraech.gespraech(),
            &self.teilnehmer.kennung,
            self.ziel,
        );
        match ergebnis {
            Ok(NachrichtGeloescht::Neu {
                nachricht,
                anhaenge,
            }) => {
                vorlauf::eintragen(&self.protokoll, self.gespraeche.zeit_vorlauf_ms());
                self.melden(anhaenge.umfang.anhaenge, None);
                protokollieren(
                    &self.protokoll,
                    &self.teilnehmer,
                    &anhaenge,
                    Anlass::Nachricht,
                );
                veroeffentlichen(&self.bus, &anhaenge);
                geloescht(&self.gespraech, &nachricht, "geloescht", Some(&anhaenge))
            }
            Ok(NachrichtGeloescht::Schon(nachricht)) => {
                self.melden(0, Some("schon_geloescht"));
                geloescht(&self.gespraech, &nachricht, "schon_geloescht", None)
            }
            Err(fehler) => {
                abweisung_melden(&self.protokoll, &fehler, &channel, wer);
                abgewiesen(fehler, &self.gespraech, wer)
            }
        }
    }

    fn melden(&self, anhaenge: u64, grund: Option<&str>) {
        geloescht_melden(
            &self.protokoll,
            &self.gespraech.name(),
            self.teilnehmer.kennung.als_text(),
            anhaenge,
            self.eintritt,
            grund,
        );
    }
}

/// `DEBUG aendern Nachricht gelöscht` (ADR-0048 Regel 13): Channel, Kennung, Zahl der mitgelöschten Anhänge, die Zeit
/// vom Eintritt in den Handler bis zum Ende; ohne Satz `grund=schon_geloescht` — nie Text.
fn geloescht_melden(
    protokoll: &Protokoll,
    channel: &str,
    wer: &str,
    anhaenge: u64,
    eintritt: Instant,
    grund: Option<&str>,
) {
    let kontext = Kontext::neu()
        .mit(bezug(channel), channel)
        .mit(Schluessel::Kennung, wer)
        .mit(Schluessel::Anhaenge, anhaenge)
        .mit(Schluessel::DauerMs, eintritt.elapsed().as_millis());
    let kontext = match grund {
        Some(grund) => kontext.mit(Schluessel::Grund, grund),
        None => kontext,
    };
    protokoll.eintragen(
        Stufe::Debug,
        Bereich::Aendern,
        "Nachricht gelöscht",
        kontext,
    );
}

/// Abweisungen unter der Sperre mit eigener Episode wie beim Bearbeiten: 403 `fremd` je Kennung, 500 je Datei und
/// Grund, 507 in der Quote-Episode der Kennung mit `route=loeschen`.
fn abweisung_melden(protokoll: &Protokoll, fehler: &Loeschfehler, channel: &str, wer: &str) {
    match fehler {
        Loeschfehler::Fremd { .. } => protokoll.episode(fremd_auftreten(channel, wer)),
        Loeschfehler::Ablage { datei, grund } => {
            protokoll.episode(schreibfehler_auftreten(channel, wer, datei, grund));
        }
        Loeschfehler::Quote { belegt, hoechstens } => {
            protokoll.episode(
                quote_auftreten(channel, wer, *belegt, *hoechstens)
                    .einzeln(Schluessel::Route, "loeschen"),
            );
        }
        Loeschfehler::Unbekannt | Loeschfehler::Nachricht => {}
    }
}

/// Die Antworten der Prüfungen unter der Sperre.
fn abgewiesen(fehler: Loeschfehler, ziel: &Ziel, wer: &str) -> Response {
    match fehler {
        Loeschfehler::Unbekannt => mit_regel(unbekannt_wie(ziel), unbekannt_regel(ziel), Some(wer)),
        Loeschfehler::Nachricht => mit_regel(nachricht_fehlt(), "nachricht", Some(wer)),
        Loeschfehler::Fremd { absender } => {
            mit_regel(fremd_abgewiesen(&absender), "fremd", Some(wer))
        }
        Loeschfehler::Quote { belegt, hoechstens } => mit_regel(
            quote_erreicht("Nicht gelöscht", belegt, hoechstens),
            "quote",
            Some(wer),
        ),
        Loeschfehler::Ablage { .. } => nicht_gespeichert_antwort(),
    }
}

/// 200 mit der gelöschten Fassung und der Bilanz der mitgelöschten Anhänge `{geloescht, bytes, fehler}` — schon
/// gelöscht: nichts mitgelöscht.
fn geloescht(
    ziel: &Ziel,
    nachricht: &Nachricht,
    ergebnis: &str,
    anhaenge: Option<&Geloescht>,
) -> Response {
    let (zahl, bytes, fehler) = anhaenge.map_or((0, 0, 0), |anhaenge| {
        (
            anhaenge.umfang.anhaenge,
            anhaenge.umfang.bytes,
            anhaenge.fehler,
        )
    });
    let mut antwort = ziel.antwort_mit(json!(NachrichtAntwort::aus(nachricht)));
    antwort["ergebnis"] = json!(ergebnis);
    antwort["anhaenge"] = json!({"geloescht": zahl, "bytes": bytes, "fehler": fehler});
    (StatusCode::OK, Json(antwort)).into_response()
}

/// Das erste Feld neben `zeit` (ADR-0048 Regel 5, Regel 12: auch `bestaetigt`); ein Nicht-Objekt hat keines.
fn weiteres_feld(wert: &Value) -> Option<String> {
    wert.as_object()?
        .keys()
        .find(|feld| *feld != "zeit")
        .cloned()
}

fn feld_abgewiesen(feld: &str) -> Response {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(json!({
            "fehler": format!("Unbekanntes Feld {feld} — erlaubt ist nur zeit."),
            "regel": "feld",
            "feld": feld,
        })),
    )
        .into_response()
}

const BEISPIEL: &str = r#"{"zeit":"2026-10-30T14:07:12.345+01:00"}"#;

fn koerper_abgewiesen(fehler: Koerperfehler) -> Response {
    koerperfehler_beantworten(
        fehler,
        "Der Körper ist zu groß — höchstens 1.024 Byte; er braucht nur zeit.",
        BEISPIEL,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gespraeche::Gespraech;
    use crate::http::loeschaufbau::{aufbau, aufbau_mit, naechstes_anhaenge_ereignis};
    use crate::protokoll::testpanik::Testpanik;
    use crate::zufall::FesteFolge;
    use serde_json::json;
    use std::ffi::OsStr;
    use std::time::Duration;

    /// R00043 B0666 (ADR-0048 K2): bricht der Client nach dem Start des Löschens ab, stehen trotzdem genau eine Zeile
    /// `DEBUG aendern Nachricht gelöscht`, eine `INFORMATION anhang Anhänge gelöscht … grund=nachricht` und ein
    /// Ereignis `anhaenge` (Mutante „Protokoll nach dem await“ rot).
    #[tokio::test(flavor = "multi_thread")]
    async fn der_abbruch_des_clients_nach_dem_loeschsatz_verliert_weder_protokoll_noch_ereignis() {
        let aufbau = aufbau();
        let (zeit, id) = aufbau.nachricht_mit_anhang("beta@127.0.0.1");
        let mut abonnement = aufbau.zustand.bus.abonnieren();

        aufbau
            .abbrechen_am_kuerzen(aufbau.nachricht_loeschen("beta@127.0.0.1", zeit))
            .await;
        let ereignis = naechstes_anhaenge_ereignis(&mut abonnement).await;

        assert_eq!(ereignis, Some(vec![(id.als_text().to_string(), zeit)]));
        let zeilen = aufbau.zeilen.zeilen();
        assert_eq!(
            aufbau.zeilen_mit(" DEBUG aendern Nachricht gelöscht").len(),
            1,
            "{zeilen:#?}"
        );
        let information = aufbau.zeilen_mit(" INFORMATION anhang Anhänge gelöscht");
        assert_eq!(information.len(), 1, "{zeilen:#?}");
        assert!(
            information[0].contains("grund=nachricht"),
            "{information:#?}"
        );
    }

    /// R00043 B0666: scheitert das Kürzen, steht trotz Abbruch des Clients die `ERROR anhang`-Zeile.
    #[tokio::test(flavor = "multi_thread")]
    async fn der_abbruch_des_clients_verliert_auch_den_fehler_beim_kuerzen_nicht() {
        let aufbau = aufbau();
        let (zeit, _) = aufbau.nachricht_mit_anhang("beta@127.0.0.1");
        let datei = aufbau
            .anhaenge
            .anhaenge()
            .into_keys()
            .next()
            .expect("gebunden")
            .2;
        aufbau.anhaenge.kuerzen_scheitert(&datei);

        aufbau
            .abbrechen_am_kuerzen(aufbau.nachricht_loeschen("beta@127.0.0.1", zeit))
            .await;
        aufbau
            .warten_auf_zeile(" INFORMATION anhang Anhänge gelöscht")
            .await;

        let fehler = aufbau.zeilen_mit(" ERROR anhang Löschen gescheitert");
        assert_eq!(fehler.len(), 1, "{:#?}", aufbau.zeilen.zeilen());
    }

    async fn fehlertext(antwort: Response) -> String {
        let koerper = axum::body::to_bytes(antwort.into_body(), 4096)
            .await
            .expect("Körper");
        let json: Value = serde_json::from_slice(&koerper).expect("JSON");
        json["fehler"].as_str().expect("fehler").to_string()
    }

    /// R00043 B0667 (ADR-0048 K3): eine geschlossene Hochladebank antwortet 500 ohne Fremdtext und schreibt genau
    /// einen Eintrag `EXCEPTION programmfehler … grund=werkbank_geschlossen operation=nachricht_loeschen`.
    #[tokio::test(flavor = "multi_thread")]
    async fn eine_geschlossene_hochladebank_beim_loeschen_wird_beantwortet_und_protokolliert() {
        let aufbau = aufbau();
        let (zeit, _) = aufbau.nachricht_mit_anhang("beta@127.0.0.1");
        aufbau.zustand.hochladebank.close();

        let antwort = aufbau.nachricht_loeschen("beta@127.0.0.1", zeit).await;

        assert_eq!(antwort.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            fehlertext(antwort).await,
            "Interner Fehler — Nachricht nicht gelöscht; bitte erneut versuchen"
        );
        let eintraege = aufbau.zeilen_mit(" programmfehler ");
        assert_eq!(eintraege.len(), 1, "{eintraege:#?}");
        assert!(
            eintraege[0].contains(" EXCEPTION programmfehler "),
            "{eintraege:#?}"
        );
        assert!(
            eintraege[0].contains("route=\"POST /api/channels/{name}/nachrichten/loeschen\" operation=nachricht_loeschen status=500 kennung=beta@127.0.0.1 channel=allgemein grund=werkbank_geschlossen"),
            "{eintraege:#?}"
        );
    }

    /// R00043 B0667: eine Panik im Blocking-Teil (Testknopf `loeschen`) antwortet 500 ohne Fremdtext und schreibt
    /// genau einen `ERROR programmfehler … grund=panik`; die Erlaubnis kehrt zurück, danach löscht und sendet es
    /// unverändert.
    #[tokio::test(flavor = "multi_thread")]
    async fn eine_panik_beim_loeschen_wird_beantwortet_und_danach_geht_alles_weiter() {
        let panik = Testpanik::aus_wert(Some(OsStr::new("loeschen"))).expect("gültig");
        let aufbau = aufbau_mit(Arc::new(FesteFolge::neu(1..=100)), panik);
        let (zeit, _) = aufbau.nachricht_mit_anhang("beta@127.0.0.1");
        let frei_vorher = aufbau.zustand.hochladebank.available_permits();

        let antwort = aufbau.nachricht_loeschen("beta@127.0.0.1", zeit).await;

        assert_eq!(antwort.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let text = fehlertext(antwort).await;
        assert_eq!(
            text,
            "Interner Fehler — Nachricht nicht gelöscht; bitte erneut versuchen"
        );
        assert!(!text.contains("Testpanik"), "{text}");
        let eintraege = aufbau.zeilen_mit(" programmfehler ");
        assert_eq!(eintraege.len(), 1, "{eintraege:#?}");
        assert!(
            eintraege[0].contains(" ERROR programmfehler "),
            "{eintraege:#?}"
        );
        assert!(
            eintraege[0].contains("operation=nachricht_loeschen status=500 kennung=beta@127.0.0.1 channel=allgemein grund=panik"),
            "{eintraege:#?}"
        );
        assert_eq!(aufbau.zustand.hochladebank.available_permits(), frei_vorher);
        let gesendet = aufbau.zustand.gespraeche.senden(
            &Gespraech::channel("allgemein"),
            &teilnehmer_kennung("beta@127.0.0.1"),
            crate::anwesenheit::Art::Agent,
            "weiter",
        );
        assert!(gesendet.is_ok(), "{gesendet:?}");
        let erneut = aufbau.nachricht_loeschen("beta@127.0.0.1", zeit).await;
        assert_eq!(erneut.status(), StatusCode::OK);
    }

    fn teilnehmer_kennung(text: &str) -> crate::gespraeche::Kennung {
        crate::gespraeche::Kennung::bilden(text).expect("Kennung")
    }

    /// R00043 B0669 (ADR-0048 K5): über den echten Einstieg (Router, HTTP, Health) — acht Erlaubnisse der Hochladebank
    /// sind belegt (`hochladebank_belegt` 8); das Löschen einer Nachricht **ohne** Anhang antwortet 200 in unter 500 ms,
    /// das einer Nachricht **mit** Anhang wartet und gelingt, sobald eine Erlaubnis frei wird. Mutant „Erlaubnis
    /// immer“ rot. Die Bank lässt sich im laufenden Programm nicht von außen halten (die Erlaubnis lebt nur während
    /// eines Stücks im Blocking-Thread), darum hält der Test sie am Zustand der Instanz.
    #[tokio::test(flavor = "multi_thread")]
    async fn loeschen_ohne_anhang_wartet_nicht_auf_die_hochladebank_mit_anhang_schon() {
        use crate::http::HOCHLADEBANK;
        let aufbau = aufbau();
        let (mit_anhang, _) = aufbau.nachricht_mit_anhang("beta@127.0.0.1");
        let ohne_anhang = aufbau
            .zustand
            .gespraeche
            .senden(
                &Gespraech::channel("allgemein"),
                &teilnehmer_kennung("beta@127.0.0.1"),
                crate::anwesenheit::Art::Agent,
                "ohne Anhang",
            )
            .expect("gesendet")
            .zeit;
        let basis =
            crate::http::testserver::app_starten(crate::http::router((*aufbau.zustand).clone()))
                .await;
        let client = reqwest::Client::new();
        let beitritt: Value = client
            .post(format!("{basis}api/beitreten"))
            .json(&json!({"name": "beta"}))
            .send()
            .await
            .expect("Antwort")
            .json()
            .await
            .expect("JSON");
        let token = beitritt["token"].as_str().expect("Token").to_string();
        let loeschen =
            |zeit: Zeit| {
                client
                .post(format!("{basis}api/channels/allgemein/nachrichten/loeschen"))
                .header("X-LocalchatA-Sitzung", &token)
                .json(&json!({"zeit": zeit.to_rfc3339_opts(chrono::SecondsFormat::Millis, false)}))
                .send()
            };
        let belegt = || async {
            let gesundheit: Value = client
                .get(format!("{basis}api/gesundheit"))
                .send()
                .await
                .expect("Antwort")
                .json()
                .await
                .expect("JSON");
            gesundheit["hochladebank_belegt"].as_u64().expect("belegt")
        };
        let erlaubnisse = Arc::clone(&aufbau.zustand.hochladebank)
            .acquire_many_owned(HOCHLADEBANK as u32)
            .await
            .expect("acht Erlaubnisse");
        assert_eq!(belegt().await, HOCHLADEBANK as u64);

        let beginn = Instant::now();
        let ohne = tokio::time::timeout(Duration::from_secs(5), loeschen(ohne_anhang))
            .await
            .expect("ohne Anhang wartet nicht auf die Bank")
            .expect("Antwort");
        let dauer = beginn.elapsed();
        let mit = tokio::spawn(loeschen(mit_anhang));
        tokio::time::sleep(Duration::from_millis(300)).await;
        let wartet = !mit.is_finished();
        drop(erlaubnisse);
        let mit = tokio::time::timeout(Duration::from_secs(5), mit)
            .await
            .expect("nach dem Ende einer Hochladung")
            .expect("Aufgabe")
            .expect("Antwort");

        assert_eq!(ohne.status(), 200);
        assert!(dauer < Duration::from_millis(500), "{dauer:?}");
        assert!(wartet, "mit Anhang wartet auf die Hochladebank");
        assert_eq!(mit.status(), 200);
        let antwort: Value = mit.json().await.expect("JSON");
        assert_eq!(antwort["anhaenge"]["geloescht"], 1, "{antwort}");
    }

    /// Ein weiteres Feld wird genannt, auch `bestaetigt` (ADR-0048 Regel 12); nur `zeit` ist erlaubt.
    #[test]
    fn nur_zeit_ist_ein_erlaubtes_feld() {
        assert_eq!(
            weiteres_feld(&json!({"zeit": "x", "bestaetigt": true})),
            Some("bestaetigt".to_string())
        );
        assert_eq!(weiteres_feld(&json!({"zeit": "x"})), None);
        assert_eq!(weiteres_feld(&json!([1])), None);
    }
}
