//! Formatierer des Betriebsprotokolls (ADR-0040 Regeln 3–5): die einzige Stelle, an der Text und Werte
//! neutralisiert werden und eine Zeile entsteht. Rein, ohne Uhr und Datei.

/// Höchstens so viele Zeichen hat ein Wert im Kontext (ADR-0040 Regel 4).
pub const WERT_HOECHSTENS: usize = 256;
/// Höchstens so viele Zeichen hat der Text eines Eintrags.
pub const TEXT_HOECHSTENS: usize = 1024;

/// Escape (CWE-117) und Kürzung: `\` → `\\`; Steuer-, Zeilentrenner- und Richtungszeichen → `\u{…}` in
/// Kleinbuchstaben-Hex; länger als `hoechstens` Zeichen → gekürzt mit `…` am Ende, nie mitten in einem Escape.
pub fn neutralisieren(roh: &str, hoechstens: usize) -> String {
    let stuecke: Vec<String> = roh.chars().map(stueck).collect();
    let laenge: usize = stuecke.iter().map(|s| s.chars().count()).sum();
    if laenge <= hoechstens {
        return stuecke.concat();
    }
    let mut gekuerzt = String::new();
    let mut zeichen = 0;
    for stueck in &stuecke {
        let dazu = stueck.chars().count();
        if zeichen + dazu > hoechstens.saturating_sub(1) {
            break;
        }
        gekuerzt.push_str(stueck);
        zeichen += dazu;
    }
    gekuerzt.push('…');
    gekuerzt
}

/// Ein Zeichen in seiner Form im Protokoll.
fn stueck(zeichen: char) -> String {
    if zeichen == '\\' {
        "\\\\".to_string()
    } else if zu_escapen(zeichen) {
        format!("\\u{{{:x}}}", u32::from(zeichen))
    } else {
        zeichen.to_string()
    }
}

/// Steuerzeichen (C0, DEL, C1), Zeilentrenner und Richtungszeichen (ADR-0040 Regel 4, „Trojan Source“).
fn zu_escapen(zeichen: char) -> bool {
    matches!(zeichen,
        '\u{0}'..='\u{1f}'
        | '\u{7f}'..='\u{9f}'
        | '\u{2028}'
        | '\u{2029}'
        | '\u{200e}'
        | '\u{200f}'
        | '\u{202a}'..='\u{202e}'
        | '\u{2066}'..='\u{2069}')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steuer_zeilentrenner_und_richtungszeichen_werden_escaped() {
        assert_eq!(
            neutralisieren("Host böse\nZEILE\u{1b}[31m\u{2028}x", WERT_HOECHSTENS),
            "Host böse\\u{a}ZEILE\\u{1b}[31m\\u{2028}x"
        );
        for (roh, erwartet) in [
            ("a\rb", "a\\u{d}b"),
            ("a\u{202e}b", "a\\u{202e}b"),
            ("a\u{2066}b", "a\\u{2066}b"),
            ("a\\b", "a\\\\b"),
            ("\u{0}\u{1f}\u{7f}\u{9f}", "\\u{0}\\u{1f}\\u{7f}\\u{9f}"),
            (
                "\u{2029}\u{200e}\u{200f}\u{202a}\u{2069}",
                "\\u{2029}\\u{200e}\\u{200f}\\u{202a}\\u{2069}",
            ),
        ] {
            assert_eq!(neutralisieren(roh, WERT_HOECHSTENS), erwartet, "{roh:?}");
        }
    }

    #[test]
    fn gewoehnliche_zeichen_und_umlaute_bleiben() {
        for roh in [
            "Anfrage mit fremdem Host abgewiesen",
            "äöüß €",
            "\u{a0}\u{2027}\u{202f}\u{206a}",
        ] {
            assert_eq!(neutralisieren(roh, WERT_HOECHSTENS), roh);
        }
    }

    #[test]
    fn ein_wert_wird_auf_256_zeichen_gekuerzt_mit_auslassung_am_ende() {
        let gekuerzt = neutralisieren(&"a".repeat(300), WERT_HOECHSTENS);
        assert_eq!(gekuerzt, format!("{}…", "a".repeat(255)));
        assert_eq!(gekuerzt.chars().count(), 256);
        let genau = "a".repeat(256);
        assert_eq!(neutralisieren(&genau, WERT_HOECHSTENS), genau);
    }

    #[test]
    fn ein_text_wird_auf_1024_zeichen_gekuerzt() {
        let gekuerzt = neutralisieren(&"ä".repeat(1100), TEXT_HOECHSTENS);
        assert_eq!(gekuerzt.chars().count(), 1024);
        assert!(gekuerzt.ends_with("ä…"), "{gekuerzt}");
    }

    #[test]
    fn gekuerzt_wird_nie_mitten_in_einem_escape() {
        let roh = format!("{}\n\n\n", "a".repeat(250));
        let gekuerzt = neutralisieren(&roh, WERT_HOECHSTENS);
        assert_eq!(gekuerzt, format!("{}\\u{{a}}…", "a".repeat(250)));
        assert!(gekuerzt.chars().count() <= WERT_HOECHSTENS);
    }
}
