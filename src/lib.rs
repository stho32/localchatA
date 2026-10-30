//! localchatA — ein Programm liefert Weboberfläche und WebAPI für genau einen Ordner aus (ADR-0001).

#![forbid(unsafe_code)]

mod anwesenheit;
#[cfg(test)]
mod episodenkatalog;
pub mod fehler;
pub mod gespraeche;
mod http;
mod live;
mod name;
pub mod protokoll;
mod schreibkonto;
pub mod start;
pub mod testdaten;
pub mod uhr;
mod zufall;
