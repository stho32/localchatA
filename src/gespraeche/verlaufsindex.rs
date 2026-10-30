//! Verlaufsindex (A00009, ADR-0006): je Tag der Rohtext der Datei und die Anfänge seiner Sätze.
//!
//! Der Rohtext sind die Bytes der Tagesdatei, auch ungültiges UTF-8 darin; er bleibt byte-gleich mit ihr,
//! solange nur nach erfolgreichem Schreiben angehängt wird.

use super::abfrage::{Abfrage, Abschnitt};
use super::logformat;
use crate::uhr::Zeit;
use chrono::NaiveDate;
use std::collections::BTreeMap;

/// Der Verlauf eines Gesprächs.
#[derive(Debug, Default)]
pub struct Verlauf {
    tage: BTreeMap<NaiveDate, Tag>,
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
    /// steht davor das `\n`, das die Logablage ergänzt hat.
    pub fn anhaengen(&mut self, tag: NaiveDate, satz: &str, zeilenende_ergaenzt: bool) {
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
    pub fn tag_entfernen(&mut self, tag: NaiveDate) -> Option<(usize, u64)> {
        self.tage
            .remove(&tag)
            .map(|tag| (tag.nachrichten.len(), tag.rohtext.len() as u64))
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
    pub saetze: Vec<Vec<u8>>,
    pub aelter: bool,
    pub neuer: bool,
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
            while index > 0 && sammler.nimmt(tag.satzlaenge(index - 1)) {
                index -= 1;
                gesammelt.push(tag.satz(index));
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
            while index < laenge && sammler.nimmt(tag.satzlaenge(index)) {
                gesammelt.push(tag.satz(index));
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

    fn spanne(&self, index: usize) -> std::ops::Range<usize> {
        let anfang = self.nachrichten[index];
        let naechster = self.satzanfaenge.partition_point(|satz| *satz <= anfang);
        let ende = self
            .satzanfaenge
            .get(naechster)
            .copied()
            .unwrap_or(self.rohtext.len());
        anfang..ende
    }
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
    satzanfaenge.partition_point(|&anfang| {
        let Some(kopfzeit) = rohtext.get(anfang..).and_then(logformat::kopfzeit) else {
            return true;
        };
        match seite {
            Seite::Vor => kopfzeit < *zeit,
            Seite::Nach => kopfzeit <= *zeit,
        }
    })
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
            .map(|satz| {
                logformat::kopfzeit(satz)
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
            block.saetze,
            vec![
                b"@ 2026-10-28T10:00:00.000+01:00 a@b\n  eins\n\n".to_vec(),
                b"@ 2026-10-28T10:00:01.000+01:00 a@b\n  zwei\n\n".to_vec()
            ]
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
                neueste.saetze[1],
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
}
