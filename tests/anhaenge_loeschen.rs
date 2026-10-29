//! R00029 F0116 — Anhänge löschen über die API, über das echte Programm. Gegeben: `beta` hat `a` in `# allgemein`
//! und `b` in `# technik` gesendet, `gamma` hat `c` in `# allgemein` gesendet (je 1.048.576 Bytes).

mod gemeinsam;

use gemeinsam::Absender;
use gemeinsam::anhaenge::{MIB, ablegen, anmelden, gesundheit, hochladen, id, starten};
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
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
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

/// Kein Rückgängig: `PUT /api/anhaenge/a` → 404 `unbekannt`; Senden mit `a` → 422 `anhang`/`unbekannt`.
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

    let hoch = hochladen(beta, g.basis(), token, &g.a, vec![7; MIB as usize]).await;
    let gesendet = senden_mit(beta, g.basis(), token, "allgemein", &[&g.a]).await;

    assert_eq!(
        (hoch.status, &hoch.json["grund"]),
        (404, &json!("unbekannt"))
    );
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
