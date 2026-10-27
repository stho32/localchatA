//! localchatA — ein Programm liefert Weboberfläche und WebAPI für genau einen Ordner aus (ADR-0001).

pub mod fehler;
pub mod gespraeche;
mod http;
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "Kennung nutzt den Namen ab B0042")
)]
mod name;
pub mod start;
pub mod uhr;
