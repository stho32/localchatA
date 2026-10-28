//! F0054 Abgewiesene Körper und Debug je Anfrage — über das echte Programm (R00014, US-11, Kriterien
//! „Abgewiesene Körper“, „Debug je Anfrage“).

mod gemeinsam;

use gemeinsam::protokoll::{mit, wert, zeilen};
use gemeinsam::{Instanz, gesundheit, starten_mit};
use std::io::Write;
use std::path::Path;
use std::time::{Duration, Instant};

const LOKAL: [&str; 5] = ["--port", "0", "--host", "127.0.0.1", "--kein-browser"];

fn starten_um(ordner: &Path, umgebung: &[(&str, &str)]) -> Instanz {
    let mut alle = vec![
        ("LOCALCHATA_JETZT", "2026-10-28T12:00:00+01:00"),
        ("TZ", "Europe/Berlin"),
    ];
    alle.extend_from_slice(umgebung);
    starten_mit(ordner, &LOKAL, &alle)
}

/// Die vier Abweisungen aus US-11; liefert die Status der drei beantworteten.
async fn vier_abweisungen(instanz: &Instanz) -> Vec<u16> {
    let client = reqwest::Client::new();
    let ziel = format!("{}api/beitreten", instanz.adresse);
    let mut status = Vec::new();
    for (typ, koerper) in [
        (
            "application/json",
            format!("{{\"name\":\"{}\"}}", "x".repeat(4_989)),
        ),
        ("text/plain", "{\"name\":\"beta\"}".to_string()),
        ("application/json", "{".to_string()),
    ] {
        let antwort = client
            .post(&ziel)
            .header("Content-Type", typ)
            .body(koerper)
            .send()
            .await
            .expect("Antwort");
        status.push(antwort.status().as_u16());
    }
    let mut roh = std::net::TcpStream::connect(("127.0.0.1", instanz.port())).expect("Verbindung");
    roh.write_all(
        b"POST /api/beitreten HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: 1000\r\n\r\n{\"name\":\"",
    )
    .expect("gesendet");
    drop(roh);
    status
}

/// Wartet, bis der Health-Check `abgewiesen.abgebrochen ≥ 1` zeigt (höchstens 5 s), und liefert ihn.
async fn abgewiesen_nach_abbruch(instanz: &Instanz) -> serde_json::Value {
    let ende = Instant::now() + Duration::from_secs(5);
    loop {
        let (_, inhalt) = gesundheit(&instanz.adresse).await;
        if inhalt["abgewiesen"]["abgebrochen"] != 0 || Instant::now() > ende {
            return inhalt["abgewiesen"].clone();
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

#[tokio::test]
async fn us11_abgewiesene_koerper_werden_je_art_gezaehlt() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let instanz = starten_um(wurzel.path(), &[]);

    let status = vier_abweisungen(&instanz).await;
    let abgewiesen = abgewiesen_nach_abbruch(&instanz).await;

    assert_eq!(status, [413, 415, 400]);
    assert_eq!(
        abgewiesen,
        serde_json::json!({"zu_gross":1,"kein_json":1,"kaputt":1,"abgebrochen":1})
    );
}

#[tokio::test]
async fn debug_je_anfrage_eine_zeile_je_abweisung_mit_route_status_herkunft_und_art() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let instanz = starten_um(wurzel.path(), &[("LOCALCHATA_PROTOKOLL_STUFE", "debug")]);

    let status = vier_abweisungen(&instanz).await;
    let _ = abgewiesen_nach_abbruch(&instanz).await;
    let unbekannt = reqwest::get(format!("{}api/gibtsnicht%0aX", instanz.adresse))
        .await
        .expect("Antwort")
        .status()
        .as_u16();

    assert_eq!(status, [413, 415, 400]);
    assert_eq!(unbekannt, 404);
    let datei = zeilen(wurzel.path(), "2026-10-28");
    let koerper: Vec<String> = datei
        .iter()
        .filter(|z| {
            z.contains(" DEBUG ") && wert(z, "route").as_deref() == Some("POST /api/beitreten")
        })
        .cloned()
        .collect();
    assert_eq!(koerper.len(), 4, "{datei:#?}");
    let mit_status = |code: &str| {
        koerper
            .iter()
            .filter(|z| wert(z, "status").as_deref() == Some(code))
            .cloned()
            .collect::<Vec<_>>()
    };
    for code in ["413", "415", "400"] {
        let gefunden = mit_status(code);
        assert_eq!(gefunden.len(), 1, "{code}: {koerper:#?}");
        assert_eq!(wert(&gefunden[0], "herkunft").as_deref(), Some("127.0.0.1"));
    }
    assert_eq!(
        wert(&mit_status("415")[0], "typ").as_deref(),
        Some("text/plain")
    );
    let zu_gross = &mit_status("413")[0];
    assert_eq!(wert(zu_gross, "art").as_deref(), Some("zu_gross"));
    assert!(wert(zu_gross, "bytes").is_some(), "{zu_gross}");
    assert_eq!(wert(zu_gross, "verworfen").as_deref(), Some("vollständig"));
    assert!(
        koerper
            .iter()
            .any(|z| wert(z, "art").as_deref() == Some("abgebrochen")),
        "{koerper:#?}"
    );
    let pfad = mit(&datei, "DEBUG", "verbindung").len();
    assert!(pfad > 0, "Verbindungen auf Debug: {datei:#?}");
    let unbekannte: Vec<&String> = datei
        .iter()
        .filter(|z| z.contains(" DEBUG ") && z.contains("gibtsnicht"))
        .collect();
    assert_eq!(unbekannte.len(), 1, "{datei:#?}");
    assert!(
        unbekannte[0].contains("/api/gibtsnicht\\u{a}X"),
        "{}",
        unbekannte[0]
    );
    assert_eq!(wert(unbekannte[0], "status").as_deref(), Some("404"));
}
