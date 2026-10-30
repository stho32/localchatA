//! Nachholen über alle Gespräche (ADR-0054 Regel 3.4 und 4, B0737): die Nachrichten mit `seit < zeit ≤ marke` aus
//! allen Gesprächen, die der Filter berühren kann, in Zeitfolge zusammengeführt, mit der Trefferregel gefiltert, in
//! der aktuellen Fassung — begrenzt durch ein Budget an Treffern, Bytes und geprüften Sätzen.
//!
//! Gelesen wird nur, was gebucht ist (ADR-0054 Ergänzung „Kosten-Einheit des Nachholens“): jedes Gespräch kennt von
//! seiner nächsten Nachricht nur Zeit und Lesebytes (`Verlauf::naechste_nach`, ohne Kopie); das Budget wird an diesen
//! Lesebytes geprüft, und erst die Nachricht, die an der Reihe ist, wird kopiert — eine je Schritt, die Sperre eines
//! Gesprächs nur dafür. Alles mit einer Zeit über der Marke kommt über den Bus, nicht von hier.

use super::abfrage::{Abfrage, Abschnitt, Blockgroesse};
use super::treffer::{Filter, Treffer, treffer};
use super::verlaufsindex::Verlauf;
use super::{
    Gespraech, Gespraeche, Kennung, Nachricht, lesend, nachricht_in_fassung, sichtbar_fuer, sperren,
};
use crate::uhr::Zeit;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::sync::{Arc, Mutex};

/// Wie viel ein Aufruf liefern und prüfen darf (Regel 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Nachholbudget {
    /// Höchstens so viele Treffer je Antwort.
    pub nachrichten: usize,
    /// Höchstens so viele Lieferbytes je Antwort (`lieferbytes`); der erste Treffer immer.
    pub bytes: usize,
    /// Höchstens so viele Kosten (`kosten_eines_satzes`) je Nachholen; der erste Satz immer.
    pub kosten: usize,
}

/// Das Budget eines Aufrufs: 100 Nachrichten, 256 KiB (ADR-0039), 2.000 Kosten — K1 (ADR-0054 Ergänzung „Nachholen
/// von `warten` …“ Regel 2, R00049 F0170; seit F0171 in Kosten statt in Sätzen, ADR-0054 Ergänzung „Kosten-Einheit
/// des Nachholens“ Regel 2): hält jede Belegung der Nachholbank kurz (≈ 10 ms). Ein Satz, der das Rest-Budget
/// überschreitet, wird nicht mehr geprüft (`weitere`); bleibt nach genau 2.000 Kosten keiner ungeprüft, endet das
/// Nachholen ohne `weitere`.
pub const NACHHOLBUDGET: Nachholbudget = Nachholbudget {
    nachrichten: 100,
    bytes: 262_144,
    kosten: 2_000,
};

/// Wie viele Satzbytes eine Kosteneinheit trägt — K6 (ADR-0054 Ergänzung „Kosten-Einheit des Nachholens“ Regel 1,
/// R00049 F0171): ein Satz kostet `1 + ⌊Satzbytes / 1.024⌋`; die Einheit steht für ≈ 5 µs Rechenzeit, gleich ob der
/// Satz kurz ist oder 16 KiB lang.
const BYTE_JE_KOSTENEINHEIT: usize = 1_024;

/// Die Kosten eines Satzes der Länge `satzbytes` (Länge in der Tagesdatei; bei einer bearbeiteten Nachricht das
/// Größere aus Satz und Änderungssatz, `Verlauf::naechste_nach`) — Sätze unter 1 KiB kosten 1.
pub fn kosten_eines_satzes(satzbytes: usize) -> usize {
    1 + satzbytes / BYTE_JE_KOSTENEINHEIT
}

/// Was das Nachholen ergab: Treffer in Zeitfolge, der verarbeitete Stand und ob ein Budget vor der Marke endete.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nachgeholt {
    pub treffer: Vec<Treffer>,
    /// Bis hierher ist alles geprüft: bei `weitere` die zuletzt geprüfte Zeit, sonst `max(seit, Hs)` mit der
    /// sichtbaren Marke `Hs` (`sichtbare_marke`) — nie die globale Marke, sie verriete fremde Direktnachrichten.
    pub stand: Zeit,
    pub weitere: bool,
    /// So viele Kosten hat das Nachholen geprüft — die Abrechnung am Nachholkonto (ADR-0054 Ergänzung Regel 3).
    pub kosten: usize,
    /// So viele Sätze hat das Nachholen geprüft — der Zähler `nachholen.saetze`.
    pub saetze: usize,
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
            kosten: 0,
            saetze: 0,
        };
        let mut bytes = 0;
        let mut reihe: BinaryHeap<Reverse<(Zeit, usize)>> = straenge
            .iter()
            .enumerate()
            .filter_map(|(index, strang)| strang.kopf.map(|(zeit, _)| Reverse((zeit, index))))
            .collect();
        while let Some(Reverse((_, naechster))) = reihe.pop() {
            let strang = &mut straenge[naechster];
            let Some((_, lesebytes)) = strang.kopf else {
                continue;
            };
            let kosten = kosten_eines_satzes(lesebytes);
            let budget_erschoepft = ergebnis.treffer.len() == budget.nachrichten
                || (ergebnis.saetze > 0 && ergebnis.kosten + kosten > budget.kosten);
            if budget_erschoepft {
                ergebnis.weitere = true;
                return ergebnis;
            }
            let entnommen = strang.entnehmen();
            if let Some((zeit, _)) = strang.kopf {
                reihe.push(Reverse((zeit, naechster)));
            }
            let Some(nachricht) = entnommen else {
                continue;
            };
            let anlass = treffer(&strang.gespraech, &nachricht, wartender, filter);
            let zu_gross = anlass.is_some()
                && !ergebnis.treffer.is_empty()
                && bytes + lieferbytes(&nachricht) > budget.bytes;
            if zu_gross {
                ergebnis.weitere = true;
                return ergebnis;
            }
            ergebnis.kosten += kosten;
            ergebnis.saetze += 1;
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
        ergebnis.stand = self
            .sichtbare_marke(wartender, marke)
            .map_or(seit, |sichtbar| seit.max(sichtbar));
        ergebnis
    }

    /// Die sichtbare Marke `Hs` (ADR-0055 Regel 8, Matrix G1, G2): die größte Nachrichtenzeit `≤ marke` über alle
    /// Gespräche, die `wartender` sehen darf (`sichtbar_fuer`: die Channels und seine eigenen Direktgespräche); keine,
    /// wenn er nichts Sichtbares hat. Zwischen `Hs` und `marke` liegt keine Nachricht, die er sehen darf — ein
    /// Nachholen ab `Hs` verliert also nichts, und `Hs` verrät keinen Zeitpunkt einer fremden Direktnachricht.
    pub fn sichtbare_marke(&self, wartender: &Kennung, marke: Zeit) -> Option<Zeit> {
        lesend(&self.verzeichnis)
            .verlaeufe
            .iter()
            .filter(|(gespraech, _)| sichtbar_fuer(gespraech, wartender, false))
            .filter_map(|(_, verlauf)| sperren(verlauf).letzte_nachrichtenzeit_bis(&marke))
            .max()
    }

    /// Je Gespräch, das der Filter berühren kann, ein Strang ab `seit` bis `marke` (Regel 3.4): genannte Channels,
    /// ohne Angabe und mit `erwaehnung` alle Channels; Direktgespräche erst mit I0012.
    fn straenge(&self, seit: Zeit, marke: Zeit, filter: &Filter) -> Vec<Strang<'_>> {
        let alle = filter.ohne_angabe() || filter.erwaehnung;
        // Die Verzeichnissperre gilt nur für die Auswahl; `vorschauen` nimmt die Sperre des Gesprächs, und die beiden
        // werden nicht ineinander gehalten.
        let verlaeufe: Vec<(Gespraech, Arc<Mutex<Verlauf>>)> = lesend(&self.verzeichnis)
            .verlaeufe
            .iter()
            .filter(|(gespraech, _)| match gespraech {
                Gespraech::Channel(name) => alle || filter.channels.contains(name),
                Gespraech::Direkt(_) => false,
            })
            .map(|(gespraech, verlauf)| (gespraech.clone(), Arc::clone(verlauf)))
            .collect();
        verlaeufe
            .into_iter()
            .map(|(gespraech, verlauf)| {
                let mut strang = Strang {
                    gespraeche: self,
                    gespraech,
                    verlauf,
                    kopf: None,
                    ab: seit,
                    marke,
                };
                strang.vorschauen();
                strang
            })
            .collect()
    }
}

/// Die Nachrichten eines Gesprächs mit `ab < zeit ≤ marke`. Gelesen wird eine Nachricht erst, wenn sie an der Reihe
/// ist; bis dahin kennt der Strang nur Zeit und Lesebytes seiner nächsten (`kopf`) — das Größere aus Satz und
/// Änderungssatz, was die Trefferregel auswertet und `kosten_eines_satzes` bucht. Zeiten sind über alle Gespräche
/// verschieden (ADR-0035), die Reihenfolge der Stränge also eindeutig.
struct Strang<'a> {
    gespraeche: &'a Gespraeche,
    gespraech: Gespraech,
    verlauf: Arc<Mutex<Verlauf>>,
    /// Zeit und Lesebytes der nächsten Nachricht bis zur Marke; keine, wenn der Strang erschöpft ist.
    kopf: Option<(Zeit, usize)>,
    /// Zeit der zuletzt entnommenen Nachricht (exklusiver Cursor).
    ab: Zeit,
    marke: Zeit,
}

impl Strang<'_> {
    /// Merkt Zeit und Lesebytes der nächsten Nachricht bis zur Marke, ohne sie zu kopieren.
    fn vorschauen(&mut self) {
        self.kopf = sperren(&self.verlauf)
            .naechste_nach(&self.ab)
            .filter(|(zeit, _)| *zeit <= self.marke);
    }

    /// Kopiert die Nachricht des Kopfs — genau eine — in der aktuellen Fassung und schaut die folgende an. Der
    /// Cursor rückt auch vor, wenn der Satz sich nicht lesen lässt. Zwischen `vorschauen` und hier ist die Sperre
    /// losgelassen: ist die angeschaute Nachricht inzwischen weg (Pflege), kopiert `block` die folgende, und die
    /// liegt womöglich hinter der Marke — entnommen wird nur, was angeschaut wurde (Zeit der Kopie = Zeit des Kopfs).
    fn entnehmen(&mut self) -> Option<Nachricht> {
        let (zeit, _) = self.kopf?;
        let abfrage = Abfrage {
            abschnitt: Abschnitt::Nach(self.ab),
            n: Blockgroesse::bilden(1).expect("ein Satz ist eine gültige Blockgröße"),
        };
        let block = sperren(&self.verlauf).block(&abfrage, usize::MAX);
        self.ab = zeit;
        self.vorschauen();
        let nachricht = nachricht_in_fassung(block.saetze.first()?)?;
        if nachricht.zeit != zeit {
            return None;
        }
        Some(self.gespraeche.zustand_eintragen(nachricht))
    }
}

#[cfg(test)]
mod tests {
    use super::super::verlaufsindex::{KOPIERT, LIEFERUNGEN};
    use super::super::{Art, Gesendet, Sendung, Speicherablage};
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
            kosten: 4,
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

    /// N3 (B0757, K1): das Budget sind 2.000 Kosten je Aufruf (seit F0171 Kosten, für kurze Sätze gleich Sätze).
    #[test]
    fn das_budget_sind_zweitausend_kosten() {
        assert_eq!(NACHHOLBUDGET.kosten, 2_000);
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
        let kosten: Vec<usize> = aufrufe.iter().map(|aufruf| aufruf.kosten).collect();
        let saetze: Vec<usize> = aufrufe.iter().map(|aufruf| aufruf.saetze).collect();
        assert_eq!(
            (kosten, saetze),
            (vec![2_000, 2_000, 1_000], vec![2_000, 2_000, 1_000])
        );
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

    /// K1 (B0760, K6): Satzbytes 0 / 1.023 / 1.024 / 16.384 / 32.768 → Kosten 1 / 1 / 2 / 17 / 33 — abgerundet,
    /// ein kurzer Satz kostet nie weniger als 1, ein Satz zu 16 KiB das 17-fache.
    #[test]
    fn die_kosten_eines_satzes_wachsen_mit_den_satzbytes_je_kibibyte() {
        let kosten: Vec<usize> = [0, 1_023, 1_024, 16_384, 32_768]
            .map(kosten_eines_satzes)
            .to_vec();

        assert_eq!(kosten, vec![1, 1, 2, 17, 33]);
    }

    /// Ein Channel `a` mit `anzahl` Nachrichten von `anna`, jede mit einem Text aus `laenge` Buchstaben (ohne
    /// Zeilenumbruch), je 1 ms später; `t0…tn` wie in `reihe`.
    fn lange_reihe(anzahl: usize, laenge: usize) -> (Gespraeche, Vec<Zeit>) {
        let ablage = Arc::new(Speicherablage::neu());
        ablage.gespraech_anlegen(&Gespraech::channel("a"));
        let beginn = zeit("2026-10-30T12:00:00.000+01:00");
        let gespraeche =
            Gespraeche::laden(ablage, Arc::new(GestellteUhr::neu(beginn))).expect("geladen");
        let mut zeiten = vec![beginn - chrono::Duration::seconds(1)];
        for _ in 0..anzahl {
            let gesendet = gespraeche
                .senden(
                    &Gespraech::channel("a"),
                    &anna(),
                    Art::Mensch,
                    &"x".repeat(laenge),
                )
                .expect("gesendet");
            zeiten.push(gesendet.zeit);
        }
        (gespraeche, zeiten)
    }

    /// K2 (B0761): 300 Sätze zu 16 KiB (je 17 Kosten) ohne Treffer → 117 Sätze für 1.989 Kosten (117 × 17 ≤ 2.000 <
    /// 118 × 17), `weitere`; dann wieder 117; dann die restlichen 66 ohne `weitere`; lückenlos, ohne Doppel.
    #[test]
    fn dreihundert_lange_saetze_sind_117_117_und_66() {
        let (gespraeche, t) = lange_reihe(300, 16_384);

        let aufrufe = in_schleife(&gespraeche, &t, &erwaehnung());

        let je_aufruf: Vec<(usize, usize, Zeit, bool)> = aufrufe
            .iter()
            .map(|aufruf| (aufruf.saetze, aufruf.kosten, aufruf.stand, aufruf.weitere))
            .collect();
        assert_eq!(
            je_aufruf,
            vec![
                (117, 1_989, t[117], true),
                (117, 1_989, t[234], true),
                (66, 1_122, t[300], false),
            ]
        );
    }

    /// K2 Fortschritt (B0761): bei einem Budget von 5 wird ein Satz zu 16 KiB trotzdem geprüft — der erste Satz
    /// eines Aufrufs immer; der zweite nicht mehr (`weitere`), die Wiederholung kommt voran und endet.
    #[test]
    fn der_erste_satz_wird_auch_ueber_dem_budget_geprueft() {
        let (gespraeche, t) = lange_reihe(3, 16_384);
        let fuenf = Nachholbudget {
            kosten: 5,
            ..NACHHOLBUDGET
        };

        let erster = gespraeche.nachholen(t[0], t[3], &claude(), &erwaehnung(), fuenf);
        let zweiter = gespraeche.nachholen(erster.stand, t[3], &claude(), &erwaehnung(), fuenf);
        let dritter = gespraeche.nachholen(zweiter.stand, t[3], &claude(), &erwaehnung(), fuenf);

        assert_eq!(
            [&erster, &zweiter, &dritter].map(|aufruf| (
                aufruf.saetze,
                aufruf.stand,
                aufruf.weitere
            )),
            [(1, t[1], true), (1, t[2], true), (1, t[3], false)]
        );
    }

    /// Ein Channel `a` mit `anzahl` Nachrichten „x“ von `anna`, je 1 ms später; danach jede so bearbeitet, dass ihr
    /// Änderungssatz genau `satzbytes` Bytes lang ist (Text aus `y` in einer Zeile). `t0…tn` wie in `reihe`.
    fn bearbeitete_reihe(anzahl: usize, satzbytes: usize) -> (Gespraeche, Vec<Zeit>) {
        let (gespraeche, zeiten) = lange_reihe(anzahl, 1);
        let rahmen =
            super::super::logformat::bearbeitungssatz(zeiten[1], &anna(), zeiten[1], "y").len() - 1;
        for ziel in &zeiten[1..] {
            gespraeche
                .bearbeiten(
                    &Gespraech::channel("a"),
                    &anna(),
                    *ziel,
                    &"y".repeat(satzbytes - rahmen),
                )
                .expect("bearbeitet");
        }
        (gespraeche, zeiten)
    }

    /// Wie `lange_reihe`, danach jede Nachricht auf „y“ bearbeitet.
    fn kurz_bearbeitete_reihe(anzahl: usize, laenge: usize) -> (Gespraeche, Vec<Zeit>) {
        let (gespraeche, zeiten) = lange_reihe(anzahl, laenge);
        for ziel in &zeiten[1..] {
            gespraeche
                .bearbeiten(&Gespraech::channel("a"), &anna(), *ziel, "y")
                .expect("bearbeitet");
        }
        (gespraeche, zeiten)
    }

    /// Wie `lange_reihe`, danach jede Nachricht gelöscht.
    fn geloeschte_reihe(anzahl: usize, laenge: usize) -> (Gespraeche, Vec<Zeit>) {
        let (gespraeche, zeiten) = lange_reihe(anzahl, laenge);
        for ziel in &zeiten[1..] {
            gespraeche
                .nachricht_loeschen(&Gespraech::channel("a"), &anna(), *ziel)
                .expect("gelöscht");
        }
        (gespraeche, zeiten)
    }

    /// Kosten und Sätze eines Nachholens über `t0…tn` mit `erwaehnung` (kein Treffer).
    fn kosten_und_saetze((gespraeche, t): (Gespraeche, Vec<Zeit>)) -> (usize, usize) {
        let marke = *t.last().expect("Marke");
        let nachgeholt = gespraeche.nachholen(t[0], marke, &claude(), &erwaehnung(), NACHHOLBUDGET);
        (nachgeholt.kosten, nachgeholt.saetze)
    }

    /// Nacharbeit security-reviewer #17 Befund 1, Probe (1): eine bearbeitete Nachricht kostet nach dem Text, der
    /// ausgewertet wird — dem Änderungssatz —, nicht nach dem kurzen Original. Sechs „x“, jede bearbeitet auf
    /// 16.384 / 1.023 / 1.024 / 32.768 Satzbytes → 102 / 6 / 12 / 198; Kontrolle sechs unbearbeitete zu 16 KiB → 102.
    /// Dazu als Schutz: ein langes Original, kurz bearbeitet, kostet weiter nach dem Original (das Größere zählt, das
    /// Original wird auch gelesen); gelöschte kosten nach dem Original, das gelesen wird.
    #[test]
    fn bearbeitete_saetze_kosten_nach_dem_ausgewerteten_text() {
        let faelle = [
            ("bearbeitet auf 16.384", bearbeitete_reihe(6, 16_384), 102),
            ("bearbeitet auf 1.023", bearbeitete_reihe(6, 1_023), 6),
            ("bearbeitet auf 1.024", bearbeitete_reihe(6, 1_024), 12),
            ("bearbeitet auf 32.768", bearbeitete_reihe(6, 32_768), 198),
            ("Kontrolle unbearbeitet 16 KiB", lange_reihe(6, 16_384), 102),
            (
                "16 KiB, kurz bearbeitet",
                kurz_bearbeitete_reihe(6, 16_384),
                102,
            ),
            ("16 KiB gelöscht", geloeschte_reihe(6, 16_384), 102),
            ("kurz gelöscht", geloeschte_reihe(6, 1), 6),
        ];

        for (fall, reihe, kosten) in faelle {
            assert_eq!(kosten_und_saetze(reihe), (kosten, 6), "{fall}");
        }
    }

    /// Nacharbeit security-reviewer #17 Befund 1, Probe (2): 300 kurze Nachrichten, jede auf 16 KiB bearbeitet, wie
    /// 300 unbearbeitete zu 16 KiB → 117 / 117 / 66 Sätze bei 1.989 / 1.989 / 1.122 Kosten.
    #[test]
    fn dreihundert_auf_16_kib_bearbeitete_saetze_sind_117_117_und_66() {
        let (gespraeche, t) = bearbeitete_reihe(300, 16_384);

        let aufrufe = in_schleife(&gespraeche, &t, &erwaehnung());

        let je_aufruf: Vec<(usize, usize, Zeit, bool)> = aufrufe
            .iter()
            .map(|aufruf| (aufruf.saetze, aufruf.kosten, aufruf.stand, aufruf.weitere))
            .collect();
        assert_eq!(
            je_aufruf,
            vec![
                (117, 1_989, t[117], true),
                (117, 1_989, t[234], true),
                (66, 1_122, t[300], false),
            ]
        );
    }

    /// Wie die Nachrichten der Probe „Arbeit ≤ k × Buchung“ nachträglich verändert werden.
    #[derive(Debug, Clone, Copy)]
    enum Zustand {
        /// V1: unverändert, 16.384 Bytes.
        Unbearbeitet,
        /// V2: jede auf „y“ bearbeitet.
        KurzBearbeitet,
        /// V3: jede gelöscht.
        Geloescht,
        /// V4: als 1 Byte gesendet, danach auf 16.384 Bytes bearbeitet.
        Gewachsen,
        /// V5 (eigene Variante): 16.384 Bytes, danach auf 16.384 andere Bytes bearbeitet — Satz und Änderungssatz
        /// sind beide lang.
        Umgeschrieben,
    }

    const DIE_VIER: [Zustand; 4] = [
        Zustand::Unbearbeitet,
        Zustand::KurzBearbeitet,
        Zustand::Geloescht,
        Zustand::Gewachsen,
    ];

    /// `kanaele` Channels, reihum je 200 Nachrichten von `anna` (Erwähnung des Wartenden kommt nirgends vor), danach
    /// im `zustand`. Liefert die Gespräche, `t0` (vor der ersten Nachricht) und die Marke (die letzte gesendete).
    fn viele_kanaele(kanaele: usize, zustand: Zustand) -> (Gespraeche, Zeit, Zeit) {
        let ablage = Arc::new(Speicherablage::neu());
        let channels: Vec<Gespraech> = (0..kanaele)
            .map(|nummer| Gespraech::channel(&format!("k{nummer}")))
            .collect();
        for channel in &channels {
            ablage.gespraech_anlegen(channel);
        }
        let beginn = zeit("2026-10-30T12:00:00.000+01:00");
        let gespraeche =
            Gespraeche::laden(ablage, Arc::new(GestellteUhr::neu(beginn))).expect("geladen");
        let anfangstext = match zustand {
            Zustand::Gewachsen => "x".to_string(),
            _ => "x".repeat(16_384),
        };
        let mut gesendet = Vec::new();
        for _ in 0..200 {
            for channel in &channels {
                let nachricht = gespraeche
                    .senden(channel, &anna(), Art::Mensch, &anfangstext)
                    .expect("gesendet");
                gesendet.push((channel, nachricht.zeit));
            }
        }
        for (channel, zeit) in &gesendet {
            match zustand {
                Zustand::Unbearbeitet => {}
                Zustand::KurzBearbeitet => {
                    gespraeche
                        .bearbeiten(channel, &anna(), *zeit, "y")
                        .expect("bearbeitet");
                }
                Zustand::Gewachsen | Zustand::Umgeschrieben => {
                    gespraeche
                        .bearbeiten(channel, &anna(), *zeit, &"y".repeat(16_384))
                        .expect("bearbeitet");
                }
                Zustand::Geloescht => {
                    gespraeche
                        .nachricht_loeschen(channel, &anna(), *zeit)
                        .expect("gelöscht");
                }
            }
        }
        let marke = gesendet.last().expect("Nachrichten").1;
        (gespraeche, beginn - chrono::Duration::seconds(1), marke)
    }

    /// Ein Nachholen mit dem Standardbudget; liefert es und die Bytes, die der Verlauf dabei kopiert hat.
    fn nachholen_und_kopiertes(
        gespraeche: &Gespraeche,
        t0: Zeit,
        marke: Zeit,
    ) -> (Nachgeholt, usize) {
        KOPIERT.with(|kopiert| kopiert.set(0));
        let nachgeholt = gespraeche.nachholen(t0, marke, &claude(), &erwaehnung(), NACHHOLBUDGET);
        (nachgeholt, KOPIERT.with(std::cell::Cell::get))
    }

    /// Teil 1 der Abnahme-Probe (security-reviewer #17 Befund 3, realist-consultant #17 Schleifenbremse): am
    /// einzigen Kopierort wird gezählt, was kopiert wird — unabhängig von der Buchung. In allen zwölf Zellen
    /// (C ∈ {1, 10, 30} × V1–V4) gilt: (a) kopiert ≤ 1.024 × Kosten + 49.152, (b) 117 Sätze für 1.989 Kosten,
    /// (c) bei C = 30 absolut ≤ 2.100.000 Bytes. Gelesen wird nur, was gebucht ist.
    #[test]
    fn gelesen_wird_nur_was_gebucht_ist_in_zwoelf_zellen() {
        let mut verletzt = Vec::new();
        for kanaele in [1, 10, 30] {
            for zustand in DIE_VIER {
                let (gespraeche, t0, marke) = viele_kanaele(kanaele, zustand);

                let (nachgeholt, kopiert) = nachholen_und_kopiertes(&gespraeche, t0, marke);

                let zelle = format!(
                    "C={kanaele} {zustand:?}: {} Sätze, {} Kosten, kopiert {kopiert} B",
                    nachgeholt.saetze, nachgeholt.kosten
                );
                let (a, b, c) = (
                    kopiert <= BYTE_JE_KOSTENEINHEIT * nachgeholt.kosten + 49_152,
                    (nachgeholt.saetze, nachgeholt.kosten) == (117, 1_989),
                    kanaele != 30 || kopiert <= 2_100_000,
                );
                if !(a && b && c) {
                    verletzt.push(format!("{zelle} (a {a}, b {b}, c {c})"));
                }
            }
        }
        assert!(
            verletzt.is_empty(),
            "verletzte Zellen:\n{}",
            verletzt.join("\n")
        );
    }

    /// Ein Channel `a`: die erste Nachricht ist 16.384 Bytes lang, danach `kurze` Nachrichten „x“. Liefert die
    /// Gespräche, `t0` (vor der ersten) und die Marke (die letzte).
    fn lange_erste_dann_kurze(kurze: usize) -> (Gespraeche, Zeit, Zeit) {
        let ablage = Arc::new(Speicherablage::neu());
        ablage.gespraech_anlegen(&Gespraech::channel("a"));
        let beginn = zeit("2026-10-30T12:00:00.000+01:00");
        let gespraeche =
            Gespraeche::laden(ablage, Arc::new(GestellteUhr::neu(beginn))).expect("geladen");
        let senden = |text: &str| {
            gespraeche
                .senden(&Gespraech::channel("a"), &anna(), Art::Mensch, text)
                .expect("gesendet")
                .zeit
        };
        let mut marke = senden(&"x".repeat(16_384));
        for _ in 0..kurze {
            marke = senden("x");
        }
        (gespraeche, beginn - chrono::Duration::seconds(1), marke)
    }

    /// Befund 5 (security-reviewer #17) und S1 (code-reviewer #17): auch die Kopfzeile des ältesten bzw. jüngsten
    /// Satzes (`aelteste_kopfzeit`, `juengste_kopfzeit` je `block`-Aufruf) wird am einzigen Kopierort gezählt —
    /// und kopiert nicht mehr den ganzen Satz. 16-KiB-Erstnachricht + 1.999 Nachrichten zu 1 Byte: kopiert ≤
    /// 1.024 × Kosten + 49.152 (heute 32,8 MB gegen 2,1 MB).
    #[test]
    fn die_kopfzeile_des_aeltesten_satzes_wird_nicht_je_schritt_kopiert() {
        let (gespraeche, t0, marke) = lange_erste_dann_kurze(1_999);

        let (nachgeholt, kopiert) = nachholen_und_kopiertes(&gespraeche, t0, marke);

        assert_eq!(nachgeholt.saetze, 1_984);
        assert!(
            kopiert <= BYTE_JE_KOSTENEINHEIT * nachgeholt.kosten + 49_152,
            "kopiert {kopiert} B für {} Kosten",
            nachgeholt.kosten
        );
    }

    /// S2 (code-reviewer #17): „Budget vor dem Lesen“ ist festgenagelt, nicht nur begrenzt. Ein Channel, 200
    /// Nachrichten zu 16.384 Bytes (je 17 Kosten): kopiert wird genau, was gebucht ist — 117 Sätze, nicht ein
    /// 118. (die M6-Mutante, `entnehmen` vor der Budgetprüfung, kopiert ihn und verwirft ihn dann).
    #[test]
    fn das_budget_wird_vor_dem_lesen_geprueft_kopiert_wird_genau_die_buchung() {
        let (gespraeche, t0, marke) = viele_kanaele(1, Zustand::Unbearbeitet);
        let satzlaenge = {
            let verlauf =
                lesend(&gespraeche.verzeichnis).verlaeufe[&Gespraech::channel("k0")].clone();
            let (_, lesebytes) = sperren(&verlauf).naechste_nach(&t0).expect("erste Nachricht");
            lesebytes
        };

        let (nachgeholt, kopiert) = nachholen_und_kopiertes(&gespraeche, t0, marke);

        assert_eq!(nachgeholt.saetze, 117);
        assert!(nachgeholt.weitere);
        assert_eq!(kopiert, 117 * satzlaenge);
    }

    /// Ein Channel `a`: ein Original mit einer 16-KiB-Zeile, danach `antworten` Antworten „x“ darauf. Liefert die
    /// Gespräche, `t0` (vor dem Original) und die Marke (die letzte Antwort).
    fn original_mit_antworten(antworten: usize) -> (Gespraeche, Zeit, Zeit) {
        let ablage = Arc::new(Speicherablage::neu());
        ablage.gespraech_anlegen(&Gespraech::channel("a"));
        let beginn = zeit("2026-10-30T12:00:00.000+01:00");
        let gespraeche =
            Gespraeche::laden(ablage, Arc::new(GestellteUhr::neu(beginn))).expect("geladen");
        let original = gespraeche
            .senden(
                &Gespraech::channel("a"),
                &anna(),
                Art::Mensch,
                &"x".repeat(16_384),
            )
            .expect("Original")
            .zeit;
        let mut marke = original;
        for _ in 0..antworten {
            let antwort = Sendung {
                text: "x",
                anhaenge: &[],
                schluessel: None,
                antwort_auf: Some(original),
            };
            marke = match gespraeche
                .senden_sendung(&Gespraech::channel("a"), &anna(), Art::Mensch, antwort)
                .expect("Antwort")
            {
                Gesendet::Neu(nachricht) | Gesendet::Wiederholt(nachricht) => nachricht.zeit,
            };
        }
        (gespraeche, beginn - chrono::Duration::seconds(1), marke)
    }

    /// Befund 4 (a) (security-reviewer #17): das Zitat einer Antwort wird nur für einen Satz aufgelöst, der auch
    /// genommen wird — kein Vorausschauen auf den Folgesatz, den der Sammler dann ablehnt. Ein Original mit
    /// 16-KiB-Zeile, 2.000 Antworten „x“: `lieferung`-Aufrufe ≤ Sätze + C (C = 1; heute 3.968 für 1.984 Sätze).
    #[test]
    fn das_zitat_wird_nur_fuer_genommene_saetze_aufgeloest() {
        let (gespraeche, t0, marke) = original_mit_antworten(2_000);
        LIEFERUNGEN.with(|aufrufe| aufrufe.set(0));

        let nachgeholt = gespraeche.nachholen(t0, marke, &claude(), &erwaehnung(), NACHHOLBUDGET);

        let aufrufe = LIEFERUNGEN.with(std::cell::Cell::get);
        assert!(nachgeholt.saetze > 1_900, "{} Sätze", nachgeholt.saetze);
        assert!(
            aufrufe <= nachgeholt.saetze + 1,
            "{aufrufe} lieferung-Aufrufe für {} Sätze",
            nachgeholt.saetze
        );
    }

    /// Mikrosekunden je Kosteneinheit eines Nachholens, Median aus sieben Läufen.
    fn mikrosekunden_je_einheit(gespraeche: &Gespraeche, t0: Zeit, marke: Zeit) -> f64 {
        let mut dauern: Vec<f64> = (0..7)
            .map(|_| {
                let beginn = std::time::Instant::now();
                let nachgeholt =
                    gespraeche.nachholen(t0, marke, &claude(), &erwaehnung(), NACHHOLBUDGET);
                beginn.elapsed().as_secs_f64() * 1e6 / nachgeholt.kosten as f64
            })
            .collect();
        dauern.sort_by(f64::total_cmp);
        dauern[3]
    }

    /// Befund 4 (b) (security-reviewer #17), Abnahme-Probe 2: die Zitat-Auflösung auf eine 16-KiB-Zeile kostet je
    /// Einheit höchstens das Dreifache der Kontrolle ohne Zitat im selben Lauf (vor der Härtung ≈ 11×, nach Befund 4a ≈ 6×). Eine Lastbeobachtung im
    /// Release, kein Gate: die Wanduhr schwankt; den Vertrag hält `die_zeile_wird_vor_dem_pruefen_gekuerzt` in
    /// `zitat.rs`. Gefahren mit `cargo test --release --lib zitat_auf_lange_zeile -- --ignored --nocapture`.
    #[test]
    #[ignore = "Lastbeobachtung (Wanduhr): cargo test --release --lib zitat_auf_lange_zeile -- --ignored --nocapture"]
    fn zitat_auf_lange_zeile_kostet_hoechstens_das_dreifache_der_kontrolle() {
        let (mit_zitat, t0, marke) = original_mit_antworten(2_000);
        let (kontrolle, k0, kmarke) = lange_erste_dann_kurze(2_000);

        let zitat_us = mikrosekunden_je_einheit(&mit_zitat, t0, marke);
        let kontrolle_us = mikrosekunden_je_einheit(&kontrolle, k0, kmarke);
        println!("mit Zitat {zitat_us:.2} µs/Einheit, Kontrolle {kontrolle_us:.2} µs/Einheit");

        assert!(
            zitat_us <= 3.0 * kontrolle_us,
            "mit Zitat {zitat_us:.2} µs, Kontrolle {kontrolle_us:.2} µs je Einheit"
        );
    }

    /// O1 (code-reviewer #17): zwischen `vorschauen` und `entnehmen` ist die Sperre losgelassen; geht die
    /// angeschaute Nachricht inzwischen mit ihrem Tag (Pflege), kopiert `block` die folgende — die liegt hier hinter
    /// der Marke. Entnommen wird nur, was angeschaut wurde: die Zeit der Kopie muss die des Kopfs sein. Die folgende
    /// kommt später, in der Reihe an ihrer Stelle.
    #[test]
    fn entnommen_wird_nur_was_angeschaut_wurde() {
        let ablage = Arc::new(Speicherablage::neu());
        ablage.gespraech_anlegen(&Gespraech::channel("a"));
        let beginn = zeit("2026-10-30T12:00:00.000+01:00");
        let uhr = Arc::new(GestellteUhr::neu(beginn));
        let gespraeche = Gespraeche::laden(ablage, uhr.clone()).expect("geladen");
        let senden = || {
            gespraeche
                .senden(&Gespraech::channel("a"), &anna(), Art::Mensch, "x")
                .expect("gesendet")
                .zeit
        };
        let erste = senden();
        uhr.stellen(beginn + chrono::Duration::days(1));
        let zweite = senden();
        let mut straenge = gespraeche.straenge(
            beginn - chrono::Duration::seconds(1),
            erste,
            &erwaehnung(),
        );
        let strang = &mut straenge[0];
        assert_eq!(strang.kopf.map(|(zeit, _)| zeit), Some(erste));
        sperren(&strang.verlauf).tag_entfernen(erste.date_naive());

        let entnommen = strang.entnehmen();

        assert_eq!(entnommen, None, "die zweite liegt hinter der Marke");
        assert_eq!(strang.kopf, None, "und kommt nicht über die Marke");
        assert!(zweite > erste);
    }

    /// Eigene Variante V5 zu Teil 1: sind Satz und Änderungssatz beide 16 KiB lang, bucht das Nachholen das Größere,
    /// der Verlauf kopiert beide — die Arbeit bleibt bei höchstens dem Doppelten der Buchung.
    #[test]
    fn umgeschriebene_saetze_kopieren_hoechstens_das_doppelte_der_buchung() {
        let (gespraeche, t0, marke) = viele_kanaele(30, Zustand::Umgeschrieben);

        let (nachgeholt, kopiert) = nachholen_und_kopiertes(&gespraeche, t0, marke);

        assert_eq!((nachgeholt.saetze, nachgeholt.kosten), (117, 1_989));
        assert!(
            kopiert <= 2 * (BYTE_JE_KOSTENEINHEIT * nachgeholt.kosten + 49_152),
            "kopiert {kopiert} B für {} Kosten",
            nachgeholt.kosten
        );
    }

    /// Teil 2 der Abnahme-Probe, Spiegelung: für je eine Nachricht V1–V4 sind Zeit und Lesebytes von
    /// `naechste_nach` genau das, was `block(Nach, n = 1)` über die Kopie auswertet — das Größere aus Satz und
    /// Änderungssatz, den die Trefferregel auswertet —, und die kopierte Summe (am Zähler) ist Satz plus Änderungssatz.
    #[test]
    fn die_vorschau_spiegelt_den_block_mit_einem_satz() {
        for zustand in DIE_VIER {
            let (gespraeche, t0, _) = viele_kanaele(1, zustand);
            let verlauf =
                lesend(&gespraeche.verzeichnis).verlaeufe[&Gespraech::channel("k0")].clone();
            let abfrage = Abfrage {
                abschnitt: Abschnitt::Nach(t0),
                n: Blockgroesse::bilden(1).expect("n"),
            };
            KOPIERT.with(|kopiert| kopiert.set(0));
            let block = sperren(&verlauf).block(&abfrage, usize::MAX);
            let kopiert = KOPIERT.with(std::cell::Cell::get);
            let kopie = &block.saetze[0];

            let vorschau = sperren(&verlauf).naechste_nach(&t0);

            let zelle = format!("{zustand:?}");
            let aenderungssatz = kopie
                .aenderung
                .as_ref()
                .and_then(|aenderung| aenderung.satz.as_ref())
                .map_or(0, Vec::len);
            let erste = nachricht_in_fassung(kopie).expect("Nachricht").zeit;
            assert_eq!(
                vorschau,
                Some((erste, kopie.satz.len().max(aenderungssatz))),
                "{zelle}: Zeit und Lesebytes"
            );
            assert_eq!(
                kopiert,
                kopie.satz.len() + aenderungssatz,
                "{zelle}: kopierte Summe"
            );
        }
    }

    /// Wie die Vorschau endet: hinter der letzten Nachricht ist keine nächste.
    #[test]
    fn hinter_der_letzten_nachricht_gibt_es_keine_vorschau() {
        let (gespraeche, t) = lange_reihe(3, 5);
        let verlauf = lesend(&gespraeche.verzeichnis).verlaeufe[&Gespraech::channel("a")].clone();

        let nach_der_zweiten = sperren(&verlauf).naechste_nach(&t[2]).map(|(zeit, _)| zeit);
        let nach_der_letzten = sperren(&verlauf).naechste_nach(&t[3]);

        assert_eq!(nach_der_zweiten, Some(t[3]));
        assert_eq!(nach_der_letzten, None);
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

    /// B0774 (ADR-0055 Regel 8, Matrix G2; Mutante „Marke global“): `t1` in einem Channel, danach `t2` im
    /// Direktgespräch von `anna` und `claude`. `dora` sieht nur den Channel — ihre sichtbare Marke ist `t1`, ein
    /// Nachholen von vor `t1` endet mit `stand = t1`; die Beteiligten sehen `t2`. Eine Direktnachricht über der
    /// angefragten Marke zählt nicht.
    #[test]
    fn die_sichtbare_marke_kennt_nur_channels_und_die_eigenen_direktgespraeche() {
        let ablage = Arc::new(Speicherablage::neu());
        let direkt = Gespraech::Direkt(super::super::Direktpaar::fuer_test(
            "anna@127.0.0.1",
            "claude@127.0.0.1",
        ));
        ablage.gespraech_anlegen(&Gespraech::channel("a"));
        ablage.gespraech_anlegen(&direkt);
        let uhr = Arc::new(GestellteUhr::neu(zeit("2026-10-30T12:00:00.000+01:00")));
        let gespraeche = Gespraeche::laden(ablage, uhr.clone()).expect("geladen");
        let t1 = gespraeche
            .senden(&Gespraech::channel("a"), &anna(), Art::Mensch, "im Channel")
            .expect("t1")
            .zeit;
        uhr.stellen(zeit("2026-10-30T12:00:05.000+01:00"));
        let t2 = gespraeche
            .senden(&direkt, &anna(), Art::Mensch, "nur wir")
            .expect("t2")
            .zeit;
        let dora = kennung("dora@127.0.0.1");
        let vorher = zeit("2026-10-30T11:00:00.000+01:00");

        let nachgeholt = gespraeche.nachholen(vorher, t2, &dora, &Filter::default(), NACHHOLBUDGET);

        assert_eq!(gespraeche.sichtbare_marke(&dora, t2), Some(t1), "dora");
        assert_eq!(
            gespraeche.sichtbare_marke(&claude(), t2),
            Some(t2),
            "claude"
        );
        assert_eq!(gespraeche.sichtbare_marke(&anna(), t2), Some(t2), "anna");
        assert_eq!(
            gespraeche.sichtbare_marke(&claude(), t1),
            Some(t1),
            "über der Marke zählt nicht"
        );
        assert_eq!(nachgeholt.stand, t1, "der Stand verrät t2 nicht");
        assert_eq!(
            gespraeche.sichtbare_marke(&kennung("nie@127.0.0.1"), t2),
            Some(t1)
        );
    }

    /// Ohne ein sichtbares Gespräch mit Nachricht gibt es keine sichtbare Marke.
    #[test]
    fn wer_nichts_sieht_hat_keine_sichtbare_marke() {
        let ablage = Arc::new(Speicherablage::neu());
        let direkt = Gespraech::Direkt(super::super::Direktpaar::fuer_test(
            "anna@127.0.0.1",
            "claude@127.0.0.1",
        ));
        ablage.gespraech_anlegen(&direkt);
        let uhr = Arc::new(GestellteUhr::neu(zeit("2026-10-30T12:00:00.000+01:00")));
        let gespraeche = Gespraeche::laden(ablage, uhr).expect("geladen");
        let t = gespraeche
            .senden(&direkt, &anna(), Art::Mensch, "nur wir")
            .expect("gesendet")
            .zeit;

        assert_eq!(
            gespraeche.sichtbare_marke(&kennung("dora@127.0.0.1"), t),
            None
        );
    }
}
