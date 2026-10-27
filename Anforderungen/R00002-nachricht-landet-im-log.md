---
id: R00002
status: Neu
datum: 2026-10-27
---

# R00002: Nachricht landet im Log

## Beschreibung

Jede Nachricht steht mit Absender und Zeit als lesbarer Satz im Tageslog ihres Channels, und eine Instanz findet beim Start alles wieder, was in der Frist liegt. Der Betreiber sieht nach dem Start im Health-Check, wie viele Nachrichten geladen wurden, wie groß die geladenen Logs sind und ob Zeilen defekt waren; jede defekte Zeile steht mit Datei und Zeile auf stderr. Darunter liegt der Schreibweg, über den ab I0009 jede gesendete Nachricht läuft: Zeit vergeben, Satz bilden, an die Tagesdatei des lokalen Datums anhängen (Rotation beim ersten Satz eines neuen Tages), erst dann in den Index — bei einem Schreibfehler bleibt alles, wie es war.

Diese Anforderung baut **keine** schreibende HTTP-Route: eine Route, die in den Ordner schreibt, entsteht erst mit Sitzungsprüfung und Host-/Origin-Prüfung (Grenzregel in A00004). Von außen sichtbar ist in R00002 das Laden; der Schreibweg wird in Modul- und Integrationstests geprüft und über den Rundweg „senden → Neustart → Health-Check“ an der Grenze bestätigt.

Zahlt ein auf: [Vision](R00000-vision.md) — „Das Log ist das Gedächtnis“ und „Ein Chat, der ein Ordner ist“.

Slices (WBS `Dokumentation/Planung/localchata.md`): F0004 Verlauf beim Start laden, F0005 Nachricht an das Tageslog anhängen — beide unter I0002 Nachricht landet im Log.

## Geschäftlicher Nutzen

Ohne Log gibt es kein Gedächtnis: jede Nachricht wäre mit dem Ende der Instanz verloren, und kein Agent könnte nachlesen, was vor seinem Beitritt besprochen wurde. Mit R00002 liegt der Verlauf als Textdateien im Ordner, die ein Mensch mit `less` und ein Agent mit `grep` lesen kann, und die Instanz baut daraus beim Start ihren Index. Die Kennzahlen im Health-Check sind der Früh-Indikator für das Leistungsziel (100.000 Nachrichten, ADR-0006), und die gemeldeten defekten Zeilen machen Schäden sichtbar statt sie still zu schlucken. Der Schreibweg mit Rückbau sorgt dafür, dass eine bestätigte Nachricht nach einem Neustart genau so da ist und eine gescheiterte nirgends.

## Funktionale Anforderungen

- **Ablage** (ADR-0003): ein Gespräch ist ein Ordner unter `<ordner>/channels/<slug>/` oder `<ordner>/direkt/<name>/`; seine Tage sind die Dateien `JJJJ-MM-TT.log` darin. Andere Dateien werden ignoriert. `einstellungen.toml` ist optional und wird in R00002 nicht gelesen; es gilt die Standardfrist 7 Tage.
- **Logformat** (ADR-0004): ein Satz beginnt mit der Kopfzeile `@ <zeit> <kennung>` (Zeit RFC 3339 mit Millisekunden und Versatz, Kennung `name@herkunft`), danach Metazeilen `+ <schluessel>: <wert>` (gelesen und erhalten, in R00002 nicht geschrieben), danach die Textzeilen mit zwei Leerzeichen eingerückt; eine leere Textzeile ist eine leere Zeile; der Schreiber setzt nach jedem Satz eine Leerzeile.
- **Defekte Zeilen**: eine Zeile, die weder mit `@ `, `+ ` noch zwei Leerzeichen beginnt und nicht leer ist, ist defekt; ebenso eine Kopfzeile mit unlesbarer Zeit oder ungültiger Kennung (dann zählt der ganze Satz als **ein** Defekt mit der Zeile der Kopfzeile). Defekte werden beim Laden übersprungen und gemeldet, nie still ersetzt.
- **Kennung**: nicht leer, ohne Leer- und Steuerzeichen, genau ein `@` zwischen Name und Herkunft; `:` in einer IPv6-Herkunft ist erlaubt. Die Namensregel selbst (welche Namen erlaubt sind, Eindeutigkeit) ist I0004.
- **Laden beim Start**: nach der Ordner-Sperre und vor dem Binden lädt die Instanz je Gespräch die Tage in der Frist. **Heute zählt mit**: bei Frist 7 und heute = 2026-10-27 (lokales Datum) werden die Tage 2026-10-21 bis 2026-10-27 geladen, 2026-10-20 nicht. Tage nach heute (z. B. nach einer zurückgestellten Uhr) werden mitgeladen. Ältere Tage bleiben unangetastet liegen — das Entfallen baut I0018.
- **Kennzahlen**: `GET /api/gesundheit` meldet `nachrichten` (geladene und seitdem angehängte Sätze), `log_bytes` (Summe der Bytes aller geladenen Tagesdateien samt Angehängtem), `ladezeit_ms` (Dauer des Ladens) und `defekte_zeilen` (Zahl der Defekte beim Laden). Im leeren Ordner sind alle vier 0 wie in R00001.
- **stderr**: je Defekt genau eine Zeile, die Datei relativ zum Ordner und die Zeilennummer im Format `<datei>:<zeile>` enthält (z. B. `channels/allgemein/2026-10-27.log:4`). stdout bleibt die eine Startzeile (X00001).
- **Uhr**: „jetzt“ kommt nur aus der Uhr (X00007). Ist `LOCALCHATA_JETZT` gesetzt (RFC 3339 mit Versatz), gilt dieser Zeitpunkt als Startzeit; ein unlesbarer Wert beendet den Start mit Exit 2 und einer Zeile auf stderr, die `LOCALCHATA_JETZT` nennt. Das lokale Datum folgt der Zeitzone des Prozesses (`TZ`).
- **Senden** (Schnittstelle von Gespräche, A00006; aufgerufen ab I0009/I0015): `senden(gespraech, von, text)` in ein beim Laden gefundenes Gespräch vergibt unter der Sperre des Gesprächs die Zeit `max(jetzt, letzte + 1 ms)` — global über alle Gespräche, beim Start fortgesetzt beim jüngsten Satz im ganzen Ordner, verglichen als Zeitpunkt, nie als Zeichenkette —, bildet den Satz, hängt ihn in **einem** Schreibaufruf an die Tagesdatei des lokalen Datums der Nachrichtenzeit an und ergänzt erst danach den Index. Ein unbekanntes Gespräch ergibt den Fehler „unbekannt“; angelegt wird nichts (Anlegen ist I0007).
- **Rotation**: der erste Satz eines neuen lokalen Tages öffnet dessen Tagesdatei; einen eigenen Mitternachts-Takt gibt es nicht.
- **Rückbau und Zeilenende-Wächter** (ADR-0011): scheitert das Anhängen nach einem Teil der Bytes, wird die Datei auf die Länge vor dem Schreiben zurückgeschnitten, `senden` liefert einen Fehler, der Index bleibt unverändert, die vergebene Zeit verfällt. Endet eine Tagesdatei beim Öffnen zum Anhängen (oder nach einem gescheiterten Rückbau) nicht auf `\n`, beginnt der nächste Schreibvorgang mit `\n`. Scheitert der Rückbau selbst, steht eine Zeile mit Datei und Länge auf stderr.

## Nicht-funktionale Anforderungen

- Leistung: Laden von 100.000 Nachrichten (≈ 53 MB) bleibt unter 2 s Startzeit (ADR-0006, gemessen 0,43 s in der Probe); der Leistungstest selbst gehört zu I0008/I0011, R00002 meldet `ladezeit_ms` als Früh-Indikator.
- Lesbarkeit ohne localchatA: jede geschriebene Tagesdatei ist mit `cat`/`less` als Chat-Protokoll lesbar (ADR-0004).
- Diagnose: Defekte und gescheiterte Rückbauten erscheinen auf stderr; nichts wird still verworfen (X00004).
- Testbarkeit: Uhr, Logablage und Startzeit sind ersetzbar (Uhr-Attrappe, Speicher-Logablage mit „Fehler nach n Bytes“, `LOCALCHATA_JETZT`); Tests laufen mit `TZ=Europe/Berlin`, `--port 0` und Wegwerf-Ordnern.

## Akzeptanzkriterien

Gemeinsamer Ausgangsordner **A** (für F0004, `TZ=Europe/Berlin`):

`channels/allgemein/2026-10-26.log` (128 Bytes, 2 Sätze):

```text
@ 2026-10-26T09:00:00.000+01:00 shoff@127.0.0.1
  Guten Morgen

@ 2026-10-26T09:01:00.000+01:00 claude@10.0.0.5
  Scan läuft.

```

`channels/allgemein/2026-10-27.log` (127 Bytes, 2 Sätze, Zeile 4 defekt):

```text
@ 2026-10-27T10:00:00.000+01:00 shoff@127.0.0.1
  Wie weit?

kaputt
@ 2026-10-27T10:00:05.000+01:00 claude@10.0.0.5
  Fertig.

```

### F0004 Verlauf beim Start laden

- [ ] Start auf Ordner A mit `LOCALCHATA_JETZT=2026-10-27T12:00:00+01:00 localchatA <A> --port 0 --host 127.0.0.1 --kein-browser`: `/api/gesundheit` → 200 mit `"nachrichten":4`, `"log_bytes":255` (128 + 127, gleich der Summe aus `wc -c`), `"defekte_zeilen":1` und `"ladezeit_ms"` als Zahl ≥ 0.
- [ ] Beim selben Start steht auf stderr genau eine Zeile zu Defekten, und sie enthält `channels/allgemein/2026-10-27.log:4`; stdout enthält nur die Startzeile.
- [ ] Frist-Grenze, heute zählt mit: Ordner mit je einem Satz in `channels/allgemein/2026-10-21.log` und `2026-10-20.log`, Start mit `LOCALCHATA_JETZT=2026-10-27T12:00:00+01:00` → `"nachrichten":1`, `"log_bytes"` = Größe von `2026-10-21.log`; `2026-10-20.log` liegt nach dem Start unverändert im Ordner (Rechnung: 27 − 6 = 21 ist der älteste geladene Tag).
- [ ] Ein Satz in `direkt/claude@10.0.0.5+shoff@127.0.0.1/2026-10-27.log` wird mitgezählt (Ordner A plus diese Datei mit einem Satz → `"nachrichten":5`).
- [ ] Eine Kopfzeile mit unlesbarer Zeit (z. B. `@ 2026-10-27T14:00:0@ 2026-10-27T14:00:00.002+01:00 c@127.0.0.1` mit einer Textzeile, zwischen zwei gültigen Sätzen) zählt als ein Defekt mit der Zeile der Kopfzeile; die beiden gültigen Sätze werden geladen (`"nachrichten":2`, `"defekte_zeilen":1`).
- [ ] Dateien, die nicht `JJJJ-MM-TT.log` heißen (z. B. `notizen.txt`, `einstellungen.toml`), werden weder gezählt noch gemeldet.
- [ ] `LOCALCHATA_JETZT=gestern localchatA <A> --port 0 --kein-browser` → Exit 2, eine Zeile auf stderr nennt `LOCALCHATA_JETZT`.
- [ ] Leerer Ordner: alle Kennzahlen 0, keine Zeile auf stderr (R00001 bleibt grün).

### F0005 Nachricht an das Tageslog anhängen

Prüfweg bis I0009: je Kriterium ein Modul- oder Integrationstest gegen die Bibliothek (Speicher-Logablage bzw. Datei-Logablage im Wegwerf-Ordner, Uhr-Attrappe). Die Tests zu den User-Story-Szenarien US-4 bis US-8 tragen den Präfix `us<n>_` im Namen, damit `cargo test us<n>_` genau sie fährt.

- [ ] `senden("allgemein", claude@10.0.0.5, "# Titel\n\n@ kein Kopf\n\tTab\r\nEnde\n\n")` mit Uhr fest auf `2026-10-27T14:07:12.345+01:00` hängt an `channels/allgemein/2026-10-27.log` genau diese Bytes an: `@ 2026-10-27T14:07:12.345+01:00 claude@10.0.0.5\n  # Titel\n\n  @ kein Kopf\n  \tTab\n  Ende\n\n` (Text mit zwei Leerzeichen eingerückt, `\r\n` → `\n`, Tab erhalten, abschließende Leerzeilen entfallen, Leerzeile als Abschluss).
- [ ] Rundweg: drei `senden` in einen Ordner, danach startet das echte Programm auf diesem Ordner → `/api/gesundheit` meldet `"nachrichten":3` und `"defekte_zeilen":0`; die Zeile `@ kein Kopf` im Text erzeugt keinen eigenen Satz.
- [ ] Gleiche Millisekunde: Uhr fest auf `2026-10-27T14:07:12.345+01:00`, zwei `senden` → Zeiten `…14:07:12.345+01:00` und `…14:07:12.346+01:00`, in dieser Reihenfolge in der Datei.
- [ ] Uhr zurückgestellt: erst `14:07:12.345`, dann `14:00:00.000` (beide `+01:00`) → zweite Nachricht trägt `14:07:12.346+01:00`.
- [ ] Zeitumstellung, als Zeitpunkt verglichen: Uhr erst `2026-10-25T02:59:59.999+02:00`, dann `2026-10-25T02:00:00.000+01:00` → die zweite Nachricht behält `2026-10-25T02:00:00.000+01:00` (Rechnung: 00:59:59.999Z → 01:00:00.000Z ist 1 ms später), beide stehen in `2026-10-25.log` in Sendereihenfolge.
- [ ] Rotation: Uhr `2026-10-27T23:59:59.999+01:00` → Satz in `2026-10-27.log`; Uhr `2026-10-28T00:00:00.000+01:00` → Satz in neuer Datei `2026-10-28.log`; `2026-10-27.log` ist nach dem zweiten Senden byte-gleich wie davor.
- [ ] Fortsetzung beim Start: Ordner mit jüngstem Satz `2026-10-27T14:00:00.000+01:00` in `channels/b/2026-10-27.log` und leerem Ordner `channels/a/`, nach `laden` mit Uhr fest auf `2026-10-27T13:00:00.000+01:00` → das erste `senden` in `a` trägt `2026-10-27T14:00:00.001+01:00`.
- [ ] Unbekanntes Gespräch: `senden("gibtsnicht", …)` → Fehler „unbekannt“; es entsteht weder `channels/gibtsnicht/` noch eine Datei.
- [ ] Zeilenende-Wächter: `2026-10-27.log` endet ohne `\n` auf `  Rest` (ein Satz) → nach `senden` beginnt der neue Satz in einer neuen Zeile; ein Neustart meldet `"nachrichten":2`, `"defekte_zeilen":0`.
- [ ] Schreibfehler (Speicher-Logablage, Fehler nach 20 Bytes): `senden` liefert einen Fehler, der Inhalt hat danach dieselbe Länge wie davor, die Nachrichtenzahl des Index ist unverändert; das nächste `senden` gelingt, trägt eine spätere Zeit als die verfallene und steht heil in der Datei.
- [ ] Schreibfehler an einer echten Datei (Teilschreiben erzwungen, z. B. `RLIMIT_FSIZE` im Kindprozess): die Tagesdatei hat danach die Länge vor dem Schreiben.
- [ ] Reihenfolge unter Last: 20 Tasks senden je 50 Nachrichten gleichzeitig in denselben Channel → 1000 Sätze in der Datei, ihre Zeiten in Dateireihenfolge streng steigend, keine doppelt; `"nachrichten"` nach einem Neustart 1000.

## Betroffene Verzeichnisstruktur

- Neu: `src/uhr.rs` (Uhr-Provider, X00007); `src/gespraeche/` mit `mod.rs` (Gespräche: `laden`, `senden`), `logformat.rs` (A00007), `logablage.rs` (A00008, Port mit Datei- und Speicher-Implementierung), `verlaufsindex.rs` (A00009), `kennung.rs` oder im Logformat (Entscheidung des Entwicklers).
- Geändert: `src/start/mod.rs` (Laden zwischen Sperre und Binden, Defekte auf stderr, `LOCALCHATA_JETZT` → Exit 2), `src/http/mod.rs` (Zustand trägt die Gespräche), `src/http/gesundheit.rs` (Kennzahlen statt 0), `src/fehler.rs` falls der Fehlertyp hier entsteht (X00004).
- Tests: Unit im jeweiligen Modul, Integration unter `tests/` mit `tests/gemeinsam/mod.rs` (Umgebung `LOCALCHATA_JETZT`, `TZ` durchreichen).

## Technische Überlegungen

### Ablauf

1. **Start** (A00003): Aufruf lesen → Uhr aus `LOCALCHATA_JETZT` oder Systemuhr (unlesbar → Exit 2) → Ordner vorbereiten → sperren → **laden** → binden → Signale → Startzeile → Browser → ausliefern.
2. **Laden** (A00006): Ablage durchsuchen → je Gespräch Tage in der Frist wählen (heute − 6 bis heute und später) → Tag lesen → Sätze und Defekte → Index aufnehmen → jüngste Zeit (als Zeitpunkt) merken → Defekte auf stderr.
3. **Senden** (A00006, ADR-0005, ADR-0011): Sperre des Gesprächs → Zeit vergeben → Satz bilden → Tag = lokales Datum der Zeit → anhängen (Wächter, Rückbau) → bei Erfolg Index ergänzen → Nachricht; bei Fehler Zeit verfallen lassen.

### Grobentwurf (Vorplanung)

Die Bubbles aus der WBS als Datenfluss — **Vorplanung**: welche es am Ende werden, entscheidet der Entwickler beim Bauen.

- F0004: B0018 Uhr mit Startzeit aus der Umgebung (`LOCALCHATA_JETZT` | Systemuhr → `Uhr` | `Aufruffehler`) · B0019 Kennung bilden (Text → `Kennung` | ungültig) · B0020 Sätze lesen (Rohtext → Sätze und `Defekt {zeile}`) · B0021 Ablage durchsuchen (Ordner → Gespräche mit Tagen; `lesen` → Rohtext) · B0022 Tage in der Frist wählen (Tage, heute, 7 → Tage ab heute − 6) · B0023 Verlauf aufnehmen (Rohtext, Sätze → Index; Kennzahlen) · B0024 Ordner laden (Logablage, Uhr → `Gespraeche` mit Index, Defekten, jüngster Zeit, Ladezeit) · B0025 Laden in den Start einhängen (nach der Sperre, vor dem Binden; stderr; Gesundheit aus dem Index)
- F0005: B0026 Satz schreiben (`Nachricht` → Satztext) · B0027 Zeit vergeben (jetzt, letzte → `max(jetzt, letzte + 1 ms)`) · B0028 Satz an die Tagesdatei anhängen (ein `write_all`, Rotation, Zeilenende-Wächter) · B0029 Teilschreiben zurückbauen (Fehler nach n Bytes → alte Länge | stderr) · B0030 Speicher-Logablage (Port im Speicher, „Fehler nach n Bytes“) · B0031 Nachricht senden (Gespräch, Kennung, Text → `Nachricht {zeit}` | unbekannt | Schreibfehler)

**Elemente** (nur öffentliche Signaturen, Rust, nach A00006–A00009):

- `Uhr` (Trait) mit `fn jetzt(&self) -> Zeit`; Produktion Systemuhr mit lokaler Zone, Start aus `LOCALCHATA_JETZT`; Test fest/verschiebbar. Ob die Startuhr ab `LOCALCHATA_JETZT` weiterläuft oder steht, entscheidet der Entwickler — R00002 verlangt nur, dass „heute“ beim Laden das lokale Datum dieses Zeitpunkts ist.
- `Zeit` — Zeitpunkt mit Versatz, Ordnung nach Millisekunden seit Epoche; `Kennung` — geprüfter Text.
- `Nachricht { zeit, von: Kennung, text }` (unveränderlich).
- Logformat: `fn satz(&Nachricht) -> String`, `fn saetze(&str) -> impl Iterator<Item = Result<SatzBereich, Defekt>>`, `fn text(&str) -> String`.
- Logablage (Trait): `gespraeche()`, `tage(gespraech)`, `lesen(gespraech, tag)`, `anhaengen(gespraech, tag, satz)`; Implementierungen Datei und Speicher.
- Gespräche: `fn laden(logablage, uhr) -> Gespraeche`, `fn senden(&self, gespraech, von, text) -> Result<Nachricht, Fehler>`, Kennzahlen für den Health-Check.

Wireframe: I0002 hat keine Oberfläche; `D0001.dc.html` fehlt weiterhin (Hinweis `/wireframe verfeinern D0001`, betrifft R00002 nicht).

### Änderungen an bestehenden Klassen

- `start::starten`: Uhr bilden und Laden zwischen Sperre und Binden; `Aufruffehler` für `LOCALCHATA_JETZT` (Exit 2 wie ein falscher Aufruf).
- `http::Zustand`: trägt zusätzlich die geladenen Gespräche (geteilt, z. B. `Arc`).
- `http::gesundheit::beantworten`: Kennzahlen aus Gespräche statt fest 0; der Unit-Test „leerer Ordner → 0“ bleibt.

## Tests

Nach Skill `test-pyramide` und ADR-0009; die Szenarien der User Story werden Tests.

**Unit (rein, ohne Platte):** Logformat (Rundweg `satz` → `saetze` → `text` mit `@ kein Kopf`, `\r\n`, Tab, abschließenden Leerzeilen; defekte Zeile; Kopf mit unlesbarer Zeit); `Kennung` (gültig: `claude@10.0.0.5`, `claude@::1`; ungültig: leer, `cla ude@x`, `claude`, `a@b@c`); Zeit vergeben (gleiche ms, Uhr zurück, Zeitumstellung); Tage in der Frist (21 drin, 20 draußen, 28 drin bei heute 27); Verlaufsindex-Zählung; Gespräche mit Speicher-Logablage und Uhr-Attrappe (gleiche ms, Fehler nach n Bytes, unbekanntes Gespräch, Fortsetzung der Zeit).

**Integration (`tests/`, echtes Programm bzw. Datei-Logablage gegen `tempfile`):** Laden von Ordner A über das echte Programm mit `LOCALCHATA_JETZT` und `TZ=Europe/Berlin` (Kennzahlen, stderr); Frist-Grenze; `LOCALCHATA_JETZT` unlesbar → Exit 2; Datei-Logablage: Anhängen, Rotation, Zeilenende-Wächter, Rückbau an echter Datei; Rundweg senden → echtes Programm → Health-Check; 20 × 50 gleichzeitig.

**E2E:** keine neuen — R00002 ändert die Oberfläche nicht; der bestehende E2E-Lauf bleibt grün.

## Abhängigkeiten

- Abhängig von: I0001 Instanz starten (R00001, grün) — Start, Sperre, Health-Check, Testhilfe.
- Blockiert: I0009 Nachricht senden (Sendeweg an der Grenze nutzt `senden`), I0018 Aufbewahrungsfrist je Channel festlegen (Entfallen auf derselben Tages-Regel); innerhalb: F0005 braucht F0004 (Fortsetzung der Zeit und Rundweg brauchen das Laden).

## Offene Fragen

Keine, die R00002 blockieren. Entschieden mit Empfehlung (Karte #2):

- Heute zählt zur Frist: Frist 7 = heute und die 6 Vortage nach lokalem Datum (genau 7 Tagesdateien, wie in der Probe von ADR-0006). Der Wortlaut in ADR-0003 („älter als frist_tage Kalendertage vor heute“) lässt beide Lesarten zu; angleichen ist Sache des architekt, das Entfallen baut I0018.
- `einstellungen.toml` wird in R00002 nicht gelesen; die Standardfrist 7 gilt für alle Gespräche, bis I0007/I0018 Einstellungen schreiben.
- Defekte auf stderr als `<datei>:<zeile>` mit Pfad relativ zum Ordner.
- Unlesbares `LOCALCHATA_JETZT` ist ein falscher Aufruf: Exit 2.

Weiter offen beim Betreiber, ohne R00002 zu blockieren: die Richtungsfrage aus ADR-0005, ob Dritte an den Dateien schreiben dürfen. R00002 baut Option A (das Programm ist einziger Schreiber); Option B wäre eine eigene Interaction.

## Manuelle Vorbereitungstätigkeiten

- Keine über `AI-Team/umgebung.md` hinaus.

## Manuelle Nachbereitungstätigkeiten

- `AI-Team/umgebung.md` um `LOCALCHATA_JETZT` und `TZ=Europe/Berlin` für Tests ergänzen (Director).

## Umfang

Aus der WBS gezählt (`/planung zaehlen`); belegte Werte aus `Schaetzungen/_ist-zeiten.md` (R00001, Typ Standard, `bestaetigt` 0,01–0,05h) auf die Untergrenze 0,4h gehoben:

- F0004 Verlauf beim Start laden = 8 Bubbles: 8 Standard (0,4h). Rest: 3,2h klar · 8 von 8 Werten belegt.
- F0005 Nachricht an das Tageslog anhängen = 6 Bubbles: 5 Standard (0,4h), 1 unklar (2–4h). Rest: 2h klar + 2–4h unklar · 5 von 6 Werten belegt, Rest Richtwerte (ungemessen).

## Warum löst diese Anforderung das Problem? (Pflicht)

Auslöser: Nach I0001 läuft eine Instanz, aber sie vergisst alles — es gibt weder Logs noch Kennzahlen, und `/api/gesundheit` meldet fest 0. Wenn der Start den Ordner liest, die Tage der Frist in den Index lädt und Zahl, Größe und Defekte meldet, dann ist das Gedächtnis von außen überprüfbar, bevor überhaupt jemand senden kann, und jeder Schaden an einer Datei wird mit Datei und Zeile sichtbar. Wenn zugleich der Schreibweg mit monotoner Zeit, Rotation nach lokalem Datum, Rückbau und Zeilenende-Wächter im Modul fertig und über den Rundweg bestätigt ist, dann muss I0009 nur noch die Route mit Sitzung und Herkunftsprüfung davorsetzen, statt Format, Reihenfolge und Fehlerfall mitzuerfinden. Der Hebel liegt hier und nicht bei I0009, weil Format und Ablage schwer umkehrbar sind, sobald Bestand auf Platte liegt (ADR-0003), und weil eine vorläufige Schreibroute ohne Sitzung im LAN eine Angriffsfläche wäre (Grenzregel A00004).

## Missing-Docs

- Glossar fehlt (`Dokumentation/Domaenenwissen/` existiert nicht); Begriffe wie Gespräch, Satz, Tagesdatei, Kennung, Frist sind aus ADR-0003/0004 und A00006–A00009 übernommen.
- Wie man ein Teilschreiben an einer echten Datei portabel erzwingt (`RLIMIT_FSIZE` unter Linux; Windows/macOS) ist im Projekt nicht dokumentiert.

## Notizen

### Verworfene Alternativen

- **Vorläufige Schreibroute** `POST /api/channels/<slug>/nachrichten` mit Absender im Body, damit das Senden in R00002 von außen prüfbar ist: verworfen — die Grenzregel in A00004 verbietet schreibende Routen vor Sitzungs- und Host-/Origin-Prüfung, und der Listener auf 0.0.0.0 macht jede solche Route zur LAN-Angriffsfläche (`AI-Team/gelerntes.md`).
- **Schreibweg erst mit I0009, R00002 nur Laden**: verworfen — I0009 müsste dann Route, Sitzung, Live-Kanal, Oberfläche und Log zugleich bauen und würde zu groß; das Fertig-Kriterium von I0002 („jede Nachricht steht mit Absender und Zeit im Log“) bliebe unerfüllt.
- **Senden über einen Unterbefehl der Kommandozeile**: verworfen — eine neue Schnittstelle, die die Vision nicht nennt, und die Ordner-Sperre ließe sie neben einer laufenden Instanz ohnehin nicht schreiben.
- **Je Slice eine eigene Anforderung** (F0004, F0005): verworfen — beide haben ein Ziel (das Log ist das Gedächtnis), teilen Logformat und Index und durchlaufen als Arbeitspaket I0002 gemeinsam Entwicklertest und Abnahme.

### Bewusst außerhalb

Entfallen alter Tage und Frist je Channel (I0018), Lesen und Schreiben von `einstellungen.toml` (I0007/I0018), Channel anlegen (I0007), Blockabruf und Suche (I0008/I0011), Live-Kanal (I0009), Anhänge und Metazeilen schreiben (I0010), Direktgespräche anlegen (I0012), Kennzahlen in der Startzeile (die Startzeile bleibt wie in R00001).
