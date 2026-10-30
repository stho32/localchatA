//! Der Nachholplatz eines Warte-Aufrufs (ADR-0054 Ergänzung „Nachholen von `warten` …“ Regel 1 und 4 bis 9, B0756,
//! B0758): das Nachholen läuft nur auf der Nachholbank, nie auf der Werkbank; der Aufruf reserviert das Budget am
//! Nachholkonto **vor** Halter und Abonnement, und das erste Nachholen erstattet, was es nicht geprüft hat — im
//! Blocking-Teil selbst, damit ein abgebrochener Client die Abrechnung nicht umgeht. Das Nachholen nach `Lagged`
//! bucht nicht. `live::warten` sieht nur den Port.

use super::Werkbank;
use super::abbruch::Abgebrochen;
use crate::gespraeche::Kennung;
use crate::gespraeche::nachholen::{NACHHOLBUDGET, Nachgeholt};
use crate::live::warten::{Nacharbeit, Nachholplatz};
use crate::protokoll::episoden::{Auftreten, Quelle};
use crate::protokoll::episodentexte;
use crate::protokoll::grenze::Grenzart;
use crate::protokoll::{Bereich, Schluessel, Stufe};
use crate::schreibkonto::nachholkonto::Nachholkonto;
use crate::schreibkonto::{Gebremst, Grenze};
use axum::Json;
use axum::http::StatusCode;
use axum::http::header::RETRY_AFTER;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use std::net::IpAddr;
use std::sync::{Arc, Mutex, PoisonError};

/// K5: ein Aufruf reserviert so viele Sätze, wie er höchstens prüfen darf — das Budget (K1).
pub const RESERVIERUNG: u64 = NACHHOLBUDGET.kosten as u64;

/// Der Platz eines Aufrufs auf der Nachholbank, mit der Reservierung für sein erstes Nachholen.
pub struct Platz {
    bank: Werkbank,
    konto: Arc<Nachholkonto>,
    reservierung: Mutex<Option<Reservierung>>,
}

impl Platz {
    /// Reserviert `RESERVIERUNG` Sätze für `kennung` und `herkunft` (alles oder nichts); reicht ein Eimer nicht,
    /// die Bremse — dann gibt es keinen Platz.
    pub fn reservieren(
        bank: &Werkbank,
        konto: &Arc<Nachholkonto>,
        kennung: &Kennung,
        herkunft: IpAddr,
    ) -> Result<Platz, Gebremst> {
        konto.reservieren(kennung, herkunft, RESERVIERUNG)?;
        Ok(Platz {
            bank: bank.clone(),
            konto: Arc::clone(konto),
            reservierung: Mutex::new(Some(Reservierung {
                konto: Arc::clone(konto),
                kennung: kennung.clone(),
                herkunft,
                offen: RESERVIERUNG,
            })),
        })
    }
}

impl Nachholplatz for Platz {
    type Fehler = Abgebrochen;

    /// Das erste Nachholen rechnet die Reservierung ab; jedes weitere (nach `Lagged`) bucht nicht. Die geprüften Sätze
    /// zählen immer, abgerechnet werden die Kosten.
    fn ausfuehren<F>(&self, arbeit: F) -> Nacharbeit<Abgebrochen>
    where
        F: FnOnce() -> Option<Nachgeholt> + Send + 'static,
    {
        let bank = self.bank.clone();
        let konto = Arc::clone(&self.konto);
        let reservierung = self
            .reservierung
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        Box::pin(async move {
            bank.ausfuehren(move || {
                let nachgeholt = arbeit();
                let zahl = |wert: fn(&Nachgeholt) -> usize| {
                    let wert = nachgeholt.as_ref().map_or(0, wert);
                    u64::try_from(wert).unwrap_or(u64::MAX)
                };
                // Gezählt (Health) wird nur, was in Channels geprüft wurde; bezahlt wird alles (ADR-0055 Regel 9).
                konto.geprueft(
                    zahl(Nachgeholt::channel_saetze),
                    zahl(Nachgeholt::channel_kosten),
                );
                if let Some(reservierung) = reservierung {
                    reservierung.abrechnen(zahl(|nachgeholt| nachgeholt.kosten));
                }
                nachgeholt
            })
            .await
        })
    }
}

/// Reservierte, noch nicht abgerechnete Kosten; fällt sie ohne Abrechnung (Arbeit nie gelaufen, Abbruch vor dem
/// Platz), wird alles erstattet.
struct Reservierung {
    konto: Arc<Nachholkonto>,
    kennung: Kennung,
    herkunft: IpAddr,
    offen: u64,
}

impl Reservierung {
    /// Der Aufruf zahlt, was er geprüft hat; der Rest geht beim Fallen zurück.
    fn abrechnen(mut self, geprueft: u64) {
        self.offen = self.offen.saturating_sub(geprueft);
    }
}

impl Drop for Reservierung {
    fn drop(&mut self) {
        if self.offen > 0 {
            self.konto
                .erstatten(&self.kennung, self.herkunft, self.offen);
        }
    }
}

/// Das Auftreten einer 429 `nachholen`: Quelle ist die Kennung, die Grenze zählt als Art und steht nur in der
/// Beginn-Zeile — Stufe und Form wie die Grenz-Episode des Schreibkontos (Regel 8).
pub(crate) fn gebremst_auftreten(wer: &str, grenze: Grenze) -> Auftreten {
    let grenze = match grenze {
        Grenze::Kennung => Grenzart::Kennung,
        Grenze::Herkunft => Grenzart::Herkunft,
    };
    Auftreten::neu(
        Stufe::Warning,
        Bereich::Nachholen,
        episodentexte::NACHHOLEN_GEBREMST,
        Quelle::neu().mit(Schluessel::Kennung, wer),
    )
    .einzeln(Schluessel::Grenze, grenze)
    .art(grenze.als_text())
}

/// 429 `{fehler, grenze: "nachholen", von, wieder_in_ms}` mit `Retry-After` = ⌈`wieder_in_ms` / 1.000⌉, mindestens
/// 1 (Regel 7); der Text nennt Kennung bzw. Herkunft und sagt, dass `seit` gültig bleibt.
pub(crate) fn gebremst_beantworten(
    gebremst: Gebremst,
    kennung: &Kennung,
    herkunft: IpAddr,
) -> Response {
    let wieder_in_ms = u64::try_from(gebremst.wieder_in.as_millis()).unwrap_or(u64::MAX);
    let sekunden = wieder_in_ms.div_ceil(1_000).max(1);
    let (von, wer) = match gebremst.grenze {
        Grenze::Kennung => ("kennung", kennung.als_text().to_string()),
        Grenze::Herkunft => (
            "herkunft",
            format!("{herkunft} (alle Teilnehmer dieser Adresse zusammen)"),
        ),
    };
    let fehler = format!(
        "Zu viel Nachholen von {wer} — in {sekunden} s mit demselben seit noch einmal rufen; seit bleibt gültig, nichts geht verloren."
    );
    (
        StatusCode::TOO_MANY_REQUESTS,
        [(RETRY_AFTER, sekunden.to_string())],
        Json(json!({
            "fehler": fehler,
            "grenze": "nachholen",
            "von": von,
            "wieder_in_ms": wieder_in_ms,
        })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uhr::{GestellteUhr, Zeit};

    fn konto() -> Arc<Nachholkonto> {
        let jetzt = Zeit::parse_from_rfc3339("2026-10-30T12:00:00+01:00").expect("Zeit");
        Arc::new(Nachholkonto::neu(Arc::new(GestellteUhr::neu(jetzt))))
    }

    fn claude() -> Kennung {
        Kennung::bilden("claude@127.0.0.1").expect("Kennung")
    }

    fn h() -> IpAddr {
        "127.0.0.1".parse().expect("IP")
    }

    fn geprueft(saetze: usize) -> Option<Nachgeholt> {
        Some(nachgeholt(saetze, saetze))
    }

    fn nachgeholt(saetze: usize, kosten: usize) -> Nachgeholt {
        Nachgeholt {
            treffer: Vec::new(),
            stand: Zeit::parse_from_rfc3339("2026-10-30T12:00:00+01:00").expect("Zeit"),
            weitere: false,
            kosten,
            saetze,
            direkt_saetze: 0,
            direkt_kosten: 0,
        }
    }

    /// So viele Reservierungen gelingen `claude` noch.
    fn rest(konto: &Nachholkonto) -> usize {
        (0..60)
            .take_while(|_| konto.reservieren(&claude(), h(), RESERVIERUNG).is_ok())
            .count()
    }

    /// Die Erlaubnis der Bank lebt bis zum Ende der Arbeit (Mutante „Erlaubnis vor dem Ende frei“).
    #[tokio::test]
    async fn der_platz_lebt_bis_zum_ende_der_arbeit() {
        let bank = Werkbank::mit(1);
        let innen = bank.clone();
        let platz = Platz::reservieren(&bank, &konto(), &claude(), h()).expect("Platz");

        let belegt_waehrend = platz
            .ausfuehren(move || {
                assert_eq!(innen.belegt(), 1);
                None
            })
            .await;

        assert!(belegt_waehrend.is_ok());
        assert_eq!(bank.belegt(), 0);
    }

    /// Erstattung: wer 500 prüft, zahlt 500 — danach trägt die Kennung noch 49 volle Reservierungen
    /// (100.000 − 500 = 99.500); die geprüften Sätze zählen.
    #[tokio::test]
    async fn das_erste_nachholen_zahlt_was_es_geprueft_hat() {
        let konto = konto();
        let platz = Platz::reservieren(&Werkbank::mit(1), &konto, &claude(), h()).expect("Platz");

        platz.ausfuehren(|| geprueft(500)).await.expect("gelaufen");

        assert_eq!(rest(&konto), 49);
        assert_eq!(konto.saetze(), 500);
    }

    /// K3 (B0762): 117 Sätze zu 16 KiB kosten 1.989 von 2.000 reservierten Kosten → genau 11 gehen zurück (nicht
    /// 2.000 − 117 = 1.883, wie nach Sätzen); `saetze` zählt 117, `kosten` 1.989.
    #[tokio::test]
    async fn lange_saetze_zahlen_ihre_kosten_und_erstatten_den_rest() {
        let konto = konto();
        konto.reservieren(&claude(), h(), 98_000).expect("Vorrat");
        let platz = Platz::reservieren(&Werkbank::mit(1), &konto, &claude(), h()).expect("Platz");

        platz
            .ausfuehren(|| Some(nachgeholt(117, 1_989)))
            .await
            .expect("gelaufen");

        assert!(konto.reservieren(&claude(), h(), 11).is_ok());
        assert!(konto.reservieren(&claude(), h(), 1).is_err());
        assert_eq!((konto.saetze(), konto.kosten()), (117, 1_989));
    }

    /// ADR-0055 Regel 9 (B0748): das Konto rechnet alle Kosten ab, auch die der Direktsätze; der Health-Check zählt nur
    /// die der Channels (`saetze`, `kosten`). 2.000 Sätze, davon 1.500 direkt: die Reservierung ist ganz bezahlt,
    /// die Zähler stehen auf 500.
    #[tokio::test]
    async fn das_konto_zahlt_direktsaetze_der_health_check_zaehlt_sie_nicht() {
        let konto = konto();
        konto.reservieren(&claude(), h(), 98_000).expect("Vorrat");
        let platz = Platz::reservieren(&Werkbank::mit(1), &konto, &claude(), h()).expect("Platz");

        platz
            .ausfuehren(|| {
                Some(Nachgeholt {
                    direkt_saetze: 1_500,
                    direkt_kosten: 1_500,
                    ..nachgeholt(2_000, 2_000)
                })
            })
            .await
            .expect("gelaufen");

        assert_eq!(rest(&konto), 0, "alle 2.000 Kosten sind bezahlt");
        assert_eq!((konto.saetze(), konto.kosten()), (500, 500));
    }

    /// `Lagged` bucht nicht (Regel 6): bei leerem Konto läuft das zweite Nachholen desselben Platzes trotzdem und
    /// wird nicht abgewiesen; die Sätze beider zählen.
    #[tokio::test]
    async fn das_nachholen_nach_lagged_laeuft_auch_bei_leerem_konto() {
        let konto = konto();
        konto.reservieren(&claude(), h(), 98_000).expect("Vorrat");
        let platz = Platz::reservieren(&Werkbank::mit(1), &konto, &claude(), h()).expect("Platz");
        platz.ausfuehren(|| geprueft(2_000)).await.expect("erstes");

        let zweites = platz.ausfuehren(|| geprueft(2_000)).await;

        assert!(zweites.expect("zweites").is_some());
        assert_eq!(rest(&konto), 0);
        assert_eq!(konto.saetze(), 4_000);
    }

    /// `Lagged` bucht nicht (Regel 6): bleiben nach dem ersten Nachholen 10.000 Sätze, sind es nach dem zweiten
    /// (nach `Lagged`) noch immer 10.000 — fünf volle Reservierungen.
    #[tokio::test]
    async fn das_nachholen_nach_lagged_bucht_nichts() {
        let konto = konto();
        konto.reservieren(&claude(), h(), 88_000).expect("Vorrat");
        let platz = Platz::reservieren(&Werkbank::mit(1), &konto, &claude(), h()).expect("Platz");
        platz.ausfuehren(|| geprueft(2_000)).await.expect("erstes");

        platz.ausfuehren(|| geprueft(2_000)).await.expect("zweites");

        assert_eq!(rest(&konto), 5);
    }

    /// Fällt der Platz, ohne dass die Arbeit lief (Client bricht vor der Bank ab), geht die ganze Reservierung
    /// zurück.
    #[tokio::test]
    async fn ein_platz_ohne_arbeit_erstattet_alles() {
        let konto = konto();
        let platz = Platz::reservieren(&Werkbank::mit(1), &konto, &claude(), h()).expect("Platz");

        drop(platz);

        assert_eq!(rest(&konto), 50);
    }
}
