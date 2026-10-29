//! R00024 B0485 (ADR-0010 Ergänzung „Karte #35 Nacharbeit, Prozessende“): das echte Programm endet nach SIGTERM,
//! während eine Namensauflösung im NSS-Modul hängt, in weniger als 1 s mit Exit 0 und vollständigem
//! Betriebsprotokoll.
//!
//! **L2r** — nur auf Linux mit `nss-mdns`, `#[ignore]`: nicht im Standard-Gate, ausdrücklich gefahren mit
//! `cargo test --test prozessende -- --ignored`. Ohne `LOCALCHATA_TESTNAMEN`: die Auflösung geht über die echte
//! Namensauskunft des Rechners.
#![cfg(target_os = "linux")]

mod gemeinsam;

use std::net::ToSocketAddrs;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Grenze aus dem Kriterium „Prozessende < 1 s nach dem Signal“.
const GRENZE: Duration = Duration::from_secs(1);

/// Ab dieser Dauer hängt eine Auflösung im Modul länger als die Frist des Grenzschutzes (1 s) — erst dann
/// prüft der Test das Prozessende überhaupt (ADR-0010, Vorbedingung ≥ 2 s).
const HAENGT_MINDESTENS: Duration = Duration::from_secs(2);

#[test]
#[ignore = "L2r: braucht nss-mdns; cargo test --test prozessende -- --ignored"]
fn sigterm_waehrend_einer_haengenden_mdns_aufloesung_endet_unter_einer_sekunde_mit_exit_null() {
    vorbedingung_nss_mdns();

    prozessende_waehrend_aufloesung(&zufallsname());
}

/// Derselbe Ablauf mit dem Namen aus US-14 wörtlich (avahi kann ihn negativ zwischenspeichern; dann hängt
/// nichts, und das Ende muss erst recht unter der Grenze liegen).
#[test]
#[ignore = "L2r: braucht nss-mdns; cargo test --test prozessende -- --ignored"]
fn sigterm_waehrend_gibtsnicht_local_endet_unter_einer_sekunde_mit_exit_null() {
    vorbedingung_nss_mdns();

    prozessende_waehrend_aufloesung("gibtsnicht.local");
}

fn prozessende_waehrend_aufloesung(name: &str) {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let ablage = tempfile::tempdir().expect("Ordner für stdout");
    let stdout_datei = ablage.path().join("stdout");
    let mut laufend = Laufend(programm_starten(wurzel.path(), &stdout_datei));
    let kind = &mut laufend.0;
    let adresse = adresse_abwarten(kind, &stdout_datei);
    let ziel: std::net::SocketAddr = adresse
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .parse()
        .expect("Adresse ip:port");

    let antwort = gemeinsam::roh_anfragen(
        ziel,
        &format!(
            "GET /api/gesundheit HTTP/1.1\r\nHost: {name}:{}",
            ziel.port()
        ),
        "",
    );
    assert_eq!(antwort.status, 403, "{antwort:?}");

    let signal = Instant::now();
    gemeinsam::signal_senden(kind, "TERM");
    let status =
        gemeinsam::warten(kind, Duration::from_secs(15)).expect("Programm endet nach dem Signal");
    let dauer = signal.elapsed();

    assert!(
        dauer < GRENZE,
        "Prozessende {dauer:?} nach SIGTERM, Grenze {GRENZE:?} ({name})"
    );
    assert_eq!(status.code(), Some(0), "{status:?}");
    let zeilen = gemeinsam::protokoll::alle_zeilen(wurzel.path());
    let letzte = zeilen.last().expect("Protokoll hat Zeilen");
    assert!(
        letzte.contains(" INFORMATION ende Instanz beendet"),
        "letzte Zeile: {letzte} — {zeilen:#?}"
    );
    let ausgabe = std::fs::read_to_string(&stdout_datei).expect("stdout lesen");
    let startzeile = format!(
        "localchatA läuft für {} unter {adresse}\n",
        wurzel.path().display()
    );
    assert!(
        ausgabe.contains(&startzeile),
        "Startzeile vollständig in stdout: {ausgabe:?}"
    );
}

/// Hält den Prozess; endet der Test vorzeitig, wird er hart beendet.
struct Laufend(Child);

impl Drop for Laufend {
    fn drop(&mut self) {
        if let Ok(None) = self.0.try_wait() {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

/// Rot statt übersprungen, wenn der Rechner `.local` nicht über ein hängendes NSS-Modul auflöst.
fn vorbedingung_nss_mdns() {
    let name = zufallsname();
    let beginn = Instant::now();
    let _ = (name.as_str(), 80).to_socket_addrs();
    let dauer = beginn.elapsed();
    assert!(
        dauer >= HAENGT_MINDESTENS,
        "Vorbedingung fehlt: nss-mdns — Auflösung von {name} dauerte {dauer:?}, verlangt ≥ {HAENGT_MINDESTENS:?}"
    );
}

/// `gibtsnicht-<zufall>.local` — gegen den Negativ-Cache von avahi.
fn zufallsname() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Uhr nach 1970")
        .subsec_nanos();
    format!("gibtsnicht-{}-{nanos}.local", std::process::id())
}

/// stdout in eine Datei, damit nach dem Ende alles lesbar ist, was das Programm geschrieben hat.
fn programm_starten(ordner: &Path, stdout_datei: &Path) -> Child {
    let stdout = std::fs::File::create(stdout_datei).expect("stdout-Datei");
    Command::new(gemeinsam::PROGRAMM)
        .arg(ordner)
        .args(["--port", "0", "--host", "127.0.0.1", "--kein-browser"])
        .env_remove("LOCALCHATA_TESTNAMEN")
        .stdout(stdout)
        .stderr(Stdio::inherit())
        .spawn()
        .expect("localchatA startet")
}

/// Wartet auf die vollständige Startzeile (mit Zeilenende) und liefert die Adresse daraus.
fn adresse_abwarten(kind: &mut Child, stdout_datei: &Path) -> String {
    let ende = Instant::now() + Duration::from_secs(30);
    while Instant::now() < ende {
        let ausgabe = std::fs::read_to_string(stdout_datei).unwrap_or_default();
        if let Some((zeile, _)) = ausgabe.split_once('\n') {
            return zeile.rsplit(' ').next().expect("Adresse").to_string();
        }
        assert!(
            kind.try_wait().expect("Prozessstatus").is_none(),
            "Programm endete vor der Startzeile"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    let _ = kind.kill();
    panic!("keine Startzeile binnen 30 s");
}
