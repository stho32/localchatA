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
    satzanfaenge: Vec<usize>,
}

impl Verlauf {
    /// Nimmt einen geladenen Tag auf: Rohtext und die Byte-Anfänge seiner gültigen Sätze.
    pub fn tag_aufnehmen(&mut self, tag: NaiveDate, rohtext: Vec<u8>, satzanfaenge: Vec<usize>) {
        self.tage.insert(
            tag,
            Tag {
                rohtext,
                satzanfaenge,
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
        tag.satzanfaenge.push(tag.rohtext.len());
        tag.rohtext.extend_from_slice(satz.as_bytes());
    }

    pub fn nachrichten(&self) -> u64 {
        self.tage
            .values()
            .map(|tag| tag.satzanfaenge.len() as u64)
            .sum()
    }

    pub fn bytes(&self) -> u64 {
        self.tage.values().map(|tag| tag.rohtext.len() as u64).sum()
    }
}

/// Ein Block des Verlaufs: Kopien der Sätze, älteste zuerst, und ob es davor bzw. danach noch Sätze gibt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub saetze: Vec<Vec<u8>>,
    pub aelter: bool,
    pub neuer: bool,
}

impl Verlauf {
    /// Höchstens `n` Sätze des Abschnitts über beliebig viele Tage, älteste zuerst (ADR-0025 Regeln 2–4). Die Arbeit
    /// hängt an `n` und an der Zahl der Tage im Abschnitt, nie an der Länge des Verlaufs.
    pub fn block(&self, abfrage: &Abfrage) -> Block {
        let n = abfrage.n.wert();
        match abfrage.abschnitt {
            Abschnitt::Neueste => self.rueckwaerts(None, n),
            Abschnitt::Vor(zeit) => self.rueckwaerts(Some(zeit), n),
            Abschnitt::Nach(zeit) => self.vorwaerts(zeit, n),
        }
    }

    /// Die `n` jüngsten Sätze echt vor `bis` (ohne `bis`: die neuesten), vom jüngsten Tag zurück.
    fn rueckwaerts(&self, bis: Option<Zeit>, n: usize) -> Block {
        let letzter_tag = bis.map_or(NaiveDate::MAX, |zeit| tag_mit_abstand(zeit, 2));
        let mut gesammelt = Vec::new();
        let mut aelter = false;
        for tag in self.tage.range(..=letzter_tag).rev().map(|(_, tag)| tag) {
            let ende = match bis {
                Some(zeit) => grenze_im_tag(&tag.rohtext, &tag.satzanfaenge, &zeit, Seite::Vor),
                None => tag.satzanfaenge.len(),
            };
            if gesammelt.len() == n {
                aelter = ende > 0;
                if aelter {
                    break;
                }
                continue;
            }
            let anfang = ende.saturating_sub(n - gesammelt.len());
            gesammelt.extend((anfang..ende).rev().map(|index| tag.satz(index)));
            if anfang > 0 {
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

    /// Die `n` ältesten Sätze echt nach `ab`, vom Tag der Zeit vorwärts.
    fn vorwaerts(&self, ab: Zeit, n: usize) -> Block {
        let erster_tag = tag_mit_abstand(ab, -2);
        let mut gesammelt = Vec::new();
        let mut neuer = false;
        for tag in self.tage.range(erster_tag..).map(|(_, tag)| tag) {
            let anfang = grenze_im_tag(&tag.rohtext, &tag.satzanfaenge, &ab, Seite::Nach);
            let laenge = tag.satzanfaenge.len();
            if gesammelt.len() == n {
                neuer = anfang < laenge;
                if neuer {
                    break;
                }
                continue;
            }
            let ende = laenge.min(anfang + n - gesammelt.len());
            gesammelt.extend((anfang..ende).map(|index| tag.satz(index)));
            if ende < laenge {
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
            .find(|tag| !tag.satzanfaenge.is_empty())?;
        logformat::kopfzeit(&tag.satz(tag.satzanfaenge.len() - 1))
    }

    fn aelteste_kopfzeit(&self) -> Option<Zeit> {
        let tag = self
            .tage
            .values()
            .find(|tag| !tag.satzanfaenge.is_empty())?;
        logformat::kopfzeit(&tag.satz(0))
    }
}

impl Tag {
    /// Kopie des Satzes `index`: von seinem Anfang bis vor den nächsten Satz bzw. bis zum Ende des Tages.
    fn satz(&self, index: usize) -> Vec<u8> {
        let anfang = self.satzanfaenge[index];
        let ende = self
            .satzanfaenge
            .get(index + 1)
            .copied()
            .unwrap_or(self.rohtext.len());
        self.rohtext[anfang..ende].to_vec()
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
        let block = drei_tage().block(&abfrage(Abschnitt::Neueste, 100));

        assert_eq!(zeiten_im(&block), erwartet("2026-10-28", 150..250));
        assert!(block.aelter);
        assert!(!block.neuer);
    }

    #[test]
    fn vor_reicht_lueckenlos_ueber_die_tagesgrenze() {
        let vor = zeit(&satzzeit("2026-10-28", 49));

        let block = drei_tage().block(&abfrage(Abschnitt::Vor(vor), 100));

        let mut zeiten = erwartet("2026-10-27", 199..250);
        zeiten.extend(erwartet("2026-10-28", 0..49));
        assert_eq!(zeiten_im(&block), zeiten);
        assert!(block.aelter);
        assert!(block.neuer);
    }

    #[test]
    fn nach_dem_letzten_des_aeltesten_tages_kommt_der_anfang_des_mittleren() {
        let nach = zeit(&satzzeit("2026-10-26", 249));

        let block = drei_tage().block(&abfrage(Abschnitt::Nach(nach), 100));

        assert_eq!(zeiten_im(&block), erwartet("2026-10-27", 0..100));
        assert!(block.aelter);
        assert!(block.neuer);
    }

    #[test]
    fn nach_einer_zeit_vor_dem_aeltesten_tag_kommen_die_ersten_n() {
        let block = drei_tage().block(&abfrage(
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

        let vor_dem_ersten = verlauf.block(&abfrage(
            Abschnitt::Vor(zeit(&satzzeit("2026-10-26", 0))),
            100,
        ));
        let nach_dem_letzten = verlauf.block(&abfrage(
            Abschnitt::Nach(zeit(&satzzeit("2026-10-28", 249))),
            100,
        ));
        let leer = Verlauf::default().block(&abfrage(Abschnitt::Neueste, 100));

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

        let alle_bis_auf_einen = verlauf.block(&abfrage(
            Abschnitt::Vor(zeit(&satzzeit("2026-10-26", 200))),
            200,
        ));
        let ab_dem_zweiten = verlauf.block(&abfrage(
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

        let block = verlauf.block(&abfrage(Abschnitt::Neueste, 100));

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
}
