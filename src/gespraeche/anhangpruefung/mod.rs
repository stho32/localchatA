//! Anhangprüfung (A00018): aus Anzeigename und Inhalt einer hochgeladenen Datei folgt, unter welchem Namen sie
//! gespeichert und als welche Art sie ausgeliefert wird. Rein — keine Platte, keine Uhr.

mod art;
mod avif;
pub mod bild;
pub mod erkenner;
pub mod exif;
pub mod medien;
mod name;
pub mod text;
pub mod tilger;

pub use art::{Art, DATEI, inline};
pub use name::{Namensfehler, name_pruefen};
