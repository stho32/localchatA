//! Name vor dem `@` (ADR-0013): Erlaubnisliste `[a-z0-9-]`, 1 bis 32 Zeichen, `-` nicht am Rand.
//! Abweisen statt umformen — die Umformung gibt es nur als Vorschlag.

use std::fmt;

/// Die Regel in Worten, für Abweisungen und Hinweise.
pub const NAMENSREGEL: &str =
    "1 bis 32 Zeichen aus a–z, 0–9 und -, nicht mit - am Anfang oder Ende";

/// Ein geprüfter Name; zwei Namen sind gleich, wenn sie Zeichen für Zeichen gleich sind.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Name(String);

/// Der Text verletzt die Namensregel; `vorschlag` ist seine Umformung, wenn sie nicht leer ist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Namensfehler {
    pub regel: &'static str,
    pub vorschlag: Option<Name>,
}

impl Name {
    pub fn bilden(text: &str) -> Result<Name, Namensfehler> {
        if regel_erfuellt(text) {
            return Ok(Name(text.to_string()));
        }
        Err(Namensfehler {
            regel: NAMENSREGEL,
            vorschlag: umformen(text),
        })
    }

    pub fn als_text(&self) -> &str {
        &self.0
    }
}

const LAENGE: usize = 32;

fn regel_erfuellt(text: &str) -> bool {
    let erlaubt = |zeichen: char| zeichen.is_ascii_lowercase() || zeichen.is_ascii_digit();
    let laenge_passt = (1..=LAENGE).contains(&text.len());
    laenge_passt
        && text
            .chars()
            .all(|zeichen| erlaubt(zeichen) || zeichen == '-')
        && !text.starts_with('-')
        && !text.ends_with('-')
}

/// Klein, Umlaute zu `ae/oe/ue/ss`, sonst nicht Erlaubtes zu `-`, Mehrfach-`-` zusammengefasst, Ränder
/// entfernt, auf 32 gekürzt; bleibt nichts übrig, gibt es keinen Vorschlag.
fn umformen(text: &str) -> Option<Name> {
    let mut umgeformt = String::new();
    for zeichen in text.to_lowercase().chars() {
        match zeichen {
            'ä' => umgeformt.push_str("ae"),
            'ö' => umgeformt.push_str("oe"),
            'ü' => umgeformt.push_str("ue"),
            'ß' => umgeformt.push_str("ss"),
            'a'..='z' | '0'..='9' => umgeformt.push(zeichen),
            _ if umgeformt.ends_with('-') => {}
            _ => umgeformt.push('-'),
        }
    }
    let gekuerzt: String = umgeformt.trim_matches('-').chars().take(LAENGE).collect();
    let vorschlag = gekuerzt.trim_end_matches('-');
    if vorschlag.is_empty() {
        return None;
    }
    Some(Name(vorschlag.to_string()))
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vorschlag(text: &str) -> Option<String> {
        match Name::bilden(text) {
            Ok(name) => panic!("{text:?} ist gültig: {name}"),
            Err(fehler) => {
                assert_eq!(fehler.regel, NAMENSREGEL);
                fehler.vorschlag.map(|name| name.als_text().to_string())
            }
        }
    }

    #[test]
    fn gueltige_namen_bleiben_wie_sie_sind() {
        let zweiunddreissig = "a".repeat(32);
        for text in [
            "a",
            "7",
            "a--b",
            "claude-2",
            "claude-x",
            "a-b-c",
            "x1",
            &zweiunddreissig,
        ] {
            assert_eq!(
                Name::bilden(text).map(|name| name.to_string()),
                Ok(text.to_string()),
                "{text}"
            );
        }
    }

    #[test]
    fn ungueltige_namen_werden_mit_der_umformung_als_vorschlag_abgewiesen() {
        let fall = |text: &str, erwartet: &str| {
            assert_eq!(vorschlag(text).as_deref(), Some(erwartet), "{text:?}");
        };
        fall("Jörg", "joerg");
        fall("Claude", "claude");
        fall("ÄÖÜß", "aeoeuess");
        fall("a_b", "a-b");
        fall("a.b", "a-b");
        fall("a b", "a-b");
        fall("a@b", "a-b");
        fall("a/b", "a-b");
        fall("a\\b", "a-b");
        fall("a+b", "a-b");
        fall("-a", "a");
        fall("a-", "a");
        fall("a__..b", "a-b");
        fall("chef\u{200B}", "chef");
        fall("\u{202E}abc", "abc");
        fall("\u{FEFF}abc", "abc");
        fall(&"c".repeat(33), &"c".repeat(32));
        fall(&format!("{}-b", "a".repeat(31)), &"a".repeat(31));
    }

    #[test]
    fn ohne_verwertbares_zeichen_gibt_es_keinen_vorschlag() {
        for text in ["", "___", "-", "  ", "\u{200B}", "…"] {
            assert_eq!(vorschlag(text), None, "{text:?}");
        }
    }

    #[test]
    fn jeder_vorschlag_erfuellt_die_regel() {
        for text in [
            "Jörg",
            "a_b",
            &"Ä".repeat(40),
            &format!("{}_x", "b".repeat(31)),
        ] {
            let vorgeschlagen = vorschlag(text).expect("Vorschlag");
            assert!(
                Name::bilden(&vorgeschlagen).is_ok(),
                "{text:?} → {vorgeschlagen:?}"
            );
        }
    }
}
