//! Frist: welche Tage eines Gesprächs geladen werden (R00002: heute zählt mit, spätere Tage auch).

use chrono::NaiveDate;

/// Standardfrist, wenn `einstellungen.toml` oder darin `frist_tage` fehlt (ADR-0029 Regel 1).
pub const STANDARD_FRIST_TAGE: u32 = 7;
/// Kürzeste Frist: nur heute (ADR-0029 Regel 1).
pub const FRIST_MIN: u32 = 1;
/// Längste Frist, etwa zehn Jahre — „praktisch unbegrenzt“ (ADR-0029 Regel 1).
pub const FRIST_MAX: u32 = 3650;

/// Eine ganze Zahl als Frist in Tagen, wenn sie in der Spanne liegt — dieselbe Prüfung für API und Datei.
pub fn frist_in_spanne(zahl: i64) -> Option<u32> {
    u32::try_from(zahl)
        .ok()
        .filter(|tage| (FRIST_MIN..=FRIST_MAX).contains(tage))
}

/// Tage ab `heute − (frist_tage − 1)`, einschließlich heute und späterer Tage; ältere fallen weg.
pub fn tage_in_frist(tage: &[NaiveDate], heute: NaiveDate, frist_tage: u32) -> Vec<NaiveDate> {
    let zurueck = chrono::Days::new(u64::from(frist_tage.saturating_sub(1)));
    let aeltester = heute.checked_sub_days(zurueck).unwrap_or(NaiveDate::MIN);
    tage.iter()
        .copied()
        .filter(|tag| *tag >= aeltester)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tag(text: &str) -> NaiveDate {
        NaiveDate::parse_from_str(text, "%Y-%m-%d").expect("Testtag")
    }

    #[test]
    fn bei_frist_sieben_ist_der_einundzwanzigste_drin_der_zwanzigste_nicht() {
        let tage = [tag("2026-10-20"), tag("2026-10-21"), tag("2026-10-27")];

        assert_eq!(
            tage_in_frist(&tage, tag("2026-10-27"), STANDARD_FRIST_TAGE),
            vec![tag("2026-10-21"), tag("2026-10-27")]
        );
    }

    #[test]
    fn tage_nach_heute_werden_mitgeladen() {
        let tage = [tag("2026-10-27"), tag("2026-10-28")];
        assert_eq!(
            tage_in_frist(&tage, tag("2026-10-27"), 7),
            vec![tag("2026-10-27"), tag("2026-10-28")]
        );
    }

    #[test]
    fn frist_eins_ist_nur_heute_und_die_grenze_geht_ueber_den_monat() {
        let tage = [tag("2026-10-26"), tag("2026-10-27")];
        assert_eq!(
            tage_in_frist(&tage, tag("2026-10-27"), 1),
            vec![tag("2026-10-27")]
        );
        let ueber_monat = [tag("2026-10-26"), tag("2026-10-27"), tag("2026-11-02")];
        assert_eq!(
            tage_in_frist(&ueber_monat, tag("2026-11-02"), 7),
            vec![tag("2026-10-27"), tag("2026-11-02")]
        );
    }
}
