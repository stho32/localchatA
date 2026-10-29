//! Anhangeingang (A00017, ADR-0046 Regeln 1–3, 10 und Ergänzung „Karte #9 Gate“): angemeldete, laufende und
//! fertig hochgeladene, noch nicht gesendete Dateien. Rein — die Einträge liegen samt `reserviert` unter der
//! Schreibsperre von Gespräche (eine Sperre für alle Übergänge, Regel 5); Dateien räumt der Aufrufer.

use super::Kennung;
use super::anhangid::Anhangid;
use super::anhangpruefung::Art;
use super::anhangquote::zaehlmass;
use crate::uhr::Zeit;
use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::watch;

/// Offene Hochladungen je Kennung (angemeldet, laufend, fertig).
pub const OFFEN_JE_KENNUNG: usize = 4;
/// Offene Hochladungen je Herkunft (angemeldet, laufend, fertig).
pub const OFFEN_JE_HERKUNFT: usize = 16;
/// Laufende Hochladungen gesamt (Ergänzung „Karte #9 Gate“ Regel 3: nur `laeuft`).
pub const LAUFEND_GESAMT: usize = 64;
/// Mindestrate einer Übertragung in Bytes je Sekunde (1 MiB/s, Regel 2.4).
pub const MINDESTRATE: u64 = 1_048_576;
/// Anlauffrist einer Anmeldung ohne laufendes Hochladen.
pub const ANLAUF_MS: u64 = 60_000;
/// So lange gilt eine fertig hochgeladene Datei ab dem Abschluss.
pub const FERTIG_MS: u64 = 15 * 60_000;
/// So lange antwortet eine verfallene ID noch `abgelaufen` statt `unbekannt`.
const GRABSTEIN_MS: u64 = 60 * 60_000;

/// `lebensende = jetzt + 60 s + 2 × bytes / 1 MiB/s`, in ms aufgerundet (Ergänzung „Karte #9 Gate“ Regel 1).
pub fn lebensende(jetzt: Zeit, bytes: u64) -> Zeit {
    let uebertragung_ms = (u128::from(bytes) * 2 * 1_000).div_ceil(u128::from(MINDESTRATE));
    plus_ms(jetzt, u128::from(ANLAUF_MS) + uebertragung_ms)
}

/// `zeit + ms`, gekappt bei hundert Jahren — keine Frist dieser Instanz ist länger.
pub fn plus_ms(zeit: Zeit, ms: u128) -> Zeit {
    const HUNDERT_JAHRE_MS: u128 = 100 * 366 * 24 * 3_600 * 1_000;
    let ms = i64::try_from(ms.min(HUNDERT_JAHRE_MS)).unwrap_or(i64::MAX);
    zeit.checked_add_signed(chrono::TimeDelta::milliseconds(ms))
        .unwrap_or(zeit)
}

/// Warum ein `PUT` oder ein Platz endet, ohne dass er selbst abbricht.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Abbruchgrund {
    /// `DELETE` der eigenen ID — der `PUT` räumt seine Datei und antwortet 404.
    Verworfen,
    /// Lebensende erreicht — der `PUT` räumt seine Datei und antwortet 408.
    Lebensende,
}

/// Welche Grenze der offenen Hochladungen gegriffen hat (429 mit `grenze` und `hoechstens`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Offen {
    Kennung,
    Herkunft,
    Gesamt,
}

impl Offen {
    pub fn grenze(self) -> &'static str {
        match self {
            Offen::Kennung => "offen_kennung",
            Offen::Herkunft => "offen_herkunft",
            Offen::Gesamt => "offen_gesamt",
        }
    }

    pub fn hoechstens(self) -> usize {
        match self {
            Offen::Kennung => OFFEN_JE_KENNUNG,
            Offen::Herkunft => OFFEN_JE_HERKUNFT,
            Offen::Gesamt => LAUFEND_GESAMT,
        }
    }
}

/// Eine angenommene Anmeldung: Anlauffrist und Lebensende (`hochladen_bis`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Anmeldung {
    pub id: Anhangid,
    pub anlauf_bis: Zeit,
    pub hochladen_bis: Zeit,
}

/// Was der Kopf eines `PUT` sagt — geprüft in der Reihenfolge der Regel 2.1/2.2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Kopf {
    pub typ_passt: bool,
    pub laenge: Option<u64>,
}

/// Warum ein `PUT` nicht beginnt; vor dem ersten gelesenen Byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Beginnfehler {
    Unbekannt,
    Abgelaufen,
    Laeuft,
    Fertig,
    OffenGesamt,
    Typ,
    OhneLaenge,
    Laenge { erwartet: u64 },
}

/// Ein laufender `PUT`: woran er erkennt, dass er abbrechen muss, und wie weit er geschrieben hat.
#[derive(Debug)]
pub struct Platz {
    pub id: Anhangid,
    pub generation: u64,
    pub bytes: u64,
    pub beginn: Zeit,
    pub lebensende: Zeit,
    pub abbruch: watch::Receiver<Option<Abbruchgrund>>,
    pub geschrieben: Arc<AtomicU64>,
}

/// Der Eintrag gehört nicht mehr diesem `PUT` (verworfen, verfallen, genommen).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verloren {
    Unbekannt,
    Abgelaufen,
}

/// Ein verworfener Eintrag: lag schon eine Datei im Eingang (`laufend` heißt: eine Teildatei)?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Verworfen {
    pub laufend: bool,
    pub fertig: bool,
    pub bytes: u64,
}

/// Warum eine ID beim Senden nicht genommen wird (422 `anhang` mit `grund`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nehmgrund {
    Unbekannt,
    Abgelaufen,
    Fremd,
    Doppelt,
    Unvollstaendig,
}

impl Nehmgrund {
    pub fn als_text(self) -> &'static str {
        match self {
            Nehmgrund::Unbekannt => "unbekannt",
            Nehmgrund::Abgelaufen => "abgelaufen",
            Nehmgrund::Fremd => "fremd",
            Nehmgrund::Doppelt => "doppelt",
            Nehmgrund::Unvollstaendig => "unvollstaendig",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nehmfehler {
    pub id: Anhangid,
    pub grund: Nehmgrund,
}

/// Ein beim Senden genommener, fertig hochgeladener Eintrag — bis zum Binden oder Zurücklegen.
#[derive(Debug)]
pub struct Genommen {
    pub id: Anhangid,
    eintrag: Eintrag,
}

impl Genommen {
    pub fn bytes(&self) -> u64 {
        self.eintrag.bytes
    }

    pub fn name(&self) -> &str {
        &self.eintrag.name
    }

    pub fn ascii(&self) -> &str {
        &self.eintrag.ascii
    }

    pub fn art(&self) -> Art {
        match &self.eintrag.zustand {
            Zustand::Fertig { art, .. } => *art,
            Zustand::Angemeldet { .. } | Zustand::Laeuft { .. } => Art::datei(),
        }
    }
}

/// Ein verfallener Eintrag; `ohne_hochladen`, wenn er nie fertig wurde (Episode „Reservierung verfallen ohne
/// Hochladen“). Liegt eine Datei im Eingang (`fertig`) oder eine Teildatei (`laufend`), räumt der Aufrufer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verfallen {
    pub id: Anhangid,
    pub kennung: Kennung,
    pub herkunft: IpAddr,
    pub bytes: u64,
    pub ohne_hochladen: bool,
    pub laufend: bool,
    pub fertig: bool,
}

#[derive(Debug)]
struct Eintrag {
    kennung: Kennung,
    herkunft: IpAddr,
    name: String,
    ascii: String,
    bytes: u64,
    /// Gesetzt nur in `anmelden`; kein anderer Übergang ändert es.
    lebensende: Zeit,
    zustand: Zustand,
}

#[derive(Debug)]
enum Zustand {
    Angemeldet {
        anlauf_bis: Zeit,
    },
    Laeuft {
        generation: u64,
        signal: watch::Sender<Option<Abbruchgrund>>,
        geschrieben: Arc<AtomicU64>,
    },
    Fertig {
        gueltig_bis: Zeit,
        art: Art,
    },
}

/// Alle Einträge des Eingangs.
#[derive(Debug, Default)]
pub struct Eingang {
    eintraege: HashMap<Anhangid, Eintrag>,
    /// Verfallene IDs: Kennung und bis wann sie noch `abgelaufen` antworten.
    grabsteine: HashMap<Anhangid, (Kennung, Zeit)>,
    generation: u64,
}

impl Eingang {
    /// Je Kennung 4 und je Herkunft 16 über alle drei Zustände, gesamt 64 nur über `laeuft`.
    pub fn offen_pruefen(&self, kennung: &Kennung, herkunft: IpAddr) -> Result<(), Offen> {
        let je_kennung = self
            .eintraege
            .values()
            .filter(|eintrag| &eintrag.kennung == kennung)
            .count();
        if je_kennung >= OFFEN_JE_KENNUNG {
            return Err(Offen::Kennung);
        }
        let je_herkunft = self
            .eintraege
            .values()
            .filter(|eintrag| eintrag.herkunft == herkunft)
            .count();
        if je_herkunft >= OFFEN_JE_HERKUNFT {
            return Err(Offen::Herkunft);
        }
        if self.laufend() >= LAUFEND_GESAMT {
            return Err(Offen::Gesamt);
        }
        Ok(())
    }

    /// Nimmt eine geprüfte Anmeldung auf (Quote und Platte prüft Gespräche davor): Zustand `angemeldet`,
    /// Lebensende ab jetzt, Anlauffrist `min(jetzt + 60 s, lebensende)`.
    #[allow(clippy::too_many_arguments)]
    pub fn anmelden(
        &mut self,
        id: Anhangid,
        kennung: &Kennung,
        herkunft: IpAddr,
        name: &str,
        ascii: &str,
        bytes: u64,
        jetzt: Zeit,
    ) -> Result<Anmeldung, Offen> {
        self.offen_pruefen(kennung, herkunft)?;
        let lebensende = lebensende(jetzt, bytes);
        let anlauf_bis = anlauf(jetzt, lebensende);
        self.eintraege.insert(
            id.clone(),
            Eintrag {
                kennung: kennung.clone(),
                herkunft,
                name: name.to_string(),
                ascii: ascii.to_string(),
                bytes,
                lebensende,
                zustand: Zustand::Angemeldet { anlauf_bis },
            },
        );
        Ok(Anmeldung {
            id,
            anlauf_bis,
            hochladen_bis: lebensende,
        })
    }

    /// `PUT` beginnt: 404 · 409 · 429 `offen_gesamt` · 415 · 411 · 422 `laenge`, sonst `laeuft`.
    pub fn beginnen(
        &mut self,
        kennung: &Kennung,
        id: &Anhangid,
        kopf: Kopf,
        jetzt: Zeit,
    ) -> Result<Platz, Beginnfehler> {
        let laufend = self.laufend();
        let Some(eintrag) = self
            .eintraege
            .get_mut(id)
            .filter(|eintrag| &eintrag.kennung == kennung)
        else {
            return Err(self
                .verfallen_oder_unbekannt(kennung, id)
                .map_or(Beginnfehler::Unbekannt, |()| Beginnfehler::Abgelaufen));
        };
        let anlauf_bis = match &eintrag.zustand {
            Zustand::Laeuft { .. } => return Err(Beginnfehler::Laeuft),
            Zustand::Fertig { .. } => return Err(Beginnfehler::Fertig),
            Zustand::Angemeldet { anlauf_bis } => *anlauf_bis,
        };
        if jetzt > anlauf_bis || jetzt > eintrag.lebensende {
            return Err(Beginnfehler::Abgelaufen);
        }
        if laufend >= LAUFEND_GESAMT {
            return Err(Beginnfehler::OffenGesamt);
        }
        if !kopf.typ_passt {
            return Err(Beginnfehler::Typ);
        }
        let laenge = kopf.laenge.ok_or(Beginnfehler::OhneLaenge)?;
        if laenge != eintrag.bytes {
            return Err(Beginnfehler::Laenge {
                erwartet: eintrag.bytes,
            });
        }
        self.generation += 1;
        let (signal, abbruch) = watch::channel(None);
        let geschrieben = Arc::new(AtomicU64::new(0));
        eintrag.zustand = Zustand::Laeuft {
            generation: self.generation,
            signal,
            geschrieben: Arc::clone(&geschrieben),
        };
        Ok(Platz {
            id: id.clone(),
            generation: self.generation,
            bytes: eintrag.bytes,
            beginn: jetzt,
            lebensende: eintrag.lebensende,
            abbruch,
            geschrieben,
        })
    }

    /// Der `PUT` hat alle Bytes und die Datei liegt fertig: `laeuft` → `fertig`, gültig 15 min ab jetzt. Gehört der
    /// Eintrag nicht mehr diesem `PUT`, verliert er und räumt seine Datei selbst.
    pub fn abschliessen(
        &mut self,
        id: &Anhangid,
        generation: u64,
        art: Art,
        jetzt: Zeit,
    ) -> Result<Zeit, Verloren> {
        let eintrag = self.laufender(id, generation)?;
        let gueltig_bis = plus_ms(jetzt, u128::from(FERTIG_MS));
        eintrag.zustand = Zustand::Fertig { gueltig_bis, art };
        Ok(gueltig_bis)
    }

    /// Ein `PUT` bricht ab (Frist, Lesefehler, Client weg, Schreibfehler): `laeuft` → `angemeldet` mit
    /// Anlauffrist `min(jetzt + 60 s, lebensende)`; das Lebensende bleibt. `false`, wenn er schon verloren hatte.
    pub fn abbrechen(&mut self, id: &Anhangid, generation: u64, jetzt: Zeit) -> bool {
        let Ok(eintrag) = self.laufender(id, generation) else {
            return false;
        };
        eintrag.zustand = Zustand::Angemeldet {
            anlauf_bis: anlauf(jetzt, eintrag.lebensende),
        };
        true
    }

    /// `DELETE` der eigenen ID: Eintrag weg, Reservierung frei, ein laufender `PUT` endet als Abbruch. Fremde,
    /// unbekannte und schon genommene IDs: nichts.
    pub fn verwerfen(&mut self, kennung: &Kennung, id: &Anhangid) -> Option<Verworfen> {
        if self
            .eintraege
            .get(id)
            .is_none_or(|eintrag| &eintrag.kennung != kennung)
        {
            return None;
        }
        let eintrag = self.eintraege.remove(id)?;
        let (laufend, fertig) = match &eintrag.zustand {
            Zustand::Laeuft { signal, .. } => {
                signal.send_replace(Some(Abbruchgrund::Verworfen));
                (true, false)
            }
            Zustand::Fertig { .. } => (false, true),
            Zustand::Angemeldet { .. } => (false, false),
        };
        Some(Verworfen {
            laufend,
            fertig,
            bytes: eintrag.bytes,
        })
    }

    /// Beim Senden: alle IDs oder keine — doppelt, unbekannt, abgelaufen, fremd, unvollständig → abgewiesen.
    pub fn nehmen(
        &mut self,
        kennung: &Kennung,
        ids: &[Anhangid],
        jetzt: Zeit,
    ) -> Result<Vec<Genommen>, Nehmfehler> {
        let mut gesehen = HashSet::new();
        for id in ids {
            if !gesehen.insert(id) {
                return Err(Nehmfehler {
                    id: id.clone(),
                    grund: Nehmgrund::Doppelt,
                });
            }
        }
        for id in ids {
            self.nehmbar(kennung, id, jetzt)
                .map_err(|grund| Nehmfehler {
                    id: id.clone(),
                    grund,
                })?;
        }
        Ok(ids
            .iter()
            .filter_map(|id| {
                self.eintraege.remove(id).map(|eintrag| Genommen {
                    id: id.clone(),
                    eintrag,
                })
            })
            .collect())
    }

    /// Nach gescheitertem Binden: die Einträge unverändert zurück — Lebensende und Gültigkeit bleiben.
    pub fn zuruecklegen(&mut self, genommen: Vec<Genommen>) {
        for Genommen { id, eintrag } in genommen {
            self.eintraege.insert(id, eintrag);
        }
    }

    /// Sekundentakt: `angemeldet` nach der Anlauffrist, `laeuft` am Lebensende (der `PUT` erhält das Signal),
    /// `fertig` nach 15 min. Reservierung sofort frei; die ID antwortet noch eine Stunde `abgelaufen`.
    pub fn ablaufen(&mut self, jetzt: Zeit) -> Vec<Verfallen> {
        self.grabsteine.retain(|_, (_, bis)| *bis >= jetzt);
        let faellig: Vec<Anhangid> = self
            .eintraege
            .iter()
            .filter(|(_, eintrag)| eintrag.verfallen_um(jetzt))
            .map(|(id, _)| id.clone())
            .collect();
        let mut verfallen = Vec::new();
        for id in faellig {
            let Some(eintrag) = self.eintraege.remove(&id) else {
                continue;
            };
            let (laufend, fertig) = match &eintrag.zustand {
                Zustand::Laeuft { signal, .. } => {
                    signal.send_replace(Some(Abbruchgrund::Lebensende));
                    (true, false)
                }
                Zustand::Fertig { .. } => (false, true),
                Zustand::Angemeldet { .. } => (false, false),
            };
            self.grabsteine.insert(
                id.clone(),
                (
                    eintrag.kennung.clone(),
                    plus_ms(jetzt, u128::from(GRABSTEIN_MS)),
                ),
            );
            verfallen.push(Verfallen {
                id,
                kennung: eintrag.kennung,
                herkunft: eintrag.herkunft,
                bytes: eintrag.bytes,
                ohne_hochladen: !fertig,
                laufend,
                fertig,
            });
        }
        verfallen
    }

    /// Beim Start: der Eingang ist leer (die Dateien leert die Logablage).
    pub fn leeren(&mut self) {
        self.eintraege.clear();
        self.grabsteine.clear();
    }

    /// `reserviert` A = Σ `z(bytes)` aller Einträge.
    pub fn reserviert(&self) -> u64 {
        self.eintraege
            .values()
            .map(|eintrag| zaehlmass(eintrag.bytes))
            .sum()
    }

    /// Noch zu schreibende Bytes im Zählmaß: Σ (`z(bytes)` − geschrieben) — für die Prüfung der Platte.
    pub fn offen_zu_schreiben(&self) -> u64 {
        self.eintraege
            .values()
            .map(|eintrag| {
                let geschrieben = match &eintrag.zustand {
                    Zustand::Angemeldet { .. } => 0,
                    Zustand::Laeuft { geschrieben, .. } => geschrieben.load(Ordering::Relaxed),
                    Zustand::Fertig { .. } => eintrag.bytes,
                };
                zaehlmass(eintrag.bytes).saturating_sub(geschrieben)
            })
            .sum()
    }

    /// Offene Hochladungen (alle drei Zustände).
    pub fn anzahl(&self) -> usize {
        self.eintraege.len()
    }

    pub fn laufend(&self) -> usize {
        self.eintraege
            .values()
            .filter(|eintrag| matches!(eintrag.zustand, Zustand::Laeuft { .. }))
            .count()
    }

    /// Ob die ID im Eingang liegt (eigene wie fremde) — für die Route Löschen ist sie `unbekannt`.
    pub fn enthaelt(&self, id: &Anhangid) -> bool {
        self.eintraege.contains_key(id)
    }

    fn laufender(&mut self, id: &Anhangid, generation: u64) -> Result<&mut Eintrag, Verloren> {
        let grabstein = self.grabsteine.contains_key(id);
        match self.eintraege.get_mut(id) {
            Some(eintrag) if matches!(eintrag.zustand, Zustand::Laeuft { generation: g, .. } if g == generation) => {
                Ok(eintrag)
            }
            _ if grabstein => Err(Verloren::Abgelaufen),
            _ => Err(Verloren::Unbekannt),
        }
    }

    /// `Some(())`, wenn die ID dieser Kennung verfallen ist (Grabstein); sonst unbekannt — auch für fremde.
    fn verfallen_oder_unbekannt(&self, kennung: &Kennung, id: &Anhangid) -> Option<()> {
        self.grabsteine
            .get(id)
            .filter(|(besitzer, _)| besitzer == kennung)
            .map(|_| ())
    }

    fn nehmbar(&self, kennung: &Kennung, id: &Anhangid, jetzt: Zeit) -> Result<(), Nehmgrund> {
        let Some(eintrag) = self.eintraege.get(id) else {
            return Err(self
                .verfallen_oder_unbekannt(kennung, id)
                .map_or(Nehmgrund::Unbekannt, |()| Nehmgrund::Abgelaufen));
        };
        if &eintrag.kennung != kennung {
            return Err(Nehmgrund::Fremd);
        }
        if eintrag.verfallen_um(jetzt) {
            return Err(Nehmgrund::Abgelaufen);
        }
        match eintrag.zustand {
            Zustand::Fertig { .. } => Ok(()),
            Zustand::Angemeldet { .. } | Zustand::Laeuft { .. } => Err(Nehmgrund::Unvollstaendig),
        }
    }
}

impl Eintrag {
    fn verfallen_um(&self, jetzt: Zeit) -> bool {
        match &self.zustand {
            Zustand::Angemeldet { anlauf_bis } => jetzt > *anlauf_bis,
            Zustand::Laeuft { .. } => jetzt > self.lebensende,
            Zustand::Fertig { gueltig_bis, .. } => jetzt > *gueltig_bis,
        }
    }
}

/// Anlauffrist: `min(jetzt + 60 s, lebensende)`.
fn anlauf(jetzt: Zeit, lebensende: Zeit) -> Zeit {
    plus_ms(jetzt, u128::from(ANLAUF_MS)).min(lebensende)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::DateTime;

    const T0: &str = "2026-10-29T12:00:00.000+01:00";

    fn t(sekunden_ms: i64) -> Zeit {
        DateTime::parse_from_rfc3339(T0).expect("Zeit")
            + chrono::TimeDelta::milliseconds(sekunden_ms)
    }

    fn kennung(text: &str) -> Kennung {
        Kennung::bilden(text).expect("Kennung")
    }

    fn ip(text: &str) -> IpAddr {
        text.parse().expect("IP")
    }

    fn id(nummer: u128) -> Anhangid {
        let token = crate::zufall::Zufall::token(&crate::zufall::FesteFolge::neu([nummer]))
            .expect("Zufall");
        Anhangid::aus_token(&token).expect("ID")
    }

    fn kopf(laenge: u64) -> Kopf {
        Kopf {
            typ_passt: true,
            laenge: Some(laenge),
        }
    }

    fn beta() -> Kennung {
        kennung("beta@127.0.0.1")
    }

    fn anmelden(
        eingang: &mut Eingang,
        nummer: u128,
        wer: &Kennung,
        bytes: u64,
        jetzt: Zeit,
    ) -> Anmeldung {
        let herkunft = wer.herkunft().expect("Herkunft");
        eingang
            .anmelden(
                id(nummer),
                wer,
                herkunft,
                "bericht.pdf",
                "bericht.pdf",
                bytes,
                jetzt,
            )
            .expect("angemeldet")
    }

    /// Lebensende 60 s + 2 × Länge / 1 MiB/s in ms aufgerundet: 307.200 → 60,586 s; 1 → 60,001 s; 9 MiB → 78 s.
    #[test]
    fn das_lebensende_folgt_der_laenge() {
        assert_eq!(lebensende(t(0), 307_200), t(60_586));
        assert_eq!(lebensende(t(0), 1), t(60_001));
        assert_eq!(lebensende(t(0), 9_437_184), t(78_000));
        assert_eq!(lebensende(t(0), 2 * 1_073_741_824), t(60_000 + 4_096_000));
        assert!(lebensende(t(0), u64::MAX) > t(0));
    }

    /// ADR-0046 Ergänzung „Karte #9 Gate“, Erste Tests (L1): Lebensende und Anlauffrist; Abbruch verlängert nie.
    #[test]
    fn das_lebensende_haelt_durch_abbrueche_und_endet_bei_78_s() {
        let mut eingang = Eingang::default();
        let anmeldung = anmelden(&mut eingang, 1, &beta(), 9_437_184, t(0));
        assert_eq!(anmeldung.hochladen_bis, t(78_000));
        assert_eq!(anmeldung.anlauf_bis, t(60_000));

        let platz = eingang
            .beginnen(&beta(), &anmeldung.id, kopf(9_437_184), t(50_000))
            .expect("läuft");
        assert!(eingang.abbrechen(&anmeldung.id, platz.generation, t(50_100)));
        let platz = eingang
            .beginnen(&beta(), &anmeldung.id, kopf(9_437_184), t(77_999))
            .expect("läuft");
        assert_eq!(platz.lebensende, t(78_000));
        assert!(eingang.abbrechen(&anmeldung.id, platz.generation, t(77_999)));
        assert_eq!(
            eingang.ablaufen(t(78_000)),
            Vec::new(),
            "am Lebensende noch nicht"
        );
        assert_eq!(eingang.reserviert(), 9_437_184);

        let verfallen = eingang.ablaufen(t(78_001));
        assert_eq!(verfallen.len(), 1);
        assert!(verfallen[0].ohne_hochladen);
        assert_eq!(eingang.reserviert(), 0);
        assert_eq!(
            eingang
                .beginnen(&beta(), &anmeldung.id, kopf(9_437_184), t(78_002))
                .map(|_| ()),
            Err(Beginnfehler::Abgelaufen)
        );
    }

    /// Nach einem Abbruch bei t0 + 50 s ist die Anlauffrist `min(t0 + 110 s, t0 + 78 s)` — bei t0 + 79 s ist die
    /// Anmeldung verfallen (Mutanten „Abbruch setzt Lebensende neu“, „Anlauffrist ohne `min`“).
    #[test]
    fn nach_dem_abbruch_gilt_die_anlauffrist_hoechstens_bis_zum_lebensende() {
        let mut eingang = Eingang::default();
        let anmeldung = anmelden(&mut eingang, 1, &beta(), 9_437_184, t(0));
        let platz = eingang
            .beginnen(&beta(), &anmeldung.id, kopf(9_437_184), t(50_000))
            .expect("läuft");
        assert!(eingang.abbrechen(&anmeldung.id, platz.generation, t(50_000)));

        assert_eq!(eingang.ablaufen(t(78_001)).len(), 1);
        assert_eq!(eingang.reserviert(), 0);
    }

    /// Ein laufender Platz am Lebensende erhält das Abbruchsignal; danach antwortet die ID `abgelaufen`.
    #[test]
    fn ein_laufender_platz_erhaelt_am_lebensende_das_abbruchsignal() {
        let mut eingang = Eingang::default();
        let anmeldung = anmelden(&mut eingang, 1, &beta(), 9_437_184, t(0));
        let platz = eingang
            .beginnen(&beta(), &anmeldung.id, kopf(9_437_184), t(1_000))
            .expect("läuft");
        assert_eq!(*platz.abbruch.borrow(), None);
        assert_eq!(eingang.ablaufen(t(78_000)), Vec::new());

        let verfallen = eingang.ablaufen(t(78_001));

        assert_eq!(verfallen.len(), 1);
        assert!(verfallen[0].laufend && verfallen[0].ohne_hochladen);
        assert_eq!(*platz.abbruch.borrow(), Some(Abbruchgrund::Lebensende));
        assert_eq!(
            eingang.abschliessen(&anmeldung.id, platz.generation, Art::datei(), t(78_002)),
            Err(Verloren::Abgelaufen)
        );
    }

    /// Fertig bei t0 + 70 s → gültig bis t0 + 70 s + 15 min; vom Lebensende frei.
    #[test]
    fn eine_fertige_datei_gilt_15_min_ab_abschluss() {
        let mut eingang = Eingang::default();
        let anmeldung = anmelden(&mut eingang, 1, &beta(), 9_437_184, t(0));
        let platz = eingang
            .beginnen(&beta(), &anmeldung.id, kopf(9_437_184), t(60_000))
            .expect("läuft");
        let gueltig_bis = eingang
            .abschliessen(&anmeldung.id, platz.generation, Art::datei(), t(70_000))
            .expect("fertig");
        assert_eq!(gueltig_bis, t(70_000 + 15 * 60_000));
        assert_eq!(eingang.ablaufen(t(79_000)), Vec::new());
        assert_eq!(eingang.ablaufen(t(70_000 + 15 * 60_000)), Vec::new());
        let verfallen = eingang.ablaufen(t(70_001 + 15 * 60_000));
        assert_eq!(verfallen.len(), 1);
        assert!(!verfallen[0].ohne_hochladen && verfallen[0].fertig);
        assert_eq!(eingang.reserviert(), 0);
    }

    /// Anmeldung ohne `PUT`, 1 Byte: Lebensende 60,001 s, Anlauffrist 60 s — bei + 60 s noch reserviert, bei
    /// + 60,001 s frei; Zählmaß 65.536.
    #[test]
    fn eine_anmeldung_ohne_put_verfaellt_nach_der_anlauffrist() {
        let mut eingang = Eingang::default();
        anmelden(&mut eingang, 1, &beta(), 1, t(0));
        assert_eq!(eingang.reserviert(), 65_536);
        assert_eq!(eingang.ablaufen(t(60_000)), Vec::new());
        assert_eq!(eingang.reserviert(), 65_536);
        let verfallen = eingang.ablaufen(t(60_001));
        assert_eq!(verfallen[0].kennung, beta());
        assert_eq!(verfallen[0].bytes, 1);
        assert_eq!(eingang.reserviert(), 0);
    }

    /// Probe M20 (L1, Testuhr) wörtlich: „Anmelden 9.437.184, 20 min alle 50 s PUT nach 1 Byte geschlossen →
    /// spätestens nach der absoluten Lebensdauer 404 `abgelaufen`, reserviert 0.“ Takt jede Sekunde.
    #[test]
    fn probe_m20_reservierung_halten_endet_am_lebensende() {
        let mut eingang = Eingang::default();
        let anmeldung = anmelden(&mut eingang, 1, &beta(), 9_437_184, t(0));
        let mut erster_abgelaufen = None;
        let mut reserviert_bei = Vec::new();
        for sekunde in 1..=20 * 60 {
            let jetzt = t(sekunde * 1_000);
            eingang.ablaufen(jetzt);
            reserviert_bei.push((sekunde, eingang.reserviert()));
            if sekunde % 50 == 0 {
                match eingang.beginnen(&beta(), &anmeldung.id, kopf(9_437_184), jetzt) {
                    Ok(platz) => {
                        platz.geschrieben.store(1, Ordering::Relaxed);
                        eingang.abbrechen(&anmeldung.id, platz.generation, jetzt);
                    }
                    Err(fehler) => {
                        assert_eq!(fehler, Beginnfehler::Abgelaufen);
                        erster_abgelaufen.get_or_insert(sekunde);
                    }
                }
            }
        }
        assert_eq!(erster_abgelaufen, Some(100));
        let frei_ab = reserviert_bei
            .iter()
            .find(|(_, reserviert)| *reserviert == 0)
            .map(|(sekunde, _)| *sekunde);
        assert_eq!(
            frei_ab,
            Some(79),
            "spätestens nach der absoluten Lebensdauer"
        );
        assert!(
            reserviert_bei
                .iter()
                .filter(|(sekunde, _)| *sekunde >= 79)
                .all(|(_, r)| *r == 0)
        );
    }

    /// Erste Tests der Ergänzung „Karte #9 Gate“: zwei Herkünfte mit je 16 `angemeldet` sperren niemanden; 64
    /// `laeuft` → das 65. `beginnen` 429 `offen_gesamt`, der Eintrag bleibt `angemeldet`; die 17. Anmeldung einer
    /// Herkunft → `offen_herkunft`; die 5. einer Kennung → `offen_kennung` (Mutante „`offen_gesamt` zählt
    /// `angemeldet`“).
    #[test]
    fn offene_hochladungen_je_kennung_herkunft_und_laufend_gesamt() {
        let mut eingang = Eingang::default();
        let mut nummer = 0;
        for herkunft in ["127.0.0.2", "127.0.0.3"] {
            for name in ["a", "b", "c", "d"] {
                let wer = kennung(&format!("{name}@{herkunft}"));
                for _ in 0..4 {
                    nummer += 1;
                    anmelden(&mut eingang, nummer, &wer, 1, t(0));
                }
            }
        }
        assert_eq!(eingang.anzahl(), 32);
        anmelden(&mut eingang, 999, &beta(), 1, t(0));
        let fuenfte = kennung("e@127.0.0.2");
        assert_eq!(
            eingang
                .anmelden(id(1_000), &fuenfte, ip("127.0.0.2"), "x", "x", 1, t(0))
                .map(|_| ()),
            Err(Offen::Herkunft)
        );
        for _ in 0..3 {
            nummer += 1;
            anmelden(
                &mut eingang,
                2_000 + nummer,
                &kennung("f@127.0.0.9"),
                1,
                t(0),
            );
        }
        anmelden(&mut eingang, 3_000, &kennung("f@127.0.0.9"), 1, t(0));
        assert_eq!(
            eingang
                .anmelden(
                    id(3_001),
                    &kennung("f@127.0.0.9"),
                    ip("127.0.0.9"),
                    "x",
                    "x",
                    1,
                    t(0)
                )
                .map(|_| ()),
            Err(Offen::Kennung)
        );

        let mut laufend = Eingang::default();
        let mut ids = Vec::new();
        for herkunft in 2..=6 {
            for name in ["a", "b", "c", "d"] {
                let wer = kennung(&format!("{name}@127.0.0.{herkunft}"));
                for _ in 0..4 {
                    nummer += 1;
                    let anmeldung = anmelden(&mut laufend, 10_000 + nummer, &wer, 1, t(0));
                    ids.push((wer.clone(), anmeldung.id));
                }
            }
        }
        for (wer, id) in &ids[..64] {
            laufend.beginnen(wer, id, kopf(1), t(1)).expect("läuft");
        }
        let (wer, fuenfundsechzigste) = &ids[64];
        assert_eq!(
            laufend
                .beginnen(wer, fuenfundsechzigste, kopf(1), t(1))
                .map(|_| ()),
            Err(Beginnfehler::OffenGesamt)
        );
        assert_eq!(laufend.laufend(), 64);
        assert_eq!(
            laufend.offen_pruefen(&kennung("z@127.0.0.1"), ip("127.0.0.1")),
            Err(Offen::Gesamt)
        );
        assert_eq!(laufend.anzahl(), 80, "der 65. bleibt angemeldet");
    }

    /// Reihenfolge der Abweisungen am `PUT`: unbekannt (auch fremd) → laeuft/fertig → 415 → 411 → 422 `laenge`.
    #[test]
    fn beginnen_weist_in_der_reihenfolge_der_regel_ab() {
        let mut eingang = Eingang::default();
        let anmeldung = anmelden(&mut eingang, 1, &beta(), 307_200, t(0));
        let gamma = kennung("gamma@127.0.0.1");
        let fehler = |eingang: &mut Eingang, wer: &Kennung, kopf: Kopf| {
            eingang.beginnen(wer, &anmeldung.id, kopf, t(1)).map(|_| ())
        };
        assert_eq!(
            fehler(&mut eingang, &gamma, kopf(307_200)),
            Err(Beginnfehler::Unbekannt)
        );
        assert_eq!(
            eingang
                .beginnen(&beta(), &id(77), kopf(1), t(1))
                .map(|_| ()),
            Err(Beginnfehler::Unbekannt)
        );
        let ohne_typ = Kopf {
            typ_passt: false,
            laenge: None,
        };
        assert_eq!(
            fehler(&mut eingang, &beta(), ohne_typ),
            Err(Beginnfehler::Typ)
        );
        let ohne_laenge = Kopf {
            typ_passt: true,
            laenge: None,
        };
        assert_eq!(
            fehler(&mut eingang, &beta(), ohne_laenge),
            Err(Beginnfehler::OhneLaenge)
        );
        assert_eq!(
            fehler(&mut eingang, &beta(), kopf(307_199)),
            Err(Beginnfehler::Laenge { erwartet: 307_200 })
        );
        let platz = eingang
            .beginnen(&beta(), &anmeldung.id, kopf(307_200), t(1))
            .expect("läuft");
        assert_eq!(
            fehler(&mut eingang, &beta(), ohne_typ),
            Err(Beginnfehler::Laeuft)
        );
        eingang
            .abschliessen(&anmeldung.id, platz.generation, Art::datei(), t(2))
            .expect("fertig");
        assert_eq!(
            fehler(&mut eingang, &beta(), kopf(307_200)),
            Err(Beginnfehler::Fertig)
        );
    }

    /// Zwei `PUT` auf dieselbe ID: genau einer läuft (M23); ein Abbruch des Verlierers ändert nichts.
    #[test]
    fn von_zwei_beginnen_laeuft_genau_eines() {
        let mut eingang = Eingang::default();
        let anmeldung = anmelden(&mut eingang, 1, &beta(), 10, t(0));
        let erster = eingang
            .beginnen(&beta(), &anmeldung.id, kopf(10), t(1))
            .expect("läuft");
        let zweiter = eingang.beginnen(&beta(), &anmeldung.id, kopf(10), t(1));
        assert_eq!(zweiter.map(|_| ()), Err(Beginnfehler::Laeuft));
        assert!(!eingang.abbrechen(&anmeldung.id, erster.generation + 1, t(2)));
        assert_eq!(eingang.laufend(), 1);
    }

    /// `DELETE` während `laeuft`: Signal an den `PUT`, Reservierung sofort 0, sein Abschluss verliert. Fremd: nichts.
    #[test]
    fn verwerfen_beendet_den_laufenden_put_und_gibt_frei() {
        let mut eingang = Eingang::default();
        let anmeldung = anmelden(&mut eingang, 1, &beta(), 307_200, t(0));
        let platz = eingang
            .beginnen(&beta(), &anmeldung.id, kopf(307_200), t(1))
            .expect("läuft");
        assert_eq!(
            eingang.verwerfen(&kennung("gamma@127.0.0.1"), &anmeldung.id),
            None
        );
        assert_eq!(eingang.reserviert(), 307_200);

        let verworfen = eingang
            .verwerfen(&beta(), &anmeldung.id)
            .expect("verworfen");

        assert!(verworfen.laufend);
        assert_eq!(eingang.reserviert(), 0);
        assert_eq!(*platz.abbruch.borrow(), Some(Abbruchgrund::Verworfen));
        assert_eq!(
            eingang.abschliessen(&anmeldung.id, platz.generation, Art::datei(), t(2)),
            Err(Verloren::Unbekannt)
        );
        assert_eq!(
            eingang.verwerfen(&beta(), &anmeldung.id),
            None,
            "zweimal: ohne Wirkung"
        );
    }

    /// Nehmen beim Senden: doppelt, unbekannt, fremd, unvollständig, abgelaufen — alle oder keine; zweimal
    /// nehmen → `unbekannt`; zurücklegen lässt Gültigkeit und Lebensende stehen (Mutante „Zurücklegen setzt
    /// Lebensende neu“).
    #[test]
    fn nehmen_prueft_alle_ids_und_zuruecklegen_aendert_keine_frist() {
        let mut eingang = Eingang::default();
        let gamma = kennung("gamma@127.0.0.1");
        let fertig = anmelden(&mut eingang, 1, &beta(), 10, t(0)).id;
        let halb = anmelden(&mut eingang, 2, &beta(), 10, t(0)).id;
        let fremd = anmelden(&mut eingang, 3, &gamma, 10, t(0)).id;
        let platz = eingang
            .beginnen(&beta(), &fertig, kopf(10), t(1))
            .expect("läuft");
        eingang
            .abschliessen(&fertig, platz.generation, Art::datei(), t(2))
            .expect("fertig");
        let grund = |ergebnis: Result<Vec<Genommen>, Nehmfehler>| {
            ergebnis.map(|g| g.len()).map_err(|f| f.grund)
        };

        assert_eq!(
            grund(eingang.nehmen(&beta(), &[fertig.clone(), fertig.clone()], t(3))),
            Err(Nehmgrund::Doppelt)
        );
        assert_eq!(
            grund(eingang.nehmen(&beta(), &[fertig.clone(), id(9)], t(3))),
            Err(Nehmgrund::Unbekannt)
        );
        assert_eq!(
            grund(eingang.nehmen(&beta(), &[fertig.clone(), fremd.clone()], t(3))),
            Err(Nehmgrund::Fremd)
        );
        assert_eq!(
            grund(eingang.nehmen(&beta(), &[fertig.clone(), halb.clone()], t(3))),
            Err(Nehmgrund::Unvollstaendig)
        );
        assert_eq!(eingang.anzahl(), 3, "nichts genommen");

        let genommen = eingang
            .nehmen(&beta(), std::slice::from_ref(&fertig), t(3))
            .expect("genommen");
        assert_eq!(genommen.len(), 1);
        assert_eq!(eingang.reserviert(), 2 * 65_536);
        assert_eq!(
            grund(eingang.nehmen(&beta(), std::slice::from_ref(&fertig), t(3))),
            Err(Nehmgrund::Unbekannt)
        );
        eingang.zuruecklegen(genommen);
        assert_eq!(eingang.reserviert(), 3 * 65_536);
        let gueltig_bis = t(2 + 15 * 60_000);
        assert!(eingang.ablaufen(gueltig_bis).iter().all(|v| v.id != fertig));
        assert!(
            eingang
                .ablaufen(t(3 + 15 * 60_000))
                .iter()
                .any(|v| v.id == fertig)
        );
        assert_eq!(
            grund(eingang.nehmen(&beta(), std::slice::from_ref(&fertig), t(4 + 15 * 60_000))),
            Err(Nehmgrund::Abgelaufen)
        );
        assert_eq!(
            grund(eingang.nehmen(&gamma, std::slice::from_ref(&fertig), t(4 + 15 * 60_000))),
            Err(Nehmgrund::Unbekannt),
            "verfallene fremde ID: kein Orakel"
        );
    }

    /// Noch zu schreibende Bytes im Zählmaß: angemeldet ganz, laufend abzüglich geschrieben, fertig nichts.
    #[test]
    fn offen_zu_schreiben_zaehlt_was_noch_auf_die_platte_kommt() {
        let mut eingang = Eingang::default();
        let a = anmelden(&mut eingang, 1, &beta(), 100_000, t(0)).id;
        anmelden(&mut eingang, 2, &beta(), 1, t(0));
        assert_eq!(eingang.offen_zu_schreiben(), 102_400 + 65_536);
        let platz = eingang
            .beginnen(&beta(), &a, kopf(100_000), t(1))
            .expect("läuft");
        platz.geschrieben.store(40_000, Ordering::Relaxed);
        assert_eq!(eingang.offen_zu_schreiben(), 62_400 + 65_536);
        eingang
            .abschliessen(&a, platz.generation, Art::datei(), t(2))
            .expect("fertig");
        assert_eq!(eingang.offen_zu_schreiben(), 2_400 + 65_536);
    }

    #[test]
    fn das_zaehlmass_hat_ein_minimum_und_rundet_auf_4096() {
        use super::super::anhangquote::zaehlmass;
        assert_eq!(zaehlmass(1), 65_536);
        assert_eq!(zaehlmass(65_536), 65_536);
        assert_eq!(zaehlmass(65_537), 69_632);
        assert_eq!(zaehlmass(307_200), 307_200);
        assert_eq!(zaehlmass(10_485_761), 10_489_856);
        assert_eq!(zaehlmass(u64::MAX), u64::MAX);
    }
}
