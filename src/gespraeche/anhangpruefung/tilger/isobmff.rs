//! Ortsangaben in ISOBMFF tilgen (B0503, A00018 Ergänzung F4; ADR-0047 Ergänzung „Karte #9 Security-Gate“ Regeln
//! 1–3; Ergänzung „Karte #9 Security-Review B2“ Regel 5): MP4, MOV, M4A, 3GP, HEIC, HEIF, AVIF über Boxköpfe per
//! Position, je Ebene eine Erlaubnisliste; Leseziele ≤ 1 MiB, Nullziele nie gelesen.

use super::Zaehler;
use super::minimal::{minimales_exif, traegt_ort};
use super::port::{Abbruch, Ergebnis, Port, STUECK};
use crate::gespraeche::anhangpruefung::exif::orientierung;
use std::collections::{HashMap, HashSet};

/// `moov` und `meta` sind höchstens so groß — nur als Längengrenze, nie als Puffer.
pub const HOECHSTENS_MOOV: u64 = 64 * 1_048_576;

/// Erlaubnisliste der obersten Ebene (ADR-0047 Ergänzung „Karte #9 Security-Review B2“ Regel 5a, ohne `meco`, `emsg`
/// und `prft` nach „B3“ Regel 5i; Platzhalter nach 5h); jede andere Box, auch jede `uuid`, wird unkenntlich.
const OBEN_ERLAUBT: &[&[u8; 4]] = &[
    b"ftyp", b"styp", b"moov", b"mdat", b"moof", b"mfra", b"sidx", b"ssix", b"pdin", b"meta",
];

/// Erlaubnisliste in `moof` (B3 Regel 5i): Wiedergabestruktur; in `traf` steigt der Tilger nicht ab.
const MOOF_ERLAUBT: &[&[u8; 4]] = &[b"mfhd", b"traf", b"pssh"];

/// Erlaubnisliste in `moov` (Regel 5b); `cmov` ist nicht prüfbar.
const MOOV_ERLAUBT: &[&[u8; 4]] = &[
    b"mvhd", b"trak", b"mvex", b"iods", b"pssh", b"udta", b"meta", b"free", b"skip", b"wide",
];

/// Erlaubnisliste im obersten `meta` mit Handler `pict` (Regel 5e); `iprp` wird betreten (`udes`).
const HEIF_META_ERLAUBT: &[&[u8; 4]] = &[
    b"hdlr", b"pitm", b"iloc", b"iinf", b"iref", b"iprp", b"idat", b"dinf", b"grpl", b"ipro",
];

/// Bild- und abgeleitete Items, die bleiben (Regel 5e); `Exif` wird minimal, jedes andere Item genullt.
const HEIF_ITEMS_ERLAUBT: &[&[u8; 4]] = &[
    b"hvc1", b"hvt1", b"av01", b"avc1", b"jpeg", b"j2k1", b"vvc1", b"unci", b"mski", b"grid",
    b"iovl", b"iden", b"tmap",
];

/// Erlaubnisliste in `udta` (Regel 5c): Kapitel, Spurauswahl, `meta`.
const UDTA_ERLAUBT: &[&[u8; 4]] = &[b"chpl", b"tsel", b"meta"];

/// Platzhalter (ADR-0047 Ergänzung „Karte #9 Security-Review B3“ Regel 5h): auf jeder Ebene, die der Tilger betritt,
/// bleiben Typ, Größe und Kopf, der Inhalt wird genullt.
const PLATZHALTER: &[&[u8; 4]] = &[b"free", b"skip", b"wide"];

fn metadaten<T>(grund: &'static str) -> Ergebnis<T> {
    Err(Abbruch::Metadaten(grund))
}

/// Ob die Datei mit einer `ftyp`-Box beginnt.
pub fn ist_isobmff(anfang: &[u8]) -> bool {
    anfang.get(4..8) == Some(b"ftyp")
}

/// Eine Box: Anfang, Typ, Anfang des Inhalts (hinter `size`, Typ und ggf. 64-Bit-Länge) und Ende.
#[derive(Debug, Clone, Copy)]
struct Kiste {
    pos: u64,
    typ: [u8; 4],
    inhalt: u64,
    ende: u64,
}

fn kiste(port: &Port<'_>, pos: u64, grenze: u64, oben: bool) -> Ergebnis<Kiste> {
    let kopf = port.lesen(pos, 8.min(grenze.saturating_sub(pos)))?;
    if kopf.len() < 8 {
        return metadaten("Längen widersprechen sich");
    }
    let groesse = u64::from(u32::from_be_bytes([kopf[0], kopf[1], kopf[2], kopf[3]]));
    let typ = [kopf[4], kopf[5], kopf[6], kopf[7]];
    let (inhalt, ende) = match groesse {
        1 => {
            let lang = port.lesen(pos + 8, 8)?;
            let groesse = u64::from_be_bytes(lang.try_into().unwrap_or([0; 8]));
            (pos + 16, pos.checked_add(groesse))
        }
        0 if oben => (pos + 8, Some(grenze)),
        _ => (pos + 8, pos.checked_add(groesse)),
    };
    match ende {
        Some(ende) if ende >= inhalt && ende <= grenze => Ok(Kiste {
            pos,
            typ,
            inhalt,
            ende,
        }),
        _ => metadaten("Längen widersprechen sich"),
    }
}

/// Die Kinder in `[anfang, ende)`; ein Rest unter 8 Bytes (QuickTime-Abschluss) beendet die Liste.
fn kinder(port: &Port<'_>, anfang: u64, ende: u64, zaehler: &mut Zaehler) -> Ergebnis<Vec<Kiste>> {
    let mut liste = Vec::new();
    let mut pos = anfang;
    while ende - pos >= 8 {
        let kind = kiste(port, pos, ende, false)?;
        zaehler.block()?;
        if kind.ende <= pos {
            return metadaten("Längen widersprechen sich");
        }
        pos = kind.ende;
        liste.push(kind);
    }
    Ok(liste)
}

/// Tilgt eine Datei mit `ftyp` am Anfang.
pub fn isobmff(port: &Port<'_>, zaehler: &mut Zaehler) -> Ergebnis<()> {
    let mut pos = 0;
    while port.laenge() - pos >= 8 {
        let kiste = kiste(port, pos, port.laenge(), true)?;
        zaehler.block()?;
        match &kiste.typ {
            b"moov" => moov(port, kiste, zaehler)?,
            b"moof" => moof(port, kiste, zaehler)?,
            typ if PLATZHALTER.contains(&typ) => platzhalter(port, kiste)?,
            b"meta" => match handler(port, kiste)? {
                Some(handler) if &handler != b"pict" => nutzer_meta(port, kiste, zaehler)?,
                _ => oberstes_meta(port, kiste, zaehler)?,
            },
            typ if OBEN_ERLAUBT.contains(&typ) => {}
            _ => unkenntlich(port, kiste, zaehler)?,
        }
        if kiste.ende <= pos {
            return metadaten("Längen widersprechen sich");
        }
        pos = kiste.ende;
    }
    Ok(())
}

fn zu_gross(kiste: Kiste) -> bool {
    kiste.ende - kiste.pos > HOECHSTENS_MOOV
}

/// Eine Box außerhalb der Erlaubnisliste → Typ `free`, Inhalt genullt (bei `uuid` samt den 16 Bytes des Typs), nie
/// gelesen; `©xyz` und `loci` sind ein Ort.
fn unkenntlich(port: &Port<'_>, kiste: Kiste, zaehler: &mut Zaehler) -> Ergebnis<()> {
    zaehler.ort |= matches!(&kiste.typ, [0xA9, b'x', b'y', b'z'] | b"loci");
    port.schreiben(kiste.pos + 4, b"free")?;
    port.nullen(kiste.inhalt, kiste.ende - kiste.inhalt)
}

/// Ein Platzhalter: Inhalt hinter dem Kopf genullt (Nullziel, nie gelesen); ohne Inhalt nichts zu tun.
fn platzhalter(port: &Port<'_>, kiste: Kiste) -> Ergebnis<()> {
    port.nullen(kiste.inhalt, kiste.ende - kiste.inhalt)
}

/// `moov`: Erlaubnisliste, `cmov` ist nicht prüfbar; Abstieg in `trak`, `udta`, `meta`.
fn moov(port: &Port<'_>, moov: Kiste, zaehler: &mut Zaehler) -> Ergebnis<()> {
    if zu_gross(moov) {
        return metadaten("moov über 64 MiB");
    }
    for kind in kinder(port, moov.inhalt, moov.ende, zaehler)? {
        match &kind.typ {
            b"cmov" => return metadaten("cmov nicht prüfbar"),
            b"mvhd" => zeiten_nullen(port, kind)?,
            b"trak" => trak(port, kind, zaehler)?,
            b"udta" => udta(port, kind, zaehler)?,
            b"meta" => nutzer_meta(port, kind, zaehler)?,
            typ if PLATZHALTER.contains(&typ) => platzhalter(port, kind)?,
            typ if MOOV_ERLAUBT.contains(&typ) => {}
            _ => unkenntlich(port, kind, zaehler)?,
        }
    }
    Ok(())
}

/// `moof` (B3 Regel 5i): nur Boxköpfe gelesen; Erlaubnisliste, Platzhalter genullt, jede andere Box unkenntlich.
fn moof(port: &Port<'_>, moof: Kiste, zaehler: &mut Zaehler) -> Ergebnis<()> {
    for kind in kinder(port, moof.inhalt, moof.ende, zaehler)? {
        match &kind.typ {
            typ if PLATZHALTER.contains(&typ) => platzhalter(port, kind)?,
            typ if MOOF_ERLAUBT.contains(&typ) => {}
            _ => unkenntlich(port, kind, zaehler)?,
        }
    }
    Ok(())
}

/// `trak`: Abstieg zu `udta`, `meta`, den Zeitfeldern in `tkhd` und — nur für `mdhd` — in `mdia`; alles andere ist
/// Wiedergabestruktur und bleibt.
fn trak(port: &Port<'_>, trak: Kiste, zaehler: &mut Zaehler) -> Ergebnis<()> {
    for kind in kinder(port, trak.inhalt, trak.ende, zaehler)? {
        match &kind.typ {
            b"udta" => udta(port, kind, zaehler)?,
            b"meta" => nutzer_meta(port, kind, zaehler)?,
            b"tkhd" => zeiten_nullen(port, kind)?,
            b"mdia" => {
                for enkel in kinder(port, kind.inhalt, kind.ende, zaehler)? {
                    match &enkel.typ {
                        b"mdhd" => zeiten_nullen(port, enkel)?,
                        typ if PLATZHALTER.contains(&typ) => platzhalter(port, enkel)?,
                        _ => {}
                    }
                }
            }
            typ if PLATZHALTER.contains(&typ) => platzhalter(port, kind)?,
            _ => {}
        }
    }
    Ok(())
}

/// Zeitfelder der Struktur (ADR-0047 B2 Regel 5g): `creation_time` und `modification_time` hinter Version und Flaggen
/// genullt — Version 0 je 4, Version 1 je 8 Bytes; gelesen wird nur das Versionsbyte. Unbekannte Version oder zu kurz
/// → `metadaten`.
fn zeiten_nullen(port: &Port<'_>, kiste: Kiste) -> Ergebnis<()> {
    let version = port.lesen(kiste.inhalt, 1.min(kiste.ende - kiste.inhalt))?;
    let breite = match version.first() {
        Some(0) => 4,
        Some(1) => 8,
        _ => return metadaten("Zeitfelder unlesbar"),
    };
    if kiste.ende - kiste.inhalt < 4 + 2 * breite {
        return metadaten("Zeitfelder unlesbar");
    }
    port.nullen(kiste.inhalt + 4, 2 * breite)
}

/// `udta` (unter `moov` und `trak`): Erlaubnisliste, Abstieg in `meta`; ein Rest unter 8 Bytes ist kein Block.
fn udta(port: &Port<'_>, udta: Kiste, zaehler: &mut Zaehler) -> Ergebnis<()> {
    for kind in kinder(port, udta.inhalt, udta.ende, zaehler)? {
        match &kind.typ {
            b"meta" => nutzer_meta(port, kind, zaehler)?,
            typ if PLATZHALTER.contains(&typ) => platzhalter(port, kind)?,
            typ if UDTA_ERLAUBT.contains(&typ) => {}
            _ => unkenntlich(port, kind, zaehler)?,
        }
    }
    Ok(())
}

/// Anfang der Kinder eines `meta`: ISO `meta` ist eine FullBox (4 Bytes Version und Flaggen), QuickTime nicht —
/// erkannt daran, dass das erste Kind `hdlr` gleich am Anfang steht.
fn meta_kinder_ab(port: &Port<'_>, meta: Kiste) -> Ergebnis<u64> {
    if meta.ende - meta.inhalt >= 8 && port.lesen(meta.inhalt + 4, 4)? == b"hdlr" {
        Ok(meta.inhalt)
    } else {
        Ok(meta.inhalt + 4)
    }
}

/// Erlaubnisliste eines `meta` mit Nutzerdaten (Regel 5d); `keys` bleibt und wird nicht gelesen.
const NUTZER_META_ERLAUBT: &[&[u8; 4]] = &[b"hdlr", b"keys", b"ilst"];

/// Ein `meta` unter `moov`, `trak`, `udta` oder ein oberstes `meta` mit Handler ungleich `pict` (ADR-0047 B2 Regel
/// 5d): Erlaubnisliste; in `ilst` je Eintrag die Werte (`data`, `mean`, `name`) hinter ihrem Kopf genullt — alle
/// Werte, nicht nur Orte —, jede andere Box im Eintrag `free`.
fn nutzer_meta(port: &Port<'_>, meta: Kiste, zaehler: &mut Zaehler) -> Ergebnis<()> {
    if zu_gross(meta) {
        return metadaten("meta über 64 MiB");
    }
    let anfang = meta_kinder_ab(port, meta)?.min(meta.ende);
    for kind in kinder(port, anfang, meta.ende, zaehler)? {
        match &kind.typ {
            b"ilst" => {
                for eintrag in kinder(port, kind.inhalt, kind.ende, zaehler)? {
                    if PLATZHALTER.contains(&&eintrag.typ) {
                        platzhalter(port, eintrag)?;
                    } else {
                        werte_nullen(port, eintrag, zaehler)?;
                    }
                }
            }
            typ if PLATZHALTER.contains(&typ) => platzhalter(port, kind)?,
            typ if NUTZER_META_ERLAUBT.contains(&typ) => {}
            _ => unkenntlich(port, kind, zaehler)?,
        }
    }
    Ok(())
}

/// Ein `ilst`-Eintrag: `data` behält Kopf, Typ und Sprache (8 Bytes), `mean` und `name` Version und Flaggen
/// (4 Bytes); der Inhalt dahinter wird genullt, nie gelesen.
fn werte_nullen(port: &Port<'_>, eintrag: Kiste, zaehler: &mut Zaehler) -> Ergebnis<()> {
    zaehler.ort |= eintrag.typ == [0xA9, b'x', b'y', b'z'];
    for wert in kinder(port, eintrag.inhalt, eintrag.ende, zaehler)? {
        let kopf = match &wert.typ {
            b"data" => 8,
            b"mean" | b"name" => 4,
            _ => {
                unkenntlich(port, wert, zaehler)?;
                continue;
            }
        };
        let ab = (wert.inhalt + kopf).min(wert.ende);
        port.nullen(ab, wert.ende - ab)?;
    }
    Ok(())
}

/// Der Handler eines `meta` (`hdlr` an erster Stelle, `handler_type` hinter Version, Flaggen und `pre_defined`);
/// ohne `hdlr` vorn gilt das oberste `meta` als HEIF (Regel 5e, eigene Erlaubnisliste).
fn handler(port: &Port<'_>, meta: Kiste) -> Ergebnis<Option<[u8; 4]>> {
    let anfang = meta_kinder_ab(port, meta)?;
    if meta.ende.saturating_sub(anfang) < 20 || port.lesen(anfang + 4, 4)? != b"hdlr" {
        return Ok(None);
    }
    let typ = port.lesen(anfang + 16, 4)?;
    Ok(Some([typ[0], typ[1], typ[2], typ[3]]))
}

fn lies_u32(bytes: &[u8], stelle: usize) -> Ergebnis<u32> {
    bytes
        .get(stelle..stelle + 4)
        .and_then(|vier| vier.try_into().ok())
        .map(u32::from_be_bytes)
        .ok_or(Abbruch::Metadaten("Längen widersprechen sich"))
}

fn lies_zahl(bytes: &[u8], stelle: usize, breite: usize) -> Ergebnis<u64> {
    let teil = bytes
        .get(stelle..stelle + breite)
        .ok_or(Abbruch::Metadaten("Längen widersprechen sich"))?;
    Ok(teil
        .iter()
        .fold(0u64, |zahl, byte| (zahl << 8) | u64::from(*byte)))
}

/// Das oberste `meta` (HEIF): Items `Exif` → minimales EXIF hinter dem `exif_tiff_header_offset`, XMP-Items genullt;
/// nur über `iloc`-Ausdehnungen mit `construction_method` 0 innerhalb der Datei. Vor dem ersten Schreiben (ADR-0047
/// B1 Regel 1): jede `item_ID` höchstens einmal in `iinf` und in `iloc`, kein Ziel überschneidet ein anderes Ziel
/// oder die Ausdehnung eines anderen Items.
fn oberstes_meta(port: &Port<'_>, meta: Kiste, zaehler: &mut Zaehler) -> Ergebnis<()> {
    if zu_gross(meta) {
        return metadaten("meta über 64 MiB");
    }
    let anfang = (meta.inhalt + 4).min(meta.ende);
    let kinder = kinder(port, anfang, meta.ende, zaehler)?;
    for kind in &kinder {
        match &kind.typ {
            b"iprp" => eigenschaften(port, *kind, zaehler)?,
            typ if PLATZHALTER.contains(&typ) => platzhalter(port, *kind)?,
            typ if HEIF_META_ERLAUBT.contains(&typ) => {}
            _ => unkenntlich(port, *kind, zaehler)?,
        }
    }
    let Some(iinf) = kinder.iter().find(|kind| &kind.typ == b"iinf") else {
        return Ok(());
    };
    let ziele = items(port, *iinf, zaehler)?;
    let Some(iloc) = kinder.iter().find(|kind| &kind.typ == b"iloc") else {
        if ziele.is_empty() {
            return Ok(());
        }
        return metadaten("Exif-Item nicht als Bytebereich der Datei");
    };
    let orte = ausdehnungen(port, *iloc)?;
    ueberschneidungsfrei(&orte.bereiche, &ziele)?;
    let mut schritte = Vec::with_capacity(ziele.len());
    for (id, item) in ziele {
        let Some(&(pos, laenge)) = orte.einzeln.get(&id) else {
            return metadaten("Exif-Item nicht als Bytebereich der Datei");
        };
        if !port.enthaelt(pos, laenge) {
            return metadaten("Exif-Item nicht als Bytebereich der Datei");
        }
        schritte.push((pos, laenge, item));
    }
    for (pos, laenge, item) in schritte {
        match item {
            Item::Exif => exif_item(port, pos, laenge, zaehler)?,
            Item::Nullen => port.nullen(pos, laenge)?,
        }
    }
    Ok(())
}

/// Kein Ziel überschneidet ein anderes Ziel oder die Ausdehnung eines anderen Items: über die nach Anfang sortierten
/// Bereiche, je mit dem weitesten Ende aller Bereiche und aller Ziele davor.
fn ueberschneidungsfrei(bereiche: &[(u64, u64, u32)], ziele: &HashMap<u32, Item>) -> Ergebnis<()> {
    let mut sortiert: Vec<(u64, u64, bool)> = bereiche
        .iter()
        .map(|&(anfang, ende, id)| (anfang, ende, ziele.contains_key(&id)))
        .collect();
    sortiert.sort_unstable();
    let (mut ende_alle, mut ende_ziele) = (0u64, 0u64);
    for (anfang, ende, ziel) in sortiert {
        if anfang < ende_ziele || (ziel && anfang < ende_alle) {
            return metadaten("Verweise überschneiden sich");
        }
        ende_alle = ende_alle.max(ende);
        if ziel {
            ende_ziele = ende_ziele.max(ende);
        }
    }
    Ok(())
}

/// `iprp/ipco`: Eigenschaften beschreiben das Bild und bleiben — außer `udes` (Nutzerbeschreibung) → unkenntlich.
fn eigenschaften(port: &Port<'_>, iprp: Kiste, zaehler: &mut Zaehler) -> Ergebnis<()> {
    for ipco in kinder(port, iprp.inhalt, iprp.ende, zaehler)? {
        if &ipco.typ != b"ipco" {
            continue;
        }
        for eigenschaft in kinder(port, ipco.inhalt, ipco.ende, zaehler)? {
            match &eigenschaft.typ {
                b"udes" => unkenntlich(port, eigenschaft, zaehler)?,
                typ if PLATZHALTER.contains(&typ) => platzhalter(port, eigenschaft)?,
                _ => {}
            }
        }
    }
    Ok(())
}

/// Was der Tilger mit einem Ziel-Item tut: minimales EXIF oder ganz nullen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Item {
    Exif,
    Nullen,
}

/// `iinf`: die Ziele — `Exif` und jedes Item, das kein Bild- oder abgeleitetes Item der Erlaubnisliste ist (Regel
/// 5e; `infe` vor Version 2 nennt keinen Typ und ist damit Ziel); jede `item_ID` höchstens einmal (alle `infe`, jede
/// Version), sonst „Verweise doppelt“.
fn items(port: &Port<'_>, iinf: Kiste, zaehler: &mut Zaehler) -> Ergebnis<HashMap<u32, Item>> {
    let kopf = port.lesen(iinf.inhalt, 8.min(iinf.ende - iinf.inhalt))?;
    let version = *kopf
        .first()
        .ok_or(Abbruch::Metadaten("Längen widersprechen sich"))?;
    let anfang = iinf.inhalt + if version == 0 { 6 } else { 8 };
    let mut gesehen = HashSet::new();
    let mut gefunden = HashMap::new();
    for infe in kinder(port, anfang.min(iinf.ende), iinf.ende, zaehler)? {
        if &infe.typ != b"infe" {
            continue;
        }
        let bytes = port.lesen(infe.inhalt, infe.ende - infe.inhalt)?;
        let version = *bytes
            .first()
            .ok_or(Abbruch::Metadaten("Längen widersprechen sich"))?;
        let breite = if version == 3 { 4 } else { 2 };
        let id = lies_zahl(&bytes, 4, breite)? as u32;
        if !gesehen.insert(id) {
            return metadaten("Verweise doppelt");
        }
        let typ = if version < 2 {
            None
        } else {
            bytes.get(4 + breite + 2..4 + breite + 6)
        };
        match typ {
            Some(b"Exif") => {
                gefunden.insert(id, Item::Exif);
            }
            Some(typ) if HEIF_ITEMS_ERLAUBT.iter().any(|erlaubt| &erlaubt[..] == typ) => {}
            _ => {
                gefunden.insert(id, Item::Nullen);
            }
        }
    }
    Ok(gefunden)
}

/// Was `iloc` nennt: je Item mit genau einer Ausdehnung, `construction_method` 0 und `data_reference_index` 0 →
/// (Anfang, Länge); dazu jede nicht leere Ausdehnung mit `construction_method` 0 als (Anfang, Ende, ID).
struct Orte {
    einzeln: HashMap<u32, (u64, u64)>,
    bereiche: Vec<(u64, u64, u32)>,
}

/// `iloc` lesen; eine `item_ID` zweimal → „Verweise doppelt“.
fn ausdehnungen(port: &Port<'_>, iloc: Kiste) -> Ergebnis<Orte> {
    let bytes = port.lesen(iloc.inhalt, iloc.ende - iloc.inhalt)?;
    let version = *bytes
        .first()
        .ok_or(Abbruch::Metadaten("Längen widersprechen sich"))?;
    let groessen = lies_zahl(&bytes, 4, 2)?;
    let (versatz, laenge, basis) = (
        (groessen >> 12) as usize & 0xF,
        (groessen >> 8) as usize & 0xF,
        (groessen >> 4) as usize & 0xF,
    );
    let index = if version >= 1 {
        groessen as usize & 0xF
    } else {
        0
    };
    let id_breite = if version < 2 { 2 } else { 4 };
    let mut stelle = 6;
    let anzahl = lies_zahl(&bytes, stelle, id_breite)?;
    stelle += id_breite;
    let mut orte = Orte {
        einzeln: HashMap::new(),
        bereiche: Vec::new(),
    };
    let mut gesehen = HashSet::new();
    for _ in 0..anzahl {
        let id = lies_zahl(&bytes, stelle, id_breite)? as u32;
        stelle += id_breite;
        if !gesehen.insert(id) {
            return metadaten("Verweise doppelt");
        }
        let methode = if version >= 1 {
            stelle += 2;
            lies_zahl(&bytes, stelle - 2, 2)? & 0xF
        } else {
            0
        };
        let referenz = lies_zahl(&bytes, stelle, 2)?;
        stelle += 2;
        let basis_versatz = lies_zahl(&bytes, stelle, basis)?;
        stelle += basis;
        let ausdehnungen = lies_zahl(&bytes, stelle, 2)?;
        stelle += 2;
        // Ausdehnungen ohne Bytes für Versatz und Länge sind leer: sie werden übersprungen, nicht gezählt — die
        // Arbeit hängt an den Bytes, nicht an `extent_count` (Befund #51 B1).
        if versatz + laenge == 0 {
            stelle += (ausdehnungen as usize) * index;
            if stelle > bytes.len() {
                return metadaten("Längen widersprechen sich");
            }
            continue;
        }
        let mut erste = None;
        for nummer in 0..ausdehnungen {
            stelle += index;
            let ab = lies_zahl(&bytes, stelle, versatz)?;
            stelle += versatz;
            let lang = lies_zahl(&bytes, stelle, laenge)?;
            stelle += laenge;
            let anfang = basis_versatz.checked_add(ab);
            if nummer == 0 {
                erste = anfang.map(|anfang| (anfang, lang));
            }
            if let (0, Some(anfang)) = (methode, anfang)
                && lang > 0
            {
                let ende = anfang
                    .checked_add(lang)
                    .ok_or(Abbruch::Metadaten("Längen widersprechen sich"))?;
                orte.bereiche.push((anfang, ende, id));
            }
        }
        if let (Some(ort), 0, 0, 1) = (erste, methode, referenz, ausdehnungen) {
            orte.einzeln.insert(id, ort);
        }
    }
    Ok(orte)
}

/// Exif-Item: `exif_tiff_header_offset` (u32), dahinter das TIFF → minimales EXIF; Leseziel sind TIFF-Kopf und IFD0
/// in der ersten 1 MiB, der Rest wird nur genullt.
fn exif_item(port: &Port<'_>, pos: u64, laenge: u64, zaehler: &mut Zaehler) -> Ergebnis<()> {
    let versatz = u64::from(lies_u32(&port.lesen(pos, 4.min(laenge))?, 0)?);
    let tiff = (pos + 4)
        .checked_add(versatz)
        .filter(|tiff| *tiff <= pos + laenge)
        .ok_or(Abbruch::Metadaten(
            "Exif-Item nicht als Bytebereich der Datei",
        ))?;
    let tiff_laenge = pos + laenge - tiff;
    let gelesen = tiff_laenge.min(STUECK as u64);
    let alt = port.lesen(tiff, gelesen)?;
    zaehler.ort |= traegt_ort(&alt);
    let neu = minimales_exif(orientierung(&alt), gelesen as usize, b"");
    port.schreiben(tiff, &neu)?;
    port.nullen(tiff + gelesen, tiff_laenge - gelesen)
}

#[cfg(test)]
mod tests {
    use super::super::port::attrappe::Datei;
    use super::*;
    use crate::gespraeche::anhangpruefung::exif::Ordnung;
    use crate::gespraeche::anhangpruefung::exif::tests::tiff;

    const MIB: usize = 1_048_576;
    /// Typ der XMP-`uuid`-Box.
    const XMP_UUID: [u8; 16] = [
        0xBE, 0x7A, 0xCF, 0xCB, 0x97, 0xA9, 0x42, 0xE8, 0x9C, 0x71, 0x99, 0x94, 0x91, 0xE3, 0xAF,
        0xAC,
    ];

    fn kiste(typ: &[u8], inhalt: &[u8]) -> Vec<u8> {
        let mut bytes = ((inhalt.len() + 8) as u32).to_be_bytes().to_vec();
        bytes.extend_from_slice(typ);
        bytes.extend_from_slice(inhalt);
        bytes
    }

    /// `hdlr` mit Handler `pict` (HEIF), 25 Bytes Inhalt.
    fn hdlr_pict() -> Vec<u8> {
        kiste(b"hdlr", &[&[0u8; 8][..], b"pict", &[0; 13]].concat())
    }

    fn ftyp() -> Vec<u8> {
        kiste(b"ftyp", b"isom\0\0\x02\0isommp41")
    }

    fn tilgen(daten: Vec<u8>) -> (Ergebnis<Zaehler>, Datei) {
        let datei = Datei::mit(daten);
        let mut zaehler = Zaehler::default();
        let ergebnis = {
            let port = Port::neu(&datei).expect("Port");
            isobmff(&port, &mut zaehler).map(|()| zaehler)
        };
        (ergebnis, datei)
    }

    fn enthaelt(heu: &[u8], nadel: &[u8]) -> bool {
        heu.windows(nadel.len()).any(|fenster| fenster == nadel)
    }

    /// MP4: `moov` mit `trak` (samt `stco`), `udta/©xyz`, `udta/XMP_`, `uuid`-XMP oben; `mdat` mit Daten.
    fn mp4(trak_fuellung: usize, xyz: &[u8]) -> Vec<u8> {
        let stco = kiste(b"stco", b"\0\0\0\0\0\0\0\x01\0\0\0\x30");
        let trak = kiste(
            b"trak",
            &[stco, kiste(b"edts", &vec![7u8; trak_fuellung])].concat(),
        );
        let udta = kiste(
            b"udta",
            &[
                kiste(&[0xA9, b'x', b'y', b'z'], xyz),
                kiste(b"XMP_", b"<GPSLatitude>52</GPSLatitude>"),
            ]
            .concat(),
        );
        let moov = kiste(b"moov", &[trak, udta].concat());
        let uuid = kiste(
            b"uuid",
            &[&XMP_UUID[..], b"<exif:GPSLongitude>13</exif:GPSLongitude>"].concat(),
        );
        [ftyp(), uuid, moov, kiste(b"mdat", b"Mediendaten")].concat()
    }

    #[test]
    fn mp4_verliert_xyz_und_xmp_stco_und_mdat_bleiben() {
        let original = mp4(100, b"\0\x11\x15\xc7+52.5200+013.4050/");

        let (ergebnis, datei) = tilgen(original.clone());

        let zaehler = ergebnis.expect("getilgt");
        let getilgt = datei.inhalt();
        assert_eq!(getilgt.len(), original.len());
        assert!(
            !enthaelt(&getilgt, b"+52.5200")
                && !enthaelt(&getilgt, b"GPS")
                && !enthaelt(&getilgt, &XMP_UUID)
        );
        assert!(!enthaelt(&getilgt, b"XMP_"));
        assert!(
            enthaelt(&getilgt, b"stco\0\0\0\0\0\0\0\x01\0\0\0\x30")
                && enthaelt(&getilgt, b"Mediendaten")
        );
        // `uuid` oben, `udta/XMP_` und `udta/©xyz` (Erlaubnisliste, ADR-0047 B2 Regeln 5a, 5c).
        assert_eq!(getilgt.windows(4).filter(|f| f == b"free").count(), 3);
        assert!(zaehler.ort);
    }

    #[test]
    fn moov_knapp_unter_64_mib_mit_xyz_am_ende() {
        let original = mp4(64 * MIB - 116, b"+52.5200+013.4050/");
        let moov = original
            .windows(4)
            .position(|f| f == b"moov")
            .expect("moov")
            - 4;
        let groesse =
            u32::from_be_bytes(original[moov..moov + 4].try_into().expect("Größe")) as usize;
        assert_eq!(groesse, 64 * MIB - 1);

        let (ergebnis, datei) = tilgen(original.clone());

        ergebnis.expect("getilgt");
        let getilgt = datei.inhalt();
        assert!(
            datei.groesste_lesung() <= MIB,
            "{}",
            datei.groesste_lesung()
        );
        assert!(!enthaelt(&getilgt[getilgt.len() - 300..], b"+52.5200"));
        let unterschiede = original
            .iter()
            .zip(&getilgt)
            .filter(|(a, b)| a != b)
            .count();
        assert!(unterschiede < 200, "{unterschiede}");
    }

    #[test]
    fn ein_xyz_von_3_mib_wird_in_stuecken_genullt_und_nie_gelesen() {
        let original = mp4(10, &vec![b'5'; 3 * MIB]);

        let (ergebnis, datei) = tilgen(original);

        ergebnis.expect("getilgt");
        assert!(!enthaelt(&datei.inhalt(), &[b'5'; 64]));
        assert!(datei.groesste_lesung() <= 16, "{}", datei.groesste_lesung());
        assert!(datei.groesste_schreibung() <= MIB);
    }

    #[test]
    fn eine_box_mit_64_bit_laenge_wird_gelesen() {
        let mut gross = 1u32.to_be_bytes().to_vec();
        gross.extend_from_slice(b"wide");
        gross.extend_from_slice(&(16u64 + 4).to_be_bytes());
        gross.extend_from_slice(b"\0\0\0\0");
        let rest = mp4(10, b"+52.5200+013.4050/");
        let original = [ftyp(), gross, rest[ftyp().len()..].to_vec()].concat();

        let (ergebnis, datei) = tilgen(original);

        ergebnis.expect("getilgt");
        assert!(!enthaelt(&datei.inhalt(), b"+52.5200"));
    }

    /// ADR-0047 B2 Regel 5d, Vermerk „Überholt“: `keys` wird nicht mehr gelesen — auch eines von 1 MiB + 1 bleibt
    /// unberührt, statt die Datei abzuweisen.
    #[test]
    fn keys_von_1_mib_und_einem_byte_wird_nicht_gelesen() {
        let keys = kiste(b"keys", &vec![0u8; MIB + 1]);
        let meta = kiste(b"meta", &[kiste(b"hdlr", &[0; 25]), keys].concat());
        let original = [ftyp(), kiste(b"moov", &meta)].concat();

        let (ergebnis, datei) = tilgen(original.clone());

        ergebnis.expect("getilgt");
        assert!(datei.groesste_lesung() <= 16, "{}", datei.groesste_lesung());
        assert_eq!(datei.inhalt(), original);
    }

    #[test]
    fn quicktime_ortsschluessel_in_ilst_werden_genullt() {
        let schluessel = |name: &[u8]| {
            let mut bytes = ((name.len() + 8) as u32).to_be_bytes().to_vec();
            bytes.extend_from_slice(b"mdta");
            bytes.extend_from_slice(name);
            bytes
        };
        let keys = kiste(
            b"keys",
            &[
                &[0u8, 0, 0, 0, 0, 0, 0, 2][..],
                &schluessel(b"com.apple.quicktime.make"),
                &schluessel(b"com.apple.quicktime.location.ISO6709"),
            ]
            .concat(),
        );
        let wert = |text: &[u8]| kiste(b"data", &[&[0u8, 0, 0, 1, 0, 0, 0, 0][..], text].concat());
        let ilst = kiste(
            b"ilst",
            &[
                kiste(&1u32.to_be_bytes(), &wert(b"Apple")),
                kiste(&2u32.to_be_bytes(), &wert(b"+52.5200+013.4050/")),
            ]
            .concat(),
        );
        let meta = kiste(b"meta", &[kiste(b"hdlr", &[0; 25]), keys, ilst].concat());
        let original = [ftyp(), kiste(b"moov", &meta)].concat();

        let (ergebnis, datei) = tilgen(original);

        // Seit ADR-0047 B2 Regel 5d: alle Werte in `ilst`, nicht nur die Orte.
        ergebnis.expect("getilgt");
        let getilgt = datei.inhalt();
        assert!(!enthaelt(&getilgt, b"+52.5200") && !enthaelt(&getilgt, b"Apple"));
    }

    /// HEIC: oberstes `meta` mit `iinf` (Exif-Item 2, XMP-Item 3), `iloc` (Version `methode`), Daten in `mdat`.
    fn heic(methode: u16, versatz_aendern: u64) -> Vec<u8> {
        let tiff_bytes = {
            let mut tiff = tiff(
                Ordnung::Intel,
                &[(0x0112, 3, 1, 6), (0x8825, 4, 1, 40)],
                None,
            );
            tiff.extend_from_slice(b"GPS 52 13 Kamera");
            tiff
        };
        let exif = [&0u32.to_be_bytes()[..], &tiff_bytes].concat();
        let xmp = b"<x:xmpmeta>GPSLatitude</x:xmpmeta>".to_vec();
        let infe = |id: u16, typ: &[u8], rest: &[u8]| {
            kiste(
                b"infe",
                &[&[2u8, 0, 0, 0][..], &id.to_be_bytes(), &[0, 0], typ, rest].concat(),
            )
        };
        let iinf = kiste(
            b"iinf",
            &[
                &[0u8, 0, 0, 0, 0, 2][..],
                &infe(2, b"Exif", b"\0"),
                &infe(3, b"mime", b"\0application/rdf+xml\0"),
            ]
            .concat(),
        );
        let mdat_inhalt = [exif.clone(), xmp.clone()].concat();
        let iloc_laenge = 8 + 4 + 2 + 2 + 2 * (2 + 2 + 2 + 2 + 4 + 4);
        let meta_ohne_iloc = 8 + 4 + hdlr_pict().len() + iinf.len();
        let mdat_anfang = (ftyp().len() + meta_ohne_iloc + iloc_laenge + 8) as u64;
        let eintrag = |id: u16, ab: u64, lang: usize| {
            [
                &id.to_be_bytes()[..],
                &methode.to_be_bytes(),
                &[0, 0],
                &1u16.to_be_bytes(),
                &((ab + versatz_aendern) as u32).to_be_bytes(),
                &(lang as u32).to_be_bytes(),
            ]
            .concat()
        };
        let iloc = kiste(
            b"iloc",
            &[
                &[1u8, 0, 0, 0, 0x44, 0x00][..],
                &2u16.to_be_bytes(),
                &eintrag(2, mdat_anfang, exif.len()),
                &eintrag(3, mdat_anfang + exif.len() as u64, xmp.len()),
            ]
            .concat(),
        );
        assert_eq!(iloc.len(), iloc_laenge);
        let meta = kiste(
            b"meta",
            &[&[0u8, 0, 0, 0][..], &hdlr_pict(), &iinf, &iloc].concat(),
        );
        [ftyp(), meta, kiste(b"mdat", &mdat_inhalt)].concat()
    }

    #[test]
    fn heic_exif_item_wird_minimal_an_der_stelle_und_xmp_genullt() {
        let original = heic(0, 0);

        let (ergebnis, datei) = tilgen(original.clone());

        assert!(ergebnis.expect("getilgt").ort);
        let getilgt = datei.inhalt();
        assert_eq!(getilgt.len(), original.len());
        assert!(!enthaelt(&getilgt, b"GPS") && !enthaelt(&getilgt, b"Kamera"));
        let tiff = getilgt
            .windows(4)
            .position(|f| f == b"MM\0*")
            .expect("minimales EXIF");
        assert_eq!(orientierung(&getilgt[tiff..]), Some(6));
    }

    /// Eine HEIC aus Teilen: `ftyp heic`, oberstes `meta` mit `hdlr`, `iinf` (Version 0) aus `infe` und dem `iloc`,
    /// das `iloc` aus dem Anfang des `mdat` gebaut wird; `mdat` mit `daten`.
    fn heic_aus(infe: &[Vec<u8>], iloc: impl Fn(u64) -> Vec<u8>, daten: &[u8]) -> Vec<u8> {
        let iinf = kiste(
            b"iinf",
            &[&[0u8, 0, 0, 0, 0, 0][..], &infe.concat()].concat(),
        );
        let bauen = |mdat_anfang: u64| {
            let meta = kiste(
                b"meta",
                &[&[0u8, 0, 0, 0][..], &hdlr_pict(), &iinf, &iloc(mdat_anfang)].concat(),
            );
            [kiste(b"ftyp", b"heic\0\0\0\0mif1heic"), meta].concat()
        };
        let mdat_anfang = bauen(0).len() as u64 + 8;
        [bauen(mdat_anfang), kiste(b"mdat", daten)].concat()
    }

    fn exif_infe(id: u16) -> Vec<u8> {
        kiste(
            b"infe",
            &[&[2u8, 0, 0, 0][..], &id.to_be_bytes(), &[0, 0], b"Exif\0"].concat(),
        )
    }

    /// `iloc` mit Feldbreiten 0 (Größenbytes `00 00`): je Item `extent_count` 0xFFFF, keine Ausdehnung belegt ein Byte.
    fn iloc_ohne_breiten(version: u8, items: u32) -> Vec<u8> {
        let id_breite = if version < 2 { 2 } else { 4 };
        let mut inhalt = vec![version, 0, 0, 0, 0x00, 0x00];
        inhalt.extend_from_slice(&items.to_be_bytes()[4 - id_breite..]);
        for id in 1..=items {
            inhalt.extend_from_slice(&id.to_be_bytes()[4 - id_breite..]);
            inhalt.extend_from_slice(&[0, 0, 0, 0, 0xFF, 0xFF]);
        }
        kiste(b"iloc", &inhalt)
    }

    fn in_hoechstens_2_s<T>(
        was: &str,
        daten: Vec<u8>,
        pruefen: impl Fn(&Ergebnis<Zaehler>) -> T,
    ) -> T {
        let start = std::time::Instant::now();
        let (ergebnis, _) = tilgen(daten);
        let dauer = start.elapsed();
        assert!(dauer.as_secs_f64() <= 2.0, "{was}: {dauer:?}");
        pruefen(&ergebnis)
    }

    /// Befund #51 B1: Feldbreite 0 und `extent_count` 0xFFFF — die Arbeit hängt an den Bytes, nicht an den Zählern.
    #[test]
    fn iloc_mit_feldbreite_0_und_vielen_ausdehnungen_endet_in_2_s() {
        for (version, items) in [(2u8, 1_000u32), (2, 104_000), (1, 1_000), (1, 65_535)] {
            let daten = heic_aus(&[exif_infe(1)], |_| iloc_ohne_breiten(version, items), b"");
            assert!(daten.len() <= 1_100_000, "{}", daten.len());

            in_hoechstens_2_s(
                &format!("iloc v{version}, {items} Items"),
                daten,
                |ergebnis| {
                    assert!(
                        matches!(
                            ergebnis,
                            Err(Abbruch::Metadaten(
                                "Exif-Item nicht als Bytebereich der Datei"
                            ))
                        ),
                        "{ergebnis:?}"
                    )
                },
            );
        }
    }

    /// Befund #51 B1, gleiche Klasse: 50.000 Exif-Items gegen ein volles `iloc` (Items am Ende) — Suche je Item nicht
    /// linear. 150.000 Einträge passen in kein `iloc` ≤ 1 MiB (Eintrag ≥ 10 Bytes ab Version 2, darunter 16-Bit-Zahl);
    /// hier 69.000 Einträge zu 15 Bytes. Seit ADR-0047 B1 Regel 1 hat jedes Item seinen eigenen Bereich (je ein
    /// Exif im `mdat`, die fremden dahinter) — derselbe Bereich für alle wäre „Verweise überschneiden sich“.
    #[test]
    fn viele_exif_items_gegen_ein_volles_iloc_enden_in_2_s() {
        let exif = [
            &0u32.to_be_bytes()[..],
            &tiff(Ordnung::Intel, &[(0x0112, 3, 1, 6)], None),
        ]
        .concat();
        let items = 50_000u32;
        let fremde = 19_000u32;
        let infe: Vec<_> = (1..=items).map(|id| exif_infe(id as u16)).collect();
        let iloc = |mdat_anfang: u64| {
            let mut inhalt = vec![2u8, 0, 0, 0, 0x01, 0x40];
            inhalt.extend_from_slice(&(fremde + items).to_be_bytes());
            let lang = exif.len() as u64;
            let fremde_ab = mdat_anfang + u64::from(items) * lang;
            let ids = (1..=fremde)
                .map(|n| (1_000_000 + n, fremde_ab + u64::from(n) * lang))
                .chain((1..=items).map(|id| (id, mdat_anfang + u64::from(id - 1) * lang)));
            for (id, ab) in ids {
                inhalt.extend_from_slice(&id.to_be_bytes());
                inhalt.extend_from_slice(&[0, 0, 0, 0]);
                inhalt.extend_from_slice(&(ab as u32).to_be_bytes());
                inhalt.extend_from_slice(&1u16.to_be_bytes());
                inhalt.push(exif.len() as u8);
            }
            kiste(b"iloc", &inhalt)
        };
        let daten = heic_aus(&infe, iloc, &exif.repeat(items as usize));

        let zaehler = in_hoechstens_2_s("50.000 Items", daten, |ergebnis| {
            ergebnis
                .as_ref()
                .map(|zaehler| *zaehler)
                .map_err(|fehler| format!("{fehler:?}"))
        });

        assert!(zaehler.expect("getilgt").bloecke > 50_000);
    }

    /// Befund #51 B1, gleiche Klasse: 65.000 Ortsschlüssel in `keys` gegen 90.000 fremde `ilst`-Einträge.
    #[test]
    fn viele_ortsschluessel_gegen_viele_ilst_eintraege_enden_in_2_s() {
        let schluessel = 65_000u32;
        let mut keys = vec![0u8, 0, 0, 0];
        keys.extend_from_slice(&schluessel.to_be_bytes());
        for _ in 0..schluessel {
            keys.extend_from_slice(&16u32.to_be_bytes());
            keys.extend_from_slice(b"mdtalocation");
        }
        let ilst: Vec<u8> = (0..90_000)
            .flat_map(|_| kiste(&u32::MAX.to_be_bytes(), b""))
            .collect();
        let meta = kiste(
            b"meta",
            &[
                kiste(b"hdlr", &[0; 25]),
                kiste(b"keys", &keys),
                kiste(b"ilst", &ilst),
            ]
            .concat(),
        );
        let daten = [ftyp(), kiste(b"moov", &meta)].concat();

        in_hoechstens_2_s("65.000 Schlüssel", daten, |ergebnis| {
            assert!(ergebnis.is_ok(), "{ergebnis:?}")
        });
    }

    /// Befund #51 B2 (b), R00029 „Präparierte Metadaten“: HEIC mit zwei Exif-Items → beide minimal (Orientierung
    /// bleibt), kein Ort, keine Kameradaten; Länge gleich.
    #[test]
    fn heic_mit_zwei_exif_items_verliert_beide_orte() {
        let exif = |ort: &[u8]| {
            let mut tiff = tiff(
                Ordnung::Intel,
                &[(0x0112, 3, 1, 6), (0x8825, 4, 1, 40)],
                None,
            );
            tiff.extend_from_slice(ort);
            [&0u32.to_be_bytes()[..], &tiff].concat()
        };
        let (erstes, zweites) = (exif(b"GPS 52.5200 Kamera"), exif(b"GPS 48.1371 Kamera"));
        let iloc = |mdat_anfang: u64| {
            let mut inhalt = vec![1u8, 0, 0, 0, 0x44, 0x00];
            inhalt.extend_from_slice(&2u16.to_be_bytes());
            for (id, ab, lang) in [
                (1u16, mdat_anfang, erstes.len()),
                (2, mdat_anfang + erstes.len() as u64, zweites.len()),
            ] {
                inhalt.extend_from_slice(&id.to_be_bytes());
                inhalt.extend_from_slice(&[0, 0, 0, 0]);
                inhalt.extend_from_slice(&1u16.to_be_bytes());
                inhalt.extend_from_slice(&(ab as u32).to_be_bytes());
                inhalt.extend_from_slice(&(lang as u32).to_be_bytes());
            }
            kiste(b"iloc", &inhalt)
        };
        let original = heic_aus(
            &[exif_infe(1), exif_infe(2)],
            iloc,
            &[erstes.clone(), zweites.clone()].concat(),
        );

        let (ergebnis, datei) = tilgen(original.clone());

        assert!(ergebnis.expect("getilgt").ort);
        let getilgt = datei.inhalt();
        assert_eq!(getilgt.len(), original.len());
        assert!(!enthaelt(&getilgt, b"GPS") && !enthaelt(&getilgt, b"Kamera"));
        let minimale: Vec<usize> = getilgt
            .windows(4)
            .enumerate()
            .filter(|(_, f)| *f == b"MM\0*")
            .map(|(stelle, _)| stelle)
            .collect();
        assert_eq!(minimale.len(), 2);
        for stelle in minimale {
            assert_eq!(orientierung(&getilgt[stelle..]), Some(6));
        }
    }

    /// Befund #51 B2 (b), R00029 „Präparierte Metadaten“: MOV mit `©xyz` in `udta` **und** einem Ortsschlüssel in
    /// `meta/keys` → beide genullt, Hersteller und `mdat` bleiben.
    #[test]
    fn mov_mit_xyz_in_udta_und_ort_in_meta_keys_verliert_beide() {
        let schluessel = |name: &[u8]| {
            let mut bytes = ((name.len() + 8) as u32).to_be_bytes().to_vec();
            bytes.extend_from_slice(b"mdta");
            bytes.extend_from_slice(name);
            bytes
        };
        let keys = kiste(
            b"keys",
            &[
                &[0u8, 0, 0, 0, 0, 0, 0, 2][..],
                &schluessel(b"com.apple.quicktime.make"),
                &schluessel(b"com.apple.quicktime.location.ISO6709"),
            ]
            .concat(),
        );
        let wert = |text: &[u8]| kiste(b"data", &[&[0u8, 0, 0, 1, 0, 0, 0, 0][..], text].concat());
        let ilst = kiste(
            b"ilst",
            &[
                kiste(&1u32.to_be_bytes(), &wert(b"Apple")),
                kiste(&2u32.to_be_bytes(), &wert(b"+48.1371+011.5754/")),
            ]
            .concat(),
        );
        let meta = kiste(b"meta", &[kiste(b"hdlr", &[0; 25]), keys, ilst].concat());
        let udta = kiste(
            b"udta",
            &kiste(
                &[0xA9, b'x', b'y', b'z'],
                b"\0\x11\x15\xc7+52.5200+013.4050/",
            ),
        );
        let original = [
            kiste(b"ftyp", b"qt  \0\0\x02\0qt  "),
            kiste(b"moov", &[udta, meta].concat()),
            kiste(b"mdat", b"Mediendaten"),
        ]
        .concat();

        let (ergebnis, datei) = tilgen(original.clone());

        assert!(ergebnis.expect("getilgt").ort);
        let getilgt = datei.inhalt();
        assert_eq!(getilgt.len(), original.len());
        assert!(!enthaelt(&getilgt, b"+52.5200") && !enthaelt(&getilgt, b"+48.1371"));
        // Seit ADR-0047 B2 Regel 5d gehen alle Werte in `ilst`, auch der Hersteller.
        assert!(!enthaelt(&getilgt, b"Apple") && enthaelt(&getilgt, b"Mediendaten"));
    }

    #[test]
    fn heic_mit_construction_method_1_oder_versatz_ausserhalb_ist_nicht_tilgbar() {
        for (methode, versatz) in [(1u16, 0u64), (0, 1 << 30)] {
            let (ergebnis, _) = tilgen(heic(methode, versatz));
            assert!(
                matches!(ergebnis, Err(Abbruch::Metadaten(_))),
                "{methode} {versatz}"
            );
        }
    }

    /// `iloc` Version 1 mit Feldbreiten 4/4, ohne Basis: je Eintrag (ID, Anfang, Länge), `construction_method` 0.
    fn iloc_aus(eintraege: &[(u16, u64, u64)]) -> Vec<u8> {
        let mut inhalt = vec![1u8, 0, 0, 0, 0x44, 0x00];
        inhalt.extend_from_slice(&(eintraege.len() as u16).to_be_bytes());
        for (id, ab, lang) in eintraege {
            inhalt.extend_from_slice(&id.to_be_bytes());
            inhalt.extend_from_slice(&[0, 0, 0, 0]);
            inhalt.extend_from_slice(&1u16.to_be_bytes());
            inhalt.extend_from_slice(&(*ab as u32).to_be_bytes());
            inhalt.extend_from_slice(&(*lang as u32).to_be_bytes());
        }
        kiste(b"iloc", &inhalt)
    }

    fn item_infe(id: u16, typ: &[u8; 4]) -> Vec<u8> {
        kiste(
            b"infe",
            &[&[2u8, 0, 0, 0][..], &id.to_be_bytes(), &[0, 0], typ, b"\0"].concat(),
        )
    }

    /// Exif-Item mit Ort (`exif_tiff_header_offset` 0, TIFF mit GPS-Text).
    fn exif_mit_ort() -> Vec<u8> {
        let mut tiff = tiff(
            Ordnung::Intel,
            &[(0x0112, 3, 1, 6), (0x8825, 4, 1, 40)],
            None,
        );
        tiff.extend_from_slice(b"GPS 52.5200 13.4050");
        [&0u32.to_be_bytes()[..], &tiff].concat()
    }

    /// Ergebnis und ob die Datei unverändert blieb; je Fall ≤ 2 s.
    fn verweise(daten: Vec<u8>) -> (Option<&'static str>, bool) {
        let start = std::time::Instant::now();
        let (ergebnis, datei) = tilgen(daten.clone());
        assert!(
            start.elapsed().as_secs_f64() <= 2.0,
            "{:?}",
            start.elapsed()
        );
        let grund = match ergebnis {
            Err(Abbruch::Metadaten(grund)) => Some(grund),
            _ => None,
        };
        (grund, datei.inhalt() == daten)
    }

    /// ADR-0047 B1 P1 (B0638): ein Exif-Item, dessen `infe` 2.000-mal mit derselben `item_ID` steht → „Verweise
    /// doppelt“, nichts geschrieben; dasselbe für eine `item_ID` zweimal in `iloc`.
    #[test]
    fn doppelte_item_ids_sind_nicht_tilgbar() {
        let exif = exif_mit_ort();
        let lang = exif.len() as u64;
        let infe = vec![item_infe(1, b"Exif"); 2_000];
        let daten = heic_aus(&infe, |ab| iloc_aus(&[(1, ab, lang)]), &exif);
        assert_eq!(verweise(daten), (Some("Verweise doppelt"), true));

        let daten = heic_aus(
            &[item_infe(1, b"Exif")],
            |ab| iloc_aus(&[(1, ab, lang), (1, ab, lang)]),
            &exif,
        );
        assert_eq!(verweise(daten), (Some("Verweise doppelt"), true));
    }

    /// ADR-0047 B1 P2 (B0638): 2.000 verschiedene Exif-Items auf denselben Bereich; zwei, die sich um ein Byte
    /// überschneiden; ein Exif-Item, das eine Bildkachel um ein Byte überdeckt → „Verweise überschneiden sich“,
    /// nichts geschrieben (die Kachel byte-gleich).
    #[test]
    fn ueberschneidende_ziele_sind_nicht_tilgbar() {
        let exif = exif_mit_ort();
        let lang = exif.len() as u64;
        let infe: Vec<_> = (1..=2_000).map(|id| item_infe(id, b"Exif")).collect();
        let daten = heic_aus(
            &infe,
            |ab| iloc_aus(&(1..=2_000).map(|id| (id, ab, lang)).collect::<Vec<_>>()),
            &exif,
        );
        assert_eq!(verweise(daten), (Some("Verweise überschneiden sich"), true));

        let zwei = [exif.clone(), exif.clone()].concat();
        let daten = heic_aus(
            &[item_infe(1, b"Exif"), item_infe(2, b"Exif")],
            |ab| iloc_aus(&[(1, ab, lang), (2, ab + lang - 1, lang)]),
            &zwei,
        );
        assert_eq!(verweise(daten), (Some("Verweise überschneiden sich"), true));

        let kachel = vec![0x5A; 50];
        let mit_kachel = [kachel.clone(), exif.clone()].concat();
        let daten = heic_aus(
            &[item_infe(1, b"hvc1"), item_infe(2, b"Exif")],
            |ab| iloc_aus(&[(1, ab, 50), (2, ab + 49, lang)]),
            &mit_kachel,
        );
        assert_eq!(verweise(daten), (Some("Verweise überschneiden sich"), true));
    }

    /// Kontrolle: Exif-Item direkt hinter der Kachel (keine Überschneidung) → getilgt, Kachel byte-gleich.
    #[test]
    fn ein_exif_item_direkt_hinter_der_kachel_wird_getilgt() {
        let exif = exif_mit_ort();
        let lang = exif.len() as u64;
        let kachel = vec![0x5A; 50];
        let daten = heic_aus(
            &[item_infe(1, b"hvc1"), item_infe(2, b"Exif")],
            |ab| iloc_aus(&[(1, ab, 50), (2, ab + 50, lang)]),
            &[kachel.clone(), exif].concat(),
        );

        let (ergebnis, datei) = tilgen(daten);

        assert!(ergebnis.expect("getilgt").ort);
        let getilgt = datei.inhalt();
        assert!(enthaelt(&getilgt, &kachel) && !enthaelt(&getilgt, b"GPS"));
    }

    /// Typ der C2PA-`uuid`-Box.
    const C2PA_UUID: [u8; 16] = [
        0xD8, 0xFE, 0xC3, 0xD6, 0x1B, 0x0E, 0x48, 0x3C, 0x92, 0x97, 0x58, 0x28, 0x87, 0x7E, 0xC4,
        0x81,
    ];

    const ORT: &[u8] = b"GPSLatitude 52.52 +52.5200+013.4050/";

    /// Eine Box ist `free` mit Nullen (bei `uuid` samt den 16 Bytes des Typs), gleich lang wie `alt`.
    fn frei_an(getilgt: &[u8], alt: &[u8]) -> bool {
        getilgt.windows(alt.len()).any(|kiste| {
            kiste[..4] == alt[..4] && &kiste[4..8] == b"free" && kiste[8..].iter().all(|b| *b == 0)
        })
    }

    /// ADR-0047 B2 P4, P6 und Regeln 5a–5c (B0644): oben `uuid` C2PA und eine unbekannte Box, in `moov` eine
    /// unbekannte Box, in `moov/trak/udta` `©xyz`, in `moov/udta` `©day` → je `free` mit Nullen; `chpl`, `stco`,
    /// `mdat` byte-gleich; ein Rest von 4 Nullbytes am Ende von `udta` ist kein Block.
    #[test]
    fn isobmff_behaelt_nur_erlaubte_boxen_oben_in_moov_trak_und_udta() {
        let uuid = kiste(b"uuid", &[&C2PA_UUID[..], ORT].concat());
        let oben = kiste(b"abcd", ORT);
        let im_moov = kiste(b"xyzw", ORT);
        let xyz = kiste(&[0xA9, b'x', b'y', b'z'], ORT);
        let tag = kiste(&[0xA9, b'd', b'a', b'y'], b"2026-10-29T12:34:56Z");
        let chpl = kiste(b"chpl", b"\x01\0\0\0\0\x01Kapitel");
        let stco = kiste(b"stco", b"\0\0\0\0\0\0\0\x01\0\0\0\x30");
        let trak = kiste(
            b"trak",
            &[
                stco.clone(),
                kiste(b"udta", &[xyz.clone(), vec![0; 4]].concat()),
            ]
            .concat(),
        );
        let udta = kiste(b"udta", &[chpl.clone(), tag.clone()].concat());
        let mdat = kiste(b"mdat", b"Mediendaten");
        let moov = kiste(
            b"moov",
            &[kiste(b"mvhd", &[0; 100]), trak, im_moov.clone(), udta].concat(),
        );
        let original = [ftyp(), uuid.clone(), oben.clone(), moov, mdat.clone()].concat();

        let (ergebnis, datei) = tilgen(original.clone());

        ergebnis.expect("getilgt");
        let getilgt = datei.inhalt();
        assert_eq!(getilgt.len(), original.len());
        assert!(!enthaelt(&getilgt, b"52.52") && !enthaelt(&getilgt, b"GPS"));
        assert!(!enthaelt(&getilgt, &C2PA_UUID) && !enthaelt(&getilgt, b"2026-10-29"));
        for (was, alt) in [
            ("uuid", &uuid),
            ("oben", &oben),
            ("moov", &im_moov),
            ("xyz", &xyz),
            ("day", &tag),
        ] {
            assert!(frei_an(&getilgt, alt), "{was}");
        }
        for bleibt in [&chpl, &stco, &mdat] {
            assert!(
                enthaelt(&getilgt, bleibt),
                "{}",
                String::from_utf8_lossy(&bleibt[4..8])
            );
        }
    }

    /// ADR-0047 B2 P9 (B0644): `cmov` in `moov` → `metadaten`; eine Box in `udta` mit `size` über das Ende von
    /// `udta` → `metadaten`.
    #[test]
    fn cmov_und_eine_box_ueber_das_ende_von_udta_sind_nicht_tilgbar() {
        let cmov = [ftyp(), kiste(b"moov", &kiste(b"cmov", b"gepackt"))].concat();
        let (ergebnis, _) = tilgen(cmov);
        assert!(
            matches!(ergebnis, Err(Abbruch::Metadaten(_))),
            "{ergebnis:?}"
        );

        let mut zu_lang = kiste(&[0xA9, b'x', b'y', b'z'], ORT);
        zu_lang[3] += 1;
        let udta = kiste(b"udta", &zu_lang);
        let ueber = [
            ftyp(),
            kiste(b"moov", &[udta, kiste(b"free", b"x")].concat()),
        ]
        .concat();
        let (ergebnis, _) = tilgen(ueber);
        assert!(
            matches!(ergebnis, Err(Abbruch::Metadaten(_))),
            "{ergebnis:?}"
        );
    }

    /// `meta` mit `hdlr` (Handler `mdir`), `keys`, `ilst` aus den Einträgen und einer unbekannten Box.
    fn nutzer_meta(eintraege: &[Vec<u8>], volle_box: bool) -> Vec<u8> {
        let hdlr = kiste(b"hdlr", &[&[0u8; 8][..], b"mdir", &[0; 13]].concat());
        let keys = kiste(b"keys", b"\0\0\0\0\0\0\0\x01\0\0\0\x0cmdtamake");
        let fremd = kiste(b"abcd", ORT);
        let kopf: &[u8] = if volle_box { &[0, 0, 0, 0] } else { &[] };
        kiste(
            b"meta",
            &[
                kopf,
                &hdlr,
                &keys,
                &kiste(b"ilst", &eintraege.concat()),
                &fremd,
            ]
            .concat(),
        )
    }

    fn wert(typ: &[u8], text: &[u8]) -> Vec<u8> {
        kiste(
            typ,
            &kiste(b"data", &[&[0u8, 0, 0, 1, 0, 0, 0, 0][..], text].concat()),
        )
    }

    /// ADR-0047 B2 P5 und Regel 5d (B0645): Ort nur in `moov/udta/meta/ilst/©xyz`, dazu Hersteller, Titel, ein
    /// `----`-Eintrag mit `mean`/`name` → jeder Wert genullt (Köpfe bleiben), `keys` und `hdlr` byte-gleich, eine
    /// unbekannte Box im `meta` → `free`; dasselbe für `meta` in `trak` und ein oberstes `meta` mit Handler `mdir`.
    #[test]
    fn in_ilst_werden_alle_werte_getilgt() {
        let freiform = kiste(
            b"----",
            &[
                kiste(b"mean", b"\0\0\0\0com.apple.iTunes"),
                kiste(b"name", b"\0\0\0\0Aufnahmeort"),
                kiste(b"data", b"\0\0\0\x01\0\0\0\0Berlin Mitte"),
            ]
            .concat(),
        );
        let eintraege = vec![
            wert(&[0xA9, b'x', b'y', b'z'], b"+52.5200+013.4050/"),
            wert(&[0xA9, b'm', b'a', b'k'], b"Telefonbauer"),
            wert(&[0xA9, b'n', b'a', b'm'], b"Urlaubsfilm"),
            wert(&1u32.to_be_bytes(), b"Kamera X"),
            freiform,
        ];
        let im_udta = kiste(b"udta", &nutzer_meta(&eintraege, true));
        let im_trak = kiste(b"trak", &nutzer_meta(&eintraege, false));
        let original = [
            ftyp(),
            nutzer_meta(&eintraege, true),
            kiste(b"moov", &[im_trak, im_udta].concat()),
            kiste(b"mdat", b"Mediendaten"),
        ]
        .concat();

        let (ergebnis, datei) = tilgen(original.clone());

        ergebnis.expect("getilgt");
        let getilgt = datei.inhalt();
        assert_eq!(getilgt.len(), original.len());
        for spur in [
            &b"+52.5200"[..],
            b"52.52",
            b"Telefonbauer",
            b"Urlaubsfilm",
            b"Kamera X",
            b"Berlin",
            b"Aufnahmeort",
            b"com.apple.iTunes",
            b"GPS",
        ] {
            assert!(
                !enthaelt(&getilgt, spur),
                "{}",
                String::from_utf8_lossy(spur)
            );
        }
        assert!(enthaelt(&getilgt, b"mdtamake") && enthaelt(&getilgt, b"mdir"));
        assert_eq!(getilgt.windows(4).filter(|f| f == b"abcd").count(), 0);
        assert_eq!(getilgt.windows(4).filter(|f| f == b"data").count(), 15);
        assert!(enthaelt(&getilgt, b"Mediendaten"));
    }

    fn mime_infe(id: u16, inhaltstyp: &[u8]) -> Vec<u8> {
        kiste(
            b"infe",
            &[
                &[2u8, 0, 0, 0][..],
                &id.to_be_bytes(),
                &[0, 0],
                b"mime",
                b"\0",
                inhaltstyp,
                b"\0",
            ]
            .concat(),
        )
    }

    /// ADR-0047 B2 P7 und P8 (HEIC-Teil), Regel 5e (B0646): `mime`-Item `application/c2pa`, unbekanntes Item `abcd`,
    /// `xml ` im obersten `meta`, `udes` in `ipco` → genullt bzw. `free`; Bildkacheln `hvc1`, `grid`, `tmap`,
    /// Eigenschaften `colr`, `irot`, `ispe` byte-gleich, Exif minimal.
    #[test]
    fn heif_behaelt_bild_items_und_eigenschaften_der_rest_wird_getilgt() {
        let exif = exif_mit_ort();
        let lang = exif.len() as u64;
        let kachel = vec![0x5A; 40];
        let grid = vec![0, 0, 0, 0, 0, 0x10, 0, 0x10];
        let tmap = vec![0x7B; 24];
        let c2pa = [&b"c2pa manifest "[..], ORT].concat();
        let fremd = [&b"abcd "[..], ORT].concat();
        let daten = [
            kachel.clone(),
            grid.clone(),
            tmap.clone(),
            exif.clone(),
            c2pa.clone(),
            fremd.clone(),
        ]
        .concat();
        let mut ab = Vec::new();
        let mut stelle = 0u64;
        for teil in [&kachel, &grid, &tmap, &exif, &c2pa, &fremd] {
            ab.push(stelle);
            stelle += teil.len() as u64;
        }
        let colr = kiste(b"colr", b"nclx\0\x01\0\x0d\0\x01\x80");
        let irot = kiste(b"irot", b"\x01");
        let ispe = kiste(b"ispe", b"\0\0\0\0\0\0\0\x10\0\0\0\x10");
        let udes = kiste(b"udes", &[&b"\0\0\0\0de\0Urlaub\0"[..], ORT].concat());
        let iprp = kiste(
            b"iprp",
            &[
                kiste(
                    b"ipco",
                    &[colr.clone(), irot.clone(), ispe.clone(), udes].concat(),
                ),
                kiste(b"ipma", b"\0\0\0\0\0\0\0\0"),
            ]
            .concat(),
        );
        let xml = kiste(b"xml ", &[&b"\0\0\0\0<x>"[..], ORT].concat());
        let infe = [
            item_infe(1, b"hvc1"),
            item_infe(2, b"grid"),
            item_infe(3, b"tmap"),
            item_infe(4, b"Exif"),
            mime_infe(5, b"application/c2pa"),
            item_infe(6, b"abcd"),
        ];
        let iinf = kiste(
            b"iinf",
            &[&[0u8, 0, 0, 0, 0, 6][..], &infe.concat()].concat(),
        );
        let laengen = [40u64, 8, 24, lang, c2pa.len() as u64, fremd.len() as u64];
        let bauen = |mdat_anfang: u64| {
            let verweise: Vec<(u16, u64, u64)> = (0..6)
                .map(|n| (n as u16 + 1, mdat_anfang + ab[n], laengen[n]))
                .collect();
            let meta = kiste(
                b"meta",
                &[
                    &[0u8, 0, 0, 0][..],
                    &hdlr_pict(),
                    &kiste(b"pitm", b"\0\0\0\0\0\x02"),
                    &iinf,
                    &iloc_aus(&verweise),
                    &iprp,
                    &xml,
                ]
                .concat(),
            );
            [kiste(b"ftyp", b"heic\0\0\0\0mif1heic"), meta].concat()
        };
        let mdat_anfang = bauen(0).len() as u64 + 8;
        let original = [bauen(mdat_anfang), kiste(b"mdat", &daten)].concat();

        let (ergebnis, datei) = tilgen(original.clone());

        ergebnis.expect("getilgt");
        let getilgt = datei.inhalt();
        assert_eq!(getilgt.len(), original.len());
        assert!(!enthaelt(&getilgt, b"52.52") && !enthaelt(&getilgt, b"GPS"));
        assert!(!enthaelt(&getilgt, b"c2pa manifest") && !enthaelt(&getilgt, b"Urlaub"));
        for bleibt in [&kachel, &grid, &tmap, &colr, &irot, &ispe, &iinf] {
            assert!(
                enthaelt(&getilgt, bleibt),
                "{}",
                String::from_utf8_lossy(&bleibt[..8.min(bleibt.len())])
            );
        }
        assert!(!enthaelt(&getilgt, b"udes") && !enthaelt(&getilgt, b"xml "));
        let tiff = getilgt
            .windows(4)
            .position(|f| f == b"MM\0*")
            .expect("minimales EXIF");
        assert_eq!(orientierung(&getilgt[tiff..]), Some(6));
    }

    /// 2026-10-29T12:34:56Z in Sekunden seit 1904-01-01 (Zeitrechnung von ISOBMFF).
    const ZEIT: u64 = 3_876_122_096;

    /// Eine Zeit-Box (`mvhd`, `tkhd`, `mdhd`) in Version 0 oder 1: Erstell- und Änderungszeit, dann `timescale`
    /// bzw. Spurkennung und `duration`, dann `rest`.
    fn zeitkiste(typ: &[u8], version: u8, mitte: u32, dauer: u64, rest: &[u8]) -> Vec<u8> {
        let mut inhalt = vec![version, 0, 0, 0];
        if version == 1 {
            inhalt.extend_from_slice(&ZEIT.to_be_bytes());
            inhalt.extend_from_slice(&(ZEIT + 1).to_be_bytes());
            inhalt.extend_from_slice(&mitte.to_be_bytes());
            if typ == b"tkhd" {
                inhalt.extend_from_slice(&[0; 4]);
            }
            inhalt.extend_from_slice(&dauer.to_be_bytes());
        } else {
            inhalt.extend_from_slice(&(ZEIT as u32).to_be_bytes());
            inhalt.extend_from_slice(&(ZEIT as u32 + 1).to_be_bytes());
            inhalt.extend_from_slice(&mitte.to_be_bytes());
            if typ == b"tkhd" {
                inhalt.extend_from_slice(&[0; 4]);
            }
            inhalt.extend_from_slice(&(dauer as u32).to_be_bytes());
        }
        inhalt.extend_from_slice(rest);
        kiste(typ, &inhalt)
    }

    fn film(version: u8) -> Vec<u8> {
        let mvhd = zeitkiste(b"mvhd", version, 1_000, 5_000, &[0x11; 80]);
        let tkhd = zeitkiste(b"tkhd", version, 1, 5_000, &[0x22; 60]);
        let mdhd = zeitkiste(b"mdhd", version, 90_000, 450_000, &[0x55, 0xC4, 0, 0]);
        let mdia = kiste(
            b"mdia",
            &[
                mdhd,
                kiste(b"hdlr", &[0x33; 25]),
                kiste(b"minf", &[0x44; 30]),
            ]
            .concat(),
        );
        let trak = kiste(b"trak", &[tkhd, kiste(b"edts", &[0x66; 20]), mdia].concat());
        let tfdt = kiste(
            b"tfdt",
            &[&[1u8, 0, 0, 0][..], &ZEIT.to_be_bytes()].concat(),
        );
        let mvex = kiste(
            b"mvex",
            &kiste(
                b"mehd",
                &[&[1u8, 0, 0, 0][..], &ZEIT.to_be_bytes()].concat(),
            ),
        );
        let moof = kiste(b"moof", &kiste(b"traf", &tfdt));
        [
            ftyp(),
            kiste(b"moov", &[mvhd, trak, mvex].concat()),
            moof,
            kiste(b"mdat", b"Mediendaten"),
        ]
        .concat()
    }

    /// Die Bytes einer Box `typ` (erste Fundstelle, ab dem Kopf).
    fn box_bei<'a>(bytes: &'a [u8], typ: &[u8]) -> &'a [u8] {
        let stelle = bytes.windows(4).position(|f| f == typ).expect("Box") - 4;
        let groesse =
            u32::from_be_bytes(bytes[stelle..stelle + 4].try_into().expect("Größe")) as usize;
        &bytes[stelle..stelle + groesse]
    }

    /// ADR-0047 B2 Regel 5g, P10 (Unit) (B0647): in `mvhd`, `trak/tkhd`, `trak/mdia/mdhd` sind Erstell- und
    /// Änderungszeit genullt (Version 0 je 4, Version 1 je 8 Bytes); `timescale`, Spurkennung, `duration`, Sprache,
    /// der Rest der Boxen, `tfdt` und `mehd` bleiben byte-gleich.
    #[test]
    fn zeitfelder_werden_genullt_dauer_und_takt_bleiben() {
        for version in [0u8, 1] {
            let original = film(version);
            let breite = if version == 1 { 8 } else { 4 };

            let (ergebnis, datei) = tilgen(original.clone());

            ergebnis.expect("getilgt");
            let getilgt = datei.inhalt();
            assert_eq!(getilgt.len(), original.len());
            for typ in [&b"mvhd"[..], b"tkhd", b"mdhd"] {
                let vorher = box_bei(&original, typ);
                let nachher = box_bei(&getilgt, typ);
                let zeiten = 12..12 + 2 * breite;
                assert!(
                    nachher[zeiten.clone()].iter().all(|b| *b == 0),
                    "v{version} {}",
                    String::from_utf8_lossy(typ)
                );
                assert_eq!(nachher[..12], vorher[..12]);
                assert_eq!(
                    nachher[zeiten.end..],
                    vorher[zeiten.end..],
                    "v{version} Dauer und Rest"
                );
            }
            for typ in [&b"tfdt"[..], b"mehd", b"edts", b"hdlr", b"minf", b"mdat"] {
                assert_eq!(box_bei(&getilgt, typ), box_bei(&original, typ));
            }
            assert_eq!(
                getilgt
                    .windows(4)
                    .filter(|f| *f == (ZEIT as u32).to_be_bytes())
                    .count(),
                2,
                "nur tfdt und mehd tragen die Zahl noch"
            );
        }
    }

    /// Eine Ortsspur wie in Dashcams: NMEA-Satz und Koordinate als Text.
    const NMEA: &[u8] =
        b"$GPRMC,123456.00,A,5231.2000,N,01324.3000,E,0.0,0.0,291026,,,A*6C GPSLatitude 52.52";

    /// Platzhalter `typ` mit der Kennung `marke` und der Ortsspur als Inhalt.
    fn platzhalter(typ: &[u8; 4], marke: &str) -> Vec<u8> {
        kiste(typ, &[marke.as_bytes(), NMEA].concat())
    }

    /// Der Platzhalter mit dem Inhalt `alt` steht im Ergebnis mit gleichem Kopf (Größe, Typ) und genulltem Inhalt.
    fn platzhalter_genullt(original: &[u8], getilgt: &[u8], alt: &[u8], was: &str) {
        let stelle = original
            .windows(alt.len())
            .position(|fenster| fenster == alt)
            .expect("Platzhalter im Original");
        let (kopf, ende) = (stelle..stelle + 8, stelle + alt.len());
        assert_eq!(getilgt[kopf.clone()], original[kopf], "{was}: Kopf");
        assert!(
            getilgt[stelle + 8..ende].iter().all(|b| *b == 0),
            "{was}: Inhalt"
        );
    }

    /// ADR-0047 B3 Regel 5h, Proben (a)–(f) (B0649): `free`, `skip`, `wide` auf jeder betretenen Ebene — oben (mit
    /// `ptrh`/`gps`, `LIGOGPSINFO`, einem `mdat` samt JPEG mit Exif-GPS), in `moov`, `trak`, `mdia`, `udta`, `meta`,
    /// `ilst` — behalten Typ, Größe und Kopf, ihr Inhalt ist Nullen; `mdat`, `hdlr`, `stco` byte-gleich.
    #[test]
    fn platzhalter_behalten_typ_und_groesse_ihr_inhalt_ist_genullt() {
        let jpeg_mit_gps = [
            &b"\xFF\xD8\xFF\xE1\0\x30Exif\0\0MM\0*GPS\0\0\0\x34\0\0\0\x01\0\0\0\x1F"[..],
            NMEA,
        ]
        .concat();
        let oben = [
            platzhalter(b"free", "ptrh gps "),
            platzhalter(b"skip", "LIGOGPSINFO "),
            kiste(b"free", &kiste(b"mdat", &jpeg_mit_gps)),
            platzhalter(b"wide", "wide oben "),
        ];
        let hdlr = kiste(b"hdlr", &[&[0u8; 8][..], b"mdir", &[0; 13]].concat());
        let ilst = kiste(
            b"ilst",
            &[
                platzhalter(b"free", "in ilst "),
                wert(&[0xA9, b'n', b'a', b'm'], b"Titel"),
            ]
            .concat(),
        );
        let meta_kinder = [
            hdlr.clone(),
            platzhalter(b"free", "in meta "),
            platzhalter(b"wide", "wide in meta "),
            ilst,
        ];
        let udta_kinder = [
            platzhalter(b"free", "in udta "),
            platzhalter(b"skip", "skip in udta "),
            kiste(b"meta", &[&[0u8; 4][..], &meta_kinder.concat()].concat()),
        ];
        let stco = kiste(b"stco", b"\0\0\0\0\0\0\0\x01\0\0\0\x30");
        let mdia = kiste(b"mdia", &platzhalter(b"free", "in mdia "));
        let trak = kiste(
            b"trak",
            &[stco.clone(), platzhalter(b"skip", "in trak "), mdia].concat(),
        );
        let moov = kiste(
            b"moov",
            &[
                platzhalter(b"free", "in moov "),
                trak,
                kiste(b"udta", &udta_kinder.concat()),
                platzhalter(b"wide", "wide in moov "),
            ]
            .concat(),
        );
        let mdat = kiste(b"mdat", b"Mediendaten");
        let original = [ftyp(), oben.concat(), moov, mdat.clone()].concat();

        let (ergebnis, datei) = tilgen(original.clone());

        ergebnis.expect("getilgt");
        let getilgt = datei.inhalt();
        assert_eq!(getilgt.len(), original.len());
        for spur in [&b"5231.2000"[..], b"52.52", b"GPS", b"LIGOGPSINFO", b"ptrh"] {
            assert!(
                !enthaelt(&getilgt, spur),
                "{}",
                String::from_utf8_lossy(spur)
            );
        }
        for (marke, typ) in [
            ("ptrh gps ", b"free"),
            ("LIGOGPSINFO ", b"skip"),
            ("wide oben ", b"wide"),
            ("in moov ", b"free"),
            ("in trak ", b"skip"),
            ("in mdia ", b"free"),
            ("in udta ", b"free"),
            ("skip in udta ", b"skip"),
            ("in meta ", b"free"),
            ("wide in meta ", b"wide"),
            ("wide in moov ", b"wide"),
            ("in ilst ", b"free"),
        ] {
            platzhalter_genullt(&original, &getilgt, &platzhalter(typ, marke), marke);
        }
        let mit_jpeg = kiste(b"free", &kiste(b"mdat", &jpeg_mit_gps));
        platzhalter_genullt(&original, &getilgt, &mit_jpeg, "free mit JPEG");
        for bleibt in [&mdat, &hdlr, &stco] {
            assert!(enthaelt(&getilgt, bleibt));
        }
        assert_eq!(&getilgt[getilgt.len() - mdat.len()..], &mdat[..]);
    }

    /// ADR-0047 B3 Regel 5h, Probe (g) (B0649): `free` und `skip` im obersten HEIF-`meta` und ein `free` in `ipco` →
    /// Kopf gleich, Inhalt Nullen; das Exif-Item minimal wie vorher.
    #[test]
    fn platzhalter_im_obersten_heif_meta_und_in_ipco_werden_genullt() {
        let exif = exif_mit_ort();
        let lang = exif.len() as u64;
        let im_meta = platzhalter(b"free", "HEIF meta ");
        let skip = platzhalter(b"skip", "HEIF skip ");
        let in_ipco = platzhalter(b"free", "in ipco ");
        let iprp = kiste(b"iprp", &kiste(b"ipco", &in_ipco));
        let iinf = kiste(
            b"iinf",
            &[&[0u8, 0, 0, 0, 0, 1][..], &item_infe(1, b"Exif")].concat(),
        );
        let bauen = |mdat_anfang: u64| {
            let meta = kiste(
                b"meta",
                &[
                    &[0u8, 0, 0, 0][..],
                    &hdlr_pict(),
                    &iinf,
                    &iloc_aus(&[(1, mdat_anfang, lang)]),
                    &iprp,
                    &im_meta,
                    &skip,
                ]
                .concat(),
            );
            [kiste(b"ftyp", b"heic\0\0\0\0mif1heic"), meta].concat()
        };
        let mdat_anfang = bauen(0).len() as u64 + 8;
        let original = [bauen(mdat_anfang), kiste(b"mdat", &exif)].concat();

        let (ergebnis, datei) = tilgen(original.clone());

        ergebnis.expect("getilgt");
        let getilgt = datei.inhalt();
        assert!(!enthaelt(&getilgt, b"5231.2000") && !enthaelt(&getilgt, b"GPS"));
        for (alt, was) in [(&im_meta, "meta"), (&skip, "skip"), (&in_ipco, "ipco")] {
            platzhalter_genullt(&original, &getilgt, alt, was);
        }
    }

    /// ADR-0047 B3 Regel 5h (B0649): `free` mit `size` = 1 → 16 Bytes Kopf bleiben, Rest Nullen; `free` mit `size` 0
    /// am Dateiende → bis zum Ende Nullen, Länge gleich; ein `free` von 8 Bytes bleibt, wie es ist; ein `skip` von
    /// 3 MiB wird nie gelesen.
    #[test]
    fn platzhalter_mit_64_bit_groesse_groesse_0_und_ohne_inhalt() {
        let inhalt = [&b"64 Bit "[..], NMEA].concat();
        let mut gross = 1u32.to_be_bytes().to_vec();
        gross.extend_from_slice(b"free");
        gross.extend_from_slice(&(16 + inhalt.len() as u64).to_be_bytes());
        gross.extend_from_slice(&inhalt);
        let leer = kiste(b"free", b"");
        let riesig = kiste(b"skip", &[NMEA, &vec![b'7'; 3 * MIB]].concat());
        let bis_ende = [&0u32.to_be_bytes()[..], b"free", b"bis ende ", NMEA].concat();
        let original = [
            ftyp(),
            gross.clone(),
            leer.clone(),
            riesig,
            kiste(b"mdat", b"Mediendaten"),
            bis_ende.clone(),
        ]
        .concat();

        let (ergebnis, datei) = tilgen(original.clone());

        ergebnis.expect("getilgt");
        let getilgt = datei.inhalt();
        assert_eq!(getilgt.len(), original.len());
        assert!(!enthaelt(&getilgt, b"5231.2000") && !enthaelt(&getilgt, &[b'7'; 64]));
        let stelle = ftyp().len();
        assert_eq!(getilgt[stelle..stelle + 16], gross[..16]);
        assert!(
            getilgt[stelle + 16..stelle + gross.len()]
                .iter()
                .all(|b| *b == 0)
        );
        let ende = getilgt.len() - bis_ende.len();
        assert_eq!(getilgt[ende..ende + 8], bis_ende[..8]);
        assert!(getilgt[ende + 8..].iter().all(|b| *b == 0));
        assert!(enthaelt(&getilgt, b"Mediendaten"));
        assert!(datei.groesste_lesung() <= 16, "{}", datei.groesste_lesung());
        let leer_an = stelle + gross.len();
        assert_eq!(getilgt[leer_an..leer_an + 8], leer[..]);
    }

    /// ADR-0047 B3 Regel 5h (B0649): `free` mit `size` 0 in `moov` → `metadaten`.
    #[test]
    fn ein_platzhalter_mit_groesse_0_in_moov_ist_nicht_tilgbar() {
        let ohne_groesse = [&0u32.to_be_bytes()[..], b"free", NMEA].concat();
        let daten = [ftyp(), kiste(b"moov", &ohne_groesse)].concat();

        let (ergebnis, _) = tilgen(daten);

        assert!(
            matches!(ergebnis, Err(Abbruch::Metadaten(_))),
            "{ergebnis:?}"
        );
    }

    /// NTP-Zeit 2026-10-29T12:34:56Z (Sekunden seit 1900, Bruchteil 0).
    const NTP: [u8; 8] = [0xEE, 0x8D, 0xBB, 0xF0, 0, 0, 0, 0];

    /// ADR-0047 B3 Regel 5i, P11 (h)–(k) (B0650): oben `meco` (mit `meta`/`ilst`/`©xyz`), `emsg` (Ort in
    /// `message_data`), `prft` (`ntp_timestamp`) → je `free`, Inhalt genullt; `moof` wird betreten: `meta` und eine
    /// unbekannte Box darin → `free` mit Nullen, `free` darin nach 5h, `mfhd`, `traf` (samt `uuid` darin), `pssh`
    /// byte-gleich. Kontrolle: `sidx`, `ssix`, `mfra`, `pdin` oben byte-gleich.
    #[test]
    fn meco_emsg_prft_werden_unkenntlich_und_moof_wird_betreten() {
        let ilst_meta = |marke: &[u8]| {
            let hdlr = kiste(b"hdlr", &[&[0u8; 8][..], b"mdir", &[0; 13]].concat());
            let ilst = kiste(
                b"ilst",
                &wert(&[0xA9, b'x', b'y', b'z'], &[marke, ORT].concat()),
            );
            kiste(b"meta", &[&[0u8; 4][..], &hdlr, &ilst].concat())
        };
        let meco = kiste(b"meco", &ilst_meta(b"meco "));
        let emsg = kiste(
            b"emsg",
            &[&[0u8; 4][..], b"urn:ort\0\0", &[0; 16], ORT].concat(),
        );
        let prft = kiste(
            b"prft",
            &[&[0u8; 4][..], &[0, 0, 0, 1], &NTP, &[0; 4]].concat(),
        );
        let mfhd = kiste(b"mfhd", b"\0\0\0\0\0\0\0\x01");
        let uuid = kiste(b"uuid", &[&[0x11u8; 16][..], b"PIFF"].concat());
        let traf = kiste(
            b"traf",
            &[kiste(b"tfhd", b"\0\x02\0\0\0\0\0\x01"), uuid.clone()].concat(),
        );
        let pssh = kiste(b"pssh", &[0x22; 24]);
        let im_moof = platzhalter(b"free", "in moof ");
        let fremd = kiste(b"abcd", ORT);
        let moof = kiste(
            b"moof",
            &[
                mfhd.clone(),
                ilst_meta(b"moof "),
                traf.clone(),
                pssh.clone(),
                fremd,
                im_moof.clone(),
            ]
            .concat(),
        );
        let bleiben = [
            kiste(b"sidx", &[0x33; 32]),
            kiste(b"ssix", &[0x44; 12]),
            kiste(b"pdin", &[0x55; 12]),
            kiste(b"mfra", &kiste(b"mfro", &[0, 0, 0, 0, 0, 0, 0, 16])),
        ];
        let original = [
            ftyp(),
            meco.clone(),
            bleiben[0].clone(),
            bleiben[1].clone(),
            bleiben[2].clone(),
            emsg.clone(),
            prft.clone(),
            moof,
            kiste(b"mdat", b"Mediendaten"),
            bleiben[3].clone(),
        ]
        .concat();

        let (ergebnis, datei) = tilgen(original.clone());

        ergebnis.expect("getilgt");
        let getilgt = datei.inhalt();
        assert_eq!(getilgt.len(), original.len());
        assert!(!enthaelt(&getilgt, b"52.52") && !enthaelt(&getilgt, b"GPS"));
        assert!(!enthaelt(&getilgt, &NTP) && !enthaelt(&getilgt, b"urn:ort"));
        for (alt, was) in [(&meco, "meco"), (&emsg, "emsg"), (&prft, "prft")] {
            assert!(frei_an(&getilgt, alt), "{was}");
        }
        for typ in [&b"meco"[..], b"emsg", b"prft", b"abcd"] {
            assert!(!enthaelt(&getilgt, typ), "{}", String::from_utf8_lossy(typ));
        }
        platzhalter_genullt(&original, &getilgt, &im_moof, "free in moof");
        for bleibt in [
            &mfhd,
            &traf,
            &uuid,
            &pssh,
            &bleiben[0],
            &bleiben[1],
            &bleiben[2],
            &bleiben[3],
        ] {
            assert!(
                enthaelt(&getilgt, bleibt),
                "{}",
                String::from_utf8_lossy(&bleibt[4..8])
            );
        }
        assert_eq!(getilgt.windows(4).filter(|f| f == b"moof").count(), 1);
    }

    /// Regel 5g: eine Zeit-Box mit unbekannter Version oder zu kurz → `metadaten`.
    #[test]
    fn eine_zeitbox_mit_unbekannter_version_oder_zu_kurz_ist_nicht_tilgbar() {
        for mvhd in [
            kiste(b"mvhd", &[2u8; 100]),
            kiste(b"mvhd", &[0u8; 10]),
            kiste(b"mvhd", &[1u8, 0, 0, 0, 0, 0]),
        ] {
            let (ergebnis, _) = tilgen([ftyp(), kiste(b"moov", &mvhd)].concat());
            assert!(
                matches!(ergebnis, Err(Abbruch::Metadaten(_))),
                "{ergebnis:?}"
            );
        }
    }
}
