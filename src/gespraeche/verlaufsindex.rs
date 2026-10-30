//! Verlaufsindex (A00009, ADR-0006): je Tag der Rohtext der Datei und die Anfänge seiner Sätze.
//!
//! Der Rohtext sind die Bytes der Tagesdatei, auch ungültiges UTF-8 darin; er bleibt byte-gleich mit ihr,
//! solange nur nach erfolgreichem Schreiben angehängt wird.

use super::abfrage::{Abfrage, Abschnitt};
use super::logformat;
use super::zitat::{self, Fund, Fundaenderung, Zitat};
use crate::uhr::Zeit;
use chrono::NaiveDate;
use std::collections::BTreeMap;

/// Der Verlauf eines Gesprächs.
#[derive(Debug, Default)]
pub struct Verlauf {
    tage: BTreeMap<NaiveDate, Tag>,
    /// Fassung je geänderter Nachricht (ADR-0048 Regel 3), über die Nachrichtenzeit als Zeitpunkt.
    fassungen: BTreeMap<Zeit, Fassung>,
}

/// Die aktuelle Fassung einer geänderten Nachricht: Zeit des letzten gültigen Änderungssatzes und Zustand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fassung {
    pub geaendert: Zeit,
    pub zustand: Fassungszustand,
}

/// Bearbeitet mit dem Ort des Änderungssatzes (Tag, Byte-Anfang im Rohtext) oder gelöscht (endgültig).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fassungszustand {
    Bearbeitet { tag: NaiveDate, anfang: usize },
    Geloescht,
}

/// Was `fassungen_bilden` übergangen hat: Waisen still gezählt, Defekte mit Tag und Zeile (1-basiert, Kopfzeile).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Fassungsbericht {
    pub waisen: u64,
    pub defekte: Vec<Aenderungsbefund>,
}

/// Ein übergangener Änderungssatz (ADR-0048 Regel 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Aenderungsbefund {
    pub tag: NaiveDate,
    pub zeile: usize,
    pub grund: Aenderungsgrund,
}

/// Warum ein Änderungssatz übergangen wurde.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Aenderungsgrund {
    /// Schon der Satz ist defekt (zwei Bezüge, Bezugszeit, Reihenfolge, Anhang).
    Satz(logformat::Aenderungsdefekt),
    /// Die Kennung des Änderungssatzes ist nicht der Absender der Nachricht.
    Fremd,
    /// `bearbeitet` nach `geloescht` derselben Nachricht.
    NachGeloescht,
}

impl Verlauf {
    /// Bildet die Fassungen aus allen Änderungssätzen, nachdem alle Tage aufgenommen sind: in Zeitfolge über alle
    /// Tage (ADR-0048 Regel 3, Pre-Mortem 1); Bearbeitet ersetzt Bearbeitet, Geloescht ist endgültig.
    pub fn fassungen_bilden(&mut self) -> Fassungsbericht {
        let mut bericht = Fassungsbericht::default();
        for (datum, tag) in &self.tage {
            for (anfang, satz) in tag.aenderungssaetze() {
                let befund = |grund| Aenderungsbefund {
                    tag: *datum,
                    zeile: zeile_von(&tag.rohtext, anfang),
                    grund,
                };
                let (ziel, bezug) = match logformat::satzart(satz) {
                    logformat::Satzart::Aenderung { ziel, bezug } => (ziel, bezug),
                    logformat::Satzart::DefekteAenderung(defekt) => {
                        bericht.defekte.push(befund(Aenderungsgrund::Satz(defekt)));
                        continue;
                    }
                    logformat::Satzart::Nachricht => continue,
                };
                let Some(nachricht) = nachricht_finden(&self.tage, &ziel) else {
                    bericht.waisen += 1;
                    continue;
                };
                let (Some((geaendert, von)), Some((zeit, absender))) =
                    (logformat::kopf(satz), logformat::kopf(nachricht))
                else {
                    continue;
                };
                if von != absender {
                    bericht.defekte.push(befund(Aenderungsgrund::Fremd));
                    continue;
                }
                let bisher = self.fassungen.get(&zeit).map(|fassung| fassung.zustand);
                let zustand = match (bisher, bezug) {
                    (Some(Fassungszustand::Geloescht), logformat::Bezug::Bearbeitet) => {
                        bericht.defekte.push(befund(Aenderungsgrund::NachGeloescht));
                        continue;
                    }
                    (Some(Fassungszustand::Geloescht), logformat::Bezug::Geloescht) => continue,
                    (_, logformat::Bezug::Bearbeitet) => Fassungszustand::Bearbeitet {
                        tag: *datum,
                        anfang,
                    },
                    (_, logformat::Bezug::Geloescht) => Fassungszustand::Geloescht,
                };
                self.fassungen.insert(zeit, Fassung { geaendert, zustand });
            }
        }
        bericht
    }

    /// Die Nachricht mit genau dieser Zeit (als Zeitpunkt) samt Fassung, wenn sie im Index steht.
    pub fn nachricht(&self, zeit: &Zeit) -> Option<Kopie> {
        let satz = nachricht_finden(&self.tage, zeit)?;
        let eigene = logformat::kopfzeit(satz)?;
        let fassung = self.fassung(&eigene);
        let aenderung = fassung.map(|fassung| Kopierte {
            geaendert: fassung.geaendert,
            satz: self.aenderungssatz(&fassung),
        });
        Some(Kopie {
            satz: satz.to_vec(),
            aenderung,
            zitat: self.zitat_der_antwort(satz, fassung),
        })
    }

    /// Das Zitat einer Antwort, aufgelöst über ihr Original im eigenen Index (ADR-0053 Regel 3); ohne Bezug, mit
    /// defektem Bezug oder als gelöschte Antwort (der Bezug ist Inhalt) keines.
    fn zitat_der_antwort(&self, satz: &[u8], fassung: Option<Fassung>) -> Option<Box<Zitat>> {
        if fassung.is_some_and(|fassung| fassung.zustand == Fassungszustand::Geloescht) {
            return None;
        }
        let bezug = logformat::antwortbezug(satz).ok().flatten()?;
        Some(Box::new(zitat::zitat(&bezug, self.fund(&bezug.zeit))))
    }

    /// Das Original mit genau dieser Zeit (als Zeitpunkt), geliehen statt kopiert, samt Fassung (ADR-0053 Regel 3):
    /// das Zitat liest daraus nur, bis sein Textanfang feststeht.
    pub fn fund(&self, zeit: &Zeit) -> Option<Fund<'_>> {
        let satz = nachricht_finden(&self.tage, zeit)?;
        let eigene = logformat::kopfzeit(satz)?;
        let aenderung = self.fassung(&eigene).map(|fassung| Fundaenderung {
            geaendert: fassung.geaendert,
            satz: self
                .aenderungsspanne(&fassung)
                .map(|(tag, spanne)| &tag.rohtext[spanne]),
        });
        Some(Fund { satz, aenderung })
    }

    /// Die Zeiten der gelöschten Nachrichten, aufsteigend.
    pub fn geloeschte(&self) -> impl Iterator<Item = Zeit> + '_ {
        self.fassungen
            .iter()
            .filter(|(_, fassung)| fassung.zustand == Fassungszustand::Geloescht)
            .map(|(zeit, _)| *zeit)
    }

    /// Die Fassung der Nachricht mit dieser Zeit, wenn sie geändert ist.
    pub fn fassung(&self, zeit: &Zeit) -> Option<Fassung> {
        self.fassungen.get(zeit).copied()
    }

    /// Der Änderungssatz einer bearbeiteten Fassung (Kopie); für Geloescht keiner.
    pub fn aenderungssatz(&self, fassung: &Fassung) -> Option<Vec<u8>> {
        self.aenderungsspanne(fassung)
            .map(|(tag, spanne)| tag.rohtext[spanne].to_vec())
    }

    /// Hängt einen Bearbeitungssatz an und setzt die Fassung der Nachricht `ziel` (nach erfolgreichem Schreiben).
    pub fn bearbeitung_anhaengen(
        &mut self,
        tag: NaiveDate,
        satz: &str,
        zeilenende_ergaenzt: bool,
        ziel: Zeit,
        geaendert: Zeit,
    ) {
        let anfang = self.anhaengen(tag, satz, zeilenende_ergaenzt);
        self.fassungen.insert(
            ziel,
            Fassung {
                geaendert,
                zustand: Fassungszustand::Bearbeitet { tag, anfang },
            },
        );
    }

    /// Hängt einen Löschsatz an und setzt die Fassung der Nachricht `ziel` auf `Geloescht` (nach erfolgreichem
    /// Schreiben; endgültig).
    pub fn loeschung_anhaengen(
        &mut self,
        tag: NaiveDate,
        satz: &str,
        zeilenende_ergaenzt: bool,
        ziel: Zeit,
        geaendert: Zeit,
    ) {
        self.anhaengen(tag, satz, zeilenende_ergaenzt);
        self.fassungen.insert(
            ziel,
            Fassung {
                geaendert,
                zustand: Fassungszustand::Geloescht,
            },
        );
    }
}

#[derive(Debug, Default)]
struct Tag {
    rohtext: Vec<u8>,
    /// Anfänge aller gültigen Sätze — Nachrichten und Änderungssätze; begrenzen die Spanne einer Nachricht.
    satzanfaenge: Vec<usize>,
    /// Anfänge der Sätze, die Nachrichten sind (ADR-0048 Regel 3): nur über sie zählt, sucht und kopiert der Block.
    nachrichten: Vec<usize>,
    /// Ob ein Satz des Tages eine Metazeile `+ anhang:` trägt — beim Laden mitgezählt, nicht ein zweites Mal gesucht.
    traegt_anhang: bool,
}

impl Verlauf {
    /// Nimmt einen geladenen Tag auf: Rohtext und die Byte-Anfänge seiner gültigen Sätze (Tests; das Laden
    /// kennt das Merkmal schon).
    #[cfg(test)]
    pub fn tag_aufnehmen(&mut self, tag: NaiveDate, rohtext: Vec<u8>, satzanfaenge: Vec<usize>) {
        let traegt_anhang = satzanfaenge
            .iter()
            .any(|anfang| satz_traegt_anhang(&rohtext[*anfang..]));
        self.tag_mit_anhangflagge(tag, rohtext, satzanfaenge, traegt_anhang);
    }

    /// Wie `tag_aufnehmen`, mit dem beim Lesen schon bestimmten Merkmal, ob der Tag Metazeilen `+ anhang:` trägt.
    #[cfg(test)]
    pub fn tag_mit_anhangflagge(
        &mut self,
        tag: NaiveDate,
        rohtext: Vec<u8>,
        satzanfaenge: Vec<usize>,
        traegt_anhang: bool,
    ) {
        let nachrichten = satzanfaenge
            .iter()
            .enumerate()
            .filter(|(index, anfang)| {
                let ende = satzanfaenge
                    .get(index + 1)
                    .copied()
                    .unwrap_or(rohtext.len());
                ist_nachricht(&rohtext[**anfang..ende])
            })
            .map(|(_, anfang)| *anfang)
            .collect();
        self.tag_mit_nachrichten(tag, rohtext, satzanfaenge, nachrichten, traegt_anhang);
    }

    /// Nimmt einen Tag auf, dessen Sätze das Laden schon gedeutet hat (ein Durchgang über die Metazeilen,
    /// ADR-0053 Regel 1): Anfänge aller Sätze, Anfänge der Nachrichten darunter, Merkmal `+ anhang:`.
    pub fn tag_mit_nachrichten(
        &mut self,
        tag: NaiveDate,
        rohtext: Vec<u8>,
        satzanfaenge: Vec<usize>,
        nachrichten: Vec<usize>,
        traegt_anhang: bool,
    ) {
        self.tage.insert(
            tag,
            Tag {
                rohtext,
                satzanfaenge,
                nachrichten,
                traegt_anhang,
            },
        );
    }

    /// Hängt einen geschriebenen Satz an den Tag an (neuer Tag beginnt leer); mit `zeilenende_ergaenzt`
    /// steht davor das `\n`, das die Logablage ergänzt hat. Liefert den Anfang des Satzes im Rohtext.
    pub fn anhaengen(&mut self, tag: NaiveDate, satz: &str, zeilenende_ergaenzt: bool) -> usize {
        let tag = self.tage.entry(tag).or_default();
        if zeilenende_ergaenzt {
            tag.rohtext.push(b'\n');
        }
        let anfang = tag.rohtext.len();
        tag.satzanfaenge.push(anfang);
        if ist_nachricht(satz.as_bytes()) {
            tag.nachrichten.push(anfang);
        }
        tag.traegt_anhang |= satz_traegt_anhang(satz.as_bytes());
        tag.rohtext.extend_from_slice(satz.as_bytes());
        anfang
    }

    /// Die Zeit der jüngsten Nachricht im Index (aus der Kopfzeile ihres Satzes); ohne Nachricht keine.
    pub fn letzte_nachrichtenzeit(&self) -> Option<Zeit> {
        self.tage.values().rev().find_map(|tag| {
            let anfang = tag.nachrichten.last()?;
            logformat::kopfzeit(&tag.rohtext[*anfang..])
        })
    }

    /// Wie viele Nachrichten der Index hält — Änderungssätze zählen nicht (ADR-0048 Regel 1).
    pub fn nachrichten(&self) -> u64 {
        self.tage
            .values()
            .map(|tag| tag.nachrichten.len() as u64)
            .sum()
    }

    /// Tage mit ihrem Rohtext, aufsteigend (Laden der Anhänge); ein Tag, in dem kein Satz eine Metazeile
    /// `+ anhang:` trägt, liefert einen leeren Rohtext — er wird kein zweites Mal zerlegt (Ladezeit QA5). Geprüft werden
    /// nur die Zeilen `+ …` direkt nach jedem Satzkopf, und zwar schon beim Lesen der Tagesdatei.
    pub fn tage_roh(&self) -> impl Iterator<Item = (NaiveDate, &[u8])> {
        self.tage.iter().map(|(tag, inhalt)| {
            let rohtext: &[u8] = if inhalt.traegt_anhang {
                &inhalt.rohtext
            } else {
                &[]
            };
            (*tag, rohtext)
        })
    }

    pub fn bytes(&self) -> u64 {
        self.tage.values().map(|tag| tag.rohtext.len() as u64).sum()
    }

    /// Nimmt den Tag aus dem Index (ADR-0006: Eintrag fallen lassen, kein Umbau) und nennt seine Nachrichten und
    /// Bytes; stand er nicht darin, nichts.
    /// Die Fassungen der Nachrichten des Tages gehen mit (ADR-0048 Regel 2).
    pub fn tag_entfernen(&mut self, datum: NaiveDate) -> Option<(usize, u64)> {
        let tag = self.tage.remove(&datum)?;
        let kopfzeit = |anfang: Option<&usize>| {
            anfang.and_then(|anfang| logformat::kopfzeit(&tag.rohtext[*anfang..]))
        };
        if let (Some(erste), Some(letzte)) = (
            kopfzeit(tag.nachrichten.first()),
            kopfzeit(tag.nachrichten.last()),
        ) && erste <= letzte
        {
            let weg: Vec<Zeit> = self
                .fassungen
                .range(erste..=letzte)
                .map(|(zeit, _)| *zeit)
                .collect();
            for zeit in weg {
                self.fassungen.remove(&zeit);
            }
        }
        // Eine Bearbeitung, deren Satz mit diesem Tag geht, gilt nicht mehr — wie nach einem Neustart (Code-Review #48
        // A1: sonst sähe die Nachricht gelöscht aus, wenn ihr eigener Tag beim Entfernen scheiterte).
        self.fassungen.retain(|_, fassung| {
            !matches!(fassung.zustand, Fassungszustand::Bearbeitet { tag: ort, .. } if ort == datum)
        });
        Some((tag.nachrichten.len(), tag.rohtext.len() as u64))
    }

    /// Der älteste andere Tag im Index, auf dessen Nachricht ein Änderungssatz des Tages `datum` zielt; keiner, wenn
    /// der Tag nicht im Index steht oder seine Änderungssätze nur Nachrichten des eigenen Tages oder entfallener Tage
    /// treffen (ADR-0048 Ergänzung „Karte #73“ K6).
    pub fn tag_haelt_aenderungen_fuer_bestehende_tage(
        &self,
        datum: NaiveDate,
    ) -> Option<NaiveDate> {
        self.tage
            .get(&datum)?
            .aenderungssaetze()
            .filter_map(|(_, satz)| match logformat::satzart(satz) {
                logformat::Satzart::Aenderung { ziel, .. } => tag_der_nachricht(&self.tage, &ziel),
                logformat::Satzart::DefekteAenderung(_) | logformat::Satzart::Nachricht => None,
            })
            .filter(|ziel_tag| *ziel_tag != datum)
            .min()
    }

    /// Wie viele Nachrichten die Tage echt vor `grenze` im Index haben.
    pub fn saetze_vor(&self, grenze: NaiveDate) -> usize {
        self.tage
            .range(..grenze)
            .map(|(_, tag)| tag.nachrichten.len())
            .sum()
    }
}

/// Ob ein Satz eine Nachricht ist — ein Satz mit Bezugs-Metazeile ist es nie, auch defekt nicht (ADR-0048 Regel 1).
fn ist_nachricht(satz: &[u8]) -> bool {
    logformat::satzart(satz) == logformat::Satzart::Nachricht
}

/// Ob ein Satz (ab seinem Kopf) unter den Metazeilen `+ …` eine `+ anhang:` trägt.
fn satz_traegt_anhang(satz: &[u8]) -> bool {
    satz.split(|byte| *byte == b'\n')
        .skip(1)
        .take_while(|zeile| zeile.starts_with(b"+ "))
        .any(|zeile| zeile.starts_with(b"+ anhang:"))
}

/// Ein Block des Verlaufs: Kopien der Nachrichten, älteste zuerst, und ob es davor bzw. danach noch welche gibt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub saetze: Vec<Kopie>,
    pub aelter: bool,
    pub neuer: bool,
}

// Zähler der Probe „Arbeit ≤ k × Buchung“ (R00049, Karte #17): die Bytes, die `Verlauf::kopie` auf diesem Thread
// kopiert hat — am einzigen Kopierort, unabhängig davon, was das Nachholen bucht.
#[cfg(test)]
thread_local! {
    pub(super) static KOPIERT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

impl Verlauf {
    /// Höchstens `n` Sätze des Abschnitts über beliebig viele Tage, älteste zuerst (ADR-0025 Regeln 2–4), und
    /// höchstens `budget` Satzbytes (ADR-0039 Regel 2): in Cursor-Richtung Satz für Satz, solange die Summe der
    /// Satzlängen das Budget nicht übersteigt; der erste Satz immer. Abgeschnitten wird am cursorfernen Ende,
    /// kopiert nur, was genommen ist. Die Arbeit hängt an `n` und an der Zahl der Tage im Abschnitt, nie an der
    /// Länge des Verlaufs.
    pub fn block(&self, abfrage: &Abfrage, budget: usize) -> Block {
        let sammler = Sammler::neu(abfrage.n.wert(), budget);
        match abfrage.abschnitt {
            Abschnitt::Neueste => self.rueckwaerts(None, sammler),
            Abschnitt::Vor(zeit) => self.rueckwaerts(Some(zeit), sammler),
            Abschnitt::Nach(zeit) => self.vorwaerts(zeit, sammler),
        }
    }

    /// Die jüngsten Sätze echt vor `bis` (ohne `bis`: die neuesten), vom jüngsten Tag zurück.
    fn rueckwaerts(&self, bis: Option<Zeit>, mut sammler: Sammler) -> Block {
        let letzter_tag = bis.map_or(NaiveDate::MAX, |zeit| tag_mit_abstand(zeit, 2));
        let mut gesammelt = Vec::new();
        let mut aelter = false;
        for tag in self.tage.range(..=letzter_tag).rev().map(|(_, tag)| tag) {
            let mut index = match bis {
                Some(zeit) => grenze_im_tag(&tag.rohtext, &tag.nachrichten, &zeit, Seite::Vor),
                None => tag.nachrichten.len(),
            };
            while index > 0 {
                let (laenge, zitat) = self.lieferung(tag, index - 1);
                if !sammler.nimmt(laenge) {
                    break;
                }
                index -= 1;
                gesammelt.push(self.kopie(tag, index, zitat));
            }
            if index > 0 {
                aelter = true;
                break;
            }
        }
        gesammelt.reverse();
        let neuer = bis.is_some_and(|zeit| {
            self.juengste_kopfzeit()
                .is_some_and(|juengste| juengste >= zeit)
        });
        Block {
            saetze: gesammelt,
            aelter,
            neuer,
        }
    }

    /// Die ältesten Sätze echt nach `ab`, vom Tag der Zeit vorwärts.
    fn vorwaerts(&self, ab: Zeit, mut sammler: Sammler) -> Block {
        let erster_tag = tag_mit_abstand(ab, -2);
        let mut gesammelt = Vec::new();
        let mut neuer = false;
        for tag in self.tage.range(erster_tag..).map(|(_, tag)| tag) {
            let mut index = grenze_im_tag(&tag.rohtext, &tag.nachrichten, &ab, Seite::Nach);
            let laenge = tag.nachrichten.len();
            while index < laenge {
                let (bytes, zitat) = self.lieferung(tag, index);
                if !sammler.nimmt(bytes) {
                    break;
                }
                gesammelt.push(self.kopie(tag, index, zitat));
                index += 1;
            }
            if index < laenge {
                neuer = true;
                break;
            }
        }
        let aelter = self
            .aelteste_kopfzeit()
            .is_some_and(|aelteste| aelteste <= ab);
        Block {
            saetze: gesammelt,
            aelter,
            neuer,
        }
    }

    /// Wie viele Bytes die Nachricht `index` im Budget zählt (ADR-0048 Regel 3, ADR-0053 Regel 3) und ihr Zitat:
    /// bearbeitet der Änderungssatz, gelöscht nur die Kopfzeile (Text, Anhänge und Bezug werden nicht geliefert), sonst
    /// die Nachricht — eine Antwort zusätzlich mit den Bytes ihres Zitats.
    fn lieferung(&self, tag: &Tag, index: usize) -> (usize, Option<Box<Zitat>>) {
        let fassung = self.fassung_im_tag(tag, index);
        let zitat = self.zitat_der_antwort(&tag.rohtext[tag.spanne(index)], fassung);
        let zitatbytes = zitat.as_ref().map_or(0, |zitat| zitat.lieferbytes());
        let laenge = match fassung {
            None => tag.satzlaenge(index),
            Some(fassung) if fassung.zustand == Fassungszustand::Geloescht => tag.kopflaenge(index),
            Some(fassung) => self
                .aenderungsspanne(&fassung)
                .map_or_else(|| tag.satzlaenge(index), |(_, spanne)| spanne.len()),
        };
        (laenge + zitatbytes, zitat)
    }

    /// Kopie der Nachricht `index` mit ihrer Fassung und ihrem Zitat.
    fn kopie(&self, tag: &Tag, index: usize, zitat: Option<Box<Zitat>>) -> Kopie {
        let aenderung = self.fassung_im_tag(tag, index).map(|fassung| Kopierte {
            geaendert: fassung.geaendert,
            satz: self.aenderungssatz(&fassung),
        });
        let kopie = Kopie {
            satz: tag.satz(index),
            aenderung,
            zitat,
        };
        #[cfg(test)]
        KOPIERT.with(|kopiert| kopiert.set(kopiert.get() + kopie.kopierte_bytes()));
        kopie
    }

    /// Zeit und Lesebytes der ersten Nachricht echt nach `ab` — ohne Kopie: was die Trefferregel für sie liest, das
    /// Größere aus Satz und Änderungssatz (auch eine gelöschte liest ihr Original). Das Nachholen bemisst daran,
    /// was es kopieren darf, bevor es kopiert.
    pub fn naechste_nach(&self, ab: &Zeit) -> Option<(Zeit, usize)> {
        for tag in self
            .tage
            .range(tag_mit_abstand(*ab, -2)..)
            .map(|(_, tag)| tag)
        {
            let index = grenze_im_tag(&tag.rohtext, &tag.nachrichten, ab, Seite::Nach);
            if index < tag.nachrichten.len() {
                let zeit = logformat::kopfzeit(&tag.rohtext[tag.nachrichten[index]..])?;
                let aenderung = self
                    .fassung_im_tag(tag, index)
                    .and_then(|fassung| self.aenderungsspanne(&fassung))
                    .map_or(0, |(_, spanne)| spanne.len());
                return Some((zeit, tag.satzlaenge(index).max(aenderung)));
            }
        }
        None
    }

    /// Die Fassung der Nachricht `index` des Tages — ohne Fassungen im Gespräch ohne Nachschlagen (O(log m) sonst).
    fn fassung_im_tag(&self, tag: &Tag, index: usize) -> Option<Fassung> {
        if self.fassungen.is_empty() {
            return None;
        }
        let zeit = logformat::kopfzeit(&tag.rohtext[tag.nachrichten[index]..])?;
        self.fassung(&zeit)
    }

    /// Tag und Spanne des Änderungssatzes einer bearbeiteten Fassung.
    fn aenderungsspanne(&self, fassung: &Fassung) -> Option<(&Tag, std::ops::Range<usize>)> {
        let Fassungszustand::Bearbeitet { tag, anfang } = fassung.zustand else {
            return None;
        };
        let tag = self.tage.get(&tag)?;
        Some((tag, tag.satzspanne(anfang)))
    }

    fn juengste_kopfzeit(&self) -> Option<Zeit> {
        let tag = self
            .tage
            .values()
            .rev()
            .find(|tag| !tag.nachrichten.is_empty())?;
        logformat::kopfzeit(&tag.satz(tag.nachrichten.len() - 1))
    }

    fn aelteste_kopfzeit(&self) -> Option<Zeit> {
        let tag = self.tage.values().find(|tag| !tag.nachrichten.is_empty())?;
        logformat::kopfzeit(&tag.satz(0))
    }
}

/// Eine kopierte Nachricht und, wenn sie geändert ist, ihre Fassung (ADR-0048 Regel 3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kopie {
    pub satz: Vec<u8>,
    pub aenderung: Option<Kopierte>,
    /// Das Zitat einer Antwort, aufgelöst unter der Sperre des Gesprächs (ADR-0053 Regel 3).
    pub zitat: Option<Box<Zitat>>,
}

impl Kopie {
    /// Wie viele Bytes diese Kopie aus dem Verlauf gekopiert hat: Satz und Änderungssatz.
    #[cfg(test)]
    fn kopierte_bytes(&self) -> usize {
        let aenderungssatz = self
            .aenderung
            .as_ref()
            .and_then(|aenderung| aenderung.satz.as_ref())
            .map_or(0, Vec::len);
        self.satz.len() + aenderungssatz
    }
}

/// Die Fassung einer kopierten Nachricht: Zeit der Änderung und der kopierte Bearbeitungssatz, gelöscht keiner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kopierte {
    pub geaendert: Zeit,
    pub satz: Option<Vec<u8>>,
}

impl Tag {
    /// Kopie der Nachricht `index`: von ihrem Anfang bis vor den nächsten Satz gleich welcher Art bzw. bis zum Ende
    /// des Tages.
    fn satz(&self, index: usize) -> Vec<u8> {
        self.rohtext[self.spanne(index)].to_vec()
    }

    /// Länge der Nachricht `index` in Bytes, ohne sie zu kopieren.
    fn satzlaenge(&self, index: usize) -> usize {
        self.spanne(index).len()
    }

    /// Länge der Kopfzeile der Nachricht `index` samt Zeilenende.
    fn kopflaenge(&self, index: usize) -> usize {
        let spanne = self.spanne(index);
        self.rohtext[spanne.clone()]
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(spanne.len(), |ende| ende + 1)
    }

    fn spanne(&self, index: usize) -> std::ops::Range<usize> {
        self.satzspanne(self.nachrichten[index])
    }

    /// Der Satz ab `anfang` bis vor den nächsten Satz gleich welcher Art bzw. bis zum Ende des Tages.
    fn satzspanne(&self, anfang: usize) -> std::ops::Range<usize> {
        let naechster = self.satzanfaenge.partition_point(|satz| *satz <= anfang);
        let ende = self
            .satzanfaenge
            .get(naechster)
            .copied()
            .unwrap_or(self.rohtext.len());
        anfang..ende
    }

    /// Die Sätze, die keine Nachrichten sind, mit ihrem Anfang — in Dateifolge.
    fn aenderungssaetze(&self) -> impl Iterator<Item = (usize, &[u8])> {
        self.satzanfaenge
            .iter()
            .filter(|anfang| self.nachrichten.binary_search(anfang).is_err())
            .map(|anfang| (*anfang, &self.rohtext[self.satzspanne(*anfang)]))
    }
}

/// Die Nachricht mit genau dieser Zeit (als Zeitpunkt), als Satz ab ihrem Kopf; gesucht zuerst im Tag ihres eigenen
/// Datums (dort steht sie, wenn der Versatz der des Servers ist), dann in den Tagen um die Zeit — so kostet das
/// Nachschlagen beim Laden vieler Änderungssätze eine binäre Suche statt bis zu fünf (ADR-0048 QE2).
fn nachricht_finden<'a>(tage: &'a BTreeMap<NaiveDate, Tag>, zeit: &Zeit) -> Option<&'a [u8]> {
    fundort(tage, zeit).map(|(_, satz)| satz)
}

/// Der Tag im Index, in dem die Nachricht mit genau dieser Zeit steht (gesucht wie in `nachricht_finden`).
fn tag_der_nachricht(tage: &BTreeMap<NaiveDate, Tag>, zeit: &Zeit) -> Option<NaiveDate> {
    fundort(tage, zeit).map(|(datum, _)| datum)
}

/// Tag und Satz der Nachricht mit genau dieser Zeit: zuerst im Tag ihres eigenen Datums, dann in den Tagen um die Zeit.
fn fundort<'a>(tage: &'a BTreeMap<NaiveDate, Tag>, zeit: &Zeit) -> Option<(NaiveDate, &'a [u8])> {
    let im_tag = |tag: &'a Tag| {
        let index = grenze_im_tag(&tag.rohtext, &tag.nachrichten, zeit, Seite::Vor);
        let anfang = *tag.nachrichten.get(index)?;
        (logformat::kopfzeit(&tag.rohtext[anfang..]) == Some(*zeit))
            .then(|| &tag.rohtext[tag.spanne(index)])
    };
    let eigenes = zeit.date_naive();
    let im_eigenen = tage
        .get(&eigenes)
        .and_then(im_tag)
        .map(|satz| (eigenes, satz));
    im_eigenen.or_else(|| {
        tage.range(tag_mit_abstand(*zeit, -2)..=tag_mit_abstand(*zeit, 2))
            .filter(|(datum, _)| **datum != eigenes)
            .find_map(|(datum, tag)| im_tag(tag).map(|satz| (*datum, satz)))
    })
}

/// Die Zeile (1-basiert), in der der Satz ab `anfang` beginnt.
fn zeile_von(rohtext: &[u8], anfang: usize) -> usize {
    rohtext[..anfang]
        .iter()
        .filter(|byte| **byte == b'\n')
        .count()
        + 1
}

/// Zählt, was ein Block schon genommen hat, über alle Tage hinweg: höchstens `n` Sätze und höchstens `budget`
/// Satzbytes, der erste Satz immer (ADR-0039 Regel 2).
struct Sammler {
    n: usize,
    budget: usize,
    genommen: usize,
    summe: usize,
}

impl Sammler {
    fn neu(n: usize, budget: usize) -> Sammler {
        Sammler {
            n,
            budget,
            genommen: 0,
            summe: 0,
        }
    }

    /// Nimmt einen Satz dieser Länge, wenn er noch passt.
    fn nimmt(&mut self, satzlaenge: usize) -> bool {
        let summe = self.summe.saturating_add(satzlaenge);
        let passt = self.genommen < self.n && (self.genommen == 0 || summe <= self.budget);
        if passt {
            self.genommen += 1;
            self.summe = summe;
        }
        passt
    }
}

/// Das Datum der Zeit in UTC, um `tage` verschoben: ein Tag der Datei (lokales Datum des Servers) mit größerem
/// Abstand kann keinen Satz auf der anderen Seite der Zeit enthalten — Versätze liegen innerhalb ±14 h.
fn tag_mit_abstand(zeit: Zeit, tage: i64) -> NaiveDate {
    let datum = zeit.naive_utc().date();
    let verschoben = datum.checked_add_signed(chrono::Duration::days(tage));
    match verschoben {
        Some(datum) => datum,
        None if tage < 0 => NaiveDate::MIN,
        None => NaiveDate::MAX,
    }
}

/// Auf welcher Seite einer Zeit die Sätze liegen sollen: echt davor oder echt danach (ADR-0025 Regel 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Seite {
    Vor,
    Nach,
}

/// Index des ersten Satzes im Tag, der nicht mehr echt vor (`Vor`) bzw. der echt nach (`Nach`) der Zeit liegt —
/// binäre Suche über die Kopfzeiten, verglichen als Zeitpunkt (ADR-0025 Regel 4).
/// Ein Kopf ohne lesbare Zeit kommt nicht vor (der Index nimmt nur gültige Sätze auf); er zählt als älter.
fn grenze_im_tag(rohtext: &[u8], satzanfaenge: &[usize], zeit: &Zeit, seite: Seite) -> usize {
    let vergleich = Kopfvergleich::neu(zeit);
    satzanfaenge.partition_point(|&anfang| {
        let Some(ordnung) = rohtext
            .get(anfang..)
            .and_then(|satz| vergleich.kopf_gegen_zeit(satz))
        else {
            return true;
        };
        match seite {
            Seite::Vor => ordnung == std::cmp::Ordering::Less,
            Seite::Nach => ordnung != std::cmp::Ordering::Greater,
        }
    })
}

/// Vergleicht die Kopfzeit eines Satzes mit einer Zeit (ADR-0025 Regel 4: als Zeitpunkt). Steht die Kopfzeit in der
/// Schreibweise des Logs (`JJJJ-MM-TTTHH:MM:SS.mmm±HH:MM`) mit demselben Versatz wie die Zeit, entscheidet der Text —
/// gleich breit und gleich versetzt ist die Textfolge die Zeitfolge —, sonst der geparste Zeitpunkt. So kostet eine
/// binäre Suche keine Zeitparser je Schritt (ADR-0048 QE2: Laden vieler Änderungssätze).
struct Kopfvergleich {
    zeit: Zeit,
    text: String,
}

impl Kopfvergleich {
    fn neu(zeit: &Zeit) -> Kopfvergleich {
        Kopfvergleich {
            zeit: *zeit,
            text: zeit.format("%Y-%m-%dT%H:%M:%S%.3f%:z").to_string(),
        }
    }

    fn kopf_gegen_zeit(&self, satz: &[u8]) -> Option<std::cmp::Ordering> {
        let kopf = satz.strip_prefix(b"@ ")?;
        let text = self.text.as_bytes();
        let breite = text.len();
        let gleich_geschrieben = kopf.len() > breite
            && matches!(kopf[breite], b' ' | b'\n')
            && kopf[10] == b'T'
            && kopf[19] == b'.'
            && kopf[breite - 6..breite] == text[breite - 6..];
        if gleich_geschrieben {
            return Some(kopf[..breite].cmp(text));
        }
        logformat::kopfzeit(satz).map(|kopfzeit| kopfzeit.cmp(&self.zeit))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gespraeche::abfrage::Blockgroesse;

    fn zeit(text: &str) -> Zeit {
        Zeit::parse_from_rfc3339(text).expect("Testzeit")
    }

    /// Rohtext und Satzanfänge eines Tages aus Sätzen mit diesen Zeiten.
    fn tag_mit(zeiten: &[&str]) -> (Vec<u8>, Vec<usize>) {
        let mut rohtext = Vec::new();
        let mut anfaenge = Vec::new();
        for zeit in zeiten {
            anfaenge.push(rohtext.len());
            rohtext.extend_from_slice(format!("@ {zeit} a@127.0.0.2\n  x\n\n").as_bytes());
        }
        (rohtext, anfaenge)
    }

    /// Drei Tage (26., 27., 28.10.) zu je 250 Sätzen im Sekundentakt ab 10:00; Satz `i` des Tages um 10:00 + i s.
    fn drei_tage() -> Verlauf {
        let mut verlauf = Verlauf::default();
        for datum in ["2026-10-26", "2026-10-27", "2026-10-28"] {
            let zeiten: Vec<String> = (0..250).map(|i| satzzeit(datum, i)).collect();
            let zeiten: Vec<&str> = zeiten.iter().map(String::as_str).collect();
            let (rohtext, anfaenge) = tag_mit(&zeiten);
            verlauf.tag_aufnehmen(tag(datum), rohtext, anfaenge);
        }
        verlauf
    }

    fn satzzeit(datum: &str, index: usize) -> String {
        format!("{datum}T10:{:02}:{:02}.000+01:00", index / 60, index % 60)
    }

    fn abfrage(abschnitt: Abschnitt, n: usize) -> Abfrage {
        Abfrage {
            abschnitt,
            n: Blockgroesse::bilden(n).expect("Blockgröße"),
        }
    }

    fn zeiten_im(block: &Block) -> Vec<String> {
        block
            .saetze
            .iter()
            .map(|kopie| {
                logformat::kopfzeit(&kopie.satz)
                    .expect("Kopf")
                    .to_rfc3339_opts(chrono::SecondsFormat::Millis, false)
            })
            .collect()
    }

    fn erwartet(datum: &str, indizes: std::ops::Range<usize>) -> Vec<String> {
        indizes.map(|index| satzzeit(datum, index)).collect()
    }

    #[test]
    fn die_neuesten_sind_die_letzten_des_juengsten_tages_aelteste_zuerst() {
        let block = drei_tage().block_ohne_grenze(&abfrage(Abschnitt::Neueste, 100));

        assert_eq!(zeiten_im(&block), erwartet("2026-10-28", 150..250));
        assert!(block.aelter);
        assert!(!block.neuer);
    }

    #[test]
    fn vor_reicht_lueckenlos_ueber_die_tagesgrenze() {
        let vor = zeit(&satzzeit("2026-10-28", 49));

        let block = drei_tage().block_ohne_grenze(&abfrage(Abschnitt::Vor(vor), 100));

        let mut zeiten = erwartet("2026-10-27", 199..250);
        zeiten.extend(erwartet("2026-10-28", 0..49));
        assert_eq!(zeiten_im(&block), zeiten);
        assert!(block.aelter);
        assert!(block.neuer);
    }

    #[test]
    fn nach_dem_letzten_des_aeltesten_tages_kommt_der_anfang_des_mittleren() {
        let nach = zeit(&satzzeit("2026-10-26", 249));

        let block = drei_tage().block_ohne_grenze(&abfrage(Abschnitt::Nach(nach), 100));

        assert_eq!(zeiten_im(&block), erwartet("2026-10-27", 0..100));
        assert!(block.aelter);
        assert!(block.neuer);
    }

    #[test]
    fn nach_einer_zeit_vor_dem_aeltesten_tag_kommen_die_ersten_n() {
        let block = drei_tage().block_ohne_grenze(&abfrage(
            Abschnitt::Nach(zeit("2026-10-01T00:00:00.000+02:00")),
            7,
        ));

        assert_eq!(zeiten_im(&block), erwartet("2026-10-26", 0..7));
        assert!(!block.aelter);
        assert!(block.neuer);
    }

    #[test]
    fn an_den_raendern_ist_der_block_leer_mit_der_richtigen_seite() {
        let verlauf = drei_tage();

        let vor_dem_ersten = verlauf.block_ohne_grenze(&abfrage(
            Abschnitt::Vor(zeit(&satzzeit("2026-10-26", 0))),
            100,
        ));
        let nach_dem_letzten = verlauf.block_ohne_grenze(&abfrage(
            Abschnitt::Nach(zeit(&satzzeit("2026-10-28", 249))),
            100,
        ));
        let leer = Verlauf::default().block_ohne_grenze(&abfrage(Abschnitt::Neueste, 100));

        assert!(vor_dem_ersten.saetze.is_empty());
        assert_eq!((vor_dem_ersten.aelter, vor_dem_ersten.neuer), (false, true));
        assert!(nach_dem_letzten.saetze.is_empty());
        assert_eq!(
            (nach_dem_letzten.aelter, nach_dem_letzten.neuer),
            (true, false)
        );
        assert!(leer.saetze.is_empty());
        assert_eq!((leer.aelter, leer.neuer), (false, false));
    }

    #[test]
    fn genau_n_bis_zum_rand_meldet_keinen_weiteren() {
        let verlauf = drei_tage();

        let alle_bis_auf_einen = verlauf.block_ohne_grenze(&abfrage(
            Abschnitt::Vor(zeit(&satzzeit("2026-10-26", 200))),
            200,
        ));
        let ab_dem_zweiten = verlauf.block_ohne_grenze(&abfrage(
            Abschnitt::Nach(zeit(&satzzeit("2026-10-28", 49))),
            200,
        ));

        assert_eq!(
            zeiten_im(&alle_bis_auf_einen),
            erwartet("2026-10-26", 0..200)
        );
        assert!(!alle_bis_auf_einen.aelter);
        assert_eq!(zeiten_im(&ab_dem_zweiten), erwartet("2026-10-28", 50..250));
        assert!(!ab_dem_zweiten.neuer);
    }

    #[test]
    fn der_ganze_satz_mit_text_wird_kopiert() {
        let mut verlauf = Verlauf::default();
        verlauf.tag_aufnehmen(
            tag("2026-10-28"),
            b"@ 2026-10-28T10:00:00.000+01:00 a@b\n  eins\n\n@ 2026-10-28T10:00:01.000+01:00 a@b\n  zwei\n\n".to_vec(),
            vec![0, 44],
        );

        let block = verlauf.block_ohne_grenze(&abfrage(Abschnitt::Neueste, 100));

        assert_eq!(
            block
                .saetze
                .into_iter()
                .map(|kopie| kopie.satz)
                .collect::<Vec<_>>(),
            vec![
                b"@ 2026-10-28T10:00:00.000+01:00 a@b\n  eins\n\n".to_vec(),
                b"@ 2026-10-28T10:00:01.000+01:00 a@b\n  zwei\n\n".to_vec()
            ]
        );
    }

    /// R00047 B0706: `fund` liefert das Original geliehen, mit dem Änderungssatz der Fassung; eine unbekannte Zeit,
    /// die Zeit eines Änderungssatzes und eine andere Millisekunde finden nichts.
    #[test]
    fn der_fund_leiht_das_original_und_seinen_aenderungssatz() {
        let original = b"@ 2026-10-28T10:00:00.000+01:00 a@b\n  eins\n\n".to_vec();
        let mut verlauf = Verlauf::default();
        verlauf.tag_aufnehmen(tag("2026-10-28"), original.clone(), vec![0]);
        let bearbeitung = "@ 2026-10-28T10:00:05.000+01:00 a@b\n+ bearbeitet: 2026-10-28T10:00:00.000+01:00\n  zwei\n\n";
        verlauf.bearbeitung_anhaengen(
            tag("2026-10-28"),
            bearbeitung,
            false,
            zeit("2026-10-28T10:00:00+01:00"),
            zeit("2026-10-28T10:00:05+01:00"),
        );

        let fund = verlauf
            .fund(&zeit("2026-10-28T09:00:00Z"))
            .expect("als Zeitpunkt gefunden");

        assert_eq!(fund.satz, original.as_slice());
        assert_eq!(
            fund.aenderung,
            Some(Fundaenderung {
                geaendert: zeit("2026-10-28T10:00:05+01:00"),
                satz: Some(bearbeitung.as_bytes()),
            })
        );
        for nichts in [
            "2026-10-28T10:00:00.001+01:00",
            "2026-10-28T10:00:05.000+01:00",
            "2026-10-27T10:00:00.000+01:00",
        ] {
            assert_eq!(verlauf.fund(&zeit(nichts)), None, "{nichts}");
        }
    }

    /// R00047 B0709 (ADR-0053 Regel 3, ADR-0039): das Byte-Budget zählt die Bytes des Zitats mit — reicht es nur für
    /// zwei Antworten ohne ihre Zitate, kommt eine; jede gelieferte Antwort trägt ihr Zitat.
    #[test]
    fn das_byte_budget_zaehlt_das_zitat_einer_antwort_mit() {
        let original = "@ 2026-10-28T10:00:00.000+01:00 a@b\n  Frage?\n\n";
        let antwort = |sekunde: u32| {
            format!(
                "@ 2026-10-28T10:00:0{sekunde}.000+01:00 c@d\n+ antwort_auf: 2026-10-28T10:00:00.000+01:00 a@b\n  Antwort {sekunde}\n\n"
            )
        };
        let antworten = [antwort(1), antwort(2), antwort(3)];
        let rohtext = format!("{original}{}{}{}", antworten[0], antworten[1], antworten[2]);
        let mut anfaenge = vec![0];
        for vorher in [original.len(), antworten[0].len(), antworten[1].len()] {
            anfaenge.push(anfaenge.last().expect("Anfang") + vorher);
        }
        let mut verlauf = Verlauf::default();
        verlauf.tag_aufnehmen(tag("2026-10-28"), rohtext.into_bytes(), anfaenge);
        let bezug = logformat::antwortbezug(antworten[0].as_bytes())
            .expect("gültig")
            .expect("Bezug");
        let zitatbytes = zitat::zitat(&bezug, verlauf.fund(&bezug.zeit)).lieferbytes();
        let budget = antworten[1].len() + antworten[2].len() + 2 * zitatbytes - 1;

        let block = verlauf.block(&abfrage(Abschnitt::Neueste, 100), budget);

        assert_eq!(block.saetze.len(), 1);
        assert!(block.aelter);
        let zitat = block.saetze[0].zitat.as_ref().expect("Zitat");
        assert_eq!(
            (zitat.zustand, zitat.textanfang.as_str()),
            (zitat::Zitatzustand::Da, "Frage?")
        );
        let alle = verlauf.block(&abfrage(Abschnitt::Neueste, 100), usize::MAX);
        assert_eq!(
            alle.saetze
                .iter()
                .map(|kopie| kopie.zitat.is_some())
                .collect::<Vec<_>>(),
            [false, true, true, true]
        );
    }

    #[test]
    fn die_grenze_im_tag_ist_exklusiv_in_beide_richtungen() {
        let (rohtext, anfaenge) = tag_mit(&[
            "2026-10-28T10:00:00.000+01:00",
            "2026-10-28T10:00:02.000+01:00",
        ]);
        let grenze =
            |zeitpunkt: &str, seite| grenze_im_tag(&rohtext, &anfaenge, &zeit(zeitpunkt), seite);

        assert_eq!(grenze("2026-10-28T10:00:02.000+01:00", Seite::Vor), 1);
        assert_eq!(grenze("2026-10-28T10:00:00.000+01:00", Seite::Vor), 0);
        assert_eq!(grenze("2026-10-28T10:00:01.000+01:00", Seite::Vor), 1);
        assert_eq!(grenze("2026-10-28T11:00:00.000+01:00", Seite::Vor), 2);
        assert_eq!(grenze("2026-10-28T10:00:01.000+01:00", Seite::Nach), 1);
        assert_eq!(grenze("2026-10-28T10:00:00.000+01:00", Seite::Nach), 1);
        assert_eq!(grenze("2026-10-28T10:00:02.000+01:00", Seite::Nach), 2);
        assert_eq!(grenze("2026-10-28T09:00:00.000+01:00", Seite::Nach), 0);
    }

    #[test]
    fn die_grenze_vergleicht_zeitpunkte_nicht_zeichenketten() {
        let (rohtext, anfaenge) = tag_mit(&["2026-10-25T02:45:00.000+02:00"]);

        let vor = grenze_im_tag(
            &rohtext,
            &anfaenge,
            &zeit("2026-10-25T02:30:00.000+01:00"),
            Seite::Vor,
        );

        assert_eq!(
            vor, 1,
            "02:45+02:00 (00:45 UTC) liegt vor 02:30+01:00 (01:30 UTC)"
        );
    }

    /// QE2: der Textvergleich der Kopfzeiten entscheidet wie der Zeitpunkt — für Kopfzeiten mit demselben und anderem
    /// Versatz, ohne Millisekunden, in UTC und über die Sommerzeitgrenze, je Zeit eine Millisekunde davor, gleich und
    /// danach in zwei Versätzen (Mutante „Versatz nicht verglichen“ rot).
    #[test]
    fn der_textvergleich_der_kopfzeiten_entscheidet_wie_der_zeitpunkt() {
        let kopfzeiten = [
            "2026-10-25T01:59:59.999+02:00",
            "2026-10-25T02:00:00+02:00",
            "2026-10-25T01:00:00.500Z",
            "2026-10-25T02:30:00.000+01:00",
            "2026-10-25T02:30:00.001+01:00",
            "2026-10-25T03:00:00.000+01:00",
        ];
        let (rohtext, anfaenge) = tag_mit(&kopfzeiten);
        let nach_zeitpunkt = |ziel: &Zeit, seite: Seite| {
            anfaenge.partition_point(|&anfang| {
                let kopfzeit = logformat::kopfzeit(&rohtext[anfang..]).expect("Kopf");
                match seite {
                    Seite::Vor => kopfzeit < *ziel,
                    Seite::Nach => kopfzeit <= *ziel,
                }
            })
        };
        for kopfzeit in kopfzeiten {
            let mitte = zeit(kopfzeit);
            for versatz in [3600, 7200] {
                let versetzt = chrono::FixedOffset::east_opt(versatz).expect("Versatz");
                for abstand in [-1, 0, 1] {
                    let ziel =
                        (mitte + chrono::Duration::milliseconds(abstand)).with_timezone(&versetzt);
                    for seite in [Seite::Vor, Seite::Nach] {
                        assert_eq!(
                            grenze_im_tag(&rohtext, &anfaenge, &ziel, seite),
                            nach_zeitpunkt(&ziel, seite),
                            "{ziel} {seite:?}"
                        );
                    }
                }
            }
        }
    }

    fn tag(text: &str) -> NaiveDate {
        NaiveDate::parse_from_str(text, "%Y-%m-%d").expect("Testtag")
    }

    impl Verlauf {
        /// Die Tests aus R00007 fragen nach `n` allein — ein Budget, das nie greift.
        fn block_ohne_grenze(&self, abfrage: &Abfrage) -> Block {
            self.block(abfrage, usize::MAX)
        }
    }

    // K1 (R00013, ADR-0039 Regel 2): Budget `B` als Parameter, Sätze bekannter Länge `L` (alle gleich lang).

    const SATZ: usize = 49;

    /// Ein Verlauf aus `je_tag` Sätzen je Tag ab 10:00 im Sekundentakt; jeder Satz ist genau `SATZ` Bytes lang.
    fn verlauf_aus(je_tag: &[(&str, usize)]) -> Verlauf {
        let mut verlauf = Verlauf::default();
        for (datum, anzahl) in je_tag {
            let zeiten: Vec<String> = (0..*anzahl).map(|i| satzzeit(datum, i)).collect();
            let zeiten: Vec<&str> = zeiten.iter().map(String::as_str).collect();
            let (rohtext, anfaenge) = tag_mit(&zeiten);
            assert_eq!(rohtext.len(), SATZ * anzahl, "Satzlänge");
            verlauf.tag_aufnehmen(tag(datum), rohtext, anfaenge);
        }
        verlauf
    }

    const NEUESTE: Abschnitt = Abschnitt::Neueste;

    fn vor_allem() -> Abschnitt {
        Abschnitt::Vor(zeit("2026-10-28T23:00:00.000+01:00"))
    }

    fn nach_allem() -> Abschnitt {
        Abschnitt::Nach(zeit("2026-10-01T00:00:00.000+02:00"))
    }

    fn vier_saetze() -> Verlauf {
        verlauf_aus(&[("2026-10-28", 4)])
    }

    #[test]
    fn k1a_summe_genau_budget_nimmt_alle_vier_ohne_flag_zum_fernen_ende() {
        let budget = 4 * SATZ;
        for abschnitt in [NEUESTE, vor_allem()] {
            let block = vier_saetze().block(&abfrage(abschnitt, 200), budget);
            assert_eq!(
                zeiten_im(&block),
                erwartet("2026-10-28", 0..4),
                "{abschnitt:?}"
            );
            assert!(!block.aelter, "{abschnitt:?}");
        }
        let block = vier_saetze().block(&abfrage(nach_allem(), 200), budget);
        assert_eq!(zeiten_im(&block), erwartet("2026-10-28", 0..4));
        assert!(!block.neuer);
    }

    #[test]
    fn k1b_summe_budget_plus_eins_nimmt_die_drei_cursornahen_mit_flag_zum_fernen_ende() {
        let budget = 4 * SATZ - 1;
        for abschnitt in [NEUESTE, vor_allem()] {
            let block = vier_saetze().block(&abfrage(abschnitt, 200), budget);
            assert_eq!(
                zeiten_im(&block),
                erwartet("2026-10-28", 1..4),
                "{abschnitt:?}"
            );
            assert!(block.aelter, "{abschnitt:?}");
            assert!(!block.neuer, "{abschnitt:?}");
        }
        let block = vier_saetze().block(&abfrage(nach_allem(), 200), budget);
        assert_eq!(zeiten_im(&block), erwartet("2026-10-28", 0..3));
        assert!(block.neuer);
        assert!(!block.aelter);
    }

    #[test]
    fn k1c_ein_satz_ueber_dem_budget_wird_allein_genommen() {
        let mut rohtext = Vec::new();
        let mut anfaenge = Vec::new();
        for index in 0..3 {
            anfaenge.push(rohtext.len());
            let kopf = format!("@ {} a@127.0.0.2\n", satzzeit("2026-10-28", index));
            rohtext.extend_from_slice(kopf.as_bytes());
            rohtext.extend_from_slice(format!("  {}\n\n", "x".repeat(300)).as_bytes());
        }
        let laenge = rohtext.len() / 3;
        let budget = laenge - 100;
        let mut verlauf = Verlauf::default();
        verlauf.tag_aufnehmen(tag("2026-10-28"), rohtext, anfaenge);
        for abschnitt in [NEUESTE, vor_allem()] {
            let block = verlauf.block(&abfrage(abschnitt, 200), budget);
            assert_eq!(
                zeiten_im(&block),
                erwartet("2026-10-28", 2..3),
                "{abschnitt:?}"
            );
            assert!(block.aelter, "{abschnitt:?}");
        }
        let block = verlauf.block(&abfrage(nach_allem(), 200), budget);
        assert_eq!(zeiten_im(&block), erwartet("2026-10-28", 0..1));
        assert!(block.neuer);
    }

    #[test]
    fn k1d_weit_unter_dem_budget_begrenzt_n() {
        let verlauf = verlauf_aus(&[("2026-10-28", 10)]);
        for abschnitt in [NEUESTE, vor_allem()] {
            let block = verlauf.block(&abfrage(abschnitt, 2), 100 * SATZ);
            assert_eq!(
                zeiten_im(&block),
                erwartet("2026-10-28", 8..10),
                "{abschnitt:?}"
            );
            assert!(block.aelter, "{abschnitt:?}");
        }
        let block = verlauf.block(&abfrage(nach_allem(), 2), 100 * SATZ);
        assert_eq!(zeiten_im(&block), erwartet("2026-10-28", 0..2));
        assert!(block.neuer);
    }

    #[test]
    fn k1e_die_summe_zaehlt_ueber_die_tagesgrenze_weiter() {
        let budget = 3 * SATZ;
        let rueckwaerts = verlauf_aus(&[("2026-10-27", 3), ("2026-10-28", 2)]);
        for abschnitt in [NEUESTE, vor_allem()] {
            let block = rueckwaerts.block(&abfrage(abschnitt, 200), budget);
            let mut zeiten = erwartet("2026-10-27", 2..3);
            zeiten.extend(erwartet("2026-10-28", 0..2));
            assert_eq!(zeiten_im(&block), zeiten, "{abschnitt:?}");
            assert!(block.aelter, "{abschnitt:?}");
        }
        let vorwaerts = verlauf_aus(&[("2026-10-27", 2), ("2026-10-28", 3)]);
        let block = vorwaerts.block(&abfrage(nach_allem(), 200), budget);
        let mut zeiten = erwartet("2026-10-27", 0..2);
        zeiten.extend(erwartet("2026-10-28", 0..1));
        assert_eq!(zeiten_im(&block), zeiten);
        assert!(block.neuer);
    }

    #[test]
    fn aufgenommene_tage_ergeben_nachrichten_und_bytes() {
        let mut verlauf = Verlauf::default();
        assert_eq!((verlauf.nachrichten(), verlauf.bytes()), (0, 0));

        verlauf.tag_aufnehmen(tag("2026-10-26"), "a".repeat(128).into_bytes(), vec![0, 64]);
        verlauf.tag_aufnehmen(tag("2026-10-27"), "b".repeat(127).into_bytes(), vec![0, 60]);

        assert_eq!(verlauf.nachrichten(), 4);
        assert_eq!(verlauf.bytes(), 255);
    }

    #[test]
    fn anhaengen_verlaengert_den_rohtext_byte_gleich_und_merkt_den_satzanfang() {
        let mut verlauf = Verlauf::default();
        verlauf.tag_aufnehmen(tag("2026-10-27"), "@ eins\n  x\n\n".into(), vec![0]);

        verlauf.anhaengen(tag("2026-10-27"), "@ zwei\n  y\n\n", false);

        let heute = &verlauf.tage[&tag("2026-10-27")];
        assert_eq!(heute.rohtext, b"@ eins\n  x\n\n@ zwei\n  y\n\n");
        assert_eq!(heute.satzanfaenge, vec![0, 12]);
        assert_eq!(verlauf.nachrichten(), 2);
        assert_eq!(verlauf.bytes(), 24);
    }

    #[test]
    fn ergaenztes_zeilenende_steht_vor_dem_satz_und_verschiebt_den_anfang() {
        let mut verlauf = Verlauf::default();
        verlauf.tag_aufnehmen(tag("2026-10-27"), "@ eins\n  Rest".into(), vec![0]);

        verlauf.anhaengen(tag("2026-10-27"), "@ zwei\n\n", true);

        let heute = &verlauf.tage[&tag("2026-10-27")];
        assert_eq!(heute.rohtext, b"@ eins\n  Rest\n@ zwei\n\n");
        assert_eq!(heute.satzanfaenge, vec![0, 14]);
    }

    #[test]
    fn ein_neuer_tag_beginnt_leer() {
        let mut verlauf = Verlauf::default();
        verlauf.tag_aufnehmen(tag("2026-10-27"), "@ eins\n\n".into(), vec![0]);

        verlauf.anhaengen(tag("2026-10-28"), "@ zwei\n\n", false);

        assert_eq!(verlauf.tage[&tag("2026-10-27")].rohtext, b"@ eins\n\n");
        assert_eq!(verlauf.tage[&tag("2026-10-28")].rohtext, b"@ zwei\n\n");
        assert_eq!(verlauf.tage[&tag("2026-10-28")].satzanfaenge, vec![0]);
        assert_eq!(verlauf.nachrichten(), 2);
    }

    /// Je Tag `anzahl` Sätze à 10 Bytes.
    fn mit_tagen(tage: &[(&str, usize)]) -> Verlauf {
        let mut verlauf = Verlauf::default();
        for (text, anzahl) in tage {
            for _ in 0..*anzahl {
                verlauf.anhaengen(tag(text), "@ x\n  y\n\n\n", false);
            }
        }
        verlauf
    }

    /// R00008 B0182: der Tag geht mit seinen Sätzen und Bytes; die übrigen bleiben.
    #[test]
    fn tag_entfernen_nennt_saetze_und_bytes_und_laesst_die_uebrigen() {
        let mut verlauf = mit_tagen(&[("2026-10-21", 3), ("2026-10-22", 1)]);

        assert_eq!(verlauf.tag_entfernen(tag("2026-10-21")), Some((3, 30)));
        assert_eq!(verlauf.tag_entfernen(tag("2026-10-21")), None);
        assert_eq!(verlauf.tag_entfernen(tag("2026-10-20")), None);
        assert_eq!((verlauf.nachrichten(), verlauf.bytes()), (1, 10));
    }

    #[test]
    fn saetze_vor_zaehlt_nur_die_tage_echt_vor_der_grenze() {
        let verlauf = mit_tagen(&[("2026-10-20", 2), ("2026-10-21", 3), ("2026-10-22", 1)]);

        assert_eq!(verlauf.saetze_vor(tag("2026-10-22")), 5);
        assert_eq!(verlauf.saetze_vor(tag("2026-10-21")), 2);
        assert_eq!(verlauf.saetze_vor(tag("2026-10-20")), 0);
        assert_eq!(verlauf.saetze_vor(tag("2026-10-29")), 6);
    }

    const BEARBEITET_UM_10_00_01: &str = "@ 2026-10-28T10:00:01.500+01:00 a@127.0.0.2\n+ bearbeitet: 2026-10-28T10:00:00.000+01:00\n  neu und länger als der alte Text\n\n";

    /// R00043 F0147 (ADR-0048 Ergänzung „Karte #73“ K6): ein Tag hält nur, wenn ein Änderungssatz auf eine Nachricht
    /// eines anderen Tages im Index zielt — der älteste solche Tag; Änderungen am eigenen Tag, an einem entfallenen Tag
    /// (Waise) und ein Tag außerhalb des Index halten nichts. Mutanten „eigener Tag hält“ und „Waise hält“ rot.
    #[test]
    fn ein_tag_haelt_nur_fuer_ziele_in_anderen_tagen_des_index() {
        let mut verlauf = Verlauf::default();
        for (datum, satz) in [
            (
                "2026-10-25",
                "@ 2026-10-25T10:00:00.000+01:00 a@127.0.0.2\n  v\n\n",
            ),
            (
                "2026-10-26",
                "@ 2026-10-26T10:00:00.000+01:00 a@127.0.0.2\n  w\n\n",
            ),
            (
                "2026-10-27",
                "@ 2026-10-27T09:00:00.000+01:00 a@127.0.0.2\n  x\n\n",
            ),
            (
                "2026-10-27",
                "@ 2026-10-27T09:00:01.000+01:00 a@127.0.0.2\n+ bearbeitet: 2026-10-27T09:00:00.000+01:00\n  x2\n\n",
            ),
            (
                "2026-10-27",
                "@ 2026-10-27T09:00:02.000+01:00 a@127.0.0.2\n+ geloescht: 2026-10-26T10:00:00.000+01:00\n\n",
            ),
            (
                "2026-10-27",
                "@ 2026-10-27T09:00:03.000+01:00 a@127.0.0.2\n+ bearbeitet: 2026-10-25T10:00:00.000+01:00\n  v2\n\n",
            ),
            (
                "2026-10-28",
                "@ 2026-10-28T09:00:00.000+01:00 a@127.0.0.2\n+ geloescht: 2026-10-20T10:00:00.000+01:00\n\n",
            ),
            (
                "2026-10-28",
                "@ 2026-10-28T09:00:01.000+01:00 a@127.0.0.2\n+ bearbeitet: 2026-10-28T09:00:00.000+01:00\n  y\n\n",
            ),
        ] {
            verlauf.anhaengen(tag(datum), satz, false);
        }
        let haelt = |verlauf: &Verlauf, datum| {
            verlauf.tag_haelt_aenderungen_fuer_bestehende_tage(tag(datum))
        };

        assert_eq!(haelt(&verlauf, "2026-10-27"), Some(tag("2026-10-25")));
        assert_eq!(haelt(&verlauf, "2026-10-28"), None, "Waise und Satzzeit");
        assert_eq!(haelt(&verlauf, "2026-10-26"), None, "ohne Änderungssätze");
        assert_eq!(haelt(&verlauf, "2026-10-29"), None, "nicht im Index");
        verlauf.tag_entfernen(tag("2026-10-25"));
        assert_eq!(haelt(&verlauf, "2026-10-27"), Some(tag("2026-10-26")));
        verlauf.tag_entfernen(tag("2026-10-26"));
        assert_eq!(
            haelt(&verlauf, "2026-10-27"),
            None,
            "nur noch der eigene Tag"
        );
    }

    /// Ein Tag 28.10. mit Nachrichten um 10:00:00, 10:00:01, 10:00:02 und Änderungssätzen nach der ersten und nach
    /// der letzten — geladen oder angehängt.
    fn mit_aenderungssaetzen(angehaengt: bool) -> Verlauf {
        let saetze = [
            "@ 2026-10-28T10:00:00.000+01:00 a@127.0.0.2\n  x\n\n",
            "@ 2026-10-28T10:00:01.000+01:00 a@127.0.0.2\n  y\n\n",
            BEARBEITET_UM_10_00_01,
            "@ 2026-10-28T10:00:02.000+01:00 a@127.0.0.2\n  z\n\n",
            "@ 2026-10-28T10:00:03.000+01:00 a@127.0.0.2\n+ geloescht: 2026-10-28T10:00:02.000+01:00\n\n",
        ];
        let mut verlauf = Verlauf::default();
        if angehaengt {
            for satz in saetze {
                verlauf.anhaengen(tag("2026-10-28"), satz, false);
            }
        } else {
            let mut rohtext = Vec::new();
            let mut anfaenge = Vec::new();
            for satz in saetze {
                anfaenge.push(rohtext.len());
                rohtext.extend_from_slice(satz.as_bytes());
            }
            verlauf.tag_aufnehmen(tag("2026-10-28"), rohtext, anfaenge);
        }
        verlauf
    }

    /// R00039 B0595 (ADR-0048 Regel 3): Änderungssätze stehen im Rohtext (Bytes, `belegt`), aber der Block zählt,
    /// sucht und kopiert nur Nachrichten; die Spanne einer Nachricht endet vor dem nächsten Satz gleich welcher
    /// Art. Mutante „Änderungssatz im Block als Nachricht“ rot.
    #[test]
    fn aenderungssaetze_sind_im_rohtext_aber_nie_im_block() {
        for angehaengt in [false, true] {
            let verlauf = mit_aenderungssaetzen(angehaengt);
            let nach = |zeit_text: &str, n| {
                verlauf.block_ohne_grenze(&abfrage(Abschnitt::Nach(zeit(zeit_text)), n))
            };

            let neueste = verlauf.block_ohne_grenze(&abfrage(Abschnitt::Neueste, 100));
            let nach_erster = nach("2026-10-28T10:00:00.000+01:00", 1);
            let vor_letzter = verlauf.block_ohne_grenze(&abfrage(
                Abschnitt::Vor(zeit("2026-10-28T10:00:02.000+01:00")),
                1,
            ));

            assert_eq!(
                zeiten_im(&neueste),
                erwartet("2026-10-28", 0..3),
                "angehängt {angehaengt}"
            );
            assert_eq!(
                neueste.saetze[1].satz,
                b"@ 2026-10-28T10:00:01.000+01:00 a@127.0.0.2\n  y\n\n".to_vec(),
                "die Spanne endet vor dem Änderungssatz"
            );
            assert_eq!(zeiten_im(&nach_erster), erwartet("2026-10-28", 1..2));
            assert!(nach_erster.neuer);
            assert_eq!(zeiten_im(&vor_letzter), erwartet("2026-10-28", 1..2));
            assert!(nach("2026-10-28T10:00:02.000+01:00", 100).saetze.is_empty());
            assert!(!nach("2026-10-28T10:00:02.000+01:00", 100).neuer);
            assert_eq!(verlauf.nachrichten(), 3);
            let alle: usize = verlauf.tage.values().map(|tag| tag.rohtext.len()).sum();
            assert_eq!(verlauf.bytes(), alle as u64);
            assert_eq!(verlauf.saetze_vor(tag("2026-10-29")), 3);
        }
        let mut verlauf = mit_aenderungssaetzen(true);
        let bytes = verlauf.bytes();
        assert_eq!(verlauf.tag_entfernen(tag("2026-10-28")), Some((3, bytes)));
    }

    /// Das Budget zählt nur Nachrichten: zwei Nachrichten passen genau, der Änderungssatz dazwischen zählt nicht.
    #[test]
    fn das_budget_zaehlt_keine_aenderungssaetze() {
        let verlauf = mit_aenderungssaetzen(false);
        let zwei = 2 * "@ 2026-10-28T10:00:01.000+01:00 a@127.0.0.2\n  y\n\n".len();

        let block = verlauf.block(&abfrage(Abschnitt::Neueste, 100), zwei);

        assert_eq!(zeiten_im(&block), erwartet("2026-10-28", 1..3));
    }

    /// R00039 B0596 (ADR-0048 Regeln 1–3): Fassungen aus Änderungssätzen über drei Tage — Bearbeitet ersetzt
    /// Bearbeitet, Geloescht ist endgültig (Mutante „Bearbeitet nach Geloescht übernommen“ rot), fremde Kennung und
    /// `bearbeitet` nach `geloescht` sind Defekte mit Zeile, eine Änderung ohne Nachricht ist eine stille Waise;
    /// Entfallen des Tages nimmt die Fassungen seiner Nachrichten mit.
    #[test]
    fn fassungen_werden_ueber_alle_tage_gebildet_und_mit_dem_tag_entfernt() {
        let t1 = "2026-10-27T10:00:00.000+01:00";
        let t2 = "2026-10-27T10:00:01.000+01:00";
        let t3 = "2026-10-27T10:00:02.000+01:00";
        let tag_27 = format!(
            "@ {t1} alice@127.0.0.2\n  eins\n\n@ {t2} alice@127.0.0.2\n  zwei\n\n@ {t3} bob@127.0.0.3\n  drei\n\n"
        );
        let tag_28 = format!(
            "@ 2026-10-28T09:00:00.000+01:00 alice@127.0.0.2\n+ bearbeitet: {t1}\n  erste Fassung\n\n\
             @ 2026-10-28T09:00:01.000+01:00 bob@127.0.0.3\n+ bearbeitet: {t1}\n  fremd\n\n\
             @ 2026-10-28T09:00:02.000+01:00 alice@127.0.0.2\n+ geloescht: {t2}\n\n\
             @ 2026-10-28T09:00:03.000+01:00 alice@127.0.0.2\n+ bearbeitet: {t2}\n  zu spät\n\n\
             @ 2026-10-28T09:00:04.000+01:00 alice@127.0.0.2\n+ bearbeitet: {t1}\n  zweite Fassung\n\n\
             @ 2026-10-28T09:00:05.000+01:00 alice@127.0.0.2\n+ bearbeitet: 2026-10-20T10:00:00.000+02:00\n  Waise\n\n\
             @ 2026-10-28T09:00:06.000+01:00 alice@127.0.0.2\n+ bearbeitet: {t1}\n+ geloescht: {t1}\n\n"
        );
        let tag_29 = format!(
            "@ 2026-10-29T09:00:00.000+01:00 bob@127.0.0.3\n+ bearbeitet: {t3}\n  dritte\n\n"
        );
        let mut verlauf = Verlauf::default();
        for (datum, rohtext) in [
            ("2026-10-27", &tag_27),
            ("2026-10-28", &tag_28),
            ("2026-10-29", &tag_29),
        ] {
            let anfaenge = logformat::saetze(rohtext.as_bytes())
                .into_iter()
                .filter_map(Result::ok)
                .map(|satz| satz.anfang)
                .collect();
            verlauf.tag_aufnehmen(tag(datum), rohtext.clone().into_bytes(), anfaenge);
        }
        let zeile = |kopf: &str| {
            tag_28[..tag_28.find(kopf).expect("Kopf")]
                .matches('\n')
                .count()
                + 1
        };

        let bericht = verlauf.fassungen_bilden();

        assert_eq!(bericht.waisen, 1);
        assert_eq!(
            bericht.defekte,
            vec![
                Aenderungsbefund {
                    tag: tag("2026-10-28"),
                    zeile: zeile("@ 2026-10-28T09:00:01"),
                    grund: Aenderungsgrund::Fremd
                },
                Aenderungsbefund {
                    tag: tag("2026-10-28"),
                    zeile: zeile("@ 2026-10-28T09:00:03"),
                    grund: Aenderungsgrund::NachGeloescht
                },
                Aenderungsbefund {
                    tag: tag("2026-10-28"),
                    zeile: zeile("@ 2026-10-28T09:00:06"),
                    grund: Aenderungsgrund::Satz(logformat::Aenderungsdefekt::ZweiBezuege),
                },
            ]
        );
        let erste = verlauf.fassung(&zeit(t1)).expect("t1 bearbeitet");
        assert_eq!(erste.geaendert, zeit("2026-10-28T09:00:04.000+01:00"));
        let satz = verlauf.aenderungssatz(&erste).expect("Satz");
        assert_eq!(
            logformat::text(&String::from_utf8_lossy(&satz)),
            "zweite Fassung"
        );
        assert_eq!(
            verlauf.fassung(&zeit(t2)),
            Some(Fassung {
                geaendert: zeit("2026-10-28T09:00:02.000+01:00"),
                zustand: Fassungszustand::Geloescht
            })
        );
        let dritte = verlauf
            .fassung(&zeit("2026-10-27T09:00:02.000Z"))
            .expect("t3 in UTC");
        assert_eq!(dritte.geaendert, zeit("2026-10-29T09:00:00.000+01:00"));
        assert_eq!(verlauf.nachrichten(), 3);

        verlauf.tag_entfernen(tag("2026-10-27"));

        for ziel in [t1, t2, t3] {
            assert_eq!(verlauf.fassung(&zeit(ziel)), None, "{ziel}");
        }
    }

    /// B0596 schreiben: ein angehängter Bearbeitungssatz setzt die Fassung seiner Nachricht an seinen Ort; die Zahl
    /// der Nachrichten bleibt.
    #[test]
    fn eine_angehaengte_bearbeitung_setzt_die_fassung() {
        let mut verlauf = Verlauf::default();
        verlauf.anhaengen(
            tag("2026-10-27"),
            "@ 2026-10-27T10:00:00.000+01:00 a@127.0.0.2\n  alt\n\n",
            false,
        );
        let satz = "@ 2026-10-28T10:00:00.000+01:00 a@127.0.0.2\n+ bearbeitet: 2026-10-27T10:00:00.000+01:00\n  neu\n\n";

        verlauf.bearbeitung_anhaengen(
            tag("2026-10-28"),
            satz,
            false,
            zeit("2026-10-27T10:00:00.000+01:00"),
            zeit("2026-10-28T10:00:00.000+01:00"),
        );

        let fassung = verlauf
            .fassung(&zeit("2026-10-27T10:00:00.000+01:00"))
            .expect("Fassung");
        assert_eq!(
            fassung,
            Fassung {
                geaendert: zeit("2026-10-28T10:00:00.000+01:00"),
                zustand: Fassungszustand::Bearbeitet {
                    tag: tag("2026-10-28"),
                    anfang: 0
                }
            }
        );
        assert_eq!(
            verlauf.aenderungssatz(&fassung),
            Some(satz.as_bytes().to_vec())
        );
        assert_eq!(verlauf.nachrichten(), 1);
    }

    /// R00039 B0597 (ADR-0048 Regel 3): eine bearbeitete Nachricht kommt mit ihrem Änderungssatz in den Block, eine
    /// gelöschte ohne; das Budget zählt bei Bearbeitet den Änderungssatz statt der Nachricht.
    #[test]
    fn der_block_liefert_die_fassung_und_zaehlt_den_aenderungssatz() {
        let mut verlauf = Verlauf::default();
        let erste = "@ 2026-10-28T10:00:00.000+01:00 a@127.0.0.2\n  x\n\n";
        let zweite = "@ 2026-10-28T10:00:01.000+01:00 a@127.0.0.2\n  y\n\n";
        let bearbeitung = "@ 2026-10-28T10:00:02.000+01:00 a@127.0.0.2\n+ bearbeitet: 2026-10-28T10:00:00.000+01:00\n  ein deutlich längerer neuer Text\n\n";
        verlauf.anhaengen(tag("2026-10-28"), erste, false);
        verlauf.anhaengen(tag("2026-10-28"), zweite, false);
        verlauf.bearbeitung_anhaengen(
            tag("2026-10-28"),
            bearbeitung,
            false,
            zeit("2026-10-28T10:00:00.000+01:00"),
            zeit("2026-10-28T10:00:02.000+01:00"),
        );

        let alle = verlauf.block_ohne_grenze(&abfrage(Abschnitt::Neueste, 100));
        let knapp = verlauf.block(
            &abfrage(Abschnitt::Neueste, 100),
            erste.len() + zweite.len(),
        );

        assert_eq!(
            alle.saetze[0].aenderung,
            Some(Kopierte {
                geaendert: zeit("2026-10-28T10:00:02.000+01:00"),
                satz: Some(bearbeitung.as_bytes().to_vec()),
            })
        );
        assert_eq!(alle.saetze[0].satz, erste.as_bytes().to_vec());
        assert_eq!(alle.saetze[1].aenderung, None);
        assert_eq!(
            zeiten_im(&knapp),
            vec!["2026-10-28T10:00:01.000+01:00".to_string()]
        );
        assert!(knapp.aelter);
    }

    /// Code-Review #48 A4 (R00040, ADR-0048 Regel 3 „das Budget zählt die gelieferten Bytes“): eine gelöschte
    /// Nachricht liefert weder Text noch Anhänge — im Budget zählt nur ihre Kopfzeile, nicht ihr langer Text.
    #[test]
    fn eine_geloeschte_nachricht_zaehlt_im_budget_nur_ihre_kopfzeile() {
        let mut verlauf = Verlauf::default();
        let kopf = "@ 2026-10-28T10:00:00.000+01:00 a@127.0.0.2\n";
        let lang = format!("{kopf}  {}\n\n", "x".repeat(2_000));
        let zweite = "@ 2026-10-28T10:00:01.000+01:00 a@127.0.0.2\n  y\n\n";
        let loeschung = "@ 2026-10-28T10:00:02.000+01:00 a@127.0.0.2\n+ geloescht: 2026-10-28T10:00:00.000+01:00\n\n";
        verlauf.anhaengen(tag("2026-10-28"), &lang, false);
        verlauf.anhaengen(tag("2026-10-28"), zweite, false);
        verlauf.loeschung_anhaengen(
            tag("2026-10-28"),
            loeschung,
            false,
            zeit("2026-10-28T10:00:00.000+01:00"),
            zeit("2026-10-28T10:00:02.000+01:00"),
        );

        let knapp = verlauf.block(&abfrage(Abschnitt::Neueste, 100), kopf.len() + zweite.len());

        assert_eq!(
            zeiten_im(&knapp),
            vec![
                "2026-10-28T10:00:00.000+01:00".to_string(),
                "2026-10-28T10:00:01.000+01:00".to_string()
            ]
        );
        assert!(!knapp.aelter);
    }
}
