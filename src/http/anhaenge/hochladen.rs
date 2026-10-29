//! Stücke schreiben (B0429, ADR-0046 Regel 2 mit Ergänzungen „Karte #9 Gate“ Regeln 1, 3, 5, 6, 8 und „F1′“ Regel
//! 3, „Karte #9 Nacharbeit F1“ Regel 3): `PUT /api/anhaenge/{id}` mit Sitzung. Vor dem ersten gelesenen Byte 404 ·
//! 409 · 429 `laufend_kennung` | `laufend_herkunft` | `offen_gesamt` · 415 · 411 · 422 `laenge`, je mit `Connection: close` und nie `100 Continue` (der Körper wird nicht angefasst). Danach
//! ohne Quote-Prüfung stückweise unter der Hochladebank nach `.eingang/<id>.teil` → `rename` ohne Überschreiben →
//! 200. Frist, Lesefehler, Client weg, Schreibfehler, `DELETE` und Lebensende brechen ab: die Teildatei ist weg,
//! die Anmeldung gilt weiter bis `min(jetzt + 60 s, Lebensende)`.

use super::super::Zustand;
use super::super::anfrageprotokoll::mit_regel;
use super::super::teilnehmer::Teilnehmer;
use super::super::teilnehmerliste::zeit_als_text;
use crate::gespraeche::Gespraeche;
use crate::gespraeche::anhangablage::Teildatei;
use crate::gespraeche::anhangeingang::{Abbruchgrund, Beginnfehler, Kopf, Platz, Verloren};
use crate::gespraeche::anhangid::Anhangid;
use crate::gespraeche::anhangpruefung::Art;
use crate::gespraeche::anhangpruefung::erkenner::Erkenner;
use crate::gespraeche::anhangpruefung::tilger::port::Abbruch;
use crate::gespraeche::anhangpruefung::tilger::{Getilgt, tilgen};
use crate::gespraeche::hochladefristen::{Fristart, Fristen, STUECKFRIST, gesamtfrist};
use crate::protokoll::episoden::Auftreten;
use crate::protokoll::{Bereich, Kontext, Schluessel, Stufe};
use axum::Json;
use axum::body::{Body, Bytes};
use axum::extract::{Path, State};
use axum::http::header::{
    CONNECTION, CONTENT_LENGTH, CONTENT_TYPE, RETRY_AFTER, TRANSFER_ENCODING,
};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use futures_util::{Stream, StreamExt};
use serde_json::{Value, json};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tokio::sync::Semaphore;

/// Erlaubnisse der Hochladebank (ADR-0046 Frage 7): höchstens so viele Blocking-Threads schreiben Stücke.
pub const HOCHLADEBANK: usize = 8;
/// So viele Bytes sammelt eine Hochladung, bevor sie ein Stück schreibt — höchstens ein Stück im Speicher.
pub const STUECK: usize = 256 * 1024;

pub async fn hochladen(
    teilnehmer: Teilnehmer,
    State(zustand): State<Arc<Zustand>>,
    Path(id): Path<String>,
    kopf: HeaderMap,
    koerper: Body,
) -> Response {
    let wer = teilnehmer.kennung.als_text().to_string();
    let Some(id) = Anhangid::lesen(&id) else {
        return abgewiesen(Beginnfehler::Unbekannt, &wer);
    };
    let angaben = Kopf {
        typ_passt: roh_angekuendigt(&kopf),
        laenge: laenge_angekuendigt(&kopf),
    };
    let platz = match zustand
        .gespraeche
        .hochladen_beginnen(&teilnehmer.kennung, &id, angaben)
    {
        Ok(platz) => platz,
        Err(fehler) => return abgewiesen(fehler, &wer),
    };
    let beginn = std::time::Instant::now();
    let ergebnis = uebertragen(
        Arc::clone(&zustand.gespraeche),
        Arc::clone(&zustand.hochladebank),
        platz,
        koerper.into_data_stream(),
    )
    .await;
    ende_melden(
        &zustand.protokoll,
        &teilnehmer,
        &id,
        &ergebnis,
        beginn.elapsed(),
    );
    match &ergebnis {
        Ergebnis::Fertig {
            getilgt: Some(getilgt),
            ..
        } => zustand.protokoll.eintragen(
            Stufe::Debug,
            Bereich::Anhang,
            "Metadaten getilgt",
            Kontext::neu()
                .mit(Schluessel::Art, getilgt.art)
                .mit(Schluessel::Bloecke, getilgt.bloecke)
                .mit(Schluessel::Ort, if getilgt.ort { "ja" } else { "nein" }),
        ),
        Ergebnis::Frist(_) | Ergebnis::Lebensende => {
            zustand
                .protokoll
                .abgewiesen(crate::protokoll::zaehler::Abweisungsart::ZuLangsam);
            zustand.protokoll.episode(Auftreten {
                stufe: Stufe::Warning,
                bereich: Bereich::Anhang,
                text: crate::protokoll::episodentexte::HOCHLADEN_ZU_LANGSAM,
                quelle: teilnehmer.herkunft.to_string(),
                kontext: Kontext::neu().mit(Schluessel::Herkunft, teilnehmer.herkunft),
                art: None,
                verworfen: None,
                bytes: None,
                anzeige: None,
            });
        }
        Ergebnis::Metadaten(grund) => zustand
            .protokoll
            .episode(metadaten_auftreten(teilnehmer.herkunft, grund)),
        _ => {}
    }
    if let Ergebnis::Fehler(grund) | Ergebnis::Platte { grund, .. } = &ergebnis {
        zustand.protokoll.eintragen(
            Stufe::Error,
            Bereich::Anhang,
            "Anhang nicht geschrieben — Hochladen abgebrochen, die Teildatei ist gelöscht",
            Kontext::neu()
                .mit(Schluessel::Herkunft, teilnehmer.herkunft)
                .mit(Schluessel::Datei, format!(".eingang/{}", id.praefix()))
                .mit(Schluessel::Grund, grund),
        );
    }
    beantworten(ergebnis, &wer)
}

/// Jede Antwort der Route `PUT /api/anhaenge/{id}` schließt die Verbindung, gleich welcher Status — auch die
/// Abweisungen der Extraktoren (ADR-0046 Ergänzung „Connection: close auf jeder PUT-Antwort“, Regel C2): Bytes
/// nach dem Körper werden nie als zweite Anfrage gelesen.
pub async fn verbindung_schliessen(mut antwort: Response) -> Response {
    antwort
        .headers_mut()
        .insert(CONNECTION, axum::http::HeaderValue::from_static("close"));
    antwort
}

/// Ab dieser Länge steht das Ende eines Hochladens als `INFORMATION` im Protokoll (ADR-0042 Ergänzung Karte #9).
pub const GROSSES_HOCHLADEN: u64 = 64 * 1_048_576;

/// Ende und Abbruch einer Übertragung (B0462): `DEBUG anhang` mit `kennung`, `herkunft`, `bytes`, `art`,
/// `dauer_ms`; ab 64 MiB zusätzlich `INFORMATION anhang Großes Hochladen abgeschlossen` mit `bytes`, `dauer_ms`,
/// `mib_s`. Die Datei nur als ID-Präfix des Eingangs.
fn ende_melden(
    protokoll: &crate::protokoll::Protokoll,
    teilnehmer: &Teilnehmer,
    id: &Anhangid,
    ergebnis: &Ergebnis,
    dauer: Duration,
) {
    let dauer_ms = u64::try_from(dauer.as_millis()).unwrap_or(u64::MAX);
    let kontext = Kontext::neu()
        .mit(Schluessel::Kennung, teilnehmer.kennung.als_text())
        .mit(Schluessel::Herkunft, teilnehmer.herkunft)
        .mit(Schluessel::Datei, format!(".eingang/{}", id.praefix()));
    match ergebnis {
        Ergebnis::Fertig { bytes, art, .. } => {
            protokoll.eintragen(
                Stufe::Debug,
                Bereich::Anhang,
                "Hochladen abgeschlossen",
                kontext
                    .mit(Schluessel::Bytes, *bytes)
                    .mit(Schluessel::Art, art.typ)
                    .mit(Schluessel::DauerMs, dauer_ms),
            );
            if *bytes >= GROSSES_HOCHLADEN {
                let sekunden = dauer.as_secs_f64().max(0.001);
                let mib_s = *bytes as f64 / 1_048_576.0 / sekunden;
                protokoll.eintragen(
                    Stufe::Information,
                    Bereich::Anhang,
                    "Großes Hochladen abgeschlossen",
                    Kontext::neu()
                        .mit(Schluessel::Bytes, *bytes)
                        .mit(Schluessel::DauerMs, dauer_ms)
                        .mit(Schluessel::MibS, format!("{mib_s:.1}")),
                );
            }
        }
        anders => protokoll.eintragen(
            Stufe::Debug,
            Bereich::Anhang,
            "Hochladen abgebrochen",
            kontext
                .mit(Schluessel::Grund, abbruchgrund(anders))
                .mit(Schluessel::DauerMs, dauer_ms),
        ),
    }
}

fn abbruchgrund(ergebnis: &Ergebnis) -> &'static str {
    match ergebnis {
        Ergebnis::Fertig { .. } => "fertig",
        Ergebnis::Metadaten(_) => "metadaten",
        Ergebnis::Frist(Fristart::Stueckfrist) => "stueckfrist",
        Ergebnis::Frist(Fristart::Gesamtfrist) => "gesamtfrist",
        Ergebnis::Lebensende => "lebensende",
        Ergebnis::Verloren(_) => "verloren",
        Ergebnis::Platte { .. } => "platte",
        Ergebnis::Fehler(_) => "fehler",
        Ergebnis::Abgebrochen => "abgebrochen",
    }
}

/// Wie eine Übertragung endete.
#[derive(Debug)]
pub enum Ergebnis {
    Fertig {
        id: Anhangid,
        name: String,
        bytes: u64,
        art: Art,
        gueltig_bis: crate::uhr::Zeit,
        getilgt: Option<Getilgt>,
    },
    /// Nicht sicher tilgbar (422 `metadaten`): Teildatei gelöscht, Eintrag verworfen, Anspruch frei.
    Metadaten(&'static str),
    /// Stück- oder Gesamtfrist (408).
    Frist(Fristart),
    /// Das Lebensende kam während des Hochladens (408).
    Lebensende,
    /// `DELETE`, Ablauf oder Verlust des Übergangs (404).
    Verloren(Verloren),
    /// Kein Platz auf der Platte beim Schreiben (507 `platte`).
    Platte { frei: u64, grund: String },
    /// Schreibfehler oder ein vorhandenes Ziel (500, M24, M39).
    Fehler(String),
    /// Der Client ist weg oder schickte weniger Bytes; niemand liest die Antwort.
    Abgebrochen,
}

/// Räumt, wenn eine Übertragung ohne Abschluss endet — auch, wenn der Handler fallen gelassen wird (Client weg).
/// Nach dem `rename` räumt er auch `.eingang/<id>` (Befund #9 B2): die Datei gehört dann diesem `PUT`, bis der
/// Übergang `laeuft` → `fertig` gelingt.
struct Raeumer {
    gespraeche: Arc<Gespraeche>,
    id: Anhangid,
    generation: u64,
    umbenannt: bool,
    erledigt: bool,
}

impl Raeumer {
    /// Abbruch: Teildatei weg, `laeuft` → `angemeldet` (wenn der Eintrag noch diesem `PUT` gehört).
    fn abbrechen(mut self) {
        self.erledigt = true;
        let _ = self.gespraeche.anhangablage().teil_loeschen(&self.id);
        self.gespraeche
            .hochladen_abbrechen(&self.id, self.generation);
    }

    /// Nicht tilgbar: Teildatei weg, Eintrag verworfen.
    fn verwerfen(mut self) {
        self.erledigt = true;
        self.gespraeche
            .hochladen_verwerfen(&self.id, self.generation);
    }

    fn erledigt(mut self) {
        self.erledigt = true;
    }
}

impl Drop for Raeumer {
    fn drop(&mut self) {
        if self.erledigt {
            return;
        }
        let gespraeche = Arc::clone(&self.gespraeche);
        let id = self.id.clone();
        let generation = self.generation;
        let umbenannt = self.umbenannt;
        let raeumen = move || {
            let ablage = gespraeche.anhangablage();
            let _ = if umbenannt {
                ablage.eingang_loeschen(&id)
            } else {
                ablage.teil_loeschen(&id)
            };
            gespraeche.hochladen_abbrechen(&id, generation);
        };
        match tokio::runtime::Handle::try_current() {
            Ok(laufzeit) => {
                laufzeit.spawn_blocking(raeumen);
            }
            Err(_) => raeumen(),
        }
    }
}

/// Liest die Stücke mit Gegendruck, schreibt sie unter der Hochladebank, achtet auf Fristen, `DELETE` und das
/// Lebensende, und schließt ab.
pub async fn uebertragen<S, E>(
    gespraeche: Arc<Gespraeche>,
    bank: Arc<Semaphore>,
    mut platz: Platz,
    mut strom: S,
) -> Ergebnis
where
    S: Stream<Item = Result<Bytes, E>> + Unpin,
{
    let raeumer = Raeumer {
        gespraeche: Arc::clone(&gespraeche),
        id: platz.id.clone(),
        generation: platz.generation,
        umbenannt: false,
        erledigt: false,
    };
    let beginn = tokio::time::Instant::now();
    let bis_lebensende = (platz.lebensende - platz.beginn)
        .to_std()
        .unwrap_or(Duration::ZERO);
    let mut fristen = Fristen::neu(platz.bytes, gesamtfrist(platz.bytes, bis_lebensende));
    let mut teil = match gespraeche.anhangablage().teil_anlegen(&platz.id) {
        Ok(teil) => Some(teil),
        Err(fehler) => {
            raeumer.abbrechen();
            return Ergebnis::Fehler(fehler.to_string());
        }
    };
    let mut puffer: Vec<u8> = Vec::with_capacity(STUECK);
    let mut erkenner = Some(Erkenner::neu());
    let mut gelesen: u64 = 0;
    loop {
        let (frist, art) = fristen.naechste();
        tokio::select! {
            biased;
            _ = platz.abbruch.changed() => {
                let grund = *platz.abbruch.borrow();
                raeumer.abbrechen();
                return match grund {
                    Some(Abbruchgrund::Lebensende) => Ergebnis::Lebensende,
                    _ => Ergebnis::Verloren(Verloren::Unbekannt),
                };
            }
            () = tokio::time::sleep_until(beginn + frist) => {
                raeumer.abbrechen();
                return Ergebnis::Frist(art);
            }
            naechstes = strom.next() => {
                let stueck = match naechstes {
                    Some(Ok(stueck)) => stueck,
                    Some(Err(_)) | None if gelesen < platz.bytes => {
                        raeumer.abbrechen();
                        return Ergebnis::Abgebrochen;
                    }
                    _ => Bytes::new(),
                };
                gelesen = gelesen.saturating_add(stueck.len() as u64);
                fristen.gelesen(stueck.len() as u64, beginn.elapsed());
                puffer.extend_from_slice(&stueck);
                let am_ende = gelesen >= platz.bytes;
                if puffer.len() >= STUECK || (am_ende && !puffer.is_empty()) {
                    let geschrieben =
                        schreiben(&bank, teil.take(), erkenner.take(), std::mem::take(&mut puffer)).await;
                    match geschrieben {
                        Ok((zurueck, erkannt, leer)) => {
                            teil = Some(zurueck);
                            erkenner = Some(erkannt);
                            puffer = leer;
                            platz.geschrieben.store(gelesen.min(platz.bytes), Ordering::Relaxed);
                        }
                        Err(fehler) => {
                            raeumer.abbrechen();
                            return schreibfehler(&gespraeche, &fehler);
                        }
                    }
                }
                if am_ende {
                    break;
                }
            }
        }
    }
    let getilgt = match tilgen_unter_bank(&bank, teil).await {
        Ok(getilgt) => getilgt,
        Err(Abbruch::Metadaten(grund)) => {
            raeumer.verwerfen();
            return Ergebnis::Metadaten(grund);
        }
        Err(Abbruch::Platte(fehler)) => {
            raeumer.abbrechen();
            return schreibfehler(&gespraeche, &fehler);
        }
    };
    let art = erkenner.map_or_else(Art::datei, |erkenner| erkenner.ergebnis());
    abschliessen(gespraeche, raeumer, platz, art, getilgt).await
}

/// Nach dem letzten Byte, vor `rename`: der Tilger im Blocking-Thread unter einer Erlaubnis der Hochladebank — keine
/// Sperre, nicht die Werkbank, außerhalb der Gesamtfrist (ADR-0047 Ergänzung F4 Regel 1).
async fn tilgen_unter_bank(
    bank: &Arc<Semaphore>,
    teil: Option<Box<dyn Teildatei>>,
) -> Result<Option<Getilgt>, Abbruch> {
    let Some(teil) = teil else {
        return Err(Abbruch::Platte(std::io::Error::other("Teildatei fehlt")));
    };
    let erlaubnis = Arc::clone(bank)
        .acquire_owned()
        .await
        .map_err(|_| Abbruch::Platte(std::io::Error::other("Hochladebank geschlossen")))?;
    tokio::task::spawn_blocking(move || {
        let _erlaubnis = erlaubnis;
        tilgen(teil.as_ref())
    })
    .await
    .map_err(|fehler| Abbruch::Platte(std::io::Error::other(fehler.to_string())))?
}

/// `rename` ohne Überschreiben und der Übergang `laeuft` → `fertig`; wer verliert, räumt seine Datei. Der Räumer
/// geht mit in den Blocking-Thread des `rename` und kommt mit dem Ergebnis zurück: wird der Handler dabei fallen
/// gelassen, fällt er erst nach dem `rename` und räumt dann `.eingang/<id>`.
async fn abschliessen(
    gespraeche: Arc<Gespraeche>,
    raeumer: Raeumer,
    platz: Platz,
    art: Art,
    getilgt: Option<Getilgt>,
) -> Ergebnis {
    let umbenannt = {
        let gespraeche = Arc::clone(&gespraeche);
        let id = platz.id.clone();
        let mut raeumer = raeumer;
        tokio::task::spawn_blocking(move || {
            let umbenannt = gespraeche.anhangablage().teil_abschliessen(&id);
            raeumer.umbenannt = umbenannt.is_ok();
            (raeumer, umbenannt)
        })
        .await
    };
    let raeumer = match umbenannt {
        Ok((raeumer, Ok(()))) => raeumer,
        Ok((raeumer, Err(fehler))) if fehler.kind() == std::io::ErrorKind::NotFound => {
            raeumer.abbrechen();
            return Ergebnis::Verloren(Verloren::Unbekannt);
        }
        Ok((raeumer, Err(fehler))) => {
            raeumer.abbrechen();
            return Ergebnis::Fehler(fehler.to_string());
        }
        Err(fehler) => return Ergebnis::Fehler(fehler.to_string()),
    };
    match gespraeche.hochladen_abschliessen(&platz.id, platz.generation, art) {
        Ok(gueltig_bis) => {
            raeumer.erledigt();
            Ergebnis::Fertig {
                id: platz.id,
                name: platz.name,
                bytes: platz.bytes,
                art,
                gueltig_bis,
                getilgt,
            }
        }
        Err(verloren) => {
            raeumer.erledigt();
            let _ = gespraeche.anhangablage().eingang_loeschen(&platz.id);
            Ergebnis::Verloren(verloren)
        }
    }
}

/// Ein Stück in einem Blocking-Thread unter einer Erlaubnis der Hochladebank — nie die Werkbank, nie eine Sperre;
/// dort sieht auch der Erkenner das Stück (Art und Maße aus dem Inhalt, A00018).
async fn schreiben(
    bank: &Arc<Semaphore>,
    teil: Option<Box<dyn Teildatei>>,
    erkenner: Option<Erkenner>,
    puffer: Vec<u8>,
) -> std::io::Result<(Box<dyn Teildatei>, Erkenner, Vec<u8>)> {
    let (Some(mut teil), Some(mut erkenner)) = (teil, erkenner) else {
        return Err(std::io::Error::other("Teildatei fehlt"));
    };
    let erlaubnis = Arc::clone(bank)
        .acquire_owned()
        .await
        .map_err(|_| std::io::Error::other("Hochladebank geschlossen"))?;
    tokio::task::spawn_blocking(move || {
        let _erlaubnis = erlaubnis;
        teil.anhaengen(&puffer)?;
        erkenner.stueck(&puffer);
        let mut leer = puffer;
        leer.clear();
        Ok((teil, erkenner, leer))
    })
    .await
    .map_err(|fehler| std::io::Error::other(fehler.to_string()))?
}

/// `ENOSPC` → 507 `platte` mit dem freien Platz, sonst 500 (Ergänzung „Karte #9 Gate“ Regel 8).
fn schreibfehler(gespraeche: &Gespraeche, fehler: &std::io::Error) -> Ergebnis {
    if fehler.raw_os_error() == Some(rustix::io::Errno::NOSPC.raw_os_error())
        || fehler.kind() == std::io::ErrorKind::StorageFull
    {
        Ergebnis::Platte {
            frei: gespraeche.platte_frei(),
            grund: fehler.to_string(),
        }
    } else {
        Ergebnis::Fehler(fehler.to_string())
    }
}

/// `application/octet-stream`, Groß-/Kleinschreibung egal, Parameter erlaubt.
fn roh_angekuendigt(kopf: &HeaderMap) -> bool {
    let mut werte = kopf.get_all(CONTENT_TYPE).iter();
    let (Some(wert), None) = (werte.next(), werte.next()) else {
        return false;
    };
    wert.to_str().is_ok_and(|wert| {
        wert.split(';')
            .next()
            .unwrap_or_default()
            .trim()
            .eq_ignore_ascii_case("application/octet-stream")
    })
}

/// Genau eine `Content-Length` aus Ziffern und kein `Transfer-Encoding` — sonst keine Länge (411).
fn laenge_angekuendigt(kopf: &HeaderMap) -> Option<u64> {
    if kopf.contains_key(TRANSFER_ENCODING) {
        return None;
    }
    let mut werte = kopf.get_all(CONTENT_LENGTH).iter();
    let (Some(wert), None) = (werte.next(), werte.next()) else {
        return None;
    };
    let text = wert.to_str().ok()?;
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

/// Abweisung vor dem ersten gelesenen Byte — immer mit `Connection: close`.
fn abgewiesen(fehler: Beginnfehler, wer: &str) -> Response {
    let (status, regel, koerper, wiederholen): (StatusCode, &'static str, Value, Option<&str>) =
        match fehler {
            Beginnfehler::Unbekannt => (
                StatusCode::NOT_FOUND,
                "anhang_unbekannt",
                json!({ "fehler": "Diese Anmeldung gibt es nicht — neu anmelden.", "grund": "unbekannt" }),
                None,
            ),
            Beginnfehler::Abgelaufen => (
                StatusCode::NOT_FOUND,
                "anhang_abgelaufen",
                json!({ "fehler": "Diese Anmeldung ist abgelaufen — neu anmelden.", "grund": "abgelaufen" }),
                None,
            ),
            Beginnfehler::Laeuft => (
                StatusCode::CONFLICT,
                "laeuft",
                json!({ "fehler": "Für diese Anmeldung läuft schon ein Hochladen.", "regel": "laeuft" }),
                None,
            ),
            Beginnfehler::Fertig => (
                StatusCode::CONFLICT,
                "fertig",
                json!({ "fehler": "Diese Datei ist schon hochgeladen — jetzt senden.", "regel": "fertig" }),
                None,
            ),
            Beginnfehler::Grenze(grenze) => (
                StatusCode::TOO_MANY_REQUESTS,
                grenze.grenze(),
                json!({
                    "fehler": super::anmelden::grenze_text(grenze),
                    "grenze": grenze.grenze(),
                    "hoechstens": grenze.hoechstens(),
                }),
                Some("5"),
            ),
            Beginnfehler::Typ => (
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "kein_octet_stream",
                json!({ "fehler": "Der Körper muss die Datei selbst sein — mit Content-Type: application/octet-stream schicken." }),
                None,
            ),
            Beginnfehler::OhneLaenge => (
                StatusCode::LENGTH_REQUIRED,
                "ohne_laenge",
                json!({ "fehler": "Content-Length fehlt — die angemeldete Länge angeben, ohne chunked." }),
                None,
            ),
            Beginnfehler::Laenge { erwartet } => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "laenge",
                json!({
                    "fehler": format!("Content-Length muss die angemeldete Länge sein: {erwartet} Bytes."),
                    "regel": "laenge",
                    "erwartet": erwartet,
                }),
                None,
            ),
        };
    let mut antwort = (status, [(CONNECTION, "close")], Json(koerper)).into_response();
    if let Some(sekunden) = wiederholen
        && let Ok(wert) = sekunden.parse()
    {
        antwort.headers_mut().insert(RETRY_AFTER, wert);
    }
    mit_regel(antwort, regel, Some(wer))
}

/// Quelle der Episode „Metadaten nicht tilgbar“ ist die Herkunft; der Grund (welche Prüfung scheiterte) gehört
/// dem einzelnen Auftreten und steht nur in der Beginn-Zeile (ADR-0042 Ergänzung Karte #66).
fn metadaten_auftreten(herkunft: std::net::IpAddr, grund: &str) -> Auftreten {
    Auftreten {
        stufe: Stufe::Warning,
        bereich: Bereich::Anhang,
        text: crate::protokoll::episodentexte::METADATEN_NICHT_TILGBAR,
        quelle: herkunft.to_string(),
        kontext: Kontext::neu()
            .mit(Schluessel::Herkunft, herkunft)
            .mit_nur_beginn(Schluessel::Grund, grund),
        art: None,
        verworfen: None,
        bytes: None,
        anzeige: None,
    }
}

fn beantworten(ergebnis: Ergebnis, wer: &str) -> Response {
    match ergebnis {
        Ergebnis::Fertig {
            id,
            name,
            bytes,
            art,
            gueltig_bis,
            ..
        } => (
            StatusCode::OK,
            [(CONNECTION, "close")],
            Json(json!({
                "anhang": {
                    "id": id.als_text(),
                    "name": name,
                    "bytes": bytes,
                    "art": art.typ,
                    "breite": art.breite,
                    "hoehe": art.hoehe,
                    "gueltig_bis": zeit_als_text(&gueltig_bis),
                }
            })),
        )
            .into_response(),
        Ergebnis::Metadaten(grund) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({
                "fehler": "Nicht hochgeladen: die Datei ist beschädigt — Ortsangaben ließen sich nicht sicher entfernen.",
                "regel": "metadaten",
                "grund": grund,
            })),
        )
            .into_response(),
        Ergebnis::Frist(art) => zu_langsam(match art {
            Fristart::Stueckfrist => format!(
                "Der Körper kam nicht vollständig an: {} s lang keine weiteren 64 KiB — Anfrage abgebrochen; über eine schnellere Verbindung erneut senden.",
                STUECKFRIST.as_secs()
            ),
            Fristart::Gesamtfrist => "Der Körper kam nicht rechtzeitig vollständig an (mindestens 1 MiB/s nach 60 s Anlauf) — Anfrage abgebrochen; über eine schnellere Verbindung erneut senden.".to_string(),
        }),
        Ergebnis::Lebensende => zu_langsam(
            "Der Körper kam nicht vor dem Ende der Anmeldung vollständig an — Anfrage abgebrochen; neu anmelden und über eine schnellere Verbindung senden.".to_string(),
        ),
        Ergebnis::Verloren(verloren) => abgewiesen(
            match verloren {
                Verloren::Unbekannt => Beginnfehler::Unbekannt,
                Verloren::Abgelaufen => Beginnfehler::Abgelaufen,
            },
            wer,
        ),
        Ergebnis::Platte { frei, .. } => (
            StatusCode::INSUFFICIENT_STORAGE,
            [(CONNECTION, "close")],
            Json(json!({
                "fehler": format!(
                    "Nicht hochgeladen: auf der Platte ist kein Platz mehr — frei sind {}.",
                    super::super::senden::groesse_in_worten(frei)
                ),
                "regel": "platte",
                "frei": frei,
            })),
        )
            .into_response(),
        Ergebnis::Fehler(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            [(CONNECTION, "close")],
            Json(json!({ "fehler": "Nicht hochgeladen — der Server konnte die Datei nicht schreiben." })),
        )
            .into_response(),
        Ergebnis::Abgebrochen => (
            StatusCode::BAD_REQUEST,
            [(CONNECTION, "close")],
            Json(json!({ "fehler": "Der Körper kam nicht vollständig an — Anfrage abgebrochen." })),
        )
            .into_response(),
    }
}

fn zu_langsam(fehler: String) -> Response {
    (
        StatusCode::REQUEST_TIMEOUT,
        [(CONNECTION, "close")],
        Json(json!({ "fehler": fehler, "regel": "zu_langsam" })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gespraeche::anhangspeicher::Anhangspeicher;
    use crate::gespraeche::{Kennung, Speicherablage};
    use crate::uhr::GestellteUhr;
    use crate::zufall::FesteFolge;
    use futures_util::stream;
    use std::convert::Infallible;
    use std::sync::Barrier;

    const MIB: u64 = 1_048_576;

    /// R00038 B0591 (Bestandsaufnahme): zwei Hochladungen derselben Herkunft scheitern an verschiedenen Prüfungen →
    /// Beginn mit `grund=` der ersten, Ende nur `herkunft=`.
    #[test]
    fn die_ende_zeile_der_metadaten_nennt_nur_die_herkunft() {
        use crate::protokoll::Stufe;
        use crate::protokoll::testhilfe::{im_speicher, zeit};
        let (protokoll, speicher, uhr, _) =
            im_speicher("2026-10-28T12:00:00+01:00", Stufe::Information);
        let herkunft = "10.0.0.9".parse().expect("IP");
        for grund in ["MPF-Index unlesbar", "über 100.000 Blöcke"] {
            protokoll.episode(metadaten_auftreten(herkunft, grund));
        }
        uhr.stellen(zeit("2026-10-28T12:01:00+01:00"));
        protokoll.episoden_schliessen();

        let zeilen = speicher.zeilen();
        assert_eq!(zeilen.len(), 2, "{zeilen:#?}");
        assert!(zeilen[0].contains("grund="), "{zeilen:#?}");
        assert!(zeilen[1].contains("Episode endet: 2 in "), "{zeilen:#?}");
        assert!(zeilen[1].ends_with("| herkunft=10.0.0.9"), "{zeilen:#?}");
    }

    struct Aufbau {
        gespraeche: Arc<Gespraeche>,
        ablage: Arc<Anhangspeicher>,
        uhr: Arc<GestellteUhr>,
        bank: Arc<Semaphore>,
    }

    fn zeit(text: &str) -> crate::uhr::Zeit {
        chrono::DateTime::parse_from_rfc3339(text).expect("Zeit")
    }

    fn aufbauen(q: Option<u64>) -> Aufbau {
        let ablage = Arc::new(Anhangspeicher::neu());
        let uhr = Arc::new(GestellteUhr::neu(zeit("2026-10-29T12:00:00.000+01:00")));
        let gespraeche = Gespraeche::laden(Arc::new(Speicherablage::neu()), uhr.clone())
            .expect("geladen")
            .mit_anhangquote(q)
            .mit_anhangablage(ablage.clone())
            .mit_zufall(Arc::new(FesteFolge::neu(1..=100)));
        Aufbau {
            gespraeche: Arc::new(gespraeche),
            ablage,
            uhr,
            bank: Arc::new(Semaphore::new(HOCHLADEBANK)),
        }
    }

    fn beta() -> Kennung {
        Kennung::bilden("beta@127.0.0.1").expect("Kennung")
    }

    fn beginnen(aufbau: &Aufbau, bytes: u64) -> Platz {
        let anmeldung = aufbau
            .gespraeche
            .anhang_anmelden(
                &beta(),
                beta().herkunft().expect("Herkunft"),
                "bericht.pdf",
                bytes,
            )
            .expect("angemeldet");
        aufbau
            .gespraeche
            .hochladen_beginnen(
                &beta(),
                &anmeldung.id,
                Kopf {
                    typ_passt: true,
                    laenge: Some(bytes),
                },
            )
            .expect("läuft")
    }

    fn stuecke(teile: Vec<Vec<u8>>) -> impl Stream<Item = Result<Bytes, Infallible>> + Unpin {
        stream::iter(teile.into_iter().map(|teil| Ok(Bytes::from(teil))))
    }

    /// Hauptweg: zwei Stücke → fertig; im Eingang liegt `<id>` mit allen Bytes, keine `.teil`; die Reservierung
    /// bleibt bis zum Senden.
    #[tokio::test]
    async fn alle_stuecke_ergeben_die_fertige_datei_im_eingang() {
        let aufbau = aufbauen(Some(10 * MIB));
        let platz = beginnen(&aufbau, 10);
        let id = platz.id.clone();

        let ergebnis = uebertragen(
            Arc::clone(&aufbau.gespraeche),
            Arc::clone(&aufbau.bank),
            platz,
            stuecke(vec![b"hallo".to_vec(), b"welt!".to_vec()]),
        )
        .await;

        let Ergebnis::Fertig {
            gueltig_bis,
            art,
            bytes,
            ..
        } = ergebnis
        else {
            panic!("fertig erwartet: {ergebnis:?}");
        };
        assert_eq!(bytes, 10);
        assert_eq!(
            art.typ,
            crate::gespraeche::anhangpruefung::erkenner::TEXT,
            "der Erkenner sah die Stücke"
        );
        assert_eq!(gueltig_bis, zeit("2026-10-29T12:15:00.000+01:00"));
        let eingang = aufbau.ablage.eingang();
        assert_eq!(
            eingang.get(id.als_text()).map(Vec::as_slice),
            Some(&b"hallowelt!"[..])
        );
        assert_eq!(eingang.len(), 1, "keine .teil");
        assert_eq!(aufbau.gespraeche.anhangkennzahlen().reserviert, 65_536);
    }

    /// „PUT prüft die Quote nicht mehr“ (F1′ Regel 3): die ganze Anhang-Quote ist beansprucht, das Hochladen in
    /// einem Zug gelingt trotzdem.
    #[tokio::test]
    async fn eine_angenommene_hochladung_endet_nie_wegen_der_quote() {
        let aufbau = aufbauen(Some(MIB));
        let platz = beginnen(&aufbau, MIB);
        let teile = (0..4).map(|_| vec![7u8; 256 * 1024]).collect();

        let ergebnis = uebertragen(
            Arc::clone(&aufbau.gespraeche),
            Arc::clone(&aufbau.bank),
            platz,
            stuecke(teile),
        )
        .await;

        assert!(matches!(ergebnis, Ergebnis::Fertig { .. }), "{ergebnis:?}");
        assert_eq!(aufbau.gespraeche.anhangkennzahlen().reserviert, MIB);
    }

    /// M24: `ENOSPC` beim zweiten Stück → 507 `platte`, keine `.teil`, die Anmeldung ist wieder `angemeldet`
    /// (ein neuer `PUT` beginnt); `EIO` → 500.
    #[tokio::test]
    async fn ein_schreibfehler_bricht_ab_und_raeumt_die_teildatei() {
        for (fehler, platte) in [
            (
                std::io::Error::from_raw_os_error(rustix::io::Errno::NOSPC.raw_os_error()),
                true,
            ),
            (
                std::io::Error::from_raw_os_error(rustix::io::Errno::IO.raw_os_error()),
                false,
            ),
        ] {
            let aufbau = aufbauen(Some(10 * MIB));
            aufbau.ablage.stueck_scheitert(2, fehler);
            let platz = beginnen(&aufbau, 3 * 256 * 1024);
            let id = platz.id.clone();
            let teile = (0..3).map(|_| vec![1u8; 256 * 1024]).collect();

            let ergebnis = uebertragen(
                Arc::clone(&aufbau.gespraeche),
                Arc::clone(&aufbau.bank),
                platz,
                stuecke(teile),
            )
            .await;

            if platte {
                assert!(matches!(ergebnis, Ergebnis::Platte { .. }), "{ergebnis:?}");
            } else {
                assert!(matches!(ergebnis, Ergebnis::Fehler(_)), "{ergebnis:?}");
            }
            assert!(aufbau.ablage.eingang().is_empty(), "keine .teil");
            let wieder = aufbau.gespraeche.hochladen_beginnen(
                &beta(),
                &id,
                Kopf {
                    typ_passt: true,
                    laenge: Some(3 * 256 * 1024),
                },
            );
            assert!(wieder.is_ok(), "wieder angemeldet");
        }
    }

    /// M39: liegt `.eingang/<id>` beim Abschluss schon da, bleibt es byte-gleich; 500, keine `.teil`.
    #[tokio::test]
    async fn ein_vorhandenes_ziel_wird_nie_ueberschrieben() {
        let aufbau = aufbauen(Some(10 * MIB));
        let platz = beginnen(&aufbau, 4);
        aufbau
            .ablage
            .eingang_setzen(platz.id.als_text(), b"untergeschoben");
        let id = platz.id.clone();

        let ergebnis = uebertragen(
            Arc::clone(&aufbau.gespraeche),
            Arc::clone(&aufbau.bank),
            platz,
            stuecke(vec![b"neu!".to_vec()]),
        )
        .await;

        assert!(matches!(ergebnis, Ergebnis::Fehler(_)), "{ergebnis:?}");
        let eingang = aufbau.ablage.eingang();
        assert_eq!(
            eingang.get(id.als_text()).map(Vec::as_slice),
            Some(&b"untergeschoben"[..])
        );
        assert_eq!(eingang.len(), 1);
    }

    /// M23: `DELETE` zwischen letztem Stück und `rename` → der `PUT` räumt seine `.teil`, 404; nichts bleibt.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn ein_delete_vor_dem_rename_gewinnt() {
        let aufbau = aufbauen(Some(10 * MIB));
        let platz = beginnen(&aufbau, 4);
        let id = platz.id.clone();
        let schranke = Arc::new(Barrier::new(2));
        aufbau.ablage.vor_abschluss_warten(Arc::clone(&schranke));
        let uebertragung = tokio::spawn(uebertragen(
            Arc::clone(&aufbau.gespraeche),
            Arc::clone(&aufbau.bank),
            platz,
            stuecke(vec![b"abcd".to_vec()]),
        ));
        let gespraeche = Arc::clone(&aufbau.gespraeche);
        let verworfen = tokio::task::spawn_blocking(move || {
            schranke.wait();
            let verworfen = gespraeche.anhang_verwerfen(&beta(), &id);
            schranke.wait();
            verworfen
        })
        .await
        .expect("verworfen");

        let ergebnis = uebertragung.await.expect("Übertragung");

        assert!(verworfen.is_some_and(|verworfen| verworfen.laufend));
        assert!(
            matches!(ergebnis, Ergebnis::Verloren(Verloren::Unbekannt)),
            "{ergebnis:?}"
        );
        assert!(aufbau.ablage.eingang().is_empty());
        assert_eq!(aufbau.gespraeche.anhangkennzahlen().reserviert, 0);
    }

    /// Lebensende während des Hochladens: der laufende Platz erhält das Signal → 408 (Lebensende), keine `.teil`;
    /// danach `abgelaufen` (Mutante „Lebensende im `PUT` fehlt“).
    #[tokio::test]
    async fn am_lebensende_endet_der_laufende_put_mit_408() {
        let aufbau = aufbauen(Some(10 * MIB));
        let platz = beginnen(&aufbau, 1_000);
        let id = platz.id.clone();
        let strom = stream::iter(vec![Ok::<Bytes, Infallible>(Bytes::from(vec![0u8; 10]))])
            .chain(stream::pending());
        let uebertragung = tokio::spawn(uebertragen(
            Arc::clone(&aufbau.gespraeche),
            Arc::clone(&aufbau.bank),
            platz,
            strom,
        ));
        tokio::task::yield_now().await;
        aufbau.uhr.stellen(zeit("2026-10-29T12:01:00.003+01:00"));
        let verfallen = aufbau.gespraeche.anhaenge_ablaufen();

        let ergebnis = uebertragung.await.expect("Übertragung");

        assert_eq!(verfallen.len(), 1);
        assert!(matches!(ergebnis, Ergebnis::Lebensende), "{ergebnis:?}");
        assert!(aufbau.ablage.eingang().is_empty());
        let danach = aufbau.gespraeche.hochladen_beginnen(
            &beta(),
            &id,
            Kopf {
                typ_passt: true,
                laenge: Some(1_000),
            },
        );
        assert_eq!(danach.map(|_| ()), Err(Beginnfehler::Abgelaufen));
    }

    /// Stückfrist mit gestellter Zeit: 64 KiB, dann Stille → 408 Stückfrist nach 10 s, keine `.teil`, wieder
    /// `angemeldet`.
    #[tokio::test(start_paused = true)]
    async fn stille_nach_64_kib_endet_mit_der_stueckfrist() {
        let aufbau = aufbauen(Some(10 * MIB));
        let platz = beginnen(&aufbau, MIB);
        let id = platz.id.clone();
        let strom = stream::iter(vec![Ok::<Bytes, Infallible>(Bytes::from(vec![
            0u8;
            65_536
        ]))])
        .chain(stream::pending());
        let beginn = tokio::time::Instant::now();

        let ergebnis = uebertragen(
            Arc::clone(&aufbau.gespraeche),
            Arc::clone(&aufbau.bank),
            platz,
            strom,
        )
        .await;

        assert!(
            matches!(ergebnis, Ergebnis::Frist(Fristart::Stueckfrist)),
            "{ergebnis:?}"
        );
        assert_eq!(beginn.elapsed(), Duration::from_secs(10));
        assert!(aufbau.ablage.eingang().is_empty());
        assert!(
            aufbau
                .gespraeche
                .hochladen_beginnen(
                    &beta(),
                    &id,
                    Kopf {
                        typ_passt: true,
                        laenge: Some(MIB)
                    }
                )
                .is_ok()
        );
    }

    /// Client weg (Lesefehler, zu wenig Bytes): Abbruch, keine `.teil`, Reservierung unverändert.
    #[tokio::test]
    async fn ein_abbruch_des_clients_raeumt_und_laesst_die_reservierung() {
        let aufbau = aufbauen(Some(10 * MIB));
        let platz = beginnen(&aufbau, 100);
        let strom = stream::iter(vec![
            Ok(Bytes::from(vec![0u8; 40])),
            Err(std::io::Error::other("weg")),
        ]);

        let ergebnis = uebertragen(
            Arc::clone(&aufbau.gespraeche),
            Arc::clone(&aufbau.bank),
            platz,
            strom,
        )
        .await;

        assert!(matches!(ergebnis, Ergebnis::Abgebrochen), "{ergebnis:?}");
        assert!(aufbau.ablage.eingang().is_empty());
        assert_eq!(aufbau.gespraeche.anhangkennzahlen().reserviert, 65_536);

        let platz = beginnen(&aufbau, 100);
        let kurz = stream::iter(vec![Ok::<Bytes, Infallible>(Bytes::from(vec![0u8; 40]))]);
        let ergebnis = uebertragen(
            Arc::clone(&aufbau.gespraeche),
            Arc::clone(&aufbau.bank),
            platz,
            kurz,
        )
        .await;
        assert!(
            matches!(ergebnis, Ergebnis::Abgebrochen),
            "weniger Bytes: {ergebnis:?}"
        );
    }

    /// Wird der Handler fallen gelassen (Client weg, Ende der Instanz), räumt der Räumer: keine `.teil`, die
    /// Anmeldung gilt weiter.
    #[tokio::test]
    async fn ein_fallen_gelassener_put_raeumt_seine_teildatei() {
        let aufbau = aufbauen(Some(10 * MIB));
        let platz = beginnen(&aufbau, 100);
        let id = platz.id.clone();
        let strom = stream::iter(vec![Ok::<Bytes, Infallible>(Bytes::from(vec![0u8; 40]))])
            .chain(stream::pending());
        let uebertragung = tokio::spawn(uebertragen(
            Arc::clone(&aufbau.gespraeche),
            Arc::clone(&aufbau.bank),
            platz,
            strom,
        ));
        tokio::task::yield_now().await;
        uebertragung.abort();
        let _ = uebertragung.await;
        for _ in 0..100 {
            if aufbau.ablage.eingang().is_empty() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        assert!(aufbau.ablage.eingang().is_empty());
        let wieder = aufbau.gespraeche.hochladen_beginnen(
            &beta(),
            &id,
            Kopf {
                typ_passt: true,
                laenge: Some(100),
            },
        );
        assert!(wieder.is_ok());
    }

    /// Befund #9 B2 (L1): fallen gelassen nach dem `rename`, vor dem Übergang `laeuft` → `fertig` (Client schließt
    /// nach dem letzten Byte) → weder `<id>` noch `<id>.teil` bleiben, die Anmeldung gilt weiter, ein neuer `PUT`
    /// wird fertig. Die Laufzeit hat einen Thread: solange der Test sie hält, wird die Übertragung nicht gepollt.
    #[tokio::test(flavor = "current_thread")]
    async fn fallen_gelassen_nach_dem_rename_bleibt_nichts_im_eingang() {
        let aufbau = aufbauen(Some(10 * MIB));
        let platz = beginnen(&aufbau, 4);
        let id = platz.id.clone();
        let schranke = Arc::new(Barrier::new(2));
        aufbau.ablage.vor_abschluss_warten(Arc::clone(&schranke));
        let uebertragung = tokio::spawn(uebertragen(
            Arc::clone(&aufbau.gespraeche),
            Arc::clone(&aufbau.bank),
            platz,
            stuecke(vec![b"abcd".to_vec()]),
        ));
        let vor_dem_rename = Arc::clone(&schranke);
        tokio::task::spawn_blocking(move || vor_dem_rename.wait())
            .await
            .expect("vor dem rename");
        schranke.wait();
        for _ in 0..400 {
            if aufbau.ablage.eingang().contains_key(id.als_text()) {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(
            aufbau.ablage.eingang().contains_key(id.als_text()),
            "rename gelaufen"
        );

        uebertragung.abort();
        assert!(
            uebertragung
                .await
                .is_err_and(|fehler| fehler.is_cancelled())
        );
        for _ in 0..400 {
            if aufbau.ablage.eingang().is_empty() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }

        assert_eq!(
            aufbau.ablage.eingang().keys().cloned().collect::<Vec<_>>(),
            Vec::<String>::new()
        );
        let wieder = aufbau
            .gespraeche
            .hochladen_beginnen(
                &beta(),
                &id,
                Kopf {
                    typ_passt: true,
                    laenge: Some(4),
                },
            )
            .expect("wieder angemeldet");
        let ergebnis = uebertragen(
            Arc::clone(&aufbau.gespraeche),
            Arc::clone(&aufbau.bank),
            wieder,
            stuecke(vec![b"efgh".to_vec()]),
        )
        .await;
        assert!(matches!(ergebnis, Ergebnis::Fertig { .. }), "{ergebnis:?}");
        assert_eq!(
            aufbau
                .ablage
                .eingang()
                .get(id.als_text())
                .map(Vec::as_slice),
            Some(&b"efgh"[..])
        );
    }

    #[test]
    fn content_type_und_content_length_werden_genau_gelesen() {
        let kopf = |paare: &[(&'static str, &'static str)]| {
            let mut kopf = HeaderMap::new();
            for (name, wert) in paare {
                kopf.append(*name, wert.parse().expect("Wert"));
            }
            kopf
        };
        assert!(roh_angekuendigt(&kopf(&[(
            "content-type",
            "application/octet-stream"
        )])));
        assert!(roh_angekuendigt(&kopf(&[(
            "content-type",
            "Application/Octet-Stream; x=1"
        )])));
        assert!(!roh_angekuendigt(&kopf(&[("content-type", "text/plain")])));
        assert!(!roh_angekuendigt(&kopf(&[])));
        assert_eq!(
            laenge_angekuendigt(&kopf(&[("content-length", "307200")])),
            Some(307_200)
        );
        for falsch in ["+5", "-5", " 5", "5 ", "", "x"] {
            assert_eq!(
                laenge_angekuendigt(&kopf(&[("content-length", falsch)])),
                None,
                "{falsch:?}"
            );
        }
        assert_eq!(
            laenge_angekuendigt(&kopf(&[("content-length", "5"), ("content-length", "5")])),
            None
        );
        assert_eq!(
            laenge_angekuendigt(&kopf(&[
                ("content-length", "5"),
                ("transfer-encoding", "chunked")
            ])),
            None
        );
    }
}
