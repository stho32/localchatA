//! Verlaufsindex (A00009, ADR-0006): je Tag der Rohtext der Datei und die Anfänge seiner Sätze.
//!
//! Der Rohtext sind die Bytes der Tagesdatei, auch ungültiges UTF-8 darin; er bleibt byte-gleich mit ihr,
//! solange nur nach erfolgreichem Schreiben angehängt wird.

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
