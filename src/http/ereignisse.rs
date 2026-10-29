//! Ereignisse beantworten (B0084, ADR-0015 Regel 1, ADR-0016 Regel 5 und 8): `GET /api/ereignisse` mit der
//! Sitzung im Header → `text/event-stream`. Der offene Strom ist ein Halter seiner Sitzung; er beginnt mit
//! dem Stand der Teilnehmerliste und dem Stand der Channels (ADR-0023 Regel 4), danach je Wechsel ein Ereignis und alle `herzschlag` ein Lebenszeichen,
//! bis der Client schließt, das Programm endet oder die eigene Sitzung `weg` ist (verlassen, B0115: das `weg`
//! geht noch hinaus, dann endet der Strom). Ein Token im Query zählt nicht.

use super::Zustand;
use super::abbruch::{Abgebrochen, Grenzort, abbruch_beantworten, abbruch_eintragen};
use super::channels::{channel_als_json, stand_als_json as channels_als_json};
use super::nachrichten::NachrichtAntwort;
use super::teilnehmer::{gemeldetes_token, nicht_erkannt};
use super::teilnehmerliste::{stand_als_json, zeile_als_json, zeit_als_text};
use crate::anwesenheit::status::Status;
use crate::anwesenheit::{Anwesenheit, Halter, Stand, Wechsel};
use crate::gespraeche::{ChannelStand, ChannelWechsel, Nachricht};
use crate::live::bus::{Abonnement, Empfang, Ereignis};
use crate::protokoll::Protokoll;
use crate::start::ende::Ende;
use axum::extract::{ConnectInfo, State};
use axum::http::HeaderMap;
use axum::response::sse::{Event, Sse};
use axum::response::{IntoResponse, Response};
use futures_util::stream;
use serde_json::json;
use std::collections::VecDeque;
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
    let gespraeche = Arc::clone(&zustand.gespraeche);
    strom_beantworten(&zustand, halter, Arc::new(move || gespraeche.channels())).await
}

/// Woher der Strom den Stand der Channels holt — im Programm aus den Gesprächen.
type Channelquelle = Arc<dyn Fn() -> ChannelStand + Send + Sync>;

/// Wo der Stand der Channels geholt wird (ADR-0042 Ergänzung #24 Punkt 1).
fn channels_ort(kennung: &str) -> Grenzort<'_> {
    Grenzort {
        route: "GET /api/ereignisse",
        operation: "channels_stand",
        kennung: Some(kennung),
        channel: None,
    }
}

/// Der Strom eines erkannten Halters; scheitert der Stand der Channels davor, 500 mit Eintrag.
async fn strom_beantworten(zustand: &Zustand, halter: Halter, quelle: Channelquelle) -> Response {
    let (stand, abonnement) = zustand.anwesenheit.stand_mit(|| zustand.bus.abonnieren());
    let kennung = halter.kennung().als_text().to_string();
    let channels = match channels_holen(&quelle).await {
        Ok(channels) => channels,
        Err(abbruch) => {
            return abbruch_beantworten(
                &zustand.protokoll,
                &abbruch,
                &channels_ort(&kennung),
                "Stand der Channels nicht geholt",
            );
        }
    };
    let herzschlag = zustand.herzschlag;
    let strom = Strom {
        anwesenheit: Arc::clone(&zustand.anwesenheit),
        channels: quelle,
        protokoll: Arc::clone(&zustand.protokoll),
        kennung,
        abonnement,
        ende: zustand.ende.clone(),
        herzschlag: tokio::time::interval_at(tokio::time::Instant::now() + herzschlag, herzschlag),
        zuletzt: stand.stand,
        zuletzt_channels: channels.stand,
        ausstehend: VecDeque::from([Ausgabe::Stand(stand), Ausgabe::Channels(channels)]),
        eigenes_weg_gesendet: false,
        halter,
    };
    Sse::new(stream::unfold(strom, Strom::naechstes)).into_response()
}

/// Der Stand der Channels unter der Sperre des Verzeichnisses — eine `std`-Sperre, darum über `spawn_blocking`
/// (ADR-0021 Regel 7); `Err` nur, wenn das Holen abbrach.
async fn channels_holen(channels: &Channelquelle) -> Result<ChannelStand, Abgebrochen> {
    let channels = Arc::clone(channels);
    tokio::task::spawn_blocking(move || channels())
        .await
        .map_err(Abgebrochen::from)
}

/// Was ein offener Strom hält; mit ihm fällt der Halter (Client weg oder Ende des Programms).
struct Strom {
    anwesenheit: Arc<Anwesenheit>,
    channels: Channelquelle,
    protokoll: Arc<Protokoll>,
    /// Kennung des Halters — für die Spur, wenn der Strom am Stand der Channels endet.
    kennung: String,
    abonnement: Abonnement,
    ende: Ende,
    herzschlag: Interval,
    /// Nummer des zuletzt gesendeten Stands oder Wechsels der Teilnehmer; ältere oder gleiche gehen schon im
    /// Stand auf.
    zuletzt: u64,
    /// Dasselbe für das Thema Channels (ADR-0023 Regel 5).
    zuletzt_channels: u64,
    /// Was vor dem nächsten Empfang hinaus muss: zu Beginn und nach einem neuen Stand beide Stände.
    ausstehend: VecDeque<Ausgabe>,
    /// Das `weg` der eigenen Sitzung ist hinaus — danach endet der Strom (ADR-0018 Regel 4).
    eigenes_weg_gesendet: bool,
    halter: Halter,
}

/// Was der Strom als Nächstes sendet.
#[derive(Debug)]
enum Ausgabe {
    Stand(Stand),
    Wechsel(Wechsel),
    Channels(ChannelStand),
    Channel(ChannelWechsel),
    Nachricht {
        channel: String,
        nachricht: Nachricht,
    },
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
            Ausgabe::Channels(stand) => channels_ereignis(&stand),
            Ausgabe::Channel(wechsel) => channel_ereignis(&wechsel),
            Ausgabe::Nachricht { channel, nachricht } => nachricht_ereignis(&channel, &nachricht),
            Ausgabe::Herzschlag => herzschlag_ereignis(&self.anwesenheit),
        };
        Some((Ok(ereignis), self))
    }

    /// Zuerst die ausstehenden Stände, danach je Thema Wechsel mit größerem `stand` als dem zuletzt gesendeten
    /// (kleinere oder gleiche stecken schon im Stand — nach `Lagged` bringt der Bus sie noch), Herzschläge;
    /// `None` am Ende.
    async fn naechste_ausgabe(&mut self) -> Option<Ausgabe> {
        if let Some(ausstehend) = self.ausstehend.pop_front() {
            return Some(ausstehend);
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
                    Empfang::Ereignis(Ereignis::Channel(wechsel)) => {
                        if wechsel.stand <= self.zuletzt_channels {
                            continue;
                        }
                        self.zuletzt_channels = wechsel.stand;
                        Ausgabe::Channel(wechsel)
                    }
                    Empfang::Ereignis(Ereignis::Nachricht { channel, nachricht }) => {
                        Ausgabe::Nachricht { channel, nachricht }
                    }
                    Empfang::NeuerStand => {
                        let stand = self.anwesenheit.teilnehmer();
                        self.zuletzt = stand.stand;
                        let channels = match channels_holen(&self.channels).await {
                            Ok(channels) => channels,
                            Err(abbruch) => {
                                abbruch_eintragen(
                                    &self.protokoll,
                                    &abbruch,
                                    &channels_ort(&self.kennung),
                                    "Ereignisstrom beendet — Stand der Channels nicht geholt; der Client verbindet neu",
                                );
                                return None;
                            }
                        };
                        self.zuletzt_channels = channels.stand;
                        self.ausstehend.push_back(Ausgabe::Channels(channels));
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

fn channels_ereignis(stand: &ChannelStand) -> Event {
    let mut stand = channels_als_json(stand);
    stand["art"] = json!("channels");
    Event::default().data(stand.to_string())
}

fn channel_ereignis(wechsel: &ChannelWechsel) -> Event {
    let ereignis = json!({
        "art": "channel",
        "stand": wechsel.stand,
        "channel": channel_als_json(&wechsel.channel),
    });
    Event::default().data(ereignis.to_string())
}

/// `{"art":"nachricht","channel":…,"nachricht":{zeit, von, art, text}}` — dieselbe Form wie Verlauf und 201.
fn nachricht_ereignis(channel: &str, nachricht: &Nachricht) -> Event {
    let ereignis = json!({
        "art": "nachricht",
        "channel": channel,
        "nachricht": NachrichtAntwort::aus(nachricht),
    });
    Event::default().data(ereignis.to_string())
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
    use crate::gespraeche::{Gespraeche, Kennung, Speicherablage};
    use crate::live::bus::{Bus, anwesenheit_veroeffentlichen, channels_veroeffentlichen};
    use crate::uhr::Systemuhr;
    use crate::zufall::Betriebssystemzufall;
    use axum::http::StatusCode;
    use std::time::Duration;

    /// Gespräche im Speicher, deren Channel-Wechsel auf `bus` gehen.
    fn gespraeche_am(bus: &Arc<Bus>) -> Arc<Gespraeche> {
        let gespraeche = Gespraeche::laden(Arc::new(Speicherablage::neu()), Arc::new(Systemuhr))
            .expect("geladen")
            .mit_veroeffentlichen(channels_veroeffentlichen(bus));
        Arc::new(gespraeche)
    }

    /// Ein Strom wie im Handler, ohne Herzschlag in Reichweite des Tests.
    fn strom_fuer(
        anwesenheit: &Arc<Anwesenheit>,
        gespraeche: &Arc<Gespraeche>,
        stand: Stand,
        abonnement: Abonnement,
        halter: Halter,
    ) -> Strom {
        let channels = gespraeche.channels();
        let quelle = Arc::clone(gespraeche);
        Strom {
            anwesenheit: Arc::clone(anwesenheit),
            channels: Arc::new(move || quelle.channels()),
            protokoll: Arc::new(crate::protokoll::Protokoll::ohne_ablage()),
            kennung: halter.kennung().als_text().to_string(),
            abonnement,
            ende: Ende::nie(),
            herzschlag: tokio::time::interval_at(
                tokio::time::Instant::now() + Duration::from_secs(3600),
                Duration::from_secs(3600),
            ),
            zuletzt: stand.stand,
            zuletzt_channels: channels.stand,
            ausstehend: VecDeque::from([Ausgabe::Stand(stand), Ausgabe::Channels(channels)]),
            eigenes_weg_gesendet: false,
            halter,
        }
    }

    fn beitreten(anwesenheit: &Anwesenheit, name: &str) -> crate::zufall::Token {
        beitreten_von(anwesenheit, name, "127.0.0.2".parse().expect("IP"))
    }

    fn beitreten_von(
        anwesenheit: &Anwesenheit,
        name: &str,
        herkunft: std::net::IpAddr,
    ) -> crate::zufall::Token {
        let wunsch = Beitrittswunsch::pruefen(&json!({ "name": name })).expect("Wunsch");
        anwesenheit
            .beitreten(&wunsch, herkunft)
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
        let mut strom = strom_fuer(
            &anwesenheit,
            &gespraeche_am(&bus),
            stand,
            abonnement,
            halter,
        );
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
                Ausgabe::Channels(_)
                | Ausgabe::Channel(_)
                | Ausgabe::Nachricht { .. }
                | Ausgabe::Herzschlag => {}
            }
            if neuer_stand.is_some() {
                break;
            }
        }
        let channels_danach =
            tokio::time::timeout(Duration::from_millis(100), strom.naechste_ausgabe()).await;
        let gepuffert =
            tokio::time::timeout(Duration::from_millis(100), strom.naechste_ausgabe()).await;
        beitreten(&anwesenheit, "danach");
        let danach = tokio::time::timeout(Duration::from_secs(1), strom.naechste_ausgabe())
            .await
            .expect("Ausgabe binnen 1 s");

        let neuer_stand = neuer_stand.expect("neuer Stand nach Lagged");
        assert!(
            matches!(channels_danach, Ok(Some(Ausgabe::Channels(_)))),
            "nach S' gehört der Stand der Channels dazu (ADR-0023 Regel 5), kam {channels_danach:?}"
        );
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
        let strom = strom_fuer(
            &anwesenheit,
            &gespraeche_am(&bus),
            stand,
            abonnement,
            halter,
        );
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

        assert_eq!(
            gesendet, 4,
            "Stand der Teilnehmer, Stand der Channels, weg anderer, weg leser — dann Ende"
        );
    }

    /// ADR-0023 Regel 5: wer zu langsam liest, bekommt beide Stände neu — danach nur Channel-Wechsel über dem
    /// neuen Stand der Channels, auch wenn der Bus ältere noch liefert.
    #[tokio::test]
    async fn nach_einem_neuen_stand_kommen_beide_staende_und_nur_juengere_channel_wechsel() {
        let bus = Arc::new(Bus::neu(2));
        let anwesenheit = Arc::new(
            Anwesenheit::neu(
                Arc::new(Systemuhr),
                Arc::new(Betriebssystemzufall),
                Duration::from_secs(60),
            )
            .mit_veroeffentlichen(anwesenheit_veroeffentlichen(&bus)),
        );
        let gespraeche = gespraeche_am(&bus);
        let token = beitreten(&anwesenheit, "leser");
        let (halter, _) = anwesenheit
            .verbunden(&token, "127.0.0.2".parse().expect("IP"))
            .expect("Halter");
        let (stand, abonnement) = anwesenheit.stand_mit(|| bus.abonnieren());
        let mut strom = strom_fuer(&anwesenheit, &gespraeche, stand, abonnement, halter);
        let alpha = Kennung::bilden("alpha@127.0.0.2").expect("Kennung");
        for nummer in 0..5 {
            gespraeche
                .channel_anlegen(&format!("c{nummer}"), None, &alpha)
                .expect("angelegt");
        }

        let mut folge = Vec::new();
        for _ in 0..4 {
            let ausgabe = tokio::time::timeout(Duration::from_secs(1), strom.naechste_ausgabe())
                .await
                .expect("Ausgabe binnen 1 s")
                .expect("Strom läuft");
            folge.push(match ausgabe {
                Ausgabe::Stand(_) => "teilnehmer".to_string(),
                Ausgabe::Channels(stand) => format!("channels {}", stand.stand),
                other => format!("{other:?}"),
            });
        }
        let gepuffert =
            tokio::time::timeout(Duration::from_millis(100), strom.naechste_ausgabe()).await;
        gespraeche
            .channel_anlegen("danach", None, &alpha)
            .expect("angelegt");
        let danach = tokio::time::timeout(Duration::from_secs(1), strom.naechste_ausgabe())
            .await
            .expect("Ausgabe binnen 1 s");

        assert_eq!(
            folge,
            vec!["teilnehmer", "channels 0", "teilnehmer", "channels 5"]
        );
        assert!(gepuffert.is_err(), "nach dem neuen Stand kam {gepuffert:?}");
        let Some(Ausgabe::Channel(wechsel)) = danach else {
            panic!("erwartet ein Channel-Wechsel über 5, kam {danach:?}");
        };
        assert_eq!(wechsel.stand, 6);
    }

    /// Ein Stand der Channels, der beim `n`-ten Holen (ab 1) in Panik gerät — davor der echte Stand.
    fn channels_mit_panik_ab(gespraeche: &Arc<Gespraeche>, ab: usize) -> Channelquelle {
        let gespraeche = Arc::clone(gespraeche);
        let geholt = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        Arc::new(move || {
            let nummer = geholt.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
            assert!(nummer < ab, "Probe-Panik beim Stand der Channels");
            gespraeche.channels()
        })
    }

    /// R00018 F0059, erster Test aus ADR-0042 Ergänzung #24 Punkt 1 (vor dem Strom): scheitert der Stand der
    /// Channels, antwortet der Strom 500 mit dem deutschen Satz, und genau eine Zeile steht im Protokoll.
    #[tokio::test]
    async fn scheitert_der_stand_der_channels_vor_dem_strom_ist_es_500_und_eine_zeile() {
        use crate::protokoll::Stufe;
        use crate::protokoll::testhilfe::im_speicher;
        let mut zustand = crate::http::testserver::zustand(std::path::Path::new("/tmp"));
        let (protokoll, speicher, _, _) =
            im_speicher("2026-10-28T12:00:00+01:00", Stufe::Information);
        zustand.protokoll = protokoll;
        let herkunft: std::net::IpAddr = "127.0.0.1".parse().expect("IP");
        let token = beitreten_von(&zustand.anwesenheit, "beta", herkunft);
        let (halter, _) = zustand
            .anwesenheit
            .verbunden(&token, herkunft)
            .expect("Halter");
        let channels = channels_mit_panik_ab(&zustand.gespraeche, 1);

        let antwort = strom_beantworten(&zustand, halter, channels).await;

        assert_eq!(antwort.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let koerper = axum::body::to_bytes(antwort.into_body(), 4096)
            .await
            .expect("Körper");
        let json: serde_json::Value = serde_json::from_slice(&koerper).expect("JSON");
        assert_eq!(
            json["fehler"],
            "Interner Fehler — Stand der Channels nicht geholt; bitte erneut versuchen"
        );
        let zeilen = speicher.zeilen();
        assert_eq!(zeilen.len(), 1, "{zeilen:#?}");
        assert!(
            zeilen[0].contains(" ERROR programmfehler Anfrage an einem Programmfehler gescheitert — Stand der Channels nicht geholt | route=\"GET /api/ereignisse\" operation=channels_stand status=500 kennung=beta@127.0.0.1 grund=panik ursache="),
            "{zeilen:#?}"
        );
    }

    /// R00018 F0059 (im Strom): scheitert der Stand der Channels nach einem neuen Stand, endet der Strom, und
    /// genau eine Zeile „Ereignisstrom beendet …“ steht im Protokoll.
    #[tokio::test]
    async fn scheitert_der_stand_der_channels_im_strom_endet_er_mit_einer_zeile() {
        use crate::protokoll::Stufe;
        use crate::protokoll::testhilfe::im_speicher;
        let (protokoll, speicher, _, _) =
            im_speicher("2026-10-28T12:00:00+01:00", Stufe::Information);
        let bus = Arc::new(Bus::neu(2));
        let anwesenheit = Arc::new(
            Anwesenheit::neu(
                Arc::new(Systemuhr),
                Arc::new(Betriebssystemzufall),
                Duration::from_secs(60),
            )
            .mit_veroeffentlichen(anwesenheit_veroeffentlichen(&bus)),
        );
        let herkunft: std::net::IpAddr = "127.0.0.1".parse().expect("IP");
        let token = beitreten_von(&anwesenheit, "beta", herkunft);
        let (halter, _) = anwesenheit.verbunden(&token, herkunft).expect("Halter");
        let (stand, abonnement) = anwesenheit.stand_mit(|| bus.abonnieren());
        let gespraeche = gespraeche_am(&bus);
        let mut strom = strom_fuer(&anwesenheit, &gespraeche, stand, abonnement, halter);
        strom.channels = channels_mit_panik_ab(&gespraeche, 1);
        strom.protokoll = protokoll;
        for nummer in 0..5 {
            beitreten(&anwesenheit, &format!("w{nummer}"));
        }

        let mut ausgaben = 0;
        while tokio::time::timeout(Duration::from_secs(5), strom.naechste_ausgabe())
            .await
            .expect("Strom endet, statt zu warten")
            .is_some()
        {
            ausgaben += 1;
            assert!(ausgaben < 10, "der Strom endet nicht");
        }

        let zeilen = speicher.zeilen();
        assert_eq!(zeilen.len(), 1, "{zeilen:#?}");
        assert!(
            zeilen[0].contains(" ERROR programmfehler Ereignisstrom beendet — Stand der Channels nicht geholt; der Client verbindet neu | route=\"GET /api/ereignisse\" operation=channels_stand kennung=beta@127.0.0.1 grund=panik ursache="),
            "{zeilen:#?}"
        );
    }
}
