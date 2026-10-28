//! F0012 Anwesenheit live im Ereignisstrom — über das echte Programm (R00004, US-5 bis US-13).
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::{Absender, ApiAntwort, Instanz, aufrufen_mit, starten_mit, wegwerf_starten};
use serde_json::{Value, json};
use std::time::Duration;
use tokio::time::Instant;

const LOKAL: &[&str] = &["--port", "0", "--host", "127.0.0.1", "--kein-browser"];

fn token_aus(antwort: &ApiAntwort) -> String {
    antwort.json["token"].as_str().expect("Token").to_string()
}

async fn beitreten(absender: &Absender, instanz: &Instanz, name: &str) -> String {
    token_aus(
        &absender
            .beitreten(&instanz.adresse, json!({ "name": name }), None)
            .await,
    )
}

/// Ein Ereignis, wie der Strom es liefert: `event` (leer für `data`-Ereignisse), `data` als JSON, Ankunft.
#[derive(Debug, Clone)]
struct Empfangen {
    event: String,
    data: Value,
    ankunft: Instant,
}

/// Ein offener Ereignisstrom: liest Teilstücke und zerlegt sie in Ereignisse; `drop` schließt ihn.
struct Strom {
    antwort: reqwest::Response,
    rest: String,
    ereignisse: Vec<Empfangen>,
}

impl Strom {
    async fn oeffnen(absender: &Absender, basis: &str, token: &str) -> Strom {
        let antwort = absender
            .anfrage(reqwest::Method::GET, format!("{basis}api/ereignisse"))
            .header("X-LocalchatA-Sitzung", token)
            .send()
            .await
            .expect("Strom");
        assert_eq!(antwort.status(), 200);
        Strom {
            antwort,
            rest: String::new(),
            ereignisse: Vec::new(),
        }
    }

    /// Liest bis zum nächsten Ereignis, das `passt`, höchstens `hoechstens`; alle gelesenen bleiben in
    /// `ereignisse`. `None`, wenn keines kam oder der Strom endete.
    async fn bis(
        &mut self,
        hoechstens: Duration,
        passt: impl Fn(&Empfangen) -> bool,
    ) -> Option<Empfangen> {
        let ende = Instant::now() + hoechstens;
        let mut gelesen = self.ereignisse.len();
        loop {
            if let Some(treffer) = self.ereignisse[gelesen..].iter().find(|e| passt(e)) {
                return Some(treffer.clone());
            }
            gelesen = self.ereignisse.len();
            let stueck = tokio::time::timeout_at(ende, self.antwort.chunk()).await;
            match stueck {
                Ok(Ok(Some(bytes))) => self.zerlegen(&String::from_utf8_lossy(&bytes)),
                _ => return None,
            }
        }
    }

    /// Liest `dauer` lang alles mit; `true`, wenn der Strom dabei endete.
    async fn mitlesen(&mut self, dauer: Duration) -> bool {
        let ende = Instant::now() + dauer;
        loop {
            match tokio::time::timeout_at(ende, self.antwort.chunk()).await {
                Ok(Ok(Some(bytes))) => self.zerlegen(&String::from_utf8_lossy(&bytes)),
                Ok(_) => return true,
                Err(_) => return false,
            }
        }
    }

    fn zerlegen(&mut self, stueck: &str) {
        self.rest.push_str(stueck);
        while let Some(stelle) = self.rest.find("\n\n") {
            let block: String = self.rest.drain(..stelle + 2).collect();
            let mut event = String::new();
            let mut data = String::new();
            for zeile in block.lines() {
                if let Some(wert) = zeile.strip_prefix("event:") {
                    event = wert.trim().to_string();
                } else if let Some(wert) = zeile.strip_prefix("data:") {
                    data.push_str(wert.trim());
                }
            }
            self.ereignisse.push(Empfangen {
                event,
                data: serde_json::from_str(&data).unwrap_or(Value::Null),
                ankunft: Instant::now(),
            });
        }
    }

    fn wechsel_von(&self, kennung: &str) -> Vec<Empfangen> {
        self.ereignisse
            .iter()
            .filter(|e| {
                e.data["art"] == "anwesenheit" && e.data["teilnehmer"]["kennung"] == kennung
            })
            .cloned()
            .collect()
    }

    fn staende(&self) -> Vec<u64> {
        self.ereignisse
            .iter()
            .filter_map(|e| e.data["stand"].as_u64())
            .collect()
    }
}

fn ist_wechsel(kennung: &'static str, status: &'static str) -> impl Fn(&Empfangen) -> bool {
    move |e| {
        e.data["art"] == "anwesenheit"
            && e.data["teilnehmer"]["kennung"] == kennung
            && e.data["teilnehmer"]["status"] == status
    }
}

fn streng_steigend(staende: &[u64]) -> bool {
    staende.windows(2).all(|paar| paar[0] < paar[1])
}

fn status_in(stand: &Value, kennung: &str) -> Value {
    stand["liste"]
        .as_array()
        .expect("liste")
        .iter()
        .find(|eintrag| eintrag["kennung"] == kennung)
        .map(|eintrag| eintrag["status"].clone())
        .unwrap_or(Value::Null)
}

#[tokio::test]
async fn us9_das_token_zaehlt_nur_im_header() {
    let instanz = wegwerf_starten();
    let zwei = Absender::von("127.0.0.2");
    let alpha = beitreten(&zwei, &instanz, "alpha").await;
    let url = format!("{}api/ereignisse", instanz.adresse);

    let kurz = Duration::from_secs(2);
    let ohne = gemeinsam::api_antwort(
        zwei.anfrage(reqwest::Method::GET, url.clone())
            .timeout(kurz),
    )
    .await;
    let mit_query = gemeinsam::api_antwort(
        zwei.anfrage(reqwest::Method::GET, format!("{url}?sitzung={alpha}"))
            .timeout(kurz),
    )
    .await;
    let fremde_herkunft = gemeinsam::api_antwort(
        Absender::von("127.0.0.3")
            .anfrage(reqwest::Method::GET, url.clone())
            .header("X-LocalchatA-Sitzung", &alpha)
            .timeout(kurz),
    )
    .await;
    let mit_header = zwei
        .anfrage(reqwest::Method::GET, url)
        .header("X-LocalchatA-Sitzung", &alpha)
        .send()
        .await
        .expect("Strom");

    for antwort in [ohne, mit_query, fremde_herkunft] {
        assert_eq!(antwort.status, 401, "{}", antwort.json);
        assert!(antwort.json["fehler"].is_string(), "{}", antwort.json);
    }
    assert_eq!(mit_header.status(), 200);
    assert_eq!(mit_header.headers()["content-type"], "text/event-stream");
    assert_eq!(mit_header.headers()["cache-control"], "no-store");
}

#[tokio::test]
async fn us5_der_strom_beginnt_mit_der_liste() {
    let instanz = wegwerf_starten();
    let (zwei, drei) = (Absender::von("127.0.0.2"), Absender::von("127.0.0.3"));
    beitreten(&zwei, &instanz, "alpha").await;
    let beta = beitreten(&drei, &instanz, "beta").await;
    tokio::time::sleep(Duration::from_millis(1500)).await;

    let mut strom = Strom::oeffnen(&drei, &instanz.adresse, &beta).await;
    let erstes = strom
        .bis(Duration::from_secs(2), |_| true)
        .await
        .expect("erstes Ereignis");

    assert_eq!(erstes.event, "");
    assert_eq!(erstes.data["art"], "teilnehmer", "{}", erstes.data);
    assert_eq!(status_in(&erstes.data, "alpha@127.0.0.2"), "offline");
    assert_eq!(status_in(&erstes.data, "beta@127.0.0.3"), "online");
}

/// US-6, Rechenbeispiel: alpha öffnet → beta liest alpha online in < 1 s; alpha schließt bei t1 → beta liest
/// alpha offline frühestens bei t1 + 1 s und vor t1 + 2 s; stand über den ganzen Strom streng steigend.
#[tokio::test]
async fn us6_kommen_und_gehen_erscheinen_live() {
    let instanz = wegwerf_starten();
    let (zwei, drei) = (Absender::von("127.0.0.2"), Absender::von("127.0.0.3"));
    let alpha = beitreten(&zwei, &instanz, "alpha").await;
    let beta = beitreten(&drei, &instanz, "beta").await;
    let mut strom_beta = Strom::oeffnen(&drei, &instanz.adresse, &beta).await;
    strom_beta
        .bis(
            Duration::from_secs(3),
            ist_wechsel("alpha@127.0.0.2", "offline"),
        )
        .await
        .expect("alpha nach dem Anlauf offline");

    let geoeffnet = Instant::now();
    let strom_alpha = Strom::oeffnen(&zwei, &instanz.adresse, &alpha).await;
    let online = strom_beta
        .bis(
            Duration::from_secs(2),
            ist_wechsel("alpha@127.0.0.2", "online"),
        )
        .await
        .expect("alpha online");
    tokio::time::sleep(Duration::from_millis(500)).await;
    let t1 = Instant::now();
    drop(strom_alpha);
    let offline = strom_beta
        .bis(
            Duration::from_secs(3),
            ist_wechsel("alpha@127.0.0.2", "offline"),
        )
        .await
        .expect("alpha offline");

    assert!(online.ankunft - geoeffnet < Duration::from_secs(1));
    let nach_dem_schliessen = offline.ankunft - t1;
    assert!(
        nach_dem_schliessen >= Duration::from_millis(1000)
            && nach_dem_schliessen < Duration::from_secs(2),
        "{nach_dem_schliessen:?}"
    );
    assert!(offline.data["stand"].as_u64() > online.data["stand"].as_u64());
    assert!(
        streng_steigend(&strom_beta.staende()),
        "{:?}",
        strom_beta.staende()
    );
}

#[tokio::test]
async fn us7_kurzes_neuverbinden_bleibt_unsichtbar() {
    let instanz = wegwerf_starten();
    let (zwei, drei) = (Absender::von("127.0.0.2"), Absender::von("127.0.0.3"));
    let alpha = beitreten(&zwei, &instanz, "alpha").await;
    let beta = beitreten(&drei, &instanz, "beta").await;
    let strom_alpha = Strom::oeffnen(&zwei, &instanz.adresse, &alpha).await;
    let mut strom_beta = Strom::oeffnen(&drei, &instanz.adresse, &beta).await;
    strom_beta.mitlesen(Duration::from_millis(1500)).await;
    let vorher = strom_beta.wechsel_von("alpha@127.0.0.2").len();

    drop(strom_alpha);
    tokio::time::sleep(Duration::from_millis(400)).await;
    let _wieder = Strom::oeffnen(&zwei, &instanz.adresse, &alpha).await;
    strom_beta.mitlesen(Duration::from_secs(2)).await;

    assert_eq!(
        strom_beta.wechsel_von("alpha@127.0.0.2").len(),
        vorher,
        "{:?}",
        strom_beta.wechsel_von("alpha@127.0.0.2")
    );
}

/// US-8 mit `LOCALCHATA_GNADENFRIST_MS=3000`: gamma tritt bei 0 s bei und schweigt → offline zwischen 1 s und
/// 2 s, weg zwischen 3 s und 4 s (letztes Lebenszeichen 0 s + 3 s, plus höchstens ein Takt).
#[tokio::test]
async fn us8_wer_sich_nicht_mehr_meldet_wird_als_weg_gemeldet() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let instanz = starten_mit(
        wurzel.path(),
        LOKAL,
        &[("LOCALCHATA_GNADENFRIST_MS", "3000")],
    );
    let (zwei, drei) = (Absender::von("127.0.0.2"), Absender::von("127.0.0.3"));
    let beta = beitreten(&drei, &instanz, "beta").await;
    let mut strom = Strom::oeffnen(&drei, &instanz.adresse, &beta).await;
    strom.bis(Duration::from_secs(2), |_| true).await;

    let null = Instant::now();
    beitreten(&zwei, &instanz, "gamma").await;
    let online = strom
        .bis(
            Duration::from_secs(1),
            ist_wechsel("gamma@127.0.0.2", "online"),
        )
        .await;
    let offline = strom
        .bis(
            Duration::from_secs(3),
            ist_wechsel("gamma@127.0.0.2", "offline"),
        )
        .await
        .expect("gamma offline");
    let weg = strom
        .bis(
            Duration::from_secs(5),
            ist_wechsel("gamma@127.0.0.2", "weg"),
        )
        .await
        .expect("gamma weg");

    assert!(online.is_some(), "gamma online");
    let bei = |e: &Empfangen| e.ankunft - null;
    assert!(
        bei(&offline) >= Duration::from_secs(1) && bei(&offline) < Duration::from_secs(2),
        "{:?}",
        bei(&offline)
    );
    assert!(
        bei(&weg) >= Duration::from_secs(3) && bei(&weg) < Duration::from_secs(4),
        "{:?}",
        bei(&weg)
    );
    assert!(streng_steigend(&strom.staende()), "{:?}", strom.staende());
}

/// US-10: mit `LOCALCHATA_HERZSCHLAG_MS=500` in 2 s mindestens 3 Zeilen `event: herzschlag` (2000 / 500 = 4,
/// eine Toleranz).
#[tokio::test]
async fn us10_herzschlag_haelt_den_strom_erkennbar_am_leben() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let instanz = starten_mit(wurzel.path(), LOKAL, &[("LOCALCHATA_HERZSCHLAG_MS", "500")]);
    let zwei = Absender::von("127.0.0.2");
    let alpha = beitreten(&zwei, &instanz, "alpha").await;

    let mut strom = Strom::oeffnen(&zwei, &instanz.adresse, &alpha).await;
    strom.mitlesen(Duration::from_secs(2)).await;

    let herzschlaege = strom
        .ereignisse
        .iter()
        .filter(|e| e.event == "herzschlag")
        .count();
    assert!(herzschlaege >= 3, "{herzschlaege}: {:?}", strom.ereignisse);
}

#[test]
fn us11_falscher_herzschlag_wert_verhindert_den_start() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let ordner = wurzel.path().display().to_string();

    let ende = aufrufen_mit(
        &[&ordner, "--port", "0", "--kein-browser"],
        &[("LOCALCHATA_HERZSCHLAG_MS", "bald")],
    );

    assert_eq!(ende.exit, Some(2));
    assert!(
        ende.stderr.contains("LOCALCHATA_HERZSCHLAG_MS"),
        "{}",
        ende.stderr
    );
    assert_eq!(ende.stdout, "", "keine Startzeile");
}

/// US-13: bei offenem Strom SIGTERM → der Client liest das Ende des Stroms, das Programm endet mit Exit 0
/// in weniger als 1 s.
#[tokio::test]
async fn us13_das_programm_endet_sofort_auch_mit_offenem_strom() {
    let mut instanz = wegwerf_starten();
    let drei = Absender::von("127.0.0.3");
    let beta = beitreten(&drei, &instanz, "beta").await;
    let mut strom = Strom::oeffnen(&drei, &instanz.adresse, &beta).await;
    strom.bis(Duration::from_secs(2), |_| true).await;

    let gesendet = std::time::Instant::now();
    gemeinsam::signal_senden(&instanz.kind, "TERM");
    let strom_endete = strom.mitlesen(Duration::from_secs(1)).await;
    let ende = gemeinsam::warten(&mut instanz.kind, Duration::from_secs(1));

    assert!(strom_endete, "der Client liest das Ende des Stroms");
    assert_eq!(ende.and_then(|status| status.code()), Some(0));
    assert!(
        gesendet.elapsed() < Duration::from_secs(1),
        "{:?}",
        gesendet.elapsed()
    );
}

/// US-12: alpha und beta beigetreten, nur beta liest den Strom → sitzungen 2, halter 1; danach halter 0.
#[tokio::test]
async fn us12_gesundheit_zaehlt_sitzungen_und_offene_stroeme() {
    let instanz = wegwerf_starten();
    let (zwei, drei) = (Absender::von("127.0.0.2"), Absender::von("127.0.0.3"));
    beitreten(&zwei, &instanz, "alpha").await;
    let beta = beitreten(&drei, &instanz, "beta").await;
    let mut strom = Strom::oeffnen(&drei, &instanz.adresse, &beta).await;
    strom.bis(Duration::from_secs(2), |_| true).await;

    let (_, offen) = gemeinsam::gesundheit(&instanz.adresse).await;
    drop(strom);
    let mut geschlossen = Value::Null;
    for _ in 0..50 {
        tokio::time::sleep(Duration::from_millis(20)).await;
        geschlossen = gemeinsam::gesundheit(&instanz.adresse).await.1;
        if geschlossen["halter"] == 0 {
            break;
        }
    }

    assert_eq!(
        (&offen["sitzungen"], &offen["halter"]),
        (&json!(2), &json!(1)),
        "{offen}"
    );
    assert_eq!(geschlossen["halter"], 0, "{geschlossen}");
    assert_eq!(geschlossen["sitzungen"], 2, "{geschlossen}");
}

/// Je Strom: (a) `stand` streng steigend; (b) nach dem ersten Stand S kommt jede Nummer S+1 … höchste genau
/// einmal (keine Lücke, kein Doppel). Leer, wenn alles stimmt, sonst die Abweichung.
fn reihenfolge_pruefen(name: &str, strom: &Strom) -> Option<String> {
    let staende = strom.staende();
    let Some((&erster, wechsel)) = staende.split_first() else {
        return Some(format!("{name}: kein Ereignis"));
    };
    if strom.ereignisse.first().map(|e| e.data["art"].clone()) != Some(json!("teilnehmer")) {
        return Some(format!("{name}: erstes Ereignis ist kein Stand"));
    }
    if !streng_steigend(&staende) {
        return Some(format!("{name}: (a) nicht streng steigend {staende:?}"));
    }
    let erwartet: Vec<u64> = (erster + 1..=erster + wechsel.len() as u64).collect();
    if wechsel != erwartet.as_slice() {
        return Some(format!(
            "{name}: (b) Lücke oder Doppel nach Stand {erster}: {wechsel:?}"
        ));
    }
    None
}

async fn lesen_und_pruefen(
    herkunft: String,
    basis: String,
    token: String,
    dauer: Duration,
) -> Option<String> {
    let absender = Absender::von(&herkunft);
    let mut strom = Strom::oeffnen(&absender, &basis, &token).await;
    strom.mitlesen(dauer).await;
    reihenfolge_pruefen(&herkunft, &strom)
}

const ZUSAETZLICHE_STROEME: usize = 400;
const ABSTAND_MS: u64 = 3;

/// Nacharbeit 1, Code-Review B1/B2 (ADR-0016 Regel 5): unter Gleichzeitigkeit — Beitritte mit kurzen eigenen
/// Strömen auf acht Herkünften, dazu 40 Ströme, die im Abstand von 10 ms öffnen — sieht jeder Strom den Stand
/// und danach jeden Wechsel genau einmal, in steigender Folge.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn unter_last_sieht_jeder_strom_jeden_wechsel_genau_einmal_in_folge() {
    let instanz = wegwerf_starten();
    let basis = instanz.adresse.clone();
    let mut beobachter = Vec::new();
    for stelle in 2..=9 {
        let herkunft = format!("127.0.0.{stelle}");
        let token = beitreten(&Absender::von(&herkunft), &instanz, &format!("b{stelle}")).await;
        beobachter.push((herkunft, token));
    }
    let mut aufgaben = tokio::task::JoinSet::new();
    for (herkunft, token) in &beobachter {
        for _ in 0..3 {
            aufgaben.spawn(lesen_und_pruefen(
                herkunft.clone(),
                basis.clone(),
                token.clone(),
                Duration::from_secs(4),
            ));
        }
    }
    for (nummer, (herkunft, _)) in beobachter.iter().enumerate() {
        let (herkunft_der_folge, basis_der_folge) = (herkunft.clone(), basis.clone());
        aufgaben.spawn(async move {
            let (herkunft, basis) = (herkunft_der_folge, basis_der_folge);
            tokio::time::sleep(Duration::from_millis(200)).await;
            let absender = Absender::von(&herkunft);
            for folge in 0..7 {
                let antwort = absender
                    .beitreten(
                        &basis,
                        json!({ "name": format!("j{nummer}-{folge}") }),
                        None,
                    )
                    .await;
                let strom = Strom::oeffnen(&absender, &basis, &token_aus(&antwort)).await;
                tokio::time::sleep(Duration::from_millis(30)).await;
                drop(strom);
            }
            None
        });
        for gruppe in 0..8 {
            let (herkunft, basis) = (herkunft.clone(), basis.clone());
            aufgaben.spawn(async move {
                tokio::time::sleep(Duration::from_millis(250)).await;
                let absender = Absender::von(&herkunft);
                for folge in 0..6 {
                    let name = format!("k{nummer}-{gruppe}-{folge}");
                    absender
                        .beitreten(&basis, json!({ "name": name }), None)
                        .await;
                }
                None
            });
        }
    }
    tokio::time::sleep(Duration::from_millis(200)).await;
    for nummer in 0..ZUSAETZLICHE_STROEME {
        let (herkunft, token) = beobachter[nummer % beobachter.len()].clone();
        let dauer = if nummer < 40 { 2000 } else { 600 };
        aufgaben.spawn(lesen_und_pruefen(
            herkunft,
            basis.clone(),
            token,
            Duration::from_millis(dauer),
        ));
        tokio::time::sleep(Duration::from_millis(ABSTAND_MS)).await;
    }

    let mut abweichungen = Vec::new();
    while let Some(ergebnis) = aufgaben.join_next().await {
        abweichungen.extend(ergebnis.expect("Aufgabe"));
    }
    assert!(abweichungen.is_empty(), "{}", abweichungen.join("\n"));
}
