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
    erwaehnt_name(text, name.als_text())
}

/// Ob der Text den Namen `name` (in ASCII-Kleinbuchstaben) erwähnt — dieselbe Regel wie `erwaehnte_namen`, aber
/// ohne Allokation und mit frühem Abbruch (Nacharbeit security-reviewer #17 Befund 2): an jedem `@`, das eine
/// Erwähnung beginnt, gleicht die Folge dem Namen, wenn sie mit ihm beginnt (Groß/Klein egal) und danach nur noch `-`
/// folgen. Die Folgen zweier Erwähnungen überlappen nie, denn vor einem `@` darf kein Zeichen der Folge stehen.
pub fn erwaehnt_name(text: &str, name: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.chunks(STUECK).enumerate().any(|(nummer, stueck)| {
        enthaelt_klammeraffen(stueck)
            && erwaehnt_im_stueck(bytes, nummer * STUECK, stueck, name.as_bytes())
    })
}

/// So viele Bytes prüft `erwaehnt_name` am Stück auf ein `@`, bevor es Byte für Byte sucht — ein Text ohne `@` kostet
/// so kaum mehr als ein Lesen (Gegenprobe des Befunds: 16 KiB ≤ 5 µs).
const STUECK: usize = 64;

/// Ob ein `@` im Stück steht — ohne frühen Abbruch, damit der Übersetzer den Vergleich bündeln kann.
fn enthaelt_klammeraffen(stueck: &[u8]) -> bool {
    stueck
        .iter()
        .fold(false, |gefunden, byte| gefunden | (*byte == b'@'))
}

/// Ob eine Erwähnung, die im `stueck` ab `anfang` von `bytes` beginnt, `name` ist; je `@` zuerst nur der Blick auf
/// das Zeichen danach.
fn erwaehnt_im_stueck(bytes: &[u8], anfang: usize, stueck: &[u8], name: &[u8]) -> bool {
    let erstes = name.first().copied();
    stueck.iter().enumerate().any(|(versatz, byte)| {
        let stelle = anfang + versatz;
        *byte == b'@'
            && bytes.get(stelle + 1).map(u8::to_ascii_lowercase) == erstes
            && (stelle == 0 || !im_namen(bytes[stelle - 1]))
            && folge_gleicht(&bytes[stelle + 1..], name)
    })
}

/// Ob die Folge am Anfang von `rest` ohne `-` am Ende `name` ist; bricht am ersten abweichenden Zeichen ab.
fn folge_gleicht(rest: &[u8], name: &[u8]) -> bool {
    let Some((anfang, danach)) = rest.split_at_checked(name.len()) else {
        return false;
    };
    !name.is_empty()
        && anfang
            .iter()
            .zip(name)
            .all(|(zeichen, gesucht)| zeichen.to_ascii_lowercase() == *gesucht)
        && danach
            .iter()
            .take_while(|byte| im_namen(**byte))
            .all(|byte| *byte == b'-')
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

    /// Nacharbeit security-reviewer #17 Befund 2: die Prüfung je Name sagt für jeden Text dasselbe wie die Liste der
    /// erwähnten Namen — Groß/Klein, Striche am Ende, `@` am Ende, Doppel-`@`, Folge länger als der Name, Nicht-ASCII
    /// davor und danach, erwähnungsdicht.
    #[test]
    fn die_pruefung_je_name_gleicht_der_liste_der_namen() {
        let dicht = "@a ".repeat(5_461);
        let texte = [
            "@claude hi",
            "hi @Claude.",
            "@CLAUDE--- x",
            "@claude-entwickler",
            "@claudex @claude",
            "shoff@claude",
            "@ claude",
            "@@claude",
            "x@claude @",
            "ü@claude",
            "@claudeü",
            "@-claude",
            "@cla",
            "",
            "@",
            dicht.as_str(),
        ];
        for text in texte {
            for gesucht in ["claude", "a", "claude-entwickler", "cla"] {
                assert_eq!(
                    erwaehnt_name(text, gesucht),
                    erwaehnte_namen(text).iter().any(|name| name == gesucht),
                    "{text:?} / {gesucht}"
                );
            }
        }
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
