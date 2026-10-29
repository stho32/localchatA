//! Health-Check: `GET /api/gesundheit` meldet Bereitschaft und Kennzahlen des Ordners (ADR-0009).

use super::Zustand;
use super::teilnehmerliste::zeit_als_text;
use crate::protokoll::zaehler::AbgewiesenStand;
use crate::start::abrisspflege::pflege_aus;
use axum::Json;
use axum::extract::State;
use serde::Serialize;
use std::sync::Arc;

#[derive(Debug, Serialize)]
pub struct Gesundheit {
    status: &'static str,
    version: &'static str,
    ordner: String,
    nachrichten: u64,
    log_bytes: u64,
    ladezeit_ms: u64,
    defekte_zeilen: u64,
    channels: u64,
    /// Vorlauf der Nachrichtenzeit vor der Uhr (ADR-0035 Regel 4), 0 ohne.
    zeit_vorlauf_ms: u64,
    /// Zahl der 429 des Schreibkontos seit dem Start (ADR-0036 Regel 7).
    gebremst: u64,
    sitzungen: usize,
    halter: usize,
    herzschlag_ms: u128,
    abrissgrenze_ms: u128,
    protokoll_stufe: &'static str,
    protokoll_ok: bool,
    protokoll_bytes_heute: u64,
    protokoll_verworfen: u64,
    warnings: u64,
    errors: u64,
    exceptions: u64,
    letzter_error: Option<String>,
    letzte_exception: Option<String>,
    aufgaben_ausgefallen: Vec<&'static str>,
    abgewiesen: AbgewiesenStand,
    /// Abrisse an der Abrissgrenze seit dem Start (ADR-0043 Ergänzung #24).
    abgerissen: u64,
    /// Plätze der Werkbank, fest seit dem Start.
    werkbank_plaetze: usize,
    /// Belegte Plätze der Werkbank, Momentwert.
    werkbank_belegt: usize,
    /// `--alle-namen` an (ADR-0045 Regel 9).
    alle_namen: bool,
    /// Wirksame Plattenquote in Bytes, `null` bei `aus` (ADR-0044 Regel 9).
    plattenquote: Option<u64>,
    /// Zahl der 507 wegen der Quote seit dem Start (ADR-0044 Regel 9).
    quote_abgewiesen: u64,
    /// Wiederholschlüssel, die seit dem Start vor Ablauf verdrängt wurden (ADR-0043 Ergänzung Karte #36).
    schluessel_verdraengt: u64,
    /// Anhang-Quote in Bytes, `null` bei `aus` (ADR-0043 Ergänzungen „F1“, „F1′“).
    anhangquote: Option<u64>,
    /// `G + A`: gebundene Anhänge samt Waisen plus Ansprüche offener Hochladungen.
    anhang_belegt: u64,
    /// `A`: Ansprüche offener Hochladungen im Zählmaß.
    reserviert: u64,
    /// Offene Hochladungen (angemeldet, laufend, fertig, nicht gebunden).
    hochladungen: u64,
    /// Belegte Erlaubnisse der Hochladebank, Momentwert.
    hochladebank_belegt: usize,
    /// 507 `quote` an Anmeldungen seit dem Start, beide Gründe.
    anhang_abgewiesen: u64,
}

/// Antwortet der Port, ist die Instanz bereit: geladen wird vor dem Binden (A00003).
/// Die Kennzahlen kommen aus Laden und Verlaufsindex (R00002), dem Channelverzeichnis (Anzahl, ADR-0021) und der Anwesenheit: lebende Sitzungen und
/// offene Ströme (R00004) — Zahlen ohne Namen, ohne Sitzung lesbar; dazu die Programmversion, mit der die
/// Oberfläche ihren Worker anlegt (ADR-0015 Regel 3), und die wirksamen Werte von Herzschlag und Abrissgrenze
/// (ADR-0017, B0103).
pub async fn beantworten(State(zustand): State<Arc<Zustand>>) -> Json<Gesundheit> {
    let kennzahlen = zustand.gespraeche.kennzahlen();
    let anhaenge = zustand.gespraeche.anhangkennzahlen();
    let betrieb = zustand.anwesenheit.betrieb();
    let protokoll = zustand.protokoll.kennzahlen();
    Json(Gesundheit {
        status: "bereit",
        version: env!("CARGO_PKG_VERSION"),
        ordner: zustand.ordner.display().to_string(),
        nachrichten: kennzahlen.nachrichten,
        log_bytes: kennzahlen.log_bytes,
        ladezeit_ms: kennzahlen.ladezeit_ms,
        defekte_zeilen: kennzahlen.defekte_zeilen,
        channels: kennzahlen.channels,
        zeit_vorlauf_ms: zustand.gespraeche.zeit_vorlauf_ms(),
        gebremst: zustand.schreibkonto.gebremst(),
        sitzungen: betrieb.sitzungen,
        halter: betrieb.halter,
        herzschlag_ms: zustand.herzschlag.as_millis(),
        abrissgrenze_ms: pflege_aus(zustand.herzschlag).user_timeout.as_millis(),
        protokoll_stufe: protokoll.stufe.als_wert(),
        protokoll_ok: protokoll.ok,
        protokoll_bytes_heute: protokoll.bytes_heute,
        protokoll_verworfen: protokoll.verworfen,
        warnings: protokoll.zaehler.warnings,
        errors: protokoll.zaehler.errors,
        exceptions: protokoll.zaehler.exceptions,
        letzter_error: protokoll.zaehler.letzter_error.as_ref().map(zeit_als_text),
        letzte_exception: protokoll
            .zaehler
            .letzte_exception
            .as_ref()
            .map(zeit_als_text),
        aufgaben_ausgefallen: protokoll.ausgefallen,
        abgewiesen: protokoll.abgewiesen,
        abgerissen: protokoll.abgerissen,
        werkbank_plaetze: zustand.werkbank.plaetze(),
        werkbank_belegt: zustand.werkbank.belegt(),
        alle_namen: zustand.grenzschutz.alle_namen(),
        plattenquote: kennzahlen.plattenquote,
        quote_abgewiesen: kennzahlen.quote_abgewiesen,
        schluessel_verdraengt: kennzahlen.schluessel_verdraengt,
        anhangquote: anhaenge.anhangquote,
        anhang_belegt: anhaenge.gebunden.saturating_add(anhaenge.reserviert),
        reserviert: anhaenge.reserviert,
        hochladungen: anhaenge.hochladungen,
        hochladebank_belegt: super::anhaenge::hochladen::HOCHLADEBANK
            .saturating_sub(zustand.hochladebank.available_permits()),
        anhang_abgewiesen: anhaenge.abgewiesen,
    })
}

#[cfg(test)]
mod tests {
    use crate::http::{Werkbank, testserver};
    use serde_json::json;
    use std::path::Path;

    async fn gesundheit_fuer(ordner: &str) -> (u16, String, serde_json::Value) {
        let mut zustand = testserver::zustand(Path::new(ordner));
        zustand.werkbank = Werkbank::mit(2);
        gesundheit_von(zustand).await
    }

    async fn gesundheit_von(zustand: crate::http::Zustand) -> (u16, String, serde_json::Value) {
        let adresse = testserver::app_starten(crate::http::router(zustand)).await;
        let antwort = reqwest::get(format!("{adresse}api/gesundheit"))
            .await
            .expect("Antwort");
        let status = antwort.status().as_u16();
        let typ = antwort.headers()["content-type"]
            .to_str()
            .expect("Content-Type")
            .to_string();
        (status, typ, antwort.json().await.expect("JSON"))
    }

    #[tokio::test]
    async fn leerer_ordner_ist_bereit_mit_kennzahlen_null() {
        let anfang = std::time::Instant::now();
        let (status, typ, mut inhalt) = gesundheit_fuer("/tmp/lc-r1").await;
        let gemessen = anfang.elapsed();
        let ladezeit_ms = inhalt["ladezeit_ms"]
            .take()
            .as_u64()
            .expect("ladezeit_ms als Zahl");

        assert!(
            u128::from(ladezeit_ms) <= gemessen.as_millis(),
            "{ladezeit_ms} ms"
        );
        assert_eq!(status, 200);
        assert_eq!(typ, "application/json");
        assert_eq!(
            inhalt,
            json!({
                "status": "bereit",
                "version": env!("CARGO_PKG_VERSION"),
                "ordner": "/tmp/lc-r1",
                "nachrichten": 0,
                "log_bytes": 0,
                "ladezeit_ms": null,
                "defekte_zeilen": 0,
                "channels": 0,
                "zeit_vorlauf_ms": 0,
                "gebremst": 0,
                "sitzungen": 0,
                "halter": 0,
                "herzschlag_ms": 5000,
                "abrissgrenze_ms": 10_000,
                "protokoll_stufe": "information",
                "protokoll_ok": true,
                "protokoll_bytes_heute": 0,
                "protokoll_verworfen": 0,
                "warnings": 0,
                "errors": 0,
                "exceptions": 0,
                "letzter_error": null,
                "letzte_exception": null,
                "aufgaben_ausgefallen": [],
                "abgewiesen": {"zu_gross": 0, "kein_json": 0, "kaputt": 0, "abgebrochen": 0, "zu_langsam": 0},
                "abgerissen": 0,
                "werkbank_plaetze": 2,
                "werkbank_belegt": 0,
                "alle_namen": false,
                "plattenquote": 4_294_967_296_u64,
                "quote_abgewiesen": 0,
                "schluessel_verdraengt": 0,
                "anhangquote": 21_474_836_480_u64,
                "anhang_belegt": 0,
                "reserviert": 0,
                "hochladungen": 0,
                "hochladebank_belegt": 0,
                "anhang_abgewiesen": 0,
            })
        );
    }

    /// R00018 F0060 (ADR-0043 Ergänzung #24): hält der Test einen Platz der Werkbank, ist einer belegt; ein
    /// Abriss zählt in `abgerissen`.
    #[tokio::test]
    async fn ein_gehaltener_platz_ist_belegt_und_ein_abriss_gezaehlt() {
        let mut zustand = testserver::zustand(Path::new("/tmp/lc-r1"));
        zustand.werkbank = Werkbank::mit(2);
        let gehalten = zustand.werkbank.erwerben().await.expect("Erlaubnis");
        zustand.protokoll.abriss_zaehlen();

        let (_, _, inhalt) = gesundheit_von(zustand).await;

        assert_eq!(inhalt["werkbank_plaetze"], 2);
        assert_eq!(inhalt["werkbank_belegt"], 1);
        assert_eq!(inhalt["abgerissen"], 1);
        drop(gehalten);
    }

    #[tokio::test]
    async fn der_ordner_der_instanz_steht_in_der_antwort() {
        let (_, _, inhalt) = gesundheit_fuer("/srv/anderer ordner").await;
        assert_eq!(inhalt["ordner"], "/srv/anderer ordner");
    }
}
