//! JSON-Körper lesen (Gate-Stabilität, Befund aus R00003): höchstens so viele Bytes, wie die Route als Grenze
//! nennt (R00010, ADR-0010: jede JSON-Route nennt ihre Grenze mit Herleitung). Ist der Körper größer,
//! liest der Server den Rest noch bis `VERWERFEN_HOECHSTENS` bzw. `VERWERFEN_FRIST` und verwirft ihn, bevor
//! er antwortet — schlösse er mit ungelesenen Bytes, setzte das Betriebssystem die Verbindung zurück, und der
//! Client sähe einen Abbruch statt der 413. Die Obergrenzen halten Großlast und Tröpfeln aus (ADR-0010).

use crate::protokoll::episoden::Auftreten;
use crate::protokoll::zaehler::Abweisungsart;
use crate::protokoll::{Bereich, Kontext, Protokoll, Schluessel, Stufe};
use axum::body::Body;
use axum::http::HeaderMap;
use axum::http::header::CONTENT_TYPE;
use futures_util::StreamExt;
use serde_json::Value;
use std::net::IpAddr;
use std::time::Duration;

const VERWERFEN_HOECHSTENS: usize = 16 * 1024 * 1024;
const VERWERFEN_FRIST: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Koerperfehler {
    /// Kein `Content-Type: application/json` (oder `…+json`).
    KeinJson,
    /// Gelesen, aber kein gültiges JSON.
    Kaputt,
    /// Lesefehler im Strom: der Client ist weg (ADR-0042, Ergänzung 2).
    Abgebrochen,
    /// Größer als die Grenze der Route — erkannt vor jedem Parse.
    ZuGross,
}

/// Wo ein Körper gelesen wird: Protokoll, Routenmuster, Herkunft, Kennung falls bekannt.
pub struct Koerperort<'a> {
    pub protokoll: &'a Protokoll,
    pub route: &'static str,
    pub herkunft: IpAddr,
    pub kennung: Option<&'a str>,
}

/// Wie der Rest eines zu großen Körpers verworfen wurde (ADR-0042, Ergänzung 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verworfen {
    Vollstaendig,
    Frist,
    Obergrenze,
}

impl Verworfen {
    fn als_text(self) -> &'static str {
        match self {
            Verworfen::Vollstaendig => "vollständig",
            Verworfen::Frist => "frist",
            Verworfen::Obergrenze => "obergrenze",
        }
    }
}

/// Liest den JSON-Körper wie `lesen`; jede Abweisung zählt in `abgewiesen` (ADR-0043, Ergänzung) und steht als
/// DEBUG `körper` mit Route, Status, Herkunft, Art, Bytes, Verwerfen und angekündigtem Typ im Protokoll —
/// zentral hier, nicht je Handler. Nie Körperinhalt.
pub async fn json_lesen(
    kopf: &HeaderMap,
    koerper: Body,
    grenze: usize,
    ort: &Koerperort<'_>,
) -> Result<Value, Koerperfehler> {
    let gelesen = lesen_mit_umstaenden(kopf, koerper, grenze).await;
    if let Err(fehler) = gelesen.ergebnis {
        abweisung_eintragen(kopf, ort, fehler, &gelesen);
    }
    gelesen.ergebnis
}

fn abweisung_eintragen(
    kopf: &HeaderMap,
    ort: &Koerperort<'_>,
    fehler: Koerperfehler,
    gelesen: &Gelesen,
) {
    let (art, status, abweisungsart) = match fehler {
        Koerperfehler::ZuGross => ("zu_gross", Some(413), Abweisungsart::ZuGross),
        Koerperfehler::KeinJson => ("kein_json", Some(415), Abweisungsart::KeinJson),
        Koerperfehler::Kaputt => ("kaputt", Some(400), Abweisungsart::Kaputt),
        Koerperfehler::Abgebrochen => ("abgebrochen", None, Abweisungsart::Abgebrochen),
    };
    ort.protokoll.abgewiesen(abweisungsart);
    let mut kontext = Kontext::neu().mit(Schluessel::Route, ort.route);
    if let Some(status) = status {
        kontext = kontext.mit(Schluessel::Status, status);
    }
    kontext = match ort.kennung {
        Some(kennung) => kontext.mit(Schluessel::Kennung, kennung),
        None => kontext.mit(Schluessel::Herkunft, ort.herkunft),
    };
    if ort.kennung.is_some() {
        kontext = kontext.mit(Schluessel::Herkunft, ort.herkunft);
    }
    kontext = kontext
        .mit(Schluessel::Art, art)
        .mit(Schluessel::Bytes, gelesen.bytes);
    if let Some(verworfen) = gelesen.verworfen {
        kontext = kontext.mit(Schluessel::Verworfen, verworfen.als_text());
    }
    if fehler == Koerperfehler::KeinJson {
        let typ = kopf
            .get(CONTENT_TYPE)
            .map(|wert| String::from_utf8_lossy(wert.as_bytes()).into_owned())
            .unwrap_or_default();
        kontext = kontext.mit(Schluessel::Typ, typ);
    }
    ort.protokoll
        .eintragen(Stufe::Debug, Bereich::Koerper, "Körper abgewiesen", kontext);
    let auftreten = Auftreten {
        stufe: Stufe::Warning,
        bereich: Bereich::Koerper,
        text: "Körper abgewiesen",
        quelle: format!("{} {}", ort.herkunft, ort.route),
        kontext: Kontext::neu()
            .mit(Schluessel::Herkunft, ort.herkunft)
            .mit(Schluessel::Route, ort.route),
        art: Some(art),
        verworfen: gelesen.verworfen.map(Verworfen::als_text),
        bytes: Some(gelesen.bytes as u64),
    };
    match fehler {
        Koerperfehler::ZuGross | Koerperfehler::Kaputt => ort.protokoll.episode(auftreten),
        Koerperfehler::Abgebrochen => ort.protokoll.episode_falls_offen(&auftreten),
        Koerperfehler::KeinJson => {}
    }
}

/// Ergebnis und Umstände des Lesens: gelesene Bytes (mit dem Verworfenen) und wie verworfen wurde.
struct Gelesen {
    ergebnis: Result<Value, Koerperfehler>,
    bytes: usize,
    verworfen: Option<Verworfen>,
}

/// Erst den Körper lesen (höchstens `grenze` Bytes), dann Typ und Inhalt prüfen — so ist er in jedem Fall
/// gelesen, bevor die Antwort geht, und über der Grenze wird nichts geparst.
#[cfg(test)]
async fn lesen(kopf: &HeaderMap, koerper: Body, grenze: usize) -> Result<Value, Koerperfehler> {
    lesen_mit_umstaenden(kopf, koerper, grenze).await.ergebnis
}

async fn lesen_mit_umstaenden(kopf: &HeaderMap, koerper: Body, grenze: usize) -> Gelesen {
    let mut strom = koerper.into_data_stream();
    let mut gelesen = Vec::new();
    while let Some(stueck) = strom.next().await {
        let Ok(stueck) = stueck else {
            return Gelesen {
                ergebnis: Err(Koerperfehler::Abgebrochen),
                bytes: gelesen.len(),
                verworfen: None,
            };
        };
        let gesamt = gelesen.len() + stueck.len();
        if gesamt > grenze {
            let (bytes, verworfen) =
                match tokio::time::timeout(VERWERFEN_FRIST, rest_verwerfen(&mut strom, gesamt))
                    .await
                {
                    Ok(ergebnis) => ergebnis,
                    Err(_) => (gesamt, Verworfen::Frist),
                };
            return Gelesen {
                ergebnis: Err(Koerperfehler::ZuGross),
                bytes,
                verworfen: Some(verworfen),
            };
        }
        gelesen.extend_from_slice(&stueck);
    }
    let ergebnis = if json_angekuendigt(kopf) {
        serde_json::from_slice(&gelesen).map_err(|_| Koerperfehler::Kaputt)
    } else {
        Err(Koerperfehler::KeinJson)
    };
    Gelesen {
        ergebnis,
        bytes: gelesen.len(),
        verworfen: None,
    }
}

async fn rest_verwerfen(
    strom: &mut axum::body::BodyDataStream,
    mut gesamt: usize,
) -> (usize, Verworfen) {
    while gesamt <= VERWERFEN_HOECHSTENS {
        match strom.next().await {
            Some(Ok(stueck)) => gesamt += stueck.len(),
            _ => return (gesamt, Verworfen::Vollstaendig),
        }
    }
    (gesamt, Verworfen::Obergrenze)
}

/// Wie axum: `application/json` oder ein Untertyp mit Endung `+json`, Parameter wie `charset` erlaubt.
fn json_angekuendigt(kopf: &HeaderMap) -> bool {
    let Some(wert) = kopf.get(CONTENT_TYPE).and_then(|wert| wert.to_str().ok()) else {
        return false;
    };
    let typ = wert
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    let Some(untertyp) = typ.strip_prefix("application/") else {
        return false;
    };
    untertyp == "json" || untertyp.ends_with("+json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn mit_typ(typ: &str) -> HeaderMap {
        let mut kopf = HeaderMap::new();
        kopf.insert(CONTENT_TYPE, HeaderValue::from_str(typ).expect("Typ"));
        kopf
    }

    #[test]
    fn json_ist_application_json_oder_plus_json_mit_parametern() {
        for typ in [
            "application/json",
            "application/json; charset=utf-8",
            "Application/JSON",
            "application/problem+json",
        ] {
            assert!(json_angekuendigt(&mit_typ(typ)), "{typ}");
        }
        for typ in [
            "text/plain",
            "application/xml",
            "multipart/form-data",
            "text/json",
            "application/jsonx",
        ] {
            assert!(!json_angekuendigt(&mit_typ(typ)), "{typ}");
        }
        assert!(!json_angekuendigt(&HeaderMap::new()));
    }

    /// R00010: genau an der Grenze angenommen, ein Byte darüber `ZuGross` — auch ungültiges JSON, denn die
    /// Grenze greift vor dem Parse. Zwei Grenzen, damit keine feste Zahl im Code den Test besteht.
    #[tokio::test]
    async fn grenze_je_aufruf_als_spanne() {
        let kopf = mit_typ("application/json");
        let gueltig = |laenge: usize| format!("{:<laenge$}", "{\"name\":\"spanne\"}");
        let kaputt = |laenge: usize| format!("{{{}", "x".repeat(laenge - 1));

        for grenze in [4096, 64] {
            assert!(
                lesen(&kopf, Body::from(gueltig(grenze)), grenze)
                    .await
                    .is_ok(),
                "{grenze} Bytes an der Grenze {grenze}"
            );
            for koerper in [gueltig(grenze + 1), kaputt(grenze + 1)] {
                assert_eq!(
                    lesen(&kopf, Body::from(koerper), grenze).await,
                    Err(Koerperfehler::ZuGross),
                    "{} Bytes an der Grenze {grenze}",
                    grenze + 1
                );
            }
        }
        assert_eq!(
            lesen(&kopf, Body::from(kaputt(4096)), 4096).await,
            Err(Koerperfehler::Kaputt)
        );
    }
}
