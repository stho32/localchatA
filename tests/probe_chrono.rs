//! Probe für `chrono` (Skill dependency-probe): die Annahmen, auf denen Uhr, Logformat und Rotation stehen.
//!
//! Annahmen: (1) RFC 3339 mit Versatz wird gelesen, „gestern“ und eine Zeit ohne Versatz nicht;
//! (2) `DateTime<FixedOffset>` vergleicht als Zeitpunkt, nicht nach Ziffern; (3) `%.3f%:z` schreibt
//! Millisekunden und Versatz wie im Logformat; (4) `Local` folgt der Zeitzone aus `TZ` samt Sommerzeit.

use chrono::{DateTime, Local, TimeZone};
use std::process::Command;

#[test]
fn probe_rfc3339_mit_versatz_wird_gelesen_anderes_nicht() {
    let zeit = DateTime::parse_from_rfc3339("2026-10-27T12:00:00+01:00").expect("lesbar");
    assert_eq!(zeit.timestamp(), 1_793_098_800);

    assert!(DateTime::parse_from_rfc3339("gestern").is_err());
    assert!(DateTime::parse_from_rfc3339("2026-10-27T12:00:00").is_err());
    assert!(DateTime::parse_from_rfc3339("2026-10-27T14:00:0@").is_err());
}

#[test]
fn probe_vergleich_ist_nach_zeitpunkt_nicht_nach_ziffern() {
    let sommer = DateTime::parse_from_rfc3339("2026-10-25T02:59:59.999+02:00").expect("lesbar");
    let winter = DateTime::parse_from_rfc3339("2026-10-25T02:00:00.000+01:00").expect("lesbar");

    assert!(winter > sommer);
    assert_eq!((winter - sommer).num_milliseconds(), 1);
}

#[test]
fn probe_format_schreibt_millisekunden_und_versatz() {
    let zeit = DateTime::parse_from_rfc3339("2026-10-27T14:07:12.345+01:00").expect("lesbar");
    assert_eq!(
        zeit.format("%Y-%m-%dT%H:%M:%S%.3f%:z").to_string(),
        "2026-10-27T14:07:12.345+01:00"
    );
    let ganz = DateTime::parse_from_rfc3339("2026-10-27T14:07:12Z").expect("lesbar");
    assert_eq!(
        ganz.format("%Y-%m-%dT%H:%M:%S%.3f%:z").to_string(),
        "2026-10-27T14:07:12.000+00:00"
    );
}

/// Hilfslauf für die TZ-Probe: druckt Versatz und lokales Datum des Zeitpunkts aus `PROBE_ZEITPUNKT`.
#[test]
#[ignore = "nur als Kindprozess der TZ-Probe"]
fn probe_hilfslauf_lokale_zeit() {
    let wert = std::env::var("PROBE_ZEITPUNKT").expect("PROBE_ZEITPUNKT");
    let zeitpunkt = DateTime::parse_from_rfc3339(&wert).expect("lesbar");
    let lokal = Local.from_utc_datetime(&zeitpunkt.naive_utc());
    println!("PROBE {} {}", lokal.offset(), lokal.date_naive());
}

fn lokal_unter(tz: &str, zeitpunkt: &str) -> String {
    let ausgabe = Command::new(std::env::current_exe().expect("Testprogramm"))
        .args([
            "--exact",
            "probe_hilfslauf_lokale_zeit",
            "--ignored",
            "--nocapture",
        ])
        .env("TZ", tz)
        .env("PROBE_ZEITPUNKT", zeitpunkt)
        .output()
        .expect("Hilfslauf");
    let text = String::from_utf8_lossy(&ausgabe.stdout).into_owned();
    text.lines()
        .find_map(|zeile| zeile.strip_prefix("PROBE "))
        .unwrap_or_else(|| panic!("keine Probe-Zeile in {text}"))
        .to_string()
}

#[test]
fn probe_local_folgt_tz_samt_sommerzeit() {
    assert_eq!(
        lokal_unter("Europe/Berlin", "2026-10-27T23:30:00Z"),
        "+01:00 2026-10-28"
    );
    assert_eq!(
        lokal_unter("Europe/Berlin", "2026-07-01T12:00:00Z"),
        "+02:00 2026-07-01"
    );
    assert_eq!(
        lokal_unter("America/New_York", "2026-10-28T01:00:00Z"),
        "-04:00 2026-10-27"
    );
}
