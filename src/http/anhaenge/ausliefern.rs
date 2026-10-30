//! Anhang ausliefern (B0446, ADR-0047 Regeln 1–4, 8, Ergänzung Karte #9 Gate Regeln 1, 3, 5): `GET` und `HEAD
//! /anhaenge/{id}` ohne Sitzung, unter dem Grenzschutz. Die ID ist nur Schlüssel des Verzeichnisses; erst die
//! Existenz, dann `If-None-Match`, dann die Datei über die Verzeichnisdeskriptoren, dann der Bereich.

use super::bereich::{Bereich, bereich_lesen};
use super::kopfzeilen::{self, Datei};
use super::strom::bereich_strom;
use crate::gespraeche::anhangablage::Anhangdatei;
use crate::gespraeche::anhangid::Anhangid;
use crate::gespraeche::anhangpruefung::medienart;
use crate::gespraeche::anhangverzeichnis::{Eintrag, Zustand as Anhangzustand, praefix};
use crate::http::Zustand;
use crate::http::grenzschutz::AnhangMedium;
use crate::protokoll::{Bereich as Protokollbereich, Kontext, Schluessel, Stufe};
use axum::body::Body;
use axum::extract::{ConnectInfo, Path, State};
use axum::http::header::{ETAG, IF_NONE_MATCH, IF_RANGE, RANGE};
use axum::http::{HeaderMap, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use std::sync::Arc;

pub const NICHT_DA: &str = "Diesen Anhang gibt es nicht (mehr).";
pub const GELOESCHT: &str = "Dieser Anhang wurde gelöscht. Die Nachricht bleibt.";

pub async fn ausliefern(
    State(zustand): State<Arc<Zustand>>,
    ConnectInfo(gegenstelle): ConnectInfo<std::net::SocketAddr>,
    methode: Method,
    Path(id): Path<String>,
    kopf: HeaderMap,
) -> Response {
    let Some(eintrag) = Anhangid::lesen(&id).and_then(|id| zustand.gespraeche.anhang_finden(&id))
    else {
        return nicht_da();
    };
    match eintrag.zustand() {
        Anhangzustand::Vorhanden => {}
        Anhangzustand::Geloescht => return geloescht(),
        Anhangzustand::Entfallen => return nicht_da(),
    }
    let etag = kopfzeilen::etag(eintrag.id.als_text());
    if passt_schon(&kopf, &etag) {
        return medium_markieren(nicht_geaendert(&etag), eintrag.art);
    }
    let datei = match oeffnen(&zustand, &eintrag).await {
        Ok(datei) => datei,
        Err(grund) => {
            fehlt_melden(&zustand, &eintrag, &grund);
            return nicht_da();
        }
    };
    let bereich = if methode == Method::HEAD {
        Bereich::Ganz
    } else {
        let range: Vec<&str> = kopf
            .get_all(RANGE)
            .iter()
            .filter_map(|wert| wert.to_str().ok())
            .collect();
        let if_range = kopf.get(IF_RANGE).map(|wert| wert.to_str().unwrap_or(""));
        bereich_lesen(&range, if_range, &etag, eintrag.bytes)
    };
    let beschrieben = Datei {
        id: eintrag.id.als_text(),
        typ: eintrag.art,
        inline: crate::gespraeche::anhangpruefung::inline(eintrag.art, eintrag.bytes),
        anzeigename: &eintrag.name,
        ascii: ascii_teil(&eintrag.datei),
        laenge: eintrag.bytes,
    };
    let (status, von, bis) = match bereich {
        Bereich::Unerfuellbar => {
            let mut antwort = StatusCode::RANGE_NOT_SATISFIABLE.into_response();
            kopfzeilen::unerfuellbar(antwort.headers_mut(), eintrag.bytes);
            return medium_markieren(antwort, eintrag.art);
        }
        Bereich::Ganz => (StatusCode::OK, 0, eintrag.bytes),
        Bereich::Teil(von, bis) => (StatusCode::PARTIAL_CONTENT, von, bis + 1),
    };
    let koerper = if methode == Method::HEAD {
        Body::empty()
    } else {
        Body::from_stream(bereich_strom(datei, von, bis).await)
    };
    zustand.protokoll.eintragen(
        Stufe::Debug,
        Protokollbereich::Anhang,
        "Anhang abgerufen",
        Kontext::neu()
            .mit(Schluessel::Herkunft, gegenstelle.ip())
            .mit(Schluessel::Ordner, eintrag.gespraech.pfad())
            .mit(Schluessel::Tag, eintrag.tag.format("%Y-%m-%d"))
            .mit(Schluessel::Anhang, praefix(&eintrag.datei))
            .mit(Schluessel::Bytes, eintrag.bytes)
            .mit(Schluessel::Art, eintrag.art)
            .mit(Schluessel::Bereich, bis - von),
    );
    let mut antwort = (status, koerper).into_response();
    let teil = matches!(bereich, Bereich::Teil(..)).then(|| (von, bis - 1));
    kopfzeilen::datei(antwort.headers_mut(), &beschrieben, teil);
    medium_markieren(antwort, eintrag.art)
}

/// Marker für die CSP-Weiche (ADR-0014 Ergänzung „Karte #71“ Regeln 2, 3): auf jeder Antwort mit der Datei einer
/// Medienart — 200, 206, 304, 416 und `HEAD` gleich —, allein nach der Art, die der Server aus dem Inhalt bestimmt hat.
fn medium_markieren(mut antwort: Response, art: &str) -> Response {
    if medienart(art) {
        antwort.extensions_mut().insert(AnhangMedium);
    }
    antwort
}

/// `<hhmmssmmm>-<n>-<ascii>` → `<ascii>`.
fn ascii_teil(datei: &str) -> &str {
    datei.splitn(3, '-').nth(2).unwrap_or("datei")
}

/// `If-None-Match` mit dem ETag (auch in einer Liste) oder `*`.
fn passt_schon(kopf: &HeaderMap, etag: &str) -> bool {
    kopf.get_all(IF_NONE_MATCH)
        .iter()
        .filter_map(|wert| wert.to_str().ok())
        .flat_map(|wert| wert.split(','))
        .any(|marke| {
            let marke = marke.trim();
            marke == etag || marke == "*"
        })
}

async fn oeffnen(
    zustand: &Zustand,
    eintrag: &Arc<Eintrag>,
) -> Result<Box<dyn Anhangdatei>, String> {
    let gespraeche = Arc::clone(&zustand.gespraeche);
    let eintrag = Arc::clone(eintrag);
    tokio::task::spawn_blocking(move || {
        gespraeche
            .anhangablage()
            .zum_lesen(&eintrag.gespraech, eintrag.tag, &eintrag.datei)
            .map_err(|fehler| fehler.to_string())
    })
    .await
    .map_err(|fehler| fehler.to_string())?
}

/// `WARNING anhang` mit Gespräch, Tag und Datei nur als `<hhmmssmmm>-<n>` — nie ID, ASCII-Teil oder Anzeigename.
fn fehlt_melden(zustand: &Zustand, eintrag: &Eintrag, grund: &str) {
    zustand.protokoll.eintragen(
        Stufe::Warning,
        Protokollbereich::Anhang,
        "Anhang fehlt auf der Platte — 404",
        Kontext::neu()
            .mit(Schluessel::Ordner, eintrag.gespraech.pfad())
            .mit(Schluessel::Tag, eintrag.tag.format("%Y-%m-%d"))
            .mit(Schluessel::Anhang, praefix(&eintrag.datei))
            .mit(Schluessel::Grund, grund),
    );
}

fn nicht_da() -> Response {
    let mut antwort = (
        StatusCode::NOT_FOUND,
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; charset=utf-8",
        )],
        NICHT_DA,
    )
        .into_response();
    kopfzeilen::route(antwort.headers_mut());
    antwort
}

/// 410 nach `finden` und vor `If-None-Match` und `Range` (ADR-0047 Ergänzung „F1′“ Regel 1): ein alter ETag ergibt
/// nie 304; ohne Gespräch, Tag und Name (M44′).
fn geloescht() -> Response {
    let mut antwort = (
        StatusCode::GONE,
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; charset=utf-8",
        )],
        GELOESCHT,
    )
        .into_response();
    kopfzeilen::route(antwort.headers_mut());
    antwort.headers_mut().insert(
        axum::http::header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-store"),
    );
    antwort
}

fn nicht_geaendert(etag: &str) -> Response {
    let mut antwort = StatusCode::NOT_MODIFIED.into_response();
    kopfzeilen::route(antwort.headers_mut());
    if let Ok(wert) = etag.parse() {
        antwort.headers_mut().insert(ETAG, wert);
    }
    antwort.headers_mut().insert(
        axum::http::header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("private, no-cache"),
    );
    antwort
}
