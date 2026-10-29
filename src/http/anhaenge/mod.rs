//! Anhänge über die API (R00029 F0097, ADR-0046): anmelden (`POST /api/anhaenge`), hochladen (`PUT
//! /api/anhaenge/{id}`), verwerfen (`DELETE /api/anhaenge/{id}`).

mod anmelden;

pub use anmelden::anmelden;
