//! Sichtbarkeit (A00020, ADR-0055 Regel 5): die **eine** Funktion, die entscheidet, wer ein Gespräch sieht. Alle
//! verteilenden Wege fragen sie — Strom, `warten`, Anhänge löschen, später Suche und Laufwerk; ein zweiter Vergleich
//! einer Kennung mit Teilen eines Ordnernamens ist verboten (Architekturtest, B0781).

use super::{Gespraech, Kennung};

/// Ob `kennung` das Gespräch sehen darf: ein Channel jeder mit Sitzung, ein Direktgespräch genau die zwei
/// Kennungen seines Paars, ein archiviertes Gespräch niemand (I0022).
///
/// Die Routen unter `/api/direkt/{kennung}/…` fragen nicht: dort ist das Paar aus Sitzung und Pfad gebildet, ein
/// fremdes Gespräch ist gar nicht nennbar. Jede Verzweigung über `Gespraech` ist vollständig (kein `_ =>`), damit
/// eine neue Gesprächsart nicht still durchgeht.
pub fn sichtbar_fuer(gespraech: &Gespraech, kennung: &Kennung, archiviert: bool) -> bool {
    if archiviert {
        return false;
    }
    match gespraech {
        Gespraech::Channel(_) => true,
        Gespraech::Direkt(paar) => paar.beteiligt(kennung),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gespraeche::Direktpaar;

    fn kennung(text: &str) -> Kennung {
        Kennung::bilden(text).expect(text)
    }

    fn direkt(a: &str, b: &str) -> Gespraech {
        Gespraech::Direkt(Direktpaar::bilden(&kennung(a), &kennung(b)).expect("Paar"))
    }

    /// B0766 D3, erster Test: Channel jeder, Paar genau beide (auch ein IPv6-Beteiligter), archiviert niemand.
    #[test]
    fn channel_fuer_jeden_paar_nur_fuer_die_zwei_archiviert_fuer_niemanden() {
        let paar = direkt("anna@10.0.0.7", "ben@10.0.0.8");
        let ipv6 = direkt("anna@::1", "ben@127.0.0.1");
        let allgemein = Gespraech::channel("allgemein");
        let fall = |gespraech: &Gespraech, wer: &str, archiviert: bool| {
            sichtbar_fuer(gespraech, &kennung(wer), archiviert)
        };

        assert!(fall(&allgemein, "cora@10.0.0.9", false));
        assert!(fall(&paar, "anna@10.0.0.7", false));
        assert!(fall(&paar, "ben@10.0.0.8", false));
        assert!(!fall(&paar, "cora@10.0.0.9", false));
        assert!(!fall(&paar, "anna@10.0.0.70", false), "kein Teilstück");
        assert!(!fall(&paar, "nna@10.0.0.7", false), "kein Teilstück");
        assert!(!fall(&paar, "anna@10.0.0.9", false), "nicht nur der Name");
        assert!(fall(&ipv6, "anna@::1", false), "IPv6-Beteiligter sieht es");
        assert!(fall(&ipv6, "ben@127.0.0.1", false));
        assert!(!fall(&ipv6, "cora@::1", false));
        assert!(!fall(&allgemein, "anna@10.0.0.7", true));
        assert!(!fall(&paar, "anna@10.0.0.7", true));
    }
}
