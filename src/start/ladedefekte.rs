//! Ladedefekte als Einträge (ADR-0042 Regel 2, `laden`): je Datei höchstens 10 einzeln, dann eine Zeile
//! „und 1 weiterer Defekt …“ bzw. „und n weitere Defekte …“. Rein.

use crate::gespraeche::{Defektart, Ladedefekt};
use crate::protokoll::{Bereich, Eintrag, Kontext, Schluessel, Stufe};

/// So viele Defekte einer Datei stehen einzeln im Protokoll.
const JE_DATEI_EINZELN: usize = 10;

/// Die Einträge zu den Defekten, in ihrer Reihenfolge; Defekte derselben Datei über 10 als eine Zeile.
pub fn eintraege(defekte: &[Ladedefekt]) -> Vec<Eintrag> {
    let mut eintraege = Vec::new();
    let mut dateien: Vec<(&str, usize)> = Vec::new();
    for defekt in defekte {
        let gezaehlt = match dateien.iter_mut().find(|(datei, _)| *datei == defekt.datei) {
            Some((_, anzahl)) => {
                *anzahl += 1;
                *anzahl
            }
            None => {
                dateien.push((&defekt.datei, 1));
                1
            }
        };
        if gezaehlt <= JE_DATEI_EINZELN {
            eintraege.push(eintrag(defekt));
        }
    }
    for (datei, anzahl) in dateien {
        if anzahl > JE_DATEI_EINZELN {
            eintraege.push(warnung(
                restzeile(anzahl - JE_DATEI_EINZELN),
                Kontext::neu()
                    .mit(Schluessel::Datei, datei)
                    .mit(Schluessel::Anzahl, anzahl - JE_DATEI_EINZELN),
            ));
        }
    }
    eintraege
}

/// „und 1 weiterer Defekt …“ in der Einzahl, ab zwei „und n weitere Defekte …“ (R00014, Rechtschreib-Review #24).
fn restzeile(weitere: usize) -> String {
    if weitere == 1 {
        "und 1 weiterer Defekt in dieser Datei".to_string()
    } else {
        format!("und {weitere} weitere Defekte in dieser Datei")
    }
}

fn eintrag(defekt: &Ladedefekt) -> Eintrag {
    let datei = Kontext::neu().mit(Schluessel::Datei, &defekt.datei);
    match &defekt.art {
        Defektart::Zeile(zeile) => warnung(
            "Defekte Zeile übersprungen",
            datei.mit(Schluessel::Zeile, zeile),
        ),
        Defektart::Unlesbar(grund) => warnung(
            "Tagesdatei nicht lesbar, Tag übersprungen",
            datei.mit(Schluessel::Grund, grund),
        ),
        Defektart::KeinChannelname => {
            warnung("Kein gültiger Channelname, Ordner übergangen", datei)
        }
        Defektart::Einstellungen { zeile, grund } => warnung(
            "Einstellungen nicht lesbar, als unbekannt gelesen",
            datei
                .mit(Schluessel::Zeile, zeile)
                .mit(Schluessel::Grund, grund),
        ),
        Defektart::EinstellungenUnlesbar(grund) => warnung(
            "Einstellungen nicht lesbar, als unbekannt gelesen",
            datei.mit(Schluessel::Grund, grund),
        ),
    }
}

fn warnung(text: impl Into<String>, kontext: Kontext) -> Eintrag {
    Eintrag::neu(Stufe::Warning, Bereich::Laden, text, kontext)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zeile(datei: &str, zeile: usize) -> Ladedefekt {
        Ladedefekt {
            datei: datei.to_string(),
            art: Defektart::Zeile(zeile),
        }
    }

    #[test]
    fn dreizehn_defekte_einer_datei_ergeben_zehn_einzelne_und_und_drei_weitere() {
        let mut defekte: Vec<Ladedefekt> = (1..=13).map(|n| zeile("channels/a/x.log", n)).collect();
        defekte.push(zeile("channels/b/y.log", 7));

        let eintraege = eintraege(&defekte);

        assert_eq!(eintraege.len(), 12);
        assert_eq!(eintraege[9].kontext.wert(Schluessel::Zeile), Some("10"));
        assert_eq!(
            eintraege[10].kontext.wert(Schluessel::Datei),
            Some("channels/b/y.log")
        );
        let rest = &eintraege[11];
        assert_eq!(rest.text, "und 3 weitere Defekte in dieser Datei");
        assert_eq!(
            rest.kontext.wert(Schluessel::Datei),
            Some("channels/a/x.log")
        );
        assert!(
            eintraege
                .iter()
                .all(|e| e.stufe == Stufe::Warning && e.bereich == Bereich::Laden)
        );
    }

    fn restzeile_bei(anzahl: usize) -> String {
        let defekte: Vec<Ladedefekt> = (1..=anzahl).map(|n| zeile("channels/a/x.log", n)).collect();
        eintraege(&defekte).last().expect("Restzeile").text.clone()
    }

    #[test]
    fn elf_defekte_ergeben_einen_weiteren_defekt_in_der_einzahl() {
        assert_eq!(restzeile_bei(11), "und 1 weiterer Defekt in dieser Datei");
    }

    #[test]
    fn ab_zwei_weiteren_defekten_steht_die_mehrzahl() {
        assert_eq!(restzeile_bei(12), "und 2 weitere Defekte in dieser Datei");
        assert_eq!(restzeile_bei(13), "und 3 weitere Defekte in dieser Datei");
    }

    #[test]
    fn zehn_defekte_ergeben_keine_sammelzeile() {
        let defekte: Vec<Ladedefekt> = (1..=10).map(|n| zeile("channels/a/x.log", n)).collect();
        assert_eq!(eintraege(&defekte).len(), 10);
    }
}
