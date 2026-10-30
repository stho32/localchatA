//! Gespräche: einziger Schreiber des Ordners (A00006) mit Logformat, Logablage und Verlaufsindex.

pub mod abfrage;
mod anhaenge;
pub mod anhangablage;
pub mod anhangeingang;
pub mod anhangid;
pub mod anhangpfade;
pub mod anhangpruefung;
pub mod anhangquote;
#[cfg(test)]
pub mod anhangspeicher;
pub mod anhangverzeichnis;
pub mod einstellungen;
mod frist;
pub mod fristeintrag;
pub mod hochladefristen;
mod kennung;
pub mod loeschen;
mod logablage;
pub mod logformat;
mod pflege;
pub mod platte;
pub mod quote;
mod speicherablage;
pub mod textregel;
mod verlaufsindex;
mod verzeichnis;
pub mod wiederholung;

pub use crate::anwesenheit::Art;
pub use anhaenge::{
    Anhangkennzahlen, Anhangschwelle, AnhangschwelleMelden, Anmeldefehler, STANDARD_ANHANGQUOTE,
};
pub use frist::{FRIST_MAX, FRIST_MIN, Frist, STANDARD_FRIST_TAGE, frist_in_spanne};
pub use kennung::Kennung;
pub use logablage::{Angehaengt, Dateiablage, Gespraech, Logablage};
pub use pflege::{Entfaellt, FristGesetzt, Fristfehler, Pflegeanlass, Pflegebericht};
pub use speicherablage::Speicherablage;
pub use verzeichnis::{
    Anlegefehler, Channel, ChannelStand, ChannelVeroeffentlichen, ChannelWechsel, GESAMT, Grenzart,
    Grenze, JE_HERKUNFT,
};

use crate::fehler::Fehler;
use crate::name::Channelname;
use crate::uhr::{Uhr, Zeit};
use chrono::NaiveDate;
use std::fmt;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use verlaufsindex::Verlauf;
use verzeichnis::Verzeichnis;
use wiederholung::{Pruefung, Verdraengt, Wiederholung};

/// Eine Nachricht, wie sie im Log steht; die Zeit ist zugleich ihre ID (ADR-0005).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nachricht {
    pub zeit: Zeit,
    pub von: Kennung,
    /// Art des Absenders aus der Metazeile `+ art:`; ohne sie (von Hand, ältere Sätze) keine (ADR-0034 Regel 5).
    pub art: Option<Art>,
    pub text: String,
    /// Anhänge aus den Metazeilen `+ anhang:`, in Sendefolge (ADR-0046 Regel 7).
    pub anhaenge: Vec<logformat::Anhangzeile>,
    /// Wann und wie die Nachricht zuletzt geändert wurde; ungeändert keine (ADR-0048 Regel 4).
    pub geaendert: Option<Geaendert>,
}

/// Die letzte Änderung einer Nachricht: Zeit des Änderungssatzes und Zustand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Geaendert {
    pub zeit: Zeit,
    pub zustand: Aenderungszustand,
}

/// `bearbeitet` (neuer Text) oder `geloescht` (Text und Anhänge leer).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Aenderungszustand {
    Bearbeitet,
    Geloescht,
}

/// Ein Block des Verlaufs als Nachrichten, älteste zuerst (ADR-0025 Regel 3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nachrichtenblock {
    pub nachrichten: Vec<Nachricht>,
    pub aelter: bool,
    pub neuer: bool,
}

/// Warum `senden` nichts angehängt hat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sendefehler {
    /// Das Gespräch ist weder geladen noch angelegt.
    Unbekannt,
    /// Die Tagesdatei (relativ zum Ordner) nahm den Satz nicht an; nichts steht im Log oder Index.
    Ablage { datei: String, grund: String },
    /// Der Wiederholschlüssel gehört zu einem anderen Text oder Gespräch (ADR-0034 Regel 6).
    SchluesselAnders,
    /// Der Satz brächte die Gespräche über die Plattenquote; nichts angehängt (ADR-0044 Regel 2).
    Quote { belegt: u64, hoechstens: u64 },
    /// Ein Anhang lässt sich nicht senden (422 `anhang`, ADR-0046 Regel 5); nichts genommen.
    Anhang {
        id: anhangid::Anhangid,
        grund: anhangeingang::Nehmgrund,
    },
    /// Binden scheiterte (auch: Ziel vorhanden, Symlink im Pfad); zurückgebaut, kein Satz (ADR-0046 Regel 6).
    /// `datei` nur als Präfix; `waisen`: Präfixe der Dateien, deren Zurücklegen scheiterte — sie bleiben als Waise
    /// im Tag.
    Binden {
        tag: NaiveDate,
        datei: String,
        grund: String,
        waisen: Vec<String>,
    },
}

/// Was `senden_einmal` getan hat: neu angehängt, oder dieselbe Nachricht schon gesendet — dann mit ihrer
/// ursprünglichen Zeit, ohne Satz und ohne Ereignis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Gesendet {
    Neu(Nachricht),
    Wiederholt(Nachricht),
}

impl fmt::Display for Sendefehler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Sendefehler::Unbekannt => write!(f, "Gespräch unbekannt"),
            Sendefehler::Ablage { datei, grund } => write!(f, "{datei} nicht geschrieben: {grund}"),
            Sendefehler::SchluesselAnders => {
                write!(f, "Schlüssel gehört zu einer anderen Nachricht")
            }
            Sendefehler::Quote { belegt, hoechstens } => {
                write!(f, "Quote erreicht: {belegt} von {hoechstens} Bytes belegt")
            }
            Sendefehler::Anhang { grund, .. } => write!(f, "Anhang {}", grund.als_text()),
            Sendefehler::Binden { datei, grund, .. } => {
                write!(f, "Anhang {datei} nicht gebunden: {grund}")
            }
        }
    }
}

/// Was `bearbeiten` getan hat (ADR-0048 Regeln 7, 8): einen Änderungssatz angehängt, oder nichts, weil der Text
/// gleich der aktuellen Fassung ist — beides mit der Nachricht in ihrer jetzigen Fassung.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Bearbeitet {
    Neu(Nachricht),
    Unveraendert(Nachricht),
    /// Derselbe Wiederholschlüssel mit derselben Prüfsumme: die aktuelle Fassung, nichts geschrieben.
    Wiederholt(Nachricht),
}

/// Warum `bearbeiten` nichts angehängt hat (ADR-0048 Regel 5, Antworten unter der Sperre).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Bearbeitfehler {
    /// Das Gespräch ist weder geladen noch angelegt.
    Unbekannt,
    /// Keine Nachricht mit dieser Zeit in diesem Gespräch (nie da, entfallen, anderer Channel).
    Nachricht,
    /// Die Kennung ist nicht der Absender.
    Fremd { absender: Kennung },
    /// Die Nachricht ist gelöscht.
    Geloescht,
    /// Nur Leerraum, und die Nachricht hat keinen vorhandenen Anhang.
    Leer,
    /// Der Änderungssatz brächte die Gespräche über die Plattenquote; nichts angehängt (ADR-0048 Regel 10).
    Quote { belegt: u64, hoechstens: u64 },
    /// Die Tagesdatei nahm den Satz nicht an; nichts steht im Log oder Index.
    Ablage { datei: String, grund: String },
    /// Der Wiederholschlüssel gehört zu einer anderen Anfrage (anderer Text, andere Nachricht, Senden).
    SchluesselAnders,
}

/// Den Channel gibt es nicht — unbekannt oder kein gültiger Name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unbekannt;

/// Alle Gespräche des Ordners mit ihrem Verlauf in der Frist.
pub struct Gespraeche {
    ablage: Arc<dyn Logablage>,
    uhr: Arc<dyn Uhr>,
    verzeichnis: RwLock<Verzeichnis>,
    /// Schreibsperre über alle Gespräche (ADR-0035 Regel 1); hält die zuletzt vergebene Zeit und die
    /// Wiederholschlüssel (ADR-0034 Regel 6).
    schreiben: Mutex<Schreibstand>,
    /// Die zuletzt vergebene Zeit in ms seit 1970 — gesetzt unter `schreiben`, gelesen ohne sie (ADR-0035 Regel 4).
    letzte_ms: AtomicI64,
    defekte: Vec<Ladedefekt>,
    /// Änderungssätze ohne Nachricht beim Laden (ADR-0048 Regel 1).
    aenderungswaisen: u64,
    ladezeit: Duration,
    veroeffentlichen: ChannelVeroeffentlichen,
    nachricht_veroeffentlichen: NachrichtVeroeffentlichen,
    /// Plattenquote in Bytes, `None` heißt aus (ADR-0044 Regel 11); ohne `mit_quote` der Standard.
    hoechstens: Option<u64>,
    /// Σ Bytes aller Tage im Index — Invariante `belegt = Σ Verlauf::bytes()` (ADR-0044 Regel 2).
    belegung: quote::Belegung,
    quote_melden: quote::QuoteMelden,
    /// Wiederholschlüssel, die vor Ablauf weichen mussten — gezählt nach der Schreibsperre (ADR-0043 Ergänzung
    /// Karte #36), gemeldet an `verdraengt_melden` (im Start: Episode im Betriebsprotokoll).
    schluessel_verdraengt: std::sync::atomic::AtomicU64,
    verdraengt_melden: VerdraengtMelden,
    /// Anhang-Quote in Bytes, `None` heißt aus (ADR-0046 Ergänzung „F1“, Regel 1 dort); getrennt von `hoechstens`.
    anhangquote: Option<u64>,
    anhangzaehler: anhaenge::Zaehler,
    plattenauskunft: Arc<dyn platte::Plattenauskunft>,
    /// Eingang und Anhänge auf der Platte (A00008 Ergänzung „Karte #9 Security-Gate“).
    anhangablage: Arc<dyn anhangablage::Anhangablage>,
    /// Zufall der Anhang-IDs (ADR-0047 Frage 2).
    zufall: Arc<dyn crate::zufall::Zufall>,
    /// Gebundene Anhänge (A00016).
    anhangverzeichnis: anhangverzeichnis::Anhangverzeichnis,
    /// Was beim Laden der Anhänge übergangen wurde (`WARNING laden`).
    anhangbefunde: Vec<Anhangladebefund>,
    /// Anhang-Tagordner ohne Tagesdatei beim Laden (`WARNING laden Anhänge ohne Tagesdatei`).
    waisentage: Vec<Waisentag>,
    /// Ob `gebunden` über 90 % der Anhang-Quote steht (B0515); gewechselt nur über `anhangschwelle_pruefen`.
    anhangschwelle: std::sync::atomic::AtomicBool,
    anhangschwelle_melden: anhaenge::AnhangschwelleMelden,
}

/// Ein beim Laden übergangener Anhang bzw. ein Anhang-Ordner, der sich nicht öffnen ließ (ADR-0046 Regel 7 und
/// Ergänzung „Karte #9 Security-Gate“ Regel 3): Datei der Anhänge nur als Präfix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Anhangladebefund {
    pub gespraech: Gespraech,
    pub tag: NaiveDate,
    /// Zeile der Tagesdatei; ohne Zeile: der Ordner `anhaenge/<tag>` war nicht zu öffnen.
    pub zeile: Option<usize>,
    pub grund: String,
    pub datei: Option<String>,
}

/// Ein Anhang-Tagordner ohne Tagesdatei beim Laden (ADR-0046 Ergänzung „Review #53 B1“ Regel 2): alle Dateien darin
/// sind Waisen; `bytes` ist ihre Länge zusammen, nicht das Zählmaß.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Waisentag {
    pub gespraech: Gespraech,
    pub tag: NaiveDate,
    pub dateien: usize,
    pub bytes: u64,
}

/// Meldet einen vor Ablauf verdrängten Wiederholschlüssel; gerufen nach Freigabe der Schreibsperre (Regel E1.7).
pub type VerdraengtMelden = Arc<dyn Fn(&Verdraengt) + Send + Sync>;

struct Schreibstand {
    letzte: Option<Zeit>,
    wiederholung: Wiederholung,
    /// Der Anhangeingang liegt unter derselben Sperre (ADR-0046 Ergänzung „Karte #9 Gate“ Regel 5).
    eingang: anhangeingang::Eingang,
}

/// Veröffentlicht eine angehängte Nachricht (im Start: auf den Bus, ADR-0028); gerufen unter der Schreibsperre.
pub type NachrichtVeroeffentlichen = Arc<dyn Fn(&Gespraech, &Nachricht) + Send + Sync>;

/// Was beim Laden übersprungen wurde: eine defekte Zeile, eine nicht lesbare Tagesdatei oder ein Ordner, der
/// kein Channel ist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ladedefekt {
    /// Tagesdatei bzw. Ordner relativ zum Ordner.
    pub datei: String,
    pub art: Defektart,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Defektart {
    Zeile(usize),
    Unlesbar(String),
    /// Ein Ordner unter `channels/`, dessen Name kein Channelname ist (ADR-0020 Regel 6).
    KeinChannelname,
    /// Eine `einstellungen.toml`, die sich nicht deuten lässt (ADR-0022 Regel 3).
    Einstellungen {
        zeile: usize,
        grund: String,
    },
    /// Eine `einstellungen.toml`, die das Dateisystem verweigert.
    EinstellungenUnlesbar(String),
    /// Ein Änderungssatz, der nicht gilt (ADR-0048 Regel 1): Zeile seiner Kopfzeile und Grund als Wort
    /// (`fremd`, `nach_geloescht`, `zwei_bezuege`, `bezugszeit`, `reihenfolge`, `anhang`).
    Aenderung {
        zeile: usize,
        grund: &'static str,
    },
}

impl fmt::Display for Ladedefekt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.art {
            Defektart::Zeile(zeile) => {
                write!(f, "{}:{zeile}: defekte Zeile übersprungen", self.datei)
            }
            Defektart::Einstellungen { zeile, grund } => write!(
                f,
                "{}:{zeile}: Einstellungen nicht lesbar, als unbekannt gelesen ({grund})",
                self.datei
            ),
            Defektart::EinstellungenUnlesbar(grund) => write!(
                f,
                "{}: nicht lesbar, als unbekannt gelesen ({grund})",
                self.datei
            ),
            Defektart::KeinChannelname => {
                write!(f, "{}: kein gültiger Channelname, übergangen", self.datei)
            }
            Defektart::Aenderung { zeile, grund } => {
                write!(
                    f,
                    "{}:{zeile}: Änderungssatz übergangen ({grund})",
                    self.datei
                )
            }
            Defektart::Unlesbar(grund) => {
                write!(
                    f,
                    "{}: nicht lesbar, Tag übersprungen ({grund})",
                    self.datei
                )
            }
        }
    }
}

/// Kennzahlen für den Health-Check (R00002).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Kennzahlen {
    pub nachrichten: u64,
    pub log_bytes: u64,
    pub ladezeit_ms: u64,
    pub defekte_zeilen: u64,
    /// Channels im Verzeichnis — wie nah die Gesamtgrenze ist (ADR-0021).
    pub channels: u64,
    /// Wirksame Plattenquote in Bytes, `None` bei `aus` (ADR-0044 Regel 9).
    pub plattenquote: Option<u64>,
    /// Abweisungen an der Quote seit dem Start (ADR-0044 Regel 9).
    pub quote_abgewiesen: u64,
    /// Wiederholschlüssel, die seit dem Start vor Ablauf verdrängt wurden (ADR-0043 Ergänzung Karte #36).
    pub schluessel_verdraengt: u64,
}

impl Gespraeche {
    /// Lädt je Gespräch die Tage in der Frist; Defekte werden übersprungen und gesammelt, nicht abgebrochen.
    pub fn laden(ablage: Arc<dyn Logablage>, uhr: Arc<dyn Uhr>) -> Result<Gespraeche, Fehler> {
        let beginn = Instant::now();
        let heute = uhr.jetzt().date_naive();
        let mut geladen = Geladen::default();
        let gespraeche = ablage
            .gespraeche()
            .map_err(|fehler| Fehler::Ablage(format!("Gespräche nicht lesbar: {fehler}")))?;
        for gespraech in gespraeche {
            if !geladen.ist_gespraech(&gespraech) {
                continue;
            }
            let channel = geladen.channel_lesen(ablage.as_ref(), &gespraech);
            let frist_tage = channel
                .as_ref()
                .map_or(frist::STANDARD_FRIST_TAGE, |channel| {
                    channel.frist.zum_laden()
                });
            let tage = ablage.tage(&gespraech).map_err(|fehler| {
                Fehler::Ablage(format!("Tage von {gespraech} nicht lesbar: {fehler}"))
            })?;
            let mut verlauf = Verlauf::default();
            let (behalten, _) = frist::teilen(&tage, heute, frist_tage);
            for tag in behalten {
                geladen.tag_laden(ablage.as_ref(), &gespraech, tag, &mut verlauf);
            }
            geladen.aenderungen_melden(&gespraech, verlauf.fassungen_bilden());
            geladen.belegt += verlauf.bytes();
            if let Some(channel) = channel {
                geladen
                    .verzeichnis
                    .channels
                    .insert(channel.name.clone(), channel);
            }
            geladen
                .verzeichnis
                .verlaeufe
                .insert(gespraech, Arc::new(Mutex::new(verlauf)));
        }
        Ok(Gespraeche {
            ablage,
            uhr,
            verzeichnis: RwLock::new(geladen.verzeichnis),
            letzte_ms: AtomicI64::new(
                geladen
                    .juengste
                    .map_or(i64::MIN, |zeit| zeit.timestamp_millis()),
            ),
            schreiben: Mutex::new(Schreibstand {
                letzte: geladen.juengste,
                wiederholung: Wiederholung::default(),
                eingang: anhangeingang::Eingang::default(),
            }),
            defekte: geladen.defekte,
            aenderungswaisen: geladen.aenderungswaisen,
            ladezeit: beginn.elapsed(),
            veroeffentlichen: Arc::new(|_: &ChannelWechsel| {}),
            nachricht_veroeffentlichen: Arc::new(|_: &Gespraech, _: &Nachricht| {}),
            hoechstens: Some(quote::STANDARD_BYTES),
            belegung: quote::Belegung::neu(geladen.belegt),
            quote_melden: Arc::new(|_: &quote::Quotemeldung| {}),
            schluessel_verdraengt: std::sync::atomic::AtomicU64::new(0),
            verdraengt_melden: Arc::new(|_: &Verdraengt| {}),
            anhangquote: Some(anhaenge::STANDARD_ANHANGQUOTE),
            anhangzaehler: anhaenge::Zaehler::default(),
            plattenauskunft: Arc::new(platte::Unbegrenzt),
            anhangablage: Arc::new(anhangablage::KeineAnhangablage),
            anhangverzeichnis: anhangverzeichnis::Anhangverzeichnis::default(),
            anhangbefunde: Vec::new(),
            waisentage: Vec::new(),
            anhangschwelle: std::sync::atomic::AtomicBool::new(false),
            anhangschwelle_melden: Arc::new(|_: &anhaenge::Anhangschwelle| {}),
            zufall: Arc::new(crate::zufall::Betriebssystemzufall),
        })
    }

    /// Hängt eine Nachricht an die Tagesdatei des lokalen Datums ihrer Zeit. Sperrordnung Gespräch → Schreibsperre
    /// (ADR-0035 Regel 1): Zeit `max(jetzt, letzte + 1 ms)` → Satz → anhängen → Index → veröffentlichen, alles unter
    /// beiden Sperren — so sind die Zeiten über alle Gespräche streng steigend in der Folge, in der Log, Index und
    /// Bus sie sehen. Scheitert das Anhängen, bleibt der Index unverändert, nichts wird veröffentlicht und die
    /// vergebene Zeit verfällt (streng steigend, nicht lückenlos). Angelegt wird nichts.
    pub fn senden(
        &self,
        gespraech: &Gespraech,
        von: &Kennung,
        art: Art,
        text: &str,
    ) -> Result<Nachricht, Sendefehler> {
        match self.senden_mit(gespraech, von, art, text, &[], None)?.0 {
            Gesendet::Neu(nachricht) | Gesendet::Wiederholt(nachricht) => Ok(nachricht),
        }
    }

    /// Wie `senden`, mit Wiederholschlüssel (ADR-0034 Regel 6): unter der Schreibsperre geprüft — bekannt mit
    /// gleichem Text und Gespräch → die ursprüngliche Nachricht, nichts angehängt, nichts veröffentlicht; bekannt mit
    /// anderem → `SchluesselAnders`; neu → senden und erst nach dem Anhängen merken. Musste dafür ein anderer
    /// Schlüssel vor Ablauf weichen, wird das nach Freigabe der Sperren gezählt und gemeldet (Regel E1.7).
    pub fn senden_einmal(
        &self,
        gespraech: &Gespraech,
        von: &Kennung,
        art: Art,
        text: &str,
        schluessel: &str,
    ) -> Result<Gesendet, Sendefehler> {
        self.senden_mit_anhaengen(gespraech, von, art, text, &[], Some(schluessel))
    }

    /// Wie `senden_einmal`, mit Anhängen (ADR-0046 Regeln 5, 6): die IDs werden unter Gesprächs- und Schreibsperre
    /// genommen, in den Tag der Nachricht gebunden, dann der Satz mit ihren Metazeilen angehängt; scheitert das,
    /// liegen die Dateien wieder im Eingang und kein Satz steht im Log. Ohne Schlüssel keine Wiederholprüfung.
    pub fn senden_mit_anhaengen(
        &self,
        gespraech: &Gespraech,
        von: &Kennung,
        art: Art,
        text: &str,
        anhaenge: &[anhangid::Anhangid],
        schluessel: Option<&str>,
    ) -> Result<Gesendet, Sendefehler> {
        let (gesendet, verdraengt) =
            self.senden_mit(gespraech, von, art, text, anhaenge, schluessel)?;
        if let Some(verdraengt) = verdraengt {
            self.schluessel_verdraengt.fetch_add(1, Ordering::SeqCst);
            (self.verdraengt_melden)(&verdraengt);
        }
        Ok(gesendet)
    }

    fn senden_mit(
        &self,
        gespraech: &Gespraech,
        von: &Kennung,
        art: Art,
        text: &str,
        ids: &[anhangid::Anhangid],
        schluessel: Option<&str>,
    ) -> Result<(Gesendet, Option<Verdraengt>), Sendefehler> {
        let verlauf = lesend(&self.verzeichnis)
            .verlaeufe
            .get(gespraech)
            .cloned()
            .ok_or(Sendefehler::Unbekannt)?;
        let mut verlauf = sperren(&verlauf);
        let mut stand = sperren(&self.schreiben);
        let jetzt = self.uhr.jetzt();
        let pruefsumme = wiederholung::pruefsumme_mit(text, ids);
        let pruefung = schluessel.map(|schluessel| {
            stand
                .wiederholung
                .pruefen(von, schluessel, gespraech, pruefsumme, jetzt)
        });
        match pruefung {
            Some(Pruefung::Anders) => return Err(Sendefehler::SchluesselAnders),
            Some(Pruefung::Wiederholt { zeit }) => {
                let zeilen = self.anhangzeilen_der_ids(ids);
                return Ok((
                    Gesendet::Wiederholt(nachricht_bilden(zeit, von, art, text, zeilen).0),
                    None,
                ));
            }
            Some(Pruefung::Neu) | None => {}
        }
        let genommen = if ids.is_empty() {
            Vec::new()
        } else {
            stand
                .eingang
                .nehmen(von, ids, jetzt)
                .map_err(|fehler| Sendefehler::Anhang {
                    id: fehler.id,
                    grund: fehler.grund,
                })?
        };
        let zeit = crate::uhr::naechste_zeit(jetzt, stand.letzte);
        stand.letzte = Some(zeit);
        self.letzte_ms
            .store(zeit.timestamp_millis(), Ordering::Relaxed);
        let zeilen = anhangzeilen_bilden(zeit, &genommen);
        let (nachricht, satz) = nachricht_bilden(zeit, von, art, text, zeilen.clone());
        let satzbytes = satz.len() as u64;
        if let Some(hoechstens) = self.hoechstens
            && let Err(ueber) = self.belegung.pruefen(satzbytes, hoechstens)
        {
            stand.eingang.zuruecklegen(genommen);
            return Err(Sendefehler::Quote {
                belegt: ueber.belegt,
                hoechstens: ueber.hoechstens,
            });
        }
        let tag = nachricht.zeit.date_naive();
        let gescheitert =
            genommen
                .iter()
                .zip(&zeilen)
                .enumerate()
                .find_map(|(nummer, (eintrag, zeile))| {
                    self.anhangablage
                        .binden(&eintrag.id, gespraech, tag, &zeile.datei)
                        .err()
                        .map(|fehler| (nummer, fehler))
                });
        if let Some((nummer, fehler)) = gescheitert {
            let datei = anhangverzeichnis::praefix(&zeilen[nummer].datei);
            let waisen = self.zurueckbauen(
                &mut stand.eingang,
                gespraech,
                tag,
                genommen,
                &zeilen[..nummer],
            );
            return Err(Sendefehler::Binden {
                tag,
                datei,
                grund: fehler.to_string(),
                waisen,
            });
        }
        let angehaengt = match self.ablage.anhaengen(gespraech, tag, &satz) {
            Ok(angehaengt) => angehaengt,
            Err(fehler) => {
                let waisen =
                    self.zurueckbauen(&mut stand.eingang, gespraech, tag, genommen, &zeilen);
                if !waisen.is_empty() {
                    return Err(Sendefehler::Binden {
                        tag,
                        datei: gespraech.tagesdatei(tag),
                        grund: fehler.to_string(),
                        waisen,
                    });
                }
                return Err(Sendefehler::Ablage {
                    datei: gespraech.tagesdatei(tag),
                    grund: fehler.to_string(),
                });
            }
        };
        verlauf.anhaengen(tag, &satz, angehaengt.zeilenende_ergaenzt);
        let dazu = satzbytes + u64::from(angehaengt.zeilenende_ergaenzt);
        let vorher = self.belegung.dazu(dazu);
        self.ueber_90_melden(vorher, vorher.saturating_add(dazu));
        let gebunden: u64 = zeilen
            .iter()
            .map(|zeile| anhangquote::zaehlmass(zeile.bytes))
            .sum();
        self.anhangzaehler
            .gebunden
            .fetch_add(gebunden, Ordering::SeqCst);
        self.anhangschwelle_pruefen("gebunden");
        for zeile in &zeilen {
            self.anhangverzeichnis
                .eintragen(anhangverzeichnis::Eintrag::neu(
                    zeile,
                    gespraech,
                    tag,
                    zeit,
                    von,
                    anhangverzeichnis::Zustand::Vorhanden,
                ));
        }
        self.eingang_spiegeln(&stand.eingang);
        let verdraengt = schluessel.and_then(|schluessel| {
            stand
                .wiederholung
                .merken(von, schluessel, gespraech, pruefsumme, zeit, jetzt)
        });
        (self.nachricht_veroeffentlichen)(gespraech, &nachricht);
        Ok((Gesendet::Neu(nachricht), verdraengt))
    }

    /// Bearbeitet den Text der Nachricht `ziel` (ADR-0048 Regel 8) unter Gesprächs- und Schreibsperre: nachschlagen
    /// (`Nachricht`, `Fremd`, `Geloescht`, `Leer`) → unverändert? → Quote → Zeit vergeben → Bearbeitungssatz in die
    /// Tagesdatei der Änderung anhängen (scheitert es, nichts geändert, die Zeit verfällt) → Index und Fassung →
    /// `belegt`. Die Anhänge bleiben, wie sie sind.
    pub fn bearbeiten(
        &self,
        gespraech: &Gespraech,
        von: &Kennung,
        ziel: Zeit,
        text: &str,
    ) -> Result<Bearbeitet, Bearbeitfehler> {
        self.bearbeiten_mit(gespraech, von, ziel, text, None)
    }

    /// Wie `bearbeiten`, mit Wiederholschlüssel (ADR-0048 Regel 8): derselbe Speicher wie beim Senden, geprüft vor dem
    /// Nachschlagen — bekannt mit gleicher Prüfsumme → `Wiederholt` mit der aktuellen Fassung, anders →
    /// `SchluesselAnders`; neu → bearbeiten und nach dem Anhängen merken.
    pub fn bearbeiten_mit(
        &self,
        gespraech: &Gespraech,
        von: &Kennung,
        ziel: Zeit,
        text: &str,
        schluessel: Option<&str>,
    ) -> Result<Bearbeitet, Bearbeitfehler> {
        let (bearbeitet, verdraengt) =
            self.bearbeiten_unter_sperre(gespraech, von, ziel, text, schluessel)?;
        if let Some(verdraengt) = verdraengt {
            self.schluessel_verdraengt.fetch_add(1, Ordering::SeqCst);
            (self.verdraengt_melden)(&verdraengt);
        }
        Ok(bearbeitet)
    }

    fn bearbeiten_unter_sperre(
        &self,
        gespraech: &Gespraech,
        von: &Kennung,
        ziel: Zeit,
        text: &str,
        schluessel: Option<&str>,
    ) -> Result<(Bearbeitet, Option<Verdraengt>), Bearbeitfehler> {
        let verlauf = lesend(&self.verzeichnis)
            .verlaeufe
            .get(gespraech)
            .cloned()
            .ok_or(Bearbeitfehler::Unbekannt)?;
        let mut verlauf = sperren(&verlauf);
        let mut stand = sperren(&self.schreiben);
        let jetzt = self.uhr.jetzt();
        let pruefsumme = wiederholung::pruefsumme_bearbeiten(&ziel, text);
        let pruefung = schluessel.map(|schluessel| {
            stand
                .wiederholung
                .pruefen(von, schluessel, gespraech, pruefsumme, jetzt)
        });
        let nachricht = verlauf
            .nachricht(&ziel)
            .as_ref()
            .and_then(nachricht_in_fassung)
            .map(|nachricht| self.zustand_eintragen(nachricht));
        match pruefung {
            Some(Pruefung::Anders) => return Err(Bearbeitfehler::SchluesselAnders),
            Some(Pruefung::Wiederholt { .. }) => {
                let nachricht = nachricht.ok_or(Bearbeitfehler::Nachricht)?;
                return Ok((Bearbeitet::Wiederholt(nachricht), None));
            }
            Some(Pruefung::Neu) | None => {}
        }
        let nachricht = nachricht.ok_or(Bearbeitfehler::Nachricht)?;
        let text = neuer_text(&nachricht, von, text)?;
        if text == nachricht.text {
            return Ok((Bearbeitet::Unveraendert(nachricht), None));
        }
        let zeit = crate::uhr::naechste_zeit(jetzt, stand.letzte);
        let satz = logformat::bearbeitungssatz(zeit, von, nachricht.zeit, &text);
        let satzbytes = satz.len() as u64;
        if let Some(hoechstens) = self.hoechstens
            && let Err(ueber) = self.belegung.pruefen(satzbytes, hoechstens)
        {
            return Err(Bearbeitfehler::Quote {
                belegt: ueber.belegt,
                hoechstens: ueber.hoechstens,
            });
        }
        stand.letzte = Some(zeit);
        self.letzte_ms
            .store(zeit.timestamp_millis(), Ordering::Relaxed);
        let tag = zeit.date_naive();
        let angehaengt = self
            .ablage
            .anhaengen(gespraech, tag, &satz)
            .map_err(|fehler| Bearbeitfehler::Ablage {
                datei: gespraech.tagesdatei(tag),
                grund: fehler.to_string(),
            })?;
        verlauf.bearbeitung_anhaengen(
            tag,
            &satz,
            angehaengt.zeilenende_ergaenzt,
            nachricht.zeit,
            zeit,
        );
        let dazu = satzbytes + u64::from(angehaengt.zeilenende_ergaenzt);
        let vorher = self.belegung.dazu(dazu);
        self.ueber_90_melden(vorher, vorher.saturating_add(dazu));
        let verdraengt = schluessel.and_then(|schluessel| {
            stand.wiederholung.merken(
                von,
                schluessel,
                gespraech,
                pruefsumme,
                nachricht.zeit,
                jetzt,
            )
        });
        let neu = Nachricht {
            text,
            geaendert: Some(Geaendert {
                zeit,
                zustand: Aenderungszustand::Bearbeitet,
            }),
            ..nachricht
        };
        Ok((Bearbeitet::Neu(neu), verdraengt))
    }

    /// Rückbau eines gescheiterten Bindens (ADR-0046 Regel 6): jede schon gebundene Datei zurück in den Eingang
    /// (ohne Überschreiben), dann die Einträge unverändert zurück. Scheitert das Zurücklegen, bleibt die Datei als
    /// Waise im Tag: sie zählt an `G`, ihr Eintrag verlässt den Eingang (Invarianten aus „F1′“ Regel 1). Liefert die
    /// Präfixe der Waisen.
    fn zurueckbauen(
        &self,
        eingang: &mut anhangeingang::Eingang,
        gespraech: &Gespraech,
        tag: NaiveDate,
        genommen: Vec<anhangeingang::Genommen>,
        gebunden: &[logformat::Anhangzeile],
    ) -> Vec<String> {
        let mut waisen = Vec::new();
        let mut zurueck = Vec::new();
        for eintrag in genommen {
            let zeile = gebunden.iter().find(|zeile| zeile.id == eintrag.id);
            match zeile {
                Some(zeile) => {
                    match self
                        .anhangablage
                        .zuruecklegen(gespraech, tag, &zeile.datei, &eintrag.id)
                    {
                        Ok(()) => zurueck.push(eintrag),
                        Err(_) => {
                            let z = anhangquote::zaehlmass(eintrag.bytes());
                            self.anhangverzeichnis.waise(gespraech, tag, z);
                            self.anhangzaehler.gebunden.fetch_add(z, Ordering::SeqCst);
                            waisen.push(anhangverzeichnis::praefix(&zeile.datei));
                        }
                    }
                }
                None => zurueck.push(eintrag),
            }
        }
        eingang.zuruecklegen(zurueck);
        self.eingang_spiegeln(eingang);
        waisen
    }

    /// Die Metazeilen zu gebundenen IDs (für die Antwort auf eine Wiederholung).
    fn anhangzeilen_der_ids(&self, ids: &[anhangid::Anhangid]) -> Vec<logformat::Anhangzeile> {
        ids.iter()
            .filter_map(|id| self.anhangverzeichnis.finden(id))
            .map(|eintrag| logformat::Anhangzeile {
                id: eintrag.id.clone(),
                bytes: eintrag.bytes,
                art: eintrag.art,
                masse: eintrag.masse,
                datei: eintrag.datei.clone(),
                name: eintrag.name.clone(),
                geloescht: eintrag.zustand() == anhangverzeichnis::Zustand::Geloescht,
            })
            .collect()
    }

    /// Die jüngste Nachrichtenzeit im Ordner (als Zeitpunkt); an ihr setzt die nächste vergebene Zeit an.
    pub fn letzte_zeit(&self) -> Option<Zeit> {
        sperren(&self.schreiben).letzte
    }

    /// Wie weit die zuletzt vergebene Zeit vor der Uhr liegt, in ms, 0 ohne Vorlauf — ohne die Schreibsperre, damit
    /// ein hängendes Plattenschreiben den Health-Check nicht anhält (ADR-0035 Regel 4).
    pub fn zeit_vorlauf_ms(&self) -> u64 {
        let letzte = self.letzte_ms.load(Ordering::Relaxed);
        let jetzt = self.uhr.jetzt().timestamp_millis();
        u64::try_from(letzte.saturating_sub(jetzt)).unwrap_or(0)
    }

    /// Steigt `belegt` beim Senden über 90 % der Quote, einmal melden (ADR-0044 Regel 7).
    fn ueber_90_melden(&self, vorher: u64, nachher: u64) {
        let Some(hoechstens) = self.hoechstens else {
            return;
        };
        if quote::wechsel(vorher, nachher, hoechstens) == Some(quote::Richtung::NachOben) {
            (self.quote_melden)(&quote::Quotemeldung::Ueber90 {
                belegt: nachher,
                hoechstens,
            });
        }
    }

    /// Fällt `belegt` beim Entfallen wieder unter 90 % der Quote, einmal melden mit dem Anlass (ADR-0044 Regel 7).
    fn unter_90_melden(&self, vorher: u64, nachher: u64, anlass: &Pflegeanlass) {
        let Some(hoechstens) = self.hoechstens else {
            return;
        };
        if quote::wechsel(vorher, nachher, hoechstens) == Some(quote::Richtung::NachUnten) {
            (self.quote_melden)(&quote::Quotemeldung::Unter90 {
                belegt: nachher,
                hoechstens,
                anlass: anlass.clone(),
            });
        }
    }

    /// Der Stand der Quote nach dem Laden: über der Quote oder über 90 %, sonst nichts; ohne Quote nichts.
    pub fn quote_beim_start(&self) -> Option<quote::Quotemeldung> {
        self.hoechstens
            .and_then(|hoechstens| quote::beim_start(self.belegt(), hoechstens))
    }

    /// Bytes, die die Gespräche belegen (ADR-0044 Regel 2).
    pub fn belegt(&self) -> u64 {
        self.belegung.stand()
    }

    pub fn defekte(&self) -> &[Ladedefekt] {
        &self.defekte
    }

    /// Änderungssätze ohne Nachricht beim Laden (ADR-0048 Regel 1, Waisen) — still übergangen, nur gezählt.
    pub fn aenderungswaisen(&self) -> u64 {
        self.aenderungswaisen
    }

    pub fn kennzahlen(&self) -> Kennzahlen {
        let mut nachrichten = 0;
        let verzeichnis = lesend(&self.verzeichnis);
        for verlauf in verzeichnis.verlaeufe.values() {
            nachrichten += sperren(verlauf).nachrichten();
        }
        Kennzahlen {
            nachrichten,
            log_bytes: self.belegt(),
            ladezeit_ms: u64::try_from(self.ladezeit.as_millis()).unwrap_or(u64::MAX),
            defekte_zeilen: self.defekte.len() as u64,
            channels: verzeichnis.channels.len() as u64,
            plattenquote: self.hoechstens,
            quote_abgewiesen: self.belegung.abgewiesen(),
            schluessel_verdraengt: self.schluessel_verdraengt.load(Ordering::SeqCst),
        }
    }
}

impl Gespraeche {
    /// Ersetzt den Wiederholspeicher (Grenzen aus Unit-Tests oder dem Testknopf, ADR-0042 Ergänzung Karte #36).
    pub fn mit_wiederholung(mut self, wiederholung: Wiederholung) -> Gespraeche {
        self.schreiben
            .get_mut()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .wiederholung = wiederholung;
        self
    }

    /// Hängt die Meldung verdrängter Wiederholschlüssel an (im Start: Episode im Betriebsprotokoll).
    pub fn mit_verdraengt_melden(self, verdraengt_melden: VerdraengtMelden) -> Gespraeche {
        Gespraeche {
            verdraengt_melden,
            ..self
        }
    }

    /// Ob die Schreibsperre gerade frei ist — nur für die Probe „nach der Sperre“.
    #[cfg(test)]
    fn schreibsperre_frei(&self) -> bool {
        self.schreiben.try_lock().is_ok()
    }

    /// Hängt die Meldungen der Quote an (im Start: das Betriebsprotokoll); ohne sie gehen sie an niemanden.
    pub fn mit_quote_melden(self, quote_melden: quote::QuoteMelden) -> Gespraeche {
        Gespraeche {
            quote_melden,
            ..self
        }
    }

    /// Setzt die Plattenquote in Bytes; `None` heißt aus (ADR-0044 Regel 11).
    pub fn mit_quote(self, hoechstens: Option<u64>) -> Gespraeche {
        Gespraeche { hoechstens, ..self }
    }

    /// Hängt die Veröffentlichung der Channel-Wechsel an (im Start: der Bus); ohne sie gehen sie an niemanden.
    pub fn mit_veroeffentlichen(self, veroeffentlichen: ChannelVeroeffentlichen) -> Gespraeche {
        Gespraeche {
            veroeffentlichen,
            ..self
        }
    }

    /// Hängt die Veröffentlichung der Nachrichten an (im Start: der Bus); ohne sie gehen sie an niemanden.
    pub fn mit_nachricht_veroeffentlichen(
        self,
        nachricht_veroeffentlichen: NachrichtVeroeffentlichen,
    ) -> Gespraeche {
        Gespraeche {
            nachricht_veroeffentlichen,
            ..self
        }
    }

    /// Legt einen Channel an, unter der Sperre des Verzeichnisses (ADR-0021 Regel 5): Namensregel → vorhanden →
    /// Grenzen (gesamt, je Herkunft) → Ordner → Einstellungen → eintragen, `stand` + 1 → veröffentlichen, noch unter der Sperre (ADR-0023 Regel 2).
    pub fn channel_anlegen(
        &self,
        name: &str,
        beschreibung: Option<einstellungen::Beschreibung>,
        von: &Kennung,
    ) -> Result<Channel, Anlegefehler> {
        let name = Channelname::bilden(name).map_err(Anlegefehler::Regel)?;
        let mut verzeichnis = schreibend(&self.verzeichnis);
        if let Some(vorhanden) = verzeichnis.channels.get(&name) {
            return Err(Anlegefehler::Vorhanden(Box::new(vorhanden.clone())));
        }
        verzeichnis::grenze_pruefen(verzeichnis.channels.values(), von.herkunft())
            .map_err(Anlegefehler::Grenze)?;
        let gespraech = Gespraech::channel(name.als_text());
        let channel = Channel {
            name,
            angelegt: Some(crate::uhr::auf_millisekunden(self.uhr.jetzt())),
            angelegt_von: Some(von.clone()),
            beschreibung,
            frist: Frist::Tage(STANDARD_FRIST_TAGE),
        };
        self.ablegen(&gespraech, &channel)?;
        verzeichnis
            .verlaeufe
            .insert(gespraech, Arc::new(Mutex::new(Verlauf::default())));
        verzeichnis
            .channels
            .insert(channel.name.clone(), channel.clone());
        verzeichnis.stand += 1;
        (self.veroeffentlichen)(&ChannelWechsel {
            stand: verzeichnis.stand,
            channel: channel.clone(),
        });
        Ok(channel)
    }

    /// Ordner und Einstellungsdatei des neuen Channels (ADR-0020 Regel 5, ADR-0022 Regel 2); scheitern die
    /// Einstellungen, wird der eben angelegte Ordner entfernt — kein halber Channel (ADR-0021 Regel 1, 500).
    fn ablegen(&self, gespraech: &Gespraech, channel: &Channel) -> Result<(), Anlegefehler> {
        let einstellungen = einstellungen::Einstellungen {
            angelegt: channel.angelegt,
            angelegt_von: channel.angelegt_von.clone(),
            beschreibung: channel.beschreibung.clone(),
            frist_tage: None,
        };
        let text = einstellungen::schreiben(&einstellungen, None);
        self.ablage
            .channel_ordner_anlegen(gespraech)
            .map_err(|fehler| match fehler.kind() {
                std::io::ErrorKind::AlreadyExists => Anlegefehler::OrdnerVorhanden,
                _ => Anlegefehler::Ablage(format!("{gespraech}: {fehler}")),
            })?;
        let geschrieben = self
            .ablage
            .einstellungen_schreiben(gespraech, text.as_bytes());
        geschrieben.map_err(|fehler| {
            let mut grund = format!("{}: {fehler}", gespraech.einstellungsdatei());
            if let Err(rueckbau) = self.ablage.ordner_entfernen(gespraech) {
                grund.push_str(&format!(
                    "; Rückbau von {gespraech} gescheitert ({rueckbau})"
                ));
            }
            Anlegefehler::Ablage(grund)
        })
    }

    /// Ein Block aus dem Verlauf des Channels `name` mit höchstens `budget` Satzbytes (ADR-0025 Regel 5,
    /// ADR-0039 Regel 2): das Verzeichnis lesend nur zum Nachschlagen, die Sperre des Gesprächs nur zum
    /// Herauskopieren der Sätze; gedeutet wird ohne Sperre.
    pub fn verlauf(
        &self,
        name: &str,
        abfrage: &abfrage::Abfrage,
        budget: usize,
    ) -> Result<Nachrichtenblock, Unbekannt> {
        let name = Channelname::bilden(name).map_err(|_| Unbekannt)?;
        let verlauf = lesend(&self.verzeichnis)
            .verlaeufe
            .get(&Gespraech::channel(name.als_text()))
            .cloned()
            .ok_or(Unbekannt)?;
        let block = sperren(&verlauf).block(abfrage, budget);
        Ok(Nachrichtenblock {
            nachrichten: block
                .saetze
                .iter()
                .filter_map(nachricht_in_fassung)
                .map(|nachricht| self.zustand_eintragen(nachricht))
                .collect(),
            aelter: block.aelter,
            neuer: block.neuer,
        })
    }

    /// Je Anhang, ob er gelöscht ist (F0117): nur das Verzeichnis weiß es, die Metazeile bleibt.
    fn zustand_eintragen(&self, mut nachricht: Nachricht) -> Nachricht {
        for anhang in &mut nachricht.anhaenge {
            anhang.geloescht = self
                .anhangverzeichnis
                .finden(&anhang.id)
                .is_some_and(|eintrag| eintrag.zustand() == anhangverzeichnis::Zustand::Geloescht);
        }
        nachricht
    }

    /// Ob es den Channel `name` gibt (gültiger Name, im Verzeichnis).
    pub fn channel_bekannt(&self, name: &str) -> bool {
        Channelname::bilden(name)
            .is_ok_and(|name| lesend(&self.verzeichnis).channels.contains_key(&name))
    }

    /// Alle Channels nach Name mit `stand`.
    pub fn channels(&self) -> ChannelStand {
        lesend(&self.verzeichnis).stand()
    }
}

/// Was das Laden sammelt, bis alle Gespräche durch sind.
#[derive(Default)]
struct Geladen {
    verzeichnis: Verzeichnis,
    defekte: Vec<Ladedefekt>,
    juengste: Option<Zeit>,
    /// Σ Bytes der geladenen Tage aller Gespräche.
    belegt: u64,
    aenderungswaisen: u64,
}

impl Geladen {
    /// Ein Unterordner von `channels/` mit ungültigem Namen ist kein Gespräch (ADR-0020 Regel 6): beginnt er mit
    /// `.`, still übergangen, sonst als Defekt gemeldet; er bleibt, wie er ist.
    fn ist_gespraech(&mut self, gespraech: &Gespraech) -> bool {
        let Gespraech::Channel(ordner) = gespraech else {
            return true;
        };
        if Channelname::bilden(ordner).is_ok() {
            return true;
        }
        if !ordner.starts_with('.') {
            self.defekte.push(Ladedefekt {
                datei: gespraech.pfad(),
                art: Defektart::KeinChannelname,
            });
        }
        false
    }

    /// Ein Ordner unter `channels/` mit gültigem Namen ist ein Channel; `angelegt`, `angelegt_von`, Beschreibung und
    /// Frist kommen aus seiner Einstellungsdatei, fehlt sie, sind sie unbekannt bzw. 7 Tage; ist sie defekt, ist die
    /// Frist gesperrt (ADR-0022 Regel 3, ADR-0032 Regel 5). Ein Direktgespräch ist kein Channel.
    fn channel_lesen(&mut self, ablage: &dyn Logablage, gespraech: &Gespraech) -> Option<Channel> {
        let Gespraech::Channel(ordner) = gespraech else {
            return None;
        };
        let name = Channelname::bilden(ordner).ok()?;
        let (gelesen, frist) = self.einstellungen_laden(ablage, gespraech);
        Some(Channel {
            name,
            angelegt: gelesen.angelegt,
            angelegt_von: gelesen.angelegt_von,
            beschreibung: gelesen.beschreibung,
            frist,
        })
    }

    /// Übergangene Änderungssätze eines Gesprächs als Ladedefekte mit Datei und Zeile; Waisen nur gezählt.
    fn aenderungen_melden(
        &mut self,
        gespraech: &Gespraech,
        bericht: verlaufsindex::Fassungsbericht,
    ) {
        use logformat::Aenderungsdefekt;
        use verlaufsindex::Aenderungsgrund;
        self.aenderungswaisen += bericht.waisen;
        for befund in bericht.defekte {
            let grund = match befund.grund {
                Aenderungsgrund::Fremd => "fremd",
                Aenderungsgrund::NachGeloescht => "nach_geloescht",
                Aenderungsgrund::Satz(Aenderungsdefekt::ZweiBezuege) => "zwei_bezuege",
                Aenderungsgrund::Satz(Aenderungsdefekt::Bezugszeit) => "bezugszeit",
                Aenderungsgrund::Satz(Aenderungsdefekt::NichtVorher) => "reihenfolge",
                Aenderungsgrund::Satz(Aenderungsdefekt::Anhang) => "anhang",
            };
            self.defekte.push(Ladedefekt {
                datei: gespraech.tagesdatei(befund.tag),
                art: Defektart::Aenderung {
                    zeile: befund.zeile,
                    grund,
                },
            });
        }
    }

    /// Liest einen Tag, nimmt seine gültigen Sätze in den Verlauf und merkt Defekte und jüngste Zeit.
    fn tag_laden(
        &mut self,
        ablage: &dyn Logablage,
        gespraech: &Gespraech,
        tag: NaiveDate,
        verlauf: &mut Verlauf,
    ) {
        let datei = gespraech.tagesdatei(tag);
        let rohtext = match ablage.lesen(gespraech, tag) {
            Ok(rohtext) => rohtext,
            Err(fehler) => {
                self.defekte.push(Ladedefekt {
                    datei,
                    art: Defektart::Unlesbar(fehler.to_string()),
                });
                return;
            }
        };
        let mut satzanfaenge = Vec::new();
        let (saetze, traegt_anhang) = logformat::saetze_und_metazeilen(&rohtext);
        for gelesen in saetze {
            match gelesen {
                Ok(satz) => {
                    satzanfaenge.push(satz.anfang);
                    self.juengste = self.juengste.max(Some(satz.zeit));
                }
                Err(defekt) => self.defekte.push(Ladedefekt {
                    datei: datei.clone(),
                    art: Defektart::Zeile(defekt.zeile),
                }),
            }
        }
        verlauf.tag_mit_anhangflagge(tag, rohtext, satzanfaenge, traegt_anhang);
    }
}

/// Eine kopierte Nachricht in ihrer Fassung (ADR-0048 Regeln 3, 4): bearbeitet mit dem Text des Änderungssatzes,
/// gelöscht mit leerem Text und ohne Anhänge.
fn nachricht_in_fassung(kopie: &verlaufsindex::Kopie) -> Option<Nachricht> {
    let mut nachricht = logformat::satz_als_nachricht(&kopie.satz)?;
    if let Some(aenderung) = &kopie.aenderung {
        let zustand = match &aenderung.satz {
            Some(satz) => {
                nachricht.text = logformat::text(&String::from_utf8_lossy(satz));
                Aenderungszustand::Bearbeitet
            }
            None => {
                nachricht.text.clear();
                nachricht.anhaenge.clear();
                Aenderungszustand::Geloescht
            }
        };
        nachricht.geaendert = Some(Geaendert {
            zeit: aenderung.geaendert,
            zustand,
        });
    }
    Some(nachricht)
}

/// Der neue Text, wie er im Log stehen wird — nach den Prüfungen unter der Sperre (ADR-0048 Regeln 6, 7): Absender,
/// nicht gelöscht, nur Leerraum nur mit einem vorhandenen Anhang (dann `""`).
fn neuer_text(nachricht: &Nachricht, von: &Kennung, text: &str) -> Result<String, Bearbeitfehler> {
    if &nachricht.von != von {
        return Err(Bearbeitfehler::Fremd {
            absender: nachricht.von.clone(),
        });
    }
    if nachricht
        .geaendert
        .is_some_and(|geaendert| geaendert.zustand == Aenderungszustand::Geloescht)
    {
        return Err(Bearbeitfehler::Geloescht);
    }
    if !text.chars().all(char::is_whitespace) {
        return Ok(logformat::text_wie_im_log(text));
    }
    if nachricht.anhaenge.iter().any(|anhang| !anhang.geloescht) {
        Ok(String::new())
    } else {
        Err(Bearbeitfehler::Leer)
    }
}

/// Lesend: wie `sperren` — das Verzeichnis wird erst nach erfolgreichem Schreiben verändert.
fn lesend<T>(sperre: &RwLock<T>) -> std::sync::RwLockReadGuard<'_, T> {
    sperre
        .read()
        .unwrap_or_else(|vergiftet| vergiftet.into_inner())
}

/// Die Nachricht mit ihrem Satz; der Text so, wie die Verlauf-API ihn später liefert (ADR-0034 Regel 4).
fn nachricht_bilden(
    zeit: Zeit,
    von: &Kennung,
    art: Art,
    text: &str,
    anhaenge: Vec<logformat::Anhangzeile>,
) -> (Nachricht, String) {
    let mut nachricht = Nachricht {
        zeit,
        von: von.clone(),
        art: Some(art),
        text: text.to_string(),
        anhaenge,
        geaendert: None,
    };
    let satz = logformat::satz(&nachricht);
    nachricht.text = logformat::text(&satz);
    (nachricht, satz)
}

/// Die Metazeilen der genommenen Anhänge: gespeicherter Name `<hhmmssmmm>-<n>-<ascii>` aus der lokalen Zeit der
/// Nachricht und der Sendefolge ab 1 (ADR-0046 Regel 4).
fn anhangzeilen_bilden(
    zeit: Zeit,
    genommen: &[anhangeingang::Genommen],
) -> Vec<logformat::Anhangzeile> {
    let stempel = zeit.format("%H%M%S%3f").to_string();
    genommen
        .iter()
        .enumerate()
        .map(|(nummer, eintrag)| {
            let art = eintrag.art();
            logformat::Anhangzeile {
                id: eintrag.id.clone(),
                bytes: eintrag.bytes(),
                art: art.typ,
                masse: art.breite.zip(art.hoehe),
                datei: format!("{stempel}-{}-{}", nummer + 1, eintrag.ascii()),
                name: eintrag.name().to_string(),
                geloescht: false,
            }
        })
        .collect()
}

fn schreibend<T>(sperre: &RwLock<T>) -> std::sync::RwLockWriteGuard<'_, T> {
    sperre
        .write()
        .unwrap_or_else(|vergiftet| vergiftet.into_inner())
}

impl Geladen {
    /// Die Einstellungen eines Channels und seine Frist; fehlt die Datei, unbekannt und 7 Tage; ist sie defekt oder
    /// nicht lesbar, unbekannt, die Frist gesperrt und ein Ladedefekt — die Datei bleibt unangetastet.
    fn einstellungen_laden(
        &mut self,
        ablage: &dyn Logablage,
        gespraech: &Gespraech,
    ) -> (einstellungen::Einstellungen, Frist) {
        let datei = gespraech.einstellungsdatei();
        let (art, zeile) = match ablage.einstellungen_lesen(gespraech) {
            Ok(None) => {
                return (
                    einstellungen::Einstellungen::default(),
                    Frist::Tage(STANDARD_FRIST_TAGE),
                );
            }
            Ok(Some(bytes)) => match einstellungen::lesen(&bytes) {
                Ok(gelesen) => {
                    let frist = Frist::Tage(gelesen.frist_tage.unwrap_or(STANDARD_FRIST_TAGE));
                    return (gelesen, frist);
                }
                Err(defekt) => (
                    Defektart::Einstellungen {
                        zeile: defekt.zeile,
                        grund: defekt.grund,
                    },
                    Some(defekt.zeile),
                ),
            },
            Err(fehler) => (Defektart::EinstellungenUnlesbar(fehler.to_string()), None),
        };
        self.defekte.push(Ladedefekt {
            datei: datei.clone(),
            art,
        });
        (
            einstellungen::Einstellungen::default(),
            Frist::Gesperrt { datei, zeile },
        )
    }
}

/// Eine vergiftete Sperre heißt nur, dass ein anderer Aufruf abbrach; der Verlauf selbst bleibt gültig,
/// weil er erst nach erfolgreichem Schreiben verändert wird.
/// Im Testbau meldet sie jeden Schritt einem Beobachter je Thread (Naht für den Sperrordnungstest); im Release-Bau
/// gibt es diese Zeilen nicht.
fn sperren<T>(sperre: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    #[cfg(test)]
    tests::sperrhaken(tests::Sperrschritt::Will(std::ptr::from_ref(sperre).addr()));
    let gesperrt = sperre
        .lock()
        .unwrap_or_else(|vergiftet| vergiftet.into_inner());
    #[cfg(test)]
    tests::sperrhaken(tests::Sperrschritt::Hat(std::ptr::from_ref(sperre).addr()));
    gesperrt
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uhr::GestellteUhr;
    use chrono::DateTime;
    use std::fs;
    use std::path::Path;
    use std::sync::Arc;

    const TAG_26: &str = "@ 2026-10-26T09:00:00.000+01:00 shoff@127.0.0.1\n  Guten Morgen\n\n@ 2026-10-26T09:01:00.000+01:00 claude@10.0.0.5\n  Scan läuft.\n\n";
    const TAG_27: &str = "@ 2026-10-27T10:00:00.000+01:00 shoff@127.0.0.1\n  Wie weit?\n\nkaputt\n@ 2026-10-27T10:00:05.000+01:00 claude@10.0.0.5\n  Fertig.\n\n";
    const DIREKT: &str = "@ 2026-10-27T11:00:00.000+01:00 claude@10.0.0.5\n  privat\n\n";

    fn zeit(text: &str) -> Zeit {
        DateTime::parse_from_rfc3339(text).expect("Testzeit")
    }

    fn datei(ordner: &Path, pfad: &str, inhalt: &[u8]) {
        let ziel = ordner.join(pfad);
        fs::create_dir_all(ziel.parent().expect("Elternordner")).expect("Ordner");
        fs::write(ziel, inhalt).expect("Datei");
    }

    fn laden_um(ordner: &Path, jetzt: &str) -> Gespraeche {
        Gespraeche::laden(
            Arc::new(Dateiablage::neu(
                ordner,
                Arc::new(crate::protokoll::Protokoll::ohne_ablage()),
            )),
            Arc::new(GestellteUhr::neu(zeit(jetzt))),
        )
        .expect("geladen")
    }

    /// Speicher-Logablage mit dem leeren Channel `allgemein` und eine Uhr, beides für den Test zugänglich.
    fn im_speicher(jetzt: &str) -> (Gespraeche, Arc<Speicherablage>, Arc<GestellteUhr>) {
        let ablage = Arc::new(Speicherablage::neu());
        ablage.gespraech_anlegen(&Gespraech::channel("allgemein"));
        let uhr = Arc::new(GestellteUhr::neu(zeit(jetzt)));
        let gespraeche = Gespraeche::laden(ablage.clone(), uhr.clone()).expect("geladen");
        (gespraeche, ablage, uhr)
    }

    fn claude() -> Kennung {
        Kennung::bilden("claude@10.0.0.5").expect("Kennung")
    }

    fn tag(text: &str) -> NaiveDate {
        NaiveDate::parse_from_str(text, "%Y-%m-%d").expect("Testtag")
    }

    /// ADR-0034 „Erster Unit-Test“ (R00012 B0238): `k1` zweimal mit gleichem Text → einmal angehängt, die zweite
    /// `Wiederholt` mit derselben Zeit und dem Text wie im Verlauf, ein Rückruf; `k1` mit anderem Text →
    /// `SchluesselAnders`; derselbe `k1` von einer anderen Kennung → neu; nach 10 min + 1 ms → neu.
    #[test]
    fn ein_wiederholschluessel_haengt_einmal_an_und_veroeffentlicht_einmal() {
        let (gespraeche, ablage, uhr) = im_speicher("2026-10-27T14:07:12.345+01:00");
        // 10 min + 1 ms nach dem Merken (14:07:12.345) ist 14:17:12.346.
        let rueckrufe: Arc<Mutex<Vec<String>>> = Arc::default();
        let gespraeche = gespraeche.mit_nachricht_veroeffentlichen({
            let rueckrufe = Arc::clone(&rueckrufe);
            Arc::new(move |_: &Gespraech, nachricht: &Nachricht| {
                sperren(&rueckrufe).push(nachricht.text.clone());
            })
        });
        let allgemein = Gespraech::channel("allgemein");
        let beta = Kennung::bilden("beta@127.0.0.1").expect("Kennung");
        let senden = |von: &Kennung, text: &str| {
            gespraeche.senden_einmal(&allgemein, von, Art::Agent, text, "k1")
        };

        let erste = senden(&beta, "Hallo\r\n\n").expect("erste");
        uhr.stellen(zeit("2026-10-27T14:07:12.350+01:00"));
        let zweite = senden(&beta, "Hallo\r\n\n").expect("zweite");
        let andere = senden(&beta, "Hallo!");
        let fremd = senden(&claude(), "Hallo\r\n\n").expect("fremd");
        uhr.stellen(zeit("2026-10-27T14:17:12.346+01:00"));
        let spaeter = senden(&beta, "Hallo\r\n\n").expect("später");

        let Gesendet::Neu(ursprung) = erste else {
            panic!("erste nicht neu: {erste:?}")
        };
        assert_eq!(ursprung.text, "Hallo");
        assert_eq!(zweite, Gesendet::Wiederholt(ursprung.clone()));
        assert_eq!(andere, Err(Sendefehler::SchluesselAnders));
        assert!(matches!(fremd, Gesendet::Neu(_)), "{fremd:?}");
        assert!(matches!(spaeter, Gesendet::Neu(_)), "{spaeter:?}");
        let inhalt = ablage.inhalt(&allgemein, tag("2026-10-27")).expect("Tag");
        assert_eq!(inhalt.matches("  Hallo\n").count(), 3, "{inhalt}");
        assert_eq!(sperren(&rueckrufe).len(), 3);
    }

    /// R00025 „Nach der Sperre“ (Regel E1.7, Angriffsmatrix Zeile 15): die Meldung einer Verdrängung kommt aus dem
    /// Senden, wenn die Schreibsperre wieder frei ist — die Senke prüft es beim Aufruf.
    #[test]
    fn eine_verdraengung_wird_erst_nach_freigabe_der_schreibsperre_gemeldet() {
        use std::sync::{OnceLock, Weak};
        let (gespraeche, _, _) = im_speicher("2026-10-27T14:07:12.345+01:00");
        let selbst: Arc<OnceLock<Weak<Gespraeche>>> = Arc::default();
        let frei: Arc<Mutex<Vec<bool>>> = Arc::default();
        let gespraeche = Arc::new(
            gespraeche
                .mit_wiederholung(wiederholung::Wiederholung::neu(640, 1).expect("Grenzen"))
                .mit_verdraengt_melden({
                    let (selbst, frei) = (Arc::clone(&selbst), Arc::clone(&frei));
                    Arc::new(move |_: &wiederholung::Verdraengt| {
                        let gespraeche = selbst.get().and_then(Weak::upgrade).expect("Gespräche");
                        sperren(&frei).push(gespraeche.schreibsperre_frei());
                    })
                }),
        );
        selbst.set(Arc::downgrade(&gespraeche)).expect("einmal");
        let allgemein = Gespraech::channel("allgemein");

        for (von, schluessel) in [
            (claude(), "k1"),
            (Kennung::bilden("beta@127.0.0.1").expect("Kennung"), "k2"),
        ] {
            gespraeche
                .senden_einmal(&allgemein, &von, Art::Agent, "x", schluessel)
                .expect("gesendet");
        }

        assert_eq!(*sperren(&frei), [true]);
        assert_eq!(gespraeche.kennzahlen().schluessel_verdraengt, 1);
    }

    /// Scheitert das Anhängen, wird der Schlüssel nicht gemerkt: die Wiederholung ist ein neuer Versuch und hängt an.
    #[test]
    fn ein_gescheitertes_anhaengen_merkt_den_schluessel_nicht() {
        let (gespraeche, ablage, _) = im_speicher("2026-10-27T14:07:12.345+01:00");
        let allgemein = Gespraech::channel("allgemein");
        ablage.fehler_nach(3);

        let gescheitert =
            gespraeche.senden_einmal(&allgemein, &claude(), Art::Agent, "Hallo", "k1");
        let wiederholt = gespraeche
            .senden_einmal(&allgemein, &claude(), Art::Agent, "Hallo", "k1")
            .expect("zweiter Versuch");

        assert!(
            matches!(gescheitert, Err(Sendefehler::Ablage { .. })),
            "{gescheitert:?}"
        );
        assert!(matches!(wiederholt, Gesendet::Neu(_)), "{wiederholt:?}");
        let inhalt = ablage.inhalt(&allgemein, tag("2026-10-27")).expect("Tag");
        assert_eq!(inhalt.matches("  Hallo\n").count(), 1, "{inhalt}");
    }

    #[test]
    fn senden_haengt_den_satz_an_den_tag_der_zeit_und_ergaenzt_erst_dann_den_index() {
        let (gespraeche, ablage, _) = im_speicher("2026-10-27T14:07:12.345+01:00");
        let allgemein = Gespraech::channel("allgemein");

        let nachricht = gespraeche
            .senden(&allgemein, &claude(), Art::Agent, "Hallo")
            .expect("gesendet");

        assert_eq!(nachricht.zeit, zeit("2026-10-27T14:07:12.345+01:00"));
        assert_eq!(nachricht.von, claude());
        assert_eq!(nachricht.art, Some(Art::Agent));
        assert_eq!(nachricht.text, "Hallo");
        let erwartet = "@ 2026-10-27T14:07:12.345+01:00 claude@10.0.0.5\n+ art: agent\n  Hallo\n\n";
        assert_eq!(
            ablage.inhalt(&allgemein, tag("2026-10-27")).as_deref(),
            Some(erwartet)
        );
        let kennzahlen = gespraeche.kennzahlen();
        assert_eq!(kennzahlen.nachrichten, 1);
        assert_eq!(kennzahlen.log_bytes, erwartet.len() as u64);
        assert_eq!(gespraeche.letzte_zeit(), Some(nachricht.zeit));
    }

    /// B0209: die Nachricht trägt die Art; ihr Text ist der Text, wie ihn der Verlauf liefert (ADR-0034 Regel 4).
    #[test]
    fn gesendet_wird_mit_art_und_der_verlauf_liefert_dieselbe_nachricht() {
        let (gespraeche, _, _) = im_speicher("2026-10-28T12:00:00.000+01:00");
        let allgemein = Gespraech::channel("allgemein");

        let mensch = gespraeche
            .senden(&allgemein, &claude(), Art::Mensch, "Hallo\r\nWelt\n\n")
            .expect("gesendet");

        assert_eq!(mensch.art, Some(Art::Mensch));
        assert_eq!(mensch.text, "Hallo\nWelt");
        let block = gespraeche
            .verlauf(
                "allgemein",
                &abfrage(abfrage::Abschnitt::Neueste, 100),
                usize::MAX,
            )
            .expect("bekannt");
        assert_eq!(block.nachrichten, vec![mensch]);
    }

    #[test]
    fn zwei_nachrichten_in_derselben_millisekunde_folgen_mit_einer_millisekunde_abstand() {
        let (gespraeche, _, _) = im_speicher("2026-10-27T14:07:12.345+01:00");
        let allgemein = Gespraech::channel("allgemein");

        let erste = gespraeche
            .senden(&allgemein, &claude(), Art::Agent, "a")
            .expect("erste");
        let zweite = gespraeche
            .senden(&allgemein, &claude(), Art::Agent, "b")
            .expect("zweite");

        assert_eq!(zweite.zeit - erste.zeit, chrono::Duration::milliseconds(1));
    }

    #[test]
    fn unbekanntes_gespraech_ist_ein_fehler_und_legt_nichts_an() {
        let (gespraeche, ablage, _) = im_speicher("2026-10-27T14:07:12.345+01:00");

        let ergebnis = gespraeche.senden(
            &Gespraech::channel("gibtsnicht"),
            &claude(),
            Art::Agent,
            "x",
        );

        assert!(
            matches!(ergebnis, Err(Sendefehler::Unbekannt)),
            "{ergebnis:?}"
        );
        assert_eq!(
            ablage.gespraeche().expect("Gespräche"),
            vec![Gespraech::channel("allgemein")]
        );
        assert_eq!(gespraeche.kennzahlen().nachrichten, 0);
        assert_eq!(gespraeche.letzte_zeit(), None);
    }

    #[test]
    fn schreibfehler_laesst_inhalt_und_index_unveraendert_und_die_zeit_verfaellt() {
        let (gespraeche, ablage, _) = im_speicher("2026-10-27T14:07:12.345+01:00");
        let allgemein = Gespraech::channel("allgemein");
        gespraeche
            .senden(&allgemein, &claude(), Art::Agent, "eins")
            .expect("erste");
        let vorher = ablage.inhalt(&allgemein, tag("2026-10-27")).expect("Tag");
        ablage.fehler_nach(20);

        let gescheitert = gespraeche.senden(&allgemein, &claude(), Art::Agent, "zwei");

        assert!(
            matches!(gescheitert, Err(Sendefehler::Ablage { ref datei, .. }) if datei == "channels/allgemein/2026-10-27.log"),
            "{gescheitert:?}"
        );
        assert_eq!(
            ablage.inhalt(&allgemein, tag("2026-10-27")),
            Some(vorher.clone())
        );
        assert_eq!(gespraeche.kennzahlen().nachrichten, 1);
        assert_eq!(gespraeche.kennzahlen().log_bytes, vorher.len() as u64);
        let dritte = gespraeche
            .senden(&allgemein, &claude(), Art::Agent, "drei")
            .expect("dritte");
        assert_eq!(dritte.zeit, zeit("2026-10-27T14:07:12.347+01:00"));
    }

    #[test]
    fn die_zeit_setzt_beim_juengsten_satz_im_ganzen_ordner_fort() {
        let ablage = Arc::new(Speicherablage::neu());
        let a = Gespraech::channel("a");
        ablage.gespraech_anlegen(&a);
        ablage.tag_setzen(
            &Gespraech::channel("b"),
            tag("2026-10-27"),
            "@ 2026-10-27T14:00:00.000+01:00 shoff@127.0.0.1\n  vorher\n\n",
        );
        let uhr = Arc::new(GestellteUhr::neu(zeit("2026-10-27T13:00:00.000+01:00")));
        let gespraeche = Gespraeche::laden(ablage, uhr).expect("geladen");

        let erste = gespraeche
            .senden(&a, &claude(), Art::Agent, "x")
            .expect("gesendet");

        assert_eq!(erste.zeit.to_rfc3339(), "2026-10-27T14:00:00.001+01:00");
    }

    #[test]
    fn die_tagesdatei_folgt_dem_datum_der_vergebenen_zeit() {
        let (gespraeche, ablage, uhr) = im_speicher("2026-10-27T23:59:59.999+01:00");
        let allgemein = Gespraech::channel("allgemein");
        gespraeche
            .senden(&allgemein, &claude(), Art::Agent, "spät")
            .expect("erste");
        uhr.stellen(zeit("2026-10-28T00:00:00.000+01:00"));

        gespraeche
            .senden(&allgemein, &claude(), Art::Agent, "früh")
            .expect("zweite");

        assert_eq!(
            ablage.tage(&allgemein).expect("Tage"),
            vec![tag("2026-10-27"), tag("2026-10-28")]
        );
        assert_eq!(gespraeche.kennzahlen().nachrichten, 2);
    }

    #[test]
    fn laedt_die_tage_in_der_frist_aller_gespraeche_mit_kennzahlen_defekten_und_juengster_zeit() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let ordner = wurzel.path();
        datei(
            ordner,
            "channels/allgemein/2026-10-26.log",
            TAG_26.as_bytes(),
        );
        datei(
            ordner,
            "channels/allgemein/2026-10-27.log",
            TAG_27.as_bytes(),
        );
        datei(
            ordner,
            "channels/allgemein/2026-10-20.log",
            TAG_26.as_bytes(),
        );
        datei(
            ordner,
            "direkt/claude@10.0.0.5+shoff@127.0.0.1/2026-10-27.log",
            DIREKT.as_bytes(),
        );

        let gespraeche = laden_um(ordner, "2026-10-27T12:00:00+01:00");

        let kennzahlen = gespraeche.kennzahlen();
        assert_eq!(kennzahlen.nachrichten, 5);
        assert_eq!(kennzahlen.log_bytes, (128 + 127 + DIREKT.len()) as u64);
        assert_eq!(kennzahlen.defekte_zeilen, 1);
        assert_eq!(
            gespraeche
                .defekte()
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            vec!["channels/allgemein/2026-10-27.log:4: defekte Zeile übersprungen"]
        );
        assert_eq!(
            gespraeche.letzte_zeit(),
            Some(zeit("2026-10-27T11:00:00+01:00"))
        );
        assert!(
            lesend(&gespraeche.verzeichnis)
                .verlaeufe
                .contains_key(&Gespraech::channel("allgemein"))
        );
    }

    /// R00039 B0596 Laden: ein Änderungssatz mit fremder Kennung ist ein Defekt mit Datei und Zeile (M13), eine
    /// Änderung, deren Nachricht entfallen ist, eine stille Waise; die gültige Änderung gilt, die Zahl der Nachrichten
    /// zählt nur Nachrichten, die Bytes alle Sätze.
    #[test]
    fn laden_bildet_fassungen_meldet_fremde_aenderungen_und_zaehlt_waisen() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let ordner = wurzel.path();
        let tag_27 = "@ 2026-10-27T10:00:00.000+01:00 beta@127.0.0.1\n+ art: agent\n  Hallo\n\n\
                      @ 2026-10-27T10:00:01.000+01:00 gamma@127.0.0.1\n+ bearbeitet: 2026-10-27T10:00:00.000+01:00\n  gefälscht\n\n";
        let tag_28 = "@ 2026-10-28T09:00:00.000+01:00 beta@127.0.0.1\n+ bearbeitet: 2026-10-27T10:00:00.000+01:00\n  Hallo zusammen\n\n\
                      @ 2026-10-28T09:00:01.000+01:00 beta@127.0.0.1\n+ bearbeitet: 2026-10-01T10:00:00.000+02:00\n  entfallen\n\n";
        datei(
            ordner,
            "channels/allgemein/2026-10-27.log",
            tag_27.as_bytes(),
        );
        datei(
            ordner,
            "channels/allgemein/2026-10-28.log",
            tag_28.as_bytes(),
        );

        let gespraeche = laden_um(ordner, "2026-10-28T12:00:00+01:00");

        assert_eq!(
            gespraeche
                .defekte()
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            vec!["channels/allgemein/2026-10-27.log:5: Änderungssatz übergangen (fremd)"]
        );
        assert_eq!(gespraeche.aenderungswaisen(), 1);
        let kennzahlen = gespraeche.kennzahlen();
        assert_eq!(kennzahlen.nachrichten, 1);
        assert_eq!(kennzahlen.log_bytes, (tag_27.len() + tag_28.len()) as u64);
        let verlauf = lesend(&gespraeche.verzeichnis)
            .verlaeufe
            .get(&Gespraech::channel("allgemein"))
            .cloned()
            .expect("allgemein");
        let fassung = sperren(&verlauf)
            .fassung(&zeit("2026-10-27T10:00:00.000+01:00"))
            .expect("bearbeitet");
        assert_eq!(fassung.geaendert, zeit("2026-10-28T09:00:00.000+01:00"));
    }

    /// R00039 B0597: der Verlauf liefert eine bearbeitete Nachricht mit dem neuen Text, `Bearbeitet` und der Zeit
    /// der Änderung; Art, Absender und Zeit bleiben die der Nachricht, der Änderungssatz ist kein Eintrag.
    #[test]
    fn der_verlauf_liefert_die_nachricht_in_ihrer_fassung() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let tag_28 = "@ 2026-10-28T10:00:00.000+01:00 beta@127.0.0.1\n+ art: agent\n  Hallo\n\n\
                      @ 2026-10-28T10:05:00.000+01:00 beta@127.0.0.1\n+ bearbeitet: 2026-10-28T10:00:00.000+01:00\n  Hallo zusammen\n\n";
        datei(
            wurzel.path(),
            "channels/allgemein/2026-10-28.log",
            tag_28.as_bytes(),
        );
        let gespraeche = laden_um(wurzel.path(), "2026-10-28T12:00:00+01:00");

        let block = gespraeche
            .verlauf(
                "allgemein",
                &abfrage(abfrage::Abschnitt::Neueste, 100),
                usize::MAX,
            )
            .expect("bekannt");

        assert_eq!(
            block.nachrichten,
            vec![Nachricht {
                zeit: zeit("2026-10-28T10:00:00.000+01:00"),
                von: Kennung::bilden("beta@127.0.0.1").expect("Kennung"),
                art: Some(Art::Agent),
                text: "Hallo zusammen".to_string(),
                anhaenge: Vec::new(),
                geaendert: Some(Geaendert {
                    zeit: zeit("2026-10-28T10:05:00.000+01:00"),
                    zustand: Aenderungszustand::Bearbeitet,
                }),
            }]
        );
    }

    fn kennung(text: &str) -> Kennung {
        Kennung::bilden(text).expect("Kennung")
    }

    fn neueste(gespraeche: &Gespraeche) -> Vec<Nachricht> {
        gespraeche
            .verlauf(
                "allgemein",
                &abfrage(abfrage::Abschnitt::Neueste, 100),
                usize::MAX,
            )
            .expect("bekannt")
            .nachrichten
    }

    fn fassung_von(gespraeche: &Gespraeche, zeit: &Zeit) -> Option<verlaufsindex::Fassung> {
        let verlauf = lesend(&gespraeche.verzeichnis)
            .verlaeufe
            .get(&Gespraech::channel("allgemein"))
            .cloned()
            .expect("allgemein");
        sperren(&verlauf).fassung(zeit)
    }

    /// R00039 B0598 „Gespräche“ (ADR-0048 erster Unit-Test ohne Löschen): `alice` sendet t1 und t2 am Tag 1; Tag 2:
    /// `alice` bearbeitet t1 zweimal, `bob` → `Fremd` und die Ablage bleibt; der Block zeigt t1 mit dem zweiten Text,
    /// `Bearbeitet` und der Zeit der zweiten Änderung, t2 ungeändert; `belegt` = Σ Bytes aller Sätze; Neuladen →
    /// derselbe Block; Entfallen von Tag 1 nimmt die Fassung mit; Neuladen ohne Tag 1 → keine Warnung, zwei Waisen;
    /// gleicher Text → kein Satz. Mutanten „Kennung nicht geprüft“ und „Änderungssatz im Block als Nachricht“ rot.
    #[test]
    fn bearbeiten_ueber_zwei_tage_mit_neuladen_und_entfallen() {
        let (gespraeche, ablage, uhr) = im_speicher("2026-10-27T14:00:00.000+01:00");
        let allgemein = Gespraech::channel("allgemein");
        let (alice, bob) = (kennung("alice@127.0.0.2"), kennung("bob@127.0.0.3"));
        let t1 = gespraeche
            .senden(&allgemein, &alice, Art::Agent, "eins")
            .expect("t1")
            .zeit;
        let t2 = gespraeche
            .senden(&allgemein, &alice, Art::Agent, "zwei")
            .expect("t2")
            .zeit;
        uhr.stellen(zeit("2026-10-28T09:00:00.000+01:00"));

        let erste = gespraeche.bearbeiten(&allgemein, &alice, t1, "eins, korrigiert");
        uhr.stellen(zeit("2026-10-28T09:00:05.000+01:00"));
        let zweite = gespraeche.bearbeiten(&allgemein, &alice, t1, "eins, noch einmal\r\n");
        let inhalt = |tag_text| ablage.inhalt(&allgemein, tag(tag_text)).unwrap_or_default();
        let vorher = (inhalt("2026-10-27"), inhalt("2026-10-28"));
        let fremd = gespraeche.bearbeiten(&allgemein, &bob, t1, "von bob");
        let gleich = gespraeche.bearbeiten(&allgemein, &alice, t1, "eins, noch einmal");

        assert!(matches!(erste, Ok(Bearbeitet::Neu(_))), "{erste:?}");
        let Ok(Bearbeitet::Neu(neu)) = zweite else {
            panic!("zweite: {zweite:?}")
        };
        let geaendert = Geaendert {
            zeit: zeit("2026-10-28T09:00:05.000+01:00"),
            zustand: Aenderungszustand::Bearbeitet,
        };
        assert_eq!(neu.text, "eins, noch einmal");
        assert_eq!(neu.geaendert, Some(geaendert));
        assert_eq!(
            (neu.zeit, &neu.von, neu.art),
            (t1, &alice, Some(Art::Agent))
        );
        assert_eq!(
            fremd,
            Err(Bearbeitfehler::Fremd {
                absender: alice.clone()
            })
        );
        assert_eq!(gleich, Ok(Bearbeitet::Unveraendert(neu.clone())));
        assert_eq!((inhalt("2026-10-27"), inhalt("2026-10-28")), vorher);
        assert!(
            inhalt("2026-10-28")
                .contains("+ bearbeitet: 2026-10-27T14:00:00.000+01:00\n  eins, noch einmal\n")
        );
        let block = neueste(&gespraeche);
        assert_eq!(block.len(), 2);
        assert_eq!(block[0], neu);
        assert_eq!((block[1].zeit, block[1].geaendert), (t2, None));
        assert_eq!(
            gespraeche.belegt(),
            (vorher.0.len() + vorher.1.len()) as u64
        );
        assert_eq!(gespraeche.kennzahlen().nachrichten, 2);

        let neu_geladen = Gespraeche::laden(ablage.clone(), uhr.clone()).expect("geladen");
        assert_eq!(neueste(&neu_geladen), block);
        assert_eq!(neu_geladen.belegt(), gespraeche.belegt());

        gespraeche.pflegen(tag("2026-11-03"), Pflegeanlass::Tageswechsel);
        assert!(neueste(&gespraeche).is_empty());
        assert_eq!(fassung_von(&gespraeche, &t1), None);
        let ohne_tag_1 = Gespraeche::laden(ablage.clone(), uhr.clone()).expect("geladen");
        assert!(
            ohne_tag_1.defekte().is_empty(),
            "{:?}",
            ohne_tag_1.defekte()
        );
        assert_eq!(ohne_tag_1.aenderungswaisen(), 2);
        assert_eq!(ohne_tag_1.kennzahlen().nachrichten, 0);
    }

    /// B0598 Abweisungen unter der Sperre: Zeit ohne Nachricht, dieselbe Zeit in einem anderen Channel und ein
    /// unbekanntes Gespräch → `Nachricht`/`Unbekannt`; gelöschte Nachricht → `Geloescht`; nur Leerraum ohne Anhang →
    /// `Leer`; jedes Mal nichts geschrieben.
    #[test]
    fn bearbeiten_weist_ab_ohne_zu_schreiben() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let t1 = "2026-10-28T10:00:00.000+01:00";
        let t2 = "2026-10-28T10:00:01.000+01:00";
        let tag_28 = format!(
            "@ {t1} beta@127.0.0.1\n  Hallo\n\n@ {t2} beta@127.0.0.1\n  weg\n\n\
             @ 2026-10-28T10:00:02.000+01:00 beta@127.0.0.1\n+ geloescht: {t2}\n\n"
        );
        datei(
            wurzel.path(),
            "channels/allgemein/2026-10-28.log",
            tag_28.as_bytes(),
        );
        fs::create_dir_all(wurzel.path().join("channels/technik")).expect("technik");
        let gespraeche = laden_um(wurzel.path(), "2026-10-28T12:00:00+01:00");
        let (allgemein, technik) = (
            Gespraech::channel("allgemein"),
            Gespraech::channel("technik"),
        );
        let beta = kennung("beta@127.0.0.1");
        let bearbeiten = |gespraech: &Gespraech, ziel: &str, text: &str| {
            gespraeche.bearbeiten(gespraech, &beta, zeit(ziel), text)
        };

        assert_eq!(
            bearbeiten(&allgemein, "2026-10-28T10:00:00.001+01:00", "x"),
            Err(Bearbeitfehler::Nachricht)
        );
        assert_eq!(
            bearbeiten(&technik, t1, "x"),
            Err(Bearbeitfehler::Nachricht)
        );
        assert_eq!(
            bearbeiten(&Gespraech::channel("gibtsnicht"), t1, "x"),
            Err(Bearbeitfehler::Unbekannt)
        );
        assert_eq!(
            bearbeiten(&allgemein, t2, "x"),
            Err(Bearbeitfehler::Geloescht)
        );
        assert_eq!(
            bearbeiten(&allgemein, t1, " \n\t"),
            Err(Bearbeitfehler::Leer)
        );
        assert_eq!(
            fs::read(wurzel.path().join("channels/allgemein/2026-10-28.log")).expect("Datei"),
            tag_28.as_bytes()
        );
        assert!(
            !wurzel
                .path()
                .join("channels/technik/2026-10-28.log")
                .exists()
        );
        let ok = bearbeiten(&allgemein, "2026-10-28T09:00:00.000Z", "Hallo zusammen");
        assert!(
            matches!(ok, Ok(Bearbeitet::Neu(_))),
            "dieselbe Zeit in UTC: {ok:?}"
        );
    }

    /// B0598 Schreibfehler (R00039 „Schreibfehler“): Anhängen scheitert → `Ablage`, die Ablage byte-gleich, Index und
    /// Fassung unverändert, `belegt` unverändert.
    #[test]
    fn ein_schreibfehler_beim_bearbeiten_aendert_nichts() {
        let (gespraeche, ablage, _) = im_speicher("2026-10-27T14:00:00.000+01:00");
        let allgemein = Gespraech::channel("allgemein");
        let t1 = gespraeche
            .senden(&allgemein, &claude(), Art::Agent, "Hallo")
            .expect("t1")
            .zeit;
        let vorher = ablage.inhalt(&allgemein, tag("2026-10-27"));
        let belegt = gespraeche.belegt();
        ablage.fehler_nach(20);

        let gescheitert = gespraeche.bearbeiten(&allgemein, &claude(), t1, "Hallo zusammen");

        assert!(
            matches!(gescheitert, Err(Bearbeitfehler::Ablage { ref datei, .. }) if datei == "channels/allgemein/2026-10-27.log"),
            "{gescheitert:?}"
        );
        assert_eq!(ablage.inhalt(&allgemein, tag("2026-10-27")), vorher);
        assert_eq!(gespraeche.belegt(), belegt);
        assert_eq!(fassung_von(&gespraeche, &t1), None);
        assert_eq!(neueste(&gespraeche)[0].text, "Hallo");
    }

    /// R00039 B0600 (M11): `k1` zweimal → einmal angehängt, die zweite `Wiederholt` mit derselben Fassung; `k1` mit
    /// anderem Text → `SchluesselAnders`; ein Schlüssel des Sendens → `SchluesselAnders`; derselbe `k1` einer anderen
    /// Kennung an ihrer eigenen Nachricht → neu.
    #[test]
    fn ein_wiederholschluessel_beim_bearbeiten_haengt_einmal_an() {
        let (gespraeche, ablage, uhr) = im_speicher("2026-10-27T14:00:00.000+01:00");
        let allgemein = Gespraech::channel("allgemein");
        let (beta, gamma) = (kennung("beta@127.0.0.1"), kennung("gamma@127.0.0.1"));
        let t1 = gespraeche
            .senden(&allgemein, &beta, Art::Agent, "Hallo")
            .expect("t1")
            .zeit;
        let t_gamma = gespraeche
            .senden(&allgemein, &gamma, Art::Agent, "Servus")
            .expect("gamma")
            .zeit;
        gespraeche
            .senden_einmal(&allgemein, &beta, Art::Agent, "Hallo zusammen", "k2")
            .expect("k2 beim Senden");
        let bearbeiten = |von: &Kennung, ziel: Zeit, text: &str, schluessel: &str| {
            gespraeche.bearbeiten_mit(&allgemein, von, ziel, text, Some(schluessel))
        };

        let erste = bearbeiten(&beta, t1, "Hallo zusammen", "k1");
        uhr.stellen(zeit("2026-10-27T14:00:05.000+01:00"));
        let zweite = bearbeiten(&beta, t1, "Hallo zusammen", "k1");
        let anders = bearbeiten(&beta, t1, "Hallo allerseits", "k1");
        let vom_senden = bearbeiten(&beta, t1, "Hallo zusammen", "k2");
        let fremde_kennung = bearbeiten(&gamma, t_gamma, "Servus zusammen", "k1");

        let Ok(Bearbeitet::Neu(neu)) = erste else {
            panic!("erste: {erste:?}")
        };
        assert_eq!(zweite, Ok(Bearbeitet::Wiederholt(neu)));
        assert_eq!(anders, Err(Bearbeitfehler::SchluesselAnders));
        assert_eq!(vom_senden, Err(Bearbeitfehler::SchluesselAnders));
        assert!(
            matches!(fremde_kennung, Ok(Bearbeitet::Neu(_))),
            "{fremde_kennung:?}"
        );
        let inhalt = ablage.inhalt(&allgemein, tag("2026-10-27")).expect("Tag");
        assert_eq!(inhalt.matches("+ bearbeitet: ").count(), 2, "{inhalt}");
    }

    #[test]
    fn juengste_zeit_wird_als_zeitpunkt_verglichen_nicht_als_zeichenkette() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        datei(
            wurzel.path(),
            "channels/a/2026-10-25.log",
            b"@ 2026-10-25T02:59:59.999+02:00 a@b\n  vor\n\n@ 2026-10-25T02:00:00.000+01:00 a@b\n  nach\n\n",
        );

        datei(
            wurzel.path(),
            "channels/b/2026-10-25.log",
            b"@ 2026-10-25T01:30:00.000+01:00 a@b\n  frueher, aber zuletzt geladen\n\n",
        );

        let gespraeche = laden_um(wurzel.path(), "2026-10-25T12:00:00+01:00");

        assert_eq!(
            gespraeche.letzte_zeit(),
            Some(zeit("2026-10-25T02:00:00.000+01:00"))
        );
    }

    #[test]
    fn heute_ist_das_datum_der_uhr_samt_versatz() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let ordner = wurzel.path();
        datei(ordner, "channels/a/2026-10-21.log", TAG_26.as_bytes());
        datei(ordner, "channels/a/2026-10-22.log", TAG_26.as_bytes());

        assert_eq!(
            laden_um(ordner, "2026-10-27T12:00:00+01:00")
                .kennzahlen()
                .nachrichten,
            4
        );
        assert_eq!(
            laden_um(ordner, "2026-10-28T00:30:00+01:00")
                .kennzahlen()
                .nachrichten,
            2
        );
    }

    /// Nicht lesbar heißt: das Dateisystem verweigert die Datei (hier per Rechte); ungültiges UTF-8 ist
    /// kein Lesefehler, sondern ein Defekt seiner Zeile (Code-Review B1).
    #[cfg(unix)]
    #[test]
    fn nicht_lesbare_tagesdatei_ist_ein_defekt_und_der_rest_wird_geladen() {
        use std::os::unix::fs::PermissionsExt;
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let ordner = wurzel.path();
        datei(ordner, "channels/a/2026-10-26.log", TAG_26.as_bytes());
        let gesperrt = ordner.join("channels/a/2026-10-26.log");
        fs::set_permissions(&gesperrt, fs::Permissions::from_mode(0o000)).expect("Rechte");
        datei(ordner, "channels/a/2026-10-27.log", DIREKT.as_bytes());

        let gespraeche = laden_um(ordner, "2026-10-27T12:00:00+01:00");
        fs::set_permissions(&gesperrt, fs::Permissions::from_mode(0o644)).expect("Rechte");

        assert_eq!(gespraeche.kennzahlen().nachrichten, 1);
        assert_eq!(gespraeche.kennzahlen().defekte_zeilen, 1);
        assert_eq!(gespraeche.defekte()[0].datei, "channels/a/2026-10-26.log");
        assert!(matches!(
            gespraeche.defekte()[0].art,
            Defektart::Unlesbar(_)
        ));
    }

    #[test]
    fn ungueltiges_utf8_ist_ein_zeilendefekt_der_tag_bleibt_mit_rohbytes_und_juengster_zeit() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let rohtext: &[u8] = b"@ 2026-10-27T10:00:00.000+01:00 a@b\n  eins\n\n@ 2026-10-27T10:00:06.000+01:00 c@d\n  Scan l\xC3";
        datei(wurzel.path(), "channels/a/2026-10-27.log", rohtext);

        let gespraeche = laden_um(wurzel.path(), "2026-10-27T12:00:00+01:00");

        let kennzahlen = gespraeche.kennzahlen();
        assert_eq!(kennzahlen.nachrichten, 2);
        assert_eq!(kennzahlen.log_bytes, rohtext.len() as u64);
        assert_eq!(
            gespraeche.defekte(),
            &[Ladedefekt {
                datei: "channels/a/2026-10-27.log".into(),
                art: Defektart::Zeile(5),
            }]
        );
        assert_eq!(
            gespraeche.letzte_zeit(),
            Some(zeit("2026-10-27T10:00:06+01:00"))
        );
    }

    #[test]
    fn leerer_ordner_hat_kennzahlen_null_ausser_der_ladezeit() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");

        let gespraeche = laden_um(wurzel.path(), "2026-10-27T12:00:00+01:00");

        let kennzahlen = gespraeche.kennzahlen();
        assert_eq!(
            (
                kennzahlen.nachrichten,
                kennzahlen.log_bytes,
                kennzahlen.defekte_zeilen
            ),
            (0, 0, 0)
        );
        assert!(gespraeche.defekte().is_empty());
        assert_eq!(gespraeche.letzte_zeit(), None);
    }

    fn alpha() -> Kennung {
        Kennung::bilden("alpha@127.0.0.2").expect("Kennung")
    }

    fn leer_im_speicher(jetzt: &str) -> (Gespraeche, Arc<Speicherablage>) {
        let ablage = Arc::new(Speicherablage::neu());
        let uhr = Arc::new(GestellteUhr::neu(zeit(jetzt)));
        let gespraeche = Gespraeche::laden(ablage.clone(), uhr).expect("geladen");
        (gespraeche, ablage)
    }

    #[test]
    fn anlegen_traegt_serverzeit_und_anleger_ein_und_zaehlt_den_stand() {
        let (gespraeche, ablage) = leer_im_speicher("2026-10-28T10:15:00.123456+01:00");

        let channel = gespraeche
            .channel_anlegen("allgemein", None, &alpha())
            .expect("angelegt");

        assert_eq!(channel.name.als_text(), "allgemein");
        assert_eq!(
            channel.angelegt,
            Some(zeit("2026-10-28T10:15:00.123+01:00"))
        );
        assert_eq!(channel.angelegt_von, Some(alpha()));
        assert_eq!(
            gespraeche.channels(),
            ChannelStand {
                stand: 1,
                liste: vec![channel.clone()],
            }
        );
        let allgemein = Gespraech::channel("allgemein");
        let datei = ablage
            .einstellungen_lesen(&allgemein)
            .expect("lesbar")
            .expect("Einstellungsdatei");
        assert_eq!(
            einstellungen::lesen(&datei),
            Ok(einstellungen::Einstellungen {
                angelegt: channel.angelegt,
                angelegt_von: channel.angelegt_von.clone(),
                beschreibung: None,
                frist_tage: None,
            })
        );
        gespraeche
            .senden(&allgemein, &claude(), Art::Agent, "erste Nachricht")
            .expect("im neuen Channel gesendet");
    }

    #[test]
    fn derselbe_name_noch_einmal_ist_vorhanden_ohne_schreibvorgang_und_ohne_neuen_stand() {
        let (gespraeche, ablage) = leer_im_speicher("2026-10-28T10:15:00.123+01:00");
        let erster = gespraeche
            .channel_anlegen("allgemein", None, &alpha())
            .expect("angelegt");
        let geschrieben = ablage.schreibvorgaenge();

        let zweiter = gespraeche.channel_anlegen("allgemein", None, &claude());

        assert_eq!(zweiter, Err(Anlegefehler::Vorhanden(Box::new(erster))));
        assert_eq!(ablage.schreibvorgaenge(), geschrieben);
        assert_eq!(gespraeche.channels().stand, 1);
    }

    #[test]
    fn ungueltiger_name_ist_ein_regelverstoss_und_legt_nichts_an() {
        let (gespraeche, ablage) = leer_im_speicher("2026-10-28T10:15:00.123+01:00");

        let ergebnis = gespraeche.channel_anlegen("Allgemein", None, &alpha());

        let Err(Anlegefehler::Regel(fehler)) = ergebnis else {
            panic!("{ergebnis:?}");
        };
        assert_eq!(
            fehler.vorschlag.map(|name| name.to_string()),
            Some("allgemein".to_string())
        );
        assert_eq!(ablage.schreibvorgaenge(), 0);
        assert_eq!(gespraeche.channels().stand, 0);
    }

    #[test]
    fn die_liste_steht_nach_name_nicht_nach_anlage() {
        let (gespraeche, _) = leer_im_speicher("2026-10-28T10:15:00.123+01:00");
        for name in ["security-entwicklung", "allgemein", "b"] {
            gespraeche
                .channel_anlegen(name, None, &alpha())
                .expect("angelegt");
        }

        let stand = gespraeche.channels();

        assert_eq!(stand.stand, 3);
        assert_eq!(
            stand
                .liste
                .iter()
                .map(|channel| channel.name.to_string())
                .collect::<Vec<_>>(),
            vec!["allgemein", "b", "security-entwicklung"]
        );
    }

    #[test]
    fn laden_liest_die_einstellungen_je_channel_und_beginnt_bei_stand_null() {
        let ablage = Arc::new(Speicherablage::neu());
        let uhr = Arc::new(GestellteUhr::neu(zeit("2026-10-28T10:15:00.123+01:00")));
        let vorher = Gespraeche::laden(ablage.clone(), uhr.clone()).expect("geladen");
        let angelegt = vorher
            .channel_anlegen("allgemein", None, &alpha())
            .expect("angelegt");
        ablage.gespraech_anlegen(&Gespraech::channel("notizen"));

        let nachher = Gespraeche::laden(ablage, uhr).expect("neu geladen");

        assert_eq!(
            nachher.channels(),
            ChannelStand {
                stand: 0,
                liste: vec![
                    angelegt,
                    Channel {
                        name: Channelname::bilden("notizen").expect("Name"),
                        angelegt: None,
                        angelegt_von: None,
                        beschreibung: None,
                        frist: Frist::Tage(STANDARD_FRIST_TAGE),
                    },
                ],
            }
        );
        assert!(nachher.defekte().is_empty());
    }

    /// Je Wechsel: Nummer und ob das Verzeichnis beim Veröffentlichen gesperrt war.
    type Aufzeichnung = Arc<Mutex<Vec<(u64, bool)>>>;

    /// Gespräche, deren Veröffentlichung Nummer und Sperrzustand jedes Wechsels aufzeichnet.
    fn mit_aufzeichnung() -> (Arc<Gespraeche>, Aufzeichnung) {
        let selbst: Arc<std::sync::OnceLock<std::sync::Weak<Gespraeche>>> = Arc::default();
        let aufgezeichnet: Aufzeichnung = Arc::default();
        let rueckruf: ChannelVeroeffentlichen = {
            let selbst = Arc::clone(&selbst);
            let aufgezeichnet = Arc::clone(&aufgezeichnet);
            Arc::new(move |wechsel: &ChannelWechsel| {
                let gespraeche = selbst
                    .get()
                    .and_then(std::sync::Weak::upgrade)
                    .expect("Gespräche");
                let gesperrt = gespraeche.verzeichnis.try_read().is_err();
                sperren(&aufgezeichnet).push((wechsel.stand, gesperrt));
            })
        };
        let (gespraeche, _) = leer_im_speicher("2026-10-28T10:15:00.123+01:00");
        let gespraeche = Arc::new(gespraeche.mit_veroeffentlichen(rueckruf));
        selbst
            .set(Arc::downgrade(&gespraeche))
            .expect("einmal gesetzt");
        (gespraeche, aufgezeichnet)
    }

    #[test]
    fn jeder_neue_channel_geht_unter_der_sperre_mit_seinem_stand_an_die_veroeffentlichung() {
        let (gespraeche, aufgezeichnet) = mit_aufzeichnung();

        for name in ["c", "a", "b"] {
            gespraeche
                .channel_anlegen(name, None, &alpha())
                .expect("angelegt");
        }
        let _ = gespraeche.channel_anlegen("a", None, &alpha());
        let _ = gespraeche.channel_anlegen("Ungueltig", None, &alpha());

        assert_eq!(
            *sperren(&aufgezeichnet),
            vec![(1, true), (2, true), (3, true)]
        );
        let stand = gespraeche.channels();
        assert_eq!(stand.stand, 3);
        assert_eq!(
            stand
                .liste
                .iter()
                .map(|channel| channel.name.to_string())
                .collect::<Vec<_>>(),
            vec!["a", "b", "c"]
        );
    }

    fn von(herkunft: &str) -> Kennung {
        Kennung::bilden(&format!("a@{herkunft}")).expect("Kennung")
    }

    /// Legt `anzahl` Channels `<vorsilbe><n>` von `herkunft` an; jeder muss gelingen.
    fn viele_anlegen(gespraeche: &Gespraeche, herkunft: &str, anzahl: usize, vorsilbe: &str) {
        for nummer in 0..anzahl {
            gespraeche
                .channel_anlegen(&format!("{vorsilbe}{nummer}"), None, &von(herkunft))
                .unwrap_or_else(|fehler| panic!("{vorsilbe}{nummer} von {herkunft}: {fehler:?}"));
        }
    }

    const HERKUNFT_128: Anlegefehler = Anlegefehler::Grenze(Grenze {
        art: Grenzart::Herkunft,
        hoechstens: 128,
    });
    const GESAMT_256: Anlegefehler = Anlegefehler::Grenze(Grenze {
        art: Grenzart::Gesamt,
        hoechstens: 256,
    });

    /// R00015 G3 (ändert R00006 von 32 auf 128).
    #[test]
    fn je_herkunft_gehen_128_die_129_nicht_auch_nach_erneutem_laden() {
        let ablage = Arc::new(Speicherablage::neu());
        let uhr = Arc::new(GestellteUhr::neu(zeit("2026-10-28T10:15:00.123+01:00")));
        let gespraeche = Gespraeche::laden(ablage.clone(), uhr.clone()).expect("geladen");
        viele_anlegen(&gespraeche, "10.0.0.1", 128, "c");
        let geschrieben = ablage.schreibvorgaenge();

        let hundertneunundzwanzigste = gespraeche.channel_anlegen("c128", None, &von("10.0.0.1"));

        assert_eq!(hundertneunundzwanzigste, Err(HERKUNFT_128));
        assert_eq!(ablage.schreibvorgaenge(), geschrieben);
        assert_eq!(gespraeche.channels().stand, 128);
        assert!(
            gespraeche
                .channel_anlegen("d0", None, &von("10.0.0.2"))
                .is_ok()
        );
        let neu_geladen = Gespraeche::laden(ablage, uhr).expect("neu geladen");
        assert_eq!(
            neu_geladen.channel_anlegen("c128", None, &von("10.0.0.1")),
            Err(HERKUNFT_128)
        );
    }

    /// R00015 G3: 2 Herkünfte × 128 = 256.
    #[test]
    fn gesamt_gehen_256_der_257_nicht_ein_vorhandener_name_bleibt_vorhanden() {
        let (gespraeche, _) = leer_im_speicher("2026-10-28T10:15:00.123+01:00");
        for nummer in 1..=2 {
            viele_anlegen(
                &gespraeche,
                &format!("10.0.0.{nummer}"),
                128,
                &format!("h{nummer}-"),
            );
        }

        let zweihundertsiebenundfuenfzigste =
            gespraeche.channel_anlegen("neu", None, &von("10.0.0.9"));
        let vorhanden = gespraeche.channel_anlegen("h1-0", None, &von("10.0.0.9"));

        assert_eq!(zweihundertsiebenundfuenfzigste, Err(GESAMT_256));
        assert!(
            matches!(vorhanden, Err(Anlegefehler::Vorhanden(_))),
            "{vorhanden:?}"
        );
        assert_eq!(gespraeche.channels().liste.len(), 256);
    }

    #[test]
    fn channels_ohne_anleger_zaehlen_nur_gesamt() {
        let ablage = Arc::new(Speicherablage::neu());
        for nummer in 0..10 {
            ablage.gespraech_anlegen(&Gespraech::channel(&format!("hand{nummer}")));
        }
        let uhr = Arc::new(GestellteUhr::neu(zeit("2026-10-28T10:15:00.123+01:00")));
        let gespraeche = Gespraeche::laden(ablage, uhr).expect("geladen");

        viele_anlegen(&gespraeche, "10.0.0.1", 128, "c");
        viele_anlegen(&gespraeche, "10.0.0.2", 118, "h2-");

        assert_eq!(
            gespraeche.channel_anlegen("zu-viel", None, &von("10.0.0.9")),
            Err(GESAMT_256)
        );
    }

    #[test]
    fn ordner_mit_ungueltigem_namen_sind_kein_channel_mit_defekt_punktordner_still() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let ordner = wurzel.path();
        datei(
            ordner,
            "channels/allgemein/2026-10-27.log",
            TAG_27.as_bytes(),
        );
        datei(ordner, "channels/Test/2026-10-27.log", TAG_27.as_bytes());
        datei(ordner, "channels/.neu-x/zettel.txt", b"hallo");

        let gespraeche = laden_um(ordner, "2026-10-27T12:00:00+01:00");

        assert_eq!(
            gespraeche
                .channels()
                .liste
                .iter()
                .map(|channel| channel.name.to_string())
                .collect::<Vec<_>>(),
            vec!["allgemein"]
        );
        let defekte: Vec<String> = gespraeche
            .defekte()
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(
            defekte,
            vec![
                "channels/Test: kein gültiger Channelname, übergangen",
                "channels/allgemein/2026-10-27.log:4: defekte Zeile übersprungen",
            ]
        );
        assert_eq!(gespraeche.kennzahlen().nachrichten, 2);
        assert!(ordner.join("channels/Test/2026-10-27.log").is_file());
        assert!(ordner.join("channels/.neu-x/zettel.txt").is_file());
    }

    #[test]
    fn defekte_einstellungen_ergeben_unbekannte_werte_und_einen_defekt_ohne_zu_schreiben() {
        let ablage = Arc::new(Speicherablage::neu());
        ablage.einstellungen_setzen(&Gespraech::channel("kaputt"), b"angelegt = \"");
        ablage.einstellungen_setzen(&Gespraech::channel("bunt"), b"farbe = \"rot\"\n");
        let uhr = Arc::new(GestellteUhr::neu(zeit("2026-10-28T10:15:00.123+01:00")));

        let gespraeche = Gespraeche::laden(ablage.clone(), uhr).expect("geladen");

        let liste = gespraeche.channels().liste;
        assert_eq!(liste.len(), 2);
        assert!(
            liste
                .iter()
                .all(|channel| channel.angelegt.is_none() && channel.angelegt_von.is_none()),
            "{liste:?}"
        );
        let defekte: Vec<String> = gespraeche
            .defekte()
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(defekte.len(), 1, "{defekte:?}");
        assert!(
            defekte[0].starts_with("channels/kaputt/einstellungen.toml:1: Einstellungen nicht lesbar, als unbekannt gelesen ("),
            "{defekte:?}"
        );
        assert_eq!(gespraeche.kennzahlen().defekte_zeilen, 1);
        assert_eq!(ablage.schreibvorgaenge(), 0);
        assert_eq!(
            ablage
                .einstellungen_lesen(&Gespraech::channel("kaputt"))
                .expect("lesbar"),
            Some(b"angelegt = \"".to_vec())
        );
    }

    #[cfg(unix)]
    #[test]
    fn eine_verweigerte_einstellungsdatei_ist_ein_defekt_und_der_channel_bleibt() {
        use std::os::unix::fs::PermissionsExt;
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        datei(
            wurzel.path(),
            "channels/zu/einstellungen.toml",
            b"angelegt_von = \"a@1\"\n",
        );
        let gesperrt = wurzel.path().join("channels/zu/einstellungen.toml");
        fs::set_permissions(&gesperrt, fs::Permissions::from_mode(0o000)).expect("Rechte");

        let gespraeche = laden_um(wurzel.path(), "2026-10-27T12:00:00+01:00");
        fs::set_permissions(&gesperrt, fs::Permissions::from_mode(0o644)).expect("Rechte");

        assert_eq!(gespraeche.channels().liste.len(), 1);
        let defekte: Vec<String> = gespraeche
            .defekte()
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(defekte.len(), 1, "{defekte:?}");
        assert!(
            defekte[0].starts_with(
                "channels/zu/einstellungen.toml: nicht lesbar, als unbekannt gelesen ("
            ),
            "{defekte:?}"
        );
    }

    #[test]
    fn ein_von_hand_angelegter_nicht_geladener_ordner_ist_ordner_vorhanden() {
        let (gespraeche, ablage) = leer_im_speicher("2026-10-28T10:15:00.123+01:00");
        let notizen = Gespraech::channel("notizen");
        ablage.tag_setzen(&notizen, tag("2026-10-28"), "von Hand\n");

        let ergebnis = gespraeche.channel_anlegen("notizen", None, &alpha());

        assert_eq!(ergebnis, Err(Anlegefehler::OrdnerVorhanden));
        assert_eq!(gespraeche.channels().stand, 0);
        assert_eq!(ablage.einstellungen_lesen(&notizen).expect("lesbar"), None);
        assert_eq!(
            ablage.inhalt(&notizen, tag("2026-10-28")).as_deref(),
            Some("von Hand\n")
        );
    }

    #[test]
    fn scheitern_die_einstellungen_wird_der_ordner_zurueckgebaut_ohne_stand_und_ohne_rueckruf() {
        let ablage = Arc::new(Speicherablage::neu());
        let uhr = Arc::new(GestellteUhr::neu(zeit("2026-10-28T10:15:00.123+01:00")));
        let veroeffentlicht: Arc<Mutex<Vec<u64>>> = Arc::default();
        let gespraeche = Gespraeche::laden(ablage.clone(), uhr)
            .expect("geladen")
            .mit_veroeffentlichen({
                let veroeffentlicht = Arc::clone(&veroeffentlicht);
                Arc::new(move |wechsel: &ChannelWechsel| {
                    sperren(&veroeffentlicht).push(wechsel.stand);
                })
            });
        ablage.einstellungen_scheitern();

        let gescheitert = gespraeche.channel_anlegen("neu", None, &alpha());

        let Err(Anlegefehler::Ablage(grund)) = gescheitert else {
            panic!("{gescheitert:?}");
        };
        assert!(grund.contains("channels/neu"), "{grund}");
        assert!(ablage.gespraeche().expect("Gespräche").is_empty());
        assert_eq!(gespraeche.channels().stand, 0);
        assert!(sperren(&veroeffentlicht).is_empty());
        let danach = gespraeche.channel_anlegen("neu", None, &alpha());
        assert!(danach.is_ok(), "{danach:?}");
        assert_eq!(*sperren(&veroeffentlicht), vec![1]);
    }
    fn abfrage(abschnitt: abfrage::Abschnitt, n: usize) -> abfrage::Abfrage {
        abfrage::Abfrage {
            abschnitt,
            n: abfrage::Blockgroesse::bilden(n).expect("Blockgröße"),
        }
    }

    /// 250 Nachrichten in `allgemein`, 150 am 27. ab 23:00 im Sekundentakt und 100 am 28. — Text „n<nummer>“.
    fn zweihundertfuenfzig_ueber_mitternacht() -> (Gespraeche, Vec<Nachricht>) {
        let (gespraeche, _, uhr) = im_speicher("2026-10-27T23:00:00.000+01:00");
        let allgemein = Gespraech::channel("allgemein");
        let mut gesendet = Vec::new();
        for nummer in 0..250 {
            let sekunden = if nummer < 150 { nummer } else { 3600 + nummer };
            uhr.stellen(
                zeit("2026-10-27T23:00:00.000+01:00") + chrono::Duration::seconds(sekunden),
            );
            gesendet.push(
                gespraeche
                    .senden(
                        &allgemein,
                        &claude(),
                        Art::Agent,
                        &format!("n{nummer}\nzweite Zeile"),
                    )
                    .expect("gesendet"),
            );
        }
        (gespraeche, gesendet)
    }

    #[test]
    fn verlauf_liefert_die_neuesten_als_nachrichten_mit_zeit_absender_und_text() {
        let (gespraeche, gesendet) = zweihundertfuenfzig_ueber_mitternacht();

        let block = gespraeche
            .verlauf(
                "allgemein",
                &abfrage(abfrage::Abschnitt::Neueste, 100),
                usize::MAX,
            )
            .expect("bekannt");

        assert_eq!(block.nachrichten, gesendet[150..].to_vec());
        assert_eq!(block.nachrichten[0].text, "n150\nzweite Zeile");
        assert!(block.aelter);
        assert!(!block.neuer);
    }

    #[test]
    fn verlauf_vor_der_ersten_reicht_ueber_mitternacht_und_nach_der_letzten_ist_leer() {
        let (gespraeche, gesendet) = zweihundertfuenfzig_ueber_mitternacht();

        let davor = gespraeche
            .verlauf(
                "allgemein",
                &abfrage(abfrage::Abschnitt::Vor(gesendet[150].zeit), 100),
                usize::MAX,
            )
            .expect("bekannt");
        let danach = gespraeche
            .verlauf(
                "allgemein",
                &abfrage(abfrage::Abschnitt::Nach(gesendet[249].zeit), 100),
                usize::MAX,
            )
            .expect("bekannt");

        assert_eq!(davor.nachrichten, gesendet[50..150].to_vec());
        assert!(davor.aelter && davor.neuer);
        assert!(danach.nachrichten.is_empty());
        assert!(danach.aelter && !danach.neuer);
    }

    #[test]
    fn verlauf_eines_unbekannten_oder_ungueltigen_namens_ist_unbekannt() {
        let (gespraeche, _, _) = im_speicher("2026-10-27T23:00:00.000+01:00");

        for name in ["fehlt", "Allgemein", "../allgemein", ""] {
            assert_eq!(
                gespraeche.verlauf(name, &abfrage(abfrage::Abschnitt::Neueste, 100), usize::MAX),
                Err(Unbekannt),
                "{name}"
            );
        }
    }

    /// `anzahl` Sätze am Tag `tag` (09:00 im Minutentakt, Versatz +02:00 wie im Oktober), Text „n<nummer>“.
    fn saetze(tag: &str, anzahl: usize) -> String {
        (0..anzahl)
            .map(|nummer| {
                format!("@ {tag}T09:{nummer:02}:00.000+02:00 a@127.0.0.9\n  n{nummer}\n\n")
            })
            .collect()
    }

    fn frist_von(gespraeche: &Gespraeche, name: &str) -> Frist {
        gespraeche
            .channels()
            .liste
            .into_iter()
            .find(|channel| channel.name.als_text() == name)
            .map(|channel| channel.frist)
            .expect("Channel")
    }

    /// R00008 F0033 „Laden mit der Frist des Channels“: 28.10. − 29 Tage = 29.09.; defekt lädt mit 7.
    #[test]
    fn laden_nimmt_die_tage_in_der_frist_des_channels_und_defekt_laedt_mit_sieben() {
        let ablage = Arc::new(Speicherablage::neu());
        let allgemein = Gespraech::channel("allgemein");
        ablage.einstellungen_setzen(&allgemein, b"farbe = \"blau\"\nfrist_tage = 30\n");
        ablage.tag_setzen(&allgemein, tag("2026-09-28"), &saetze("2026-09-28", 3));
        ablage.tag_setzen(&allgemein, tag("2026-09-29"), &saetze("2026-09-29", 2));
        ablage.tag_setzen(&allgemein, tag("2026-10-28"), &saetze("2026-10-28", 1));
        let kaputt = Gespraech::channel("kaputt");
        ablage.einstellungen_setzen(&kaputt, b"frist_tage = \"sieben\"\n");
        ablage.tag_setzen(&kaputt, tag("2026-10-21"), &saetze("2026-10-21", 4));
        ablage.tag_setzen(&kaputt, tag("2026-10-22"), &saetze("2026-10-22", 1));
        ablage.tag_setzen(&Gespraech::channel("ohne"), tag("2026-10-28"), "");
        let uhr = Arc::new(GestellteUhr::neu(zeit("2026-10-28T12:00:00.000+01:00")));

        let gespraeche = Gespraeche::laden(ablage, uhr).expect("geladen");

        assert_eq!(gespraeche.kennzahlen().nachrichten, 3 + 1);
        assert_eq!(frist_von(&gespraeche, "allgemein"), Frist::Tage(30));
        assert_eq!(
            frist_von(&gespraeche, "kaputt"),
            Frist::Gesperrt {
                datei: "channels/kaputt/einstellungen.toml".into(),
                zeile: Some(1),
            }
        );
        assert_eq!(frist_von(&gespraeche, "ohne"), Frist::Tage(7));
    }

    /// Logablage, die das Anhängen an `allgemein` an einer Schranke festhält, bis der Test sie freigibt.
    struct Schranke {
        innen: Arc<Speicherablage>,
        angekommen: Mutex<std::sync::mpsc::Sender<()>>,
        frei: Mutex<std::sync::mpsc::Receiver<()>>,
    }

    impl Logablage for Schranke {
        fn gespraeche(&self) -> std::io::Result<Vec<Gespraech>> {
            self.innen.gespraeche()
        }
        fn tage(&self, gespraech: &Gespraech) -> std::io::Result<Vec<NaiveDate>> {
            self.innen.tage(gespraech)
        }
        fn lesen(&self, gespraech: &Gespraech, tag: NaiveDate) -> std::io::Result<Vec<u8>> {
            self.innen.lesen(gespraech, tag)
        }
        fn anhaengen(
            &self,
            gespraech: &Gespraech,
            tag: NaiveDate,
            satz: &str,
        ) -> std::io::Result<Angehaengt> {
            if *gespraech == Gespraech::channel("allgemein") {
                sperren(&self.angekommen).send(()).expect("Test wartet");
                sperren(&self.frei).recv().expect("Freigabe");
            }
            self.innen.anhaengen(gespraech, tag, satz)
        }
        fn channel_ordner_anlegen(&self, gespraech: &Gespraech) -> std::io::Result<()> {
            self.innen.channel_ordner_anlegen(gespraech)
        }
        fn einstellungen_schreiben(
            &self,
            gespraech: &Gespraech,
            bytes: &[u8],
        ) -> std::io::Result<()> {
            self.innen.einstellungen_schreiben(gespraech, bytes)
        }
        fn einstellungen_lesen(&self, gespraech: &Gespraech) -> std::io::Result<Option<Vec<u8>>> {
            self.innen.einstellungen_lesen(gespraech)
        }
        fn ordner_entfernen(&self, gespraech: &Gespraech) -> std::io::Result<()> {
            self.innen.ordner_entfernen(gespraech)
        }
        fn tag_entfernen(&self, gespraech: &Gespraech, tag: NaiveDate) -> std::io::Result<()> {
            self.innen.tag_entfernen(gespraech, tag)
        }
    }

    /// R00011 F0042 „Schranke über zwei Channels“ (B0213): Thread 1 sendet in `allgemein` und hängt an der
    /// Schranke; Thread 2 sendet in `technik`. Nach der Freigabe sieht der Rückruf `allgemein` vor `technik`, und
    /// `zeit(allgemein) < zeit(technik)` — mit einer Sperre je Gespräch käme `technik` zuerst (ADR-0035).
    #[test]
    fn die_veroeffentlichung_folgt_den_zeiten_ueber_zwei_channels() {
        let innen = Arc::new(Speicherablage::neu());
        innen.gespraech_anlegen(&Gespraech::channel("allgemein"));
        innen.gespraech_anlegen(&Gespraech::channel("technik"));
        let (angekommen_senden, angekommen) = std::sync::mpsc::channel();
        let (frei, frei_empfangen) = std::sync::mpsc::channel();
        let schranke = Schranke {
            innen,
            angekommen: Mutex::new(angekommen_senden),
            frei: Mutex::new(frei_empfangen),
        };
        let gesehen: Arc<Mutex<Vec<(Gespraech, Zeit)>>> = Arc::default();
        let uhr = Arc::new(GestellteUhr::neu(zeit("2026-10-28T12:00:00.000+01:00")));
        let gespraeche = Arc::new(
            Gespraeche::laden(Arc::new(schranke), uhr)
                .expect("geladen")
                .mit_nachricht_veroeffentlichen({
                    let gesehen = Arc::clone(&gesehen);
                    Arc::new(move |gespraech: &Gespraech, nachricht: &Nachricht| {
                        sperren(&gesehen).push((gespraech.clone(), nachricht.zeit));
                    })
                }),
        );

        let eins = {
            let gespraeche = Arc::clone(&gespraeche);
            std::thread::spawn(move || {
                gespraeche.senden(
                    &Gespraech::channel("allgemein"),
                    &claude(),
                    Art::Agent,
                    "eins",
                )
            })
        };
        angekommen
            .recv_timeout(Duration::from_secs(10))
            .expect("Thread 1 an der Schranke");
        let zwei = {
            let gespraeche = Arc::clone(&gespraeche);
            std::thread::spawn(move || {
                gespraeche.senden(
                    &Gespraech::channel("technik"),
                    &claude(),
                    Art::Agent,
                    "zwei",
                )
            })
        };
        std::thread::sleep(Duration::from_millis(200));
        frei.send(()).expect("Freigabe");
        let eins = eins.join().expect("Thread 1").expect("gesendet");
        let zwei = zwei.join().expect("Thread 2").expect("gesendet");

        assert!(eins.zeit < zwei.zeit, "{} < {}", eins.zeit, zwei.zeit);
        assert_eq!(
            *sperren(&gesehen),
            vec![
                (Gespraech::channel("allgemein"), eins.zeit),
                (Gespraech::channel("technik"), zwei.zeit),
            ]
        );
    }

    /// Ein Schritt in `sperren`, gemeldet mit der Adresse der Sperre: davor (`Will`) und danach (`Hat`).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(super) enum Sperrschritt {
        Will(usize),
        Hat(usize),
    }

    /// Beobachter der Schritte eines Threads; `false` legt ihn ab.
    type Sperrhaken = Box<dyn FnMut(Sperrschritt) -> bool>;

    thread_local! {
        static SPERRHAKEN: std::cell::RefCell<Option<Sperrhaken>> = const { std::cell::RefCell::new(None) };
    }

    /// Nur im Testbau aus `sperren` gerufen: meldet den Schritt dem Beobachter des Threads, falls einer gesetzt ist.
    pub(super) fn sperrhaken(schritt: Sperrschritt) {
        let Some(mut haken) = SPERRHAKEN.with_borrow_mut(Option::take) else {
            return;
        };
        if haken(schritt) {
            SPERRHAKEN.set(Some(haken));
        }
    }

    fn sperrhaken_setzen(haken: impl FnMut(Sperrschritt) -> bool + 'static) {
        SPERRHAKEN.set(Some(Box::new(haken)));
    }

    /// Beobachter für die Pflege: nach ihrer ersten genommenen Sperre gibt er das Senden frei (`los`) und hält die
    /// Sperre, bis das Senden seine erste Sperre hat (andere Sperre zuerst → ABBA, beide verklemmen) oder auf genau
    /// diese wartet (gleiche Ordnung, kein Hängen möglich). Danach legt er sich ab.
    fn erste_sperre_halten(
        los: std::sync::mpsc::Sender<()>,
        sendeschritte: std::sync::mpsc::Receiver<Sperrschritt>,
    ) -> impl FnMut(Sperrschritt) -> bool {
        move |schritt| {
            let Sperrschritt::Hat(gehalten) = schritt else {
                return true;
            };
            los.send(()).expect("Senden wartet auf die Freigabe");
            loop {
                match sendeschritte.recv().expect("Senden meldet seine Schritte") {
                    Sperrschritt::Will(sperre) if sperre == gehalten => return false,
                    Sperrschritt::Hat(_) => return false,
                    Sperrschritt::Will(_) => {}
                }
            }
        }
    }

    /// R00011 F0042 „Sperrordnung“: Pflege der Frist und Senden je 200-mal gleichzeitig über denselben Channel;
    /// je Runde liegt ein Tag jenseits der Frist in der Ablage, den die Pflege unter der Gesprächssperre entfernt
    /// → beide enden innerhalb von 10 s (kein Hängen), 200 Nachrichten, 200 Tage entfallen, nur heute bleibt.
    /// Die Überlappung ist erzwungen, nicht dem Zufall überlassen: die Pflege hält ihre erste Sperre, bis das
    /// Senden seine erste genommen hat oder auf genau diese wartet (`erste_sperre_halten`). Nimmt die Pflege eine
    /// andere Sperre zuerst als das Senden (ABBA), verklemmen sich beide in jedem Lauf.
    #[test]
    fn pflege_und_senden_gleichzeitig_haengen_nicht() {
        let (gespraeche, ablage, _) = im_speicher("2026-10-28T12:00:00.000+01:00");
        let gespraeche = Arc::new(gespraeche);
        let allgemein = Gespraech::channel("allgemein");
        let heute = tag("2026-10-28");
        let erster_alter_tag = tag("2026-01-01");
        let (fertig, fertige) = std::sync::mpsc::channel();
        let (los, los_empfangen) = std::sync::mpsc::channel();
        let (sendeschritt, sendeschritte) = std::sync::mpsc::channel();
        let pflege = {
            let gespraeche = Arc::clone(&gespraeche);
            let fertig = fertig.clone();
            std::thread::spawn(move || {
                let mut entfallene_tage = 0;
                sperrhaken_setzen(erste_sperre_halten(los, sendeschritte));
                for runde in 0..200 {
                    let alter_tag = erster_alter_tag + chrono::Days::new(runde);
                    ablage.tag_setzen(&allgemein, alter_tag, "@ alt\n\n");
                    for bericht in gespraeche.pflegen(heute, Pflegeanlass::Tageswechsel) {
                        if let Pflegebericht::Entfallen { tage, .. } = bericht {
                            entfallene_tage += tage.len();
                        }
                    }
                }
                fertig.send("pflege").expect("Test wartet");
                (entfallene_tage, ablage)
            })
        };
        let senden = {
            let gespraeche = Arc::clone(&gespraeche);
            std::thread::spawn(move || {
                los_empfangen.recv().expect("Pflege hält ihre erste Sperre");
                sperrhaken_setzen(move |schritt| {
                    // Hat die Pflege entschieden, hört sie nicht mehr zu; das Melden darf dann ins Leere gehen.
                    let _ = sendeschritt.send(schritt);
                    matches!(schritt, Sperrschritt::Will(_))
                });
                for nummer in 0..200 {
                    gespraeche
                        .senden(
                            &Gespraech::channel("allgemein"),
                            &claude(),
                            Art::Agent,
                            &format!("n{nummer}"),
                        )
                        .expect("gesendet");
                }
                fertig.send("senden").expect("Test wartet");
            })
        };

        let frist = Instant::now() + Duration::from_secs(10);
        let erstes = fertige.recv_timeout(frist.saturating_duration_since(Instant::now()));
        let zweites = fertige.recv_timeout(frist.saturating_duration_since(Instant::now()));

        assert!(erstes.is_ok() && zweites.is_ok(), "{erstes:?} {zweites:?}");
        let (entfallene_tage, ablage) = pflege.join().expect("Pflege");
        senden.join().expect("Senden");
        assert_eq!(gespraeche.kennzahlen().nachrichten, 200);
        assert_eq!(entfallene_tage, 200);
        assert_eq!(
            Logablage::tage(ablage.as_ref(), &Gespraech::channel("allgemein")).expect("Tage"),
            vec![heute]
        );
    }
}
