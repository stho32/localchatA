//! Aufruf lesen: Argumente → Startauftrag, Hilfewunsch oder Aufruffehler. Rein, ohne Platte und Netz.

use std::fmt;
use std::path::PathBuf;

pub const AUFRUF: &str =
    "Aufruf: localchatA [<ordner>] [--port <n>] [--host <adresse>] [--kein-browser] [--hilfe]";

/// Welcher Port verlangt ist: der Standard mit Ausweichen, die Wahl des Systems oder genau einer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Portwunsch {
    Standard,
    System,
    Fest(u16),
}

/// Was der Aufruf verlangt: welcher Ordner, wo gelauscht wird, ob ein Browser aufgeht.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Startauftrag {
    pub ordner: PathBuf,
    pub port: Portwunsch,
    pub host: String,
    pub browser_oeffnen: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Aufruf {
    Starten(Startauftrag),
    Hilfe,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Aufruffehler {
    UnbekannteOption(String),
    UeberzaehligesArgument(String),
    WertFehlt(String),
    PortUngueltig(String),
}

impl fmt::Display for Aufruffehler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Aufruffehler::UnbekannteOption(option) => write!(f, "Unbekannte Option {option}"),
            Aufruffehler::UeberzaehligesArgument(argument) => {
                write!(
                    f,
                    "Überzähliges Argument {argument} — es gibt nur einen Ordner"
                )
            }
            Aufruffehler::WertFehlt(option) => write!(f, "{option} braucht einen Wert"),
            Aufruffehler::PortUngueltig(wert) => write!(f, "Kein gültiger Port: {wert}"),
        }?;
        write!(f, " — Aufruf siehe localchatA --hilfe")
    }
}

impl Startauftrag {
    /// Liest die Argumente ohne den Programmnamen. Ohne Ordner gilt das aktuelle Verzeichnis.
    pub fn aus_argumenten<I: IntoIterator<Item = String>>(args: I) -> Result<Aufruf, Aufruffehler> {
        let mut auftrag = Startauftrag {
            ordner: PathBuf::from("."),
            port: Portwunsch::Standard,
            host: "0.0.0.0".into(),
            browser_oeffnen: true,
        };
        let mut ordner_gelesen = false;
        let mut args = args.into_iter();
        while let Some(argument) = args.next() {
            match argument.as_str() {
                "--hilfe" => return Ok(Aufruf::Hilfe),
                "--kein-browser" => auftrag.browser_oeffnen = false,
                "--host" => auftrag.host = args.next().ok_or(Aufruffehler::WertFehlt(argument))?,
                "--port" => {
                    let wert = args.next().ok_or(Aufruffehler::WertFehlt(argument))?;
                    auftrag.port = portwunsch(wert)?;
                }
                _ if argument.starts_with('-') => {
                    return Err(Aufruffehler::UnbekannteOption(argument));
                }
                _ if ordner_gelesen => return Err(Aufruffehler::UeberzaehligesArgument(argument)),
                _ => {
                    auftrag.ordner = PathBuf::from(argument);
                    ordner_gelesen = true;
                }
            }
        }
        Ok(Aufruf::Starten(auftrag))
    }
}

fn portwunsch(wert: String) -> Result<Portwunsch, Aufruffehler> {
    match wert.parse::<u16>() {
        Ok(0) => Ok(Portwunsch::System),
        Ok(port) => Ok(Portwunsch::Fest(port)),
        Err(_) => Err(Aufruffehler::PortUngueltig(wert)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lies(argumente: &[&str]) -> Result<Aufruf, Aufruffehler> {
        Startauftrag::aus_argumenten(argumente.iter().map(|a| a.to_string()))
    }

    fn auftrag(argumente: &[&str]) -> Startauftrag {
        match lies(argumente) {
            Ok(Aufruf::Starten(auftrag)) => auftrag,
            anderes => panic!("Startauftrag erwartet, erhalten {anderes:?}"),
        }
    }

    #[test]
    fn ordner_port_null_und_kein_browser_ergeben_den_startauftrag() {
        assert_eq!(
            auftrag(&["/tmp/x", "--port", "0", "--kein-browser"]),
            Startauftrag {
                ordner: PathBuf::from("/tmp/x"),
                port: Portwunsch::System,
                host: "0.0.0.0".into(),
                browser_oeffnen: false,
            }
        );
    }

    #[test]
    fn ohne_argumente_gelten_aktuelles_verzeichnis_standardport_alle_schnittstellen_und_browser() {
        assert_eq!(
            auftrag(&[]),
            Startauftrag {
                ordner: PathBuf::from("."),
                port: Portwunsch::Standard,
                host: "0.0.0.0".into(),
                browser_oeffnen: true,
            }
        );
    }

    #[test]
    fn ausdruecklicher_port_und_host_werden_uebernommen() {
        let gelesen = auftrag(&["--host", "127.0.0.1", "ordner", "--port", "8123"]);
        assert_eq!(gelesen.port, Portwunsch::Fest(8123));
        assert_eq!(gelesen.host, "127.0.0.1");
        assert_eq!(gelesen.ordner, PathBuf::from("ordner"));
        assert!(gelesen.browser_oeffnen);
    }

    #[test]
    fn hilfe_wird_erkannt_auch_neben_anderen_argumenten() {
        assert_eq!(lies(&["--hilfe"]), Ok(Aufruf::Hilfe));
        assert_eq!(lies(&["/tmp/x", "--hilfe"]), Ok(Aufruf::Hilfe));
    }

    #[test]
    fn unbekannte_option_und_zweiter_ordner_sind_aufruffehler() {
        assert_eq!(
            lies(&["--gibtsnicht"]),
            Err(Aufruffehler::UnbekannteOption("--gibtsnicht".into()))
        );
        assert_eq!(
            lies(&["a", "b"]),
            Err(Aufruffehler::UeberzaehligesArgument("b".into()))
        );
    }

    #[test]
    fn fehlender_oder_ungueltiger_portwert_ist_ein_aufruffehler() {
        assert_eq!(
            lies(&["--port"]),
            Err(Aufruffehler::WertFehlt("--port".into()))
        );
        assert_eq!(
            lies(&["--host"]),
            Err(Aufruffehler::WertFehlt("--host".into()))
        );
        assert_eq!(
            lies(&["--port", "70000"]),
            Err(Aufruffehler::PortUngueltig("70000".into()))
        );
        assert_eq!(
            lies(&["--port", "x"]),
            Err(Aufruffehler::PortUngueltig("x".into()))
        );
    }

    #[test]
    fn jede_fehlermeldung_ist_eine_zeile_nennt_das_argument_und_verweist_auf_die_hilfe() {
        for (fehler, nennt) in [
            (
                Aufruffehler::UnbekannteOption("--gibtsnicht".into()),
                "--gibtsnicht",
            ),
            (Aufruffehler::UeberzaehligesArgument("b".into()), "b"),
            (Aufruffehler::WertFehlt("--port".into()), "--port"),
            (Aufruffehler::PortUngueltig("x".into()), "x"),
        ] {
            let text = fehler.to_string();
            assert!(text.contains(nennt), "{text}");
            assert!(text.contains("localchatA --hilfe"), "{text}");
            assert!(!text.contains('\n'), "{text}");
        }
    }
}
