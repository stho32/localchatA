//! Logformat (A00007, ADR-0004): Kopfzeile `@ <zeit> <kennung>`, Metazeilen `+ `, Text mit zwei Leerzeichen.
//!
//! Rein, ohne Platte: übersetzt zwischen Rohtext einer Tagesdatei und Sätzen.

use super::{Kennung, Nachricht};
use crate::uhr::Zeit;
use chrono::DateTime;

/// Ein gültiger Satz im Rohtext: Kopf und Byte-Spanne `anfang..ende` (bis vor den nächsten Satz).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SatzBereich {
    pub zeit: Zeit,
    pub von: Kennung,
    pub anfang: usize,
    pub ende: usize,
}

/// Eine defekte Zeile (1-basiert) — oder die Kopfzeile eines Satzes, der als Ganzes defekt ist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Defekt {
    pub zeile: usize,
}

/// Der Satz einer Nachricht: Kopfzeile, Text mit zwei Leerzeichen eingerückt (leere Zeile bleibt leer),
/// `\r\n` als `\n`, ohne abschließende Leerzeilen, danach eine Leerzeile als Abschluss.
pub fn satz(nachricht: &Nachricht) -> String {
    let mut satz = format!(
        "@ {} {}\n",
        nachricht.zeit.format("%Y-%m-%dT%H:%M:%S%.3f%:z"),
        nachricht.von
    );
    let text = nachricht.text.replace("\r\n", "\n");
    for zeile in text.trim_end_matches('\n').split('\n') {
        if !zeile.is_empty() {
            satz.push_str("  ");
            satz.push_str(zeile);
        }
        satz.push('\n');
    }
    satz.push('\n');
    satz
}

/// Der Text eines Satzes ohne Kopf, Metazeilen und Einrückung (Umkehrung von `satz`).
pub fn text(satz: &str) -> String {
    let zeilen: Vec<&str> = satz
        .split('\n')
        .filter_map(|zeile| match zeilenart(zeile) {
            Zeilenart::Satzinhalt => zeile.strip_prefix("  "),
            Zeilenart::Leer => Some(""),
            Zeilenart::Kopf | Zeilenart::Defekt => None,
        })
        .collect();
    zeilen.join("\n").trim_end_matches('\n').to_string()
}

/// Zerlegt den Rohtext einer Tagesdatei in Sätze und Defekte, in der Reihenfolge, in der sie abgeschlossen werden.
pub fn saetze(rohtext: &str) -> Vec<Result<SatzBereich, Defekt>> {
    let mut ergebnis = Vec::new();
    let mut offen = Offen::Anfang;
    let mut anfang = 0;
    for (index, zeile) in rohtext.split_inclusive('\n').enumerate() {
        let nummer = index + 1;
        let inhalt = zeile.strip_suffix('\n').unwrap_or(zeile);
        match zeilenart(inhalt) {
            Zeilenart::Kopf => {
                offen.schliessen(anfang, &mut ergebnis);
                offen = match kopf_lesen(inhalt) {
                    Some((zeit, von)) => Offen::Satz { zeit, von, anfang },
                    None => {
                        ergebnis.push(Err(Defekt { zeile: nummer }));
                        Offen::DefekterSatz
                    }
                };
            }
            Zeilenart::Satzinhalt if matches!(offen, Offen::Anfang) => {
                ergebnis.push(Err(Defekt { zeile: nummer }));
                offen = Offen::DefekterSatz;
            }
            Zeilenart::Satzinhalt | Zeilenart::Leer => {}
            Zeilenart::Defekt => ergebnis.push(Err(Defekt { zeile: nummer })),
        }
        anfang += zeile.len();
    }
    offen.schliessen(rohtext.len(), &mut ergebnis);
    ergebnis
}

/// Was beim Lesen gerade offen ist: noch kein Satz, ein gültiger oder ein als Ganzes defekter.
enum Offen {
    Anfang,
    Satz {
        zeit: Zeit,
        von: Kennung,
        anfang: usize,
    },
    DefekterSatz,
}

impl Offen {
    fn schliessen(&mut self, ende: usize, ergebnis: &mut Vec<Result<SatzBereich, Defekt>>) {
        if let Offen::Satz { zeit, von, anfang } = std::mem::replace(self, Offen::Anfang) {
            ergebnis.push(Ok(SatzBereich {
                zeit,
                von,
                anfang,
                ende,
            }));
        }
    }
}

enum Zeilenart {
    Kopf,
    Satzinhalt,
    Leer,
    Defekt,
}

fn zeilenart(inhalt: &str) -> Zeilenart {
    if inhalt.starts_with("@ ") {
        Zeilenart::Kopf
    } else if inhalt.starts_with("+ ") || inhalt.starts_with("  ") {
        Zeilenart::Satzinhalt
    } else if inhalt.is_empty() {
        Zeilenart::Leer
    } else {
        Zeilenart::Defekt
    }
}

/// `@ <zeit> <kennung>` mit genau einem Leerzeichen zwischen den Feldern; sonst keiner.
fn kopf_lesen(inhalt: &str) -> Option<(Zeit, Kennung)> {
    let felder = inhalt.strip_prefix("@ ")?;
    let (zeit, von) = felder.split_once(' ')?;
    let zeit = DateTime::parse_from_rfc3339(zeit).ok()?;
    let von = Kennung::bilden(von).ok()?;
    Some((zeit, von))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TAG_27: &str = "@ 2026-10-27T10:00:00.000+01:00 shoff@127.0.0.1\n  Wie weit?\n\nkaputt\n@ 2026-10-27T10:00:05.000+01:00 claude@10.0.0.5\n  Fertig.\n\n";

    fn gueltige(rohtext: &str) -> Vec<SatzBereich> {
        saetze(rohtext).into_iter().filter_map(Result::ok).collect()
    }

    fn defekte(rohtext: &str) -> Vec<usize> {
        saetze(rohtext)
            .into_iter()
            .filter_map(Result::err)
            .map(|defekt| defekt.zeile)
            .collect()
    }

    #[test]
    fn zwei_saetze_und_eine_defekte_zeile_wie_im_ausgangsordner() {
        let saetze = gueltige(TAG_27);

        assert_eq!(saetze.len(), 2);
        assert_eq!(
            saetze[0].zeit,
            DateTime::parse_from_rfc3339("2026-10-27T10:00:00+01:00").expect("Zeit")
        );
        assert_eq!(saetze[0].von.als_text(), "shoff@127.0.0.1");
        assert_eq!(saetze[1].von.als_text(), "claude@10.0.0.5");
        assert_eq!(saetze[0].anfang, 0);
        assert_eq!(saetze[0].ende, saetze[1].anfang);
        assert!(TAG_27[saetze[1].anfang..].starts_with("@ 2026-10-27T10:00:05"));
        assert_eq!(saetze[1].ende, TAG_27.len());
        assert_eq!(defekte(TAG_27), vec![4]);
    }

    #[test]
    fn kopf_mit_unlesbarer_zeit_ist_ein_defekt_mit_seiner_zeile_samt_text() {
        let rohtext = "@ 2026-10-27T14:00:00.000+01:00 a@127.0.0.1\n  eins\n\n@ 2026-10-27T14:00:0@ 2026-10-27T14:00:00.002+01:00 c@127.0.0.1\n  zwei\n\n@ 2026-10-27T14:00:00.003+01:00 b@127.0.0.1\n  drei\n\n";

        let saetze = gueltige(rohtext);
        assert_eq!(saetze.len(), 2);
        assert_eq!(saetze[0].von.als_text(), "a@127.0.0.1");
        assert_eq!(saetze[1].von.als_text(), "b@127.0.0.1");
        assert_eq!(defekte(rohtext), vec![4]);
    }

    #[test]
    fn kopf_mit_ungueltiger_kennung_oder_falscher_feldzahl_ist_ein_defekt() {
        for kopf in [
            "@ 2026-10-27T14:00:00.000+01:00 ohne-at",
            "@ 2026-10-27T14:00:00.000+01:00",
            "@ 2026-10-27T14:00:00.000+01:00 a@b zu viel",
            "@ 2026-10-27T14:00:00.000+01:00  a@b",
        ] {
            let rohtext = format!("{kopf}\n  text\n\n");
            assert!(gueltige(&rohtext).is_empty(), "{kopf}");
            assert_eq!(defekte(&rohtext), vec![1], "{kopf}");
        }
    }

    #[test]
    fn metazeilen_leere_zeilen_und_text_mit_at_gehoeren_zum_satz() {
        let rohtext =
            "@ 2026-10-27T14:00:00.000+01:00 a@b\n+ antwort: x\n  @ kein Kopf\n\n  weiter\n\n";
        let saetze = gueltige(rohtext);
        assert_eq!(saetze.len(), 1);
        assert_eq!(saetze[0].ende, rohtext.len());
        assert!(defekte(rohtext).is_empty());
    }

    fn nachricht(zeit: &str, von: &str, text: &str) -> Nachricht {
        Nachricht {
            zeit: DateTime::parse_from_rfc3339(zeit).expect("Zeit"),
            von: Kennung::bilden(von).expect("Kennung"),
            text: text.to_string(),
        }
    }

    #[test]
    fn satz_hat_kopf_eingerueckten_text_und_leerzeile_genau_wie_im_kriterium() {
        let satz = satz(&nachricht(
            "2026-10-27T14:07:12.345+01:00",
            "claude@10.0.0.5",
            "# Titel\n\n@ kein Kopf\n\tTab\r\nEnde\n\n",
        ));

        assert_eq!(
            satz,
            "@ 2026-10-27T14:07:12.345+01:00 claude@10.0.0.5\n  # Titel\n\n  @ kein Kopf\n  \tTab\n  Ende\n\n"
        );
    }

    #[test]
    fn zeit_steht_immer_mit_millisekunden_und_versatz() {
        let satz = satz(&nachricht("2026-10-27T14:07:12Z", "a@b", "x"));
        assert_eq!(satz, "@ 2026-10-27T14:07:12.000+00:00 a@b\n  x\n\n");
    }

    #[test]
    fn rundweg_satz_saetze_text_ergibt_denselben_text_und_einen_satz() {
        let original = "# Titel\n\n@ kein Kopf\n  eingerückt\n```\ncode\n```";
        let rohtext = satz(&nachricht("2026-10-27T14:07:12.345+01:00", "a@b", original));

        let gelesen = saetze(&rohtext);

        assert_eq!(gelesen.len(), 1);
        let bereich = gelesen[0].clone().expect("gültiger Satz");
        assert_eq!(text(&rohtext[bereich.anfang..bereich.ende]), original);
    }

    #[test]
    fn rundweg_normalisiert_zeilenenden_und_abschliessende_leerzeilen() {
        let rohtext = satz(&nachricht(
            "2026-10-27T14:07:12.345+01:00",
            "a@b",
            "\neins\r\n\r\nzwei\n\n\n",
        ));
        assert_eq!(text(&rohtext), "\neins\n\nzwei");
    }

    #[test]
    fn text_ueberspringt_metazeilen_und_defekte_zeilen() {
        assert_eq!(
            text("@ 2026-10-27T14:00:00.000+01:00 a@b\n+ antwort: x\n  eins\nkaputt\n  zwei\n\n"),
            "eins\nzwei"
        );
    }

    #[test]
    fn leerer_rohtext_hat_weder_saetze_noch_defekte() {
        assert!(saetze("").is_empty());
        assert!(saetze("\n\n").is_empty());
    }

    #[test]
    fn text_vor_dem_ersten_kopf_ist_ein_defekt_an_seiner_ersten_zeile() {
        let rohtext = "\n  verwaist\n+ x: y\n@ 2026-10-27T14:00:00.000+01:00 a@b\n  da\n";
        assert_eq!(gueltige(rohtext).len(), 1);
        assert_eq!(defekte(rohtext), vec![2]);
    }

    #[test]
    fn letzte_zeile_ohne_zeilenende_gehoert_zum_satz() {
        let rohtext = "@ 2026-10-27T14:00:00.000+01:00 a@b\n  Rest";
        let saetze = gueltige(rohtext);
        assert_eq!(saetze.len(), 1);
        assert_eq!(saetze[0].ende, rohtext.len());
    }
}
