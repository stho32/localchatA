//! R00040 F0136 — eigene Nachricht über die API löschen (ADR-0048 Regeln 5, 9 a, 10, 12, 13): das echte Programm
//! mit `channels/allgemein/` und `channels/technik/`, `--plattenquote 10MiB --anhangquote 10MiB`; `beta` sendet
//! `Hallo` (Zeit `t1`), `alpha` liest den Strom.

mod gemeinsam;

use gemeinsam::anhaenge::gesundheit;
use gemeinsam::senden::{beitreten, ordner, senden, verlauf};
use gemeinsam::strom::{Empfangen, Strom};
use gemeinsam::{Absender, ApiAntwort, Instanz, api_antwort};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::time::Instant;

const JETZT: &str = "2026-10-28T12:00:00.000+01:00";
const HEUTE: &str = "2026-10-28";
/// Zeitkriterium (`AI-Team/planer.md`): Ereignis weniger als 1 s nach der 200.
const GRENZE: Duration = Duration::from_secs(1);
/// Abwesenheit über die doppelte Grenze beobachtet.
const STILLE: Duration = Duration::from_secs(2);

fn starten_mit(ordner: &Path, plattenquote: &str) -> Instanz {
    gemeinsam::starten_mit(
        ordner,
        &[
            "--port",
            "0",
            "--host",
            "127.0.0.1",
            "--kein-browser",
            "--plattenquote",
            plattenquote,
            "--anhangquote",
            "10MiB",
        ],
        &[("TZ", "Europe/Berlin"), ("LOCALCHATA_JETZT", JETZT)],
    )
}

fn starten(ordner: &Path) -> Instanz {
    starten_mit(ordner, "10MiB")
}

/// `POST …/channels/<channel>/nachrichten/loeschen` mit Rohkörper, Typ, optionalem Token und weiteren Köpfen.
async fn loeschen_roh(
    absender: &Absender,
    basis: &str,
    token: Option<&str>,
    channel: &str,
    typ: &str,
    koerper: Vec<u8>,
    koepfe: &[(&str, &str)],
) -> ApiAntwort {
    let mut anfrage = absender
        .anfrage(
            reqwest::Method::POST,
            format!("{basis}api/channels/{channel}/nachrichten/loeschen"),
        )
        .header("Content-Type", typ)
        .body(koerper);
    if let Some(token) = token {
        anfrage = anfrage.header("X-LocalchatA-Sitzung", token);
    }
    for (name, wert) in koepfe {
        anfrage = anfrage.header(*name, *wert);
    }
    api_antwort(anfrage).await
}

/// Löschen in `channel` mit `{"zeit": zeit}`.
async fn loeschen_in(
    absender: &Absender,
    basis: &str,
    token: &str,
    channel: &str,
    zeit: &str,
) -> ApiAntwort {
    loeschen_roh(
        absender,
        basis,
        Some(token),
        channel,
        "application/json",
        json!({ "zeit": zeit }).to_string().into_bytes(),
        &[],
    )
    .await
}

async fn loeschen(absender: &Absender, basis: &str, token: &str, zeit: &str) -> ApiAntwort {
    loeschen_in(absender, basis, token, "allgemein", zeit).await
}

async fn bearbeiten(
    absender: &Absender,
    basis: &str,
    token: &str,
    zeit: &str,
    text: &str,
) -> ApiAntwort {
    api_antwort(
        absender
            .anfrage(
                reqwest::Method::POST,
                format!("{basis}api/channels/allgemein/nachrichten/bearbeiten"),
            )
            .header("X-LocalchatA-Sitzung", token)
            .json(&json!({ "zeit": zeit, "text": text })),
    )
    .await
}

/// Alle Dateien unter `channels/` mit ihrem Inhalt — „Tageslog unverändert“ heißt: vorher gleich nachher.
fn channels_stand(ordner: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut stand = BTreeMap::new();
    let mut offen = vec![ordner.join("channels")];
    while let Some(pfad) = offen.pop() {
        for eintrag in std::fs::read_dir(&pfad).expect("Ordner") {
            let pfad = eintrag.expect("Eintrag").path();
            if pfad.is_dir() {
                offen.push(pfad);
            } else {
                stand.insert(pfad.clone(), std::fs::read(&pfad).expect("Datei"));
            }
        }
    }
    stand
}

fn tagesdatei(ordner: &Path) -> String {
    std::fs::read_to_string(ordner.join(format!("channels/allgemein/{HEUTE}.log")))
        .unwrap_or_default()
}

fn zeit_von(antwort: &ApiAntwort) -> String {
    assert_eq!(antwort.status, 201, "{}", antwort.json);
    antwort.json["nachricht"]["zeit"]
        .as_str()
        .expect("zeit")
        .to_string()
}

/// Beta ist beigetreten und hat `Hallo` gesendet: Token und `t1`.
async fn beta_mit_hallo(beta: &Absender, basis: &str) -> (String, String) {
    let token = beitreten(beta, basis, "beta", Some("agent")).await;
    let t1 = zeit_von(&senden(beta, basis, &token, "allgemein", "Hallo").await);
    (token, t1)
}

fn ist_aenderung(e: &Empfangen) -> bool {
    e.data["art"] == "aenderung"
}

fn nachrichten_im(verlauf: &Value) -> &Vec<Value> {
    verlauf["nachrichten"].as_array().expect("nachrichten")
}

/// R00040 „Rundweg“ (US-1, US-5): 200 mit `ergebnis: "geloescht"`, leerem Text, ohne Anhänge, `zustand: "geloescht"`,
/// `geaendert` > t1 und `anhaenge` 0/0/0; der Verlauf zeigt t1 gelöscht; Health `nachrichten` bleibt 1, `log_bytes`
/// wächst um genau den Löschsatz; `Hallo` steht weiter genau einmal in der Tagesdatei, dazu `+ geloescht: <t1>`;
/// nach einem Neustart derselbe Verlauf.
#[tokio::test]
async fn rundweg_mit_neustart_und_text_im_log() {
    let wurzel = ordner();
    let beta = Absender::von("127.0.0.1");
    let vorher_verlauf = {
        let instanz = starten(wurzel.path());
        let basis = instanz.adresse.as_str();
        let (token, t1) = beta_mit_hallo(&beta, basis).await;
        let bytes_vorher = gesundheit(basis).await["log_bytes"]
            .as_u64()
            .expect("log_bytes");
        let log_vorher = tagesdatei(wurzel.path());

        let antwort = loeschen(&beta, basis, &token, &t1).await;

        assert_eq!(antwort.status, 200, "{}", antwort.json);
        assert_eq!(antwort.json["channel"], "allgemein");
        assert_eq!(antwort.json["ergebnis"], "geloescht");
        assert_eq!(
            antwort.json["anhaenge"],
            json!({"geloescht": 0, "bytes": 0, "fehler": 0})
        );
        let nachricht = &antwort.json["nachricht"];
        assert_eq!(nachricht["zeit"], t1.as_str());
        assert_eq!(nachricht["von"], "beta@127.0.0.1");
        assert_eq!(nachricht["art"], "agent");
        assert_eq!(nachricht["text"], "");
        assert_eq!(nachricht["anhaenge"], json!([]));
        assert_eq!(nachricht["zustand"], "geloescht");
        let geaendert = nachricht["geaendert"].as_str().expect("geaendert");
        assert!(
            chrono::DateTime::parse_from_rfc3339(geaendert).expect("geaendert")
                > chrono::DateTime::parse_from_rfc3339(&t1).expect("t1")
        );
        let jetzt = verlauf(&beta, basis, &token, "allgemein").await;
        assert_eq!(nachrichten_im(&jetzt).len(), 1, "{jetzt}");
        assert_eq!(nachrichten_im(&jetzt)[0], *nachricht);
        let health = gesundheit(basis).await;
        assert_eq!(health["nachrichten"], 1);
        let log = tagesdatei(wurzel.path());
        let loeschsatz = &log[log_vorher.len()..];
        assert_eq!(
            loeschsatz,
            format!("@ {geaendert} beta@127.0.0.1\n+ geloescht: {t1}\n\n")
        );
        assert_eq!(
            health["log_bytes"].as_u64(),
            Some(bytes_vorher + loeschsatz.len() as u64)
        );
        assert_eq!(log.matches("\n  Hallo\n").count(), 1, "{log}");
        jetzt
    };
    let instanz = starten(wurzel.path());
    let token = beitreten(&beta, &instanz.adresse, "beta", Some("agent")).await;

    let nach_neustart = verlauf(&beta, &instanz.adresse, &token, "allgemein").await;

    assert_eq!(
        nachrichten_im(&nach_neustart),
        nachrichten_im(&vorher_verlauf)
    );
}

/// R00040 „Zweimal löschen, danach bearbeiten“ (US-2, M5 Teil Folge): das zweite Löschen → 200 `schon_geloescht`,
/// Tageslog unverändert, kein `aenderung` bei `alpha` binnen 2 s; Bearbeiten → 409 `geloescht` mit „Diese Nachricht
/// ist gelöscht.“, Tageslog unverändert.
#[tokio::test]
async fn zweimal_loeschen_und_danach_bearbeiten() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path());
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let alpha = beitreten(&beta, basis, "alpha", None).await;
    let (token, t1) = beta_mit_hallo(&beta, basis).await;
    let erstes = loeschen(&beta, basis, &token, &t1).await;
    assert_eq!(erstes.status, 200, "{}", erstes.json);
    let mut strom = Strom::oeffnen(&beta, basis, &alpha).await;
    let vorher = channels_stand(wurzel.path());

    let zweites = loeschen(&beta, basis, &token, &t1).await;
    let bearbeitet = bearbeiten(&beta, basis, &token, &t1, "wieder da").await;
    strom.mitlesen(STILLE).await;

    assert_eq!(zweites.status, 200, "{}", zweites.json);
    assert_eq!(zweites.json["ergebnis"], "schon_geloescht");
    assert_eq!(zweites.json["nachricht"], erstes.json["nachricht"]);
    assert_eq!(
        zweites.json["anhaenge"],
        json!({"geloescht": 0, "bytes": 0, "fehler": 0})
    );
    assert_eq!(bearbeitet.status, 409, "{}", bearbeitet.json);
    assert_eq!(
        bearbeitet.json,
        json!({"fehler": "Diese Nachricht ist gelöscht.", "regel": "geloescht"})
    );
    assert_eq!(channels_stand(wurzel.path()), vorher);
    assert!(
        !strom.ereignisse.iter().any(ist_aenderung),
        "{:?}",
        strom.ereignisse
    );
}

/// R00040 „Spätere Sitzung derselben Kennung“ (Richtungsfrage 6): `beta` verlässt und tritt erneut als `beta` bei →
/// Löschen von t1 mit dem neuen Token → 200.
#[tokio::test]
async fn eine_spaetere_sitzung_derselben_kennung_darf() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path());
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let (token, t1) = beta_mit_hallo(&beta, basis).await;
    let verlassen = api_antwort(
        beta.anfrage(reqwest::Method::POST, format!("{basis}api/verlassen"))
            .header("X-LocalchatA-Sitzung", &token),
    )
    .await;
    assert!(verlassen.status < 300, "{}", verlassen.json);
    let neu = beitreten(&beta, basis, "beta", Some("agent")).await;

    let antwort = loeschen(&beta, basis, &neu, &t1).await;

    assert_eq!(antwort.status, 200, "{}", antwort.json);
    assert_eq!(antwort.json["ergebnis"], "geloescht");
}

/// R00040 „Fremd und nicht da“ (US-3, M1, M3, M4): `gamma` löscht t1 → 403 `fremd`; t1 + 1 ms → 404 `nachricht`; t1
/// an `# technik` → 404 `nachricht`; ein unbekannter Channel → 404; jeweils Tageslog unverändert, kein Ereignis, der
/// Verlauf zeigt weiter `Hallo`.
#[tokio::test]
async fn fremd_und_nicht_da_aendern_nichts() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path());
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let alpha = beitreten(&beta, basis, "alpha", None).await;
    let (token, t1) = beta_mit_hallo(&beta, basis).await;
    let gamma = beitreten(&beta, basis, "gamma", None).await;
    let danach = (chrono::DateTime::parse_from_rfc3339(&t1).expect("t1")
        + chrono::Duration::milliseconds(1))
    .to_rfc3339_opts(chrono::SecondsFormat::Millis, false);
    let mut strom = Strom::oeffnen(&beta, basis, &alpha).await;
    let vorher = channels_stand(wurzel.path());

    let fremd = loeschen(&beta, basis, &gamma, &t1).await;
    let ohne = loeschen(&beta, basis, &token, &danach).await;
    let technik = loeschen_in(&beta, basis, &token, "technik", &t1).await;
    let unbekannt = loeschen_in(&beta, basis, &token, "gibtsnicht", &t1).await;
    strom.mitlesen(STILLE).await;

    assert_eq!(fremd.status, 403, "{}", fremd.json);
    assert_eq!(
        fremd.json,
        json!({"fehler": "Nur der Absender beta@127.0.0.1 kann diese Nachricht ändern.", "regel": "fremd"})
    );
    for antwort in [&ohne, &technik] {
        assert_eq!(antwort.status, 404, "{}", antwort.json);
        assert_eq!(
            antwort.json,
            json!({ "fehler": "Diese Nachricht gibt es nicht (mehr).", "regel": "nachricht" })
        );
    }
    assert_eq!(unbekannt.status, 404, "{}", unbekannt.json);
    assert_eq!(unbekannt.json["fehler"], "# gibtsnicht gibt es nicht.");
    assert_eq!(channels_stand(wurzel.path()), vorher);
    assert!(!strom.ereignisse.iter().any(ist_aenderung));
    let jetzt = verlauf(&beta, basis, &token, "allgemein").await;
    assert_eq!(nachrichten_im(&jetzt)[0]["text"], "Hallo");
}

/// R00040 „Live“ (US-4; ADR-0048 Integrationstest „Löschen über die API mit Strom-Mitschnitt“): `alpha` erhält genau
/// ein `aenderung` mit der gelöschten Nachricht wie in der 200, weniger als 1 s nach der 200; Neustart → gleicher
/// Verlauf.
#[tokio::test]
async fn das_loeschen_kommt_live_an() {
    let wurzel = ordner();
    let beta = Absender::von("127.0.0.1");
    let (antwort, verlauf_vorher) = {
        let instanz = starten(wurzel.path());
        let basis = instanz.adresse.as_str();
        let alpha = beitreten(&beta, basis, "alpha", None).await;
        let (token, t1) = beta_mit_hallo(&beta, basis).await;
        let mut strom = Strom::oeffnen(&beta, basis, &alpha).await;

        let antwort = loeschen(&beta, basis, &token, &t1).await;
        let erhalten = Instant::now();
        let ereignis = strom.bis(Duration::from_secs(10), ist_aenderung).await;
        strom.mitlesen(STILLE).await;

        assert_eq!(antwort.status, 200, "{}", antwort.json);
        let ereignis = ereignis.expect("aenderung");
        assert_eq!(
            ereignis.data,
            json!({"art": "aenderung", "channel": "allgemein", "nachricht": {
                "zeit": t1, "von": "beta@127.0.0.1", "art": "agent", "text": "", "zustand": "geloescht",
                "geaendert": antwort.json["nachricht"]["geaendert"], "anhaenge": []}})
        );
        let dauer = ereignis.ankunft.saturating_duration_since(erhalten);
        println!("Ereignis nach der 200: {dauer:?} (Grenze {GRENZE:?})");
        assert!(dauer < GRENZE, "{dauer:?}");
        assert_eq!(
            strom.ereignisse.iter().filter(|e| ist_aenderung(e)).count(),
            1
        );
        let jetzt = verlauf(&beta, basis, &token, "allgemein").await;
        (antwort, jetzt)
    };
    let instanz = starten(wurzel.path());
    let token = beitreten(&beta, &instanz.adresse, "beta", Some("agent")).await;

    let nach_neustart = verlauf(&beta, &instanz.adresse, &token, "allgemein").await;

    assert_eq!(nach_neustart["nachrichten"], verlauf_vorher["nachrichten"]);
    assert_eq!(nachrichten_im(&nach_neustart)[0], antwort.json["nachricht"]);
}

/// R00040 „Körper“ (M10, M12): doppeltes `zeit` → 422 `feld`/`doppelt`; `bestaetigt` → 422 `feld`; 1.025 Bytes → 413
/// vor dem Parse; `text/plain` → 415; fremder `Origin` → 403; ohne Sitzung → 401; `"zeit":"gestern"` → 422 `zeit`;
/// jeweils Tageslog unverändert.
#[tokio::test]
async fn falsche_koerper_werden_abgewiesen_ohne_zu_schreiben() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path());
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let (token, t1) = beta_mit_hallo(&beta, basis).await;
    let vorher = channels_stand(wurzel.path());
    let gross = format!(
        r#"{{"zeit":"{t1}","x":"{}"}}"#,
        "a".repeat(1_025 - 18 - t1.len())
    );
    assert_eq!(gross.len(), 1_025);
    let ok = format!(r#"{{"zeit":"{t1}"}}"#);
    let faelle: Vec<(bool, &str, String, Vec<(&str, &str)>, u16)> = vec![
        (
            true,
            "application/json",
            format!(r#"{{"zeit":"{t1}","zeit":"{t1}"}}"#),
            vec![],
            422,
        ),
        (
            true,
            "application/json",
            format!(r#"{{"zeit":"{t1}","bestaetigt":true}}"#),
            vec![],
            422,
        ),
        (true, "application/json", gross, vec![], 413),
        (true, "text/plain", ok.clone(), vec![], 415),
        (
            true,
            "application/json",
            ok.clone(),
            vec![("Origin", "http://evil.example")],
            403,
        ),
        (false, "application/json", ok.clone(), vec![], 401),
        (
            true,
            "application/json",
            r#"{"zeit":"gestern"}"#.to_string(),
            vec![],
            422,
        ),
        (true, "application/json", "{".to_string(), vec![], 400),
    ];
    let mut antworten = Vec::new();
    for (mit_token, typ, koerper, koepfe, _) in &faelle {
        let token = mit_token.then_some(token.as_str());
        antworten.push(
            loeschen_roh(
                &beta,
                basis,
                token,
                "allgemein",
                typ,
                koerper.clone().into_bytes(),
                koepfe,
            )
            .await,
        );
    }

    for (antwort, (.., status)) in antworten.iter().zip(&faelle) {
        assert_eq!(antwort.status, *status, "{}", antwort.json);
    }
    assert_eq!(
        antworten[0].json,
        json!({
            "fehler": "Das Feld zeit steht zweimal im Körper — jedes Feld nur einmal schicken.",
            "regel": "feld", "feld": "zeit", "grund": "doppelt"
        })
    );
    assert_eq!(antworten[1].json["regel"], "feld");
    assert_eq!(antworten[1].json["feld"], "bestaetigt");
    assert_eq!(antworten[6].json["regel"], "zeit");
    assert_eq!(channels_stand(wurzel.path()), vorher);
}

/// R00040 „Kosten“: ein Löschen kostet 1 Einheit — ist der Vorrat von `beta` mit Sendungen in einem Zug aufgebraucht,
/// antwortet Löschen 429 mit `Retry-After`, nichts geschrieben.
#[tokio::test]
async fn loeschen_bucht_am_schreibkonto() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path());
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let (token, t1) = beta_mit_hallo(&beta, basis).await;
    let mut gebremst = false;
    for nummer in 0..30 {
        let antwort = senden(&beta, basis, &token, "allgemein", &format!("s{nummer}")).await;
        gebremst |= antwort.status == 429;
    }
    while !gebremst {
        gebremst = senden(&beta, basis, &token, "allgemein", "noch eine")
            .await
            .status
            == 429;
    }
    let vorher = channels_stand(wurzel.path());

    let antwort = loeschen(&beta, basis, &token, &t1).await;

    assert_eq!(antwort.status, 429, "{}", antwort.json);
    assert!(antwort.kopf.get("retry-after").is_some());
    assert_eq!(channels_stand(wurzel.path()), vorher);
}

/// R00040 „Plattenquote im echten Programm“ (US-6, M9; ADR-0048 Integrationstest „Quote voll → Bearbeiten 507, Löschen
/// 200, bis R erschöpft → 507“): bis 300 Bytes unter `hoechstens` gefüllt; `Hallo` (t1) und `Welt` (t2); Bearbeiten
/// von t1 mit 400 Bytes → 507; Löschen von t1 → 200. Neustart mit `--plattenquote 5MiB` (Grenze 6.291.456 <
/// `belegt`) → Löschen von t2 → 507 „Nicht gelöscht: …“, Tageslog unverändert.
#[tokio::test]
async fn an_der_vollen_quote_loescht_es_bis_zur_reserve() {
    let wurzel = ordner();
    let jetzt = localchata::uhr::Zeit::parse_from_rfc3339(JETZT).expect("jetzt");
    let berlin = |zeitpunkt: chrono::DateTime<chrono::Utc>| {
        zeitpunkt.with_timezone(&chrono::FixedOffset::east_opt(3600).expect("Versatz"))
    };
    localchata::testdaten::quote_ordner(
        wurzel.path(),
        "allgemein",
        10_485_760 - 300,
        jetzt,
        berlin,
    )
    .expect("Quote-Ordner");
    let beta = Absender::von("127.0.0.1");
    let t2 = {
        let instanz = starten(wurzel.path());
        let basis = instanz.adresse.as_str();
        let (token, t1) = beta_mit_hallo(&beta, basis).await;
        let t2 = zeit_von(&senden(&beta, basis, &token, "allgemein", "Welt").await);

        let bearbeitet = bearbeiten(&beta, basis, &token, &t1, &"q".repeat(400)).await;
        let geloescht = loeschen(&beta, basis, &token, &t1).await;

        assert_eq!(bearbeitet.status, 507, "{}", bearbeitet.json);
        assert_eq!(geloescht.status, 200, "{}", geloescht.json);
        t2
    };
    let instanz = starten_mit(wurzel.path(), "5MiB");
    let basis = instanz.adresse.as_str();
    let token = beitreten(&beta, basis, "beta", Some("agent")).await;
    let vorher = channels_stand(wurzel.path());

    let antwort = loeschen(&beta, basis, &token, &t2).await;

    assert_eq!(antwort.status, 507, "{}", antwort.json);
    assert_eq!(antwort.json["regel"], "quote");
    assert_eq!(antwort.json["hoechstens"], 5_242_880);
    let fehler = antwort.json["fehler"].as_str().expect("fehler");
    assert!(
        fehler.starts_with("Nicht gelöscht: die Gespräche belegen "),
        "{fehler}"
    );
    assert!(fehler.contains("--plattenquote"), "{fehler}");
    assert_eq!(channels_stand(wurzel.path()), vorher);
}

/// R00040 „Wettlauf Bearbeiten und Löschen“ (M5, Integration): fünf Paare gleichzeitig → jede Nachricht endet
/// `geloescht`, jedes Bearbeiten ist 200 (davor) oder 409 (danach), im Log nie ein Änderungssatz nach dem Löschsatz;
/// Neustart → gleicher Verlauf.
#[tokio::test]
async fn bearbeiten_und_loeschen_gleichzeitig() {
    let wurzel = ordner();
    let beta = Absender::von("127.0.0.1");
    let vorher = {
        let instanz = starten(wurzel.path());
        let basis = instanz.adresse.as_str();
        let token = beitreten(&beta, basis, "beta", Some("agent")).await;
        let mut zeiten = Vec::new();
        for nummer in 0..5 {
            zeiten.push(zeit_von(
                &senden(&beta, basis, &token, "allgemein", &format!("n{nummer}")).await,
            ));
        }

        for zeit in &zeiten {
            let (bearbeitet, geloescht) = tokio::join!(
                bearbeiten(&beta, basis, &token, zeit, "neu"),
                loeschen(&beta, basis, &token, zeit)
            );
            assert_eq!(geloescht.status, 200, "{}", geloescht.json);
            assert!(
                [200, 409].contains(&bearbeitet.status),
                "{} {}",
                bearbeitet.status,
                bearbeitet.json
            );
        }

        let jetzt = verlauf(&beta, basis, &token, "allgemein").await;
        assert!(
            nachrichten_im(&jetzt)
                .iter()
                .all(|nachricht| nachricht["zustand"] == "geloescht" && nachricht["text"] == ""),
            "{jetzt}"
        );
        let log = tagesdatei(wurzel.path());
        for zeit in &zeiten {
            let stelle = log
                .find(&format!("+ geloescht: {zeit}\n"))
                .expect("Löschsatz");
            assert!(
                !log[stelle..].contains(&format!("+ bearbeitet: {zeit}\n")),
                "{log}"
            );
        }
        jetzt
    };
    let instanz = starten(wurzel.path());
    let token = beitreten(&beta, &instanz.adresse, "beta", Some("agent")).await;

    let nach_neustart = verlauf(&beta, &instanz.adresse, &token, "allgemein").await;

    assert_eq!(nach_neustart["nachrichten"], vorher["nachrichten"]);
}

/// R00040 „Von Hand geschrieben“: nach einem Löschsatz zu t1 steht ein `bearbeitet`-Satz derselben Kennung zu t1 →
/// beim Start `WARNING laden` mit Datei und Zeile, t1 bleibt `geloescht`.
#[tokio::test]
async fn eine_bearbeitung_nach_dem_loeschsatz_gilt_nicht() {
    let wurzel = ordner();
    let t1 = "2026-10-28T11:00:00.000+01:00";
    std::fs::write(
        wurzel
            .path()
            .join(format!("channels/allgemein/{HEUTE}.log")),
        format!(
            "@ {t1} beta@127.0.0.1\n+ art: agent\n  Hallo\n\n\
             @ 2026-10-28T11:00:01.000+01:00 beta@127.0.0.1\n+ geloescht: {t1}\n\n\
             @ 2026-10-28T11:00:02.000+01:00 beta@127.0.0.1\n+ bearbeitet: {t1}\n  wieder da\n\n"
        ),
    )
    .expect("Tagesdatei");
    let instanz = starten(wurzel.path());
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, basis, "beta", Some("agent")).await;

    let jetzt = verlauf(&beta, basis, &token, "allgemein").await;

    assert_eq!(nachrichten_im(&jetzt).len(), 1, "{jetzt}");
    assert_eq!(nachrichten_im(&jetzt)[0]["text"], "");
    assert_eq!(nachrichten_im(&jetzt)[0]["zustand"], "geloescht");
    let protokoll = std::fs::read_to_string(wurzel.path().join(format!("logs/{HEUTE}.log")))
        .expect("Protokoll");
    let zeile = protokoll
        .lines()
        .find(|zeile| zeile.contains(" WARNING laden "))
        .expect("WARNING laden");
    assert!(
        zeile.contains(&format!("datei=channels/allgemein/{HEUTE}.log"))
            && zeile.contains("zeile=8"),
        "{zeile}"
    );
}
