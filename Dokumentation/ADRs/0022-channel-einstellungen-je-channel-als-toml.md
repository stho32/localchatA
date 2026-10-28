# ADR-0022: Channel-Einstellungen als `einstellungen.toml` im Ordner des Channels — geschrieben beim Anlegen, gelesen beim Start, Crate `toml`

**Status**: Akzeptiert
**Datum**: 2026-10-28
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #6 I0007)
**Kontext-Anforderung**: — (WBS I0007, I0018; B0021 „`einstellungen.toml` wird nicht gelesen (I0007/I0018)“)
**Erprobt durch**: Dependency-Probe im Scratchpad (`toml` 1.1.8, `serde` 1.0.229, 2026-10-28): Rundweg einer Struktur mit Anführungszeichen, `↔` und Zeilenumbruch in der Beschreibung byte-gleich; unbekannter Schlüssel (`farbe`) wird beim Lesen in die Struktur übergangen und über `toml::Table` beim Zurückschreiben erhalten, ein Kommentar dagegen geht verloren; Fehler mit Meldung und Byte-Spanne für offene Zeichenkette (`invalid basic string`), `-3` für `u32` (`invalid value`), doppelten Schlüssel (`duplicate key`), fehlendes Pflichtfeld, ungültiges UTF-8 über `from_slice`. Neue Crates im Lockfile: `toml`, `toml_parser`, `toml_writer`, `toml_datetime`, `serde_spanned`, `winnow`, `indexmap`, `hashbrown`, `equivalent` (9; heute 120). Probe danach gelöscht.

## Kontext

- ADR-0003 legte `channels/<slug>/einstellungen.toml` mit `name, frist_tage, angelegt, angelegt_von` fest, geschrieben über temporäre Datei + `rename`; Vorteil dort: „jedes Gespräch für sich kopierbar“. Gebaut ist nichts davon; `Gespraeche::laden` nimmt Standardwerte (7 Tage, `src/gespraeche/frist.rs`).
- ADR-0005 (vorgeschlagen, Empfehlung „nur lesbar“): das Programm ist einziger Schreiber; ein Dateibeobachter ist nicht vorgesehen.
- Vision: „lesbar ohne localchatA“, Frist je Channel wählbar, Standard 7 Tage, ältere Tage entfallen (I0018).
- ADR-0020: der Name ist der Ordnername — er steht deshalb nicht mehr in der Datei.
- ADR-0021: die Grenze je Herkunft zählt über `angelegt_von` dauerhaft — die Datei muss beim Anlegen entstehen.
- Designsystem 4.3: „Name, optional ein Satz Beschreibung“.

## Optionen

### Option A: Eine Datei je Channel, `channels/<name>/einstellungen.toml`, mit der Crate `toml` (serde)
- Vorteile: ein Channel-Ordner ist vollständig und für sich kopierbar (Logs, Anhänge, Einstellungen); TOML ist für Hand und Auge gemacht (`frist_tage = 30`), Rust-Nutzern aus `Cargo.toml` vertraut; Fehler mit Position (Probe); Wortlaut von ADR-0003.
- Nachteile: neun neue Crates; Kommentare gehen beim Zurückschreiben verloren (Probe); viele kleine Dateien.

### Option B: Eine Datei je Channel als `einstellungen.json` mit dem vorhandenen `serde_json`
- Vorteile: keine neue Abhängigkeit; gleiche Ablage-Vorteile wie A.
- Nachteile: von Hand fehleranfällig (Komma, Anführungszeichen), keine Kommentare; weicht von ADR-0003 ab.

### Option C: Eine zentrale `<ordner>/einstellungen.toml` mit `[channels.<name>]`
- Vorteile: ein Ort für alle Einstellungen; einmal lesen beim Start.
- Nachteile: der Channel-Ordner ist nicht mehr für sich kopierbar; jede Änderung schreibt die Einträge aller Channels neu; entfernt jemand einen Channel-Ordner von Hand, bleibt ein verwaister Eintrag; eine defekte Datei trifft alle Channels zugleich.

### Option D: Eine Datei je Channel in einer selbstgeschriebenen TOML-Teilmenge (`schluessel = "text"` | Zahl)
- Vorteile: keine Abhängigkeit; Format wie A für den Leser.
- Nachteile: eigener Parser für Escapes, Unicode, Mehrzeiliges und Fehlerpositionen — genau die Sorte Code, die das Security-Review #2 bei der Kennung beanstandete; jede Hand-Änderung jenseits der Teilmenge bricht.

## Entscheidung

**Gewählt: Option A**

Es überwiegt, dass der Channel-Ordner vollständig bleibt und die Datei so lesbar und von Hand änderbar ist wie die Logs, mit einem erprobten Parser statt eigenem. In Kauf genommen: neun Crates mehr im Lockfile und verlorene Kommentare, wenn das Programm die Datei neu schreibt.

Regeln:

1. **Ort und Inhalt** — `channels/<name>/einstellungen.toml`:
   ```toml
   angelegt = "2026-10-28T10:15:00.123+01:00"   # Serverzeit (Uhr-Provider), RFC 3339 mit ms, als Text
   angelegt_von = "shoff@127.0.0.1"             # Kennung des Anlegers
   beschreibung = "Absprachen zwischen Security- und Entwicklungs-Factory"   # optional
   # ab I0018: frist_tage = 7                    # optional, fehlt = Standard 7
   ```
   Kein `name` — der Name ist der Ordner (ADR-0020); zwei Quellen könnten sich widersprechen.
2. **Geschrieben** nur von [Gespräche](../Architektur/A00006-gespraeche.md) über den Port der [Logablage](../Architektur/A00008-logablage.md) (`einstellungen_schreiben(gespraech, bytes)`): erst `einstellungen.toml.neu`, dann `rename` — nie halb geschrieben. Beim Anlegen direkt nach `create_dir`; scheitert das Schreiben, wird der eben angelegte Ordner wieder entfernt (ADR-0021, 500). Das Format (TOML ↔ `Einstellungen`) ist eine reine Operation ([A00013 Einstellungsformat](../Architektur/A00013-einstellungsformat.md)); die Logablage sieht nur Bytes.
3. **Gelesen** beim Start in `laden`, je Channel einmal. Fehlt die Datei: `angelegt` und `angelegt_von` unbekannt (`null` in Liste und Ereignis), keine Beschreibung, Frist 7. Ist sie defekt (TOML-Fehler, falscher Typ, ungültiges UTF-8, `angelegt_von` keine gültige Kennung, `beschreibung` verletzt Regel 5): dieselben Standardwerte **und** ein Ladedefekt mit Datei und Zeile auf stderr, mitgezählt in `defekte_zeilen` des Health-Checks; die Datei bleibt unangetastet. Ein unbekannter Schlüssel ist kein Defekt.
4. **Fremdänderung**: das Programm liest nur beim Start und beobachtet die Datei nicht (ADR-0005). Wer sie von Hand ändert, tut es bei gestopptem Programm; Änderungen während des Laufs sieht erst der nächste Start, und ein Schreiben des Programms (ab I0018: Frist ändern) überschreibt sie. Beim Zurückschreiben bleiben unbekannte Schlüssel erhalten (Lesen als Tabelle, bekannte Schlüssel setzen); Kommentare nicht.
5. **Beschreibung** (optional): fehlt, `null` oder `""` = keine. Sonst 1–200 Unicode-Zeichen, keine Steuerzeichen (Kategorie Cc, also auch kein Zeilenumbruch); Verletzung → 422 `{fehler, regel}`. Kein Umformen. Die Oberfläche zeigt sie nur als Text und isoliert ihre Schreibrichtung (`<bdi>` bzw. `unicode-bidi: isolate`), damit Formatzeichen wie U+202E nichts außerhalb verdrehen.
6. **Sicherheit vor Bequemlichkeit für I0018**: ist die Datei eines Channels defekt, entfällt bei ihm **kein** Tag, bis sie lesbar ist — eine kaputte Datei darf nie über die Standardfrist zu Datenverlust führen. (Regel für den Bau von I0018, hier festgehalten, weil sie aus dieser Wahl folgt.)

## Konsequenzen

**Positiv:**
- `cat channels/security-entwicklung/einstellungen.toml` sagt, wer den Channel wann angelegt hat; Kopieren des Ordners nimmt alles mit.
- I0018 ergänzt nur einen Schlüssel und das Zurückschreiben; Lesen, Defektbehandlung und Ablage stehen.

**Negativ / Inkauf zu nehmen:**
- Neue Abhängigkeit mit neun Crates (Lockfile von 120 auf 129 Pakete); Bauzeit der Probe ab leerem Ziel 5 s.
- Ein von Hand gesetzter Kommentar verschwindet beim ersten Schreiben durch das Programm.

**Pre-Mortem (ein Monat nach dem Start gescheitert, weil …):**
1. … ein Tippfehler in `frist_tage` die Frist still auf 7 setzt und das Entfallen 23 Tage Verlauf löscht. Früh-Indikator: Ladedefekt-Zeile beim Start. Gegenmaßnahme: Regel 6 (kein Entfallen bei defekter Datei), Test zuerst in I0018.
2. … ein Absturz zwischen `create_dir` und `rename` einen Channel ohne Einstellungen hinterlässt, der dann nie zur Grenze seiner Herkunft zählt. Früh-Indikator: Channel mit `angelegt_von: null`, den das Programm angelegt hat. Gegenmaßnahme: Regel 3 lädt ihn trotzdem (Gesamtgrenze zählt), der Rückbau bei Fehler (Regel 2) deckt jeden Fehler außer dem Absturz; hingenommen.
3. … die Crate ein Update mit geändertem Fehlerverhalten bekommt. Früh-Indikator: Test „defekte Datei → Defektzeile“ rot nach `cargo update`. Gegenmaßnahme: die Probe-Fälle werden als Unit-Tests von A00013 gebaut (Skill `dependency-probe`, Probe behalten).

**Folgeentscheidungen / Aufgaben:**
- [ ] `toml = "1"` (Features Standard: `parse`, `display`, `serde`) in `Cargo.toml`; `serde` ist schon da.
- [ ] Erster Test (Unit, A00013, rein): `Einstellungen{angelegt, angelegt_von, beschreibung}` → Text → zurück gleich; Text mit offener Zeichenkette, `angelegt_von = "Claude@x"`, `beschreibung` mit `\n`, ungültigem UTF-8 → je `Defekt` mit Zeile; unbekannter Schlüssel → gelesen ohne Defekt und beim Zurückschreiben erhalten.
- [ ] Erster Integrationstest (Datei-Logablage gegen `tempfile`): `einstellungen_schreiben` hinterlässt keine `.neu`-Datei, `rename` ersetzt eine vorhandene Datei; Laden eines Ordners mit fehlender, gültiger und defekter Datei → zwei Channels mit Standardwerten, einer mit Werten, genau ein Ladedefekt.

## Quellen / Bezuege

- [A00013 Einstellungsformat](../Architektur/A00013-einstellungsformat.md), [A00012 Channelverzeichnis](../Architektur/A00012-channelverzeichnis.md), [A00008 Logablage](../Architektur/A00008-logablage.md), [A00006 Gespräche](../Architektur/A00006-gespraeche.md)
- ADR-0003 (Ablage-Layout), ADR-0005 (einziger Schreiber), ADR-0020, ADR-0021
- Vision R00000 (Frist je Channel, lesbar ohne localchatA); WBS B0021, I0018; `design-prompt.md` 4.3
- Crate `toml` 1.1.8 (toml-rs), Verhalten wie oben geprobt

## Ergänzungen

- **2026-10-28 (Karte #12 I0018) — `frist_tage`:** Ganzzahl 1–3650; ein anderer Typ oder ein Wert außerhalb ist ein Defekt nach Regel 3 (Standardwerte, Ladedefekt, Datei unangetastet) **und** sperrt nach Regel 6 jedes Entfallen in diesem Channel; eine **fehlende** Datei oder ein fehlender Schlüssel ist kein Defekt — es gilt 7 und es entfällt wie sonst ([ADR-0029](0029-frist-in-kalendertagen-heute-zaehlt-mit-1-bis-3650.md), [ADR-0032](0032-loeschen-nur-tagesdateien-nach-erlaubnisliste-keine-symlinks-defekt-heisst-nichts.md)). Geschrieben wird `frist_tage` erst bei der ersten Änderung über `PUT /api/channels/{name}/frist`, nie über eine beim Start defekte Datei (409 `einstellungen-defekt`, [ADR-0031](0031-frist-setzen-put-api-channels-name-frist-jeder-mit-sitzung-bestaetigung-beim-verkuerzen.md)). Liste, Stand und Wechsel tragen `frist_tage` (Zahl, `null` bei defekter Datei).
