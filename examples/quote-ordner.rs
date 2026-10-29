//! Testdaten für R00023: `cargo run --example quote-ordner -- <ordner> <channel> <bytes>` legt
//! `<ordner>/channels/<channel>/` mit Frist 7 und einer Tagesdatei von gestern (nach `LOCALCHATA_JETZT` bzw.
//! Systemuhr und `TZ`) mit genau `<bytes>` Bytes an — „der Quote-Ordner“ mit 1048276.

use chrono::{DateTime, Local, Utc};
use localchata::testdaten::quote_ordner;
use localchata::uhr::{self, Zeit};
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let argumente: Vec<String> = std::env::args().skip(1).collect();
    let [ordner, channel, bytes] = argumente.as_slice() else {
        eprintln!("Aufruf: quote-ordner <ordner> <channel> <bytes>");
        return ExitCode::from(2);
    };
    let Ok(bytes) = bytes.parse::<u64>() else {
        eprintln!("<bytes> ist eine ganze Zahl");
        return ExitCode::from(2);
    };
    let startzeit = std::env::var_os(uhr::STARTZEIT_VARIABLE);
    let jetzt = match uhr::startzeit_lesen(startzeit.as_deref()).and_then(uhr::aus_umgebung) {
        Ok(uhr) => uhr.jetzt(),
        Err(fehler) => {
            eprintln!("{fehler}");
            return ExitCode::from(2);
        }
    };
    let ordner = PathBuf::from(ordner);
    match quote_ordner(&ordner, channel, bytes, jetzt, ortszeit) {
        Ok(()) => {
            println!(
                "{bytes} Bytes von gestern unter {}",
                ordner.join("channels").join(channel).display()
            );
            ExitCode::SUCCESS
        }
        Err(fehler) => {
            eprintln!("Nicht erzeugt: {fehler}");
            ExitCode::FAILURE
        }
    }
}

fn ortszeit(zeitpunkt: DateTime<Utc>) -> Zeit {
    zeitpunkt.with_timezone(&Local).fixed_offset()
}
