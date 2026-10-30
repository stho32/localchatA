//! R00049 F0168 B0749 — eine Direktnachricht weckt den Wartenden (W7c, Matrix K2 und G3; ADR-0054 Regel 2 und 4,
//! ADR-0055 Regel 6 bis 8; Linie L2, echtes Programm): binnen 1 s `anlass: ["direkt"]`, `direkt` ist das Gegenüber,
//! `channel` ist `null`; die eigene Direktnachricht weckt nicht; ein Dritter erfährt nichts und `ben` verliert nichts,
//! auch wenn `cora` mit demselben `seit` vorbeischaut (Verlustfreiheit). Die Hälften der sichtbaren Marke ohne Filter
//! und mit `erwaehnung` (G1, G2) stehen in `warten_marke.rs`.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::anhaenge::{ablegen, starten as starten_anhaenge};
use gemeinsam::direkt::{ANNA, BEN, Runde, Teilnehmer, runde, runde_mit};
use gemeinsam::senden::{JETZT, ordner, senden};
use gemeinsam::{Absender, ApiAntwort};
use serde_json::{Value, json};
use std::time::{Duration, Instant};

/// `GET /api/warten?<anfrage>` als `wer`.
async fn warten(wer: &Teilnehmer, basis: &str, anfrage: &str) -> ApiAntwort {
    gemeinsam::api_antwort(
        wer.absender
            .anfrage(reqwest::Method::GET, format!("{basis}api/warten?{anfrage}"))
            .header("X-LocalchatA-Sitzung", &wer.token),
    )
    .await
}

fn zeit_der_antwort(antwort: &ApiAntwort) -> String {
    assert_eq!(antwort.status, 201, "{}", antwort.json);
    antwort.json["nachricht"]["zeit"]
        .as_str()
        .expect("zeit")
        .to_string()
}

/// Ein neuer Teilnehmer-Griff mit eigener Verbindung für einen Aufruf in einer Task.
fn griff(wer: &Teilnehmer, adresse: &str) -> Teilnehmer {
    Teilnehmer {
        absender: Absender::von(adresse),
        token: wer.token.clone(),
    }
}

fn als_parameter(zeit: &str) -> String {
    zeit.replace('+', "%2B")
}

/// `cora` sendet `vorher` in `allgemein`; liefert dessen Zeit (der Zeiger `t0` der Wartenden).
async fn t0(r: &Runde, basis: &str) -> String {
    zeit_der_antwort(
        &senden(
            &r.cora.absender,
            basis,
            &r.cora.token,
            "allgemein",
            "vorher",
        )
        .await,
    )
}

/// K2 und W7c (erster Test, wörtlich): `ben` wartet auf `direkt`; eine Channel-Nachricht weckt nicht; `anna` sendet
/// `ben` eine Direktnachricht → binnen 1 s nach der 201 `anlass: ["direkt"]`, `direkt: "anna@127.0.0.2"`, `channel:
/// null`, `ende: "treffer"`.
#[tokio::test]
async fn eine_direktnachricht_weckt_den_empfaenger_binnen_einer_sekunde() {
    let r = runde(&[]).await;
    let basis = r.instanz.adresse.clone();
    let t0 = t0(&r, &basis).await;
    let aufruf = tokio::spawn({
        let (basis, ben) = (basis.clone(), griff(&r.ben, "127.0.0.3"));
        let anfrage = format!("bis=10&direkt&seit={}", als_parameter(&t0));
        async move { warten(&ben, &basis, &anfrage).await }
    });
    tokio::time::sleep(Duration::from_millis(300)).await;
    senden(
        &r.cora.absender,
        &basis,
        &r.cora.token,
        "allgemein",
        "im Channel",
    )
    .await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(!aufruf.is_finished(), "eine Channel-Nachricht weckt nicht");

    let gesendet = r
        .anna
        .senden(&basis, BEN, &json!({"text": "nur für dich"}))
        .await;
    let ab = Instant::now();
    assert_eq!(gesendet.status, 201, "{}", gesendet.json);
    let antwort = aufruf.await.expect("Aufruf");
    let dauer = ab.elapsed();

    assert_eq!(antwort.status, 200, "{}", antwort.json);
    assert!(dauer < Duration::from_secs(1), "geweckt nach {dauer:?}");
    assert_eq!(antwort.json["ende"], "treffer");
    let treffer = antwort.json["nachrichten"].as_array().expect("Liste");
    assert_eq!(treffer.len(), 1, "{}", antwort.json);
    assert_eq!(treffer[0]["anlass"], json!(["direkt"]));
    assert_eq!(treffer[0]["direkt"], json!(ANNA));
    assert_eq!(treffer[0]["channel"], Value::Null);
    assert_eq!(treffer[0]["nachricht"]["text"], "nur für dich");
    assert_eq!(treffer[0]["nachricht"]["von"], json!(ANNA));
}

/// Die eigene Direktnachricht weckt den Absender nicht — mit dem Filter `direkt` und ohne Filter: 3 s Frist leer.
#[tokio::test]
async fn die_eigene_direktnachricht_weckt_den_absender_nicht() {
    let r = runde(&[]).await;
    let basis = r.instanz.adresse.clone();
    let t0 = t0(&r, &basis).await;
    let aufrufe: Vec<_> = ["bis=2&direkt", "bis=2"]
        .into_iter()
        .map(|filter| {
            let (basis, anna) = (basis.clone(), griff(&r.anna, "127.0.0.2"));
            let anfrage = format!("{filter}&seit={}", als_parameter(&t0));
            tokio::spawn(async move { warten(&anna, &basis, &anfrage).await })
        })
        .collect();
    tokio::time::sleep(Duration::from_millis(300)).await;

    let gesendet = r
        .anna
        .senden(&basis, BEN, &json!({"text": "ich an dich"}))
        .await;
    assert_eq!(gesendet.status, 201, "{}", gesendet.json);

    for aufruf in aufrufe {
        let antwort = aufruf.await.expect("Aufruf");
        assert_eq!(antwort.status, 200, "{}", antwort.json);
        assert_eq!(antwort.json["nachrichten"], json!([]), "{}", antwort.json);
        assert_eq!(antwort.json["ende"], "frist");
    }
}

/// Nachholen (B0748, US-24): was ankam, während `ben` nicht wartete, kommt mit dem alten `seit` — ohne Filter mit dem
/// Channel zusammen in Zeitfolge (Channel: `anlass: []`, `direkt: null`; Direkt: `["direkt"]`, `channel: null`), mit
/// `direkt` nur das Direktgespräch, mit `channel` nur der Channel.
#[tokio::test]
async fn eine_verpasste_direktnachricht_kommt_mit_dem_alten_seit() {
    let r = runde(&[]).await;
    let basis = r.instanz.adresse.clone();
    let t0 = t0(&r, &basis).await;
    let kanal = senden(
        &r.cora.absender,
        &basis,
        &r.cora.token,
        "allgemein",
        "im Channel",
    )
    .await;
    let tc = zeit_der_antwort(&kanal);
    let direkt = r
        .anna
        .senden(&basis, BEN, &json!({"text": "verpasst"}))
        .await;
    let td = zeit_der_antwort(&direkt);
    let seit = als_parameter(&t0);

    let ohne = warten(&r.ben, &basis, &format!("bis=0&seit={seit}")).await;
    let nur_direkt = warten(&r.ben, &basis, &format!("bis=0&direkt&seit={seit}")).await;
    let nur_channel = warten(
        &r.ben,
        &basis,
        &format!("bis=0&channel=allgemein&seit={seit}"),
    )
    .await;

    let zeiten = |antwort: &ApiAntwort| -> Vec<Value> {
        antwort.json["nachrichten"]
            .as_array()
            .expect("Liste")
            .iter()
            .map(|eintrag| eintrag["nachricht"]["zeit"].clone())
            .collect()
    };
    assert_eq!(zeiten(&ohne), vec![json!(tc), json!(td)], "{}", ohne.json);
    let (erste, zweite) = (&ohne.json["nachrichten"][0], &ohne.json["nachrichten"][1]);
    assert_eq!(
        (&erste["channel"], &erste["direkt"], &erste["anlass"]),
        (&json!("allgemein"), &Value::Null, &json!([]))
    );
    assert_eq!(
        (&zweite["channel"], &zweite["direkt"], &zweite["anlass"]),
        (&Value::Null, &json!(ANNA), &json!(["direkt"]))
    );
    assert_eq!(ohne.json["seit"], json!(td));
    assert_eq!(zeiten(&nur_direkt), vec![json!(td)], "{}", nur_direkt.json);
    assert_eq!(
        zeiten(&nur_channel),
        vec![json!(tc)],
        "{}",
        nur_channel.json
    );
}

/// G3 (wörtlich) mit G1: `cora` ruft `warten?bis=0` → `s0`; `anna` schreibt `ben`; `cora` ruft mit `seit=s0` ohne
/// Filter, mit `direkt`, mit `erwaehnung` (Text `@cora`) → kein Treffer, `seit` bleibt `s0`; danach erhält `ben` mit
/// `seit=s0` die Direktnachricht — das Vorbeischauen von `cora` hat nichts verbraucht (Verlustfreiheit). Mutanten: das
/// unsichtbare Bus-Ereignis rückt den Stand vor; Nachholen über alle Direktgespräche.
#[tokio::test]
async fn g3_ben_erhaelt_mit_dem_seit_von_cora_die_direktnachricht() {
    let r = runde(&[]).await;
    let basis = r.instanz.adresse.clone();
    t0(&r, &basis).await;
    let erster = warten(&r.cora, &basis, "bis=0").await;
    let s0 = erster.json["seit"].as_str().expect("s0").to_string();
    let seit = als_parameter(&s0);
    let direkt = r
        .anna
        .senden(&basis, BEN, &json!({"text": "geheim für @cora"}))
        .await;
    let td = zeit_der_antwort(&direkt);

    for filter in ["", "&direkt", "&erwaehnung", "&direkt&erwaehnung"] {
        let cora = warten(&r.cora, &basis, &format!("bis=0&seit={seit}{filter}")).await;
        assert_eq!(
            cora.json["nachrichten"],
            json!([]),
            "{filter}: {}",
            cora.json
        );
        assert_eq!(cora.json["seit"], json!(s0), "{filter}: {}", cora.json);
    }
    let ben = warten(&r.ben, &basis, &format!("bis=0&seit={seit}")).await;

    assert_eq!(
        ben.json["nachrichten"].as_array().map(Vec::len),
        Some(1),
        "{}",
        ben.json
    );
    assert_eq!(ben.json["nachrichten"][0]["nachricht"]["zeit"], json!(td));
    assert_eq!(
        ben.json["nachrichten"][0]["nachricht"]["text"],
        "geheim für @cora"
    );
}

/// W7c, Dritte nie (erster Test der Sichtbarkeit): `cora` wartet ohne Filter, mit `direkt` und mit `erwaehnung`
/// (`@cora` im fremden Text) — `anna` schreibt `ben`: `cora` wird nie geweckt (3 s Frist leer) und bekommt die
/// Nachricht nicht beim Nachholen mit einem `seit` davor; eine Anhang-ID des Gesprächs steht in keiner Antwort an
/// `cora`, wohl in der an `ben` (Kontrolle). Mutante „Sichtbarkeit weg“: `cora` wird geweckt.
#[tokio::test]
async fn ein_dritter_wird_nie_geweckt_und_sieht_keine_anhang_id() {
    let wurzel = ordner();
    let instanz = starten_anhaenge(wurzel.path(), &[], &[("LOCALCHATA_JETZT", JETZT)]);
    let r = runde_mit(wurzel, instanz).await;
    let basis = r.instanz.adresse.clone();
    let t0 = t0(&r, &basis).await;
    let seit = als_parameter(&t0);
    let aufrufe: Vec<_> = ["", "&direkt", "&erwaehnung"]
        .into_iter()
        .map(|filter| {
            let (basis, cora) = (basis.clone(), griff(&r.cora, "127.0.0.1"));
            let anfrage = format!("bis=2&seit={seit}{filter}");
            tokio::spawn(async move { warten(&cora, &basis, &anfrage).await })
        })
        .collect();
    tokio::time::sleep(Duration::from_millis(300)).await;

    let id = ablegen(
        &r.anna.absender,
        &basis,
        &r.anna.token,
        "geheim.txt",
        b"nur wir".to_vec(),
    )
    .await;
    let gesendet = r
        .anna
        .senden(
            &basis,
            BEN,
            &json!({"text": "geheim für @cora", "anhaenge": [id]}),
        )
        .await;
    assert_eq!(gesendet.status, 201, "{}", gesendet.json);

    for aufruf in aufrufe {
        let antwort = aufruf.await.expect("Aufruf");
        assert_eq!(antwort.json["nachrichten"], json!([]), "{}", antwort.json);
        assert_eq!(antwort.json["ende"], "frist", "{}", antwort.json);
    }
    let nachholen = warten(&r.cora, &basis, &format!("bis=0&seit={seit}")).await;
    let kontrolle = warten(&r.ben, &basis, &format!("bis=0&seit={seit}")).await;
    assert_eq!(
        nachholen.json["nachrichten"],
        json!([]),
        "{}",
        nachholen.json
    );
    let an_cora = nachholen.json.to_string();
    assert!(
        !an_cora.contains(&id) && !an_cora.contains("geheim.txt"),
        "{an_cora}"
    );
    let an_ben = kontrolle.json.to_string();
    assert!(an_ben.contains(&id), "Kontrolle: {an_ben}");
}
