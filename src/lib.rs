//! localchatA — ein Programm liefert Weboberfläche und WebAPI für genau einen Ordner aus (ADR-0001).

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "die Route beitreten kommt mit B0049")
)]
mod anwesenheit;
pub mod fehler;
pub mod gespraeche;
mod http;
mod name;
pub mod start;
pub mod uhr;
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "die Anwesenheit zieht Tokens ab B0048")
)]
mod zufall;
