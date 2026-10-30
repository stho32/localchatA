//! R00050 F0174 B0776 — Anhänge löschen: die Sichtbarkeit sitzt an der Auflösung der ID (ADR-0055 Regel 5 „Anhänge
//! löschen“, B3; Matrix A2; Linie L2, echtes Programm). Für `cora` ist die ID eines Anhangs aus `anna` ↔ `ben` in
//! jedem Ausgang `unbekannt` — byte-gleich zu einer nie vergebenen ID, ohne 409-Vorschau, ohne `schon_geloescht`,
//! ohne Wirkung auf Datei, Quote und Zähler.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::anhaenge::{ablegen, gesundheit, starten as starten_anhaenge};
use gemeinsam::direkt::{BEN, Runde, Teilnehmer, direkt_stand, runde_mit};
use gemeinsam::senden::{JETZT, ordner};
use gemeinsam::{ApiAntwort, api_antwort};
use serde_json::{Value, json};
use std::time::Duration;

/// Zwei nie vergebene IDs (26 Zeichen aus `[a-z2-7]`) als Gegenprobe.
const ZUFALL: &str = "zzzzzzzzzzzzzzzzzzzzzzzzzz";
const ZUFALL_ZWEI: &str = "yyyyyyyyyyyyyyyyyyyyyyyyyy";

async fn runde_anhaenge() -> Runde {
    let wurzel = ordner();
    let instanz = starten_anhaenge(wurzel.path(), &[], &[("LOCALCHATA_JETZT", JETZT)]);
    runde_mit(wurzel, instanz).await
}

/// `anna` legt einen Anhang ab und sendet ihn an `ben`; liefert die ID.
async fn direkt_anhang(r: &Runde, inhalt: &[u8]) -> String {
    let basis = &r.instanz.adresse;
    let id = ablegen(
        &r.anna.absender,
        basis,
        &r.anna.token,
        "geheim.txt",
        inhalt.to_vec(),
    )
    .await;
    let gesendet = r
        .anna
        .senden(basis, BEN, &json!({"text": "für dich", "anhaenge": [id]}))
        .await;
    assert_eq!(gesendet.status, 201, "{}", gesendet.json);
    id
}

/// Ein Anhang von `wer` in `allgemein`; liefert die ID.
async fn channel_anhang(r: &Runde, wer: &Teilnehmer, inhalt: &[u8]) -> String {
    let basis = &r.instanz.adresse;
    let id = ablegen(
        &wer.absender,
        basis,
        &wer.token,
        "eigen.txt",
        inhalt.to_vec(),
    )
    .await;
    let gesendet = api_antwort(
        wer.absender
            .anfrage(
                reqwest::Method::POST,
                format!("{basis}api/channels/allgemein/nachrichten"),
            )
            .header("X-LocalchatA-Sitzung", &wer.token)
            .json(&json!({"text": "eigen", "anhaenge": [id]})),
    )
    .await;
    assert_eq!(gesendet.status, 201, "{}", gesendet.json);
    id
}

async fn loeschen(r: &Runde, wer: &Teilnehmer, ids: &[&str], bestaetigt: bool) -> ApiAntwort {
    api_antwort(
        wer.absender
            .anfrage(
                reqwest::Method::POST,
                format!("{}api/anhaenge/loeschen", r.instanz.adresse),
            )
            .header("X-LocalchatA-Sitzung", &wer.token)
            .json(&json!({"ids": ids, "bestaetigt": bestaetigt})),
    )
    .await
}

/// Status und Körper, die ID durch `<id>` ersetzt — „byte-gleich“ bis auf die genannte ID.
fn gestalt(antwort: &ApiAntwort, ids: &[&str]) -> (u16, String) {
    let mut text = antwort.json.to_string();
    for id in ids {
        text = text.replace(id, "<id>");
    }
    (antwort.status, text)
}

async fn abruf(r: &Runde, id: &str) -> (u16, Vec<u8>) {
    let antwort = reqwest::get(format!("{}anhaenge/{id}", r.instanz.adresse))
        .await
        .expect("Abruf");
    (
        antwort.status().as_u16(),
        antwort.bytes().await.expect("Körper").to_vec(),
    )
}

/// Zähler, die ein fremdes Löschen nie bewegen darf.
async fn zaehler(r: &Runde) -> Value {
    let health = gesundheit(&r.instanz.adresse).await;
    json!({
        "geloescht": health["anhaenge_geloescht"],
        "belegt": health["anhang_belegt"],
        "ungekuerzt": health["anhaenge_ungekuerzt"],
    })
}

/// A2 (a) und (b): `cora` fragt die ID aus `anna` ↔ `ben` unbestätigt und bestätigt → beide Male 200 `unbekannt`,
/// gleich der Antwort auf eine nie vergebene ID; keine 409-Vorschau; Datei, Zähler, Abruf unverändert. Kontrolle:
/// `ben` bekommt für dieselbe ID die Vorschau.
#[tokio::test]
async fn unbestaetigt_und_bestaetigt_ist_die_fremde_id_unbekannt() {
    let r = runde_anhaenge().await;
    let id = direkt_anhang(&r, b"nur fuer anna und ben").await;
    let vorher = (direkt_stand(r.wurzel.path()), zaehler(&r).await);

    let kontrolle = loeschen(&r, &r.ben, &[&id], false).await;
    let a = loeschen(&r, &r.cora, &[&id], false).await;
    let a_zufall = loeschen(&r, &r.cora, &[ZUFALL], false).await;
    let b = loeschen(&r, &r.cora, &[&id], true).await;
    let b_zufall = loeschen(&r, &r.cora, &[ZUFALL], true).await;

    assert_eq!(
        kontrolle.status, 409,
        "Beteiligte sehen die Vorschau: {}",
        kontrolle.json
    );
    assert_eq!(kontrolle.json["loeschen"]["anhaenge"], 1);
    assert_eq!(a.status, 200, "keine Vorschau für cora: {}", a.json);
    assert_eq!(
        a.json["ergebnisse"][0]["ergebnis"], "unbekannt",
        "{}",
        a.json
    );
    assert_eq!(gestalt(&a, &[&id]), gestalt(&a_zufall, &[ZUFALL]));
    assert_eq!(b.status, 200, "{}", b.json);
    assert_eq!(
        b.json["ergebnisse"][0]["ergebnis"], "unbekannt",
        "{}",
        b.json
    );
    assert_eq!(gestalt(&b, &[&id]), gestalt(&b_zufall, &[ZUFALL]));
    assert_eq!((direkt_stand(r.wurzel.path()), zaehler(&r).await), vorher);
    assert_eq!(
        abruf(&r, &id).await,
        (200, b"nur fuer anna und ben".to_vec())
    );
}

/// A2 (c): nachdem `anna` den Anhang gelöscht hat, ist er für `cora` weiter `unbekannt`, nie `schon_geloescht`;
/// für `ben` dagegen `schon_geloescht`.
#[tokio::test]
async fn nach_dem_loeschen_durch_anna_heisst_es_nicht_schon_geloescht() {
    let r = runde_anhaenge().await;
    let id = direkt_anhang(&r, b"weg damit").await;
    let weg = loeschen(&r, &r.anna, &[&id], true).await;
    assert_eq!(weg.status, 200, "{}", weg.json);
    assert_eq!(weg.json["ergebnisse"][0]["ergebnis"], "geloescht");
    let vorher = (direkt_stand(r.wurzel.path()), zaehler(&r).await);

    let kontrolle = loeschen(&r, &r.ben, &[&id], false).await;
    let ohne = loeschen(&r, &r.cora, &[&id], false).await;
    let ohne_zufall = loeschen(&r, &r.cora, &[ZUFALL], false).await;
    let mit = loeschen(&r, &r.cora, &[&id], true).await;
    let mit_zufall = loeschen(&r, &r.cora, &[ZUFALL], true).await;

    assert_eq!(
        kontrolle.json["ergebnisse"][0]["ergebnis"], "schon_geloescht",
        "Beteiligte sehen es: {}",
        kontrolle.json
    );
    assert_eq!(
        ohne.json["ergebnisse"][0]["ergebnis"], "unbekannt",
        "{}",
        ohne.json
    );
    assert_eq!(gestalt(&ohne, &[&id]), gestalt(&ohne_zufall, &[ZUFALL]));
    assert_eq!(
        mit.json["ergebnisse"][0]["ergebnis"], "unbekannt",
        "{}",
        mit.json
    );
    assert_eq!(gestalt(&mit, &[&id]), gestalt(&mit_zufall, &[ZUFALL]));
    assert_eq!((direkt_stand(r.wurzel.path()), zaehler(&r).await), vorher);
}

/// A2 (d): eine Mischung aus der fremden und einer eigenen ID — die Vorschau zählt nur die eigene (gleich der
/// Vorschau auf die eigene plus eine nie vergebene), die Wirkung löscht nur die eigene; die fremde bleibt `unbekannt`.
#[tokio::test]
async fn eine_mischung_zaehlt_und_loescht_nur_die_eigene() {
    let r = runde_anhaenge().await;
    let fremd = direkt_anhang(&r, b"nur fuer anna und ben").await;
    let eigen = channel_anhang(&r, &r.cora, b"meins").await;
    let vorher = direkt_stand(r.wurzel.path());

    let vorschau = loeschen(&r, &r.cora, &[&fremd, &eigen], false).await;
    let vergleich = loeschen(&r, &r.cora, &[ZUFALL, &eigen], false).await;
    let wirkung = loeschen(&r, &r.cora, &[&fremd, &eigen], true).await;

    assert_eq!(vorschau.status, 409, "{}", vorschau.json);
    assert_eq!(
        vorschau.json["loeschen"]["anhaenge"], 1,
        "{}",
        vorschau.json
    );
    assert_eq!(vorschau.json["loeschen"]["bytes"], 5);
    assert_eq!(vorschau.json["loeschen"]["gespraeche"], 1);
    assert_eq!(vorschau.json["loeschen"]["fremde"], 0);
    assert_eq!(gestalt(&vorschau, &[]), gestalt(&vergleich, &[]));
    assert_eq!(wirkung.status, 200, "{}", wirkung.json);
    assert_eq!(wirkung.json["geloescht"], 1, "{}", wirkung.json);
    assert_eq!(
        wirkung.json["ergebnisse"],
        json!([
            {"id": fremd, "ergebnis": "unbekannt"},
            {"id": eigen, "ergebnis": "geloescht"},
        ])
    );
    assert_eq!(direkt_stand(r.wurzel.path()), vorher);
    assert_eq!(abruf(&r, &fremd).await.0, 200);
    assert_eq!(abruf(&r, &eigen).await.0, 410);
}

/// A2 (e): `cora` und `anna` löschen dieselbe ID gleichzeitig, mehrere Runden mit je neuer ID → `anna` löscht, für
/// `cora` bleibt es in jedem Ausgang `unbekannt` — auch dort, wo ihre Anfrage zeitlich nach der von `anna` die
/// Sperre nimmt.
#[tokio::test]
async fn gleichzeitiges_loeschen_zeigt_cora_nie_schon_geloescht() {
    let r = runde_anhaenge().await;
    for runde in 0..4 {
        let id = direkt_anhang(&r, format!("runde {runde}").as_bytes()).await;
        let ids = [id.as_str()];
        let (von_anna, von_cora) = tokio::join!(
            loeschen(&r, &r.anna, &ids, true),
            loeschen(&r, &r.cora, &ids, true),
        );
        let zufall = loeschen(&r, &r.cora, &[ZUFALL_ZWEI], true).await;

        assert_eq!(
            von_anna.json["ergebnisse"][0]["ergebnis"], "geloescht",
            "{}",
            von_anna.json
        );
        assert_eq!(
            von_cora.json["ergebnisse"][0]["ergebnis"], "unbekannt",
            "{}",
            von_cora.json
        );
        // `platz` ist instanzweit und zeitabhängig: annas Löschen verbucht die Quote nebenher. Das zeigt dieselbe
        // Zahl im Health-Check ohne Sitzung (`anhang_belegt`, Restorakel (f) der ADR-0055) und sagt nichts über das
        // Gespräch; verglichen wird der Rest der Antwort, `platz` nur in seiner Form.
        let ohne_platz = |antwort: &ApiAntwort, id: &str| {
            let mut json = antwort.json.clone();
            let platz = json.as_object_mut().expect("Objekt").remove("platz");
            assert!(platz.is_some_and(|p| p["belegt"].is_u64() && p["frei"].is_u64()));
            (antwort.status, json.to_string().replace(id, "<id>"))
        };
        assert_eq!(ohne_platz(&von_cora, &id), ohne_platz(&zufall, ZUFALL_ZWEI));
        assert_eq!(von_cora.json["geloescht"], 0);
        tokio::time::sleep(Duration::from_millis(600)).await;
    }
}
