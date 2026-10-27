//! Logablage (A00008, ADR-0003): Gespräche und ihre Tage als Dateien im Ordner.
//!
//! Kennt das Logformat nicht — liest und schreibt Sätze als fertigen Text.

use chrono::NaiveDate;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

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
    fn lesen(&self, gespraech: &Gespraech, tag: NaiveDate) -> io::Result<String>;
}

/// Die Ablage im Ordner der Instanz.
pub struct Dateiablage {
    ordner: PathBuf,
}

impl Dateiablage {
    pub fn neu(ordner: &Path) -> Dateiablage {
        Dateiablage {
            ordner: ordner.to_path_buf(),
        }
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

    fn lesen(&self, gespraech: &Gespraech, tag: NaiveDate) -> io::Result<String> {
        std::fs::read_to_string(self.ordner.join(gespraech.tagesdatei(tag)))
    }
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
            "eins\n"
        );
        assert_eq!(
            ablage
                .lesen(&gespraeche[2], tag("2026-10-27"))
                .expect("lesen"),
            "d\n"
        );
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
}
