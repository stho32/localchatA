//! Gnadenfrist aus der Umgebung (B0050): Standard 60 s, Testknopf `LOCALCHATA_GNADENFRIST_MS` (ADR-0009).

use std::ffi::OsStr;
use std::fmt;
use std::time::Duration;

pub const VARIABLE: &str = "LOCALCHATA_GNADENFRIST_MS";
const STANDARD: Duration = Duration::from_secs(60);

/// `LOCALCHATA_GNADENFRIST_MS` ist gesetzt, aber keine ganze Zahl von Millisekunden.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gnadenfristfehler {
    pub wert: String,
}

impl fmt::Display for Gnadenfristfehler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{VARIABLE} ist keine ganze Zahl von Millisekunden: {} — z. B. 3000",
            self.wert
        )
    }
}

/// Ohne Wert der Standard; ein Wert, der kein UTF-8 oder keine Zahl ist, ist ein Fehler — nie still der Standard.
pub fn aus_umgebung(wert: Option<&OsStr>) -> Result<Duration, Gnadenfristfehler> {
    let Some(roh) = wert else {
        return Ok(STANDARD);
    };
    let fehler = || Gnadenfristfehler {
        wert: roh.to_string_lossy().into_owned(),
    };
    let text = roh.to_str().ok_or_else(fehler)?;
    let nur_ziffern = !text.is_empty() && text.bytes().all(|zeichen| zeichen.is_ascii_digit());
    if !nur_ziffern {
        return Err(fehler());
    }
    let millisekunden: u64 = text.parse().map_err(|_| fehler())?;
    Ok(Duration::from_millis(millisekunden))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lies(wert: &str) -> Result<Duration, Gnadenfristfehler> {
        aus_umgebung(Some(OsStr::new(wert)))
    }

    #[test]
    fn ohne_wert_gilt_der_standard_von_60_sekunden() {
        assert_eq!(aus_umgebung(None), Ok(Duration::from_secs(60)));
    }

    #[test]
    fn millisekunden_werden_uebernommen() {
        assert_eq!(lies("3000"), Ok(Duration::from_millis(3000)));
        assert_eq!(lies("2500"), Ok(Duration::from_millis(2500)));
        assert_eq!(lies("0"), Ok(Duration::ZERO));
    }

    #[test]
    fn alles_andere_ist_ein_fehler_der_die_variable_und_den_wert_nennt() {
        for wert in [
            "bald",
            "",
            "-1",
            "3000ms",
            "3,5",
            " 3000",
            "1e3",
            "99999999999999999999999",
        ] {
            match lies(wert) {
                Err(fehler) => {
                    let text = fehler.to_string();
                    assert!(text.contains("LOCALCHATA_GNADENFRIST_MS"), "{text}");
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
        let roh = OsStr::from_bytes(b"30\xff00");
        assert!(aus_umgebung(Some(roh)).is_err());
    }
}
