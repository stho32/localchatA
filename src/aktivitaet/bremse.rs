//! Bremse der Aktivität (ADR-0051 Regel 9): Token-Eimer je Kennung **und Gespräch** (10, 1 je s) und je Herkunft
//! (120, 12 je s) — eigene Tabellen mit dem Eimer-Typ des Schreibkontos, **nicht** das Schreibkonto (PO Zeile 139).
//! Nur das Setzen kostet, 1 Einheit, gebucht nach allen 4xx-Prüfungen; Löschen und Lesen kosten nichts. Volle Eimer
//! werden nicht gehalten (wie ADR-0036 Regel 6).

use crate::gespraeche::{Gespraech, Kennung};
use crate::schreibkonto::eimer::{Eimer, Eimerart};
use crate::uhr::Zeit;
use std::collections::HashMap;
use std::hash::Hash;
use std::net::IpAddr;
use std::time::Duration;

/// Je Kennung und Gespräch: Vorrat 10, Nachschub 1 je Sekunde.
pub const JE_GESPRAECH: Eimerart = Eimerart {
    vorrat: 10,
    nachschub_mikro_je_ms: 1_000,
};

/// Je Herkunft: Vorrat 120, Nachschub 12 je Sekunde.
pub const JE_HERKUNFT: Eimerart = Eimerart {
    vorrat: 120,
    nachschub_mikro_je_ms: 12_000,
};

/// Mehr Einträge lösen einen Durchgang aus, der wieder volle Eimer entfernt.
const DURCHGANG_AB: usize = 1_024;

/// Welcher Eimer bremst; auf der Leitung und im Protokoll `gespraech` bzw. `herkunft`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grenze {
    Gespraech,
    Herkunft,
}

impl Grenze {
    pub fn als_text(self) -> &'static str {
        match self {
            Grenze::Gespraech => "gespraech",
            Grenze::Herkunft => "herkunft",
        }
    }
}

/// Abgewiesen: der Eimer, der länger braucht, und die Zeit, bis beide wieder eine Einheit tragen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gebremst {
    pub grenze: Grenze,
    pub wieder_in: Duration,
}

#[derive(Default)]
pub(super) struct Bremse {
    gespraeche: HashMap<(Kennung, Gespraech), Eimer>,
    herkuenfte: HashMap<IpAddr, Eimer>,
}

impl Bremse {
    /// Bucht 1 Einheit an beiden Eimern — alles oder nichts.
    pub(super) fn buchen(
        &mut self,
        kennung: &Kennung,
        gespraech: &Gespraech,
        herkunft: IpAddr,
        jetzt: Zeit,
    ) -> Result<(), Gebremst> {
        let schluessel = (kennung.clone(), gespraech.clone());
        let je_gespraech = nachgefuellt(&self.gespraeche, &schluessel, JE_GESPRAECH, jetzt);
        let je_herkunft = nachgefuellt(&self.herkuenfte, &herkunft, JE_HERKUNFT, jetzt);
        let reichen = je_gespraech.reicht(1) && je_herkunft.reicht(1);
        let (nach_gespraech, nach_herkunft) = if reichen {
            (je_gespraech.abgebucht(1), je_herkunft.abgebucht(1))
        } else {
            (je_gespraech, je_herkunft)
        };
        ablegen(
            &mut self.gespraeche,
            schluessel,
            nach_gespraech,
            JE_GESPRAECH,
        );
        ablegen(&mut self.herkuenfte, herkunft, nach_herkunft, JE_HERKUNFT);
        self.aufraeumen(jetzt);
        if reichen {
            Ok(())
        } else {
            Err(bremsen(&je_gespraech, &je_herkunft))
        }
    }

    fn aufraeumen(&mut self, jetzt: Zeit) {
        if self.gespraeche.len() > DURCHGANG_AB {
            self.gespraeche.retain(|_, eimer| {
                !eimer
                    .nachgefuellt(JE_GESPRAECH, jetzt)
                    .ist_voll(JE_GESPRAECH)
            });
        }
        if self.herkuenfte.len() > DURCHGANG_AB {
            self.herkuenfte
                .retain(|_, eimer| !eimer.nachgefuellt(JE_HERKUNFT, jetzt).ist_voll(JE_HERKUNFT));
        }
    }

    #[cfg(test)]
    pub(super) fn eintraege(&self) -> (usize, usize) {
        (self.gespraeche.len(), self.herkuenfte.len())
    }
}

/// Der Eimer zum Schlüssel, nachgefüllt bis `jetzt`; ohne Eintrag ein voller.
fn nachgefuellt<S: Hash + Eq>(
    tabelle: &HashMap<S, Eimer>,
    schluessel: &S,
    art: Eimerart,
    jetzt: Zeit,
) -> Eimer {
    tabelle
        .get(schluessel)
        .map_or(Eimer::voll(art, jetzt), |eimer| {
            eimer.nachgefuellt(art, jetzt)
        })
}

/// Ein voller Eimer wird nicht gehalten — er gleicht dem fehlenden.
fn ablegen<S: Hash + Eq>(
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

/// Die Zeit, bis beide Eimer eine Einheit tragen; die Grenze ist der Eimer, der länger braucht.
fn bremsen(je_gespraech: &Eimer, je_herkunft: &Eimer) -> Gebremst {
    let gespraech_ms = je_gespraech.wartezeit_ms(JE_GESPRAECH, 1);
    let herkunft_ms = je_herkunft.wartezeit_ms(JE_HERKUNFT, 1);
    let (grenze, ms) = if gespraech_ms >= herkunft_ms {
        (Grenze::Gespraech, gespraech_ms)
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

    fn t(ms: i64) -> Zeit {
        Zeit::parse_from_rfc3339("2026-10-30T09:00:00+01:00").expect("Zeit")
            + chrono::TimeDelta::milliseconds(ms)
    }

    /// Volle Eimer bleiben nicht liegen (Regel 9, wie ADR-0036 Regel 6): über 1.024 Einträge räumt ein Buchen alle
    /// wieder vollen ab; darunter bleiben sie bis dahin.
    #[test]
    fn volle_eimer_werden_ab_1024_eintraegen_entfernt() {
        let mut bremse = Bremse::default();
        let ip =
            |n: usize| -> IpAddr { format!("10.0.{}.{}", n / 200, n % 200).parse().expect("IP") };
        let anna = Kennung::bilden("anna@10.0.0.7").expect("Kennung");
        for n in 0..=DURCHGANG_AB {
            bremse
                .buchen(&anna, &Gespraech::channel(&format!("c{n}")), ip(n), t(0))
                .expect("frei");
        }
        let voll_belegt = bremse.eintraege();

        bremse
            .buchen(&anna, &Gespraech::channel("neu"), ip(9_999), t(10_000))
            .expect("frei");

        assert_eq!(voll_belegt, (DURCHGANG_AB + 1, DURCHGANG_AB + 1));
        assert_eq!(bremse.eintraege(), (1, 1), "nur der eben gebuchte bleibt");
    }
}
