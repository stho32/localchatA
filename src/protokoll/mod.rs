//! Betriebsprotokoll (A00015, ADR-0040 bis ADR-0043): eine escapte Zeile je Eintrag in `logs/<lokales Datum>.log`,
//! ab Warning dieselbe Zeile auf stderr.

pub mod bereinigung;
pub mod senke;
pub mod tagestakt;
pub mod zeile;

pub use zeile::{Bereich, Eintrag, Kontext, Schluessel, Stufe};

use crate::uhr::{Uhr, Zeit};
use bereinigung::{Anlass, Bereinigung};
use chrono::NaiveDate;
use senke::{Senke, Tagesdateien};
use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

/// Wohin Zeilen auf stderr gehen — im Programm stderr, im Test eine Aufzeichnung.
pub type Spiegel = Arc<dyn Fn(&str) + Send + Sync>;

/// Das Betriebsprotokoll einer Instanz; geteilt als `Arc<Protokoll>`, nie global (außer im Panik-Haken).
pub struct Protokoll {
    uhr: Arc<dyn Uhr>,
    mindeststufe: Stufe,
    spiegel: Spiegel,
    inneres: Mutex<Inneres>,
    ok: AtomicBool,
    /// Namen beendeter Hintergrundaufgaben (ADR-0042 Regel 4) — für `aufgaben_ausgefallen`.
    ausgefallen: Mutex<Vec<&'static str>>,
}

struct Inneres {
    senke: Box<dyn Senke>,
    /// Im Rückfall auf stderr (ADR-0040 Regel 8): die Episode hat ihre Warning schon geschrieben.
    rueckfall: bool,
}

impl Protokoll {
    /// Das Protokoll unter `<ordner>/logs/`; scheitert nie — lässt sich `logs/` nicht anlegen, beginnt es im
    /// Rückfall auf stderr.
    pub fn oeffnen(ordner: &Path, uhr: Arc<dyn Uhr>, mindeststufe: Stufe) -> Protokoll {
        Protokoll::oeffnen_mit_spiegel(ordner, uhr, mindeststufe, stderr_spiegel())
    }

    /// Wie `oeffnen`, mit eigenem Spiegel statt stderr.
    pub fn oeffnen_mit_spiegel(
        ordner: &Path,
        uhr: Arc<dyn Uhr>,
        mindeststufe: Stufe,
        spiegel: Spiegel,
    ) -> Protokoll {
        let dateien = Tagesdateien::neu(ordner);
        let angelegt = dateien.ordner_anlegen();
        let protokoll = Protokoll::mit(Box::new(dateien), spiegel, uhr, mindeststufe);
        if let Err(fehler) = angelegt {
            let tag = protokoll.uhr.jetzt().date_naive();
            let mut inneres = protokoll.sperren();
            protokoll.rueckfall_beginnen(&mut inneres, tag, &fehler);
        }
        protokoll
    }

    /// Ein Protokoll über eine beliebige Senke und einen beliebigen Spiegel (Tests).
    pub fn mit(
        senke: Box<dyn Senke>,
        spiegel: Spiegel,
        uhr: Arc<dyn Uhr>,
        mindeststufe: Stufe,
    ) -> Protokoll {
        Protokoll {
            uhr,
            mindeststufe,
            spiegel,
            inneres: Mutex::new(Inneres {
                senke,
                rueckfall: false,
            }),
            ok: AtomicBool::new(true),
            ausgefallen: Mutex::new(Vec::new()),
        }
    }

    /// Ein Protokoll, dessen Einträge nirgends stehen — für Bausteine, die außerhalb des Programms laufen
    /// (Testserver, Werkzeuge); Zähler und Kennzahlen arbeiten wie sonst.
    pub fn ohne_ablage() -> Protokoll {
        Protokoll::mit(
            Box::new(senke::Speicher::neu()),
            Arc::new(|_: &str| {}),
            Arc::new(crate::uhr::Systemuhr),
            Stufe::Information,
        )
    }

    /// `true`, solange Einträge in die Datei gehen; `false` im Rückfall auf stderr (ADR-0043 Regel 5).
    pub fn ok(&self) -> bool {
        self.ok.load(Ordering::SeqCst)
    }

    /// Eine Hintergrundaufgabe ist beendet: `ERROR programmfehler Hintergrundaufgabe <name> beendet — <folge>`,
    /// und ihr Name steht in den ausgefallenen.
    pub fn aufgabe_ausgefallen(&self, name: &'static str, folge: &str) {
        self.ausgefallen
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(name);
        self.eintragen(
            Stufe::Error,
            Bereich::Programmfehler,
            format!("Hintergrundaufgabe {name} beendet — {folge}"),
            Kontext::neu(),
        );
    }

    /// Namen der beendeten Hintergrundaufgaben in der Reihenfolge ihres Ausfalls.
    pub fn ausgefallen(&self) -> Vec<&'static str> {
        self.ausgefallen
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    pub fn mindeststufe(&self) -> Stufe {
        self.mindeststufe
    }

    /// Schreibt einen Eintrag in die Datei des lokalen Datums seiner Zeit; unter der Mindeststufe nichts;
    /// ab Warning zusätzlich auf stderr.
    pub fn eintragen(
        &self,
        stufe: Stufe,
        bereich: Bereich,
        text: impl Into<String>,
        kontext: Kontext,
    ) {
        if stufe < self.mindeststufe {
            return;
        }
        let jetzt = self.uhr.jetzt();
        let eintrag = Eintrag::neu(stufe, bereich, text, kontext);
        let mut inneres = self.sperren();
        self.ablegen(&mut inneres, &eintrag, jetzt);
    }

    /// Löscht die Protokolldateien, die älter als 14 Kalendertage sind, und trägt das Ergebnis ein: eine
    /// INFORMATION, wenn etwas entfiel, je gescheiterter Datei ein ERROR (ADR-0041 Regel 7).
    pub fn bereinigen(&self, heute: NaiveDate, anlass: Anlass) -> Bereinigung {
        let Some(logs) = self.sperren().senke.ordner() else {
            return Bereinigung::default();
        };
        let bereinigung = match bereinigung::bereinigen_im_ordner(&logs, heute) {
            Ok(bereinigung) => bereinigung,
            Err(fehler) => {
                self.eintragen(
                    Stufe::Error,
                    Bereich::Protokoll,
                    "Protokollordner nicht lesbar — alte Protokolldateien bleiben liegen",
                    Kontext::neu()
                        .mit(Schluessel::Datei, senke::LOGS)
                        .mit(Schluessel::Grund, fehler),
                );
                return Bereinigung::default();
            }
        };
        if let (Some(aelteste), Some(juengste)) =
            (bereinigung.entfallen.first(), bereinigung.entfallen.last())
        {
            self.eintragen(
                Stufe::Information,
                Bereich::Protokoll,
                format!(
                    "{} Protokolldateien entfallen ({aelteste} bis {juengste}; Aufbewahrung {} Tage; {})",
                    bereinigung.entfallen.len(),
                    bereinigung::AUFBEWAHRUNG_TAGE,
                    anlass.als_text()
                ),
                Kontext::neu(),
            );
        }
        for (datei, grund) in &bereinigung.gescheitert {
            self.eintragen(
                Stufe::Error,
                Bereich::Protokoll,
                "Protokolldatei nicht entfallen",
                Kontext::neu()
                    .mit(Schluessel::Datei, datei)
                    .mit(Schluessel::Grund, grund),
            );
        }
        bereinigung
    }

    fn sperren(&self) -> MutexGuard<'_, Inneres> {
        self.inneres.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Die Zeile in die Datei, ab Warning auf stderr; scheitert die Datei, Rückfall: die Zeile auf stderr.
    fn ablegen(&self, inneres: &mut Inneres, eintrag: &Eintrag, jetzt: Zeit) {
        let tag = jetzt.date_naive();
        if inneres.rueckfall && self.mindeststufe <= Stufe::Information {
            self.rueckfall_beenden(inneres, jetzt);
        }
        let zeile = zeile::zeile(eintrag, jetzt);
        match inneres.senke.anhaengen(tag, &zeile) {
            Ok(_) => {
                inneres.rueckfall = false;
                self.ok.store(true, Ordering::SeqCst);
                if eintrag.stufe >= Stufe::Warning {
                    (self.spiegel)(&zeile);
                }
            }
            Err(fehler) => {
                if !inneres.rueckfall {
                    self.rueckfall_beginnen(inneres, tag, &fehler);
                }
                (self.spiegel)(&zeile);
            }
        }
    }

    /// Eine Warning je Episode, nur auf stderr; das Protokoll gilt als nicht schreibbar.
    fn rueckfall_beginnen(&self, inneres: &mut Inneres, tag: NaiveDate, fehler: &io::Error) {
        inneres.rueckfall = true;
        self.ok.store(false, Ordering::SeqCst);
        let warnung = Eintrag::neu(
            Stufe::Warning,
            Bereich::Protokoll,
            "Betriebsprotokoll nicht schreibbar — Einträge gehen bis auf Weiteres nur auf stderr",
            Kontext::neu()
                .mit(Schluessel::Datei, inneres.senke.datei(tag))
                .mit(Schluessel::Grund, fehler),
        );
        (self.spiegel)(&zeile::zeile(&warnung, self.uhr.jetzt()));
    }

    /// Versucht die Information, die die Episode beendet; gelingt sie, geht es in der Datei weiter.
    fn rueckfall_beenden(&self, inneres: &mut Inneres, jetzt: Zeit) {
        let ende = Eintrag::neu(
            Stufe::Information,
            Bereich::Protokoll,
            "Betriebsprotokoll wieder schreibbar — Einträge gehen wieder in die Datei",
            Kontext::neu(),
        );
        if inneres
            .senke
            .anhaengen(jetzt.date_naive(), &zeile::zeile(&ende, jetzt))
            .is_ok()
        {
            inneres.rueckfall = false;
            self.ok.store(true, Ordering::SeqCst);
        }
    }
}

/// stderr des Prozesses; eine Zeile je Aufruf in einem `write_all`.
fn stderr_spiegel() -> Spiegel {
    Arc::new(|zeile: &str| {
        let _ = std::io::stderr().lock().write_all(zeile.as_bytes());
    })
}

#[cfg(test)]
pub(crate) mod testhilfe {
    use super::*;
    use crate::uhr::{GestellteUhr, Zeit};
    use senke::Speicher;

    pub fn zeit(text: &str) -> Zeit {
        Zeit::parse_from_rfc3339(text).expect("Testzeit")
    }

    /// Aufgezeichnete stderr-Zeilen (ohne Zeilenende).
    #[derive(Clone, Default)]
    pub struct Aufzeichnung(pub Arc<Mutex<Vec<String>>>);

    impl Aufzeichnung {
        pub fn spiegel(&self) -> Spiegel {
            let zeilen = Arc::clone(&self.0);
            Arc::new(move |zeile: &str| {
                zeilen
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .push(zeile.trim_end_matches('\n').to_string());
            })
        }

        pub fn zeilen(&self) -> Vec<String> {
            self.0
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone()
        }
    }

    /// Protokoll in eine Speicher-Senke mit gestellter Uhr und aufgezeichnetem stderr.
    pub fn im_speicher(
        start: &str,
        mindeststufe: Stufe,
    ) -> (Arc<Protokoll>, Speicher, Arc<GestellteUhr>, Aufzeichnung) {
        let uhr = Arc::new(GestellteUhr::neu(zeit(start)));
        let speicher = Speicher::neu();
        let stderr = Aufzeichnung::default();
        let protokoll = Protokoll::mit(
            Box::new(speicher.clone()),
            stderr.spiegel(),
            Arc::clone(&uhr) as Arc<dyn Uhr>,
            mindeststufe,
        );
        (Arc::new(protokoll), speicher, uhr, stderr)
    }
}

#[cfg(test)]
mod tests {
    use super::testhilfe::*;
    use super::*;
    use crate::uhr::GestellteUhr;
    use bereinigung::Anlass;

    struct InDateien {
        _wurzel: tempfile::TempDir,
        ordner: std::path::PathBuf,
        protokoll: Protokoll,
        uhr: Arc<GestellteUhr>,
        stderr: Aufzeichnung,
    }

    fn in_dateien(start: &str, mindeststufe: Stufe) -> InDateien {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let ordner = wurzel.path().to_path_buf();
        let uhr = Arc::new(GestellteUhr::neu(zeit(start)));
        let stderr = Aufzeichnung::default();
        let protokoll = Protokoll::mit(
            Box::new(Tagesdateien::neu(&ordner)),
            stderr.spiegel(),
            Arc::clone(&uhr) as Arc<dyn Uhr>,
            mindeststufe,
        );
        InDateien {
            _wurzel: wurzel,
            ordner,
            protokoll,
            uhr,
            stderr,
        }
    }

    impl InDateien {
        fn um(&self, zeitpunkt: &str, text: &str) {
            self.uhr.stellen(zeit(zeitpunkt));
            self.protokoll
                .eintragen(Stufe::Information, Bereich::Start, text, Kontext::neu());
        }

        fn datei(&self, name: &str) -> Vec<String> {
            std::fs::read_to_string(self.ordner.join("logs").join(name))
                .unwrap_or_default()
                .lines()
                .map(str::to_string)
                .collect()
        }
    }

    #[test]
    fn eintraege_um_mitternacht_liegen_in_der_datei_ihres_lokalen_tages() {
        let p = in_dateien("2026-10-28T12:00:00+01:00", Stufe::Information);
        p.um("2026-10-28T23:59:59.999+01:00", "vorher");
        p.um("2026-10-29T00:00:00.000+01:00", "nachher");

        assert_eq!(
            p.datei("2026-10-28.log"),
            ["2026-10-28T23:59:59.999+01:00 INFORMATION start vorher"]
        );
        assert_eq!(
            p.datei("2026-10-29.log"),
            ["2026-10-29T00:00:00.000+01:00 INFORMATION start nachher"]
        );
    }

    #[test]
    fn rueckstellung_und_vorstellung_bleiben_im_selben_tag_in_schreibreihenfolge() {
        let p = in_dateien("2026-10-25T00:00:00+02:00", Stufe::Information);
        p.um("2026-10-25T02:30:00.000+02:00", "a");
        p.um("2026-10-25T02:10:00.000+01:00", "b");
        p.um("2026-03-29T01:59:59.999+01:00", "c");
        p.um("2026-03-29T03:00:00.000+02:00", "d");

        let texte = |name| {
            p.datei(name)
                .iter()
                .map(|zeile| zeile.rsplit(' ').next().unwrap_or_default().to_string())
                .collect::<Vec<_>>()
        };
        assert_eq!(texte("2026-10-25.log"), ["a", "b"]);
        assert_eq!(texte("2026-03-29.log"), ["c", "d"]);
    }

    #[test]
    fn rueckwaerts_ueber_mitternacht_haengt_an_die_aeltere_datei_an() {
        let p = in_dateien("2026-10-28T12:00:00+01:00", Stufe::Information);
        p.um("2026-10-28T23:00:00.000+01:00", "erst");
        p.um("2026-10-29T00:00:01.000+01:00", "neu");
        p.um("2026-10-28T23:59:59.000+01:00", "zurück");

        assert_eq!(p.datei("2026-10-28.log").len(), 2);
        assert!(p.datei("2026-10-28.log")[1].ends_with("zurück"));
        assert_eq!(p.datei("2026-10-29.log").len(), 1);
    }

    #[test]
    fn das_datum_ist_das_lokale_nicht_das_utc_datum() {
        let p = in_dateien("2026-10-29T00:30:00.000+01:00", Stufe::Information);
        p.um("2026-10-29T00:30:00.000+01:00", "früh");

        assert_eq!(p.datei("2026-10-29.log").len(), 1);
        assert!(p.datei("2026-10-28.log").is_empty());
    }

    #[test]
    fn ab_warning_steht_dieselbe_zeile_auf_stderr_darunter_nie() {
        let p = in_dateien("2026-10-28T12:00:00+01:00", Stufe::Debug);
        for stufe in [
            Stufe::Debug,
            Stufe::Information,
            Stufe::Warning,
            Stufe::Error,
            Stufe::Exception,
        ] {
            p.protokoll
                .eintragen(stufe, Bereich::Laden, stufe.als_text(), Kontext::neu());
        }

        let datei = p.datei("2026-10-28.log");
        assert_eq!(datei.len(), 5);
        assert_eq!(p.stderr.zeilen(), datei[2..].to_vec());
    }

    #[test]
    fn unter_der_mindeststufe_wird_nichts_geschrieben() {
        let p = in_dateien("2026-10-28T12:00:00+01:00", Stufe::Warning);
        p.protokoll
            .eintragen(Stufe::Information, Bereich::Start, "leise", Kontext::neu());
        p.protokoll
            .eintragen(Stufe::Warning, Bereich::Start, "laut", Kontext::neu());

        assert_eq!(p.datei("2026-10-28.log").len(), 1);
    }

    #[test]
    fn einen_vom_betreiber_geloeschten_logs_ordner_legt_der_naechste_eintrag_neu_an() {
        let p = in_dateien("2026-10-28T12:00:00+01:00", Stufe::Information);
        p.um("2026-10-28T12:00:00+01:00", "eins");
        std::fs::remove_dir_all(p.ordner.join("logs")).expect("logs weg");
        p.um("2026-10-29T12:00:00+01:00", "zwei");

        assert_eq!(p.datei("2026-10-29.log").len(), 1);
    }

    #[test]
    fn rueckfall_eintraege_gehen_ab_der_mindeststufe_auf_stderr_mit_einer_warning_je_episode() {
        let (protokoll, speicher, _, stderr) =
            im_speicher("2026-10-28T12:00:00+01:00", Stufe::Information);
        speicher.scheitern_lassen(true);

        protokoll.eintragen(Stufe::Information, Bereich::Start, "eins", Kontext::neu());
        protokoll.eintragen(Stufe::Debug, Bereich::Start, "leise", Kontext::neu());
        protokoll.eintragen(Stufe::Information, Bereich::Sitzung, "zwei", Kontext::neu());

        let zeilen = stderr.zeilen();
        assert_eq!(zeilen.len(), 3, "{zeilen:#?}");
        assert!(
            zeilen[0].starts_with("2026-10-28T12:00:00.000+01:00 WARNING protokoll Betriebsprotokoll nicht schreibbar — Einträge gehen bis auf Weiteres nur auf stderr | datei=logs/2026-10-28.log grund="),
            "{zeilen:#?}"
        );
        assert!(zeilen[1].ends_with("INFORMATION start eins"), "{zeilen:#?}");
        assert!(
            zeilen[2].ends_with("INFORMATION sitzung zwei"),
            "{zeilen:#?}"
        );
        assert!(!protokoll.ok());
    }

    #[test]
    fn die_naechste_gelungene_zeile_beendet_den_rueckfall_mit_einer_information() {
        let (protokoll, speicher, _, stderr) =
            im_speicher("2026-10-28T12:00:00+01:00", Stufe::Information);
        speicher.scheitern_lassen(true);
        protokoll.eintragen(Stufe::Warning, Bereich::Laden, "verloren", Kontext::neu());
        speicher.scheitern_lassen(false);

        protokoll.eintragen(
            Stufe::Information,
            Bereich::Sitzung,
            "beigetreten",
            Kontext::neu(),
        );
        speicher.scheitern_lassen(true);
        protokoll.eintragen(
            Stufe::Information,
            Bereich::Sitzung,
            "wieder",
            Kontext::neu(),
        );

        let datei = speicher.zeilen();
        assert_eq!(datei.len(), 2, "{datei:#?}");
        assert!(
            datei[0].contains(" INFORMATION protokoll Betriebsprotokoll wieder schreibbar"),
            "{datei:#?}"
        );
        assert!(
            datei[1].ends_with("INFORMATION sitzung beigetreten"),
            "{datei:#?}"
        );
        let warnungen = stderr
            .zeilen()
            .iter()
            .filter(|zeile| zeile.contains("nicht schreibbar"))
            .count();
        assert_eq!(warnungen, 2, "eine neue Episode, eine neue Warning");
        assert!(!protokoll.ok());
    }

    #[test]
    fn ohne_rueckfall_ist_das_protokoll_ok_und_nichts_doppelt_auf_stderr() {
        let (protokoll, speicher, _, stderr) =
            im_speicher("2026-10-28T12:00:00+01:00", Stufe::Information);
        protokoll.eintragen(Stufe::Warning, Bereich::Laden, "eins", Kontext::neu());

        assert!(protokoll.ok());
        assert_eq!(speicher.zeilen().len(), 1);
        assert_eq!(stderr.zeilen(), speicher.zeilen());
    }

    #[cfg(unix)]
    #[test]
    fn ist_logs_eine_datei_startet_das_protokoll_im_rueckfall() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        std::fs::write(wurzel.path().join("logs"), "keine Ablage").expect("Datei logs");
        let uhr: Arc<dyn Uhr> = Arc::new(GestellteUhr::neu(zeit("2026-10-28T12:00:00+01:00")));
        let stderr = Aufzeichnung::default();

        let protokoll = Protokoll::oeffnen_mit_spiegel(
            wurzel.path(),
            uhr,
            Stufe::Information,
            stderr.spiegel(),
        );
        protokoll.eintragen(Stufe::Information, Bereich::Start, "bereit", Kontext::neu());

        assert!(!protokoll.ok());
        let zeilen = stderr.zeilen();
        assert_eq!(zeilen.len(), 2, "{zeilen:#?}");
        assert!(zeilen[0].contains("WARNING protokoll Betriebsprotokoll nicht schreibbar"));
        assert!(zeilen[1].ends_with("INFORMATION start bereit"));
        assert_eq!(
            std::fs::read_to_string(wurzel.path().join("logs")).expect("logs"),
            "keine Ablage",
            "eine Datei namens logs wird nie ersetzt"
        );
    }

    #[test]
    fn die_bereinigung_berichtet_eine_information_mit_anzahl_spanne_und_anlass() {
        let p = in_dateien("2026-10-28T12:00:00+01:00", Stufe::Information);
        let logs = p.ordner.join("logs");
        std::fs::create_dir_all(&logs).expect("logs");
        for name in [
            "2026-10-13.log",
            "2026-10-14.log",
            "2026-10-15.log",
            "notizen.txt",
        ] {
            std::fs::write(logs.join(name), "x").expect("Datei");
        }

        p.protokoll.bereinigen(
            zeit("2026-10-28T12:00:00+01:00").date_naive(),
            Anlass::Start,
        );
        p.protokoll.bereinigen(
            zeit("2026-10-28T12:00:00+01:00").date_naive(),
            Anlass::Start,
        );

        assert_eq!(
            p.datei("2026-10-28.log"),
            [
                "2026-10-28T12:00:00.000+01:00 INFORMATION protokoll 2 Protokolldateien entfallen (2026-10-13 bis 2026-10-14; Aufbewahrung 14 Tage; Start)"
            ],
            "der zweite Lauf ohne Entfallendes schreibt nichts"
        );
    }

    #[cfg(unix)]
    #[test]
    fn je_nicht_loeschbarer_datei_steht_ein_error_mit_datei_und_grund() {
        use std::os::unix::fs::PermissionsExt;
        let p = in_dateien("2026-10-28T12:00:00+01:00", Stufe::Information);
        let logs = p.ordner.join("logs");
        std::fs::create_dir_all(&logs).expect("logs");
        for name in ["2026-10-13.log", "2026-10-14.log", "2026-10-28.log"] {
            std::fs::write(logs.join(name), "").expect("Datei");
        }
        std::fs::set_permissions(&logs, std::fs::Permissions::from_mode(0o555)).expect("chmod");

        p.protokoll.bereinigen(
            zeit("2026-10-28T12:00:00+01:00").date_naive(),
            Anlass::Tageswechsel,
        );
        std::fs::set_permissions(&logs, std::fs::Permissions::from_mode(0o755)).expect("chmod");

        let datei = p.datei("2026-10-28.log");
        assert_eq!(datei.len(), 2, "{datei:#?}");
        assert!(
            datei[0].contains(
                " ERROR protokoll Protokolldatei nicht entfallen | datei=logs/2026-10-13.log grund="
            ),
            "{datei:#?}"
        );
        assert!(
            datei[1].contains("datei=logs/2026-10-14.log grund="),
            "{datei:#?}"
        );
    }
}
