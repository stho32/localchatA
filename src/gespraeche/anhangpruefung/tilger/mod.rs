//! Ortsangaben beim Hochladen tilgen (F0115, A00018 Ergänzung F4, ADR-0047 Ergänzung „Karte #9 Betreiber F4“): eigene
//! Leser nach Struktur, an Ort und Stelle bei gleicher Länge, über den Positionsport.

pub mod bild;
pub mod isobmff;
pub mod minimal;
pub mod port;

use port::{Abbruch, Ergebnis};

/// Höchstens so viele gedeutete Blöcke je Datei (Unterblöcke eines GIF und `IDAT`/`fdAT` zählen nie).
pub const HOECHSTENS_BLOECKE: u64 = 100_000;

/// Was der Tilger gedeutet hat: Zahl der Blöcke und ob ein Ort (GPS-IFD, Ortsbox) dabei war.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Zaehler {
    pub bloecke: u64,
    pub ort: bool,
}

impl Zaehler {
    /// Ein gedeuteter Block mehr; über 100.000 → `metadaten`.
    pub fn block(&mut self) -> Ergebnis<()> {
        self.bloecke += 1;
        if self.bloecke > HOECHSTENS_BLOECKE {
            return Err(Abbruch::Metadaten("über 100.000 Blöcke"));
        }
        Ok(())
    }
}
