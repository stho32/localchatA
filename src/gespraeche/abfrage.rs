//! Abfrage des Verlaufs (ADR-0025 Regel 2): welcher Abschnitt und wie viele Nachrichten — ein Wert, rein.

use crate::uhr::Zeit;

/// Welcher Teil des Verlaufs: die neuesten, die jüngsten echt vor einer Zeit oder die ältesten echt nach ihr.
/// Die Zeit muss zu keiner Nachricht gehören und wird als Zeitpunkt verglichen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Abschnitt {
    Neueste,
    Vor(Zeit),
    Nach(Zeit),
}

/// Höchstens so viele Nachrichten je Block — begrenzt die Arbeit und die Antwort (ADR-0025 Regel 2).
pub const BLOCK_HOECHSTENS: usize = 200;
/// Ohne `n` ein Block des Designsystems.
pub const BLOCK_STANDARD: usize = 100;

/// Eine Blockgröße von 1 bis `BLOCK_HOECHSTENS`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Blockgroesse(usize);

impl Blockgroesse {
    pub fn bilden(n: usize) -> Option<Blockgroesse> {
        if (1..=BLOCK_HOECHSTENS).contains(&n) {
            Some(Blockgroesse(n))
        } else {
            None
        }
    }

    pub fn standard() -> Blockgroesse {
        Blockgroesse(BLOCK_STANDARD)
    }

    pub fn wert(self) -> usize {
        self.0
    }
}

/// Eine geprüfte Abfrage: Abschnitt und Blockgröße.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Abfrage {
    pub abschnitt: Abschnitt,
    pub n: Blockgroesse,
}
