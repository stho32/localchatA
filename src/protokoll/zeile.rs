//! Formatierer des Betriebsprotokolls (ADR-0040 Regeln 3–5): die einzige Stelle, an der Text und Werte
//! neutralisiert werden und eine Zeile entsteht. Rein, ohne Uhr und Datei.

/// Höchstens so viele Zeichen hat ein Wert im Kontext (ADR-0040 Regel 4).
use crate::uhr::Zeit;

/// Stufe eines Eintrags, aufsteigend (ADR-0042 Regel 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Stufe {
    Debug,
    Information,
    Warning,
    Error,
    Exception,
}

impl Stufe {
    pub fn als_text(self) -> &'static str {
        match self {
            Stufe::Debug => "DEBUG",
            Stufe::Information => "INFORMATION",
            Stufe::Warning => "WARNING",
            Stufe::Error => "ERROR",
            Stufe::Exception => "EXCEPTION",
        }
    }
}

/// Bereich eines Eintrags aus der festen Liste (ADR-0040 Regel 3 mit Ergänzung `körper`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Bereich {
    Start,
    Ende,
    Protokoll,
    Laden,
    Verbindung,
    Grenzschutz,
    Sitzung,
    Channels,
    Verlauf,
    Senden,
    Schreibkonto,
    Frist,
    Zeit,
    Programmfehler,
    Koerper,
}

impl Bereich {
    pub fn als_text(self) -> &'static str {
        match self {
            Bereich::Start => "start",
            Bereich::Ende => "ende",
            Bereich::Protokoll => "protokoll",
            Bereich::Laden => "laden",
            Bereich::Verbindung => "verbindung",
            Bereich::Grenzschutz => "grenzschutz",
            Bereich::Sitzung => "sitzung",
            Bereich::Channels => "channels",
            Bereich::Verlauf => "verlauf",
            Bereich::Senden => "senden",
            Bereich::Schreibkonto => "schreibkonto",
            Bereich::Frist => "frist",
            Bereich::Zeit => "zeit",
            Bereich::Programmfehler => "programmfehler",
            Bereich::Koerper => "körper",
        }
    }
}

/// Schlüssel des Kontexts aus der Erlaubnisliste (ADR-0040 Regel 3 mit Ergänzung) — es gibt keinen anderen Weg
/// in den Kontext, also auch keinen für Kopfzeilen, Token oder Körper.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Schluessel {
    Kennung,
    Herkunft,
    Channel,
    Route,
    Status,
    Datei,
    Zeile,
    Grund,
    Anzahl,
    DauerMs,
    Grenze,
    Ort,
    Thread,
    Version,
    Adresse,
    Ordner,
    Stufe,
    Option,
    Seit,
    Typ,
    Name,
    Bytes,
    Art,
    Verworfen,
    Operation,
}

impl Schluessel {
    pub fn als_text(self) -> &'static str {
        match self {
            Schluessel::Kennung => "kennung",
            Schluessel::Herkunft => "herkunft",
            Schluessel::Channel => "channel",
            Schluessel::Route => "route",
            Schluessel::Status => "status",
            Schluessel::Datei => "datei",
            Schluessel::Zeile => "zeile",
            Schluessel::Grund => "grund",
            Schluessel::Anzahl => "anzahl",
            Schluessel::DauerMs => "dauer_ms",
            Schluessel::Grenze => "grenze",
            Schluessel::Ort => "ort",
            Schluessel::Thread => "thread",
            Schluessel::Version => "version",
            Schluessel::Adresse => "adresse",
            Schluessel::Ordner => "ordner",
            Schluessel::Stufe => "stufe",
            Schluessel::Option => "option",
            Schluessel::Seit => "seit",
            Schluessel::Typ => "typ",
            Schluessel::Name => "name",
            Schluessel::Bytes => "bytes",
            Schluessel::Art => "art",
            Schluessel::Verworfen => "verworfen",
            Schluessel::Operation => "operation",
        }
    }
}

/// Benannte Werte eines Eintrags in der Reihenfolge, in der sie gesetzt werden.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Kontext(Vec<(Schluessel, String)>);

impl Kontext {
    pub fn neu() -> Kontext {
        Kontext::default()
    }

    /// Setzt einen Wert; der Wert wird erst in der Zeile neutralisiert.
    pub fn mit(mut self, schluessel: Schluessel, wert: impl ToString) -> Kontext {
        self.0.push((schluessel, wert.to_string()));
        self
    }

    /// Der Wert zu einem Schlüssel, falls gesetzt (roh).
    pub fn wert(&self, schluessel: Schluessel) -> Option<&str> {
        self.0
            .iter()
            .find(|(gesetzt, _)| *gesetzt == schluessel)
            .map(|(_, wert)| wert.as_str())
    }
}

/// Ein Eintrag: Stufe, Bereich, deutscher Text, Kontext.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Eintrag {
    pub stufe: Stufe,
    pub bereich: Bereich,
    pub text: String,
    pub kontext: Kontext,
}

impl Eintrag {
    pub fn neu(
        stufe: Stufe,
        bereich: Bereich,
        text: impl Into<String>,
        kontext: Kontext,
    ) -> Eintrag {
        Eintrag {
            stufe,
            bereich,
            text: text.into(),
            kontext,
        }
    }
}

/// `<zeit> <STUFE> <bereich> <text>[ | <schlüssel>=<wert> …]` und `\n` — genau eine Zeile, was immer im Text oder
/// in den Werten steht.
pub fn zeile(eintrag: &Eintrag, zeit: Zeit) -> String {
    let mut zeile = format!(
        "{} {} {} {}",
        zeit.format("%Y-%m-%dT%H:%M:%S%.3f%:z"),
        eintrag.stufe.als_text(),
        eintrag.bereich.als_text(),
        neutralisieren(&eintrag.text, TEXT_HOECHSTENS)
    );
    for (nummer, (schluessel, wert)) in eintrag.kontext.0.iter().enumerate() {
        zeile.push_str(if nummer == 0 { " | " } else { " " });
        zeile.push_str(schluessel.als_text());
        zeile.push('=');
        zeile.push_str(&wert_in_zeile(wert));
    }
    zeile.push('\n');
    zeile
}

/// Neutralisiert und gekürzt; mit Leerzeichen, `=`, `|` oder `"` (oder leer) in `"…"` mit `\"`.
fn wert_in_zeile(roh: &str) -> String {
    let wert = neutralisieren(roh, WERT_HOECHSTENS);
    if wert.is_empty() || wert.contains([' ', '=', '|', '"']) {
        format!("\"{}\"", wert.replace('"', "\\\""))
    } else {
        wert
    }
}

/// Höchstens so viele Zeichen hat ein Wert im Kontext (ADR-0040 Regel 4).
pub const WERT_HOECHSTENS: usize = 256;
/// Höchstens so viele Zeichen hat der Text eines Eintrags.
pub const TEXT_HOECHSTENS: usize = 1024;

/// Escape (CWE-117) und Kürzung: `\` → `\\`; Steuer-, Zeilentrenner- und Richtungszeichen → `\u{…}` in
/// Kleinbuchstaben-Hex; länger als `hoechstens` Zeichen → gekürzt mit `…` am Ende, nie mitten in einem Escape.
pub fn neutralisieren(roh: &str, hoechstens: usize) -> String {
    let stuecke: Vec<String> = roh.chars().map(stueck).collect();
    let laenge: usize = stuecke.iter().map(|s| s.chars().count()).sum();
    if laenge <= hoechstens {
        return stuecke.concat();
    }
    let mut gekuerzt = String::new();
    let mut zeichen = 0;
    for stueck in &stuecke {
        let dazu = stueck.chars().count();
        if zeichen + dazu > hoechstens.saturating_sub(1) {
            break;
        }
        gekuerzt.push_str(stueck);
        zeichen += dazu;
    }
    gekuerzt.push('…');
    gekuerzt
}

/// Ein Zeichen in seiner Form im Protokoll.
fn stueck(zeichen: char) -> String {
    if zeichen == '\\' {
        "\\\\".to_string()
    } else if zu_escapen(zeichen) {
        format!("\\u{{{:x}}}", u32::from(zeichen))
    } else {
        zeichen.to_string()
    }
}

/// Steuerzeichen (C0, DEL, C1), Zeilentrenner und Richtungszeichen (ADR-0040 Regel 4, „Trojan Source“).
fn zu_escapen(zeichen: char) -> bool {
    matches!(zeichen,
        '\u{0}'..='\u{1f}'
        | '\u{7f}'..='\u{9f}'
        | '\u{2028}'
        | '\u{2029}'
        | '\u{200e}'
        | '\u{200f}'
        | '\u{202a}'..='\u{202e}'
        | '\u{2066}'..='\u{2069}')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zeit(text: &str) -> Zeit {
        Zeit::parse_from_rfc3339(text).expect("Testzeit")
    }

    #[test]
    fn erster_test_aus_adr_0040_genau_eine_zeile_escaped() {
        let eintrag = Eintrag::neu(
            Stufe::Warning,
            Bereich::Grenzschutz,
            "Anfrage mit fremdem Host abgewiesen",
            Kontext::neu()
                .mit(Schluessel::Herkunft, "10.0.0.9")
                .mit(Schluessel::Grund, "Host böse\nZEILE\u{1b}[31m\u{2028}x"),
        );

        assert_eq!(
            zeile(&eintrag, zeit("2026-10-28T14:03:07.123+01:00")),
            "2026-10-28T14:03:07.123+01:00 WARNING grenzschutz Anfrage mit fremdem Host abgewiesen | herkunft=10.0.0.9 grund=\"Host böse\\u{a}ZEILE\\u{1b}[31m\\u{2028}x\"\n"
        );
    }

    #[test]
    fn ohne_kontext_keine_trennung_und_millisekunden_immer_dreistellig() {
        let eintrag = Eintrag::neu(Stufe::Information, Bereich::Start, "bereit", Kontext::neu());
        assert_eq!(
            zeile(&eintrag, zeit("2026-03-29T03:00:00+02:00")),
            "2026-03-29T03:00:00.000+02:00 INFORMATION start bereit\n"
        );
    }

    #[test]
    fn werte_mit_leerzeichen_gleich_strich_oder_anfuehrung_stehen_in_anfuehrung() {
        let eintrag = Eintrag::neu(
            Stufe::Debug,
            Bereich::Koerper,
            "Text mit | Strich",
            Kontext::neu()
                .mit(Schluessel::Grund, "a=b|c \"d\"")
                .mit(Schluessel::Route, "GET /api/channels")
                .mit(Schluessel::Anzahl, 3)
                .mit(Schluessel::Typ, ""),
        );
        assert_eq!(
            zeile(&eintrag, zeit("2026-10-28T14:03:07.123+01:00")),
            "2026-10-28T14:03:07.123+01:00 DEBUG körper Text mit | Strich | grund=\"a=b|c \\\"d\\\"\" route=\"GET /api/channels\" anzahl=3 typ=\"\"\n"
        );
    }

    #[test]
    fn ein_zeilenumbruch_im_text_ergibt_keine_zweite_zeile() {
        let eintrag = Eintrag::neu(
            Stufe::Error,
            Bereich::Programmfehler,
            "erste\n2026-10-28T00:00:00.000+01:00 ERROR start gefälscht",
            Kontext::neu().mit(Schluessel::Name, "x\r\ny"),
        );
        let text = zeile(&eintrag, zeit("2026-10-28T14:03:07.123+01:00"));
        assert_eq!(text.matches('\n').count(), 1, "{text}");
        assert!(text.ends_with('\n'), "{text}");
        assert!(text.contains("erste\\u{a}2026"), "{text}");
        assert!(text.contains("name=x\\u{d}\\u{a}y"), "{text}");
    }

    #[test]
    fn stufen_sind_aufsteigend_geordnet() {
        assert!(Stufe::Debug < Stufe::Information);
        assert!(Stufe::Information < Stufe::Warning);
        assert!(Stufe::Warning < Stufe::Error);
        assert!(Stufe::Error < Stufe::Exception);
    }

    #[test]
    fn steuer_zeilentrenner_und_richtungszeichen_werden_escaped() {
        assert_eq!(
            neutralisieren("Host böse\nZEILE\u{1b}[31m\u{2028}x", WERT_HOECHSTENS),
            "Host böse\\u{a}ZEILE\\u{1b}[31m\\u{2028}x"
        );
        for (roh, erwartet) in [
            ("a\rb", "a\\u{d}b"),
            ("a\u{202e}b", "a\\u{202e}b"),
            ("a\u{2066}b", "a\\u{2066}b"),
            ("a\\b", "a\\\\b"),
            ("\u{0}\u{1f}\u{7f}\u{9f}", "\\u{0}\\u{1f}\\u{7f}\\u{9f}"),
            (
                "\u{2029}\u{200e}\u{200f}\u{202a}\u{2069}",
                "\\u{2029}\\u{200e}\\u{200f}\\u{202a}\\u{2069}",
            ),
        ] {
            assert_eq!(neutralisieren(roh, WERT_HOECHSTENS), erwartet, "{roh:?}");
        }
    }

    #[test]
    fn gewoehnliche_zeichen_und_umlaute_bleiben() {
        for roh in [
            "Anfrage mit fremdem Host abgewiesen",
            "äöüß €",
            "\u{a0}\u{2027}\u{202f}\u{206a}",
        ] {
            assert_eq!(neutralisieren(roh, WERT_HOECHSTENS), roh);
        }
    }

    #[test]
    fn ein_wert_wird_auf_256_zeichen_gekuerzt_mit_auslassung_am_ende() {
        let gekuerzt = neutralisieren(&"a".repeat(300), WERT_HOECHSTENS);
        assert_eq!(gekuerzt, format!("{}…", "a".repeat(255)));
        assert_eq!(gekuerzt.chars().count(), 256);
        let genau = "a".repeat(256);
        assert_eq!(neutralisieren(&genau, WERT_HOECHSTENS), genau);
        assert_eq!(
            neutralisieren(&"a".repeat(257), WERT_HOECHSTENS),
            format!("{}…", "a".repeat(255))
        );
    }

    #[test]
    fn ein_text_wird_auf_1024_zeichen_gekuerzt() {
        let gekuerzt = neutralisieren(&"ä".repeat(1100), TEXT_HOECHSTENS);
        assert_eq!(gekuerzt.chars().count(), 1024);
        assert!(gekuerzt.ends_with("ä…"), "{gekuerzt}");
    }

    #[test]
    fn gekuerzt_wird_nie_mitten_in_einem_escape() {
        let roh = format!("{}\n\n\n", "a".repeat(250));
        let gekuerzt = neutralisieren(&roh, WERT_HOECHSTENS);
        assert_eq!(gekuerzt, format!("{}\\u{{a}}…", "a".repeat(250)));
        assert!(gekuerzt.chars().count() <= WERT_HOECHSTENS);
    }
}
