//! Speicher-Logablage: zweite Implementierung des Ports (A00008) für Tests der Gespräche, ohne Platte.
//!
//! Kann „Fehler nach n Bytes“: das nächste Anhängen schreibt n Bytes, scheitert und baut zurück wie die Datei.

use super::logablage::{Angehaengt, Gespraech, Logablage};
use chrono::NaiveDate;
use std::collections::BTreeMap;
use std::io;
use std::sync::Mutex;

#[derive(Default)]
pub struct Speicherablage {
    innen: Mutex<Innen>,
}

#[derive(Default)]
struct Innen {
    gespraeche: BTreeMap<Gespraech, BTreeMap<NaiveDate, String>>,
    fehler_nach: Option<usize>,
    einstellungen: BTreeMap<Gespraech, Vec<u8>>,
    einstellungen_scheitert: bool,
    entfernen_scheitert: std::collections::BTreeSet<(Gespraech, NaiveDate)>,
    auflisten_scheitert: std::collections::BTreeSet<Gespraech>,
    schreibvorgaenge: usize,
}

impl Speicherablage {
    pub fn neu() -> Speicherablage {
        Speicherablage::default()
    }

    /// Legt ein Gespräch ohne Tage an (wie ein leerer Ordner `channels/<slug>/`).
    pub fn gespraech_anlegen(&self, gespraech: &Gespraech) {
        self.innen()
            .gespraeche
            .entry(gespraech.clone())
            .or_default();
    }

    /// Setzt den Inhalt eines Tages, als läge die Datei schon da.
    pub fn tag_setzen(&self, gespraech: &Gespraech, tag: NaiveDate, inhalt: &str) {
        self.innen()
            .gespraeche
            .entry(gespraech.clone())
            .or_default()
            .insert(tag, inhalt.to_string());
    }

    /// Das nächste `anhaengen` schreibt `bytes` Bytes, liefert dann einen Fehler und baut zurück.
    pub fn fehler_nach(&self, bytes: usize) {
        self.innen().fehler_nach = Some(bytes);
    }

    /// Das nächste `einstellungen_schreiben` scheitert, ohne etwas abzulegen.
    pub fn einstellungen_scheitern(&self) {
        self.innen().einstellungen_scheitert = true;
    }

    /// Jedes `tag_entfernen` dieses Tages scheitert, ohne etwas zu entfernen (wie eine schreibgeschützte Datei).
    pub fn entfernen_scheitern(&self, gespraech: &Gespraech, tag: NaiveDate) {
        self.innen()
            .entfernen_scheitert
            .insert((gespraech.clone(), tag));
    }

    /// Das Entfernen dieses Tages gelingt wieder (Rechte zurückgesetzt).
    pub fn entfernen_gelingt(&self, gespraech: &Gespraech, tag: NaiveDate) {
        self.innen()
            .entfernen_scheitert
            .remove(&(gespraech.clone(), tag));
    }

    /// Jedes `tage` dieses Gesprächs scheitert (wie ein Ordner ohne Leserecht, `chmod a-r`).
    pub fn auflisten_scheitern(&self, gespraech: &Gespraech) {
        self.innen().auflisten_scheitert.insert(gespraech.clone());
    }

    /// Legt Einstellungsbytes ab, als läge die Datei schon da.
    pub fn einstellungen_setzen(&self, gespraech: &Gespraech, bytes: &[u8]) {
        let mut innen = self.innen();
        innen.gespraeche.entry(gespraech.clone()).or_default();
        innen
            .einstellungen
            .insert(gespraech.clone(), bytes.to_vec());
    }

    /// Wie oft Ordner angelegt oder Einstellungen geschrieben wurden.
    pub fn schreibvorgaenge(&self) -> usize {
        self.innen().schreibvorgaenge
    }

    pub fn inhalt(&self, gespraech: &Gespraech, tag: NaiveDate) -> Option<String> {
        self.innen().gespraeche.get(gespraech)?.get(&tag).cloned()
    }

    fn innen(&self) -> std::sync::MutexGuard<'_, Innen> {
        self.innen
            .lock()
            .unwrap_or_else(|vergiftet| vergiftet.into_inner())
    }
}

impl Logablage for Speicherablage {
    fn gespraeche(&self) -> io::Result<Vec<Gespraech>> {
        Ok(self.innen().gespraeche.keys().cloned().collect())
    }

    fn tage(&self, gespraech: &Gespraech) -> io::Result<Vec<NaiveDate>> {
        let innen = self.innen();
        if innen.auflisten_scheitert.contains(gespraech) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Auflisten scheitert (Speicher-Logablage)",
            ));
        }
        let tage = innen.gespraeche.get(gespraech).ok_or_else(unbekannt)?;
        Ok(tage.keys().copied().collect())
    }

    fn lesen(&self, gespraech: &Gespraech, tag: NaiveDate) -> io::Result<Vec<u8>> {
        self.inhalt(gespraech, tag)
            .map(String::into_bytes)
            .ok_or_else(unbekannt)
    }

    fn anhaengen(
        &self,
        gespraech: &Gespraech,
        tag: NaiveDate,
        satz: &str,
    ) -> io::Result<Angehaengt> {
        let mut innen = self.innen();
        let fehler_nach = innen.fehler_nach.take();
        let tage = innen.gespraeche.get_mut(gespraech).ok_or_else(unbekannt)?;
        let inhalt = tage.entry(tag).or_default();
        let laenge = inhalt.len();
        let zeilenende_ergaenzt = !inhalt.is_empty() && !inhalt.ends_with('\n');
        let mut bytes = String::with_capacity(satz.len() + 1);
        if zeilenende_ergaenzt {
            bytes.push('\n');
        }
        bytes.push_str(satz);
        if let Some(n) = fehler_nach {
            inhalt.push_str(bytes.get(..n).unwrap_or(&bytes));
            inhalt.truncate(laenge);
            return Err(io::Error::other(format!(
                "Fehler nach {n} Bytes (Speicher-Logablage)"
            )));
        }
        inhalt.push_str(&bytes);
        Ok(Angehaengt {
            zeilenende_ergaenzt,
        })
    }

    fn channel_ordner_anlegen(&self, gespraech: &Gespraech) -> io::Result<()> {
        let mut innen = self.innen();
        if innen.gespraeche.contains_key(gespraech) {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "Ordner besteht schon (Speicher-Logablage)",
            ));
        }
        innen.gespraeche.insert(gespraech.clone(), BTreeMap::new());
        innen.schreibvorgaenge += 1;
        Ok(())
    }

    fn einstellungen_schreiben(&self, gespraech: &Gespraech, bytes: &[u8]) -> io::Result<()> {
        let mut innen = self.innen();
        if std::mem::take(&mut innen.einstellungen_scheitert) {
            return Err(io::Error::other(
                "Einstellungen schreiben scheitert (Speicher-Logablage)",
            ));
        }
        if !innen.gespraeche.contains_key(gespraech) {
            return Err(unbekannt());
        }
        innen
            .einstellungen
            .insert(gespraech.clone(), bytes.to_vec());
        innen.schreibvorgaenge += 1;
        Ok(())
    }

    fn einstellungen_lesen(&self, gespraech: &Gespraech) -> io::Result<Option<Vec<u8>>> {
        Ok(self.innen().einstellungen.get(gespraech).cloned())
    }

    fn ordner_entfernen(&self, gespraech: &Gespraech) -> io::Result<()> {
        let mut innen = self.innen();
        innen.einstellungen.remove(gespraech);
        innen
            .gespraeche
            .remove(gespraech)
            .map(|_| ())
            .ok_or_else(unbekannt)
    }

    fn leeren_ordner_entfernen(&self, gespraech: &Gespraech) -> io::Result<()> {
        let mut innen = self.innen();
        match innen.gespraeche.get(gespraech) {
            None => Ok(()),
            Some(tage) if tage.is_empty() => {
                innen.gespraeche.remove(gespraech);
                Ok(())
            }
            Some(_) => Err(io::Error::new(
                io::ErrorKind::DirectoryNotEmpty,
                "Ordner nicht leer (Speicher-Logablage)",
            )),
        }
    }

    fn tag_entfernen(&self, gespraech: &Gespraech, tag: NaiveDate) -> io::Result<()> {
        let mut innen = self.innen();
        if innen
            .entfernen_scheitert
            .contains(&(gespraech.clone(), tag))
        {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Entfernen scheitert (Speicher-Logablage)",
            ));
        }
        if let Some(tage) = innen.gespraeche.get_mut(gespraech) {
            tage.remove(&tag);
        }
        Ok(())
    }
}

fn unbekannt() -> io::Error {
    io::Error::new(io::ErrorKind::NotFound, "Gespräch unbekannt")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tag(text: &str) -> NaiveDate {
        NaiveDate::parse_from_str(text, "%Y-%m-%d").expect("Testtag")
    }

    fn mit_allgemein() -> (Speicherablage, Gespraech) {
        let ablage = Speicherablage::neu();
        let allgemein = Gespraech::channel("allgemein");
        ablage.gespraech_anlegen(&allgemein);
        (ablage, allgemein)
    }

    #[test]
    fn angelegte_gespraeche_und_tage_werden_gefunden_und_gelesen() {
        let (ablage, allgemein) = mit_allgemein();
        ablage.gespraech_anlegen(&Gespraech::channel("b"));
        ablage.tag_setzen(&allgemein, tag("2026-10-27"), "@ zwei\n\n");
        ablage.tag_setzen(&allgemein, tag("2026-10-26"), "@ eins\n\n");

        assert_eq!(
            ablage.gespraeche().expect("Gespräche"),
            vec![allgemein.clone(), Gespraech::channel("b")]
        );
        assert_eq!(
            ablage.tage(&allgemein).expect("Tage"),
            vec![tag("2026-10-26"), tag("2026-10-27")]
        );
        assert_eq!(
            ablage.lesen(&allgemein, tag("2026-10-26")).expect("lesen"),
            b"@ eins\n\n"
        );
    }

    #[test]
    fn anhaengen_verlaengert_den_tag_und_ergaenzt_ein_fehlendes_zeilenende() {
        let (ablage, allgemein) = mit_allgemein();
        ablage.tag_setzen(&allgemein, tag("2026-10-27"), "@ eins\n  Rest");

        let angehaengt = ablage
            .anhaengen(&allgemein, tag("2026-10-27"), "@ zwei\n\n")
            .expect("angehängt");
        ablage
            .anhaengen(&allgemein, tag("2026-10-28"), "@ drei\n\n")
            .expect("angehängt");

        assert!(angehaengt.zeilenende_ergaenzt);
        assert_eq!(
            ablage.inhalt(&allgemein, tag("2026-10-27")).as_deref(),
            Some("@ eins\n  Rest\n@ zwei\n\n")
        );
        assert_eq!(
            ablage.inhalt(&allgemein, tag("2026-10-28")).as_deref(),
            Some("@ drei\n\n")
        );
    }

    #[test]
    fn unbekanntes_gespraech_ist_ein_fehler_und_entsteht_nicht() {
        let ablage = Speicherablage::neu();
        let gibtsnicht = Gespraech::channel("gibtsnicht");

        assert!(
            ablage
                .anhaengen(&gibtsnicht, tag("2026-10-27"), "@ x\n\n")
                .is_err()
        );
        assert!(ablage.gespraeche().expect("Gespräche").is_empty());
    }

    #[test]
    fn fehler_nach_n_bytes_laesst_die_laenge_gleich_und_gilt_nur_einmal() {
        let (ablage, allgemein) = mit_allgemein();
        ablage.tag_setzen(&allgemein, tag("2026-10-27"), "@ eins\n\n");
        ablage.fehler_nach(3);

        let gescheitert = ablage.anhaengen(&allgemein, tag("2026-10-27"), "@ zwei\n\n");

        assert!(gescheitert.is_err());
        assert_eq!(
            ablage.inhalt(&allgemein, tag("2026-10-27")).as_deref(),
            Some("@ eins\n\n")
        );
        ablage
            .anhaengen(&allgemein, tag("2026-10-27"), "@ drei\n\n")
            .expect("nächstes gelingt");
        assert_eq!(
            ablage.inhalt(&allgemein, tag("2026-10-27")).as_deref(),
            Some("@ eins\n\n@ drei\n\n")
        );
    }

    #[test]
    fn scheitert_das_anhaengen_an_einem_neuen_tag_bleibt_der_tag_leer() {
        let (ablage, allgemein) = mit_allgemein();
        ablage.fehler_nach(2);

        assert!(
            ablage
                .anhaengen(&allgemein, tag("2026-10-28"), "@ x\n\n")
                .is_err()
        );
        assert_eq!(
            ablage
                .inhalt(&allgemein, tag("2026-10-28"))
                .unwrap_or_default(),
            ""
        );
    }

    #[test]
    fn channel_ordner_und_einstellungen_im_speicher_mit_zaehler_und_rueckbau() {
        let ablage = Speicherablage::neu();
        let allgemein = Gespraech::channel("allgemein");

        ablage.channel_ordner_anlegen(&allgemein).expect("angelegt");
        let zweites = ablage.channel_ordner_anlegen(&allgemein);
        ablage
            .einstellungen_schreiben(&allgemein, b"a = 1\n")
            .expect("geschrieben");

        assert_eq!(
            zweites.map_err(|fehler| fehler.kind()),
            Err(io::ErrorKind::AlreadyExists)
        );
        assert_eq!(
            ablage.gespraeche().expect("Gespräche"),
            vec![allgemein.clone()]
        );
        assert_eq!(
            ablage.einstellungen_lesen(&allgemein).expect("lesbar"),
            Some(b"a = 1\n".to_vec())
        );
        assert_eq!(ablage.schreibvorgaenge(), 2);

        ablage.ordner_entfernen(&allgemein).expect("entfernt");

        assert!(ablage.gespraeche().expect("Gespräche").is_empty());
        assert_eq!(
            ablage.einstellungen_lesen(&allgemein).expect("lesbar"),
            None
        );
    }

    #[test]
    fn einstellungen_schreiben_kann_einmal_scheitern() {
        let (ablage, allgemein) = mit_allgemein();
        ablage.einstellungen_scheitern();

        assert!(
            ablage
                .einstellungen_schreiben(&allgemein, b"a = 1\n")
                .is_err()
        );
        assert_eq!(
            ablage.einstellungen_lesen(&allgemein).expect("lesbar"),
            None
        );
        ablage
            .einstellungen_schreiben(&allgemein, b"a = 1\n")
            .expect("das nächste gelingt");
    }

    #[test]
    fn tag_entfernen_nimmt_den_tag_und_kann_je_tag_scheitern() {
        let (ablage, allgemein) = mit_allgemein();
        ablage.tag_setzen(&allgemein, tag("2026-10-20"), "@ alt\n\n");
        ablage.tag_setzen(&allgemein, tag("2026-10-21"), "@ fest\n\n");
        ablage.tag_setzen(&allgemein, tag("2026-10-22"), "@ neu\n\n");
        ablage.entfernen_scheitern(&allgemein, tag("2026-10-21"));

        ablage
            .tag_entfernen(&allgemein, tag("2026-10-20"))
            .expect("entfernt");
        let gescheitert = ablage.tag_entfernen(&allgemein, tag("2026-10-21"));
        ablage
            .tag_entfernen(&allgemein, tag("2026-10-19"))
            .expect("fehlt — gilt als entfernt");

        assert!(gescheitert.is_err());
        assert_eq!(
            ablage.tage(&allgemein).expect("Tage"),
            vec![tag("2026-10-21"), tag("2026-10-22")]
        );
    }
}
