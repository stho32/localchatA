//! Start: vom Aufruf bis zum lauschenden Server (A00003).
//!
//! Reihenfolge: Aufruf lesen → Uhr → Gnadenfrist → Herzschlag → Ordner vorbereiten → sperren → Bus → Verlauf laden (Channel-Wechsel an den Bus) → Fristpflege (Start) → Anwesenheit → binden → Ende-Signal → Takt → Startzeile → Browser → ausliefern bis zum Signal.

mod abriss;
pub(crate) mod abrisspflege;
mod allenamen;
mod annahmefehler;
mod aufruf;
pub(crate) mod ausliefern;
mod browser;
pub(crate) mod ende;
mod ladedefekte;
pub mod laufzeit;
mod ordner;
mod plattenquote;
mod port;
mod protokollstufe;
mod rechnername;
mod sperre;
mod startfehler;
pub mod verbindungen;

use abrisspflege::{Pflege, pflege_aus};
use aufruf::{Aufruf, Portwunsch, Startauftrag};
use ende::Ende;
use plattenquote::Plattenquote;
use startfehler::Startfehler;
use verbindungen::Verbindungsgrenzen;

use crate::anwesenheit::status::Status;
use crate::anwesenheit::{Anwesenheit, gnadenfrist};
use crate::gespraeche::{Dateiablage, Gespraeche, Pflegeanlass, fristeintrag};
use crate::http::{self, Auskunft, Echt, Grenzschutz, Namensauskunft, Tabelle, Zustand};
use crate::live::bus::{self, Bus};
use crate::live::{self, herzschlag, takt};
use crate::protokoll::bereinigung::Anlass;
use crate::protokoll::episoden::Auftreten;
use crate::protokoll::episodenruhe;
use crate::protokoll::tagestakt::{self, Tagestakt};
use crate::protokoll::testpanik::Testpanik;
use crate::protokoll::wache::bewacht;
use crate::protokoll::{Bereich, Kontext, Protokoll, Schluessel, Stufe};
use crate::schreibkonto::Schreibkonto;
use crate::uhr::{self, Uhr};
use crate::zufall::Betriebssystemzufall;
use aufruf::HILFE;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

/// Exit-Code: geordnet beendet oder Hilfe. Die drei Codes stehen nur hier (ADR-0010 Ergänzung Karte #35, Regel 3).
pub const GELAUFEN: u8 = 0;
/// Exit-Code: Laufzeit, Ordner, Port oder Sperre scheitert.
pub const STARTFEHLER: u8 = 1;
/// Exit-Code: falscher Aufruf oder unlesbarer Wert aus der Umgebung.
pub const FALSCHER_AUFRUF: u8 = 2;

/// Führt einen Aufruf (Argumente ohne Programmnamen) bis zum Ende aus und liefert den Exit-Code:
/// 0 geordnet beendet oder Hilfe, 1 Ordner, Port oder Sperre scheitert, 2 falscher Aufruf oder
/// unlesbare Startzeit in `LOCALCHATA_JETZT`, Gnadenfrist in `LOCALCHATA_GNADENFRIST_MS` bzw. Herzschlag in
/// `LOCALCHATA_HERZSCHLAG_MS`. Eine Zahl statt `ExitCode`, weil `main` sie auf Unix an `_exit` gibt.
pub async fn ausfuehren<I: IntoIterator<Item = String>>(argumente: I) -> u8 {
    match Startauftrag::aus_argumenten(argumente) {
        Ok(Aufruf::Hilfe) => {
            println!("{HILFE}");
            GELAUFEN
        }
        Ok(Aufruf::Starten(mut auftrag)) => {
            let umgebung = || std::env::var_os(allenamen::VARIABLE);
            auftrag.alle_namen = match allenamen::wirksam(auftrag.alle_namen, umgebung) {
                Ok(alle_namen) => alle_namen,
                Err(fehler) => {
                    eprintln!("{fehler}");
                    return FALSCHER_AUFRUF;
                }
            };
            let startzeit = std::env::var_os(uhr::STARTZEIT_VARIABLE);
            let uhr = match uhr::startzeit_lesen(startzeit.as_deref()).and_then(uhr::aus_umgebung) {
                Ok(uhr) => uhr,
                Err(fehler) => {
                    eprintln!("{fehler}");
                    return FALSCHER_AUFRUF;
                }
            };
            let gnadenfrist_wert = std::env::var_os(gnadenfrist::VARIABLE);
            let gnadenfrist = match gnadenfrist::aus_umgebung(gnadenfrist_wert.as_deref()) {
                Ok(gnadenfrist) => gnadenfrist,
                Err(fehler) => {
                    eprintln!("{fehler}");
                    return FALSCHER_AUFRUF;
                }
            };
            let herzschlag_wert = std::env::var_os(herzschlag::VARIABLE);
            let herzschlag = match herzschlag::aus_umgebung(herzschlag_wert.as_deref()) {
                Ok(herzschlag) => herzschlag,
                Err(fehler) => {
                    eprintln!("{fehler}");
                    return FALSCHER_AUFRUF;
                }
            };
            let stufe_wert = std::env::var_os(protokollstufe::VARIABLE);
            let stufe = match protokollstufe::wirksame_stufe(
                auftrag.protokoll_stufe,
                stufe_wert.as_deref(),
            ) {
                Ok(stufe) => stufe,
                Err(fehler) => {
                    eprintln!("{fehler}");
                    return FALSCHER_AUFRUF;
                }
            };
            let quote_wert = std::env::var_os(plattenquote::VARIABLE);
            let quote =
                match plattenquote::wirksame_quote(auftrag.plattenquote, quote_wert.as_deref()) {
                    Ok(quote) => quote,
                    Err(fehler) => {
                        eprintln!("{fehler}");
                        return FALSCHER_AUFRUF;
                    }
                };
            let ruhe_wert = std::env::var_os(episodenruhe::VARIABLE);
            let ruhe = match episodenruhe::aus_umgebung(ruhe_wert.as_deref()) {
                Ok(ruhe) => ruhe,
                Err(fehler) => {
                    eprintln!("{fehler}");
                    return FALSCHER_AUFRUF;
                }
            };
            let testpanik = match Testpanik::aus_umgebung() {
                Ok(testpanik) => testpanik,
                Err(fehler) => {
                    eprintln!("{fehler}");
                    return FALSCHER_AUFRUF;
                }
            };
            let namensauskunft: Arc<dyn Namensauskunft> = match Tabelle::aus_umgebung() {
                Ok(Some(tabelle)) => Arc::new(tabelle),
                Ok(None) => Arc::new(Echt),
                Err(fehler) => {
                    eprintln!("{fehler}");
                    return FALSCHER_AUFRUF;
                }
            };
            match starten(
                auftrag,
                uhr,
                gnadenfrist,
                herzschlag,
                Knoepfe {
                    testpanik,
                    namensauskunft,
                },
                Wirksam { stufe, quote },
                ruhe,
            )
            .await
            {
                Ok(()) => GELAUFEN,
                Err(Startabbruch::VorDerSperre(fehler)) => {
                    eprintln!("{fehler}");
                    STARTFEHLER
                }
                Err(Startabbruch::Eingetragen) => STARTFEHLER,
            }
        }
        Err(fehler) => {
            eprintln!("{fehler}");
            FALSCHER_AUFRUF
        }
    }
}

/// Warum der Start endete, ohne zu laufen: vor der Sperre steht der Fehler auf stderr, danach im Protokoll.
enum Startabbruch {
    VorDerSperre(Startfehler),
    Eingetragen,
}

async fn starten(
    auftrag: Startauftrag,
    uhr: Box<dyn Uhr>,
    gnadenfrist: Duration,
    herzschlag: Duration,
    knoepfe: Knoepfe,
    Wirksam {
        stufe: (stufe, stufe_herkunft),
        quote,
    }: Wirksam,
    ruhe: Duration,
) -> Result<(), Startabbruch> {
    let uhr: Arc<dyn Uhr> = Arc::from(uhr);
    let ordner = ordner::ordner_vorbereiten(&auftrag.ordner).map_err(Startabbruch::VorDerSperre)?;
    let _sperre = sperre::ordner_sperren(&ordner).map_err(Startabbruch::VorDerSperre)?;
    let beginn = Instant::now();
    let protokoll =
        Arc::new(Protokoll::oeffnen(&ordner, Arc::clone(&uhr), stufe).mit_episodenruhe(ruhe));
    crate::protokoll::haken_setzen(Arc::clone(&protokoll));
    if auftrag.alle_namen {
        protokoll.eintragen(
            Stufe::Warning,
            Bereich::Grenzschutz,
            allenamen::RISIKO,
            Kontext::neu(),
        );
    }
    let bereinigt_am = uhr.jetzt().date_naive();
    protokoll.bereinigen(bereinigt_am, Anlass::Start);
    let laufen = nach_der_sperre(
        auftrag,
        uhr,
        gnadenfrist,
        herzschlag,
        ordner,
        Arc::clone(&protokoll),
        Startlaeufe {
            beginn,
            bereinigt_am,
            knoepfe,
            stufe_herkunft,
            quote,
        },
    );
    laufen.await.map_err(|fehler| {
        protokoll.eintragen(
            Stufe::Error,
            Bereich::Start,
            "Start gescheitert — die Instanz endet",
            Kontext::neu().mit(Schluessel::Grund, fehler),
        );
        Startabbruch::Eingetragen
    })
}

/// Was die Start-Läufe vor dem Laden ergaben: Beginn der Laufzeit und das Datum der Start-Bereinigung.
struct Startlaeufe {
    beginn: Instant,
    bereinigt_am: chrono::NaiveDate,
    knoepfe: Knoepfe,
    /// Woher die Mindeststufe kommt (`option=` im Start-Eintrag).
    stufe_herkunft: &'static str,
    /// Wirksame Plattenquote und woher sie kommt.
    quote: (Plattenquote, &'static str),
}

/// Was beim Start aus Option, Umgebung oder Standard wirksam wurde, je mit seiner Herkunft (ADR-0043 Regel 4).
struct Wirksam {
    stufe: (Stufe, &'static str),
    quote: (Plattenquote, &'static str),
}

/// Was von außen in den Lauf eingreift: der Testknopf für Panik und die Namensauskunft (echt oder, im
/// Debug-Build, die Tabelle aus `LOCALCHATA_TESTNAMEN`).
struct Knoepfe {
    testpanik: Testpanik,
    namensauskunft: Arc<dyn Namensauskunft>,
}

/// Die Gespräche und das Datum ihrer Pflege beim Start — der Startwert der Tagesaufgabe `fristpflege`.
struct Fristpflege {
    gespraeche: Arc<Gespraeche>,
    gepflegt_am: chrono::NaiveDate,
}

/// Der Tagestakt mit der Bereinigung und der Fristpflege als Tagesaufgaben, in dieser Reihenfolge; `zuletzt` je
/// Aufgabe = Datum ihres Start-Laufs (ADR-0041, Ergänzung „Vertrag des Tagestakts“, ADR-0030 Ergänzung).
fn tagestakt(
    uhr: &Arc<dyn Uhr>,
    protokoll: &Arc<Protokoll>,
    bereinigt_am: chrono::NaiveDate,
    fristpflege: Fristpflege,
) -> Tagestakt {
    let bereinigen = Arc::clone(protokoll);
    let episoden = Arc::clone(protokoll);
    let frist_protokoll = Arc::clone(protokoll);
    let Fristpflege {
        gespraeche,
        gepflegt_am,
    } = fristpflege;
    Tagestakt::neu(Arc::clone(uhr), Arc::clone(protokoll))
        .tagesaufgabe(
            "bereinigen",
            "alte Protokolldateien bleiben bis dahin liegen",
            bereinigt_am,
            Arc::new(move |heute| {
                bereinigen.bereinigen(heute, Anlass::Tageswechsel);
            }),
        )
        .tagesaufgabe(
            "fristpflege",
            "Tage jenseits der Frist bleiben bis dahin liegen",
            gepflegt_am,
            Arc::new(move |heute| {
                frist_pflegen(
                    &gespraeche,
                    heute,
                    Pflegeanlass::Tageswechsel,
                    &frist_protokoll,
                );
            }),
        )
        .sekundenaufgabe(
            "episoden",
            "Episoden enden nicht mehr",
            Box::new(move |_| episoden.episoden_schliessen()),
        )
}

/// Vom Laden bis zum Ende; jeder Fehler hier gehört ins Protokoll (ADR-0042 Regel 2, `start`).
async fn nach_der_sperre(
    auftrag: Startauftrag,
    uhr: Arc<dyn Uhr>,
    gnadenfrist: Duration,
    herzschlag: Duration,
    ordner: std::path::PathBuf,
    protokoll: Arc<Protokoll>,
    startlaeufe: Startlaeufe,
) -> Result<(), Startfehler> {
    let Startlaeufe {
        beginn,
        bereinigt_am,
        knoepfe: Knoepfe {
            testpanik,
            namensauskunft,
        },
        stufe_herkunft,
        quote: (quote, quote_herkunft),
    } = startlaeufe;
    let bus = Arc::new(Bus::neu(live::KAPAZITAET));
    let gespraeche = Arc::new(
        verlauf_laden(&ordner, Arc::clone(&uhr), &protokoll)?
            .mit_quote(quote.hoechstens())
            .mit_quote_melden(quote_melden(&protokoll))
            .mit_verdraengt_melden(verdraengt_melden(&protokoll))
            .mit_veroeffentlichen(bus::channels_veroeffentlichen(&bus))
            .mit_nachricht_veroeffentlichen(bus::nachrichten_veroeffentlichen(&bus)),
    );
    quote_beim_start_melden(&gespraeche, &protokoll);
    let gepflegt_am = uhr.jetzt().date_naive();
    frist_pflegen(&gespraeche, gepflegt_am, Pflegeanlass::Start, &protokoll);
    let anwesenheit = Arc::new(
        Anwesenheit::neu(
            Arc::clone(&uhr),
            Arc::new(Betriebssystemzufall),
            gnadenfrist,
        )
        .mit_veroeffentlichen(anwesenheit_veroeffentlichen(&bus, &protokoll)),
    );
    let listener = port::listener_binden(&auftrag.host, auftrag.port).await?;
    let adresse = listener.local_addr().map_err(|fehler| Startfehler::Port {
        host: auftrag.host.clone(),
        port: 0,
        grund: fehler.to_string(),
    })?;
    let (ende, signal) = ende_einrichten(Arc::clone(&protokoll), beginn);
    bewacht(
        Arc::clone(&protokoll),
        "tagestakt",
        "alte Protokolldateien und Tage jenseits der Frist bleiben liegen, Episoden enden nicht mehr",
        ende.clone(),
        tagestakt(
            &uhr,
            &protokoll,
            bereinigt_am,
            Fristpflege {
                gespraeche: Arc::clone(&gespraeche),
                gepflegt_am,
            },
        )
        .takten(ende.clone(), tagestakt::PERIODE),
    );
    bewacht(
        Arc::clone(&protokoll),
        "takt",
        "Anwesenheit wird nicht mehr gepflegt",
        ende.clone(),
        takt::takten(
            Arc::clone(&anwesenheit),
            Arc::clone(&uhr),
            Arc::clone(&protokoll),
            testpanik.clone(),
            ende.clone(),
            takt::PERIODE,
        ),
    );
    let kennzahlen = gespraeche.kennzahlen();
    protokoll.eintragen(
        Stufe::Information,
        Bereich::Start,
        "Instanz bereit",
        Kontext::neu()
            .mit(Schluessel::Version, env!("CARGO_PKG_VERSION"))
            .mit(Schluessel::Adresse, format!("http://{adresse}/"))
            .mit(Schluessel::Ordner, ordner.display())
            .mit(Schluessel::Stufe, protokoll.mindeststufe().als_wert())
            .mit(Schluessel::Option, stufe_herkunft)
            .mit(Schluessel::Plattenquote, quote.als_wert())
            .mit(Schluessel::Belegt, kennzahlen.log_bytes)
            .mit(Schluessel::PlattenquoteOption, quote_herkunft)
            .mit(Schluessel::Anzahl, kennzahlen.nachrichten)
            .mit(Schluessel::DauerMs, kennzahlen.ladezeit_ms),
    );
    startzeile_schreiben(&ordner, adresse);
    if auftrag.browser_oeffnen {
        browser::oeffnen(adresse, Arc::clone(&protokoll));
    }
    let app = http::router(Zustand {
        ordner,
        gespraeche,
        anwesenheit,
        bus,
        herzschlag,
        ende: ende.clone(),
        grenzschutz: Grenzschutz::neu(rechnername::lesen(), auftrag.erreichbar_als)
            .mit_auskunft(Arc::new(Auskunft::neu(
                namensauskunft,
                Arc::clone(&uhr),
                Arc::clone(&protokoll),
            )))
            .mit_alle_namen(auftrag.alle_namen),
        werkbank: http::Werkbank::aus_kernen(),
        schreibkonto: Arc::new(Schreibkonto::neu(Arc::clone(&uhr))),
        protokoll: Arc::clone(&protokoll),
        testpanik,
    });
    ausliefern::ausliefern(
        listener,
        app,
        ende,
        Verbindungsgrenzen::fuer_diesen_prozess(),
        Pflege::kernel(pflege_aus(herzschlag), Arc::clone(&protokoll)),
        Arc::clone(&protokoll),
    )
    .await;
    protokoll.alle_episoden_schliessen();
    protokoll.eintragen(
        Stufe::Information,
        Bereich::Ende,
        "Instanz beendet",
        ende_kontext(signal.get().copied(), beginn),
    );
    Ok(())
}

/// Lädt den Verlauf vor dem Binden — ab der Startzeile ist er vollständig da (A00003).
/// Jeder Defekt ist eine WARNING `laden` im Protokoll (ADR-0042 Regel 2).
fn verlauf_laden(
    ordner: &Path,
    uhr: Arc<dyn Uhr>,
    protokoll: &Arc<Protokoll>,
) -> Result<Gespraeche, Startfehler> {
    let ablage = Dateiablage::neu(ordner, Arc::clone(protokoll));
    let gespraeche =
        Gespraeche::laden(Arc::new(ablage), uhr).map_err(|fehler| Startfehler::Laden {
            ordner: ordner.to_path_buf(),
            grund: fehler.to_string(),
        })?;
    for eintrag in ladedefekte::eintraege(gespraeche.defekte()) {
        protokoll.eintragen(
            eintrag.stufe,
            eintrag.bereich,
            eintrag.text,
            eintrag.kontext,
        );
    }
    Ok(gespraeche)
}

/// Meldungen der Quote ins Betriebsprotokoll (ADR-0044 Regel 7).
fn quote_melden(protokoll: &Arc<Protokoll>) -> crate::gespraeche::quote::QuoteMelden {
    let protokoll = Arc::clone(protokoll);
    Arc::new(move |meldung: &crate::gespraeche::quote::Quotemeldung| {
        let eintrag = crate::gespraeche::quote::meldung_eintrag(meldung);
        protokoll.eintragen(
            eintrag.stufe,
            eintrag.bereich,
            eintrag.text,
            eintrag.kontext,
        );
    })
}

/// Verdrängte Wiederholschlüssel als WARNING-Episode `senden` je verdrängter Kennung (ADR-0042 Ergänzung Karte #36
/// Punkt 1): Beginn mit `kennung`, `grenze`, `hoechstens`, `alter_ms`, Ende mit `kennung` und der Zählung je Grenze;
/// nie Schlüssel, nie Text.
fn verdraengt_melden(protokoll: &Arc<Protokoll>) -> crate::gespraeche::VerdraengtMelden {
    let protokoll = Arc::clone(protokoll);
    Arc::new(
        move |verdraengt: &crate::gespraeche::wiederholung::Verdraengt| {
            let kennung = verdraengt.kennung.als_text();
            protokoll.episode(Auftreten {
                stufe: Stufe::Warning,
                bereich: Bereich::Senden,
                text: crate::protokoll::episodentexte::SCHLUESSEL_VERDRAENGT,
                quelle: kennung.to_string(),
                kontext: Kontext::neu()
                    .mit(Schluessel::Kennung, kennung)
                    .mit_nur_beginn(Schluessel::Grenze, verdraengt.grenze.als_wert())
                    .mit_nur_beginn(Schluessel::Hoechstens, verdraengt.hoechstens)
                    .mit_nur_beginn(Schluessel::AlterMs, verdraengt.alter_ms),
                art: Some(verdraengt.grenze.als_wert()),
                verworfen: None,
                bytes: None,
                anzeige: None,
            });
        },
    )
}

/// Nach dem Laden einmal: über der Quote bzw. über 90 % steht eine WARNING; geladen ist trotzdem alles.
fn quote_beim_start_melden(gespraeche: &Gespraeche, protokoll: &Protokoll) {
    if let Some(meldung) = gespraeche.quote_beim_start() {
        let eintrag = crate::gespraeche::quote::meldung_eintrag(&meldung);
        protokoll.eintragen(
            eintrag.stufe,
            eintrag.bereich,
            eintrag.text,
            eintrag.kontext,
        );
    }
}

/// Pflegt alle Channels nach ihrer Frist und trägt jeden Bericht ins Protokoll ein (R00017: nie selbst auf stderr).
fn frist_pflegen(
    gespraeche: &Gespraeche,
    heute: chrono::NaiveDate,
    anlass: Pflegeanlass,
    protokoll: &Protokoll,
) {
    for bericht in gespraeche.pflegen(heute, anlass) {
        let eintrag = fristeintrag::pflege_eintrag(&bericht);
        protokoll.eintragen(
            eintrag.stufe,
            eintrag.bereich,
            eintrag.text,
            eintrag.kontext,
        );
    }
}

/// Wechsel an den Bus; online/offline zusätzlich als DEBUG `sitzung` (ADR-0042 Regel 2).
fn anwesenheit_veroeffentlichen(
    bus: &Arc<Bus>,
    protokoll: &Arc<Protokoll>,
) -> crate::anwesenheit::Veroeffentlichen {
    let an_den_bus = bus::anwesenheit_veroeffentlichen(bus);
    let protokoll = Arc::clone(protokoll);
    Arc::new(move |wechsel: &crate::anwesenheit::Wechsel| {
        let text = match wechsel.teilnehmer.status {
            Status::Online => Some("Sitzung online"),
            Status::Offline => Some("Sitzung offline"),
            Status::Weg => None,
        };
        if let Some(text) = text {
            protokoll.eintragen(
                Stufe::Debug,
                Bereich::Sitzung,
                text,
                Kontext::neu().mit(Schluessel::Kennung, wechsel.teilnehmer.kennung.als_text()),
            );
        }
        an_den_bus(wechsel);
    })
}

/// Die einzige Zeile auf stdout; Tests und Director lesen die Adresse daraus (X00001).
fn startzeile_schreiben(ordner: &Path, adresse: SocketAddr) {
    println!(
        "localchatA läuft für {} unter http://{adresse}/",
        ordner.display()
    );
}

/// Das Ende-Signal für alle: trifft Strg+C oder SIGTERM ein, steht `INFORMATION ende` im Protokoll und das Ende
/// wird gemeldet; welches Signal es war, steht danach im zweiten Wert.
fn ende_einrichten(
    protokoll: Arc<Protokoll>,
    beginn: Instant,
) -> (Ende, Arc<OnceLock<&'static str>>) {
    let (melden, ende) = Ende::neu();
    let signal = Arc::new(OnceLock::new());
    let gemerkt = Arc::clone(&signal);
    let beenden = beenden_einrichten(&protokoll);
    tokio::spawn(async move {
        let grund = beenden.await;
        let _ = gemerkt.set(grund);
        protokoll.eintragen(
            Stufe::Information,
            Bereich::Ende,
            "Ende-Signal empfangen — die Instanz endet",
            ende_kontext(Some(grund), beginn),
        );
        melden.melden();
    });
    (ende, signal)
}

/// `grund` (welches Signal) und `dauer_ms` (Laufzeit seit der Sperre).
fn ende_kontext(grund: Option<&str>, beginn: Instant) -> Kontext {
    Kontext::neu()
        .mit(Schluessel::Grund, grund.unwrap_or("unbekannt"))
        .mit(Schluessel::DauerMs, beginn.elapsed().as_millis())
}

/// Richtet den Empfang von Strg+C und SIGTERM sofort ein — vor der Startzeile, denn ab ihr gilt die Instanz
/// als bereit, und ein Signal ab da muss geordnet beenden. Das Ergebnis endet, sobald eines eintrifft.
#[cfg(unix)]
fn beenden_einrichten(
    protokoll: &Protokoll,
) -> impl Future<Output = &'static str> + Send + 'static {
    use tokio::signal::unix::SignalKind;
    let strg_c = signal_einrichten(SignalKind::interrupt(), "Strg+C", protokoll);
    let sigterm = signal_einrichten(SignalKind::terminate(), "SIGTERM", protokoll);
    async move {
        tokio::select! {
            _ = eintreffen(strg_c) => "Strg+C",
            _ = eintreffen(sigterm) => "SIGTERM",
        }
    }
}

/// Lässt sich ein Signal nicht abfangen, steht eine WARNING im Protokoll; beendet wird dann nur über das andere.
#[cfg(unix)]
fn signal_einrichten(
    art: tokio::signal::unix::SignalKind,
    name: &str,
    protokoll: &Protokoll,
) -> Option<tokio::signal::unix::Signal> {
    match tokio::signal::unix::signal(art) {
        Ok(signal) => Some(signal),
        Err(fehler) => {
            signal_warnen(protokoll, name, &fehler);
            None
        }
    }
}

fn signal_warnen(protokoll: &Protokoll, name: &str, fehler: &std::io::Error) {
    protokoll.eintragen(
        Stufe::Warning,
        Bereich::Start,
        format!("{name} lässt sich nicht abfangen — beendet wird nur über das andere Signal"),
        Kontext::neu().mit(Schluessel::Grund, fehler),
    );
}

#[cfg(unix)]
async fn eintreffen(signal: Option<tokio::signal::unix::Signal>) {
    match signal {
        Some(mut signal) => {
            signal.recv().await;
        }
        None => std::future::pending::<()>().await,
    }
}

/// Wie unter Unix, nur mit Strg+C; lässt es sich nicht abfangen, wird nie beendet statt sofort.
#[cfg(windows)]
fn beenden_einrichten(
    protokoll: &Protokoll,
) -> impl Future<Output = &'static str> + Send + 'static {
    let strg_c = tokio::signal::windows::ctrl_c();
    if let Err(fehler) = &strg_c {
        signal_warnen(protokoll, "Strg+C", fehler);
    }
    async move {
        match strg_c {
            Ok(mut strg_c) => {
                strg_c.recv().await;
                "Strg+C"
            }
            Err(_) => std::future::pending::<&'static str>().await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anwesenheit::{Art, Teilnehmerzeile, Wechsel};
    use crate::gespraeche::Kennung;
    use crate::name::Name;
    use crate::protokoll::testhilfe::{im_speicher, zeit};

    fn wechsel(stand: u64, status: Status) -> Wechsel {
        Wechsel {
            stand,
            jetzt: zeit("2026-10-28T12:00:00+01:00"),
            teilnehmer: Teilnehmerzeile {
                kennung: Kennung::bilden("alpha@127.0.0.2").expect("Kennung"),
                name: Name::bilden("alpha").expect("Name"),
                herkunft: "127.0.0.2".parse().expect("IP"),
                art: Art::Agent,
                status,
                seit: zeit("2026-10-28T12:00:00+01:00"),
                weg_um: None,
            },
        }
    }

    /// Code-Review #24 B1, Abnahme-Probe 4 (Teil Anwesenheit): auf Stufe debug je Wechsel nach online bzw.
    /// offline genau eine DEBUG-Zeile mit der Kennung; ein Wechsel nach `weg` schreibt hier keine.
    #[test]
    fn online_und_offline_sind_je_genau_eine_debug_zeile_mit_kennung_weg_keine() {
        let (protokoll, speicher, _, _) = im_speicher("2026-10-28T12:00:00+01:00", Stufe::Debug);
        let veroeffentlichen = anwesenheit_veroeffentlichen(&Arc::new(Bus::neu(16)), &protokoll);

        veroeffentlichen(&wechsel(1, Status::Online));
        veroeffentlichen(&wechsel(2, Status::Offline));
        veroeffentlichen(&wechsel(3, Status::Weg));

        assert_eq!(
            speicher.zeilen(),
            [
                "2026-10-28T12:00:00.000+01:00 DEBUG sitzung Sitzung online | kennung=alpha@127.0.0.2",
                "2026-10-28T12:00:00.000+01:00 DEBUG sitzung Sitzung offline | kennung=alpha@127.0.0.2",
            ]
        );
    }

    /// Ein gültiger Satz von genau `bytes` Bytes für die Speicherablage.
    fn satz_mit(zeit: &str, bytes: usize) -> String {
        let kopf = format!("@ {zeit} a@127.0.0.9\n");
        format!("{kopf}  {}\n\n", "x".repeat(bytes - kopf.len() - 4))
    }

    /// Gespräche aus der Speicherablage mit `allgemein` und Tagen der Größen `tage` (Datum, Bytes), Quote, Meldungen
    /// ins Protokoll im Speicher.
    fn gespraeche_mit(
        tage: &[(&str, usize)],
        quote: Option<u64>,
        protokoll: &Arc<Protokoll>,
    ) -> Gespraeche {
        use crate::gespraeche::{Gespraech, Speicherablage};
        let ablage = Arc::new(Speicherablage::neu());
        let allgemein = Gespraech::channel("allgemein");
        ablage.gespraech_anlegen(&allgemein);
        for (tag, bytes) in tage {
            ablage.tag_setzen(
                &allgemein,
                chrono::NaiveDate::parse_from_str(tag, "%Y-%m-%d").expect("Tag"),
                &satz_mit(&format!("{tag}T09:00:00.000+01:00"), *bytes),
            );
        }
        let uhr = Arc::new(crate::uhr::GestellteUhr::neu(zeit(
            "2026-10-28T12:00:00+01:00",
        )));
        Gespraeche::laden(ablage, uhr)
            .expect("geladen")
            .mit_quote(quote)
            .mit_quote_melden(quote_melden(protokoll))
    }

    fn quote_zeilen(speicher: &crate::protokoll::senke::Speicher) -> Vec<String> {
        speicher
            .zeilen()
            .into_iter()
            .filter(|zeile| zeile.contains(" quote "))
            .map(|zeile| zeile.split_once(' ').expect("Zeitstempel").1.to_string())
            .collect()
    }

    /// Gespräche aus der Speicherablage mit `allgemein`, einem Wiederholspeicher mit Gesamtgrenze `gesamt` und den
    /// Verdrängungen ins Protokoll im Speicher; die Uhr der Gespräche ist gestellt.
    fn verdraengend(
        gesamt: usize,
        protokoll: &Arc<Protokoll>,
    ) -> (Gespraeche, Arc<crate::uhr::GestellteUhr>) {
        use crate::gespraeche::{Gespraech, Speicherablage, wiederholung::Wiederholung};
        let ablage = Arc::new(Speicherablage::neu());
        ablage.gespraech_anlegen(&Gespraech::channel("allgemein"));
        let uhr = Arc::new(crate::uhr::GestellteUhr::neu(zeit(
            "2026-10-28T12:00:00+01:00",
        )));
        let gespraeche = Gespraeche::laden(ablage, Arc::clone(&uhr) as Arc<dyn crate::uhr::Uhr>)
            .expect("geladen")
            .mit_wiederholung(Wiederholung::neu(640, gesamt).expect("Grenzen"))
            .mit_verdraengt_melden(verdraengt_melden(protokoll));
        (gespraeche, uhr)
    }

    /// Je Kennung `k0` … eine Nachricht `GEHEIMTEXT` mit Schlüssel `GEHEIM<n>`, im Abstand von 1 s.
    fn geheim_senden(
        gespraeche: &Gespraeche,
        uhr: &crate::uhr::GestellteUhr,
        anzahl: usize,
        ab_ms: i64,
    ) {
        let allgemein = crate::gespraeche::Gespraech::channel("allgemein");
        for nummer in 0..anzahl {
            uhr.stellen(
                zeit("2026-10-28T12:00:00+01:00")
                    + chrono::Duration::milliseconds(ab_ms + 1_000 * nummer as i64),
            );
            let wer = Kennung::bilden(&format!("k{nummer}@127.0.0.1")).expect("Kennung");
            gespraeche
                .senden_einmal(
                    &allgemein,
                    &wer,
                    Art::Agent,
                    "GEHEIMTEXT",
                    &format!("GEHEIM{nummer}"),
                )
                .expect("gesendet");
        }
    }

    fn senden_zeilen(speicher: &crate::protokoll::senke::Speicher) -> Vec<String> {
        speicher
            .zeilen()
            .into_iter()
            .filter(|zeile| zeile.contains(" senden "))
            .map(|zeile| zeile.split_once(' ').expect("Zeitstempel").1.to_string())
            .collect()
    }

    /// R00025 „Verdrängung sichtbar“ (Speicher-Senke, gesamt 5, Stufe Debug): die sechste Kennung verdrängt den
    /// Eintrag der ersten → genau eine Beginn-Zeile mit `kennung=` der verdrängten, `grenze=gesamt`,
    /// `hoechstens=5`, `alter_ms=`; das Ende zählt je Grenze; nie Schlüssel oder Text; Zähler 1.
    #[test]
    fn eine_verdraengung_steht_als_episode_ohne_schluessel_und_text_und_zaehlt() {
        let (protokoll, speicher, _, _) = im_speicher("2026-10-28T12:00:00+01:00", Stufe::Debug);
        let (gespraeche, uhr) = verdraengend(5, &protokoll);

        geheim_senden(&gespraeche, &uhr, 6, 0);
        protokoll.alle_episoden_schliessen();

        assert_eq!(
            senden_zeilen(&speicher),
            [
                "WARNING senden Wiederholschlüssel vor Ablauf verdrängt — eine Wiederholung kann doppelt ankommen — Episode beginnt | kennung=k0@127.0.0.1 grenze=gesamt hoechstens=5 alter_ms=5000",
                "WARNING senden Wiederholschlüssel vor Ablauf verdrängt — eine Wiederholung kann doppelt ankommen — Episode endet: 1 in 0 s (gesamt 1) | kennung=k0@127.0.0.1",
            ]
        );
        assert!(
            speicher
                .zeilen()
                .iter()
                .all(|zeile| !zeile.contains("GEHEIM"))
        );
        assert_eq!(gespraeche.kennzahlen().schluessel_verdraengt, 1);
    }

    /// Kontrolle (Angriffsmatrix Zeile 8, K2): ist der älteste abgelaufen, fällt er ohne Zeile, Zähler 0.
    #[test]
    fn ein_abgelaufener_eintrag_ergibt_keine_zeile_und_keinen_zaehler() {
        let (protokoll, speicher, _, _) = im_speicher("2026-10-28T12:00:00+01:00", Stufe::Debug);
        let (gespraeche, uhr) = verdraengend(5, &protokoll);
        geheim_senden(&gespraeche, &uhr, 5, 0);
        let allgemein = crate::gespraeche::Gespraech::channel("allgemein");
        uhr.stellen(zeit("2026-10-28T12:10:00.001+01:00"));

        gespraeche
            .senden_einmal(
                &allgemein,
                &Kennung::bilden("neu@127.0.0.1").expect("Kennung"),
                Art::Agent,
                "x",
                "n1",
            )
            .expect("gesendet");
        protokoll.alle_episoden_schliessen();

        assert_eq!(senden_zeilen(&speicher), Vec::<String>::new());
        assert_eq!(gespraeche.kennzahlen().schluessel_verdraengt, 0);
    }

    /// R00025 „Nach der Sperre und Episodenflut“ (Angriffsmatrix Zeile 16, gesamt 1): 2.000 verschiedene Kennungen
    /// werden verdrängt → höchstens 1.024 offene Episoden und die Sammelepisode, nichts über dem Budget verworfen,
    /// der Zähler genau 2.000.
    #[test]
    fn eine_flut_von_verdraengungen_bleibt_in_den_episoden_und_wird_genau_gezaehlt() {
        let (protokoll, speicher, _, _) = im_speicher("2026-10-28T12:00:00+01:00", Stufe::Debug);
        let (gespraeche, uhr) = verdraengend(1, &protokoll);

        geheim_senden(&gespraeche, &uhr, 2_001, 0);

        let beginne = senden_zeilen(&speicher)
            .iter()
            .filter(|zeile| zeile.contains("Episode beginnt"))
            .count();
        assert_eq!(beginne, 1_024 + 1);
        assert_eq!(gespraeche.kennzahlen().schluessel_verdraengt, 2_000);
        assert_eq!(protokoll.kennzahlen().verworfen, 0);
    }

    /// B0362 (R00023 F0076 „Wechsel melden“, Speicher-Senke, Quote 1.000; Sätze ab 53 Bytes, darum 790 + 60
    /// geladen): Senden über 90 % → genau eine WARNING; noch einmal → keine; Verkürzen nimmt den 790-Byte-Tag →
    /// genau eine INFORMATION mit `grund=frist_gesetzt`.
    #[test]
    fn wechsel_beim_senden_und_verkuerzen_stehen_je_einmal_im_protokoll() {
        let (protokoll, speicher, _, _) = im_speicher("2026-10-28T12:00:00+01:00", Stufe::Debug);
        let gespraeche = gespraeche_mit(
            &[("2026-10-26", 790), ("2026-10-27", 60)],
            Some(1_000),
            &protokoll,
        );
        let allgemein = crate::gespraeche::Gespraech::channel("allgemein");
        let beta = Kennung::bilden("beta@127.0.0.1").expect("Kennung");

        gespraeche
            .senden(&allgemein, &beta, crate::anwesenheit::Art::Agent, "1")
            .expect("gesendet");
        gespraeche
            .senden(&allgemein, &beta, crate::anwesenheit::Art::Agent, "2")
            .expect("gesendet");
        gespraeche
            .frist_setzen("allgemein", 2, true, &beta)
            .expect("verkürzt");

        assert_eq!(
            quote_zeilen(&speicher),
            [
                "WARNING quote Gespräche belegen 90 % der Quote | belegt=915 hoechstens=1000",
                "INFORMATION quote Gespräche wieder unter 90 % der Quote | belegt=190 hoechstens=1000 grund=frist_gesetzt",
            ]
        );
    }

    /// Dieselbe Zeile beim Tageswechsel trägt `grund=tageswechsel`; mit `aus` gibt es keine `quote`-Zeile.
    #[test]
    fn der_tageswechsel_meldet_mit_seinem_grund_aus_meldet_nichts() {
        for (quote, erwartet) in [
            (
                Some(1_000),
                vec![
                    "WARNING quote Gespräche belegen 90 % der Quote | belegt=915 hoechstens=1000",
                    "INFORMATION quote Gespräche wieder unter 90 % der Quote | belegt=125 hoechstens=1000 grund=tageswechsel",
                ],
            ),
            (None, vec![]),
        ] {
            let (protokoll, speicher, _, _) =
                im_speicher("2026-10-28T12:00:00+01:00", Stufe::Debug);
            let gespraeche = gespraeche_mit(
                &[("2026-10-26", 790), ("2026-10-27", 60)],
                quote,
                &protokoll,
            );
            let beta = Kennung::bilden("beta@127.0.0.1").expect("Kennung");
            gespraeche
                .senden(
                    &crate::gespraeche::Gespraech::channel("allgemein"),
                    &beta,
                    crate::anwesenheit::Art::Agent,
                    "1",
                )
                .expect("gesendet");

            let heute = chrono::NaiveDate::parse_from_str("2026-11-03", "%Y-%m-%d").expect("Tag");
            frist_pflegen(&gespraeche, heute, Pflegeanlass::Tageswechsel, &protokoll);

            assert_eq!(quote_zeilen(&speicher), erwartet, "{quote:?}");
        }
    }

    /// B0362 (R00023 F0076 „Start“, Speicher-Senke): über der Quote nur „mehr als die Quote“, über 90 % die
    /// 90-%-Zeile, mit `aus` nichts. Mutante „keine Prüfung beim Start“ wird rot.
    #[test]
    fn der_start_meldet_den_stand_der_quote_einmal() {
        for (bytes, quote, erwartet) in [
            (
                1_001,
                Some(1_000),
                vec![
                    "WARNING quote Gespräche belegen mehr als die Quote — Senden wird abgewiesen | belegt=1001 hoechstens=1000",
                ],
            ),
            (
                950,
                Some(1_000),
                vec!["WARNING quote Gespräche belegen 90 % der Quote | belegt=950 hoechstens=1000"],
            ),
            (900, Some(1_000), vec![]),
            (2_000, None, vec![]),
        ] {
            let (protokoll, speicher, _, _) =
                im_speicher("2026-10-28T12:00:00+01:00", Stufe::Debug);
            let gespraeche = gespraeche_mit(&[("2026-10-27", bytes)], quote, &protokoll);

            quote_beim_start_melden(&gespraeche, &protokoll);

            assert_eq!(quote_zeilen(&speicher), erwartet, "{bytes} {quote:?}");
        }
    }
}
