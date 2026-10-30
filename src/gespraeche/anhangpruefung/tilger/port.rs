//! Positionsport des Tilgers (B0521, A00018 Ergänzung „Karte #9 Security-Gate“; ADR-0047 Ergänzung „Karte #9
//! Security-Gate“ Regel 2): über eine offene Teildatei wird nur per Position gelesen und geschrieben, kein Lese- oder
//! Schreibpuffer ist größer als 1 MiB, und die Länge der Datei ändert sich nie.

use crate::gespraeche::anhangablage::Teildatei;
use std::cell::Cell;
use std::io;

/// Größter Lese- und Schreibpuffer des Tilgers.
pub const STUECK: usize = 1_048_576;

/// Warum der Tilger aufhört: die Datei ist nicht sicher tilgbar (422 `metadaten`) oder die Platte versagt (500).
#[derive(Debug)]
pub enum Abbruch {
    Metadaten(&'static str),
    Platte(io::Error),
}

impl From<io::Error> for Abbruch {
    fn from(fehler: io::Error) -> Abbruch {
        Abbruch::Platte(fehler)
    }
}

pub type Ergebnis<T> = Result<T, Abbruch>;

/// Fester Anteil des Arbeitsbudgets für Kopf-Lesungen und Vorauslesen an Bereichsgrenzen.
const BUDGET_SOCKEL: u64 = 8 * STUECK as u64;

/// Die Datei des Tilgers: Länge fest, Lesen und Schreiben nur innerhalb, jedes Byte gegen das Arbeitsbudget gezählt
/// (ADR-0047 Ergänzung „Karte #9 Security-Review B1“ Regel 4).
pub struct Port<'a> {
    datei: &'a dyn Teildatei,
    laenge: u64,
    budget: u64,
    verbraucht: Cell<u64>,
    erschoepft: Cell<bool>,
}

impl<'a> Port<'a> {
    /// Port mit dem Budget B = 2 × Länge + 8 MiB.
    pub fn neu(datei: &'a dyn Teildatei) -> io::Result<Port<'a>> {
        let laenge = datei.laenge()?;
        let budget = laenge.saturating_mul(2).saturating_add(BUDGET_SOCKEL);
        Ok(Port::mit_budget(datei, laenge, budget))
    }

    fn mit_budget(datei: &'a dyn Teildatei, laenge: u64, budget: u64) -> Port<'a> {
        Port {
            datei,
            laenge,
            budget,
            verbraucht: Cell::new(0),
            erschoepft: Cell::new(false),
        }
    }

    /// Gelesene und geschriebene Bytes bis jetzt.
    pub fn verbraucht(&self) -> u64 {
        self.verbraucht.get()
    }

    pub fn laenge(&self) -> u64 {
        self.laenge
    }

    /// Ob `[pos, pos + n)` ganz in der Datei liegt (ohne Überlauf).
    pub fn enthaelt(&self, pos: u64, n: u64) -> bool {
        pos.checked_add(n).is_some_and(|ende| ende <= self.laenge)
    }

    /// Liest ein Leseziel: höchstens 1 MiB, ganz in der Datei — sonst `metadaten`.
    pub fn lesen(&self, pos: u64, n: u64) -> Ergebnis<Vec<u8>> {
        if n > STUECK as u64 {
            return Err(Abbruch::Metadaten("Leseziel über 1 MiB"));
        }
        if !self.enthaelt(pos, n) {
            return Err(Abbruch::Metadaten("Längen widersprechen sich"));
        }
        let mut puffer = vec![0u8; usize::try_from(n).unwrap_or(STUECK)];
        self.voll_lesen(pos, &mut puffer)?;
        Ok(puffer)
    }

    /// Schreibt höchstens 1 MiB innerhalb der Datei; die Länge bleibt.
    pub fn schreiben(&self, pos: u64, bytes: &[u8]) -> Ergebnis<()> {
        if bytes.len() > STUECK {
            return Err(Abbruch::Metadaten("Schreibziel über 1 MiB"));
        }
        if !self.enthaelt(pos, bytes.len() as u64) {
            return Err(Abbruch::Metadaten("Längen widersprechen sich"));
        }
        self.buchen(bytes.len() as u64)?;
        self.datei.schreiben_an(pos, bytes)?;
        Ok(())
    }

    /// Nullt ein Nullziel beliebiger Länge in Stücken ≤ 1 MiB aus einem festen Nullpuffer — nie gelesen.
    pub fn nullen(&self, pos: u64, n: u64) -> Ergebnis<()> {
        if !self.enthaelt(pos, n) {
            return Err(Abbruch::Metadaten("Längen widersprechen sich"));
        }
        self.buchen(n)?;
        let nullen = vec![0u8; STUECK.min(usize::try_from(n).unwrap_or(STUECK))];
        let mut erledigt = 0u64;
        while erledigt < n {
            let stueck = (n - erledigt).min(nullen.len() as u64) as usize;
            self.datei.schreiben_an(pos + erledigt, &nullen[..stueck])?;
            erledigt += stueck as u64;
        }
        Ok(())
    }

    /// Ein gepufferter Leser für eine sequentielle Kette über die ganze Datei.
    pub fn kette(&self) -> Kette<'_, 'a> {
        self.kette_bis(self.laenge)
    }

    /// Ein gepufferter Leser, der nie über `ende` hinaus liest oder schreibt — das Ende des Bereichs, den ein
    /// Verweis nennt (ADR-0047 B1 Regel 3).
    pub fn kette_bis(&self, ende: u64) -> Kette<'_, 'a> {
        Kette {
            port: self,
            ende: ende.min(self.laenge),
            anfang: 0,
            puffer: Vec::new(),
            geaendert: false,
        }
    }

    /// Bucht `n` Bytes Arbeit vor dem Zugriff; wer das Budget überschreiten würde, bricht ab, und danach gibt es
    /// keinen Zugriff mehr.
    fn buchen(&self, n: u64) -> Ergebnis<()> {
        let verbraucht = self.verbraucht.get().saturating_add(n);
        if self.erschoepft.get() || verbraucht > self.budget {
            self.erschoepft.set(true);
            return Err(Abbruch::Metadaten("Arbeit über dem Budget"));
        }
        self.verbraucht.set(verbraucht);
        Ok(())
    }

    fn voll_lesen(&self, pos: u64, puffer: &mut [u8]) -> Ergebnis<()> {
        self.buchen(puffer.len() as u64)?;
        let mut gelesen = 0;
        while gelesen < puffer.len() {
            let n = self
                .datei
                .lesen_an(pos + gelesen as u64, &mut puffer[gelesen..])?;
            if n == 0 {
                return Err(io::Error::from(io::ErrorKind::UnexpectedEof).into());
            }
            gelesen += n;
        }
        Ok(())
    }
}

/// Gepufferter Leser über eine sequentielle Kette (JPEG-Segmente, PNG-Chunks, GIF-Blöcke und -Unterblöcke): ein
/// Puffer ≤ 1 MiB über `pread`, nie ein Systemaufruf je Unterblock; im Puffer genullte Bytes werden beim Weiterrücken
/// und mit `fertig` zurückgeschrieben.
pub struct Kette<'p, 'a> {
    port: &'p Port<'a>,
    ende: u64,
    anfang: u64,
    puffer: Vec<u8>,
    geaendert: bool,
}

impl Kette<'_, '_> {
    /// Ob `[pos, pos + n)` ganz im Bereich der Kette liegt (ohne Überlauf).
    pub fn enthaelt(&self, pos: u64, n: u64) -> bool {
        pos.checked_add(n).is_some_and(|ende| ende <= self.ende)
    }

    /// Die Bytes `[pos, pos + n)` mit n ≤ 1 MiB, ganz in der Datei — sonst `metadaten`.
    pub fn bytes(&mut self, pos: u64, n: usize) -> Ergebnis<&[u8]> {
        self.laden(pos, n)?;
        let stelle = (pos - self.anfang) as usize;
        Ok(&self.puffer[stelle..stelle + n])
    }

    pub fn byte(&mut self, pos: u64) -> Ergebnis<u8> {
        Ok(self.bytes(pos, 1)?[0])
    }

    /// Nullt `[pos, pos + n)`: bis 1 MiB im Puffer (zurückgeschrieben beim Weiterrücken oder mit `fertig`),
    /// größer als Nullziel direkt in Stücken ≤ 1 MiB, nie gelesen.
    pub fn nullen(&mut self, pos: u64, n: u64) -> Ergebnis<()> {
        if !self.enthaelt(pos, n) {
            return Err(Abbruch::Metadaten("Längen widersprechen sich"));
        }
        if n > STUECK as u64 {
            self.fertig()?;
            self.puffer.clear();
            return self.port.nullen(pos, n);
        }
        let n = n as usize;
        self.laden(pos, n)?;
        let stelle = (pos - self.anfang) as usize;
        self.puffer[stelle..stelle + n].fill(0);
        self.geaendert |= n > 0;
        Ok(())
    }

    /// Schreibt höchstens 1 MiB über den Puffer.
    pub fn schreiben(&mut self, pos: u64, bytes: &[u8]) -> Ergebnis<()> {
        self.laden(pos, bytes.len())?;
        let stelle = (pos - self.anfang) as usize;
        self.puffer[stelle..stelle + bytes.len()].copy_from_slice(bytes);
        self.geaendert |= !bytes.is_empty();
        Ok(())
    }

    /// Schreibt geänderte Bytes des Puffers zurück.
    pub fn fertig(&mut self) -> Ergebnis<()> {
        if self.geaendert {
            self.port.schreiben(self.anfang, &self.puffer)?;
            self.geaendert = false;
        }
        Ok(())
    }

    fn laden(&mut self, pos: u64, n: usize) -> Ergebnis<()> {
        if n > STUECK {
            return Err(Abbruch::Metadaten("Leseziel über 1 MiB"));
        }
        if !self.enthaelt(pos, n as u64) {
            return Err(Abbruch::Metadaten("Längen widersprechen sich"));
        }
        let ende = self.anfang + self.puffer.len() as u64;
        if pos >= self.anfang && pos + n as u64 <= ende {
            return Ok(());
        }
        self.fertig()?;
        let laenge = (self.ende - pos).min(STUECK as u64) as usize;
        self.puffer.resize(laenge, 0);
        self.anfang = pos;
        self.port.voll_lesen(pos, &mut self.puffer)?;
        Ok(())
    }
}

#[cfg(test)]
pub mod attrappe {
    //! Eine Teildatei im Speicher, die jede Lese- und Schreiblänge festhält.

    use crate::gespraeche::anhangablage::Teildatei;
    use std::io;
    use std::sync::Mutex;

    #[derive(Default)]
    pub struct Datei {
        pub daten: Mutex<Vec<u8>>,
        pub lesungen: Mutex<Vec<usize>>,
        pub schreibungen: Mutex<Vec<usize>>,
    }

    impl Datei {
        pub fn mit(daten: Vec<u8>) -> Datei {
            Datei {
                daten: Mutex::new(daten),
                ..Datei::default()
            }
        }

        pub fn inhalt(&self) -> Vec<u8> {
            self.daten.lock().expect("Daten").clone()
        }

        pub fn groesste_lesung(&self) -> usize {
            self.lesungen
                .lock()
                .expect("Lesungen")
                .iter()
                .copied()
                .max()
                .unwrap_or(0)
        }

        pub fn groesste_schreibung(&self) -> usize {
            self.schreibungen
                .lock()
                .expect("Schreibungen")
                .iter()
                .copied()
                .max()
                .unwrap_or(0)
        }

        pub fn anzahl_lesungen(&self) -> usize {
            self.lesungen.lock().expect("Lesungen").len()
        }
    }

    impl Teildatei for Datei {
        fn anhaengen(&mut self, bytes: &[u8]) -> io::Result<()> {
            self.daten.lock().expect("Daten").extend_from_slice(bytes);
            Ok(())
        }

        fn lesen_an(&self, pos: u64, puffer: &mut [u8]) -> io::Result<usize> {
            self.lesungen.lock().expect("Lesungen").push(puffer.len());
            let daten = self.daten.lock().expect("Daten");
            let pos = pos as usize;
            if pos >= daten.len() {
                return Ok(0);
            }
            let n = puffer.len().min(daten.len() - pos);
            puffer[..n].copy_from_slice(&daten[pos..pos + n]);
            Ok(n)
        }

        fn schreiben_an(&self, pos: u64, bytes: &[u8]) -> io::Result<()> {
            self.schreibungen
                .lock()
                .expect("Schreibungen")
                .push(bytes.len());
            let mut daten = self.daten.lock().expect("Daten");
            let pos = pos as usize;
            assert!(
                pos + bytes.len() <= daten.len(),
                "die Länge ändert sich nie"
            );
            daten[pos..pos + bytes.len()].copy_from_slice(bytes);
            Ok(())
        }

        fn laenge(&self) -> io::Result<u64> {
            Ok(self.daten.lock().expect("Daten").len() as u64)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::attrappe::Datei;
    use super::*;

    const MIB: usize = 1_048_576;

    #[test]
    fn nullen_ueber_drei_mib_schreibt_nur_stuecke_bis_1_mib_und_liest_nie() {
        let datei = Datei::mit(vec![7; 4 * MIB]);
        let port = Port::neu(&datei).expect("Port");

        port.nullen(10, 3 * MIB as u64).expect("genullt");

        let inhalt = datei.inhalt();
        assert!(inhalt[10..10 + 3 * MIB].iter().all(|byte| *byte == 0));
        assert_eq!((inhalt[9], inhalt[10 + 3 * MIB]), (7, 7));
        assert_eq!(datei.groesste_schreibung(), MIB);
        assert_eq!(datei.anzahl_lesungen(), 0);
        assert_eq!(inhalt.len(), 4 * MIB);
    }

    #[test]
    fn ein_leseziel_ueber_1_mib_ist_ein_fehler_ohne_grossen_puffer() {
        let datei = Datei::mit(vec![0; 2 * MIB]);
        let port = Port::neu(&datei).expect("Port");

        let gelesen = port.lesen(0, MIB as u64 + 1);

        assert!(matches!(
            gelesen,
            Err(Abbruch::Metadaten("Leseziel über 1 MiB"))
        ));
        assert_eq!(datei.anzahl_lesungen(), 0);
        assert_eq!(port.lesen(0, MIB as u64).expect("1 MiB").len(), MIB);
    }

    fn ueber_dem_budget(ergebnis: Ergebnis<()>) -> bool {
        matches!(ergebnis, Err(Abbruch::Metadaten("Arbeit über dem Budget")))
    }

    /// ADR-0047 B1 P4 (B0637): Budget 100 Bytes, Leser fordert 101 → „Arbeit über dem Budget“, kein Zugriff danach.
    #[test]
    fn ein_leser_ueber_dem_budget_bricht_vor_dem_zugriff_ab_und_greift_danach_nie_mehr_zu() {
        let datei = Datei::mit(vec![3; 1_000]);
        let port = Port::mit_budget(&datei, 1_000, 100);

        assert!(ueber_dem_budget(port.lesen(0, 101).map(drop)));
        assert!(ueber_dem_budget(port.lesen(0, 1).map(drop)));
        assert!(ueber_dem_budget(port.schreiben(0, &[1])));
        assert!(ueber_dem_budget(port.nullen(0, 1)));
        assert!(ueber_dem_budget(port.kette().bytes(0, 1).map(drop)));

        assert_eq!(datei.anzahl_lesungen(), 0);
        assert_eq!(datei.groesste_schreibung(), 0);
        assert_eq!(datei.inhalt(), vec![3; 1_000]);
    }

    /// Lesen, Schreiben und Nullen zählen gemeinsam; genau am Budget geht es noch, ein Byte darüber nicht.
    #[test]
    fn lesen_schreiben_und_nullen_zaehlen_gemeinsam_bis_genau_zum_budget() {
        let datei = Datei::mit(vec![3; 1_000]);
        let port = Port::mit_budget(&datei, 1_000, 100);

        assert_eq!(port.lesen(0, 40).expect("40").len(), 40);
        port.schreiben(0, &[1; 30]).expect("30");
        port.nullen(500, 30).expect("30");
        assert_eq!(port.verbraucht(), 100);
        assert!(ueber_dem_budget(port.nullen(600, 1)));

        assert_eq!(datei.anzahl_lesungen(), 1);
        assert_eq!(datei.inhalt()[600], 3);
    }

    /// Das Budget eines Ports ist 2 × Länge + 8 MiB: eine Datei von 1 MiB darf zehnmal ganz gelesen werden.
    #[test]
    fn das_budget_ist_zweimal_die_laenge_und_8_mib() {
        let datei = Datei::mit(vec![0; MIB]);
        let port = Port::neu(&datei).expect("Port");

        for _ in 0..10 {
            port.lesen(0, MIB as u64).expect("im Budget");
        }
        assert!(ueber_dem_budget(port.lesen(0, 1).map(drop)));
        assert_eq!(datei.anzahl_lesungen(), 10);
    }

    #[test]
    fn nichts_ausserhalb_der_datei() {
        let datei = Datei::mit(vec![0; 100]);
        let port = Port::neu(&datei).expect("Port");

        assert!(matches!(port.lesen(90, 11), Err(Abbruch::Metadaten(_))));
        assert!(matches!(
            port.schreiben(99, &[1, 2]),
            Err(Abbruch::Metadaten(_))
        ));
        assert!(matches!(
            port.nullen(u64::MAX, 2),
            Err(Abbruch::Metadaten(_))
        ));
        assert_eq!(datei.inhalt(), vec![0; 100]);
    }

    #[test]
    fn die_kette_liest_viele_kleine_bloecke_mit_wenigen_lesevorgaengen_und_schreibt_genulltes_zurueck()
     {
        let datei = Datei::mit(vec![5; 3 * MIB]);
        let port = Port::neu(&datei).expect("Port");
        let mut kette = port.kette();

        for pos in (0..3 * MIB as u64).step_by(256) {
            assert_eq!(kette.byte(pos).expect("Byte"), 5);
            kette.nullen(pos + 1, 10).expect("genullt");
        }
        kette.fertig().expect("zurückgeschrieben");

        assert!(datei.anzahl_lesungen() <= 3, "{}", datei.anzahl_lesungen());
        assert!(datei.groesste_lesung() <= MIB && datei.groesste_schreibung() <= MIB);
        let inhalt = datei.inhalt();
        assert_eq!((inhalt[0], inhalt[1], inhalt[10], inhalt[11]), (5, 0, 0, 5));
        assert_eq!(inhalt[256 * 4000 + 1], 0);
    }
}
