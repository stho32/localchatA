//! Bereinigung nach 14 Kalendertagen (ADR-0041 Regeln 4–7): welche Protokolldatei entfällt, und das Löschen
//! nach Erlaubnisliste.

use chrono::NaiveDate;

/// Aufbewahrung in Kalendertagen; heute zählt mit (ADR-0029, ADR-0041 Regel 4).
pub const AUFBEWAHRUNG_TAGE: i64 = 14;

/// Der Tag aus einem Dateinamen, wenn er genau `JJJJ-MM-TT.log` mit gültigem Datum ist; sonst ist die Datei fremd.
pub fn tag_aus_namen(name: &str) -> Option<NaiveDate> {
    let stamm = name.strip_suffix(".log")?;
    let form_passt = stamm.len() == 10
        && stamm.char_indices().all(|(stelle, zeichen)| match stelle {
            4 | 7 => zeichen == '-',
            _ => zeichen.is_ascii_digit(),
        });
    if !form_passt {
        return None;
    }
    NaiveDate::parse_from_str(stamm, "%Y-%m-%d").ok()
}

/// Datei vom Tag `tag` entfällt ⇔ `tag < heute − 13`.
pub fn entfaellt(tag: NaiveDate, heute: NaiveDate) -> bool {
    tag < heute - chrono::Duration::days(AUFBEWAHRUNG_TAGE - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tag(text: &str) -> NaiveDate {
        NaiveDate::parse_from_str(text, "%Y-%m-%d").expect("Tag")
    }

    #[test]
    fn am_28_bleiben_der_15_bis_28_der_14_entfaellt() {
        let heute = tag("2026-10-28");
        assert!(entfaellt(tag("2026-10-13"), heute));
        assert!(entfaellt(tag("2026-10-14"), heute));
        assert!(!entfaellt(tag("2026-10-15"), heute));
        assert!(!entfaellt(tag("2026-10-28"), heute));
        assert!(!entfaellt(tag("2026-10-29"), heute), "Zukunft bleibt");
    }

    #[test]
    fn nur_genau_jjjj_mm_tt_log_mit_gueltigem_datum_ist_eine_protokolldatei() {
        assert_eq!(tag_aus_namen("2026-10-14.log"), Some(tag("2026-10-14")));
        for fremd in [
            "notizen.txt",
            "2026-10-14.log.gz",
            "2026-10-14.LOG",
            "2026-1-14.log",
            "2026-02-30.log",
            "x2026-10-14.log",
            "2026-10-14.logx",
            " 2026-10-14.log",
            "+2026-10-14.log",
            "2026-10-14",
        ] {
            assert_eq!(tag_aus_namen(fremd), None, "{fremd}");
        }
    }
}
