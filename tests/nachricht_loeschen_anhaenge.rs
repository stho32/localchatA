//! R00040 F0137 — Anhänge einer gelöschten Nachricht mitlöschen (ADR-0048 Regel 9; ADR-0046 Ergänzung F1′ Regeln 5,
//! 8): das echte Programm mit `--plattenquote 10MiB --anhangquote 10MiB`; „Nachricht mit zwei Anhängen“: `beta` hat
//! `foto.png` (307.200 Bytes) und `bericht.pdf` (2.457.600 Bytes) hochgeladen und mit dem Text `Bericht` gesendet
//! (Zeit `t3`), `anhang_belegt` = 2.764.800.

mod gemeinsam;

use gemeinsam::anhaenge::{ablegen, gesundheit};
use gemeinsam::senden::{beitreten, ordner, roh_senden, senden, verlauf};
use gemeinsam::strom::{Empfangen, Strom};
use gemeinsam::{Absender, ApiAntwort, Instanz, api_antwort};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::time::Instant;

const JETZT: &str = "2026-10-28T12:00:00.000+01:00";
const HEUTE: &str = "2026-10-28";
const FOTO: u64 = 307_200;
const BERICHT: u64 = 2_457_600;
/// Zeitkriterium: Ereignis und freier Platz weniger als 1 s nach der 200.
const GRENZE: Duration = Duration::from_secs(1);

fn starten(ordner: &Path) -> Instanz {
    gemeinsam::starten_mit(
        ordner,
        &[
            "--port",
            "0",
            "--host",
            "127.0.0.1",
            "--kein-browser",
            "--plattenquote",
            "10MiB",
            "--anhangquote",
            "10MiB",
        ],
        &[
            ("TZ", "Europe/Berlin"),
            ("LOCALCHATA_JETZT", JETZT),
            ("LOCALCHATA_PROTOKOLL_STUFE", "debug"),
        ],
    )
}

/// Die Nachricht mit zwei Anhängen: Token von `beta`, `t3`, IDs von `foto.png` und `bericht.pdf`.
struct Bericht {
    token: String,
    t3: String,
    foto: String,
    bericht: String,
}

async fn bericht_senden(beta: &Absender, basis: &str) -> Bericht {
    let token = beitreten(beta, basis, "beta", Some("agent")).await;
    let foto = ablegen(beta, basis, &token, "foto.png", vec![7; FOTO as usize]).await;
    let bericht = ablegen(
        beta,
        basis,
        &token,
        "bericht.pdf",
        vec![8; BERICHT as usize],
    )
    .await;
    let gesendet = roh_senden(
        beta,
        basis,
        Some(&token),
        "allgemein",
        "application/json",
        json!({"text": "Bericht", "anhaenge": [foto, bericht]})
            .to_string()
            .into_bytes(),
    )
    .await;
    assert_eq!(gesendet.status, 201, "{}", gesendet.json);
    let t3 = gesendet.json["nachricht"]["zeit"]
        .as_str()
        .expect("zeit")
        .to_string();
    Bericht {
        token,
        t3,
        foto,
        bericht,
    }
}

async fn loeschen(absender: &Absender, basis: &str, token: &str, zeit: &str) -> ApiAntwort {
    api_antwort(
        absender
            .anfrage(
                reqwest::Method::POST,
                format!("{basis}api/channels/allgemein/nachrichten/loeschen"),
            )
            .header("X-LocalchatA-Sitzung", token)
            .json(&json!({ "zeit": zeit })),
    )
    .await
}

/// `GET <basis>anhaenge/<id>`: Status und Körper als Text.
async fn abrufen(absender: &Absender, basis: &str, id: &str) -> (u16, String) {
    let antwort = absender
        .anfrage(reqwest::Method::GET, format!("{basis}anhaenge/{id}"))
        .send()
        .await
        .expect("GET");
    let status = antwort.status().as_u16();
    (status, antwort.text().await.unwrap_or_default())
}

async fn anhang_belegt(basis: &str) -> u64 {
    gesundheit(basis).await["anhang_belegt"]
        .as_u64()
        .expect("anhang_belegt")
}

/// Die Datei des Anhangs mit diesem Namen unter `channels/allgemein/anhaenge/<heute>/`.
fn datei_von(ordner: &Path, name: &str) -> PathBuf {
    let tag = ordner.join(format!("channels/allgemein/anhaenge/{HEUTE}"));
    std::fs::read_dir(&tag)
        .expect("Anhang-Tag")
        .map(|eintrag| eintrag.expect("Eintrag").path())
        .find(|pfad| pfad.to_string_lossy().ends_with(name))
        .expect("Datei")
}

fn laenge(pfad: &Path) -> u64 {
    std::fs::metadata(pfad).expect("Datei").len()
}

fn protokoll(ordner: &Path) -> String {
    std::fs::read_to_string(ordner.join(format!("logs/{HEUTE}.log"))).unwrap_or_default()
}

fn ist(art: &'static str) -> impl Fn(&Empfangen) -> bool {
    move |e| e.data["art"] == art
}

/// R00040 „Rundweg mit Anhängen“ (US-7): `beta` löscht t3 → 200 mit 2 gelöscht, 0 Fehler; der erste Abruf beider
/// Anhänge nach der 200 antwortet 410; `anhang_belegt` sinkt von 2.764.800 auf 0 binnen 1 s; `alpha` erhält
/// `aenderung` (t3 ohne Anhänge) und `anhaenge` mit beiden IDs und Satzzeit t3, beide binnen 1 s; im Protokoll genau
/// eine `INFORMATION anhang Anhänge gelöscht` mit `grund=nachricht` und `fremde=0`, ohne ID und ohne `foto.png`;
/// beide Dateien haben 0 Bytes.
#[tokio::test]
async fn die_anhaenge_verschwinden_mit_der_nachricht() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path());
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let alpha = beitreten(&beta, basis, "alpha", None).await;
    let gegeben = bericht_senden(&beta, basis).await;
    assert_eq!(anhang_belegt(basis).await, FOTO + BERICHT);
    let mut strom = Strom::oeffnen(&beta, basis, &alpha).await;

    let antwort = loeschen(&beta, basis, &gegeben.token, &gegeben.t3).await;
    let erhalten = Instant::now();
    let abrufe = (
        abrufen(&beta, basis, &gegeben.foto).await.0,
        abrufen(&beta, basis, &gegeben.bericht).await.0,
    );
    let mut frei = anhang_belegt(basis).await;
    while frei > 0 && erhalten.elapsed() < GRENZE {
        frei = anhang_belegt(basis).await;
    }
    let frei_nach = erhalten.elapsed();
    let aenderung = strom.bis(Duration::from_secs(10), ist("aenderung")).await;
    let anhaenge = strom.bis(Duration::from_secs(10), ist("anhaenge")).await;

    assert_eq!(antwort.status, 200, "{}", antwort.json);
    assert_eq!(
        antwort.json["anhaenge"],
        json!({"geloescht": 2, "bytes": FOTO + BERICHT, "fehler": 0})
    );
    assert_eq!(antwort.json["nachricht"]["anhaenge"], json!([]));
    assert_eq!(abrufe, (410, 410));
    assert_eq!(frei, 0, "nach {frei_nach:?}");
    let aenderung = aenderung.expect("aenderung");
    assert_eq!(aenderung.data["nachricht"]["zeit"], gegeben.t3.as_str());
    assert_eq!(aenderung.data["nachricht"]["anhaenge"], json!([]));
    let anhaenge = anhaenge.expect("anhaenge");
    assert_eq!(
        anhaenge.data,
        json!({"art": "anhaenge", "channel": "allgemein", "geloescht": [
            {"id": gegeben.foto, "zeit": gegeben.t3}, {"id": gegeben.bericht, "zeit": gegeben.t3}]})
    );
    for ereignis in [&aenderung, &anhaenge] {
        let dauer = ereignis.ankunft.saturating_duration_since(erhalten);
        println!("{} nach der 200: {dauer:?}", ereignis.data["art"]);
        assert!(dauer < GRENZE, "{dauer:?}");
    }
    let protokoll = protokoll(wurzel.path());
    let information: Vec<&str> = protokoll
        .lines()
        .filter(|zeile| zeile.contains(" INFORMATION anhang Anhänge gelöscht"))
        .collect();
    assert_eq!(information.len(), 1, "{protokoll}");
    assert!(
        information[0].contains("anhaenge=2")
            && information[0].contains("fremde=0")
            && information[0].contains("fehler=0")
            && information[0].ends_with(" grund=nachricht"),
        "{}",
        information[0]
    );
    for verboten in [&gegeben.foto, &gegeben.bericht, "foto.png", "bericht.pdf"] {
        assert!(!protokoll.contains(verboten), "{verboten} im Protokoll");
    }
    assert!(
        protokoll.contains(
            " DEBUG aendern Nachricht gelöscht | channel=allgemein kennung=beta@127.0.0.1 anhaenge=2 "
        ),
        "{protokoll}"
    );
    assert_eq!(laenge(&datei_von(wurzel.path(), "foto.png")), 0);
    assert_eq!(laenge(&datei_von(wurzel.path(), "bericht.pdf")), 0);
}

/// R00040 „Vorher im Laufwerk gelöscht“ (US-8, M7): `gamma` löscht `foto.png` im Laufwerk (`bestaetigt: true`);
/// danach löscht `beta` t3 → `anhaenge.geloescht` 1; `anhang_belegt` ist insgesamt um genau 2.764.800 gesunken.
#[tokio::test]
async fn ein_im_laufwerk_geloeschter_anhang_wird_nicht_doppelt_abgezogen() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path());
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let gegeben = bericht_senden(&beta, basis).await;
    let vorher = anhang_belegt(basis).await;
    let gamma = beitreten(&beta, basis, "gamma", None).await;
    let laufwerk = api_antwort(
        beta.anfrage(
            reqwest::Method::POST,
            format!("{basis}api/anhaenge/loeschen"),
        )
        .header("X-LocalchatA-Sitzung", &gamma)
        .json(&json!({"ids": [gegeben.foto], "bestaetigt": true})),
    )
    .await;
    assert_eq!(laufwerk.status, 200, "{}", laufwerk.json);

    let antwort = loeschen(&beta, basis, &gegeben.token, &gegeben.t3).await;

    assert_eq!(antwort.status, 200, "{}", antwort.json);
    assert_eq!(antwort.json["anhaenge"]["geloescht"], 1);
    assert_eq!(antwort.json["anhaenge"]["fehler"], 0);
    assert_eq!(vorher - anhang_belegt(basis).await, FOTO + BERICHT);
}

/// R00040 „Kürzen scheitert“ (US-9, nicht als root): `bericht.pdf` schreibgeschützt (`0444`) → 200 mit 1 gelöscht und
/// 1 Fehler; beide Abrufe 410; `anhang_belegt` = 2.457.600; `ERROR anhang Löschen gescheitert` im Protokoll.
#[cfg(unix)]
#[tokio::test]
async fn scheitert_das_kuerzen_bleibt_der_anhang_geloescht_und_gezaehlt() {
    use std::os::unix::fs::PermissionsExt;
    let wurzel = ordner();
    let instanz = starten(wurzel.path());
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let gegeben = bericht_senden(&beta, basis).await;
    let datei = datei_von(wurzel.path(), "bericht.pdf");
    std::fs::set_permissions(&datei, std::fs::Permissions::from_mode(0o444)).expect("Rechte");

    let antwort = loeschen(&beta, basis, &gegeben.token, &gegeben.t3).await;

    std::fs::set_permissions(&datei, std::fs::Permissions::from_mode(0o644)).expect("Rechte");
    assert_eq!(antwort.status, 200, "{}", antwort.json);
    assert_eq!(
        (
            &antwort.json["anhaenge"]["geloescht"],
            &antwort.json["anhaenge"]["fehler"]
        ),
        (&json!(1), &json!(1))
    );
    assert_eq!(abrufen(&beta, basis, &gegeben.foto).await.0, 410);
    assert_eq!(abrufen(&beta, basis, &gegeben.bericht).await.0, 410);
    assert_eq!(anhang_belegt(basis).await, BERICHT);
    assert_eq!(laenge(&datei), BERICHT);
    assert!(
        protokoll(wurzel.path())
            .lines()
            .any(|zeile| zeile.contains(" ERROR anhang Löschen gescheitert")),
        "{}",
        protokoll(wurzel.path())
    );
    drop(instanz);
    let neu = starten(wurzel.path());

    assert_eq!(anhang_belegt(&neu.adresse).await, 0);
    assert_eq!(laenge(&datei), 0);
    assert_eq!(abrufen(&beta, &neu.adresse, &gegeben.bericht).await.0, 410);
}

/// Ein Ordner mit einer Nachricht von `beta` (t1) mit einem gebundenen Anhang in voller Länge (4.096 Bytes) und
/// danach von Hand einem Löschsatz derselben Kennung — wie nach einem Absturz zwischen Löschsatz und Kürzen.
fn nach_absturz(ordner: &Path) -> (String, PathBuf) {
    const ID: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaa";
    let t1 = "2026-10-28T11:00:00.000+01:00";
    let tag = ordner.join(format!("channels/allgemein/anhaenge/{HEUTE}"));
    std::fs::create_dir_all(&tag).expect("Anhangordner");
    let datei = tag.join("110000000-1-geheim.bin");
    std::fs::write(&datei, vec![5u8; 4_096]).expect("Datei");
    std::fs::write(
        ordner.join(format!("channels/allgemein/{HEUTE}.log")),
        format!(
            "@ {t1} beta@127.0.0.1\n+ art: agent\n+ anhang: {ID} 4096 application/octet-stream - 110000000-1-geheim.bin geheim.bin\n  Geheim\n\n\
             @ 2026-10-28T11:00:01.000+01:00 beta@127.0.0.1\n+ geloescht: {t1}\n\n"
        ),
    )
    .expect("Tagesdatei");
    (ID.to_string(), datei)
}

/// R00040 „Absturz zwischen Löschsatz und Kürzen“ (QE3, Pre-Mortem 3): nach dem Start antwortet der Abruf 410,
/// `anhang_belegt` ist 0, die Datei hat 0 Bytes; der Verlauf zeigt die Nachricht gelöscht ohne Anhänge.
#[tokio::test]
async fn ein_start_nach_dem_absturz_loescht_die_anhaenge_fertig() {
    let wurzel = ordner();
    let (id, datei) = nach_absturz(wurzel.path());
    let instanz = starten(wurzel.path());
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, basis, "beta", Some("agent")).await;

    let abruf = abrufen(&beta, basis, &id).await;
    let jetzt = verlauf(&beta, basis, &token, "allgemein").await;

    assert_eq!(abruf.0, 410);
    assert_eq!(anhang_belegt(basis).await, 0);
    assert_eq!(laenge(&datei), 0);
    assert_eq!(jetzt["nachrichten"][0]["zustand"], "geloescht");
    assert_eq!(jetzt["nachrichten"][0]["anhaenge"], json!([]));
}

/// Wie nach dem Absturz, aber die Datei ist schreibgeschützt (nicht als root): nach dem Start bleibt der Anhang
/// gelöscht (410) und gezählt, das Protokoll nennt `ERROR anhang Löschen gescheitert` ohne Namen und ohne ID.
#[cfg(unix)]
#[tokio::test]
async fn scheitert_das_kuerzen_beim_start_bleibt_der_anhang_geloescht() {
    use std::os::unix::fs::PermissionsExt;
    let wurzel = ordner();
    let (id, datei) = nach_absturz(wurzel.path());
    std::fs::set_permissions(&datei, std::fs::Permissions::from_mode(0o444)).expect("Rechte");
    let instanz = starten(wurzel.path());
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");

    let abruf = abrufen(&beta, basis, &id).await;

    std::fs::set_permissions(&datei, std::fs::Permissions::from_mode(0o644)).expect("Rechte");
    assert_eq!(abruf.0, 410);
    assert_eq!(
        anhang_belegt(basis).await,
        65_536,
        "Zählmaß z(4.096) = 64 KiB"
    );
    let protokoll = protokoll(wurzel.path());
    let fehler: Vec<&str> = protokoll
        .lines()
        .filter(|zeile| zeile.contains(" ERROR anhang Löschen gescheitert"))
        .collect();
    assert_eq!(fehler.len(), 1, "{protokoll}");
    assert!(
        fehler[0].contains("anhang=110000000-1")
            && !fehler[0].contains("geheim")
            && !fehler[0].contains(&id),
        "{}",
        fehler[0]
    );
}

/// R00040 „Fremd und nicht da“, Teil Anhänge (M1): `gamma` löscht t3 → 403; beide Anhänge weiter abrufbar (200).
#[tokio::test]
async fn eine_fremde_kennung_loescht_keine_anhaenge() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path());
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let gegeben = bericht_senden(&beta, basis).await;
    let gamma = beitreten(&beta, basis, "gamma", None).await;

    let antwort = loeschen(&beta, basis, &gamma, &gegeben.t3).await;

    assert_eq!(antwort.status, 403, "{}", antwort.json);
    assert_eq!(abrufen(&beta, basis, &gegeben.foto).await.0, 200);
    assert_eq!(abrufen(&beta, basis, &gegeben.bericht).await.0, 200);
    assert_eq!(anhang_belegt(basis).await, FOTO + BERICHT);
}

/// R00040 „Gelöschter Inhalt kommt nicht zurück“ (M15): nach dem Löschen von t3 nennt keine Antwort — Verlauf,
/// Senden-Antwort einer neuen Nachricht, Ereignis, Abruf der Anhänge — `Bericht`, `foto.png` oder `bericht.pdf`.
#[tokio::test]
async fn geloeschter_inhalt_kommt_nicht_zurueck() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path());
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let alpha = beitreten(&beta, basis, "alpha", None).await;
    let gegeben = bericht_senden(&beta, basis).await;
    let mut strom = Strom::oeffnen(&beta, basis, &alpha).await;

    let geloescht = loeschen(&beta, basis, &gegeben.token, &gegeben.t3).await;
    let neu = senden(&beta, basis, &gegeben.token, "allgemein", "Danach").await;
    let jetzt = verlauf(&beta, basis, &gegeben.token, "allgemein").await;
    let abrufe = [
        abrufen(&beta, basis, &gegeben.foto).await.1,
        abrufen(&beta, basis, &gegeben.bericht).await.1,
    ];
    strom.mitlesen(Duration::from_secs(1)).await;

    let ereignisse: Vec<Value> = strom.ereignisse.iter().map(|e| e.data.clone()).collect();
    let texte = [
        geloescht.json.to_string(),
        neu.json.to_string(),
        jetzt.to_string(),
        json!(ereignisse).to_string(),
        abrufe.join("\n"),
    ];
    for text in &texte {
        for verboten in ["Bericht", "foto.png", "bericht.pdf"] {
            assert!(!text.contains(verboten), "{verboten} in {text}");
        }
    }
}
