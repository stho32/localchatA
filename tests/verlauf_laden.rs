//! F0004 Verlauf beim Start laden — über das echte Programm (R00002, US-1 bis US-3 und Kriterien).

mod gemeinsam;

use gemeinsam::protokoll::ort;
use gemeinsam::{Instanz, aufrufen_mit, gesundheit, starten_mit};
use std::fs;
use std::path::Path;

const TAG_26: &str = "@ 2026-10-26T09:00:00.000+01:00 shoff@127.0.0.1\n  Guten Morgen\n\n@ 2026-10-26T09:01:00.000+01:00 claude@10.0.0.5\n  Scan läuft.\n\n";
const TAG_27: &str = "@ 2026-10-27T10:00:00.000+01:00 shoff@127.0.0.1\n  Wie weit?\n\nkaputt\n@ 2026-10-27T10:00:05.000+01:00 claude@10.0.0.5\n  Fertig.\n\n";
const EIN_SATZ: &str = "@ 2026-10-21T08:00:00.000+02:00 shoff@127.0.0.1\n  Ein Satz.\n\n";

/// Die Orte der Ladedefekte auf stderr (`<datei>:<zeile>`), seit R00014 als Protokollzeilen.
fn orte(stderr: &str) -> Vec<String> {
    stderr.lines().map(ort).collect()
}

fn datei(ordner: &Path, pfad: &str, inhalt: &str) {
    let ziel = ordner.join(pfad);
    fs::create_dir_all(ziel.parent().expect("Elternordner")).expect("Ordner");
    fs::write(ziel, inhalt).expect("Datei");
}

/// Ordner A aus den Akzeptanzkriterien von R00002.
fn ordner_a(ordner: &Path) {
    datei(ordner, "channels/allgemein/2026-10-26.log", TAG_26);
    datei(ordner, "channels/allgemein/2026-10-27.log", TAG_27);
}

fn starten_um(ordner: &Path, jetzt: &str) -> Instanz {
    starten_mit(
        ordner,
        &["--port", "0", "--host", "127.0.0.1", "--kein-browser"],
        &[("LOCALCHATA_JETZT", jetzt), ("TZ", "Europe/Berlin")],
    )
}

/// Startet, liest den Health-Check, beendet geordnet; liefert Antwort, stderr und restliches stdout.
async fn kennzahlen_und_ausgaben(
    ordner: &Path,
    jetzt: &str,
) -> (serde_json::Value, String, String) {
    let mut instanz = starten_um(ordner, jetzt);
    let (status, inhalt) = gesundheit(&instanz.adresse).await;
    assert_eq!(status, 200);
    assert!(instanz.signal_und_warten("TERM").success());
    let stderr = instanz.stderr_bis_zum_ende();
    let stdout = instanz.restliche_ausgabe();
    (inhalt, stderr, stdout)
}

#[tokio::test]
async fn us1_verlauf_laden_und_kennzahlen_ablesen() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    ordner_a(wurzel.path());
    let groesse_26 = fs::metadata(wurzel.path().join("channels/allgemein/2026-10-26.log"))
        .expect("Datei")
        .len();
    assert_eq!(groesse_26, 128, "Ordner A wie in R00002");

    let (inhalt, stderr, stdout) =
        kennzahlen_und_ausgaben(wurzel.path(), "2026-10-27T12:00:00+01:00").await;

    assert_eq!(inhalt["status"], "bereit");
    assert_eq!(inhalt["nachrichten"], 4);
    assert_eq!(inhalt["log_bytes"], 255);
    assert_eq!(inhalt["defekte_zeilen"], 1);
    assert!(inhalt["ladezeit_ms"].as_u64().is_some(), "{inhalt}");
    let zeilen: Vec<&str> = stderr.lines().collect();
    assert_eq!(zeilen.len(), 1, "{stderr:?}");
    assert_eq!(ort(zeilen[0]), "channels/allgemein/2026-10-27.log:4");
    assert_eq!(stdout, "", "stdout trägt nur die Startzeile");
}

#[tokio::test]
async fn us2_nur_tage_in_der_frist_heute_zaehlt_mit() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    datei(wurzel.path(), "channels/allgemein/2026-10-21.log", EIN_SATZ);
    let alt = "@ 2026-10-20T08:00:00.000+02:00 shoff@127.0.0.1\n  Zu alt.\n\n";
    datei(wurzel.path(), "channels/allgemein/2026-10-20.log", alt);

    let (inhalt, stderr, _) =
        kennzahlen_und_ausgaben(wurzel.path(), "2026-10-27T12:00:00+01:00").await;

    assert_eq!(inhalt["nachrichten"], 1);
    assert_eq!(inhalt["log_bytes"], EIN_SATZ.len());
    assert_eq!(inhalt["defekte_zeilen"], 0);
    assert_eq!(stderr, "");
    assert_eq!(
        fs::read_to_string(wurzel.path().join("channels/allgemein/2026-10-20.log"))
            .expect("liegt noch"),
        alt
    );
}

#[tokio::test]
async fn das_lokale_datum_folgt_tz_auch_wenn_die_startzeit_in_utc_steht() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    datei(wurzel.path(), "channels/a/2026-10-21.log", EIN_SATZ);
    datei(wurzel.path(), "channels/a/2026-10-22.log", EIN_SATZ);

    let (inhalt, _, _) = kennzahlen_und_ausgaben(wurzel.path(), "2026-10-27T23:30:00Z").await;

    assert_eq!(
        inhalt["nachrichten"], 1,
        "23:30 UTC ist in Berlin der 28.; ältester Tag der 22."
    );
}

#[tokio::test]
async fn ein_direktgespraech_wird_mitgezaehlt() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    ordner_a(wurzel.path());
    datei(
        wurzel.path(),
        "direkt/claude@10.0.0.5+shoff@127.0.0.1/2026-10-27.log",
        "@ 2026-10-27T11:00:00.000+01:00 claude@10.0.0.5\n  Unter uns.\n\n",
    );

    let (inhalt, _, _) = kennzahlen_und_ausgaben(wurzel.path(), "2026-10-27T12:00:00+01:00").await;

    assert_eq!(inhalt["nachrichten"], 5);
}

#[tokio::test]
async fn kopf_mit_unlesbarer_zeit_ist_ein_defekt_an_der_kopfzeile_die_nachbarn_werden_geladen() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    datei(
        wurzel.path(),
        "channels/allgemein/2026-10-27.log",
        "@ 2026-10-27T14:00:00.000+01:00 a@127.0.0.1\n  eins\n\n@ 2026-10-27T14:00:0@ 2026-10-27T14:00:00.002+01:00 c@127.0.0.1\n  zwei\n\n@ 2026-10-27T14:00:00.003+01:00 b@127.0.0.1\n  drei\n\n",
    );

    let (inhalt, stderr, _) =
        kennzahlen_und_ausgaben(wurzel.path(), "2026-10-27T15:00:00+01:00").await;

    assert_eq!(inhalt["nachrichten"], 2);
    assert_eq!(inhalt["defekte_zeilen"], 1);
    assert_eq!(orte(&stderr), ["channels/allgemein/2026-10-27.log:4"]);
}

#[tokio::test]
async fn andere_dateien_werden_weder_gezaehlt_noch_gemeldet() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    datei(wurzel.path(), "channels/allgemein/2026-10-27.log", EIN_SATZ);
    datei(wurzel.path(), "channels/allgemein/notizen.txt", "kaputt\n");
    datei(
        wurzel.path(),
        "channels/allgemein/einstellungen.toml",
        "frist_tage = 1\n",
    );

    let (inhalt, stderr, _) =
        kennzahlen_und_ausgaben(wurzel.path(), "2026-10-27T12:00:00+01:00").await;

    assert_eq!(inhalt["nachrichten"], 1);
    assert_eq!(inhalt["log_bytes"], EIN_SATZ.len());
    assert_eq!(inhalt["defekte_zeilen"], 0);
    assert_eq!(stderr, "");
}

#[tokio::test]
async fn leerer_ordner_hat_alle_kennzahlen_null_und_nichts_auf_stderr() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");

    let anfang = std::time::Instant::now();
    let (inhalt, stderr, _) =
        kennzahlen_und_ausgaben(wurzel.path(), "2026-10-27T12:00:00+01:00").await;
    let gemessen = anfang.elapsed();

    assert_eq!(inhalt["nachrichten"], 0);
    assert_eq!(inhalt["log_bytes"], 0);
    // Die Ladezeit misst die echte Uhr: auch ein leerer Ordner kann 1 ms brauchen. Ehrlich ist: eine ganze
    // Zahl, höchstens so lang wie der ganze Lauf von außen gemessen (geladen wird vor der Startzeile).
    let ladezeit_ms = inhalt["ladezeit_ms"]
        .as_u64()
        .expect("ladezeit_ms als Zahl");
    assert!(
        u128::from(ladezeit_ms) <= gemessen.as_millis(),
        "{ladezeit_ms} ms > {gemessen:?}"
    );
    assert_eq!(inhalt["defekte_zeilen"], 0);
    assert_eq!(stderr, "");
}

#[test]
fn us3_unlesbare_startzeit_endet_mit_exit_zwei_und_nennt_die_variable() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    ordner_a(wurzel.path());
    let ordner = wurzel.path().display().to_string();

    let ende = aufrufen_mit(
        &[&ordner, "--port", "0", "--kein-browser"],
        &[("LOCALCHATA_JETZT", "gestern"), ("TZ", "Europe/Berlin")],
    );

    assert_eq!(ende.exit, Some(2), "stderr: {}", ende.stderr);
    let zeilen: Vec<&str> = ende.stderr.lines().collect();
    assert_eq!(zeilen.len(), 1, "{:?}", ende.stderr);
    assert!(zeilen[0].contains("LOCALCHATA_JETZT"), "{}", zeilen[0]);
    assert_eq!(ende.stdout, "");
}

#[test]
fn nicht_lesbare_ablage_beendet_den_start_mit_exit_eins_und_nennt_den_ordner() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    datei(
        wurzel.path(),
        "channels",
        "eine Datei, wo der Ordner sein müsste",
    );
    let ordner = wurzel.path().display().to_string();

    let ende = aufrufen_mit(
        &[
            &ordner,
            "--port",
            "0",
            "--host",
            "127.0.0.1",
            "--kein-browser",
        ],
        &[("TZ", "Europe/Berlin")],
    );

    assert_eq!(ende.exit, Some(1), "stderr: {}", ende.stderr);
    let zeilen: Vec<&str> = ende.stderr.lines().collect();
    assert_eq!(zeilen.len(), 1, "{:?}", ende.stderr);
    assert!(zeilen[0].contains(&ordner), "{}", zeilen[0]);
    assert_eq!(ende.stdout, "", "keine Startzeile");
}

/// Zwei heile Sätze, dann ein dritter, mitten im Zeichen `ä` abgeschnitten (177 Bytes, Code-Review B1).
const ABGESCHNITTEN: &[u8] = b"@ 2026-10-27T10:00:00.000+01:00 shoff@127.0.0.1\n  Wie weit?\n\n@ 2026-10-27T10:00:05.000+01:00 claude@10.0.0.5\n  Fertig.\n\n@ 2026-10-27T10:00:06.000+01:00 claude@10.0.0.5\n  Scan l\xC3";

fn datei_bytes(ordner: &Path, pfad: &str, inhalt: &[u8]) {
    let ziel = ordner.join(pfad);
    fs::create_dir_all(ziel.parent().expect("Elternordner")).expect("Ordner");
    fs::write(ziel, inhalt).expect("Datei");
}

#[tokio::test]
async fn b1a_mitten_im_zeichen_abgeschnitten_verliert_nur_die_zeile_nicht_den_tag() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    datei_bytes(
        wurzel.path(),
        "channels/allgemein/2026-10-27.log",
        ABGESCHNITTEN,
    );
    assert_eq!(ABGESCHNITTEN.len(), 177, "Datei wie in der Abnahme-Probe");

    let (inhalt, stderr, _) =
        kennzahlen_und_ausgaben(wurzel.path(), "2026-10-27T12:00:00+01:00").await;

    assert_eq!(
        inhalt["nachrichten"], 3,
        "zwei heile und der verkürzte Satz"
    );
    assert_eq!(inhalt["log_bytes"], 177);
    assert_eq!(inhalt["defekte_zeilen"], 1);
    let zeilen: Vec<&str> = stderr.lines().collect();
    assert_eq!(zeilen.len(), 1, "{stderr:?}");
    assert_eq!(ort(zeilen[0]), "channels/allgemein/2026-10-27.log:8");
}

#[tokio::test]
async fn b1b_ungueltiges_byte_im_ersten_satz_ist_ein_defekt_seiner_zeile() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    datei_bytes(
        wurzel.path(),
        "channels/allgemein/2026-10-27.log",
        b"@ 2026-10-27T10:00:00.000+01:00 shoff@127.0.0.1\n  Wie we\xffit?\n\n@ 2026-10-27T10:00:05.000+01:00 claude@10.0.0.5\n  Fertig.\n\n",
    );

    let (inhalt, stderr, _) =
        kennzahlen_und_ausgaben(wurzel.path(), "2026-10-27T12:00:00+01:00").await;

    assert_eq!(inhalt["nachrichten"], 2);
    assert_eq!(inhalt["defekte_zeilen"], 1);
    let zeilen: Vec<&str> = stderr.lines().collect();
    assert_eq!(zeilen.len(), 1, "{stderr:?}");
    assert_eq!(ort(zeilen[0]), "channels/allgemein/2026-10-27.log:2");
}

#[tokio::test]
async fn ungueltiges_byte_in_der_kopfzeile_ist_ein_defekt_des_satzes_der_nachbar_bleibt() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    datei_bytes(
        wurzel.path(),
        "channels/allgemein/2026-10-27.log",
        b"@ 2026-10-27T10:00:00.000+01:00 sh\xc3ff@127.0.0.1\n  eins\n\n@ 2026-10-27T10:00:05.000+01:00 claude@10.0.0.5\n  zwei\n\n",
    );

    let (inhalt, stderr, _) =
        kennzahlen_und_ausgaben(wurzel.path(), "2026-10-27T12:00:00+01:00").await;

    assert_eq!(inhalt["nachrichten"], 1);
    assert_eq!(inhalt["defekte_zeilen"], 1);
    assert_eq!(orte(&stderr), ["channels/allgemein/2026-10-27.log:1"]);
}

#[cfg(unix)]
#[test]
fn a2_startzeit_die_kein_utf8_ist_endet_mit_exit_zwei_und_nennt_die_variable() {
    use std::os::unix::ffi::OsStrExt;
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let mut kind = std::process::Command::new(gemeinsam::PROGRAMM)
        .arg(wurzel.path())
        .args(["--port", "0", "--host", "127.0.0.1", "--kein-browser"])
        .env("TZ", "Europe/Berlin")
        .env("LOCALCHATA_JETZT", std::ffi::OsStr::from_bytes(b"\xff"))
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("localchatA startet");
    let status = gemeinsam::warten(&mut kind, std::time::Duration::from_secs(5));
    if status.is_none() {
        let _ = kind.kill();
    }
    let ausgabe = kind.wait_with_output().expect("Ausgabe");
    let stderr = String::from_utf8_lossy(&ausgabe.stderr);

    assert_eq!(status.and_then(|s| s.code()), Some(2), "stderr: {stderr}");
    let zeilen: Vec<&str> = stderr.lines().collect();
    assert_eq!(zeilen.len(), 1, "{stderr:?}");
    assert!(zeilen[0].contains("LOCALCHATA_JETZT"), "{}", zeilen[0]);
    assert!(ausgabe.stdout.is_empty(), "keine Startzeile");
}
