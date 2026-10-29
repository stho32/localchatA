//! Namensregel eines Anhangs (ADR-0046 Regel 4, B0424): der Anzeigename wird abgewiesen, nie umgeformt; der
//! gespeicherte Name ist sein ASCII-Ersatz. Rein.

/// Höchstens so viele Bytes UTF-8 hat ein Anzeigename.
pub const NAME_HOECHSTENS_BYTES: usize = 255;

/// Warum ein Anzeigename abgewiesen wird (422 `name` mit `grund`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Namensfehler {
    Leer,
    Laenge { bytes: usize },
    Zeichen { stelle: usize, zeichen: String },
    Punkte,
}

/// Höchstens so viele Zeichen hat der ASCII-Teil des gespeicherten Namens.
const ASCII_HOECHSTENS: usize = 64;
/// Eine Endung bis zu so vielen Zeichen (ohne Punkt) überlebt das Kürzen.
const ENDUNG_HOECHSTENS: usize = 16;
/// Ersatz für einen Stamm ohne Buchstaben und Ziffern.
const ERSATZSTAMM: &str = "datei";

/// Prüft den Anzeigenamen und liefert den ASCII-Teil des gespeicherten Namens (ohne `<hhmmssmmm>-<n>-`).
/// Reihenfolge: leer → zu lang (Bytes) → verbotenes Zeichen (das erste, Stelle in Zeichen ab 0) → `.`/`..`.
pub fn name_pruefen(anzeigename: &str) -> Result<String, Namensfehler> {
    if anzeigename.is_empty() {
        return Err(Namensfehler::Leer);
    }
    if anzeigename.len() > NAME_HOECHSTENS_BYTES {
        return Err(Namensfehler::Laenge {
            bytes: anzeigename.len(),
        });
    }
    if let Some((stelle, zeichen)) = anzeigename
        .chars()
        .enumerate()
        .find(|(_, zeichen)| verboten(*zeichen))
    {
        return Err(Namensfehler::Zeichen {
            stelle,
            zeichen: format!("U+{:04X}", u32::from(zeichen)),
        });
    }
    if anzeigename == "." || anzeigename == ".." {
        return Err(Namensfehler::Punkte);
    }
    Ok(ascii_ersatz(anzeigename))
}

/// Steuerzeichen jeder Art (auch Tab, CR, LF), Richtungszeichen, Pfadtrenner und Anführungszeichen.
fn verboten(zeichen: char) -> bool {
    zeichen.is_control()
        || matches!(zeichen, '/' | '\\' | '"')
        || matches!(zeichen, '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
}

/// Jedes Zeichen außerhalb `[A-Za-z0-9._-]` zu `_`, höchstens 64 Zeichen mit Endung, nie nichtssagend.
fn ascii_ersatz(anzeigename: &str) -> String {
    let ersetzt: String = anzeigename
        .chars()
        .map(|zeichen| {
            if zeichen.is_ascii_alphanumeric() || matches!(zeichen, '.' | '_' | '-') {
                zeichen
            } else {
                '_'
            }
        })
        .collect();
    let sagt_etwas = |teil: &str| teil.chars().any(|zeichen| zeichen.is_ascii_alphanumeric());
    let (stamm, endung) = teilen(&ersetzt);
    match (sagt_etwas(stamm), sagt_etwas(endung)) {
        (true, _) => kuerzen(stamm, endung),
        (false, true) => kuerzen(ERSATZSTAMM, endung),
        (false, false) => ERSATZSTAMM.to_string(),
    }
}

/// Stamm und Endung (mit Punkt) am letzten Punkt; ein Punkt am Anfang beginnt keine Endung.
fn teilen(name: &str) -> (&str, &str) {
    match name.rfind('.') {
        Some(stelle) if stelle > 0 => name.split_at(stelle),
        _ => (name, ""),
    }
}

/// Höchstens 64 Zeichen: eine Endung bis 16 Zeichen bleibt am Ende, eine längere entfällt mit dem Rest.
fn kuerzen(stamm: &str, endung: &str) -> String {
    let ganz = format!("{stamm}{endung}");
    if ganz.len() <= ASCII_HOECHSTENS {
        return ganz;
    }
    if !endung.is_empty() && endung.len() - 1 <= ENDUNG_HOECHSTENS {
        let platz = ASCII_HOECHSTENS - endung.len();
        return format!("{}{endung}", &stamm[..platz]);
    }
    ganz[..ASCII_HOECHSTENS].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zeichen(stelle: usize, zeichen: &str) -> Result<String, Namensfehler> {
        Err(Namensfehler::Zeichen {
            stelle,
            zeichen: zeichen.to_string(),
        })
    }

    /// R00029 „Namensregel“: `Bericht März.pdf` → `Bericht_M_rz.pdf`; jedes Zeichen außerhalb `[A-Za-z0-9._-]`
    /// wird zu `_` (je Zeichen, nicht je Byte).
    #[test]
    fn umlaute_und_leerzeichen_werden_zu_unterstrichen() {
        assert_eq!(
            name_pruefen("Bericht März.pdf"),
            Ok("Bericht_M_rz.pdf".into())
        );
        assert_eq!(name_pruefen("a-b_c.D9.txt"), Ok("a-b_c.D9.txt".into()));
        assert_eq!(name_pruefen("😀x"), Ok("_x".into()));
    }

    /// Pfad, Rückstrich, Anführungszeichen, Steuerzeichen, Richtungszeichen und die Namen `.`/`..` → abgewiesen.
    #[test]
    fn verbotene_zeichen_werden_mit_stelle_abgewiesen() {
        for (name, stelle, schreibweise) in [
            ("../x", 2, "U+002F"),
            ("a/b", 1, "U+002F"),
            ("a\\b", 1, "U+005C"),
            ("a\"b", 1, "U+0022"),
            ("rechnung\u{202e}fdp.exe", 8, "U+202E"),
            ("a\tb", 1, "U+0009"),
            ("a\nb", 1, "U+000A"),
            ("a\rb", 1, "U+000D"),
            ("\u{1b}[31m", 0, "U+001B"),
            ("x\u{7f}", 1, "U+007F"),
            ("x\u{85}", 1, "U+0085"),
            ("x\u{200e}", 1, "U+200E"),
            ("x\u{200f}", 1, "U+200F"),
            ("x\u{202a}", 1, "U+202A"),
            ("x\u{2066}", 1, "U+2066"),
            ("x\u{2069}", 1, "U+2069"),
        ] {
            assert_eq!(
                name_pruefen(name),
                zeichen(stelle, schreibweise),
                "{name:?}"
            );
        }
    }

    /// Kontrolle: Zeichen neben den Richtungszeichen bleiben erlaubt (Anzeigename bleibt, M29).
    #[test]
    fn nachbarn_der_richtungszeichen_und_homoglyphen_sind_erlaubt() {
        for name in [
            "x\u{2065}",
            "x\u{2029}",
            "x\u{202f}",
            "раураl.pdf",
            "a  .exe",
            "b.pdf.exe",
        ] {
            assert!(name_pruefen(name).is_ok(), "{name:?}");
        }
    }

    #[test]
    fn leer_punkt_und_punktpunkt_werden_abgewiesen() {
        assert_eq!(name_pruefen(""), Err(Namensfehler::Leer));
        assert_eq!(name_pruefen("."), Err(Namensfehler::Punkte));
        assert_eq!(name_pruefen(".."), Err(Namensfehler::Punkte));
        assert_eq!(name_pruefen("..."), Ok("datei".into()));
        assert_eq!(name_pruefen(".x"), Ok(".x".into()));
    }

    /// 255 Bytes ok, 256 zu lang — Bytes, nicht Zeichen.
    #[test]
    fn die_grenze_liegt_bei_255_bytes() {
        assert!(name_pruefen(&"a".repeat(255)).is_ok());
        assert_eq!(
            name_pruefen(&"a".repeat(256)),
            Err(Namensfehler::Laenge { bytes: 256 })
        );
        assert!(name_pruefen(&"ä".repeat(127)).is_ok());
        assert_eq!(
            name_pruefen(&"ä".repeat(128)),
            Err(Namensfehler::Laenge { bytes: 256 })
        );
    }

    /// Höchstens 64 Zeichen; eine Endung bis 16 Zeichen (ohne Punkt) bleibt, eine längere entfällt beim Kürzen.
    #[test]
    fn gekuerzt_auf_64_zeichen_mit_endung() {
        assert_eq!(name_pruefen("x.tar.gz"), Ok("x.tar.gz".into()));
        let lang = format!("{}.tar.gz", "a".repeat(70));
        assert_eq!(name_pruefen(&lang), Ok(format!("{}.gz", "a".repeat(61))));
        let sechzehn = format!("{}.{}", "a".repeat(70), "e".repeat(16));
        assert_eq!(
            name_pruefen(&sechzehn),
            Ok(format!("{}.{}", "a".repeat(47), "e".repeat(16)))
        );
        let siebzehn = format!("{}.{}", "a".repeat(70), "e".repeat(17));
        assert_eq!(name_pruefen(&siebzehn), Ok("a".repeat(64)));
        assert_eq!(name_pruefen(&"b".repeat(64)), Ok("b".repeat(64)));
        assert_eq!(name_pruefen(&"b".repeat(65)), Ok("b".repeat(64)));
    }

    /// Ein Name nur aus Zeichen außerhalb des ASCII-Satzes ergibt nie einen leeren oder nichtssagenden Namen.
    #[test]
    fn nur_umlaute_ergeben_datei() {
        assert_eq!(name_pruefen("äöü"), Ok("datei".into()));
        assert_eq!(name_pruefen("ä"), Ok("datei".into()));
        assert_eq!(name_pruefen("äöü.pdf"), Ok("datei.pdf".into()));
        assert_eq!(name_pruefen("Ä1"), Ok("_1".into()));
        assert_eq!(name_pruefen("ä.ö"), Ok("datei".into()));
    }
}
