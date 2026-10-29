//! Debug je Anfrage (ADR-0042 Regel 2, letzte Zeile): jede einzelne 400, 401, 404, 405, 409, 413, 415 und 422 als
//! `DEBUG` mit Routenmuster, Status, Herkunft und Grund — im Bereich der Route nach Methode und Muster, mit
//! `channel=` aus dem Pfadparameter (Ergänzung R00019 Punkt 2). Nur bei 404 auf unbekannte Pfade steht der Pfad selbst
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
/// nie eine Eingabe), falls bekannt, wer anfragte, und weitere Schlüssel, die der Katalog für diese Abweisung verlangt.
#[derive(Debug, Clone)]
pub struct Abweisung {
    pub regel: &'static str,
    pub kennung: Option<String>,
    pub zusatz: Vec<(Schluessel, String)>,
}

/// Hängt die Regel und die Kennung an eine abweisende Antwort (ADR-0042 Ergänzung #24 Punkt 3).
pub fn mit_regel(antwort: Response, regel: &'static str, kennung: Option<&str>) -> Response {
    mit_regel_und(antwort, regel, kennung, Vec::new())
}

/// Wie `mit_regel`, mit weiteren Schlüsseln des Katalogs; sie stehen in der Zeile nach der Kennung, vor `grund`
/// (z. B. `anhaenge` der Rückfrage beim Löschen, ADR-0042 Katalog „Löschen nicht bestätigt“).
pub fn mit_regel_und(
    mut antwort: Response,
    regel: &'static str,
    kennung: Option<&str>,
    zusatz: Vec<(Schluessel, String)>,
) -> Response {
    antwort.extensions_mut().insert(Abweisung {
        regel,
        kennung: kennung.map(str::to_string),
        zusatz,
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
    let ziel = muster
        .clone()
        .unwrap_or_else(|| ids_gekuerzt(&prozent_dekodiert(&pfad)));
    let mut kontext = Kontext::neu()
        .mit(Schluessel::Route, format!("{methode} {ziel}"))
        .mit(Schluessel::Status, status);
    if let Some(channel) = channel_im_pfad(muster.as_deref(), &pfad) {
        kontext = kontext.mit(Schluessel::Channel, channel);
    }
    match (
        abweisung.and_then(|abweisung| abweisung.kennung.as_deref()),
        herkunft,
    ) {
        (Some(kennung), _) => kontext = kontext.mit(Schluessel::Kennung, kennung),
        (None, Some(herkunft)) => kontext = kontext.mit(Schluessel::Herkunft, herkunft),
        (None, None) => {}
    }
    for (schluessel, wert) in abweisung.map_or(&[][..], |abweisung| &abweisung.zusatz) {
        kontext = kontext.mit(*schluessel, wert);
    }
    protokoll.eintragen(
        Stufe::Debug,
        bereich(methode.as_str(), muster.as_deref().unwrap_or(&pfad)),
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

/// Der Pfadparameter `{name}` der passenden Route, dekodiert wie der Router; ohne Muster oder ohne `{name}` nichts
/// (ADR-0042 Ergänzung R00019 Punkt 2 — so trägt auch die 401 aus dem Extraktor `channel=`).
fn channel_im_pfad(muster: Option<&str>, pfad: &str) -> Option<String> {
    let stelle = muster?.split('/').position(|teil| teil == "{name}")?;
    pfad.split('/').nth(stelle).map(prozent_dekodiert)
}

/// Der Fachbereich der Route aus Methode und Routenmuster, die spezifischste Regel zuerst — derselbe Bereich, unter
/// dem ihre Information- und Error-Zeilen stehen (ADR-0042 Ergänzung R00019 Punkt 2). Ohne Muster (Router-404)
/// steht hier der Pfad.
fn bereich(methode: &str, muster: &str) -> Bereich {
    match (methode, muster) {
        ("PUT", "/api/channels/{name}/frist") => Bereich::Frist,
        ("POST", "/api/channels/{name}/nachrichten") => Bereich::Senden,
        ("GET", "/api/channels/{name}/nachrichten") => Bereich::Verlauf,
        (_, muster) if muster.starts_with("/api/channels") => Bereich::Channels,
        (_, muster) if muster.starts_with("/api/anhaenge") || muster.starts_with("/anhaenge") => {
            Bereich::Anhang
        }
        (_, muster)
            if [
                "/api/beitreten",
                "/api/ich",
                "/api/verlassen",
                "/api/teilnehmer",
            ]
            .iter()
            .any(|sitzung| muster.starts_with(sitzung)) =>
        {
            Bereich::Sitzung
        }
        _ => Bereich::Verbindung,
    }
}

/// Nach dem Dekodieren jedes Segment, das mit einer ID beginnt (26 Zeichen aus `[A-Za-z2-7]`), auf 6 Zeichen und
/// `…` (ADR-0047 Ergänzung Karte #9 Gate Regel 2) — in jedem Pfad, auch ohne Routenmuster.
fn ids_gekuerzt(pfad: &str) -> String {
    pfad.split('/')
        .map(crate::gespraeche::anhangid::im_protokoll)
        .collect::<Vec<_>>()
        .join("/")
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

    /// M18 (rein): nach dem Dekodieren gekürzt — prozentkodiert, groß geschrieben, mit Anhängsel; andere Segmente
    /// bleiben (Mutante „kürzen vor dem Dekodieren“: `%6B…` bliebe ganz).
    #[test]
    fn ids_im_pfad_werden_nach_dem_dekodieren_gekuerzt() {
        let id = "k3v7q2m5xpc4h6n6tewaz2b5yr";
        assert_eq!(
            ids_gekuerzt(&format!("/anhaenge/{id}/x")),
            "/anhaenge/k3v7q2…/x"
        );
        assert_eq!(
            ids_gekuerzt(&format!("/api/anhaenge/{id}/")),
            "/api/anhaenge/k3v7q2…/"
        );
        let kodiert = format!("/anhaenge/%6B{}", &id[1..]);
        assert_eq!(
            ids_gekuerzt(&prozent_dekodiert(&kodiert)),
            "/anhaenge/k3v7q2…"
        );
        assert_eq!(
            ids_gekuerzt(&format!("/x/{}", id.to_uppercase())),
            "/x/K3V7Q2…"
        );
        assert_eq!(ids_gekuerzt("/api/gibtsnicht"), "/api/gibtsnicht");
    }

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

    /// R00019 F0064 „Bereich nach Methode und Routenmuster“ (ADR-0042 Ergänzung R00019 Punkt 2, erster Test).
    #[test]
    fn der_bereich_folgt_methode_und_routenmuster_nicht_dem_pfadpraefix() {
        assert_eq!(bereich("PUT", "/api/channels/{name}/frist"), Bereich::Frist);
        assert_eq!(
            bereich("POST", "/api/channels/{name}/nachrichten"),
            Bereich::Senden
        );
        assert_eq!(
            bereich("GET", "/api/channels/{name}/nachrichten"),
            Bereich::Verlauf
        );
        assert_eq!(bereich("POST", "/api/channels"), Bereich::Channels);
        assert_eq!(bereich("GET", "/api/channels/x/y"), Bereich::Channels);
        assert_eq!(bereich("POST", "/api/beitreten"), Bereich::Sitzung);
        assert_eq!(bereich("GET", "/api/gibtsnicht"), Bereich::Verbindung);
    }

    /// R00019 F0064: `channel=` aus dem Pfadparameter `{name}` der passenden Route, dekodiert wie der Router.
    #[test]
    fn der_channel_kommt_aus_dem_pfadparameter_der_passenden_route() {
        assert_eq!(
            channel_im_pfad(
                Some("/api/channels/{name}/frist"),
                "/api/channels/allgemein/frist"
            )
            .as_deref(),
            Some("allgemein")
        );
        assert_eq!(
            channel_im_pfad(
                Some("/api/channels/{name}/nachrichten"),
                "/api/channels/a%0ab/nachrichten"
            )
            .as_deref(),
            Some("a\nb")
        );
        assert_eq!(
            channel_im_pfad(Some("/api/channels"), "/api/channels"),
            None
        );
        assert_eq!(channel_im_pfad(None, "/api/channels/x/y"), None);
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
