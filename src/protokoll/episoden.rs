//! Episoden (ADR-0042 Regel 5 mit Ergänzungen 3, 5 und Festlegung (3)): gleichartige Auftreten einer Quelle als
//! „Episode beginnt“, nur gezählt, alle 10 Minuten „läuft weiter“, nach der Ruhe eine Zusammenfassung in der Stufe
//! der Episode. Rein: die Tabelle liefert die Einträge, das Protokoll schreibt sie.

use super::{Bereich, Eintrag, Kontext, Schluessel, Stufe};
use crate::uhr::Zeit;
use std::collections::{BTreeMap, HashMap};
use std::time::Duration;

/// Ruhe bis zum Ende einer Episode, wenn nichts anderes eingestellt ist.
pub const RUHE: Duration = Duration::from_secs(60);
/// Abstand der Zwischenzeilen.
pub const ZWISCHEN: Duration = Duration::from_secs(600);
/// Höchstens so viele Episoden sind offen; weitere Quellen zählen in eine Sammelepisode.
pub const HOECHSTENS_OFFEN: usize = 1024;

/// Ein Auftreten: Stufe und Bereich der Episode, Text des Ereignisses, Quelle (Herkunft, Kennung oder
/// Datei+Grund) und der Kontext der ersten Zeile; bei Körperfehlern Art, Verwerfen und gelesene Bytes.
#[derive(Debug, Clone)]
pub struct Auftreten {
    pub stufe: Stufe,
    pub bereich: Bereich,
    pub text: &'static str,
    pub quelle: String,
    pub kontext: Kontext,
    pub art: Option<&'static str>,
    pub verworfen: Option<&'static str>,
    pub bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Schluessel3 {
    bereich: Bereich,
    text: &'static str,
    quelle: Option<String>,
}

struct Episode {
    stufe: Stufe,
    bereich: Bereich,
    text: &'static str,
    sammel: bool,
    kontext: Kontext,
    erst: Zeit,
    zuletzt: Zeit,
    naechste_zwischen: Zeit,
    anzahl: u64,
    je_art: BTreeMap<&'static str, u64>,
    je_verworfen: BTreeMap<&'static str, u64>,
    bytes: Option<u64>,
}

/// Die offenen Episoden.
pub struct Episoden {
    ruhe: Duration,
    offen: HashMap<Schluessel3, Episode>,
}

impl Episoden {
    pub fn neu(ruhe: Duration) -> Episoden {
        Episoden {
            ruhe,
            offen: HashMap::new(),
        }
    }

    /// Ein Auftreten um `jetzt`; liefert die Zeilen, die dabei entstehen (Beginn oder Zwischenzeile).
    pub fn auftreten(&mut self, auftreten: Auftreten, jetzt: Zeit) -> Vec<Eintrag> {
        let mut schluessel = Schluessel3 {
            bereich: auftreten.bereich,
            text: auftreten.text,
            quelle: Some(auftreten.quelle.clone()),
        };
        let offen_ohne_sammel = self.offen.keys().filter(|s| s.quelle.is_some()).count();
        let sammel = !self.offen.contains_key(&schluessel) && offen_ohne_sammel >= HOECHSTENS_OFFEN;
        if sammel {
            schluessel.quelle = None;
        }
        if let Some(episode) = self.offen.get_mut(&schluessel) {
            episode.zaehlen(&auftreten, jetzt);
            return episode.zwischenzeile(jetzt).into_iter().collect();
        }
        let episode = Episode::beginnen(auftreten, jetzt, sammel);
        let beginn = episode.zeile("Episode beginnt".to_string(), episode.kontext.clone());
        self.offen.insert(schluessel, episode);
        vec![beginn]
    }

    /// Zählt ein Auftreten nur, wenn seine Episode schon offen ist (`Abgebrochen` in der Körper-Episode).
    pub fn falls_offen_zaehlen(&mut self, auftreten: &Auftreten, jetzt: Zeit) {
        let schluessel = Schluessel3 {
            bereich: auftreten.bereich,
            text: auftreten.text,
            quelle: Some(auftreten.quelle.clone()),
        };
        if let Some(episode) = self.offen.get_mut(&schluessel) {
            episode.zaehlen(auftreten, jetzt);
        }
    }

    /// Schließt jede Episode, deren letztes Auftreten mindestens die Ruhe zurückliegt; liefert die Zeilen.
    pub fn schliessen(&mut self, jetzt: Zeit) -> Vec<Eintrag> {
        let ruhe = chrono::Duration::from_std(self.ruhe).unwrap_or(chrono::Duration::MAX);
        let mut zeilen: Vec<Eintrag> = self
            .offen
            .values_mut()
            .filter_map(|episode| episode.zwischenzeile(jetzt))
            .collect();
        let mut endend: Vec<Schluessel3> = self
            .offen
            .iter()
            .filter(|(_, episode)| jetzt - episode.zuletzt >= ruhe)
            .map(|(schluessel, _)| schluessel.clone())
            .collect();
        endend.sort_by(|a, b| a.quelle.cmp(&b.quelle));
        for schluessel in endend {
            if let Some(episode) = self.offen.remove(&schluessel) {
                zeilen.push(episode.ende());
            }
        }
        zeilen
    }
}

impl Episode {
    fn beginnen(auftreten: Auftreten, jetzt: Zeit, sammel: bool) -> Episode {
        let zwischen = chrono::Duration::from_std(ZWISCHEN).unwrap_or(chrono::Duration::MAX);
        let mut episode = Episode {
            stufe: auftreten.stufe,
            bereich: auftreten.bereich,
            text: auftreten.text,
            sammel,
            kontext: if sammel {
                Kontext::neu()
            } else {
                auftreten.kontext.clone()
            },
            erst: jetzt,
            zuletzt: jetzt,
            naechste_zwischen: jetzt + zwischen,
            anzahl: 0,
            je_art: BTreeMap::new(),
            je_verworfen: BTreeMap::new(),
            bytes: None,
        };
        episode.zaehlen(&auftreten, jetzt);
        episode
    }

    /// Zählt; `zuletzt` fällt nie (eine zurückspringende Uhr beendet nichts vorzeitig).
    fn zaehlen(&mut self, auftreten: &Auftreten, jetzt: Zeit) {
        self.anzahl += 1;
        self.zuletzt = self.zuletzt.max(jetzt);
        if let Some(art) = auftreten.art {
            *self.je_art.entry(art).or_default() += 1;
        }
        if let Some(verworfen) = auftreten.verworfen {
            *self.je_verworfen.entry(verworfen).or_default() += 1;
        }
        if let Some(bytes) = auftreten.bytes {
            self.bytes = Some(self.bytes.map_or(bytes, |bisher| bisher.max(bytes)));
        }
    }

    /// Je 10 Minuten seit Beginn eine Zeile „läuft weiter — n seit Beginn“.
    fn zwischenzeile(&mut self, jetzt: Zeit) -> Option<Eintrag> {
        if jetzt < self.naechste_zwischen {
            return None;
        }
        let zwischen = chrono::Duration::from_std(ZWISCHEN).unwrap_or(chrono::Duration::MAX);
        while self.naechste_zwischen <= jetzt {
            self.naechste_zwischen += zwischen;
        }
        Some(self.zeile(
            format!("läuft weiter — {} seit Beginn", self.anzahl),
            self.kontext.clone(),
        ))
    }

    /// „Episode endet: n in d s“, d vom ersten bis zum letzten Auftreten, aufgerundet; bei Körperfehlern je Art
    /// und Verwerfen, dazu die größte Bytezahl.
    fn ende(&self) -> Eintrag {
        let millis = (self.zuletzt - self.erst).num_milliseconds().max(0);
        let sekunden = (millis + 999) / 1000;
        let mut text = format!("Episode endet: {} in {sekunden} s", self.anzahl);
        let teile: Vec<String> = self
            .je_art
            .iter()
            .map(|(art, n)| format!("{art} {n}"))
            .chain(
                self.je_verworfen
                    .iter()
                    .map(|(verworfen, n)| format!("verworfen {verworfen} {n}")),
            )
            .collect();
        if !teile.is_empty() {
            text.push_str(&format!(" ({})", teile.join("; ")));
        }
        let mut kontext = self.kontext.clone();
        if let Some(bytes) = self.bytes {
            kontext = kontext.mit(Schluessel::Bytes, bytes);
        }
        self.zeile(text, kontext)
    }

    fn zeile(&self, zusatz: String, kontext: Kontext) -> Eintrag {
        let quelle = if self.sammel {
            " — weitere Quellen"
        } else {
            ""
        };
        Eintrag::neu(
            self.stufe,
            self.bereich,
            format!("{}{quelle} — {zusatz}", self.text),
            kontext,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protokoll::testhilfe::{im_speicher, zeit};

    fn t(sekunden: f64) -> Zeit {
        zeit("2026-10-28T12:00:00+01:00")
            + chrono::Duration::milliseconds((sekunden * 1000.0).round() as i64)
    }

    fn grenzschutz(herkunft: &str) -> Auftreten {
        Auftreten {
            stufe: Stufe::Warning,
            bereich: Bereich::Grenzschutz,
            text: "Anfrage mit fremdem Host abgewiesen",
            quelle: herkunft.to_string(),
            kontext: Kontext::neu()
                .mit(Schluessel::Herkunft, herkunft)
                .mit(Schluessel::Grund, "Host boese.example"),
            art: None,
            verworfen: None,
            bytes: None,
        }
    }

    fn texte(eintraege: &[Eintrag]) -> Vec<String> {
        eintraege
            .iter()
            .map(|e| format!("{} {}", e.stufe.als_text(), e.text))
            .collect()
    }

    #[test]
    fn hundert_auftreten_in_zehn_sekunden_ergeben_beginn_und_nach_60_s_ruhe_ein_ende_in_der_stufe_der_episode()
     {
        let mut episoden = Episoden::neu(RUHE);
        let mut zeilen = Vec::new();
        for n in 0..100 {
            zeilen.extend(episoden.auftreten(grenzschutz("10.0.0.9"), t(f64::from(n) / 10.0)));
            zeilen.extend(episoden.schliessen(t(f64::from(n) / 10.0)));
        }
        assert_eq!(
            texte(&zeilen),
            ["WARNING Anfrage mit fremdem Host abgewiesen — Episode beginnt"]
        );
        assert_eq!(
            zeilen[0].kontext.wert(Schluessel::Herkunft),
            Some("10.0.0.9")
        );

        assert!(episoden.schliessen(t(9.9 + 59.999)).is_empty());
        let ende = episoden.schliessen(t(9.9 + 60.0));
        assert_eq!(
            texte(&ende),
            ["WARNING Anfrage mit fremdem Host abgewiesen — Episode endet: 100 in 10 s"]
        );
        assert_eq!(ende[0].kontext.wert(Schluessel::Herkunft), Some("10.0.0.9"));
        assert!(
            episoden.schliessen(t(500.0)).is_empty(),
            "einmal geschlossen"
        );
    }

    #[test]
    fn eine_lange_episode_hat_eine_zwischenzeile_je_10_minuten() {
        let mut episoden = Episoden::neu(RUHE);
        let mut zeilen = Vec::new();
        for sekunde in 0..1500 {
            zeilen.extend(episoden.auftreten(grenzschutz("10.0.0.9"), t(f64::from(sekunde))));
            zeilen.extend(episoden.schliessen(t(f64::from(sekunde))));
        }
        zeilen.extend(episoden.schliessen(t(1499.0 + 60.0)));

        assert_eq!(
            texte(&zeilen),
            [
                "WARNING Anfrage mit fremdem Host abgewiesen — Episode beginnt",
                "WARNING Anfrage mit fremdem Host abgewiesen — läuft weiter — 601 seit Beginn",
                "WARNING Anfrage mit fremdem Host abgewiesen — läuft weiter — 1201 seit Beginn",
                "WARNING Anfrage mit fremdem Host abgewiesen — Episode endet: 1500 in 1499 s",
            ]
        );
    }

    #[test]
    fn hoechstens_1024_offene_weitere_quellen_in_einer_sammelepisode() {
        let mut episoden = Episoden::neu(RUHE);
        let mut zeilen = Vec::new();
        for n in 0..1030 {
            zeilen.extend(episoden.auftreten(
                grenzschutz(&format!("10.0.{}.{}", n / 256, n % 256)),
                t(0.0),
            ));
        }
        let ende = episoden.schliessen(t(60.0));

        let beginn = zeilen
            .iter()
            .filter(|e| e.text.ends_with("— Episode beginnt"))
            .count();
        assert_eq!(beginn, 1025);
        let sammel: Vec<&Eintrag> = zeilen
            .iter()
            .filter(|e| e.text.contains("weitere Quellen — Episode beginnt"))
            .collect();
        assert_eq!(sammel.len(), 1);
        assert_eq!(sammel[0].kontext.wert(Schluessel::Herkunft), None);
        let sammelende: Vec<&Eintrag> = ende
            .iter()
            .filter(|e| e.text.contains("weitere Quellen"))
            .collect();
        assert_eq!(sammelende.len(), 1);
        assert!(
            sammelende[0].text.ends_with("Episode endet: 6 in 0 s"),
            "{}",
            sammelende[0].text
        );
        assert_eq!(ende.len(), 1025);
    }

    #[test]
    fn eine_zurueckspringende_uhr_beendet_keine_episode_vorzeitig() {
        let mut episoden = Episoden::neu(RUHE);
        episoden.auftreten(grenzschutz("10.0.0.9"), t(10.0));

        for sekunde in 0..=120 {
            assert!(
                episoden
                    .schliessen(t(-3600.0 + f64::from(sekunde)))
                    .is_empty()
            );
        }
        assert!(episoden.schliessen(t(69.999)).is_empty());
        assert_eq!(episoden.schliessen(t(70.0)).len(), 1);
    }

    #[test]
    fn koerper_episoden_nennen_arten_verwerfen_und_groesste_bytes() {
        let mut episoden = Episoden::neu(Duration::from_secs(1));
        for (art, bytes) in [("zu_gross", 5120), ("zu_gross", 6000), ("kaputt", 1)] {
            episoden.auftreten(
                Auftreten {
                    stufe: Stufe::Warning,
                    bereich: Bereich::Koerper,
                    text: "Körper abgewiesen",
                    quelle: "127.0.0.1 POST /api/beitreten".into(),
                    kontext: Kontext::neu().mit(Schluessel::Herkunft, "127.0.0.1"),
                    art: Some(art),
                    verworfen: (art == "zu_gross").then_some("vollständig"),
                    bytes: Some(bytes),
                },
                t(0.0),
            );
        }
        let ende = episoden.schliessen(t(1.0));

        assert_eq!(
            ende[0].text,
            "Körper abgewiesen — Episode endet: 3 in 0 s (kaputt 1; zu_gross 2; verworfen vollständig 2)"
        );
        assert_eq!(ende[0].kontext.wert(Schluessel::Bytes), Some("6000"));
    }

    #[test]
    fn das_protokoll_schreibt_die_zeilen_und_zaehlt_jedes_auftreten() {
        let (protokoll, speicher, uhr, _) =
            im_speicher("2026-10-28T12:00:00+01:00", Stufe::Warning);
        for _ in 0..3 {
            protokoll.episode(grenzschutz("10.0.0.9"));
        }
        uhr.stellen(t(60.0));
        protokoll.episoden_schliessen();

        let zeilen = speicher.zeilen();
        assert_eq!(zeilen.len(), 2, "{zeilen:#?}");
        assert!(zeilen[0].contains(" WARNING grenzschutz Anfrage mit fremdem Host abgewiesen — Episode beginnt | herkunft=10.0.0.9"));
        assert!(zeilen[1].contains(" WARNING grenzschutz Anfrage mit fremdem Host abgewiesen — Episode endet: 3 in 0 s | herkunft=10.0.0.9"));
        assert_eq!(protokoll.zaehlerstand().warnings, 3);
    }
}
