//! Verlaufsindex (A00009, ADR-0006): je Tag der Rohtext der Datei und die Anfänge seiner Sätze.
//!
//! Der Rohtext sind die Bytes der Tagesdatei, auch ungültiges UTF-8 darin; er bleibt byte-gleich mit ihr,
//! solange nur nach erfolgreichem Schreiben angehängt wird.

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

#[cfg(test)]
mod tests {
    use super::*;

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
