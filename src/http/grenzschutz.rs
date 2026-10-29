//! Grenzschutz (ADR-0014): welche Host-Namen die Instanz anspricht (gegen DNS-Rebinding) und von welchem
//! Ursprung sie Wirkung annimmt (gegen CSRF).

use super::abweisungsseite;
use super::auskunft::{Abfrage, Auskunft};
use super::hostnamen::{
    FesteNamen, Grund, host_urteil, im_lokalen_namensraum, normalform, zeigt_auf_eigene,
};
use super::namensauskunft::Namensauskunft;
use crate::protokoll::episoden::Auftreten;
use crate::protokoll::{Bereich, Kontext, Protokoll, Schluessel, Stufe};
use crate::start::ausliefern::LokaleAdresse;
use axum::Json;
use axum::extract::{ConnectInfo, Request, State};
use axum::http::header::{
    ACCEPT, CACHE_CONTROL, CONTENT_SECURITY_POLICY, CONTENT_TYPE, HOST, HeaderName, HeaderValue,
    ORIGIN, REFERRER_POLICY, X_CONTENT_TYPE_OPTIONS, X_FRAME_OPTIONS,
};
use axum::http::{HeaderMap, Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;

/// Die äußerste Schicht vor jeder Route (auch 404 und 405): Host prüfen (403 mit Ausweg als Text),
/// Ursprung prüfen (403 `{fehler}`), dann die Route; auf jede Antwort kommt der Header-Satz.
pub async fn schicht(
    State((schutz, protokoll)): State<(Arc<Grenzschutz>, Arc<Protokoll>)>,
    anfrage: Request,
    weiter: Next,
) -> Response {
    let pfad = anfrage.uri().path().to_string();
    let host = einziger_wert(anfrage.headers(), &HOST).map(str::to_string);
    let lokal = anfrage
        .extensions()
        .get::<LokaleAdresse>()
        .map(|LokaleAdresse(lokal)| *lokal);
    let mut antwort = match abweisung(&schutz, anfrage.method(), anfrage.headers(), lokal).await {
        Some((abweisung, ereignis, kopf)) => {
            abweisung_eintragen(&protokoll, &anfrage, ereignis, &kopf);
            abweisung
        }
        None => weiter.run(anfrage).await,
    };
    sicherheitsheader_setzen(antwort.headers_mut(), &pfad, host.as_deref());
    antwort
}

/// Episode `grenzschutz` je Herkunft (ADR-0042 Regel 2): `grund` ist der abgewiesene Kopf mit seinem Wert, so
/// wie er kam (escaped und gekürzt erst in der Zeile).
fn abweisung_eintragen(
    protokoll: &Protokoll,
    anfrage: &Request,
    ereignis: &'static str,
    kopf: &HeaderName,
) {
    let wert = anfrage
        .headers()
        .get_all(kopf)
        .iter()
        .map(|wert| String::from_utf8_lossy(wert.as_bytes()).into_owned())
        .collect::<Vec<_>>()
        .join(", ");
    let herkunft = anfrage
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map_or_else(
            || "unbekannt".to_string(),
            |ConnectInfo(gegenstelle)| gegenstelle.ip().to_string(),
        );
    protokoll.episode(Auftreten {
        stufe: Stufe::Warning,
        bereich: Bereich::Grenzschutz,
        text: ereignis,
        quelle: herkunft.clone(),
        kontext: Kontext::neu()
            .mit(Schluessel::Herkunft, herkunft)
            .mit(Schluessel::Grund, format!("{kopf}: {wert}")),
        art: None,
        verworfen: None,
        bytes: None,
        anzeige: None,
    });
}

/// Die Antwort auf eine Abweisung, dazu Ereignis und Kopf, an dem sie hing.
async fn abweisung(
    schutz: &Grenzschutz,
    methode: &Method,
    kopf: &HeaderMap,
    lokal: Option<SocketAddr>,
) -> Option<(Response, &'static str, HeaderName)> {
    let host = einziger_wert(kopf, &HOST);
    if let Err(abgewiesen) = schutz
        .host_pruefen(host, lokal.map(|lokal| lokal.ip()))
        .await
    {
        return Some((
            host_abweisung(&abgewiesen, kopf, lokal),
            "Anfrage mit fremdem Host abgewiesen",
            HOST,
        ));
    }
    let sec_fetch_site = HeaderName::from_static("sec-fetch-site");
    let ursprung = ursprung_erlaubt(
        methode,
        einziger_wert(kopf, &ORIGIN),
        einziger_wert(kopf, &sec_fetch_site),
        host,
    );
    match ursprung {
        Ok(()) => None,
        Err(abgewiesen) => {
            let fehler = Json(json!({ "fehler": abgewiesen.to_string() }));
            Some((
                (StatusCode::FORBIDDEN, fehler).into_response(),
                "Anfrage mit fremdem Ursprung abgewiesen",
                ORIGIN,
            ))
        }
    }
}

/// 403 für einen abgewiesenen Host (ADR-0045 Regel 7, E4): ein Browser-Dokument bekommt die Seite mit dem Link
/// auf die lokale Adresse (ohne Namen, wenn der Host fehlt oder leer ist), alle anderen den Text; ein doppelter
/// `Host` nie die Seite. Kein Redirect.
fn host_abweisung(
    abgewiesen: &HostAbgewiesen,
    kopf: &HeaderMap,
    lokal: Option<SocketAddr>,
) -> Response {
    let name = abgewiesen.name.as_deref().filter(|name| !name.is_empty());
    let doppelt = kopf.get_all(HOST).iter().count() > 1;
    match lokal {
        Some(lokal) if browser_dokument(kopf) && !doppelt => {
            let html = [(CONTENT_TYPE, "text/html; charset=utf-8")];
            (
                StatusCode::FORBIDDEN,
                html,
                abweisungsseite::seite(name, lokal),
            )
                .into_response()
        }
        _ => {
            let text = [(CONTENT_TYPE, "text/plain; charset=utf-8")];
            let name = if doppelt {
                abgewiesen.name.as_deref()
            } else {
                name
            };
            (
                StatusCode::FORBIDDEN,
                text,
                abweisungsseite::text(name, lokal),
            )
                .into_response()
        }
    }
}

/// Fragt ein Browser nach einem Dokument? `Sec-Fetch-Dest: document` oder `Accept` mit `text/html`.
fn browser_dokument(kopf: &HeaderMap) -> bool {
    let ziel = HeaderName::from_static("sec-fetch-dest");
    let dokument = kopf
        .get_all(ziel)
        .iter()
        .any(|wert| wert.as_bytes().eq_ignore_ascii_case(b"document"));
    let html = kopf.get_all(ACCEPT).iter().any(|wert| {
        String::from_utf8_lossy(wert.as_bytes())
            .to_ascii_lowercase()
            .contains("text/html")
    });
    dokument || html
}

/// Der Wert einer Kopfzeile, die höchstens einmal vorkommen darf. Mehrfach oder kein Text gilt als
/// leerer Wert — den lässt keine Prüfung durch, ein erlaubter erster Wert versteckt so keinen zweiten.
pub(super) fn einziger_wert<'a>(kopf: &'a HeaderMap, name: &HeaderName) -> Option<&'a str> {
    let mut werte = kopf.get_all(name).iter();
    let erster = werte.next()?;
    if werte.next().is_some() {
        return Some("");
    }
    Some(erster.to_str().unwrap_or_default())
}

/// Welche Host-Namen gelten (ADR-0045): die beim Start festgelegten Namen und, über die Auskunft mit Cache, der
/// Rückwärtsname der angesprochenen Adresse.
#[derive(Clone)]
pub struct Grenzschutz {
    feste: FesteNamen,
    auskunft: Option<Arc<Auskunft>>,
    alle_namen: bool,
}

/// Der Host ist nicht freigegeben; `name` ist der Host-Anteil ohne Port, `None` heißt: kein Host-Header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostAbgewiesen {
    pub name: Option<String>,
}

impl Grenzschutz {
    /// Nur die festen Namen; ohne Auskunft wird nichts aufgelöst.
    pub fn neu(rechnername: Option<String>, erreichbar_als: Vec<String>) -> Self {
        Grenzschutz {
            feste: FesteNamen::neu(rechnername.as_deref(), &erreichbar_als),
            auskunft: None,
            alle_namen: false,
        }
    }

    /// `--alle-namen` (Regel 9): jeder Name in Normalform gilt, die Auskunft wird nie gefragt; die
    /// Ursprungsregel bleibt.
    pub fn mit_alle_namen(mut self, alle_namen: bool) -> Self {
        self.alle_namen = alle_namen;
        self
    }

    /// Ist `--alle-namen` an? (Health-Check)
    pub fn alle_namen(&self) -> bool {
        self.alle_namen
    }

    pub fn mit_auskunft(mut self, auskunft: Arc<Auskunft>) -> Self {
        self.auskunft = Some(auskunft);
        self
    }

    /// Urteil über den `Host` einer Anfrage, die über die lokale Adresse `lokal` kam: IP-Literal erlaubt; ein
    /// Name nur in Normalform (Regel 1), dann fest (Regel 2), als Rückwärtsname von `lokal` (Regel 3) oder im
    /// lokalen Namensraum mit eigener Adresse (Regel 4) — nur dort wird vorwärts aufgelöst; alles andere
    /// abgewiesen (Regel 5). Ein Wert, der sich nicht als Host mit optionalem Port lesen lässt, nie.
    pub async fn host_pruefen(
        &self,
        host: Option<&str>,
        lokal: Option<IpAddr>,
    ) -> Result<Grund, HostAbgewiesen> {
        let Some(wert) = host else {
            return Err(HostAbgewiesen { name: None });
        };
        let name = match host_anteil(wert) {
            Some(Hostanteil::IpLiteral) => return Ok(Grund::Fest),
            Some(Hostanteil::Name(name)) => name,
            None => {
                return Err(HostAbgewiesen {
                    name: Some(wert.to_string()),
                });
            }
        };
        let abgewiesen = || HostAbgewiesen {
            name: Some(name.to_string()),
        };
        let normal = normalform(name).ok_or_else(abgewiesen)?;
        if self.alle_namen {
            return Ok(Grund::AlleNamen);
        }
        if let Some(grund) = host_urteil(&normal, &self.feste, Vec::new) {
            return Ok(grund);
        }
        let (Some(auskunft), Some(lokal)) = (&self.auskunft, lokal) else {
            return Err(abgewiesen());
        };
        let lokal = lokal.to_canonical();
        let namen = auskunft.namen_der_adresse(lokal).await.unwrap_or_default();
        if let Some(grund) = host_urteil(&normal, &self.feste, || namen) {
            auskunft.erlaubt_melden(&Abfrage::Rueckwaerts(lokal), &normal, lokal);
            return Ok(grund);
        }
        if !im_lokalen_namensraum(&normal) {
            return Err(abgewiesen());
        }
        let name = normal.clone();
        let urteilen = move |port: &dyn Namensauskunft| {
            let adressen = port.adressen_des_namens(&name);
            zeigt_auf_eigene(&adressen, &mut |adresse| port.ist_eigene_adresse(adresse))
        };
        if auskunft.urteil_ueber(&normal, urteilen).await != Some(true) {
            return Err(abgewiesen());
        }
        auskunft.erlaubt_melden(&Abfrage::Vorwaerts(normal.clone()), &normal, lokal);
        Ok(Grund::LokalerNamensraum)
    }
}

/// Eine nicht lesende Anfrage kommt von einem fremden Ursprung; `grund` nennt den Header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UrsprungAbgewiesen {
    pub grund: String,
}

/// Lesende Methoden (`GET`, `HEAD`, `OPTIONS`) sind wirkungsfrei und immer erlaubt. Jede andere braucht:
/// `Origin` fehlt oder ist genau `http://<Host>` (Name ohne Groß-/Kleinschreibung, `null` nie), und
/// `Sec-Fetch-Site` fehlt oder ist `same-origin` bzw. `none`. Fehlen beide, kommt sie nicht vom Browser.
pub fn ursprung_erlaubt(
    methode: &Method,
    origin: Option<&str>,
    sec_fetch_site: Option<&str>,
    host: Option<&str>,
) -> Result<(), UrsprungAbgewiesen> {
    let lesend = [Method::GET, Method::HEAD, Method::OPTIONS].contains(methode);
    if lesend {
        return Ok(());
    }
    if let Some(origin) = origin {
        let eigener_ursprung = host.map(|host| format!("http://{host}"));
        let ist_eigener =
            eigener_ursprung.is_some_and(|eigener| eigener.eq_ignore_ascii_case(origin));
        if !ist_eigener {
            return Err(UrsprungAbgewiesen {
                grund: format!("Origin {origin:?} ist nicht der Ursprung dieser Instanz"),
            });
        }
    }
    if let Some(site) = sec_fetch_site
        && !["same-origin", "none"].contains(&site)
    {
        return Err(UrsprungAbgewiesen {
            grund: format!("Sec-Fetch-Site {site:?} ist nicht same-origin"),
        });
    }
    Ok(())
}

impl fmt::Display for UrsprungAbgewiesen {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Anfrage von einer fremden Website abgewiesen — {}.",
            self.grund
        )
    }
}

/// Setzt den Header-Satz aus ADR-0014 Regel 5 auf eine Antwort (vorhandene Werte werden ersetzt), unter
/// `/api/` zusätzlich `Cache-Control: no-store`, und entfernt jede CORS-Freigabe (Regel 4).
/// `Cross-Origin-Opener-Policy` nur unter einem vertrauenswürdigen Ursprung: sonst ignoriert der Browser
/// den Header (HTML, „obtain an opener policy“, Schritt 2) und Chromium meldet das bei jedem Laden als
/// Konsolenfehler — über die LAN-Adresse, auf die die Vision zielt, wäre er nur Lärm.
pub fn sicherheitsheader_setzen(kopf: &mut HeaderMap, pfad: &str, host: Option<&str>) {
    let cors_freigaben: Vec<HeaderName> = kopf
        .keys()
        .filter(|name| name.as_str().starts_with("access-control-allow-"))
        .cloned()
        .collect();
    for name in cors_freigaben {
        kopf.remove(name);
    }
    for (name, wert) in HEADER_SATZ {
        kopf.insert(name, HeaderValue::from_static(wert));
    }
    let opener = HeaderName::from_static("cross-origin-opener-policy");
    if ursprung_vertrauenswuerdig(host) {
        kopf.insert(opener, HeaderValue::from_static("same-origin"));
    } else {
        kopf.remove(opener);
    }
    if pfad.starts_with("/api/") {
        kopf.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    }
}

const HEADER_SATZ: [(HeaderName, &str); 5] = [
    (X_CONTENT_TYPE_OPTIONS, "nosniff"),
    (
        CONTENT_SECURITY_POLICY,
        "default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self' data:; font-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'",
    ),
    (X_FRAME_OPTIONS, "DENY"),
    (REFERRER_POLICY, "no-referrer"),
    (
        HeaderName::from_static("cross-origin-resource-policy"),
        "same-origin",
    ),
];

/// Ein `http`-Ursprung ist nur auf Loopback vertrauenswürdig (Secure Contexts, „potentially trustworthy
/// origin“): `127.0.0.0/8`, `[::1]`, `localhost` und `*.localhost`.
fn ursprung_vertrauenswuerdig(host: Option<&str>) -> bool {
    let Some(wert) = host else {
        return false;
    };
    if let Some(rest) = wert.strip_prefix('[') {
        let adresse = rest.split_once(']').map(|(adresse, _)| adresse);
        return adresse
            .and_then(|adresse| adresse.parse::<Ipv6Addr>().ok())
            .is_some_and(|ip| ip.is_loopback());
    }
    let name = wert.split(':').next().unwrap_or_default();
    if let Ok(ip) = name.parse::<Ipv4Addr>() {
        return ip.is_loopback();
    }
    let klein = name.to_ascii_lowercase();
    klein == "localhost" || klein.ends_with(".localhost")
}

enum Hostanteil<'a> {
    IpLiteral,
    Name(&'a str),
}

/// `Host`-Wert → Host-Anteil: IPv6 nur in eckigen Klammern, danach höchstens `:<ziffern>`.
fn host_anteil(wert: &str) -> Option<Hostanteil<'_>> {
    if let Some(rest) = wert.strip_prefix('[') {
        let (adresse, nach) = rest.split_once(']')?;
        adresse.parse::<Ipv6Addr>().ok()?;
        return port_lesbar(nach).then_some(Hostanteil::IpLiteral);
    }
    let (name, nach) = match wert.find(':') {
        Some(stelle) => wert.split_at(stelle),
        None => (wert, ""),
    };
    if !port_lesbar(nach) {
        return None;
    }
    if name.parse::<Ipv4Addr>().is_ok() {
        return Some(Hostanteil::IpLiteral);
    }
    Some(Hostanteil::Name(name))
}

/// Leer oder `:` gefolgt von mindestens einer Ziffer und nur Ziffern.
fn port_lesbar(nach_dem_host: &str) -> bool {
    match nach_dem_host.strip_prefix(':') {
        Some(port) => !port.is_empty() && port.bytes().all(|zeichen| zeichen.is_ascii_digit()),
        None => nach_dem_host.is_empty(),
    }
}

impl fmt::Display for HostAbgewiesen {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.name {
            Some(name) => write!(
                f,
                "Diese Instanz ist unter dem Namen {name} nicht freigegeben — über die IP-Adresse öffnen oder mit --erreichbar-als {name} starten."
            ),
            None => write!(
                f,
                "Diese Anfrage nennt keinen Host — über die IP-Adresse öffnen."
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl Grenzschutz {
        /// Das Urteil ohne Auskunft und ohne lokale Adresse — nur Normalform und feste Namen.
        fn host_erlaubt(&self, host: Option<&str>) -> Result<(), HostAbgewiesen> {
            tokio::runtime::Builder::new_current_thread()
                .build()
                .expect("Laufzeit")
                .block_on(self.host_pruefen(host, None))
                .map(|_| ())
        }
    }

    fn schutz() -> Grenzschutz {
        Grenzschutz::neu(Some("nuc5".into()), vec!["chat.lan".into(), "nuc".into()])
    }

    #[test]
    fn ip_literale_localhost_rechnername_und_freigegebene_namen_sind_erlaubt() {
        for host in [
            "127.0.0.1:7430",
            "127.0.0.1",
            "10.0.0.5:80",
            "[::1]:7430",
            "[::1]",
            "[fe80::1]:80",
            "localhost:7430",
            "localhost",
            "LocalHost:1",
            // ADR-0045 Regel 1: ein abschließender Punkt fällt in der Normalform.
            "localhost.:1",
            "nuc5:7430",
            "NUC5:7430",
            "nuc5.local:7430",
            "Nuc5.Local",
            "chat.lan:80",
            "CHAT.LAN",
            "nuc",
        ] {
            assert_eq!(schutz().host_erlaubt(Some(host)), Ok(()), "{host}");
        }
    }

    #[test]
    fn fremde_namen_werden_ohne_port_genannt_abgewiesen() {
        for (host, name) in [
            ("evil.example", "evil.example"),
            ("evil.example:80", "evil.example"),
            ("127.0.0.1.evil.example:7430", "127.0.0.1.evil.example"),
            ("localhost.evil.example", "localhost.evil.example"),
            ("xlocalhost:1", "xlocalhost"),
            ("nuc5.evil:1", "nuc5.evil"),
            ("nuc5.local.evil", "nuc5.local.evil"),
            ("chat.lan.evil:80", "chat.lan.evil"),
            ("evil.chat.lan", "evil.chat.lan"),
            ("", ""),
            (":80", ""),
        ] {
            assert_eq!(
                schutz().host_erlaubt(Some(host)),
                Err(HostAbgewiesen {
                    name: Some(name.into())
                }),
                "{host}"
            );
        }
    }

    #[test]
    fn unlesbare_host_werte_werden_abgewiesen() {
        for host in [
            "::1",
            "[::1",
            "[::1]x",
            "[::1]:",
            "[evil]:80",
            "[127.0.0.1]",
            "127.0.0.1:abc",
            "127.0.0.1:",
            "127.0.0.1:80:80",
            "localhost:80 ",
            "[fe80::1%25eth0]",
        ] {
            assert!(schutz().host_erlaubt(Some(host)).is_err(), "{host}");
        }
    }

    #[test]
    fn ohne_host_header_wird_abgewiesen() {
        assert_eq!(
            schutz().host_erlaubt(None),
            Err(HostAbgewiesen { name: None })
        );
    }

    #[test]
    fn ohne_rechnername_gilt_kein_rechnername_und_leere_freigaben_gelten_nicht() {
        let schutz = Grenzschutz::neu(None, vec![String::new()]);
        assert!(schutz.host_erlaubt(Some("nuc5:1")).is_err());
        assert!(schutz.host_erlaubt(Some(":1")).is_err());
        assert!(schutz.host_erlaubt(Some(".local")).is_err());
        assert_eq!(schutz.host_erlaubt(Some("127.0.0.1:1")), Ok(()));
    }

    /// Kriterium F0080 „Jeder Name“ (Unit, zählende Tabelle), Matrix I1, I5: 0 Fragen; Normalform und fehlender
    /// Host bleiben abgewiesen.
    #[tokio::test]
    async fn mit_alle_namen_gilt_jeder_name_in_normalform_ohne_frage() {
        let tabelle = Arc::new(
            crate::http::Tabelle::aus_wert(
                "127.0.0.1>nuc5.fritz.box;boese.example=127.0.0.1;x.local=127.0.0.1",
            )
            .expect("Tabelle"),
        );
        let auskunft = Arc::new(Auskunft::neu(
            Arc::clone(&tabelle) as Arc<dyn Namensauskunft>,
            Arc::new(crate::uhr::Systemuhr),
            Arc::new(Protokoll::ohne_ablage()),
        ));
        let schutz = Grenzschutz::neu(None, Vec::new())
            .mit_auskunft(auskunft)
            .mit_alle_namen(true);
        let lokal = Some("127.0.0.1".parse().expect("Adresse"));

        for host in [
            "boese.example:7430",
            "chat.lan:7430",
            "x.local",
            "Boese.Example.",
        ] {
            assert_eq!(
                schutz.host_pruefen(Some(host), lokal).await,
                Ok(Grund::AlleNamen),
                "{host}"
            );
        }
        for host in ["a_b.example", "bücher.local", "", "2130706433"] {
            assert!(
                schutz.host_pruefen(Some(host), lokal).await.is_err(),
                "{host}"
            );
        }
        assert!(schutz.host_pruefen(None, lokal).await.is_err());
        for (art, schluessel) in [
            ("rueckwaerts", "127.0.0.1"),
            ("vorwaerts", "boese.example"),
            ("vorwaerts", "x.local"),
        ] {
            assert_eq!(tabelle.fragen(art, schluessel), 0, "{art} {schluessel}");
        }
    }

    #[test]
    fn die_abweisung_nennt_den_namen_und_den_ausweg() {
        let text = HostAbgewiesen {
            name: Some("chat.lan".into()),
        }
        .to_string();
        assert_eq!(
            text,
            "Diese Instanz ist unter dem Namen chat.lan nicht freigegeben — über die IP-Adresse öffnen oder mit --erreichbar-als chat.lan starten."
        );
    }

    #[test]
    fn die_abweisung_ohne_host_sagt_dass_er_fehlt() {
        let text = HostAbgewiesen { name: None }.to_string();
        assert!(text.contains("keinen Host"), "{text}");
        assert!(text.contains("IP-Adresse"), "{text}");
    }

    const HOST: Option<&str> = Some("127.0.0.1:7430");

    fn ursprung(methode: Method, origin: Option<&str>, sec_fetch_site: Option<&str>) -> bool {
        ursprung_erlaubt(&methode, origin, sec_fetch_site, HOST).is_ok()
    }

    #[test]
    fn lesende_methoden_werden_nie_nach_dem_ursprung_abgewiesen() {
        for methode in [Method::GET, Method::HEAD, Method::OPTIONS] {
            assert!(
                ursprung(
                    methode.clone(),
                    Some("http://evil.example"),
                    Some("cross-site")
                ),
                "{methode}"
            );
        }
    }

    #[test]
    fn ohne_origin_und_ohne_sec_fetch_site_ist_es_kein_browser_und_erlaubt() {
        for methode in [Method::POST, Method::PUT, Method::DELETE, Method::PATCH] {
            assert!(ursprung(methode.clone(), None, None), "{methode}");
        }
    }

    #[test]
    fn der_eigene_ursprung_ist_erlaubt_auch_mit_sec_fetch_site_same_origin_oder_none() {
        assert!(ursprung(Method::POST, Some("http://127.0.0.1:7430"), None));
        assert!(ursprung(
            Method::POST,
            Some("http://127.0.0.1:7430"),
            Some("same-origin")
        ));
        assert!(ursprung(Method::POST, None, Some("none")));
        assert!(
            ursprung_erlaubt(
                &Method::POST,
                Some("http://LocalHost:7430"),
                None,
                Some("localhost:7430")
            )
            .is_ok()
        );
        assert!(
            ursprung_erlaubt(
                &Method::POST,
                Some("http://[::1]:7430"),
                Some("same-origin"),
                Some("[::1]:7430")
            )
            .is_ok()
        );
    }

    #[test]
    fn fremder_origin_wird_bei_jeder_nicht_lesenden_methode_abgewiesen() {
        for origin in [
            "http://evil.example",
            "null",
            "http://127.0.0.1:7421",
            "https://127.0.0.1:7430",
            "http://127.0.0.1:7430/",
            "http://127.0.0.1",
            "http://127.0.0.1:7430.evil.example",
            "",
        ] {
            for methode in [Method::POST, Method::PUT, Method::DELETE, Method::PATCH] {
                assert!(
                    !ursprung(methode.clone(), Some(origin), None),
                    "{methode} {origin:?}"
                );
            }
        }
        let eigene = Method::from_bytes(b"FANGEN").expect("Methode");
        assert!(!ursprung(eigene, Some("http://evil.example"), None));
    }

    #[test]
    fn sec_fetch_site_ausser_same_origin_und_none_wird_abgewiesen() {
        for wert in ["cross-site", "same-site", "SAME-ORIGIN", "", "same-origin "] {
            assert!(!ursprung(Method::POST, None, Some(wert)), "{wert:?}");
        }
        assert!(!ursprung(
            Method::POST,
            Some("http://127.0.0.1:7430"),
            Some("cross-site")
        ));
    }

    #[test]
    fn origin_ohne_host_wird_abgewiesen() {
        assert!(
            ursprung_erlaubt(&Method::POST, Some("http://127.0.0.1:7430"), None, None).is_err()
        );
    }

    #[test]
    fn die_ursprungs_abweisung_nennt_den_header() {
        let origin = ursprung_erlaubt(&Method::POST, Some("null"), None, HOST);
        let site = ursprung_erlaubt(&Method::POST, None, Some("cross-site"), HOST);
        assert!(
            matches!(&origin, Err(abgewiesen) if abgewiesen.grund.contains("Origin")),
            "{origin:?}"
        );
        assert!(
            matches!(&site, Err(abgewiesen) if abgewiesen.grund.contains("Sec-Fetch-Site")),
            "{site:?}"
        );
    }

    const CSP: &str = "default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self' data:; font-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'";

    fn kopf_nach(pfad: &str, vorher: &[(&'static str, &'static str)]) -> HeaderMap {
        kopf_unter(Some("127.0.0.1:7430"), pfad, vorher)
    }

    fn kopf_unter(
        host: Option<&str>,
        pfad: &str,
        vorher: &[(&'static str, &'static str)],
    ) -> HeaderMap {
        let mut kopf = HeaderMap::new();
        for (name, wert) in vorher {
            kopf.insert(*name, wert.parse().expect("Wert"));
        }
        sicherheitsheader_setzen(&mut kopf, pfad, host);
        kopf
    }

    #[test]
    fn coop_nur_unter_vertrauenswuerdigen_ursprungen() {
        let vertrauenswuerdig = [
            "127.0.0.1:7430",
            "127.8.9.1",
            "localhost:7430",
            "LocalHost",
            "app.localhost:7430",
            "[::1]:7430",
        ];
        for host in vertrauenswuerdig {
            let kopf = kopf_unter(Some(host), "/", &[]);
            assert_eq!(
                wert(&kopf, "cross-origin-opener-policy"),
                Some("same-origin"),
                "{host}"
            );
        }
        let nicht_vertrauenswuerdig = [
            Some("192.168.178.65:7430"),
            Some("10.0.0.1"),
            Some("[fe80::1]:7430"),
            Some("chat.lan:7430"),
            Some("nuc5"),
            Some("evil.example"),
            Some("127.0.0.1.evil.example"),
            Some("localhost.evil.example"),
            Some("[::1"),
            None,
        ];
        for host in nicht_vertrauenswuerdig {
            let kopf = kopf_unter(
                host,
                "/api/x",
                &[("cross-origin-opener-policy", "same-origin")],
            );
            assert_eq!(wert(&kopf, "cross-origin-opener-policy"), None, "{host:?}");
            assert_eq!(wert(&kopf, "x-content-type-options"), Some("nosniff"));
            assert_eq!(wert(&kopf, "content-security-policy"), Some(CSP));
            assert_eq!(wert(&kopf, "x-frame-options"), Some("DENY"));
            assert_eq!(wert(&kopf, "referrer-policy"), Some("no-referrer"));
            assert_eq!(
                wert(&kopf, "cross-origin-resource-policy"),
                Some("same-origin")
            );
            assert_eq!(wert(&kopf, "cache-control"), Some("no-store"));
            assert_eq!(kopf.len(), 6, "{host:?}");
        }
    }

    fn wert<'a>(kopf: &'a HeaderMap, name: &str) -> Option<&'a str> {
        kopf.get(name).and_then(|wert| wert.to_str().ok())
    }

    #[test]
    fn jede_antwort_bekommt_den_header_satz_mit_genau_diesen_werten() {
        let kopf = kopf_nach("/", &[]);
        assert_eq!(wert(&kopf, "x-content-type-options"), Some("nosniff"));
        assert_eq!(wert(&kopf, "content-security-policy"), Some(CSP));
        assert_eq!(wert(&kopf, "x-frame-options"), Some("DENY"));
        assert_eq!(wert(&kopf, "referrer-policy"), Some("no-referrer"));
        assert_eq!(
            wert(&kopf, "cross-origin-opener-policy"),
            Some("same-origin")
        );
        assert_eq!(
            wert(&kopf, "cross-origin-resource-policy"),
            Some("same-origin")
        );
        assert_eq!(wert(&kopf, "cache-control"), None);
        assert_eq!(kopf.len(), 6);
    }

    #[test]
    fn unter_api_kommt_no_store_dazu_und_ersetzt_anderes() {
        let kopf = kopf_nach("/api/gesundheit", &[("cache-control", "no-cache")]);
        assert_eq!(wert(&kopf, "cache-control"), Some("no-store"));
        assert_eq!(kopf.get_all("cache-control").iter().count(), 1);
        assert_eq!(wert(&kopf, "x-frame-options"), Some("DENY"));
    }

    #[test]
    fn ausserhalb_von_api_bleibt_das_cache_control_der_route() {
        let kopf = kopf_nach("/web/lc-tokens.css", &[("cache-control", "no-cache")]);
        assert_eq!(wert(&kopf, "cache-control"), Some("no-cache"));
        let aehnlich = kopf_nach("/apix", &[]);
        assert_eq!(wert(&aehnlich, "cache-control"), None);
    }

    #[test]
    fn vorhandene_werte_werden_ueberschrieben_und_cors_freigaben_entfernt() {
        let kopf = kopf_nach(
            "/api/x",
            &[
                ("x-frame-options", "SAMEORIGIN"),
                ("access-control-allow-origin", "*"),
                ("access-control-allow-credentials", "true"),
            ],
        );
        assert_eq!(wert(&kopf, "x-frame-options"), Some("DENY"));
        assert_eq!(kopf.get_all("x-frame-options").iter().count(), 1);
        assert!(
            !kopf
                .keys()
                .any(|name| name.as_str().starts_with("access-control-allow-")),
            "{kopf:?}"
        );
    }
}
