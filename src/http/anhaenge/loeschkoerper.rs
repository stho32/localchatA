//! Löschkörper prüfen (B0505, ADR-0046 Ergänzung „F1′“ Regel 4, Ergänzung „Karte #9 Security-Gate“ L9): `{ids:
//! [<id>, …], bestaetigt?: <bool>}` — abgewiesen, nie umgeformt. Reihenfolge: Feld doppelt → weiteres Feld → `leer`
//! → `anzahl` → `id` → `doppelt` → `bestaetigt`. `stelle` zählt ab 0 wie der Index der Liste. Rein.
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "die Route Löschen (B0508) nutzt die Prüfung")
)]

use super::super::felder::{Felder, doppelt, wert};
use crate::gespraeche::anhangid::Anhangid;
use serde_json::{Value, json};
use std::collections::HashSet;

/// Höchstens so viele IDs je Anfrage.
pub const HOECHSTENS_IDS: usize = 500;

/// Ein geprüfter Auftrag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Auftrag {
    pub ids: Vec<Anhangid>,
    pub bestaetigt: bool,
}

/// Eine 422 mit Körper und Regelschlüssel fürs Anfrageprotokoll.
#[derive(Debug, Clone, PartialEq)]
pub struct Abweisung {
    pub koerper: Value,
    pub regel: &'static str,
}

fn abweisung(regel: &'static str, koerper: Value) -> Abweisung {
    Abweisung { koerper, regel }
}

pub fn auftrag_pruefen(felder: Option<&Felder>) -> Result<Auftrag, Abweisung> {
    let Some(felder) = felder else {
        return Err(abweisung(
            "feld_unbekannt",
            json!({
                "fehler": r#"Der Körper braucht ein JSON-Objekt, zum Beispiel {"ids":["<id>"]}."#,
                "regel": "feld",
            }),
        ));
    };
    if let Some(feld) = doppelt(felder) {
        return Err(abweisung(
            "feld_doppelt",
            json!({
                "fehler": format!("Das Feld {feld} steht zweimal im Körper — jedes Feld nur einmal schicken."),
                "regel": "feld",
                "feld": feld,
                "grund": "doppelt",
            }),
        ));
    }
    if let Some((feld, _)) = felder
        .iter()
        .find(|(feld, _)| feld != "ids" && feld != "bestaetigt")
    {
        return Err(abweisung(
            "feld_unbekannt",
            json!({
                "fehler": format!("Unbekanntes Feld {feld} — erlaubt sind ids und bestaetigt."),
                "regel": "feld",
                "feld": feld,
            }),
        ));
    }
    let liste = match wert(felder, "ids").and_then(Value::as_array) {
        Some(liste) if !liste.is_empty() => liste,
        _ => {
            return Err(abweisung(
                "leer",
                json!({
                    "fehler": "ids braucht eine Liste mit mindestens einer Anhang-ID.",
                    "regel": "leer",
                }),
            ));
        }
    };
    if liste.len() > HOECHSTENS_IDS {
        return Err(abweisung(
            "anzahl",
            json!({
                "fehler": format!("Höchstens {HOECHSTENS_IDS} Anhänge je Anfrage — die Liste in Teilen schicken."),
                "regel": "anzahl",
                "hoechstens": HOECHSTENS_IDS,
            }),
        ));
    }
    let mut ids = Vec::with_capacity(liste.len());
    for (stelle, eintrag) in liste.iter().enumerate() {
        let Some(id) = eintrag.as_str().and_then(Anhangid::lesen) else {
            return Err(abweisung(
                "id",
                json!({
                    "fehler": format!("Eintrag {stelle} ist keine Anhang-ID (26 Zeichen a–z und 2–7)."),
                    "regel": "id",
                    "stelle": stelle,
                }),
            ));
        };
        ids.push(id);
    }
    let mut gesehen = HashSet::with_capacity(ids.len());
    if let Some(stelle) = ids.iter().position(|id| !gesehen.insert(id.clone())) {
        return Err(abweisung(
            "doppelt",
            json!({
                "fehler": format!("Eintrag {stelle} steht schon früher in der Liste — jede ID nur einmal."),
                "regel": "doppelt",
                "stelle": stelle,
            }),
        ));
    }
    let bestaetigt = match wert(felder, "bestaetigt") {
        None => false,
        Some(Value::Bool(wert)) => *wert,
        Some(_) => {
            return Err(abweisung(
                "bestaetigt",
                json!({
                    "fehler": "bestaetigt ist true oder false.",
                    "regel": "bestaetigt",
                }),
            ));
        }
    };
    Ok(Auftrag { ids, bestaetigt })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::felder::felder_lesen;

    const A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaa";
    const B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn pruefen(roh: &str) -> Result<Auftrag, Abweisung> {
        auftrag_pruefen(felder_lesen(roh.as_bytes()).expect("JSON").as_ref())
    }

    fn regel(roh: &str) -> (&'static str, Value) {
        let abgewiesen = pruefen(roh).expect_err("abgewiesen");
        (abgewiesen.regel, abgewiesen.koerper)
    }

    #[test]
    fn ein_gueltiger_auftrag() {
        assert_eq!(
            pruefen(&format!(r#"{{"ids":["{A}","{B}"],"bestaetigt":true}}"#)),
            Ok(Auftrag {
                ids: vec![
                    Anhangid::lesen(A).expect("A"),
                    Anhangid::lesen(B).expect("B")
                ],
                bestaetigt: true,
            })
        );
        assert!(
            !pruefen(&format!(r#"{{"ids":["{A}"]}}"#))
                .expect("ohne")
                .bestaetigt
        );
    }

    #[test]
    fn die_abweisungen_in_ihrer_reihenfolge() {
        let viele: Vec<String> = (0..501).map(|_| format!("\"{A}\"")).collect();
        let faelle = [
            (r#"{"ids":[],"bestaetigt":true}"#.to_string(), "leer"),
            (r#"{"bestaetigt":true}"#.to_string(), "leer"),
            (r#"{"ids":"x"}"#.to_string(), "leer"),
            (format!(r#"{{"ids":[{}]}}"#, viele.join(",")), "anzahl"),
            (format!(r#"{{"ids":["{A}","{}"]}}"#, A.to_uppercase()), "id"),
            (format!(r#"{{"ids":["{}"]}}"#, &A[..25]), "id"),
            (format!(r#"{{"ids":["{A}a"]}}"#), "id"),
            (r#"{"ids":[1]}"#.to_string(), "id"),
            (r#"{"ids":[null]}"#.to_string(), "id"),
            (r#"{"ids":[{}]}"#.to_string(), "id"),
            (format!(r#"{{"ids":["{A}","{B}","{A}"]}}"#), "doppelt"),
            (
                format!(r#"{{"ids":["{A}"],"bestaetigt":"ja"}}"#),
                "bestaetigt",
            ),
            (
                format!(r#"{{"ids":["{A}"],"bestaetigt":null}}"#),
                "bestaetigt",
            ),
            (format!(r#"{{"ids":["{A}"],"x":1}}"#), "feld_unbekannt"),
            (
                format!(r#"{{"ids":["{A}"],"ids":["{B}"]}}"#),
                "feld_doppelt",
            ),
            ("[1]".to_string(), "feld_unbekannt"),
        ];
        for (roh, erwartet) in faelle {
            assert_eq!(regel(&roh).0, erwartet, "{roh}");
        }
        assert_eq!(
            regel(&format!(r#"{{"ids":["{A}","{}"]}}"#, A.to_uppercase())).1["stelle"],
            1
        );
        assert_eq!(
            regel(&format!(r#"{{"ids":["{A}","{B}","{A}"]}}"#)).1["stelle"],
            2
        );
        assert_eq!(
            regel(&format!(r#"{{"ids":[{}]}}"#, viele.join(","))).1["hoechstens"],
            500
        );
    }

    /// L9: `"ids"` zweimal → genau 422 `feld`/`doppelt`, nie die letzte Fassung (Mutante „letzter Wert gewinnt“).
    #[test]
    fn ein_doppelter_schluessel_ist_immer_feld_doppelt() {
        let (regel, koerper) = regel(&format!(
            r#"{{"ids":["{A}"],"ids":["{B}"],"bestaetigt":true}}"#
        ));
        assert_eq!(regel, "feld_doppelt");
        assert_eq!(
            koerper,
            json!({
                "fehler": "Das Feld ids steht zweimal im Körper — jedes Feld nur einmal schicken.",
                "regel": "feld",
                "feld": "ids",
                "grund": "doppelt",
            })
        );
    }
}
