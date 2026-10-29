//! R00029 F0099 „Neustart“ (B0442) und „Entfallen“ (B0443) über das echte Programm.

mod gemeinsam;

use gemeinsam::Absender;
use gemeinsam::anhaenge::{ablegen, gesundheit, starten};
use gemeinsam::protokoll::alle_zeilen;
use gemeinsam::senden::{beitreten, ordner, roh_senden, verlauf};
use serde_json::{Value, json};
use std::os::unix::fs::symlink;
use std::path::Path;

async fn mit_anhang_senden(basis: &str, text: &str, bytes: Vec<u8>) -> (Value, String) {
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, basis, "beta", Some("agent")).await;
    let id = ablegen(&beta, basis, &token, "bericht.pdf", bytes).await;
    let koerper = json!({"text": text, "anhaenge": [id]})
        .to_string()
        .into_bytes();
    let antwort = roh_senden(
        &beta,
        basis,
        Some(&token),
        "allgemein",
        "application/json",
        koerper,
    )
    .await;
    assert_eq!(antwort.status, 201, "{}", antwort.json);
    let tag = antwort.json["nachricht"]["zeit"].as_str().expect("zeit")[..10].to_string();
    (antwort.json["nachricht"].clone(), tag)
}

async fn liste(basis: &str) -> Value {
    let leser = Absender::von("127.0.0.3");
    let token = beitreten(&leser, basis, "leser", None).await;
    verlauf(&leser, basis, &token, "allgemein").await["nachrichten"].clone()
}

fn laden_warnungen(ordner: &Path) -> Vec<String> {
    alle_zeilen(ordner)
        .into_iter()
        .filter(|zeile| zeile.contains(" WARNING laden "))
        .collect()
}

/// Neustart: dieselbe `anhaenge`-Liste, `anhang_belegt` = 307.200, `log_bytes` = Bytes der Tagesdatei; eine Waise von
/// 1 Byte zählt 65.536, ein Symlink zählt nicht; eine unlesbare Metazeile macht den Satz nicht defekt, der Anhang
/// fehlt, `WARNING laden` nennt Tagesdatei und Zeile.
#[tokio::test]
async fn nach_dem_neustart_sind_die_anhaenge_wieder_da() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let (nachricht, tag) = mit_anhang_senden(&instanz.adresse, "", vec![0; 307_200]).await;
    drop(instanz);

    let instanz = starten(wurzel.path(), &[], &[]);
    let gelesen = liste(&instanz.adresse).await;
    assert_eq!(gelesen[0]["anhaenge"], nachricht["anhaenge"]);
    let stand = gesundheit(&instanz.adresse).await;
    let tagesdatei = wurzel.path().join(format!("channels/allgemein/{tag}.log"));
    assert_eq!(stand["anhang_belegt"], 307_200);
    assert_eq!(
        stand["log_bytes"],
        std::fs::metadata(&tagesdatei).expect("Tag").len()
    );
    drop(instanz);

    let anhaenge = wurzel
        .path()
        .join(format!("channels/allgemein/anhaenge/{tag}"));
    std::fs::write(anhaenge.join("999999999-1-waise.bin"), b"w").expect("Waise");
    let fremd = tempfile::tempdir().expect("fremd");
    std::fs::write(fremd.path().join("gross.bin"), vec![1; 300_000]).expect("fremde Datei");
    symlink(fremd.path().join("gross.bin"), anhaenge.join("zeiger.bin")).expect("Symlink");
    let mut inhalt = std::fs::read_to_string(&tagesdatei).expect("Tag");
    inhalt.push_str(&format!(
        "@ {tag}T23:59:00.000+01:00 beta@127.0.0.1\n+ anhang: zu kurz\n  kaputt\n\n"
    ));
    std::fs::write(&tagesdatei, &inhalt).expect("Tag ergänzt");

    let instanz = starten(wurzel.path(), &[], &[]);
    assert_eq!(
        gesundheit(&instanz.adresse).await["anhang_belegt"],
        307_200 + 65_536
    );
    let gelesen = liste(&instanz.adresse).await;
    assert_eq!(gelesen.as_array().expect("Liste").len(), 2);
    assert_eq!(gelesen[1]["anhaenge"], json!([]));
    assert_eq!(gelesen[1]["text"], "kaputt");
    drop(instanz);
    let zeile = inhalt.lines().count() - 2;
    let warnungen = laden_warnungen(wurzel.path());
    assert!(
        warnungen
            .iter()
            .any(|w| w.contains(&format!("{tag}.log")) && w.contains(&format!("zeile={zeile}"))),
        "{warnungen:#?}"
    );
}

/// L2 beim Laden: `anhaenge/<heute>` durch einen Symlink auf einen fremden Ordner mit gleichnamiger Datei ersetzt →
/// die Anhänge des Tags ohne Eintrag im Verzeichnis (die Metazeile bleibt Teil des Satzes), `WARNING laden` mit
/// Gespräch und Tag, im fremden Ordner nichts gezählt.
#[tokio::test]
async fn ein_tag_als_symlink_bleibt_ohne_anhaenge() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let (_, tag) = mit_anhang_senden(&instanz.adresse, "x", vec![0; 1_000]).await;
    drop(instanz);
    let anhaenge = wurzel
        .path()
        .join(format!("channels/allgemein/anhaenge/{tag}"));
    let fremd = tempfile::tempdir().expect("fremd");
    for eintrag in std::fs::read_dir(&anhaenge).expect("Tag") {
        let eintrag = eintrag.expect("Eintrag");
        std::fs::copy(eintrag.path(), fremd.path().join(eintrag.file_name())).expect("Kopie");
    }
    std::fs::remove_dir_all(&anhaenge).expect("weg");
    symlink(fremd.path(), &anhaenge).expect("Symlink");

    let instanz = starten(wurzel.path(), &[], &[]);
    assert_eq!(gesundheit(&instanz.adresse).await["anhang_belegt"], 0);
    drop(instanz);
    let warnungen = laden_warnungen(wurzel.path());
    assert!(
        warnungen
            .iter()
            .any(|w| w.contains(&format!("tag={tag}")) && w.contains("channels/allgemein")),
        "{warnungen:#?}"
    );
}
