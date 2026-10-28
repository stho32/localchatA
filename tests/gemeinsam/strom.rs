//! Ein offener Ereignisstrom im Test (aus `tests/ereignisse.rs` gezogen, geteilt mit `abriss.rs` und
//! `verlassen.rs`): liest Teilstücke, zerlegt sie in Ereignisse und merkt sich die Ankunft.

use super::Absender;
use serde_json::Value;
use std::time::Duration;
use tokio::time::Instant;

/// Ein Ereignis, wie der Strom es liefert: `event` (leer für `data`-Ereignisse), `data` als JSON, Ankunft.
#[derive(Debug, Clone)]
pub struct Empfangen {
    pub event: String,
    pub data: Value,
    pub ankunft: Instant,
}

/// Ein offener Ereignisstrom: liest Teilstücke und zerlegt sie in Ereignisse; `drop` schließt ihn.
pub struct Strom {
    antwort: reqwest::Response,
    rest: String,
    pub ereignisse: Vec<Empfangen>,
}

impl Strom {
    pub async fn oeffnen(absender: &Absender, basis: &str, token: &str) -> Strom {
        let antwort = absender
            .anfrage(reqwest::Method::GET, format!("{basis}api/ereignisse"))
            .header("X-LocalchatA-Sitzung", token)
            .send()
            .await
            .expect("Strom");
        assert_eq!(antwort.status(), 200);
        Strom {
            antwort,
            rest: String::new(),
            ereignisse: Vec::new(),
        }
    }

    /// Liest bis zum nächsten Ereignis, das `passt`, höchstens `hoechstens`; alle gelesenen bleiben in
    /// `ereignisse`. `None`, wenn keines kam oder der Strom endete.
    pub async fn bis(
        &mut self,
        hoechstens: Duration,
        passt: impl Fn(&Empfangen) -> bool,
    ) -> Option<Empfangen> {
        let ende = Instant::now() + hoechstens;
        let mut gelesen = self.ereignisse.len();
        loop {
            if let Some(treffer) = self.ereignisse[gelesen..].iter().find(|e| passt(e)) {
                return Some(treffer.clone());
            }
            gelesen = self.ereignisse.len();
            let stueck = tokio::time::timeout_at(ende, self.antwort.chunk()).await;
            match stueck {
                Ok(Ok(Some(bytes))) => self.zerlegen(&String::from_utf8_lossy(&bytes)),
                _ => return None,
            }
        }
    }

    /// Liest `dauer` lang alles mit; `true`, wenn der Strom dabei endete.
    pub async fn mitlesen(&mut self, dauer: Duration) -> bool {
        let ende = Instant::now() + dauer;
        loop {
            match tokio::time::timeout_at(ende, self.antwort.chunk()).await {
                Ok(Ok(Some(bytes))) => self.zerlegen(&String::from_utf8_lossy(&bytes)),
                Ok(_) => return true,
                Err(_) => return false,
            }
        }
    }

    fn zerlegen(&mut self, stueck: &str) {
        self.rest.push_str(stueck);
        while let Some(stelle) = self.rest.find("\n\n") {
            let block: String = self.rest.drain(..stelle + 2).collect();
            let mut event = String::new();
            let mut data = String::new();
            for zeile in block.lines() {
                if let Some(wert) = zeile.strip_prefix("event:") {
                    event = wert.trim().to_string();
                } else if let Some(wert) = zeile.strip_prefix("data:") {
                    data.push_str(wert.trim());
                }
            }
            self.ereignisse.push(Empfangen {
                event,
                data: serde_json::from_str(&data).unwrap_or(Value::Null),
                ankunft: Instant::now(),
            });
        }
    }

    pub fn wechsel_von(&self, kennung: &str) -> Vec<Empfangen> {
        self.ereignisse
            .iter()
            .filter(|e| {
                e.data["art"] == "anwesenheit" && e.data["teilnehmer"]["kennung"] == kennung
            })
            .cloned()
            .collect()
    }

    pub fn staende(&self) -> Vec<u64> {
        self.ereignisse
            .iter()
            .filter_map(|e| e.data["stand"].as_u64())
            .collect()
    }
}

pub fn ist_wechsel(kennung: &'static str, status: &'static str) -> impl Fn(&Empfangen) -> bool {
    move |e| {
        e.data["art"] == "anwesenheit"
            && e.data["teilnehmer"]["kennung"] == kennung
            && e.data["teilnehmer"]["status"] == status
    }
}
