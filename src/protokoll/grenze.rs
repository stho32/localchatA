//! Welche Grenze griff (ADR-0042 Ergänzung Karte #26 Punkt 1): im Betriebsprotokoll ist `grenze` immer ein Wort
//! dieser Aufzählung, nie eine Zahl oder Freitext; die Zahl der gegriffenen Grenze heißt `hoechstens`.

use super::episoden::Auftreten;
use super::{Bereich, Kontext, Schluessel, Stufe};
use std::fmt;
use std::net::IpAddr;

/// Die Art einer Grenze: je Herkunft, gesamt oder je Kennung (Schreibkonto).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grenzart {
    Herkunft,
    Gesamt,
    Kennung,
}

impl Grenzart {
    pub fn als_text(self) -> &'static str {
        match self {
            Grenzart::Herkunft => "herkunft",
            Grenzart::Gesamt => "gesamt",
            Grenzart::Kennung => "kennung",
        }
    }
}

impl fmt::Display for Grenzart {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.als_text())
    }
}

/// Eine gegriffene Grenze: welche und ihre Zahl.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gegriffen {
    pub art: Grenzart,
    pub hoechstens: usize,
}

/// Ein Auftreten an einer Platzgrenze (Verbindung, Sitzung, Channels): WARNING-Episode je Herkunft mit dem
/// Kontext `herkunft`, `grenze`, `hoechstens`; die Grenze ist zugleich die Art des Auftretens, damit die
/// Ende-Zeile je Grenze zählt (`(gesamt <g>; herkunft <h>)`). `grenze` und `hoechstens` gehören dem einzelnen
/// Auftreten und stehen nur in der Beginn-Zeile (ADR-0042 Ergänzung Karte #26 Punkt 1 und Karte #66).
pub fn grenzauftreten(
    bereich: Bereich,
    text: &'static str,
    herkunft: IpAddr,
    gegriffen: Gegriffen,
) -> Auftreten {
    Auftreten {
        stufe: Stufe::Warning,
        bereich,
        text,
        quelle: herkunft.to_string(),
        kontext: Kontext::neu()
            .mit(Schluessel::Herkunft, herkunft)
            .mit_nur_beginn(Schluessel::Grenze, gegriffen.art)
            .mit_nur_beginn(Schluessel::Hoechstens, gegriffen.hoechstens),
        art: Some(gegriffen.art.als_text()),
        verworfen: None,
        bytes: None,
        anzeige: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protokoll::testhilfe::{im_speicher, zeit};

    const SITZUNG: &str = "Beitritt an der Sitzungsgrenze abgewiesen";

    fn sitzung(art: Grenzart, hoechstens: usize) -> Auftreten {
        grenzauftreten(
            Bereich::Sitzung,
            SITZUNG,
            "127.0.0.1".parse().expect("IP"),
            Gegriffen { art, hoechstens },
        )
    }

    /// R00018 F0081 „Eine Schreibweise“: `grenze` ist das Wort, `hoechstens` die Zahl.
    #[test]
    fn der_kontext_nennt_die_grenze_als_wort_und_ihre_zahl() {
        let auftreten = sitzung(Grenzart::Gesamt, 512);

        assert_eq!(auftreten.kontext.wert(Schluessel::Grenze), Some("gesamt"));
        assert_eq!(auftreten.kontext.wert(Schluessel::Hoechstens), Some("512"));
        assert_eq!(
            auftreten.kontext.wert(Schluessel::Herkunft),
            Some("127.0.0.1")
        );
        assert_eq!(auftreten.art, Some("gesamt"));
    }

    /// R00018 F0081 „Zählung je Art“: 5 × herkunft und 1 × gesamt derselben Herkunft → eine Episode, deren
    /// Ende-Zeile je Grenze zählt.
    #[test]
    fn die_ende_zeile_zaehlt_je_grenze() {
        let (protokoll, speicher, uhr, _) =
            im_speicher("2026-10-28T12:00:00+01:00", Stufe::Information);
        for _ in 0..5 {
            protokoll.episode(sitzung(Grenzart::Herkunft, 128));
        }
        protokoll.episode(sitzung(Grenzart::Gesamt, 512));
        uhr.stellen(zeit("2026-10-28T12:01:01+01:00"));
        protokoll.episoden_schliessen();

        let zeilen = speicher.zeilen();
        assert_eq!(zeilen.len(), 2, "{zeilen:#?}");
        assert!(
            zeilen[0].ends_with(&format!(
                " WARNING sitzung {SITZUNG} — Episode beginnt | herkunft=127.0.0.1 grenze=herkunft hoechstens=128"
            )),
            "{zeilen:#?}"
        );
        assert!(
            zeilen[1].ends_with(&format!(
                " WARNING sitzung {SITZUNG} — Episode endet: 6 in 0 s (gesamt 1; herkunft 5) | herkunft=127.0.0.1"
            )),
            "R00038 B0591: grenze und hoechstens gehören dem ersten Auftreten, das Ende zählt je Grenze: {zeilen:#?}"
        );
    }
}
