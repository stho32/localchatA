//! Bereinigung nach 14 Kalendertagen (ADR-0041 Regeln 4–7): welche Protokolldatei entfällt, und das Löschen
//! nach Erlaubnisliste.

use super::senke::dateiname;
use chrono::NaiveDate;
use std::io;
use std::path::Path;

/// Aufbewahrung in Kalendertagen; heute zählt mit (ADR-0029, ADR-0041 Regel 4).
pub const AUFBEWAHRUNG_TAGE: i64 = 14;

/// Der Tag aus einem Dateinamen, wenn er genau `JJJJ-MM-TT.log` mit gültigem Datum ist; sonst ist die Datei fremd.
pub fn tag_aus_namen(name: &str) -> Option<NaiveDate> {
    let stamm = name.strip_suffix(".log")?;
    let form_passt = stamm.len() == 10
        && stamm.char_indices().all(|(stelle, zeichen)| match stelle {
            4 | 7 => zeichen == '-',
            _ => zeichen.is_ascii_digit(),
        });
    if !form_passt {
        return None;
    }
    NaiveDate::parse_from_str(stamm, "%Y-%m-%d").ok()
}

/// Datei vom Tag `tag` entfällt ⇔ `tag < heute − 13`.
pub fn entfaellt(tag: NaiveDate, heute: NaiveDate) -> bool {
    tag < heute - chrono::Duration::days(AUFBEWAHRUNG_TAGE - 1)
}

/// Was eine Bereinigung bewirkte: entfallene Tage (aufsteigend) und je gescheiterter Datei Pfad und Grund.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Bereinigung {
    pub entfallen: Vec<NaiveDate>,
    pub gescheitert: Vec<(String, String)>,
}

/// Anlass einer Bereinigung — steht im Text der Zeile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anlass {
    Start,
    Tageswechsel,
}

impl Anlass {
    pub fn als_text(self) -> &'static str {
        match self {
            Anlass::Start => "Start",
            Anlass::Tageswechsel => "Tageswechsel",
        }
    }
}

/// Löscht in `logs` jede reguläre Datei `JJJJ-MM-TT.log`, deren Tag entfällt (Pfad aus dem Datum, geprüft mit
/// `symlink_metadata`); Unterordner, Symlinks und fremde Dateien bleiben. Fehlt `logs`, entfällt nichts.
pub fn bereinigen_im_ordner(logs: &Path, heute: NaiveDate) -> io::Result<Bereinigung> {
    let eintraege = match std::fs::read_dir(logs) {
        Ok(eintraege) => eintraege,
        Err(fehler) if fehler.kind() == io::ErrorKind::NotFound => {
            return Ok(Bereinigung::default());
        }
        Err(fehler) => return Err(fehler),
    };
    let mut tage: Vec<NaiveDate> = eintraege
        .filter_map(Result::ok)
        .filter_map(|eintrag| eintrag.file_name().to_str().and_then(tag_aus_namen))
        .filter(|tag| entfaellt(*tag, heute))
        .collect();
    tage.sort();
    let mut bereinigung = Bereinigung::default();
    for tag in tage {
        let pfad = logs.join(dateiname(tag));
        let regulaer =
            std::fs::symlink_metadata(&pfad).is_ok_and(|daten| daten.file_type().is_file());
        if !regulaer {
            continue;
        }
        match std::fs::remove_file(&pfad) {
            Ok(()) => bereinigung.entfallen.push(tag),
            Err(fehler) => bereinigung
                .gescheitert
                .push((format!("logs/{}", dateiname(tag)), fehler.to_string())),
        }
    }
    Ok(bereinigung)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tag(text: &str) -> NaiveDate {
        NaiveDate::parse_from_str(text, "%Y-%m-%d").expect("Tag")
    }

    /// Der Ordner aus dem Kriterium „Bereinigung“ (R00014 F0052); liefert Wurzel und `logs/`.
    fn ordner_aus_dem_kriterium() -> (tempfile::TempDir, std::path::PathBuf) {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let logs = wurzel.path().join("logs");
        std::fs::create_dir(&logs).expect("logs");
        for name in [
            "2026-10-13.log",
            "2026-10-14.log",
            "2026-10-15.log",
            "2026-10-28.log",
            "notizen.txt",
            "2026-10-14.log.gz",
        ] {
            std::fs::write(logs.join(name), name).expect("Datei");
        }
        std::fs::write(wurzel.path().join("ziel.log"), "Ziel").expect("Ziel");
        #[cfg(unix)]
        std::os::unix::fs::symlink(wurzel.path().join("ziel.log"), logs.join("2026-10-01.log"))
            .expect("Symlink");
        std::fs::create_dir(logs.join("2026-10-02.log")).expect("Ordner");
        (wurzel, logs)
    }

    fn inhalt(logs: &Path) -> Vec<String> {
        let mut namen: Vec<String> = std::fs::read_dir(logs)
            .expect("logs")
            .map(|e| {
                e.expect("Eintrag")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        namen.sort();
        namen
    }

    #[test]
    fn entfernt_genau_die_alten_tagesdateien_symlink_ordner_und_fremdes_bleiben() {
        let (wurzel, logs) = ordner_aus_dem_kriterium();

        let bereinigung = bereinigen_im_ordner(&logs, tag("2026-10-28")).expect("bereinigt");

        assert_eq!(
            bereinigung.entfallen,
            [tag("2026-10-13"), tag("2026-10-14")]
        );
        assert!(bereinigung.gescheitert.is_empty());
        let mut erwartet = vec![
            "2026-10-02.log",
            "2026-10-14.log.gz",
            "2026-10-15.log",
            "2026-10-28.log",
            "notizen.txt",
        ];
        if cfg!(unix) {
            erwartet.insert(0, "2026-10-01.log");
        }
        assert_eq!(inhalt(&logs), erwartet);
        assert!(
            wurzel.path().join("ziel.log").exists(),
            "Ziel des Symlinks bleibt"
        );
    }

    #[test]
    fn ohne_logs_entfaellt_nichts() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        assert_eq!(
            bereinigen_im_ordner(&wurzel.path().join("logs"), tag("2026-10-28")).expect("leer"),
            Bereinigung::default()
        );
    }

    #[cfg(unix)]
    #[test]
    fn eine_nicht_loeschbare_datei_ist_gescheitert_mit_pfad_und_grund() {
        use std::os::unix::fs::PermissionsExt;
        let (_wurzel, logs) = ordner_aus_dem_kriterium();
        std::fs::set_permissions(&logs, std::fs::Permissions::from_mode(0o555)).expect("chmod");

        let bereinigung = bereinigen_im_ordner(&logs, tag("2026-10-28"));
        std::fs::set_permissions(&logs, std::fs::Permissions::from_mode(0o755)).expect("chmod");

        let bereinigung = bereinigung.expect("gelesen");
        assert!(bereinigung.entfallen.is_empty());
        let dateien: Vec<&str> = bereinigung
            .gescheitert
            .iter()
            .map(|(d, _)| d.as_str())
            .collect();
        assert_eq!(dateien, ["logs/2026-10-13.log", "logs/2026-10-14.log"]);
        assert!(
            bereinigung
                .gescheitert
                .iter()
                .all(|(_, grund)| !grund.is_empty())
        );
    }

    #[test]
    fn am_28_bleiben_der_15_bis_28_der_14_entfaellt() {
        let heute = tag("2026-10-28");
        assert!(entfaellt(tag("2026-10-13"), heute));
        assert!(entfaellt(tag("2026-10-14"), heute));
        assert!(!entfaellt(tag("2026-10-15"), heute));
        assert!(!entfaellt(tag("2026-10-28"), heute));
        assert!(!entfaellt(tag("2026-10-29"), heute), "Zukunft bleibt");
    }

    #[test]
    fn nur_genau_jjjj_mm_tt_log_mit_gueltigem_datum_ist_eine_protokolldatei() {
        assert_eq!(tag_aus_namen("2026-10-14.log"), Some(tag("2026-10-14")));
        for fremd in [
            "notizen.txt",
            "2026-10-14.log.gz",
            "2026-10-14.LOG",
            "2026-1-14.log",
            "2026-02-30.log",
            "x2026-10-14.log",
            "2026-10-14.logx",
            " 2026-10-14.log",
            "+2026-10-14.log",
            "2026-10-14",
        ] {
            assert_eq!(tag_aus_namen(fremd), None, "{fremd}");
        }
    }
}
