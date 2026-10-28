//! Einstellungsformat (A00013, ADR-0022): `einstellungen.toml` eines Channels ↔ geprüfte Werte, rein.
//!
//! Kennt keine Datei, nur Bytes und Text; die Logablage schreibt und liest sie.

use super::Kennung;
use crate::uhr::Zeit;
use chrono::SecondsFormat;
use toml::{Table, Value};

/// Was die Datei über einen Channel sagt; fehlt ein Schlüssel, ist der Wert unbekannt.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Einstellungen {
    pub angelegt: Option<Zeit>,
    pub angelegt_von: Option<Kennung>,
}

/// Die Datei ist nicht lesbar: Zeile (ab 1) und Grund.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Defekt {
    pub zeile: usize,
    pub grund: String,
}

const ANGELEGT: &str = "angelegt";
const ANGELEGT_VON: &str = "angelegt_von";

pub fn lesen(bytes: &[u8]) -> Result<Einstellungen, Defekt> {
    let text = std::str::from_utf8(bytes).map_err(|fehler| Defekt {
        zeile: 0,
        grund: fehler.to_string(),
    })?;
    let tabelle: Table = text.parse().map_err(|fehler: toml::de::Error| Defekt {
        zeile: 0,
        grund: fehler.message().to_string(),
    })?;
    let text_von = |schluessel: &str| tabelle.get(schluessel).and_then(Value::as_str);
    Ok(Einstellungen {
        angelegt: text_von(ANGELEGT).and_then(|text| Zeit::parse_from_rfc3339(text).ok()),
        angelegt_von: text_von(ANGELEGT_VON).and_then(|text| Kennung::bilden(text).ok()),
    })
}

/// Bekannte Schlüssel gesetzt, unbekannte aus `vorher` erhalten (Kommentare nicht, ADR-0022 Regel 4).
/// Ist `vorher` nicht lesbar, beginnt die Tabelle leer.
pub fn schreiben(einstellungen: &Einstellungen, vorher: Option<&[u8]>) -> String {
    let mut tabelle: Table = vorher
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .and_then(|text| text.parse().ok())
        .unwrap_or_default();
    setzen(
        &mut tabelle,
        ANGELEGT,
        einstellungen.angelegt.as_ref().map(zeit_als_text),
    );
    setzen(
        &mut tabelle,
        ANGELEGT_VON,
        einstellungen
            .angelegt_von
            .as_ref()
            .map(|kennung| kennung.als_text().to_string()),
    );
    tabelle.to_string()
}

fn setzen(tabelle: &mut Table, schluessel: &str, wert: Option<String>) {
    match wert {
        Some(wert) => {
            tabelle.insert(schluessel.to_string(), Value::String(wert));
        }
        None => {
            tabelle.remove(schluessel);
        }
    }
}

/// RFC 3339 mit Millisekunden und Versatz, wie an der HTTP-Grenze.
fn zeit_als_text(zeit: &Zeit) -> String {
    zeit.to_rfc3339_opts(SecondsFormat::Millis, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zeit(text: &str) -> Zeit {
        Zeit::parse_from_rfc3339(text).expect("Testzeit")
    }

    fn alpha() -> Einstellungen {
        Einstellungen {
            angelegt: Some(zeit("2026-10-28T10:15:00.123+01:00")),
            angelegt_von: Some(Kennung::bilden("alpha@127.0.0.2").expect("Kennung")),
        }
    }

    #[test]
    fn rundweg_ergibt_dieselben_einstellungen() {
        let text = schreiben(&alpha(), None);

        assert_eq!(lesen(text.as_bytes()), Ok(alpha()));
    }

    #[test]
    fn die_datei_nennt_angelegt_mit_millisekunden_und_den_anleger_ohne_namen() {
        let text = schreiben(&alpha(), None);
        let tabelle: toml::Table = text.parse().expect("TOML");

        assert_eq!(
            tabelle["angelegt"].as_str(),
            Some("2026-10-28T10:15:00.123+01:00")
        );
        assert_eq!(tabelle["angelegt_von"].as_str(), Some("alpha@127.0.0.2"));
        assert_eq!(tabelle.len(), 2, "{text}");
    }

    #[test]
    fn gueltige_handgeschriebene_datei_wird_gelesen() {
        let text = "# von Hand\nangelegt_von = \"shoff@127.0.0.1\"\nangelegt = \"2026-10-28T10:15:00.123+01:00\"\n";

        assert_eq!(
            lesen(text.as_bytes()),
            Ok(Einstellungen {
                angelegt: Some(zeit("2026-10-28T10:15:00.123+01:00")),
                angelegt_von: Some(Kennung::bilden("shoff@127.0.0.1").expect("Kennung")),
            })
        );
    }

    #[test]
    fn unbekannte_schluessel_bleiben_beim_zurueckschreiben_erhalten() {
        let vorher = b"farbe = \"rot\"\nbeschreibung = \"Absprachen\"\nangelegt = \"2020-01-01T00:00:00.000+01:00\"\n";

        let text = schreiben(&alpha(), Some(vorher));
        let tabelle: toml::Table = text.parse().expect("TOML");

        assert_eq!(tabelle["farbe"].as_str(), Some("rot"));
        assert_eq!(tabelle["beschreibung"].as_str(), Some("Absprachen"));
        assert_eq!(lesen(text.as_bytes()), Ok(alpha()));
    }
}
