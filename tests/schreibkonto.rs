//! F0045 Schreibkonto (R00011, US-22 bis US-26): das echte Programm; Spannen aus der gemessenen Dauer.

mod gemeinsam;

use gemeinsam::senden::{beitreten, ordner, roh_senden, senden, starten, tagesinhalt};
use gemeinsam::strom::Strom;
use gemeinsam::{Absender, ApiAntwort};
use serde_json::json;
use std::time::{Duration, Instant};

fn sekunden_aufgerundet(dauer: Duration) -> usize {
    usize::try_from(dauer.as_millis().div_ceil(1_000)).expect("klein")
}

fn saetze(ordner: &std::path::Path) -> usize {
    String::from_utf8(tagesinhalt(ordner, "allgemein"))
        .expect("UTF-8")
        .lines()
        .filter(|zeile| zeile.starts_with("@ "))
        .count()
}

fn retry_after(antwort: &ApiAntwort) -> u64 {
    antwort
        .kopf
        .get("retry-after")
        .and_then(|wert| wert.to_str().ok())
        .and_then(|wert| wert.parse().ok())
        .unwrap_or(0)
}

/// US-22 „Kennung im echten Programm als Spanne“ mit „Betrieb“: 60 schnelle Nachrichten → 201 in
/// [30, 30 + ⌈Dauer⌉ + 1]; jede 429 mit `grenze: kennung`, `wieder_in_ms` > 0, `Retry-After` ≥ 1; Log und Strom
/// tragen genau die durchgegangenen; `gebremst` gleich der Zahl der 429.
#[tokio::test]
async fn us22_ein_zug_geht_durch_eine_schleife_wird_gebremst() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[]);
    let alpha = Absender::von("127.0.0.1");
    let beta = Absender::von("127.0.0.1");
    let token_alpha = beitreten(&alpha, &instanz.adresse, "alpha", None).await;
    let token = beitreten(&beta, &instanz.adresse, "beta", None).await;
    let mut strom = Strom::oeffnen(&alpha, &instanz.adresse, &token_alpha).await;

    let mut antworten = Vec::new();
    let mut erste = None;
    for nummer in 0..60 {
        let antwort = senden(
            &beta,
            &instanz.adresse,
            &token,
            "allgemein",
            &format!("s{nummer}"),
        )
        .await;
        erste.get_or_insert_with(Instant::now);
        antworten.push(antwort);
    }
    let dauer = erste.expect("Antwort").elapsed();
    strom.mitlesen(Duration::from_millis(500)).await;
    let (_, gesundheit) = gemeinsam::gesundheit(&instanz.adresse).await;

    let durch = antworten.iter().filter(|a| a.status == 201).count();
    let gebremst: Vec<_> = antworten.iter().filter(|a| a.status == 429).collect();
    assert_eq!(durch + gebremst.len(), 60);
    assert!(
        (30..=30 + sekunden_aufgerundet(dauer) + 1).contains(&durch),
        "{durch} in {dauer:?}"
    );
    for antwort in &gebremst {
        assert_eq!(antwort.json["grenze"], "kennung");
        assert!(antwort.json["wieder_in_ms"].as_u64().expect("ms") > 0);
        assert!(retry_after(antwort) >= 1);
        assert!(
            antwort.json["fehler"]
                .as_str()
                .expect("fehler")
                .starts_with("Zu viele Nachrichten von beta@127.0.0.1 — wieder möglich in "),
            "{}",
            antwort.json
        );
    }
    assert_eq!(saetze(wurzel.path()), durch);
    let gelesen = strom
        .ereignisse
        .iter()
        .filter(|e| e.data["art"] == "nachricht")
        .count();
    assert_eq!(gelesen, durch);
    assert_eq!(gesundheit["gebremst"], gebremst.len());
    let frisch = ordner();
    let frische_instanz = starten(frisch.path(), &[]);
    assert_eq!(
        gemeinsam::gesundheit(&frische_instanz.adresse).await.1["gebremst"],
        0
    );
}

/// US-24 „Herkunft im echten Programm“: fünf Agenten von 127.0.0.2 senden gleichzeitig je 30 → 201 in
/// [120, 120 + 8 × ⌈Dauer⌉ + 1], mindestens eine 429 mit `grenze: herkunft` und dem Text; danach von 127.0.0.3 → 201.
#[tokio::test]
async fn us24_alle_einer_adresse_zusammen() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[]);
    let mut agenten = Vec::new();
    for nummer in 0..5 {
        let absender = Absender::von("127.0.0.2");
        let token = beitreten(&absender, &instanz.adresse, &format!("h{nummer}"), None).await;
        agenten.push((absender, token));
    }

    let beginn = Instant::now();
    let laeufe: Vec<_> = agenten
        .into_iter()
        .map(|(absender, token)| {
            let basis = instanz.adresse.clone();
            tokio::spawn(async move {
                let mut antworten = Vec::new();
                for nummer in 0..30 {
                    antworten.push(
                        senden(
                            &absender,
                            &basis,
                            &token,
                            "allgemein",
                            &format!("n{nummer}"),
                        )
                        .await,
                    );
                }
                antworten
            })
        })
        .collect();
    let mut antworten = Vec::new();
    for lauf in laeufe {
        antworten.extend(lauf.await.expect("Lauf"));
    }
    let dauer = beginn.elapsed();
    let andere = Absender::von("127.0.0.3");
    let token_andere = beitreten(&andere, &instanz.adresse, "anders", None).await;
    let danach = senden(
        &andere,
        &instanz.adresse,
        &token_andere,
        "allgemein",
        "danach",
    )
    .await;

    let durch = antworten.iter().filter(|a| a.status == 201).count();
    assert!(
        (120..=120 + 8 * sekunden_aufgerundet(dauer) + 1).contains(&durch),
        "{durch} in {dauer:?}"
    );
    let herkunft: Vec<_> = antworten
        .iter()
        .filter(|a| a.status == 429 && a.json["grenze"] == "herkunft")
        .collect();
    assert!(!herkunft.is_empty());
    assert_eq!(
        herkunft[0].json["fehler"],
        format!(
            "Zu viele Nachrichten von 127.0.0.2 (alle Teilnehmer dieser Adresse zusammen) — wieder möglich in {} s.",
            retry_after(herkunft[0])
        )
    );
    assert_eq!(danach.status, 201);
}

/// US-23 „Kosten nach Größe“ mit „Text“: 10 Texte zu 16 384 Bytes → die ersten 7 mit 201, insgesamt höchstens
/// 7 + ⌈Dauer / 4⌉ + 1; die erste 429 mit `grenze: kennung`, `Retry-After: 3` und dem Text.
#[tokio::test]
async fn us23_grosse_texte_kosten_mehr() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[]);
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, &instanz.adresse, "beta", None).await;
    let gross = "a".repeat(16_384);

    let beginn = Instant::now();
    let mut antworten = Vec::new();
    for _ in 0..10 {
        antworten.push(senden(&beta, &instanz.adresse, &token, "allgemein", &gross).await);
    }
    let dauer = beginn.elapsed();

    assert!(antworten[..7].iter().all(|a| a.status == 201));
    let durch = antworten.iter().filter(|a| a.status == 201).count();
    let grenze = 7 + usize::try_from(dauer.as_millis().div_ceil(4_000)).expect("klein") + 1;
    assert!(durch <= grenze, "{durch} in {dauer:?}");
    let erste = antworten.iter().find(|a| a.status == 429).expect("429");
    assert_eq!(erste.json["grenze"], "kennung");
    assert_eq!(retry_after(erste), 3);
    assert_eq!(
        erste.json["fehler"],
        "Zu viele Nachrichten von beta@127.0.0.1 — wieder möglich in 3 s."
    );
}

/// US-26 „Vorab vor dem Körper“: 30 × `{}` (422) und dann gültige bis zur ersten 429 → davor höchstens
/// ⌈Dauer⌉ + 1 mit 201 (Dauer ab dem ersten `{}`, gültig nur unter 5 s); sofort danach 131 073 Bytes → 429, nicht
/// 413. Mit leerem Eimer: fremder `Origin` → 403, ohne Header → 401, `gebremst` steigt dabei nicht.
#[tokio::test]
async fn us26_auch_abgewiesenes_und_grosse_koerper_kosten() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[]);
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, &instanz.adresse, "beta", None).await;
    let basis = &instanz.adresse;
    let leer = json!({}).to_string().into_bytes();

    let beginn = Instant::now();
    for _ in 0..30 {
        let antwort = roh_senden(
            &beta,
            basis,
            Some(&token),
            "allgemein",
            "application/json",
            leer.clone(),
        )
        .await;
        assert_eq!(antwort.status, 422);
    }
    let mut durch = 0;
    loop {
        let antwort = senden(&beta, basis, &token, "allgemein", "gültig").await;
        if antwort.status == 429 {
            break;
        }
        assert_eq!(antwort.status, 201);
        durch += 1;
        assert!(durch <= 30, "nie gebremst");
    }
    let mut gross = b"{".to_vec();
    gross.extend(std::iter::repeat_n(b'a', 131_072));
    let zu_gross = roh_senden(
        &beta,
        basis,
        Some(&token),
        "allgemein",
        "application/json",
        gross,
    )
    .await;
    let dauer = beginn.elapsed();
    let (_, vorher) = gemeinsam::gesundheit(basis).await;
    let fremd = gemeinsam::api_antwort(
        beta.anfrage(
            reqwest::Method::POST,
            format!("{basis}api/channels/allgemein/nachrichten"),
        )
        .header("Content-Type", "application/json")
        .header("Origin", "http://fremd.example")
        .header("X-LocalchatA-Sitzung", &token)
        .body(json!({"text": "x"}).to_string()),
    )
    .await;
    let ohne = roh_senden(
        &beta,
        basis,
        None,
        "allgemein",
        "application/json",
        b"{\"text\":\"x\"}".to_vec(),
    )
    .await;
    let (_, nachher) = gemeinsam::gesundheit(basis).await;

    assert!(
        dauer < Duration::from_secs(5),
        "ungültig, zu langsam: {dauer:?}"
    );
    assert!(
        durch <= sekunden_aufgerundet(dauer) + 1,
        "{durch} in {dauer:?}"
    );
    assert_eq!(zu_gross.status, 429, "{}", zu_gross.json);
    assert_eq!(fremd.status, 403);
    assert_eq!(ohne.status, 401);
    assert_eq!(vorher["gebremst"], nachher["gebremst"]);
}

/// US-25 „Kein Gegenzug“: 30 durch, die 31. → 429; verlassen, mit demselben Namen neu beitreten, sofort 5 → höchstens
/// ⌈Sekunden seit der ersten 429⌉ + 1 mit 201 (gültig nur unter 3 s).
#[tokio::test]
async fn us25_verlassen_und_wiederkommen_hilft_nicht() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[]);
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, &instanz.adresse, "beta", None).await;
    let basis = &instanz.adresse;
    let beginn = Instant::now();
    for nummer in 0..30 {
        assert_eq!(
            senden(&beta, basis, &token, "allgemein", &format!("n{nummer}"))
                .await
                .status,
            201
        );
    }
    assert_eq!(
        senden(&beta, basis, &token, "allgemein", "31").await.status,
        429
    );
    let erste_429 = Instant::now();

    let verlassen = gemeinsam::api_antwort(
        beta.anfrage(reqwest::Method::POST, format!("{basis}api/verlassen"))
            .header("X-LocalchatA-Sitzung", &token),
    )
    .await;
    assert!(verlassen.status < 300, "{}", verlassen.status);
    let neu = beitreten(&beta, basis, "beta", None).await;
    let mut durch = 0;
    for nummer in 0..5 {
        if senden(&beta, basis, &neu, "allgemein", &format!("w{nummer}"))
            .await
            .status
            == 201
        {
            durch += 1;
        }
    }
    let seit = erste_429.elapsed();

    assert!(
        beginn.elapsed() < Duration::from_secs(3),
        "ungültig, zu langsam"
    );
    assert!(
        durch <= sekunden_aufgerundet(seit) + 1,
        "{durch} nach {seit:?}"
    );
}
