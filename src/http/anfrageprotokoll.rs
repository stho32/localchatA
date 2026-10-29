//! Debug je Anfrage (ADR-0042 Regel 2, letzte Zeile): jede einzelne 400, 401, 404, 405, 409, 413, 415 und 422 als
//! `DEBUG` mit Routenmuster, Status, Herkunft und Grund. Nur bei 404 auf unbekannte Pfade steht der Pfad selbst
//! (dekodiert, im Protokoll escaped und gekürzt). Körperfehler trägt `json_lesen` selbst ein — ihre Antwort
//! trägt die Marke `Eingetragen` und wird hier übergangen. Der Grund ist ein Regelschlüssel (ADR-0042 Ergänzung
//! #24 Punkt 3): aus der Erweiterung `Abweisung` des abweisenden Handlers, sonst einer je Status.

use crate::protokoll::{Bereich, Kontext, Protokoll, Schluessel, Stufe};
use axum::extract::{ConnectInfo, MatchedPath, Request, State};
use axum::middleware::Next;
use axum::response::Response;
use std::net::SocketAddr;
use std::sync::Arc;

/// Marke an einer Antwort, deren Abweisung schon im Protokoll steht.
#[derive(Debug, Clone, Copy)]
pub struct Eingetragen;

/// Erweiterung an einer abweisenden Antwort: welche Regel griff (festes ASCII-Wort in `snake_case` aus dem Code,
/// nie eine Eingabe) und, falls bekannt, wer anfragte.
#[derive(Debug, Clone)]
pub struct Abweisung {
    pub regel: &'static str,
    pub kennung: Option<String>,
}

/// Hängt die Regel und die Kennung an eine abweisende Antwort (ADR-0042 Ergänzung #24 Punkt 3).
pub fn mit_regel(mut antwort: Response, regel: &'static str, kennung: Option<&str>) -> Response {
    antwort.extensions_mut().insert(Abweisung {
        regel,
        kennung: kennung.map(str::to_string),
    });
    antwort
}

/// Die Schicht: lässt die Anfrage durch und trägt eine Abweisung aus der Liste ein.
pub async fn schicht(
    State(protokoll): State<Arc<Protokoll>>,
    anfrage: Request,
    weiter: Next,
) -> Response {
    if protokoll.mindeststufe() > Stufe::Debug {
        return weiter.run(anfrage).await;
    }
    let methode = anfrage.method().clone();
    let muster = anfrage
        .extensions()
        .get::<MatchedPath>()
        .map(|pfad| pfad.as_str().to_string());
    let pfad = anfrage.uri().path().to_string();
    let herkunft = anfrage
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ConnectInfo(gegenstelle)| gegenstelle.ip());
    let antwort = weiter.run(anfrage).await;
    let status = antwort.status().as_u16();
    let Some(je_status) = grund(status) else {
        return antwort;
    };
    if antwort.extensions().get::<Eingetragen>().is_some() {
        return antwort;
    }
    let abweisung = antwort.extensions().get::<Abweisung>();
    let grund = abweisung.map_or(je_status, |abweisung| abweisung.regel);
    let ziel = muster.clone().unwrap_or_else(|| prozent_dekodiert(&pfad));
    let mut kontext = Kontext::neu()
        .mit(Schluessel::Route, format!("{methode} {ziel}"))
        .mit(Schluessel::Status, status);
    match (
        abweisung.and_then(|abweisung| abweisung.kennung.as_deref()),
        herkunft,
    ) {
        (Some(kennung), _) => kontext = kontext.mit(Schluessel::Kennung, kennung),
        (None, Some(herkunft)) => kontext = kontext.mit(Schluessel::Herkunft, herkunft),
        (None, None) => {}
    }
    protokoll.eintragen(
        Stufe::Debug,
        bereich(muster.as_deref().unwrap_or(&pfad)),
        "Anfrage abgewiesen",
        kontext.mit(Schluessel::Grund, grund),
    );
    antwort
}

/// Regelschlüssel je Status aus der Liste, wenn der Handler keinen nennt (Router-404, -405); andere Status trägt
/// die Schicht nicht ein.
fn grund(status: u16) -> Option<&'static str> {
    Some(match status {
        400 => "ungueltig",
        401 => "nicht_angemeldet",
        404 => "unbekannt",
        405 => "methode",
        409 => "konflikt",
        413 => "zu_gross",
        415 => "kein_json",
        422 => "regel",
        507 => "quote",
        _ => return None,
    })
}

fn bereich(pfad: &str) -> Bereich {
    if pfad.contains("/nachrichten") {
        Bereich::Verlauf
    } else if pfad.starts_with("/api/channels") {
        Bereich::Channels
    } else if [
        "/api/beitreten",
        "/api/ich",
        "/api/verlassen",
        "/api/teilnehmer",
    ]
    .iter()
    .any(|sitzung| pfad.starts_with(sitzung))
    {
        Bereich::Sitzung
    } else {
        Bereich::Verbindung
    }
}

/// `%0a` → Zeilenumbruch usw.; ungültige Folgen bleiben, wie sie sind; Bytes als UTF-8 (verlustbehaftet).
fn prozent_dekodiert(pfad: &str) -> String {
    let bytes = pfad.as_bytes();
    let mut ergebnis = Vec::with_capacity(bytes.len());
    let mut stelle = 0;
    while stelle < bytes.len() {
        let hex = bytes
            .get(stelle + 1..stelle + 3)
            .and_then(|paar| std::str::from_utf8(paar).ok())
            .and_then(|paar| u8::from_str_radix(paar, 16).ok());
        match (bytes[stelle], hex) {
            (b'%', Some(wert)) => {
                ergebnis.push(wert);
                stelle += 3;
            }
            (byte, _) => {
                ergebnis.push(byte);
                stelle += 1;
            }
        }
    }
    String::from_utf8_lossy(&ergebnis).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prozent_folgen_werden_dekodiert_ungueltige_bleiben() {
        assert_eq!(
            prozent_dekodiert("/api/gibtsnicht%0aX"),
            "/api/gibtsnicht\nX"
        );
        assert_eq!(prozent_dekodiert("/a%zz%4"), "/a%zz%4");
        assert_eq!(prozent_dekodiert("/%C3%A4"), "/ä");
    }

    /// R00018 F0059: die Schlüssel je Status sind feste ASCII-Wörter in `snake_case`.
    #[test]
    fn die_schluessel_je_status_sind_snake_case() {
        for status in [400, 401, 404, 405, 409, 413, 415, 422] {
            let grund = grund(status).expect("Grund");
            assert!(
                grund.bytes().all(|z| z.is_ascii_lowercase() || z == b'_'),
                "{status}: {grund}"
            );
        }
        assert_eq!(grund(405), Some("methode"));
        assert_eq!(grund(401), Some("nicht_angemeldet"));
    }

    #[test]
    fn nur_die_status_der_liste_haben_einen_grund() {
        for status in [400, 401, 404, 405, 409, 413, 415, 422] {
            assert!(grund(status).is_some(), "{status}");
        }
        for status in [200, 201, 204, 403, 408, 429, 500] {
            assert!(grund(status).is_none(), "{status}");
        }
    }
}
