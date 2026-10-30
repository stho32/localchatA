//! R00050 F0175 B0777 — Direktgespräche entfallen nach 7 Kalendertagen samt Anhängen (ADR-0055 Regel 10, Baustein D5;
//! Matrix O3): über das echte Programm mit gestellter Uhr (heute der 30.10.2026, Frist 7 Tage: der 24. bleibt, der
//! 23. entfällt). Fremdes unter `direkt/` und ein Symlink als Tagesdatei bleiben unberührt.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::direkt::{ANNA_BEN, BEN, runde_mit};
use gemeinsam::senden::{ordner, starten};
use std::path::Path;

const HEUTE: &str = "2026-10-30T12:00:00.000+01:00";
const ANNA_CORA: &str = "anna@127.0.0.2+cora@127.0.0.1";

fn satz(tag: &str, text: &str) -> String {
    format!("@ {tag}T09:00:00.000+02:00 anna@127.0.0.2\n  {text}\n\n")
}

fn datei(wurzel: &Path, pfad: &str, inhalt: &[u8]) {
    let ziel = wurzel.join(pfad);
    std::fs::create_dir_all(ziel.parent().expect("Eltern")).expect("Ordner");
    std::fs::write(ziel, inhalt).expect("Datei");
}

/// O3 (wörtlich): Testuhr, Tage 10-23/10-24, Fremddatei `notiz.txt`, Symlink-Tagesdatei → 10-23 samt Anhängen weg,
/// 10-24 bleibt, Fremdes unberührt, Symlink nicht gefolgt; danach `GET /api/direkt` ohne das Gespräch, dessen einziger
/// Tag entfallen ist.
#[tokio::test]
async fn o3_der_start_entfernt_den_alten_tag_samt_anhaengen_und_laesst_fremdes_und_links_in_ruhe() {
    let wurzel = ordner();
    let w = wurzel.path();
    let paar = format!("direkt/{ANNA_BEN}");
    datei(
        w,
        &format!("{paar}/2026-10-23.log"),
        satz("2026-10-23", "alt").as_bytes(),
    );
    datei(
        w,
        &format!("{paar}/2026-10-24.log"),
        satz("2026-10-24", "neu").as_bytes(),
    );
    datei(w, &format!("{paar}/anhaenge/2026-10-23/x.bin"), b"alt");
    datei(w, &format!("{paar}/anhaenge/2026-10-24/y.bin"), b"neu");
    datei(w, &format!("{paar}/notiz.txt"), b"fremd");
    let ziel = tempfile::tempdir().expect("Ziel des Links");
    datei(ziel.path(), "fremd.log", b"nicht anfassen");
    std::os::unix::fs::symlink(
        ziel.path().join("fremd.log"),
        w.join(format!("{paar}/2026-10-22.log")),
    )
    .expect("Symlink als Tagesdatei");
    datei(
        w,
        &format!("direkt/{ANNA_CORA}/2026-10-23.log"),
        satz("2026-10-23", "nur alt").as_bytes(),
    );
    let instanz = starten(w, &[("LOCALCHATA_JETZT", HEUTE)]);
    let r = runde_mit(wurzel, instanz).await;
    let basis = r.instanz.adresse.clone();
    let w = r.wurzel.path();

    assert!(
        !w.join(format!("{paar}/2026-10-23.log")).exists(),
        "10-23 weg"
    );
    assert!(
        !w.join(format!("{paar}/anhaenge/2026-10-23")).exists(),
        "Anhänge von 10-23 weg"
    );
    assert!(
        w.join(format!("{paar}/2026-10-24.log")).exists(),
        "10-24 bleibt"
    );
    assert_eq!(
        std::fs::read(w.join(format!("{paar}/anhaenge/2026-10-24/y.bin"))).expect("Anhang"),
        b"neu"
    );
    assert_eq!(
        std::fs::read(w.join(format!("{paar}/notiz.txt"))).expect("Fremddatei"),
        b"fremd"
    );
    assert!(
        std::fs::symlink_metadata(w.join(format!("{paar}/2026-10-22.log")))
            .expect("Link bleibt")
            .file_type()
            .is_symlink()
    );
    assert_eq!(
        std::fs::read(ziel.path().join("fremd.log")).expect("Ziel"),
        b"nicht anfassen",
        "Symlink nicht gefolgt"
    );
    assert!(
        !w.join(format!("direkt/{ANNA_CORA}/2026-10-23.log"))
            .exists(),
        "auch der einzige Tag des zweiten Paars entfällt"
    );
    let liste = r.anna.lesen(&basis, BEN).await;
    assert_eq!(liste.status, 200, "{}", liste.json);
    let texte: Vec<&str> = liste.json["nachrichten"]
        .as_array()
        .expect("Liste")
        .iter()
        .filter_map(|nachricht| nachricht["text"].as_str())
        .collect();
    assert_eq!(texte, ["neu"]);
    let uebersicht = gemeinsam::api_antwort(
        r.anna
            .absender
            .anfrage(reqwest::Method::GET, format!("{basis}api/direkt"))
            .header("X-LocalchatA-Sitzung", &r.anna.token),
    )
    .await;
    let kennungen: Vec<&str> = uebersicht.json["direktgespraeche"]
        .as_array()
        .expect("Liste")
        .iter()
        .filter_map(|eintrag| eintrag["kennung"].as_str())
        .collect();
    assert_eq!(
        kennungen,
        [BEN],
        "anna ↔ cora hat keine Nachricht in der Frist mehr"
    );
}

/// F2 und der Schluss von O3 (wörtlich): ein Ordner nach der Frist ist leer → wird entfernt (nur `rmdir`, von innen
/// nach außen); ein Ordner mit Fremddatei bleibt, ein Symlink wird nicht gefolgt (Ziel unberührt); `direkt/` bleibt.
#[tokio::test]
async fn f2_der_leere_paarordner_verschwindet_ein_ordner_mit_fremdem_bleibt() {
    let wurzel = ordner();
    let w = wurzel.path();
    let alt = |name: &str| {
        datei(
            w,
            &format!("direkt/{name}/2026-10-23.log"),
            satz("2026-10-23", "alt").as_bytes(),
        )
    };
    // (1) nur der alte Tag samt Anhang → am Ende nichts mehr da
    let nur_alt = "anna@127.0.0.2+ben@127.0.0.3";
    alt(nur_alt);
    datei(
        w,
        &format!("direkt/{nur_alt}/anhaenge/2026-10-23/x.bin"),
        b"alt",
    );
    // (2) der alte Tag und eine Fremddatei → der Tag geht, Ordner und Fremddatei bleiben
    let mit_fremdem = "anna@127.0.0.2+cora@127.0.0.1";
    alt(mit_fremdem);
    datei(w, &format!("direkt/{mit_fremdem}/notiz.txt"), b"fremd");
    // (3) von Hand angelegt und leer (nie benutzt) → weg
    std::fs::create_dir_all(w.join("direkt/dora@127.0.0.4+erik@127.0.0.5")).expect("leer");
    // (4) nur ein Symlink als Tagesdatei → der Link ist Inhalt, Ordner und Ziel bleiben
    let mit_link = "ben@127.0.0.3+cora@127.0.0.1";
    let ziel = tempfile::tempdir().expect("Ziel des Links");
    datei(ziel.path(), "fremd.log", b"nicht anfassen");
    std::fs::create_dir_all(w.join(format!("direkt/{mit_link}"))).expect("Paar");
    std::os::unix::fs::symlink(
        ziel.path().join("fremd.log"),
        w.join(format!("direkt/{mit_link}/2026-10-22.log")),
    )
    .expect("Link");
    // (5) `anhaenge` als Symlink in einen fremden Ordner → nicht gefolgt, der Paarordner bleibt
    let mit_anhaengelink = "ben@127.0.0.3+dora@127.0.0.4";
    alt(mit_anhaengelink);
    std::fs::create_dir_all(ziel.path().join("anhaenge-ziel/2026-10-23")).expect("Ziel");
    datei(ziel.path(), "anhaenge-ziel/2026-10-23/y.bin", b"fremd");
    std::os::unix::fs::symlink(
        ziel.path().join("anhaenge-ziel"),
        w.join(format!("direkt/{mit_anhaengelink}/anhaenge")),
    )
    .expect("Link anhaenge");
    let mut instanz = starten(w, &[("LOCALCHATA_JETZT", HEUTE)]);

    instanz.signal_und_warten("TERM");

    assert!(!w.join(format!("direkt/{nur_alt}")).exists(), "leer → weg");
    assert!(w.join("direkt").is_dir(), "direkt/ bleibt");
    assert!(
        !w.join("direkt/dora@127.0.0.4+erik@127.0.0.5").exists(),
        "von Hand angelegt und leer → weg"
    );
    assert!(
        !w.join(format!("direkt/{mit_fremdem}/2026-10-23.log"))
            .exists()
    );
    assert_eq!(
        std::fs::read(w.join(format!("direkt/{mit_fremdem}/notiz.txt"))).expect("Fremddatei"),
        b"fremd"
    );
    assert!(
        std::fs::symlink_metadata(w.join(format!("direkt/{mit_link}/2026-10-22.log")))
            .expect("Link bleibt")
            .file_type()
            .is_symlink()
    );
    assert!(
        std::fs::symlink_metadata(w.join(format!("direkt/{mit_anhaengelink}/anhaenge")))
            .expect("anhaenge bleibt")
            .file_type()
            .is_symlink()
    );
    assert_eq!(
        std::fs::read(ziel.path().join("fremd.log")).expect("Ziel"),
        b"nicht anfassen"
    );
    assert_eq!(
        std::fs::read(ziel.path().join("anhaenge-ziel/2026-10-23/y.bin"))
            .expect("Ziel von anhaenge"),
        b"fremd",
        "ein Link als anhaenge wird nicht gefolgt"
    );
}
