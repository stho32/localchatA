//! Frist setzen und pflegen (ADR-0030, ADR-0031 Regel 2, A00006 und A00012 Ergänzung I0018): Setzen unter der
//! Sperre des Verzeichnisses; Pflegen je Tag unter der Sperre des Gesprächs — erst die Datei, dann der Index.

use super::einstellungen::{self, Einstellungen};
use super::frist::{self, Frist};
use super::logablage::Gespraech;
use super::verzeichnis::Channel;
use super::{Gespraeche, Kennung, lesend, schreibend, sperren};
use crate::name::Channelname;
use chrono::NaiveDate;

/// Warum gepflegt wird (ADR-0030 Regel 2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pflegeanlass {
    Start,
    Tageswechsel,
    FristGesetzt { von: Kennung },
}

/// Was eine Pflege in einem Channel tat; nichts zu tun ergibt keinen Bericht.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pflegebericht {
    /// Tagesdateien entfielen: ihre Tage aufsteigend, ihre Nachrichten im Index, die Frist und der Anlass.
    Entfallen {
        channel: Channelname,
        tage: Vec<NaiveDate>,
        nachrichten: usize,
        frist: u32,
        anlass: Pflegeanlass,
    },
    /// Ein Tag ließ sich nicht entfernen; er bleibt in Datei und Index bis zum nächsten Anlass.
    NichtEntfallen {
        channel: Channelname,
        datei: String,
        grund: String,
    },
    /// Die Einstellungen sind defekt — in diesem Channel entfällt nichts (ADR-0032 Regel 5).
    Gesperrt {
        channel: Channelname,
        datei: String,
        zeile: Option<usize>,
    },
}

impl Gespraeche {
    /// Pflegt jeden Channel nach seiner Frist mit `heute` (ADR-0030 Regel 1): Direktgespräche nie.
    pub fn pflegen(&self, heute: NaiveDate, anlass: Pflegeanlass) -> Vec<Pflegebericht> {
        let namen: Vec<Channelname> = lesend(&self.verzeichnis).channels.keys().cloned().collect();
        namen
            .iter()
            .flat_map(|name| self.channel_pflegen(name, heute, &anlass))
            .collect()
    }

    /// Ein Channel: Frist und Verlauf lesend nachschlagen → Tage der Platte teilen (ADR-0030 Regel 3) → je
    /// entfallendem Tag unter der Sperre des Gesprächs Datei entfernen, nur bei Erfolg aus dem Index (Regel 4).
    fn channel_pflegen(
        &self,
        name: &Channelname,
        heute: NaiveDate,
        anlass: &Pflegeanlass,
    ) -> Vec<Pflegebericht> {
        let gespraech = Gespraech::channel(name.als_text());
        let (frist, verlauf) = {
            let verzeichnis = lesend(&self.verzeichnis);
            let (Some(channel), Some(verlauf)) = (
                verzeichnis.channels.get(name),
                verzeichnis.verlaeufe.get(&gespraech),
            ) else {
                return Vec::new();
            };
            (channel.frist.clone(), std::sync::Arc::clone(verlauf))
        };
        let frist_tage = match frist {
            Frist::Tage(tage) => tage,
            Frist::Gesperrt { datei, zeile } => {
                return vec![Pflegebericht::Gesperrt {
                    channel: name.clone(),
                    datei,
                    zeile,
                }];
            }
        };
        let auf_der_platte = match self.ablage.tage(&gespraech) {
            Ok(tage) => tage,
            Err(fehler) => {
                return vec![Pflegebericht::NichtEntfallen {
                    channel: name.clone(),
                    datei: gespraech.pfad(),
                    grund: fehler.to_string(),
                }];
            }
        };
        let (_, entfallend) = frist::teilen(&auf_der_platte, heute, frist_tage);
        let mut entfallen = Vec::new();
        let mut nachrichten = 0;
        let mut fehler = Vec::new();
        for tag in entfallend {
            let mut verlauf = sperren(&verlauf);
            match self.ablage.tag_entfernen(&gespraech, tag) {
                Ok(()) => {
                    nachrichten += verlauf.tag_entfernen(tag).map_or(0, |(saetze, _)| saetze);
                    entfallen.push(tag);
                }
                Err(grund) => fehler.push(Pflegebericht::NichtEntfallen {
                    channel: name.clone(),
                    datei: gespraech.tagesdatei(tag),
                    grund: grund.to_string(),
                }),
            }
        }
        let mut berichte = Vec::new();
        if !entfallen.is_empty() {
            berichte.push(Pflegebericht::Entfallen {
                channel: name.clone(),
                tage: entfallen,
                nachrichten,
                frist: frist_tage,
                anlass: anlass.clone(),
            });
        }
        berichte.extend(fehler);
        berichte
    }
}

/// Eine gesetzte Frist: der Channel danach und, wenn sie sich geändert hat, die Frist davor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FristGesetzt {
    pub channel: Channel,
    /// Die Frist davor; `None` heißt: derselbe Wert, nichts geschrieben.
    pub geaendert_von: Option<u32>,
}

/// Warum eine Frist nicht gesetzt wurde; nichts hat sich geändert.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fristfehler {
    /// Den Channel gibt es nicht — unbekannt oder kein gültiger Name.
    Unbekannt,
    /// Die Einstellungsdatei war beim Start defekt oder nicht lesbar; sie wird nie überschrieben.
    EinstellungenDefekt { datei: String },
    /// Die Einstellungen ließen sich nicht schreiben (ADR-0031 Regel 2.7).
    Ablage { datei: String, grund: String },
}

impl Gespraeche {
    /// Setzt die Frist des Channels `name` unter der Sperre des Verzeichnisses: unbekannt → defekt → derselbe Wert
    /// (nichts geschrieben) → `einstellungen.toml` geschrieben (übrige Schlüssel aus den gelesenen Bytes) → Frist im
    /// Channel gesetzt.
    pub fn frist_setzen(&self, name: &str, tage: u32) -> Result<FristGesetzt, Fristfehler> {
        let name = Channelname::bilden(name).map_err(|_| Fristfehler::Unbekannt)?;
        let mut verzeichnis = schreibend(&self.verzeichnis);
        let channel = verzeichnis
            .channels
            .get_mut(&name)
            .ok_or(Fristfehler::Unbekannt)?;
        let vorher = match &channel.frist {
            Frist::Gesperrt { datei, .. } => {
                return Err(Fristfehler::EinstellungenDefekt {
                    datei: datei.clone(),
                });
            }
            Frist::Tage(vorher) => *vorher,
        };
        if vorher == tage {
            return Ok(FristGesetzt {
                channel: channel.clone(),
                geaendert_von: None,
            });
        }
        self.einstellungen_ablegen(channel, tage)?;
        channel.frist = Frist::Tage(tage);
        Ok(FristGesetzt {
            channel: channel.clone(),
            geaendert_von: Some(vorher),
        })
    }

    /// Schreibt `einstellungen.toml` mit der neuen Frist; die bekannten Schlüssel aus dem Channel, die übrigen aus
    /// den Bytes, die jetzt in der Datei stehen (ADR-0031 Regel 2.7, `.neu` und Umbenennen in der Logablage).
    fn einstellungen_ablegen(&self, channel: &Channel, tage: u32) -> Result<(), Fristfehler> {
        let gespraech = Gespraech::channel(channel.name.als_text());
        let datei = gespraech.einstellungsdatei();
        let ablagefehler = |fehler: std::io::Error| Fristfehler::Ablage {
            datei: datei.clone(),
            grund: fehler.to_string(),
        };
        let vorher = self
            .ablage
            .einstellungen_lesen(&gespraech)
            .map_err(ablagefehler)?;
        let einstellungen = Einstellungen {
            angelegt: channel.angelegt,
            angelegt_von: channel.angelegt_von.clone(),
            beschreibung: channel.beschreibung.clone(),
            frist_tage: Some(tage),
        };
        let text = einstellungen::schreiben(&einstellungen, vorher.as_deref());
        self.ablage
            .einstellungen_schreiben(&gespraech, text.as_bytes())
            .map_err(ablagefehler)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gespraeche::Speicherablage;
    use crate::uhr::{GestellteUhr, Zeit};
    use std::sync::Arc;

    const ALLGEMEIN: &[u8] = b"angelegt = \"2026-10-27T09:00:00.000+01:00\"\nangelegt_von = \"alpha@127.0.0.2\"\nfarbe = \"blau\"\n";

    fn geladen(ablage: &Arc<Speicherablage>) -> Gespraeche {
        let uhr = Arc::new(GestellteUhr::neu(
            Zeit::parse_from_rfc3339("2026-10-28T12:00:00.000+01:00").expect("Zeit"),
        ));
        Gespraeche::laden(ablage.clone(), uhr).expect("geladen")
    }

    fn einstellungen_von(ablage: &Speicherablage, name: &str) -> Option<Vec<u8>> {
        crate::gespraeche::Logablage::einstellungen_lesen(ablage, &Gespraech::channel(name))
            .expect("lesbar")
    }

    #[test]
    fn setzen_schreibt_die_frist_behaelt_die_uebrigen_schluessel_und_nennt_die_alte() {
        let ablage = Arc::new(Speicherablage::neu());
        ablage.einstellungen_setzen(&Gespraech::channel("allgemein"), ALLGEMEIN);
        let gespraeche = geladen(&ablage);

        let gesetzt = gespraeche.frist_setzen("allgemein", 30).expect("gesetzt");

        assert_eq!(gesetzt.geaendert_von, Some(7));
        assert_eq!(gesetzt.channel.frist, Frist::Tage(30));
        let datei = einstellungen_von(&ablage, "allgemein").expect("Datei");
        let tabelle: toml::Table = String::from_utf8(datei)
            .expect("UTF-8")
            .parse()
            .expect("TOML");
        assert_eq!(tabelle["frist_tage"].as_integer(), Some(30));
        assert_eq!(tabelle["farbe"].as_str(), Some("blau"));
        assert_eq!(tabelle["angelegt_von"].as_str(), Some("alpha@127.0.0.2"));
        assert_eq!(gespraeche.channels().liste[0].frist, Frist::Tage(30));
    }

    #[test]
    fn derselbe_wert_schreibt_nichts_und_aendert_nichts() {
        let ablage = Arc::new(Speicherablage::neu());
        ablage.einstellungen_setzen(&Gespraech::channel("allgemein"), b"frist_tage = 30\n");
        let gespraeche = geladen(&ablage);

        let gesetzt = gespraeche.frist_setzen("allgemein", 30).expect("gesetzt");

        assert_eq!(gesetzt.geaendert_von, None);
        assert_eq!(gesetzt.channel.frist, Frist::Tage(30));
        assert_eq!(ablage.schreibvorgaenge(), 0);
    }

    #[test]
    fn ohne_einstellungsdatei_entsteht_sie_mit_der_frist() {
        let ablage = Arc::new(Speicherablage::neu());
        ablage.gespraech_anlegen(&Gespraech::channel("leer"));
        let gespraeche = geladen(&ablage);

        let gesetzt = gespraeche.frist_setzen("leer", 30).expect("gesetzt");

        assert_eq!(gesetzt.geaendert_von, Some(7));
        assert_eq!(
            einstellungen_von(&ablage, "leer").as_deref(),
            Some(&b"frist_tage = 30\n"[..])
        );
    }

    #[test]
    fn unbekannt_ungueltig_und_defekt_aendern_nichts() {
        let ablage = Arc::new(Speicherablage::neu());
        ablage.einstellungen_setzen(&Gespraech::channel("allgemein"), ALLGEMEIN);
        ablage.einstellungen_setzen(&Gespraech::channel("kaputt"), b"frist_tage = \"sieben\"\n");
        let gespraeche = geladen(&ablage);

        for name in ["fehlt", "Allgemein", "../allgemein", ""] {
            assert_eq!(
                gespraeche.frist_setzen(name, 30),
                Err(Fristfehler::Unbekannt),
                "{name}"
            );
        }
        assert_eq!(
            gespraeche.frist_setzen("kaputt", 30),
            Err(Fristfehler::EinstellungenDefekt {
                datei: "channels/kaputt/einstellungen.toml".into()
            })
        );
        assert_eq!(ablage.schreibvorgaenge(), 0);
        assert_eq!(
            einstellungen_von(&ablage, "kaputt").as_deref(),
            Some(&b"frist_tage = \"sieben\"\n"[..])
        );
        assert_eq!(gespraeche.channels().stand, 0);
    }

    fn tag(text: &str) -> NaiveDate {
        NaiveDate::parse_from_str(text, "%Y-%m-%d").expect("Testtag")
    }

    fn satz(tag: &str) -> String {
        format!("@ {tag}T09:00:00.000+02:00 a@127.0.0.9\n  x\n\n")
    }

    fn allgemein() -> Channelname {
        Channelname::bilden("allgemein").expect("Name")
    }

    /// R00008 F0034 „Pflegen“ und ADR-0030, erster Unit-Test: der Start nimmt nur Dateien (der Index hat die Tage
    /// nicht), der Tageswechsel Datei **und** Index.
    #[test]
    fn pflegen_entfernt_tage_jenseits_der_frist_aus_ablage_und_index() {
        let ablage = Arc::new(Speicherablage::neu());
        let gespraech = Gespraech::channel("allgemein");
        for tag_text in ["2026-10-20", "2026-10-21", "2026-10-22", "2026-10-28"] {
            ablage.tag_setzen(&gespraech, tag(tag_text), &satz(tag_text));
        }
        let direkt = Gespraech::Direkt("a@x+b@y".into());
        ablage.tag_setzen(&direkt, tag("2026-01-01"), &satz("2026-01-01"));
        let gespraeche = geladen(&ablage);
        assert_eq!(gespraeche.kennzahlen().nachrichten, 2);

        let beim_start = gespraeche.pflegen(tag("2026-10-28"), Pflegeanlass::Start);

        assert_eq!(
            beim_start,
            vec![Pflegebericht::Entfallen {
                channel: allgemein(),
                tage: vec![tag("2026-10-20"), tag("2026-10-21")],
                nachrichten: 0,
                frist: 7,
                anlass: Pflegeanlass::Start,
            }]
        );
        assert_eq!(
            crate::gespraeche::Logablage::tage(ablage.as_ref(), &gespraech).expect("Tage"),
            vec![tag("2026-10-22"), tag("2026-10-28")]
        );
        assert_eq!(gespraeche.kennzahlen().nachrichten, 2);

        let am_naechsten_tag = gespraeche.pflegen(tag("2026-10-29"), Pflegeanlass::Tageswechsel);

        assert_eq!(
            am_naechsten_tag,
            vec![Pflegebericht::Entfallen {
                channel: allgemein(),
                tage: vec![tag("2026-10-22")],
                nachrichten: 1,
                frist: 7,
                anlass: Pflegeanlass::Tageswechsel,
            }]
        );
        assert_eq!(gespraeche.kennzahlen().nachrichten, 1);
        assert_eq!(
            crate::gespraeche::Logablage::tage(ablage.as_ref(), &gespraech).expect("Tage"),
            vec![tag("2026-10-28")]
        );
        assert_eq!(
            crate::gespraeche::Logablage::tage(ablage.as_ref(), &direkt).expect("Tage"),
            vec![tag("2026-01-01")]
        );
        let davor = gespraeche
            .verlauf(
                "allgemein",
                &crate::gespraeche::abfrage::Abfrage {
                    abschnitt: crate::gespraeche::abfrage::Abschnitt::Vor(
                        Zeit::parse_from_rfc3339("2026-10-28T09:00:00.000+02:00").expect("Zeit"),
                    ),
                    n: crate::gespraeche::abfrage::Blockgroesse::bilden(100).expect("n"),
                },
                usize::MAX,
            )
            .expect("bekannt");
        assert!(davor.nachrichten.is_empty() && !davor.aelter);
        assert!(
            gespraeche
                .pflegen(tag("2026-10-29"), Pflegeanlass::Tageswechsel)
                .is_empty()
        );
    }

    /// ADR-0032 Regel 5: defekt heißt nichts — ein Bericht „gesperrt“ je Pflege, keine Datei geht.
    #[test]
    fn defekte_einstellungen_sperren_das_entfallen() {
        let ablage = Arc::new(Speicherablage::neu());
        let kaputt = Gespraech::channel("kaputt");
        ablage.einstellungen_setzen(&kaputt, b"frist_tage = 0\n");
        ablage.tag_setzen(&kaputt, tag("2026-01-01"), &satz("2026-01-01"));
        let gespraeche = geladen(&ablage);

        let berichte = gespraeche.pflegen(tag("2026-10-28"), Pflegeanlass::Start);

        assert_eq!(
            berichte,
            vec![Pflegebericht::Gesperrt {
                channel: Channelname::bilden("kaputt").expect("Name"),
                datei: "channels/kaputt/einstellungen.toml".into(),
                zeile: Some(1),
            }]
        );
        assert_eq!(
            crate::gespraeche::Logablage::tage(ablage.as_ref(), &kaputt).expect("Tage"),
            vec![tag("2026-01-01")]
        );
    }
}
