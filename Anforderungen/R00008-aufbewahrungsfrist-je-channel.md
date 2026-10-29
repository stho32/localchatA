---
id: R00008
status: Neu
datum: 2026-10-28
---

# R00008: Aufbewahrungsfrist je Channel festlegen

## Beschreibung

Jeder Channel hat eine Aufbewahrungsfrist in ganzen Kalendertagen, Standard 7, wählbar von 1 bis 3650; heute zählt mit. Tagesdateien, die älter sind als die Frist, entfallen — beim Start, um Mitternacht in der laufenden Instanz und sofort, wenn jemand die Frist verkürzt. Weil das Löschen endgültig ist, verlangt ein Verkürzen, das Tage löscht, eine Bestätigung mit den Zahlen dessen, was wegfällt; gelöscht werden nur die eigenen Tagesdateien des Programms, nie Symlinks, Notizen oder Sicherungen im Ordner. Ein Agent setzt die Frist mit einem Aufruf über die API, ein Mensch über `# name ▾` in der Kopfzeile; alle anderen sehen die neue Frist live.

Zahlt ein auf: [Vision](R00000-vision.md) — „Das Log ist das Gedächtnis“ („Tage, die älter sind als die Aufbewahrungsfrist des Channels, entfallen. Die Frist ist je Channel wählbar, Standard sind 7 Tage.“), „Mensch und KI sind gleichberechtigt“ (dieselbe Einstellung über Oberfläche und API), „Schnell, auch bei großem Verlauf“ (die Frist begrenzt Index und Speicher, ADR-0006), „lesbar ohne localchatA“ (der Ordner bleibt auch für fremde Dateien sicher).

Entschieden vom architekt (ADR-0029 bis ADR-0033, Vision nicht berührt): Kalendertage, heute zählt mit, 1–3650; Entfallen beim Start, beim Tageswechsel im Sekundentakt und beim Verkürzen, durch Löschen; `PUT /api/channels/{name}/frist` für jeden mit Sitzung, Verkürzen nur mit Bestätigung; Erlaubnisliste fürs Löschen; Dialog „Channel-Einstellungen“ über `# name` in der Kopfzeile.

Slices (WBS `Dokumentation/Planung/localchata.md`, alle unter I0018 Aufbewahrungsfrist je Channel festlegen): F0033 Frist über die API setzen und lesen, F0034 Entfallen beim Start und beim Verkürzen, F0035 Tageswechsel, F0036 Frist live im Strom, F0037 Channel-Einstellungen im Browser, F0038 Löschschutz und Fehlerwege.

## Geschäftlicher Nutzen

Bis R00007 wächst jeder Channel ohne Ende auf der Platte: der Start lädt zwar nur die letzten 7 Tage, aber ältere Dateien bleiben liegen, und eine Instanz, die wochenlang läuft, hält alles seit ihrem Start im Speicher. Ein Channel, in dem zwei Factories täglich Befunde austauschen, braucht vielleicht 30 Tage, ein Spielkanal einen. R00008 macht die Frist der Vision wahr und wählbar — für Agenten mit einem `curl`, für Menschen mit zwei Klicks — und schützt dabei, was nicht weg darf: kein Verkürzen ohne eine Rückmeldung mit Zahlen, kein Löschen außerhalb der eigenen Tagesdateien, kein Löschen bei kaputten Einstellungen.

## Funktionale Anforderungen

### Frist über die API setzen und lesen (F0033; ADR-0029 Regeln 1–3, ADR-0031 Regeln 1–3, 5, 6)

- Jeder Channel hat eine Frist in ganzen Kalendertagen, **1 bis 3650**, Standard **7**: fehlt `einstellungen.toml` oder darin `frist_tage`, gilt 7.
- **Grenze**: der älteste behaltene Tag ist `heute − (Frist − 1)`; heute ist das lokale Datum der Uhr des Servers. Tage danach (auch nach heute) bleiben. Laden und Entfallen rechnen mit derselben Grenze.
- `PUT /api/channels/{name}/frist` mit Sitzung und Körper `{"tage": <Ganzzahl>}` setzt die Frist; jeder mit Sitzung darf das (Vision: keine Rechte).
- **Prüfreihenfolge**: ohne Sitzung 401 · fremder `Origin` 403 · kein JSON-`Content-Type` 415 · zu groß 413 · kein JSON 400 → unbekannter oder ungültiger Channelname 404 `{fehler}` → `tage` fehlt, ist `null`, keine Ganzzahl (`"7"`, `7.0`, `7.5`) oder außerhalb 1–3650 → 422 `{fehler, regel:"frist"}` → Einstellungen des Channels beim Start defekt oder nicht lesbar → 409 `{fehler, regel:"einstellungen-defekt", datei}`, die Datei wird nie überschrieben.
- **Antwort** 200 `{channel, entfallen: {tage, nachrichten}}`; `channel` wie in der Liste. Derselbe Wert wie die geltende Frist → 200 ohne Schreiben.
- **Schreiben**: `einstellungen.toml` bekommt `frist_tage`; alle übrigen Schlüssel (auch unbekannte) bleiben erhalten; geschrieben wird über `einstellungen.toml.neu` und Umbenennen.
- **Darstellung**: jeder Channel trägt in Liste, Antwort, Stand und Ereignis `frist_tage` — die geltende Frist oder `null`, wenn seine Einstellungsdatei defekt ist (`null` heißt: nichts entfällt).
- **Laden**: der Start lädt je Channel die Tage in **seiner** Frist (defekte Einstellungen: geladen wird mit 7). Eine `frist_tage` außerhalb 1–3650 oder mit anderem Typ macht die Datei defekt (Zeile auf stderr, mitgezählt in `defekte_zeilen`, Datei unangetastet).
- **Protokoll**: jede wirksame Änderung schreibt auf stderr `channels/allgemein: Frist 7 → 30 Tage, gesetzt von beta@127.0.0.1`.
- `POST /api/channels` nimmt weiter keine Frist; `frist_tage` steht erst nach der ersten Änderung in der Datei.

### Entfallen beim Start und beim Verkürzen (F0034; ADR-0030 Regeln 1–7, ADR-0031 Regeln 2.6, 2.8, ADR-0032 Regeln 1, 4, 5)

- **Start**: nach dem Laden und **vor** der Startzeile werden in jedem Channel die Tagesdateien jenseits seiner Frist gelöscht. Wer die Startzeile liest, findet sie nicht mehr.
- **Verkürzen**: der Körper darf `"bestaetigt": true|false` tragen (fehlt = `false`; kein Wahrheitswert → 422 `{fehler, regel:"bestaetigt"}`). Würden mit der neuen Frist Tagesdateien entfallen und `bestaetigt` ist nicht `true` → 409 `{fehler, regel:"bestaetigen", entfaellt:{tage, nachrichten, bis}}` (Zahl der Tagesdateien, Zahl ihrer Nachrichten im Index, jüngster entfallender Tag `JJJJ-MM-TT`); nichts wird geändert. `fehler` sagt es dem Agenten in einem Satz, Verb und beide Substantive nach ihrer Zahl gebeugt: „Mit dieser Frist entfällt sofort 1 Tagesdatei mit 1 Nachricht bis 2026-10-21 — mit "bestaetigt":true erneut senden.“ bzw. „Mit dieser Frist entfallen sofort 2 Tagesdateien mit 5 Nachrichten bis 2026-10-21 — …“ (Nacharbeit #12). Mit `bestaetigt: true` wird gesetzt und sofort gelöscht; die 200 nennt in `entfallen` die tatsächlichen Zahlen.
- Verkürzen, das nichts löscht, und Verlängern brauchen keine Bestätigung; Verlängern holt keine entfallenen Tage zurück.
- **Je Tag**: erst die Datei löschen, nur bei Erfolg den Tag aus dem Index nehmen — Ordner und Index zeigen immer dieselben Nachrichten; `nachrichten` und `log_bytes` in `/api/gesundheit` sinken mit.
- **Defekt heißt nichts**: ist die Einstellungsdatei eines Channels defekt oder nicht lesbar, entfällt in ihm nichts; jede Pflege meldet das einmal auf stderr. Eine fehlende Datei ist kein Defekt (Standard 7).
- Nur Channels: `direkt/` wird nicht gepflegt (bis I0012).
- Entfallen erzeugt kein Ereignis und ändert `stand` nicht.
- **Protokoll** auf stderr, eine Zeile je Channel und Pflege, nur wenn etwas entfallen ist oder scheiterte: `channels/allgemein: 2 Tage entfallen (2026-10-20 bis 2026-10-21; Frist 7 Tage; Start)`; beim Verkürzen endet die Klammer mit `Frist gesetzt von <kennung>`.

### Tageswechsel (F0035; ADR-0030 Regel 2 mit Ergänzung „Fristpflege als Tagesaufgabe“, ADR-0041 Ergänzung „Vertrag des Tagestakts“)

- Den Takt stellt der Tagestakt aus R00014 (F0052). Die Fristpflege ist dort die zweite Tagesaufgabe `fristpflege`, nach der Bereinigung des Betriebsprotokolls; einen eigenen Takt der Fristpflege gibt es nicht.
- Der Tagestakt prüft jede Sekunde das lokale Datum der Uhr. Nur wenn es vom Datum der letzten Pflege abweicht (vorwärts oder rückwärts), pflegt er alle Channels; am selben Tag pflegt er nie ein zweites Mal. Das Datum der letzten Pflege ist anfangs das Datum der Pflege beim Start und wird vor jedem Aufruf gesetzt.
- Der Tagestakt beginnt vor der Startzeile und endet mit dem Ende-Signal; die Protokollzeile endet mit `Tageswechsel`.
- Gerät die Pflege in Panik, läuft der Tagestakt weiter, mit der Zeile `ERROR programmfehler Tagesaufgabe fristpflege gescheitert — Tage jenseits der Frist bleiben bis dahin liegen; erneuter Versuch beim nächsten Tageswechsel` (R00014); erneut gepflegt wird beim nächsten Tageswechsel.

### Frist live im Strom (F0036; ADR-0031 Regel 4, ADR-0023)

- Eine wirksame Änderung zählt `stand` um 1 hoch und geht an jeden offenen Strom als `{"art":"channel","stand","channel"}` mit `frist_tage`; derselbe Wert, 404, 409, 422 und 500 erzeugen kein Ereignis.
- Der Stand eines neu geöffneten Stroms trägt `frist_tage` für jeden Channel.

### Channel-Einstellungen im Browser (F0037; ADR-0033; Designsystem `components/ChannelEinstellungen`, `components/Kopfzeile`)

- **Kopfzeile**, Zone „Ort“: `# name` wird ein Schalter `# name ▾` (`<button aria-haspopup="dialog">`, zugänglicher Name „# name — Channel-Einstellungen“); ab 720 px Fensterbreite folgt `· 7 Tage` (`· 1 Tag`), bei `frist_tage: null` `· ⚠ Frist gesperrt` — vor einer Beschreibung; unter 720 px entfällt der Text. Er folgt still jedem Channel-Wechsel aus dem Strom.
- **Dialog** „# name — Einstellungen“ mit dem Feld „Aufbewahrungsfrist“ (Zahl, dahinter „Tage“, vorbelegt mit der geltenden Frist) und dem Hinweis „Ältere Tage entfallen. Heute zählt mit. Standard: 7 Tage.“; „Speichern“ und „Abbrechen“; `Enter` speichert, `Esc` und „Abbrechen“ schließen, der Fokus kehrt zu `# name` zurück.
- **Prüfung vor dem Senden**: nur Ziffern, Wert 1–3650 (führende Nullen erlaubt); sonst Fehlerzeile „Frist: ganze Zahl von 1 bis 3650 Tagen.“ mit `aria-invalid="true"`, kein Abruf.
- **Speichern** sendet ohne Bestätigung; 200 → Dialog zu, Statuszeile „Frist für # name: 30 Tage.“
- **Verkürzen**: 409 `bestaetigen` → zweiter Schritt im selben Dialog: Feld schreibgeschützt, Satz „Bei 7 Tagen entfallen sofort 2 Tage bis einschließlich 21. Oktober 2026 mit 5 Nachrichten. Das lässt sich nicht rückgängig machen.“ (Zahlen deutsch, Datum ausgeschrieben, Einzahl bei 1; das Verb richtet sich nach der Zahl der entfallenden Tage, nicht nach der Frist: „Bei 7 Tagen entfällt sofort 1 Tag …“, „Bei 1 Tag entfallen sofort 2 Tage …“; „1 Nachricht“ bzw. „n Nachrichten“ unabhängig davon); Schalter „Zurück“ (Fokus hier) und „Frist verkürzen“. Der Satz ist die zugängliche Beschreibung von „Zurück“ (oder des Dialogs, oder er steht in `role="alert"`), damit ein Bildschirmleser ihn mit dem Fokus vorliest (WCAG 4.1.3, 1.3.1; Nacharbeit #12). „Zurück“ führt zu Schritt 1 mit der Eingabe; „Frist verkürzen“ sendet mit `bestaetigt: true` → Dialog zu, Statuszeile „Frist für # name: 7 Tage — 2 Tage mit 5 Nachrichten entfallen.“, und der Tab öffnet den Channel neu.
- **Fehler** (409 `einstellungen-defekt`, 404, 422, Netz) → Fehlerzeile im offenen Dialog mit ⚠ in `kd-danger` („Nicht gespeichert — Verbindung prüfen.“ beim Netzfehler); die Eingabe bleibt.
- **Gesperrt**: ist `frist_tage` beim Öffnen `null`, ist das Feld gesperrt, und der Dialog sagt „⚠ Die Einstellungsdatei `channels/name/einstellungen.toml` ist defekt — in diesem Channel entfällt nichts. Von Hand reparieren und localchatA neu starten.“ Dieser Hinweis ist die zugängliche Beschreibung des Dialogs (Nacharbeit #12).
- **Fremde Änderung** bei offenem Dialog: die Eingabe bleibt, der Hinweis ergänzt „Inzwischen geändert: 31 Tage.“ Andere Tabs öffnen nichts neu.

### Löschschutz und Fehlerwege (F0038; ADR-0032 Regeln 1–3, 6, ADR-0030 Regel 4, ADR-0031 Regel 2.7)

- Gelöscht werden nur reguläre Dateien namens `JJJJ-MM-TT.log` (kanonisches, gültiges Datum) in Ordnern gültiger Channels. Nie: Symlinks (auch mit Tagesnamen), Ordner, `2026-10-04.log.bak`, `notizen.txt`, `einstellungen.toml`, alles unter `direkt/`.
- Der Pfad entsteht aus Channelname und Datum; unmittelbar vor dem Löschen wird geprüft, dass dort eine reguläre Datei liegt — sonst wird nichts gelöscht (fehlt sie, gilt der Tag als entfallen).
- Scheitert das Löschen einer Datei, bleibt der Tag in Ordner und Index, eine Zeile auf stderr nennt Datei und Grund, die übrigen Tage und Channels werden weiter gepflegt; erneut versucht wird erst beim nächsten Anlass (Tageswechsel, Neustart, Setzen der Frist), nie in jedem Takt.
- Scheitert beim `PUT` das Schreiben der Einstellungen → 500 `{fehler}`; Frist, Datei und `stand` bleiben, kein Ereignis, eine Zeile auf stderr.

## Nicht-funktionale Anforderungen

- **Zeiten** — die Grenzen stehen nur hier; Kriterien und User Story nennen sie über ihre Nummer, Tests übernehmen sie wörtlich und sind nie schärfer. Die Regeln selbst (welcher Tag entfällt, wann der Takt pflegt, ob ein Ereignis kommt) prüfen die Tests exakt über die gestellte Uhr bzw. über `stand`; die Zeiten unten sind nur Zustellung.

  | # | Was | Art | Startpunkt der Uhr / Messweg | erwartet | Grenze | Zweck |
  |---|---|---|---|---|---|---|
  | Z1 | Friständerung im Strom | Gate (Integration) | Erhalt der 200 beim Setzenden im Testprozess → Ereignis `channel` beim Strom-Leser gelesen | wenige ms (Bus ohne Takt, wie ADR-0023) | 2 s | wer zusieht, sieht die neue Frist ohne Neuladen |
  | Z2 | Friständerung im zweiten Browser | Gate (E2E) | Antwort 200 im Tab A (`waitForResponse`) → Kopfzeile in Tab B zeigt den neuen Text; Prüfintervall ≤ 25 ms | < 0,1 s (gerechnet wie R00006: Strom → Worker → Tab ohne Takt) | 2 s | dasselbe im Browser |
  | Z3 | Tageswechsel in der laufenden Instanz | Gate (Integration) | Startzeile gelesen (Start mit `LOCALCHATA_JETZT=2026-10-28T23:59:58+01:00`) → Datei fehlt; Prüfintervall ≤ 50 ms | 2–3 s (bis Mitternacht ≤ 2 s, dazu höchstens ein Takt von 1 s) | 4 s | eine Instanz, die wochenlang läuft, hält die Frist ohne Neustart |
  | Z4 | Ruhe nach gescheitertem Löschen | Gate (Integration) | nach der Zeile des Tageswechsels 3 s beobachten (≥ 2 weitere Takte) | keine weitere Zeile | keine weitere Zeile | „nie je Takt“: stderr läuft bei einer nicht löschbaren Datei nicht voll |
  | M3 | Wechsel-Flut | **Messwert im Entwicklertest, kein Gate** | ein Agent setzt 10 s lang in einer Schleife abwechselnd 7 und 3650 (eine Verbindung); Zuschauer-Tab mit Seitenleiste: wirksame Wechsel je Sekunde (aus `stand`), längste Aufgabe (`longtask`), Anteil Frames über 50 ms | ungemessen | längste Aufgabe ≤ 250 ms (wie M1 in R00007) | Lehre #5 (CWE-770): `beitreten`/`verlassen` in Schleife ergab 2.456 Wechsel/s und fror einen Zuschauer-Tab 28,7 s ein |

  Reserve: Z1 und Z2 > 20 × über dem erwarteten Wert, Z3 33 % über dem größten erwarteten Wert (3 s); Z4 ist eine Zählung über ein Fenster von drei Takten. Über der Grenze von M3 ist ein Befund an ADR-0031 (Pre-Mortem 2), kein roter Test — ob dann eine Rate je Herkunft kommt, entscheidet der Betreiber (Offene Fragen).
- **Sicherheit**: Setzen nur mit Sitzung, unter Grenzschutz (Host, Origin, Content-Type, Körpergröße); Werte werden abgewiesen, nie umgeformt; Löschen nur nach Erlaubnisliste, Pfad aus Typen, keine Symlinks; kaputte Einstellungen sperren das Löschen; jede wirksame Änderung und jedes Löschen steht mit Kennung auf stderr.
- **Benutzerfreundlichkeit**: Verkürzen nennt vorher Zahl der Tage, Zahl der Nachrichten und das Datum; der sichere Schalter hat den Fokus, und ein Bildschirmleser liest mit dem Fokus vor, was entfällt; Fehler nie nur über Farbe (⚠ und Wort); bei 320 px kein Querscrollen; Fokus kehrt nach dem Dialog zum Auslöser zurück.
- **Testbarkeit**: „heute“ über die gestellte Uhr (`LOCALCHATA_JETZT`, `TZ=Europe/Berlin`); der Tageswechsel ohne echte Mitternacht über eine Startzeit 2 s davor; die Regel des Takts im Tagestakt-Test aus R00014 (Unit mit `tokio::time::pause` und gestellter Uhr).

## Akzeptanzkriterien

Gemeinsam für alle Kriterien, wenn nicht anders genannt: Start mit `TZ=Europe/Berlin LOCALCHATA_JETZT=2026-10-28T12:00:00.000+01:00 localchatA <ordner> --port 0 --host 127.0.0.1 --kein-browser` auf einem Wegwerf-Ordner, `<p>` ist der Port aus der Startzeile; heute ist der 28. Oktober 2026. Tagesdateien entstehen vor dem Start als `channels/<name>/<datum>.log` im Logformat; „`20.(2)`“ heißt: `2026-10-20.log` mit 2 Nachrichten. `einstellungen.toml` mit `frist_tage = 30` heißt: die Datei enthält diese Zeile, dazu `farbe = "blau"` als unbekannter Schlüssel. `S(<name>)` ist der Header `X-LocalchatA-Sitzung` mit dem Token aus dem Beitritt von `<name>` per `curl` (Agent, Herkunft `127.0.0.1`). `PUT` meint `PUT http://127.0.0.1:<p>/api/channels/<name>/frist` mit `Content-Type: application/json` und `S(beta)`. Browser A und B sind getrennte Browser-Kontexte (Chromium, 1280 × 800).

### F0033 Frist über die API setzen und lesen

- [ ] Grenze (Unit, rein): `teilen([2026-10-21, 2026-10-22, 2026-10-28, 2026-10-29], heute 2026-10-28, 7)` → behalten `[22., 28., 29.]`, entfallen `[21.]`; `n = 1` → entfallen `[21., 22.]`; `n = 3650` → entfallen `[]`; jeder Tag liegt in genau einer Menge. Rechnung: 28.10. − (7 − 1) = 22.10. ist der älteste behaltene Tag. Weitere Beispiele aus ADR-0029 Regel 5 (Frist 30 am 2026-11-02 → ältester behaltener 2026-10-04; Frist 7 am 2027-01-03 → 2026-12-28). Die Mutanten „heute zählt nicht“ (behält den 21.) und „`n − 2`“ (wirft den 22.) werden je rot.
- [ ] Einstellungsformat (Unit, rein): `frist_tage = 1` und `= 3650` gelesen; `0`, `3651`, `-3`, `"7"`, `7.5` → je Defekt mit Zeile; Rundweg mit `frist_tage = 30` und `farbe = "blau"` → beide Schlüssel stehen nach dem Schreiben da; derselbe Wert ergibt dieselben Bytes.
- [ ] Setzen (Integration, echtes Programm): `allgemein` mit `einstellungen.toml` (`angelegt`, `angelegt_von`, `farbe = "blau"`, ohne `frist_tage`) → `GET /api/channels` nennt `frist_tage: 7`; `PUT {"tage":30}` → 200 `{"channel":{"name":"allgemein", …, "frist_tage":30}, "entfallen":{"tage":0,"nachrichten":0}}`; die Datei enthält `frist_tage = 30`, `angelegt`, `angelegt_von` und `farbe` unverändert; stderr enthält `channels/allgemein: Frist 7 → 30 Tage, gesetzt von beta@127.0.0.1`.
- [ ] Ohne Datei (Integration): `channels/leer/` ohne `einstellungen.toml` → `frist_tage: 7`; `PUT {"tage":30}` → 200, die Datei entsteht mit `frist_tage = 30`.
- [ ] Neustart (Integration): nach dem Setzen auf 30 beenden und neu starten → `GET /api/channels` nennt `frist_tage: 30`.
- [ ] Derselbe Wert (Integration): Frist 30, `PUT {"tage":30}` → 200 mit `entfallen` 0/0; die Datei ist byte-gleich und dieselbe Datei (Inode unverändert, also nicht neu geschrieben); keine neue Zeile auf stderr.
- [ ] Wertprüfung als Spanne (Integration): `tage` `1` → 200, `3650` → 200; `0`, `3651`, `-1`, `"7"`, `7.0`, `7.5`, `null` und `{}` (fehlt) → je 422 `regel:"frist"`, die Datei byte-gleich. Die Mutanten „untere Grenze 0“, „untere Grenze 2“, „obere Grenze 3649“, „obere Grenze 3651“ werden je rot (der Entwickler nennt die vier Ergebnisse).
- [ ] Grenzschutz und Fehler (Integration): ohne Header → 401; `Origin: http://fremd.example` → 403; `Content-Type: text/plain` → 415; Körper `{` → 400; `PUT` auf `fehlt` und auf `Allgemein` → je 404 `{fehler}`; `fehlt` mit `{"tage":0}` → 404 (Channel vor Wert).
- [ ] Defekte Einstellungen (Integration): `channels/kaputt/einstellungen.toml` mit `frist_tage = "sieben"` → stderr nennt die Datei mit Zeile, `defekte_zeilen` zählt sie, `GET /api/channels` nennt für `kaputt` `frist_tage: null`; `PUT {"tage":30}` auf `kaputt` → 409 `{fehler, regel:"einstellungen-defekt", datei:"channels/kaputt/einstellungen.toml"}`, die Datei ist byte-gleich; `PUT {"tage":0}` auf `kaputt` → 422 (Wert vor Defekt).
- [ ] Laden mit der Frist des Channels (Integration): `allgemein` mit `frist_tage = 30` und `2026-09-28.(3)`, `2026-09-29.(2)`, `2026-10-28.(1)` → `/api/gesundheit` meldet `"nachrichten":3` — geladen sind der 29.09. und der 28.10., nicht der 28.09. Rechnung: 28.10. − 29 Tage = 29.09. (der September hat 30 Tage). Die Mutanten „Frist + 1“ (lädt den 28.09., 6), „Frist − 1“ (lädt den 29.09. nicht, 1) und „immer 7“ (1) werden je rot.

### F0034 Entfallen beim Start und beim Verkürzen

- [ ] Pflegen (Unit, Speicherablage, ohne Uhr): `allgemein` mit Frist 7, Tage 20., 21., 22., 28. auf der Ablage, nur 22. und 28. im Index → `pflegen(2026-10-28, Start)` entfernt 20. und 21. aus der Ablage, der Index bleibt; `pflegen(2026-10-29, Tageswechsel)` entfernt den 22. aus Ablage **und** Index (die Zahl der Nachrichten im Index sinkt um die des 22.).
- [ ] Start (Integration): `allgemein` **ohne** `einstellungen.toml` mit `20.(1)`, `21.(1)`, `22.(1)`, `28.(1)` → sobald die Startzeile gelesen ist, fehlen `2026-10-20.log` und `2026-10-21.log`, `2026-10-22.log` und `2026-10-28.log` liegen da; `/api/gesundheit` meldet `"nachrichten":2`; sobald die Startzeile gelesen ist, steht auf stderr schon `channels/allgemein: 2 Tage entfallen (2026-10-20 bis 2026-10-21; Frist 7 Tage; Start)`. Die Mutanten „heute zählt nicht“ (der 21. bleibt) und „`n − 2`“ (der 22. fehlt) werden je rot.
- [ ] Verkürzen braucht Bestätigung (Integration): `allgemein` mit `frist_tage = 30` und `20.(2)`, `21.(3)`, `22.(1)`, `28.(1)`; nach dem Start ist nichts gelöscht (`"nachrichten":7`). `PUT {"tage":7}` → 409 `{fehler, regel:"bestaetigen", entfaellt:{tage:2, nachrichten:5, bis:"2026-10-21"}}` mit `fehler` „Mit dieser Frist entfallen sofort 2 Tagesdateien mit 5 Nachrichten bis 2026-10-21 — mit "bestaetigt":true erneut senden.“; `PUT {"tage":7,"bestaetigt":false}` → dieselbe 409; danach liegen alle vier Dateien da, `GET /api/channels` nennt `frist_tage: 30`, die Einstellungsdatei ist byte-gleich. Rechnung: ältester behaltener Tag 22.10., es entfallen der 20. (2) und der 21. (3).
- [ ] Grenze der Vorschau (Integration, wie oben): `PUT {"tage":8}` → 409 mit `entfaellt:{tage:1, nachrichten:2, bis:"2026-10-20"}` (ältester behaltener Tag 21.10.) und `fehler` „Mit dieser Frist entfällt sofort 1 Tagesdatei mit 2 Nachrichten bis 2026-10-20 — …“.
- [ ] Einzahl in der Rückfrage (Unit und Integration; Abnahme-Probe aus dem Rechtschreib-Review #12, Muss 3, wörtlich übernommen): über das Programm `allgemein` mit `frist_tage = 30` und `21.(1)`, `22.(1)`, `28.(1)` → `PUT {"tage":7}` → 409 mit `fehler` „Mit dieser Frist entfällt sofort 1 Tagesdatei mit 1 Nachricht bis 2026-10-21 — mit "bestaetigt":true erneut senden.“; Unit für (Tage/Nachrichten) 1/1, 1/2, 2/1 („entfallen sofort 2 Tagesdateien mit 1 Nachricht“) und 2/5. `grep -rE '\b1 (Tagesdateien|Nachrichten)\b' src` findet nichts; die Mutante „immer Mehrzahl“ wird rot.
- [ ] `bestaetigt` als Wahrheitswert (Integration, wie oben): `{"tage":7,"bestaetigt":"ja"}` → 422 `regel:"bestaetigt"`; `{"tage":7,"bestaetigt":1}` → 422 `regel:"bestaetigt"`; nichts geändert.
- [ ] Bestätigt verkürzen (Integration, wie oben): `PUT {"tage":7,"bestaetigt":true}` → 200 `{"channel":{…,"frist_tage":7}, "entfallen":{"tage":2,"nachrichten":5}}`; `2026-10-20.log` und `2026-10-21.log` fehlen, der 22. und der 28. liegen da; `/api/gesundheit` meldet `"nachrichten":2`; stderr enthält `channels/allgemein: Frist 30 → 7 Tage, gesetzt von beta@127.0.0.1` und danach `channels/allgemein: 2 Tage entfallen (2026-10-20 bis 2026-10-21; Frist 7 Tage; Frist gesetzt von beta@127.0.0.1)`.
- [ ] Verlängern und Verkürzen ohne Entfallen (Integration, im Anschluss): `PUT {"tage":30}` → 200 mit `entfallen` 0/0, keine Datei kommt zurück, `"nachrichten":2`; `PUT {"tage":10}` (ältester behaltener Tag 19.10., nichts älter vorhanden) → 200 ohne Bestätigung.
- [ ] Defekt heißt nichts (Integration): `channels/kaputt/` mit `frist_tage = "sieben"` und `2026-01-01.log`, `channels/zu-klein/` mit `frist_tage = 0` und `2026-01-01.log` → nach dem Start liegen beide Tagesdateien da, beide Einstellungsdateien byte-gleich; stderr enthält `channels/kaputt: nichts entfällt — Einstellungen defekt (channels/kaputt/einstellungen.toml:1)` und dieselbe Zeile für `zu-klein`; `GET /api/channels` nennt für beide `frist_tage: null`.

### F0035 Tageswechsel

- [ ] Takt: geprüft durch das Kriterium „Tagestakt“ aus R00014 F0052 (Teile Grenze, Rückwärts — der Fall „Uhr zurückgestellt“ —, Startwert und Panik, mit den Mutanten dort); F0035 baut keinen eigenen Takt-Test.
- [ ] Mitternacht im echten Programm (Integration): `allgemein` ohne Einstellungen mit `20.(1)`, `21.(1)`, `22.(1)`, `28.(1)`; Start mit `LOCALCHATA_JETZT=2026-10-28T23:59:58+01:00` → sobald die Startzeile gelesen ist, fehlen der 20. und 21., der 22. und 28. liegen da; danach fehlt `2026-10-22.log` innerhalb der Grenze **Z3** nach der Startzeile, `2026-10-28.log` liegt weiter da; auf stderr steht eine zweite Pflegezeile für `channels/allgemein`, die die Zahl 1, `2026-10-22` und `Tageswechsel` nennt (seit R00017: die Frist-Zeile mit `anzahl=1 grund=tageswechsel` in `logs/2026-10-29.log`). Rechnung: am 29.10. ist der älteste behaltene Tag 23.10.
- [ ] Mitternacht ohne Wettlauf (Test-Kriterium zu „Mitternacht im echten Programm“, Nacharbeit #12 aus dem Code-Review, Sollte 1): das Programm trägt die Pflegezeile erst ein, wenn alle Channels gepflegt sind — der Test wartet deshalb mit einem Prüfintervall ≤ 50 ms, höchstens bis **Z3** nach der Startzeile, auf die Frist-Zeile in `logs/2026-10-29.log` und prüft erst danach das Fehlen von `2026-10-22.log` (wie „Nicht löschbar“). Abnahme-Probe, wörtlich übernommen: mit einer Pause von 0, 60 und 500 ms zwischen Pflegen und Eintragen (Probe des Entwicklers, nicht committet) bleibt der Test grün; die Zeile `anzahl=1 grund=tageswechsel` steht innerhalb **Z3** in `logs/2026-10-29.log`, auch mit zwei Channels (`allgemein` und `zweiter`, je mit `22.(1)`; dann je Channel eine Zeile).

### F0036 Frist live im Strom

- [ ] Ereignis (Integration): der Agent `alpha` (im Folgenden A) liest `GET /api/ereignisse` mit `S(alpha)` und kennt aus dem Stand `stand = s` und für `allgemein` `frist_tage: 7`; `beta` setzt 30 → A liest `{"art":"channel","stand":s+1,"channel":{…,"name":"allgemein","frist_tage":30}}` innerhalb der Grenze **Z1** nach Erhalt der 200.
- [ ] Kein Ereignis ohne Änderung (Integration, im Anschluss, exakt über `stand`): `beta` setzt 30 erneut (200), dann `{"tage":0}` (422), dann 31 (200) → das nächste Ereignis bei A ist `channel` mit `stand: s+2` und `frist_tage: 31` — dazwischen kam keines.
- [ ] Stand (Integration): ein danach geöffneter Strom nennt im Stand für `allgemein` `frist_tage: 31` und für einen Channel ohne Einstellungen `frist_tage: 7`.
- [ ] **M3** — Prüfpunkt im Entwicklertest, kein Gate: Wechsel-Flut nach der Tabelle; wirksame Wechsel je Sekunde, längste Aufgabe und Anteil Frames über 50 ms im Zuschauer-Tab stehen im Bericht des Entwicklertests und in ADR-0031 unter „Ergänzungen“.

### F0037 Channel-Einstellungen im Browser

- [ ] Eingabe prüfen (Node-Test, rein): `fristPruefen` — `"1"`, `"3650"`, `"7"`, `"007"` (= 7) gültig; `"0"`, `"3651"`, `"-1"`, `"7.5"`, `"7,5"`, `"1e3"`, `""`, `" 7"`, `"+7"` ungültig. Die Mutanten „untere Grenze 0“, „untere Grenze 2“, „obere Grenze 3649“, „obere Grenze 3651“ werden je rot.
- [ ] Texte (Node-Test, rein): `fristText(7)` → „· 7 Tage“, `fristText(1)` → „· 1 Tag“, `fristText(null)` → „· ⚠ Frist gesperrt“; Bestätigung zu `{tage:23, nachrichten:4210, bis:"2026-10-21"}` und Frist 7 → „Bei 7 Tagen entfallen sofort 23 Tage bis einschließlich 21. Oktober 2026 mit 4.210 Nachrichten. Das lässt sich nicht rückgängig machen.“; zu `{tage:1, nachrichten:1, bis:"2026-10-27"}` und Frist 1 → „Bei 1 Tag entfällt sofort 1 Tag bis einschließlich 27. Oktober 2026 mit 1 Nachricht. …“. Das Verb folgt der Zahl der entfallenden Tage (Abnahme-Probe aus dem Rechtschreib-Review #12, Muss 1, wörtlich übernommen): `bestaetigungText(7,{tage:1,…})` → „Bei 7 Tagen entfällt sofort 1 Tag …“, `(1,{tage:1})` → „Bei 1 Tag entfällt …“, `(1,{tage:2,nachrichten:1})` → „… entfallen sofort 2 Tage … mit 1 Nachricht“, `(7,{tage:2,nachrichten:4})` → „… 2 Tage … 4 Nachrichten“. `grep -rE 'entfallen sofort 1 Ta(g) ' web e2e Anforderungen` findet nichts; die Mutanten „Verb nach der Frist“ und „immer Mehrzahl“ werden je rot.
- [ ] Setzen (E2E): `allgemein` ohne Einstellungen; A und B haben `# allgemein` gewählt → die Kopfzeile zeigt `# allgemein ▾` und `· 7 Tage`; A klickt `# allgemein` → Dialog „# allgemein — Einstellungen“, Feld „Aufbewahrungsfrist“ mit 7, Hinweis mit „Heute zählt mit.“; A gibt 30 ein, `Enter` → Dialog zu, Statuszeile „Frist für # allgemein: 30 Tage.“, Kopfzeile `· 30 Tage`; B zeigt `· 30 Tage` ohne Neuladen innerhalb der Grenze **Z2**.
- [ ] Ungültig (E2E): A gibt `0` ein → Fehlerzeile „Frist: ganze Zahl von 1 bis 3650 Tagen.“ mit ⚠, das Feld trägt `aria-invalid="true"`; `Enter` sendet keinen Abruf an `…/frist`.
- [ ] Verkürzen in zwei Schritten (E2E): `allgemein` mit `frist_tage = 30` und `20.(2)`, `21.(3)`, `22.(1)`, `28.(1)`; A öffnet den Dialog, gibt 7 ein, `Enter` → zweiter Schritt: Feld schreibgeschützt, Satz „Bei 7 Tagen entfallen sofort 2 Tage bis einschließlich 21. Oktober 2026 mit 5 Nachrichten. Das lässt sich nicht rückgängig machen.“, der Fokus liegt auf „Zurück“; noch einmal `Enter` → Schritt 1 mit der Eingabe 7; danach liegen alle vier Dateien da und `GET /api/channels` nennt `frist_tage: 30` (zweimal `Enter` löscht nichts). A speichert erneut und klickt „Frist verkürzen“ → Statuszeile „Frist für # allgemein: 7 Tage — 2 Tage mit 5 Nachrichten entfallen.“; der Verlauf ist neu geöffnet und seine älteste Nachricht ist die vom 22. Oktober; `2026-10-20.log` und `2026-10-21.log` fehlen.
- [ ] Bildschirmleser (E2E; Abnahme-Probe aus dem UX-Review #12, Muss 1, wörtlich übernommen; WCAG 4.1.3, 1.3.1): `allgemein` mit `frist_tage = 30` und `20.(2)`, `21.(3)`, `22.(1)`, `28.(1)`; Dialog, 7, `Enter` → `getByRole('button', {name: 'Zurück'})` hat eine zugängliche Beschreibung, die auf `/2 Tage .* 5 Nachrichten .* nicht rückgängig/` passt (`toHaveAccessibleDescription`) — oder die Beschreibung des Dialogs bzw. ein Element mit `role="alert"` enthält den Satz. Varianten: Einzahl (`21.(1)`, `22.(1)`, `28.(1)`, Frist 30 → 7) → die Beschreibung enthält „1 Tag“ und „1 Nachricht“; Zustand `kaputt` (`frist_tage = "sieben"`) → die Beschreibung des Dialogs enthält `channels/kaputt/einstellungen.toml` und „entfällt nichts“. Die Mutante „`aria-describedby` entfernt“ wird rot.
- [ ] Esc (E2E): A öffnet den Dialog und drückt `Esc` → Dialog zu, der Fokus liegt auf `# allgemein`, nichts gesendet.
- [ ] Fremde Änderung bei offenem Dialog (E2E): A hat den Dialog offen und 12 eingegeben; `beta` setzt per API 31 → das Feld zeigt weiter 12, der Hinweis ergänzt „Inzwischen geändert: 31 Tage.“
- [ ] Gesperrt (E2E, Ausnahmeweg, Kriterium nach Nutzen): `kaputt` mit `frist_tage = "sieben"`; A wählt `# kaputt` → Kopfzeile `· ⚠ Frist gesperrt`; der Dialog zeigt das Feld gesperrt und den Satz mit `channels/kaputt/einstellungen.toml`.
- [ ] Netzfehler (E2E, Ausnahmeweg, Kriterium nach Nutzen): das Programm wird beendet, A speichert 30 → Fehlerzeile „Nicht gespeichert — Verbindung prüfen.“ mit ⚠, der Dialog bleibt offen, die Eingabe 30 steht noch.
- [ ] Schmal (E2E): bei 719 px ist der Frist-Text nicht sichtbar, bei 720 px sichtbar; bei 320 px mit und ohne offenen Dialog `scrollWidth <= innerWidth`.
- [ ] Konsole bei A und B: keine unbehandelte Ausnahme und kein CSP-Verstoß. Konsolenfehler: in „Setzen“, „Ungültig“, „Esc“, „Fremde Änderung“, „Gesperrt“ und „Schmal“ keine; in „Verkürzen in zwei Schritten“ höchstens „Failed to load resource“ zu den beiden `PUT …/frist` mit **409** (`bestaetigen`, vorgeschrieben nach ADR-0031). Für „Netzfehler“ gilt das Kriterium nach Nutzen, keine Vorgabe zur Konsole.

### F0038 Löschschutz und Fehlerwege

- [ ] Erlaubnisliste (Integration gegen das echte Programm): in `channels/allgemein/` ohne Einstellungen die reguläre `2026-10-01.log`, ein Symlink `2026-10-02.log` auf `aussen/ziel.log` außerhalb von `channels/`, ein Ordner `2026-10-03.log/`, `2026-10-04.log.bak`, `notizen.txt`; dazu `direkt/a@x+b@y/2026-01-01.log` → nach dem Start fehlt nur `2026-10-01.log`; Symlink, Ordner, `.bak`, `notizen.txt`, `aussen/ziel.log` und die Datei unter `direkt/` liegen byte-gleich da.
- [ ] Wächter vor dem Löschen (Unit, `tempfile`): zwischen Auswahl und Löschen liegt an der Stelle von `2026-10-01.log` ein Symlink → er wird nicht gelöscht, sein Ziel bleibt, der Bericht nennt einen Fehler; liegt dort nichts mehr → der Tag gilt als entfallen.
- [ ] Nicht löschbar (Integration; Testlauf nicht als root, weil `chmod` dort nicht wirkt): `allgemein` mit `2026-10-20.log`, Ordner `channels/allgemein/` ohne Schreibrecht (`chmod a-w`); `zweiter` mit `2026-10-20.log`; Start mit `LOCALCHATA_JETZT=2026-10-28T23:59:58+01:00` → nach der Startzeile liegt `allgemein/2026-10-20.log` noch da, `zweiter/2026-10-20.log` fehlt; stderr enthält genau eine Zeile `channels/allgemein/2026-10-20.log: nicht entfallen (<Grund>)`; nach dem Tageswechsel (innerhalb **Z3**) genau eine zweite solche Zeile; danach in **Z4** keine weitere.
- [ ] Löschfehler im Index (Unit, Speicherablage mit „scheitert“): der 21. liegt in Ablage und Index, sein Entfernen scheitert → nach `pflegen` liegt er in beiden, die Zahl der Nachrichten im Index ist unverändert, der Bericht nennt den Tag; die übrigen entfallenden Tage sind entfernt.
- [ ] Schreibfehler (Integration; nicht als root): `allgemein` ohne Einstellungen, Ordner ohne Schreibrecht; der Agent `alpha` liest den Strom (Stand `s`) → `PUT {"tage":30}` → 500 `{fehler}`; `GET /api/channels` nennt `frist_tage: 7` und `stand: s`; keine Datei `einstellungen.toml` und keine `einstellungen.toml.neu` im Ordner; stderr nennt den Fehler; nach `chmod u+w` und `PUT {"tage":31}` ist das nächste Ereignis bei `alpha` `stand: s+1` mit `frist_tage: 31` (die 500 erzeugte keines).
- [ ] Mutationsprobe der Wächter: je einzeln abgeschaltet — `is_file` in `Logablage::tage`, `symlink_metadata` vor dem Löschen, „defekt heißt nichts“ in `pflegen`, Ausschluss von `direkt/` — wird je mindestens ein Test rot; der Entwickler nennt die vier Ergebnisse.

## Betroffene Verzeichnisstruktur

- Frist und Pflege: `src/gespraeche/frist.rs` (`Frist`, `teilen` ersetzt `tage_in_frist`), `src/gespraeche/einstellungen.rs` (`frist_tage`), `src/gespraeche/mod.rs` (`laden` mit der Frist des Channels, `frist_setzen`, `pflegen`, `Pflegebericht`), `src/gespraeche/verzeichnis.rs` (Channel mit `frist`, Wechsel), `src/gespraeche/verlaufsindex.rs` (`tag_entfernen`, `saetze_vor`), `src/gespraeche/logablage.rs` und `speicherablage.rs` (`tag_entfernen`).
- Tageswechsel: kein eigenes Modul; `src/start/mod.rs` (Pflege beim Start, Anmeldung der Tagesaufgabe `fristpflege` am Tagestakt aus R00014 vor der Startzeile).
- HTTP: neu `src/http/frist.rs`, `src/http/mod.rs` (Route), `src/http/channels.rs` (`frist_tage` in `channel_als_json`).
- Oberfläche: neu `web/frist.js` (rein), `web/channel-einstellungen.js` (Dialog); geändert `web/kopfzeile.js`, `web/api.js`, `web/hauptansicht.js`, `web/lc-seite.css`.
- Tests: Unit im jeweiligen Modul; Integration neu `tests/frist.rs`, `tests/frist_entfallen.rs`, `tests/frist_tageswechsel.rs`, `tests/frist_schutz.rs`; Oberfläche rein `e2e/einheit/frist.test.mjs`; Abläufe `e2e/tests/r00008-*.spec.js`.

## Technische Überlegungen

### Ablauf

1. **Laden** (Start): Einstellungen je Channel → `Frist::Tage(n)` | `Gesperrt { datei }` → `teilen(tage, heute, n)` → behaltene Tage in den Index.
2. **Pflege beim Start**: `pflegen(heute, Start)` → Zeilen auf stderr → Binden → Tagestakt (R00014) mit der Tagesaufgabe `fristpflege` starten, `zuletzt` = das `heute` dieser Pflege → Startzeile.
3. **Pflegen** (`heute`, Anlass):
   - 3.1 je Channel mit lesbaren Einstellungen (Verzeichnis lesend): Tage der Platte → `teilen` → entfallende Tage;
   - 3.2 je Tag unter der Gesprächssperre: Datei entfernen (Wächter `symlink_metadata`) → bei Erfolg Tag aus dem Index;
   - 3.3 Bericht → Zeilen auf stderr.
4. **Tagestakt** (R00014 F0052): jede Sekunde `uhr.jetzt()` → Datum ≠ `zuletzt` der Tagesaufgabe `fristpflege`? → `zuletzt = heute` → `pflegen(heute, Tageswechsel)` über `spawn_blocking`.
5. **Setzen** `PUT …/frist`:
   - 5.1 Grenzschutz, Sitzung, Körper → 404 → 422 `frist`/`bestaetigt` → 409 `einstellungen-defekt`;
   - 5.2 gleicher Wert → 200 ohne Schreiben;
   - 5.3 Vorschau des Entfallens → ohne Bestätigung 409 `bestaetigen`;
   - 5.4 unter der Schreibsperre des Verzeichnisses: Datei schreiben (500 bei Fehler) → Frist setzen → `stand + 1` → Wechsel veröffentlichen;
   - 5.5 ohne Schreibsperre: `pflegen(heute, FristGesetzt { von })` für diesen Channel → 200 mit den Zahlen.
6. **Browser**: `# name` → Dialog → `fristPruefen` → `PUT` → 200: Statuszeile | 409 `bestaetigen`: zweiter Schritt → `bestaetigt: true` → Statuszeile, Channel neu öffnen | Fehler: Fehlerzeile.

### Grobentwurf (Vorplanung)

Die Bubbles aus der WBS als Datenfluss — **Vorplanung**: welche es am Ende werden, entscheidet der Entwickler beim Bauen.

- F0033: B0175 Frist prüfen (rein) · B0176 Frist in den Einstellungen (rein) · B0177 Tage nach der Frist teilen (rein) · B0178 Laden mit der Frist des Channels · B0179 Frist im Verzeichnis setzen · B0180 Frist beantworten
- F0034: B0181 Tagesdatei entfernen · B0182 Tag aus dem Index nehmen (rein) · B0183 Pflegen (unklar) · B0184 Pflegebericht als Zeilen (rein) · B0185 Pflege beim Start · B0186 Vorschau des Entfallens · B0187 Verkürzen bestätigen
- F0035: B0291 Fristpflege als Tagesaufgabe anmelden (B0188 ist in B0268 „Tagestakt“ aus R00014 aufgegangen, B0189 und B0190 in B0291)
- F0036: B0191 Friständerung veröffentlichen · B0192 Wechsel-Flut messen (Testhilfe M3)
- F0037: B0193 Frist-Eingabe prüfen (rein) · B0194 Frist-Texte formen (rein) · B0195 Frist setzen abrufen · B0196 Kopfzeile mit Schalter und Frist · B0197 Dialog Channel-Einstellungen (unklar) · B0198 Verkürzen bestätigen im Dialog (unklar)
- Nacharbeit #12: B0315 Bestätigungssatz nach Zahl gebeugt (F0037, rein) · B0316 409-Text nach Zahl gebeugt (F0034, rein) · B0317 Bestätigung und Sperre für Bildschirmleser (F0037) · B0318 Mitternachtstest wartet auf die Pflegezeile (F0035, nur Test)
- F0038: B0199 Nur reguläre Dateien entfernen · B0200 Löschfehler je Datei · B0201 Schreibfehler beim Setzen

**Elemente** (nur öffentliche Signaturen, nach A00006, A00008, A00009, A00012, A00013, A00005):

- `Frist` (Wert) — `Tage(1..=3650)` | `Gesperrt { datei }`; Konstanten `FRIST_MIN = 1`, `FRIST_MAX = 3650`, `STANDARD_FRIST_TAGE = 7`; `fn frist_bilden(wert: Option<&Value>) -> Result<u32, Regelfehler>`.
- `fn teilen(tage: &[NaiveDate], heute: NaiveDate, n: u32) -> (Vec<NaiveDate>, Vec<NaiveDate>)` (rein, `frist.rs`).
- `Einstellungen` — um `frist_tage: Option<u32>` erweitert (A00013).
- `Gespraeche` — zusätzlich `fn frist_setzen(&self, name: &str, tage: u32, bestaetigt: bool, von: &Kennung) -> Result<FristGesetzt, Fristfehler>` mit `FristGesetzt { channel, entfallen: Entfallen }` und `Fristfehler = Unbekannt | EinstellungenDefekt { datei } | Bestaetigen { tage, nachrichten, bis } | Ablage(String)`; `fn pflegen(&self, heute: NaiveDate, anlass: Anlass) -> Vec<Pflegebericht>` mit `Anlass = Start | Tageswechsel | FristGesetzt { von }`.
- `Logablage` — zusätzlich `fn tag_entfernen(&self, gespraech: &Gespraech, tag: NaiveDate) -> io::Result<()>`.
- `Verlauf` (A00009) — zusätzlich `fn tag_entfernen(&mut self, tag: NaiveDate) -> Option<(usize, u64)>`, `fn saetze_vor(&self, tag: NaiveDate) -> usize`.
- Tageswechsel: keine eigenen Elemente — die Tagesaufgabe `fristpflege` (Folge-Text „Tage jenseits der Frist bleiben bis dahin liegen“, `zuletzt`, `ausfuehren(heute)` → `pflegen(heute, Tageswechsel)`) am `tagestakt::takten` aus R00014; `datum_gewechselt` gehört dorthin.
- Oberfläche (rein, `web/frist.js`): `fristPruefen(text)`, `fristText(tage)`, `bestaetigungText(tage, entfaellt)`, `statusText(name, tage, entfallen)`; am Port (`web/api.js`): `fristSetzen(name, tage, bestaetigt)`.

Wireframe: die WBS führt `wireframes: Dokumentation/Wireframes/`, ein Artboard `D0003.dc.html` fehlt (Hinweis `/wireframe verfeinern D0003`); ADR-0033 Regel 7 verzichtet bewusst darauf. Gestaltungsquelle: Designsystem `components/ChannelEinstellungen/README.md` (Dialog, beide Schritte, Gesperrt, Rückmeldung) und `components/Kopfzeile/README.md` (Schalter, Frist-Text) — Vorgabe für die Gestaltung, nicht Akzeptanzkriterium.

### Änderungen an bestehenden Klassen

- `gespraeche::frist`: `tage_in_frist` wird `teilen` (eine Grenze für Laden und Entfallen); die Tests aus R00002 ziehen mit.
- `gespraeche::Gespraeche::laden`: Frist je Channel statt `STANDARD_FRIST_TAGE`.
- `gespraeche::einstellungen`: `frist_tage` lesen, prüfen, schreiben.
- `gespraeche::verzeichnis::Channel`: Feld `frist`; Wechsel bei Friständerung.
- `http::channels::channel_als_json`: `frist_tage`.
- `start`: Pflege beim Start und Anmeldung der Tagesaufgabe `fristpflege` am Tagestakt, beides vor der Startzeile.
- `web/kopfzeile.js` (Schalter, Frist-Text), `web/hauptansicht.js` (Dialog verdrahten, Neu-Öffnen nach dem Verkürzen), `web/api.js`.

## Tests

Nach Skill `test-pyramide` und ADR-0009; die Szenarien der User Story werden Tests. Tests sind nie schärfer als diese Anforderung: sie übernehmen Z1–Z4 wörtlich; M3 ist ein Messwert, kein Test im Gate.

**Unit (rein, gestellte Uhr):** `teilen` (Beispiele aus ADR-0029, Mutanten), Frist-Spanne (1/3650, Mutanten 0/2 und 3649/3651), Einstellungsformat mit `frist_tage`, `pflegen` mit Speicherablage (Start, Tageswechsel, Löschfehler), Wächter vor dem Löschen (`tempfile`); die Takt-Regel prüft der Tagestakt-Test aus R00014.

**Integration (`tests/`, echtes Programm):** Setzen, Neustart, Spanne, Grenzschutz, Defekt, Laden mit Frist; Start löscht, Verkürzen mit und ohne Bestätigung, Vorschau, Defekt heißt nichts; Mitternacht über `LOCALCHATA_JETZT`; Ereignis und `stand`; Erlaubnisliste, nicht löschbare Datei, Schreibfehler. Mutationsproben: Grenzen der Frist, Mutanten von `teilen` und Laden, die vier Wächter (die Mutanten des Takts im Tagestakt-Test aus R00014).

**Oberfläche rein (`e2e/einheit/`):** `fristPruefen` (Spanne, Mutanten), Texte in der Einzahl und mit deutschen Zahlen.

**E2E (Playwright):** die Szenarien aus F0037 mit zwei Kontexten; Z2 ereignisgenau (`waitForFunction` mit `polling: 'raf'`), nie im Standardraster von `expect`; zweimal `Enter` löscht nichts.

**Entwicklertest, Messwert (kein Gate):** M3 mit B0192.

## Abhängigkeiten

- Abhängig von: I0002 Nachricht landet im Log (R00002, grün) — Logablage, Index, Grenze „heute zählt mit“; I0007 Channel anlegen (R00006, grün) — Channelverzeichnis, `einstellungen.toml`, Channel-Wechsel im Strom, Dialog-Muster; I0008 Channel wechseln und Verlauf lesen (R00007, rot) — F0037 braucht F0028 (Kopfzeile mit `# name`) und F0029 (Channel öffnen); die übrigen Slices hängen nicht an I0008. I0021 Betriebsprotokoll (R00014, rot) — F0035 braucht F0052 (Tagestakt).
- Innerhalb (WBS `Braucht`): F0033 Welle 1; F0034 und F0036 brauchen F0033 — Welle 2 (parallel); F0035 braucht F0034 und F0052 (R00014) — Welle 3; F0037 braucht F0034, F0036, F0028, F0029 — Welle 3; F0038 braucht F0034 und F0035 — Welle 4.
- Blockiert: I0010 Anhänge (löscht `anhaenge/<tag>/` mit, ADR-0030 Regel 8), I0012 Direktnachricht (Frist der Direktgespräche), I0019 Claude Code einbinden (Anleitung nennt die Frist-Regel und „nie `bestaetigt` ohne vorherige 409“).

## Offene Fragen

- **An den Betreiber (Richtungsfrage, blockiert R00008 nicht): Rate je Herkunft für Friständerungen?** Geprüft (Lehre #5, CWE-770): ja, eine Wechsel-Flut ist möglich — nach dem ersten bestätigten Verkürzen liegen keine Tage mehr jenseits von 7, also braucht 3650 → 7 keine Bestätigung mehr; ein Agent kann in einer Schleife abwechselnd 7 und 3650 setzen, jeder `PUT` schreibt die Datei, schreibt eine stderr-Zeile und schickt ein Ereignis an jeden Strom. „Derselbe Wert ergibt kein Ereignis“ bremst das nicht. ADR-0031 Pre-Mortem 2 nimmt es hin. Empfehlung: in R00008 als M3 messen (Messwert im Entwicklertest); liegt die längste Aufgabe im Zuschauer-Tab über 250 ms, plant der Director eine eigene Interaction „Schreibende Änderungen je Herkunft begrenzen“ in D0003 (dieselbe Klasse trifft Senden in I0009), sonst bleibt es hingenommen.

Entschieden vom planer für R00008 (Rückfragen mit Empfehlung beantwortet):

- **Anzeige „Frist gesperrt“ in F0037**, nicht im Ausnahme-Slice: sie ist Teil von Kopfzeile und Dialog; F0038 bleibt serverseitig und hängt nicht an der Oberfläche.
- **F0038 braucht F0035**: „erneut versucht wird nie in jedem Takt“ ist ohne Takt nicht prüfbar.
- **Geklärt 2026-10-28 — Takt der Fristpflege** (ADR-0041, Ergänzung „Vertrag des Tagestakts“): kein eigener Takt; die Fristpflege ist die zweite Tagesaufgabe des Tagestakts aus R00014 F0052, mit dem Datum der Pflege beim Start als Startwert, Vergleich auf Ungleichheit und Weiterlaufen nach einer Panik. F0035 braucht deshalb F0052.
- **F0034 ohne Verlauf-API prüfbar**: über die Dateien im Ordner und `nachrichten` in `/api/gesundheit`; „der Verlauf beginnt am 22.“ prüft der Browser in F0037. So hängen nur F0037 an I0008.
- **Protokollzeile für einen Tag**: die Kriterien prüfen Zahl, Datum und Anlass, nicht die Pluralform — die Zeile ist Diagnose für den Betreiber, keine Oberfläche.
- **Kein Ereignis beim 409, 422, 500 und bei gleichem Wert** wird exakt über `stand` geprüft statt über ein Wartefenster.
- **Nachgezogen 2026-10-28 (Nacharbeit #12, Auto-Reviews)**: das Kriterium „Texte“ schrieb mit „Bei 1 Tag …“ selbst einen Beugungsfehler fest — jetzt folgt das Verb der Zahl der entfallenden Tage; der Wortlaut von `fehler` der 409 `bestaetigen` steht jetzt in der Anforderung (vorher frei); neues Kriterium „Bildschirmleser“ (der Fokus auf „Zurück“ war zugesagt, der Grund nicht hörbar); Test-Kriterium „Mitternacht ohne Wettlauf“. Die Härtung aus denselben Reviews steht in R00019.

## Manuelle Vorbereitungstätigkeiten

- Für M3: ein ruhiger Rechner (`uptime` unter 2), Chromium aus `e2e/node_modules`, Fenster 1280 × 800.
- Für die Kriterien mit `chmod a-w`: Testlauf als normaler Benutzer, nicht als root.

## Manuelle Nachbereitungstätigkeiten

- ADR-0031: M3 unter „Ergänzungen“ eintragen — architekt über den Director.
- Hilfe-Übersicht `?`: „Channel-Einstellungen: Klick auf # name“ (ADR-0033 Pre-Mortem 1) — gehört zur Übersicht, sobald es sie gibt.

## Umfang

Aus der WBS gezählt (`/planung zaehlen I0018`); Standard-Bubbles mit der Untergrenze 0,4h (belegt aus `Schaetzungen/_ist-zeiten.md`, R00001–R00006, Typ Standard), unklare als Richtwert 2–4h:

- F0033 Frist über die API setzen und lesen = 6 Bubbles: 6 Standard (0,4h). Rest: 2,4h klar · 6 von 6 Werten belegt.
- F0034 Entfallen beim Start und beim Verkürzen = 7 Bubbles: 6 Standard (0,4h), 1 unklar (2–4h). Rest: 2,4h klar + 2–4h unklar · 6 von 7 Werten belegt, Rest Richtwert (ungemessen).
- F0035 Tageswechsel = 1 Bubble: 1 Standard (0,4h). Rest: 0,4h klar · 1 von 1 Wert belegt (nachgezogen 2026-10-28: vorher 3 Bubbles, B0188–B0190 verworfen, B0291 neu).
- F0036 Frist live im Strom = 2 Bubbles: 2 Standard (0,4h). Rest: 0,8h klar · 2 von 2 Werten belegt.
- F0037 Channel-Einstellungen im Browser = 6 Bubbles: 4 Standard (0,4h), 2 unklar (2–4h). Rest: 1,6h klar + 4–8h unklar · 4 von 6 Werten belegt, Rest Richtwerte (ungemessen).
- F0038 Löschschutz und Fehlerwege = 3 Bubbles: 3 Standard (0,4h). Rest: 1,2h klar · 3 von 3 Werten belegt.
- Nacharbeit #12 (2026-10-28) = 4 Bubbles: 4 Standard (0,4h) — B0315, B0317 (F0037), B0316 (F0034), B0318 (F0035). Rest: 1,6h klar (Untergrenze, ungemessen für den Typ Nacharbeit).
- I0018 gesamt = 25 Bubbles (ohne F0058 aus R00017, ohne Nacharbeit #12 und ohne die Härtung F0063–F0066 aus R00019). Rest: 8,8h klar + 6–12h unklar · 22 von 25 Werten belegt, Rest Richtwerte (ungemessen).
- Nicht enthalten: Frist der Direktgespräche (I0012), Anhänge löschen (I0010), Feld `beginn` in der Verlauf-Antwort, Rate je Herkunft (offene Frage).

## Warum löst diese Anforderung das Problem? (Pflicht)

Auslöser: Die Vision verspricht „Tage, die älter sind als die Aufbewahrungsfrist des Channels, entfallen“, aber heute lässt der Start alte Tage nur ungeladen liegen — die Platte wächst, und eine Instanz, die drei Wochen läuft, hält drei Wochen Verlauf im Speicher, obwohl ADR-0006 die Frist als Grenze des Index annimmt. Wenn der Start, der Tageswechsel und das Verkürzen dieselbe Grenze anwenden wie das Laden, dann liegt zu jeder Zeit genau das im Ordner und im Index, was die Frist erlaubt — der Speicher bleibt begrenzt, und „öffnet sofort“ aus R00007 hängt nicht an der Laufzeit. Wenn die Frist über eine schmale Route mit Bestätigung beim Verkürzen gesetzt wird, dann kann ein Agent sie mit einem Aufruf ändern, ohne versehentlich Verlauf zu löschen, weil die erste Antwort ihm die Zahlen zeigt — und die Oberfläche bekommt dieselben Zahlen für ihre Rückfrage. Wenn nur die eigenen Tagesdateien gelöscht werden und kaputte Einstellungen das Löschen sperren, dann bleibt der Ordner „lesbar ohne localchatA“ und sicher für Notizen und Sicherungen des Menschen. Der Hebel ist das Löschen selbst: es ist die einzige endgültige Handlung des Programms, deshalb trägt R00008 die Schutzregeln, nicht eine spätere Anforderung.

## Missing-Docs

- Glossar fehlt (`Dokumentation/Domaenenwissen/` existiert nicht); Begriffe wie Frist, Entfallen, Pflege, Tageswechsel, Vorschau sind aus ADR-0029 bis ADR-0033 übernommen.
- Kein Artboard für D0003; ADR-0033 Regel 7 verzichtet bewusst darauf.
- Verhalten von `remove_file` in einem Ordner ohne Schreibrecht ist Linux-Wissen (EACCES), nicht im Projekt geprobt.

## Notizen

### Verworfene Alternativen

- **Je Slice eine eigene Anforderung**: verworfen — alle sechs haben ein Ziel (die Frist der Vision) und durchlaufen als Arbeitspaket I0018 gemeinsam Entwicklertest und Abnahme.
- **„Verlauf beginnt am 22.“ in F0034 über die Verlauf-API**: verworfen — F0034 hinge dann an F0027 aus I0008; Ordner und `/api/gesundheit` zeigen dasselbe, der Browser prüft den Verlauf in F0037.
- **Anzeige „Frist gesperrt“ in F0038** (Zuschnitt des architekt, Punkt 6): verworfen — F0038 hätte F0037 gebraucht und wäre halb Server, halb Oberfläche.
- **Grenze für eine Wechsel-Flut als Kriterium**: verworfen — das wäre eine Rate je Herkunft, die ADR-0031 ausdrücklich hinnimmt; Richtungsfrage an den Betreiber, R00008 misst nur (M3).
- **Kein Ereignis prüfen über ein Wartefenster von 1 s** (ADR-0031, Folgeaufgaben): ersetzt durch die exakte Prüfung über `stand` beim nächsten Ereignis.

### Bewusst außerhalb

Frist der Direktgespräche (I0012); Anhänge löschen (I0010); Feld `beginn` in der Verlauf-Antwort (ADR-0030, Konsequenzen); Frist eines nicht geöffneten Channels ändern (ADR-0033); Tastenkürzel für den Dialog (ADR-0033 Regel 6); Papierkorb statt Löschen (ADR-0030); Rate je Herkunft (offene Frage).
