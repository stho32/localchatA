//! Einstellungsformat (A00013, ADR-0022): `einstellungen.toml` eines Channels ↔ geprüfte Werte, rein.
//!
//! Kennt keine Datei, nur Bytes und Text; die Logablage schreibt und liest sie.

use super::Kennung;
use super::frist::frist_in_spanne;
use crate::uhr::Zeit;
use chrono::SecondsFormat;
use serde::Deserialize;
use toml::{Spanned, Table, Value};

/// Was die Datei über einen Channel sagt; fehlt ein Schlüssel, ist der Wert unbekannt.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Einstellungen {
    pub angelegt: Option<Zeit>,
    pub angelegt_von: Option<Kennung>,
    pub beschreibung: Option<Beschreibung>,
    /// Aufbewahrungsfrist in Tagen (1–3650); fehlt sie, gilt der Standard (ADR-0029 Regel 1).
    pub frist_tage: Option<u32>,
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
const BESCHREIBUNG: &str = "beschreibung";
const FRIST_TAGE: &str = "frist_tage";

/// Liest die Datei; jeder Defekt nennt seine Zeile: kein UTF-8, kein TOML (auch doppelter Schlüssel), `angelegt`
/// keine Zeit nach RFC 3339 als Text, `angelegt_von` keine Kennung, `beschreibung` gegen die Regel (`""` ist keine),
/// `frist_tage` keine ganze Zahl von 1 bis 3650 (ADR-0029 Regel 1). Unbekannte Schlüssel sind kein Defekt.
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
        angelegt: deuten(bytes, roh.angelegt, ANGELEGT, |wert| {
            wert.as_str()
                .and_then(|text| Zeit::parse_from_rfc3339(text).ok())
        })?,
        angelegt_von: deuten(bytes, roh.angelegt_von, ANGELEGT_VON, |wert| {
            wert.as_str().and_then(|text| Kennung::bilden(text).ok())
        })?,
        beschreibung: deuten(bytes, roh.beschreibung, BESCHREIBUNG, |wert| {
            wert.as_str()
                .and_then(|text| Beschreibung::bilden(text).ok())
        })?
        .flatten(),
        frist_tage: deuten(bytes, roh.frist_tage, FRIST_TAGE, |wert| {
            wert.as_integer().and_then(frist_in_spanne)
        })?,
    })
}

/// Die bekannten Schlüssel mit ihrer Stelle; alle anderen übergeht serde.
#[derive(Deserialize)]
struct Roh {
    angelegt: Option<Spanned<Value>>,
    angelegt_von: Option<Spanned<Value>>,
    beschreibung: Option<Spanned<Value>>,
    frist_tage: Option<Spanned<Value>>,
}

/// Ein Wert, den `wert_aus` versteht — sonst ein Defekt in der Zeile des Werts.
fn deuten<T>(
    bytes: &[u8],
    roh: Option<Spanned<Value>>,
    schluessel: &str,
    wert_aus: impl Fn(&Value) -> Option<T>,
) -> Result<Option<T>, Defekt> {
    let Some(roh) = roh else {
        return Ok(None);
    };
    match wert_aus(roh.get_ref()) {
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
        einstellungen
            .angelegt
            .as_ref()
            .map(|zeit| Value::String(zeit_als_text(zeit))),
    );
    setzen(
        &mut tabelle,
        ANGELEGT_VON,
        einstellungen
            .angelegt_von
            .as_ref()
            .map(|kennung| Value::String(kennung.als_text().to_string())),
    );
    setzen(
        &mut tabelle,
        BESCHREIBUNG,
        einstellungen
            .beschreibung
            .as_ref()
            .map(|beschreibung| Value::String(beschreibung.als_text().to_string())),
    );
    setzen(
        &mut tabelle,
        FRIST_TAGE,
        einstellungen
            .frist_tage
            .map(|tage| Value::Integer(i64::from(tage))),
    );
    tabelle.to_string()
}

fn setzen(tabelle: &mut Table, schluessel: &str, wert: Option<Value>) {
    match wert {
        Some(wert) => {
            tabelle.insert(schluessel.to_string(), wert);
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
            beschreibung: None,
            frist_tage: None,
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
                beschreibung: None,
                frist_tage: None,
            })
        );
    }

    #[test]
    fn unbekannte_schluessel_bleiben_beim_zurueckschreiben_erhalten() {
        // Seit R00007 ist beschreibung ein bekannter Schlüssel (eigener Test oben); notiz steht für die unbekannten.
        let vorher = b"farbe = \"rot\"\nnotiz = \"Absprachen\"\nangelegt = \"2020-01-01T00:00:00.000+01:00\"\n";

        let text = schreiben(&alpha(), Some(vorher));
        let tabelle: toml::Table = text.parse().expect("TOML");

        assert_eq!(tabelle["farbe"].as_str(), Some("rot"));
        assert_eq!(tabelle["notiz"].as_str(), Some("Absprachen"));
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
    fn mit_beschreibung(text: &str) -> Einstellungen {
        Einstellungen {
            beschreibung: Beschreibung::bilden(text).expect("Regel"),
            ..alpha()
        }
    }

    #[test]
    fn rundweg_mit_beschreibung_samt_formatzeichen() {
        for text in [
            "Absprachen zwischen Security- und Entwicklungs-Factory",
            "\u{202e}abc \"zitiert\" \\",
        ] {
            let einstellungen = mit_beschreibung(text);

            let geschrieben = schreiben(&einstellungen, None);

            assert_eq!(
                lesen(geschrieben.as_bytes()),
                Ok(einstellungen),
                "{geschrieben}"
            );
            let tabelle: toml::Table = geschrieben.parse().expect("TOML");
            assert_eq!(tabelle["beschreibung"].as_str(), Some(text));
        }
    }

    #[test]
    fn ohne_beschreibung_steht_kein_schluessel_und_leer_ist_keine() {
        let geschrieben = schreiben(&alpha(), Some(b"beschreibung = \"alt\"\n"));

        assert!(!geschrieben.contains("beschreibung"), "{geschrieben}");
        assert_eq!(
            lesen(b"beschreibung = \"\"\n").map(|gelesen| gelesen.beschreibung),
            Ok(None)
        );
    }

    #[test]
    fn eine_beschreibung_gegen_die_regel_ist_ein_defekt_ihrer_zeile() {
        let zu_lang = format!(
            "angelegt_von = \"a@1\"\nbeschreibung = \"{}\"\n",
            "ä".repeat(201)
        );
        assert_eq!(
            defekt_in_zeile(b"angelegt_von = \"alpha@127.0.0.2\"\nbeschreibung = \"a\\nb\"\n"),
            2
        );
        assert_eq!(defekt_in_zeile(zu_lang.as_bytes()), 2);
        assert_eq!(defekt_in_zeile(b"beschreibung = 5\n"), 1);
        assert_eq!(
            lesen(format!("beschreibung = \"{}\"\n", "ä".repeat(200)).as_bytes())
                .map(|gelesen| gelesen.beschreibung.map(|b| b.als_text().chars().count())),
            Ok(Some(200))
        );
    }

    fn frist_gelesen(bytes: &[u8]) -> Option<u32> {
        lesen(bytes).expect("lesbar").frist_tage
    }

    /// R00008 F0033 „Einstellungsformat“: die Spanne 1 bis 3650 wie an der API (dieselbe Prüfung).
    #[test]
    fn frist_tage_eins_und_3650_werden_gelesen_fehlen_heisst_unbekannt() {
        assert_eq!(frist_gelesen(b"frist_tage = 1\n"), Some(1));
        assert_eq!(frist_gelesen(b"frist_tage = 3650\n"), Some(3650));
        assert_eq!(frist_gelesen(b"farbe = \"blau\"\n"), None);
    }

    #[test]
    fn frist_tage_ausserhalb_der_spanne_oder_kein_ganzzahlwert_ist_ein_defekt_seiner_zeile() {
        for wert in ["0", "3651", "-3", "\"7\"", "7.5", "7.0", "true"] {
            let text = format!("farbe = \"blau\"\nfrist_tage = {wert}\n");
            assert_eq!(defekt_in_zeile(text.as_bytes()), 2, "{wert}");
        }
    }

    #[test]
    fn rundweg_mit_frist_erhaelt_unbekannte_schluessel_und_derselbe_wert_ergibt_dieselben_bytes() {
        let vorher = b"angelegt = \"2026-10-28T10:15:00.123+01:00\"\nangelegt_von = \"alpha@127.0.0.2\"\nfarbe = \"blau\"\n";
        let mut einstellungen = lesen(vorher).expect("lesbar");
        einstellungen.frist_tage = Some(30);

        let geschrieben = schreiben(&einstellungen, Some(vorher));

        let tabelle: toml::Table = geschrieben.parse().expect("TOML");
        assert_eq!(tabelle["frist_tage"].as_integer(), Some(30), "{geschrieben}");
        assert_eq!(tabelle["farbe"].as_str(), Some("blau"));
        assert_eq!(
            tabelle["angelegt"].as_str(),
            Some("2026-10-28T10:15:00.123+01:00")
        );
        assert_eq!(lesen(geschrieben.as_bytes()), Ok(einstellungen.clone()));
        assert_eq!(
            schreiben(&einstellungen, Some(geschrieben.as_bytes())),
            geschrieben
        );
    }
}
