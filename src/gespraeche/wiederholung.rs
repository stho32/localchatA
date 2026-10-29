//! Wiederholschlüssel (B0237, ADR-0034 Regel 6 mit Ergänzungen Karte #36): `(Kennung, Schlüssel) → (Gespräch, Zeit,
//! Prüfsumme)`, höchstens 640 je Kennung und 36.000 insgesamt, je höchstens 10 Minuten ab dem Merken. Rein, mit
//! gestellter Zeit; gehalten unter der Schreibsperre der Gespräche, nur im Speicher. Der Speicher meldet eine
//! Verdrängung vor Ablauf nur; Protokoll und Zähler macht der Aufrufer nach der Sperre (Regel E1.7).

use super::{Gespraech, Kennung};
use crate::uhr::Zeit;
use std::collections::HashMap;
use std::fmt;

/// Höchstens so viele Schlüssel je Kennung (ADR-0034 Ergänzung Karte #36: Bedarf 630 aus dem Schreibkonto + Reserve).
pub const HOECHSTENS_JE_KENNUNG: usize = 640;
/// Höchstens so viele Schlüssel insgesamt — das Doppelte der Spitze des Lastprofils über 10 Minuten.
///
/// Speicher (ADR-0034 Ergänzung „Karte #36 Speicher“, S1): höchstens 1.200 Bytes allokiert je Eintrag im
/// ungünstigsten Fall (je Eintrag eine eigene Kennung), bei 36.000 also ≤ 43,2 MB; im Lastprofil ≈ 10 MB. Gemessen
/// am zählenden Allokator in `tests/wiederholung_speicher.rs` (S2), nicht am RSS der Instanz.
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

/// Der Speicher (Regeln E1.3, E1.4): Einträge auf wiederverwendeten Plätzen, verkettet in der Folge des Merkens
/// (gesamt) und je Kennung; je Kennung ein Index Schlüssel → Platz; je Anzahl 1 … `je_kennung` ein Fach mit den
/// Kennungen in der Folge, in der sie diese Anzahl erreicht haben, und ein Zeiger auf das höchste besetzte Fach.
/// Nachschlagen, Merken und jede Verdrängung sind amortisiert konstant; eine Kennung ohne Einträge verschwindet
/// im selben Zug aus allen Strukturen.
#[derive(Debug, Clone)]
pub struct Wiederholung {
    je_kennung: usize,
    gesamt: usize,
    eintraege: Plaetze<Eintrag>,
    kennungen: Plaetze<Kennungsstand>,
    nach_kennung: HashMap<Kennung, usize>,
    /// Alle Einträge, ältester zuerst.
    folge: Reihe,
    /// Index = Anzahl der Einträge einer Kennung; Fach 0 bleibt leer.
    faecher: Vec<Reihe>,
    hoechstes_fach: usize,
    anzahl: usize,
}

#[derive(Debug, Clone)]
struct Eintrag {
    kennung: usize,
    schluessel: String,
    gespraech: Gespraech,
    pruefsumme: u64,
    zeit: Zeit,
    gemerkt: Zeit,
    in_folge: Glied,
    in_eigenen: Glied,
}

#[derive(Debug, Clone)]
struct Kennungsstand {
    kennung: Kennung,
    schluessel: HashMap<String, usize>,
    /// Die Einträge dieser Kennung, ältester zuerst.
    eigene: Reihe,
    im_fach: Glied,
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
            eintraege: Plaetze::neu(),
            kennungen: Plaetze::neu(),
            nach_kennung: HashMap::new(),
            folge: Reihe::default(),
            faecher: vec![Reihe::default(); je_kennung + 1],
            hoechstes_fach: 0,
            anzahl: 0,
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
        let Some(eintrag) = self
            .platz_von(kennung, schluessel)
            .and_then(|platz| self.eintraege.lesen(platz))
        else {
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
        if let Some(platz) = self.platz_von(kennung, schluessel) {
            self.entfernen(platz);
        }
        self.abgelaufene_entfernen(jetzt);
        let verdraengt = if self.anzahl_von(kennung) >= self.je_kennung {
            self.aeltesten_verdraengen(kennung, Grenze::Kennung, jetzt)
        } else if self.anzahl >= self.gesamt {
            self.aeltesten_der_meisten_verdraengen(jetzt)
        } else {
            None
        };
        self.einfuegen(kennung, schluessel, gespraech, pruefsumme, zeit, jetzt);
        verdraengt
    }

    fn platz_von(&self, kennung: &Kennung, schluessel: &str) -> Option<usize> {
        let stand = self.kennungen.lesen(*self.nach_kennung.get(kennung)?)?;
        stand.schluessel.get(schluessel).copied()
    }

    fn anzahl_von(&self, kennung: &Kennung) -> usize {
        self.nach_kennung
            .get(kennung)
            .and_then(|&platz| self.kennungen.lesen(platz))
            .map_or(0, |stand| stand.schluessel.len())
    }

    /// Abgelaufene stehen vorn in der Folge des Merkens; eine zurückgestellte Uhr lässt sie später fallen.
    fn abgelaufene_entfernen(&mut self, jetzt: Zeit) {
        while let Some(aeltester) = self.folge.erster {
            match self.eintraege.lesen(aeltester) {
                Some(eintrag) if abgelaufen(eintrag, jetzt) => {
                    self.entfernen(aeltester);
                }
                _ => break,
            }
        }
    }

    fn aeltesten_verdraengen(
        &mut self,
        kennung: &Kennung,
        grenze: Grenze,
        jetzt: Zeit,
    ) -> Option<Verdraengt> {
        let stand = self.kennungen.lesen(*self.nach_kennung.get(kennung)?)?;
        let aeltester = stand.eigene.erster?;
        self.verdraengen(aeltester, grenze, jetzt)
    }

    fn aeltesten_der_meisten_verdraengen(&mut self, jetzt: Zeit) -> Option<Verdraengt> {
        let meiste = self.faecher.get(self.hoechstes_fach)?.erster?;
        let aeltester = self.kennungen.lesen(meiste)?.eigene.erster?;
        self.verdraengen(aeltester, Grenze::Gesamt, jetzt)
    }

    /// Entfernt einen Eintrag; gemeldet wird er nur, wenn er noch galt.
    fn verdraengen(&mut self, platz: usize, grenze: Grenze, jetzt: Zeit) -> Option<Verdraengt> {
        let (kennung, eintrag) = self.entfernen(platz)?;
        if abgelaufen(&eintrag, jetzt) {
            return None;
        }
        let hoechstens = match grenze {
            Grenze::Kennung => self.je_kennung,
            Grenze::Gesamt => self.gesamt,
        };
        Some(Verdraengt {
            kennung,
            grenze,
            hoechstens,
            alter_ms: alter_ms(&eintrag, jetzt),
        })
    }

    fn entfernen(&mut self, platz: usize) -> Option<(Kennung, Eintrag)> {
        let k = self.eintraege.lesen(platz)?.kennung;
        aushaengen(&mut self.folge, &mut self.eintraege, platz, |e| {
            &mut e.in_folge
        });
        if let Some(stand) = self.kennungen.schreiben(k) {
            aushaengen(&mut stand.eigene, &mut self.eintraege, platz, |e| {
                &mut e.in_eigenen
            });
        }
        let eintrag = self.eintraege.nehmen(platz)?;
        self.anzahl -= 1;
        let stand = self.kennungen.schreiben(k)?;
        stand.schluessel.remove(&eintrag.schluessel);
        let nachher = stand.schluessel.len();
        let kennung = stand.kennung.clone();
        self.fach_wechseln(k, nachher + 1, nachher);
        Some((kennung, eintrag))
    }

    fn einfuegen(
        &mut self,
        kennung: &Kennung,
        schluessel: &str,
        gespraech: &Gespraech,
        pruefsumme: u64,
        zeit: Zeit,
        jetzt: Zeit,
    ) {
        let k = match self.nach_kennung.get(kennung) {
            Some(&k) => k,
            None => {
                let k = self.kennungen.ablegen(Kennungsstand {
                    kennung: kennung.clone(),
                    schluessel: HashMap::new(),
                    eigene: Reihe::default(),
                    im_fach: Glied::default(),
                });
                self.nach_kennung.insert(kennung.clone(), k);
                k
            }
        };
        let platz = self.eintraege.ablegen(Eintrag {
            kennung: k,
            schluessel: schluessel.to_string(),
            gespraech: gespraech.clone(),
            pruefsumme,
            zeit,
            gemerkt: jetzt,
            in_folge: Glied::default(),
            in_eigenen: Glied::default(),
        });
        self.anzahl += 1;
        anhaengen(&mut self.folge, &mut self.eintraege, platz, |e| {
            &mut e.in_folge
        });
        let Some(stand) = self.kennungen.schreiben(k) else {
            return;
        };
        anhaengen(&mut stand.eigene, &mut self.eintraege, platz, |e| {
            &mut e.in_eigenen
        });
        let vorher = stand.schluessel.len();
        stand.schluessel.insert(schluessel.to_string(), platz);
        self.fach_wechseln(k, vorher, vorher + 1);
    }

    /// Die Kennung wechselt von der Anzahl `von` auf `nach`: hinten ins neue Fach; bei 0 verschwindet sie.
    fn fach_wechseln(&mut self, k: usize, von: usize, nach: usize) {
        if let Some(fach) = self.faecher.get_mut(von).filter(|_| von > 0) {
            aushaengen(fach, &mut self.kennungen, k, |s| &mut s.im_fach);
        }
        if nach == 0 {
            if let Some(stand) = self.kennungen.nehmen(k) {
                self.nach_kennung.remove(&stand.kennung);
            }
        } else if let Some(fach) = self.faecher.get_mut(nach) {
            anhaengen(fach, &mut self.kennungen, k, |s| &mut s.im_fach);
            self.hoechstes_fach = self.hoechstes_fach.max(nach);
        }
        while self.hoechstes_fach > 0 && self.faecher[self.hoechstes_fach].erster.is_none() {
            self.hoechstes_fach -= 1;
        }
    }
}

/// Glied einer verketteten Reihe über Plätze.
#[derive(Debug, Clone, Copy, Default)]
struct Glied {
    vor: Option<usize>,
    nach: Option<usize>,
}

/// Anfang und Ende einer verketteten Reihe.
#[derive(Debug, Clone, Copy, Default)]
struct Reihe {
    erster: Option<usize>,
    letzter: Option<usize>,
}

/// Plätze mit Wiederverwendung: nie mehr, als gleichzeitig belegt waren.
#[derive(Debug, Clone)]
struct Plaetze<T> {
    belegt: Vec<Option<T>>,
    frei: Vec<usize>,
}

impl<T> Plaetze<T> {
    fn neu() -> Plaetze<T> {
        Plaetze {
            belegt: Vec::new(),
            frei: Vec::new(),
        }
    }

    fn ablegen(&mut self, wert: T) -> usize {
        match self.frei.pop() {
            Some(platz) => {
                self.belegt[platz] = Some(wert);
                platz
            }
            None => {
                self.belegt.push(Some(wert));
                self.belegt.len() - 1
            }
        }
    }

    fn nehmen(&mut self, platz: usize) -> Option<T> {
        let wert = self.belegt.get_mut(platz)?.take()?;
        self.frei.push(platz);
        Some(wert)
    }

    fn lesen(&self, platz: usize) -> Option<&T> {
        self.belegt.get(platz)?.as_ref()
    }

    fn schreiben(&mut self, platz: usize) -> Option<&mut T> {
        self.belegt.get_mut(platz)?.as_mut()
    }
}

fn anhaengen<T>(
    reihe: &mut Reihe,
    plaetze: &mut Plaetze<T>,
    platz: usize,
    glied: fn(&mut T) -> &mut Glied,
) {
    let bisher_letzter = reihe.letzter;
    if let Some(element) = plaetze.schreiben(platz) {
        *glied(element) = Glied {
            vor: bisher_letzter,
            nach: None,
        };
    }
    match bisher_letzter.and_then(|letzter| plaetze.schreiben(letzter)) {
        Some(letzter) => glied(letzter).nach = Some(platz),
        None => reihe.erster = Some(platz),
    }
    reihe.letzter = Some(platz);
}

fn aushaengen<T>(
    reihe: &mut Reihe,
    plaetze: &mut Plaetze<T>,
    platz: usize,
    glied: fn(&mut T) -> &mut Glied,
) {
    let Some(Glied { vor, nach }) = plaetze.schreiben(platz).map(|element| *glied(element)) else {
        return;
    };
    match vor.and_then(|vor| plaetze.schreiben(vor)) {
        Some(vorgaenger) => glied(vorgaenger).nach = nach,
        None => reihe.erster = nach,
    }
    match nach.and_then(|nach| plaetze.schreiben(nach)) {
        Some(nachfolger) => glied(nachfolger).vor = vor,
        None => reihe.letzter = vor,
    }
    if let Some(element) = plaetze.schreiben(platz) {
        *glied(element) = Glied::default();
    }
}

/// Größen der inneren Strukturen (R00025 „Strukturen begrenzt“): je Struktur Elemente und Kennungen darin.
#[cfg(test)]
#[derive(Debug)]
struct Groessen {
    eintraege: usize,
    strukturen: Vec<(&'static str, usize, usize)>,
}

#[cfg(test)]
impl Groessen {
    fn groesste_kennungszahl(&self) -> usize {
        self.strukturen
            .iter()
            .map(|(_, _, kennungen)| *kennungen)
            .max()
            .unwrap_or(0)
    }

    fn groesste_elementzahl(&self) -> usize {
        self.strukturen
            .iter()
            .map(|(_, elemente, _)| *elemente)
            .max()
            .unwrap_or(0)
    }
}

#[cfg(test)]
impl Wiederholung {
    fn groessen(&self) -> Groessen {
        use std::collections::HashSet;
        let reihe = |reihe: &Reihe, plaetze_nach: &dyn Fn(usize) -> Option<usize>| {
            let mut glieder = Vec::new();
            let mut naechster = reihe.erster;
            while let Some(platz) = naechster {
                glieder.push(platz);
                naechster = plaetze_nach(platz);
            }
            glieder
        };
        let in_folge = reihe(&self.folge, &|p| self.eintraege.lesen(p)?.in_folge.nach);
        let kennungen_in_folge: HashSet<usize> = in_folge
            .iter()
            .filter_map(|&p| Some(self.eintraege.lesen(p)?.kennung))
            .collect();
        let mut in_faechern = 0;
        for fach in &self.faecher {
            in_faechern += reihe(fach, &|k| self.kennungen.lesen(k)?.im_fach.nach).len();
        }
        let staende: Vec<&Kennungsstand> = self.kennungen.belegt.iter().flatten().collect();
        let mut in_eigenen = 0;
        for stand in &staende {
            in_eigenen += reihe(&stand.eigene, &|p| self.eintraege.lesen(p)?.in_eigenen.nach).len();
        }
        let im_index: usize = staende.iter().map(|stand| stand.schluessel.len()).sum();
        Groessen {
            eintraege: self.anzahl,
            strukturen: vec![
                (
                    "eintragsplaetze",
                    self.eintraege.belegt.len(),
                    kennungen_in_folge.len(),
                ),
                ("folge", in_folge.len(), kennungen_in_folge.len()),
                (
                    "kennungsplaetze",
                    self.kennungen.belegt.len(),
                    staende.len(),
                ),
                (
                    "nach_kennung",
                    self.nach_kennung.len(),
                    self.nach_kennung.len(),
                ),
                ("faecher", in_faechern, in_faechern),
                ("eigene", in_eigenen, staende.len()),
                ("index", im_index, staende.len()),
            ],
        }
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
    pruefsumme_mit(text, &[])
}

/// Prüfsumme über Text **und** Anhang-IDs (ADR-0034 Ergänzung Karte #9 Punkt 4): ohne Anhänge dieselbe wie über den
/// Text allein; gleicher Schlüssel mit anderen IDs → wie ein anderer Text.
pub fn pruefsumme_mit(text: &str, anhaenge: &[super::anhangid::Anhangid]) -> u64 {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    for anhang in anhaenge {
        anhang.als_text().hash(&mut hasher);
    }
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Prüfsumme über Text und IDs: dieselben IDs → gleich; andere ID oder andere Reihenfolge → anders; ohne IDs
    /// wie über den Text allein.
    #[test]
    fn die_pruefsumme_umfasst_text_und_anhang_ids() {
        let id = |text: &str| super::super::anhangid::Anhangid::lesen(text).expect("ID");
        let (a, b) = (
            id("aaaaaaaaaaaaaaaaaaaaaaaaaa"),
            id("bbbbbbbbbbbbbbbbbbbbbbbbbb"),
        );
        assert_eq!(pruefsumme_mit("x", &[]), pruefsumme("x"));
        assert_eq!(
            pruefsumme_mit("x", std::slice::from_ref(&a)),
            pruefsumme_mit("x", std::slice::from_ref(&a))
        );
        assert_ne!(
            pruefsumme_mit("x", std::slice::from_ref(&a)),
            pruefsumme_mit("x", std::slice::from_ref(&b))
        );
        assert_ne!(
            pruefsumme_mit("x", &[a.clone(), b.clone()]),
            pruefsumme_mit("x", &[b, a.clone()])
        );
        assert_ne!(pruefsumme_mit("x", &[a]), pruefsumme("x"));
    }

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
        assert_eq!(gemerkt.groessen().eintraege, 1);
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

    /// Wächter „Uhr zurück“ (code-reviewer #36 A2, Abnahme-Probe wörtlich): „bei 10.000 ms merkt `a` „a1“; die Uhr
    /// springt auf 0, `b` merkt „k“; bei 600.001 ms ist „k“ neu und wird neu gemerkt; bei 610.002 ms merkt `c` „c1“.
    /// Erwartet ist dann `pruefen(b, "k")` = `Wiederholt { zeit: 600.001 }` und 2 Einträge. Mit Mutante (Zeilen 181–183
    /// entfernt) kommt `Neu`.“ Gemeint ist der Wächter in `merken`, der einen vorhandenen Eintrag desselben Schlüssels
    /// vor dem Einfügen entfernt: sonst bliebe der alte, in der Folge hinter `a1` stehende Eintrag, und sein spätes
    /// Entfernen löschte den Verweis auf den neuen.
    #[test]
    fn eine_zurueckgestellte_uhr_laesst_den_neu_gemerkten_schluessel_nicht_vergessen() {
        let (a, b, c) = (
            kennung("a@127.0.0.1"),
            kennung("b@127.0.0.1"),
            kennung("c@127.0.0.1"),
        );
        let mut speicher = Wiederholung::default();
        merke(&mut speicher, &a, "a1", 10_000);
        merke(&mut speicher, &b, "k", 0);
        assert_eq!(
            speicher.pruefen(&b, "k", &allgemein(), pruefsumme("k"), t(600_001)),
            Pruefung::Neu,
            "nach 600.001 ms ist „k“ abgelaufen"
        );
        merke(&mut speicher, &b, "k", 600_001);

        merke(&mut speicher, &c, "c1", 610_002);

        assert_eq!(
            speicher.pruefen(&b, "k", &allgemein(), pruefsumme("k"), t(610_002)),
            Pruefung::Wiederholt { zeit: t(600_001) }
        );
        assert_eq!(speicher.groessen().eintraege, 2);
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

    /// Gemeinsames Präfix der Schlüssel im ungünstigen Fall (Z16): 58 gleiche Zeichen und eine laufende Nummer
    /// mit 6 Stellen — 64 Zeichen, die längste erlaubte Form.
    fn schluessel_mit_praefix(nummer: usize) -> String {
        format!("{}{nummer:06}", "p".repeat(58))
    }

    /// 10.000 × `pruefen` + `merken` einer Kennung mit neuen Schlüsseln; die Dauer.
    fn zehntausend_paare(
        speicher: &mut Wiederholung,
        wer: &Kennung,
        ab: usize,
    ) -> std::time::Duration {
        let beginn = std::time::Instant::now();
        for nummer in ab..ab + 10_000 {
            let schluessel = schluessel_mit_praefix(nummer);
            let summe = pruefsumme(&schluessel);
            let pruefung = speicher.pruefen(wer, &schluessel, &allgemein(), summe, t(0));
            assert_eq!(pruefung, Pruefung::Neu);
            let _ = speicher.merken(wer, &schluessel, &allgemein(), summe, t(0), t(0));
        }
        beginn.elapsed()
    }

    /// R00025 „Laufzeit“ (Z16, Angriffsmatrix Zeile 13, Regel E1.3): `pruefen` + `merken` gegen einen vollen
    /// Speicher (36.000 Einträge, Schlüssel mit gemeinsamem Präfix) dauern höchstens das Dreifache wie gegen einen
    /// leeren — Median je Paar über 7 Runden, leer und voll abwechselnd. Lesart: „36.000 Einträge einer Kennung“
    /// verbietet die Grenze je Kennung (640); voll heißt hier 36.000 Kennungen je ein Eintrag, die messende Kennung
    /// trifft so bei jedem Merken die Gesamtgrenze (Auswahl der Kennung mit den meisten) und schlägt unter 36.000
    /// Einträgen nach.
    #[test]
    fn pruefen_und_merken_dauern_voll_hoechstens_dreimal_so_lange_wie_leer() {
        let messend = kennung("messend@127.0.0.1");
        let mut voll = Wiederholung::default();
        for nummer in 0..HOECHSTENS_GESAMT {
            let wer = kennung(&format!("k{nummer}@127.0.0.2"));
            let schluessel = schluessel_mit_praefix(nummer);
            let _ = voll.merken(&wer, &schluessel, &allgemein(), 1, t(0), t(0));
        }
        let mut verhaeltnisse: Vec<f64> = (0..7)
            .map(|runde| {
                let mut leer = Wiederholung::default();
                let ab = 1_000_000 + runde * 10_000;
                let dauer_leer = zehntausend_paare(&mut leer, &messend, ab);
                let dauer_voll = zehntausend_paare(&mut voll, &messend, ab);
                dauer_voll.as_secs_f64() / dauer_leer.as_secs_f64()
            })
            .collect();
        verhaeltnisse.sort_by(f64::total_cmp);

        let median = verhaeltnisse[3];
        assert!(
            median <= 3.0,
            "voll ÷ leer = {median:.2} (Runden {verhaeltnisse:.2?})"
        );
    }

    /// R00025 „Strukturen begrenzt“ (10, Regel E1.4): 100.000 Kennungen je ein Schlüssel, die Uhr rückt um
    /// 600.001 ms vor, eine Sendung → genau ein Eintrag, und keine Struktur kennt mehr Kennungen als die mit
    /// Einträgen.
    #[test]
    fn nach_kennungswechsel_und_ablauf_bleibt_keine_leere_kennung_stehen() {
        let mut speicher = Wiederholung::default();
        for nummer in 0..100_000 {
            let wer = kennung(&format!("k{nummer}@127.0.0.2"));
            let _ = speicher.merken(&wer, "x", &allgemein(), 1, t(0), t(0));
        }
        let _ = speicher.merken(
            &kennung("letzte@127.0.0.1"),
            "x",
            &allgemein(),
            1,
            t(1),
            t(600_001),
        );

        let groessen = speicher.groessen();
        assert_eq!(groessen.eintraege, 1, "{groessen:?}");
        assert_eq!(groessen.groesste_kennungszahl(), 1, "{groessen:?}");
    }

    /// R00025 „Strukturen begrenzt“ (11, Regel E1.4; je Kennung 3, gesamt 5): eine Kennung merkt 1.000.000
    /// Schlüssel → jede interne Struktur hält höchstens 5 + konstant viele Elemente.
    #[test]
    fn eine_kennung_mit_einer_million_schluesseln_haelt_jede_struktur_klein() {
        let wer = kennung("fluter@127.0.0.1");
        let mut speicher = klein(3, 5);
        for nummer in 0..1_000_000 {
            let _ = speicher.merken(&wer, &format!("s{nummer}"), &allgemein(), 1, t(0), t(0));
        }

        let groessen = speicher.groessen();
        assert_eq!(groessen.eintraege, 3, "{groessen:?}");
        assert!(groessen.groesste_elementzahl() <= 5 + 1, "{groessen:?}");
    }
}
