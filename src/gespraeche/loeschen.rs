//! Anhänge löschen in drei Schritten (B0507, ADR-0046 Ergänzung „Karte #9 Betreiber F1′“ Regeln 5, 6, 9, 10): (a)
//! unter der Schreibsperre nachschlagen und `vorhanden` → `geloescht`, (b) außerhalb jeder Sperre auf 0 Bytes
//! kürzen, (c) unter der Schreibsperre `G −= z` je Erfolg, je Fehler zurück nach `vorhanden`. Löschen nimmt keine
//! Gesprächssperre.

use super::anhangid::Anhangid;
use super::anhangverzeichnis::{Eintrag, Zustand, praefix};
use super::{Gespraech, Gespraeche, Kennung, sperren};
use crate::uhr::Zeit;
use chrono::NaiveDate;
use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::Ordering;

/// Ergebnis je ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Einzelergebnis {
    Geloescht,
    SchonGeloescht,
    Unbekannt,
    Fehler,
}

impl Einzelergebnis {
    pub fn als_text(self) -> &'static str {
        match self {
            Einzelergebnis::Geloescht => "geloescht",
            Einzelergebnis::SchonGeloescht => "schon_geloescht",
            Einzelergebnis::Unbekannt => "unbekannt",
            Einzelergebnis::Fehler => "fehler",
        }
    }
}

/// Was ein Löschen beträfe oder betraf: Zahl, Bytes, Gespräche, davon fremde.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Umfang {
    pub anhaenge: u64,
    pub bytes: u64,
    pub gespraeche: u64,
    pub fremde: u64,
}

/// Platz an der Anhang-Quote nach dem Löschen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Platz {
    pub hoechstens: Option<u64>,
    pub belegt: u64,
    pub frei: Option<u64>,
}

/// Eine gekürzte Datei — fürs Protokoll nur als Präfix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gekuerzt {
    pub gespraech: Gespraech,
    pub tag: NaiveDate,
    pub datei: String,
    pub bytes: u64,
    pub grund: Option<String>,
}

/// Das Ergebnis einer bestätigten Anfrage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Geloescht {
    pub ergebnisse: Vec<(Anhangid, Einzelergebnis)>,
    /// Gelöscht: Zahl, Bytes, Gespräche, fremde.
    pub umfang: Umfang,
    pub fehler: u64,
    pub platz: Platz,
    /// Je Gespräch die gelöschten (ID, Satzzeit) — fürs Ereignis `anhaenge`.
    pub je_gespraech: Vec<(Gespraech, Vec<(Anhangid, Zeit)>)>,
    /// Je Datei, gekürzt oder gescheitert.
    pub dateien: Vec<Gekuerzt>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Loeschen {
    /// Ohne Bestätigung und mit mindestens einem Anhang `vorhanden`: nichts geändert.
    Bestaetigen(Umfang),
    /// Die Rest-Buchung am Schreibkonto scheiterte: nichts geändert.
    Gebremst,
    Geloescht(Geloescht),
}

/// Rest-Buchung am Schreibkonto: ⌈n / 100⌉ Einheiten.
pub fn rest_einheiten(ids: usize) -> u32 {
    u32::try_from(ids.div_ceil(100)).unwrap_or(u32::MAX)
}

impl Gespraeche {
    /// Löscht gebundene Anhänge, eigene wie fremde. `buchen` bucht die Rest-Einheiten unter der Schreibsperre, bevor
    /// ein Zustand wechselt. Aufrufen außerhalb des Laufzeit-Threads (Schritt b berührt die Platte).
    pub fn anhaenge_loeschen(
        &self,
        aufrufer: &Kennung,
        ids: &[Anhangid],
        bestaetigt: bool,
        buchen: impl FnOnce(u32) -> bool,
    ) -> Loeschen {
        let mut ergebnisse: Vec<(Anhangid, Einzelergebnis)> = ids
            .iter()
            .map(|id| (id.clone(), Einzelergebnis::Unbekannt))
            .collect();
        let gewonnen: Vec<(usize, Arc<Eintrag>)> = {
            let stand = sperren(&self.schreiben);
            let mut kandidaten = Vec::new();
            for (stelle, id) in ids.iter().enumerate() {
                match self.anhangverzeichnis.finden(id) {
                    Some(eintrag) if eintrag.zustand() == Zustand::Vorhanden => {
                        kandidaten.push((stelle, eintrag))
                    }
                    Some(eintrag) if eintrag.zustand() == Zustand::Geloescht => {
                        ergebnisse[stelle].1 = Einzelergebnis::SchonGeloescht;
                    }
                    _ => {}
                }
            }
            if kandidaten.is_empty() {
                drop(stand);
                return Loeschen::Geloescht(self.bericht(ergebnisse, Vec::new(), Vec::new()));
            }
            if !bestaetigt {
                return Loeschen::Bestaetigen(umfang(
                    aufrufer,
                    kandidaten.iter().map(|(_, eintrag)| eintrag),
                ));
            }
            if !buchen(rest_einheiten(ids.len())) {
                return Loeschen::Gebremst;
            }
            kandidaten
                .into_iter()
                .filter(|(stelle, eintrag)| {
                    let gewonnen = eintrag.tauschen(Zustand::Vorhanden, Zustand::Geloescht);
                    if !gewonnen {
                        ergebnisse[*stelle].1 = match eintrag.zustand() {
                            Zustand::Geloescht => Einzelergebnis::SchonGeloescht,
                            _ => Einzelergebnis::Unbekannt,
                        };
                    }
                    gewonnen
                })
                .collect()
        };
        let gekuerzt: Vec<(usize, Arc<Eintrag>, Result<(), String>)> = gewonnen
            .into_iter()
            .map(|(stelle, eintrag)| {
                let ergebnis = self
                    .anhangablage
                    .kuerzen(&eintrag.gespraech, eintrag.tag, &eintrag.datei)
                    .map_err(|fehler| fehler.to_string());
                (stelle, eintrag, ergebnis)
            })
            .collect();
        let mut erfolge = Vec::new();
        let mut dateien = Vec::new();
        {
            let _stand = sperren(&self.schreiben);
            for (stelle, eintrag, ergebnis) in gekuerzt {
                let grund = match ergebnis {
                    Ok(()) => {
                        let _ = self.anhangzaehler.gebunden.fetch_update(
                            Ordering::SeqCst,
                            Ordering::SeqCst,
                            |g| Some(g.saturating_sub(eintrag.z())),
                        );
                        self.anhangzaehler.geloescht.fetch_add(1, Ordering::SeqCst);
                        ergebnisse[stelle].1 = Einzelergebnis::Geloescht;
                        erfolge.push(Arc::clone(&eintrag));
                        None
                    }
                    Err(grund) => {
                        eintrag.tauschen(Zustand::Geloescht, Zustand::Vorhanden);
                        ergebnisse[stelle].1 = Einzelergebnis::Fehler;
                        Some(grund)
                    }
                };
                dateien.push(Gekuerzt {
                    gespraech: eintrag.gespraech.clone(),
                    tag: eintrag.tag,
                    datei: praefix(&eintrag.datei),
                    bytes: eintrag.bytes,
                    grund,
                });
            }
        }
        let mut bericht = self.bericht(ergebnisse, Vec::new(), dateien);
        bericht.umfang = umfang(aufrufer, erfolge.iter());
        bericht.je_gespraech = je_gespraech(&erfolge);
        Loeschen::Geloescht(bericht)
    }

    fn bericht(
        &self,
        ergebnisse: Vec<(Anhangid, Einzelergebnis)>,
        je_gespraech: Vec<(Gespraech, Vec<(Anhangid, Zeit)>)>,
        dateien: Vec<Gekuerzt>,
    ) -> Geloescht {
        let kennzahlen = self.anhangkennzahlen();
        let belegt = kennzahlen.gebunden.saturating_add(kennzahlen.reserviert);
        let fehler = ergebnisse
            .iter()
            .filter(|(_, ergebnis)| *ergebnis == Einzelergebnis::Fehler)
            .count() as u64;
        Geloescht {
            ergebnisse,
            umfang: Umfang::default(),
            fehler,
            platz: Platz {
                hoechstens: kennzahlen.anhangquote,
                belegt,
                frei: kennzahlen
                    .anhangquote
                    .map(|hoechstens| hoechstens.saturating_sub(belegt)),
            },
            je_gespraech,
            dateien,
        }
    }
}

fn umfang<'a>(aufrufer: &Kennung, eintraege: impl Iterator<Item = &'a Arc<Eintrag>>) -> Umfang {
    let mut umfang = Umfang::default();
    let mut gespraeche = BTreeSet::new();
    for eintrag in eintraege {
        umfang.anhaenge += 1;
        umfang.bytes = umfang.bytes.saturating_add(eintrag.bytes);
        if &eintrag.absender != aufrufer {
            umfang.fremde += 1;
        }
        gespraeche.insert(eintrag.gespraech.pfad());
    }
    umfang.gespraeche = gespraeche.len() as u64;
    umfang
}

/// Je Gespräch die gelöschten (ID, Satzzeit), in der Reihenfolge der Anfrage.
fn je_gespraech(erfolge: &[Arc<Eintrag>]) -> Vec<(Gespraech, Vec<(Anhangid, Zeit)>)> {
    let mut liste: Vec<(Gespraech, Vec<(Anhangid, Zeit)>)> = Vec::new();
    for eintrag in erfolge {
        match liste
            .iter_mut()
            .find(|(gespraech, _)| *gespraech == eintrag.gespraech)
        {
            Some((_, ids)) => ids.push((eintrag.id.clone(), eintrag.satzzeit)),
            None => liste.push((
                eintrag.gespraech.clone(),
                vec![(eintrag.id.clone(), eintrag.satzzeit)],
            )),
        }
    }
    liste
}
