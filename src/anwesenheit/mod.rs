//! Anwesenheit (A00010): wer unter welchem Namen beigetreten ist — der einzige Schreiber aller Sitzungen.

mod bestand;
pub mod gnadenfrist;
mod stand;
pub(crate) mod status;
mod vorschlag;
mod wunsch;

pub use bestand::{Anwesenheit, Beitrittsfehler, GESAMT, JE_HERKUNFT, Sitzung};
pub use stand::{Stand, Teilnehmerzeile, Wechsel};
pub use wunsch::{Abweisung, Art, Beitrittswunsch};
