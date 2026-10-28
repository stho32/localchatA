//! Frist setzen (ADR-0031 Regel 2, A00012 Ergänzung I0018): unter der Sperre des Verzeichnisses.

use super::einstellungen::{self, Einstellungen};
use super::frist::Frist;
use super::logablage::Gespraech;
use super::verzeichnis::Channel;
use super::{Gespraeche, schreibend};
use crate::name::Channelname;

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
}
