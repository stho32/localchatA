//! F0057 Senden im Betriebsprotokoll (R00016): Bremse, Schreibfehler und Vorlauf als Episoden; das echte Programm
//! mit `LOCALCHATA_EPISODE_RUHE_MS=1000`.

mod gemeinsam;

use gemeinsam::protokoll::{mit, zeilen};
use gemeinsam::senden::{HEUTE, beitreten, ordner, senden, starten, tagesdatei};
use gemeinsam::{Absender, Instanz};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const RUHE: (&str, &str) = ("LOCALCHATA_EPISODE_RUHE_MS", "1000");

/// Wartet höchstens `hoechstens`, bis eine Zeile von `logs/<heute>.log` `passt`; liefert die Wartezeit.
async fn warten_auf(
    ordner: &Path,
    hoechstens: Duration,
    passt: impl Fn(&str) -> bool,
) -> Option<Duration> {
    let beginn = Instant::now();
    while beginn.elapsed() < hoechstens {
        if zeilen(ordner, HEUTE).iter().any(|zeile| passt(zeile)) {
            return Some(beginn.elapsed());
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    None
}

/// Sendet 60 kleine `Schleife-<i>` so schnell wie möglich; liefert die Zahl der 429.
async fn schleife(absender: &Absender, instanz: &Instanz, token: &str) -> usize {
    let mut abgewiesen = 0;
    for nummer in 0..60 {
        let antwort = senden(
            absender,
            &instanz.adresse,
            token,
            "allgemein",
            &format!("Schleife-{nummer}"),
        )
        .await;
        if antwort.status == 429 {
            abgewiesen += 1;
        }
    }
    abgewiesen
}

fn stderr_zeilen(stderr: &Arc<Mutex<Vec<String>>>) -> Vec<String> {
    stderr.lock().expect("stderr").clone()
}

/// „Bremse im Betrieb messen“: genau eine Beginn- und eine Ende-Zeile mit der Zahl der 429, innerhalb 3 s nach der
/// letzten 429; `gebremst` gleich; stderr dieselben Zeilen und nicht die alte; kein Nachrichtentext im Protokoll.
#[tokio::test]
async fn die_bremse_ist_eine_episode_mit_zahl_und_dauer() {
    let wurzel = ordner();
    let mut instanz = starten(wurzel.path(), &[RUHE]);
    let stderr = instanz.stderr_mitschreiben();
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, &instanz.adresse, "beta", None).await;

    let k = schleife(&beta, &instanz, &token).await;
    let ende = warten_auf(wurzel.path(), Duration::from_secs(10), |z| {
        z.contains(" WARNING schreibkonto ") && z.contains("Episode endet")
    })
    .await;
    let (_, gesundheit) = gemeinsam::gesundheit(&instanz.adresse).await;

    assert!(k > 0);
    let ende = ende.expect("Ende-Zeile");
    println!("Ende der Episode {ende:?} nach der letzten 429 (Grenze 3 s)");
    assert!(ende < Duration::from_secs(3), "{ende:?}");
    let alle = zeilen(wurzel.path(), HEUTE);
    let konto = mit(&alle, "WARNING", "schreibkonto");
    assert_eq!(konto.len(), 2, "{konto:#?}");
    assert!(
        konto[0].contains("Episode beginnt | ")
            && konto[0].contains("kennung=beta@127.0.0.1")
            && konto[0].contains("grenze=kennung"),
        "{konto:#?}"
    );
    assert!(
        konto[1].contains(&format!("Episode endet: {k} in ")),
        "{konto:#?}"
    );
    assert_eq!(gesundheit["gebremst"], k);
    let fehler = stderr_zeilen(&stderr);
    assert_eq!(
        fehler
            .iter()
            .filter(|z| z.contains(" WARNING schreibkonto "))
            .count(),
        2,
        "{fehler:#?}"
    );
    assert!(
        !fehler
            .iter()
            .any(|z| z.contains("gebremst (kennung) — Flut?"))
    );
    assert!(!alle.iter().any(|z| z.contains("Schleife-")));
}

/// „Episode je Kennung“: `beta` und `gamma` laufen in die Bremse → zwei getrennte Episoden; `gebremst` die Summe.
#[tokio::test]
async fn je_kennung_eine_episode() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[RUHE]);
    let beta = Absender::von("127.0.0.1");
    let gamma = Absender::von("127.0.0.1");
    let token_beta = beitreten(&beta, &instanz.adresse, "beta", None).await;
    let token_gamma = beitreten(&gamma, &instanz.adresse, "gamma", None).await;

    let k = schleife(&beta, &instanz, &token_beta).await
        + schleife(&gamma, &instanz, &token_gamma).await;
    let enden = warten_auf(wurzel.path(), Duration::from_secs(10), |_| {
        mit(&zeilen(wurzel.path(), HEUTE), "WARNING", "schreibkonto")
            .iter()
            .filter(|z| z.contains("Episode endet"))
            .count()
            == 2
    })
    .await;
    let (_, gesundheit) = gemeinsam::gesundheit(&instanz.adresse).await;

    assert!(enden.is_some());
    let konto = mit(&zeilen(wurzel.path(), HEUTE), "WARNING", "schreibkonto");
    for wer in ["beta@127.0.0.1", "gamma@127.0.0.1"] {
        let eigene: Vec<_> = konto
            .iter()
            .filter(|z| z.contains(&format!("kennung={wer}")))
            .collect();
        assert_eq!(eigene.len(), 2, "{wer}: {konto:#?}");
    }
    assert_eq!(gesundheit["gebremst"], k);
}

/// „Schreibfehler“ (nicht als root): zweimal 500 → genau eine `ERROR senden … Episode beginnt` mit Channel, Kennung,
/// Datei und Grund; nach der Ruhe `Episode endet: 2`; `errors` ≥ 2; der Text steht nicht im Protokoll.
#[cfg(unix)]
#[tokio::test]
async fn der_schreibfehler_ist_eine_episode_je_datei_und_grund() {
    use std::os::unix::fs::PermissionsExt;
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[RUHE]);
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, &instanz.adresse, "beta", None).await;
    let channel = wurzel.path().join("channels/allgemein");
    std::fs::set_permissions(&channel, std::fs::Permissions::from_mode(0o555)).expect("Rechte");

    let erste = senden(&beta, &instanz.adresse, &token, "allgemein", "Hallo").await;
    let zweite = senden(&beta, &instanz.adresse, &token, "allgemein", "Hallo").await;
    std::fs::set_permissions(&channel, std::fs::Permissions::from_mode(0o755)).expect("Rechte");
    let ende = warten_auf(wurzel.path(), Duration::from_secs(10), |z| {
        z.contains(" ERROR senden ") && z.contains("Episode endet")
    })
    .await;
    let (_, gesundheit) = gemeinsam::gesundheit(&instanz.adresse).await;

    assert_eq!((erste.status, zweite.status), (500, 500));
    assert!(ende.is_some());
    let alle = zeilen(wurzel.path(), HEUTE);
    let senden_zeilen = mit(&alle, "ERROR", "senden");
    let beginn: Vec<_> = senden_zeilen
        .iter()
        .filter(|z| z.contains("Episode beginnt"))
        .collect();
    assert_eq!(beginn.len(), 1, "{senden_zeilen:#?}");
    let datei = format!("datei=channels/allgemein/{HEUTE}.log");
    for teil in [
        "channel=allgemein",
        "kennung=beta@127.0.0.1",
        datei.as_str(),
        "grund=",
    ] {
        assert!(beginn[0].contains(teil), "{teil}: {}", beginn[0]);
    }
    assert!(
        senden_zeilen
            .iter()
            .any(|z| z.contains("Episode endet: 2 ")),
        "{senden_zeilen:#?}"
    );
    assert!(gesundheit["errors"].as_u64().expect("errors") >= 2);
    assert!(!alle.iter().any(|z| z.contains("Hallo")));
    assert!(!tagesdatei(wurzel.path(), "allgemein").exists());
}
