//! R00019 F0064 „Debug-Abweisungen im Bereich frist“ (US-4, US-8) — über das echte Programm auf Stufe debug: jede
//! abgewiesene Anfrage an `PUT /api/channels/{name}/frist` steht im Bereich `frist` mit `channel=` aus dem Pfad,
//! `kennung=` bzw. ohne Sitzung `herkunft=` und dem Regelschlüssel in `grund`.

mod gemeinsam;

use gemeinsam::frist::{MITTAG, beitreten, datei_anlegen, frist_setzen, tag_anlegen};
use gemeinsam::protokoll::{wert, zeilen};
use gemeinsam::{Absender, starten_mit};
use serde_json::json;

const ROUTE: &str = "route=\"PUT /api/channels/{name}/frist\"";

/// Die DEBUG-Zeilen mit `status=4…` in der Reihenfolge der Datei.
fn abweisungen(ordner: &std::path::Path) -> Vec<String> {
    zeilen(ordner, "2026-10-28")
        .into_iter()
        .filter(|zeile| zeile.contains(" DEBUG "))
        .filter(|zeile| wert(zeile, "status").is_some_and(|status| status.starts_with('4')))
        .collect()
}

#[tokio::test]
async fn abgewiesene_frist_anfragen_stehen_im_bereich_frist_mit_channel_und_kennung() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let ordner = wurzel.path();
    datei_anlegen(
        ordner,
        "channels/allgemein/einstellungen.toml",
        b"frist_tage = 30\n",
    );
    for (tag, anzahl) in [
        ("2026-10-20", 2),
        ("2026-10-21", 3),
        ("2026-10-22", 1),
        ("2026-10-28", 1),
    ] {
        tag_anlegen(ordner, "allgemein", tag, anzahl);
    }
    let instanz = starten_mit(
        ordner,
        &["--port", "0", "--host", "127.0.0.1", "--kein-browser"],
        &[
            ("LOCALCHATA_JETZT", MITTAG),
            ("TZ", "Europe/Berlin"),
            ("LOCALCHATA_PROTOKOLL_STUFE", "debug"),
        ],
    );
    let eins = Absender::von("127.0.0.1");
    let beta = beitreten(&eins, &instanz, "beta").await;

    let null = frist_setzen(&eins, &instanz, &beta, "allgemein", json!({ "tage": 0 })).await;
    let sieben = frist_setzen(&eins, &instanz, &beta, "allgemein", json!({ "tage": 7 })).await;
    let ohne_sitzung = eins
        .anfrage(
            reqwest::Method::PUT,
            format!("{}api/channels/allgemein/frist", instanz.adresse),
        )
        .json(&json!({ "tage": 30 }))
        .send()
        .await
        .expect("401")
        .status()
        .as_u16();

    assert_eq!([null.status, sieben.status, ohne_sitzung], [422, 409, 401]);
    let zeilen = abweisungen(ordner);
    let frist: Vec<&String> = zeilen
        .iter()
        .filter(|zeile| zeile.contains(" DEBUG frist "))
        .collect();
    assert_eq!(frist.len(), 3, "{zeilen:#?}");
    for (zeile, status) in frist.iter().zip(["422", "409", "401"]) {
        assert_eq!(wert(zeile, "status").as_deref(), Some(status), "{zeile}");
        assert_eq!(
            wert(zeile, "channel").as_deref(),
            Some("allgemein"),
            "{zeile}"
        );
        assert!(zeile.contains(ROUTE), "{zeile}");
    }
    assert_eq!(wert(frist[0], "kennung").as_deref(), Some("beta@127.0.0.1"));
    assert_eq!(wert(frist[0], "grund").as_deref(), Some("frist_ungueltig"));
    assert_eq!(wert(frist[1], "kennung").as_deref(), Some("beta@127.0.0.1"));
    assert_eq!(wert(frist[1], "grund").as_deref(), Some("bestaetigen"));
    assert_eq!(wert(frist[2], "herkunft").as_deref(), Some("127.0.0.1"));
    assert_eq!(wert(frist[2], "kennung"), None, "{}", frist[2]);
    assert!(
        !zeilen
            .iter()
            .any(|zeile| zeile.contains(" DEBUG channels ") && zeile.contains(ROUTE)),
        "{zeilen:#?}"
    );
}
