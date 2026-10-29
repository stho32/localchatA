//! R00029 F0099 „Neustart“ (B0442) und „Entfallen“ (B0443) über das echte Programm.

mod gemeinsam;

use gemeinsam::Absender;
use gemeinsam::anhaenge::{ablegen, gesundheit, starten};
use gemeinsam::frist::{KURZ_VOR_MITTERNACHT, MITTAG, frist_setzen};
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

/// Sendet am `jetzt` eine Nachricht mit 1.000 Bytes und stoppt; liefert den Tag.
async fn am_tag_senden(ordner: &Path, jetzt: &str) -> String {
    let instanz = starten(ordner, &[], &[("LOCALCHATA_JETZT", jetzt)]);
    let (_, tag) = mit_anhang_senden(&instanz.adresse, "x", vec![0; 1_000]).await;
    tag
}

/// Entfallen beim Start (Frist 7): Nachricht mit Anhang vom 20. Oktober, Start am 28. → Tagesdatei und
/// `anhaenge/<tag>/` entfallen, beide Zähler zählen sie nicht; ein Symlink im Tag auf eine fremde Datei ist weg, sein
/// Ziel bleibt.
#[tokio::test]
async fn mit_dem_tag_entfallen_die_anhaenge() {
    let wurzel = ordner();
    let tag = am_tag_senden(wurzel.path(), "2026-10-20T12:00:00+02:00").await;
    let fremd = tempfile::tempdir().expect("fremd");
    std::fs::write(fremd.path().join("ziel"), b"bleibt").expect("Ziel");
    let anhaenge = wurzel
        .path()
        .join(format!("channels/allgemein/anhaenge/{tag}"));
    symlink(fremd.path().join("ziel"), anhaenge.join("zeiger")).expect("Symlink");

    let instanz = starten(wurzel.path(), &[], &[("LOCALCHATA_JETZT", MITTAG)]);

    let stand = gesundheit(&instanz.adresse).await;
    assert_eq!(
        (stand["anhang_belegt"].as_u64(), stand["log_bytes"].as_u64()),
        (Some(0), Some(0))
    );
    assert!(std::fs::symlink_metadata(&anhaenge).is_err());
    assert!(
        !wurzel
            .path()
            .join(format!("channels/allgemein/{tag}.log"))
            .exists()
    );
    assert_eq!(
        std::fs::read(fremd.path().join("ziel")).expect("Ziel"),
        b"bleibt"
    );
}

/// `anhaenge` als Symlink auf einen fremden Ordner mit gleichnamigem Tag → im fremden Ordner ist nichts gelöscht; der
/// Tag hat beim Laden keinen Eintrag (Symlink im Pfad, B0442), zählt also 0, und das Entfallen meldet sich als nicht
/// entfallen.
#[tokio::test]
async fn entfallen_folgt_keinem_symlink_auf_anhaenge() {
    let wurzel = ordner();
    let tag = am_tag_senden(wurzel.path(), "2026-10-20T12:00:00+02:00").await;
    let anhaenge = wurzel.path().join("channels/allgemein/anhaenge");
    let fremd = tempfile::tempdir().expect("fremd");
    std::fs::create_dir_all(fremd.path().join(&tag)).expect("fremder Tag");
    for eintrag in std::fs::read_dir(anhaenge.join(&tag)).expect("Tag") {
        let eintrag = eintrag.expect("Eintrag");
        std::fs::copy(
            eintrag.path(),
            fremd.path().join(&tag).join(eintrag.file_name()),
        )
        .expect("Kopie");
    }
    std::fs::rename(&anhaenge, wurzel.path().join("echte-anhaenge")).expect("weg");
    symlink(fremd.path(), &anhaenge).expect("Symlink");

    let instanz = starten(wurzel.path(), &[], &[("LOCALCHATA_JETZT", MITTAG)]);

    assert_eq!(gesundheit(&instanz.adresse).await["anhang_belegt"], 0);
    drop(instanz);
    assert_eq!(
        std::fs::read_dir(fremd.path().join(&tag))
            .expect("fremd")
            .count(),
        1,
        "nichts gelöscht"
    );
    assert!(
        std::fs::symlink_metadata(&anhaenge)
            .expect("Link")
            .file_type()
            .is_symlink()
    );
}

/// `anhaenge/<tag>` als Symlink auf einen fremden Ordner → nur der Link ist weg, sein Ziel bleibt (Mutante
/// „`remove_dir_all` auf dem vollen Pfad“).
#[tokio::test]
async fn ein_tag_als_symlink_entfaellt_nur_als_link() {
    let wurzel = ordner();
    let tag = am_tag_senden(wurzel.path(), "2026-10-20T12:00:00+02:00").await;
    let tagordner = wurzel
        .path()
        .join(format!("channels/allgemein/anhaenge/{tag}"));
    let fremd = tempfile::tempdir().expect("fremd");
    std::fs::write(fremd.path().join("x"), b"x").expect("x");
    std::fs::remove_dir_all(&tagordner).expect("weg");
    symlink(fremd.path(), &tagordner).expect("Symlink");

    let instanz = starten(wurzel.path(), &[], &[("LOCALCHATA_JETZT", MITTAG)]);
    drop(instanz);

    assert!(
        std::fs::symlink_metadata(&tagordner).is_err(),
        "nur der Link ist weg"
    );
    assert_eq!(std::fs::read(fremd.path().join("x")).expect("x"), b"x");
}

/// Verkürzen der Frist: Nachricht mit Anhang vom 26. Oktober, Start am 28. (Frist 7: bleibt), `PUT frist {tage: 1}`
/// → der 26. entfällt samt `anhaenge/<tag>/`, `anhang_belegt` 0.
#[tokio::test]
async fn beim_verkuerzen_der_frist_entfallen_die_anhaenge() {
    let wurzel = ordner();
    let tag = am_tag_senden(wurzel.path(), "2026-10-26T12:00:00+01:00").await;
    let instanz = starten(wurzel.path(), &[], &[("LOCALCHATA_JETZT", MITTAG)]);
    assert_eq!(gesundheit(&instanz.adresse).await["anhang_belegt"], 65_536);
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, &instanz.adresse, "beta", Some("agent")).await;

    let gesetzt = frist_setzen(
        &beta,
        &instanz,
        &token,
        "allgemein",
        json!({"tage": 1, "bestaetigt": true}),
    )
    .await;

    assert_eq!(gesetzt.status, 200, "{}", gesetzt.json);
    assert_eq!(gesundheit(&instanz.adresse).await["anhang_belegt"], 0);
    assert!(
        !wurzel
            .path()
            .join(format!("channels/allgemein/anhaenge/{tag}"))
            .exists()
    );
}

/// Tageswechsel: Nachricht mit Anhang vom 22. Oktober, Start 2 s vor Mitternacht am 28. (bleibt), nach Mitternacht
/// entfällt der 22. samt Anhängen ohne Neustart.
#[tokio::test]
async fn beim_tageswechsel_entfallen_die_anhaenge() {
    let wurzel = ordner();
    let tag = am_tag_senden(wurzel.path(), "2026-10-22T12:00:00+02:00").await;
    let instanz = starten(
        wurzel.path(),
        &[],
        &[("LOCALCHATA_JETZT", KURZ_VOR_MITTERNACHT)],
    );
    assert_eq!(gesundheit(&instanz.adresse).await["anhang_belegt"], 65_536);
    let tagordner = wurzel
        .path()
        .join(format!("channels/allgemein/anhaenge/{tag}"));

    let beginn = std::time::Instant::now();
    while gesundheit(&instanz.adresse).await["anhang_belegt"] != 0
        && beginn.elapsed() < std::time::Duration::from_secs(10)
    {
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }

    assert_eq!(gesundheit(&instanz.adresse).await["anhang_belegt"], 0);
    assert!(!tagordner.exists());
}

/// Befund #53 B2: dieselbe Metazeile (dieselbe ID) am 20. und am 25. → je `WARNING laden` „ID doppelt“, kein
/// Eintrag, beide Dateien Waisen (131.072); Frist 3 → der 20. entfällt, 65.536 bleiben.
#[tokio::test]
async fn dieselbe_id_in_zwei_tagen_ist_doppelt() {
    let wurzel = ordner();
    let tag = am_tag_senden(wurzel.path(), "2026-10-25T12:00:00+02:00").await;
    assert_eq!(tag, "2026-10-25");
    let channel = wurzel.path().join("channels/allgemein");
    let tagesdatei = std::fs::read_to_string(channel.join("2026-10-25.log")).expect("Tag");
    std::fs::write(
        channel.join("2026-10-20.log"),
        tagesdatei.replace("2026-10-25T", "2026-10-20T"),
    )
    .expect("20.");
    std::fs::create_dir_all(channel.join("anhaenge/2026-10-20")).expect("Tag");
    for eintrag in std::fs::read_dir(channel.join("anhaenge/2026-10-25")).expect("Tag") {
        let eintrag = eintrag.expect("Eintrag");
        std::fs::copy(
            eintrag.path(),
            channel
                .join("anhaenge/2026-10-20")
                .join(eintrag.file_name()),
        )
        .expect("Kopie");
    }
    let id = tagesdatei
        .lines()
        .find_map(|zeile| zeile.strip_prefix("+ anhang: "))
        .and_then(|rest| rest.split(' ').next())
        .expect("ID")
        .to_string();

    let instanz = starten(
        wurzel.path(),
        &[],
        &[("LOCALCHATA_JETZT", "2026-10-25T13:00:00+02:00")],
    );
    assert_eq!(gesundheit(&instanz.adresse).await["anhang_belegt"], 131_072);
    let beta = Absender::von("127.0.0.1");
    let abrufen = |basis: String| {
        let beta = &beta;
        let id = &id;
        async move {
            beta.anfrage(reqwest::Method::GET, format!("{basis}anhaenge/{id}"))
                .send()
                .await
                .expect("Antwort")
                .status()
                .as_u16()
        }
    };
    assert_eq!(abrufen(instanz.adresse.clone()).await, 404, "kein Eintrag");
    let token = beitreten(&beta, &instanz.adresse, "beta", Some("agent")).await;
    let gesetzt = frist_setzen(
        &beta,
        &instanz,
        &token,
        "allgemein",
        json!({"tage": 3, "bestaetigt": true}),
    )
    .await;
    assert_eq!(gesetzt.status, 200, "{}", gesetzt.json);

    assert_eq!(gesundheit(&instanz.adresse).await["anhang_belegt"], 65_536);
    assert_eq!(abrufen(instanz.adresse.clone()).await, 404, "auch danach");
    drop(instanz);
    let doppelt: Vec<String> = laden_warnungen(wurzel.path())
        .into_iter()
        .filter(|zeile| zeile.contains("ID doppelt"))
        .collect();
    assert!(
        ["2026-10-20.log", "2026-10-25.log"]
            .iter()
            .all(|datei| doppelt.iter().any(|zeile| zeile.contains(datei))),
        "{doppelt:#?}"
    );
    assert!(
        doppelt.iter().all(|zeile| !zeile.contains(&id)),
        "keine ID im Protokoll"
    );
}
