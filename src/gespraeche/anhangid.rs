//! Anhang-ID (ADR-0047 Frage 2, Z1): 128 Bit Zufall als 26 Zeichen Base32 klein (`[a-z2-7]`) — zugleich die
//! Berechtigung zum Öffnen. `Debug` und `Display` zeigen nur die ersten 6 Zeichen: die ID gehört wie ein Token in
//! keine Ausgabe (ADR-0047 Regel 8).

use crate::zufall::Token;
use std::fmt;

/// Länge einer Anhang-ID in Zeichen.
pub const ID_ZEICHEN: usize = 26;
/// So viele Zeichen einer ID dürfen im Protokoll stehen (ADR-0047 Ergänzung Karte #9 Gate Regel 2).
pub const PRAEFIX_ZEICHEN: usize = 6;

const ALPHABET: &[u8; 32] = b"abcdefghijklmnopqrstuvwxyz234567";

#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Anhangid(String);

impl Anhangid {
    /// Genau 26 Zeichen aus `[a-z2-7]` — Großbuchstaben, andere Längen und Zeichen sind keine ID.
    pub fn lesen(text: &str) -> Option<Anhangid> {
        let gueltig = text.len() == ID_ZEICHEN && text.bytes().all(|byte| ALPHABET.contains(&byte));
        gueltig.then(|| Anhangid(text.to_string()))
    }

    /// Aus den 128 Bit eines Tokens des Zufalls (32 Hex-Zeichen), Base32 ohne Auffüllen.
    pub fn aus_token(token: &Token) -> Option<Anhangid> {
        let zahl = u128::from_str_radix(token.als_text(), 16).ok()?;
        let mut text = String::with_capacity(ID_ZEICHEN);
        for stelle in 0..ID_ZEICHEN {
            let verschiebung = 128_i32 - 5 * (stelle as i32 + 1);
            let wert = if verschiebung >= 0 {
                (zahl >> verschiebung) & 0x1f
            } else {
                (zahl << -verschiebung) & 0x1f
            };
            text.push(char::from(ALPHABET[wert as usize]));
        }
        Some(Anhangid(text))
    }

    pub fn als_text(&self) -> &str {
        &self.0
    }

    /// Die ersten 6 Zeichen — so darf eine Datei des Eingangs im Protokoll stehen.
    pub fn praefix(&self) -> &str {
        &self.0[..PRAEFIX_ZEICHEN]
    }
}

/// Ein Name, wie er im Protokoll stehen darf (ADR-0047 Ergänzung Karte #9 Gate Regel 2): beginnt er mit 26 Zeichen
/// aus `[A-Za-z2-7]` — eine ID, auch groß geschrieben —, nur die ersten 6 Zeichen und `…`; sonst unverändert.
pub fn im_protokoll(name: &str) -> String {
    let beginnt_mit_id = name.len() >= ID_ZEICHEN
        && name.as_bytes()[..ID_ZEICHEN]
            .iter()
            .all(|byte| byte.is_ascii_alphabetic() || (b'2'..=b'7').contains(byte));
    if beginnt_mit_id {
        format!("{}…", &name[..PRAEFIX_ZEICHEN])
    } else {
        name.to_string()
    }
}

impl fmt::Debug for Anhangid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Anhangid({}…)", self.praefix())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zufall::{FesteFolge, Zufall};

    #[test]
    fn eine_id_ist_genau_26_zeichen_aus_a_bis_z_und_2_bis_7() {
        let gut = "k3v7q2m5xpc4h6n6tewaz2b5yr";
        assert_eq!(
            Anhangid::lesen(gut).map(|id| id.als_text().to_string()),
            Some(gut.into())
        );
        for schlecht in [
            "",
            "k3v7q2m5xpc4h6n6tewaz2b5y",
            "k3v7q2m5xpc4h6n6tewaz2b5yra",
            "K3v7q2m5xpc4h6n6tewaz2b5yr",
            "k3v7q2m5xpc4h6n6tewaz2b5y1",
            "k3v7q2m5xpc4h6n6tewaz2b5y8",
            "k3v7q2m5xpc4h6n6tewaz2b5y.",
            "k3v7q2m5xpc4h6n6tewaz2b5ÿ",
        ] {
            assert_eq!(Anhangid::lesen(schlecht), None, "{schlecht:?}");
        }
    }

    /// Base32 der 128 Bit: 0 → nur `a`, alle Bits 1 → `7` und am Ende die drei restlichen Bits mit zwei Nullen (`11100` = `4`).
    #[test]
    fn aus_dem_zufall_entsteht_base32_ohne_auffuellen() {
        let folge = FesteFolge::neu([0, u128::MAX, 1]);
        let id = |folge: &FesteFolge| {
            Anhangid::aus_token(&folge.token().expect("Zufall"))
                .expect("ID")
                .als_text()
                .to_string()
        };
        assert_eq!(id(&folge), "a".repeat(26));
        assert_eq!(id(&folge), format!("{}4", "7".repeat(25)));
        assert_eq!(id(&folge), format!("{}e", "a".repeat(25)));
        for _ in 0..100 {
            let token = crate::zufall::Betriebssystemzufall.token().expect("Zufall");
            let text = Anhangid::aus_token(&token)
                .expect("ID")
                .als_text()
                .to_string();
            assert!(Anhangid::lesen(&text).is_some(), "{text}");
        }
    }

    /// Eine ID am Anfang eines Namens wird gekürzt, auch groß geschrieben und mit Endung; andere Namen bleiben.
    #[test]
    fn im_protokoll_steht_von_einer_id_nur_der_praefix() {
        assert_eq!(im_protokoll("k3v7q2m5xpc4h6n6tewaz2b5yr"), "k3v7q2…");
        assert_eq!(im_protokoll("K3V7Q2M5XPC4H6N6TEWAZ2B5YR.teil"), "K3V7Q2…");
        assert_eq!(im_protokoll("unter"), "unter");
        assert_eq!(
            im_protokoll("k3v7q2m5xpc4h6n6tewaz2b5y"),
            "k3v7q2m5xpc4h6n6tewaz2b5y"
        );
        assert_eq!(
            im_protokoll("k3v7q2m5xpc4h6n6tewaz2b5y1"),
            "k3v7q2m5xpc4h6n6tewaz2b5y1"
        );
    }

    #[test]
    fn debug_zeigt_nur_den_praefix() {
        let id = Anhangid::lesen("k3v7q2m5xpc4h6n6tewaz2b5yr").expect("ID");
        assert_eq!(format!("{id:?}"), "Anhangid(k3v7q2…)");
        assert_eq!(id.praefix(), "k3v7q2");
    }
}
