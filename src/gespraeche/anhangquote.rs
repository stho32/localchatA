//! Anhang-Quote (ADR-0046 Ergänzungen „Karte #9 Betreiber F1“ Frage D, „F1′“ Regeln 1, 2): eigene Quote für alle
//! Anhänge zusammen, getrennt von der Plattenquote des Textes. Rein.

/// Mindestmaß eines Anhangs an der Quote.
pub const MINDESTMASS: u64 = 65_536;
/// Blockgröße des Zählmaßes.
pub const BLOCK: u64 = 4_096;
/// Abstand zum vollen Dateisystem, den jede Anmeldung lässt (ADR-0046 Regel 1.5).
pub const PLATTE_ABSTAND: u64 = 1_073_741_824;

/// Zählmaß `z(n) = max(65.536, ⌈n / 4.096⌉ × 4.096)` — so zählt ein Anhang an der Anhang-Quote.
pub fn zaehlmass(bytes: u64) -> u64 {
    bytes.div_ceil(BLOCK).saturating_mul(BLOCK).max(MINDESTMASS)
}

/// Die Zahlen einer 507 `quote` (ADR-0046 Ergänzung „F1′“ Regel 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quotestand {
    /// `G + A`.
    pub belegt: u64,
    pub gebunden: u64,
    pub reserviert: u64,
    pub hoechstens: u64,
    /// `Q − G − A`, nie unter 0.
    pub frei: u64,
    /// `z(bytes)` der Anmeldung.
    pub braucht: u64,
}

/// Warum eine Anmeldung keinen Platz bekommt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platzfehler {
    /// `z > Q` — größer als der ganze Platz für Anhänge (422 `zu_gross`).
    ZuGross { hoechstens: u64 },
    /// `z + offen + 1 GiB > frei` der Platte (507 `platte`), auch ohne Anhang-Quote.
    Platte { frei: u64 },
    /// `G + z > Q` — passt selbst ohne laufende Hochladungen nicht (507 `voll`, ohne `Retry-After`).
    Voll(Quotestand),
    /// `G + A + z > Q` — laufende Hochladungen belegen den Platz (507 `hochladungen`, `Retry-After: 30`).
    Hochladungen(Quotestand),
}

/// Vor allem anderen und ohne Sperre: ist die Datei größer als die ganze Anhang-Quote?
pub fn zu_gross(bytes: u64, hoechstens: Option<u64>) -> Result<(), Platzfehler> {
    match hoechstens {
        Some(hoechstens) if zaehlmass(bytes) > hoechstens => {
            Err(Platzfehler::ZuGross { hoechstens })
        }
        _ => Ok(()),
    }
}

/// Unter der Schreibsperre: Platte, dann die Anhang-Quote. `offen` = noch zu schreibende Bytes des Eingangs im
/// Zählmaß. Ohne Anhang-Quote prüft nur die Platte. Liefert den Anspruch `z(bytes)`.
pub fn platz_pruefen(
    gebunden: u64,
    reserviert: u64,
    hoechstens: Option<u64>,
    bytes: u64,
    frei_platte: u64,
    offen: u64,
) -> Result<u64, Platzfehler> {
    let z = zaehlmass(bytes);
    if z.saturating_add(offen).saturating_add(PLATTE_ABSTAND) > frei_platte {
        return Err(Platzfehler::Platte { frei: frei_platte });
    }
    let Some(hoechstens) = hoechstens else {
        return Ok(z);
    };
    let stand = Quotestand {
        belegt: gebunden.saturating_add(reserviert),
        gebunden,
        reserviert,
        hoechstens,
        frei: hoechstens.saturating_sub(gebunden.saturating_add(reserviert)),
        braucht: z,
    };
    if gebunden.saturating_add(z) > hoechstens {
        return Err(Platzfehler::Voll(stand));
    }
    if gebunden.saturating_add(reserviert).saturating_add(z) > hoechstens {
        return Err(Platzfehler::Hochladungen(stand));
    }
    Ok(z)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIB: u64 = 1_048_576;
    const Q: u64 = 10 * MIB;
    const VIEL_PLATZ: u64 = u64::MAX;

    /// ADR-0046 Ergänzung „F1′“, Erste Tests (Teil Anmelden): Q = 10 MiB, G = 9 MiB — 11 MiB → `zu_gross`,
    /// 2 MiB → `voll`, 1 MiB → ok, danach mit A = 1 MiB 64 KiB → `hochladungen`.
    #[test]
    fn rechenbeispiel_der_vollen_anhang_quote() {
        let g = 9 * MIB;
        assert_eq!(
            zu_gross(11 * MIB, Some(Q)),
            Err(Platzfehler::ZuGross { hoechstens: Q })
        );
        assert_eq!(
            platz_pruefen(g, 0, Some(Q), 2 * MIB, VIEL_PLATZ, 0),
            Err(Platzfehler::Voll(Quotestand {
                belegt: 9_437_184,
                gebunden: 9_437_184,
                reserviert: 0,
                hoechstens: 10_485_760,
                frei: 1_048_576,
                braucht: 2_097_152,
            }))
        );
        assert_eq!(platz_pruefen(g, 0, Some(Q), MIB, VIEL_PLATZ, 0), Ok(MIB));
        assert_eq!(
            platz_pruefen(g, MIB, Some(Q), 1, VIEL_PLATZ, 0),
            Err(Platzfehler::Hochladungen(Quotestand {
                belegt: 10_485_760,
                gebunden: 9_437_184,
                reserviert: 1_048_576,
                hoechstens: 10_485_760,
                frei: 0,
                braucht: 65_536,
            }))
        );
    }

    /// Grenzen gehören dazu (Mutante „`>=` statt `>`“): G + z = Q passt, ebenso G + A + z = Q; z = Q passt auf
    /// leerer Quote, Q + 1 Byte nicht (z(10.485.761) = 10.489.856).
    #[test]
    fn genau_an_der_quote_passt_es() {
        assert_eq!(
            platz_pruefen(9 * MIB, 0, Some(Q), MIB, VIEL_PLATZ, 0),
            Ok(MIB)
        );
        assert_eq!(
            platz_pruefen(8 * MIB, MIB, Some(Q), MIB, VIEL_PLATZ, 0),
            Ok(MIB)
        );
        assert_eq!(platz_pruefen(0, 0, Some(Q), Q, VIEL_PLATZ, 0), Ok(Q));
        assert_eq!(zu_gross(Q, Some(Q)), Ok(()));
        assert_eq!(
            zu_gross(Q + 1, Some(Q)),
            Err(Platzfehler::ZuGross { hoechstens: Q })
        );
    }

    /// Das Zählmaß zählt auch kleine Dateien mit 64 KiB (Mutante „Zählmaß ohne Minimum“): bei G = Q − 65.535
    /// passt ein Byte nicht.
    #[test]
    fn ein_byte_zaehlt_mit_64_kib() {
        assert!(matches!(
            platz_pruefen(Q - 65_535, 0, Some(Q), 1, VIEL_PLATZ, 0),
            Err(Platzfehler::Voll(_))
        ));
        assert_eq!(
            platz_pruefen(Q - 65_536, 0, Some(Q), 1, VIEL_PLATZ, 0),
            Ok(65_536)
        );
    }

    /// Platte: `z + offen + 1 GiB > frei` → `platte`, an der Grenze ok — auch ohne Anhang-Quote.
    #[test]
    fn die_platte_behaelt_1_gib_abstand() {
        let z = zaehlmass(307_200);
        let offen = 5 * MIB;
        let grenze = z + offen + PLATTE_ABSTAND;
        assert_eq!(
            platz_pruefen(0, 0, None, 307_200, grenze - 1, offen),
            Err(Platzfehler::Platte { frei: grenze - 1 })
        );
        assert_eq!(platz_pruefen(0, 0, None, 307_200, grenze, offen), Ok(z));
        assert_eq!(
            platz_pruefen(0, 0, Some(Q), 307_200, grenze - 1, offen),
            Err(Platzfehler::Platte { frei: grenze - 1 })
        );
    }

    /// Ohne Anhang-Quote: kein `zu_gross`, kein `quote` — nur die Platte.
    #[test]
    fn ohne_anhang_quote_prueft_nur_die_platte() {
        assert_eq!(zu_gross(u64::MAX, None), Ok(()));
        assert_eq!(
            platz_pruefen(u64::MAX, u64::MAX, None, Q + 1, VIEL_PLATZ, 0),
            Ok(10_489_856)
        );
    }
}
