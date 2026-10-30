//! Trefferregel (ADR-0054 Regel 2, E1, E3; B0736): ob eine Nachricht einen Wartenden weckt und mit welchem Anlass.
//! Rein — Nachholen und Bus benutzen sie beide.
//!
//! Nie trifft die eigene Nachricht des Wartenden und nie eine gelöschte. Ohne Filter trifft jede sichtbare
//! Nachricht; mit Filter genügt **ein** passender Teil (Vereinigung). Direktgespräche treffen erst, wenn ihre
//! Beteiligten bekannt sind (I0012, B0747), bis dahin nie.

use super::erwaehnung::erwaehnte_namen;
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
    let anlass = anlaesse(nachricht, wartender);
    let im_channel = filter.channels.contains(channel);
    let erwaehnt = filter.erwaehnung
        && (anlass.contains(&Anlass::Erwaehnung) || anlass.contains(&Anlass::Antwort));
    (filter.ohne_angabe() || im_channel || erwaehnt).then_some(anlass)
}

/// Die Fakten einer fremden Nachricht für den Wartenden, aufsteigend (Regel 2): ob sie seinen Namen erwähnt und ob
/// sie einer Nachricht von ihm antwortet — über die Kennung im Bezug der Metazeile, ohne das Original nachzuschlagen
/// (ADR-0053 Regel 9).
fn anlaesse(nachricht: &Nachricht, wartender: &Kennung) -> Vec<Anlass> {
    let mut anlass = Vec::new();
    let name = wartender.name();
    if erwaehnte_namen(&nachricht.text)
        .iter()
        .any(|erwaehnt| erwaehnt == name)
    {
        anlass.push(Anlass::Erwaehnung);
    }
    if nachricht
        .bezug
        .as_ref()
        .is_some_and(|bezug| &bezug.von == wartender)
    {
        anlass.push(Anlass::Antwort);
    }
    anlass
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

    fn mit_text(von: &str, text: &str) -> Nachricht {
        Nachricht {
            text: text.to_string(),
            ..nachricht(von)
        }
    }

    fn erwaehnung(namen: &[&str]) -> Filter {
        Filter {
            erwaehnung: true,
            ..channels(namen)
        }
    }

    /// W2b (B0742): Trefferregel Erwähnung — `erwaehnung` gesetzt, Vereinigung mit `channel`, eigene Nachricht,
    /// Sammel-Erwähnungen (A1); `anlass` nennt die Erwähnung als Fakt, auch ohne Schalter (Regel 2).
    #[test]
    fn die_trefferregel_der_erwaehnung_als_tabelle() {
        let a = Gespraech::channel("a");
        let b = Gespraech::channel("b");
        let nur_erwaehnung = erwaehnung(&[]);
        let a_oder_erwaehnung = erwaehnung(&["a"]);
        let ohne = Filter::default();
        let nur_a = channels(&["a"]);
        let kein: Option<Vec<Anlass>> = None;
        let leer = Some(Vec::new());
        let erwaehnt = Some(vec![Anlass::Erwaehnung]);
        let faelle = [
            (
                "erwaehnung, @claude in b",
                &b,
                mit_text(ANNA, "@claude bitte prüfen"),
                &nur_erwaehnung,
                erwaehnt.clone(),
            ),
            (
                "erwaehnung, ohne Erwähnung in b",
                &b,
                mit_text(ANNA, "bitte prüfen"),
                &nur_erwaehnung,
                kein.clone(),
            ),
            (
                "erwaehnung, @claudex in b",
                &b,
                mit_text(ANNA, "@claudex hi"),
                &nur_erwaehnung,
                kein.clone(),
            ),
            (
                "channel=a&erwaehnung, @claude in b",
                &b,
                mit_text(ANNA, "hi @Claude."),
                &a_oder_erwaehnung,
                erwaehnt.clone(),
            ),
            (
                "channel=a&erwaehnung, ohne Erwähnung in b",
                &b,
                mit_text(ANNA, "Hallo"),
                &a_oder_erwaehnung,
                kein.clone(),
            ),
            (
                "channel=a&erwaehnung, ohne Erwähnung in a",
                &a,
                mit_text(ANNA, "Hallo"),
                &a_oder_erwaehnung,
                leer.clone(),
            ),
            (
                "channel=a&erwaehnung, @claude in a",
                &a,
                mit_text(ANNA, "@claude"),
                &a_oder_erwaehnung,
                erwaehnt.clone(),
            ),
            (
                "erwaehnung, eigene mit eigenem Namen",
                &b,
                mit_text(CLAUDE, "@claude merkt sich das"),
                &nur_erwaehnung,
                kein.clone(),
            ),
            (
                "channel=a&erwaehnung, eigene mit eigenem Namen in a",
                &a,
                mit_text(CLAUDE, "@claude"),
                &a_oder_erwaehnung,
                kein.clone(),
            ),
            (
                "erwaehnung, @alle (A1)",
                &b,
                mit_text(ANNA, "@alle bitte lesen"),
                &nur_erwaehnung,
                kein.clone(),
            ),
            (
                "erwaehnung, @hier (A1)",
                &b,
                mit_text(ANNA, "@hier kurz melden"),
                &nur_erwaehnung,
                kein.clone(),
            ),
            (
                "erwaehnung, gelöscht",
                &b,
                Nachricht {
                    text: "@claude".to_string(),
                    ..geloescht(ANNA)
                },
                &nur_erwaehnung,
                kein.clone(),
            ),
            (
                "erwaehnung, Direktgespräch",
                &Gespraech::Direkt("anna".to_string()),
                mit_text(ANNA, "@claude"),
                &nur_erwaehnung,
                kein.clone(),
            ),
            (
                "ohne Filter, @claude in b: Anlass als Fakt",
                &b,
                mit_text(ANNA, "@claude"),
                &ohne,
                erwaehnt.clone(),
            ),
            (
                "channel=a, @claude in a: Anlass als Fakt",
                &a,
                mit_text(ANNA, "@claude"),
                &nur_a,
                erwaehnt.clone(),
            ),
            (
                "channel=a, @claude in b: ohne Schalter kein Treffer",
                &b,
                mit_text(ANNA, "@claude"),
                &nur_a,
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

    /// Die Erwähnung trifft den Namen, nicht die Herkunft (ADR-0054 Regel 6, ADR-0013): ein gleichnamiger
    /// Wartender anderer Herkunft wird von `@claude` geweckt — auch `@claude@127.0.0.1` nennt nur den Namen.
    #[test]
    fn die_erwaehnung_trifft_den_namen_jeder_herkunft() {
        let b = Gespraech::channel("b");
        let anderswo = kennung("claude@10.0.0.5");
        for text in ["@claude", "@claude@127.0.0.1"] {
            assert_eq!(
                treffer(&b, &mit_text(ANNA, text), &anderswo, &erwaehnung(&[])),
                Some(vec![Anlass::Erwaehnung]),
                "{text}"
            );
        }
    }

    /// Eine Antwort von `von` auf eine Nachricht von `original` (Bezug aus der Metazeile, ADR-0053 Regel 1).
    fn antwort(von: &str, original: &str, text: &str) -> Nachricht {
        Nachricht {
            bezug: Some(super::super::logformat::Antwortbezug {
                zeit: Zeit::parse_from_rfc3339("2026-10-30T11:00:00.000+01:00").expect("Zeit"),
                von: kennung(original),
            }),
            ..mit_text(von, text)
        }
    }

    /// P13 (B0712, R00047 F0158; ADR-0054 Regel 2, ADR-0053 Regel 9): eine Antwort auf eine Nachricht des Wartenden
    /// trägt den Anlass `antwort` und weckt unter `erwaehnung` wie eine Erwähnung — über die Kennung im Bezug, nie
    /// die eigene Antwort; gleichnamig anderer Herkunft ist nicht der Beantwortete.
    #[test]
    fn die_trefferregel_der_antwort_als_tabelle() {
        let b = Gespraech::channel("b");
        let nur_erwaehnung = erwaehnung(&[]);
        let ohne = Filter::default();
        let nur_a = channels(&["a"]);
        let kein: Option<Vec<Anlass>> = None;
        let beantwortet = Some(vec![Anlass::Antwort]);
        let faelle = [
            (
                "erwaehnung, Antwort von anna auf claude",
                antwort(ANNA, CLAUDE, "Erledigt"),
                &nur_erwaehnung,
                beantwortet.clone(),
            ),
            (
                "erwaehnung, Antwort von anna auf codex",
                antwort(ANNA, "codex@127.0.0.1", "Erledigt"),
                &nur_erwaehnung,
                kein.clone(),
            ),
            (
                "erwaehnung, Antwort von claude auf sich (Z12)",
                antwort(CLAUDE, CLAUDE, "Nachtrag"),
                &nur_erwaehnung,
                kein.clone(),
            ),
            (
                "erwaehnung, Antwort auf gleichnamig anderer Herkunft",
                antwort(ANNA, "claude@10.0.0.5", "Erledigt"),
                &nur_erwaehnung,
                kein.clone(),
            ),
            (
                "erwaehnung, Antwort auf claude mit @claude",
                antwort(ANNA, CLAUDE, "@claude erledigt"),
                &nur_erwaehnung,
                Some(vec![Anlass::Erwaehnung, Anlass::Antwort]),
            ),
            (
                "ohne Filter, Antwort auf claude: Anlass als Fakt",
                antwort(ANNA, CLAUDE, "Erledigt"),
                &ohne,
                beantwortet.clone(),
            ),
            (
                "channel=a, Antwort auf claude in b: ohne Schalter kein Treffer",
                antwort(ANNA, CLAUDE, "Erledigt"),
                &nur_a,
                kein.clone(),
            ),
            (
                "erwaehnung, gelöschte Antwort auf claude",
                Nachricht {
                    bezug: antwort(ANNA, CLAUDE, "").bezug,
                    ..geloescht(ANNA)
                },
                &nur_erwaehnung,
                kein.clone(),
            ),
        ];
        for (fall, nachricht, filter, erwartet) in faelle {
            assert_eq!(
                treffer(&b, &nachricht, &kennung(CLAUDE), filter),
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
