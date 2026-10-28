//! Testknopf `LOCALCHATA_EPISODE_RUHE_MS` (ADR-0042, Ergänzung 5): Ruhe bis zum Ende einer Episode in
//! Millisekunden, Standard 60.000; 0, leer oder keine Zahl ist ein Fehler (Exit 2).

use super::episoden::RUHE;
use std::ffi::OsStr;
use std::fmt;
use std::time::Duration;

pub const VARIABLE: &str = "LOCALCHATA_EPISODE_RUHE_MS";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ruhefehler {
    pub wert: String,
}

impl fmt::Display for Ruhefehler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{VARIABLE} ist keine ganze Zahl von Millisekunden ab 1: {:?} — z. B. 1000",
            self.wert
        )
    }
}

pub fn aus_umgebung(wert: Option<&OsStr>) -> Result<Duration, Ruhefehler> {
    let Some(roh) = wert else {
        return Ok(RUHE);
    };
    let fehler = || Ruhefehler {
        wert: roh.to_string_lossy().into_owned(),
    };
    let text = roh.to_str().ok_or_else(fehler)?;
    if text.is_empty() || !text.bytes().all(|zeichen| zeichen.is_ascii_digit()) {
        return Err(fehler());
    }
    match text.parse::<u64>() {
        Ok(0) | Err(_) => Err(fehler()),
        Ok(millisekunden) => Ok(Duration::from_millis(millisekunden)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ohne_wert_60_s_sonst_die_millisekunden() {
        assert_eq!(aus_umgebung(None), Ok(Duration::from_secs(60)));
        assert_eq!(
            aus_umgebung(Some(OsStr::new("1000"))),
            Ok(Duration::from_millis(1000))
        );
    }

    #[test]
    fn null_leer_und_keine_zahl_sind_fehler_die_die_variable_nennen() {
        for falsch in ["0", "", "-1", "1s", " 5"] {
            let fehler = aus_umgebung(Some(OsStr::new(falsch))).expect_err(falsch);
            assert!(fehler.to_string().contains(VARIABLE), "{fehler}");
        }
    }
}
