//! Frist setzen und pflegen (ADR-0030, ADR-0031 Regel 2, A00006 und A00012 Ergänzung I0018): Setzen unter der
//! Sperre des Verzeichnisses; Pflegen je Tag unter der Sperre des Gesprächs — erst die Datei, dann der Index.

use super::Direktpaar;
use super::einstellungen::{self, Einstellungen};
use super::frist::{self, DIREKT_FRIST_TAGE, Frist};
use super::logablage::Gespraech;
use super::verlaufsindex::Verlauf;
use super::verzeichnis::{Channel, ChannelWechsel};
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

/// Was gepflegt wurde: ein Channel oder ein Direktgespräch (ADR-0055 Regel 10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pflegegespraech {
    Channel(Channelname),
    Direkt(Direktpaar),
}

impl From<Channelname> for Pflegegespraech {
    fn from(name: Channelname) -> Pflegegespraech {
        Pflegegespraech::Channel(name)
    }
}

/// Was eine Pflege in einem Gespräch tat; nichts zu tun ergibt keinen Bericht.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pflegebericht {
    /// Tagesdateien entfielen: ihre Tage aufsteigend, ihre Nachrichten im Index, die Frist und der Anlass.
    Entfallen {
        gespraech: Pflegegespraech,
        tage: Vec<NaiveDate>,
        nachrichten: usize,
        frist: u32,
        anlass: Pflegeanlass,
    },
    /// Ein Tag ließ sich nicht entfernen; er bleibt in Datei und Index bis zum nächsten Anlass. `ursache` ist der
    /// Text des Plattenfehlers.
    NichtEntfallen {
        gespraech: Pflegegespraech,
        datei: String,
        anlass: Pflegeanlass,
        ursache: String,
    },
    /// Die Anhänge eines Tags ließen sich nicht entfernen (seine Tagesdatei ist weg oder gab es nie); sie bleiben als
    /// Waisen gezählt, jede nächste Pflege versucht es erneut (ADR-0046 Ergänzung „Review #53 B1“ Regel 4).
    AnhaengeNichtEntfernt {
        gespraech: Pflegegespraech,
        tag: NaiveDate,
        anlass: Pflegeanlass,
        ursache: String,
    },
    /// Der Ordner des Channels ließ sich nicht auflisten; nichts entfällt bis zum nächsten Anlass (ADR-0042
    /// Ergänzung R00019 Punkt 1). `ordner` ist `channels/<name>`, `ursache` der Text des Plattenfehlers.
    TageNichtErmittelt {
        gespraech: Pflegegespraech,
        ordner: String,
        anlass: Pflegeanlass,
        ursache: String,
    },
    /// Ein Tag trägt Änderungssätze zu Nachrichten eines Tags, der noch im Index steht (dessen Entfernen scheiterte):
    /// er bleibt in Datei und Index bis zum nächsten Anlass, damit Gelöschtes gelöscht bleibt (ADR-0048 Ergänzung
    /// „Karte #73“ K6). `ziel` ist der ältere Tag, der ihn hält.
    Zurueckgestellt {
        gespraech: Pflegegespraech,
        datei: String,
        ziel: NaiveDate,
    },
    /// Die Einstellungen sind defekt — in diesem Channel entfällt nichts (ADR-0032 Regel 5).
    Gesperrt {
        gespraech: Pflegegespraech,
        datei: String,
        zeile: Option<usize>,
    },
}

/// Was mit einer neuen Frist sofort entfiele (ADR-0031 Regel 2.6): Zahl der Tagesdateien, ihre Sätze im Index und
/// der jüngste entfallende Tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entfaellt {
    pub tage: usize,
    pub nachrichten: usize,
    pub bis: NaiveDate,
}

/// Die Vorschau des Entfallens aus den Tagen der Platte und dem Index; entfiele nichts, nichts.
fn vorschau(
    auf_der_platte: &[NaiveDate],
    verlauf: &Verlauf,
    heute: NaiveDate,
    n: u32,
) -> Option<Entfaellt> {
    let (_, entfallend) = frist::teilen(auf_der_platte, heute, n);
    let bis = entfallend.iter().max().copied()?;
    Some(Entfaellt {
        tage: entfallend.len(),
        nachrichten: verlauf.saetze_vor(frist::aeltester_behaltener(heute, n)),
        bis,
    })
}

impl Gespraeche {
    /// Pflegt jeden Channel nach seiner Frist und jedes Direktgespräch nach der festen Frist von 7 Tagen mit `heute`
    /// (ADR-0030 Regel 1, ADR-0055 Regel 10).
    pub fn pflegen(&self, heute: NaiveDate, anlass: Pflegeanlass) -> Vec<Pflegebericht> {
        let (namen, paare): (Vec<Channelname>, Vec<Direktpaar>) = {
            let verzeichnis = lesend(&self.verzeichnis);
            (
                verzeichnis.channels.keys().cloned().collect(),
                verzeichnis
                    .verlaeufe
                    .keys()
                    .filter_map(|gespraech| match gespraech {
                        Gespraech::Direkt(paar) => Some(paar.clone()),
                        Gespraech::Channel(_) => None,
                    })
                    .collect(),
            )
        };
        let channels = namen
            .iter()
            .flat_map(|name| self.channel_pflegen(name, heute, &anlass));
        let direkt = paare
            .iter()
            .flat_map(|paar| self.direkt_pflegen(paar, heute, &anlass));
        channels.chain(direkt).collect()
    }

    /// Ein Direktgespräch: die feste Frist (PO 166, bis zur Ausbaustufe „Frist wählen“), sonst wie ein Channel.
    fn direkt_pflegen(
        &self,
        paar: &Direktpaar,
        heute: NaiveDate,
        anlass: &Pflegeanlass,
    ) -> Vec<Pflegebericht> {
        let gespraech = Gespraech::Direkt(paar.clone());
        let Some(verlauf) = lesend(&self.verzeichnis)
            .verlaeufe
            .get(&gespraech)
            .map(std::sync::Arc::clone)
        else {
            return Vec::new();
        };
        self.gespraech_pflegen(
            &gespraech,
            Pflegegespraech::Direkt(paar.clone()),
            &verlauf,
            DIREKT_FRIST_TAGE,
            heute,
            anlass,
        )
    }

    /// Ein Channel: Frist und Verlauf lesend nachschlagen, dann wie jedes Gespräch.
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
                    gespraech: name.clone().into(),
                    datei,
                    zeile,
                }];
            }
        };
        self.gespraech_pflegen(
            &gespraech,
            name.clone().into(),
            &verlauf,
            frist_tage,
            heute,
            anlass,
        )
    }

    /// Tage der Platte teilen (ADR-0030 Regel 3) → je entfallendem Tag unter der Sperre des Gesprächs:
    /// zurückstellen, solange er Änderungssätze zu einem Tag im Index trägt (ADR-0048 Ergänzung „Karte #73“ K6),
    /// sonst Datei entfernen, nur bei Erfolg aus dem Index (Regel 4).
    fn gespraech_pflegen(
        &self,
        gespraech: &Gespraech,
        pflegeort: Pflegegespraech,
        verlauf: &std::sync::Arc<std::sync::Mutex<Verlauf>>,
        frist_tage: u32,
        heute: NaiveDate,
        anlass: &Pflegeanlass,
    ) -> Vec<Pflegebericht> {
        let auf_der_platte = match self.ablage.tage(gespraech) {
            Ok(tage) => tage,
            Err(fehler) => {
                return vec![Pflegebericht::TageNichtErmittelt {
                    gespraech: pflegeort,
                    ordner: gespraech.pfad(),
                    anlass: anlass.clone(),
                    ursache: fehler.to_string(),
                }];
            }
        };
        let mit_anhangtagen = self.mit_anhangtagen(gespraech, &auf_der_platte);
        let (_, entfallend) = frist::teilen(&mit_anhangtagen, heute, frist_tage);
        let mut entfallen = Vec::new();
        let mut nachrichten = 0;
        let mut je_tag = Vec::new();
        for tag in entfallend {
            #[cfg(test)]
            self.schranken.passieren(super::schranke::Vorgang::Pflegen);
            let mut verlauf = sperren(verlauf);
            if let Some(ziel) = verlauf.tag_haelt_aenderungen_fuer_bestehende_tage(tag) {
                je_tag.push(Pflegebericht::Zurueckgestellt {
                    gespraech: pflegeort.clone(),
                    datei: gespraech.tagesdatei(tag),
                    ziel,
                });
                continue;
            }
            match self.ablage.tag_entfernen(gespraech, tag) {
                Ok(()) => {
                    if let Some((saetze, bytes)) = verlauf.tag_entfernen(tag) {
                        nachrichten += saetze;
                        let vorher = self.belegung.weg(bytes);
                        self.unter_90_melden(vorher, vorher.saturating_sub(bytes), anlass);
                    }
                    if let Err(ursache) = self.anhaenge_entfallen(gespraech, tag, anlass) {
                        je_tag.push(Pflegebericht::AnhaengeNichtEntfernt {
                            gespraech: pflegeort.clone(),
                            tag,
                            anlass: anlass.clone(),
                            ursache: ursache.to_string(),
                        });
                    }
                    if auf_der_platte.contains(&tag) {
                        entfallen.push(tag);
                    }
                }
                Err(ursache) => je_tag.push(Pflegebericht::NichtEntfallen {
                    gespraech: pflegeort.clone(),
                    datei: gespraech.tagesdatei(tag),
                    anlass: anlass.clone(),
                    ursache: ursache.to_string(),
                }),
            }
        }
        let mut berichte = Vec::new();
        if !entfallen.is_empty() {
            berichte.push(Pflegebericht::Entfallen {
                gespraech: pflegeort.clone(),
                tage: entfallen,
                nachrichten,
                frist: frist_tage,
                anlass: anlass.clone(),
            });
        }
        berichte.extend(je_tag);
        berichte
    }
}

impl Gespraeche {
    /// Nach der Tagesdatei entfallen die Anhänge des Tags (ADR-0046 Regel 9, Ergänzung „Karte #9 Security-Gate“
    /// Regel 4): über Verzeichnisdeskriptoren, nie einem Symlink folgend; erst danach aus dem Verzeichnis und `G −=` die
    /// gewonnenen Abzüge samt Waisen (Befund #67 B2: auch nicht gekürzte Gelöschte). Scheitert das Entfernen, gehen die
    /// Einträge nach `entfallen`, ihre Bytes bleiben als Waisen des Tags in `G`, und jede nächste Pflege versucht es
    /// erneut — der Tag steht weiter in `anhaenge/` (ADR-0046 Ergänzung „Review #53 B1“ Regeln 3 und 4).
    pub(super) fn anhaenge_entfallen(
        &self,
        gespraech: &Gespraech,
        tag: NaiveDate,
        anlass: &Pflegeanlass,
    ) -> std::io::Result<()> {
        if let Err(fehler) = self.anhangablage.tag_entfernen(gespraech, tag) {
            self.anhangverzeichnis.tag_verwaisen(gespraech, tag);
            return Err(fehler);
        }
        let weg = self.anhangverzeichnis.tag_entfernen(gespraech, tag);
        let _ = self.anhangzaehler.gebunden.fetch_update(
            std::sync::atomic::Ordering::SeqCst,
            std::sync::atomic::Ordering::SeqCst,
            |gebunden| Some(gebunden.saturating_sub(weg)),
        );
        self.anhangschwelle_pruefen(match anlass {
            Pflegeanlass::Start => "start",
            Pflegeanlass::Tageswechsel => "tageswechsel",
            Pflegeanlass::FristGesetzt { .. } => "frist_gesetzt",
        });
        Ok(())
    }
}

/// Eine gesetzte Frist: der Channel danach, wenn sie sich geändert hat, die Frist davor, und was die Pflege danach
/// tat (ADR-0031 Regel 2.8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FristGesetzt {
    pub channel: Channel,
    /// Die Frist davor; `None` heißt: derselbe Wert, nichts geschrieben.
    pub geaendert_von: Option<u32>,
    pub berichte: Vec<Pflegebericht>,
}

impl FristGesetzt {
    /// Die tatsächlich entfallenen Tagesdateien und ihre Nachrichten im Index.
    pub fn entfallen(&self) -> (usize, usize) {
        self.berichte
            .iter()
            .filter_map(|bericht| match bericht {
                Pflegebericht::Entfallen {
                    tage, nachrichten, ..
                } => Some((tage.len(), *nachrichten)),
                Pflegebericht::NichtEntfallen { .. }
                | Pflegebericht::Zurueckgestellt { .. }
                | Pflegebericht::AnhaengeNichtEntfernt { .. }
                | Pflegebericht::TageNichtErmittelt { .. }
                | Pflegebericht::Gesperrt { .. } => None,
            })
            .fold(
                (0, 0),
                |(tage, nachrichten), (dazu_tage, dazu_nachrichten)| {
                    (tage + dazu_tage, nachrichten + dazu_nachrichten)
                },
            )
    }
}

/// Warum eine Frist nicht gesetzt wurde; nichts hat sich geändert.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fristfehler {
    /// Den Channel gibt es nicht — unbekannt oder kein gültiger Name.
    Unbekannt,
    /// Die Einstellungsdatei war beim Start defekt oder nicht lesbar; sie wird nie überschrieben.
    EinstellungenDefekt { datei: String },
    /// Mit der neuen Frist entfielen Tage, und es war nicht bestätigt — nichts geändert (ADR-0031 Regel 2.6).
    Bestaetigen(Entfaellt),
    /// Verkürzt ohne Bestätigung, und der Ordner ließ sich nicht auflisten — ohne Vorschau keine Rückfrage, nichts
    /// geändert (ADR-0031 Ergänzung R00019). `ordner` ist `channels/<name>`, `ursache` der Text des Plattenfehlers.
    TageNichtErmittelt { ordner: String, ursache: String },
    /// Die Einstellungen ließen sich nicht schreiben (ADR-0031 Regel 2.7).
    Ablage { datei: String, ursache: String },
}

impl Gespraeche {
    /// Setzt die Frist des Channels `name` (ADR-0031 Regel 2): unbekannt → eintragen unter der Sperre des
    /// Verzeichnisses → bei einer wirksamen Änderung sofort pflegen mit dem Anlass „Frist gesetzt von <von>“.
    /// Meldungen schreibt der Aufrufer (R00017).
    pub fn frist_setzen(
        &self,
        name: &str,
        tage: u32,
        bestaetigt: bool,
        von: &Kennung,
    ) -> Result<FristGesetzt, Fristfehler> {
        let name = Channelname::bilden(name).map_err(|_| Fristfehler::Unbekannt)?;
        let heute = self.uhr.jetzt().date_naive();
        let (channel, vorher) = self.frist_eintragen(&name, tage, bestaetigt, heute)?;
        let berichte = match vorher {
            Some(_) => self.channel_pflegen(
                &name,
                heute,
                &Pflegeanlass::FristGesetzt { von: von.clone() },
            ),
            None => Vec::new(),
        };
        Ok(FristGesetzt {
            channel,
            geaendert_von: vorher,
            berichte,
        })
    }

    /// Unter der Schreibsperre des Verzeichnisses (ADR-0031 Regeln 2.4–2.7): defekt → derselbe Wert (nichts
    /// geschrieben, `None`) → nur beim Verkürzen ohne Bestätigung auflisten und Vorschau, entfiele etwas, abgewiesen
    /// (Ergänzung R00019; Verlängern ist verlustfrei) → Datei schreiben → Frist setzen, `stand` + 1,
    /// Wechsel veröffentlichen, noch unter der Sperre (Regel 4, ADR-0023 Regel 2). Gepflegt wird danach ohne diese
    /// Sperre (Regel 2.8).
    fn frist_eintragen(
        &self,
        name: &Channelname,
        tage: u32,
        bestaetigt: bool,
        heute: NaiveDate,
    ) -> Result<(Channel, Option<u32>), Fristfehler> {
        let mut verzeichnis = schreibend(&self.verzeichnis);
        let gespraech = Gespraech::channel(name.als_text());
        let verlauf = verzeichnis
            .verlaeufe
            .get(&gespraech)
            .cloned()
            .ok_or(Fristfehler::Unbekannt)?;
        let channel = verzeichnis
            .channels
            .get_mut(name)
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
            return Ok((channel.clone(), None));
        }
        if tage < vorher && !bestaetigt {
            let auf_der_platte =
                self.ablage
                    .tage(&gespraech)
                    .map_err(|fehler| Fristfehler::TageNichtErmittelt {
                        ordner: gespraech.pfad(),
                        ursache: fehler.to_string(),
                    })?;
            if let Some(entfaellt) = vorschau(&auf_der_platte, &sperren(&verlauf), heute, tage) {
                return Err(Fristfehler::Bestaetigen(entfaellt));
            }
        }
        self.einstellungen_ablegen(channel, tage)?;
        channel.frist = Frist::Tage(tage);
        let channel = channel.clone();
        verzeichnis.stand += 1;
        (self.veroeffentlichen)(&ChannelWechsel {
            stand: verzeichnis.stand,
            channel: channel.clone(),
        });
        Ok((channel, Some(vorher)))
    }

    /// Schreibt `einstellungen.toml` mit der neuen Frist; die bekannten Schlüssel aus dem Channel, die übrigen aus
    /// den Bytes, die jetzt in der Datei stehen (ADR-0031 Regel 2.7, `.neu` und Umbenennen in der Logablage).
    fn einstellungen_ablegen(&self, channel: &Channel, tage: u32) -> Result<(), Fristfehler> {
        let gespraech = Gespraech::channel(channel.name.als_text());
        let datei = gespraech.einstellungsdatei();
        let ablagefehler = |fehler: std::io::Error| Fristfehler::Ablage {
            datei: datei.clone(),
            ursache: fehler.to_string(),
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

        let gesetzt = gespraeche
            .frist_setzen("allgemein", 30, false, &beta())
            .expect("gesetzt");

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

        let gesetzt = gespraeche
            .frist_setzen("allgemein", 30, false, &beta())
            .expect("gesetzt");

        assert_eq!(gesetzt.geaendert_von, None);
        assert_eq!(gesetzt.channel.frist, Frist::Tage(30));
        assert_eq!(ablage.schreibvorgaenge(), 0);
    }

    #[test]
    fn ohne_einstellungsdatei_entsteht_sie_mit_der_frist() {
        let ablage = Arc::new(Speicherablage::neu());
        ablage.gespraech_anlegen(&Gespraech::channel("leer"));
        let gespraeche = geladen(&ablage);

        let gesetzt = gespraeche
            .frist_setzen("leer", 30, false, &beta())
            .expect("gesetzt");

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
                gespraeche.frist_setzen(name, 30, false, &beta()),
                Err(Fristfehler::Unbekannt),
                "{name}"
            );
        }
        assert_eq!(
            gespraeche.frist_setzen("kaputt", 30, false, &beta()),
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
        let gespraeche = geladen(&ablage);
        assert_eq!(gespraeche.kennzahlen().nachrichten, 2);

        let beim_start = gespraeche.pflegen(tag("2026-10-28"), Pflegeanlass::Start);

        assert_eq!(
            beim_start,
            vec![Pflegebericht::Entfallen {
                gespraech: allgemein().into(),
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
                gespraech: allgemein().into(),
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
                gespraech: Channelname::bilden("kaputt").expect("Name").into(),
                datei: "channels/kaputt/einstellungen.toml".into(),
                zeile: Some(1),
            }]
        );
        assert_eq!(
            crate::gespraeche::Logablage::tage(ablage.as_ref(), &kaputt).expect("Tage"),
            vec![tag("2026-01-01")]
        );
    }

    /// R00008 F0034 „Verkürzen braucht Bestätigung“ und „Grenze der Vorschau“: 20.(2), 21.(3), 22.(1), 28.(1).
    #[test]
    fn vorschau_nennt_tage_saetze_im_index_und_den_juengsten_entfallenden_tag() {
        let tage: Vec<NaiveDate> = ["2026-10-20", "2026-10-21", "2026-10-22", "2026-10-28"]
            .iter()
            .map(|text| tag(text))
            .collect();
        let mut verlauf = Verlauf::default();
        for (tag_text, anzahl) in [
            ("2026-10-20", 2),
            ("2026-10-21", 3),
            ("2026-10-22", 1),
            ("2026-10-28", 1),
        ] {
            for _ in 0..anzahl {
                verlauf.anhaengen(tag(tag_text), &satz(tag_text), false);
            }
        }
        let heute = tag("2026-10-28");

        assert_eq!(
            vorschau(&tage, &verlauf, heute, 7),
            Some(Entfaellt {
                tage: 2,
                nachrichten: 5,
                bis: tag("2026-10-21")
            })
        );
        assert_eq!(
            vorschau(&tage, &verlauf, heute, 8),
            Some(Entfaellt {
                tage: 1,
                nachrichten: 2,
                bis: tag("2026-10-20")
            })
        );
        assert_eq!(
            vorschau(&tage, &verlauf, heute, 1),
            Some(Entfaellt {
                tage: 3,
                nachrichten: 6,
                bis: tag("2026-10-22")
            })
        );
        assert_eq!(vorschau(&tage, &verlauf, heute, 9), None);
        assert_eq!(vorschau(&tage, &verlauf, heute, 30), None);
    }

    fn beta() -> Kennung {
        Kennung::bilden("beta@127.0.0.1").expect("Kennung")
    }

    /// `allgemein` mit Frist 30 und 20.(2), 21.(3), 22.(1), 28.(1) — alle im Index (R00008 F0034).
    fn verkuerzbar() -> (Gespraeche, Arc<Speicherablage>, Gespraech) {
        let ablage = Arc::new(Speicherablage::neu());
        let gespraech = Gespraech::channel("allgemein");
        ablage.einstellungen_setzen(&gespraech, b"frist_tage = 30\n");
        for (tag_text, anzahl) in [
            ("2026-10-20", 2),
            ("2026-10-21", 3),
            ("2026-10-22", 1),
            ("2026-10-28", 1),
        ] {
            ablage.tag_setzen(&gespraech, tag(tag_text), &satz(tag_text).repeat(anzahl));
        }
        let gespraeche = geladen(&ablage);
        assert_eq!(gespraeche.kennzahlen().nachrichten, 7);
        (gespraeche, ablage, gespraech)
    }

    #[test]
    fn verkuerzen_ohne_bestaetigung_nennt_was_entfiele_und_aendert_nichts() {
        let (gespraeche, ablage, gespraech) = verkuerzbar();

        let ergebnis = gespraeche.frist_setzen("allgemein", 7, false, &beta());

        assert_eq!(
            ergebnis,
            Err(Fristfehler::Bestaetigen(Entfaellt {
                tage: 2,
                nachrichten: 5,
                bis: tag("2026-10-21"),
            }))
        );
        assert_eq!(ablage.schreibvorgaenge(), 0);
        assert_eq!(gespraeche.channels().liste[0].frist, Frist::Tage(30));
        assert_eq!(
            crate::gespraeche::Logablage::tage(ablage.as_ref(), &gespraech)
                .expect("Tage")
                .len(),
            4
        );
        assert_eq!(gespraeche.kennzahlen().nachrichten, 7);
    }

    #[test]
    fn bestaetigt_verkuerzen_setzt_und_pflegt_sofort_mit_der_kennung() {
        let (gespraeche, ablage, gespraech) = verkuerzbar();

        let gesetzt = gespraeche
            .frist_setzen("allgemein", 7, true, &beta())
            .expect("gesetzt");

        assert_eq!(gesetzt.channel.frist, Frist::Tage(7));
        assert_eq!(gesetzt.geaendert_von, Some(30));
        assert_eq!(
            gesetzt.berichte,
            vec![Pflegebericht::Entfallen {
                gespraech: allgemein().into(),
                tage: vec![tag("2026-10-20"), tag("2026-10-21")],
                nachrichten: 5,
                frist: 7,
                anlass: Pflegeanlass::FristGesetzt { von: beta() },
            }]
        );
        assert_eq!(gesetzt.entfallen(), (2, 5));
        assert_eq!(
            crate::gespraeche::Logablage::tage(ablage.as_ref(), &gespraech).expect("Tage"),
            vec![tag("2026-10-22"), tag("2026-10-28")]
        );
        assert_eq!(gespraeche.kennzahlen().nachrichten, 2);
    }

    #[test]
    fn verlaengern_und_verkuerzen_ohne_entfallen_brauchen_keine_bestaetigung() {
        let (gespraeche, _, _) = verkuerzbar();

        let verkuerzt = gespraeche
            .frist_setzen("allgemein", 9, false, &beta())
            .expect("nichts entfällt");
        let verlaengert = gespraeche
            .frist_setzen("allgemein", 3650, false, &beta())
            .expect("verlängert");

        assert_eq!(verkuerzt.entfallen(), (0, 0));
        assert!(verkuerzt.berichte.is_empty());
        assert_eq!(verlaengert.channel.frist, Frist::Tage(3650));
        assert_eq!(gespraeche.kennzahlen().nachrichten, 7);
    }

    /// Je veröffentlichtem Wechsel: Nummer, Frist des Channels und ob das Verzeichnis dabei gesperrt war.
    type Aufzeichnung = Arc<std::sync::Mutex<Vec<(u64, Frist, bool)>>>;

    fn mit_aufzeichnung(ablage: &Arc<Speicherablage>) -> (Arc<Gespraeche>, Aufzeichnung) {
        let selbst: Arc<std::sync::OnceLock<std::sync::Weak<Gespraeche>>> = Arc::default();
        let aufgezeichnet: Aufzeichnung = Arc::default();
        let rueckruf: crate::gespraeche::ChannelVeroeffentlichen = {
            let selbst = Arc::clone(&selbst);
            let aufgezeichnet = Arc::clone(&aufgezeichnet);
            Arc::new(move |wechsel: &crate::gespraeche::ChannelWechsel| {
                let gespraeche = selbst
                    .get()
                    .and_then(std::sync::Weak::upgrade)
                    .expect("Gespräche");
                let gesperrt = gespraeche.verzeichnis.try_read().is_err();
                sperren(&aufgezeichnet).push((
                    wechsel.stand,
                    wechsel.channel.frist.clone(),
                    gesperrt,
                ));
            })
        };
        let gespraeche = Arc::new(geladen(ablage).mit_veroeffentlichen(rueckruf));
        selbst
            .set(Arc::downgrade(&gespraeche))
            .expect("einmal gesetzt");
        (gespraeche, aufgezeichnet)
    }

    /// R00008 F0036 (B0191): eine wirksame Änderung zählt `stand` hoch und geht unter der Sperre hinaus; derselbe
    /// Wert, unbekannt, defekt und die Rückfrage nicht.
    #[test]
    fn nur_eine_wirksame_aenderung_wird_als_wechsel_veroeffentlicht() {
        let ablage = Arc::new(Speicherablage::neu());
        ablage.einstellungen_setzen(&Gespraech::channel("allgemein"), b"frist_tage = 30\n");
        ablage.tag_setzen(
            &Gespraech::channel("allgemein"),
            tag("2026-10-20"),
            &satz("2026-10-20"),
        );
        ablage.einstellungen_setzen(&Gespraech::channel("kaputt"), b"frist_tage = 0\n");
        let (gespraeche, aufgezeichnet) = mit_aufzeichnung(&ablage);

        gespraeche
            .frist_setzen("allgemein", 31, false, &beta())
            .expect("gesetzt");
        gespraeche
            .frist_setzen("allgemein", 31, false, &beta())
            .expect("derselbe Wert");
        let _ = gespraeche.frist_setzen("fehlt", 31, false, &beta());
        let _ = gespraeche.frist_setzen("kaputt", 31, false, &beta());
        let _ = gespraeche.frist_setzen("allgemein", 7, false, &beta());
        gespraeche
            .frist_setzen("allgemein", 8, true, &beta())
            .expect("bestätigt");

        assert_eq!(
            *sperren(&aufgezeichnet),
            vec![(1, Frist::Tage(31), true), (2, Frist::Tage(8), true)]
        );
        assert_eq!(gespraeche.channels().stand, 2);
    }

    /// R00008 F0038 „Löschfehler im Index“ (ADR-0030 Regel 4, ADR-0032 Regel 6): der Tag bleibt in Ablage und
    /// Index, der Bericht nennt ihn; die übrigen entfallenden Tage gehen.
    #[test]
    fn ein_gescheitertes_entfernen_laesst_den_tag_in_ablage_und_index_die_uebrigen_gehen() {
        let ablage = Arc::new(Speicherablage::neu());
        let gespraech = Gespraech::channel("allgemein");
        ablage.einstellungen_setzen(&gespraech, b"frist_tage = 30\n");
        for (tag_text, anzahl) in [
            ("2026-10-20", 2),
            ("2026-10-21", 3),
            ("2026-10-22", 1),
            ("2026-10-28", 1),
        ] {
            ablage.tag_setzen(&gespraech, tag(tag_text), &satz(tag_text).repeat(anzahl));
        }
        let gespraeche = geladen(&ablage);
        ablage.entfernen_scheitern(&gespraech, tag("2026-10-21"));

        let gesetzt = gespraeche
            .frist_setzen("allgemein", 7, true, &beta())
            .expect("gesetzt");

        assert_eq!(
            gesetzt.berichte,
            vec![
                Pflegebericht::Entfallen {
                    gespraech: allgemein().into(),
                    tage: vec![tag("2026-10-20")],
                    nachrichten: 2,
                    frist: 7,
                    anlass: Pflegeanlass::FristGesetzt { von: beta() },
                },
                Pflegebericht::NichtEntfallen {
                    gespraech: allgemein().into(),
                    datei: "channels/allgemein/2026-10-21.log".into(),
                    anlass: Pflegeanlass::FristGesetzt { von: beta() },
                    ursache: "Entfernen scheitert (Speicher-Logablage)".into(),
                },
            ]
        );
        assert_eq!(gesetzt.entfallen(), (1, 2));
        assert_eq!(
            crate::gespraeche::Logablage::tage(ablage.as_ref(), &gespraech).expect("Tage"),
            vec![tag("2026-10-21"), tag("2026-10-22"), tag("2026-10-28")]
        );
        assert_eq!(gespraeche.kennzahlen().nachrichten, 3 + 1 + 1);
    }

    /// R00008 F0038 „Schreibfehler“ (ADR-0031 Regel 2.7): Frist, Datei und `stand` bleiben, kein Wechsel.
    #[test]
    fn scheitert_das_schreiben_aendert_sich_nichts_und_nichts_wird_veroeffentlicht() {
        let ablage = Arc::new(Speicherablage::neu());
        ablage.gespraech_anlegen(&Gespraech::channel("allgemein"));
        let (gespraeche, aufgezeichnet) = mit_aufzeichnung(&ablage);
        ablage.einstellungen_scheitern();

        let gescheitert = gespraeche.frist_setzen("allgemein", 30, false, &beta());

        assert_eq!(
            gescheitert,
            Err(Fristfehler::Ablage {
                datei: "channels/allgemein/einstellungen.toml".into(),
                ursache: "Einstellungen schreiben scheitert (Speicher-Logablage)".into(),
            })
        );
        assert_eq!(gespraeche.channels().liste[0].frist, Frist::Tage(7));
        assert_eq!(gespraeche.channels().stand, 0);
        assert!(sperren(&aufgezeichnet).is_empty());
        assert_eq!(einstellungen_von(&ablage, "allgemein"), None);
    }

    /// Fehler je Datei, nie je Lauf (ADR-0032 Regel 6): scheitert der älteste Tag, gehen die jüngeren trotzdem.
    #[test]
    fn nach_einem_gescheiterten_tag_werden_die_folgenden_weiter_gepflegt() {
        let ablage = Arc::new(Speicherablage::neu());
        let gespraech = Gespraech::channel("allgemein");
        for tag_text in ["2026-10-19", "2026-10-20", "2026-10-21", "2026-10-28"] {
            ablage.tag_setzen(&gespraech, tag(tag_text), &satz(tag_text));
        }
        ablage.entfernen_scheitern(&gespraech, tag("2026-10-19"));
        let gespraeche = geladen(&ablage);

        let berichte = gespraeche.pflegen(tag("2026-10-28"), Pflegeanlass::Start);

        assert_eq!(berichte.len(), 2, "{berichte:?}");
        assert!(matches!(
            &berichte[0],
            Pflegebericht::Entfallen { tage, .. } if *tage == vec![tag("2026-10-20"), tag("2026-10-21")]
        ));
        assert_eq!(
            crate::gespraeche::Logablage::tage(ablage.as_ref(), &gespraech).expect("Tage"),
            vec![tag("2026-10-19"), tag("2026-10-28")]
        );
    }

    /// R00019 F0063 „Pflege“ (Speicherablage „Auflisten scheitert“): `allgemein` meldet „Tage nicht ermittelt“ mit
    /// Ordner, Anlass und Plattenfehler, nicht „Tag nicht entfallen“; `zweiter` wird weiter gepflegt.
    #[test]
    fn ein_nicht_auflistbarer_ordner_meldet_tage_nicht_ermittelt_die_uebrigen_werden_gepflegt() {
        let ablage = Arc::new(Speicherablage::neu());
        let gespraech = Gespraech::channel("allgemein");
        let zweiter = Gespraech::channel("zweiter");
        for tag_text in ["2026-10-20", "2026-10-28"] {
            ablage.tag_setzen(&gespraech, tag(tag_text), &satz(tag_text));
        }
        for tag_text in ["2026-10-20", "2026-10-22"] {
            ablage.tag_setzen(&zweiter, tag(tag_text), &satz(tag_text));
        }
        let gespraeche = geladen(&ablage);
        ablage.auflisten_scheitern(&gespraech);

        let berichte = gespraeche.pflegen(tag("2026-10-29"), Pflegeanlass::Tageswechsel);

        assert_eq!(berichte.len(), 2, "{berichte:?}");
        assert_eq!(
            berichte[0],
            Pflegebericht::TageNichtErmittelt {
                gespraech: allgemein().into(),
                ordner: "channels/allgemein".into(),
                anlass: Pflegeanlass::Tageswechsel,
                ursache: "Auflisten scheitert (Speicher-Logablage)".into(),
            }
        );
        assert!(
            matches!(
                &berichte[1],
                Pflegebericht::Entfallen { gespraech, tage, anlass: Pflegeanlass::Tageswechsel, .. }
                    if *gespraech == Pflegegespraech::Channel(Channelname::bilden("zweiter").expect("Name"))
                        && *tage == vec![tag("2026-10-20"), tag("2026-10-22")]
            ),
            "{berichte:?}"
        );
        assert_eq!(
            crate::gespraeche::Logablage::tage(ablage.as_ref(), &zweiter).expect("Tage"),
            Vec::<NaiveDate>::new()
        );
    }

    /// R00050 B0777 (ADR-0055 Regel 10, ändert ADR-0032 Regel 1 „Nur Channels“): ein Direktgespräch entfällt nach der
    /// festen Frist von 7 Tagen — Datei und Index —, der Bericht nennt das Paar, nicht einen Channel; der Tag am Rand
    /// der Frist (heute − 6) bleibt, der davor geht.
    #[test]
    fn ein_direktgespraech_entfaellt_nach_sieben_tagen_aus_ablage_und_index() {
        let ablage = Arc::new(Speicherablage::neu());
        let paar = crate::gespraeche::Direktpaar::fuer_test("a@10.0.0.1", "b@10.0.0.2");
        let direkt = Gespraech::Direkt(paar.clone());
        for tag_text in ["2026-10-23", "2026-10-24", "2026-10-30"] {
            ablage.tag_setzen(&direkt, tag(tag_text), &satz(tag_text));
        }
        let gespraeche = geladen(&ablage);

        let berichte = gespraeche.pflegen(tag("2026-10-30"), Pflegeanlass::Tageswechsel);

        assert_eq!(
            berichte,
            vec![Pflegebericht::Entfallen {
                gespraech: Pflegegespraech::Direkt(paar),
                tage: vec![tag("2026-10-23")],
                nachrichten: 1,
                frist: 7,
                anlass: Pflegeanlass::Tageswechsel,
            }]
        );
        assert_eq!(
            crate::gespraeche::Logablage::tage(ablage.as_ref(), &direkt).expect("Tage"),
            vec![tag("2026-10-24"), tag("2026-10-30")]
        );
    }

    /// R00019 F0063, erster Test aus ADR-0031 Ergänzung R00019 (Speicherablage „Auflisten scheitert“, Frist 30):
    /// Verlängern listet nicht und gelingt; Verkürzen ohne Bestätigung scheitert mit „Tage nicht ermittelt“ und
    /// ändert nichts; bestätigt Verkürzen braucht keine Vorschau und gelingt.
    #[test]
    fn nur_verkuerzen_ohne_bestaetigung_listet_den_ordner_auf() {
        let ablage = Arc::new(Speicherablage::neu());
        let gespraech = Gespraech::channel("allgemein");
        ablage.einstellungen_setzen(&gespraech, b"frist_tage = 30\n");
        ablage.tag_setzen(&gespraech, tag("2026-10-28"), &satz("2026-10-28"));
        let (gespraeche, aufgezeichnet) = mit_aufzeichnung(&ablage);
        ablage.auflisten_scheitern(&gespraech);

        let verlaengert = gespraeche.frist_setzen("allgemein", 60, false, &beta());
        let verkuerzt = gespraeche.frist_setzen("allgemein", 7, false, &beta());
        let stand_danach = gespraeche.channels().stand;
        let frist_danach = gespraeche.channels().liste[0].frist.clone();
        let bestaetigt = gespraeche.frist_setzen("allgemein", 7, true, &beta());

        let verlaengert = verlaengert.expect("verlängert ohne Auflisten");
        assert_eq!(verlaengert.channel.frist, Frist::Tage(60));
        assert_eq!(
            verkuerzt,
            Err(Fristfehler::TageNichtErmittelt {
                ordner: "channels/allgemein".into(),
                ursache: "Auflisten scheitert (Speicher-Logablage)".into(),
            })
        );
        assert_eq!(frist_danach, Frist::Tage(60));
        assert_eq!(stand_danach, 1);
        let bestaetigt = bestaetigt.expect("bestätigt ohne Vorschau");
        assert_eq!(bestaetigt.channel.frist, Frist::Tage(7));
        assert_eq!(bestaetigt.entfallen(), (0, 0));
        assert_eq!(
            *sperren(&aufgezeichnet),
            vec![(1, Frist::Tage(60), true), (2, Frist::Tage(7), true)]
        );
    }
}
