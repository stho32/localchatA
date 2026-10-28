//! Anwesenheit (A00010): wer unter welchem Namen beigetreten ist — der einzige Schreiber aller Sitzungen.

mod bestand;
pub mod gnadenfrist;
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "Brücke bis B0076: der Bestand nutzt den Status")
)]
mod status;
mod vorschlag;
mod wunsch;

pub use bestand::{Anwesenheit, Beitrittsfehler, GESAMT, JE_HERKUNFT, Sitzung};
pub use wunsch::{Abweisung, Art, Beitrittswunsch};
