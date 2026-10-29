//! Ausliefern bis zum Signal (B0017): eigene Annahmeschleife statt `axum::serve`, weil diese keine
//! Kopf-Zeitgrenze, keine Verbindungsgrenze und kein Ende nach Frist kennt (Security-Review, CWE-400).
//! Sie setzt je Verbindung die kanonische Gegenstelle als `ConnectInfo` — die Herkunft (ADR-0012).

use super::abrisspflege::Pflege;
use super::annahmefehler::annahmefehler_eintragen;
use super::ende::Ende;
use super::verbindungen::{Belegung, Platz, Verbindungsgrenzen};
use crate::protokoll::episoden::Auftreten;
use crate::protokoll::{Bereich, Kontext, Protokoll, Schluessel, Stufe};
use axum::extract::ConnectInfo;
use axum::{Extension, Router};
use hyper::server::conn::http1;
use hyper_util::rt::{TokioIo, TokioTimer};
use hyper_util::service::TowerToHyperService;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinSet;

/// Die lokale Adresse einer Verbindung — über welche eigene Adresse und welchen Port der Client kam,
/// kanonisch wie die Gegenstelle (ADR-0045 Regel 3, Request-Extension neben `ConnectInfo`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LokaleAdresse(pub SocketAddr);

/// So lange darf ein Request-Kopf vom ersten Byte bis zur Leerzeile brauchen (gesamt, nicht je Lesevorgang).
/// Hyper startet die Uhr auch nach jeder Antwort neu: sie ist zugleich die Leerlaufgrenze zwischen Requests.
/// Laufende Antworten (Live-Kanal) zählen nicht, die Uhr läuft erst, wenn der nächste Kopf erwartet wird.
pub const KOPF_ZEITGRENZE: Duration = Duration::from_secs(10);
/// So lange bekommen offene Verbindungen nach dem Signal, dann werden sie abgebrochen.
pub const ENDE_FRIST: Duration = Duration::from_secs(5);
/// Pause nach einem Fehler der Annahme, damit ein dauerhafter Fehler die Schleife nicht heiß laufen lässt.
const ANNAHMEPAUSE: Duration = Duration::from_millis(50);

/// Nimmt Verbindungen an und bedient sie, bis das Ende gemeldet ist; danach endet jede offene Verbindung
/// geordnet oder spätestens nach `ENDE_FRIST`. Handler, die selbst auf das Ende hören (Ströme), enden sofort.
/// Jede zugelassene Verbindung wird vor dem Bedienen einmal gepflegt (ADR-0017), abgewiesene nicht.
pub async fn ausliefern(
    listener: TcpListener,
    app: Router,
    ende: Ende,
    grenzen: Verbindungsgrenzen,
    pflege: Pflege,
    protokoll: Arc<Protokoll>,
) {
    let belegung = Belegung::neu(grenzen);
    let mut verbindungen = JoinSet::new();
    let mut beenden = ende.clone();
    loop {
        tokio::select! {
            () = beenden.abwarten() => break,
            Some(_) = verbindungen.join_next(), if !verbindungen.is_empty() => {}
            angenommen = listener.accept() => match angenommen {
                Ok((strom, gegenstelle)) => {
                    let gegenstelle = kanonisch(gegenstelle);
                    // Ohne Platz fällt der Strom hier und die Verbindung ist sofort geschlossen.
                    let platz = belegung.zulassen(gegenstelle.ip());
                    if platz.is_none() {
                        abweisung_eintragen(&protokoll, gegenstelle.ip(), belegung.erreichte_grenze());
                    }
                    if let Some(platz) = platz {
                        pflege.anwenden(&strom);
                        let mut app = app.clone().layer(Extension(ConnectInfo(gegenstelle)));
                        // Ohne lesbare lokale Adresse fehlt die Extension: dann kein Rückwärtsname, Abweisung ohne Adresse.
                        if let Ok(lokal) = strom.local_addr() {
                            app = app.layer(Extension(LokaleAdresse(kanonisch(lokal))));
                        }
                        let verbindung = Verbindungsprotokoll::angenommen(&protokoll, gegenstelle);
                        let ende = ende.clone();
                        verbindungen.spawn(async move {
                            bedienen(strom, app, platz, ende).await;
                            verbindung.beendet();
                        });
                    }
                }
                Err(fehler) => {
                    annahmefehler_eintragen(&protokoll, &fehler);
                    tokio::time::sleep(ANNAHMEPAUSE).await;
                }
            },
        }
    }
    drop(listener);
    let alle_beendet = async { while verbindungen.join_next().await.is_some() {} };
    if tokio::time::timeout(ENDE_FRIST, alle_beendet)
        .await
        .is_err()
    {
        verbindungen.shutdown().await;
    }
}

/// Episode `verbindung` je Herkunft an der Platzgrenze (ADR-0042 Regel 2) — heute nicht mehr still.
fn abweisung_eintragen(protokoll: &Protokoll, herkunft: IpAddr, grenze: usize) {
    protokoll.episode(Auftreten {
        stufe: Stufe::Warning,
        bereich: Bereich::Verbindung,
        text: "Verbindung an der Platzgrenze abgewiesen",
        quelle: herkunft.to_string(),
        kontext: Kontext::neu()
            .mit(Schluessel::Herkunft, herkunft)
            .mit(Schluessel::Grenze, grenze),
        art: None,
        verworfen: None,
        bytes: None,
        anzeige: None,
    });
}

/// DEBUG `verbindung` je zugelassener Verbindung: angenommen, und beendet mit der Dauer (ADR-0042 Regel 2).
struct Verbindungsprotokoll {
    protokoll: Arc<Protokoll>,
    herkunft: IpAddr,
    seit: Instant,
}

impl Verbindungsprotokoll {
    fn angenommen(protokoll: &Arc<Protokoll>, gegenstelle: SocketAddr) -> Verbindungsprotokoll {
        protokoll.eintragen(
            Stufe::Debug,
            Bereich::Verbindung,
            "Verbindung angenommen",
            Kontext::neu().mit(Schluessel::Herkunft, gegenstelle.ip()),
        );
        Verbindungsprotokoll {
            protokoll: Arc::clone(protokoll),
            herkunft: gegenstelle.ip(),
            seit: Instant::now(),
        }
    }

    fn beendet(self) {
        self.protokoll.eintragen(
            Stufe::Debug,
            Bereich::Verbindung,
            "Verbindung beendet",
            Kontext::neu()
                .mit(Schluessel::Herkunft, self.herkunft)
                .mit(Schluessel::DauerMs, self.seit.elapsed().as_millis()),
        );
    }
}

/// Die Gegenstelle einmal normalisiert: IPv4-gemappte IPv6-Adressen werden IPv4 (ADR-0012) — Herkunft
/// und Platzzählung sehen dieselbe Adresse.
fn kanonisch(gegenstelle: SocketAddr) -> SocketAddr {
    SocketAddr::new(gegenstelle.ip().to_canonical(), gegenstelle.port())
}

/// Bedient eine Verbindung mit HTTP/1.1; nach dem Ende-Signal keine neuen Requests mehr.
/// Das Ergebnis der Verbindung (abgelaufene Kopf-Zeitgrenze, Abbruch der Gegenstelle) betrifft nur sie
/// selbst und wird bewusst nicht gemeldet — eine Zeile je Fremdverbindung würde stderr fluten.
async fn bedienen(strom: TcpStream, app: Router, _platz: Platz, mut ende: Ende) {
    let verbindung = http1::Builder::new()
        .timer(TokioTimer::new())
        .header_read_timeout(KOPF_ZEITGRENZE)
        .serve_connection(TokioIo::new(strom), TowerToHyperService::new(app));
    tokio::pin!(verbindung);
    tokio::select! {
        _ = verbindung.as_mut() => return,
        () = ende.abwarten() => {}
    }
    verbindung.as_mut().graceful_shutdown();
    let _ = verbindung.await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::start::abrisspflege::{Pflegefehler, pflege_aus};
    use axum::routing::get;
    use std::net::IpAddr;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    fn programmpflege() -> Pflege {
        Pflege::kernel(
            pflege_aus(Duration::from_secs(5)),
            Arc::new(crate::protokoll::Protokoll::ohne_ablage()),
        )
    }

    /// Startet die Annahmeschleife mit einer Route, die die Herkunft der Anfrage nennt.
    async fn herkunft_zeigen(host: &str) -> u16 {
        let app = Router::new().route(
            "/",
            get(
                |ConnectInfo(gegenstelle): ConnectInfo<SocketAddr>| async move {
                    gegenstelle.ip().to_string()
                },
            ),
        );
        let listener = TcpListener::bind((host, 0)).await.expect("gebunden");
        let port = listener.local_addr().expect("Adresse").port();
        let grenzen = Verbindungsgrenzen::fuer_deskriptoren(None);
        tokio::spawn(ausliefern(
            listener,
            app,
            Ende::nie(),
            grenzen,
            programmpflege(),
            Arc::new(Protokoll::ohne_ablage()),
        ));
        port
    }

    async fn herkunft(ziel: &str, absender: Option<IpAddr>) -> String {
        let client = reqwest::Client::builder()
            .local_address(absender)
            .build()
            .expect("Client");
        let antwort = client.get(ziel).send().await.expect("Antwort");
        assert_eq!(antwort.status(), 200);
        antwort.text().await.expect("Text")
    }

    #[tokio::test]
    async fn auf_allen_ipv6_schnittstellen_ist_ipv4_die_herkunft_nie_die_gemappte_form() {
        let port = herkunft_zeigen("::").await;

        let ueber_v4 = herkunft(&format!("http://127.0.0.1:{port}/"), None).await;
        let ueber_v6 = herkunft(&format!("http://[::1]:{port}/"), None).await;

        assert_eq!(ueber_v4, "127.0.0.1");
        assert_eq!(ueber_v6, "::1");
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn ein_zweiter_absender_im_loopback_ist_seine_eigene_herkunft() {
        let port = herkunft_zeigen("0.0.0.0").await;
        let zweiter = IpAddr::from([127, 0, 0, 2]);

        let gesehen = herkunft(&format!("http://127.0.0.1:{port}/"), Some(zweiter)).await;

        assert_eq!(gesehen, "127.0.0.2");
    }

    /// Startet die Annahmeschleife mit einer Route, die die lokale Adresse der Verbindung nennt (B0366).
    async fn lokale_adresse_zeigen(host: &str) -> u16 {
        let app = Router::new().route(
            "/",
            get(
                |Extension(LokaleAdresse(lokal)): Extension<LokaleAdresse>| async move {
                    lokal.to_string()
                },
            ),
        );
        let listener = TcpListener::bind((host, 0)).await.expect("gebunden");
        let port = listener.local_addr().expect("Adresse").port();
        tokio::spawn(ausliefern(
            listener,
            app,
            Ende::nie(),
            Verbindungsgrenzen::fuer_deskriptoren(None),
            programmpflege(),
            Arc::new(Protokoll::ohne_ablage()),
        ));
        port
    }

    #[tokio::test]
    async fn die_lokale_adresse_ist_die_angesprochene_mit_port() {
        let port = lokale_adresse_zeigen("127.0.0.1").await;

        let gesehen = herkunft(&format!("http://127.0.0.1:{port}/"), None).await;

        assert_eq!(gesehen, format!("127.0.0.1:{port}"));
    }

    /// Matrix K4, B6: über `[::1]` bei `--host ::` → `::1`; über 127.0.0.2 bei `--host ::` kanonisch 127.0.0.2.
    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn die_lokale_adresse_ist_kanonisch_auch_auf_allen_ipv6_schnittstellen() {
        let port = lokale_adresse_zeigen("::").await;

        let ueber_v6 = herkunft(&format!("http://[::1]:{port}/"), None).await;
        let ueber_zweite = herkunft(&format!("http://127.0.0.2:{port}/"), None).await;

        assert_eq!(ueber_v6, format!("[::1]:{port}"));
        assert_eq!(ueber_zweite, format!("127.0.0.2:{port}"));
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn auf_allen_ipv4_schnittstellen_ist_die_lokale_adresse_die_angesprochene() {
        let port = lokale_adresse_zeigen("0.0.0.0").await;

        let ueber_zweite = herkunft(&format!("http://127.0.0.2:{port}/"), None).await;
        let ueber_erste = herkunft(&format!("http://127.0.0.1:{port}/"), None).await;

        assert_eq!(ueber_zweite, format!("127.0.0.2:{port}"));
        assert_eq!(ueber_erste, format!("127.0.0.1:{port}"));
    }

    /// Ende-Signal für alle (B0078): ein Handler, der wie ein Strom auf das Ende hört, antwortet beim Signal
    /// selbst — die Schleife ist danach fertig, ohne die Frist von 5 s abzuwarten.
    #[tokio::test]
    async fn ein_handler_der_das_ende_hoert_endet_mit_dem_signal_ohne_die_frist() {
        let (melden, ende) = Ende::neu();
        let fuer_den_handler = ende.clone();
        let app = Router::new().route(
            "/warten",
            get(move || {
                let mut ende = fuer_den_handler.clone();
                async move {
                    ende.abwarten().await;
                    "beendet"
                }
            }),
        );
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.expect("gebunden");
        let adresse = listener.local_addr().expect("Adresse");
        let grenzen = Verbindungsgrenzen::fuer_deskriptoren(None);
        let schleife = tokio::spawn(ausliefern(
            listener,
            app,
            ende,
            grenzen,
            programmpflege(),
            Arc::new(Protokoll::ohne_ablage()),
        ));
        let anfrage = tokio::spawn(reqwest::get(format!("http://{adresse}/warten")));
        tokio::time::sleep(Duration::from_millis(200)).await;

        let gesendet = tokio::time::Instant::now();
        melden.melden();
        let antwort = tokio::time::timeout(Duration::from_secs(1), anfrage)
            .await
            .expect("Antwort in unter 1 s nach dem Signal")
            .expect("Aufgabe")
            .expect("Antwort");
        let text = antwort.text().await.expect("Text");
        tokio::time::timeout(Duration::from_secs(1), schleife)
            .await
            .expect("Schleife endet in unter 1 s")
            .expect("Aufgabe");

        assert_eq!(text, "beendet");
        assert!(gesendet.elapsed() < Duration::from_secs(1));
    }

    /// Eine Pflege, die zählt, wie oft sie gerufen wird, `ergebnis` liefert und jede Meldung aufschreibt.
    struct Aufzeichnung {
        aufrufe: Arc<AtomicUsize>,
        meldungen: Arc<Mutex<Vec<String>>>,
    }

    fn aufzeichnend(scheitert: bool) -> (Pflege, Aufzeichnung) {
        let aufrufe = Arc::new(AtomicUsize::new(0));
        let meldungen = Arc::new(Mutex::new(Vec::new()));
        let (gezaehlt, notiert) = (Arc::clone(&aufrufe), Arc::clone(&meldungen));
        let pflege = Pflege::neu(
            move |_: &TcpStream| {
                gezaehlt.fetch_add(1, Ordering::SeqCst);
                if scheitert {
                    return Err(Pflegefehler {
                        option: "TCP_USER_TIMEOUT",
                        fehler: std::io::Error::from_raw_os_error(92),
                    });
                }
                Ok(())
            },
            move |fehler: &Pflegefehler| notiert.lock().expect("Liste").push(fehler.to_string()),
        );
        (pflege, Aufzeichnung { aufrufe, meldungen })
    }

    async fn mit_pflege(pflege: Pflege, grenzen: Verbindungsgrenzen) -> SocketAddr {
        let app = Router::new().route("/", get(|| async { "da" }));
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.expect("gebunden");
        let adresse = listener.local_addr().expect("Adresse");
        tokio::spawn(ausliefern(
            listener,
            app,
            Ende::nie(),
            grenzen,
            pflege,
            Arc::new(Protokoll::ohne_ablage()),
        ));
        adresse
    }

    /// Schickt `anzahl` Anfragen über eine Verbindung (die letzte schließt) und liefert die Statuszeilen.
    async fn anfragen(adresse: SocketAddr, anzahl: usize) -> Vec<String> {
        let mut verbindung = TcpStream::connect(adresse).await.expect("verbunden");
        let mut statuszeilen = Vec::new();
        for nummer in 1..=anzahl {
            let schluss = if nummer == anzahl {
                "Connection: close\r\n"
            } else {
                ""
            };
            let anfrage = format!("GET / HTTP/1.1\r\nHost: 127.0.0.1\r\n{schluss}\r\n");
            verbindung
                .write_all(anfrage.as_bytes())
                .await
                .expect("gesendet");
            let mut puffer = vec![0u8; 4096];
            let gelesen = verbindung.read(&mut puffer).await.expect("gelesen");
            let antwort = String::from_utf8_lossy(&puffer[..gelesen]).into_owned();
            statuszeilen.push(antwort.lines().next().unwrap_or_default().to_string());
        }
        statuszeilen
    }

    fn reichlich() -> Verbindungsgrenzen {
        Verbindungsgrenzen::fuer_deskriptoren(None)
    }

    #[tokio::test]
    async fn zwei_verbindungen_mit_je_einer_anfrage_werden_je_einmal_gepflegt() {
        let (pflege, aufzeichnung) = aufzeichnend(false);
        let adresse = mit_pflege(pflege, reichlich()).await;

        let erste = anfragen(adresse, 1).await;
        let zweite = anfragen(adresse, 1).await;

        assert_eq!(
            [erste, zweite].concat(),
            ["HTTP/1.1 200 OK", "HTTP/1.1 200 OK"]
        );
        assert_eq!(aufzeichnung.aufrufe.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn zwei_anfragen_auf_derselben_verbindung_werden_einmal_gepflegt() {
        let (pflege, aufzeichnung) = aufzeichnend(false);
        let adresse = mit_pflege(pflege, reichlich()).await;

        let statuszeilen = anfragen(adresse, 2).await;

        assert_eq!(statuszeilen, ["HTTP/1.1 200 OK", "HTTP/1.1 200 OK"]);
        assert_eq!(aufzeichnung.aufrufe.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn eine_ueber_der_verbindungsgrenze_abgewiesene_verbindung_wird_nicht_gepflegt() {
        let (pflege, aufzeichnung) = aufzeichnend(false);
        let grenzen = Verbindungsgrenzen {
            je_gegenstelle: 1,
            gesamt: 1,
        };
        let adresse = mit_pflege(pflege, grenzen).await;
        let mut gehalten = TcpStream::connect(adresse).await.expect("erste");
        gehalten
            .write_all(b"GET / HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
            .await
            .expect("gesendet");
        let mut puffer = vec![0u8; 4096];
        let gelesen = gehalten.read(&mut puffer).await.expect("Antwort");
        assert!(String::from_utf8_lossy(&puffer[..gelesen]).starts_with("HTTP/1.1 200"));

        let mut abgewiesen = TcpStream::connect(adresse).await.expect("zweite");
        let beim_lesen = tokio::time::timeout(Duration::from_secs(2), abgewiesen.read(&mut puffer))
            .await
            .expect("abgewiesen binnen 2 s");

        assert!(
            matches!(beim_lesen, Ok(0) | Err(_)),
            "die zweite Verbindung ist geschlossen: {beim_lesen:?}"
        );
        assert_eq!(aufzeichnung.aufrufe.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn scheitert_die_pflege_wird_trotzdem_bedient_und_nur_der_erste_fehler_gemeldet() {
        let (pflege, aufzeichnung) = aufzeichnend(true);
        let adresse = mit_pflege(pflege, reichlich()).await;

        let erste = anfragen(adresse, 1).await;
        let zweite = anfragen(adresse, 1).await;

        assert_eq!(
            [erste, zweite].concat(),
            ["HTTP/1.1 200 OK", "HTTP/1.1 200 OK"]
        );
        assert_eq!(aufzeichnung.aufrufe.load(Ordering::SeqCst), 2);
        let meldungen = aufzeichnung.meldungen.lock().expect("Liste").clone();
        assert_eq!(meldungen.len(), 1, "{meldungen:?}");
        assert!(meldungen[0].contains("TCP_USER_TIMEOUT"), "{meldungen:?}");
        assert!(meldungen[0].contains("os error 92"), "{meldungen:?}");
    }

    /// Code-Review #24 B1, Abnahme-Probe 4 (Teil Verbindung): auf Stufe debug je zugelassener Verbindung genau
    /// eine Zeile „angenommen“ und, wenn sie endet, genau eine „beendet“ mit Herkunft und Dauer.
    #[tokio::test]
    async fn eine_beendete_verbindung_ist_genau_eine_debug_zeile_mit_herkunft_und_dauer() {
        use crate::protokoll::testhilfe::im_speicher;
        let (protokoll, speicher, _, _) = im_speicher("2026-10-28T12:00:00+01:00", Stufe::Debug);
        let app = Router::new().route("/", get(|| async { "da" }));
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.expect("gebunden");
        let adresse = listener.local_addr().expect("Adresse");
        tokio::spawn(ausliefern(
            listener,
            app,
            Ende::nie(),
            reichlich(),
            programmpflege(),
            protokoll,
        ));

        assert_eq!(anfragen(adresse, 1).await, ["HTTP/1.1 200 OK"]);
        let beendet = |zeilen: &[String]| {
            zeilen
                .iter()
                .filter(|zeile| zeile.contains(" DEBUG verbindung Verbindung beendet | "))
                .cloned()
                .collect::<Vec<_>>()
        };
        let frist = tokio::time::Instant::now() + Duration::from_secs(5);
        while beendet(&speicher.zeilen()).is_empty() && tokio::time::Instant::now() < frist {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        let zeilen = speicher.zeilen();
        let angenommen: Vec<&String> = zeilen
            .iter()
            .filter(|zeile| zeile.contains(" DEBUG verbindung Verbindung angenommen | "))
            .collect();
        assert_eq!(angenommen.len(), 1, "{zeilen:#?}");
        assert!(
            angenommen[0].ends_with(" | herkunft=127.0.0.1"),
            "{zeilen:#?}"
        );
        let beendet = beendet(&zeilen);
        assert_eq!(beendet.len(), 1, "{zeilen:#?}");
        let (_, kontext) = beendet[0].split_once(" | ").expect("Kontext");
        let dauer = kontext
            .strip_prefix("herkunft=127.0.0.1 dauer_ms=")
            .expect("herkunft und dauer_ms");
        assert!(dauer.parse::<u64>().is_ok(), "{kontext}");
    }
}
