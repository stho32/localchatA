//! R00029 F0097 „Eingang als Symlink“ (M26) und „Start leert den Eingang“ (Teil ohne Hochladen, B0431) über das
//! echte Programm.

mod gemeinsam;

use gemeinsam::protokoll::alle_zeilen;
use gemeinsam::{aufrufen_mit, lokal_starten};
use std::os::unix::fs::symlink;
use std::path::Path;

const LOKAL: [&str; 5] = ["--port", "0", "--host", "127.0.0.1", "--kein-browser"];

fn aufrufen_in(ordner: &Path) -> gemeinsam::Ende {
    let ordner = ordner.display().to_string();
    let mut argumente = vec![ordner.as_str()];
    argumente.extend_from_slice(&LOKAL);
    aufrufen_mit(&argumente, &[])
}

/// `.eingang` als Symlink auf einen fremden Ordner mit einer Datei → Exit 2, stderr nennt den Pfad, der fremde
/// Ordner und seine Datei sind unverändert.
#[test]
fn ein_eingang_als_symlink_endet_mit_exit_2_und_loescht_nichts() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let fremd = tempfile::tempdir().expect("fremder Ordner");
    std::fs::write(fremd.path().join("wichtig"), b"fremd").expect("fremde Datei");
    symlink(fremd.path(), wurzel.path().join(".eingang")).expect("Symlink");

    let ende = aufrufen_in(wurzel.path());

    assert_eq!(ende.exit, Some(2), "{}", ende.stderr);
    let pfad = wurzel.path().join(".eingang").display().to_string();
    assert!(ende.stderr.contains(&pfad), "{}", ende.stderr);
    assert_eq!(ende.stderr.trim_end().lines().count(), 1, "{}", ende.stderr);
    assert_eq!(
        std::fs::read(fremd.path().join("wichtig")).expect("bleibt"),
        b"fremd"
    );
    assert!(
        std::fs::symlink_metadata(wurzel.path().join(".eingang"))
            .expect("Link")
            .file_type()
            .is_symlink()
    );
}

/// `.eingang` als reguläre Datei → Exit 2, die Datei bleibt.
#[test]
fn ein_eingang_als_datei_endet_mit_exit_2() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    std::fs::write(wurzel.path().join(".eingang"), b"keine Ablage").expect("Datei");

    let ende = aufrufen_in(wurzel.path());

    assert_eq!(ende.exit, Some(2), "{}", ende.stderr);
    assert_eq!(
        std::fs::read(wurzel.path().join(".eingang")).expect("bleibt"),
        b"keine Ablage"
    );
}

/// Fehlt `.eingang`, legt der Start ihn an; liegen dort Dateien, leert er nur die regulären — Symlink und
/// Unterordner bleiben mit je einer `WARNING laden`, das Ziel des Symlinks bleibt unberührt.
#[test]
fn der_start_legt_den_eingang_an_und_leert_nur_regulaere_dateien() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    drop(lokal_starten(wurzel.path()));
    assert!(wurzel.path().join(".eingang").is_dir());

    let fremd = tempfile::tempdir().expect("fremder Ordner");
    std::fs::write(fremd.path().join("ziel"), b"fremd").expect("Ziel");
    let eingang = wurzel.path().join(".eingang");
    std::fs::write(eingang.join("k3v7q2m5xpc4h6n6tewaz2b5yr"), b"fertig").expect("fertig");
    std::fs::write(eingang.join("aaaaaaaaaaaaaaaaaaaaaaaaaa.teil"), b"halb").expect("Teil");
    symlink(fremd.path().join("ziel"), eingang.join("zeiger")).expect("Symlink");
    std::fs::create_dir(eingang.join("unter")).expect("Unterordner");

    drop(lokal_starten(wurzel.path()));

    let mut uebrig: Vec<String> = std::fs::read_dir(&eingang)
        .expect("Eingang")
        .map(|eintrag| {
            eintrag
                .expect("Eintrag")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    uebrig.sort();
    assert_eq!(uebrig, vec!["unter", "zeiger"]);
    assert_eq!(
        std::fs::read(fremd.path().join("ziel")).expect("Ziel"),
        b"fremd"
    );
    let warnungen: Vec<String> = alle_zeilen(wurzel.path())
        .into_iter()
        .filter(|zeile| zeile.contains(" WARNING laden ") && zeile.contains(".eingang/"))
        .collect();
    assert_eq!(warnungen.len(), 2, "{warnungen:#?}");
}
