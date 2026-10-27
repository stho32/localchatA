//! Gespräche: einziger Schreiber des Ordners (A00006) mit Logformat, Logablage und Verlaufsindex.

mod kennung;
pub mod logablage;
pub mod logformat;

pub use kennung::Kennung;
pub use logablage::{Dateiablage, Gespraech, Logablage};
