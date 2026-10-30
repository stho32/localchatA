//! Ortsangaben in ISOBMFF tilgen (B0503, A00018 Ergänzung F4; ADR-0047 Ergänzung „Karte #9 Security-Gate“ Regeln
//! 1–3): MP4, MOV, M4A, 3GP, HEIC, HEIF, AVIF über Boxköpfe per Position. Abstieg nur in `moov`, `moov/udta`,
//! `moov/meta`, `moov/meta/ilst` und das oberste `meta`; Leseziele ≤ 1 MiB, Nullziele nie gelesen.

use super::Zaehler;
use super::minimal::{minimales_exif, traegt_ort};
use super::port::{Abbruch, Ergebnis, Port, STUECK};
use crate::gespraeche::anhangpruefung::exif::orientierung;
use std::collections::{HashMap, HashSet};

/// `moov` und `meta` sind höchstens so groß — nur als Längengrenze, nie als Puffer.
pub const HOECHSTENS_MOOV: u64 = 64 * 1_048_576;
/// Typ der XMP-`uuid`-Box.
pub const XMP_UUID: [u8; 16] = [
    0xBE, 0x7A, 0xCF, 0xCB, 0x97, 0xA9, 0x42, 0xE8, 0x9C, 0x71, 0x99, 0x94, 0x91, 0xE3, 0xAF, 0xAC,
];

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
            b"meta" => oberstes_meta(port, kiste, zaehler)?,
            b"uuid" => xmp_uuid(port, kiste)?,
            _ => {}
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

/// `uuid` mit dem XMP-Typ → Typ `free`, Inhalt samt den 16 Bytes des Typs genullt.
fn xmp_uuid(port: &Port<'_>, kiste: Kiste) -> Ergebnis<()> {
    if kiste.ende - kiste.inhalt < 16 || port.lesen(kiste.inhalt, 16)? != XMP_UUID {
        return Ok(());
    }
    frei_machen(port, kiste)
}

fn frei_machen(port: &Port<'_>, kiste: Kiste) -> Ergebnis<()> {
    port.schreiben(kiste.pos + 4, b"free")?;
    port.nullen(kiste.inhalt, kiste.ende - kiste.inhalt)
}

fn inhalt_nullen(port: &Port<'_>, kiste: Kiste, zaehler: &mut Zaehler) -> Ergebnis<()> {
    zaehler.ort = true;
    port.nullen(kiste.inhalt, kiste.ende - kiste.inhalt)
}

fn moov(port: &Port<'_>, moov: Kiste, zaehler: &mut Zaehler) -> Ergebnis<()> {
    if zu_gross(moov) {
        return metadaten("moov über 64 MiB");
    }
    for kind in kinder(port, moov.inhalt, moov.ende, zaehler)? {
        match &kind.typ {
            b"udta" => udta(port, kind, zaehler)?,
            b"meta" => moov_meta(port, kind, zaehler)?,
            b"uuid" => xmp_uuid(port, kind)?,
            _ => {}
        }
    }
    Ok(())
}

fn udta(port: &Port<'_>, udta: Kiste, zaehler: &mut Zaehler) -> Ergebnis<()> {
    for kind in kinder(port, udta.inhalt, udta.ende, zaehler)? {
        match &kind.typ {
            [0xA9, b'x', b'y', b'z'] | b"loci" => inhalt_nullen(port, kind, zaehler)?,
            b"XMP_" => frei_machen(port, kind)?,
            b"uuid" => xmp_uuid(port, kind)?,
            _ => {}
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

/// `moov/meta`: `keys` lesen (Leseziel ≤ 1 MiB), in `ilst` die `data`-Werte der Schlüssel mit `location` nullen.
fn moov_meta(port: &Port<'_>, meta: Kiste, zaehler: &mut Zaehler) -> Ergebnis<()> {
    if zu_gross(meta) {
        return metadaten("meta über 64 MiB");
    }
    let anfang = meta_kinder_ab(port, meta)?.min(meta.ende);
    let kinder = kinder(port, anfang, meta.ende, zaehler)?;
    let mut orte = HashSet::new();
    if let Some(keys) = kinder.iter().find(|kind| &kind.typ == b"keys") {
        orte = ortsschluessel(port, *keys)?;
    }
    for kind in kinder.iter().filter(|kind| &kind.typ == b"ilst") {
        for eintrag in self::kinder(port, kind.inhalt, kind.ende, zaehler)? {
            if !orte.contains(&u32::from_be_bytes(eintrag.typ)) {
                continue;
            }
            for wert in self::kinder(port, eintrag.inhalt, eintrag.ende, zaehler)? {
                if &wert.typ == b"data" && wert.ende - wert.inhalt >= 8 {
                    zaehler.ort = true;
                    port.nullen(wert.inhalt + 8, wert.ende - wert.inhalt - 8)?;
                }
            }
        }
    }
    Ok(())
}

/// Die Nummern (ab 1) der Schlüssel in `keys`, die `location` enthalten.
fn ortsschluessel(port: &Port<'_>, keys: Kiste) -> Ergebnis<HashSet<u32>> {
    let bytes = port.lesen(keys.inhalt, keys.ende - keys.inhalt)?;
    let anzahl = lies_u32(&bytes, 4)?;
    let mut stelle = 8usize;
    let mut orte = HashSet::new();
    for nummer in 1..=anzahl {
        let groesse = lies_u32(&bytes, stelle)? as usize;
        if groesse < 8
            || stelle
                .checked_add(groesse)
                .is_none_or(|ende| ende > bytes.len())
        {
            return metadaten("Längen widersprechen sich");
        }
        let schluessel =
            String::from_utf8_lossy(&bytes[stelle + 8..stelle + groesse]).to_lowercase();
        if schluessel.contains("location") {
            orte.insert(nummer);
        }
        stelle += groesse;
    }
    Ok(orte)
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
            Item::Xmp => port.nullen(pos, laenge)?,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Item {
    Exif,
    Xmp,
}

/// `iinf`: die Ziele — Items vom Typ `Exif` und `mime` mit XMP (`infe` ab Version 2); jede `item_ID` höchstens einmal
/// (alle `infe`, jede Version), sonst „Verweise doppelt“.
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
        if version < 2 {
            continue;
        }
        let typ = bytes
            .get(4 + breite + 2..4 + breite + 6)
            .ok_or(Abbruch::Metadaten("Längen widersprechen sich"))?;
        if typ == b"Exif" {
            gefunden.insert(id, Item::Exif);
        } else if typ == b"mime" {
            let rest = String::from_utf8_lossy(&bytes[4 + breite + 6..]).to_lowercase();
            if rest.contains("rdf+xml") || rest.contains("xmp") {
                gefunden.insert(id, Item::Xmp);
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

    fn kiste(typ: &[u8], inhalt: &[u8]) -> Vec<u8> {
        let mut bytes = ((inhalt.len() + 8) as u32).to_be_bytes().to_vec();
        bytes.extend_from_slice(typ);
        bytes.extend_from_slice(inhalt);
        bytes
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
            &[&stco[..], &vec![7u8; trak_fuellung][..]].concat(),
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
        assert_eq!(getilgt.windows(4).filter(|f| f == b"free").count(), 2);
        assert!(zaehler.ort);
    }

    #[test]
    fn moov_knapp_unter_64_mib_mit_xyz_am_ende() {
        let original = mp4(64 * MIB - 108, b"+52.5200+013.4050/");
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

    #[test]
    fn keys_von_1_mib_und_einem_byte_ist_nicht_tilgbar() {
        let keys = kiste(b"keys", &vec![0u8; MIB + 1]);
        let meta = kiste(b"meta", &[kiste(b"hdlr", &[0; 25]), keys].concat());
        let original = [ftyp(), kiste(b"moov", &meta)].concat();

        let (ergebnis, _) = tilgen(original);

        assert!(matches!(
            ergebnis,
            Err(Abbruch::Metadaten("Leseziel über 1 MiB"))
        ));
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

        assert!(ergebnis.expect("getilgt").ort);
        let getilgt = datei.inhalt();
        assert!(!enthaelt(&getilgt, b"+52.5200") && enthaelt(&getilgt, b"Apple"));
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
        let meta_ohne_iloc = 8 + 4 + kiste(b"hdlr", &[0; 25]).len() + iinf.len();
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
            &[&[0u8, 0, 0, 0][..], &kiste(b"hdlr", &[0; 25]), &iinf, &iloc].concat(),
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
                &[
                    &[0u8, 0, 0, 0][..],
                    &kiste(b"hdlr", &[0; 25]),
                    &iinf,
                    &iloc(mdat_anfang),
                ]
                .concat(),
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
        assert!(enthaelt(&getilgt, b"Apple") && enthaelt(&getilgt, b"Mediendaten"));
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
}
