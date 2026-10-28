//! Logablage (A00008, ADR-0003): Gespräche und ihre Tage als Dateien im Ordner.
//!
//! Kennt das Logformat nicht — liest und schreibt Sätze als fertigen Text.

use crate::protokoll::{Bereich, Kontext, Protokoll, Schluessel, Stufe};
use chrono::NaiveDate;
use std::fmt;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Ein Gespräch ist ein Ordner unter `channels/<slug>` oder `direkt/<name>`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Gespraech {
    Channel(String),
    Direkt(String),
}

impl Gespraech {
    pub fn channel(slug: &str) -> Gespraech {
        Gespraech::Channel(slug.to_string())
    }

    /// Pfad relativ zum Ordner, immer mit `/` getrennt.
    pub fn pfad(&self) -> String {
        match self {
            Gespraech::Channel(slug) => format!("channels/{slug}"),
            Gespraech::Direkt(name) => format!("direkt/{name}"),
        }
    }

    /// Einstellungsdatei relativ zum Ordner, z. B. `channels/allgemein/einstellungen.toml` (ADR-0022).
    pub fn einstellungsdatei(&self) -> String {
        format!("{}/{EINSTELLUNGEN}", self.pfad())
    }

    /// Tagesdatei relativ zum Ordner, z. B. `channels/allgemein/2026-10-27.log`.
    pub fn tagesdatei(&self, tag: NaiveDate) -> String {
        format!("{}/{}.log", self.pfad(), tag.format("%Y-%m-%d"))
    }
}

impl fmt::Display for Gespraech {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.pfad())
    }
}

/// Port zur Ablage; Implementierungen: Dateisystem (Produktion) und Speicher (Tests).
pub trait Logablage: Send + Sync {
    fn gespraeche(&self) -> io::Result<Vec<Gespraech>>;
    fn tage(&self, gespraech: &Gespraech) -> io::Result<Vec<NaiveDate>>;
    /// Die Bytes der Tagesdatei, ungeprüft — ob sie gültiges UTF-8 sind, entscheidet das Logformat je Zeile.
    fn lesen(&self, gespraech: &Gespraech, tag: NaiveDate) -> io::Result<Vec<u8>>;

    /// Hängt einen fertigen Satz in einem Schreibaufruf an die Tagesdatei (neuer Tag → neue Datei).
    /// Endet die Datei nicht auf `\n`, wird zuerst `\n` geschrieben (Zeilenende-Wächter, ADR-0011).
    fn anhaengen(
        &self,
        gespraech: &Gespraech,
        tag: NaiveDate,
        satz: &str,
    ) -> io::Result<Angehaengt>;

    /// Legt den Ordner des Gesprächs an (`channels/` bei Bedarf mit); gibt es ihn schon, scheitert es mit
    /// `AlreadyExists` und lässt ihn unberührt (ADR-0020 Regel 5).
    fn channel_ordner_anlegen(&self, gespraech: &Gespraech) -> io::Result<()>;

    /// Schreibt die Einstellungen nie halb: erst `einstellungen.toml.neu`, dann Umbenennen (ADR-0022 Regel 2).
    fn einstellungen_schreiben(&self, gespraech: &Gespraech, bytes: &[u8]) -> io::Result<()>;

    /// Die Bytes der Einstellungsdatei; fehlt sie, `None`.
    fn einstellungen_lesen(&self, gespraech: &Gespraech) -> io::Result<Option<Vec<u8>>>;

    /// Entfernt den Ordner des Gesprächs samt Inhalt — nur für den Rückbau eines eben angelegten.
    fn ordner_entfernen(&self, gespraech: &Gespraech) -> io::Result<()>;

    /// Entfernt die Tagesdatei des Tages; der Pfad entsteht aus Gespräch und Datum (ADR-0032 Regel 3). Fehlt sie
    /// schon, gilt sie als entfernt.
    fn tag_entfernen(&self, gespraech: &Gespraech, tag: NaiveDate) -> io::Result<()>;
}

const EINSTELLUNGEN: &str = "einstellungen.toml";
const EINSTELLUNGEN_NEU: &str = "einstellungen.toml.neu";

/// Was beim Anhängen geschrieben wurde: der Satz, davor womöglich ein ergänztes `\n`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Angehaengt {
    pub zeilenende_ergaenzt: bool,
}

/// Die Ablage im Ordner der Instanz.
pub struct Dateiablage {
    ordner: PathBuf,
    protokoll: Option<Arc<Protokoll>>,
}

impl Dateiablage {
    pub fn neu(ordner: &Path) -> Dateiablage {
        Dateiablage {
            ordner: ordner.to_path_buf(),
            protokoll: None,
        }
    }

    /// Ein gescheiterter Rückbau steht dann als ERROR im Betriebsprotokoll (ADR-0042 Regel 2, `senden`).
    pub fn mit_protokoll(mut self, protokoll: Arc<Protokoll>) -> Dateiablage {
        self.protokoll = Some(protokoll);
        self
    }
}

impl Logablage for Dateiablage {
    fn gespraeche(&self) -> io::Result<Vec<Gespraech>> {
        let mut gespraeche = Vec::new();
        for name in unterordner(&self.ordner.join("channels"))? {
            gespraeche.push(Gespraech::Channel(name));
        }
        for name in unterordner(&self.ordner.join("direkt"))? {
            gespraeche.push(Gespraech::Direkt(name));
        }
        gespraeche.sort();
        Ok(gespraeche)
    }

    fn tage(&self, gespraech: &Gespraech) -> io::Result<Vec<NaiveDate>> {
        let mut tage = Vec::new();
        for eintrag in std::fs::read_dir(self.ordner.join(gespraech.pfad()))? {
            let eintrag = eintrag?;
            if !eintrag.file_type()?.is_file() {
                continue;
            }
            if let Some(tag) = eintrag.file_name().to_str().and_then(tag_aus_dateiname) {
                tage.push(tag);
            }
        }
        tage.sort();
        Ok(tage)
    }

    fn lesen(&self, gespraech: &Gespraech, tag: NaiveDate) -> io::Result<Vec<u8>> {
        std::fs::read(self.ordner.join(gespraech.tagesdatei(tag)))
    }

    fn anhaengen(
        &self,
        gespraech: &Gespraech,
        tag: NaiveDate,
        satz: &str,
    ) -> io::Result<Angehaengt> {
        let tagesdatei = gespraech.tagesdatei(tag);
        let mut datei = std::fs::OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(self.ordner.join(&tagesdatei))?;
        let laenge = datei.metadata()?.len();
        let zeilenende_ergaenzt = !endet_mit_zeilenende(&mut datei, laenge)?;
        let mut bytes = Vec::with_capacity(satz.len() + 1);
        if zeilenende_ergaenzt {
            bytes.push(b'\n');
        }
        bytes.extend_from_slice(satz.as_bytes());
        if let Err(fehler) = datei.write_all(&bytes) {
            if let (Some(grund), Some(protokoll)) = (
                zurueckbauen(|auf| datei.set_len(auf), laenge, &tagesdatei),
                &self.protokoll,
            ) {
                protokoll.eintragen(
                    Stufe::Error,
                    Bereich::Senden,
                    "Rückbau gescheitert — der Rest des Satzes bleibt in der Tagesdatei stehen",
                    Kontext::neu()
                        .mit(Schluessel::Datei, &tagesdatei)
                        .mit(Schluessel::Grund, grund)
                        .mit(Schluessel::Anzahl, laenge),
                );
            }
            return Err(fehler);
        }
        Ok(Angehaengt {
            zeilenende_ergaenzt,
        })
    }

    fn channel_ordner_anlegen(&self, gespraech: &Gespraech) -> io::Result<()> {
        let ordner = self.ordner.join(gespraech.pfad());
        if let Some(eltern) = ordner.parent() {
            std::fs::create_dir_all(eltern)?;
        }
        std::fs::create_dir(ordner)
    }

    fn einstellungen_schreiben(&self, gespraech: &Gespraech, bytes: &[u8]) -> io::Result<()> {
        let ordner = self.ordner.join(gespraech.pfad());
        let neu = ordner.join(EINSTELLUNGEN_NEU);
        std::fs::write(&neu, bytes)?;
        std::fs::rename(neu, ordner.join(EINSTELLUNGEN))
    }

    fn einstellungen_lesen(&self, gespraech: &Gespraech) -> io::Result<Option<Vec<u8>>> {
        match std::fs::read(self.ordner.join(gespraech.einstellungsdatei())) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(fehler) if fehler.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(fehler) => Err(fehler),
        }
    }

    fn ordner_entfernen(&self, gespraech: &Gespraech) -> io::Result<()> {
        std::fs::remove_dir_all(self.ordner.join(gespraech.pfad()))
    }

    fn tag_entfernen(&self, gespraech: &Gespraech, tag: NaiveDate) -> io::Result<()> {
        match std::fs::remove_file(self.ordner.join(gespraech.tagesdatei(tag))) {
            Err(fehler) if fehler.kind() == io::ErrorKind::NotFound => Ok(()),
            entfernt => entfernt,
        }
    }
}

/// Schneidet nach einem gescheiterten Schreiben auf die alte Länge zurück (ADR-0011). Scheitert auch das,
/// kommt die Meldung für stderr zurück; der Zeilenende-Wächter fängt den Rest beim nächsten Satz.
fn zurueckbauen(
    kuerzen: impl FnOnce(u64) -> io::Result<()>,
    laenge: u64,
    tagesdatei: &str,
) -> Option<String> {
    kuerzen(laenge).err().map(|fehler| {
        format!(
            "{tagesdatei}: Rückbau auf {laenge} Bytes gescheitert, Rest bleibt stehen ({fehler})"
        )
    })
}

/// Leer oder letztes Byte `\n` — dann beginnt der nächste Satz in Spalte 1.
fn endet_mit_zeilenende(datei: &mut File, laenge: u64) -> io::Result<bool> {
    if laenge == 0 {
        return Ok(true);
    }
    let mut letztes = [0u8; 1];
    datei.seek(SeekFrom::Start(laenge - 1))?;
    datei.read_exact(&mut letztes)?;
    Ok(letztes[0] == b'\n')
}

/// Namen der Unterordner; fehlt der Ordner, gibt es keine.
fn unterordner(ordner: &Path) -> io::Result<Vec<String>> {
    let eintraege = match std::fs::read_dir(ordner) {
        Ok(eintraege) => eintraege,
        Err(fehler) if fehler.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(fehler) => return Err(fehler),
    };
    let mut namen = Vec::new();
    for eintrag in eintraege {
        let eintrag = eintrag?;
        if eintrag.file_type()?.is_dir()
            && let Some(name) = eintrag.file_name().to_str()
        {
            namen.push(name.to_string());
        }
    }
    Ok(namen)
}

/// `JJJJ-MM-TT.log` mit gültigem Datum, genau in dieser Schreibweise.
fn tag_aus_dateiname(name: &str) -> Option<NaiveDate> {
    let stamm = name.strip_suffix(".log")?;
    let tag = NaiveDate::parse_from_str(stamm, "%Y-%m-%d").ok()?;
    (tag.format("%Y-%m-%d").to_string() == stamm).then_some(tag)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tag(text: &str) -> NaiveDate {
        NaiveDate::parse_from_str(text, "%Y-%m-%d").expect("Testtag")
    }

    fn datei(ordner: &Path, pfad: &str, inhalt: &str) {
        let ziel = ordner.join(pfad);
        fs::create_dir_all(ziel.parent().expect("Elternordner")).expect("Ordner");
        fs::write(ziel, inhalt).expect("Datei");
    }

    #[test]
    fn findet_channels_und_direktgespraeche_mit_ihren_tagen_und_ignoriert_anderes() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let ordner = wurzel.path();
        datei(ordner, "channels/allgemein/2026-10-27.log", "zwei\n");
        datei(ordner, "channels/allgemein/2026-10-26.log", "eins\n");
        for anderes in [
            "notizen.txt",
            "einstellungen.toml",
            "2026-13-01.log",
            "2026-1-05.log",
            "x2026-10-27.log",
            "2026-10-27.log.bak",
        ] {
            datei(ordner, &format!("channels/allgemein/{anderes}"), "x");
        }
        datei(
            ordner,
            "channels/allgemein/2026-10-25.log/darin",
            "Ordner, keine Datei",
        );
        datei(ordner, "channels/leer/.versteckt", "");
        datei(ordner, "channels/keinordner", "Datei statt Gespräch");
        datei(
            ordner,
            "direkt/claude@10.0.0.5+shoff@127.0.0.1/2026-10-27.log",
            "d\n",
        );
        datei(ordner, ".localchatA.sperre", "");
        let ablage = Dateiablage::neu(ordner);

        let gespraeche = ablage.gespraeche().expect("Gespräche");

        assert_eq!(
            gespraeche,
            vec![
                Gespraech::channel("allgemein"),
                Gespraech::channel("leer"),
                Gespraech::Direkt("claude@10.0.0.5+shoff@127.0.0.1".into()),
            ]
        );
        assert_eq!(
            ablage.tage(&Gespraech::channel("allgemein")).expect("Tage"),
            vec![tag("2026-10-26"), tag("2026-10-27")]
        );
        assert!(
            ablage
                .tage(&Gespraech::channel("leer"))
                .expect("Tage")
                .is_empty()
        );
        assert_eq!(
            ablage
                .lesen(&Gespraech::channel("allgemein"), tag("2026-10-26"))
                .expect("lesen"),
            b"eins\n"
        );
        assert_eq!(
            ablage
                .lesen(&gespraeche[2], tag("2026-10-27"))
                .expect("lesen"),
            b"d\n"
        );
    }

    fn allgemein(ordner: &Path) -> (Dateiablage, Gespraech) {
        fs::create_dir_all(ordner.join("channels/allgemein")).expect("Channel-Ordner");
        (Dateiablage::neu(ordner), Gespraech::channel("allgemein"))
    }

    #[test]
    fn zwei_saetze_stehen_in_reihenfolge_in_der_tagesdatei() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let (ablage, allgemein) = allgemein(wurzel.path());

        let erstes = ablage
            .anhaengen(&allgemein, tag("2026-10-27"), "@ eins\n  a\n\n")
            .expect("angehängt");
        ablage
            .anhaengen(&allgemein, tag("2026-10-27"), "@ zwei\n  b\n\n")
            .expect("angehängt");

        assert!(!erstes.zeilenende_ergaenzt);
        assert_eq!(
            fs::read_to_string(wurzel.path().join("channels/allgemein/2026-10-27.log"))
                .expect("Tagesdatei"),
            "@ eins\n  a\n\n@ zwei\n  b\n\n"
        );
    }

    #[test]
    fn neuer_tag_oeffnet_neue_datei_und_laesst_den_alten_byte_gleich() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let (ablage, allgemein) = allgemein(wurzel.path());
        ablage
            .anhaengen(&allgemein, tag("2026-10-27"), "@ alt\n\n")
            .expect("angehängt");
        let alt = wurzel.path().join("channels/allgemein/2026-10-27.log");
        let vorher = fs::read(&alt).expect("alter Tag");

        ablage
            .anhaengen(&allgemein, tag("2026-10-28"), "@ neu\n\n")
            .expect("angehängt");

        assert_eq!(fs::read(&alt).expect("alter Tag"), vorher);
        assert_eq!(
            fs::read_to_string(wurzel.path().join("channels/allgemein/2026-10-28.log"))
                .expect("neuer Tag"),
            "@ neu\n\n"
        );
    }

    #[test]
    fn endet_die_datei_ohne_zeilenende_beginnt_der_satz_in_einer_neuen_zeile() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let (ablage, allgemein) = allgemein(wurzel.path());
        let pfad = wurzel.path().join("channels/allgemein/2026-10-27.log");
        fs::write(&pfad, "@ eins\n  Rest").expect("ohne Zeilenende");

        let angehaengt = ablage
            .anhaengen(&allgemein, tag("2026-10-27"), "@ zwei\n\n")
            .expect("angehängt");

        assert!(angehaengt.zeilenende_ergaenzt);
        assert_eq!(
            fs::read_to_string(&pfad).expect("Tagesdatei"),
            "@ eins\n  Rest\n@ zwei\n\n"
        );
    }

    #[test]
    fn leere_vorhandene_datei_braucht_kein_zeilenende() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let (ablage, allgemein) = allgemein(wurzel.path());
        let pfad = wurzel.path().join("channels/allgemein/2026-10-27.log");
        fs::write(&pfad, "").expect("leer");

        let angehaengt = ablage
            .anhaengen(&allgemein, tag("2026-10-27"), "@ eins\n\n")
            .expect("angehängt");

        assert!(!angehaengt.zeilenende_ergaenzt);
        assert_eq!(fs::read_to_string(&pfad).expect("Tagesdatei"), "@ eins\n\n");
    }

    #[test]
    fn unbekanntes_gespraech_wird_nicht_angelegt() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let ablage = Dateiablage::neu(wurzel.path());

        let ergebnis = ablage.anhaengen(
            &Gespraech::channel("gibtsnicht"),
            tag("2026-10-27"),
            "@ x\n\n",
        );

        assert!(ergebnis.is_err());
        assert!(!wurzel.path().join("channels/gibtsnicht").exists());
    }

    #[test]
    fn gescheiterter_rueckbau_meldet_datei_und_laenge_gelungener_nichts() {
        let gelungen = zurueckbauen(|_| Ok(()), 55, "channels/allgemein/2026-10-27.log");
        assert_eq!(gelungen, None);

        let gescheitert = zurueckbauen(
            |_| Err(io::Error::other("Platte weg")),
            55,
            "channels/allgemein/2026-10-27.log",
        )
        .expect("Meldung");

        assert!(
            gescheitert.contains("channels/allgemein/2026-10-27.log"),
            "{gescheitert}"
        );
        assert!(gescheitert.contains("55"), "{gescheitert}");
        assert!(gescheitert.contains("Platte weg"), "{gescheitert}");
        assert!(!gescheitert.contains('\n'), "{gescheitert}");
    }

    #[test]
    fn rueckbau_kuerzt_auf_die_uebergebene_laenge() {
        let mut gekuerzt = None;
        zurueckbauen(
            |auf| {
                gekuerzt = Some(auf);
                Ok(())
            },
            55,
            "x",
        );
        assert_eq!(gekuerzt, Some(55));
    }

    #[test]
    fn leerer_ordner_hat_keine_gespraeche() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        assert!(
            Dateiablage::neu(wurzel.path())
                .gespraeche()
                .expect("Gespräche")
                .is_empty()
        );
    }

    #[test]
    fn tagesdatei_ist_relativ_zum_ordner_mit_schraegstrich() {
        assert_eq!(
            Gespraech::channel("allgemein").tagesdatei(tag("2026-10-27")),
            "channels/allgemein/2026-10-27.log"
        );
        assert_eq!(
            Gespraech::Direkt("a@b+c@d".into()).tagesdatei(tag("2026-01-02")),
            "direkt/a@b+c@d/2026-01-02.log"
        );
    }

    #[test]
    fn channel_ordner_entsteht_auch_ohne_channels_und_ein_zweites_mal_scheitert_unberuehrt() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let ablage = Dateiablage::neu(wurzel.path());
        let allgemein = Gespraech::channel("allgemein");

        ablage.channel_ordner_anlegen(&allgemein).expect("angelegt");
        fs::write(wurzel.path().join("channels/allgemein/zettel.txt"), "hallo").expect("Zettel");
        let zweites = ablage.channel_ordner_anlegen(&allgemein);

        assert!(wurzel.path().join("channels/allgemein").is_dir());
        assert_eq!(
            zweites.map_err(|fehler| fehler.kind()),
            Err(io::ErrorKind::AlreadyExists)
        );
        assert_eq!(
            fs::read_to_string(wurzel.path().join("channels/allgemein/zettel.txt"))
                .expect("Zettel"),
            "hallo"
        );
    }

    #[test]
    fn einstellungen_werden_ueber_neu_und_umbenennen_geschrieben_und_ersetzt() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let (ablage, allgemein) = allgemein(wurzel.path());
        let ordner = wurzel.path().join("channels/allgemein");

        assert_eq!(
            ablage.einstellungen_lesen(&allgemein).expect("lesbar"),
            None
        );
        ablage
            .einstellungen_schreiben(&allgemein, b"a = 1\n")
            .expect("geschrieben");
        ablage
            .einstellungen_schreiben(&allgemein, b"a = 2\n")
            .expect("ersetzt");

        assert_eq!(
            fs::read(ordner.join("einstellungen.toml")).expect("Datei"),
            b"a = 2\n"
        );
        assert!(!ordner.join("einstellungen.toml.neu").exists());
        assert_eq!(
            ablage.einstellungen_lesen(&allgemein).expect("lesbar"),
            Some(b"a = 2\n".to_vec())
        );
        assert_eq!(
            allgemein.einstellungsdatei(),
            "channels/allgemein/einstellungen.toml"
        );
    }

    #[test]
    fn ordner_entfernen_nimmt_den_ordner_samt_inhalt() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let (ablage, allgemein) = allgemein(wurzel.path());
        ablage
            .einstellungen_schreiben(&allgemein, b"a = 1\n")
            .expect("geschrieben");

        ablage.ordner_entfernen(&allgemein).expect("entfernt");

        assert!(!wurzel.path().join("channels/allgemein").exists());
        assert!(wurzel.path().join("channels").is_dir());
    }

    /// R00008 B0181: nur die Tagesdatei des Tages geht, Nachbarn und fremde Dateien bleiben; fehlt sie, gilt sie
    /// als entfernt.
    #[test]
    fn tag_entfernen_nimmt_genau_die_tagesdatei_und_fehlend_gilt_als_entfernt() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let ordner = wurzel.path();
        datei(ordner, "channels/allgemein/2026-10-20.log", "alt\n");
        datei(ordner, "channels/allgemein/2026-10-21.log", "neu\n");
        datei(
            ordner,
            "channels/allgemein/2026-10-20.log.bak",
            "Sicherung\n",
        );
        let ablage = Dateiablage::neu(ordner);
        let allgemein = Gespraech::channel("allgemein");

        ablage
            .tag_entfernen(&allgemein, tag("2026-10-20"))
            .expect("entfernt");

        assert!(!ordner.join("channels/allgemein/2026-10-20.log").exists());
        assert!(ordner.join("channels/allgemein/2026-10-21.log").is_file());
        assert!(
            ordner
                .join("channels/allgemein/2026-10-20.log.bak")
                .is_file()
        );
        assert_eq!(
            ablage.tage(&allgemein).expect("Tage"),
            vec![tag("2026-10-21")]
        );
        ablage
            .tag_entfernen(&allgemein, tag("2026-10-20"))
            .expect("fehlt schon — gilt als entfernt");
    }
}
