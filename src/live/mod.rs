//! Live-Kanal (A00011): Ereignisse an alle offenen Ströme; ab I0005 die Anwesenheit.

pub mod bus;
pub mod herzschlag;
pub mod takt;
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "die Route verdrahtet B0739 (R00049)")
)]
pub mod warten;

/// So viele Ereignisse darf ein Strom zurückliegen, bevor er einen neuen Stand bekommt.
pub const KAPAZITAET: usize = 1024;

/// So viele Signale darf ein Strom zurückliegen, bevor er eine Lücke meldet (ADR-0051 Regel 6) — ein eigener Puffer
/// neben dem der Ereignisse.
pub const SIGNAL_KAPAZITAET: usize = 1024;
