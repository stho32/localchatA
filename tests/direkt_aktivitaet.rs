//! R00050 F0173 B0773 — Tippen und „denkt nach“ im Direktgespräch über das echte Programm (ADR-0055 Regeln 3 und 6,
//! ADR-0051 Regel 10; Matrix X1, X2, K3): `PUT|DELETE|GET /api/direkt/{kennung}/aktivitaet` — das Signal geht nur an
//! das Gegenüber, nie an den Absender (auch nicht im zweiten Tab) und nie an einen Dritten; ohne bestehendes Gespräch
//! ist die Antwort ein 404 wie für ein nie vergebenes Gespräch.
#![cfg(target_os = "linux")]

mod gemeinsam;

use gemeinsam::direkt::{ANNA, BEN, CORA, Runde, Teilnehmer, direkt_eintraege, runde};
use gemeinsam::senden::{beitreten, ordner};
use gemeinsam::strom::{Empfangen, Strom};
use gemeinsam::{Absender, ApiAntwort};
use reqwest::Method;
use serde_json::json;
use std::time::Duration;
use tokio::time::Instant;

/// Erscheinen binnen 1 s nach der 200 (R00045 „Erscheinen“).
const ERSCHEINEN: Duration = Duration::from_secs(1);
/// So lange wird auf etwas gewartet, das nicht kommen darf (Matrix X1: „nichts in 3 s“).
const STILLE: Duration = Duration::from_secs(3);

fn ist_aktivitaet(e: &Empfangen) -> bool {
    e.data["art"] == "aktivitaet"
}

async fn strom_offen(wer: &Teilnehmer, basis: &str) -> Strom {
    let mut strom = Strom::oeffnen(&wer.absender, basis, &wer.token).await;
    strom
        .bis(Duration::from_secs(5), |e| e.data["art"] == "channels")
        .await
        .expect("Stand der Channels");
    strom
}

async fn tippt(wer: &Teilnehmer, basis: &str, gegenueber: &str) -> ApiAntwort {
    wer.aufrufen(
        Method::PUT,
        basis,
        gegenueber,
        "aktivitaet",
        Some(&json!({"zustand": "tippt"})),
    )
    .await
}

async fn lesen(wer: &Teilnehmer, basis: &str, gegenueber: &str) -> ApiAntwort {
    wer.aufrufen(Method::GET, basis, gegenueber, "aktivitaet", None)
        .await
}

/// Das Gespräch besteht, wenn eine Nachricht in der Frist liegt: `anna` und `ben` haben sich geschrieben.
async fn gespraech_anna_ben(r: &Runde, basis: &str) {
    let antwort = r.anna.senden(basis, BEN, &json!({"text": "hallo"})).await;
    assert_eq!(antwort.status, 201, "{}", antwort.json);
}

/// X1 (wörtlich): `anna` setzt `tippt` im Gespräch mit `ben`; Ströme `ben`, `cora` und `anna` (zweiter Tab) →
/// `ben`: `{art: aktivitaet, channel: null, direkt: anna@…}` binnen 1 s; `cora` und `anna`: nichts in 3 s. Danach
/// liest `ben` den Zustand mit `GET`, `anna` nicht ihren eigenen, und `DELETE` beendet ihn im Strom von `ben`.
#[tokio::test]
async fn x1_tippen_im_direktgespraech_sieht_nur_das_gegenueber() {
    let r = runde(&[]).await;
    let basis = r.instanz.adresse.clone();
    gespraech_anna_ben(&r, &basis).await;
    let mut strom_ben = strom_offen(&r.ben, &basis).await;
    let mut strom_cora = strom_offen(&r.cora, &basis).await;
    let mut strom_anna = strom_offen(&r.anna, &basis).await;

    let gesetzt = tippt(&r.anna, &basis, BEN).await;
    let nach_der_antwort = Instant::now();
    let bei_ben = strom_ben
        .bis(Duration::from_secs(5), ist_aktivitaet)
        .await
        .expect("aktivitaet bei ben");
    let (cora, anna) = tokio::join!(
        strom_cora.bis(STILLE, ist_aktivitaet),
        strom_anna.bis(STILLE, ist_aktivitaet)
    );
    let ben_liest = lesen(&r.ben, &basis, ANNA).await;
    let anna_liest = lesen(&r.anna, &basis, BEN).await;
    let geloescht = r
        .anna
        .aufrufen(Method::DELETE, &basis, BEN, "aktivitaet", None)
        .await;
    let ende_bei_ben = strom_ben
        .bis(Duration::from_secs(5), |e| {
            ist_aktivitaet(e) && e.data["zustand"].is_null()
        })
        .await
        .expect("Ende bei ben");

    assert_eq!(gesetzt.status, 200, "{}", gesetzt.json);
    assert_eq!(gesetzt.json["zustand"], "tippt");
    let dauer = bei_ben.ankunft.saturating_duration_since(nach_der_antwort);
    assert!(dauer <= ERSCHEINEN, "{dauer:?}");
    assert_eq!(
        bei_ben.data,
        json!({
            "art": "aktivitaet", "channel": null, "direkt": ANNA, "kennung": ANNA,
            "zustand": "tippt", "bis": gesetzt.json["bis"], "jetzt": gesetzt.json["jetzt"]
        })
    );
    assert!(cora.is_none(), "cora bekam {cora:?}");
    assert!(anna.is_none(), "anna bekam im zweiten Tab {anna:?}");
    assert_eq!(ben_liest.status, 200, "{}", ben_liest.json);
    assert_eq!(
        ben_liest.json["aktivitaeten"],
        json!([{"kennung": ANNA, "zustand": "tippt", "bis": gesetzt.json["bis"]}])
    );
    assert_eq!(
        anna_liest.json["aktivitaeten"],
        json!([]),
        "nie sich selbst"
    );
    assert_eq!(geloescht.status, 204);
    assert_eq!(ende_bei_ben.data["direkt"], ANNA);
    assert_eq!(ende_bei_ben.data["channel"], json!(null));
}

/// X2 (wörtlich): `cora` liest `…/anna@…/aktivitaet`, während `anna` an `ben` tippt, und setzt an `anna` ohne
/// Gespräch → 404, byte-gleich zu einem nie vergebenen Gespräch, kein Ordner. Mit eigenem Gespräch `cora` ↔ `anna`
/// liest sie nur dessen Zustand — nie `annas` Tippen an `ben`. `selbst` und unzulässige Kennung sind 422.
#[tokio::test]
async fn x2_ein_dritter_liest_und_setzt_nur_in_seinem_eigenen_gespraech() {
    let r = runde(&[]).await;
    let basis = r.instanz.adresse.clone();
    gespraech_anna_ben(&r, &basis).await;
    assert_eq!(tippt(&r.anna, &basis, BEN).await.status, 200);
    let ordner_vorher = direkt_eintraege(r.wurzel.path());
    let mut strom_anna = strom_offen(&r.anna, &basis).await;
    let mut strom_ben = strom_offen(&r.ben, &basis).await;

    let lesen_ohne = lesen(&r.cora, &basis, ANNA).await;
    let setzen_ohne = tippt(&r.cora, &basis, ANNA).await;
    let loeschen_ohne = r
        .cora
        .aufrufen(Method::DELETE, &basis, ANNA, "aktivitaet", None)
        .await;
    let nie = lesen(&r.cora, &basis, "zed@127.0.0.9").await;
    let selbst = tippt(&r.cora, &basis, CORA).await;
    let unzulaessig = tippt(&r.cora, &basis, "anna").await;
    let ordner_nachher = direkt_eintraege(r.wurzel.path());

    for ohne in [&lesen_ohne, &setzen_ohne, &loeschen_ohne, &nie] {
        assert_eq!(ohne.status, 404, "{}", ohne.json);
        assert_eq!(ohne.json, nie.json, "byte-gleich zu nie vergeben");
    }
    assert_eq!(selbst.status, 422, "{}", selbst.json);
    assert_eq!(selbst.json["regel"], "selbst");
    assert_eq!(unzulaessig.status, 422, "{}", unzulaessig.json);
    assert_eq!(unzulaessig.json["regel"], "kennung");
    assert_eq!(ordner_vorher, ordner_nachher, "kein Ordner für cora");

    let antwort = r.cora.senden(&basis, ANNA, &json!({"text": "hi"})).await;
    assert_eq!(antwort.status, 201, "{}", antwort.json);
    let eigenes = lesen(&r.cora, &basis, ANNA).await;
    assert_eq!(eigenes.status, 200, "{}", eigenes.json);
    assert_eq!(
        eigenes.json["aktivitaeten"],
        json!([]),
        "annas Tippen an ben gehört cora nicht"
    );
    assert_eq!(tippt(&r.cora, &basis, ANNA).await.status, 200);
    let bei_anna = strom_anna
        .bis(Duration::from_secs(5), ist_aktivitaet)
        .await
        .expect("aktivitaet bei anna");
    let bei_ben = strom_ben.bis(STILLE, ist_aktivitaet).await;
    assert_eq!(bei_anna.data["direkt"], CORA);
    assert_eq!(bei_anna.data["kennung"], CORA);
    assert!(bei_ben.is_none(), "ben bekam {bei_ben:?}");
}

/// K3 (wörtlich): Instanz `--host ::`; `anna` über `::1` (Kennung `anna@::1`), `ben` über 127.0.0.1; senden, lesen,
/// Tippen → Ordner `anna@%3A%3A1+ben@127.0.0.1`; `anna` sieht Verlauf, Strom und Tippen von `ben` (der alte Vergleich
/// auf dem kodierten Ordnernamen schloss einen IPv6-Beteiligten aus).
#[tokio::test]
async fn k3_ein_ipv6_beteiligter_sieht_verlauf_strom_und_tippen() {
    let wurzel = ordner();
    let instanz = gemeinsam::starten_mit(
        wurzel.path(),
        &["--port", "0", "--host", "::", "--kein-browser"],
        &[
            ("TZ", "Europe/Berlin"),
            ("LOCALCHATA_JETZT", gemeinsam::senden::JETZT),
        ],
    );
    let port = instanz.port();
    let v6 = format!("http://[::1]:{port}/");
    let v4 = format!("http://127.0.0.1:{port}/");
    let (absender_anna, absender_ben) = (Absender::von("::1"), Absender::von("127.0.0.1"));
    let anna = Teilnehmer {
        token: beitreten(&absender_anna, &v6, "anna", None).await,
        absender: absender_anna,
    };
    let ben = Teilnehmer {
        token: beitreten(&absender_ben, &v4, "ben", None).await,
        absender: absender_ben,
    };
    let anna_kennung = "anna@::1";
    let ben_kennung = "ben@127.0.0.1";
    let mut strom_anna = strom_offen(&anna, &v6).await;

    let gesendet = ben
        .senden(&v4, anna_kennung, &json!({"text": "hallo ipv6"}))
        .await;
    let verlauf = anna.lesen(&v6, ben_kennung).await;
    let tippt_ben = ben
        .aufrufen(
            Method::PUT,
            &v4,
            anna_kennung,
            "aktivitaet",
            Some(&json!({"zustand": "tippt"})),
        )
        .await;
    let nachricht = strom_anna
        .bis(Duration::from_secs(5), |e| e.data["art"] == "nachricht")
        .await
        .expect("nachricht bei anna");
    let aktivitaet = strom_anna
        .bis(Duration::from_secs(5), ist_aktivitaet)
        .await
        .expect("aktivitaet bei anna");
    let anna_liest = lesen(&anna, &v6, ben_kennung).await;

    assert_eq!(gesendet.status, 201, "{}", gesendet.json);
    assert_eq!(gesendet.json["direkt"], anna_kennung);
    assert_eq!(
        direkt_eintraege(wurzel.path()),
        ["anna@%3A%3A1+ben@127.0.0.1"]
    );
    assert_eq!(verlauf.status, 200, "{}", verlauf.json);
    assert_eq!(verlauf.json["nachrichten"][0]["text"], "hallo ipv6");
    assert_eq!(tippt_ben.status, 200, "{}", tippt_ben.json);
    assert_eq!(nachricht.data["direkt"], ben_kennung);
    assert_eq!(aktivitaet.data["direkt"], ben_kennung);
    assert_eq!(aktivitaet.data["zustand"], "tippt");
    assert_eq!(
        anna_liest.json["aktivitaeten"][0]["kennung"], ben_kennung,
        "{}",
        anna_liest.json
    );
}
