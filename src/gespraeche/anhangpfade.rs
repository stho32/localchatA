//! Anhangpfade über Verzeichnisdeskriptoren (B0524, ADR-0046 Ergänzung „Karte #9 Security-Gate“ Regeln 1–4, P2;
//! ersetzt ADR-0032 Regel 7): ab dem gehaltenen Deskriptor des Instanzordners Komponente für Komponente
//! `openat(…, O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC)`, die Datei zuletzt mit `O_NOFOLLOW` — ein Symlink an jeder
//! Stelle ergibt `ELOOP` bzw. `ENOTDIR`, nie den Weg in einen fremden Ordner. Jede Komponente kommt aus einem Typ;
//! geprüft wird trotzdem: nie leer, nie `.` oder `..`, nie mit `/`.

use super::anhangverzeichnis::Tagesdatei;
use super::logablage::Gespraech;
use chrono::NaiveDate;
use rustix::fd::OwnedFd;
use rustix::fs::{AtFlags, FileType, Mode, OFlags};
use std::fs::File;
use std::io;

const ANHAENGE: &str = "anhaenge";

/// Eine Komponente eines Anhangpfads.
fn komponente(name: &str) -> io::Result<&str> {
    if name.is_empty() || name == "." || name == ".." || name.contains('/') || name.contains('\0') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "unzulässige Pfadkomponente",
        ));
    }
    Ok(name)
}

/// Öffnet einen Ordner relativ zu `oben`, ohne einem Symlink zu folgen.
pub fn ordner(oben: &OwnedFd, name: &str) -> io::Result<OwnedFd> {
    Ok(rustix::fs::openat(
        oben,
        komponente(name)?,
        OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::RDONLY,
        Mode::empty(),
    )?)
}

/// Legt einen Ordner relativ zu `oben` an, wenn er fehlt, und öffnet ihn danach ohne Symlink.
fn ordner_anlegen(oben: &OwnedFd, name: &str) -> io::Result<OwnedFd> {
    match rustix::fs::mkdirat(oben, komponente(name)?, Mode::from_raw_mode(0o755)) {
        Ok(()) | Err(rustix::io::Errno::EXIST) => {}
        Err(fehler) => return Err(fehler.into()),
    }
    ordner(oben, name)
}

fn gespraechsteile(gespraech: &Gespraech) -> (&'static str, &str) {
    match gespraech {
        Gespraech::Channel(name) => ("channels", name.as_str()),
        Gespraech::Direkt(name) => ("direkt", name.as_str()),
    }
}

fn tagname(tag: NaiveDate) -> String {
    tag.format("%Y-%m-%d").to_string()
}

/// `<gespräch>/anhaenge` ohne Symlink in einer Komponente.
pub fn anhaenge(instanz: &OwnedFd, gespraech: &Gespraech) -> io::Result<OwnedFd> {
    let (art, name) = gespraechsteile(gespraech);
    let art = ordner(instanz, art)?;
    let gespraech = ordner(&art, name)?;
    ordner(&gespraech, ANHAENGE)
}

/// `<gespräch>/anhaenge/<tag>` ohne Symlink in einer Komponente.
pub fn tag(instanz: &OwnedFd, gespraech: &Gespraech, tag: NaiveDate) -> io::Result<OwnedFd> {
    ordner(&anhaenge(instanz, gespraech)?, &tagname(tag))
}

/// Wie `tag`, legt `anhaenge` und den Tag bei Bedarf an (Binden; der Gesprächsordner muss es geben).
pub fn tag_anlegen(
    instanz: &OwnedFd,
    gespraech: &Gespraech,
    tag: NaiveDate,
) -> io::Result<OwnedFd> {
    let (art, name) = gespraechsteile(gespraech);
    let art = ordner(instanz, art)?;
    let gespraech = ordner(&art, name)?;
    let anhaenge = ordner_anlegen(&gespraech, ANHAENGE)?;
    ordner_anlegen(&anhaenge, &tagname(tag))
}

/// Die Datei zum Lesen (Ausliefern), `O_NOFOLLOW` in jeder Komponente.
pub fn zum_lesen(
    instanz: &OwnedFd,
    gespraech: &Gespraech,
    tag_: NaiveDate,
    datei: &str,
) -> io::Result<File> {
    let tag = tag(instanz, gespraech, tag_)?;
    let fd = rustix::fs::openat(
        &tag,
        komponente(datei)?,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )?;
    Ok(File::from(fd))
}

/// Kürzt die Datei auf 0 Bytes (Löschen, ADR-0046 Ergänzung „F1′“ Regel 5 b): `O_WRONLY | O_NOFOLLOW`,
/// `ftruncate(0)`, kein `unlink`; fehlt sie (oder ihr Tag), ist das Erfolg.
pub fn kuerzen(
    instanz: &OwnedFd,
    gespraech: &Gespraech,
    tag_: NaiveDate,
    datei: &str,
) -> io::Result<()> {
    let tag = match tag(instanz, gespraech, tag_) {
        Err(fehler) if fehler.kind() == io::ErrorKind::NotFound => return Ok(()),
        anderes => anderes?,
    };
    let fd = match rustix::fs::openat(
        &tag,
        komponente(datei)?,
        OFlags::WRONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    ) {
        Err(rustix::io::Errno::NOENT) => return Ok(()),
        anderes => anderes?,
    };
    rustix::fs::ftruncate(&fd, 0)?;
    Ok(())
}

/// Die regulären Dateien eines Tags mit ihrer Länge (`fstatat(AT_SYMLINK_NOFOLLOW)`): Symlinks und Unterordner
/// zählen nicht. Fehlt der Tag (oder `anhaenge`), keine.
pub fn tag_dateien(
    instanz: &OwnedFd,
    gespraech: &Gespraech,
    tag_: NaiveDate,
) -> io::Result<Vec<Tagesdatei>> {
    let tag = match tag(instanz, gespraech, tag_) {
        Err(fehler) if fehler.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        anderes => anderes?,
    };
    let mut dateien = Vec::new();
    for eintrag in rustix::fs::Dir::read_from(&tag)? {
        let eintrag = eintrag?;
        if ist_punkt(eintrag.file_name()) {
            continue;
        }
        // Gelesen wird über den rohen Namen; ein Name ohne UTF-8 kann zu keiner Metazeile gehören und zählt mit
        // seinem verlustbehafteten Text als Waise — er macht den Tag nicht unlesbar (Befund #53 A2).
        let stand = rustix::fs::statat(&tag, eintrag.file_name(), AtFlags::SYMLINK_NOFOLLOW)?;
        if FileType::from_raw_mode(stand.st_mode).is_file() {
            dateien.push(Tagesdatei {
                name: eintrag.file_name().to_string_lossy().into_owned(),
                laenge: u64::try_from(stand.st_size).unwrap_or(0),
            });
        }
    }
    dateien.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(dateien)
}

/// Die Tage in `<gespräch>/anhaenge/` (ADR-0046 Ergänzung „Review #53 B1“ T2): echte Ordner, deren Name genau
/// `JJJJ-MM-TT` mit gültigem Datum ist, aufsteigend; Dateien, Symlinks und andere Namen gehören zu keinem Tag. Fehlt
/// `anhaenge`, keine; ein Symlink als `anhaenge` ist ein Fehler.
pub fn anhang_tage(instanz: &OwnedFd, gespraech: &Gespraech) -> io::Result<Vec<NaiveDate>> {
    let anhaenge = match anhaenge(instanz, gespraech) {
        Err(fehler) if fehler.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        anderes => anderes?,
    };
    let mut tage = Vec::new();
    for eintrag in rustix::fs::Dir::read_from(&anhaenge)? {
        let eintrag = eintrag?;
        let Some(tag) = eintrag.file_name().to_str().ok().and_then(|name| {
            NaiveDate::parse_from_str(name, "%Y-%m-%d")
                .ok()
                .filter(|tag| tagname(*tag) == name)
        }) else {
            continue;
        };
        let stand = rustix::fs::statat(&anhaenge, eintrag.file_name(), AtFlags::SYMLINK_NOFOLLOW)?;
        if FileType::from_raw_mode(stand.st_mode).is_dir() {
            tage.push(tag);
        }
    }
    tage.sort();
    Ok(tage)
}

/// Entfallen der Anhänge eines Tags (Regel 4): `anhaenge` ein Symlink → Fehler, nichts gelöscht; `anhaenge/<tag>`
/// ein Symlink → nur der Link; sonst den Tag-Ordner über seinen Deskriptor leeren (Symlinks als Einträge entfernt,
/// nie verfolgt; Unterordner rekursiv) und entfernen. Fehlt etwas, ist es schon weg.
pub fn tag_entfernen(instanz: &OwnedFd, gespraech: &Gespraech, tag_: NaiveDate) -> io::Result<()> {
    let anhaenge = match anhaenge(instanz, gespraech) {
        Err(fehler) if fehler.kind() == io::ErrorKind::NotFound => return Ok(()),
        anderes => anderes?,
    };
    let name = tagname(tag_);
    let stand = match rustix::fs::statat(&anhaenge, name.as_str(), AtFlags::SYMLINK_NOFOLLOW) {
        Err(rustix::io::Errno::NOENT) => return Ok(()),
        anderes => anderes?,
    };
    if !FileType::from_raw_mode(stand.st_mode).is_dir() {
        rustix::fs::unlinkat(&anhaenge, name.as_str(), AtFlags::empty())?;
        return Ok(());
    }
    let tag = ordner(&anhaenge, &name)?;
    leeren(&tag, 0)?;
    rustix::fs::unlinkat(&anhaenge, name.as_str(), AtFlags::REMOVEDIR)?;
    Ok(())
}

/// Höchstens so tief werden Unterordner eines Tags geleert.
const TIEFE: usize = 16;

fn leeren(ordner_fd: &OwnedFd, tiefe: usize) -> io::Result<()> {
    if tiefe > TIEFE {
        return Err(io::Error::other("Unterordner zu tief verschachtelt"));
    }
    let mut namen = Vec::new();
    for eintrag in rustix::fs::Dir::read_from(ordner_fd)? {
        let eintrag = eintrag?;
        if !ist_punkt(eintrag.file_name()) {
            namen.push(eintrag.file_name().to_owned());
        }
    }
    // Rohe Namen: auch ein Eintrag ohne UTF-8 im Namen wird entfernt (Befund #53 A2).
    for name in namen {
        let stand = rustix::fs::statat(ordner_fd, name.as_c_str(), AtFlags::SYMLINK_NOFOLLOW)?;
        if FileType::from_raw_mode(stand.st_mode).is_dir() {
            let unter = rustix::fs::openat(
                ordner_fd,
                name.as_c_str(),
                OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::RDONLY,
                Mode::empty(),
            )?;
            leeren(&unter, tiefe + 1)?;
            rustix::fs::unlinkat(ordner_fd, name.as_c_str(), AtFlags::REMOVEDIR)?;
        } else {
            rustix::fs::unlinkat(ordner_fd, name.as_c_str(), AtFlags::empty())?;
        }
    }
    Ok(())
}

/// `.` oder `..` eines Verzeichnisses.
fn ist_punkt(name: &std::ffi::CStr) -> bool {
    matches!(name.to_bytes(), b"." | b"..")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;
    use std::path::Path;

    fn tag_29() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 29).expect("Tag")
    }

    fn c() -> Gespraech {
        Gespraech::channel("c")
    }

    fn instanz(ordner: &Path) -> OwnedFd {
        rustix::fs::open(
            ordner,
            OFlags::DIRECTORY | OFlags::CLOEXEC | OFlags::RDONLY,
            Mode::empty(),
        )
        .expect("Ordner")
    }

    /// Ordner mit `channels/c/anhaenge/2026-10-29/x.bin` (6 Bytes) und ein fremder Ordner mit derselben Struktur.
    fn aufbau() -> (tempfile::TempDir, tempfile::TempDir) {
        let wurzel = tempfile::tempdir().expect("Wegwerf-Ordner");
        let fremd = tempfile::tempdir().expect("fremder Ordner");
        for basis in [wurzel.path(), fremd.path()] {
            std::fs::create_dir_all(basis.join("channels/c/anhaenge/2026-10-29")).expect("Ordner");
        }
        std::fs::write(
            wurzel.path().join("channels/c/anhaenge/2026-10-29/x.bin"),
            b"eigene",
        )
        .expect("eigen");
        std::fs::write(
            fremd.path().join("channels/c/anhaenge/2026-10-29/x.bin"),
            b"fremde",
        )
        .expect("fremd");
        (wurzel, fremd)
    }

    fn fremd_unveraendert(fremd: &Path) {
        assert_eq!(
            std::fs::read(fremd.join("channels/c/anhaenge/2026-10-29/x.bin")).expect("fremd"),
            b"fremde"
        );
        assert_eq!(
            std::fs::read_dir(fremd.join("channels/c/anhaenge/2026-10-29"))
                .expect("Ordner")
                .count(),
            1
        );
    }

    /// Hauptweg: lesen, Länge, kürzen (0 Bytes, nicht gelöscht), Tag entfernen.
    #[test]
    fn ohne_symlinks_lesen_kuerzen_und_entfernen() {
        let (wurzel, _) = aufbau();
        let fd = instanz(wurzel.path());
        let mut inhalt = String::new();
        std::io::Read::read_to_string(
            &mut zum_lesen(&fd, &c(), tag_29(), "x.bin").expect("lesbar"),
            &mut inhalt,
        )
        .expect("gelesen");
        assert_eq!(inhalt, "eigene");
        assert_eq!(
            tag_dateien(&fd, &c(), tag_29()).expect("Dateien"),
            vec![Tagesdatei {
                name: "x.bin".into(),
                laenge: 6
            }]
        );
        kuerzen(&fd, &c(), tag_29(), "x.bin").expect("gekürzt");
        assert_eq!(
            std::fs::metadata(wurzel.path().join("channels/c/anhaenge/2026-10-29/x.bin"))
                .expect("da")
                .len(),
            0
        );
        kuerzen(&fd, &c(), tag_29(), "fehlt.bin").expect("fehlt ist Erfolg");
        let anderer = NaiveDate::from_ymd_opt(2026, 10, 1).expect("Tag");
        kuerzen(&fd, &c(), anderer, "x.bin").expect("Tag fehlt ist Erfolg");
        assert!(tag_dateien(&fd, &c(), anderer).expect("leer").is_empty());
        tag_entfernen(&fd, &c(), tag_29()).expect("entfernt");
        assert!(
            !wurzel
                .path()
                .join("channels/c/anhaenge/2026-10-29")
                .exists()
        );
        tag_entfernen(&fd, &c(), tag_29()).expect("zweimal");
    }

    /// B0524 Unit (Kernel, keine Attrappe): je Komponente `channels/c`, `channels/c/anhaenge`, `anhaenge/<tag>` und
    /// die Datei durch einen Symlink auf den fremden Ordner mit gleichnamigem Inhalt gleicher Länge ersetzt → Kürzen,
    /// Lesen und Binden scheitern; die fremde Datei ist byte-gleich, im fremden Ordner ist nichts hinzugekommen
    /// (Mutante „nur `O_NOFOLLOW` auf dem vollen Pfad“).
    #[test]
    fn ein_symlink_an_jeder_stelle_fuehrt_nie_in_den_fremden_ordner() {
        for stelle in [
            "channels/c",
            "channels/c/anhaenge",
            "channels/c/anhaenge/2026-10-29",
            "channels/c/anhaenge/2026-10-29/x.bin",
        ] {
            let (wurzel, fremd) = aufbau();
            let eigen = wurzel.path().join(stelle);
            if eigen.is_dir() {
                std::fs::remove_dir_all(&eigen).expect("weg");
            } else {
                std::fs::remove_file(&eigen).expect("weg");
            }
            symlink(fremd.path().join(stelle), &eigen).expect("Symlink");
            let fd = instanz(wurzel.path());

            assert!(kuerzen(&fd, &c(), tag_29(), "x.bin").is_err(), "{stelle}");
            assert!(zum_lesen(&fd, &c(), tag_29(), "x.bin").is_err(), "{stelle}");
            let angelegt = tag_anlegen(&fd, &c(), tag_29());
            if stelle.ends_with("x.bin") {
                assert!(angelegt.is_ok(), "{stelle}: der Tag selbst ist echt");
                assert!(
                    tag_dateien(&fd, &c(), tag_29())
                        .expect("Dateien")
                        .is_empty(),
                    "Symlink zählt nicht"
                );
            } else {
                assert!(angelegt.is_err(), "{stelle}");
                assert!(tag_dateien(&fd, &c(), tag_29()).is_err(), "{stelle}");
            }
            fremd_unveraendert(fremd.path());
        }
    }

    /// Regel 4: `anhaenge` als Symlink → Fehler, fremder Ordner unverändert; `anhaenge/<tag>` als Symlink → nur der
    /// Link weg; ein Symlink als Eintrag im Tag → entfernt, sein Ziel bleibt (Mutante „`remove_dir_all` auf dem vollen
    /// Pfad“).
    #[test]
    fn tag_entfernen_folgt_nie_einem_symlink() {
        let (wurzel, fremd) = aufbau();
        std::fs::remove_dir_all(wurzel.path().join("channels/c/anhaenge")).expect("weg");
        symlink(
            fremd.path().join("channels/c/anhaenge"),
            wurzel.path().join("channels/c/anhaenge"),
        )
        .expect("Symlink");
        assert!(tag_entfernen(&instanz(wurzel.path()), &c(), tag_29()).is_err());
        fremd_unveraendert(fremd.path());

        let (wurzel, fremd) = aufbau();
        let tag_pfad = wurzel.path().join("channels/c/anhaenge/2026-10-29");
        std::fs::remove_dir_all(&tag_pfad).expect("weg");
        symlink(
            fremd.path().join("channels/c/anhaenge/2026-10-29"),
            &tag_pfad,
        )
        .expect("Symlink");
        tag_entfernen(&instanz(wurzel.path()), &c(), tag_29()).expect("nur der Link");
        assert!(std::fs::symlink_metadata(&tag_pfad).is_err());
        fremd_unveraendert(fremd.path());

        let (wurzel, fremd) = aufbau();
        let tag_pfad = wurzel.path().join("channels/c/anhaenge/2026-10-29");
        symlink(
            fremd.path().join("channels/c/anhaenge/2026-10-29/x.bin"),
            tag_pfad.join("zeiger"),
        )
        .expect("Symlink");
        std::fs::create_dir_all(tag_pfad.join("unter/tiefer")).expect("Unterordner");
        std::fs::write(tag_pfad.join("unter/tiefer/y"), b"y").expect("y");
        tag_entfernen(&instanz(wurzel.path()), &c(), tag_29()).expect("geleert");
        assert!(!tag_pfad.exists());
        fremd_unveraendert(fremd.path());
    }

    #[test]
    fn unzulaessige_komponenten_werden_abgewiesen() {
        let (wurzel, _) = aufbau();
        let fd = instanz(wurzel.path());
        for name in ["", ".", "..", "a/b"] {
            assert!(zum_lesen(&fd, &c(), tag_29(), name).is_err(), "{name:?}");
        }
        assert!(zum_lesen(&fd, &Gespraech::channel(".."), tag_29(), "x.bin").is_err());
    }
}
