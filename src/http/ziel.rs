//! Das Gespräch einer Route (A00004 „Gespräch im Pfad“, ADR-0055 Regel 3): ein Channel aus dem Pfad
//! (`/api/channels/{name}/…`) oder ein Direktgespräch aus **Sitzung und Pfad** (`/api/direkt/{kennung}/…`). Dieselben
//! Handler bedienen beide; die Zugriffsgrenze des Direktgesprächs gilt durch den Bau: das Paar entsteht aus der
//! Kennung der Sitzung und dem Gegenüber im Pfad, ein fremdes Gespräch ist gar nicht nennbar.

use super::Zustand;
use super::anfrageprotokoll::mit_regel;
use super::teilnehmer::Teilnehmer;
use crate::gespraeche::{Direktpaar, Gespraech, Kennung};
use crate::protokoll::Schluessel;
use axum::Json;
use axum::extract::FromRequestParts;
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};
use std::convert::Infallible;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub(super) enum Ziel {
    Channel(String),
    Direkt {
        paar: Direktpaar,
        gegenueber: Kennung,
    },
}

impl Ziel {
    /// Der Name für Protokoll und Fehlertexte: der Channelname, bei einem Direktgespräch `direkt/<ordner>` —
    /// nie die Kennung des Gegenübers allein, nie ein Text.
    pub(super) fn name(&self) -> String {
        match self {
            Ziel::Channel(name) => name.clone(),
            Ziel::Direkt { paar, .. } => format!("direkt/{}", paar.ordnername()),
        }
    }

    pub(super) fn gespraech(&self) -> Gespraech {
        match self {
            Ziel::Channel(name) => Gespraech::channel(name),
            Ziel::Direkt { paar, .. } => Gespraech::Direkt(paar.clone()),
        }
    }

    /// `channel` einer Antwort: der Name oder `null`.
    pub(super) fn channel_json(&self) -> Value {
        match self {
            Ziel::Channel(name) => json!(name),
            Ziel::Direkt { .. } => Value::Null,
        }
    }

    /// `direkt` einer Antwort: die Kennung des Gegenübers; bei einem Channel fehlt das Feld (Channel-Antworten
    /// bleiben, wie sie waren).
    pub(super) fn gegenueber(&self) -> Option<&Kennung> {
        match self {
            Ziel::Channel(_) => None,
            Ziel::Direkt { gegenueber, .. } => Some(gegenueber),
        }
    }

    /// Die Antwort `{channel, [direkt,] nachricht}` — Channels wie bisher, Direktgespräche mit `channel: null`
    /// und `direkt` (Matrix K1).
    pub(super) fn antwort_mit(&self, nachricht: Value) -> Value {
        match self.gegenueber() {
            None => json!({ "channel": self.channel_json(), "nachricht": nachricht }),
            Some(gegenueber) => json!({
                "channel": Value::Null,
                "direkt": gegenueber.als_text(),
                "nachricht": nachricht,
            }),
        }
    }
}

/// Der Schlüssel, unter dem ein Gespräch im Protokoll steht: `channel=` für einen Channel, `ordner=` für ein
/// Direktgespräch (ADR-0055 Regel 11) — erkannt am Namen aus `Ziel::name`; ein Channelname trägt nie einen `/`.
pub(super) fn bezug(name: &str) -> Schluessel {
    if name.starts_with("direkt/") {
        Schluessel::Ordner
    } else {
        Schluessel::Channel
    }
}

/// Das Pfadsegment hinter `/api/direkt/` **roh**, wie es ankam (prozentkodiert) — nicht von einem typisierten
/// Extraktor gelesen, der selbst ablehnt (axums 400 bei ungültigem UTF-8): die Antwort auf eine unzulässige Form
/// gibt erst `ziel_aus_pfad`, nach der Sitzung (B4, Matrix R1, W1).
pub(super) struct Segment(pub String);

impl<S: Send + Sync> FromRequestParts<S> for Segment {
    type Rejection = Infallible;

    async fn from_request_parts(teile: &mut Parts, _: &S) -> Result<Segment, Infallible> {
        let pfad = teile.uri.path();
        let rest = pfad.strip_prefix("/api/direkt/").unwrap_or_default();
        Ok(Segment(
            rest.split('/').next().unwrap_or_default().to_string(),
        ))
    }
}

/// Warum der Pfad kein Direktgespräch benennt (beide 422, abgewiesen, nie umgeformt).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Pfadfehler {
    /// Nicht `name@herkunft` in kanonischer Schreibweise.
    Kennung,
    /// Die eigene Kennung (PO 169).
    Selbst,
}

impl Pfadfehler {
    /// Die Antwort, mit der Regel und Kennung für das Anfrageprotokoll.
    pub(super) fn antwort(self, wer: &Kennung) -> Response {
        let wer = Some(wer.als_text());
        match self {
            Pfadfehler::Kennung => mit_regel(kennung_abgewiesen(), "direkt_kennung", wer),
            Pfadfehler::Selbst => mit_regel(selbst_abgewiesen(), "direkt_selbst", wer),
        }
    }
}

/// Das Direktgespräch von Sitzung und Pfad: das Segment einmal prozentdekodiert, dann die kanonische Kennung
/// (`:` darf roh oder als `%3A` kommen) → `Kennung`; die eigene Kennung → `Selbst`.
pub(super) fn ziel_aus_pfad(
    teilnehmer: &Teilnehmer,
    segment: &Segment,
) -> Result<Ziel, Pfadfehler> {
    let gegenueber = prozent_dekodiert(&segment.0)
        .and_then(|text| Kennung::kanonisch(&text).ok())
        .ok_or(Pfadfehler::Kennung)?;
    let paar =
        Direktpaar::bilden(&teilnehmer.kennung, &gegenueber).map_err(|_| Pfadfehler::Selbst)?;
    Ok(Ziel::Direkt { paar, gegenueber })
}

/// `%XX` als Byte, alles andere unverändert (ein `+` bleibt `+`); eine kaputte Folge oder kein UTF-8 ist `None`.
fn prozent_dekodiert(segment: &str) -> Option<String> {
    let bytes = segment.as_bytes();
    let mut ergebnis = Vec::with_capacity(bytes.len());
    let mut stelle = 0;
    while stelle < bytes.len() {
        if bytes[stelle] == b'%' {
            let paar = bytes.get(stelle + 1..stelle + 3)?;
            let hex = std::str::from_utf8(paar).ok()?;
            ergebnis.push(u8::from_str_radix(hex, 16).ok()?);
            stelle += 3;
        } else {
            ergebnis.push(bytes[stelle]);
            stelle += 1;
        }
    }
    String::from_utf8(ergebnis).ok()
}

fn kennung_abgewiesen() -> Response {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(json!({
            "fehler": "Die Kennung im Pfad ist nicht name@herkunft in kanonischer Schreibweise — zum Beispiel ben@192.168.0.8; die Herkunft ist eine Adresse, IPv6 kleingeschrieben und gekürzt.",
            "regel": "kennung",
        })),
    )
        .into_response()
}

fn selbst_abgewiesen() -> Response {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(json!({
            "fehler": "Eine Direktnachricht an sich selbst gibt es nicht — nenne die Kennung eines anderen Teilnehmers.",
            "regel": "selbst",
        })),
    )
        .into_response()
}

/// Ob die Kennung in der Teilnehmerliste steht (ADR-0055 Regel 3, PO 170): nur wer gerade da ist, kann ein neues
/// Gespräch bekommen.
pub(super) fn in_teilnehmerliste(zustand: &Arc<Zustand>, kennung: &Kennung) -> bool {
    zustand
        .anwesenheit
        .teilnehmer()
        .liste
        .iter()
        .any(|zeile| zeile.kennung == *kennung)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prozent_dekodiert_nur_prozent_folgen_und_nie_ein_plus() {
        assert_eq!(
            prozent_dekodiert("ben@127.0.0.3").as_deref(),
            Some("ben@127.0.0.3")
        );
        assert_eq!(prozent_dekodiert("a%40b").as_deref(), Some("a@b"));
        assert_eq!(prozent_dekodiert("fd00%3A%3A7").as_deref(), Some("fd00::7"));
        assert_eq!(prozent_dekodiert("a+b").as_deref(), Some("a+b"));
        assert_eq!(prozent_dekodiert("%252F").as_deref(), Some("%2F"), "einmal");
        for kaputt in ["%", "%4", "%GG", "%FF", "a%C3", "%00%"] {
            assert_eq!(prozent_dekodiert(kaputt), None, "{kaputt}");
        }
        assert_eq!(prozent_dekodiert("%00").as_deref(), Some("\0"));
    }

    #[test]
    fn der_bezug_im_protokoll_ist_channel_oder_ordner() {
        assert_eq!(bezug("allgemein"), Schluessel::Channel);
        assert_eq!(
            bezug("direkt/anna@10.0.0.7+ben@10.0.0.8"),
            Schluessel::Ordner
        );
    }
}
