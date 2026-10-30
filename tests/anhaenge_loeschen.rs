//! R00029 F0116 — Anhänge löschen über die API, über das echte Programm. Gegeben: `beta` hat `a` in `# allgemein`
//! und `b` in `# technik` gesendet, `gamma` hat `c` in `# allgemein` gesendet (je 1.048.576 Bytes).

mod gemeinsam;

use gemeinsam::Absender;
use gemeinsam::anhaenge::{
    MIB, ablegen, anmelden, gesundheit, hochladen, hochladen_abgewiesen, id, starten,
};
use gemeinsam::senden::{beitreten, ordner, roh_senden, verlauf};
use gemeinsam::{ApiAntwort, api_antwort};
use serde_json::{Value, json};
use std::path::Path;

struct Gegeben {
    wurzel: tempfile::TempDir,
    instanz: gemeinsam::Instanz,
    beta: (Absender, String),
    gamma: (Absender, String),
    a: String,
    b: String,
    c: String,
}

impl Gegeben {
    fn basis(&self) -> &str {
        &self.instanz.adresse
    }

    fn pfad(&self) -> &Path {
        self.wurzel.path()
    }
}

async fn senden_mit(
    absender: &Absender,
    basis: &str,
    token: &str,
    channel: &str,
    ids: &[&str],
) -> ApiAntwort {
    let koerper = json!({"text": "", "anhaenge": ids})
        .to_string()
        .into_bytes();
    roh_senden(
        absender,
        basis,
        Some(token),
        channel,
        "application/json",
        koerper,
    )
    .await
}

async fn gegeben() -> Gegeben {
    gegeben_mit(&[]).await
}

async fn gegeben_mit(weitere: &[&str]) -> Gegeben {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), weitere, &[]);
    let basis = instanz.adresse.clone();
    let beta = Absender::von("127.0.0.1");
    let gamma = Absender::von("127.0.0.1");
    let token_beta = beitreten(&beta, &basis, "beta", Some("agent")).await;
    let token_gamma = beitreten(&gamma, &basis, "gamma", Some("agent")).await;
    let mut ids = Vec::new();
    for (absender, token, channel, name) in [
        (&beta, &token_beta, "allgemein", "a.bin"),
        (&beta, &token_beta, "technik", "b.bin"),
        (&gamma, &token_gamma, "allgemein", "c.bin"),
    ] {
        let id = ablegen(absender, &basis, token, name, vec![7u8; MIB as usize]).await;
        assert_eq!(
            senden_mit(absender, &basis, token, channel, &[&id])
                .await
                .status,
            201
        );
        ids.push(id);
    }
    Gegeben {
        wurzel,
        instanz,
        beta: (beta, token_beta),
        gamma: (gamma, token_gamma),
        a: ids[0].clone(),
        b: ids[1].clone(),
        c: ids[2].clone(),
    }
}

async fn loeschen(absender: &Absender, basis: &str, token: &str, koerper: Value) -> ApiAntwort {
    loeschen_roh(
        absender,
        basis,
        token,
        "application/json",
        koerper.to_string(),
    )
    .await
}

async fn loeschen_roh(
    absender: &Absender,
    basis: &str,
    token: &str,
    typ: &str,
    koerper: String,
) -> ApiAntwort {
    api_antwort(
        absender
            .anfrage(
                reqwest::Method::POST,
                format!("{basis}api/anhaenge/loeschen"),
            )
            .header("Content-Type", typ)
            .header("X-LocalchatA-Sitzung", token)
            .body(koerper),
    )
    .await
}

async fn abrufen(absender: &Absender, basis: &str, id: &str) -> u16 {
    absender
        .anfrage(reqwest::Method::GET, format!("{basis}anhaenge/{id}"))
        .send()
        .await
        .expect("Antwort")
        .status()
        .as_u16()
}

/// Die Datei eines Anhangs unter `channels/<channel>/anhaenge/<tag>/`, gefunden über ihren ASCII-Teil.
fn datei(ordner: &Path, channel: &str, ascii: &str) -> std::path::PathBuf {
    let anhaenge = ordner.join(format!("channels/{channel}/anhaenge"));
    let tag = std::fs::read_dir(&anhaenge)
        .expect("anhaenge")
        .next()
        .expect("Tag")
        .expect("Tag")
        .path();
    std::fs::read_dir(tag)
        .expect("Tag")
        .map(|eintrag| eintrag.expect("Eintrag").path())
        .find(|pfad| pfad.to_string_lossy().ends_with(ascii))
        .expect("Datei")
}

/// Rückfrage (K-a): `gamma` löscht `[a, b]` → 409 mit 2 Anhängen, 2,0 MiB, 2 Gesprächen, 2 fremden; nichts
/// geändert. `[c]` ohne Bestätigung → 409 mit `fremde: 0`.
#[tokio::test]
async fn rueckfrage_aendert_nichts() {
    let g = gegeben().await;
    let (gamma, token) = &g.gamma;
    let vorher = gesundheit(g.basis()).await["anhang_belegt"].clone();

    let antwort = loeschen(gamma, g.basis(), token, json!({"ids": [g.a, g.b]})).await;

    assert_eq!(antwort.status, 409);
    assert_eq!(
        antwort.json,
        json!({
            "fehler": "Wirklich 2 Anhänge (2,0 MiB) aus 2 Gesprächen löschen, davon 2 von anderen? Die Nachrichten bleiben.",
            "regel": "bestaetigen",
            "loeschen": {"anhaenge": 2, "bytes": 2_097_152, "gespraeche": 2, "fremde": 2},
        })
    );
    assert_eq!(abrufen(gamma, g.basis(), &g.a).await, 200);
    assert_eq!(gesundheit(g.basis()).await["anhang_belegt"], vorher);
    let eigene = loeschen(gamma, g.basis(), token, json!({"ids": [g.c]})).await;
    assert_eq!(
        (eigene.status, &eigene.json["loeschen"]["fremde"]),
        (409, &json!(0))
    );
}

/// Befund #55 B2 (ADR-0042 Katalog Z. 367, B0511): die 409 „Löschen nicht bestätigt“ steht als `DEBUG` mit `kennung`
/// und `anhaenge` (Zahl der Anhänge der Rückfrage), genau einmal.
#[tokio::test]
async fn die_rueckfrage_steht_mit_kennung_und_anhaenge_im_protokoll() {
    let g = gegeben_mit(&["--protokoll-stufe", "debug"]).await;
    let (gamma, token) = &g.gamma;

    let antwort = loeschen(gamma, g.basis(), token, json!({"ids": [g.a, g.b]})).await;

    assert_eq!(antwort.status, 409);
    let zeilen: Vec<String> = gemeinsam::protokoll::alle_zeilen(g.pfad())
        .into_iter()
        .filter(|zeile| zeile.contains("status=409"))
        .collect();
    assert_eq!(zeilen.len(), 1, "{zeilen:#?}");
    assert!(
        zeilen[0].ends_with(
            " DEBUG anhang Anfrage abgewiesen | route=\"POST /api/anhaenge/loeschen\" status=409 kennung=gamma@127.0.0.1 anhaenge=2 grund=bestaetigen"
        ),
        "{}",
        zeilen[0]
    );
}

/// Löschen mit Bestätigung: 200 mit Ergebnis je ID und Platz; die Datei hat 0 Bytes und ihren Namen; der Verlauf
/// zeigt die Nachricht von `beta` weiter.
#[tokio::test]
async fn loeschen_mit_bestaetigung() {
    let g = gegeben().await;
    let (gamma, token) = &g.gamma;

    let antwort = loeschen(
        gamma,
        g.basis(),
        token,
        json!({"ids": [g.a, g.b], "bestaetigt": true}),
    )
    .await;

    assert_eq!(antwort.status, 200, "{}", antwort.json);
    assert_eq!(
        antwort.json,
        json!({
            "geloescht": 2,
            "bytes": 2_097_152,
            "ergebnisse": [{"id": g.a, "ergebnis": "geloescht"}, {"id": g.b, "ergebnis": "geloescht"}],
            "platz": {"hoechstens": 10_485_760, "belegt": 1_048_576, "frei": 9_437_184},
        })
    );
    assert_eq!(
        std::fs::metadata(datei(g.pfad(), "allgemein", "a.bin"))
            .expect("Datei")
            .len(),
        0
    );
    let gelesen = verlauf(gamma, g.basis(), token, "allgemein").await;
    assert_eq!(gelesen["nachrichten"][0]["anhaenge"][0]["id"], g.a.as_str());
}

/// Neun gebunden, 507 `voll` für 2 MiB; nach dem bestätigten Löschen von zweien nimmt dieselbe Anmeldung < 1 s
/// nach der 200 an.
#[tokio::test]
async fn nach_dem_loeschen_ist_der_platz_frei() {
    let g = gegeben().await;
    let (beta, token) = &g.beta;
    let mut ids = vec![g.a.clone(), g.b.clone(), g.c.clone()];
    for nummer in 0..6 {
        let id = ablegen(
            beta,
            g.basis(),
            token,
            &format!("n{nummer}.bin"),
            vec![1u8; MIB as usize],
        )
        .await;
        assert_eq!(
            senden_mit(beta, g.basis(), token, "allgemein", &[&id])
                .await
                .status,
            201
        );
        ids.push(id);
    }
    let voll = anmelden(beta, g.basis(), token, "gross.bin", 2 * MIB).await;
    assert_eq!(
        (voll.status, &voll.json["grund"]),
        (507, &json!("voll")),
        "{}",
        voll.json
    );

    let geloescht = loeschen(
        beta,
        g.basis(),
        token,
        json!({"ids": [ids[3], ids[4]], "bestaetigt": true}),
    )
    .await;
    let nach_der_200 = std::time::Instant::now();
    let danach = anmelden(beta, g.basis(), token, "gross.bin", 2 * MIB).await;

    assert_eq!(geloescht.status, 200);
    assert_eq!(danach.status, 201, "{}", danach.json);
    assert!(nach_der_200.elapsed() < std::time::Duration::from_secs(1));
}

/// Erneut, ausgedacht, Eingang (M44′): ein zweites Mal ohne Bestätigung → sofort 200 `schon_geloescht`;
/// ausgedachte ID und eigene, nur angemeldete Hochladung → `unbekannt` mit gleich langer Antwort, der `PUT` gelingt
/// danach; `[c, ausgedacht]` → `c` gelöscht; die fertige Hochladung von `gamma` mit `S(beta)` → `unbekannt`,
/// `gamma` sendet sie danach.
#[tokio::test]
async fn erneut_ausgedacht_und_eingang_sind_kein_orakel() {
    let g = gegeben().await;
    let (beta, token_beta) = &g.beta;
    let (gamma, token_gamma) = &g.gamma;
    let ausgedacht = "a".repeat(26);
    assert_eq!(
        loeschen(
            gamma,
            g.basis(),
            token_gamma,
            json!({"ids": [g.a], "bestaetigt": true})
        )
        .await
        .status,
        200
    );

    let erneut = loeschen(gamma, g.basis(), token_gamma, json!({"ids": [g.a]})).await;
    assert_eq!(
        (erneut.status, &erneut.json["ergebnisse"][0]["ergebnis"]),
        (200, &json!("schon_geloescht"))
    );

    let angemeldet = id(&anmelden(beta, g.basis(), token_beta, "neu.bin", 10).await);
    let eingang = loeschen(
        beta,
        g.basis(),
        token_beta,
        json!({"ids": [angemeldet], "bestaetigt": true}),
    )
    .await;
    let erfunden = loeschen(
        beta,
        g.basis(),
        token_beta,
        json!({"ids": [ausgedacht], "bestaetigt": true}),
    )
    .await;
    assert_eq!(eingang.json["ergebnisse"][0]["ergebnis"], "unbekannt");
    assert_eq!(erfunden.json["ergebnisse"][0]["ergebnis"], "unbekannt");
    assert_eq!(
        eingang.json.to_string().len(),
        erfunden.json.to_string().len()
    );
    assert_eq!(
        hochladen(beta, g.basis(), token_beta, &angemeldet, vec![0; 10])
            .await
            .status,
        200
    );

    let gemischt = loeschen(
        beta,
        g.basis(),
        token_beta,
        json!({"ids": [g.c, ausgedacht], "bestaetigt": true}),
    )
    .await;
    assert_eq!(
        gemischt.json["ergebnisse"],
        json!([{"id": g.c, "ergebnis": "geloescht"}, {"id": ausgedacht, "ergebnis": "unbekannt"}])
    );

    let fertig = ablegen(gamma, g.basis(), token_gamma, "fertig.bin", vec![3; 10]).await;
    let fremd = loeschen(
        beta,
        g.basis(),
        token_beta,
        json!({"ids": [fertig], "bestaetigt": true}),
    )
    .await;
    assert_eq!(fremd.json["ergebnisse"][0]["ergebnis"], "unbekannt");
    assert_eq!(
        senden_mit(gamma, g.basis(), token_gamma, "allgemein", &[&fertig])
            .await
            .status,
        201
    );
}

/// Körper (M53): 422 je Regel, 413 über 16.384 Bytes, 415 `text/plain`; nichts gelöscht, keine 409-Vorschau.
#[tokio::test]
async fn koerper_wird_abgewiesen_nie_umgeformt() {
    let g = gegeben().await;
    let (gamma, token) = &g.gamma;
    let viele: Vec<String> = (0..501).map(|_| g.a.clone()).collect();
    for (roh, status, regel) in [
        (r#"{"ids":[]}"#.to_string(), 422, "leer"),
        (r#"{}"#.to_string(), 422, "leer"),
        (json!({"ids": viele}).to_string(), 422, "anzahl"),
        (json!({"ids": [g.a, g.a]}).to_string(), 422, "doppelt"),
        (json!({"ids": [g.a.to_uppercase()]}).to_string(), 422, "id"),
        (json!({"ids": [&g.a[..25]]}).to_string(), 422, "id"),
        (json!({"ids": [format!("{}a", g.a)]}).to_string(), 422, "id"),
        (json!({"ids": [1, null]}).to_string(), 422, "id"),
        (
            json!({"ids": [g.a], "bestaetigt": "ja"}).to_string(),
            422,
            "bestaetigt",
        ),
        (
            json!({"ids": [g.a], "bestaetigt": null}).to_string(),
            422,
            "bestaetigt",
        ),
        (json!({"ids": [g.a], "x": 1}).to_string(), 422, "feld"),
        (
            format!(
                r#"{{"ids":["{}"],"ids":["{}"],"bestaetigt":true}}"#,
                g.a, g.b
            ),
            422,
            "feld",
        ),
        (
            format!(r#"{{"ids":["{}"],"x":"{}"}}"#, g.a, "x".repeat(16_385)),
            413,
            "",
        ),
    ] {
        let antwort = loeschen_roh(gamma, g.basis(), token, "application/json", roh.clone()).await;
        assert_eq!(
            antwort.status,
            status,
            "{}: {}",
            &roh[..roh.len().min(60)],
            antwort.json
        );
        if !regel.is_empty() {
            assert_eq!(
                antwort.json["regel"],
                regel,
                "{}",
                &roh[..roh.len().min(60)]
            );
        }
    }
    let doppelt = loeschen_roh(
        gamma,
        g.basis(),
        token,
        "application/json",
        format!(
            r#"{{"ids":["{}"],"ids":["{}"],"bestaetigt":true}}"#,
            g.a, g.b
        ),
    )
    .await;
    assert_eq!(
        (&doppelt.json["feld"], &doppelt.json["grund"]),
        (&json!("ids"), &json!("doppelt"))
    );
    let text = loeschen_roh(
        gamma,
        g.basis(),
        token,
        "text/plain",
        json!({"ids": [g.a], "bestaetigt": true}).to_string(),
    )
    .await;
    assert_eq!(text.status, 415);
    assert_eq!(
        abrufen(gamma, g.basis(), &g.a).await,
        200,
        "nichts gelöscht"
    );
    assert_eq!(abrufen(gamma, g.basis(), &g.b).await, 200);
}

/// Kein Rückgängig: `PUT /api/anhaenge/a` → 404 `unbekannt`; Senden mit `a` → 422 `anhang`/`unbekannt`. Der `PUT`
/// geht roh (Befund #55 B1): die 404 kommt vor dem ersten Körperbyte mit `Connection: close`, ein Client, der den
/// Körper danach schreibt, sieht `EPIPE`/`ECONNRESET` — geprüft wird die Antwort, nicht das Schreiben.
#[tokio::test]
async fn kein_rueckgaengig() {
    let g = gegeben().await;
    let (beta, token) = &g.beta;
    assert_eq!(
        loeschen(
            beta,
            g.basis(),
            token,
            json!({"ids": [g.a], "bestaetigt": true})
        )
        .await
        .status,
        200
    );

    let ohne_koerper = hochladen_abgewiesen(&g.instanz, token, &g.a, 10, None).await;
    let koerper_spaeter = hochladen_abgewiesen(
        &g.instanz,
        token,
        &g.a,
        10,
        Some(std::time::Duration::from_millis(200)),
    )
    .await;
    let gesendet = senden_mit(beta, g.basis(), token, "allgemein", &[&g.a]).await;

    for antwort in [&ohne_koerper, &koerper_spaeter] {
        assert_eq!(
            (antwort.status, &antwort.json["grund"]),
            (404, &json!("unbekannt"))
        );
    }
    assert_eq!(
        (gesendet.status, &gesendet.json["grund"]),
        (422, &json!("unbekannt"))
    );
}

/// Kürzen und Symlink (M45′): die Datei von `c` ist durch einen Symlink auf eine fremde Datei ersetzt → `fehler`,
/// fremde Datei unverändert, `c` wieder `vorhanden` (ein zweiter Versuch ist wieder `fehler`, kein
/// `schon_geloescht`); `GET /anhaenge/c` liefert nie die fremden Bytes.
#[tokio::test]
async fn kuerzen_folgt_keinem_symlink() {
    let g = gegeben().await;
    let (gamma, token) = &g.gamma;
    let c = datei(g.pfad(), "allgemein", "c.bin");
    let fremd = tempfile::NamedTempFile::new().expect("fremd");
    std::fs::write(fremd.path(), vec![9u8; MIB as usize]).expect("fremde Bytes");
    std::fs::remove_file(&c).expect("weg");
    std::os::unix::fs::symlink(fremd.path(), &c).expect("Symlink");
    let vorher = gesundheit(g.basis()).await["anhang_belegt"].clone();

    let antwort = loeschen(
        gamma,
        g.basis(),
        token,
        json!({"ids": [g.c], "bestaetigt": true}),
    )
    .await;
    let erneut = loeschen(
        gamma,
        g.basis(),
        token,
        json!({"ids": [g.c], "bestaetigt": true}),
    )
    .await;

    assert_eq!(
        antwort.json["ergebnisse"][0]["ergebnis"], "fehler",
        "{}",
        antwort.json
    );
    assert_eq!(erneut.json["ergebnisse"][0]["ergebnis"], "fehler");
    assert_eq!(
        std::fs::read(fremd.path()).expect("fremd"),
        vec![9u8; MIB as usize]
    );
    assert_eq!(gesundheit(g.basis()).await["anhang_belegt"], vorher);
    assert_eq!(abrufen(gamma, g.basis(), &g.c).await, 404);
}

/// R00038 F0131 (B0593, ADR-0046 Ergänzung Karte #66 Teil D, wörtlich): die Datei von `c` ist durch eine FIFO gleichen
/// Namens ersetzt → `GET /anhaenge/c` antwortet binnen 2 s mit dem Fehlerstatus des Lesefehlers (404, nicht 200,
/// kein Hängen), ein zweiter Abruf ebenso, `GET /api/gesundheit` bleibt erreichbar; `POST /api/anhaenge/loeschen`
/// endet binnen 2 s, die Antwort nennt den Fehler, `anhang_belegt` ist unverändert. Mutante „ohne `NONBLOCK`“ →
/// der Abruf läuft in die Zeitgrenze; „ohne `S_ISREG`“ → der Abruf liefert 200.
#[tokio::test]
async fn eine_fifo_statt_der_anhangdatei_haengt_weder_abruf_noch_loeschen() {
    use std::time::{Duration, Instant};
    let g = gegeben().await;
    let (gamma, token) = &g.gamma;
    let c = datei(g.pfad(), "allgemein", "c.bin");
    std::fs::remove_file(&c).expect("weg");
    rustix::fs::mknodat(
        rustix::fs::CWD,
        &c,
        rustix::fs::FileType::Fifo,
        rustix::fs::Mode::from_raw_mode(0o644),
        0,
    )
    .expect("mkfifo");
    let vorher = gesundheit(g.basis()).await["anhang_belegt"].clone();
    let frist = Duration::from_secs(2);

    for versuch in 1..=2 {
        let start = Instant::now();
        let status = tokio::time::timeout(frist, abrufen(gamma, g.basis(), &g.c))
            .await
            .unwrap_or_else(|_| panic!("Abruf {versuch} hängt (Frist 2 s)"));
        assert_eq!(status, 404, "Abruf {versuch}");
        assert!(start.elapsed() < frist, "Abruf {versuch}");
    }
    assert_eq!(gesundheit(g.basis()).await["anhang_belegt"], vorher);

    let antwort = tokio::time::timeout(
        frist,
        loeschen(
            gamma,
            g.basis(),
            token,
            json!({"ids": [g.c], "bestaetigt": true}),
        ),
    )
    .await
    .expect("Löschen hängt (Frist 2 s)");
    assert_eq!(
        antwort.json["ergebnisse"][0]["ergebnis"], "fehler",
        "{}",
        antwort.json
    );
    assert_eq!(gesundheit(g.basis()).await["anhang_belegt"], vorher);
}

/// Gelöscht ausliefern (B0509, M14, M44′): `GET` und `HEAD /anhaenge/a` → 410 `text/plain` mit dem Text, CSP
/// `sandbox`, `nosniff`, `Cache-Control: no-store` — auch mit `If-None-Match: "<a>"` und mit `Range` (nie 304, nie
/// 206); ohne Gespräch, Tag oder Name im Körper.
#[tokio::test]
async fn geloescht_antwortet_410_vor_etag_und_bereich() {
    let g = gegeben().await;
    let (gamma, token) = &g.gamma;
    assert_eq!(
        loeschen(
            gamma,
            g.basis(),
            token,
            json!({"ids": [g.a], "bestaetigt": true})
        )
        .await
        .status,
        200
    );
    let etag = format!("\"{}\"", g.a);

    for (methode, kopf) in [
        (reqwest::Method::GET, vec![]),
        (reqwest::Method::HEAD, vec![]),
        (reqwest::Method::GET, vec![("if-none-match", etag.as_str())]),
        (reqwest::Method::GET, vec![("range", "bytes=0-9")]),
    ] {
        let mut anfrage = gamma.anfrage(methode.clone(), format!("{}anhaenge/{}", g.basis(), g.a));
        for (name, wert) in &kopf {
            anfrage = anfrage.header(*name, *wert);
        }
        let antwort = anfrage.send().await.expect("Antwort");
        let wert = |name: &str| {
            antwort
                .headers()
                .get(name)
                .and_then(|w| w.to_str().ok())
                .unwrap_or("")
                .to_string()
        };
        assert_eq!(antwort.status(), 410, "{methode} {kopf:?}");
        assert_eq!(
            wert("content-security-policy"),
            "sandbox; default-src 'none'"
        );
        assert_eq!(wert("x-content-type-options"), "nosniff");
        assert_eq!(wert("cache-control"), "no-store");
        assert_eq!(wert("content-type"), "text/plain; charset=utf-8");
        assert_eq!(wert("cross-origin-resource-policy"), "same-origin");
        if methode == reqwest::Method::GET {
            assert_eq!(
                antwort.text().await.expect("Text"),
                "Dieser Anhang wurde gelöscht. Die Nachricht bleibt."
            );
        }
    }
}

/// Neustart (B0510): nach dem Löschen bleiben `a` und `b` 410, `anhang_belegt` wie vorher, keine `WARNING laden`
/// für die 0-Byte-Dateien (Mutante „Länge 0 ist Waise“).
#[tokio::test]
async fn geloescht_bleibt_nach_dem_neustart() {
    let g = gegeben().await;
    let (gamma, token) = &g.gamma;
    assert_eq!(
        loeschen(
            gamma,
            g.basis(),
            token,
            json!({"ids": [g.a, g.b], "bestaetigt": true})
        )
        .await
        .status,
        200
    );
    let belegt = gesundheit(g.basis()).await["anhang_belegt"].clone();
    let Gegeben {
        wurzel,
        instanz,
        a,
        b,
        ..
    } = g;
    drop(instanz);

    let instanz = starten(wurzel.path(), &[], &[]);

    let leser = Absender::von("127.0.0.1");
    for id in [&a, &b] {
        assert_eq!(abrufen(&leser, &instanz.adresse, id).await, 410);
    }
    assert_eq!(gesundheit(&instanz.adresse).await["anhang_belegt"], belegt);
    drop(instanz);
    let laden: Vec<String> = gemeinsam::protokoll::alle_zeilen(wurzel.path())
        .into_iter()
        .filter(|zeile| zeile.contains("WARNING laden"))
        .collect();
    assert!(laden.is_empty(), "{laden:#?}");
}

fn zeilen_mit(ordner: &Path, text: &str) -> Vec<String> {
    gemeinsam::protokoll::alle_zeilen(ordner)
        .into_iter()
        .filter(|zeile| zeile.contains(text))
        .collect()
}

/// Protokoll und Health (B0511, K-b, M19, M51): genau eine `INFORMATION anhang Anhänge gelöscht` mit Kennung,
/// Herkunft und Zahlen; Health `anhaenge_geloescht` 2; nie eine ID im Protokoll; eine echte und eine ausgedachte ID
/// → genau eine weitere Zeile; einziges Kürzen scheitert → `fehler=1`; nur ausgedachte → keine `INFORMATION`, aber
/// `DEBUG anhang Löschen ohne Wirkung`; je Datei `DEBUG anhang Anhang gelöscht` nur mit Präfix.
#[tokio::test]
async fn protokoll_und_health() {
    let g = gegeben_mit(&["--protokoll-stufe", "debug"]).await;
    let (gamma, token) = &g.gamma;
    let ausgedacht = "b".repeat(26);

    assert_eq!(
        loeschen(
            gamma,
            g.basis(),
            token,
            json!({"ids": [g.a, g.b], "bestaetigt": true})
        )
        .await
        .status,
        200
    );
    assert_eq!(gesundheit(g.basis()).await["anhaenge_geloescht"], 2);
    for _ in 0..3 {
        loeschen(
            gamma,
            g.basis(),
            token,
            json!({"ids": [ausgedacht], "bestaetigt": true}),
        )
        .await;
    }
    let c = datei(g.pfad(), "allgemein", "c.bin");
    let fremd = tempfile::NamedTempFile::new().expect("fremd");
    std::fs::remove_file(&c).expect("weg");
    std::os::unix::fs::symlink(fremd.path(), &c).expect("Symlink");
    loeschen(
        gamma,
        g.basis(),
        token,
        json!({"ids": [g.c, ausgedacht], "bestaetigt": true}),
    )
    .await;
    assert_eq!(
        gesundheit(g.basis()).await["anhaenge_geloescht"],
        2,
        "ohne Wirkung zählt nicht"
    );
    let Gegeben {
        wurzel,
        instanz,
        a,
        b,
        c: id_c,
        ..
    } = g;
    drop(instanz);

    let information = zeilen_mit(wurzel.path(), "INFORMATION anhang Anhänge gelöscht");
    assert_eq!(information.len(), 2, "{information:#?}");
    assert!(information[0].ends_with(
        "| kennung=gamma@127.0.0.1 herkunft=127.0.0.1 anhaenge=2 bytes=2097152 gespraeche=2 fremde=2 fehler=0"
    ), "{}", information[0]);
    assert!(
        information[1].contains("anhaenge=0") && information[1].contains("fehler=1"),
        "{}",
        information[1]
    );
    let ohne_wirkung = zeilen_mit(wurzel.path(), "DEBUG anhang Löschen ohne Wirkung");
    assert_eq!(ohne_wirkung.len(), 3);
    assert!(
        ohne_wirkung[0].contains("ids=1 unbekannt=1 schon_geloescht=0"),
        "{}",
        ohne_wirkung[0]
    );
    assert_eq!(
        zeilen_mit(wurzel.path(), "DEBUG anhang Anhang gelöscht").len(),
        2
    );
    assert_eq!(
        zeilen_mit(wurzel.path(), "ERROR anhang Löschen gescheitert").len(),
        1
    );
    for zeile in gemeinsam::protokoll::alle_zeilen(wurzel.path()) {
        for id in [&a, &b, &id_c, &ausgedacht] {
            assert!(!zeile.contains(id.as_str()), "{id} in {zeile}");
        }
        assert!(
            !zeile.contains("a.bin") && !zeile.contains("c.bin"),
            "{zeile}"
        );
    }
}
