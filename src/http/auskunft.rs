//! Auskunft mit Cache (ADR-0045 Regel 6 mit Ergänzungen E3, E6, E8 und „Karte #35 Nacharbeit“): jede Frage an die
//! Namensauskunft läuft blockierend auf einem eigenen Faden, höchstens zwei gleichzeitig; eine Anfrage wartet
//! höchstens eine Sekunde ab ihrem Eingang, danach läuft die Auflösung zu Ende und füllt den Cache. Einziger Schreiber des Caches ist diese Auskunft, im Auftrag
//! des Grenzschutzes.

use super::namensauskunft::Namensauskunft;
use crate::protokoll::episoden::Auftreten;
use crate::protokoll::{Bereich, Kontext, Protokoll, Schluessel, Stufe};
use crate::uhr::{Uhr, Zeit};
use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;
use tokio::sync::{OwnedSemaphorePermit, Semaphore, watch};
use tokio::time::Instant;

/// Höchstens so viele Auflösungen laufen gleichzeitig (eigene Semaphore, nicht die Werkbank).
pub const GLEICHZEITIG: usize = 2;
/// So lange wartet eine Anfrage höchstens auf ihre Auflösung — eine Frist je Anfrage, ab ihrem Eingang.
pub const WARTEZEIT: Duration = Duration::from_secs(1);
/// So lange gilt ein Ergebnis „erlaubt“ bzw. „nicht erlaubt“ (auch Fehler und Zeitgrenze des Resolvers).
pub const ERLAUBT_GILT: Duration = Duration::from_secs(5 * 60);
pub const NICHT_ERLAUBT_GILT: Duration = Duration::from_secs(60);
/// So lange gilt ein positiver Eintrag nach seinem Ablauf noch, während er im Hintergrund aufgefrischt wird.
pub const GNADENFRIST: Duration = Duration::from_secs(10 * 60);
/// Höchstens so viele fertige Einträge: die Summe der vier Fächer 128 + 96 + 16 + 16 (ADR-0045 Ergänzung „Karte
/// #35 Nacharbeit“ Punkt 2); dazu höchstens `GLEICHZEITIG` laufende.
pub const HOECHSTENS_EINTRAEGE: usize = 256;

/// Wonach gefragt wird: die Namen einer lokalen Adresse oder das Urteil über einen Namen.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Abfrage {
    Rueckwaerts(IpAddr),
    Vorwaerts(String),
}

/// Was der Cache hält: die Rückwärtsnamen einer Adresse oder ob ein Name auf eine eigene Adresse zeigt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ergebnis {
    Namen(Vec<String>),
    Erlaubt(bool),
}

impl Ergebnis {
    fn positiv(&self) -> bool {
        match self {
            Ergebnis::Namen(namen) => !namen.is_empty(),
            Ergebnis::Erlaubt(erlaubt) => *erlaubt,
        }
    }
}

/// Die Arbeit einer Auflösung: fragt die Namensauskunft (blockierend) und bildet das Ergebnis.
pub type Arbeit = Box<dyn FnOnce(&dyn Namensauskunft) -> Ergebnis + Send>;

enum Stand {
    Laeuft(watch::Receiver<Option<Ergebnis>>),
    Fertig {
        ergebnis: Ergebnis,
        bis: Zeit,
        gemeldet: HashSet<String>,
        /// Läuft für diesen positiven Eintrag in der Gnadenfrist schon eine Auffrischung?
        auffrischung: bool,
    },
}

struct Eintrag {
    stand: Stand,
    nummer: u64,
}

#[derive(Default)]
struct Cache {
    eintraege: HashMap<Abfrage, Eintrag>,
    naechste: u64,
    eimer: Eimer,
}

/// Neue Auflösungen je Sekunde und Vorrat des Eimers (ADR-0045 Ergänzung „Karte #35 Nacharbeit“ Punkt 2).
pub const NEUE_JE_SEKUNDE: u64 = 10;
pub const VORRAT: u64 = 20;

/// Ein Eimer für die ganze Instanz, gemessen in Tausendstel Zeichen; Zeit über den Uhr-Provider.
struct Eimer {
    tausendstel: u64,
    stand: Option<Zeit>,
}

impl Default for Eimer {
    fn default() -> Eimer {
        Eimer {
            tausendstel: VORRAT * 1000,
            stand: None,
        }
    }
}

impl Eimer {
    /// Füllt nach der verstrichenen Zeit auf (höchstens bis zum Vorrat) und nimmt ein Zeichen, wenn eines da ist.
    fn zeichen_nehmen(&mut self, jetzt: Zeit) -> bool {
        let verstrichen_ms = self.stand.map_or(0, |stand| {
            (jetzt - stand).num_milliseconds().max(0).unsigned_abs()
        });
        self.tausendstel =
            (self.tausendstel + verstrichen_ms.saturating_mul(NEUE_JE_SEKUNDE)).min(VORRAT * 1000);
        if self.stand.is_none_or(|stand| jetzt > stand) {
            self.stand = Some(jetzt);
        }
        let genug = self.tausendstel >= 1000;
        if genug {
            self.tausendstel -= 1000;
        }
        genug
    }
}

struct Innen {
    port: Arc<dyn Namensauskunft>,
    uhr: Arc<dyn Uhr>,
    protokoll: Arc<Protokoll>,
    cache: Mutex<Cache>,
}

/// Die Auskunft mit Cache, Semaphore und Wartezeit.
pub struct Auskunft {
    innen: Arc<Innen>,
    erlaubnis: Arc<Semaphore>,
    fadenstart: Fadenstart,
}

/// Startet die Arbeit einer Auflösung auf einem eigenen Faden; im Test ersetzbar, um einen gescheiterten Start
/// herzustellen.
type Fadenstart = fn(Box<dyn FnOnce() + Send>) -> std::io::Result<()>;

fn faden_starten(arbeit: Box<dyn FnOnce() + Send>) -> std::io::Result<()> {
    std::thread::Builder::new()
        .name("namensauskunft".to_string())
        .spawn(arbeit)
        .map(|_| ())
}

enum Nachsehen {
    Treffer(Ergebnis),
    Warten(watch::Receiver<Option<Ergebnis>>),
    KeineErlaubnis,
    FristAbgelaufen,
}

impl Auskunft {
    pub fn neu(
        port: Arc<dyn Namensauskunft>,
        uhr: Arc<dyn Uhr>,
        protokoll: Arc<Protokoll>,
    ) -> Auskunft {
        Auskunft {
            innen: Arc::new(Innen {
                port,
                uhr,
                protokoll,
                cache: Mutex::new(Cache::default()),
            }),
            erlaubnis: Arc::new(Semaphore::new(GLEICHZEITIG)),
            fadenstart: faden_starten,
        }
    }

    /// Die Rückwärtsnamen der (kanonischen) Adresse — `None`, wenn keine Antwort binnen der Wartezeit kam oder
    /// keine Auflösung starten durfte.
    pub async fn namen_der_adresse(&self, adresse: IpAddr, frist: Instant) -> Option<Vec<String>> {
        let adresse = adresse.to_canonical();
        let arbeit: Arbeit = Box::new(move |port| Ergebnis::Namen(port.namen_der_adresse(adresse)));
        match self
            .erfragen(Abfrage::Rueckwaerts(adresse), arbeit, frist)
            .await
        {
            Some(Ergebnis::Namen(namen)) => Some(namen),
            _ => None,
        }
    }

    /// Das Urteil über einen Namen; `urteilen` fragt die Namensauskunft (vorwärts, eigen?) und entscheidet.
    pub async fn urteil_ueber(
        &self,
        name: &str,
        urteilen: impl FnOnce(&dyn Namensauskunft) -> bool + Send + 'static,
        frist: Instant,
    ) -> Option<bool> {
        let arbeit: Arbeit = Box::new(move |port| Ergebnis::Erlaubt(urteilen(port)));
        match self
            .erfragen(Abfrage::Vorwaerts(name.to_string()), arbeit, frist)
            .await
        {
            Some(Ergebnis::Erlaubt(erlaubt)) => Some(erlaubt),
            _ => None,
        }
    }

    /// Information „Host-Name über … erlaubt“ mit Name und lokaler Adresse — einmal je Cache-Eintrag und Name
    /// (ADR-0045 Regel 8).
    pub fn erlaubt_melden(&self, abfrage: &Abfrage, name: &str, lokal: IpAddr) {
        if !self.erstmals_gemeldet(abfrage, name) {
            return;
        }
        let text = match abfrage {
            Abfrage::Rueckwaerts(_) => "Host-Name über Rückwärtsauflösung erlaubt",
            Abfrage::Vorwaerts(_) => "Host-Name über lokalen Namensraum erlaubt",
        };
        self.innen.protokoll.eintragen(
            Stufe::Information,
            Bereich::Grenzschutz,
            text,
            Kontext::neu()
                .mit(Schluessel::Name, name)
                .mit(Schluessel::Adresse, lokal),
        );
    }

    /// `true` genau beim ersten Mal je gültigem Cache-Eintrag und Name — für die Information „… erlaubt“.
    fn erstmals_gemeldet(&self, abfrage: &Abfrage, name: &str) -> bool {
        let mut cache = self
            .innen
            .cache
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        match cache
            .eintraege
            .get_mut(abfrage)
            .map(|eintrag| &mut eintrag.stand)
        {
            Some(Stand::Fertig { gemeldet, .. }) => gemeldet.insert(name.to_string()),
            _ => false,
        }
    }

    /// Treffer im Cache, sonst auf eine laufende Auflösung warten oder eine starten — höchstens bis zur `frist` der
    /// Anfrage (ADR-0045 Ergänzung „Karte #35 Nacharbeit“ Punkt 1: eine Frist je Anfrage, nie je Frage neu).
    async fn erfragen(&self, abfrage: Abfrage, arbeit: Arbeit, frist: Instant) -> Option<Ergebnis> {
        let mut empfang = match self.nachsehen_oder_starten(&abfrage, arbeit, frist) {
            Nachsehen::Treffer(ergebnis) => return Some(ergebnis),
            Nachsehen::KeineErlaubnis => return None,
            Nachsehen::FristAbgelaufen => {
                self.zu_langsam(&abfrage);
                return None;
            }
            Nachsehen::Warten(empfang) => empfang,
        };
        let warten = async {
            empfang
                .wait_for(Option::is_some)
                .await
                .ok()
                .and_then(|ergebnis| ergebnis.clone())
        };
        match tokio::time::timeout_at(frist, warten).await {
            Ok(ergebnis) => ergebnis,
            Err(_) => {
                self.zu_langsam(&abfrage);
                None
            }
        }
    }

    /// Unter der Sperre des Caches: gültiger Eintrag → Treffer; laufende Auflösung → warten; sonst eine Erlaubnis
    /// ohne Warten holen (E3) und ein Zeichen aus dem Eimer (Rate) — ohne beides kein Eintrag, keine Frage; ist die
    /// Frist schon abgelaufen, ebenso.
    fn nachsehen_oder_starten(
        &self,
        abfrage: &Abfrage,
        arbeit: Arbeit,
        frist: Instant,
    ) -> Nachsehen {
        let jetzt = self.innen.uhr.jetzt();
        let mut cache = self
            .innen
            .cache
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        match cache
            .eintraege
            .get_mut(abfrage)
            .map(|eintrag| &mut eintrag.stand)
        {
            Some(Stand::Fertig { ergebnis, bis, .. }) if jetzt < *bis => {
                return Nachsehen::Treffer(ergebnis.clone());
            }
            Some(Stand::Fertig {
                ergebnis,
                bis,
                auffrischung,
                ..
            }) if ergebnis.positiv() && jetzt < *bis + GNADENFRIST => {
                let treffer = ergebnis.clone();
                // Gnadenfrist (ADR-0045 Ergänzung „Karte #35 Nacharbeit“ Punkt 3): sofort erlaubt; auffrischen nur
                // mit freier Erlaubnis und ohne Zeichen aus dem Eimer, höchstens eine Auffrischung zugleich.
                if !*auffrischung
                    && let Ok(erlaubnis) = Arc::clone(&self.erlaubnis).try_acquire_owned()
                {
                    *auffrischung = true;
                    drop(cache);
                    let (senden, _) = watch::channel(None);
                    self.aufloesung_starten(abfrage.clone(), arbeit, senden, erlaubnis);
                }
                return Nachsehen::Treffer(treffer);
            }
            Some(Stand::Laeuft(empfang)) => return Nachsehen::Warten(empfang.clone()),
            _ => {}
        }
        if Instant::now() >= frist {
            return Nachsehen::FristAbgelaufen;
        }
        let Ok(erlaubnis) = Arc::clone(&self.erlaubnis).try_acquire_owned() else {
            return Nachsehen::KeineErlaubnis;
        };
        // Ohne Zeichen aus dem Eimer wie ohne Erlaubnis (E3): keine Frage, kein Eintrag.
        if !cache.eimer.zeichen_nehmen(jetzt) {
            return Nachsehen::KeineErlaubnis;
        }
        let (senden, empfang) = watch::channel(None);
        cache.eintragen(abfrage.clone(), Stand::Laeuft(empfang.clone()));
        drop(cache);
        self.aufloesung_starten(abfrage.clone(), arbeit, senden, erlaubnis);
        Nachsehen::Warten(empfang)
    }

    /// Startet die Auflösung auf einem eigenen, nicht gejointen Faden (ADR-0045 Ergänzung „Karte #35 Nacharbeit“
    /// Punkt 3): das Ende des Programms wartet nicht auf ein hängendes `getaddrinfo`. Die Erlaubnis hält der Faden
    /// bis zu seinem Ende; ihr Ergebnis ersetzt den Eintrag. Scheitert der Start, gilt das als „ohne Antwort“ ohne
    /// Eintrag, und die Erlaubnis ist mit der verworfenen Arbeit sofort zurück.
    fn aufloesung_starten(
        &self,
        abfrage: Abfrage,
        arbeit: Arbeit,
        senden: watch::Sender<Option<Ergebnis>>,
        erlaubnis: OwnedSemaphorePermit,
    ) {
        let innen = Arc::clone(&self.innen);
        let schluessel = abfrage.clone();
        let faden = Box::new(move || innen.aufloesen(abfrage, arbeit, senden, erlaubnis));
        if (self.fadenstart)(faden).is_err() {
            self.ohne_faden(&schluessel);
        }
    }

    /// Nach einem gescheiterten Fadenstart: ein laufender Eintrag fällt, eine Auffrischung gilt als nicht gestartet.
    fn ohne_faden(&self, abfrage: &Abfrage) {
        let mut cache = self
            .innen
            .cache
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        match cache
            .eintraege
            .get_mut(abfrage)
            .map(|eintrag| &mut eintrag.stand)
        {
            Some(Stand::Laeuft(_)) => {
                cache.eintraege.remove(abfrage);
            }
            Some(Stand::Fertig { auffrischung, .. }) => *auffrischung = false,
            None => {}
        }
    }

    /// Warning-Episode „Namensauflösung zu langsam“ (ADR-0045 Regel 8), eine Quelle für alle Schlüssel.
    fn zu_langsam(&self, abfrage: &Abfrage) {
        self.innen.protokoll.episode(Auftreten {
            stufe: Stufe::Warning,
            bereich: Bereich::Grenzschutz,
            text: "Namensauflösung zu langsam",
            quelle: "namensauskunft".to_string(),
            kontext: Kontext::neu()
                .mit(Schluessel::Art, abfrage.art())
                .mit(Schluessel::Schluessel, abfrage.schluessel()),
            art: None,
            verworfen: None,
            bytes: None,
            anzeige: None,
        });
    }

    #[cfg(test)]
    fn ist_fertig(&self, abfrage: &Abfrage) -> bool {
        matches!(
            self.innen
                .cache
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .eintraege
                .get(abfrage)
                .map(|eintrag| &eintrag.stand),
            Some(Stand::Fertig { .. })
        )
    }

    #[cfg(test)]
    fn fertige(&self) -> usize {
        self.innen
            .cache
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .eintraege
            .values()
            .filter(|eintrag| matches!(eintrag.stand, Stand::Fertig { .. }))
            .count()
    }

    #[cfg(test)]
    fn hat_eintrag(&self, abfrage: &Abfrage) -> bool {
        self.innen
            .cache
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .eintraege
            .contains_key(abfrage)
    }
}

impl Innen {
    /// Im blockierenden Faden: fragen (jede Frage mit DEBUG-Zeile davor), das Ergebnis eintragen und melden; die
    /// Erlaubnis fällt erst nach der Arbeit, auch wenn niemand mehr wartet. Eine Panik der Arbeit gilt als „nicht erlaubt“.
    fn aufloesen(
        &self,
        abfrage: Abfrage,
        arbeit: Arbeit,
        senden: watch::Sender<Option<Ergebnis>>,
        erlaubnis: OwnedSemaphorePermit,
    ) {
        let fragend = Fragend {
            port: &*self.port,
            protokoll: &self.protokoll,
        };
        let ergebnis = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| arbeit(&fragend)))
            .unwrap_or_else(|_| abfrage.ohne_antwort());
        let gilt = if ergebnis.positiv() {
            ERLAUBT_GILT
        } else {
            NICHT_ERLAUBT_GILT
        };
        let bis = self.uhr.jetzt() + gilt;
        self.cache
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .eintragen(
                abfrage,
                Stand::Fertig {
                    ergebnis: ergebnis.clone(),
                    bis,
                    gemeldet: HashSet::new(),
                    auffrischung: false,
                },
            );
        // Erst die Erlaubnis zurück, dann melden: wer auf das Ergebnis wartet und gleich die nächste Auflösung
        // braucht, findet sie frei.
        drop(erlaubnis);
        let _ = senden.send(Some(ergebnis));
    }
}

impl Cache {
    /// Setzt den Stand eines Schlüssels mit der nächsten Nummer (dem Alter). Ein fertiger Eintrag gehört in das
    /// Fach seines Ergebnisses; ist es voll, fällt der älteste fertige Eintrag **desselben** Fachs (ADR-0045
    /// Ergänzung „Karte #35 Nacharbeit“ Punkt 2). Laufende Einträge gehören in kein Fach.
    fn eintragen(&mut self, abfrage: Abfrage, stand: Stand) {
        let fach = match &stand {
            Stand::Fertig { ergebnis, .. } => Some(Fach::von(ergebnis)),
            Stand::Laeuft(_) => None,
        };
        self.naechste += 1;
        self.eintraege.insert(
            abfrage,
            Eintrag {
                stand,
                nummer: self.naechste,
            },
        );
        if let Some(fach) = fach {
            self.fach_begrenzen(fach);
        }
    }

    fn fach_begrenzen(&mut self, fach: Fach) {
        let im_fach = |eintrag: &Eintrag| match &eintrag.stand {
            Stand::Fertig { ergebnis, .. } => Fach::von(ergebnis) == fach,
            Stand::Laeuft(_) => false,
        };
        while self
            .eintraege
            .values()
            .filter(|eintrag| im_fach(eintrag))
            .count()
            > fach.plaetze()
        {
            let aeltester = self
                .eintraege
                .iter()
                .filter(|(_, eintrag)| im_fach(eintrag))
                .min_by_key(|(_, eintrag)| eintrag.nummer)
                .map(|(abfrage, _)| abfrage.clone());
            match aeltester {
                Some(abfrage) => self.eintraege.remove(&abfrage),
                None => break,
            };
        }
    }
}

/// Die vier Fächer des Caches: ein negatives Ergebnis verdrängt nie ein positives, ein Vorwärtseintrag nie einen
/// Rückwärtseintrag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fach {
    VorwaertsErlaubt,
    VorwaertsNichtErlaubt,
    RueckwaertsMitNamen,
    RueckwaertsOhneNamen,
}

const _: () = assert!(128 + 96 + 16 + 16 == HOECHSTENS_EINTRAEGE);

impl Fach {
    fn von(ergebnis: &Ergebnis) -> Fach {
        match ergebnis {
            Ergebnis::Erlaubt(true) => Fach::VorwaertsErlaubt,
            Ergebnis::Erlaubt(false) => Fach::VorwaertsNichtErlaubt,
            Ergebnis::Namen(namen) if namen.is_empty() => Fach::RueckwaertsOhneNamen,
            Ergebnis::Namen(_) => Fach::RueckwaertsMitNamen,
        }
    }

    fn plaetze(self) -> usize {
        match self {
            Fach::VorwaertsErlaubt => 128,
            Fach::VorwaertsNichtErlaubt => 96,
            Fach::RueckwaertsMitNamen | Fach::RueckwaertsOhneNamen => 16,
        }
    }
}

/// Die Namensauskunft mit einer DEBUG-Zeile vor jeder Frage (E6) — hier, nicht im Port, damit sie mit der
/// echten Auskunft und mit dem Testknopf gleich entsteht.
struct Fragend<'a> {
    port: &'a dyn Namensauskunft,
    protokoll: &'a Protokoll,
}

impl Fragend<'_> {
    fn gefragt(&self, art: &str, schluessel: impl ToString) {
        self.protokoll.eintragen(
            Stufe::Debug,
            Bereich::Grenzschutz,
            "Namensauskunft gefragt",
            Kontext::neu()
                .mit(Schluessel::Art, art)
                .mit(Schluessel::Schluessel, schluessel),
        );
    }
}

impl Namensauskunft for Fragend<'_> {
    fn namen_der_adresse(&self, adresse: IpAddr) -> Vec<String> {
        self.gefragt("rueckwaerts", adresse);
        self.port.namen_der_adresse(adresse)
    }

    fn adressen_des_namens(&self, name: &str) -> Vec<IpAddr> {
        self.gefragt("vorwaerts", name);
        self.port.adressen_des_namens(name)
    }

    fn ist_eigene_adresse(&self, adresse: IpAddr) -> bool {
        self.gefragt("eigen", adresse);
        self.port.ist_eigene_adresse(adresse)
    }
}

impl Abfrage {
    /// Das Ergebnis, wenn die Arbeit keine Antwort brachte.
    fn ohne_antwort(&self) -> Ergebnis {
        match self {
            Abfrage::Rueckwaerts(_) => Ergebnis::Namen(Vec::new()),
            Abfrage::Vorwaerts(_) => Ergebnis::Erlaubt(false),
        }
    }

    fn art(&self) -> &'static str {
        match self {
            Abfrage::Rueckwaerts(_) => "rueckwaerts",
            Abfrage::Vorwaerts(_) => "vorwaerts",
        }
    }

    fn schluessel(&self) -> String {
        match self {
            Abfrage::Rueckwaerts(adresse) => adresse.to_string(),
            Abfrage::Vorwaerts(name) => name.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::testauskunft::{Testauskunft, ip};
    use crate::protokoll::senke::Speicher;
    use crate::protokoll::testhilfe::{im_speicher, zeit};
    use crate::uhr::GestellteUhr;
    use std::sync::atomic::Ordering;
    use std::time::Instant;

    /// Die Frist einer Anfrage, die jetzt eingeht.
    fn frist() -> tokio::time::Instant {
        tokio::time::Instant::now() + WARTEZEIT
    }

    const START: &str = "2026-10-29T12:00:00+01:00";

    struct Aufbau {
        auskunft: Arc<Auskunft>,
        uhr: Arc<GestellteUhr>,
        speicher: Speicher,
    }

    fn aufbau(test: Arc<Testauskunft>) -> Aufbau {
        let (protokoll, speicher, uhr, _) = im_speicher(START, Stufe::Debug);
        let auskunft = Arc::new(Auskunft::neu(
            Arc::clone(&test) as Arc<dyn Namensauskunft>,
            Arc::clone(&uhr) as Arc<dyn Uhr>,
            protokoll,
        ));
        Aufbau {
            auskunft,
            uhr,
            speicher,
        }
    }

    /// Urteil wie Regel 4: vorwärts auflösen, dann je Adresse „eigen?“.
    fn eigen_urteil(port: &dyn Namensauskunft, name: &str) -> bool {
        port.adressen_des_namens(name)
            .into_iter()
            .any(|adresse| port.ist_eigene_adresse(adresse))
    }

    async fn urteil(auskunft: &Auskunft, name: &str) -> Option<bool> {
        urteil_bis(auskunft, name, frist()).await
    }

    async fn urteil_bis(
        auskunft: &Auskunft,
        name: &str,
        frist: tokio::time::Instant,
    ) -> Option<bool> {
        let eigener = name.to_string();
        auskunft
            .urteil_ueber(name, move |port| eigen_urteil(port, &eigener), frist)
            .await
    }

    fn warnungen_zu_langsam(speicher: &Speicher) -> usize {
        speicher
            .zeilen()
            .into_iter()
            .filter(|zeile| zeile.contains(" WARNING grenzschutz Namensauflösung zu langsam"))
            .count()
    }

    /// Nacharbeit #35, B1 (ADR-0045 Ergänzung „Karte #35 Nacharbeit“ Punkt 1): die Frist gehört der Anfrage — eine
    /// Frage, die mit 300 ms Rest beginnt, wartet höchstens diese 300 ms, nicht eine eigene Sekunde (Mutante
    /// „Frist je Frage neu“).
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn die_frist_der_anfrage_begrenzt_die_frage() {
        let test = Testauskunft::mit(&[], &[]);
        test.sperren("langsam.local");
        let a = aufbau(Arc::clone(&test));

        let beginn = Instant::now();
        let frist = tokio::time::Instant::now() + Duration::from_millis(300);
        let antwort = begrenzt(urteil_bis(&a.auskunft, "langsam.local", frist)).await;
        let gewartet = beginn.elapsed();

        assert_eq!(antwort, None);
        assert!(gewartet >= Duration::from_millis(300), "{gewartet:?}");
        assert!(gewartet < Duration::from_millis(400), "{gewartet:?}");
        assert_eq!(warnungen_zu_langsam(&a.speicher), 1);
        test.oeffnen("langsam.local");
    }

    /// Nacharbeit #35, B1: ist die Frist beim Start einer Frage schon abgelaufen, wird nicht gefragt — keine
    /// Erlaubnis, kein Eintrag, abgewiesen wie nach einer Zeitgrenze; ein Treffer im Cache kostet keine Frist.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn mit_abgelaufener_frist_keine_frage_und_kein_eintrag() {
        let test = Testauskunft::mit(&[], &[("x.local", &["127.0.0.1"])]);
        let a = aufbau(Arc::clone(&test));
        let abgelaufen = tokio::time::Instant::now();

        assert_eq!(urteil_bis(&a.auskunft, "x.local", abgelaufen).await, None);
        assert_eq!(test.alle_fragen(), 0);
        assert!(
            !a.auskunft
                .hat_eintrag(&Abfrage::Vorwaerts("x.local".into()))
        );
        assert_eq!(a.auskunft.erlaubnis.available_permits(), GLEICHZEITIG);
        assert_eq!(warnungen_zu_langsam(&a.speicher), 1);

        assert_eq!(urteil(&a.auskunft, "x.local").await, Some(true));
        let abgelaufen = tokio::time::Instant::now();
        assert_eq!(
            urteil_bis(&a.auskunft, "x.local", abgelaufen).await,
            Some(true)
        );
        assert_eq!(test.fragen("vorwaerts", "x.local"), 1);
    }

    /// Eine Anfrage auf eine hängende Auskunft endet nur durch die Zeitgrenze; ohne sie hinge der Test — die
    /// Testgrenze (3 s) macht ihn stattdessen rot.
    async fn begrenzt<T>(anfrage: impl Future<Output = T>) -> T {
        tokio::time::timeout(Duration::from_secs(3), anfrage)
            .await
            .expect("Anfrage endet nicht — Zeitgrenze fehlt")
    }

    /// Wartet (real) darauf, dass `bedingung` wahr wird — höchstens 5 s.
    async fn bis(bedingung: impl Fn() -> bool) {
        let ende = Instant::now() + Duration::from_secs(5);
        while !bedingung() {
            assert!(Instant::now() < ende, "Bedingung nicht binnen 5 s erfüllt");
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }

    /// Kriterium „Gleichzeitigkeit“ (Matrix F1): 4 Namen, jede Auflösung hängt → nie mehr als 2 gleichzeitig; die
    /// beiden ohne Erlaubnis sind sofort abgewiesen und fragen nicht.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn hoechstens_zwei_aufloesungen_gleichzeitig_die_uebrigen_sofort_abgewiesen() {
        let test = Testauskunft::mit(&[], &[]);
        for name in ["a.local", "b.local", "c.local", "d.local"] {
            test.sperren(name);
        }
        let a = aufbau(Arc::clone(&test));
        let mut laeufe = Vec::new();
        for name in ["a.local", "b.local"] {
            let auskunft = Arc::clone(&a.auskunft);
            laeufe.push(tokio::spawn(async move {
                begrenzt(urteil(&auskunft, name)).await
            }));
        }
        bis(|| test.alle_fragen() == 2).await;

        let beginn = Instant::now();
        let c = urteil(&a.auskunft, "c.local").await;
        let d = urteil(&a.auskunft, "d.local").await;
        let ohne_erlaubnis = beginn.elapsed();

        assert_eq!((c, d), (None, None));
        assert!(
            ohne_erlaubnis < Duration::from_millis(100),
            "{ohne_erlaubnis:?}"
        );
        assert_eq!(
            test.fragen("vorwaerts", "c.local") + test.fragen("vorwaerts", "d.local"),
            0
        );
        for lauf in laeufe {
            assert_eq!(lauf.await.expect("Lauf"), None);
        }
        assert_eq!(test.hoechstens.load(Ordering::SeqCst), 2);
        for name in ["a.local", "b.local", "c.local", "d.local"] {
            test.oeffnen(name);
        }
    }

    /// Rechenbeispiel „Zeitgrenze“ (Matrix F5): A wartet 1 s und ist abgewiesen; die Auflösung läuft zu Ende und
    /// füllt den Cache; B danach erlaubt aus dem Cache, 1 Frage; genau eine WARNING-Episode.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn nach_der_zeitgrenze_laeuft_die_aufloesung_zu_ende_und_fuellt_den_cache() {
        let test = Testauskunft::mit(&[("192.168.178.65", "nuc5.fritz.box")], &[]);
        test.sperren("192.168.178.65");
        let a = aufbau(Arc::clone(&test));

        let beginn = Instant::now();
        let anfrage_a = begrenzt(a.auskunft.namen_der_adresse(ip("192.168.178.65"), frist())).await;
        let gewartet = beginn.elapsed();

        assert_eq!(anfrage_a, None);
        assert!(gewartet >= WARTEZEIT, "{gewartet:?}");
        assert!(gewartet < Duration::from_millis(1500), "{gewartet:?}");
        test.oeffnen("192.168.178.65");
        let abfrage = Abfrage::Rueckwaerts(ip("192.168.178.65"));
        bis(|| a.auskunft.ist_fertig(&abfrage)).await;

        let anfrage_b = a
            .auskunft
            .namen_der_adresse(ip("192.168.178.65"), frist())
            .await;

        assert_eq!(anfrage_b, Some(vec!["nuc5.fritz.box".to_string()]));
        assert_eq!(test.fragen("rueckwaerts", "192.168.178.65"), 1);
        let episoden: Vec<String> = a
            .speicher
            .zeilen()
            .into_iter()
            .filter(|zeile| zeile.contains(" WARNING grenzschutz Namensauflösung zu langsam"))
            .collect();
        assert_eq!(episoden.len(), 1, "{episoden:?}");
    }

    /// Matrix F1 (L1): 300 verschiedene `.local`-Namen, 50 gleichzeitig, keine Frage kommt zurück — höchstens 2
    /// Fragen laufen, jede Anfrage ist nach höchstens 1 s (+100 ms) abgewiesen, die übrigen ohne Frage.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn flut_aus_zufaelligen_lokalen_namen_bleibt_bei_zwei_fragen() {
        let test = Testauskunft::mit(&[], &[]);
        let namen: Vec<String> = (0..300).map(|nummer| format!("r{nummer}.local")).collect();
        for name in &namen {
            test.sperren(name);
        }
        let a = aufbau(Arc::clone(&test));
        let beginn = Instant::now();
        for welle in namen.chunks(50) {
            let mut laeufe = Vec::new();
            for name in welle {
                let auskunft = Arc::clone(&a.auskunft);
                let name = name.clone();
                laeufe.push(tokio::spawn(async move {
                    let start = Instant::now();
                    (begrenzt(urteil(&auskunft, &name)).await, start.elapsed())
                }));
            }
            for lauf in laeufe {
                let (urteil, dauer) = lauf.await.expect("Lauf");
                assert_eq!(urteil, None);
                assert!(dauer <= Duration::from_millis(1100), "{dauer:?}");
            }
        }

        assert!(
            beginn.elapsed() < Duration::from_secs(8),
            "{:?}",
            beginn.elapsed()
        );
        assert_eq!(test.alle_fragen(), 2);
        assert_eq!(test.hoechstens.load(Ordering::SeqCst), 2);
        for name in &namen {
            test.oeffnen(name);
        }
    }

    /// Kriterium „Geteilte Auflösung“ (Matrix F2): läuft für den Schlüssel schon eine Auflösung, wartet die
    /// zweite Anfrage darauf — 1 Frage; ebenso 50 gleichzeitige Anfragen.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn eine_laufende_aufloesung_wird_geteilt() {
        let test = Testauskunft::mit(&[], &[("same.local", &["127.0.0.1"])]);
        test.sperren("same.local");
        let a = aufbau(Arc::clone(&test));
        let mut laeufe = Vec::new();
        for _ in 0..50 {
            let auskunft = Arc::clone(&a.auskunft);
            laeufe.push(tokio::spawn(async move {
                urteil(&auskunft, "same.local").await
            }));
            tokio::time::sleep(Duration::from_millis(4)).await;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
        test.oeffnen("same.local");

        for lauf in laeufe {
            assert_eq!(lauf.await.expect("Lauf"), Some(true));
        }
        assert_eq!(test.fragen("vorwaerts", "same.local"), 1);
    }

    /// Kriterium „Ablauf erlaubt“ (Matrix F4): 4:59 min keine neue Frage, nach 5 min eine — seit der Gnadenfrist
    /// (Nacharbeit #35, ADR-0045 Ergänzung Punkt 3) im Hintergrund, die Anfrage ist sofort erlaubt.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn erlaubt_gilt_fuenf_minuten() {
        let test = Testauskunft::mit(&[], &[("x.local", &["127.0.0.1"])]);
        let a = aufbau(Arc::clone(&test));

        assert_eq!(urteil(&a.auskunft, "x.local").await, Some(true));
        a.uhr.stellen(zeit("2026-10-29T12:04:59+01:00"));
        assert_eq!(urteil(&a.auskunft, "x.local").await, Some(true));
        assert_eq!(test.fragen("vorwaerts", "x.local"), 1);
        a.uhr.stellen(zeit("2026-10-29T12:05:00+01:00"));
        assert_eq!(urteil(&a.auskunft, "x.local").await, Some(true));
        bis(|| test.fragen("vorwaerts", "x.local") == 2).await;
    }

    /// Rechenbeispiel „Ablauf nicht erlaubt“ (Matrix C4, C10, B5, F4): t = 0 (1 Frage), 59 s (1), 61 s (2).
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn nicht_erlaubt_gilt_sechzig_sekunden() {
        let test = Testauskunft::mit(&[], &[("drucker.local", &["192.0.2.1"])]);
        let a = aufbau(Arc::clone(&test));

        assert_eq!(urteil(&a.auskunft, "drucker.local").await, Some(false));
        a.uhr.stellen(zeit("2026-10-29T12:00:59+01:00"));
        assert_eq!(urteil(&a.auskunft, "drucker.local").await, Some(false));
        assert_eq!(test.fragen("vorwaerts", "drucker.local"), 1);
        a.uhr.stellen(zeit("2026-10-29T12:01:01+01:00"));
        assert_eq!(urteil(&a.auskunft, "drucker.local").await, Some(false));
        assert_eq!(test.fragen("vorwaerts", "drucker.local"), 2);

        let ohne = a.auskunft.namen_der_adresse(ip("127.0.0.3"), frist()).await;
        assert_eq!(ohne, Some(Vec::new()));
        a.uhr.stellen(zeit("2026-10-29T12:02:00+01:00"));
        a.auskunft.namen_der_adresse(ip("127.0.0.3"), frist()).await;
        assert_eq!(test.fragen("rueckwaerts", "127.0.0.3"), 1);
        a.uhr.stellen(zeit("2026-10-29T12:02:02+01:00"));
        a.auskunft.namen_der_adresse(ip("127.0.0.3"), frist()).await;
        assert_eq!(test.fragen("rueckwaerts", "127.0.0.3"), 2);
    }

    /// Rückt die Testuhr um 0,1 s vor — so viel, wie der Eimer für ein Zeichen braucht (Rate, ADR-0045 Ergänzung
    /// „Karte #35 Nacharbeit“ Punkt 2).
    fn weiter(a: &Aufbau) {
        a.uhr
            .stellen(a.uhr.jetzt() + chrono::Duration::milliseconds(100));
    }

    /// Kriterium „Größe“, Matrix F3 neu (Nacharbeit #35, Fach Vorwärts „erlaubt“ 128): nach 200 erlaubten Namen
    /// ist der erste neu gefragt, die letzten 128 nicht.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn erlaubte_namen_verdraengen_sich_nur_gegenseitig_128_bleiben() {
        let test = Testauskunft::mit(&[], &[]);
        for nummer in 0..200 {
            test.antworten(&format!("e{nummer}.local"), &["127.0.0.1"]);
        }
        let a = aufbau(Arc::clone(&test));
        for nummer in 0..200 {
            weiter(&a);
            assert_eq!(
                urteil(&a.auskunft, &format!("e{nummer}.local")).await,
                Some(true)
            );
        }

        for nummer in 72..200 {
            let name = format!("e{nummer}.local");
            assert_eq!(urteil(&a.auskunft, &name).await, Some(true));
            assert_eq!(test.fragen("vorwaerts", &name), 1, "{name}");
        }
        weiter(&a);
        assert_eq!(urteil(&a.auskunft, "e0.local").await, Some(true));
        assert_eq!(test.fragen("vorwaerts", "e0.local"), 2);
    }

    /// Nacharbeit #35 W1 (Fach Vorwärts „nicht erlaubt“ 96): 200 abgewiesene Namen verdrängen weder `x.local` noch
    /// den Rückwärtseintrag `127.0.0.2 → chat.fritz.box`; der älteste abgewiesene wird neu gefragt.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn abgewiesene_namen_verdraengen_weder_erlaubte_noch_rueckwaertseintraege() {
        let test = Testauskunft::mit(
            &[("127.0.0.2", "chat.fritz.box")],
            &[("x.local", &["127.0.0.1"])],
        );
        let a = aufbau(Arc::clone(&test));
        let router = vec!["chat.fritz.box".to_string()];
        assert_eq!(urteil(&a.auskunft, "x.local").await, Some(true));
        assert_eq!(
            a.auskunft.namen_der_adresse(ip("127.0.0.2"), frist()).await,
            Some(router.clone())
        );
        for nummer in 0..200 {
            weiter(&a);
            assert_eq!(
                urteil(&a.auskunft, &format!("n{nummer}.local")).await,
                Some(false)
            );
        }

        assert_eq!(urteil(&a.auskunft, "x.local").await, Some(true));
        assert_eq!(
            a.auskunft.namen_der_adresse(ip("127.0.0.2"), frist()).await,
            Some(router)
        );
        assert_eq!(test.fragen("vorwaerts", "x.local"), 1);
        assert_eq!(test.fragen("rueckwaerts", "127.0.0.2"), 1);
        weiter(&a);
        urteil(&a.auskunft, "n0.local").await;
        assert_eq!(test.fragen("vorwaerts", "n0.local"), 2);
    }

    /// Nacharbeit #35 W1 (Fächer Rückwärts je 16): 40 lokale Adressen ohne Rückwärtsnamen verdrängen den Eintrag
    /// `127.0.0.2 → chat.fritz.box` nicht; die älteste Adresse ohne Namen wird neu gefragt.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn adressen_ohne_namen_verdraengen_den_rueckwaertseintrag_mit_namen_nicht() {
        let test = Testauskunft::mit(&[("127.0.0.2", "chat.fritz.box")], &[]);
        let a = aufbau(Arc::clone(&test));
        a.auskunft.namen_der_adresse(ip("127.0.0.2"), frist()).await;
        for nummer in 0..40 {
            weiter(&a);
            let ohne = a
                .auskunft
                .namen_der_adresse(ip(&format!("127.0.1.{nummer}")), frist())
                .await;
            assert_eq!(ohne, Some(Vec::new()));
        }

        let namen = a.auskunft.namen_der_adresse(ip("127.0.0.2"), frist()).await;
        assert_eq!(namen, Some(vec!["chat.fritz.box".to_string()]));
        assert_eq!(test.fragen("rueckwaerts", "127.0.0.2"), 1);
        weiter(&a);
        a.auskunft.namen_der_adresse(ip("127.0.1.0"), frist()).await;
        assert_eq!(test.fragen("rueckwaerts", "127.0.1.0"), 2);
    }

    /// Nacharbeit #35 W1: nach einer gemischten Flut sind nie mehr als 256 fertige Einträge da (128 + 96 + 16 + 16).
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn nach_gemischter_flut_hoechstens_256_fertige_eintraege() {
        let test = Testauskunft::mit(&[], &[]);
        for nummer in 0..200 {
            test.antworten(&format!("e{nummer}.local"), &["127.0.0.1"]);
        }
        let a = aufbau(Arc::clone(&test));
        for nummer in 0..200 {
            weiter(&a);
            urteil(&a.auskunft, &format!("e{nummer}.local")).await;
            weiter(&a);
            urteil(&a.auskunft, &format!("n{nummer}.local")).await;
            weiter(&a);
            a.auskunft
                .namen_der_adresse(
                    ip(&format!("127.0.{}.{}", 1 + nummer / 100, nummer % 100)),
                    frist(),
                )
                .await;
            assert!(
                a.auskunft.fertige() <= HOECHSTENS_EINTRAEGE,
                "{}",
                a.auskunft.fertige()
            );
        }
        assert_eq!(HOECHSTENS_EINTRAEGE, 256);
    }

    /// Kriterium „Nacharbeit Karte #35: Rate neuer Auflösungen“: die Testuhr steht, 25 neue Namen → genau 20
    /// Fragen, 5 sofort abgewiesen ohne Frage und ohne Eintrag; Testuhr +1 s, 25 weitere → 10 Fragen.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn ohne_konto_hoechstens_zwanzig_auf_einmal_dann_zehn_je_sekunde() {
        let test = Testauskunft::mit(&[], &[]);
        let erste: Vec<String> = (0..25).map(|nummer| format!("a{nummer}.local")).collect();
        let zweite: Vec<String> = (0..25).map(|nummer| format!("b{nummer}.local")).collect();
        for name in erste.iter().chain(&zweite) {
            test.antworten(name, &["127.0.0.1"]);
        }
        let a = aufbau(Arc::clone(&test));

        let (gefragt, abgewiesen) = welle(&a, &test, &erste).await;
        assert_eq!((gefragt, abgewiesen), (20, 5));
        a.uhr.stellen(a.uhr.jetzt() + chrono::Duration::seconds(1));
        let (gefragt, abgewiesen) = welle(&a, &test, &zweite).await;
        assert_eq!((gefragt, abgewiesen), (10, 15));
    }

    /// Fragt jeden Namen einmal; zählt Fragen und Abweisungen und prüft: eine Abweisung hat weder Frage noch
    /// Eintrag.
    async fn welle(a: &Aufbau, test: &Testauskunft, namen: &[String]) -> (usize, usize) {
        let mut abgewiesen = 0;
        for name in namen {
            match urteil(&a.auskunft, name).await {
                Some(true) => {}
                None => {
                    abgewiesen += 1;
                    assert_eq!(test.fragen("vorwaerts", name), 0, "{name}");
                    assert!(
                        !a.auskunft.hat_eintrag(&Abfrage::Vorwaerts(name.clone())),
                        "{name}"
                    );
                }
                anders => panic!("{name}: {anders:?}"),
            }
        }
        let gefragt = namen
            .iter()
            .map(|name| test.fragen("vorwaerts", name))
            .sum();
        (gefragt, abgewiesen)
    }

    /// Belegt beide Erlaubnisse mit hängenden Fragen nach `h1.local` und `h2.local`; zurück kommen die Läufe.
    async fn erlaubnisse_belegen(
        a: &Aufbau,
        test: &Arc<Testauskunft>,
    ) -> Vec<tokio::task::JoinHandle<Option<bool>>> {
        let mut laeufe = Vec::new();
        for name in ["h1.local", "h2.local"] {
            test.sperren(name);
            let auskunft = Arc::clone(&a.auskunft);
            laeufe.push(tokio::spawn(async move {
                begrenzt(urteil(&auskunft, name)).await
            }));
        }
        bis(|| test.fragen("vorwaerts", "h1.local") + test.fragen("vorwaerts", "h2.local") == 2)
            .await;
        laeufe
    }

    async fn erlaubnisse_freigeben(
        a: &Aufbau,
        test: &Testauskunft,
        laeufe: Vec<tokio::task::JoinHandle<Option<bool>>>,
    ) {
        test.oeffnen("h1.local");
        test.oeffnen("h2.local");
        for lauf in laeufe {
            lauf.await.expect("Lauf");
        }
        bis(|| a.auskunft.erlaubnis.available_permits() == GLEICHZEITIG).await;
    }

    /// Kriterium „Nacharbeit Karte #35: Gnadenfrist“ (ADR-0045 Ergänzung Punkt 3): `x.local` erlaubt bei t = 0; bei
    /// 5:01 min sind beide Erlaubnisse belegt → erlaubt nach < 100 ms, 0 neue Fragen. Sind sie frei, ist die nächste
    /// Anfrage sofort erlaubt und startet genau eine Auffrischung, auch bei leerem Eimer; antwortet sie „fremd“, ist
    /// die folgende Anfrage abgewiesen.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn ein_bestaetigter_name_bleibt_in_der_gnadenfrist_sofort_erlaubt() {
        let test = Testauskunft::mit(&[], &[("x.local", &["127.0.0.1"])]);
        let a = aufbau(Arc::clone(&test));
        assert_eq!(urteil(&a.auskunft, "x.local").await, Some(true));
        a.uhr.stellen(zeit("2026-10-29T12:05:01+01:00"));

        let laeufe = erlaubnisse_belegen(&a, &test).await;
        let beginn = Instant::now();
        assert_eq!(urteil(&a.auskunft, "x.local").await, Some(true));
        assert!(
            beginn.elapsed() < Duration::from_millis(100),
            "{:?}",
            beginn.elapsed()
        );
        assert_eq!(test.fragen("vorwaerts", "x.local"), 1);
        erlaubnisse_freigeben(&a, &test, laeufe).await;

        let leeren: Vec<String> = (0..20)
            .map(|nummer| format!("leer{nummer}.local"))
            .collect();
        for name in &leeren {
            urteil(&a.auskunft, name).await;
        }
        assert!(
            leeren
                .iter()
                .any(|name| test.fragen("vorwaerts", name) == 0),
            "Eimer leer"
        );
        test.sperren("x.local");
        for _ in 0..5 {
            let beginn = Instant::now();
            assert_eq!(urteil(&a.auskunft, "x.local").await, Some(true));
            assert!(
                beginn.elapsed() < Duration::from_millis(100),
                "{:?}",
                beginn.elapsed()
            );
        }
        bis(|| test.fragen("vorwaerts", "x.local") == 2).await;
        test.antworten("x.local", &["192.0.2.1"]);
        test.oeffnen("x.local");
        let abfrage = Abfrage::Vorwaerts("x.local".into());
        bis(|| {
            a.auskunft.erlaubnis.available_permits() == GLEICHZEITIG
                && a.auskunft.ist_fertig(&abfrage)
        })
        .await;

        assert_eq!(urteil(&a.auskunft, "x.local").await, Some(false));
        assert_eq!(test.fragen("vorwaerts", "x.local"), 2);
    }

    /// Gnadenfrist, Ende: ohne Auffrischung ist `x.local` bei 14:59 min noch, bei 15:01 min nicht mehr aus der
    /// Gnadenfrist erlaubt (beide Erlaubnisse belegt, also keine neue Frage); ebenso ein Rückwärtseintrag.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn ohne_auffrischung_endet_die_gnadenfrist_nach_fuenfzehn_minuten() {
        let test = Testauskunft::mit(
            &[("127.0.0.2", "chat.fritz.box")],
            &[("x.local", &["127.0.0.1"])],
        );
        let a = aufbau(Arc::clone(&test));
        let router = Some(vec!["chat.fritz.box".to_string()]);
        assert_eq!(urteil(&a.auskunft, "x.local").await, Some(true));
        assert_eq!(
            a.auskunft.namen_der_adresse(ip("127.0.0.2"), frist()).await,
            router
        );
        let laeufe = erlaubnisse_belegen(&a, &test).await;

        a.uhr.stellen(zeit("2026-10-29T12:14:59+01:00"));
        assert_eq!(urteil(&a.auskunft, "x.local").await, Some(true));
        assert_eq!(
            a.auskunft.namen_der_adresse(ip("127.0.0.2"), frist()).await,
            router
        );
        a.uhr.stellen(zeit("2026-10-29T12:15:01+01:00"));
        assert_eq!(urteil(&a.auskunft, "x.local").await, None);
        assert_eq!(
            a.auskunft.namen_der_adresse(ip("127.0.0.2"), frist()).await,
            None
        );

        assert_eq!(test.fragen("vorwaerts", "x.local"), 1);
        assert_eq!(test.fragen("rueckwaerts", "127.0.0.2"), 1);
        erlaubnisse_freigeben(&a, &test, laeufe).await;
    }

    /// Kriterium Ende (Code-Review #35 A3, ADR-0045 Ergänzung Punkt 3): eine Laufzeit, in der eine Frage hängt
    /// (Testauskunft blockiert bis 10 s), wird fallen gelassen → nach < 200 ms zurück (Mutante „Auflösung wieder in
    /// `spawn_blocking`“).
    #[test]
    fn eine_haengende_aufloesung_haelt_das_ende_der_laufzeit_nicht_auf() {
        let test = Testauskunft::mit(&[], &[]);
        test.sperren("haengt.local");
        let a = aufbau(Arc::clone(&test));
        let laufzeit = crate::start::laufzeit::laufzeit().expect("Laufzeit");
        let kurz = laufzeit.block_on(async {
            let frist = tokio::time::Instant::now() + Duration::from_millis(100);
            urteil_bis(&a.auskunft, "haengt.local", frist).await
        });
        assert_eq!(kurz, None);
        assert_eq!(test.fragen("vorwaerts", "haengt.local"), 1);

        let beginn = Instant::now();
        drop(laufzeit);
        let ende = beginn.elapsed();

        test.oeffnen("haengt.local");
        assert!(ende < Duration::from_millis(200), "{ende:?}");
    }

    /// Scheitert der Start des Fadens, gilt das als „ohne Antwort“ ohne Eintrag; die Erlaubnis ist sofort zurück und
    /// die nächste Anfrage fragt neu.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn scheitert_der_fadenstart_gilt_es_als_ohne_antwort_ohne_eintrag() {
        let test = Testauskunft::mit(&[], &[("x.local", &["127.0.0.1"])]);
        let mut a = aufbau(Arc::clone(&test));
        Arc::get_mut(&mut a.auskunft).expect("allein").fadenstart =
            |_| Err(std::io::Error::other("kein Faden"));

        assert_eq!(urteil(&a.auskunft, "x.local").await, None);

        assert_eq!(test.alle_fragen(), 0);
        assert!(
            !a.auskunft
                .hat_eintrag(&Abfrage::Vorwaerts("x.local".into()))
        );
        assert_eq!(a.auskunft.erlaubnis.available_permits(), GLEICHZEITIG);
        Arc::get_mut(&mut a.auskunft).expect("allein").fadenstart = faden_starten;
        assert_eq!(urteil(&a.auskunft, "x.local").await, Some(true));
    }

    /// Rechenbeispiel „Ohne Erlaubnis keine Auflösung“ (E3, Matrix F6): zwei laufen; `neu.local` sofort abgewiesen,
    /// 0 Fragen, kein Eintrag; danach 1 Frage, erlaubt.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn ohne_erlaubnis_keine_frage_und_kein_cache_eintrag() {
        let test = Testauskunft::mit(&[], &[("neu.local", &["127.0.0.1"])]);
        test.sperren("a.local");
        test.sperren("b.local");
        let a = aufbau(Arc::clone(&test));
        let mut laeufe = Vec::new();
        for name in ["a.local", "b.local"] {
            let auskunft = Arc::clone(&a.auskunft);
            laeufe.push(tokio::spawn(async move {
                begrenzt(urteil(&auskunft, name)).await
            }));
        }
        bis(|| test.alle_fragen() == 2).await;

        assert_eq!(urteil(&a.auskunft, "neu.local").await, None);
        assert_eq!(test.fragen("vorwaerts", "neu.local"), 0);
        assert!(
            !a.auskunft
                .hat_eintrag(&Abfrage::Vorwaerts("neu.local".into()))
        );

        test.oeffnen("a.local");
        test.oeffnen("b.local");
        for lauf in laeufe {
            lauf.await.expect("Lauf");
        }
        bis(|| a.auskunft.erlaubnis.available_permits() == GLEICHZEITIG).await;
        assert_eq!(urteil(&a.auskunft, "neu.local").await, Some(true));
        assert_eq!(test.fragen("vorwaerts", "neu.local"), 1);
    }

    /// Matrix F7 (E8): die Rückwärtsfrage hängt an der lokalen Adresse — 300 Anfragen, die hängende Frage läuft
    /// einmal, jede Anfrage ist nach höchstens 1 s (+100 ms) abgewiesen.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn rueckwaerts_laesst_sich_nicht_fluten() {
        let test = Testauskunft::mit(&[("127.0.0.1", "localhost")], &[]);
        test.sperren("127.0.0.1");
        let a = aufbau(Arc::clone(&test));
        let beginn = Instant::now();
        let mut laeufe = Vec::new();
        for _ in 0..300 {
            let auskunft = Arc::clone(&a.auskunft);
            laeufe.push(tokio::spawn(async move {
                let start = Instant::now();
                let namen = begrenzt(auskunft.namen_der_adresse(ip("127.0.0.1"), frist())).await;
                (namen, start.elapsed())
            }));
        }
        for lauf in laeufe {
            let (namen, dauer) = lauf.await.expect("Lauf");
            assert_eq!(namen, None);
            assert!(dauer <= Duration::from_millis(1100), "{dauer:?}");
        }

        assert!(beginn.elapsed() < Duration::from_secs(3));
        assert_eq!(test.fragen("rueckwaerts", "127.0.0.1"), 1);
        assert_eq!(test.alle_fragen(), 1);
        test.oeffnen("127.0.0.1");
    }

    /// E6: vor jeder Frage an die Namensauskunft genau eine DEBUG-Zeile mit `art` und `schluessel`; ein Treffer
    /// im Cache schreibt keine.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn jede_frage_ist_davor_eine_debug_zeile_ein_treffer_keine() {
        let test = Testauskunft::mit(
            &[("127.0.0.1", "localhost")],
            &[("chat.local", &["192.0.2.1", "127.0.0.1"])],
        );
        let a = aufbau(Arc::clone(&test));

        urteil(&a.auskunft, "chat.local").await;
        urteil(&a.auskunft, "chat.local").await;
        a.auskunft
            .namen_der_adresse(ip("::ffff:127.0.0.1"), frist())
            .await;
        a.auskunft.namen_der_adresse(ip("127.0.0.1"), frist()).await;

        let debug: Vec<String> = a
            .speicher
            .zeilen()
            .into_iter()
            .filter(|zeile| zeile.contains(" DEBUG grenzschutz "))
            .map(|zeile| zeile.split_once(' ').expect("Zeit").1.to_string())
            .collect();
        assert_eq!(
            debug,
            [
                "DEBUG grenzschutz Namensauskunft gefragt | art=vorwaerts schluessel=chat.local",
                "DEBUG grenzschutz Namensauskunft gefragt | art=eigen schluessel=192.0.2.1",
                "DEBUG grenzschutz Namensauskunft gefragt | art=eigen schluessel=127.0.0.1",
                "DEBUG grenzschutz Namensauskunft gefragt | art=rueckwaerts schluessel=127.0.0.1",
            ]
        );
        assert_eq!(test.alle_fragen(), 4);
    }

    /// Information „… erlaubt“ einmal je Cache-Eintrag und Name; nach dem Ablauf wieder einmal.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn erstmals_gemeldet_einmal_je_eintrag_und_name() {
        let test = Testauskunft::mit(&[], &[("x.local", &["127.0.0.1"])]);
        let a = aufbau(Arc::clone(&test));
        let abfrage = Abfrage::Vorwaerts("x.local".into());

        assert!(!a.auskunft.erstmals_gemeldet(&abfrage, "x.local"));
        urteil(&a.auskunft, "x.local").await;
        assert!(a.auskunft.erstmals_gemeldet(&abfrage, "x.local"));
        assert!(!a.auskunft.erstmals_gemeldet(&abfrage, "x.local"));
        a.uhr.stellen(zeit("2026-10-29T12:05:00+01:00"));
        urteil(&a.auskunft, "x.local").await;
        // Die Auffrischung läuft im Hintergrund (Gnadenfrist); ihr Ergebnis ist ein neuer Eintrag.
        bis(|| {
            test.fragen("vorwaerts", "x.local") == 2
                && a.auskunft.erlaubnis.available_permits() == GLEICHZEITIG
        })
        .await;
        assert!(a.auskunft.erstmals_gemeldet(&abfrage, "x.local"));
    }
}
