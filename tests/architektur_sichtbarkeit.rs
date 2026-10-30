//! R00050 F0176 B0781 — Architekturtest Sichtbarkeit (ADR-0055 Regel 5 Satz „Genau eine Funktion“, B5, Baustein D14;
//! Matrix S17, Zeile U1 für künftige Wege): eine Textprüfung über `src/**/*.rs` im normalen `cargo test`.
//!
//! Vier Prüfpunkte:
//! 1. `Gespraech::Direkt` kommt in Quelltext (ohne Tests, Kommentare, Zeichenketten) nur in Dateien von
//!    [`DIREKT_ERLAUBT`] vor, je Zeile mit Begründung;
//! 2. jede Datei, die über alle Gespräche läuft (`verlaeufe` durchlaufen, `.gespraeche()` der Ablage), ruft
//!    `sichtbar_fuer(` auf oder steht mit Stellenzahl und Begründung in [`ALLE_OHNE_PRUEFUNG`];
//! 3. kein zweiter Name der Entscheidung: kein `fn sichtbar`, `fn ist_sichtbar…`, `fn darf_sehen…`, keine Funktion mit
//!    `beteiligt` im Namen außerhalb von Sichtbarkeit und Paar, kein `beteiligt(`-Aufruf dort;
//! 4. kein `_ =>` in einem `match` über `Gespraech` — nirgends, also auch nicht in Strom, Warten und Anhänge löschen.
//!
//! Eine neue Datei mit `Gespraech::Direkt` oder eine neue Iteration über alle Gespräche macht den Test rot und zwingt
//! zu einer Zeile in einer der Listen — die sieht das Review. Die Prüfung arbeitet auf `Quelle`n im Speicher; die
//! Mutantentests unten ändern Kopien, nie das Arbeitsverzeichnis.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

/// Dateien, in denen `Gespraech::Direkt` im Nicht-Test-Code vorkommen darf — mit Grund. Neue Zeile = Entscheidung des
/// Reviews: warum entscheidet diese Datei nicht über Sichtbarkeit, sondern bildet, benennt oder zählt nur?
const DIREKT_ERLAUBT: &[(&str, &str)] = &[
    (
        "src/gespraeche/anhangpfade.rs",
        "Ablageordner der Anhänge: benennt den Ordner direkt/<paar>/, entscheidet nichts",
    ),
    (
        "src/gespraeche/direktgespraeche.rs",
        "Verzeichnis der Direktgespräche: Anlegen, Bestehen, Zählen; die Liste fragt sichtbar_fuer",
    ),
    (
        "src/gespraeche/logablage.rs",
        "Ablage: Ordnername direkt/<paar>/, Laden und Anlegen des Ordners (D5, Paar beim Laden geprüft)",
    ),
    (
        "src/gespraeche/mod.rs",
        "Kennzahlen: Direktgespräche getrennt zählen (Health zeigt nur Zähler, ADR-0055 O2)",
    ),
    (
        "src/gespraeche/nachholen.rs",
        "Nachholen und sichtbare Marke: Stränge nur für Gespräche, die sichtbar_fuer freigibt; Direktsätze getrennt gezählt (Regel 9)",
    ),
    (
        "src/gespraeche/treffer.rs",
        "Trefferregel: fragt zuerst sichtbar_fuer; Direkt nur für den Anlass direkt und den Filter direkt",
    ),
    (
        "src/gespraeche/pflege.rs",
        "Pflege nach fester Frist (7 Tage) und Aufräumen leerer Paarordner: gibt keinem Aufrufer etwas aus, der Bericht nennt das Paar nur fürs Betriebsprotokoll",
    ),
    (
        "src/gespraeche/sichtbarkeit.rs",
        "die eine Entscheidung selbst (A00020)",
    ),
    (
        "src/http/aktivitaet.rs",
        "Aktivität: Gespräch im Pfad bilden, das Paar steht aus Sitzung und Pfad fest (ADR-0051 Regel 10)",
    ),
    (
        "src/http/ereignisse.rs",
        "Strom: Ort des Ereignisses (Gegenüber), nachdem sichtbar_fuer entschieden hat",
    ),
    (
        "src/http/warten.rs",
        "Warten: Feld `channel` der Antwort, Direktgespräche haben keinen Channelnamen",
    ),
    (
        "src/http/ziel.rs",
        "Auflösung „Gespräch im Pfad“ (A00004): Paar aus Sitzung und Pfad, kein fremdes Paar nennbar",
    ),
];

/// Dateien, die über alle Gespräche laufen, ohne `sichtbar_fuer(` zu fragen — mit der Zahl ihrer Durchlauf-Stellen
/// (jede weitere ist rot) und Grund. Was einem Aufrufer etwas ausgibt, gehört nicht hierher, sondern fragt `sichtbar_fuer`.
const ALLE_OHNE_PRUEFUNG: &[(&str, usize, &str)] = &[
    (
        "src/gespraeche/anhaenge.rs",
        2,
        "Abgleich beim Start und Tilgen gelöschter Nachrichten: liest Dateien, gibt keinem Aufrufer etwas aus",
    ),
    (
        "src/gespraeche/pflege.rs",
        1,
        "Pflege beim Start und Tageswechsel: Tage jenseits der Frist entfernen und leere Paarordner aufräumen, gibt keinem Aufrufer etwas aus (Bericht nur ans Betriebsprotokoll)",
    ),
    (
        "src/gespraeche/mod.rs",
        2,
        "Laden beim Start (`.gespraeche()` der Ablage) und Kennzahlen (nur Zähler, getrennt nach Art)",
    ),
];

/// Dateien, in denen `beteiligt(` stehen darf: die Entscheidung selbst und der Aufbau des Paars.
const BETEILIGT_ERLAUBT: &[&str] = &[
    "src/gespraeche/sichtbarkeit.rs",
    "src/gespraeche/direktpaar.rs",
];

/// Die Datei der einen Entscheidung: nur hier darf `fn sichtbar_fuer` stehen.
const SICHTBARKEIT: &str = "src/gespraeche/sichtbarkeit.rs";

/// Durchlauf aller Gespräche, als Zeichenfolge im Quelltext ohne Leerraum (ein Aufruf über mehrere Zeilen zählt).
const DURCHLAUF: &[&str] = &[
    "verlaeufe.iter(",
    "verlaeufe.iter_mut(",
    "verlaeufe.values(",
    "verlaeufe.values_mut(",
    "verlaeufe.keys(",
    "verlaeufe.into_iter(",
    "verlaeufe.retain(",
    "verlaeufe.drain(",
    "verlaeufe{",
    ".gespraeche()",
];

/// Ein Quelltext mit seinem Pfad relativ zum Projekt (`src/…`, Schrägstriche).
#[derive(Clone)]
struct Quelle {
    pfad: String,
    text: String,
}

impl Quelle {
    fn neu(pfad: &str, text: &str) -> Quelle {
        Quelle {
            pfad: pfad.to_string(),
            text: text.to_string(),
        }
    }
}

fn quellen_lesen() -> Vec<Quelle> {
    let wurzel = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut gefunden = Vec::new();
    sammeln(wurzel, &wurzel.join("src"), &mut gefunden);
    gefunden.sort_by(|a, b| a.pfad.cmp(&b.pfad));
    gefunden
}

fn sammeln(wurzel: &Path, ordner: &Path, gefunden: &mut Vec<Quelle>) {
    for eintrag in fs::read_dir(ordner).expect("Ordner lesbar") {
        let pfad = eintrag.expect("Eintrag").path();
        if pfad.is_dir() {
            sammeln(wurzel, &pfad, gefunden);
        } else if pfad.extension().is_some_and(|endung| endung == "rs") {
            let relativ = pfad.strip_prefix(wurzel).expect("unter dem Projekt");
            gefunden.push(Quelle {
                pfad: relativ.to_string_lossy().replace('\\', "/"),
                text: fs::read_to_string(&pfad).expect("Quelle lesbar"),
            });
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Aufbereitung: Nicht-Test-Code ohne Kommentare und Zeichenketteninhalt

/// Pfadpräfixe der Module, die ihr Elternmodul als `#[cfg(test)] mod x;` einbindet — samt ihrer Untermodule.
fn test_module(quellen: &[Quelle]) -> Vec<String> {
    let mut praefixe = Vec::new();
    for quelle in quellen {
        let zeilen: Vec<&str> = quelle.text.lines().collect();
        for paar in zeilen.windows(2) {
            let Some(name) = modulname(paar[1], ';') else {
                continue;
            };
            if paar[0].trim() == "#[cfg(test)]" {
                let ordner = modulordner(&quelle.pfad);
                praefixe.push(format!("{ordner}/{name}.rs"));
                praefixe.push(format!("{ordner}/{name}/"));
            }
        }
    }
    praefixe
}

/// Der Name eines Moduls aus einer Zeile `[pub[(…)]] mod name<ende>` (`;` für eine Datei, `{` für ein Inline-Modul).
fn modulname(zeile: &str, ende: char) -> Option<&str> {
    let zeile = zeile.trim();
    let zeile = match zeile.strip_prefix("pub") {
        Some(rest) => rest.trim_start_matches(|c| c != ' ').trim_start(),
        None => zeile,
    };
    let name = zeile.strip_prefix("mod ")?.strip_suffix(ende)?.trim();
    (!name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_')).then_some(name)
}

/// Ordner, in dem die Untermodule einer Datei liegen: `a/b/mod.rs` → `a/b`, `a/b.rs` → `a/b`, `src/lib.rs` → `src`.
fn modulordner(pfad: &str) -> String {
    let (ordner, datei) = pfad.rsplit_once('/').expect("Pfad mit Ordner");
    match datei {
        "mod.rs" | "lib.rs" | "main.rs" => ordner.to_string(),
        _ => format!("{ordner}/{}", datei.trim_end_matches(".rs")),
    }
}

/// Der Quelltext ohne Testmodul am Dateiende (`#[cfg(test)]` + `mod …`), ohne Kommentare und mit leeren Zeichenketten.
fn produktiv(quelle: &Quelle) -> String {
    let zeilen: Vec<&str> = quelle.text.lines().collect();
    let ende = zeilen
        .windows(2)
        .position(|paar| paar[0] == "#[cfg(test)]" && modulname(paar[1], '{').is_some())
        .unwrap_or(zeilen.len());
    ohne_kommentare_und_zeichenketten(&zeilen[..ende].join("\n"))
}

fn ohne_kommentare_und_zeichenketten(text: &str) -> String {
    let zeichen: Vec<char> = text.chars().collect();
    let mut aus = String::with_capacity(text.len());
    let mut i = 0;
    while i < zeichen.len() {
        let c = zeichen[i];
        let weiter = zeichen.get(i + 1).copied();
        match (c, weiter) {
            ('/', Some('/')) => {
                while i < zeichen.len() && zeichen[i] != '\n' {
                    i += 1;
                }
            }
            ('/', Some('*')) => i = block_kommentar_ende(&zeichen, i),
            ('"', _) => {
                aus.push_str("\"\"");
                i = zeichenkette_ende(&zeichen, i + 1);
            }
            ('r', Some('"' | '#')) if roh_beginn(&zeichen, i) => {
                aus.push_str("\"\"");
                i = rohkette_ende(&zeichen, i);
            }
            ('\'', _) if zeichen_literal(&zeichen, i).is_some() => {
                aus.push_str("' '");
                i = zeichen_literal(&zeichen, i).expect("Literal");
            }
            _ => {
                aus.push(c);
                i += 1;
            }
        }
    }
    aus
}

fn block_kommentar_ende(zeichen: &[char], von: usize) -> usize {
    let (mut i, mut tiefe) = (von, 0);
    while i < zeichen.len() {
        match (zeichen[i], zeichen.get(i + 1)) {
            ('/', Some('*')) => {
                tiefe += 1;
                i += 2;
            }
            ('*', Some('/')) => {
                tiefe -= 1;
                i += 2;
                if tiefe == 0 {
                    return i;
                }
            }
            _ => i += 1,
        }
    }
    i
}

/// Hinter dem schließenden `"` (Beginn hinter dem öffnenden).
fn zeichenkette_ende(zeichen: &[char], von: usize) -> usize {
    let mut i = von;
    while i < zeichen.len() {
        match zeichen[i] {
            '\\' => i += 2,
            '"' => return i + 1,
            _ => i += 1,
        }
    }
    i
}

fn roh_beginn(zeichen: &[char], von: usize) -> bool {
    let mut i = von + 1;
    while zeichen.get(i) == Some(&'#') {
        i += 1;
    }
    zeichen.get(i) == Some(&'"')
}

fn rohkette_ende(zeichen: &[char], von: usize) -> usize {
    let mut i = von + 1;
    let mut rauten = 0;
    while zeichen[i] == '#' {
        rauten += 1;
        i += 1;
    }
    i += 1;
    while i < zeichen.len() {
        if zeichen[i] == '"' && zeichen[i + 1..].iter().take(rauten).all(|c| *c == '#') {
            return i + 1 + rauten;
        }
        i += 1;
    }
    i
}

/// Ende eines Zeichenliterals (`'x'`, `'\n'`, `'\u{1F600}'`), `None` bei einer Lebensdauer (`'a`).
fn zeichen_literal(zeichen: &[char], von: usize) -> Option<usize> {
    match (zeichen.get(von + 1)?, zeichen.get(von + 2)?) {
        ('\\', _) => {
            let ende = zeichen[von + 2..].iter().position(|c| *c == '\'')?;
            (ende <= 10).then_some(von + 2 + ende + 1)
        }
        (_, '\'') => Some(von + 3),
        _ => None,
    }
}

fn kompakt(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect()
}

// ---------------------------------------------------------------------------------------------------------------
// Die vier Prüfpunkte

/// Alle Befunde über `quellen`; leer heißt: die Regel hält.
fn pruefen(quellen: &[Quelle]) -> Vec<String> {
    let ausgenommen = test_module(quellen);
    let mut befunde = Vec::new();
    let mut direkt_gesehen = BTreeSet::new();
    let mut durchlauf_gesehen = BTreeSet::new();
    for quelle in quellen {
        if ausgenommen
            .iter()
            .any(|p| quelle.pfad == *p || quelle.pfad.starts_with(p))
        {
            continue;
        }
        let text = produktiv(quelle);
        direkt_pruefen(quelle, &text, &mut direkt_gesehen, &mut befunde);
        durchlauf_pruefen(quelle, &text, &mut durchlauf_gesehen, &mut befunde);
        namen_pruefen(quelle, &text, &mut befunde);
        wildcard_pruefen(quelle, &text, &mut befunde);
    }
    ueberfluessige_zeilen(quellen, &direkt_gesehen, &durchlauf_gesehen, &mut befunde);
    befunde
}

fn direkt_pruefen(
    quelle: &Quelle,
    text: &str,
    gesehen: &mut BTreeSet<String>,
    befunde: &mut Vec<String>,
) {
    if !kompakt(text).contains("Gespraech::Direkt") {
        return;
    }
    gesehen.insert(quelle.pfad.clone());
    if !DIREKT_ERLAUBT.iter().any(|(pfad, _)| *pfad == quelle.pfad) {
        befunde.push(format!(
            "1: {} nennt Gespraech::Direkt und steht nicht in DIREKT_ERLAUBT (Zeile mit Begründung ergänzen oder die \
             Entscheidung an sichtbar_fuer abgeben)",
            quelle.pfad
        ));
    }
}

fn durchlauf_pruefen(
    quelle: &Quelle,
    text: &str,
    gesehen: &mut BTreeSet<String>,
    befunde: &mut Vec<String>,
) {
    let kompakt = kompakt(text);
    let stellen: usize = DURCHLAUF
        .iter()
        .map(|muster| kompakt.matches(muster).count())
        .sum();
    if stellen == 0 {
        return;
    }
    let fragt = kompakt.contains("sichtbar_fuer(");
    let erlaubt = ALLE_OHNE_PRUEFUNG
        .iter()
        .find(|(pfad, _, _)| *pfad == quelle.pfad);
    if erlaubt.is_some() {
        gesehen.insert(quelle.pfad.clone());
    }
    match (fragt, erlaubt) {
        (true, _) => {}
        (false, None) => befunde.push(format!(
            "2: {} läuft über alle Gespräche ({stellen} Stelle(n)) und ruft sichtbar_fuer( nicht auf (aufrufen oder \
             in ALLE_OHNE_PRUEFUNG mit Grund eintragen)",
            quelle.pfad
        )),
        (false, Some((_, erwartet, _))) if *erwartet != stellen => befunde.push(format!(
            "2: {} hat {stellen} Durchlauf-Stelle(n) über alle Gespräche, ALLE_OHNE_PRUEFUNG erwartet {erwartet}",
            quelle.pfad
        )),
        (false, Some(_)) => {}
    }
}

fn namen_pruefen(quelle: &Quelle, text: &str, befunde: &mut Vec<String>) {
    for name in funktionsnamen(text) {
        let zweiter_name = name == "sichtbar"
            || name.starts_with("ist_sichtbar")
            || name.starts_with("sichtbar_")
            || name.starts_with("darf_sehen")
            || name.starts_with("darf_lesen")
            || name.starts_with("kann_sehen")
            || name.starts_with("sieht");
        let beteiligt =
            name.contains("beteiligt") && !BETEILIGT_ERLAUBT.contains(&quelle.pfad.as_str());
        let doppelt = name == "sichtbar_fuer" && quelle.pfad != SICHTBARKEIT;
        if zweiter_name && quelle.pfad != SICHTBARKEIT || beteiligt || doppelt {
            befunde.push(format!(
                "3: {} definiert `fn {name}` — zweiter Name der Entscheidung; es gibt nur sichtbar_fuer in {SICHTBARKEIT}",
                quelle.pfad
            ));
        }
    }
    let ruft = kompakt(text).contains("beteiligt(");
    if ruft && !BETEILIGT_ERLAUBT.contains(&quelle.pfad.as_str()) {
        befunde.push(format!(
            "3: {} ruft beteiligt( auf — außerhalb von Sichtbarkeit und Paar entscheidet nur sichtbar_fuer",
            quelle.pfad
        ));
    }
}

fn funktionsnamen(text: &str) -> Vec<String> {
    let mut namen = Vec::new();
    let mut rest = text;
    while let Some(stelle) = rest.find("fn ") {
        let davor_ok = rest[..stelle]
            .chars()
            .next_back()
            .is_none_or(|c| !c.is_alphanumeric() && c != '_');
        let hinter = &rest[stelle + 3..];
        if davor_ok {
            let name: String = hinter
                .trim_start()
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                namen.push(name);
            }
        }
        rest = hinter;
    }
    namen
}

/// Jeder `match`, dessen Arme `Gespraech::…` nennen oder dessen Prüfwert `gespraech` heißt, hat keinen Arm `_ =>`.
fn wildcard_pruefen(quelle: &Quelle, text: &str, befunde: &mut Vec<String>) {
    let zeichen: Vec<char> = text.chars().collect();
    for beginn in wortstellen(&zeichen, "match") {
        let Some((kopf, koerper)) = match_teile(&zeichen, beginn + "match".len()) else {
            continue;
        };
        let ueber_gespraech = kopf.contains("gespraech") && !kopf.contains("(")
            || arme_oberste_ebene(&koerper)
                .iter()
                .any(|arm| arm.contains("Gespraech::"));
        if ueber_gespraech
            && arme_oberste_ebene(&koerper)
                .iter()
                .any(|arm| ist_wildcard_arm(arm))
        {
            let zeile = zeichen[..beginn].iter().filter(|c| **c == '\n').count() + 1;
            befunde.push(format!(
                "4: {}:{zeile} — `_ =>` in einem match über Gespraech; jede Gesprächsart ausdrücklich nennen, damit eine \
                 neue nicht still durchgeht",
                quelle.pfad
            ));
        }
    }
}

/// Ein Arm `_ =>`, `… | _ =>` oder `_ if … =>`: das Muster, nach dem letzten Komma (Rest eines Vorgängers), ist ein
/// bloßes `_`.
fn ist_wildcard_arm(arm: &str) -> bool {
    let muster = arm.trim().trim_end_matches("=>").trim();
    let muster = muster.rsplit(',').next().unwrap_or("").trim();
    muster == "_" || muster.starts_with("_ if ") || muster.ends_with("| _")
}

fn wortstellen(zeichen: &[char], wort: &str) -> Vec<usize> {
    let wort: Vec<char> = wort.chars().collect();
    (0..zeichen.len().saturating_sub(wort.len()))
        .filter(|&i| zeichen[i..i + wort.len()] == wort[..])
        .filter(|&i| i == 0 || !(zeichen[i - 1].is_alphanumeric() || zeichen[i - 1] == '_'))
        .filter(|&i| {
            zeichen
                .get(i + wort.len())
                .is_some_and(|c| c.is_whitespace() || *c == '(')
        })
        .collect()
}

/// Prüfwert (Text bis zur öffnenden `{` auf Klammertiefe 0) und Körper (zwischen den passenden Klammern).
fn match_teile(zeichen: &[char], von: usize) -> Option<(String, String)> {
    let (mut i, mut runde) = (von, 0_i32);
    while i < zeichen.len() {
        match zeichen[i] {
            '(' | '[' => runde += 1,
            ')' | ']' => runde -= 1,
            '{' if runde == 0 => break,
            _ => {}
        }
        i += 1;
    }
    let kopf: String = zeichen[von..i.min(zeichen.len())].iter().collect();
    let (mut tiefe, mut j) = (0_i32, i);
    while j < zeichen.len() {
        match zeichen[j] {
            '{' => tiefe += 1,
            '}' => {
                tiefe -= 1;
                if tiefe == 0 {
                    return Some((kopf, zeichen[i + 1..j].iter().collect()));
                }
            }
            _ => {}
        }
        j += 1;
    }
    None
}

/// Die Teile des Körpers, die auf Tiefe 0 (nicht in Armkörpern, Klammern) beginnen: je Arm sein Muster samt `=>`.
fn arme_oberste_ebene(koerper: &str) -> Vec<String> {
    let (mut tiefe, mut aktuell, mut arme) = (0_i32, String::new(), Vec::new());
    let mut im_muster = true;
    let zeichen: Vec<char> = koerper.chars().collect();
    for (i, c) in zeichen.iter().enumerate() {
        match c {
            '{' | '(' | '[' => tiefe += 1,
            '}' | ')' | ']' => tiefe -= 1,
            _ => {}
        }
        if tiefe == 0 && im_muster {
            aktuell.push(*c);
            if *c == '>' && i > 0 && zeichen[i - 1] == '=' {
                arme.push(std::mem::take(&mut aktuell));
                im_muster = false;
            }
        } else if tiefe == 0 && (*c == ',' || *c == '}') {
            im_muster = true;
        }
    }
    arme
}

fn ueberfluessige_zeilen(
    quellen: &[Quelle],
    direkt_gesehen: &BTreeSet<String>,
    durchlauf_gesehen: &BTreeSet<String>,
    befunde: &mut Vec<String>,
) {
    for (pfad, grund) in DIREKT_ERLAUBT {
        if grund.trim().is_empty() {
            befunde.push(format!("1: DIREKT_ERLAUBT-Zeile {pfad} ohne Begründung"));
        }
        if quellen.iter().any(|q| q.pfad == *pfad) && !direkt_gesehen.contains(*pfad) {
            befunde.push(format!("1: DIREKT_ERLAUBT-Zeile {pfad} ist überflüssig (Datei nennt Gespraech::Direkt nicht mehr)"));
        }
    }
    for (pfad, _, grund) in ALLE_OHNE_PRUEFUNG {
        if grund.trim().is_empty() {
            befunde.push(format!(
                "2: ALLE_OHNE_PRUEFUNG-Zeile {pfad} ohne Begründung"
            ));
        }
        if quellen.iter().any(|q| q.pfad == *pfad) && !durchlauf_gesehen.contains(*pfad) {
            befunde.push(format!(
                "2: ALLE_OHNE_PRUEFUNG-Zeile {pfad} ist überflüssig (kein Durchlauf mehr)"
            ));
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Tests

fn echt() -> Vec<Quelle> {
    quellen_lesen()
}

fn mit_geaendert(quellen: &[Quelle], pfad: &str, aendern: impl Fn(&str) -> String) -> Vec<Quelle> {
    let mut kopie = quellen.to_vec();
    let stelle = kopie
        .iter_mut()
        .find(|q| q.pfad == pfad)
        .unwrap_or_else(|| panic!("{pfad} fehlt"));
    let neu = aendern(&stelle.text);
    assert_ne!(neu, stelle.text, "die Mutation hat {pfad} nicht verändert");
    stelle.text = neu;
    kopie
}

fn befunde_mit(befunde: &[String], teil: &str) -> bool {
    befunde.iter().any(|b| b.contains(teil))
}

/// Erster Test (B0781, wörtlich die Regel): das echte `src/` hält alle vier Prüfpunkte.
#[test]
fn der_quelltext_haelt_genau_eine_funktion_fuer_sichtbarkeit() {
    let befunde = pruefen(&echt());
    assert!(befunde.is_empty(), "Befunde:\n{}", befunde.join("\n"));
}

/// Mutant 1: ein neues Modul `suche.rs`, das `Gespraech::Direkt` nennt, ohne Listeneintrag.
#[test]
fn ein_neues_modul_mit_direkt_ohne_listeneintrag_ist_rot() {
    let mut quellen = echt();
    quellen.push(Quelle::neu(
        "src/gespraeche/suche.rs",
        "pub fn treffer(g: &Gespraech) -> bool {\n    matches!(g, Gespraech::Direkt(_))\n}\n",
    ));
    let befunde = pruefen(&quellen);
    assert!(
        befunde_mit(&befunde, "1: src/gespraeche/suche.rs"),
        "{befunde:?}"
    );
}

/// Mutant 2: eine neue Iteration über alle Gespräche ohne `sichtbar_fuer(`.
#[test]
fn eine_neue_iteration_ueber_alle_gespraeche_ohne_sichtbar_fuer_ist_rot() {
    let mut quellen = echt();
    quellen.push(Quelle::neu(
        "src/gespraeche/laufwerk.rs",
        "pub fn liste(v: &Verzeichnis) -> Vec<Zeit> {\n    v.verlaeufe\n        .values()\n        .map(|l| l.zeit())\n        .collect()\n}\n",
    ));
    let befunde = pruefen(&quellen);
    assert!(
        befunde_mit(&befunde, "2: src/gespraeche/laufwerk.rs"),
        "{befunde:?}"
    );
}

/// Mutant 2b: der echte Weg `nachholen.rs` verliert seine Frage — die Datei läuft weiter über alle Gespräche.
#[test]
fn ein_echter_weg_ohne_sichtbar_fuer_ist_rot() {
    let quellen = mit_geaendert(&echt(), "src/gespraeche/nachholen.rs", |text| {
        text.replace("sichtbar_fuer(", "ist_erlaubt(")
    });
    let befunde = pruefen(&quellen);
    assert!(
        befunde_mit(&befunde, "2: src/gespraeche/nachholen.rs"),
        "{befunde:?}"
    );
}

/// Mutant 2c: eine zweite Durchlauf-Stelle in einer Datei der Ausnahmeliste (Stellenzahl).
#[test]
fn eine_weitere_durchlauf_stelle_in_einer_ausnahmedatei_ist_rot() {
    let Some((pfad, stellen, _)) = ALLE_OHNE_PRUEFUNG.first() else {
        panic!("ALLE_OHNE_PRUEFUNG ist leer: die Stellenzahl wird nicht geprüft");
    };
    let quellen = mit_geaendert(&echt(), pfad, |text| {
        format!("fn heimlich(v: &Verzeichnis) {{ for _ in v.verlaeufe.values() {{}} }}\n{text}")
    });
    let befunde = pruefen(&quellen);
    assert!(
        befunde_mit(
            &befunde,
            &format!("{pfad} hat {} Durchlauf-Stelle(n)", stellen + 1)
        ),
        "{befunde:?}"
    );
}

/// Mutant 3: ein zweiter Name der Entscheidung (`fn sichtbar`, `fn ist_sichtbar`, `fn darf_sehen`, `beteiligt`).
#[test]
fn ein_zweiter_name_der_entscheidung_ist_rot() {
    for (name, aufruf) in [
        ("sichtbar", ""),
        ("ist_sichtbar", ""),
        ("darf_sehen", ""),
        ("pair_beteiligt", ""),
        ("sichtbar_fuer", ""),
        ("hilfe", "paar.beteiligt(&kennung)"),
    ] {
        let text = format!("fn {name}(g: &Gespraech) -> bool {{ {aufruf}; true }}\n");
        let mut quellen = echt();
        quellen.push(Quelle::neu("src/http/zweitens.rs", &text));
        let befunde = pruefen(&quellen);
        assert!(
            befunde_mit(&befunde, "3: src/http/zweitens.rs"),
            "{name}: {befunde:?}"
        );
    }
}

/// Ein Name wie `sichtbare_marke` ist keine zweite Entscheidung.
#[test]
fn die_eine_funktion_und_verwandte_namen_sind_nicht_rot() {
    let mut quellen = echt();
    quellen.push(Quelle::neu(
        "src/http/marke.rs",
        "fn sichtbare_marke() {}\n",
    ));
    let befunde = pruefen(&quellen);
    assert!(!befunde_mit(&befunde, "src/http/marke.rs"), "{befunde:?}");
}

/// Mutant 4: ein `_ =>` im Strom-Filter (`ereignisse.rs`, echter `match` über `Gespraech`).
#[test]
fn ein_wildcard_im_strom_filter_ist_rot() {
    let quellen = mit_geaendert(&echt(), "src/http/ereignisse.rs", |text| {
        text.replace(
            "Gespraech::Direkt(paar) => Ort::Direkt(paar.gegenueber(halter).cloned()),",
            "_ => Ort::Direkt(None),",
        )
    });
    let befunde = pruefen(&quellen);
    assert!(
        befunde_mit(&befunde, "4: src/http/ereignisse.rs"),
        "{befunde:?}"
    );
}

/// Mutant 4b: das `_ =>` in den anderen verteilenden Wegen — Warten (`nachholen.rs`) und Anhänge (`anhangpfade.rs`).
#[test]
fn ein_wildcard_in_warten_und_anhangpfaden_ist_rot() {
    let nachholen = mit_geaendert(&echt(), "src/gespraeche/nachholen.rs", |text| {
        text.replace(
            "Gespraech::Direkt(_) => alle || filter.direkt,",
            "_ => false,",
        )
    });
    assert!(befunde_mit(
        &pruefen(&nachholen),
        "4: src/gespraeche/nachholen.rs"
    ));
    let pfade = mit_geaendert(&echt(), "src/gespraeche/anhangpfade.rs", |text| {
        text.replace(
            "Gespraech::Direkt(paar) => (\"direkt\", paar.ordnername()),",
            "_ => (\"direkt\", String::new()),",
        )
    });
    assert!(befunde_mit(
        &pruefen(&pfade),
        "4: src/gespraeche/anhangpfade.rs"
    ));
}

/// Kein Fehlalarm: `_ =>` in einem `match` über etwas anderes, im Kommentar, in der Zeichenkette und im Testmodul.
#[test]
fn ein_wildcard_ausserhalb_eines_match_ueber_gespraech_ist_nicht_rot() {
    let text = "fn a(z: Option<u8>) -> u8 {\n    match z {\n        Some(1) => 1,\n        _ => 0,\n    }\n}\n\
                // match g { Gespraech::Channel(_) => 1, _ => 2 }\n\
                const T: &str = \"match g { Gespraech::Direkt(_) => 1, _ => 2 }\";\n\
                #[cfg(test)]\nmod tests {\n    fn t(g: Gespraech) -> u8 { match g { Gespraech::Direkt(_) => 1, _ => 2 } }\n}\n";
    let mut quellen = echt();
    quellen.push(Quelle::neu("src/http/ruhig.rs", text));
    let befunde = pruefen(&quellen);
    assert!(!befunde_mit(&befunde, "src/http/ruhig.rs"), "{befunde:?}");
}

/// Die Aufbereitung: Testmodule, Kommentare, Zeichenketten (auch roh) und Zeichenliterale enthalten keine Treffer.
#[test]
fn nur_nicht_test_code_zaehlt() {
    let text = "// Gespraech::Direkt\n/* Gespraech::Direkt /* verschachtelt */ */\nconst A: &str = \"Gespraech::Direkt\\\"\";\n\
                const B: &str = r#\"Gespraech::Direkt\"#;\nconst C: char = '\"';\nfn f<'a>(x: &'a str) -> &'a str { x }\n\
                #[cfg(test)]\nmod tests {\n    fn t() { let _ = Gespraech::Direkt(p); }\n}\n";
    let bereinigt = produktiv(&Quelle::neu("src/x.rs", text));
    assert!(!bereinigt.contains("Direkt"), "{bereinigt}");
    assert!(bereinigt.contains("fn f<'a>"), "{bereinigt}");
    let eigenes_modul = vec![
        Quelle::neu("src/x/mod.rs", "#[cfg(test)]\nmod probe;\n"),
        Quelle::neu("src/x/probe.rs", "fn t() { Gespraech::Direkt(p); }\n"),
        Quelle::neu("src/x/probe/tief.rs", "fn t() { Gespraech::Direkt(p); }\n"),
    ];
    assert!(
        pruefen(&eigenes_modul).is_empty(),
        "{:?}",
        pruefen(&eigenes_modul)
    );
}
