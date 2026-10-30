//! F0158 Antwort weckt den Beantworteten — über das echte Programm (R00047 „Wecken“, US-8; ADR-0053 Regel 9,
//! ADR-0054 Regel 2; P13, B0712). `anna`, `claude` und `codex` treten von `127.0.0.1` bei; `codex` wartet mit
//! `erwaehnung`, eine Antwort auf seine Nachricht weckt ihn wie eine Erwähnung.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::senden::{beitreten, ordner, roh_senden, starten};
use gemeinsam::{Absender, ApiAntwort, Instanz, api_antwort};
use serde_json::{Value, json};
use std::time::{Duration, Instant};

struct Gegeben {
    _wurzel: tempfile::TempDir,
    instanz: Instanz,
    absender: Absender,
    anna: String,
    claude: String,
    codex: String,
}

async fn gegeben() -> Gegeben {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[]);
    let absender = Absender::von("127.0.0.1");
    let anna = beitreten(&absender, &instanz.adresse, "anna", Some("mensch")).await;
    let claude = beitreten(&absender, &instanz.adresse, "claude", None).await;
    let codex = beitreten(&absender, &instanz.adresse, "codex", None).await;
    Gegeben {
        _wurzel: wurzel,
        instanz,
        absender,
        anna,
        claude,
        codex,
    }
}

/// Sendet `koerper` nach `# technik` und liefert die Zeit aus der 201.
async fn gesendet(g: &Gegeben, token: &str, koerper: Value) -> String {
    let antwort = roh_senden(
        &g.absender,
        &g.instanz.adresse,
        Some(token),
        "technik",
        "application/json",
        koerper.to_string().into_bytes(),
    )
    .await;
    assert_eq!(antwort.status, 201, "{}", antwort.json);
    antwort.json["nachricht"]["zeit"]
        .as_str()
        .expect("zeit")
        .to_string()
}

async fn text(g: &Gegeben, token: &str, text: &str) -> String {
    gesendet(g, token, json!({ "text": text })).await
}

async fn antwort_auf(g: &Gegeben, token: &str, original: &str, text: &str) -> String {
    gesendet(g, token, json!({ "text": text, "antwort_auf": original })).await
}

/// Öffnet `GET api/warten?<query>&seit=<seit>` als `codex`; liefert Antwort, Dauer ab dem Eingang und Ankunft.
fn warten(
    g: &Gegeben,
    query: &str,
    seit: &str,
) -> tokio::task::JoinHandle<(ApiAntwort, Duration, Instant)> {
    let url = format!(
        "{}api/warten?{query}&seit={}",
        g.instanz.adresse,
        seit.replace('+', "%2B")
    );
    let token = g.codex.clone();
    tokio::spawn(async move {
        let beginn = Instant::now();
        let antwort = api_antwort(
            Absender::von("127.0.0.1")
                .anfrage(reqwest::Method::GET, url)
                .header("X-LocalchatA-Sitzung", token),
        )
        .await;
        (antwort, beginn.elapsed(), Instant::now())
    })
}

fn nicht_geweckt(fall: &str, antwort: &ApiAntwort, dauer: Duration) {
    assert_eq!(antwort.status, 200, "{fall}: {}", antwort.json);
    assert_eq!(antwort.json["nachrichten"], json!([]), "{fall}");
    assert_eq!(antwort.json["ende"], "frist", "{fall}");
    assert!(
        dauer >= Duration::from_secs(3) && dauer <= Duration::from_secs(4),
        "{fall}: {dauer:?}"
    );
}

/// P13: `anna` antwortet auf `N` von `codex` ohne `@codex` → das Warten kehrt binnen 1 s mit der Antwort zurück,
/// `anlass: ["antwort"]`, der Bezug nennt `codex`.
#[tokio::test]
async fn eine_antwort_weckt_den_beantworteten_binnen_einer_sekunde() {
    let g = gegeben().await;
    let n = text(&g, &g.codex, "Wer prüft das?").await;
    let offen = warten(&g, "bis=30&erwaehnung", &n);
    tokio::time::sleep(Duration::from_millis(300)).await;

    let t = antwort_auf(&g, &g.anna, &n, "Ich").await;
    let nach_der_201 = Instant::now();
    let (antwort, _, angekommen) = offen.await.expect("Aufruf");

    let zustellung = angekommen.saturating_duration_since(nach_der_201);
    assert!(zustellung < Duration::from_secs(1), "{zustellung:?}");
    assert_eq!(antwort.json["ende"], "treffer", "{}", antwort.json);
    let nachrichten = antwort.json["nachrichten"].as_array().expect("nachrichten");
    assert_eq!(nachrichten.len(), 1, "{}", antwort.json);
    assert_eq!(nachrichten[0]["nachricht"]["zeit"], json!(t));
    assert_eq!(nachrichten[0]["anlass"], json!(["antwort"]));
    assert_eq!(
        nachrichten[0]["nachricht"]["antwort_auf"]["von"],
        "codex@127.0.0.1"
    );
}

/// P13: eine Antwort auf eine Nachricht von `claude` weckt `codex` nicht.
#[tokio::test]
async fn eine_antwort_an_einen_anderen_weckt_nicht() {
    let g = gegeben().await;
    let m = text(&g, &g.claude, "Frage von claude").await;
    let offen = warten(&g, "bis=3&erwaehnung", &m);
    tokio::time::sleep(Duration::from_millis(300)).await;

    antwort_auf(&g, &g.anna, &m, "Antwort an claude").await;
    let (antwort, dauer, _) = offen.await.expect("Aufruf");

    nicht_geweckt("Antwort an claude", &antwort, dauer);
}

/// P13 (Z12): antwortet `codex` auf die eigene Nachricht, kehrt sein Warten nicht zurück.
#[tokio::test]
async fn die_eigene_antwort_weckt_nicht() {
    let g = gegeben().await;
    let n = text(&g, &g.codex, "Wer prüft das?").await;
    let offen = warten(&g, "bis=3&erwaehnung", &n);
    tokio::time::sleep(Duration::from_millis(300)).await;

    antwort_auf(&g, &g.codex, &n, "Nachtrag").await;
    let (antwort, dauer, _) = offen.await.expect("Aufruf");

    nicht_geweckt("eigene Antwort", &antwort, dauer);
}

/// P13: ein Ereignis `aenderung` weckt nicht — `anna` bearbeitet ihre Antwort auf `N`, während `codex` wartet.
#[tokio::test]
async fn die_bearbeitung_einer_antwort_weckt_nicht() {
    let g = gegeben().await;
    let n = text(&g, &g.codex, "Wer prüft das?").await;
    let t = antwort_auf(&g, &g.anna, &n, "Ich").await;
    let offen = warten(&g, "bis=3&erwaehnung", &t);
    tokio::time::sleep(Duration::from_millis(300)).await;

    let bearbeitet = api_antwort(
        g.absender
            .anfrage(
                reqwest::Method::POST,
                format!(
                    "{}api/channels/technik/nachrichten/bearbeiten",
                    g.instanz.adresse
                ),
            )
            .header("X-LocalchatA-Sitzung", &g.anna)
            .header("Content-Type", "application/json")
            .body(json!({"zeit": t, "text": "Ich, morgen"}).to_string()),
    )
    .await;
    assert!(
        (200..300).contains(&bearbeitet.status),
        "{}",
        bearbeitet.json
    );
    let (antwort, dauer, _) = offen.await.expect("Aufruf");

    nicht_geweckt("Bearbeitung", &antwort, dauer);
}

/// Beim Nachholen liest `warten` die Kennung aus der Metazeile: mit `seit` vor der Antwort kommt sie mit
/// `anlass: ["antwort"]`, die Antwort an `claude` nicht.
#[tokio::test]
async fn eine_verpasste_antwort_wird_nachgeholt() {
    let g = gegeben().await;
    let n = text(&g, &g.codex, "Wer prüft das?").await;
    let m = text(&g, &g.claude, "Frage von claude").await;
    let an_claude = antwort_auf(&g, &g.anna, &m, "Antwort an claude").await;
    let t = antwort_auf(&g, &g.anna, &n, "Ich").await;

    let (antwort, _, _) = warten(&g, "bis=0&erwaehnung", &n).await.expect("Aufruf");

    assert_eq!(antwort.status, 200, "{}", antwort.json);
    let nachrichten = antwort.json["nachrichten"].as_array().expect("nachrichten");
    let zeiten: Vec<&Value> = nachrichten
        .iter()
        .map(|eintrag| &eintrag["nachricht"]["zeit"])
        .collect();
    assert_eq!(zeiten, vec![&json!(t)], "nicht {an_claude}");
    assert_eq!(nachrichten[0]["anlass"], json!(["antwort"]));
}
