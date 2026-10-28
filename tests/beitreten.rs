//! F0008 Beitreten mit Namen über die API — über das echte Programm (R00003, US-6 bis US-10 und Kriterien).
//! Absender `127.0.0.1` und `127.0.0.2` (Linux-Loopback); jeder `POST` mit `Content-Type: application/json`.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::{Absender, ApiAntwort, api_antwort, starten, starten_mit, wegwerf_starten};
use serde_json::{Value, json};

const HEADER_SATZ: [&str; 6] = [
    "x-content-type-options",
    "content-security-policy",
    "x-frame-options",
    "referrer-policy",
    "cross-origin-opener-policy",
    "cross-origin-resource-policy",
];

fn ist_token(wert: &Value) -> bool {
    wert.as_str().is_some_and(|text| {
        text.len() == 32
            && text
                .bytes()
                .all(|zeichen| matches!(zeichen, b'0'..=b'9' | b'a'..=b'f'))
    })
}

fn header_satz_mit_no_store(antwort: &ApiAntwort) {
    for name in HEADER_SATZ {
        assert!(
            antwort.kopf.contains_key(name),
            "{name} fehlt: {:?}",
            antwort.kopf
        );
    }
    assert_eq!(antwort.kopf["cache-control"], "no-store");
}

fn kennung(antwort: &ApiAntwort) -> &str {
    antwort.json["kennung"].as_str().unwrap_or_default()
}

#[tokio::test]
async fn us6_beitreten_von_127_0_0_2_ergibt_name_at_herkunft_mit_token() {
    let instanz = wegwerf_starten();

    let antwort = Absender::von("127.0.0.2")
        .beitreten(&instanz.adresse, json!({"name": "claude"}), None)
        .await;

    assert_eq!(antwort.status, 201, "{}", antwort.json);
    assert_eq!(antwort.json["kennung"], "claude@127.0.0.2");
    assert_eq!(antwort.json["name"], "claude");
    assert_eq!(antwort.json["herkunft"], "127.0.0.2");
    assert_eq!(antwort.json["art"], "agent");
    assert_eq!(antwort.json["gnadenfrist_s"], 60);
    assert!(ist_token(&antwort.json["token"]), "{}", antwort.json);
    header_satz_mit_no_store(&antwort);
}

#[tokio::test]
async fn art_mensch_wird_uebernommen() {
    let instanz = wegwerf_starten();

    let antwort = Absender::von("127.0.0.1")
        .beitreten(
            &instanz.adresse,
            json!({"name": "shoff", "art": "mensch"}),
            None,
        )
        .await;

    assert_eq!(antwort.status, 201);
    assert_eq!(antwort.json["art"], "mensch");
    assert_eq!(antwort.json["kennung"], "shoff@127.0.0.1");
}

#[tokio::test]
async fn us7_vergebener_name_wird_abgewiesen_egal_von_wo_mit_traeger_seit_und_vorschlag() {
    let instanz = wegwerf_starten();
    let (eins, zwei) = (Absender::von("127.0.0.1"), Absender::von("127.0.0.2"));
    zwei.beitreten(&instanz.adresse, json!({"name": "claude"}), None)
        .await;

    let vergeben = eins
        .beitreten(&instanz.adresse, json!({"name": "claude"}), None)
        .await;
    let zweiter = eins
        .beitreten(&instanz.adresse, json!({"name": "claude-2"}), None)
        .await;
    let danach = eins
        .beitreten(&instanz.adresse, json!({"name": "claude"}), None)
        .await;

    assert_eq!(vergeben.status, 409, "{}", vergeben.json);
    assert_eq!(vergeben.json["belegt_von"], "claude@127.0.0.2");
    assert_eq!(vergeben.json["vorschlag"], "claude-2");
    let seit = vergeben.json["seit"].as_str().expect("seit");
    assert!(chrono::DateTime::parse_from_rfc3339(seit).is_ok(), "{seit}");
    assert!(vergeben.json["fehler"].is_string());
    assert_eq!(
        (zweiter.status, kennung(&zweiter)),
        (201, "claude-2@127.0.0.1")
    );
    assert_eq!(danach.json["vorschlag"], "claude-3");
}

#[tokio::test]
async fn vorschlag_an_der_laengengrenze_kuerzt_den_stamm() {
    let instanz = wegwerf_starten();
    let absender = Absender::von("127.0.0.1");
    let a32 = "a".repeat(32);
    absender
        .beitreten(&instanz.adresse, json!({ "name": a32 }), None)
        .await;

    let vergeben = absender
        .beitreten(&instanz.adresse, json!({ "name": a32 }), None)
        .await;

    assert_eq!(vergeben.status, 409);
    let erwartet = format!("{}-2", "a".repeat(30));
    assert_eq!(vergeben.json["vorschlag"], erwartet);
    assert_eq!(erwartet.len(), 32);
}

#[tokio::test]
async fn us8_namensregel_ueber_die_route_gueltig_mit_vorschlag_und_ohne() {
    let instanz = wegwerf_starten();
    let absender = Absender::von("127.0.0.1");
    let beitreten =
        |name: String| absender.beitreten(&instanz.adresse, json!({ "name": name }), None);

    for name in ["a".to_string(), "claude-x".to_string(), "b".repeat(32)] {
        assert_eq!(beitreten(name.clone()).await.status, 201, "{name}");
    }
    let mit_vorschlag = [
        ("Jörg".to_string(), "joerg".to_string()),
        ("Claude".into(), "claude".into()),
        ("a_b".into(), "a-b".into()),
        ("a.b".into(), "a-b".into()),
        ("a b".into(), "a-b".into()),
        ("a@b".into(), "a-b".into()),
        ("-a".into(), "a".into()),
        ("a-".into(), "a".into()),
        ("chef\u{200B}".into(), "chef".into()),
        ("\u{202E}abc".into(), "abc".into()),
        ("c".repeat(33), "c".repeat(32)),
    ];
    for (name, vorschlag) in mit_vorschlag {
        let antwort = beitreten(name.clone()).await;
        assert_eq!(antwort.status, 422, "{name:?}");
        assert_eq!(antwort.json["vorschlag"], vorschlag, "{name:?}");
        assert!(antwort.json["regel"].is_string() && antwort.json["fehler"].is_string());
    }
    for name in ["", "___"] {
        let antwort = beitreten(name.to_string()).await;
        assert_eq!(antwort.status, 422, "{name:?}");
        assert!(antwort.json.get("vorschlag").is_none(), "{}", antwort.json);
    }
    let joerg = beitreten("joerg".into()).await;
    assert_eq!(joerg.status, 201, "nichts wurde still umgeformt und belegt");
}

#[tokio::test]
async fn reservierte_namen_und_unbekannte_art_ergeben_422_und_belegen_nichts() {
    let instanz = wegwerf_starten();
    let absender = Absender::von("127.0.0.1");

    let alle = absender
        .beitreten(&instanz.adresse, json!({"name": "alle"}), None)
        .await;
    let hier = absender
        .beitreten(&instanz.adresse, json!({"name": "hier"}), None)
        .await;
    let roboter = absender
        .beitreten(
            &instanz.adresse,
            json!({"name": "x1", "art": "roboter"}),
            None,
        )
        .await;
    let x1 = absender
        .beitreten(&instanz.adresse, json!({"name": "x1"}), None)
        .await;

    assert_eq!((alle.status, hier.status, roboter.status), (422, 422, 422));
    assert!(
        alle.json["fehler"]
            .as_str()
            .unwrap_or_default()
            .contains("reserviert")
    );
    assert_eq!(x1.status, 201);
}

#[tokio::test]
async fn herkunft_ist_kanonisch_ipv4_ueber_127_0_0_1_und_ipv6_ueber_doppelpunkt_1() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let instanz = starten(
        wurzel.path(),
        &["--port", "0", "--host", "::", "--kein-browser"],
    );
    let port = instanz.port();
    let client = reqwest::Client::new();
    let beitreten = |basis: String, name: &str| {
        api_antwort(
            client
                .post(format!("{basis}api/beitreten"))
                .json(&json!({ "name": name })),
        )
    };

    let v4 = beitreten(format!("http://127.0.0.1:{port}/"), "v4").await;
    let v6 = beitreten(format!("http://[::1]:{port}/"), "v6").await;

    assert_eq!(kennung(&v4), "v4@127.0.0.1");
    assert_eq!(kennung(&v6), "v6@::1");
}

#[tokio::test]
async fn us9_hoechstens_64_sitzungen_je_herkunft() {
    let instanz = wegwerf_starten();
    let (eins, zwei) = (Absender::von("127.0.0.1"), Absender::von("127.0.0.2"));
    for nummer in 1..=64 {
        let antwort = eins
            .beitreten(
                &instanz.adresse,
                json!({"name": format!("n{nummer}")}),
                None,
            )
            .await;
        assert_eq!(antwort.status, 201, "n{nummer}");
    }

    let zu_viel = eins
        .beitreten(&instanz.adresse, json!({"name": "n65"}), None)
        .await;
    let anderswo = zwei
        .beitreten(&instanz.adresse, json!({"name": "n65"}), None)
        .await;

    assert_eq!(zu_viel.status, 429);
    assert!(zu_viel.json["fehler"].is_string(), "{}", zu_viel.json);
    assert_eq!(anderswo.status, 201);
}

#[tokio::test]
async fn hoechstens_512_sitzungen_insgesamt() {
    let instanz = wegwerf_starten();
    for letzte_stelle in 1..=8 {
        let absender = Absender::von(&format!("127.0.0.{letzte_stelle}"));
        for nummer in 1..=64 {
            let name = format!("h{letzte_stelle}-{nummer}");
            let antwort = absender
                .beitreten(&instanz.adresse, json!({ "name": name }), None)
                .await;
            assert_eq!(antwort.status, 201, "{name}");
        }
    }

    let ueber = Absender::von("127.0.0.9")
        .beitreten(&instanz.adresse, json!({"name": "neun"}), None)
        .await;

    assert_eq!(ueber.status, 429);
}

#[tokio::test]
async fn zwanzig_gleichzeitige_beitritte_mit_demselben_namen_ergeben_genau_einen_erfolg() {
    let instanz = wegwerf_starten();
    let laeufe: Vec<_> = (1..=20)
        .map(|nummer| {
            let basis = instanz.adresse.clone();
            tokio::spawn(async move {
                Absender::von(&format!("127.0.1.{nummer}"))
                    .beitreten(&basis, json!({"name": "wettlauf"}), None)
                    .await
                    .status
            })
        })
        .collect();
    let mut status = Vec::new();
    for lauf in laeufe {
        status.push(lauf.await.expect("Lauf"));
    }

    status.sort();
    let mut erwartet = vec![201];
    erwartet.extend([409; 19]);
    assert_eq!(status, erwartet);
}

#[tokio::test]
async fn us10_eine_fremde_website_kann_keinen_namen_besetzen_und_ohne_json_kein_beitritt() {
    let instanz = wegwerf_starten();
    let absender = Absender::von("127.0.0.1");
    let url = format!("{}api/beitreten", instanz.adresse);
    let koerper = json!({"name": "opfer"}).to_string();

    let fremd = api_antwort(
        absender
            .anfrage(reqwest::Method::POST, url.clone())
            .header("Origin", "http://evil.example")
            .header("Content-Type", "application/json")
            .body(koerper.clone()),
    )
    .await;
    let als_text = api_antwort(
        absender
            .anfrage(reqwest::Method::POST, url.clone())
            .header("Content-Type", "text/plain")
            .body(json!({"name": "text"}).to_string()),
    )
    .await;
    let ohne_origin = absender
        .beitreten(&instanz.adresse, json!({"name": "opfer"}), None)
        .await;
    let text_danach = absender
        .beitreten(&instanz.adresse, json!({"name": "text"}), None)
        .await;

    assert_eq!(fremd.status, 403);
    assert_eq!(als_text.status, 415);
    for name in HEADER_SATZ {
        assert!(als_text.kopf.contains_key(name), "{name} fehlt an 415");
    }
    assert_eq!(ohne_origin.status, 201, "der Name war nicht belegt");
    assert_eq!(text_danach.status, 201, "415 belegt keinen Namen");
}

#[tokio::test]
async fn us6_das_token_steht_nie_auf_stdout_stderr_oder_im_ordner() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let mut instanz = starten_mit(
        wurzel.path(),
        &["--port", "0", "--host", "127.0.0.1", "--kein-browser"],
        &[],
    );
    let absender = Absender::von("127.0.0.1");
    let mut tokens = Vec::new();
    for name in ["eins", "zwei", "drei"] {
        let antwort = absender
            .beitreten(&instanz.adresse, json!({ "name": name }), None)
            .await;
        tokens.push(antwort.json["token"].as_str().expect("Token").to_string());
    }

    assert!(instanz.signal_und_warten("TERM").success());
    let stdout = instanz.restliche_ausgabe();
    let stderr = instanz.stderr_bis_zum_ende();
    let ausgaben = format!("{}\n{stdout}\n{stderr}", instanz.startzeile);
    let mut dateien = String::new();
    for eintrag in walk(wurzel.path()) {
        dateien.push_str(&String::from_utf8_lossy(
            &std::fs::read(eintrag).expect("lesbar"),
        ));
    }
    for token in tokens {
        assert!(!ausgaben.contains(&token), "Token in der Ausgabe");
        assert!(!dateien.contains(&token), "Token im Ordner");
    }
}

fn walk(ordner: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut dateien = Vec::new();
    for eintrag in std::fs::read_dir(ordner).expect("Ordner").flatten() {
        let pfad = eintrag.path();
        if pfad.is_dir() {
            dateien.extend(walk(&pfad));
        } else {
            dateien.push(pfad);
        }
    }
    dateien
}

#[tokio::test]
async fn kein_name_ueber_den_neustart() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let argumente = ["--port", "0", "--host", "127.0.0.1", "--kein-browser"];
    let mut erste = starten(wurzel.path(), &argumente);
    Absender::von("127.0.0.2")
        .beitreten(&erste.adresse, json!({"name": "claude"}), None)
        .await;
    assert!(erste.signal_und_warten("TERM").success());

    let zweite = starten(wurzel.path(), &argumente);
    let antwort = Absender::von("127.0.0.1")
        .beitreten(&zweite.adresse, json!({"name": "claude"}), None)
        .await;

    assert_eq!(
        (antwort.status, kennung(&antwort)),
        (201, "claude@127.0.0.1")
    );
}

/// Nacharbeit Code-Review A2 / Security-Review A4 (B0069): Körper, die kein JSON-Objekt ergeben, und falsche
/// Content-Types bekommen eine deutsche `{fehler}`-Antwort mit Grund und Ausweg statt des Klartexts von axum.
#[tokio::test]
async fn a2_kaputtes_json_und_falscher_content_type_ergeben_deutsches_fehler_json() {
    let instanz = wegwerf_starten();
    let absender = Absender::von("127.0.0.1");
    let url = format!("{}api/beitreten", instanz.adresse);
    let zu_gross = format!("{{\"name\":\"{}\"}}", "x".repeat(3 * 1024 * 1024));
    let json = Some("application/json");
    let ohne_json = "Content-Type: application/json";
    let kaputt = "kein gültiges JSON";
    let objekt = || String::from("{\"name\":\"x\"}");
    let faelle: [(Option<&str>, String, u16, &str); 9] = [
        (json, "{kaputt".into(), 400, kaputt),
        (json, String::new(), 400, kaputt),
        (json, "{\"name\":".into(), 400, kaputt),
        (json, "[1,".into(), 400, kaputt),
        (Some("text/plain"), objekt(), 415, ohne_json),
        (None, objekt(), 415, ohne_json),
        (
            Some("application/xml"),
            "<name>x</name>".into(),
            415,
            ohne_json,
        ),
        (Some("multipart/form-data"), "name=x".into(), 415, ohne_json),
        (json, zu_gross, 413, "zu groß"),
    ];

    for (content_type, koerper, erwartet, kern) in faelle {
        let mut anfrage = absender.anfrage(reqwest::Method::POST, url.clone());
        if let Some(content_type) = content_type {
            anfrage = anfrage.header("Content-Type", content_type);
        }
        let antwort = api_antwort(anfrage.body(koerper.clone())).await;
        let fall = format!("{content_type:?} {:.20}", koerper);

        assert_eq!(antwort.status, erwartet, "{fall}: {}", antwort.json);
        let fehler = antwort.json["fehler"].as_str().unwrap_or_default();
        assert!(fehler.contains(kern), "{fall}: {fehler:?}");
        assert!(
            fehler.contains("{\"name\":\"shoff\"}"),
            "Ausweg fehlt — {fall}: {fehler:?}"
        );
        assert!(
            antwort.kopf["content-type"]
                .to_str()
                .is_ok_and(|wert| wert.starts_with("application/json")),
            "{fall}: {:?}",
            antwort.kopf
        );
        header_satz_mit_no_store(&antwort);
    }
    let danach = absender
        .beitreten(&instanz.adresse, json!({"name": "x"}), None)
        .await;
    assert_eq!(
        danach.status, 201,
        "kein abgewiesener Körper belegt einen Namen"
    );
}
