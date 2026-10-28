//! Teilnehmerstand und Wechsel (B0076, ADR-0016 Regel 3 und 5): die Liste als `Stand` mit laufender Nummer,
//! jede Statusänderung als `Wechsel` mit größerer Nummer — der Empfänger ordnet danach, nicht nach Ankunft.

use super::status::Status;
use super::wunsch::Art;
use crate::gespraeche::Kennung;
use crate::name::Name;
use crate::uhr::Zeit;
use std::net::IpAddr;

/// Ein Teilnehmer, wie ihn alle anderen sehen: wer, woher, welche Art, welcher Status seit wann — und bei
/// `offline`, bis wann die Sitzung noch lebt (ADR-0019 Regel 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Teilnehmerzeile {
    pub kennung: Kennung,
    pub name: Name,
    pub herkunft: IpAddr,
    pub art: Art,
    pub status: Status,
    pub seit: Zeit,
    /// Bei `offline` der letzte Augenblick, in dem die Sitzung noch lebt (letztes Lebenszeichen +
    /// Gnadenfrist); sonst keiner.
    pub weg_um: Option<Zeit>,
}

/// Eine Statusänderung; `stand` wächst unter der Sperre der Anwesenheit mit jedem Wechsel. `jetzt` ist die
/// Serverzeit, unter der Sperre gelesen und nie fallend (ADR-0019 Regel 2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wechsel {
    pub stand: u64,
    pub jetzt: Zeit,
    pub teilnehmer: Teilnehmerzeile,
}

/// Alle lebenden Sitzungen nach Kennung sortiert, mit dem Stand des zuletzt gemeldeten Wechsels und der
/// Serverzeit, zu der die Liste galt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stand {
    pub stand: u64,
    pub jetzt: Zeit,
    pub liste: Vec<Teilnehmerzeile>,
}
