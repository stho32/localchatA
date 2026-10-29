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
}

impl Gespraeche {
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

    /// Setzt die Ablage für Eingang und Anhänge (im Start: die Deskriptoren des Ordners).
    pub fn mit_anhangablage(self, anhangablage: std::sync::Arc<dyn Anhangablage>) -> Gespraeche {
        Gespraeche {
            anhangablage,
            ..self
        }
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
    fn eingang_spiegeln(&self, eingang: &super::anhangeingang::Eingang) {
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
    use crate::gespraeche::anhangquote::Quotestand;
    use crate::gespraeche::platte::Gestellt;
    use crate::uhr::GestellteUhr;
    use crate::zufall::FesteFolge;

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
        for _ in 0..4 {
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
