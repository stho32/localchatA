//! Status je Sitzung (B0075, ADR-0016 Regel 1): `online`, solange ein Halter offen ist oder seit
//! `max(Beitritt, Ende des letzten Halters)` weniger als der Nachlauf von 1 s vergangen ist; sonst `offline`.
//! Rein: „jetzt“ kommt als Argument (X00007).

use crate::uhr::Zeit;
use serde::Serialize;

/// Anlauf nach dem Beitritt und Nachlauf nach dem letzten Halter — glättet Neuladen und Warte-Zyklen.
pub const NACHLAUF: chrono::Duration = chrono::Duration::seconds(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Online,
    Offline,
    #[cfg_attr(
        test,
        expect(dead_code, reason = "Brücke bis B0081: pflegen meldet weg")
    )]
    /// Die Sitzung lebt nicht mehr (Gnadenfrist ohne Lebenszeichen vorbei) — sie steht in keiner Liste.
    Weg,
}

/// Was den Status einer Sitzung bestimmt: Beitritt, offene Halter, Ende des letzten Halters und seit wann
/// sie ohne Unterbrechung online ist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Haltung {
    pub beitritt: Zeit,
    pub halter: usize,
    pub halter_ende: Option<Zeit>,
    pub online_seit: Zeit,
}

impl Haltung {
    /// Status und der Zeitpunkt, ab dem er gilt: `online` seit dem Beitritt bzw. dem Halter nach einem
    /// `offline`; `offline` seit dem Ende des Nachlaufs.
    pub fn status(&self, jetzt: Zeit) -> (Status, Zeit) {
        if self.halter > 0 {
            return (Status::Online, self.online_seit);
        }
        let zuletzt_gebunden = self
            .halter_ende
            .map_or(self.beitritt, |ende| ende.max(self.beitritt));
        let nachlauf_ende = zuletzt_gebunden + NACHLAUF;
        if jetzt < nachlauf_ende {
            return (Status::Online, self.online_seit);
        }
        (Status::Offline, nachlauf_ende)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: &str = "2026-10-28T09:00:00.000+01:00";

    fn nach(millisekunden: i64) -> Zeit {
        Zeit::parse_from_rfc3339(T0).expect("Zeit") + chrono::Duration::milliseconds(millisekunden)
    }

    fn beigetreten() -> Haltung {
        Haltung {
            beitritt: nach(0),
            halter: 0,
            halter_ende: None,
            online_seit: nach(0),
        }
    }

    #[test]
    fn ohne_halter_online_bis_999_ms_nach_dem_beitritt_ab_1_s_offline() {
        let haltung = beigetreten();

        assert_eq!(haltung.status(nach(0)), (Status::Online, nach(0)));
        assert_eq!(haltung.status(nach(500)), (Status::Online, nach(0)));
        assert_eq!(haltung.status(nach(999)), (Status::Online, nach(0)));
        assert_eq!(haltung.status(nach(1000)), (Status::Offline, nach(1000)));
        assert_eq!(haltung.status(nach(1500)), (Status::Offline, nach(1000)));
    }

    #[test]
    fn mit_offenem_halter_online_seit_dem_beginn_auch_lange_danach() {
        let haltung = Haltung {
            halter: 2,
            online_seit: nach(4000),
            ..beigetreten()
        };

        assert_eq!(
            haltung.status(nach(3_600_000)),
            (Status::Online, nach(4000))
        );
    }

    #[test]
    fn nach_dem_letzten_halter_laeuft_der_nachlauf_ab_dessen_ende() {
        let haltung = Haltung {
            halter_ende: Some(nach(7000)),
            online_seit: nach(2000),
            ..beigetreten()
        };

        assert_eq!(haltung.status(nach(7999)), (Status::Online, nach(2000)));
        assert_eq!(haltung.status(nach(8000)), (Status::Offline, nach(8000)));
    }

    #[test]
    fn ein_halter_ende_vor_dem_beitritt_verkuerzt_den_anlauf_nicht() {
        let haltung = Haltung {
            halter_ende: Some(nach(-5000)),
            ..beigetreten()
        };

        assert_eq!(haltung.status(nach(999)), (Status::Online, nach(0)));
        assert_eq!(haltung.status(nach(1000)), (Status::Offline, nach(1000)));
    }
}
