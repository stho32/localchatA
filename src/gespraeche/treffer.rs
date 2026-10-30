//! Trefferregel (ADR-0054 Regel 2, E1, E3; B0736): ob eine Nachricht einen Wartenden weckt und mit welchem Anlass.
//! Rein — Nachholen und Bus benutzen sie beide.
//!
//! Nie trifft die eigene Nachricht des Wartenden und nie eine gelöschte. Ohne Filter trifft jede sichtbare
//! Nachricht; mit Filter genügt **ein** passender Teil (Vereinigung). Ein Gespräch, das der Wartende nicht sehen
//! darf (fremdes Direktgespräch), trifft nie — die Sichtbarkeit geht jedem Anlass voraus.

use super::erwaehnung::erwaehnt_name;
use super::{Aenderungszustand, Gespraech, Kennung, Nachricht, sichtbar_fuer};
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
///
/// Sichtbarkeit zuerst (ADR-0055 Regel 7, E13): was der Wartende nicht sehen darf, weckt ihn nie — auch nicht
/// durch eine Erwähnung oder eine Antwort darin.
pub fn treffer(
    gespraech: &Gespraech,
    nachricht: &Nachricht,
    wartender: &Kennung,
    filter: &Filter,
) -> Option<Vec<Anlass>> {
    if !sichtbar_fuer(gespraech, wartender, false) {
        return None;
    }
    let geloescht = nachricht
        .geaendert
        .is_some_and(|geaendert| geaendert.zustand == Aenderungszustand::Geloescht);
    if &nachricht.von == wartender || geloescht {
        return None;
    }
    let anlass = anlaesse(gespraech, nachricht, wartender);
    let genannt = match gespraech {
        Gespraech::Channel(channel) => filter.channels.contains(channel),
        Gespraech::Direkt(_) => filter.direkt,
    };
    let erwaehnt = filter.erwaehnung
        && (anlass.contains(&Anlass::Erwaehnung) || anlass.contains(&Anlass::Antwort));
    (filter.ohne_angabe() || genannt || erwaehnt).then_some(anlass)
}

/// Die Fakten einer fremden Nachricht für den Wartenden, aufsteigend (Regel 2): ob sie in seinem Direktgespräch
/// steht, seinen Namen erwähnt und ob sie einer Nachricht von ihm antwortet — über die Kennung im Bezug der Metazeile, ohne das Original nachzuschlagen
/// (ADR-0053 Regel 9).
fn anlaesse(gespraech: &Gespraech, nachricht: &Nachricht, wartender: &Kennung) -> Vec<Anlass> {
    let mut anlass = Vec::new();
    if matches!(gespraech, Gespraech::Direkt(_)) {
        anlass.push(Anlass::Direkt);
    }
    let name = wartender.name();
    if erwaehnt_name(&nachricht.text, name) {
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
    use crate::gespraeche::Direktpaar;
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
        let direkt = Gespraech::Direkt(Direktpaar::fuer_test("anna@10.0.0.7", "ben@10.0.0.8"));
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
                &Gespraech::Direkt(Direktpaar::fuer_test("anna@10.0.0.7", "ben@10.0.0.8")),
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

    /// Median in µs von `wiederholungen` Aufrufen der Trefferregel für `claude` mit `erwaehnung` auf `text`.
    fn median_in_us(text: &str, wiederholungen: usize) -> f64 {
        let b = Gespraech::channel("b");
        let nachricht = mit_text(ANNA, text);
        let wartender = kennung(CLAUDE);
        let filter = erwaehnung(&[]);
        let mut dauern: Vec<f64> = (0..wiederholungen)
            .map(|_| {
                let beginn = std::time::Instant::now();
                std::hint::black_box(treffer(
                    std::hint::black_box(&b),
                    std::hint::black_box(&nachricht),
                    &wartender,
                    &filter,
                ));
                beginn.elapsed().as_secs_f64() * 1e6
            })
            .collect();
        dauern.sort_by(f64::total_cmp);
        dauern[dauern.len() / 2]
    }

    /// Nacharbeit security-reviewer #17 Befund 2: erwähnungsdicht 16 KiB „@a “ × 5.461, 2.000 Aufrufe → Median
    /// ≤ 20 µs (vorher 464 µs: ein String je `@`); Gegenprobe 16 KiB „x“ ≤ 5 µs. Eine Lastbeobachtung im Release,
    /// kein Gate: die Wanduhr schwankt mit der Last des Rechners; den Vertrag „keine Allokation je `@`“ hält
    /// `tests/erwaehnung_allokationsfrei.rs` zählend. Gefahren mit
    /// `cargo test --release --lib erwaehnungsdicht -- --ignored --nocapture`.
    #[test]
    #[ignore = "Lastbeobachtung (Wanduhr): cargo test --release --lib erwaehnungsdicht -- --ignored --nocapture"]
    fn erwaehnungsdicht_bleibt_die_trefferregel_unter_20_us() {
        let dicht = "@a ".repeat(5_461);
        let ohne = "x".repeat(16_384);

        let dicht_us = median_in_us(&dicht, 2_000);
        let ohne_us = median_in_us(&ohne, 2_000);
        println!("erwähnungsdicht {dicht_us:.1} µs, ohne @ {ohne_us:.1} µs");

        assert!(dicht_us <= 20.0, "erwähnungsdicht Median {dicht_us:.1} µs");
        assert!(ohne_us <= 5.0, "ohne @ Median {ohne_us:.1} µs");
    }

    fn direkt_filter(channels_: &[&str]) -> Filter {
        Filter {
            direkt: true,
            ..channels(channels_)
        }
    }

    /// W2c (B0747, ADR-0054 Regel 2 und 4, ADR-0055 Regel 7): Trefferregel Direkt für `mia` im Gespräch
    /// `claude` ↔ `mia`. Sichtbarkeit zuerst (E13): ein Dritter wird nie getroffen, auch nicht ohne Filter und nicht
    /// von einer Erwähnung; die eigene Nachricht nie; `direkt` und `channel` bilden eine Vereinigung.
    #[test]
    fn die_trefferregel_des_direktgespraechs_als_tabelle() {
        const MIA: &str = "mia@127.0.0.1";
        const SHOFF: &str = "shoff@127.0.0.1";
        let gespraech = Gespraech::Direkt(Direktpaar::fuer_test(CLAUDE, MIA));
        let channel_a = Gespraech::channel("a");
        let ohne = Filter::default();
        let nur_direkt = direkt_filter(&[]);
        let nur_a = channels(&["a"]);
        let a_oder_direkt = direkt_filter(&["a"]);
        let nur_erwaehnung = erwaehnung(&[]);
        let kein: Option<Vec<Anlass>> = None;
        let direkt = Some(vec![Anlass::Direkt]);
        let leer = Some(Vec::new());
        let faelle = [
            (
                "direkt, claude → mia, für mia",
                &gespraech,
                mit_text(CLAUDE, "Hallo"),
                MIA,
                &nur_direkt,
                direkt.clone(),
            ),
            (
                "direkt, claude → mia, für shoff (Dritter)",
                &gespraech,
                mit_text(CLAUDE, "Hallo"),
                SHOFF,
                &nur_direkt,
                kein.clone(),
            ),
            (
                "ohne Filter, claude → mia, für shoff (Dritter)",
                &gespraech,
                mit_text(CLAUDE, "Hallo"),
                SHOFF,
                &ohne,
                kein.clone(),
            ),
            (
                "direkt, claude → mia, für claude (eigene)",
                &gespraech,
                mit_text(CLAUDE, "Hallo"),
                CLAUDE,
                &nur_direkt,
                kein.clone(),
            ),
            (
                "ohne Filter, claude → mia, für claude (eigene)",
                &gespraech,
                mit_text(CLAUDE, "Hallo"),
                CLAUDE,
                &ohne,
                kein.clone(),
            ),
            (
                "ohne Filter, claude → mia, für mia",
                &gespraech,
                mit_text(CLAUDE, "Hallo"),
                MIA,
                &ohne,
                direkt.clone(),
            ),
            (
                "ohne Filter, Channel-Nachricht, für mia",
                &channel_a,
                mit_text(CLAUDE, "Hallo"),
                MIA,
                &ohne,
                leer.clone(),
            ),
            (
                "direkt, Channel-Nachricht, für mia",
                &channel_a,
                mit_text(CLAUDE, "Hallo"),
                MIA,
                &nur_direkt,
                kein.clone(),
            ),
            (
                "channel=a, Direktnachricht, für mia",
                &gespraech,
                mit_text(CLAUDE, "Hallo"),
                MIA,
                &nur_a,
                kein.clone(),
            ),
            (
                "channel=a&direkt, Direktnachricht, für mia (Vereinigung)",
                &gespraech,
                mit_text(CLAUDE, "Hallo"),
                MIA,
                &a_oder_direkt,
                direkt.clone(),
            ),
            (
                "channel=a&direkt, Channel a, für mia (Vereinigung)",
                &channel_a,
                mit_text(CLAUDE, "Hallo"),
                MIA,
                &a_oder_direkt,
                leer.clone(),
            ),
            (
                "direkt, gelöscht, für mia",
                &gespraech,
                geloescht(CLAUDE),
                MIA,
                &nur_direkt,
                kein.clone(),
            ),
            (
                "erwaehnung, ohne Erwähnung, Direktnachricht: Anlass als Fakt, kein Schalter",
                &gespraech,
                mit_text(CLAUDE, "Hallo"),
                MIA,
                &nur_erwaehnung,
                kein.clone(),
            ),
            (
                "erwaehnung, @mia im eigenen Gespräch: beides",
                &gespraech,
                mit_text(CLAUDE, "@mia schau mal"),
                MIA,
                &nur_erwaehnung,
                Some(vec![Anlass::Direkt, Anlass::Erwaehnung]),
            ),
            (
                "ohne Filter, @mia im eigenen Gespräch: beides",
                &gespraech,
                mit_text(CLAUDE, "@mia schau mal"),
                MIA,
                &ohne,
                Some(vec![Anlass::Direkt, Anlass::Erwaehnung]),
            ),
            (
                "E13: @shoff im fremden Gespräch weckt nie, auch nicht unter erwaehnung",
                &gespraech,
                mit_text(CLAUDE, "@shoff komm"),
                SHOFF,
                &nur_erwaehnung,
                kein.clone(),
            ),
            (
                "E13: @shoff im fremden Gespräch weckt nie, auch nicht ohne Filter",
                &gespraech,
                mit_text(CLAUDE, "@shoff komm"),
                SHOFF,
                &ohne,
                kein.clone(),
            ),
            (
                "E13: Antwort auf shoff im fremden Gespräch weckt nie",
                &gespraech,
                antwort(CLAUDE, SHOFF, "Erledigt"),
                SHOFF,
                &nur_erwaehnung,
                kein.clone(),
            ),
            (
                "erwaehnung, Antwort auf mia im eigenen Gespräch: direkt und antwort",
                &gespraech,
                antwort(CLAUDE, MIA, "Erledigt"),
                MIA,
                &nur_erwaehnung,
                Some(vec![Anlass::Direkt, Anlass::Antwort]),
            ),
        ];
        for (fall, gespraech, nachricht, wartender, filter, erwartet) in faelle {
            assert_eq!(
                treffer(gespraech, &nachricht, &kennung(wartender), filter),
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
