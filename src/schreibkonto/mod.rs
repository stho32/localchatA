//! Schreibkonto (A00014, ADR-0036): zwei Token-Eimer je Kennung und je Herkunft, nur im Speicher. Gebucht wird
//! in zwei Schritten (Regel 3): `vorab` 1 Einheit vor dem Lesen des Körpers, `rest` die übrigen Kosten nach allen
//! 4xx-Prüfungen — jeweils an beiden Eimern, alles oder nichts, unter einer Sperre.

pub mod eimer;

use crate::gespraeche::Kennung;
use crate::uhr::Uhr;
use eimer::{Eimer, Eimerart, JE_HERKUNFT, JE_KENNUNG};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

/// Welcher Eimer bremst.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grenze {
    Kennung,
    Herkunft,
}

/// Abgewiesen: der Eimer, der länger braucht, und die Zeit, bis beide die vollen Kosten tragen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gebremst {
    pub grenze: Grenze,
    pub wieder_in: Duration,
}

#[derive(Default)]
struct Tabellen {
    kennungen: HashMap<Kennung, Eimer>,
    herkuenfte: HashMap<IpAddr, Eimer>,
}

pub struct Schreibkonto {
    uhr: Arc<dyn Uhr>,
    tabellen: Mutex<Tabellen>,
    gebremst: AtomicU64,
}

impl Schreibkonto {
    pub fn neu(uhr: Arc<dyn Uhr>) -> Schreibkonto {
        Schreibkonto {
            uhr,
            tabellen: Mutex::default(),
            gebremst: AtomicU64::new(0),
        }
    }

    /// 1 Einheit nach der Sitzung, vor dem Lesen des Körpers; wird nie erstattet.
    pub fn vorab(&self, kennung: &Kennung, herkunft: IpAddr) -> Result<(), Gebremst> {
        self.abbuchen(kennung, herkunft, 1, 1)
    }

    /// Der Rest der Kosten (`kosten` − 1) nach allen 4xx-Prüfungen; `wieder_in` bis die vollen Kosten tragen.
    pub fn rest(&self, kennung: &Kennung, herkunft: IpAddr, kosten: u32) -> Result<(), Gebremst> {
        let kosten = u64::from(kosten.max(1));
        self.abbuchen(kennung, herkunft, kosten - 1, kosten)
    }

    /// Zahl der Bremsungen seit dem Start (`/api/gesundheit`).
    pub fn gebremst(&self) -> u64 {
        self.gebremst.load(Ordering::Relaxed)
    }

    fn abbuchen(
        &self,
        kennung: &Kennung,
        herkunft: IpAddr,
        einheiten: u64,
        volle_kosten: u64,
    ) -> Result<(), Gebremst> {
        let jetzt = self.uhr.jetzt();
        let mut tabellen = self.tabellen.lock().unwrap_or_else(PoisonError::into_inner);
        let je_kennung = nachgefuellt(&tabellen.kennungen, kennung, JE_KENNUNG, jetzt);
        let je_herkunft = nachgefuellt(&tabellen.herkuenfte, &herkunft, JE_HERKUNFT, jetzt);
        let reichen = je_kennung.reicht(einheiten) && je_herkunft.reicht(einheiten);
        let (nach_kennung, nach_herkunft) = if reichen {
            (
                je_kennung.abgebucht(einheiten),
                je_herkunft.abgebucht(einheiten),
            )
        } else {
            (je_kennung, je_herkunft)
        };
        ablegen(
            &mut tabellen.kennungen,
            kennung.clone(),
            nach_kennung,
            JE_KENNUNG,
        );
        ablegen(
            &mut tabellen.herkuenfte,
            herkunft,
            nach_herkunft,
            JE_HERKUNFT,
        );
        aufraeumen(&mut tabellen, jetzt);
        if !reichen {
            self.gebremst.fetch_add(1, Ordering::Relaxed);
            return Err(bremsen(&je_kennung, &je_herkunft, volle_kosten));
        }
        Ok(())
    }
}

/// Mehr Einträge lösen einen Durchgang aus (ADR-0036 Regel 6).
const DURCHGANG_AB: usize = 1_024;

/// Ein voller Eimer wird nicht gehalten — er gleicht dem fehlenden (ADR-0036 Regel 6).
fn ablegen<S: std::hash::Hash + Eq>(
    tabelle: &mut HashMap<S, Eimer>,
    schluessel: S,
    eimer: Eimer,
    art: Eimerart,
) {
    if eimer.ist_voll(art) {
        tabelle.remove(&schluessel);
    } else {
        tabelle.insert(schluessel, eimer);
    }
}

/// Bei mehr als 1.024 Einträgen fallen alle Eimer weg, die bis `jetzt` wieder voll sind.
fn aufraeumen(tabellen: &mut Tabellen, jetzt: crate::uhr::Zeit) {
    if tabellen.kennungen.len() > DURCHGANG_AB {
        tabellen
            .kennungen
            .retain(|_, eimer| !eimer.nachgefuellt(JE_KENNUNG, jetzt).ist_voll(JE_KENNUNG));
    }
    if tabellen.herkuenfte.len() > DURCHGANG_AB {
        tabellen
            .herkuenfte
            .retain(|_, eimer| !eimer.nachgefuellt(JE_HERKUNFT, jetzt).ist_voll(JE_HERKUNFT));
    }
}

/// Der Eimer zum Schlüssel, nachgefüllt bis `jetzt`; ohne Eintrag ein voller.
fn nachgefuellt<S: std::hash::Hash + Eq>(
    tabelle: &HashMap<S, Eimer>,
    schluessel: &S,
    art: Eimerart,
    jetzt: crate::uhr::Zeit,
) -> Eimer {
    tabelle
        .get(schluessel)
        .map_or(Eimer::voll(art, jetzt), |eimer| {
            eimer.nachgefuellt(art, jetzt)
        })
}

/// Die Zeit, bis beide Eimer die vollen Kosten tragen; die Grenze ist der Eimer, der länger braucht.
fn bremsen(je_kennung: &Eimer, je_herkunft: &Eimer, volle_kosten: u64) -> Gebremst {
    let kennung_ms = je_kennung.wartezeit_ms(JE_KENNUNG, volle_kosten);
    let herkunft_ms = je_herkunft.wartezeit_ms(JE_HERKUNFT, volle_kosten);
    let (grenze, ms) = if kennung_ms >= herkunft_ms {
        (Grenze::Kennung, kennung_ms)
    } else {
        (Grenze::Herkunft, herkunft_ms)
    };
    Gebremst {
        grenze,
        wieder_in: Duration::from_millis(ms),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uhr::{GestellteUhr, Zeit};

    fn t(ms: i64) -> Zeit {
        Zeit::parse_from_rfc3339("2026-10-28T12:00:00+01:00").expect("Zeit")
            + chrono::Duration::milliseconds(ms)
    }

    fn konto() -> (Schreibkonto, Arc<GestellteUhr>) {
        let uhr = Arc::new(GestellteUhr::neu(t(0)));
        (Schreibkonto::neu(uhr.clone()), uhr)
    }

    fn kennung(text: &str) -> Kennung {
        Kennung::bilden(text).expect("Kennung")
    }

    fn ip(text: &str) -> IpAddr {
        text.parse().expect("IP")
    }

    fn gebremst(grenze: Grenze, ms: u64) -> Result<(), Gebremst> {
        Err(Gebremst {
            grenze,
            wieder_in: Duration::from_millis(ms),
        })
    }

    /// R00011 F0045 „Eimer je Kennung“: 30 × 1 ok, die 31. gebremst mit 1.000 ms; nach 999 ms gebremst, nach
    /// 1.000 ms frei.
    #[test]
    fn eimer_je_kennung_30_31_und_nachschub_als_spanne() {
        let (konto, uhr) = konto();
        let a = kennung("a@10.0.0.1");
        for nummer in 0..30 {
            assert_eq!(konto.vorab(&a, ip("10.0.0.1")), Ok(()), "{nummer}");
        }

        let einunddreissigste = konto.vorab(&a, ip("10.0.0.1"));
        uhr.stellen(t(999));
        let nach_999 = konto.vorab(&a, ip("10.0.0.1"));
        uhr.stellen(t(1_000));
        let nach_1000 = konto.vorab(&a, ip("10.0.0.1"));

        assert_eq!(einunddreissigste, gebremst(Grenze::Kennung, 1_000));
        assert_eq!(nach_999, gebremst(Grenze::Kennung, 1));
        assert_eq!(nach_1000, Ok(()));
        assert_eq!(konto.gebremst(), 2);
    }

    /// „Eimer je Herkunft“: vier Kennungen von 10.0.0.1 je 30 → ok; die 121. Einheit (fünfte Kennung) gebremst mit
    /// 125 ms; eine Kennung von 10.0.0.2 → ok.
    #[test]
    fn eimer_je_herkunft_120_121() {
        let (konto, _) = konto();
        for nummer in 0..4 {
            let k = kennung(&format!("k{nummer}@10.0.0.1"));
            for _ in 0..30 {
                assert_eq!(konto.vorab(&k, ip("10.0.0.1")), Ok(()));
            }
        }

        let fuenfte = konto.vorab(&kennung("k4@10.0.0.1"), ip("10.0.0.1"));
        let andere = konto.vorab(&kennung("x@10.0.0.2"), ip("10.0.0.2"));

        assert_eq!(fuenfte, gebremst(Grenze::Herkunft, 125));
        assert_eq!(andere, Ok(()));
    }

    /// „Alles oder nichts“: Kennung voll, Herkunft leer → gebremst, und der Eimer der Kennung bleibt voll — nach
    /// 15 s (Herkunft wieder voll) gehen genau 30.
    #[test]
    fn alles_oder_nichts_der_volle_eimer_bleibt_voll() {
        let (konto, uhr) = konto();
        for nummer in 0..4 {
            let k = kennung(&format!("k{nummer}@10.0.0.1"));
            for _ in 0..30 {
                konto.vorab(&k, ip("10.0.0.1")).expect("frei");
            }
        }
        let e = kennung("e@10.0.0.1");

        assert_eq!(
            konto.vorab(&e, ip("10.0.0.1")),
            gebremst(Grenze::Herkunft, 125)
        );
        uhr.stellen(t(15_000));
        let durch = (0..31)
            .take_while(|_| konto.vorab(&e, ip("10.0.0.1")).is_ok())
            .count();
        assert_eq!(durch, 30);
    }

    #[test]
    fn eine_zurueckgestellte_uhr_ist_kein_fehler_und_fuellt_nichts() {
        let (konto, uhr) = konto();
        let a = kennung("a@10.0.0.1");
        for _ in 0..30 {
            konto.vorab(&a, ip("10.0.0.1")).expect("frei");
        }

        uhr.stellen(t(-3_600_000));

        assert_eq!(
            konto.vorab(&a, ip("10.0.0.1")),
            gebremst(Grenze::Kennung, 1_000)
        );
    }

    /// „Zwei Schritte und volle Kosten“: Kennung mit 2 Einheiten; vorab 1 → ok; Rest 3 → gebremst mit 3.000 ms, die
    /// Vorab-Einheit bleibt verbraucht. Danach reicht der Eimer nach 2.999 ms nicht für 1 + 3, nach 3.000 ms schon.
    #[test]
    fn zwei_schritte_und_volle_kosten() {
        let mit_zwei = || {
            let (konto, uhr) = konto();
            let a = kennung("a@10.0.0.1");
            for _ in 0..28 {
                konto.vorab(&a, ip("10.0.0.1")).expect("frei");
            }
            let vorab = konto.vorab(&a, ip("10.0.0.1"));
            let rest = konto.rest(&a, ip("10.0.0.1"), 4);
            (konto, uhr, a, vorab, rest)
        };
        let (konto, uhr, a, vorab, rest) = mit_zwei();
        let (spaeter, spaeter_uhr, b, _, _) = mit_zwei();

        uhr.stellen(t(2_999));
        let nach_2999 = konto
            .vorab(&a, ip("10.0.0.1"))
            .and_then(|()| konto.rest(&a, ip("10.0.0.1"), 4));
        spaeter_uhr.stellen(t(3_000));
        let nach_3000 = spaeter
            .vorab(&b, ip("10.0.0.1"))
            .and_then(|()| spaeter.rest(&b, ip("10.0.0.1"), 4));

        assert_eq!(vorab, Ok(()));
        assert_eq!(rest, gebremst(Grenze::Kennung, 3_000));
        assert!(nach_2999.is_err(), "{nach_2999:?}");
        assert_eq!(nach_3000, Ok(()));
    }

    /// Reichen beide nicht, nennt `grenze` den Eimer, der länger braucht.
    #[test]
    fn die_grenze_ist_der_eimer_der_laenger_braucht() {
        let (konto, _) = konto();
        for nummer in 0..4 {
            let k = kennung(&format!("k{nummer}@10.0.0.1"));
            for _ in 0..30 {
                konto.vorab(&k, ip("10.0.0.1")).expect("frei");
            }
        }

        assert_eq!(
            konto.vorab(&kennung("k0@10.0.0.1"), ip("10.0.0.1")),
            gebremst(Grenze::Kennung, 1_000)
        );
    }

    #[cfg(test)]
    fn eintraege(konto: &Schreibkonto) -> (usize, usize) {
        let tabellen = konto.tabellen.lock().expect("Tabellen");
        (tabellen.kennungen.len(), tabellen.herkuenfte.len())
    }

    /// R00011 F0045 „Speicher“ (B0232): 1.025 Kennungen buchen je 1; 30 s später (alle wieder voll) bucht eine
    /// weitere → danach genau 1 Eintrag je Tabelle.
    #[test]
    fn volle_eimer_werden_ab_1024_eintraegen_entfernt() {
        let (konto, uhr) = konto();
        for nummer in 0..1_025_u32 {
            let herkunft = format!("10.0.{}.{}", nummer / 256, nummer % 256);
            konto
                .vorab(&kennung(&format!("k@{herkunft}")), ip(&herkunft))
                .expect("frei");
        }
        assert_eq!(eintraege(&konto), (1_025, 1_025));

        uhr.stellen(t(30_000));
        konto
            .vorab(&kennung("neu@10.9.9.9"), ip("10.9.9.9"))
            .expect("frei");

        assert_eq!(eintraege(&konto), (1, 1));
    }

    /// Beim Zugriff: ein Eimer, der nach dem Nachfüllen voll ist und nichts bucht, wird nicht abgelegt.
    #[test]
    fn ein_voller_eimer_wird_beim_zugriff_entfernt() {
        let (konto, uhr) = konto();
        let a = kennung("a@10.0.0.1");
        konto.vorab(&a, ip("10.0.0.1")).expect("frei");
        uhr.stellen(t(1_000));

        konto.rest(&a, ip("10.0.0.1"), 1).expect("frei");

        assert_eq!(eintraege(&konto), (0, 0));
    }
}
