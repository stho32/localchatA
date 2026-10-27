//! Listener binden: Standard-Port mit Ausweichen, die Wahl des Systems oder genau der verlangte Port.

use super::{Portwunsch, Startfehler};
use std::io;
use tokio::net::TcpListener;

pub const STANDARD_PORT: u16 = 7430;
const AUSWEICHVERSUCHE: u16 = 50;

/// Die Ports, die in dieser Reihenfolge versucht werden.
pub fn kandidaten(wunsch: Portwunsch) -> Vec<u16> {
    match wunsch {
        Portwunsch::Standard => (STANDARD_PORT..STANDARD_PORT + AUSWEICHVERSUCHE).collect(),
        Portwunsch::System => vec![0],
        Portwunsch::Fest(port) => vec![port],
    }
}

/// Welche Ports versucht werden und ob bei einem belegten zum nächsten ausgewichen wird.
#[derive(Debug, PartialEq, Eq)]
struct Bindeplan {
    kandidaten: Vec<u16>,
    ausweichen: bool,
}

/// Ausgewichen wird nur beim Standard-Port; ein ausdrücklicher Port scheitert, wenn er belegt ist.
fn bindeplan(wunsch: Portwunsch) -> Bindeplan {
    Bindeplan {
        kandidaten: kandidaten(wunsch),
        ausweichen: wunsch == Portwunsch::Standard,
    }
}

/// Bindet den ersten freien Kandidaten nach dem Bindeplan.
pub async fn listener_binden(host: &str, wunsch: Portwunsch) -> Result<TcpListener, Startfehler> {
    let plan = bindeplan(wunsch);
    erster_freier(host, &plan.kandidaten, plan.ausweichen).await
}

async fn erster_freier(
    host: &str,
    kandidaten: &[u16],
    ausweichen: bool,
) -> Result<TcpListener, Startfehler> {
    let portfehler = |port: u16, grund: String| Startfehler::Port {
        host: host.to_string(),
        port,
        grund,
    };
    for &port in kandidaten {
        match TcpListener::bind((host, port)).await {
            Ok(listener) => return Ok(listener),
            Err(fehler) if ausweichen && fehler.kind() == io::ErrorKind::AddrInUse => continue,
            Err(fehler) => return Err(portfehler(port, fehler.to_string())),
        }
    }
    let erster = kandidaten.first().copied().unwrap_or_default();
    Err(portfehler(
        erster,
        format!("alle {} Ports ab {erster} sind belegt", kandidaten.len()),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn freier_port() -> u16 {
        let probe = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("Probe gebunden");
        probe.local_addr().expect("Probe-Adresse").port()
    }

    #[test]
    fn standard_versucht_fuenfzig_ports_ab_7430_aufwaerts() {
        let ports = kandidaten(Portwunsch::Standard);
        assert_eq!(ports.len(), 50);
        assert_eq!(ports.first(), Some(&7430));
        assert_eq!(ports.last(), Some(&7479));
        assert!(ports.windows(2).all(|paar| paar[1] == paar[0] + 1));
    }

    #[test]
    fn fester_port_und_systemwahl_haben_genau_einen_kandidaten() {
        assert_eq!(kandidaten(Portwunsch::Fest(8123)), vec![8123]);
        assert_eq!(kandidaten(Portwunsch::System), vec![0]);
    }

    #[test]
    fn nur_der_standard_port_weicht_aus() {
        let standard = bindeplan(Portwunsch::Standard);
        assert_eq!(standard.kandidaten, kandidaten(Portwunsch::Standard));
        assert!(standard.ausweichen);
        assert!(!bindeplan(Portwunsch::Fest(8123)).ausweichen);
        assert!(!bindeplan(Portwunsch::System).ausweichen);
    }

    #[tokio::test]
    async fn systemwahl_liefert_einen_echten_port() {
        let listener = listener_binden("127.0.0.1", Portwunsch::System)
            .await
            .expect("gebunden");
        assert_ne!(listener.local_addr().expect("Adresse").port(), 0);
    }

    #[tokio::test]
    async fn belegter_kandidat_wird_beim_ausweichen_uebersprungen() {
        let belegt = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("belegt");
        let belegter_port = belegt.local_addr().expect("Adresse").port();
        let freier = freier_port();

        let listener = erster_freier("127.0.0.1", &[belegter_port, freier], true)
            .await
            .expect("ausgewichen");

        assert_eq!(listener.local_addr().expect("Adresse").port(), freier);
    }

    #[tokio::test]
    async fn belegter_fester_port_ist_ein_portfehler_ohne_ausweichen() {
        let belegt = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("belegt");
        let belegter_port = belegt.local_addr().expect("Adresse").port();

        let fehler = listener_binden("127.0.0.1", Portwunsch::Fest(belegter_port))
            .await
            .expect_err("darf nicht ausweichen");

        assert!(matches!(fehler, Startfehler::Port { port, .. } if port == belegter_port));
        assert!(fehler.to_string().contains(&belegter_port.to_string()));
    }

    #[tokio::test]
    async fn sind_alle_kandidaten_belegt_nennt_der_fehler_den_ersten() {
        let erster = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("belegt");
        let zweiter = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("belegt");
        let ports = [
            erster.local_addr().expect("Adresse").port(),
            zweiter.local_addr().expect("Adresse").port(),
        ];

        let fehler = erster_freier("127.0.0.1", &ports, true)
            .await
            .expect_err("alles belegt");

        assert!(matches!(fehler, Startfehler::Port { port, .. } if port == ports[0]));
    }
}
