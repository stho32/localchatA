//! Namen belegen (B0048): der Bestand aller Sitzungen hinter einer Sperre — Prüfen und Belegen eines Namens
//! geschehen unter derselben Sperre, gleichzeitige Beitritte mit demselben Namen ergeben genau einen Erfolg.

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
    sitzungen: Mutex<HashMap<Token, Sitzung>>,
}

impl Anwesenheit {
    pub fn neu(uhr: Arc<dyn Uhr>, zufall: Arc<dyn Zufall>, gnadenfrist: Duration) -> Anwesenheit {
        Anwesenheit {
            uhr,
            zufall,
            gnadenfrist,
            sitzungen: Mutex::new(HashMap::new()),
        }
    }

    /// So lange lebt eine Sitzung nach ihrem letzten Lebenszeichen (ADR-0012).
    pub fn gnadenfrist(&self) -> Duration {
        self.gnadenfrist
    }

    /// Unter einer Sperre: Name lebend belegt (egal von wo) → vergeben mit Vorschlag; Grenze je Herkunft
    /// oder gesamt erreicht → zu viele; sonst eine neue Sitzung mit frischem Token.
    pub fn beitreten(
        &self,
        wunsch: &Beitrittswunsch,
        herkunft: IpAddr,
    ) -> Result<Sitzung, Beitrittsfehler> {
        let mut sitzungen = self.sitzungen();
        let jetzt = self.uhr.jetzt();
        let traeger = sitzungen
            .values()
            .find(|sitzung| sitzung.name == wunsch.name);
        if let Some(traeger) = traeger {
            let belegt = |name: &Name| sitzungen.values().any(|sitzung| &sitzung.name == name);
            return Err(Beitrittsfehler::NameVergeben {
                belegt_von: traeger.kennung.clone(),
                seit: traeger.seit,
                vorschlag: freier_name(&wunsch.name, belegt),
            });
        }
        let von_dieser_herkunft = sitzungen
            .values()
            .filter(|sitzung| sitzung.herkunft == herkunft)
            .count();
        if von_dieser_herkunft >= JE_HERKUNFT || sitzungen.len() >= GESAMT {
            return Err(Beitrittsfehler::ZuVieleSitzungen);
        }
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
        sitzungen.insert(token, sitzung.clone());
        Ok(sitzung)
    }

    fn sitzungen(&self) -> MutexGuard<'_, HashMap<Token, Sitzung>> {
        self.sitzungen
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
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
            .expect("beigetreten");

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
        assert_eq!(anwesenheit.sitzungen().len(), GESAMT);
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
        assert!(anwesenheit.sitzungen().is_empty());
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
        assert_eq!(anwesenheit.sitzungen().len(), 1);
    }
}
