//! localchatA — ein Programm liefert Weboberfläche und WebAPI für genau einen Ordner aus (ADR-0001).

mod anwesenheit;
pub mod fehler;
pub mod gespraeche;
mod http;
mod live;
mod name;
pub mod protokoll;
#[allow(dead_code)] // bis B0233 das Senden bucht
mod schreibkonto;
pub mod start;
pub mod testdaten;
pub mod uhr;
mod zufall;
