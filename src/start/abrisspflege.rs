//! Abrisspflege (B0099, ADR-0017 Regel 2): aus dem Herzschlag `H` die Werte, mit denen der Kernel eine tote
//! Verbindung selbst abbricht — `TCP_USER_TIMEOUT` = 2H (die Abrissgrenze), Keepalive nach H, alle H, 2 Proben.
//! Rein: der Herzschlag kommt als Argument, gesetzt wird anderswo.

use std::time::Duration;

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
