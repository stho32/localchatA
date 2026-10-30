//! F0041 Senden über die API (R00011, US-1 bis US-6): das echte Programm, Agenten ohne `Origin`.
//!
//! `cargo test --test senden` fährt die Kriterien „Senden und Rundweg“ bis „Plattenfehler“.

mod gemeinsam;

use gemeinsam::Absender;
use gemeinsam::senden::{
    HEUTE, aufgefuellt, beitreten, ordner, roh_senden, senden, starten, tagesdatei, tagesinhalt,
    verlauf,
};
use serde_json::{Value, json};
use std::time::Duration;

fn lokal() -> Absender {
    Absender::von("127.0.0.1")
}

fn neueste(verlauf: &Value) -> Value {
    verlauf["nachrichten"]
        .as_array()
        .and_then(|liste| liste.last())
        .cloned()
        .unwrap_or(Value::Null)
}

/// US-1 „Senden und Rundweg“: 201 in der einen Form, Satz mit Metazeile, Verlauf gleich — auch nach dem Neustart.
#[tokio::test]
async fn us1_senden_und_dieselbe_nachricht_wieder_lesen_auch_nach_dem_neustart() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[]);
    let beta = lokal();
    let token = beitreten(&beta, &instanz.adresse, "beta", None).await;

    let antwort = senden(
        &beta,
        &instanz.adresse,
        &token,
        "allgemein",
        "Hallo\r\nWelt\n\n",
    )
    .await;

    assert_eq!(antwort.status, 201, "{}", antwort.json);
    let zeit = antwort.json["nachricht"]["zeit"]
        .as_str()
        .expect("Zeit")
        .to_string();
    assert_eq!(
        antwort.json,
        json!({"channel": "allgemein", "nachricht": {"zeit": zeit, "von": "beta@127.0.0.1", "art": "agent", "text": "Hallo\nWelt", "anhaenge": [], "zustand": null, "geaendert": null, "antwort_auf": null}})
    );
    let datei = String::from_utf8(tagesinhalt(wurzel.path(), "allgemein")).expect("UTF-8");
    assert!(
        datei.ends_with(&format!(
            "@ {zeit} beta@127.0.0.1\n+ art: agent\n  Hallo\n  Welt\n\n"
        )),
        "{datei:?}"
    );
    assert!(zeit.starts_with(HEUTE), "{zeit}");
    assert_eq!(
        neueste(&verlauf(&beta, &instanz.adresse, &token, "allgemein").await),
        antwort.json["nachricht"]
    );
    drop(instanz);
    let neu = starten(wurzel.path(), &[]);
    let token = beitreten(&beta, &neu.adresse, "beta", None).await;
    assert_eq!(
        neueste(&verlauf(&beta, &neu.adresse, &token, "allgemein").await),
        antwort.json["nachricht"]
    );
}

/// US-2 „Mensch“: Beitritt mit `"art":"mensch"` → Antwort und Satz tragen `mensch`.
#[tokio::test]
async fn us2_ein_mensch_ueber_die_api_ist_ein_mensch() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[]);
    let anna = lokal();
    let token = beitreten(&anna, &instanz.adresse, "anna", Some("mensch")).await;

    let antwort = senden(&anna, &instanz.adresse, &token, "allgemein", "Hallo").await;

    assert_eq!(antwort.status, 201);
    assert_eq!(antwort.json["nachricht"]["art"], "mensch");
    let datei = String::from_utf8(tagesinhalt(wurzel.path(), "allgemein")).expect("UTF-8");
    assert!(datei.contains("\n+ art: mensch\n"), "{datei:?}");
}

/// „Satz ohne Metazeile“: von Hand geschrieben → `art: null`.
#[tokio::test]
async fn ein_satz_ohne_metazeile_hat_art_null() {
    let wurzel = ordner();
    std::fs::write(
        tagesdatei(wurzel.path(), "allgemein"),
        "@ 2026-10-28T09:00:00.000+01:00 claude@10.0.0.1\n  von Hand\n\n",
    )
    .expect("Tagesdatei");
    let instanz = starten(wurzel.path(), &[]);
    let beta = lokal();
    let token = beitreten(&beta, &instanz.adresse, "beta", None).await;

    let nachricht = neueste(&verlauf(&beta, &instanz.adresse, &token, "allgemein").await);

    assert_eq!(nachricht["text"], "von Hand");
    assert_eq!(nachricht["art"], Value::Null);
    assert!(nachricht.as_object().expect("Objekt").contains_key("art"));
}

/// US-4 „Körpergrenze als Spanne“: 131 072 gültig → 201; 131 073 kein JSON → 413, nicht 400; als text/plain → 413.
/// R00020 F0069 US-4 „Zu groß heißt teilen“: die 413 schreibt die Grenze wie die Oberfläche, ohne Beispiel, und
/// schließt; der Verlauf trägt nur die angenommene Nachricht.
#[tokio::test]
async fn us4_die_koerpergrenze_liegt_genau_bei_131072_bytes() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[]);
    let beta = lokal();
    let token = beitreten(&beta, &instanz.adresse, "beta", None).await;
    let gueltig = aufgefuellt(&format!("{{\"text\":\"{}\"}}", "a".repeat(16_384)), 131_072);
    let mut kaputt = b"{".to_vec();
    kaputt.extend(std::iter::repeat_n(b'a', 131_072));

    let angenommen = roh_senden(
        &beta,
        &instanz.adresse,
        Some(&token),
        "allgemein",
        "application/json",
        gueltig,
    )
    .await;
    let zu_gross = roh_senden(
        &beta,
        &instanz.adresse,
        Some(&token),
        "allgemein",
        "application/json",
        kaputt.clone(),
    )
    .await;
    let zu_gross_text = roh_senden(
        &beta,
        &instanz.adresse,
        Some(&token),
        "allgemein",
        "text/plain",
        kaputt,
    )
    .await;

    assert_eq!(angenommen.status, 201, "{}", angenommen.json);
    assert_eq!(
        angenommen.json["nachricht"]["text"],
        json!("a".repeat(16_384))
    );
    let zu_gross_erwartet =
        json!({"fehler": "Der Körper ist zu groß — höchstens 131.072 Byte; längere Texte teilen."});
    for antwort in [&zu_gross, &zu_gross_text] {
        assert_eq!((antwort.status, &antwort.json), (413, &zu_gross_erwartet));
        assert_eq!(
            antwort.kopf.get("connection").map(|w| w.as_bytes()),
            Some(&b"close"[..])
        );
    }
    let nachrichten = verlauf(&beta, &instanz.adresse, &token, "allgemein").await;
    assert_eq!(
        nachrichten["nachrichten"].as_array().map(Vec::len),
        Some(1),
        "nur die angenommene: {nachrichten}"
    );
}

/// „Längste Schreibweise“: jedes Zeichen als `a` (98 315 Bytes) → 201; 5 462 × `€` → 422 `laenge`.
#[tokio::test]
async fn die_laengste_schreibweise_passt_und_16386_bytes_nicht() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[]);
    let beta = lokal();
    let token = beitreten(&beta, &instanz.adresse, "beta", None).await;
    let laengste = format!("{{\"text\":\"{}\"}}", "\\u0061".repeat(16_384)).into_bytes();
    assert_eq!(laengste.len(), 98_315);

    let angenommen = roh_senden(
        &beta,
        &instanz.adresse,
        Some(&token),
        "allgemein",
        "application/json",
        laengste,
    )
    .await;
    let zu_lang = senden(
        &beta,
        &instanz.adresse,
        &token,
        "allgemein",
        &"€".repeat(5_462),
    )
    .await;

    assert_eq!(angenommen.status, 201, "{}", angenommen.json);
    assert_eq!(
        angenommen.json["nachricht"]["text"],
        json!("a".repeat(16_384))
    );
    assert_eq!(zu_lang.status, 422);
    assert_eq!(zu_lang.json["regel"], "laenge");
    assert_eq!(zu_lang.json["bytes"], 16_386);
}

/// US-3 „Abweisungen“: vier 422 mit Regel und Text; danach Tagesdatei byte-gleich und Verlauf unverändert;
/// 5 461 × `€` + `a` wird angenommen.
#[tokio::test]
async fn us3_abgewiesen_wird_mit_regel_und_text_nichts_wird_umgeformt() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[]);
    let beta = lokal();
    let token = beitreten(&beta, &instanz.adresse, "beta", None).await;
    let vorher_gesendet = senden(&beta, &instanz.adresse, &token, "allgemein", "vorher").await;
    assert_eq!(vorher_gesendet.status, 201);
    let datei_vorher = tagesinhalt(wurzel.path(), "allgemein");
    let verlauf_vorher = verlauf(&beta, &instanz.adresse, &token, "allgemein").await;
    let roh = |koerper: Value| koerper.to_string().into_bytes();

    let leer = senden(&beta, &instanz.adresse, &token, "allgemein", "  ").await;
    let lang = senden(
        &beta,
        &instanz.adresse,
        &token,
        "allgemein",
        &"a".repeat(16_385),
    )
    .await;
    let steuer = senden(
        &beta,
        &instanz.adresse,
        &token,
        "allgemein",
        "rot \u{1b}[31m",
    )
    .await;
    let feld = roh_senden(
        &beta,
        &instanz.adresse,
        Some(&token),
        "allgemein",
        "application/json",
        roh(json!({"text": "Hallo", "antwort": "x"})),
    )
    .await;
    let kein_text = roh_senden(
        &beta,
        &instanz.adresse,
        Some(&token),
        "allgemein",
        "application/json",
        roh(json!({"text": 5})),
    )
    .await;

    for antwort in [&leer, &lang, &steuer, &feld, &kein_text] {
        assert_eq!(antwort.status, 422, "{}", antwort.json);
    }
    assert_eq!(
        leer.json,
        json!({"fehler": "Die Nachricht ist leer.", "regel": "leer"})
    );
    assert_eq!(
        lang.json,
        json!({"fehler": "Zu lang: 16.385 von höchstens 16.384 Byte — bitte teilen.", "regel": "laenge", "hoechstens": 16_384, "bytes": 16_385})
    );
    assert_eq!(
        steuer.json,
        json!({"fehler": "Steuerzeichen U+001B an Stelle 5 — bitte entfernen.", "regel": "zeichen", "stelle": 4, "zeichen": "U+001B"})
    );
    assert_eq!(
        feld.json,
        json!({"fehler": "Unbekanntes Feld antwort — erlaubt sind text, anhaenge und antwort_auf.", "regel": "feld", "feld": "antwort"})
    );
    assert_eq!(
        kein_text.json,
        json!({"fehler": "Der Körper braucht text als Zeichenkette.", "regel": "text"})
    );
    assert_eq!(tagesinhalt(wurzel.path(), "allgemein"), datei_vorher);
    assert_eq!(
        verlauf(&beta, &instanz.adresse, &token, "allgemein").await,
        verlauf_vorher
    );
    let genau = senden(
        &beta,
        &instanz.adresse,
        &token,
        "allgemein",
        &format!("{}a", "€".repeat(5_461)),
    )
    .await;
    assert_eq!(genau.status, 201);
}

/// „Steuerzeichen und JSON-Ausdehnung“ über HTTP: U+0001 → 422, Datei byte-gleich; Tab und Zeilenumbruch → 201.
#[tokio::test]
async fn ein_steuerzeichen_ohne_kurzform_wird_abgewiesen_tab_und_umbruch_nicht() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[]);
    let beta = lokal();
    let token = beitreten(&beta, &instanz.adresse, "beta", None).await;

    let eins = senden(&beta, &instanz.adresse, &token, "allgemein", "a\u{1}b").await;
    let datei_danach = tagesinhalt(wurzel.path(), "allgemein");
    let kontrolle = senden(&beta, &instanz.adresse, &token, "allgemein", "a\tb\nc").await;

    assert_eq!(eins.status, 422);
    assert_eq!(
        eins.json,
        json!({"fehler": "Steuerzeichen U+0001 an Stelle 2 — bitte entfernen.", "regel": "zeichen", "stelle": 1, "zeichen": "U+0001"})
    );
    assert!(datei_danach.is_empty(), "nichts geschrieben");
    assert_eq!(kontrolle.status, 201);
    assert_eq!(
        neueste(&verlauf(&beta, &instanz.adresse, &token, "allgemein").await)["text"],
        "a\tb\nc"
    );
}

/// US-5 „Grenzschutz und Reihenfolge“: 401, 403, 415, 400, 404 mit Text; Channel vor Text.
#[tokio::test]
async fn us5_grenzschutz_sitzung_typ_json_und_channel_in_dieser_reihenfolge() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[]);
    let beta = lokal();
    let token = beitreten(&beta, &instanz.adresse, "beta", None).await;
    let hallo = json!({"text": "Hallo"}).to_string().into_bytes();
    let basis = &instanz.adresse;

    let ohne = roh_senden(
        &beta,
        basis,
        None,
        "allgemein",
        "application/json",
        hallo.clone(),
    )
    .await;
    let fremd = api_mit_origin(&beta, basis, hallo.clone()).await;
    let typ = roh_senden(
        &beta,
        basis,
        Some(&token),
        "allgemein",
        "text/plain",
        hallo.clone(),
    )
    .await;
    let kaputt = roh_senden(
        &beta,
        basis,
        Some(&token),
        "allgemein",
        "application/json",
        b"{".to_vec(),
    )
    .await;
    let fehlt = senden(&beta, basis, &token, "fehlt", "Hallo").await;
    let gross = senden(&beta, basis, &token, "Allgemein", "Hallo").await;
    let fehlt_leer = senden(&beta, basis, &token, "fehlt", "").await;

    assert_eq!(ohne.status, 401);
    assert_eq!(fremd.status, 403);
    assert_eq!(typ.status, 415);
    assert_eq!(kaputt.status, 400);
    assert_eq!(
        (fehlt.status, &fehlt.json),
        (404, &json!({"fehler": "# fehlt gibt es nicht."}))
    );
    assert_eq!(gross.status, 404);
    assert_eq!(gross.json["fehler"], "# Allgemein gibt es nicht.");
    assert_eq!(fehlt_leer.status, 404, "Channel vor Text");
    assert!(tagesinhalt(wurzel.path(), "allgemein").is_empty());
}

/// Fremder `Origin` ohne Sitzung.
async fn api_mit_origin(
    absender: &Absender,
    basis: &str,
    koerper: Vec<u8>,
) -> gemeinsam::ApiAntwort {
    gemeinsam::api_antwort(
        absender
            .anfrage(
                reqwest::Method::POST,
                format!("{basis}api/channels/allgemein/nachrichten"),
            )
            .header("Content-Type", "application/json")
            .header("Origin", "http://fremd.example")
            .body(koerper),
    )
    .await
}

/// US-6 „Plattenfehler“ (nicht als root): ohne Schreibrecht 500 mit Datei und Grund auf stderr, nichts
/// geschrieben; mit Schreibrecht wieder 201.
#[cfg(unix)]
#[tokio::test]
async fn us6_die_platte_nimmt_nichts_an_und_danach_wieder() {
    use std::os::unix::fs::PermissionsExt;
    let wurzel = ordner();
    let mut instanz = starten(wurzel.path(), &[]);
    let stderr = instanz.stderr_mitschreiben();
    let beta = lokal();
    let token = beitreten(&beta, &instanz.adresse, "beta", None).await;
    let channel = wurzel.path().join("channels/allgemein");
    std::fs::set_permissions(&channel, std::fs::Permissions::from_mode(0o555)).expect("Rechte");

    let gescheitert = senden(&beta, &instanz.adresse, &token, "allgemein", "Hallo").await;
    let verlauf_danach = verlauf(&beta, &instanz.adresse, &token, "allgemein").await;
    std::fs::set_permissions(&channel, std::fs::Permissions::from_mode(0o755)).expect("Rechte");
    let wieder = senden(&beta, &instanz.adresse, &token, "allgemein", "Hallo").await;

    assert_eq!(gescheitert.status, 500, "{}", gescheitert.json);
    assert!(gescheitert.json["fehler"].is_string());
    assert_eq!(verlauf_danach["nachrichten"], json!([]));
    assert_eq!(wieder.status, 201);
    let datei = format!("channels/allgemein/{HEUTE}.log");
    let ende = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        let zeilen = stderr.lock().expect("stderr").clone();
        if zeilen
            .iter()
            .any(|zeile| zeile.contains(&datei) && zeile.contains("grund="))
        {
            break;
        }
        assert!(
            std::time::Instant::now() < ende,
            "keine Zeile mit Datei und Grund: {zeilen:#?}"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}
