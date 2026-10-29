//! Speicher-Attrappe der Anhangablage für Tests ohne Platte: Eingang als Namen → Bytes, mit Fehlern beim n-ten
//! Stück, einem untergeschobenen Ziel und einer Schranke vor dem Abschluss (Wettläufe, M23, M24, M39).

use super::anhangablage::{Anhangablage, Teildatei};
use super::anhangid::Anhangid;
use std::collections::BTreeMap;
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
