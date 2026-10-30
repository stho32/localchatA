//! Direktgespräche anlegen, laden und senden (A00006 Gespräche, ADR-0055 Regeln 1 und 4): ein Paar entsteht beim
//! ersten Senden unter der Sperre des Verzeichnisses, geladen wird nur, was ein Direktpaar benennt.

#[cfg(test)]
use super::schranke::Vorgang;
use super::verlaufsindex::Verlauf;
use super::{
    Art, Direktpaar, Gesendet, Gespraech, Gespraeche, Kennung, Sendefehler, Sendung, lesend,
    schreibend, sichtbar_fuer, sperren,
};
use crate::uhr::Zeit;
use std::sync::{Arc, Mutex};

/// Die Zeichen, die ein Ordnername im Protokoll höchstens trägt (ADR-0055 Regel 11, Matrix P2).
const ORDNER_IM_PROTOKOLL_HOECHSTENS: usize = 128;

/// `direkt/<name>` für eine Protokollzeile: Steuerzeichen durch `?` ersetzt, auf 128 Zeichen gekürzt — der Name
/// eines Ordners von Hand ist fremder Text und ergibt genau eine Zeile.
pub(super) fn ordner_fuer_protokoll(name: &str) -> String {
    format!("direkt/{name}")
        .chars()
        .map(|zeichen| if zeichen.is_control() { '?' } else { zeichen })
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
    /// Senden (Quote, Bezug, Anhang, Platte), wird ein dabei entstandener leerer Ordner wieder entfernt — ein Paar
    /// ohne Nachricht belegt nichts. Jede Prüfung davor (Teilnehmerliste, Grenzen) macht der Aufrufer.
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
            let neu = self.direkt_bereitstellen(&gespraech)?;
            let ergebnis = self.senden_sendung(&gespraech, von, art, sendung);
            if neu && ergebnis.is_err() {
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
    /// leeren Verlauf eintragen. `true`, wenn dieser Aufruf es angelegt hat. Gleichzeitiges erstes Senden beider
    /// Seiten ergibt einen Ordner, weil das Nachschlagen und das Anlegen unter derselben Sperre liegen.
    fn direkt_bereitstellen(&self, gespraech: &Gespraech) -> Result<bool, Sendefehler> {
        #[cfg(test)]
        self.schranken.passieren(Vorgang::DirektAnlegen);
        let mut verzeichnis = schreibend(&self.verzeichnis);
        if verzeichnis.verlaeufe.contains_key(gespraech) {
            return Ok(false);
        }
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
        verzeichnis
            .verlaeufe
            .insert(gespraech.clone(), Arc::new(Mutex::new(Verlauf::default())));
        Ok(angelegt)
    }

    /// Nimmt ein Paar zurück, dessen erstes Senden scheiterte: nur wenn es noch keine Nachricht hat, nur ein leerer
    /// Ordner (`rmdir`); scheitert das, bleibt es stehen und die Pflege räumt es auf (ADR-0055 Regel 10).
    fn direkt_zurueckbauen(&self, gespraech: &Gespraech) {
        let mut verzeichnis = schreibend(&self.verzeichnis);
        let leer = verzeichnis.verlaeufe.get(gespraech).is_some_and(|verlauf| {
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
}
