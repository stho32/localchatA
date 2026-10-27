//! F0006 Grenzschutz an der Systemgrenze — Angriffsmatrix G1–G20 aus R00003, je Zeile ein Test über das
//! echte Programm. Anfragen gehen roh über TCP, damit `Host`, `Origin` und `Sec-Fetch-Site` genau so
//! ankommen, wie die Zeile sie nennt (auch ohne `Host`).
#![cfg(unix)]

mod gemeinsam;

use gemeinsam::{Instanz, RohAntwort, roh_anfragen, starten, wegwerf_starten};
use std::net::SocketAddr;

const CSP: &str = "default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self' data:; font-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'";

fn ziel(instanz: &Instanz) -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], instanz.port()))
}

/// `GET <pfad>` mit genau den genannten Kopfzeilen.
fn anfragen(instanz: &Instanz, zeile: &str, kopf: &[&str]) -> RohAntwort {
    let mut anfrage = vec![format!("{zeile} HTTP/1.1")];
    anfrage.extend(kopf.iter().map(|zeile| zeile.to_string()));
    roh_anfragen(ziel(instanz), &anfrage.join("\r\n"), "")
}

fn eigener_host(instanz: &Instanz) -> String {
    format!("Host: 127.0.0.1:{}", instanz.port())
}

fn rechnername() -> String {
    let ausgabe = std::process::Command::new("uname")
        .arg("-n")
        .output()
        .expect("uname -n");
    String::from_utf8(ausgabe.stdout)
        .expect("UTF-8")
        .trim_end()
        .to_string()
}

/// Alle sechs Sicherheitsheader mit genau diesen Werten; `no-store` nur, wenn verlangt.
fn header_satz(antwort: &RohAntwort, mit_no_store: bool) {
    assert_eq!(antwort.header("x-content-type-options"), Some("nosniff"));
    assert_eq!(antwort.header("content-security-policy"), Some(CSP));
    assert_eq!(antwort.header("x-frame-options"), Some("DENY"));
    assert_eq!(antwort.header("referrer-policy"), Some("no-referrer"));
    assert_eq!(
        antwort.header("cross-origin-opener-policy"),
        Some("same-origin")
    );
    assert_eq!(
        antwort.header("cross-origin-resource-policy"),
        Some("same-origin")
    );
    let no_store = antwort.header("cache-control") == Some("no-store");
    assert_eq!(
        no_store,
        mit_no_store,
        "{:?}",
        antwort.header("cache-control")
    );
}

fn keine_cors_freigabe(antwort: &RohAntwort) {
    let freigaben: Vec<_> = antwort
        .kopf
        .iter()
        .filter(|(name, _)| name.starts_with("access-control-allow-"))
        .collect();
    assert!(freigaben.is_empty(), "{freigaben:?}");
}

#[test]
fn g1_fremder_host_auf_der_wurzel_ergibt_403_mit_ausweg_und_header_satz() {
    let instanz = wegwerf_starten();

    let antwort = anfragen(&instanz, "GET /", &["Host: evil.example"]);

    assert_eq!(antwort.status, 403);
    assert!(
        antwort.inhalt.contains("evil.example"),
        "{}",
        antwort.inhalt
    );
    assert!(
        antwort.inhalt.contains("--erreichbar-als"),
        "{}",
        antwort.inhalt
    );
    header_satz(&antwort, false);
}

#[test]
fn g2_fremder_host_auf_einer_datei_der_oberflaeche_ergibt_403() {
    let instanz = wegwerf_starten();

    let antwort = anfragen(&instanz, "GET /web/lc-tokens.css", &["Host: evil.example"]);

    assert_eq!(antwort.status, 403);
    assert!(!antwort.inhalt.contains("--kd-bg"), "{}", antwort.inhalt);
}

#[test]
fn g3_fremder_host_unter_api_ergibt_403_mit_header_satz_und_no_store() {
    let instanz = wegwerf_starten();

    let antwort = anfragen(&instanz, "GET /api/gesundheit", &["Host: evil.example"]);

    assert_eq!(antwort.status, 403);
    assert!(!antwort.inhalt.contains("bereit"), "{}", antwort.inhalt);
    header_satz(&antwort, true);
}

#[test]
fn g4_ohne_host_header_ergibt_403() {
    let instanz = wegwerf_starten();

    let antwort = anfragen(&instanz, "GET /api/gesundheit", &[]);

    assert_eq!(antwort.status, 403);
}

#[test]
fn g5_ip_literal_als_praefix_eines_namens_ist_kein_ip_literal() {
    let instanz = wegwerf_starten();
    let host = format!("Host: 127.0.0.1.evil.example:{}", instanz.port());

    let antwort = anfragen(&instanz, "GET /api/gesundheit", &[&host]);

    assert_eq!(antwort.status, 403);
}

#[test]
fn g6_eigenes_ip_literal_ergibt_200() {
    let instanz = wegwerf_starten();

    let antwort = anfragen(&instanz, "GET /api/gesundheit", &[&eigener_host(&instanz)]);

    assert_eq!(antwort.status, 200);
    assert!(antwort.inhalt.contains("bereit"), "{}", antwort.inhalt);
}

#[test]
fn g7_localhost_ergibt_200() {
    let instanz = wegwerf_starten();
    let host = format!("Host: localhost:{}", instanz.port());

    let antwort = anfragen(&instanz, "GET /api/gesundheit", &[&host]);

    assert_eq!(antwort.status, 200);
}

#[test]
fn g8_rechnername_auch_mit_local_und_in_grossbuchstaben_ergibt_200() {
    let instanz = wegwerf_starten();
    let rn = rechnername();
    let port = instanz.port();

    for host in [
        format!("Host: {rn}:{port}"),
        format!("Host: {rn}.local:{port}"),
        format!("Host: {}:{port}", rn.to_uppercase()),
    ] {
        let antwort = anfragen(&instanz, "GET /api/gesundheit", &[&host]);
        assert_eq!(antwort.status, 200, "{host}");
    }
}

#[test]
fn g9_ipv6_literal_in_eckigen_klammern_ergibt_200() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let instanz = starten(
        wurzel.path(),
        &["--port", "0", "--host", "::", "--kein-browser"],
    );
    let ziel = SocketAddr::from((std::net::Ipv6Addr::LOCALHOST, instanz.port()));

    let antwort = roh_anfragen(
        ziel,
        &format!(
            "GET /api/gesundheit HTTP/1.1\r\nHost: [::1]:{}",
            instanz.port()
        ),
        "",
    );

    assert_eq!(antwort.status, 200);
}

#[test]
fn g10_nicht_freigegebener_name_ergibt_403() {
    let instanz = wegwerf_starten();
    let host = format!("Host: chat.lan:{}", instanz.port());

    let antwort = anfragen(&instanz, "GET /", &[&host]);

    assert_eq!(antwort.status, 403);
    assert!(
        antwort.inhalt.contains("--erreichbar-als chat.lan"),
        "{}",
        antwort.inhalt
    );
}

#[test]
fn g11_mit_erreichbar_als_freigegebene_namen_ergeben_200() {
    let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
    let instanz = starten(
        wurzel.path(),
        &[
            "--port",
            "0",
            "--host",
            "127.0.0.1",
            "--kein-browser",
            "--erreichbar-als",
            "chat.lan",
            "--erreichbar-als",
            "nuc",
        ],
    );

    for name in ["chat.lan", "nuc"] {
        let host = format!("Host: {name}:{}", instanz.port());
        let antwort = anfragen(&instanz, "GET /", &[&host]);
        assert_eq!(antwort.status, 200, "{name}");
    }
}

/// `POST /api/gesundheit` mit eigenem Host und den genannten weiteren Kopfzeilen.
fn post_gesundheit(instanz: &Instanz, kopf: &[&str]) -> RohAntwort {
    let host = eigener_host(instanz);
    let mut alle = vec![host.as_str()];
    alle.extend_from_slice(kopf);
    anfragen(instanz, "POST /api/gesundheit", &alle)
}

fn abgewiesen_mit_fehler(antwort: &RohAntwort) {
    assert_eq!(antwort.status, 403);
    let json: serde_json::Value = serde_json::from_str(&antwort.inhalt).expect("JSON");
    assert!(json["fehler"].is_string(), "{json}");
}

#[test]
fn g12_fremder_origin_ergibt_403_mit_fehler() {
    let instanz = wegwerf_starten();

    abgewiesen_mit_fehler(&post_gesundheit(&instanz, &["Origin: http://evil.example"]));
}

#[test]
fn g13_origin_null_ergibt_403() {
    let instanz = wegwerf_starten();

    abgewiesen_mit_fehler(&post_gesundheit(&instanz, &["Origin: null"]));
}

#[test]
fn g14_origin_mit_anderem_port_ergibt_403() {
    let instanz = wegwerf_starten();

    abgewiesen_mit_fehler(&post_gesundheit(
        &instanz,
        &["Origin: http://127.0.0.1:7421"],
    ));
}

#[test]
fn g15_sec_fetch_site_cross_site_ohne_origin_ergibt_403() {
    let instanz = wegwerf_starten();

    abgewiesen_mit_fehler(&post_gesundheit(&instanz, &["Sec-Fetch-Site: cross-site"]));
}

#[test]
fn g16_sec_fetch_site_same_site_ohne_origin_ergibt_403() {
    let instanz = wegwerf_starten();

    abgewiesen_mit_fehler(&post_gesundheit(&instanz, &["Sec-Fetch-Site: same-site"]));
}

#[test]
fn g17_eigener_origin_mit_same_origin_ist_nicht_403_sondern_405() {
    let instanz = wegwerf_starten();
    let origin = format!("Origin: http://127.0.0.1:{}", instanz.port());

    let antwort = post_gesundheit(&instanz, &[&origin, "Sec-Fetch-Site: same-origin"]);

    assert_eq!(antwort.status, 405);
}

#[test]
fn g18_ohne_origin_und_sec_fetch_site_ist_nicht_403_sondern_405() {
    let instanz = wegwerf_starten();

    let antwort = post_gesundheit(&instanz, &[]);

    assert_eq!(antwort.status, 405);
}

#[test]
fn g19_vorabfrage_einer_fremden_website_bekommt_keine_cors_freigabe() {
    let instanz = wegwerf_starten();
    let host = eigener_host(&instanz);

    let antwort = anfragen(
        &instanz,
        "OPTIONS /api/gesundheit",
        &[
            &host,
            "Origin: http://evil.example",
            "Access-Control-Request-Method: POST",
        ],
    );

    keine_cors_freigabe(&antwort);
    assert_ne!(antwort.status / 100, 2, "keine zustimmende Vorabfrage");
}

#[test]
fn g20_200_404_und_403_tragen_je_den_header_satz_die_wurzel_ohne_no_store() {
    let instanz = wegwerf_starten();
    let host = eigener_host(&instanz);

    let wurzel = anfragen(&instanz, "GET /", &[&host]);
    let fehlt = anfragen(&instanz, "GET /web/gibtsnicht.js", &[&host]);
    let fremd = anfragen(&instanz, "GET /", &["Host: evil.example"]);

    assert_eq!((wurzel.status, fehlt.status, fremd.status), (200, 404, 403));
    header_satz(&wurzel, false);
    assert_eq!(wurzel.header("cache-control"), Some("no-cache"));
    header_satz(&fehlt, false);
    header_satz(&fremd, false);
}

/// Eigene Variante: auch 405 unter `/api/` trägt den Satz mit `no-store`.
#[test]
fn variante_405_unter_api_traegt_den_header_satz_mit_no_store() {
    let instanz = wegwerf_starten();

    let antwort = post_gesundheit(&instanz, &[]);

    assert_eq!(antwort.status, 405);
    header_satz(&antwort, true);
}

/// Eigene Variante: doppelte Kopfzeilen gelten nicht als „der eine Wert“ — ein zweiter Host, Origin
/// oder `Sec-Fetch-Site` kann einen erlaubten ersten nicht verstecken.
#[test]
fn variante_doppelte_kopfzeilen_werden_abgewiesen() {
    let instanz = wegwerf_starten();
    let host = eigener_host(&instanz);
    let origin = format!("Origin: http://127.0.0.1:{}", instanz.port());

    let zwei_hosts = anfragen(
        &instanz,
        "GET /api/gesundheit",
        &[&host, "Host: evil.example"],
    );
    let zwei_origins = post_gesundheit(&instanz, &[&origin, "Origin: http://evil.example"]);
    let zwei_sites = post_gesundheit(
        &instanz,
        &["Sec-Fetch-Site: same-origin", "Sec-Fetch-Site: cross-site"],
    );

    assert_ne!(zwei_hosts.status, 200);
    assert_eq!(zwei_origins.status, 403);
    assert_eq!(zwei_sites.status, 403);
}

/// Eigene Variante aus ADR-0014 „Durchsetzung“: die Route mit Wirkung ist unter fremdem Host gesperrt,
/// und die Abweisung belegt keinen Namen.
#[test]
fn variante_beitreten_unter_fremdem_host_ergibt_403_und_belegt_nichts() {
    let instanz = wegwerf_starten();
    let koerper = r#"{"name":"opfer"}"#;
    let senden = |host: &str| {
        roh_anfragen(
            ziel(&instanz),
            &format!("POST /api/beitreten HTTP/1.1\r\n{host}\r\nContent-Type: application/json"),
            koerper,
        )
    };

    let fremd = senden("Host: evil.example");
    let eigen = senden(&eigener_host(&instanz));

    assert_eq!(fremd.status, 403);
    header_satz(&fremd, true);
    assert_eq!(eigen.status, 201, "{}", eigen.inhalt);
}
