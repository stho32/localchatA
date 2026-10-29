//! Feldleser (ADR-0046 Ergänzung „Karte #9 Security-Gate“, L9, J1): die Paare der obersten Ebene eines
//! JSON-Objekts in Reihenfolge, Schlüssel dekodiert — damit ein doppelter Schlüssel erkannt wird, statt still den
//! letzten Wert zu nehmen (`serde_json::Value` behält nur den letzten). Rein.

use serde::Deserializer;
use serde::de::{MapAccess, Visitor};
use serde_json::Value;
use std::collections::HashSet;
use std::fmt;

/// Die Paare der obersten Ebene, wie sie im Körper stehen.
pub type Felder = Vec<(String, Value)>;

/// Gültiges JSON, aber kein Objekt (`None`) — oder kein gültiges JSON (`Err`). Ein Objekt wird genau einmal gelesen.
pub fn felder_lesen(roh: &[u8]) -> Result<Option<Felder>, serde_json::Error> {
    if roh.iter().find(|byte| !byte.is_ascii_whitespace()) != Some(&b'{') {
        serde_json::from_slice::<Value>(roh)?;
        return Ok(None);
    }
    let mut leser = serde_json::Deserializer::from_slice(roh);
    let felder = (&mut leser).deserialize_map(PaareSammeln)?;
    leser.end()?;
    Ok(Some(felder))
}

/// Der erste Schlüssel, der ein zweites Mal vorkommt — in einem Durchgang, linear in der Zahl der Schlüssel
/// (ein Körper bis 131 072 Bytes trägt über 15 000 Schlüssel).
pub fn doppelt(felder: &Felder) -> Option<&str> {
    let mut gesehen = HashSet::with_capacity(felder.len());
    felder
        .iter()
        .map(|(schluessel, _)| schluessel.as_str())
        .find(|schluessel| !gesehen.insert(*schluessel))
}

/// Der Wert eines Schlüssels (nach der Prüfung auf Doppelte genau einer).
pub fn wert<'a>(felder: &'a Felder, schluessel: &str) -> Option<&'a Value> {
    felder
        .iter()
        .find(|(name, _)| name == schluessel)
        .map(|(_, wert)| wert)
}

struct PaareSammeln;

impl<'de> Visitor<'de> for PaareSammeln {
    type Value = Felder;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ein JSON-Objekt")
    }

    fn visit_map<M: MapAccess<'de>>(self, mut paare: M) -> Result<Felder, M::Error> {
        let mut felder = Vec::new();
        while let Some(paar) = paare.next_entry::<String, Value>()? {
            felder.push(paar);
        }
        Ok(felder)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn felder(roh: &str) -> Felder {
        felder_lesen(roh.as_bytes()).expect("JSON").expect("Objekt")
    }

    /// L9: `{"ids":["a"],"ids":["b"]}` → beide Paare in Reihenfolge; `doppelt` nennt `ids` — nie die letzte Fassung
    /// allein (Mutante „letzter Wert gewinnt“).
    #[test]
    fn ein_doppelter_schluessel_bleibt_als_zwei_paare_erhalten() {
        let gelesen = felder(r#"{"ids":["a"],"ids":["b"],"bestaetigt":true}"#);
        assert_eq!(
            gelesen,
            vec![
                ("ids".to_string(), json!(["a"])),
                ("ids".to_string(), json!(["b"])),
                ("bestaetigt".to_string(), json!(true)),
            ]
        );
        assert_eq!(doppelt(&gelesen), Some("ids"));
    }

    /// Dekodiert verglichen: `"bytes"` ist `bytes`.
    #[test]
    fn ein_kodiert_doppelter_schluessel_ist_doppelt() {
        let gelesen = felder(r#"{"name":"a","bytes":1,"bytes":2}"#);
        assert_eq!(doppelt(&gelesen), Some("bytes"));
    }

    #[test]
    fn einfache_schluessel_sind_nicht_doppelt() {
        let gelesen = felder(r#"{"name":"a","bytes":1}"#);
        assert_eq!(doppelt(&gelesen), None);
        assert_eq!(wert(&gelesen, "bytes"), Some(&json!(1)));
        assert_eq!(wert(&gelesen, "x"), None);
    }

    #[test]
    fn kein_objekt_und_kaputtes_json_werden_unterschieden() {
        assert_eq!(felder_lesen(b"[1]").expect("JSON"), None);
        assert_eq!(felder_lesen(b"\"x\"").expect("JSON"), None);
        assert!(felder_lesen(b"{\"a\":").is_err());
        assert!(felder_lesen(b"").is_err());
        assert!(felder_lesen(b"{\"a\":1}x").is_err(), "Müll hinter dem Objekt");
        assert!(felder_lesen(b"{\"a\":1}{}").is_err());
        assert_eq!(felder_lesen(b" \n{}").expect("JSON"), Some(vec![]));
    }
}
