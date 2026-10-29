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
}

#[cfg(test)]
pub(super) mod tests {
    use super::super::*;
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
}
