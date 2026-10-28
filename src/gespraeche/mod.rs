//! Gespräche: einziger Schreiber des Ordners (A00006) mit Logformat, Logablage und Verlaufsindex.

pub mod einstellungen;
mod frist;
mod kennung;
mod logablage;
pub mod logformat;
mod speicherablage;
mod verlaufsindex;
mod verzeichnis;

pub use kennung::Kennung;
pub use logablage::{Angehaengt, Dateiablage, Gespraech, Logablage};
pub use speicherablage::Speicherablage;
pub use verzeichnis::{Anlegefehler, Channel, ChannelStand};

use crate::fehler::Fehler;
use crate::name::Channelname;
use crate::uhr::{Uhr, Zeit};
use chrono::NaiveDate;
use std::fmt;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use verlaufsindex::Verlauf;
use verzeichnis::Verzeichnis;

/// Eine Nachricht, wie sie im Log steht; die Zeit ist zugleich ihre ID (ADR-0005).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nachricht {
    pub zeit: Zeit,
    pub von: Kennung,
    pub text: String,
}

/// Alle Gespräche des Ordners mit ihrem Verlauf in der Frist.
pub struct Gespraeche {
    ablage: Arc<dyn Logablage>,
    uhr: Arc<dyn Uhr>,
    verzeichnis: RwLock<Verzeichnis>,
    letzte: Mutex<Option<Zeit>>,
    defekte: Vec<Ladedefekt>,
    ladezeit: Duration,
}

/// Was beim Laden übersprungen wurde: eine defekte Zeile oder eine nicht lesbare Tagesdatei.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ladedefekt {
    /// Tagesdatei relativ zum Ordner.
    pub datei: String,
    pub art: Defektart,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Defektart {
    Zeile(usize),
    Unlesbar(String),
}

impl fmt::Display for Ladedefekt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.art {
            Defektart::Zeile(zeile) => {
                write!(f, "{}:{zeile}: defekte Zeile übersprungen", self.datei)
            }
            Defektart::Unlesbar(grund) => {
                write!(
                    f,
                    "{}: nicht lesbar, Tag übersprungen ({grund})",
                    self.datei
                )
            }
        }
    }
}

/// Kennzahlen für den Health-Check (R00002).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Kennzahlen {
    pub nachrichten: u64,
    pub log_bytes: u64,
    pub ladezeit_ms: u64,
    pub defekte_zeilen: u64,
}

impl Gespraeche {
    /// Lädt je Gespräch die Tage in der Frist; Defekte werden übersprungen und gesammelt, nicht abgebrochen.
    pub fn laden(ablage: Arc<dyn Logablage>, uhr: Arc<dyn Uhr>) -> Result<Gespraeche, Fehler> {
        let beginn = Instant::now();
        let heute = uhr.jetzt().date_naive();
        let mut geladen = Geladen::default();
        let gespraeche = ablage
            .gespraeche()
            .map_err(|fehler| Fehler::Ablage(format!("Gespräche nicht lesbar: {fehler}")))?;
        for gespraech in gespraeche {
            let tage = ablage.tage(&gespraech).map_err(|fehler| {
                Fehler::Ablage(format!("Tage von {gespraech} nicht lesbar: {fehler}"))
            })?;
            let mut verlauf = Verlauf::default();
            for tag in frist::tage_in_frist(&tage, heute, frist::STANDARD_FRIST_TAGE) {
                geladen.tag_laden(ablage.as_ref(), &gespraech, tag, &mut verlauf);
            }
            geladen.channel_aufnehmen(&gespraech);
            geladen
                .verzeichnis
                .verlaeufe
                .insert(gespraech, Arc::new(Mutex::new(verlauf)));
        }
        Ok(Gespraeche {
            ablage,
            uhr,
            verzeichnis: RwLock::new(geladen.verzeichnis),
            letzte: Mutex::new(geladen.juengste),
            defekte: geladen.defekte,
            ladezeit: beginn.elapsed(),
        })
    }

    /// Hängt eine Nachricht an die Tagesdatei des lokalen Datums ihrer Zeit, unter der Sperre des Gesprächs:
    /// Zeit vergeben → Satz → anhängen → erst dann Index (ADR-0005, ADR-0011). Scheitert das Anhängen,
    /// bleibt der Index unverändert und die vergebene Zeit verfällt. Angelegt wird nichts.
    pub fn senden(
        &self,
        gespraech: &Gespraech,
        von: &Kennung,
        text: &str,
    ) -> Result<Nachricht, Fehler> {
        let verlauf = lesend(&self.verzeichnis)
            .verlaeufe
            .get(gespraech)
            .cloned()
            .ok_or_else(|| Fehler::Unbekannt(format!("Gespräch {gespraech} unbekannt")))?;
        let mut verlauf = sperren(&verlauf);
        let nachricht = Nachricht {
            zeit: self.zeit_vergeben(),
            von: von.clone(),
            text: text.to_string(),
        };
        let satz = logformat::satz(&nachricht);
        let tag = nachricht.zeit.date_naive();
        let angehaengt = self
            .ablage
            .anhaengen(gespraech, tag, &satz)
            .map_err(|fehler| {
                Fehler::Ablage(format!(
                    "{} nicht geschrieben: {fehler}",
                    gespraech.tagesdatei(tag)
                ))
            })?;
        verlauf.anhaengen(tag, &satz, angehaengt.zeilenende_ergaenzt);
        Ok(nachricht)
    }

    /// `max(jetzt, letzte + 1 ms)`, global über alle Gespräche; die Zeit gilt als vergeben, auch wenn das
    /// Anhängen danach scheitert (Zeiten sind streng steigend, nicht lückenlos).
    fn zeit_vergeben(&self) -> Zeit {
        let mut letzte = sperren(&self.letzte);
        let zeit = crate::uhr::naechste_zeit(self.uhr.jetzt(), *letzte);
        *letzte = Some(zeit);
        zeit
    }

    /// Die jüngste Nachrichtenzeit im Ordner (als Zeitpunkt); an ihr setzt die nächste vergebene Zeit an.
    pub fn letzte_zeit(&self) -> Option<Zeit> {
        *sperren(&self.letzte)
    }

    pub fn defekte(&self) -> &[Ladedefekt] {
        &self.defekte
    }

    pub fn kennzahlen(&self) -> Kennzahlen {
        let mut nachrichten = 0;
        let mut log_bytes = 0;
        for verlauf in lesend(&self.verzeichnis).verlaeufe.values() {
            let verlauf = sperren(verlauf);
            nachrichten += verlauf.nachrichten();
            log_bytes += verlauf.bytes();
        }
        Kennzahlen {
            nachrichten,
            log_bytes,
            ladezeit_ms: u64::try_from(self.ladezeit.as_millis()).unwrap_or(u64::MAX),
            defekte_zeilen: self.defekte.len() as u64,
        }
    }
}

impl Gespraeche {
    /// Legt einen Channel an, unter der Sperre des Verzeichnisses (ADR-0021 Regel 5): Namensregel → vorhanden →
    /// Ordner → Einstellungen → eintragen, `stand` + 1.
    pub fn channel_anlegen(&self, name: &str, von: &Kennung) -> Result<Channel, Anlegefehler> {
        let name = Channelname::bilden(name).map_err(Anlegefehler::Regel)?;
        let mut verzeichnis = schreibend(&self.verzeichnis);
        if let Some(vorhanden) = verzeichnis.channels.get(&name) {
            return Err(Anlegefehler::Vorhanden(vorhanden.clone()));
        }
        let gespraech = Gespraech::channel(name.als_text());
        let channel = Channel {
            name,
            angelegt: Some(crate::uhr::auf_millisekunden(self.uhr.jetzt())),
            angelegt_von: Some(von.clone()),
        };
        self.ablegen(&gespraech, &channel)?;
        verzeichnis
            .verlaeufe
            .insert(gespraech, Arc::new(Mutex::new(Verlauf::default())));
        verzeichnis
            .channels
            .insert(channel.name.clone(), channel.clone());
        verzeichnis.stand += 1;
        Ok(channel)
    }

    /// Ordner und Einstellungsdatei des neuen Channels (ADR-0020 Regel 5, ADR-0022 Regel 2).
    fn ablegen(&self, gespraech: &Gespraech, channel: &Channel) -> Result<(), Anlegefehler> {
        let einstellungen = einstellungen::Einstellungen {
            angelegt: channel.angelegt,
            angelegt_von: channel.angelegt_von.clone(),
        };
        let text = einstellungen::schreiben(&einstellungen, None);
        self.ablage
            .channel_ordner_anlegen(gespraech)
            .map_err(|fehler| Anlegefehler::Ablage(format!("{gespraech}: {fehler}")))?;
        self.ablage
            .einstellungen_schreiben(gespraech, text.as_bytes())
            .map_err(|fehler| {
                Anlegefehler::Ablage(format!("{}: {fehler}", gespraech.einstellungsdatei()))
            })
    }

    /// Alle Channels nach Name mit `stand`.
    pub fn channels(&self) -> ChannelStand {
        lesend(&self.verzeichnis).stand()
    }
}

/// Was das Laden sammelt, bis alle Gespräche durch sind.
#[derive(Default)]
struct Geladen {
    verzeichnis: Verzeichnis,
    defekte: Vec<Ladedefekt>,
    juengste: Option<Zeit>,
}

impl Geladen {
    /// Ein Ordner unter `channels/` mit gültigem Namen ist ein Channel.
    fn channel_aufnehmen(&mut self, gespraech: &Gespraech) {
        let Gespraech::Channel(ordner) = gespraech else {
            return;
        };
        if let Ok(name) = Channelname::bilden(ordner) {
            self.verzeichnis.channels.insert(
                name.clone(),
                Channel {
                    name,
                    angelegt: None,
                    angelegt_von: None,
                },
            );
        }
    }

    /// Liest einen Tag, nimmt seine gültigen Sätze in den Verlauf und merkt Defekte und jüngste Zeit.
    fn tag_laden(
        &mut self,
        ablage: &dyn Logablage,
        gespraech: &Gespraech,
        tag: NaiveDate,
        verlauf: &mut Verlauf,
    ) {
        let datei = gespraech.tagesdatei(tag);
        let rohtext = match ablage.lesen(gespraech, tag) {
            Ok(rohtext) => rohtext,
            Err(fehler) => {
                self.defekte.push(Ladedefekt {
                    datei,
                    art: Defektart::Unlesbar(fehler.to_string()),
                });
                return;
            }
        };
        let mut satzanfaenge = Vec::new();
        for gelesen in logformat::saetze(&rohtext) {
            match gelesen {
                Ok(satz) => {
                    satzanfaenge.push(satz.anfang);
                    self.juengste = self.juengste.max(Some(satz.zeit));
                }
                Err(defekt) => self.defekte.push(Ladedefekt {
                    datei: datei.clone(),
                    art: Defektart::Zeile(defekt.zeile),
                }),
            }
        }
        verlauf.tag_aufnehmen(tag, rohtext, satzanfaenge);
    }
}

/// Lesend: wie `sperren` — das Verzeichnis wird erst nach erfolgreichem Schreiben verändert.
fn lesend<T>(sperre: &RwLock<T>) -> std::sync::RwLockReadGuard<'_, T> {
    sperre
        .read()
        .unwrap_or_else(|vergiftet| vergiftet.into_inner())
}

fn schreibend<T>(sperre: &RwLock<T>) -> std::sync::RwLockWriteGuard<'_, T> {
    sperre
        .write()
        .unwrap_or_else(|vergiftet| vergiftet.into_inner())
}

/// Eine vergiftete Sperre heißt nur, dass ein anderer Aufruf abbrach; der Verlauf selbst bleibt gültig,
/// weil er erst nach erfolgreichem Schreiben verändert wird.
fn sperren<T>(sperre: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    sperre
        .lock()
        .unwrap_or_else(|vergiftet| vergiftet.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uhr::GestellteUhr;
    use chrono::DateTime;
    use std::fs;
    use std::path::Path;
    use std::sync::Arc;

    const TAG_26: &str = "@ 2026-10-26T09:00:00.000+01:00 shoff@127.0.0.1\n  Guten Morgen\n\n@ 2026-10-26T09:01:00.000+01:00 claude@10.0.0.5\n  Scan läuft.\n\n";
    const TAG_27: &str = "@ 2026-10-27T10:00:00.000+01:00 shoff@127.0.0.1\n  Wie weit?\n\nkaputt\n@ 2026-10-27T10:00:05.000+01:00 claude@10.0.0.5\n  Fertig.\n\n";
    const DIREKT: &str = "@ 2026-10-27T11:00:00.000+01:00 claude@10.0.0.5\n  privat\n\n";

    fn zeit(text: &str) -> Zeit {
        DateTime::parse_from_rfc3339(text).expect("Testzeit")
    }

    fn datei(ordner: &Path, pfad: &str, inhalt: &[u8]) {
        let ziel = ordner.join(pfad);
        fs::create_dir_all(ziel.parent().expect("Elternordner")).expect("Ordner");
        fs::write(ziel, inhalt).expect("Datei");
    }

    fn laden_um(ordner: &Path, jetzt: &str) -> Gespraeche {
        Gespraeche::laden(
            Arc::new(Dateiablage::neu(ordner)),
            Arc::new(GestellteUhr::neu(zeit(jetzt))),
        )
        .expect("geladen")
    }

    /// Speicher-Logablage mit dem leeren Channel `allgemein` und eine Uhr, beides für den Test zugänglich.
    fn im_speicher(jetzt: &str) -> (Gespraeche, Arc<Speicherablage>, Arc<GestellteUhr>) {
        let ablage = Arc::new(Speicherablage::neu());
        ablage.gespraech_anlegen(&Gespraech::channel("allgemein"));
        let uhr = Arc::new(GestellteUhr::neu(zeit(jetzt)));
        let gespraeche = Gespraeche::laden(ablage.clone(), uhr.clone()).expect("geladen");
        (gespraeche, ablage, uhr)
    }

    fn claude() -> Kennung {
        Kennung::bilden("claude@10.0.0.5").expect("Kennung")
    }

    fn tag(text: &str) -> NaiveDate {
        NaiveDate::parse_from_str(text, "%Y-%m-%d").expect("Testtag")
    }

    #[test]
    fn senden_haengt_den_satz_an_den_tag_der_zeit_und_ergaenzt_erst_dann_den_index() {
        let (gespraeche, ablage, _) = im_speicher("2026-10-27T14:07:12.345+01:00");
        let allgemein = Gespraech::channel("allgemein");

        let nachricht = gespraeche
            .senden(&allgemein, &claude(), "Hallo")
            .expect("gesendet");

        assert_eq!(nachricht.zeit, zeit("2026-10-27T14:07:12.345+01:00"));
        assert_eq!(nachricht.von, claude());
        assert_eq!(nachricht.text, "Hallo");
        let erwartet = "@ 2026-10-27T14:07:12.345+01:00 claude@10.0.0.5\n  Hallo\n\n";
        assert_eq!(
            ablage.inhalt(&allgemein, tag("2026-10-27")).as_deref(),
            Some(erwartet)
        );
        let kennzahlen = gespraeche.kennzahlen();
        assert_eq!(kennzahlen.nachrichten, 1);
        assert_eq!(kennzahlen.log_bytes, erwartet.len() as u64);
        assert_eq!(gespraeche.letzte_zeit(), Some(nachricht.zeit));
    }

    #[test]
    fn zwei_nachrichten_in_derselben_millisekunde_folgen_mit_einer_millisekunde_abstand() {
        let (gespraeche, _, _) = im_speicher("2026-10-27T14:07:12.345+01:00");
        let allgemein = Gespraech::channel("allgemein");

        let erste = gespraeche
            .senden(&allgemein, &claude(), "a")
            .expect("erste");
        let zweite = gespraeche
            .senden(&allgemein, &claude(), "b")
            .expect("zweite");

        assert_eq!(zweite.zeit - erste.zeit, chrono::Duration::milliseconds(1));
    }

    #[test]
    fn unbekanntes_gespraech_ist_ein_fehler_und_legt_nichts_an() {
        let (gespraeche, ablage, _) = im_speicher("2026-10-27T14:07:12.345+01:00");

        let ergebnis = gespraeche.senden(&Gespraech::channel("gibtsnicht"), &claude(), "x");

        assert!(
            matches!(ergebnis, Err(Fehler::Unbekannt(ref text)) if text.contains("gibtsnicht")),
            "{ergebnis:?}"
        );
        assert_eq!(
            ablage.gespraeche().expect("Gespräche"),
            vec![Gespraech::channel("allgemein")]
        );
        assert_eq!(gespraeche.kennzahlen().nachrichten, 0);
        assert_eq!(gespraeche.letzte_zeit(), None);
    }

    #[test]
    fn schreibfehler_laesst_inhalt_und_index_unveraendert_und_die_zeit_verfaellt() {
        let (gespraeche, ablage, _) = im_speicher("2026-10-27T14:07:12.345+01:00");
        let allgemein = Gespraech::channel("allgemein");
        gespraeche
            .senden(&allgemein, &claude(), "eins")
            .expect("erste");
        let vorher = ablage.inhalt(&allgemein, tag("2026-10-27")).expect("Tag");
        ablage.fehler_nach(20);

        let gescheitert = gespraeche.senden(&allgemein, &claude(), "zwei");

        assert!(
            matches!(gescheitert, Err(Fehler::Ablage(_))),
            "{gescheitert:?}"
        );
        assert_eq!(
            ablage.inhalt(&allgemein, tag("2026-10-27")),
            Some(vorher.clone())
        );
        assert_eq!(gespraeche.kennzahlen().nachrichten, 1);
        assert_eq!(gespraeche.kennzahlen().log_bytes, vorher.len() as u64);
        let dritte = gespraeche
            .senden(&allgemein, &claude(), "drei")
            .expect("dritte");
        assert_eq!(dritte.zeit, zeit("2026-10-27T14:07:12.347+01:00"));
    }

    #[test]
    fn die_zeit_setzt_beim_juengsten_satz_im_ganzen_ordner_fort() {
        let ablage = Arc::new(Speicherablage::neu());
        let a = Gespraech::channel("a");
        ablage.gespraech_anlegen(&a);
        ablage.tag_setzen(
            &Gespraech::channel("b"),
            tag("2026-10-27"),
            "@ 2026-10-27T14:00:00.000+01:00 shoff@127.0.0.1\n  vorher\n\n",
        );
        let uhr = Arc::new(GestellteUhr::neu(zeit("2026-10-27T13:00:00.000+01:00")));
        let gespraeche = Gespraeche::laden(ablage, uhr).expect("geladen");

        let erste = gespraeche.senden(&a, &claude(), "x").expect("gesendet");

        assert_eq!(erste.zeit.to_rfc3339(), "2026-10-27T14:00:00.001+01:00");
    }

    #[test]
    fn die_tagesdatei_folgt_dem_datum_der_vergebenen_zeit() {
        let (gespraeche, ablage, uhr) = im_speicher("2026-10-27T23:59:59.999+01:00");
        let allgemein = Gespraech::channel("allgemein");
        gespraeche
            .senden(&allgemein, &claude(), "spät")
            .expect("erste");
        uhr.stellen(zeit("2026-10-28T00:00:00.000+01:00"));

        gespraeche
            .senden(&allgemein, &claude(), "früh")
            .expect("zweite");

        assert_eq!(
            ablage.tage(&allgemein).expect("Tage"),
            vec![tag("2026-10-27"), tag("2026-10-28")]
        );
        assert_eq!(gespraeche.kennzahlen().nachrichten, 2);
    }

    #[test]
    fn laedt_die_tage_in_der_frist_aller_gespraeche_mit_kennzahlen_defekten_und_juengster_zeit() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let ordner = wurzel.path();
        datei(
            ordner,
            "channels/allgemein/2026-10-26.log",
            TAG_26.as_bytes(),
        );
        datei(
            ordner,
            "channels/allgemein/2026-10-27.log",
            TAG_27.as_bytes(),
        );
        datei(
            ordner,
            "channels/allgemein/2026-10-20.log",
            TAG_26.as_bytes(),
        );
        datei(
            ordner,
            "direkt/claude@10.0.0.5+shoff@127.0.0.1/2026-10-27.log",
            DIREKT.as_bytes(),
        );

        let gespraeche = laden_um(ordner, "2026-10-27T12:00:00+01:00");

        let kennzahlen = gespraeche.kennzahlen();
        assert_eq!(kennzahlen.nachrichten, 5);
        assert_eq!(kennzahlen.log_bytes, (128 + 127 + DIREKT.len()) as u64);
        assert_eq!(kennzahlen.defekte_zeilen, 1);
        assert_eq!(
            gespraeche
                .defekte()
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            vec!["channels/allgemein/2026-10-27.log:4: defekte Zeile übersprungen"]
        );
        assert_eq!(
            gespraeche.letzte_zeit(),
            Some(zeit("2026-10-27T11:00:00+01:00"))
        );
        assert!(
            lesend(&gespraeche.verzeichnis)
                .verlaeufe
                .contains_key(&Gespraech::channel("allgemein"))
        );
    }

    #[test]
    fn juengste_zeit_wird_als_zeitpunkt_verglichen_nicht_als_zeichenkette() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        datei(
            wurzel.path(),
            "channels/a/2026-10-25.log",
            b"@ 2026-10-25T02:59:59.999+02:00 a@b\n  vor\n\n@ 2026-10-25T02:00:00.000+01:00 a@b\n  nach\n\n",
        );

        datei(
            wurzel.path(),
            "channels/b/2026-10-25.log",
            b"@ 2026-10-25T01:30:00.000+01:00 a@b\n  frueher, aber zuletzt geladen\n\n",
        );

        let gespraeche = laden_um(wurzel.path(), "2026-10-25T12:00:00+01:00");

        assert_eq!(
            gespraeche.letzte_zeit(),
            Some(zeit("2026-10-25T02:00:00.000+01:00"))
        );
    }

    #[test]
    fn heute_ist_das_datum_der_uhr_samt_versatz() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let ordner = wurzel.path();
        datei(ordner, "channels/a/2026-10-21.log", TAG_26.as_bytes());
        datei(ordner, "channels/a/2026-10-22.log", TAG_26.as_bytes());

        assert_eq!(
            laden_um(ordner, "2026-10-27T12:00:00+01:00")
                .kennzahlen()
                .nachrichten,
            4
        );
        assert_eq!(
            laden_um(ordner, "2026-10-28T00:30:00+01:00")
                .kennzahlen()
                .nachrichten,
            2
        );
    }

    /// Nicht lesbar heißt: das Dateisystem verweigert die Datei (hier per Rechte); ungültiges UTF-8 ist
    /// kein Lesefehler, sondern ein Defekt seiner Zeile (Code-Review B1).
    #[cfg(unix)]
    #[test]
    fn nicht_lesbare_tagesdatei_ist_ein_defekt_und_der_rest_wird_geladen() {
        use std::os::unix::fs::PermissionsExt;
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let ordner = wurzel.path();
        datei(ordner, "channels/a/2026-10-26.log", TAG_26.as_bytes());
        let gesperrt = ordner.join("channels/a/2026-10-26.log");
        fs::set_permissions(&gesperrt, fs::Permissions::from_mode(0o000)).expect("Rechte");
        datei(ordner, "channels/a/2026-10-27.log", DIREKT.as_bytes());

        let gespraeche = laden_um(ordner, "2026-10-27T12:00:00+01:00");
        fs::set_permissions(&gesperrt, fs::Permissions::from_mode(0o644)).expect("Rechte");

        assert_eq!(gespraeche.kennzahlen().nachrichten, 1);
        assert_eq!(gespraeche.kennzahlen().defekte_zeilen, 1);
        assert_eq!(gespraeche.defekte()[0].datei, "channels/a/2026-10-26.log");
        assert!(matches!(
            gespraeche.defekte()[0].art,
            Defektart::Unlesbar(_)
        ));
    }

    #[test]
    fn ungueltiges_utf8_ist_ein_zeilendefekt_der_tag_bleibt_mit_rohbytes_und_juengster_zeit() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let rohtext: &[u8] = b"@ 2026-10-27T10:00:00.000+01:00 a@b\n  eins\n\n@ 2026-10-27T10:00:06.000+01:00 c@d\n  Scan l\xC3";
        datei(wurzel.path(), "channels/a/2026-10-27.log", rohtext);

        let gespraeche = laden_um(wurzel.path(), "2026-10-27T12:00:00+01:00");

        let kennzahlen = gespraeche.kennzahlen();
        assert_eq!(kennzahlen.nachrichten, 2);
        assert_eq!(kennzahlen.log_bytes, rohtext.len() as u64);
        assert_eq!(
            gespraeche.defekte(),
            &[Ladedefekt {
                datei: "channels/a/2026-10-27.log".into(),
                art: Defektart::Zeile(5),
            }]
        );
        assert_eq!(
            gespraeche.letzte_zeit(),
            Some(zeit("2026-10-27T10:00:06+01:00"))
        );
    }

    #[test]
    fn leerer_ordner_hat_kennzahlen_null_ausser_der_ladezeit() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");

        let gespraeche = laden_um(wurzel.path(), "2026-10-27T12:00:00+01:00");

        let kennzahlen = gespraeche.kennzahlen();
        assert_eq!(
            (
                kennzahlen.nachrichten,
                kennzahlen.log_bytes,
                kennzahlen.defekte_zeilen
            ),
            (0, 0, 0)
        );
        assert!(gespraeche.defekte().is_empty());
        assert_eq!(gespraeche.letzte_zeit(), None);
    }

    fn alpha() -> Kennung {
        Kennung::bilden("alpha@127.0.0.2").expect("Kennung")
    }

    fn leer_im_speicher(jetzt: &str) -> (Gespraeche, Arc<Speicherablage>) {
        let ablage = Arc::new(Speicherablage::neu());
        let uhr = Arc::new(GestellteUhr::neu(zeit(jetzt)));
        let gespraeche = Gespraeche::laden(ablage.clone(), uhr).expect("geladen");
        (gespraeche, ablage)
    }

    #[test]
    fn anlegen_traegt_serverzeit_und_anleger_ein_und_zaehlt_den_stand() {
        let (gespraeche, ablage) = leer_im_speicher("2026-10-28T10:15:00.123456+01:00");

        let channel = gespraeche
            .channel_anlegen("allgemein", &alpha())
            .expect("angelegt");

        assert_eq!(channel.name.als_text(), "allgemein");
        assert_eq!(
            channel.angelegt,
            Some(zeit("2026-10-28T10:15:00.123+01:00"))
        );
        assert_eq!(channel.angelegt_von, Some(alpha()));
        assert_eq!(
            gespraeche.channels(),
            ChannelStand {
                stand: 1,
                liste: vec![channel.clone()],
            }
        );
        let allgemein = Gespraech::channel("allgemein");
        let datei = ablage
            .einstellungen_lesen(&allgemein)
            .expect("lesbar")
            .expect("Einstellungsdatei");
        assert_eq!(
            einstellungen::lesen(&datei),
            Ok(einstellungen::Einstellungen {
                angelegt: channel.angelegt,
                angelegt_von: channel.angelegt_von.clone(),
            })
        );
        gespraeche
            .senden(&allgemein, &claude(), "erste Nachricht")
            .expect("im neuen Channel gesendet");
    }

    #[test]
    fn derselbe_name_noch_einmal_ist_vorhanden_ohne_schreibvorgang_und_ohne_neuen_stand() {
        let (gespraeche, ablage) = leer_im_speicher("2026-10-28T10:15:00.123+01:00");
        let erster = gespraeche
            .channel_anlegen("allgemein", &alpha())
            .expect("angelegt");
        let geschrieben = ablage.schreibvorgaenge();

        let zweiter = gespraeche.channel_anlegen("allgemein", &claude());

        assert_eq!(zweiter, Err(Anlegefehler::Vorhanden(erster)));
        assert_eq!(ablage.schreibvorgaenge(), geschrieben);
        assert_eq!(gespraeche.channels().stand, 1);
    }

    #[test]
    fn ungueltiger_name_ist_ein_regelverstoss_und_legt_nichts_an() {
        let (gespraeche, ablage) = leer_im_speicher("2026-10-28T10:15:00.123+01:00");

        let ergebnis = gespraeche.channel_anlegen("Allgemein", &alpha());

        let Err(Anlegefehler::Regel(fehler)) = ergebnis else {
            panic!("{ergebnis:?}");
        };
        assert_eq!(
            fehler.vorschlag.map(|name| name.to_string()),
            Some("allgemein".to_string())
        );
        assert_eq!(ablage.schreibvorgaenge(), 0);
        assert_eq!(gespraeche.channels().stand, 0);
    }

    #[test]
    fn die_liste_steht_nach_name_nicht_nach_anlage() {
        let (gespraeche, _) = leer_im_speicher("2026-10-28T10:15:00.123+01:00");
        for name in ["security-entwicklung", "allgemein", "b"] {
            gespraeche
                .channel_anlegen(name, &alpha())
                .expect("angelegt");
        }

        let stand = gespraeche.channels();

        assert_eq!(stand.stand, 3);
        assert_eq!(
            stand
                .liste
                .iter()
                .map(|channel| channel.name.to_string())
                .collect::<Vec<_>>(),
            vec!["allgemein", "b", "security-entwicklung"]
        );
    }
}
