//! Anhänge über die API (R00029 F0097, ADR-0046): anmelden (`POST /api/anhaenge`), hochladen (`PUT
//! /api/anhaenge/{id}`), verwerfen (`DELETE /api/anhaenge/{id}`).

mod anmelden;
mod ausliefern;
pub mod bereich;
pub mod hochladen;
mod kopfzeilen;
mod strom;
mod verwerfen;

pub use anmelden::anmelden;
pub use ausliefern::ausliefern;
pub use hochladen::hochladen;
pub use verwerfen::verwerfen;
