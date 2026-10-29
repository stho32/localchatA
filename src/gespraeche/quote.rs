//! Plattenquote der Gespräche (ADR-0044): die Grenze in Bytes, die Gespräche hält (A00006 Ergänzung Karte #34).

/// Standard 4 GiB (ADR-0044 Regel 3): Normalwoche ≈ 1,6 GB und Bestand des Lastprofils ≈ 2,65 GB liegen unter der
/// Warnschwelle von 3,6 GiB; eine flutende Herkunft braucht vom Normalstand ≈ 11 h bis hierher. Gerechnet, nicht
/// gemessen — anheben ist ein Commit.
pub const STANDARD_BYTES: u64 = 4 * 1024 * 1024 * 1024;
