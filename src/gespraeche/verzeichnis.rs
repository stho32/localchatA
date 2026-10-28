//! Channelverzeichnis (A00012): die Menge der Channels samt Einstellungen als nummerierter Stand, der sich nur
//! unter der Sperre des Verzeichnisses ändert (ADR-0021, ADR-0023).

use super::Kennung;
use super::einstellungen::Beschreibung;
use super::frist::Frist;
use super::logablage::Gespraech;
use super::verlaufsindex::Verlauf;
use crate::name::{Channelname, Namensfehler};
use crate::uhr::Zeit;
use std::collections::BTreeMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};

/// Ein Channel, wie Liste, Stand und Ereignis ihn zeigen; unbekannte Werte sind `None`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Channel {
    pub name: Channelname,
    pub angelegt: Option<Zeit>,
    pub angelegt_von: Option<Kennung>,
    pub beschreibung: Option<Beschreibung>,
    /// Aufbewahrungsfrist (ADR-0031 Regel 3: in Liste, Stand und Ereignis).
    pub frist: Frist,
}

/// Alle Channels nach Name mit der Nummer des letzten Wechsels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelStand {
    pub stand: u64,
    pub liste: Vec<Channel>,
}

/// Ein neuer Channel mit der Nummer, die er im Stand bekam (ADR-0023 Regel 3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelWechsel {
    pub stand: u64,
    pub channel: Channel,
}

/// Wohin jeder Wechsel geht — aufgerufen unter der Sperre des Verzeichnisses, damit die Reihenfolge der
/// Veröffentlichung die von `stand` ist (ADR-0023 Regel 2). Gespräche kennen den Bus nicht.
pub type ChannelVeroeffentlichen = Arc<dyn Fn(&ChannelWechsel) + Send + Sync>;

/// Warum ein Channel nicht angelegt wurde.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Anlegefehler {
    /// Den Namen gibt es schon — mit dem vorhandenen Channel.
    Vorhanden(Channel),
    /// `channels/<name>/` liegt im Ordner, ist aber nicht geladen (von Hand angelegt, während das Programm
    /// lief); nichts wird überschrieben (ADR-0021 Regel 1).
    OrdnerVorhanden,
    /// Der Name verletzt die Namensregel oder ist reserviert.
    Regel(Namensfehler),
    /// Eine Grenze ist erreicht; nichts wurde angelegt.
    Grenze(Grenze),
    /// Die Ablage ließ sich nicht schreiben; der Text nennt Ort und Grund.
    Ablage(String),
}

/// Höchstens so viele Channels insgesamt, einschließlich der beim Start geladenen (ADR-0021 Regel 3).
pub const GESAMT: usize = 256;
/// Höchstens so viele Channels, deren `angelegt_von` eine Herkunft trägt — über Neustarts (ADR-0021 Regel 4).
/// R00015, Lastprofil „Team-LAN“ (2026-10-28, A00001, ADR-0021 Ergänzung R00015): 50 aktive Channels, im
/// ungünstigen Fall alle von den Agenten eines Rechners angelegt; 128 mit Reserve, gesamt bleibt 256
/// (2 Herkünfte × 128).
pub const JE_HERKUNFT: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grenzart {
    Gesamt,
    Herkunft,
}

/// Eine erreichte Grenze: welche und ihr Wert.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Grenze {
    pub art: Grenzart,
    pub hoechstens: usize,
}

/// Frei oder die erreichte Grenze — zuerst gesamt, dann je Herkunft; Channels ohne bekannten Anleger zählen
/// nur gesamt (ADR-0021 Regeln 4, 5); ohne Herkunft (Kennung ohne Adresse) gilt nur die Gesamtgrenze.
pub(super) fn grenze_pruefen<'a>(
    channels: impl ExactSizeIterator<Item = &'a Channel>,
    herkunft: Option<IpAddr>,
) -> Result<(), Grenze> {
    if channels.len() >= GESAMT {
        return Err(Grenze {
            art: Grenzart::Gesamt,
            hoechstens: GESAMT,
        });
    }
    let von_dieser_herkunft = channels
        .filter_map(|channel| channel.angelegt_von.as_ref())
        .filter(|anleger| herkunft.is_some() && anleger.herkunft() == herkunft)
        .count();
    if von_dieser_herkunft >= JE_HERKUNFT {
        return Err(Grenze {
            art: Grenzart::Herkunft,
            hoechstens: JE_HERKUNFT,
        });
    }
    Ok(())
}

/// Was unter der Sperre steht: Verläufe aller Gespräche, die Channels und die Nummer des letzten Wechsels.
/// Sperrordnung: Verzeichnis vor Gespräch, nie umgekehrt (A00012).
#[derive(Default)]
pub(super) struct Verzeichnis {
    pub(super) verlaeufe: BTreeMap<Gespraech, Arc<Mutex<Verlauf>>>,
    pub(super) channels: BTreeMap<Channelname, Channel>,
    pub(super) stand: u64,
}

impl Verzeichnis {
    pub(super) fn stand(&self) -> ChannelStand {
        ChannelStand {
            stand: self.stand,
            liste: self.channels.values().cloned().collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn channel(name: &str, anleger: Option<&str>) -> Channel {
        Channel {
            name: Channelname::bilden(name).expect("Name"),
            angelegt: None,
            angelegt_von: anleger.map(|text| Kennung::bilden(text).expect("Kennung")),
            beschreibung: None,
            frist: Frist::Tage(7),
        }
    }

    /// `anzahl` Channels der Herkunft `herkunft` (Anleger `a@<herkunft>`), Namen mit Vorsilbe.
    fn von(herkunft: &str, anzahl: usize, vorsilbe: &str) -> Vec<Channel> {
        (0..anzahl)
            .map(|nummer| {
                channel(
                    &format!("{vorsilbe}{nummer}"),
                    Some(&format!("a@{herkunft}")),
                )
            })
            .collect()
    }

    fn pruefen(channels: &[Channel], herkunft: &str) -> Result<(), Grenze> {
        grenze_pruefen(channels.iter(), Some(herkunft.parse().expect("IP")))
    }

    const HERKUNFT: Grenze = Grenze {
        art: Grenzart::Herkunft,
        hoechstens: 128,
    };
    const INSGESAMT: Grenze = Grenze {
        art: Grenzart::Gesamt,
        hoechstens: 256,
    };

    /// R00015 G3: der 128. einer Herkunft ist frei, ab 128 vorhandenen gilt die Grenze.
    #[test]
    fn je_herkunft_ist_der_128_frei_und_ab_128_vorhandenen_die_grenze() {
        assert_eq!(pruefen(&von("10.0.0.1", 127, "c"), "10.0.0.1"), Ok(()));
        assert_eq!(
            pruefen(&von("10.0.0.1", 128, "c"), "10.0.0.1"),
            Err(HERKUNFT)
        );
        assert_eq!(pruefen(&von("10.0.0.1", 128, "c"), "10.0.0.2"), Ok(()));
        assert_eq!(
            pruefen(&von("::ffff:10.0.0.1", 128, "c"), "10.0.0.1"),
            Err(HERKUNFT)
        );
    }

    /// R00015 G3: 2 Herkünfte × 128 = 256 erreichen die Gesamtgrenze.
    #[test]
    fn gesamt_ist_der_256_frei_und_ab_256_vorhandenen_die_grenze_vor_der_herkunft() {
        let mut channels = von("10.0.0.1", 128, "h1-");
        channels.extend(von("10.0.0.2", 127, "h2-"));
        assert_eq!(channels.len(), 255);
        assert_eq!(pruefen(&channels, "10.0.0.9"), Ok(()));
        assert_eq!(pruefen(&channels, "10.0.0.2"), Ok(()));

        channels.push(channel("h2-127", Some("a@10.0.0.2")));

        assert_eq!(pruefen(&channels, "10.0.0.9"), Err(INSGESAMT));
        assert_eq!(pruefen(&channels, "10.0.0.2"), Err(INSGESAMT));
    }

    #[test]
    fn ohne_bekannten_anleger_zaehlt_ein_channel_nur_gesamt() {
        let mut channels: Vec<Channel> = (0..40)
            .map(|nummer| channel(&format!("hand{nummer}"), None))
            .collect();
        channels.push(channel("zahl", Some("a@1")));
        channels.extend(von("10.0.0.1", 127, "c"));

        assert_eq!(pruefen(&channels, "10.0.0.1"), Ok(()));
        assert_eq!(grenze_pruefen(channels.iter(), None), Ok(()));
    }
}
