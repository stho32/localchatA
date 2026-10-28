//! Herzschlag aus der Umgebung (B0079, B0102): Standard 5 s, Testknopf `LOCALCHATA_HERZSCHLAG_MS` (ADR-0007,
//! ADR-0015, ADR-0017 Regel 1 — er erzeugt die unbestätigten Daten, an denen der Kernel den Abriss erkennt).

use std::ffi::OsStr;
use std::fmt;
use std::time::Duration;

pub const VARIABLE: &str = "LOCALCHATA_HERZSCHLAG_MS";
const STANDARD: Duration = Duration::from_secs(5);

/// `LOCALCHATA_HERZSCHLAG_MS` ist gesetzt, aber keine ganze Zahl von Millisekunden ab 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Herzschlagfehler {
    pub wert: String,
}

impl fmt::Display for Herzschlagfehler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{VARIABLE} ist keine ganze Zahl von Millisekunden ab 1: {} — z. B. 500",
            self.wert
        )
    }
}

/// Ohne Wert der Standard; ein Wert, der kein UTF-8, keine Zahl oder 0 ist, ist ein Fehler — nie still der
/// Standard (ein Takt von 0 ms gibt es nicht).
pub fn aus_umgebung(wert: Option<&OsStr>) -> Result<Duration, Herzschlagfehler> {
    let Some(roh) = wert else {
        return Ok(STANDARD);
    };
    let fehler = || Herzschlagfehler {
        wert: roh.to_string_lossy().into_owned(),
    };
    let text = roh.to_str().ok_or_else(fehler)?;
    let nur_ziffern = !text.is_empty() && text.bytes().all(|zeichen| zeichen.is_ascii_digit());
    if !nur_ziffern {
        return Err(fehler());
    }
    let millisekunden: u64 = text.parse().map_err(|_| fehler())?;
    if millisekunden == 0 {
        return Err(fehler());
    }
    Ok(Duration::from_millis(millisekunden))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lies(wert: &str) -> Result<Duration, Herzschlagfehler> {
        aus_umgebung(Some(OsStr::new(wert)))
    }

    #[test]
    fn ohne_wert_alle_5_sekunden() {
        assert_eq!(aus_umgebung(None), Ok(Duration::from_secs(5)));
    }

    #[test]
    fn millisekunden_werden_uebernommen() {
        assert_eq!(lies("500"), Ok(Duration::from_millis(500)));
        assert_eq!(lies("1"), Ok(Duration::from_millis(1)));
    }

    #[test]
    fn alles_andere_ist_ein_fehler_der_die_variable_und_den_wert_nennt() {
        for wert in [
            "bald",
            "",
            "0",
            "-1",
            "500ms",
            " 500",
            "1e3",
            "99999999999999999999999",
        ] {
            match lies(wert) {
                Err(fehler) => {
                    let text = fehler.to_string();
                    assert!(text.contains("LOCALCHATA_HERZSCHLAG_MS"), "{text}");
                    assert!(text.contains(wert), "{text}");
                }
                Ok(dauer) => panic!("{wert:?} ergab {dauer:?}"),
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn ein_wert_der_kein_utf8_ist_ist_ein_fehler() {
        use std::os::unix::ffi::OsStrExt;
        assert!(aus_umgebung(Some(OsStr::from_bytes(b"5\xff00"))).is_err());
    }
}
