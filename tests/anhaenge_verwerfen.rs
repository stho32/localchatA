//! R00029 F0097 „Verwerfen“ über das echte Programm (B0430): `DELETE` der eigenen ID während eines laufenden `PUT`
//! → 204, der `PUT` endet ohne 200, keine `.teil`, Reservierung sofort frei; fremde und unbekannte ID → 204 ohne
//! Wirkung.

mod gemeinsam;

use gemeinsam::anhaenge::{
    Q, anmelden, hochladen, id, im_eingang, roh_antwort, roh_oeffnen, starten,
};
use gemeinsam::senden::{beitreten, ordner};
use gemeinsam::{Absender, api_antwort};
use std::time::Duration;
use tokio::io::AsyncWriteExt;

async fn loeschen(absender: &Absender, basis: &str, token: &str, id: &str) -> u16 {
    api_antwort(
        absender
            .anfrage(reqwest::Method::DELETE, format!("{basis}api/anhaenge/{id}"))
            .header("X-LocalchatA-Sitzung", token),
    )
    .await
    .status
}

#[tokio::test]
async fn verwerfen_waehrend_des_hochladens_gibt_den_platz_sofort_frei() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let alpha = Absender::von("127.0.0.1");
    let token = beitreten(&beta, basis, "beta", Some("agent")).await;
    let token_alpha = beitreten(&alpha, basis, "alpha", Some("agent")).await;
    let id = id(&anmelden(&beta, basis, &token, "abbild.img", Q).await);
    let kopf = format!(
        "PUT /api/anhaenge/{id} HTTP/1.1\r\nHost: 127.0.0.1\r\nX-LocalchatA-Sitzung: {token}\r\nContent-Type: application/octet-stream\r\nContent-Length: {Q}"
    );
    let mut verbindung = roh_oeffnen(instanz.port(), "127.0.0.1", &kopf).await;
    verbindung
        .write_all(&vec![1u8; 300 * 1024])
        .await
        .expect("Anfang");
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(
        anmelden(&alpha, basis, &token_alpha, "x", Q).await.status,
        507,
        "Q ist beansprucht"
    );

    assert_eq!(loeschen(&beta, basis, &token, &id).await, 204);

    let antwort = roh_antwort(&mut verbindung, Duration::from_secs(5)).await;
    assert!(!antwort.starts_with("HTTP/1.1 200"), "{antwort}");
    assert_eq!(
        anmelden(&alpha, basis, &token_alpha, "x", Q).await.status,
        201,
        "sofort frei"
    );
    let bis = std::time::Instant::now() + Duration::from_secs(5);
    while im_eingang(wurzel.path())
        .iter()
        .any(|name| name.ends_with(".teil"))
        && std::time::Instant::now() < bis
    {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(
        im_eingang(wurzel.path()).is_empty(),
        "{:?}",
        im_eingang(wurzel.path())
    );
}

#[tokio::test]
async fn eine_fremde_oder_unbekannte_id_bleibt_unberuehrt() {
    let wurzel = ordner();
    let instanz = starten(wurzel.path(), &[], &[]);
    let basis = instanz.adresse.as_str();
    let beta = Absender::von("127.0.0.1");
    let gamma = Absender::von("127.0.0.1");
    let token = beitreten(&beta, basis, "beta", Some("agent")).await;
    let token_gamma = beitreten(&gamma, basis, "gamma", Some("agent")).await;
    let id_gamma = id(&anmelden(&gamma, basis, &token_gamma, "g.txt", 10).await);

    assert_eq!(loeschen(&beta, basis, &token, &id_gamma).await, 204);
    assert_eq!(
        loeschen(&beta, basis, &token, "aaaaaaaaaaaaaaaaaaaaaaaaaa").await,
        204
    );
    assert_eq!(loeschen(&beta, basis, &token, "kein-id").await, 204);

    let hoch = hochladen(&gamma, basis, &token_gamma, &id_gamma, vec![b'g'; 10]).await;
    assert_eq!(hoch.status, 200, "gamma lädt danach hoch: {}", hoch.json);
}
