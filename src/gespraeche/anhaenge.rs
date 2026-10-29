//! Anhänge in Gespräche (A00006 Ergänzungen Karte #9, „F1′“): zwei Quoten — die Text-Quote zählt nur Tagesdateien,
//! die Anhang-Quote `Q` gebundene Anhänge `G` und Ansprüche des Eingangs `A`. Gespräche ist einziger Halter der
//! Zähler; jeder Übergang des Eingangs liegt mit `A` unter der Schreibsperre (ADR-0046 Ergänzung „Karte #9 Gate“
//! Regel 5). Die Zähler stehen zusätzlich in Atomics, damit der Health-Check die Sperre nie braucht (ADR-0035
//! Regel 4).

use super::anhangablage::Anhangablage;
use super::anhangeingang::{
    Anmeldung, Beginnfehler, Kopf, Offen, Platz, Verfallen, Verloren, Verworfen,
};
use super::anhangid::Anhangid;
use super::anhangpruefung::Art;
use super::anhangpruefung::{Namensfehler, name_pruefen};
use super::anhangquote::{self, Platzfehler};
use super::anhangverzeichnis::{doppelte_ids, tag_lesen};
use super::{Gespraeche, Kennung, sperren};
use crate::uhr::Zeit;
use std::net::IpAddr;
use std::sync::atomic::{AtomicU64, Ordering};

/// Standard der Anhang-Quote: 20 GiB (Betreiber F1 „z. B. 20 GB“, als GiB wie im Haus).
pub const STANDARD_ANHANGQUOTE: u64 = 20 * 1_073_741_824;

/// Zähler der Anhänge, lesbar ohne Sperre; geschrieben unter der Schreibsperre (Abziehen beim Entfallen auch ohne,
/// das macht eine laufende Prüfung höchstens strenger, wie ADR-0044 Regel 2).
#[derive(Debug, Default)]
pub(super) struct Zaehler {
    pub(super) gebunden: AtomicU64,
    pub(super) reserviert: AtomicU64,
    pub(super) hochladungen: AtomicU64,
    /// 507 `quote` an Anmeldungen seit dem Start, beide Gründe (ADR-0043 Ergänzung „F1′“).
    pub(super) abgewiesen: AtomicU64,
    /// Gelöschte Anhänge seit dem Start (Health `anhaenge_geloescht`).
    pub(super) geloescht: AtomicU64,
}

/// Warum eine Anmeldung abgewiesen wird; nichts ist beansprucht.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Anmeldefehler {
    Name(Namensfehler),
    Platz(Platzfehler),
    Offen(Offen),
    /// Das Betriebssystem lieferte keinen Zufall für die ID.
    Zufall(String),
}

/// Stand der Anhang-Quote für Health und 507: `G`, `A`, offene Hochladungen, Abweisungen, `Q`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Anhangkennzahlen {
    pub anhangquote: Option<u64>,
    pub gebunden: u64,
    pub reserviert: u64,
    pub hochladungen: u64,
    pub abgewiesen: u64,
    pub geloescht: u64,
}

/// Ein Wechsel über bzw. unter 90 % der Anhang-Quote, gemessen an `gebunden` (nie an `gebunden + reserviert`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Anhangschwelle {
    Ueber {
        gebunden: u64,
        hoechstens: u64,
    },
    Unter {
        gebunden: u64,
        hoechstens: u64,
        grund: &'static str,
    },
}

pub type AnhangschwelleMelden = std::sync::Arc<dyn Fn(&Anhangschwelle) + Send + Sync>;

impl Gespraeche {
    /// Hängt die Meldungen der Warnschwelle an (im Start: das Betriebsprotokoll).
    pub fn mit_anhangschwelle_melden(
        self,
        anhangschwelle_melden: AnhangschwelleMelden,
    ) -> Gespraeche {
        Gespraeche {
            anhangschwelle_melden,
            ..self
        }
    }

    /// Warnschwelle (B0515, ADR-0042 Ergänzung „F1′“, Muster ADR-0044 Regel 7): steigt `gebunden` über 90 % der
    /// Anhang-Quote, einmal `Ueber`; fällt es wieder darunter, einmal `Unter` mit dem Grund (`geloescht`, `start`,
    /// `tageswechsel`, `frist_gesetzt`). Ohne Anhang-Quote nichts.
    pub fn anhangschwelle_pruefen(&self, grund: &'static str) {
        let Some(hoechstens) = self.anhangquote else {
            return;
        };
        let gebunden = self.anhangzaehler.gebunden.load(Ordering::SeqCst);
        let ueber = u128::from(gebunden) * 10 > u128::from(hoechstens) * 9;
        if self.anhangschwelle.swap(ueber, Ordering::SeqCst) == ueber {
            return;
        }
        let meldung = if ueber {
            Anhangschwelle::Ueber {
                gebunden,
                hoechstens,
            }
        } else {
            Anhangschwelle::Unter {
                gebunden,
                hoechstens,
                grund,
            }
        };
        (self.anhangschwelle_melden)(&meldung);
    }

    /// Anmelden (ADR-0046 Regel 1 Schritte 3–7 mit Ergänzungen „Karte #9 Gate“ und „F1′“): Name → `zu_gross` →
    /// unter der Schreibsperre offene Hochladungen → Platte → Anhang-Quote (`voll` | `hochladungen`) → Anspruch über
    /// die ganze Länge (`A += z`) und Eintrag `angemeldet`.
    pub fn anhang_anmelden(
        &self,
        kennung: &Kennung,
        herkunft: IpAddr,
        name: &str,
        bytes: u64,
    ) -> Result<Anmeldung, Anmeldefehler> {
        let ascii = name_pruefen(name).map_err(Anmeldefehler::Name)?;
        anhangquote::zu_gross(bytes, self.anhangquote).map_err(Anmeldefehler::Platz)?;
        let frei_platte = self.plattenauskunft.frei().unwrap_or(0);
        let mut stand = sperren(&self.schreiben);
        stand
            .eingang
            .offen_pruefen(kennung, herkunft)
            .map_err(Anmeldefehler::Offen)?;
        let gebunden = self.anhangzaehler.gebunden.load(Ordering::SeqCst);
        anhangquote::platz_pruefen(
            gebunden,
            stand.eingang.reserviert(),
            self.anhangquote,
            bytes,
            frei_platte,
            stand.eingang.offen_zu_schreiben(),
        )
        .map_err(|fehler| {
            if matches!(fehler, Platzfehler::Voll(_) | Platzfehler::Hochladungen(_)) {
                self.anhangzaehler.abgewiesen.fetch_add(1, Ordering::SeqCst);
            }
            Anmeldefehler::Platz(fehler)
        })?;
        let token = self
            .zufall
            .token()
            .map_err(|fehler| Anmeldefehler::Zufall(fehler.0))?;
        let id = Anhangid::aus_token(&token)
            .ok_or_else(|| Anmeldefehler::Zufall("Token ohne 128 Bit".into()))?;
        let anmeldung = stand
            .eingang
            .anmelden(id, kennung, herkunft, name, &ascii, bytes, self.uhr.jetzt())
            .map_err(Anmeldefehler::Offen)?;
        self.eingang_spiegeln(&stand.eingang);
        Ok(anmeldung)
    }

    /// `PUT` beginnt (Regel 2.1/2.2, Ergänzung „Karte #9 Gate“ Regel 3): ein O(1)-Übergang unter der Schreibsperre.
    pub fn hochladen_beginnen(
        &self,
        kennung: &Kennung,
        id: &Anhangid,
        kopf: Kopf,
    ) -> Result<Platz, Beginnfehler> {
        let mut stand = sperren(&self.schreiben);
        let platz = stand
            .eingang
            .beginnen(kennung, id, kopf, self.uhr.jetzt())?;
        self.eingang_spiegeln(&stand.eingang);
        Ok(platz)
    }

    /// Alle Bytes liegen fertig im Eingang: `laeuft` → `fertig`; verloren, wenn der Eintrag nicht mehr diesem
    /// `PUT` gehört (der räumt dann seine Datei selbst).
    pub fn hochladen_abschliessen(
        &self,
        id: &Anhangid,
        generation: u64,
        art: Art,
    ) -> Result<Zeit, Verloren> {
        let mut stand = sperren(&self.schreiben);
        let fertig = stand
            .eingang
            .abschliessen(id, generation, art, self.uhr.jetzt());
        self.eingang_spiegeln(&stand.eingang);
        fertig
    }

    /// Ein `PUT` bricht ab: `laeuft` → `angemeldet` mit Anlauffrist `min(jetzt + 60 s, lebensende)`.
    pub fn hochladen_abbrechen(&self, id: &Anhangid, generation: u64) -> bool {
        let mut stand = sperren(&self.schreiben);
        let abgebrochen = stand.eingang.abbrechen(id, generation, self.uhr.jetzt());
        self.eingang_spiegeln(&stand.eingang);
        abgebrochen
    }

    /// Nicht tilgbar: Eintrag weg, Anspruch frei, Teildatei weg (ADR-0047 Ergänzung F4 Regel 5).
    pub fn hochladen_verwerfen(&self, id: &Anhangid, generation: u64) -> bool {
        let verworfen = {
            let mut stand = sperren(&self.schreiben);
            let verworfen = stand.eingang.hochladung_verwerfen(id, generation);
            self.eingang_spiegeln(&stand.eingang);
            verworfen
        };
        let _ = self.anhangablage.teil_loeschen(id);
        verworfen
    }

    /// `DELETE` der eigenen ID (Regel 3): Eintrag weg, Reservierung sofort frei, ein laufender `PUT` endet als
    /// Abbruch; danach, ohne Sperre, Teil- und fertige Datei weg. Fremde, unbekannte, gebundene IDs: nichts.
    pub fn anhang_verwerfen(&self, kennung: &Kennung, id: &Anhangid) -> Option<Verworfen> {
        let verworfen = {
            let mut stand = sperren(&self.schreiben);
            let verworfen = stand.eingang.verwerfen(kennung, id);
            self.eingang_spiegeln(&stand.eingang);
            verworfen
        }?;
        let _ = self.anhangablage.eingang_loeschen(id);
        Some(verworfen)
    }

    /// Ein gebundener Anhang aus dem Verzeichnis — die ID ist nur Schlüssel, nie ein Dateiname (Ausliefern, B0446).
    pub fn anhang_finden(
        &self,
        id: &Anhangid,
    ) -> Option<std::sync::Arc<super::anhangverzeichnis::Eintrag>> {
        self.anhangverzeichnis.finden(id)
    }

    /// Die Ablage für Eingang und Anhänge.
    pub fn anhangablage(&self) -> &std::sync::Arc<dyn Anhangablage> {
        &self.anhangablage
    }

    /// Freier Platz der Platte (0, wenn die Auskunft scheitert).
    pub fn platte_frei(&self) -> u64 {
        self.plattenauskunft.frei().unwrap_or(0)
    }

    /// Der Stand der Anhang-Quote, ohne Sperre.
    pub fn anhangkennzahlen(&self) -> Anhangkennzahlen {
        Anhangkennzahlen {
            anhangquote: self.anhangquote,
            gebunden: self.anhangzaehler.gebunden.load(Ordering::SeqCst),
            reserviert: self.anhangzaehler.reserviert.load(Ordering::SeqCst),
            hochladungen: self.anhangzaehler.hochladungen.load(Ordering::SeqCst),
            abgewiesen: self.anhangzaehler.abgewiesen.load(Ordering::SeqCst),
            geloescht: self.anhangzaehler.geloescht.load(Ordering::SeqCst),
        }
    }

    /// Setzt die Anhang-Quote in Bytes; `None` heißt aus.
    pub fn mit_anhangquote(self, anhangquote: Option<u64>) -> Gespraeche {
        Gespraeche {
            anhangquote,
            ..self
        }
    }

    /// Setzt die Plattenauskunft (im Start: `statvfs` des Ordners).
    pub fn mit_plattenauskunft(
        self,
        plattenauskunft: std::sync::Arc<dyn super::platte::Plattenauskunft>,
    ) -> Gespraeche {
        Gespraeche {
            plattenauskunft,
            ..self
        }
    }

    /// Setzt die Ablage für Eingang und Anhänge (im Start: die Deskriptoren des Ordners) und lädt die Anhänge aller
    /// Tage im Index und aller Tage in `anhaenge/` (B0442, ADR-0046 Regel 8 „Laden“; Befund #53 B1): je Tag die
    /// Metazeilen und die regulären Dateien in `anhaenge/<tag>/` — `G` zählt jede reguläre Datei im Zählmaß, auch
    /// Waisen und Tage ohne Tagesdatei; Symlinks und Unterordner nie. Eine ID, die in mehr als einer Metazeile über
    /// alle Tage und Gespräche steht, ergibt keinen Eintrag (Befund #53 B2). Ist `anhaenge` oder der Tag nicht zu
    /// öffnen (Symlink, kein Ordner), bleiben die Anhänge des Tags ohne Eintrag.
    pub fn mit_anhangablage(
        mut self,
        anhangablage: std::sync::Arc<dyn Anhangablage>,
    ) -> Gespraeche {
        self.anhangablage = anhangablage;
        let beginn = std::time::Instant::now();
        let mut befunde = Vec::new();
        let mut waisentage = Vec::new();
        let mut gelesen = Vec::new();
        {
            let verzeichnis = super::lesend(&self.verzeichnis);
            for (gespraech, verlauf) in &verzeichnis.verlaeufe {
                let verlauf = sperren(verlauf);
                let roh: std::collections::BTreeMap<_, _> = verlauf.tage_roh().collect();
                let tage: Vec<_> = roh.keys().copied().collect();
                for tag in self.mit_anhangtagen(gespraech, &tage) {
                    let rohtext = roh.get(&tag).copied().unwrap_or_default();
                    match self.anhangablage.tag_dateien(gespraech, tag) {
                        Ok(dateien) => {
                            if !roh.contains_key(&tag) {
                                waisentage.push(super::Waisentag {
                                    gespraech: gespraech.clone(),
                                    tag,
                                    dateien: dateien.len(),
                                    bytes: dateien.iter().map(|datei| datei.laenge).sum(),
                                });
                            }
                            gelesen.push((gespraech.clone(), tag, tag_lesen(rohtext), dateien));
                        }
                        Err(fehler) => befunde.push(super::Anhangladebefund {
                            gespraech: gespraech.clone(),
                            tag,
                            zeile: None,
                            grund: format!("Anhänge des Tages nicht lesbar ({fehler})"),
                            datei: None,
                        }),
                    }
                }
            }
        }
        let doppelt = doppelte_ids(gelesen.iter().map(|(_, _, zeilen, _)| zeilen));
        let mut gebunden = 0u64;
        for (gespraech, tag, zeilen, dateien) in gelesen {
            let (tagesbefunde, tag_g) = self
                .anhangverzeichnis
                .tag_eintragen(&gespraech, tag, zeilen, &dateien, &doppelt);
            gebunden = gebunden.saturating_add(tag_g);
            befunde.extend(
                tagesbefunde
                    .into_iter()
                    .map(|befund| super::Anhangladebefund {
                        gespraech: gespraech.clone(),
                        tag,
                        zeile: Some(befund.zeile),
                        grund: befund.grund.to_string(),
                        datei: befund.datei,
                    }),
            );
        }
        self.anhangzaehler
            .gebunden
            .store(gebunden, Ordering::SeqCst);
        self.anhangbefunde = befunde;
        self.waisentage = waisentage;
        self.ladezeit += beginn.elapsed();
        self
    }

    /// Die Tage `tage` (mit Tagesdatei) und die Tage, die es nur in `anhaenge/` gibt (Befund #53 B1), aufsteigend.
    /// Lässt sich `anhaenge/` nicht auflisten, nur `tage` — der Fehler zeigt sich am Tag selbst (Laden, Entfallen).
    pub(super) fn mit_anhangtagen(
        &self,
        gespraech: &super::logablage::Gespraech,
        tage: &[chrono::NaiveDate],
    ) -> Vec<chrono::NaiveDate> {
        let mut alle = tage.to_vec();
        alle.extend(self.anhangablage.anhang_tage(gespraech).unwrap_or_default());
        alle.sort();
        alle.dedup();
        alle
    }

    /// Die Anhang-Tagordner ohne Tagesdatei beim Laden.
    pub fn waisentage(&self) -> &[super::Waisentag] {
        &self.waisentage
    }

    /// Was beim Laden der Anhänge übergangen wurde.
    pub fn anhangbefunde(&self) -> &[super::Anhangladebefund] {
        &self.anhangbefunde
    }

    /// Sekundentakt (ADR-0046 Regel 10, Ergänzung „Karte #9 Gate“ Regel 1): unter der Schreibsperre verfallen
    /// angemeldete Einträge nach der Anlauffrist, laufende am Lebensende (der `PUT` räumt seine Teildatei selbst),
    /// fertige nach 15 min; danach, ohne Sperre, die Dateien fertiger Einträge löschen. Liefert die Verfallenen für
    /// das Betriebsprotokoll.
    pub fn anhaenge_ablaufen(&self) -> Vec<Verfallen> {
        let verfallen = {
            let mut stand = sperren(&self.schreiben);
            let verfallen = stand.eingang.ablaufen(self.uhr.jetzt());
            self.eingang_spiegeln(&stand.eingang);
            verfallen
        };
        for eintrag in verfallen.iter().filter(|eintrag| eintrag.fertig) {
            let _ = self.anhangablage.eingang_loeschen(&eintrag.id);
        }
        verfallen
    }

    /// Ersetzt den Zufall der Anhang-IDs (Tests: feste Folge).
    #[cfg(test)]
    pub(crate) fn mit_zufall(
        self,
        zufall: std::sync::Arc<dyn crate::zufall::Zufall>,
    ) -> Gespraeche {
        Gespraeche { zufall, ..self }
    }

    /// `A` und die Zahl offener Hochladungen nach einem Übergang des Eingangs in die Atomics (unter der Sperre).
    pub(super) fn eingang_spiegeln(&self, eingang: &super::anhangeingang::Eingang) {
        self.anhangzaehler
            .reserviert
            .store(eingang.reserviert(), Ordering::SeqCst);
        self.anhangzaehler
            .hochladungen
            .store(eingang.anzahl() as u64, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use super::*;
    use crate::anwesenheit::Art;
    use crate::gespraeche::anhangquote::Quotestand;
    use crate::gespraeche::platte::Gestellt;
    use crate::uhr::GestellteUhr;
    use crate::zufall::FesteFolge;
    use chrono::NaiveDate;

    const MIB: u64 = 1_048_576;

    fn kennung(text: &str) -> Kennung {
        Kennung::bilden(text).expect("Kennung")
    }

    fn mit_quote(q: Option<u64>, frei: u64) -> Gespraeche {
        let ablage = Arc::new(Speicherablage::neu());
        let uhr = Arc::new(GestellteUhr::neu(
            chrono::DateTime::parse_from_rfc3339("2026-10-29T12:00:00.000+01:00").expect("Zeit"),
        ));
        Gespraeche::laden(ablage, uhr)
            .expect("geladen")
            .mit_anhangquote(q)
            .mit_plattenauskunft(Arc::new(Gestellt(AtomicU64::new(frei))))
            .mit_zufall(Arc::new(FesteFolge::neu(1..=100)))
    }

    fn anmelden(
        gespraeche: &Gespraeche,
        wer: &str,
        bytes: u64,
    ) -> Result<Anmeldung, Anmeldefehler> {
        let wer = kennung(wer);
        let herkunft = wer.herkunft().expect("Herkunft");
        gespraeche.anhang_anmelden(&wer, herkunft, "bericht.pdf", bytes)
    }

    /// Der Anspruch deckt die ganze Länge im Zählmaß; Q ganz beanspruchbar „für alle zusammen“ (F1), dann
    /// `hochladungen` mit den Zahlen; nichts wird beansprucht, wenn abgewiesen; Invariante `G + A ≤ Q`.
    #[test]
    fn die_anmeldung_beansprucht_die_ganze_laenge_und_haelt_g_plus_a_unter_q() {
        let gespraeche = mit_quote(Some(10 * MIB), u64::MAX);
        let erste = anmelden(&gespraeche, "beta@127.0.0.1", 307_200).expect("201");
        assert_eq!(gespraeche.anhangkennzahlen().reserviert, 307_200);
        assert_eq!(gespraeche.anhangkennzahlen().hochladungen, 1);
        assert_eq!(erste.id.als_text().len(), 26);

        let zweite = anmelden(&gespraeche, "gamma@127.0.0.1", 10 * MIB - 307_200).expect("bis Q");
        assert_ne!(zweite.id, erste.id);
        let kennzahlen = gespraeche.anhangkennzahlen();
        assert_eq!(kennzahlen.reserviert, 10 * MIB);
        assert!(kennzahlen.gebunden + kennzahlen.reserviert <= 10 * MIB);

        assert_eq!(
            anmelden(&gespraeche, "alpha@127.0.0.1", 1),
            Err(Anmeldefehler::Platz(Platzfehler::Hochladungen(
                Quotestand {
                    belegt: 10 * MIB,
                    gebunden: 0,
                    reserviert: 10 * MIB,
                    hoechstens: 10 * MIB,
                    frei: 0,
                    braucht: 65_536,
                }
            )))
        );
        let danach = gespraeche.anhangkennzahlen();
        assert_eq!(
            (danach.reserviert, danach.hochladungen, danach.abgewiesen),
            (10 * MIB, 2, 1)
        );
    }

    /// Reihenfolge: Name → `zu_gross` → offen → Platte → Quote; abgewiesen wird nie umgeformt.
    #[test]
    fn name_zu_gross_offen_platte_in_dieser_reihenfolge() {
        let gespraeche = mit_quote(Some(10 * MIB), u64::MAX);
        let wer = kennung("beta@127.0.0.1");
        let herkunft = wer.herkunft().expect("Herkunft");
        assert_eq!(
            gespraeche.anhang_anmelden(&wer, herkunft, "../x", 20 * MIB),
            Err(Anmeldefehler::Name(Namensfehler::Zeichen {
                stelle: 2,
                zeichen: "U+002F".into()
            }))
        );
        assert_eq!(
            anmelden(&gespraeche, "beta@127.0.0.1", 10 * MIB + 1),
            Err(Anmeldefehler::Platz(Platzfehler::ZuGross {
                hoechstens: 10 * MIB
            }))
        );
        for _ in 0..10 {
            anmelden(&gespraeche, "beta@127.0.0.1", 1).expect("201");
        }
        assert_eq!(
            anmelden(&gespraeche, "beta@127.0.0.1", 1),
            Err(Anmeldefehler::Offen(Offen::Kennung))
        );
        assert_eq!(
            gespraeche.anhangkennzahlen().abgewiesen,
            0,
            "nur 507 quote zählt"
        );

        let knapp = mit_quote(None, anhangquote::PLATTE_ABSTAND + 65_536 - 1);
        assert_eq!(
            anmelden(&knapp, "beta@127.0.0.1", 1),
            Err(Anmeldefehler::Platz(Platzfehler::Platte {
                frei: anhangquote::PLATTE_ABSTAND + 65_535
            }))
        );
        assert_eq!(knapp.anhangkennzahlen().reserviert, 0);
    }

    /// Die Platte rechnet die noch nicht geschriebenen Ansprüche mit; ohne Anhang-Quote beansprucht jede Länge.
    #[test]
    fn die_platte_rechnet_offene_ansprueche_mit() {
        let frei = anhangquote::PLATTE_ABSTAND + 2 * 65_536;
        let gespraeche = mit_quote(None, frei);
        anmelden(&gespraeche, "beta@127.0.0.1", 1).expect("erste passt");
        anmelden(&gespraeche, "beta@127.0.0.1", 1).expect("zweite passt");
        assert_eq!(
            anmelden(&gespraeche, "beta@127.0.0.1", 1),
            Err(Anmeldefehler::Platz(Platzfehler::Platte { frei }))
        );
        assert_eq!(gespraeche.anhangkennzahlen().anhangquote, None);
    }

    /// Sekundentakt: eine Anmeldung ohne `PUT` verfällt nach 60 s, die Reservierung ist im Zähler sofort frei.
    #[test]
    fn der_takt_laesst_verfallene_anmeldungen_frei() {
        let ablage = Arc::new(Speicherablage::neu());
        let uhr = Arc::new(GestellteUhr::neu(
            chrono::DateTime::parse_from_rfc3339("2026-10-29T12:00:00.000+01:00").expect("Zeit"),
        ));
        let gespraeche = Gespraeche::laden(ablage, uhr.clone())
            .expect("geladen")
            .mit_zufall(Arc::new(FesteFolge::neu(1..=10)));
        anmelden(&gespraeche, "beta@127.0.0.1", 1).expect("201");
        assert_eq!(gespraeche.anhaenge_ablaufen(), Vec::new());
        assert_eq!(gespraeche.anhangkennzahlen().reserviert, 65_536);

        uhr.stellen(
            chrono::DateTime::parse_from_rfc3339("2026-10-29T12:01:00.001+01:00").expect("Zeit"),
        );
        let verfallen = gespraeche.anhaenge_ablaufen();

        assert_eq!(verfallen.len(), 1);
        assert!(verfallen[0].ohne_hochladen);
        let kennzahlen = gespraeche.anhangkennzahlen();
        assert_eq!((kennzahlen.reserviert, kennzahlen.hochladungen), (0, 0));
    }

    /// Gespräche mit Speicherablage, Anhangspeicher, fester Uhr (12:00:00.000) und Channels `allgemein`, `technik`.
    fn mit_ablage() -> (
        Arc<Gespraeche>,
        Arc<Speicherablage>,
        Arc<crate::gespraeche::anhangspeicher::Anhangspeicher>,
    ) {
        let ablage = Arc::new(Speicherablage::neu());
        ablage.gespraech_anlegen(&Gespraech::channel("allgemein"));
        ablage.gespraech_anlegen(&Gespraech::channel("technik"));
        let anhaenge = Arc::new(crate::gespraeche::anhangspeicher::Anhangspeicher::neu());
        let uhr = Arc::new(GestellteUhr::neu(
            chrono::DateTime::parse_from_rfc3339("2026-10-29T12:00:00.000+01:00").expect("Zeit"),
        ));
        let gespraeche = Gespraeche::laden(ablage.clone(), uhr)
            .expect("geladen")
            .mit_anhangquote(Some(10 * MIB))
            .mit_anhangablage(anhaenge.clone())
            .mit_zufall(Arc::new(FesteFolge::neu(1..=100)));
        (Arc::new(gespraeche), ablage, anhaenge)
    }

    /// Anmelden, hochladen (über den Eingang der Attrappe) und abschließen: eine fertige ID.
    fn fertig(gespraeche: &Gespraeche, wer: &str, name: &str, bytes: &[u8]) -> Anhangid {
        let wer = kennung(wer);
        let anmeldung = gespraeche
            .anhang_anmelden(
                &wer,
                wer.herkunft().expect("Herkunft"),
                name,
                bytes.len() as u64,
            )
            .expect("angemeldet");
        let kopf = super::super::anhangeingang::Kopf {
            typ_passt: true,
            laenge: Some(bytes.len() as u64),
        };
        let platz = gespraeche
            .hochladen_beginnen(&wer, &anmeldung.id, kopf)
            .expect("läuft");
        let mut teil = gespraeche
            .anhangablage()
            .teil_anlegen(&anmeldung.id)
            .expect("Teil");
        teil.anhaengen(bytes).expect("geschrieben");
        gespraeche
            .anhangablage()
            .teil_abschliessen(&anmeldung.id)
            .expect("fertig");
        gespraeche
            .hochladen_abschliessen(
                &anmeldung.id,
                platz.generation,
                crate::gespraeche::anhangpruefung::Art::datei(),
            )
            .expect("abgeschlossen");
        anmeldung.id
    }

    fn tag_29() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 29).expect("Tag")
    }

    /// Invarianten (ADR-0046 Ergänzung „F1′“ Regel 1): `G = Σ z(vorhandene) + W` über alle Tage, `A = Σ z` im Eingang.
    fn invarianten(gespraeche: &Gespraeche, tage: &[(&str, NaiveDate)]) {
        let g: u64 = tage
            .iter()
            .map(|(channel, tag)| {
                gespraeche
                    .anhangverzeichnis
                    .bytes_je_tag(&Gespraech::channel(channel), *tag)
            })
            .sum();
        let kennzahlen = gespraeche.anhangkennzahlen();
        assert_eq!(kennzahlen.gebunden, g, "G");
        assert_eq!(
            kennzahlen.reserviert,
            sperren(&gespraeche.schreiben).eingang.reserviert(),
            "A"
        );
        assert!(kennzahlen.gebunden + kennzahlen.reserviert <= 10 * MIB);
    }

    /// B0440 Hauptweg: zwei Anhänge, Text leer → die Dateien liegen unter `anhaenge/<tag>/<hhmmssmmm>-<n>-<ascii>`, der
    /// Satz trägt beide Metazeilen in Sendefolge, `A → G` in einem Schritt, das Verzeichnis kennt Satzzeit, Absender
    /// und Zustand; der Eingang ist leer.
    #[test]
    fn senden_bindet_die_anhaenge_in_den_tag_der_nachricht() {
        let (gespraeche, ablage, anhaenge) = mit_ablage();
        let eins = fertig(&gespraeche, "beta@127.0.0.1", "Bericht März.pdf", &[1; 10]);
        let zwei = fertig(&gespraeche, "beta@127.0.0.1", "b.bin", &[2; 65_537]);
        assert_eq!(gespraeche.anhangkennzahlen().reserviert, 65_536 + 69_632);

        let gesendet = gespraeche
            .senden_mit_anhaengen(
                &Gespraech::channel("allgemein"),
                &kennung("beta@127.0.0.1"),
                Art::Agent,
                "",
                &[eins.clone(), zwei.clone()],
                None,
            )
            .expect("gesendet");

        let Gesendet::Neu(nachricht) = gesendet else {
            panic!("neu erwartet")
        };
        assert_eq!(nachricht.text, "");
        let dateien: Vec<&str> = nachricht
            .anhaenge
            .iter()
            .map(|anhang| anhang.datei.as_str())
            .collect();
        assert_eq!(
            dateien,
            vec!["120000000-1-Bericht_M_rz.pdf", "120000000-2-b.bin"]
        );
        let gebunden = anhaenge.anhaenge();
        assert_eq!(gebunden.len(), 2);
        assert_eq!(
            gebunden[&(
                Gespraech::channel("allgemein"),
                tag_29(),
                "120000000-1-Bericht_M_rz.pdf".to_string()
            )],
            vec![1; 10]
        );
        assert!(anhaenge.eingang().is_empty());
        let inhalt = ablage
            .inhalt(&Gespraech::channel("allgemein"), tag_29())
            .expect("Tag");
        assert!(inhalt.contains(&format!("+ anhang: {} 10 application/octet-stream - 120000000-1-Bericht_M_rz.pdf Bericht März.pdf\n", eins.als_text())), "{inhalt}");
        let kennzahlen = gespraeche.anhangkennzahlen();
        assert_eq!(
            (
                kennzahlen.gebunden,
                kennzahlen.reserviert,
                kennzahlen.hochladungen
            ),
            (65_536 + 69_632, 0, 0)
        );
        let eintrag = gespraeche.anhangverzeichnis.finden(&eins).expect("Eintrag");
        assert_eq!(eintrag.satzzeit, nachricht.zeit);
        assert_eq!(eintrag.absender.als_text(), "beta@127.0.0.1");
        assert_eq!(
            eintrag.zustand(),
            crate::gespraeche::anhangverzeichnis::Zustand::Vorhanden
        );
        invarianten(&gespraeche, &[("allgemein", tag_29())]);

        let erneut = gespraeche.senden_mit_anhaengen(
            &Gespraech::channel("allgemein"),
            &kennung("beta@127.0.0.1"),
            Art::Agent,
            "x",
            std::slice::from_ref(&eins),
            None,
        );
        assert_eq!(
            erneut,
            Err(Sendefehler::Anhang {
                id: eins,
                grund: super::super::anhangeingang::Nehmgrund::Unbekannt
            })
        );
    }

    /// „Nichts halb“: scheitert das zweite Binden oder das Anhängen des Satzes → beide Dateien wieder im Eingang,
    /// kein Satz, kein Ereignis, `A` unverändert, die IDs danach sendbar (Mutante „Bindung zieht `reserviert` nicht
    /// ab“ fällt im Hauptweg).
    #[test]
    fn nichts_halb_wenn_binden_oder_anhaengen_scheitert() {
        let (gespraeche, ablage, anhaenge) = mit_ablage();
        let ereignisse: Arc<std::sync::Mutex<usize>> = Arc::default();
        let gespraeche = Arc::new(
            Arc::try_unwrap(gespraeche)
                .ok()
                .expect("allein")
                .mit_nachricht_veroeffentlichen({
                    let ereignisse = Arc::clone(&ereignisse);
                    Arc::new(move |_: &Gespraech, _: &Nachricht| *sperren(&ereignisse) += 1)
                }),
        );
        let eins = fertig(&gespraeche, "beta@127.0.0.1", "a.bin", &[1; 3]);
        let zwei = fertig(&gespraeche, "beta@127.0.0.1", "b.bin", &[2; 3]);
        let vorher = gespraeche.anhangkennzahlen();
        let senden = |gespraeche: &Gespraeche| {
            gespraeche.senden_mit_anhaengen(
                &Gespraech::channel("allgemein"),
                &kennung("beta@127.0.0.1"),
                Art::Agent,
                "x",
                &[eins.clone(), zwei.clone()],
                None,
            )
        };

        anhaenge.binden_scheitert(2);
        assert!(
            matches!(senden(&gespraeche), Err(Sendefehler::Binden { ref waisen, .. }) if waisen.is_empty())
        );
        assert_eq!(anhaenge.eingang().len(), 2);
        assert!(anhaenge.anhaenge().is_empty());
        assert_eq!(
            ablage.inhalt(&Gespraech::channel("allgemein"), tag_29()),
            None
        );
        assert_eq!(gespraeche.anhangkennzahlen(), vorher);

        ablage.fehler_nach(5);
        assert!(matches!(
            senden(&gespraeche),
            Err(Sendefehler::Ablage { .. })
        ));
        assert_eq!(anhaenge.eingang().len(), 2);
        assert!(anhaenge.anhaenge().is_empty());
        assert_eq!(gespraeche.anhangkennzahlen(), vorher);
        assert_eq!(*sperren(&ereignisse), 0);
        invarianten(&gespraeche, &[("allgemein", tag_29())]);

        assert!(
            matches!(senden(&gespraeche), Ok(Gesendet::Neu(_))),
            "danach sendbar"
        );
        assert_eq!(*sperren(&ereignisse), 1);
        invarianten(&gespraeche, &[("allgemein", tag_29())]);
    }

    /// Scheitert das Zurücklegen, bleibt die Datei als Waise im Tag: sie zählt an `G`, verlässt den Eingang, der
    /// Fehler nennt sie nur als Präfix; die Invarianten halten.
    #[test]
    fn eine_datei_die_nicht_zurueck_kann_bleibt_als_waise() {
        let (gespraeche, _, anhaenge) = mit_ablage();
        let eins = fertig(&gespraeche, "beta@127.0.0.1", "geheim.bin", &[1; 3]);
        let zwei = fertig(&gespraeche, "beta@127.0.0.1", "b.bin", &[2; 3]);
        anhaenge.binden_scheitert(2);
        anhaenge.zuruecklegen_scheitert();

        let fehler = gespraeche.senden_mit_anhaengen(
            &Gespraech::channel("allgemein"),
            &kennung("beta@127.0.0.1"),
            Art::Agent,
            "x",
            &[eins, zwei],
            None,
        );

        let Err(Sendefehler::Binden { waisen, datei, .. }) = fehler else {
            panic!("{fehler:?}")
        };
        assert_eq!(waisen, vec!["120000000-1".to_string()]);
        assert_eq!(datei, "120000000-2");
        let kennzahlen = gespraeche.anhangkennzahlen();
        assert_eq!(
            (kennzahlen.gebunden, kennzahlen.reserviert),
            (65_536, 65_536)
        );
        invarianten(&gespraeche, &[("allgemein", tag_29())]);
    }

    /// M39: liegt das Ziel `anhaenge/<tag>/<datei>` schon da → Rückbau, das vorhandene Ziel bleibt byte-gleich.
    #[test]
    fn ein_vorhandenes_ziel_beim_binden_wird_nie_ueberschrieben() {
        let (gespraeche, _, anhaenge) = mit_ablage();
        let eins = fertig(&gespraeche, "beta@127.0.0.1", "a.bin", &[1; 3]);
        anhaenge.anhang_setzen(
            &Gespraech::channel("allgemein"),
            tag_29(),
            "120000000-1-a.bin",
            b"vorhanden",
        );

        let fehler = gespraeche.senden_mit_anhaengen(
            &Gespraech::channel("allgemein"),
            &kennung("beta@127.0.0.1"),
            Art::Agent,
            "x",
            std::slice::from_ref(&eins),
            None,
        );

        assert!(
            matches!(fehler, Err(Sendefehler::Binden { .. })),
            "{fehler:?}"
        );
        assert_eq!(
            anhaenge.anhaenge()[&(
                Gespraech::channel("allgemein"),
                tag_29(),
                "120000000-1-a.bin".to_string()
            )],
            b"vorhanden"
        );
        assert!(anhaenge.eingang().contains_key(eins.als_text()));
    }

    /// M23: dieselbe ID in zwei Sendungen in zwei Channels (gleichzeitig) → genau eine bindet, die andere 422
    /// `anhang`/`unbekannt`; fremde ID → `fremd`; nicht fertig → `unvollstaendig`.
    #[test]
    fn dieselbe_id_in_zwei_sendungen_bindet_genau_einmal() {
        let (gespraeche, _, _) = mit_ablage();
        let id = fertig(&gespraeche, "beta@127.0.0.1", "a.bin", &[1; 3]);
        let schranke = Arc::new(std::sync::Barrier::new(2));
        let faden: Vec<_> = ["allgemein", "technik"]
            .into_iter()
            .map(|channel| {
                let (gespraeche, schranke, id) =
                    (Arc::clone(&gespraeche), Arc::clone(&schranke), id.clone());
                std::thread::spawn(move || {
                    schranke.wait();
                    gespraeche.senden_mit_anhaengen(
                        &Gespraech::channel(channel),
                        &kennung("beta@127.0.0.1"),
                        Art::Agent,
                        "x",
                        &[id],
                        None,
                    )
                })
            })
            .collect();
        let ergebnisse: Vec<_> = faden
            .into_iter()
            .map(|faden| faden.join().expect("Faden"))
            .collect();
        assert_eq!(
            ergebnisse
                .iter()
                .filter(|ergebnis| matches!(ergebnis, Ok(Gesendet::Neu(_))))
                .count(),
            1
        );
        assert_eq!(
            ergebnisse
                .iter()
                .filter(|ergebnis| matches!(
                    ergebnis,
                    Err(Sendefehler::Anhang {
                        grund: super::super::anhangeingang::Nehmgrund::Unbekannt,
                        ..
                    })
                ))
                .count(),
            1
        );

        let fremd = fertig(&gespraeche, "gamma@127.0.0.1", "g.bin", &[1; 3]);
        let antwort = gespraeche.senden_mit_anhaengen(
            &Gespraech::channel("allgemein"),
            &kennung("beta@127.0.0.1"),
            Art::Agent,
            "x",
            std::slice::from_ref(&fremd),
            None,
        );
        assert_eq!(
            antwort,
            Err(Sendefehler::Anhang {
                id: fremd,
                grund: super::super::anhangeingang::Nehmgrund::Fremd
            })
        );
        let wer = kennung("beta@127.0.0.1");
        let halb = gespraeche
            .anhang_anmelden(&wer, wer.herkunft().expect("Herkunft"), "h.bin", 5)
            .expect("angemeldet")
            .id;
        let antwort = gespraeche.senden_mit_anhaengen(
            &Gespraech::channel("allgemein"),
            &wer,
            Art::Agent,
            "x",
            std::slice::from_ref(&halb),
            None,
        );
        assert_eq!(
            antwort,
            Err(Sendefehler::Anhang {
                id: halb,
                grund: super::super::anhangeingang::Nehmgrund::Unvollstaendig
            })
        );
    }

    /// M23: ein `DELETE` und der Sekundentakt warten während des Bindens auf die Sperre und finden danach nichts —
    /// die Bindung gelingt.
    #[test]
    fn verwerfen_und_ablaufen_waehrend_des_bindens_warten_auf_die_sperre() {
        let (gespraeche, _, _) = mit_ablage();
        let id = fertig(&gespraeche, "beta@127.0.0.1", "a.bin", &[1; 3]);
        let senden = {
            let (gespraeche, id) = (Arc::clone(&gespraeche), id.clone());
            std::thread::spawn(move || {
                let verlauf = lesend(&gespraeche.verzeichnis)
                    .verlaeufe
                    .get(&Gespraech::channel("allgemein"))
                    .cloned()
                    .expect("Verlauf");
                let _gespraechssperre = sperren(&verlauf);
                std::thread::sleep(std::time::Duration::from_millis(50));
                drop(_gespraechssperre);
                gespraeche.senden_mit_anhaengen(
                    &Gespraech::channel("allgemein"),
                    &kennung("beta@127.0.0.1"),
                    Art::Agent,
                    "x",
                    &[id],
                    None,
                )
            })
        };
        let gesendet = senden.join().expect("Faden");
        assert!(matches!(gesendet, Ok(Gesendet::Neu(_))));
        assert_eq!(
            gespraeche.anhang_verwerfen(&kennung("beta@127.0.0.1"), &id),
            None,
            "gebunden: ohne Wirkung"
        );
        assert!(gespraeche.anhaenge_ablaufen().is_empty());
        assert!(gespraeche.anhangverzeichnis.finden(&id).is_some());
    }

    /// B0442: nach einem Neustart (neu laden über dieselben Ablagen) kennt das Verzeichnis die Anhänge wieder, `G`
    /// zählt sie und eine Waise im Zählmaß; eine unplausible Metazeile ergibt einen Befund mit Tagesdatei und Zeile.
    #[test]
    fn laden_kennt_die_anhaenge_wieder_und_zaehlt_waisen() {
        let (gespraeche, ablage, anhaenge) = mit_ablage();
        let id = fertig(&gespraeche, "beta@127.0.0.1", "a.bin", &[1; 10]);
        gespraeche
            .senden_mit_anhaengen(
                &Gespraech::channel("allgemein"),
                &kennung("beta@127.0.0.1"),
                Art::Agent,
                "x",
                std::slice::from_ref(&id),
                None,
            )
            .expect("gesendet");
        anhaenge.anhang_setzen(
            &Gespraech::channel("allgemein"),
            tag_29(),
            "999999999-1-waise.bin",
            b"w",
        );
        let inhalt = ablage
            .inhalt(&Gespraech::channel("allgemein"), tag_29())
            .expect("Tag");
        ablage.tag_setzen(
            &Gespraech::channel("allgemein"),
            tag_29(),
            &format!("{inhalt}@ 2026-10-29T12:30:00.000+01:00 beta@127.0.0.1\n+ anhang: zu kurz\n  kaputt\n\n"),
        );

        let uhr = Arc::new(GestellteUhr::neu(
            chrono::DateTime::parse_from_rfc3339("2026-10-29T13:00:00.000+01:00").expect("Zeit"),
        ));
        let neu = Gespraeche::laden(ablage, uhr)
            .expect("geladen")
            .mit_anhangablage(anhaenge);

        let eintrag = neu.anhangverzeichnis.finden(&id).expect("wieder da");
        assert_eq!(eintrag.datei, "120000000-1-a.bin");
        assert_eq!(neu.anhangkennzahlen().gebunden, 2 * 65_536);
        let befunde = neu.anhangbefunde();
        assert_eq!(befunde.len(), 1, "{befunde:?}");
        assert_eq!(
            (befunde[0].zeile, befunde[0].grund.as_str()),
            (Some(7), "Metazeile anhang: ID unlesbar")
        );
        let verlauf = neu
            .verlauf(
                "allgemein",
                &abfrage::Abfrage {
                    abschnitt: abfrage::Abschnitt::Neueste,
                    n: abfrage::Blockgroesse::standard(),
                },
                1_000_000,
            )
            .expect("Verlauf");
        assert_eq!(verlauf.nachrichten[0].anhaenge.len(), 1);
    }

    /// B0443: der Tag entfällt (Frist 7, gepflegt am 6. November) → Tagesdatei und `anhaenge/<tag>/` sind weg, `G`
    /// zählt ihre Bytes nicht mehr, das Verzeichnis kennt die ID nicht mehr (Mutante „Entfallen vergisst
    /// Anhangbytes“).
    #[test]
    fn mit_dem_tag_entfallen_die_anhaenge_samt_ihren_bytes() {
        let (gespraeche, _, anhaenge) = mit_ablage();
        let id = fertig(&gespraeche, "beta@127.0.0.1", "a.bin", &[1; 10]);
        gespraeche
            .senden_mit_anhaengen(
                &Gespraech::channel("allgemein"),
                &kennung("beta@127.0.0.1"),
                Art::Agent,
                "x",
                std::slice::from_ref(&id),
                None,
            )
            .expect("gesendet");
        anhaenge.anhang_setzen(
            &Gespraech::channel("allgemein"),
            tag_29(),
            "999999999-1-waise.bin",
            b"w",
        );
        gespraeche
            .anhangverzeichnis
            .waise(&Gespraech::channel("allgemein"), tag_29(), 65_536);
        gespraeche
            .anhangzaehler
            .gebunden
            .fetch_add(65_536, Ordering::SeqCst);
        assert_eq!(gespraeche.anhangkennzahlen().gebunden, 2 * 65_536);

        let berichte = gespraeche.pflegen(
            NaiveDate::from_ymd_opt(2026, 11, 6).expect("Tag"),
            crate::gespraeche::Pflegeanlass::Tageswechsel,
        );

        assert!(
            berichte.iter().all(|bericht| matches!(
                bericht,
                crate::gespraeche::Pflegebericht::Entfallen { .. }
            )),
            "{berichte:?}"
        );
        assert_eq!(gespraeche.anhangkennzahlen().gebunden, 0);
        assert!(anhaenge.anhaenge().is_empty());
        assert!(gespraeche.anhangverzeichnis.finden(&id).is_none());
    }

    /// Befund #53 B1 (ADR-0046 Ergänzung „Review #53 B1“ Regeln 3, 4 und 6): das Entfernen von `anhaenge/<tag>`
    /// scheitert einmal nach der Tagesdatei → Bericht „Anhänge nicht entfernt“, der Eintrag ist `entfallen` und nicht
    /// mehr zu finden, `G` unverändert = `W(tag)`; die nächste Pflege findet den Tag in `anhaenge/` und entfernt ihn,
    /// `G` 0 (Mutanten „gescheiterter Tag nicht verwaist“, „`G` beim Scheitern abgezogen“, „Tag nur aus dem Index“).
    #[test]
    fn ein_gescheitertes_entfernen_der_anhaenge_bleibt_gezaehlt_und_wird_wiederholt() {
        let (gespraeche, _, anhaenge) = mit_ablage();
        let allgemein = Gespraech::channel("allgemein");
        let id = fertig(&gespraeche, "beta@127.0.0.1", "a.bin", &[1; 10]);
        gespraeche
            .senden_mit_anhaengen(
                &allgemein,
                &kennung("beta@127.0.0.1"),
                Art::Agent,
                "x",
                std::slice::from_ref(&id),
                None,
            )
            .expect("gesendet");
        let eintrag = gespraeche.anhangverzeichnis.finden(&id).expect("da");
        anhaenge.tag_entfernen_scheitert_einmal(&allgemein, tag_29());

        let berichte = gespraeche.pflegen(
            NaiveDate::from_ymd_opt(2026, 11, 6).expect("Tag"),
            crate::gespraeche::Pflegeanlass::Tageswechsel,
        );

        assert!(
            berichte.iter().any(|bericht| matches!(
                bericht,
                crate::gespraeche::Pflegebericht::AnhaengeNichtEntfernt { tag, .. } if *tag == tag_29()
            )),
            "{berichte:?}"
        );
        assert_eq!(
            eintrag.zustand(),
            crate::gespraeche::anhangverzeichnis::Zustand::Entfallen
        );
        assert!(gespraeche.anhangverzeichnis.finden(&id).is_none());
        assert_eq!(gespraeche.anhangkennzahlen().gebunden, 65_536);
        assert_eq!(
            gespraeche
                .anhangverzeichnis
                .bytes_je_tag(&allgemein, tag_29()),
            65_536,
            "G = Σ z(vorhanden) + Σ W"
        );
        assert!(!anhaenge.anhaenge().is_empty());

        let berichte = gespraeche.pflegen(
            NaiveDate::from_ymd_opt(2026, 11, 7).expect("Tag"),
            crate::gespraeche::Pflegeanlass::Tageswechsel,
        );

        assert!(berichte.is_empty(), "{berichte:?}");
        assert_eq!(gespraeche.anhangkennzahlen().gebunden, 0);
        assert_eq!(
            gespraeche
                .anhangverzeichnis
                .bytes_je_tag(&allgemein, tag_29()),
            0
        );
        assert!(anhaenge.anhaenge().is_empty());
    }

    /// Neun Anhänge zu 1 MiB in `allgemein` (sieben von `alice`, zwei von `bob`), einer in `technik` von `alice`.
    fn neun_und_einer() -> (
        Arc<Gespraeche>,
        Arc<crate::gespraeche::anhangspeicher::Anhangspeicher>,
        Vec<Anhangid>,
    ) {
        let (gespraeche, _, speicher) = mit_ablage();
        let mut ids = Vec::new();
        for nummer in 0..10 {
            let wer = if (7..9).contains(&nummer) {
                "bob@127.0.0.1"
            } else {
                "alice@127.0.0.1"
            };
            let channel = if nummer == 9 { "technik" } else { "allgemein" };
            let id = fertig(
                &gespraeche,
                wer,
                &format!("t{nummer}.bin"),
                &vec![nummer as u8; MIB as usize],
            );
            gespraeche
                .senden_mit_anhaengen(
                    &Gespraech::channel(channel),
                    &kennung(wer),
                    Art::Agent,
                    "x",
                    std::slice::from_ref(&id),
                    None,
                )
                .expect("gesendet");
            ids.push(id);
        }
        (gespraeche, speicher, ids)
    }

    fn zustand(
        gespraeche: &Gespraeche,
        id: &Anhangid,
    ) -> crate::gespraeche::anhangverzeichnis::Zustand {
        gespraeche
            .anhangverzeichnis
            .finden(id)
            .expect("Eintrag")
            .zustand()
    }

    /// B0507 „Drei Schritte und Wettläufe“ (ADR-0046 Ergänzung „F1′“ „Erste Tests“): Rückfrage ändert nichts;
    /// während des Kürzens ist `t1` schon `geloescht`, aber der Platz noch nicht frei; danach G um 2 MiB kleiner;
    /// erneut → `schon_geloescht`; `ELOOP` → `fehler` und wieder `vorhanden`; Entfallen des Tages zieht nur die
    /// vorhandenen ab (Mutanten „Platz frei vor dem Kürzen“, „Löschen ohne Bestätigung“, „Entfallen zieht Gelöschte
    /// ab“).
    #[test]
    fn loeschen_in_drei_schritten() {
        use crate::gespraeche::anhangverzeichnis::Zustand;
        use crate::gespraeche::loeschen::{Einzelergebnis, Loeschen, Umfang};
        let (gespraeche, speicher, ids) = neun_und_einer();
        let bob = kennung("bob@127.0.0.1");
        assert_eq!(gespraeche.anhangkennzahlen().gebunden, 10 * MIB);

        let rueckfrage =
            gespraeche.anhaenge_loeschen(&bob, &ids[..2], false, |_| panic!("keine Buchung"));
        assert_eq!(
            rueckfrage,
            Loeschen::Bestaetigen(Umfang {
                anhaenge: 2,
                bytes: 2 * MIB,
                gespraeche: 1,
                fremde: 2
            })
        );
        assert_eq!(
            (
                zustand(&gespraeche, &ids[0]),
                gespraeche.anhangkennzahlen().gebunden
            ),
            (Zustand::Vorhanden, 10 * MIB)
        );

        let schranke = Arc::new(std::sync::Barrier::new(2));
        speicher.vor_kuerzen_warten(Arc::clone(&schranke));
        let faden = {
            let (gespraeche, ids, bob) = (Arc::clone(&gespraeche), ids[..2].to_vec(), bob.clone());
            std::thread::spawn(move || {
                gespraeche.anhaenge_loeschen(&bob, &ids, true, |einheiten| einheiten == 1)
            })
        };
        schranke.wait();
        assert_eq!(zustand(&gespraeche, &ids[0]), Zustand::Geloescht);
        assert_eq!(
            gespraeche.anhangkennzahlen().gebunden,
            10 * MIB,
            "Platz erst nach dem Kürzen frei"
        );
        schranke.wait();
        let Loeschen::Geloescht(bericht) = faden.join().expect("Faden") else {
            panic!("gelöscht erwartet");
        };
        assert_eq!(
            bericht
                .ergebnisse
                .iter()
                .map(|(_, e)| *e)
                .collect::<Vec<_>>(),
            vec![Einzelergebnis::Geloescht, Einzelergebnis::Geloescht]
        );
        assert_eq!(
            bericht.umfang,
            Umfang {
                anhaenge: 2,
                bytes: 2 * MIB,
                gespraeche: 1,
                fremde: 2
            }
        );
        assert_eq!(gespraeche.anhangkennzahlen().gebunden, 8 * MIB);
        assert_eq!(bericht.platz.belegt, 8 * MIB);
        assert_eq!(bericht.je_gespraech.len(), 1);
        assert_eq!(gespraeche.anhangkennzahlen().geloescht, 2);

        let erneut = gespraeche.anhaenge_loeschen(&bob, &ids[..1], false, |_| {
            panic!("keine Buchung ohne Wirkung")
        });
        let Loeschen::Geloescht(erneut) = erneut else {
            panic!("sofort 200 erwartet")
        };
        assert_eq!(erneut.ergebnisse[0].1, Einzelergebnis::SchonGeloescht);

        let t3 = gespraeche
            .anhangverzeichnis
            .finden(&ids[2])
            .expect("t3")
            .datei
            .clone();
        speicher.kuerzen_scheitert(&t3);
        let Loeschen::Geloescht(gescheitert) =
            gespraeche.anhaenge_loeschen(&bob, &ids[2..3], true, |_| true)
        else {
            panic!("Bericht erwartet");
        };
        assert_eq!(
            (gescheitert.ergebnisse[0].1, gescheitert.fehler),
            (Einzelergebnis::Fehler, 1)
        );
        assert_eq!(
            (
                zustand(&gespraeche, &ids[2]),
                gespraeche.anhangkennzahlen().gebunden
            ),
            (Zustand::Vorhanden, 8 * MIB)
        );

        let gebremst = gespraeche.anhaenge_loeschen(&bob, &ids[3..4], true, |_| false);
        assert_eq!(
            (gebremst, zustand(&gespraeche, &ids[3])),
            (Loeschen::Gebremst, Zustand::Vorhanden)
        );

        let berichte = gespraeche.pflegen(
            NaiveDate::from_ymd_opt(2026, 11, 6).expect("Tag"),
            crate::gespraeche::Pflegeanlass::Tageswechsel,
        );
        assert!(!berichte.is_empty());
        assert_eq!(
            gespraeche.anhangkennzahlen().gebunden,
            0,
            "allgemein und technik entfallen am selben Tag"
        );
    }

    /// Entfallen gegen Gelöschte: der Tag von `allgemein` entfällt allein → G sinkt nur um die sechs vorhandenen.
    #[test]
    fn entfallen_zieht_geloeschte_nicht_ab() {
        let (gespraeche, _, ids) = neun_und_einer();
        let bob = kennung("bob@127.0.0.1");
        gespraeche.anhaenge_loeschen(&bob, &ids[..3], true, |_| true);
        assert_eq!(gespraeche.anhangkennzahlen().gebunden, 7 * MIB);

        gespraeche
            .anhaenge_entfallen(
                &Gespraech::channel("allgemein"),
                tag_29(),
                &crate::gespraeche::Pflegeanlass::Tageswechsel,
            )
            .expect("entfallen");

        assert_eq!(
            gespraeche.anhangkennzahlen().gebunden,
            MIB,
            "nur technik bleibt"
        );
    }

    /// Zwei gleichzeitige Löschungen derselben ID → genau ein `geloescht`, ein `schon_geloescht`; ein Eintrag im
    /// Zwischenzustand `entfallen` ist `unbekannt` (L6); eine gelöschte ID beim Senden → `unbekannt` (L14, M38).
    #[test]
    fn wettlaeufe_und_zwischenzustaende() {
        use crate::gespraeche::anhangverzeichnis::Zustand;
        use crate::gespraeche::loeschen::{Einzelergebnis, Loeschen};
        let (gespraeche, _, ids) = neun_und_einer();
        let faeden: Vec<_> = (0..2)
            .map(|_| {
                let (gespraeche, id) = (Arc::clone(&gespraeche), ids[0].clone());
                std::thread::spawn(move || {
                    match gespraeche.anhaenge_loeschen(
                        &kennung("bob@127.0.0.1"),
                        &[id],
                        true,
                        |_| true,
                    ) {
                        Loeschen::Geloescht(bericht) => bericht.ergebnisse[0].1,
                        anders => panic!("{anders:?}"),
                    }
                })
            })
            .collect();
        let mut ergebnisse: Vec<Einzelergebnis> = faeden
            .into_iter()
            .map(|faden| faden.join().expect("Faden"))
            .collect();
        ergebnisse.sort_by_key(|e| e.als_text());
        assert_eq!(
            ergebnisse,
            vec![Einzelergebnis::Geloescht, Einzelergebnis::SchonGeloescht]
        );
        assert_eq!(gespraeche.anhangkennzahlen().gebunden, 9 * MIB);

        let eintrag = gespraeche.anhangverzeichnis.finden(&ids[1]).expect("t1");
        assert!(eintrag.tauschen(Zustand::Vorhanden, Zustand::Entfallen));
        let Loeschen::Geloescht(bericht) =
            gespraeche.anhaenge_loeschen(&kennung("bob@127.0.0.1"), &ids[1..2], true, |_| true)
        else {
            panic!("Bericht erwartet");
        };
        assert_eq!(
            bericht.ergebnisse[0].1,
            Einzelergebnis::Unbekannt,
            "entfallen ist kein schon_geloescht"
        );

        let gesendet = gespraeche.senden_mit_anhaengen(
            &Gespraech::channel("allgemein"),
            &kennung("alice@127.0.0.1"),
            Art::Agent,
            "nochmal",
            &ids[..1],
            None,
        );
        assert!(gesendet.is_err(), "eine gelöschte ID ist verbraucht");
    }

    /// B0515: die Schwelle wechselt nur über `gebunden` — Anmeldungen ändern nichts (Mutante „Schwelle über G + A“);
    /// das Fallen durch Entfallen nennt den Anlass.
    #[test]
    fn warnschwelle_nur_ueber_gebunden() {
        let gemeldet = Arc::new(std::sync::Mutex::new(Vec::new()));
        let (gespraeche, _, ids) = neun_und_einer();
        let gespraeche = Arc::try_unwrap(gespraeche)
            .ok()
            .expect("allein")
            .mit_anhangschwelle_melden({
                let gemeldet = Arc::clone(&gemeldet);
                Arc::new(move |meldung: &Anhangschwelle| {
                    gemeldet.lock().expect("Liste").push(meldung.clone())
                })
            });
        assert_eq!(gespraeche.anhangkennzahlen().gebunden, 10 * MIB);
        gespraeche.anhangschwelle_pruefen("start");
        assert!(
            gemeldet.lock().expect("Liste").is_empty(),
            "schon beim Binden über 90 %"
        );
        let bob = kennung("bob@127.0.0.1");
        gespraeche.anhaenge_loeschen(&bob, &ids[..1], true, |_| true);
        assert_eq!(
            gemeldet.lock().expect("Liste").last(),
            Some(&Anhangschwelle::Unter {
                gebunden: 9 * MIB,
                hoechstens: 10 * MIB,
                grund: "geloescht"
            })
        );
        gespraeche
            .anhang_anmelden(&bob, bob.herkunft().expect("Herkunft"), "x.bin", MIB)
            .expect("angemeldet");
        gespraeche.anhangschwelle_pruefen("geloescht");
        assert_eq!(gemeldet.lock().expect("Liste").len(), 1, "A zählt nicht");
    }

    #[test]
    fn warnschwelle_faellt_durch_entfallen_mit_dem_anlass() {
        for (anlass, grund) in [
            (crate::gespraeche::Pflegeanlass::Start, "start"),
            (
                crate::gespraeche::Pflegeanlass::Tageswechsel,
                "tageswechsel",
            ),
            (
                crate::gespraeche::Pflegeanlass::FristGesetzt {
                    von: kennung("bob@127.0.0.1"),
                },
                "frist_gesetzt",
            ),
        ] {
            let gemeldet = Arc::new(std::sync::Mutex::new(Vec::new()));
            let (gespraeche, _, _) = neun_und_einer();
            let gespraeche = Arc::try_unwrap(gespraeche)
                .ok()
                .expect("allein")
                .mit_anhangschwelle_melden({
                    let gemeldet = Arc::clone(&gemeldet);
                    Arc::new(move |meldung: &Anhangschwelle| {
                        gemeldet.lock().expect("Liste").push(meldung.clone())
                    })
                });
            gespraeche.anhangschwelle_pruefen("start");
            gespraeche
                .anhaenge_entfallen(&Gespraech::channel("allgemein"), tag_29(), &anlass)
                .expect("entfallen");
            assert_eq!(
                gemeldet.lock().expect("Liste").last(),
                Some(&Anhangschwelle::Unter {
                    gebunden: MIB,
                    hoechstens: 10 * MIB,
                    grund
                }),
                "{grund}"
            );
        }
    }

    #[test]
    fn rest_einheiten_je_hundert() {
        use crate::gespraeche::loeschen::rest_einheiten;
        assert_eq!(
            (
                rest_einheiten(1),
                rest_einheiten(100),
                rest_einheiten(101),
                rest_einheiten(500)
            ),
            (1, 1, 2, 5)
        );
    }

    #[test]
    fn standard_der_anhang_quote_ist_20_gib() {
        let ablage = Arc::new(Speicherablage::neu());
        let uhr = Arc::new(GestellteUhr::neu(
            chrono::DateTime::parse_from_rfc3339("2026-10-29T12:00:00.000+01:00").expect("Zeit"),
        ));
        let gespraeche = Gespraeche::laden(ablage, uhr).expect("geladen");
        assert_eq!(
            gespraeche.anhangkennzahlen().anhangquote,
            Some(21_474_836_480)
        );
    }
}
