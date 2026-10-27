//! Anwesenheit (A00010): wer unter welchem Namen beigetreten ist — der einzige Schreiber aller Sitzungen.

mod bestand;
pub mod gnadenfrist;
mod vorschlag;
mod wunsch;

pub use bestand::{Anwesenheit, Beitrittsfehler, GESAMT, JE_HERKUNFT, Sitzung};
pub use wunsch::{Abweisung, Art, Beitrittswunsch};
