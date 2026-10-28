//! F0051 Protokoll schreibt — über das echte Programm (R00014, US-1 bis US-5 und Kriterien).

mod gemeinsam;

use gemeinsam::protokoll::{ist_protokollzeile, mit, wert, zeilen};
use gemeinsam::{Absender, Instanz, starten_mit};
use serde_json::json;
use std::path::Path;

const ARGUMENTE: [&str; 5] = ["--port", "0", "--host", "127.0.0.1", "--kein-browser"];

fn starten_um(ordner: &Path, jetzt: &str, weitere: &[(&str, &str)]) -> Instanz {
    let mut umgebung = vec![("LOCALCHATA_JETZT", jetzt), ("TZ", "Europe/Berlin")];
    umgebung.extend_from_slice(weitere);
    starten_mit(ordner, &ARGUMENTE, &umgebung)
}

#[tokio::test]
async fn start_steht_mit_version_adresse_ordner_stufe_anzahl_und_dauer_im_protokoll() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let instanz = starten_um(wurzel.path(), "2026-10-28T12:00:00+01:00", &[]);

    let datei = zeilen(wurzel.path(), "2026-10-28");
    let start = mit(&datei, "INFORMATION", "start");
    assert_eq!(start.len(), 1, "{datei:#?}");
    let zeile = &start[0];
    assert!(ist_protokollzeile(zeile), "{zeile}");
    assert_eq!(
        wert(zeile, "version").as_deref(),
        Some(env!("CARGO_PKG_VERSION"))
    );
    assert_eq!(wert(zeile, "adresse"), Some(instanz.adresse.clone()));
    assert_eq!(
        wert(zeile, "ordner"),
        Some(wurzel.path().display().to_string())
    );
    assert_eq!(wert(zeile, "stufe").as_deref(), Some("information"));
    assert_eq!(wert(zeile, "option").as_deref(), Some("standard"));
    assert_eq!(wert(zeile, "anzahl").as_deref(), Some("0"));
    assert!(
        wert(zeile, "dauer_ms").is_some_and(|ms| ms.parse::<u64>().is_ok()),
        "{zeile}"
    );
}

#[tokio::test]
async fn ein_channel_namens_logs_liegt_unter_channels_und_beruehrt_das_protokoll_nicht() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let instanz = starten_um(wurzel.path(), "2026-10-28T12:00:00+01:00", &[]);
    let absender = Absender::von("127.0.0.1");
    let beitritt = absender
        .beitreten(&instanz.adresse, json!({"name": "beta"}), None)
        .await;
    let token = beitritt.json["token"].as_str().expect("Token").to_string();

    let angelegt = absender
        .channel_anlegen(&instanz.adresse, &token, json!({"name": "logs"}))
        .await;

    assert_eq!(angelegt.status, 201, "{:?}", angelegt.json);
    assert!(wurzel.path().join("channels/logs").is_dir());
    let mut inhalt: Vec<String> = std::fs::read_dir(wurzel.path().join("logs"))
        .expect("logs/")
        .map(|eintrag| {
            eintrag
                .expect("Eintrag")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    inhalt.sort();
    assert_eq!(inhalt, ["2026-10-28.log"]);
}

#[tokio::test]
async fn nach_sigterm_stehen_ende_und_beendet_in_dieser_reihenfolge() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let mut instanz = starten_um(wurzel.path(), "2026-10-28T12:00:00+01:00", &[]);

    let status = instanz.signal_und_warten("TERM");

    assert_eq!(status.code(), Some(0));
    let ende = mit(&zeilen(wurzel.path(), "2026-10-28"), "INFORMATION", "ende");
    assert_eq!(ende.len(), 2, "{ende:#?}");
    for zeile in &ende {
        assert!(ist_protokollzeile(zeile), "{zeile}");
        assert_eq!(wert(zeile, "grund").as_deref(), Some("SIGTERM"), "{zeile}");
        assert!(wert(zeile, "dauer_ms").is_some(), "{zeile}");
    }
    assert!(!ende[0].contains("beendet"), "{ende:#?}");
    assert!(ende[1].contains("beendet"), "{ende:#?}");
}

/// 13 defekte Kopfzeilen in einer Tagesdatei.
fn dreizehn_defekte(ordner: &Path) {
    let ziel = ordner.join("channels/allgemein/2026-10-28.log");
    std::fs::create_dir_all(ziel.parent().expect("Elternordner")).expect("Ordner");
    std::fs::write(ziel, "@ kaputt\n\n".repeat(13)).expect("Tagesdatei");
}

#[tokio::test]
async fn ladedefekte_zehn_einzeln_und_der_rest_als_eine_zeile_in_datei_und_auf_stderr() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    dreizehn_defekte(wurzel.path());
    let mut instanz = starten_um(wurzel.path(), "2026-10-28T12:00:00+01:00", &[]);

    let (status, gesundheit) = gemeinsam::gesundheit(&instanz.adresse).await;
    assert!(instanz.signal_und_warten("TERM").success());
    let stderr = instanz.stderr_bis_zum_ende();
    let stdout = instanz.restliche_ausgabe();

    assert_eq!(status, 200);
    assert_eq!(gesundheit["defekte_zeilen"], 13);
    let datei = zeilen(wurzel.path(), "2026-10-28");
    let laden = mit(&datei, "WARNING", "laden");
    assert_eq!(laden.len(), 11, "{laden:#?}");
    for zeile in &laden[..10] {
        assert_eq!(
            wert(zeile, "datei").as_deref(),
            Some("channels/allgemein/2026-10-28.log"),
            "{zeile}"
        );
        assert!(wert(zeile, "zeile").is_some(), "{zeile}");
    }
    assert!(laden[10].contains("und 3 weitere"), "{}", laden[10]);
    // stderr nur im Protokollformat ab Warning, wortgleich in der Datei; stdout nur die Startzeile.
    let stderr_zeilen: Vec<String> = stderr.lines().map(str::to_string).collect();
    assert_eq!(stderr_zeilen, laden, "{stderr}");
    for zeile in &stderr_zeilen {
        assert!(ist_protokollzeile(zeile), "{zeile}");
        assert!(datei.contains(zeile), "{zeile}");
    }
    assert_eq!(stdout, "");
}

/// Alle Protokolldateien als ein Text (wie `grep … logs/*.log`).
fn alle_protokolle(ordner: &Path) -> String {
    let mut text = String::new();
    for eintrag in std::fs::read_dir(ordner.join("logs")).expect("logs/") {
        text.push_str(&std::fs::read_to_string(eintrag.expect("Eintrag").path()).expect("lesbar"));
    }
    text
}

#[tokio::test]
async fn betriebsereignisse_mit_kennung_und_channel_aber_nie_token_beschreibung_oder_kopf() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let instanz = starten_um(
        wurzel.path(),
        "2026-10-28T12:00:00+01:00",
        &[("LOCALCHATA_PROTOKOLL_STUFE", "debug")],
    );
    let absender = Absender::von("127.0.0.1");
    let beitritt = absender
        .beitreten(&instanz.adresse, json!({"name": "beta"}), None)
        .await;
    let token = beitritt.json["token"].as_str().expect("Token").to_string();
    let angelegt = absender
        .channel_anlegen(
            &instanz.adresse,
            &token,
            json!({"name": "planung", "beschreibung": "GEHEIM-BESCHREIBUNG-41"}),
        )
        .await;
    let fremd = absender
        .ich(&instanz.adresse, Some("0123456789abcdef0123456789abcdef"))
        .await;
    let verlassen = absender
        .anfrage(
            reqwest::Method::POST,
            format!("{}api/verlassen", instanz.adresse),
        )
        .header("X-LocalchatA-Sitzung", &token)
        .send()
        .await
        .expect("Antwort");

    assert_eq!(angelegt.status, 201);
    assert_eq!(fremd.status, 401);
    assert_eq!(verlassen.status(), 204);
    let datei = zeilen(wurzel.path(), "2026-10-28");
    let sitzung = mit(&datei, "INFORMATION", "sitzung");
    assert_eq!(sitzung.len(), 2, "{datei:#?}");
    assert!(sitzung[0].contains("beigetreten"), "{sitzung:#?}");
    assert!(sitzung[1].contains("verlassen"), "{sitzung:#?}");
    for zeile in &sitzung {
        assert_eq!(wert(zeile, "kennung").as_deref(), Some("beta@127.0.0.1"));
    }
    let channels = mit(&datei, "INFORMATION", "channels");
    assert_eq!(channels.len(), 1, "{datei:#?}");
    assert_eq!(wert(&channels[0], "channel").as_deref(), Some("planung"));
    assert_eq!(
        wert(&channels[0], "kennung").as_deref(),
        Some("beta@127.0.0.1")
    );
    let alles = alle_protokolle(wurzel.path());
    assert_eq!(alles.matches(&token).count(), 0);
    assert_eq!(alles.matches("0123456789abcdef0123456789abcdef").count(), 0);
    assert_eq!(alles.matches("GEHEIM-BESCHREIBUNG-41").count(), 0);
    assert_eq!(
        alles.to_lowercase().matches("x-localchata-sitzung").count(),
        0
    );
}

#[tokio::test]
async fn rueckfall_ist_logs_eine_datei_startet_die_instanz_und_schreibt_nach_dem_entfernen_wieder()
{
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    std::fs::write(wurzel.path().join("logs"), "keine Ablage").expect("Datei logs");
    let mut instanz = starten_um(wurzel.path(), "2026-10-28T12:00:00+01:00", &[]);
    let (status, vorher) = gemeinsam::gesundheit(&instanz.adresse).await;
    assert_eq!(status, 200);
    assert_eq!(vorher["protokoll_ok"], false, "{vorher}");

    std::fs::remove_file(wurzel.path().join("logs")).expect("Datei logs entfernt");
    let beitritt = Absender::von("127.0.0.1")
        .beitreten(&instanz.adresse, json!({"name": "beta"}), None)
        .await;
    assert_eq!(beitritt.status, 201);
    let (_, nachher) = gemeinsam::gesundheit(&instanz.adresse).await;
    assert_eq!(nachher["protokoll_ok"], true, "{nachher}");
    assert!(instanz.signal_und_warten("TERM").success());
    let stderr = instanz.stderr_bis_zum_ende();

    let stderr_zeilen: Vec<&str> = stderr.lines().collect();
    let warnungen: Vec<&&str> = stderr_zeilen
        .iter()
        .filter(|zeile| zeile.contains(" WARNING protokoll ") && zeile.contains("nicht schreibbar"))
        .collect();
    assert_eq!(warnungen.len(), 1, "{stderr}");
    assert!(wert(warnungen[0], "datei").is_some(), "{stderr}");
    assert!(wert(warnungen[0], "grund").is_some(), "{stderr}");
    assert_eq!(
        stderr_zeilen
            .iter()
            .filter(|zeile| zeile.contains(" INFORMATION start "))
            .count(),
        1,
        "{stderr}"
    );
    let datei = zeilen(wurzel.path(), "2026-10-28");
    let protokoll = mit(&datei, "INFORMATION", "protokoll");
    assert_eq!(protokoll.len(), 1, "{datei:#?}");
    assert!(protokoll[0].contains("wieder schreibbar"), "{datei:#?}");
    assert_eq!(mit(&datei, "INFORMATION", "sitzung").len(), 1, "{datei:#?}");
}
