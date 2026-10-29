//! Anhänge über die API (R00029 F0097, ADR-0046): anmelden (`POST /api/anhaenge`), hochladen (`PUT
//! /api/anhaenge/{id}`), verwerfen (`DELETE /api/anhaenge/{id}`).

mod anmelden;
pub mod hochladen;
mod verwerfen;

pub use anmelden::anmelden;
pub use hochladen::hochladen;
pub use verwerfen::verwerfen;
