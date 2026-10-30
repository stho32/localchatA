//! Anhangverzeichnis (A00016, ADR-0046 Regeln 6–9 und Ergänzung „F1′“ Regeln 1, 5, 8): je Anhang-ID Ort, Größe,
//! Art, Anzeigename, Satzzeit, Absender und Zustand `vorhanden | geloescht | entfallen`. Übergänge nur per
//! Vergleichen-und-Tauschen — wer gewinnt, zieht genau einmal ab. Die Sperre des Verzeichnisses ist ein Blatt: unter
//! ihr wird keine andere genommen.

use super::Kennung;
use super::anhangid::Anhangid;
use super::anhangquote::zaehlmass;
use super::logablage::Gespraech;
use super::logformat::{self, Anhangbefund};
use crate::uhr::Zeit;
use chrono::NaiveDate;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zustand {
    Vorhanden,
    Geloescht,
    Entfallen,
}

impl Zustand {
    fn als_zahl(self) -> u8 {
        match self {
            Zustand::Vorhanden => 0,
            Zustand::Geloescht => 1,
            Zustand::Entfallen => 2,
        }
    }

    fn aus_zahl(zahl: u8) -> Zustand {
        match zahl {
            0 => Zustand::Vorhanden,
            1 => Zustand::Geloescht,
            _ => Zustand::Entfallen,
        }
    }
}

/// Ein gebundener Anhang.
#[derive(Debug)]
pub struct Eintrag {
    pub id: Anhangid,
    pub gespraech: Gespraech,
    pub tag: NaiveDate,
    pub datei: String,
    pub bytes: u64,
    pub art: &'static str,
    pub masse: Option<(u32, u32)>,
    pub name: String,
    /// Zeit des Satzes, an dem der Anhang hängt.
    pub satzzeit: Zeit,
    pub absender: Kennung,
    zustand: AtomicU8,
}

impl Eintrag {
    #[allow(clippy::too_many_arguments)]
    pub fn neu(
        zeile: &logformat::Anhangzeile,
        gespraech: &Gespraech,
        tag: NaiveDate,
        satzzeit: Zeit,
        absender: &Kennung,
        zustand: Zustand,
    ) -> Eintrag {
        Eintrag {
            id: zeile.id.clone(),
            gespraech: gespraech.clone(),
            tag,
            datei: zeile.datei.clone(),
            bytes: zeile.bytes,
            art: zeile.art,
            masse: zeile.masse,
            name: zeile.name.clone(),
            satzzeit,
            absender: absender.clone(),
            zustand: AtomicU8::new(zustand.als_zahl()),
        }
    }

    pub fn zustand(&self) -> Zustand {
        Zustand::aus_zahl(self.zustand.load(Ordering::SeqCst))
    }

    /// Vergleichen-und-Tauschen: `true`, wenn dieser Aufruf den Übergang gewann.
    pub fn tauschen(&self, von: Zustand, nach: Zustand) -> bool {
        self.zustand
            .compare_exchange(
                von.als_zahl(),
                nach.als_zahl(),
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .is_ok()
    }

    /// Das Zählmaß an der Anhang-Quote.
    pub fn z(&self) -> u64 {
        zaehlmass(self.bytes)
    }
}

/// Was beim Laden eines Tages auffiel (`WARNING laden`): die Zeile der Tagesdatei und der Grund; eine Datei nur
/// als Präfix `<hhmmssmmm>-<n>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ladebefund {
    pub zeile: usize,
    pub grund: &'static str,
    pub datei: Option<String>,
}

/// Eine reguläre Datei in `anhaenge/<tag>/` mit ihrer Länge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tagesdatei {
    pub name: String,
    pub laenge: u64,
}

#[derive(Debug, Default)]
struct Tag {
    ids: Vec<Anhangid>,
    /// Σ `z` der regulären Dateien ohne gültige Metazeile — gezählt, nicht löschbar.
    waisen: u64,
}

#[derive(Debug, Default)]
struct Innen {
    eintraege: HashMap<Anhangid, Arc<Eintrag>>,
    tage: HashMap<(Gespraech, NaiveDate), Tag>,
}

#[derive(Debug, Default)]
pub struct Anhangverzeichnis {
    innen: RwLock<Innen>,
}

/// `<hhmmssmmm>-<n>` einer gespeicherten Datei — so darf sie im Protokoll stehen (ADR-0047 Ergänzung Gate Regel 3).
pub fn praefix(datei: &str) -> String {
    let mut teile = datei.splitn(3, '-');
    match (teile.next(), teile.next()) {
        (Some(zeit), Some(nummer)) => format!("{zeit}-{nummer}"),
        _ => String::new(),
    }
}

impl Anhangverzeichnis {
    fn lesend(&self) -> std::sync::RwLockReadGuard<'_, Innen> {
        self.innen
            .read()
            .unwrap_or_else(|vergiftet| vergiftet.into_inner())
    }

    fn schreibend(&self) -> std::sync::RwLockWriteGuard<'_, Innen> {
        self.innen
            .write()
            .unwrap_or_else(|vergiftet| vergiftet.into_inner())
    }

    /// Beim Binden (unter der Schreibsperre): ein neuer Eintrag `vorhanden`.
    pub fn eintragen(&self, eintrag: Eintrag) {
        let mut innen = self.schreibend();
        let schluessel = (eintrag.gespraech.clone(), eintrag.tag);
        innen
            .tage
            .entry(schluessel)
            .or_default()
            .ids
            .push(eintrag.id.clone());
        innen
            .eintraege
            .insert(eintrag.id.clone(), Arc::new(eintrag));
    }

    pub fn finden(&self, id: &Anhangid) -> Option<Arc<Eintrag>> {
        self.lesend().eintraege.get(id).cloned()
    }

    /// Eine Waise im Tag (Datei ohne Metazeile, oder Zurücklegen gescheitert): zählt an `G` bis zum Entfallen.
    pub fn waise(&self, gespraech: &Gespraech, tag: NaiveDate, z: u64) {
        let mut innen = self.schreibend();
        let eintrag = innen.tage.entry((gespraech.clone(), tag)).or_default();
        eintrag.waisen = eintrag.waisen.saturating_add(z);
    }

    /// Σ `z` der Einträge `vorhanden` plus Waisen eines Tages.
    pub fn bytes_je_tag(&self, gespraech: &Gespraech, tag: NaiveDate) -> u64 {
        let innen = self.lesend();
        let Some(eintrag) = innen.tage.get(&(gespraech.clone(), tag)) else {
            return 0;
        };
        eintrag
            .ids
            .iter()
            .filter_map(|id| innen.eintraege.get(id))
            .filter(|eintrag| eintrag.zustand() == Zustand::Vorhanden)
            .map(|eintrag| eintrag.z())
            .sum::<u64>()
            + eintrag.waisen
    }

    /// Der Tag entfällt: jeder Eintrag `vorhanden` → `entfallen` (wer gewinnt, zieht ab), dann aus dem Verzeichnis;
    /// liefert Σ `z` der gewonnenen plus Waisen — Gelöschte zieht der Löschende ab.
    pub fn tag_entfernen(&self, gespraech: &Gespraech, tag: NaiveDate) -> u64 {
        let ids = {
            let innen = self.lesend();
            match innen.tage.get(&(gespraech.clone(), tag)) {
                Some(eintrag) => eintrag.ids.clone(),
                None => return 0,
            }
        };
        let mut abzuziehen = 0u64;
        let mut entfallen = Vec::new();
        {
            let innen = self.lesend();
            for id in &ids {
                if let Some(eintrag) = innen.eintraege.get(id) {
                    if eintrag.tauschen(Zustand::Vorhanden, Zustand::Entfallen) {
                        abzuziehen = abzuziehen.saturating_add(eintrag.z());
                    }
                    entfallen.push(Arc::clone(eintrag));
                }
            }
        }
        let mut innen = self.schreibend();
        let waisen = innen
            .tage
            .remove(&(gespraech.clone(), tag))
            .map_or(0, |eintrag| eintrag.waisen);
        for eintrag in entfallen {
            innen.eintraege.remove(&eintrag.id);
        }
        abzuziehen.saturating_add(waisen)
    }

    /// Die Anhänge eines Tags ließen sich nach seiner Tagesdatei nicht entfernen (ADR-0046 Ergänzung „Review #53 B1“
    /// Regel 4): jeder Eintrag `vorhanden` → `entfallen` (wer gewinnt, gibt sein Zählmaß an die Waisen des Tags), alle
    /// Einträge aus dem Verzeichnis; der Tag bleibt mit `W` — `G` ändert sich nicht. Liefert das neue `W`.
    pub fn tag_verwaisen(&self, gespraech: &Gespraech, tag: NaiveDate) -> u64 {
        let mut innen = self.schreibend();
        let Some(ids) = innen
            .tage
            .get_mut(&(gespraech.clone(), tag))
            .map(|eintrag| std::mem::take(&mut eintrag.ids))
        else {
            return 0;
        };
        let mut gewonnen = 0u64;
        for id in ids {
            if let Some(eintrag) = innen.eintraege.remove(&id)
                && eintrag.tauschen(Zustand::Vorhanden, Zustand::Entfallen)
            {
                gewonnen = gewonnen.saturating_add(eintrag.z());
            }
        }
        let eintrag = innen.tage.entry((gespraech.clone(), tag)).or_default();
        eintrag.waisen = eintrag.waisen.saturating_add(gewonnen);
        eintrag.waisen
    }

    /// Lädt einen einzelnen Tag (`tag_lesen` und `tag_eintragen`); doppelt ist eine ID nur innerhalb des Tags.
    pub fn tag_laden(
        &self,
        gespraech: &Gespraech,
        tag: NaiveDate,
        rohtext: &[u8],
        dateien: &[Tagesdatei],
    ) -> (Vec<Ladebefund>, u64) {
        let gelesen = tag_lesen(rohtext);
        let doppelt = doppelte_ids([&gelesen]);
        self.tag_eintragen(gespraech, tag, gelesen, dateien, &doppelt)
    }

    /// Trägt einen gelesenen Tag ein, mit den regulären Dateien in `anhaenge/<tag>/`: je gültiger Metazeile, deren
    /// ID nicht in `doppelt` steht und deren Datei es mit genau `bytes` Bytes gibt, ein Eintrag `vorhanden` (Länge 0
    /// bei `bytes` > 0: `geloescht`); jede andere Datei ist eine Waise. Liefert die Befunde (die des Lesens zuerst)
    /// und `G` des Tages.
    pub fn tag_eintragen(
        &self,
        gespraech: &Gespraech,
        tag: NaiveDate,
        gelesen: Tageszeilen,
        dateien: &[Tagesdatei],
        doppelt: &HashSet<Anhangid>,
    ) -> (Vec<Ladebefund>, u64) {
        let Tageszeilen {
            zeilen,
            mut befunde,
        } = gelesen;
        let laengen: HashMap<&str, u64> = dateien
            .iter()
            .map(|datei| (datei.name.as_str(), datei.laenge))
            .collect();
        let mut belegt: HashSet<String> = HashSet::new();
        let mut gebunden = 0u64;
        for Metazeile {
            zeile,
            anhang,
            satzzeit,
            von,
        } in zeilen
        {
            let befund = |grund| Ladebefund {
                zeile,
                grund,
                datei: Some(praefix(&anhang.datei)),
            };
            if doppelt.contains(&anhang.id) {
                befunde.push(befund("ID doppelt"));
                continue;
            }
            let zustand = match laengen.get(anhang.datei.as_str()) {
                None => {
                    befunde.push(befund("Datei fehlt"));
                    continue;
                }
                Some(0) => Zustand::Geloescht,
                Some(laenge) if *laenge == anhang.bytes => Zustand::Vorhanden,
                Some(_) => {
                    befunde.push(befund("Länge stimmt nicht"));
                    continue;
                }
            };
            belegt.insert(anhang.datei.clone());
            if zustand == Zustand::Vorhanden {
                gebunden = gebunden.saturating_add(zaehlmass(anhang.bytes));
            }
            self.eintragen(Eintrag::neu(
                &anhang, gespraech, tag, satzzeit, &von, zustand,
            ));
        }
        let waisen: u64 = dateien
            .iter()
            .filter(|datei| !belegt.contains(&datei.name))
            .map(|datei| zaehlmass(datei.laenge))
            .sum();
        if waisen > 0 {
            self.waise(gespraech, tag, waisen);
        }
        (befunde, gebunden.saturating_add(waisen))
    }

    /// Zahl der Einträge (alle Zustände).
    pub fn anzahl(&self) -> usize {
        self.lesend().eintraege.len()
    }
}

/// Eine gültige Metazeile `+ anhang:` mit ihrer Zeile und ihrem Satz.
#[derive(Debug)]
pub struct Metazeile {
    zeile: usize,
    anhang: logformat::Anhangzeile,
    satzzeit: Zeit,
    von: Kennung,
}

/// Die Metazeilen eines Tages und die Befunde unlesbarer Zeilen (M17), noch ohne Dateien.
#[derive(Debug, Default)]
pub struct Tageszeilen {
    zeilen: Vec<Metazeile>,
    befunde: Vec<Ladebefund>,
}

/// Liest die Metazeilen der Nachrichtensätze aus dem Rohtext einer Tagesdatei; ohne Tagesdatei (Tag nur in `anhaenge/`) leer.
pub fn tag_lesen(rohtext: &[u8]) -> Tageszeilen {
    let mut gelesen = Tageszeilen::default();
    let (mut gezaehlt_bis, mut zeilen_davor) = (0, 0);
    for satz in logformat::saetze(rohtext).into_iter().flatten() {
        let text = String::from_utf8_lossy(&rohtext[satz.anfang..satz.ende]);
        zeilen_davor += rohtext[gezaehlt_bis..satz.anfang]
            .iter()
            .filter(|byte| **byte == b'\n')
            .count();
        gezaehlt_bis = satz.anfang;
        // `+ anhang:` gilt nur in Nachrichtensätzen; ein Änderungssatz (auch ein defekter) trägt keinen Anhang.
        if logformat::satzart(&rohtext[satz.anfang..satz.ende]) != logformat::Satzart::Nachricht {
            continue;
        }
        for (im_satz, ergebnis) in logformat::anhaenge_im_satz(&text) {
            let zeile = zeilen_davor + im_satz;
            match ergebnis {
                Ok(anhang) => gelesen.zeilen.push(Metazeile {
                    zeile,
                    anhang,
                    satzzeit: satz.zeit,
                    von: satz.von.clone(),
                }),
                Err(befund) => gelesen.befunde.push(Ladebefund {
                    zeile,
                    grund: befundtext(befund),
                    datei: None,
                }),
            }
        }
    }
    gelesen
}

/// Die IDs, die über alle gelesenen Tage (aller Gespräche) mehr als einmal vorkommen — sie ergeben keinen Eintrag
/// (Befund #53 B2: eine ID gehört genau einem Tag eines Gesprächs).
pub fn doppelte_ids<'a>(tage: impl IntoIterator<Item = &'a Tageszeilen>) -> HashSet<Anhangid> {
    let mut gesehen: HashSet<&Anhangid> = HashSet::new();
    let mut doppelt = HashSet::new();
    for metazeile in tage.into_iter().flat_map(|tag| &tag.zeilen) {
        if !gesehen.insert(&metazeile.anhang.id) {
            doppelt.insert(metazeile.anhang.id.clone());
        }
    }
    doppelt
}

fn befundtext(befund: Anhangbefund) -> &'static str {
    match befund {
        Anhangbefund::Id => "Metazeile anhang: ID unlesbar",
        Anhangbefund::Bytes => "Metazeile anhang: Bytes unlesbar",
        Anhangbefund::Art => "Metazeile anhang: Art außerhalb der Tabelle",
        Anhangbefund::Masse => "Metazeile anhang: Maße unlesbar",
        Anhangbefund::Datei => "Metazeile anhang: Dateiname unzulässig",
        Anhangbefund::Name => "Metazeile anhang: Anzeigename fehlt",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaa";
    const ID_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbb";
    const ID_C: &str = "cccccccccccccccccccccccccc";

    fn tag() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 29).expect("Tag")
    }

    fn allgemein() -> Gespraech {
        Gespraech::channel("allgemein")
    }

    fn datei(name: &str, laenge: u64) -> Tagesdatei {
        Tagesdatei {
            name: name.to_string(),
            laenge,
        }
    }

    fn rohtext() -> String {
        format!(
            "@ 2026-10-29T14:00:00.000+01:00 beta@127.0.0.1\n+ art: agent\n\
             + anhang: {ID_A} 1 application/octet-stream - 140000000-1-a.bin a.bin\n\
             + anhang: {ID_B} 65537 image/png 2x3 140000000-2-b.png Bild B.png\n  zwei\n\n\
             @ 2026-10-29T14:01:00.000+01:00 gamma@127.0.0.1\n+ anhang: zu kurz\n  kaputt\n\n"
        )
    }

    /// A00016 „Erster Test“: zwei Sätze, einer mit zwei `+ anhang:`-Zeilen, einer mit einer kaputten → zwei
    /// Einträge, ein Befund; `finden` liefert Gespräch, Tag, Datei, Satzzeit und Absender; `tag_entfernen` →
    /// `finden` = `None`, Rückgabe = Σ Zählmaß (1 Byte → 65.536; 65.537 → 69.632).
    #[test]
    fn ein_tag_mit_zwei_anhaengen_und_einer_kaputten_zeile() {
        let verzeichnis = Anhangverzeichnis::default();
        let (befunde, gebunden) = verzeichnis.tag_laden(
            &allgemein(),
            tag(),
            rohtext().as_bytes(),
            &[
                datei("140000000-1-a.bin", 1),
                datei("140000000-2-b.png", 65_537),
            ],
        );

        assert_eq!(
            befunde,
            vec![Ladebefund {
                zeile: 8,
                grund: "Metazeile anhang: ID unlesbar",
                datei: None
            }]
        );
        assert_eq!(gebunden, 65_536 + 69_632);
        let a = verzeichnis
            .finden(&Anhangid::lesen(ID_A).expect("ID"))
            .expect("a");
        assert_eq!(
            (&a.gespraech, a.tag, a.datei.as_str()),
            (&allgemein(), tag(), "140000000-1-a.bin")
        );
        assert_eq!(
            a.satzzeit,
            Zeit::parse_from_rfc3339("2026-10-29T14:00:00+01:00").expect("Zeit")
        );
        assert_eq!(a.absender.als_text(), "beta@127.0.0.1");
        assert_eq!(a.zustand(), Zustand::Vorhanden);
        let b = verzeichnis
            .finden(&Anhangid::lesen(ID_B).expect("ID"))
            .expect("b");
        assert_eq!(
            (b.art, b.masse, b.name.as_str()),
            ("image/png", Some((2, 3)), "Bild B.png")
        );
        assert_eq!(
            verzeichnis.bytes_je_tag(&allgemein(), tag()),
            65_536 + 69_632
        );

        assert_eq!(
            verzeichnis.tag_entfernen(&allgemein(), tag()),
            65_536 + 69_632
        );
        assert!(
            verzeichnis
                .finden(&Anhangid::lesen(ID_A).expect("ID"))
                .is_none()
        );
        assert_eq!(verzeichnis.anzahl(), 0);
        assert_eq!(a.zustand(), Zustand::Entfallen);
    }

    /// M17 und Waisen: `bytes` ≠ Datei, Datei fehlt, zwei Zeilen mit derselben ID → kein Eintrag, je ein Befund mit
    /// der Datei nur als Präfix; eine Datei ohne Metazeile zählt als Waise; Länge 0 bei `bytes` > 0 → `geloescht`,
    /// zählt 0, kein Befund.
    #[test]
    fn unplausible_metazeilen_und_waisen() {
        let rohtext = format!(
            "@ 2026-10-29T14:00:00.000+01:00 beta@127.0.0.1\n\
             + anhang: {ID_A} 5 application/octet-stream - 140000000-1-Geheim.bin Geheim.bin\n\
             + anhang: {ID_B} 5 application/octet-stream - 140000000-2-fehlt.bin fehlt.bin\n\
             + anhang: {ID_C} 5 application/octet-stream - 140000000-3-weg.bin weg.bin\n\n\
             @ 2026-10-29T14:01:00.000+01:00 beta@127.0.0.1\n\
             + anhang: dddddddddddddddddddddddddd 5 application/octet-stream - 140000000-4-x.bin x\n\
             + anhang: dddddddddddddddddddddddddd 5 application/octet-stream - 140000000-5-y.bin y\n\n"
        );
        let verzeichnis = Anhangverzeichnis::default();
        let (befunde, gebunden) = verzeichnis.tag_laden(
            &allgemein(),
            tag(),
            rohtext.as_bytes(),
            &[
                datei("140000000-1-Geheim.bin", 6),
                datei("140000000-3-weg.bin", 0),
                datei("140000000-4-x.bin", 5),
                datei("140000000-5-y.bin", 5),
                datei("140000000-9-waise.bin", 1),
            ],
        );

        let gruende: Vec<(&str, Option<&str>)> = befunde
            .iter()
            .map(|befund| (befund.grund, befund.datei.as_deref()))
            .collect();
        assert_eq!(
            gruende,
            vec![
                ("Länge stimmt nicht", Some("140000000-1")),
                ("Datei fehlt", Some("140000000-2")),
                ("ID doppelt", Some("140000000-4")),
                ("ID doppelt", Some("140000000-5")),
            ]
        );
        assert!(
            befunde
                .iter()
                .all(|befund| !format!("{befund:?}").contains("Geheim"))
        );
        let c = verzeichnis
            .finden(&Anhangid::lesen(ID_C).expect("ID"))
            .expect("c");
        assert_eq!(c.zustand(), Zustand::Geloescht);
        assert_eq!(verzeichnis.anzahl(), 1);
        let waisen = 3 * 65_536 + 65_536;
        assert_eq!(
            gebunden, waisen,
            "Waisen: Geheim (Länge falsch), x, y, waise; gelöscht zählt 0"
        );
        assert_eq!(verzeichnis.tag_entfernen(&allgemein(), tag()), waisen);
    }

    /// Vergleichen-und-Tauschen: von zwei Übergängen aus `vorhanden` gewinnt genau einer; Entfallen zieht Gelöschte
    /// nicht ab (Mutante „Entfallen zieht Gelöschte ab“).
    #[test]
    fn genau_ein_uebergang_gewinnt_und_entfallen_zieht_geloeschte_nicht_ab() {
        let verzeichnis = Anhangverzeichnis::default();
        verzeichnis.tag_laden(
            &allgemein(),
            tag(),
            rohtext().as_bytes(),
            &[
                datei("140000000-1-a.bin", 1),
                datei("140000000-2-b.png", 65_537),
            ],
        );
        let a = verzeichnis
            .finden(&Anhangid::lesen(ID_A).expect("ID"))
            .expect("a");
        assert!(a.tauschen(Zustand::Vorhanden, Zustand::Geloescht));
        assert!(!a.tauschen(Zustand::Vorhanden, Zustand::Geloescht));
        assert_eq!(verzeichnis.bytes_je_tag(&allgemein(), tag()), 69_632);

        assert_eq!(verzeichnis.tag_entfernen(&allgemein(), tag()), 69_632);
        assert_eq!(a.zustand(), Zustand::Geloescht);
    }

    #[test]
    fn der_praefix_einer_datei() {
        assert_eq!(praefix("140712345-1-Bericht_M_rz.pdf"), "140712345-1");
        assert_eq!(praefix("kaputt"), "");
    }
}
