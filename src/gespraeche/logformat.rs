//! Logformat (A00007, ADR-0004): Kopfzeile `@ <zeit> <kennung>`, Metazeilen `+ `, Text mit zwei Leerzeichen.
//!
//! Rein, ohne Platte: übersetzt zwischen Rohtext einer Tagesdatei und Sätzen.

use super::anhangid::Anhangid;
use super::anhangpruefung::erkenner::ARTEN;
use super::{Art, Kennung, Nachricht};
use crate::uhr::Zeit;
use chrono::DateTime;
use std::borrow::Cow;

/// Ein gültiger Satz im Rohtext: Kopf und Byte-Spanne `anfang..ende` (bis vor den nächsten Satz).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SatzBereich {
    pub zeit: Zeit,
    pub von: Kennung,
    pub anfang: usize,
    pub ende: usize,
}

/// Eine defekte Zeile (1-basiert) — oder die Kopfzeile eines Satzes, der als Ganzes defekt ist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Defekt {
    pub zeile: usize,
}

/// Der Satz einer Nachricht: Kopfzeile, Text mit zwei Leerzeichen eingerückt (leere Zeile bleibt leer),
/// `\r\n` als `\n`, ohne abschließende Leerzeilen, danach eine Leerzeile als Abschluss.
pub fn satz(nachricht: &Nachricht) -> String {
    let mut satz = format!("@ {} {}\n", zeit_im_log(&nachricht.zeit), nachricht.von);
    if let Some(art) = nachricht.art {
        satz.push_str(&format!("{ART_METAZEILE}{}\n", art_als_text(art)));
    }
    if let Some(bezug) = &nachricht.bezug {
        satz.push_str(&format!(
            "{ANTWORT_METAZEILE}{} {}\n",
            zeit_im_log(&bezug.zeit),
            bezug.von
        ));
    }
    for anhang in &nachricht.anhaenge {
        satz.push_str(&anhangzeile(anhang));
        satz.push('\n');
    }
    text_einruecken(&mut satz, &nachricht.text, !nachricht.anhaenge.is_empty());
    satz
}

/// Metazeile des Bezugs einer Antwort (ADR-0053 Regel 1): `+ antwort_auf: <zeit> <kennung>`.
const ANTWORT_METAZEILE: &str = "+ antwort_auf: ";

/// Worauf eine Antwort sich bezieht: Zeit und Kennung des Originals aus dessen Kopfzeile (ADR-0053 Regel 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Antwortbezug {
    pub zeit: Zeit,
    pub von: Kennung,
}

/// Warum eine Metazeile `+ antwort_auf:` nicht gilt — beim Laden `WARNING laden`, die Nachricht bleibt ohne Bezug.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Antwortdefekt {
    /// Mehr als eine Metazeile `+ antwort_auf:`.
    ZweiBezuege,
    /// Die Zeit ist kein RFC 3339.
    Zeit,
    /// Die Kennung fehlt oder ist keine Kennung.
    Kennung,
    /// Die Zeit des Originals liegt nicht echt vor der eigenen Zeit.
    NichtVorher,
}

/// Der Bezug einer Antwort aus den Metazeilen direkt nach dem Kopf; ohne Metazeile keiner; eingerückter Text zählt
/// nie. Rein.
pub fn antwortbezug(satz: &[u8]) -> Result<Option<Antwortbezug>, Antwortdefekt> {
    deuten(satz).antwort
}

/// Was die Metazeilen direkt nach dem Kopf über einen Satz sagen: seine Art und der Bezug einer Antwort — in einem
/// Durchgang über die Metazeilen (ADR-0053 Regel 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deutung {
    pub art: Satzart,
    pub antwort: Result<Option<Antwortbezug>, Antwortdefekt>,
}

/// Deutet Art und Antwortbezug eines Satzes (ab seinem Kopf) über die Metazeilen direkt nach dem Kopf; eingerückter
/// Text zählt nie (M14). Rein.
pub fn deuten(satz: &[u8]) -> Deutung {
    let mut bezuege = Vec::new();
    let mut antworten = Vec::new();
    let mut traegt_anhang = false;
    for zeile in metazeilen(satz) {
        traegt_anhang |= zeile.starts_with(ANHANG_METAZEILE.as_bytes());
        for (metazeile, bezug) in [
            (BEARBEITET_METAZEILE, Bezug::Bearbeitet),
            (GELOESCHT_METAZEILE, Bezug::Geloescht),
        ] {
            if let Some(wert) = zeile.strip_prefix(metazeile.as_bytes()) {
                bezuege.push((bezug, wert));
            }
        }
        if let Some(wert) = zeile.strip_prefix(ANTWORT_METAZEILE.as_bytes()) {
            antworten.push(wert);
        }
    }
    Deutung {
        art: art_aus_bezuegen(satz, &bezuege, traegt_anhang),
        antwort: antwort_aus_metazeilen(satz, &antworten),
    }
}

/// Die Metazeilen `+ …` direkt nach dem Kopf.
fn metazeilen(satz: &[u8]) -> impl Iterator<Item = &[u8]> {
    satz.split(|byte| *byte == b'\n')
        .skip(1)
        .take_while(|zeile| zeile.starts_with(b"+ "))
}

/// Der Bezug aus den Werten der Metazeilen `+ antwort_auf:`: höchstens eine, Zeit in RFC 3339, eine Kennung, die Zeit
/// echt vor der eigenen.
fn antwort_aus_metazeilen(
    satz: &[u8],
    werte: &[&[u8]],
) -> Result<Option<Antwortbezug>, Antwortdefekt> {
    let wert = match werte {
        [] => return Ok(None),
        [wert] => *wert,
        _ => return Err(Antwortdefekt::ZweiBezuege),
    };
    let wert = std::str::from_utf8(wert).map_err(|_| Antwortdefekt::Zeit)?;
    let (zeit, von) = wert.split_once(' ').unwrap_or((wert, ""));
    let zeit = DateTime::parse_from_rfc3339(zeit).map_err(|_| Antwortdefekt::Zeit)?;
    let von = Kennung::bilden(von).map_err(|_| Antwortdefekt::Kennung)?;
    if kopfzeit(satz).is_none_or(|eigene| zeit >= eigene) {
        return Err(Antwortdefekt::NichtVorher);
    }
    Ok(Some(Antwortbezug { zeit, von }))
}

/// Bezugs-Metazeile eines Bearbeitungssatzes (ADR-0048 Regel 1).
const BEARBEITET_METAZEILE: &str = "+ bearbeitet: ";
/// Bezugs-Metazeile eines Löschsatzes (ADR-0048 Regel 1).
const GELOESCHT_METAZEILE: &str = "+ geloescht: ";

/// Worauf sich ein Änderungssatz bezieht: die Nachricht bekommt einen neuen Text oder ist gelöscht.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bezug {
    Bearbeitet,
    Geloescht,
}

/// Warum ein Satz mit Bezugs-Metazeile nicht gilt (ADR-0048 Regel 1) — beim Laden `WARNING laden`, nie still.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Aenderungsdefekt {
    /// Mehr als eine Bezugs-Metazeile.
    ZweiBezuege,
    /// Die Bezugszeit ist kein RFC 3339.
    Bezugszeit,
    /// Die Bezugszeit liegt nicht echt vor der eigenen Zeit.
    NichtVorher,
    /// Ein Änderungssatz trägt `+ anhang:`.
    Anhang,
}

/// Was ein gültiger Satz ist: eine Nachricht, eine Änderung an der Nachricht mit der Zeit `ziel` oder ein
/// defekter Änderungssatz — der ist weder Nachricht noch Änderung.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Satzart {
    Nachricht,
    Aenderung { ziel: Zeit, bezug: Bezug },
    DefekteAenderung(Aenderungsdefekt),
}

/// Deutet einen Satz (ab seinem Kopf) über die Metazeilen direkt nach dem Kopf; eingerückter Text zählt nie
/// (M14). Rein.
pub fn satzart(satz: &[u8]) -> Satzart {
    deuten(satz).art
}

/// Die Art eines Satzes aus seinen Bezugs-Metazeilen `+ bearbeitet:` / `+ geloescht:` (ADR-0048 Regel 1).
fn art_aus_bezuegen(satz: &[u8], bezuege: &[(Bezug, &[u8])], traegt_anhang: bool) -> Satzart {
    let [(bezug, wert)] = bezuege[..] else {
        return if bezuege.is_empty() {
            Satzart::Nachricht
        } else {
            Satzart::DefekteAenderung(Aenderungsdefekt::ZweiBezuege)
        };
    };
    let Some(ziel) = std::str::from_utf8(wert)
        .ok()
        .and_then(|wert| DateTime::parse_from_rfc3339(wert).ok())
    else {
        return Satzart::DefekteAenderung(Aenderungsdefekt::Bezugszeit);
    };
    if kopfzeit(satz).is_none_or(|eigene| ziel >= eigene) {
        return Satzart::DefekteAenderung(Aenderungsdefekt::NichtVorher);
    }
    if traegt_anhang {
        return Satzart::DefekteAenderung(Aenderungsdefekt::Anhang);
    }
    Satzart::Aenderung { ziel, bezug }
}

/// Der Bearbeitungssatz (ADR-0048 Regel 1): eigene Kopfzeile, genau eine Bezugs-Metazeile, darunter der ganze
/// neue Text eingerückt wie bei einer Nachricht; ein leerer Text ist nur die abschließende Leerzeile.
pub fn bearbeitungssatz(zeit: Zeit, von: &Kennung, ziel: Zeit, text: &str) -> String {
    let mut satz = format!(
        "@ {} {von}\n{BEARBEITET_METAZEILE}{}\n",
        zeit_im_log(&zeit),
        zeit_im_log(&ziel)
    );
    text_einruecken(&mut satz, text, true);
    satz
}

/// Der Löschsatz (ADR-0048 Regel 1): eigene Kopfzeile, genau eine Bezugs-Metazeile `+ geloescht:`, kein Text — nur
/// die abschließende Leerzeile.
pub fn loeschsatz(zeit: Zeit, von: &Kennung, ziel: Zeit) -> String {
    format!(
        "@ {} {von}\n{GELOESCHT_METAZEILE}{}\n\n",
        zeit_im_log(&zeit),
        zeit_im_log(&ziel)
    )
}

/// Der Text so, wie ihn ein Satz nach dem Zurücklesen liefert (`\r\n` als `\n`, ohne abschließende Leerzeilen).
pub fn text_wie_im_log(text: &str) -> String {
    let mut satz = String::from("@ x\n");
    text_einruecken(&mut satz, text, true);
    self::text(&satz)
}

/// Eine Zeit, wie sie im Log steht: Millisekunden und Versatz (ADR-0004).
fn zeit_im_log(zeit: &Zeit) -> chrono::format::DelayedFormat<chrono::format::StrftimeItems<'_>> {
    zeit.format("%Y-%m-%dT%H:%M:%S%.3f%:z")
}

/// Hängt den Text eingerückt an (leere Zeile bleibt leer, `\r\n` als `\n`, ohne abschließende Leerzeilen) und
/// schließt mit einer Leerzeile; ist der Text leer und `leer_erlaubt`, nur die Leerzeile.
fn text_einruecken(satz: &mut String, text: &str, leer_erlaubt: bool) {
    let text = text.replace("\r\n", "\n");
    let zeilen = text.trim_end_matches('\n');
    if zeilen.is_empty() && leer_erlaubt {
        satz.push('\n');
        return;
    }
    for zeile in zeilen.split('\n') {
        if !zeile.is_empty() {
            satz.push_str("  ");
            satz.push_str(zeile);
        }
        satz.push('\n');
    }
    satz.push('\n');
}

/// Der Text eines Satzes ohne Kopf, Metazeilen und Einrückung (Umkehrung von `satz`).
pub fn text(satz: &str) -> String {
    let zeilen: Vec<&str> = satz
        .split('\n')
        .filter_map(|zeile| match zeilenart(zeile) {
            Zeilenart::Satzinhalt => zeile.strip_prefix("  "),
            Zeilenart::Leer => Some(""),
            Zeilenart::Kopf | Zeilenart::Defekt => None,
        })
        .collect();
    zeilen.join("\n").trim_end_matches('\n').to_string()
}

/// Ein kopierter Satz als Nachricht (ADR-0025 Regel 3): nur der Satz am Anfang, Text ohne Einrückung,
/// ungültiges UTF-8 ersetzt; ohne gültigen Kopf am Anfang keine.
pub fn satz_als_nachricht(satz: &[u8]) -> Option<Nachricht> {
    let bereich = saetze(satz)
        .into_iter()
        .find_map(Result::ok)
        .filter(|bereich| bereich.anfang == 0)?;
    let roh = String::from_utf8_lossy(&satz[..bereich.ende]);
    Some(Nachricht {
        zeit: bereich.zeit,
        von: bereich.von,
        art: art_lesen(&roh),
        text: text(&roh),
        anhaenge: anhaenge_im_satz(&roh)
            .into_iter()
            .filter_map(|(_, gelesen)| gelesen.ok())
            .collect(),
        geaendert: None,
        bezug: antwortbezug(&satz[..bereich.ende]).ok().flatten(),
    })
}

/// Metazeile eines Anhangs (ADR-0046 Regel 7).
const ANHANG_METAZEILE: &str = "+ anhang: ";

/// Ein Anhang, wie er in seiner Metazeile steht: `+ anhang: <id> <bytes> <art> <b>x<h>|- <datei> <anzeigename>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Anhangzeile {
    pub id: Anhangid,
    pub bytes: u64,
    /// Eine Art der Tabelle (ADR-0047 Frage 3).
    pub art: &'static str,
    pub masse: Option<(u32, u32)>,
    /// Gespeicherter Name `<hhmmssmmm>-<n>-<ascii>` unter `anhaenge/<tag>/`.
    pub datei: String,
    pub name: String,
    /// Gelöscht (F0116) — nicht Teil der Metazeile, gesetzt beim Lesen aus dem Verzeichnis.
    pub geloescht: bool,
}

/// Warum eine Metazeile `+ anhang:` nicht gilt — kein Defekt des Satzes (ADR-0004 Ergänzung Karte #9), aber kein
/// Eintrag (M17).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anhangbefund {
    Id,
    Bytes,
    Art,
    Masse,
    Datei,
    Name,
}

/// Die Metazeile eines Anhangs, ohne Zeilenende.
pub fn anhangzeile(anhang: &Anhangzeile) -> String {
    let masse = anhang.masse.map_or_else(
        || "-".to_string(),
        |(breite, hoehe)| format!("{breite}x{hoehe}"),
    );
    format!(
        "{ANHANG_METAZEILE}{} {} {} {masse} {} {}",
        anhang.id.als_text(),
        anhang.bytes,
        anhang.art,
        anhang.datei,
        anhang.name
    )
}

/// Liest den Teil hinter `+ anhang: `; die Art ist eine der Tabelle (sie darf Leerzeichen tragen).
pub fn anhangzeile_lesen(rest: &str) -> Result<Anhangzeile, Anhangbefund> {
    let (id, rest) = rest.split_once(' ').ok_or(Anhangbefund::Id)?;
    let id = Anhangid::lesen(id).ok_or(Anhangbefund::Id)?;
    let (bytes, rest) = rest.split_once(' ').ok_or(Anhangbefund::Bytes)?;
    if bytes.is_empty() || !bytes.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(Anhangbefund::Bytes);
    }
    let bytes: u64 = bytes.parse().map_err(|_| Anhangbefund::Bytes)?;
    if bytes == 0 {
        return Err(Anhangbefund::Bytes);
    }
    let (art, rest) = ARTEN
        .iter()
        .find_map(|art| {
            rest.strip_prefix(art)
                .and_then(|rest| rest.strip_prefix(' '))
                .map(|rest| (*art, rest))
        })
        .ok_or(Anhangbefund::Art)?;
    let (masse, rest) = rest.split_once(' ').ok_or(Anhangbefund::Masse)?;
    let masse = masse_lesen(masse)?;
    let (datei, name) = rest.split_once(' ').ok_or(Anhangbefund::Datei)?;
    if !datei_gueltig(datei) {
        return Err(Anhangbefund::Datei);
    }
    if name.is_empty() {
        return Err(Anhangbefund::Name);
    }
    Ok(Anhangzeile {
        id,
        bytes,
        art,
        masse,
        datei: datei.to_string(),
        name: name.to_string(),
        geloescht: false,
    })
}

/// `-` oder `<breite>x<hoehe>` mit Zahlen ab 1.
fn masse_lesen(text: &str) -> Result<Option<(u32, u32)>, Anhangbefund> {
    if text == "-" {
        return Ok(None);
    }
    let zahl = |teil: &str| -> Option<u32> {
        (!teil.is_empty() && teil.bytes().all(|byte| byte.is_ascii_digit()))
            .then(|| teil.parse().ok())
            .flatten()
            .filter(|wert| *wert > 0)
    };
    let (breite, hoehe) = text.split_once('x').ok_or(Anhangbefund::Masse)?;
    match (zahl(breite), zahl(hoehe)) {
        (Some(breite), Some(hoehe)) => Ok(Some((breite, hoehe))),
        _ => Err(Anhangbefund::Masse),
    }
}

/// `<hhmmssmmm>-<n>-<ascii>`: 9 Ziffern, Bindestrich, Nummer, Bindestrich, ASCII-Teil aus `[A-Za-z0-9._-]`; nie `/`,
/// nie `.` oder `..` als Ganzes (M17).
pub fn datei_gueltig(datei: &str) -> bool {
    let Some((zeit, rest)) = datei.split_at_checked(9) else {
        return false;
    };
    let Some(rest) = rest.strip_prefix('-') else {
        return false;
    };
    let Some((nummer, ascii)) = rest.split_once('-') else {
        return false;
    };
    zeit.bytes().all(|byte| byte.is_ascii_digit())
        && !nummer.is_empty()
        && nummer.bytes().all(|byte| byte.is_ascii_digit())
        && !ascii.is_empty()
        && ascii
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

/// Die Metazeilen `+ anhang:` direkt nach dem Kopf, mit ihrer Zeile im Satz (1-basiert); eingerückter Text zählt nie
/// (M16).
pub fn anhaenge_im_satz(satz: &str) -> Vec<(usize, Result<Anhangzeile, Anhangbefund>)> {
    satz.split('\n')
        .enumerate()
        .skip(1)
        .take_while(|(_, zeile)| zeile.starts_with("+ "))
        .filter_map(|(nummer, zeile)| {
            zeile
                .strip_prefix(ANHANG_METAZEILE)
                .map(|rest| (nummer + 1, anhangzeile_lesen(rest)))
        })
        .collect()
}

/// Metazeile der Art des Absenders (ADR-0034 Regel 5).
const ART_METAZEILE: &str = "+ art: ";

fn art_als_text(art: Art) -> &'static str {
    match art {
        Art::Mensch => "mensch",
        Art::Agent => "agent",
    }
}

/// Die Art aus den Metazeilen direkt nach dem Kopf; ein unbekannter Wert ist keine Art.
fn art_lesen(satz: &str) -> Option<Art> {
    let wert = satz
        .split('\n')
        .skip(1)
        .take_while(|zeile| zeile.starts_with("+ "))
        .find_map(|zeile| zeile.strip_prefix(ART_METAZEILE))?;
    match wert {
        "mensch" => Some(Art::Mensch),
        "agent" => Some(Art::Agent),
        _ => None,
    }
}

/// Die Zeit aus der Kopfzeile am Anfang von `satz` — nur die erste Zeile wird gelesen.
pub fn kopfzeit(satz: &[u8]) -> Option<Zeit> {
    let kopf = satz.strip_prefix(b"@ ")?;
    let ende = kopf
        .iter()
        .position(|&byte| byte == b' ' || byte == b'\n')?;
    let zeit = std::str::from_utf8(&kopf[..ende]).ok()?;
    DateTime::parse_from_rfc3339(zeit).ok()
}

/// Zeit und Kennung aus der Kopfzeile am Anfang von `satz`; ohne gültigen Kopf keine.
pub fn kopf(satz: &[u8]) -> Option<(Zeit, Kennung)> {
    let erste = satz.split(|byte| *byte == b'\n').next()?;
    kopf_lesen(std::str::from_utf8(erste).ok()?)
}

/// Zerlegt die Bytes einer Tagesdatei in Sätze und Defekte, in der Reihenfolge, in der sie abgeschlossen werden.
/// Die Bytespannen beziehen sich auf die Rohbytes. Eine Zeile, die kein gültiges UTF-8 ist, ist ein Defekt
/// ihrer Zeile — als Kopfzeile ein Defekt des ganzen Satzes —, nie des ganzen Tages (ADR-0011).
pub fn saetze(rohtext: &[u8]) -> Vec<Result<SatzBereich, Defekt>> {
    saetze_und_metazeilen(rohtext).0
}

/// Wie `saetze`, dazu ob eine Zeile mit `+ anhang:` beginnt — im selben Durchgang (Ladezeit, B0442).
pub fn saetze_und_metazeilen(rohtext: &[u8]) -> (Vec<Result<SatzBereich, Defekt>>, bool) {
    let mut traegt_anhang = false;
    let mut ergebnis = Vec::new();
    let mut offen = Offen::Anfang;
    let mut anfang = 0;
    for (index, zeile) in rohtext.split_inclusive(|&byte| byte == b'\n').enumerate() {
        let nummer = index + 1;
        let roh = zeile.strip_suffix(b"\n").unwrap_or(zeile);
        let (inhalt, gueltig) = match std::str::from_utf8(roh) {
            Ok(inhalt) => (Cow::Borrowed(inhalt), true),
            Err(_) => (String::from_utf8_lossy(roh), false),
        };
        traegt_anhang |= inhalt.starts_with(ANHANG_METAZEILE);
        match zeilenart(&inhalt) {
            Zeilenart::Kopf => {
                offen.schliessen(anfang, &mut ergebnis);
                let kopf = if gueltig { kopf_lesen(&inhalt) } else { None };
                offen = match kopf {
                    Some((zeit, von)) => Offen::Satz { zeit, von, anfang },
                    None => {
                        ergebnis.push(Err(Defekt { zeile: nummer }));
                        Offen::DefekterSatz
                    }
                };
            }
            Zeilenart::Satzinhalt if matches!(offen, Offen::Anfang) => {
                ergebnis.push(Err(Defekt { zeile: nummer }));
                offen = Offen::DefekterSatz;
            }
            Zeilenart::Satzinhalt if !gueltig && matches!(offen, Offen::Satz { .. }) => {
                ergebnis.push(Err(Defekt { zeile: nummer }));
            }
            Zeilenart::Satzinhalt | Zeilenart::Leer => {}
            Zeilenart::Defekt => ergebnis.push(Err(Defekt { zeile: nummer })),
        }
        anfang += zeile.len();
    }
    offen.schliessen(rohtext.len(), &mut ergebnis);
    (ergebnis, traegt_anhang)
}

/// Was beim Lesen gerade offen ist: noch kein Satz, ein gültiger oder ein als Ganzes defekter.
enum Offen {
    Anfang,
    Satz {
        zeit: Zeit,
        von: Kennung,
        anfang: usize,
    },
    DefekterSatz,
}

impl Offen {
    fn schliessen(&mut self, ende: usize, ergebnis: &mut Vec<Result<SatzBereich, Defekt>>) {
        if let Offen::Satz { zeit, von, anfang } = std::mem::replace(self, Offen::Anfang) {
            ergebnis.push(Ok(SatzBereich {
                zeit,
                von,
                anfang,
                ende,
            }));
        }
    }
}

enum Zeilenart {
    Kopf,
    Satzinhalt,
    Leer,
    Defekt,
}

fn zeilenart(inhalt: &str) -> Zeilenart {
    if inhalt.starts_with("@ ") {
        Zeilenart::Kopf
    } else if inhalt.starts_with("+ ") || inhalt.starts_with("  ") {
        Zeilenart::Satzinhalt
    } else if inhalt.is_empty() {
        Zeilenart::Leer
    } else {
        Zeilenart::Defekt
    }
}

/// `@ <zeit> <kennung>` mit genau einem Leerzeichen zwischen den Feldern; sonst keiner.
fn kopf_lesen(inhalt: &str) -> Option<(Zeit, Kennung)> {
    let felder = inhalt.strip_prefix("@ ")?;
    let (zeit, von) = felder.split_once(' ')?;
    let zeit = DateTime::parse_from_rfc3339(zeit).ok()?;
    let von = Kennung::bilden(von).ok()?;
    Some((zeit, von))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TAG_27: &str = "@ 2026-10-27T10:00:00.000+01:00 shoff@127.0.0.1\n  Wie weit?\n\nkaputt\n@ 2026-10-27T10:00:05.000+01:00 claude@10.0.0.5\n  Fertig.\n\n";

    fn gueltige(rohtext: &str) -> Vec<SatzBereich> {
        gueltige_roh(rohtext.as_bytes())
    }

    fn defekte(rohtext: &str) -> Vec<usize> {
        defekte_roh(rohtext.as_bytes())
    }

    fn anhang(
        id: &str,
        bytes: u64,
        art: &'static str,
        masse: Option<(u32, u32)>,
        datei: &str,
        name: &str,
    ) -> Anhangzeile {
        Anhangzeile {
            id: Anhangid::lesen(id).expect("ID"),
            bytes,
            art,
            masse,
            datei: datei.to_string(),
            name: name.to_string(),
            geloescht: false,
        }
    }

    /// B0437 (ADR-0046 Regel 7): je Anhang eine Metazeile nach `+ art:`, in Sendefolge; die Art mit Leerzeichen
    /// (`text/plain; charset=utf-8`) und der Anzeigename mit Leerzeichen lesen sich zurück.
    #[test]
    fn metazeilen_der_anhaenge_schreiben_und_lesen() {
        let mut gesendet = nachricht(
            "2026-10-29T14:07:12.345+01:00",
            "beta@127.0.0.1",
            "Hier die Belege",
        );
        gesendet.art = Some(Art::Agent);
        gesendet.anhaenge = vec![
            anhang(
                "k3v7q2m5xpc4h6n6tewaz2b5yr",
                2_457_600,
                "application/pdf",
                None,
                "140712345-1-Bericht_M_rz.pdf",
                "Bericht März.pdf",
            ),
            anhang(
                "aaaaaaaaaaaaaaaaaaaaaaaaab",
                12,
                "text/plain; charset=utf-8",
                None,
                "140712345-2-notiz.txt",
                " zwei  Leer ",
            ),
            anhang(
                "aaaaaaaaaaaaaaaaaaaaaaaaac",
                3,
                "image/jpeg",
                Some((3000, 4000)),
                "140712345-3-hochkant.jpg",
                "hochkant.jpg",
            ),
        ];

        let geschrieben = satz(&gesendet);

        assert_eq!(
            geschrieben,
            "@ 2026-10-29T14:07:12.345+01:00 beta@127.0.0.1\n+ art: agent\n\
             + anhang: k3v7q2m5xpc4h6n6tewaz2b5yr 2457600 application/pdf - 140712345-1-Bericht_M_rz.pdf Bericht März.pdf\n\
             + anhang: aaaaaaaaaaaaaaaaaaaaaaaaab 12 text/plain; charset=utf-8 - 140712345-2-notiz.txt  zwei  Leer \n\
             + anhang: aaaaaaaaaaaaaaaaaaaaaaaaac 3 image/jpeg 3000x4000 140712345-3-hochkant.jpg hochkant.jpg\n\
             \x20 Hier die Belege\n\n"
        );
        assert_eq!(satz_als_nachricht(geschrieben.as_bytes()), Some(gesendet));
    }

    /// ADR-0004 Ergänzung Karte #9, erster Test: Kopf, `+ art: mensch`, `+ anhang: …`, Leerzeile, nächster Satz →
    /// zwei Sätze, der erste mit Text `""` und einem Anhang; geschrieben ohne Textzeile.
    #[test]
    fn ein_satz_ohne_text_mit_anhang() {
        let rohtext = "@ 2026-10-29T14:00:00.000+01:00 a@127.0.0.1\n+ art: mensch\n+ anhang: aaaaaaaaaaaaaaaaaaaaaaaaaa 5 application/octet-stream - 140000000-1-x.bin x.bin\n\n@ 2026-10-29T14:00:01.000+01:00 b@127.0.0.1\n  zwei\n\n";
        let saetze = gueltige(rohtext);
        assert_eq!(saetze.len(), 2);
        assert!(defekte(rohtext).is_empty());
        let erster = satz_als_nachricht(&rohtext.as_bytes()[saetze[0].anfang..saetze[0].ende])
            .expect("Satz");
        assert_eq!(erster.text, "");
        assert_eq!(erster.anhaenge.len(), 1);
        assert_eq!(satz(&erster), &rohtext[saetze[0].anfang..saetze[0].ende]);
    }

    /// M16: eine Zeile `+ anhang: …` im Text bleibt Text (eingerückt) — kein Anhang.
    #[test]
    fn eine_eingeschleuste_metazeile_bleibt_text() {
        let gesendet = nachricht(
            "2026-10-29T14:00:00.000+01:00",
            "a@127.0.0.1",
            "+ anhang: aaaaaaaaaaaaaaaaaaaaaaaaaa 1 text/plain - x x\n+ anhang: zweite",
        );
        let geschrieben = satz(&gesendet);
        let gelesen = satz_als_nachricht(geschrieben.as_bytes()).expect("Satz");
        assert!(gelesen.anhaenge.is_empty());
        assert_eq!(gelesen.text, gesendet.text);
        assert!(anhaenge_im_satz(&geschrieben).is_empty());
    }

    /// M17 (Form): ID falscher Form, Bytes keine Zahl oder 0, Art außerhalb der Tabelle, Maße 0, Datei mit `/`,
    /// `..`, ohne Präfix, Name leer → je ein Befund, kein Defekt des Satzes.
    #[test]
    fn unplausible_metazeilen_sind_befunde() {
        let gut = "aaaaaaaaaaaaaaaaaaaaaaaaaa 5 application/octet-stream - 140000000-1-x.bin x.bin";
        assert!(anhangzeile_lesen(gut).is_ok());
        for (zeile, befund) in [
            (
                "AAAAAAAAAAAAAAAAAAAAAAAAAA 5 application/octet-stream - 140000000-1-x.bin x.bin",
                Anhangbefund::Id,
            ),
            ("zu kurz", Anhangbefund::Id),
            (
                "aaaaaaaaaaaaaaaaaaaaaaaaaa 0 application/octet-stream - 140000000-1-x.bin x",
                Anhangbefund::Bytes,
            ),
            (
                "aaaaaaaaaaaaaaaaaaaaaaaaaa +5 application/octet-stream - 140000000-1-x.bin x",
                Anhangbefund::Bytes,
            ),
            (
                "aaaaaaaaaaaaaaaaaaaaaaaaaa 5 image/svg+xml - 140000000-1-x.bin x",
                Anhangbefund::Art,
            ),
            (
                "aaaaaaaaaaaaaaaaaaaaaaaaaa 5 text/html - 140000000-1-x.bin x",
                Anhangbefund::Art,
            ),
            (
                "aaaaaaaaaaaaaaaaaaaaaaaaaa 5 image/png 0x5 140000000-1-x.png x",
                Anhangbefund::Masse,
            ),
            (
                "aaaaaaaaaaaaaaaaaaaaaaaaaa 5 image/png 5 140000000-1-x.png x",
                Anhangbefund::Masse,
            ),
            (
                "aaaaaaaaaaaaaaaaaaaaaaaaaa 5 application/octet-stream - ../x x",
                Anhangbefund::Datei,
            ),
            (
                "aaaaaaaaaaaaaaaaaaaaaaaaaa 5 application/octet-stream - 140000000-1-a/b x",
                Anhangbefund::Datei,
            ),
            (
                "aaaaaaaaaaaaaaaaaaaaaaaaaa 5 application/octet-stream - x.bin x",
                Anhangbefund::Datei,
            ),
            (
                "aaaaaaaaaaaaaaaaaaaaaaaaaa 5 application/octet-stream - 14000000x-1-x.bin x",
                Anhangbefund::Datei,
            ),
            (
                "aaaaaaaaaaaaaaaaaaaaaaaaaa 5 application/octet-stream - 140000000-1-x.bin ",
                Anhangbefund::Name,
            ),
        ] {
            assert_eq!(anhangzeile_lesen(zeile), Err(befund), "{zeile}");
        }
        let rohtext = "@ 2026-10-29T14:00:00.000+01:00 a@127.0.0.1\n+ anhang: zu kurz\n  Text\n\n";
        assert!(defekte(rohtext).is_empty());
        let gelesen = anhaenge_im_satz(rohtext);
        assert_eq!(gelesen, vec![(2, Err(Anhangbefund::Id))]);
    }

    const T1: &str = "2026-10-27T14:07:12.345+01:00";
    const T2: &str = "2026-10-29T15:02:11.004+01:00";

    fn t(text: &str) -> Zeit {
        DateTime::parse_from_rfc3339(text).expect("Zeit")
    }

    /// R00039 B0594 „Logformat“ (ADR-0048 erster Unit-Test): `@ t2 k / + bearbeitet: t1 / text` → Änderung
    /// Bearbeitet mit Ziel t1 und Text; `+ geloescht:` ohne Text → Löschsatz; zwei Bezugszeilen, unlesbare Bezugszeit,
    /// `t1 ≥ t2` und `+ anhang:` → defekt; eine Nachricht ohne Bezugszeile bleibt Nachricht (Mutante „Nachricht ohne
    /// Bezugszeile als Änderung erkannt“ rot).
    #[test]
    fn aenderungssaetze_werden_erkannt_und_defekte_abgewiesen() {
        let bearbeitet =
            format!("@ {T2} claude@10.0.0.5\n+ bearbeitet: {T1}\n  Der Scan ist durch.\n\n");
        let geloescht = format!("@ {T2} claude@10.0.0.5\n+ geloescht: {T1}\n\n");
        let anhang = "+ anhang: aaaaaaaaaaaaaaaaaaaaaaaaaa 5 application/octet-stream - 140000000-1-x.bin x.bin";
        let defekt = |satz: String| satzart(satz.as_bytes());

        assert_eq!(
            satzart(bearbeitet.as_bytes()),
            Satzart::Aenderung {
                ziel: t(T1),
                bezug: Bezug::Bearbeitet
            }
        );
        assert_eq!(text(&bearbeitet), "Der Scan ist durch.");
        assert_eq!(
            satzart(geloescht.as_bytes()),
            Satzart::Aenderung {
                ziel: t(T1),
                bezug: Bezug::Geloescht
            }
        );
        assert_eq!(
            defekt(format!(
                "@ {T2} k@b\n+ bearbeitet: {T1}\n+ geloescht: {T1}\n  x\n\n"
            )),
            Satzart::DefekteAenderung(Aenderungsdefekt::ZweiBezuege)
        );
        assert_eq!(
            defekt(format!(
                "@ {T2} k@b\n+ bearbeitet: {T1}\n+ bearbeitet: {T1}\n\n"
            )),
            Satzart::DefekteAenderung(Aenderungsdefekt::ZweiBezuege)
        );
        assert_eq!(
            defekt(format!("@ {T2} k@b\n+ bearbeitet: gestern\n  x\n\n")),
            Satzart::DefekteAenderung(Aenderungsdefekt::Bezugszeit)
        );
        for ziel in [T2, "2026-10-29T15:02:11.005+01:00"] {
            assert_eq!(
                defekt(format!("@ {T2} k@b\n+ bearbeitet: {ziel}\n  x\n\n")),
                Satzart::DefekteAenderung(Aenderungsdefekt::NichtVorher),
                "{ziel}"
            );
        }
        assert_eq!(
            defekt(format!("@ {T2} k@b\n+ bearbeitet: {T1}\n{anhang}\n  x\n\n")),
            Satzart::DefekteAenderung(Aenderungsdefekt::Anhang)
        );
        for nachricht in [
            format!("@ {T1} beta@127.0.0.1\n+ art: agent\n  Hallo\n\n"),
            format!("@ {T1} beta@127.0.0.1\n{anhang}\n\n"),
            format!("@ {T1} beta@127.0.0.1\n  + bearbeitet: {T1}\n\n"),
        ] {
            assert_eq!(
                satzart(nachricht.as_bytes()),
                Satzart::Nachricht,
                "{nachricht}"
            );
        }
    }

    /// B0594 schreiben: Kopf mit eigener Zeit und Kennung, die Bezugs-Metazeile mit der Zeit der Nachricht, der
    /// Text eingerückt mit `\r\n` als `\n`; liest sich als Änderung mit demselben Text zurück. Leerer Text (mit
    /// Anhang erlaubt) ist nur die Abschlusszeile.
    #[test]
    fn ein_bearbeitungssatz_wird_geschrieben_und_zurueckgelesen() {
        let von = Kennung::bilden("claude@10.0.0.5").expect("Kennung");

        let geschrieben = bearbeitungssatz(
            t(T2),
            &von,
            t(T1),
            "Der Scan ist durch.\r\n**Drei Befunde:**\n\n",
        );
        let leer = bearbeitungssatz(t(T2), &von, t(T1), "");

        assert_eq!(
            geschrieben,
            format!(
                "@ {T2} claude@10.0.0.5\n+ bearbeitet: {T1}\n  Der Scan ist durch.\n  **Drei Befunde:**\n\n"
            )
        );
        assert_eq!(text(&geschrieben), "Der Scan ist durch.\n**Drei Befunde:**");
        assert_eq!(
            leer,
            format!("@ {T2} claude@10.0.0.5\n+ bearbeitet: {T1}\n\n")
        );
        assert_eq!(text(&leer), "");
        for satz in [&geschrieben, &leer] {
            assert_eq!(saetze(satz.as_bytes()).len(), 1);
            assert_eq!(
                satzart(satz.as_bytes()),
                Satzart::Aenderung {
                    ziel: t(T1),
                    bezug: Bezug::Bearbeitet
                }
            );
        }
    }

    /// R00047 B0704, ADR-0053 P1: die Metazeile `+ antwort_auf: <t1 im Logformat> <kennung>` steht nach `+ art:`, vor
    /// `+ anhang:`; die Zeit in der Schreibweise des Logs (Millisekunden, Versatz), nie wie übergeben (Mutante Z8);
    /// der Rückweg liest den Bezug, der Satz bleibt eine Nachricht.
    #[test]
    fn eine_antwort_traegt_die_metazeile_antwort_auf_und_liest_sie_zurueck() {
        let mut antwort = nachricht(
            "2026-10-30T09:12:00.120+01:00",
            "codex@127.0.0.1",
            "Ja, Tests sind grün.",
        );
        antwort.art = Some(Art::Agent);
        antwort.bezug = Some(Antwortbezug {
            zeit: t("2026-10-30T09:10:03+01:00"),
            von: Kennung::bilden("claude@127.0.0.1").expect("Kennung"),
        });
        antwort.anhaenge = vec![anhang(
            "aaaaaaaaaaaaaaaaaaaaaaaaab",
            12,
            "text/plain; charset=utf-8",
            None,
            "091200120-1-notiz.txt",
            "notiz.txt",
        )];

        let geschrieben = satz(&antwort);

        assert_eq!(
            geschrieben,
            "@ 2026-10-30T09:12:00.120+01:00 codex@127.0.0.1\n+ art: agent\n\
             + antwort_auf: 2026-10-30T09:10:03.000+01:00 claude@127.0.0.1\n\
             + anhang: aaaaaaaaaaaaaaaaaaaaaaaaab 12 text/plain; charset=utf-8 - 091200120-1-notiz.txt notiz.txt\n\
             \x20 Ja, Tests sind grün.\n\n"
        );
        assert_eq!(satzart(geschrieben.as_bytes()), Satzart::Nachricht);
        assert_eq!(
            antwortbezug(geschrieben.as_bytes()),
            Ok(antwort.bezug.clone())
        );
        assert_eq!(satz_als_nachricht(geschrieben.as_bytes()), Some(antwort));
    }

    /// B0704 Defekte (ADR-0053 Regel 1): zwei Bezugszeilen, unlesbare Zeit, Zeit nicht echt vor der eigenen, fehlende
    /// oder ungültige Kennung → Defekt; die Nachricht bleibt, nur ohne Bezug. Ohne Metazeile kein Bezug, kein Defekt.
    #[test]
    fn eine_defekte_metazeile_antwort_auf_laesst_die_nachricht_ohne_bezug() {
        let kopf = format!("@ {T2} codex@127.0.0.1\n+ art: agent\n");
        for (metazeilen, defekt) in [
            (
                format!("+ antwort_auf: {T1} a@b\n+ antwort_auf: {T1} a@b\n"),
                Antwortdefekt::ZweiBezuege,
            ),
            (
                "+ antwort_auf: gestern x@y\n".to_string(),
                Antwortdefekt::Zeit,
            ),
            (
                format!("+ antwort_auf: {T2} x@b\n"),
                Antwortdefekt::NichtVorher,
            ),
            (
                "+ antwort_auf: 2026-10-29T15:02:11.005+01:00 x@b\n".to_string(),
                Antwortdefekt::NichtVorher,
            ),
            (format!("+ antwort_auf: {T1}\n"), Antwortdefekt::Kennung),
            (format!("+ antwort_auf: {T1} \n"), Antwortdefekt::Kennung),
            (
                format!("+ antwort_auf: {T1} ohne-at\n"),
                Antwortdefekt::Kennung,
            ),
            (
                format!("+ antwort_auf: {T1} a@b c\n"),
                Antwortdefekt::Kennung,
            ),
        ] {
            let roh = format!("{kopf}{metazeilen}  Text\n\n");

            assert_eq!(antwortbezug(roh.as_bytes()), Err(defekt), "{roh:?}");
            assert_eq!(satzart(roh.as_bytes()), Satzart::Nachricht, "{roh:?}");
            let gelesen = satz_als_nachricht(roh.as_bytes()).expect("Nachricht bleibt");
            assert_eq!((gelesen.bezug, gelesen.text.as_str()), (None, "Text"));
        }
        let ohne = format!("{kopf}  Text\n\n");
        assert_eq!(antwortbezug(ohne.as_bytes()), Ok(None));
        let gueltig = format!("{kopf}+ antwort_auf: {T1} claude@10.0.0.5\n  Text\n\n");
        assert_eq!(
            antwortbezug(gueltig.as_bytes()),
            Ok(Some(Antwortbezug {
                zeit: t(T1),
                von: Kennung::bilden("claude@10.0.0.5").expect("Kennung"),
            }))
        );
    }

    /// B0704 (M14 für den Bezug): ein Text, der eine Metazeile `+ antwort_auf:` vortäuscht, bleibt eingerückter Text.
    #[test]
    fn ein_vorgetaeuschter_antwortbezug_im_text_bleibt_text() {
        let gesendet = nachricht(
            T2,
            "a@127.0.0.1",
            &format!("x\n+ antwort_auf: {T1} claude@127.0.0.1"),
        );

        let geschrieben = satz(&gesendet);

        assert_eq!(antwortbezug(geschrieben.as_bytes()), Ok(None));
        let gelesen = satz_als_nachricht(geschrieben.as_bytes()).expect("Satz");
        assert_eq!(gelesen.bezug, None);
        assert_eq!(gelesen.text, gesendet.text);
    }

    /// M14: ein Text, der eine Bezugs-Metazeile oder eine Kopfzeile vortäuscht, bleibt eingerückter Text — ein Satz,
    /// eine Bezugszeile.
    #[test]
    fn ein_vorgetaeuschter_bezug_im_text_bleibt_text() {
        let von = Kennung::bilden("beta@127.0.0.1").expect("Kennung");
        let falsch =
            format!("x\n+ geloescht: {T1}\n@ 2026-10-30T15:00:00.000+01:00 gamma@127.0.0.1\n  y");

        let geschrieben = bearbeitungssatz(t(T2), &von, t(T1), &falsch);

        assert_eq!(saetze(geschrieben.as_bytes()).len(), 1);
        assert_eq!(
            satzart(geschrieben.as_bytes()),
            Satzart::Aenderung {
                ziel: t(T1),
                bezug: Bezug::Bearbeitet
            }
        );
        assert_eq!(text(&geschrieben), falsch);
    }

    #[test]
    fn zwei_saetze_und_eine_defekte_zeile_wie_im_ausgangsordner() {
        let saetze = gueltige(TAG_27);

        assert_eq!(saetze.len(), 2);
        assert_eq!(
            saetze[0].zeit,
            DateTime::parse_from_rfc3339("2026-10-27T10:00:00+01:00").expect("Zeit")
        );
        assert_eq!(saetze[0].von.als_text(), "shoff@127.0.0.1");
        assert_eq!(saetze[1].von.als_text(), "claude@10.0.0.5");
        assert_eq!(saetze[0].anfang, 0);
        assert_eq!(saetze[0].ende, saetze[1].anfang);
        assert!(TAG_27[saetze[1].anfang..].starts_with("@ 2026-10-27T10:00:05"));
        assert_eq!(saetze[1].ende, TAG_27.len());
        assert_eq!(defekte(TAG_27), vec![4]);
    }

    #[test]
    fn kopf_mit_unlesbarer_zeit_ist_ein_defekt_mit_seiner_zeile_samt_text() {
        let rohtext = "@ 2026-10-27T14:00:00.000+01:00 a@127.0.0.1\n  eins\n\n@ 2026-10-27T14:00:0@ 2026-10-27T14:00:00.002+01:00 c@127.0.0.1\n  zwei\n\n@ 2026-10-27T14:00:00.003+01:00 b@127.0.0.1\n  drei\n\n";

        let saetze = gueltige(rohtext);
        assert_eq!(saetze.len(), 2);
        assert_eq!(saetze[0].von.als_text(), "a@127.0.0.1");
        assert_eq!(saetze[1].von.als_text(), "b@127.0.0.1");
        assert_eq!(defekte(rohtext), vec![4]);
    }

    #[test]
    fn kopf_mit_ungueltiger_kennung_oder_falscher_feldzahl_ist_ein_defekt() {
        for kopf in [
            "@ 2026-10-27T14:00:00.000+01:00 ohne-at",
            "@ 2026-10-27T14:00:00.000+01:00",
            "@ 2026-10-27T14:00:00.000+01:00 a@b zu viel",
            "@ 2026-10-27T14:00:00.000+01:00  a@b",
        ] {
            let rohtext = format!("{kopf}\n  text\n\n");
            assert!(gueltige(&rohtext).is_empty(), "{kopf}");
            assert_eq!(defekte(&rohtext), vec![1], "{kopf}");
        }
    }

    #[test]
    fn metazeilen_leere_zeilen_und_text_mit_at_gehoeren_zum_satz() {
        let rohtext =
            "@ 2026-10-27T14:00:00.000+01:00 a@b\n+ antwort: x\n  @ kein Kopf\n\n  weiter\n\n";
        let saetze = gueltige(rohtext);
        assert_eq!(saetze.len(), 1);
        assert_eq!(saetze[0].ende, rohtext.len());
        assert!(defekte(rohtext).is_empty());
    }

    fn nachricht(zeit: &str, von: &str, text: &str) -> Nachricht {
        Nachricht {
            zeit: DateTime::parse_from_rfc3339(zeit).expect("Zeit"),
            von: Kennung::bilden(von).expect("Kennung"),
            art: None,
            text: text.to_string(),
            anhaenge: Vec::new(),
            geaendert: None,
            bezug: None,
        }
    }

    /// B0208: die Art steht als Metazeile direkt nach dem Kopf, vor dem Text.
    #[test]
    fn satz_mit_art_traegt_die_metazeile_nach_dem_kopf() {
        for (art, zeile) in [(Art::Agent, "+ art: agent"), (Art::Mensch, "+ art: mensch")] {
            let mut gesendet = nachricht(
                "2026-10-28T12:00:00.000+01:00",
                "beta@127.0.0.1",
                "Hallo\r\nWelt\n\n",
            );
            gesendet.art = Some(art);

            assert_eq!(
                satz(&gesendet),
                format!(
                    "@ 2026-10-28T12:00:00.000+01:00 beta@127.0.0.1\n{zeile}\n  Hallo\n  Welt\n\n"
                )
            );
        }
    }

    /// B0208 Rückweg: der Satz liefert Art und Text; ohne Metazeile oder mit unbekanntem Wert keine Art.
    #[test]
    fn die_art_wird_aus_der_metazeile_gelesen_ohne_sie_keine() {
        for (roh, art) in [
            (
                "@ 2026-10-28T12:00:00.000+01:00 a@b\n+ art: agent\n  Hallo\n\n",
                Some(Art::Agent),
            ),
            (
                "@ 2026-10-28T12:00:00.000+01:00 a@b\n+ art: mensch\n  Hallo\n\n",
                Some(Art::Mensch),
            ),
            ("@ 2026-10-28T12:00:00.000+01:00 a@b\n  Hallo\n\n", None),
            (
                "@ 2026-10-28T12:00:00.000+01:00 a@b\n+ art: roboter\n  Hallo\n\n",
                None,
            ),
            (
                "@ 2026-10-28T12:00:00.000+01:00 a@b\n  + art: agent\n\n",
                None,
            ),
        ] {
            let gelesen = satz_als_nachricht(roh.as_bytes()).expect("Nachricht");

            assert_eq!(gelesen.art, art, "{roh:?}");
        }
        let mit_art =
            satz_als_nachricht(b"@ 2026-10-28T12:00:00.000+01:00 a@b\n+ art: agent\n  Hallo\n\n")
                .expect("Nachricht");
        assert_eq!(mit_art.text, "Hallo");
    }

    #[test]
    fn satz_hat_kopf_eingerueckten_text_und_leerzeile_genau_wie_im_kriterium() {
        let satz = satz(&nachricht(
            "2026-10-27T14:07:12.345+01:00",
            "claude@10.0.0.5",
            "# Titel\n\n@ kein Kopf\n\tTab\r\nEnde\n\n",
        ));

        assert_eq!(
            satz,
            "@ 2026-10-27T14:07:12.345+01:00 claude@10.0.0.5\n  # Titel\n\n  @ kein Kopf\n  \tTab\n  Ende\n\n"
        );
    }

    #[test]
    fn zeit_steht_immer_mit_millisekunden_und_versatz() {
        let satz = satz(&nachricht("2026-10-27T14:07:12Z", "a@b", "x"));
        assert_eq!(satz, "@ 2026-10-27T14:07:12.000+00:00 a@b\n  x\n\n");
    }

    #[test]
    fn rundweg_satz_saetze_text_ergibt_denselben_text_und_einen_satz() {
        let original = "# Titel\n\n@ kein Kopf\n  eingerückt\n```\ncode\n```";
        let rohtext = satz(&nachricht("2026-10-27T14:07:12.345+01:00", "a@b", original));

        let gelesen = saetze(rohtext.as_bytes());

        assert_eq!(gelesen.len(), 1);
        let bereich = gelesen[0].clone().expect("gültiger Satz");
        assert_eq!(text(&rohtext[bereich.anfang..bereich.ende]), original);
    }

    #[test]
    fn rundweg_normalisiert_zeilenenden_und_abschliessende_leerzeilen() {
        let rohtext = satz(&nachricht(
            "2026-10-27T14:07:12.345+01:00",
            "a@b",
            "\neins\r\n\r\nzwei\n\n\n",
        ));
        assert_eq!(text(&rohtext), "\neins\n\nzwei");
    }

    #[test]
    fn text_ueberspringt_metazeilen_und_defekte_zeilen() {
        assert_eq!(
            text("@ 2026-10-27T14:00:00.000+01:00 a@b\n+ antwort: x\n  eins\nkaputt\n  zwei\n\n"),
            "eins\nzwei"
        );
    }

    #[test]
    fn leerer_rohtext_hat_weder_saetze_noch_defekte() {
        assert!(saetze(b"").is_empty());
        assert!(saetze(b"\n\n").is_empty());
    }

    #[test]
    fn text_vor_dem_ersten_kopf_ist_ein_defekt_an_seiner_ersten_zeile() {
        let rohtext = "\n  verwaist\n+ x: y\n@ 2026-10-27T14:00:00.000+01:00 a@b\n  da\n";
        assert_eq!(gueltige(rohtext).len(), 1);
        assert_eq!(defekte(rohtext), vec![2]);
    }

    #[test]
    fn letzte_zeile_ohne_zeilenende_gehoert_zum_satz() {
        let rohtext = "@ 2026-10-27T14:00:00.000+01:00 a@b\n  Rest";
        let saetze = gueltige(rohtext);
        assert_eq!(saetze.len(), 1);
        assert_eq!(saetze[0].ende, rohtext.len());
    }

    fn gueltige_roh(rohtext: &[u8]) -> Vec<SatzBereich> {
        saetze(rohtext).into_iter().filter_map(Result::ok).collect()
    }

    fn defekte_roh(rohtext: &[u8]) -> Vec<usize> {
        saetze(rohtext)
            .into_iter()
            .filter_map(Result::err)
            .map(|defekt| defekt.zeile)
            .collect()
    }

    #[test]
    fn mitten_im_zeichen_abgeschnittener_text_ist_ein_defekt_seiner_zeile_der_satz_bleibt() {
        let rohtext = b"@ 2026-10-27T10:00:00.000+01:00 a@b\n  eins\n\n@ 2026-10-27T10:00:06.000+01:00 c@d\n  Scan l\xC3";

        let saetze = gueltige_roh(rohtext);

        assert_eq!(saetze.len(), 2);
        assert_eq!(saetze[1].ende, rohtext.len(), "Spanne über die Rohbytes");
        assert_eq!(defekte_roh(rohtext), vec![5]);
    }

    #[test]
    fn ungueltiges_byte_mitten_im_text_trifft_nur_seine_zeile() {
        let rohtext = b"@ 2026-10-27T10:00:00.000+01:00 a@b\n  Wie we\xffit?\n  weiter\n\n@ 2026-10-27T10:00:05.000+01:00 c@d\n  Fertig.\n\n";

        let saetze = gueltige_roh(rohtext);

        assert_eq!(saetze.len(), 2);
        assert_eq!(saetze[0].ende, saetze[1].anfang);
        assert_eq!(defekte_roh(rohtext), vec![2]);
    }

    #[test]
    fn ungueltiges_byte_in_der_kopfzeile_ist_ein_defekt_des_ganzen_satzes() {
        let rohtext = b"@ 2026-10-27T10:00:00.000+01:00 a\xc3@b\n  eins\n  zw\xffei\n\n@ 2026-10-27T10:00:05.000+01:00 c@d\n  zwei\n\n";

        let saetze = gueltige_roh(rohtext);

        assert_eq!(saetze.len(), 1);
        assert_eq!(saetze[0].von.als_text(), "c@d");
        assert_eq!(defekte_roh(rohtext), vec![1], "ein Defekt je defektem Satz");
    }

    #[test]
    fn ungueltige_zeile_vor_dem_ersten_kopf_und_als_eigene_zeile_ist_ein_defekt() {
        let rohtext = b"  verw\xffaist\n@ 2026-10-27T10:00:00.000+01:00 a@b\n  eins\nkap\xffutt\n";

        assert_eq!(gueltige_roh(rohtext).len(), 1);
        assert_eq!(defekte_roh(rohtext), vec![1, 4]);
    }
    #[test]
    fn ein_kopierter_satz_wird_zur_nachricht_ohne_einrueckung() {
        let gelesen = satz_als_nachricht(
            b"@ 2026-10-28T14:01:00.000+01:00 alpha@127.0.0.2\n  zwei\n  drei\n\n",
        );

        assert_eq!(
            gelesen,
            Some(Nachricht {
                zeit: DateTime::parse_from_rfc3339("2026-10-28T14:01:00+01:00").expect("Zeit"),
                von: Kennung::bilden("alpha@127.0.0.2").expect("Kennung"),
                art: None,
                text: "zwei\ndrei".to_string(),
                anhaenge: Vec::new(),
                geaendert: None,
                bezug: None,
            })
        );
    }

    #[test]
    fn ein_defekter_satz_dahinter_gehoert_nicht_zum_text() {
        let gelesen = satz_als_nachricht(
            b"@ 2026-10-28T14:00:00.000+01:00 a@b\n  eins\n\n@ 2026-10-28T14:0 kaputt\n  fremd\n\nkaputt\n",
        )
        .expect("Nachricht");

        assert_eq!(gelesen.text, "eins");
    }

    #[test]
    fn ungueltiges_utf8_im_text_wird_ersetzt() {
        let gelesen = satz_als_nachricht(b"@ 2026-10-28T14:00:00.000+01:00 a@b\n  Gr\xf6\xdfe\n\n")
            .expect("Nachricht");

        assert_eq!(gelesen.text, "Gr\u{fffd}\u{fffd}e");
    }

    #[test]
    fn ohne_gueltigen_kopf_am_anfang_keine_nachricht() {
        assert_eq!(satz_als_nachricht(b"  eins\n\n"), None);
        assert_eq!(satz_als_nachricht(b"@ gestern a@b\n  eins\n\n"), None);
        assert_eq!(satz_als_nachricht(b""), None);
    }
}
