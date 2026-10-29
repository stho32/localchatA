//! AVIF-Maße (B0585, ADR-0047 Ergänzung „Karte #9 Nacharbeit F3 und F4“ Regeln 3, 4): `ispe` des Primärbilds über die
//! Längenkette `meta` → `pitm` und `iprp` → `ipco`/`ipma`; `irot` mit Winkel 1 oder 3 tauscht Breite und Höhe. Rein,
//! im Signaturfenster des Erkenners; jede Verletzung ergibt kein Maß, nie eine Abweisung.

/// Höchstens so viele Pixel je Seite und insgesamt (Grenzen der Bilder, Regel 3).
const SEITE_HOECHSTENS: u32 = 16_384;
const PIXEL_HOECHSTENS: u64 = 50_000_000;

/// Eine Box: Typ und Inhalt hinter dem Kopf.
#[derive(Debug, Clone, Copy)]
struct Kiste<'a> {
    typ: [u8; 4],
    inhalt: &'a [u8],
}

/// Die angezeigten Maße des Primärbilds, sonst keine.
pub fn masse(kopf: &[u8]) -> Option<(u32, u32)> {
    let meta = finden(kopf, b"meta")?.inhalt.get(4..)?;
    let primaer = primaerbild(finden(meta, b"pitm")?.inhalt)?;
    let iprp = finden(meta, b"iprp")?.inhalt;
    let eigenschaften = alle(finden(iprp, b"ipco")?.inhalt)?;
    let zugeordnet = zuordnung(finden(iprp, b"ipma")?.inhalt, primaer)?;
    let mut masse = None;
    let mut gedreht = false;
    for index in zugeordnet {
        let Some(eigenschaft) = index.checked_sub(1).and_then(|i| eigenschaften.get(i)) else {
            continue;
        };
        match &eigenschaft.typ {
            b"ispe" if masse.is_none() => masse = ispe(eigenschaft.inhalt),
            b"irot" => {
                gedreht = matches!(eigenschaft.inhalt.first().map(|b| b & 0x03), Some(1 | 3))
            }
            _ => {}
        }
    }
    let (breite, hoehe) = masse?;
    let gueltig = breite > 0
        && hoehe > 0
        && breite <= SEITE_HOECHSTENS
        && hoehe <= SEITE_HOECHSTENS
        && u64::from(breite) * u64::from(hoehe) <= PIXEL_HOECHSTENS;
    if !gueltig {
        return None;
    }
    Some(if gedreht {
        (hoehe, breite)
    } else {
        (breite, hoehe)
    })
}

/// Die Box an `stelle` und die Stelle dahinter; ragt sie über den Bereich hinaus, keine.
fn kiste_an(bereich: &[u8], stelle: usize) -> Option<(Kiste<'_>, usize)> {
    let rest = bereich.get(stelle..)?;
    let typ: [u8; 4] = rest.get(4..8)?.try_into().ok()?;
    let (kopf, groesse) = match u32_be(rest, 0)? {
        1 => (16, u64::from_be_bytes(rest.get(8..16)?.try_into().ok()?)),
        0 => (8, rest.len() as u64),
        groesse => (8, u64::from(groesse)),
    };
    if groesse < kopf || groesse > rest.len() as u64 {
        return None;
    }
    let groesse = usize::try_from(groesse).ok()?;
    let inhalt = rest.get(usize::try_from(kopf).ok()?..groesse)?;
    Some((Kiste { typ, inhalt }, stelle + groesse))
}

/// Die erste Kindbox des Typs über die Längenkette; eine Box über den Bereich hinaus vorher → keine.
fn finden<'a>(bereich: &'a [u8], typ: &[u8; 4]) -> Option<Kiste<'a>> {
    let mut stelle = 0;
    while stelle < bereich.len() {
        let (kiste, weiter) = kiste_an(bereich, stelle)?;
        if &kiste.typ == typ {
            return Some(kiste);
        }
        stelle = weiter;
    }
    None
}

/// Alle Kindboxen; jede mindestens 8 Bytes — ihre Zahl ist durch die Länge begrenzt.
fn alle(bereich: &[u8]) -> Option<Vec<Kiste<'_>>> {
    let mut kisten = Vec::new();
    let mut stelle = 0;
    while stelle < bereich.len() {
        let (kiste, weiter) = kiste_an(bereich, stelle)?;
        kisten.push(kiste);
        stelle = weiter;
    }
    Some(kisten)
}

/// `pitm`: Version 0 mit u16, sonst u32.
fn primaerbild(inhalt: &[u8]) -> Option<u32> {
    match inhalt.first()? {
        0 => u16_be(inhalt, 4).map(u32::from),
        _ => u32_be(inhalt, 4),
    }
}

/// `ipma`: die Eigenschaften (1-basiert) des Items in einem Durchgang; `entry_count` höchstens, was die Box trägt.
fn zuordnung(inhalt: &[u8], item: u32) -> Option<Vec<usize>> {
    let version = *inhalt.first()?;
    let breit = inhalt.get(3)? & 1 == 1;
    let anzahl = u64::from(u32_be(inhalt, 4)?);
    let id_laenge: usize = if version < 1 { 2 } else { 4 };
    let index_laenge: usize = if breit { 2 } else { 1 };
    let platz = inhalt.len().checked_sub(8)? as u64;
    if anzahl.checked_mul(id_laenge as u64 + 1)? > platz {
        return None;
    }
    let mut stelle = 8;
    for _ in 0..anzahl {
        let id = if version < 1 {
            u32::from(u16_be(inhalt, stelle)?)
        } else {
            u32_be(inhalt, stelle)?
        };
        let zahl = usize::from(*inhalt.get(stelle + id_laenge)?);
        stelle += id_laenge + 1;
        let ende = stelle.checked_add(zahl.checked_mul(index_laenge)?)?;
        let eintraege = inhalt.get(stelle..ende)?;
        if id == item {
            return Some(
                eintraege
                    .chunks_exact(index_laenge)
                    .map(|index| match index {
                        [hoch, tief] => usize::from(u16::from_be_bytes([*hoch, *tief]) & 0x7fff),
                        [einzeln] => usize::from(einzeln & 0x7f),
                        _ => 0,
                    })
                    .collect(),
            );
        }
        stelle = ende;
    }
    None
}

/// `ispe` (FullBox): Breite und Höhe je u32.
fn ispe(inhalt: &[u8]) -> Option<(u32, u32)> {
    Some((u32_be(inhalt, 4)?, u32_be(inhalt, 8)?))
}

fn u16_be(bytes: &[u8], stelle: usize) -> Option<u16> {
    Some(u16::from_be_bytes(
        bytes.get(stelle..stelle.checked_add(2)?)?.try_into().ok()?,
    ))
}

fn u32_be(bytes: &[u8], stelle: usize) -> Option<u32> {
    Some(u32::from_be_bytes(
        bytes.get(stelle..stelle.checked_add(4)?)?.try_into().ok()?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kiste(typ: &[u8; 4], inhalt: &[u8]) -> Vec<u8> {
        let mut bytes = ((8 + inhalt.len()) as u32).to_be_bytes().to_vec();
        bytes.extend(typ);
        bytes.extend(inhalt);
        bytes
    }

    fn voll(typ: &[u8; 4], version: u8, flags: u32, inhalt: &[u8]) -> Vec<u8> {
        let mut kopf = vec![version];
        kopf.extend(&flags.to_be_bytes()[1..]);
        kopf.extend(inhalt);
        kiste(typ, &kopf)
    }

    fn ispe(breite: u32, hoehe: u32) -> Vec<u8> {
        let mut inhalt = breite.to_be_bytes().to_vec();
        inhalt.extend(hoehe.to_be_bytes());
        voll(b"ispe", 0, 0, &inhalt)
    }

    fn irot(winkel: u8) -> Vec<u8> {
        kiste(b"irot", &[winkel])
    }

    /// `ipma` Version 0, Flags 0: je Item (u16) die Eigenschaften (1-basiert, u8 mit Bit „wesentlich“).
    fn ipma(eintraege: &[(u16, &[u8])]) -> Vec<u8> {
        let mut inhalt = (eintraege.len() as u32).to_be_bytes().to_vec();
        for (item, eigenschaften) in eintraege {
            inhalt.extend(item.to_be_bytes());
            inhalt.push(eigenschaften.len() as u8);
            inhalt.extend(eigenschaften.iter().map(|index| index | 0x80));
        }
        voll(b"ipma", 0, 0, &inhalt)
    }

    /// `ftyp avif`, `meta` mit `hdlr pict`, `pitm`, `iprp` [`ipco` [eigenschaften], `ipma`], dann `mdat`.
    fn avif(primaer: Option<u16>, eigenschaften: &[Vec<u8>], zuordnung: Vec<u8>) -> Vec<u8> {
        let mut meta = voll(b"hdlr", 0, 0, b"\0\0\0\0pict\0\0\0\0\0\0\0\0\0\0\0\0\0");
        if let Some(item) = primaer {
            meta.extend(voll(b"pitm", 0, 0, &item.to_be_bytes()));
        }
        let mut iprp = kiste(b"ipco", &eigenschaften.concat());
        iprp.extend(zuordnung);
        meta.extend(kiste(b"iprp", &iprp));
        let mut bytes = kiste(b"ftyp", b"avif\0\0\0\0avifmif1miaf");
        bytes.extend(voll(b"meta", 0, 0, &meta));
        bytes.extend(kiste(b"mdat", b"AV1-DATEN"));
        bytes
    }

    /// ADR-0047 Ergänzung „Karte #9 Nacharbeit F3 und F4“, Erste Tests (Unit Maße, wörtlich; Mutante „`irot`
    /// ignoriert“).
    #[test]
    fn masse_des_primaerbilds_aus_ispe_und_irot() {
        let einfach = avif(Some(1), &[ispe(4032, 3024)], ipma(&[(1, &[1])]));
        assert_eq!(masse(&einfach), Some((4032, 3024)));

        let gedreht = avif(Some(1), &[ispe(4032, 3024), irot(1)], ipma(&[(1, &[1, 2])]));
        assert_eq!(masse(&gedreht), Some((3024, 4032)));
        let dreiviertel = avif(Some(1), &[ispe(4032, 3024), irot(3)], ipma(&[(1, &[1, 2])]));
        assert_eq!(masse(&dreiviertel), Some((3024, 4032)));
        let halb = avif(Some(1), &[ispe(4032, 3024), irot(2)], ipma(&[(1, &[1, 2])]));
        assert_eq!(masse(&halb), Some((4032, 3024)));
        let fremd_gedreht = avif(
            Some(1),
            &[ispe(4032, 3024), irot(1)],
            ipma(&[(2, &[2]), (1, &[1])]),
        );
        assert_eq!(
            masse(&fremd_gedreht),
            Some((4032, 3024)),
            "irot gehört Item 2"
        );

        let zwei = avif(
            Some(1),
            &[ispe(256, 192), ispe(4032, 3024)],
            ipma(&[(2, &[1]), (1, &[2])]),
        );
        assert_eq!(masse(&zwei), Some((4032, 3024)));

        let mut riesig = 0xFFFF_FFFFu32.to_be_bytes().to_vec();
        riesig.extend([0u8; 20]);
        let riesig = avif(Some(1), &[ispe(4032, 3024)], voll(b"ipma", 0, 0, &riesig));
        assert_eq!(riesig.windows(4).filter(|w| *w == b"ipma").count(), 1);
        let beginn = std::time::Instant::now();
        assert_eq!(masse(&riesig), None);
        assert!(
            beginn.elapsed() <= std::time::Duration::from_millis(10),
            "{:?}",
            beginn.elapsed()
        );

        assert_eq!(
            masse(&avif(Some(1), &[ispe(0, 3024)], ipma(&[(1, &[1])]))),
            None
        );
        assert_eq!(
            masse(&avif(None, &[ispe(4032, 3024)], ipma(&[(1, &[1])]))),
            None,
            "ohne pitm"
        );
        assert_eq!(
            masse(&avif(Some(1), &[ispe(4032, 3024)], ipma(&[(2, &[1])]))),
            None,
            "nicht zugeordnet"
        );
        assert_eq!(
            masse(&avif(Some(1), &[ispe(16_385, 100)], ipma(&[(1, &[1])]))),
            None,
            "Seite"
        );
        assert_eq!(
            masse(&avif(Some(1), &[ispe(10_000, 5_001)], ipma(&[(1, &[1])]))),
            None,
            "Megapixel"
        );
        assert_eq!(
            masse(&avif(Some(1), &[ispe(10_000, 5_000)], ipma(&[(1, &[1])]))),
            Some((10_000, 5_000))
        );
    }

    /// `meta` über das Fenster hinaus, eine Box über ihre Elternbox hinaus, abgeschnitten — kein Maß.
    #[test]
    fn kein_mass_ueber_fenster_oder_elternbox_hinaus() {
        let mut gross = avif(Some(1), &[ispe(4032, 3024)], ipma(&[(1, &[1])]));
        let meta = gross.windows(4).position(|w| w == b"meta").expect("meta") - 4;
        let laenge = u32::from_be_bytes(gross[meta..meta + 4].try_into().expect("Länge"));
        gross[meta..meta + 4].copy_from_slice(&(laenge + 70_000).to_be_bytes());
        assert_eq!(masse(&gross), None);

        let einfach = avif(Some(1), &[ispe(4032, 3024)], ipma(&[(1, &[1])]));
        for laenge in 0..einfach.len() - 17 {
            assert_eq!(masse(&einfach[..laenge]), None, "{laenge}");
        }
        let mut ueber = einfach.clone();
        let ispe_an = ueber.windows(4).position(|w| w == b"ispe").expect("ispe") - 4;
        ueber[ispe_an..ispe_an + 4].copy_from_slice(&1_000u32.to_be_bytes());
        assert_eq!(masse(&ueber), None);
    }
}
