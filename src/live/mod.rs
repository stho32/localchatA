//! Live-Kanal (A00011): Ereignisse an alle offenen Ströme; ab I0005 die Anwesenheit.

pub mod bus;
pub mod herzschlag;
pub mod takt;

/// So viele Ereignisse darf ein Strom zurückliegen, bevor er einen neuen Stand bekommt.
pub const KAPAZITAET: usize = 1024;
