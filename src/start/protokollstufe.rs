//! Wirksame Protokollstufe (ADR-0043 Regeln 1–2, 4): `--protokoll-stufe` vor `LOCALCHATA_PROTOKOLL_STUFE` vor
//! dem Standard `information`; dazu, woher sie kommt. Rein.

use super::aufruf::{ERLAUBTE_STUFEN, stufe_lesen};
use crate::protokoll::{Stufe, in_anfuehrung};
use std::ffi::OsStr;
use std::fmt;

pub const VARIABLE: &str = "LOCALCHATA_PROTOKOLL_STUFE";

/// `LOCALCHATA_PROTOKOLL_STUFE` ist gesetzt, aber keine wählbare Stufe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stufenfehler {
    pub wert: String,
}

impl fmt::Display for Stufenfehler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{VARIABLE} ist keine Protokollstufe: {} — erlaubt sind {ERLAUBTE_STUFEN}",
            in_anfuehrung(&self.wert)
        )
    }
}

/// Stufe und Herkunft (`--protokoll-stufe` | `LOCALCHATA_PROTOKOLL_STUFE` | `standard`). Eine gültige Option
/// übersteuert: dann wird die Umgebung nicht gelesen, auch nicht geprüft (ADR-0043 Ergänzung Karte #26).
pub fn wirksame_stufe(
    option: Option<Stufe>,
    umgebung: Option<&OsStr>,
) -> Result<(Stufe, &'static str), Stufenfehler> {
    if let Some(stufe) = option {
        return Ok((stufe, "--protokoll-stufe"));
    }
    let Some(roh) = umgebung else {
        return Ok((Stufe::Information, "standard"));
    };
    let stufe = roh
        .to_str()
        .and_then(stufe_lesen)
        .ok_or_else(|| Stufenfehler {
            wert: roh.to_string_lossy().into_owned(),
        })?;
    Ok((stufe, VARIABLE))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// R00018 F0062: der Wert steht in „…“, Steuerzeichen escaped wie im Protokoll.
    #[test]
    fn die_meldung_nennt_den_wert_in_anfuehrung_mit_escape() {
        let meldung = Stufenfehler {
            wert: "la\u{1b}ut".to_string(),
        }
        .to_string();

        assert!(meldung.contains("„la\\u{1b}ut“"), "{meldung}");
        assert!(!meldung.contains('\u{1b}'), "{meldung}");
        assert!(!meldung.contains('"'), "{meldung}");
    }

    #[test]
    fn option_vor_umgebung_vor_standard() {
        assert_eq!(
            wirksame_stufe(Some(Stufe::Warning), Some(OsStr::new("debug"))),
            Ok((Stufe::Warning, "--protokoll-stufe"))
        );
        assert_eq!(
            wirksame_stufe(None, Some(OsStr::new("DEBUG"))),
            Ok((Stufe::Debug, VARIABLE))
        );
        assert_eq!(
            wirksame_stufe(None, None),
            Ok((Stufe::Information, "standard"))
        );
    }

    #[test]
    /// R00018 F0082 (ADR-0043 Ergänzung Karte #26, Option A): eine gültige Option übersteuert, die Umgebung wird
    /// nicht gelesen — auch nicht, wenn sie falsch ist (kehrt den Test aus R00014 um).
    fn eine_gueltige_option_uebersteuert_eine_falsche_umgebung() {
        for falsch in ["laut", "exception", ""] {
            assert_eq!(
                wirksame_stufe(Some(Stufe::Error), Some(OsStr::new(falsch))),
                Ok((Stufe::Error, "--protokoll-stufe")),
                "{falsch}"
            );
        }
    }

    #[test]
    fn ohne_option_ist_eine_falsche_umgebung_ein_fehler() {
        for falsch in ["laut", "exception", ""] {
            let fehler = wirksame_stufe(None, Some(OsStr::new(falsch))).expect_err(falsch);
            let text = fehler.to_string();
            assert!(text.contains(VARIABLE), "{text}");
            assert!(
                text.contains("debug, information, warning, error"),
                "{text}"
            );
        }
    }
}
