//! F0038 Löschschutz und Fehlerwege (R00008, US-21 bis US-23) mit den Frist-Zeilen im Betriebsprotokoll
//! (R00017) — über das echte Programm. Die Kriterien mit `chmod a-w` laufen nicht als root (dort wirkt es nicht).
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::frist::{MITTAG, datei_anlegen, starten_um, tag_anlegen};
use std::fs;
use std::path::Path;

fn inhalt(ordner: &Path, pfad: &str) -> Vec<u8> {
    fs::read(ordner.join(pfad)).unwrap_or_else(|fehler| panic!("{pfad}: {fehler}"))
}

/// US-21, Kriterium „Erlaubnisliste“: nur die reguläre Tagesdatei geht; Symlink, Ordner, `.bak`, Notizen, das Ziel
/// des Symlinks und `direkt/` bleiben byte-gleich.
#[test]
fn us21_fremde_dateien_bleiben() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let ordner = wurzel.path();
    tag_anlegen(ordner, "allgemein", "2026-10-01", 1);
    datei_anlegen(ordner, "aussen/ziel.log", b"Archiv\n");
    std::os::unix::fs::symlink(
        ordner.join("aussen/ziel.log"),
        ordner.join("channels/allgemein/2026-10-02.log"),
    )
    .expect("Symlink");
    datei_anlegen(
        ordner,
        "channels/allgemein/2026-10-03.log/darin",
        b"Ordner\n",
    );
    datei_anlegen(
        ordner,
        "channels/allgemein/2026-10-04.log.bak",
        b"Sicherung\n",
    );
    datei_anlegen(ordner, "channels/allgemein/notizen.txt", b"Notiz\n");
    datei_anlegen(
        ordner,
        "direkt/a@x+b@y/2026-01-01.log",
        b"@ 2026-01-01T09:00:00.000+01:00 a@x\n  privat\n\n",
    );

    let _instanz = starten_um(ordner, MITTAG);

    assert!(!ordner.join("channels/allgemein/2026-10-01.log").exists());
    assert!(
        fs::symlink_metadata(ordner.join("channels/allgemein/2026-10-02.log"))
            .expect("Symlink")
            .file_type()
            .is_symlink()
    );
    assert_eq!(inhalt(ordner, "aussen/ziel.log"), b"Archiv\n");
    assert_eq!(
        inhalt(ordner, "channels/allgemein/2026-10-03.log/darin"),
        b"Ordner\n"
    );
    assert_eq!(
        inhalt(ordner, "channels/allgemein/2026-10-04.log.bak"),
        b"Sicherung\n"
    );
    assert_eq!(inhalt(ordner, "channels/allgemein/notizen.txt"), b"Notiz\n");
    assert_eq!(
        inhalt(ordner, "direkt/a@x+b@y/2026-01-01.log"),
        b"@ 2026-01-01T09:00:00.000+01:00 a@x\n  privat\n\n"
    );
}
