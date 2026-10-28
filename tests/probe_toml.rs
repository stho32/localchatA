//! Probe für `toml` (Skill dependency-probe, ADR-0022): die Annahmen, auf denen das Einstellungsformat steht.
//!
//! Annahmen: (1) eine Tabelle mit Text-Werten geht als Text hin und unverändert zurück; (2) ein unbekannter
//! Schlüssel bleibt in `toml::Table` erhalten; (3) Fehler nennen eine Byte-Spanne — offene Zeichenkette,
//! doppelter Schlüssel; (4) ein Wert falschen Typs ist kein Lesefehler der Tabelle, sondern erst beim Deuten
//! sichtbar; (5) `Spanned` nennt die Stelle eines Werts.

use toml::{Spanned, Table, Value};

#[test]
fn probe_tabelle_mit_text_geht_hin_und_zurueck_und_behaelt_unbekannte_schluessel() {
    let mut tabelle: Table = "farbe = \"rot\"\n".parse().expect("lesbar");
    tabelle.insert(
        "angelegt".into(),
        Value::String("2026-10-28T10:15:00.123+01:00".into()),
    );
    tabelle.insert(
        "angelegt_von".into(),
        Value::String("Pr\u{fc}fung \"x\" \u{2194}".into()),
    );

    let text = toml::to_string(&tabelle).expect("schreibbar");
    let zurueck: Table = text.parse().expect("wieder lesbar");

    assert_eq!(zurueck, tabelle);
    assert_eq!(zurueck["farbe"].as_str(), Some("rot"));
}

#[test]
fn probe_fehler_nennen_eine_byte_spanne() {
    let offen = "a = 1\nangelegt = \""
        .parse::<Table>()
        .expect_err("offene Zeichenkette");
    let spanne = offen.span().expect("Spanne");
    assert!(spanne.start >= 6, "{spanne:?}: {offen}");

    let doppelt = "a = 1\na = 2\n"
        .parse::<Table>()
        .expect_err("doppelter Schlüssel");
    assert!(doppelt.span().expect("Spanne").start >= 6, "{doppelt}");
}

#[test]
fn probe_falscher_typ_zeigt_sich_erst_beim_deuten_mit_stelle() {
    #[derive(serde::Deserialize)]
    struct Roh {
        angelegt: Option<Spanned<Value>>,
    }
    let roh: Roh = toml::from_str("\n\nangelegt = 5\nanderes = true\n").expect("lesbar");

    let angelegt = roh.angelegt.expect("vorhanden");
    assert_eq!(angelegt.get_ref().as_str(), None);
    assert_eq!(angelegt.get_ref().as_integer(), Some(5));
    assert_eq!(angelegt.span().start, 13);
}
