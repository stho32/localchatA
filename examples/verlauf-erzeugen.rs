//! Testdaten für R00007: `cargo run --release --example verlauf-erzeugen -- <ordner> <channel> <anzahl> <tage>`
//! schreibt `<anzahl>` Nachrichten über `<tage>` Tagesdateien bis heute (nach `LOCALCHATA_JETZT` bzw. Systemuhr
//! und `TZ`) nach `<ordner>/channels/<channel>/`.

use chrono::{DateTime, Local, Utc};
use localchata::testdaten::verlauf_erzeugen;
use localchata::uhr::{self, Zeit};
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let argumente: Vec<String> = std::env::args().skip(1).collect();
    let [ordner, channel, anzahl, tage] = argumente.as_slice() else {
        eprintln!("Aufruf: verlauf-erzeugen <ordner> <channel> <anzahl> <tage>");
        return ExitCode::from(2);
    };
    let (Ok(anzahl), Ok(tage)) = (anzahl.parse::<usize>(), tage.parse::<u32>()) else {
        eprintln!("<anzahl> und <tage> sind ganze Zahlen");
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
    match verlauf_erzeugen(&ordner, channel, anzahl, tage, jetzt, ortszeit) {
        Ok(()) => {
            println!(
                "{anzahl} Nachrichten in {tage} Tagesdateien unter {}",
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
