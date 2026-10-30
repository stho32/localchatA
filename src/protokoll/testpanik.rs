//! Testknopf `LOCALCHATA_TESTPANIK=verlauf|takt|loeschen` (ADR-0042 Regel 4 mit Ergänzung, ADR-0048 K3): nur im
//! Debug-Build gelesen; `verlauf` lässt den nächsten Verlaufsblock einmal in Panik geraten, `takt` den
//! Anwesenheits-Takt, `loeschen` den nächsten Blocking-Teil eines Löschens (Nachricht, Laufwerk) oder Verwerfens. Im
//! Release-Build wird die Variable nie gelesen und nichts gerät in Panik.

use std::ffi::OsStr;
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub const VARIABLE: &str = "LOCALCHATA_TESTPANIK";

/// Wo eine Panik ausgelöst werden soll; je Ort genau einmal.
#[derive(Debug, Clone, Default)]
pub struct Testpanik {
    verlauf: Arc<AtomicBool>,
    takt: Arc<AtomicBool>,
    loeschen: Arc<AtomicBool>,
}

/// Ein Wert außer `verlauf`, `takt` und `loeschen` (auch leer).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Testpanikfehler {
    pub wert: String,
}

impl fmt::Display for Testpanikfehler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{VARIABLE} kennt nur verlauf, takt und loeschen: {}",
            super::in_anfuehrung(&self.wert)
        )
    }
}

impl Testpanik {
    /// Keine Panik.
    pub fn keine() -> Testpanik {
        Testpanik::default()
    }

    /// Aus dem Wert der Variablen (nur im Debug-Build aufgerufen).
    pub fn aus_wert(wert: Option<&OsStr>) -> Result<Testpanik, Testpanikfehler> {
        let testpanik = Testpanik::keine();
        match wert.map(|roh| roh.to_str().ok_or(roh)) {
            None => {}
            Some(Ok("verlauf")) => testpanik.verlauf.store(true, Ordering::SeqCst),
            Some(Ok("takt")) => testpanik.takt.store(true, Ordering::SeqCst),
            Some(Ok("loeschen")) => testpanik.loeschen.store(true, Ordering::SeqCst),
            Some(Ok(anderer)) => {
                return Err(Testpanikfehler {
                    wert: anderer.to_string(),
                });
            }
            Some(Err(roh)) => {
                return Err(Testpanikfehler {
                    wert: roh.to_string_lossy().into_owned(),
                });
            }
        }
        Ok(testpanik)
    }

    /// Im Debug-Build aus der Umgebung, im Release-Build immer keine — die Variable wird dort nicht gelesen.
    pub fn aus_umgebung() -> Result<Testpanik, Testpanikfehler> {
        if cfg!(debug_assertions) {
            Testpanik::aus_wert(std::env::var_os(VARIABLE).as_deref())
        } else {
            Ok(Testpanik::keine())
        }
    }

    /// Gerät einmal in Panik, wenn `verlauf` gewählt ist.
    pub fn verlauf(&self) {
        if self.verlauf.swap(false, Ordering::SeqCst) {
            panic!("Testpanik im Verlauf ({VARIABLE}=verlauf)");
        }
    }

    /// Gerät einmal in Panik, wenn `loeschen` gewählt ist.
    pub fn loeschen(&self) {
        if self.loeschen.swap(false, Ordering::SeqCst) {
            panic!("Testpanik beim Löschen ({VARIABLE}=loeschen)");
        }
    }

    /// Gerät einmal in Panik, wenn `takt` gewählt ist.
    pub fn takt(&self) {
        if self.takt.swap(false, Ordering::SeqCst) {
            panic!("Testpanik im Takt ({VARIABLE}=takt)");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// R00018 F0062: der Wert steht in „…“, Steuerzeichen escaped wie im Protokoll.
    #[test]
    fn die_meldung_nennt_den_wert_in_anfuehrung_mit_escape() {
        let meldung = Testpanikfehler {
            wert: "la\u{1b}ut".to_string(),
        }
        .to_string();

        assert!(meldung.contains("„la\\u{1b}ut“"), "{meldung}");
        assert!(!meldung.contains('\u{1b}'), "{meldung}");
        assert!(!meldung.contains('"'), "{meldung}");
    }

    #[test]
    fn verlauf_geraet_genau_einmal_in_panik() {
        let testpanik = Testpanik::aus_wert(Some(OsStr::new("verlauf"))).expect("gültig");
        assert!(std::panic::catch_unwind(|| testpanik.verlauf()).is_err());
        assert!(std::panic::catch_unwind(|| testpanik.verlauf()).is_ok());
        assert!(std::panic::catch_unwind(|| testpanik.takt()).is_ok());
    }

    /// R00043 B0667 (ADR-0048 K3): `loeschen` gerät genau einmal in Panik und berührt die anderen Orte nicht.
    #[test]
    fn loeschen_geraet_genau_einmal_in_panik() {
        let testpanik = Testpanik::aus_wert(Some(OsStr::new("loeschen"))).expect("gültig");
        assert!(std::panic::catch_unwind(|| testpanik.verlauf()).is_ok());
        assert!(std::panic::catch_unwind(|| testpanik.loeschen()).is_err());
        assert!(std::panic::catch_unwind(|| testpanik.loeschen()).is_ok());
    }

    #[test]
    fn ohne_wert_keine_panik_andere_werte_sind_fehler() {
        let keine = Testpanik::aus_wert(None).expect("ohne Wert");
        assert!(std::panic::catch_unwind(|| keine.verlauf()).is_ok());
        for falsch in ["laut", "", "Verlauf"] {
            let fehler = Testpanik::aus_wert(Some(OsStr::new(falsch))).expect_err(falsch);
            assert!(fehler.to_string().contains(VARIABLE), "{fehler}");
        }
    }
}
