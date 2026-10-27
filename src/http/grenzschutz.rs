//! Grenzschutz (ADR-0014): welche Host-Namen die Instanz anspricht — gegen DNS-Rebinding.

use std::fmt;
use std::net::{Ipv4Addr, Ipv6Addr};

/// Die beim Start festgelegten Namen, unter denen die Instanz außer IP-Literalen und `localhost` antwortet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grenzschutz {
    rechnername: Option<String>,
    erreichbar_als: Vec<String>,
}

/// Der Host ist nicht freigegeben; `name` ist der Host-Anteil ohne Port, `None` heißt: kein Host-Header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostAbgewiesen {
    pub name: Option<String>,
}

impl Grenzschutz {
    pub fn neu(rechnername: Option<String>, erreichbar_als: Vec<String>) -> Self {
        Grenzschutz {
            rechnername,
            erreichbar_als,
        }
    }

    /// Erlaubt ist der Host-Anteil (ohne Port), wenn er ein IP-Literal, `localhost`, der Rechnername,
    /// `<rechnername>.local` oder ein mit `--erreichbar-als` freigegebener Name ist — Namen ohne Beachtung
    /// der Groß-/Kleinschreibung. Ein Wert, der sich nicht als Host mit optionalem Port lesen lässt, nie.
    pub fn host_erlaubt(&self, host: Option<&str>) -> Result<(), HostAbgewiesen> {
        let Some(wert) = host else {
            return Err(HostAbgewiesen { name: None });
        };
        match host_anteil(wert) {
            Some(Hostanteil::IpLiteral) => Ok(()),
            Some(Hostanteil::Name(name)) if self.name_freigegeben(name) => Ok(()),
            Some(Hostanteil::Name(name)) => Err(HostAbgewiesen {
                name: Some(name.to_string()),
            }),
            None => Err(HostAbgewiesen {
                name: Some(wert.to_string()),
            }),
        }
    }

    fn name_freigegeben(&self, name: &str) -> bool {
        let ist = |frei: &str| !frei.is_empty() && frei.eq_ignore_ascii_case(name);
        let rechnername = self.rechnername.as_deref().unwrap_or_default();
        ist("localhost")
            || ist(rechnername)
            || (!rechnername.is_empty() && ist(&format!("{rechnername}.local")))
            || self.erreichbar_als.iter().any(|frei| ist(frei))
    }
}

enum Hostanteil<'a> {
    IpLiteral,
    Name(&'a str),
}

/// `Host`-Wert → Host-Anteil: IPv6 nur in eckigen Klammern, danach höchstens `:<ziffern>`.
fn host_anteil(wert: &str) -> Option<Hostanteil<'_>> {
    if let Some(rest) = wert.strip_prefix('[') {
        let (adresse, nach) = rest.split_once(']')?;
        adresse.parse::<Ipv6Addr>().ok()?;
        return port_lesbar(nach).then_some(Hostanteil::IpLiteral);
    }
    let (name, nach) = match wert.find(':') {
        Some(stelle) => wert.split_at(stelle),
        None => (wert, ""),
    };
    if !port_lesbar(nach) {
        return None;
    }
    if name.parse::<Ipv4Addr>().is_ok() {
        return Some(Hostanteil::IpLiteral);
    }
    Some(Hostanteil::Name(name))
}

/// Leer oder `:` gefolgt von mindestens einer Ziffer und nur Ziffern.
fn port_lesbar(nach_dem_host: &str) -> bool {
    match nach_dem_host.strip_prefix(':') {
        Some(port) => !port.is_empty() && port.bytes().all(|zeichen| zeichen.is_ascii_digit()),
        None => nach_dem_host.is_empty(),
    }
}

impl fmt::Display for HostAbgewiesen {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.name {
            Some(name) => write!(
                f,
                "Diese Instanz ist unter dem Namen {name} nicht freigegeben — über die IP-Adresse öffnen oder mit --erreichbar-als {name} starten."
            ),
            None => write!(
                f,
                "Diese Anfrage nennt keinen Host — über die IP-Adresse öffnen."
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schutz() -> Grenzschutz {
        Grenzschutz::neu(Some("nuc5".into()), vec!["chat.lan".into(), "nuc".into()])
    }

    #[test]
    fn ip_literale_localhost_rechnername_und_freigegebene_namen_sind_erlaubt() {
        for host in [
            "127.0.0.1:7430",
            "127.0.0.1",
            "10.0.0.5:80",
            "[::1]:7430",
            "[::1]",
            "[fe80::1]:80",
            "localhost:7430",
            "localhost",
            "LocalHost:1",
            "nuc5:7430",
            "NUC5:7430",
            "nuc5.local:7430",
            "Nuc5.Local",
            "chat.lan:80",
            "CHAT.LAN",
            "nuc",
        ] {
            assert_eq!(schutz().host_erlaubt(Some(host)), Ok(()), "{host}");
        }
    }

    #[test]
    fn fremde_namen_werden_ohne_port_genannt_abgewiesen() {
        for (host, name) in [
            ("evil.example", "evil.example"),
            ("evil.example:80", "evil.example"),
            ("127.0.0.1.evil.example:7430", "127.0.0.1.evil.example"),
            ("localhost.evil.example", "localhost.evil.example"),
            ("xlocalhost:1", "xlocalhost"),
            ("localhost.:1", "localhost."),
            ("nuc5.evil:1", "nuc5.evil"),
            ("nuc5.local.evil", "nuc5.local.evil"),
            ("chat.lan.evil:80", "chat.lan.evil"),
            ("evil.chat.lan", "evil.chat.lan"),
            ("", ""),
            (":80", ""),
        ] {
            assert_eq!(
                schutz().host_erlaubt(Some(host)),
                Err(HostAbgewiesen {
                    name: Some(name.into())
                }),
                "{host}"
            );
        }
    }

    #[test]
    fn unlesbare_host_werte_werden_abgewiesen() {
        for host in [
            "::1",
            "[::1",
            "[::1]x",
            "[::1]:",
            "[evil]:80",
            "[127.0.0.1]",
            "127.0.0.1:abc",
            "127.0.0.1:",
            "127.0.0.1:80:80",
            "localhost:80 ",
            "[fe80::1%25eth0]",
        ] {
            assert!(schutz().host_erlaubt(Some(host)).is_err(), "{host}");
        }
    }

    #[test]
    fn ohne_host_header_wird_abgewiesen() {
        assert_eq!(
            schutz().host_erlaubt(None),
            Err(HostAbgewiesen { name: None })
        );
    }

    #[test]
    fn ohne_rechnername_gilt_kein_rechnername_und_leere_freigaben_gelten_nicht() {
        let schutz = Grenzschutz::neu(None, vec![String::new()]);
        assert!(schutz.host_erlaubt(Some("nuc5:1")).is_err());
        assert!(schutz.host_erlaubt(Some(":1")).is_err());
        assert!(schutz.host_erlaubt(Some(".local")).is_err());
        assert_eq!(schutz.host_erlaubt(Some("127.0.0.1:1")), Ok(()));
    }

    #[test]
    fn die_abweisung_nennt_den_namen_und_den_ausweg() {
        let text = HostAbgewiesen {
            name: Some("chat.lan".into()),
        }
        .to_string();
        assert_eq!(
            text,
            "Diese Instanz ist unter dem Namen chat.lan nicht freigegeben — über die IP-Adresse öffnen oder mit --erreichbar-als chat.lan starten."
        );
    }

    #[test]
    fn die_abweisung_ohne_host_sagt_dass_er_fehlt() {
        let text = HostAbgewiesen { name: None }.to_string();
        assert!(text.contains("keinen Host"), "{text}");
        assert!(text.contains("IP-Adresse"), "{text}");
    }
}
