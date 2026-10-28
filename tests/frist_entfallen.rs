//! F0034 Entfallen beim Start und beim Verkürzen (R00008, US-7 bis US-11) mit den Frist-Zeilen im
//! Betriebsprotokoll (R00017) — über das echte Programm mit gestellter Uhr (heute 28.10.2026).
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::Absender;
use gemeinsam::frist::{
    MITTAG, beitreten, channel_in_liste, datei_anlegen, frist_setzen, nachrichten, starten_um,
    tag_anlegen, tag_da,
};
use gemeinsam::protokoll::{mit, zeilen};
use serde_json::{Value, json};
use std::fs;

const HEUTE: &str = "2026-10-28";

/// Die Frist-Zeilen ab der Stufe, ohne Zeitstempel.
fn frist_zeilen(zeilen: &[String]) -> Vec<String> {
    zeilen
        .iter()
        .filter(|zeile| zeile.contains(" frist "))
        .map(|zeile| zeile.split_once(' ').expect("Zeitstempel").1.to_string())
        .collect()
}

/// US-7, Kriterien „Start“ (R00008) und „Start“ (R00017): weg, sobald die Startzeile da ist; die Zeile steht vor
/// `INFORMATION start`, stderr schweigt.
#[tokio::test]
async fn us7_der_start_raeumt_alte_tage_weg_und_schreibt_es_vor_der_startmeldung() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    for tag in ["2026-10-20", "2026-10-21", "2026-10-22", "2026-10-28"] {
        tag_anlegen(wurzel.path(), "allgemein", tag, 1);
    }

    let mut instanz = starten_um(wurzel.path(), MITTAG);

    assert!(!tag_da(wurzel.path(), "allgemein", "2026-10-20"));
    assert!(!tag_da(wurzel.path(), "allgemein", "2026-10-21"));
    assert!(tag_da(wurzel.path(), "allgemein", "2026-10-22"));
    assert!(tag_da(wurzel.path(), "allgemein", "2026-10-28"));
    let protokoll = zeilen(wurzel.path(), HEUTE);
    let erwartet = "INFORMATION frist 2 Tage entfallen (2026-10-20 bis 2026-10-21; Frist 7 Tage) | channel=allgemein anzahl=2 grund=start";
    assert_eq!(frist_zeilen(&protokoll), vec![erwartet]);
    let frist_an = protokoll.iter().position(|zeile| zeile.ends_with(erwartet));
    let start_an = protokoll
        .iter()
        .position(|zeile| zeile.contains(" INFORMATION start Instanz bereit"));
    assert!(frist_an < start_an && start_an.is_some(), "{protokoll:#?}");
    assert_eq!(nachrichten(&instanz).await, 2);
    instanz.signal_und_warten("TERM");
    let stderr = instanz.stderr_bis_zum_ende();
    assert!(!stderr.contains("entfallen"), "{stderr}");
}

/// US-11, Kriterien „Defekt heißt nichts“ (R00008) und „Einstellungen defekt bleibt laut“ (R00017).
#[tokio::test]
async fn us11_kaputte_einstellungen_sperren_das_loeschen_und_melden_sich_in_datei_und_stderr() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    datei_anlegen(
        wurzel.path(),
        "channels/kaputt/einstellungen.toml",
        b"frist_tage = \"sieben\"\n",
    );
    datei_anlegen(
        wurzel.path(),
        "channels/zu-klein/einstellungen.toml",
        b"frist_tage = 0\n",
    );
    tag_anlegen(wurzel.path(), "kaputt", "2026-01-01", 1);
    tag_anlegen(wurzel.path(), "zu-klein", "2026-01-01", 1);

    let mut instanz = starten_um(wurzel.path(), MITTAG);

    assert!(tag_da(wurzel.path(), "kaputt", "2026-01-01"));
    assert!(tag_da(wurzel.path(), "zu-klein", "2026-01-01"));
    assert_eq!(
        fs::read(wurzel.path().join("channels/kaputt/einstellungen.toml")).expect("Datei"),
        b"frist_tage = \"sieben\"\n"
    );
    assert_eq!(
        fs::read(wurzel.path().join("channels/zu-klein/einstellungen.toml")).expect("Datei"),
        b"frist_tage = 0\n"
    );
    let eins = Absender::von("127.0.0.1");
    let beta = beitreten(&eins, &instanz, "beta").await;
    for name in ["kaputt", "zu-klein"] {
        let (channel, _) = channel_in_liste(&eins, &instanz, &beta, name).await;
        assert_eq!(channel["frist_tage"], Value::Null, "{name}");
    }
    let erwartet = [
        "WARNING frist Nichts entfällt — Einstellungen defekt | channel=kaputt datei=channels/kaputt/einstellungen.toml zeile=1",
        "WARNING frist Nichts entfällt — Einstellungen defekt | channel=zu-klein datei=channels/zu-klein/einstellungen.toml zeile=1",
    ];
    assert_eq!(frist_zeilen(&zeilen(wurzel.path(), HEUTE)), erwartet);
    instanz.signal_und_warten("TERM");
    let stderr: Vec<String> = instanz
        .stderr_bis_zum_ende()
        .lines()
        .map(str::to_string)
        .collect();
    assert_eq!(frist_zeilen(&stderr), erwartet, "{stderr:#?}");
    assert_eq!(mit(&stderr, "WARNING", "laden").len(), 2, "{stderr:#?}");
}

/// `allgemein` mit `frist_tage = 30` und 20.(2), 21.(3), 22.(1), 28.(1) (R00008 F0034).
fn verkuerzbar(ordner: &std::path::Path) {
    datei_anlegen(
        ordner,
        "channels/allgemein/einstellungen.toml",
        b"farbe = \"blau\"\nfrist_tage = 30\n",
    );
    for (tag, anzahl) in [
        ("2026-10-20", 2),
        ("2026-10-21", 3),
        ("2026-10-22", 1),
        ("2026-10-28", 1),
    ] {
        tag_anlegen(ordner, "allgemein", tag, anzahl);
    }
}

fn alle_vier_da(ordner: &std::path::Path) -> bool {
    ["2026-10-20", "2026-10-21", "2026-10-22", "2026-10-28"]
        .iter()
        .all(|tag| tag_da(ordner, "allgemein", tag))
}

/// US-8 und US-10, Kriterien „Verkürzen braucht Bestätigung“, „Grenze der Vorschau“, „`bestaetigt` als
/// Wahrheitswert“ (R00008) und „Bestätigt verkürzen“, erster Teil (R00017: eine 409 schreibt nichts).
#[tokio::test]
async fn us8_us10_verkuerzen_fragt_mit_zahlen_zurueck_und_aendert_nichts() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    verkuerzbar(wurzel.path());
    let vorher =
        fs::read(wurzel.path().join("channels/allgemein/einstellungen.toml")).expect("Datei");
    let instanz = starten_um(wurzel.path(), MITTAG);
    assert_eq!(nachrichten(&instanz).await, 7);
    let eins = Absender::von("127.0.0.1");
    let beta = beitreten(&eins, &instanz, "beta").await;

    for koerper in [
        json!({ "tage": 7 }),
        json!({ "tage": 7, "bestaetigt": false }),
    ] {
        let rueckfrage = frist_setzen(&eins, &instanz, &beta, "allgemein", koerper.clone()).await;
        assert_eq!(rueckfrage.status, 409, "{koerper} → {}", rueckfrage.json);
        assert_eq!(rueckfrage.json["regel"], "bestaetigen");
        assert_eq!(
            rueckfrage.json["entfaellt"],
            json!({ "tage": 2, "nachrichten": 5, "bis": "2026-10-21" })
        );
        assert_eq!(
            rueckfrage.json["fehler"],
            "Mit dieser Frist entfallen sofort 2 Tagesdateien mit 5 Nachrichten bis 2026-10-21 — mit \"bestaetigt\":true erneut senden."
        );
    }
    let grenze = frist_setzen(&eins, &instanz, &beta, "allgemein", json!({ "tage": 8 })).await;
    assert_eq!(grenze.status, 409, "{}", grenze.json);
    assert_eq!(
        grenze.json["entfaellt"],
        json!({ "tage": 1, "nachrichten": 2, "bis": "2026-10-20" })
    );
    assert_eq!(
        grenze.json["fehler"],
        "Mit dieser Frist entfällt sofort 1 Tagesdatei mit 2 Nachrichten bis 2026-10-20 — mit \"bestaetigt\":true erneut senden."
    );
    for bestaetigt in [json!("ja"), json!(1), Value::Null] {
        let abgewiesen = frist_setzen(
            &eins,
            &instanz,
            &beta,
            "allgemein",
            json!({ "tage": 7, "bestaetigt": bestaetigt }),
        )
        .await;
        assert_eq!(abgewiesen.status, 422, "{bestaetigt} → {}", abgewiesen.json);
        assert_eq!(abgewiesen.json["regel"], "bestaetigt");
    }

    assert!(alle_vier_da(wurzel.path()));
    assert_eq!(nachrichten(&instanz).await, 7);
    let (channel, _) = channel_in_liste(&eins, &instanz, &beta, "allgemein").await;
    assert_eq!(channel["frist_tage"], 30);
    assert_eq!(
        fs::read(wurzel.path().join("channels/allgemein/einstellungen.toml")).expect("Datei"),
        vorher
    );
    assert!(frist_zeilen(&zeilen(wurzel.path(), HEUTE)).is_empty());
}

/// US-24 und Kriterium „Einzahl in der Rückfrage“ (R00008; Abnahme-Probe Rechtschreib-Review #12, Muss 3, über
/// das Programm): `fehler` beugt Verb, Tagesdatei und Nachricht nach ihrer Zahl — 1/1 und 2/1 (leere Tagesdatei).
#[tokio::test]
async fn us24_die_rueckfrage_nennt_eine_tagesdatei_und_eine_nachricht_in_der_einzahl() {
    for (tage, entfaellt, verb_und_zahlen) in [
        (
            &[("2026-10-21", 1), ("2026-10-22", 1), ("2026-10-28", 1)][..],
            json!({ "tage": 1, "nachrichten": 1, "bis": "2026-10-21" }),
            "entfällt sofort 1 Tagesdatei mit 1 Nachricht",
        ),
        (
            &[
                ("2026-10-20", 0),
                ("2026-10-21", 1),
                ("2026-10-22", 1),
                ("2026-10-28", 1),
            ][..],
            json!({ "tage": 2, "nachrichten": 1, "bis": "2026-10-21" }),
            "entfallen sofort 2 Tagesdateien mit 1 Nachricht",
        ),
    ] {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        datei_anlegen(
            wurzel.path(),
            "channels/allgemein/einstellungen.toml",
            b"frist_tage = 30\n",
        );
        for (tag, anzahl) in tage {
            tag_anlegen(wurzel.path(), "allgemein", tag, *anzahl);
        }
        let instanz = starten_um(wurzel.path(), MITTAG);
        let eins = Absender::von("127.0.0.1");
        let beta = beitreten(&eins, &instanz, "beta").await;

        let rueckfrage =
            frist_setzen(&eins, &instanz, &beta, "allgemein", json!({ "tage": 7 })).await;

        assert_eq!(rueckfrage.status, 409, "{}", rueckfrage.json);
        assert_eq!(rueckfrage.json["entfaellt"], entfaellt);
        assert_eq!(
            rueckfrage.json["fehler"],
            format!(
                "Mit dieser Frist {verb_und_zahlen} bis 2026-10-21 — mit \"bestaetigt\":true erneut senden."
            )
        );
    }
}

/// US-9, Kriterien „Bestätigt verkürzen“ und „Verlängern und Verkürzen ohne Entfallen“ (R00008) und „Bestätigt
/// verkürzen“ (R00017: gesetzt, dann entfallen, mit Kennung; stderr schweigt).
#[tokio::test]
async fn us9_bestaetigt_verkuerzen_loescht_sofort_und_steht_mit_kennung_im_protokoll() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    verkuerzbar(wurzel.path());
    let mut instanz = starten_um(wurzel.path(), MITTAG);
    let eins = Absender::von("127.0.0.1");
    let beta = beitreten(&eins, &instanz, "beta").await;

    let verkuerzt = frist_setzen(
        &eins,
        &instanz,
        &beta,
        "allgemein",
        json!({ "tage": 7, "bestaetigt": true }),
    )
    .await;

    assert_eq!(verkuerzt.status, 200, "{}", verkuerzt.json);
    assert_eq!(verkuerzt.json["channel"]["frist_tage"], 7);
    assert_eq!(
        verkuerzt.json["entfallen"],
        json!({ "tage": 2, "nachrichten": 5 })
    );
    assert!(!tag_da(wurzel.path(), "allgemein", "2026-10-20"));
    assert!(!tag_da(wurzel.path(), "allgemein", "2026-10-21"));
    assert!(tag_da(wurzel.path(), "allgemein", "2026-10-22"));
    assert!(tag_da(wurzel.path(), "allgemein", "2026-10-28"));
    assert_eq!(nachrichten(&instanz).await, 2);
    assert_eq!(
        frist_zeilen(&zeilen(wurzel.path(), HEUTE)),
        vec![
            "INFORMATION frist Frist 30 → 7 Tage gesetzt | channel=allgemein kennung=beta@127.0.0.1",
            "INFORMATION frist 2 Tage entfallen (2026-10-20 bis 2026-10-21; Frist 7 Tage) | channel=allgemein anzahl=2 grund=frist_gesetzt kennung=beta@127.0.0.1",
        ]
    );

    let verlaengert =
        frist_setzen(&eins, &instanz, &beta, "allgemein", json!({ "tage": 30 })).await;
    let ohne_entfallen =
        frist_setzen(&eins, &instanz, &beta, "allgemein", json!({ "tage": 10 })).await;

    assert_eq!(verlaengert.status, 200, "{}", verlaengert.json);
    assert_eq!(
        verlaengert.json["entfallen"],
        json!({ "tage": 0, "nachrichten": 0 })
    );
    assert_eq!(ohne_entfallen.status, 200, "{}", ohne_entfallen.json);
    assert!(!tag_da(wurzel.path(), "allgemein", "2026-10-20"));
    assert_eq!(nachrichten(&instanz).await, 2);
    instanz.signal_und_warten("TERM");
    let stderr = instanz.stderr_bis_zum_ende();
    assert!(!stderr.contains(" frist "), "{stderr}");
}
