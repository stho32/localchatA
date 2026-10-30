//! Zitat einer Antwort (ADR-0053 Regel 3): der Bezug aus der Metazeile, aufgelöst beim Lesen über das Original im
//! Index des eigenen Gesprächs — ein lebender Verweis, keine Kopie. Rein.

use super::Kennung;
#[cfg(test)]
use super::Nachricht;
use super::logformat::{self, Antwortbezug};
use super::textanfang::textanfang_aus;
use crate::uhr::Zeit;

/// Das Zitat, wie jede gelieferte Antwort es trägt: `{zeit, von, zustand, textanfang, anhang, geaendert}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Zitat {
    pub zeit: Zeit,
    pub von: Kennung,
    pub zustand: Zitatzustand,
    /// Höchstens 120 Zeichen der aktuellen Fassung; bei `geloescht` und `entfallen` leer.
    pub textanfang: String,
    /// Anzeigename des ersten Anhangs, wenn der Text der Fassung leer ist; sonst keiner.
    pub anhang: Option<String>,
    /// Zeit des letzten Änderungssatzes des Originals.
    pub geaendert: Option<Zeit>,
}

/// Was ein Zitat ohne seine variablen Teile in der Antwort kostet: Schlüssel, zwei Zeiten, längster Zustand (ADR-0039
/// Byte-Budget, ADR-0053 Regel 3).
const ZITAT_GERUEST: &str = r#"{"zeit":"2026-10-30T09:10:03.004+01:00","von":"","zustand":"bearbeitet","textanfang":"","anhang":null,"geaendert":"2026-10-30T09:10:03.004+01:00"}"#;

impl Zitat {
    /// Bytes, die das Zitat im Byte-Budget eines Blocks zählt: Gerüst, Kennung, Textanfang, Anhangname.
    pub fn lieferbytes(&self) -> usize {
        ZITAT_GERUEST.len()
            + self.von.als_text().len()
            + self.textanfang.len()
            + self.anhang.as_ref().map_or(0, String::len)
    }
}

/// Zustand des Originals: unverändert da, bearbeitet, gelöscht oder durch die Frist entfallen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zitatzustand {
    Da,
    Bearbeitet,
    Geloescht,
    Entfallen,
}

impl Zitatzustand {
    pub fn als_text(self) -> &'static str {
        match self {
            Zitatzustand::Da => "da",
            Zitatzustand::Bearbeitet => "bearbeitet",
            Zitatzustand::Geloescht => "geloescht",
            Zitatzustand::Entfallen => "entfallen",
        }
    }
}

/// Das Original im Index, geliehen statt kopiert: sein Satz und, wenn geändert, seine Fassung.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fund<'a> {
    pub satz: &'a [u8],
    pub aenderung: Option<Fundaenderung<'a>>,
}

/// Die Fassung eines geänderten Originals: Zeit des letzten Änderungssatzes und dessen Satz; gelöscht keiner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fundaenderung<'a> {
    pub geaendert: Zeit,
    pub satz: Option<&'a [u8]>,
}

/// Löst den Bezug über den Fund auf (ADR-0053 Regel 3): kein Fund oder einer mit anderer Kennung → `entfallen`;
/// gelöscht → `geloescht` ohne Text und Anhang; bearbeitet → Textanfang aus dem Änderungssatz; sonst `da`. Gelesen
/// wird nur, bis der Textanfang feststeht.
pub fn zitat(bezug: &Antwortbezug, fund: Option<Fund<'_>>) -> Zitat {
    let leer = |zustand, geaendert| Zitat {
        zeit: bezug.zeit,
        von: bezug.von.clone(),
        zustand,
        textanfang: String::new(),
        anhang: None,
        geaendert,
    };
    let Some(fund) =
        fund.filter(|fund| logformat::kopf(fund.satz).is_some_and(|(_, von)| von == bezug.von))
    else {
        return leer(Zitatzustand::Entfallen, None);
    };
    let (zustand, textsatz, geaendert) = match fund.aenderung {
        None => (Zitatzustand::Da, fund.satz, None),
        Some(Fundaenderung {
            geaendert,
            satz: Some(satz),
        }) => (Zitatzustand::Bearbeitet, satz, Some(geaendert)),
        Some(Fundaenderung {
            geaendert,
            satz: None,
        }) => return leer(Zitatzustand::Geloescht, Some(geaendert)),
    };
    let textanfang = textanfang_aus(textzeichen(textsatz));
    let anhang = textanfang
        .is_empty()
        .then(|| erster_anhang(fund.satz))
        .flatten();
    Zitat {
        zeit: bezug.zeit,
        von: bezug.von.clone(),
        zustand,
        textanfang,
        anhang,
        geaendert,
    }
}

/// Die Zeichen des Texts eines Satzes, ohne Kopf, Metazeilen und Einrückung, Zeilen getrennt durch `\n` — faul,
/// ohne den Satz zu kopieren; ungültiges UTF-8 als Ersatzzeichen (wie `logformat::text`).
fn textzeichen(satz: &[u8]) -> impl Iterator<Item = char> + '_ {
    satz.split(|byte| *byte == b'\n')
        .skip(1)
        .filter_map(|zeile| {
            if zeile.is_empty() {
                Some(&zeile[..0])
            } else {
                zeile.strip_prefix(b"  ")
            }
        })
        .flat_map(|zeile| {
            zeile
                .utf8_chunks()
                .flat_map(|stueck| {
                    stueck
                        .valid()
                        .chars()
                        .chain((!stueck.invalid().is_empty()).then_some('\u{fffd}'))
                })
                .chain(std::iter::once('\n'))
        })
}

/// Der Anzeigename des ersten gültigen Anhangs in den Metazeilen des Satzes.
fn erster_anhang(satz: &[u8]) -> Option<String> {
    let kopf_und_metazeilen = satz
        .split(|byte| *byte == b'\n')
        .take_while(|zeile| zeile.starts_with(b"@ ") || zeile.starts_with(b"+ "))
        .map(String::from_utf8_lossy)
        .collect::<Vec<_>>()
        .join("\n");
    logformat::anhaenge_im_satz(&kopf_und_metazeilen)
        .into_iter()
        .find_map(|(_, gelesen)| gelesen.ok())
        .map(|anhang| anhang.name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::DateTime;

    const T1: &str = "2026-10-30T09:10:03.004+01:00";
    const T2: &str = "2026-10-30T09:11:00.000+01:00";

    fn t(text: &str) -> Zeit {
        DateTime::parse_from_rfc3339(text).expect("Zeit")
    }

    fn bezug(von: &str) -> Antwortbezug {
        Antwortbezug {
            zeit: t(T1),
            von: Kennung::bilden(von).expect("Kennung"),
        }
    }

    fn original(text_und_metazeilen: &str) -> String {
        format!("@ {T1} claude@127.0.0.1\n+ art: agent\n{text_und_metazeilen}\n")
    }

    const ANHANG: &str = "+ anhang: aaaaaaaaaaaaaaaaaaaaaaaaab 12 application/pdf - 091003004-1-Bericht.pdf Bericht März.pdf\n";

    /// R00047 B0706 (ADR-0053 Regel 3): ungeändert → `da` mit Textanfang; kein Fund → `entfallen` mit Zeit und
    /// Kennung aus dem Bezug, ohne Text.
    #[test]
    fn ein_vorhandenes_original_ist_da_ein_fehlendes_entfallen() {
        let satz = original("  Frage?\n");
        let fund = Fund {
            satz: satz.as_bytes(),
            aenderung: None,
        };

        let da = zitat(&bezug("claude@127.0.0.1"), Some(fund));
        let entfallen = zitat(&bezug("claude@127.0.0.1"), None);

        assert_eq!(
            da,
            Zitat {
                zeit: t(T1),
                von: Kennung::bilden("claude@127.0.0.1").expect("Kennung"),
                zustand: Zitatzustand::Da,
                textanfang: "Frage?".to_string(),
                anhang: None,
                geaendert: None,
            }
        );
        assert_eq!(
            entfallen,
            Zitat {
                zustand: Zitatzustand::Entfallen,
                textanfang: String::new(),
                ..da
            }
        );
    }

    /// B0706 Mutante Z11: ein Fund mit anderer Kennung als im Bezug (nur von Hand möglich) ist `entfallen` — nie
    /// fremder Text unter falschem Namen.
    #[test]
    fn ein_fund_mit_anderer_kennung_ist_entfallen() {
        let satz = original("  Geheim\n");
        let fund = Fund {
            satz: satz.as_bytes(),
            aenderung: None,
        };

        let gelesen = zitat(&bezug("codex@127.0.0.1"), Some(fund));

        assert_eq!(gelesen.zustand, Zitatzustand::Entfallen);
        assert_eq!(gelesen.textanfang, "");
        assert_eq!(gelesen.von.als_text(), "codex@127.0.0.1");
    }

    /// B0706 Mutanten Z3 und Z4: bearbeitet → Textanfang aus dem Änderungssatz, nie aus dem Satz des Originals,
    /// `geaendert` der Fassung; gelöscht → `geloescht` ohne Text und ohne Anhang.
    #[test]
    fn bearbeitet_liefert_die_neue_fassung_geloescht_nichts() {
        let satz = original("  Frage?\n");
        let aenderungssatz = format!("@ {T2} claude@127.0.0.1\n+ bearbeitet: {T1}\n  Frage!\n\n");
        let bearbeitet = Fund {
            satz: satz.as_bytes(),
            aenderung: Some(Fundaenderung {
                geaendert: t(T2),
                satz: Some(aenderungssatz.as_bytes()),
            }),
        };
        let nur_anhang = original(ANHANG);
        let geloescht = Fund {
            satz: nur_anhang.as_bytes(),
            aenderung: Some(Fundaenderung {
                geaendert: t(T2),
                satz: None,
            }),
        };

        let neu = zitat(&bezug("claude@127.0.0.1"), Some(bearbeitet));
        let weg = zitat(&bezug("claude@127.0.0.1"), Some(geloescht));

        assert_eq!(
            (neu.zustand, neu.textanfang.as_str(), neu.geaendert),
            (Zitatzustand::Bearbeitet, "Frage!", Some(t(T2)))
        );
        assert_eq!(
            (
                weg.zustand,
                weg.textanfang.as_str(),
                weg.anhang,
                weg.geaendert
            ),
            (Zitatzustand::Geloescht, "", None, Some(t(T2)))
        );
    }

    /// B0706 Anhang: Text leer → Anzeigename des ersten Anhangs; mit Text keiner; ein bearbeitetes Original ohne Text
    /// behält den Anhang seines Satzes.
    #[test]
    fn ein_original_nur_aus_anhaengen_nennt_den_ersten_anhang() {
        let nur_anhang = original(ANHANG);
        let mit_text = original(&format!("{ANHANG}  Siehe Anhang\n"));
        let leerer_text = format!("@ {T2} claude@127.0.0.1\n+ bearbeitet: {T1}\n\n");

        let ohne_text = zitat(
            &bezug("claude@127.0.0.1"),
            Some(Fund {
                satz: nur_anhang.as_bytes(),
                aenderung: None,
            }),
        );
        let mit = zitat(
            &bezug("claude@127.0.0.1"),
            Some(Fund {
                satz: mit_text.as_bytes(),
                aenderung: None,
            }),
        );
        let geleert = zitat(
            &bezug("claude@127.0.0.1"),
            Some(Fund {
                satz: mit_text.as_bytes(),
                aenderung: Some(Fundaenderung {
                    geaendert: t(T2),
                    satz: Some(leerer_text.as_bytes()),
                }),
            }),
        );

        assert_eq!(
            (ohne_text.textanfang.as_str(), ohne_text.anhang.as_deref()),
            ("", Some("Bericht März.pdf"))
        );
        assert_eq!(
            (mit.textanfang.as_str(), mit.anhang.as_deref()),
            ("Siehe Anhang", None)
        );
        assert_eq!(
            (
                geleert.zustand,
                geleert.textanfang.as_str(),
                geleert.anhang.as_deref()
            ),
            (Zitatzustand::Bearbeitet, "", Some("Bericht März.pdf"))
        );
    }

    /// B0706: der Textanfang aus dem Satz folgt `logformat::text` — Zeilen zu Leerzeichen, Einrückung weg, ein Text,
    /// der wie eine Metazeile aussieht, bleibt Text; lange Texte werden nach Zeichen gekürzt.
    #[test]
    fn der_textanfang_aus_dem_satz_gleicht_dem_aus_dem_text() {
        let lang = format!("{}\n\n+ anhang: x\n  ä", "ö".repeat(200));
        let satz = logformat::satz(&Nachricht {
            zeit: t(T1),
            von: Kennung::bilden("claude@127.0.0.1").expect("Kennung"),
            art: None,
            text: format!("Zeile eins\n\n  zwei\t+ drei\n{lang}"),
            anhaenge: Vec::new(),
            geaendert: None,
            bezug: None,
            antwort_auf: None,
        });
        let kurz = logformat::satz(&Nachricht {
            zeit: t(T1),
            von: Kennung::bilden("claude@127.0.0.1").expect("Kennung"),
            art: None,
            text: "a\n+ antwort_auf: x\n\nb".to_string(),
            anhaenge: Vec::new(),
            geaendert: None,
            bezug: None,
            antwort_auf: None,
        });

        for satz in [satz, kurz] {
            let gelesen = zitat(
                &bezug("claude@127.0.0.1"),
                Some(Fund {
                    satz: satz.as_bytes(),
                    aenderung: None,
                }),
            );
            assert_eq!(
                gelesen.textanfang,
                super::super::textanfang::textanfang(&logformat::text(&satz))
            );
        }
    }
}
