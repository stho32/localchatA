//! R00050 F0172 B0767 — Direktgespräche laden über das echte Programm (ADR-0055 Regeln 1, 4 und 11; Matrix O1, O2, P2):
//! `direkt` als Symlink ist Exit 2, jeder ungültige Ordner unter `direkt/` ist **eine** `WARNING laden`-Zeile mit
//! gekürztem `ordner`, und nichts davon erreicht den Health-Check.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::protokoll::{alle_zeilen, ist_protokollzeile, mit, wert};
use gemeinsam::{aufrufen_mit, gesundheit, starten_mit};
use std::path::Path;

const LOKAL: [&str; 5] = ["--port", "0", "--host", "127.0.0.1", "--kein-browser"];

fn satz(zeit: &str) -> String {
    format!("@ {zeit} anna@10.0.0.7\n  text\n\n")
}

fn tag_anlegen(ordner: &Path, name: &str, zeit: &str) {
    let ziel = ordner.join("direkt").join(name);
    std::fs::create_dir_all(&ziel).expect("Ordner");
    std::fs::write(ziel.join("2026-10-28.log"), satz(zeit)).expect("Tagesdatei");
}

/// O1: `direkt` als Symlink auf einen Ordner endet mit Exit 2, nichts wird angelegt, das Ziel bleibt.
#[test]
fn direkt_als_symlink_endet_mit_exit_2_und_aendert_nichts() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let ziel = tempfile::tempdir().expect("Ziel");
    std::fs::create_dir(ziel.path().join("anna@10.0.0.7+ben@10.0.0.8")).expect("Paar");
    std::os::unix::fs::symlink(ziel.path(), wurzel.path().join("direkt")).expect("Symlink");
    let ordner = wurzel.path().display().to_string();
    let mut argumente = vec![ordner.as_str()];
    argumente.extend_from_slice(&LOKAL);

    let ende = aufrufen_mit(&argumente, &[]);

    assert_eq!(ende.exit, Some(2), "{}", ende.stderr);
    let pfad = wurzel.path().join("direkt").display().to_string();
    assert!(
        ende.stderr.contains(&pfad) && ende.stderr.contains("Instanzordner"),
        "{}",
        ende.stderr
    );
    assert!(!wurzel.path().join("logs").exists(), "nichts angelegt");
    assert_eq!(
        std::fs::read_dir(ziel.path()).expect("Ziel").count(),
        1,
        "Ziel unberührt"
    );
}

/// O2, P2 (wörtlich): Ordner `x`, `b@1.1.1.1+a@1.1.1.1`, `a@1.1.1.1+a@1.1.1.1`, drei Teile, `%3a`, `%2F`,
/// Alias-Herkunft, ein Name mit Zeilenumbruch und ein langer Name, dazu die richtig kodierte Kontrolle → je ungültiger
/// Ordner genau eine `WARNING laden`-Zeile, `ordner` höchstens 128 Zeichen, jede Zeile des Protokolls eine
/// Protokollzeile; die Kontrolle ist geladen; `defekte_zeilen` bleibt 0.
#[tokio::test]
async fn jeder_ungueltige_direktordner_ist_genau_eine_warnung_laden_und_der_health_check_bleibt_unberuehrt()
 {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let ordner = wurzel.path();
    let ungueltig = [
        "x".to_string(),
        "b@1.1.1.1+a@1.1.1.1".to_string(),
        "a@1.1.1.1+a@1.1.1.1".to_string(),
        "a@1.1.1.1+b@1.1.1.1+c@1.1.1.1".to_string(),
        "anna@fd00%3a%3a7+ben@1.1.1.1".to_string(),
        "anna@1.1.1.1%2F+ben@1.1.1.1".to_string(),
        "anna@%3A%3Affff%3A1.1.1.1+ben@1.1.1.1".to_string(),
        "evil\n2026-10-28T12:00:00.000+01:00 INFORMATION start fake".to_string(),
        "l".repeat(250),
    ];
    for name in &ungueltig {
        tag_anlegen(ordner, name, "2026-10-28T23:00:00.000+01:00");
    }
    tag_anlegen(
        ordner,
        "anna@fd00%3A%3A7+ben@1.1.1.1",
        "2026-10-28T09:00:00.000+01:00",
    );
    let mut instanz = starten_mit(
        ordner,
        &LOKAL,
        &[
            ("TZ", "Europe/Berlin"),
            ("LOCALCHATA_JETZT", "2026-10-28T12:00:00.000+01:00"),
        ],
    );

    let (status, gesund) = gesundheit(&instanz.adresse).await;
    instanz.signal_und_warten("TERM");

    assert_eq!(status, 200);
    assert_eq!(gesund["defekte_zeilen"], 0, "{gesund}");
    let zeilen = alle_zeilen(ordner);
    assert!(
        zeilen.iter().all(|zeile| ist_protokollzeile(zeile)),
        "jede Zeile eine Protokollzeile (kein Einschleusen): {zeilen:#?}"
    );
    let laden = mit(&zeilen, "WARNING", "laden");
    assert_eq!(laden.len(), ungueltig.len(), "{laden:#?}");
    for zeile in &laden {
        let ordner_wert = wert(zeile, "ordner").expect("ordner=");
        assert!(ordner_wert.starts_with("direkt/"), "{zeile}");
        assert!(ordner_wert.chars().count() <= 128, "{zeile}");
    }
    assert!(
        !zeilen
            .iter()
            .any(|zeile| zeile.starts_with("2026-10-28T12:00:00.000+01:00 INFORMATION start fake")),
        "der Name schleust keine Zeile ein: {zeilen:#?}"
    );
}

/// O1, dritter Teil (wörtlich): `anhaenge` im Paar als Symlink wird nicht gefolgt — ein Senden mit Anhang scheitert
/// mit 500 `nicht gespeichert`, nichts steht im Log, das Ziel des Symlinks bleibt leer, der Anhang liegt wieder im
/// Eingang.
#[tokio::test]
async fn anhaenge_im_paar_als_symlink_wird_nie_gefolgt() {
    use gemeinsam::anhaenge::{ablegen, im_eingang, starten as starten_anhaenge};
    use gemeinsam::direkt::{ANNA_BEN, BEN, paarinhalt, runde_mit};
    use gemeinsam::senden::ordner as senden_ordner;
    use serde_json::json;
    let wurzel = senden_ordner();
    let ziel = tempfile::tempdir().expect("Ziel");
    let paar = wurzel.path().join("direkt").join(ANNA_BEN);
    std::fs::create_dir_all(&paar).expect("Paar");
    std::os::unix::fs::symlink(ziel.path(), paar.join("anhaenge")).expect("Symlink anhaenge");
    let instanz = starten_anhaenge(
        wurzel.path(),
        &[],
        &[("LOCALCHATA_JETZT", "2026-10-28T12:00:00.000+01:00")],
    );
    let r = runde_mit(wurzel, instanz).await;
    let basis = r.instanz.adresse.clone();
    let id = ablegen(
        &r.anna.absender,
        &basis,
        &r.anna.token,
        "notiz.txt",
        b"anhang".to_vec(),
    )
    .await;

    let antwort = r
        .anna
        .senden(
            &basis,
            BEN,
            &json!({"text": "mit Anhang", "anhaenge": [id]}),
        )
        .await;

    assert_eq!(antwort.status, 500, "{}", antwort.json);
    assert_eq!(
        std::fs::read_dir(ziel.path()).expect("Ziel").count(),
        0,
        "Ziel unberührt"
    );
    assert!(
        paarinhalt(r.wurzel.path(), ANNA_BEN).is_empty(),
        "kein Satz im Log"
    );
    assert_eq!(
        im_eingang(r.wurzel.path()).len(),
        1,
        "der Anhang liegt wieder im Eingang"
    );
}
