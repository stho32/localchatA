//! Direktgespräche anlegen, laden und senden (A00006 Gespräche, ADR-0055 Regeln 1 und 4): ein Paar entsteht beim
//! ersten Senden unter der Sperre des Verzeichnisses, geladen wird nur, was ein Direktpaar benennt.

#[cfg(test)]
use super::schranke::Vorgang;
use super::verlaufsindex::Verlauf;
use super::verzeichnis::{Grenzart, Grenze, Verzeichnis};
use super::{
    Art, Direktpaar, Gesendet, Gespraech, Gespraeche, Kennung, Sendefehler, Sendung, lesend,
    schreibend, sichtbar_fuer, sperren,
};
use crate::uhr::Zeit;
use std::sync::{Arc, Mutex};

/// Höchstens so viele Paarordner insgesamt, auch leere (ADR-0055 Regel 4, E16): 2 Herkünfte, die ihre Grenze voll
/// ausschöpfen.
pub const DIREKT_GESAMT: usize = 8_192;
/// Höchstens so viele Paare, deren Anleger eine Herkunft trägt: 128 Sitzungen je Herkunft (ADR-0021 Ergänzung R00015)
/// × 32 Gegenüber.
pub const DIREKT_JE_HERKUNFT: usize = 4_096;

/// Die Grenzen neuer Paare; im Betrieb die Konstanten, im Test klein.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Direktgrenzen {
    pub gesamt: usize,
    pub je_herkunft: usize,
}

impl Default for Direktgrenzen {
    fn default() -> Direktgrenzen {
        Direktgrenzen {
            gesamt: DIREKT_GESAMT,
            je_herkunft: DIREKT_JE_HERKUNFT,
        }
    }
}

/// Die Zeichen, die ein Ordnername im Protokoll höchstens trägt (ADR-0055 Regel 11, Matrix P2).
const ORDNER_IM_PROTOKOLL_HOECHSTENS: usize = 128;

/// Steuerzeichen, Zeilentrenner (U+2028/2029) und unsichtbare Format- und Richtungszeichen (Soft-Hyphen, Arabic Letter
/// Mark, Zero-Width, Bidi-Einbettungen und -Isolate, Wortverbinder, BOM) — nichts davon darf eine Zeile teilen oder
/// ihre Leserichtung ändern.
fn ersetzen(zeichen: char) -> bool {
    zeichen.is_control()
        || matches!(
            zeichen,
            '\u{00AD}'
                | '\u{061C}'
                | '\u{200B}'..='\u{200F}'
                | '\u{2028}'..='\u{202E}'
                | '\u{2060}'..='\u{2064}'
                | '\u{2066}'..='\u{206F}'
                | '\u{FEFF}'
        )
}

/// `direkt/<name>` für eine Protokollzeile: Steuerzeichen durch `?` ersetzt, auf 128 Zeichen gekürzt — der Name
/// eines Ordners von Hand ist fremder Text und ergibt genau eine Zeile.
pub(super) fn ordner_fuer_protokoll(name: &str) -> String {
    format!("direkt/{name}")
        .chars()
        .map(|zeichen| if ersetzen(zeichen) { '?' } else { zeichen })
        .take(ORDNER_IM_PROTOKOLL_HOECHSTENS)
        .collect()
}

/// Ein Direktgespräch in der Liste der eigenen: das Gegenüber und die Zeit der jüngsten Nachricht.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirektUebersicht {
    pub gegenueber: Kennung,
    pub letzte: Zeit,
}

impl Gespraeche {
    /// Die Direktgespräche von `kennung` mit mindestens einer Nachricht, jüngste zuerst (ADR-0055 Regel 3). Über alle
    /// Gespräche läuft nur diese Schleife, und sie fragt für jedes `sichtbar_fuer` — ein fremdes Gespräch kommt nie in
    /// die Liste.
    pub fn direktgespraeche_von(&self, kennung: &Kennung) -> Vec<DirektUebersicht> {
        let sichtbare: Vec<(Gespraech, Arc<Mutex<Verlauf>>)> = lesend(&self.verzeichnis)
            .verlaeufe
            .iter()
            .filter(|(gespraech, _)| sichtbar_fuer(gespraech, kennung, false))
            .map(|(gespraech, verlauf)| (gespraech.clone(), Arc::clone(verlauf)))
            .collect();
        let mut liste: Vec<DirektUebersicht> = sichtbare
            .into_iter()
            .filter_map(|(gespraech, verlauf)| {
                let Gespraech::Direkt(paar) = gespraech else {
                    return None;
                };
                let letzte = sperren(&verlauf).letzte_nachrichtenzeit()?;
                Some(DirektUebersicht {
                    gegenueber: paar.gegenueber(kennung)?.clone(),
                    letzte,
                })
            })
            .collect();
        liste.sort_by_key(|eintrag| std::cmp::Reverse(eintrag.letzte));
        liste
    }

    /// Sendet im Direktgespräch des Paars; das erste Senden legt es an (ADR-0055 Regel 4). Scheitert dieses erste
    /// Senden (Quote, Bezug, Anhang, Platte) oder wird es nur wiederholt, wird ein dabei entstandener leerer Ordner
    /// wieder entfernt — ein Paar ohne Nachricht belegt nichts. Die Grenzen neuer Paare prüft das Bereitstellen unter der
    /// Sperre; die Prüfung davor (Teilnehmerliste) macht der Aufrufer.
    pub fn direkt_senden(
        &self,
        paar: &Direktpaar,
        von: &Kennung,
        art: Art,
        sendung: Sendung<'_>,
    ) -> Result<Gesendet, Sendefehler> {
        let gespraech = Gespraech::Direkt(paar.clone());
        let mut versuche = 0;
        loop {
            let (neu, halter) = self.direkt_bereitstellen(&gespraech, von)?;
            #[cfg(test)]
            if self.schranken.verschwindet() {
                schreibend(&self.verzeichnis).verlaeufe.remove(&gespraech);
            }
            let ergebnis = self.senden_sendung(&gespraech, von, art, sendung);
            // Was keinen Satz schrieb (Fehler, aber auch `Wiederholt` für ein Paar, das die Pflege inzwischen entfernt
            // hat), lässt keinen leeren Ordner zurück.
            // Der Halter fällt vorher: nur wer sonst noch das Paar hält, hält das Zurückbauen auf.
            drop(halter);
            if neu && !matches!(ergebnis, Ok(Gesendet::Neu(_))) {
                self.direkt_zurueckbauen(&gespraech);
            }
            // Ein anderer Sender hat das eben angelegte Paar wegen seines gescheiterten ersten Sendens
            // zurückgebaut, bevor dieser Aufruf es nutzte: einmal neu anlegen.
            if ergebnis == Err(Sendefehler::Unbekannt) && versuche == 0 {
                versuche += 1;
                continue;
            }
            return ergebnis;
        }
    }

    /// Ob das Paar ein bestehendes Gespräch hat: einen Verlauf mit mindestens einer Nachricht (ADR-0055 Regel 4).
    /// Ein noch nicht entfernter leerer Ordner zählt nicht — er wird beim Senden wiederverwendet, belegt aber nichts
    /// Neues.
    pub fn direkt_besteht(&self, paar: &Direktpaar) -> bool {
        lesend(&self.verzeichnis)
            .verlaeufe
            .get(&Gespraech::Direkt(paar.clone()))
            .is_some_and(|verlauf| sperren(verlauf).nachrichten() > 0)
    }

    /// Ob das Paar im Verzeichnis steht (auch ohne Nachricht).
    pub fn direkt_bekannt(&self, paar: &Direktpaar) -> bool {
        lesend(&self.verzeichnis)
            .verlaeufe
            .contains_key(&Gespraech::Direkt(paar.clone()))
    }

    /// Stellt das Paar bereit: unter der Sperre des Verzeichnisses nachschlagen, fehlt es, Ordner anlegen und den
    /// leeren Verlauf eintragen, der den Anleger schon vermerkt — die Grenze je Herkunft zählt ihn vom Bereitstellen an,
    /// nicht erst mit seiner ersten Nachricht (sonst zählten gleichzeitig beginnende Paare nicht mit). Gibt außerdem
    /// den Verlauf zurück, den der Aufrufer bis zum Ende seines Sendens hält: wer das Paar hält, dessen Ordner nimmt
    /// weder die Pflege noch ein anderer Sender zurück, auch nicht zwischen Bereitstellen und Senden. `true`, wenn dieser Aufruf es angelegt hat. Gleichzeitiges erstes Senden beider
    /// Seiten ergibt einen Ordner, weil das Nachschlagen und das Anlegen unter derselben Sperre liegen.
    fn direkt_bereitstellen(
        &self,
        gespraech: &Gespraech,
        von: &Kennung,
    ) -> Result<(bool, Arc<Mutex<Verlauf>>), Sendefehler> {
        #[cfg(test)]
        self.schranken.passieren(Vorgang::DirektAnlegen);
        let mut verzeichnis = schreibend(&self.verzeichnis);
        if let Some(verlauf) = verzeichnis.verlaeufe.get(gespraech) {
            return Ok((false, Arc::clone(verlauf)));
        }
        self.direkt_grenze_pruefen(&verzeichnis, von)
            .map_err(Sendefehler::DirektGrenze)?;
        let angelegt = match self.ablage.channel_ordner_anlegen(gespraech) {
            Ok(()) => true,
            // Ein echter Ordner, den jemand von Hand angelegt hat, während das Programm lief: vorhanden ist Erfolg.
            Err(fehler) if fehler.kind() == std::io::ErrorKind::AlreadyExists => false,
            Err(fehler) => {
                return Err(Sendefehler::Ablage {
                    datei: gespraech.pfad(),
                    grund: fehler.to_string(),
                });
            }
        };
        let verlauf = Arc::new(Mutex::new(if angelegt {
            Verlauf::angelegt_von(von.clone())
        } else {
            Verlauf::default()
        }));
        verzeichnis
            .verlaeufe
            .insert(gespraech.clone(), Arc::clone(&verlauf));
        Ok((angelegt, verlauf))
    }

    /// Die Grenzen neuer Paare (ADR-0055 Regel 4, E16–E18), unter der Schreibsperre des Verzeichnisses, nur für ein
    /// Paar, dessen Ordner neu entstünde: zuerst die Gesamtgrenze (alle geladenen Paarordner, auch leere), dann die
    /// Grenze je Herkunft (Paare, deren Anleger — Absender der ältesten Nachricht in der Frist, bis dahin der
    /// Sender, der das Paar bereitstellte — diese Herkunft trägt; ein von Hand angelegter Ordner ohne Nachricht hat
    /// keinen Anleger). Ohne Herkunft in der Kennung gilt nur die Gesamtgrenze.
    fn direkt_grenze_pruefen(
        &self,
        verzeichnis: &Verzeichnis,
        von: &Kennung,
    ) -> Result<(), Grenze> {
        let grenzen = self.direkt_grenzen;
        let paare: Vec<&Arc<Mutex<Verlauf>>> = verzeichnis
            .verlaeufe
            .iter()
            .filter(|(gespraech, _)| matches!(gespraech, Gespraech::Direkt(_)))
            .map(|(_, verlauf)| verlauf)
            .collect();
        if paare.len() >= grenzen.gesamt {
            return Err(Grenze {
                art: Grenzart::Gesamt,
                hoechstens: grenzen.gesamt,
            });
        }
        let Some(herkunft) = von.herkunft() else {
            return Ok(());
        };
        let von_dieser_herkunft = paare
            .iter()
            .filter(|verlauf| {
                sperren(verlauf)
                    .aeltester_absender()
                    .is_some_and(|anleger| anleger.herkunft() == Some(herkunft))
            })
            .count();
        if von_dieser_herkunft >= grenzen.je_herkunft {
            return Err(Grenze {
                art: Grenzart::Herkunft,
                hoechstens: grenzen.je_herkunft,
            });
        }
        Ok(())
    }

    /// Nimmt ein Paar zurück, dessen erstes Senden scheiterte: nur wenn es noch keine Nachricht hat und niemand sonst
    /// den Verlauf hält (wie die Pflege: ein zweiter Sender, der das Paar vorfand, behält es), nur ein leerer Ordner
    /// (`rmdir`); scheitert das, bleibt es stehen und die Pflege räumt es auf (ADR-0055 Regel 10).
    fn direkt_zurueckbauen(&self, gespraech: &Gespraech) {
        #[cfg(test)]
        self.schranken.passieren(Vorgang::DirektZurueckbauen);
        let mut verzeichnis = schreibend(&self.verzeichnis);
        let leer = verzeichnis.verlaeufe.get(gespraech).is_some_and(|verlauf| {
            if Arc::strong_count(verlauf) > 1 {
                return false;
            }
            let verlauf = sperren(verlauf);
            verlauf.nachrichten() == 0 && verlauf.bytes() == 0
        });
        if leer && self.ablage.leeren_ordner_entfernen(gespraech).is_ok() {
            verzeichnis.verlaeufe.remove(gespraech);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gespraeche::{Dateiablage, Defektart};
    use crate::uhr::GestellteUhr;
    use std::path::Path;
    use std::sync::Arc;

    fn kennung(text: &str) -> Kennung {
        Kennung::bilden(text).expect(text)
    }

    fn anna() -> Kennung {
        kennung("anna@10.0.0.7")
    }

    fn ben() -> Kennung {
        kennung("ben@10.0.0.8")
    }

    fn paar() -> Direktpaar {
        Direktpaar::bilden(&anna(), &ben()).expect("Paar")
    }

    /// `anna<nummer>@10.0.0.7`: das Gegenüber von `ben` im Paar Nummer `nummer`.
    fn anna_n(nummer: usize) -> Kennung {
        kennung(&format!("anna{nummer}@10.0.0.7"))
    }

    fn paar_von(nummer: usize) -> Direktpaar {
        Direktpaar::bilden(&anna_n(nummer), &ben()).expect("Paar")
    }

    fn sendung(text: &str) -> Sendung<'_> {
        Sendung {
            text,
            anhaenge: &[],
            schluessel: None,
            antwort_auf: None,
        }
    }

    fn laden(ordner: &Path) -> Gespraeche {
        let uhr = GestellteUhr::neu(
            chrono::DateTime::parse_from_rfc3339("2026-10-28T12:00:00.000+01:00").expect("Zeit"),
        );
        Gespraeche::laden(
            Arc::new(Dateiablage::neu(
                ordner,
                Arc::new(crate::protokoll::Protokoll::ohne_ablage()),
            )),
            Arc::new(uhr),
        )
        .expect("geladen")
    }

    fn eintraege(ordner: &Path) -> Vec<String> {
        let mut namen: Vec<String> = std::fs::read_dir(ordner)
            .map(|liste| {
                liste
                    .flatten()
                    .map(|eintrag| eintrag.file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        namen.sort();
        namen
    }

    const TAG: &str = "2026-10-28.log";
    const PAARORDNER: &str = "anna@10.0.0.7+ben@10.0.0.8";

    /// B0767 D4, erster Test: das erste Senden legt `direkt/<paar>/JJJJ-MM-TT.log` an, das zweite schreibt hinein.
    #[test]
    fn das_erste_senden_legt_den_paarordner_an_das_zweite_schreibt_hinein() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let gespraeche = laden(wurzel.path());
        assert!(!gespraeche.direkt_bekannt(&paar()));

        let erste = gespraeche.direkt_senden(&paar(), &anna(), Art::Agent, sendung("eins"));
        let zweite = gespraeche.direkt_senden(&paar(), &ben(), Art::Mensch, sendung("zwei"));

        assert!(matches!(erste, Ok(Gesendet::Neu(_))), "{erste:?}");
        assert!(matches!(zweite, Ok(Gesendet::Neu(_))), "{zweite:?}");
        assert!(gespraeche.direkt_bekannt(&paar()));
        assert_eq!(eintraege(&wurzel.path().join("direkt")), [PAARORDNER]);
        let log = std::fs::read_to_string(wurzel.path().join(format!("direkt/{PAARORDNER}/{TAG}")))
            .expect("Tagesdatei");
        assert!(log.contains("eins") && log.contains("zwei"), "{log}");
        assert_eq!(gespraeche.kennzahlen().direkt_nachrichten, 2);
    }

    /// B0767 W5: beide senden gleichzeitig zuerst — der eine steht an der Schranke vor der Sperre des
    /// Verzeichnisses, der andere legt an und sendet; danach findet der erste das Paar vor. Ein Ordner, zwei
    /// Nachrichten (Mutante „Anlegen außerhalb der Verzeichnissperre“).
    #[test]
    fn beide_senden_gleichzeitig_zuerst_ein_ordner_und_zwei_nachrichten() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let gespraeche = Arc::new(laden(wurzel.path()));
        let schranke = gespraeche
            .vor_den_sperren_anhalten(crate::gespraeche::schranke::Vorgang::DirektAnlegen);

        let anna_sendet = {
            let gespraeche = Arc::clone(&gespraeche);
            std::thread::spawn(move || {
                gespraeche.direkt_senden(&paar(), &anna(), Art::Agent, sendung("von anna"))
            })
        };
        schranke.wait();
        let ben_gesendet =
            gespraeche.direkt_senden(&paar(), &ben(), Art::Agent, sendung("von ben"));
        schranke.wait();
        let anna_gesendet = anna_sendet.join().expect("Thread");

        assert!(
            matches!(ben_gesendet, Ok(Gesendet::Neu(_))),
            "{ben_gesendet:?}"
        );
        assert!(
            matches!(anna_gesendet, Ok(Gesendet::Neu(_))),
            "{anna_gesendet:?}"
        );
        assert_eq!(eintraege(&wurzel.path().join("direkt")), [PAARORDNER]);
        let log = std::fs::read_to_string(wurzel.path().join(format!("direkt/{PAARORDNER}/{TAG}")))
            .expect("Tagesdatei");
        assert!(log.contains("von anna") && log.contains("von ben"), "{log}");
        assert_eq!(
            gespraeche.kennzahlen().direkt_nachrichten,
            2,
            "Index zählt beide"
        );
    }

    /// B0767 W5 (Nacharbeit code-reviewer #13 F0172, Sollte 2): der Haken oben erzwingt eine feste Verschränkung, aber
    /// nicht die Lücke zwischen Prüfen und Einfügen. Hier laufen zwei Threads über `PAARE` Paare; vor jedem Paar
    /// stellt sie eine gemeinsame Schranke auf denselben Start, danach senden beide als Erste. Je Paar zählt der Index
    /// zwei Nachrichten und es gibt genau einen Ordner (Mutante M13 „Nachschlagen und Anlegen unter getrennten
    /// Sperren“: ein Sender ersetzt den eben angelegten Verlauf des anderen und dessen Nachricht fehlt im Index).
    #[test]
    fn zwei_threads_legen_hunderte_paare_gleichzeitig_an_ohne_luecke_zwischen_pruefen_und_einfuegen()
     {
        const PAARE: usize = 400;
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let gespraeche = Arc::new(laden(wurzel.path()));
        let start = Arc::new(std::sync::Barrier::new(2));
        let zweig = |von: fn(usize) -> Kennung| {
            let (gespraeche, start) = (Arc::clone(&gespraeche), Arc::clone(&start));
            std::thread::spawn(move || {
                (0..PAARE)
                    .filter(|&nummer| {
                        start.wait();
                        gespraeche
                            .direkt_senden(
                                &paar_von(nummer),
                                &von(nummer),
                                Art::Agent,
                                sendung("hallo"),
                            )
                            .is_err()
                    })
                    .count()
            })
        };
        let von_anna = zweig(anna_n);
        let von_ben = zweig(|_| ben());

        let fehler = von_anna.join().expect("Thread") + von_ben.join().expect("Thread");

        assert_eq!(fehler, 0, "kein Senden scheitert");
        assert_eq!(
            gespraeche.kennzahlen().direkt_nachrichten,
            2 * PAARE as u64,
            "der Index zählt beide Nachrichten je Paar"
        );
        assert_eq!(eintraege(&wurzel.path().join("direkt")).len(), PAARE);
        for nummer in 0..PAARE {
            let ordner = paar_von(nummer).ordnername();
            let log = std::fs::read_to_string(wurzel.path().join(format!("direkt/{ordner}/{TAG}")))
                .expect("Tagesdatei");
            assert_eq!(log.matches("hallo").count(), 2, "{ordner}");
        }
    }

    #[test]
    fn nach_dem_neustart_ist_das_paar_mit_seinen_nachrichten_geladen() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let erste = laden(wurzel.path());
        erste
            .direkt_senden(&paar(), &anna(), Art::Agent, sendung("bleibt"))
            .expect("gesendet");

        let neu = laden(wurzel.path());

        assert!(neu.direkt_bekannt(&paar()));
        assert_eq!(neu.kennzahlen().direkt_nachrichten, 1);
        assert!(neu.defekte().is_empty(), "{:?}", neu.defekte());
    }

    /// Matrix O2 / P2 (Integration): jeder ungültige Ordner unter `direkt/` ist **ein** Defekt `KeinDirektpaar`, kein
    /// Gespräch, und bestimmt weder die jüngste Zeit noch eine Kennzahl; der richtig kodierte Ordner (Kontrolle) wird
    /// geladen.
    #[test]
    fn ungueltige_direktordner_sind_je_ein_defekt_und_nie_ein_gespraech() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let satz = |zeit: &str| format!("@ {zeit} anna@10.0.0.7\n  text\n\n");
        let lang = "l".repeat(250); // NAME_MAX 255: länger lässt das Dateisystem keinen Ordnernamen zu
        let ungueltig = [
            "x",
            "b@1.1.1.1+a@1.1.1.1",
            "a@1.1.1.1+a@1.1.1.1",
            "a@1.1.1.1+b@1.1.1.1+c@1.1.1.1",
            "anna@fd00%3a%3a7+ben@1.1.1.1",
            "anna@1.1.1.1%2F+ben@1.1.1.1",
            "anna@%3A%3Affff%3A1.1.1.1+ben@1.1.1.1",
            "evil\n2026-10-28T12:00:00.000+01:00 INFORMATION start fake",
            lang.as_str(),
        ];
        for name in ungueltig {
            let ordner = wurzel.path().join("direkt").join(name);
            std::fs::create_dir_all(&ordner).expect("Ordner");
            std::fs::write(ordner.join(TAG), satz("2026-10-28T23:00:00.000+01:00")).expect("Tag");
        }
        std::fs::create_dir_all(wurzel.path().join("direkt/.neu-x")).expect("versteckt");
        let kontrolle = wurzel.path().join("direkt/anna@fd00%3A%3A7+ben@1.1.1.1");
        std::fs::create_dir_all(&kontrolle).expect("Kontrolle");
        std::fs::write(kontrolle.join(TAG), satz("2026-10-28T09:00:00.000+01:00")).expect("Tag");

        let gespraeche = laden(wurzel.path());

        let defekte = gespraeche.defekte();
        assert_eq!(defekte.len(), ungueltig.len(), "{defekte:?}");
        assert!(defekte.iter().all(|d| d.art == Defektart::KeinDirektpaar));
        for defekt in defekte {
            assert!(defekt.datei.starts_with("direkt/"), "{}", defekt.datei);
            assert!(defekt.datei.chars().count() <= 128, "{}", defekt.datei);
            assert!(
                !defekt.datei.chars().any(char::is_control),
                "{:?}",
                defekt.datei
            );
        }
        assert_eq!(
            gespraeche.kennzahlen().direkt_nachrichten,
            1,
            "nur die Kontrolle"
        );
        assert_eq!(gespraeche.kennzahlen().defekte_zeilen, 0);
        assert_eq!(
            gespraeche.letzte_zeit().map(|zeit| zeit.to_rfc3339()),
            Some("2026-10-28T09:00:00+01:00".to_string()),
            "seit kommt nie aus einem ungültigen Ordner"
        );
        assert!(gespraeche.direkt_bekannt(
            &Direktpaar::bilden(&kennung("anna@fd00::7"), &kennung("ben@1.1.1.1")).expect("Paar")
        ));
    }

    /// Das erste Senden, das scheitert, hinterlässt keinen Paarordner (kein Anlegen ohne Nachricht): hier ein Bezug,
    /// den es im neuen Gespräch nicht geben kann.
    #[test]
    fn ein_gescheitertes_erstes_senden_laesst_keinen_ordner_zurueck() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let gespraeche = laden(wurzel.path());
        let bezug =
            chrono::DateTime::parse_from_rfc3339("2026-10-28T11:00:00.000+01:00").expect("Zeit");

        let ergebnis = gespraeche.direkt_senden(
            &paar(),
            &anna(),
            Art::Agent,
            Sendung {
                antwort_auf: Some(bezug),
                ..sendung("Antwort")
            },
        );

        assert_eq!(
            ergebnis,
            Err(Sendefehler::Antwort(
                crate::gespraeche::Antwortfehler::Unbekannt
            ))
        );
        assert!(!gespraeche.direkt_bekannt(&paar()));
        assert!(
            !wurzel.path().join("direkt").join(PAARORDNER).exists(),
            "kein leerer Paarordner"
        );
    }

    /// ADR-0055 Regel 4: ein Symlink als `direkt` oder als Paarordner ist ein Ablagefehler (500), nichts wird
    /// geschrieben, das Ziel bleibt leer.
    #[test]
    fn symlink_als_direkt_oder_als_paarordner_ist_ein_ablagefehler_und_das_ziel_bleibt_leer() {
        use std::os::unix::fs::symlink;
        let ziel = tempfile::tempdir().expect("Ziel");
        let wurzel_a = tempfile::tempdir().expect("Wurzel A");
        let gespraeche_a = laden(wurzel_a.path());
        symlink(ziel.path(), wurzel_a.path().join("direkt")).expect("Symlink direkt");

        let a = gespraeche_a.direkt_senden(&paar(), &anna(), Art::Agent, sendung("x"));

        let wurzel_b = tempfile::tempdir().expect("Wurzel B");
        std::fs::create_dir(wurzel_b.path().join("direkt")).expect("direkt");
        symlink(ziel.path(), wurzel_b.path().join("direkt").join(PAARORDNER))
            .expect("Symlink Paar");
        let gespraeche_b = laden(wurzel_b.path());
        assert!(
            !gespraeche_b.direkt_bekannt(&paar()),
            "Symlink nicht geladen"
        );
        assert!(
            gespraeche_b.defekte().is_empty(),
            "{:?}",
            gespraeche_b.defekte()
        );
        let b = gespraeche_b.direkt_senden(&paar(), &anna(), Art::Agent, sendung("x"));

        assert!(matches!(a, Err(Sendefehler::Ablage { .. })), "{a:?}");
        assert!(matches!(b, Err(Sendefehler::Ablage { .. })), "{b:?}");
        assert_eq!(
            eintraege(ziel.path()),
            Vec::<String>::new(),
            "Ziel unberührt"
        );
        assert!(!gespraeche_a.direkt_bekannt(&paar()));
        assert!(!gespraeche_b.direkt_bekannt(&paar()));
    }

    /// Code-Review #13 F0172 (Hinweis zu F0175): ein Wiederholschlüssel überlebt das Paar (10 Minuten), das die Pflege
    /// samt Ordner entfernt hat; dasselbe Senden legt das Paar neu an und antwortet `Wiederholt`, ohne einen Satz zu
    /// schreiben — den eben angelegten leeren Ordner nimmt `direkt_senden` zurück, er belegte sonst einen Platz der
    /// Grenzen bis zur nächsten Pflege.
    #[test]
    fn eine_wiederholung_nach_entferntem_paar_laesst_keinen_leeren_ordner_stehen() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let gespraeche = laden(wurzel.path());
        let mit_schluessel = || Sendung {
            schluessel: Some("k1"),
            ..sendung("einmal")
        };
        let erste = gespraeche.direkt_senden(&paar(), &anna(), Art::Agent, mit_schluessel());
        assert!(matches!(erste, Ok(Gesendet::Neu(_))), "{erste:?}");
        let berichte = gespraeche.pflegen(
            chrono::NaiveDate::from_ymd_opt(2026, 11, 5).expect("Tag"),
            crate::gespraeche::Pflegeanlass::Tageswechsel,
        );
        assert!(!gespraeche.direkt_bekannt(&paar()), "{berichte:?}");
        assert!(!wurzel.path().join("direkt").join(PAARORDNER).exists());

        let zweite = gespraeche.direkt_senden(&paar(), &anna(), Art::Agent, mit_schluessel());

        assert!(matches!(zweite, Ok(Gesendet::Wiederholt(_))), "{zweite:?}");
        assert!(!gespraeche.direkt_bekannt(&paar()));
        assert!(
            !wurzel.path().join("direkt").join(PAARORDNER).exists(),
            "kein leerer Paarordner"
        );
    }

    /// Eine Ablage, die jeden Ordner „leer“ nennt: die Leer-Prüfung des Rückbaus muss vom Verlauf kommen, nicht nur
    /// vom `rmdir` (Code-Review #13 F0172 Sollte 1, Mutante „Leer-Prüfung weg“).
    struct LeerLuegner(Arc<crate::gespraeche::Speicherablage>);

    impl crate::gespraeche::Logablage for LeerLuegner {
        fn gespraeche(&self) -> std::io::Result<Vec<Gespraech>> {
            self.0.gespraeche()
        }
        fn tage(&self, gespraech: &Gespraech) -> std::io::Result<Vec<chrono::NaiveDate>> {
            self.0.tage(gespraech)
        }
        fn lesen(&self, gespraech: &Gespraech, tag: chrono::NaiveDate) -> std::io::Result<Vec<u8>> {
            self.0.lesen(gespraech, tag)
        }
        fn anhaengen(
            &self,
            gespraech: &Gespraech,
            tag: chrono::NaiveDate,
            satz: &str,
        ) -> std::io::Result<crate::gespraeche::Angehaengt> {
            self.0.anhaengen(gespraech, tag, satz)
        }
        fn channel_ordner_anlegen(&self, gespraech: &Gespraech) -> std::io::Result<()> {
            self.0.channel_ordner_anlegen(gespraech)
        }
        fn einstellungen_schreiben(
            &self,
            gespraech: &Gespraech,
            bytes: &[u8],
        ) -> std::io::Result<()> {
            self.0.einstellungen_schreiben(gespraech, bytes)
        }
        fn einstellungen_lesen(&self, gespraech: &Gespraech) -> std::io::Result<Option<Vec<u8>>> {
            self.0.einstellungen_lesen(gespraech)
        }
        fn ordner_entfernen(&self, gespraech: &Gespraech) -> std::io::Result<()> {
            self.0.ordner_entfernen(gespraech)
        }
        fn leeren_ordner_entfernen(&self, _: &Gespraech) -> std::io::Result<()> {
            Ok(())
        }
        fn tag_entfernen(
            &self,
            gespraech: &Gespraech,
            tag: chrono::NaiveDate,
        ) -> std::io::Result<()> {
            self.0.tag_entfernen(gespraech, tag)
        }
    }

    #[test]
    fn der_rueckbau_nimmt_ein_paar_mit_nachricht_nie_zurueck_auch_wenn_die_ablage_leer_meldet() {
        let uhr = GestellteUhr::neu(
            chrono::DateTime::parse_from_rfc3339("2026-10-28T12:00:00.000+01:00").expect("Zeit"),
        );
        let gespraeche = Gespraeche::laden(
            Arc::new(LeerLuegner(Arc::new(
                crate::gespraeche::Speicherablage::neu(),
            ))),
            Arc::new(uhr),
        )
        .expect("geladen");
        gespraeche
            .direkt_senden(&paar(), &anna(), Art::Agent, sendung("bleibt"))
            .expect("gesendet");

        gespraeche.direkt_zurueckbauen(&Gespraech::Direkt(paar()));

        assert!(gespraeche.direkt_besteht(&paar()), "Nachricht im Verlauf");
    }

    /// Code-Review #13 F0175 Sollte 3: ein zweiter Sender hält das Paar schon (er hat es unter der Sperre des
    /// Verzeichnisses vorgefunden), da scheitert das erste Senden des Anlegers und will zurückbauen. Es darf nicht — wie
    /// die Pflege nimmt es kein Paar, das ein anderer hält (`strong_count`); sonst findet der zweite Sender sein Paar
    /// nicht mehr (Mutante „Halter-Prüfung weg“: `Unbekannt`, der Ordner fehlt).
    #[test]
    fn das_zurueckbauen_nimmt_kein_paar_das_ein_zweiter_sender_schon_haelt() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let gespraeche = Arc::new(laden(wurzel.path()));
        let schranke = gespraeche.vor_den_sperren_anhalten(Vorgang::DirektZurueckbauen);
        let bezug =
            chrono::DateTime::parse_from_rfc3339("2026-10-28T11:00:00.000+01:00").expect("Zeit");
        let anleger = {
            let gespraeche = Arc::clone(&gespraeche);
            std::thread::spawn(move || {
                gespraeche.direkt_senden(
                    &paar(),
                    &anna(),
                    Art::Agent,
                    Sendung {
                        antwort_auf: Some(bezug),
                        ..sendung("scheitert")
                    },
                )
            })
        };
        schranke.wait(); // der Anleger steht vor dem Zurückbauen
        let gespraech = Gespraech::Direkt(paar());
        let halter = gespraeche
            .direkt_bereitstellen(&gespraech, &ben())
            .expect("vorgefunden");
        schranke.wait(); // der Anleger baut jetzt zurück
        let anleger = anleger.join().expect("Thread");

        let zweiter =
            gespraeche.senden_sendung(&gespraech, &ben(), Art::Agent, sendung("kommt an"));

        assert_eq!(
            anleger,
            Err(Sendefehler::Antwort(
                crate::gespraeche::Antwortfehler::Unbekannt
            ))
        );
        assert!(matches!(zweiter, Ok(Gesendet::Neu(_))), "{zweiter:?}");
        assert_eq!(eintraege(&wurzel.path().join("direkt")), [PAARORDNER]);
        drop(halter);
    }

    // ---- B0779: Grenzen neuer Paare (ADR-0055 Regel 4, E16–E18, Baustein D13) -------------------------------------

    fn grenzen(gesamt: usize, je_herkunft: usize) -> Direktgrenzen {
        Direktgrenzen {
            gesamt,
            je_herkunft,
        }
    }

    fn laden_mit(ordner: &Path, grenzen: Direktgrenzen) -> Gespraeche {
        laden(ordner).mit_direkt_grenzen(grenzen)
    }

    /// Das `nummer`. Gegenüber, jedes aus einer eigenen Herkunft `10.1.<nummer>.1` (kein Gegenüber ist je einer der
    /// Anleger der Tests).
    fn gegenueber(nummer: usize) -> Kennung {
        kennung(&format!("g{nummer}@10.1.{nummer}.1"))
    }

    /// `von` beginnt (oder setzt fort) das Gespräch mit dem `nummer`. Gegenüber.
    fn senden_an(
        gespraeche: &Gespraeche,
        von: &Kennung,
        nummer: usize,
    ) -> Result<Gesendet, Sendefehler> {
        let paar = Direktpaar::bilden(von, &gegenueber(nummer)).expect("Paar");
        gespraeche.direkt_senden(&paar, von, Art::Agent, sendung("x"))
    }

    fn herkunftsgrenze(hoechstens: usize) -> Sendefehler {
        Sendefehler::DirektGrenze(crate::gespraeche::Grenze {
            art: crate::gespraeche::Grenzart::Herkunft,
            hoechstens,
        })
    }

    fn gesamtgrenze(hoechstens: usize) -> Sendefehler {
        Sendefehler::DirektGrenze(crate::gespraeche::Grenze {
            art: crate::gespraeche::Grenzart::Gesamt,
            hoechstens,
        })
    }

    fn anna_von(herkunft: &str) -> Kennung {
        kennung(&format!("anna@{herkunft}"))
    }

    /// B0779 D13, erster Test (wörtlich): `anna@10.0.0.1` beginnt 3 Paare → 201, das 4. **neue** → 429 Herkunft 3;
    /// ein Paar von `10.0.0.2` → 201; bestehende Paare senden weiter (Mutanten „Grenze weg“, „Grenze prüft auch
    /// bestehende Paare“).
    #[test]
    fn das_vierte_neue_paar_einer_herkunft_ist_die_grenze_bestehende_und_andere_herkuenfte_gehen() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let gespraeche = laden_mit(wurzel.path(), grenzen(100, 3));
        let anna = anna_von("10.0.0.1");
        for nummer in 1..=3 {
            let erste = senden_an(&gespraeche, &anna, nummer);
            assert!(matches!(erste, Ok(Gesendet::Neu(_))), "{nummer}: {erste:?}");
        }

        let viertes = senden_an(&gespraeche, &anna, 4);
        let andere_herkunft = senden_an(&gespraeche, &anna_von("10.0.0.2"), 4);
        let bestehendes = senden_an(&gespraeche, &anna, 1);

        assert_eq!(viertes, Err(herkunftsgrenze(3)));
        assert!(
            matches!(andere_herkunft, Ok(Gesendet::Neu(_))),
            "{andere_herkunft:?}"
        );
        assert!(
            matches!(bestehendes, Ok(Gesendet::Neu(_))),
            "{bestehendes:?}"
        );
        assert!(
            !gespraeche.direkt_bekannt(&Direktpaar::bilden(&anna, &gegenueber(4)).expect("Paar")),
            "nichts angelegt"
        );
        assert_eq!(
            eintraege(&wurzel.path().join("direkt")).len(),
            4,
            "3 von anna, 1 von der anderen Herkunft — kein Ordner für das abgewiesene"
        );
    }

    /// `anzahl` Threads starten zugleich (gemeinsame Schranke) und senden je `von(nummer)` an das Gegenüber `nummer`;
    /// das Ergebnis je Nummer.
    fn gleichzeitig_senden(
        gespraeche: &Arc<Gespraeche>,
        anzahl: usize,
        von: impl Fn(usize) -> Kennung + Send + Sync + 'static,
    ) -> Vec<Result<Gesendet, Sendefehler>> {
        let start = Arc::new(std::sync::Barrier::new(anzahl));
        let von = Arc::new(von);
        let threads: Vec<_> = (0..anzahl)
            .map(|nummer| {
                let (gespraeche, start, von) =
                    (Arc::clone(gespraeche), Arc::clone(&start), Arc::clone(&von));
                std::thread::spawn(move || {
                    start.wait();
                    senden_an(&gespraeche, &von(nummer), nummer)
                })
            })
            .collect();
        threads
            .into_iter()
            .map(|thread| thread.join().expect("Thread"))
            .collect()
    }

    fn kurz(ergebnisse: &[Result<Gesendet, Sendefehler>]) -> String {
        ergebnisse
            .iter()
            .map(|e| match e {
                Ok(_) => "Ok".to_string(),
                Err(fehler) => format!("{fehler:?}"),
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Code-Review #13 F0175 Sollte 1 (Abnahme-Probe wörtlich): 32 Threads, eine Schranke, alle senden mit
    /// `anna@10.0.0.1` an g0…g31, Herkunftsgrenze 3 → genau 3 × `Neu` und 29 × `DirektGrenze{Herkunft,3}`, danach
    /// genau 3 Paarordner. Der Anleger zählt schon ab dem Bereitstellen, nicht erst mit der ersten Nachricht (Mutante
    /// „Anleger erst mit der Nachricht“: 32 × Ok). Nach dem Neustart bleibt die Zählung gleich.
    #[test]
    fn gleichzeitig_beginnende_paare_einer_herkunft_halten_die_herkunftsgrenze_hart() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let gespraeche = Arc::new(laden_mit(wurzel.path(), grenzen(100, 3)));

        let ergebnisse = gleichzeitig_senden(&gespraeche, 32, |_| anna_von("10.0.0.1"));

        let neu = ergebnisse
            .iter()
            .filter(|e| matches!(e, Ok(Gesendet::Neu(_))))
            .count();
        let abgewiesen = ergebnisse
            .iter()
            .filter(|e| **e == Err(herkunftsgrenze(3)))
            .count();
        assert_eq!((neu, abgewiesen), (3, 29), "{}", kurz(&ergebnisse));
        assert_eq!(eintraege(&wurzel.path().join("direkt")).len(), 3);
        drop(gespraeche);

        let nach_neustart = laden_mit(wurzel.path(), grenzen(100, 3));

        assert_eq!(
            senden_an(&nach_neustart, &anna_von("10.0.0.1"), 40),
            Err(herkunftsgrenze(3)),
            "Anleger aus dem Index"
        );
    }

    /// Gegenprobe zu oben: die Gesamtgrenze 3 hält unter derselben Gleichzeitigkeit (sie zählte schon alle Ordner).
    #[test]
    fn gleichzeitig_beginnende_paare_halten_auch_die_gesamtgrenze_hart() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let gespraeche = Arc::new(laden_mit(wurzel.path(), grenzen(3, 100)));

        let ergebnisse = gleichzeitig_senden(&gespraeche, 32, |nummer| {
            anna_von(&format!("10.0.{nummer}.1"))
        });

        let neu = ergebnisse
            .iter()
            .filter(|e| matches!(e, Ok(Gesendet::Neu(_))))
            .count();
        let abgewiesen = ergebnisse
            .iter()
            .filter(|e| **e == Err(gesamtgrenze(3)))
            .count();
        assert_eq!((neu, abgewiesen), (3, 29), "{}", kurz(&ergebnisse));
        assert_eq!(eintraege(&wurzel.path().join("direkt")).len(), 3);
    }

    /// Gesamtgrenze 5 → 429 `gesamt`, und sie steht vor der Herkunftsgrenze: auch `anna@10.0.0.1` mit ihren 3 Paaren
    /// bekommt `gesamt`, nicht `herkunft`.
    #[test]
    fn die_gesamtgrenze_steht_vor_der_herkunftsgrenze() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let gespraeche = laden_mit(wurzel.path(), grenzen(5, 3));
        let anna = anna_von("10.0.0.1");
        for nummer in 1..=3 {
            senden_an(&gespraeche, &anna, nummer).expect("anna");
        }
        senden_an(&gespraeche, &anna_von("10.0.0.2"), 4).expect("fünftes Paar? nein, viertes");
        senden_an(&gespraeche, &anna_von("10.0.0.3"), 5).expect("fünftes");

        let von_frischer_herkunft = senden_an(&gespraeche, &anna_von("10.0.0.4"), 6);
        let von_anna = senden_an(&gespraeche, &anna, 7);
        let bestehendes = senden_an(&gespraeche, &anna_von("10.0.0.3"), 5);

        assert_eq!(von_frischer_herkunft, Err(gesamtgrenze(5)));
        assert_eq!(von_anna, Err(gesamtgrenze(5)));
        assert!(
            matches!(bestehendes, Ok(Gesendet::Neu(_))),
            "{bestehendes:?}"
        );
    }

    /// Nach dem Neustart derselben Ablage gleiche Zählung (Anleger = Absender der ältesten Nachricht, aus dem
    /// Verlaufsindex; Mutante „Zählung nur im Speicher“); wer geantwortet hat, ist nicht der Anleger.
    #[test]
    fn nach_dem_neustart_zaehlt_dieselbe_ablage_gleich_und_der_anleger_ist_der_erste_absender() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let erste = laden_mit(wurzel.path(), grenzen(100, 3));
        let anna = anna_von("10.0.0.1");
        let ben = anna_von("10.0.0.2");
        for nummer in 1..=3 {
            senden_an(&erste, &anna, nummer).expect("anna");
        }
        // Das Gegenüber des ersten Paars antwortet: das Paar bleibt eines von anna.
        let antwort = Direktpaar::bilden(&anna, &gegenueber(1)).expect("Paar");
        erste
            .direkt_senden(&antwort, &gegenueber(1), Art::Agent, sendung("zurück"))
            .expect("Antwort");
        drop(erste);

        let neu = laden_mit(wurzel.path(), grenzen(100, 3));

        assert_eq!(senden_an(&neu, &anna, 4), Err(herkunftsgrenze(3)));
        for nummer in 11..=13 {
            let paar = senden_an(&neu, &ben, nummer);
            assert!(matches!(paar, Ok(Gesendet::Neu(_))), "{nummer}: {paar:?}");
        }
        // Wer nur geantwortet hat, hat nichts angelegt: das Gegenüber beginnt seine 3 eigenen Paare.
        for nummer in 20..=22 {
            let paar = senden_an(&neu, &gegenueber(1), nummer);
            assert!(matches!(paar, Ok(Gesendet::Neu(_))), "{nummer}: {paar:?}");
        }
        assert_eq!(senden_an(&neu, &gegenueber(1), 23), Err(herkunftsgrenze(3)));
    }

    /// Beim Start mehr Ordner als die Grenze: alle geladen, nur neue abgewiesen.
    #[test]
    fn beim_start_mit_mehr_ordnern_als_der_grenze_sind_alle_geladen_und_nur_neue_abgewiesen() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        {
            let ohne_grenze = laden_mit(wurzel.path(), grenzen(100, 100));
            for nummer in 1..=6 {
                senden_an(&ohne_grenze, &anna_von(&format!("10.0.0.{nummer}")), nummer)
                    .expect("angelegt");
            }
        }

        let gespraeche = laden_mit(wurzel.path(), grenzen(5, 100));

        for nummer in 1..=6 {
            let paar =
                Direktpaar::bilden(&anna_von(&format!("10.0.0.{nummer}")), &gegenueber(nummer))
                    .expect("Paar");
            assert!(gespraeche.direkt_bekannt(&paar), "{nummer} geladen");
        }
        assert_eq!(
            senden_an(&gespraeche, &anna_von("10.0.0.9"), 9),
            Err(gesamtgrenze(5))
        );
        let bestehendes = senden_an(&gespraeche, &anna_von("10.0.0.1"), 1);
        assert!(
            matches!(bestehendes, Ok(Gesendet::Neu(_))),
            "{bestehendes:?}"
        );
    }

    /// Ein leerer Ordner (von Hand angelegt oder nach dem Entfallen noch nicht entfernt) zählt zur Gesamtgrenze,
    /// nicht zu einer Herkunft; das Senden in ihn ist keine neue Anlage und läuft auch an der Grenze.
    #[test]
    fn ein_leerer_ordner_zaehlt_gesamt_nicht_je_herkunft_und_sein_senden_ist_keine_anlage() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let anna = anna_von("10.0.0.1");
        let leer = Direktpaar::bilden(&anna, &gegenueber(9)).expect("Paar");
        std::fs::create_dir_all(wurzel.path().join("direkt").join(leer.ordnername()))
            .expect("leerer Ordner");
        let gespraeche = laden_mit(wurzel.path(), grenzen(4, 3));
        senden_an(&gespraeche, &anna, 1).expect("1");
        senden_an(&gespraeche, &anna, 2).expect("2");
        assert!(gespraeche.direkt_bekannt(&leer));

        let drittes = senden_an(&gespraeche, &anna, 3);
        let viertes_ueber_gesamt = senden_an(&gespraeche, &anna_von("10.0.0.2"), 4);
        let in_den_leeren = senden_an(&gespraeche, &anna, 9);

        assert!(
            matches!(drittes, Ok(Gesendet::Neu(_))),
            "leer zählt nicht je Herkunft: {drittes:?}"
        );
        assert_eq!(
            viertes_ueber_gesamt,
            Err(gesamtgrenze(4)),
            "leer zählt gesamt"
        );
        assert!(
            matches!(in_den_leeren, Ok(Gesendet::Neu(_))),
            "{in_den_leeren:?}"
        );
    }

    /// Nach dem Entfallen und Entfernen (B0777, B0778) ist ein Platz frei: die Grenze ist kein Gegenzug-Ausweg und
    /// kein Dauerzustand.
    #[test]
    fn nach_entfallen_und_entfernen_ist_ein_platz_frei() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let gespraeche = laden_mit(wurzel.path(), grenzen(100, 2));
        let anna = anna_von("10.0.0.1");
        senden_an(&gespraeche, &anna, 1).expect("1");
        senden_an(&gespraeche, &anna, 2).expect("2");
        assert_eq!(senden_an(&gespraeche, &anna, 3), Err(herkunftsgrenze(2)));

        gespraeche.pflegen(
            chrono::NaiveDate::from_ymd_opt(2026, 11, 5).expect("Tag"),
            crate::gespraeche::Pflegeanlass::Tageswechsel,
        );

        let frei = senden_an(&gespraeche, &anna, 3);
        assert!(matches!(frei, Ok(Gesendet::Neu(_))), "{frei:?}");
    }

    /// Wertprüfung (D13, Mutanten 4.095/4.097): die Zahlen der ADR — 4.096 je Herkunft (128 Sitzungen × 32 Gegenüber),
    /// 8.192 gesamt (2 Herkünfte, die ihre Grenze ausschöpfen).
    #[test]
    fn die_konstanten_sind_die_zahlen_der_adr() {
        assert_eq!(DIREKT_JE_HERKUNFT, 4_096);
        assert_eq!(DIREKT_GESAMT, 8_192);
        assert_eq!(DIREKT_GESAMT, 2 * DIREKT_JE_HERKUNFT);
        assert_eq!(Direktgrenzen::default(), grenzen(8_192, 4_096));
    }

    /// Security-Review #13 (Optional): `ordner_fuer_protokoll` ersetzt auch Zeilentrenner (U+2028/2029, U+0085) und
    /// Richtungs- und Formatzeichen (Bidi, Zero-Width, BOM) — ein Ordnername von Hand bleibt eine Zeile und kehrt
    /// die Lesrichtung der Zeile nicht um.
    #[test]
    fn der_ordner_im_protokoll_traegt_keine_zeilentrenner_und_keine_richtungszeichen() {
        for zeichen in [
            '\u{2028}', '\u{2029}', '\u{85}', '\u{202E}', '\u{202A}', '\u{2066}', '\u{2069}',
            '\u{200B}', '\u{200F}', '\u{FEFF}', '\u{061C}', '\u{00AD}', '\n', '\u{7}',
        ] {
            let name = format!("a{zeichen}b");

            let im_protokoll = ordner_fuer_protokoll(&name);

            assert_eq!(im_protokoll, "direkt/a?b", "U+{:04X}", zeichen as u32);
        }
        assert_eq!(
            ordner_fuer_protokoll("anna@fd00%3A%3A7+ben@1.1.1.1"),
            "direkt/anna@fd00%3A%3A7+ben@1.1.1.1"
        );
        assert_eq!(ordner_fuer_protokoll("müller+ünal"), "direkt/müller+ünal");
    }
}
