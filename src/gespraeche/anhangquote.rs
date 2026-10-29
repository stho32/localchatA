//! Anhang-Quote (ADR-0046 Ergänzungen „Karte #9 Betreiber F1“ Frage D, „F1′“ Regeln 1, 2): eigene Quote für alle
//! Anhänge zusammen, getrennt von der Plattenquote des Textes. Rein.

/// Mindestmaß eines Anhangs an der Quote.
pub const MINDESTMASS: u64 = 65_536;
/// Blockgröße des Zählmaßes.
pub const BLOCK: u64 = 4_096;

/// Zählmaß `z(n) = max(65.536, ⌈n / 4.096⌉ × 4.096)` — so zählt ein Anhang an der Anhang-Quote.
pub fn zaehlmass(bytes: u64) -> u64 {
    bytes.div_ceil(BLOCK).saturating_mul(BLOCK).max(MINDESTMASS)
}
