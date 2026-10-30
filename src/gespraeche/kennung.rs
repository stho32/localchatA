//! Kennung `name@herkunft` des Absenders — geprüft beim Bilden, damit das Logformat sich darauf verlassen kann.

use crate::fehler::Fehler;
use crate::name::Name;
use std::fmt;
use std::net::IpAddr;

/// Name nach der Namensregel (ADR-0013), genau ein `@`, Herkunft als Textform einer IP-Adresse
/// (`[0-9a-f.:]`, 1 bis 45 Zeichen).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Kennung(String);

impl Kennung {
    pub fn bilden(text: &str) -> Result<Kennung, Fehler> {
        let gueltig = match text.split_once('@') {
            Some((name, herkunft)) => Name::bilden(name).is_ok() && herkunft_gueltig(herkunft),
            None => false,
        };
        if gueltig {
            Ok(Kennung(text.to_string()))
        } else {
            Err(Fehler::Ungueltig(format!(
                "Kennung {text:?} ungültig — erwartet name@herkunft, Name 1 bis 32 Zeichen aus a–z, 0–9 und -, Herkunft eine IP-Adresse"
            )))
        }
    }

    /// Die kanonische Kennung für Pfad und Ordner (ADR-0055 Regel 2): `bilden` und die Herkunft als Adresse
    /// gelesen, in ihre kanonische Form gebracht und wieder als Text **byte-gleich** dem Eingang. Abgewiesen,
    /// nie umgeformt (ADR-0013): `anna@::ffff:127.0.0.1`, `anna@fd00:0::7` und `anna@127.0.0.01` sind Aliase
    /// derselben Adresse und damit zweite Namen desselben Gesprächs.
    pub fn kanonisch(text: &str) -> Result<Kennung, Fehler> {
        let kennung = Kennung::bilden(text)?;
        let kanonisch = text.split_once('@').is_some_and(|(_, herkunft)| {
            herkunft
                .parse::<IpAddr>()
                .is_ok_and(|adresse| adresse.to_canonical().to_string() == herkunft)
        });
        if kanonisch {
            Ok(kennung)
        } else {
            Err(Fehler::Ungueltig(format!(
                "Kennung {text:?} nicht kanonisch — erwartet name@herkunft mit der Herkunft als Adresse in ihrer kürzesten Schreibweise (IPv6 kleingeschrieben und gekürzt, IPv4 ohne führende Nullen, keine IPv4-in-IPv6-Form)"
            )))
        }
    }

    /// Aus einem geprüften Namen und der Herkunft einer Verbindung — die Textform einer IP-Adresse erfüllt
    /// die Herkunftsregel immer.
    pub(crate) fn aus(name: &Name, herkunft: IpAddr) -> Kennung {
        Kennung(format!("{name}@{herkunft}"))
    }

    pub fn als_text(&self) -> &str {
        &self.0
    }

    /// Der Name, der Teil vor dem `@` — ihn trifft eine Erwähnung (ADR-0054 Regel 6).
    pub fn name(&self) -> &str {
        self.0
            .split_once('@')
            .map_or(self.0.as_str(), |(name, _)| name)
    }

    /// Die Herkunft als Adresse, IPv4-gemappte Formen als IPv4 (wie an der Grenze, ADR-0012); eine Herkunft,
    /// die keine Adresse ist (von Hand geschrieben, etwa `x`), hat keine.
    pub fn herkunft(&self) -> Option<IpAddr> {
        let (_, herkunft) = self.0.split_once('@')?;
        herkunft.parse::<IpAddr>().ok().map(|ip| ip.to_canonical())
    }
}

fn herkunft_gueltig(herkunft: &str) -> bool {
    let erlaubt = |zeichen: char| matches!(zeichen, '0'..='9' | 'a'..='f' | '.' | ':');
    (1..=45).contains(&herkunft.len()) && herkunft.chars().all(erlaubt)
}

impl fmt::Display for Kennung {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn der_name_ist_der_teil_vor_dem_at() {
        for (text, name) in [
            ("claude@10.0.0.5", "claude"),
            ("claude-entwickler@fe80::1", "claude-entwickler"),
            ("a@1", "a"),
        ] {
            assert_eq!(Kennung::bilden(text).expect(text).name(), name);
        }
    }

    #[test]
    fn name_at_herkunft_ist_eine_kennung_auch_mit_ipv6() {
        for text in [
            "claude@10.0.0.5",
            "claude@::1",
            "shoff@127.0.0.1",
            "shoff@fe80::1",
            "claude-2@127.0.0.1",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa@10.0.0.5",
            "a@1",
        ] {
            let kennung = Kennung::bilden(text).expect(text);
            assert_eq!(kennung.als_text(), text);
            assert_eq!(kennung.to_string(), text);
        }
    }

    #[test]
    fn leer_leerzeichen_steuerzeichen_und_falsche_at_zahl_sind_ungueltig() {
        for text in [
            "",
            "cla ude@x",
            "claude",
            "a@b@c",
            "@herkunft",
            "name@",
            "a\t@b",
            "a@b\n",
            "a@b\r",
            "a\u{0}@b",
            "a\u{a0}b@c",
        ] {
            ungueltig(text);
        }
    }

    #[test]
    fn aus_name_und_herkunft_entsteht_dieselbe_kennung_wie_beim_bilden() {
        let name = Name::bilden("claude").expect("Name");
        for herkunft in [
            "127.0.0.2",
            "::1",
            "fe80::1",
            "ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff",
        ] {
            let ip: IpAddr = herkunft.parse().expect("IP");
            let text = format!("claude@{herkunft}");
            assert_eq!(
                Kennung::aus(&name, ip),
                Kennung::bilden(&text).expect(&text)
            );
        }
    }

    #[test]
    fn ein_name_gegen_die_namensregel_macht_die_kennung_ungueltig() {
        for text in [
            "ä@fe80::1",
            "Claude@127.0.0.1",
            "jörg@127.0.0.1",
            "chef\u{200B}@127.0.0.1",
            "\u{202E}abc@127.0.0.1",
            "\u{FEFF}abc@127.0.0.1",
            "a_b@127.0.0.1",
            "a.b@127.0.0.1",
            "a/b@127.0.0.1",
            "a\\b@127.0.0.1",
            "a+b@127.0.0.1",
            "-a@127.0.0.1",
            "a-@127.0.0.1",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa@127.0.0.1",
        ] {
            ungueltig(text);
        }
    }

    #[test]
    fn eine_herkunft_ausserhalb_von_0_9_a_f_punkt_doppelpunkt_oder_ueber_45_zeichen_ist_ungueltig()
    {
        let sechsundvierzig = format!("a@{}", "1".repeat(46));
        let fuenfundvierzig = format!("a@{}", "1".repeat(45));
        for text in [
            "claude@localhost",
            "claude@FE80::1",
            "claude@g::1",
            "claude@10.0.0.5/24",
            "claude@[::1]",
            "claude@10.0.0.5\u{200B}",
            sechsundvierzig.as_str(),
        ] {
            ungueltig(text);
        }
        assert!(Kennung::bilden(&fuenfundvierzig).is_ok());
    }

    fn ungueltig(text: &str) {
        {
            match Kennung::bilden(text) {
                Err(Fehler::Ungueltig(meldung)) => {
                    assert!(meldung.contains("Kennung"), "{meldung}")
                }
                anderes => panic!("{text:?} ergab {anderes:?}"),
            }
        }
    }

    /// B0765 D2 (ADR-0055 Regel 2, Matrix R5, W3): jede Schreibweise außer der einen kanonischen ist ein Fehler, die
    /// kanonische geht und bleibt byte-gleich.
    #[test]
    fn nur_die_kanonische_schreibweise_einer_kennung_gilt_aliase_werden_abgewiesen() {
        let lang = format!("{}@127.0.0.1", "a".repeat(200));
        for alias in [
            "anna@::ffff:127.0.0.1",
            "anna@::ffff:127.0.0.2",
            "anna@0:0:0:0:0:ffff:7f00:2",
            "anna@FD00::7",
            "anna@fd00:0::7",
            "anna@[fd00::7]",
            "anna@127.0.0.01",
            "anna@127.0.0.02",
            "anna@fd00::7%25eth0",
            "Anna@127.0.0.1",
            lang.as_str(),
            "anna@127.0.0.1%00",
            "anna@",
            "anna",
            "anna@fd00:0000:0000:0000:0000:0000:0000:0007",
        ] {
            ungueltig_kanonisch(alias);
        }
        for kanonisch in ["anna@fd00::7", "anna@127.0.0.1", "anna@::1"] {
            let kennung = Kennung::kanonisch(kanonisch).expect(kanonisch);
            assert_eq!(kennung.als_text(), kanonisch);
        }
        // `bilden` lässt „1“ als Herkunft zu (ADR-0012); eine Adresse ist es nicht.
        ungueltig_kanonisch("x@1");
    }

    fn ungueltig_kanonisch(text: &str) {
        match Kennung::kanonisch(text) {
            Err(Fehler::Ungueltig(meldung)) => assert!(meldung.contains("Kennung"), "{meldung}"),
            anderes => panic!("{text:?} ergab {anderes:?}"),
        }
    }

    #[test]
    fn die_herkunft_ist_die_adresse_hinter_dem_at_gemappt_als_ipv4() {
        let herkunft = |text: &str| Kennung::bilden(text).expect(text).herkunft();
        assert_eq!(herkunft("alpha@127.0.0.2"), "127.0.0.2".parse().ok());
        assert_eq!(herkunft("alpha@::ffff:10.0.0.1"), "10.0.0.1".parse().ok());
        assert_eq!(herkunft("alpha@fe80::1"), "fe80::1".parse().ok());
        assert_eq!(herkunft("alpha@1"), None);
    }
}
