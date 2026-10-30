//! Grenzschutz (ADR-0014): welche Host-Namen die Instanz anspricht (gegen DNS-Rebinding) und von welchem
//! Ursprung sie Wirkung annimmt (gegen CSRF).

use super::abweisungsseite;
use super::auskunft::{Abfrage, Auskunft, WARTEZEIT};
use super::hostnamen::{
    FesteNamen, Grund, host_urteil, im_lokalen_namensraum, normalform, zeigt_auf_eigene,
};
use super::namensauskunft::Namensauskunft;
use crate::protokoll::episoden::{Auftreten, Quelle};
use crate::protokoll::{Bereich, Protokoll, Schluessel, Stufe};
use crate::start::ausliefern::LokaleAdresse;
use axum::Json;
use axum::extract::{ConnectInfo, Request, State};
use axum::http::header::{
    ACCEPT, CACHE_CONTROL, CONNECTION, CONTENT_SECURITY_POLICY, CONTENT_TYPE, HOST, HeaderName,
    HeaderValue, ORIGIN, REFERRER_POLICY, X_CONTENT_TYPE_OPTIONS, X_FRAME_OPTIONS,
};
use axum::http::{HeaderMap, Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use tokio::time::Instant;

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
        Some((mut abweisung, ereignis, kopf)) => {
            abweisung_eintragen(&protokoll, &anfrage, ereignis, &kopf);
            schliessen(&mut abweisung);
            abweisung
        }
        None => weiter.run(anfrage).await,
    };
    let medium = antwort.extensions().get::<AnhangMedium>().copied();
    sicherheitsheader_setzen(antwort.headers_mut(), &pfad, host.as_deref(), medium);
    antwort
}

/// Jede Abweisung des Grenzschutzes schließt die Verbindung (ADR-0014 Ergänzung 2026-10-30): kein Körper der
/// abgewiesenen Anfrage wird gelesen, keine angehängte Anfrage beantwortet.
fn schliessen(abweisung: &mut Response) {
    abweisung
        .headers_mut()
        .insert(CONNECTION, HeaderValue::from_static("close"));
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
    protokoll.episode(abweisung_auftreten(
        ereignis,
        herkunft,
        &format!("{kopf}: {wert}"),
    ));
}

/// Quelle der Episode ist die Herkunft; der abgewiesene Kopf mit seinem Wert gehört dem einzelnen Auftreten und
/// steht nur in der Beginn-Zeile (ADR-0042 Ergänzung Karte #66).
pub(crate) fn abweisung_auftreten(
    ereignis: &'static str,
    herkunft: String,
    grund: &str,
) -> Auftreten {
    Auftreten::neu(
        Stufe::Warning,
        Bereich::Grenzschutz,
        ereignis,
        Quelle::neu().mit(Schluessel::Herkunft, herkunft),
    )
    .einzeln(Schluessel::Grund, grund)
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
            crate::protokoll::episodentexte::FREMDER_HOST,
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
                crate::protokoll::episodentexte::FREMDER_URSPRUNG,
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
    /// Name nur in Normalform (Regel 1), dann fest (Regel 2), außerhalb des lokalen Namensraums als Rückwärtsname
    /// von `lokal` (Regel 3), darin nur mit eigener Adresse (Regel 4) — nur dort wird vorwärts aufgelöst; alles andere
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
        // Eine Frist je Anfrage und ein Weg je Name (ADR-0045 Ergänzung „Karte #35 Nacharbeit“ Punkt 1): ein Name
        // im lokalen Namensraum nur vorwärts (Regel 4), jeder andere nur rückwärts (Regel 3).
        let frist = Instant::now() + WARTEZEIT;
        if !im_lokalen_namensraum(&normal) {
            let namen = auskunft
                .namen_der_adresse(lokal, frist)
                .await
                .unwrap_or_default();
            let grund = host_urteil(&normal, &self.feste, || namen).ok_or_else(abgewiesen)?;
            auskunft.erlaubt_melden(&Abfrage::Rueckwaerts(lokal), &normal, lokal);
            return Ok(grund);
        }
        let name = normal.clone();
        let urteilen = move |port: &dyn Namensauskunft| {
            let adressen = port.adressen_des_namens(&name);
            zeigt_auf_eigene(&adressen, &mut |adresse| port.ist_eigene_adresse(adresse))
        };
        if auskunft.urteil_ueber(&normal, urteilen, frist).await != Some(true) {
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
pub fn sicherheitsheader_setzen(
    kopf: &mut HeaderMap,
    pfad: &str,
    host: Option<&str>,
    medium: Option<AnhangMedium>,
) {
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
    if anhangpfad(pfad) {
        kopf.insert(
            CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(anhang_csp(medium)),
        );
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

/// CSP jeder Antwort unter `/anhaenge` (ADR-0047 Ergänzung Karte #9 Gate Regel 1): kein Skript, kein Stil, kein
/// Laden — der Anhang ist ein eigener, leerer Ursprung.
pub const ANHANG_CSP: &str = "sandbox; default-src 'none'";

/// CSP der Antworten auf die fünf Medienarten (ADR-0014 Ergänzung „Karte #71 (Chrome 154)“ Regel 1): das
/// Medien-Dokument des Browsers läuft im Ursprung der Instanz (Google Chrome lädt das Medium sonst als CORS-Abruf aus
/// dem Ursprung `null`) und darf genau die eigene URL als Medium laden. Nie `allow-scripts`, nie `allow-downloads`,
/// nie ein weiteres Token: neben `allow-same-origin` bliebe sonst nur `default-src 'none'` vor dem Sitzungs-Token.
/// Es gibt keine dritte Anhang-CSP.
pub const ANHANG_CSP_MEDIUM: &str =
    "sandbox allow-same-origin; default-src 'none'; media-src 'self'";

/// Marker in den Erweiterungen der Antwort (Regel 2): die Route sagt der Weiche, dass die vom Server aus dem Inhalt
/// bestimmte Art eine Medienart ist. Ein Typ ohne Inhalt — keine freie Zeichenkette, von außen nicht setzbar; er wählt
/// nur zwischen den beiden Anhang-CSP.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AnhangMedium;

/// Weiche unter `/anhaenge` (ADR-0014 Ergänzung „Karte #71 (Chrome 154)“ Regeln 1–3): mit Marker die Medien-CSP,
/// sonst `ANHANG_CSP`. CORP bleibt `same-origin` aus dem Header-Satz, auch mit Marker.
fn anhang_csp(medium: Option<AnhangMedium>) -> &'static str {
    match medium {
        Some(AnhangMedium) => ANHANG_CSP_MEDIUM,
        None => ANHANG_CSP,
    }
}

const CROSS_ORIGIN_RESOURCE_POLICY: HeaderName =
    HeaderName::from_static("cross-origin-resource-policy");

/// Ob der Pfad — roh oder prozent-dekodiert — `/anhaenge` ist oder darunter liegt (B0467).
fn anhangpfad(pfad: &str) -> bool {
    let unter = |pfad: &str| pfad == "/anhaenge" || pfad.starts_with("/anhaenge/");
    unter(pfad) || unter(&prozent_dekodiert(pfad))
}

/// `%XX` → Byte; ungültige Folgen bleiben, wie sie sind.
fn prozent_dekodiert(pfad: &str) -> String {
    let bytes = pfad.as_bytes();
    let mut dekodiert = Vec::with_capacity(bytes.len());
    let mut stelle = 0;
    while stelle < bytes.len() {
        let hex = bytes
            .get(stelle + 1..stelle + 3)
            .and_then(|hex| std::str::from_utf8(hex).ok())
            .and_then(|hex| u8::from_str_radix(hex, 16).ok());
        match (bytes[stelle], hex) {
            (b'%', Some(byte)) => {
                dekodiert.push(byte);
                stelle += 3;
            }
            (byte, _) => {
                dekodiert.push(byte);
                stelle += 1;
            }
        }
    }
    String::from_utf8_lossy(&dekodiert).into_owned()
}

const HEADER_SATZ: [(HeaderName, &str); 5] = [
    (X_CONTENT_TYPE_OPTIONS, "nosniff"),
    (
        CONTENT_SECURITY_POLICY,
        "default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self' data:; font-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'",
    ),
    (X_FRAME_OPTIONS, "DENY"),
    (REFERRER_POLICY, "no-referrer"),
    (CROSS_ORIGIN_RESOURCE_POLICY, "same-origin"),
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

    /// R00038 B0591 (Bestandsaufnahme): zwei Abweisungen derselben Herkunft mit verschiedenem Host → Beginn mit
    /// `grund=` des ersten Kopfes, Ende nur `herkunft=`.
    #[test]
    fn die_ende_zeile_der_abweisung_nennt_nur_die_herkunft() {
        use crate::protokoll::Stufe;
        use crate::protokoll::testhilfe::{im_speicher, zeit};
        let (protokoll, speicher, uhr, _) =
            im_speicher("2026-10-28T12:00:00+01:00", Stufe::Information);
        for host in ["boese.example", "andere.example"] {
            protokoll.episode(abweisung_auftreten(
                "Anfrage mit fremdem Host abgewiesen",
                "10.0.0.9".into(),
                &format!("host: {host}"),
            ));
        }
        uhr.stellen(zeit("2026-10-28T12:01:00+01:00"));
        protokoll.episoden_schliessen();

        let zeilen = speicher.zeilen();
        assert_eq!(zeilen.len(), 2, "{zeilen:#?}");
        assert!(
            zeilen[0].ends_with("| herkunft=10.0.0.9 grund=\"host: boese.example\""),
            "{zeilen:#?}"
        );
        assert!(zeilen[1].contains("Episode endet: 2 in "), "{zeilen:#?}");
        assert!(zeilen[1].ends_with("| herkunft=10.0.0.9"), "{zeilen:#?}");
    }

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
        sicherheitsheader_setzen(&mut kopf, pfad, host, None);
        kopf
    }

    fn kopf_mit_marker(pfad: &str, medium: Option<AnhangMedium>) -> HeaderMap {
        let mut kopf = HeaderMap::new();
        for (name, wert) in [
            ("content-security-policy", "script-src *"),
            ("cross-origin-resource-policy", "cross-origin"),
            ("access-control-allow-origin", "*"),
            ("x-content-type-options", "nosniff"),
            ("x-frame-options", "DENY"),
        ] {
            kopf.insert(name, HeaderValue::from_static(wert));
        }
        sicherheitsheader_setzen(&mut kopf, pfad, Some("127.0.0.1:7430"), medium);
        kopf
    }

    /// B0662, B0727 (ADR-0014 Ergänzung „Karte #71 (Chrome 154)“ Regeln 1–4; B2–B5): unter `/anhaenge` wählt der
    /// Marker nur die CSP — mit Marker die Medien-CSP, ohne `ANHANG_CSP`; CORP ist in beiden Fällen `same-origin`, auch
    /// wenn die Route etwas anderes setzt; außerhalb von `/anhaenge` wirkt der Marker nicht. `nosniff`,
    /// `X-Frame-Options`, `Referrer-Policy` unverändert (B4), keine CORS-Freigabe, kein COEP (B5). Mutanten „Weiche
    /// ignoriert Marker“, „Marker auf Nicht-Anhangpfad wirkt“ und „CORP `cross-origin` mit Marker“ werden rot.
    #[test]
    fn die_csp_weiche_nach_dem_marker() {
        for pfad in ["/anhaenge/x", "/anhaenge/x/y", "/%61nhaenge/x"] {
            for (medium, csp) in [
                (
                    Some(AnhangMedium),
                    "sandbox allow-same-origin; default-src 'none'; media-src 'self'",
                ),
                (None, "sandbox; default-src 'none'"),
            ] {
                let kopf = kopf_mit_marker(pfad, medium);
                let fall = format!("{pfad} {medium:?}");
                assert_eq!(wert(&kopf, "content-security-policy"), Some(csp), "{fall}");
                assert_eq!(
                    wert(&kopf, "cross-origin-resource-policy"),
                    Some("same-origin"),
                    "{fall}"
                );
                assert_eq!(
                    wert(&kopf, "x-content-type-options"),
                    Some("nosniff"),
                    "{fall}"
                );
                assert_eq!(wert(&kopf, "x-frame-options"), Some("DENY"), "{fall}");
                assert_eq!(
                    wert(&kopf, "referrer-policy"),
                    Some("no-referrer"),
                    "{fall}"
                );
                assert_eq!(wert(&kopf, "access-control-allow-origin"), None, "{fall}");
                assert_eq!(wert(&kopf, "cross-origin-embedder-policy"), None, "{fall}");
                assert_eq!(kopf.len(), 6, "{fall}");
            }
        }
        for pfad in ["/anhaengex", "/api/anhaenge/x", "/", "/x/anhaenge/"] {
            let kopf = kopf_mit_marker(pfad, Some(AnhangMedium));
            assert_eq!(wert(&kopf, "content-security-policy"), Some(CSP), "{pfad}");
            assert_eq!(
                wert(&kopf, "cross-origin-resource-policy"),
                Some("same-origin"),
                "{pfad}"
            );
        }
    }

    /// B0727 (Regel 1 und 2 der ADR-0014 Ergänzung „Karte #71 (Chrome 154)“): die Medien-CSP ist `sandbox` mit genau
    /// dem einen Token `allow-same-origin`, `default-src 'none'` und genau `media-src 'self'` — kein `allow-scripts`,
    /// kein `allow-downloads`, kein `*`, kein `data:`/`blob:`, keine weitere Direktive; die Anhang-CSP ohne Token.
    #[test]
    fn die_medien_csp_hat_genau_diesen_wortlaut() {
        assert_eq!(
            ANHANG_CSP_MEDIUM,
            "sandbox allow-same-origin; default-src 'none'; media-src 'self'"
        );
        assert_eq!(ANHANG_CSP, "sandbox; default-src 'none'");
    }

    /// B0467: unter `/anhaenge` (roh oder dekodiert) genau `sandbox; default-src 'none'`, sonst die App-CSP; alle
    /// anderen Kopfzeilen des Satzes unverändert (Mutante „Weiche fehlt“).
    #[test]
    fn die_csp_weiche_nach_dem_pfad() {
        for pfad in [
            "/anhaenge",
            "/anhaenge/",
            "/anhaenge/x",
            "/anhaenge/x/y",
            "/%61nhaenge/x",
            "/%61NHAENGE/x",
        ] {
            let erwartet = if pfad == "/%61NHAENGE/x" {
                CSP
            } else {
                ANHANG_CSP
            };
            let kopf = kopf_nach(pfad, &[("content-security-policy", "script-src *")]);
            assert_eq!(
                wert(&kopf, "content-security-policy"),
                Some(erwartet),
                "{pfad}"
            );
            assert_eq!(
                wert(&kopf, "x-content-type-options"),
                Some("nosniff"),
                "{pfad}"
            );
            assert_eq!(wert(&kopf, "x-frame-options"), Some("DENY"));
            assert_eq!(
                wert(&kopf, "cross-origin-resource-policy"),
                Some("same-origin")
            );
        }
        for pfad in [
            "/anhaengex",
            "/api/anhaenge",
            "/api/anhaenge/x",
            "/",
            "/x/anhaenge/",
        ] {
            let kopf = kopf_nach(pfad, &[]);
            assert_eq!(wert(&kopf, "content-security-policy"), Some(CSP), "{pfad}");
        }
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

    /// Abnahme-Probe B1 des `code-reviewer` (Nacharbeit #35, ADR-0045 Ergänzung „Karte #35 Nacharbeit“ Punkt 1):
    /// über `host_pruefen` mit blockierender Testauskunft, lokale Adresse 192.168.178.65.
    mod frist_je_anfrage {
        use super::*;
        use crate::http::testauskunft::{Testauskunft, ip};
        use std::sync::atomic::Ordering;
        use std::time::{Duration, Instant};

        const LOKAL: &str = "192.168.178.65";
        const GRENZE: Duration = Duration::from_millis(1100);

        fn schutz_mit(test: &Arc<Testauskunft>) -> Arc<Grenzschutz> {
            let auskunft = Auskunft::neu(
                Arc::clone(test) as Arc<dyn Namensauskunft>,
                Arc::new(crate::uhr::Systemuhr),
                Arc::new(Protokoll::ohne_ablage()),
            );
            Arc::new(Grenzschutz::neu(None, Vec::new()).mit_auskunft(Arc::new(auskunft)))
        }

        /// Rückwärtsfrage für die lokale Adresse und Vorwärtsfrage für `namen` hängen je `dauer`.
        fn haengend(namen: &[&str], dauer: Duration) -> Arc<Testauskunft> {
            let test = Testauskunft::mit(&[(LOKAL, "nuc5.fritz.box")], &[]);
            test.haengen(LOKAL, dauer);
            for name in namen {
                test.haengen(name, dauer);
            }
            test
        }

        async fn gepruefte_dauer(
            schutz: &Grenzschutz,
            host: &str,
        ) -> (Result<Grund, HostAbgewiesen>, Duration) {
            let beginn = Instant::now();
            let urteil = schutz.host_pruefen(Some(host), Some(ip(LOKAL))).await;
            (urteil, beginn.elapsed())
        }

        /// „Host chat.local:7430, nas.home.arpa:7430, dienst.internal:7430, chat:7430 → je abgewiesen nach
        /// ≤ 1 s + 100 ms ab Aufruf“ (Rückwärts- und Vorwärtsfrage blockieren je 3 s).
        #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
        async fn lokale_namen_sind_nach_hoechstens_einer_sekunde_abgewiesen() {
            for name in ["chat.local", "nas.home.arpa", "dienst.internal", "chat"] {
                let test = haengend(&[name], Duration::from_secs(3));
                let schutz = schutz_mit(&test);
                let (urteil, dauer) = gepruefte_dauer(&schutz, &format!("{name}:7430")).await;
                assert!(urteil.is_err(), "{name}");
                assert!(dauer <= GRENZE, "{name}: {dauer:?}");
            }
        }

        /// „zweite Anfrage nas.home.arpa:7430 bei t = 0,5 s während laufender Rückwärtsfrage → ≤ 1 s + 100 ms ab
        /// eigenem Eingang“.
        #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
        async fn eine_zweite_anfrage_wartet_hoechstens_eine_sekunde_ab_eigenem_eingang() {
            let test = haengend(&["chat.local", "nas.home.arpa"], Duration::from_secs(3));
            let schutz = schutz_mit(&test);
            let erste = {
                let schutz = Arc::clone(&schutz);
                tokio::spawn(async move { gepruefte_dauer(&schutz, "chat.local:7430").await })
            };
            tokio::time::sleep(Duration::from_millis(500)).await;

            let (urteil, dauer) = gepruefte_dauer(&schutz, "nas.home.arpa:7430").await;

            assert!(urteil.is_err());
            assert!(dauer <= GRENZE, "{dauer:?}");
            let (urteil, dauer) = erste.await.expect("erste Anfrage");
            assert!(urteil.is_err());
            assert!(dauer <= GRENZE, "{dauer:?}");
        }

        /// „Kontrolle boese.example:7430 bei blockierter Rückwärtsfrage → ≤ 1,1 s, 0 Vorwärtsfragen“.
        #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
        async fn ein_oeffentlicher_name_wartet_hoechstens_eine_sekunde_ohne_vorwaertsfrage() {
            let test = haengend(&[], Duration::from_secs(3));
            let schutz = schutz_mit(&test);

            let (urteil, dauer) = gepruefte_dauer(&schutz, "boese.example:7430").await;

            assert!(urteil.is_err());
            assert!(dauer <= GRENZE, "{dauer:?}");
            assert_eq!(test.fragen("vorwaerts", "boese.example"), 0);
        }

        /// „Gegenprobe ohne Blockade chat.local → 127.0.0.1 erlaubt“.
        #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
        async fn ohne_blockade_ist_ein_lokaler_name_auf_eine_eigene_adresse_erlaubt() {
            let test = Testauskunft::mit(&[], &[("chat.local", &["127.0.0.1"])]);
            let schutz = schutz_mit(&test);

            let (urteil, _) = gepruefte_dauer(&schutz, "chat.local:7430").await;

            assert_eq!(urteil, Ok(Grund::LokalerNamensraum));
        }

        /// ADR-0045 Durchsetzung, ergänzt: Rückwärtsfrage blockiert 3 s, `chat.local → 127.0.0.1` antwortet sofort →
        /// erlaubt nach < 100 ms, 0 Rückwärtsfragen (Mutante „lokaler Name fragt auch rückwärts“).
        #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
        async fn ein_lokaler_name_wird_nie_rueckwaerts_gefragt() {
            let test = Testauskunft::mit(
                &[(LOKAL, "nuc5.fritz.box")],
                &[("chat.local", &["127.0.0.1"])],
            );
            test.haengen(LOKAL, Duration::from_secs(3));
            let schutz = schutz_mit(&test);

            let (urteil, dauer) = gepruefte_dauer(&schutz, "chat.local:7430").await;

            assert_eq!(urteil, Ok(Grund::LokalerNamensraum));
            assert!(dauer < Duration::from_millis(100), "{dauer:?}");
            assert_eq!(test.fragen("rueckwaerts", LOKAL), 0);
        }

        /// „Matrix F1 über host_pruefen: 50 gleichzeitige r<n>.local, jede Frage hängt 5 s → jede ≤ 1,1 s, höchstens
        /// 2 Fragen gleichzeitig“.
        #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
        async fn fuenfzig_gleichzeitige_lokale_namen_bleiben_bei_zwei_fragen() {
            let namen: Vec<String> = (0..50).map(|nummer| format!("r{nummer}.local")).collect();
            let alle: Vec<&str> = namen.iter().map(String::as_str).collect();
            let test = haengend(&alle, Duration::from_secs(5));
            let schutz = schutz_mit(&test);
            let mut laeufe = Vec::new();
            for name in &namen {
                let schutz = Arc::clone(&schutz);
                let host = format!("{name}:7430");
                laeufe.push(tokio::spawn(async move {
                    gepruefte_dauer(&schutz, &host).await
                }));
            }
            for lauf in laeufe {
                let (urteil, dauer) = lauf.await.expect("Lauf");
                assert!(urteil.is_err());
                assert!(dauer <= GRENZE, "{dauer:?}");
            }
            assert!(test.hoechstens.load(Ordering::SeqCst) <= 2);
        }
    }

    /// Abnahme-Probe W1 des `security-reviewer` (Nacharbeit #35, L1, Testauskunft und Testuhr): „x.local=127.0.0.1
    /// und 127.0.0.2>chat.fritz.box je einmal erlaubt; danach 1.000 verschiedene einteilige Namen f<i> sofort
    /// beantwortet und abgeschlossen → x.local weiter erlaubt mit fragen("vorwaerts","x.local") == 1,
    /// chat.fritz.box über 127.0.0.2 erlaubt mit fragen("rueckwaerts","127.0.0.2") == 1; Varianten Flut auf
    /// .local/.internal/.home.arpa“. Die Testuhr rückt je Name 0,1 s vor (Rate).
    mod flut_verdraengt_keinen_rechtmaessigen_namen {
        use super::*;
        use crate::http::testauskunft::{Testauskunft, ip};
        use crate::protokoll::testhilfe::zeit;
        use crate::uhr::{GestellteUhr, Uhr};

        async fn flut_mit(endung: &str) {
            let test = Testauskunft::mit(
                &[("127.0.0.2", "chat.fritz.box")],
                &[("x.local", &["127.0.0.1"])],
            );
            let uhr = Arc::new(GestellteUhr::neu(zeit("2026-10-29T12:00:00+01:00")));
            let auskunft = Auskunft::neu(
                Arc::clone(&test) as Arc<dyn Namensauskunft>,
                Arc::clone(&uhr) as Arc<dyn crate::uhr::Uhr>,
                Arc::new(Protokoll::ohne_ablage()),
            );
            let schutz = Grenzschutz::neu(None, Vec::new()).mit_auskunft(Arc::new(auskunft));
            let ueber_1 = Some(ip("127.0.0.1"));
            let ueber_2 = Some(ip("127.0.0.2"));
            assert_eq!(
                schutz.host_pruefen(Some("x.local:7430"), ueber_1).await,
                Ok(Grund::LokalerNamensraum)
            );
            assert_eq!(
                schutz
                    .host_pruefen(Some("chat.fritz.box:7430"), ueber_2)
                    .await,
                Ok(Grund::Rueckwaertsname)
            );

            for nummer in 0..1000 {
                uhr.stellen(uhr.jetzt() + chrono::Duration::milliseconds(100));
                let host = format!("f{nummer}{endung}:7430");
                assert!(
                    schutz.host_pruefen(Some(&host), ueber_1).await.is_err(),
                    "{host}"
                );
            }

            assert_eq!(
                schutz.host_pruefen(Some("x.local:7430"), ueber_1).await,
                Ok(Grund::LokalerNamensraum),
                "{endung}"
            );
            assert_eq!(test.fragen("vorwaerts", "x.local"), 1, "{endung}");
            assert_eq!(
                schutz
                    .host_pruefen(Some("chat.fritz.box:7430"), ueber_2)
                    .await,
                Ok(Grund::Rueckwaertsname),
                "{endung}"
            );
            assert_eq!(test.fragen("rueckwaerts", "127.0.0.2"), 1, "{endung}");
            assert_eq!(
                test.fragen("vorwaerts", &format!("f999{endung}")),
                1,
                "die Flut wurde gefragt"
            );
        }

        #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
        async fn flut_einteiliger_namen() {
            flut_mit("").await;
        }

        #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
        async fn flut_auf_local() {
            flut_mit(".local").await;
        }

        #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
        async fn flut_auf_internal() {
            flut_mit(".internal").await;
        }

        #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
        async fn flut_auf_home_arpa() {
            flut_mit(".home.arpa").await;
        }
    }
}
