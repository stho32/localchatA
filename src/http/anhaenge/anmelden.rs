//! Anmelden beantworten (B0427, ADR-0046 Regel 1 mit Ergänzungen „Karte #9 Gate“, „F1′“, „Karte #9
//! Security-Gate“ L9): `POST /api/anhaenge {name, bytes}` mit Sitzung. Prüfreihenfolge, abgewiesen, nie umgeformt:
//! 403 · 401 · 429 (1 Einheit vorab) · 415/400/413/408 · 422 `feld` (doppelt, dann weiteres Feld) · 422 `bytes` ·
//! 422 `leer` · 422 `name` · 422 `zu_gross` · 429 offen · 507 `platte` · 507 `quote` (`voll` | `hochladungen`) · 201.

use super::super::Zustand;
use super::super::abbruch::{Abgebrochen, Grenzort, abbruch_beantworten};
use super::super::anfrageprotokoll::mit_regel;
use super::super::bremse::{Gegenstand, gebremst_melden};
use super::super::felder::{Felder, doppelt, wert};
use super::super::koerper::{Koerperfehler, Koerperort, felder_lesen, koerperfehler_beantworten};
use super::super::senden::groesse_in_worten;
use super::super::teilnehmer::Teilnehmer;
use super::super::teilnehmerliste::zeit_als_text;
use crate::gespraeche::Anmeldefehler;
use crate::gespraeche::anhangeingang::{Anmeldung, Offen};
use crate::gespraeche::anhangpruefung::Namensfehler;
use crate::gespraeche::anhangquote::{Platzfehler, Quotestand, zaehlmass};
use crate::protokoll::episoden::{Auftreten, Quelle};
use crate::protokoll::episodentexte;
use crate::protokoll::{Bereich, Kontext, Schluessel, Stufe};
use axum::Json;
use axum::body::Body;
use axum::extract::State;
use axum::http::header::RETRY_AFTER;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};
use std::sync::Arc;

const ROUTE: &str = "POST /api/anhaenge";

/// Körpergrenze (ADR-0046 Regel 1): Name ≤ 255 Zeichen in der längsten Schreibweise `\uXXXX` = 1.530, dazu
/// `{"name":"","bytes":18446744073709551615}` = 40, plus 30 % ≈ 2.041 → 2.048.
pub const ANMELDEN_GRENZE: usize = 2_048;

/// Vorab-Buchung am Schreibkonto (ADR-0036 Ergänzung Karte #9).
const VORAB_ANMELDEN: u32 = 1;

const BEISPIEL: &str = r#"{"name":"bericht.pdf","bytes":2457600}"#;

pub async fn anmelden(
    teilnehmer: Teilnehmer,
    State(zustand): State<Arc<Zustand>>,
    kopf: HeaderMap,
    koerper: Body,
) -> Response {
    if let Err(gebremst) =
        zustand
            .schreibkonto
            .vorab(&teilnehmer.kennung, teilnehmer.herkunft, VORAB_ANMELDEN)
    {
        return gebremst_melden(
            &zustand.protokoll,
            gebremst,
            Gegenstand::Anmeldungen,
            &teilnehmer,
        );
    }
    let wer = teilnehmer.kennung.als_text().to_string();
    let ort = Koerperort {
        protokoll: &zustand.protokoll,
        route: ROUTE,
        herkunft: teilnehmer.herkunft,
        kennung: Some(&wer),
    };
    let felder = match felder_lesen(&kopf, koerper, ANMELDEN_GRENZE, &ort).await {
        Ok(felder) => felder,
        Err(fehler) => return koerper_abgewiesen(fehler),
    };
    let (name, bytes) = match koerper_pruefen(felder.as_ref()) {
        Ok(gelesen) => gelesen,
        Err(abweisung) => {
            let antwort =
                (StatusCode::UNPROCESSABLE_ENTITY, Json(abweisung.koerper)).into_response();
            return mit_regel(antwort, abweisung.regel, Some(&wer));
        }
    };
    let gespraeche = Arc::clone(&zustand.gespraeche);
    let kennung = teilnehmer.kennung.clone();
    let herkunft = teilnehmer.herkunft;
    let anzeigename = name.clone();
    let angemeldet = tokio::task::spawn_blocking(move || {
        gespraeche.anhang_anmelden(&kennung, herkunft, &anzeigename, bytes)
    })
    .await;
    match angemeldet {
        Ok(Ok(anmeldung)) => {
            zustand.protokoll.eintragen(
                Stufe::Debug,
                Bereich::Anhang,
                "Anmeldung angenommen",
                Kontext::neu()
                    .mit(Schluessel::Kennung, wer.as_str())
                    .mit(Schluessel::Herkunft, teilnehmer.herkunft)
                    .mit(Schluessel::Bytes, bytes)
                    .mit(
                        Schluessel::Datei,
                        format!(".eingang/{}", anmeldung.id.praefix()),
                    ),
            );
            anspruch_melden(&zustand, &teilnehmer, bytes, &anmeldung);
            angenommen(&anmeldung, &name, bytes)
        }
        Ok(Err(fehler)) => {
            abweisung_melden(&zustand, &teilnehmer, &fehler, bytes);
            let (antwort, regel) = abgewiesen(&fehler);
            mit_regel(antwort, regel, Some(&wer))
        }
        Err(fehler) => abbruch_beantworten(
            &zustand.protokoll,
            &Abgebrochen::from(fehler),
            &Grenzort {
                route: ROUTE,
                operation: "anhaenge_anmelden",
                kennung: Some(&wer),
                channel: None,
            },
            "Anhang nicht angemeldet",
        ),
    }
}

/// Eine 422 des Körpers mit ihrem Regelschlüssel für das Anfrageprotokoll.
#[derive(Debug)]
struct Abweisung {
    koerper: Value,
    regel: &'static str,
}

fn unprocessable(regel: &'static str, koerper: Value) -> Abweisung {
    Abweisung { koerper, regel }
}

/// Feld doppelt → weiteres Feld → `bytes` → `leer` (B0427); `name` als Zeichenkette prüft die Namensregel danach.
fn koerper_pruefen(felder: Option<&Felder>) -> Result<(String, u64), Abweisung> {
    let Some(felder) = felder else {
        return Err(unprocessable(
            "feld_unbekannt",
            json!({
                "fehler": format!("Der Körper braucht ein JSON-Objekt, zum Beispiel {BEISPIEL}."),
                "regel": "feld",
            }),
        ));
    };
    if let Some(feld) = doppelt(felder) {
        return Err(unprocessable(
            "feld_doppelt",
            json!({
                "fehler": format!("Das Feld {feld} steht zweimal im Körper — jedes Feld nur einmal schicken."),
                "regel": "feld",
                "feld": feld,
                "grund": "doppelt",
            }),
        ));
    }
    if let Some((feld, _)) = felder
        .iter()
        .find(|(feld, _)| feld != "name" && feld != "bytes")
    {
        return Err(unprocessable(
            "feld_unbekannt",
            json!({
                "fehler": format!("Unbekanntes Feld {feld} — erlaubt sind name und bytes."),
                "regel": "feld",
                "feld": feld,
            }),
        ));
    }
    let Some(bytes) = wert(felder, "bytes").and_then(Value::as_u64) else {
        return Err(unprocessable(
            "bytes",
            json!({
                "fehler": "bytes braucht die Länge der Datei als ganze Zahl in Bytes.",
                "regel": "bytes",
            }),
        ));
    };
    if bytes == 0 {
        return Err(unprocessable(
            "leer",
            json!({ "fehler": "Die Datei ist leer — es gibt nichts hochzuladen.", "regel": "leer" }),
        ));
    }
    let Some(name) = wert(felder, "name").and_then(Value::as_str) else {
        return Err(unprocessable(
            "name",
            json!({
                "fehler": "name braucht den Dateinamen als Zeichenkette.",
                "regel": "name",
                "grund": "fehlt",
            }),
        ));
    };
    Ok((name.to_string(), bytes))
}

/// Ob eine angenommene Anmeldung ein großer Anspruch ist: `z(bytes) ≥ Q/4` bei gesetzter Anhang-Quote (B0525).
pub fn grosser_anspruch(bytes: u64, anhangquote: Option<u64>) -> bool {
    anhangquote.is_some_and(|quote| zaehlmass(bytes).saturating_mul(4) >= quote)
}

/// Episode je Kennung `INFORMATION anhang Großer Anspruch` (ADR-0042 und ADR-0046 Ergänzungen „Karte #9
/// Security-Gate“, M42′ Bedingung 2): wer so viel beansprucht, kann mit leeren `PUT`s das Hochladen aller sperren.
fn anspruch_melden(zustand: &Zustand, teilnehmer: &Teilnehmer, bytes: u64, anmeldung: &Anmeldung) {
    if !grosser_anspruch(bytes, zustand.gespraeche.anhangkennzahlen().anhangquote) {
        return;
    }
    zustand.protokoll.episode(anspruch_auftreten(
        teilnehmer.kennung.als_text(),
        teilnehmer.herkunft,
        bytes,
        &zeit_als_text(&anmeldung.hochladen_bis),
    ));
}

/// Quelle der Episode „Großer Anspruch“ ist die Kennung (mit ihrer Herkunft); Bytes und Frist sind Werte der
/// einzelnen Anmeldung und stehen nur in der Beginn-Zeile (ADR-0042 Ergänzung Karte #66).
pub(crate) fn anspruch_auftreten(
    wer: &str,
    herkunft: std::net::IpAddr,
    bytes: u64,
    hochladen_bis: &str,
) -> Auftreten {
    Auftreten::neu(
        Stufe::Information,
        Bereich::Anhang,
        episodentexte::GROSSER_ANSPRUCH,
        Quelle::neu()
            .mit(Schluessel::Kennung, wer)
            .mit(Schluessel::Herkunft, herkunft),
    )
    .einzeln(Schluessel::Bytes, bytes)
    .einzeln(Schluessel::HochladenBis, hochladen_bis)
}

/// 201 `{anhang: {id, name, bytes, hochladen, anlauf_bis, hochladen_bis}}`.
fn angenommen(anmeldung: &Anmeldung, name: &str, bytes: u64) -> Response {
    let id = anmeldung.id.als_text();
    (
        StatusCode::CREATED,
        Json(json!({
            "anhang": {
                "id": id,
                "name": name,
                "bytes": bytes,
                "hochladen": format!("/api/anhaenge/{id}"),
                "anlauf_bis": zeit_als_text(&anmeldung.anlauf_bis),
                "hochladen_bis": zeit_als_text(&anmeldung.hochladen_bis),
            }
        })),
    )
        .into_response()
}

/// Episoden der Abweisungen am Platz (B0463, ADR-0042 Ergänzungen „F1 und F4“ und „F1′“): `voll` und
/// `hochladungen` je Kennung, `platte` je Herkunft — alle unter `anhang`, nie unter `quote`.
fn abweisung_melden(
    zustand: &Zustand,
    teilnehmer: &Teilnehmer,
    fehler: &Anmeldefehler,
    bytes: u64,
) {
    if let Some(auftreten) = abweisung_auftreten(teilnehmer, fehler, bytes) {
        zustand.protokoll.episode(auftreten);
    }
}

/// Das Auftreten einer Abweisung am Platz. Quelle ist bei `voll` und `hochladungen` die Kennung (mit der Grenze),
/// bei `platte` die Herkunft; `bytes`, `belegt`, `reserviert` und `frei` sind Werte der einzelnen Anmeldung und
/// stehen nur in der Beginn-Zeile (ADR-0042 Ergänzung Karte #66).
pub(crate) fn abweisung_auftreten(
    teilnehmer: &Teilnehmer,
    fehler: &Anmeldefehler,
    bytes: u64,
) -> Option<Auftreten> {
    let wer = teilnehmer.kennung.als_text();
    let (text, quelle, einzeln) = match fehler {
        Anmeldefehler::Platz(Platzfehler::Voll(stand)) => (
            episodentexte::ANHAENGE_VOLL,
            Quelle::neu()
                .mit(Schluessel::Kennung, wer)
                .mit(Schluessel::Hoechstens, stand.hoechstens),
            vec![
                (Schluessel::Bytes, bytes),
                (Schluessel::Belegt, stand.belegt),
            ],
        ),
        Anmeldefehler::Platz(Platzfehler::Hochladungen(stand)) => (
            episodentexte::HOCHLADUNGEN_BELEGEN,
            Quelle::neu()
                .mit(Schluessel::Kennung, wer)
                .mit(Schluessel::Hoechstens, stand.hoechstens),
            vec![(Schluessel::Reserviert, stand.reserviert)],
        ),
        Anmeldefehler::Platz(Platzfehler::Platte { frei }) => (
            episodentexte::PLATTE_FAST_VOLL,
            Quelle::neu().mit(Schluessel::Herkunft, teilnehmer.herkunft),
            vec![(Schluessel::Bytes, bytes), (Schluessel::Frei, *frei)],
        ),
        _ => return None,
    };
    let auftreten = Auftreten::neu(Stufe::Warning, Bereich::Anhang, text, quelle);
    Some(
        einzeln
            .into_iter()
            .fold(auftreten, |auftreten, (schluessel, wert)| {
                auftreten.einzeln(schluessel, wert)
            }),
    )
}

/// Die Antwort auf eine abgewiesene Anmeldung und ihr Regelschlüssel.
fn abgewiesen(fehler: &Anmeldefehler) -> (Response, &'static str) {
    match fehler {
        Anmeldefehler::Name(fehler) => (name_abgewiesen(fehler), "name"),
        Anmeldefehler::Platz(Platzfehler::ZuGross { hoechstens }) => (
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(json!({
                    "fehler": format!(
                        "Nicht hochgeladen: die Datei ist größer als der ganze Platz für Anhänge dieser Instanz ({}).",
                        groesse_in_worten(*hoechstens)
                    ),
                    "regel": "zu_gross",
                    "hoechstens": hoechstens,
                })),
            )
                .into_response(),
            "zu_gross",
        ),
        Anmeldefehler::Offen(offen) => (offen_abgewiesen(*offen), offen.grenze()),
        Anmeldefehler::Platz(Platzfehler::Platte { frei }) => (
            (
                StatusCode::INSUFFICIENT_STORAGE,
                Json(json!({
                    "fehler": format!(
                        "Nicht hochgeladen: auf der Platte ist nicht genug Platz — frei sind {}, 1,0 GiB bleiben immer frei.",
                        groesse_in_worten(*frei)
                    ),
                    "regel": "platte",
                    "frei": frei,
                })),
            )
                .into_response(),
            "platte",
        ),
        Anmeldefehler::Platz(Platzfehler::Voll(stand)) => (
            quote_abgewiesen(
                stand,
                "voll",
                format!(
                    "Nicht hochgeladen: der Platz für Anhänge ist voll — frei sind {}, die Datei braucht {}. Platz entsteht, wenn Anhänge gelöscht werden oder ältere Tage entfallen; Text kann weiter gesendet werden.",
                    groesse_in_worten(stand.frei),
                    groesse_in_worten(stand.braucht)
                ),
            )
            .into_response(),
            "quote_voll",
        ),
        Anmeldefehler::Platz(Platzfehler::Hochladungen(stand)) => (
            (
                [(RETRY_AFTER, "30")],
                quote_abgewiesen(
                    stand,
                    "hochladungen",
                    format!(
                        "Nicht hochgeladen: laufende Hochladungen belegen gerade den Platz für Anhänge — noch {} frei. Gleich noch einmal versuchen.",
                        groesse_in_worten(stand.frei)
                    ),
                ),
            )
                .into_response(),
            "quote_hochladungen",
        ),
        Anmeldefehler::Zufall(_) => (
            super::super::abbruch::interner_fehler("Anmelden"),
            "zufall",
        ),
    }
}

fn quote_abgewiesen(stand: &Quotestand, grund: &str, fehler: String) -> impl IntoResponse {
    (
        StatusCode::INSUFFICIENT_STORAGE,
        Json(json!({
            "fehler": fehler,
            "regel": "quote",
            "grund": grund,
            "belegt": stand.belegt,
            "gebunden": stand.gebunden,
            "reserviert": stand.reserviert,
            "hoechstens": stand.hoechstens,
            "frei": stand.frei,
            "braucht": stand.braucht,
        })),
    )
}

/// 429 `{fehler, grenze, hoechstens}` mit `Retry-After: 5`.
fn offen_abgewiesen(offen: Offen) -> Response {
    (
        StatusCode::TOO_MANY_REQUESTS,
        [(RETRY_AFTER, "5")],
        Json(json!({
            "fehler": grenze_text(offen),
            "grenze": offen.grenze(),
            "hoechstens": offen.hoechstens(),
        })),
    )
        .into_response()
}

/// Text des 429 je Grenze (ADR-0046 Ergänzung „Karte #9 Nacharbeit F1“ Regel 2): nur `offen_kennung` liegt in der
/// Hand des Teilnehmers.
pub(super) fn grenze_text(offen: Offen) -> &'static str {
    match offen {
        Offen::Kennung => {
            "Nicht angemeldet: höchstens 10 Dateien gleichzeitig offen — erst senden oder abbrechen."
        }
        Offen::Herkunft | Offen::Gesamt | Offen::LaufendKennung | Offen::LaufendHerkunft => {
            "Gerade zu viele Hochladungen — gleich noch einmal versuchen."
        }
    }
}

/// 422 `name` mit `grund` (`leer` | `laenge` | `zeichen` | `punkte`).
fn name_abgewiesen(fehler: &Namensfehler) -> Response {
    let (grund, text) = match fehler {
        Namensfehler::Leer => ("leer", "Der Dateiname ist leer.".to_string()),
        Namensfehler::Laenge { bytes } => (
            "laenge",
            format!("Der Dateiname ist zu lang: {bytes} von höchstens 255 Byte."),
        ),
        Namensfehler::Zeichen { stelle, zeichen } => (
            "zeichen",
            format!(
                "Der Dateiname enthält das Zeichen {zeichen} an Stelle {} — bitte umbenennen.",
                stelle + 1
            ),
        ),
        Namensfehler::Punkte => (
            "punkte",
            "Der Dateiname darf nicht „.“ oder „..“ sein.".to_string(),
        ),
    };
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(json!({ "fehler": text, "regel": "name", "grund": grund })),
    )
        .into_response()
}

/// Die eine Antwort für Körperfehler mit dem 413-Satz und dem Beispiel der Route.
fn koerper_abgewiesen(fehler: Koerperfehler) -> Response {
    koerperfehler_beantworten(
        fehler,
        "Der Körper ist zu groß — höchstens 2.048 Byte; nur name und bytes schicken.",
        BEISPIEL,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// R00038 B0591 (Bestandsaufnahme): `voll`, `hochladungen` und `platte` — je zwei Abweisungen mit verschiedenen
    /// Werten; das Ende nennt nur die Quelle (und die Grenze), nie `bytes`, `belegt`, `reserviert`, `frei`.
    #[test]
    fn die_ende_zeilen_der_abweisungen_am_platz_nennen_nur_ihre_quelle() {
        use crate::anwesenheit::Art;
        use crate::gespraeche::Kennung;
        use crate::gespraeche::anhangquote::{Platzfehler, Quotestand};
        use crate::name::Name;
        use crate::protokoll::Stufe;
        use crate::protokoll::testhilfe::{im_speicher, zeit};
        let teilnehmer = Teilnehmer {
            kennung: Kennung::bilden("beta@10.0.0.9").expect("Kennung"),
            name: Name::bilden("beta").expect("Name"),
            herkunft: "10.0.0.9".parse().expect("IP"),
            art: Art::Agent,
        };
        fn stand(zahl: u64) -> Quotestand {
            Quotestand {
                belegt: 100 + zahl,
                gebunden: 1,
                reserviert: 10 + zahl,
                hoechstens: 1000,
                frei: 0,
                braucht: 1,
            }
        }
        type Fall = (
            &'static str,
            fn(u64) -> Anmeldefehler,
            &'static str,
            &'static str,
        );
        let faelle: [Fall; 3] = [
            (
                "Platz für Anhänge voll",
                |n| Anmeldefehler::Platz(Platzfehler::Voll(stand(n))),
                "| kennung=beta@10.0.0.9 hoechstens=1000 bytes=5 belegt=101",
                "| kennung=beta@10.0.0.9 hoechstens=1000",
            ),
            (
                "laufende Hochladungen belegen den Platz",
                |n| Anmeldefehler::Platz(Platzfehler::Hochladungen(stand(n))),
                "| kennung=beta@10.0.0.9 hoechstens=1000 reserviert=11",
                "| kennung=beta@10.0.0.9 hoechstens=1000",
            ),
            (
                "Platte fast voll",
                |n| Anmeldefehler::Platz(Platzfehler::Platte { frei: 7 + n }),
                "bytes=5 frei=8",
                "| herkunft=10.0.0.9",
            ),
        ];
        for (name, fehler, beginn, ende) in faelle {
            let (protokoll, speicher, uhr, _) =
                im_speicher("2026-10-28T12:00:00+01:00", Stufe::Information);
            for (n, bytes) in [(1, 5), (2, 6)] {
                let auftreten =
                    abweisung_auftreten(&teilnehmer, &fehler(n), bytes).expect("Episode");
                protokoll.episode(auftreten);
            }
            uhr.stellen(zeit("2026-10-28T12:01:00+01:00"));
            protokoll.episoden_schliessen();

            let zeilen = speicher.zeilen();
            assert_eq!(zeilen.len(), 2, "{name}: {zeilen:#?}");
            assert!(zeilen[0].contains(beginn), "{name}: {zeilen:#?}");
            assert!(
                zeilen[1].contains("Episode endet: 2 in "),
                "{name}: {zeilen:#?}"
            );
            assert!(zeilen[1].ends_with(ende), "{name}: {zeilen:#?}");
        }
    }

    struct Panikzufall;

    impl crate::zufall::Zufall for Panikzufall {
        fn token(&self) -> Result<crate::zufall::Token, crate::zufall::Zufallsfehler> {
            panic!("Panik im Zufall");
        }
    }

    /// R00043 B0667 (ADR-0048 K3): eine Panik im Blocking-Teil des Anmeldens antwortet 500 ohne Fremdtext und
    /// schreibt genau einen `ERROR programmfehler … operation=anhaenge_anmelden grund=panik`.
    #[tokio::test(flavor = "multi_thread")]
    async fn eine_panik_beim_anmelden_wird_beantwortet_und_protokolliert() {
        use crate::http::loeschaufbau::aufbau_mit;
        use crate::protokoll::testpanik::Testpanik;
        let aufbau = aufbau_mit(Arc::new(Panikzufall), Testpanik::keine());

        let antwort = aufbau.anhaenge_anmelden("beta@127.0.0.1").await;

        assert_eq!(antwort.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let koerper = axum::body::to_bytes(antwort.into_body(), 4096)
            .await
            .expect("Körper");
        let json: Value = serde_json::from_slice(&koerper).expect("JSON");
        assert_eq!(
            json["fehler"],
            "Interner Fehler — Anhang nicht angemeldet; bitte erneut versuchen"
        );
        let eintraege = aufbau.zeilen_mit(" programmfehler ");
        assert_eq!(eintraege.len(), 1, "{eintraege:#?}");
        assert!(
            eintraege[0].contains(" ERROR programmfehler "),
            "{eintraege:#?}"
        );
        assert!(
            eintraege[0].contains("route=\"POST /api/anhaenge\" operation=anhaenge_anmelden status=500 kennung=beta@127.0.0.1 grund=panik"),
            "{eintraege:#?}"
        );
    }

    /// B0525 an der Grenze im Zählmaß (Q = 10 MiB): 2.617.345 Bytes (z = 640 × 4.096 = Q/4) → großer Anspruch,
    /// 2.617.344 (z = 639 × 4.096) → nicht; ohne Anhang-Quote nie (Mutanten „> statt ≥“, „bytes statt z“).
    #[test]
    fn grosser_anspruch_an_der_grenze() {
        let quote = Some(10 * 1_048_576);
        assert!(grosser_anspruch(2_617_345, quote));
        assert!(!grosser_anspruch(2_617_344, quote));
        assert!(grosser_anspruch(3 * 1_048_576, quote));
        assert!(!grosser_anspruch(u64::MAX / 8, None));
    }

    fn felder(roh: &str) -> Felder {
        super::super::super::felder::felder_lesen(roh.as_bytes())
            .expect("JSON")
            .expect("Objekt")
    }

    async fn regel(felder: Option<&Felder>) -> (String, Value) {
        let abweisung = koerper_pruefen(felder).expect_err("abgewiesen");
        (abweisung.regel.to_string(), abweisung.koerper)
    }

    /// R00029 „Abweisungen der Anmeldung“: weiteres Feld → `feld`; `bytes` −1, 1.5, "1", 2⁶⁴ → `bytes`; 0 → `leer`;
    /// `bytes` zweimal → `feld`/`doppelt`, nie die letzte Fassung.
    #[tokio::test]
    async fn der_koerper_wird_in_der_reihenfolge_geprueft() {
        let (regel_x, x) = regel(Some(&felder(r#"{"name":"a","bytes":1,"x":1}"#))).await;
        assert_eq!(
            (regel_x.as_str(), x["regel"].as_str(), x["feld"].as_str()),
            ("feld_unbekannt", Some("feld"), Some("x"))
        );
        for falsch in ["-1", "1.5", "\"1\"", "18446744073709551616", "null"] {
            let (_, antwort) = regel(Some(&felder(&format!(
                r#"{{"name":"a","bytes":{falsch}}}"#
            ))))
            .await;
            assert_eq!(antwort["regel"], "bytes", "{falsch}");
        }
        let (_, leer) = regel(Some(&felder(r#"{"name":"a","bytes":0}"#))).await;
        assert_eq!(leer["regel"], "leer");
        let (_, doppelt) = regel(Some(&felder(r#"{"name":"a","bytes":1,"bytes":2}"#))).await;
        assert_eq!(
            (
                doppelt["regel"].as_str(),
                doppelt["feld"].as_str(),
                doppelt["grund"].as_str()
            ),
            (Some("feld"), Some("bytes"), Some("doppelt"))
        );
        let (_, doppelt_vor_fremd) =
            regel(Some(&felder(r#"{"x":1,"name":"a","name":"b","bytes":1}"#))).await;
        assert_eq!(doppelt_vor_fremd["grund"], "doppelt");
        let (_, ohne_name) = regel(Some(&felder(r#"{"bytes":1}"#))).await;
        assert_eq!(
            (ohne_name["regel"].as_str(), ohne_name["grund"].as_str()),
            (Some("name"), Some("fehlt"))
        );
        let (_, kein_objekt) = regel(None).await;
        assert_eq!(kein_objekt["regel"], "feld");
        assert_eq!(
            koerper_pruefen(Some(&felder(
                r#"{"name":"a","bytes":18446744073709551615}"#
            )))
            .ok(),
            Some(("a".to_string(), u64::MAX))
        );
    }
}
