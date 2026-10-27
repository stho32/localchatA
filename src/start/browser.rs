//! Browser öffnen: nach der Startzeile den Standardbrowser mit der Oberfläche; scheitert das, ein Hinweis.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::process::{Command, Stdio};

/// Testknopf: ersetzt den Standardbrowser durch ein eigenes Programm (ADR-0009, X00006).
const EIGENER_BROWSER: &str = "LOCALCHATA_BROWSER";

/// Öffnet die Oberfläche im Browser; scheitert das, steht ein Hinweis auf stderr und der Start läuft weiter.
pub fn oeffnen(adresse: SocketAddr) {
    let ziel = browser_adresse(adresse);
    let (programm, argumente) = befehl(std::env::var(EIGENER_BROWSER).ok(), &ziel);
    let gestartet = Command::new(&programm)
        .args(&argumente)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    match gestartet {
        Ok(mut kind) => {
            std::thread::spawn(move || match kind.wait() {
                Ok(ende) if ende.success() => {}
                Ok(ende) => hinweis(&ziel, &format!("{programm} endete mit {ende}")),
                Err(fehler) => hinweis(&ziel, &fehler.to_string()),
            });
        }
        Err(fehler) => hinweis(&ziel, &format!("{programm}: {fehler}")),
    }
}

fn hinweis(ziel: &str, grund: &str) {
    eprintln!("Browser ließ sich nicht öffnen ({grund}) — bitte {ziel} selbst aufrufen");
}

/// Unter welcher Adresse ein Browser auf diesem Rechner die Instanz erreicht.
pub fn browser_adresse(adresse: SocketAddr) -> String {
    let mut ziel = adresse;
    match adresse.ip() {
        IpAddr::V4(ip) if ip.is_unspecified() => ziel.set_ip(IpAddr::V4(Ipv4Addr::LOCALHOST)),
        IpAddr::V6(ip) if ip.is_unspecified() => ziel.set_ip(IpAddr::V6(Ipv6Addr::LOCALHOST)),
        _ => {}
    }
    format!("http://{ziel}/")
}

/// Programm und Argumente, die den Browser öffnen; ein eigenes Programm geht dem Standard vor.
pub fn befehl(eigenes: Option<String>, adresse: &str) -> (String, Vec<String>) {
    let adresse = adresse.to_string();
    match eigenes {
        Some(programm) => (programm, vec![adresse]),
        None if cfg!(target_os = "macos") => ("open".into(), vec![adresse]),
        None if cfg!(windows) => (
            "cmd".into(),
            vec!["/C".into(), "start".into(), String::new(), adresse],
        ),
        None => ("xdg-open".into(), vec![adresse]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn adresse(text: &str) -> SocketAddr {
        text.parse().expect("Socket-Adresse")
    }

    #[test]
    fn alle_schnittstellen_werden_fuer_den_browser_zur_eigenen_maschine() {
        assert_eq!(
            browser_adresse(adresse("0.0.0.0:7430")),
            "http://127.0.0.1:7430/"
        );
        assert_eq!(browser_adresse(adresse("[::]:7431")), "http://[::1]:7431/");
    }

    #[test]
    fn eine_bestimmte_adresse_bleibt_die_der_startzeile() {
        assert_eq!(
            browser_adresse(adresse("127.0.0.1:8123")),
            "http://127.0.0.1:8123/"
        );
        assert_eq!(
            browser_adresse(adresse("192.168.1.20:7430")),
            "http://192.168.1.20:7430/"
        );
    }

    #[test]
    fn ein_eigenes_programm_bekommt_nur_die_adresse() {
        assert_eq!(
            befehl(Some("/opt/mein-browser".into()), "http://127.0.0.1:1/"),
            (
                "/opt/mein-browser".to_string(),
                vec!["http://127.0.0.1:1/".to_string()]
            )
        );
    }

    #[test]
    fn ohne_eigenes_programm_gilt_der_standardbrowser_der_plattform() {
        let (programm, argumente) = befehl(None, "http://127.0.0.1:1/");
        let erwartet = if cfg!(target_os = "macos") {
            "open"
        } else if cfg!(windows) {
            "cmd"
        } else {
            "xdg-open"
        };
        assert_eq!(programm, erwartet);
        assert_eq!(
            argumente.last().map(String::as_str),
            Some("http://127.0.0.1:1/")
        );
    }
}
