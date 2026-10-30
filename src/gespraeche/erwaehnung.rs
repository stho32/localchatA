//! Erwähnungsregel (ADR-0054 Regel 6, B0741): wen ein Text mit `@name` erwähnt. Rein; geteilt mit I0013.
//!
//! Eine Erwähnung ist `@`, davor Textanfang oder ein Zeichen außerhalb `[A-Za-z0-9-]`, danach die längste Folge aus
//! `[A-Za-z0-9-]`, in ASCII-Kleinbuchstaben, ohne `-` am Ende. Sie erwähnt den Namen, der ihr Zeichen für Zeichen
//! gleicht. Es gibt keinen Markdown-Leser: auch in `` `@claude` `` zählt sie.

use crate::name::Name;

/// Die erwähnten Namen in der Folge des Textes, in ASCII-Kleinbuchstaben; Doppel bleiben stehen.
pub fn erwaehnte_namen(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    text.match_indices('@')
        .filter(|(stelle, _)| *stelle == 0 || !im_namen(bytes[stelle - 1]))
        .filter_map(|(stelle, _)| {
            let folge: String = bytes[stelle + 1..]
                .iter()
                .take_while(|byte| im_namen(**byte))
                .map(|byte| char::from(byte.to_ascii_lowercase()))
                .collect();
            let name = folge.trim_end_matches('-');
            (!name.is_empty()).then(|| name.to_string())
        })
        .collect()
}

/// `[A-Za-z0-9-]` — ASCII-Bytes kommen in UTF-8 nie innerhalb eines anderen Zeichens vor, darum genügt der Byte-Blick.
fn im_namen(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'-'
}

/// Ob der Text den Namen erwähnt.
pub fn erwaehnt(text: &str, name: &Name) -> bool {
    erwaehnte_namen(text)
        .iter()
        .any(|erwaehnt| erwaehnt == name.als_text())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name(text: &str) -> Name {
        Name::bilden(text).expect("Testname")
    }

    /// W1 (B0741): die Tabelle der ADR — erwähnt `claude`.
    #[test]
    fn diese_texte_erwaehnen_claude() {
        for text in [
            "@claude hi",
            "hi @Claude.",
            "@claude@127.0.0.1",
            "`@claude`",
            "@claude-",
        ] {
            assert!(erwaehnt(text, &name("claude")), "{text:?} erwähnt claude");
        }
    }

    /// W1 (B0741): erwähnt `claude` nicht — E-Mail-Form, längere Folge, Leerzeichen, anderer Name, Sammelname.
    #[test]
    fn diese_texte_erwaehnen_claude_nicht() {
        for text in [
            "shoff@claude",
            "@claude-entwickler",
            "@ claude",
            "@claudex",
            "@alle",
        ] {
            assert!(
                !erwaehnt(text, &name("claude")),
                "{text:?} erwähnt claude nicht"
            );
        }
    }

    /// W1 (B0741): die längste Folge ist der Name — `@claude-entwickler` erwähnt `claude-entwickler`.
    #[test]
    fn die_laengste_folge_ist_der_erwaehnte_name() {
        assert!(erwaehnt("@claude-entwickler", &name("claude-entwickler")));
        assert_eq!(
            erwaehnte_namen("@claude-entwickler"),
            vec!["claude-entwickler"]
        );
    }

    /// Die Namen in Textfolge, klein, ohne `-` am Ende; ein `@` mitten in einer Folge beginnt keine Erwähnung.
    #[test]
    fn die_namen_kommen_in_textfolge_klein_und_ohne_strich_am_ende() {
        assert_eq!(
            erwaehnte_namen("@Mia und (@ben--), shoff@claude, @codex@127.0.0.1 @"),
            vec!["mia", "ben", "codex"]
        );
        assert_eq!(erwaehnte_namen("ü@anna"), vec!["anna"]);
        assert!(erwaehnte_namen("@- @@ @").is_empty());
    }
}
