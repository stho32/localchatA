//! Rechnername lesen (B0035): der Name, unter dem sich der Rechner selbst kennt (`uname -n`).
//! Der Grenzschutz lässt ihn als Host zu (ADR-0014); ohne Unix gibt es keinen.

/// Der Rechnername oder keiner, wenn er leer oder kein UTF-8 ist.
#[cfg(unix)]
pub fn lesen() -> Option<String> {
    let uname = rustix::system::uname();
    let name = uname.nodename().to_str().ok()?;
    if name.is_empty() {
        return None;
    }
    Some(name.to_string())
}

/// Ohne Unix gelten nur IP-Literale, `localhost` und `--erreichbar-als`.
#[cfg(not(unix))]
pub fn lesen() -> Option<String> {
    None
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn der_rechnername_ist_der_von_uname_n() {
        let ausgabe = std::process::Command::new("uname")
            .arg("-n")
            .output()
            .expect("uname -n");
        let erwartet = String::from_utf8(ausgabe.stdout).expect("UTF-8");

        assert_eq!(lesen().as_deref(), Some(erwartet.trim_end()));
    }
}
