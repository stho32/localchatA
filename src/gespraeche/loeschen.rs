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
        let (stellen, eintraege): (Vec<usize>, Vec<Arc<Eintrag>>) = gewonnen.into_iter().unzip();
        let mut erfolge = Vec::new();
        let mut dateien = Vec::new();
        for (stelle, (eintrag, datei)) in stellen
            .into_iter()
            .zip(self.kuerzen_und_abziehen(eintraege))
        {
            ergebnisse[stelle].1 = match datei.grund {
                None => {
                    erfolge.push(eintrag);
                    Einzelergebnis::Geloescht
                }
                Some(_) => Einzelergebnis::Fehler,
            };
            dateien.push(datei);
        }
        self.anhangschwelle_pruefen("geloescht");
        let mut bericht = self.bericht(ergebnisse, Vec::new(), dateien);
        bericht.umfang = umfang(aufrufer, erfolge.iter());
        bericht.je_gespraech = je_gespraech(&erfolge);
        Loeschen::Geloescht(bericht)
    }

    /// Die gewonnenen Anhänge einer gelöschten Nachricht kürzen und abziehen (ADR-0048 Regel 9 b, c): wie im
    /// Laufwerk, nur bleibt ein Anhang, dessen Kürzen scheitert, `geloescht` (410) und gezählt — die Nachricht ist
    /// gelöscht, ihr Anhang darf nicht wieder abrufbar werden; der nächste Start kürzt ihn. Aufrufen ohne Sperre,
    /// außerhalb des Laufzeit-Threads.
    pub(super) fn anhaenge_der_nachricht_kuerzen(
        &self,
        absender: &Kennung,
        gewonnen: Vec<Arc<Eintrag>>,
    ) -> Geloescht {
        let mut ergebnisse = Vec::new();
        let mut erfolge = Vec::new();
        let mut dateien = Vec::new();
        for (eintrag, datei) in self.kuerzen_und_abziehen(gewonnen) {
            let ergebnis = match datei.grund {
                None => Einzelergebnis::Geloescht,
                Some(_) => Einzelergebnis::Fehler,
            };
            ergebnisse.push((eintrag.id.clone(), ergebnis));
            if ergebnis == Einzelergebnis::Geloescht {
                erfolge.push(eintrag);
            }
            dateien.push(datei);
        }
        self.anhangschwelle_pruefen("geloescht");
        let mut bericht = self.bericht(ergebnisse, je_gespraech(&erfolge), dateien);
        bericht.umfang = umfang(absender, erfolge.iter());
        bericht
    }

    /// Schritte (b) und (c) für gewonnene Einträge (Zustand schon `geloescht`): (b) ohne Sperre je Datei auf 0 Bytes
    /// kürzen, (c) unter der Schreibsperre je Erfolg `G −= z`, wenn der Abzug nicht schon an das Entfallen des Tags
    /// ging (Befund #67 B2), und den Zähler `geloescht` erhöhen; je Fehler zurück nach `vorhanden` (F1′ Regel 5 c) —
    /// außer die Nachricht des Anhangs ist gelöscht, dann bleibt er `geloescht` und gezählt (ADR-0048 Regel 9 c), gleich
    /// wer ihn gewann (Befund #67 B1). Das Merkmal setzt das Nachricht-Löschen unter derselben Schreibsperre, unter
    /// der hier zurückgesetzt wird. Liefert je Eintrag die Datei mit Grund des Fehlers (ohne Grund: gekürzt).
    fn kuerzen_und_abziehen(&self, gewonnen: Vec<Arc<Eintrag>>) -> Vec<(Arc<Eintrag>, Gekuerzt)> {
        let gekuerzt: Vec<(Arc<Eintrag>, Result<(), String>)> = gewonnen
            .into_iter()
            .map(|eintrag| {
                let ergebnis = self
                    .anhangablage
                    .kuerzen(&eintrag.gespraech, eintrag.tag, &eintrag.datei)
                    .map_err(|fehler| fehler.to_string());
                (eintrag, ergebnis)
            })
            .collect();
        let _stand = sperren(&self.schreiben);
        gekuerzt
            .into_iter()
            .map(|(eintrag, ergebnis)| {
                let grund = match ergebnis {
                    Ok(()) => {
                        if eintrag.abzug_gewinnen() {
                            let _ = self.anhangzaehler.gebunden.fetch_update(
                                Ordering::SeqCst,
                                Ordering::SeqCst,
                                |g| Some(g.saturating_sub(eintrag.z())),
                            );
                        }
                        self.anhangzaehler.geloescht.fetch_add(1, Ordering::SeqCst);
                        None
                    }
                    Err(grund) => {
                        if !eintrag.nachricht_geloescht() {
                            eintrag.tauschen(Zustand::Geloescht, Zustand::Vorhanden);
                        }
                        Some(grund)
                    }
                };
                let datei = Gekuerzt {
                    gespraech: eintrag.gespraech.clone(),
                    tag: eintrag.tag,
                    datei: praefix(&eintrag.datei),
                    bytes: eintrag.bytes,
                    grund,
                };
                (eintrag, datei)
            })
            .collect()
    }

    /// Nach dem Laden, vor der Startzeile: die Anhänge gelöschter Nachrichten, deren Datei noch Bytes hat, kürzen und
    /// abziehen (ADR-0048 Regel 9 „Atomarität“). Liefert je Datei das Ergebnis fürs Protokoll.
    pub fn geloeschte_anhaenge_kuerzen(&self) -> Vec<Gekuerzt> {
        let gewonnen = std::mem::take(&mut *sperren(&self.geloescht_nachzukuerzen));
        let dateien = self
            .kuerzen_und_abziehen(gewonnen)
            .into_iter()
            .map(|(_, datei)| datei)
            .collect();
        self.anhangschwelle_pruefen("geloescht");
        dateien
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
