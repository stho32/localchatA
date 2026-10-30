//! Ablauf eines Warte-Aufrufs (ADR-0054 Regel 3 und 5, B0738): abonnieren → Marke `H` unter der Schreibsperre →
//! nachholen bis `H` → den Bus filtern. Die Reihenfolge **ist** die Verlustfreiheit: was bis `H` vergeben ist, steht
//! im Index; was danach kommt, hat eine Zeit über `H` und liegt im Abonnement. Vom Bus zählt nur, was über dem
//! verarbeiteten Stand liegt (Dublettenregel); verliert das Abonnement Ereignisse, wird ab dem Stand nachgeholt.

use crate::anwesenheit::status::Status;
use crate::gespraeche::nachholen::{NACHHOLBUDGET, Nachgeholt, Nachholbudget, lieferbytes};
use crate::gespraeche::treffer::{Filter, Treffer, treffer};
use crate::gespraeche::{Gespraech, Gespraeche, Kennung, Nachricht, sichtbar_fuer};
use crate::live::bus::{Abonnement, Bus, Empfang, Ereignis};
use crate::protokoll::{Bereich, Kontext, Protokoll, Schluessel, Stufe};
use crate::start::ende::Ende as Programmende;
use crate::uhr::Zeit;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

/// Port des Nachholens (ADR-0054 Ergänzung „Nachholen von `warten` …“ Regel 9): führt die Arbeit des Nachholens
/// auf der Nachholbank aus und rechnet sie ab — `warten` kennt weder Bank noch Konto. `Fehler` ist der Abbruch
/// der Bank.
pub trait Nachholplatz: Send + Sync {
    type Fehler: Send;

    /// Führt `arbeit` auf einem Platz der Bank aus; der Platz lebt bis zum Ende der Arbeit.
    fn ausfuehren<F>(&self, arbeit: F) -> Nacharbeit<Self::Fehler>
    where
        F: FnOnce() -> Option<Nachgeholt> + Send + 'static;
}

/// Die laufende Arbeit eines Nachholplatzes; besitzt alles, was sie braucht (`'static`), damit `warten` in jedem
/// Executor `Send` bleibt.
pub type Nacharbeit<E> = Pin<Box<dyn Future<Output = Result<Option<Nachgeholt>, E>> + Send>>;

/// Ein Warte-Aufruf: ab `seit` (ohne: ab der Marke), höchstens `bis`, für `wartender` unter `filter`.
#[derive(Clone)]
pub struct Warteauftrag {
    pub seit: Option<Zeit>,
    pub bis: Duration,
    pub wartender: Kennung,
    pub filter: Filter,
    /// Das Ende des Programms: der Aufruf endet sofort mit `Ende::Programmende` (Regel 9).
    pub programmende: Programmende,
    /// Das Betriebsprotokoll: ein überholtes Abonnement schreibt eine DEBUG-Zeile (Regel 3.6 und 10).
    pub protokoll: Arc<Protokoll>,
    /// Ob die Sitzung des Wartenden noch im Bestand ist — nach einem überholten Abonnement gefragt, weil ihr `weg`
    /// unter den verlorenen Ereignissen sein kann (Code-Review #17 F0169 A1).
    pub sitzung_besteht: Arc<dyn Fn() -> bool + Send + Sync>,
}

/// DEBUG `live` je überholtem Abonnement, genau eine Zeile: wer wartet und wie viele Ereignisse fehlten — nie Inhalt
/// oder Token (ADR-0054 Regel 3.6).
fn ueberholt_melden(auftrag: &Warteauftrag, verpasst: u64) {
    auftrag.protokoll.eintragen(
        Stufe::Debug,
        Bereich::Live,
        "Abonnement überholt — nachgeholt",
        Kontext::neu()
            .mit(Schluessel::Kennung, auftrag.wartender.als_text())
            .mit(Schluessel::Anzahl, verpasst),
    );
}

/// Warum der Aufruf endet (Regel 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ende {
    /// Treffer geliefert, nichts mehr aufgelaufen.
    Treffer,
    /// `bis` abgelaufen, nichts getroffen.
    Frist,
    /// Ein Budget ist erschöpft — sofort wieder rufen.
    Weitere,
    /// Das Programm endet (SIGTERM, Strg+C) — kein Treffer, nicht mehr rufen, bis es wieder läuft.
    Programmende,
    /// Der Wartende hat verlassen (`weg` der eigenen Sitzung) — das Token gilt nicht mehr.
    Verlassen,
}

/// Was der Aufruf liefert: Treffer in Zeitfolge, den verarbeiteten Stand (`None` nur, solange es noch keine
/// Nachricht gibt) und den Grund des Endes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Warteergebnis {
    pub treffer: Vec<Treffer>,
    pub seit: Option<Zeit>,
    pub ende: Ende,
}

/// Die Schritte des Ablaufs, an denen ein Beobachter gerufen wird (feste Verschränkungen im Test, Probe W5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Schritt {
    /// Der Bus ist abonniert, die Marke noch nicht gelesen.
    Abonniert,
    /// Die Marke ist gelesen, nachgeholt ist noch nicht (auf dem Nachholplatz gerufen).
    MarkeGelesen,
    /// Nachgeholt ist, auf den Bus wird noch nicht gewartet.
    Nachgeholt,
}

/// Beobachter der Schritte; im Programm tut er nichts.
pub type Beobachter = Arc<dyn Fn(Schritt) + Send + Sync>;

/// Wartet nach Regel 3; `Err` nur, wenn die Arbeit auf dem Nachholplatz abbrach.
pub async fn warten<P: Nachholplatz>(
    gespraeche: Arc<Gespraeche>,
    bus: &Bus,
    platz: &P,
    auftrag: Warteauftrag,
    beobachter: Beobachter,
) -> Result<Warteergebnis, P::Fehler> {
    let frist = tokio::time::Instant::now() + auftrag.bis;
    let mut abonnement = bus.abonnieren();
    beobachter(Schritt::Abonniert);
    let mut sammlung = Sammlung {
        treffer: Vec::new(),
        bytes: 0,
        stand: auftrag.seit,
        ab_anfang: false,
    };
    let nachgeholt = nachholen(&gespraeche, platz, &auftrag, &sammlung, &beobachter).await?;
    beobachter(Schritt::Nachgeholt);
    // Ohne `seit` auf leerer Ablage gibt es nichts „davor“: was später im Index steht, ist neu.
    sammlung.ab_anfang = auftrag.seit.is_none() && nachgeholt.is_none();
    let mut ende = sammlung.aufnehmen(nachgeholt);
    let mut programmende = auftrag.programmende.clone();
    loop {
        match ende {
            Some(Ende::Treffer) => {
                let ende = sammlung.mitnehmen(&mut abonnement, &auftrag);
                return Ok(sammlung.ergebnis(ende));
            }
            Some(ende) => return Ok(sammlung.ergebnis(ende)),
            None => {}
        }
        let empfang = tokio::select! {
            biased;
            () = programmende.abwarten() => return Ok(sammlung.ergebnis(Ende::Programmende)),
            () = tokio::time::sleep_until(frist) => return Ok(sammlung.ergebnis(Ende::Frist)),
            empfang = abonnement.naechstes() => empfang,
        };
        ende = match empfang {
            None => Some(Ende::Frist),
            Some(Empfang::Ereignis(Ereignis::Nachricht {
                gespraech,
                nachricht,
            })) => sammlung
                .vom_bus(&gespraech, nachricht, &auftrag)
                .or_else(|| (!sammlung.treffer.is_empty()).then_some(Ende::Treffer)),
            Some(Empfang::Ereignis(Ereignis::Anwesenheit(wechsel))) => {
                let verlassen = wechsel.teilnehmer.status == Status::Weg
                    && wechsel.teilnehmer.kennung == auftrag.wartender;
                verlassen.then_some(Ende::Verlassen)
            }
            Some(Empfang::Ereignis(_)) => None,
            Some(Empfang::NeuerStand { verpasst }) => {
                ueberholt_melden(&auftrag, verpasst);
                if !(auftrag.sitzung_besteht)() {
                    return Ok(sammlung.ergebnis(Ende::Verlassen));
                }
                let nachgeholt =
                    nachholen(&gespraeche, platz, &auftrag, &sammlung, &beobachter).await?;
                sammlung.aufnehmen(nachgeholt)
            }
        };
    }
}

/// Liest die Marke `H` unter der Schreibsperre und holt `(Stand, H]` nach — auf dem Nachholplatz, mit dem Budget,
/// das der Sammlung noch bleibt. Ohne Stand gilt „ab jetzt“: der Stand wird die sichtbare Marke `Hs ≤ H`.
async fn nachholen<P: Nachholplatz>(
    gespraeche: &Arc<Gespraeche>,
    platz: &P,
    auftrag: &Warteauftrag,
    sammlung: &Sammlung,
    beobachter: &Beobachter,
) -> Result<Option<Nachgeholt>, P::Fehler> {
    let gespraeche = Arc::clone(gespraeche);
    let beobachter = Arc::clone(beobachter);
    let stand = sammlung.stand;
    let ab_anfang = sammlung.ab_anfang;
    let budget = sammlung.restbudget();
    let wartender = auftrag.wartender.clone();
    let filter = auftrag.filter.clone();
    platz
        .ausfuehren(move || {
            let marke = gespraeche.letzte_zeit();
            beobachter(Schritt::MarkeGelesen);
            let marke = marke?;
            // Ohne `seit` gilt „ab jetzt“ — aber nur über das, was der Wartende sehen darf: die sichtbare Marke
            // (ADR-0055 Regel 8). Sieht er noch nichts, gilt der Aufruf wie einer auf leerer Ablage.
            let ab = match stand {
                Some(stand) => stand,
                None if ab_anfang => anfang(),
                None => gespraeche.sichtbare_marke(&wartender, marke)?,
            };
            Some(gespraeche.nachholen(ab, marke, &wartender, &filter, budget))
        })
        .await
}

/// Was ein Aufruf bisher gesammelt hat: Treffer, ihre Lieferbytes und den verarbeiteten Stand.
struct Sammlung {
    treffer: Vec<Treffer>,
    bytes: usize,
    stand: Option<Zeit>,
    /// Der Aufruf begann ohne `seit` auf leerer Ablage: ohne Stand wird ab dem Anfang nachgeholt, nicht ab der Marke.
    ab_anfang: bool,
}

/// Vor jeder Nachricht.
fn anfang() -> Zeit {
    chrono::DateTime::<chrono::Utc>::MIN_UTC.fixed_offset()
}

impl Sammlung {
    /// Das Budget, das nach den gesammelten Treffern bleibt.
    fn restbudget(&self) -> Nachholbudget {
        Nachholbudget {
            nachrichten: NACHHOLBUDGET.nachrichten - self.treffer.len(),
            bytes: NACHHOLBUDGET.bytes.saturating_sub(self.bytes),
            ..NACHHOLBUDGET
        }
    }

    /// Nimmt ein Nachholen auf; endet der Aufruf damit, der Grund. Ohne Marke (noch keine Nachricht) nichts.
    fn aufnehmen(&mut self, nachgeholt: Option<Nachgeholt>) -> Option<Ende> {
        let nachgeholt = nachgeholt?;
        if nachgeholt.stand != anfang() {
            self.stand = Some(nachgeholt.stand);
        }
        for treffer in nachgeholt.treffer {
            self.bytes += lieferbytes(&treffer.nachricht);
            self.treffer.push(treffer);
        }
        if nachgeholt.weitere {
            Some(Ende::Weitere)
        } else if self.treffer.is_empty() {
            None
        } else {
            Some(Ende::Treffer)
        }
    }

    /// Eine Nachricht vom Bus, soweit der Wartende das Gespräch sehen darf — sonst nichts: ein Ereignis eines
    /// fremden Gesprächs rückt den verarbeiteten Stand nie vor und ist für den Wartenden nicht geschehen
    /// (ADR-0055 Regel 8).
    fn vom_bus(
        &mut self,
        gespraech: &Gespraech,
        nachricht: Nachricht,
        auftrag: &Warteauftrag,
    ) -> Option<Ende> {
        if !sichtbar_fuer(gespraech, &auftrag.wartender, false) {
            return None;
        }
        self.pruefen(gespraech, nachricht, auftrag)
    }

    /// Prüft eine Nachricht vom Bus: nur über dem Stand zählt sie (Dublettenregel), und sie rückt ihn vor. Passt ein
    /// Treffer nicht mehr ins Budget, bleibt der Stand davor und der Aufruf endet mit `Weitere`.
    fn pruefen(
        &mut self,
        gespraech: &Gespraech,
        nachricht: Nachricht,
        auftrag: &Warteauftrag,
    ) -> Option<Ende> {
        if self.stand.is_some_and(|stand| nachricht.zeit <= stand) {
            return None;
        }
        if let Some(anlass) = treffer(gespraech, &nachricht, &auftrag.wartender, &auftrag.filter) {
            let bytes = lieferbytes(&nachricht);
            let voll = self.treffer.len() == NACHHOLBUDGET.nachrichten
                || (!self.treffer.is_empty() && self.bytes + bytes > NACHHOLBUDGET.bytes);
            if voll {
                return Some(Ende::Weitere);
            }
            self.bytes += bytes;
            self.stand = Some(nachricht.zeit);
            self.treffer.push(Treffer {
                gespraech: gespraech.clone(),
                anlass,
                nachricht,
            });
            return None;
        }
        self.stand = Some(nachricht.zeit);
        None
    }

    /// Nach dem ersten Treffer: alles schon Empfangene ohne Warten mitnehmen, bis das Abonnement leer ist oder ein
    /// Budget endet. Hat das Abonnement inzwischen Ereignisse verloren, endet es mit `Weitere` am Stand — der nächste
    /// Aufruf holt sie aus dem Index.
    fn mitnehmen(&mut self, abonnement: &mut Abonnement, auftrag: &Warteauftrag) -> Ende {
        loop {
            match abonnement.sofort() {
                None => return Ende::Treffer,
                Some(Empfang::NeuerStand { verpasst }) => {
                    ueberholt_melden(auftrag, verpasst);
                    if !(auftrag.sitzung_besteht)() {
                        return Ende::Verlassen;
                    }
                    return Ende::Weitere;
                }
                Some(Empfang::Ereignis(Ereignis::Nachricht {
                    gespraech,
                    nachricht,
                })) => {
                    if let Some(ende) = self.vom_bus(&gespraech, nachricht, auftrag) {
                        return ende;
                    }
                }
                Some(Empfang::Ereignis(_)) => {}
            }
        }
    }

    fn ergebnis(self, ende: Ende) -> Warteergebnis {
        Warteergebnis {
            treffer: self.treffer,
            seit: self.stand,
            ende,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Nachholplatz im Test: die Arbeit auf einem Blocking-Thread, ohne Bank und Konto.
    struct Blockierend;

    impl Nachholplatz for Blockierend {
        type Fehler = tokio::task::JoinError;

        fn ausfuehren<F>(&self, arbeit: F) -> Nacharbeit<Self::Fehler>
        where
            F: FnOnce() -> Option<Nachgeholt> + Send + 'static,
        {
            Box::pin(tokio::task::spawn_blocking(arbeit))
        }
    }
    use crate::gespraeche::{Angehaengt, Art, Logablage, Speicherablage};
    use crate::uhr::GestellteUhr;
    use chrono::NaiveDate;
    use std::sync::Mutex;

    fn zeit(text: &str) -> Zeit {
        Zeit::parse_from_rfc3339(text).expect("Testzeit")
    }

    fn kennung(text: &str) -> Kennung {
        Kennung::bilden(text).expect("Kennung")
    }

    fn anna() -> Kennung {
        kennung("anna@127.0.0.1")
    }

    fn agent() -> Kennung {
        kennung("agent@127.0.0.1")
    }

    /// Gespräche mit den Channels `a` und `b` auf der Ablage, deren Nachrichten auf `bus` veröffentlicht werden.
    fn gespraeche_mit(ablage: Arc<dyn Logablage>, bus: &Arc<Bus>) -> Arc<Gespraeche> {
        let uhr = Arc::new(GestellteUhr::neu(zeit("2026-10-30T12:00:00.000+01:00")));
        Arc::new(
            Gespraeche::laden(ablage, uhr)
                .expect("geladen")
                .mit_nachricht_veroeffentlichen(crate::live::bus::nachrichten_veroeffentlichen(
                    bus,
                )),
        )
    }

    fn speicher_ablage() -> Arc<Speicherablage> {
        speicher()
    }

    fn speicher() -> Arc<Speicherablage> {
        let ablage = Arc::new(Speicherablage::neu());
        ablage.gespraech_anlegen(&Gespraech::channel("a"));
        ablage.gespraech_anlegen(&Gespraech::channel("b"));
        ablage
    }

    fn senden(gespraeche: &Gespraeche, channel: &str, text: &str) -> Zeit {
        gespraeche
            .senden(&Gespraech::channel(channel), &anna(), Art::Mensch, text)
            .expect("gesendet")
            .zeit
    }

    fn auftrag(seit: Option<Zeit>, bis: Duration) -> Warteauftrag {
        Warteauftrag {
            seit,
            bis,
            wartender: agent(),
            filter: Filter::default(),
            programmende: Programmende::nie(),
            protokoll: Arc::new(Protokoll::ohne_ablage()),
            sitzung_besteht: Arc::new(|| true),
        }
    }

    fn still() -> Beobachter {
        Arc::new(|_| {})
    }

    /// Ein Beobachter, der am Schritt `bei` einmal `tun` ausführt.
    fn bei(schritt: Schritt, tun: impl Fn() + Send + Sync + 'static) -> Beobachter {
        let getan = std::sync::atomic::AtomicBool::new(false);
        Arc::new(move |jetzt| {
            if jetzt == schritt && !getan.swap(true, std::sync::atomic::Ordering::SeqCst) {
                tun();
            }
        })
    }

    fn zeiten(ergebnis: &Warteergebnis) -> Vec<Zeit> {
        ergebnis
            .treffer
            .iter()
            .map(|treffer| treffer.nachricht.zeit)
            .collect()
    }

    const LANG: Duration = Duration::from_secs(10);
    const KURZ: Duration = Duration::from_secs(2);

    /// W5 (a): zwischen Abonnieren und Marke sendet `anna` `t1` — sie steht im Index (≤ `H`) **und** im Abonnement
    /// und wird genau einmal geliefert (Dublettenregel „Bus nur über dem Stand“).
    #[tokio::test]
    async fn w5a_zwischen_abonnieren_und_marke_genau_einmal() {
        let bus = Arc::new(Bus::neu(crate::live::KAPAZITAET));
        let gespraeche = gespraeche_mit(speicher(), &bus);
        let t0 = senden(&gespraeche, "a", "vorher");
        let t1 = Arc::new(Mutex::new(None));
        let beobachter = {
            let gespraeche = Arc::clone(&gespraeche);
            let t1 = Arc::clone(&t1);
            bei(Schritt::Abonniert, move || {
                *t1.lock().expect("t1") = Some(senden(&gespraeche, "a", "@agent t1"));
            })
        };

        let ergebnis = warten(
            Arc::clone(&gespraeche),
            &bus,
            &Blockierend,
            auftrag(Some(t0), LANG),
            beobachter,
        )
        .await
        .expect("gewartet");

        let t1 = t1.lock().expect("t1").expect("gesendet");
        assert_eq!(zeiten(&ergebnis), vec![t1]);
        assert_eq!((ergebnis.seit, ergebnis.ende), (Some(t1), Ende::Treffer));
    }

    /// W5 M1: nach der Marke, vor dem Nachholen sendet `anna` `t1` — sie liegt nur im Abonnement; wer erst danach
    /// abonniert, verliert sie (Mutante „Abonnieren nach `H`“ oder „nach dem Nachholen“).
    #[tokio::test]
    async fn w5_m1_nach_der_marke_kommt_ueber_das_abonnement() {
        let bus = Arc::new(Bus::neu(crate::live::KAPAZITAET));
        let gespraeche = gespraeche_mit(speicher(), &bus);
        let t0 = senden(&gespraeche, "a", "vorher");
        let t1 = Arc::new(Mutex::new(None));
        let beobachter = {
            let gespraeche = Arc::clone(&gespraeche);
            let t1 = Arc::clone(&t1);
            bei(Schritt::MarkeGelesen, move || {
                *t1.lock().expect("t1") = Some(senden(&gespraeche, "b", "@agent t1"));
            })
        };

        let ergebnis = warten(
            Arc::clone(&gespraeche),
            &bus,
            &Blockierend,
            auftrag(Some(t0), KURZ),
            beobachter,
        )
        .await
        .expect("gewartet");

        let t1 = t1.lock().expect("t1").expect("gesendet");
        assert_eq!(zeiten(&ergebnis), vec![t1]);
        assert_eq!(ergebnis.ende, Ende::Treffer);
    }

    /// W5 (b): zwischen Nachholen und Bus-Warten sendet `anna` `t2` in einem anderen Channel → genau einmal.
    #[tokio::test]
    async fn w5b_zwischen_nachholen_und_warten_genau_einmal() {
        let bus = Arc::new(Bus::neu(crate::live::KAPAZITAET));
        let gespraeche = gespraeche_mit(speicher(), &bus);
        let t0 = senden(&gespraeche, "a", "vorher");
        let t2 = Arc::new(Mutex::new(None));
        let beobachter = {
            let gespraeche = Arc::clone(&gespraeche);
            let t2 = Arc::clone(&t2);
            bei(Schritt::Nachgeholt, move || {
                *t2.lock().expect("t2") = Some(senden(&gespraeche, "b", "@agent t2"));
            })
        };

        let ergebnis = warten(
            Arc::clone(&gespraeche),
            &bus,
            &Blockierend,
            auftrag(Some(t0), KURZ),
            beobachter,
        )
        .await
        .expect("gewartet");

        let t2 = t2.lock().expect("t2").expect("gesendet");
        assert_eq!(zeiten(&ergebnis), vec![t2]);
        assert_eq!((ergebnis.seit, ergebnis.ende), (Some(t2), Ende::Treffer));
    }

    fn weg_von(kennung: &str, name: &str) -> Ereignis {
        let jetzt = zeit("2026-10-30T12:00:05.000+01:00");
        Ereignis::Anwesenheit(crate::anwesenheit::Wechsel {
            stand: 7,
            jetzt,
            teilnehmer: crate::anwesenheit::Teilnehmerzeile {
                kennung: Kennung::bilden(kennung).expect("Kennung"),
                name: crate::name::Name::bilden(name).expect("Name"),
                herkunft: "127.0.0.1".parse().expect("IP"),
                art: crate::anwesenheit::Art::Agent,
                status: Status::Weg,
                seit: jetzt,
                weg_um: None,
            },
        })
    }

    /// W6b: das Ende des Programms beendet den wartenden Aufruf sofort — ohne Treffer, mit leerer Liste.
    #[tokio::test]
    async fn das_ende_des_programms_beendet_den_aufruf_sofort() {
        let bus = Arc::new(Bus::neu(crate::live::KAPAZITAET));
        let gespraeche = gespraeche_mit(speicher(), &bus);
        let (melden, programmende) = Programmende::neu();
        let mut warte = auftrag(None, LANG);
        warte.programmende = programmende;

        let aufruf = tokio::spawn({
            let gespraeche = Arc::clone(&gespraeche);
            let bus = Arc::clone(&bus);
            async move { warten(gespraeche, &bus, &Blockierend, warte, still()).await }
        });
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(!aufruf.is_finished(), "der Aufruf wartet");
        let beginn = tokio::time::Instant::now();
        melden.melden();
        let ergebnis = aufruf.await.expect("Aufruf").expect("gewartet");

        assert!(ergebnis.treffer.is_empty());
        assert_eq!(ergebnis.ende, Ende::Programmende);
        assert!(beginn.elapsed() < Duration::from_secs(1));
    }

    /// W6b: das `weg` der eigenen Kennung beendet den Aufruf mit `Verlassen`; das `weg` einer anderen Kennung nicht.
    #[tokio::test]
    async fn das_weg_der_eigenen_sitzung_beendet_den_aufruf_ein_fremdes_nicht() {
        let bus = Arc::new(Bus::neu(crate::live::KAPAZITAET));
        let gespraeche = gespraeche_mit(speicher(), &bus);
        let aufruf = tokio::spawn({
            let gespraeche = Arc::clone(&gespraeche);
            let bus = Arc::clone(&bus);
            async move { warten(gespraeche, &bus, &Blockierend, auftrag(None, KURZ), still()).await }
        });
        tokio::time::sleep(Duration::from_millis(200)).await;
        bus.veroeffentlichen(weg_von("anna@127.0.0.1", "anna"));
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(!aufruf.is_finished(), "ein fremdes weg beendet nichts");
        bus.veroeffentlichen(weg_von("agent@127.0.0.1", "agent"));
        let ergebnis = aufruf.await.expect("Aufruf").expect("gewartet");

        assert!(ergebnis.treffer.is_empty());
        assert_eq!(ergebnis.ende, Ende::Verlassen);
    }

    /// Code-Review #17 F0169 A1 (Probe wörtlich): Bus Kapazität 2, der Aufruf wartet 3 s, `weg_von(agent)` und
    /// drei fremde Ereignisse synchron → das Abonnement ist überholt und hat das `weg` verloren; nach `NeuerStand`
    /// prüft der Aufruf die Sitzung gegen den Bestand und endet mit `Verlassen` (nicht mit `Frist`).
    #[tokio::test]
    async fn ein_verlorenes_weg_der_eigenen_sitzung_endet_mit_verlassen() {
        let bus = Arc::new(Bus::neu(2));
        let gespraeche = gespraeche_mit(speicher(), &bus);
        let besteht = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let mut warte = auftrag(None, Duration::from_secs(3));
        warte.sitzung_besteht = {
            let besteht = Arc::clone(&besteht);
            Arc::new(move || besteht.load(std::sync::atomic::Ordering::SeqCst))
        };
        let aufruf = tokio::spawn({
            let gespraeche = Arc::clone(&gespraeche);
            let bus = Arc::clone(&bus);
            async move { warten(gespraeche, &bus, &Blockierend, warte, still()).await }
        });
        tokio::time::sleep(Duration::from_millis(200)).await;

        besteht.store(false, std::sync::atomic::Ordering::SeqCst);
        bus.veroeffentlichen(weg_von("agent@127.0.0.1", "agent"));
        for _ in 0..3 {
            bus.veroeffentlichen(weg_von("anna@127.0.0.1", "anna"));
        }
        let ergebnis = tokio::time::timeout(Duration::from_secs(5), aufruf)
            .await
            .expect("Ende")
            .expect("Aufruf")
            .expect("gewartet");

        assert_eq!(ergebnis.ende, Ende::Verlassen);
    }

    /// A1 Gegenprobe: ein überholtes Abonnement bei bestehender Sitzung endet nicht mit `Verlassen`.
    #[tokio::test]
    async fn ein_ueberholtes_abonnement_bei_bestehender_sitzung_wartet_weiter() {
        let bus = Arc::new(Bus::neu(2));
        let gespraeche = gespraeche_mit(speicher(), &bus);
        let aufruf = tokio::spawn({
            let gespraeche = Arc::clone(&gespraeche);
            let bus = Arc::clone(&bus);
            async move { warten(gespraeche, &bus, &Blockierend, auftrag(None, KURZ), still()).await }
        });
        tokio::time::sleep(Duration::from_millis(200)).await;

        for _ in 0..4 {
            bus.veroeffentlichen(weg_von("anna@127.0.0.1", "anna"));
        }
        let ergebnis = aufruf.await.expect("Aufruf").expect("gewartet");

        assert_eq!(ergebnis.ende, Ende::Frist);
    }

    /// B0772 (ADR-0055 Regel 8, Mutante „unsichtbares Bus-Ereignis rückt den Stand vor“): ein Direktgespräch zweier
    /// anderer, das nach dem Nachholen nur über den Bus kommt, ist für den Wartenden nicht geschehen — kein Treffer,
    /// `seit` bleibt `t0`. Ein Channel danach rückt ihn vor.
    #[tokio::test]
    async fn ein_fremdes_direktgespraech_ueber_den_bus_rueckt_den_stand_nicht_vor() {
        let bus = Arc::new(Bus::neu(crate::live::KAPAZITAET));
        let ablage = speicher();
        let fremd = Gespraech::Direkt(crate::gespraeche::Direktpaar::fuer_test(
            "ben@127.0.0.1",
            "dora@127.0.0.1",
        ));
        ablage.gespraech_anlegen(&fremd);
        let gespraeche = gespraeche_mit(ablage, &bus);
        let t0 = senden(&gespraeche, "a", "vorher");
        let beobachter = {
            let gespraeche = Arc::clone(&gespraeche);
            bei(Schritt::Nachgeholt, move || {
                gespraeche
                    .senden(&fremd, &kennung("ben@127.0.0.1"), Art::Mensch, "geheim")
                    .expect("gesendet");
            })
        };

        let ergebnis = warten(
            Arc::clone(&gespraeche),
            &bus,
            &Blockierend,
            auftrag(Some(t0), Duration::from_millis(400)),
            beobachter,
        )
        .await
        .expect("gewartet");

        assert!(ergebnis.treffer.is_empty(), "{:?}", ergebnis.treffer);
        assert_eq!((ergebnis.seit, ergebnis.ende), (Some(t0), Ende::Frist));
    }

    /// Wartet in Schleife mit dem jeweils gelieferten `seit`, bis `anzahl` Treffer da sind oder eine Frist ohne
    /// Treffer endet; liefert alle Zeiten in Empfangsfolge.
    async fn in_schleife(
        gespraeche: &Arc<Gespraeche>,
        bus: &Bus,
        mut seit: Option<Zeit>,
        anzahl: usize,
    ) -> Vec<Zeit> {
        let mut empfangen = Vec::new();
        while empfangen.len() < anzahl {
            let ergebnis = warten(
                Arc::clone(gespraeche),
                bus,
                &Blockierend,
                auftrag(seit, KURZ),
                still(),
            )
            .await
            .expect("gewartet");
            empfangen.extend(zeiten(&ergebnis));
            seit = ergebnis.seit;
            if ergebnis.ende == Ende::Frist {
                break;
            }
        }
        empfangen
    }

    /// W5 (c), ADR-0035 Pre-Mortem 1: die Ablage hält das Anhängen in `a` fest; in dieser Zeit beginnt `warten`, und
    /// `b` sendet danach. Nach der Freigabe kommen beide in Zeitfolge, je einmal.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn w5c_eine_haengende_ablage_in_a_verliert_b_nicht() {
        let (angekommen_senden, angekommen) = std::sync::mpsc::channel();
        let (frei, frei_empfangen) = std::sync::mpsc::channel();
        let ablage = Arc::new(Haltende {
            innen: speicher(),
            angekommen: Mutex::new(angekommen_senden),
            frei: Mutex::new(frei_empfangen),
        });
        let bus = Arc::new(Bus::neu(crate::live::KAPAZITAET));
        let gespraeche = gespraeche_mit(ablage, &bus);
        let t0 = senden(&gespraeche, "b", "vorher");
        let in_a = {
            let gespraeche = Arc::clone(&gespraeche);
            std::thread::spawn(move || senden(&gespraeche, "a", "@agent in a"))
        };
        angekommen
            .recv_timeout(Duration::from_secs(10))
            .expect("a hängt beim Anhängen");
        let wartend = {
            let gespraeche = Arc::clone(&gespraeche);
            let bus = Arc::clone(&bus);
            tokio::spawn(async move { in_schleife(&gespraeche, &bus, Some(t0), 2).await })
        };
        tokio::time::sleep(Duration::from_millis(200)).await;
        let in_b = {
            let gespraeche = Arc::clone(&gespraeche);
            std::thread::spawn(move || senden(&gespraeche, "b", "@agent in b"))
        };
        tokio::time::sleep(Duration::from_millis(200)).await;
        frei.send(()).expect("Freigabe");
        let ta = in_a.join().expect("a");
        let tb = in_b.join().expect("b");

        let empfangen = wartend.await.expect("Schleife");

        assert!(ta < tb, "{ta} < {tb}");
        assert_eq!(empfangen, vec![ta, tb]);
    }

    /// W5 (d): Bus mit Kapazität 2, fünf Nachrichten ohne Verarbeitung → `Lagged` → aus dem Index nachgeholt, alle
    /// fünf genau einmal in Zeitfolge (Mutante „`Lagged` als Neuer Stand oder still übergangen“ verliert drei).
    #[tokio::test]
    async fn w5d_lagged_holt_aus_dem_index_nach() {
        let bus = Arc::new(Bus::neu(2));
        let gespraeche = gespraeche_mit(speicher(), &bus);
        let t0 = senden(&gespraeche, "a", "vorher");
        let (nachgeholt_senden, nachgeholt) = tokio::sync::oneshot::channel::<()>();
        let nachgeholt_senden = Mutex::new(Some(nachgeholt_senden));
        let beobachter = bei(Schritt::Nachgeholt, move || {
            if let Some(melden) = nachgeholt_senden.lock().expect("melden").take() {
                let _ = melden.send(());
            }
        });
        let wartend = {
            let gespraeche = Arc::clone(&gespraeche);
            let bus = Arc::clone(&bus);
            tokio::spawn(async move {
                warten(
                    gespraeche,
                    &bus,
                    &Blockierend,
                    auftrag(Some(t0), LANG),
                    beobachter,
                )
                .await
            })
        };
        nachgeholt.await.expect("nachgeholt");
        let gesendet: Vec<Zeit> = (1..=5)
            .map(|n| senden(&gespraeche, ["a", "b"][n % 2], &format!("t{n}")))
            .collect();

        let ergebnis = wartend.await.expect("Aufgabe").expect("gewartet");

        assert_eq!(zeiten(&ergebnis), gesendet);
        assert_eq!(
            (ergebnis.seit, ergebnis.ende),
            (gesendet.last().copied(), Ende::Treffer)
        );
    }

    /// B3 (code-reviewer #17), Probe 1: leere Ablage, Bus mit Kapazität 2, fünf Nachrichten nach dem Nachholen →
    /// `Lagged` ohne Stand holt ab dem Anfang nach: genau `t1` bis `t5`, `seit` ist `t5`.
    #[tokio::test]
    async fn lagged_ohne_stand_auf_leerer_ablage_verliert_nichts() {
        let bus = Arc::new(Bus::neu(2));
        let gespraeche = gespraeche_mit(speicher(), &bus);

        let (ergebnis, gesendet) = lagged_ohne_stand(&gespraeche, &bus, false).await;

        assert_eq!(zeiten(&ergebnis), gesendet);
        assert_eq!(
            (ergebnis.seit, ergebnis.ende),
            (gesendet.last().copied(), Ende::Treffer)
        );
    }

    /// B3 Probe 2: das erste Ereignis ist keine Nachricht (ein Beitritt), danach fünf Nachrichten bei Kapazität 2 → alle fünf.
    #[tokio::test]
    async fn lagged_nach_einem_fremden_ereignis_ohne_stand_verliert_nichts() {
        let bus = Arc::new(Bus::neu(2));
        let gespraeche = gespraeche_mit(speicher(), &bus);

        let (ergebnis, gesendet) = lagged_ohne_stand(&gespraeche, &bus, true).await;

        assert_eq!(zeiten(&ergebnis), gesendet);
    }

    /// Wartet ab jetzt auf leerer Ablage; nach dem Nachholen kommt (optional ein Beitritt und) fünf Nachrichten.
    async fn lagged_ohne_stand(
        gespraeche: &Arc<Gespraeche>,
        bus: &Arc<Bus>,
        mit_beitritt: bool,
    ) -> (Warteergebnis, Vec<Zeit>) {
        let (nachgeholt_senden, nachgeholt) = tokio::sync::oneshot::channel::<()>();
        let nachgeholt_senden = Mutex::new(Some(nachgeholt_senden));
        let beobachter = bei(Schritt::Nachgeholt, move || {
            if let Some(melden) = nachgeholt_senden.lock().expect("melden").take() {
                let _ = melden.send(());
            }
        });
        let wartend = {
            let gespraeche = Arc::clone(gespraeche);
            let bus = Arc::clone(bus);
            tokio::spawn(async move {
                warten(
                    gespraeche,
                    &bus,
                    &Blockierend,
                    auftrag(None, LANG),
                    beobachter,
                )
                .await
            })
        };
        nachgeholt.await.expect("nachgeholt");
        if mit_beitritt {
            bus.veroeffentlichen(weg_von("anna@127.0.0.1", "anna"));
        }
        let gesendet: Vec<Zeit> = (1..=5)
            .map(|n| senden(gespraeche, ["a", "b"][n % 2], &format!("t{n}")))
            .collect();
        let ergebnis = wartend.await.expect("Aufgabe").expect("gewartet");
        (ergebnis, gesendet)
    }

    /// S1 (observability-reviewer #17): ein überholtes Abonnement schreibt genau eine DEBUG-Zeile `live` mit Kennung
    /// und Zahl der verpassten Ereignisse; eine Wiederholung eine weitere; kein Nachrichtentext in der Zeile. Alle fünf
    /// Nachrichten kommen genau einmal.
    #[tokio::test]
    async fn ein_ueberholtes_abonnement_ist_genau_eine_debug_zeile() {
        let (protokoll, speicher, _uhr, _) =
            crate::protokoll::testhilfe::im_speicher("2026-10-30T12:00:00+01:00", Stufe::Debug);
        let bus = Arc::new(Bus::neu(2));
        let gespraeche = gespraeche_mit(speicher_ablage(), &bus);
        let mut seit = senden(&gespraeche, "a", "vorher");

        for lauf in 1..=2 {
            let (nachgeholt_senden, nachgeholt) = tokio::sync::oneshot::channel::<()>();
            let nachgeholt_senden = Mutex::new(Some(nachgeholt_senden));
            let beobachter = bei(Schritt::Nachgeholt, move || {
                if let Some(melden) = nachgeholt_senden.lock().expect("melden").take() {
                    let _ = melden.send(());
                }
            });
            let mut warte = auftrag(Some(seit), LANG);
            warte.protokoll = Arc::clone(&protokoll);
            let wartend = {
                let gespraeche = Arc::clone(&gespraeche);
                let bus = Arc::clone(&bus);
                tokio::spawn(async move {
                    warten(gespraeche, &bus, &Blockierend, warte, beobachter).await
                })
            };
            nachgeholt.await.expect("nachgeholt");
            let gesendet: Vec<Zeit> = (1..=5)
                .map(|n| {
                    senden(
                        &gespraeche,
                        ["a", "b"][n % 2],
                        &format!("GEHEIM-{lauf}-{n}"),
                    )
                })
                .collect();

            let ergebnis = wartend.await.expect("Aufgabe").expect("gewartet");

            assert_eq!(zeiten(&ergebnis), gesendet, "Lauf {lauf}");
            seit = ergebnis.seit.expect("seit");
            let zeilen: Vec<String> = speicher
                .zeilen()
                .into_iter()
                .filter(|zeile| zeile.contains(" DEBUG live "))
                .collect();
            assert_eq!(zeilen.len(), lauf, "{zeilen:#?}");
            let neue = zeilen.last().expect("Zeile");
            assert!(
                neue.contains(
                    " DEBUG live Abonnement überholt — nachgeholt | kennung=agent@127.0.0.1 anzahl="
                ),
                "{neue}"
            );
            assert!(
                zeilen.iter().all(|zeile| !zeile.contains("GEHEIM")),
                "{zeilen:#?}"
            );
        }
    }

    /// Eigene Nachrichten und andere Themen wecken nicht, rücken aber den Stand vor; nach `bis` endet es mit `Frist`.
    #[tokio::test]
    async fn eigene_nachrichten_wecken_nicht_und_der_stand_rueckt_vor() {
        let bus = Arc::new(Bus::neu(crate::live::KAPAZITAET));
        let gespraeche = gespraeche_mit(speicher(), &bus);
        let t0 = senden(&gespraeche, "a", "vorher");
        let eigene = Arc::new(Mutex::new(None));
        let beobachter = {
            let gespraeche = Arc::clone(&gespraeche);
            let eigene = Arc::clone(&eigene);
            bei(Schritt::Nachgeholt, move || {
                let zeit = gespraeche
                    .senden(&Gespraech::channel("a"), &agent(), Art::Agent, "eigene")
                    .expect("gesendet")
                    .zeit;
                *eigene.lock().expect("eigene") = Some(zeit);
            })
        };
        let beginn = std::time::Instant::now();

        let ergebnis = warten(
            Arc::clone(&gespraeche),
            &bus,
            &Blockierend,
            auftrag(Some(t0), Duration::from_millis(300)),
            beobachter,
        )
        .await
        .expect("gewartet");

        assert!(beginn.elapsed() >= Duration::from_millis(300));
        assert!(ergebnis.treffer.is_empty());
        assert_eq!(
            (ergebnis.seit, ergebnis.ende),
            (*eigene.lock().expect("eigene"), Ende::Frist)
        );
    }

    /// Ohne `seit` gilt „ab jetzt“: was vorher gesendet war, kommt nicht; der Stand ist die Marke.
    #[tokio::test]
    async fn ohne_seit_gilt_ab_jetzt() {
        let bus = Arc::new(Bus::neu(crate::live::KAPAZITAET));
        let gespraeche = gespraeche_mit(speicher(), &bus);
        let vorher = senden(&gespraeche, "a", "vorher");

        let ergebnis = warten(
            Arc::clone(&gespraeche),
            &bus,
            &Blockierend,
            auftrag(None, Duration::ZERO),
            still(),
        )
        .await
        .expect("gewartet");

        assert!(ergebnis.treffer.is_empty());
        assert_eq!((ergebnis.seit, ergebnis.ende), (Some(vorher), Ende::Frist));
    }

    /// Ist beim Nachholen ein Budget erschöpft, endet der Aufruf sofort mit `Weitere` am Stand des Nachholens.
    #[tokio::test]
    async fn ein_erschoepftes_budget_endet_sofort_mit_weitere() {
        let bus = Arc::new(Bus::neu(crate::live::KAPAZITAET));
        let gespraeche = gespraeche_mit(speicher(), &bus);
        let t0 = senden(&gespraeche, "a", "vorher");
        let gesendet: Vec<Zeit> = (0..=NACHHOLBUDGET.nachrichten)
            .map(|n| senden(&gespraeche, "b", &format!("n{n}")))
            .collect();

        let erster = warten(
            Arc::clone(&gespraeche),
            &bus,
            &Blockierend,
            auftrag(Some(t0), LANG),
            still(),
        )
        .await
        .expect("gewartet");

        assert_eq!(zeiten(&erster), gesendet[..NACHHOLBUDGET.nachrichten]);
        assert_eq!(
            (erster.seit, erster.ende),
            (Some(gesendet[NACHHOLBUDGET.nachrichten - 1]), Ende::Weitere)
        );
    }

    /// Logablage, die das Anhängen an `a` festhält, bis der Test sie freigibt (ADR-0035 Pre-Mortem 1).
    struct Haltende {
        innen: Arc<Speicherablage>,
        angekommen: Mutex<std::sync::mpsc::Sender<()>>,
        frei: Mutex<std::sync::mpsc::Receiver<()>>,
    }

    impl Logablage for Haltende {
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
            if *gespraech == Gespraech::channel("a") {
                self.angekommen
                    .lock()
                    .expect("angekommen")
                    .send(())
                    .expect("Test wartet");
                self.frei.lock().expect("frei").recv().expect("Freigabe");
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
}
