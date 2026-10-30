//! R00050 F0175 B0780 — Betriebsprotokoll der Direktgespräche (ADR-0055 Regel 11, Matrix P1, P2; Proben S1, S3 und
//! Optionales des observability-reviewers, Karte #13 Welle 1+2): nichts Inhaltliches, die Route heißt
//! `/api/direkt/{kennung}/…`, Anlegen und Entfallen stehen auf DEBUG, auf Stufe `information` schweigt der Alltag.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::anhaenge::{ablegen, starten as starten_anhaenge};
use gemeinsam::direkt::{ANNA, ANNA_BEN, BEN, runde_mit};
use gemeinsam::protokoll::{mit, wert, zeilen};
use gemeinsam::senden::{JETZT, ordner};
use serde_json::json;

const HEUTE: &str = "2026-10-28";

fn debug_umgebung<'a>(weitere: &[(&'a str, &'a str)]) -> Vec<(&'a str, &'a str)> {
    let mut umgebung = vec![
        ("LOCALCHATA_JETZT", JETZT),
        ("LOCALCHATA_PROTOKOLL_STUFE", "debug"),
    ];
    umgebung.extend_from_slice(weitere);
    umgebung
}

/// P1 (wörtlich): voller Austausch anna ↔ ben bei `--protokoll-stufe debug`, dann `grep` in `logs/` → kein Text, kein
/// Anhang-Anzeigename, keine volle Anhang-ID (nur Präfix), kein Token; `ordner=direkt/<paar>` erlaubt. Der Austausch:
/// Senden mit Anhang, Antworten, Bearbeiten, Löschen, Lesen, Aktivität.
#[tokio::test]
async fn p1_ein_voller_austausch_laesst_keinen_inhalt_im_protokoll() {
    let wurzel = ordner();
    let instanz = starten_anhaenge(wurzel.path(), &[], &debug_umgebung(&[]));
    let r = runde_mit(wurzel, instanz).await;
    let basis = r.instanz.adresse.clone();
    let id = ablegen(
        &r.anna.absender,
        &basis,
        &r.anna.token,
        "geheimname-p1.txt",
        b"geheimer anhang".to_vec(),
    )
    .await;
    let eins = r
        .anna
        .senden(
            &basis,
            BEN,
            &json!({"text": "GEHEIMTEXT-EINS", "anhaenge": [id]}),
        )
        .await;
    assert_eq!(eins.status, 201, "{}", eins.json);
    let zeit = eins.json["nachricht"]["zeit"]
        .as_str()
        .expect("Zeit")
        .to_string();
    let zwei = r
        .ben
        .senden(
            &basis,
            ANNA,
            &json!({"text": "GEHEIMTEXT-ZWEI", "antwort_auf": zeit}),
        )
        .await;
    assert_eq!(zwei.status, 201, "{}", zwei.json);
    let bearbeitet = r
        .anna
        .post(
            &basis,
            BEN,
            "nachrichten/bearbeiten",
            &json!({"zeit": zeit, "text": "GEHEIMTEXT-NEU"}),
        )
        .await;
    assert_eq!(bearbeitet.status, 200, "{}", bearbeitet.json);
    let z2 = zwei.json["nachricht"]["zeit"]
        .as_str()
        .expect("Zeit")
        .to_string();
    let geloescht = r
        .ben
        .post(&basis, ANNA, "nachrichten/loeschen", &json!({"zeit": z2}))
        .await;
    assert_eq!(geloescht.status, 200, "{}", geloescht.json);
    let gelesen = r.anna.lesen(&basis, BEN).await;
    assert_eq!(gelesen.status, 200);

    let mut r = r;
    assert!(r.instanz.signal_und_warten("TERM").success());
    let protokoll = zeilen(r.wurzel.path(), HEUTE).join("\n");

    for verboten in [
        "GEHEIMTEXT",
        "geheimname-p1",
        id.as_str(),
        r.anna.token.as_str(),
        r.ben.token.as_str(),
    ] {
        assert!(
            !protokoll.contains(verboten),
            "{verboten} im Protokoll:\n{protokoll}"
        );
    }
    assert!(
        protokoll.contains(&format!("ordner=direkt/{ANNA_BEN}")),
        "der Ordner darf stehen:\n{protokoll}"
    );
}

/// S1 Auslöser A (wörtlich): `LOCALCHATA_TESTPANIK=verlauf`, anna ↔ ben, ben ruft `GET /api/direkt/anna@…/nachrichten`
/// → `ERROR programmfehler … route="GET /api/direkt/{kennung}/nachrichten" … ordner=direkt/<paar>`; keine der Zeilen
/// trägt das Channel-Muster.
#[tokio::test]
async fn s1_a_die_panik_im_direkt_verlauf_nennt_die_direkt_route_und_den_ordner() {
    let wurzel = ordner();
    let instanz = gemeinsam::senden::starten(
        wurzel.path(),
        &debug_umgebung(&[("LOCALCHATA_TESTPANIK", "verlauf")]),
    );
    let r = runde_mit(wurzel, instanz).await;
    let basis = r.instanz.adresse.clone();
    let _ = r.anna.senden(&basis, BEN, &json!({"text": "hallo"})).await;

    let antwort = r.ben.lesen(&basis, ANNA).await;
    assert_eq!(antwort.status, 500, "{}", antwort.json);

    let datei = zeilen(r.wurzel.path(), HEUTE);
    let fehler = mit(&datei, "ERROR", "programmfehler");
    assert!(!fehler.is_empty(), "{datei:#?}");
    for zeile in &fehler {
        assert_eq!(
            wert(zeile, "route").as_deref(),
            Some("GET /api/direkt/{kennung}/nachrichten"),
            "{zeile}"
        );
        assert!(!zeile.contains("/api/channels/{name}"), "{zeile}");
    }
    assert!(
        fehler
            .iter()
            .any(|zeile| wert(zeile, "ordner").as_deref() == Some(&format!("direkt/{ANNA_BEN}"))),
        "{fehler:#?}"
    );
}

/// S1 Auslöser B (wörtlich): anna sendet an `/api/direkt/ben@…/nachrichten` mit dem Body `{"text":"kaputt` →
/// `WARNING körper Körper abgewiesen — Episode beginnt | … route="POST /api/direkt/{kennung}/nachrichten"`.
#[tokio::test]
async fn s1_b_ein_kaputter_koerper_nennt_die_direkt_route() {
    let wurzel = ordner();
    let instanz = gemeinsam::senden::starten(wurzel.path(), &debug_umgebung(&[]));
    let r = runde_mit(wurzel, instanz).await;
    let basis = r.instanz.adresse.clone();

    let antwort = gemeinsam::api_antwort(
        r.anna
            .absender
            .anfrage(
                reqwest::Method::POST,
                format!("{basis}api/direkt/{BEN}/nachrichten"),
            )
            .header("X-LocalchatA-Sitzung", &r.anna.token)
            .header("Content-Type", "application/json")
            .body("{\"text\":\"kaputt"),
    )
    .await;

    assert_eq!(antwort.status, 400, "{}", antwort.json);
    let datei = zeilen(r.wurzel.path(), HEUTE);
    let koerper = mit(&datei, "WARNING", "körper");
    assert!(!koerper.is_empty(), "{datei:#?}");
    for zeile in &koerper {
        assert_eq!(
            wert(zeile, "route").as_deref(),
            Some("POST /api/direkt/{kennung}/nachrichten"),
            "{zeile}"
        );
    }
    assert!(
        !datei
            .iter()
            .any(|zeile| zeile.contains("/api/channels/{name}")),
        "{datei:#?}"
    );
}

/// S3 (wörtlich): Debug, `GET /api/direkt/ben@…/unfug` → die Zeile nennt `route="GET /api/direkt/{kennung}/…"`, und
/// die Kennung des Gegenübers steht nirgends im Pfad der Zeile.
#[tokio::test]
async fn s3_eine_nicht_passende_direkt_route_schreibt_den_rohen_pfad_nicht() {
    let wurzel = ordner();
    let instanz = gemeinsam::senden::starten(wurzel.path(), &debug_umgebung(&[]));
    let r = runde_mit(wurzel, instanz).await;
    let basis = r.instanz.adresse.clone();

    let antwort = gemeinsam::api_antwort(
        r.anna
            .absender
            .anfrage(
                reqwest::Method::GET,
                format!("{basis}api/direkt/{BEN}/unfug"),
            )
            .header("X-LocalchatA-Sitzung", &r.anna.token),
    )
    .await;

    assert_eq!(antwort.status, 404);
    let datei = zeilen(r.wurzel.path(), HEUTE);
    let abgewiesen: Vec<&String> = datei
        .iter()
        .filter(|zeile| zeile.contains("Anfrage abgewiesen") && zeile.contains("/api/direkt"))
        .collect();
    assert_eq!(abgewiesen.len(), 1, "{datei:#?}");
    assert_eq!(
        wert(abgewiesen[0], "route").as_deref(),
        Some("GET /api/direkt/{kennung}/…")
    );
    assert!(
        !datei
            .iter()
            .any(|zeile| zeile.contains(&format!("{BEN}/unfug"))),
        "{datei:#?}"
    );
}

/// Regel 11: Anlegen eines Paars steht auf DEBUG mit `ordner=` (O2), auf Stufe `information` ändern Anlegen und
/// Senden `protokoll_bytes_heute` nicht — wer ohne Sitzung den Health-Check liest, sieht kein Wachstum.
#[tokio::test]
async fn anlegen_steht_auf_debug_und_auf_information_waechst_das_protokoll_nicht() {
    let wurzel = ordner();
    let instanz = gemeinsam::senden::starten(wurzel.path(), &debug_umgebung(&[]));
    let r = runde_mit(wurzel, instanz).await;
    let basis = r.instanz.adresse.clone();
    r.anna.senden(&basis, BEN, &json!({"text": "hallo"})).await;
    let datei = zeilen(r.wurzel.path(), HEUTE);
    // Sollte 6 (Code-Review #13 F0172): das Senden nennt den Ordner, nicht `channel=`.
    let gesendet = mit(&datei, "DEBUG", "senden");
    assert!(!gesendet.is_empty(), "{datei:#?}");
    for zeile in &gesendet {
        assert_eq!(
            wert(zeile, "ordner").as_deref(),
            Some(format!("direkt/{ANNA_BEN}").as_str()),
            "{zeile}"
        );
        assert!(wert(zeile, "channel").is_none(), "{zeile}");
    }
    let angelegt: Vec<&String> = datei
        .iter()
        .filter(|zeile| zeile.contains(" DEBUG ") && zeile.contains("Direktgespräch angelegt"))
        .collect();
    assert_eq!(angelegt.len(), 1, "{datei:#?}");
    assert_eq!(
        wert(angelegt[0], "ordner").as_deref(),
        Some(format!("direkt/{ANNA_BEN}").as_str())
    );

    let wurzel = ordner();
    let instanz = gemeinsam::senden::starten(wurzel.path(), &[]);
    let r = runde_mit(wurzel, instanz).await;
    let basis = r.instanz.adresse.clone();
    let (_, vorher) = gemeinsam::gesundheit(&basis).await;
    r.anna.senden(&basis, BEN, &json!({"text": "hallo"})).await;
    r.ben.senden(&basis, ANNA, &json!({"text": "zurück"})).await;
    let (_, nachher) = gemeinsam::gesundheit(&basis).await;
    assert_eq!(
        vorher["protokoll_bytes_heute"], nachher["protokoll_bytes_heute"],
        "{vorher} → {nachher}"
    );
}
