//! Testknopf `LOCALCHATA_TESTGRENZE_SCHLUESSEL=<n>` (ADR-0042 Ergänzungen Karte #36 und „Karte #36 Angriffsmatrix“,
//! E3): setzt im Debug-Build die Gesamtgrenze des Wiederholspeichers; gültig ist nur `[1-9][0-9]{0,4}` bis 36.000,
//! jeder andere Wert ist ein Fehler (Exit 2, vor der Sperre). Im Release-Build wird die Variable nicht gelesen.

use std::ffi::OsStr;
use std::fmt;

pub const VARIABLE: &str = "LOCALCHATA_TESTGRENZE_SCHLUESSEL";

/// Ein Wert außerhalb der Schreibweise `[1-9][0-9]{0,4}` oder über 36.000 (auch leer).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Testgrenzefehler {
    pub wert: String,
}

impl fmt::Display for Testgrenzefehler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{VARIABLE} erlaubt nur eine ganze Zahl von 1 bis 36.000 ohne Vorzeichen, führende Null oder Leerzeichen: {}",
            crate::protokoll::in_anfuehrung(&self.wert)
        )
    }
}

/// Die Gesamtgrenze aus dem Wert der Variablen: `debug: false` (Release-Build) liest nichts; ohne Variable keine.
pub fn testgrenze(wert: Option<&OsStr>, debug: bool) -> Result<Option<usize>, Testgrenzefehler> {
    if !debug {
        return Ok(None);
    }
    let Some(roh) = wert else {
        return Ok(None);
    };
    let text = roh.to_string_lossy();
    let ziffern = text.as_bytes();
    let schreibweise = (1..=5).contains(&ziffern.len())
        && ziffern[0] != b'0'
        && ziffern.iter().all(u8::is_ascii_digit);
    match text.parse::<usize>() {
        Ok(grenze)
            if schreibweise && grenze <= crate::gespraeche::wiederholung::HOECHSTENS_GESAMT =>
        {
            Ok(Some(grenze))
        }
        _ => Err(Testgrenzefehler {
            wert: text.into_owned(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pruefen(wert: &str, debug: bool) -> Result<Option<usize>, Testgrenzefehler> {
        testgrenze(Some(OsStr::new(wert)), debug)
    }

    /// ADR-0042 Ergänzung „Karte #36 Angriffsmatrix“, erste Tests (wörtlich): „`"1"`, `"5"`, `"36000"` → Grenze;
    /// `"0"`, `"36001"`, `"+5"`, `"05"`, `" 5"`, `"5 "`, `"-1"`, `"x"`, `""` → Fehler; je Wert mit `debug: false` →
    /// nicht gelesen.“
    #[test]
    fn nur_die_schreibweise_bis_36000_gilt_und_im_release_wird_nichts_gelesen() {
        for (wert, grenze) in [("1", 1), ("5", 5), ("36000", 36_000)] {
            assert_eq!(pruefen(wert, true), Ok(Some(grenze)), "{wert:?}");
        }
        for falsch in ["0", "36001", "+5", "05", " 5", "5 ", "-1", "x", ""] {
            let fehler = pruefen(falsch, true).expect_err(falsch);
            let meldung = fehler.to_string();
            assert!(meldung.contains(VARIABLE), "{meldung}");
            assert!(meldung.contains("1 bis 36.000"), "{meldung}");
        }
        for wert in [
            "1", "5", "36000", "0", "36001", "+5", "05", " 5", "5 ", "-1", "x", "",
        ] {
            assert_eq!(pruefen(wert, false), Ok(None), "{wert:?}");
        }
        assert_eq!(testgrenze(None, true), Ok(None));
    }
}
