//! R00029 F0103 — Anhänge im Betrieb über das echte Programm: Großer Anspruch (B0525), Episoden (B0463),
//! Ereignisse (B0462), Warnschwelle (B0515), Start (B0516), keine ID und kein Name im Protokoll (B0464).

mod gemeinsam;

use gemeinsam::Absender;
use gemeinsam::anhaenge::{anmelden, id, starten, verwerfen};
use gemeinsam::senden::{beitreten, ordner};
use std::path::Path;

fn zeilen_mit(ordner: &Path, text: &str) -> Vec<String> {
    gemeinsam::protokoll::alle_zeilen(ordner)
        .into_iter()
        .filter(|zeile| zeile.contains(text))
        .collect()
}

/// Großer Anspruch (M42′ Bedingung 2): `beta` meldet 3 MiB an (Q 10 MiB) → eine Episodenzeile mit
/// `kennung=beta@127.0.0.1` und `hochladen_bis`; zehnmal anmelden und verwerfen → eine Episode.
#[tokio::test]
async fn grosser_anspruch_als_episode_je_kennung() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, basis, "beta", Some("agent")).await;

    for _ in 0..10 {
        let angemeldet = id(&anmelden(&beta, basis, &token, "gross.bin", 3 * 1_048_576).await);
        assert_eq!(verwerfen(&beta, basis, &token, &angemeldet).await, 204);
    }
    id(&anmelden(&beta, basis, &token, "klein.bin", 1).await);
    drop(instanz);

    let beginn = zeilen_mit(
        wurzel.path(),
        "INFORMATION anhang Großer Anspruch — Episode beginnt",
    );
    assert_eq!(beginn.len(), 1, "{beginn:#?}");
    assert!(
        beginn[0].contains("kennung=beta@127.0.0.1") && beginn[0].contains("hochladen_bis="),
        "{}",
        beginn[0]
    );
    assert!(beginn[0].contains("bytes=3145728"), "{}", beginn[0]);
}
