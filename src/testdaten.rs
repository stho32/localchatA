//! Testdaten-Erzeuger (B0147, R00007): viele Nachrichten in einem Channel für Gate, Entwicklertest und Abnahme —
//! bis I0009 gibt es keine Senderoute. Aufruf über `cargo run --release --example verlauf-erzeugen`.

use crate::gespraeche::{Kennung, Nachricht, logformat};
use crate::uhr::Zeit;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use std::fmt::Write as _;
use std::fs;
use std::io::{self, Write as _};
use std::path::Path;

/// Schreibt `anzahl` Nachrichten gleichmäßig über `tage` Tagesdateien bis zum heutigen Datum (`jetzt`) nach
/// `<ordner>/channels/<channel>/`: je Tag gleich viele (der Rest auf die ersten Tage), Zeiten streng steigend
/// zwischen der Mitternacht des Tages und der nächsten bzw. `jetzt`, im Versatz aus `ortszeit`; Texte mit 10–90
/// Wörtern, jede zwanzigste mit einem 45-zeiligen Codeblock, Absender `claude@10.0.0.1` … `claude@10.0.0.5`.
/// Vorhandene Tagesdateien gleichen Namens werden ersetzt.
pub fn verlauf_erzeugen(
    ordner: &Path,
    channel: &str,
    anzahl: usize,
    tage: u32,
    jetzt: Zeit,
    ortszeit: impl Fn(DateTime<Utc>) -> Zeit,
) -> io::Result<()> {
    let gueltig = !channel.is_empty()
        && channel.len() <= 32
        && channel
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
    if !gueltig || tage == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Channel nach der Namensregel (a–z, 0–9, -) und mindestens ein Tag",
        ));
    }
    let heute = jetzt.date_naive();
    let ziel = ordner.join("channels").join(channel);
    fs::create_dir_all(&ziel)?;
    let mut nummer = 0;
    for tag_nummer in 0..tage {
        let datum = heute - Duration::days(i64::from(tage - 1 - tag_nummer));
        let im_tag =
            anzahl / tage as usize + usize::from((tag_nummer as usize) < anzahl % tage as usize);
        let anfang = mitternacht(datum, &ortszeit);
        let ende = if datum == heute {
            jetzt.with_timezone(&Utc)
        } else {
            mitternacht(datum + Duration::days(1), &ortszeit)
        };
        let schritt_ms = (ende - anfang).num_milliseconds() / (im_tag as i64 + 1);
        if schritt_ms < 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{im_tag} Nachrichten passen nicht streng steigend in den {datum}"),
            ));
        }
        let mut rohtext = String::new();
        for index in 0..im_tag {
            let zeitpunkt = anfang + Duration::milliseconds(schritt_ms * (index as i64 + 1));
            rohtext.push_str(&logformat::satz(&nachricht(nummer, ortszeit(zeitpunkt))?));
            nummer += 1;
        }
        let mut datei = fs::File::create(ziel.join(format!("{datum}.log")))?;
        datei.write_all(rohtext.as_bytes())?;
    }
    Ok(())
}

/// Die Mitternacht des Tages in der Ortszeit als Zeitpunkt; der Versatz wird an der Stelle selbst bestimmt.
fn mitternacht(datum: NaiveDate, ortszeit: &impl Fn(DateTime<Utc>) -> Zeit) -> DateTime<Utc> {
    let naiv = datum.and_hms_opt(0, 0, 0).unwrap_or_default();
    let mut zeitpunkt = naiv.and_utc();
    for _ in 0..2 {
        let versatz = ortszeit(zeitpunkt).offset().local_minus_utc();
        zeitpunkt = (naiv - Duration::seconds(i64::from(versatz))).and_utc();
    }
    zeitpunkt
}

const WOERTER: [&str; 12] = [
    "Befund", "Scan", "läuft", "auf", "dem", "Branch", "Größe", "geprüft", "keine", "neuen",
    "Funde", "weiter",
];

fn nachricht(nummer: usize, zeit: Zeit) -> io::Result<Nachricht> {
    let von = Kennung::bilden(&format!("claude@10.0.0.{}", nummer % 5 + 1))
        .map_err(|_| io::Error::other("Kennung"))?;
    let woerter = 10 + (nummer * 37) % 81;
    let mut text = String::new();
    for wort in 0..woerter {
        text.push_str(WOERTER[(nummer + wort) % WOERTER.len()]);
        text.push(if wort % 12 == 11 { '\n' } else { ' ' });
    }
    let mut text = text.trim_end().to_string();
    if nummer.is_multiple_of(20) {
        text.push_str("\n```\n");
        for zeile in 0..45 {
            let _ = writeln!(text, "let x{zeile} = {zeile};");
        }
        text.push_str("```");
    }
    Ok(Nachricht {
        zeit,
        von,
        art: None,
        text,
    })
}
