//! Auskunft mit Cache (ADR-0045 Regel 6 mit Ergänzungen E3, E6, E8): jede Frage an die Namensauskunft läuft
//! blockierend in `spawn_blocking`, höchstens zwei gleichzeitig; eine Anfrage wartet höchstens eine Sekunde, danach
//! läuft die Auflösung zu Ende und füllt den Cache. Einziger Schreiber des Caches ist diese Auskunft, im Auftrag
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

/// Höchstens so viele Auflösungen laufen gleichzeitig (eigene Semaphore, nicht die Werkbank).
pub const GLEICHZEITIG: usize = 2;
/// So lange wartet eine Anfrage höchstens auf eine Auflösung.
pub const WARTEZEIT: Duration = Duration::from_secs(1);
/// So lange gilt ein Ergebnis „erlaubt“ bzw. „nicht erlaubt“ (auch Fehler und Zeitgrenze des Resolvers).
pub const ERLAUBT_GILT: Duration = Duration::from_secs(5 * 60);
pub const NICHT_ERLAUBT_GILT: Duration = Duration::from_secs(60);
/// Höchstens so viele Einträge; der älteste fällt.
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
}

enum Nachsehen {
    Treffer(Ergebnis),
    Warten(watch::Receiver<Option<Ergebnis>>),
    KeineErlaubnis,
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
        }
    }

    /// Die Rückwärtsnamen der (kanonischen) Adresse — `None`, wenn keine Antwort binnen der Wartezeit kam oder
    /// keine Auflösung starten durfte.
    pub async fn namen_der_adresse(&self, adresse: IpAddr) -> Option<Vec<String>> {
        let adresse = adresse.to_canonical();
        let arbeit: Arbeit = Box::new(move |port| Ergebnis::Namen(port.namen_der_adresse(adresse)));
        match self.erfragen(Abfrage::Rueckwaerts(adresse), arbeit).await {
            Some(Ergebnis::Namen(namen)) => Some(namen),
            _ => None,
        }
    }

    /// Das Urteil über einen Namen; `urteilen` fragt die Namensauskunft (vorwärts, eigen?) und entscheidet.
    pub async fn urteil_ueber(
        &self,
        name: &str,
        urteilen: impl FnOnce(&dyn Namensauskunft) -> bool + Send + 'static,
    ) -> Option<bool> {
        let arbeit: Arbeit = Box::new(move |port| Ergebnis::Erlaubt(urteilen(port)));
        match self
            .erfragen(Abfrage::Vorwaerts(name.to_string()), arbeit)
            .await
        {
            Some(Ergebnis::Erlaubt(erlaubt)) => Some(erlaubt),
            _ => None,
        }
    }

    /// `true` genau beim ersten Mal je gültigem Cache-Eintrag und Name — für die Information „… erlaubt“.
    pub fn erstmals_gemeldet(&self, abfrage: &Abfrage, name: &str) -> bool {
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

    /// Treffer im Cache, sonst auf eine laufende Auflösung warten oder eine starten — höchstens `WARTEZEIT`.
    async fn erfragen(&self, abfrage: Abfrage, arbeit: Arbeit) -> Option<Ergebnis> {
        let mut empfang = match self.nachsehen_oder_starten(&abfrage, arbeit) {
            Nachsehen::Treffer(ergebnis) => return Some(ergebnis),
            Nachsehen::KeineErlaubnis => return None,
            Nachsehen::Warten(empfang) => empfang,
        };
        let warten = async {
            empfang
                .wait_for(Option::is_some)
                .await
                .ok()
                .and_then(|ergebnis| ergebnis.clone())
        };
        match tokio::time::timeout(WARTEZEIT, warten).await {
            Ok(ergebnis) => ergebnis,
            Err(_) => {
                self.zu_langsam(&abfrage);
                None
            }
        }
    }

    /// Unter der Sperre des Caches: gültiger Eintrag → Treffer; laufende Auflösung → warten; sonst eine Erlaubnis
    /// ohne Warten holen (E3) — ohne Erlaubnis kein Eintrag, keine Frage.
    fn nachsehen_oder_starten(&self, abfrage: &Abfrage, arbeit: Arbeit) -> Nachsehen {
        let jetzt = self.innen.uhr.jetzt();
        let mut cache = self
            .innen
            .cache
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        match cache.eintraege.get(abfrage).map(|eintrag| &eintrag.stand) {
            Some(Stand::Fertig { ergebnis, bis, .. }) if jetzt < *bis => {
                return Nachsehen::Treffer(ergebnis.clone());
            }
            Some(Stand::Laeuft(empfang)) => return Nachsehen::Warten(empfang.clone()),
            _ => {}
        }
        let Ok(erlaubnis) = Arc::clone(&self.erlaubnis).try_acquire_owned() else {
            return Nachsehen::KeineErlaubnis;
        };
        let (senden, empfang) = watch::channel(None);
        cache.eintragen(abfrage.clone(), Stand::Laeuft(empfang.clone()));
        drop(cache);
        let innen = Arc::clone(&self.innen);
        let abfrage = abfrage.clone();
        tokio::task::spawn_blocking(move || innen.aufloesen(abfrage, arbeit, senden, erlaubnis));
        Nachsehen::Warten(empfang)
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
                },
            );
        // Erst die Erlaubnis zurück, dann melden: wer auf das Ergebnis wartet und gleich die nächste Auflösung
        // braucht, findet sie frei.
        drop(erlaubnis);
        let _ = senden.send(Some(ergebnis));
    }
}

impl Cache {
    /// Setzt den Stand eines Schlüssels; ein neuer Schlüssel bekommt die nächste Nummer, und über
    /// `HOECHSTENS_EINTRAEGE` fällt der älteste fertige Eintrag.
    fn eintragen(&mut self, abfrage: Abfrage, stand: Stand) {
        if let Some(eintrag) = self.eintraege.get_mut(&abfrage)
            && matches!(eintrag.stand, Stand::Laeuft(_))
        {
            eintrag.stand = stand;
            return;
        }
        self.naechste += 1;
        self.eintraege.insert(
            abfrage,
            Eintrag {
                stand,
                nummer: self.naechste,
            },
        );
        while self.eintraege.len() > HOECHSTENS_EINTRAEGE {
            let aeltester = self
                .eintraege
                .iter()
                .filter(|(_, eintrag)| matches!(eintrag.stand, Stand::Fertig { .. }))
                .min_by_key(|(_, eintrag)| eintrag.nummer)
                .map(|(abfrage, _)| abfrage.clone());
            match aeltester {
                Some(abfrage) => self.eintraege.remove(&abfrage),
                None => break,
            };
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
    use crate::protokoll::senke::Speicher;
    use crate::protokoll::testhilfe::{im_speicher, zeit};
    use crate::uhr::GestellteUhr;
    use std::sync::Condvar;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Instant;

    /// Testauskunft: feste Antworten, je Schlüssel ein Tor (gesperrt blockiert die Frage, bis es öffnet),
    /// zählt Fragen und die höchste Zahl gleichzeitiger Fragen.
    #[derive(Default)]
    struct Testauskunft {
        rueckwaerts: HashMap<IpAddr, Vec<String>>,
        vorwaerts: HashMap<String, Vec<IpAddr>>,
        gesperrt: Mutex<HashSet<String>>,
        tor: Condvar,
        laufend: AtomicUsize,
        hoechstens: AtomicUsize,
        fragen: Mutex<Vec<String>>,
    }

    impl Testauskunft {
        fn mit(rueckwaerts: &[(&str, &str)], vorwaerts: &[(&str, &[&str])]) -> Arc<Testauskunft> {
            Arc::new(Testauskunft {
                rueckwaerts: rueckwaerts
                    .iter()
                    .map(|(adresse, name)| (ip(adresse), vec![name.to_string()]))
                    .collect(),
                vorwaerts: vorwaerts
                    .iter()
                    .map(|(name, adressen)| {
                        (name.to_string(), adressen.iter().map(|a| ip(a)).collect())
                    })
                    .collect(),
                ..Testauskunft::default()
            })
        }

        fn sperren(&self, schluessel: &str) {
            self.gesperrt
                .lock()
                .expect("Tor")
                .insert(schluessel.to_string());
        }

        fn oeffnen(&self, schluessel: &str) {
            self.gesperrt.lock().expect("Tor").remove(schluessel);
            self.tor.notify_all();
        }

        fn fragen(&self, art: &str, schluessel: &str) -> usize {
            let gesucht = format!("{art} {schluessel}");
            self.fragen
                .lock()
                .expect("Fragen")
                .iter()
                .filter(|frage| **frage == gesucht)
                .count()
        }

        fn alle_fragen(&self) -> usize {
            self.fragen.lock().expect("Fragen").len()
        }

        fn frage(&self, art: &str, schluessel: &str) {
            self.fragen
                .lock()
                .expect("Fragen")
                .push(format!("{art} {schluessel}"));
            let jetzt = self.laufend.fetch_add(1, Ordering::SeqCst) + 1;
            self.hoechstens.fetch_max(jetzt, Ordering::SeqCst);
            let mut gesperrt = self.gesperrt.lock().expect("Tor");
            while gesperrt.contains(schluessel) {
                gesperrt = self.tor.wait(gesperrt).expect("Tor");
            }
            self.laufend.fetch_sub(1, Ordering::SeqCst);
        }
    }

    impl Namensauskunft for Testauskunft {
        fn namen_der_adresse(&self, adresse: IpAddr) -> Vec<String> {
            self.frage("rueckwaerts", &adresse.to_string());
            self.rueckwaerts.get(&adresse).cloned().unwrap_or_default()
        }

        fn adressen_des_namens(&self, name: &str) -> Vec<IpAddr> {
            self.frage("vorwaerts", name);
            self.vorwaerts.get(name).cloned().unwrap_or_default()
        }

        fn ist_eigene_adresse(&self, adresse: IpAddr) -> bool {
            self.frage("eigen", &adresse.to_string());
            adresse.is_loopback()
        }
    }

    fn ip(text: &str) -> IpAddr {
        text.parse().expect("Adresse")
    }

    const START: &str = "2026-10-29T12:00:00+01:00";

    struct Aufbau {
        auskunft: Arc<Auskunft>,
        test: Arc<Testauskunft>,
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
            test,
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
        let eigener = name.to_string();
        auskunft
            .urteil_ueber(name, move |port| eigen_urteil(port, &eigener))
            .await
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
            laeufe.push(tokio::spawn(async move { urteil(&auskunft, name).await }));
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
        let anfrage_a = a.auskunft.namen_der_adresse(ip("192.168.178.65")).await;
        let gewartet = beginn.elapsed();

        assert_eq!(anfrage_a, None);
        assert!(gewartet >= WARTEZEIT, "{gewartet:?}");
        assert!(gewartet < Duration::from_millis(1500), "{gewartet:?}");
        test.oeffnen("192.168.178.65");
        let abfrage = Abfrage::Rueckwaerts(ip("192.168.178.65"));
        bis(|| a.auskunft.ist_fertig(&abfrage)).await;

        let anfrage_b = a.auskunft.namen_der_adresse(ip("192.168.178.65")).await;

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

    /// Kriterium „Ablauf erlaubt“ (Matrix F4): 4:59 min keine neue Frage, nach 5 min eine.
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
        assert_eq!(test.fragen("vorwaerts", "x.local"), 2);
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

        let ohne = a.auskunft.namen_der_adresse(ip("127.0.0.3")).await;
        assert_eq!(ohne, Some(Vec::new()));
        a.uhr.stellen(zeit("2026-10-29T12:02:00+01:00"));
        a.auskunft.namen_der_adresse(ip("127.0.0.3")).await;
        assert_eq!(test.fragen("rueckwaerts", "127.0.0.3"), 1);
        a.uhr.stellen(zeit("2026-10-29T12:02:02+01:00"));
        a.auskunft.namen_der_adresse(ip("127.0.0.3")).await;
        assert_eq!(test.fragen("rueckwaerts", "127.0.0.3"), 2);
    }

    /// Kriterium „Größe“ (Matrix F3): nach 257 Schlüsseln ist der erste nicht mehr im Cache, der letzte schon.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn hoechstens_256_eintraege_der_aelteste_faellt() {
        let test = Testauskunft::mit(&[], &[]);
        let a = aufbau(Arc::clone(&test));
        for nummer in 0..257 {
            urteil(&a.auskunft, &format!("n{nummer}.local")).await;
        }

        urteil(&a.auskunft, "n256.local").await;
        urteil(&a.auskunft, "n1.local").await;
        urteil(&a.auskunft, "n0.local").await;

        assert_eq!(test.fragen("vorwaerts", "n256.local"), 1);
        assert_eq!(test.fragen("vorwaerts", "n1.local"), 1);
        assert_eq!(test.fragen("vorwaerts", "n0.local"), 2);
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
            laeufe.push(tokio::spawn(async move { urteil(&auskunft, name).await }));
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
                let namen = auskunft.namen_der_adresse(ip("127.0.0.1")).await;
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
        a.auskunft.namen_der_adresse(ip("::ffff:127.0.0.1")).await;
        a.auskunft.namen_der_adresse(ip("127.0.0.1")).await;

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
        assert!(a.auskunft.erstmals_gemeldet(&abfrage, "x.local"));
    }
}
