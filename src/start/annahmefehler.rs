//! Annahme gescheitert (R00018 F0059, ADR-0042 Ergänzung #24 Punkt 2): scheitert `accept`, kommen neue Clients
//! nicht herein — eine ERROR-Episode je Fehlernummer. Nur die Fehler, die eine einzelne Fremdverbindung betreffen
//! (`ECONNABORTED`, `EPROTO`, `EINTR`), sind DEBUG ohne Episode.

use crate::protokoll::episoden::Auftreten;
use crate::protokoll::{Bereich, Kontext, Protokoll, Schluessel, Stufe};
use std::io;

const TEXT: &str = "Verbindung nicht angenommen";

/// Wie ein Fehler der Annahme ins Protokoll geht.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Annahmefehler {
    /// ERROR-Episode; Quelle ist die Fehlernummer, sonst die Art des Fehlers.
    Episode { quelle: String },
    /// Betrifft nur eine Fremdverbindung: DEBUG ohne Episode.
    Debug,
}

/// Ordnet einen Fehler von `accept` ein (rein).
pub fn annahmefehler_einordnen(fehler: &io::Error) -> Annahmefehler {
    let nur_fremd = matches!(
        fehler.kind(),
        io::ErrorKind::ConnectionAborted | io::ErrorKind::Interrupted
    ) || ist_eproto(fehler);
    if nur_fremd {
        return Annahmefehler::Debug;
    }
    let quelle = fehler.raw_os_error().map_or_else(
        || format!("{:?}", fehler.kind()),
        |nummer| nummer.to_string(),
    );
    Annahmefehler::Episode { quelle }
}

#[cfg(unix)]
fn ist_eproto(fehler: &io::Error) -> bool {
    fehler.raw_os_error() == Some(rustix::io::Errno::PROTO.raw_os_error())
}

#[cfg(not(unix))]
fn ist_eproto(_fehler: &io::Error) -> bool {
    false
}

/// Trägt einen Fehler der Annahme ein: Episode je Fehlernummer oder eine DEBUG-Zeile; `grund` ist der Text des
/// Fehlers mit Nummer (ADR-0042 Ergänzung #24 Punkt 2).
pub fn annahmefehler_eintragen(protokoll: &Protokoll, fehler: &io::Error) {
    let kontext = Kontext::neu().mit(Schluessel::Grund, fehler);
    match annahmefehler_einordnen(fehler) {
        Annahmefehler::Episode { quelle } => protokoll.episode(Auftreten {
            stufe: Stufe::Error,
            bereich: Bereich::Verbindung,
            text: TEXT,
            quelle,
            kontext,
            art: None,
            verworfen: None,
            bytes: None,
            anzeige: None,
        }),
        Annahmefehler::Debug => {
            protokoll.eintragen(Stufe::Debug, Bereich::Verbindung, TEXT, kontext);
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::protokoll::testhilfe::im_speicher;
    use rustix::io::Errno;

    fn fehler(nummer: Errno) -> io::Error {
        io::Error::from_raw_os_error(nummer.raw_os_error())
    }

    #[test]
    fn fehlende_ressourcen_sind_eine_episode_je_fehlernummer() {
        for nummer in [Errno::MFILE, Errno::NFILE, Errno::NOMEM, Errno::NOBUFS] {
            assert_eq!(
                annahmefehler_einordnen(&fehler(nummer)),
                Annahmefehler::Episode {
                    quelle: nummer.raw_os_error().to_string()
                },
                "{nummer:?}"
            );
        }
        assert_eq!(
            annahmefehler_einordnen(&fehler(Errno::MFILE)),
            Annahmefehler::Episode {
                quelle: "24".to_string()
            }
        );
    }

    #[test]
    fn fehler_einer_fremdverbindung_sind_debug() {
        for nummer in [Errno::CONNABORTED, Errno::PROTO, Errno::INTR] {
            assert_eq!(
                annahmefehler_einordnen(&fehler(nummer)),
                Annahmefehler::Debug,
                "{nummer:?}"
            );
        }
    }

    #[test]
    fn zwanzig_mal_emfile_ist_genau_eine_error_zeile_mit_grund() {
        let (protokoll, speicher, _, _) = im_speicher("2026-10-28T12:00:00+01:00", Stufe::Debug);

        for _ in 0..20 {
            annahmefehler_eintragen(&protokoll, &fehler(Errno::MFILE));
        }

        let zeilen = speicher.zeilen();
        assert_eq!(zeilen.len(), 1, "{zeilen:#?}");
        assert!(
            zeilen[0].contains(
                " ERROR verbindung Verbindung nicht angenommen — Episode beginnt | grund="
            ),
            "{zeilen:#?}"
        );
        assert!(zeilen[0].contains("os error 24"), "{zeilen:#?}");
        assert_eq!(protokoll.zaehlerstand().errors, 20);
    }

    #[test]
    fn ein_abgebrochener_verbindungsaufbau_ist_eine_debug_zeile_ohne_episode() {
        let (protokoll, speicher, _, _) = im_speicher("2026-10-28T12:00:00+01:00", Stufe::Debug);

        annahmefehler_eintragen(&protokoll, &fehler(Errno::CONNABORTED));
        annahmefehler_eintragen(&protokoll, &fehler(Errno::CONNABORTED));

        let zeilen = speicher.zeilen();
        assert_eq!(zeilen.len(), 2, "{zeilen:#?}");
        assert!(
            zeilen
                .iter()
                .all(|zeile| zeile
                    .contains(" DEBUG verbindung Verbindung nicht angenommen | grund=")),
            "{zeilen:#?}"
        );
    }
}
