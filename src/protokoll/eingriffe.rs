//! Eingriffszeilen (ADR-0049 Regel 2): Zeilen, die gemeinsame oder fremde Daten ändern oder löschen, zählen in die
//! eigene Budgetklasse `EINGRIFF`. Rein; die eine Stelle der Zuordnung beim Schreiben und beim Wiederlesen.

use super::Stufe;

/// Anfang des Textes der Zeile „Anhänge gelöscht“ (ADR-0042, Ergänzung Karte #9); wer sie schreibt, nimmt den Text
/// von hier.
pub const ANHAENGE_GELOESCHT: &str = "Anhänge gelöscht";

/// Ist die Zeile mit Stufe, Bereichsname und Text eine Eingriffszeile? `INFORMATION` mit Bereich `frist`, oder
/// Bereich `anhang` mit Text, der mit [`ANHAENGE_GELOESCHT`] beginnt; alles andere nicht.
pub fn ist_eingriff(stufe: Stufe, bereich: &str, text: &str) -> bool {
    stufe == Stufe::Information
        && match bereich {
            "frist" => true,
            "anhang" => text.starts_with(ANHAENGE_GELOESCHT),
            _ => false,
        }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// R00034 F0125 (ADR-0049, Regel 2): die Fälle wörtlich aus der ADR.
    #[test]
    fn eingriff_ist_information_frist_oder_geloeschte_anhaenge() {
        let faelle = [
            (
                Stufe::Information,
                "frist",
                "Frist 7 → 30 Tage gesetzt",
                true,
            ),
            (Stufe::Information, "frist", "2 Tage entfallen (…)", true),
            (Stufe::Information, "anhang", "Anhänge gelöscht", true),
            (
                Stufe::Information,
                "anhang",
                "Großes Hochladen abgeschlossen",
                false,
            ),
            (Stufe::Error, "frist", "Tag nicht entfallen", false),
            (Stufe::Information, "sitzung", "Sitzung beigetreten", false),
            (Stufe::Warning, "frist", "Frist 7 → 30 Tage gesetzt", false),
            (Stufe::Debug, "frist", "Frist 7 → 30 Tage gesetzt", false),
            (Stufe::Exception, "anhang", "Anhänge gelöscht", false),
        ];
        for (stufe, bereich, text, erwartet) in faelle {
            assert_eq!(
                ist_eingriff(stufe, bereich, text),
                erwartet,
                "{stufe:?} {bereich} {text}"
            );
        }
    }

    #[test]
    fn die_konstante_lautet_woertlich_anhaenge_geloescht() {
        assert_eq!(ANHAENGE_GELOESCHT, "Anhänge gelöscht");
    }
}
