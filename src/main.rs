//! Einstieg des Binärprogramms `localchatA`; die Arbeit liegt in der Bibliothek. Die Laufzeit baut
//! `start::laufzeit` mit dem Rückhalt für Blocking-Threads (ADR-0039 Regel 5).
//!
//! Auf Unix endet das Programm über `_exit` statt über die Rückkehr aus `main`: `exit()` der C-Bibliothek
//! wartet sonst auf einen Faden, der in einem NSS-Modul hängt (ADR-0010 Ergänzung Karte #35, Prozessende).

use localchata::start;
use std::process::ExitCode;

fn main() -> ExitCode {
    let code = match start::laufzeit::laufzeit() {
        Ok(laufzeit) => {
            let code = laufzeit.block_on(start::ausfuehren(std::env::args().skip(1)));
            drop(laufzeit);
            code
        }
        Err(fehler) => {
            eprintln!("localchatA: Laufzeit nicht gestartet — {fehler}");
            start::STARTFEHLER
        }
    };
    beenden(code)
}

/// Nach dem Abbau der Laufzeit: stdout und stderr leeren, dann sofort beenden — ohne `exit()` der C-Bibliothek.
#[cfg(unix)]
fn beenden(code: u8) -> ExitCode {
    use std::io::Write;
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    sofort_beenden(code)
}

/// Ob Windows beim Ende auf einen Faden in `getaddrinfo` wartet, ist ungeprüft: Rückkehr aus `main` wie bisher.
#[cfg(not(unix))]
fn beenden(code: u8) -> ExitCode {
    ExitCode::from(code)
}

/// Die einzige Stelle mit `unsafe` im Paket (ADR-0010 Ergänzung Karte #35, Regel 5).
#[cfg(unix)]
#[allow(unsafe_code)]
fn sofort_beenden(code: u8) -> ! {
    // SAFETY: `_exit` ist async-signal-sicher und hat keine Vorbedingung an Speicher oder Fäden; es beendet alle
    // Fäden, ruft keine `atexit`-Handler und keine Destruktoren. Vorher ist die Laufzeit abgebaut (alles
    // Plattenschreiben aus `spawn_blocking` fertig) und stdout/stderr sind geleert — es geht nichts verloren.
    unsafe { libc::_exit(i32::from(code)) }
}
