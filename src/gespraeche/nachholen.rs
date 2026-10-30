//! Nachholen über alle Gespräche (ADR-0054 Regel 3.4 und 4, B0737): die Nachrichten mit `seit < zeit ≤ marke` aus
//! allen Gesprächen, die der Filter berühren kann, in Zeitfolge zusammengeführt, mit der Trefferregel gefiltert, in
//! der aktuellen Fassung — begrenzt durch ein Budget an Treffern, Bytes und geprüften Sätzen.
//!
//! Gelesen wird je Gespräch blockweise über den Verlaufsindex (ADR-0025 `nach`), die Sperre eines Gesprächs nur zum
//! Herauskopieren eines Blocks; alles mit einer Zeit über der Marke kommt über den Bus, nicht von hier.

use super::abfrage::{Abfrage, Abschnitt, BLOCK_HOECHSTENS, Blockgroesse};
use super::treffer::{Filter, Treffer, treffer};
use super::verlaufsindex::Verlauf;
use super::{Gespraech, Gespraeche, Kennung, Nachricht, lesend, nachricht_in_fassung, sperren};
use crate::uhr::Zeit;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// Höchstens so viele Satzbytes je gelesenem Block — wie ein Verlaufsblock (ADR-0039 Regel 1).
const BLOCKBYTES: usize = 262_144;

/// Wie viel ein Aufruf liefern und prüfen darf (Regel 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Nachholbudget {
    /// Höchstens so viele Treffer je Antwort.
    pub nachrichten: usize,
    /// Höchstens so viele Lieferbytes je Antwort (`lieferbytes`); der erste Treffer immer.
    pub bytes: usize,
    /// Höchstens so viele geprüfte Sätze je Nachholen.
    pub saetze: usize,
}

/// Das Budget eines Aufrufs: 100 Nachrichten, 256 KiB (ADR-0039), 2.000 geprüfte Sätze — K1 (ADR-0054 Ergänzung
/// „Nachholen von `warten` …“ Regel 2, R00049 F0170): hält jede Belegung der Nachholbank kurz (≈ 10 ms). Bleibt
/// nach genau so vielen Sätzen keiner ungeprüft, endet das Nachholen ohne `weitere`.
pub const NACHHOLBUDGET: Nachholbudget = Nachholbudget {
    nachrichten: 100,
    bytes: 262_144,
    saetze: 2_000,
};

/// Was das Nachholen ergab: Treffer in Zeitfolge, der verarbeitete Stand und ob ein Budget vor der Marke endete.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nachgeholt {
    pub treffer: Vec<Treffer>,
    /// Bis hierher ist alles geprüft: bei `weitere` die zuletzt geprüfte Zeit, sonst `max(seit, marke)`.
    pub stand: Zeit,
    pub weitere: bool,
}

/// Wie viele Bytes ein Treffer im Budget zählt: Text, Namen der Anhänge und Textanfang des Zitats — der Inhalt,
/// der mit der Nachricht wächst (ADR-0039 zählt Satzbytes; die feste Form je Nachricht zählt nicht).
pub fn lieferbytes(nachricht: &Nachricht) -> usize {
    let anhaenge: usize = nachricht
        .anhaenge
        .iter()
        .map(|anhang| anhang.name.len())
        .sum();
    let zitat = nachricht
        .antwort_auf
        .as_ref()
        .map_or(0, |zitat| zitat.textanfang.len());
    nachricht.text.len() + anhaenge + zitat
}

impl Gespraeche {
    /// Holt nach, was `wartender` unter `filter` mit `seit < zeit ≤ marke` trifft.
    pub fn nachholen(
        &self,
        seit: Zeit,
        marke: Zeit,
        wartender: &Kennung,
        filter: &Filter,
        budget: Nachholbudget,
    ) -> Nachgeholt {
        let mut straenge = self.straenge(seit, marke, filter);
        let mut ergebnis = Nachgeholt {
            treffer: Vec::new(),
            stand: seit,
            weitere: false,
        };
        let mut bytes = 0;
        let mut geprueft = 0;
        while let Some(naechster) = naechster_strang(&mut straenge) {
            let budget_erschoepft =
                geprueft == budget.saetze || ergebnis.treffer.len() == budget.nachrichten;
            let Some(nachricht) = straenge[naechster].puffer.front() else {
                break;
            };
            let anlass = treffer(&straenge[naechster].gespraech, nachricht, wartender, filter);
            let zu_gross = anlass.is_some()
                && !ergebnis.treffer.is_empty()
                && bytes + lieferbytes(nachricht) > budget.bytes;
            if budget_erschoepft || zu_gross {
                ergebnis.weitere = true;
                return ergebnis;
            }
            let strang = &mut straenge[naechster];
            let Some(nachricht) = strang.puffer.pop_front() else {
                break;
            };
            geprueft += 1;
            ergebnis.stand = nachricht.zeit;
            if let Some(anlass) = anlass {
                bytes += lieferbytes(&nachricht);
                ergebnis.treffer.push(Treffer {
                    gespraech: strang.gespraech.clone(),
                    anlass,
                    nachricht,
                });
            }
        }
        ergebnis.stand = seit.max(marke);
        ergebnis
    }

    /// Je Gespräch, das der Filter berühren kann, ein Strang ab `seit` bis `marke` (Regel 3.4): genannte Channels,
    /// ohne Angabe und mit `erwaehnung` alle Channels; Direktgespräche erst mit I0012.
    fn straenge(&self, seit: Zeit, marke: Zeit, filter: &Filter) -> Vec<Strang<'_>> {
        let alle = filter.ohne_angabe() || filter.erwaehnung;
        lesend(&self.verzeichnis)
            .verlaeufe
            .iter()
            .filter(|(gespraech, _)| match gespraech {
                Gespraech::Channel(name) => alle || filter.channels.contains(name),
                Gespraech::Direkt(_) => false,
            })
            .map(|(gespraech, verlauf)| Strang {
                gespraeche: self,
                gespraech: gespraech.clone(),
                verlauf: Arc::clone(verlauf),
                puffer: VecDeque::new(),
                ab: seit,
                marke,
                erschoepft: false,
            })
            .collect()
    }
}

/// Der Strang mit der ältesten nächsten Nachricht, nachdem jeder leere Strang nachgeladen hat; keiner, wenn alle
/// erschöpft sind. Zeiten sind über alle Gespräche verschieden (ADR-0035).
fn naechster_strang(straenge: &mut [Strang<'_>]) -> Option<usize> {
    for strang in straenge.iter_mut() {
        strang.nachladen();
    }
    straenge
        .iter()
        .enumerate()
        .filter_map(|(index, strang)| strang.puffer.front().map(|kopf| (index, kopf.zeit)))
        .min_by_key(|(_, zeit)| *zeit)
        .map(|(index, _)| index)
}

/// Die Nachrichten eines Gesprächs mit `ab < zeit ≤ marke`, blockweise aus dem Index gelesen.
struct Strang<'a> {
    gespraeche: &'a Gespraeche,
    gespraech: Gespraech,
    verlauf: Arc<Mutex<Verlauf>>,
    puffer: VecDeque<Nachricht>,
    /// Zeit der zuletzt gelesenen Nachricht (exklusiver Cursor des nächsten Blocks).
    ab: Zeit,
    marke: Zeit,
    erschoepft: bool,
}

impl Strang<'_> {
    /// Liest den nächsten Block, wenn der Puffer leer ist und noch etwas bis zur Marke kommen kann.
    fn nachladen(&mut self) {
        if !self.puffer.is_empty() || self.erschoepft {
            return;
        }
        let abfrage = Abfrage {
            abschnitt: Abschnitt::Nach(self.ab),
            n: Blockgroesse::bilden(BLOCK_HOECHSTENS).unwrap_or_else(Blockgroesse::standard),
        };
        let block = sperren(&self.verlauf).block(&abfrage, BLOCKBYTES);
        let nachrichten: Vec<Nachricht> = block
            .saetze
            .iter()
            .filter_map(nachricht_in_fassung)
            .map(|nachricht| self.gespraeche.zustand_eintragen(nachricht))
            .collect();
        let Some(letzte) = nachrichten.last().map(|nachricht| nachricht.zeit) else {
            self.erschoepft = true;
            return;
        };
        self.ab = letzte;
        self.erschoepft = !block.neuer || letzte >= self.marke;
        self.puffer.extend(
            nachrichten
                .into_iter()
                .filter(|nachricht| nachricht.zeit <= self.marke),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Art, Speicherablage};
    use super::*;
    use crate::uhr::GestellteUhr;
    use std::sync::Arc;

    fn zeit(text: &str) -> Zeit {
        Zeit::parse_from_rfc3339(text).expect("Testzeit")
    }

    fn kennung(text: &str) -> Kennung {
        Kennung::bilden(text).expect("Kennung")
    }

    fn anna() -> Kennung {
        kennung("anna@127.0.0.1")
    }

    fn claude() -> Kennung {
        kennung("claude@127.0.0.1")
    }

    /// Drei Channels `a`, `b`, `c`; `anna` sendet `t1…t9` reihum (`t1` in `a`, `t2` in `b`, …), je 1 s später, Text
    /// `Nachricht n`. Danach löscht sie `t5` und bearbeitet `t6` zu `neu`. Liefert die Gespräche und `t0…t9`
    /// (`t0` liegt vor `t1`).
    fn verschraenkt() -> (Gespraeche, Vec<Zeit>) {
        let ablage = Arc::new(Speicherablage::neu());
        for channel in ["a", "b", "c"] {
            ablage.gespraech_anlegen(&Gespraech::channel(channel));
        }
        let uhr = Arc::new(GestellteUhr::neu(zeit("2026-10-30T12:00:00.000+01:00")));
        let gespraeche = Gespraeche::laden(ablage, uhr.clone()).expect("geladen");
        let mut zeiten = vec![zeit("2026-10-30T11:59:59.000+01:00")];
        for n in 1..=9 {
            uhr.stellen(zeit("2026-10-30T12:00:00.000+01:00") + chrono::Duration::seconds(n));
            let channel = ["a", "b", "c"][usize::try_from(n - 1).expect("n") % 3];
            let gesendet = gespraeche
                .senden(
                    &Gespraech::channel(channel),
                    &anna(),
                    Art::Mensch,
                    &format!("Nachricht {n}"),
                )
                .expect("gesendet");
            zeiten.push(gesendet.zeit);
        }
        gespraeche
            .nachricht_loeschen(&Gespraech::channel("b"), &anna(), zeiten[5])
            .expect("gelöscht");
        gespraeche
            .bearbeiten(&Gespraech::channel("c"), &anna(), zeiten[6], "neu")
            .expect("bearbeitet");
        (gespraeche, zeiten)
    }

    fn zeiten_der(nachgeholt: &Nachgeholt) -> Vec<Zeit> {
        nachgeholt
            .treffer
            .iter()
            .map(|treffer| treffer.nachricht.zeit)
            .collect()
    }

    /// W4 (B0737): `seit = t3`, `H = t8` → genau `t4, t6, t7, t8` in Zeitfolge über drei Channels; `t5` ist
    /// gelöscht und fehlt, `t6` kommt in der aktuellen Fassung, `t9` liegt über der Marke; Stand `t8`.
    #[test]
    fn nachgeholt_wird_ueber_alle_channels_in_zeitfolge_bis_zur_marke() {
        let (gespraeche, t) = verschraenkt();

        let nachgeholt =
            gespraeche.nachholen(t[3], t[8], &claude(), &Filter::default(), NACHHOLBUDGET);

        assert_eq!(zeiten_der(&nachgeholt), vec![t[4], t[6], t[7], t[8]]);
        let gespraeche_der: Vec<Gespraech> = nachgeholt
            .treffer
            .iter()
            .map(|treffer| treffer.gespraech.clone())
            .collect();
        assert_eq!(
            gespraeche_der,
            ["a", "c", "a", "b"].map(Gespraech::channel).to_vec()
        );
        let t6 = &nachgeholt.treffer[1];
        assert_eq!(t6.nachricht.text, "neu");
        assert!(t6.nachricht.geaendert.is_some(), "t6 ist bearbeitet");
        assert!(
            nachgeholt
                .treffer
                .iter()
                .all(|treffer| treffer.anlass.is_empty())
        );
        assert_eq!((nachgeholt.stand, nachgeholt.weitere), (t[8], false));
    }

    /// W4: gefiltert nach `channel=a` → nur `t4` und `t7` aus `(t3, t8]`; der Stand ist trotzdem die Marke.
    #[test]
    fn nachgeholt_wird_nur_aus_den_genannten_channels() {
        let (gespraeche, t) = verschraenkt();
        let filter = Filter {
            channels: ["a".to_string()].into(),
            ..Filter::default()
        };

        let nachgeholt = gespraeche.nachholen(t[3], t[8], &claude(), &filter, NACHHOLBUDGET);

        assert_eq!(zeiten_der(&nachgeholt), vec![t[4], t[7]]);
        assert_eq!((nachgeholt.stand, nachgeholt.weitere), (t[8], false));
    }

    /// W4: liegt `seit` hinter der Marke (Zeit in der Zukunft), ist nichts nachzuholen und der Stand ist `seit`.
    #[test]
    fn ein_seit_hinter_der_marke_holt_nichts_nach() {
        let (gespraeche, t) = verschraenkt();

        let nachgeholt =
            gespraeche.nachholen(t[9], t[8], &claude(), &Filter::default(), NACHHOLBUDGET);

        assert!(nachgeholt.treffer.is_empty());
        assert_eq!((nachgeholt.stand, nachgeholt.weitere), (t[9], false));
    }

    /// W4 Budget Nachrichten: höchstens 2 Treffer → `t1, t2`, `weitere`, Stand `t2`; genau so viele Treffer wie
    /// übrig → kein `weitere`.
    #[test]
    fn das_budget_an_nachrichten_endet_mit_weitere_an_der_letzten_gelieferten() {
        let (gespraeche, t) = verschraenkt();
        let zwei = Nachholbudget {
            nachrichten: 2,
            ..NACHHOLBUDGET
        };

        let erster = gespraeche.nachholen(t[0], t[9], &claude(), &Filter::default(), zwei);
        let letzter = gespraeche.nachholen(t[7], t[9], &claude(), &Filter::default(), zwei);

        assert_eq!(zeiten_der(&erster), vec![t[1], t[2]]);
        assert_eq!((erster.stand, erster.weitere), (t[2], true));
        assert_eq!(zeiten_der(&letzter), vec![t[8], t[9]]);
        assert_eq!((letzter.stand, letzter.weitere), (t[9], false));
    }

    /// W4 Budget Sätze: wartet `anna` selbst (alles eigene, kein Treffer), endet das Nachholen nach 4 geprüften
    /// Sätzen mit `weitere` und Stand `t4`; die gelöschte `t5` zählt als geprüft.
    #[test]
    fn das_budget_an_saetzen_endet_mit_weitere_an_der_letzten_gepruefen() {
        let (gespraeche, t) = verschraenkt();
        let vier = Nachholbudget {
            saetze: 4,
            ..NACHHOLBUDGET
        };

        let erster = gespraeche.nachholen(t[0], t[9], &anna(), &Filter::default(), vier);
        let zweiter = gespraeche.nachholen(erster.stand, t[9], &anna(), &Filter::default(), vier);
        let dritter = gespraeche.nachholen(zweiter.stand, t[9], &anna(), &Filter::default(), vier);

        assert!(erster.treffer.is_empty() && zweiter.treffer.is_empty());
        assert_eq!((erster.stand, erster.weitere), (t[4], true));
        assert_eq!((zweiter.stand, zweiter.weitere), (t[8], true));
        assert_eq!((dritter.stand, dritter.weitere), (t[9], false));
    }

    /// W4 Budget Bytes: der erste Treffer kommt immer, der nächste nur, solange die Summe das Budget hält.
    #[test]
    fn das_budget_an_bytes_liefert_mindestens_einen() {
        let (gespraeche, t) = verschraenkt();
        let knapp = Nachholbudget {
            bytes: lieferbytes(&nachricht_mit("Nachricht 1")) + 5,
            ..NACHHOLBUDGET
        };

        let nachgeholt = gespraeche.nachholen(t[0], t[9], &claude(), &Filter::default(), knapp);

        assert_eq!(zeiten_der(&nachgeholt), vec![t[1]]);
        assert_eq!((nachgeholt.stand, nachgeholt.weitere), (t[1], true));
    }

    /// Ein Channel `a` mit `anzahl` Nachrichten `Nachricht n`, je 1 ms später; `von(n)` sendet die n-te (ab 1
    /// gezählt). Liefert die Gespräche und `t0…tn` (`t0` liegt vor `t1`).
    fn reihe(anzahl: usize, von: impl Fn(usize) -> Kennung) -> (Gespraeche, Vec<Zeit>) {
        let ablage = Arc::new(Speicherablage::neu());
        ablage.gespraech_anlegen(&Gespraech::channel("a"));
        let beginn = zeit("2026-10-30T12:00:00.000+01:00");
        let uhr = Arc::new(GestellteUhr::neu(beginn));
        let gespraeche = Gespraeche::laden(ablage, uhr).expect("geladen");
        let mut zeiten = vec![beginn - chrono::Duration::seconds(1)];
        for nummer in 1..=anzahl {
            let gesendet = gespraeche
                .senden(
                    &Gespraech::channel("a"),
                    &von(nummer),
                    Art::Mensch,
                    &format!("Nachricht {nummer}"),
                )
                .expect("gesendet");
            zeiten.push(gesendet.zeit);
        }
        (gespraeche, zeiten)
    }

    /// Ruft wie ein Agent in Schleife mit dem jeweils zurückgegebenen Stand, bis kein `weitere` mehr kommt.
    fn in_schleife(gespraeche: &Gespraeche, t: &[Zeit], filter: &Filter) -> Vec<Nachgeholt> {
        let marke = *t.last().expect("Marke");
        let mut aufrufe = Vec::new();
        let mut seit = t[0];
        loop {
            let nachgeholt = gespraeche.nachholen(seit, marke, &claude(), filter, NACHHOLBUDGET);
            seit = nachgeholt.stand;
            let weitere = nachgeholt.weitere;
            aufrufe.push(nachgeholt);
            if !weitere {
                return aufrufe;
            }
        }
    }

    fn erwaehnung() -> Filter {
        Filter {
            erwaehnung: true,
            ..Filter::default()
        }
    }

    /// N3 (B0757, K1): das Budget sind 2.000 geprüfte Sätze je Aufruf.
    #[test]
    fn das_budget_sind_zweitausend_saetze() {
        assert_eq!(NACHHOLBUDGET.saetze, 2_000);
    }

    /// N3 (B0757, K1): 5.000 Sätze ohne Treffer → `weitere` am 2.000., `weitere` am 4.000., dann `frist` (kein
    /// `weitere`) mit dem Stand der Marke; jeder Stand ist genau der zuletzt geprüfte Satz (lückenlos, ohne Doppel).
    #[test]
    fn fuenftausend_saetze_ohne_treffer_sind_drei_aufrufe() {
        let (gespraeche, t) = reihe(5_000, |_| anna());

        let aufrufe = in_schleife(&gespraeche, &t, &erwaehnung());

        let staende: Vec<(Zeit, bool)> = aufrufe
            .iter()
            .map(|aufruf| (aufruf.stand, aufruf.weitere))
            .collect();
        assert_eq!(
            staende,
            vec![(t[2_000], true), (t[4_000], true), (t[5_000], false)]
        );
        assert!(aufrufe.iter().all(|aufruf| aufruf.treffer.is_empty()));
    }

    /// N3 Grenzfall (Auslegung des planer, beim Bau bestätigt): genau 4.000 Sätze → der zweite Aufruf prüft die
    /// letzten 2.000 und endet ohne `weitere`, denn kein Satz bleibt ungeprüft.
    #[test]
    fn genau_viertausend_saetze_sind_zwei_aufrufe() {
        let (gespraeche, t) = reihe(4_000, |_| anna());

        let aufrufe = in_schleife(&gespraeche, &t, &erwaehnung());

        let staende: Vec<(Zeit, bool)> = aufrufe
            .iter()
            .map(|aufruf| (aufruf.stand, aufruf.weitere))
            .collect();
        assert_eq!(staende, vec![(t[2_000], true), (t[4_000], false)]);
    }

    /// N3 lückenlos und ohne Doppel an der Budgetgrenze: 4.500 Sätze, `claude` selbst sendet alle außer dem 2.000.,
    /// 2.001. und 4.001. (die sendet `anna`, sie treffen ohne Filter) → `[t2000]` mit `weitere` am 2.000.,
    /// `[t2001]` mit `weitere` am 4.000., `[t4001]` ohne `weitere`.
    #[test]
    fn treffer_an_der_budgetgrenze_kommen_genau_einmal() {
        let von_anna = [2_000, 2_001, 4_001];
        let (gespraeche, t) = reihe(4_500, |nummer| {
            if von_anna.contains(&nummer) {
                anna()
            } else {
                claude()
            }
        });

        let aufrufe = in_schleife(&gespraeche, &t, &Filter::default());

        let je_aufruf: Vec<(Vec<Zeit>, Zeit, bool)> = aufrufe
            .iter()
            .map(|aufruf| (zeiten_der(aufruf), aufruf.stand, aufruf.weitere))
            .collect();
        assert_eq!(
            je_aufruf,
            vec![
                (vec![t[2_000]], t[2_000], true),
                (vec![t[2_001]], t[4_000], true),
                (vec![t[4_001]], t[4_500], false),
            ]
        );
    }

    /// Lieferbytes: der Text zählt mit seiner Länge in Bytes.
    #[test]
    fn lieferbytes_wachsen_mit_dem_text() {
        assert_eq!(
            lieferbytes(&nachricht_mit("ü".repeat(10).as_str())) - lieferbytes(&nachricht_mit("")),
            20
        );
    }

    fn nachricht_mit(text: &str) -> Nachricht {
        Nachricht {
            zeit: zeit("2026-10-30T12:00:00.000+01:00"),
            von: anna(),
            art: None,
            text: text.to_string(),
            anhaenge: Vec::new(),
            geaendert: None,
            bezug: None,
            antwort_auf: None,
        }
    }
}
