//! R00029 F0097 B0583 — die Wurzel `channels` muss ein echter Ordner sein (ADR-0046 Ergänzung „`channels` muss ein
//! echter Ordner sein“, Regeln 1–4), über das echte Programm.

mod gemeinsam;

use gemeinsam::Absender;
use gemeinsam::anhaenge::{ablegen, starten};
use gemeinsam::aufrufen_mit;
use gemeinsam::senden::{beitreten, roh_senden};
use serde_json::json;
use std::collections::BTreeMap;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const LOKAL: [&str; 5] = ["--port", "0", "--host", "127.0.0.1", "--kein-browser"];

fn aufrufen_in(ordner: &Path) -> gemeinsam::Ende {
    let ordner = ordner.display().to_string();
    let mut argumente = vec![ordner.as_str()];
    argumente.extend_from_slice(&LOKAL);
    aufrufen_mit(&argumente, &[])
}

/// Alle Einträge unter `wurzel` mit Inhalt (Dateien) bzw. Art (Ordner, Links) — für „byte-gleich“.
fn abbild(wurzel: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut abbild = BTreeMap::new();
    let mut offen = vec![wurzel.to_path_buf()];
    while let Some(ordner) = offen.pop() {
        for eintrag in std::fs::read_dir(&ordner).expect("lesbar") {
            let pfad = eintrag.expect("Eintrag").path();
            let art = std::fs::symlink_metadata(&pfad).expect("Art").file_type();
            let inhalt = if art.is_dir() {
                offen.push(pfad.clone());
                b"<ordner>".to_vec()
            } else if art.is_symlink() {
                b"<link>".to_vec()
            } else {
                std::fs::read(&pfad).expect("Inhalt")
            };
            abbild.insert(
                pfad.strip_prefix(wurzel).expect("darunter").to_path_buf(),
                inhalt,
            );
        }
    }
    abbild
}

/// Probe (wörtlich): `channels` als Symlink auf `ziel/channels` (mit einem Channel darin) → Start endet mit Exit 2
/// binnen 2 s, stderr nennt `channels` und „Instanzordner“, kein `logs/`, `ziel/` byte-gleich; nichts angelegt.
#[test]
fn channels_als_symlink_endet_mit_exit_2_und_aendert_nichts() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let ziel = tempfile::tempdir().expect("Ziel");
    std::fs::create_dir_all(ziel.path().join("channels/allgemein")).expect("Channel");
    std::fs::write(
        ziel.path().join("channels/allgemein/2026-10-29.log"),
        b"Inhalt",
    )
    .expect("Tagesdatei");
    symlink(ziel.path().join("channels"), wurzel.path().join("channels")).expect("Symlink");
    let vorher = abbild(ziel.path());

    let beginn = Instant::now();
    let ende = aufrufen_in(wurzel.path());

    assert_eq!(ende.exit, Some(2), "{}", ende.stderr);
    assert!(
        beginn.elapsed() < Duration::from_secs(2),
        "{:?}",
        beginn.elapsed()
    );
    let pfad = wurzel.path().join("channels").display().to_string();
    assert!(
        ende.stderr.contains(&pfad) && ende.stderr.contains("Instanzordner"),
        "{}",
        ende.stderr
    );
    assert_eq!(ende.stderr.trim_end().lines().count(), 1, "{}", ende.stderr);
    assert!(!wurzel.path().join("logs").exists());
    assert!(!wurzel.path().join(".eingang").exists(), "nichts angelegt");
    assert_eq!(abbild(ziel.path()), vorher);
}

/// `channels` als reguläre Datei → Exit 2, die Datei bleibt.
#[test]
fn channels_als_datei_endet_mit_exit_2() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    std::fs::write(wurzel.path().join("channels"), b"kein Ordner").expect("Datei");

    let ende = aufrufen_in(wurzel.path());

    assert_eq!(ende.exit, Some(2), "{}", ende.stderr);
    assert!(ende.stderr.contains("Instanzordner"), "{}", ende.stderr);
    assert_eq!(
        std::fs::read(wurzel.path().join("channels")).expect("bleibt"),
        b"kein Ordner"
    );
    assert!(!wurzel.path().join("logs").exists());
}

/// Kontrolle: der Instanzordner als Symlink auf einen echten Ordner → Start ok, Senden mit Anhang 201, die Datei liegt
/// unter `<ziel>/channels/<c>/anhaenge/<tag>/`.
#[tokio::test]
async fn der_instanzordner_darf_ein_link_sein() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let ziel = wurzel.path().join("ziel");
    std::fs::create_dir_all(ziel.join("channels/allgemein")).expect("Channel");
    let link = wurzel.path().join("instanz");
    symlink(&ziel, &link).expect("Symlink");
    let instanz = starten(&link, &[], &[]);
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, basis, "beta", Some("agent")).await;
    let id = ablegen(&beta, basis, &token, "bericht.pdf", vec![0u8; 1_000]).await;

    let gesendet = roh_senden(
        &beta,
        basis,
        Some(&token),
        "allgemein",
        "application/json",
        json!({"text": "mit Anhang", "anhaenge": [id]})
            .to_string()
            .into_bytes(),
    )
    .await;

    assert_eq!(gesendet.status, 201, "{}", gesendet.json);
    let tag = &gesendet.json["nachricht"]["zeit"].as_str().expect("zeit")[..10];
    let dateien: Vec<_> =
        std::fs::read_dir(ziel.join(format!("channels/allgemein/anhaenge/{tag}")))
            .expect("Tagordner im Ziel")
            .collect();
    assert_eq!(dateien.len(), 1);
}
