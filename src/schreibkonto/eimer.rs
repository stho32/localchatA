//! Token-Eimer und Kosten (B0230, ADR-0036 Regeln 2, 3): rein, mit gestellter Zeit. Gerechnet wird in
//! Mikro-Einheiten, damit Nachschub je Millisekunde ganzzahlig bleibt (1/s = 1.000 Mikro-Einheiten je ms).

use crate::uhr::Zeit;

const MIKRO: u64 = 1_000_000;

/// Vorrat und Nachschub einer Eimerart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Eimerart {
    /// Höchstens so viele Einheiten.
    pub vorrat: u64,
    /// Nachschub in Mikro-Einheiten je Millisekunde (1 Einheit/s = 1.000).
    pub nachschub_mikro_je_ms: u64,
}

/// Je Kennung: Vorrat 30, Nachschub 1 je Sekunde.
pub const JE_KENNUNG: Eimerart = Eimerart {
    vorrat: 30,
    nachschub_mikro_je_ms: 1_000,
};

/// Je Herkunft: Vorrat 120, Nachschub 8 je Sekunde.
pub const JE_HERKUNFT: Eimerart = Eimerart {
    vorrat: 120,
    nachschub_mikro_je_ms: 8_000,
};

/// Ein Eimer: Stand in Mikro-Einheiten und Zeitpunkt des letzten Nachfüllens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Eimer {
    stand_mikro: u64,
    zuletzt: Zeit,
}

impl Eimer {
    pub fn voll(art: Eimerart, jetzt: Zeit) -> Eimer {
        Eimer {
            stand_mikro: art.vorrat * MIKRO,
            zuletzt: jetzt,
        }
    }

    /// Nachschub = vergangene Zeit × Rate, höchstens bis zum Vorrat; springt die Uhr zurück, kein Nachschub und
    /// `zuletzt := jetzt`.
    pub fn nachgefuellt(self, art: Eimerart, jetzt: Zeit) -> Eimer {
        let vergangen_ms = u64::try_from((jetzt - self.zuletzt).num_milliseconds()).unwrap_or(0);
        let nachschub = vergangen_ms.saturating_mul(art.nachschub_mikro_je_ms);
        Eimer {
            stand_mikro: self
                .stand_mikro
                .saturating_add(nachschub)
                .min(art.vorrat * MIKRO),
            zuletzt: jetzt,
        }
    }

    pub fn reicht(&self, einheiten: u64) -> bool {
        self.stand_mikro >= einheiten * MIKRO
    }

    pub fn ist_voll(&self, art: Eimerart) -> bool {
        self.stand_mikro >= art.vorrat * MIKRO
    }

    /// Bucht ab; der Aufrufer hat `reicht` geprüft.
    pub fn abgebucht(self, einheiten: u64) -> Eimer {
        Eimer {
            stand_mikro: self.stand_mikro.saturating_sub(einheiten * MIKRO),
            ..self
        }
    }

    /// So viele ms, bis der Eimer `einheiten` trägt (0, wenn er sie schon trägt), aufgerundet.
    pub fn wartezeit_ms(&self, art: Eimerart, einheiten: u64) -> u64 {
        let fehlt = (einheiten * MIKRO).saturating_sub(self.stand_mikro);
        fehlt.div_ceil(art.nachschub_mikro_je_ms)
    }
}

/// Kosten einer Nachricht: ⌈Bytes / 4.096⌉, mindestens 1 (ADR-0036 Regel 3).
pub fn kosten_senden(bytes: usize) -> u32 {
    let kosten = bytes.div_ceil(4_096).max(1);
    u32::try_from(kosten).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(ms: i64) -> Zeit {
        Zeit::parse_from_rfc3339("2026-10-28T12:00:00+01:00").expect("Zeit")
            + chrono::Duration::milliseconds(ms)
    }

    /// 30 Einheiten abbuchen, dann reicht 1 nicht mehr; nach 999 ms nicht, nach 1.000 ms wieder.
    #[test]
    fn eimer_je_kennung_als_spanne() {
        let mut eimer = Eimer::voll(JE_KENNUNG, t(0));
        for _ in 0..30 {
            eimer = eimer.nachgefuellt(JE_KENNUNG, t(0));
            assert!(eimer.reicht(1));
            eimer = eimer.abgebucht(1);
        }

        assert!(!eimer.nachgefuellt(JE_KENNUNG, t(0)).reicht(1));
        assert_eq!(eimer.wartezeit_ms(JE_KENNUNG, 1), 1_000);
        assert!(!eimer.nachgefuellt(JE_KENNUNG, t(999)).reicht(1));
        assert!(eimer.nachgefuellt(JE_KENNUNG, t(1_000)).reicht(1));
    }

    #[test]
    fn eimer_je_herkunft_traegt_120_und_fuellt_8_je_sekunde() {
        let leer = Eimer::voll(JE_HERKUNFT, t(0)).abgebucht(120);

        assert!(Eimer::voll(JE_HERKUNFT, t(0)).reicht(120));
        assert!(!Eimer::voll(JE_HERKUNFT, t(0)).reicht(121));
        assert_eq!(leer.wartezeit_ms(JE_HERKUNFT, 1), 125);
        assert!(!leer.nachgefuellt(JE_HERKUNFT, t(124)).reicht(1));
        assert!(leer.nachgefuellt(JE_HERKUNFT, t(125)).reicht(1));
    }

    #[test]
    fn nachschub_hoechstens_bis_zum_vorrat() {
        let eimer = Eimer::voll(JE_KENNUNG, t(0))
            .abgebucht(1)
            .nachgefuellt(JE_KENNUNG, t(3_600_000));

        assert!(eimer.ist_voll(JE_KENNUNG));
        assert!(!eimer.reicht(31));
    }

    /// Uhr 1 h zurück: kein Nachschub, kein Abzug, kein Fehler; ab dem neuen `jetzt` läuft der Nachschub.
    #[test]
    fn springt_die_uhr_zurueck_gibt_es_keinen_nachschub() {
        let leer = Eimer::voll(JE_KENNUNG, t(0)).abgebucht(30);

        let zurueck = leer.nachgefuellt(JE_KENNUNG, t(-3_600_000));

        assert!(!zurueck.reicht(1));
        assert_eq!(zurueck.wartezeit_ms(JE_KENNUNG, 1), 1_000);
        assert!(zurueck.nachgefuellt(JE_KENNUNG, t(-3_599_000)).reicht(1));
    }

    #[test]
    fn kosten_nach_groesse() {
        for (bytes, kosten) in [(1, 1), (4_096, 1), (4_097, 2), (16_384, 4), (0, 1)] {
            assert_eq!(kosten_senden(bytes), kosten, "{bytes}");
        }
    }
}
