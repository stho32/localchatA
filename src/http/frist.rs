//! Frist setzen (R00008 F0033, ADR-0031): `PUT /api/channels/{name}/frist {tage, bestaetigt?}`.

use crate::gespraeche::frist_in_spanne;
use serde_json::Value;

/// `tage` ist keine ganze Zahl von 1 bis 3650 — abgewiesen, nie umgeformt (ADR-0031 Regel 2.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Regelfehler;

/// Der JSON-Wert `tage` als Frist: nur eine ganze Zahl (`"7"`, `7.0`, `7.5`, `null` und Fehlen nicht) in der Spanne.
pub fn frist_bilden(wert: Option<&Value>) -> Result<u32, Regelfehler> {
    wert.and_then(Value::as_i64)
        .and_then(frist_in_spanne)
        .ok_or(Regelfehler)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn gebildet(koerper: Value) -> Result<u32, Regelfehler> {
        frist_bilden(koerper.get("tage"))
    }

    #[test]
    fn eins_und_3650_gelten_als_spanne() {
        assert_eq!(gebildet(json!({ "tage": 1 })), Ok(1));
        assert_eq!(gebildet(json!({ "tage": 7 })), Ok(7));
        assert_eq!(gebildet(json!({ "tage": 3650 })), Ok(3650));
    }

    #[test]
    fn null_3651_und_negative_liegen_ausserhalb() {
        for tage in [json!(0), json!(3651), json!(-1), json!(i64::MAX), json!(u64::MAX)] {
            assert_eq!(gebildet(json!({ "tage": tage })), Err(Regelfehler), "{tage}");
        }
    }

    #[test]
    fn keine_ganze_zahl_ist_ein_regelfehler_und_wird_nicht_umgeformt() {
        let koerper: Value = serde_json::from_str(r#"{"tage":7.0}"#).expect("JSON");
        assert_eq!(gebildet(koerper), Err(Regelfehler));
        for tage in [json!("7"), json!(7.5), Value::Null, json!([7]), json!({ "n": 7 }), json!(true)] {
            assert_eq!(gebildet(json!({ "tage": tage })), Err(Regelfehler), "{tage}");
        }
        assert_eq!(gebildet(json!({})), Err(Regelfehler));
    }
}
