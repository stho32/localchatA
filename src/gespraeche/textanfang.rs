//! Textanfang eines Zitats (ADR-0053 F4): Leerraumfolgen (auch Zeilenumbrüche) werden ein Leerzeichen, der Rand
//! fällt weg; über 120 Zeichen (Unicode-Skalare) bleiben die ersten 119 ohne Rand und „…“. Markdown bleibt Quelltext.
//! Rein.

/// So viele Zeichen hat ein Textanfang höchstens, „…“ mitgezählt.
pub const TEXTANFANG_ZEICHEN: usize = 120;

/// Der Textanfang eines Texts.
pub fn textanfang(text: &str) -> String {
    textanfang_aus(text.chars())
}

/// Der Textanfang aus den Zeichen eines Texts; liest nur so weit, bis feststeht, ob gekürzt wird.
pub fn textanfang_aus(zeichen: impl Iterator<Item = char>) -> String {
    let mut anfang = String::new();
    let mut anzahl = 0;
    let mut leerraum_davor = false;
    for zeichen in zeichen {
        if zeichen.is_whitespace() {
            leerraum_davor = anzahl > 0;
            continue;
        }
        let dazu = if leerraum_davor { 2 } else { 1 };
        if anzahl + dazu > TEXTANFANG_ZEICHEN {
            return gekuerzt(&anfang);
        }
        if leerraum_davor {
            anfang.push(' ');
            leerraum_davor = false;
        }
        anfang.push(zeichen);
        anzahl += dazu;
    }
    anfang
}

/// Die ersten 119 Zeichen ohne Rand und „…“ — der Text hat mehr als 120.
fn gekuerzt(anfang: &str) -> String {
    let mut gekuerzt: String = anfang.chars().take(TEXTANFANG_ZEICHEN - 1).collect();
    gekuerzt.truncate(gekuerzt.trim_end().len());
    gekuerzt.push('…');
    gekuerzt
}

#[cfg(test)]
mod tests {
    use super::*;

    /// R00047 B0705, ADR-0053 P2: die sechs Fälle der Probe.
    #[test]
    fn der_textanfang_fasst_leerraum_zusammen_und_kuerzt_nach_zeichen() {
        let x119 = "x".repeat(119);
        for (text, erwartet) in [
            ("Frage?".to_string(), "Frage?".to_string()),
            ("a\n\n  b\tc ".to_string(), "a b c".to_string()),
            ("x".repeat(300), format!("{x119}…")),
            (format!("{x119} yyy"), format!("{x119}…")),
            ("ä".repeat(200), format!("{}…", "ä".repeat(119))),
            (String::new(), String::new()),
        ] {
            let anfang = textanfang(&text);

            assert_eq!(anfang, erwartet, "{text:?}");
            assert!(anfang.chars().count() <= TEXTANFANG_ZEICHEN);
        }
    }

    /// B0705 Grenzen: genau 120 Zeichen bleiben ungekürzt, 121 werden gekürzt; Rand vorn fällt weg; eine Kürzung an
    /// einem Leerraum endet ohne Leerzeichen vor „…“, auch wenn der Leerraum mitten in den 119 Zeichen liegt.
    #[test]
    fn genau_120_zeichen_bleiben_121_werden_gekuerzt() {
        let x120 = "x".repeat(120);
        assert_eq!(textanfang(&x120), x120);
        assert_eq!(textanfang(&format!("  \n{x120}\n\n")), x120);
        assert_eq!(
            textanfang(&"x".repeat(121)),
            format!("{}…", "x".repeat(119))
        );
        let x118 = "x".repeat(118);
        let gekuerzt = format!("{x118}…");
        assert_eq!(
            textanfang(&format!("{x118}\n\n\nzzz{}", "y".repeat(10))),
            gekuerzt
        );
        assert_eq!(textanfang(&format!("{x118} yyyyy")), gekuerzt);
        assert_eq!(
            textanfang(&format!("a  b{}", "c".repeat(200))),
            format!("a b{}…", "c".repeat(116))
        );
    }
}
