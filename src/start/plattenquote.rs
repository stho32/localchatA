//! Plattenquote einstellen (ADR-0044 Regel 3, ADR-0043 Ergänzung Karte #34): `--plattenquote` vor
//! `LOCALCHATA_PLATTENQUOTE` vor dem Standard 4 GiB; eine ganze Zahl mit `MiB` oder `GiB` zwischen 1 MiB und
//! 1.024 GiB, oder `aus`. Rein.

use std::ffi::OsStr;
use std::fmt;

pub const VARIABLE: &str = "LOCALCHATA_PLATTENQUOTE";

const MIB: u64 = 1024 * 1024;
const GIB: u64 = 1024 * MIB;

/// Standard 4 GiB (ADR-0044 Regel 3): Normalwoche ≈ 1,6 GB und Bestand des Lastprofils ≈ 2,65 GB liegen unter
/// der Warnschwelle von 3,6 GiB; eine flutende Herkunft braucht vom Normalstand ≈ 11 h bis hierher. Gerechnet,
/// nicht gemessen — anheben ist ein Commit.
pub const STANDARD: Plattenquote = Plattenquote::Bytes(4 * GIB);

/// Die wirksame Quote: höchstens so viele Bytes für alle Gespräche, oder keine Grenze.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Plattenquote {
    Bytes(u64),
    Aus,
}

impl Plattenquote {
    /// Die Grenze in Bytes, `None` bei `aus`.
    pub fn hoechstens(self) -> Option<u64> {
        match self {
            Plattenquote::Bytes(bytes) => Some(bytes),
            Plattenquote::Aus => None,
        }
    }

    /// Wie der Start-Eintrag sie nennt: die Bytes oder `aus`.
    pub fn als_wert(self) -> String {
        match self {
            Plattenquote::Bytes(bytes) => bytes.to_string(),
            Plattenquote::Aus => "aus".to_string(),
        }
    }
}

/// Der Text ist keine Größe nach der Regel; die Meldung nennt Beispiele (ADR-0044 Regel 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Groessenfehler;

impl fmt::Display for Groessenfehler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "--plattenquote braucht eine Größe wie 4GiB oder 512MiB, oder aus"
        )
    }
}

/// `4GiB`, `512mib`, `aus` → Quote; alles andere (ohne Einheit, `MB`/`GB`, null, außerhalb 1 MiB bis 1.024 GiB,
/// Vorzeichen, Leerzeichen, leer) → `Groessenfehler`.
pub fn lesen(text: &str) -> Result<Plattenquote, Groessenfehler> {
    let klein = text.to_ascii_lowercase();
    if klein == "aus" {
        return Ok(Plattenquote::Aus);
    }
    let (zahl, einheit) = if let Some(zahl) = klein.strip_suffix("mib") {
        (zahl, MIB)
    } else if let Some(zahl) = klein.strip_suffix("gib") {
        (zahl, GIB)
    } else {
        return Err(Groessenfehler);
    };
    if zahl.is_empty() || !zahl.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(Groessenfehler);
    }
    let bytes = zahl
        .parse::<u64>()
        .ok()
        .and_then(|zahl| zahl.checked_mul(einheit))
        .ok_or(Groessenfehler)?;
    if (MIB..=1024 * GIB).contains(&bytes) {
        Ok(Plattenquote::Bytes(bytes))
    } else {
        Err(Groessenfehler)
    }
}

/// Quote und Herkunft (`--plattenquote` | `LOCALCHATA_PLATTENQUOTE` | `standard`). Eine gültige Option übersteuert:
/// dann wird die Umgebung nicht gelesen, auch nicht geprüft (ADR-0044 Ergänzung Karte #26).
pub fn wirksame_quote(
    option: Option<Plattenquote>,
    umgebung: Option<&OsStr>,
) -> Result<(Plattenquote, &'static str), Groessenfehler> {
    if let Some(quote) = option {
        return Ok((quote, "--plattenquote"));
    }
    let Some(roh) = umgebung else {
        return Ok((STANDARD, "standard"));
    };
    let quote = roh.to_str().ok_or(Groessenfehler).and_then(lesen)?;
    Ok((quote, VARIABLE))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Abnahme-Probe B0352 (ADR-0044 „Folgeentscheidungen“, Unit Aufruf, wörtlich): „`--plattenquote 4GiB` →
    /// 4.294.967.296; `512mib` → 536.870.912; `aus` → keine Quote; `1MiB` → 1.048.576“; dazu die Obergrenze.
    #[test]
    fn gueltige_groessen_ergeben_bytes_und_aus_keine_quote() {
        assert_eq!(lesen("4GiB"), Ok(Plattenquote::Bytes(4_294_967_296)));
        assert_eq!(lesen("512mib"), Ok(Plattenquote::Bytes(536_870_912)));
        assert_eq!(lesen("aus"), Ok(Plattenquote::Aus));
        assert_eq!(lesen("1MiB"), Ok(Plattenquote::Bytes(1_048_576)));
        assert_eq!(lesen("1024GiB"), Ok(Plattenquote::Bytes(1_099_511_627_776)));
        assert_eq!(
            lesen("1048576MIB"),
            Ok(Plattenquote::Bytes(1_099_511_627_776))
        );
    }

    /// „`0MiB`, `0`, `4`, `4GB`, `1025GiB`, `-1GiB`, leer → Aufruffehler mit „4GiB oder 512MiB, oder aus““ — dazu
    /// die Mutanten „`GB` wie `GiB`“, „Spanne ohne Obergrenze“, „`0` heißt aus“ und Schreibweisen am Rand.
    #[test]
    fn falsche_groessen_sind_ein_fehler_mit_beispiel() {
        for falsch in [
            "0MiB",
            "0",
            "4",
            "4GB",
            "4MB",
            "1025GiB",
            "1048577MiB",
            "-1GiB",
            "+1GiB",
            "",
            "GiB",
            "4 GiB",
            " 4GiB",
            "4,5GiB",
            "4.5GiB",
            "Aus ",
            "99999999999999999999GiB",
            "0GiB",
        ] {
            assert_eq!(lesen(falsch), Err(Groessenfehler), "{falsch:?}");
        }
        assert!(
            Groessenfehler
                .to_string()
                .contains("4GiB oder 512MiB, oder aus")
        );
    }

    /// „Option schlägt Umgebung; ohne beides → 4 GiB“ mit Herkunft (ADR-0044 Ergänzung Karte #26: die Umgebung
    /// wird neben einer gültigen Option nicht gelesen, auch nicht, wenn sie falsch ist).
    #[test]
    fn option_vor_umgebung_vor_standard() {
        let mib_1 = Plattenquote::Bytes(1_048_576);
        assert_eq!(
            wirksame_quote(Some(mib_1), Some(OsStr::new("4"))),
            Ok((mib_1, "--plattenquote"))
        );
        assert_eq!(
            wirksame_quote(Some(mib_1), Some(OsStr::new("aus"))),
            Ok((mib_1, "--plattenquote"))
        );
        assert_eq!(
            wirksame_quote(None, Some(OsStr::new("aus"))),
            Ok((Plattenquote::Aus, VARIABLE))
        );
        assert_eq!(
            wirksame_quote(None, Some(OsStr::new("512MiB"))),
            Ok((Plattenquote::Bytes(536_870_912), VARIABLE))
        );
        assert_eq!(
            wirksame_quote(None, None),
            Ok((Plattenquote::Bytes(4_294_967_296), "standard"))
        );
    }

    #[test]
    fn ohne_option_ist_eine_falsche_umgebung_ein_fehler() {
        for falsch in ["4", "4GB", ""] {
            assert_eq!(
                wirksame_quote(None, Some(OsStr::new(falsch))),
                Err(Groessenfehler),
                "{falsch:?}"
            );
        }
    }

    #[test]
    fn der_start_eintrag_nennt_bytes_oder_aus() {
        assert_eq!(Plattenquote::Bytes(536_870_912).als_wert(), "536870912");
        assert_eq!(Plattenquote::Aus.als_wert(), "aus");
        assert_eq!(Plattenquote::Aus.hoechstens(), None);
        assert_eq!(STANDARD.hoechstens(), Some(4_294_967_296));
    }
}
