//! Ereignisbus (B0082, A00011): jedes veröffentlichte Ereignis erreicht alle Abonnenten; wer zu langsam
//! liest, erfährt es als „neuer Stand“ statt einer stillen Lücke (ADR-0016 Regel 5).

use crate::anwesenheit::Wechsel;
use crate::gespraeche::ChannelWechsel;
use tokio::sync::broadcast;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ereignis {
    Anwesenheit(Wechsel),
    Channel(ChannelWechsel),
}

/// Was ein Abonnent als Nächstes bekommt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Empfang {
    Ereignis(Ereignis),
    /// Ereignisse sind verloren gegangen — der Abonnent braucht einen neuen Stand.
    NeuerStand,
}

pub struct Bus {
    sender: broadcast::Sender<Ereignis>,
}

pub struct Abonnement {
    empfaenger: broadcast::Receiver<Ereignis>,
}

impl Bus {
    /// `kapazitaet`: so viele Ereignisse darf ein Abonnent zurückliegen, bevor er einen neuen Stand braucht.
    pub fn neu(kapazitaet: usize) -> Bus {
        let (sender, _) = broadcast::channel(kapazitaet);
        Bus { sender }
    }

    /// Ohne Abonnenten geht das Ereignis an niemanden — das ist kein Fehler.
    pub fn veroeffentlichen(&self, ereignis: Ereignis) {
        let _ohne_abonnenten = self.sender.send(ereignis);
    }

    pub fn abonnieren(&self) -> Abonnement {
        Abonnement {
            empfaenger: self.sender.subscribe(),
        }
    }
}

impl Abonnement {
    /// Das nächste Ereignis oder `NeuerStand`; `None`, wenn der Bus fort ist.
    pub async fn naechstes(&mut self) -> Option<Empfang> {
        match self.empfaenger.recv().await {
            Ok(ereignis) => Some(Empfang::Ereignis(ereignis)),
            Err(broadcast::error::RecvError::Lagged(_)) => Some(Empfang::NeuerStand),
            Err(broadcast::error::RecvError::Closed) => None,
        }
    }
}

/// Die Veröffentlichung für die Anwesenheit: jeder Wechsel als Ereignis auf den Bus.
pub fn anwesenheit_veroeffentlichen(
    bus: &std::sync::Arc<Bus>,
) -> crate::anwesenheit::Veroeffentlichen {
    let bus = std::sync::Arc::clone(bus);
    std::sync::Arc::new(move |wechsel: &Wechsel| {
        bus.veroeffentlichen(Ereignis::Anwesenheit(wechsel.clone()));
    })
}

/// Die Veröffentlichung für die Channels: jeder Wechsel als Ereignis auf den Bus (ADR-0023 Regel 2).
pub fn channels_veroeffentlichen(
    bus: &std::sync::Arc<Bus>,
) -> crate::gespraeche::ChannelVeroeffentlichen {
    let bus = std::sync::Arc::clone(bus);
    std::sync::Arc::new(move |wechsel: &ChannelWechsel| {
        bus.veroeffentlichen(Ereignis::Channel(wechsel.clone()));
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anwesenheit::status::Status;
    use crate::anwesenheit::{Art, Teilnehmerzeile};
    use crate::gespraeche::Kennung;
    use crate::name::Name;
    use crate::uhr::Zeit;

    fn wechsel(stand: u64) -> Ereignis {
        Ereignis::Anwesenheit(Wechsel {
            stand,
            jetzt: Zeit::parse_from_rfc3339("2026-10-28T09:00:00+01:00").expect("Zeit"),
            teilnehmer: Teilnehmerzeile {
                kennung: Kennung::bilden("alpha@127.0.0.2").expect("Kennung"),
                name: Name::bilden("alpha").expect("Name"),
                herkunft: "127.0.0.2".parse().expect("IP"),
                art: Art::Agent,
                status: Status::Online,
                seit: Zeit::parse_from_rfc3339("2026-10-28T09:00:00+01:00").expect("Zeit"),
                weg_um: None,
            },
        })
    }

    #[tokio::test]
    async fn jedes_ereignis_erreicht_alle_abonnenten_in_der_reihenfolge() {
        let bus = Bus::neu(16);
        let mut erster = bus.abonnieren();
        let mut zweiter = bus.abonnieren();

        bus.veroeffentlichen(wechsel(1));
        bus.veroeffentlichen(wechsel(2));

        for abonnement in [&mut erster, &mut zweiter] {
            assert_eq!(
                abonnement.naechstes().await,
                Some(Empfang::Ereignis(wechsel(1)))
            );
            assert_eq!(
                abonnement.naechstes().await,
                Some(Empfang::Ereignis(wechsel(2)))
            );
        }
    }

    #[tokio::test]
    async fn wer_zu_langsam_liest_bekommt_einen_neuen_stand_und_liest_dann_weiter() {
        let bus = Bus::neu(2);
        let mut langsam = bus.abonnieren();

        for stand in 1..=4 {
            bus.veroeffentlichen(wechsel(stand));
        }

        assert_eq!(langsam.naechstes().await, Some(Empfang::NeuerStand));
        assert_eq!(
            langsam.naechstes().await,
            Some(Empfang::Ereignis(wechsel(3)))
        );
        assert_eq!(
            langsam.naechstes().await,
            Some(Empfang::Ereignis(wechsel(4)))
        );
    }

    #[tokio::test]
    async fn ohne_abonnenten_ist_veroeffentlichen_kein_fehler_und_spaetere_abonnenten_sehen_nur_neues()
     {
        let bus = Bus::neu(4);
        bus.veroeffentlichen(wechsel(1));
        let mut spaet = bus.abonnieren();
        bus.veroeffentlichen(wechsel(2));

        assert_eq!(spaet.naechstes().await, Some(Empfang::Ereignis(wechsel(2))));
    }

    #[tokio::test]
    async fn ist_der_bus_fort_endet_das_abonnement() {
        let bus = Bus::neu(4);
        let mut abonnement = bus.abonnieren();
        drop(bus);

        assert_eq!(abonnement.naechstes().await, None);
    }

    #[tokio::test]
    async fn ein_channel_wechsel_geht_als_ereignis_auf_den_bus() {
        use crate::gespraeche::Channel;
        use crate::name::Channelname;
        let bus = std::sync::Arc::new(Bus::neu(4));
        let mut abonnement = bus.abonnieren();
        let wechsel = ChannelWechsel {
            stand: 7,
            channel: Channel {
                name: Channelname::bilden("allgemein").expect("Name"),
                angelegt: None,
                angelegt_von: None,
                beschreibung: None,
                frist: crate::gespraeche::Frist::Tage(7),
            },
        };

        channels_veroeffentlichen(&bus)(&wechsel);

        assert_eq!(
            tokio::time::timeout(std::time::Duration::from_secs(1), abonnement.naechstes())
                .await
                .ok()
                .flatten(),
            Some(Empfang::Ereignis(Ereignis::Channel(wechsel)))
        );
    }
}
