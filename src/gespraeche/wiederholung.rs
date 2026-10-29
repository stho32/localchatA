//! Wiederholschlüssel (B0237, ADR-0034 Regel 6 mit Ergänzungen Karte #36): `(Kennung, Schlüssel) → (Gespräch, Zeit,
//! Prüfsumme)`, höchstens 640 je Kennung und 36.000 insgesamt, je höchstens 10 Minuten ab dem Merken. Rein, mit
//! gestellter Zeit; gehalten unter der Schreibsperre der Gespräche, nur im Speicher. Der Speicher meldet eine
//! Verdrängung vor Ablauf nur; Protokoll und Zähler macht der Aufrufer nach der Sperre (Regel E1.7).

use super::{Gespraech, Kennung};
use crate::uhr::Zeit;
use std::collections::{HashMap, VecDeque};
use std::fmt;

/// Höchstens so viele Schlüssel je Kennung (ADR-0034 Ergänzung Karte #36: Bedarf 630 aus dem Schreibkonto + Reserve).
pub const HOECHSTENS_JE_KENNUNG: usize = 640;
/// Höchstens so viele Schlüssel insgesamt — das Doppelte der Spitze des Lastprofils über 10 Minuten.
pub const HOECHSTENS_GESAMT: usize = 36_000;
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

/// Welche Grenze einen Eintrag vor Ablauf verdrängt hat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grenze {
    Kennung,
    Gesamt,
}

impl Grenze {
    /// Das Wort im Protokoll (`grenze=`).
    pub fn als_wert(self) -> &'static str {
        match self {
            Grenze::Kennung => "kennung",
            Grenze::Gesamt => "gesamt",
        }
    }
}

/// Ein Eintrag fiel vor Ablauf seiner 10 Minuten: wessen, an welcher Grenze (mit ihrem Wert), wie alt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verdraengt {
    pub kennung: Kennung,
    pub grenze: Grenze,
    pub hoechstens: usize,
    pub alter_ms: u64,
}

/// Grenzen außerhalb von 1 bis 640 je Kennung bzw. 1 bis 36.000 insgesamt (Regel E1.6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grenzfehler {
    pub je_kennung: usize,
    pub gesamt: usize,
}

impl fmt::Display for Grenzfehler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Grenzen des Wiederholspeichers ungültig: je Kennung {} (erlaubt 1 bis 640), gesamt {} (erlaubt 1 bis \
             36.000)",
            self.je_kennung, self.gesamt
        )
    }
}

#[derive(Debug, Clone)]
pub struct Wiederholung {
    je_kennung: usize,
    gesamt: usize,
    /// Älteste zuerst — in der Folge des Merkens.
    eintraege: VecDeque<Eintrag>,
    /// Je Kennung mit Einträgen: Anzahl und seit wann (Folge des Merkens) sie auf dieser Anzahl steht.
    anzahlen: HashMap<Kennung, Stand>,
    folge: u64,
}

#[derive(Debug, Clone, Copy)]
struct Stand {
    anzahl: usize,
    seit: u64,
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
        Wiederholung::leer(HOECHSTENS_JE_KENNUNG, HOECHSTENS_GESAMT)
    }
}

impl Wiederholung {
    /// Speicher mit eigenen Grenzen (Unit-Tests, Testknopf); nur 1 ≤ je Kennung ≤ 640 und 1 ≤ gesamt ≤ 36.000.
    pub fn neu(je_kennung: usize, gesamt: usize) -> Result<Wiederholung, Grenzfehler> {
        let erlaubt = (1..=HOECHSTENS_JE_KENNUNG).contains(&je_kennung)
            && (1..=HOECHSTENS_GESAMT).contains(&gesamt);
        if !erlaubt {
            return Err(Grenzfehler { je_kennung, gesamt });
        }
        Ok(Wiederholung::leer(je_kennung, gesamt))
    }

    fn leer(je_kennung: usize, gesamt: usize) -> Wiederholung {
        Wiederholung {
            je_kennung,
            gesamt,
            eintraege: VecDeque::new(),
            anzahlen: HashMap::new(),
            folge: 0,
        }
    }

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

    /// Merkt eine gesendete Nachricht; meldet höchstens einen Eintrag, der dafür vor Ablauf weichen musste. In
    /// dieser Reihenfolge (Regeln E1.1, E1.2): Abgelaufene fallen ohne Meldung; hat die Kennung ihre Grenze, fällt ihr
    /// ältester; sonst, ist der Speicher voll, der älteste der Kennung mit den meisten Einträgen — bei Gleichstand
    /// der Kennung, die am längsten auf dieser Anzahl steht.
    pub fn merken(
        &mut self,
        kennung: &Kennung,
        schluessel: &str,
        gespraech: &Gespraech,
        pruefsumme: u64,
        zeit: Zeit,
        jetzt: Zeit,
    ) -> Option<Verdraengt> {
        self.entfernen_wo(|eintrag| {
            &eintrag.kennung == kennung && eintrag.schluessel == schluessel
        });
        self.entfernen_wo(|eintrag| abgelaufen(eintrag, jetzt));
        let verdraengt = if self.anzahl(kennung) >= self.je_kennung {
            self.aeltesten_verdraengen(kennung, Grenze::Kennung, self.je_kennung, jetzt)
        } else if self.eintraege.len() >= self.gesamt {
            let meiste = self.kennung_mit_den_meisten()?;
            self.aeltesten_verdraengen(&meiste, Grenze::Gesamt, self.gesamt, jetzt)
        } else {
            None
        };
        self.eintraege.push_back(Eintrag {
            kennung: kennung.clone(),
            schluessel: schluessel.to_string(),
            gespraech: gespraech.clone(),
            pruefsumme,
            zeit,
            gemerkt: jetzt,
        });
        self.zaehlen(kennung, 1);
        verdraengt
    }

    fn anzahl(&self, kennung: &Kennung) -> usize {
        self.anzahlen.get(kennung).map_or(0, |stand| stand.anzahl)
    }

    /// Die Kennung mit den meisten Einträgen; bei Gleichstand die, die am längsten auf dieser Anzahl steht.
    fn kennung_mit_den_meisten(&self) -> Option<Kennung> {
        self.anzahlen
            .iter()
            .max_by(|(_, a), (_, b)| a.anzahl.cmp(&b.anzahl).then(b.seit.cmp(&a.seit)))
            .map(|(kennung, _)| kennung.clone())
    }

    fn aeltesten_verdraengen(
        &mut self,
        kennung: &Kennung,
        grenze: Grenze,
        hoechstens: usize,
        jetzt: Zeit,
    ) -> Option<Verdraengt> {
        let stelle = self
            .eintraege
            .iter()
            .position(|eintrag| &eintrag.kennung == kennung)?;
        let eintrag = self.eintraege.remove(stelle)?;
        self.zaehlen(kennung, -1);
        let alter_ms = alter_ms(&eintrag, jetzt);
        Some(Verdraengt {
            kennung: eintrag.kennung,
            grenze,
            hoechstens,
            alter_ms,
        })
    }

    fn entfernen_wo(&mut self, weg: impl Fn(&Eintrag) -> bool) {
        let mut gefallen = Vec::new();
        self.eintraege.retain(|eintrag| {
            let faellt = weg(eintrag);
            if faellt {
                gefallen.push(eintrag.kennung.clone());
            }
            !faellt
        });
        for kennung in gefallen {
            self.zaehlen(&kennung, -1);
        }
    }

    /// Ändert die Anzahl einer Kennung um eins; ohne Einträge verschwindet sie.
    fn zaehlen(&mut self, kennung: &Kennung, um: isize) {
        self.folge += 1;
        let anzahl = self.anzahl(kennung).saturating_add_signed(um);
        if anzahl == 0 {
            self.anzahlen.remove(kennung);
        } else {
            self.anzahlen.insert(
                kennung.clone(),
                Stand {
                    anzahl,
                    seit: self.folge,
                },
            );
        }
    }

    fn finden(&self, kennung: &Kennung, schluessel: &str) -> Option<&Eintrag> {
        self.eintraege
            .iter()
            .rev()
            .find(|eintrag| &eintrag.kennung == kennung && eintrag.schluessel == schluessel)
    }
}

/// Alter ab dem Merken, gesättigt: eine zurückgestellte Uhr ergibt 0 (Regel E1.5).
fn alter_ms(eintrag: &Eintrag, jetzt: Zeit) -> u64 {
    u64::try_from((jetzt - eintrag.gemerkt).num_milliseconds()).unwrap_or(0)
}

fn abgelaufen(eintrag: &Eintrag, jetzt: Zeit) -> bool {
    alter_ms(eintrag, jetzt) > HOECHSTENS_ALTER_MS as u64
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

    fn klein(je_kennung: usize, gesamt: usize) -> Wiederholung {
        Wiederholung::neu(je_kennung, gesamt).expect("Grenzen gültig")
    }

    fn merke(
        speicher: &mut Wiederholung,
        wer: &Kennung,
        schluessel: &str,
        ms: i64,
    ) -> Option<Verdraengt> {
        speicher.merken(
            wer,
            schluessel,
            &allgemein(),
            pruefsumme(schluessel),
            t(ms),
            t(ms),
        )
    }

    fn bekannt(speicher: &Wiederholung, wer: &Kennung, schluessel: &str, ms: i64) -> bool {
        matches!(
            speicher.pruefen(wer, schluessel, &allgemein(), pruefsumme(schluessel), t(ms)),
            Pruefung::Wiederholt { .. }
        )
    }

    /// R00025 „Speicher“ (Angriffsmatrix Zeilen 5, 8, 3 nach der Fortschreibung; je Kennung 3, gesamt 5): `b1`
    /// fällt an der eigenen Grenze, `b2` an der Gesamtgrenze — B hat die meisten Einträge —, `a1` bleibt.
    #[test]
    fn die_eigene_grenze_trifft_die_eigenen_die_gesamtgrenze_die_kennung_mit_den_meisten() {
        let (a, b, c) = (
            kennung("a@127.0.0.1"),
            kennung("b@127.0.0.1"),
            kennung("c@127.0.0.1"),
        );
        let mut speicher = klein(3, 5);

        assert_eq!(merke(&mut speicher, &a, "a1", 0), None);
        assert_eq!(merke(&mut speicher, &b, "b1", 10), None);
        assert_eq!(merke(&mut speicher, &b, "b2", 20), None);
        assert_eq!(merke(&mut speicher, &b, "b3", 30), None);
        let eigene = merke(&mut speicher, &b, "b4", 40);
        assert_eq!(merke(&mut speicher, &c, "c1", 50), None);
        let gesamt = merke(&mut speicher, &c, "c2", 60);

        assert_eq!(
            eigene,
            Some(Verdraengt {
                kennung: b.clone(),
                grenze: Grenze::Kennung,
                hoechstens: 3,
                alter_ms: 30
            })
        );
        assert_eq!(
            gesamt,
            Some(Verdraengt {
                kennung: b.clone(),
                grenze: Grenze::Gesamt,
                hoechstens: 5,
                alter_ms: 40
            })
        );
        assert!(
            bekannt(&speicher, &a, "a1", 70),
            "a1 bleibt (unter „ältester insgesamt“ fiele er)"
        );
        assert!(!bekannt(&speicher, &b, "b1", 70));
        assert!(!bekannt(&speicher, &b, "b2", 70));
        for (wer, schluessel) in [(&b, "b3"), (&b, "b4"), (&c, "c1"), (&c, "c2")] {
            assert!(bekannt(&speicher, wer, schluessel, 70), "{schluessel}");
        }
    }

    /// Angriffsmatrix Zeile 8: ein abgelaufener Eintrag fällt auch bei vollem Speicher, ohne Meldung.
    #[test]
    fn abgelaufen_faellt_bei_vollem_speicher_ohne_meldung() {
        let mut speicher = klein(640, 5);
        for nummer in 0..5 {
            let wer = kennung(&format!("k{nummer}@127.0.0.1"));
            assert_eq!(merke(&mut speicher, &wer, "x", nummer), None);
        }
        let neu = kennung("neu@127.0.0.1");

        let meldung = merke(&mut speicher, &neu, "y", 600_001);

        assert_eq!(meldung, None);
        assert!(!bekannt(&speicher, &kennung("k0@127.0.0.1"), "x", 600_001));
        assert!(bekannt(&speicher, &kennung("k1@127.0.0.1"), "x", 600_001));
    }

    /// Angriffsmatrix Zeile 3b (wörtlich): „L1, `H_k` = 640, `H_g` = 5, Gleichstand: fünf Kennungen je 1 Eintrag,
    /// sechste sendet → der Eintrag der Kennung fällt, die am längsten auf Anzahl 1 steht (hier die erste)“.
    #[test]
    fn bei_gleichstand_faellt_die_kennung_die_am_laengsten_auf_der_anzahl_steht() {
        let mut speicher = klein(640, 5);
        for nummer in 1..=5 {
            merke(
                &mut speicher,
                &kennung(&format!("k{nummer}@127.0.0.1")),
                "x",
                nummer,
            );
        }

        let meldung = merke(&mut speicher, &kennung("k6@127.0.0.1"), "x", 6);

        assert_eq!(meldung.map(|m| m.kennung), Some(kennung("k1@127.0.0.1")));
    }

    /// Eigene Variante zu 3b: „am längsten auf der Anzahl“ ist nicht „ältester Eintrag“ — B steht mit `b2` (älter
    /// als `a1`) erst seit dem Ablauf von `b1` auf 1, A länger; es fällt `a1`.
    #[test]
    fn gleichstand_zaehlt_ab_dem_erreichen_der_anzahl_nicht_ab_dem_aeltesten_eintrag() {
        let (a, b, c, d) = (
            kennung("a@127.0.0.1"),
            kennung("b@127.0.0.1"),
            kennung("c@127.0.0.1"),
            kennung("d@127.0.0.1"),
        );
        let mut speicher = klein(640, 3);
        merke(&mut speicher, &b, "b1", 0);
        merke(&mut speicher, &b, "b2", 1);
        merke(&mut speicher, &a, "a1", 2);
        assert_eq!(
            merke(&mut speicher, &c, "c1", 600_001),
            None,
            "b1 abgelaufen, B jetzt auf 1"
        );

        let meldung = merke(&mut speicher, &d, "d1", 600_001);

        assert_eq!(meldung.map(|m| m.kennung), Some(a.clone()));
        assert!(bekannt(&speicher, &b, "b2", 600_001));
    }

    /// Angriffsmatrix Zeile 1 (640/36.000): 128 Kennungen merken zusammen 4.920 Einträge, je Kennung höchstens 630
    /// — das Opfer wiederholt `v1`, keine Meldung.
    #[test]
    fn viele_kennungen_im_lastprofil_verdraengen_nichts() {
        let opfer = kennung("opfer@127.0.0.1");
        let mut speicher = Wiederholung::default();
        merke(&mut speicher, &opfer, "v1", 0);
        let mut meldungen = 0;
        for nummer in 0..4_920_i64 {
            let wer = kennung(&format!("k{}@127.0.0.2", nummer % 128));
            meldungen += usize::from(
                merke(&mut speicher, &wer, &format!("s{nummer}"), 1 + nummer).is_some(),
            );
        }

        assert_eq!(meldungen, 0);
        assert!(bekannt(&speicher, &opfer, "v1", 5_000));
    }

    /// Angriffsmatrix Zeilen 3 (L1, gesamt 5) und 4 (längste Kennung, IPv6): der Fluter trifft sich selbst, jede
    /// Meldung nennt ihn unverkürzt mit `grenze=gesamt`, das Opfer bleibt.
    #[test]
    fn der_fluter_trifft_an_der_gesamtgrenze_sich_selbst() {
        let lang = "a".repeat(32);
        for (opfer, fluter) in [
            (
                "opfer@127.0.0.1".to_string(),
                "fluter@127.0.0.2".to_string(),
            ),
            (
                format!("{lang}@2001:db8:85a3:0:0:8a2e:370:7334"),
                format!("{}@2001:0db8:85a3:0000:0000:8a2e:0370:7335", "b".repeat(32)),
            ),
        ] {
            let (opfer, fluter) = (kennung(&opfer), kennung(&fluter));
            let mut speicher = klein(640, 5);
            merke(&mut speicher, &opfer, "v1", 0);
            let meldungen: Vec<Verdraengt> = (1..=100)
                .filter_map(|nummer| merke(&mut speicher, &fluter, &format!("f{nummer}"), nummer))
                .collect();

            assert_eq!(meldungen.len(), 96, "ab dem 5. Eintrag des Fluters je eine");
            assert!(
                meldungen.iter().all(|m| m.kennung == fluter
                    && m.grenze == Grenze::Gesamt
                    && m.hoechstens == 5)
            );
            assert!(bekannt(&speicher, &opfer, "v1", 101));
        }
    }

    /// Angriffsmatrix Zeile 6 (640/36.000): A1 merkt 10.000 eigene, hält höchstens 640, A2 behält `v1`; jede
    /// Meldung `kennung=A1`, `grenze=kennung`.
    #[test]
    fn eine_kennung_ueber_ihr_konto_hinaus_haelt_hoechstens_640() {
        let (a1, a2) = (kennung("a1@127.0.0.1"), kennung("a2@127.0.0.1"));
        let mut speicher = Wiederholung::default();
        merke(&mut speicher, &a2, "v1", 0);
        let meldungen: Vec<Verdraengt> = (0..10_000)
            .filter_map(|nummer| merke(&mut speicher, &a1, &format!("s{nummer}"), 1 + nummer))
            .collect();

        assert_eq!(meldungen.len(), 10_000 - 640);
        assert!(
            meldungen
                .iter()
                .all(|m| m.kennung == a1 && m.grenze == Grenze::Kennung && m.hoechstens == 640)
        );
        assert!(bekannt(&speicher, &a1, "s9360", 20_000), "der 640-jüngste");
        assert!(!bekannt(&speicher, &a1, "s9359", 20_000), "der 641-jüngste");
        assert!(bekannt(&speicher, &a2, "v1", 20_000));
    }

    /// Angriffsmatrix Zeilen 9 und 14: die Uhr springt zurück — kein Abbruch, Alter gesättigt 0, beide Grenzen
    /// halten; je Merken höchstens eine Meldung (der Rückgabetyp).
    #[test]
    fn eine_zurueckgestellte_uhr_bricht_nichts_und_das_alter_ist_nie_negativ() {
        let (a, b) = (kennung("a@127.0.0.1"), kennung("b@127.0.0.1"));
        let mut speicher = klein(2, 3);
        merke(&mut speicher, &a, "a1", 10_000);
        merke(&mut speicher, &a, "a2", 10_000);

        let eigene = merke(&mut speicher, &a, "a3", 0);
        merke(&mut speicher, &b, "b1", 0);
        let gesamt = merke(&mut speicher, &b, "b2", 0);

        assert_eq!(
            eigene.map(|m| (m.grenze, m.alter_ms)),
            Some((Grenze::Kennung, 0))
        );
        assert_eq!(
            gesamt.map(|m| (m.kennung, m.grenze, m.alter_ms)),
            Some((a.clone(), Grenze::Gesamt, 0))
        );
        assert!(bekannt(&speicher, &a, "a3", 0));
        assert!(bekannt(&speicher, &b, "b1", 0));
        assert!(bekannt(&speicher, &b, "b2", 0));
    }

    /// Angriffsmatrix Zeile 22, Regel E1.6: Grenzen nur 1 ≤ gesamt ≤ 36.000 und 1 ≤ je Kennung ≤ 640.
    #[test]
    fn grenzen_ausserhalb_des_bereichs_sind_ein_fehler() {
        for (je_kennung, gesamt) in [(640, 0), (640, 36_001), (0, 5), (641, 5)] {
            assert!(
                Wiederholung::neu(je_kennung, gesamt).is_err(),
                "{je_kennung}/{gesamt}"
            );
        }
        assert!(Wiederholung::neu(1, 1).is_ok());
        assert!(Wiederholung::neu(640, 36_000).is_ok());
        assert_eq!((HOECHSTENS_JE_KENNUNG, HOECHSTENS_GESAMT), (640, 36_000));
    }

    /// Angriffsmatrix Zeile 7 (Durchsetzung ADR-0034 Ergänzung Karte #36): der Bedarf einer Kennung aus dem
    /// Schreibkonto (Vorrat + Nachschub × 600 s = 630) passt unter die Grenze je Kennung; die Gesamtgrenze trägt die
    /// Spitze des Lastprofils (30/s × 600 s, `AI-Team/umgebung.md`).
    #[test]
    fn die_grenzen_halten_den_bedarf_des_schreibkontos_und_des_lastprofils() {
        use crate::schreibkonto::eimer::JE_KENNUNG;
        let bedarf = JE_KENNUNG.vorrat + JE_KENNUNG.nachschub_mikro_je_ms * 600_000 / 1_000_000;

        assert_eq!(bedarf, 630);
        assert!(bedarf <= HOECHSTENS_JE_KENNUNG as u64);
        const { assert!(HOECHSTENS_GESAMT >= 30 * 600) };
    }

    /// Kontrollpaar K2: gemerkt bei t, eine andere Sendung bei t + 600.001 ms → keine Meldung, `v1` ist neu.
    #[test]
    fn nach_ablauf_ist_der_schluessel_neu_ohne_meldung() {
        let (opfer, andere) = (kennung("opfer@127.0.0.1"), kennung("x@127.0.0.1"));
        let mut speicher = Wiederholung::default();
        merke(&mut speicher, &opfer, "v1", 0);

        let meldung = merke(&mut speicher, &andere, "x", 600_001);

        assert_eq!(meldung, None);
        assert!(!bekannt(&speicher, &opfer, "v1", 600_001));
    }
}
