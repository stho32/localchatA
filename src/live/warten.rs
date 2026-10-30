//! Ablauf eines Warte-Aufrufs (ADR-0054 Regel 3 und 5, B0738): abonnieren → Marke `H` unter der Schreibsperre →
//! nachholen bis `H` → den Bus filtern. Die Reihenfolge **ist** die Verlustfreiheit: was bis `H` vergeben ist, steht
//! im Index; was danach kommt, hat eine Zeit über `H` und liegt im Abonnement. Vom Bus zählt nur, was über dem
//! verarbeiteten Stand liegt (Dublettenregel); verliert das Abonnement Ereignisse, wird ab dem Stand nachgeholt.

use crate::gespraeche::nachholen::{NACHHOLBUDGET, Nachgeholt, Nachholbudget, lieferbytes};
use crate::gespraeche::treffer::{Filter, Treffer, treffer};
use crate::gespraeche::{Gespraech, Gespraeche, Kennung, Nachricht};
use crate::http::Werkbank;
use crate::http::abbruch::Abgebrochen;
use crate::live::bus::{Abonnement, Bus, Empfang, Ereignis};
use crate::uhr::Zeit;
use std::sync::Arc;
use std::time::Duration;

/// Ein Warte-Aufruf: ab `seit` (ohne: ab der Marke), höchstens `bis`, für `wartender` unter `filter`.
#[derive(Debug, Clone)]
pub struct Warteauftrag {
    pub seit: Option<Zeit>,
    pub bis: Duration,
    pub wartender: Kennung,
    pub filter: Filter,
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
    /// Die Marke ist gelesen, nachgeholt ist noch nicht (auf der Werkbank gerufen).
    MarkeGelesen,
    /// Nachgeholt ist, auf den Bus wird noch nicht gewartet.
    Nachgeholt,
}

/// Beobachter der Schritte; im Programm tut er nichts.
pub type Beobachter = Arc<dyn Fn(Schritt) + Send + Sync>;

/// Wartet nach Regel 3; `Err` nur, wenn die Arbeit auf der Werkbank abbrach.
pub async fn warten(
    gespraeche: Arc<Gespraeche>,
    bus: &Bus,
    werkbank: &Werkbank,
    auftrag: Warteauftrag,
    beobachter: Beobachter,
) -> Result<Warteergebnis, Abgebrochen> {
    let frist = tokio::time::Instant::now() + auftrag.bis;
    let mut abonnement = bus.abonnieren();
    beobachter(Schritt::Abonniert);
    let mut sammlung = Sammlung {
        treffer: Vec::new(),
        bytes: 0,
        stand: auftrag.seit,
    };
    let nachgeholt = nachholen(&gespraeche, werkbank, &auftrag, &sammlung, &beobachter).await?;
    beobachter(Schritt::Nachgeholt);
    let mut ende = sammlung.aufnehmen(nachgeholt);
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
            () = tokio::time::sleep_until(frist) => return Ok(sammlung.ergebnis(Ende::Frist)),
            empfang = abonnement.naechstes() => empfang,
        };
        ende = match empfang {
            None => Some(Ende::Frist),
            Some(Empfang::Ereignis(Ereignis::Nachricht { channel, nachricht })) => sammlung
                .pruefen(&Gespraech::Channel(channel), nachricht, &auftrag)
                .or_else(|| (!sammlung.treffer.is_empty()).then_some(Ende::Treffer)),
            Some(Empfang::Ereignis(_)) => None,
            Some(Empfang::NeuerStand { .. }) => {
                let nachgeholt =
                    nachholen(&gespraeche, werkbank, &auftrag, &sammlung, &beobachter).await?;
                sammlung.aufnehmen(nachgeholt)
            }
        };
    }
}

/// Liest die Marke `H` unter der Schreibsperre und holt `(Stand, H]` nach — auf der Werkbank, mit dem Budget, das
/// der Sammlung noch bleibt. Ohne Stand gilt „ab jetzt“: der Stand wird die Marke.
async fn nachholen(
    gespraeche: &Arc<Gespraeche>,
    werkbank: &Werkbank,
    auftrag: &Warteauftrag,
    sammlung: &Sammlung,
    beobachter: &Beobachter,
) -> Result<Option<Nachgeholt>, Abgebrochen> {
    let gespraeche = Arc::clone(gespraeche);
    let beobachter = Arc::clone(beobachter);
    let stand = sammlung.stand;
    let budget = sammlung.restbudget();
    let wartender = auftrag.wartender.clone();
    let filter = auftrag.filter.clone();
    werkbank
        .ausfuehren(move || {
            let marke = gespraeche.letzte_zeit();
            beobachter(Schritt::MarkeGelesen);
            let ab = stand.or(marke)?;
            let marke = marke?;
            Some(gespraeche.nachholen(ab, marke, &wartender, &filter, budget))
        })
        .await
}

/// Was ein Aufruf bisher gesammelt hat: Treffer, ihre Lieferbytes und den verarbeiteten Stand.
struct Sammlung {
    treffer: Vec<Treffer>,
    bytes: usize,
    stand: Option<Zeit>,
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
        self.stand = Some(nachgeholt.stand);
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
                Some(Empfang::NeuerStand { .. }) => return Ende::Weitere,
                Some(Empfang::Ereignis(Ereignis::Nachricht { channel, nachricht })) => {
                    if let Some(ende) =
                        self.pruefen(&Gespraech::Channel(channel), nachricht, auftrag)
                    {
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
            &Werkbank::aus_kernen(),
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
            &Werkbank::aus_kernen(),
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
            &Werkbank::aus_kernen(),
            auftrag(Some(t0), KURZ),
            beobachter,
        )
        .await
        .expect("gewartet");

        let t2 = t2.lock().expect("t2").expect("gesendet");
        assert_eq!(zeiten(&ergebnis), vec![t2]);
        assert_eq!((ergebnis.seit, ergebnis.ende), (Some(t2), Ende::Treffer));
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
                &Werkbank::aus_kernen(),
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
                    &Werkbank::aus_kernen(),
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
            &Werkbank::aus_kernen(),
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
            &Werkbank::aus_kernen(),
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
            &Werkbank::aus_kernen(),
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
