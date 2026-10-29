//! Testauskunft für die Unit-Tests von Auskunft und Grenzschutz: feste Antworten, sperrbare Fragen, Zähler.

use super::namensauskunft::Namensauskunft;
use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

/// Testauskunft: feste Antworten, je Schlüssel ein Tor (gesperrt blockiert die Frage, bis es öffnet),
/// zählt Fragen und die höchste Zahl gleichzeitiger Fragen.
#[derive(Default)]
pub(crate) struct Testauskunft {
    pub(crate) rueckwaerts: HashMap<IpAddr, Vec<String>>,
    pub(crate) vorwaerts: HashMap<String, Vec<IpAddr>>,
    pub(crate) gesperrt: Mutex<HashSet<String>>,
    pub(crate) tor: Condvar,
    pub(crate) laufend: AtomicUsize,
    pub(crate) hoechstens: AtomicUsize,
    pub(crate) fragen: Mutex<Vec<String>>,
    /// Je Schlüssel: so lange hängt jede Frage danach (blockierender Schlaf), ohne Tor.
    pub(crate) haengt: Mutex<HashMap<String, Duration>>,
}

impl Testauskunft {
    pub(crate) fn mit(
        rueckwaerts: &[(&str, &str)],
        vorwaerts: &[(&str, &[&str])],
    ) -> Arc<Testauskunft> {
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

    pub(crate) fn sperren(&self, schluessel: &str) {
        self.gesperrt
            .lock()
            .expect("Tor")
            .insert(schluessel.to_string());
    }

    /// Jede Frage nach `schluessel` hängt `dauer` lang, dann kommt die Antwort.
    pub(crate) fn haengen(&self, schluessel: &str, dauer: Duration) {
        self.haengt
            .lock()
            .expect("Hängen")
            .insert(schluessel.to_string(), dauer);
    }

    pub(crate) fn oeffnen(&self, schluessel: &str) {
        self.gesperrt.lock().expect("Tor").remove(schluessel);
        self.tor.notify_all();
    }

    pub(crate) fn fragen(&self, art: &str, schluessel: &str) -> usize {
        let gesucht = format!("{art} {schluessel}");
        self.fragen
            .lock()
            .expect("Fragen")
            .iter()
            .filter(|frage| **frage == gesucht)
            .count()
    }

    pub(crate) fn alle_fragen(&self) -> usize {
        self.fragen.lock().expect("Fragen").len()
    }

    pub(crate) fn frage(&self, art: &str, schluessel: &str) {
        self.fragen
            .lock()
            .expect("Fragen")
            .push(format!("{art} {schluessel}"));
        let jetzt = self.laufend.fetch_add(1, Ordering::SeqCst) + 1;
        self.hoechstens.fetch_max(jetzt, Ordering::SeqCst);
        // Höchstens 10 s gesperrt: bricht ein Test vorher ab, gibt der Faden trotzdem frei, sonst wartete das
        // Ende der Laufzeit für immer auf ihn.
        let ende = Instant::now() + Duration::from_secs(10);
        let mut gesperrt = self.gesperrt.lock().expect("Tor");
        while gesperrt.contains(schluessel) && Instant::now() < ende {
            gesperrt = self
                .tor
                .wait_timeout(gesperrt, Duration::from_millis(100))
                .expect("Tor")
                .0;
        }
        drop(gesperrt);
        let haengt = self.haengt.lock().expect("Hängen").get(schluessel).copied();
        if let Some(dauer) = haengt {
            std::thread::sleep(dauer);
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

pub(crate) fn ip(text: &str) -> IpAddr {
    text.parse().expect("Adresse")
}
