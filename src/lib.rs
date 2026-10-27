//! localchatA — ein Programm liefert Weboberfläche und WebAPI für genau einen Ordner aus (ADR-0001).

pub mod fehler;
pub mod gespraeche;
mod http;
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "Regel und Vorschlag nutzt das Beitreten ab B0046")
)]
mod name;
pub mod start;
pub mod uhr;
