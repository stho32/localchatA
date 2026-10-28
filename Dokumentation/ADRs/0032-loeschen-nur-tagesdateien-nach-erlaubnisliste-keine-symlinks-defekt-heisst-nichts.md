# ADR-0032: Löschen nur nach Erlaubnisliste — reguläre Tagesdateien `JJJJ-MM-TT.log` gültiger Channels, keine Symlinks, Pfad aus Typen; defekte Einstellungen heißen „nichts entfällt“

**Status**: Akzeptiert
**Datum**: 2026-10-28
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #12 I0018)
**Kontext-Anforderung**: — (WBS I0018; prägt I0010 Anhänge, I0012 Direktgespräche)
**Erprobt durch**: Wegwerf-Probe im Scratchpad (`rustc` 2021, Linux, 2026-10-28): in `channels/a/` liefert `read_dir` für einen Symlink `2026-01-01.log` → Datei außerhalb `file_type().is_file() = false`, `is_symlink() = true`; ein Ordner `2026-01-03.log/` `is_file() = false`; eine reguläre `2026-01-02.log` `is_file() = true`; `symlink_metadata(…).is_file()` für den Symlink `false`; `remove_dir_all(anhaenge)` mit einem Symlink auf einen Ordner außerhalb entfernt nur den Link, `aussen/geheim.txt` und `aussen/ziel.log` bleiben. Probe danach gelöscht.

## Kontext

- I0018 ist die erste Stelle, an der localchatA Dateien des Nutzers **löscht**. Der Ordner ist „lesbar ohne localchatA“ (Vision) — Menschen legen dort Notizen, Kopien, Symlinks oder Sicherungen ab. Ein Fehler beim Löschen ist nicht rückholbar ([ADR-0030](0030-entfallen-beim-start-beim-tageswechsel-und-beim-verkuerzen-loeschen-unter-der-gespraechssperre.md)).
- Gebaut: `Logablage::tage` nimmt nur Einträge mit `file_type().is_file()` (folgt keinem Symlink) und Namen, die `tag_aus_dateiname` als `JJJJ-MM-TT.log` in genau dieser Schreibweise mit gültigem Datum erkennt (`src/gespraeche/logablage.rs:113`, `:235`); `unterordner` nimmt nur echte Ordner (`:216`). Ein Ordner mit ungültigem Channelnamen ist kein Channel ([ADR-0020](0020-channelname-ist-ordnername-erlaubnisliste.md) Regel 6).
- [ADR-0022](0022-channel-einstellungen-je-channel-als-toml.md) Regel 6: ist die Einstellungsdatei defekt, entfällt in diesem Channel kein Tag.
- Aspekt [Logging und Diagnose](../Architektur/Aspekte/X00001-logging-und-diagnose.md): Ladedefekte stehen auf stderr und im Health-Check.

## Optionen

### Option A: Erlaubnisliste — nur reguläre Tagesdateien gültiger Channels, Pfad aus Typen, Symlinks nie
- Vorteile: was nicht ausdrücklich erkannt ist, bleibt; die Erkennung ist dieselbe wie beim Laden (gebaut, getestet); der Pfad entsteht aus `Channelname` und `NaiveDate`, nie aus einem Text der Platte — kein `..`, kein fremder Ordner.
- Nachteile: liegengebliebene, nicht erkannte Dateien (`2026-1-5.log`, `alt.log.bak`) räumt niemand weg.

### Option B: Alles im Channel-Ordner, das älter ist als die Frist (Änderungszeit), außer `einstellungen.toml`
- Vorteile: räumt auch Fremdes auf; hält den Ordner klein.
- Nachteile: löscht Notizen und Sicherungen des Nutzers; die Änderungszeit ist nicht das Datum des Inhalts (Kopieren setzt sie neu); verletzt „lesbar ohne localchatA“ — der Ordner gehört nicht nur dem Programm.

### Option C: Nur Tage, die im Index stehen
- Vorteile: das Programm löscht nur, was es selbst geladen hat.
- Nachteile: Tage außerhalb der Frist werden beim Start gar nicht geladen — sie blieben für immer liegen; Fertig-Kriterium verfehlt.

### Option D: Wie A, aber in einen Papierkorb verschieben statt löschen
- Vorteile: auch ein Fehler in der Erkennung ist rückholbar.
- Nachteile: siehe ADR-0030 (Platz wird nicht frei, zweite Pflege nötig); der Schutz gegen falsche Erkennung kommt bei A aus der Erlaubnisliste und ihren Tests.

## Entscheidung

**Gewählt: Option A**

Es überwiegt, dass nur gelöscht wird, was das Programm selbst als seine Tagesdatei erkennt — mit derselben Erkennung, die beim Laden seit I0002 gilt —, und dass kein Pfad aus Text der Platte entsteht. In Kauf genommen: Fremdes und nicht Erkanntes bleibt für immer liegen; aufräumen muss der Mensch.

Regeln:

1. **Nur Channels**: gepflegt werden Unterordner von `channels/` mit gültigem Channelnamen (ADR-0020), die beim Start geladen oder seither angelegt wurden. Nicht gepflegt: `direkt/` (bis I0012 entscheidet), Ordner mit ungültigem Namen, `.`-Ordner, symbolische Links auf Ordner (`unterordner` nimmt sie nicht auf).
2. **Nur reguläre Tagesdateien**: Kandidat ist ein Eintrag, den `Logablage::tage` liefert — `file_type()` ohne Symlink-Folgen `is_file()`, Name genau `JJJJ-MM-TT.log` mit gültigem, kanonisch geschriebenem Datum. Nicht: Symlinks (auch wenn sie so heißen), Ordner, `2026-1-05.log`, `2026-10-21.log.bak`, `einstellungen.toml`, alles andere.
3. **Pfad aus Typen**: `tag_entfernen(gespraech, tag)` bildet den Pfad aus `Gespraech::Channel(<Channelname>)` und `tag.format("%Y-%m-%d")` (`Gespraech::tagesdatei`), prüft unmittelbar davor mit `symlink_metadata`, dass dort eine reguläre Datei liegt, und ruft dann `remove_file`. Liegt dort etwas anderes (inzwischen ein Symlink, ein Ordner, nichts), wird nichts gelöscht; „nichts“ zählt als entfallen, alles andere als Fehler (stderr).
4. **Nie heute oder später**: `pflegen` löscht nur Tage, die `frist::teilen` als entfallend liefert (ADR-0029). Die Frist ist mindestens 1 (geprüft an der API und beim Lesen der Datei), also liegt der älteste behaltene Tag nie nach heute. Kein zweiter Riegel im Code — er wäre bei richtiger Auswahl nicht beobachtbar und damit ungetestet; geschützt wird über die Tests der Auswahl und der Spanne (Mutanten in ADR-0029).
5. **Defekt heißt nichts**: ist die Einstellungsdatei beim Start defekt oder nicht lesbar (Ladedefekt, ADR-0022 Regel 3), oder liegt `frist_tage` außerhalb 1–3650 bzw. hat einen anderen Typ (ADR-0029 Regel 1), entfällt in diesem Channel nichts, bis die Datei repariert und das Programm neu gestartet ist. Jede Pflege meldet das einmal je Anlass auf stderr: `channels/x: nichts entfällt — Einstellungen defekt (channels/x/einstellungen.toml:3)`. Eine **fehlende** Datei ist kein Defekt: es gilt der Standard 7, und es entfällt wie sonst (Vision: Standard 7 Tage).
6. **Fehler je Datei**, nie je Lauf: scheitert das Löschen einer Datei, bleibt sie in Platte und Index, eine Zeile auf stderr nennt Datei und Grund; die übrigen Tage und Channels werden weiter gepflegt (ADR-0030 Regel 4).
7. **Anhänge** (I0010): nach der Logdatei `anhaenge/<tag>/` mit `remove_dir_all` — es folgt keinen Symlinks (Probe); ist `anhaenge/<tag>` selbst ein Symlink, wird nur der Link entfernt (`remove_file`), nie sein Ziel.
8. **Protokoll** auf stderr wie ADR-0030 Regel 6 und ADR-0031 Regel 5 — wer, was, wie viele, warum.

## Konsequenzen

**Positiv:**
- Kein Weg, über einen Namen auf der Platte etwas außerhalb des Channel-Ordners zu löschen; ein Symlink im Ordner ist für die Pflege unsichtbar.
- Eine kaputte `frist_tage`-Zeile kostet nie Verlauf (Pre-Mortem 1 aus ADR-0022 ist damit geschlossen).
- Die Regeln sind Eigenschaften der Logablage und der reinen Auswahl — prüfbar mit `tempfile` ohne Programm.

**Negativ / Inkauf zu nehmen:**
- Ein Symlink `2026-01-01.log` wird weder geladen (seit I0002) noch gelöscht.
- Zwischen `symlink_metadata` und `remove_file` könnte ein lokaler Prozess die Datei gegen einen Symlink tauschen; `remove_file` entfernt dann den Link, nicht sein Ziel (unlink folgt nie) — kein Schaden außerhalb.
- Wer seine Einstellungsdatei kaputt macht, behält in diesem Channel alles, bis er sie repariert — der Platz wächst.

**Pre-Mortem (ein Monat nach dem Start gescheitert, weil …):**
1. … ein Betreiber eine Sicherung `channels/allgemein/2026-09-01.log` als Symlink auf sein Archiv legt und das Archiv verschwindet. Früh-Indikator: Integrationstest „Symlink mit Tagesnamen“. Gegenmaßnahme: Regeln 2 und 3, Test zuerst (unten).
2. … ein Channel mit defekter Einstellungsdatei über Monate wächst und niemand es merkt. Früh-Indikator: Ladedefekt im Health-Check (`defekte_zeilen`), stderr bei jedem Tageswechsel, `frist_tage: null` in der Liste, Oberfläche zeigt „Frist gesperrt“ ([ADR-0033](0033-frist-in-der-kopfzeile-dialog-mit-bestaetigung-beim-verkuerzen.md)). Gegenmaßnahme: Regel 5 meldet täglich.
3. … ein Direktgespräch-Ordner gepflegt wird, bevor I0012 seine Frist entschieden hat. Früh-Indikator: Test „`direkt/…/2026-01-01.log` bleibt“. Gegenmaßnahme: Regel 1.

**Folgeentscheidungen / Aufgaben:**
- [ ] Erster Integrationstest (Datei-Logablage gegen `tempfile`, ohne Programm): in `channels/allgemein/` die reguläre `2026-10-01.log`, ein Symlink `2026-10-02.log` auf `aussen/ziel.log` außerhalb, ein Ordner `2026-10-03.log/`, `2026-10-04.log.bak`, `notizen.txt`; `pflegen(2026-10-28)` mit Frist 7 → nur `2026-10-01.log` fehlt; alle anderen Einträge und `aussen/ziel.log` liegen byte-gleich da.
- [ ] Integrationstest (echtes Programm, `LOCALCHATA_JETZT=2026-10-28T12:00:00+01:00`): `channels/kaputt/einstellungen.toml` mit `frist_tage = "sieben"` und `2026-01-01.log`, `channels/zu-gross/` mit `frist_tage = 0`, `direkt/a@x+b@y/2026-01-01.log` → nach dem Start liegen alle drei Tagesdateien da, die Einstellungsdateien byte-gleich; stderr nennt beide Channels mit „nichts entfällt“; `GET /api/channels` → `frist_tage: null` für beide; `PUT …/kaputt/frist` → 409 `einstellungen-defekt`.
- [ ] Mutationsprobe der Wächter (Gelerntes „Mutationsprobe für Wächter“): je einzeln abgeschaltet — `is_file`-Prüfung in `tage`, `symlink_metadata`-Prüfung in `tag_entfernen`, „defekt heißt nichts“ in `pflegen`, Ausschluss von `direkt/` — wird je mindestens ein Test rot.

## Quellen / Bezuege

- [A00008 Logablage](../Architektur/A00008-logablage.md), [A00006 Gespräche](../Architektur/A00006-gespraeche.md), [A00013 Einstellungsformat](../Architektur/A00013-einstellungsformat.md); Aspekte [X00001 Logging und Diagnose](../Architektur/Aspekte/X00001-logging-und-diagnose.md), [X00004 Fehlerbehandlung](../Architektur/Aspekte/X00004-fehlerbehandlung.md)
- ADR-0003, ADR-0020, ADR-0022, ADR-0029, ADR-0030, ADR-0031; `src/gespraeche/logablage.rs` (`tage`, `unterordner`, `tag_aus_dateiname`)
- Rust std: `DirEntry::file_type` folgt keinen Symlinks; `remove_dir_all` folgt seit Rust 1.58.1 keinen Symlinks (CVE-2022-21658) — beides oben geprobt
