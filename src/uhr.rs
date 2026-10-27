//! Uhr: der einzige Ort, an dem „jetzt“ entsteht (Aspekt X00007).
//!
//! Zeiten sind Zeitpunkte mit Versatz (`Zeit`), auf Millisekunden gekürzt, und werden als Zeitpunkt
//! verglichen, nie als Zeichenkette. Das lokale Datum folgt der Zeitzone des Prozesses (`TZ`).

use chrono::{DateTime, FixedOffset, Local, TimeZone, Timelike, Utc};
use std::fmt;
use std::time::Instant;

/// Zeitpunkt mit Versatz; `Ord` und `Eq` vergleichen den Zeitpunkt (02:00+01:00 ist jünger als 02:59+02:00).
pub type Zeit = DateTime<FixedOffset>;

/// Woher „jetzt“ kommt: Systemuhr, Startzeit aus der Umgebung oder im Test eine gestellte Uhr.
pub trait Uhr: Send + Sync {
    fn jetzt(&self) -> Zeit;
}

/// Name der Umgebungsvariablen, die die Startzeit vorgibt (Testuhr, R00002).
pub const STARTZEIT_VARIABLE: &str = "LOCALCHATA_JETZT";

/// `LOCALCHATA_JETZT` ist gesetzt, aber kein Zeitpunkt nach RFC 3339 mit Versatz.
#[derive(Debug, PartialEq, Eq)]
pub struct Startzeitfehler {
    pub wert: String,
}

impl fmt::Display for Startzeitfehler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{STARTZEIT_VARIABLE} ist kein Zeitpunkt nach RFC 3339 mit Versatz: {} — z. B. 2026-10-27T12:00:00+01:00",
            self.wert
        )
    }
}

/// Bildet die Uhr des Prozesses: ohne Wert die Systemuhr, sonst eine Uhr, die ab diesem Zeitpunkt weiterläuft.
pub fn aus_umgebung(wert: Option<&str>) -> Result<Box<dyn Uhr>, Startzeitfehler> {
    match wert {
        None => Ok(Box::new(Systemuhr)),
        Some(text) => match DateTime::parse_from_rfc3339(text) {
            Ok(start) => Ok(Box::new(Startuhr {
                start: start.with_timezone(&Utc),
                seit: Instant::now(),
            })),
            Err(_) => Err(Startzeitfehler {
                wert: text.to_string(),
            }),
        },
    }
}

/// Die Uhr des Rechners in der lokalen Zone.
pub struct Systemuhr;

impl Uhr for Systemuhr {
    fn jetzt(&self) -> Zeit {
        lokal(Utc::now())
    }
}

/// Läuft ab der vorgegebenen Startzeit mit der verstrichenen Prozesszeit weiter.
struct Startuhr {
    start: DateTime<Utc>,
    seit: Instant,
}

impl Uhr for Startuhr {
    fn jetzt(&self) -> Zeit {
        let verstrichen = chrono::Duration::from_std(self.seit.elapsed()).unwrap_or_default();
        lokal(self.start + verstrichen)
    }
}

/// Zeitpunkt in der lokalen Zone des Prozesses, auf Millisekunden gekürzt.
fn lokal(zeitpunkt: DateTime<Utc>) -> Zeit {
    let lokal = Local
        .from_utc_datetime(&zeitpunkt.naive_utc())
        .fixed_offset();
    auf_millisekunden(lokal)
}

/// Kürzt auf ganze Millisekunden — feiner steht keine Zeit im Log.
pub fn auf_millisekunden(zeit: Zeit) -> Zeit {
    let millis = zeit.nanosecond() / 1_000_000 * 1_000_000;
    zeit.with_nanosecond(millis).unwrap_or(zeit)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zeit(text: &str) -> Zeit {
        DateTime::parse_from_rfc3339(text).expect("Testzeit")
    }

    #[test]
    fn startzeit_aus_der_umgebung_gilt_als_jetzt_und_laeuft_weiter() {
        let uhr = aus_umgebung(Some("2026-10-27T12:00:00+01:00")).expect("lesbar");

        let erst = uhr.jetzt();
        let start = zeit("2026-10-27T12:00:00+01:00");
        assert!(erst >= start, "{erst} vor {start}");
        assert!(erst - start < chrono::Duration::seconds(5), "{erst}");
        assert!(uhr.jetzt() >= erst);
    }

    #[test]
    fn startzeit_in_utc_meint_denselben_zeitpunkt() {
        let uhr = aus_umgebung(Some("2026-10-27T11:00:00Z")).expect("lesbar");
        assert!(uhr.jetzt() >= zeit("2026-10-27T12:00:00+01:00"));
        assert!(uhr.jetzt() < zeit("2026-10-27T12:00:05+01:00"));
    }

    #[test]
    fn unlesbare_startzeit_ist_ein_fehler_der_die_variable_nennt() {
        for wert in ["gestern", "2026-10-27T12:00:00", ""] {
            let fehler = match aus_umgebung(Some(wert)) {
                Err(fehler) => fehler,
                Ok(_) => panic!("{wert:?} darf keine Uhr ergeben"),
            };
            assert_eq!(fehler.wert, wert);
            let text = fehler.to_string();
            assert!(text.contains("LOCALCHATA_JETZT"), "{text}");
            assert!(!text.contains('\n'), "{text}");
        }
    }

    #[test]
    fn ohne_startzeit_gilt_die_systemuhr() {
        let vorher = Utc::now() - chrono::Duration::milliseconds(1);
        let jetzt = aus_umgebung(None).expect("Systemuhr").jetzt();
        assert!(jetzt >= vorher && jetzt <= Utc::now(), "{jetzt}");
    }

    #[test]
    fn jetzt_ist_auf_millisekunden_gekuerzt() {
        let uhr = aus_umgebung(Some("2026-10-27T12:00:00.123456789+01:00")).expect("lesbar");
        assert_eq!(uhr.jetzt().nanosecond() % 1_000_000, 0);
        assert_eq!(
            auf_millisekunden(zeit("2026-10-27T12:00:00.123999+01:00")),
            zeit("2026-10-27T12:00:00.123+01:00")
        );
    }
}
