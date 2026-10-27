# ADR-0006: Verlaufsindex im Speicher über dem Rohtext der Tagesdateien

**Status**: Akzeptiert
**Datum**: 2026-10-27
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #1 I0001)
**Kontext-Anforderung**: — (WBS I0008, I0011)
**Erprobt durch**: Wegwerf-Probe im Scratchpad (Rust, `--release`, 8 Kerne, 31 GB): 10 Channels × 7 Tagesdateien im Format aus ADR-0004, Nachrichten 10–90 Wörter, 5 % mit Codeblock (~45 Zeilen); Probe nach dem Lauf gelöscht

## Kontext

Leistungsziel der Vision: mit 100.000 Nachrichten und mehr in der Frist öffnen Channels sofort, der Verlauf scrollt leicht, die Suche liefert schnell. Das Designsystem konkretisiert: Blöcke zu 100, nach oben verankert; die ersten 50 Treffer sofort, dann „Weitere 50 laden“. Eine Datenbank ist ausgeschlossen (Leitplanke).

## Optionen

### Option A: Je Nachricht ein Objekt im Speicher (Text + Kleinschreibungskopie)
- Probe 100.000: Laden 1,5–2,0 s, Speicher +120 MB; letzter Block < 1 µs; Suche ohne Treffer (Vollscan) 11–13 ms.
- Vorteile: einfach; schnellste Suche.
- Nachteile: Laden langsam (viele kleine Zuweisungen); bei 1.000.000: 16,6 s und +1,2 GB.

### Option B: Rohtext jeder Tagesdatei als ein `String`, Satzanfänge als Offsets, Kleinschreibungskopie je Datei
- Probe 100.000 (53 MB Logs): Laden 0,43 s (ein Kern); Speicher ≈ 2 × Loggröße ≈ 110 MB (Rohtext + Suchkopie + 4 B je Satz); letzter Block zu 100 einschließlich Aufbereitung 0,06 ms; Suche häufiger Begriff, 50 Treffer: 0,03 ms; seltener Begriff (3 Treffer): 24 ms; ohne Treffer (Vollscan): 41 ms.
- Probe 1.000.000 (533 MB): Laden 3,5 s; Vollscan 220–380 ms; Speicher ≈ 1,1 GB.
- Vorteile: Laden ist Lesen + ein Durchlauf; die Datei im Speicher ist dieselbe wie auf Platte (Anhängen = `push_str`).
- Nachteile: Speicher wächst linear mit der Loggröße; ab etwa 1.000.000 Nachrichten wird die Suche spürbar.

### Option C: Invertierter Index auf Platte (z. B. `tantivy`) neben den Logs
- Vorteile: skaliert in die Millionen; Wort- und Präfixsuche.
- Nachteile: zweiter Bestand im Ordner, der mit den Logs synchron sein muss (Neuaufbau nach jedem fremden Eingriff); große Abhängigkeit; Teilwortsuche schwieriger; widerspricht dem Geist „Ordner aus Logs, keine Datenbank“.

### Option D: Nur Offsets im Speicher, Text bei Bedarf von Platte (`pread`)
- Vorteile: Speicher ~4 MB bei 100.000.
- Nachteile: jede Suche liest alle Dateien (Seitencache hilft, garantiert aber nichts); mehr Code für dieselbe Leistung bei 100.000.

## Entscheidung

**Gewählt: Option B**

Sie erfüllt Q1 und Q2 mit großem Abstand (Ziel < 50 ms bzw. < 100 ms; gemessen 0,06 ms bzw. ≤ 41 ms) und Q6 (Start < 2 s; gemessen 0,43 s) bei 100.000 Nachrichten, ohne zweiten Bestand im Ordner. In Kauf genommen: Speicher ≈ 2 × Loggröße und eine praktische Grenze um 1.000.000 Nachrichten in der Frist; die Frist (Standard 7 Tage) begrenzt das Wachstum. Der Index steht hinter der schmalen Schnittstelle von [A00009](../Architektur/A00009-verlaufsindex.md), sodass D oder C später ohne Änderung der Aufrufer möglich sind.

## Konsequenzen

**Positiv:**
- Aufbau beim Start: alle Tagesdateien in der Frist lesen (parallel je Gespräch möglich), danach erst binden — der Health-Check ist ab der Startzeile grün.
- Rotieren: neue Tagesdatei = neuer Eintrag; Entfallen: Eintrag fallen lassen, Dateien löschen — kein Umbau des Index.
- Suche Tag für Tag von neu nach alt; „Weitere 50“ setzt bei der Zeit des letzten Treffers fort.

**Negativ / Inkauf zu nehmen:**
- Grobe Rechnung je Nachricht ≈ 530 B Rohtext (Mittel der Probe) → 100.000 ≈ 53 MB Log, ≈ 110 MB Speicher; 1.000.000 ≈ 1,1 GB.

**Folgeentscheidungen / Aufgaben:**
- [ ] Früh-Indikator: Startzeile und `/api/gesundheit` nennen Nachrichtenzahl, Loggröße und Ladezeit.
- [ ] Leistungstest als Gate-Teil (I0008/I0011): 100.000 erzeugte Nachrichten, Block < 50 ms und Suche ohne Treffer < 100 ms über HTTP gegen das echte Programm (im Release-Profil oder mit Toleranz für Debug).

## Quellen / Bezuege

- [A00009 Verlaufsindex](../Architektur/A00009-verlaufsindex.md), [A00006 Gespräche](../Architektur/A00006-gespraeche.md); ADR-0003, ADR-0004
- Vision, Zielbild „Schnell, auch bei großem Verlauf“; Designsystem README „Leistung als Gestaltung“; WBS I0008, I0011
