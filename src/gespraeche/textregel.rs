//! Textregel einer Nachricht (B0206, ADR-0034 Regel 3.4–3.6): abgewiesen, nie umgeformt. Rein.
//!
//! Reihenfolge: leer (nur Leerraum nach `char::is_whitespace`) → zu lang (Bytes UTF-8, wie der Text ankommt) →
//! verbotenes Zeichen (das erste, Stelle in Zeichen ab 0).

/// Höchstens so viele Bytes UTF-8 hat der Text einer Nachricht (ADR-0034 Regel 3.5).
pub const TEXT_HOECHSTENS_BYTES: usize = 16_384;

/// Warum ein Text abgewiesen wird.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Textfehler {
    Leer,
    Laenge { bytes: usize },
    Zeichen { stelle: usize, zeichen: String },
}

pub fn text_pruefen(text: &str) -> Result<(), Textfehler> {
    if text.chars().all(char::is_whitespace) {
        return Err(Textfehler::Leer);
    }
    if text.len() > TEXT_HOECHSTENS_BYTES {
        return Err(Textfehler::Laenge { bytes: text.len() });
    }
    match erstes_verbotenes(text) {
        Some((stelle, zeichen)) => Err(Textfehler::Zeichen {
            stelle,
            zeichen: format!("U+{:04X}", u32::from(zeichen)),
        }),
        None => Ok(()),
    }
}

/// Stelle (in Zeichen ab 0) und Zeichen des ersten verbotenen Zeichens: C0 außer Tab und Zeilenumbruch, DEL, C1;
/// ein Wagenrücklauf nur unmittelbar vor einem Zeilenumbruch.
fn erstes_verbotenes(text: &str) -> Option<(usize, char)> {
    let mut zeichen = text.chars().enumerate().peekable();
    while let Some((stelle, einzeln)) = zeichen.next() {
        let wagenruecklauf_vor_umbruch = einzeln == '\r'
            && zeichen
                .peek()
                .is_some_and(|(_, naechstes)| *naechstes == '\n');
        let erlaubt = matches!(einzeln, '\t' | '\n') || wagenruecklauf_vor_umbruch;
        if einzeln.is_control() && !erlaubt {
            return Some((stelle, einzeln));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zeichen(stelle: usize, zeichen: &str) -> Result<(), Textfehler> {
        Err(Textfehler::Zeichen {
            stelle,
            zeichen: zeichen.to_string(),
        })
    }

    /// R00011 F0041 „Textregel“: Leerraum jeder Art ist leer.
    #[test]
    fn nur_leerraum_ist_leer() {
        for text in ["", " \n\t", "\u{a0}"] {
            assert_eq!(text_pruefen(text), Err(Textfehler::Leer), "{text:?}");
        }
    }

    /// Spanne in Bytes: 16 384 ok, 16 385 zu lang; mit `€` (3 Bytes) zählen Bytes, nicht Zeichen.
    #[test]
    fn die_grenze_liegt_bei_16_384_bytes_utf8() {
        assert_eq!(text_pruefen(&"a".repeat(16_384)), Ok(()));
        assert_eq!(
            text_pruefen(&"a".repeat(16_385)),
            Err(Textfehler::Laenge { bytes: 16_385 })
        );
        assert_eq!(text_pruefen(&format!("{}a", "€".repeat(5_461))), Ok(()));
        assert_eq!(
            text_pruefen(&"€".repeat(5_462)),
            Err(Textfehler::Laenge { bytes: 16_386 })
        );
    }

    #[test]
    fn leer_geht_vor_laenge() {
        assert_eq!(text_pruefen(&" ".repeat(16_385)), Err(Textfehler::Leer));
    }

    /// Je eine Vertreterin jeder verbotenen Klasse hinter `ab` → Stelle 2 und die Schreibweise.
    #[test]
    fn jede_verbotene_klasse_wird_mit_stelle_und_schreibweise_abgewiesen() {
        for (zeichen_selbst, schreibweise) in [
            ('\0', "U+0000"),
            ('\x08', "U+0008"),
            ('\x0b', "U+000B"),
            ('\x0c', "U+000C"),
            ('\x1b', "U+001B"),
            ('\x7f', "U+007F"),
            ('\u{85}', "U+0085"),
            ('\u{9f}', "U+009F"),
        ] {
            assert_eq!(
                text_pruefen(&format!("ab{zeichen_selbst}")),
                zeichen(2, schreibweise),
                "{schreibweise}"
            );
        }
    }

    #[test]
    fn ein_wagenruecklauf_ist_nur_vor_einem_zeilenumbruch_erlaubt() {
        assert_eq!(text_pruefen("a\rb"), zeichen(1, "U+000D"));
        assert_eq!(text_pruefen("ab\r"), zeichen(2, "U+000D"));
        assert_eq!(text_pruefen("a\r\nb"), Ok(()));
    }

    /// Das Kriterium nennt `"\t"` unter „ok“; allein ist ein Tab aber nur Leerraum und damit `leer` (dieselbe
    /// Anforderung: `" \n\t"` → `leer`, Leer vor Zeichen). Geprüft wird die Absicht: Tab ist kein verbotenes Zeichen.
    #[test]
    fn tab_emoji_verbinder_und_richtungszeichen_sind_erlaubt() {
        for text in ["a\tb", "👩‍💻", "\u{202e}abc"] {
            assert_eq!(text_pruefen(text), Ok(()), "{text:?}");
        }
        assert_eq!(text_pruefen("\t"), Err(Textfehler::Leer));
    }

    /// Die Stelle zählt Zeichen, nicht Bytes: zwei `€` davor sind Stelle 2.
    #[test]
    fn die_stelle_zaehlt_zeichen_nicht_bytes() {
        assert_eq!(text_pruefen("€€\x1b"), zeichen(2, "U+001B"));
    }

    /// R00011 „Steuerzeichen und JSON-Ausdehnung“: Klassengrenzen als Spanne, je hinter `ab`.
    #[test]
    fn klassengrenzen_als_spanne() {
        for (zeichen_selbst, erwartet) in [
            ('\u{08}', zeichen(2, "U+0008")),
            ('\u{09}', Ok(())),
            ('\u{0a}', Ok(())),
            ('\u{0b}', zeichen(2, "U+000B")),
            ('\u{1f}', zeichen(2, "U+001F")),
            ('\u{20}', Ok(())),
            ('\u{7e}', Ok(())),
            ('\u{7f}', zeichen(2, "U+007F")),
            ('\u{9f}', zeichen(2, "U+009F")),
            ('\u{a0}', Ok(())),
        ] {
            assert_eq!(
                text_pruefen(&format!("ab{zeichen_selbst}")),
                erwartet,
                "U+{:04X}",
                zeichen_selbst as u32
            );
        }
    }

    /// Ein angenommener Text wird als JSON höchstens doppelt so lang (+ 2 Anführungszeichen): Tab, `"`, `\` und
    /// Zeilenumbruch haben Kurzformen mit 2 Bytes; Steuerzeichen ohne Kurzform (6 Bytes) kommen nie durch.
    #[test]
    fn die_json_ausdehnung_eines_angenommenen_texts_ist_hoechstens_doppelt() {
        for zeichen_selbst in ['\t', '"', '\\', '\n'] {
            let text = format!("a{}", zeichen_selbst.to_string().repeat(16_383));

            assert_eq!(text_pruefen(&text), Ok(()), "{zeichen_selbst:?}");
            let json = serde_json::to_vec(&text).expect("JSON");
            assert!(
                json.len() <= 2 * TEXT_HOECHSTENS_BYTES + 2,
                "{zeichen_selbst:?}: {} Bytes",
                json.len()
            );
        }
    }
}
