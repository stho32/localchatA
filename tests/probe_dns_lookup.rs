//! Probe für `dns-lookup` (Skill dependency-probe; Bedingung aus ADR-0045, B0364): die Annahmen, auf denen die
//! echte Namensauskunft steht.
//!
//! Annahmen: (1) `lookup_addr` liefert für eine Adresse mit Rückwärtseintrag ihren Namen (`127.0.0.1` →
//! `localhost`); (2) für eine Adresse ohne Eintrag — und bei einem Resolver, der nie antwortet — einen Fehler
//! (`NoName` bzw. `Again`) statt einer Panik, und der Faden endet nach der Zeitgrenze des Resolvers, nie
//! unbegrenzt; (3) die Vorwärtsauflösung der Standardbibliothek endet für einen nicht vorhandenen `.local`-Namen
//! ebenso (mit `nss-mdns` nach etwa 5 s); (4) im LAN liefert `lookup_addr` für die eigene Adresse denselben Namen
//! wie `getent hosts`.
//!
//! Gemessen am 2026-10-29 auf `nuc5` (Ubuntu 25.04, glibc 2.41, `hosts: files mdns4_minimal [NOTFOUND=return] dns`):
//! `127.0.0.1` → `localhost` 0,6 ms; `192.168.178.65` → `nuc5.fritz.box` 3,5 ms; `::1` → `localhost`;
//! `10.255.255.1` → `NoName` 3 ms; `192.0.2.1` → `NoName` 25 ms; `fe80::1` → `Again` 2 ms; `gibtsnicht.local`
//! vorwärts → Fehler nach 5,0 s. Im Wegwerf-Container (Debian 13, glibc 2.41) mit `--dns 192.0.2.1`, einem
//! Resolver, der nie antwortet: `10.255.255.1` → `Again` nach 10,0 s (2 Versuche × 5 s), der Faden endet;
//! `127.0.0.1` → `localhost` aus `/etc/hosts` ohne Wartezeit.

use std::net::{IpAddr, ToSocketAddrs};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// Höchstens so lange darf ein Faden brauchen: die Zeitgrenze von glibc (2 Versuche × 5 s, ohne Suchdomäne)
/// mit Reserve für eine belastete Maschine.
const FADEN_ENDET_SPAETESTENS: Duration = Duration::from_secs(30);

/// Führt `frage` in einem eigenen Faden aus und liefert ihr Ergebnis — oder `None`, wenn der Faden nicht
/// rechtzeitig endet oder in Panik geriet.
fn im_faden<T: Send + 'static>(frage: impl FnOnce() -> T + Send + 'static) -> Option<T> {
    let (senden, empfangen) = mpsc::channel();
    thread::spawn(move || {
        let _ = senden.send(frage());
    });
    empfangen.recv_timeout(FADEN_ENDET_SPAETESTENS).ok()
}

fn rueckwaerts(adresse: &str) -> Option<Result<String, dns_lookup::LookupError>> {
    let adresse: IpAddr = adresse.parse().expect("Adresse");
    im_faden(move || dns_lookup::lookup_addr(&adresse))
}

#[test]
fn probe_rueckwaerts_loopback_liefert_localhost() {
    let ergebnis = rueckwaerts("127.0.0.1").expect("Faden endet ohne Panik");

    assert_eq!(ergebnis.expect("Name"), "localhost");
}

#[test]
fn probe_rueckwaerts_ohne_eintrag_ist_ein_fehler_ohne_panik_und_der_faden_endet() {
    // 192.0.2.1 ist TEST-NET-1 (RFC 5737) und nie vergeben; ein Resolver ohne Antwort ergibt `Again`.
    let ergebnis = rueckwaerts("192.0.2.1").expect("Faden endet ohne Panik");

    let fehler = ergebnis.expect_err("kein Name für eine nicht vergebene Adresse");
    assert!(
        matches!(
            fehler.kind(),
            dns_lookup::LookupErrorKind::NoName | dns_lookup::LookupErrorKind::Again
        ),
        "{fehler:?}"
    );
}

#[test]
fn probe_vorwaerts_nicht_vorhandener_local_name_endet_mit_fehler() {
    let ergebnis = im_faden(|| {
        ("gibtsnicht-localchata-probe.local", 0)
            .to_socket_addrs()
            .map(|adressen| adressen.count())
    })
    .expect("Faden endet ohne Panik");

    assert!(ergebnis.is_err(), "{ergebnis:?}");
}

/// Nur im LAN (`cargo test --test probe_dns_lookup -- --ignored`): braucht eine Adresse mit Rückwärtseintrag.
#[test]
#[ignore = "braucht eine LAN-Adresse mit Rückwärtseintrag"]
fn probe_rueckwaerts_der_lan_adresse_ist_der_name_aus_getent() {
    let sonde = std::net::UdpSocket::bind("0.0.0.0:0").expect("UDP-Sonde");
    sonde.connect("192.0.2.1:9").expect("Route ins LAN");
    let lan = sonde.local_addr().expect("lokale Adresse").ip();
    let getent = std::process::Command::new("getent")
        .args(["hosts", &lan.to_string()])
        .output()
        .expect("getent");
    let erwartet = String::from_utf8(getent.stdout).expect("UTF-8");
    let erwartet = erwartet.split_whitespace().nth(1).expect("Name aus getent");

    let ergebnis = rueckwaerts(&lan.to_string()).expect("Faden endet ohne Panik");

    assert_eq!(ergebnis.expect("Name"), erwartet);
}
