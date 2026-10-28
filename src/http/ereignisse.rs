//! Ereignisse beantworten (B0084, ADR-0015 Regel 1, ADR-0016 Regel 5 und 8): `GET /api/ereignisse` mit der
//! Sitzung im Header → `text/event-stream`. Der offene Strom ist ein Halter seiner Sitzung; er beginnt mit
//! dem Stand der Teilnehmerliste, danach je Wechsel ein Ereignis und alle `herzschlag` ein Lebenszeichen,
//! bis der Client schließt oder das Programm endet. Ein Token im Query zählt nicht.

use super::Zustand;
use super::teilnehmer::{gemeldetes_token, nicht_erkannt};
use super::teilnehmerliste::{stand_als_json, zeile_als_json};
use crate::anwesenheit::{Anwesenheit, Halter, Stand, Wechsel};
use crate::live::bus::{Abonnement, Empfang, Ereignis};
use crate::start::ende::Ende;
use axum::extract::{ConnectInfo, State};
use axum::http::HeaderMap;
use axum::response::sse::{Event, Sse};
use axum::response::{IntoResponse, Response};
use futures_util::stream;
use serde_json::json;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::time::Interval;

pub async fn beantworten(
    State(zustand): State<Arc<Zustand>>,
    ConnectInfo(gegenstelle): ConnectInfo<SocketAddr>,
    kopf: HeaderMap,
) -> Response {
    let verbunden = gemeldetes_token(&kopf)
        .and_then(|token| zustand.anwesenheit.verbunden(&token, gegenstelle.ip()));
    let Some((halter, _veroeffentlicht)) = verbunden else {
        return nicht_erkannt();
    };
    let (stand, abonnement) = zustand.anwesenheit.stand_mit(|| zustand.bus.abonnieren());
    let herzschlag = zustand.herzschlag;
    let strom = Strom {
        anwesenheit: Arc::clone(&zustand.anwesenheit),
        abonnement,
        ende: zustand.ende.clone(),
        herzschlag: tokio::time::interval_at(tokio::time::Instant::now() + herzschlag, herzschlag),
        zuletzt: stand.stand,
        erstes: Some(stand_ereignis(&stand)),
        _halter: halter,
    };
    Sse::new(stream::unfold(strom, Strom::naechstes)).into_response()
}

/// Was ein offener Strom hält; mit ihm fällt der Halter (Client weg oder Ende des Programms).
struct Strom {
    anwesenheit: Arc<Anwesenheit>,
    abonnement: Abonnement,
    ende: Ende,
    herzschlag: Interval,
    /// Nummer des zuletzt gesendeten Stands oder Wechsels; ältere oder gleiche gehen schon im Stand auf.
    zuletzt: u64,
    erstes: Option<Event>,
    _halter: Halter,
}

impl Strom {
    async fn naechstes(mut self) -> Option<(Result<Event, Infallible>, Strom)> {
        if let Some(erstes) = self.erstes.take() {
            return Some((Ok(erstes), self));
        }
        loop {
            let ereignis = tokio::select! {
                () = self.ende.abwarten() => return None,
                _ = self.herzschlag.tick() => Event::default().event("herzschlag").data("{}"),
                empfang = self.abonnement.naechstes() => match empfang? {
                    Empfang::Ereignis(Ereignis::Anwesenheit(wechsel)) => {
                        if wechsel.stand <= self.zuletzt {
                            continue;
                        }
                        self.zuletzt = wechsel.stand;
                        wechsel_ereignis(&wechsel)
                    }
                    Empfang::NeuerStand => {
                        let stand = self.anwesenheit.teilnehmer();
                        self.zuletzt = stand.stand;
                        stand_ereignis(&stand)
                    }
                },
            };
            return Some((Ok(ereignis), self));
        }
    }
}

fn stand_ereignis(stand: &Stand) -> Event {
    let mut stand = stand_als_json(stand);
    stand["art"] = json!("teilnehmer");
    Event::default().data(stand.to_string())
}

fn wechsel_ereignis(wechsel: &Wechsel) -> Event {
    let ereignis = json!({
        "art": "anwesenheit",
        "stand": wechsel.stand,
        "teilnehmer": zeile_als_json(&wechsel.teilnehmer),
    });
    Event::default().data(ereignis.to_string())
}
