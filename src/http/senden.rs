//! Nachricht senden (F0041; ADR-0034): `POST /api/channels/{name}/nachrichten {text}` mit Sitzung.

use serde_json::Value;

/// Warum ein Sendekörper keinen Text liefert (422 `text` bzw. `feld`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sendekoerperfehler {
    /// Kein Objekt, `text` fehlt oder ist keine Zeichenkette.
    Text,
    /// Ein weiteres Feld neben `text`.
    Feld { feld: String },
}

/// Der Text aus dem Körper `{"text": "…"}` (B0207, ADR-0034 Regel 3.3); jedes weitere Feld wird abgewiesen.
pub fn sendekoerper_lesen(wert: &Value) -> Result<&str, Sendekoerperfehler> {
    let objekt = wert.as_object().ok_or(Sendekoerperfehler::Text)?;
    let text = objekt
        .get("text")
        .and_then(Value::as_str)
        .ok_or(Sendekoerperfehler::Text)?;
    if let Some(feld) = objekt.keys().find(|feld| *feld != "text") {
        return Err(Sendekoerperfehler::Feld { feld: feld.clone() });
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn ein_objekt_mit_text_liefert_den_text() {
        assert_eq!(sendekoerper_lesen(&json!({"text": "Hallo"})), Ok("Hallo"));
        assert_eq!(sendekoerper_lesen(&json!({"text": "  "})), Ok("  "));
    }

    #[test]
    fn ohne_text_als_zeichenkette_ist_es_regel_text() {
        for wert in [
            json!({}),
            json!({"text": 5}),
            json!({"text": null}),
            json!([1]),
            json!("Hallo"),
        ] {
            assert_eq!(
                sendekoerper_lesen(&wert),
                Err(Sendekoerperfehler::Text),
                "{wert}"
            );
        }
    }

    #[test]
    fn ein_weiteres_feld_wird_mit_seinem_namen_abgewiesen() {
        assert_eq!(
            sendekoerper_lesen(&json!({"text": "Hallo", "antwort": "x"})),
            Err(Sendekoerperfehler::Feld {
                feld: "antwort".into()
            })
        );
    }
}
