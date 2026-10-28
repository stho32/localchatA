//! B0288 Lesefrist für den Körper — über das echte Programm (R00015 G4, Abnahme-Probe des Security-Reviews zu
//! #21; ADR-0010 und ADR-0043, Ergänzungen R00015). Ein Client, der seinen Körper nicht schickt, hält keinen
//! Platz länger als die Frist (10 s): er bekommt 408 `{fehler}` oder die Verbindung wird geschlossen.
//!
//! Zeitgrenze 13,5 s ab dem Startpunkt der jeweiligen Verbindung (34 % über den erwarteten ≈ 10,1 s, R00015);
//! geprüft durch Lesen mit Zeitgrenze, nicht mit `sleep`. Die Regel selbst (10,000 s) prüft der Unit-Test
//! in `src/http/koerper.rs` exakt.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::protokoll::{mit, wert, zeilen};
use gemeinsam::{Instanz, gesundheit, starten_mit};
use std::path::Path;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpSocket, TcpStream};
use tokio::time::Instant;

const GRENZE: Duration = Duration::from_millis(13_500);
const LOKAL: [&str; 5] = ["--port", "0", "--host", "127.0.0.1", "--kein-browser"];
/// Kopf eines Beitritts, der 4.096 Bytes Körper ankündigt.
const KOPF_4096: &str = "POST /api/beitreten HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: 4096\r\n\r\n";

fn starten_um(ordner: &Path, umgebung: &[(&str, &str)]) -> Instanz {
    let mut alle = vec![
        ("LOCALCHATA_JETZT", "2026-10-28T12:00:00+01:00"),
        ("TZ", "Europe/Berlin"),
    ];
    alle.extend_from_slice(umgebung);
    starten_mit(ordner, &LOKAL, &alle)
}

/// Die Gesamtgrenze ist `min(512, Deskriptoren / 2)`; darunter prüfte der Test eine andere Grenze — Abbruch
/// statt Befund (R00015, „Gemeinsam“).
fn deskriptoren_reichen() {
    let weich = rustix::process::getrlimit(rustix::process::Resource::Nofile).current;
    assert!(
        weich.is_none_or(|grenze| grenze >= 1024),
        "Abbruch, kein Befund: ulimit -n ist {weich:?}, der Test braucht mindestens 1024"
    );
}

async fn verbinden(port: u16, herkunft: &str) -> TcpStream {
    let socket = TcpSocket::new_v4().expect("Socket");
    socket
        .bind(format!("{herkunft}:0").parse().expect("Herkunft"))
        .expect("gebunden");
    socket
        .connect(format!("127.0.0.1:{port}").parse().expect("Ziel"))
        .await
        .expect("verbunden")
}

/// Wie eine Verbindung endete: Status und Inhalt einer Antwort (falls eine kam), `zu` = die Instanz hat
/// geschlossen, und wann — gemessen ab `seit`.
#[derive(Debug)]
struct Ausgang {
    status: Option<u16>,
    /// Kopfzeilen der Antwort, Namen klein geschrieben.
    kopf: Vec<String>,
    inhalt: String,
    zu: bool,
    nach: Duration,
}

/// Liest bis zum Schließen durch die Instanz, höchstens bis `seit + GRENZE`.
async fn bis_zum_schliessen(strom: &mut TcpStream, seit: Instant) -> Ausgang {
    let mut gelesen = Vec::new();
    let mut puffer = [0u8; 4096];
    let zu = loop {
        match tokio::time::timeout_at(seit + GRENZE, strom.read(&mut puffer)).await {
            Ok(Ok(0) | Err(_)) => break true,
            Ok(Ok(n)) => gelesen.extend_from_slice(&puffer[..n]),
            Err(_) => break false,
        }
    };
    let nach = seit.elapsed();
    let text = String::from_utf8_lossy(&gelesen).into_owned();
    let status = text
        .split(' ')
        .nth(1)
        .and_then(|code| code.parse().ok())
        .filter(|_| text.starts_with("HTTP/1.1 "));
    let (kopf, inhalt) = text.split_once("\r\n\r\n").unwrap_or_default();
    Ausgang {
        status,
        kopf: kopf.lines().skip(1).map(str::to_lowercase).collect(),
        inhalt: inhalt.to_string(),
        zu,
        nach,
    }
}

/// 408 mit `{fehler}` oder ohne Antwort geschlossen — beides innerhalb der Grenze.
fn rechtzeitig_beendet(ausgang: &Ausgang) -> bool {
    let antwort_passt = match ausgang.status {
        None => true,
        Some(408) => serde_json::from_str::<serde_json::Value>(&ausgang.inhalt)
            .is_ok_and(|json| json["fehler"].is_string()),
        Some(_) => false,
    };
    ausgang.zu && antwort_passt && ausgang.nach <= GRENZE
}

/// Öffnet eine Verbindung, sendet den Kopf und 10 Körperbytes; liefert sie mit dem Startpunkt der Uhr.
async fn schweigend_oeffnen(port: u16, herkunft: &str) -> (TcpStream, Instant) {
    let mut strom = verbinden(port, herkunft).await;
    strom
        .write_all(format!("{KOPF_4096}{{\"name\":\"a").as_bytes())
        .await
        .expect("Kopf und 10 Bytes");
    (strom, Instant::now())
}

/// Abnahme-Probe (R00015 G4, Security-Review zu #21): G = 160 schweigende Körper von 127.0.0.1 — jede
/// Verbindung bekommt 408 oder wird geschlossen, spätestens 13,5 s nach ihren 10 Bytes; danach, vor 15 s nach
/// dem Start der Probe, ist für 127.0.0.1 wieder Platz.
#[tokio::test]
async fn g4_160_schweigende_koerper_einer_herkunft_halten_keinen_platz_ueber_die_frist() {
    deskriptoren_reichen();
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let instanz = starten_um(wurzel.path(), &[]);
    let port = instanz.port();
    let g = localchata::start::verbindungen::JE_GEGENSTELLE;

    let probe = Instant::now();
    let mut offen = Vec::new();
    for _ in 0..g {
        offen.push(schweigend_oeffnen(port, "127.0.0.1").await);
    }
    let laeufe: Vec<_> = offen
        .into_iter()
        .map(|(mut strom, seit)| {
            tokio::spawn(async move { bis_zum_schliessen(&mut strom, seit).await })
        })
        .collect();
    let mut ausgaenge = Vec::new();
    for lauf in laeufe {
        ausgaenge.push(lauf.await.expect("Lesen"));
    }
    let danach = gesundheit(&instanz.adresse).await.0;
    let seit_probe = probe.elapsed();

    let zu_spaet: Vec<&Ausgang> = ausgaenge
        .iter()
        .filter(|ausgang| !rechtzeitig_beendet(ausgang))
        .collect();
    let laengste = ausgaenge.iter().map(|a| a.nach).max();
    eprintln!(
        "messung g4 verbindungen={g} laengste={laengste:?} mit_408={} seit_probe={seit_probe:?}",
        ausgaenge.iter().filter(|a| a.status == Some(408)).count()
    );
    assert!(
        zu_spaet.is_empty(),
        "{} zu spät: {zu_spaet:#?}",
        zu_spaet.len()
    );
    assert_eq!(danach, 200);
    assert!(seit_probe < Duration::from_secs(15), "{seit_probe:?}");
}

/// Tröpfler: der Körper eines Beitritts zu 4.096 Bytes, ein Byte je Sekunde → 408 oder geschlossen spätestens
/// 13,5 s nach dem ersten Körperbyte (die Frist gilt gesamt, nicht je Stück).
#[tokio::test]
async fn g4_ein_troepfler_mit_einem_byte_je_sekunde_wird_nach_der_frist_beendet() {
    let instanz = starten_um(tempfile::tempdir().expect("Ordner").path(), &[]);
    let strom = verbinden(instanz.port(), "127.0.0.1").await;
    let (mut lesen, mut schreiben) = strom.into_split();
    schreiben
        .write_all(format!("{KOPF_4096}{{").as_bytes())
        .await
        .expect("Kopf und erstes Körperbyte");
    let seit = Instant::now();
    let troepfeln = tokio::spawn(async move {
        let mut takt =
            tokio::time::interval_at(seit + Duration::from_secs(1), Duration::from_secs(1));
        for _ in 1..4096 {
            takt.tick().await;
            if schreiben.write_all(b" ").await.is_err() {
                return;
            }
        }
    });

    let mut gelesen = Vec::new();
    let mut puffer = [0u8; 4096];
    let zu = loop {
        match tokio::time::timeout_at(seit + GRENZE, lesen.read(&mut puffer)).await {
            Ok(Ok(0) | Err(_)) => break true,
            Ok(Ok(n)) => gelesen.extend_from_slice(&puffer[..n]),
            Err(_) => break false,
        }
    };
    let nach = seit.elapsed();
    troepfeln.abort();

    let text = String::from_utf8_lossy(&gelesen);
    eprintln!("messung troepfler nach={nach:?}");
    assert!(zu, "nach {nach:?} noch offen");
    assert!(nach <= GRENZE, "{nach:?}");
    assert!(
        text.is_empty() || text.starts_with("HTTP/1.1 408"),
        "{text}"
    );
}

/// Kontrolle: ein gültiger Beitritt mit 248 Bytes Körper, in 8 Stücken zu 31 Bytes je 1 s Abstand (fertig nach
/// ≈ 7 s, unter der Frist) → 201.
#[tokio::test]
async fn g4_ein_langsamer_gueltiger_koerper_unter_der_frist_tritt_bei() {
    let instanz = starten_um(tempfile::tempdir().expect("Ordner").path(), &[]);
    let koerper = format!("{:<248}", "{\"name\":\"kontrolle\"}");
    let mut strom = verbinden(instanz.port(), "127.0.0.1").await;
    strom
        .write_all(
            b"POST /api/beitreten HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: 248\r\nConnection: close\r\n\r\n",
        )
        .await
        .expect("Kopf");
    let beginn = Instant::now();
    for (nummer, stueck) in koerper.as_bytes().chunks(31).enumerate() {
        if nummer > 0 {
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        strom.write_all(stueck).await.expect("Stück");
    }
    let gesendet = beginn.elapsed();
    let ausgang = bis_zum_schliessen(&mut strom, Instant::now()).await;

    eprintln!("messung kontrolle gesendet_nach={gesendet:?}");
    assert_eq!(ausgang.status, Some(201), "{ausgang:?}");
}

/// ADR-0043, Ergänzung R00015, erster Test: 3 schweigende Körper → je 408 mit `Connection: close` und dem Text
/// der ADR; `abgewiesen.zu_langsam = 3`, die übrigen Arten 0; genau eine Körper-Episode (beginnt, endet mit
/// `zu_langsam 3`) für `POST /api/beitreten` von 127.0.0.1.
#[tokio::test]
async fn g4_jede_408_wird_als_zu_langsam_gezaehlt_und_als_episode_gemeldet() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let instanz = starten_um(wurzel.path(), &[("LOCALCHATA_EPISODE_RUHE_MS", "1000")]);
    let mut offen = Vec::new();
    for _ in 0..3 {
        offen.push(schweigend_oeffnen(instanz.port(), "127.0.0.1").await);
    }
    let mut ausgaenge = Vec::new();
    for (strom, seit) in &mut offen {
        ausgaenge.push(bis_zum_schliessen(strom, *seit).await);
    }
    let (_, inhalt) = gesundheit(&instanz.adresse).await;
    let datei = abwarten_episode_endet(wurzel.path()).await;

    for ausgang in &ausgaenge {
        assert_eq!(ausgang.status, Some(408), "{ausgang:?}");
        let json: serde_json::Value = serde_json::from_str(&ausgang.inhalt).expect("JSON");
        assert_eq!(
            json,
            serde_json::json!({"fehler": "Körper nicht vollständig innerhalb von 10 s — Anfrage abgebrochen."})
        );
        assert!(
            ausgang.kopf.iter().any(|z| z == "connection: close"),
            "{ausgang:?}"
        );
        assert!(ausgang.zu, "{ausgang:?}");
    }
    assert_eq!(
        inhalt["abgewiesen"],
        serde_json::json!({"zu_gross":0,"kein_json":0,"kaputt":0,"abgebrochen":0,"zu_langsam":3})
    );
    let koerper = mit(&datei, "WARNING", "körper");
    assert_eq!(koerper.len(), 2, "{datei:#?}");
    assert!(koerper[0].contains("— Episode beginnt"), "{}", koerper[0]);
    assert!(
        koerper[1].contains("— Episode endet: 3 ") && koerper[1].contains("(zu_langsam 3)"),
        "{}",
        koerper[1]
    );
    for zeile in &koerper {
        assert_eq!(wert(zeile, "route").as_deref(), Some("POST /api/beitreten"));
        assert_eq!(wert(zeile, "herkunft").as_deref(), Some("127.0.0.1"));
    }
}

/// Muster-Suche: jede Route, die über `json_lesen` liest, antwortet nach der Frist 408 — auch die Anlage eines
/// Channels (mit gültiger Sitzung), nicht nur der Beitritt.
#[tokio::test]
async fn g4_auch_die_anlage_eines_channels_antwortet_nach_der_frist_408() {
    let instanz = starten_um(tempfile::tempdir().expect("Ordner").path(), &[]);
    let token = gemeinsam::Absender::von("127.0.0.1")
        .beitreten(&instanz.adresse, serde_json::json!({"name": "alpha"}), None)
        .await
        .json["token"]
        .as_str()
        .expect("Token")
        .to_string();
    let mut strom = verbinden(instanz.port(), "127.0.0.1").await;
    strom
        .write_all(
            format!(
                "POST /api/channels HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nX-LocalchatA-Sitzung: {token}\r\nContent-Length: 4096\r\n\r\n{{\"name\":\"a"
            )
            .as_bytes(),
        )
        .await
        .expect("Kopf und 10 Bytes");
    let ausgang = bis_zum_schliessen(&mut strom, Instant::now()).await;

    assert_eq!(ausgang.status, Some(408), "{ausgang:?}");
    assert!(
        ausgang.kopf.iter().any(|z| z == "connection: close"),
        "{ausgang:?}"
    );
    assert!(ausgang.zu && ausgang.nach <= GRENZE, "{ausgang:?}");
    let json: serde_json::Value = serde_json::from_str(&ausgang.inhalt).expect("JSON");
    assert_eq!(
        json["fehler"],
        "Körper nicht vollständig innerhalb von 10 s — Anfrage abgebrochen."
    );
}

/// Wartet höchstens 10 s auf das Ende der Körper-Episode in den Zeilen von heute.
async fn abwarten_episode_endet(ordner: &Path) -> Vec<String> {
    let ende = Instant::now() + Duration::from_secs(10);
    loop {
        let datei = zeilen(ordner, "2026-10-28");
        let fertig = mit(&datei, "WARNING", "körper")
            .iter()
            .any(|z| z.contains("Episode endet"));
        if fertig || Instant::now() > ende {
            return datei;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
