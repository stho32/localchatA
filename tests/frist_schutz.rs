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

/// Setzt beim Ende die Rechte eines Ordners zurück, damit der Wegwerf-Ordner sich entfernen lässt.
struct Schreibschutz(std::path::PathBuf);

impl Schreibschutz {
    fn setzen(ordner: &Path) -> Schreibschutz {
        use std::os::unix::fs::PermissionsExt;
        assert!(
            !rustix::process::geteuid().is_root(),
            "chmod a-w wirkt für root nicht — als normaler Benutzer fahren"
        );
        fs::set_permissions(ordner, fs::Permissions::from_mode(0o555)).expect("chmod a-w");
        Schreibschutz(ordner.to_path_buf())
    }
}

impl Drop for Schreibschutz {
    fn drop(&mut self) {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&self.0, fs::Permissions::from_mode(0o755));
    }
}

fn frist_zeilen<'a>(zeilen: impl IntoIterator<Item = &'a String>) -> Vec<String> {
    zeilen
        .into_iter()
        .filter(|zeile| zeile.contains(" frist "))
        .map(|zeile| {
            zeile
                .split_once(' ')
                .map_or(zeile.as_str(), |(_, rest)| rest)
                .to_string()
        })
        .collect()
}

const NICHT_ENTFALLEN: &str = "ERROR frist Tag nicht entfallen — erneuter Versuch beim nächsten Anlass | channel=allgemein datei=channels/allgemein/2026-10-20.log grund=\"";

/// US-22, Kriterien „Nicht löschbar“ (R00008) und „Nicht löschbar bleibt laut, die Ruhe gilt für beide“ (R00017):
/// je Anlass genau eine Zeile in Datei und auf stderr, nie je Takt (Z3 4 s, Z4 3 s).
#[test]
fn us22_eine_nicht_loeschbare_datei_meldet_sich_einmal_je_anlass() {
    use gemeinsam::frist::{KURZ_VOR_MITTERNACHT, tag_da};
    use gemeinsam::protokoll::zeilen;
    use std::time::{Duration, Instant};
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let ordner = wurzel.path();
    tag_anlegen(ordner, "allgemein", "2026-10-20", 1);
    tag_anlegen(ordner, "zweiter", "2026-10-20", 1);
    let _schutz = Schreibschutz::setzen(&ordner.join("channels/allgemein"));

    let mut instanz = starten_um(ordner, KURZ_VOR_MITTERNACHT);
    let startzeile = Instant::now();
    let stderr = instanz.stderr_mitschreiben();

    assert!(tag_da(ordner, "allgemein", "2026-10-20"));
    assert!(!tag_da(ordner, "zweiter", "2026-10-20"));
    let am_start = frist_zeilen(&zeilen(ordner, "2026-10-28"));
    assert_eq!(
        am_start
            .iter()
            .filter(|z| z.starts_with(NICHT_ENTFALLEN))
            .count(),
        1,
        "{am_start:#?}"
    );
    assert!(
        am_start.iter().any(|z| z.starts_with("INFORMATION frist ")
            && z.ends_with("| channel=zweiter anzahl=1 grund=start")),
        "{am_start:#?}"
    );
    let auf_stderr = |stderr: &std::sync::Mutex<Vec<String>>| {
        frist_zeilen(stderr.lock().expect("stderr").iter())
    };
    let mitternacht = loop {
        let neu = frist_zeilen(&zeilen(ordner, "2026-10-29"));
        if !neu.is_empty() || startzeile.elapsed() > Duration::from_secs(10) {
            break startzeile.elapsed();
        }
        std::thread::sleep(Duration::from_millis(25));
    };
    println!("Z3 gemessen: {} ms", mitternacht.as_millis());
    assert!(
        mitternacht <= Duration::from_secs(4),
        "{mitternacht:?} > Z3"
    );
    std::thread::sleep(Duration::from_millis(100));
    let nach_mitternacht = frist_zeilen(&zeilen(ordner, "2026-10-29"));
    assert_eq!(nach_mitternacht.len(), 1, "{nach_mitternacht:#?}");
    assert!(
        nach_mitternacht[0].starts_with(NICHT_ENTFALLEN),
        "{nach_mitternacht:#?}"
    );
    let bis_dahin = auf_stderr(&stderr);
    assert_eq!(bis_dahin.len(), 2, "{bis_dahin:#?}");
    assert!(
        bis_dahin.iter().all(|z| z.starts_with(NICHT_ENTFALLEN)),
        "{bis_dahin:#?}"
    );

    std::thread::sleep(Duration::from_secs(3));

    assert_eq!(frist_zeilen(&zeilen(ordner, "2026-10-29")).len(), 1);
    assert_eq!(auf_stderr(&stderr).len(), 2, "{:#?}", auf_stderr(&stderr));
    assert!(tag_da(ordner, "allgemein", "2026-10-20"));
}
