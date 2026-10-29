//! Text über Stückgrenzen (B0435, ADR-0047 Frage 3): gültiges UTF-8 über die ganze Datei mit Übertrag an den
//! Stückgrenzen, kein NUL; über 16 MiB `attachment`. Rein — das Urteil hängt nicht an der Stückelung.

/// Bis zu dieser Größe wird Text inline geöffnet (Betreiber F3).
pub const TEXT_INLINE_HOECHSTENS: u64 = 16 * 1_048_576;

/// Prüft Stück für Stück, ob die Datei Text ist.
#[derive(Debug, Clone, Default)]
pub struct Textpruefer {
    /// Ein unvollständiges Zeichen am Ende des letzten Stücks (höchstens 3 Bytes).
    uebertrag: Vec<u8>,
    kein_text: bool,
}

impl Textpruefer {
    pub fn stueck(&mut self, bytes: &[u8]) {
        if self.kein_text {
            return;
        }
        if bytes.contains(&0) {
            self.kein_text = true;
            return;
        }
        let mut rest = bytes;
        if !self.uebertrag.is_empty() {
            let brauche = (4 - self.uebertrag.len()).min(rest.len());
            let mut probe = self.uebertrag.clone();
            probe.extend_from_slice(&rest[..brauche]);
            match std::str::from_utf8(&probe) {
                Ok(_) => {
                    self.uebertrag.clear();
                    rest = &rest[brauche..];
                }
                Err(fehler) if fehler.valid_up_to() > 0 => {
                    let verbraucht = fehler.valid_up_to() - self.uebertrag.len();
                    self.uebertrag.clear();
                    rest = &rest[verbraucht..];
                }
                Err(fehler) if fehler.error_len().is_none() => {
                    self.uebertrag = probe;
                    return;
                }
                Err(_) => {
                    self.kein_text = true;
                    return;
                }
            }
        }
        match std::str::from_utf8(rest) {
            Ok(_) => {}
            Err(fehler) if fehler.error_len().is_none() => {
                self.uebertrag = rest[fehler.valid_up_to()..].to_vec();
            }
            Err(_) => self.kein_text = true,
        }
    }

    /// Text, wenn alles gültiges UTF-8 ohne NUL war und kein Zeichen offen blieb.
    pub fn ist_text(&self) -> bool {
        !self.kein_text && self.uebertrag.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn urteil(bytes: &[u8], stueckgroesse: usize) -> bool {
        let mut pruefer = Textpruefer::default();
        for stueck in bytes.chunks(stueckgroesse.max(1)) {
            pruefer.stueck(stueck);
        }
        pruefer.ist_text()
    }

    /// Dieselbe Datei in Stücken zu 1, 3 und 65.536 Bytes → gleiches Urteil, auch mit `ä` und `😀` über einer
    /// Stückgrenze und mit ungültigem UTF-8 genau an der Grenze.
    #[test]
    fn das_urteil_haengt_nicht_an_der_stueckelung() {
        let text = "Grüße 😀 aus dem Text — mit Umlauten äöü\n".repeat(3_000);
        let mut kaputt_an_grenze = vec![b'a'; 65_535];
        kaputt_an_grenze.push(0xc3);
        kaputt_an_grenze.push(b'x');
        let mut offen_am_ende = text.as_bytes().to_vec();
        offen_am_ende.extend([0xf0, 0x9f, 0x98]);
        for groesse in [1, 2, 3, 4, 7, 65_536] {
            assert!(urteil(text.as_bytes(), groesse), "{groesse}");
            assert!(!urteil(&kaputt_an_grenze, groesse), "{groesse}");
            assert!(!urteil(&offen_am_ende, groesse), "{groesse}");
            assert!(!urteil(b"a\0b", groesse), "{groesse}");
            assert!(!urteil(&[0xff, b'a'], groesse), "{groesse}");
        }
        assert!(urteil(b"", 1));
    }

    #[test]
    fn die_grenze_fuer_inline_ist_16_mib() {
        assert_eq!(TEXT_INLINE_HOECHSTENS, 16_777_216);
    }
}
