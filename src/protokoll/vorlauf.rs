//! Der Vorlauf der Nachrichtenzeit als Episode `WARNING zeit` der Instanz (R00016, ADR-0035 Regel 4 mit Ergänzung;
//! R00022: auch beim Start, ADR-0042 Ergänzung Karte #31 Punkt 3). Sendet und Start melden auf demselben Weg, damit
//! die Zählung „Sendungen im Vorlauf plus das eine beim Start“ eine Episode bleibt.

use super::episoden::{Auftreten, Quelle};
use super::{Bereich, Protokoll, Schluessel, Stufe};
use crate::uhr::{vorlauf_melden, vorlauf_text};

/// Meldet einen Vorlauf über 60 s als Auftreten der Episode `WARNING zeit`; `dauer_ms` der Ende-Zeile ist der
/// Wert des letzten Auftretens. Bis 60 s nichts.
pub fn eintragen(protokoll: &Protokoll, vorlauf_ms: u64) {
    if !vorlauf_melden(vorlauf_ms) {
        return;
    }
    protokoll.episode(vorlauf_auftreten(vorlauf_ms));
}

/// Das Auftreten „Nachrichtenzeit läuft der Systemuhr voraus“: eine Episode der Instanz; `dauer_ms` ist
/// fortlaufend, der Text der Zeilen nennt den Vorlauf in Worten.
pub(crate) fn vorlauf_auftreten(vorlauf_ms: u64) -> Auftreten {
    Auftreten::neu(
        Stufe::Warning,
        Bereich::Zeit,
        "Nachrichtenzeit läuft der Systemuhr voraus",
        Quelle::instanz(),
    )
    .fortlaufend(Schluessel::DauerMs, vorlauf_ms)
    .anzeige(vorlauf_text(vorlauf_ms))
}
