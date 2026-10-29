//! R00029 F0097 „Kennzahlen und ID gekürzt“ (B0432, M11, M18, M22) und „Anhang-Quote einstellen“ (Werte im
//! Health-Check) über das echte Programm.

mod gemeinsam;

use gemeinsam::Absender;
use gemeinsam::anhaenge::{Q, anmelden, gesundheit, hochladen, id, starten};
use gemeinsam::protokoll::alle_zeilen;
use gemeinsam::senden::{beitreten, ordner};

/// Die Felder der Anhang-Quote; `belegt` gibt es nicht. Eine Anmeldung reserviert im Zählmaß; eine 507 `quote`
/// zählt in `anhang_abgewiesen`.
#[tokio::test]
async fn der_health_check_nennt_die_anhang_quote_und_ihre_belegung() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let basis = instanz.adresse.as_str();
    let leer = gesundheit(basis).await;
    assert_eq!(leer["anhangquote"], 10_485_760);
    assert_eq!(leer["anhang_belegt"], 0);
    assert_eq!(leer["reserviert"], 0);
    assert_eq!(leer["hochladungen"], 0);
    assert_eq!(leer["hochladebank_belegt"], 0);
    assert_eq!(leer["anhang_abgewiesen"], 0);
    assert!(leer.get("belegt").is_none(), "{leer}");

    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, basis, "beta", Some("agent")).await;
    id(&anmelden(&beta, basis, &token, "bericht.pdf", 307_200).await);
    let eine = gesundheit(basis).await;
    assert_eq!(
        (
            eine["reserviert"].as_u64(),
            eine["anhang_belegt"].as_u64(),
            eine["hochladungen"].as_u64()
        ),
        (Some(307_200), Some(307_200), Some(1))
    );

    id(&anmelden(&beta, basis, &token, "rest.bin", Q - 307_200).await);
    assert_eq!(
        anmelden(&beta, basis, &token, "zu.viel", 1).await.status,
        507
    );
    let voll = gesundheit(basis).await;
    assert_eq!(voll["anhang_abgewiesen"], 1);
    assert_eq!(voll["reserviert"], Q);
}

/// Ohne Angabe 20 GiB; Option und Umgebung; `aus` → `null`; gültige Option übersteuert falsche Umgebung; die
/// Ränder 16.384 GiB und 1 MiB starten.
#[tokio::test]
async fn die_anhang_quote_kommt_aus_option_umgebung_oder_standard() {
    type Fall<'a> = (&'a [&'a str], &'a [(&'a str, &'a str)], serde_json::Value);
    let faelle: [Fall; 6] = [
        (&[], &[], serde_json::json!(21_474_836_480u64)),
        (
            &["--anhangquote", "512MiB"],
            &[],
            serde_json::json!(536_870_912),
        ),
        (
            &[],
            &[("LOCALCHATA_ANHANGQUOTE", "1gib")],
            serde_json::json!(1_073_741_824),
        ),
        (&["--anhangquote", "aus"], &[], serde_json::Value::Null),
        (
            &["--anhangquote", "1GiB"],
            &[("LOCALCHATA_ANHANGQUOTE", "kaputt")],
            serde_json::json!(1_073_741_824),
        ),
        (
            &["--anhangquote", "16384GiB"],
            &[],
            serde_json::json!(17_592_186_044_416u64),
        ),
    ];
    for (argumente, umgebung, erwartet) in faelle {
        let wurzel = ordner();
        let mut alle = vec!["--port", "0", "--host", "127.0.0.1", "--kein-browser"];
        alle.extend_from_slice(argumente);
        let mut mit_tz = vec![("TZ", "Europe/Berlin")];
        mit_tz.extend_from_slice(umgebung);
        let instanz = gemeinsam::starten_wie_ausgeliefert(wurzel.path(), &alle, &mit_tz);
        assert_eq!(
            gesundheit(&instanz.adresse).await["anhangquote"],
            erwartet,
            "{argumente:?} {umgebung:?}"
        );
    }
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &["--anhangquote", "1MiB"], &[]);
    assert_eq!(gesundheit(&instanz.adresse).await["anhangquote"], 1_048_576);
}

/// Mit `--protokoll-stufe debug`: nach Anmelden und Hochladen steht die ID nicht im Protokoll; verfehlte Routen mit
/// der ID — Pfad dahinter, Schrägstrich, prozentkodiert, Großbuchstaben — stehen nur mit 6 Zeichen da (M18).
#[tokio::test]
async fn die_id_steht_nie_im_protokoll_auch_nicht_bei_verfehlten_routen() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &["--protokoll-stufe", "debug"], &[]);
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, basis, "beta", Some("agent")).await;
    let id = id(&anmelden(&beta, basis, &token, "bericht.pdf", 10).await);
    assert_eq!(
        hochladen(&beta, basis, &token, &id, vec![1; 10])
            .await
            .status,
        200
    );

    let kodiert = format!("%{:02X}{}", id.as_bytes()[0], &id[1..]);
    for pfad in [
        format!("anhaenge/{id}/x"),
        format!("api/anhaenge/{id}/"),
        format!("anhaenge/{kodiert}/x"),
        format!("anhaenge/{}/x", id.to_uppercase()),
    ] {
        let antwort = reqwest::get(format!("{basis}{pfad}"))
            .await
            .expect("Antwort");
        assert_eq!(antwort.status(), 404, "{pfad}");
    }
    drop(instanz);

    let zeilen = alle_zeilen(wurzel.path());
    let klein = id.to_lowercase();
    let mit_id: Vec<&String> = zeilen
        .iter()
        .filter(|zeile| zeile.to_lowercase().contains(&klein))
        .collect();
    assert!(mit_id.is_empty(), "{mit_id:#?}");
    let gekuerzt = zeilen
        .iter()
        .filter(|zeile| {
            zeile.contains(&format!("{}…", &id[..6]))
                || zeile.contains(&format!("{}…", &id[..6].to_uppercase()))
        })
        .count();
    assert!(gekuerzt >= 4, "{zeilen:#?}");
}
