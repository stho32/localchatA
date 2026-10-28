//! Ereignisse beantworten (B0084, ADR-0015 Regel 1, ADR-0016 Regel 5 und 8): `GET /api/ereignisse` mit der
//! Sitzung im Header → `text/event-stream`. Der offene Strom ist ein Halter seiner Sitzung; er beginnt mit
//! dem Stand der Teilnehmerliste, danach je Wechsel ein Ereignis und alle `herzschlag` ein Lebenszeichen,
//! bis der Client schließt, das Programm endet oder die eigene Sitzung `weg` ist (verlassen, B0115: das `weg`
//! geht noch hinaus, dann endet der Strom). Ein Token im Query zählt nicht.

use super::Zustand;
use super::teilnehmer::{gemeldetes_token, nicht_erkannt};
use super::teilnehmerliste::{stand_als_json, zeile_als_json, zeit_als_text};
use crate::anwesenheit::status::Status;
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
        erstes: Some(stand),
        eigenes_weg_gesendet: false,
        halter,
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
    erstes: Option<Stand>,
    /// Das `weg` der eigenen Sitzung ist hinaus — danach endet der Strom (ADR-0018 Regel 4).
    eigenes_weg_gesendet: bool,
    halter: Halter,
}

/// Was der Strom als Nächstes sendet.
#[derive(Debug)]
enum Ausgabe {
    Stand(Stand),
    Wechsel(Wechsel),
    Herzschlag,
}

impl Strom {
    async fn naechstes(mut self) -> Option<(Result<Event, Infallible>, Strom)> {
        if self.eigenes_weg_gesendet {
            return None;
        }
        let ereignis = match self.naechste_ausgabe().await? {
            Ausgabe::Stand(stand) => stand_ereignis(&stand),
            Ausgabe::Wechsel(wechsel) => wechsel_ereignis(&wechsel),
            Ausgabe::Herzschlag => herzschlag_ereignis(&self.anwesenheit),
        };
        Some((Ok(ereignis), self))
    }

    /// Zuerst der Stand, danach Wechsel mit größerem `stand` als dem zuletzt gesendeten (kleinere oder
    /// gleiche stecken schon im Stand — nach `Lagged` bringt der Bus sie noch), Herzschläge; `None` am Ende.
    async fn naechste_ausgabe(&mut self) -> Option<Ausgabe> {
        if let Some(erstes) = self.erstes.take() {
            return Some(Ausgabe::Stand(erstes));
        }
        loop {
            return Some(tokio::select! {
                () = self.ende.abwarten() => return None,
                _ = self.herzschlag.tick() => Ausgabe::Herzschlag,
                empfang = self.abonnement.naechstes() => match empfang? {
                    Empfang::Ereignis(Ereignis::Anwesenheit(wechsel)) => {
                        if wechsel.stand <= self.zuletzt {
                            continue;
                        }
                        self.zuletzt = wechsel.stand;
                        self.eigenes_weg_gesendet = self.ist_eigenes_weg(&wechsel);
                        Ausgabe::Wechsel(wechsel)
                    }
                    Empfang::NeuerStand => {
                        let stand = self.anwesenheit.teilnehmer();
                        self.zuletzt = stand.stand;
                        Ausgabe::Stand(stand)
                    }
                },
            });
        }
    }
}

/// Der Herzschlag trägt die Serverzeit (B0107, ADR-0019 Regel 2): daran gleicht der Leser seine Uhr ab.
fn herzschlag_ereignis(anwesenheit: &Anwesenheit) -> Event {
    let jetzt = json!({ "jetzt": zeit_als_text(&anwesenheit.jetzt()) });
    Event::default().event("herzschlag").data(jetzt.to_string())
}

impl Strom {
    fn ist_eigenes_weg(&self, wechsel: &Wechsel) -> bool {
        wechsel.teilnehmer.status == Status::Weg
            && &wechsel.teilnehmer.kennung == self.halter.kennung()
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
        "jetzt": zeit_als_text(&wechsel.jetzt),
        "teilnehmer": zeile_als_json(&wechsel.teilnehmer),
    });
    Event::default().data(ereignis.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anwesenheit::Beitrittswunsch;
    use crate::live::bus::{Bus, anwesenheit_veroeffentlichen};
    use crate::uhr::Systemuhr;
    use crate::zufall::Betriebssystemzufall;
    use std::time::Duration;

    fn beitreten(anwesenheit: &Anwesenheit, name: &str) -> crate::zufall::Token {
        let wunsch = Beitrittswunsch::pruefen(&json!({ "name": name })).expect("Wunsch");
        anwesenheit
            .beitreten(&wunsch, "127.0.0.2".parse().expect("IP"))
            .expect("beigetreten")
            .sitzung
            .token
    }

    /// Nacharbeit 2 (Code-Review): wer zu langsam liest, bekommt einen neuen Stand S' — und danach nur Wechsel
    /// über S', streng steigend, obwohl der Bus die älteren noch liefert.
    #[tokio::test]
    async fn nach_einem_neuen_stand_kommen_nur_juengere_wechsel() {
        let bus = Arc::new(Bus::neu(2));
        let anwesenheit = Arc::new(
            Anwesenheit::neu(
                Arc::new(Systemuhr),
                Arc::new(Betriebssystemzufall),
                Duration::from_secs(60),
            )
            .mit_veroeffentlichen(anwesenheit_veroeffentlichen(&bus)),
        );
        let token = beitreten(&anwesenheit, "leser");
        let (halter, _) = anwesenheit
            .verbunden(&token, "127.0.0.2".parse().expect("IP"))
            .expect("Halter");
        let (stand, abonnement) = anwesenheit.stand_mit(|| bus.abonnieren());
        let mut strom = Strom {
            anwesenheit: Arc::clone(&anwesenheit),
            abonnement,
            ende: Ende::nie(),
            herzschlag: tokio::time::interval_at(
                tokio::time::Instant::now() + Duration::from_secs(3600),
                Duration::from_secs(3600),
            ),
            zuletzt: stand.stand,
            erstes: Some(stand),
            eigenes_weg_gesendet: false,
            halter,
        };
        for nummer in 0..5 {
            beitreten(&anwesenheit, &format!("w{nummer}"));
        }

        let mut staende = Vec::new();
        let mut neuer_stand = None;
        for _ in 0..3 {
            match strom.naechste_ausgabe().await.expect("Ausgabe") {
                Ausgabe::Stand(stand) if staende.is_empty() => staende.push(stand.stand),
                Ausgabe::Stand(stand) => {
                    neuer_stand = Some(stand.stand);
                    staende.push(stand.stand);
                }
                Ausgabe::Wechsel(wechsel) => staende.push(wechsel.stand),
                Ausgabe::Herzschlag => {}
            }
            if neuer_stand.is_some() {
                break;
            }
        }
        let gepuffert =
            tokio::time::timeout(Duration::from_millis(100), strom.naechste_ausgabe()).await;
        beitreten(&anwesenheit, "danach");
        let danach = tokio::time::timeout(Duration::from_secs(1), strom.naechste_ausgabe())
            .await
            .expect("Ausgabe binnen 1 s");

        let neuer_stand = neuer_stand.expect("neuer Stand nach Lagged");
        assert!(
            gepuffert.is_err(),
            "nach S' = {neuer_stand} kam noch {gepuffert:?} (Folge {staende:?})"
        );
        let Some(Ausgabe::Wechsel(wechsel)) = danach else {
            panic!(
                "erwartet ein Wechsel über S' = {neuer_stand}, kam {danach:?} (Folge {staende:?})"
            );
        };
        assert_eq!(wechsel.stand, neuer_stand + 1, "Folge {staende:?}");
        assert!(
            staende.windows(2).all(|paar| paar[0] < paar[1]),
            "{staende:?}"
        );
    }

    /// B0115 (ADR-0018 Regel 4): der Strom sendet das `weg` seiner eigenen Sitzung noch und endet dann; ein
    /// fremdes `weg` beendet ihn nicht.
    #[tokio::test]
    async fn nach_dem_eigenen_weg_endet_der_strom_ein_fremdes_laesst_ihn_laufen() {
        let bus = Arc::new(Bus::neu(16));
        let anwesenheit = Arc::new(
            Anwesenheit::neu(
                Arc::new(Systemuhr),
                Arc::new(Betriebssystemzufall),
                Duration::from_secs(60),
            )
            .mit_veroeffentlichen(anwesenheit_veroeffentlichen(&bus)),
        );
        let herkunft: std::net::IpAddr = "127.0.0.2".parse().expect("IP");
        let eigenes = beitreten(&anwesenheit, "leser");
        let fremdes = beitreten(&anwesenheit, "anderer");
        let (halter, _) = anwesenheit.verbunden(&eigenes, herkunft).expect("Halter");
        let (stand, abonnement) = anwesenheit.stand_mit(|| bus.abonnieren());
        let strom = Strom {
            anwesenheit: Arc::clone(&anwesenheit),
            abonnement,
            ende: Ende::nie(),
            herzschlag: tokio::time::interval_at(
                tokio::time::Instant::now() + Duration::from_secs(3600),
                Duration::from_secs(3600),
            ),
            zuletzt: stand.stand,
            erstes: Some(stand),
            eigenes_weg_gesendet: false,
            halter,
        };
        anwesenheit.verlassen(&fremdes, herkunft);
        anwesenheit.verlassen(&eigenes, herkunft);

        let mut strom = Some(strom);
        let mut gesendet = 0;
        while let Some(laufend) = strom.take() {
            let naechstes = tokio::time::timeout(Duration::from_secs(1), laufend.naechstes())
                .await
                .expect("Strom endet, statt zu warten");
            strom = naechstes.map(|(_, weiter)| weiter);
            gesendet += usize::from(strom.is_some());
        }

        assert_eq!(gesendet, 3, "Stand, weg anderer, weg leser — dann Ende");
    }
}
