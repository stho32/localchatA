//! R00039 F0132 — eigene Nachricht über die API bearbeiten (ADR-0048 Regeln 1–8, 10, 13): das echte Programm mit
//! `channels/allgemein/` und `channels/technik/`, `--plattenquote 10MiB`; `beta` sendet `Hallo` (Zeit `t1`).

mod gemeinsam;

use gemeinsam::senden::{beitreten, ordner, senden, verlauf};
use gemeinsam::{Absender, ApiAntwort, Instanz, api_antwort};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const JETZT: &str = "2026-10-28T12:00:00.000+01:00";
const HEUTE: &str = "2026-10-28";

/// Startet mit `TZ=Europe/Berlin`, der Uhr ab `jetzt` und `--plattenquote 10MiB`, dazu weitere Optionen.
fn starten_um(ordner: &Path, jetzt: &str, weitere: &[&str], umgebung: &[(&str, &str)]) -> Instanz {
    let mut argumente = vec![
        "--port",
        "0",
        "--host",
        "127.0.0.1",
        "--kein-browser",
        "--plattenquote",
        "10MiB",
    ];
    argumente.extend_from_slice(weitere);
    let mut alle = vec![("TZ", "Europe/Berlin"), ("LOCALCHATA_JETZT", jetzt)];
    alle.extend_from_slice(umgebung);
    gemeinsam::starten_mit(ordner, &argumente, &alle)
}

fn starten(ordner: &Path) -> Instanz {
    starten_um(ordner, JETZT, &[], &[])
}

/// `POST …/channels/<channel>/nachrichten/bearbeiten` mit Rohkörper, Typ, optionalem Token und weiteren Köpfen.
async fn bearbeiten_roh(
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
            format!("{basis}api/channels/{channel}/nachrichten/bearbeiten"),
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

/// Bearbeiten in `# allgemein` mit `{"zeit": zeit, "text": text}`.
async fn bearbeiten(
    absender: &Absender,
    basis: &str,
    token: &str,
    zeit: &str,
    text: &str,
) -> ApiAntwort {
    bearbeiten_roh(
        absender,
        basis,
        Some(token),
        "allgemein",
        "application/json",
        json!({ "zeit": zeit, "text": text })
            .to_string()
            .into_bytes(),
        &[],
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

fn tagesdatei(ordner: &Path, tag: &str) -> String {
    std::fs::read_to_string(ordner.join(format!("channels/allgemein/{tag}.log")))
        .unwrap_or_default()
}

/// Beta ist beigetreten und hat `Hallo` gesendet: Token und `t1`.
async fn beta_mit_hallo(beta: &Absender, basis: &str) -> (String, String) {
    let token = beitreten(beta, basis, "beta", Some("agent")).await;
    let gesendet = senden(beta, basis, &token, "allgemein", "Hallo").await;
    assert_eq!(gesendet.status, 201, "{}", gesendet.json);
    let t1 = gesendet.json["nachricht"]["zeit"]
        .as_str()
        .expect("zeit")
        .to_string();
    (token, t1)
}

fn nachrichten_im(verlauf: &Value) -> &Vec<Value> {
    verlauf["nachrichten"].as_array().expect("nachrichten")
}

/// R00039 „Rundweg“ und „Lesbar“ (QE5): 200 mit neuem Text, `bearbeitet` und `geaendert` > t1; der Verlauf hat genau
/// eine Nachricht t1 mit dem neuen Text, Health zählt 1; die Tagesdatei zeigt unter eigener Kopfzeile `+ bearbeitet:
/// <t1>` und eingerückt den Text, `Hallo` steht darüber unverändert; nach einem Neustart derselbe Verlauf.
#[tokio::test]
async fn rundweg_mit_neustart_und_lesbarem_log() {
    let wurzel = ordner();
    let beta = Absender::von("127.0.0.1");
    let (antwort, vorher_verlauf) = {
        let instanz = starten(wurzel.path());
        let basis = instanz.adresse.as_str();
        let (token, t1) = beta_mit_hallo(&beta, basis).await;

        let antwort = bearbeiten(&beta, basis, &token, &t1, "Hallo zusammen").await;

        assert_eq!(antwort.status, 200, "{}", antwort.json);
        assert_eq!(antwort.json["channel"], "allgemein");
        let nachricht = &antwort.json["nachricht"];
        assert_eq!(nachricht["zeit"], t1.as_str());
        assert_eq!(nachricht["von"], "beta@127.0.0.1");
        assert_eq!(nachricht["art"], "agent");
        assert_eq!(nachricht["text"], "Hallo zusammen");
        assert_eq!(nachricht["zustand"], "bearbeitet");
        assert_eq!(nachricht["anhaenge"], json!([]));
        let geaendert = nachricht["geaendert"].as_str().expect("geaendert");
        let zeitpunkt = |text: &str| chrono::DateTime::parse_from_rfc3339(text).expect("Zeit");
        assert!(zeitpunkt(geaendert) > zeitpunkt(&t1), "{geaendert} > {t1}");
        let jetzt = verlauf(&beta, basis, &token, "allgemein").await;
        assert_eq!(nachrichten_im(&jetzt).len(), 1, "{jetzt}");
        assert_eq!(nachrichten_im(&jetzt)[0], *nachricht);
        let (_, gesundheit) = gemeinsam::gesundheit(basis).await;
        assert_eq!(gesundheit["nachrichten"], 1, "{gesundheit}");
        let log = tagesdatei(wurzel.path(), HEUTE);
        let erwartet = format!(
            "@ {t1} beta@127.0.0.1\n+ art: agent\n  Hallo\n\n@ {geaendert} beta@127.0.0.1\n+ bearbeitet: {t1}\n  Hallo zusammen\n\n"
        );
        assert_eq!(log, erwartet);
        (antwort, jetzt)
    };
    let instanz = starten(wurzel.path());
    let basis = instanz.adresse.as_str();
    let token = beitreten(&beta, basis, "beta", Some("agent")).await;

    let nach_dem_neustart = verlauf(&beta, basis, &token, "allgemein").await;

    assert_eq!(
        nachrichten_im(&nach_dem_neustart),
        nachrichten_im(&vorher_verlauf)
    );
    assert_eq!(
        nachrichten_im(&nach_dem_neustart)[0],
        antwort.json["nachricht"]
    );
}

/// R00039 „Spätere Sitzung derselben Kennung“ (Richtungsfrage 6, M2): `beta` verlässt und tritt erneut bei → das
/// neue Token bearbeitet t1 mit 200.
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
    assert!(
        verlassen.status < 300,
        "{} {}",
        verlassen.status,
        verlassen.json
    );
    let neu = beitreten(&beta, basis, "beta", Some("agent")).await;

    let antwort = bearbeiten(&beta, basis, &neu, &t1, "Hallo allerseits").await;

    assert_eq!(antwort.status, 200, "{}", antwort.json);
    assert_eq!(antwort.json["nachricht"]["text"], "Hallo allerseits");
}

/// R00039 „Änderung an einem späteren Tag, Neustart über drei Tage“ (Pre-Mortem 1): t1 am 27.10. um 23:59:58,
/// bearbeitet am 28. und am 29. — je Satz in der Datei seines Tages, der 27. byte-gleich; nach einem weiteren Neustart
/// zeigt der Verlauf `Dritter`.
#[tokio::test]
async fn aenderungen_an_spaeteren_tagen_ueber_drei_neustarts() {
    let wurzel = ordner();
    let beta = Absender::von("127.0.0.1");
    let t1 = {
        let instanz = starten_um(wurzel.path(), "2026-10-27T23:59:58.000+01:00", &[], &[]);
        let (_, t1) = beta_mit_hallo(&beta, &instanz.adresse).await;
        t1
    };
    let tag_27 = tagesdatei(wurzel.path(), "2026-10-27");
    assert!(tag_27.contains("  Hallo\n"), "{tag_27}");
    for (jetzt, text) in [
        ("2026-10-28T10:00:00.000+01:00", "Zweiter"),
        ("2026-10-29T10:00:00.000+01:00", "Dritter"),
    ] {
        let instanz = starten_um(wurzel.path(), jetzt, &[], &[]);
        let token = beitreten(&beta, &instanz.adresse, "beta", Some("agent")).await;
        let antwort = bearbeiten(&beta, &instanz.adresse, &token, &t1, text).await;
        assert_eq!(antwort.status, 200, "{jetzt}: {}", antwort.json);
    }
    let instanz = starten_um(wurzel.path(), "2026-10-29T11:00:00.000+01:00", &[], &[]);
    let token = beitreten(&beta, &instanz.adresse, "beta", Some("agent")).await;

    let jetzt = verlauf(&beta, &instanz.adresse, &token, "allgemein").await;

    assert_eq!(tagesdatei(wurzel.path(), "2026-10-27"), tag_27);
    let tag_28 = tagesdatei(wurzel.path(), "2026-10-28");
    let tag_29 = tagesdatei(wurzel.path(), "2026-10-29");
    assert!(
        tag_28.contains(&format!("+ bearbeitet: {t1}\n  Zweiter\n")),
        "{tag_28}"
    );
    assert!(
        tag_29.contains(&format!("+ bearbeitet: {t1}\n  Dritter\n")),
        "{tag_29}"
    );
    assert_eq!(nachrichten_im(&jetzt).len(), 1, "{jetzt}");
    assert_eq!(nachrichten_im(&jetzt)[0]["zeit"], t1.as_str());
    assert_eq!(nachrichten_im(&jetzt)[0]["text"], "Dritter");
    assert_eq!(nachrichten_im(&jetzt)[0]["zustand"], "bearbeitet");
}

/// R00039 „Fremde Kennung“ (M1): `gamma` → 403 `fremd`, der Text beginnt mit „Nur der Absender“ und nennt
/// `beta@127.0.0.1`; Tageslog und Verlauf unverändert.
#[tokio::test]
async fn eine_fremde_kennung_wird_abgewiesen() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path());
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let (token, t1) = beta_mit_hallo(&beta, basis).await;
    let gamma = beitreten(&beta, basis, "gamma", None).await;
    let vorher = channels_stand(wurzel.path());

    let antwort = bearbeiten(&beta, basis, &gamma, &t1, "gefälscht").await;

    assert_eq!(antwort.status, 403, "{}", antwort.json);
    assert_eq!(antwort.json["regel"], "fremd");
    let fehler = antwort.json["fehler"].as_str().expect("fehler");
    assert!(fehler.starts_with("Nur der Absender"), "{fehler}");
    assert!(fehler.contains("beta@127.0.0.1"), "{fehler}");
    assert_eq!(channels_stand(wurzel.path()), vorher);
    let jetzt = verlauf(&beta, basis, &token, "allgemein").await;
    assert_eq!(nachrichten_im(&jetzt)[0]["text"], "Hallo");
    assert_eq!(nachrichten_im(&jetzt)[0]["zustand"], Value::Null);
    assert_eq!(nachrichten_im(&jetzt)[0]["geaendert"], Value::Null);
}

/// R00039 „Nachricht nicht da“ (M3, M4): t1 + 1 ms → 404 `nachricht`; t1 an `# technik` → 404 `nachricht`; ein
/// unbekannter Channel → 404 wie für Channels; jeweils nichts geschrieben.
#[tokio::test]
async fn eine_zeit_ohne_nachricht_in_diesem_channel_ist_404() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path());
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let (token, t1) = beta_mit_hallo(&beta, basis).await;
    let danach = (chrono::DateTime::parse_from_rfc3339(&t1).expect("t1")
        + chrono::Duration::milliseconds(1))
    .to_rfc3339_opts(chrono::SecondsFormat::Millis, false);
    let vorher = channels_stand(wurzel.path());
    let koerper = |zeit: &str| {
        json!({ "zeit": zeit, "text": "x" })
            .to_string()
            .into_bytes()
    };
    let an = |channel: &'static str, zeit: &str| {
        let koerper = koerper(zeit);
        let beta = &beta;
        let token = token.as_str();
        async move {
            bearbeiten_roh(
                beta,
                basis,
                Some(token),
                channel,
                "application/json",
                koerper,
                &[],
            )
            .await
        }
    };

    let ohne = an("allgemein", &danach).await;
    let technik = an("technik", &t1).await;
    let unbekannt = an("gibtsnicht", &t1).await;

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
}

/// R00039 „Schreibweise der Zeit“: t1 wie geliefert, derselbe Zeitpunkt in UTC und mit `+` als `+` → jeweils
/// derselbe Treffer (200); `"gestern"`, `5` und ohne `zeit` → 422 `zeit`.
#[tokio::test]
async fn die_zeit_gilt_als_zeitpunkt_in_jeder_schreibweise() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path());
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let (token, t1) = beta_mit_hallo(&beta, basis).await;
    let utc = chrono::DateTime::parse_from_rfc3339(&t1)
        .expect("t1")
        .with_timezone(&chrono::Utc)
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string();
    let kodiert = format!(
        r#"{{"zeit":"{}","text":"drei"}}"#,
        t1.replace('+', "\\u002B")
    );
    let roh = |koerper: Vec<u8>| {
        bearbeiten_roh(
            &beta,
            basis,
            Some(&token),
            "allgemein",
            "application/json",
            koerper,
            &[],
        )
    };

    let wie_geliefert = bearbeiten(&beta, basis, &token, &t1, "eins").await;
    let in_utc = bearbeiten(&beta, basis, &token, &utc, "zwei").await;
    let mit_folge = roh(kodiert.into_bytes()).await;
    let gestern = roh(br#"{"zeit":"gestern","text":"x"}"#.to_vec()).await;
    let zahl = roh(br#"{"zeit":5,"text":"x"}"#.to_vec()).await;
    let ohne = roh(br#"{"text":"x"}"#.to_vec()).await;

    for (antwort, text) in [
        (&wie_geliefert, "eins"),
        (&in_utc, "zwei"),
        (&mit_folge, "drei"),
    ] {
        assert_eq!(antwort.status, 200, "{text}: {}", antwort.json);
        assert_eq!(antwort.json["nachricht"]["zeit"], t1.as_str());
        assert_eq!(antwort.json["nachricht"]["text"], text);
    }
    for antwort in [&gestern, &zahl, &ohne] {
        assert_eq!(antwort.status, 422, "{}", antwort.json);
        assert_eq!(antwort.json["regel"], "zeit");
    }
}

/// R00039 „Körper“ (M10, M12): doppeltes `zeit` bzw. `text` → 422 `feld`/`doppelt`; ein weiteres Feld → 422 `feld`;
/// 131.073 Bytes → 413; `text/plain` → 415; fremder Ursprung → 403; ohne Sitzung → 401; jedes Mal nichts geschrieben.
#[tokio::test]
async fn falsche_koerper_werden_abgewiesen_ohne_zu_schreiben() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path());
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let (token, t1) = beta_mit_hallo(&beta, basis).await;
    let vorher = channels_stand(wurzel.path());
    let gross = format!(
        r#"{{"zeit":"{t1}","text":"{}"}}"#,
        "a".repeat(131_073 - 21 - t1.len())
    );
    assert_eq!(gross.len(), 131_073);
    let fall =
        |token: Option<&'static str>,
         typ: &'static str,
         koerper: String,
         koepfe: Vec<(&'static str, &'static str)>| { (token, typ, koerper, koepfe) };
    let faelle = vec![
        (
            fall(
                None,
                "application/json",
                format!(r#"{{"zeit":"{t1}","zeit":"{t1}","text":"x"}}"#),
                vec![],
            ),
            422,
        ),
        (
            fall(
                None,
                "application/json",
                format!(r#"{{"zeit":"{t1}","text":"x","text":"y"}}"#),
                vec![],
            ),
            422,
        ),
        (
            fall(
                None,
                "application/json",
                format!(r#"{{"zeit":"{t1}","text":"x","anhaenge":[]}}"#),
                vec![],
            ),
            422,
        ),
        (fall(None, "application/json", gross, vec![]), 413),
        (
            fall(
                None,
                "text/plain",
                format!(r#"{{"zeit":"{t1}","text":"x"}}"#),
                vec![],
            ),
            415,
        ),
        (
            fall(
                None,
                "application/json",
                format!(r#"{{"zeit":"{t1}","text":"x"}}"#),
                vec![("Origin", "http://evil.example")],
            ),
            403,
        ),
        (
            fall(
                Some("ohne"),
                "application/json",
                format!(r#"{{"zeit":"{t1}","text":"x"}}"#),
                vec![],
            ),
            401,
        ),
    ];
    let mut antworten = Vec::new();
    for ((ohne, typ, koerper, koepfe), _) in &faelle {
        let token = if ohne.is_some() {
            None
        } else {
            Some(token.as_str())
        };
        antworten.push(
            bearbeiten_roh(
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

    for (antwort, (_, status)) in antworten.iter().zip(&faelle) {
        assert_eq!(antwort.status, *status, "{}", antwort.json);
    }
    assert_eq!(
        antworten[0].json,
        json!({
            "fehler": "Das Feld zeit steht zweimal im Körper — jedes Feld nur einmal schicken.",
            "regel": "feld", "feld": "zeit", "grund": "doppelt"
        })
    );
    assert_eq!(antworten[1].json["feld"], "text");
    assert_eq!(antworten[1].json["grund"], "doppelt");
    assert_eq!(antworten[2].json["regel"], "feld");
    assert_eq!(antworten[2].json["feld"], "anhaenge");
    assert_eq!(channels_stand(wurzel.path()), vorher);
}

/// R00039 „Textregeln“: 16.385 Bytes → 422 `laenge`; `\u0007` → 422 `zeichen`; `a\r\nb` → 200 mit `a\nb`; nur
/// Leerraum ohne Anhang → 422 `leer`; nur Leerraum an einer Nachricht mit Anhang → 200 mit `""`, der Anhang bleibt
/// `vorhanden`.
#[tokio::test]
async fn die_textregeln_gelten_wie_beim_senden() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path());
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let (token, t1) = beta_mit_hallo(&beta, basis).await;
    let id =
        gemeinsam::anhaenge::ablegen(&beta, basis, &token, "log.txt", b"Zeile 1\n".to_vec()).await;
    let mit_anhang = api_antwort(
        beta.anfrage(
            reqwest::Method::POST,
            format!("{basis}api/channels/allgemein/nachrichten"),
        )
        .header("X-LocalchatA-Sitzung", &token)
        .json(&json!({ "text": "mit Log", "anhaenge": [id] })),
    )
    .await;
    assert_eq!(mit_anhang.status, 201, "{}", mit_anhang.json);
    let t_anhang = mit_anhang.json["nachricht"]["zeit"]
        .as_str()
        .expect("zeit")
        .to_string();

    let lang = bearbeiten(&beta, basis, &token, &t1, &"a".repeat(16_385)).await;
    let klingel = bearbeiten(&beta, basis, &token, &t1, "a\u{7}b").await;
    let umbruch = bearbeiten(&beta, basis, &token, &t1, "a\r\nb").await;
    let leer = bearbeiten(&beta, basis, &token, &t1, "   ").await;
    let leer_mit_anhang = bearbeiten(&beta, basis, &token, &t_anhang, "   ").await;

    assert_eq!(
        (lang.status, &lang.json["regel"]),
        (422, &json!("laenge")),
        "{}",
        lang.json
    );
    assert_eq!(
        (klingel.status, &klingel.json["regel"]),
        (422, &json!("zeichen")),
        "{}",
        klingel.json
    );
    assert_eq!(umbruch.status, 200, "{}", umbruch.json);
    assert_eq!(umbruch.json["nachricht"]["text"], "a\nb");
    assert_eq!(
        (leer.status, &leer.json["regel"]),
        (422, &json!("leer")),
        "{}",
        leer.json
    );
    assert_eq!(leer_mit_anhang.status, 200, "{}", leer_mit_anhang.json);
    let nachricht = &leer_mit_anhang.json["nachricht"];
    assert_eq!(nachricht["text"], "");
    assert_eq!(nachricht["zustand"], "bearbeitet");
    assert_eq!(nachricht["anhaenge"][0]["id"], id.as_str());
    assert_eq!(nachricht["anhaenge"][0]["zustand"], "vorhanden");
    let jetzt = verlauf(&beta, basis, &token, "allgemein").await;
    assert_eq!(nachrichten_im(&jetzt)[1], *nachricht);
}

/// R00039 „Text täuscht Metazeilen vor“ (M14): 200; im Tageslog sind die Zeilen eingerückt; nach einem Neustart ist
/// t1 weiter `bearbeitet` mit genau diesem Text, keine Nachricht von `gamma`, keine `WARNING laden`.
#[tokio::test]
async fn ein_text_mit_vorgetaeuschten_metazeilen_bleibt_text() {
    let wurzel = ordner();
    let beta = Absender::von("127.0.0.1");
    let (t1, text) = {
        let instanz = starten(wurzel.path());
        let basis = instanz.adresse.as_str();
        let (token, t1) = beta_mit_hallo(&beta, basis).await;
        let text =
            format!("x\n+ geloescht: {t1}\n@ 2026-10-28T15:00:00.000+01:00 gamma@127.0.0.1\n  y");
        let antwort = bearbeiten(&beta, basis, &token, &t1, &text).await;
        assert_eq!(antwort.status, 200, "{}", antwort.json);
        (t1, text)
    };
    let log = tagesdatei(wurzel.path(), HEUTE);
    assert!(
        log.contains(&format!(
            "  x\n  + geloescht: {t1}\n  @ 2026-10-28T15:00:00.000+01:00 gamma@127.0.0.1\n    y\n"
        )),
        "{log}"
    );
    let instanz = starten(wurzel.path());
    let basis = instanz.adresse.as_str();
    let token = beitreten(&beta, basis, "beta", Some("agent")).await;

    let jetzt = verlauf(&beta, basis, &token, "allgemein").await;

    assert_eq!(nachrichten_im(&jetzt).len(), 1, "{jetzt}");
    assert_eq!(nachrichten_im(&jetzt)[0]["text"], text.as_str());
    assert_eq!(nachrichten_im(&jetzt)[0]["zustand"], "bearbeitet");
    let protokoll = std::fs::read_to_string(wurzel.path().join(format!("logs/{HEUTE}.log")))
        .unwrap_or_default();
    assert!(!protokoll.contains(" WARNING laden "), "{protokoll}");
}

/// R00039 „Unverändert“: der aktuelle Text noch einmal → 200 mit unverändertem `geaendert`; Tageslog unverändert.
#[tokio::test]
async fn der_gleiche_text_aendert_nichts() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path());
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let (token, t1) = beta_mit_hallo(&beta, basis).await;
    let erste = bearbeiten(&beta, basis, &token, &t1, "Hallo zusammen").await;
    assert_eq!(erste.status, 200, "{}", erste.json);
    let vorher = channels_stand(wurzel.path());

    let gleich = bearbeiten(&beta, basis, &token, &t1, "Hallo zusammen").await;

    assert_eq!(gleich.status, 200, "{}", gleich.json);
    assert_eq!(gleich.json, erste.json);
    assert_eq!(channels_stand(wurzel.path()), vorher);
}

/// R00039 „Schreibkonto“ (M8): Bearbeiten bucht wie Senden — ist der Vorrat von `beta` mit Sendungen in einem Zug
/// aufgebraucht, antwortet ein Bearbeiten mit 10.000 Bytes (3 Einheiten) 429 mit `Retry-After`; nichts geschrieben.
#[tokio::test]
async fn bearbeiten_bucht_am_schreibkonto() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path());
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let (token, t1) = beta_mit_hallo(&beta, basis).await;
    let mut gebremst = false;
    for nummer in 0..29 {
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

    let antwort = bearbeiten(&beta, basis, &token, &t1, &"z".repeat(10_000)).await;

    assert_eq!(antwort.status, 429, "{}", antwort.json);
    assert!(antwort.kopf.get("retry-after").is_some());
    assert_eq!(channels_stand(wurzel.path()), vorher);
}

/// R00039 „Plattenquote“ (M9 Teil Bearbeiten): der Channel ist bis 300 Bytes unter `hoechstens` gefüllt; `Hallo` geht,
/// Bearbeiten mit 400 Bytes → 507 `quote`, der Text beginnt mit „Nicht bearbeitet: die Gespräche belegen “ und nennt
/// `--plattenquote`; Tageslog unverändert.
#[tokio::test]
async fn an_der_vollen_quote_ist_bearbeiten_507() {
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
    let instanz = starten(wurzel.path());
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let (token, t1) = beta_mit_hallo(&beta, basis).await;
    let vorher = channels_stand(wurzel.path());

    let antwort = bearbeiten(&beta, basis, &token, &t1, &"q".repeat(400)).await;

    assert_eq!(antwort.status, 507, "{}", antwort.json);
    assert_eq!(antwort.json["regel"], "quote");
    let fehler = antwort.json["fehler"].as_str().expect("fehler");
    assert!(
        fehler.starts_with("Nicht bearbeitet: die Gespräche belegen "),
        "{fehler}"
    );
    assert!(fehler.contains("--plattenquote"), "{fehler}");
    assert_eq!(channels_stand(wurzel.path()), vorher);
    let protokoll = std::fs::read_to_string(wurzel.path().join(format!("logs/{HEUTE}.log")))
        .expect("Protokoll");
    let quote = protokoll
        .lines()
        .find(|zeile| zeile.contains(" WARNING quote ") && zeile.contains("Episode beginnt"))
        .expect("Quote-Episode");
    assert!(quote.contains("route=bearbeiten"), "{quote}");
}

/// R00039 „Von Hand geschriebener Änderungssatz“ (M13): ein Satz mit Kennung `gamma` bezieht sich auf `beta`s t1 →
/// beim Start `WARNING laden` mit Datei und Zeile; der Verlauf zeigt t1 mit `Hallo` und `zustand: null`.
#[tokio::test]
async fn ein_handgeschriebener_fremder_aenderungssatz_gilt_nicht() {
    let wurzel = ordner();
    let t1 = "2026-10-28T11:00:00.000+01:00";
    std::fs::write(
        wurzel.path().join(format!("channels/allgemein/{HEUTE}.log")),
        format!(
            "@ {t1} beta@127.0.0.1\n+ art: agent\n  Hallo\n\n@ 2026-10-28T11:00:01.000+01:00 gamma@127.0.0.1\n+ bearbeitet: {t1}\n  gefälscht\n\n"
        ),
    )
    .expect("Tagesdatei");
    let instanz = starten(wurzel.path());
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, basis, "beta", Some("agent")).await;

    let jetzt = verlauf(&beta, basis, &token, "allgemein").await;

    assert_eq!(nachrichten_im(&jetzt).len(), 1, "{jetzt}");
    assert_eq!(nachrichten_im(&jetzt)[0]["text"], "Hallo");
    assert_eq!(nachrichten_im(&jetzt)[0]["zustand"], Value::Null);
    let protokoll = std::fs::read_to_string(wurzel.path().join(format!("logs/{HEUTE}.log")))
        .expect("Protokoll");
    let zeile = protokoll
        .lines()
        .find(|zeile| zeile.contains(" WARNING laden "))
        .expect("WARNING laden");
    assert!(
        zeile.contains(&format!("datei=channels/allgemein/{HEUTE}.log"))
            && zeile.contains("zeile=5"),
        "{zeile}"
    );
}

/// R00039 „Wiederholschlüssel“ (M11; ADR-0048 Integrationstest „Wiederholschlüssel aus Senden beim Bearbeiten →
/// 422“): `k1` zweimal → zweimal 200 mit derselben Fassung, genau ein Änderungssatz; `k1` mit anderem Text → 422
/// `schluessel`; `k2` zuerst beim Senden, dann beim Bearbeiten → 422; `k1` von `gamma` an seiner eigenen Nachricht →
/// wie neu.
#[tokio::test]
async fn ein_wiederholschluessel_aendert_nichts_doppelt() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path());
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let (token, t1) = beta_mit_hallo(&beta, basis).await;
    let gamma = beitreten(&beta, basis, "gamma", None).await;
    let t_gamma = senden(&beta, basis, &gamma, "allgemein", "Servus")
        .await
        .json["nachricht"]["zeit"]
        .as_str()
        .expect("zeit")
        .to_string();
    let mit = |token: &str, zeit: &str, text: &str, schluessel: &'static str| {
        let koerper = json!({ "zeit": zeit, "text": text })
            .to_string()
            .into_bytes();
        let token = token.to_string();
        let beta = &beta;
        async move {
            bearbeiten_roh(
                beta,
                basis,
                Some(&token),
                "allgemein",
                "application/json",
                koerper,
                &[("Idempotency-Key", schluessel)],
            )
            .await
        }
    };
    let gesendet = api_antwort(
        beta.anfrage(
            reqwest::Method::POST,
            format!("{basis}api/channels/allgemein/nachrichten"),
        )
        .header("X-LocalchatA-Sitzung", &token)
        .header("Idempotency-Key", "k2")
        .json(&json!({ "text": "Hallo zusammen" })),
    )
    .await;
    assert_eq!(gesendet.status, 201, "{}", gesendet.json);

    let erste = mit(&token, &t1, "Hallo zusammen", "k1").await;
    let zweite = mit(&token, &t1, "Hallo zusammen", "k1").await;
    let anders = mit(&token, &t1, "Hallo allerseits", "k1").await;
    let vom_senden = mit(&token, &t1, "Hallo zusammen", "k2").await;
    let von_gamma = mit(&gamma, &t_gamma, "Servus zusammen", "k1").await;

    assert_eq!(erste.status, 200, "{}", erste.json);
    assert_eq!(zweite.status, 200, "{}", zweite.json);
    assert_eq!(zweite.json, erste.json);
    for antwort in [&anders, &vom_senden] {
        assert_eq!(antwort.status, 422, "{}", antwort.json);
        assert_eq!(antwort.json["regel"], "schluessel");
    }
    assert_eq!(von_gamma.status, 200, "{}", von_gamma.json);
    let log = tagesdatei(wurzel.path(), HEUTE);
    assert_eq!(
        log.matches(&format!("+ bearbeitet: {t1}\n")).count(),
        1,
        "{log}"
    );
    assert_eq!(
        log.matches(&format!("+ bearbeitet: {t_gamma}\n")).count(),
        1,
        "{log}"
    );
}
