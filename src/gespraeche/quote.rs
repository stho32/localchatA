//! Plattenquote der Gespräche (ADR-0044): die Grenze in Bytes und der Zähler `belegt`, beide hält Gespräche
//! (A00006 Ergänzung Karte #34).

use std::sync::atomic::{AtomicU64, Ordering};

/// Standard 4 GiB (ADR-0044 Regel 3): Normalwoche ≈ 1,6 GB und Bestand des Lastprofils ≈ 2,65 GB liegen unter der
/// Warnschwelle von 3,6 GiB; eine flutende Herkunft braucht vom Normalstand ≈ 11 h bis hierher. Gerechnet, nicht
/// gemessen — anheben ist ein Commit.
pub const STANDARD_BYTES: u64 = 4 * 1024 * 1024 * 1024;

/// Zähler der Bytes, die die Gespräche belegen (ADR-0044 Regel 2): Σ Rohtext aller Tage im Verlaufsindex.
#[derive(Debug, Default)]
pub(super) struct Belegung {
    belegt: AtomicU64,
}

impl Belegung {
    pub(super) fn neu(belegt: u64) -> Belegung {
        Belegung {
            belegt: AtomicU64::new(belegt),
        }
    }

    pub(super) fn stand(&self) -> u64 {
        self.belegt.load(Ordering::SeqCst)
    }

    /// Ob `bytes` mehr noch unter `hoechstens` passen (die Grenze gehört dazu).
    pub(super) fn pruefen(&self, bytes: u64, hoechstens: u64) -> Result<(), Ueber> {
        let belegt = self.stand();
        if belegt
            .checked_add(bytes)
            .is_none_or(|danach| danach > hoechstens)
        {
            return Err(Ueber { belegt, hoechstens });
        }
        Ok(())
    }

    /// Erhöht um `bytes`; liefert den Stand davor. Gerufen nur unter der Schreibsperre, also überholen sich zwei
    /// Sendungen nie über die Quote (ADR-0044 Regel 2).
    pub(super) fn dazu(&self, bytes: u64) -> u64 {
        self.belegt.fetch_add(bytes, Ordering::SeqCst)
    }

    /// Vermindert um `bytes` eines entfallenen Tages, ohne Schreibsperre; liefert den Stand davor. Ein gleichzeitiges
    /// Abziehen macht eine laufende Prüfung höchstens strenger (ADR-0044 Regel 2).
    pub(super) fn weg(&self, bytes: u64) -> u64 {
        self.belegt
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |belegt| {
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
    use super::{Belegung, Ueber};
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
}
