//! Anhangprüfung (A00018): aus Anzeigename und Inhalt einer hochgeladenen Datei folgt, unter welchem Namen sie
//! gespeichert und als welche Art sie ausgeliefert wird. Rein — keine Platte, keine Uhr.

mod art;
pub mod bild;
pub mod exif;
mod name;

pub use art::{Art, DATEI};
pub use name::{Namensfehler, name_pruefen};
