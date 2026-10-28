//! F0053 Panik-Haken (R00014, Kriterium „Panik bei gehaltener Sperre“): eigener Testprozess, weil der Haken
//! prozessweit gilt. Während der Test die Protokollsperre hält, gerät ein anderer Thread in Panik.

use localchata::protokoll::senke::Speicher;
use localchata::protokoll::{Protokoll, Stufe, haken_setzen};
use localchata::uhr::{GestellteUhr, Zeit};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[test]
fn panik_bei_gehaltener_sperre_kehrt_ohne_verklemmung_zurueck_steht_auf_stderr_und_zaehlt() {
    let stderr = Arc::new(Mutex::new(Vec::<String>::new()));
    let aufzeichnen = Arc::clone(&stderr);
    let speicher = Speicher::neu();
    let protokoll = Arc::new(Protokoll::mit(
        Box::new(speicher.clone()),
        Arc::new(move |zeile: &str| {
            aufzeichnen.lock().expect("stderr").push(zeile.to_string());
        }),
        Arc::new(GestellteUhr::neu(
            Zeit::parse_from_rfc3339("2026-10-28T12:00:00+01:00").expect("Zeit"),
        )),
        Stufe::Information,
    ));
    haken_setzen(Arc::clone(&protokoll));
    let vorher = protokoll.zaehlerstand().exceptions;

    let gehalten = protokoll.sperre_halten();
    let anderer = std::thread::Builder::new()
        .name("paniker".into())
        .spawn(|| panic!("Panik bei gehaltener Sperre (Probe)"))
        .expect("Thread");
    let (fertig, ende) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = fertig.send(anderer.join().is_err());
    });
    let ergebnis = ende.recv_timeout(Duration::from_secs(1));
    drop(gehalten);

    assert_eq!(ergebnis, Ok(true), "der Thread endet mit Panik binnen 1 s");
    let zeilen = stderr.lock().expect("stderr").clone();
    assert_eq!(zeilen.len(), 1, "{zeilen:#?}");
    assert!(
        zeilen[0].contains(" EXCEPTION programmfehler Panik bei gehaltener Sperre (Probe) | ort=tests/panik_haken.rs:"),
        "{zeilen:#?}"
    );
    assert!(zeilen[0].contains("thread=paniker"), "{zeilen:#?}");
    assert!(speicher.zeilen().is_empty(), "die Sperre war belegt");
    assert_eq!(protokoll.zaehlerstand().exceptions, vorher + 1);
}
