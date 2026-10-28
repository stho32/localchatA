# ADR-0040: Betriebsprotokoll — eine Datei je lokalem Tag unter `logs/`, eine Zeile je Eintrag mit Zeit, Stufe, Bereich, Text und Kontext; stderr spiegelt ab Warning

**Status**: Akzeptiert
**Datum**: 2026-10-28
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Vorlauf „Betriebsprotokoll“; Vorgabe des Betreibers 2026-10-28)
**Kontext-Anforderung**: — (neue Interaction „Betriebsprotokoll“, noch nicht in der WBS)
**Erprobt durch**: — (überträgt gebaute Mechanik: Tagesdatei nach lokalem Datum über den Uhr-Provider — `src/uhr.rs`, `src/gespraeche/logablage.rs`; Zeitformat RFC 3339 mit Millisekunden und Versatz wie die Nachrichtenzeit, ADR-0004)

## Kontext

- **Vorgabe des Betreibers** (`AI-Team/umgebung.md`, „Logging-Konzept“, 2026-10-28): Betriebsprotokoll als Datei im Instanzordner unter `logs/`, täglich rotierend, Aufbewahrung 14 Tage, danach bereinigt; Stufen Debug · Information · Warning · Error (Fachlogik) · Exception (Programmfehler), Standard Information, Debug zuschaltbar. Ziel: Monitoring; Fehler müssen irgendwo mitgeschrieben werden. Die Details entscheidet der architekt; geprüft wird vom `observability-reviewer` (Factory-Rolle; Prüfraster: Ziel, Format mit Zeitzone, eine Zeile je Eintrag, Kontext ohne Secrets, Episoden statt Fluten, CWE-117).
- **Bestand** (Code, Stand `main` 77ba581): Meldungen gehen mit `eprintln!` auf stderr — Ladedefekte (`src/start/mod.rs:155`), Abrisspflege, erster Fehler je Lauf (`src/start/abrisspflege.rs:120`), gescheiterter Rückbau (`src/gespraeche/logablage.rs:153`), Channel nicht angelegt (`src/http/channels.rs:135`), Signal nicht abfangbar, Browser nicht zu öffnen, Startfehler (`src/start/mod.rs`). Weitere stderr-Zeilen sind entschieden, aber noch nicht gebaut: Fristpflege (ADR-0030 Regel 6), Frist gesetzt (ADR-0031 Regel 5), Löschfehler (ADR-0032 Regel 6), Vorlauf der Nachrichtenzeit (ADR-0035 Regel 4), Schreibkonto-Episoden (ADR-0036 Regel 7). stderr geht verloren, sobald niemand das Terminal liest — für Monitoring reicht es nicht; [X00001](../Architektur/Aspekte/X00001-logging-und-diagnose.md) hatte stderr empfohlen, die offene Frage beantwortet jetzt der Betreiber.
- **Ordner** ([ADR-0003](0003-ablage-layout-im-ordner.md)): `<ordner>/.localchatA.sperre`, `channels/<name>/`, `direkt/<a>+<b>/`. Die Logablage liest nur `channels/` und `direkt/` (`src/gespraeche/logablage.rs:102–108`); ein Channel namens `logs` liegt unter `channels/logs/` — **kein Konflikt** mit `<ordner>/logs/`. Das Löschen der Fristpflege greift nur unter `channels/` (ADR-0032).
- **Begriff**: im Projekt heißt „Log“ das Tageslog eines Gesprächs (ADR-0004, `log_bytes` im Health-Check). Damit beides nicht verwechselt wird, heißt das Neue im Modell und im Code **Betriebsprotokoll** (`protokoll`), der Ordner bleibt nach Vorgabe `logs/`.
- **Zeit** ([X00007](../Architektur/Aspekte/X00007-zeit.md)): „jetzt“ nur über den Uhr-Provider; Tagesdateien nach **lokalem** Datum; Testuhr `LOCALCHATA_JETZT`, in Tests `TZ=Europe/Berlin`.
- **Sperre**: die Sperrdatei verhindert zwei Instanzen auf einem Ordner (A00003). Eine zweite Instanz, die vor der Sperre schon ins Protokoll schriebe, würde in das Protokoll der ersten schreiben.

## Optionen

### Ziel

#### Option A: Datei `logs/JJJJ-MM-TT.log` nach lokalem Datum, stderr spiegelt Warning und höher
- Vorteile: das Protokoll liegt neben dem, was es beschreibt, und wandert mit dem Ordner; dasselbe Datum wie die Tageslogs — wer „was war am 27.?“ fragt, öffnet zwei Dateien gleichen Namens; der Mensch am Terminal sieht Probleme weiter sofort; Tests, die heute stderr lesen, behalten ihre Zeilen für Warning und höher.
- Nachteile: Warning und höher stehen doppelt, wenn stderr ebenfalls in eine Datei umgeleitet wird (Dienstbetrieb).

#### Option B: Nur Datei, stderr nur noch für Startfehler vor der Sperre
- Vorteile: eine Quelle, keine Dopplung im Dienstbetrieb.
- Nachteile: wer im Terminal startet, sieht einen Plattenfehler erst, wenn er die Datei liest; Ladedefekte beim Start verschwinden aus dem Blick.

#### Option C: Datei nach UTC-Datum, Zeitstempel in UTC
- Vorteile: keine Zeitumstellung im Dateinamen; Standard vieler Bibliotheken (siehe ADR-0041).
- Nachteile: zwischen 00:00 und 01:00/02:00 Ortszeit stehen Einträge in der Datei des Vortags — anders als die Tageslogs der Gespräche; Zeitstempel müssen beim Lesen umgerechnet werden.

### Format

#### Option F1: Eine Textzeile je Eintrag — Zeit, Stufe, Bereich, Text, dann `| schlüssel=wert …`
- Vorteile: mit `less`, `grep`, `tail -f` lesbar wie die Tageslogs (Qualitätsziel Q5 sinngemäß); `grep ' ERROR '` und `grep 'kennung=claude@'` funktionieren; mit einer Escape-Regel eindeutig.
- Nachteile: kein fertiges Schema für Auswertungswerkzeuge; der Kontext braucht eine eigene Quoting-Regel.

#### Option F2: JSON je Zeile (JSON Lines)
- Vorteile: maschinenlesbar, Escape durch JSON gelöst.
- Nachteile: für den Menschen am Morgen schwer zu lesen (Zielgruppe ist der Betreiber, kein Log-Stack im LAN); Umlaute und Text in Anführungszeichen.

#### Option F3: Mehrzeilige Einträge (Kopfzeile, eingerückte Fortsetzung wie ADR-0004)
- Vorteile: lange Ursachen und Panik-Texte bleiben lesbar.
- Nachteile: `grep` findet nur Teile eines Eintrags; der Prüfer verlangt „eine Zeile je Eintrag“.

## Entscheidung

**Gewählt: Ziel A, Format F1**

Es überwiegt, dass das Protokoll wie der übrige Ordner ohne Werkzeug lesbar ist und dasselbe Datum trägt wie die Gespräche, und dass der Mensch am Terminal Probleme weiter sofort sieht. In Kauf genommen: im Dienstbetrieb stehen Warning und höher doppelt (Datei und Journal), und Auswertungswerkzeuge brauchen einen eigenen Parser für F1.

Regeln:

1. **Ort**: `<ordner>/logs/`, angelegt beim Start **nach** der Sperre ([A00003](../Architektur/A00003-start.md)); vor der Sperre (Aufruf, Ordner, Sperre selbst, Umgebungswerte) bleibt es bei stderr und den Exit-Codes von heute — ein gesperrter Ordner gehört einer anderen Instanz.
2. **Datei** `logs/JJJJ-MM-TT.log`; das Datum ist das **lokale Datum der Eintragszeit** aus dem Uhr-Provider. Ein Eintrag um `23:59:59.999` steht in der Datei des Tages, einer um `00:00:00.000` in der des Folgetags; die Zeitumstellung ändert nur, welches Datum gilt (X00007). Geschrieben wird mit `append` und `create`, je Eintrag **ein** `write_all` der ganzen Zeile ohne Puffer — ein Absturz verliert höchstens den Eintrag, der gerade entsteht.
3. **Zeile**: `<zeit> <STUFE> <bereich> <text>[ | <schlüssel>=<wert>]…` und `\n`.
   - `<zeit>`: RFC 3339 mit Millisekunden und Versatz, wie die Nachrichtenzeit — `2026-10-28T14:03:07.123+01:00`.
   - `<STUFE>`: `DEBUG`, `INFORMATION`, `WARNING`, `ERROR`, `EXCEPTION` (Großbuchstaben, ohne Auffüllen).
   - `<bereich>`: ein Wort aus einer festen Liste im Code — `start`, `ende`, `protokoll`, `laden`, `verbindung`, `grenzschutz`, `sitzung`, `channels`, `verlauf`, `senden`, `schreibkonto`, `frist`, `zeit`, `programmfehler`; neue Bereiche kommen mit der Karte, die sie braucht.
   - `<text>`: deutscher Satz ohne Kennungen in Freitext — was geschah und was es bedeutet („Nachricht nicht gesendet — die Tagesdatei ließ sich nicht schreiben“).
   - **Kontext** nach ` | ` als `schlüssel=wert`, getrennt durch ein Leerzeichen; Werte mit Leerzeichen, `=`, `|` oder `"` in `"…"` mit `\"`. Schlüssel aus einer **Erlaubnisliste** im Code: `kennung`, `herkunft`, `channel`, `route`, `status`, `datei`, `zeile`, `grund`, `anzahl`, `dauer_ms`, `grenze`, `ort`, `thread`, `version`, `adresse`, `ordner`, `stufe`, `option`, `seit`; neue Schlüssel kommen mit der Karte, die sie braucht, und mit Begründung.
   - Beispiel: `2026-10-28T14:03:07.123+01:00 ERROR senden Nachricht nicht gesendet — die Tagesdatei ließ sich nicht schreiben | channel=allgemein kennung=shoff@10.0.0.7 datei=channels/allgemein/2026-10-28.log grund="No space left on device (os error 28)"`
4. **Escape (CWE-117)** — für `<text>` und jeden Wert, bevor er in die Zeile kommt: `\` → `\\`; jedes Steuerzeichen (U+0000–U+001F, U+007F–U+009F), die Zeilentrenner U+2028 und U+2029 und die Richtungszeichen U+200E, U+200F, U+202A–U+202E, U+2066–U+2069 → `\u{…}` in Hexadezimal (`\n` → `\u{a}`, ESC → `\u{1b}`). Ungültiges UTF-8 aus Pfaden → `to_string_lossy` vor dem Escape. Längen: ein Wert höchstens 256 Zeichen, `<text>` höchstens 1.024 Zeichen, Kürzung mit `…` am Ende. Damit ist jede Zeile genau ein Eintrag, was immer ein Client als Name, Host oder Pfad schickt.
5. **Was nie ins Protokoll kommt**: Sitzungs-Token und jeder Kopf `X-Sitzung`/`Authorization`/`Cookie`, Nachrichtentexte, Channel-Beschreibungen, Anfragekörper, ganze Kopfzeilen. Routen erscheinen als **Muster** (`POST /api/channels/{name}/nachrichten`) mit dem Namen als eigenem Wert `channel=`; nur bei 404 auf unbekannte Pfade steht der Pfad selbst — escaped und gekürzt. Kennung `name@herkunft` und Herkunft (IP) stehen im Protokoll; sie stehen ohnehin in den Tageslogs (ADR-0004).
6. **stderr**: jeder Eintrag ab `WARNING` erscheint zusätzlich auf stderr, in derselben Zeilenform. stdout bleibt allein der Startzeile (X00001). Die Spiegelung ist unabhängig von der eingestellten Stufe nach unten begrenzt: `DEBUG` und `INFORMATION` gehen nie auf stderr.
7. **Schreibrecht**: nur das Programm schreibt nach `logs/` ([ADR-0005](0005-programm-als-einziger-schreiber-und-reihenfolge.md) sinngemäß); Lesen, Kopieren und Löschen darf der Betreiber jederzeit — fehlt die Datei des Tages, legt der nächste Eintrag sie neu an.
8. **Rückfall**: lässt sich `logs/` nicht anlegen oder eine Zeile nicht schreiben, startet bzw. arbeitet die Instanz weiter; der Eintrag geht dann auf stderr (alle Stufen ab der eingestellten), dazu **eine** Zeile je Episode `WARNING protokoll Betriebsprotokoll nicht schreibbar — Einträge gehen bis auf Weiteres nur auf stderr | datei=… grund=…`, und der Health-Check zeigt es ([ADR-0043](0043-protokollstufe-einstellen-und-kennzahlen-im-health-check.md)). Die nächste gelungene Zeile beendet die Episode mit einer `INFORMATION`-Zeile. Grund: ein Chat, der wegen seines Protokolls nicht startet, verfehlt das Ziel des Protokolls.

## Konsequenzen

**Positiv:**
- `tail -f logs/2026-10-28.log`, `grep ' ERROR \| EXCEPTION ' logs/*.log` und `grep 'kennung=claude@' logs/*.log` beantworten die Morgenfrage „was, wann, bei wem, warum“ ohne Code.
- Rotation und Bereinigung laufen über den Uhr-Provider und sind mit `LOCALCHATA_JETZT` prüfbar ([ADR-0041](0041-betriebsprotokoll-als-eigenes-modul-rotation-beim-schreiben-bereinigung-14-tage.md)).
- Die Grenzregel „keine Inhalte, keine Token“ ist an zwei Stellen erzwungen: Erlaubnisliste der Schlüssel und Escape in einem Formatierer.

**Negativ / Inkauf zu nehmen:**
- `grep -r` über den ganzen Ordner findet nun auch Protokollzeilen (Kennungen, Channelnamen) — nie Nachrichtentexte.
- Mehrzeilige Panik-Texte werden zu einer langen, escapten Zeile.
- Im Dienstbetrieb stehen Warning und höher doppelt.

**Pre-Mortem (ein Monat nach dem Start gescheitert, weil …):**
1. … ein Client einen Namen mit `\n2026-10-28T… ERROR …` schickt und das Protokoll eine gefälschte Zeile zeigt. Früh-Indikator: Unit-Test des Formatierers mit `\n`, `\r`, ESC, U+2028, U+202E. Gegenmaßnahme: Regel 4 an genau einer Stelle (Formatierer), Kontext nur über typisierte Werte.
2. … ein Token im Protokoll landet, weil jemand „zur Diagnose“ Anfrageköpfe protokolliert. Früh-Indikator: Integrationstest `grep <token> logs/*.log` = 0 nach Beitreten, Senden, Abweisung. Gegenmaßnahme: Regel 5, Erlaubnisliste der Schlüssel, Review-Punkt des `observability-reviewer`.
3. … die Platte voll läuft, weil Debug vergessen wurde. Früh-Indikator: `protokoll_bytes_heute` im Health-Check. Gegenmaßnahme: Tagesgrenze ([ADR-0042](0042-stufen-ereigniskatalog-episoden-und-tagesgrenze-des-betriebsprotokolls.md) Regel 6), Bereinigung nach 14 Tagen.

**Folgeentscheidungen / Aufgaben:**
- [ ] Erster Unit-Test (Formatierer, rein): Eintrag `Stufe::Warning`, Bereich `grenzschutz`, Text „Anfrage mit fremdem Host abgewiesen“, Kontext `herkunft=10.0.0.9`, `grund="Host böse\nZEILE\u{1b}[31m\u{2028}x"`, Zeit `2026-10-28T14:03:07.123+01:00` → genau eine Zeile `2026-10-28T14:03:07.123+01:00 WARNING grenzschutz Anfrage mit fremdem Host abgewiesen | herkunft=10.0.0.9 grund="Host böse\u{a}ZEILE\u{1b}[31m\u{2028}x"` und ein `\n`. Ersetzt: nichts.
- [ ] Unit (Datei, gestellte Uhr, Wegwerf-Ordner): Einträge um `2026-10-28T23:59:59.999+01:00` und `2026-10-29T00:00:00.000+01:00` → je eine Zeile in `logs/2026-10-28.log` bzw. `logs/2026-10-29.log`; über die Rückstellung `2026-10-25T02:30:00.000+02:00` und `2026-10-25T02:10:00.000+01:00` → beide in `logs/2026-10-25.log` in Schreibreihenfolge. Ersetzt: Uhr.
- [ ] Integrationstest (echtes Programm): Start mit `LOCALCHATA_JETZT=2026-10-28T12:00:00+01:00`, `TZ=Europe/Berlin` → `logs/2026-10-28.log` existiert nach der Startzeile und enthält eine `INFORMATION start`-Zeile mit `version=`, `adresse=`, `ordner=`, `stufe=information`; ein Channel `logs` lässt sich anlegen und liegt unter `channels/logs/`, `logs/` bleibt unberührt.
- [ ] `AI-Team/umgebung.md` (Director): Zeile „Betriebsprotokoll `<ordner>/logs/JJJJ-MM-TT.log`“ ergänzen, sobald gebaut.

## Quellen / Bezuege

- [A00015 Betriebsprotokoll](../Architektur/A00015-betriebsprotokoll.md), [A00003 Start](../Architektur/A00003-start.md), [A00008 Logablage](../Architektur/A00008-logablage.md); Aspekte [X00001 Logging und Diagnose](../Architektur/Aspekte/X00001-logging-und-diagnose.md), [X00007 Zeit](../Architektur/Aspekte/X00007-zeit.md), [X00004 Fehlerbehandlung](../Architektur/Aspekte/X00004-fehlerbehandlung.md)
- ADR-0003, ADR-0004, ADR-0005, ADR-0029 (Kalendertage), ADR-0030, ADR-0032; ADR-0041 bis ADR-0043
- `AI-Team/umgebung.md` „Logging-Konzept“ (Betreiber 2026-10-28); Factory-Rolle `observability-reviewer` (Prüfraster, Einstufung)
- CWE-117 (Improper Output Neutralization for Logs) — aus dem Gedächtnis, in diesem Auftrag nicht geöffnet; Unicode-Zeilentrenner U+2028/U+2029 und Richtungszeichen (CVE-2021-42574 „Trojan Source“) — aus dem Gedächtnis
