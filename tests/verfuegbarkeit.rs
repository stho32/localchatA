//! B0017 Verbindungen begrenzen — Angriffsmatrix des Security-Reviews (CWE-400, Slowloris), Zeilen 1–6,
//! je Zeile ein Test über das echte Programm auf Port 0. G ist die Kopf-Zeitgrenze des Programms;
//! sie gilt auch als Leerlaufgrenze zwischen zwei Requests derselben Verbindung.
#![cfg(unix)]

mod gemeinsam;

use gemeinsam::{
    Instanz, PROGRAMM, auf_startzeile_warten_hoechstens, lokal_starten, signal_senden,
    wegwerf_starten,
};
use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

/// Kopf-Zeitgrenze G laut Befund (empfohlen ≤ 10 s) — die Matrix erlaubt bis zum Schließen G + 2 s.
const G: Duration = Duration::from_secs(10);
const SPIELRAUM: Duration = Duration::from_secs(2);
/// Frist für offene Verbindungen nach dem Signal (eigene Variante, nicht Teil der Matrix).
const ENDE_FRIST: Duration = Duration::from_secs(5);
/// So lange wartet ein Test höchstens auf das Schließen, damit er heute rot statt hängend endet.
const BEOBACHTEN: Duration = Duration::from_secs(20);

const HALBER_KOPF: &[u8] = b"GET / HTTP/1.1\r\n";
const VOLLSTAENDIGER_REQUEST: &[u8] = b"GET /api/gesundheit HTTP/1.1\r\nHost: localchata\r\n\r\n";

fn verbinden(instanz: &Instanz) -> TcpStream {
    TcpStream::connect(("127.0.0.1", instanz.port())).expect("Verbindung zur Instanz")
}

/// Wartet, bis der Server die Verbindung schließt (EOF oder Reset; eine 408-Antwort davor ist erlaubt),
/// und liefert die Zeit seit `seit` — oder `None`, wenn sie nach `BEOBACHTEN` noch offen ist.
fn schliessen_abwarten(verbindung: &mut TcpStream, seit: Instant) -> Option<Duration> {
    let mut puffer = [0u8; 1024];
    while seit.elapsed() < BEOBACHTEN {
        verbindung
            .set_read_timeout(Some(Duration::from_millis(200)))
            .expect("Lesegrenze");
        match verbindung.read(&mut puffer) {
            Ok(0) => return Some(seit.elapsed()),
            Ok(_) => continue,
            Err(fehler) if matches!(fehler.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                continue;
            }
            Err(_) => return Some(seit.elapsed()),
        }
    }
    None
}

/// Liest eine vollständige Antwort (Kopf plus `Content-Length` Bytes) und liefert den Statuscode.
fn antwort_lesen(leser: &mut BufReader<TcpStream>) -> u16 {
    leser
        .get_ref()
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("Lesegrenze");
    let mut statuszeile = String::new();
    leser.read_line(&mut statuszeile).expect("Statuszeile");
    let status = statuszeile
        .split(' ')
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or_else(|| panic!("keine Statuszeile: {statuszeile:?}"));
    let mut laenge = 0usize;
    loop {
        let mut zeile = String::new();
        leser.read_line(&mut zeile).expect("Kopfzeile");
        if zeile == "\r\n" || zeile.is_empty() {
            break;
        }
        if let Some((name, wert)) = zeile.split_once(':')
            && name.eq_ignore_ascii_case("content-length")
        {
            laenge = wert.trim().parse().expect("Content-Length");
        }
    }
    let mut inhalt = vec![0u8; laenge];
    leser.read_exact(&mut inhalt).expect("Inhalt");
    status
}

fn innerhalb(gemessen: Option<Duration>, grenze: Duration, was: &str) {
    match gemessen {
        Some(dauer) => assert!(
            dauer <= grenze,
            "{was}: erst nach {dauer:?} geschlossen, erlaubt {grenze:?}"
        ),
        None => panic!("{was}: nach {BEOBACHTEN:?} noch offen, erlaubt {grenze:?}"),
    }
}

#[test]
fn zeile1_halber_kopf_ohne_weitere_bytes_wird_spaetestens_nach_g_plus_2s_geschlossen() {
    let instanz = wegwerf_starten();
    let mut verbindung = verbinden(&instanz);
    verbindung.write_all(HALBER_KOPF).expect("halber Kopf");
    let seit = Instant::now();

    let geschlossen = schliessen_abwarten(&mut verbindung, seit);

    innerhalb(geschlossen, G + SPIELRAUM, "halber Kopf");
}

/// Eigene Variante: eine Verbindung, die kein einziges Byte sendet, fällt unter dieselbe Kopf-Zeitgrenze.
#[test]
fn variante_verbindung_ohne_ein_byte_wird_spaetestens_nach_g_plus_2s_geschlossen() {
    let instanz = wegwerf_starten();
    let mut verbindung = verbinden(&instanz);
    let seit = Instant::now();

    let geschlossen = schliessen_abwarten(&mut verbindung, seit);

    innerhalb(geschlossen, G + SPIELRAUM, "stumme Verbindung");
}

#[test]
fn zeile2_troepfelnder_kopf_ein_byte_alle_2s_wird_spaetestens_nach_g_plus_2s_geschlossen() {
    let instanz = wegwerf_starten();
    let mut verbindung = verbinden(&instanz);
    let seit = Instant::now();
    let kopf = [HALBER_KOPF, &[b'X'; 200]].concat();
    let mut puffer = [0u8; 1024];
    let mut geschlossen = None;

    for byte in kopf.iter().take_while(|_| seit.elapsed() < BEOBACHTEN) {
        if verbindung.write_all(&[*byte]).is_err() {
            geschlossen = Some(seit.elapsed());
            break;
        }
        verbindung
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("Lesegrenze");
        match verbindung.read(&mut puffer) {
            Err(fehler) if matches!(fehler.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
            }
            _ => {
                geschlossen = schliessen_abwarten(&mut verbindung, seit);
                break;
            }
        }
    }

    innerhalb(geschlossen, G + SPIELRAUM, "tröpfelnder Kopf");
}

#[test]
fn zeile3_keep_alive_leerlauf_nach_vollstaendigem_request_wird_nach_fester_grenze_geschlossen() {
    let instanz = wegwerf_starten();
    let mut verbindung = verbinden(&instanz);
    verbindung
        .write_all(VOLLSTAENDIGER_REQUEST)
        .expect("Request");
    let mut leser = BufReader::new(verbindung.try_clone().expect("Leser"));
    assert_eq!(antwort_lesen(&mut leser), 200);
    let seit = Instant::now();

    let geschlossen = schliessen_abwarten(&mut verbindung, seit);

    innerhalb(geschlossen, G + SPIELRAUM, "Keep-Alive-Leerlauf");
}

/// Startet das Programm in einer Shell mit `ulimit -n <grenze>` — ohne `unsafe` im Testcode.
fn mit_deskriptorgrenze_starten(ordner: &Path, grenze: u32) -> Instanz {
    let kind = Command::new("sh")
        .arg("-c")
        .arg(format!("ulimit -n {grenze} && exec \"$0\" \"$@\""))
        .arg(PROGRAMM)
        .arg(ordner)
        .args(["--port", "0", "--host", "127.0.0.1", "--kein-browser"])
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("localchatA mit Deskriptorgrenze startet");
    auf_startzeile_warten_hoechstens(ordner, kind, Duration::from_secs(30))
}

#[tokio::test(flavor = "multi_thread")]
async fn zeile4_1100_halbe_koepfe_einer_adresse_bei_1024_deskriptoren_lassen_den_health_check_durch()
 {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let instanz = mit_deskriptorgrenze_starten(wurzel.path(), 1024);
    let ziel = std::net::SocketAddr::from(([127, 0, 0, 1], instanz.port()));
    let angreifer = std::net::SocketAddr::from(([127, 0, 0, 2], 0));

    let mut halbe_koepfe = Vec::new();
    for _ in 0..1100 {
        let sockel = tokio::net::TcpSocket::new_v4().expect("Sockel");
        sockel.bind(angreifer).expect("Absender 127.0.0.2");
        let Ok(verbindung) = sockel.connect(ziel).await else {
            continue;
        };
        let _ = verbindung.try_write(HALBER_KOPF);
        halbe_koepfe.push(verbindung);
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .expect("Client");
    let antwort = client
        .get(format!("{}api/gesundheit", instanz.adresse))
        .send()
        .await;

    let status = antwort.map(|antwort| antwort.status().as_u16());
    assert!(
        matches!(status, Ok(200)),
        "Health-Check eines zweiten Clients bei {} halben Köpfen: {status:?}",
        halbe_koepfe.len()
    );
}

/// Öffnet einen halben Kopf, beweist mit einem Kontrollrequest, dass er angenommen ist, schickt SIGTERM
/// und liefert Ende und Dauer bis zum Ende; der halbe Kopf bleibt dabei offen.
fn sigterm_bei_offenem_halbem_kopf(instanz: &mut Instanz) -> (ExitStatus, Duration) {
    let mut halber = verbinden(instanz);
    halber.write_all(HALBER_KOPF).expect("halber Kopf");
    let mut kontrolle = verbinden(instanz);
    kontrolle
        .write_all(VOLLSTAENDIGER_REQUEST)
        .expect("Kontrollrequest");
    assert_eq!(
        antwort_lesen(&mut BufReader::new(kontrolle)),
        200,
        "die Instanz bedient, der halbe Kopf ist angenommen"
    );

    signal_senden(&instanz.kind, "TERM");
    let seit = Instant::now();
    let ende = gemeinsam::warten(&mut instanz.kind, BEOBACHTEN);

    let dauer = seit.elapsed();
    let ende = ende.unwrap_or_else(|| panic!("nach SIGTERM nach {dauer:?} nicht beendet"));
    drop(halber);
    (ende, dauer)
}

#[test]
fn zeile5_sigterm_bei_offenem_halbem_kopf_endet_mit_exit_0_und_der_neustart_gelingt() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let mut instanz = lokal_starten(wurzel.path());

    let (ende, dauer) = sigterm_bei_offenem_halbem_kopf(&mut instanz);

    assert_eq!(ende.code(), Some(0), "Exit-Code nach SIGTERM");
    assert!(dauer <= G + SPIELRAUM, "Ende erst nach {dauer:?}");
    let neustart = lokal_starten(wurzel.path());
    assert!(
        neustart.startzeile.starts_with("localchatA läuft für "),
        "{}",
        neustart.startzeile
    );
}

/// Eigene Variante: das Ende hängt nicht an der Kopf-Zeitgrenze, sondern an der Frist für offene
/// Verbindungen (5 s) — sonst hielte ein künftiger Antwortstrom, den keine Kopf-Zeitgrenze trifft, das Ende auf.
#[test]
fn variante_sigterm_bricht_offene_verbindungen_nach_der_frist_von_5s_ab() {
    let mut instanz = wegwerf_starten();

    let (ende, dauer) = sigterm_bei_offenem_halbem_kopf(&mut instanz);

    assert_eq!(ende.code(), Some(0), "Exit-Code nach SIGTERM");
    assert!(
        dauer <= ENDE_FRIST + SPIELRAUM,
        "Ende erst nach {dauer:?}, Frist {ENDE_FRIST:?}"
    );
}

#[test]
fn zeile6_vollstaendiger_request_mit_sofortiger_antwort_bleibt_200() {
    let instanz = wegwerf_starten();
    let mut verbindung = verbinden(&instanz);

    verbindung
        .write_all(VOLLSTAENDIGER_REQUEST)
        .expect("Request");

    assert_eq!(antwort_lesen(&mut BufReader::new(verbindung)), 200);
}

#[test]
fn zeile6_langsamer_gueltiger_client_mit_pause_unter_g_wird_nicht_getrennt() {
    let instanz = wegwerf_starten();
    let mut verbindung = verbinden(&instanz);
    let (anfang, rest) = VOLLSTAENDIGER_REQUEST.split_at(HALBER_KOPF.len() + 6);

    verbindung.write_all(anfang).expect("Anfang des Kopfes");
    std::thread::sleep(G / 2);
    verbindung.write_all(rest).expect("Rest des Kopfes");

    assert_eq!(antwort_lesen(&mut BufReader::new(verbindung)), 200);
}

#[test]
fn zeile6_zweiter_request_nach_pause_unter_g_auf_derselben_verbindung_bleibt_200() {
    let instanz = wegwerf_starten();
    let mut verbindung = verbinden(&instanz);
    let mut leser = BufReader::new(verbindung.try_clone().expect("Leser"));
    verbindung
        .write_all(VOLLSTAENDIGER_REQUEST)
        .expect("erster Request");
    assert_eq!(antwort_lesen(&mut leser), 200);

    std::thread::sleep(G / 2);
    verbindung
        .write_all(VOLLSTAENDIGER_REQUEST)
        .expect("zweiter Request");

    assert_eq!(antwort_lesen(&mut leser), 200);
}
