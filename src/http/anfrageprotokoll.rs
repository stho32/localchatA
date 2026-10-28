//! Debug je Anfrage (ADR-0042 Regel 2, letzte Zeile): jede einzelne 400, 401, 404, 405, 409, 413, 415 und 422 als
//! `DEBUG` mit Routenmuster, Status, Herkunft und Grund. Nur bei 404 auf unbekannte Pfade steht der Pfad selbst
//! (dekodiert, im Protokoll escaped und gekürzt). Körperfehler trägt `json_lesen` selbst ein — ihre Antwort
//! trägt die Marke `Eingetragen` und wird hier übergangen.

use crate::protokoll::{Bereich, Kontext, Protokoll, Schluessel, Stufe};
use axum::extract::{ConnectInfo, MatchedPath, Request, State};
use axum::middleware::Next;
use axum::response::Response;
use std::net::SocketAddr;
use std::sync::Arc;

/// Marke an einer Antwort, deren Abweisung schon im Protokoll steht.
#[derive(Debug, Clone, Copy)]
pub struct Eingetragen;

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
    let Some(grund) = grund(status) else {
        return antwort;
    };
    if antwort.extensions().get::<Eingetragen>().is_some() {
        return antwort;
    }
    let ziel = muster.clone().unwrap_or_else(|| prozent_dekodiert(&pfad));
    let mut kontext = Kontext::neu()
        .mit(Schluessel::Route, format!("{methode} {ziel}"))
        .mit(Schluessel::Status, status);
    if let Some(herkunft) = herkunft {
        kontext = kontext.mit(Schluessel::Herkunft, herkunft);
    }
    protokoll.eintragen(
        Stufe::Debug,
        bereich(muster.as_deref().unwrap_or(&pfad)),
        "Anfrage abgewiesen",
        kontext.mit(Schluessel::Grund, grund),
    );
    antwort
}

/// Grund je Status aus der Liste; andere Status trägt die Schicht nicht ein.
fn grund(status: u16) -> Option<&'static str> {
    Some(match status {
        400 => "ungültige Anfrage",
        401 => "nicht angemeldet",
        404 => "unbekannt",
        405 => "Methode nicht erlaubt",
        409 => "Konflikt",
        413 => "zu groß",
        415 => "kein JSON",
        422 => "Regel verletzt",
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
