//! F0046 Wiederholschlüssel an der Grenze (R00012, US-1; ADR-0034 Regeln 3.7, 3.9, 6): das echte Programm,
//! `alpha` liest den Strom, `beta` sendet mit `Idempotency-Key`.

mod gemeinsam;

use gemeinsam::senden::{beitreten, ordner, senden, starten, tagesinhalt};
use gemeinsam::strom::{Empfangen, Strom};
use gemeinsam::{Absender, ApiAntwort, api_antwort};
use serde_json::json;
use std::time::Duration;

/// `POST …/nachrichten` mit `{"text": text}` und dem Kopf `Idempotency-Key: schluessel`.
async fn mit_schluessel(
    absender: &Absender,
    basis: &str,
    token: &str,
    channel: &str,
    text: &str,
    schluessel: &str,
) -> ApiAntwort {
    api_antwort(
        absender
            .anfrage(
                reqwest::Method::POST,
                format!("{basis}api/channels/{channel}/nachrichten"),
            )
            .header("X-LocalchatA-Sitzung", token)
            .header("Idempotency-Key", schluessel)
            .json(&json!({ "text": text })),
    )
    .await
}

fn ist_nachricht(e: &Empfangen) -> bool {
    e.data["art"] == "nachricht"
}

fn saetze_mit(inhalt: &[u8], text: &str) -> usize {
    String::from_utf8_lossy(inhalt)
        .lines()
        .filter(|zeile| *zeile == format!("  {text}"))
        .count()
}

/// R00012 F0046 „Format“: `a`, `A-z_0-9` und 64 × `a` → 201; leer, 65 × `a`, `a b`, `ä`, `a.b` → je 422
/// `regel: "schluessel"`; dazu zwei Köpfe mit unverbrauchten Schlüsseln → 422; nichts geschrieben.
#[tokio::test]
async fn das_format_des_schluessels_wird_geprueft_nichts_geschrieben() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[]);
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, &instanz.adresse, "beta", None).await;
    let basis = &instanz.adresse;
    let lang = "a".repeat(64);
    let zu_lang = "a".repeat(65);

    let mut gueltig = Vec::new();
    for (nummer, schluessel) in ["a", "A-z_0-9", lang.as_str()].iter().enumerate() {
        let text = format!("gut{nummer}");
        gueltig.push(mit_schluessel(&beta, basis, &token, "allgemein", &text, schluessel).await);
    }
    let vorher = tagesinhalt(wurzel.path(), "allgemein");
    let mut ungueltig = Vec::new();
    for schluessel in ["", zu_lang.as_str(), "a b", "ä", "a.b"] {
        let kopf = reqwest::header::HeaderValue::from_bytes(schluessel.as_bytes()).expect("Kopf");
        let antwort = api_antwort(
            beta.anfrage(
                reqwest::Method::POST,
                format!("{basis}api/channels/allgemein/nachrichten"),
            )
            .header("X-LocalchatA-Sitzung", &token)
            .header("Idempotency-Key", kopf)
            .json(&json!({ "text": "schlecht" })),
        )
        .await;
        ungueltig.push((schluessel.to_string(), antwort));
    }
    // Zwei Kopfzeilen roh über TCP — ein HTTP-Client könnte sie zusammenfassen.
    let doppelt = gemeinsam::roh_anfragen(
        std::net::SocketAddr::from(([127, 0, 0, 1], instanz.port())),
        &format!(
            "POST /api/channels/allgemein/nachrichten HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nX-LocalchatA-Sitzung: {token}\r\nContent-Type: application/json\r\nIdempotency-Key: zwei1\r\nIdempotency-Key: zwei2",
            instanz.port()
        ),
        r#"{"text":"schlecht"}"#,
    );
    let doppelt = ApiAntwort {
        status: doppelt.status,
        kopf: reqwest::header::HeaderMap::new(),
        json: serde_json::from_str(&doppelt.inhalt).unwrap_or_default(),
    };
    ungueltig.push(("zwei1, zwei2 (zwei Köpfe)".to_string(), doppelt));

    assert_eq!(
        gueltig.iter().map(|a| a.status).collect::<Vec<_>>(),
        [201, 201, 201]
    );
    for (schluessel, antwort) in &ungueltig {
        assert_eq!(antwort.status, 422, "{schluessel:?}: {}", antwort.json);
        assert_eq!(antwort.json["regel"], "schluessel", "{schluessel:?}");
        assert!(antwort.json["fehler"].is_string(), "{}", antwort.json);
    }
    assert_eq!(tagesinhalt(wurzel.path(), "allgemein"), vorher);
    assert_eq!(saetze_mit(&vorher, "schlecht"), 0);
}

/// R00012 F0046 „Wiederholung“ (US-1): 201 mit Zeit t; dieselbe noch einmal → 200 mit derselben Nachricht;
/// `Hallo!` mit `k1` → 422 `schluessel` mit dem Text; das nächste Ereignis nach `Hallo` ist `drei`; `Hallo`
/// genau einmal in der Tagesdatei; `k1` weder in Datei, Antworten noch Ereignissen.
#[tokio::test]
async fn us1_dieselbe_nachricht_mit_demselben_schluessel_ist_200_ohne_satz_und_ohne_ereignis() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[]);
    let alpha = Absender::von("127.0.0.1");
    let beta = Absender::von("127.0.0.1");
    let token_alpha = beitreten(&alpha, &instanz.adresse, "alpha", None).await;
    let token = beitreten(&beta, &instanz.adresse, "beta", None).await;
    let mut strom = Strom::oeffnen(&alpha, &instanz.adresse, &token_alpha).await;
    let basis = &instanz.adresse;

    let erste = mit_schluessel(&beta, basis, &token, "allgemein", "Hallo", "k1").await;
    let zweite = mit_schluessel(&beta, basis, &token, "allgemein", "Hallo", "k1").await;
    let andere = mit_schluessel(&beta, basis, &token, "allgemein", "Hallo!", "k1").await;
    let drei = senden(&beta, basis, &token, "allgemein", "drei").await;
    let hallo = strom
        .bis(Duration::from_secs(10), ist_nachricht)
        .await
        .expect("Ereignis Hallo");
    let danach = strom
        .bis(Duration::from_secs(10), |e| {
            ist_nachricht(e) && e.data["nachricht"]["text"] != "Hallo"
        })
        .await
        .expect("Ereignis danach");
    let inhalt = tagesinhalt(wurzel.path(), "allgemein");

    assert_eq!(erste.status, 201, "{}", erste.json);
    assert_eq!(zweite.status, 200, "{}", zweite.json);
    assert_eq!(zweite.json, erste.json);
    assert_eq!(andere.status, 422, "{}", andere.json);
    assert_eq!(
        andere.json,
        json!({"fehler": "Dieser Schlüssel gehört zu einer anderen Nachricht.", "regel": "schluessel"})
    );
    assert_eq!(drei.status, 201);
    assert_eq!(hallo.data["nachricht"]["text"], "Hallo");
    assert_eq!(danach.data["nachricht"]["text"], "drei");
    let ereignisse_hallo = strom
        .ereignisse
        .iter()
        .filter(|e| ist_nachricht(e) && e.data["nachricht"]["text"] == "Hallo")
        .count();
    assert_eq!(ereignisse_hallo, 1);
    assert_eq!(saetze_mit(&inhalt, "Hallo"), 1);
    assert_eq!(saetze_mit(&inhalt, "Hallo!"), 0);
    assert!(!String::from_utf8_lossy(&inhalt).contains("k1"));
    for antwort in [&erste, &zweite, &andere, &drei] {
        assert!(!antwort.json.to_string().contains("k1"), "{}", antwort.json);
    }
    for ereignis in &strom.ereignisse {
        assert!(
            !ereignis.data.to_string().contains("k1"),
            "{}",
            ereignis.data
        );
    }
}

/// R00012 F0046 „Wiederholung“: derselbe Schlüssel an einen anderen Channel → 422 `schluessel`; ein Schlüssel
/// einer anderen Kennung ist ein anderer Schlüssel → 201.
#[tokio::test]
async fn ein_schluessel_gilt_je_kennung_und_je_channel() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[]);
    let beta = Absender::von("127.0.0.1");
    let gamma = Absender::von("127.0.0.1");
    let token_beta = beitreten(&beta, &instanz.adresse, "beta", None).await;
    let token_gamma = beitreten(&gamma, &instanz.adresse, "gamma", None).await;
    let basis = &instanz.adresse;

    let erste = mit_schluessel(&beta, basis, &token_beta, "allgemein", "Hallo", "k1").await;
    let technik = mit_schluessel(&beta, basis, &token_beta, "technik", "Hallo", "k1").await;
    let fremd = mit_schluessel(&gamma, basis, &token_gamma, "allgemein", "Hallo", "k1").await;

    assert_eq!(erste.status, 201);
    assert_eq!(technik.status, 422, "{}", technik.json);
    assert_eq!(technik.json["regel"], "schluessel");
    assert_eq!(fremd.status, 201, "{}", fremd.json);
    assert_ne!(
        fremd.json["nachricht"]["zeit"],
        erste.json["nachricht"]["zeit"]
    );
    assert_eq!(
        saetze_mit(&tagesinhalt(wurzel.path(), "allgemein"), "Hallo"),
        2
    );
}

/// R00012 F0046 „Gleichzeitig“: 20 gleichzeitige `POST` von `beta` mit `k2` → genau eine 201, 19 × 200, alle mit
/// derselben Zeit; ein Satz im Log; `alpha` liest genau ein Ereignis dazu.
#[tokio::test]
async fn zwanzig_gleichzeitige_wiederholungen_ergeben_einen_satz() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[]);
    let alpha = Absender::von("127.0.0.1");
    let beta = std::sync::Arc::new(Absender::von("127.0.0.1"));
    let token_alpha = beitreten(&alpha, &instanz.adresse, "alpha", None).await;
    let token = beitreten(&beta, &instanz.adresse, "beta", None).await;
    let mut strom = Strom::oeffnen(&alpha, &instanz.adresse, &token_alpha).await;

    let laeufe: Vec<_> = (0..20)
        .map(|_| {
            let beta = std::sync::Arc::clone(&beta);
            let basis = instanz.adresse.clone();
            let token = token.clone();
            tokio::spawn(async move {
                mit_schluessel(&beta, &basis, &token, "allgemein", "Hallo", "k2").await
            })
        })
        .collect();
    let mut antworten = Vec::new();
    for lauf in laeufe {
        antworten.push(lauf.await.expect("Lauf"));
    }
    strom.bis(Duration::from_secs(10), ist_nachricht).await;
    strom.mitlesen(Duration::from_millis(500)).await;

    let mut status: Vec<_> = antworten.iter().map(|a| a.status).collect();
    status.sort_unstable();
    assert_eq!(status[0], 200);
    assert_eq!(
        status.iter().filter(|s| **s == 201).count(),
        1,
        "{status:?}"
    );
    assert_eq!(
        status.iter().filter(|s| **s == 200).count(),
        19,
        "{status:?}"
    );
    let zeiten: std::collections::BTreeSet<_> = antworten
        .iter()
        .map(|a| a.json["nachricht"]["zeit"].to_string())
        .collect();
    assert_eq!(zeiten.len(), 1, "{zeiten:?}");
    assert_eq!(
        saetze_mit(&tagesinhalt(wurzel.path(), "allgemein"), "Hallo"),
        1
    );
    assert_eq!(
        strom.ereignisse.iter().filter(|e| ist_nachricht(e)).count(),
        1
    );
}

/// R00012 F0046 „Wiederholung bucht“: `beta` sendet mit Schlüsseln, bis der Eimer leer ist (mindestens die 30 des
/// Vorrats); die Wiederholung der letzten durchgegangenen unmittelbar danach → 429 (keine Ausnahme vom Konto);
/// nach `Retry-After` dieselbe Wiederholung → 200 mit der ursprünglichen Zeit.
#[tokio::test]
async fn eine_wiederholung_bucht_am_schreibkonto_wie_ein_erster_versuch() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[]);
    let beta = Absender::von("127.0.0.1");
    let token = beitreten(&beta, &instanz.adresse, "beta", None).await;
    let basis = &instanz.adresse;

    let mut durch = Vec::new();
    for nummer in 0..60 {
        let schluessel = format!("s{nummer}");
        let antwort = mit_schluessel(
            &beta,
            basis,
            &token,
            "allgemein",
            &format!("n{nummer}"),
            &schluessel,
        )
        .await;
        if antwort.status == 429 {
            break;
        }
        assert_eq!(antwort.status, 201, "{}", antwort.json);
        durch.push((nummer, antwort));
    }
    let (letzte, ursprung) = durch.last().expect("durchgegangen");
    let text = format!("n{letzte}");
    let schluessel = format!("s{letzte}");
    let gebremst = mit_schluessel(&beta, basis, &token, "allgemein", &text, &schluessel).await;
    let warten: u64 = gebremst
        .kopf
        .get("retry-after")
        .and_then(|wert| wert.to_str().ok())
        .and_then(|wert| wert.parse().ok())
        .expect("Retry-After");
    tokio::time::sleep(Duration::from_secs(warten)).await;
    let spaeter = mit_schluessel(&beta, basis, &token, "allgemein", &text, &schluessel).await;

    assert!(durch.len() >= 30, "{}", durch.len());
    assert_eq!(gebremst.status, 429, "{}", gebremst.json);
    assert_eq!(spaeter.status, 200, "{}", spaeter.json);
    assert_eq!(
        spaeter.json["nachricht"]["zeit"],
        ursprung.json["nachricht"]["zeit"]
    );
    assert_eq!(
        saetze_mit(&tagesinhalt(wurzel.path(), "allgemein"), &text),
        1
    );
}

/// Ein Teilnehmer der Flut: Absenderadresse, Name, Token.
struct Teilnehmer {
    absender: Absender,
    name: String,
    token: String,
}

/// Je Herkunft `127.0.1.<n>` (n aus `herkuenfte`) `je_herkunft` Kennungen `<vorsilbe><n>x<j>`.
async fn viele_beitreten(
    basis: &str,
    herkuenfte: std::ops::RangeInclusive<u8>,
    je_herkunft: usize,
    vorsilbe: &str,
) -> Vec<Teilnehmer> {
    let mut alle = Vec::new();
    for herkunft in herkuenfte {
        for j in 0..je_herkunft {
            let absender = Absender::von(&format!("127.0.1.{herkunft}"));
            let name = format!("{vorsilbe}{herkunft}x{j}");
            let token = beitreten(&absender, basis, &name, Some("agent")).await;
            alle.push(Teilnehmer {
                absender,
                name,
                token,
            });
        }
    }
    alle
}

/// Jede Kennung sendet `je_kennung` Nachrichten mit eigenem Schlüssel, alle Kennungen zugleich; die Statuscodes.
async fn fluten(basis: &str, teilnehmer: &[Teilnehmer], je_kennung: usize) -> Vec<u16> {
    let je_teilnehmer = futures_util::future::join_all(teilnehmer.iter().map(|t| async move {
        let mut status = Vec::new();
        for nummer in 0..je_kennung {
            let text = format!("{} {nummer}", t.name);
            let schluessel = format!("{}-{nummer}", t.name);
            status.push(
                mit_schluessel(
                    &t.absender,
                    basis,
                    &t.token,
                    "allgemein",
                    &text,
                    &schluessel,
                )
                .await
                .status,
            );
        }
        status
    }))
    .await;
    je_teilnehmer.into_iter().flatten().collect()
}

fn verdraengt_zeilen(ordner: &std::path::Path) -> Vec<String> {
    gemeinsam::protokoll::alle_zeilen(ordner)
        .into_iter()
        .filter(|zeile| zeile.contains("Wiederholschlüssel vor Ablauf verdrängt"))
        .collect()
}

/// Lauf der Abnahme-Probe aus dem Observability-Review #22 (S1): `alfa` sendet „Wichtig“ mit `auftrag-1`; 160
/// Kennungen (40 Herkünfte × 4) senden je 28 = 4.480 Nachrichten; `alfa` wiederholt. Liefert die Wiederholung,
/// die Zahl der Sätze „Wichtig“, die Episodenzeilen und den Zähler.
async fn wichtig_unter_last(umgebung: &[(&str, &str)]) -> (ApiAntwort, usize, Vec<String>, u64) {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), umgebung);
    let basis = &instanz.adresse;
    let alfa = Absender::von("127.0.0.1");
    let token = beitreten(&alfa, basis, "alfa", Some("agent")).await;
    let erste = mit_schluessel(&alfa, basis, &token, "allgemein", "Wichtig", "auftrag-1").await;
    assert_eq!(erste.status, 201, "{}", erste.json);
    let flut = viele_beitreten(basis, 2..=41, 4, "f").await;

    let status = fluten(basis, &flut, 28).await;
    let wiederholt =
        mit_schluessel(&alfa, basis, &token, "allgemein", "Wichtig", "auftrag-1").await;

    assert_eq!(status.len(), 4_480);
    assert!(status.iter().all(|s| *s == 201), "Flut nicht angenommen");
    let saetze = saetze_mit(&tagesinhalt(wurzel.path(), "allgemein"), "Wichtig");
    let (_, gesundheit) = gemeinsam::gesundheit(basis).await;
    let zaehler = gesundheit["schluessel_verdraengt"]
        .as_u64()
        .expect("Zähler");
    (
        wiederholt,
        saetze,
        verdraengt_zeilen(wurzel.path()),
        zaehler,
    )
}

/// R00025 „Unter Last“ (Abnahme-Probe Observability #22 S1, wörtlich: „alfa sendet „Wichtig“ mit auftrag-1; 160
/// Kennungen senden 4.480 Nachrichten; alfa wiederholt → …“), Lesart nach E1 mit den echten Grenzen: 200, „Wichtig“
/// genau einmal, keine Episode, Zähler 0. Vor der Härtung rot (201, zwei Sätze).
#[tokio::test]
async fn unter_last_haelt_der_schluessel_mit_den_echten_grenzen() {
    let (wiederholt, saetze, episoden, zaehler) = wichtig_unter_last(&[]).await;

    assert_eq!(wiederholt.status, 200, "{}", wiederholt.json);
    assert_eq!(saetze, 1);
    assert_eq!(episoden, Vec::<String>::new());
    assert_eq!(zaehler, 0);
}

/// R00025 „Unter Last“ mit `LOCALCHATA_TESTGRENZE_SCHLUESSEL=4096` (die 4.096 der Probe, Debug-Build): die
/// Wiederholung antwortet 200 mit „Wichtig“ genau einmal — `alfa` hat 1 Eintrag, jede der 160 Kennungen 28 —, und
/// die Episoden nennen nur Fluter, `grenze=gesamt`, `hoechstens=4096`; Zähler ≥ 1.
#[tokio::test]
async fn unter_last_mit_kleiner_gesamtgrenze_trifft_die_verdraengung_die_fluter() {
    let (wiederholt, saetze, episoden, zaehler) =
        wichtig_unter_last(&[("LOCALCHATA_TESTGRENZE_SCHLUESSEL", "4096")]).await;

    assert_eq!(wiederholt.status, 200, "{}", wiederholt.json);
    assert_eq!(saetze, 1);
    let beginne: Vec<&String> = episoden
        .iter()
        .filter(|zeile| zeile.contains("Episode beginnt"))
        .collect();
    assert!(!beginne.is_empty(), "keine Episode");
    for zeile in &beginne {
        assert!(zeile.contains(" WARNING senden "), "{zeile}");
        assert!(zeile.contains(" kennung=f"), "{zeile}");
        assert!(!zeile.contains("alfa"), "{zeile}");
        assert!(
            zeile.contains(" grenze=gesamt hoechstens=4096 alter_ms="),
            "{zeile}"
        );
    }
    assert!(zaehler >= 1);
}

/// R00025 „Fremde verdrängen nicht“ (Abnahme-Probe Security #22, wörtlich: „A merkt v1; 4.096 Sendungen anderer
/// Kennungen; A wiederholt v1 → 200, eine Zeile“), Aufbau nach Angriffsmatrix Zeile 2: 35 Herkünfte
/// `127.0.1.2`–`127.0.1.36` × 4 Kennungen × 30 Sendungen, ohne Testknopf → 200, ein Satz, Zähler 0, keine Episode.
#[tokio::test]
async fn fremde_verdraengen_den_schluessel_nicht() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[]);
    let basis = &instanz.adresse;
    let a = Absender::von("127.0.0.1");
    let token = beitreten(&a, basis, "opfer", None).await;
    let erste = mit_schluessel(&a, basis, &token, "allgemein", "Mein Satz", "v1").await;
    let flut = viele_beitreten(basis, 2..=36, 4, "g").await;

    let status = fluten(basis, &flut, 30).await;
    let wiederholt = mit_schluessel(&a, basis, &token, "allgemein", "Mein Satz", "v1").await;
    let (_, gesundheit) = gemeinsam::gesundheit(basis).await;

    assert_eq!(erste.status, 201);
    assert!(status.len() >= 4_096 && status.iter().all(|s| *s == 201));
    assert_eq!(wiederholt.status, 200, "{}", wiederholt.json);
    assert_eq!(wiederholt.json, erste.json);
    assert_eq!(
        saetze_mit(&tagesinhalt(wurzel.path(), "allgemein"), "Mein Satz"),
        1
    );
    assert_eq!(gesundheit["schluessel_verdraengt"], 0);
    assert_eq!(verdraengt_zeilen(wurzel.path()), Vec::<String>::new());
}

/// R00025 „Testknopf“ (ADR-0042 Ergänzungen Karte #36, erste Tests wörtlich: „`=0` und `=x` → Exit 2, kein
/// `logs/`“; Angriffsmatrix Zeile 19): jeder Wert außer `[1-9][0-9]{0,4}` bis 36.000 → Exit 2, stderr nennt die
/// Variable und „1 bis 36.000“, kein `logs/`.
#[test]
fn falsche_werte_des_testknopfs_enden_mit_exit_2_ohne_protokoll() {
    for falsch in ["0", "x", "", "36001", "-1", "+5", "05", " 5", "5 "] {
        let wurzel = ordner();
        let pfad = wurzel.path().to_string_lossy().into_owned();
        let ende = gemeinsam::aufrufen_mit(
            &[
                &pfad,
                "--port",
                "0",
                "--host",
                "127.0.0.1",
                "--kein-browser",
            ],
            &[("LOCALCHATA_TESTGRENZE_SCHLUESSEL", falsch)],
        );

        assert_eq!(ende.exit, Some(2), "{falsch:?}: {}", ende.stderr);
        assert!(
            ende.stderr.contains("LOCALCHATA_TESTGRENZE_SCHLUESSEL"),
            "{}",
            ende.stderr
        );
        assert!(ende.stderr.contains("1 bis 36.000"), "{}", ende.stderr);
        assert!(!wurzel.path().join("logs").exists(), "{falsch:?}");
    }
}

/// R00025 „Testknopf“ (wörtlich: „`=5` → die sechste Sendung verdrängt, Episode und Zähler 1“; Angriffsmatrix
/// Zeile 20): 6 Kennungen senden je 1 mit Schlüssel → der Eintrag der ersten fällt (Gleichstand, am längsten auf
/// 1), Episode `grenze=gesamt hoechstens=5`, Zähler 1, ihre Wiederholung antwortet 201; `=1` und `=36000` starten.
#[tokio::test]
async fn mit_testgrenze_5_verdraengt_die_sechste_sendung_den_ersten() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[("LOCALCHATA_TESTGRENZE_SCHLUESSEL", "5")]);
    let basis = &instanz.adresse;
    let sechs = viele_beitreten(basis, 2..=3, 3, "s").await;

    let mut status = Vec::new();
    for teilnehmer in &sechs {
        status.extend(fluten(basis, std::slice::from_ref(teilnehmer), 1).await);
    }
    let erste = &sechs[0];
    let (_, gesundheit) = gemeinsam::gesundheit(basis).await;
    let beginne: Vec<String> = verdraengt_zeilen(wurzel.path())
        .into_iter()
        .filter(|zeile| zeile.contains("Episode beginnt"))
        .collect();
    let wiederholt = mit_schluessel(
        &erste.absender,
        basis,
        &erste.token,
        "allgemein",
        &format!("{} 0", erste.name),
        &format!("{}-0", erste.name),
    )
    .await;

    assert_eq!(status, [201; 6]);
    assert_eq!(beginne.len(), 1, "{beginne:?}");
    assert!(
        beginne[0].contains(&format!(
            " kennung={}@127.0.1.2 grenze=gesamt hoechstens=5 alter_ms=",
            erste.name
        )),
        "{}",
        beginne[0]
    );
    assert_eq!(gesundheit["schluessel_verdraengt"], 1);
    assert_eq!(wiederholt.status, 201, "{}", wiederholt.json);
    for gueltig in ["1", "36000"] {
        let wurzel = ordner();
        let instanz = starten(
            wurzel.path(),
            &[("LOCALCHATA_TESTGRENZE_SCHLUESSEL", gueltig)],
        );
        assert_eq!(
            gemeinsam::gesundheit(&instanz.adresse).await.0,
            200,
            "{gueltig}"
        );
    }
}

/// R00025 „Gleichzeitig bei vollem Speicher“ (Angriffsmatrix Zeile 27): Testknopf `=5`, Speicher voll; 20
/// gleichzeitige Sendungen mit demselben neuen Schlüssel → eine 201, 19 × 200 mit gleicher `zeit`, ein Satz,
/// genau eine Verdrängung.
#[tokio::test]
async fn gleichzeitig_bei_vollem_speicher_genau_ein_satz_und_eine_verdraengung() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[("LOCALCHATA_TESTGRENZE_SCHLUESSEL", "5")]);
    let basis = &instanz.adresse;
    let fuellung = viele_beitreten(basis, 2..=2, 5, "v").await;
    assert_eq!(fluten(basis, &fuellung, 1).await, [201; 5]);
    let neu = Absender::von("127.0.1.3");
    let token = beitreten(&neu, basis, "gleichzeitig", None).await;

    let antworten = futures_util::future::join_all(
        (0..20).map(|_| mit_schluessel(&neu, basis, &token, "allgemein", "Einmal", "neu-1")),
    )
    .await;
    let (_, gesundheit) = gemeinsam::gesundheit(basis).await;

    let mut status: Vec<u16> = antworten.iter().map(|a| a.status).collect();
    status.sort_unstable();
    assert_eq!(status, [[200; 19].as_slice(), &[201]].concat());
    let zeiten: std::collections::BTreeSet<String> = antworten
        .iter()
        .map(|a| a.json["nachricht"]["zeit"].to_string())
        .collect();
    assert_eq!(zeiten.len(), 1, "{zeiten:?}");
    assert_eq!(
        saetze_mit(&tagesinhalt(wurzel.path(), "allgemein"), "Einmal"),
        1
    );
    assert_eq!(gesundheit["schluessel_verdraengt"], 1);
}
