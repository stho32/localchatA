//! Tagesbudget je Protokolldatei und Stufenklasse (ADR-0042 Regel 6): Debug 64 MiB, Information+Warning 32 MiB,
//! Error+Exception 32 MiB. Rein: entscheidet, ob eine Zeile noch passt.

use super::Stufe;
use chrono::NaiveDate;

const MIB: u64 = 1024 * 1024;

/// Budgets je Klasse in Bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Budgets {
    pub debug: u64,
    pub information_warning: u64,
    pub error_exception: u64,
}

/// Die Budgets des Programms.
pub const BUDGETS: Budgets = Budgets {
    debug: 64 * MIB,
    information_warning: 32 * MIB,
    error_exception: 32 * MIB,
};

/// Stufenklasse eines Eintrags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Klasse {
    Debug,
    InformationWarning,
    ErrorException,
}

impl Klasse {
    pub fn von(stufe: Stufe) -> Klasse {
        match stufe {
            Stufe::Debug => Klasse::Debug,
            Stufe::Information | Stufe::Warning => Klasse::InformationWarning,
            Stufe::Error | Stufe::Exception => Klasse::ErrorException,
        }
    }

    pub fn als_text(self) -> &'static str {
        match self {
            Klasse::Debug => "DEBUG",
            Klasse::InformationWarning => "INFORMATION+WARNING",
            Klasse::ErrorException => "ERROR+EXCEPTION",
        }
    }

    fn stelle(self) -> usize {
        match self {
            Klasse::Debug => 0,
            Klasse::InformationWarning => 1,
            Klasse::ErrorException => 2,
        }
    }
}

/// Was mit einer Zeile geschieht.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Urteil {
    Schreiben,
    /// Das Budget ist eben erreicht: die Meldung schreiben, die Zeile verwerfen.
    ErstmalsVerworfen,
    Verworfen,
}

/// Verbrauch des laufenden lokalen Tages.
pub struct Tagesbudget {
    budgets: Budgets,
    tag: Option<NaiveDate>,
    verbraucht: [u64; 3],
    erreicht: [bool; 3],
}

impl Tagesbudget {
    pub fn neu(budgets: Budgets) -> Tagesbudget {
        Tagesbudget {
            budgets,
            tag: None,
            verbraucht: [0; 3],
            erreicht: [false; 3],
        }
    }

    /// Passt eine Zeile von `laenge` Bytes der Klasse `klasse` am Tag `tag` noch ins Budget? Ein neuer Tag beginnt
    /// mit neuem Budget.
    pub fn pruefen(&mut self, tag: NaiveDate, klasse: Klasse, laenge: u64) -> Urteil {
        if self.tag != Some(tag) {
            self.tag = Some(tag);
            self.verbraucht = [0; 3];
            self.erreicht = [false; 3];
        }
        let budget = match klasse {
            Klasse::Debug => self.budgets.debug,
            Klasse::InformationWarning => self.budgets.information_warning,
            Klasse::ErrorException => self.budgets.error_exception,
        };
        let stelle = klasse.stelle();
        if self.verbraucht[stelle] + laenge <= budget && !self.erreicht[stelle] {
            self.verbraucht[stelle] += laenge;
            return Urteil::Schreiben;
        }
        if self.erreicht[stelle] {
            Urteil::Verworfen
        } else {
            self.erreicht[stelle] = true;
            Urteil::ErstmalsVerworfen
        }
    }
}

/// Verbrauch einer vorhandenen Tagesdatei je Klasse `[DEBUG, INFORMATION+WARNING, ERROR+EXCEPTION]` in Bytes
/// (ADR-0042 Ergänzung #24 Punkt 9): je Zeile ihre Bytes, wie gegeben (mit Zeilenende, wenn es dabei ist), der
/// Klasse der Stufe im zweiten Wort; eine Zeile ohne erkennbare Stufe zählt zu DEBUG (rein).
pub fn verbrauch_je_klasse<Z: AsRef<[u8]>>(zeilen: impl IntoIterator<Item = Z>) -> [u64; 3] {
    let mut verbrauch = [0; 3];
    for zeile in zeilen {
        let zeile = zeile.as_ref();
        let klasse = zeile
            .split(|&byte| byte == b' ')
            .nth(1)
            .and_then(stufe_aus_wort)
            .map_or(Klasse::Debug, Klasse::von);
        verbrauch[klasse.stelle()] += zeile.len() as u64;
    }
    verbrauch
}

fn stufe_aus_wort(wort: &[u8]) -> Option<Stufe> {
    [
        Stufe::Debug,
        Stufe::Information,
        Stufe::Warning,
        Stufe::Error,
        Stufe::Exception,
    ]
    .into_iter()
    .find(|stufe| stufe.als_text().as_bytes() == wort)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// R00018 F0061: Bytes je Klasse nach dem zweiten Wort; eine kaputte Zeile zählt zu DEBUG.
    #[test]
    fn verbrauch_je_klasse_nach_dem_zweiten_wort_fremde_zeilen_zu_debug() {
        let zeile = |stufe: &str, laenge: usize| {
            let anfang = format!("t {stufe} start ");
            format!("{anfang}{}", "x".repeat(laenge - anfang.len()))
        };
        let zeilen = [
            zeile("DEBUG", 100),
            zeile("WARNING", 50),
            zeile("EXCEPTION", 30),
            "kaputte zeile".to_string(),
        ];

        assert_eq!(zeilen[0].len(), 100);
        assert_eq!(verbrauch_je_klasse(&zeilen), [113, 50, 30]);
    }

    #[test]
    fn verbrauch_ordnet_information_und_error_ihren_klassen_zu() {
        let zeilen = [
            "t INFORMATION start a\n",
            "t ERROR protokoll b\n",
            "t WARNINGX start c\n",
        ];

        assert_eq!(verbrauch_je_klasse(zeilen), [19, 22, 20]);
    }
    use crate::protokoll::senke::Speicher;
    use crate::protokoll::testhilfe::{Aufzeichnung, zeit};
    use crate::protokoll::{Bereich, Kontext, Protokoll};
    use crate::uhr::{GestellteUhr, Uhr};
    use std::sync::Arc;

    #[test]
    fn die_budgets_sind_64_32_und_32_mib() {
        assert_eq!(BUDGETS.debug, 67_108_864);
        assert_eq!(BUDGETS.information_warning, 33_554_432);
        assert_eq!(BUDGETS.error_exception, 33_554_432);
    }

    #[test]
    fn ueber_dem_budget_wird_verworfen_errors_bleiben_und_am_naechsten_tag_gilt_ein_neues() {
        let uhr = Arc::new(GestellteUhr::neu(zeit("2026-10-28T12:00:00+01:00")));
        let speicher = Speicher::neu();
        let protokoll = Protokoll::mit(
            Box::new(speicher.clone()),
            Aufzeichnung::default().spiegel(),
            Arc::clone(&uhr) as Arc<dyn Uhr>,
            Stufe::Debug,
        )
        .mit_budgets(Budgets {
            debug: 1024,
            information_warning: 1024,
            error_exception: 1024,
        });
        let debug = |protokoll: &Protokoll| {
            protokoll.eintragen(
                Stufe::Debug,
                Bereich::Verlauf,
                "x".repeat(55),
                Kontext::neu(),
            );
        };

        for _ in 0..13 {
            debug(&protokoll);
        }
        let zeilen = speicher.zeilen();
        assert!(
            zeilen
                .iter()
                .all(|z| z.len() + 1 == 100 || !z.contains(" DEBUG verlauf ")),
            "100 Bytes je Zeile"
        );
        assert_eq!(
            zeilen
                .iter()
                .filter(|z| z.contains(" DEBUG verlauf "))
                .count(),
            10,
            "{zeilen:#?}"
        );
        let erreicht: Vec<&String> = zeilen
            .iter()
            .filter(|z| z.contains(" WARNING protokoll Tagesbudget DEBUG erreicht — weitere Einträge bis Mitternacht nur gezählt"))
            .collect();
        assert_eq!(erreicht.len(), 1, "{zeilen:#?}");
        assert_eq!(protokoll.kennzahlen().verworfen, 3);
        debug(&protokoll);
        assert_eq!(protokoll.kennzahlen().verworfen, 4);

        protokoll.eintragen(Stufe::Error, Bereich::Verlauf, "danach", Kontext::neu());
        assert!(
            speicher
                .zeilen()
                .last()
                .is_some_and(|z| z.contains(" ERROR verlauf danach"))
        );

        uhr.stellen(zeit("2026-10-29T00:00:00+01:00"));
        debug(&protokoll);
        assert!(
            speicher
                .zeilen()
                .last()
                .is_some_and(|z| z.contains(" DEBUG verlauf "))
        );
    }
}
