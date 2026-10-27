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
        let jetzt = self.uhr.jetzt();
        let mut sitzungen = self.lebende_sitzungen(jetzt);
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

    /// Gültiges Token derselben Herkunft → die Sitzung, ihr Lebenszeichen erneuert; sonst keine
    /// (unbekannt, abgelaufen, andere Herkunft — eine fremde Herkunft erneuert nichts).
    #[cfg_attr(not(test), expect(dead_code, reason = "B0054 /api/ich"))]
    pub fn wiederkehren(&self, token: &Token, herkunft: IpAddr) -> Option<Sitzung> {
        let jetzt = self.uhr.jetzt();
        let mut sitzungen = self.lebende_sitzungen(jetzt);
        let sitzung = sitzungen
            .get_mut(token)
            .filter(|sitzung| sitzung.herkunft == herkunft)?;
        sitzung.letztes_lebenszeichen = jetzt;
        Some(sitzung.clone())
    }

    /// Der Bestand unter der Sperre, abgelaufene Sitzungen schon entfernt: ihr Token ist ungültig, ihr Name frei.
    fn lebende_sitzungen(&self, jetzt: Zeit) -> MutexGuard<'_, HashMap<Token, Sitzung>> {
        let mut sitzungen = self.sitzungen();
        sitzungen.retain(|_, sitzung| lebt(sitzung.letztes_lebenszeichen, jetzt, self.gnadenfrist));
        sitzungen
    }

    fn sitzungen(&self) -> MutexGuard<'_, HashMap<Token, Sitzung>> {
        self.sitzungen
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

/// Eine Sitzung lebt, solange ihr letztes Lebenszeichen höchstens die Gnadenfrist zurückliegt (ADR-0012).
/// Geprüft wird beim Zugriff; einen Takt, der „weg“ meldet, gibt es erst mit I0006.
pub fn lebt(letztes_lebenszeichen: Zeit, jetzt: Zeit, gnadenfrist: Duration) -> bool {
    let frist = chrono::Duration::from_std(gnadenfrist).unwrap_or(chrono::Duration::MAX);
    jetzt - letztes_lebenszeichen <= frist
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
            nach_61.map(|sitzung| sitzung.kennung.to_string()),
            Ok("claude@127.0.0.1".into())
        );
        assert_eq!(
            anwesenheit.sitzungen().len(),
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
            .expect("beigetreten");

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
            .expect("beigetreten");

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
