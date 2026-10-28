//! Name vor dem `@` (ADR-0013): Erlaubnisliste `[a-z0-9-]`, 1 bis 32 Zeichen, `-` nicht am Rand.
//! Abweisen statt umformen — die Umformung gibt es nur als Vorschlag.

use std::fmt;
use std::sync::LazyLock;

/// Die Regel in Worten, für Abweisungen und Hinweise.
pub const NAMENSREGEL: &str =
    "1 bis 32 Zeichen aus a–z, 0–9 und -, nicht mit - am Anfang oder Ende";

/// Ein geprüfter Name; zwei Namen sind gleich, wenn sie Zeichen für Zeichen gleich sind.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Name(String);

/// Der Text verletzt die Namensregel; `vorschlag` ist seine Umformung, wenn sie nicht leer ist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Namensfehler {
    pub regel: &'static str,
    pub vorschlag: Option<Name>,
}

impl Name {
    pub fn bilden(text: &str) -> Result<Name, Namensfehler> {
        if regel_erfuellt(text) {
            return Ok(Name(text.to_string()));
        }
        Err(Namensfehler {
            regel: NAMENSREGEL,
            vorschlag: umformen(text),
        })
    }

    pub fn als_text(&self) -> &str {
        &self.0
    }
}

const LAENGE: usize = 32;

fn regel_erfuellt(text: &str) -> bool {
    let erlaubt = |zeichen: char| zeichen.is_ascii_lowercase() || zeichen.is_ascii_digit();
    let laenge_passt = (1..=LAENGE).contains(&text.len());
    laenge_passt
        && text
            .chars()
            .all(|zeichen| erlaubt(zeichen) || zeichen == '-')
        && !text.starts_with('-')
        && !text.ends_with('-')
}

/// Klein, Umlaute zu `ae/oe/ue/ss`, Akzente zum Grundbuchstaben (`AKZENTE`), sonst nicht Erlaubtes zu `-`,
/// Mehrfach-`-` zusammengefasst, Ränder entfernt, auf 32 gekürzt; bleibt nichts übrig, gibt es keinen Vorschlag.
/// Gelesen werden höchstens `GELESEN_HOECHSTENS` Zeichen: der Text kommt ungeprüft (bis 2 MB) aus dem Netz.
fn umformen(text: &str) -> Option<Name> {
    umformen_aus(text.chars())
}

/// Viermal die Länge eines Namens — genug für Ränder, Akzente und Trennzeichen vor 32 verwertbaren Zeichen.
const GELESEN_HOECHSTENS: usize = 4 * LAENGE;

fn umformen_aus(zeichen: impl Iterator<Item = char>) -> Option<Name> {
    let mut umgeformt = String::new();
    for zeichen in zeichen.take(GELESEN_HOECHSTENS).flat_map(klein) {
        match zeichen {
            'ä' => umgeformt.push_str("ae"),
            'ö' => umgeformt.push_str("oe"),
            'ü' => umgeformt.push_str("ue"),
            'ß' => umgeformt.push_str("ss"),
            'a'..='z' | '0'..='9' => umgeformt.push(zeichen),
            _ => match grundbuchstabe(zeichen) {
                Some(grund) => umgeformt.push_str(grund),
                None if umgeformt.ends_with('-') => {}
                None => umgeformt.push('-'),
            },
        }
    }
    let gekuerzt: String = umgeformt.trim_matches('-').chars().take(LAENGE).collect();
    let vorschlag = gekuerzt.trim_end_matches('-');
    if vorschlag.is_empty() {
        return None;
    }
    Some(Name(vorschlag.to_string()))
}

/// Kleinschreibung je Zeichen; ein Zeichen der Akzenttabelle bleibt, wie es ist (`İ` würde sonst zu `i` mit
/// kombinierendem Punkt).
fn klein(zeichen: char) -> Vec<char> {
    if grundbuchstabe(zeichen).is_some() {
        return vec![zeichen];
    }
    zeichen.to_lowercase().collect()
}

/// Nachschlagen in O(1): ein Feld über U+00C0–U+017F, einmal aus `AKZENTE` gebaut.
fn grundbuchstabe(zeichen: char) -> Option<&'static str> {
    let stelle = (zeichen as usize).checked_sub(AKZENT_ANFANG as usize)?;
    GRUNDBUCHSTABEN.get(stelle).copied().flatten()
}

const AKZENT_ANFANG: char = '\u{C0}';
const AKZENT_ENDE: char = '\u{17F}';

static GRUNDBUCHSTABEN: LazyLock<Vec<Option<&'static str>>> = LazyLock::new(|| {
    let mut feld = vec![None; AKZENT_ENDE as usize - AKZENT_ANFANG as usize + 1];
    for (buchstaben, grund) in AKZENTE {
        for zeichen in buchstaben.chars() {
            feld[zeichen as usize - AKZENT_ANFANG as usize] = Some(*grund);
        }
    }
    feld
});

/// Regel eines Channelnamens, der unter Windows ein Gerätename ist (ADR-0020 Regel 2).
pub const RESERVIERT: &str = "reserviert";

/// Buchstaben mit Akzent aus U+00C0–U+017F und ihr Grundbuchstabe (R00006); dieselbe Tabelle steht in
/// `web/namensregel.js`. `ä`, `ö`, `ü`, `ß` gehen eigene Wege (`ae`, `oe`, `ue`, `ss`).
pub const AKZENTE: &[(&str, &str)] = &[
    ("àáâãåāăą", "a"),
    ("æ", "ae"),
    ("çćĉċč", "c"),
    ("ðďđ", "d"),
    ("èéêëēĕėęě", "e"),
    ("ĝğġģ", "g"),
    ("ĥħ", "h"),
    ("ìíîïĩīĭįıİ", "i"),
    ("ĳ", "ij"),
    ("ĵ", "j"),
    ("ķĸ", "k"),
    ("ĺļľŀł", "l"),
    ("ñńņňŉŋ", "n"),
    ("òóôõøōŏő", "o"),
    ("œ", "oe"),
    ("ŕŗř", "r"),
    ("śŝşšſ", "s"),
    ("ţťŧ", "t"),
    ("þ", "th"),
    ("ùúûũūŭůűų", "u"),
    ("ŵ", "w"),
    ("ýÿŷ", "y"),
    ("źżž", "z"),
];

/// Ein geprüfter Channelname = Ordnername (ADR-0020): Namensregel plus reservierte Gerätenamen.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Channelname(String);

impl Channelname {
    pub fn bilden(text: &str) -> Result<Channelname, Namensfehler> {
        match Name::bilden(text) {
            Ok(name) if reserviert(name.als_text()) => Err(Namensfehler {
                regel: RESERVIERT,
                vorschlag: None,
            }),
            Ok(name) => Ok(Channelname(name.0)),
            Err(fehler) => Err(Namensfehler {
                regel: fehler.regel,
                vorschlag: fehler
                    .vorschlag
                    .filter(|vorschlag| !reserviert(vorschlag.als_text())),
            }),
        }
    }

    pub fn als_text(&self) -> &str {
        &self.0
    }
}

/// `con`, `prn`, `aux`, `nul`, `com0`–`com9`, `lpt0`–`lpt9` — Gerätenamen, die Windows nicht als Ordner zulässt.
fn reserviert(name: &str) -> bool {
    if matches!(name, "con" | "prn" | "aux" | "nul") {
        return true;
    }
    let geraet_mit_ziffer = |vorsilbe: &str| {
        name.strip_prefix(vorsilbe).is_some_and(|rest| {
            rest.len() == 1 && rest.chars().all(|zeichen| zeichen.is_ascii_digit())
        })
    };
    geraet_mit_ziffer("com") || geraet_mit_ziffer("lpt")
}

impl fmt::Display for Channelname {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vorschlag(text: &str) -> Option<String> {
        match Name::bilden(text) {
            Ok(name) => panic!("{text:?} ist gültig: {name}"),
            Err(fehler) => {
                assert_eq!(fehler.regel, NAMENSREGEL);
                fehler.vorschlag.map(|name| name.als_text().to_string())
            }
        }
    }

    #[test]
    fn gueltige_namen_bleiben_wie_sie_sind() {
        let zweiunddreissig = "a".repeat(32);
        for text in [
            "a",
            "7",
            "a--b",
            "claude-2",
            "claude-x",
            "a-b-c",
            "x1",
            &zweiunddreissig,
        ] {
            assert_eq!(
                Name::bilden(text).map(|name| name.to_string()),
                Ok(text.to_string()),
                "{text}"
            );
        }
    }

    #[test]
    fn ungueltige_namen_werden_mit_der_umformung_als_vorschlag_abgewiesen() {
        let fall = |text: &str, erwartet: &str| {
            assert_eq!(vorschlag(text).as_deref(), Some(erwartet), "{text:?}");
        };
        fall("Jörg", "joerg");
        fall("Claude", "claude");
        fall("ÄÖÜß", "aeoeuess");
        fall("a_b", "a-b");
        fall("a.b", "a-b");
        fall("a b", "a-b");
        fall("a@b", "a-b");
        fall("a/b", "a-b");
        fall("a\\b", "a-b");
        fall("a+b", "a-b");
        fall("-a", "a");
        fall("a-", "a");
        fall("a__..b", "a-b");
        fall("chef\u{200B}", "chef");
        fall("\u{202E}abc", "abc");
        fall("\u{FEFF}abc", "abc");
        fall(&"c".repeat(33), &"c".repeat(32));
        fall(&format!("{}-b", "a".repeat(31)), &"a".repeat(31));
    }

    #[test]
    fn ohne_verwertbares_zeichen_gibt_es_keinen_vorschlag() {
        for text in ["", "___", "-", "  ", "\u{200B}", "…"] {
            assert_eq!(vorschlag(text), None, "{text:?}");
        }
    }

    #[test]
    fn jeder_vorschlag_erfuellt_die_regel() {
        for text in [
            "Jörg",
            "a_b",
            &"Ä".repeat(40),
            &format!("{}_x", "b".repeat(31)),
        ] {
            let vorgeschlagen = vorschlag(text).expect("Vorschlag");
            assert!(
                Name::bilden(&vorgeschlagen).is_ok(),
                "{text:?} → {vorgeschlagen:?}"
            );
        }
    }

    #[test]
    fn akzente_werden_im_vorschlag_zum_grundbuchstaben() {
        let fall = |text: &str, erwartet: &str| {
            assert_eq!(vorschlag(text).as_deref(), Some(erwartet), "{text:?}");
        };
        fall("Allgemein", "allgemein");
        fall("Prüfung", "pruefung");
        fall("José", "jose");
        fall("Café Crème", "cafe-creme");
        fall("Ñandú", "nandu");
        fall("Łódź", "lodz");
        fall("Ærø", "aero");
        fall("Straße", "strasse");
        fall("Œuvre", "oeuvre");
        fall("İstanbul", "istanbul");
        assert_eq!(vorschlag("日本"), None);
    }

    #[test]
    fn jeder_buchstabe_der_akzenttabelle_liegt_in_u00c0_bis_u017f_und_wird_zu_a_bis_z() {
        for (buchstaben, grund) in AKZENTE {
            assert!(
                grund.chars().all(|zeichen| zeichen.is_ascii_lowercase()),
                "{grund}"
            );
            for zeichen in buchstaben.chars() {
                assert!(('\u{C0}'..='\u{17F}').contains(&zeichen), "{zeichen:?}");
                assert_eq!(
                    vorschlag(&zeichen.to_string()).as_deref(),
                    Some(*grund),
                    "{zeichen:?}"
                );
            }
        }
    }

    #[test]
    fn langer_text_vorschlag_aus_den_ersten_zeichen() {
        let text = format!("{}{}", "a".repeat(40), "ÿ".repeat(1_000_000));
        assert_eq!(vorschlag(&text), Some("a".repeat(32)));
        assert_eq!(vorschlag(&"ÿ".repeat(995_000)), Some("y".repeat(32)));
        assert_eq!(vorschlag(&"日".repeat(663_000)), None);
        assert_eq!(vorschlag(&"a".repeat(1_990_000)), Some("a".repeat(32)));
    }

    #[test]
    fn umformen_liest_hoechstens_die_feste_zahl_zeichen() {
        let text = format!("{}{}", "a".repeat(40), "ÿ".repeat(1_000_000));
        let gelesen = std::cell::Cell::new(0_usize);
        let umgeformt = umformen_aus(text.chars().inspect(|_| gelesen.set(gelesen.get() + 1)));
        assert_eq!(umgeformt.map(|name| name.0), Some("a".repeat(32)));
        assert_eq!(GELESEN_HOECHSTENS, 4 * LAENGE);
        assert!(
            gelesen.get() <= GELESEN_HOECHSTENS,
            "gelesen: {} Zeichen",
            gelesen.get()
        );
    }

    #[test]
    fn hinter_der_festen_zahl_zeichen_zaehlt_nichts_mehr() {
        let vor_der_grenze = format!("{}abc", "_".repeat(GELESEN_HOECHSTENS - 3));
        assert_eq!(vorschlag(&vor_der_grenze).as_deref(), Some("abc"));
        let hinter_der_grenze = format!("{}abc", "_".repeat(GELESEN_HOECHSTENS));
        assert_eq!(vorschlag(&hinter_der_grenze), None);
    }

    #[test]
    fn grundbuchstabe_folgt_der_akzenttabelle_fuer_jedes_zeichen_bis_u017f() {
        for zeichen in '\u{0}'..='\u{17F}' {
            let aus_der_tabelle = AKZENTE
                .iter()
                .find(|(buchstaben, _)| buchstaben.contains(zeichen))
                .map(|(_, grund)| *grund);
            assert_eq!(grundbuchstabe(zeichen), aus_der_tabelle, "{zeichen:?}");
        }
        assert_eq!(grundbuchstabe('日'), None);
    }

    fn channelfehler(text: &str) -> Namensfehler {
        match Channelname::bilden(text) {
            Ok(name) => panic!("{text:?} ist als Channelname gültig: {name}"),
            Err(fehler) => fehler,
        }
    }

    #[test]
    fn channelname_gilt_als_spanne_bis_32_zeichen() {
        assert_eq!(
            Channelname::bilden(&"a".repeat(32)).map(|name| name.als_text().to_string()),
            Ok("a".repeat(32))
        );
        let zu_lang = channelfehler(&"a".repeat(33));
        assert_eq!(zu_lang.regel, NAMENSREGEL);
    }

    #[test]
    fn channelname_folgt_der_namensregel() {
        for text in ["a", "a-1", "com10", "console", "lpt", "com", "allgemein"] {
            assert_eq!(
                Channelname::bilden(text).map(|name| name.to_string()),
                Ok(text.to_string()),
                "{text:?}"
            );
        }
        for text in ["-a", "a-", "a_b", "a b", "..", "a/b", "a\\b", ""] {
            assert_eq!(channelfehler(text).regel, NAMENSREGEL, "{text:?}");
        }
        assert_eq!(
            channelfehler("Allgemein")
                .vorschlag
                .map(|name| name.to_string()),
            Some("allgemein".to_string())
        );
    }

    #[test]
    fn reservierte_geraetenamen_sind_ohne_vorschlag_abgewiesen() {
        for text in [
            "con", "prn", "aux", "nul", "com0", "com1", "com9", "lpt0", "lpt1", "lpt9",
        ] {
            let fehler = channelfehler(text);
            assert_eq!(fehler.regel, RESERVIERT, "{text:?}");
            assert_eq!(fehler.vorschlag, None, "{text:?}");
        }
    }

    #[test]
    fn ist_die_umformung_reserviert_gibt_es_keinen_vorschlag() {
        let fehler = channelfehler("CON");
        assert_eq!(fehler.regel, NAMENSREGEL);
        assert_eq!(fehler.vorschlag, None);
        assert_eq!(channelfehler("Com1").vorschlag, None);
        assert_eq!(
            channelfehler("Com10")
                .vorschlag
                .map(|name| name.to_string()),
            Some("com10".to_string())
        );
    }
}
