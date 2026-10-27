//! Beitrittswunsch prüfen (B0046): JSON `{name, art?}` → geprüfter Name und Art oder eine Abweisung (422).

use crate::name::Name;
use serde::Serialize;
use serde_json::Value;

/// Selbstauskunft beim Beitreten, nur für die Darstellung (ADR-0012); fehlt sie, gilt `agent`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Art {
    Mensch,
    Agent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Beitrittswunsch {
    pub name: Name,
    pub art: Art,
}

/// Warum ein Wunsch nicht angenommen wird: Text für Menschen, die verletzte Regel, wenn möglich ein Vorschlag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Abweisung {
    pub fehler: String,
    pub regel: &'static str,
    pub vorschlag: Option<Name>,
}

/// Für spätere Sammel-Erwähnungen (I0013) beim Beitreten nicht wählbar.
const RESERVIERTE_NAMEN: [&str; 2] = ["alle", "hier"];
const REGEL_RESERVIERT: &str = "alle und hier sind reserviert";
const REGEL_ART: &str = "art ist mensch oder agent";
const REGEL_FORM: &str = "ein JSON-Objekt mit name (Text) und art (mensch oder agent, optional)";

impl Beitrittswunsch {
    /// Name nach der Namensregel und nicht reserviert, `art` fehlt oder ist `mensch` | `agent`.
    pub fn pruefen(koerper: &Value) -> Result<Beitrittswunsch, Abweisung> {
        let Some(text) = koerper.get("name").and_then(Value::as_str) else {
            return Err(Abweisung {
                fehler: "Der Beitritt braucht einen Namen als Text.".into(),
                regel: REGEL_FORM,
                vorschlag: None,
            });
        };
        let name = Name::bilden(text).map_err(|fehler| Abweisung {
            fehler: format!("Der Name {text:?} ist nicht erlaubt — {}.", fehler.regel),
            regel: fehler.regel,
            vorschlag: fehler.vorschlag,
        })?;
        if RESERVIERTE_NAMEN.contains(&name.als_text()) {
            return Err(Abweisung {
                fehler: format!("Der Name {name} ist reserviert."),
                regel: REGEL_RESERVIERT,
                vorschlag: None,
            });
        }
        let art = match koerper.get("art") {
            None => Art::Agent,
            Some(art) if art == "mensch" => Art::Mensch,
            Some(art) if art == "agent" => Art::Agent,
            Some(art) => {
                return Err(Abweisung {
                    fehler: format!("Die Art {art} gibt es nicht — mensch oder agent."),
                    regel: REGEL_ART,
                    vorschlag: None,
                });
            }
        };
        Ok(Beitrittswunsch { name, art })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::name::NAMENSREGEL;
    use serde_json::json;

    fn wunsch(koerper: Value) -> Beitrittswunsch {
        Beitrittswunsch::pruefen(&koerper)
            .unwrap_or_else(|abweisung| panic!("{koerper}: {abweisung:?}"))
    }

    fn abweisung(koerper: Value) -> Abweisung {
        match Beitrittswunsch::pruefen(&koerper) {
            Ok(wunsch) => panic!("{koerper} angenommen: {wunsch:?}"),
            Err(abweisung) => {
                assert!(!abweisung.fehler.is_empty(), "{abweisung:?}");
                abweisung
            }
        }
    }

    #[test]
    fn name_ohne_art_ist_ein_agent_mit_art_mensch_ein_mensch() {
        let agent = wunsch(json!({"name": "claude"}));
        let mensch = wunsch(json!({"name": "shoff", "art": "mensch"}));
        let ausdruecklich = wunsch(json!({"name": "x1", "art": "agent"}));

        assert_eq!((agent.name.als_text(), agent.art), ("claude", Art::Agent));
        assert_eq!((mensch.name.als_text(), mensch.art), ("shoff", Art::Mensch));
        assert_eq!(ausdruecklich.art, Art::Agent);
    }

    #[test]
    fn ein_name_gegen_die_regel_wird_mit_regel_und_vorschlag_abgewiesen_nie_umgeformt() {
        let jörg = abweisung(json!({"name": "Jörg"}));
        let unterstriche = abweisung(json!({"name": "___"}));

        assert_eq!(jörg.regel, NAMENSREGEL);
        assert_eq!(
            jörg.vorschlag.map(|name| name.to_string()),
            Some("joerg".into())
        );
        assert!(jörg.fehler.contains("Jörg"), "{}", jörg.fehler);
        assert_eq!(unterstriche.regel, NAMENSREGEL);
        assert_eq!(unterstriche.vorschlag, None);
    }

    #[test]
    fn reservierte_namen_werden_abgewiesen_und_als_reserviert_genannt() {
        for name in RESERVIERTE_NAMEN {
            let abgewiesen = abweisung(json!({ "name": name }));
            assert_eq!(abgewiesen.regel, REGEL_RESERVIERT);
            assert!(
                abgewiesen.fehler.contains("reserviert"),
                "{}",
                abgewiesen.fehler
            );
            assert_eq!(abgewiesen.vorschlag, None);
        }
    }

    #[test]
    fn eine_unbekannte_oder_falsch_getippte_art_wird_abgewiesen() {
        for art in [
            json!("roboter"),
            json!("Mensch"),
            json!(""),
            json!(null),
            json!(1),
        ] {
            let abgewiesen = abweisung(json!({"name": "x1", "art": art}));
            assert_eq!(abgewiesen.regel, REGEL_ART, "{art}");
        }
    }

    #[test]
    fn ohne_text_als_name_oder_ohne_objekt_wird_die_form_abgewiesen() {
        for koerper in [
            json!({}),
            json!({"name": 7}),
            json!({"name": null}),
            json!({"name": ["claude"]}),
            json!("claude"),
            json!([]),
        ] {
            assert_eq!(abweisung(koerper.clone()).regel, REGEL_FORM, "{koerper}");
        }
    }
}
