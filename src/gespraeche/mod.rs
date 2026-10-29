//! Gespräche: einziger Schreiber des Ordners (A00006) mit Logformat, Logablage und Verlaufsindex.

pub mod abfrage;
mod anhaenge;
pub mod anhangablage;
pub mod anhangeingang;
pub mod anhangid;
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
pub use anhaenge::{Anhangkennzahlen, Anmeldefehler, STANDARD_ANHANGQUOTE};
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
        }
    }
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
        match self.senden_mit(gespraech, von, art, text, None)?.0 {
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
        let (gesendet, verdraengt) =
            self.senden_mit(gespraech, von, art, text, Some(schluessel))?;
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
        let pruefsumme = wiederholung::pruefsumme(text);
        let pruefung = schluessel.map(|schluessel| {
            stand
                .wiederholung
                .pruefen(von, schluessel, gespraech, pruefsumme, jetzt)
        });
        match pruefung {
            Some(Pruefung::Anders) => return Err(Sendefehler::SchluesselAnders),
            Some(Pruefung::Wiederholt { zeit }) => {
                return Ok((
                    Gesendet::Wiederholt(nachricht_bilden(zeit, von, art, text).0),
                    None,
                ));
            }
            Some(Pruefung::Neu) | None => {}
        }
        let zeit = crate::uhr::naechste_zeit(jetzt, stand.letzte);
        stand.letzte = Some(zeit);
        self.letzte_ms
            .store(zeit.timestamp_millis(), Ordering::Relaxed);
        let (nachricht, satz) = nachricht_bilden(zeit, von, art, text);
        let satzbytes = satz.len() as u64;
        if let Some(hoechstens) = self.hoechstens {
            self.belegung
                .pruefen(satzbytes, hoechstens)
                .map_err(|ueber| Sendefehler::Quote {
                    belegt: ueber.belegt,
                    hoechstens: ueber.hoechstens,
                })?;
        }
        let tag = nachricht.zeit.date_naive();
        let angehaengt = self
            .ablage
            .anhaengen(gespraech, tag, &satz)
            .map_err(|fehler| Sendefehler::Ablage {
                datei: gespraech.tagesdatei(tag),
                grund: fehler.to_string(),
            })?;
        verlauf.anhaengen(tag, &satz, angehaengt.zeilenende_ergaenzt);
        let dazu = satzbytes + u64::from(angehaengt.zeilenende_ergaenzt);
        let vorher = self.belegung.dazu(dazu);
        self.ueber_90_melden(vorher, vorher.saturating_add(dazu));
        let verdraengt = schluessel.and_then(|schluessel| {
            stand
                .wiederholung
                .merken(von, schluessel, gespraech, pruefsumme, zeit, jetzt)
        });
        (self.nachricht_veroeffentlichen)(gespraech, &nachricht);
        Ok((Gesendet::Neu(nachricht), verdraengt))
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
                .filter_map(|satz| logformat::satz_als_nachricht(satz))
                .collect(),
            aelter: block.aelter,
            neuer: block.neuer,
        })
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
        for gelesen in logformat::saetze(&rohtext) {
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
        verlauf.tag_aufnehmen(tag, rohtext, satzanfaenge);
    }
}

/// Lesend: wie `sperren` — das Verzeichnis wird erst nach erfolgreichem Schreiben verändert.
fn lesend<T>(sperre: &RwLock<T>) -> std::sync::RwLockReadGuard<'_, T> {
    sperre
        .read()
        .unwrap_or_else(|vergiftet| vergiftet.into_inner())
}

/// Die Nachricht mit ihrem Satz; der Text so, wie die Verlauf-API ihn später liefert (ADR-0034 Regel 4).
fn nachricht_bilden(zeit: Zeit, von: &Kennung, art: Art, text: &str) -> (Nachricht, String) {
    let mut nachricht = Nachricht {
        zeit,
        von: von.clone(),
        art: Some(art),
        text: text.to_string(),
        anhaenge: Vec::new(),
    };
    let satz = logformat::satz(&nachricht);
    nachricht.text = logformat::text(&satz);
    (nachricht, satz)
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
