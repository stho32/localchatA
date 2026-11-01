//! Plattenquote der Gespräche (ADR-0044): die Grenze in Bytes und der Zähler `belegt`, beide hält Gespräche
//! (A00006 Ergänzung Karte #34).

use super::pflege::Pflegeanlass;
use crate::protokoll::{Bereich, Eintrag, Kontext, Schluessel, Stufe};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Standard 4 GiB (ADR-0044 Regel 3): Normalwoche ≈ 1,6 GB und Bestand des Lastprofils ≈ 2,65 GB liegen unter der
/// Warnschwelle von 3,6 GiB; eine flutende Herkunft braucht vom Normalstand ≈ 11 h bis hierher. Gerechnet, nicht
/// gemessen — anheben ist ein Commit.
pub const STANDARD_BYTES: u64 = 4 * 1024 * 1024 * 1024;

/// Richtung eines Wechsels über die Warnschwelle von 90 % (ADR-0044 Regel 7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Richtung {
    NachOben,
    NachUnten,
}

/// Mindestreserve des Löschens: 1 MiB ≈ 7.000 Löschsätze.
const MINDESTRESERVE: u64 = 1_048_576;

/// Reserve, um die Löschen die Quote überschreiten darf (ADR-0048 Regel 10, G3): `max(1 MiB, ⌈hoechstens / 100⌉)`
/// — an der vollen Quote lässt sich eine Nachricht noch verbergen, die Platte bleibt begrenzt. Die Warnschwelle
/// rechnet ohne sie.
pub fn reserve(hoechstens: u64) -> u64 {
    hoechstens.div_ceil(100).max(MINDESTRESERVE)
}

/// Ob `belegt` über 90 % von `hoechstens` liegt: `belegt × 10 > hoechstens × 9`, ganzzahlig.
pub fn ueber_90(belegt: u64, hoechstens: u64) -> bool {
    u128::from(belegt) * 10 > u128::from(hoechstens) * 9
}

/// Der Wechsel über die Warnschwelle zwischen zwei Ständen, sonst keiner — ein Zustandswechsel, keine Episode.
pub fn wechsel(vorher: u64, nachher: u64, hoechstens: u64) -> Option<Richtung> {
    match (ueber_90(vorher, hoechstens), ueber_90(nachher, hoechstens)) {
        (false, true) => Some(Richtung::NachOben),
        (true, false) => Some(Richtung::NachUnten),
        _ => None,
    }
}

/// Was der Betreiber über die Quote lesen soll (ADR-0044 Regel 7): über 90 % beim Senden oder Start, über der Quote
/// beim Start, wieder unter 90 % beim Entfallen mit dem Anlass der Pflege.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Quotemeldung {
    Ueber90 {
        belegt: u64,
        hoechstens: u64,
    },
    UeberQuote {
        belegt: u64,
        hoechstens: u64,
    },
    Unter90 {
        belegt: u64,
        hoechstens: u64,
        anlass: Pflegeanlass,
    },
}

/// Nimmt Meldungen der Quote entgegen (im Start: das Betriebsprotokoll); gerufen auch unter der Schreibsperre.
pub type QuoteMelden = Arc<dyn Fn(&Quotemeldung) + Send + Sync>;

/// Die Meldung als Eintrag im Bereich `quote` (ADR-0042 Ergänzung Karte #34). Rein.
pub fn meldung_eintrag(meldung: &Quotemeldung) -> Eintrag {
    let stand = |belegt: u64, hoechstens: u64| {
        Kontext::neu()
            .mit(Schluessel::Belegt, belegt)
            .mit(Schluessel::Hoechstens, hoechstens)
    };
    match meldung {
        Quotemeldung::Ueber90 { belegt, hoechstens } => Eintrag::neu(
            Stufe::Warning,
            Bereich::Quote,
            "Gespräche belegen 90 % der Quote",
            stand(*belegt, *hoechstens),
        ),
        Quotemeldung::UeberQuote { belegt, hoechstens } => Eintrag::neu(
            Stufe::Warning,
            Bereich::Quote,
            "Gespräche belegen mehr als die Quote — Senden wird abgewiesen",
            stand(*belegt, *hoechstens),
        ),
        Quotemeldung::Unter90 {
            belegt,
            hoechstens,
            anlass,
        } => Eintrag::neu(
            Stufe::Information,
            Bereich::Quote,
            "Gespräche wieder unter 90 % der Quote",
            stand(*belegt, *hoechstens).mit(Schluessel::Grund, grund(anlass)),
        ),
    }
}

fn grund(anlass: &Pflegeanlass) -> &'static str {
    match anlass {
        Pflegeanlass::Start => "start",
        Pflegeanlass::Tageswechsel => "tageswechsel",
        Pflegeanlass::FristGesetzt { .. } => "frist_gesetzt",
    }
}

/// Der Stand beim Start (ADR-0044 Regel 7): über der Quote, sonst über 90 %, sonst nichts.
pub fn beim_start(belegt: u64, hoechstens: u64) -> Option<Quotemeldung> {
    if belegt > hoechstens {
        Some(Quotemeldung::UeberQuote { belegt, hoechstens })
    } else if ueber_90(belegt, hoechstens) {
        Some(Quotemeldung::Ueber90 { belegt, hoechstens })
    } else {
        None
    }
}

/// Zähler der Bytes, die die Gespräche belegen (ADR-0044 Regel 2): Σ Rohtext aller Tage im Verlaufsindex.
#[derive(Debug, Default)]
pub(super) struct Belegung {
    belegt: AtomicU64,
    /// Zahl der Abweisungen an der Quote seit dem Start (ADR-0044 Regel 9).
    abgewiesen: AtomicU64,
}

impl Belegung {
    pub(super) fn neu(belegt: u64) -> Belegung {
        Belegung {
            belegt: AtomicU64::new(belegt),
            abgewiesen: AtomicU64::new(0),
        }
    }

    pub(super) fn stand(&self) -> u64 {
        self.belegt.load(Ordering::SeqCst)
    }

    /// Ob `bytes` mehr noch unter `hoechstens` passen (die Grenze gehört dazu); eine Abweisung wird gezählt.
    pub(super) fn pruefen(&self, bytes: u64, hoechstens: u64) -> Result<(), Ueber> {
        let belegt = self.stand();
        if belegt
            .checked_add(bytes)
            .is_none_or(|danach| danach > hoechstens)
        {
            self.abgewiesen.fetch_add(1, Ordering::SeqCst);
            return Err(Ueber { belegt, hoechstens });
        }
        Ok(())
    }

    /// Wie `pruefen`, mit der Grenze `hoechstens + reserve(hoechstens)` (Löschen, ADR-0048 Regel 10); die Abweisung
    /// nennt das echte `hoechstens`.
    pub(super) fn pruefen_mit_reserve(&self, bytes: u64, hoechstens: u64) -> Result<(), Ueber> {
        let grenze = hoechstens.saturating_add(reserve(hoechstens));
        self.pruefen(bytes, grenze).map_err(|ueber| Ueber {
            hoechstens,
            ..ueber
        })
    }

    /// Erhöht um `bytes`; liefert den Stand davor. Gerufen nur unter der Schreibsperre, also überholen sich zwei
    /// Sendungen nie über die Quote (ADR-0044 Regel 2).
    pub(super) fn dazu(&self, bytes: u64) -> u64 {
        self.belegt.fetch_add(bytes, Ordering::SeqCst)
    }

    pub(super) fn abgewiesen(&self) -> u64 {
        self.abgewiesen.load(Ordering::SeqCst)
    }

    /// Vermindert um `bytes` eines entfallenen Tages, ohne Schreibsperre; liefert den Stand davor. Ein gleichzeitiges
    /// Abziehen macht eine laufende Prüfung höchstens strenger (ADR-0044 Regel 2).
    pub(super) fn weg(&self, bytes: u64) -> u64 {
        self.belegt
            .try_update(Ordering::SeqCst, Ordering::SeqCst, |belegt| {
                Some(belegt.saturating_sub(bytes))
            })
            .unwrap_or_else(|belegt| belegt)
    }
}

/// Der neue Satz brächte die Gespräche über die Quote.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Ueber {
    pub belegt: u64,
    pub hoechstens: u64,
}

#[cfg(test)]
pub(super) mod tests {
    use super::super::*;
    use super::{Belegung, Ueber, reserve, ueber_90};
    use crate::uhr::GestellteUhr;

    pub const JETZT: &str = "2026-10-28T12:00:00.000+01:00";

    pub fn tag(text: &str) -> NaiveDate {
        NaiveDate::parse_from_str(text, "%Y-%m-%d").expect("Testtag")
    }

    /// Ein gültiger Satz von genau `bytes` Bytes (Kopf, eine eingerückte Zeile aus `x`, Leerzeile).
    pub fn satz_mit(zeit: &str, kennung: &str, bytes: usize) -> String {
        let kopf = format!("@ {zeit} {kennung}\n");
        let text = "x".repeat(bytes - kopf.len() - 4);
        format!("{kopf}  {text}\n\n")
    }

    /// Σ Bytes aller Tage im Index — die Invariante, gegen die der Zähler steht (ADR-0044 Regel 2).
    pub fn summe_im_index(gespraeche: &Gespraeche) -> u64 {
        lesend(&gespraeche.verzeichnis)
            .verlaeufe
            .values()
            .map(|verlauf| sperren(verlauf).bytes())
            .sum()
    }

    pub fn laden(ablage: &Arc<Speicherablage>) -> Gespraeche {
        let uhr = Arc::new(GestellteUhr::neu(
            Zeit::parse_from_rfc3339(JETZT).expect("Zeit"),
        ));
        Gespraeche::laden(ablage.clone(), uhr).expect("geladen")
    }

    /// B0355 (ADR-0044 Regel 2 „Laden“): `belegt` = Σ der Rohtext-Bytes der geladenen Tage, Tage außerhalb der
    /// Frist zählen nicht; `log_bytes` ist dieselbe Zahl.
    #[test]
    fn belegt_ist_nach_dem_laden_die_summe_der_geladenen_tage() {
        let ablage = Arc::new(Speicherablage::neu());
        let allgemein = Gespraech::channel("allgemein");
        let technik = Gespraech::channel("technik");
        ablage.gespraech_anlegen(&allgemein);
        ablage.gespraech_anlegen(&technik);
        ablage.tag_setzen(
            &allgemein,
            tag("2026-10-27"),
            &satz_mit("2026-10-27T09:00:00.000+01:00", "a@127.0.0.9", 900),
        );
        ablage.tag_setzen(
            &allgemein,
            tag("2026-10-01"),
            &satz_mit("2026-10-01T09:00:00.000+02:00", "a@127.0.0.9", 500),
        );
        let zwei = format!(
            "{}{}",
            satz_mit("2026-10-28T09:00:00.000+01:00", "a@127.0.0.9", 120),
            satz_mit("2026-10-28T09:01:00.000+01:00", "a@127.0.0.9", 180)
        );
        ablage.tag_setzen(&technik, tag("2026-10-28"), &zwei);

        let gespraeche = laden(&ablage);

        assert_eq!(gespraeche.belegt(), 1_200);
        assert_eq!(summe_im_index(&gespraeche), 1_200);
        assert_eq!(gespraeche.kennzahlen().log_bytes, 1_200);
    }

    /// R00040 B0614 (ADR-0048 Regel 10, G3): R = max(1 MiB, ⌈hoechstens / 100⌉) — 4 GiB → 42.949.673, 10 MiB →
    /// 1.048.576, 200 MiB → 2.097.152, 0 → 1.048.576.
    #[test]
    fn die_reserve_des_loeschens_ist_ein_prozent_mindestens_ein_mib() {
        assert_eq!(reserve(4 * 1024 * 1024 * 1024), 42_949_673);
        assert_eq!(reserve(10 * 1024 * 1024), 1_048_576);
        assert_eq!(reserve(200 * 1024 * 1024), 2_097_152);
        assert_eq!(reserve(0), 1_048_576);
        assert_eq!(reserve(u64::MAX), u64::MAX.div_ceil(100));
    }

    /// R00040 „Reserve an der Plattenquote“ am Zähler: `hoechstens` 1.048.576, also Grenze 2.097.152; L Bytes des
    /// Löschsatzes. `belegt` = 2.097.152 − L → passt, danach 2.097.152; das nächste L → abgewiesen mit dem echten
    /// `hoechstens` (für den Text der 507) und gezählt; die Warnschwelle rechnet ohne Reserve.
    #[test]
    fn loeschen_darf_bis_zur_reserve_ueber_die_quote() {
        const L: u64 = 97;
        let belegung = Belegung::neu(2_097_152 - L);

        assert_eq!(belegung.pruefen_mit_reserve(L, 1_048_576), Ok(()));
        belegung.dazu(L);
        assert_eq!(
            belegung.pruefen_mit_reserve(L, 1_048_576),
            Err(Ueber {
                belegt: 2_097_152,
                hoechstens: 1_048_576
            })
        );
        assert_eq!(belegung.abgewiesen(), 1);
        assert!(belegung.pruefen(1, 1_048_576).is_err());
        assert!(
            Belegung::neu(u64::MAX)
                .pruefen_mit_reserve(1, u64::MAX)
                .is_err()
        );
        assert!(ueber_90(943_719, 1_048_576) && !ueber_90(943_718, 1_048_576));
    }

    /// Abnahme-Probe B0356, Zahlen wörtlich am Zähler (ADR-0044 „Folgeentscheidungen“, erster Unit-Test): Quote
    /// 1.000, geladen 900; 80 → ok, 980; 21 → `Quote { belegt: 980, hoechstens: 1.000 }`; genau 20 → ok, 1.000.
    #[test]
    fn die_grenze_gehoert_dazu_darueber_wird_abgewiesen() {
        let belegung = Belegung::neu(900);

        assert_eq!(belegung.pruefen(80, 1_000), Ok(()));
        belegung.dazu(80);
        assert_eq!(belegung.stand(), 980);
        assert_eq!(
            belegung.pruefen(21, 1_000),
            Err(Ueber {
                belegt: 980,
                hoechstens: 1_000
            })
        );
        assert_eq!(belegung.stand(), 980);
        assert_eq!(belegung.pruefen(20, 1_000), Ok(()));
        belegung.dazu(20);
        assert_eq!(belegung.stand(), 1_000);
        assert!(belegung.pruefen(1, 1_000).is_err());
        assert!(Belegung::neu(u64::MAX).pruefen(1, u64::MAX).is_err());
    }

    fn beta() -> Kennung {
        Kennung::bilden("beta@127.0.0.1").expect("Kennung")
    }

    /// Ein Satz von `beta` (Agent) um JETZT hat 64 Bytes plus Text: Kopf 47, `+ art: agent` 13, Einrückung und
    /// Abschluss 4.
    fn text_fuer(satzbytes: usize) -> String {
        "y".repeat(satzbytes - 64)
    }

    /// Quote 1.000 mit gestern 740 Bytes, `rueckrufe` zählt veröffentlichte Nachrichten.
    fn an_der_quote() -> (Gespraeche, Arc<Speicherablage>, Arc<Mutex<usize>>) {
        let ablage = Arc::new(Speicherablage::neu());
        let allgemein = Gespraech::channel("allgemein");
        ablage.gespraech_anlegen(&allgemein);
        ablage.tag_setzen(
            &allgemein,
            tag("2026-10-27"),
            &satz_mit("2026-10-27T09:00:00.000+01:00", "a@127.0.0.9", 740),
        );
        let rueckrufe: Arc<Mutex<usize>> = Arc::default();
        let gezaehlt = Arc::clone(&rueckrufe);
        let gespraeche = laden(&ablage)
            .mit_quote(Some(1_000))
            .mit_nachricht_veroeffentlichen(Arc::new(move |_: &Gespraech, _: &Nachricht| {
                *sperren(&gezaehlt) += 1;
            }));
        (gespraeche, ablage, rueckrufe)
    }

    /// B0356 im Gespräch (Sätze ab 65 Bytes, darum mit 740 statt 900 geladen): 80 → ok, 820 = Σ Index; 181 →
    /// `Quote { 820, 1.000 }`, Ablage, Index und Rückrufe unverändert; genau 180 → ok, 1.000; Wiederholung mit
    /// bekanntem Schlüssel bei voller Quote → `Wiederholt`. Mutanten `>=` statt `>` und „Prüfung nach dem
    /// Anhängen“ werden rot.
    #[test]
    fn senden_prueft_die_quote_vor_dem_anhaengen() {
        let (gespraeche, ablage, rueckrufe) = an_der_quote();
        let allgemein = Gespraech::channel("allgemein");
        let heute = tag("2026-10-28");

        let erste = gespraeche.senden_einmal(&allgemein, &beta(), Art::Agent, &text_fuer(80), "k1");
        assert!(matches!(erste, Ok(Gesendet::Neu(_))), "{erste:?}");
        assert_eq!(gespraeche.belegt(), 820);
        assert_eq!(summe_im_index(&gespraeche), 820);

        let vorher = ablage.inhalt(&allgemein, heute);
        let zu_gross = gespraeche.senden(&allgemein, &beta(), Art::Agent, &text_fuer(181));
        assert_eq!(
            zu_gross,
            Err(Sendefehler::Quote {
                belegt: 820,
                hoechstens: 1_000
            })
        );
        assert_eq!(ablage.inhalt(&allgemein, heute), vorher);
        assert_eq!(gespraeche.belegt(), 820);
        assert_eq!(summe_im_index(&gespraeche), 820);
        assert_eq!(*sperren(&rueckrufe), 1);
        assert_eq!(gespraeche.kennzahlen().quote_abgewiesen, 1);

        let genau = gespraeche.senden(&allgemein, &beta(), Art::Agent, &text_fuer(180));
        assert!(genau.is_ok(), "{genau:?}");
        assert_eq!(gespraeche.belegt(), 1_000);
        assert_eq!(summe_im_index(&gespraeche), 1_000);

        let wiederholt =
            gespraeche.senden_einmal(&allgemein, &beta(), Art::Agent, &text_fuer(80), "k1");
        assert!(
            matches!(wiederholt, Ok(Gesendet::Wiederholt(_))),
            "{wiederholt:?}"
        );
        assert_eq!(gespraeche.belegt(), 1_000);
        assert_eq!(*sperren(&rueckrufe), 2);
    }

    /// Ergänzt die Logablage ein fehlendes Zeilenende, zählt dieses Byte mit (ADR-0044 Regel 2).
    #[test]
    fn ein_ergaenztes_zeilenende_zaehlt_mit() {
        let ablage = Arc::new(Speicherablage::neu());
        let allgemein = Gespraech::channel("allgemein");
        ablage.gespraech_anlegen(&allgemein);
        let ohne_ende = satz_mit("2026-10-28T09:00:00.000+01:00", "a@127.0.0.9", 100);
        ablage.tag_setzen(&allgemein, tag("2026-10-28"), &ohne_ende[..98]);
        let gespraeche = laden(&ablage);
        assert_eq!(gespraeche.belegt(), 98);

        gespraeche
            .senden(&allgemein, &beta(), Art::Agent, &text_fuer(80))
            .expect("gesendet");

        assert_eq!(gespraeche.belegt(), 98 + 1 + 80);
        assert_eq!(summe_im_index(&gespraeche), 98 + 1 + 80);
    }

    /// F0075 „Ohne Quote“: mit `aus` wird über jede Grenze angehängt; die Mutante „`aus` heißt 0“ wird rot.
    #[test]
    fn ohne_quote_wird_nie_abgewiesen() {
        let (gespraeche, _, _) = an_der_quote();
        let gespraeche = gespraeche.mit_quote(None);

        for _ in 0..4 {
            gespraeche
                .senden(
                    &Gespraech::channel("allgemein"),
                    &beta(),
                    Art::Agent,
                    &text_fuer(181),
                )
                .expect("ohne Quote gesendet");
        }

        assert_eq!(gespraeche.belegt(), 740 + 4 * 181);
        assert_eq!(summe_im_index(&gespraeche), 740 + 4 * 181);
    }

    /// B0357 (Ende der Probe aus B0356): Entfallen nimmt den 740-Byte-Tag → `belegt` = nur noch heute, = Σ Index;
    /// danach passt wieder, was vorher abgewiesen wurde. Mutante „Entfallen zieht nicht ab“ wird rot.
    #[test]
    fn entfallen_zieht_die_bytes_des_tages_ab() {
        let (gespraeche, _, _) = an_der_quote();
        let allgemein = Gespraech::channel("allgemein");
        gespraeche
            .senden(&allgemein, &beta(), Art::Agent, &text_fuer(80))
            .expect("gesendet");
        assert!(
            gespraeche
                .senden(&allgemein, &beta(), Art::Agent, &text_fuer(181))
                .is_err()
        );

        gespraeche
            .frist_setzen("allgemein", 1, true, &beta())
            .expect("gesetzt");

        assert_eq!(gespraeche.belegt(), 80);
        assert_eq!(summe_im_index(&gespraeche), 80);
        assert_eq!(gespraeche.kennzahlen().log_bytes, 80);
        gespraeche
            .senden(&allgemein, &beta(), Art::Agent, &text_fuer(181))
            .expect("wieder Platz");
        assert_eq!(gespraeche.belegt(), 261);

        gespraeche.pflegen(tag("2026-10-30"), Pflegeanlass::Tageswechsel);
        assert_eq!(gespraeche.belegt(), 0);
        assert_eq!(summe_im_index(&gespraeche), 0);
    }

    /// F0075 „Löschen scheitert“: der Tag bleibt im Index und im Zähler. Mutante „zieht vor dem Entfernen ab“ wird
    /// rot.
    #[test]
    fn ein_tag_der_nicht_entfaellt_zaehlt_weiter() {
        let (gespraeche, ablage, _) = an_der_quote();
        let allgemein = Gespraech::channel("allgemein");
        ablage.entfernen_scheitern(&allgemein, tag("2026-10-27"));

        gespraeche
            .frist_setzen("allgemein", 1, true, &beta())
            .expect("gesetzt");

        assert_eq!(gespraeche.belegt(), 740);
        assert_eq!(summe_im_index(&gespraeche), 740);
    }

    /// B0361 (R00023 F0076 „Schwelle als Zustandswechsel“, Quote 1.000). Mutanten „`>=` statt `>`“ und „Wechsel
    /// bei jedem Senden über 90 %“ werden rot.
    #[test]
    fn die_schwelle_meldet_nur_den_wechsel() {
        use super::{Richtung, wechsel};
        for (vorher, nachher, erwartet) in [
            (880, 900, None),
            (900, 901, Some(Richtung::NachOben)),
            (901, 980, None),
            (980, 901, None),
            (901, 900, Some(Richtung::NachUnten)),
            (950, 100, Some(Richtung::NachUnten)),
            (100, 950, Some(Richtung::NachOben)),
            (100, 100, None),
        ] {
            assert_eq!(
                wechsel(vorher, nachher, 1_000),
                erwartet,
                "{vorher} → {nachher}"
            );
        }
    }

    /// Rechenbeispiel Standard: 4 GiB, Warnschwelle ab 3.865.470.567 Bytes; ohne Überlauf auch an der Obergrenze.
    #[test]
    fn die_schwelle_der_standardquote_liegt_bei_3_6_gib() {
        use super::{STANDARD_BYTES, ueber_90};
        assert!(!ueber_90(3_865_470_566, STANDARD_BYTES));
        assert!(ueber_90(3_865_470_567, STANDARD_BYTES));
        assert!(ueber_90(u64::MAX, u64::MAX));
    }

    /// Jeder Anlass trägt seinen `grund`; die Mutante „grund immer start“ wird rot.
    #[test]
    fn die_meldung_unter_90_nennt_den_anlass() {
        use super::{Quotemeldung, meldung_eintrag};
        for (anlass, grund) in [
            (Pflegeanlass::Start, "start"),
            (Pflegeanlass::Tageswechsel, "tageswechsel"),
            (Pflegeanlass::FristGesetzt { von: beta() }, "frist_gesetzt"),
        ] {
            let eintrag = meldung_eintrag(&Quotemeldung::Unter90 {
                belegt: 72,
                hoechstens: 1_000,
                anlass,
            });
            assert_eq!(
                eintrag.kontext.wert(crate::protokoll::Schluessel::Grund),
                Some(grund)
            );
        }
    }
}
