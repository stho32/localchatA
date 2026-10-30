//! Trefferregel (ADR-0054 Regel 2, E1, E3; B0736): ob eine Nachricht einen Wartenden weckt und mit welchem Anlass.
//! Rein — Nachholen und Bus benutzen sie beide.
//!
//! Nie trifft die eigene Nachricht des Wartenden und nie eine gelöschte. Ohne Filter trifft jede sichtbare
//! Nachricht; mit Filter genügt **ein** passender Teil (Vereinigung). Direktgespräche treffen erst, wenn ihre
//! Beteiligten bekannt sind (I0012, B0747), bis dahin nie.

use super::{Aenderungszustand, Gespraech, Kennung, Nachricht};
use std::collections::BTreeSet;

/// Warum eine Nachricht trifft — ein Fakt der Nachricht, unabhängig vom Filter (Regel 2). Die Zweige setzen
/// B0742 (`erwaehnung`), B0747 (`direkt`) und B0712 (`antwort`); eine gewöhnliche Channel-Nachricht hat keinen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Anlass {
    Direkt,
    Erwaehnung,
    Antwort,
}

impl Anlass {
    /// Das Wort an der Grenze.
    pub fn als_text(self) -> &'static str {
        match self {
            Anlass::Direkt => "direkt",
            Anlass::Erwaehnung => "erwaehnung",
            Anlass::Antwort => "antwort",
        }
    }
}

/// Was ein Wartender sehen will: genannte Channels und die Schalter; alles leer heißt „alles Sichtbare“ (E2).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Filter {
    pub channels: BTreeSet<String>,
    pub direkt: bool,
    pub erwaehnung: bool,
}

impl Filter {
    /// Ohne jede Angabe (E2, PO Zeile 160).
    pub fn ohne_angabe(&self) -> bool {
        self.channels.is_empty() && !self.direkt && !self.erwaehnung
    }
}

/// Eine Nachricht, die einen Wartenden weckt: Gespräch, Anlass (aufsteigend, ohne Doppel) und die Nachricht.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Treffer {
    pub gespraech: Gespraech,
    pub anlass: Vec<Anlass>,
    pub nachricht: Nachricht,
}

/// Der Anlass, wenn die Nachricht des Gesprächs den Wartenden unter dem Filter weckt; sonst keiner.
pub fn treffer(
    gespraech: &Gespraech,
    nachricht: &Nachricht,
    wartender: &Kennung,
    filter: &Filter,
) -> Option<Vec<Anlass>> {
    let Gespraech::Channel(channel) = gespraech else {
        return None;
    };
    let geloescht = nachricht
        .geaendert
        .is_some_and(|geaendert| geaendert.zustand == Aenderungszustand::Geloescht);
    if &nachricht.von == wartender || geloescht {
        return None;
    }
    let anlass = Vec::new();
    let im_channel = filter.channels.contains(channel);
    (filter.ohne_angabe() || im_channel).then_some(anlass)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uhr::Zeit;

    fn kennung(text: &str) -> Kennung {
        Kennung::bilden(text).expect("Testkennung")
    }

    fn nachricht(von: &str) -> Nachricht {
        Nachricht {
            zeit: Zeit::parse_from_rfc3339("2026-10-30T12:00:00.000+01:00").expect("Zeit"),
            von: kennung(von),
            art: None,
            text: "Hallo".to_string(),
            anhaenge: Vec::new(),
            geaendert: None,
            bezug: None,
            antwort_auf: None,
        }
    }

    fn geloescht(von: &str) -> Nachricht {
        Nachricht {
            text: String::new(),
            geaendert: Some(super::super::Geaendert {
                zeit: Zeit::parse_from_rfc3339("2026-10-30T12:00:01.000+01:00").expect("Zeit"),
                zustand: Aenderungszustand::Geloescht,
            }),
            ..nachricht(von)
        }
    }

    fn channels(namen: &[&str]) -> Filter {
        Filter {
            channels: namen.iter().map(|name| (*name).to_string()).collect(),
            ..Filter::default()
        }
    }

    const CLAUDE: &str = "claude@127.0.0.1";
    const ANNA: &str = "anna@127.0.0.1";

    /// W2a (B0736): die Tabelle Gespräch × Absender × Filter → Anlass oder kein Treffer.
    #[test]
    fn die_trefferregel_als_tabelle() {
        let a = Gespraech::channel("a");
        let b = Gespraech::channel("b");
        let c = Gespraech::channel("c");
        let direkt = Gespraech::Direkt("anna".to_string());
        let ohne = Filter::default();
        let nur_a = channels(&["a"]);
        let a_und_b = channels(&["a", "b"]);
        let kein: Option<Vec<Anlass>> = None;
        let leer = Some(Vec::new());
        let faelle = [
            (
                "ohne Filter, fremd in a",
                &a,
                nachricht(ANNA),
                &ohne,
                leer.clone(),
            ),
            (
                "ohne Filter, fremd in c",
                &c,
                nachricht(ANNA),
                &ohne,
                leer.clone(),
            ),
            (
                "ohne Filter, eigene",
                &a,
                nachricht(CLAUDE),
                &ohne,
                kein.clone(),
            ),
            (
                "ohne Filter, gelöscht",
                &a,
                geloescht(ANNA),
                &ohne,
                kein.clone(),
            ),
            ("channel=a, in a", &a, nachricht(ANNA), &nur_a, leer.clone()),
            ("channel=a, in b", &b, nachricht(ANNA), &nur_a, kein.clone()),
            (
                "channel=a, eigene in a",
                &a,
                nachricht(CLAUDE),
                &nur_a,
                kein.clone(),
            ),
            (
                "channel=a, gelöscht in a",
                &a,
                geloescht(ANNA),
                &nur_a,
                kein.clone(),
            ),
            (
                "channel=a&b, in a",
                &a,
                nachricht(ANNA),
                &a_und_b,
                leer.clone(),
            ),
            (
                "channel=a&b, in b",
                &b,
                nachricht(ANNA),
                &a_und_b,
                leer.clone(),
            ),
            (
                "channel=a&b, in c",
                &c,
                nachricht(ANNA),
                &a_und_b,
                kein.clone(),
            ),
            (
                "Direktgespräch ohne Filter",
                &direkt,
                nachricht(ANNA),
                &ohne,
                kein.clone(),
            ),
        ];
        for (fall, gespraech, nachricht, filter, erwartet) in faelle {
            assert_eq!(
                treffer(gespraech, &nachricht, &kennung(CLAUDE), filter),
                erwartet,
                "{fall}"
            );
        }
    }

    #[test]
    fn ohne_angabe_heisst_kein_channel_und_kein_schalter() {
        assert!(Filter::default().ohne_angabe());
        assert!(!channels(&["a"]).ohne_angabe());
        assert!(
            !Filter {
                erwaehnung: true,
                ..Filter::default()
            }
            .ohne_angabe()
        );
    }
}
