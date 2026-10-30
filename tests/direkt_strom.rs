//! R00050 F0173 B0772 — der Ereignisstrom im Direktgespräch über das echte Programm (ADR-0055 Regel 6; Matrix K2, S1,
//! S2): eine Direktnachricht erscheint als `{art, channel: null, direkt: <Gegenüber>}` nur bei den zwei Beteiligten
//! — auch im zweiten Tab des Absenders —, binnen 1 s; ein Dritter bekommt in keinem Strom etwas davon, auch nicht
//! nach dem Wiederverbinden. Dasselbe für `aenderung` und `anhaenge`. Das erzwungene `Lagged` steht als Unit-Test
//! in `src/http/ereignisse.rs`.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::anhaenge::{ablegen, starten as starten_anhaenge};
use gemeinsam::direkt::{ANNA, BEN, Runde, Teilnehmer, runde_mit};
use gemeinsam::senden::{JETZT, ordner};
use gemeinsam::strom::{Empfangen, Strom};
use serde_json::{Value, json};
use std::time::Duration;
use tokio::time::Instant;

/// Erscheinen binnen 1 s nach der Antwort (Matrix K2).
const ERSCHEINEN: Duration = Duration::from_secs(1);
/// So lange wird auf etwas gewartet, das nicht kommen darf (Matrix S1: „nichts in 3 s“).
const STILLE: Duration = Duration::from_secs(3);

/// Die Arten, die ein Direktgespräch erzeugt und die ein Dritter nie sehen darf.
const GESPRAECHSARTEN: [&str; 4] = ["nachricht", "aenderung", "anhaenge", "aktivitaet"];

async fn strom_offen(wer: &Teilnehmer, basis: &str) -> Strom {
    let mut strom = Strom::oeffnen(&wer.absender, basis, &wer.token).await;
    strom
        .bis(Duration::from_secs(5), |e| e.data["art"] == "channels")
        .await
        .expect("Stand der Channels");
    strom
}

fn von_art(art: &'static str) -> impl Fn(&Empfangen) -> bool {
    move |e| e.data["art"] == art
}

/// Alles, was ein Strom aus einem Gespräch lieferte (egal ob Channel oder Direkt).
fn gespraechsereignisse(strom: &Strom) -> Vec<&Empfangen> {
    strom
        .ereignisse
        .iter()
        .filter(|e| GESPRAECHSARTEN.contains(&e.data["art"].as_str().unwrap_or_default()))
        .collect()
}

fn zeit_der_antwort(antwort: &gemeinsam::ApiAntwort) -> String {
    assert_eq!(antwort.status, 201, "{}", antwort.json);
    antwort.json["nachricht"]["zeit"]
        .as_str()
        .expect("zeit")
        .to_string()
}

/// Alle Ströme der Runde: `ben`, `anna` im zweiten Tab, `cora` mit zwei Tabs.
struct Stroeme {
    ben: Strom,
    anna_zweiter_tab: Strom,
    cora_eins: Strom,
    cora_zwei: Strom,
}

async fn stroeme_oeffnen(r: &Runde) -> Stroeme {
    let basis = r.instanz.adresse.as_str();
    Stroeme {
        ben: strom_offen(&r.ben, basis).await,
        anna_zweiter_tab: strom_offen(&r.anna, basis).await,
        cora_eins: strom_offen(&r.cora, basis).await,
        cora_zwei: strom_offen(&r.cora, basis).await,
    }
}

/// Wartet auf das Ereignis `art` und misst die Ankunft gegen `seit`.
async fn erwarten(strom: &mut Strom, art: &'static str, seit: Instant) -> Value {
    let ereignis = strom
        .bis(Duration::from_secs(5), von_art(art))
        .await
        .unwrap_or_else(|| panic!("kein Ereignis {art}"));
    let dauer = ereignis.ankunft.saturating_duration_since(seit);
    assert!(dauer <= ERSCHEINEN, "{art} nach {dauer:?}");
    ereignis.data
}

/// K2 und S2 (erster Test, wörtlich): `anna` sendet `ben` → Strom `ben` `{art: nachricht, channel: null, direkt:
/// anna@…}` binnen 1 s, Strom `anna` (zweiter Tab) `direkt: ben@…`; Strom `cora` (zwei Tabs, danach ein frisch
/// verbundener) nichts in 3 s. Dasselbe für `aenderung` (Bearbeiten und Löschen).
#[tokio::test]
async fn nachricht_und_aenderung_gehen_nur_an_die_zwei_beteiligten() {
    let wurzel = ordner();
    let instanz = starten_anhaenge(wurzel.path(), &[], &[("LOCALCHATA_JETZT", JETZT)]);
    let r = runde_mit(wurzel, instanz).await;
    let basis = r.instanz.adresse.clone();
    let mut s = stroeme_oeffnen(&r).await;

    let gesendet = r.anna.senden(&basis, BEN, &json!({"text": "k2"})).await;
    let nach_der_antwort = Instant::now();
    let zeit = zeit_der_antwort(&gesendet);
    let bei_ben = erwarten(&mut s.ben, "nachricht", nach_der_antwort).await;
    let bei_anna = erwarten(&mut s.anna_zweiter_tab, "nachricht", nach_der_antwort).await;

    assert_eq!(bei_ben["channel"], json!(null), "{bei_ben}");
    assert_eq!(bei_ben["direkt"], ANNA, "{bei_ben}");
    assert_eq!(bei_ben["nachricht"]["text"], "k2");
    assert_eq!(bei_ben["nachricht"]["von"], ANNA);
    assert_eq!(bei_anna["channel"], json!(null), "{bei_anna}");
    assert_eq!(
        bei_anna["direkt"], BEN,
        "S2: das Gegenüber, nicht der Absender"
    );
    assert_eq!(bei_anna["nachricht"]["zeit"], bei_ben["nachricht"]["zeit"]);

    let bearbeitet = r
        .anna
        .post(
            &basis,
            BEN,
            "nachrichten/bearbeiten",
            &json!({"zeit": zeit, "text": "k2!"}),
        )
        .await;
    let nach_bearbeiten = Instant::now();
    assert_eq!(bearbeitet.status, 200, "{}", bearbeitet.json);
    let aenderung_ben = erwarten(&mut s.ben, "aenderung", nach_bearbeiten).await;
    let aenderung_anna = erwarten(&mut s.anna_zweiter_tab, "aenderung", nach_bearbeiten).await;
    assert_eq!(aenderung_ben["channel"], json!(null), "{aenderung_ben}");
    assert_eq!(aenderung_ben["direkt"], ANNA);
    assert_eq!(aenderung_ben["nachricht"]["text"], "k2!");
    assert_eq!(aenderung_anna["direkt"], BEN);

    let weg = r.ben.senden(&basis, ANNA, &json!({"text": "weg"})).await;
    let geloescht = r
        .ben
        .post(
            &basis,
            ANNA,
            "nachrichten/loeschen",
            &json!({"zeit": zeit_der_antwort(&weg)}),
        )
        .await;
    assert_eq!(geloescht.status, 200, "{}", geloescht.json);
    let aenderungen_bei_anna = s
        .anna_zweiter_tab
        .bis(Duration::from_secs(5), |e| {
            e.data["art"] == "aenderung" && e.data["nachricht"]["zustand"] == "geloescht"
        })
        .await
        .expect("Löschen bei anna");
    assert_eq!(aenderungen_bei_anna.data["direkt"], BEN);

    // cora: zwei Tabs, danach ein frisch verbundener — in 3 s nichts aus dem Gespräch.
    let mut cora_neu = strom_offen(&r.cora, &basis).await;
    let (eins, zwei, neu) = tokio::join!(
        s.cora_eins.mitlesen(STILLE),
        s.cora_zwei.mitlesen(STILLE),
        cora_neu.mitlesen(STILLE)
    );
    assert!(!eins && !zwei && !neu, "die Ströme laufen weiter");
    for (name, strom) in [
        ("cora Tab 1", &s.cora_eins),
        ("cora Tab 2", &s.cora_zwei),
        ("cora nach dem Wiederverbinden", &cora_neu),
    ] {
        let fremd = gespraechsereignisse(strom);
        assert!(fremd.is_empty(), "{name} bekam {fremd:?}");
    }
}

/// Dasselbe für `anhaenge`: `anna` löscht eine Direktnachricht mit Anhang → `{art: anhaenge, channel: null, direkt:
/// <Gegenüber>, geloescht: [{id, zeit}]}` bei `ben` und im zweiten Tab von `anna`, bei `cora` nichts.
#[tokio::test]
async fn anhaenge_gehen_nur_an_die_zwei_beteiligten() {
    let wurzel = ordner();
    let instanz = starten_anhaenge(wurzel.path(), &[], &[("LOCALCHATA_JETZT", JETZT)]);
    let r = runde_mit(wurzel, instanz).await;
    let basis = r.instanz.adresse.clone();
    let id = ablegen(
        &r.anna.absender,
        &basis,
        &r.anna.token,
        "notiz.txt",
        b"anhang".to_vec(),
    )
    .await;
    let gesendet = r
        .anna
        .senden(&basis, BEN, &json!({"text": "mit", "anhaenge": [id]}))
        .await;
    let zeit = zeit_der_antwort(&gesendet);
    let mut s = stroeme_oeffnen(&r).await;

    let geloescht = r
        .anna
        .post(&basis, BEN, "nachrichten/loeschen", &json!({"zeit": zeit}))
        .await;
    let nach_der_antwort = Instant::now();
    assert_eq!(geloescht.status, 200, "{}", geloescht.json);
    let bei_ben = erwarten(&mut s.ben, "anhaenge", nach_der_antwort).await;
    let bei_anna = erwarten(&mut s.anna_zweiter_tab, "anhaenge", nach_der_antwort).await;

    assert_eq!(bei_ben["channel"], json!(null), "{bei_ben}");
    assert_eq!(bei_ben["direkt"], ANNA);
    assert_eq!(bei_ben["geloescht"][0]["id"], json!(id));
    assert_eq!(bei_anna["direkt"], BEN);
    let (eins, zwei) = tokio::join!(s.cora_eins.mitlesen(STILLE), s.cora_zwei.mitlesen(STILLE));
    assert!(!eins && !zwei);
    for strom in [&s.cora_eins, &s.cora_zwei] {
        assert!(
            gespraechsereignisse(strom).is_empty(),
            "{:?}",
            strom.ereignisse
        );
    }
}
