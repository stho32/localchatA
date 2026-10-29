//! Wirksamer Schalter `--alle-namen` (ADR-0045 Regel 9, ADR-0043 Ergänzung Karte #26): steht die Option im
//! Aufruf, ist er an, und `LOCALCHATA_ALLE_NAMEN` wird nicht gelesen; sonst `1` an, `0` oder leer aus, nicht
//! gesetzt aus, jeder andere Wert ein Fehler (Exit 2). Rein bis auf das Lesen der Umgebung über `lesen`.

use std::ffi::OsString;
use std::fmt;

pub const VARIABLE: &str = "LOCALCHATA_ALLE_NAMEN";

/// Der Satz, mit dem die Instanz beim Start warnt und den `--hilfe` nennt.
pub const RISIKO: &str = "--alle-namen ist aktiv: jeder Host-Name wird angenommen — eine fremde Website, die ein Teilnehmer öffnet, kann über DNS-Rebinding Channels mitlesen, Nachrichten schreiben und über die Frist Verläufe löschen.";

/// `LOCALCHATA_ALLE_NAMEN` ist gesetzt, aber weder `1`, `0` noch leer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlleNamenFehler {
    pub wert: String,
}

impl fmt::Display for AlleNamenFehler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{VARIABLE} kennt nur 1 (an), 0 oder leer (aus): {:?}",
            self.wert
        )
    }
}

/// An oder aus; `lesen` liefert den Wert der Umgebung und wird nur ohne Option gerufen.
pub fn wirksam(
    option: bool,
    lesen: impl FnOnce() -> Option<OsString>,
) -> Result<bool, AlleNamenFehler> {
    if option {
        return Ok(true);
    }
    match lesen() {
        None => Ok(false),
        Some(wert) => match wert.to_str() {
            Some("1") => Ok(true),
            Some("0" | "") => Ok(false),
            _ => Err(AlleNamenFehler {
                wert: wert.to_string_lossy().into_owned(),
            }),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn umgebung(wert: &str) -> impl FnOnce() -> Option<OsString> {
        let wert = OsString::from(wert);
        move || Some(wert)
    }

    #[test]
    fn umgebung_eins_ist_an_null_leer_und_ohne_sind_aus() {
        assert_eq!(wirksam(false, umgebung("1")), Ok(true));
        assert_eq!(wirksam(false, umgebung("0")), Ok(false));
        assert_eq!(wirksam(false, umgebung("")), Ok(false));
        assert_eq!(wirksam(false, || None), Ok(false));
    }

    /// Matrix I12, US-11: ohne Option ist jeder andere Wert ein Fehler, der die Variable nennt.
    #[test]
    fn ohne_option_ist_ein_falscher_wert_ein_fehler() {
        for falsch in ["ja", "true", "2", " 1", "1 "] {
            let fehler = wirksam(false, umgebung(falsch)).expect_err(falsch);
            assert_eq!(fehler.wert, falsch);
            assert!(fehler.to_string().contains(VARIABLE), "{fehler}");
        }
    }

    /// Matrix I13, I14 (E7), US-12: die Option gilt, die Umgebung wird nicht gelesen.
    #[test]
    fn die_option_gilt_und_die_umgebung_wird_nicht_gelesen() {
        assert_eq!(wirksam(true, umgebung("0")), Ok(true));
        assert_eq!(wirksam(true, umgebung("ja")), Ok(true));
        assert_eq!(
            wirksam(true, || panic!("Umgebung gelesen trotz Option")),
            Ok(true)
        );
    }
}
