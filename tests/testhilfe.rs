//! B0007 Testhilfe: eine Instanz im Wegwerf-Ordner endet mit dem Drop — Prozess weg, Port frei, Ordner weg.

mod gemeinsam;

use std::net::TcpStream;

#[tokio::test]
async fn wegwerf_instanz_laeuft_bis_zum_drop_und_hinterlaesst_nichts() {
    let instanz = gemeinsam::wegwerf_starten();
    let port = instanz.port();
    let ordner = instanz.ordner.clone();
    assert!(ordner.is_dir());
    assert_eq!(gemeinsam::gesundheit(&instanz.adresse).await.0, 200);

    drop(instanz);

    assert!(
        TcpStream::connect(("127.0.0.1", port)).is_err(),
        "Port {port} antwortet noch"
    );
    assert!(
        !ordner.exists(),
        "Wegwerf-Ordner {} liegt noch",
        ordner.display()
    );
}

/// Code-Review A2: kommt keine Startzeile, wird der Test rot statt zu hängen, und der Prozess ist beendet.
#[cfg(unix)]
#[test]
fn ohne_startzeile_endet_das_warten_nach_der_zeitgrenze_mit_beendetem_prozess() {
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    let schweigt = Command::new("sleep")
        .arg("30")
        .stdout(Stdio::piped())
        .spawn()
        .expect("sleep startet");
    let pid = schweigt.id();
    let ordner = tempfile::tempdir().expect("Wegwerf-Ordner");
    let beginn = Instant::now();

    let ergebnis = std::panic::catch_unwind(|| {
        gemeinsam::auf_startzeile_warten_hoechstens(
            ordner.path(),
            schweigt,
            Duration::from_millis(300),
        )
    });

    let meldung = ergebnis
        .err()
        .and_then(|fehler| fehler.downcast_ref::<String>().cloned())
        .expect("Warten muss mit einer Meldung scheitern");
    assert!(meldung.contains("keine Startzeile"), "{meldung}");
    assert!(beginn.elapsed() < Duration::from_secs(5), "hat gehangen");
    assert!(
        !std::path::Path::new(&format!("/proc/{pid}")).exists(),
        "Prozess {pid} läuft noch"
    );
}
