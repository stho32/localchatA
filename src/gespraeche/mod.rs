//! Gespräche: einziger Schreiber des Ordners (A00006) mit Logformat, Logablage und Verlaufsindex.

pub mod frist;
mod kennung;
pub mod logablage;
pub mod logformat;
pub mod verlaufsindex;

pub use kennung::Kennung;
pub use logablage::{Dateiablage, Gespraech, Logablage};
