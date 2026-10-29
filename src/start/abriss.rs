//! Abriss an der Abrissgrenze erkennen (R00018 F0060, ADR-0042 Ergänzung #24 Punkt 7): eine Verbindung, die der
//! Kernel nach der Sendefrist oder bei stillem Abriss schließt, endet mit `ETIMEDOUT` — irgendwo in der
//! Quellenkette des Fehlers, den `serve_connection` liefert.

use crate::protokoll::episoden::Auftreten;
use crate::protokoll::{Bereich, Kontext, Protokoll, Schluessel, Stufe};
use std::error::Error;
use std::io;
use std::net::IpAddr;
use std::time::Duration;

/// Was eine Verbindung braucht, um ihren Abriss einzutragen: Protokoll, Herkunft, Abrissgrenze (falls gepflegt).
pub struct Abrissspur<'a> {
    pub protokoll: &'a Protokoll,
    pub herkunft: IpAddr,
    pub grenze: Option<Duration>,
}

impl Abrissspur<'_> {
    /// Endete die Verbindung mit einem Abriss, zählt er in `abgerissen` (immer) und geht als WARNING-Episode je
    /// Herkunft ins Protokoll — `herkunft`, `hoechstens` = Abrissgrenze in ms, ohne `grenze` (es gibt nur eine Art).
    pub fn verbindung_endete<E: Error + 'static>(&self, ergebnis: &Result<(), E>) {
        let Err(fehler) = ergebnis else {
            return;
        };
        if !ist_abriss(fehler) {
            return;
        }
        self.protokoll.abriss_zaehlen();
        let mut kontext = Kontext::neu().mit(Schluessel::Herkunft, self.herkunft);
        if let Some(grenze) = self.grenze {
            kontext = kontext.mit(Schluessel::Hoechstens, grenze.as_millis());
        }
        self.protokoll.episode(Auftreten {
            stufe: Stufe::Warning,
            bereich: Bereich::Verbindung,
            text: "Verbindung an der Abrissgrenze geschlossen",
            quelle: self.herkunft.to_string(),
            kontext,
            art: None,
            verworfen: None,
            bytes: None,
            anzeige: None,
        });
    }
}

/// Wahr, wenn in der Quellenkette ein `io::Error` der Art `TimedOut` steht (rein).
pub fn ist_abriss(fehler: &(dyn Error + 'static)) -> bool {
    let mut glied = Some(fehler);
    while let Some(fehler) = glied {
        let abgelaufen = fehler
            .downcast_ref::<io::Error>()
            .is_some_and(|io| io.kind() == io::ErrorKind::TimedOut);
        if abgelaufen {
            return true;
        }
        glied = fehler.source();
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt;

    /// Ein Fehler, der einen anderen einpackt — wie hyper den `io::Error` der Verbindung.
    #[derive(Debug)]
    struct Eingepackt(io::Error);

    impl fmt::Display for Eingepackt {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "Verbindung gescheitert")
        }
    }

    impl Error for Eingepackt {
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            Some(&self.0)
        }
    }

    #[test]
    fn timed_out_ist_ein_abriss_auch_eingepackt() {
        let direkt = io::Error::from(io::ErrorKind::TimedOut);
        let eingepackt = Eingepackt(io::Error::from(io::ErrorKind::TimedOut));

        assert!(ist_abriss(&direkt));
        assert!(ist_abriss(&eingepackt));
    }

    #[test]
    fn andere_enden_sind_kein_abriss() {
        for art in [
            io::ErrorKind::ConnectionReset,
            io::ErrorKind::BrokenPipe,
            io::ErrorKind::UnexpectedEof,
        ] {
            assert!(!ist_abriss(&io::Error::from(art)), "{art:?}");
            assert!(!ist_abriss(&Eingepackt(io::Error::from(art))), "{art:?}");
        }
    }

    /// R00018 F0060 (ADR-0042 Ergänzung #24 Punkt 7): zwei Abrisse derselben Herkunft → eine Episode, zwei
    /// gezählt; ein anderes Ende zählt nicht und schreibt nichts.
    #[test]
    fn ein_abriss_ist_eine_episode_je_herkunft_und_zaehlt_jedes_mal() {
        use crate::protokoll::testhilfe::im_speicher;
        let (protokoll, speicher, _, _) =
            im_speicher("2026-10-28T12:00:00+01:00", Stufe::Information);
        let spur = Abrissspur {
            protokoll: &protokoll,
            herkunft: "127.0.0.2".parse().expect("IP"),
            grenze: Some(Duration::from_millis(4000)),
        };

        spur.verbindung_endete(&Err(Eingepackt(io::Error::from(io::ErrorKind::TimedOut))));
        spur.verbindung_endete(&Err(io::Error::from(io::ErrorKind::TimedOut)));
        spur.verbindung_endete(&Err(io::Error::from(io::ErrorKind::ConnectionReset)));
        spur.verbindung_endete::<io::Error>(&Ok(()));

        assert_eq!(
            speicher.zeilen(),
            [
                "2026-10-28T12:00:00.000+01:00 WARNING verbindung Verbindung an der Abrissgrenze geschlossen — Episode beginnt | herkunft=127.0.0.2 hoechstens=4000"
            ]
        );
        assert_eq!(protokoll.kennzahlen().abgerissen, 2);
        assert_eq!(protokoll.zaehlerstand().warnings, 2);
    }
}
