//! Formatierer des Betriebsprotokolls (ADR-0040 Regeln 3–5): die einzige Stelle, an der Text und Werte
//! neutralisiert werden und eine Zeile entsteht. Rein, ohne Uhr und Datei.

use crate::uhr::Zeit;
use std::fmt::Write;

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
    /// Klein geschrieben, wie sie gewählt wird (`stufe=`, `protokoll_stufe`).
    pub fn als_wert(self) -> &'static str {
        match self {
            Stufe::Debug => "debug",
            Stufe::Information => "information",
            Stufe::Warning => "warning",
            Stufe::Error => "error",
            Stufe::Exception => "exception",
        }
    }

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
    /// Plattenquote (ADR-0042 Ergänzung Karte #34).
    Quote,
    /// Anhänge (ADR-0042 Ergänzung Karte #9 I0010).
    Anhang,
    /// Der Ereignisstrom (ADR-0042 Ergänzung Karte #31 Punkt 7).
    Live,
    /// Eigene Nachrichten ändern (ADR-0048 Regel 13).
    Aendern,
    /// Tippt / denkt nach (ADR-0042 Ergänzung „Karte #77“, ADR-0051 Regel 11).
    Aktivitaet,
    /// Warten auf Nachrichten (ADR-0054 Regel 8 und 10).
    Warten,
    /// Nachholen von `warten` am Nachholkonto (ADR-0054 Ergänzung „Nachholen von `warten` …“ Regel 8).
    Nachholen,
    /// Direktgespräche an der Grenze neuer Paare (ADR-0055 Regel 11).
    Direkt,
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
            Bereich::Quote => "quote",
            Bereich::Anhang => "anhang",
            Bereich::Live => "live",
            Bereich::Aendern => "aendern",
            Bereich::Aktivitaet => "aktivitaet",
            Bereich::Warten => "warten",
            Bereich::Nachholen => "nachholen",
            Bereich::Direkt => "direkt",
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
    /// Schlüssel einer Frage an die Namensauskunft: Name bzw. kanonische Adresse (ADR-0045 Ergänzung E6).
    Schluessel,
    /// Fremdtext einer technischen Ursache (ADR-0040 Ergänzung „eigener Schlüssel `ursache`“).
    Ursache,
    /// Zahl der Grenze, die gegriffen hat (ADR-0042 Ergänzung Karte #26 Punkt 1).
    Hoechstens,
    /// Wirksame Plattenquote in Bytes oder `aus` (ADR-0042 Ergänzung Karte #34).
    Plattenquote,
    /// Bytes, die die Gespräche belegen (ADR-0044 Regel 2).
    Belegt,
    /// Woher die Plattenquote kommt — eigener Schlüssel, `option` nennt die Herkunft der Protokollstufe.
    PlattenquoteOption,
    /// Alter eines verdrängten Wiederholschlüssels in ms (ADR-0042 Ergänzung Karte #36).
    AlterMs,
    /// Tag einer Tagesdatei bzw. eines Anhang-Ordners (`JJJJ-MM-TT`).
    Tag,
    /// Tag, der einen anderen Tag hält (`JJJJ-MM-TT`, ADR-0042 Ergänzung „Karte #73“ (3)).
    Ziel,
    /// Gedeutete Blöcke des Tilgers.
    Bloecke,
    /// Gelesene und geschriebene Bytes des Tilgers (ADR-0047 B1 Regel 4).
    Arbeit,
    /// Zahlen eines Löschens (ADR-0042 Ergänzung „F1′“).
    Anhaenge,
    Gespraeche,
    Fremde,
    Fehler,
    Ids,
    Unbekannt,
    SchonGeloescht,
    /// Lebensende einer Anmeldung (RFC 3339).
    HochladenBis,
    /// Rate in MiB/s.
    MibS,
    /// Länge eines Teilabrufs.
    Bereich,
    Reserviert,
    Frei,
    Gebunden,
    Anhangquote,
    AnhangBelegt,
    AnhangOption,
    /// Eine gespeicherte Anhangdatei, nur als Präfix `<hhmmssmmm>-<n>` (ADR-0047 Ergänzung Gate Regel 3).
    Anhang,
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
            Schluessel::Schluessel => "schluessel",
            Schluessel::Ursache => "ursache",
            Schluessel::Hoechstens => "hoechstens",
            Schluessel::Plattenquote => "plattenquote",
            Schluessel::Belegt => "belegt",
            Schluessel::PlattenquoteOption => "plattenquote_option",
            Schluessel::AlterMs => "alter_ms",
            Schluessel::Tag => "tag",
            Schluessel::Ziel => "ziel",
            Schluessel::Anhang => "anhang",
            Schluessel::Bloecke => "bloecke",
            Schluessel::Arbeit => "arbeit",
            Schluessel::Anhaenge => "anhaenge",
            Schluessel::Gespraeche => "gespraeche",
            Schluessel::Fremde => "fremde",
            Schluessel::Fehler => "fehler",
            Schluessel::Ids => "ids",
            Schluessel::Unbekannt => "unbekannt",
            Schluessel::SchonGeloescht => "schon_geloescht",
            Schluessel::HochladenBis => "hochladen_bis",
            Schluessel::MibS => "mib_s",
            Schluessel::Bereich => "bereich",
            Schluessel::Reserviert => "reserviert",
            Schluessel::Frei => "frei",
            Schluessel::Gebunden => "gebunden",
            Schluessel::Anhangquote => "anhangquote",
            Schluessel::AnhangBelegt => "anhang_belegt",
            Schluessel::AnhangOption => "anhang_option",
        }
    }
}

/// Benannte Werte eines Eintrags in der Reihenfolge, in der sie gesetzt werden. Welche Werte einer Episode in
/// welcher Zeile stehen, bestimmt `Auftreten` (ADR-0050), nicht der Kontext.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Kontext(Vec<(Schluessel, String)>);

impl Kontext {
    pub fn neu() -> Kontext {
        Kontext::default()
    }

    /// Setzt einen Wert, roh gekürzt auf `WERT_HOECHSTENS + 1` Zeichen (an der Quelle, ADR-0040 Ergänzung
    /// CWE-770: eine offene Episode hält nie mehr); neutralisiert wird erst in der Zeile.
    pub fn mit(mut self, schluessel: Schluessel, wert: impl ToString) -> Kontext {
        self.0.push((
            schluessel,
            roh_gekuerzt(wert.to_string(), WERT_HOECHSTENS + 1),
        ));
        self
    }

    /// Hängt die Werte eines anderen Kontexts hinten an.
    pub(super) fn und(mut self, anderer: &Kontext) -> Kontext {
        self.0.extend(anderer.0.iter().cloned());
        self
    }

    /// Schlüsselwort und Wert je Eintrag in der Reihenfolge des Setzens.
    #[cfg(test)]
    pub(crate) fn paare(&self) -> Vec<(&'static str, &str)> {
        self.0
            .iter()
            .map(|(schluessel, wert)| (schluessel.als_text(), wert.as_str()))
            .collect()
    }

    /// Die Werte in der Reihenfolge, in der sie gesetzt wurden (roh, an der Quelle gekürzt).
    pub(super) fn werte(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(|(_, wert)| wert.as_str())
    }

    /// Der Wert zu einem Schlüssel, falls gesetzt (roh, an der Quelle gekürzt).
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
            text: roh_gekuerzt(text.into(), TEXT_HOECHSTENS + 1),
            kontext,
        }
    }
}

/// Höchstens `zeichen` Zeichen, in einem String genau dieser Größe; das eine Zeichen über der Grenze bleibt, damit
/// der Formatierer erkennt, dass gekürzt wurde, und `…` setzt.
fn roh_gekuerzt(text: String, zeichen: usize) -> String {
    match text.char_indices().nth(zeichen) {
        Some((stelle, _)) => text[..stelle].to_string(),
        None => text,
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
        text_neutralisieren(&eintrag.text)
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

/// Der Text eines Eintrags: wie `neutralisieren`, dazu jedes `|` als `\u{7c}` — so ist das erste „ | “ einer
/// Zeile immer der Trenner zum Kontext (ADR-0040, Ergänzung „`|` im Text wird escaped“).
fn text_neutralisieren(roh: &str) -> String {
    neutralisieren_mit(roh, TEXT_HOECHSTENS, |zeichen| zeichen == '|')
}

/// Ein fremder Wert für eine Meldung an Menschen (stderr): in „…“, Steuer- und Richtungszeichen escaped und gekürzt
/// wie im Protokoll — statt der Rust-Anführung aus `{:?}` (R00018 F0062).
pub fn in_anfuehrung(roh: &str) -> String {
    format!("„{}“", neutralisieren(roh, WERT_HOECHSTENS))
}

/// Escape (CWE-117) und Kürzung: `\` → `\\`; Steuer-, Zeilentrenner- und Richtungszeichen → `\u{…}` in
/// Kleinbuchstaben-Hex; länger als `hoechstens` Zeichen → gekürzt mit `…` am Ende, nie mitten in einem Escape.
pub fn neutralisieren(roh: &str, hoechstens: usize) -> String {
    neutralisieren_mit(roh, hoechstens, |_| false)
}

/// `neutralisieren` mit weiteren Zeichen, die als `\u{…}` stehen. Zeichenweise in einen Puffer, ohne Zwischenstring
/// je Zeichen; hört auf, sobald die Grenze überschritten ist, und schneidet hinter dem letzten ganzen Stück, das
/// mit `…` noch passt (ADR-0040, Ergänzung „an der Quelle kürzen“).
fn neutralisieren_mit(roh: &str, hoechstens: usize, auch: impl Fn(char) -> bool) -> String {
    let mut ergebnis = String::new();
    let mut zeichen = 0;
    let mut schnitt = 0;
    for roh_zeichen in roh.chars() {
        zeichen += stueck_anhaengen(&mut ergebnis, roh_zeichen, &auch);
        if zeichen > hoechstens {
            ergebnis.truncate(schnitt);
            ergebnis.push('…');
            return ergebnis;
        }
        if zeichen < hoechstens {
            schnitt = ergebnis.len();
        }
    }
    ergebnis
}

/// Hängt ein Zeichen in seiner Form im Protokoll an; liefert, wie viele Zeichen das sind.
fn stueck_anhaengen(ziel: &mut String, zeichen: char, auch: impl Fn(char) -> bool) -> usize {
    if zeichen == '\\' {
        ziel.push_str("\\\\");
        2
    } else if zu_escapen(zeichen) || auch(zeichen) {
        let vorher = ziel.len();
        let _ = write!(ziel, "\\u{{{:x}}}", u32::from(zeichen));
        ziel.len() - vorher
    } else {
        ziel.push(zeichen);
        1
    }
}

/// Steuerzeichen (C0, DEL, C1), Zeilentrenner und Richtungszeichen (ADR-0040 Regel 4, „Trojan Source“).
fn zu_escapen(zeichen: char) -> bool {
    matches!(zeichen,
        '\u{0}'..='\u{1f}'
        | '\u{7f}'..='\u{9f}'
        | '\u{61c}'
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
            "2026-10-28T14:03:07.123+01:00 DEBUG körper Text mit \\u{7c} Strich | grund=\"a=b|c \\\"d\\\"\" route=\"GET /api/channels\" anzahl=3 typ=\"\"\n"
        );
    }

    /// ADR-0040, Ergänzung „`|` im Text wird escaped“ (Code-Review #24, H1): ein fremder Text täuscht keinen
    /// Kontext vor — das erste „ | “ einer Zeile ist immer der Trenner.
    #[test]
    fn ein_strich_im_text_taeuscht_keinen_kontext_vor() {
        let jetzt = zeit("2026-10-28T14:03:07.123+01:00");
        let ohne = Eintrag::neu(
            Stufe::Exception,
            Bereich::Programmfehler,
            "x | kennung=admin",
            Kontext::neu(),
        );
        let mit = Eintrag::neu(
            Stufe::Exception,
            Bereich::Programmfehler,
            "x | kennung=admin",
            Kontext::neu().mit(Schluessel::Grund, "panik"),
        );

        let ohne = zeile(&ohne, jetzt);
        let mit = zeile(&mit, jetzt);

        assert!(ohne.contains("x \\u{7c} kennung=admin"), "{ohne}");
        assert_eq!(ohne.matches(" | ").count(), 0, "{ohne}");
        assert_eq!(mit.matches(" | ").count(), 1, "{mit}");
        assert!(mit.ends_with(" | grund=panik\n"), "{mit}");
    }

    /// ADR-0040, Ergänzung „U+061C“ (Security-Review #24, O2): ein Richtungszeichen wie U+200E/U+200F.
    #[test]
    fn das_arabische_richtungszeichen_wird_in_text_und_wert_escaped() {
        let eintrag = Eintrag::neu(
            Stufe::Warning,
            Bereich::Grenzschutz,
            "a\u{61c}b",
            Kontext::neu().mit(Schluessel::Grund, "c\u{61c}d"),
        );

        let zeile = zeile(&eintrag, zeit("2026-10-28T14:03:07.123+01:00"));

        assert!(zeile.contains(" a\\u{61c}b | "), "{zeile}");
        assert!(zeile.ends_with("grund=c\\u{61c}d\n"), "{zeile}");
        assert!(!zeile.contains('\u{61c}'), "{zeile}");
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

    /// ADR-0040, Ergänzung „an der Quelle kürzen“ (Security-Review #24, O1, CWE-770): der Kontext hält einen
    /// riesigen Wert nie ganz — auch nicht in seiner Kapazität —, die Zeile bleibt wie zuvor.
    #[test]
    fn ein_riesiger_wert_wird_schon_beim_eintritt_in_den_kontext_gekuerzt() {
        let kontext = Kontext::neu().mit(Schluessel::Grund, "x".repeat(400_000));

        let gehalten = &kontext.0[0].1;
        assert!(
            gehalten.chars().count() <= WERT_HOECHSTENS + 1,
            "{}",
            gehalten.len()
        );
        assert!(
            gehalten.capacity() <= 4 * (WERT_HOECHSTENS + 1),
            "{}",
            gehalten.capacity()
        );
        let eintrag = Eintrag::neu(Stufe::Warning, Bereich::Grenzschutz, "Host", kontext);
        assert!(
            zeile(&eintrag, zeit("2026-10-28T14:03:07.123+01:00"))
                .ends_with(&format!(" | grund={}…\n", "x".repeat(255)))
        );
    }

    #[test]
    fn ein_riesiger_text_wird_schon_beim_eintritt_in_den_eintrag_gekuerzt() {
        let eintrag = Eintrag::neu(
            Stufe::Exception,
            Bereich::Programmfehler,
            "ä".repeat(400_000),
            Kontext::neu(),
        );

        assert!(eintrag.text.chars().count() <= TEXT_HOECHSTENS + 1);
        assert!(eintrag.text.capacity() <= 4 * (TEXT_HOECHSTENS + 1));
        assert!(
            zeile(&eintrag, zeit("2026-10-28T14:03:07.123+01:00"))
                .ends_with(&format!(" programmfehler {}…\n", "ä".repeat(1023)))
        );
    }

    #[test]
    fn genau_an_der_grenze_bleibt_ein_wert_ungekuerzt_eins_darueber_nicht() {
        for (laenge, erwartet) in [
            (256, "y".repeat(256)),
            (257, format!("{}…", "y".repeat(255))),
            (258, format!("{}…", "y".repeat(255))),
        ] {
            let kontext = Kontext::neu().mit(Schluessel::Name, "y".repeat(laenge));
            let eintrag = Eintrag::neu(Stufe::Debug, Bereich::Start, "t", kontext);
            assert!(
                zeile(&eintrag, zeit("2026-10-28T14:03:07.123+01:00"))
                    .ends_with(&format!("name={erwartet}\n")),
                "{laenge}"
            );
        }
    }

    #[test]
    fn gekuerzt_wird_nie_mitten_in_einem_escape() {
        let roh = format!("{}\n\n\n", "a".repeat(250));
        let gekuerzt = neutralisieren(&roh, WERT_HOECHSTENS);
        assert_eq!(gekuerzt, format!("{}\\u{{a}}…", "a".repeat(250)));
        assert!(gekuerzt.chars().count() <= WERT_HOECHSTENS);
    }
}
