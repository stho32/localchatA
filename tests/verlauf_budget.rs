//! F0050 Große Verlaufsblöcke begrenzt — über das echte Programm (R00013, K2 und K3; US-5, US-6; ADR-0039
//! Regeln 1–3). `riesig` hat 200 Sätze zu je 16 384 Bytes `x`; ein Block endet vor `VERLAUF_BUDGET` Satzbytes,
//! weitergelesen wird nur, solange `aelter` bzw. `neuer` es sagt — nie über die Zahl der Nachrichten.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::langsam::{RIESIG, riesig_anlegen};
use gemeinsam::{Absender, Instanz, starten_mit};
use serde_json::{Value, json};
use std::fs;
use std::path::Path;

const JETZT: &str = "2026-10-28T15:00:00.000+01:00";
/// Budget aus R00013 (K1, ADR-0039 Regel 1): 256 KiB Satzbytes je Block.
const BUDGET: u64 = 262_144;
/// Die Satzlänge `s` aus der Datei gelesen: alle Sätze sind gleich lang, also Dateilänge / 200.
fn satzlaenge(ordner: &Path) -> u64 {
    let laenge = fs::metadata(ordner.join(RIESIG)).expect("Datei").len();
    assert_eq!(laenge % 200, 0, "Sätze ungleich lang");
    laenge / 200
}

struct Gegeben {
    _wurzel: tempfile::TempDir,
    instanz: Instanz,
    beta: Absender,
    token: String,
    satzlaenge: u64,
}

async fn riesig_gegeben() -> Gegeben {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    riesig_anlegen(wurzel.path());
    let satzlaenge = satzlaenge(wurzel.path());
    let instanz = starten_mit(
        wurzel.path(),
        &["--port", "0", "--host", "127.0.0.1", "--kein-browser"],
        &[("LOCALCHATA_JETZT", JETZT), ("TZ", "Europe/Berlin")],
    );
    let beta = Absender::von("127.0.0.3");
    let beitritt = beta
        .beitreten(&instanz.adresse, json!({ "name": "beta" }), None)
        .await;
    assert_eq!(beitritt.status, 201, "{}", beitritt.json);
    let token = beitritt.json["token"].as_str().expect("Token").to_string();
    Gegeben {
        _wurzel: wurzel,
        instanz,
        beta,
        token,
        satzlaenge,
    }
}

struct Antwort {
    status: u16,
    content_type: String,
    json: Value,
}

async fn abrufen(gegeben: &Gegeben, abfrage: &str) -> Antwort {
    let url = format!(
        "{}api/channels/riesig/nachrichten{abfrage}",
        gegeben.instanz.adresse
    );
    let antwort = gegeben
        .beta
        .anfrage(reqwest::Method::GET, url)
        .header("X-LocalchatA-Sitzung", &gegeben.token)
        .send()
        .await
        .expect("Antwort");
    let status = antwort.status().as_u16();
    let content_type = antwort
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|wert| wert.to_str().ok())
        .unwrap_or_default()
        .to_string();
    let text = antwort.text().await.expect("Inhalt");
    let json = serde_json::from_str(&text).expect("JSON");
    Antwort {
        status,
        content_type,
        json,
    }
}

fn zeiten(block: &Value) -> Vec<String> {
    block["nachrichten"]
        .as_array()
        .expect("nachrichten")
        .iter()
        .map(|nachricht| nachricht["zeit"].as_str().expect("zeit").to_string())
        .collect()
}

fn kodiert(zeit: &str) -> String {
    zeit.replace('+', "%2B")
}

fn satzzeit(index: usize) -> String {
    format!(
        "2026-10-28T08:{:02}:{:02}.000+01:00",
        index / 60,
        index % 60
    )
}

/// Weiterlesen allein über das Fortsetzungs-Flag (ADR-0039 Regel 3): rückwärts mit `vor=<erste Zeit>`, solange
/// `aelter`, bzw. vorwärts mit `nach=<letzte Zeit>`, solange `neuer`. Liefert alle Zeiten älteste zuerst und
/// die Zahl der Abrufe; der letzte Block muss das Flag `false` melden.
async fn alle_lesen(gegeben: &Gegeben, rueckwaerts: bool) -> (Vec<String>, usize) {
    let (flag, cursor) = if rueckwaerts {
        ("aelter", "vor")
    } else {
        ("neuer", "nach")
    };
    let mut abfrage = if rueckwaerts {
        "?n=200".to_string()
    } else {
        "?n=200&nach=2026-10-28T07:59:59.999%2B01:00".to_string()
    };
    let mut gelesen: Vec<String> = Vec::new();
    let mut abrufe = 0;
    loop {
        let antwort = abrufen(gegeben, &abfrage).await;
        assert_eq!(antwort.status, 200, "{}", antwort.json);
        abrufe += 1;
        let block = zeiten(&antwort.json);
        assert!(!block.is_empty(), "leerer Block bei {abfrage}");
        let rand = if rueckwaerts {
            block.first()
        } else {
            block.last()
        }
        .expect("Rand")
        .clone();
        if rueckwaerts {
            gelesen.splice(0..0, block);
        } else {
            gelesen.extend(block);
        }
        if antwort.json[flag] != json!(true) || abrufe > 400 {
            break;
        }
        abfrage = format!("?n=200&{cursor}={}", kodiert(&rand));
    }
    (gelesen, abrufe)
}

fn alle_200_genau_einmal_steigend(gelesen: &[String]) {
    let erwartet: Vec<String> = (0..200).map(satzzeit).collect();
    assert_eq!(gelesen.len(), 200, "gelesen: {}", gelesen.len());
    assert_eq!(gelesen, erwartet.as_slice());
}

/// K2 (US-5, Reproduktion über HTTP): `?n=200` aus `riesig` liefert genau ⌊262 144 / s⌋ = 15 Nachrichten —
/// die neuesten, 08:03:05 bis 08:03:19 —, `aelter: true`, `neuer: false`. Heute (ohne Budget) 200.
#[tokio::test]
async fn k2_ein_block_endet_vor_dem_budget_mit_den_neuesten_und_aelter() {
    let gegeben = riesig_gegeben().await;
    let erwartet_anzahl = usize::try_from(BUDGET / gegeben.satzlaenge).expect("Zahl");
    assert_eq!(gegeben.satzlaenge, 16_436, "Satzlänge laut R00013");
    assert_eq!(erwartet_anzahl, 15);

    let antwort = abrufen(&gegeben, "?n=200").await;

    assert_eq!(antwort.status, 200, "{}", antwort.json);
    let erwartet: Vec<String> = (185..200).map(satzzeit).collect();
    assert_eq!(zeiten(&antwort.json), erwartet);
    assert_eq!(antwort.json["aelter"], json!(true));
    assert_eq!(antwort.json["neuer"], json!(false));
}

/// K2: rückwärts weiter, solange `aelter` → 14 Abrufe (13 × 15 + 1 × 5), alle 200 genau einmal, steigend; der
/// letzte meldet `aelter: false`.
#[tokio::test]
async fn k2_rueckwaerts_solange_aelter_liest_alle_200_in_14_abrufen() {
    let gegeben = riesig_gegeben().await;

    let (gelesen, abrufe) = alle_lesen(&gegeben, true).await;

    alle_200_genau_einmal_steigend(&gelesen);
    assert_eq!(abrufe, 14);
}

/// K2: vorwärts ab vor dem ersten Satz, solange `neuer` → ebenso 14 Abrufe, alle 200 genau einmal.
#[tokio::test]
async fn k2_vorwaerts_solange_neuer_liest_alle_200_in_14_abrufen() {
    let gegeben = riesig_gegeben().await;

    let (gelesen, abrufe) = alle_lesen(&gegeben, false).await;

    alle_200_genau_einmal_steigend(&gelesen);
    assert_eq!(abrufe, 14);
}

/// K2: `n` bleibt Obergrenze unter dem Budget — `n=1` → 1, `n=10` → 10; die Antwort hat genau die Felder aus
/// R00007 und `Content-Type: application/json`.
#[tokio::test]
async fn k2_unter_dem_budget_gilt_n_und_die_antwort_behaelt_ihre_form() {
    let gegeben = riesig_gegeben().await;

    let eins = abrufen(&gegeben, "?n=1").await;
    let zehn = abrufen(&gegeben, "?n=10").await;

    assert_eq!(zeiten(&eins.json), vec![satzzeit(199)]);
    assert_eq!(
        zeiten(&zehn.json),
        (190..200).map(satzzeit).collect::<Vec<_>>()
    );
    for antwort in [&eins, &zehn] {
        assert_eq!(antwort.status, 200);
        assert_eq!(antwort.content_type, "application/json");
        let mut felder: Vec<&str> = antwort
            .json
            .as_object()
            .expect("Objekt")
            .keys()
            .map(String::as_str)
            .collect();
        felder.sort_unstable();
        assert_eq!(felder, ["aelter", "channel", "nachrichten", "neuer"]);
        assert_eq!(antwort.json["channel"], "riesig");
        let nachricht = antwort.json["nachrichten"][0]
            .as_object()
            .expect("Nachricht");
        let mut felder: Vec<&str> = nachricht.keys().map(String::as_str).collect();
        felder.sort_unstable();
        assert_eq!(felder, ["art", "text", "von", "zeit"], "R00011 ergänzt art");
        assert_eq!(
            nachricht["art"],
            serde_json::Value::Null,
            "Satz ohne Metazeile"
        );
        assert_eq!(nachricht["von"], "alpha@127.0.0.2");
        assert_eq!(nachricht["text"], json!("x".repeat(16_384)));
    }
}

/// Zeit und Länge jedes Satzes von `gross`, älteste zuerst, aus den Tagesdateien gelesen: ein Satz beginnt mit
/// einer Zeile `@ <zeit> <von>` (Textzeilen sind eingerückt) und reicht bis vor den nächsten bzw. ans Dateiende.
fn saetze_aus_den_tagesdateien(ordner: &Path) -> Vec<(String, u64)> {
    let mut dateien: Vec<_> = fs::read_dir(ordner.join("channels/gross"))
        .expect("Channel")
        .map(|eintrag| eintrag.expect("Eintrag").path())
        .filter(|pfad| pfad.extension().is_some_and(|endung| endung == "log"))
        .collect();
    dateien.sort();
    let mut saetze: Vec<(String, u64)> = Vec::new();
    for datei in dateien {
        let rohtext = fs::read(&datei).expect("Tagesdatei");
        let mut anfaenge: Vec<usize> = Vec::new();
        let mut zeilenanfang = 0;
        for (index, byte) in rohtext.iter().enumerate() {
            if index == zeilenanfang && rohtext[index..].starts_with(b"@ ") {
                anfaenge.push(index);
            }
            if *byte == b'\n' {
                zeilenanfang = index + 1;
            }
        }
        for (nummer, anfang) in anfaenge.iter().enumerate() {
            let ende = anfaenge.get(nummer + 1).copied().unwrap_or(rohtext.len());
            let kopf = String::from_utf8_lossy(&rohtext[anfang + 2..ende]);
            let zeit = kopf.split(' ').next().expect("Zeit").to_string();
            saetze.push((zeit, (ende - anfang) as u64));
        }
    }
    saetze
}

/// K3 (US-6, Kontrollpaar): bei `gross` (100.000 Nachrichten über 7 Tage) liefert `n=200` an 20 Stellen — darunter
/// das Fenster aus 200 aufeinanderfolgenden Sätzen mit der größten Summe der Satzlängen — je genau 200; ohne
/// Cursor und ohne `n` genau 100. Realistische Blöcke bleiben vollständig.
#[tokio::test]
async fn k3_realistische_bloecke_bleiben_vollstaendig_auch_am_groessten_fenster() {
    use localchata::testdaten::verlauf_erzeugen;
    use localchata::uhr::Zeit;
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let jetzt = Zeit::parse_from_rfc3339(JETZT).expect("jetzt");
    let ortszeit = |zeitpunkt: chrono::DateTime<chrono::Utc>| {
        zeitpunkt.with_timezone(&chrono::Local).fixed_offset()
    };
    verlauf_erzeugen(wurzel.path(), "gross", 100_000, 7, jetzt, ortszeit).expect("erzeugt");
    let saetze = saetze_aus_den_tagesdateien(wurzel.path());
    assert_eq!(saetze.len(), 100_000);
    let mut summe: u64 = saetze[..200].iter().map(|(_, laenge)| laenge).sum();
    let (mut groesste, mut groesstes_ende) = (summe, 200);
    for ende in 201..=saetze.len() {
        summe = summe + saetze[ende - 1].1 - saetze[ende - 201].1;
        if summe > groesste {
            (groesste, groesstes_ende) = (summe, ende);
        }
    }
    eprintln!("messung k3_groesstes_fenster_bytes={groesste} ende={groesstes_ende}");
    let mut enden: Vec<usize> = (0..19)
        .map(|stelle| 200 + stelle * (saetze.len() - 201) / 18)
        .collect();
    enden.push(groesstes_ende);
    let instanz = starten_mit(
        wurzel.path(),
        &["--port", "0", "--host", "127.0.0.1", "--kein-browser"],
        &[("LOCALCHATA_JETZT", JETZT), ("TZ", "Europe/Berlin")],
    );
    let beta = Absender::von("127.0.0.3");
    let beitritt = beta
        .beitreten(&instanz.adresse, json!({ "name": "beta" }), None)
        .await;
    let token = beitritt.json["token"].as_str().expect("Token").to_string();
    let abrufen = |abfrage: String| {
        let url = format!("{}api/channels/gross/nachrichten{abfrage}", instanz.adresse);
        let anfrage = beta
            .anfrage(reqwest::Method::GET, url)
            .header("X-LocalchatA-Sitzung", &token);
        gemeinsam::api_antwort(anfrage)
    };

    for ende in enden {
        let abfrage = match saetze.get(ende) {
            Some((zeit, _)) => format!("?n=200&vor={}", kodiert(zeit)),
            None => "?n=200".to_string(),
        };
        let antwort = abrufen(abfrage.clone()).await;
        assert_eq!(antwort.status, 200, "{abfrage}");
        let erwartet: Vec<&str> = saetze[ende - 200..ende]
            .iter()
            .map(|(zeit, _)| zeit.as_str())
            .collect();
        let anzahl = antwort.json["nachrichten"].as_array().expect("Block").len();
        assert_eq!(anzahl, 200, "{abfrage}");
        let erste = antwort.json["nachrichten"][0]["zeit"]
            .as_str()
            .expect("zeit");
        assert_eq!(
            Zeit::parse_from_rfc3339(erste).ok(),
            Zeit::parse_from_rfc3339(erwartet[0]).ok(),
            "{abfrage}"
        );
    }
    let ohne = abrufen(String::new()).await;
    assert_eq!(
        ohne.json["nachrichten"].as_array().expect("Block").len(),
        100
    );
}
