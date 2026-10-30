//! Direktpaar (A00020 Sichtbarkeit, ADR-0055 Regel 1): die zwei vollen Kennungen eines Direktgesprächs, sortiert —
//! der eine Wert, aus dem Ordnername, Beteiligung und Gegenüber folgen. Rein, ohne Ablage und Uhr.
//!
//! Ein Paar, kein Satz von n Kennungen: Gruppen-Direktgespräche gibt es nicht (PO 167), für Gruppen gibt es Channels.

use super::Kennung;

/// Beide Kennungen sind dieselbe — an sich selbst gibt es kein Gespräch (PO 169, 422 `selbst`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selbst;

/// Zwei verschiedene Kennungen, aufsteigend nach Bytes ihres Textes: `bilden(a, b)` und `bilden(b, a)` sind
/// dasselbe Paar und damit derselbe Ordner.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Direktpaar {
    erste: Kennung,
    zweite: Kennung,
}

/// Der Trenner der beiden Kennungen im Ordnernamen; in einer Kennung kommt er nicht vor (Namensregel, ADR-0013).
const TRENNER: char = '+';

impl Direktpaar {
    /// Das Paar aus zwei Kennungen in beliebiger Reihenfolge; dieselbe Kennung zweimal ist `Selbst`.
    pub fn bilden(a: &Kennung, b: &Kennung) -> Result<Direktpaar, Selbst> {
        match a.als_text().cmp(b.als_text()) {
            std::cmp::Ordering::Equal => Err(Selbst),
            std::cmp::Ordering::Less => Ok(Direktpaar {
                erste: a.clone(),
                zweite: b.clone(),
            }),
            std::cmp::Ordering::Greater => Ok(Direktpaar {
                erste: b.clone(),
                zweite: a.clone(),
            }),
        }
    }

    /// Ob `kennung` eine der zwei ist — Gleichheit der ganzen Kennung, nie ein Teilstück oder nur der Name.
    pub fn beteiligt(&self, kennung: &Kennung) -> bool {
        self.erste == *kennung || self.zweite == *kennung
    }

    /// Die andere Kennung des Paars; für einen Unbeteiligten keine.
    pub fn gegenueber(&self, kennung: &Kennung) -> Option<&Kennung> {
        if self.erste == *kennung {
            Some(&self.zweite)
        } else if self.zweite == *kennung {
            Some(&self.erste)
        } else {
            None
        }
    }

    /// Der Ordnername unter `direkt/`: beide Kennungen mit `+`, jedes `:` einer IPv6-Herkunft als `%3A` (Windows
    /// und Netzlaufwerke vertragen kein `:` im Namen).
    pub fn ordnername(&self) -> String {
        format!("{}{TRENNER}{}", kodiert(&self.erste), kodiert(&self.zweite))
    }

    /// Das Paar zu einem Ordnernamen — nur die Form, die `ordnername` erzeugt: genau ein `+`, je Teil `%3A` für `:`
    /// und kein anderes `%`, je Teil eine kanonische Kennung, die zwei verschieden und aufsteigend. Alles andere
    /// (ein Ordner von Hand, eine Alias-Herkunft, eine zweite Schreibweise desselben Paars) ist `None`.
    pub fn aus_ordnername(name: &str) -> Option<Direktpaar> {
        let (erste, zweite) = name.split_once(TRENNER)?;
        let erste = dekodiert(erste)?;
        let zweite = dekodiert(zweite)?;
        if erste.als_text() >= zweite.als_text() {
            return None;
        }
        let paar = Direktpaar { erste, zweite };
        (paar.ordnername() == name).then_some(paar)
    }
}

fn kodiert(kennung: &Kennung) -> String {
    kennung.als_text().replace(':', "%3A")
}

/// Ein Teil des Ordnernamens als kanonische Kennung; `%3A` ist die einzige zulässige Kodierung.
fn dekodiert(teil: &str) -> Option<Kennung> {
    if teil.replace("%3A", "").contains('%') {
        return None;
    }
    Kennung::kanonisch(&teil.replace("%3A", ":")).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kennung(text: &str) -> Kennung {
        Kennung::bilden(text).expect(text)
    }

    fn paar(a: &str, b: &str) -> Direktpaar {
        Direktpaar::bilden(&kennung(a), &kennung(b)).expect("Paar")
    }

    /// B0764 D1, erster Test: die Reihenfolge der Eingabe ändert den Ordner nicht (Mutante „Sortierung weg“).
    #[test]
    fn beide_reihenfolgen_ergeben_denselben_sortierten_ordner() {
        let vor = paar("ben@10.0.0.8", "anna@10.0.0.7");
        let rueck = paar("anna@10.0.0.7", "ben@10.0.0.8");

        assert_eq!(vor, rueck);
        assert_eq!(vor.ordnername(), "anna@10.0.0.7+ben@10.0.0.8");
        assert_eq!(rueck.ordnername(), "anna@10.0.0.7+ben@10.0.0.8");
    }

    #[test]
    fn dieselbe_kennung_zweimal_ist_selbst() {
        let anna = kennung("anna@10.0.0.7");

        assert_eq!(Direktpaar::bilden(&anna, &anna), Err(Selbst));
    }

    /// Derselbe Name von anderer Herkunft ist ein anderer Teilnehmer, kein `selbst` (ADR-0012).
    #[test]
    fn derselbe_name_von_anderer_herkunft_ist_ein_anderes_gegenueber() {
        let paar = paar("anna@10.0.0.7", "anna@10.0.0.8");

        assert_eq!(paar.ordnername(), "anna@10.0.0.7+anna@10.0.0.8");
    }

    #[test]
    fn ein_doppelpunkt_der_ipv6_herkunft_steht_als_prozent_3a_im_ordnernamen() {
        assert_eq!(
            paar("claude@fd00::7", "anna@10.0.0.7").ordnername(),
            "anna@10.0.0.7+claude@fd00%3A%3A7"
        );
        assert_eq!(
            paar("anna@::1", "ben@127.0.0.1").ordnername(),
            "anna@%3A%3A1+ben@127.0.0.1"
        );
    }

    #[test]
    fn der_ordnername_fuehrt_zum_selben_paar_zurueck() {
        for (a, b) in [
            ("ben@10.0.0.8", "anna@10.0.0.7"),
            ("claude@fd00::7", "anna@10.0.0.7"),
            ("anna@::1", "ben@127.0.0.1"),
            ("anna@fd00::7", "anna@fd00::8"),
        ] {
            let paar = paar(a, b);

            assert_eq!(
                Direktpaar::aus_ordnername(&paar.ordnername()),
                Some(paar),
                "{a} {b}"
            );
        }
    }

    /// Matrix O2 (Unit-Teil): jede Form, die `ordnername` nie erzeugt, ist kein Paar.
    #[test]
    fn ordner_von_hand_und_aliase_sind_kein_paar() {
        for name in [
            "x",
            "",
            "+",
            "anna@10.0.0.7+",
            "+ben@10.0.0.8",
            "a+a",
            "anna@10.0.0.7+anna@10.0.0.7",
            "b@1.1.1.1+a@1.1.1.1",
            "a@1.1.1.1+a@1.1.1.1",
            "anna@10.0.0.7+ben@10.0.0.8+cora@10.0.0.9",
            "anna@fd00%3a%3a7+ben@10.0.0.8",
            "anna@10.0.0.7%2F+ben@10.0.0.8",
            "anna@10.0.0.7+ben@10.0.0.8%2F",
            "anna@10.0.0.7+ben@10.0.0.8%",
            "anna@fd00:0%3A%3A7+ben@10.0.0.8",
            "anna@fd00%3A0%3A%3A7+ben@10.0.0.8",
            "anna@%3A%3Affff%3A127.0.0.2+ben@10.0.0.8",
            "anna@10.0.0.07+ben@10.0.0.8",
            "anna@fd00::7+ben@10.0.0.8",
            "Anna@10.0.0.7+ben@10.0.0.8",
        ] {
            assert_eq!(Direktpaar::aus_ordnername(name), None, "{name:?}");
        }
    }

    /// Matrix O2, Kontrolle: der richtig kodierte Ordner mit IPv6 ist ein Paar.
    #[test]
    fn ein_richtig_kodierter_ordner_mit_ipv6_ist_ein_paar() {
        for name in [
            "anna@fd00%3A%3A7+ben@10.0.0.8",
            "anna@10.0.0.7+claude@fd00%3A%3A7",
            "anna@%3A%3A1+ben@127.0.0.1",
        ] {
            let paar = Direktpaar::aus_ordnername(name).expect(name);

            assert_eq!(paar.ordnername(), name);
        }
    }

    /// Matrix K3, R6: beteiligt ist nur die ganze Kennung — kein Teilstück (`…70`), nicht der Name allein.
    #[test]
    fn beteiligt_ist_nur_die_ganze_kennung_und_das_gegenueber_die_andere() {
        let paar = paar("anna@10.0.0.7", "ben@10.0.0.8");

        assert!(paar.beteiligt(&kennung("anna@10.0.0.7")));
        assert!(paar.beteiligt(&kennung("ben@10.0.0.8")));
        assert!(!paar.beteiligt(&kennung("anna@10.0.0.70")), "Teilstück");
        assert!(!paar.beteiligt(&kennung("nna@10.0.0.7")), "Teilstück");
        assert!(!paar.beteiligt(&kennung("anna@10.0.0.9")), "nur der Name");
        assert!(!paar.beteiligt(&kennung("cora@10.0.0.9")));
        assert_eq!(
            paar.gegenueber(&kennung("anna@10.0.0.7")),
            Some(&kennung("ben@10.0.0.8"))
        );
        assert_eq!(
            paar.gegenueber(&kennung("ben@10.0.0.8")),
            Some(&kennung("anna@10.0.0.7"))
        );
        assert_eq!(paar.gegenueber(&kennung("anna@10.0.0.70")), None);
    }
}
