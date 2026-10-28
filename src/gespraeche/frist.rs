//! Frist (ADR-0029): welche Tage eines Gesprächs bleiben und welche entfallen — heute zählt mit, spätere Tage
//! bleiben auch. Laden und Entfallen rechnen mit derselben Grenze (Regel 3).

use chrono::NaiveDate;

/// Standardfrist, wenn `einstellungen.toml` oder darin `frist_tage` fehlt (ADR-0029 Regel 1).
pub const STANDARD_FRIST_TAGE: u32 = 7;
/// Kürzeste Frist: nur heute (ADR-0029 Regel 1).
pub const FRIST_MIN: u32 = 1;
/// Längste Frist, etwa zehn Jahre — „praktisch unbegrenzt“ (ADR-0029 Regel 1).
pub const FRIST_MAX: u32 = 3650;

/// Die Frist eines Channels: Tage in der Spanne, oder gesperrt, weil seine Einstellungsdatei defekt oder nicht
/// lesbar ist — dann entfällt nichts (ADR-0032 Regel 5), geladen wird mit dem Standard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Frist {
    Tage(u32),
    Gesperrt {
        /// Einstellungsdatei relativ zum Ordner.
        datei: String,
        /// Zeile des Defekts; fehlt, wenn das Dateisystem die Datei verweigert.
        zeile: Option<usize>,
    },
}

impl Frist {
    /// Mit wie vielen Tagen geladen wird: gesperrt mit dem Standard.
    pub fn zum_laden(&self) -> u32 {
        match self {
            Frist::Tage(tage) => *tage,
            Frist::Gesperrt { .. } => STANDARD_FRIST_TAGE,
        }
    }
}

/// Eine ganze Zahl als Frist in Tagen, wenn sie in der Spanne liegt — dieselbe Prüfung für API und Datei.
pub fn frist_in_spanne(zahl: i64) -> Option<u32> {
    u32::try_from(zahl)
        .ok()
        .filter(|tage| (FRIST_MIN..=FRIST_MAX).contains(tage))
}

/// Teilt die Tage in (behalten, entfallen): behalten ab `heute − (n − 1)`, einschließlich heute und späterer Tage;
/// jeder Tag liegt in genau einer Menge, beide in der Reihenfolge der Eingabe.
pub fn teilen(tage: &[NaiveDate], heute: NaiveDate, n: u32) -> (Vec<NaiveDate>, Vec<NaiveDate>) {
    let grenze = aeltester_behaltener(heute, n);
    tage.iter().partition(|tag| **tag >= grenze)
}

/// `heute − (n − 1)` (ADR-0029 Regel 2); vor dem kleinsten Datum das kleinste.
pub fn aeltester_behaltener(heute: NaiveDate, n: u32) -> NaiveDate {
    let zurueck = chrono::Days::new(u64::from(n.saturating_sub(1)));
    heute.checked_sub_days(zurueck).unwrap_or(NaiveDate::MIN)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tag(text: &str) -> NaiveDate {
        NaiveDate::parse_from_str(text, "%Y-%m-%d").expect("Testtag")
    }

    fn tage(texte: &[&str]) -> Vec<NaiveDate> {
        texte.iter().map(|text| tag(text)).collect()
    }

    /// R00008 F0033 „Grenze“: 28.10. − (7 − 1) = 22.10. ist der älteste behaltene Tag.
    #[test]
    fn frist_sieben_am_achtundzwanzigsten_behaelt_ab_dem_zweiundzwanzigsten() {
        let auf_der_platte = tage(&["2026-10-21", "2026-10-22", "2026-10-28", "2026-10-29"]);

        assert_eq!(
            teilen(&auf_der_platte, tag("2026-10-28"), 7),
            (
                tage(&["2026-10-22", "2026-10-28", "2026-10-29"]),
                tage(&["2026-10-21"])
            )
        );
    }

    #[test]
    fn frist_eins_ist_nur_heute_und_3650_behaelt_alles() {
        let auf_der_platte = tage(&["2026-10-21", "2026-10-22", "2026-10-28", "2026-10-29"]);

        assert_eq!(
            teilen(&auf_der_platte, tag("2026-10-28"), 1).1,
            tage(&["2026-10-21", "2026-10-22"])
        );
        assert_eq!(teilen(&auf_der_platte, tag("2026-10-28"), 3650).1, vec![]);
        assert_eq!(
            teilen(
                &tage(&["2016-10-30", "2016-10-31"]),
                tag("2026-10-28"),
                3650
            ),
            (tage(&["2016-10-31"]), tage(&["2016-10-30"]))
        );
    }

    /// ADR-0029 Regel 5: über den Monat und über das Jahr.
    #[test]
    fn die_grenze_geht_ueber_monat_und_jahr() {
        assert_eq!(
            teilen(&tage(&["2026-10-03", "2026-10-04"]), tag("2026-11-02"), 30),
            (tage(&["2026-10-04"]), tage(&["2026-10-03"]))
        );
        assert_eq!(
            teilen(&tage(&["2026-12-27", "2026-12-28"]), tag("2027-01-03"), 7),
            (tage(&["2026-12-28"]), tage(&["2026-12-27"]))
        );
    }

    #[test]
    fn jeder_tag_liegt_in_genau_einer_menge() {
        let auf_der_platte: Vec<NaiveDate> = (0..60)
            .map(|zurueck| tag("2026-11-15") - chrono::Days::new(zurueck))
            .collect();
        for n in [1, 2, 7, 30, 31, 59, 60, 3650] {
            let (behalten, entfallen) = teilen(&auf_der_platte, tag("2026-11-10"), n);

            assert_eq!(
                behalten.len() + entfallen.len(),
                auf_der_platte.len(),
                "{n}"
            );
            assert!(behalten.iter().all(|tag| !entfallen.contains(tag)), "{n}");
        }
    }
}
