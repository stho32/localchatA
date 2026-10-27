//! F0009 Sitzung wiedererkennen und nach der Gnadenfrist freigeben — über das echte Programm
//! (R00003, US-11 bis US-14 und Kriterien).
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::{Absender, aufrufen_mit, starten_mit};
use serde_json::json;

#[test]
fn us14_unlesbare_gnadenfrist_endet_mit_exit_zwei_und_nennt_die_variable() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let ordner = wurzel.path().display().to_string();

    let ende = aufrufen_mit(
        &[&ordner, "--port", "0", "--kein-browser"],
        &[("LOCALCHATA_GNADENFRIST_MS", "bald")],
    );

    assert_eq!(ende.exit, Some(2));
    assert!(
        ende.stderr.contains("LOCALCHATA_GNADENFRIST_MS"),
        "{}",
        ende.stderr
    );
    assert_eq!(ende.stdout, "", "keine Startzeile");
}

#[tokio::test]
async fn die_gnadenfrist_aus_der_umgebung_steht_in_der_beitrittsantwort() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let instanz = starten_mit(
        wurzel.path(),
        &["--port", "0", "--host", "127.0.0.1", "--kein-browser"],
        &[("LOCALCHATA_GNADENFRIST_MS", "3000")],
    );

    let antwort = Absender::von("127.0.0.1")
        .beitreten(&instanz.adresse, json!({"name": "claude"}), None)
        .await;

    assert_eq!(antwort.json["gnadenfrist_s"], 3, "{}", antwort.json);
}

fn token_aus(antwort: &gemeinsam::ApiAntwort) -> String {
    antwort.json["token"].as_str().expect("Token").to_string()
}

#[tokio::test]
async fn us11_das_token_weist_mich_aus_nur_von_meiner_herkunft() {
    let instanz = gemeinsam::wegwerf_starten();
    let (eins, zwei) = (Absender::von("127.0.0.1"), Absender::von("127.0.0.2"));
    let beitritt = zwei
        .beitreten(&instanz.adresse, json!({"name": "claude"}), None)
        .await;
    let token = token_aus(&beitritt);

    let ich = zwei.ich(&instanz.adresse, Some(&token)).await;
    let fremd = eins.ich(&instanz.adresse, Some(&token)).await;
    let ohne = zwei.ich(&instanz.adresse, None).await;
    let null = zwei
        .ich(&instanz.adresse, Some("00000000000000000000000000000000"))
        .await;

    assert_eq!(ich.status, 200, "{}", ich.json);
    assert_eq!(
        ich.json,
        json!({"kennung": "claude@127.0.0.2", "name": "claude", "herkunft": "127.0.0.2", "art": "agent"})
    );
    assert_eq!(ich.kopf["cache-control"], "no-store");
    for antwort in [fremd, ohne, null] {
        assert_eq!(antwort.status, 401);
        assert!(antwort.json["fehler"].is_string(), "{}", antwort.json);
    }
}

/// US-13 mit `LOCALCHATA_GNADENFRIST_MS=3000`: t = 0 s beigetreten, t = 2 s Lebenszeichen über `/api/ich`,
/// t = 4 s vergeben (2 s ≤ 3 s), t = 6,5 s frei (4,5 s > 3 s), danach ist das alte Token 401.
#[tokio::test]
async fn us13_nach_der_gnadenfrist_ist_der_name_frei() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let instanz = starten_mit(
        wurzel.path(),
        &["--port", "0", "--host", "127.0.0.1", "--kein-browser"],
        &[("LOCALCHATA_GNADENFRIST_MS", "3000")],
    );
    let (eins, zwei) = (Absender::von("127.0.0.1"), Absender::von("127.0.0.2"));
    let start = tokio::time::Instant::now();
    let bis = |sekunden: f64| {
        tokio::time::sleep_until(start + std::time::Duration::from_secs_f64(sekunden))
    };
    let token = token_aus(
        &zwei
            .beitreten(&instanz.adresse, json!({"name": "claude"}), None)
            .await,
    );

    bis(2.0).await;
    let lebenszeichen = zwei.ich(&instanz.adresse, Some(&token)).await;
    bis(4.0).await;
    let vergeben = eins
        .beitreten(&instanz.adresse, json!({"name": "claude"}), None)
        .await;
    bis(6.5).await;
    let frei = eins
        .beitreten(&instanz.adresse, json!({"name": "claude"}), None)
        .await;
    let altes_token = zwei.ich(&instanz.adresse, Some(&token)).await;

    assert_eq!(lebenszeichen.status, 200);
    assert_eq!(vergeben.status, 409, "{}", vergeben.json);
    assert_eq!(frei.status, 201, "{}", frei.json);
    assert_eq!(frei.json["kennung"], "claude@127.0.0.1");
    assert_eq!(altes_token.status, 401);
}
