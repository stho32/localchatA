//! Aktivität (A00019, ADR-0051 Regeln 1–3): je Kennung und Gespräch der flüchtige Zustand `tippt` oder
//! `denkt_nach` bis zu seinem Ablauf — nur im Speicher, nie im Tageslog, im Verlauf oder im Schreibkonto. Einziger
//! Halter der Tabelle; jede Änderung geht **unter der eigenen Sperre** an eine eingesetzte Veröffentlichung, damit die
//! Reihenfolge der Signale je Kennung und Gespräch die der Änderungen ist. Den Bus kennt der Baustein nicht, die
//! Anwesenheit ruft er nie (Sperrfolge Anwesenheit → Aktivität).

mod bremse;

pub use bremse::{Gebremst, Grenze};

use crate::gespraeche::{Gespraech, Kennung};
use crate::uhr::{Uhr, Zeit};
use bremse::Bremse;
use chrono::TimeDelta;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

/// Der Zustand mit festem Wortlaut (Regel 2); auf der Leitung `"tippt"` bzw. `"denkt_nach"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zustand {
    Tippt,
    DenktNach,
}

impl Zustand {
    /// Nur der genaue Wortlaut — `"denkt nach"`, `"Tippt"` und jeder andere Text sind kein Zustand.
    pub fn lesen(text: &str) -> Option<Zustand> {
        match text {
            "tippt" => Some(Zustand::Tippt),
            "denkt_nach" => Some(Zustand::DenktNach),
            _ => None,
        }
    }

    pub fn als_text(self) -> &'static str {
        match self {
            Zustand::Tippt => "tippt",
            Zustand::DenktNach => "denkt_nach",
        }
    }

    /// Standard-Gültigkeit je Zustand (Regel 3, F4): `tippt` 6 s, `denkt_nach` 30 s.
    fn standard_s(self) -> u32 {
        match self {
            Zustand::Tippt => 6,
            Zustand::DenktNach => 30,
        }
    }
}

/// Angebbare Gültigkeit in ganzen Sekunden: 1 bis 120 (Regel 3).
pub const GUELTIG_HOECHSTENS_S: u32 = 120;
/// Höchstens so viele gleichzeitige Zustände je Kennung, je Gespräch einer (Regel 2).
pub const JE_KENNUNG_HOECHSTENS: usize = 16;
/// Ab so vielen Einträgen räumt ein Setzen die ganze Tabelle auf, nicht nur die Einträge der Kennung (Regel 3).
const DURCHGANG_AB: usize = 1_024;

/// Ein gelungenes Setzen: der Zustand gilt bis `bis`, gerechnet ab `jetzt` (Eingang, Serverzeit).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gesetzt {
    pub zustand: Zustand,
    pub bis: Zeit,
    pub jetzt: Zeit,
}

/// Warum ein Setzen nichts bewirkt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Abweisung {
    /// `gueltig_s` 0 oder über 120 (422 `gueltigkeit`).
    Gueltigkeit,
    /// Ein 17. Gespräch derselben Kennung (422 `aktivitaeten`).
    ZuViele,
    /// Zu viele Meldungen (429, Regel 9); nur nach allen anderen Prüfungen.
    Gebremst(Gebremst),
}

/// Ein gültiger Eintrag, wie das Lesen ihn nennt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Aktiv {
    pub kennung: Kennung,
    pub zustand: Zustand,
    pub bis: Zeit,
}

/// Die gültigen Zustände eines Gesprächs zum Zeitpunkt `jetzt`, nach Kennung sortiert.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gelesen {
    pub jetzt: Zeit,
    pub aktivitaeten: Vec<Aktiv>,
}

/// Was die Veröffentlichung bekommt: `aktiv` gesetzt oder erneuert, `None` beendet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Aenderung {
    pub gespraech: Gespraech,
    pub kennung: Kennung,
    pub aktiv: Option<Gesetzt>,
}

/// Wohin jede Änderung geht — gerufen unter der Sperre der Aktivität (synchron, z. B. `broadcast::send`).
pub type Veroeffentlichen = Arc<dyn Fn(&Aenderung) + Send + Sync>;

#[derive(Debug, Clone, Copy)]
struct Eintrag {
    zustand: Zustand,
    bis: Zeit,
    gueltig: TimeDelta,
}

impl Eintrag {
    /// Gilt, solange `jetzt < bis`; liegt `bis` weiter vorn als die Gültigkeit, ging die Uhr zurück — dann lieber
    /// weg als zu lang (Regel 3).
    fn gilt(&self, jetzt: Zeit) -> bool {
        jetzt < self.bis && self.bis - jetzt <= self.gueltig
    }
}

type Tabelle = HashMap<(Kennung, Gespraech), Eintrag>;

/// Tabelle und Bremse unter einer Sperre (Regel 1).
#[derive(Default)]
struct Inneres {
    tabelle: Tabelle,
    bremse: Bremse,
}

/// Einziger Halter der Aktivitäten und ihrer Bremse (A00019).
pub struct Aktivitaet {
    uhr: Arc<dyn Uhr>,
    inneres: Mutex<Inneres>,
    veroeffentlichen: Veroeffentlichen,
}

impl Aktivitaet {
    pub fn neu(uhr: Arc<dyn Uhr>) -> Aktivitaet {
        Aktivitaet {
            uhr,
            inneres: Mutex::default(),
            veroeffentlichen: Arc::new(|_: &Aenderung| {}),
        }
    }

    pub fn mit_veroeffentlichen(self, veroeffentlichen: Veroeffentlichen) -> Aktivitaet {
        Aktivitaet {
            veroeffentlichen,
            ..self
        }
    }

    /// Setzt oder erneuert den Zustand der Kennung im Gespräch: ersetzt den bisherigen, gilt ab Eingang
    /// (`bis = jetzt + gueltig`, nie addiert); ohne Angabe der Standard des Zustands. Abgewiesen ohne Wirkung; die
    /// Bremse bucht erst, wenn sonst nichts dagegen spricht (Regel 9).
    pub fn setzen(
        &self,
        kennung: &Kennung,
        herkunft: IpAddr,
        gespraech: &Gespraech,
        zustand: Zustand,
        gueltig_s: Option<u32>,
    ) -> Result<Gesetzt, Abweisung> {
        let gueltig = gueltigkeit(zustand, gueltig_s)?;
        let mut inneres = self.sperre();
        let jetzt = self.uhr.jetzt();
        let Inneres { tabelle, bremse } = &mut *inneres;
        aufraeumen(tabelle, kennung, jetzt);
        let schluessel = (kennung.clone(), gespraech.clone());
        let neues_gespraech = !tabelle.contains_key(&schluessel);
        let schon = tabelle.keys().filter(|(k, _)| k == kennung).count();
        if neues_gespraech && schon >= JE_KENNUNG_HOECHSTENS {
            return Err(Abweisung::ZuViele);
        }
        bremse
            .buchen(kennung, gespraech, herkunft, jetzt)
            .map_err(Abweisung::Gebremst)?;
        let bis = jetzt + gueltig;
        tabelle.insert(
            schluessel,
            Eintrag {
                zustand,
                bis,
                gueltig,
            },
        );
        let gesetzt = Gesetzt {
            zustand,
            bis,
            jetzt,
        };
        self.melden(
            &inneres,
            &Aenderung {
                gespraech: gespraech.clone(),
                kennung: kennung.clone(),
                aktiv: Some(gesetzt),
            },
        );
        Ok(gesetzt)
    }

    /// Beendet den Zustand der Kennung im Gespräch; ein Ende-Signal nur, wenn er noch galt.
    pub fn beenden(&self, kennung: &Kennung, gespraech: &Gespraech) {
        let mut inneres = self.sperre();
        let jetzt = self.uhr.jetzt();
        let entfernt = inneres
            .tabelle
            .remove(&(kennung.clone(), gespraech.clone()));
        if entfernt.is_some_and(|eintrag| eintrag.gilt(jetzt)) {
            self.melden(&inneres, &beendet(kennung, gespraech));
        }
    }

    /// Beendet alle Zustände der Kennung (Verlassen, offline eines Menschen); Ende-Signale nach Gespräch geordnet.
    pub fn beenden_alle(&self, kennung: &Kennung) {
        let mut inneres = self.sperre();
        let jetzt = self.uhr.jetzt();
        let mut galten: Vec<Gespraech> = Vec::new();
        inneres.tabelle.retain(|(k, gespraech), eintrag| {
            let von_ihr = k == kennung;
            if von_ihr && eintrag.gilt(jetzt) {
                galten.push(gespraech.clone());
            }
            !von_ihr
        });
        galten.sort();
        for gespraech in &galten {
            self.melden(&inneres, &beendet(kennung, gespraech));
        }
    }

    /// Die gültigen Zustände des Gesprächs ohne den des Lesenden, nach Kennung sortiert.
    pub fn lesen(&self, gespraech: &Gespraech, ohne: &Kennung) -> Gelesen {
        let inneres = self.sperre();
        let jetzt = self.uhr.jetzt();
        let mut aktivitaeten: Vec<Aktiv> = inneres
            .tabelle
            .iter()
            .filter(|((kennung, g), eintrag)| {
                g == gespraech && kennung != ohne && eintrag.gilt(jetzt)
            })
            .map(|((kennung, _), eintrag)| Aktiv {
                kennung: kennung.clone(),
                zustand: eintrag.zustand,
                bis: eintrag.bis,
            })
            .collect();
        aktivitaeten
            .sort_by(|links, rechts| links.kennung.als_text().cmp(rechts.kennung.als_text()));
        Gelesen {
            jetzt,
            aktivitaeten,
        }
    }

    fn sperre(&self) -> MutexGuard<'_, Inneres> {
        self.inneres.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Unter der Sperre: veröffentlichen, solange niemand eine weitere Änderung machen kann.
    fn melden(&self, _sperre: &Inneres, aenderung: &Aenderung) {
        (self.veroeffentlichen)(aenderung);
    }
}

/// Die Gültigkeit als Dauer: ohne Angabe der Standard, sonst 1 bis 120 s.
fn gueltigkeit(zustand: Zustand, gueltig_s: Option<u32>) -> Result<TimeDelta, Abweisung> {
    let sekunden = gueltig_s.unwrap_or(zustand.standard_s());
    if !(1..=GUELTIG_HOECHSTENS_S).contains(&sekunden) {
        return Err(Abweisung::Gueltigkeit);
    }
    Ok(TimeDelta::seconds(i64::from(sekunden)))
}

/// Entfernt die abgelaufenen Einträge der Kennung; ab 1.024 Einträgen alle abgelaufenen (Regel 3).
fn aufraeumen(tabelle: &mut Tabelle, kennung: &Kennung, jetzt: Zeit) {
    let ganz = tabelle.len() > DURCHGANG_AB;
    tabelle.retain(|(k, _), eintrag| (!ganz && k != kennung) || eintrag.gilt(jetzt));
}

/// Sichtbarkeitsregel (ADR-0051 Regel 10): ein Channel für jeden mit Sitzung, ein Direktgespräch
/// (`<kennung-a>+<kennung-b>`, ADR-0003) nur für die Beteiligten, ein archiviertes Gespräch für niemanden. Setzen,
/// Löschen und Lesen gehen nur für Sichtbares; sonst antwortet die Route wie für ein unbekanntes.
pub fn sichtbar(gespraech: &Gespraech, fuer: &Kennung, archiviert: bool) -> bool {
    if archiviert {
        return false;
    }
    match gespraech {
        Gespraech::Channel(_) => true,
        Gespraech::Direkt(beteiligte) => beteiligte
            .split('+')
            .any(|beteiligt| beteiligt == fuer.als_text()),
    }
}

fn beendet(kennung: &Kennung, gespraech: &Gespraech) -> Aenderung {
    Aenderung {
        gespraech: gespraech.clone(),
        kennung: kennung.clone(),
        aktiv: None,
    }
}

#[cfg(test)]
mod tests;
