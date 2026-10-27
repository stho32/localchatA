//! Zufall (Aspekt X00009): der einzige Ort, an dem Zufall entsteht — Sitzungs-Tokens mit 128 Bit aus dem
//! Zufall des Betriebssystems (ADR-0012), im Test eine feste Folge.

use std::fmt;

/// 128 Bit als 32 Zeichen `[0-9a-f]`. `Debug` zeigt den Wert nicht — ein Token gehört in keine Ausgabe.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Token(String);

/// Das Betriebssystem liefert keinen Zufall; ohne ihn gibt es kein Token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Zufallsfehler(pub String);

/// Woher Tokens kommen: das Betriebssystem oder im Test eine feste Folge.
pub trait Zufall: Send + Sync {
    fn token(&self) -> Result<Token, Zufallsfehler>;
}

impl Token {
    /// Ein Token aus einem Header: genau 32 Zeichen `[0-9a-f]`, sonst keines.
    pub fn lesen(text: &str) -> Option<Token> {
        let hex = |zeichen: u8| zeichen.is_ascii_digit() || (b'a'..=b'f').contains(&zeichen);
        let gueltig = text.len() == 32 && text.bytes().all(hex);
        gueltig.then(|| Token(text.to_string()))
    }

    pub fn als_text(&self) -> &str {
        &self.0
    }

    fn aus_bytes(bytes: [u8; 16]) -> Token {
        Token(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
    }
}

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Token(…)")
    }
}

/// Zufall des Betriebssystems über `getrandom`.
pub struct Betriebssystemzufall;

impl Zufall for Betriebssystemzufall {
    fn token(&self) -> Result<Token, Zufallsfehler> {
        let mut bytes = [0u8; 16];
        getrandom::fill(&mut bytes).map_err(|fehler| Zufallsfehler(fehler.to_string()))?;
        Ok(Token::aus_bytes(bytes))
    }
}

/// Feste Folge für Tests: liefert die Zahlen der Reihe nach als Token, danach einen Zufallsfehler.
#[cfg(test)]
pub struct FesteFolge(std::sync::Mutex<std::collections::VecDeque<u128>>);

#[cfg(test)]
impl FesteFolge {
    pub fn neu(zahlen: impl IntoIterator<Item = u128>) -> FesteFolge {
        FesteFolge(std::sync::Mutex::new(zahlen.into_iter().collect()))
    }
}

#[cfg(test)]
impl Zufall for FesteFolge {
    fn token(&self) -> Result<Token, Zufallsfehler> {
        let mut folge = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match folge.pop_front() {
            Some(zahl) => Ok(Token::aus_bytes(zahl.to_be_bytes())),
            None => Err(Zufallsfehler("feste Folge erschöpft".into())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn ist_hex32(text: &str) -> bool {
        text.len() == 32
            && text
                .bytes()
                .all(|zeichen| matches!(zeichen, b'0'..=b'9' | b'a'..=b'f'))
    }

    /// Probe für `getrandom` (Skill dependency-probe): 1000 Tokens, alle 32 Hex-Zeichen, alle verschieden,
    /// und jedes der 128 Bits ist mal 0, mal 1 — ein konstantes oder halb leeres Ergebnis fiele auf.
    #[test]
    fn tokens_aus_dem_betriebssystem_sind_32_hex_zeichen_und_verschieden() {
        let tokens: Vec<Token> = (0..1000)
            .map(|_| Betriebssystemzufall.token().expect("Zufall"))
            .collect();

        assert!(tokens.iter().all(|token| ist_hex32(token.als_text())));
        let verschieden: HashSet<&Token> = tokens.iter().collect();
        assert_eq!(verschieden.len(), 1000);
        let bits = |token: &Token| u128::from_str_radix(token.als_text(), 16).expect("Hex");
        let oder = tokens
            .iter()
            .fold(0u128, |summe, token| summe | bits(token));
        let und = tokens
            .iter()
            .fold(u128::MAX, |summe, token| summe & bits(token));
        assert_eq!((oder, und), (u128::MAX, 0));
    }

    #[test]
    fn die_feste_folge_liefert_ihre_zahlen_als_token_und_dann_einen_fehler() {
        let folge = FesteFolge::neu([1, 0xabc]);

        assert_eq!(
            folge.token().map(|token| token.als_text().to_string()),
            Ok(format!("{:032x}", 1))
        );
        assert_eq!(
            folge.token().map(|token| token.als_text().to_string()),
            Ok(format!("{:032x}", 0xabc))
        );
        assert!(folge.token().is_err());
    }

    #[test]
    fn ein_token_aus_einem_header_ist_genau_32_kleine_hex_zeichen() {
        let gut = "0123456789abcdef0123456789abcdef";
        assert_eq!(
            Token::lesen(gut).map(|token| token.als_text().to_string()),
            Some(gut.into())
        );
        for schlecht in [
            "",
            "0123456789ABCDEF0123456789abcdef",
            "0123456789abcdef0123456789abcde",
            "0123456789abcdef0123456789abcdef0",
            "0123456789abcdef0123456789abcdeg",
            " 123456789abcdef0123456789abcdef",
        ] {
            assert_eq!(Token::lesen(schlecht), None, "{schlecht:?}");
        }
    }

    #[test]
    fn debug_zeigt_den_wert_nicht() {
        let token = Token::lesen("0123456789abcdef0123456789abcdef").expect("Token");
        assert_eq!(format!("{token:?}"), "Token(…)");
    }
}
