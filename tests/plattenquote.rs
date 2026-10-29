//! R00023 Plattenquote — über das echte Programm (F0074 Quote einstellen, F0075 Senden an der Quote, F0076 Quote im
//! Betriebsprotokoll).

mod gemeinsam;

use gemeinsam::protokoll::alle_zeilen;
use gemeinsam::{Instanz, aufrufen, aufrufen_mit, starten_mit};
use std::path::Path;

const LOKAL: [&str; 5] = ["--port", "0", "--host", "127.0.0.1", "--kein-browser"];
const FALSCH: &str = "--plattenquote braucht eine Größe wie 4GiB oder 512MiB, oder aus";

fn starten_um(ordner: &Path, weitere_argumente: &[&str], umgebung: &[(&str, &str)]) -> Instanz {
    let mut argumente = LOKAL.to_vec();
    argumente.extend_from_slice(weitere_argumente);
    let mut alle = vec![
        ("LOCALCHATA_JETZT", "2026-10-28T12:00:00+01:00"),
        ("TZ", "Europe/Berlin"),
    ];
    alle.extend_from_slice(umgebung);
    starten_mit(ordner, &argumente, &alle)
}

/// Ruft mit dem Ordner und den lokalen Optionen auf, ohne auf eine Startzeile zu warten.
fn aufrufen_in(
    ordner: &Path,
    weitere_argumente: &[&str],
    umgebung: &[(&str, &str)],
) -> gemeinsam::Ende {
    let ordner = ordner.display().to_string();
    let mut argumente = vec![ordner.as_str()];
    argumente.extend_from_slice(&LOKAL);
    argumente.extend_from_slice(weitere_argumente);
    aufrufen_mit(&argumente, umgebung)
}

/// Der eine Start-Eintrag `INFORMATION start Instanz bereit`.
fn start_eintrag(ordner: &Path) -> String {
    let start: Vec<String> = alle_zeilen(ordner)
        .into_iter()
        .filter(|zeile| zeile.contains(" INFORMATION start Instanz bereit"))
        .collect();
    assert_eq!(start.len(), 1, "{start:#?}");
    start[0].clone()
}

/// F0074 „Falscher Wert beim Start“: `LOCALCHATA_PLATTENQUOTE=4` → Exit 2 mit Beispiel, kein `logs/`; dazu die
/// übrigen falschen Schreibweisen aus US-2 und ADR-0044 Regel 3.
#[test]
fn eine_falsche_quote_in_der_umgebung_ist_exit_2_ohne_logs() {
    for falsch in ["4", "4GB", "0MiB", "1025GiB", "-1GiB", ""] {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");

        let ende = aufrufen_in(wurzel.path(), &[], &[("LOCALCHATA_PLATTENQUOTE", falsch)]);

        assert_eq!(ende.exit, Some(2), "{falsch:?}: {}", ende.stderr);
        assert!(ende.stderr.contains(FALSCH), "{falsch:?}: {}", ende.stderr);
        assert!(!wurzel.path().join("logs").exists(), "{falsch:?}");
    }
}

#[test]
fn eine_falsche_quote_als_option_ist_exit_2_ohne_logs() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");

    let ende = aufrufen_in(wurzel.path(), &["--plattenquote", "4GB"], &[]);

    assert_eq!(ende.exit, Some(2), "{}", ende.stderr);
    assert!(ende.stderr.contains(FALSCH), "{}", ende.stderr);
    assert!(!wurzel.path().join("logs").exists());
}

/// `--plattenquote` als letztes Argument → Exit 2 mit „braucht einen Wert“.
#[test]
fn die_option_ohne_wert_ist_exit_2() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");

    let ende = aufrufen_in(wurzel.path(), &["--plattenquote"], &[]);

    assert_eq!(ende.exit, Some(2), "{}", ende.stderr);
    assert!(
        ende.stderr
            .contains("--plattenquote braucht einen Wert — Aufruf siehe localchatA --hilfe"),
        "{}",
        ende.stderr
    );
    assert!(!wurzel.path().join("logs").exists());
}

/// ADR-0044 Ergänzung Karte #26: eine gültige Option übersteuert eine falsche Umgebung — sie wird nicht gelesen.
/// (R00023 F0074 nennt hier noch Exit 2; die spätere Ergänzung der ADR geht vor, siehe Bericht Karte #34.)
#[test]
fn eine_gueltige_option_uebersteuert_eine_falsche_umgebung() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");

    let _instanz = starten_um(
        wurzel.path(),
        &["--plattenquote", "2MiB"],
        &[("LOCALCHATA_PLATTENQUOTE", "4")],
    );

    let start = start_eintrag(wurzel.path());
    assert!(
        start.contains(" plattenquote=2097152 belegt=0 plattenquote_option=--plattenquote "),
        "{start}"
    );
}

/// F0074 „Hilfe“: Exit 0, der Text zu `--plattenquote` wörtlich, umbrochen wie die übrigen Optionen.
#[test]
fn die_hilfe_nennt_die_plattenquote() {
    let ende = aufrufen(&["--hilfe"]);

    assert_eq!(ende.exit, Some(0), "{}", ende.stderr);
    assert!(
        ende.stdout.contains("  --plattenquote <größe>\n                    Höchstgröße aller Gespräche (Tageslogs der Channels und Direktgespräche)\n                    wie 4GiB oder 512MiB, aus für keine Grenze; Standard 4GiB; auch über LOCALCHATA_PLATTENQUOTE\n"),
        "{}",
        ende.stdout
    );
}

/// F0074 „Wirksame Quote sichtbar“ (Start-Eintrag): Option, Umgebung, Standard mit Herkunft, leerer Ordner →
/// `belegt=0`; die Mutanten „Umgebung schlägt Option“ und „Herkunft immer `standard`“ werden rot. Geprüft wird die
/// Folge `plattenquote= belegt= plattenquote_option=`; jeder Schlüssel steht nur einmal in der Zeile (Hinweis
/// Entwicklertest #26: `option=` gehört der Protokollstufe).
#[test]
fn der_start_eintrag_nennt_quote_und_herkunft() {
    for (argumente, umgebung, erwartet) in [
        (
            &["--plattenquote", "512MiB"][..],
            &[("LOCALCHATA_PLATTENQUOTE", "1GiB")][..],
            " plattenquote=536870912 ",
        ),
        (
            &[][..],
            &[("LOCALCHATA_PLATTENQUOTE", "aus")][..],
            " plattenquote=aus ",
        ),
        (&[][..], &[][..], " plattenquote=4294967296 "),
    ] {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let _instanz = starten_um(wurzel.path(), argumente, umgebung);

        let start = start_eintrag(wurzel.path());
        let herkunft = match (argumente.is_empty(), umgebung.is_empty()) {
            (false, _) => "--plattenquote",
            (true, false) => "LOCALCHATA_PLATTENQUOTE",
            (true, true) => "standard",
        };
        assert!(
            start.contains(&format!(
                "{erwartet}belegt=0 plattenquote_option={herkunft} "
            )),
            "{herkunft}: {start}"
        );
        assert_eq!(start.matches(" option=").count(), 1, "{start}");
    }
}

/// F0074 „Wirksame Quote sichtbar“ (Health-Check): `512MiB` → 536870912, `aus` → null, ohne Angabe 4 GiB.
#[tokio::test]
async fn die_gesundheit_nennt_die_wirksame_quote() {
    for (argumente, umgebung, erwartet) in [
        (
            &["--plattenquote", "512MiB"][..],
            &[][..],
            serde_json::json!(536_870_912_u64),
        ),
        (
            &[][..],
            &[("LOCALCHATA_PLATTENQUOTE", "aus")][..],
            serde_json::Value::Null,
        ),
        (&[][..], &[][..], serde_json::json!(4_294_967_296_u64)),
    ] {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let instanz = starten_um(wurzel.path(), argumente, umgebung);

        let (status, gesundheit) = gemeinsam::gesundheit(&instanz.adresse).await;

        assert_eq!(status, 200);
        assert_eq!(gesundheit["plattenquote"], erwartet, "{gesundheit}");
        assert!(gesundheit.get("plattenquote").is_some(), "{gesundheit}");
    }
}

// ---- F0075 Senden an der Quote (B0359) ----

/// Uhr der Quote-Fälle; die Tagesdatei von gestern ist der 27. Oktober.
const JETZT: &str = "2026-10-28T12:00:00.000+01:00";
const GESTERN: &str = "2026-10-27";
/// Der Quote-Ordner: 300 Bytes unter 1 MiB.
const GELADEN: u64 = 1_048_576 - 300;
/// Text zu 17 Zeichen: Satz von `beta@127.0.0.1` (Agent) zu 81 Bytes — Kopf 47, `+ art: agent` 13, Text mit
/// Einrückung und Abschluss 21. R00023 rechnet 81 Bytes für 30 Zeichen ohne die Metazeile; die Zahlen der
/// Kriterien (1.048.519, 243, 1.048.600) gelten mit diesem Text unverändert (Bericht Karte #34).
const TEXT: &str = "Quote-Probe 17 Z.";
const SATZ: u64 = 81;
const TEXT_507: &str = "Nicht gesendet: die Gespräche belegen 1,0 MiB, die Quote dieser Instanz ist 1,0 MiB. Platz entsteht, wenn alte Tage nach ihrer Aufbewahrungsfrist entfallen (beim nächsten Tageswechsel) oder eine Frist verkürzt wird; die Quote stellt der Betreiber beim Start mit --plattenquote ein.";

fn berlin(zeitpunkt: chrono::DateTime<chrono::Utc>) -> localchata::uhr::Zeit {
    zeitpunkt.with_timezone(&chrono::FixedOffset::east_opt(3600).expect("Versatz"))
}

/// Der Quote-Ordner über die Testhilfe aus B0359.
fn quote_ordner(bytes: u64) -> tempfile::TempDir {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let jetzt = localchata::uhr::Zeit::parse_from_rfc3339(JETZT).expect("jetzt");
    localchata::testdaten::quote_ordner(wurzel.path(), "allgemein", bytes, jetzt, berlin)
        .expect("Quote-Ordner");
    wurzel
}

fn quote_starten(ordner: &Path, argumente: &[&str], umgebung: &[(&str, &str)]) -> Instanz {
    let mut alle = vec![("LOCALCHATA_JETZT", JETZT), ("TZ", "Europe/Berlin")];
    alle.extend_from_slice(umgebung);
    let mut mit = LOKAL.to_vec();
    mit.extend_from_slice(argumente);
    starten_mit(ordner, &mit, &alle)
}

fn heute_datei(ordner: &Path) -> Vec<u8> {
    std::fs::read(ordner.join("channels/allgemein/2026-10-28.log")).unwrap_or_default()
}

/// Testhilfe B0359: die Tagesdatei von gestern hat genau die verlangten Bytes, Frist 7, und lädt ohne Defekt.
#[tokio::test]
async fn der_quote_ordner_hat_genau_die_bytes_und_laedt_ohne_defekt() {
    for bytes in [60, 1_000, 1_025, 2_047, 4_096, GELADEN] {
        let wurzel = quote_ordner(bytes);
        let datei = wurzel
            .path()
            .join(format!("channels/allgemein/{GESTERN}.log"));
        assert_eq!(std::fs::metadata(&datei).expect("Datei").len(), bytes);
        assert_eq!(
            std::fs::read_to_string(wurzel.path().join("channels/allgemein/einstellungen.toml"))
                .expect("Einstellungen"),
            "frist_tage = 7\n"
        );
        let instanz = quote_starten(wurzel.path(), &["--plattenquote", "aus"], &[]);
        let (_, gesundheit) = gemeinsam::gesundheit(&instanz.adresse).await;
        assert_eq!(gesundheit["log_bytes"], bytes, "{gesundheit}");
        assert_eq!(gesundheit["defekte_zeilen"], 0, "{gesundheit}");
    }
}

/// F0075 „507 im echten Programm“ und „Platz über die Frist“ (US-3, US-5): drei 201, dann 507 mit Grund und Ausweg;
/// die Tagesdatei bleibt Byte für Byte, der Strom von `gamma` sieht drei Nachrichten; Health zählt; Verkürzen auf
/// 1 Tag (409, bestätigt 200) nimmt gestern, danach wieder 201. Mutante „Prüfung nach dem Anhängen“ wird rot.
#[tokio::test]
async fn an_der_quote_ist_senden_507_und_platz_entsteht_ueber_die_frist() {
    let wurzel = quote_ordner(GELADEN);
    let instanz = quote_starten(wurzel.path(), &["--plattenquote", "1MiB"], &[]);
    let absender = gemeinsam::Absender::von("127.0.0.1");
    let beta = gemeinsam::senden::beitreten(&absender, &instanz.adresse, "beta", None).await;
    let gamma = gemeinsam::senden::beitreten(&absender, &instanz.adresse, "gamma", None).await;
    let mut strom = gemeinsam::strom::Strom::oeffnen(&absender, &instanz.adresse, &gamma).await;

    for _ in 0..3 {
        let antwort =
            gemeinsam::senden::senden(&absender, &instanz.adresse, &beta, "allgemein", TEXT).await;
        assert_eq!(antwort.status, 201, "{}", antwort.json);
    }
    let vorher = heute_datei(wurzel.path());
    assert_eq!(vorher.len() as u64, 3 * SATZ);
    let abgewiesen =
        gemeinsam::senden::senden(&absender, &instanz.adresse, &beta, "allgemein", TEXT).await;

    assert_eq!(abgewiesen.status, 507, "{}", abgewiesen.json);
    assert!(abgewiesen.kopf.get("retry-after").is_none());
    assert_eq!(
        abgewiesen.json,
        serde_json::json!({
            "fehler": TEXT_507,
            "regel": "quote",
            "belegt": 1_048_519,
            "hoechstens": 1_048_576
        })
    );
    assert_eq!(heute_datei(wurzel.path()), vorher);
    strom.mitlesen(std::time::Duration::from_millis(500)).await;
    let nachrichten = strom
        .ereignisse
        .iter()
        .filter(|e| e.data["art"] == "nachricht")
        .count();
    assert_eq!(nachrichten, 3);
    let (_, gesundheit) = gemeinsam::gesundheit(&instanz.adresse).await;
    assert_eq!(gesundheit["plattenquote"], 1_048_576);
    assert_eq!(gesundheit["quote_abgewiesen"], 1);
    assert_eq!(gesundheit["log_bytes"], 1_048_519);

    let kurz = gemeinsam::frist::frist_setzen(
        &absender,
        &instanz,
        &beta,
        "allgemein",
        serde_json::json!({ "tage": 1 }),
    )
    .await;
    assert_eq!(kurz.status, 409, "{}", kurz.json);
    let bestaetigt = gemeinsam::frist::frist_setzen(
        &absender,
        &instanz,
        &beta,
        "allgemein",
        serde_json::json!({ "tage": 1, "bestaetigt": true }),
    )
    .await;
    assert_eq!(bestaetigt.status, 200, "{}", bestaetigt.json);
    assert!(!gemeinsam::frist::tag_da(
        wurzel.path(),
        "allgemein",
        GESTERN
    ));
    let (_, gesundheit) = gemeinsam::gesundheit(&instanz.adresse).await;
    assert_eq!(gesundheit["log_bytes"], 3 * SATZ);
    let wieder =
        gemeinsam::senden::senden(&absender, &instanz.adresse, &beta, "allgemein", TEXT).await;
    assert_eq!(wieder.status, 201, "{}", wieder.json);

    let zeilen = quote_zeilen(wurzel.path());
    assert_eq!(
        zeilen.first().map(String::as_str),
        Some("WARNING quote Gespräche belegen 90 % der Quote | belegt=1048276 hoechstens=1048576")
    );
    assert!(
        zeilen.contains(&format!(
            "INFORMATION quote Gespräche wieder unter 90 % der Quote | belegt={} hoechstens=1048576 grund=frist_gesetzt",
            3 * SATZ
        )),
        "{zeilen:#?}"
    );
    let alle = alle_zeilen(wurzel.path());
    let warnung = alle
        .iter()
        .position(|zeile| zeile.contains(" WARNING quote Gespräche belegen 90 % der Quote"))
        .expect("90-%-Zeile");
    let bereit = alle
        .iter()
        .position(|zeile| zeile.contains(" INFORMATION start Instanz bereit"))
        .expect("Start-Eintrag");
    assert!(
        warnung < bereit,
        "nach dem Laden, vor der ersten Anfrage: {alle:#?}"
    );
}

/// Die `quote`-Zeilen des Protokolls ab der Stufe, ohne Zeitstempel.
fn quote_zeilen(ordner: &Path) -> Vec<String> {
    alle_zeilen(ordner)
        .into_iter()
        .filter(|zeile| zeile.contains(" quote "))
        .map(|zeile| zeile.split_once(' ').expect("Zeitstempel").1.to_string())
        .collect()
}

/// F0075 „Was an der Quote weiter geht“ (US-6): Lesen 200, Channel anlegen 201, Frist verlängern 200 mit `delta`;
/// `beta` wiederholt mit dem Schlüssel einer angenommenen Nachricht → 200 mit der ursprünglichen, ohne Zählung.
#[tokio::test]
async fn an_der_quote_geht_alles_andere_weiter() {
    let wurzel = quote_ordner(GELADEN);
    let instanz = quote_starten(wurzel.path(), &["--plattenquote", "1MiB"], &[]);
    let absender = gemeinsam::Absender::von("127.0.0.1");
    let beta = gemeinsam::senden::beitreten(&absender, &instanz.adresse, "beta", None).await;
    let delta = gemeinsam::senden::beitreten(&absender, &instanz.adresse, "delta", None).await;
    let mit_schluessel = |schluessel: &'static str| {
        absender
            .anfrage(
                reqwest::Method::POST,
                format!("{}api/channels/allgemein/nachrichten", instanz.adresse),
            )
            .header("X-LocalchatA-Sitzung", beta.clone())
            .header("Idempotency-Key", schluessel)
            .json(&serde_json::json!({ "text": TEXT }))
    };
    let mut angenommen = Vec::new();
    for schluessel in ["k1", "k2", "k3"] {
        let antwort = gemeinsam::api_antwort(mit_schluessel(schluessel)).await;
        assert_eq!(antwort.status, 201, "{}", antwort.json);
        angenommen.push(antwort.json["nachricht"].clone());
    }
    let voll = gemeinsam::api_antwort(mit_schluessel("k4")).await;
    assert_eq!(voll.status, 507, "{}", voll.json);

    gemeinsam::senden::verlauf(&absender, &instanz.adresse, &delta, "allgemein").await;
    let neu = absender
        .channel_anlegen(
            &instanz.adresse,
            &delta,
            serde_json::json!({ "name": "neu" }),
        )
        .await;
    assert_eq!(neu.status, 201, "{}", neu.json);
    let laenger = gemeinsam::frist::frist_setzen(
        &absender,
        &instanz,
        &delta,
        "allgemein",
        serde_json::json!({ "tage": 30 }),
    )
    .await;
    assert_eq!(laenger.status, 200, "{}", laenger.json);
    let wiederholt = gemeinsam::api_antwort(mit_schluessel("k2")).await;
    assert_eq!(wiederholt.status, 200, "{}", wiederholt.json);
    assert_eq!(wiederholt.json["nachricht"], angenommen[1]);
    let (_, gesundheit) = gemeinsam::gesundheit(&instanz.adresse).await;
    assert_eq!(gesundheit["quote_abgewiesen"], 1);
}

/// F0075 „Ausgeschaltet“: mit `aus` viermal 201; Health `plattenquote: null`, `quote_abgewiesen: 0`, `log_bytes`
/// zählt weiter.
#[tokio::test]
async fn ohne_quote_wird_ueber_1_mib_angenommen() {
    let wurzel = quote_ordner(GELADEN);
    let instanz = quote_starten(wurzel.path(), &["--plattenquote", "aus"], &[]);
    let absender = gemeinsam::Absender::von("127.0.0.1");
    let beta = gemeinsam::senden::beitreten(&absender, &instanz.adresse, "beta", None).await;

    for _ in 0..4 {
        let antwort =
            gemeinsam::senden::senden(&absender, &instanz.adresse, &beta, "allgemein", TEXT).await;
        assert_eq!(antwort.status, 201, "{}", antwort.json);
    }

    let (_, gesundheit) = gemeinsam::gesundheit(&instanz.adresse).await;
    assert_eq!(gesundheit["plattenquote"], serde_json::Value::Null);
    assert_eq!(gesundheit["quote_abgewiesen"], 0);
    assert_eq!(gesundheit["log_bytes"], 1_048_600);
}
