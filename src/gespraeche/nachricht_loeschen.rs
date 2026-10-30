//! Eigene Nachricht löschen (R00040 F0136; ADR-0048 Regeln 5, 9, 10): der Löschsatz ist der Commit-Punkt. Unter
//! Gesprächs- und Schreibsperre: nachschlagen (`Nachricht`, `Fremd`) → schon gelöscht? → Quote mit Reserve → Zeit
//! vergeben → Löschsatz in die Tagesdatei der Änderung anhängen (scheitert es, nichts geändert) → Index und Fassung
//! `Geloescht` → `belegt` → veröffentlichen. Der Text bleibt im Log, `belegt` sinkt nicht.

use super::anhangverzeichnis::{Eintrag, Zustand};
use super::{
    Aenderungszustand, Geaendert, Gespraech, Gespraeche, Kennung, Nachricht, lesend, loeschen,
    logformat, nachricht_in_fassung, sperren,
};
use crate::uhr::Zeit;
use std::sync::Arc;
use std::sync::atomic::Ordering;

/// Was `nachricht_loeschen` getan hat — beides mit der Nachricht in ihrer Fassung `geloescht`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NachrichtGeloescht {
    /// Der Löschsatz steht im Log; `anhaenge` ist der Bericht über die mitgelöschten Anhänge (ADR-0048 Regel 9).
    Neu {
        nachricht: Nachricht,
        anhaenge: loeschen::Geloescht,
    },
    /// Schon gelöscht: nichts geschrieben, nichts veröffentlicht.
    Schon(Nachricht),
}

/// Ergebnis des Abschnitts unter den Sperren: die gelöschte Nachricht mit den gewonnenen Anhängen, oder schon gelöscht.
enum UnterSperre {
    Neu(Nachricht, Vec<Arc<Eintrag>>),
    Schon(Nachricht),
}

/// Warum `nachricht_loeschen` nichts geschrieben hat (ADR-0048 Regel 5, Antworten unter der Sperre).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Loeschfehler {
    /// Das Gespräch ist weder geladen noch angelegt.
    Unbekannt,
    /// Keine Nachricht mit dieser Zeit in diesem Gespräch (nie da, entfallen, anderer Channel).
    Nachricht,
    /// Die Kennung ist nicht der Absender.
    Fremd { absender: Kennung },
    /// Der Löschsatz brächte die Gespräche über Quote und Reserve; `hoechstens` ist die Quote ohne Reserve.
    Quote { belegt: u64, hoechstens: u64 },
    /// Die Tagesdatei nahm den Satz nicht an; nichts steht im Log oder Index.
    Ablage { datei: String, grund: String },
}

impl Gespraeche {
    /// Löscht die Nachricht `ziel` des Absenders `von` samt ihren Anhängen (ADR-0048 Regel 9): (a) unter den Sperren,
    /// dann (b) und (c) ohne Gesprächssperre. Aufrufen außerhalb des Laufzeit-Threads (Schritt b berührt die Platte).
    pub fn nachricht_loeschen(
        &self,
        gespraech: &Gespraech,
        von: &Kennung,
        ziel: Zeit,
    ) -> Result<NachrichtGeloescht, Loeschfehler> {
        Ok(match self.loeschen_unter_sperre(gespraech, von, ziel)? {
            UnterSperre::Schon(nachricht) => NachrichtGeloescht::Schon(nachricht),
            UnterSperre::Neu(nachricht, gewonnen) => NachrichtGeloescht::Neu {
                nachricht,
                anhaenge: self.anhaenge_der_nachricht_kuerzen(von, gewonnen),
            },
        })
    }

    /// Zahl der Anhänge der Nachricht `ziel` in ihrer Fassung, wie sie `von` löschen würde (ADR-0048 K5): wer nichts
    /// zu kürzen hat, braucht keine Erlaubnis der Hochladebank. Lesend unter der Verlaufssperre, nie über ein
    /// `await` gehalten. Gelöscht, entfallen, unbekannt und fremd (der Anfrager ist nicht der Absender) zählen 0 —
    /// sie enden ohne Plattenarbeit. Die Anhänge einer Nachricht ändern sich nie (Bearbeiten ändert nur den Text),
    /// die Zahl kann daher nur zu viel verlangen, nie zu wenig.
    pub fn anhangzahl(&self, gespraech: &Gespraech, von: &Kennung, ziel: Zeit) -> usize {
        let Some(verlauf) = lesend(&self.verzeichnis).verlaeufe.get(gespraech).cloned() else {
            return 0;
        };
        let verlauf = sperren(&verlauf);
        verlauf
            .nachricht(&ziel)
            .as_ref()
            .and_then(nachricht_in_fassung)
            .filter(|nachricht| &nachricht.von == von)
            .map_or(0, |nachricht| nachricht.anhaenge.len())
    }

    fn loeschen_unter_sperre(
        &self,
        gespraech: &Gespraech,
        von: &Kennung,
        ziel: Zeit,
    ) -> Result<UnterSperre, Loeschfehler> {
        let verlauf = lesend(&self.verzeichnis)
            .verlaeufe
            .get(gespraech)
            .cloned()
            .ok_or(Loeschfehler::Unbekannt)?;
        let mut verlauf = sperren(&verlauf);
        let mut stand = sperren(&self.schreiben);
        let jetzt = self.uhr.jetzt();
        let nachricht = verlauf
            .nachricht(&ziel)
            .as_ref()
            .and_then(nachricht_in_fassung)
            .ok_or(Loeschfehler::Nachricht)?;
        if &nachricht.von != von {
            return Err(Loeschfehler::Fremd {
                absender: nachricht.von,
            });
        }
        let schon_geloescht = nachricht
            .geaendert
            .is_some_and(|geaendert| geaendert.zustand == Aenderungszustand::Geloescht);
        if schon_geloescht {
            return Ok(UnterSperre::Schon(nachricht));
        }
        let zeit = crate::uhr::naechste_zeit(jetzt, stand.letzte);
        let satz = logformat::loeschsatz(zeit, von, nachricht.zeit);
        let satzbytes = satz.len() as u64;
        if let Some(hoechstens) = self.hoechstens
            && let Err(ueber) = self.belegung.pruefen_mit_reserve(satzbytes, hoechstens)
        {
            return Err(Loeschfehler::Quote {
                belegt: ueber.belegt,
                hoechstens: ueber.hoechstens,
            });
        }
        stand.letzte = Some(zeit);
        self.letzte_ms
            .store(zeit.timestamp_millis(), Ordering::Relaxed);
        let tag = zeit.date_naive();
        let angehaengt = self
            .ablage
            .anhaengen(gespraech, tag, &satz)
            .map_err(|fehler| Loeschfehler::Ablage {
                datei: gespraech.tagesdatei(tag),
                grund: fehler.to_string(),
            })?;
        verlauf.loeschung_anhaengen(
            tag,
            &satz,
            angehaengt.zeilenende_ergaenzt,
            nachricht.zeit,
            zeit,
        );
        let dazu = satzbytes + u64::from(angehaengt.zeilenende_ergaenzt);
        let vorher = self.belegung.dazu(dazu);
        self.ueber_90_melden(vorher, vorher.saturating_add(dazu));
        let gewonnen = self.anhaenge_markieren(gespraech, &nachricht);
        let geloescht = Nachricht {
            text: String::new(),
            anhaenge: Vec::new(),
            geaendert: Some(Geaendert {
                zeit,
                zustand: Aenderungszustand::Geloescht,
            }),
            ..nachricht
        };
        (self.aenderung_veroeffentlichen)(gespraech, &geloescht);
        Ok(UnterSperre::Neu(geloescht, gewonnen))
    }
}

impl Gespraeche {
    /// Je Anhang der Nachricht `vorhanden → geloescht` per Vergleichen-und-Tauschen, unter der Schreibsperre nach dem
    /// Löschsatz (ADR-0048 Regel 9 a): ab hier antwortet der Abruf 410. Ein schon im Laufwerk gelöschter Anhang wird
    /// übersprungen — je Anhang genau ein Abzug —, trägt aber wie jeder die gelöschte Nachricht: scheitert das Kürzen
    /// des Laufwerks danach, bleibt er `geloescht` (Befund #67 B1). Liefert die gewonnenen Einträge für Kürzen und
    /// Abzug.
    fn anhaenge_markieren(
        &self,
        gespraech: &Gespraech,
        nachricht: &Nachricht,
    ) -> Vec<Arc<Eintrag>> {
        nachricht
            .anhaenge
            .iter()
            .filter_map(|zeile| self.anhangverzeichnis.finden(&zeile.id))
            .filter(|eintrag| eintrag.gespraech == *gespraech && eintrag.satzzeit == nachricht.zeit)
            .inspect(|eintrag| eintrag.nachricht_geloescht_setzen())
            .filter(|eintrag| eintrag.tauschen(Zustand::Vorhanden, Zustand::Geloescht))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::super::quote::tests::satz_mit;
    use super::super::{
        Art, Bearbeitet, Bearbeitfehler, Pflegeanlass, Speicherablage, abfrage, verlaufsindex,
    };
    use super::*;
    use crate::uhr::GestellteUhr;
    use chrono::{DateTime, NaiveDate};
    use std::sync::{Arc, Mutex, OnceLock, Weak};

    fn zeit(text: &str) -> Zeit {
        DateTime::parse_from_rfc3339(text).expect("Testzeit")
    }

    fn tag(text: &str) -> NaiveDate {
        NaiveDate::parse_from_str(text, "%Y-%m-%d").expect("Testtag")
    }

    fn kennung(text: &str) -> Kennung {
        Kennung::bilden(text).expect("Kennung")
    }

    fn allgemein() -> Gespraech {
        Gespraech::channel("allgemein")
    }

    fn im_speicher(jetzt: &str) -> (Gespraeche, Arc<Speicherablage>, Arc<GestellteUhr>) {
        let ablage = Arc::new(Speicherablage::neu());
        ablage.gespraech_anlegen(&allgemein());
        let uhr = Arc::new(GestellteUhr::neu(zeit(jetzt)));
        let gespraeche = Gespraeche::laden(ablage.clone(), uhr.clone()).expect("geladen");
        (gespraeche, ablage, uhr)
    }

    fn neueste(gespraeche: &Gespraeche) -> Vec<Nachricht> {
        gespraeche
            .verlauf(
                "allgemein",
                &abfrage::Abfrage {
                    abschnitt: abfrage::Abschnitt::Neueste,
                    n: abfrage::Blockgroesse::bilden(100).expect("Blockgröße"),
                },
                usize::MAX,
            )
            .expect("bekannt")
            .nachrichten
    }

    fn fassung_von(gespraeche: &Gespraeche, zeit: &Zeit) -> Option<verlaufsindex::Fassung> {
        let verlauf = lesend(&gespraeche.verzeichnis)
            .verlaeufe
            .get(&allgemein())
            .cloned()
            .expect("allgemein");
        sperren(&verlauf).fassung(zeit)
    }

    fn senden(gespraeche: &Gespraeche, von: &Kennung, text: &str) -> Zeit {
        gespraeche
            .senden(&allgemein(), von, Art::Agent, text)
            .expect("gesendet")
            .zeit
    }

    fn geloescht_um(zeit_text: &str) -> Option<Geaendert> {
        Some(Geaendert {
            zeit: zeit(zeit_text),
            zustand: Aenderungszustand::Geloescht,
        })
    }

    /// Rückruf `aenderung`: Zeit der Nachricht, Fassung, Schreibsperre noch gehalten.
    type Aufgezeichnet = Arc<Mutex<Vec<(Zeit, Option<Geaendert>, bool)>>>;

    fn mit_aufzeichnung(gespraeche: Gespraeche) -> (Arc<Gespraeche>, Aufgezeichnet) {
        let selbst: Arc<OnceLock<Weak<Gespraeche>>> = Arc::default();
        let aufgezeichnet: Aufgezeichnet = Arc::default();
        let gespraeche = Arc::new(gespraeche.mit_aenderung_veroeffentlichen({
            let (selbst, aufgezeichnet) = (Arc::clone(&selbst), Arc::clone(&aufgezeichnet));
            Arc::new(move |_: &Gespraech, nachricht: &Nachricht| {
                let gespraeche = selbst.get().and_then(Weak::upgrade).expect("Gespräche");
                sperren(&aufgezeichnet).push((
                    nachricht.zeit,
                    nachricht.geaendert,
                    !gespraeche.schreibsperre_frei(),
                ));
            })
        }));
        selbst.set(Arc::downgrade(&gespraeche)).expect("einmal");
        (gespraeche, aufgezeichnet)
    }

    /// R00040 „Gespräche“, wörtlich aus ADR-0048 „Erster Unit-Test (Gespräche)“: `alice` sendet t1 und t2 (Tag 1);
    /// Uhr auf Tag 2; `alice` bearbeitet t1 zweimal, löscht t2; `bob` bearbeitet t1 → `Fremd`, Ablage unverändert;
    /// Block → t1 mit zweitem Text, `bearbeitet`, `geaendert` = Zeit der zweiten Änderung; t2 `geloescht`, Text leer;
    /// der Rückruf erhielt drei `aenderung` unter der Sperre; `belegt` = Σ Bytes; Neuladen → derselbe Block; Bearbeiten
    /// t2 → `Geloescht`; gleicher Text → kein Satz; Entfallen von Tag 1 → Fassungen weg, Tag 2 bleibt; Neuladen ohne
    /// Tag 1 → zwei Waisen-Nachrichten mit drei Änderungssätzen (`anzahl=3`), keine Warnung. Mutanten „Kennung nicht
    /// geprüft“, „Bearbeitet nach Geloescht übernommen“, „Änderungssatz im Block als Nachricht“ rot.
    #[test]
    fn bearbeiten_und_loeschen_ueber_zwei_tage_wie_adr_0048() {
        let (gespraeche, ablage, uhr) = im_speicher("2026-10-27T14:00:00.000+01:00");
        let (gespraeche, aufgezeichnet) = mit_aufzeichnung(gespraeche);
        let (alice, bob) = (kennung("alice@127.0.0.2"), kennung("bob@127.0.0.3"));
        let t1 = senden(&gespraeche, &alice, "eins");
        let t2 = senden(&gespraeche, &alice, "zwei");
        let inhalt = |tag_text| {
            ablage
                .inhalt(&allgemein(), tag(tag_text))
                .unwrap_or_default()
        };

        uhr.stellen(zeit("2026-10-28T09:00:00.000+01:00"));
        let erste = gespraeche.bearbeiten(&allgemein(), &alice, t1, "eins, korrigiert");
        uhr.stellen(zeit("2026-10-28T09:00:05.000+01:00"));
        let zweite = gespraeche.bearbeiten(&allgemein(), &alice, t1, "eins, noch einmal");
        uhr.stellen(zeit("2026-10-28T09:00:10.000+01:00"));
        let geloescht = gespraeche.nachricht_loeschen(&allgemein(), &alice, t2);
        let vorher = (inhalt("2026-10-27"), inhalt("2026-10-28"));
        let fremd = gespraeche.bearbeiten(&allgemein(), &bob, t1, "von bob");
        let nach_dem_loeschen = gespraeche.bearbeiten(&allgemein(), &alice, t2, "wieder da");
        let gleich = gespraeche.bearbeiten(&allgemein(), &alice, t1, "eins, noch einmal");

        assert!(matches!(erste, Ok(Bearbeitet::Neu(_))), "{erste:?}");
        let Ok(Bearbeitet::Neu(bearbeitet)) = zweite else {
            panic!("zweite: {zweite:?}")
        };
        let Ok(NachrichtGeloescht::Neu {
            nachricht: t2_geloescht,
            ..
        }) = geloescht
        else {
            panic!("löschen: {geloescht:?}")
        };
        assert_eq!(
            fremd,
            Err(Bearbeitfehler::Fremd {
                absender: alice.clone()
            })
        );
        assert_eq!(nach_dem_loeschen, Err(Bearbeitfehler::Geloescht));
        assert!(
            matches!(gleich, Ok(Bearbeitet::Unveraendert(_))),
            "{gleich:?}"
        );
        assert_eq!((inhalt("2026-10-27"), inhalt("2026-10-28")), vorher);
        assert!(
            vorher
                .1
                .ends_with("+ geloescht: 2026-10-27T14:00:00.001+01:00\n\n"),
            "{}",
            vorher.1
        );
        let block = neueste(&gespraeche);
        assert_eq!(block.len(), 2, "{block:?}");
        assert_eq!(block[0], bearbeitet);
        assert_eq!(block[0].text, "eins, noch einmal");
        assert_eq!(
            block[0].geaendert,
            Some(Geaendert {
                zeit: zeit("2026-10-28T09:00:05.000+01:00"),
                zustand: Aenderungszustand::Bearbeitet,
            })
        );
        assert_eq!(block[1], t2_geloescht);
        assert_eq!(
            (block[1].zeit, &block[1].von, block[1].art),
            (t2, &alice, Some(Art::Agent))
        );
        assert_eq!(block[1].text, "");
        assert!(block[1].anhaenge.is_empty());
        assert_eq!(
            block[1].geaendert,
            geloescht_um("2026-10-28T09:00:10.000+01:00")
        );
        assert_eq!(
            *sperren(&aufgezeichnet),
            vec![
                (
                    t1,
                    Some(Geaendert {
                        zeit: zeit("2026-10-28T09:00:00.000+01:00"),
                        zustand: Aenderungszustand::Bearbeitet,
                    }),
                    true
                ),
                (t1, block[0].geaendert, true),
                (t2, block[1].geaendert, true),
            ]
        );
        assert_eq!(
            gespraeche.belegt(),
            (vorher.0.len() + vorher.1.len()) as u64
        );
        assert_eq!(gespraeche.kennzahlen().nachrichten, 2);

        let neu_geladen = Gespraeche::laden(ablage.clone(), uhr.clone()).expect("geladen");
        assert_eq!(neueste(&neu_geladen), block);
        assert_eq!(neu_geladen.belegt(), gespraeche.belegt());

        gespraeche.pflegen(tag("2026-11-03"), Pflegeanlass::Tageswechsel);
        assert!(neueste(&gespraeche).is_empty());
        assert_eq!(
            (fassung_von(&gespraeche, &t1), fassung_von(&gespraeche, &t2)),
            (None, None)
        );
        assert!(ablage.inhalt(&allgemein(), tag("2026-10-28")).is_some());
        let ohne_tag_1 = Gespraeche::laden(ablage.clone(), uhr.clone()).expect("geladen");
        assert!(
            ohne_tag_1.defekte().is_empty(),
            "{:?}",
            ohne_tag_1.defekte()
        );
        assert_eq!(ohne_tag_1.aenderungswaisen(), 3);
        assert_eq!(ohne_tag_1.kennzahlen().nachrichten, 0);
    }

    /// B0613 Abweisungen unter der Sperre (M1, M3, M4): Zeit ohne Nachricht und dieselbe Zeit im anderen Channel →
    /// `Nachricht`, unbekanntes Gespräch → `Unbekannt`, eine andere Kennung → `Fremd`, schon gelöscht → `Schon` mit der
    /// gelöschten Fassung — jedes Mal nichts geschrieben und nichts veröffentlicht; dieselbe Zeit in UTC trifft.
    #[test]
    fn loeschen_weist_ab_ohne_zu_schreiben() {
        let ablage = Arc::new(Speicherablage::neu());
        let technik = Gespraech::channel("technik");
        ablage.gespraech_anlegen(&technik);
        let (t1, t2) = (
            "2026-10-28T10:00:00.000+01:00",
            "2026-10-28T10:00:01.000+01:00",
        );
        let tag_28 = format!(
            "@ {t1} beta@127.0.0.1\n  Hallo\n\n@ {t2} beta@127.0.0.1\n  weg\n\n\
             @ 2026-10-28T10:00:02.000+01:00 beta@127.0.0.1\n+ geloescht: {t2}\n\n"
        );
        ablage.tag_setzen(&allgemein(), tag("2026-10-28"), &tag_28);
        let uhr = Arc::new(GestellteUhr::neu(zeit("2026-10-28T12:00:00.000+01:00")));
        let (gespraeche, aufgezeichnet) =
            mit_aufzeichnung(Gespraeche::laden(ablage.clone(), uhr).expect("geladen"));
        let beta = kennung("beta@127.0.0.1");
        let loeschen = |gespraech: &Gespraech, von: &Kennung, ziel: &str| {
            gespraeche.nachricht_loeschen(gespraech, von, zeit(ziel))
        };

        assert_eq!(
            loeschen(&allgemein(), &beta, "2026-10-28T10:00:00.001+01:00"),
            Err(Loeschfehler::Nachricht)
        );
        assert_eq!(loeschen(&technik, &beta, t1), Err(Loeschfehler::Nachricht));
        assert_eq!(
            loeschen(&Gespraech::channel("gibtsnicht"), &beta, t1),
            Err(Loeschfehler::Unbekannt)
        );
        assert_eq!(
            loeschen(&allgemein(), &kennung("gamma@127.0.0.1"), t1),
            Err(Loeschfehler::Fremd {
                absender: beta.clone()
            })
        );
        let schon = loeschen(&allgemein(), &beta, t2);
        let Ok(NachrichtGeloescht::Schon(schon)) = schon else {
            panic!("schon gelöscht: {schon:?}")
        };
        assert_eq!(
            (schon.text.as_str(), schon.geaendert),
            ("", geloescht_um("2026-10-28T10:00:02.000+01:00"))
        );
        assert_eq!(
            ablage.inhalt(&allgemein(), tag("2026-10-28")).as_deref(),
            Some(tag_28.as_str())
        );
        assert_eq!(ablage.inhalt(&technik, tag("2026-10-28")), None);
        assert!(sperren(&aufgezeichnet).is_empty());

        let utc = loeschen(&allgemein(), &beta, "2026-10-28T09:00:00.000Z");

        assert!(
            matches!(utc, Ok(NachrichtGeloescht::Neu { .. })),
            "dieselbe Zeit in UTC: {utc:?}"
        );
        assert_eq!(sperren(&aufgezeichnet).len(), 1);
    }

    /// B0613 Schreibfehler: Anhängen scheitert → `Ablage`, die Ablage byte-gleich, keine Fassung, `belegt`
    /// unverändert, die Nachricht weiter mit Text.
    #[test]
    fn ein_schreibfehler_beim_loeschen_aendert_nichts() {
        let (gespraeche, ablage, _) = im_speicher("2026-10-27T14:00:00.000+01:00");
        let beta = kennung("beta@127.0.0.1");
        let t1 = senden(&gespraeche, &beta, "Hallo");
        let vorher = ablage.inhalt(&allgemein(), tag("2026-10-27"));
        let belegt = gespraeche.belegt();
        ablage.fehler_nach(20);

        let gescheitert = gespraeche.nachricht_loeschen(&allgemein(), &beta, t1);

        assert!(
            matches!(gescheitert, Err(Loeschfehler::Ablage { ref datei, .. }) if datei == "channels/allgemein/2026-10-27.log"),
            "{gescheitert:?}"
        );
        assert_eq!(ablage.inhalt(&allgemein(), tag("2026-10-27")), vorher);
        assert_eq!(gespraeche.belegt(), belegt);
        assert_eq!(fassung_von(&gespraeche, &t1), None);
        assert_eq!(neueste(&gespraeche)[0].text, "Hallo");
    }

    /// R00040 „Reserve an der Plattenquote“ (Unit, Speicherablage): `hoechstens` = 1.048.576, also R = 1.048.576 und
    /// Grenze 2.097.152; L = Länge des Löschsatzes. `belegt` = 2.097.152 − L → Löschen gelingt, `belegt` = 2.097.152;
    /// das nächste Löschen → `Quote` mit dem echten `hoechstens`, Ablage unverändert; Bearbeiten bei `belegt` ≥
    /// 1.048.576 → `Quote`; ohne Quote (`aus`) löscht es.
    #[test]
    fn loeschen_geht_bis_zur_reserve_ueber_die_quote() {
        const JETZT: &str = "2026-10-28T12:00:00.000+01:00";
        let beta = kennung("beta@127.0.0.1");
        let (t1, t2) = (
            "2026-10-28T10:00:00.000+01:00",
            "2026-10-28T10:00:01.000+01:00",
        );
        let nachrichten = format!("@ {t1} beta@127.0.0.1\n  a\n\n@ {t2} beta@127.0.0.1\n  b\n\n");
        let l = logformat::loeschsatz(zeit(JETZT), &beta, zeit(t1)).len();
        let fueller = satz_mit(
            "2026-10-28T09:00:00.000+01:00",
            "x@127.0.0.9",
            2_097_152 - l - nachrichten.len(),
        );
        let ablage = Arc::new(Speicherablage::neu());
        ablage.tag_setzen(
            &allgemein(),
            tag("2026-10-28"),
            &format!("{fueller}{nachrichten}"),
        );
        let laden = || {
            Gespraeche::laden(ablage.clone(), Arc::new(GestellteUhr::neu(zeit(JETZT))))
                .expect("geladen")
        };
        let gespraeche = laden().mit_quote(Some(1_048_576));
        assert_eq!(gespraeche.belegt(), (2_097_152 - l) as u64);

        let erstes = gespraeche.nachricht_loeschen(&allgemein(), &beta, zeit(t1));
        let vorher = ablage.inhalt(&allgemein(), tag("2026-10-28"));
        let zweites = gespraeche.nachricht_loeschen(&allgemein(), &beta, zeit(t2));
        let bearbeiten = gespraeche.bearbeiten(&allgemein(), &beta, zeit(t2), "c");

        assert!(
            matches!(erstes, Ok(NachrichtGeloescht::Neu { .. })),
            "{erstes:?}"
        );
        assert_eq!(gespraeche.belegt(), 2_097_152);
        assert_eq!(
            zweites,
            Err(Loeschfehler::Quote {
                belegt: 2_097_152,
                hoechstens: 1_048_576
            })
        );
        assert!(
            matches!(bearbeiten, Err(Bearbeitfehler::Quote { .. })),
            "{bearbeiten:?}"
        );
        assert_eq!(ablage.inhalt(&allgemein(), tag("2026-10-28")), vorher);
        let ohne_quote = laden().mit_quote(None);
        assert!(matches!(
            ohne_quote.nachricht_loeschen(&allgemein(), &beta, zeit(t2)),
            Ok(NachrichtGeloescht::Neu { .. })
        ));
    }

    /// R00040 „Wettlauf Bearbeiten und Löschen“ (M5, Unit): 100 Nachrichten, je ein Bearbeiten und ein Löschen
    /// gleichzeitig aus zwei Threads → jede endet `geloescht`; jedes Bearbeiten nach dem Löschen ergibt `Geloescht`;
    /// im Log steht nie ein Änderungssatz nach dem Löschsatz derselben Nachricht; Neuladen → dieselben Fassungen.
    #[test]
    fn bearbeiten_und_loeschen_gleichzeitig_enden_geloescht() {
        let (gespraeche, ablage, uhr) = im_speicher("2026-10-27T14:00:00.000+01:00");
        let beta = kennung("beta@127.0.0.1");
        let zeiten: Vec<Zeit> = (0..100)
            .map(|nummer| senden(&gespraeche, &beta, &format!("n{nummer}")))
            .collect();

        let (bearbeitet, geloescht) = std::thread::scope(|scope| {
            let bearbeiten = scope.spawn(|| {
                zeiten
                    .iter()
                    .map(|ziel| gespraeche.bearbeiten(&allgemein(), &beta, *ziel, "neu"))
                    .collect::<Vec<_>>()
            });
            let loeschen = scope.spawn(|| {
                zeiten
                    .iter()
                    .map(|ziel| gespraeche.nachricht_loeschen(&allgemein(), &beta, *ziel))
                    .collect::<Vec<_>>()
            });
            (
                bearbeiten.join().expect("bearbeiten"),
                loeschen.join().expect("löschen"),
            )
        });

        assert!(
            geloescht
                .iter()
                .all(|ergebnis| matches!(ergebnis, Ok(NachrichtGeloescht::Neu { .. }))),
            "{geloescht:?}"
        );
        assert!(
            bearbeitet.iter().all(|ergebnis| matches!(
                ergebnis,
                Ok(Bearbeitet::Neu(_)) | Err(Bearbeitfehler::Geloescht)
            )),
            "{bearbeitet:?}"
        );
        let block = neueste(&gespraeche);
        assert!(
            block.iter().all(|nachricht| nachricht
                .geaendert
                .is_some_and(|geaendert| geaendert.zustand == Aenderungszustand::Geloescht)
                && nachricht.text.is_empty()),
            "{block:?}"
        );
        let log = ablage.inhalt(&allgemein(), tag("2026-10-27")).expect("Tag");
        for ziel in &zeiten {
            let bezug = ziel.format("%Y-%m-%dT%H:%M:%S%.3f%:z").to_string();
            let loeschsatz = log
                .find(&format!("+ geloescht: {bezug}\n"))
                .expect("Löschsatz");
            assert!(
                !log[loeschsatz..].contains(&format!("+ bearbeitet: {bezug}\n")),
                "Bearbeitung nach dem Löschen von {bezug}"
            );
        }
        let neu_geladen = Gespraeche::laden(ablage.clone(), uhr.clone()).expect("geladen");
        assert!(
            neu_geladen.defekte().is_empty(),
            "{:?}",
            neu_geladen.defekte()
        );
        assert_eq!(neueste(&neu_geladen), block);
    }
}
