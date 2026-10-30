//! R00050 F0177 B0782 — der Health-Check zählt Direktgespräche nur als `direkt_mib` (ADR-0055 Regel 9, E10, Orakel O2;
//! Matrix H1, H2; Linie L2/L1, echtes Programm): `nachrichten`, `log_bytes`, `defekte_zeilen` und
//! `nachholen.saetze`/`kosten` zählen nur Channels; `direkt_mib` ist die Größe aller Direktgespräche, abgerundet auf
//! ganze MiB. Ohne Sitzung lesbar — darum darf eine Direktnachricht den Health-Check nicht bewegen.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::direkt::{BEN, Runde, runde_mit};
use gemeinsam::senden::{ordner, senden};
use gemeinsam::{gesundheit, starten_mit};
use serde_json::{Value, json};
use std::path::Path;

const LOKAL: [&str; 5] = ["--port", "0", "--host", "127.0.0.1", "--kein-browser"];
const MIB: usize = 1_048_576;

/// Instanz mit der echten Uhr (eine gestellte Uhr rückt die Nachrichtenzeit vor, `zeit_vorlauf_ms`, Restorakel (b)).
async fn runde_echtzeit() -> Runde {
    let wurzel = ordner();
    let instanz = starten_mit(wurzel.path(), &LOKAL, &[("TZ", "Europe/Berlin")]);
    runde_mit(wurzel, instanz).await
}

async fn health(basis: &str) -> Value {
    let (status, json) = gesundheit(basis).await;
    assert_eq!(status, 200);
    json
}

/// H1 (erster Test wörtlich): eine Direktnachricht von 1.000 Bytes, Text, Stufe `information` → der Schnappschuss
/// vor und nach ist gleich: `nachrichten`, `log_bytes`, `defekte_zeilen`, `nachholen.saetze` unverändert,
/// `direkt_mib` 0. Kontrolle: ein Channel-Satz bewegt `nachrichten` und `log_bytes` sehr wohl.
#[tokio::test]
async fn eine_direktnachricht_bewegt_den_health_check_nicht() {
    let r = runde_echtzeit().await;
    let basis = r.instanz.adresse.clone();
    senden(
        &r.cora.absender,
        &basis,
        &r.cora.token,
        "allgemein",
        "im Channel",
    )
    .await;
    let vorher = health(&basis).await;

    let gesendet = r
        .anna
        .senden(&basis, BEN, &json!({"text": "a".repeat(1_000)}))
        .await;
    let nachher = health(&basis).await;

    assert_eq!(gesendet.status, 201, "{}", gesendet.json);
    assert_eq!(vorher["nachrichten"], 1, "{vorher}");
    assert_eq!(nachher["direkt_mib"], 0, "{nachher}");
    assert_eq!(nachher, vorher, "Schnappschuss vor/nach");

    senden(
        &r.cora.absender,
        &basis,
        &r.cora.token,
        "allgemein",
        "noch einer",
    )
    .await;
    let danach = health(&basis).await;
    assert_eq!(danach["nachrichten"], 2, "Kontrolle: Channels zählen");
    assert!(danach["log_bytes"].as_u64() > vorher["log_bytes"].as_u64());
}

/// Das Nachholen von `warten` prüft Sätze; `nachholen.saetze` und `kosten` zählen nur Channel-Sätze — eine
/// Direktnachricht ändert, was derselbe Aufruf von `ben` meldet, nicht (ADR-0055 Regel 9, Inventar).
#[tokio::test]
async fn das_nachholen_zaehlt_keine_direktsaetze() {
    let r = runde_echtzeit().await;
    let basis = r.instanz.adresse.clone();
    let t0 = senden(&r.cora.absender, &basis, &r.cora.token, "allgemein", "t0").await;
    let seit = t0.json["nachricht"]["zeit"]
        .as_str()
        .expect("zeit")
        .replace('+', "%2B");
    senden(&r.cora.absender, &basis, &r.cora.token, "allgemein", "t1").await;
    let nachholen = |wer: &gemeinsam::direkt::Teilnehmer| {
        let url = format!("{basis}api/warten?bis=0&seit={seit}");
        let anfrage = wer
            .absender
            .anfrage(reqwest::Method::GET, url)
            .header("X-LocalchatA-Sitzung", &wer.token);
        gemeinsam::api_antwort(anfrage)
    };
    let geprueft = |gesund: &Value| {
        (
            gesund["nachholen"]["saetze"].as_u64().expect("saetze"),
            gesund["nachholen"]["kosten"].as_u64().expect("kosten"),
        )
    };
    let start = geprueft(&health(&basis).await);
    assert_eq!(nachholen(&r.ben).await.status, 200);
    let ohne_direkt = geprueft(&health(&basis).await);
    let ersten = r.anna.senden(&basis, BEN, &json!({"text": "geheim"})).await;
    assert_eq!(ersten.status, 201, "{}", ersten.json);

    assert_eq!(nachholen(&r.ben).await.status, 200);
    let mit_direkt = geprueft(&health(&basis).await);

    assert!(ohne_direkt.0 > start.0, "Kontrolle: Channel-Sätze zählen");
    assert_eq!(
        (mit_direkt.0 - ohne_direkt.0, mit_direkt.1 - ohne_direkt.1),
        (ohne_direkt.0 - start.0, ohne_direkt.1 - start.1),
        "der zweite Aufruf prüft dieselben Channel-Sätze, die Direktnachricht zählt nicht"
    );
}

/// Legt `direkt/<paar>/2026-10-28.log` mit genau `bytes` Bytes aus gültigen Sätzen an.
fn direkt_datei(ordner: &Path, bytes: usize) {
    const SATZ_MAX: usize = 16_000;
    let kopf = |nummer: usize| {
        format!(
            "@ 2026-10-28T{:02}:{:02}:00.000+01:00 anna@127.0.0.2\n  ",
            9 + nummer / 60,
            nummer % 60
        )
    };
    let mut inhalt = String::new();
    let mut nummer = 0;
    while inhalt.len() < bytes {
        let rest = bytes - inhalt.len();
        let rahmen = kopf(nummer).len() + 2;
        let satz = if rest <= SATZ_MAX + rahmen {
            rest
        } else if rest - (SATZ_MAX + rahmen) <= rahmen {
            SATZ_MAX + rahmen - (rahmen + 1)
        } else {
            SATZ_MAX + rahmen
        };
        inhalt.push_str(&kopf(nummer));
        inhalt.push_str(&"x".repeat(satz - rahmen));
        inhalt.push_str("\n\n");
        nummer += 1;
    }
    assert_eq!(inhalt.len(), bytes);
    let paar = ordner.join("direkt/anna@127.0.0.2+ben@127.0.0.3");
    std::fs::create_dir_all(&paar).expect("Paarordner");
    std::fs::write(paar.join("2026-10-28.log"), inhalt).expect("Tagesdatei");
}

/// H2: 1,1 MiB in Direktgesprächen → `direkt_mib` 1, `nachrichten` und `log_bytes` bleiben 0; die Grenze liegt bei
/// genau 1 MiB (1.048.575 Bytes → 0, 1.048.576 → 1, 2 MiB − 1 → 1, 2 MiB → 2).
#[tokio::test]
async fn direkt_mib_ist_die_groesse_der_direktgespraeche_abgerundet() {
    for (bytes, erwartet) in [
        (1_153_434, 1),
        (MIB - 1, 0),
        (MIB, 1),
        (2 * MIB - 1, 1),
        (2 * MIB, 2),
    ] {
        let wurzel = ordner();
        direkt_datei(wurzel.path(), bytes);
        let instanz = starten_mit(
            wurzel.path(),
            &LOKAL,
            &[
                ("TZ", "Europe/Berlin"),
                ("LOCALCHATA_JETZT", "2026-10-28T12:00:00.000+01:00"),
            ],
        );

        let gesund = health(&instanz.adresse).await;

        assert_eq!(gesund["direkt_mib"], erwartet, "{bytes} Bytes: {gesund}");
        assert_eq!(gesund["nachrichten"], 0, "{bytes} Bytes: {gesund}");
        assert_eq!(gesund["log_bytes"], 0, "{bytes} Bytes: {gesund}");
    }
}
