//! Speicher-Attrappe der Anhangablage für Tests ohne Platte: Eingang als Namen → Bytes, mit Fehlern beim n-ten
//! Stück, einem untergeschobenen Ziel und einer Schranke vor dem Abschluss (Wettläufe, M23, M24, M39).

use super::anhangablage::{Anhangablage, Anhangdatei, Teildatei};
use super::anhangid::Anhangid;
use super::anhangverzeichnis::Tagesdatei;
use super::logablage::Gespraech;
use chrono::NaiveDate;
use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::sync::{Arc, Barrier, Mutex};

#[derive(Default)]
pub struct Anhangspeicher {
    innen: Arc<Mutex<Innen>>,
}

#[derive(Default)]
struct Innen {
    eingang: BTreeMap<String, Vec<u8>>,
    /// Das n-te Stück (ab 1) aller Teildateien scheitert mit diesem Fehler.
    stueckfehler: Option<(usize, io::ErrorKind, Option<i32>)>,
    stuecke: usize,
    /// Vor `teil_abschliessen` warten beide Seiten an dieser Schranke, dann noch einmal (Wettlauf-Proben).
    vor_abschluss: Option<Arc<Barrier>>,
    /// Größte Länge eines Lesens bzw. Schreibens per Position (M54).
    groesstes_lesen: usize,
    groesstes_schreiben: usize,
    lesevorgaenge: usize,
    /// Gebundene Anhänge je (Gespräch, Tag, Datei).
    anhaenge: BTreeMap<(Gespraech, NaiveDate, String), Vec<u8>>,
    /// Das n-te Binden (ab 1) scheitert.
    binden_scheitert: Option<usize>,
    gebunden: usize,
    zuruecklegen_scheitert: bool,
    /// Diese Dateien lassen sich nicht kürzen (wie `ELOOP`).
    kuerzen_scheitert: BTreeSet<String>,
    /// Vor dem nächsten Kürzen zweimal an dieser Schranke warten.
    vor_kuerzen: Option<Arc<Barrier>>,
    /// Ein Leser bekommt nach dem ersten Lesen 0 Bytes (die Datei wurde inzwischen gekürzt).
    nach_erstem_lesen_leer: bool,
}

impl Anhangspeicher {
    pub fn neu() -> Anhangspeicher {
        Anhangspeicher::default()
    }

    fn innen(&self) -> std::sync::MutexGuard<'_, Innen> {
        self.innen
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Das n-te geschriebene Stück scheitert (z. B. `ENOSPC` beim zweiten).
    pub fn stueck_scheitert(&self, nummer: usize, fehler: io::Error) {
        self.innen().stueckfehler = Some((nummer, fehler.kind(), fehler.raw_os_error()));
    }

    /// Legt eine Datei in den Eingang, als läge sie schon da (untergeschobenes Ziel).
    pub fn eingang_setzen(&self, name: &str, bytes: &[u8]) {
        self.innen()
            .eingang
            .insert(name.to_string(), bytes.to_vec());
    }

    pub fn eingang(&self) -> BTreeMap<String, Vec<u8>> {
        self.innen().eingang.clone()
    }

    /// Vor dem nächsten `teil_abschliessen` an dieser Schranke warten — zweimal: einmal, um dem Test den Zug zu
    /// geben, einmal, um auf sein Ende zu warten.
    pub fn vor_abschluss_warten(&self, schranke: Arc<Barrier>) {
        self.innen().vor_abschluss = Some(schranke);
    }

    pub fn groesstes_lesen(&self) -> usize {
        self.innen().groesstes_lesen
    }

    pub fn groesstes_schreiben(&self) -> usize {
        self.innen().groesstes_schreiben
    }

    pub fn lesevorgaenge(&self) -> usize {
        self.innen().lesevorgaenge
    }

    pub fn anhaenge(&self) -> BTreeMap<(Gespraech, NaiveDate, String), Vec<u8>> {
        self.innen().anhaenge.clone()
    }

    pub fn anhang_setzen(&self, gespraech: &Gespraech, tag: NaiveDate, datei: &str, bytes: &[u8]) {
        self.innen()
            .anhaenge
            .insert((gespraech.clone(), tag, datei.to_string()), bytes.to_vec());
    }

    /// Das n-te Binden (ab 1) scheitert.
    pub fn binden_scheitert(&self, nummer: usize) {
        self.innen().binden_scheitert = Some(nummer);
    }

    pub fn zuruecklegen_scheitert(&self) {
        self.innen().zuruecklegen_scheitert = true;
    }

    pub fn kuerzen_scheitert(&self, datei: &str) {
        self.innen().kuerzen_scheitert.insert(datei.to_string());
    }

    pub fn vor_kuerzen_warten(&self, schranke: Arc<Barrier>) {
        self.innen().vor_kuerzen = Some(schranke);
    }

    pub fn nach_erstem_lesen_leer(&self) {
        self.innen().nach_erstem_lesen_leer = true;
    }
}

fn teilname(id: &Anhangid) -> String {
    format!("{}.teil", id.als_text())
}

impl Anhangablage for Anhangspeicher {
    fn teil_anlegen(&self, id: &Anhangid) -> io::Result<Box<dyn Teildatei>> {
        let name = teilname(id);
        let mut innen = self.innen();
        if innen.eingang.contains_key(&name) {
            return Err(io::Error::from(io::ErrorKind::AlreadyExists));
        }
        innen.eingang.insert(name.clone(), Vec::new());
        Ok(Box::new(SpeicherTeil {
            innen: Arc::clone(&self.innen),
            name,
        }))
    }

    fn teil_abschliessen(&self, id: &Anhangid) -> io::Result<()> {
        let schranke = self.innen().vor_abschluss.take();
        if let Some(schranke) = schranke {
            schranke.wait();
            schranke.wait();
        }
        let mut innen = self.innen();
        if innen.eingang.contains_key(id.als_text()) {
            return Err(io::Error::from(io::ErrorKind::AlreadyExists));
        }
        let bytes = innen
            .eingang
            .remove(&teilname(id))
            .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
        innen.eingang.insert(id.als_text().to_string(), bytes);
        Ok(())
    }

    fn eingang_loeschen(&self, id: &Anhangid) -> io::Result<()> {
        let mut innen = self.innen();
        innen.eingang.remove(&teilname(id));
        innen.eingang.remove(id.als_text());
        Ok(())
    }

    fn teil_loeschen(&self, id: &Anhangid) -> io::Result<()> {
        self.innen().eingang.remove(&teilname(id));
        Ok(())
    }

    fn binden(
        &self,
        id: &Anhangid,
        gespraech: &Gespraech,
        tag: NaiveDate,
        datei: &str,
    ) -> io::Result<()> {
        let mut innen = self.innen();
        innen.gebunden += 1;
        if innen.binden_scheitert == Some(innen.gebunden) {
            return Err(io::Error::other("Binden scheitert (Attrappe)"));
        }
        let schluessel = (gespraech.clone(), tag, datei.to_string());
        if innen.anhaenge.contains_key(&schluessel) {
            return Err(io::Error::from(io::ErrorKind::AlreadyExists));
        }
        let bytes = innen
            .eingang
            .remove(id.als_text())
            .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
        innen.anhaenge.insert(schluessel, bytes);
        Ok(())
    }

    fn zuruecklegen(
        &self,
        gespraech: &Gespraech,
        tag: NaiveDate,
        datei: &str,
        id: &Anhangid,
    ) -> io::Result<()> {
        let mut innen = self.innen();
        if innen.zuruecklegen_scheitert {
            return Err(io::Error::other("Zurücklegen scheitert (Attrappe)"));
        }
        if innen.eingang.contains_key(id.als_text()) {
            return Err(io::Error::from(io::ErrorKind::AlreadyExists));
        }
        let bytes = innen
            .anhaenge
            .remove(&(gespraech.clone(), tag, datei.to_string()))
            .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
        innen.eingang.insert(id.als_text().to_string(), bytes);
        Ok(())
    }

    fn tag_dateien(&self, gespraech: &Gespraech, tag: NaiveDate) -> io::Result<Vec<Tagesdatei>> {
        Ok(self
            .innen()
            .anhaenge
            .iter()
            .filter(|((g, t, _), _)| g == gespraech && *t == tag)
            .map(|((_, _, name), bytes)| Tagesdatei {
                name: name.clone(),
                laenge: bytes.len() as u64,
            })
            .collect())
    }

    fn anhang_tage(&self, gespraech: &Gespraech) -> io::Result<Vec<NaiveDate>> {
        let mut tage: Vec<NaiveDate> = self
            .innen()
            .anhaenge
            .keys()
            .filter(|(g, _, _)| g == gespraech)
            .map(|(_, tag, _)| *tag)
            .collect();
        tage.dedup();
        Ok(tage)
    }

    fn zum_lesen(
        &self,
        gespraech: &Gespraech,
        tag: NaiveDate,
        datei: &str,
    ) -> io::Result<Box<dyn Anhangdatei>> {
        let innen = self.innen();
        let bytes = innen
            .anhaenge
            .get(&(gespraech.clone(), tag, datei.to_string()))
            .cloned()
            .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
        Ok(Box::new(SpeicherAnhang {
            bytes,
            leer_nach_erstem: innen.nach_erstem_lesen_leer,
            gelesen: std::sync::atomic::AtomicBool::new(false),
        }))
    }

    fn kuerzen(&self, gespraech: &Gespraech, tag: NaiveDate, datei: &str) -> io::Result<()> {
        let schranke = self.innen().vor_kuerzen.take();
        if let Some(schranke) = schranke {
            schranke.wait();
            schranke.wait();
        }
        let mut innen = self.innen();
        if innen.kuerzen_scheitert.contains(datei) {
            return Err(io::Error::from_raw_os_error(
                rustix::io::Errno::LOOP.raw_os_error(),
            ));
        }
        if let Some(bytes) = innen
            .anhaenge
            .get_mut(&(gespraech.clone(), tag, datei.to_string()))
        {
            bytes.clear();
        }
        Ok(())
    }

    fn tag_entfernen(&self, gespraech: &Gespraech, tag: NaiveDate) -> io::Result<()> {
        self.innen()
            .anhaenge
            .retain(|(g, t, _), _| !(g == gespraech && *t == tag));
        Ok(())
    }
}

struct SpeicherAnhang {
    bytes: Vec<u8>,
    leer_nach_erstem: bool,
    gelesen: std::sync::atomic::AtomicBool,
}

impl Anhangdatei for SpeicherAnhang {
    fn lesen_an(&self, pos: u64, puffer: &mut [u8]) -> io::Result<usize> {
        if self.leer_nach_erstem && self.gelesen.swap(true, std::sync::atomic::Ordering::SeqCst) {
            return Ok(0);
        }
        let anfang = usize::try_from(pos)
            .unwrap_or(usize::MAX)
            .min(self.bytes.len());
        let n = puffer.len().min(self.bytes.len() - anfang);
        puffer[..n].copy_from_slice(&self.bytes[anfang..anfang + n]);
        Ok(n)
    }

    fn laenge(&self) -> io::Result<u64> {
        Ok(self.bytes.len() as u64)
    }
}

struct SpeicherTeil {
    innen: Arc<Mutex<Innen>>,
    name: String,
}

impl SpeicherTeil {
    fn innen(&self) -> std::sync::MutexGuard<'_, Innen> {
        self.innen
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

impl Teildatei for SpeicherTeil {
    fn anhaengen(&mut self, bytes: &[u8]) -> io::Result<()> {
        let mut innen = self.innen();
        innen.stuecke += 1;
        if let Some((nummer, art, roh)) = innen.stueckfehler
            && nummer == innen.stuecke
        {
            return Err(roh.map_or_else(|| io::Error::from(art), io::Error::from_raw_os_error));
        }
        let name = self.name.clone();
        innen
            .eingang
            .entry(name)
            .or_default()
            .extend_from_slice(bytes);
        Ok(())
    }

    fn lesen_an(&self, pos: u64, puffer: &mut [u8]) -> io::Result<usize> {
        let mut innen = self.innen();
        innen.lesevorgaenge += 1;
        innen.groesstes_lesen = innen.groesstes_lesen.max(puffer.len());
        let daten = innen.eingang.get(&self.name).cloned().unwrap_or_default();
        let anfang = usize::try_from(pos).unwrap_or(usize::MAX).min(daten.len());
        let n = puffer.len().min(daten.len() - anfang);
        puffer[..n].copy_from_slice(&daten[anfang..anfang + n]);
        Ok(n)
    }

    fn schreiben_an(&self, pos: u64, bytes: &[u8]) -> io::Result<()> {
        let mut innen = self.innen();
        innen.groesstes_schreiben = innen.groesstes_schreiben.max(bytes.len());
        let daten = innen.eingang.entry(self.name.clone()).or_default();
        let anfang = usize::try_from(pos).map_err(io::Error::other)?;
        if anfang + bytes.len() > daten.len() {
            return Err(io::Error::other("Schreiben über das Dateiende"));
        }
        daten[anfang..anfang + bytes.len()].copy_from_slice(bytes);
        Ok(())
    }

    fn laenge(&self) -> io::Result<u64> {
        Ok(self.innen().eingang.get(&self.name).map_or(0, Vec::len) as u64)
    }
}
