//! R00050 F0172 B0770 — die Wege im Direktgespräch über das echte Programm (ADR-0055 Regel 3; Matrix R4, R6, W2, W4,
//! W6, W8, E1, E2, E3): Senden, Verlauf, Bearbeiten, Löschen, Antworten, Anhang und Schlüssel wie im Channel; ein
//! Dritter bekommt nichts, nicht einmal die Auskunft, ob es ein Gespräch gibt.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::anhaenge::{ablegen, starten as starten_anhaenge};
use gemeinsam::direkt::{
    ANNA, ANNA_BEN, BEN, CORA, Runde, direkt_eintraege, direkt_stand, paarinhalt, runde, runde_mit,
};
use gemeinsam::senden::{JETZT, ordner};
use gemeinsam::{ApiAntwort, api_antwort};
use serde_json::{Value, json};
use std::time::Duration;

const ANNA_CORA: &str = "anna@127.0.0.2+cora@127.0.0.1";

fn zeit(antwort: &ApiAntwort) -> String {
    assert_eq!(antwort.status, 201, "{}", antwort.json);
    antwort.json["nachricht"]["zeit"]
        .as_str()
        .expect("zeit")
        .to_string()
}

/// Der Rundweg wörtlich: Senden, Verlauf, Antwort, Bearbeiten, Löschen und Anhang im Direktgespräch; jede Antwort
/// trägt `channel: null` und `direkt` (das Gegenüber des Anfragenden).
#[tokio::test]
async fn rundweg_senden_verlauf_antwort_bearbeiten_loeschen_und_anhang() {
    let wurzel = ordner();
    let instanz = starten_anhaenge(wurzel.path(), &[], &[("LOCALCHATA_JETZT", JETZT)]);
    let r = runde_mit(wurzel, instanz).await;
    let basis = r.instanz.adresse.clone();

    let eins = r.anna.senden(&basis, BEN, &json!({"text": "eins"})).await;
    let z1 = zeit(&eins);
    let zwei = r
        .ben
        .senden(&basis, ANNA, &json!({"text": "zwei", "antwort_auf": z1}))
        .await;
    let z2 = zeit(&zwei);
    let id = ablegen(
        &r.anna.absender,
        &basis,
        &r.anna.token,
        "notiz.txt",
        b"anhang".to_vec(),
    )
    .await;
    let drei = r
        .anna
        .senden(&basis, BEN, &json!({"text": "drei", "anhaenge": [id]}))
        .await;
    let z3 = zeit(&drei);
    let bearbeitet = r
        .anna
        .post(
            &basis,
            BEN,
            "nachrichten/bearbeiten",
            &json!({"zeit": z1, "text": "eins!"}),
        )
        .await;
    let geloescht = r
        .ben
        .post(&basis, ANNA, "nachrichten/loeschen", &json!({"zeit": z2}))
        .await;
    let mit_anhang_geloescht = r
        .anna
        .post(&basis, BEN, "nachrichten/loeschen", &json!({"zeit": z3}))
        .await;
    let verlauf = r.ben.lesen(&basis, ANNA).await;

    assert_eq!(zwei.json["direkt"], ANNA, "{}", zwei.json);
    assert_eq!(
        zwei.json["nachricht"]["antwort_auf"]["zeit"], z1,
        "{}",
        zwei.json
    );
    assert_eq!(zwei.json["nachricht"]["antwort_auf"]["textanfang"], "eins");
    assert_eq!(
        drei.json["nachricht"]["anhaenge"][0]["name"], "notiz.txt",
        "{}",
        drei.json
    );
    assert_eq!(bearbeitet.status, 200, "{}", bearbeitet.json);
    assert_eq!(bearbeitet.json["channel"], json!(null));
    assert_eq!(bearbeitet.json["direkt"], BEN);
    assert_eq!(bearbeitet.json["nachricht"]["text"], "eins!");
    assert_eq!(bearbeitet.json["nachricht"]["zustand"], "bearbeitet");
    assert_eq!(geloescht.status, 200, "{}", geloescht.json);
    assert_eq!(geloescht.json["direkt"], ANNA);
    assert_eq!(geloescht.json["ergebnis"], "geloescht");
    assert_eq!(
        mit_anhang_geloescht.status, 200,
        "{}",
        mit_anhang_geloescht.json
    );
    assert_eq!(mit_anhang_geloescht.json["anhaenge"]["geloescht"], 1);
    let zustaende: Vec<Value> = verlauf.json["nachrichten"]
        .as_array()
        .expect("Liste")
        .iter()
        .map(|n| n["zustand"].clone())
        .collect();
    assert_eq!(
        zustaende,
        [json!("bearbeitet"), json!("geloescht"), json!("geloescht")]
    );
    assert_eq!(direkt_eintraege(r.wurzel.path()), [ANNA_BEN]);
    let tag = &z3[..10];
    assert!(
        r.wurzel
            .path()
            .join(format!("direkt/{ANNA_BEN}/anhaenge/{tag}"))
            .is_dir(),
        "der Anhang liegt im Paarordner"
    );
}

/// W2 (wörtlich): cora → anna mit `von`, `direkt`, `channel`, `an` im Körper → wie im Channel 422 `feld`; der
/// Absender ist immer die Sitzung, die Nachricht landet nur in `anna…+cora…`, anna ↔ ben bleibt byte-gleich.
#[tokio::test]
async fn w2_felder_im_koerper_aendern_den_absender_nie_und_der_fremde_ordner_bleibt_gleich() {
    let r = runde(&[]).await;
    let basis = r.instanz.adresse.clone();
    zeit(&r.anna.senden(&basis, BEN, &json!({"text": "privat"})).await);
    let vorher = direkt_stand(r.wurzel.path());
    let koerper =
        json!({"text": "x", "von": ANNA, "direkt": BEN, "channel": "allgemein", "an": BEN});

    let abgewiesen = r.cora.senden(&basis, ANNA, &koerper).await;
    let im_channel = gemeinsam::senden::roh_senden(
        &r.cora.absender,
        &basis,
        Some(&r.cora.token),
        "allgemein",
        "application/json",
        koerper.to_string().into_bytes(),
    )
    .await;
    let gesendet = r
        .cora
        .senden(&basis, ANNA, &json!({"text": "hallo anna"}))
        .await;

    assert_eq!(abgewiesen.status, 422, "{}", abgewiesen.json);
    assert_eq!(abgewiesen.json["regel"], "feld");
    assert_eq!(
        (im_channel.status, &im_channel.json["regel"]),
        (422, &json!("feld")),
        "wie im Channel"
    );
    assert_eq!(gesendet.status, 201, "{}", gesendet.json);
    assert_eq!(gesendet.json["nachricht"]["von"], CORA);
    assert_eq!(gesendet.json["direkt"], ANNA);
    let jetzt = direkt_stand(r.wurzel.path());
    assert_eq!(
        jetzt.keys().cloned().collect::<Vec<_>>(),
        [
            format!("direkt/{ANNA_BEN}/2026-10-28.log"),
            format!("direkt/{ANNA_CORA}/2026-10-28.log")
        ]
    );
    assert_eq!(
        jetzt[&format!("direkt/{ANNA_BEN}/2026-10-28.log")],
        vorher[&format!("direkt/{ANNA_BEN}/2026-10-28.log")]
    );
}

async fn warten_bis_weg(r: &Runde, kennung: &str) {
    let basis = r.instanz.adresse.clone();
    for _ in 0..100 {
        if !r
            .cora
            .teilnehmerliste(&basis)
            .await
            .iter()
            .any(|k| k == kennung)
        {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("{kennung} steht nach 5 s noch in der Teilnehmerliste");
}

/// W4 (wörtlich): cora → `zed@127.0.0.9` nie gesehen; → `ben@127.0.0.3` nach weg ohne Gespräch; → anna nach weg
/// **mit** Gespräch → 404 `{regel:"empfaenger"}` byte-gleich für „nie“ und „weg“, kein Ordner; mit Gespräch 201.
/// Mutante „Empfängerprüfung auch bei bestehendem Gespräch“.
#[tokio::test]
async fn w4_neues_gespraech_nur_mit_der_teilnehmerliste_ein_bestehendes_geht_immer_weiter() {
    let r = runde(&[("LOCALCHATA_GNADENFRIST_MS", "200")]).await;
    let basis = r.instanz.adresse.clone();
    let bestehend = r
        .cora
        .senden(&basis, ANNA, &json!({"text": "wir kennen uns"}))
        .await;
    assert_eq!(bestehend.status, 201, "{}", bestehend.json);
    assert_eq!(r.ben.verlassen(&basis).await.status, 204);
    assert_eq!(r.anna.verlassen(&basis).await.status, 204);
    warten_bis_weg(&r, ANNA).await;
    warten_bis_weg(&r, BEN).await;
    let vorher = direkt_eintraege(r.wurzel.path());

    let nie = r
        .cora
        .senden(&basis, "zed@127.0.0.9", &json!({"text": "x"}))
        .await;
    let weg = r.cora.senden(&basis, BEN, &json!({"text": "x"})).await;
    let mit_gespraech = r
        .cora
        .senden(&basis, ANNA, &json!({"text": "noch da?"}))
        .await;

    assert_eq!(nie.status, 404, "{}", nie.json);
    assert_eq!(nie.json["regel"], "empfaenger");
    assert_eq!(
        (weg.status, &weg.json),
        (nie.status, &nie.json),
        "byte-gleich für nie und weg"
    );
    assert_eq!(vorher, [ANNA_CORA]);
    assert_eq!(
        direkt_eintraege(r.wurzel.path()),
        [ANNA_CORA],
        "kein neuer Ordner"
    );
    assert_eq!(
        mit_gespraech.status, 201,
        "ein bestehendes Gespräch geht weiter: {}",
        mit_gespraech.json
    );
}

/// W6 (wörtlich): cora in `…/anna…` mit `antwort_auf` = Zeit einer Nachricht aus anna ↔ ben → 422 `unbekannt`,
/// byte-gleich zu einer nie vergebenen Zeit; kein Zitat; die Logs bleiben unverändert und es entsteht kein Ordner.
/// Mutante „Bezug global aufgelöst“.
#[tokio::test]
async fn w6_ein_bezug_aus_einem_fremden_gespraech_ist_unbekannt_wie_eine_nie_vergebene_zeit() {
    let r = runde(&[]).await;
    let basis = r.instanz.adresse.clone();
    let fremd = zeit(&r.anna.senden(&basis, BEN, &json!({"text": "geheim"})).await);
    let vorher = direkt_stand(r.wurzel.path());

    let mit_fremder_zeit = r
        .cora
        .senden(&basis, ANNA, &json!({"text": "re", "antwort_auf": fremd}))
        .await;
    let nie_vergeben = r
        .cora
        .senden(
            &basis,
            ANNA,
            &json!({"text": "re", "antwort_auf": "2026-10-28T09:00:00.000+01:00"}),
        )
        .await;

    assert_eq!(mit_fremder_zeit.status, 422, "{}", mit_fremder_zeit.json);
    assert_eq!(
        mit_fremder_zeit.json["grund"], "unbekannt",
        "{}",
        mit_fremder_zeit.json
    );
    assert_eq!(
        (mit_fremder_zeit.status, &mit_fremder_zeit.json),
        (nie_vergeben.status, &nie_vergeben.json),
        "byte-gleich zur nie vergebenen Zeit"
    );
    assert!(!mit_fremder_zeit.json.to_string().contains("geheim"));
    assert_eq!(
        direkt_stand(r.wurzel.path()),
        vorher,
        "Logs unverändert, kein Ordner"
    );
}

/// W8 (wörtlich): anna Schlüssel `k` an ben; anna Schlüssel `k` an cora; cora Schlüssel `k` an anna → wie im Channel
/// (Wiederholung „anders“ nur für dieselbe Kennung); cora erfährt nichts aus anna ↔ ben.
#[tokio::test]
async fn w8_der_wiederholschluessel_gilt_je_kennung_und_gespraech() {
    let r = runde(&[]).await;
    let basis = r.instanz.adresse.clone();
    let mit_schluessel =
        |t: &gemeinsam::direkt::Teilnehmer, an: &'static str, text: &'static str| {
            let anfrage = t
                .absender
                .anfrage(
                    reqwest::Method::POST,
                    format!("{basis}api/direkt/{an}/nachrichten"),
                )
                .header("X-LocalchatA-Sitzung", &t.token)
                .header("Idempotency-Key", "k")
                .json(&json!({"text": text}));
            api_antwort(anfrage)
        };

    let erste = mit_schluessel(&r.anna, BEN, "eins").await;
    let wiederholt = mit_schluessel(&r.anna, BEN, "eins").await;
    let anders_im_gespraech = mit_schluessel(&r.anna, BEN, "etwas anderes").await;
    let anders_gespraech = mit_schluessel(&r.anna, CORA, "eins").await;
    let coras = mit_schluessel(&r.cora, ANNA, "eins").await;

    assert_eq!(erste.status, 201, "{}", erste.json);
    assert_eq!(wiederholt.status, 200, "{}", wiederholt.json);
    assert_eq!(wiederholt.json["nachricht"], erste.json["nachricht"]);
    assert_eq!(
        anders_im_gespraech.status, 422,
        "{}",
        anders_im_gespraech.json
    );
    assert_eq!(anders_gespraech.status, 422, "{}", anders_gespraech.json);
    assert_eq!(
        coras.status, 201,
        "cora erfährt nichts von annas Schlüssel: {}",
        coras.json
    );
    let log = String::from_utf8(paarinhalt(r.wurzel.path(), ANNA_BEN)).expect("UTF-8");
    assert_eq!(
        log.matches("eins").count(),
        1,
        "nur einmal angehängt: {log}"
    );
}

/// E1 (wörtlich): cora `POST /api/direkt/anna@127.0.0.2/nachrichten/bearbeiten` bzw. `…/loeschen` mit der Zeit aus
/// anna ↔ ben; dasselbe über `/api/channels/allgemein/…` → 404 wie „nie da“, byte-gleich zu einer Zufallszeit; das
/// Log anna ↔ ben bleibt unverändert. E3: cora ohne Gespräch mit anna → 404 wie nie da, kein Ordner. Mutante „Suche der
/// Zeit über alle Gespräche“.
#[tokio::test]
async fn e1_e3_fremde_zeit_und_fehlendes_gespraech_sind_404_wie_nie_da() {
    let r = runde(&[]).await;
    let basis = r.instanz.adresse.clone();
    let fremd = zeit(&r.anna.senden(&basis, BEN, &json!({"text": "geheim"})).await);
    let zufall = "2026-10-28T09:00:00.000+01:00";
    let vorher = direkt_stand(r.wurzel.path());
    let mut antworten = Vec::new();

    for (unterpfad, koerper) in [
        (
            "nachrichten/bearbeiten",
            json!({"zeit": fremd, "text": "x"}),
        ),
        ("nachrichten/loeschen", json!({"zeit": fremd})),
    ] {
        let zufallskoerper = {
            let mut k = koerper.clone();
            k["zeit"] = json!(zufall);
            k
        };
        // E3: kein Gespräch cora ↔ anna
        let ohne_gespraech = r.cora.post(&basis, ANNA, unterpfad, &koerper).await;
        let ohne_gespraech_zufall = r.cora.post(&basis, ANNA, unterpfad, &zufallskoerper).await;
        // E1: Gespräch cora ↔ anna besteht (zweiter Durchlauf), die Zeit ist aus anna ↔ ben
        let ueber_channel = gemeinsam::api_antwort(
            r.cora
                .absender
                .anfrage(
                    reqwest::Method::POST,
                    format!("{basis}api/channels/allgemein/{unterpfad}"),
                )
                .header("X-LocalchatA-Sitzung", &r.cora.token)
                .json(&koerper),
        )
        .await;
        antworten.push((
            unterpfad,
            ohne_gespraech,
            ohne_gespraech_zufall,
            ueber_channel,
        ));
    }
    let eigenes = r.cora.senden(&basis, ANNA, &json!({"text": "hallo"})).await;
    assert_eq!(eigenes.status, 201, "{}", eigenes.json);
    let mut mit_gespraech = Vec::new();
    for (unterpfad, koerper) in [
        (
            "nachrichten/bearbeiten",
            json!({"zeit": fremd, "text": "x"}),
        ),
        ("nachrichten/loeschen", json!({"zeit": fremd})),
    ] {
        mit_gespraech.push((
            unterpfad,
            r.cora.post(&basis, ANNA, unterpfad, &koerper).await,
        ));
    }

    for (unterpfad, ohne, zufall_antwort, ueber_channel) in &antworten {
        assert_eq!(ohne.status, 404, "{unterpfad}: {}", ohne.json);
        assert_eq!(
            ohne.json["regel"], "nachricht",
            "{unterpfad}: {}",
            ohne.json
        );
        assert_eq!(
            (ohne.status, &ohne.json),
            (zufall_antwort.status, &zufall_antwort.json),
            "{unterpfad}"
        );
        assert_eq!(
            (ueber_channel.status, &ueber_channel.json),
            (ohne.status, &ohne.json),
            "{unterpfad}: im Channel dieselbe Antwort"
        );
    }
    for (unterpfad, antwort) in &mit_gespraech {
        assert_eq!(
            (antwort.status, &antwort.json["regel"]),
            (404, &json!("nachricht")),
            "{unterpfad}"
        );
    }
    let jetzt = direkt_stand(r.wurzel.path());
    assert_eq!(
        jetzt[&format!("direkt/{ANNA_BEN}/2026-10-28.log")],
        vorher[&format!("direkt/{ANNA_BEN}/2026-10-28.log")],
        "das Log anna ↔ ben bleibt unverändert"
    );
}

/// E3 für sich: ohne Gespräch entsteht durch Bearbeiten oder Löschen kein Ordner.
#[tokio::test]
async fn e3_bearbeiten_und_loeschen_legen_nie_ein_gespraech_an() {
    let r = runde(&[]).await;
    let basis = r.instanz.adresse.clone();
    let zufall = "2026-10-28T09:00:00.000+01:00";

    let bearbeitet = r
        .cora
        .post(
            &basis,
            ANNA,
            "nachrichten/bearbeiten",
            &json!({"zeit": zufall, "text": "x"}),
        )
        .await;
    let geloescht = r
        .cora
        .post(
            &basis,
            ANNA,
            "nachrichten/loeschen",
            &json!({"zeit": zufall}),
        )
        .await;

    assert_eq!((bearbeitet.status, geloescht.status), (404, 404));
    assert_eq!(direkt_eintraege(r.wurzel.path()), Vec::<String>::new());
}

/// E2 (wörtlich): ben bearbeitet oder löscht annas Nachricht im gemeinsamen Gespräch → wie im Channel für fremde
/// Nachrichten (403 `fremd`, ADR-0048); das Log bleibt unverändert.
#[tokio::test]
async fn e2_der_andere_im_gespraech_darf_eine_fremde_nachricht_weder_bearbeiten_noch_loeschen() {
    let r = runde(&[]).await;
    let basis = r.instanz.adresse.clone();
    let z = zeit(&r.anna.senden(&basis, BEN, &json!({"text": "meine"})).await);
    let vorher = direkt_stand(r.wurzel.path());

    let bearbeiten = r
        .ben
        .post(
            &basis,
            ANNA,
            "nachrichten/bearbeiten",
            &json!({"zeit": z, "text": "deine"}),
        )
        .await;
    let loeschen = r
        .ben
        .post(&basis, ANNA, "nachrichten/loeschen", &json!({"zeit": z}))
        .await;

    for antwort in [&bearbeiten, &loeschen] {
        assert_eq!(antwort.status, 403, "{}", antwort.json);
        assert_eq!(antwort.json["regel"], "fremd");
    }
    assert_eq!(direkt_stand(r.wurzel.path()), vorher);
}

/// R4 (wörtlich): `GET /api/channels/{n}/nachrichten`, n ∈ {`anna@127.0.0.2+ben@127.0.0.3`, `anna%40127.0.0.2`,
/// `..%2Fdirekt%2Fanna@127.0.0.2+ben@127.0.0.3`, `direkt`} → 404/422 wie für ungültige bzw. unbekannte Channelnamen;
/// nie Inhalt.
#[tokio::test]
async fn r4_ueber_die_channelroute_kommt_nie_ein_direktgespraech() {
    let r = runde(&[]).await;
    let basis = r.instanz.adresse.clone();
    zeit(&r.anna.senden(&basis, BEN, &json!({"text": "geheim"})).await);

    for n in [
        "anna@127.0.0.2+ben@127.0.0.3",
        "anna%40127.0.0.2",
        "..%2Fdirekt%2Fanna@127.0.0.2+ben@127.0.0.3",
        "direkt",
        "direkt%2Fanna@127.0.0.2+ben@127.0.0.3",
    ] {
        let antwort = api_antwort(
            r.cora
                .absender
                .anfrage(
                    reqwest::Method::GET,
                    format!("{basis}api/channels/{n}/nachrichten"),
                )
                .header("X-LocalchatA-Sitzung", &r.cora.token),
        )
        .await;
        assert!(
            [404, 422].contains(&antwort.status),
            "{n}: {} {}",
            antwort.status,
            antwort.json
        );
        assert!(!antwort.json.to_string().contains("geheim"), "{n}");
        assert!(
            antwort.json.get("nachrichten").is_none(),
            "{n}: {}",
            antwort.json
        );
    }
}

/// R6 (a) (wörtlich): anna verlässt, die Gnadenfrist ist um; mallory tritt als `anna` von 127.0.0.2 bei — liest
/// anna ↔ ben, **hingenommen** (PO 168, Vision „als Absender gilt jeder, der mit derselben Kennung beitritt“).
#[tokio::test]
async fn r6a_namensuebernahme_von_derselben_herkunft_ist_hingenommen() {
    use gemeinsam::Absender;
    use gemeinsam::senden::beitreten;
    let r = runde(&[("LOCALCHATA_GNADENFRIST_MS", "200")]).await;
    let basis = r.instanz.adresse.clone();
    zeit(&r.anna.senden(&basis, BEN, &json!({"text": "geheim"})).await);
    assert_eq!(r.anna.verlassen(&basis).await.status, 204);
    warten_bis_weg(&r, ANNA).await;
    let mallory = Absender::von("127.0.0.2");
    let token = beitreten(&mallory, &basis, "anna", None).await;

    let uebernommen = api_antwort(
        mallory
            .anfrage(
                reqwest::Method::GET,
                format!("{basis}api/direkt/{BEN}/nachrichten"),
            )
            .header("X-LocalchatA-Sitzung", token),
    )
    .await;

    assert_eq!(
        uebernommen.json["nachrichten"].as_array().map(Vec::len),
        Some(1),
        "hingenommen: dieselbe Kennung, dasselbe Gespräch: {}",
        uebernommen.json
    );
}

/// R6 (b), (c) (wörtlich): mallory tritt als `anna` von 127.0.0.9 bei → Kennung `anna@127.0.0.9`, liest leer; das alte
/// Token von 127.0.0.9 → 401. Mutante „`beteiligt` vergleicht nur den Namen“ (Sichtbarkeit, B0766; hier die
/// Gegenprobe auf dem Weg der Routen).
#[tokio::test]
async fn r6bc_namensuebernahme_von_anderer_herkunft_liest_leer_das_alte_token_ist_401() {
    use gemeinsam::Absender;
    use gemeinsam::senden::beitreten;
    let r = runde(&[("LOCALCHATA_GNADENFRIST_MS", "200")]).await;
    let basis = r.instanz.adresse.clone();
    zeit(&r.anna.senden(&basis, BEN, &json!({"text": "geheim"})).await);
    assert_eq!(r.anna.verlassen(&basis).await.status, 204);
    warten_bis_weg(&r, ANNA).await;
    let mallory = Absender::von("127.0.0.9");
    let token = beitreten(&mallory, &basis, "anna", None).await;
    let lesen = |token: String| {
        api_antwort(
            mallory
                .anfrage(
                    reqwest::Method::GET,
                    format!("{basis}api/direkt/{BEN}/nachrichten"),
                )
                .header("X-LocalchatA-Sitzung", token),
        )
    };

    let andere_herkunft = lesen(token).await;
    let altes_token = lesen(r.anna.token.clone()).await;

    assert_eq!(andere_herkunft.status, 200, "{}", andere_herkunft.json);
    assert_eq!(
        andere_herkunft.json["nachrichten"],
        json!([]),
        "(b) {}",
        andere_herkunft.json
    );
    assert_eq!(altes_token.status, 401, "(c) {}", altes_token.json);
}

/// Prüfreihenfolge, Schreibkonto: ein Direktgespräch bucht am selben Konto wie der Channel (30 Züge, dann 429
/// `kennung`); die 429 kommt vor der Empfängerprüfung, danach ist der unbekannte Empfänger ein 404.
#[tokio::test]
async fn das_schreibkonto_bucht_direktnachrichten_und_die_429_kommt_vor_der_empfaengerpruefung() {
    let r = runde(&[]).await;
    let basis = r.instanz.adresse.clone();
    let mut durchgegangen = 0;
    let mut gebremst = None;
    for nummer in 0..80 {
        let antwort = r
            .anna
            .senden(&basis, BEN, &json!({"text": format!("s{nummer}")}))
            .await;
        match antwort.status {
            201 => durchgegangen += 1,
            429 => {
                gebremst = Some(antwort);
                break;
            }
            anderes => panic!("{anderes}: {}", antwort.json),
        }
    }

    let gebremst = gebremst.expect("das Schreibkonto bremst");
    assert!((30..=40).contains(&durchgegangen), "{durchgegangen}");
    assert_eq!(gebremst.json["grenze"], "kennung", "{}", gebremst.json);
    assert!(gebremst.kopf.get("retry-after").is_some());
    let unbekannt = r
        .anna
        .senden(&basis, "zed@127.0.0.9", &json!({"text": "x"}))
        .await;
    assert_eq!(
        unbekannt.status, 429,
        "429 vor 404 empfaenger: {}",
        unbekannt.json
    );
    tokio::time::sleep(Duration::from_secs(2)).await;
    let danach = r
        .anna
        .senden(&basis, "zed@127.0.0.9", &json!({"text": "x"}))
        .await;
    assert_eq!(
        (danach.status, &danach.json["regel"]),
        (404, &json!("empfaenger"))
    );
}

/// Prüfreihenfolge, Quote: der unbekannte Empfänger ist 404 vor der Plattenquote 507; an der vollen Quote ist ein
/// neues Gespräch 507 und hinterlässt keinen Paarordner.
#[tokio::test]
async fn an_der_plattenquote_ist_ein_neues_gespraech_507_und_ohne_ordner() {
    let wurzel = ordner();
    let jetzt = localchata::uhr::Zeit::parse_from_rfc3339(JETZT).expect("jetzt");
    let berlin = |zeitpunkt: chrono::DateTime<chrono::Utc>| {
        zeitpunkt.with_timezone(&chrono::FixedOffset::east_opt(3600).expect("Versatz"))
    };
    localchata::testdaten::quote_ordner(
        wurzel.path(),
        "allgemein",
        10_485_760 - 300,
        jetzt,
        berlin,
    )
    .expect("Quote-Ordner");
    let instanz = starten_anhaenge(wurzel.path(), &[], &[("LOCALCHATA_JETZT", JETZT)]);
    let r = runde_mit(wurzel, instanz).await;
    let basis = r.instanz.adresse.clone();
    let lang = "q".repeat(400);

    let unbekannt = r
        .anna
        .senden(&basis, "zed@127.0.0.9", &json!({"text": lang}))
        .await;
    let voll = r.anna.senden(&basis, BEN, &json!({"text": lang})).await;

    assert_eq!(
        (unbekannt.status, &unbekannt.json["regel"]),
        (404, &json!("empfaenger"))
    );
    assert_eq!(voll.status, 507, "{}", voll.json);
    assert_eq!(voll.json["regel"], "quote");
    assert_eq!(
        direkt_eintraege(r.wurzel.path()),
        Vec::<String>::new(),
        "kein Paarordner"
    );
}
