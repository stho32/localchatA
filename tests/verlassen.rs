//! F0019 Verlassen über die API — über das echte Programm (R00005, US-15 bis US-17; ADR-0018).
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::strom::{Strom, ist_wechsel};
use gemeinsam::{Absender, ApiAntwort, Instanz, wegwerf_starten};
use serde_json::{Value, json};
use std::time::Duration;
use tokio::time::Instant;

const NULL_TOKEN: &str = "00000000000000000000000000000000";

async fn beitreten(absender: &Absender, instanz: &Instanz, name: &str) -> ApiAntwort {
    absender
        .beitreten(&instanz.adresse, json!({ "name": name }), None)
        .await
}

async fn token(absender: &Absender, instanz: &Instanz, name: &str) -> String {
    let antwort = beitreten(absender, instanz, name).await;
    assert_eq!(antwort.status, 201, "{}", antwort.json);
    antwort.json["token"].as_str().expect("Token").to_string()
}

/// `POST /api/verlassen` von `absender`, mit dem Sitzungs-Header (wenn gegeben) und weiteren Kopfzeilen.
async fn verlassen(
    absender: &Absender,
    instanz: &Instanz,
    sitzung: Option<&str>,
    kopf: &[(&str, &str)],
) -> ApiAntwort {
    let mut anfrage = absender.anfrage(
        reqwest::Method::POST,
        format!("{}api/verlassen", instanz.adresse),
    );
    if let Some(sitzung) = sitzung {
        anfrage = anfrage.header("X-LocalchatA-Sitzung", sitzung);
    }
    for (name, wert) in kopf {
        anfrage = anfrage.header(*name, *wert);
    }
    gemeinsam::api_antwort(anfrage).await
}

async fn kennungen(absender: &Absender, instanz: &Instanz, token: &str) -> Vec<Value> {
    let antwort = gemeinsam::api_antwort(
        absender
            .anfrage(
                reqwest::Method::GET,
                format!("{}api/teilnehmer", instanz.adresse),
            )
            .header("X-LocalchatA-Sitzung", token),
    )
    .await;
    assert_eq!(antwort.status, 200, "{}", antwort.json);
    antwort.json["liste"]
        .as_array()
        .expect("liste")
        .iter()
        .map(|zeile| zeile["kennung"].clone())
        .collect()
}

fn kopfwert<'a>(antwort: &'a ApiAntwort, name: &str) -> Option<&'a str> {
    antwort.kopf.get(name).and_then(|wert| wert.to_str().ok())
}

/// US-15: `alpha` hält einen Strom, `beta` liest und hat `alpha` online gelesen; `alpha` verlässt → 204 mit
/// Header-Satz und `no-store`; `beta` liest `weg` spätestens 1 s nach der 204, ohne `offline` davor; ein
/// Beitritt `alpha` von `127.0.0.4` direkt danach ergibt 201.
#[tokio::test]
async fn us15_verlassen_gibt_den_namen_sofort_frei() {
    let instanz = wegwerf_starten();
    let (zwei, drei, vier) = (
        Absender::von("127.0.0.2"),
        Absender::von("127.0.0.3"),
        Absender::von("127.0.0.4"),
    );
    let beta = token(&drei, &instanz, "beta").await;
    let mut strom_beta = Strom::oeffnen(&drei, &instanz.adresse, &beta).await;
    let alpha = token(&zwei, &instanz, "alpha").await;
    let _strom_alpha = Strom::oeffnen(&zwei, &instanz.adresse, &alpha).await;
    strom_beta
        .bis(
            Duration::from_secs(5),
            ist_wechsel("alpha@127.0.0.2", "online"),
        )
        .await
        .expect("beta hat alpha online gelesen");

    let antwort = verlassen(&zwei, &instanz, Some(&alpha), &[]).await;
    let erhalten = Instant::now();
    let neu = beitreten(&vier, &instanz, "alpha").await;
    let weg = strom_beta
        .bis(
            Duration::from_secs(5),
            ist_wechsel("alpha@127.0.0.2", "weg"),
        )
        .await
        .expect("beta liest alpha weg");

    assert_eq!(antwort.status, 204, "{}", antwort.json);
    assert_eq!(kopfwert(&antwort, "cache-control"), Some("no-store"));
    assert_eq!(
        kopfwert(&antwort, "x-content-type-options"),
        Some("nosniff")
    );
    assert!(kopfwert(&antwort, "content-security-policy").is_some());
    assert_eq!(neu.status, 201, "{}", neu.json);
    let zustellung = weg.ankunft.saturating_duration_since(erhalten);
    eprintln!("messung us15_weg_nach_204_ms={}", zustellung.as_millis());
    assert!(zustellung <= Duration::from_secs(1), "{zustellung:?}");
    let alpha_wechsel: Vec<String> = strom_beta
        .wechsel_von("alpha@127.0.0.2")
        .iter()
        .map(|e| {
            e.data["teilnehmer"]["status"]
                .as_str()
                .unwrap_or_default()
                .to_string()
        })
        .collect();
    assert_eq!(alpha_wechsel, ["online", "weg"], "ohne offline davor");
}

/// US-16: ohne Header → 401 `{fehler}`; mit dem Header von `alpha` und fremdem `Origin` → 403; `alpha` lebt.
#[tokio::test]
async fn us16_ohne_header_oder_von_fremder_seite_kein_verlassen() {
    let instanz = wegwerf_starten();
    let (zwei, drei) = (Absender::von("127.0.0.2"), Absender::von("127.0.0.3"));
    let alpha = token(&zwei, &instanz, "alpha").await;
    let beta = token(&drei, &instanz, "beta").await;

    let ohne = verlassen(&zwei, &instanz, None, &[]).await;
    let fremd = verlassen(
        &zwei,
        &instanz,
        Some(&alpha),
        &[("Origin", "http://evil.example")],
    )
    .await;

    assert_eq!(ohne.status, 401);
    assert!(ohne.json["fehler"].is_string(), "{}", ohne.json);
    assert_eq!(fremd.status, 403);
    assert!(
        kennungen(&drei, &instanz, &beta)
            .await
            .contains(&json!("alpha@127.0.0.2"))
    );
}

/// US-17: das Token von `alpha` von `127.0.0.3` und ein unbekanntes Token → je 204 ohne Wirkung; `alpha` lebt,
/// ein Beitritt `alpha` von `127.0.0.4` ergibt 409.
#[tokio::test]
async fn us17_fremde_und_unbekannte_tokens_bewirken_nichts_und_verraten_nichts() {
    let instanz = wegwerf_starten();
    let (zwei, drei, vier) = (
        Absender::von("127.0.0.2"),
        Absender::von("127.0.0.3"),
        Absender::von("127.0.0.4"),
    );
    let alpha = token(&zwei, &instanz, "alpha").await;
    let beta = token(&drei, &instanz, "beta").await;

    let fremd = verlassen(&drei, &instanz, Some(&alpha), &[]).await;
    let unbekannt = verlassen(&zwei, &instanz, Some(NULL_TOKEN), &[]).await;
    let unlesbar = verlassen(&zwei, &instanz, Some("kein-token"), &[]).await;
    let zweiter = beitreten(&vier, &instanz, "alpha").await;

    assert_eq!(
        (fremd.status, unbekannt.status, unlesbar.status),
        (204, 204, 204)
    );
    assert!(
        kennungen(&drei, &instanz, &beta)
            .await
            .contains(&json!("alpha@127.0.0.2"))
    );
    assert_eq!(zweiter.status, 409, "{}", zweiter.json);
}

/// Kriterium „Ohne Wirkung“: nach einem erfolgreichen `verlassen` ein zweites mit demselben Token → 204, kein
/// weiterer Wechsel.
#[tokio::test]
async fn ein_zweites_verlassen_ist_204_ohne_weiteren_wechsel() {
    let instanz = wegwerf_starten();
    let (zwei, drei) = (Absender::von("127.0.0.2"), Absender::von("127.0.0.3"));
    let beta = token(&drei, &instanz, "beta").await;
    let mut strom_beta = Strom::oeffnen(&drei, &instanz.adresse, &beta).await;
    let alpha = token(&zwei, &instanz, "alpha").await;

    let erstes = verlassen(&zwei, &instanz, Some(&alpha), &[]).await;
    strom_beta
        .bis(
            Duration::from_secs(5),
            ist_wechsel("alpha@127.0.0.2", "weg"),
        )
        .await
        .expect("weg");
    let staende_davor = strom_beta.staende();
    let zweites = verlassen(&zwei, &instanz, Some(&alpha), &[]).await;
    strom_beta.mitlesen(Duration::from_millis(1500)).await;

    assert_eq!((erstes.status, zweites.status), (204, 204));
    let neu: Vec<u64> = strom_beta.staende()[staende_davor.len()..].to_vec();
    assert_eq!(neu, Vec::<u64>::new(), "kein weiterer Wechsel");
}

/// US-15 (Kriterium „Verlassen“, eigener Strom): `alpha`s Strom liefert `weg` für `alpha` und endet spätestens
/// 1 s nach der 204; ab seinem Ende liest `beta` für `alpha` 2 s lang keinen weiteren Wechsel (ein
/// fälschliches `offline` käme nach Nachlauf 1 s + Takt 0,25 s).
#[tokio::test]
async fn us15_der_eigene_strom_liefert_weg_und_endet() {
    let instanz = wegwerf_starten();
    let (zwei, drei) = (Absender::von("127.0.0.2"), Absender::von("127.0.0.3"));
    let beta = token(&drei, &instanz, "beta").await;
    let mut strom_beta = Strom::oeffnen(&drei, &instanz.adresse, &beta).await;
    let alpha = token(&zwei, &instanz, "alpha").await;
    let mut strom_alpha = Strom::oeffnen(&zwei, &instanz.adresse, &alpha).await;
    strom_beta
        .bis(
            Duration::from_secs(5),
            ist_wechsel("alpha@127.0.0.2", "online"),
        )
        .await
        .expect("beta hat alpha online gelesen");

    let antwort = verlassen(&zwei, &instanz, Some(&alpha), &[]).await;
    let erhalten = Instant::now();
    let endete = strom_alpha.mitlesen(Duration::from_secs(5)).await;
    let ende = Instant::now();
    strom_beta.mitlesen(Duration::from_secs(2)).await;

    assert_eq!(antwort.status, 204);
    assert!(endete, "alphas Strom endet");
    let bis_zum_ende = ende.saturating_duration_since(erhalten);
    eprintln!(
        "messung us15_eigener_strom_endet_ms={}",
        bis_zum_ende.as_millis()
    );
    assert!(bis_zum_ende <= Duration::from_secs(1), "{bis_zum_ende:?}");
    let eigene: Vec<String> = strom_alpha
        .wechsel_von("alpha@127.0.0.2")
        .iter()
        .map(|e| {
            e.data["teilnehmer"]["status"]
                .as_str()
                .unwrap_or_default()
                .to_string()
        })
        .collect();
    assert_eq!(
        eigene.last().map(String::as_str),
        Some("weg"),
        "{:?}",
        strom_alpha.ereignisse
    );
    let bei_beta: Vec<String> = strom_beta
        .wechsel_von("alpha@127.0.0.2")
        .iter()
        .map(|e| {
            e.data["teilnehmer"]["status"]
                .as_str()
                .unwrap_or_default()
                .to_string()
        })
        .collect();
    assert_eq!(bei_beta, ["online", "weg"], "nichts nach weg");
}

/// Gnadenfrist der Proben zur Nacharbeit 1 — klein, aber deutlich länger als 200 Zyklen dauern.
const KURZE_GNADENFRIST: Duration = Duration::from_secs(10);

/// Was eine Schleife aus `beitreten("f<i>")` → bei 201 sofort `verlassen` ergab.
#[derive(Debug, Default)]
struct Zyklen {
    erfolge: usize,
    abgewiesen: usize,
    anderes: Vec<String>,
}

/// `anzahl` Zyklen über `verbindungen` parallele Clients von den Absendern `absender` (reihum), so schnell wie
/// möglich; jeder Client hält seine Keep-alive-Verbindung.
async fn zyklen(port: u16, absender: &[&str], verbindungen: usize, anzahl: usize) -> Zyklen {
    let mut aufgaben = tokio::task::JoinSet::new();
    for nummer in 0..verbindungen {
        let von = absender[nummer % absender.len()].to_string();
        let basis = basis_fuer(&von, port);
        let eigene: Vec<usize> = (0..anzahl).filter(|i| i % verbindungen == nummer).collect();
        aufgaben.spawn(async move {
            let client = Absender::von(&von);
            let mut ergebnis = Zyklen::default();
            for i in eigene {
                let antwort = client
                    .beitreten(&basis, json!({ "name": format!("f{i}") }), None)
                    .await;
                match antwort.status {
                    201 => {
                        ergebnis.erfolge += 1;
                        let token = antwort.json["token"].as_str().expect("Token").to_string();
                        let verlassen = gemeinsam::api_antwort(
                            client
                                .anfrage(reqwest::Method::POST, format!("{basis}api/verlassen"))
                                .header("X-LocalchatA-Sitzung", token),
                        )
                        .await;
                        assert_eq!(verlassen.status, 204);
                    }
                    429 if antwort.json["fehler"].is_string() => ergebnis.abgewiesen += 1,
                    _ => ergebnis
                        .anderes
                        .push(format!("{} {}", antwort.status, antwort.json)),
                }
            }
            ergebnis
        });
    }
    let mut gesamt = Zyklen::default();
    while let Some(teil) = aufgaben.join_next().await {
        let teil = teil.expect("Aufgabe");
        gesamt.erfolge += teil.erfolge;
        gesamt.abgewiesen += teil.abgewiesen;
        gesamt.anderes.extend(teil.anderes);
    }
    gesamt
}

/// Die Adresse der Instanz in der Familie des Absenders: IPv4 an 127.0.0.1, die gemappte Form an
/// `::ffff:127.0.0.1` (die Instanz lauscht dafür auf `::`).
fn basis_fuer(absender: &str, port: u16) -> String {
    if absender.contains(':') {
        return format!("http://[::ffff:127.0.0.1]:{port}/");
    }
    format!("http://127.0.0.1:{port}/")
}

/// Abnahme-Probe Nacharbeit 1 (Security, CWE-770, ADR-0018 Regel 2a): `beta` (127.0.0.3) liest den Strom; von
/// `absender` laufen 200 Zyklen `beitreten` → `verlassen` über `verbindungen` Verbindungen → innerhalb der
/// Gnadenfrist höchstens 64 × 201, der Rest 429 mit `{fehler}`; `beta` liest höchstens 2 × 64 Wechsel mit
/// Herkunft 127.0.0.2. Kontrollpaar danach: 127.0.0.4 tritt mit 201 bei; `alpha` von 127.0.0.5 → verlassen →
/// `alpha` von 127.0.0.6 sofort 201.
async fn schleife_ist_begrenzt(host: &str, absender: &[&str], verbindungen: usize) {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let frist = KURZE_GNADENFRIST.as_millis().to_string();
    let instanz = gemeinsam::starten_mit(
        wurzel.path(),
        &["--port", "0", "--host", host, "--kein-browser"],
        &[("LOCALCHATA_GNADENFRIST_MS", &frist)],
    );
    let basis = basis_fuer("127.0.0.3", instanz.port());
    let drei = Absender::von("127.0.0.3");
    let beta = drei
        .beitreten(&basis, json!({ "name": "beta" }), None)
        .await
        .json["token"]
        .as_str()
        .expect("Token")
        .to_string();
    let mut strom_beta = Strom::oeffnen(&drei, &basis, &beta).await;

    let anfang = Instant::now();
    let ergebnis = zyklen(instanz.port(), absender, verbindungen, 200).await;
    let dauer = anfang.elapsed();
    let kontrolle = Absender::von("127.0.0.4")
        .beitreten(&basis, json!({ "name": "kontrolle" }), None)
        .await;
    let (fuenf, sechs) = (Absender::von("127.0.0.5"), Absender::von("127.0.0.6"));
    let alpha = fuenf
        .beitreten(&basis, json!({ "name": "alpha" }), None)
        .await;
    let token = alpha.json["token"].as_str().expect("Token").to_string();
    let verlassen_alpha = gemeinsam::api_antwort(
        fuenf
            .anfrage(reqwest::Method::POST, format!("{basis}api/verlassen"))
            .header("X-LocalchatA-Sitzung", token),
    )
    .await;
    let alpha_neu = sechs
        .beitreten(&basis, json!({ "name": "alpha" }), None)
        .await;
    strom_beta.mitlesen(Duration::from_millis(500)).await;

    eprintln!(
        "messung schleife_ms={} verbindungen={verbindungen}",
        dauer.as_millis()
    );
    assert!(
        dauer < KURZE_GNADENFRIST,
        "Schleife innerhalb der Gnadenfrist: {dauer:?}"
    );
    assert!(ergebnis.erfolge <= 64, "{ergebnis:?}");
    assert_eq!(ergebnis.erfolge + ergebnis.abgewiesen, 200, "{ergebnis:?}");
    let von_zwei = strom_beta
        .ereignisse
        .iter()
        .filter(|e| {
            e.data["art"] == "anwesenheit" && e.data["teilnehmer"]["herkunft"] == "127.0.0.2"
        })
        .count();
    assert!(von_zwei <= 2 * 64, "{von_zwei} Wechsel von 127.0.0.2");
    assert!(ergebnis.erfolge >= 1, "{ergebnis:?}");
    assert_eq!(
        von_zwei,
        2 * ergebnis.erfolge,
        "jeder Erfolg zählt zur Herkunft 127.0.0.2 (online und weg), auch aus der gemappten Form"
    );
    assert_eq!(kontrolle.status, 201, "{}", kontrolle.json);
    assert_eq!((alpha.status, verlassen_alpha.status), (201, 204));
    assert_eq!(
        alpha_neu.status, 201,
        "Name sofort frei: {}",
        alpha_neu.json
    );
}

#[tokio::test]
async fn nacharbeit1_beitreten_und_verlassen_in_schleife_ueber_eine_keepalive_verbindung_begrenzt()
{
    schleife_ist_begrenzt("127.0.0.1", &["127.0.0.2"], 1).await;
}

#[tokio::test]
async fn nacharbeit1_beitreten_und_verlassen_in_schleife_ueber_vier_verbindungen_begrenzt() {
    schleife_ist_begrenzt("127.0.0.1", &["127.0.0.2"], 4).await;
}

/// (b) `::ffff:127.0.0.2` zählt zur selben Herkunft wie 127.0.0.2: reihum von beiden, zusammen höchstens 64.
#[tokio::test]
async fn nacharbeit1_die_gemappte_ipv6_form_zaehlt_zur_selben_herkunft() {
    schleife_ist_begrenzt("::", &["127.0.0.2", "::ffff:127.0.0.2"], 4).await;
}
