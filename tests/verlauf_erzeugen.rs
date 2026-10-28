//! B0147 Testdaten-Erzeuger (R00007 F0027, Kriterium „Erzeuger“): 100.000 Nachrichten über 7 Tage bis zum
//! heutigen Datum, Zeiten streng steigend mit dem Versatz ihres Tages; das echte Programm lädt sie ohne Defekt.
//! Die Ortszeit ist hier eine feste Regel für Europe/Berlin im Oktober 2026 (Umstellung am 25.10. um 01:00 UTC),
//! damit der Test nicht an der Zeitzone des Testprozesses hängt.
#![cfg(target_os = "linux")]

mod gemeinsam;

use chrono::{DateTime, FixedOffset, NaiveDate, Utc};
use gemeinsam::{gesundheit, starten_mit};
use localchata::gespraeche::logformat;
use localchata::testdaten::verlauf_erzeugen;
use localchata::uhr::Zeit;
use std::collections::BTreeSet;
use std::fs;

const JETZT: &str = "2026-10-28T15:00:00.000+01:00";

fn berlin(zeitpunkt: DateTime<Utc>) -> Zeit {
    let umstellung = DateTime::parse_from_rfc3339("2026-10-25T01:00:00Z").expect("Umstellung");
    let stunden = if zeitpunkt < umstellung { 2 } else { 1 };
    zeitpunkt.with_timezone(&FixedOffset::east_opt(stunden * 3600).expect("Versatz"))
}

#[tokio::test]
async fn hunderttausend_nachrichten_ueber_sieben_tage_laden_ohne_defekt() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let jetzt = Zeit::parse_from_rfc3339(JETZT).expect("jetzt");

    verlauf_erzeugen(wurzel.path(), "gross", 100_000, 7, jetzt, berlin).expect("erzeugt");

    let ordner = wurzel.path().join("channels/gross");
    let mut dateien: Vec<String> = fs::read_dir(&ordner)
        .expect("Ordner")
        .map(|eintrag| {
            eintrag
                .expect("Eintrag")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    dateien.sort();
    let erwartet: Vec<String> = (22..=28).map(|tag| format!("2026-10-{tag}.log")).collect();
    assert_eq!(dateien, erwartet);

    let mut vorige: Option<Zeit> = None;
    let mut anzahl = 0;
    let mut je_tag = Vec::new();
    let mut absender = BTreeSet::new();
    let mut mit_codeblock = 0;
    for datei in &dateien {
        let rohtext = fs::read(ordner.join(datei)).expect("Tag");
        let datum = NaiveDate::parse_from_str(&datei[..10], "%Y-%m-%d").expect("Datum");
        let saetze = logformat::saetze(&rohtext);
        je_tag.push(saetze.len());
        for satz in saetze {
            let satz = satz.expect("kein Defekt");
            assert_eq!(satz.zeit.date_naive(), datum, "{datei}: Tag der Datei");
            assert!(
                vorige.is_none_or(|vorige| satz.zeit > vorige),
                "streng steigend"
            );
            let versatz = satz.zeit.offset().local_minus_utc() / 3600;
            if datei.starts_with("2026-10-24") {
                assert_eq!(versatz, 2);
            }
            if datei.starts_with("2026-10-26") {
                assert_eq!(versatz, 1);
            }
            let nachricht =
                logformat::satz_als_nachricht(&rohtext[satz.anfang..satz.ende]).expect("Nachricht");
            let ohne_code: String = nachricht
                .text
                .split("```")
                .step_by(2)
                .collect::<Vec<_>>()
                .join(" ");
            let woerter = ohne_code.split_whitespace().count();
            assert!((10..=90).contains(&woerter), "{woerter} Wörter");
            if nachricht.text.contains("```") {
                mit_codeblock += 1;
                assert_eq!(
                    nachricht
                        .text
                        .split("```")
                        .nth(1)
                        .expect("Code")
                        .trim_matches('\n')
                        .lines()
                        .count(),
                    45
                );
            }
            absender.insert(nachricht.von.als_text().to_string());
            vorige = Some(satz.zeit);
            anzahl += 1;
        }
    }
    assert_eq!(anzahl, 100_000);
    assert!(
        je_tag.iter().all(|&tag| tag == 14_285 || tag == 14_286),
        "{je_tag:?}"
    );
    assert_eq!(mit_codeblock, 5_000);
    assert_eq!(
        absender.into_iter().collect::<Vec<_>>(),
        (1..=5)
            .map(|nummer| format!("claude@10.0.0.{nummer}"))
            .collect::<Vec<_>>()
    );
    assert!(
        vorige.expect("letzte") < jetzt,
        "die letzte Zeit liegt vor der Startzeit"
    );

    let instanz = starten_mit(
        wurzel.path(),
        &["--port", "0", "--host", "127.0.0.1", "--kein-browser"],
        &[("LOCALCHATA_JETZT", JETZT), ("TZ", "Europe/Berlin")],
    );
    let (_, inhalt) = gesundheit(&instanz.adresse).await;
    assert_eq!(inhalt["nachrichten"], 100_000);
    assert_eq!(inhalt["defekte_zeilen"], 0);
}

#[test]
fn ein_name_ausserhalb_der_namensregel_wird_nicht_geschrieben() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let jetzt = Zeit::parse_from_rfc3339(JETZT).expect("jetzt");

    for name in ["../raus", "Gross", ""] {
        assert!(
            verlauf_erzeugen(wurzel.path(), name, 10, 1, jetzt, berlin).is_err(),
            "{name}"
        );
    }
    assert!(!wurzel.path().join("raus").exists());
    assert!(!wurzel.path().join("channels").exists());
}
