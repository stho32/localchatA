//! R00050 F0173 B0774 — die sichtbare Marke von `warten` über das echte Programm (ADR-0055 Regel 8; Matrix G1, G2):
//! `seit` in der Antwort hängt nur von Zeiten ab, die der Wartende sehen darf. Die Direktnachricht zweier anderer
//! verrät ihren Zeitpunkt weder ohne `seit` (G2) noch mit `seit` (G1) noch mitten im Warten (Bus); die eigenen
//! Direktgespräche zählen. Die Hälften mit dem Filter `direkt` und die Verlustfreiheit (G3) gehören F0168 (B0749).
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::direkt::{BEN, Runde, Teilnehmer, runde};
use gemeinsam::senden::senden;
use gemeinsam::{Absender, ApiAntwort};
use serde_json::{Value, json};
use std::time::Duration;

/// `GET /api/warten?<anfrage>` als `wer`.
async fn warten(wer: &Teilnehmer, basis: &str, anfrage: &str) -> ApiAntwort {
    gemeinsam::api_antwort(
        wer.absender
            .anfrage(reqwest::Method::GET, format!("{basis}api/warten?{anfrage}"))
            .header("X-LocalchatA-Sitzung", &wer.token),
    )
    .await
}

fn zeit_der_antwort(antwort: &ApiAntwort) -> String {
    assert_eq!(antwort.status, 201, "{}", antwort.json);
    antwort.json["nachricht"]["zeit"]
        .as_str()
        .expect("zeit")
        .to_string()
}

/// `ben` schreibt in `allgemein` (Zeit `tc`), danach schreibt `anna` `ben` eine Direktnachricht mit `@cora` im Text
/// (Zeit `td`, `td` > `tc`). Liefert `(tc, td)`.
async fn channel_dann_direkt(r: &Runde, basis: &str) -> (String, String) {
    let channel = senden(
        &r.ben.absender,
        basis,
        &r.ben.token,
        "allgemein",
        "im Channel",
    )
    .await;
    let tc = zeit_der_antwort(&channel);
    let direkt = r
        .anna
        .senden(basis, BEN, &json!({"text": "geheim für @cora"}))
        .await;
    let td = zeit_der_antwort(&direkt);
    assert!(td > tc, "{td} nach {tc}");
    (tc, td)
}

/// G2 (wörtlich): `cora` ruft `warten?bis=0` ohne `seit`, die letzte Nachricht im Ordner ist eine Direktnachricht
/// zweier anderer → `seit` ist die Zeit der letzten Channel-Nachricht, nicht die der Direktnachricht. Die Beteiligten
/// sehen ihre eigene: `ben` bekommt `td`. Dazu die Hälfte ohne Nachricht im Channel: nur Direktes im Ordner → kein
/// `seit` für `cora`.
#[tokio::test]
async fn g2_ohne_seit_antwortet_die_letzte_channel_zeit_nicht_die_der_direktnachricht() {
    let r = runde(&[]).await;
    let basis = r.instanz.adresse.clone();
    let (tc, td) = channel_dann_direkt(&r, &basis).await;

    let cora = warten(&r.cora, &basis, "bis=0").await;
    let ben = warten(&r.ben, &basis, "bis=0").await;

    assert_eq!(cora.status, 200, "{}", cora.json);
    assert_eq!(cora.json["seit"], json!(tc), "{}", cora.json);
    assert_eq!(cora.json["nachrichten"], json!([]));
    assert_eq!(
        ben.json["seit"],
        json!(td),
        "die eigene Direktnachricht zählt"
    );
}

/// G2, Hälfte „nur Direktes im Ordner“: ohne Channel-Nachricht gibt es für `cora` nichts Sichtbares, also kein `seit`.
#[tokio::test]
async fn g2_gibt_es_nur_fremdes_direktes_verraet_seit_nichts() {
    let r = runde(&[]).await;
    let basis = r.instanz.adresse.clone();
    let direkt = r
        .anna
        .senden(&basis, BEN, &json!({"text": "nur wir"}))
        .await;
    assert_eq!(direkt.status, 201, "{}", direkt.json);

    let cora = warten(&r.cora, &basis, "bis=0").await;

    assert_eq!(cora.status, 200, "{}", cora.json);
    assert_eq!(cora.json["seit"], Value::Null, "{}", cora.json);
    assert_eq!(cora.json["nachrichten"], json!([]));
}

/// Wartet, bis Health genau einen wartenden Aufruf zeigt (höchstens 5 s) — erst dann liegt der Aufruf im Warten.
async fn wartend_abwarten(basis: &str) {
    for _ in 0..100 {
        if gemeinsam::gesundheit(basis).await.1["wartend"] == 1 {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("der Aufruf wartet nach 5 s nicht (Health wartend != 1)");
}

/// G1 (wörtlich): `cora` ruft `warten?bis=0` → `s0`; `anna` schreibt `ben`; `cora` ruft erneut mit `seit=s0` — ohne
/// Filter, mit `direkt`, mit `erwaehnung` bei `@cora` im fremden Text — und bekommt keinen Treffer, `seit` bleibt
/// `s0`, solange die Channels ruhen; nach einer Channel-Nachricht ist es deren Zeit. Mitten im Warten (Bus) ändert die
/// Direktnachricht den Stand ebenso nicht.
#[tokio::test]
async fn g1_seit_bleibt_s0_solange_die_channels_ruhen_und_folgt_dann_dem_channel() {
    let r = runde(&[]).await;
    let basis = r.instanz.adresse.clone();
    let channel = senden(&r.ben.absender, &basis, &r.ben.token, "allgemein", "vorher").await;
    let s0 = zeit_der_antwort(&channel);
    let erster = warten(&r.cora, &basis, "bis=0").await;
    assert_eq!(erster.json["seit"], json!(s0), "{}", erster.json);

    let laufend = tokio::spawn({
        let basis = basis.clone();
        let cora = Teilnehmer {
            absender: Absender::von("127.0.0.1"),
            token: r.cora.token.clone(),
        };
        let anfrage = format!("seit={}&bis=2", s0.replace('+', "%2B"));
        async move { warten(&cora, &basis, &anfrage).await }
    });
    wartend_abwarten(&basis).await;
    let direkt = r
        .anna
        .senden(&basis, BEN, &json!({"text": "geheim für @cora"}))
        .await;
    assert_eq!(direkt.status, 201, "{}", direkt.json);
    let mitten = laufend.await.expect("Aufruf");

    let seit = format!("seit={}", s0.replace('+', "%2B"));
    let ohne = warten(&r.cora, &basis, &format!("{seit}&bis=0")).await;
    let mit_direkt = warten(&r.cora, &basis, &format!("{seit}&bis=0&direkt")).await;
    let mit_erwaehnung = warten(&r.cora, &basis, &format!("{seit}&bis=0&erwaehnung")).await;
    let danach = senden(
        &r.ben.absender,
        &basis,
        &r.ben.token,
        "allgemein",
        "nachher",
    )
    .await;
    let t2 = zeit_der_antwort(&danach);
    let nach_channel = warten(&r.cora, &basis, &format!("{seit}&bis=0")).await;

    for (name, antwort) in [
        ("mitten im Warten", &mitten),
        ("ohne Filter", &ohne),
        ("direkt", &mit_direkt),
        ("erwaehnung", &mit_erwaehnung),
    ] {
        assert_eq!(antwort.status, 200, "{name}: {}", antwort.json);
        assert_eq!(antwort.json["nachrichten"], json!([]), "{name}");
        assert_eq!(antwort.json["seit"], json!(s0), "{name}: {}", antwort.json);
    }
    assert_eq!(
        nach_channel.json["seit"],
        json!(t2),
        "{}",
        nach_channel.json
    );
    assert_eq!(
        nach_channel.json["nachrichten"][0]["nachricht"]["text"],
        "nachher"
    );
}
