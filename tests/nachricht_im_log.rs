//! F0005 Nachricht an das Tageslog anhängen — über die Bibliothek mit Datei-Logablage im Wegwerf-Ordner
//! und, für den Rundweg, über das echte Programm (R00002, US-4 bis US-8 und Kriterien).
//!
//! `cargo test --test nachricht_im_log us<n>_` fährt genau den Test zum Szenario.

mod gemeinsam;

use chrono::{DateTime, NaiveDate};
use gemeinsam::{gesundheit, starten_mit};
use localchata::fehler::Fehler;
use localchata::gespraeche::logformat::saetze;
use localchata::gespraeche::{Dateiablage, Gespraech, Gespraeche, Kennung, Speicherablage};
use localchata::uhr::{GestellteUhr, Zeit};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

fn zeit(text: &str) -> Zeit {
    DateTime::parse_from_rfc3339(text).expect("Testzeit")
}

fn tag(text: &str) -> NaiveDate {
    NaiveDate::parse_from_str(text, "%Y-%m-%d").expect("Testtag")
}

fn claude() -> Kennung {
    Kennung::bilden("claude@10.0.0.5").expect("Kennung")
}

fn allgemein() -> Gespraech {
    Gespraech::channel("allgemein")
}

/// Wegwerf-Ordner mit leerem `channels/allgemein/`.
fn ordner_mit_allgemein() -> tempfile::TempDir {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    fs::create_dir_all(wurzel.path().join("channels/allgemein")).expect("Channel-Ordner");
    wurzel
}

/// Lädt den Ordner mit einer gestellten Uhr; die Uhr bleibt für den Test erreichbar.
fn laden(ordner: &Path, jetzt: &str) -> (Gespraeche, Arc<GestellteUhr>) {
    let uhr = Arc::new(GestellteUhr::neu(zeit(jetzt)));
    let gespraeche =
        Gespraeche::laden(Arc::new(Dateiablage::neu(ordner)), uhr.clone()).expect("geladen");
    (gespraeche, uhr)
}

fn tagesdatei(ordner: &Path, tag: &str) -> PathBuf {
    ordner.join(format!("channels/allgemein/{tag}.log"))
}

/// Zeiten der Sätze einer Tagesdatei in Dateireihenfolge, gelesen über das Logformat.
fn zeiten_in(datei: &Path) -> Vec<Zeit> {
    let rohtext = fs::read(datei).expect("Tagesdatei");
    saetze(&rohtext)
        .into_iter()
        .map(|satz| satz.expect("kein Defekt").zeit)
        .collect()
}

/// Startet das echte Programm auf dem Ordner und liest den Health-Check.
async fn neustart_kennzahlen(ordner: &Path, jetzt: &str) -> serde_json::Value {
    let mut instanz = starten_mit(
        ordner,
        &["--port", "0", "--host", "127.0.0.1", "--kein-browser"],
        &[("LOCALCHATA_JETZT", jetzt), ("TZ", "Europe/Berlin")],
    );
    let (status, inhalt) = gesundheit(&instanz.adresse).await;
    assert_eq!(status, 200);
    assert!(instanz.signal_und_warten("TERM").success());
    inhalt
}

#[tokio::test]
async fn us4_lesbarer_satz_der_den_neustart_ueberlebt() {
    let wurzel = ordner_mit_allgemein();
    let (gespraeche, _) = laden(wurzel.path(), "2026-10-27T14:07:12.345+01:00");

    gespraeche
        .senden(
            &allgemein(),
            &claude(),
            "# Titel\n\n@ kein Kopf\n\tTab\r\nEnde\n\n",
        )
        .expect("gesendet");

    assert_eq!(
        fs::read(tagesdatei(wurzel.path(), "2026-10-27")).expect("Tagesdatei"),
        b"@ 2026-10-27T14:07:12.345+01:00 claude@10.0.0.5\n  # Titel\n\n  @ kein Kopf\n  \tTab\n  Ende\n\n"
    );
    let inhalt = neustart_kennzahlen(wurzel.path(), "2026-10-27T15:00:00+01:00").await;
    assert_eq!(inhalt["nachrichten"], 1);
    assert_eq!(inhalt["defekte_zeilen"], 0);
}

#[tokio::test]
async fn rundweg_drei_nachrichten_nach_dem_neustart_ohne_defekte() {
    let wurzel = ordner_mit_allgemein();
    let (gespraeche, _) = laden(wurzel.path(), "2026-10-27T14:07:12.345+01:00");

    for text in ["eins", "@ kein Kopf\nzwei", "drei\n\n"] {
        gespraeche
            .senden(&allgemein(), &claude(), text)
            .expect("gesendet");
    }

    let inhalt = neustart_kennzahlen(wurzel.path(), "2026-10-27T15:00:00+01:00").await;
    assert_eq!(inhalt["nachrichten"], 3);
    assert_eq!(inhalt["defekte_zeilen"], 0);
}

#[test]
fn us5_rotation_ueber_mitternacht() {
    let wurzel = ordner_mit_allgemein();
    let (gespraeche, uhr) = laden(wurzel.path(), "2026-10-27T23:59:59.999+01:00");
    gespraeche
        .senden(&allgemein(), &claude(), "spät")
        .expect("erste");
    let vorher = fs::read(tagesdatei(wurzel.path(), "2026-10-27")).expect("alter Tag");
    assert!(!tagesdatei(wurzel.path(), "2026-10-28").exists());

    uhr.stellen(zeit("2026-10-28T00:00:00.000+01:00"));
    gespraeche
        .senden(&allgemein(), &claude(), "früh")
        .expect("zweite");

    assert_eq!(
        zeiten_in(&tagesdatei(wurzel.path(), "2026-10-27")),
        vec![zeit("2026-10-27T23:59:59.999+01:00")]
    );
    assert_eq!(
        zeiten_in(&tagesdatei(wurzel.path(), "2026-10-28")),
        vec![zeit("2026-10-28T00:00:00.000+01:00")]
    );
    assert_eq!(
        fs::read(tagesdatei(wurzel.path(), "2026-10-27")).expect("alter Tag"),
        vorher
    );
}

#[test]
fn us6_zwei_nachrichten_in_derselben_millisekunde() {
    let wurzel = ordner_mit_allgemein();
    let (gespraeche, _) = laden(wurzel.path(), "2026-10-27T14:07:12.345+01:00");

    gespraeche
        .senden(&allgemein(), &claude(), "a")
        .expect("erste");
    gespraeche
        .senden(&allgemein(), &claude(), "b")
        .expect("zweite");

    let rohtext = fs::read_to_string(tagesdatei(wurzel.path(), "2026-10-27")).expect("Datei");
    let erste = rohtext
        .find("@ 2026-10-27T14:07:12.345+01:00 claude@10.0.0.5\n  a\n")
        .expect("erste Zeit");
    let zweite = rohtext
        .find("@ 2026-10-27T14:07:12.346+01:00 claude@10.0.0.5\n  b\n")
        .expect("zweite Zeit");
    assert!(erste < zweite);
}

#[test]
fn zurueckgestellte_uhr_ergibt_eine_millisekunde_nach_der_letzten() {
    let wurzel = ordner_mit_allgemein();
    let (gespraeche, uhr) = laden(wurzel.path(), "2026-10-27T14:07:12.345+01:00");
    gespraeche
        .senden(&allgemein(), &claude(), "a")
        .expect("erste");

    uhr.stellen(zeit("2026-10-27T14:00:00.000+01:00"));
    let zweite = gespraeche
        .senden(&allgemein(), &claude(), "b")
        .expect("zweite");

    assert_eq!(zweite.zeit.to_rfc3339(), "2026-10-27T14:07:12.346+01:00");
    assert!(
        fs::read_to_string(tagesdatei(wurzel.path(), "2026-10-27"))
            .expect("Datei")
            .contains("@ 2026-10-27T14:07:12.346+01:00 claude@10.0.0.5\n")
    );
}

#[test]
fn us7_zeitumstellung_im_oktober() {
    let wurzel = ordner_mit_allgemein();
    let (gespraeche, uhr) = laden(wurzel.path(), "2026-10-25T02:59:59.999+02:00");
    gespraeche
        .senden(&allgemein(), &claude(), "Sommerzeit")
        .expect("erste");

    uhr.stellen(zeit("2026-10-25T02:00:00.000+01:00"));
    let zweite = gespraeche
        .senden(&allgemein(), &claude(), "Winterzeit")
        .expect("zweite");

    assert_eq!(zweite.zeit.to_rfc3339(), "2026-10-25T02:00:00+01:00");
    let rohtext = fs::read_to_string(tagesdatei(wurzel.path(), "2026-10-25")).expect("Datei");
    let sommer = rohtext
        .find("@ 2026-10-25T02:59:59.999+02:00 claude@10.0.0.5\n  Sommerzeit\n")
        .expect("erste");
    let winter = rohtext
        .find("@ 2026-10-25T02:00:00.000+01:00 claude@10.0.0.5\n  Winterzeit\n")
        .expect("zweite");
    assert!(sommer < winter);
    assert_eq!(
        zeiten_in(&tagesdatei(wurzel.path(), "2026-10-25"))
            .windows(2)
            .map(|paar| (paar[1] - paar[0]).num_milliseconds())
            .collect::<Vec<_>>(),
        vec![1]
    );
}

#[test]
fn zeitumstellung_spaeter_in_der_wiederholten_stunde_behaelt_ihre_uhrzeit() {
    let wurzel = ordner_mit_allgemein();
    let (gespraeche, uhr) = laden(wurzel.path(), "2026-10-25T02:59:59.999+02:00");
    gespraeche
        .senden(&allgemein(), &claude(), "Sommerzeit")
        .expect("erste");

    uhr.stellen(zeit("2026-10-25T02:30:00.000+01:00"));
    let zweite = gespraeche
        .senden(&allgemein(), &claude(), "eine halbe Stunde später")
        .expect("zweite");

    assert_eq!(zweite.zeit.to_rfc3339(), "2026-10-25T02:30:00+01:00");
    assert_eq!(
        zeiten_in(&tagesdatei(wurzel.path(), "2026-10-25")),
        vec![
            zeit("2026-10-25T02:59:59.999+02:00"),
            zeit("2026-10-25T02:30:00.000+01:00")
        ]
    );
}

#[test]
fn us8_schreibfehler_hinterlaesst_nichts() {
    let ablage = Arc::new(Speicherablage::neu());
    let satz = "@ 2026-10-27T14:00:00.000+01:00 shoff@127.0.0.1\n  vorher\n\n";
    ablage.tag_setzen(&allgemein(), tag("2026-10-27"), satz);
    let uhr = Arc::new(GestellteUhr::neu(zeit("2026-10-27T14:07:12.345+01:00")));
    let gespraeche = Gespraeche::laden(ablage.clone(), uhr).expect("geladen");
    ablage.fehler_nach(20);

    let gescheitert = gespraeche.senden(&allgemein(), &claude(), "verloren");

    assert!(
        matches!(gescheitert, Err(Fehler::Ablage(_))),
        "{gescheitert:?}"
    );
    let danach = ablage.inhalt(&allgemein(), tag("2026-10-27")).expect("Tag");
    assert_eq!(danach.len(), satz.len());
    assert_eq!(gespraeche.kennzahlen().nachrichten, 1);

    let zweite = gespraeche
        .senden(&allgemein(), &claude(), "heil")
        .expect("zweite");

    assert!(zweite.zeit > zeit("2026-10-27T14:07:12.345+01:00"));
    assert_eq!(
        ablage.inhalt(&allgemein(), tag("2026-10-27")).as_deref(),
        Some(format!("{satz}@ 2026-10-27T14:07:12.346+01:00 claude@10.0.0.5\n  heil\n\n").as_str())
    );
    assert_eq!(gespraeche.kennzahlen().nachrichten, 2);
}

#[test]
fn fortsetzung_beim_start_am_juengsten_satz_im_ganzen_ordner() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    fs::create_dir_all(wurzel.path().join("channels/a")).expect("leerer Channel");
    fs::create_dir_all(wurzel.path().join("channels/b")).expect("Channel b");
    fs::write(
        wurzel.path().join("channels/b/2026-10-27.log"),
        "@ 2026-10-27T14:00:00.000+01:00 shoff@127.0.0.1\n  zuletzt\n\n",
    )
    .expect("jüngster Satz");
    let (gespraeche, _) = laden(wurzel.path(), "2026-10-27T13:00:00.000+01:00");

    let erste = gespraeche
        .senden(&Gespraech::channel("a"), &claude(), "x")
        .expect("gesendet");

    assert_eq!(erste.zeit.to_rfc3339(), "2026-10-27T14:00:00.001+01:00");
}

#[test]
fn unbekanntes_gespraech_legt_weder_ordner_noch_datei_an() {
    let wurzel = ordner_mit_allgemein();
    let (gespraeche, _) = laden(wurzel.path(), "2026-10-27T14:07:12.345+01:00");

    let ergebnis = gespraeche.senden(&Gespraech::channel("gibtsnicht"), &claude(), "x");

    assert!(
        matches!(ergebnis, Err(Fehler::Unbekannt(_))),
        "{ergebnis:?}"
    );
    assert!(!wurzel.path().join("channels/gibtsnicht").exists());
    assert_eq!(
        fs::read_dir(wurzel.path().join("channels/allgemein"))
            .expect("Ordner")
            .count(),
        0
    );
}

#[tokio::test]
async fn zeilenende_waechter_neuer_satz_in_neuer_zeile_und_neustart_ohne_defekte() {
    let wurzel = ordner_mit_allgemein();
    let datei = tagesdatei(wurzel.path(), "2026-10-27");
    fs::write(
        &datei,
        "@ 2026-10-27T14:00:00.000+01:00 shoff@127.0.0.1\n  Rest",
    )
    .expect("ohne \\n");
    let (gespraeche, _) = laden(wurzel.path(), "2026-10-27T14:07:12.345+01:00");

    gespraeche
        .senden(&allgemein(), &claude(), "neu")
        .expect("gesendet");

    assert_eq!(
        fs::read_to_string(&datei).expect("Datei"),
        "@ 2026-10-27T14:00:00.000+01:00 shoff@127.0.0.1\n  Rest\n@ 2026-10-27T14:07:12.345+01:00 claude@10.0.0.5\n  neu\n\n"
    );
    assert_eq!(
        gespraeche.kennzahlen().log_bytes,
        fs::metadata(&datei).expect("Datei").len()
    );
    let inhalt = neustart_kennzahlen(wurzel.path(), "2026-10-27T15:00:00+01:00").await;
    assert_eq!(inhalt["nachrichten"], 2);
    assert_eq!(inhalt["defekte_zeilen"], 0);
}

#[tokio::test]
async fn reihenfolge_unter_last_zwanzig_mal_fuenfzig_streng_steigend_ohne_doppelte() {
    let wurzel = ordner_mit_allgemein();
    let (gespraeche, _) = laden(wurzel.path(), "2026-10-27T14:07:12.345+01:00");
    let gespraeche = Arc::new(gespraeche);

    let faeden: Vec<_> = (0..20)
        .map(|faden| {
            let gespraeche = gespraeche.clone();
            std::thread::spawn(move || {
                for nummer in 0..50 {
                    gespraeche
                        .senden(&allgemein(), &claude(), &format!("{faden}-{nummer}"))
                        .expect("gesendet");
                }
            })
        })
        .collect();
    for faden in faeden {
        faden.join().expect("Faden");
    }

    let zeiten = zeiten_in(&tagesdatei(wurzel.path(), "2026-10-27"));
    assert_eq!(zeiten.len(), 1000);
    assert!(
        zeiten.windows(2).all(|paar| paar[0] < paar[1]),
        "Zeiten in Dateireihenfolge streng steigend"
    );
    assert_eq!(gespraeche.kennzahlen().nachrichten, 1000);
    let inhalt = neustart_kennzahlen(wurzel.path(), "2026-10-27T15:00:00+01:00").await;
    assert_eq!(inhalt["nachrichten"], 1000);
    assert_eq!(inhalt["defekte_zeilen"], 0);
}

/// Datei der Abnahme-Probe B1 a): zwei heile Sätze, der dritte mitten im Zeichen `ä` abgeschnitten (177 Bytes).
const ABGESCHNITTEN: &[u8] = b"@ 2026-10-27T10:00:00.000+01:00 shoff@127.0.0.1\n  Wie weit?\n\n@ 2026-10-27T10:00:05.000+01:00 claude@10.0.0.5\n  Fertig.\n\n@ 2026-10-27T10:00:06.000+01:00 claude@10.0.0.5\n  Scan l\xC3";

#[test]
fn b1c_senden_in_einen_abgeschnittenen_tag_bleibt_byte_gleich_und_ueberlebt_das_neue_laden() {
    let wurzel = ordner_mit_allgemein();
    let datei = tagesdatei(wurzel.path(), "2026-10-27");
    fs::write(&datei, ABGESCHNITTEN).expect("abgeschnittener Tag");
    let (vorher, _) = laden(wurzel.path(), "2026-10-27T12:00:00+01:00");
    let geladen = vorher.kennzahlen().nachrichten;
    assert!(geladen >= 2, "beide heilen Sätze geladen: {geladen}");

    vorher
        .senden(
            &allgemein(),
            &Kennung::bilden("a@b").expect("Kennung"),
            "neu",
        )
        .expect("gesendet");

    assert_eq!(
        vorher.kennzahlen().log_bytes,
        fs::metadata(&datei).expect("Datei").len(),
        "Index byte-gleich mit der Datei (A00009)"
    );
    let (nachher, _) = laden(wurzel.path(), "2026-10-27T12:00:00+01:00");
    assert_eq!(nachher.kennzahlen().nachrichten, geladen + 1);
}

#[test]
fn b1_die_zeit_setzt_nach_den_saetzen_eines_abgeschnittenen_tages_fort() {
    let wurzel = ordner_mit_allgemein();
    fs::write(tagesdatei(wurzel.path(), "2026-10-27"), ABGESCHNITTEN).expect("abgeschnittener Tag");
    let (gespraeche, _) = laden(wurzel.path(), "2026-10-27T09:00:00+01:00");

    let gesendet = gespraeche
        .senden(&allgemein(), &claude(), "danach")
        .expect("gesendet");

    assert_eq!(gesendet.zeit, zeit("2026-10-27T10:00:06.001+01:00"));
}
