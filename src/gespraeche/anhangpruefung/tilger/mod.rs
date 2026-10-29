//! Ortsangaben beim Hochladen tilgen (F0115, A00018 Ergänzung F4, ADR-0047 Ergänzung „Karte #9 Betreiber F4“): eigene
//! Leser nach Struktur, an Ort und Stelle bei gleicher Länge, über den Positionsport.

pub mod bild;
pub mod isobmff;
pub mod minimal;
pub mod mpf;
pub mod port;

use crate::gespraeche::anhangablage::Teildatei;
use port::{Abbruch, Ergebnis, Port};

/// Höchstens so viele gedeutete Blöcke je Datei (Unterblöcke eines GIF und `IDAT`/`fdAT` zählen nie).
pub const HOECHSTENS_BLOECKE: u64 = 100_000;

/// Was der Tilger gedeutet hat: Zahl der Blöcke und ob ein Ort (GPS-IFD, Ortsbox) dabei war.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Zaehler {
    pub bloecke: u64,
    pub ort: bool,
}

/// Was getilgt wurde — fürs Protokoll, nie mit Inhalt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Getilgt {
    pub art: &'static str,
    pub bloecke: u64,
    pub ort: bool,
}

/// Tilgt eine fertige Teildatei nach ihrer Struktur (nicht nach der Art des Erkenners): JPEG samt MPF-Bildern, PNG,
/// WebP, GIF, ISOBMFF; alles andere bleibt unverändert (`None`). Die Länge wird danach geprüft.
pub fn tilgen(datei: &dyn Teildatei) -> Ergebnis<Option<Getilgt>> {
    let port = Port::neu(datei)?;
    let anfang = port.lesen(0, port.laenge().min(16))?;
    let mut zaehler = Zaehler::default();
    let art = if anfang.starts_with(&[0xFF, 0xD8, 0xFF]) {
        mpf::jpeg_mit_mpf(&port, &mut zaehler)?;
        "jpeg"
    } else if anfang.starts_with(b"\x89PNG\r\n\x1a\n") {
        bild::png(&port, &mut zaehler)?;
        "png"
    } else if anfang.starts_with(b"RIFF") && anfang.get(8..12) == Some(b"WEBP") {
        bild::webp(&port, &mut zaehler)?;
        "webp"
    } else if anfang.starts_with(b"GIF87a") || anfang.starts_with(b"GIF89a") {
        bild::gif(&port, &mut zaehler)?;
        "gif"
    } else if isobmff::ist_isobmff(&anfang) {
        isobmff::isobmff(&port, &mut zaehler)?;
        "isobmff"
    } else {
        return Ok(None);
    };
    if datei.laenge()? != port.laenge() {
        return Err(Abbruch::Platte(std::io::Error::other(
            "Länge beim Tilgen geändert",
        )));
    }
    Ok(Some(Getilgt {
        art,
        bloecke: zaehler.bloecke,
        ort: zaehler.ort,
    }))
}

impl Zaehler {
    /// Ein gedeuteter Block mehr; über 100.000 → `metadaten`.
    pub fn block(&mut self) -> Ergebnis<()> {
        self.bloecke += 1;
        if self.bloecke > HOECHSTENS_BLOECKE {
            return Err(Abbruch::Metadaten("über 100.000 Blöcke"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::port::attrappe::Datei;
    use super::*;

    #[test]
    fn andere_formate_bleiben_unveraendert() {
        for inhalt in [
            &b"%PDF-1.7 GPS"[..],
            b"\x1aE\xdf\xa3webm",
            b"OggS",
            b"ID3",
            b"Text mit GPS",
            b"",
        ] {
            let datei = Datei::mit(inhalt.to_vec());
            assert_eq!(tilgen(&datei).expect("kein Fehler"), None);
            assert_eq!(datei.inhalt(), inhalt);
        }
    }

    #[test]
    fn ein_beschaedigtes_jpeg_ist_nicht_tilgbar() {
        let datei = Datei::mit(b"\xFF\xD8\xFF\xE1\xFF\xFFExif".to_vec());
        assert!(matches!(tilgen(&datei), Err(Abbruch::Metadaten(_))));
    }
}
