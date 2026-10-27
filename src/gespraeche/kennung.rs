//! Kennung `name@herkunft` des Absenders — geprüft beim Bilden, damit das Logformat sich darauf verlassen kann.

use crate::fehler::Fehler;
use std::fmt;

/// Nicht leer, ohne Leer- und Steuerzeichen, genau ein `@` zwischen Name und Herkunft.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Kennung(String);

impl Kennung {
    pub fn bilden(text: &str) -> Result<Kennung, Fehler> {
        let ohne_trenner = !text
            .chars()
            .any(|zeichen| zeichen.is_whitespace() || zeichen.is_control());
        let name_und_herkunft = match text.split_once('@') {
            Some((name, herkunft)) => {
                !name.is_empty() && !herkunft.is_empty() && !herkunft.contains('@')
            }
            None => false,
        };
        if ohne_trenner && name_und_herkunft {
            Ok(Kennung(text.to_string()))
        } else {
            Err(Fehler::Ungueltig(format!(
                "Kennung {text:?} ungültig — erwartet name@herkunft ohne Leer- und Steuerzeichen"
            )))
        }
    }

    pub fn als_text(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Kennung {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_at_herkunft_ist_eine_kennung_auch_mit_ipv6() {
        for text in [
            "claude@10.0.0.5",
            "claude@::1",
            "shoff@127.0.0.1",
            "ä@fe80::1",
        ] {
            let kennung = Kennung::bilden(text).expect(text);
            assert_eq!(kennung.als_text(), text);
            assert_eq!(kennung.to_string(), text);
        }
    }

    #[test]
    fn leer_leerzeichen_steuerzeichen_und_falsche_at_zahl_sind_ungueltig() {
        for text in [
            "",
            "cla ude@x",
            "claude",
            "a@b@c",
            "@herkunft",
            "name@",
            "a\t@b",
            "a@b\n",
            "a@b\r",
            "a\u{0}@b",
            "a\u{a0}b@c",
        ] {
            match Kennung::bilden(text) {
                Err(Fehler::Ungueltig(meldung)) => {
                    assert!(meldung.contains("Kennung"), "{meldung}")
                }
                anderes => panic!("{text:?} ergab {anderes:?}"),
            }
        }
    }
}
