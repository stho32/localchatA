//! F0011 Teilnehmerliste über die API abfragen — über das echte Programm (R00004, US-1 bis US-4).
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::{Absender, ApiAntwort, api_antwort, wegwerf_starten};
use serde_json::json;

fn token_aus(antwort: &ApiAntwort) -> String {
    antwort.json["token"].as_str().expect("Token").to_string()
}

async fn liste(absender: &Absender, basis: &str, token: Option<&str>) -> ApiAntwort {
    let mut anfrage = absender.anfrage(reqwest::Method::GET, format!("{basis}api/teilnehmer"));
    if let Some(token) = token {
        anfrage = anfrage.header("X-LocalchatA-Sitzung", token);
    }
    api_antwort(anfrage).await
}

#[tokio::test]
async fn us1_liste_mit_allen_teilnehmern_nach_kennung() {
    let instanz = wegwerf_starten();
    let (zwei, drei) = (Absender::von("127.0.0.2"), Absender::von("127.0.0.3"));
    zwei.beitreten(&instanz.adresse, json!({"name": "alpha"}), None)
        .await;
    let beta = token_aus(
        &drei
            .beitreten(&instanz.adresse, json!({"name": "beta"}), None)
            .await,
    );

    let antwort = liste(&drei, &instanz.adresse, Some(&beta)).await;

    assert_eq!(antwort.status, 200, "{}", antwort.json);
    assert_eq!(antwort.kopf["cache-control"], "no-store");
    assert_eq!(antwort.kopf["x-content-type-options"], "nosniff");
    assert!(antwort.json["stand"].as_u64().expect("stand") >= 2);
    let eintraege = antwort.json["liste"].as_array().expect("liste");
    let kennungen: Vec<&str> = eintraege
        .iter()
        .map(|eintrag| eintrag["kennung"].as_str().expect("kennung"))
        .collect();
    assert_eq!(kennungen, ["alpha@127.0.0.2", "beta@127.0.0.3"]);
    for (eintrag, (name, herkunft)) in eintraege
        .iter()
        .zip([("alpha", "127.0.0.2"), ("beta", "127.0.0.3")])
    {
        assert_eq!(eintrag["name"], name);
        assert_eq!(eintrag["herkunft"], herkunft);
        assert_eq!(eintrag["art"], "agent");
        assert!(
            ["online", "offline"].contains(&eintrag["status"].as_str().expect("status")),
            "{eintrag}"
        );
        let seit = eintrag["seit"].as_str().expect("seit");
        chrono::DateTime::parse_from_rfc3339(seit).expect("seit nach RFC 3339");
    }
}

#[tokio::test]
async fn us3_ohne_gueltige_sitzung_keine_liste() {
    let instanz = wegwerf_starten();
    let (zwei, drei) = (Absender::von("127.0.0.2"), Absender::von("127.0.0.3"));
    let alpha = token_aus(
        &zwei
            .beitreten(&instanz.adresse, json!({"name": "alpha"}), None)
            .await,
    );

    let ohne = liste(&drei, &instanz.adresse, None).await;
    let null = liste(
        &drei,
        &instanz.adresse,
        Some("00000000000000000000000000000000"),
    )
    .await;
    let fremd = liste(&drei, &instanz.adresse, Some(&alpha)).await;

    for antwort in [ohne, null, fremd] {
        assert_eq!(antwort.status, 401, "{}", antwort.json);
        assert!(antwort.json["fehler"].is_string(), "{}", antwort.json);
    }
}

fn eintrag<'a>(antwort: &'a ApiAntwort, kennung: &str) -> Option<&'a serde_json::Value> {
    antwort.json["liste"]
        .as_array()
        .expect("liste")
        .iter()
        .find(|eintrag| eintrag["kennung"] == kennung)
}

fn zeit(eintrag: &serde_json::Value) -> chrono::DateTime<chrono::FixedOffset> {
    chrono::DateTime::parse_from_rfc3339(eintrag["seit"].as_str().expect("seit")).expect("RFC 3339")
}

/// Fragt die Liste alle 50 ms ab, bis `passt` gilt, höchstens bis `frist` — liefert die passende Antwort.
async fn abfragen_bis(
    absender: &Absender,
    basis: &str,
    token: &str,
    frist: tokio::time::Instant,
    passt: impl Fn(&ApiAntwort) -> bool,
) -> Option<ApiAntwort> {
    loop {
        let antwort = liste(absender, basis, Some(token)).await;
        if passt(&antwort) {
            return Some(antwort);
        }
        if tokio::time::Instant::now() >= frist {
            return None;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
}

fn status_von(antwort: &ApiAntwort, kennung: &str) -> Option<String> {
    eintrag(antwort, kennung).map(|e| e["status"].as_str().unwrap_or_default().to_string())
}

/// US-2, Rechenbeispiel mit echter Zeit: gleich nach dem Beitritt (t0) online seit t0; offline seit t0 + 1 s,
/// gemeldet spätestens mit dem nächsten Takt — abgefragt bis zu einer Obergrenze von 2 s nach der 201,
/// gemessen ab der 201 (Nacharbeit 1, A1); der Stand ist gewachsen.
#[tokio::test]
async fn us2_wer_keinen_strom_offen_hat_steht_nach_einer_sekunde_als_offline() {
    let instanz = wegwerf_starten();
    let (zwei, drei) = (Absender::von("127.0.0.2"), Absender::von("127.0.0.3"));
    let beta = token_aus(
        &drei
            .beitreten(&instanz.adresse, json!({"name": "beta"}), None)
            .await,
    );
    zwei.beitreten(&instanz.adresse, json!({"name": "alpha"}), None)
        .await;
    let nach_der_201 = tokio::time::Instant::now();

    let frueh = liste(&drei, &instanz.adresse, Some(&beta)).await;
    let spaet = abfragen_bis(
        &drei,
        &instanz.adresse,
        &beta,
        nach_der_201 + std::time::Duration::from_secs(2),
        |antwort| status_von(antwort, "alpha@127.0.0.2").as_deref() == Some("offline"),
    )
    .await
    .expect("alpha binnen 2 s nach der 201 offline");

    let alpha_frueh = eintrag(&frueh, "alpha@127.0.0.2").expect("alpha gleich nach der 201");
    let alpha_spaet = eintrag(&spaet, "alpha@127.0.0.2").expect("alpha offline");
    assert_eq!(alpha_frueh["status"], "online", "{alpha_frueh}");
    assert_eq!(
        zeit(alpha_spaet) - zeit(alpha_frueh),
        chrono::Duration::seconds(1),
        "offline seit Beitritt + 1 s"
    );
    assert!(spaet.json["stand"].as_u64() > frueh.json["stand"].as_u64());
}

/// US-4 mit `LOCALCHATA_GNADENFRIST_MS=3000`: alpha steht offline (Abfrage bis 2 s nach der 201), fehlt nach
/// der Gnadenfrist (bis 4,5 s nach der 201, nicht vor 3 s), und der Name ist frei (Nacharbeit 1, A1).
/// beta bleibt selbst am Leben: es fragt bis zum Verschwinden alle 50 ms ab, also auch um 2 s (Nacharbeit 2).
#[tokio::test]
async fn us4_wer_sich_nicht_mehr_meldet_verschwindet_aus_der_liste() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let instanz = gemeinsam::starten_mit(
        wurzel.path(),
        &["--port", "0", "--host", "127.0.0.1", "--kein-browser"],
        &[("LOCALCHATA_GNADENFRIST_MS", "3000")],
    );
    let (zwei, drei) = (Absender::von("127.0.0.2"), Absender::von("127.0.0.3"));
    let beta = token_aus(
        &drei
            .beitreten(&instanz.adresse, json!({"name": "beta"}), None)
            .await,
    );
    let vor_der_anfrage = tokio::time::Instant::now();
    zwei.beitreten(&instanz.adresse, json!({"name": "alpha"}), None)
        .await;
    let nach_der_201 = tokio::time::Instant::now();
    let sekunden = |s: f64| std::time::Duration::from_secs_f64(s);

    let offline = abfragen_bis(
        &drei,
        &instanz.adresse,
        &beta,
        nach_der_201 + sekunden(2.0),
        |a| status_von(a, "alpha@127.0.0.2").as_deref() == Some("offline"),
    )
    .await;
    let weg = abfragen_bis(
        &drei,
        &instanz.adresse,
        &beta,
        nach_der_201 + sekunden(4.5),
        |a| eintrag(a, "alpha@127.0.0.2").is_none(),
    )
    .await;
    let weg_nach = vor_der_anfrage.elapsed();
    let neu = drei
        .beitreten(&instanz.adresse, json!({"name": "alpha"}), None)
        .await;

    assert!(offline.is_some(), "alpha binnen 2 s offline");
    assert!(weg.is_some(), "alpha binnen 4,5 s weg");
    assert!(
        weg_nach >= sekunden(3.0),
        "nicht vor der Gnadenfrist: {weg_nach:?}"
    );
    assert_eq!(neu.status, 201, "{}", neu.json);
}

/// Nacharbeit 2, Entwicklertest US-4 (a): die Liste ist zum Abrufzeitpunkt aktuell, nicht erst mit dem
/// nächsten Takt. Je Abstand 10 Läufe (parallel, je eigener Name): alpha tritt bei t0 bei, beta fragt ab
/// der 201 nach 1,0 / 1,05 / 1,1 / 1,2 s — alpha steht jedes Mal offline, seit = t0 + 1 s. Ebenso im ersten
/// Stand eines Stroms, den beta nach 1,05 s öffnet.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn us4a_die_liste_ist_zum_abrufzeitpunkt_aktuell() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let instanz = gemeinsam::starten_mit(
        wurzel.path(),
        &["--port", "0", "--host", "127.0.0.1", "--kein-browser"],
        &[("LOCALCHATA_GNADENFRIST_MS", "3000")],
    );
    let drei = Absender::von("127.0.0.3");
    let beta = token_aus(
        &drei
            .beitreten(&instanz.adresse, json!({"name": "beta"}), None)
            .await,
    );
    let mut laeufe = tokio::task::JoinSet::new();
    for (nummer, abstand_ms) in [1000u64, 1050, 1100, 1200, 1050]
        .into_iter()
        .flat_map(|abstand| std::iter::repeat_n(abstand, 10))
        .enumerate()
    {
        let (basis, beta) = (instanz.adresse.clone(), beta.clone());
        let als_strom = nummer >= 40;
        laeufe.spawn(async move {
            let (zwei, drei) = (Absender::von("127.0.0.2"), Absender::von("127.0.0.3"));
            let kennung = format!("alpha{nummer}@127.0.0.2");
            zwei.beitreten(&basis, json!({ "name": format!("alpha{nummer}") }), None)
                .await;
            let nach_der_201 = tokio::time::Instant::now();
            let sofort = liste(&drei, &basis, Some(&beta)).await;
            let t0 = zeit(eintrag(&sofort, &kennung).expect("alpha sofort"));
            tokio::time::sleep_until(nach_der_201 + std::time::Duration::from_millis(abstand_ms))
                .await;
            let spaeter = if als_strom {
                erster_stand_im_strom(&drei, &basis, &beta).await
            } else {
                liste(&drei, &basis, Some(&beta)).await.json
            };
            let alpha = spaeter["liste"]
                .as_array()
                .expect("liste")
                .iter()
                .find(|e| e["kennung"] == kennung.as_str())
                .cloned()
                .unwrap_or_default();
            let seit = alpha["seit"]
                .as_str()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok());
            let stimmt =
                alpha["status"] == "offline" && seit == Some(t0 + chrono::Duration::seconds(1));
            (!stimmt).then(|| format!("{abstand_ms} ms, Strom {als_strom}: {alpha} (t0 {t0})"))
        });
    }

    let mut abweichungen = Vec::new();
    while let Some(lauf) = laeufe.join_next().await {
        abweichungen.extend(lauf.expect("Lauf"));
    }
    assert!(
        abweichungen.is_empty(),
        "{} von 50:\n{}",
        abweichungen.len(),
        abweichungen.join("\n")
    );
}

/// Öffnet einen Strom und liefert das erste Ereignis (den Stand) als JSON.
async fn erster_stand_im_strom(absender: &Absender, basis: &str, token: &str) -> serde_json::Value {
    let mut antwort = absender
        .anfrage(reqwest::Method::GET, format!("{basis}api/ereignisse"))
        .header("X-LocalchatA-Sitzung", token)
        .send()
        .await
        .expect("Strom");
    let mut text = String::new();
    while !text.contains("\n\n") {
        let stueck = antwort
            .chunk()
            .await
            .expect("Stück")
            .expect("nicht zu Ende");
        text.push_str(&String::from_utf8_lossy(&stueck));
    }
    let daten = text
        .lines()
        .find_map(|zeile| zeile.strip_prefix("data:"))
        .expect("data-Zeile");
    serde_json::from_str(daten.trim()).expect("JSON")
}
