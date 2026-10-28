//! Einstieg des Binärprogramms `localchatA`; die Arbeit liegt in der Bibliothek. Die Laufzeit baut
//! `start::laufzeit` mit dem Rückhalt für Blocking-Threads (ADR-0039 Regel 5).

use std::process::ExitCode;

fn main() -> ExitCode {
    match localchata::start::laufzeit::laufzeit() {
        Ok(laufzeit) => laufzeit.block_on(localchata::start::ausfuehren(std::env::args().skip(1))),
        Err(fehler) => {
            eprintln!("localchatA: Laufzeit nicht gestartet — {fehler}");
            ExitCode::FAILURE
        }
    }
}
