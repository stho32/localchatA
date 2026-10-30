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
