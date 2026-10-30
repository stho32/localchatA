//! F0167 Erwähnung weckt — über das echte Programm (R00049 US-14 bis US-21; ADR-0054 E1, E3, Regel 2, 3 und 6,
//! W7b; B0743). `anna`, `claude` und `codex` treten von `127.0.0.1` bei; `codex` wartet mit `erwaehnung`.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::senden::{JETZT, beitreten, ordner, starten};
use gemeinsam::{Absender, ApiAntwort, Instanz, api_antwort};
use localchata::testdaten::verlauf_erzeugen;
use localchata::uhr::Zeit;
use serde_json::json;
use std::path::Path;
use std::time::{Duration, Instant};

struct Gegeben {
    _wurzel: tempfile::TempDir,
    instanz: Instanz,
    absender: Absender,
    anna: String,
    codex: String,
}

async fn gegeben() -> Gegeben {
    gegeben_mit(|_| {}).await
}

/// Wie `gegeben`, nachdem `vorbereiten` den Ordner vor dem Start gefüllt hat.
async fn gegeben_mit(vorbereiten: impl FnOnce(&Path)) -> Gegeben {
    let wurzel = ordner();
    vorbereiten(wurzel.path());
    let instanz = starten(wurzel.path(), &[]);
    let absender = Absender::von("127.0.0.1");
    let anna = beitreten(&absender, &instanz.adresse, "anna", Some("mensch")).await;
    beitreten(&absender, &instanz.adresse, "claude", None).await;
    let codex = beitreten(&absender, &instanz.adresse, "codex", None).await;
    Gegeben {
        _wurzel: wurzel,
        instanz,
        absender,
        anna,
        codex,
    }
}

/// Sendet und liefert die Zeit aus der 201.
async fn gesendet(g: &Gegeben, token: &str, channel: &str, text: &str) -> String {
    let antwort =
        gemeinsam::senden::senden(&g.absender, &g.instanz.adresse, token, channel, text).await;
    assert_eq!(antwort.status, 201, "{}", antwort.json);
    antwort.json["nachricht"]["zeit"]
        .as_str()
        .expect("zeit")
        .to_string()
}

/// Öffnet `GET api/warten?<query>&seit=<seit>` als `token` im Hintergrund; liefert Antwort, Dauer ab dem Eingang
/// und den Augenblick der Ankunft.
fn aufruf(
    g: &Gegeben,
    token: &str,
    query: &str,
    seit: &str,
) -> tokio::task::JoinHandle<(ApiAntwort, Duration, Instant)> {
    let url = format!(
        "{}api/warten?{query}&seit={}",
        g.instanz.adresse,
        seit.replace('+', "%2B")
    );
    let token = token.to_string();
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

/// Ein Aufruf, der nicht geweckt wurde: 200, leer, `ende: "frist"`, 3,0 bis 4,0 s nach dem Eingang (`bis=3`).
fn nicht_geweckt(fall: &str, antwort: &ApiAntwort, dauer: Duration) {
    assert_eq!(antwort.status, 200, "{fall}: {}", antwort.json);
    assert_eq!(antwort.json["nachrichten"], json!([]), "{fall}");
    assert_eq!(antwort.json["ende"], "frist", "{fall}");
    assert!(
        dauer >= Duration::from_secs(3) && dauer <= Duration::from_secs(4),
        "{fall}: {dauer:?}"
    );
}

/// Ein Aufruf, den genau die Nachricht `zeit` weckte, binnen 1 s nach ihrer 201 (`nach_der_201`).
fn geweckt_von(
    fall: &str,
    antwort: &ApiAntwort,
    zeit: &str,
    nach_der_201: Instant,
    angekommen: Instant,
) {
    assert_eq!(antwort.status, 200, "{fall}: {}", antwort.json);
    let zustellung = angekommen.saturating_duration_since(nach_der_201);
    assert!(
        zustellung < Duration::from_secs(1),
        "{fall}: {zustellung:?}"
    );
    assert_eq!(antwort.json["ende"], "treffer", "{fall}: {}", antwort.json);
    let nachrichten = antwort.json["nachrichten"].as_array().expect("nachrichten");
    assert_eq!(nachrichten.len(), 1, "{fall}: {}", antwort.json);
    assert_eq!(nachrichten[0]["nachricht"]["zeit"], json!(zeit), "{fall}");
}

/// US-14: erst die zweite Nachricht, die `@codex` erwähnt, weckt — binnen 1 s, mit Anlass, Channel und `seit`.
#[tokio::test]
async fn us14_eine_erwaehnung_weckt_binnen_einer_sekunde() {
    let g = gegeben().await;
    let t0 = gesendet(&g, &g.anna, "allgemein", "vorher").await;
    let offen = aufruf(&g, &g.codex, "bis=30&erwaehnung", &t0);
    tokio::time::sleep(Duration::from_millis(300)).await;

    gesendet(&g, &g.anna, "technik", "Wer kann das prüfen?").await;
    let t = gesendet(&g, &g.anna, "technik", "@codex bitte prüfen").await;
    let nach_der_201 = Instant::now();
    let (antwort, _, angekommen) = offen.await.expect("Aufruf");

    geweckt_von("US-14", &antwort, &t, nach_der_201, angekommen);
    let eintrag = &antwort.json["nachrichten"][0];
    assert_eq!(eintrag["anlass"], json!(["erwaehnung"]));
    assert_eq!(eintrag["channel"], "technik");
    assert_eq!(eintrag["direkt"], json!(null));
    assert_eq!(antwort.json["seit"], json!(t));
}

/// R00049 F0167 zweites Kriterium: eine Nachricht ohne `@codex` weckt nicht.
#[tokio::test]
async fn eine_nachricht_ohne_erwaehnung_weckt_nicht() {
    let g = gegeben().await;
    let t0 = gesendet(&g, &g.anna, "allgemein", "vorher").await;
    let offen = aufruf(&g, &g.codex, "bis=3&erwaehnung", &t0);
    tokio::time::sleep(Duration::from_millis(300)).await;

    gesendet(&g, &g.anna, "technik", "bitte prüfen").await;
    let (antwort, dauer, _) = offen.await.expect("Aufruf");

    nicht_geweckt("ohne Erwähnung", &antwort, dauer);
}

/// US-15 (a)–(e): so gelesen weckt die Erwähnung `codex` — je ein Senden und ein Wecken über die Route.
#[tokio::test]
async fn us15_diese_texte_wecken_codex() {
    let g = gegeben().await;
    let mut seit = gesendet(&g, &g.anna, "allgemein", "vorher").await;
    for text in [
        "@codex hi",
        "hi @Codex.",
        "@codex@127.0.0.1",
        "`@codex`",
        "@codex-",
    ] {
        let offen = aufruf(&g, &g.codex, "bis=3&erwaehnung", &seit);
        tokio::time::sleep(Duration::from_millis(100)).await;
        let t = gesendet(&g, &g.anna, "technik", text).await;
        let nach_der_201 = Instant::now();
        let (antwort, _, angekommen) = offen.await.expect("Aufruf");

        geweckt_von(text, &antwort, &t, nach_der_201, angekommen);
        assert_eq!(
            antwort.json["nachrichten"][0]["anlass"],
            json!(["erwaehnung"]),
            "{text}"
        );
        seit = t;
    }
}

/// US-15 (f)–(i): E-Mail-Form, längere Folge, Leerzeichen, anderer Name wecken `codex` nicht.
#[tokio::test]
async fn us15_diese_texte_wecken_codex_nicht() {
    let g = gegeben().await;
    let t0 = gesendet(&g, &g.anna, "allgemein", "vorher").await;
    let offen = aufruf(&g, &g.codex, "bis=3&erwaehnung", &t0);
    tokio::time::sleep(Duration::from_millis(300)).await;

    for text in ["shoff@codex", "@codex-entwickler", "@ codex", "@codexx"] {
        gesendet(&g, &g.anna, "technik", text).await;
    }
    let (antwort, dauer, _) = offen.await.expect("Aufruf");

    nicht_geweckt("US-15 (f)–(i)", &antwort, dauer);
}

/// US-15: `@codex-entwickler` weckt einen Teilnehmer namens `codex-entwickler`.
#[tokio::test]
async fn us15_die_laengste_folge_weckt_codex_entwickler() {
    let g = gegeben().await;
    let entwickler = beitreten(&g.absender, &g.instanz.adresse, "codex-entwickler", None).await;
    let t0 = gesendet(&g, &g.anna, "allgemein", "vorher").await;
    let offen = aufruf(&g, &entwickler, "bis=3&erwaehnung", &t0);
    tokio::time::sleep(Duration::from_millis(300)).await;

    let t = gesendet(&g, &g.anna, "technik", "@codex-entwickler bitte").await;
    let nach_der_201 = Instant::now();
    let (antwort, _, angekommen) = offen.await.expect("Aufruf");

    geweckt_von("codex-entwickler", &antwort, &t, nach_der_201, angekommen);
}

/// US-16: Vereinigung `channel=technik&erwaehnung` — (a) ohne Erwähnung in `# allgemein` weckt nicht, (b) mit
/// `@codex` in `# allgemein` weckt mit `erwaehnung`, (c) jede Nachricht in `# technik` weckt mit `[]`.
#[tokio::test]
async fn us16_vereinigung_aus_channel_und_erwaehnung() {
    let g = gegeben().await;
    let t0 = gesendet(&g, &g.anna, "allgemein", "vorher").await;
    let vereinigung = "bis=3&channel=technik&erwaehnung";

    let offen = aufruf(&g, &g.codex, vereinigung, &t0);
    tokio::time::sleep(Duration::from_millis(300)).await;
    let mut seit = gesendet(&g, &g.anna, "allgemein", "Hallo").await;
    let (antwort, dauer, _) = offen.await.expect("Aufruf a");
    nicht_geweckt("US-16 (a)", &antwort, dauer);

    for (fall, channel, text, anlass) in [
        (
            "US-16 (b)",
            "allgemein",
            "@codex Hallo",
            json!(["erwaehnung"]),
        ),
        ("US-16 (c)", "technik", "Hallo", json!([])),
    ] {
        let offen = aufruf(&g, &g.codex, vereinigung, &seit);
        tokio::time::sleep(Duration::from_millis(100)).await;
        let t = gesendet(&g, &g.anna, channel, text).await;
        let nach_der_201 = Instant::now();
        let (antwort, _, angekommen) = offen.await.expect(fall);

        geweckt_von(fall, &antwort, &t, nach_der_201, angekommen);
        assert_eq!(antwort.json["nachrichten"][0]["anlass"], anlass, "{fall}");
        assert_eq!(antwort.json["nachrichten"][0]["channel"], channel, "{fall}");
        seit = t;
    }
}

/// US-17: eine Erwähnung, die `codex` nicht abwartete, kommt mit seinem alten `seit` — genau `t1`, nicht die
/// Erwähnung von `claude`; `seit` mindestens `t2`.
#[tokio::test]
async fn us17_eine_verpasste_erwaehnung_wird_nachgeholt() {
    let g = gegeben().await;
    let t0 = gesendet(&g, &g.anna, "allgemein", "vorher").await;
    let t1 = gesendet(&g, &g.anna, "technik", "@codex bitte prüfen").await;
    let t2 = gesendet(&g, &g.anna, "technik", "@claude bitte prüfen").await;

    let (antwort, _, _) = aufruf(&g, &g.codex, "bis=0&erwaehnung", &t0)
        .await
        .expect("Aufruf");

    assert_eq!(antwort.status, 200, "{}", antwort.json);
    let nachrichten = antwort.json["nachrichten"].as_array().expect("nachrichten");
    assert_eq!(nachrichten.len(), 1, "{}", antwort.json);
    assert_eq!(nachrichten[0]["nachricht"]["zeit"], json!(t1));
    assert_eq!(nachrichten[0]["anlass"], json!(["erwaehnung"]));
    assert_eq!(antwort.json["ende"], "treffer");
    let seit = zeit(antwort.json["seit"].as_str().expect("seit"));
    assert!(seit >= zeit(&t2), "seit {seit} vor t2 {t2}");
}

/// US-17: schreibt `codex` selbst `@codex`, weckt das den eigenen Aufruf nicht.
#[tokio::test]
async fn us17_der_eigene_name_in_der_eigenen_nachricht_weckt_nicht() {
    let g = gegeben().await;
    let t0 = gesendet(&g, &g.anna, "allgemein", "vorher").await;
    let offen = aufruf(&g, &g.codex, "bis=3&erwaehnung", &t0);
    tokio::time::sleep(Duration::from_millis(300)).await;

    gesendet(&g, &g.codex, "technik", "@codex Notiz an mich").await;
    let (antwort, dauer, _) = offen.await.expect("Aufruf");

    nicht_geweckt("eigene Nachricht", &antwort, dauer);
}

/// US-18 [A1]: `@alle` und `@hier` wecken einen Wartenden mit `erwaehnung` nicht.
#[tokio::test]
async fn us18_sammel_erwaehnungen_wecken_nicht() {
    let g = gegeben().await;
    let t0 = gesendet(&g, &g.anna, "allgemein", "vorher").await;
    let offen = aufruf(&g, &g.codex, "bis=3&erwaehnung", &t0);
    tokio::time::sleep(Duration::from_millis(300)).await;

    gesendet(&g, &g.anna, "allgemein", "@alle bitte lesen").await;
    gesendet(&g, &g.anna, "allgemein", "@hier kurz melden").await;
    let (antwort, dauer, _) = offen.await.expect("Aufruf");

    nicht_geweckt("US-18", &antwort, dauer);
}

/// US-19 [A3] und US-20: die Bearbeitung von `t1` zu `Hallo @codex` weckt den wartenden `codex` nicht; ein späteres
/// Nachholen mit `seit` vor `t1` liefert `t1` in der aktuellen Fassung als Treffer.
#[tokio::test]
async fn us19_us20_bearbeiten_weckt_nicht_nachholen_liest_die_aktuelle_fassung() {
    let g = gegeben().await;
    let t0 = gesendet(&g, &g.anna, "allgemein", "vorher").await;
    let t1 = gesendet(&g, &g.anna, "allgemein", "Hallo").await;
    let offen = aufruf(&g, &g.codex, "bis=3&erwaehnung", &t1);
    tokio::time::sleep(Duration::from_millis(300)).await;

    let bearbeitet = api_antwort(
        g.absender
            .anfrage(
                reqwest::Method::POST,
                format!(
                    "{}api/channels/allgemein/nachrichten/bearbeiten",
                    g.instanz.adresse
                ),
            )
            .header("X-LocalchatA-Sitzung", &g.anna)
            .header("Content-Type", "application/json")
            .body(json!({"zeit": t1, "text": "Hallo @codex"}).to_string()),
    )
    .await;
    assert!(
        (200..300).contains(&bearbeitet.status),
        "{}",
        bearbeitet.json
    );
    let (antwort, dauer, _) = offen.await.expect("Aufruf");
    nicht_geweckt("US-19", &antwort, dauer);

    let (nachgeholt, _, _) = aufruf(&g, &g.codex, "bis=0&erwaehnung", &t0)
        .await
        .expect("Nachholen");
    let nachrichten = nachgeholt.json["nachrichten"]
        .as_array()
        .expect("nachrichten");
    assert_eq!(nachrichten.len(), 1, "{}", nachgeholt.json);
    assert_eq!(nachrichten[0]["nachricht"]["zeit"], json!(t1));
    assert_eq!(nachrichten[0]["nachricht"]["text"], "Hallo @codex");
    assert_eq!(nachrichten[0]["nachricht"]["zustand"], "bearbeitet");
    assert_eq!(nachrichten[0]["anlass"], json!(["erwaehnung"]));
}

fn zeit(text: &str) -> Zeit {
    Zeit::parse_from_rfc3339(text).expect("Zeit")
}

fn ortszeit(zeitpunkt: chrono::DateTime<chrono::Utc>) -> Zeit {
    zeitpunkt.with_timezone(&chrono::Local).fixed_offset()
}

/// `anzahl` erzeugte Nachrichten ohne Erwähnung in `# allgemein` vor `JETZT`; liefert ihre Zeiten aus der Datei.
fn ohne_erwaehnung_erzeugen(ordner: &Path, anzahl: usize) -> Vec<Zeit> {
    verlauf_erzeugen(ordner, "allgemein", anzahl, 1, zeit(JETZT), ortszeit).expect("erzeugt");
    let mut zeiten = Vec::new();
    for eintrag in std::fs::read_dir(ordner.join("channels/allgemein")).expect("Channel") {
        let inhalt = std::fs::read_to_string(eintrag.expect("Eintrag").path()).expect("Datei");
        assert!(!inhalt.contains("@codex"), "erzeugt mit Erwähnung");
        zeiten.extend(
            inhalt
                .lines()
                .filter_map(|zeile| zeile.strip_prefix("@ "))
                .map(|kopf| zeit(kopf.split(' ').next().expect("Zeit im Kopf"))),
        );
    }
    zeiten.sort();
    assert_eq!(zeiten.len(), anzahl);
    zeiten
}

/// Ruft `bis=0&erwaehnung` als `codex`, zuerst ab vor der ersten erzeugten Nachricht, dann mit dem zurückgegebenen
/// `seit`; liefert je Aufruf `(ende, seit)` und prüft, dass keiner etwas liefert.
async fn dreimal_nachholen(g: &Gegeben, erste: Zeit) -> Vec<(String, Zeit)> {
    let mut seit = (erste - chrono::Duration::milliseconds(1)).to_rfc3339();
    let mut folge = Vec::new();
    for _ in 0..3 {
        let (antwort, _, _) = aufruf(g, &g.codex, "bis=0&erwaehnung", &seit)
            .await
            .expect("Aufruf");
        assert_eq!(antwort.status, 200, "{}", antwort.json);
        assert_eq!(antwort.json["nachrichten"], json!([]), "{}", antwort.json);
        seit = antwort.json["seit"].as_str().expect("seit").to_string();
        folge.push((
            antwort.json["ende"].as_str().expect("ende").to_string(),
            zeit(&seit),
        ));
    }
    folge
}

/// US-21 [K1]: 5.000 Nachrichten ohne Erwähnung → `weitere` bei der 2.000., `weitere` bei der 4.000., dann `frist`
/// mit `seit` ≥ der 5.000. — lückenlos, weil jeder Aufruf genau am zurückgegebenen `seit` ansetzt.
#[tokio::test]
async fn us21_die_pruefung_ist_auf_2000_saetze_begrenzt() {
    let mut zeiten = Vec::new();
    let g = gegeben_mit(|ordner| zeiten = ohne_erwaehnung_erzeugen(ordner, 5_000)).await;

    let folge = dreimal_nachholen(&g, zeiten[0]).await;

    assert_eq!(folge[0], ("weitere".to_string(), zeiten[1_999]));
    assert_eq!(folge[1], ("weitere".to_string(), zeiten[3_999]));
    assert_eq!(folge[2].0, "frist");
    assert!(folge[2].1 >= zeiten[4_999], "{folge:?}");
}

/// US-21 Grenzfall (Auslegung des planer, bestätigt in F0170 N3): genau 4.000 Nachrichten → der zweite Aufruf
/// `frist`, weil nach ihm kein ungeprüfter Satz bleibt.
#[tokio::test]
async fn us21_grenzfall_genau_4000_saetze() {
    let mut zeiten = Vec::new();
    let g = gegeben_mit(|ordner| zeiten = ohne_erwaehnung_erzeugen(ordner, 4_000)).await;

    let folge = dreimal_nachholen(&g, zeiten[0]).await;

    assert_eq!(folge[0], ("weitere".to_string(), zeiten[1_999]));
    assert_eq!(folge[1].0, "frist");
    assert!(folge[1].1 >= zeiten[3_999], "{folge:?}");
}
