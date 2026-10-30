//! F0166 Nur Nachrichten wecken — über das echte Programm (R00049 US-6, US-10; ADR-0054 E1, Regel 3.2 und 3.5,
//! W7a; B0740; zugleich B0698 P10 aus R00045). Alle Teilnehmer treten von `127.0.0.1` bei.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::senden::{beitreten, ordner, senden, starten};
use gemeinsam::{Absender, ApiAntwort, Instanz, api_antwort};
use serde_json::{Value, json};
use std::time::{Duration, Instant};

/// Was in den 3 s des Wartens geschieht (US-10 a–h).
#[derive(Debug, Clone, Copy)]
enum Vorfall {
    Tippt,
    DenktNach,
    Bearbeiten,
    Loeschen,
    BeitretenUndVerlassen,
    ChannelAnlegen,
    FristAendern,
    EigenesSenden,
}

/// Die Instanz von US-10: `anna`, `mia`, `claude` beigetreten, zwei Nachrichten von `anna` in `# allgemein`.
struct Gegeben {
    _wurzel: tempfile::TempDir,
    instanz: Instanz,
    absender: Absender,
    anna: String,
    mia: String,
    claude: String,
    erste: String,
    zweite: String,
}

async fn gegeben() -> Gegeben {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[]);
    let absender = Absender::von("127.0.0.1");
    let anna = beitreten(&absender, &instanz.adresse, "anna", Some("mensch")).await;
    let mia = beitreten(&absender, &instanz.adresse, "mia", None).await;
    let claude = beitreten(&absender, &instanz.adresse, "claude", None).await;
    let erste = zeit_von(&senden(&absender, &instanz.adresse, &anna, "allgemein", "erste").await);
    let zweite = zeit_von(&senden(&absender, &instanz.adresse, &anna, "allgemein", "zweite").await);
    Gegeben {
        _wurzel: wurzel,
        instanz,
        absender,
        anna,
        mia,
        claude,
        erste,
        zweite,
    }
}

fn zeit_von(antwort: &ApiAntwort) -> String {
    assert_eq!(antwort.status, 201, "{}", antwort.json);
    antwort.json["nachricht"]["zeit"]
        .as_str()
        .expect("zeit")
        .to_string()
}

/// Eine Anfrage mit Sitzung und optionalem JSON-Körper; der Status muss 2xx sein.
async fn erfolgreich(
    g: &Gegeben,
    methode: reqwest::Method,
    pfad: &str,
    token: &str,
    koerper: Option<Value>,
) {
    let mut anfrage = g
        .absender
        .anfrage(methode, format!("{}{pfad}", g.instanz.adresse))
        .header("X-LocalchatA-Sitzung", token);
    if let Some(koerper) = koerper {
        anfrage = anfrage
            .header("Content-Type", "application/json")
            .body(koerper.to_string());
    }
    let antwort = api_antwort(anfrage).await;
    assert!(
        (200..300).contains(&antwort.status),
        "{pfad}: {} {}",
        antwort.status,
        antwort.json
    );
}

async fn ausloesen(g: &Gegeben, vorfall: Vorfall) {
    use reqwest::Method;
    let aktivitaet = "api/channels/allgemein/aktivitaet";
    match vorfall {
        Vorfall::Tippt => {
            erfolgreich(
                g,
                Method::PUT,
                aktivitaet,
                &g.anna,
                Some(json!({"zustand": "tippt"})),
            )
            .await;
            tokio::time::sleep(Duration::from_secs(2)).await;
            erfolgreich(
                g,
                Method::PUT,
                aktivitaet,
                &g.anna,
                Some(json!({"zustand": "tippt"})),
            )
            .await;
        }
        Vorfall::DenktNach => {
            erfolgreich(
                g,
                Method::PUT,
                aktivitaet,
                &g.mia,
                Some(json!({"zustand": "denkt_nach"})),
            )
            .await;
        }
        Vorfall::Bearbeiten => {
            let koerper = json!({"zeit": g.erste, "text": "erste, bearbeitet"});
            erfolgreich(
                g,
                Method::POST,
                "api/channels/allgemein/nachrichten/bearbeiten",
                &g.anna,
                Some(koerper),
            )
            .await;
        }
        Vorfall::Loeschen => {
            let koerper = json!({"zeit": g.zweite});
            erfolgreich(
                g,
                Method::POST,
                "api/channels/allgemein/nachrichten/loeschen",
                &g.anna,
                Some(koerper),
            )
            .await;
        }
        Vorfall::BeitretenUndVerlassen => {
            let ben = beitreten(&g.absender, &g.instanz.adresse, "ben", Some("mensch")).await;
            erfolgreich(g, Method::POST, "api/verlassen", &ben, None).await;
        }
        Vorfall::ChannelAnlegen => {
            let antwort = g
                .absender
                .channel_anlegen(&g.instanz.adresse, &g.anna, json!({"name": "neu"}))
                .await;
            assert_eq!(antwort.status, 201, "{}", antwort.json);
        }
        Vorfall::FristAendern => {
            erfolgreich(
                g,
                Method::PUT,
                "api/channels/technik/frist",
                &g.anna,
                Some(json!({"tage": 30})),
            )
            .await;
        }
        Vorfall::EigenesSenden => {
            zeit_von(
                &senden(
                    &g.absender,
                    &g.instanz.adresse,
                    &g.claude,
                    "allgemein",
                    "eigene",
                )
                .await,
            );
        }
    }
}

/// Startet `warten?bis=3&channel=allgemein&seit=<seit>` als `claude` und liefert Antwort und Dauer ab dem Senden.
fn warten_starten(
    g: &Gegeben,
    query: &str,
    seit: &str,
) -> tokio::task::JoinHandle<(ApiAntwort, Duration)> {
    let url = format!(
        "{}api/warten?{query}&seit={}",
        g.instanz.adresse,
        seit.replace('+', "%2B")
    );
    let token = g.claude.clone();
    tokio::spawn(async move {
        let beginn = Instant::now();
        let antwort = api_antwort(
            Absender::von("127.0.0.1")
                .anfrage(reqwest::Method::GET, url)
                .header("X-LocalchatA-Sitzung", token),
        )
        .await;
        (antwort, beginn.elapsed())
    })
}

/// US-10: der Vorfall weckt nicht — 200, leer, `ende: "frist"`, 3,0 bis 4,0 s nach dem Eingang.
async fn weckt_nicht(vorfall: Vorfall) {
    let g = gegeben().await;
    let aufruf = warten_starten(&g, "bis=3&channel=allgemein", &g.zweite);
    tokio::time::sleep(Duration::from_millis(300)).await;

    ausloesen(&g, vorfall).await;
    let (antwort, dauer) = aufruf.await.expect("Aufruf");

    assert_eq!(antwort.status, 200, "{vorfall:?}: {}", antwort.json);
    assert_eq!(antwort.json["nachrichten"], json!([]), "{vorfall:?}");
    assert_eq!(antwort.json["ende"], "frist", "{vorfall:?}");
    assert!(
        dauer >= Duration::from_secs(3) && dauer <= Duration::from_secs(4),
        "{vorfall:?}: {dauer:?}"
    );
}

#[tokio::test]
async fn us10a_tippen_weckt_nicht() {
    weckt_nicht(Vorfall::Tippt).await;
}

#[tokio::test]
async fn us10b_nachdenken_weckt_nicht() {
    weckt_nicht(Vorfall::DenktNach).await;
}

#[tokio::test]
async fn us10c_bearbeiten_weckt_nicht() {
    weckt_nicht(Vorfall::Bearbeiten).await;
}

#[tokio::test]
async fn us10d_loeschen_weckt_nicht() {
    weckt_nicht(Vorfall::Loeschen).await;
}

#[tokio::test]
async fn us10e_beitreten_und_verlassen_weckt_nicht() {
    weckt_nicht(Vorfall::BeitretenUndVerlassen).await;
}

#[tokio::test]
async fn us10f_channel_anlegen_weckt_nicht() {
    weckt_nicht(Vorfall::ChannelAnlegen).await;
}

#[tokio::test]
async fn us10g_frist_aendern_weckt_nicht() {
    weckt_nicht(Vorfall::FristAendern).await;
}

#[tokio::test]
async fn us10h_eigenes_senden_weckt_nicht() {
    weckt_nicht(Vorfall::EigenesSenden).await;
}

/// Gegenprobe zu US-10: eine Nachricht von `anna` in `# allgemein` weckt binnen 1 s nach der 201.
#[tokio::test]
async fn eine_nachricht_von_anna_weckt_binnen_einer_sekunde() {
    let g = gegeben().await;
    let aufruf = warten_starten(&g, "bis=3&channel=allgemein", &g.zweite);
    tokio::time::sleep(Duration::from_millis(300)).await;

    let t = zeit_von(
        &senden(
            &g.absender,
            &g.instanz.adresse,
            &g.anna,
            "allgemein",
            "weckt",
        )
        .await,
    );
    let nach_der_201 = Instant::now();
    let (antwort, _) = aufruf.await.expect("Aufruf");

    assert!(
        nach_der_201.elapsed() < Duration::from_secs(1),
        "{:?}",
        nach_der_201.elapsed()
    );
    assert_eq!(
        antwort.json["nachrichten"][0]["nachricht"]["zeit"],
        json!(t)
    );
    assert_eq!(antwort.json["ende"], "treffer");
}

/// US-6 [A2]: ohne Filter weckt eine Nachricht in irgendeinem Channel mit `anlass: []`.
#[tokio::test]
async fn us6_ohne_filter_weckt_jede_channel_nachricht_ohne_anlass() {
    let g = gegeben().await;
    let aufruf = warten_starten(&g, "bis=30", &g.zweite);
    tokio::time::sleep(Duration::from_millis(300)).await;

    let t = zeit_von(
        &senden(
            &g.absender,
            &g.instanz.adresse,
            &g.anna,
            "technik",
            "irgendwo",
        )
        .await,
    );
    let nach_der_201 = Instant::now();
    let (antwort, _) = aufruf.await.expect("Aufruf");

    assert!(
        nach_der_201.elapsed() < Duration::from_secs(1),
        "{:?}",
        nach_der_201.elapsed()
    );
    let eintrag = &antwort.json["nachrichten"][0];
    assert_eq!(eintrag["nachricht"]["zeit"], json!(t));
    assert_eq!(eintrag["channel"], "technik");
    assert_eq!(eintrag["anlass"], json!([]));
}
