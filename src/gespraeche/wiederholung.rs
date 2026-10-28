//! Wiederholschlüssel (B0237, ADR-0034 Regel 6): `(Kennung, Schlüssel) → (Gespräch, Zeit, Prüfsumme)`, höchstens
//! die 4.096 jüngsten Einträge und höchstens 10 Minuten ab dem Merken. Rein, mit gestellter Zeit; gehalten unter der
//! Schreibsperre der Gespräche, nur im Speicher.

use super::{Gespraech, Kennung};
use crate::uhr::Zeit;

/// Höchstens so viele Schlüssel werden gemerkt — die jüngsten.
pub const HOECHSTENS_EINTRAEGE: usize = 4_096;
/// So lange (ms ab dem Merken) gilt ein Schlüssel; danach ist er neu.
pub const HOECHSTENS_ALTER_MS: i64 = 600_000;

/// Was ein Schlüssel für einen Sendeauftrag bedeutet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pruefung {
    /// Unbekannt (oder vergessen): senden und danach merken.
    Neu,
    /// Dieselbe Nachricht noch einmal: die ursprüngliche Zeit, nichts schreiben.
    Wiederholt { zeit: Zeit },
    /// Der Schlüssel gehört zu einem anderen Text oder Gespräch.
    Anders,
}

#[derive(Debug, Clone)]
pub struct Wiederholung {
    /// Älteste zuerst — in der Folge des Merkens.
    eintraege: std::collections::VecDeque<Eintrag>,
}

#[derive(Debug, Clone)]
struct Eintrag {
    kennung: Kennung,
    schluessel: String,
    gespraech: Gespraech,
    pruefsumme: u64,
    zeit: Zeit,
    gemerkt: Zeit,
}

impl Default for Wiederholung {
    fn default() -> Self {
        Wiederholung {
            eintraege: std::collections::VecDeque::with_capacity(HOECHSTENS_EINTRAEGE),
        }
    }
}

impl Wiederholung {
    pub fn pruefen(
        &self,
        kennung: &Kennung,
        schluessel: &str,
        gespraech: &Gespraech,
        pruefsumme: u64,
        jetzt: Zeit,
    ) -> Pruefung {
        let Some(eintrag) = self.finden(kennung, schluessel) else {
            return Pruefung::Neu;
        };
        if abgelaufen(eintrag, jetzt) {
            return Pruefung::Neu;
        }
        if &eintrag.gespraech != gespraech || eintrag.pruefsumme != pruefsumme {
            return Pruefung::Anders;
        }
        Pruefung::Wiederholt { zeit: eintrag.zeit }
    }

    /// Merkt eine gesendete Nachricht; ein abgelaufener Eintrag desselben Schlüssels weicht, ebenso alles über
    /// der Höchstzahl und alles Abgelaufene am alten Ende.
    pub fn merken(
        &mut self,
        kennung: &Kennung,
        schluessel: &str,
        gespraech: &Gespraech,
        pruefsumme: u64,
        zeit: Zeit,
        jetzt: Zeit,
    ) {
        self.eintraege
            .retain(|eintrag| !(&eintrag.kennung == kennung && eintrag.schluessel == schluessel));
        while self
            .eintraege
            .front()
            .is_some_and(|aeltester| abgelaufen(aeltester, jetzt))
        {
            self.eintraege.pop_front();
        }
        if self.eintraege.len() == HOECHSTENS_EINTRAEGE {
            self.eintraege.pop_front();
        }
        self.eintraege.push_back(Eintrag {
            kennung: kennung.clone(),
            schluessel: schluessel.to_string(),
            gespraech: gespraech.clone(),
            pruefsumme,
            zeit,
            gemerkt: jetzt,
        });
    }

    fn finden(&self, kennung: &Kennung, schluessel: &str) -> Option<&Eintrag> {
        self.eintraege
            .iter()
            .rev()
            .find(|eintrag| &eintrag.kennung == kennung && eintrag.schluessel == schluessel)
    }
}

fn abgelaufen(eintrag: &Eintrag, jetzt: Zeit) -> bool {
    (jetzt - eintrag.gemerkt).num_milliseconds() > HOECHSTENS_ALTER_MS
}

/// Prüfsumme des Texts, wie er gesendet wurde (vor dem Umformen ins Log); fest über Läufe hinweg.
pub fn pruefsumme(text: &str) -> u64 {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(ms: i64) -> Zeit {
        Zeit::parse_from_rfc3339("2026-10-28T12:00:00+01:00").expect("Zeit")
            + chrono::Duration::milliseconds(ms)
    }

    fn kennung(text: &str) -> Kennung {
        Kennung::bilden(text).expect("Kennung")
    }

    fn allgemein() -> Gespraech {
        Gespraech::channel("allgemein")
    }

    /// R00012 F0046 „Schlüssel merken“: gleich → wiederholt mit derselben Zeit; anderer Text, anderer Channel →
    /// anders; andere Kennung → neu.
    #[test]
    fn derselbe_schluessel_ist_wiederholt_anders_oder_je_kennung_neu() {
        let beta = kennung("beta@127.0.0.1");
        let gamma = kennung("gamma@127.0.0.1");
        let mut gemerkt = Wiederholung::default();
        let x = pruefsumme("x");

        let zuerst = gemerkt.pruefen(&beta, "k1", &allgemein(), x, t(0));
        gemerkt.merken(&beta, "k1", &allgemein(), x, t(7), t(0));

        assert_eq!(zuerst, Pruefung::Neu);
        assert_eq!(
            gemerkt.pruefen(&beta, "k1", &allgemein(), x, t(1)),
            Pruefung::Wiederholt { zeit: t(7) }
        );
        assert_eq!(
            gemerkt.pruefen(&beta, "k1", &allgemein(), pruefsumme("y"), t(1)),
            Pruefung::Anders
        );
        assert_eq!(
            gemerkt.pruefen(&beta, "k1", &Gespraech::channel("technik"), x, t(1)),
            Pruefung::Anders
        );
        assert_eq!(
            gemerkt.pruefen(&gamma, "k1", &allgemein(), x, t(1)),
            Pruefung::Neu
        );
    }

    /// Vom Merken an nach 600 000 ms wiederholt, nach 600 001 ms neu (Mutanten 10 min ± 1 s werden rot).
    #[test]
    fn ein_schluessel_gilt_genau_zehn_minuten_ab_dem_merken() {
        let beta = kennung("beta@127.0.0.1");
        let mut gemerkt = Wiederholung::default();
        let x = pruefsumme("x");
        gemerkt.merken(&beta, "k1", &allgemein(), x, t(5), t(1_000));

        assert_eq!(
            gemerkt.pruefen(&beta, "k1", &allgemein(), x, t(1_000 + 600_000)),
            Pruefung::Wiederholt { zeit: t(5) }
        );
        assert_eq!(
            gemerkt.pruefen(&beta, "k1", &allgemein(), x, t(1_000 + 600_001)),
            Pruefung::Neu
        );
    }

    /// Nach 4 097 weiteren Schlüsseln ist der älteste vergessen, ebenso der 4 097-jüngste; der 4 096-jüngste ist
    /// noch bekannt (Mutanten 4 095 und 4 097 Einträge werden rot).
    #[test]
    fn hoechstens_die_4096_juengsten_schluessel_bleiben() {
        let beta = kennung("beta@127.0.0.1");
        let mut gemerkt = Wiederholung::default();
        let x = pruefsumme("x");
        gemerkt.merken(&beta, "aeltester", &allgemein(), x, t(0), t(0));
        for nummer in 1..=4_097 {
            gemerkt.merken(
                &beta,
                &format!("w{nummer}"),
                &allgemein(),
                x,
                t(nummer),
                t(nummer),
            );
        }
        let jetzt = t(5_000);

        assert_eq!(
            gemerkt.pruefen(&beta, "w2", &allgemein(), x, jetzt),
            Pruefung::Wiederholt { zeit: t(2) },
            "der 4 096-jüngste"
        );
        assert_eq!(
            gemerkt.pruefen(&beta, "w1", &allgemein(), x, jetzt),
            Pruefung::Neu,
            "der 4 097-jüngste"
        );
        assert_eq!(
            gemerkt.pruefen(&beta, "aeltester", &allgemein(), x, jetzt),
            Pruefung::Neu
        );
    }

    /// Ein abgelaufener Schlüssel kann neu gemerkt werden und gilt dann mit der neuen Zeit.
    #[test]
    fn ein_abgelaufener_schluessel_wird_neu_gemerkt() {
        let beta = kennung("beta@127.0.0.1");
        let mut gemerkt = Wiederholung::default();
        let x = pruefsumme("x");
        gemerkt.merken(&beta, "k1", &allgemein(), x, t(1), t(0));
        gemerkt.merken(
            &beta,
            "k1",
            &allgemein(),
            pruefsumme("y"),
            t(700_000),
            t(700_000),
        );

        assert_eq!(
            gemerkt.pruefen(&beta, "k1", &allgemein(), pruefsumme("y"), t(700_001)),
            Pruefung::Wiederholt { zeit: t(700_000) }
        );
        assert_eq!(gemerkt.eintraege.len(), 1);
    }
}
