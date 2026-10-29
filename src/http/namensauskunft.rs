//! Namensauskunft (ADR-0045, Schnitt): der Port zur Namensauflösung mit drei Fragen — Namen einer Adresse
//! (rückwärts), Adressen eines Namens (vorwärts), ist eine Adresse eine eigene? — dazu die Tabelle hinter dem
//! Testknopf `LOCALCHATA_TESTNAMEN` (nur Debug-Build). Jede Frage blockiert; gerufen wird nur aus der Auskunft
//! mit Cache (`auskunft.rs`), nie auf dem Laufzeit-Takt.

use std::collections::HashMap;
use std::ffi::OsStr;
use std::fmt;
use std::net::{IpAddr, ToSocketAddrs, UdpSocket};
use std::sync::{Mutex, PoisonError};

/// Name des Testknopfs.
pub const VARIABLE: &str = "LOCALCHATA_TESTNAMEN";

/// Die drei Fragen an die Namensauflösung. Jede Antwort ist vollständig: ein Fehler ist „keine Namen“, „keine
/// Adressen“ bzw. „nicht eigen“ — nie eine Panik.
pub trait Namensauskunft: Send + Sync {
    fn namen_der_adresse(&self, adresse: IpAddr) -> Vec<String>;
    fn adressen_des_namens(&self, name: &str) -> Vec<IpAddr>;
    fn ist_eigene_adresse(&self, adresse: IpAddr) -> bool;
}

/// Eine Adresse ist eigen, wenn sich ein UDP-Socket an sie binden lässt (Loopback zählt; ADR-0045 Regel 4).
pub fn bind_gelingt(adresse: IpAddr) -> bool {
    UdpSocket::bind((adresse, 0)).is_ok()
}

/// Die echte Auflösung des Systems: rückwärts über `getnameinfo` (`dns-lookup`), vorwärts über `getaddrinfo`
/// (`ToSocketAddrs`), „eigen?“ über `bind`. Probe B0364: Fehler und Zeitgrenzen kommen als Fehler, der Faden endet.
#[derive(Debug, Default)]
pub struct Echt;

impl Namensauskunft for Echt {
    fn namen_der_adresse(&self, adresse: IpAddr) -> Vec<String> {
        dns_lookup::lookup_addr(&adresse).into_iter().collect()
    }

    fn adressen_des_namens(&self, name: &str) -> Vec<IpAddr> {
        (name, 0)
            .to_socket_addrs()
            .map(|adressen| adressen.map(|adresse| adresse.ip()).collect())
            .unwrap_or_default()
    }

    fn ist_eigene_adresse(&self, adresse: IpAddr) -> bool {
        bind_gelingt(adresse)
    }
}

/// Feste Namen statt Auflösung: `<name>=<ip>[,<ip>]` vorwärts, `<ip>><name>` rückwärts, Einträge mit `;`
/// getrennt. „Eigen?“ bleibt echt (Entscheidung R00024, offene Frage 1), damit `127.0.0.1` eigen und
/// `192.0.2.1` fremd ist. Zählt jede Frage.
#[derive(Debug, Default)]
pub struct Tabelle {
    vorwaerts: HashMap<String, Vec<IpAddr>>,
    rueckwaerts: HashMap<IpAddr, Vec<String>>,
    fragen: Mutex<Vec<String>>,
}

/// `LOCALCHATA_TESTNAMEN` ist gesetzt, aber keine Tabelle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Testnamenfehler {
    pub wert: String,
}

impl fmt::Display for Testnamenfehler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{VARIABLE} ist keine Namenstabelle: {:?} — erwartet <name>=<ip>[,<ip>] oder <ip>><name>, getrennt mit ;",
            self.wert
        )
    }
}

impl Tabelle {
    /// Liest den Wert des Testknopfs; jeder Eintrag muss lesbar sein, sonst ist der ganze Wert ein Fehler.
    pub fn aus_wert(wert: &str) -> Result<Tabelle, Testnamenfehler> {
        let fehler = || Testnamenfehler {
            wert: wert.to_string(),
        };
        let mut tabelle = Tabelle::default();
        for eintrag in wert.split(';') {
            if let Some((name, adressen)) = eintrag.split_once('=') {
                let adressen = adressen_lesen(adressen).ok_or_else(fehler)?;
                if name.is_empty() {
                    return Err(fehler());
                }
                tabelle
                    .vorwaerts
                    .entry(name.to_string())
                    .or_default()
                    .extend(adressen);
            } else if let Some((adresse, name)) = eintrag.split_once('>') {
                let adresse: IpAddr = adresse.parse().map_err(|_| fehler())?;
                if name.is_empty() {
                    return Err(fehler());
                }
                tabelle
                    .rueckwaerts
                    .entry(adresse.to_canonical())
                    .or_default()
                    .push(name.to_string());
            } else {
                return Err(fehler());
            }
        }
        Ok(tabelle)
    }

    /// Im Debug-Build aus der Umgebung (`None`, wenn nicht gesetzt); im Release-Build wird die Variable nie
    /// gelesen (wie `LOCALCHATA_TESTPANIK`).
    pub fn aus_umgebung() -> Result<Option<Tabelle>, Testnamenfehler> {
        if cfg!(debug_assertions) {
            Tabelle::aus_os_wert(std::env::var_os(VARIABLE).as_deref())
        } else {
            Ok(None)
        }
    }

    fn aus_os_wert(wert: Option<&OsStr>) -> Result<Option<Tabelle>, Testnamenfehler> {
        let Some(roh) = wert else {
            return Ok(None);
        };
        let text = roh.to_str().ok_or_else(|| Testnamenfehler {
            wert: roh.to_string_lossy().into_owned(),
        })?;
        Tabelle::aus_wert(text).map(Some)
    }

    fn gefragt(&self, art: &str, schluessel: &str) {
        self.fragen
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(format!("{art} {schluessel}"));
    }

    /// Wie oft `art` (`vorwaerts`, `rueckwaerts`, `eigen`) für `schluessel` gefragt wurde.
    #[cfg(test)]
    pub fn fragen(&self, art: &str, schluessel: &str) -> usize {
        let gesucht = format!("{art} {schluessel}");
        self.fragen
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .filter(|frage| **frage == gesucht)
            .count()
    }
}

/// `<ip>[,<ip>]…`, mindestens eine.
fn adressen_lesen(text: &str) -> Option<Vec<IpAddr>> {
    text.split(',')
        .map(|adresse| adresse.parse().ok())
        .collect::<Option<Vec<IpAddr>>>()
}

impl Namensauskunft for Tabelle {
    fn namen_der_adresse(&self, adresse: IpAddr) -> Vec<String> {
        self.gefragt("rueckwaerts", &adresse.to_string());
        self.rueckwaerts
            .get(&adresse.to_canonical())
            .cloned()
            .unwrap_or_default()
    }

    fn adressen_des_namens(&self, name: &str) -> Vec<IpAddr> {
        self.gefragt("vorwaerts", name);
        self.vorwaerts.get(name).cloned().unwrap_or_default()
    }

    fn ist_eigene_adresse(&self, adresse: IpAddr) -> bool {
        self.gefragt("eigen", &adresse.to_string());
        bind_gelingt(adresse)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TESTNAMEN: &str = "127.0.0.1>nuc5.fritz.box;chat.local=127.0.0.1;drucker.local=192.0.2.1;boese.example=127.0.0.1;mix.local=192.0.2.1,127.0.0.1;127.0.0.2>CHAT.fritz.box.";

    fn ip(text: &str) -> IpAddr {
        text.parse().expect("Adresse")
    }

    #[test]
    fn die_tabelle_beantwortet_vorwaerts_und_rueckwaerts_aus_dem_testknopf() {
        let tabelle = Tabelle::aus_wert(TESTNAMEN).expect("lesbar");

        assert_eq!(
            tabelle.namen_der_adresse(ip("127.0.0.1")),
            ["nuc5.fritz.box"]
        );
        assert_eq!(
            tabelle.namen_der_adresse(ip("::ffff:127.0.0.2")),
            ["CHAT.fritz.box."]
        );
        assert!(tabelle.namen_der_adresse(ip("127.0.0.3")).is_empty());
        assert_eq!(tabelle.adressen_des_namens("chat.local"), [ip("127.0.0.1")]);
        assert_eq!(
            tabelle.adressen_des_namens("mix.local"),
            [ip("192.0.2.1"), ip("127.0.0.1")]
        );
        assert!(tabelle.adressen_des_namens("gibtsnicht.local").is_empty());
    }

    #[test]
    fn eigen_bleibt_echt_loopback_ja_test_net_nein() {
        let tabelle = Tabelle::aus_wert(TESTNAMEN).expect("lesbar");

        assert!(tabelle.ist_eigene_adresse(ip("127.0.0.1")));
        assert!(tabelle.ist_eigene_adresse(ip("::1")));
        assert!(!tabelle.ist_eigene_adresse(ip("192.0.2.1")));
    }

    #[test]
    fn die_tabelle_zaehlt_jede_frage_je_art_und_schluessel() {
        let tabelle = Tabelle::aus_wert(TESTNAMEN).expect("lesbar");

        tabelle.adressen_des_namens("chat.local");
        tabelle.adressen_des_namens("chat.local");
        tabelle.namen_der_adresse(ip("127.0.0.1"));
        tabelle.ist_eigene_adresse(ip("127.0.0.1"));

        assert_eq!(tabelle.fragen("vorwaerts", "chat.local"), 2);
        assert_eq!(tabelle.fragen("vorwaerts", "boese.example"), 0);
        assert_eq!(tabelle.fragen("rueckwaerts", "127.0.0.1"), 1);
        assert_eq!(tabelle.fragen("eigen", "127.0.0.1"), 1);
    }

    /// B0368, gegen das echte System: Loopback hat in jeder `hosts`-Datei seinen Namen; TEST-NET-1 ist nie vergeben.
    #[test]
    fn die_echte_auskunft_fragt_das_system_und_liefert_bei_fehlern_leer() {
        let echt = Echt;

        assert_eq!(echt.namen_der_adresse(ip("127.0.0.1")), ["localhost"]);
        assert!(echt.namen_der_adresse(ip("192.0.2.1")).is_empty());
        assert!(
            echt.adressen_des_namens("localhost")
                .contains(&ip("127.0.0.1")),
            "{:?}",
            echt.adressen_des_namens("localhost")
        );
        assert!(echt.adressen_des_namens("a_b.invalid").is_empty());
        assert!(echt.ist_eigene_adresse(ip("127.0.0.1")));
        assert!(!echt.ist_eigene_adresse(ip("192.0.2.1")));
    }

    /// Matrix H2: falscher Wert → Fehler, der die Variable nennt.
    #[test]
    fn falsche_werte_sind_ein_fehler_der_die_variable_nennt() {
        for falsch in [
            "quatsch",
            "x.local=",
            "x.local=999.1.1.1",
            ">x",
            "127.0.0.1>",
            "=127.0.0.1",
            "",
            "chat.local=127.0.0.1;",
            "kaputt",
        ] {
            let fehler = Tabelle::aus_wert(falsch).expect_err(falsch);
            assert_eq!(fehler.wert, falsch);
            assert!(fehler.to_string().contains(VARIABLE), "{fehler}");
        }
    }

    #[test]
    fn ohne_variable_gibt_es_keine_tabelle() {
        assert!(matches!(Tabelle::aus_os_wert(None), Ok(None)));
        assert!(matches!(
            Tabelle::aus_os_wert(Some(OsStr::new("chat=127.0.0.1"))),
            Ok(Some(_))
        ));
    }
}
