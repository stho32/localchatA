//! Abrisspflege (B0099, ADR-0017 Regel 2): aus dem Herzschlag `H` die Werte, mit denen der Kernel eine tote
//! Verbindung selbst abbricht — `TCP_USER_TIMEOUT` = 2H (die Abrissgrenze), Keepalive nach H, alle H, 2 Proben.
//! `pflege_aus` ist rein; `kernel_pflegen` setzt die Werte am Socket (B0100, über `rustix::net::sockopt`).

use crate::protokoll::{Bereich, Kontext, Protokoll, Schluessel, Stufe};
use std::fmt;
use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::net::TcpStream;

/// Proben ohne Antwort, nach denen Keepalive die Verbindung aufgibt.
const KEEPALIVE_PROBEN: u32 = 2;

/// Was je zugelassener Verbindung gesetzt wird; unveränderlich.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Verbindungspflege {
    /// So lange dürfen gesendete Daten unbestätigt bleiben, dann bricht der Kernel ab (Abrissgrenze).
    pub user_timeout: Duration,
    /// Ruhezeit bis zur ersten Keepalive-Probe, ganze Sekunden.
    pub keepidle: Duration,
    /// Abstand der Keepalive-Proben, ganze Sekunden.
    pub keepintvl: Duration,
    pub keepcnt: u32,
}

/// Abrissgrenze 2H auf ganze Millisekunden aufgerundet; Keepalive in ganzen Sekunden (die Einheit des
/// Kernels), aufgerundet und mindestens 1 s.
pub fn pflege_aus(herzschlag: Duration) -> Verbindungspflege {
    let keepalive = ganze_sekunden_aufgerundet(herzschlag);
    Verbindungspflege {
        user_timeout: ganze_millisekunden_aufgerundet(herzschlag.saturating_mul(2)),
        keepidle: keepalive,
        keepintvl: keepalive,
        keepcnt: KEEPALIVE_PROBEN,
    }
}

fn ganze_sekunden_aufgerundet(dauer: Duration) -> Duration {
    let sekunden = dauer.as_secs() + u64::from(dauer.subsec_nanos() > 0);
    Duration::from_secs(sekunden.max(1))
}

fn ganze_millisekunden_aufgerundet(dauer: Duration) -> Duration {
    let rest = dauer.subsec_nanos() % 1_000_000;
    let millisekunden = dauer.as_millis() + u128::from(rest > 0);
    Duration::from_millis(u64::try_from(millisekunden).unwrap_or(u64::MAX))
}

/// Eine Option ließ sich nicht setzen — welche und warum.
#[derive(Debug)]
pub struct Pflegefehler {
    pub option: &'static str,
    pub fehler: io::Error,
}

impl fmt::Display for Pflegefehler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Abrisspflege: {} lässt sich nicht setzen ({}) — Verbindungen werden ohne Pflege bedient",
            self.option, self.fehler
        )
    }
}

/// Setzt die Pflege am Socket: `TCP_USER_TIMEOUT` (nur Linux), dann Keepalive an mit Ruhezeit, Abstand und
/// Proben. Bricht beim ersten Fehler ab und nennt die Option.
#[cfg(unix)]
pub fn kernel_pflegen(
    socket: impl std::os::fd::AsFd,
    pflege: &Verbindungspflege,
) -> Result<(), Pflegefehler> {
    use rustix::net::sockopt;
    let socket = socket.as_fd();
    let mit = |option: &'static str| {
        move |fehler: rustix::io::Errno| Pflegefehler {
            option,
            fehler: fehler.into(),
        }
    };
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        let millisekunden = u32::try_from(pflege.user_timeout.as_millis()).unwrap_or(u32::MAX);
        sockopt::set_tcp_user_timeout(socket, millisekunden).map_err(mit("TCP_USER_TIMEOUT"))?;
    }
    sockopt::set_socket_keepalive(socket, true).map_err(mit("SO_KEEPALIVE"))?;
    sockopt::set_tcp_keepidle(socket, pflege.keepidle).map_err(mit("TCP_KEEPIDLE"))?;
    sockopt::set_tcp_keepintvl(socket, pflege.keepintvl).map_err(mit("TCP_KEEPINTVL"))?;
    sockopt::set_tcp_keepcnt(socket, pflege.keepcnt).map_err(mit("TCP_KEEPCNT"))?;
    Ok(())
}

/// Ohne Unix gibt es hier keine Kernel-Fassung (ADR-0017: Projektumgebung Linux) — sichtbar als Fehler.
#[cfg(not(unix))]
pub fn kernel_pflegen<S>(_socket: S, _pflege: &Verbindungspflege) -> Result<(), Pflegefehler> {
    Err(Pflegefehler {
        option: "SO_KEEPALIVE",
        fehler: io::Error::from(io::ErrorKind::Unsupported),
    })
}

type Pflegen = dyn Fn(&TcpStream) -> Result<(), Pflegefehler> + Send + Sync;
type Melden = dyn Fn(&Pflegefehler) + Send + Sync;

/// Die Naht für die Annahmeschleife (B0101, ADR-0017 Regel 3 und 4): pflegt eine zugelassene Verbindung;
/// scheitert es, meldet sie nur den ersten Fehler — nie einen je Verbindung (stderr-Flut, ADR-0010).
#[derive(Clone)]
pub struct Pflege {
    pflegen: Arc<Pflegen>,
    melden: Arc<Melden>,
    gemeldet: Arc<AtomicBool>,
}

impl Pflege {
    /// Die Fassung des Programms: Kernel-Pflege, der erste Fehler eine WARNING im Betriebsprotokoll.
    pub fn kernel(pflege: Verbindungspflege, protokoll: Arc<Protokoll>) -> Pflege {
        Pflege::neu(
            move |strom: &TcpStream| kernel_pflegen(strom, &pflege),
            move |fehler: &Pflegefehler| {
                protokoll.eintragen(
                    Stufe::Warning,
                    Bereich::Verbindung,
                    "Abrisspflege nicht setzbar — Verbindungen werden ohne Pflege bedient",
                    Kontext::neu()
                        .mit(Schluessel::Option, fehler.option)
                        .mit(Schluessel::Grund, &fehler.fehler),
                );
            },
        )
    }

    pub fn neu(
        pflegen: impl Fn(&TcpStream) -> Result<(), Pflegefehler> + Send + Sync + 'static,
        melden: impl Fn(&Pflegefehler) + Send + Sync + 'static,
    ) -> Pflege {
        Pflege {
            pflegen: Arc::new(pflegen),
            melden: Arc::new(melden),
            gemeldet: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Pflegt die Verbindung; ein Fehler hält sie nicht auf — sie wird ohne Pflege bedient wie zuvor.
    pub fn anwenden(&self, strom: &TcpStream) {
        let Err(fehler) = (self.pflegen)(strom) else {
            return;
        };
        let zuerst = !self.gemeldet.swap(true, Ordering::SeqCst);
        if zuerst {
            (self.melden)(&fehler);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pflege(user_timeout_ms: u64, keepalive_s: u64) -> Verbindungspflege {
        Verbindungspflege {
            user_timeout: Duration::from_millis(user_timeout_ms),
            keepidle: Duration::from_secs(keepalive_s),
            keepintvl: Duration::from_secs(keepalive_s),
            keepcnt: 2,
        }
    }

    #[test]
    fn herzschlag_5_s_ergibt_abrissgrenze_10_s_und_keepalive_5_s_5_s_2() {
        assert_eq!(pflege_aus(Duration::from_secs(5)), pflege(10_000, 5));
    }

    #[test]
    fn herzschlag_500_ms_ergibt_1000_ms_und_keepalive_aufgerundet_auf_1_s() {
        assert_eq!(pflege_aus(Duration::from_millis(500)), pflege(1_000, 1));
    }

    #[test]
    fn angebrochene_sekunden_werden_aufgerundet_nie_abgerundet() {
        assert_eq!(pflege_aus(Duration::from_millis(5_001)), pflege(10_002, 6));
        assert_eq!(pflege_aus(Duration::from_millis(1)), pflege(2, 1));
    }
}

/// Probe der Kernel-Fassung am echten Loopback-Socket (B0100, dependency-probe für `rustix::net::sockopt`):
/// gesetzt wird, was `getsockopt` genau so zurückliest; ein Deskriptor, der kein Socket ist, ergibt einen
/// Fehler, der die Option nennt.
#[cfg(all(test, target_os = "linux"))]
mod kernel {
    use super::*;
    use rustix::net::sockopt;

    #[tokio::test]
    async fn die_pflege_aus_5_s_liest_getsockopt_am_angenommenen_socket_genau_zurueck() {
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("gebunden");
        let adresse = listener.local_addr().expect("Adresse");
        let _client = tokio::net::TcpStream::connect(adresse)
            .await
            .expect("verbunden");
        let (angenommen, _) = listener.accept().await.expect("angenommen");

        kernel_pflegen(&angenommen, &pflege_aus(Duration::from_secs(5))).expect("gesetzt");

        assert_eq!(
            sockopt::tcp_user_timeout(&angenommen).expect("lesen"),
            10_000
        );
        assert!(sockopt::socket_keepalive(&angenommen).expect("lesen"));
        assert_eq!(
            sockopt::tcp_keepidle(&angenommen).expect("lesen"),
            Duration::from_secs(5)
        );
        assert_eq!(
            sockopt::tcp_keepintvl(&angenommen).expect("lesen"),
            Duration::from_secs(5)
        );
        assert_eq!(sockopt::tcp_keepcnt(&angenommen).expect("lesen"), 2);
    }

    #[test]
    fn auf_einem_deskriptor_der_kein_socket_ist_nennt_der_fehler_option_und_grund() {
        let datei = tempfile::tempfile().expect("Datei");

        let fehler =
            kernel_pflegen(&datei, &pflege_aus(Duration::from_secs(5))).expect_err("kein Socket");

        assert_eq!(fehler.option, "TCP_USER_TIMEOUT");
        assert_eq!(
            fehler.fehler.raw_os_error(),
            Some(rustix::io::Errno::NOTSOCK.raw_os_error())
        );
        let text = fehler.to_string();
        assert!(text.contains("TCP_USER_TIMEOUT"), "{text}");
    }
}
