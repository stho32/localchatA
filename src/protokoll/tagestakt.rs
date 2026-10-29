//! Tagestakt (ADR-0041 Regel 6 und Ergänzung „Vertrag des Tagestakts“): jede Sekunde die Sekundenaufgaben, bei
//! jedem Wechsel des lokalen Datums die Tagesaufgaben — je Aufgabe ein eigenes `zuletzt`, fällig bei Ungleichheit,
//! `zuletzt` vor dem Aufruf gesetzt, jede für sich über `spawn_blocking`. Eine gescheiterte Tagesaufgabe beendet
//! den Takt nicht; eine gescheiterte Sekundenaufgabe wird abgemeldet.

use super::{Bereich, Kontext, Protokoll, Schluessel, Stufe};
use crate::start::ende::Ende;
use crate::uhr::{Uhr, Zeit};
use chrono::NaiveDate;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::time::Duration;

/// Abstand der Takte.
pub const PERIODE: Duration = Duration::from_secs(1);

/// Arbeit einer Tagesaufgabe mit dem neuen lokalen Datum; läuft auf einem Blocking-Thread.
pub type Tagesarbeit = Arc<dyn Fn(NaiveDate) + Send + Sync>;
/// Arbeit einer Sekundenaufgabe mit der Zeit des Takts; läuft synchron im Takt.
pub type Sekundenarbeit = Box<dyn FnMut(Zeit) + Send>;

struct Tagesaufgabe {
    name: &'static str,
    folge: &'static str,
    zuletzt: NaiveDate,
    arbeit: Tagesarbeit,
}

struct Sekundenaufgabe {
    name: &'static str,
    folge: &'static str,
    arbeit: Sekundenarbeit,
}

/// Fällig (rein): das lokale Datum von `jetzt`, wenn es von `zuletzt` abweicht — auch rückwärts.
pub fn datum_gewechselt(zuletzt: NaiveDate, jetzt: Zeit) -> Option<NaiveDate> {
    let heute = jetzt.date_naive();
    (heute != zuletzt).then_some(heute)
}

/// Der Takt mit seinen angemeldeten Aufgaben in Anmeldereihenfolge.
pub struct Tagestakt {
    uhr: Arc<dyn Uhr>,
    protokoll: Arc<Protokoll>,
    tagesaufgaben: Vec<Tagesaufgabe>,
    sekundenaufgaben: Vec<Sekundenaufgabe>,
}

impl Tagestakt {
    pub fn neu(uhr: Arc<dyn Uhr>, protokoll: Arc<Protokoll>) -> Tagestakt {
        Tagestakt {
            uhr,
            protokoll,
            tagesaufgaben: Vec::new(),
            sekundenaufgaben: Vec::new(),
        }
    }

    /// Meldet eine Tagesaufgabe an; `zuletzt` ist das Datum, das ihr Start-Lauf bekam.
    pub fn tagesaufgabe(
        mut self,
        name: &'static str,
        folge: &'static str,
        zuletzt: NaiveDate,
        arbeit: Tagesarbeit,
    ) -> Tagestakt {
        self.tagesaufgaben.push(Tagesaufgabe {
            name,
            folge,
            zuletzt,
            arbeit,
        });
        self
    }

    /// Meldet eine Sekundenaufgabe an.
    pub fn sekundenaufgabe(
        mut self,
        name: &'static str,
        folge: &'static str,
        arbeit: Sekundenarbeit,
    ) -> Tagestakt {
        self.sekundenaufgaben.push(Sekundenaufgabe {
            name,
            folge,
            arbeit,
        });
        self
    }

    /// Ein Takt: erst die Sekundenaufgaben, dann jede fällige Tagesaufgabe.
    pub async fn takt(&mut self) {
        let jetzt = self.uhr.jetzt();
        self.sekundenaufgaben_ausfuehren(jetzt);
        for aufgabe in &mut self.tagesaufgaben {
            let Some(heute) = datum_gewechselt(aufgabe.zuletzt, jetzt) else {
                continue;
            };
            aufgabe.zuletzt = heute;
            let arbeit = Arc::clone(&aufgabe.arbeit);
            if let Err(fehler) = tokio::task::spawn_blocking(move || arbeit(heute)).await {
                tagesaufgabe_gescheitert(&self.protokoll, aufgabe, &fehler);
            }
        }
    }

    /// Je Sekundenaufgabe ein Aufruf; gerät sie in Panik, wird sie abgemeldet und als ausgefallen gemeldet.
    fn sekundenaufgaben_ausfuehren(&mut self, jetzt: Zeit) {
        let protokoll = &self.protokoll;
        self.sekundenaufgaben.retain_mut(|aufgabe| {
            let gelaufen =
                std::panic::catch_unwind(AssertUnwindSafe(|| (aufgabe.arbeit)(jetzt))).is_ok();
            if !gelaufen {
                protokoll.aufgabe_ausgefallen(aufgabe.name, aufgabe.folge);
            }
            gelaufen
        });
    }

    /// Taktet je `periode`, bis das Ende gemeldet ist.
    pub async fn takten(mut self, mut ende: Ende, periode: Duration) {
        let mut takt = tokio::time::interval(periode);
        takt.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                () = ende.abwarten() => return,
                _ = takt.tick() => self.takt().await,
            }
        }
    }
}

/// Panik (die EXCEPTION mit `ort` schreibt der Haken) → ERROR `grund=panik`; Abbruch ohne Panik → EXCEPTION
/// `grund=abgebrochen` (ADR-0041 Ergänzung, Regel 5). Kein Eintrag in den ausgefallenen.
fn tagesaufgabe_gescheitert(
    protokoll: &Protokoll,
    aufgabe: &Tagesaufgabe,
    fehler: &tokio::task::JoinError,
) {
    let (stufe, grund) = if fehler.is_panic() {
        (Stufe::Error, "panik")
    } else {
        (Stufe::Exception, "abgebrochen")
    };
    protokoll.eintragen(
        stufe,
        Bereich::Programmfehler,
        format!(
            "Tagesaufgabe {} gescheitert — {}; erneuter Versuch beim nächsten Tageswechsel",
            aufgabe.name, aufgabe.folge
        ),
        Kontext::neu()
            .mit(Schluessel::Operation, aufgabe.name)
            .mit(Schluessel::Grund, grund),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protokoll::senke::Speicher;
    use crate::protokoll::testhilfe::{im_speicher, zeit};
    use crate::uhr::GestellteUhr;
    use std::sync::Mutex;

    type Aufrufe = Arc<Mutex<Vec<(&'static str, NaiveDate)>>>;

    fn tag(text: &str) -> NaiveDate {
        NaiveDate::parse_from_str(text, "%Y-%m-%d").expect("Tag")
    }

    fn aufzeichnen(name: &'static str, aufrufe: &Aufrufe) -> Tagesarbeit {
        let aufrufe = Arc::clone(aufrufe);
        Arc::new(move |heute| aufrufe.lock().expect("Aufrufe").push((name, heute)))
    }

    fn panisch() -> Tagesarbeit {
        Arc::new(|_| panic!("Tagesaufgabe A in Panik (Test)"))
    }

    struct Aufbau {
        takt: Tagestakt,
        uhr: Arc<GestellteUhr>,
        speicher: Speicher,
        aufrufe: Aufrufe,
        protokoll: Arc<Protokoll>,
    }

    fn aufbau(start: &str, a: (NaiveDate, Option<Tagesarbeit>), b: NaiveDate) -> Aufbau {
        let (protokoll, speicher, uhr, _) = im_speicher(start, Stufe::Information);
        let aufrufe: Aufrufe = Arc::default();
        let arbeit_a = a.1.unwrap_or_else(|| aufzeichnen("A", &aufrufe));
        let takt = Tagestakt::neu(Arc::clone(&uhr) as Arc<dyn Uhr>, Arc::clone(&protokoll))
            .tagesaufgabe("A", "Folge A", a.0, arbeit_a)
            .tagesaufgabe("B", "Folge B", b, aufzeichnen("B", &aufrufe));
        Aufbau {
            takt,
            uhr,
            speicher,
            aufrufe,
            protokoll,
        }
    }

    impl Aufbau {
        async fn um(&mut self, zeitpunkt: &str, takte: usize) {
            self.uhr.stellen(zeit(zeitpunkt));
            for _ in 0..takte {
                self.takt.takt().await;
            }
        }

        fn aufrufe(&self) -> Vec<(&'static str, NaiveDate)> {
            std::mem::take(&mut *self.aufrufe.lock().expect("Aufrufe"))
        }
    }

    #[tokio::test(start_paused = true)]
    async fn grenze_erst_der_neue_tag_ruft_jede_aufgabe_genau_einmal_in_anmeldereihenfolge() {
        let mut t = aufbau(
            "2026-10-28T12:00:00+01:00",
            (tag("2026-10-28"), None),
            tag("2026-10-28"),
        );

        t.um("2026-10-28T23:59:59.999+01:00", 1).await;
        assert_eq!(t.aufrufe(), []);
        t.um("2026-10-29T00:00:00.000+01:00", 1).await;
        assert_eq!(
            t.aufrufe(),
            [("A", tag("2026-10-29")), ("B", tag("2026-10-29"))]
        );
        t.um("2026-10-29T00:00:01.000+01:00", 10).await;
        assert_eq!(t.aufrufe(), []);
    }

    #[tokio::test(start_paused = true)]
    async fn rueckwaerts_ist_ein_wechsel_und_wieder_vorwaerts_auch() {
        let mut t = aufbau(
            "2026-10-29T00:00:00+01:00",
            (tag("2026-10-29"), None),
            tag("2026-10-29"),
        );

        t.um("2026-10-28T23:00:00.000+01:00", 1).await;
        assert_eq!(
            t.aufrufe(),
            [("A", tag("2026-10-28")), ("B", tag("2026-10-28"))]
        );
        t.um("2026-10-28T23:00:01.000+01:00", 5).await;
        assert_eq!(t.aufrufe(), []);
        t.um("2026-10-29T00:00:00.000+01:00", 1).await;
        assert_eq!(
            t.aufrufe(),
            [("A", tag("2026-10-29")), ("B", tag("2026-10-29"))]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn startwert_jede_aufgabe_beginnt_mit_dem_datum_ihres_start_laufs() {
        let mut t = aufbau(
            "2026-10-29T00:00:00.100+01:00",
            (tag("2026-10-28"), None),
            tag("2026-10-29"),
        );

        t.um("2026-10-29T00:00:00.500+01:00", 1).await;

        assert_eq!(t.aufrufe(), [("A", tag("2026-10-29"))]);
    }

    #[tokio::test(start_paused = true)]
    async fn panik_einer_tagesaufgabe_eine_error_zeile_b_laeuft_naechster_versuch_am_naechsten_tag()
    {
        let mut t = aufbau(
            "2026-10-28T12:00:00+01:00",
            (tag("2026-10-28"), Some(panisch())),
            tag("2026-10-28"),
        );

        t.um("2026-10-29T00:00:00.000+01:00", 1).await;
        assert_eq!(t.aufrufe(), [("B", tag("2026-10-29"))]);
        let zeilen = t.speicher.zeilen();
        assert_eq!(zeilen.len(), 1, "{zeilen:#?}");
        assert!(
            zeilen[0].contains(" ERROR programmfehler Tagesaufgabe A gescheitert — Folge A; erneuter Versuch beim nächsten Tageswechsel | operation=A grund=panik"),
            "{zeilen:#?}"
        );
        t.um("2026-10-29T00:00:01.000+01:00", 10).await;
        assert_eq!(t.speicher.zeilen().len(), 1, "keine Zeile je Sekunde");
        t.um("2026-10-30T00:00:00.000+01:00", 1).await;
        assert_eq!(t.aufrufe(), [("B", tag("2026-10-30"))]);
        assert_eq!(t.speicher.zeilen().len(), 2, "A lief am 30. wieder");
        assert!(t.protokoll.ausgefallen().is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn eine_panische_sekundenaufgabe_wird_abgemeldet_die_tagesaufgaben_laufen_weiter() {
        let mut t = aufbau(
            "2026-10-28T12:00:00+01:00",
            (tag("2026-10-28"), None),
            tag("2026-10-28"),
        );
        let sekunden = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let gezaehlt = Arc::clone(&sekunden);
        t.takt = t.takt.sekundenaufgabe(
            "episoden",
            "Episoden enden nicht mehr",
            Box::new(move |_| {
                let n = gezaehlt.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                assert!(n < 3, "Sekundenaufgabe in Panik (Test)");
            }),
        );

        t.um("2026-10-28T12:00:01+01:00", 5).await;
        t.um("2026-10-29T00:00:00+01:00", 1).await;

        assert_eq!(sekunden.load(std::sync::atomic::Ordering::SeqCst), 3);
        let zeilen = t.speicher.zeilen();
        assert_eq!(zeilen.len(), 1, "{zeilen:#?}");
        assert!(
            zeilen[0].contains(" ERROR programmfehler Hintergrundaufgabe episoden beendet — Episoden enden nicht mehr"),
            "{zeilen:#?}"
        );
        assert_eq!(
            t.aufrufe(),
            [("A", tag("2026-10-29")), ("B", tag("2026-10-29"))]
        );
        assert_eq!(t.protokoll.ausgefallen(), ["episoden"]);
    }

    #[tokio::test(start_paused = true)]
    async fn der_takt_laeuft_je_periode_und_endet_mit_dem_ende() {
        let (protokoll, _, uhr, _) = im_speicher("2026-10-28T12:00:00+01:00", Stufe::Information);
        let sekunden = Arc::new(Mutex::new(0));
        let gezaehlt = Arc::clone(&sekunden);
        let takt = Tagestakt::neu(uhr as Arc<dyn Uhr>, protokoll).sekundenaufgabe(
            "zaehlen",
            "—",
            Box::new(move |_| *gezaehlt.lock().expect("Zähler") += 1),
        );
        let (melden, ende) = Ende::neu();
        let aufgabe = tokio::spawn(takt.takten(ende, PERIODE));

        tokio::time::sleep(Duration::from_millis(3_500)).await;
        melden.melden();
        aufgabe.await.expect("Takt endet");

        assert_eq!(
            *sekunden.lock().expect("Zähler"),
            4,
            "sofort, 1 s, 2 s, 3 s"
        );
    }
}
