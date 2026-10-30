//! Warten (ADR-0054, R00049): `GET /api/warten?seit=<zeit>&bis=<s>[&channel=<name>]…[&direkt][&erwaehnung]`.
//!
//! Die Parameter werden vor jeder Arbeit geprüft, abgewiesen statt umgeformt (Regel 1, W3); der Aufruf ist ein Halter
//! der Sitzung (Regel 7) und zählt in `wartend` (Regel 10).

use super::Zustand;
use super::abbruch::{Grenzort, abbruch_beantworten};
use super::anfrageprotokoll::mit_regel;
use super::nachholplatz::{
    Platz, gebremst_auftreten as nachholen_gebremst_auftreten,
    gebremst_beantworten as nachholen_gebremst_beantworten,
};
use super::nachrichten::{NachrichtAntwort, dekodieren};
use super::teilnehmer::{gemeldetes_token, nicht_erkannt};
use super::teilnehmerliste::zeit_als_text;
use crate::gespraeche::treffer::{Filter, Treffer};
use crate::gespraeche::{Gespraech, Kennung};
use crate::live::warten::{Ende, Warteauftrag, Warteergebnis, warten};
use crate::protokoll::episoden::{Auftreten, Quelle};
use crate::protokoll::episodentexte;
use crate::protokoll::{Bereich, Schluessel, Stufe};
use crate::uhr::Zeit;
use axum::Json;
use axum::extract::{ConnectInfo, RawQuery, State};
use axum::http::header::{CACHE_CONTROL, RETRY_AFTER};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use serde_json::json;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

/// Ohne `bis` wartet ein Aufruf so lange (ADR-0054 E4).
pub const BIS_STANDARD: Duration = Duration::from_secs(60);
/// Höchstens so viele Sekunden darf `bis` sein (ADR-0054 E4).
pub const BIS_HOECHSTENS: u64 = 3600;
/// Höchstens so viele `channel` je Aufruf (ADR-0054 Regel 1).
pub const CHANNELS_HOECHSTENS: usize = 64;

/// Ein geprüfter Aufruf: `seit` (ohne: „ab jetzt“), `bis`, genannte Channels und die Schalter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Warteparameter {
    pub seit: Option<Zeit>,
    pub bis: Duration,
    pub channels: Vec<String>,
    pub direkt: bool,
    pub erwaehnung: bool,
}

/// Welche Regel ein Aufruf verletzt (422 `regel`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Parameterfehler {
    /// `seit` ist keine Zeit nach RFC 3339 (auch: unkodiertes `+`).
    Cursor,
    /// `bis` ist keine ganze Zahl von 0 bis 3600.
    Bis,
    /// Schalter mit Wert, mehr als 64 `channel`, doppeltes `seit`, `bis` oder doppelter Schalter.
    Filter,
    /// Ein Parameter, den es nicht gibt — mit seinem Namen.
    Parameter(String),
}

impl Parameterfehler {
    /// Der Grund im Betriebsprotokoll: die verletzte Regel (ohne den Namen aus der Anfrage).
    fn grund(&self) -> &'static str {
        match self {
            Parameterfehler::Cursor => "abfrage_cursor",
            Parameterfehler::Bis => "abfrage_bis",
            Parameterfehler::Filter => "abfrage_filter",
            Parameterfehler::Parameter(_) => "abfrage_parameter",
        }
    }
}

/// Der Query-Text (ohne `?`) als Warteparameter.
pub fn parameter_lesen(query: Option<&str>) -> Result<Warteparameter, Parameterfehler> {
    let mut seit = None;
    let mut bis = None;
    let mut parameter = Warteparameter {
        seit: None,
        bis: BIS_STANDARD,
        channels: Vec::new(),
        direkt: false,
        erwaehnung: false,
    };
    for paar in query
        .unwrap_or_default()
        .split('&')
        .filter(|paar| !paar.is_empty())
    {
        let (schluessel, wert) = match paar.split_once('=') {
            Some((schluessel, wert)) => (schluessel, Some(wert)),
            None => (paar, None),
        };
        let schluessel = dekodieren(schluessel).unwrap_or_else(|| schluessel.to_string());
        match (schluessel.as_str(), wert) {
            ("seit", Some(wert)) => einmal(&mut seit, cursor_lesen(wert)?)?,
            ("bis", Some(wert)) => einmal(&mut bis, bis_lesen(wert)?)?,
            ("channel", Some(wert)) if parameter.channels.len() < CHANNELS_HOECHSTENS => parameter
                .channels
                .push(dekodieren(wert).ok_or(Parameterfehler::Filter)?),
            ("direkt", None) if !parameter.direkt => parameter.direkt = true,
            ("erwaehnung", None) if !parameter.erwaehnung => parameter.erwaehnung = true,
            ("seit" | "bis" | "channel" | "direkt" | "erwaehnung", _) => {
                return Err(Parameterfehler::Filter);
            }
            (unbekannt, _) => return Err(Parameterfehler::Parameter(unbekannt.to_string())),
        }
    }
    parameter.seit = seit;
    parameter.bis = bis.unwrap_or(BIS_STANDARD);
    Ok(parameter)
}

/// Setzt einen Wert, der höchstens einmal kommen darf; ein zweites Mal ist `filter`.
fn einmal<T>(platz: &mut Option<T>, wert: T) -> Result<(), Parameterfehler> {
    if platz.is_some() {
        return Err(Parameterfehler::Filter);
    }
    *platz = Some(wert);
    Ok(())
}

/// Eine Zeit nach RFC 3339 mit Versatz; ein unkodiertes `+` ist ein Leerzeichen und damit unlesbar.
fn cursor_lesen(wert: &str) -> Result<Zeit, Parameterfehler> {
    dekodieren(wert)
        .and_then(|text| Zeit::parse_from_rfc3339(&text).ok())
        .ok_or(Parameterfehler::Cursor)
}

/// Nur Ziffern (kein Vorzeichen, kein Punkt), höchstens 3600.
fn bis_lesen(wert: &str) -> Result<Duration, Parameterfehler> {
    let nur_ziffern =
        (1..=4).contains(&wert.len()) && wert.bytes().all(|byte| byte.is_ascii_digit());
    nur_ziffern
        .then(|| wert.parse::<u64>().ok())
        .flatten()
        .filter(|sekunden| *sekunden <= BIS_HOECHSTENS)
        .map(Duration::from_secs)
        .ok_or(Parameterfehler::Bis)
}

/// `GET /api/warten`: Sitzung (401, ohne Lebenszeichen) → Parameter (422) → Channels (404) → Grenze je Kennung
/// (429 `warten`) → Nachholkonto reservieren (429 `nachholen`) → Halter → warten → 200 `{nachrichten, seit, ende}`
/// mit `Cache-Control: no-store` (ADR-0054 Ergänzung „Nachholen von `warten` …“ Regel 5). Ein 429 hinterlässt
/// weder Halter noch Lebenszeichen. Bucht nichts am Schreibkonto (E9).
pub async fn beantworten(
    State(zustand): State<Arc<Zustand>>,
    ConnectInfo(gegenstelle): ConnectInfo<SocketAddr>,
    kopf: HeaderMap,
    RawQuery(query): RawQuery,
) -> Response {
    let herkunft = gegenstelle.ip();
    let Some(token) = gemeldetes_token(&kopf) else {
        return nicht_erkannt(&kopf);
    };
    let Some(sitzung) = zustand
        .anwesenheit
        .sitzung_ohne_lebenszeichen(&token, herkunft)
    else {
        return nicht_erkannt(&kopf);
    };
    let kennung = sitzung.kennung.als_text();
    let parameter = match parameter_lesen(query.as_deref()) {
        Ok(parameter) => parameter,
        Err(fehler) => return mit_regel(abgewiesen(&fehler), fehler.grund(), Some(kennung)),
    };
    if let Some(unbekannt) = parameter
        .channels
        .iter()
        .find(|channel| !zustand.gespraeche.channel_bekannt(channel))
    {
        return mit_regel(
            channel_unbekannt(unbekannt),
            "channel_unbekannt",
            Some(kennung),
        );
    }
    let Some(wartend) = zustand.wartende.zaehlen(kennung) else {
        zustand
            .protokoll
            .episode(gebremst_auftreten(kennung, AUFRUFE_JE_KENNUNG));
        return mit_regel(zu_viele_aufrufe(), "warten_grenze", Some(kennung));
    };
    let platz = match Platz::reservieren(
        &zustand.nachholbank,
        &zustand.nachholkonto,
        &sitzung.kennung,
        herkunft,
    ) {
        Ok(platz) => platz,
        Err(gebremst) => {
            zustand
                .protokoll
                .episode(nachholen_gebremst_auftreten(kennung, gebremst.grenze));
            return mit_regel(
                nachholen_gebremst_beantworten(gebremst, &sitzung.kennung, herkunft),
                "nachholen_grenze",
                Some(kennung),
            );
        }
    };
    let Some((halter, _veroeffentlicht)) = zustand.anwesenheit.verbunden(&token, herkunft) else {
        return nicht_erkannt(&kopf);
    };
    let auftrag = Warteauftrag {
        seit: parameter.seit,
        bis: parameter.bis,
        wartender: sitzung.kennung.clone(),
        filter: Filter {
            channels: parameter.channels.into_iter().collect(),
            direkt: parameter.direkt,
            erwaehnung: parameter.erwaehnung,
        },
        programmende: zustand.ende.clone(),
        protokoll: Arc::clone(&zustand.protokoll),
        sitzung_besteht: {
            let anwesenheit = Arc::clone(&zustand.anwesenheit);
            Arc::new(move || anwesenheit.besteht(&token))
        },
    };
    let ergebnis = warten(
        Arc::clone(&zustand.gespraeche),
        &zustand.bus,
        &platz,
        auftrag,
        Arc::new(|_| {}),
    )
    .await;
    drop(wartend);
    drop(halter);
    match ergebnis {
        Ok(ergebnis) => beantwortet(&ergebnis, &sitzung.kennung),
        Err(abbruch) => abbruch_beantworten(
            &zustand.protokoll,
            &abbruch,
            &Grenzort {
                route: ROUTE,
                operation: "warten",
                kennung: Some(kennung),
                ziel: None,
            },
            "Warten abgebrochen",
        ),
    }
}

const ROUTE: &str = "GET /api/warten";

/// Höchstens so viele gleichzeitige Warte-Aufrufe je Kennung (ADR-0054 Regel 8).
pub const AUFRUFE_JE_KENNUNG: usize = 4;
/// Nach so vielen Sekunden lohnt ein neuer Versuch über der Grenze je Kennung (`Retry-After`, ADR-0054 Regel 8).
pub const AUFRUFE_WIEDER_IN_S: u64 = 5;

/// Laufende Warte-Aufrufe je Kennung (ADR-0054 Regel 8 und 10, `wartend` im Health-Check); gezählt über einen
/// Platz, der beim Ende des Aufrufs fällt — auch, wenn der Client abbricht.
#[derive(Debug, Clone, Default)]
pub struct Wartende(Arc<Mutex<HashMap<String, usize>>>);

/// Ein belegter Platz einer Kennung unter den Wartenden; mit ihm fällt die Zählung.
pub struct Wartend {
    wartende: Wartende,
    kennung: String,
}

impl Wartende {
    /// Bucht einen Platz für `kennung`; `None`, wenn sie schon `AUFRUFE_JE_KENNUNG` Aufrufe offen hat.
    pub fn zaehlen(&self, kennung: &str) -> Option<Wartend> {
        let mut offen = self.offen();
        let belegt = offen.entry(kennung.to_string()).or_insert(0);
        if *belegt >= AUFRUFE_JE_KENNUNG {
            return None;
        }
        *belegt += 1;
        Some(Wartend {
            wartende: self.clone(),
            kennung: kennung.to_string(),
        })
    }

    pub fn anzahl(&self) -> usize {
        self.offen().values().sum()
    }

    fn offen(&self) -> MutexGuard<'_, HashMap<String, usize>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Drop for Wartend {
    fn drop(&mut self) {
        let mut offen = self.wartende.offen();
        if let Some(belegt) = offen.get_mut(&self.kennung) {
            *belegt = belegt.saturating_sub(1);
            if *belegt == 0 {
                offen.remove(&self.kennung);
            }
        }
    }
}

/// Das 429 am fünften Aufruf: `{fehler, grenze: "warten", hoechstens}` mit `Retry-After: 5` (`AUFRUFE_WIEDER_IN_S`).
fn zu_viele_aufrufe() -> Response {
    (
        StatusCode::TOO_MANY_REQUESTS,
        [(RETRY_AFTER, AUFRUFE_WIEDER_IN_S.to_string())],
        Json(json!({
            "fehler": format!("Zu viele gleichzeitige Warte-Aufrufe: höchstens {AUFRUFE_JE_KENNUNG} je Kennung — einen beenden oder in {AUFRUFE_WIEDER_IN_S} s noch einmal versuchen."),
            "grenze": "warten",
            "hoechstens": AUFRUFE_JE_KENNUNG,
        })),
    )
        .into_response()
}

/// Das Auftreten einer Abweisung an der Grenze: Quelle ist die Kennung, `hoechstens` steht nur im Beginn.
pub(crate) fn gebremst_auftreten(wer: &str, hoechstens: usize) -> Auftreten {
    Auftreten::neu(
        Stufe::Warning,
        Bereich::Warten,
        episodentexte::WARTEN_GEBREMST,
        Quelle::neu().mit(Schluessel::Kennung, wer),
    )
    .einzeln(Schluessel::Hoechstens, hoechstens)
}

/// `{channel, direkt, anlass, nachricht}` — ein Treffer an der Grenze (ADR-0054 E6).
#[derive(Serialize)]
struct Eintrag<'a> {
    channel: Option<&'a str>,
    direkt: Option<&'a str>,
    anlass: Vec<&'static str>,
    nachricht: NachrichtAntwort<'a>,
}

/// `{nachrichten, seit, ende}` (ADR-0054 Regel 4).
#[derive(Serialize)]
struct Warteantwort<'a> {
    nachrichten: Vec<Eintrag<'a>>,
    seit: Option<String>,
    ende: &'static str,
}

fn beantwortet(ergebnis: &Warteergebnis, wartender: &Kennung) -> Response {
    let antwort = Warteantwort {
        nachrichten: ergebnis
            .treffer
            .iter()
            .map(|treffer| eintrag(treffer, wartender))
            .collect(),
        seit: ergebnis.seit.as_ref().map(zeit_als_text),
        ende: match ergebnis.ende {
            Ende::Treffer => "treffer",
            Ende::Frist => "frist",
            Ende::Weitere => "weitere",
            Ende::Programmende => "programmende",
            Ende::Verlassen => "verlassen",
        },
    };
    ([(CACHE_CONTROL, "no-store")], Json(antwort)).into_response()
}

/// Ein Treffer aus Sicht von `wartender`: im Channel dessen Name und `direkt: null`, im Direktgespräch
/// `channel: null` und `direkt` = die Kennung des Gegenübers (ADR-0055 Regel 6, E8).
fn eintrag<'a>(treffer: &'a Treffer, wartender: &Kennung) -> Eintrag<'a> {
    let (channel, direkt) = match &treffer.gespraech {
        Gespraech::Channel(name) => (Some(name.as_str()), None),
        Gespraech::Direkt(paar) => (
            None,
            paar.gegenueber(wartender)
                .map(|gegenueber| gegenueber.als_text()),
        ),
    };
    Eintrag {
        channel,
        direkt,
        anlass: treffer
            .anlass
            .iter()
            .map(|anlass| anlass.als_text())
            .collect(),
        nachricht: NachrichtAntwort::aus(&treffer.nachricht),
    }
}

/// So viele Zeichen eines Namens aus der Anfrage nennt eine Abweisung höchstens.
const GENANNT_HOECHSTENS: usize = 64;

fn genannt(text: &str) -> String {
    text.chars().take(GENANNT_HOECHSTENS).collect()
}

fn abgewiesen(fehler: &Parameterfehler) -> Response {
    let koerper = match fehler {
        Parameterfehler::Cursor => json!({
            "fehler": "seit ist eine Zeit nach RFC 3339 mit Versatz, wie im Verlauf; + als %2B kodieren — zum Beispiel seit=2026-10-30T11:18:00.123%2B01:00.",
            "regel": "cursor",
        }),
        Parameterfehler::Bis => json!({
            "fehler": "bis ist eine ganze Zahl von 0 bis 3600 (Sekunden) — zum Beispiel bis=60.",
            "regel": "bis",
        }),
        Parameterfehler::Filter => json!({
            "fehler": "seit und bis je höchstens einmal, channel höchstens 64-mal; direkt und erwaehnung sind Schalter ohne Wert — zum Beispiel channel=technik&erwaehnung.",
            "regel": "filter",
        }),
        Parameterfehler::Parameter(name) => json!({
            "fehler": format!("Den Parameter {} gibt es nicht — erlaubt sind seit, bis, channel, direkt und erwaehnung.", genannt(name)),
            "regel": "parameter",
            "parameter": genannt(name),
        }),
    };
    (StatusCode::UNPROCESSABLE_ENTITY, Json(koerper)).into_response()
}

fn channel_unbekannt(name: &str) -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(json!({
            "fehler": format!("# {} gibt es nicht.", genannt(name)),
            "channel": genannt(name),
        })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gelesen(query: &str) -> Result<Warteparameter, Parameterfehler> {
        parameter_lesen(Some(query))
    }

    fn zeit(text: &str) -> Zeit {
        Zeit::parse_from_rfc3339(text).expect("Testzeit")
    }

    /// Ein Zustand wie im Programm mit `# allgemein`, einer Nachricht von `anna` und dem Token von `claude`.
    fn zustand_mit_claude() -> (super::super::Zustand, String) {
        use crate::anwesenheit::Beitrittswunsch;
        use crate::gespraeche::{Art, Kennung};
        let zustand =
            super::super::testserver::zustand(std::path::Path::new("/tmp/lc-nachholbank"));
        let wunsch = Beitrittswunsch::pruefen(&json!({"name": "claude"})).expect("Wunsch");
        let token = zustand
            .anwesenheit
            .beitreten(&wunsch, "127.0.0.1".parse().expect("IP"))
            .expect("beigetreten")
            .sitzung
            .token
            .als_text()
            .to_string();
        let anna = Kennung::bilden("anna@127.0.0.1").expect("Kennung");
        zustand
            .gespraeche
            .channel_anlegen("allgemein", None, &anna)
            .expect("angelegt");
        zustand
            .gespraeche
            .senden(
                &Gespraech::channel("allgemein"),
                &anna,
                Art::Mensch,
                "hallo",
            )
            .expect("gesendet");
        (zustand, token)
    }

    fn holen(
        basis: &str,
        pfad: &str,
        token: &str,
    ) -> impl Future<Output = reqwest::Result<reqwest::Response>> {
        reqwest::Client::new()
            .get(format!("{basis}{pfad}"))
            .header("X-LocalchatA-Sitzung", token)
            .send()
    }

    const NACHHOLEN_ALT: &str = "api/warten?bis=0&seit=2000-01-01T00:00:00.000Z";

    /// N2 (B0756, K2): hält der Test die Werkbank (ein Platz), antwortet `warten?bis=0&seit=<alt>` trotzdem binnen
    /// 1 s — das Nachholen läuft nie auf der Werkbank.
    #[tokio::test]
    async fn eine_gehaltene_werkbank_haelt_das_nachholen_nicht_auf() {
        let (mut zustand, token) = zustand_mit_claude();
        zustand.werkbank = super::super::Werkbank::mit(1);
        let gehalten = zustand.werkbank.erwerben().await.expect("Erlaubnis");
        let basis = super::super::testserver::app_starten(super::super::router(zustand)).await;

        let antwort =
            tokio::time::timeout(Duration::from_secs(1), holen(&basis, NACHHOLEN_ALT, &token))
                .await;

        let antwort = antwort
            .expect("Antwort trotz gehaltener Werkbank")
            .expect("HTTP");
        assert_eq!(antwort.status(), 200);
        let koerper: serde_json::Value = antwort.json().await.expect("JSON");
        assert_eq!(koerper["nachrichten"].as_array().map(Vec::len), Some(1));
        drop(gehalten);
    }

    /// N2 (B0756, K2): hält der Test die Nachholbank (ein Platz), antwortet der Verlauf `n=50` sofort, `warten` erst
    /// nach der Freigabe (Stil K7 in `nachrichten.rs`).
    #[tokio::test]
    async fn eine_gehaltene_nachholbank_haelt_den_verlauf_nicht_auf() {
        let (mut zustand, token) = zustand_mit_claude();
        zustand.nachholbank = super::super::Werkbank::mit(1);
        let gehalten = zustand.nachholbank.erwerben().await.expect("Erlaubnis");
        let basis = super::super::testserver::app_starten(super::super::router(zustand)).await;
        let warten = holen(&basis, NACHHOLEN_ALT, &token);
        let mut warten = std::pin::pin!(warten);

        let verlauf = tokio::time::timeout(
            Duration::from_secs(1),
            holen(&basis, "api/channels/allgemein/nachrichten?n=50", &token),
        )
        .await;
        let solange_gehalten = tokio::time::timeout(Duration::from_secs(1), &mut warten).await;
        assert!(
            solange_gehalten.is_err(),
            "warten trotz gehaltener Nachholbank"
        );
        drop(gehalten);
        let nach_freigabe = tokio::time::timeout(Duration::from_secs(1), &mut warten).await;

        assert_eq!(
            verlauf.expect("Verlauf binnen 1 s").expect("HTTP").status(),
            200
        );
        assert_eq!(
            nach_freigabe
                .expect("warten nach der Freigabe")
                .expect("HTTP")
                .status(),
            200
        );
    }

    /// W6c: vier Plätze je Kennung, der fünfte nicht; ein freigegebener Platz ist wieder da; andere Kennungen zählen
    /// für sich, `anzahl` ist die Summe.
    #[test]
    fn vier_plaetze_je_kennung_und_freigabe_beim_ende() {
        let wartende = Wartende::default();
        let mut claude: Vec<_> = (0..4)
            .map(|_| wartende.zaehlen("claude@127.0.0.1").expect("Platz"))
            .collect();

        assert!(wartende.zaehlen("claude@127.0.0.1").is_none());
        let mia = wartende.zaehlen("mia@127.0.0.1").expect("andere Kennung");
        assert_eq!(wartende.anzahl(), 5);
        claude.pop();
        assert_eq!(wartende.anzahl(), 4);
        assert!(wartende.zaehlen("claude@127.0.0.1").is_some());
        drop((claude, mia));
        assert_eq!(wartende.anzahl(), 0);
    }

    /// W3: ohne Angaben „ab jetzt“, 60 s, ohne Filter.
    #[test]
    fn ohne_angaben_gilt_ab_jetzt_sechzig_sekunden_und_kein_filter() {
        assert_eq!(
            parameter_lesen(None),
            Ok(Warteparameter {
                seit: None,
                bis: Duration::from_secs(60),
                channels: Vec::new(),
                direkt: false,
                erwaehnung: false,
            })
        );
        assert_eq!(gelesen("").map(|p| p.bis), Ok(Duration::from_secs(60)));
    }

    /// W3: `bis` 0 und 3600 gelten; 3601, -1, 1.5, abc und leer nicht.
    #[test]
    fn bis_gilt_von_null_bis_dreitausendsechshundert() {
        assert_eq!(gelesen("bis=0").map(|p| p.bis), Ok(Duration::ZERO));
        assert_eq!(
            gelesen("bis=3600").map(|p| p.bis),
            Ok(Duration::from_secs(3600))
        );
        for falsch in ["3601", "-1", "1.5", "abc", "", "%2B5", "99999999999"] {
            assert_eq!(
                gelesen(&format!("bis={falsch}")),
                Err(Parameterfehler::Bis),
                "bis={falsch}"
            );
        }
    }

    /// W3: `seit` als kodierte Zeit, auch in der Zukunft; `gestern` und ein unkodiertes `+` sind keine Zeit.
    #[test]
    fn seit_ist_eine_zeit_mit_kodiertem_plus() {
        assert_eq!(
            gelesen("seit=2026-10-30T11:18:00.123%2B01:00").map(|p| p.seit),
            Ok(Some(zeit("2026-10-30T11:18:00.123+01:00")))
        );
        assert_eq!(
            gelesen("seit=2099-01-01T00:00:00.000Z").map(|p| p.seit),
            Ok(Some(zeit("2099-01-01T00:00:00.000Z")))
        );
        for falsch in [
            "seit=gestern",
            "seit=2026-10-30T11:18:00.123+01:00",
            "seit=",
        ] {
            assert_eq!(gelesen(falsch), Err(Parameterfehler::Cursor), "{falsch}");
        }
    }

    /// W3: Channels wiederholbar bis 64; die Schalter ohne Wert.
    #[test]
    fn channels_und_schalter() {
        assert_eq!(
            gelesen("channel=technik&channel=allgemein&direkt&erwaehnung&bis=5"),
            Ok(Warteparameter {
                seit: None,
                bis: Duration::from_secs(5),
                channels: vec!["technik".to_string(), "allgemein".to_string()],
                direkt: true,
                erwaehnung: true,
            })
        );
        let vierundsechzig = vec!["channel=a"; 64].join("&");
        assert_eq!(gelesen(&vierundsechzig).map(|p| p.channels.len()), Ok(64));
    }

    /// W3: Schalter mit Wert, 65 × `channel`, doppeltes `seit`, `bis` oder doppelter Schalter → `filter`.
    #[test]
    fn falsche_filter_werden_abgewiesen() {
        let fuenfundsechzig = vec!["channel=a"; 65].join("&");
        for falsch in [
            "erwaehnung=1",
            "erwaehnung=",
            "direkt=ja",
            fuenfundsechzig.as_str(),
            "seit=2026-10-30T11:18:00.123%2B01:00&seit=2026-10-30T11:18:00.124%2B01:00",
            "bis=1&bis=2",
            "erwaehnung&erwaehnung",
        ] {
            assert_eq!(gelesen(falsch), Err(Parameterfehler::Filter), "{falsch}");
        }
    }

    /// W3: ein unbekannter Parameter wird mit seinem Namen abgewiesen, nie übergangen.
    #[test]
    fn ein_unbekannter_parameter_wird_mit_namen_abgewiesen() {
        assert_eq!(
            gelesen("bis=5&erwahnung"),
            Err(Parameterfehler::Parameter("erwahnung".to_string()))
        );
        assert_eq!(
            gelesen("n=100"),
            Err(Parameterfehler::Parameter("n".to_string()))
        );
    }
}
