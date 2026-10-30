//! F0166 Nacharbeit B2 (code-reviewer #17): (a) Warten in der Schleife ist verlustfrei und doppelfrei unter paralleler
//! Last, (b) Warten schreibt nichts in `channels/` (R00049 Zeile 94 und 98; ADR-0054 Regel 3 und 5).
//!
//! Das Schreibkonto (30 je Kennung, 120 je Herkunft) gilt im Test wie im Betrieb: 9 Sender mit je eigener Herkunft
//! (`127.0.0.2` bis `127.0.0.10`) und 4 Kennungen senden je 100 Nachrichten, Abstand 0 bis 19 ms.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::senden::{beitreten, ordner, senden, starten};
use gemeinsam::{Absender, Instanz, api_antwort};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::Duration;

const CHANNELS: [&str; 3] = ["allgemein", "technik", "zufall"];
const SENDER: usize = 9;
const KENNUNGEN_JE_SENDER: usize = 4;
const NACHRICHTEN_JE_SENDER: usize = 100;

/// Ruft in Schleife `bis=2` und gibt jedes `seit` weiter, bis zweimal hintereinander `frist` ohne Treffer kam;
/// liefert alle Zeiten in Empfangsfolge.
async fn schleife(adresse: String, claude: String, filter: &str, start: String) -> Vec<String> {
    let absender = Absender::von("127.0.0.1");
    let mut seit = start;
    let mut leer = 0;
    let mut empfangen = Vec::new();
    while leer < 2 {
        let antwort = api_antwort(
            absender
                .anfrage(
                    reqwest::Method::GET,
                    format!(
                        "{adresse}api/warten?bis=2{filter}&seit={}",
                        seit.replace('+', "%2B")
                    ),
                )
                .header("X-LocalchatA-Sitzung", &claude),
        )
        .await;
        assert_eq!(antwort.status, 200, "{}", antwort.json);
        let treffer = antwort.json["nachrichten"].as_array().expect("nachrichten");
        leer = if treffer.is_empty() { leer + 1 } else { 0 };
        empfangen.extend(treffer.iter().map(|eintrag| {
            eintrag["nachricht"]["zeit"]
                .as_str()
                .expect("zeit")
                .to_string()
        }));
        if let Some(neu) = antwort.json["seit"].as_str() {
            seit = neu.to_string();
        }
    }
    empfangen
}

/// Ein Sender: `NACHRICHTEN_JE_SENDER` Nachrichten in seinen Channel, reihum über seine Kennungen; liefert die
/// Zeiten aus den 201.
async fn sender(adresse: String, nummer: usize) -> Vec<String> {
    let absender = Absender::von(&format!("127.0.0.{}", 2 + nummer));
    let mut tokens = Vec::new();
    for kennung in 0..KENNUNGEN_JE_SENDER {
        tokens.push(beitreten(&absender, &adresse, &format!("s{nummer}k{kennung}"), None).await);
    }
    let channel = CHANNELS[nummer % CHANNELS.len()];
    let mut zufall = 0x9e37_79b9_u64.wrapping_mul(nummer as u64 + 1);
    let mut zeiten = Vec::new();
    for lauf in 0..NACHRICHTEN_JE_SENDER {
        zufall = zufall
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        tokio::time::sleep(Duration::from_millis((zufall >> 33) % 20)).await;
        let token = &tokens[lauf % KENNUNGEN_JE_SENDER];
        let antwort = senden(
            &absender,
            &adresse,
            token,
            channel,
            &format!("n{nummer}-{lauf}"),
        )
        .await;
        assert_eq!(
            antwort.status, 201,
            "Sender {nummer} Lauf {lauf}: {}",
            antwort.json
        );
        zeiten.push(
            antwort.json["nachricht"]["zeit"]
                .as_str()
                .expect("zeit")
                .to_string(),
        );
    }
    zeiten
}

/// US-5/US-8 (AK Zeile 94): 3 Channels × 3 Sender × 100 Nachrichten parallel, `claude` ruft in Schleife — die Menge
/// der empfangenen Zeiten ist die der 201 (900), keine doppelt, streng steigend; mit `channel=technik` genau dessen 300.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn die_schleife_empfaengt_unter_last_alles_genau_einmal_in_zeitfolge() {
    let wurzel = ordner();
    std::fs::create_dir_all(wurzel.path().join("channels/zufall")).expect("Channel zufall");
    let instanz = starten(wurzel.path(), &[]);
    let anna = Absender::von("127.0.0.1");
    let ersterbe = beitreten(&anna, &instanz.adresse, "anna", Some("mensch")).await;
    let claude = beitreten(&anna, &instanz.adresse, "claude", None).await;
    let start = senden(&anna, &instanz.adresse, &ersterbe, "allgemein", "start")
        .await
        .json["nachricht"]["zeit"]
        .as_str()
        .expect("zeit")
        .to_string();

    let alle = tokio::spawn(schleife(
        instanz.adresse.clone(),
        claude.clone(),
        "",
        start.clone(),
    ));
    let einer = tokio::spawn(schleife(
        instanz.adresse.clone(),
        claude,
        "&channel=technik",
        start,
    ));
    let senden: Vec<_> = (0..SENDER)
        .map(|nummer| tokio::spawn(sender(instanz.adresse.clone(), nummer)))
        .collect();
    let mut bestaetigt: BTreeMap<usize, Vec<String>> = BTreeMap::new();
    for (nummer, handle) in senden.into_iter().enumerate() {
        bestaetigt.insert(nummer, handle.await.expect("Sender"));
    }
    let alle = alle.await.expect("Schleife");
    let einer = einer.await.expect("Schleife technik");

    let alle_bestaetigt: BTreeSet<&String> = bestaetigt.values().flatten().collect();
    assert_eq!(alle_bestaetigt.len(), SENDER * NACHRICHTEN_JE_SENDER);
    assert_eq!(alle.len(), 900, "alle empfangen");
    assert_eq!(alle.iter().collect::<BTreeSet<_>>(), alle_bestaetigt);
    assert!(
        alle.windows(2).all(|paar| paar[0] < paar[1]),
        "streng steigend"
    );
    let technik: BTreeSet<&String> = bestaetigt
        .iter()
        .filter(|(nummer, _)| *nummer % CHANNELS.len() == 1)
        .flat_map(|(_, zeiten)| zeiten)
        .collect();
    assert_eq!(technik.len(), 300);
    assert_eq!(einer.len(), 300, "nur technik");
    assert_eq!(einer.iter().collect::<BTreeSet<_>>(), technik);
    assert!(
        einer.windows(2).all(|paar| paar[0] < paar[1]),
        "streng steigend"
    );
}

/// Alle Dateien unter `channels/` mit ihrem Inhalt.
fn abbild(wurzel: &Path) -> BTreeMap<String, Vec<u8>> {
    fn sammeln(ordner: &Path, wurzel: &Path, ziel: &mut BTreeMap<String, Vec<u8>>) {
        for eintrag in std::fs::read_dir(ordner).expect("Ordner").flatten() {
            let pfad = eintrag.path();
            if pfad.is_dir() {
                sammeln(&pfad, wurzel, ziel);
            } else {
                let name = pfad
                    .strip_prefix(wurzel)
                    .expect("Pfad")
                    .display()
                    .to_string();
                ziel.insert(name, std::fs::read(&pfad).expect("Datei"));
            }
        }
    }
    let mut ziel = BTreeMap::new();
    sammeln(&wurzel.join("channels"), wurzel, &mut ziel);
    ziel
}

async fn aufruf(instanz: &Instanz, claude: &str, query: &str) {
    let antwort = api_antwort(
        Absender::von("127.0.0.1")
            .anfrage(
                reqwest::Method::GET,
                format!("{}api/warten?{query}", instanz.adresse),
            )
            .header("X-LocalchatA-Sitzung", claude),
    )
    .await;
    assert_eq!(antwort.status, 200, "{query}: {}", antwort.json);
}

/// AK Zeile 98: 10 Aufrufe (mit `seit` nachholend, ohne Treffer wartend, mit Treffer) verändern keine Datei unter
/// `channels/` — gleicher Inhalt aller Dateien vor und nach (der Vergleich der Inhalte ersetzt die Prüfsumme).
#[tokio::test]
async fn warten_veraendert_keine_datei_unter_channels() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[]);
    let absender = Absender::von("127.0.0.1");
    let anna = beitreten(&absender, &instanz.adresse, "anna", Some("mensch")).await;
    let claude = beitreten(&absender, &instanz.adresse, "claude", None).await;
    let t0 = senden(&absender, &instanz.adresse, &anna, "allgemein", "t0")
        .await
        .json["nachricht"]["zeit"]
        .as_str()
        .expect("zeit")
        .replace('+', "%2B");
    for text in ["eins", "zwei", "drei"] {
        let antwort = senden(&absender, &instanz.adresse, &anna, "technik", text).await;
        assert_eq!(antwort.status, 201);
    }
    let vorher = abbild(wurzel.path());
    assert!(!vorher.is_empty());

    for _ in 0..4 {
        aufruf(&instanz, &claude, &format!("bis=0&seit={t0}")).await;
    }
    for _ in 0..3 {
        aufruf(&instanz, &claude, "bis=1").await;
    }
    for _ in 0..3 {
        aufruf(
            &instanz,
            &claude,
            &format!("bis=0&channel=technik&seit={t0}"),
        )
        .await;
    }

    assert_eq!(abbild(wurzel.path()), vorher);
}
