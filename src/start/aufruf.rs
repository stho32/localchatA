//! Aufruf lesen: Argumente → Startauftrag, Hilfewunsch oder Aufruffehler. Rein, ohne Platte und Netz.

use crate::protokoll::Stufe;
use std::fmt;
use std::path::PathBuf;

/// Aufrufzeile mit allen Optionen, darunter je Option eine Erklärung.
pub const HILFE: &str = concat!(
    "Aufruf: localchatA [<ordner>] [--port <n>] [--host <adresse>] [--kein-browser] [--erreichbar-als <name>]… [--protokoll-stufe <stufe>] [--alle-namen] [--hilfe]\n",
    "\n",
    "  <ordner>          Ordner der Instanz, wird angelegt (Standard: aktuelles Verzeichnis)\n",
    "  --port <n>        Port; ohne Angabe 7430 oder der nächste freie, 0 wählt das System\n",
    "  --host <adresse>  Adresse zum Lauschen (Standard: 0.0.0.0, alle Schnittstellen im LAN)\n",
    "  --kein-browser    nach dem Start keinen Browser öffnen\n",
    "  --erreichbar-als <name>\n",
    "                    Host-Name, unter dem die Instanz zusätzlich antwortet (mehrfach erlaubt);\n",
    "                    IP-Adressen, localhost und der Rechnername gelten immer; Namen des eigenen Netzes\n",
    "                    (Rückwärtsname der Adresse, .local, .home.arpa, .internal, einteilig) gelten automatisch\n",
    "  --protokoll-stufe <stufe>  Mindeststufe des Betriebsprotokolls unter <ordner>/logs/: debug, information (Standard), warning, error;\n",
    "                    auch über LOCALCHATA_PROTOKOLL_STUFE\n",
    "  --alle-namen      jeden Host-Namen annehmen, ohne Namensauflösung; auch über LOCALCHATA_ALLE_NAMEN=1\n",
    "                    Risiko: eine fremde Website, die ein Teilnehmer öffnet, kann über DNS-Rebinding Channels mitlesen, Nachrichten schreiben und über die Frist Verläufe löschen.\n",
    "  --hilfe           diese Hilfe zeigen\n",
    "\n",
    "Sendefrist: Das System schließt eine Verbindung, die 2 × Herzschlag (Standard: 10 s) lang nichts\n",
    "abnimmt oder bestätigt.",
);

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
    /// Weitere Host-Namen, unter denen die Instanz antworten darf (Grenzschutz, ADR-0014).
    pub erreichbar_als: Vec<String>,
    /// Mindeststufe des Betriebsprotokolls aus `--protokoll-stufe`, falls angegeben (ADR-0043).
    pub protokoll_stufe: Option<Stufe>,
    /// `--alle-namen` steht im Aufruf (ADR-0045 Regel 9).
    pub alle_namen: bool,
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
    StufeUngueltig(String),
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
            Aufruffehler::StufeUngueltig(wert) => write!(
                f,
                "Keine Protokollstufe: {wert} — erlaubt sind {ERLAUBTE_STUFEN}"
            ),
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
            erreichbar_als: Vec::new(),
            protokoll_stufe: None,
            alle_namen: false,
        };
        let mut ordner_gelesen = false;
        let mut args = args.into_iter();
        while let Some(argument) = args.next() {
            match argument.as_str() {
                "--hilfe" => return Ok(Aufruf::Hilfe),
                "--kein-browser" => auftrag.browser_oeffnen = false,
                "--alle-namen" => auftrag.alle_namen = true,
                "--host" => auftrag.host = args.next().ok_or(Aufruffehler::WertFehlt(argument))?,
                "--erreichbar-als" => {
                    let name = args.next().ok_or(Aufruffehler::WertFehlt(argument))?;
                    auftrag.erreichbar_als.push(name);
                }
                "--protokoll-stufe" => {
                    let wert = args.next().ok_or(Aufruffehler::WertFehlt(argument))?;
                    let stufe = stufe_lesen(&wert).ok_or(Aufruffehler::StufeUngueltig(wert))?;
                    auftrag.protokoll_stufe = Some(stufe);
                }
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

/// Die wählbaren Stufen in der Form, in der Meldungen sie nennen.
pub const ERLAUBTE_STUFEN: &str = "debug, information, warning, error";

/// `debug`, `information`, `warning`, `error` in beliebiger Groß- und Kleinschreibung; `exception` ist nicht
/// wählbar (ADR-0043 Regel 1).
pub fn stufe_lesen(wert: &str) -> Option<Stufe> {
    match wert.to_lowercase().as_str() {
        "debug" => Some(Stufe::Debug),
        "information" => Some(Stufe::Information),
        "warning" => Some(Stufe::Warning),
        "error" => Some(Stufe::Error),
        _ => None,
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
                erreichbar_als: vec![],
                protokoll_stufe: None,
                alle_namen: false,
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
                erreichbar_als: vec![],
                protokoll_stufe: None,
                alle_namen: false,
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
    fn erreichbar_als_ist_mehrfach_erlaubt_und_sammelt_die_namen_in_der_reihenfolge() {
        let gelesen = auftrag(&[
            "/tmp/x",
            "--erreichbar-als",
            "chat.lan",
            "--erreichbar-als",
            "nuc",
        ]);
        assert_eq!(gelesen.erreichbar_als, vec!["chat.lan", "nuc"]);
        assert_eq!(gelesen.ordner, PathBuf::from("/tmp/x"));
    }

    #[test]
    fn erreichbar_als_ohne_wert_ist_ein_aufruffehler() {
        assert_eq!(
            lies(&["/tmp/x", "--erreichbar-als"]),
            Err(Aufruffehler::WertFehlt("--erreichbar-als".into()))
        );
    }

    #[test]
    fn die_hilfe_nennt_erreichbar_als_in_der_aufrufzeile_und_erklaert_es() {
        let aufrufzeile = HILFE.lines().next().unwrap_or_default();
        assert!(
            aufrufzeile.contains("--erreichbar-als <name>"),
            "{aufrufzeile}"
        );
        assert!(
            HILFE
                .lines()
                .any(|zeile| zeile.trim_start().starts_with("--erreichbar-als")),
            "{HILFE}"
        );
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

    #[test]
    fn protokoll_stufe_wird_gelesen_auch_gross_geschrieben() {
        assert_eq!(
            auftrag(&["--protokoll-stufe", "debug"]).protokoll_stufe,
            Some(Stufe::Debug)
        );
        assert_eq!(
            auftrag(&["--protokoll-stufe", "Warning"]).protokoll_stufe,
            Some(Stufe::Warning)
        );
        assert_eq!(auftrag(&["/tmp/x"]).protokoll_stufe, None);
    }

    #[test]
    fn eine_falsche_stufe_nennt_die_erlaubten_werte_exception_ist_nicht_waehlbar() {
        for falsch in ["laut", "exception", ""] {
            let fehler = lies(&["--protokoll-stufe", falsch]).expect_err(falsch);
            assert_eq!(fehler, Aufruffehler::StufeUngueltig(falsch.into()));
            assert!(
                fehler
                    .to_string()
                    .contains("debug, information, warning, error"),
                "{fehler}"
            );
        }
    }

    #[test]
    fn protokoll_stufe_ohne_wert_braucht_einen_wert() {
        let fehler = lies(&["--protokoll-stufe"]).expect_err("ohne Wert");
        assert_eq!(
            fehler.to_string(),
            "--protokoll-stufe braucht einen Wert — Aufruf siehe localchatA --hilfe"
        );
    }

    #[test]
    fn die_hilfe_nennt_die_protokoll_stufe_und_die_umgebung() {
        assert!(
            HILFE
                .lines()
                .next()
                .unwrap_or_default()
                .contains("[--protokoll-stufe <stufe>]"),
            "{HILFE}"
        );
        assert!(HILFE.contains("--protokoll-stufe <stufe>  Mindeststufe des Betriebsprotokolls unter <ordner>/logs/: debug, information (Standard), warning, error;"), "{HILFE}");
        assert!(HILFE.contains("LOCALCHATA_PROTOKOLL_STUFE"), "{HILFE}");
    }

    /// Erster Test aus A00003 (Kriterium F0080 „Aufruf“).
    #[test]
    fn alle_namen_wird_gelesen_ohne_ist_es_aus() {
        assert!(auftrag(&["/tmp/x", "--alle-namen"]).alle_namen);
        assert!(!auftrag(&["/tmp/x"]).alle_namen);
        assert_eq!(
            lies(&["/tmp/x", "--alle-namen=1"]),
            Err(Aufruffehler::UnbekannteOption("--alle-namen=1".into()))
        );
    }

    /// Matrix I16: Aufrufzeile, eigene Zeile mit dem Risikosatz, Namen des eigenen Netzes automatisch.
    #[test]
    fn die_hilfe_nennt_alle_namen_mit_dem_risiko_und_die_eigenen_namen() {
        assert!(
            HILFE
                .lines()
                .next()
                .unwrap_or_default()
                .contains("[--alle-namen]"),
            "{HILFE}"
        );
        assert!(
            HILFE
                .lines()
                .any(|zeile| zeile.trim_start().starts_with("--alle-namen")
                    && zeile.contains("LOCALCHATA_ALLE_NAMEN")),
            "{HILFE}"
        );
        let risiko = "eine fremde Website, die ein Teilnehmer öffnet, kann über DNS-Rebinding Channels mitlesen, Nachrichten schreiben und über die Frist Verläufe löschen.";
        assert!(
            HILFE
                .lines()
                .any(|zeile| zeile.trim_start() == format!("Risiko: {risiko}")),
            "{HILFE}"
        );
        assert!(super::super::allenamen::RISIKO.ends_with(risiko));
        assert!(HILFE.contains("gelten automatisch"), "{HILFE}");
    }
}
