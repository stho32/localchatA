//! Verbindungen begrenzen (B0017): wie viele offene Verbindungen eine Gegenstelle und alle zusammen halten
//! dürfen. Die Gesamtgrenze bleibt unter dem Deskriptor-Limit des Prozesses, damit die Annahme nie an
//! fehlenden Deskriptoren scheitert; die Grenze je Gegenstelle hält einen einzelnen Rechner davon ab,
//! die Gesamtgrenze allein zu füllen.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex, PoisonError};

/// Offene Verbindungen je Gegenstelle (IP-Adresse) — R00015, Lastprofil „Team-LAN“ (2026-10-28, A00001,
/// ADR-0010 Ergänzung R00015): im ungünstigen Fall laufen 50 Agenten und ein Mensch auf einem Rechner —
/// 50 wartende Long-Polls + 50 kurze Sendeverbindungen in der Spitze + Browser (1 Strom, bis 6 Abrufe) ≈ 107;
/// 160 ≈ 1,5 × Spitze. Je Gegenstelle gilt höchstens die Gesamtgrenze.
pub const JE_GEGENSTELLE: usize = 160;
/// Offene Verbindungen gesamt, solange das Deskriptor-Limit nicht weniger erlaubt.
const GESAMT: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Verbindungsgrenzen {
    pub je_gegenstelle: usize,
    pub gesamt: usize,
}

impl Verbindungsgrenzen {
    /// Höchstens die Hälfte der weichen Deskriptorgrenze — die andere Hälfte bleibt für Logdateien,
    /// Sperre und die kurz geöffneten, sofort wieder geschlossenen Verbindungen über der Grenze.
    pub fn fuer_deskriptoren(weiche_grenze: Option<u64>) -> Self {
        let gesamt = weiche_grenze
            .map(|grenze| usize::try_from(grenze / 2).unwrap_or(usize::MAX))
            .map_or(GESAMT, |haelfte| haelfte.clamp(1, GESAMT));
        Verbindungsgrenzen {
            je_gegenstelle: JE_GEGENSTELLE.min(gesamt),
            gesamt,
        }
    }

    /// Die Grenzen für diesen Prozess.
    pub fn fuer_diesen_prozess() -> Self {
        Self::fuer_deskriptoren(weiche_deskriptorgrenze())
    }
}

#[cfg(unix)]
fn weiche_deskriptorgrenze() -> Option<u64> {
    rustix::process::getrlimit(rustix::process::Resource::Nofile).current
}

#[cfg(not(unix))]
fn weiche_deskriptorgrenze() -> Option<u64> {
    None
}

/// Zählt offene Verbindungen je Gegenstelle und gesamt.
#[derive(Debug)]
struct Zaehlung {
    grenzen: Verbindungsgrenzen,
    je_gegenstelle: HashMap<IpAddr, usize>,
    gesamt: usize,
}

impl Zaehlung {
    /// Belegt einen Platz, wenn beide Grenzen es erlauben; sonst bleibt alles, wie es war.
    fn belegen(&mut self, gegenstelle: IpAddr) -> bool {
        let offen = self.je_gegenstelle.get(&gegenstelle).copied().unwrap_or(0);
        if self.gesamt >= self.grenzen.gesamt || offen >= self.grenzen.je_gegenstelle {
            return false;
        }
        self.je_gegenstelle.insert(gegenstelle, offen + 1);
        self.gesamt += 1;
        true
    }

    fn freigeben(&mut self, gegenstelle: IpAddr) {
        if let Some(offen) = self.je_gegenstelle.get_mut(&gegenstelle) {
            *offen -= 1;
            if *offen == 0 {
                self.je_gegenstelle.remove(&gegenstelle);
            }
            self.gesamt -= 1;
        }
    }
}

/// Die geteilte Zählung aller offenen Verbindungen einer Instanz.
#[derive(Debug, Clone)]
pub struct Belegung(Arc<Mutex<Zaehlung>>);

/// Ein belegter Platz; er wird frei, sobald die Verbindung endet und der Platz fällt.
#[derive(Debug)]
pub struct Platz {
    belegung: Belegung,
    gegenstelle: IpAddr,
}

impl Belegung {
    pub fn neu(grenzen: Verbindungsgrenzen) -> Self {
        Belegung(Arc::new(Mutex::new(Zaehlung {
            grenzen,
            je_gegenstelle: HashMap::new(),
            gesamt: 0,
        })))
    }

    /// Ein Platz für die Gegenstelle oder keiner, wenn eine Grenze erreicht ist.
    pub fn zulassen(&self, gegenstelle: IpAddr) -> Option<Platz> {
        self.zaehlung().belegen(gegenstelle).then(|| Platz {
            belegung: self.clone(),
            gegenstelle,
        })
    }

    /// Die Grenze, die eine Gegenstelle gerade abweist: gesamt, wenn sie voll ist, sonst die je Gegenstelle.
    pub fn erreichte_grenze(&self) -> usize {
        let zaehlung = self.zaehlung();
        if zaehlung.gesamt >= zaehlung.grenzen.gesamt {
            zaehlung.grenzen.gesamt
        } else {
            zaehlung.grenzen.je_gegenstelle
        }
    }

    fn zaehlung(&self) -> std::sync::MutexGuard<'_, Zaehlung> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Drop for Platz {
    fn drop(&mut self) {
        self.belegung.zaehlung().freigeben(self.gegenstelle);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: IpAddr = IpAddr::V4(std::net::Ipv4Addr::new(192, 168, 1, 20));
    const B: IpAddr = IpAddr::V4(std::net::Ipv4Addr::new(192, 168, 1, 21));

    fn belegung(je_gegenstelle: usize, gesamt: usize) -> Belegung {
        Belegung::neu(Verbindungsgrenzen {
            je_gegenstelle,
            gesamt,
        })
    }

    #[test]
    fn eine_gegenstelle_bekommt_genau_ihre_grenze_und_eine_andere_trotzdem_einen_platz() {
        let belegung = belegung(2, 10);
        let plaetze: Vec<_> = (0..3).map(|_| belegung.zulassen(A)).collect();

        assert_eq!(plaetze.iter().filter(|platz| platz.is_some()).count(), 2);
        assert!(belegung.zulassen(B).is_some());
    }

    #[test]
    fn ueber_der_gesamtgrenze_wird_auch_eine_neue_gegenstelle_abgewiesen() {
        let belegung = belegung(2, 3);
        let _a = [belegung.zulassen(A), belegung.zulassen(A)];
        let _b = belegung.zulassen(B);

        assert!(belegung.zulassen(B).is_none());
    }

    #[test]
    fn ein_freigegebener_platz_steht_wieder_zur_verfuegung() {
        let belegung = belegung(1, 1);
        let platz = belegung.zulassen(A);
        assert!(belegung.zulassen(A).is_none());

        drop(platz);

        assert_eq!(belegung.zaehlung().gesamt, 0);
        assert!(belegung.zaehlung().je_gegenstelle.is_empty());
        let neuer = belegung.zulassen(A);
        assert!(neuer.is_some());
        assert_eq!(belegung.zaehlung().gesamt, 1);
    }

    /// R00015 G1: je Gegenstelle 160 bei ausreichenden Deskriptoren, sonst `min(160, gesamt)`.
    #[test]
    fn die_gesamtgrenze_ist_hoechstens_die_haelfte_der_weichen_deskriptorgrenze() {
        let grenzen = |je_gegenstelle, gesamt| Verbindungsgrenzen {
            je_gegenstelle,
            gesamt,
        };
        assert_eq!(
            Verbindungsgrenzen::fuer_deskriptoren(Some(1024)),
            grenzen(160, 512)
        );
        assert_eq!(
            Verbindungsgrenzen::fuer_deskriptoren(Some(322)),
            grenzen(160, 161)
        );
        assert_eq!(
            Verbindungsgrenzen::fuer_deskriptoren(Some(318)),
            grenzen(159, 159)
        );
        assert_eq!(
            Verbindungsgrenzen::fuer_deskriptoren(Some(200)),
            grenzen(100, 100)
        );
        assert_eq!(
            Verbindungsgrenzen::fuer_deskriptoren(Some(64)),
            grenzen(32, 32)
        );
    }

    #[test]
    fn ohne_oder_mit_grosser_deskriptorgrenze_gelten_die_festen_grenzen() {
        let fest = Verbindungsgrenzen {
            je_gegenstelle: 160,
            gesamt: 512,
        };
        assert_eq!(Verbindungsgrenzen::fuer_deskriptoren(None), fest);
        assert_eq!(Verbindungsgrenzen::fuer_deskriptoren(Some(1 << 20)), fest);
        assert_eq!(Verbindungsgrenzen::fuer_deskriptoren(Some(u64::MAX)), fest);
        assert_eq!(Verbindungsgrenzen::fuer_deskriptoren(Some(1)).gesamt, 1);
    }
}
