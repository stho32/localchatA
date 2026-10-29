//! Bereich lesen (B0444, ADR-0047 Regel 4, Frage 6, Ergänzung Karte #9 Gate Regel 5): genau ein Bereich
//! `bytes=a-b`, `a-` oder `-n`, sonst die ganze Datei; nicht erfüllbar → 416. Rein.

/// Was ein Abruf liefert.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bereich {
    Ganz,
    /// Erstes und letztes Byte, beide einschließlich.
    Teil(u64, u64),
    Unerfuellbar,
}

/// `Range`-Kopfzeilen (alle, wie sie kamen), `If-Range`, ETag der Datei (`"<id>"`) und Länge → Bereich.
pub fn bereich_lesen(range: &[&str], if_range: Option<&str>, etag: &str, laenge: u64) -> Bereich {
    let [angabe] = range else {
        return Bereich::Ganz;
    };
    if if_range.is_some_and(|wert| wert != etag) {
        return Bereich::Ganz;
    }
    let Some(spanne) = angabe.strip_prefix("bytes=") else {
        return Bereich::Ganz;
    };
    let Some((von, bis)) = spanne.split_once('-') else {
        return Bereich::Ganz;
    };
    match (zahl(von), zahl(bis)) {
        (Some(Some(von)), Some(None)) => ab(von, laenge),
        (Some(Some(von)), Some(Some(bis))) if bis >= von => ab(von, laenge).gekappt(bis),
        (Some(None), Some(Some(letzte))) => letzte_bytes(letzte, laenge),
        _ => Bereich::Ganz,
    }
}

/// `Some(None)` für leer, `Some(Some(n))` für Ziffern ohne Überlauf, `None` sonst (Leerraum, Zeichen, Überlauf).
fn zahl(text: &str) -> Option<Option<u64>> {
    if text.is_empty() {
        return Some(None);
    }
    if !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse().ok().map(Some)
}

fn ab(von: u64, laenge: u64) -> Bereich {
    if von >= laenge {
        Bereich::Unerfuellbar
    } else {
        Bereich::Teil(von, laenge - 1)
    }
}

fn letzte_bytes(anzahl: u64, laenge: u64) -> Bereich {
    if anzahl == 0 || laenge == 0 {
        return Bereich::Unerfuellbar;
    }
    Bereich::Teil(laenge.saturating_sub(anzahl), laenge - 1)
}

impl Bereich {
    fn gekappt(self, bis: u64) -> Bereich {
        match self {
            Bereich::Teil(von, letztes) => Bereich::Teil(von, bis.min(letztes)),
            anders => anders,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ETAG: &str = "\"aaaaaaaaaaaaaaaaaaaaaaaaaa\"";

    fn lesen(angabe: &str) -> Bereich {
        bereich_lesen(&[angabe], None, ETAG, 1_000)
    }

    #[test]
    fn die_tabelle_aus_der_adr() {
        for (angabe, erwartet) in [
            ("bytes=0-0", Bereich::Teil(0, 0)),
            ("bytes=0-", Bereich::Teil(0, 999)),
            ("bytes=-1", Bereich::Teil(999, 999)),
            ("bytes=-0", Bereich::Unerfuellbar),
            ("bytes=5-2", Bereich::Ganz),
            ("bytes=1000-", Bereich::Unerfuellbar),
            ("bytes=0-1010", Bereich::Teil(0, 999)),
            ("bytes=18446744073709551615-", Bereich::Unerfuellbar),
            ("bytes=18446744073709551616-", Bereich::Ganz),
            ("bytes=0-1,2-3", Bereich::Ganz),
            ("items=0-1", Bereich::Ganz),
            ("Bytes=0-1", Bereich::Ganz),
            (" bytes=0-1", Bereich::Ganz),
            ("bytes= 0-1", Bereich::Ganz),
            ("bytes=0 -1", Bereich::Ganz),
            ("bytes=-1010", Bereich::Teil(0, 999)),
            ("bytes=100-199", Bereich::Teil(100, 199)),
            ("bytes=-", Bereich::Ganz),
            ("bytes=", Bereich::Ganz),
            ("bytes=+5-6", Bereich::Ganz),
        ] {
            assert_eq!(lesen(angabe), erwartet, "{angabe}");
        }
    }

    #[test]
    fn zwei_range_kopfzeilen_oder_keine_ergeben_die_ganze_datei() {
        assert_eq!(
            bereich_lesen(&["bytes=0-1", "bytes=0-1"], None, ETAG, 1_000),
            Bereich::Ganz
        );
        assert_eq!(bereich_lesen(&[], None, ETAG, 1_000), Bereich::Ganz);
    }

    #[test]
    fn if_range_gilt_nur_mit_dem_starken_etag() {
        assert_eq!(
            bereich_lesen(&["bytes=0-9"], Some(ETAG), ETAG, 1_000),
            Bereich::Teil(0, 9)
        );
        for anders in [
            "W/\"aaaaaaaaaaaaaaaaaaaaaaaaaa\"",
            "Wed, 21 Oct 2026 07:28:00 GMT",
            "\"anders\"",
        ] {
            assert_eq!(
                bereich_lesen(&["bytes=0-9"], Some(anders), ETAG, 1_000),
                Bereich::Ganz,
                "{anders}"
            );
        }
    }

    #[test]
    fn eine_leere_datei_hat_keinen_erfuellbaren_bereich() {
        assert_eq!(
            bereich_lesen(&["bytes=0-"], None, ETAG, 0),
            Bereich::Unerfuellbar
        );
        assert_eq!(
            bereich_lesen(&["bytes=-5"], None, ETAG, 0),
            Bereich::Unerfuellbar
        );
    }
}
