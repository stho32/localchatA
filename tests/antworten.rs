//! R00047 F0157 — mit Bezug antworten über die API (ADR-0053 Regeln 1–3, 10, 11): das echte Programm mit
//! `channels/allgemein/` und `channels/technik/`, Uhr ab 2026-10-28 12:00, `--plattenquote 10MiB`; `claude` und
//! `codex` sind Agenten derselben Herkunft.

mod gemeinsam;

use gemeinsam::senden::{JETZT, beitreten, ordner, roh_senden, senden, verlauf};
use gemeinsam::{Absender, ApiAntwort, Instanz, api_antwort};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::Path;

const CLAUDE: &str = "claude@127.0.0.1";

fn starten(ordner: &Path) -> Instanz {
    gemeinsam::anhaenge::starten(ordner, &[], &[("LOCALCHATA_JETZT", JETZT)])
}

/// Jede Datei unter `channels/` mit ihren Bytes — gleich heißt: nichts gespeichert.
fn channels_stand(ordner: &Path) -> BTreeMap<String, Vec<u8>> {
    fn sammeln(wurzel: &Path, ordner: &Path, stand: &mut BTreeMap<String, Vec<u8>>) {
        for eintrag in std::fs::read_dir(ordner).expect("Ordner").flatten() {
            let pfad = eintrag.path();
            if pfad.is_dir() {
                sammeln(wurzel, &pfad, stand);
            } else {
                let name = pfad
                    .strip_prefix(wurzel)
                    .expect("unter channels")
                    .display()
                    .to_string();
                stand.insert(name, std::fs::read(&pfad).expect("Datei"));
            }
        }
    }
    let mut stand = BTreeMap::new();
    let wurzel = ordner.join("channels");
    sammeln(&wurzel, &wurzel, &mut stand);
    stand
}

/// `POST …/nachrichten` mit beliebigem JSON-Körper.
async fn senden_json(
    absender: &Absender,
    basis: &str,
    token: &str,
    channel: &str,
    koerper: &Value,
) -> ApiAntwort {
    roh_senden(
        absender,
        basis,
        Some(token),
        channel,
        "application/json",
        koerper.to_string().into_bytes(),
    )
    .await
}

/// Antwort in `# allgemein` mit `{"text": text, "antwort_auf": bezug}`.
async fn antworten(
    absender: &Absender,
    basis: &str,
    token: &str,
    text: &str,
    bezug: Value,
) -> ApiAntwort {
    senden_json(
        absender,
        basis,
        token,
        "allgemein",
        &json!({ "text": text, "antwort_auf": bezug }),
    )
    .await
}

async fn loeschen(
    absender: &Absender,
    basis: &str,
    token: &str,
    channel: &str,
    zeit: &str,
) -> Value {
    let antwort = api_antwort(
        absender
            .anfrage(
                reqwest::Method::POST,
                format!("{basis}api/channels/{channel}/nachrichten/loeschen"),
            )
            .header("Content-Type", "application/json")
            .header("X-LocalchatA-Sitzung", token)
            .body(json!({ "zeit": zeit }).to_string()),
    )
    .await;
    assert_eq!(antwort.status, 200, "{}", antwort.json);
    antwort.json
}

/// Sendet in `channel` und liefert die Zeit der Nachricht.
async fn gesendet(
    absender: &Absender,
    basis: &str,
    token: &str,
    channel: &str,
    text: &str,
) -> String {
    let antwort = senden(absender, basis, token, channel, text).await;
    assert_eq!(antwort.status, 201, "{}", antwort.json);
    antwort.json["nachricht"]["zeit"]
        .as_str()
        .expect("zeit")
        .to_string()
}

async fn belegt(basis: &str) -> u64 {
    gemeinsam::anhaenge::gesundheit(basis).await["log_bytes"]
        .as_u64()
        .expect("log_bytes")
}

/// Zwei Agenten: `claude` und `codex` mit Token.
async fn agenten(absender: &Absender, basis: &str) -> (String, String) {
    (
        beitreten(absender, basis, "claude", Some("agent")).await,
        beitreten(absender, basis, "codex", Some("agent")).await,
    )
}

/// ADR-0053 P3 (a)–(e), S1, S2, Mutanten Z1, Z2, Z7, Z8: gültig → 201 mit dem Zitat, auch mit der Zeit in anderer
/// Schreibweise (im Log steht die des Originals); unbekannt, aus `# technik`, gelöscht → 422 mit `grund`, gleicher
/// Text für (b) und (c); Format → 422 `format`. In (b)–(e) bleibt `channels/` byte-gleich, `belegt` gleich, und ein
/// mitgesendeter Anhang bleibt im Eingang — ein Senden ohne Bezug mit derselben ID ergibt danach 201.
#[tokio::test]
async fn p3_gueltig_unbekannt_fremd_geloescht_und_format() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path());
    let basis = instanz.adresse.as_str();
    let absender = Absender::von("127.0.0.1");
    let (claude, codex) = agenten(&absender, basis).await;
    let t1 = gesendet(&absender, basis, &claude, "allgemein", "Frage?").await;
    let technik = gesendet(&absender, basis, &claude, "technik", "Drüben").await;
    let weg = gesendet(&absender, basis, &claude, "allgemein", "Weg damit").await;
    loeschen(&absender, basis, &claude, "allgemein", &weg).await;
    let t1_in_utc = chrono::DateTime::parse_from_rfc3339(&t1)
        .expect("t1")
        .with_timezone(&chrono::Utc)
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    let gueltig = antworten(&absender, basis, &codex, "Antwort", json!(t1_in_utc)).await;

    assert_eq!(gueltig.status, 201, "{}", gueltig.json);
    assert_eq!(
        gueltig.json["nachricht"]["antwort_auf"],
        json!({"zeit": t1, "von": CLAUDE, "zustand": "da", "textanfang": "Frage?", "anhang": null, "geaendert": null})
    );
    let tag = String::from_utf8(gemeinsam::senden::tagesinhalt(wurzel.path(), "allgemein"))
        .expect("UTF-8");
    assert!(
        tag.contains(&format!("+ antwort_auf: {t1} {CLAUDE}\n")),
        "Zeit in der Schreibweise des Originals: {tag}"
    );

    let mut texte = Vec::new();
    for (bezug, grund) in [
        (json!("2026-10-28T11:59:59.999+01:00"), "unbekannt"),
        (json!(technik), "unbekannt"),
        (json!(weg), "geloescht"),
        (json!("gestern"), "format"),
        (json!(42), "format"),
        (json!({}), "format"),
        (json!([t1]), "format"),
        (json!({"zeit": t1, "von": CLAUDE}), "format"),
    ] {
        let id = gemeinsam::anhaenge::ablegen(&absender, basis, &codex, "notiz.txt", b"x".to_vec())
            .await;
        let vorher = (channels_stand(wurzel.path()), belegt(basis).await);

        let abgewiesen = senden_json(
            &absender,
            basis,
            &codex,
            "allgemein",
            &json!({"text": "Antwort", "anhaenge": [id], "antwort_auf": bezug}),
        )
        .await;

        assert_eq!(abgewiesen.status, 422, "{bezug}: {}", abgewiesen.json);
        assert_eq!(abgewiesen.json["regel"], "antwort_auf", "{bezug}");
        assert_eq!(abgewiesen.json["grund"], grund, "{bezug}");
        let fehler = abgewiesen.json["fehler"]
            .as_str()
            .expect("fehler")
            .to_string();
        assert_eq!(
            (channels_stand(wurzel.path()), belegt(basis).await),
            vorher,
            "{bezug}"
        );
        let danach = senden_json(
            &absender,
            basis,
            &codex,
            "allgemein",
            &json!({"text": "ohne Bezug", "anhaenge": [id]}),
        )
        .await;
        assert_eq!(danach.status, 201, "{bezug}: {}", danach.json);
        texte.push(fehler);
    }
    assert_eq!(texte[0], texte[1], "fremd antwortet wie nie dagewesen");
    assert_eq!(
        texte[0],
        "Die Nachricht, auf die du antwortest, gibt es in diesem Gespräch nicht (mehr)."
    );
    assert_eq!(
        texte[2],
        "Die Nachricht, auf die du antwortest, ist gelöscht."
    );
    assert_eq!(
        texte[3],
        "antwort_auf muss die Zeit einer Nachricht sein, wie sie im Verlauf steht (RFC 3339, z. B. 2026-10-30T09:10:03.004+01:00)."
    );
}

/// ADR-0053 S2 und S6: `antwort_auf` doppelt im Körper → 422 `feld`/`doppelt`; die Zeit eines Änderungssatzes →
/// 422 `unbekannt`; ein bearbeitetes Original ist gültig.
#[tokio::test]
async fn doppelt_im_koerper_und_zeit_eines_aenderungssatzes() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path());
    let basis = instanz.adresse.as_str();
    let absender = Absender::von("127.0.0.1");
    let (claude, codex) = agenten(&absender, basis).await;
    let t1 = gesendet(&absender, basis, &claude, "allgemein", "Frage?").await;
    let bearbeitet = api_antwort(
        absender
            .anfrage(
                reqwest::Method::POST,
                format!("{basis}api/channels/allgemein/nachrichten/bearbeiten"),
            )
            .header("Content-Type", "application/json")
            .header("X-LocalchatA-Sitzung", &claude)
            .body(json!({"zeit": t1, "text": "Frage!"}).to_string()),
    )
    .await;
    assert_eq!(bearbeitet.status, 200, "{}", bearbeitet.json);
    let aenderungszeit = bearbeitet.json["nachricht"]["geaendert"]
        .as_str()
        .expect("geaendert")
        .to_string();
    let vorher = channels_stand(wurzel.path());

    let doppelt = roh_senden(
        &absender,
        basis,
        Some(&codex),
        "allgemein",
        "application/json",
        format!(r#"{{"text":"x","antwort_auf":"{t1}","antwort_auf":"{t1}"}}"#).into_bytes(),
    )
    .await;
    let aenderungssatz = antworten(&absender, basis, &codex, "x", json!(aenderungszeit)).await;

    assert_eq!(doppelt.status, 422, "{}", doppelt.json);
    assert_eq!(doppelt.json["regel"], "feld");
    assert_eq!(doppelt.json["grund"], "doppelt", "{}", doppelt.json);
    assert_eq!(aenderungssatz.status, 422, "{}", aenderungssatz.json);
    assert_eq!(aenderungssatz.json["grund"], "unbekannt");
    assert_eq!(channels_stand(wurzel.path()), vorher);
    let auf_bearbeitet = antworten(&absender, basis, &codex, "x", json!(t1)).await;
    assert_eq!(auf_bearbeitet.status, 201, "{}", auf_bearbeitet.json);
    assert_eq!(
        auf_bearbeitet.json["nachricht"]["antwort_auf"]["zustand"],
        "bearbeitet"
    );
}

/// ADR-0053 P5, QZ1, Mutante Z5: Original mit 500 Zeichen und Marke `ZITATMARKE-4711`; in der Tagesdatei steht die
/// Marke genau einmal; der Satz der Antwort ist um genau `+ antwort_auf: ` 15 + Zeit 29 + 1 + Kennung 16 + 1 = 62
/// Bytes größer als die gleiche Antwort ohne Bezug.
#[tokio::test]
async fn p5_im_log_steht_nur_der_bezug() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path());
    let basis = instanz.adresse.as_str();
    let absender = Absender::von("127.0.0.1");
    let (claude, codex) = agenten(&absender, basis).await;
    let marke = "ZITATMARKE-4711";
    let original = format!("{marke} {}", "o".repeat(500 - marke.len() - 1));
    let t1 = gesendet(&absender, basis, &claude, "allgemein", &original).await;
    let laenge = || gemeinsam::senden::tagesinhalt(wurzel.path(), "allgemein").len();

    let vor_ohne = laenge();
    gesendet(&absender, basis, &codex, "allgemein", "Antwort").await;
    let ohne = laenge() - vor_ohne;
    let vor_mit = laenge();
    let mit = antworten(&absender, basis, &codex, "Antwort", json!(t1)).await;
    let mit_bezug = laenge() - vor_mit;

    assert_eq!(mit.status, 201, "{}", mit.json);
    assert_eq!(mit_bezug - ohne, 15 + 29 + 1 + CLAUDE.len() + 1);
    let tag = String::from_utf8(gemeinsam::senden::tagesinhalt(wurzel.path(), "allgemein"))
        .expect("UTF-8");
    assert_eq!(tag.matches(marke).count(), 1);
}

/// ADR-0053 P8: ohne Feld und mit `null` → 201 wie vorher plus `antwort_auf: null`, auch im Verlauf; der Tippfehler
/// `antwort` → 422 `feld`, dessen Text `antwort_auf` nennt; die Länge gilt auch mit Bezug (422 `laenge`).
#[tokio::test]
async fn p8_ohne_bezug_bleibt_alles_wie_bisher() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path());
    let basis = instanz.adresse.as_str();
    let absender = Absender::von("127.0.0.1");
    let (claude, codex) = agenten(&absender, basis).await;
    let t1 = gesendet(&absender, basis, &claude, "allgemein", "Frage?").await;

    let ohne_feld = senden(&absender, basis, &codex, "allgemein", "eins").await;
    let mit_null = antworten(&absender, basis, &codex, "zwei", Value::Null).await;
    let tippfehler = senden_json(
        &absender,
        basis,
        &codex,
        "allgemein",
        &json!({"text": "drei", "antwort": t1}),
    )
    .await;
    let zu_lang = antworten(&absender, basis, &codex, &"l".repeat(16_385), json!(t1)).await;

    for antwort in [&ohne_feld, &mit_null] {
        assert_eq!(antwort.status, 201, "{}", antwort.json);
        let nachricht = antwort.json["nachricht"].as_object().expect("nachricht");
        assert_eq!(
            nachricht.keys().cloned().collect::<Vec<_>>(),
            [
                "anhaenge",
                "antwort_auf",
                "art",
                "geaendert",
                "text",
                "von",
                "zeit",
                "zustand"
            ]
        );
        assert_eq!(nachricht["antwort_auf"], Value::Null);
    }
    assert_eq!(tippfehler.status, 422, "{}", tippfehler.json);
    assert_eq!(
        tippfehler.json,
        json!({"fehler": "Unbekanntes Feld antwort — erlaubt sind text, anhaenge und antwort_auf.", "regel": "feld", "feld": "antwort"})
    );
    assert_eq!(zu_lang.status, 422, "{}", zu_lang.json);
    assert_eq!(zu_lang.json["regel"], "laenge");
    let verlauf = verlauf(&absender, basis, &codex, "allgemein").await;
    let nachrichten = verlauf["nachrichten"].as_array().expect("nachrichten");
    assert_eq!(nachrichten.len(), 3);
    assert!(
        nachrichten
            .iter()
            .all(|nachricht| nachricht["antwort_auf"] == Value::Null)
    );
}

/// ADR-0053 P8 Grenzen: am erschöpften Schreibkonto antwortet auch eine Antwort 429, an der vollen Plattenquote 507 —
/// nichts geschrieben.
#[tokio::test]
async fn p8_schreibkonto_und_quote_gelten_auch_fuer_antworten() {
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
    let absender = Absender::von("127.0.0.1");
    let (claude, codex) = agenten(&absender, basis).await;
    let t1 = gesendet(&absender, basis, &claude, "allgemein", "Frage?").await;
    let vorher = channels_stand(wurzel.path());

    let voll = antworten(&absender, basis, &codex, &"q".repeat(400), json!(t1)).await;

    assert_eq!(voll.status, 507, "{}", voll.json);
    assert_eq!(voll.json["regel"], "quote");
    assert_eq!(channels_stand(wurzel.path()), vorher);
    let mut versuche = 0;
    while senden(&absender, basis, &codex, "technik", "noch eine")
        .await
        .status
        != 429
    {
        versuche += 1;
        assert!(versuche < 500, "das Schreibkonto bremst nie");
    }
    let vorher = channels_stand(wurzel.path());
    let am_konto = antworten(&absender, basis, &codex, "kurz", json!(t1)).await;
    assert_eq!(am_konto.status, 429, "{}", am_konto.json);
    assert!(am_konto.kopf.get("retry-after").is_some());
    assert_eq!(channels_stand(wurzel.path()), vorher);
}

/// `POST …/allgemein/nachrichten` mit `Idempotency-Key` und Körper.
async fn mit_schluessel(
    absender: &Absender,
    basis: &str,
    token: &str,
    schluessel: &str,
    koerper: &Value,
) -> ApiAntwort {
    api_antwort(
        absender
            .anfrage(
                reqwest::Method::POST,
                format!("{basis}api/channels/allgemein/nachrichten"),
            )
            .header("Content-Type", "application/json")
            .header("X-LocalchatA-Sitzung", token)
            .header("Idempotency-Key", schluessel)
            .body(koerper.to_string()),
    )
    .await
}

/// ADR-0053 P6 (Wiederholung), Regel 2, US-22, Mutante Z6: derselbe Schlüssel mit anderem Bezug oder ohne Bezug →
/// 422 `schluessel`, nichts geschrieben; mit demselben Bezug nach dem Löschen des Originals → wie die erste (200 nach
/// ADR-0034 Regel 6), dieselbe Zeit, ein Satz, das Zitat im heutigen Zustand `geloescht`.
#[tokio::test]
async fn p6_der_wiederholschluessel_umfasst_den_bezug() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path());
    let basis = instanz.adresse.as_str();
    let absender = Absender::von("127.0.0.1");
    let (claude, codex) = agenten(&absender, basis).await;
    let t1 = gesendet(&absender, basis, &claude, "allgemein", "Frage?").await;
    let t2 = gesendet(&absender, basis, &claude, "allgemein", "Noch eine").await;
    let erste = mit_schluessel(
        &absender,
        basis,
        &codex,
        "k1",
        &json!({"text": "Antwort", "antwort_auf": t1}),
    )
    .await;
    assert_eq!(erste.status, 201, "{}", erste.json);
    let vorher = channels_stand(wurzel.path());

    let anderer_bezug = mit_schluessel(
        &absender,
        basis,
        &codex,
        "k1",
        &json!({"text": "Antwort", "antwort_auf": t2}),
    )
    .await;
    let ohne_bezug =
        mit_schluessel(&absender, basis, &codex, "k1", &json!({"text": "Antwort"})).await;

    for antwort in [&anderer_bezug, &ohne_bezug] {
        assert_eq!(antwort.status, 422, "{}", antwort.json);
        assert_eq!(antwort.json["regel"], "schluessel");
    }
    assert_eq!(channels_stand(wurzel.path()), vorher);
    loeschen(&absender, basis, &claude, "allgemein", &t1).await;
    let vorher = channels_stand(wurzel.path());
    let wiederholt = mit_schluessel(
        &absender,
        basis,
        &codex,
        "k1",
        &json!({"text": "Antwort", "antwort_auf": t1}),
    )
    .await;
    assert_eq!(wiederholt.status, 200, "{}", wiederholt.json);
    assert_eq!(
        wiederholt.json["nachricht"]["zeit"],
        erste.json["nachricht"]["zeit"]
    );
    assert_eq!(
        wiederholt.json["nachricht"]["antwort_auf"],
        json!({"zeit": t1, "von": CLAUDE, "zustand": "geloescht", "textanfang": "", "anhang": null,
               "geaendert": wiederholt.json["nachricht"]["antwort_auf"]["geaendert"]})
    );
    assert!(wiederholt.json["nachricht"]["antwort_auf"]["geaendert"].is_string());
    assert_eq!(channels_stand(wurzel.path()), vorher);
}

/// Der Satz der Nachricht mit dieser Zeit in der heutigen Tagesdatei, bis vor den nächsten Satz.
fn satz_im_log(ordner: &Path, zeit: &str) -> String {
    let tag =
        String::from_utf8(gemeinsam::senden::tagesinhalt(ordner, "allgemein")).expect("UTF-8");
    let anfang = tag.find(&format!("@ {zeit} ")).expect("Satz im Log");
    let rest = &tag[anfang..];
    let ende = rest[1..]
        .find("\n@ ")
        .map_or(rest.len(), |stelle| stelle + 2);
    rest[..ende].to_string()
}

/// Die Nachricht mit dieser Zeit im Verlauf von `# allgemein`.
async fn im_verlauf(absender: &Absender, basis: &str, token: &str, zeit: &str) -> Value {
    verlauf(absender, basis, token, "allgemein").await["nachrichten"]
        .as_array()
        .expect("nachrichten")
        .iter()
        .find(|nachricht| nachricht["zeit"] == zeit)
        .cloned()
        .expect("im Verlauf")
}

/// ADR-0053 P4, P6 (Strom), US-2, US-23, Mutanten Z3, Z4: der Verlauf liefert das Zitat im Zustand des Originals —
/// `da`, nach dem Bearbeiten `bearbeitet` mit dem neuen Textanfang und `geaendert`, nach dem Löschen `geloescht` ohne
/// Text, nach dem Entfallen des Tags `entfallen` mit Zeit und Kennung; der Satz der Antwort im Log bleibt dabei
/// byte-gleich; der Strom trägt das Zitat im Thema `nachricht`; `Frage?` erscheint nach dem Bearbeiten nirgends.
#[tokio::test]
async fn p4_das_zitat_folgt_dem_original_im_verlauf_und_im_strom() {
    let wurzel = ordner();
    gemeinsam::frist::datei_anlegen(
        wurzel.path(),
        "channels/allgemein/2026-10-27.log",
        format!("@ 2026-10-27T09:00:00.000+01:00 {CLAUDE}\n+ art: agent\n  Von gestern\n\n")
            .as_bytes(),
    );
    let instanz = starten(wurzel.path());
    let basis = instanz.adresse.as_str();
    let absender = Absender::von("127.0.0.1");
    let (claude, codex) = agenten(&absender, basis).await;
    let gestern = "2026-10-27T09:00:00.000+01:00";
    let t1 = gesendet(&absender, basis, &claude, "allgemein", "Frage?").await;
    let t2 = gesendet(&absender, basis, &claude, "allgemein", "Zum Löschen").await;
    let mut strom = gemeinsam::strom::Strom::oeffnen(&absender, basis, &codex).await;
    let mut antworten_auf = Vec::new();
    for bezug in [&t1, &t2, gestern] {
        let antwort = antworten(&absender, basis, &codex, "Antwort", json!(bezug)).await;
        assert_eq!(antwort.status, 201, "{}", antwort.json);
        antworten_auf.push(
            antwort.json["nachricht"]["zeit"]
                .as_str()
                .expect("zeit")
                .to_string(),
        );
    }
    let im_strom = strom
        .bis(std::time::Duration::from_secs(10), |e| {
            e.data["art"] == "nachricht" && e.data["nachricht"]["zeit"] == antworten_auf[0].as_str()
        })
        .await
        .expect("nachricht im Strom");
    assert_eq!(
        im_strom.data["nachricht"]["antwort_auf"],
        json!({"zeit": t1, "von": CLAUDE, "zustand": "da", "textanfang": "Frage?", "anhang": null, "geaendert": null})
    );
    let saetze: Vec<String> = antworten_auf
        .iter()
        .map(|zeit| satz_im_log(wurzel.path(), zeit))
        .collect();
    let da = im_verlauf(&absender, basis, &codex, &antworten_auf[0]).await;
    assert_eq!(da["antwort_auf"]["zustand"], "da");
    assert_eq!(
        im_verlauf(&absender, basis, &codex, &antworten_auf[2]).await["antwort_auf"],
        json!({"zeit": gestern, "von": CLAUDE, "zustand": "da", "textanfang": "Von gestern", "anhang": null, "geaendert": null})
    );

    let bearbeitet = api_antwort(
        absender
            .anfrage(
                reqwest::Method::POST,
                format!("{basis}api/channels/allgemein/nachrichten/bearbeiten"),
            )
            .header("Content-Type", "application/json")
            .header("X-LocalchatA-Sitzung", &claude)
            .body(json!({"zeit": t1, "text": "Frage!"}).to_string()),
    )
    .await;
    assert_eq!(bearbeitet.status, 200, "{}", bearbeitet.json);
    loeschen(&absender, basis, &claude, "allgemein", &t2).await;
    let frist = gemeinsam::frist::frist_setzen(
        &absender,
        &instanz,
        &claude,
        "allgemein",
        json!({"tage": 1, "bestaetigt": true}),
    )
    .await;
    assert_eq!(frist.status, 200, "{}", frist.json);
    let nach_dem_bearbeiten = antworten(&absender, basis, &codex, "Noch eine", json!(t1)).await;
    strom.mitlesen(std::time::Duration::from_millis(300)).await;

    let block = verlauf(&absender, basis, &codex, "allgemein").await;
    let zitat = |zeit: &str| {
        block["nachrichten"]
            .as_array()
            .expect("nachrichten")
            .iter()
            .find(|nachricht| nachricht["zeit"] == zeit)
            .expect("Antwort im Verlauf")["antwort_auf"]
            .clone()
    };
    assert_eq!(
        zitat(&antworten_auf[0]),
        json!({"zeit": t1, "von": CLAUDE, "zustand": "bearbeitet", "textanfang": "Frage!", "anhang": null,
               "geaendert": bearbeitet.json["nachricht"]["geaendert"]})
    );
    let geloescht = zitat(&antworten_auf[1]);
    assert_eq!(
        (
            &geloescht["zustand"],
            &geloescht["textanfang"],
            &geloescht["anhang"]
        ),
        (&json!("geloescht"), &json!(""), &Value::Null)
    );
    assert!(geloescht["geaendert"].is_string());
    assert_eq!(
        zitat(&antworten_auf[2]),
        json!({"zeit": gestern, "von": CLAUDE, "zustand": "entfallen", "textanfang": "", "anhang": null, "geaendert": null})
    );
    let jetzt: Vec<String> = antworten_auf
        .iter()
        .map(|zeit| satz_im_log(wurzel.path(), zeit))
        .collect();
    assert_eq!(jetzt, saetze, "der Satz der Antwort bleibt byte-gleich");
    assert_eq!(
        nach_dem_bearbeiten.status, 201,
        "{}",
        nach_dem_bearbeiten.json
    );
    let nach_der_aenderung: Vec<String> = strom
        .ereignisse
        .iter()
        .skip_while(|e| e.data["art"] != "aenderung")
        .map(|e| e.data.to_string())
        .collect();
    assert!(!nach_der_aenderung.is_empty());
    for text in [block.to_string(), nach_dem_bearbeiten.json.to_string()]
        .into_iter()
        .chain(nach_der_aenderung)
    {
        assert!(
            !text.contains("Frage?"),
            "frühere Fassung geliefert: {text}"
        );
    }
}

/// ADR-0053 P7, US-5, US-20, Mutante Z13: `anna` antwortet auf die Antwort von `codex` → der Bezug nennt `codex` und
/// deren Textanfang, im Objekt kein weiteres `antwort_auf`; die Antworten stehen in Zeitfolge am Ende; ist die Antwort
/// von `codex` gelöscht, liefert sie selbst `antwort_auf: null`.
#[tokio::test]
async fn p7_keine_verschachtelung_zeitfolge_und_geloeschte_antwort() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path());
    let basis = instanz.adresse.as_str();
    let absender = Absender::von("127.0.0.1");
    let (claude, codex) = agenten(&absender, basis).await;
    let anna = beitreten(&absender, basis, "anna", Some("mensch")).await;
    let t1 = gesendet(&absender, basis, &claude, "allgemein", "Frage?").await;
    let zwischen = gesendet(&absender, basis, &claude, "allgemein", "Zwischendurch").await;
    let von_codex = antworten(&absender, basis, &codex, "Antwort von codex", json!(t1)).await;
    let t_codex = von_codex.json["nachricht"]["zeit"]
        .as_str()
        .expect("zeit")
        .to_string();

    let von_anna = antworten(&absender, basis, &anna, "Antwort von anna", json!(t_codex)).await;

    assert_eq!(von_anna.status, 201, "{}", von_anna.json);
    assert_eq!(
        von_anna.json["nachricht"]["antwort_auf"],
        json!({"zeit": t_codex, "von": "codex@127.0.0.1", "zustand": "da", "textanfang": "Antwort von codex",
               "anhang": null, "geaendert": null})
    );
    let block = verlauf(&absender, basis, &codex, "allgemein").await;
    let zeiten: Vec<&str> = block["nachrichten"]
        .as_array()
        .expect("nachrichten")
        .iter()
        .map(|nachricht| nachricht["zeit"].as_str().expect("zeit"))
        .collect();
    let t_anna = von_anna.json["nachricht"]["zeit"].as_str().expect("zeit");
    assert_eq!(
        zeiten,
        [t1.as_str(), zwischen.as_str(), t_codex.as_str(), t_anna]
    );
    assert_eq!(block["nachrichten"][3], von_anna.json["nachricht"]);

    let mut strom = gemeinsam::strom::Strom::oeffnen(&absender, basis, &claude).await;
    let antwort = loeschen(&absender, basis, &codex, "allgemein", &t_codex).await;
    let ereignis = strom
        .bis(std::time::Duration::from_secs(10), |e| {
            e.data["art"] == "aenderung"
        })
        .await
        .expect("aenderung");

    assert_eq!(antwort["nachricht"]["antwort_auf"], Value::Null);
    assert_eq!(ereignis.data["nachricht"]["antwort_auf"], Value::Null);
    let geloescht = im_verlauf(&absender, basis, &codex, &t_codex).await;
    assert_eq!(geloescht["zustand"], "geloescht");
    assert_eq!(geloescht["antwort_auf"], Value::Null);
    assert_eq!(
        im_verlauf(&absender, basis, &codex, t_anna).await["antwort_auf"]["zustand"],
        "geloescht"
    );
}
