//! B0017 Verbindungen begrenzen — Angriffsmatrix des Security-Reviews (CWE-400, Slowloris), Zeilen 1–6,
//! je Zeile ein Test über das echte Programm auf Port 0. G ist die Kopf-Zeitgrenze des Programms;
//! sie gilt auch als Leerlaufgrenze zwischen zwei Requests derselben Verbindung. Dazu die Flutmessung aus
//! R00010 (Modul `koerperflut`, nur im Release-Build).
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
const VOLLSTAENDIGER_REQUEST: &[u8] = b"GET /api/gesundheit HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n";

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

/// R00010 B0205 (Befundkarte #21, Abnahme wörtlich): „24 und 48 Schleifen dieses Körpers von 4 Herkünften für
/// 10 s → Gesundheit von einer fünften Herkunft p90 ≤ 0,15 s.“ Nur im Release-Build:
/// `cargo test --release --test verfuegbarkeit -- --ignored koerperflut`.
#[cfg(target_os = "linux")]
mod koerperflut {
    use super::gemeinsam::{Instanz, wegwerf_starten};
    use std::collections::BTreeMap;
    use std::time::{Duration, Instant, SystemTime};

    /// Grenze aus der Abnahme, nie verschärft.
    const GRENZE_P90: Duration = Duration::from_millis(150);
    /// Leerlauf-p90 darüber: der Rechner ist zu ausgelastet, der Lauf ist ungültig (nicht rot).
    const LEERLAUF_HOECHSTENS: Duration = Duration::from_millis(50);
    const LEERLAUF: Duration = Duration::from_secs(3);
    const FLUT: Duration = Duration::from_secs(10);
    const MESSBEGINN: Duration = Duration::from_secs(1);
    const TAKT: Duration = Duration::from_millis(50);
    const VERSUCHE: usize = 3;
    const FLUTHERKUENFTE: [&str; 4] = ["127.0.0.1", "127.0.0.2", "127.0.0.3", "127.0.0.4"];
    const BEOBACHTER: &str = "127.0.0.5";

    /// `[0,0,…,0]` mit 1 048 575 Nullen — 2 097 151 Bytes, 1 Byte unter 2 MiB (Körper der Reproduktion).
    fn flutkoerper() -> &'static [u8] {
        let mut koerper = String::from("[0");
        koerper.push_str(&",0".repeat(1_048_574));
        koerper.push(']');
        assert_eq!(koerper.len(), 2_097_151);
        Box::leak(koerper.into_bytes().into_boxed_slice())
    }

    /// Ohne Verbindungs-Pool: jede Anfrage öffnet ihre eigene Verbindung.
    fn client_von(herkunft: &str) -> reqwest::Client {
        reqwest::Client::builder()
            .local_address(herkunft.parse::<std::net::IpAddr>().expect("Herkunft"))
            .pool_max_idle_per_host(0)
            .build()
            .expect("Client")
    }

    /// Gesundheit alle 50 ms von der fünften Herkunft, auf eigenem Thread und eigener Laufzeit, damit die
    /// Flut-Schleifen des Tests die Messung nicht verzögern. Je Anfrage: Öffnen der Verbindung bis letztes Byte.
    fn gesundheit_messen(
        adresse: &str,
        von: Instant,
        bis: Instant,
    ) -> std::thread::JoinHandle<Vec<Duration>> {
        let url = format!("{adresse}api/gesundheit");
        std::thread::spawn(move || {
            let laufzeit = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Laufzeit");
            laufzeit.block_on(async move {
                let client = client_von(BEOBACHTER);
                tokio::time::sleep_until(von.into()).await;
                let mut takt = tokio::time::interval(TAKT);
                takt.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
                let mut dauern = Vec::new();
                loop {
                    takt.tick().await;
                    if Instant::now() >= bis {
                        return dauern;
                    }
                    let beginn = Instant::now();
                    let antwort = client.get(&url).send().await.expect("Gesundheit");
                    assert_eq!(antwort.status().as_u16(), 200);
                    antwort.bytes().await.expect("Inhalt der Gesundheit");
                    dauern.push(beginn.elapsed());
                }
            })
        })
    }

    async fn schleife(
        herkunft: &'static str,
        url: String,
        koerper: &'static [u8],
        bis: Instant,
    ) -> Vec<String> {
        let client = client_von(herkunft);
        let mut antworten = Vec::new();
        while Instant::now() < bis {
            let ergebnis = client
                .post(&url)
                .header("Content-Type", "application/json")
                .body(koerper)
                .send()
                .await;
            antworten.push(match ergebnis {
                Ok(antwort) => {
                    let status = antwort.status().as_u16().to_string();
                    let _ = antwort.bytes().await;
                    status
                }
                Err(fehler) => format!("Fehler {fehler}"),
            });
        }
        antworten
    }

    fn fluten(
        instanz: &Instanz,
        schleifen: usize,
        koerper: &'static [u8],
        bis: Instant,
    ) -> BTreeMap<String, usize> {
        let laufzeit = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .expect("Laufzeit");
        let url = format!("{}api/beitreten", instanz.adresse);
        laufzeit.block_on(async move {
            let laeufe: Vec<_> = (0..schleifen)
                .map(|nummer| {
                    let herkunft = FLUTHERKUENFTE[nummer % FLUTHERKUENFTE.len()];
                    tokio::spawn(schleife(herkunft, url.clone(), koerper, bis))
                })
                .collect();
            let mut antworten = BTreeMap::new();
            for lauf in laeufe {
                for status in lauf.await.expect("Schleife") {
                    *antworten.entry(status).or_insert(0) += 1;
                }
            }
            antworten
        })
    }

    /// 90-%-Quantil nach dem Rang (aufgerundet).
    fn quantil(sortiert: &[Duration], anteil: usize) -> Duration {
        sortiert[(sortiert.len() * anteil).div_ceil(100) - 1]
    }

    fn rss_kib(instanz: &Instanz) -> u64 {
        let status = std::fs::read_to_string(format!("/proc/{}/status", instanz.kind.id()))
            .expect("/proc/<pid>/status");
        status
            .lines()
            .find_map(|zeile| zeile.strip_prefix("VmRSS:"))
            .and_then(|wert| wert.trim().trim_end_matches("kB").trim().parse().ok())
            .expect("VmRSS")
    }

    struct Messung {
        schleifen: usize,
        versuch: usize,
        leerlauf_p90: Duration,
        sortiert: Vec<Duration>,
        antworten: BTreeMap<String, usize>,
        rss_kib: u64,
    }

    impl Messung {
        fn p90(&self) -> Duration {
            quantil(&self.sortiert, 90)
        }

        fn zeile(&self) -> String {
            format!(
                "{} Schleifen (Versuch {}): Leerlauf p90 {:?} · Gesundheit n {} min {:?} p50 {:?} p90 {:?} max {:?} · Antworten {:?} · RSS {} KiB",
                self.schleifen,
                self.versuch,
                self.leerlauf_p90,
                self.sortiert.len(),
                self.sortiert[0],
                quantil(&self.sortiert, 50),
                self.p90(),
                self.sortiert[self.sortiert.len() - 1],
                self.antworten,
                self.rss_kib
            )
        }
    }

    /// Ein gültiger Lauf: frische Instanz, 3 s Leerlauf als Kontrolle, dann Flut und Messung. Ist der Leerlauf zu
    /// langsam, wird wiederholt; nach `VERSUCHE` ungültigen Läufen gibt es kein Urteil.
    fn messen(schleifen: usize, koerper: &'static [u8]) -> Messung {
        messen_mit(schleifen, wegwerf_starten, |instanz, ende| {
            fluten(instanz, schleifen, koerper, ende)
        })
    }

    /// Wie `messen` mit eigenem Start und eigener Flut.
    fn messen_mit(
        schleifen: usize,
        starten: impl Fn() -> Instanz,
        flut: impl Fn(&Instanz, Instant) -> BTreeMap<String, usize>,
    ) -> Messung {
        for versuch in 1..=VERSUCHE {
            let instanz = starten();
            let jetzt = Instant::now();
            let mut leerlauf = gesundheit_messen(&instanz.adresse, jetzt, jetzt + LEERLAUF)
                .join()
                .expect("Leerlauf");
            leerlauf.sort();
            let leerlauf_p90 = quantil(&leerlauf, 90);
            if leerlauf_p90 > LEERLAUF_HOECHSTENS {
                println!(
                    "{schleifen} Schleifen, Versuch {versuch}: UNGÜLTIG, Leerlauf p90 {leerlauf_p90:?}"
                );
                continue;
            }
            let start = Instant::now();
            let ende = start + FLUT;
            let beobachter = gesundheit_messen(&instanz.adresse, start + MESSBEGINN, ende);
            let antworten = flut(&instanz, ende);
            let mut sortiert = beobachter.join().expect("Beobachter");
            sortiert.sort();
            let messung = Messung {
                schleifen,
                versuch,
                leerlauf_p90,
                sortiert,
                antworten,
                rss_kib: rss_kib(&instanz),
            };
            println!("{}", messung.zeile());
            return messung;
        }
        panic!(
            "UNGÜLTIG, kein Urteil: {VERSUCHE} Läufe mit Leerlauf-p90 über {LEERLAUF_HOECHSTENS:?} — Rechner zu ausgelastet"
        );
    }

    #[test]
    #[ignore = "Leistungstest im Release-Build: cargo test --release --test verfuegbarkeit -- --ignored koerperflut"]
    fn koerperflut_24_und_48_schleifen_lassen_die_gesundheit_im_p90_unter_150_ms() {
        let koerper = flutkoerper();
        let mut reihenfolge = [24, 48];
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("Uhr")
            .subsec_nanos();
        if nanos % 2 == 1 {
            reihenfolge.reverse();
        }
        println!("Reihenfolge {reihenfolge:?}");

        let messungen: Vec<Messung> = reihenfolge
            .iter()
            .map(|&schleifen| messen(schleifen, koerper))
            .collect();

        for messung in &messungen {
            let nicht_413: usize = messung
                .antworten
                .iter()
                .filter(|(status, _)| status.as_str() != "413")
                .map(|(_, anzahl)| anzahl)
                .sum();
            assert_eq!(nicht_413, 0, "{}", messung.zeile());
            assert!(messung.p90() <= GRENZE_P90, "{}", messung.zeile());
        }
    }

    /// R00011 M2 (B0236), Messwert im Entwicklertest, kein Gate: 24 Schleifen von 4 Herkünften (je 6 Agenten mit
    /// Sitzung) senden 10 s lang Körper von genau 131.072 Bytes (`text` aus 16.384 Leerzeichen, mit Leerraum
    /// aufgefüllt) an `allgemein`; Gesundheit von 127.0.0.5 alle 50 ms. Über p90 0,15 s ist ein Befund an ADR-0036,
    /// kein roter Test — deshalb nur Ausgabe.
    /// `cargo test --release --test verfuegbarkeit -- --ignored --nocapture senderflut`.
    #[test]
    #[ignore = "Messwert M2 im Release-Build: cargo test --release --test verfuegbarkeit -- --ignored --nocapture senderflut"]
    fn senderflut_mit_sitzung_messwert_m2() {
        let mut koerper = format!("{{\"text\":\"{}\"}}", " ".repeat(16_384));
        koerper.push_str(&" ".repeat(131_072 - koerper.len()));
        let koerper: &'static [u8] = Box::leak(koerper.into_bytes().into_boxed_slice());

        let messung = messen_mit(24, sender_starten, |instanz, ende| {
            sender_fluten(instanz, 24, koerper, ende)
        });

        println!("M2: {}", messung.zeile());
        println!(
            "M2 p90 {:?} (Grenze {GRENZE_P90:?}, Messwert)",
            messung.p90()
        );
    }

    fn sender_starten() -> Instanz {
        super::gemeinsam::wegwerf_starten_mit(|ordner| {
            std::fs::create_dir_all(ordner.join("channels/allgemein")).expect("Channel");
        })
    }

    fn sender_fluten(
        instanz: &Instanz,
        schleifen: usize,
        koerper: &'static [u8],
        bis: Instant,
    ) -> BTreeMap<String, usize> {
        let laufzeit = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .expect("Laufzeit");
        let basis = instanz.adresse.clone();
        laufzeit.block_on(async move {
            let mut laeufe = Vec::new();
            for nummer in 0..schleifen {
                let herkunft = FLUTHERKUENFTE[nummer % FLUTHERKUENFTE.len()];
                let client = client_von(herkunft);
                let beitritt: serde_json::Value = client
                    .post(format!("{basis}api/beitreten"))
                    .json(&serde_json::json!({ "name": format!("flut{nummer}") }))
                    .send()
                    .await
                    .expect("Beitritt")
                    .json()
                    .await
                    .expect("Token");
                let token = beitritt["token"].as_str().expect("Token").to_string();
                let url = format!("{basis}api/channels/allgemein/nachrichten");
                laeufe.push(tokio::spawn(async move {
                    let mut antworten = Vec::new();
                    while Instant::now() < bis {
                        let ergebnis = client
                            .post(&url)
                            .header("Content-Type", "application/json")
                            .header("X-LocalchatA-Sitzung", &token)
                            .body(koerper)
                            .send()
                            .await;
                        antworten.push(match ergebnis {
                            Ok(antwort) => {
                                let status = antwort.status().as_u16().to_string();
                                let _ = antwort.bytes().await;
                                status
                            }
                            Err(fehler) => format!("Fehler {fehler}"),
                        });
                    }
                    antworten
                }));
            }
            let mut antworten = BTreeMap::new();
            for lauf in laeufe {
                for status in lauf.await.expect("Schleife") {
                    *antworten.entry(status).or_insert(0) += 1;
                }
            }
            antworten
        })
    }
}
