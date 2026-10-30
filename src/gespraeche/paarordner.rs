//! Leere Paarordner entfernen (ADR-0055 Regel 10 „Leere Paarordner entfernen“, Baustein D5 Teil 2; Verzeichnisdeskriptoren
//! wie ADR-0046 Ergänzung „Karte #9 Security-Gate“ Regel 4): ab dem gehaltenen Deskriptor des Instanzordners
//! Komponente für Komponente `openat(…, O_DIRECTORY | O_NOFOLLOW)`, nie einem Symlink folgend, und nur `rmdir`
//! (`unlinkat(…, AT_REMOVEDIR)`) — nie eine Datei, nie `remove_dir_all`. Von innen nach außen: erst die leeren
//! Unterordner (`anhaenge/`, `anhaenge/<tag>/`), zuletzt der Paarordner. Liegt irgendwo etwas anderes als ein Ordner
//! (Fremddatei, Link, Anhangdatei), ist es ein Fehler und **nichts** wird entfernt.

use super::Direktpaar;
use super::anhangpfade::ordner;
use rustix::fd::OwnedFd;
use rustix::fs::{AtFlags, Dir, FileType, Mode, OFlags};
use std::io;

/// So tief unter dem Paarordner werden Ordner höchstens geprüft (`anhaenge/<tag>` ist 2 tief).
const TIEFE: usize = 4;

/// Entfernt `direkt/<paar>/` samt seiner leeren Unterordner; liegt darin etwas anderes als Ordner, ist es ein Fehler
/// (`DirectoryNotEmpty`) und nichts ist entfernt. Fehlt der Ordner, `NotFound`.
pub fn leer_entfernen(instanz: &OwnedFd, paar: &Direktpaar) -> io::Result<()> {
    let direkt = ordner(instanz, "direkt")?;
    let name = paar.ordnername();
    let paarordner = ordner(&direkt, &name)?;
    nur_ordner(&paarordner, 0)?;
    unterordner_entfernen(&paarordner)?;
    drop(paarordner);
    rustix::fs::unlinkat(&direkt, name.as_str(), AtFlags::REMOVEDIR)?;
    Ok(())
}

/// Die rohen Namen der Einträge ohne `.` und `..`.
fn eintraege(fd: &OwnedFd) -> io::Result<Vec<std::ffi::CString>> {
    let mut namen = Vec::new();
    for eintrag in Dir::read_from(fd)? {
        let eintrag = eintrag?;
        if !matches!(eintrag.file_name().to_bytes(), b"." | b"..") {
            namen.push(eintrag.file_name().to_owned());
        }
    }
    Ok(namen)
}

fn ist_ordner(fd: &OwnedFd, name: &std::ffi::CStr) -> io::Result<bool> {
    let stand = rustix::fs::statat(fd, name, AtFlags::SYMLINK_NOFOLLOW)?;
    Ok(FileType::from_raw_mode(stand.st_mode) == FileType::Directory)
}

fn unterordner_oeffnen(fd: &OwnedFd, name: &std::ffi::CStr) -> io::Result<OwnedFd> {
    Ok(rustix::fs::openat(
        fd,
        name,
        OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::RDONLY,
        Mode::empty(),
    )?)
}

fn nicht_leer() -> io::Error {
    io::Error::new(
        io::ErrorKind::DirectoryNotEmpty,
        "im Paarordner liegt etwas anderes als leere Ordner",
    )
}

/// Erster Durchgang: nur Ordner (echte, keine Links) bis `TIEFE`; nichts wird verändert.
fn nur_ordner(fd: &OwnedFd, tiefe: usize) -> io::Result<()> {
    if tiefe > TIEFE {
        return Err(nicht_leer());
    }
    for name in eintraege(fd)? {
        if !ist_ordner(fd, &name)? {
            return Err(nicht_leer());
        }
        nur_ordner(&unterordner_oeffnen(fd, &name)?, tiefe + 1)?;
    }
    Ok(())
}

/// Zweiter Durchgang: die Unterordner von innen nach außen mit `rmdir`; schlägt eines fehl, weil sich inzwischen etwas
/// darin findet, bleibt es samt dem Paarordner stehen.
fn unterordner_entfernen(fd: &OwnedFd) -> io::Result<()> {
    for name in eintraege(fd)? {
        unterordner_entfernen(&unterordner_oeffnen(fd, &name)?)?;
        rustix::fs::unlinkat(fd, name.as_c_str(), AtFlags::REMOVEDIR)?;
    }
    Ok(())
}
