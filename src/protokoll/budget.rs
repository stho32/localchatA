//! Tagesbudget je Protokolldatei und Stufenklasse (ADR-0042 Regel 6): Debug 64 MiB, Information+Warning 32 MiB,
//! Error+Exception 32 MiB, Eingriff 16 MiB (ADR-0049). Rein: entscheidet, ob eine Zeile noch passt.

use super::Stufe;
use super::eingriffe::ist_eingriff;
use chrono::NaiveDate;
use std::borrow::Cow;

const MIB: u64 = 1024 * 1024;

/// Budgets je Klasse in Bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Budgets {
    pub debug: u64,
    pub information_warning: u64,
    pub error_exception: u64,
    pub eingriff: u64,
}

/// Die Budgets des Programms.
pub const BUDGETS: Budgets = Budgets {
    debug: 64 * MIB,
    information_warning: 32 * MIB,
    error_exception: 32 * MIB,
    eingriff: 16 * MIB,
};

/// Stufenklasse eines Eintrags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Klasse {
    Debug,
    InformationWarning,
    ErrorException,
    /// Zeilen, die gemeinsame oder fremde Daten ändern oder löschen (ADR-0049).
    Eingriff,
}

impl Klasse {
    pub fn von(stufe: Stufe) -> Klasse {
        match stufe {
            Stufe::Debug => Klasse::Debug,
            Stufe::Information | Stufe::Warning => Klasse::InformationWarning,
            Stufe::Error | Stufe::Exception => Klasse::ErrorException,
        }
    }

    /// Klasse einer Zeile aus Stufe, Bereichsname und Text: Eingriffszeilen (ADR-0049) zählen in `Eingriff`, alles
    /// andere nach der Stufe. Die eine Zuordnung beim Schreiben und beim Wiederlesen.
    pub fn zuordnen(stufe: Stufe, bereich: &str, text: &str) -> Klasse {
        if ist_eingriff(stufe, bereich, text) {
            Klasse::Eingriff
        } else {
            Klasse::von(stufe)
        }
    }

    pub fn als_text(self) -> &'static str {
        match self {
            Klasse::Debug => "DEBUG",
            Klasse::InformationWarning => "INFORMATION+WARNING",
            Klasse::ErrorException => "ERROR+EXCEPTION",
            Klasse::Eingriff => "EINGRIFF",
        }
    }

    fn stelle(self) -> usize {
        match self {
            Klasse::Debug => 0,
            Klasse::InformationWarning => 1,
            Klasse::ErrorException => 2,
            Klasse::Eingriff => 3,
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
    verbraucht: [u64; 4],
    erreicht: [bool; 4],
}

impl Tagesbudget {
    pub fn neu(budgets: Budgets) -> Tagesbudget {
        Tagesbudget {
            budgets,
            tag: None,
            verbraucht: [0; 4],
            erreicht: [false; 4],
        }
    }

    /// Gilt der Verbrauch schon für `tag`? Nein vor dem ersten Eintrag des Tages im Prozess.
    pub fn gilt_fuer(&self, tag: NaiveDate) -> bool {
        self.tag == Some(tag)
    }

    /// Rechnet den Verbrauch einer vorhandenen Tagesdatei an (ADR-0042 Ergänzung #24 Punkt 9); eine Klasse über
    /// ihrem Budget verwirft ab ihrer nächsten Zeile, die erste davon meldet das wie sonst.
    pub fn anrechnen(&mut self, tag: NaiveDate, verbrauch: [u64; 4]) {
        self.tag = Some(tag);
        self.verbraucht = verbrauch;
        self.erreicht = [false; 4];
    }

    /// Passt eine Zeile von `laenge` Bytes der Klasse `klasse` am Tag `tag` noch ins Budget? Ein neuer Tag beginnt
    /// mit neuem Budget.
    pub fn pruefen(&mut self, tag: NaiveDate, klasse: Klasse, laenge: u64) -> Urteil {
        if self.tag != Some(tag) {
            self.tag = Some(tag);
            self.verbraucht = [0; 4];
            self.erreicht = [false; 4];
        }
        let budget = match klasse {
            Klasse::Debug => self.budgets.debug,
            Klasse::InformationWarning => self.budgets.information_warning,
            Klasse::ErrorException => self.budgets.error_exception,
            Klasse::Eingriff => self.budgets.eingriff,
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

/// Verbrauch einer vorhandenen Tagesdatei je Klasse `[DEBUG, INFORMATION+WARNING, ERROR+EXCEPTION, EINGRIFF]` in Bytes
/// (ADR-0042 Ergänzung #24 Punkt 9): je Zeile ihre Bytes, wie gegeben (mit Zeilenende, wenn es dabei ist), der
/// Klasse der Stufe im zweiten Wort; eine Zeile ohne erkennbare Stufe zählt zu DEBUG (rein).
pub fn verbrauch_je_klasse<Z: AsRef<[u8]>>(zeilen: impl IntoIterator<Item = Z>) -> [u64; 4] {
    let mut verbrauch = [0; 4];
    for zeile in zeilen {
        let zeile = zeile.as_ref();
        verbrauch[klasse_der_zeile(zeile).stelle()] += zeile.len() as u64;
    }
    verbrauch
}

/// Klasse einer Zeile der Tagesdatei: Stufe (zweites Wort), Bereich (drittes) und der Text danach durch dieselbe
/// Zuordnung wie beim Schreiben; was keine erkennbare Stufe hat, ist DEBUG.
fn klasse_der_zeile(zeile: &[u8]) -> Klasse {
    let mut woerter = zeile.splitn(4, |&byte| byte == b' ').skip(1);
    let Some(stufe) = woerter.next().and_then(stufe_aus_wort) else {
        return Klasse::Debug;
    };
    let bereich = woerter.next().map_or(Cow::Borrowed(""), |wort| {
        String::from_utf8_lossy(wort.trim_ascii_end())
    });
    let text = woerter
        .next()
        .map_or(Cow::Borrowed(""), String::from_utf8_lossy);
    Klasse::zuordnen(stufe, &bereich, &text)
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
        assert_eq!(verbrauch_je_klasse(&zeilen), [113, 50, 30, 0]);
    }

    #[test]
    fn verbrauch_ordnet_information_und_error_ihren_klassen_zu() {
        let zeilen = [
            "t INFORMATION start a\n",
            "t ERROR protokoll b\n",
            "t WARNINGX start c\n",
        ];

        assert_eq!(verbrauch_je_klasse(zeilen), [19, 22, 20, 0]);
    }
    /// R00034 F0125 (ADR-0049 Regel 3, Wiederlesen): 2.100 B `INFORMATION sitzung` und 100 B `INFORMATION frist` →
    /// `[0, 2100, 0, 100]`. Mutante „Wiederlesen nur nach Stufe“ → `[0, 2200, 0, 0]`.
    #[test]
    fn wiederlesen_ordnet_fristzeilen_der_klasse_eingriff_zu() {
        let zeile = |bereich: &str, text: &str, laenge: usize| {
            let anfang = format!("2026-10-29T11:00:00.000+01:00 INFORMATION {bereich} {text}");
            format!("{anfang}{}\n", "x".repeat(laenge - anfang.len() - 1))
        };
        let zeilen = [
            zeile("sitzung", "Sitzung beigetreten ", 1050),
            zeile("sitzung", "Sitzung verlassen ", 1050),
            zeile("frist", "Frist 7 → 30 Tage gesetzt ", 100),
        ];
        assert_eq!(zeilen.iter().map(String::len).sum::<usize>(), 2200);

        assert_eq!(verbrauch_je_klasse(&zeilen), [0, 2100, 0, 100]);
    }

    #[test]
    fn wiederlesen_erkennt_geloeschte_anhaenge_nur_am_textanfang_und_nur_bei_information() {
        let zeilen = [
            "t INFORMATION anhang Anhänge gelöscht | anhaenge=1\n",
            "t INFORMATION anhang Großes Hochladen abgeschlossen\n",
            "t ERROR frist Tag nicht entfallen\n",
            "t INFORMATION frist\n",
        ];

        let verbrauch = verbrauch_je_klasse(zeilen);

        assert_eq!(verbrauch[3], (zeilen[0].len() + zeilen[3].len()) as u64);
        assert_eq!(verbrauch[1], zeilen[1].len() as u64);
        assert_eq!(verbrauch[2], zeilen[2].len() as u64);
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

    /// R00034 F0125 (ADR-0049 Regel 1 und 4): 16 MiB, in Summe 144 MiB je Tag.
    #[test]
    fn das_eingriffsbudget_hat_16_mib_und_die_grenzen_summieren_sich_auf_144_mib() {
        assert_eq!(BUDGETS.eingriff, 16_777_216);
        let summe = BUDGETS.debug
            + BUDGETS.information_warning
            + BUDGETS.error_exception
            + BUDGETS.eingriff;
        assert_eq!(summe, 144 * MIB);
    }

    /// R00034 F0125: Klasse EINGRIFF mit 1 KiB — genau eine Budgetmeldung, danach verworfen; INFORMATION+WARNING
    /// bleibt davon unberührt, und umgekehrt.
    #[test]
    fn die_klasse_eingriff_hat_ihr_eigenes_budget_getrennt_von_information_warning() {
        let tag = NaiveDate::from_ymd_opt(2026, 10, 29).expect("Tag");
        let mut budget = Tagesbudget::neu(Budgets {
            debug: 1024,
            information_warning: 1024,
            error_exception: 1024,
            eingriff: 1024,
        });

        assert_eq!(
            budget.pruefen(tag, Klasse::Eingriff, 1000),
            Urteil::Schreiben
        );
        assert_eq!(
            budget.pruefen(tag, Klasse::Eingriff, 100),
            Urteil::ErstmalsVerworfen
        );
        assert_eq!(budget.pruefen(tag, Klasse::Eingriff, 1), Urteil::Verworfen);
        assert_eq!(
            budget.pruefen(tag, Klasse::InformationWarning, 1000),
            Urteil::Schreiben
        );
        assert_eq!(Klasse::Eingriff.als_text(), "EINGRIFF");

        let mut weit = Tagesbudget::neu(Budgets {
            information_warning: 1024,
            eingriff: 4096,
            ..BUDGETS
        });
        assert_eq!(
            weit.pruefen(tag, Klasse::Eingriff, 3000),
            Urteil::Schreiben,
            "eigenes Budget, nicht das der Information"
        );

        let mut anders = Tagesbudget::neu(Budgets {
            eingriff: 1024,
            ..BUDGETS
        });
        assert_eq!(
            anders.pruefen(tag, Klasse::Eingriff, 1024),
            Urteil::Schreiben
        );
        anders.anrechnen(tag, [0, 33_554_432, 0, 0]);
        assert_eq!(
            anders.pruefen(tag, Klasse::InformationWarning, 1),
            Urteil::ErstmalsVerworfen
        );
        assert_eq!(anders.pruefen(tag, Klasse::Eingriff, 1), Urteil::Schreiben);
    }

    fn protokoll_mit_kleinen_budgets(speicher: &Speicher) -> Protokoll {
        let uhr = Arc::new(GestellteUhr::neu(zeit("2026-10-29T12:00:00+01:00")));
        Protokoll::mit(
            Box::new(speicher.clone()),
            Aufzeichnung::default().spiegel(),
            uhr as Arc<dyn Uhr>,
            Stufe::Information,
        )
        .mit_budgets(Budgets {
            debug: 1024,
            information_warning: 1024,
            error_exception: 1024,
            eingriff: 1024,
        })
    }

    /// R00034 F0125 (ADR-0049 Regel 3): eine Frist-Zeile zählt in EINGRIFF, nicht in INFORMATION+WARNING — sie
    /// steht auch dann in der Datei, wenn das allgemeine Budget erschöpft ist. Mutante „Klasse nur aus der Stufe“
    /// lässt sie verwerfen.
    #[test]
    fn eine_fristzeile_steht_auch_bei_erschoepftem_informationsbudget() {
        let speicher = Speicher::neu();
        let protokoll = protokoll_mit_kleinen_budgets(&speicher);
        for _ in 0..30 {
            protokoll.eintragen(
                Stufe::Information,
                Bereich::Sitzung,
                "Sitzung beigetreten",
                Kontext::neu(),
            );
        }
        assert!(
            protokoll.kennzahlen().verworfen > 0,
            "Budget muss erschöpft sein"
        );

        protokoll.eintragen(
            Stufe::Information,
            Bereich::Frist,
            "Frist 7 → 30 Tage gesetzt",
            Kontext::neu(),
        );

        let zeilen = speicher.zeilen();
        assert!(
            zeilen
                .last()
                .is_some_and(|z| z.contains(" INFORMATION frist Frist 7 → 30 Tage gesetzt")),
            "{zeilen:#?}"
        );
        assert!(
            zeilen.iter().all(|z| !z.contains("Tagesbudget EINGRIFF")),
            "{zeilen:#?}"
        );
    }

    /// R00034 F0125: „Anhänge gelöscht“ ebenso; die Klassen sind in beide Richtungen getrennt.
    #[test]
    fn geloeschte_anhaenge_zaehlen_in_eingriff_und_sitzungszeilen_nicht() {
        let speicher = Speicher::neu();
        let protokoll = protokoll_mit_kleinen_budgets(&speicher);
        for _ in 0..30 {
            protokoll.eintragen(
                Stufe::Information,
                Bereich::Frist,
                "Frist 7 → 30 Tage gesetzt",
                Kontext::neu(),
            );
        }
        let verworfen = protokoll.kennzahlen().verworfen;
        assert!(verworfen > 0, "EINGRIFF muss erschöpft sein");
        assert!(
            speicher
                .zeilen()
                .iter()
                .any(|z| z.contains("Tagesbudget EINGRIFF erreicht")),
            "Budgetzeile der Klasse EINGRIFF"
        );

        protokoll.eintragen(
            Stufe::Information,
            Bereich::Sitzung,
            "Sitzung beigetreten",
            Kontext::neu(),
        );
        protokoll.eintragen(
            Stufe::Information,
            Bereich::Anhang,
            "Anhänge gelöscht",
            Kontext::neu(),
        );

        let zeilen = speicher.zeilen();
        assert_eq!(
            protokoll.kennzahlen().verworfen,
            verworfen + 1,
            "nur die Anhang-Zeile zählt in EINGRIFF"
        );
        assert!(
            zeilen
                .iter()
                .any(|z| z.contains(" INFORMATION sitzung Sitzung beigetreten")),
            "{zeilen:#?}"
        );
        assert!(
            zeilen
                .iter()
                .all(|z| !z.contains("Tagesbudget INFORMATION+WARNING")),
            "{zeilen:#?}"
        );
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
            eingriff: 1024,
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
