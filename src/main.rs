//! Einstieg des Binärprogramms `localchatA`; die Arbeit liegt in der Bibliothek.

use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    localchata::start::ausfuehren(std::env::args().skip(1)).await
}
