//! F0042 Nachricht live im Strom (R00011, US-7 bis US-9): das echte Programm, `alpha` liest den Strom.

mod gemeinsam;

use gemeinsam::Absender;
use gemeinsam::senden::{beitreten, ordner, roh_senden, senden, starten, tagesinhalt};
use gemeinsam::strom::{Empfangen, Strom};
use serde_json::json;
use std::time::Duration;
use tokio::time::Instant;

fn ist_nachricht(e: &Empfangen) -> bool {
    e.data["art"] == "nachricht"
}

/// Z10 aus R00011: Grenze der Zustellung an einen Agenten.
const Z10: Duration = Duration::from_secs(2);

/// US-7 „Ereignis“: dieselbe Nachricht wie in der 201, innerhalb Z10 ab Erhalt der 201.
#[tokio::test]
async fn us7_die_nachricht_kommt_im_strom_an() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[]);
    let alpha = Absender::von("127.0.0.1");
    let beta = Absender::von("127.0.0.1");
    let token_alpha = beitreten(&alpha, &instanz.adresse, "alpha", None).await;
    let token_beta = beitreten(&beta, &instanz.adresse, "beta", None).await;
    let mut strom = Strom::oeffnen(&alpha, &instanz.adresse, &token_alpha).await;

    let antwort = senden(&beta, &instanz.adresse, &token_beta, "allgemein", "Hallo").await;
    let erhalten = Instant::now();
    let ereignis = strom
        .bis(Duration::from_secs(10), ist_nachricht)
        .await
        .expect("Ereignis");

    assert_eq!(antwort.status, 201);
    assert_eq!(
        ereignis.data,
        json!({"art": "nachricht", "channel": "allgemein", "nachricht": {
            "zeit": antwort.json["nachricht"]["zeit"], "von": "beta@127.0.0.1", "art": "agent", "text": "Hallo", "anhaenge": [], "zustand": null, "geaendert": null}})
    );
    let dauer = ereignis.ankunft.saturating_duration_since(erhalten);
    println!("Z10 gemessen: {dauer:?} (Grenze {Z10:?})");
    assert!(dauer < Z10, "{dauer:?}");
}

/// US-8 „Kein Ereignis bei Fehlern“: nach 422, 404 und 413 ist das nächste Ereignis `zwei`; nach einem 500 ist das
/// nächste `vier`.
#[tokio::test]
async fn us8_abgewiesenes_und_ungespeichertes_erzeugt_nichts_im_strom() {
    use std::os::unix::fs::PermissionsExt;
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[]);
    let alpha = Absender::von("127.0.0.1");
    let beta = Absender::von("127.0.0.1");
    let token_alpha = beitreten(&alpha, &instanz.adresse, "alpha", None).await;
    let token_beta = beitreten(&beta, &instanz.adresse, "beta", None).await;
    let mut strom = Strom::oeffnen(&alpha, &instanz.adresse, &token_alpha).await;
    let basis = &instanz.adresse;
    let mut gross = b"{".to_vec();
    gross.extend(std::iter::repeat_n(b'a', 131_072));

    let leer = senden(&beta, basis, &token_beta, "allgemein", " ").await;
    let fehlt = senden(&beta, basis, &token_beta, "fehlt", "x").await;
    let zu_gross = roh_senden(
        &beta,
        basis,
        Some(&token_beta),
        "allgemein",
        "application/json",
        gross,
    )
    .await;
    let zwei = senden(&beta, basis, &token_beta, "allgemein", "zwei").await;
    let erstes = strom
        .bis(Duration::from_secs(10), ist_nachricht)
        .await
        .expect("Ereignis");
    let channel = wurzel.path().join("channels/allgemein");
    let datei = gemeinsam::senden::tagesdatei(wurzel.path(), "allgemein");
    let rechte = |pfad: &std::path::Path, modus| {
        std::fs::set_permissions(pfad, std::fs::Permissions::from_mode(modus)).expect("Rechte");
    };
    rechte(&datei, 0o444);
    rechte(&channel, 0o555);
    let drei = senden(&beta, basis, &token_beta, "allgemein", "drei").await;
    rechte(&channel, 0o755);
    rechte(&datei, 0o644);
    let vier = senden(&beta, basis, &token_beta, "allgemein", "vier").await;
    let zweites = strom
        .bis(Duration::from_secs(10), |e| {
            ist_nachricht(e) && e.data["nachricht"]["text"] != "zwei"
        })
        .await
        .expect("Ereignis");

    assert_eq!(
        [
            leer.status,
            fehlt.status,
            zu_gross.status,
            zwei.status,
            drei.status,
            vier.status
        ],
        [422, 404, 413, 201, 500, 201]
    );
    assert_eq!(erstes.data["nachricht"]["text"], "zwei");
    assert_eq!(zweites.data["nachricht"]["text"], "vier");
}

/// US-9 „Ordnung über Channels“: 40 Agenten (je 4 von 127.0.0.2 bis 127.0.0.11) senden gleichzeitig je 25
/// Nachrichten abwechselnd an `allgemein` und `technik` → alle 201; 1 000 Ereignisse mit streng steigenden Zeiten;
/// je Channel dieselbe Folge wie die Sätze im Log.
#[tokio::test]
async fn us9_eine_folge_ueber_alle_channels() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[]);
    let alpha = Absender::von("127.0.0.1");
    let token_alpha = beitreten(&alpha, &instanz.adresse, "alpha", None).await;
    let mut agenten = Vec::new();
    for nummer in 0..40 {
        let herkunft = format!("127.0.0.{}", 2 + nummer / 4);
        let absender = Absender::von(&herkunft);
        let token = beitreten(&absender, &instanz.adresse, &format!("a{nummer}"), None).await;
        agenten.push((absender, token));
    }
    let mut strom = Strom::oeffnen(&alpha, &instanz.adresse, &token_alpha).await;

    let laeufe: Vec<_> = agenten
        .into_iter()
        .enumerate()
        .map(|(nummer, (absender, token))| {
            let basis = instanz.adresse.clone();
            tokio::spawn(async move {
                let mut status = Vec::new();
                for folge in 0..25 {
                    let channel = if folge % 2 == 0 {
                        "allgemein"
                    } else {
                        "technik"
                    };
                    let antwort = senden(
                        &absender,
                        &basis,
                        &token,
                        channel,
                        &format!("{nummer}-{folge}"),
                    )
                    .await;
                    status.push(antwort.status);
                }
                status
            })
        })
        .collect();
    let mut alle = Vec::new();
    for lauf in laeufe {
        alle.extend(lauf.await.expect("Lauf"));
    }
    let ende = Instant::now() + Duration::from_secs(30);
    while strom.ereignisse.iter().filter(|e| ist_nachricht(e)).count() < 1_000
        && Instant::now() < ende
    {
        strom.mitlesen(Duration::from_millis(200)).await;
    }

    assert_eq!(alle.len(), 1_000);
    assert!(alle.iter().all(|status| *status == 201), "{alle:?}");
    let nachrichten: Vec<_> = strom
        .ereignisse
        .iter()
        .filter(|e| ist_nachricht(e))
        .collect();
    assert_eq!(nachrichten.len(), 1_000);
    let zeiten: Vec<chrono::DateTime<chrono::FixedOffset>> = nachrichten
        .iter()
        .map(|e| {
            chrono::DateTime::parse_from_rfc3339(
                e.data["nachricht"]["zeit"].as_str().expect("Zeit"),
            )
            .expect("Zeit")
        })
        .collect();
    assert!(
        zeiten.windows(2).all(|paar| paar[0] < paar[1]),
        "streng steigend"
    );
    for channel in ["allgemein", "technik"] {
        let im_strom: Vec<String> = nachrichten
            .iter()
            .filter(|e| e.data["channel"] == channel)
            .map(|e| {
                e.data["nachricht"]["zeit"]
                    .as_str()
                    .expect("Zeit")
                    .to_string()
            })
            .collect();
        let im_log: Vec<String> = String::from_utf8(tagesinhalt(wurzel.path(), channel))
            .expect("UTF-8")
            .lines()
            .filter_map(|zeile| zeile.strip_prefix("@ "))
            .map(|kopf| kopf.split(' ').next().expect("Zeit").to_string())
            .collect();
        assert_eq!(im_strom, im_log, "{channel}");
    }
}

/// US-10 „Vorlauf im echten Programm“: Uhr 12:00, im Log ein Satz um 14:03:30 → zwei Nachrichten um .001 und
/// .002; genau eine Zeile „… 2 h 03 min voraus“ (`WARNING zeit` mit `dauer_ms`); die Gesundheit nennt
/// `zeit_vorlauf_ms` zwischen 7 410 002 − (ms seit der Startzeile + 1 000) und 7 410 002. Ohne Satz aus der
/// Zukunft 0.
#[tokio::test]
async fn us10_eine_falsch_gestellte_uhr_wird_sichtbar() {
    let wurzel = ordner();
    std::fs::write(
        gemeinsam::senden::tagesdatei(wurzel.path(), "allgemein"),
        "@ 2026-10-28T14:03:30.000+01:00 claude@10.0.0.1\n  aus der Zukunft\n\n",
    )
    .expect("Tagesdatei");
    let mut instanz = starten(wurzel.path(), &[]);
    let gestartet = Instant::now();
    let stderr = instanz.stderr_mitschreiben();
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, &instanz.adresse, "beta", None).await;

    let erste = senden(&beta, &instanz.adresse, &token, "allgemein", "eins").await;
    let zweite = senden(&beta, &instanz.adresse, &token, "allgemein", "zwei").await;
    let (_, gesundheit) = gemeinsam::gesundheit(&instanz.adresse).await;
    let seit_start = u64::try_from(gestartet.elapsed().as_millis()).expect("ms");
    tokio::time::sleep(Duration::from_millis(300)).await;

    assert_eq!(
        erste.json["nachricht"]["zeit"],
        "2026-10-28T14:03:30.001+01:00"
    );
    assert_eq!(
        zweite.json["nachricht"]["zeit"],
        "2026-10-28T14:03:30.002+01:00"
    );
    let vorlauf = gesundheit["zeit_vorlauf_ms"]
        .as_u64()
        .expect("zeit_vorlauf_ms");
    assert!(
        vorlauf <= 7_410_002 && vorlauf >= 7_410_002 - (seit_start + 1_000),
        "{vorlauf} nach {seit_start} ms"
    );
    let zeilen: Vec<String> = stderr
        .lock()
        .expect("stderr")
        .iter()
        .filter(|zeile| zeile.contains("Nachrichtenzeit läuft der Systemuhr 2 h 03 min voraus"))
        .cloned()
        .collect();
    assert_eq!(zeilen.len(), 1, "{zeilen:#?}");
    assert!(
        zeilen[0].contains(" WARNING zeit ") && zeilen[0].contains("dauer_ms="),
        "{zeilen:#?}"
    );

    let protokoll = gemeinsam::protokoll::mit(
        &gemeinsam::protokoll::zeilen(wurzel.path(), "2026-10-28"),
        "WARNING",
        "zeit",
    );
    assert_eq!(protokoll.len(), 1, "R00016 „Vorlauf“: {protokoll:#?}");
    assert!(protokoll[0].contains("2 h 03 min voraus") && protokoll[0].contains("dauer_ms="));
    assert!(
        protokoll[0].contains(", bis die Uhr aufholt — Episode beginnt | dauer_ms="),
        "R00020 „Vorlauf ohne Punkt“: {protokoll:#?}"
    );

    let ohne = ordner();
    let ohne_instanz = starten(ohne.path(), &[]);
    let (_, ohne_gesundheit) = gemeinsam::gesundheit(&ohne_instanz.adresse).await;
    assert_eq!(ohne_gesundheit["zeit_vorlauf_ms"], 0);
}
