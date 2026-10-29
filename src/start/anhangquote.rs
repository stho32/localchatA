//! Anhang-Quote einstellen (B0500; ADR-0043 Ergänzungen „Karte #9 Betreiber F1“ und „F1′“): `--anhangquote` vor
//! `LOCALCHATA_ANHANGQUOTE` vor dem Standard 20 GiB; eine ganze Zahl mit `MiB` oder `GiB` (Groß-/Kleinschreibung
//! egal, ohne Leerzeichen) zwischen 1 MiB und 16.384 GiB, oder `aus`. Rein.

use crate::gespraeche::STANDARD_ANHANGQUOTE;
use std::ffi::OsStr;
use std::fmt;

pub const VARIABLE: &str = "LOCALCHATA_ANHANGQUOTE";
pub const OPTION: &str = "--anhangquote";

const MIB: u64 = 1024 * 1024;
const GIB: u64 = 1024 * MIB;
/// Obergrenze: fängt Tippfehler wie `20000GiB`, lässt Platten mit vielen Terabyte zu (die Text-Quote bleibt bei
/// 1.024 GiB, weil sie zugleich den Speicher begrenzt).
const HOECHSTENS: u64 = 16_384 * GIB;

pub const STANDARD: Anhangquote = Anhangquote::Bytes(STANDARD_ANHANGQUOTE);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anhangquote {
    Bytes(u64),
    Aus,
}

impl Anhangquote {
    pub fn hoechstens(self) -> Option<u64> {
        match self {
            Anhangquote::Bytes(bytes) => Some(bytes),
            Anhangquote::Aus => None,
        }
    }
}

/// Der Wert aus Option oder Umgebung ist keine Größe nach der Regel; die Zeile nennt Quelle, Wert und Spanne.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Groessenfehler {
    pub quelle: &'static str,
    pub wert: String,
}

impl fmt::Display for Groessenfehler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} braucht eine Größe von 1 MiB bis 16.384 GiB wie 20GiB oder 512MiB, oder aus — nicht „{}“",
            self.quelle, self.wert
        )
    }
}

/// `20GiB`, `512mib`, `aus` → Quote; alles andere (ohne Einheit, `MB`/`GB`, null, außerhalb 1 MiB bis
/// 16.384 GiB, Vorzeichen, Leerzeichen, leer) → kein Wert.
pub fn lesen(text: &str) -> Option<Anhangquote> {
    let klein = text.to_ascii_lowercase();
    if klein == "aus" {
        return Some(Anhangquote::Aus);
    }
    let (zahl, einheit) = if let Some(zahl) = klein.strip_suffix("mib") {
        (zahl, MIB)
    } else if let Some(zahl) = klein.strip_suffix("gib") {
        (zahl, GIB)
    } else {
        return None;
    };
    if zahl.is_empty() || !zahl.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let bytes = zahl.parse::<u64>().ok()?.checked_mul(einheit)?;
    (MIB..=HOECHSTENS)
        .contains(&bytes)
        .then_some(Anhangquote::Bytes(bytes))
}

/// Quote und Herkunft (`--anhangquote` | `LOCALCHATA_ANHANGQUOTE` | `standard`). Eine gültige Option übersteuert:
/// dann wird die Umgebung nicht gelesen, auch nicht geprüft (ADR-0043 Ergänzung Karte #26).
pub fn wirksame_anhangquote(
    option: Option<Anhangquote>,
    umgebung: Option<&OsStr>,
) -> Result<(Anhangquote, &'static str), Groessenfehler> {
    if let Some(quote) = option {
        return Ok((quote, OPTION));
    }
    let Some(roh) = umgebung else {
        return Ok((STANDARD, "standard"));
    };
    let fehler = || Groessenfehler {
        quelle: VARIABLE,
        wert: roh.to_string_lossy().into_owned(),
    };
    let quote = roh.to_str().and_then(lesen).ok_or_else(fehler)?;
    Ok((quote, VARIABLE))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// R00029 „Anhang-Quote einstellen“: `512MiB`, `1gib`, `aus`, die Ränder `1MiB` und `16384GiB`.
    #[test]
    fn gueltige_groessen_bis_16384_gib() {
        assert_eq!(lesen("512MiB"), Some(Anhangquote::Bytes(536_870_912)));
        assert_eq!(lesen("1gib"), Some(Anhangquote::Bytes(1_073_741_824)));
        assert_eq!(lesen("aus"), Some(Anhangquote::Aus));
        assert_eq!(lesen("AUS"), Some(Anhangquote::Aus));
        assert_eq!(lesen("1MiB"), Some(Anhangquote::Bytes(1_048_576)));
        assert_eq!(
            lesen("16384GiB"),
            Some(Anhangquote::Bytes(17_592_186_044_416))
        );
        assert_eq!(
            lesen("1025GiB"),
            Some(Anhangquote::Bytes(1_100_585_369_600))
        );
    }

    /// `0MiB`, `20000`, `20 GiB`, `16385GiB`, `512KiB`, leer → kein Wert (Mutante „Spanne der Text-Quote“: 1025GiB
    /// oben ist gültig, 16385GiB nicht).
    #[test]
    fn falsche_groessen_sind_kein_wert() {
        for falsch in [
            "0MiB",
            "0GiB",
            "20000",
            "20 GiB",
            "16385GiB",
            "512KiB",
            "",
            "GiB",
            "-1GiB",
            "+1GiB",
            " 1GiB",
            "1,5GiB",
            "20GB",
            "99999999999999999999GiB",
        ] {
            assert_eq!(lesen(falsch), None, "{falsch:?}");
        }
    }

    /// Die Zeile nennt Quelle, Wert und Spanne.
    #[test]
    fn der_fehler_nennt_quelle_wert_und_spanne() {
        let fehler = Groessenfehler {
            quelle: OPTION,
            wert: "20 GiB".into(),
        };
        assert_eq!(
            fehler.to_string(),
            "--anhangquote braucht eine Größe von 1 MiB bis 16.384 GiB wie 20GiB oder 512MiB, oder aus — nicht „20 GiB“"
        );
    }

    /// Option vor Umgebung vor Standard 20 GiB; eine gültige Option übersteuert eine falsche Umgebung (Mutante
    /// „Umgebung schlägt Option“).
    #[test]
    fn option_vor_umgebung_vor_standard() {
        let gib = Anhangquote::Bytes(1_073_741_824);
        assert_eq!(
            wirksame_anhangquote(Some(gib), Some(OsStr::new("kaputt"))),
            Ok((gib, OPTION))
        );
        assert_eq!(
            wirksame_anhangquote(Some(gib), Some(OsStr::new("aus"))),
            Ok((gib, OPTION))
        );
        assert_eq!(
            wirksame_anhangquote(None, Some(OsStr::new("1gib"))),
            Ok((gib, VARIABLE))
        );
        assert_eq!(
            wirksame_anhangquote(None, None),
            Ok((Anhangquote::Bytes(21_474_836_480), "standard"))
        );
        assert_eq!(
            wirksame_anhangquote(None, Some(OsStr::new("0GiB"))),
            Err(Groessenfehler {
                quelle: VARIABLE,
                wert: "0GiB".into()
            })
        );
    }
}
