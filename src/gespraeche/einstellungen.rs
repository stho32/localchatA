//! Einstellungsformat (A00013, ADR-0022): `einstellungen.toml` eines Channels ↔ geprüfte Werte, rein.
//!
//! Kennt keine Datei, nur Bytes und Text; die Logablage schreibt und liest sie.

use super::Kennung;
use crate::uhr::Zeit;
use chrono::SecondsFormat;
use serde::Deserialize;
use toml::{Spanned, Table, Value};

/// Was die Datei über einen Channel sagt; fehlt ein Schlüssel, ist der Wert unbekannt.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Einstellungen {
    pub angelegt: Option<Zeit>,
    pub angelegt_von: Option<Kennung>,
}

/// Höchstens so viele Unicode-Zeichen hat eine Beschreibung (ADR-0022 Regel 5).
pub const BESCHREIBUNG_HOECHSTENS: usize = 200;

/// Ein Satz Beschreibung: 1 bis 200 Unicode-Zeichen ohne Steuerzeichen (Kategorie Cc).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Beschreibung(String);

/// Die Beschreibung verletzt die Regel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Beschreibungsfehler;

impl Beschreibung {
    /// `""` ist keine Beschreibung; sonst geprüft, nie umgeformt.
    /// Gelesen werden höchstens 201 Zeichen — mehr entscheidet nichts.
    pub fn bilden(text: &str) -> Result<Option<Beschreibung>, Beschreibungsfehler> {
        if text.is_empty() {
            return Ok(None);
        }
        let zeichen: Vec<char> = text.chars().take(BESCHREIBUNG_HOECHSTENS + 1).collect();
        if zeichen.len() > BESCHREIBUNG_HOECHSTENS
            || zeichen.iter().any(|zeichen| zeichen.is_control())
        {
            return Err(Beschreibungsfehler);
        }
        Ok(Some(Beschreibung(text.to_string())))
    }

    pub fn als_text(&self) -> &str {
        &self.0
    }
}

/// Die Datei ist nicht lesbar: Zeile (ab 1) und Grund.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Defekt {
    pub zeile: usize,
    pub grund: String,
}

const ANGELEGT: &str = "angelegt";
const ANGELEGT_VON: &str = "angelegt_von";

/// Liest die Datei; jeder Defekt nennt seine Zeile: kein UTF-8, kein TOML (auch doppelter Schlüssel), `angelegt`
/// keine Zeit nach RFC 3339 als Text, `angelegt_von` keine Kennung. Unbekannte Schlüssel sind kein Defekt.
pub fn lesen(bytes: &[u8]) -> Result<Einstellungen, Defekt> {
    let text = std::str::from_utf8(bytes).map_err(|fehler| Defekt {
        zeile: zeile_bei(bytes, fehler.valid_up_to()),
        grund: "kein UTF-8".to_string(),
    })?;
    let roh: Roh = toml::from_str(text).map_err(|fehler| Defekt {
        zeile: zeile_bei(bytes, fehler.span().map_or(0, |spanne| spanne.start)),
        grund: fehler.message().trim().to_string(),
    })?;
    Ok(Einstellungen {
        angelegt: deuten(bytes, roh.angelegt, ANGELEGT, |text| {
            Zeit::parse_from_rfc3339(text).ok()
        })?,
        angelegt_von: deuten(bytes, roh.angelegt_von, ANGELEGT_VON, |text| {
            Kennung::bilden(text).ok()
        })?,
    })
}

/// Die bekannten Schlüssel mit ihrer Stelle; alle anderen übergeht serde.
#[derive(Deserialize)]
struct Roh {
    angelegt: Option<Spanned<Value>>,
    angelegt_von: Option<Spanned<Value>>,
}

/// Ein Text, den `wert_aus` versteht — sonst ein Defekt in der Zeile des Werts.
fn deuten<T>(
    bytes: &[u8],
    roh: Option<Spanned<Value>>,
    schluessel: &str,
    wert_aus: impl Fn(&str) -> Option<T>,
) -> Result<Option<T>, Defekt> {
    let Some(roh) = roh else {
        return Ok(None);
    };
    match roh.get_ref().as_str().and_then(wert_aus) {
        Some(wert) => Ok(Some(wert)),
        None => Err(Defekt {
            zeile: zeile_bei(bytes, roh.span().start),
            grund: format!("{schluessel} = {} ist ungültig", roh.get_ref()),
        }),
    }
}

/// Zeile (ab 1) des Bytes an `stelle`.
fn zeile_bei(bytes: &[u8], stelle: usize) -> usize {
    let davor = bytes.get(..stelle).unwrap_or(bytes);
    1 + davor.iter().filter(|&&byte| byte == b'\n').count()
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

    fn defekt_in_zeile(bytes: &[u8]) -> usize {
        match lesen(bytes) {
            Ok(gelesen) => panic!(
                "{:?} gilt als lesbar: {gelesen:?}",
                String::from_utf8_lossy(bytes)
            ),
            Err(defekt) => {
                assert!(!defekt.grund.is_empty(), "{defekt:?}");
                defekt.zeile
            }
        }
    }

    #[test]
    fn defekte_dateien_nennen_ihre_zeile() {
        assert_eq!(defekt_in_zeile(b"angelegt = \""), 1);
        assert_eq!(defekt_in_zeile(b"# Kopf\nangelegt_von = \"Claude@x\"\n"), 2);
        assert_eq!(defekt_in_zeile(b"angelegt = 5\n"), 1);
        assert_eq!(
            defekt_in_zeile(b"farbe = \"rot\"\n\nangelegt = \"gestern\"\n"),
            3
        );
        assert_eq!(
            defekt_in_zeile(b"angelegt_von = \"a@1\"\nnotiz = \"\xff\"\n"),
            2
        );
        assert_eq!(defekt_in_zeile(b"a = 1\na = 2\n"), 2);
    }

    #[test]
    fn ein_unbekannter_schluessel_ist_kein_defekt() {
        assert_eq!(lesen(b"farbe = \"rot\"\n"), Ok(Einstellungen::default()));
        assert_eq!(lesen(b""), Ok(Einstellungen::default()));
    }
    fn beschreibung(text: &str) -> Result<Option<String>, Beschreibungsfehler> {
        Beschreibung::bilden(text).map(|gebildet| gebildet.map(|b| b.als_text().to_string()))
    }

    #[test]
    fn beschreibung_als_spanne_zweihundert_zeichen_gehen_zweihunderteins_nicht() {
        assert_eq!(beschreibung(""), Ok(None));
        assert_eq!(beschreibung(&"ä".repeat(200)), Ok(Some("ä".repeat(200))));
        assert_eq!(beschreibung(&"😀".repeat(200)), Ok(Some("😀".repeat(200))));
        assert_eq!(beschreibung("a"), Ok(Some("a".to_string())));
        assert_eq!(beschreibung(&"ä".repeat(201)), Err(Beschreibungsfehler));
        assert_eq!(beschreibung(&"😀".repeat(201)), Err(Beschreibungsfehler));
    }

    #[test]
    fn beschreibung_ohne_steuerzeichen_formatzeichen_sind_erlaubt() {
        for verstoss in ["a\nb", "a\tb", "a\u{7}", "a\rb", "\u{85}", "a\u{7f}", "\0"] {
            assert_eq!(
                beschreibung(verstoss),
                Err(Beschreibungsfehler),
                "{verstoss:?}"
            );
        }
        assert_eq!(
            beschreibung("\u{202e}abc"),
            Ok(Some("\u{202e}abc".to_string()))
        );
        assert_eq!(
            beschreibung("  Absprachen  "),
            Ok(Some("  Absprachen  ".to_string()))
        );
    }
}
