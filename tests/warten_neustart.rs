//! F0167 Verlust über Neustart (R00049 US-22, QW1 verkleinert; ADR-0054 Regel 5; W8, B0745): `agent` wartet in
//! Schleife mit `erwaehnung` und gibt jedes `seit` weiter; nach etwa 150 empfangenen Erwähnungen endet die Instanz
//! mit SIGTERM und startet auf demselben Ordner neu → empfangen ist genau, was mit 201 bestätigt wurde, jede einmal.
//!
//! „3 Sender × 3 Channels × 100“: `anna`, `ben` und `mia` senden in jeden Channel je 100 Nachrichten mit `@agent`.
//! Jedes Paar (Sender, Channel) kommt von einer eigenen Herkunft (`127.0.0.2` bis `127.0.0.10`) mit vier Kennungen,
//! weil das Schreibkonto (30 je Kennung, 120 je Herkunft) im Test wie im Betrieb gilt. Wer während des Neustarts
//! scheitert, wiederholt; wer 401 bekommt, tritt mit seinem Sitzungs-Header wieder bei.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::senden::{beitreten, ordner, starten};
use gemeinsam::{Absender, Instanz};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;
use tokio::sync::RwLock;

const CHANNELS: [&str; 3] = ["allgemein", "technik", "zufall"];
const SENDER: [&str; 3] = ["anna", "ben", "mia"];
const KENNUNGEN: usize = 4;
const JE_CHANNEL: usize = 100;
const NEUSTART_NACH: usize = 150;

/// Die Adresse der laufenden Instanz — nach dem Neustart eine andere.
type Adresse = Arc<RwLock<String>>;

/// Eine Anfrage, die scheitern darf: Status und JSON, oder keiner, wenn die Instanz gerade nicht antwortet.
async fn versuchen(anfrage: reqwest::RequestBuilder) -> Option<(u16, Value)> {
    let antwort = anfrage.send().await.ok()?;
    let status = antwort.status().as_u16();
    let text = antwort.text().await.ok()?;
    Some((status, serde_json::from_str(&text).unwrap_or(Value::Null)))
}

/// Tritt mit dem bisherigen Sitzungs-Header wieder bei und liefert das (neue) Token; bis die Instanz antwortet.
async fn wieder_beitreten(
    absender: &Absender,
    adresse: &Adresse,
    name: &str,
    token: &str,
) -> String {
    loop {
        let basis = adresse.read().await.clone();
        let anfrage = absender
            .anfrage(reqwest::Method::POST, format!("{basis}api/beitreten"))
            .header("X-LocalchatA-Sitzung", token)
            .json(&json!({ "name": name }));
        match versuchen(anfrage).await {
            Some((200 | 201, json)) => return json["token"].as_str().expect("Token").to_string(),
            _ => tokio::time::sleep(Duration::from_millis(50)).await,
        }
    }
}

/// Ein Paar (Sender, Channel): 100 Nachrichten mit `@agent` in zufälligem Abstand, reihum über vier Kennungen;
/// liefert die Zeiten aus den 201.
async fn senden(adresse: Adresse, nummer: usize, gesendet: Arc<AtomicUsize>) -> Vec<String> {
    let absender = Absender::von(&format!("127.0.0.{}", 2 + nummer));
    let sender = SENDER[nummer / CHANNELS.len()];
    let channel = CHANNELS[nummer % CHANNELS.len()];
    let basis = adresse.read().await.clone();
    let mut kennungen = Vec::new();
    for kennung in 0..KENNUNGEN {
        let name = format!("{sender}-{channel}-{kennung}");
        let token = beitreten(&absender, &basis, &name, None).await;
        kennungen.push((name, token));
    }
    let mut zufall = 0x9e37_79b9_u64.wrapping_mul(nummer as u64 + 1);
    let mut zeiten = Vec::new();
    for lauf in 0..JE_CHANNEL {
        zufall = zufall
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        tokio::time::sleep(Duration::from_millis((zufall >> 33) % 40)).await;
        let (name, token) = &mut kennungen[lauf % KENNUNGEN];
        let text = format!("@agent {sender} {channel} {lauf}");
        loop {
            let basis = adresse.read().await.clone();
            let anfrage = absender
                .anfrage(
                    reqwest::Method::POST,
                    format!("{basis}api/channels/{channel}/nachrichten"),
                )
                .header("X-LocalchatA-Sitzung", token.as_str())
                .json(&json!({ "text": text }));
            match versuchen(anfrage).await {
                Some((201, json)) => {
                    zeiten.push(
                        json["nachricht"]["zeit"]
                            .as_str()
                            .expect("zeit")
                            .to_string(),
                    );
                    gesendet.fetch_add(1, Ordering::SeqCst);
                    break;
                }
                Some((401, _)) => *token = wieder_beitreten(&absender, &adresse, name, token).await,
                Some((429, _)) => tokio::time::sleep(Duration::from_millis(500)).await,
                Some((status @ (400..=499), json)) => panic!("{text}: {status} {json}"),
                _ => tokio::time::sleep(Duration::from_millis(50)).await,
            }
        }
    }
    zeiten
}

/// `agent` ruft `warten?bis=5&erwaehnung` in Schleife mit seinem `seit`, bis alle Sender fertig sind und ein
/// Aufruf leer mit `frist` endet; meldet über `empfangen` die Zahl der Treffer. Liefert die Zeiten in Empfangsfolge.
async fn agent(
    adresse: Adresse,
    mut token: String,
    start: String,
    empfangen: Arc<AtomicUsize>,
    fertig: Arc<AtomicBool>,
) -> Vec<String> {
    let absender = Absender::von("127.0.0.1");
    let mut seit = start;
    let mut zeiten = Vec::new();
    loop {
        let basis = adresse.read().await.clone();
        let anfrage = absender
            .anfrage(
                reqwest::Method::GET,
                format!(
                    "{basis}api/warten?bis=5&erwaehnung&seit={}",
                    seit.replace('+', "%2B")
                ),
            )
            .header("X-LocalchatA-Sitzung", token.as_str());
        let alle_gesendet = fertig.load(Ordering::SeqCst);
        match versuchen(anfrage).await {
            Some((200, json)) => {
                let treffer = json["nachrichten"].as_array().expect("nachrichten");
                zeiten.extend(treffer.iter().map(|eintrag| {
                    eintrag["nachricht"]["zeit"]
                        .as_str()
                        .expect("zeit")
                        .to_string()
                }));
                empfangen.store(zeiten.len(), Ordering::SeqCst);
                if let Some(neu) = json["seit"].as_str() {
                    seit = neu.to_string();
                }
                if alle_gesendet && treffer.is_empty() && json["ende"] == "frist" {
                    return zeiten;
                }
            }
            Some((401, _)) => token = wieder_beitreten(&absender, &adresse, "agent", &token).await,
            Some((status, json)) => panic!("warten: {status} {json}"),
            None => tokio::time::sleep(Duration::from_millis(50)).await,
        }
    }
}

/// US-22: SIGTERM mitten in der Last und Neustart auf demselben Ordner — die Menge der empfangenen Zeiten ist die
/// Menge der 201 (900), keine doppelt.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ueber_einen_neustart_der_instanz_geht_keine_erwaehnung_verloren() {
    let wurzel = ordner();
    std::fs::create_dir_all(wurzel.path().join("channels/zufall")).expect("Channel zufall");
    let mut instanz: Instanz = starten(wurzel.path(), &[]);
    let lokal = Absender::von("127.0.0.1");
    let anna = beitreten(&lokal, &instanz.adresse, "anna", Some("mensch")).await;
    let token = beitreten(&lokal, &instanz.adresse, "agent", None).await;
    let start = gemeinsam::senden::senden(&lokal, &instanz.adresse, &anna, "allgemein", "start")
        .await
        .json["nachricht"]["zeit"]
        .as_str()
        .expect("zeit")
        .to_string();
    let adresse: Adresse = Arc::new(RwLock::new(instanz.adresse.clone()));
    let empfangen = Arc::new(AtomicUsize::new(0));
    let fertig = Arc::new(AtomicBool::new(false));
    let gesendet = Arc::new(AtomicUsize::new(0));

    let wartend = tokio::spawn(agent(
        Arc::clone(&adresse),
        token,
        start,
        Arc::clone(&empfangen),
        Arc::clone(&fertig),
    ));
    let sendend: Vec<_> = (0..SENDER.len() * CHANNELS.len())
        .map(|nummer| tokio::spawn(senden(Arc::clone(&adresse), nummer, Arc::clone(&gesendet))))
        .collect();
    while empfangen.load(Ordering::SeqCst) < NEUSTART_NACH {
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    let vor_dem_neustart = empfangen.load(Ordering::SeqCst);
    let gesendet_vor_dem_neustart = gesendet.load(Ordering::SeqCst);
    let ende = tokio::task::block_in_place(|| instanz.signal_und_warten("TERM"));
    assert!(ende.success(), "{ende:?}");
    instanz = starten(wurzel.path(), &[]);
    *adresse.write().await = instanz.adresse.clone();

    let mut bestaetigt = Vec::new();
    for handle in sendend {
        bestaetigt.extend(handle.await.expect("Sender"));
    }
    fertig.store(true, Ordering::SeqCst);
    let zeiten = wartend.await.expect("agent");

    let bestaetigt: BTreeSet<String> = bestaetigt.into_iter().collect();
    let menge: BTreeSet<&String> = zeiten.iter().collect();
    assert_eq!(bestaetigt.len(), SENDER.len() * CHANNELS.len() * JE_CHANNEL);
    assert!(
        gesendet_vor_dem_neustart < bestaetigt.len(),
        "Neustart erst nach {gesendet_vor_dem_neustart} Sendungen ({vor_dem_neustart} empfangen)"
    );
    assert_eq!(menge.len(), zeiten.len(), "keine doppelt");
    assert_eq!(menge, bestaetigt.iter().collect::<BTreeSet<_>>());
}
