//! R00019 F0063 Ordner nicht lesbar — über das echte Programm. Nach der Startzeile nimmt `chmod a-r` dem Ordner
//! eines Channels das Leserecht (Auflisten scheitert, Dateien mit bekanntem Namen bleiben erreichbar). Läuft nicht
//! als root (dort wirkt `chmod` nicht).
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::frist::{KURZ_VOR_MITTERNACHT, Rechte, starten_um, tag_anlegen, tag_da};
use gemeinsam::protokoll::zeilen;
use std::time::{Duration, Instant};

/// Z3 und Z4 aus R00008.
const Z3: Duration = Duration::from_secs(4);
const Z4: Duration = Duration::from_secs(3);

const TAGE_NICHT_ERMITTELT: &str = "ERROR frist Tage nicht ermittelt — Ordner nicht lesbar; erneuter Versuch beim nächsten Anlass | channel=allgemein datei=channels/allgemein";

fn frist_zeilen(ordner: &std::path::Path, tag: &str) -> Vec<String> {
    zeilen(ordner, tag)
        .into_iter()
        .filter(|zeile| zeile.contains(" frist "))
        .map(|zeile| zeile.split_once(' ').expect("Zeitstempel").1.to_string())
        .collect()
}

/// R00019 F0063 „Ruhe am Tageswechsel“: `allgemein` mit 20.(1), 28.(1), `zweiter` mit 20.(1), 22.(1); sofort nach
/// der Startzeile `chmod a-r channels/allgemein` → innerhalb Z3 genau eine Zeile „Tage nicht ermittelt“ mit
/// `grund=tageswechsel` und für `zweiter` das Entfallen des 22.; in Z4 danach keine weitere Frist-Zeile.
#[test]
fn am_tageswechsel_meldet_ein_nicht_lesbarer_ordner_sich_einmal_die_uebrigen_werden_gepflegt() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let ordner = wurzel.path();
    tag_anlegen(ordner, "allgemein", "2026-10-20", 1);
    tag_anlegen(ordner, "allgemein", "2026-10-28", 1);
    tag_anlegen(ordner, "zweiter", "2026-10-20", 1);
    tag_anlegen(ordner, "zweiter", "2026-10-22", 1);

    let mut instanz = starten_um(ordner, KURZ_VOR_MITTERNACHT);
    let startzeile = Instant::now();
    let _leseschutz = Rechte::setzen(&ordner.join("channels/allgemein"), 0o311);
    let stderr = instanz.stderr_mitschreiben();

    let tageswechsel = || -> Vec<String> {
        frist_zeilen(ordner, "2026-10-29")
            .into_iter()
            .filter(|zeile| zeile.contains("grund=tageswechsel"))
            .collect()
    };
    while tageswechsel().len() < 2 && startzeile.elapsed() < Duration::from_secs(10) {
        std::thread::sleep(Duration::from_millis(25));
    }
    let gemessen = startzeile.elapsed();
    println!("Z3 gemessen: {} ms", gemessen.as_millis());
    let mut neuer_tag = frist_zeilen(ordner, "2026-10-29");
    neuer_tag.sort();

    assert!(gemessen <= Z3, "{gemessen:?} > Z3, {neuer_tag:#?}");
    assert_eq!(neuer_tag.len(), 2, "{neuer_tag:#?}");
    assert!(
        neuer_tag[0].starts_with(&format!(
            "{TAGE_NICHT_ERMITTELT} grund=tageswechsel ursache=\""
        )) && neuer_tag[0].ends_with('"'),
        "{neuer_tag:#?}"
    );
    assert!(
        neuer_tag[1].starts_with("INFORMATION frist 1 Tag entfallen (2026-10-22;")
            && neuer_tag[1].ends_with("| channel=zweiter anzahl=1 grund=tageswechsel"),
        "{neuer_tag:#?}"
    );
    assert!(!tag_da(ordner, "zweiter", "2026-10-22"));
    assert!(
        !frist_zeilen(ordner, "2026-10-28")
            .iter()
            .chain(&neuer_tag)
            .any(|zeile| zeile.contains("Tag nicht entfallen")),
        "{neuer_tag:#?}"
    );

    std::thread::sleep(Z4);

    assert_eq!(frist_zeilen(ordner, "2026-10-29").len(), 2);
    let auf_stderr: Vec<String> = stderr
        .lock()
        .expect("stderr")
        .iter()
        .filter(|zeile| zeile.contains(" frist "))
        .cloned()
        .collect();
    assert_eq!(auf_stderr.len(), 1, "{auf_stderr:#?}");
    assert!(
        auf_stderr[0].contains(TAGE_NICHT_ERMITTELT),
        "{auf_stderr:#?}"
    );
}

const FRIST_NICHT_GESETZT: &str = "ERROR frist Frist nicht gesetzt — Tage nicht ermittelt | channel=allgemein kennung=beta@127.0.0.1 datei=channels/allgemein ursache=\"";
const NACH_DEM_SETZEN: &str = " grund=frist_gesetzt kennung=beta@127.0.0.1 ursache=\"";

/// R00019 F0063 „Verlängern trotz nicht lesbarem Ordner“, „Verkürzen bei nicht lesbarem Ordner“ und „Bestätigt
/// verkürzen bei nicht lesbarem Ordner“ (US-1 bis US-3), im Anschluss: `allgemein` ohne Einstellungen mit 28.(1);
/// nach der Startzeile `chmod a-r channels/allgemein`.
#[tokio::test]
async fn verlaengern_gelingt_verkuerzen_sagt_woran_es_liegt_bestaetigt_gelingt() {
    use gemeinsam::Absender;
    use gemeinsam::frist::{MITTAG, beitreten, channel_in_liste, frist_setzen};
    use gemeinsam::strom::Strom;
    use serde_json::json;
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let ordner = wurzel.path();
    tag_anlegen(ordner, "allgemein", "2026-10-28", 1);
    let mut instanz = starten_um(ordner, MITTAG);
    let leseschutz = Rechte::setzen(&ordner.join("channels/allgemein"), 0o311);
    let stderr = instanz.stderr_mitschreiben();
    let zwei = Absender::von("127.0.0.2");
    let eins = Absender::von("127.0.0.1");
    let alpha = beitreten(&zwei, &instanz, "alpha").await;
    let beta = beitreten(&eins, &instanz, "beta").await;
    let heute = || frist_zeilen(ordner, "2026-10-28");
    let einstellungen = ordner.join("channels/allgemein/einstellungen.toml");

    let verlaengert =
        frist_setzen(&eins, &instanz, &beta, "allgemein", json!({ "tage": 30 })).await;

    assert_eq!(verlaengert.status, 200, "{}", verlaengert.json);
    assert_eq!(
        verlaengert.json["entfallen"],
        json!({ "tage": 0, "nachrichten": 0 })
    );
    let datei = std::fs::read(&einstellungen).expect("einstellungen.toml");
    assert!(
        String::from_utf8_lossy(&datei).contains("frist_tage = 30"),
        "{}",
        String::from_utf8_lossy(&datei)
    );
    let (channel, s) = channel_in_liste(&eins, &instanz, &beta, "allgemein").await;
    assert_eq!(channel["frist_tage"], 30);
    let nach_dem_verlaengern = heute();
    assert!(
        nach_dem_verlaengern.contains(
            &"INFORMATION frist Frist 7 → 30 Tage gesetzt | channel=allgemein kennung=beta@127.0.0.1"
                .to_string()
        ),
        "{nach_dem_verlaengern:#?}"
    );
    let ordner_zeile = format!("{TAGE_NICHT_ERMITTELT}{NACH_DEM_SETZEN}");
    assert_eq!(
        nach_dem_verlaengern
            .iter()
            .filter(|zeile| zeile.starts_with(&ordner_zeile))
            .count(),
        1,
        "{nach_dem_verlaengern:#?}"
    );

    let mut strom = Strom::oeffnen(&zwei, &instanz.adresse, &alpha).await;
    strom
        .bis(Duration::from_secs(5), |e| e.data["art"] == "channels")
        .await
        .expect("Stand");
    let verkuerzt = frist_setzen(&eins, &instanz, &beta, "allgemein", json!({ "tage": 7 })).await;

    assert_eq!(verkuerzt.status, 500, "{}", verkuerzt.json);
    assert_eq!(
        verkuerzt.json,
        json!({ "fehler": "Frist nicht gesetzt — die Tage von # allgemein ließen sich nicht ermitteln (Ordner nicht lesbar). Leserecht von channels/allgemein prüfen und erneut versuchen." })
    );
    let (channel, stand) = channel_in_liste(&eins, &instanz, &beta, "allgemein").await;
    assert_eq!(channel["frist_tage"], 30);
    assert_eq!(stand, s);
    assert_eq!(
        std::fs::read(&einstellungen).expect("einstellungen.toml"),
        datei
    );
    let nach_dem_verkuerzen = heute();
    assert_eq!(
        nach_dem_verkuerzen
            .iter()
            .filter(|zeile| zeile.starts_with(FRIST_NICHT_GESETZT))
            .count(),
        1,
        "{nach_dem_verkuerzen:#?}"
    );
    assert!(
        !nach_dem_verkuerzen
            .iter()
            .any(|zeile| zeile.contains("Einstellungen ließen sich nicht schreiben")),
        "{nach_dem_verkuerzen:#?}"
    );

    let bestaetigt = frist_setzen(
        &eins,
        &instanz,
        &beta,
        "allgemein",
        json!({ "tage": 7, "bestaetigt": true }),
    )
    .await;

    assert_eq!(bestaetigt.status, 200, "{}", bestaetigt.json);
    assert_eq!(bestaetigt.json["channel"]["frist_tage"], 7);
    assert_eq!(
        bestaetigt.json["entfallen"],
        json!({ "tage": 0, "nachrichten": 0 })
    );
    let ereignis = strom
        .bis(Duration::from_secs(5), |e| e.data["art"] == "channel")
        .await
        .expect("Ereignis channel");
    assert_eq!(ereignis.data["stand"], s + 1, "kein Ereignis beim 500");
    assert!(tag_da(ordner, "allgemein", "2026-10-28"));
    assert_eq!(
        heute()
            .iter()
            .filter(|zeile| zeile.starts_with(&ordner_zeile))
            .count(),
        2,
        "{:#?}",
        heute()
    );
    drop(leseschutz);
    let auf_stderr: Vec<String> = stderr
        .lock()
        .expect("stderr")
        .iter()
        .filter(|zeile| zeile.contains(FRIST_NICHT_GESETZT))
        .cloned()
        .collect();
    assert_eq!(auf_stderr.len(), 1, "{auf_stderr:#?}");
}
