//! Nachholkonto (ADR-0054 Ergänzung „Nachholen von `warten` …“ Regel 3 und 4, B0755): zwei Eimer in geprüften
//! Sätzen je Kennung und je Herkunft, im Speicher — eine zweite Instanz des Eimer-Typs des Schreibkontos, nie ein
//! Buchen am Schreibkonto (E9). Ein Warte-Aufruf reserviert das Budget in beiden Eimern (alles oder nichts) und
//! erstattet nach dem Nachholen, was er nicht geprüft hat.

use super::eimer::Eimerart;
use super::{Buchung, Gebremst, Kontoarten, Tabellen};
use crate::gespraeche::Kennung;
use crate::uhr::Uhr;
use std::net::IpAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

/// K3: je Kennung Vorrat 100.000 Sätze, Nachschub 20.000 je Sekunde.
pub const JE_KENNUNG: Eimerart = Eimerart {
    vorrat: 100_000,
    nachschub_mikro_je_ms: 20_000_000,
};

/// K4: je Herkunft Vorrat 500.000 Sätze, Nachschub 60.000 je Sekunde.
pub const JE_HERKUNFT: Eimerart = Eimerart {
    vorrat: 500_000,
    nachschub_mikro_je_ms: 60_000_000,
};

/// Die Eimer des Nachholkontos.
const NACHHOLEN: Kontoarten = Kontoarten {
    kennung: JE_KENNUNG,
    herkunft: JE_HERKUNFT,
};

pub struct Nachholkonto {
    uhr: Arc<dyn Uhr>,
    tabellen: Mutex<Tabellen>,
    gebremst: AtomicU64,
}

impl Nachholkonto {
    pub fn neu(uhr: Arc<dyn Uhr>) -> Nachholkonto {
        Nachholkonto {
            uhr,
            tabellen: Mutex::default(),
            gebremst: AtomicU64::new(0),
        }
    }

    /// Reserviert `saetze` in beiden Eimern oder in keinem; reicht einer nicht, der Eimer, der länger braucht, und
    /// die Zeit, bis beide die Reservierung tragen.
    pub fn reservieren(
        &self,
        kennung: &Kennung,
        herkunft: IpAddr,
        saetze: u64,
    ) -> Result<(), Gebremst> {
        let jetzt = self.uhr.jetzt();
        let gebucht = self.tabellen().buchen(
            NACHHOLEN,
            kennung,
            herkunft,
            Buchung {
                einheiten: saetze,
                volle_kosten: saetze,
            },
            jetzt,
        );
        if gebucht.is_err() {
            self.gebremst.fetch_add(1, Ordering::Relaxed);
        }
        gebucht
    }

    /// Gibt `saetze` an beide Eimer zurück, höchstens bis zum Vorrat.
    pub fn erstatten(&self, kennung: &Kennung, herkunft: IpAddr, saetze: u64) {
        let jetzt = self.uhr.jetzt();
        self.tabellen()
            .gutschreiben(NACHHOLEN, kennung, herkunft, saetze, jetzt);
    }

    fn tabellen(&self) -> MutexGuard<'_, Tabellen> {
        self.tabellen.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Zahl der Bremsungen seit dem Start (`/api/gesundheit` `nachholen.gebremst`).
    pub fn gebremst(&self) -> u64 {
        self.gebremst.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::super::Grenze;
    use super::*;
    use crate::uhr::{GestellteUhr, Zeit};
    use std::time::Duration;

    /// Die Reservierung eines Aufrufs (K5 = Budget K1).
    const R: u64 = 2_000;

    fn t(ms: i64) -> Zeit {
        Zeit::parse_from_rfc3339("2026-10-30T12:00:00+01:00").expect("Zeit")
            + chrono::Duration::milliseconds(ms)
    }

    fn konto() -> (Nachholkonto, Arc<GestellteUhr>) {
        let uhr = Arc::new(GestellteUhr::neu(t(0)));
        (Nachholkonto::neu(uhr.clone()), uhr)
    }

    fn kennung(text: &str) -> Kennung {
        Kennung::bilden(text).expect("Kennung")
    }

    fn ip(text: &str) -> IpAddr {
        text.parse().expect("IP")
    }

    fn a() -> Kennung {
        kennung("a@127.0.0.1")
    }

    fn h() -> IpAddr {
        ip("127.0.0.1")
    }

    /// So viele Reservierungen gelingen `wer` in Folge, höchstens `hoechstens`.
    fn gelingen(konto: &Nachholkonto, wer: &Kennung, hoechstens: usize) -> usize {
        (0..hoechstens)
            .take_while(|_| konto.reservieren(wer, h(), R).is_ok())
            .count()
    }

    /// N1 K3: 50 × 2.000 ok, die 51. → `Gebremst{kennung}` mit `wieder_in` 100 ms; nach 100 ms geht eine wieder.
    #[test]
    fn fuenfzig_reservierungen_je_kennung_die_einundfuenfzigste_ist_gebremst() {
        let (konto, uhr) = konto();

        assert_eq!(gelingen(&konto, &a(), 50), 50);
        assert_eq!(
            konto.reservieren(&a(), h(), R),
            Err(Gebremst {
                grenze: Grenze::Kennung,
                wieder_in: Duration::from_millis(100),
            })
        );
        uhr.stellen(t(99));
        assert!(konto.reservieren(&a(), h(), R).is_err());
        uhr.stellen(t(100));
        assert!(konto.reservieren(&a(), h(), R).is_ok());
        assert_eq!(konto.gebremst(), 2);
    }

    /// N1 K4: 250 Reservierungen verteilt auf 16 Kennungen einer Herkunft ok, die 251. → `Gebremst{herkunft}`
    /// (2.000 Sätze bei 60.000 je Sekunde: 34 ms).
    #[test]
    fn zweihundertfuenfzig_reservierungen_je_herkunft_die_naechste_ist_gebremst() {
        let (konto, _) = konto();
        let kennungen: Vec<Kennung> = (0..16)
            .map(|nummer| kennung(&format!("k{nummer}@127.0.0.1")))
            .collect();

        let gelungen = (0..250)
            .filter(|nummer| konto.reservieren(&kennungen[nummer % 16], h(), R).is_ok())
            .count();

        assert_eq!(gelungen, 250);
        assert_eq!(
            konto.reservieren(&kennungen[250 % 16], h(), R),
            Err(Gebremst {
                grenze: Grenze::Herkunft,
                wieder_in: Duration::from_millis(34),
            })
        );
    }

    /// N1 alles oder nichts: reicht die Herkunft, aber nicht die Kennung, bleibt die Herkunft unverändert — nach 100
    /// abgewiesenen Versuchen von `a` tragen andere Kennungen der Herkunft genau die übrigen 200 Reservierungen.
    #[test]
    fn eine_abgewiesene_kennung_bucht_nichts_an_der_herkunft() {
        let (konto, _) = konto();
        assert_eq!(gelingen(&konto, &a(), 50), 50);

        let abgewiesen = (0..100)
            .filter(|_| konto.reservieren(&a(), h(), R).is_err())
            .count();
        let andere = (0..201)
            .filter(|nummer| {
                konto
                    .reservieren(&kennung(&format!("k{}@127.0.0.1", nummer % 8)), h(), R)
                    .is_ok()
            })
            .count();

        assert_eq!((abgewiesen, andere), (100, 200));
    }

    /// N1 alles oder nichts, umgekehrt: ist die Herkunft leer, bleibt der Eimer der Kennung unverändert.
    #[test]
    fn eine_leere_herkunft_bucht_nichts_an_der_kennung() {
        let (konto, _) = konto();
        for nummer in 0..250 {
            konto
                .reservieren(&kennung(&format!("k{}@127.0.0.1", nummer % 16)), h(), R)
                .expect("Vorrat der Herkunft");
        }
        for _ in 0..100 {
            assert!(konto.reservieren(&a(), h(), R).is_err());
        }

        assert_eq!(gelingen(&konto, &a(), 1), 0);
        let anderswo = gelingen_von(&konto, &a(), ip("127.0.0.2"), 51);
        assert_eq!(anderswo, 50, "die Kennung hat ihren vollen Vorrat");
    }

    fn gelingen_von(konto: &Nachholkonto, wer: &Kennung, von: IpAddr, hoechstens: usize) -> usize {
        (0..hoechstens)
            .take_while(|_| konto.reservieren(wer, von, R).is_ok())
            .count()
    }

    /// N1 Erstattung: 1.000 × (reservieren 2.000, erstatten 2.000) bei stehender Uhr gelingen alle.
    #[test]
    fn tausend_leichte_aufrufe_bei_stehender_uhr_gelingen_alle() {
        let (konto, _) = konto();

        let gelungen = (0..1_000)
            .filter(|_| {
                let ok = konto.reservieren(&a(), h(), R).is_ok();
                konto.erstatten(&a(), h(), R);
                ok
            })
            .count();

        assert_eq!(gelungen, 1_000);
        assert_eq!(konto.gebremst(), 0);
    }

    /// N1 Erstattung teilweise: wer 500 prüft, erstattet 1.500 — nach 197 solchen Aufrufen (98.500 gezahlt) bleiben
    /// 1.500, die nächste Reservierung von 2.000 ist gebremst.
    #[test]
    fn wer_teilweise_prueft_zahlt_was_er_geprueft_hat() {
        let (konto, _) = konto();

        let gelungen = (0..201)
            .take_while(|_| {
                let ok = konto.reservieren(&a(), h(), R).is_ok();
                if ok {
                    konto.erstatten(&a(), h(), R - 500);
                }
                ok
            })
            .count();

        assert_eq!(gelungen, 197);
    }

    /// N1 `erstatten` ist bis zum Vorrat gedeckelt — an beiden Eimern.
    #[test]
    fn erstatten_hebt_keinen_eimer_ueber_den_vorrat() {
        let (konto, _) = konto();
        konto.erstatten(&a(), h(), 50_000);
        assert_eq!(gelingen(&konto, &a(), 60), 50);

        let (konto, _) = konto_mit_erstattung_der_herkunft();
        let gelungen = (0..300)
            .filter(|nummer| {
                konto
                    .reservieren(&kennung(&format!("k{}@127.0.0.1", nummer % 16)), h(), R)
                    .is_ok()
            })
            .count();
        assert_eq!(gelungen, 250);
    }

    fn konto_mit_erstattung_der_herkunft() -> (Nachholkonto, Arc<GestellteUhr>) {
        let (konto, uhr) = konto();
        konto.erstatten(&kennung("x@127.0.0.1"), h(), 300_000);
        (konto, uhr)
    }

    /// N1 Uhr zurück: kein Nachschub; ab dem neuen „jetzt“ läuft er wieder.
    #[test]
    fn springt_die_uhr_zurueck_gibt_es_keinen_nachschub() {
        let (konto, uhr) = konto();
        assert_eq!(gelingen(&konto, &a(), 50), 50);

        uhr.stellen(t(-3_600_000));
        assert!(konto.reservieren(&a(), h(), R).is_err());
        uhr.stellen(t(-3_600_000 + 100));
        assert!(konto.reservieren(&a(), h(), R).is_ok());
    }

    /// N1 volle Eimer werden entfernt: nach Reservieren und vollständigem Erstatten hält das Konto keinen Eintrag.
    #[test]
    fn volle_eimer_werden_entfernt() {
        let (konto, uhr) = konto();
        konto.reservieren(&a(), h(), R).expect("Vorrat");
        assert_eq!(eintraege(&konto), (1, 1));

        konto.erstatten(&a(), h(), R);
        assert_eq!(eintraege(&konto), (0, 0));

        konto.reservieren(&a(), h(), R).expect("Vorrat");
        uhr.stellen(t(1_000));
        konto
            .reservieren(&kennung("b@127.0.0.1"), h(), 0)
            .expect("nichts");
        konto.erstatten(&a(), h(), 0);
        assert_eq!(eintraege(&konto), (0, 0));
    }

    fn eintraege(konto: &Nachholkonto) -> (usize, usize) {
        let tabellen = konto.tabellen.lock().expect("Tabellen");
        (tabellen.kennungen.len(), tabellen.herkuenfte.len())
    }
}
