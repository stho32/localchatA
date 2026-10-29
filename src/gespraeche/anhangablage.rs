//! Anhangablage — Port der Logablage für den Eingang und die Anhänge (A00008 Ergänzungen Karte #9 und „Karte #9
//! Security-Gate“, ADR-0046 Ergänzung „Karte #9 Security-Gate“ Regeln 1–4): jeder Zugriff geht relativ zu beim
//! Start gehaltenen Verzeichnisdeskriptoren, Komponente für Komponente mit `O_NOFOLLOW`; nie über einen vollen Pfad.
//! Zwei Implementierungen: Dateisystem (Produktion) und Speicher (Tests mit Schranken und Fehlern).

use super::anhangid::Anhangid;
use super::anhangpfade;
use super::anhangverzeichnis::Tagesdatei;
use super::logablage::Gespraech;
use chrono::NaiveDate;
use std::fs::File;
use std::io;
use std::os::unix::fs::FileExt;
use std::path::{Path, PathBuf};

/// Eine Teildatei im Eingang (`.eingang/<id>.teil`): anhängen beim Hochladen, Lesen und Schreiben per Position
/// beim Tilgen. Die Länge ändert sich nach dem Hochladen nie.
pub trait Teildatei: Send + Sync {
    fn anhaengen(&mut self, bytes: &[u8]) -> io::Result<()>;
    fn lesen_an(&self, pos: u64, puffer: &mut [u8]) -> io::Result<usize>;
    fn schreiben_an(&self, pos: u64, bytes: &[u8]) -> io::Result<()>;
    fn laenge(&self) -> io::Result<u64>;
}

/// Eine gebundene Anhangdatei zum Lesen per Position (Ausliefern).
pub trait Anhangdatei: Send + Sync {
    fn lesen_an(&self, pos: u64, puffer: &mut [u8]) -> io::Result<usize>;
    fn laenge(&self) -> io::Result<u64>;
}

impl Anhangdatei for File {
    fn lesen_an(&self, pos: u64, puffer: &mut [u8]) -> io::Result<usize> {
        self.read_at(puffer, pos)
    }

    fn laenge(&self) -> io::Result<u64> {
        Ok(self.metadata()?.len())
    }
}

/// Was beim Leeren des Eingangs beim Start stehen blieb: Symlinks und Unterordner (`WARNING laden`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stehengeblieben {
    pub name: String,
    pub art: &'static str,
}

pub trait Anhangablage: Send + Sync {
    /// Legt `.eingang/<id>.teil` neu an (`O_CREAT | O_EXCL | O_NOFOLLOW`) — eine vorhandene Datei ist ein Fehler.
    fn teil_anlegen(&self, id: &Anhangid) -> io::Result<Box<dyn Teildatei>>;
    /// `.teil` → `.eingang/<id>` mit `RENAME_NOREPLACE` — ein vorhandenes Ziel ist ein Fehler und bleibt.
    fn teil_abschliessen(&self, id: &Anhangid) -> io::Result<()>;
    /// Löscht `.eingang/<id>.teil` und `.eingang/<id>`; was fehlt, gilt als gelöscht.
    fn eingang_loeschen(&self, id: &Anhangid) -> io::Result<()>;
    /// Löscht nur `.eingang/<id>.teil` (Abbruch eines `PUT`); fehlt sie, gilt sie als gelöscht.
    fn teil_loeschen(&self, id: &Anhangid) -> io::Result<()>;
    /// Binden: `.eingang/<id>` → `<gespräch>/anhaenge/<tag>/<datei>` ohne Überschreiben; `anhaenge` und der Tag
    /// werden bei Bedarf angelegt, nie über einen Symlink (`ELOOP`/`ENOTDIR` ist ein Fehler).
    fn binden(
        &self,
        id: &Anhangid,
        gespraech: &Gespraech,
        tag: NaiveDate,
        datei: &str,
    ) -> io::Result<()>;
    /// Zurücklegen nach gescheitertem Binden: `<tag>/<datei>` → `.eingang/<id>` ohne Überschreiben.
    fn zuruecklegen(
        &self,
        gespraech: &Gespraech,
        tag: NaiveDate,
        datei: &str,
        id: &Anhangid,
    ) -> io::Result<()>;
    /// Die regulären Dateien in `anhaenge/<tag>/` mit ihrer Länge; ein Symlink im Pfad ist ein Fehler.
    fn tag_dateien(&self, gespraech: &Gespraech, tag: NaiveDate) -> io::Result<Vec<Tagesdatei>>;
    /// Die Tage in `anhaenge/` (Ordner und Symlinks mit Tagesnamen), aufsteigend — auch Tage ohne Tagesdatei
    /// (Befund #53 B1); fehlt `anhaenge`, keine.
    fn anhang_tage(&self, gespraech: &Gespraech) -> io::Result<Vec<NaiveDate>>;
    /// Eine gebundene Datei zum Lesen, `O_NOFOLLOW` in jeder Komponente.
    fn zum_lesen(
        &self,
        gespraech: &Gespraech,
        tag: NaiveDate,
        datei: &str,
    ) -> io::Result<Box<dyn Anhangdatei>>;
    /// Kürzt eine gebundene Datei auf 0 Bytes; fehlt sie, ist das Erfolg.
    fn kuerzen(&self, gespraech: &Gespraech, tag: NaiveDate, datei: &str) -> io::Result<()>;
    /// Entfernt `anhaenge/<tag>` ohne einem Symlink zu folgen (ADR-0046 Ergänzung „Karte #9 Security-Gate“ Regel 4).
    fn tag_entfernen(&self, gespraech: &Gespraech, tag: NaiveDate) -> io::Result<()>;
}

/// Ohne Ablage für Anhänge (Gespräche ohne Start, Tests ohne Anhänge): jeder Zugriff scheitert.
pub struct KeineAnhangablage;

impl Anhangablage for KeineAnhangablage {
    fn teil_anlegen(&self, _: &Anhangid) -> io::Result<Box<dyn Teildatei>> {
        Err(ohne_ablage())
    }

    fn teil_abschliessen(&self, _: &Anhangid) -> io::Result<()> {
        Err(ohne_ablage())
    }

    fn eingang_loeschen(&self, _: &Anhangid) -> io::Result<()> {
        Ok(())
    }

    fn teil_loeschen(&self, _: &Anhangid) -> io::Result<()> {
        Ok(())
    }

    fn binden(&self, _: &Anhangid, _: &Gespraech, _: NaiveDate, _: &str) -> io::Result<()> {
        Err(ohne_ablage())
    }

    fn zuruecklegen(&self, _: &Gespraech, _: NaiveDate, _: &str, _: &Anhangid) -> io::Result<()> {
        Err(ohne_ablage())
    }

    fn tag_dateien(&self, _: &Gespraech, _: NaiveDate) -> io::Result<Vec<Tagesdatei>> {
        Ok(Vec::new())
    }

    fn anhang_tage(&self, _: &Gespraech) -> io::Result<Vec<NaiveDate>> {
        Ok(Vec::new())
    }

    fn zum_lesen(&self, _: &Gespraech, _: NaiveDate, _: &str) -> io::Result<Box<dyn Anhangdatei>> {
        Err(io::Error::from(io::ErrorKind::NotFound))
    }

    fn kuerzen(&self, _: &Gespraech, _: NaiveDate, _: &str) -> io::Result<()> {
        Err(ohne_ablage())
    }

    fn tag_entfernen(&self, _: &Gespraech, _: NaiveDate) -> io::Result<()> {
        Ok(())
    }
}

fn ohne_ablage() -> io::Error {
    io::Error::new(io::ErrorKind::Unsupported, "keine Ablage für Anhänge")
}

/// Warum der Eingang beim Start nicht taugt (ADR-0046 Ergänzung „Karte #9 Gate“ Regel 7): Exit 2, nichts gelöscht.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Eingangsfehler {
    /// `.eingang` ist ein Symlink oder kein Ordner.
    KeinOrdner { pfad: PathBuf },
    /// Anlegen, Leeren oder Öffnen scheiterte.
    Platte { pfad: PathBuf, grund: String },
}

impl std::fmt::Display for Eingangsfehler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Eingangsfehler::KeinOrdner { pfad } => write!(
                f,
                "{} ist kein Ordner (Symlink oder Datei) — die Instanz startet nicht; den Eintrag entfernen oder einen Ordner daraus machen",
                pfad.display()
            ),
            Eingangsfehler::Platte { pfad, grund } => {
                write!(f, "{} nicht nutzbar: {grund}", pfad.display())
            }
        }
    }
}

/// Die Ablage im Dateisystem: gehaltene Deskriptoren des Instanzordners und des Eingangs.
pub struct Anhangdateien {
    /// Der Instanzordner (der vom Betreiber genannte Pfad darf selbst ein Symlink sein — seine Wahl).
    instanz: rustix::fd::OwnedFd,
    eingang: rustix::fd::OwnedFd,
}

const EINGANG: &str = ".eingang";

impl Anhangdateien {
    /// Beim Start, nach der Sperre: `.eingang` per `symlink_metadata` prüfen — fehlt er, anlegen; Symlink oder kein
    /// Ordner → Fehler ohne Löschen; sonst leeren (nur reguläre Dateien) und mit `O_DIRECTORY | O_NOFOLLOW`
    /// öffnen. Ein später gegen einen Symlink getauschter `.eingang` ändert nichts mehr.
    pub fn oeffnen(ordner: &Path) -> Result<(Anhangdateien, Vec<Stehengeblieben>), Eingangsfehler> {
        let pfad = ordner.join(EINGANG);
        let platte = |fehler: io::Error| Eingangsfehler::Platte {
            pfad: pfad.clone(),
            grund: fehler.to_string(),
        };
        match std::fs::symlink_metadata(&pfad) {
            Ok(art) if art.file_type().is_dir() => {}
            Ok(_) => return Err(Eingangsfehler::KeinOrdner { pfad }),
            Err(fehler) if fehler.kind() == io::ErrorKind::NotFound => {
                std::fs::create_dir(&pfad).map_err(platte)?;
            }
            Err(fehler) => return Err(platte(fehler)),
        }
        let eingang = rustix::fs::open(
            &pfad,
            rustix::fs::OFlags::DIRECTORY
                | rustix::fs::OFlags::NOFOLLOW
                | rustix::fs::OFlags::CLOEXEC
                | rustix::fs::OFlags::RDONLY,
            rustix::fs::Mode::empty(),
        )
        .map_err(|fehler| platte(fehler.into()))?;
        let stehen = leeren(&eingang).map_err(platte)?;
        let instanz = rustix::fs::open(
            ordner,
            rustix::fs::OFlags::DIRECTORY
                | rustix::fs::OFlags::CLOEXEC
                | rustix::fs::OFlags::RDONLY,
            rustix::fs::Mode::empty(),
        )
        .map_err(|fehler| platte(fehler.into()))?;
        Ok((Anhangdateien { instanz, eingang }, stehen))
    }
}

/// Nur reguläre Dateien; Symlinks und Unterordner bleiben und werden gemeldet.
fn leeren(eingang: &rustix::fd::OwnedFd) -> io::Result<Vec<Stehengeblieben>> {
    let mut stehen = Vec::new();
    let mut namen = Vec::new();
    for eintrag in rustix::fs::Dir::read_from(eingang)? {
        let eintrag = eintrag?;
        let name = eintrag.file_name().to_string_lossy().into_owned();
        if name == "." || name == ".." {
            continue;
        }
        namen.push(name);
    }
    for name in namen {
        let art = rustix::fs::statat(
            eingang,
            name.as_str(),
            rustix::fs::AtFlags::SYMLINK_NOFOLLOW,
        )?;
        let typ = rustix::fs::FileType::from_raw_mode(art.st_mode);
        if typ.is_file() {
            rustix::fs::unlinkat(eingang, name.as_str(), rustix::fs::AtFlags::empty())?;
        } else {
            stehen.push(Stehengeblieben {
                name,
                art: if typ.is_symlink() {
                    "symlink"
                } else if typ.is_dir() {
                    "ordner"
                } else {
                    "andere"
                },
            });
        }
    }
    Ok(stehen)
}

fn teilname(id: &Anhangid) -> String {
    format!("{}.teil", id.als_text())
}

/// `renameat2(…, RENAME_NOREPLACE)`; kann das Dateisystem es nicht (`EINVAL`), `linkat` + `unlinkat`, das ebenfalls
/// nie überschreibt (ADR-0046 Ergänzung „Karte #9 Gate“ Regel 6).
pub(super) fn umbenennen_ohne_ueberschreiben(
    von_ordner: &rustix::fd::OwnedFd,
    von: &str,
    nach_ordner: &rustix::fd::OwnedFd,
    nach: &str,
) -> io::Result<()> {
    match rustix::fs::renameat_with(
        von_ordner,
        von,
        nach_ordner,
        nach,
        rustix::fs::RenameFlags::NOREPLACE,
    ) {
        Err(rustix::io::Errno::INVAL) => {
            rustix::fs::linkat(
                von_ordner,
                von,
                nach_ordner,
                nach,
                rustix::fs::AtFlags::empty(),
            )?;
            rustix::fs::unlinkat(von_ordner, von, rustix::fs::AtFlags::empty())?;
            Ok(())
        }
        anderes => anderes.map_err(io::Error::from),
    }
}

impl Anhangablage for Anhangdateien {
    fn teil_anlegen(&self, id: &Anhangid) -> io::Result<Box<dyn Teildatei>> {
        let fd = rustix::fs::openat(
            &self.eingang,
            teilname(id).as_str(),
            rustix::fs::OFlags::CREATE
                | rustix::fs::OFlags::EXCL
                | rustix::fs::OFlags::NOFOLLOW
                | rustix::fs::OFlags::CLOEXEC
                | rustix::fs::OFlags::RDWR,
            rustix::fs::Mode::from_raw_mode(0o644),
        )?;
        Ok(Box::new(DateiTeil {
            datei: File::from(fd),
            ende: 0,
        }))
    }

    fn teil_abschliessen(&self, id: &Anhangid) -> io::Result<()> {
        umbenennen_ohne_ueberschreiben(&self.eingang, &teilname(id), &self.eingang, id.als_text())
    }

    fn eingang_loeschen(&self, id: &Anhangid) -> io::Result<()> {
        self.teil_loeschen(id)?;
        loeschen_falls_da(&self.eingang, id.als_text())
    }

    fn teil_loeschen(&self, id: &Anhangid) -> io::Result<()> {
        loeschen_falls_da(&self.eingang, &teilname(id))
    }

    fn binden(
        &self,
        id: &Anhangid,
        gespraech: &Gespraech,
        tag: NaiveDate,
        datei: &str,
    ) -> io::Result<()> {
        let ziel = anhangpfade::tag_anlegen(&self.instanz, gespraech, tag)?;
        umbenennen_ohne_ueberschreiben(&self.eingang, id.als_text(), &ziel, datei)
    }

    fn zuruecklegen(
        &self,
        gespraech: &Gespraech,
        tag: NaiveDate,
        datei: &str,
        id: &Anhangid,
    ) -> io::Result<()> {
        let quelle = anhangpfade::tag(&self.instanz, gespraech, tag)?;
        umbenennen_ohne_ueberschreiben(&quelle, datei, &self.eingang, id.als_text())
    }

    fn tag_dateien(&self, gespraech: &Gespraech, tag: NaiveDate) -> io::Result<Vec<Tagesdatei>> {
        anhangpfade::tag_dateien(&self.instanz, gespraech, tag)
    }

    fn anhang_tage(&self, gespraech: &Gespraech) -> io::Result<Vec<NaiveDate>> {
        anhangpfade::anhang_tage(&self.instanz, gespraech)
    }

    fn zum_lesen(
        &self,
        gespraech: &Gespraech,
        tag: NaiveDate,
        datei: &str,
    ) -> io::Result<Box<dyn Anhangdatei>> {
        Ok(Box::new(anhangpfade::zum_lesen(
            &self.instanz,
            gespraech,
            tag,
            datei,
        )?))
    }

    fn kuerzen(&self, gespraech: &Gespraech, tag: NaiveDate, datei: &str) -> io::Result<()> {
        anhangpfade::kuerzen(&self.instanz, gespraech, tag, datei)
    }

    fn tag_entfernen(&self, gespraech: &Gespraech, tag: NaiveDate) -> io::Result<()> {
        anhangpfade::tag_entfernen(&self.instanz, gespraech, tag)
    }
}

/// `unlinkat`; was fehlt, gilt als gelöscht.
fn loeschen_falls_da(ordner: &rustix::fd::OwnedFd, name: &str) -> io::Result<()> {
    match rustix::fs::unlinkat(ordner, name, rustix::fs::AtFlags::empty()) {
        Ok(()) | Err(rustix::io::Errno::NOENT) => Ok(()),
        Err(fehler) => Err(fehler.into()),
    }
}

struct DateiTeil {
    datei: File,
    ende: u64,
}

impl Teildatei for DateiTeil {
    fn anhaengen(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.datei.write_all_at(bytes, self.ende)?;
        self.ende += bytes.len() as u64;
        Ok(())
    }

    fn lesen_an(&self, pos: u64, puffer: &mut [u8]) -> io::Result<usize> {
        self.datei.read_at(puffer, pos)
    }

    fn schreiben_an(&self, pos: u64, bytes: &[u8]) -> io::Result<()> {
        self.datei.write_all_at(bytes, pos)
    }

    fn laenge(&self) -> io::Result<u64> {
        Ok(self.datei.metadata()?.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    fn id(nummer: u128) -> Anhangid {
        let token = crate::zufall::Zufall::token(&crate::zufall::FesteFolge::neu([nummer]))
            .expect("Zufall");
        Anhangid::aus_token(&token).expect("ID")
    }

    /// Fehlt `.eingang`, legt der Start ihn an; reguläre Dateien gehen, Symlink und Unterordner bleiben gemeldet,
    /// das Ziel des Symlinks bleibt unberührt (R00029 „Start leert den Eingang“).
    #[test]
    fn der_start_legt_an_leert_nur_regulaere_dateien_und_meldet_den_rest() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let (_, stehen) = Anhangdateien::oeffnen(wurzel.path()).expect("angelegt");
        assert!(stehen.is_empty());
        assert!(wurzel.path().join(".eingang").is_dir());

        let fremd = tempfile::tempdir().expect("fremder Ordner");
        std::fs::write(fremd.path().join("ziel"), b"fremd").expect("Ziel");
        std::fs::write(wurzel.path().join(".eingang/aaaa"), b"x").expect("Datei");
        std::fs::write(wurzel.path().join(".eingang/bbbb.teil"), b"x").expect("Teil");
        symlink(
            fremd.path().join("ziel"),
            wurzel.path().join(".eingang/zeiger"),
        )
        .expect("Symlink");
        std::fs::create_dir(wurzel.path().join(".eingang/unter")).expect("Unterordner");

        let (_, mut stehen) = Anhangdateien::oeffnen(wurzel.path()).expect("geleert");

        stehen.sort_by(|a, b| a.name.cmp(&b.name));
        assert_eq!(
            stehen,
            vec![
                Stehengeblieben {
                    name: "unter".into(),
                    art: "ordner"
                },
                Stehengeblieben {
                    name: "zeiger".into(),
                    art: "symlink"
                },
            ]
        );
        assert!(!wurzel.path().join(".eingang/aaaa").exists());
        assert!(!wurzel.path().join(".eingang/bbbb.teil").exists());
        assert_eq!(
            std::fs::read(fremd.path().join("ziel")).expect("Ziel"),
            b"fremd"
        );
    }

    /// M26: `.eingang` als Symlink auf einen fremden Ordner oder als reguläre Datei → Fehler mit dem Pfad, nichts
    /// gelöscht.
    #[test]
    fn ein_eingang_als_symlink_oder_datei_ist_ein_fehler_ohne_loeschen() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let fremd = tempfile::tempdir().expect("fremder Ordner");
        std::fs::write(fremd.path().join("datei"), b"fremd").expect("fremde Datei");
        symlink(fremd.path(), wurzel.path().join(".eingang")).expect("Symlink");
        let fehler = Anhangdateien::oeffnen(wurzel.path())
            .err()
            .expect("abgewiesen");
        assert_eq!(
            fehler,
            Eingangsfehler::KeinOrdner {
                pfad: wurzel.path().join(".eingang")
            }
        );
        assert!(
            fehler
                .to_string()
                .contains(&wurzel.path().join(".eingang").display().to_string())
        );
        assert_eq!(
            std::fs::read(fremd.path().join("datei")).expect("bleibt"),
            b"fremd"
        );

        let datei = tempfile::tempdir().expect("Wegwerf-Ordner");
        std::fs::write(datei.path().join(".eingang"), b"x").expect("Datei");
        assert!(matches!(
            Anhangdateien::oeffnen(datei.path()),
            Err(Eingangsfehler::KeinOrdner { .. })
        ));
        assert_eq!(
            std::fs::read(datei.path().join(".eingang")).expect("bleibt"),
            b"x"
        );
    }

    /// Nach dem Start gegen einen Symlink getauscht: die Teildatei entsteht im echten, gehaltenen Eingang.
    #[test]
    fn ein_spaeter_getauschter_eingang_aendert_nichts() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let fremd = tempfile::tempdir().expect("fremder Ordner");
        let (ablage, _) = Anhangdateien::oeffnen(wurzel.path()).expect("geöffnet");
        std::fs::rename(wurzel.path().join(".eingang"), wurzel.path().join("echt")).expect("weg");
        symlink(fremd.path(), wurzel.path().join(".eingang")).expect("Symlink");

        let mut teil = ablage.teil_anlegen(&id(1)).expect("Teil");
        teil.anhaengen(b"abc").expect("geschrieben");
        ablage.teil_abschliessen(&id(1)).expect("abgeschlossen");

        assert_eq!(std::fs::read_dir(fremd.path()).expect("fremd").count(), 0);
        assert_eq!(
            std::fs::read(wurzel.path().join("echt").join(id(1).als_text())).expect("echt"),
            b"abc"
        );
    }

    /// Binden und Zurücklegen relativ zu den gehaltenen Deskriptoren, nie überschreibend; `anhaenge` als Symlink
    /// → Fehler, die Datei bleibt im Eingang, im fremden Ordner kommt nichts an (M45′, M39).
    #[test]
    fn binden_und_zuruecklegen_ueber_deskriptoren() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        std::fs::create_dir_all(wurzel.path().join("channels/c")).expect("Channel");
        let (ablage, _) = Anhangdateien::oeffnen(wurzel.path()).expect("geöffnet");
        let c = Gespraech::channel("c");
        let tag = NaiveDate::from_ymd_opt(2026, 10, 29).expect("Tag");
        let mut teil = ablage.teil_anlegen(&id(1)).expect("Teil");
        teil.anhaengen(b"inhalt").expect("geschrieben");
        ablage.teil_abschliessen(&id(1)).expect("fertig");

        ablage
            .binden(&id(1), &c, tag, "140000000-1-x.bin")
            .expect("gebunden");
        let gebunden = wurzel
            .path()
            .join("channels/c/anhaenge/2026-10-29/140000000-1-x.bin");
        assert_eq!(std::fs::read(&gebunden).expect("da"), b"inhalt");
        assert_eq!(
            ablage.tag_dateien(&c, tag).expect("Dateien"),
            vec![Tagesdatei {
                name: "140000000-1-x.bin".into(),
                laenge: 6
            }]
        );
        assert_eq!(
            ablage
                .zum_lesen(&c, tag, "140000000-1-x.bin")
                .expect("lesbar")
                .laenge()
                .expect("Länge"),
            6
        );
        ablage
            .zuruecklegen(&c, tag, "140000000-1-x.bin", &id(1))
            .expect("zurück");
        assert!(!gebunden.exists());
        assert!(
            wurzel
                .path()
                .join(".eingang")
                .join(id(1).als_text())
                .exists()
        );

        std::fs::write(&gebunden, b"vorhanden").expect("untergeschoben");
        assert!(ablage.binden(&id(1), &c, tag, "140000000-1-x.bin").is_err());
        assert_eq!(std::fs::read(&gebunden).expect("bleibt"), b"vorhanden");

        let fremd = tempfile::tempdir().expect("fremd");
        std::fs::remove_dir_all(wurzel.path().join("channels/c/anhaenge")).expect("weg");
        symlink(fremd.path(), wurzel.path().join("channels/c/anhaenge")).expect("Symlink");
        assert!(ablage.binden(&id(1), &c, tag, "140000000-1-x.bin").is_err());
        assert_eq!(std::fs::read_dir(fremd.path()).expect("fremd").count(), 0);
        assert!(
            wurzel
                .path()
                .join(".eingang")
                .join(id(1).als_text())
                .exists(),
            "bleibt im Eingang"
        );
    }

    /// Teildatei: `O_EXCL` (zweimal anlegen scheitert), Abschluss ohne Überschreiben (M39), Löschen beider Namen.
    /// Befund #53 B1: die Tage in `anhaenge/` — Ordner und Symlinks mit Tagesnamen, aufsteigend; eine Datei mit
    /// Tagesnamen, ein anderer Name und eine andere Schreibweise zählen nicht; ohne `anhaenge` keine, als Symlink ein
    /// Fehler.
    #[test]
    fn die_tage_in_anhaenge_auch_ohne_tagesdatei() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let anhaenge = wurzel.path().join("channels/c/anhaenge");
        std::fs::create_dir_all(wurzel.path().join("channels/d")).expect("d");
        for ordner in ["2026-10-21", "2026-10-20", "notiz", "2026-1-5"] {
            std::fs::create_dir_all(anhaenge.join(ordner)).expect("Ordner");
        }
        std::fs::write(anhaenge.join("2026-10-22"), b"x").expect("Datei");
        let fremd = tempfile::tempdir().expect("fremd");
        symlink(fremd.path(), anhaenge.join("2026-10-23")).expect("Symlink");
        let (ablage, _) = Anhangdateien::oeffnen(wurzel.path()).expect("geöffnet");
        let tag = |t: &str| NaiveDate::parse_from_str(t, "%Y-%m-%d").expect("Tag");

        let tage = ablage
            .anhang_tage(&Gespraech::channel("c"))
            .expect("aufgelistet");

        assert_eq!(
            tage,
            vec![tag("2026-10-20"), tag("2026-10-21"), tag("2026-10-23")]
        );
        assert_eq!(
            ablage.anhang_tage(&Gespraech::channel("d")).expect("leer"),
            Vec::<NaiveDate>::new()
        );
        symlink(&anhaenge, wurzel.path().join("channels/d/anhaenge")).expect("Symlink");
        assert!(ablage.anhang_tage(&Gespraech::channel("d")).is_err());
    }

    #[test]
    fn teil_anlegen_abschliessen_und_loeschen_ueberschreiben_nie() {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let (ablage, _) = Anhangdateien::oeffnen(wurzel.path()).expect("geöffnet");
        let eingang = wurzel.path().join(".eingang");
        let mut teil = ablage.teil_anlegen(&id(1)).expect("Teil");
        assert!(ablage.teil_anlegen(&id(1)).is_err(), "O_EXCL");
        teil.anhaengen(b"hallo ").expect("1");
        teil.anhaengen(b"welt").expect("2");
        assert_eq!(teil.laenge().expect("Länge"), 10);
        teil.schreiben_an(0, b"H").expect("an Stelle");
        let mut puffer = [0u8; 5];
        assert_eq!(teil.lesen_an(6, &mut puffer).expect("gelesen"), 4);
        assert_eq!(&puffer[..4], b"welt");

        std::fs::write(eingang.join(id(1).als_text()), b"untergeschoben").expect("untergeschoben");
        assert!(ablage.teil_abschliessen(&id(1)).is_err());
        assert_eq!(
            std::fs::read(eingang.join(id(1).als_text())).expect("bleibt"),
            b"untergeschoben"
        );

        ablage.eingang_loeschen(&id(1)).expect("gelöscht");
        ablage.eingang_loeschen(&id(1)).expect("zweimal");
        assert_eq!(std::fs::read_dir(&eingang).expect("leer").count(), 0);

        std::fs::write(wurzel.path().join("fremd"), b"fremd").expect("fremd");
        symlink(wurzel.path().join("fremd"), eingang.join(teilname(&id(2))))
            .expect("Symlink als Teil");
        assert!(ablage.teil_anlegen(&id(2)).is_err(), "O_NOFOLLOW");
        assert_eq!(
            std::fs::read(wurzel.path().join("fremd")).expect("fremd"),
            b"fremd"
        );
    }
}
