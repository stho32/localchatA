//! Verlauf beantworten (B0144, B0146; ADR-0025): `GET /api/channels/{name}/nachrichten[?vor=<zeit> | ?nach=<zeit>]
//! [&n=1..200]` mit Sitzung → 200 `{channel, nachrichten, aelter, neuer}` | 401 | 404 `{fehler}` | 422 `{fehler,
//! regel}`. Die Abfrage wird vor jeder Arbeit begrenzt und geprüft, abgewiesen statt umgeformt.

use crate::gespraeche::abfrage::{Abfrage, Abschnitt, Blockgroesse};
use crate::uhr::Zeit;

/// Welche Regel eine Abfrage verletzt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Abfragefehler {
    Cursor,
    N,
}

/// Längere Abfragen liest niemand: zwei Cursor mit Kodierung und `n` brauchen unter 100 Zeichen.
const ABFRAGE_HOECHSTENS: usize = 256;

/// Der Query-Text (ohne `?`) als Abfrage: höchstens einer von `vor` und `nach` als Zeit nach RFC 3339 mit Versatz
/// (ein unkodiertes `+` ist ein Leerzeichen und damit unlesbar), `n` als ganze Zahl 1–200, Standard 100.
/// Unbekannte Schlüssel werden übergangen.
pub fn abfrage_lesen(query: Option<&str>) -> Result<Abfrage, Abfragefehler> {
    let query = query.unwrap_or_default();
    if query.len() > ABFRAGE_HOECHSTENS {
        return Err(Abfragefehler::Cursor);
    }
    let mut abschnitt = None;
    let mut n = None;
    for paar in query.split('&').filter(|paar| !paar.is_empty()) {
        let (schluessel, wert) = paar.split_once('=').unwrap_or((paar, ""));
        match dekodieren(schluessel).as_deref() {
            Some("vor") if abschnitt.is_none() => {
                abschnitt = Some(Abschnitt::Vor(cursor_lesen(wert)?))
            }
            Some("nach") if abschnitt.is_none() => {
                abschnitt = Some(Abschnitt::Nach(cursor_lesen(wert)?))
            }
            Some("vor" | "nach") => return Err(Abfragefehler::Cursor),
            Some("n") if n.is_none() => n = Some(n_lesen(wert)?),
            Some("n") => return Err(Abfragefehler::N),
            _ => {}
        }
    }
    Ok(Abfrage {
        abschnitt: abschnitt.unwrap_or(Abschnitt::Neueste),
        n: n.unwrap_or_else(Blockgroesse::standard),
    })
}

fn cursor_lesen(wert: &str) -> Result<Zeit, Abfragefehler> {
    dekodieren(wert)
        .and_then(|text| Zeit::parse_from_rfc3339(&text).ok())
        .ok_or(Abfragefehler::Cursor)
}

/// Eine bis drei Ziffern, sonst keine ganze Zahl im Sinn der Regel (kein Vorzeichen, kein Exponent).
fn n_lesen(wert: &str) -> Result<Blockgroesse, Abfragefehler> {
    let text = dekodieren(wert).ok_or(Abfragefehler::N)?;
    let nur_ziffern =
        (1..=3).contains(&text.len()) && text.bytes().all(|byte| byte.is_ascii_digit());
    if !nur_ziffern {
        return Err(Abfragefehler::N);
    }
    text.parse()
        .ok()
        .and_then(Blockgroesse::bilden)
        .ok_or(Abfragefehler::N)
}

/// Formular-Kodierung: `+` ist ein Leerzeichen, `%XX` ein Byte; eine kaputte Sequenz oder kein UTF-8 heißt
/// unlesbar.
fn dekodieren(text: &str) -> Option<String> {
    let mut bytes = Vec::with_capacity(text.len());
    let mut rest = text.as_bytes();
    while let Some((&erstes, danach)) = rest.split_first() {
        rest = danach;
        match erstes {
            b'+' => bytes.push(b' '),
            b'%' => {
                let hex = rest
                    .get(..2)
                    .filter(|hex| hex.iter().all(u8::is_ascii_hexdigit))?;
                bytes.push(u8::from_str_radix(std::str::from_utf8(hex).ok()?, 16).ok()?);
                rest = &rest[2..];
            }
            anderes => bytes.push(anderes),
        }
    }
    String::from_utf8(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zeit(text: &str) -> Zeit {
        Zeit::parse_from_rfc3339(text).expect("Testzeit")
    }

    fn gelesen(query: &str) -> Result<(Abschnitt, usize), Abfragefehler> {
        abfrage_lesen(Some(query)).map(|abfrage| (abfrage.abschnitt, abfrage.n.wert()))
    }

    const VIERZEHN_UHR: &str = "2026-10-28T14:00:00.000+01:00";

    #[test]
    fn ohne_abfrage_die_neuesten_hundert() {
        assert_eq!(
            abfrage_lesen(None).map(|abfrage| (abfrage.abschnitt, abfrage.n.wert())),
            Ok((Abschnitt::Neueste, 100))
        );
        assert_eq!(gelesen(""), Ok((Abschnitt::Neueste, 100)));
        assert_eq!(gelesen("farbe=rot"), Ok((Abschnitt::Neueste, 100)));
    }

    #[test]
    fn n_als_spanne_eins_und_zweihundert_gehen() {
        assert_eq!(gelesen("n=1"), Ok((Abschnitt::Neueste, 1)));
        assert_eq!(gelesen("n=200"), Ok((Abschnitt::Neueste, 200)));
    }

    #[test]
    fn n_ausserhalb_oder_keine_ganze_zahl_ist_regel_n() {
        for query in [
            "n=0", "n=201", "n=abc", "n=1.5", "n=-1", "n=", "n=%2B1", "n=+1", "n=1&n=2", "n=0100",
            "n=1e2",
        ] {
            assert_eq!(gelesen(query), Err(Abfragefehler::N), "{query}");
        }
    }

    #[test]
    fn cursor_kodiert_wird_als_zeitpunkt_gelesen() {
        assert_eq!(
            gelesen("vor=2026-10-28T14:00:00.000%2B01:00"),
            Ok((Abschnitt::Vor(zeit(VIERZEHN_UHR)), 100))
        );
        assert_eq!(
            gelesen("nach=2026-10-28T14:00:00.000%2B01:00&n=7"),
            Ok((Abschnitt::Nach(zeit(VIERZEHN_UHR)), 7))
        );
        assert_eq!(
            gelesen("n=7&vor=2026-10-28T13:00:00.000Z"),
            Ok((Abschnitt::Vor(zeit("2026-10-28T14:00:00.000+01:00")), 7))
        );
        assert_eq!(
            gelesen("%76or=2026-10-28T14%3A00%3A00.000%2B01%3A00"),
            Ok((Abschnitt::Vor(zeit(VIERZEHN_UHR)), 100))
        );
    }

    #[test]
    fn unkodiertes_plus_zwei_cursor_und_unlesbares_sind_regel_cursor() {
        for query in [
            "vor=2026-10-28T14:00:00.000+01:00",
            "vor=2026-10-28T14:00:00.000%2B01:00&nach=2026-10-28T14:00:00.000%2B01:00",
            "vor=2026-10-28T14:00:00.000%2B01:00&vor=2026-10-28T14:00:00.000%2B01:00",
            "vor=gestern",
            "nach=",
            "vor=2026-10-28T14:00:00.000",
            "vor=2026-10-28T14:00:00.000%2",
            "vor=%FF",
            "vor=%+1",
        ] {
            assert_eq!(gelesen(query), Err(Abfragefehler::Cursor), "{query}");
        }
    }

    #[test]
    fn eine_zu_lange_abfrage_wird_nicht_gelesen() {
        let lang = format!("farbe={}&n=5", "x".repeat(300));

        assert_eq!(gelesen(&lang), Err(Abfragefehler::Cursor));
        assert_eq!(
            gelesen(&format!("farbe={}&n=5", "x".repeat(200))),
            Ok((Abschnitt::Neueste, 5))
        );
    }
}
