//! Betriebsprotokoll lesen (R00014): Zeilen der Tagesdatei, Form einer Protokollzeile.

use std::path::Path;

/// Die Zeilen von `logs/<tag>.log` (leer, wenn die Datei fehlt).
pub fn zeilen(ordner: &Path, tag: &str) -> Vec<String> {
    std::fs::read_to_string(ordner.join("logs").join(format!("{tag}.log")))
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect()
}

/// Alle Zeilen aller Tagesdateien unter `logs/`, nach Datum geordnet — für Instanzen mit der echten Uhr.
pub fn alle_zeilen(ordner: &Path) -> Vec<String> {
    let mut dateien: Vec<_> = std::fs::read_dir(ordner.join("logs"))
        .expect("logs/")
        .flatten()
        .map(|eintrag| eintrag.path())
        .collect();
    dateien.sort();
    dateien
        .iter()
        .flat_map(|datei| {
            std::fs::read_to_string(datei)
                .unwrap_or_default()
                .lines()
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Zeilen, die Stufe und Bereich tragen (`" INFORMATION start "`).
pub fn mit(zeilen: &[String], stufe: &str, bereich: &str) -> Vec<String> {
    let teil = format!(" {stufe} {bereich} ");
    zeilen
        .iter()
        .filter(|zeile| zeile.contains(&teil))
        .cloned()
        .collect()
}

/// `^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d\.\d{3}[+-]\d\d:\d\d (DEBUG|INFORMATION|WARNING|ERROR|EXCEPTION) [a-zäöü]+ .+$`
pub fn ist_protokollzeile(zeile: &str) -> bool {
    let zeichen: Vec<char> = zeile.chars().collect();
    let muster = "dddd-dd-ddTdd:dd:dd.ddd*dd:dd ";
    if zeichen.len() <= muster.len() {
        return false;
    }
    let zeit_passt = muster.chars().zip(&zeichen).all(|(m, z)| match m {
        'd' => z.is_ascii_digit(),
        '*' => *z == '+' || *z == '-',
        anderes => *z == anderes,
    });
    let rest: String = zeichen[muster.len()..].iter().collect();
    let mut teile = rest.splitn(3, ' ');
    let stufe = teile.next().unwrap_or_default();
    let bereich = teile.next().unwrap_or_default();
    let text = teile.next().unwrap_or_default();
    zeit_passt
        && ["DEBUG", "INFORMATION", "WARNING", "ERROR", "EXCEPTION"].contains(&stufe)
        && !bereich.is_empty()
        && bereich
            .chars()
            .all(|z| z.is_ascii_lowercase() || "äöü".contains(z))
        && !text.is_empty()
}

/// Der Wert zu `schluessel=` in einer Zeile (ohne Anführung), falls vorhanden.
pub fn wert(zeile: &str, schluessel: &str) -> Option<String> {
    let kontext = zeile.split_once(" | ")?.1;
    let anfang = kontext
        .match_indices(&format!("{schluessel}="))
        .find(|(stelle, _)| *stelle == 0 || kontext.as_bytes()[stelle - 1] == b' ')?
        .0
        + schluessel.len()
        + 1;
    let rest = &kontext[anfang..];
    if let Some(innen) = rest.strip_prefix('"') {
        let mut wert = String::new();
        let mut zeichen = innen.chars();
        while let Some(z) = zeichen.next() {
            match z {
                '\\' => {
                    if let Some(naechstes) = zeichen.next() {
                        if naechstes != '"' {
                            wert.push('\\');
                        }
                        wert.push(naechstes);
                    }
                }
                '"' => return Some(wert),
                anderes => wert.push(anderes),
            }
        }
        Some(wert)
    } else {
        Some(rest.split(' ').next().unwrap_or_default().to_string())
    }
}

/// Der Ort, den eine `laden`-Zeile nennt: `<datei>:<zeile>` bzw. `<datei>` ohne Zeile — die Form, in der
/// ältere Kriterien Ladedefekte nennen.
pub fn ort(zeile: &str) -> String {
    let datei = wert(zeile, "datei").unwrap_or_default();
    match wert(zeile, "zeile") {
        Some(nummer) => format!("{datei}:{nummer}"),
        None => datei,
    }
}
