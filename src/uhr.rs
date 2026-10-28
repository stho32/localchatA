//! Uhr: der einzige Ort, an dem „jetzt“ entsteht (Aspekt X00007).
//!
//! Zeiten sind Zeitpunkte mit Versatz (`Zeit`), auf Millisekunden gekürzt, und werden als Zeitpunkt
//! verglichen, nie als Zeichenkette. Das lokale Datum folgt der Zeitzone des Prozesses (`TZ`).

use chrono::{DateTime, FixedOffset, Local, TimeZone, Timelike, Utc};
use std::ffi::OsStr;
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

/// Der Wert von `LOCALCHATA_JETZT` als Text; ist er gesetzt, aber kein UTF-8, ist er unlesbar wie jeder
/// andere falsche Wert — nie still „nicht gesetzt“ (Code-Review A2).
pub fn startzeit_lesen(wert: Option<&OsStr>) -> Result<Option<&str>, Startzeitfehler> {
    wert.map(|roh| {
        roh.to_str().ok_or_else(|| Startzeitfehler {
            wert: roh.to_string_lossy().into_owned(),
        })
    })
    .transpose()
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

/// Uhr für Tests: steht, bis sie gestellt wird (X00007 „Test: fest/verschiebbar“).
pub struct GestellteUhr {
    zeit: std::sync::Mutex<Zeit>,
}

impl GestellteUhr {
    pub fn neu(zeit: Zeit) -> GestellteUhr {
        GestellteUhr {
            zeit: std::sync::Mutex::new(zeit),
        }
    }

    pub fn stellen(&self, zeit: Zeit) {
        *self
            .zeit
            .lock()
            .unwrap_or_else(|vergiftet| vergiftet.into_inner()) = zeit;
    }
}

impl Uhr for GestellteUhr {
    fn jetzt(&self) -> Zeit {
        *self
            .zeit
            .lock()
            .unwrap_or_else(|vergiftet| vergiftet.into_inner())
    }
}

/// Zeitpunkt in der lokalen Zone des Prozesses, auf Millisekunden gekürzt.
fn lokal(zeitpunkt: DateTime<Utc>) -> Zeit {
    let lokal = Local
        .from_utc_datetime(&zeitpunkt.naive_utc())
        .fixed_offset();
    auf_millisekunden(lokal)
}

/// Die Zeit der nächsten Nachricht: `max(jetzt, letzte + 1 ms)`, verglichen als Zeitpunkt (ADR-0005).
/// Läuft die Zeit der Uhr voraus, steht sie im Versatz von `jetzt` — dem lokalen Versatz der Gegenwart.
pub fn naechste_zeit(jetzt: Zeit, letzte: Option<Zeit>) -> Zeit {
    let jetzt = auf_millisekunden(jetzt);
    match letzte {
        Some(letzte) if jetzt <= letzte => {
            (letzte + chrono::Duration::milliseconds(1)).with_timezone(jetzt.offset())
        }
        _ => jetzt,
    }
}

/// Ab mehr als so viel Vorlauf der Nachrichtenzeit vor der Uhr wird gemeldet (ADR-0035 Regel 4).
pub const VORLAUF_MELDEN_AB_MS: u64 = 60_000;

/// Wie weit die zuletzt vergebene Zeit vor `jetzt` liegt, in ms; 0, wenn sie nicht vorausläuft (B0215).
pub fn vorlauf_ms(letzte: Zeit, jetzt: Zeit) -> u64 {
    u64::try_from((letzte - jetzt).num_milliseconds()).unwrap_or(0)
}

/// Ob ein Vorlauf zu melden ist: mehr als 60 s.
pub fn vorlauf_melden(vorlauf_ms: u64) -> bool {
    vorlauf_ms > VORLAUF_MELDEN_AB_MS
}

/// „Nachrichtenzeit läuft der Systemuhr 2 h 03 min voraus (Uhr zurückgestellt?) — …“; Stunden und Minuten
/// abgerundet.
pub fn vorlauf_text(vorlauf_ms: u64) -> String {
    let minuten = vorlauf_ms / 60_000;
    format!(
        "Nachrichtenzeit läuft der Systemuhr {} h {:02} min voraus (Uhr zurückgestellt?) — Zeiten bleiben streng steigend, bis die Uhr aufholt.",
        minuten / 60,
        minuten % 60
    )
}

/// Kürzt auf ganze Millisekunden — feiner steht keine Zeit im Log.
pub fn auf_millisekunden(zeit: Zeit) -> Zeit {
    let millis = zeit.nanosecond() / 1_000_000 * 1_000_000;
    zeit.with_nanosecond(millis).unwrap_or(zeit)
}

#[cfg(test)]
mod tests {

    /// R00011 F0042 „Vorlauf“ (B0215): letzte 12:00:00.000, Uhr 11:58:59.000 → vergeben 12:00:00.001, Vorlauf
    /// 61.001 ms, zu melden.
    #[test]
    fn vorlauf_der_vergebenen_zeit_vor_der_uhr() {
        let letzte = DateTime::parse_from_rfc3339("2026-10-28T12:00:00.000+01:00").expect("Zeit");
        let jetzt = DateTime::parse_from_rfc3339("2026-10-28T11:58:59.000+01:00").expect("Zeit");

        let vergeben = naechste_zeit(jetzt, Some(letzte));

        assert_eq!(vergeben.to_rfc3339(), "2026-10-28T12:00:00.001+01:00");
        assert_eq!(vorlauf_ms(vergeben, jetzt), 61_001);
        assert!(vorlauf_melden(61_001));
        assert_eq!(vorlauf_ms(jetzt, vergeben), 0, "Uhr voraus: kein Vorlauf");
    }

    /// Schwelle als Spanne: 60.000 ms keine Meldung, 60.001 ms eine.
    #[test]
    fn gemeldet_wird_erst_ueber_60_s() {
        assert!(!vorlauf_melden(0));
        assert!(!vorlauf_melden(60_000));
        assert!(vorlauf_melden(60_001));
    }

    #[test]
    fn der_text_nennt_stunden_und_minuten_abgerundet() {
        assert_eq!(
            vorlauf_text(7_410_002),
            "Nachrichtenzeit läuft der Systemuhr 2 h 03 min voraus (Uhr zurückgestellt?) — Zeiten bleiben streng steigend, bis die Uhr aufholt."
        );
        assert!(vorlauf_text(61_001).contains(" 0 h 01 min voraus"));
    }

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
    fn startzeit_in_utf8_wird_text_fehlende_bleibt_fehlend() {
        assert_eq!(
            startzeit_lesen(Some(OsStr::new("2026-10-27T12:00:00+01:00"))),
            Ok(Some("2026-10-27T12:00:00+01:00"))
        );
        assert_eq!(startzeit_lesen(None), Ok(None));
    }

    #[cfg(unix)]
    #[test]
    fn startzeit_die_kein_utf8_ist_ist_ein_fehler_der_die_variable_nennt() {
        use std::os::unix::ffi::OsStrExt;

        let fehler = startzeit_lesen(Some(OsStr::from_bytes(b"2026\xff")))
            .expect_err("kein UTF-8 darf keine Startzeit ergeben");

        assert_eq!(fehler.wert, "2026\u{FFFD}");
        assert!(fehler.to_string().contains("LOCALCHATA_JETZT"), "{fehler}");
    }

    #[test]
    fn ohne_startzeit_gilt_die_systemuhr() {
        let vorher = Utc::now() - chrono::Duration::milliseconds(1);
        let jetzt = aus_umgebung(None).expect("Systemuhr").jetzt();
        assert!(jetzt >= vorher && jetzt <= Utc::now(), "{jetzt}");
    }

    #[test]
    fn ohne_letzte_zeit_gilt_jetzt_auf_millisekunden() {
        assert_eq!(
            naechste_zeit(zeit("2026-10-27T14:07:12.345678+01:00"), None),
            zeit("2026-10-27T14:07:12.345+01:00")
        );
    }

    #[test]
    fn gleiche_millisekunde_ergibt_eine_millisekunde_spaeter() {
        let jetzt = zeit("2026-10-27T14:07:12.345+01:00");
        let erste = naechste_zeit(jetzt, None);
        let zweite = naechste_zeit(jetzt, Some(erste));
        assert_eq!(erste.to_rfc3339(), "2026-10-27T14:07:12.345+01:00");
        assert_eq!(zweite.to_rfc3339(), "2026-10-27T14:07:12.346+01:00");
    }

    #[test]
    fn spaetere_uhr_gilt_unveraendert() {
        assert_eq!(
            naechste_zeit(
                zeit("2026-10-27T14:07:13.000+01:00"),
                Some(zeit("2026-10-27T14:07:12.345+01:00"))
            )
            .to_rfc3339(),
            "2026-10-27T14:07:13+01:00"
        );
    }

    #[test]
    fn zurueckgestellte_uhr_ergibt_eine_millisekunde_nach_der_letzten() {
        assert_eq!(
            naechste_zeit(
                zeit("2026-10-27T14:00:00.000+01:00"),
                Some(zeit("2026-10-27T14:07:12.345+01:00"))
            )
            .to_rfc3339(),
            "2026-10-27T14:07:12.346+01:00"
        );
    }

    #[test]
    fn zeitumstellung_wird_als_zeitpunkt_verglichen_und_jetzt_behaelt_seine_zeit() {
        assert_eq!(
            naechste_zeit(
                zeit("2026-10-25T02:00:00.000+01:00"),
                Some(zeit("2026-10-25T02:59:59.999+02:00"))
            )
            .to_rfc3339(),
            "2026-10-25T02:00:00+01:00"
        );
    }

    #[test]
    fn in_der_wiederholten_stunde_gilt_jetzt_obwohl_die_ziffern_kleiner_sind() {
        assert_eq!(
            naechste_zeit(
                zeit("2026-10-25T02:30:00.000+01:00"),
                Some(zeit("2026-10-25T02:59:59.999+02:00"))
            )
            .to_rfc3339(),
            "2026-10-25T02:30:00+01:00"
        );
    }

    #[test]
    fn vorauslaufende_zeit_steht_im_versatz_von_jetzt() {
        assert_eq!(
            naechste_zeit(
                zeit("2026-10-25T01:30:00.000+01:00"),
                Some(zeit("2026-10-25T02:59:59.999+02:00"))
            )
            .to_rfc3339(),
            "2026-10-25T02:00:00+01:00"
        );
    }

    #[test]
    fn gestellte_uhr_steht_bis_sie_gestellt_wird() {
        let uhr = GestellteUhr::neu(zeit("2026-10-27T14:07:12.345+01:00"));
        assert_eq!(uhr.jetzt(), zeit("2026-10-27T14:07:12.345+01:00"));
        assert_eq!(uhr.jetzt(), zeit("2026-10-27T14:07:12.345+01:00"));
        uhr.stellen(zeit("2026-10-28T00:00:00.000+01:00"));
        assert_eq!(uhr.jetzt(), zeit("2026-10-28T00:00:00.000+01:00"));
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
