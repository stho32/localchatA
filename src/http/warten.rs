//! Warten (ADR-0054, R00049): `GET /api/warten?seit=<zeit>&bis=<s>[&channel=<name>]…[&direkt][&erwaehnung]`.
//!
//! Die Parameter werden vor jeder Arbeit geprüft, abgewiesen statt umgeformt (Regel 1, W3).

use super::nachrichten::dekodieren;
use crate::uhr::Zeit;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn gelesen(query: &str) -> Result<Warteparameter, Parameterfehler> {
        parameter_lesen(Some(query))
    }

    fn zeit(text: &str) -> Zeit {
        Zeit::parse_from_rfc3339(text).expect("Testzeit")
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
