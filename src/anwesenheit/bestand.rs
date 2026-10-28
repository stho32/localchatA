//! Namen belegen (B0048): der Bestand aller Sitzungen hinter einer Sperre — Prüfen und Belegen eines Namens
//! geschehen unter derselben Sperre, gleichzeitige Beitritte mit demselben Namen ergeben genau einen Erfolg.
//! Teilnehmerstand (B0076): unter derselben Sperre zählt `stand` jeden gemeldeten Statuswechsel (ADR-0016).

use super::stand::{Stand, Teilnehmerzeile, Wechsel};
use super::status::{Haltung, Status};
use super::vorschlag::freier_name;
use super::wunsch::{Art, Beitrittswunsch};
use crate::gespraeche::Kennung;
use crate::name::Name;
use crate::uhr::{Uhr, Zeit};
use crate::zufall::{Token, Zufall, Zufallsfehler};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

/// Lebende Sitzungen je Herkunft und gesamt — gleich den Verbindungsgrenzen (ADR-0012).
pub const JE_HERKUNFT: usize = 64;
pub const GESAMT: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sitzung {
    pub token: Token,
    pub kennung: Kennung,
    pub name: Name,
    pub herkunft: IpAddr,
    pub art: Art,
    /// Zeitpunkt des Beitritts.
    pub seit: Zeit,
    pub letztes_lebenszeichen: Zeit,
}

/// Ein gelungener Beitritt und die Wechsel, die er ausgelöst hat (zu veröffentlichen nach der Sperre).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Beitritt {
    pub sitzung: Sitzung,
    pub wechsel: Vec<Wechsel>,
}

/// Kennzahlen der Anwesenheit für `/api/gesundheit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Betrieb {
    pub sitzungen: usize,
    pub halter: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Beitrittsfehler {
    NameVergeben {
        belegt_von: Kennung,
        seit: Zeit,
        vorschlag: Name,
    },
    ZuVieleSitzungen,
    KeinZufall(Zufallsfehler),
}

/// Einziger Schreiber aller Sitzungen (A00010); nur im Speicher, ein Neustart beendet alle.
pub struct Anwesenheit {
    uhr: Arc<dyn Uhr>,
    zufall: Arc<dyn Zufall>,
    gnadenfrist: Duration,
    bestand: Mutex<Bestand>,
    veroeffentlichen: Veroeffentlichen,
}

/// Wohin jeder Wechsel geht — aufgerufen unter der Sperre, damit die Reihenfolge der Veröffentlichung die
/// Reihenfolge von `stand` ist (Nacharbeit 1, B1). Die Anwesenheit kennt den Bus nicht (ADR-0016 Regel 3).
pub type Veroeffentlichen = Arc<dyn Fn(&Wechsel) + Send + Sync>;

/// Was unter der Sperre steht: die Sitzungen mit ihrer Haltung und dem zuletzt gemeldeten Status, und die
/// laufende Nummer des letzten Wechsels.
#[derive(Default)]
struct Bestand {
    eintraege: HashMap<Token, Eintrag>,
    stand: u64,
    /// Die späteste Zeit, mit der unter der Sperre gerechnet wurde — „jetzt“ läuft nie dahinter zurück.
    zuletzt: Option<Zeit>,
}

#[derive(Clone)]
struct Eintrag {
    sitzung: Sitzung,
    haltung: Haltung,
    gemeldet: (Status, Zeit),
}

impl Eintrag {
    /// Lebt, solange ein Halter offen ist oder das letzte Lebenszeichen höchstens die Gnadenfrist
    /// zurückliegt (ADR-0012).
    fn lebt(&self, jetzt: Zeit, gnadenfrist: Duration) -> bool {
        self.haltung.halter > 0 || lebt(self.sitzung.letztes_lebenszeichen, jetzt, gnadenfrist)
    }

    fn kennung_text(&self) -> &str {
        self.sitzung.kennung.als_text()
    }

    /// Die Zeile mit dem zuletzt gemeldeten Status; bei `offline` mit `weg_um` aus dem letzten Lebenszeichen.
    fn zeile(&self, gnadenfrist: Duration) -> Teilnehmerzeile {
        let (status, seit) = self.gemeldet;
        let weg_um = (status == Status::Offline)
            .then(|| weg_um(self.sitzung.letztes_lebenszeichen, gnadenfrist));
        Teilnehmerzeile {
            kennung: self.sitzung.kennung.clone(),
            name: self.sitzung.name.clone(),
            herkunft: self.sitzung.herkunft,
            art: self.sitzung.art,
            status,
            seit,
            weg_um,
        }
    }
}

impl Bestand {
    fn monoton(&mut self, jetzt: Zeit) -> Zeit {
        let spaetestens = self.zuletzt.map_or(jetzt, |zuletzt| zuletzt.max(jetzt));
        self.zuletzt = Some(spaetestens);
        spaetestens
    }

    /// Entfernt jede Sitzung, die nicht mehr lebt, und meldet sie als `weg` — nach Kennung, nie still.
    fn abgelaufene_melden(&mut self, jetzt: Zeit, gnadenfrist: Duration) -> Vec<Wechsel> {
        let mut abgelaufen: Vec<Eintrag> = Vec::new();
        self.eintraege.retain(|_, eintrag| {
            let lebendig = eintrag.lebt(jetzt, gnadenfrist);
            if !lebendig {
                abgelaufen.push(eintrag.clone());
            }
            lebendig
        });
        abgelaufen.sort_by(|links, rechts| links.kennung_text().cmp(rechts.kennung_text()));
        abgelaufen
            .into_iter()
            .map(|eintrag| {
                self.stand += 1;
                Wechsel {
                    stand: self.stand,
                    jetzt,
                    teilnehmer: Teilnehmerzeile {
                        status: Status::Weg,
                        seit: jetzt,
                        weg_um: None,
                        ..eintrag.zeile(gnadenfrist)
                    },
                }
            })
            .collect()
    }

    /// Meldet jede Sitzung, deren Status sich gegenüber dem zuletzt gemeldeten geändert hat — nach Kennung.
    fn status_melden(&mut self, jetzt: Zeit, gnadenfrist: Duration) -> Vec<Wechsel> {
        let mut geaendert: Vec<(Token, Status, Zeit, String)> = self
            .eintraege
            .iter()
            .filter_map(|(token, eintrag)| {
                let (status, seit) = eintrag.haltung.status(jetzt);
                let gewechselt = status != eintrag.gemeldet.0;
                gewechselt.then(|| {
                    (
                        token.clone(),
                        status,
                        seit,
                        eintrag.kennung_text().to_string(),
                    )
                })
            })
            .collect();
        geaendert.sort_by(|links, rechts| links.3.cmp(&rechts.3));
        geaendert
            .into_iter()
            .filter_map(|(token, status, seit, _)| {
                self.melden(&token, status, seit, jetzt, gnadenfrist)
            })
            .collect()
    }

    /// Meldet `status` für den Eintrag: zählt `stand` hoch und liefert den Wechsel.
    fn melden(
        &mut self,
        token: &Token,
        status: Status,
        seit: Zeit,
        jetzt: Zeit,
        gnadenfrist: Duration,
    ) -> Option<Wechsel> {
        let eintrag = self.eintraege.get_mut(token)?;
        eintrag.gemeldet = (status, seit);
        self.stand += 1;
        Some(Wechsel {
            stand: self.stand,
            jetzt,
            teilnehmer: eintrag.zeile(gnadenfrist),
        })
    }
}

impl Anwesenheit {
    pub fn neu(uhr: Arc<dyn Uhr>, zufall: Arc<dyn Zufall>, gnadenfrist: Duration) -> Anwesenheit {
        Anwesenheit {
            uhr,
            zufall,
            gnadenfrist,
            bestand: Mutex::new(Bestand::default()),
            veroeffentlichen: Arc::new(|_: &Wechsel| {}),
        }
    }

    /// Jeder Wechsel geht unter der Sperre an `veroeffentlichen` (synchron, z. B. `broadcast::send`).
    pub fn mit_veroeffentlichen(self, veroeffentlichen: Veroeffentlichen) -> Anwesenheit {
        Anwesenheit {
            veroeffentlichen,
            ..self
        }
    }

    /// Unter der Sperre: Wechsel veröffentlichen, solange niemand einen weiteren melden kann.
    fn melden_unter_der_sperre(&self, _sperre: &MutexGuard<'_, Bestand>, wechsel: &[Wechsel]) {
        for einzeln in wechsel {
            (self.veroeffentlichen)(einzeln);
        }
    }

    /// So lange lebt eine Sitzung nach ihrem letzten Lebenszeichen (ADR-0012).
    pub fn gnadenfrist(&self) -> Duration {
        self.gnadenfrist
    }

    /// Unter einer Sperre: Name lebend belegt (egal von wo) → vergeben mit Vorschlag; Grenze je Herkunft
    /// oder gesamt erreicht → zu viele; sonst eine neue Sitzung mit frischem Token, gemeldet als `online`.
    /// Abgelaufene Sitzungen zählen nicht; gelingt der Beitritt, gehen sie mit `weg` vor dem `online`.
    pub fn beitreten(
        &self,
        wunsch: &Beitrittswunsch,
        herkunft: IpAddr,
    ) -> Result<Beitritt, Beitrittsfehler> {
        let (mut bestand, jetzt) = self.bestand_und_jetzt();
        self.beitritt_pruefen(&bestand, wunsch, herkunft, jetzt)?;
        let token = self.zufall.token().map_err(Beitrittsfehler::KeinZufall)?;
        let sitzung = Sitzung {
            token: token.clone(),
            kennung: Kennung::aus(&wunsch.name, herkunft),
            name: wunsch.name.clone(),
            herkunft,
            art: wunsch.art,
            seit: jetzt,
            letztes_lebenszeichen: jetzt,
        };
        let eintrag = Eintrag {
            sitzung: sitzung.clone(),
            haltung: Haltung {
                beitritt: jetzt,
                halter: 0,
                halter_ende: None,
                online_seit: jetzt,
            },
            gemeldet: (Status::Online, jetzt),
        };
        let mut wechsel = bestand.abgelaufene_melden(jetzt, self.gnadenfrist);
        bestand.eintraege.insert(token.clone(), eintrag);
        wechsel.extend(bestand.melden(&token, Status::Online, jetzt, jetzt, self.gnadenfrist));
        self.melden_unter_der_sperre(&bestand, &wechsel);
        Ok(Beitritt { sitzung, wechsel })
    }

    fn beitritt_pruefen(
        &self,
        bestand: &Bestand,
        wunsch: &Beitrittswunsch,
        herkunft: IpAddr,
        jetzt: Zeit,
    ) -> Result<(), Beitrittsfehler> {
        let lebende: Vec<&Sitzung> = bestand
            .eintraege
            .values()
            .filter(|eintrag| eintrag.lebt(jetzt, self.gnadenfrist))
            .map(|eintrag| &eintrag.sitzung)
            .collect();
        let traeger = lebende.iter().find(|sitzung| sitzung.name == wunsch.name);
        if let Some(traeger) = traeger {
            let belegt = |name: &Name| lebende.iter().any(|sitzung| &sitzung.name == name);
            return Err(Beitrittsfehler::NameVergeben {
                belegt_von: traeger.kennung.clone(),
                seit: traeger.seit,
                vorschlag: freier_name(&wunsch.name, belegt),
            });
        }
        let von_dieser_herkunft = lebende
            .iter()
            .filter(|sitzung| sitzung.herkunft == herkunft)
            .count();
        if von_dieser_herkunft >= JE_HERKUNFT || lebende.len() >= GESAMT {
            return Err(Beitrittsfehler::ZuVieleSitzungen);
        }
        Ok(())
    }

    /// Gültiges Token derselben Herkunft → die Sitzung, ihr Lebenszeichen erneuert; sonst keine
    /// (unbekannt, abgelaufen, andere Herkunft — eine fremde Herkunft erneuert nichts).
    pub fn wiederkehren(&self, token: &Token, herkunft: IpAddr) -> Option<Sitzung> {
        let (mut bestand, jetzt) = self.bestand_und_jetzt();
        let eintrag = self.lebender_eintrag(&mut bestand, token, herkunft, jetzt)?;
        eintrag.sitzung.letztes_lebenszeichen = jetzt;
        Some(eintrag.sitzung.clone())
    }

    /// Ein Halter der Sitzung (ADR-0016 Regel 2), nur mit gültigem Token derselben Herkunft; ein
    /// Lebenszeichen. War die Sitzung `offline`, ist sie ab jetzt `online` — der Wechsel kommt mit.
    pub fn verbunden(
        self: &Arc<Self>,
        token: &Token,
        herkunft: IpAddr,
    ) -> Option<(Halter, Vec<Wechsel>)> {
        let (mut bestand, jetzt) = self.bestand_und_jetzt();
        let eintrag = self.lebender_eintrag(&mut bestand, token, herkunft, jetzt)?;
        eintrag.sitzung.letztes_lebenszeichen = jetzt;
        eintrag.haltung.halter_beginnt(jetzt);
        let wechsel = bestand.status_melden(jetzt, self.gnadenfrist);
        self.melden_unter_der_sperre(&bestand, &wechsel);
        let halter = Halter {
            anwesenheit: Arc::clone(self),
            token: token.clone(),
        };
        Some((halter, wechsel))
    }

    /// Der Takt (ADR-0016 Regel 4): Gnadenfrist ohne Lebenszeichen und ohne Halter vorbei → `weg` (Sitzung
    /// entfernt, Name frei); Nachlauf vorbei ohne Halter → `offline`. Je Sitzung nur echte Wechsel.
    pub fn pflegen(&self, jetzt: Zeit) -> Vec<Wechsel> {
        let mut bestand = self.bestand();
        let jetzt = bestand.monoton(jetzt);
        self.pflegen_unter_der_sperre(&mut bestand, jetzt)
    }

    fn pflegen_unter_der_sperre(
        &self,
        bestand: &mut MutexGuard<'_, Bestand>,
        jetzt: Zeit,
    ) -> Vec<Wechsel> {
        let mut wechsel = bestand.abgelaufene_melden(jetzt, self.gnadenfrist);
        wechsel.extend(bestand.status_melden(jetzt, self.gnadenfrist));
        self.melden_unter_der_sperre(bestand, &wechsel);
        wechsel
    }

    /// Alle lebenden Sitzungen mit ihrem zuletzt gemeldeten Status, nach Kennung sortiert, und der Stand des
    /// letzten Wechsels — die Liste passt zu den Wechseln, die bis zu diesem Stand veröffentlicht wurden.
    pub fn teilnehmer(&self) -> Stand {
        self.stand_mit(|| ()).0
    }

    /// Der Stand und, unter derselben Sperre, `unter_der_sperre` — etwa das Abonnieren des Busses: zwischen
    /// Abonnieren und Stand kann so kein Wechsel liegen (ADR-0016 Regel 5).
    /// Vorher pflegt sie unter derselben Sperre wie der Takt (Wechsel veröffentlicht): die Liste ist zum
    /// Abrufzeitpunkt aktuell und passt trotzdem zu den Wechseln bis zu ihrem Stand (Nacharbeit 2, US-4).
    pub fn stand_mit<T>(&self, unter_der_sperre: impl FnOnce() -> T) -> (Stand, T) {
        let (mut bestand, jetzt) = self.bestand_und_jetzt();
        self.pflegen_unter_der_sperre(&mut bestand, jetzt);
        let ergebnis = unter_der_sperre();
        let mut liste: Vec<Teilnehmerzeile> = bestand
            .eintraege
            .values()
            .filter(|eintrag| eintrag.lebt(jetzt, self.gnadenfrist))
            .map(|eintrag| eintrag.zeile(self.gnadenfrist))
            .collect();
        liste.sort_by(|links, rechts| links.kennung.als_text().cmp(rechts.kennung.als_text()));
        let stand = Stand {
            stand: bestand.stand,
            jetzt,
            liste,
        };
        (stand, ergebnis)
    }

    /// Die Serverzeit, unter der Sperre gelesen und nie früher als jede schon verwendete (ADR-0019 Regel 2).
    pub fn jetzt(&self) -> Zeit {
        self.bestand_und_jetzt().1
    }

    /// Für den Betrieb: wie viele Sitzungen leben und wie viele Halter (offene Ströme) sie zusammen haben.
    pub fn betrieb(&self) -> Betrieb {
        let (bestand, jetzt) = self.bestand_und_jetzt();
        let lebende = bestand
            .eintraege
            .values()
            .filter(|eintrag| eintrag.lebt(jetzt, self.gnadenfrist));
        let (sitzungen, halter) = lebende.fold((0, 0), |(sitzungen, halter), eintrag| {
            (sitzungen + 1, halter + eintrag.haltung.halter)
        });
        Betrieb { sitzungen, halter }
    }

    /// Der Eintrag zum Token, wenn er lebt und von derselben Herkunft kommt.
    fn lebender_eintrag<'b>(
        &self,
        bestand: &'b mut Bestand,
        token: &Token,
        herkunft: IpAddr,
        jetzt: Zeit,
    ) -> Option<&'b mut Eintrag> {
        bestand.eintraege.get_mut(token).filter(|eintrag| {
            eintrag.sitzung.herkunft == herkunft && eintrag.lebt(jetzt, self.gnadenfrist)
        })
    }

    /// Die Sperre und „jetzt“, gelesen unter ihr und nie früher als die zuletzt verwendete Zeit: ein Abruf,
    /// der die Sperre nach dem Takt bekommt, oder eine zurückspringende Uhr dreht keinen Wechsel zurück
    /// (Nacharbeit 3, B1).
    fn bestand_und_jetzt(&self) -> (MutexGuard<'_, Bestand>, Zeit) {
        let mut bestand = self.bestand();
        let jetzt = bestand.monoton(self.uhr.jetzt());
        (bestand, jetzt)
    }

    fn bestand(&self) -> MutexGuard<'_, Bestand> {
        self.bestand.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Ein offener Strom (später ein Warte-Aufruf) seiner Sitzung: solange er lebt, ist sie `online`; sein
/// `Drop` zählt herunter und ist ein Lebenszeichen (ADR-0016 Regel 2). Ohne `await`, die Sperre nur kurz.
pub struct Halter {
    anwesenheit: Arc<Anwesenheit>,
    token: Token,
}

impl Drop for Halter {
    fn drop(&mut self) {
        let (mut bestand, jetzt) = self.anwesenheit.bestand_und_jetzt();
        if let Some(eintrag) = bestand.eintraege.get_mut(&self.token) {
            eintrag.haltung.halter_endet(jetzt);
            eintrag.sitzung.letztes_lebenszeichen = jetzt;
        }
    }
}

/// Ohne Halter lebt eine Sitzung, solange ihr letztes Lebenszeichen höchstens die Gnadenfrist zurückliegt —
/// weg ist sie erst, wenn `jetzt > weg_um`.
fn lebt(letztes_lebenszeichen: Zeit, jetzt: Zeit, gnadenfrist: Duration) -> bool {
    jetzt <= weg_um(letztes_lebenszeichen, gnadenfrist)
}

/// Der letzte Augenblick, in dem eine Sitzung ohne Halter noch lebt (ADR-0019 Regel 1, B0105).
fn weg_um(letztes_lebenszeichen: Zeit, gnadenfrist: Duration) -> Zeit {
    let frist = chrono::Duration::from_std(gnadenfrist).unwrap_or(chrono::Duration::MAX);
    letztes_lebenszeichen
        .checked_add_signed(frist)
        .unwrap_or(Zeit::MAX_UTC.fixed_offset())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uhr::GestellteUhr;
    use crate::zufall::FesteFolge;
    use std::net::Ipv4Addr;

    const T0: &str = "2026-10-27T09:14:00.000+01:00";

    fn zeit(text: &str) -> Zeit {
        Zeit::parse_from_rfc3339(text).expect("Zeit")
    }

    fn anwesenheit(zufall: FesteFolge) -> Anwesenheit {
        Anwesenheit::neu(
            Arc::new(GestellteUhr::neu(zeit(T0))),
            Arc::new(zufall),
            Duration::from_secs(60),
        )
    }

    fn wunsch(name: &str, art: Art) -> Beitrittswunsch {
        Beitrittswunsch {
            name: Name::bilden(name).expect("Name"),
            art,
        }
    }

    fn ip(text: &str) -> IpAddr {
        text.parse().expect("IP")
    }

    #[test]
    fn beitreten_ergibt_kennung_name_at_herkunft_mit_token_aus_dem_zufall() {
        let anwesenheit = anwesenheit(FesteFolge::neu([7]));

        let sitzung = anwesenheit
            .beitreten(&wunsch("claude", Art::Agent), ip("10.0.0.5"))
            .expect("beigetreten")
            .sitzung;

        assert_eq!(sitzung.kennung.als_text(), "claude@10.0.0.5");
        assert_eq!(sitzung.token.als_text(), format!("{:032x}", 7));
        assert_eq!(
            (sitzung.name.als_text(), sitzung.art),
            ("claude", Art::Agent)
        );
        assert_eq!(sitzung.herkunft, ip("10.0.0.5"));
        assert_eq!(
            (sitzung.seit, sitzung.letztes_lebenszeichen),
            (zeit(T0), zeit(T0))
        );
    }

    #[test]
    fn derselbe_name_von_anderswo_ist_vergeben_mit_traeger_seit_und_vorschlag() {
        let anwesenheit = anwesenheit(FesteFolge::neu([1, 2, 3]));
        anwesenheit
            .beitreten(&wunsch("claude", Art::Agent), ip("10.0.0.5"))
            .expect("erster");

        let zweiter = anwesenheit.beitreten(&wunsch("claude", Art::Mensch), ip("127.0.0.1"));
        let gleiche_herkunft = anwesenheit.beitreten(&wunsch("claude", Art::Agent), ip("10.0.0.5"));

        let erwartet = Beitrittsfehler::NameVergeben {
            belegt_von: Kennung::bilden("claude@10.0.0.5").expect("Kennung"),
            seit: zeit(T0),
            vorschlag: Name::bilden("claude-2").expect("Name"),
        };
        assert_eq!(zweiter, Err(erwartet.clone()));
        assert_eq!(gleiche_herkunft, Err(erwartet));
    }

    #[test]
    fn der_vorschlag_ueberspringt_belegte_namen() {
        let anwesenheit = anwesenheit(FesteFolge::neu([1, 2]));
        for name in ["claude", "claude-2"] {
            anwesenheit
                .beitreten(&wunsch(name, Art::Agent), ip("10.0.0.5"))
                .expect(name);
        }

        let vergeben = anwesenheit.beitreten(&wunsch("claude", Art::Agent), ip("127.0.0.1"));

        assert!(
            matches!(&vergeben, Err(Beitrittsfehler::NameVergeben { vorschlag, .. }) if vorschlag.als_text() == "claude-3"),
            "{vergeben:?}"
        );
    }

    #[test]
    fn hoechstens_64_sitzungen_je_herkunft_und_512_gesamt() {
        let anwesenheit = anwesenheit(FesteFolge::neu(1..=600));
        for nummer in 1..=JE_HERKUNFT {
            anwesenheit
                .beitreten(&wunsch(&format!("n{nummer}"), Art::Agent), ip("127.0.0.1"))
                .expect("innerhalb der Grenze");
        }

        let zu_viel = anwesenheit.beitreten(&wunsch("n65", Art::Agent), ip("127.0.0.1"));
        let andere_herkunft = anwesenheit.beitreten(&wunsch("n65", Art::Agent), ip("127.0.0.2"));

        assert_eq!(zu_viel, Err(Beitrittsfehler::ZuVieleSitzungen));
        assert!(andere_herkunft.is_ok());

        let mut nummer = 0;
        for letzte_stelle in 2..=8u8 {
            let herkunft = IpAddr::V4(Ipv4Addr::new(127, 0, 0, letzte_stelle));
            let schon_da = usize::from(letzte_stelle == 2);
            for _ in schon_da..JE_HERKUNFT {
                nummer += 1;
                anwesenheit
                    .beitreten(&wunsch(&format!("m{nummer}"), Art::Agent), herkunft)
                    .expect("innerhalb der Gesamtgrenze");
            }
        }
        assert_eq!(anwesenheit.bestand().eintraege.len(), GESAMT);
        let ueber_gesamt = anwesenheit.beitreten(&wunsch("neu", Art::Agent), ip("127.0.0.9"));
        assert_eq!(ueber_gesamt, Err(Beitrittsfehler::ZuVieleSitzungen));
    }

    #[test]
    fn ohne_zufall_kein_beitritt_und_der_name_bleibt_frei() {
        let anwesenheit = anwesenheit(FesteFolge::neu([]));

        let ergebnis = anwesenheit.beitreten(&wunsch("claude", Art::Agent), ip("10.0.0.5"));

        assert!(
            matches!(ergebnis, Err(Beitrittsfehler::KeinZufall(_))),
            "{ergebnis:?}"
        );
        assert!(anwesenheit.bestand().eintraege.is_empty());
    }

    #[test]
    fn lebt_bis_einschliesslich_der_gnadenfrist_danach_nicht_mehr() {
        let frist = Duration::from_secs(60);
        let t0 = zeit(T0);
        let nach = |millisekunden: i64| t0 + chrono::Duration::milliseconds(millisekunden);

        assert!(lebt(t0, t0, frist));
        assert!(lebt(t0, nach(60_000), frist));
        assert!(!lebt(t0, nach(60_001), frist));
        assert!(!lebt(t0, nach(3_600_000), frist));
        assert!(
            lebt(t0, nach(-5_000), frist),
            "eine zurückgestellte Uhr beendet nichts"
        );
    }

    #[test]
    fn nach_der_gnadenfrist_ist_der_name_frei_und_die_sitzung_zaehlt_nicht_mehr() {
        let uhr = Arc::new(GestellteUhr::neu(zeit(T0)));
        let anwesenheit = Anwesenheit::neu(
            Arc::clone(&uhr) as Arc<dyn Uhr>,
            Arc::new(FesteFolge::neu(1..=100)),
            Duration::from_secs(60),
        );
        anwesenheit
            .beitreten(&wunsch("claude", Art::Agent), ip("10.0.0.5"))
            .expect("beigetreten");

        uhr.stellen(zeit("2026-10-27T09:14:59.000+01:00"));
        let nach_59 = anwesenheit.beitreten(&wunsch("claude", Art::Mensch), ip("127.0.0.1"));
        uhr.stellen(zeit("2026-10-27T09:15:01.000+01:00"));
        let nach_61 = anwesenheit.beitreten(&wunsch("claude", Art::Mensch), ip("127.0.0.1"));

        assert!(
            matches!(nach_59, Err(Beitrittsfehler::NameVergeben { .. })),
            "{nach_59:?}"
        );
        assert_eq!(
            nach_61.map(|beitritt| beitritt.sitzung.kennung.to_string()),
            Ok("claude@127.0.0.1".into())
        );
        assert_eq!(
            anwesenheit.bestand().eintraege.len(),
            1,
            "die abgelaufene Sitzung ist weg"
        );
    }

    #[test]
    fn abgelaufene_sitzungen_zaehlen_nicht_gegen_die_grenze() {
        let uhr = Arc::new(GestellteUhr::neu(zeit(T0)));
        let anwesenheit = Anwesenheit::neu(
            Arc::clone(&uhr) as Arc<dyn Uhr>,
            Arc::new(FesteFolge::neu(1..=100)),
            Duration::from_secs(60),
        );
        for nummer in 1..=JE_HERKUNFT {
            anwesenheit
                .beitreten(&wunsch(&format!("n{nummer}"), Art::Agent), ip("127.0.0.1"))
                .expect("innerhalb der Grenze");
        }

        uhr.stellen(zeit("2026-10-27T09:16:00.000+01:00"));
        let danach = anwesenheit.beitreten(&wunsch("n65", Art::Agent), ip("127.0.0.1"));

        assert!(danach.is_ok(), "{danach:?}");
    }

    fn mit_gestellter_uhr() -> (Arc<GestellteUhr>, Anwesenheit) {
        let uhr = Arc::new(GestellteUhr::neu(zeit(T0)));
        let anwesenheit = Anwesenheit::neu(
            Arc::clone(&uhr) as Arc<dyn Uhr>,
            Arc::new(FesteFolge::neu(1..=100)),
            Duration::from_secs(60),
        );
        (uhr, anwesenheit)
    }

    fn sekunden_nach_t0(sekunden: i64) -> Zeit {
        zeit(T0) + chrono::Duration::seconds(sekunden)
    }

    /// Erster Test aus A00010 und Kriterium „Standard ohne Testknopf“ (R00003, F0009).
    #[test]
    fn wiederkehren_erneuert_das_lebenszeichen_danach_laeuft_die_frist_neu() {
        let (uhr, anwesenheit) = mit_gestellter_uhr();
        let sitzung = anwesenheit
            .beitreten(&wunsch("claude", Art::Agent), ip("10.0.0.5"))
            .expect("beigetreten")
            .sitzung;

        uhr.stellen(sekunden_nach_t0(59));
        let vergeben_bei_59 =
            anwesenheit.beitreten(&wunsch("claude", Art::Mensch), ip("127.0.0.1"));
        let wiedergekehrt = anwesenheit.wiederkehren(&sitzung.token, ip("10.0.0.5"));
        uhr.stellen(sekunden_nach_t0(59 + 59));
        let vergeben_bei_118 =
            anwesenheit.beitreten(&wunsch("claude", Art::Mensch), ip("127.0.0.1"));
        uhr.stellen(sekunden_nach_t0(59 + 61));
        let frei_bei_120 = anwesenheit.beitreten(&wunsch("claude", Art::Mensch), ip("127.0.0.1"));

        assert!(matches!(
            vergeben_bei_59,
            Err(Beitrittsfehler::NameVergeben { .. })
        ));
        let wiedergekehrt = wiedergekehrt.expect("Sitzung");
        assert_eq!(wiedergekehrt.kennung, sitzung.kennung);
        assert_eq!(wiedergekehrt.letztes_lebenszeichen, sekunden_nach_t0(59));
        assert_eq!(wiedergekehrt.seit, zeit(T0));
        assert!(
            matches!(vergeben_bei_118, Err(Beitrittsfehler::NameVergeben { .. })),
            "{vergeben_bei_118:?}"
        );
        assert!(frei_bei_120.is_ok(), "{frei_bei_120:?}");
        assert_eq!(
            anwesenheit.wiederkehren(&sitzung.token, ip("10.0.0.5")),
            None,
            "altes Token ungültig"
        );
    }

    #[test]
    fn eine_andere_herkunft_kehrt_nicht_wieder_und_erneuert_nichts() {
        let (uhr, anwesenheit) = mit_gestellter_uhr();
        let sitzung = anwesenheit
            .beitreten(&wunsch("claude", Art::Agent), ip("10.0.0.5"))
            .expect("beigetreten")
            .sitzung;

        uhr.stellen(sekunden_nach_t0(30));
        let fremd = anwesenheit.wiederkehren(&sitzung.token, ip("10.0.0.6"));
        uhr.stellen(sekunden_nach_t0(61));
        let frei = anwesenheit.beitreten(&wunsch("claude", Art::Agent), ip("127.0.0.1"));

        assert_eq!(fremd, None);
        assert!(
            frei.is_ok(),
            "die fremde Herkunft hat kein Lebenszeichen gesetzt: {frei:?}"
        );
    }

    #[test]
    fn ein_unbekanntes_token_kehrt_nicht_wieder() {
        let (_, anwesenheit) = mit_gestellter_uhr();
        anwesenheit
            .beitreten(&wunsch("claude", Art::Agent), ip("10.0.0.5"))
            .expect("beigetreten");
        let unbekannt = Token::lesen("00000000000000000000000000000000").expect("Token");

        assert_eq!(anwesenheit.wiederkehren(&unbekannt, ip("10.0.0.5")), None);
    }

    fn nach_ms(millisekunden: i64) -> Zeit {
        zeit(T0) + chrono::Duration::milliseconds(millisekunden)
    }

    fn zeile(kennung: &str, art: Art, status: Status, seit: Zeit) -> Teilnehmerzeile {
        let kennung = Kennung::bilden(kennung).expect("Kennung");
        let (name, herkunft) = kennung.als_text().split_once('@').expect("@");
        Teilnehmerzeile {
            name: Name::bilden(name).expect("Name"),
            herkunft: ip(herkunft),
            kennung: kennung.clone(),
            art,
            status,
            seit,
            weg_um: None,
        }
    }

    #[test]
    fn beitreten_meldet_online_und_der_stand_nennt_alle_lebenden_nach_kennung() {
        let (uhr, anwesenheit) = mit_gestellter_uhr();
        let beta = anwesenheit
            .beitreten(&wunsch("beta", Art::Agent), ip("127.0.0.3"))
            .expect("beta");
        uhr.stellen(nach_ms(200));
        let alpha = anwesenheit
            .beitreten(&wunsch("alpha", Art::Mensch), ip("127.0.0.2"))
            .expect("alpha");
        let vergeben = anwesenheit.beitreten(&wunsch("alpha", Art::Agent), ip("127.0.0.3"));
        let stand = anwesenheit.teilnehmer();

        let beta_online = zeile("beta@127.0.0.3", Art::Agent, Status::Online, nach_ms(0));
        let alpha_online = zeile("alpha@127.0.0.2", Art::Mensch, Status::Online, nach_ms(200));
        assert_eq!(
            beta.wechsel,
            vec![Wechsel {
                stand: 1,
                jetzt: nach_ms(0),
                teilnehmer: beta_online.clone()
            }]
        );
        assert_eq!(
            alpha.wechsel,
            vec![Wechsel {
                stand: 2,
                jetzt: nach_ms(200),
                teilnehmer: alpha_online.clone()
            }]
        );
        assert!(vergeben.is_err());
        assert_eq!(
            stand,
            Stand {
                stand: 2,
                jetzt: nach_ms(200),
                liste: vec![alpha_online, beta_online]
            },
            "nach Kennung sortiert, ein abgewiesener Beitritt zählt nicht"
        );
    }

    #[test]
    fn der_stand_nennt_keine_abgelaufene_sitzung() {
        let (uhr, anwesenheit) = mit_gestellter_uhr();
        anwesenheit
            .beitreten(&wunsch("claude", Art::Agent), ip("10.0.0.5"))
            .expect("beigetreten");

        uhr.stellen(sekunden_nach_t0(61));

        assert_eq!(anwesenheit.teilnehmer().liste, vec![]);
    }

    #[test]
    fn halter_nur_mit_gueltigem_token_derselben_herkunft() {
        let (_, anwesenheit) = mit_gestellter_uhr();
        let anwesenheit = Arc::new(anwesenheit);
        let sitzung = anwesenheit
            .beitreten(&wunsch("claude", Art::Agent), ip("10.0.0.5"))
            .expect("beigetreten")
            .sitzung;
        let unbekannt = Token::lesen("00000000000000000000000000000000").expect("Token");

        assert!(
            anwesenheit
                .verbunden(&sitzung.token, ip("10.0.0.6"))
                .is_none()
        );
        assert!(anwesenheit.verbunden(&unbekannt, ip("10.0.0.5")).is_none());
        assert_eq!(haltung(&anwesenheit, &sitzung.token).halter, 0);
    }

    #[test]
    fn mehrere_halter_je_sitzung_erst_der_letzte_setzt_ende_und_lebenszeichen() {
        let (uhr, anwesenheit) = mit_gestellter_uhr();
        let anwesenheit = Arc::new(anwesenheit);
        let sitzung = anwesenheit
            .beitreten(&wunsch("claude", Art::Agent), ip("10.0.0.5"))
            .expect("beigetreten")
            .sitzung;

        uhr.stellen(nach_ms(500));
        let (erster, wechsel_erster) = anwesenheit
            .verbunden(&sitzung.token, ip("10.0.0.5"))
            .expect("erster Halter");
        let (zweiter, wechsel_zweiter) = anwesenheit
            .verbunden(&sitzung.token, ip("10.0.0.5"))
            .expect("zweiter Halter");
        let mit_zweien = haltung(&anwesenheit, &sitzung.token);
        uhr.stellen(nach_ms(30_000));
        drop(erster);
        let nach_dem_ersten = haltung(&anwesenheit, &sitzung.token);
        uhr.stellen(nach_ms(40_000));
        drop(zweiter);
        let nach_dem_zweiten = haltung(&anwesenheit, &sitzung.token);

        assert_eq!(wechsel_erster, vec![], "im Anlauf schon online gemeldet");
        assert_eq!(wechsel_zweiter, vec![]);
        assert_eq!((mit_zweien.halter, mit_zweien.halter_ende), (2, None));
        assert_eq!(
            (nach_dem_ersten.halter, nach_dem_ersten.halter_ende),
            (1, None)
        );
        assert_eq!(
            (nach_dem_zweiten.halter, nach_dem_zweiten.halter_ende),
            (0, Some(nach_ms(40_000)))
        );
        assert_eq!(nach_dem_zweiten.online_seit, nach_ms(0));
        assert_eq!(
            lebenszeichen(&anwesenheit, &sitzung.token),
            nach_ms(40_000),
            "das Ende des letzten Halters ist ein Lebenszeichen"
        );
    }

    fn haltung(anwesenheit: &Anwesenheit, token: &Token) -> Haltung {
        anwesenheit.bestand().eintraege[token].haltung
    }

    fn lebenszeichen(anwesenheit: &Anwesenheit, token: &Token) -> Zeit {
        anwesenheit.bestand().eintraege[token]
            .sitzung
            .letztes_lebenszeichen
    }

    fn kurz(wechsel: &[Wechsel]) -> Vec<(u64, String, Status, Zeit)> {
        wechsel
            .iter()
            .map(|wechsel| {
                let zeile = &wechsel.teilnehmer;
                (
                    wechsel.stand,
                    zeile.kennung.to_string(),
                    zeile.status,
                    zeile.seit,
                )
            })
            .collect()
    }

    /// Erster Unit-Test aus ADR-0016 („Folgeentscheidungen“), gestellte Uhr.
    #[test]
    fn adr_0016_anlauf_nachlauf_und_halter_ergeben_nur_echte_wechsel_mit_steigendem_stand() {
        let (uhr, anwesenheit) = mit_gestellter_uhr();
        let anwesenheit = Arc::new(anwesenheit);
        let claude = "claude@10.0.0.5".to_string();
        let beitritt = anwesenheit
            .beitreten(&wunsch("claude", Art::Agent), ip("10.0.0.5"))
            .expect("beigetreten");
        let token = beitritt.sitzung.token.clone();
        let verbinden = || {
            anwesenheit
                .verbunden(&token, ip("10.0.0.5"))
                .expect("Halter")
        };

        assert_eq!(
            kurz(&beitritt.wechsel),
            [(1, claude.clone(), Status::Online, nach_ms(0))]
        );
        assert_eq!(kurz(&anwesenheit.pflegen(nach_ms(999))), []);
        assert_eq!(
            kurz(&anwesenheit.pflegen(nach_ms(1000))),
            [(2, claude.clone(), Status::Offline, nach_ms(1000))]
        );
        uhr.stellen(nach_ms(1200));
        let (erster, online) = verbinden();
        assert_eq!(
            kurz(&online),
            [(3, claude.clone(), Status::Online, nach_ms(1200))]
        );
        let (zweiter, nichts) = verbinden();
        assert_eq!(kurz(&nichts), []);
        drop(erster);
        assert_eq!(kurz(&anwesenheit.pflegen(nach_ms(5000))), []);
        uhr.stellen(nach_ms(6000));
        drop(zweiter);
        uhr.stellen(nach_ms(6500));
        let (dritter, neu_geladen) = verbinden();
        assert_eq!(
            kurz(&neu_geladen),
            [],
            "Neuladen im Nachlauf bleibt unsichtbar"
        );
        assert_eq!(kurz(&anwesenheit.pflegen(nach_ms(8000))), []);
        uhr.stellen(nach_ms(9000));
        drop(dritter);
        assert_eq!(kurz(&anwesenheit.pflegen(nach_ms(9999))), []);
        assert_eq!(
            kurz(&anwesenheit.pflegen(nach_ms(10_000))),
            [(4, claude.clone(), Status::Offline, nach_ms(10_000))]
        );
        assert_eq!(anwesenheit.teilnehmer().stand, 4);
        assert_eq!(
            kurz(&anwesenheit.pflegen(nach_ms(10_250))),
            [],
            "offline wird einmal gemeldet"
        );
    }

    #[test]
    fn rechenbeispiel_f0011_stand_waechst_mit_dem_wechsel_nach_offline() {
        let (uhr, anwesenheit) = mit_gestellter_uhr();
        anwesenheit
            .beitreten(&wunsch("alpha", Art::Agent), ip("127.0.0.2"))
            .expect("alpha");

        uhr.stellen(nach_ms(500));
        anwesenheit.pflegen(nach_ms(500));
        let frueh = anwesenheit.teilnehmer();
        uhr.stellen(nach_ms(1500));
        anwesenheit.pflegen(nach_ms(1500));
        let spaet = anwesenheit.teilnehmer();

        assert_eq!(
            (frueh.liste[0].status, frueh.liste[0].seit),
            (Status::Online, nach_ms(0))
        );
        assert_eq!(
            (spaet.liste[0].status, spaet.liste[0].seit),
            (Status::Offline, nach_ms(1000))
        );
        assert!(spaet.stand > frueh.stand);
    }

    #[test]
    fn nach_der_gnadenfrist_ohne_lebenszeichen_meldet_pflegen_weg_und_der_name_ist_frei() {
        let (uhr, anwesenheit) = mit_gestellter_uhr();
        anwesenheit
            .beitreten(&wunsch("gamma", Art::Agent), ip("127.0.0.2"))
            .expect("gamma");
        anwesenheit.pflegen(nach_ms(1000));

        let noch_da = anwesenheit.pflegen(nach_ms(60_000));
        let weg = anwesenheit.pflegen(nach_ms(60_001));
        let danach = anwesenheit.pflegen(nach_ms(60_250));
        uhr.stellen(nach_ms(60_300));
        let neu = anwesenheit.beitreten(&wunsch("gamma", Art::Agent), ip("127.0.0.3"));

        assert_eq!(kurz(&noch_da), []);
        assert_eq!(
            kurz(&weg),
            [(3, "gamma@127.0.0.2".into(), Status::Weg, nach_ms(60_001))]
        );
        assert_eq!(kurz(&danach), [], "weg wird einmal gemeldet");
        assert!(neu.is_ok(), "{neu:?}");
    }

    /// Kriterium „Regel weg“ (R00005 F0017, ADR-0019 Regel 1): fällt der Halter bei t, ist die Sitzung ab
    /// t + 1 s `offline` mit `weg_um` = t + 60 s, bei t + 60 s noch da, bei t + 60 s + 1 ms `weg`.
    #[test]
    fn faellt_der_halter_bei_t_ist_weg_um_t_plus_gnadenfrist_und_weg_erst_danach() {
        let (uhr, anwesenheit) = mit_gestellter_uhr();
        let anwesenheit = Arc::new(anwesenheit);
        let token = anwesenheit
            .beitreten(&wunsch("alpha", Art::Agent), ip("127.0.0.2"))
            .expect("alpha")
            .sitzung
            .token;
        let (halter, _) = anwesenheit
            .verbunden(&token, ip("127.0.0.2"))
            .expect("Halter");
        let t = nach_ms(5000);
        let nach_t = |millisekunden: i64| t + chrono::Duration::milliseconds(millisekunden);
        uhr.stellen(t);
        drop(halter);

        let offline = anwesenheit.pflegen(nach_t(1000));
        let noch_da = anwesenheit.pflegen(nach_t(60_000));
        let weg = anwesenheit.pflegen(nach_t(60_001));

        assert_eq!(offline.len(), 1, "{offline:?}");
        let zeile = &offline[0].teilnehmer;
        assert_eq!(
            (zeile.status, zeile.seit, zeile.weg_um),
            (Status::Offline, nach_t(1000), Some(nach_t(60_000)))
        );
        assert_eq!(noch_da, vec![]);
        assert_eq!(
            kurz(&weg),
            [(3, "alpha@127.0.0.2".into(), Status::Weg, nach_t(60_001))]
        );
        assert!(weg[0].teilnehmer.seit > nach_t(60_000));
    }

    #[test]
    fn online_zeilen_tragen_kein_weg_um_offline_zeilen_im_stand_schon() {
        let (uhr, anwesenheit) = mit_gestellter_uhr();
        anwesenheit
            .beitreten(&wunsch("alpha", Art::Agent), ip("127.0.0.2"))
            .expect("alpha");
        let online = anwesenheit.teilnehmer();
        uhr.stellen(nach_ms(2000));
        let offline = anwesenheit.teilnehmer();

        assert_eq!(online.liste[0].weg_um, None);
        assert_eq!(
            (
                offline.liste[0].status,
                offline.liste[0].seit,
                offline.liste[0].weg_um
            ),
            (Status::Offline, nach_ms(1000), Some(nach_ms(60_000)))
        );
    }

    #[test]
    fn ein_beitritt_nach_abgelaufener_sitzung_meldet_erst_weg_dann_online() {
        let (uhr, anwesenheit) = mit_gestellter_uhr();
        anwesenheit
            .beitreten(&wunsch("gamma", Art::Agent), ip("127.0.0.2"))
            .expect("gamma");

        uhr.stellen(nach_ms(61_000));
        let neu = anwesenheit
            .beitreten(&wunsch("gamma", Art::Agent), ip("127.0.0.2"))
            .expect("neu");

        assert_eq!(
            kurz(&neu.wechsel),
            [
                (2, "gamma@127.0.0.2".into(), Status::Weg, nach_ms(61_000)),
                (3, "gamma@127.0.0.2".into(), Status::Online, nach_ms(61_000)),
            ],
            "das weg der alten Sitzung geht nicht verloren (Code-Review A3, Karte #3)"
        );
    }

    #[test]
    fn ein_offener_halter_haelt_die_sitzung_ueber_die_gnadenfrist_hinaus_am_leben() {
        let (uhr, anwesenheit) = mit_gestellter_uhr();
        let anwesenheit = Arc::new(anwesenheit);
        let sitzung = anwesenheit
            .beitreten(&wunsch("claude", Art::Agent), ip("10.0.0.5"))
            .expect("beigetreten")
            .sitzung;
        let (halter, _) = anwesenheit
            .verbunden(&sitzung.token, ip("10.0.0.5"))
            .expect("Halter");

        uhr.stellen(sekunden_nach_t0(7200));
        let gepflegt = anwesenheit.pflegen(sekunden_nach_t0(7200));
        let wieder = anwesenheit.wiederkehren(&sitzung.token, ip("10.0.0.5"));
        let liste = anwesenheit.teilnehmer().liste;

        assert_eq!(kurz(&gepflegt), []);
        assert!(wieder.is_some());
        assert_eq!(liste.len(), 1);
        drop(halter);
    }

    #[test]
    fn der_stand_ist_zum_abrufzeitpunkt_gepflegt_ohne_takt() {
        let (uhr, anwesenheit) = mit_gestellter_uhr();
        anwesenheit
            .beitreten(&wunsch("alpha", Art::Agent), ip("127.0.0.2"))
            .expect("alpha");

        uhr.stellen(nach_ms(999));
        let davor = anwesenheit.teilnehmer();
        uhr.stellen(nach_ms(1000));
        let danach = anwesenheit.teilnehmer();

        assert_eq!((davor.liste[0].status, davor.stand), (Status::Online, 1));
        assert_eq!(
            (danach.liste[0].status, danach.liste[0].seit, danach.stand),
            (Status::Offline, nach_ms(1000), 2)
        );
    }

    #[test]
    fn jetzt_faellt_nie_auch_wenn_die_uhr_zurueckspringt() {
        let (uhr, anwesenheit) = mit_gestellter_uhr();
        uhr.stellen(nach_ms(2000));
        let erst = anwesenheit.jetzt();
        uhr.stellen(nach_ms(1000));
        let dann = anwesenheit.jetzt();
        uhr.stellen(nach_ms(3000));
        let spaeter = anwesenheit.jetzt();

        assert_eq!(
            (erst, dann, spaeter),
            (nach_ms(2000), nach_ms(2000), nach_ms(3000))
        );
    }

    #[test]
    fn stand_mit_ruft_unter_der_sperre() {
        let (_, anwesenheit) = mit_gestellter_uhr();

        let (_, gesperrt) = anwesenheit.stand_mit(|| anwesenheit.bestand.try_lock().is_err());

        assert!(
            gesperrt,
            "abonnieren muss unter der Sperre geschehen (ADR-0016 Regel 5)"
        );
    }

    /// Nacharbeit 2 (Code-Review): jeder Wechsel geht unter der Sperre an die Veröffentlichung — für
    /// beitreten, pflegen und verbunden, deterministisch statt über Last.
    #[test]
    fn beitreten_pflegen_und_verbunden_veroeffentlichen_unter_der_sperre() {
        let uhr = Arc::new(GestellteUhr::neu(zeit(T0)));
        let selbst: Arc<std::sync::OnceLock<std::sync::Weak<Anwesenheit>>> = Arc::default();
        let gesperrt: Arc<Mutex<Vec<(u64, bool)>>> = Arc::default();
        let (selbst_im_aufruf, gesperrt_im_aufruf) = (Arc::clone(&selbst), Arc::clone(&gesperrt));
        let anwesenheit = Arc::new(
            Anwesenheit::neu(
                Arc::clone(&uhr) as Arc<dyn Uhr>,
                Arc::new(FesteFolge::neu(1..=10)),
                Duration::from_secs(60),
            )
            .mit_veroeffentlichen(Arc::new(move |wechsel: &Wechsel| {
                let anwesenheit = selbst_im_aufruf.get().and_then(std::sync::Weak::upgrade);
                let unter_sperre =
                    anwesenheit.is_some_and(|anwesenheit| anwesenheit.bestand.try_lock().is_err());
                gesperrt_im_aufruf
                    .lock()
                    .expect("Liste")
                    .push((wechsel.stand, unter_sperre));
            })),
        );
        selbst
            .set(Arc::downgrade(&anwesenheit))
            .expect("einmal gesetzt");

        let token = anwesenheit
            .beitreten(&wunsch("claude", Art::Agent), ip("10.0.0.5"))
            .expect("beigetreten")
            .sitzung
            .token;
        anwesenheit.pflegen(nach_ms(1000));
        uhr.stellen(nach_ms(2000));
        let (_halter, _) = anwesenheit
            .verbunden(&token, ip("10.0.0.5"))
            .expect("Halter");

        assert_eq!(
            *gesperrt.lock().expect("Liste"),
            vec![(1, true), (2, true), (3, true)],
            "beitreten (online), pflegen (offline), verbunden (online)"
        );
    }

    /// Nacharbeit 3, Code-Review B1: die Pflege beim Abruf dreht keinen Wechsel zurück — auch wenn „jetzt“
    /// des Abrufs vor der Zeit liegt, mit der schon gepflegt wurde (unfairer Mutex, Uhr springt zurück).
    #[test]
    fn ein_abruf_mit_frueherer_zeit_dreht_keinen_wechsel_zurueck() {
        for (uhr_beim_abruf, ueber_stand_mit) in
            [(999, false), (500, false), (1000, false), (999, true)]
        {
            let uhr = Arc::new(GestellteUhr::neu(zeit(T0)));
            let gemeldet = Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let zaehler = Arc::clone(&gemeldet);
            let anwesenheit = Anwesenheit::neu(
                Arc::clone(&uhr) as Arc<dyn Uhr>,
                Arc::new(FesteFolge::neu([1])),
                Duration::from_secs(60),
            )
            .mit_veroeffentlichen(Arc::new(move |_: &Wechsel| {
                zaehler.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }));
            anwesenheit
                .beitreten(&wunsch("alpha", Art::Agent), ip("127.0.0.2"))
                .expect("alpha");

            let gepflegt = anwesenheit.pflegen(nach_ms(1000));
            uhr.stellen(nach_ms(uhr_beim_abruf));
            let stand = if ueber_stand_mit {
                anwesenheit.stand_mit(|| ()).0
            } else {
                anwesenheit.teilnehmer()
            };

            let fall = format!("Uhr {uhr_beim_abruf} ms, stand_mit {ueber_stand_mit}");
            assert_eq!(
                kurz(&gepflegt),
                [(2, "alpha@127.0.0.2".into(), Status::Offline, nach_ms(1000))],
                "{fall}"
            );
            assert_eq!(
                (stand.liste[0].status, stand.liste[0].seit, stand.stand),
                (Status::Offline, nach_ms(1000), 2),
                "{fall}"
            );
            assert_eq!(
                gemeldet.load(std::sync::atomic::Ordering::SeqCst),
                2,
                "{fall}"
            );
        }
    }

    #[test]
    fn gleichzeitige_beitritte_mit_demselben_namen_ergeben_genau_einen_erfolg() {
        let anwesenheit = Arc::new(anwesenheit(FesteFolge::neu(1..=20)));
        let faeden: Vec<_> = (0..20u8)
            .map(|nummer| {
                let anwesenheit = Arc::clone(&anwesenheit);
                std::thread::spawn(move || {
                    let herkunft = IpAddr::V4(Ipv4Addr::new(10, 0, 0, nummer));
                    anwesenheit.beitreten(&wunsch("wettlauf", Art::Agent), herkunft)
                })
            })
            .collect();

        let ergebnisse: Vec<_> = faeden
            .into_iter()
            .map(|faden| faden.join().expect("Faden"))
            .collect();

        assert_eq!(
            ergebnisse
                .iter()
                .filter(|ergebnis| ergebnis.is_ok())
                .count(),
            1
        );
        assert_eq!(anwesenheit.bestand().eintraege.len(), 1);
    }
}
