//! R00050 F0172 B0769 — der Zugang zu den Direktrouten über das echte Programm (ADR-0055 Regel 3 „Extraktoren“,
//! Matrix R1, R2, R3, R5, W1, W3): die Sitzung vor dem Pfad, die Kanonprüfung des Gegenübers, Lesen legt nichts an.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::ApiAntwort;
use gemeinsam::direkt::{
    ANNA, ANNA_BEN, BEN, CORA, Runde, direkt_eintraege, lesen_ohne_sitzung, runde,
};
use serde_json::json;

fn ohne_datum(antwort: &ApiAntwort) -> (u16, String) {
    (antwort.status, antwort.json.to_string())
}

/// R1 (wörtlich): `GET /api/direkt/{x}/nachrichten` ohne Sitzung, x ∈ {`ben@127.0.0.3`, `..%2F..`, `%FF`, `a+b`} →
/// 401 für alle x (Sitzung vor Kennung), byte-gleich. Mutante „422 vor 401 (Path-Extraktor vor Teilnehmer)“.
#[tokio::test]
async fn r1_ohne_sitzung_ist_es_fuer_jede_pfadform_dieselbe_401() {
    let r = runde(&[]).await;
    let basis = r.instanz.adresse.clone();
    let mut antworten = Vec::new();

    for x in [BEN, "..%2F..", "%FF", "a+b", "%00", "anna%40127.0.0.2"] {
        let gelesen = lesen_ohne_sitzung(&r.cora.absender, &basis, x).await;
        assert_eq!(gelesen.status, 401, "GET {x}: {}", gelesen.json);
        let gesendet = gemeinsam::api_antwort(
            r.cora
                .absender
                .anfrage(
                    reqwest::Method::POST,
                    format!("{basis}api/direkt/{x}/nachrichten"),
                )
                .json(&json!({"text": "x"})),
        )
        .await;
        assert_eq!(gesendet.status, 401, "POST {x}: {}", gesendet.json);
        antworten.push(ohne_datum(&gelesen));
        antworten.push(ohne_datum(&gesendet));
    }

    assert!(
        antworten.windows(2).all(|paar| paar[0] == paar[1]),
        "{antworten:?}"
    );
    assert_eq!(direkt_eintraege(r.wurzel.path()), Vec::<String>::new());
}

/// W1 (wörtlich): `POST` ohne Sitzung mit fremdem Origin → 403 vor 401; ohne Sitzung ohne Origin mit ungültiger
/// Kennung → 401 vor 422. Mutante „Reihenfolge vertauscht“.
#[tokio::test]
async fn w1_403_vor_401_und_401_vor_422() {
    let r = runde(&[]).await;
    let basis = r.instanz.adresse.clone();
    let post = |segment: &'static str, origin: Option<&'static str>| {
        let mut anfrage = r.cora.absender.anfrage(
            reqwest::Method::POST,
            format!("{basis}api/direkt/{segment}/nachrichten"),
        );
        if let Some(origin) = origin {
            anfrage = anfrage.header("Origin", origin);
        }
        gemeinsam::api_antwort(anfrage.json(&json!({"text": "x"})))
    };

    let fremd = post(BEN, Some("http://evil.example")).await;
    let ohne_ungueltig = post("%FF", None).await;
    let ohne_gueltig = post(BEN, None).await;

    assert_eq!(fremd.status, 403, "{}", fremd.json);
    assert_eq!(ohne_ungueltig.status, 401, "{}", ohne_ungueltig.json);
    assert_eq!(ohne_gueltig.status, 401, "{}", ohne_gueltig.json);
    assert_eq!(ohne_datum(&ohne_ungueltig), ohne_datum(&ohne_gueltig));
    assert_eq!(direkt_eintraege(r.wurzel.path()), Vec::<String>::new());
}

/// Alles, was keine kanonische `name@herkunft` ist — R2 (wörtlich) und `%FF` (mit Sitzung nie axums 400).
fn unzulaessige_formen() -> Vec<String> {
    let mut formen: Vec<String> = [
        "anna@127.0.0.2+ben@127.0.0.3",
        "anna@127.0.0.2%2Bben@127.0.0.3",
        "anna",
        "anna@",
        "..%2Fdirekt%2Fanna@127.0.0.2+ben@127.0.0.3",
        "%252F",
        "Anna@127.0.0.2",
        "anna@127.0.0.2%00",
        "%FF",
        "%",
        "%G1",
    ]
    .map(str::to_string)
    .to_vec();
    formen.push("a".repeat(200));
    formen
}

async fn kein_ordner_und_422_kennung(r: &Runde, formen: &[String], schreibend: bool) {
    let basis = r.instanz.adresse.clone();
    let mut koerper = Vec::new();
    for x in formen {
        let antwort = if schreibend {
            r.cora.senden(&basis, x, &json!({"text": "x"})).await
        } else {
            r.cora.lesen(&basis, x).await
        };
        assert_eq!(antwort.status, 422, "{x}: {}", antwort.json);
        assert_eq!(antwort.json["regel"], "kennung", "{x}: {}", antwort.json);
        koerper.push(antwort.json.to_string());
    }
    assert!(
        koerper.windows(2).all(|paar| paar[0] == paar[1]),
        "Körper je Variante gleich"
    );
    assert_eq!(
        direkt_eintraege(r.wurzel.path()),
        Vec::<String>::new(),
        "kein Ordner"
    );
}

/// R2 (wörtlich): cora mit unzulässigen Formen → 422 `{regel:"kennung"}`, Körper je Variante gleich; kein Ordner.
#[tokio::test]
async fn r2_unzulaessige_formen_sind_422_kennung_lesend_wie_schreibend_und_legen_nichts_an() {
    let r = runde(&[]).await;

    kein_ordner_und_422_kennung(&r, &unzulaessige_formen(), false).await;
    kein_ordner_und_422_kennung(&r, &unzulaessige_formen(), true).await;
}

/// R5 (wörtlich): Aliase der Herkunft — cora liest, anna sendet an einen Alias von ben → 422 `kennung`; kein zweiter
/// Ordner, kein Lesen unter anderem Namen. Mutante „nur `Kennung::bilden`“.
#[tokio::test]
async fn r5_aliase_der_herkunft_sind_422_kennung_und_ein_zweiter_name_entsteht_nie() {
    let r = runde(&[]).await;
    let basis = r.instanz.adresse.clone();
    let aliase_lesen = [
        "anna@::ffff:127.0.0.2",
        "anna@0:0:0:0:0:ffff:7f00:2",
        "anna@127.0.0.02",
        "anna@fd00:0::7",
        "anna@FD00::7",
        "anna@[fd00::7]",
        "anna@fd00::7%25eth0",
    ];
    let aliase_ben = [
        "ben@::ffff:127.0.0.3",
        "ben@0:0:0:0:0:ffff:7f00:3",
        "ben@127.0.0.03",
        "ben@fd00:0::7",
        "ben@FD00::7",
        "ben@[fd00::7]",
        "ben@fd00::7%25eth0",
    ];
    let k1 = r.anna.senden(&basis, BEN, &json!({"text": "k1"})).await;
    assert_eq!(k1.status, 201, "Kontrollpaar: {}", k1.json);

    for x in aliase_lesen {
        let antwort = r.cora.lesen(&basis, x).await;
        assert_eq!(
            (antwort.status, antwort.json["regel"].clone()),
            (422, json!("kennung")),
            "{x}"
        );
    }
    for x in aliase_ben {
        let antwort = r.anna.senden(&basis, x, &json!({"text": "alias"})).await;
        assert_eq!(
            (antwort.status, antwort.json["regel"].clone()),
            (422, json!("kennung")),
            "{x}"
        );
    }

    assert_eq!(direkt_eintraege(r.wurzel.path()), [ANNA_BEN]);
    let verlauf = r.ben.lesen(&basis, ANNA).await;
    assert_eq!(
        verlauf.json["nachrichten"].as_array().map(Vec::len),
        Some(1)
    );
}

/// R3 (wörtlich): cora `GET …/anna@127.0.0.2/nachrichten` und `…/ben@127.0.0.3/…` → 200 leer, nie Inhalt von
/// anna ↔ ben; **kein** Ordner `anna…+cora…`. Mutante „`GET` legt an“.
#[tokio::test]
async fn r3_lesen_ohne_gespraech_ist_200_leer_nie_fremder_inhalt_und_legt_nichts_an() {
    let r = runde(&[]).await;
    let basis = r.instanz.adresse.clone();
    let k1 = r.anna.senden(&basis, BEN, &json!({"text": "geheim"})).await;
    assert_eq!(k1.status, 201, "Kontrollpaar: {}", k1.json);
    let vorher = direkt_eintraege(r.wurzel.path());

    let bei_anna = r.cora.lesen(&basis, ANNA).await;
    let bei_ben = r.cora.lesen(&basis, BEN).await;

    for antwort in [&bei_anna, &bei_ben] {
        assert_eq!(antwort.status, 200, "{}", antwort.json);
        assert_eq!(antwort.json["nachrichten"], json!([]), "{}", antwort.json);
        assert_eq!(antwort.json["aelter"], false);
        assert_eq!(antwort.json["neuer"], false);
        assert!(!antwort.json.to_string().contains("geheim"));
    }
    assert_eq!(bei_anna.json["direkt"], ANNA);
    assert_eq!(bei_ben.json["direkt"], BEN);
    assert_eq!(
        direkt_eintraege(r.wurzel.path()),
        vorher,
        "ls direkt/ vorher = nachher"
    );
    assert_eq!(vorher, [ANNA_BEN]);
}

/// W3 (wörtlich): cora → `cora@127.0.0.1` → 422 `selbst`; cora → `cora@::ffff:127.0.0.1` → 422 `kennung` (nicht
/// `selbst`), kein Ordner. Mutante „Alias als selbst erkannt und umgeformt“.
#[tokio::test]
async fn w3_an_sich_selbst_ist_422_selbst_und_ein_alias_der_eigenen_kennung_422_kennung() {
    let r = runde(&[]).await;
    let basis = r.instanz.adresse.clone();

    let selbst = r.cora.senden(&basis, CORA, &json!({"text": "x"})).await;
    let alias = r
        .cora
        .senden(&basis, "cora@::ffff:127.0.0.1", &json!({"text": "x"}))
        .await;
    let lesen_selbst = r.cora.lesen(&basis, CORA).await;

    assert_eq!(
        (selbst.status, selbst.json["regel"].clone()),
        (422, json!("selbst"))
    );
    assert_eq!(
        (alias.status, alias.json["regel"].clone()),
        (422, json!("kennung"))
    );
    assert_eq!(
        lesen_selbst.status, 422,
        "auch lesen ist an sich selbst 422"
    );
    assert_eq!(lesen_selbst.json["regel"], "selbst");
    assert_eq!(direkt_eintraege(r.wurzel.path()), Vec::<String>::new());
}

/// Prüfreihenfolge: 401 → 422 `kennung` → 422 `selbst` vor dem Körper (415, 413, 400) — ein falscher Typ an einer
/// unzulässigen Kennung ist 422, kein 415.
#[tokio::test]
async fn die_kennung_wird_vor_dem_koerper_geprueft() {
    let r = runde(&[]).await;
    let basis = r.instanz.adresse.clone();
    let roh = |segment: &str| {
        gemeinsam::api_antwort(
            r.cora
                .absender
                .anfrage(
                    reqwest::Method::POST,
                    format!("{basis}api/direkt/{segment}/nachrichten"),
                )
                .header("X-LocalchatA-Sitzung", &r.cora.token)
                .header("Content-Type", "text/plain")
                .body("kein json"),
        )
    };

    let ungueltig = roh("%FF").await;
    let selbst = roh(CORA).await;
    let gueltig = roh(BEN).await;

    assert_eq!(ungueltig.status, 422);
    assert_eq!(ungueltig.json["regel"], "kennung");
    assert_eq!(selbst.status, 422);
    assert_eq!(selbst.json["regel"], "selbst");
    assert_eq!(
        gueltig.status, 415,
        "mit gültiger Kennung gilt der Körper: {}",
        gueltig.json
    );
}
