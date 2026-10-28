//! Channelverzeichnis (A00012): die Menge der Channels samt Einstellungen als nummerierter Stand, der sich nur
//! unter der Sperre des Verzeichnisses ändert (ADR-0021, ADR-0023).

use super::Kennung;
use super::logablage::Gespraech;
use super::verlaufsindex::Verlauf;
use crate::name::{Channelname, Namensfehler};
use crate::uhr::Zeit;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

/// Ein Channel, wie Liste, Stand und Ereignis ihn zeigen; unbekannte Werte sind `None`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Channel {
    pub name: Channelname,
    pub angelegt: Option<Zeit>,
    pub angelegt_von: Option<Kennung>,
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
    /// Der Name verletzt die Namensregel oder ist reserviert.
    Regel(Namensfehler),
    /// Die Ablage ließ sich nicht schreiben; der Text nennt Ort und Grund.
    Ablage(String),
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
