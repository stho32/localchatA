# ADR-0034: Nachricht senden — `POST /api/channels/{name}/nachrichten` mit `{text}` für Mensch und Agent, höchstens 16 KiB, Steuerzeichen abweisen, Wiederholschlüssel gegen doppeltes Senden

**Status**: Akzeptiert
**Datum**: 2026-10-28
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #8 I0009)
**Kontext-Anforderung**: — (WBS I0009; prägt I0010, I0012, I0013, I0015, I0016, I0017, I0019)
**Erprobt durch**: Wegwerf-Probe im Scratchpad (2026-10-28, Chromium headless aus Playwright 1.57 des Projekts, 1280 × 800, 8 Kerne, Last ≈ 4–7 durch einen parallelen Bau): Block zu 100 Nachrichten gleicher Größe parsen, zeichnen (`textContent`, `pre-wrap`, `max-height: 240px; overflow: hidden` wie ADR-0026 Regel 4) und bis nach zwei Frames layouten, je 9 Läufe — Tabelle unter „Optionen Größe“. Eine einzelne Nachricht jeder Größe live anhängen: Median 33 ms (zwei Frames) in allen Größen. Probe danach gelöscht.

## Kontext

- Vision: „Mensch und KI sind gleichberechtigt. Der Mensch schreibt im Browser, der Agent über die API“; „Live überall … egal ob sie aus dem Browser oder über die API kommt“; Markdown in Nachrichten (I0017), Anhänge (I0010), Direktnachrichten (I0012). Fertig-Kriterium I0009: „Nachricht erscheint im Channel bei allen offenen Sichten sofort“.
- [ADR-0025](0025-verlauf-ueber-die-api-bloecke-mit-zeit-als-cursor.md) Regel 1: dieselbe Ressource `…/nachrichten` nimmt ab I0009 mit `POST` neue Nachrichten an; Regel 3: Form `{zeit, von, text}`, Felder kommen hinzu, keines wird umgedeutet; die Art des Absenders entscheidet I0009 (Empfehlung Metazeile `+ art:`).
- [ADR-0004](0004-logformat-kopfzeile-und-eingerueckter-text.md): Logformat zeilensicher durch Einrückung; „mit `less`/`cat` lesbar wie ein Chat-Protokoll“; Metazeilen `+ schluessel: wert`. [ADR-0011](0011-anhaengen-mit-rueckbau-und-zeilenende-waechter.md): scheitert das Anhängen, ist die Nachricht nirgends.
- Grenzregel für Routen mit Wirkung (A00004, Ergänzung #3): nur unter `grenzschutz` ([ADR-0014](0014-grenzschutz-host-origin-sicherheitsheader.md)), Sitzung über `X-LocalchatA-Sitzung` ([ADR-0012](0012-identitaet-und-sitzung.md)), JSON, Körpergrenze (`src/http/koerper.rs`; bis R00010 eine Grenze von 2 MiB für alle Routen, mit R00010 je Route mit Herleitung und Parse großer Körper außerhalb des Workers — [ADR-0010](0010-eigene-annahmeschleife-hyper-http1.md), Ergänzung). Hausregel „abweisen, nicht umformen“ ([ADR-0013](0013-namensregel-als-erlaubnisliste.md)).
- Lehre #6 (CWE-407): keine Eingabe unbegrenzt verarbeiten. [ADR-0026](0026-verlauf-im-browser-als-fenster-aus-hoechstens-fuenf-bloecken.md) L2: Channel öffnen < 500 ms, ein Block zu 100 Nachrichten; [ADR-0028](0028-nachrichten-live-als-drittes-thema-ordnung-ueber-die-nachrichtenzeit.md): jede Nachricht geht an jeden Browser.
- Designsystem `components/Nachricht`: „wird gesendet …“, gesendet ohne Marke, fehlgeschlagen mit „Erneut senden“; `VerlaufsMarken`: bei verlorener Verbindung wird Gesendetes gepuffert und als „wird gesendet“ gezeigt — der Browser sendet also nach einem Abbruch erneut, ohne zu wissen, ob der erste Versuch angekommen ist.
- Code: `Gespraeche::senden(gespraech, von, text)` hängt an, ergänzt den Index, veröffentlicht noch nicht (`src/gespraeche/mod.rs`); `logformat::satz` wandelt `\r\n` in `\n`, lässt abschließende Leerzeilen fallen, rückt jede Zeile ein; `logformat::text` überspringt `+ `-Zeilen.

## Optionen

### Route und Körper

#### Option A: `POST /api/channels/{name}/nachrichten` mit JSON `{text}` — derselbe Weg für Browser und Agent
- Vorteile: eine Ressource zum Lesen und Schreiben (ADR-0025); Mensch und Agent nutzen nachweislich denselben Weg — „gleichberechtigt“ wird testbar statt behauptet; JSON hat Platz für spätere Felder (`antwort`, `anhaenge`); `Content-Type: application/json` ist eine zweite Hürde gegen Fremdseiten neben der Origin-Prüfung (kein „einfacher“ Cross-Site-Request).
- Nachteile: ein Agent muss Text als JSON kodieren (`jq -Rs '{text: .}'`); das Werkzeug `lc senden` (I0019) verbirgt es.

#### Option B: Roher Körper `text/markdown` (`curl --data-binary @bericht.md`)
- Vorteile: für Agenten ohne JSON-Werkzeug am bequemsten.
- Nachteile: kein Platz für Metadaten (Antwort, Anhänge); `text/plain` ist ein „einfacher“ Request, den jede Fremdseite ohne Preflight absenden kann — der Schutz hinge allein an der Origin-Prüfung; zwei Körperformen, wenn Metadaten kommen.

#### Option C: Eigene RPC-Route `POST /api/senden {ziel, text}` für Channels und Direktnachrichten
- Vorteile: ein Aufruf für „in Channel oder direkt“ (I0015).
- Nachteile: zwei Wege zur selben Wirkung neben der Ressource aus ADR-0025; Direktgespräche bekommen nach ADR-0025 Regel 7 ohnehin eine eigene Ressource gleicher Form.

#### Option D: `multipart/form-data` von Anfang an (für Anhänge vorbereitet)
- Vorteile: I0010 bräuchte keine zweite Körperform.
- Nachteile: Anhänge sind nicht entschieden (Größe, Ablage — Frage 9 des Designsystems); Multipart-Parsing jetzt ohne Nutzen; tote Flexibilität.

### Größe des Texts

| Größe je Nachricht | Block zu 100: JSON | Block zu 100 zeichnen, Median (min–max), Absatz | dto. Codezeilen |
|---|---|---|---|
| 0,5 KB | 56 KB | 33 ms (33–35) | 33 ms (33–50) |
| 4 KiB | 407 KB | 83 ms (67–100) | 83 ms (83–100) |
| **16 KiB** | **1,6 MB** | **283 ms (267–300)** | **317 ms (283–333)** |
| 32 KiB | 3,2 MB | 550 ms (533–588) | 553 ms (533–630) |
| 64 KiB | 6,4 MB | 1.086 ms (1.051–1.188) | 1.050 ms (1.034–1.268) |

Die Kosten wachsen linear mit den Bytes (≈ 170 ms je MB): das CSS-Einklappen (ADR-0026 Regel 4) spart das Zeichnen, nicht das Layout.

- **4 KiB** — Stärke: jeder Block bleibt unter 100 ms. Schwäche: ein Befundbericht mit Code (20 Befunde, 200 Zeilen) passt nicht; bis I0010 gibt es keinen Ausweg über Anhänge.
- **16 KiB** — Stärke: ≈ 300 Zeilen Code oder ein langer Bericht passen; ein Block aus lauter Höchstgrößen öffnet in ≈ 0,3 s, unter L2 (500 ms) mit gut 50 % Reserve. Schwäche: Logs und große Diffs müssen bis I0010 gekürzt werden.
- **64 KiB** — Stärke: fast alles passt. Schwäche: ein Block aus Höchstgrößen kostet über 1 s — ein einzelner Agent könnte „öffnet sofort“ für einen Channel brechen.
- **Blockgrenze in Bytes statt Nachrichtengrenze** (ADR-0025 liefert weniger als `n`, wenn ein Block 1 MB überschreitet) — Stärke: große Nachrichten ohne Block-Risiko. Schwäche: ändert eine akzeptierte Schnittstelle, die gerade mit I0008 gebaut wird, und L3 („genau 500 im DOM“) gälte nicht mehr.

### Zeichen im Text

- **Steuerzeichen abweisen** — Stärke: Hausregel; das Log bleibt mit `cat`/`less` lesbar, ohne dass eine Nachricht das Terminal steuert (Escape-Folgen, CWE-150) oder mit einem einzelnen Wagenrücklauf eine scheinbare Kopfzeile über eine echte schreibt. Schwäche: Agenten müssen Farbcodes aus Werkzeugausgaben entfernen.
- **Steuerzeichen entfernen oder ersetzen** — Stärke: kein Fehler für den Absender. Schwäche: der gespeicherte Text weicht still vom gesendeten ab (gegen die Hausregel).
- **Alles zulassen** — Stärke: einfach. Schwäche: siehe oben; NUL und Escape-Folgen im lesbaren Log.

### Doppeltes Senden

- **Nichts tun** — Stärke: kein Zustand. Schwäche: Antwort verloren (WLAN weg nach dem Schreiben) + „Erneut senden“ oder Puffern nach Wiederverbinden = dieselbe Nachricht zweimal; im Designsystem ist genau das der vorgesehene Ablauf.
- **Wiederholschlüssel `Idempotency-Key` je Absender, im Speicher** — Stärke: der Browser kann gefahrlos wiederholen; Agenten können es (optional) auch; nichts im Log. Schwäche: ein Neustart zwischen erstem Versuch und Wiederholung vergisst den Schlüssel.
- **Inhaltsgleichheit im Zeitfenster** (gleicher Absender, gleicher Text, 10 s) — Stärke: ohne Mitwirkung des Clients. Schwäche: ein Agent, der zweimal „ok“ sagt, verliert die zweite Nachricht still.
- **Schlüssel als Metazeile im Log** — Stärke: übersteht Neustarts. Schwäche: Rauschen im lesbaren Log; Nachschlagen beim Senden bräuchte einen eigenen Index.

## Entscheidung

**Gewählt: Route A; Text höchstens 16 KiB; Steuerzeichen abweisen; Wiederholschlüssel im Speicher; die Art des Absenders als Metazeile.**

Es überwiegt, dass ein einziger Weg für Mensch und Agent die Gleichberechtigung der Vision prüfbar macht und JSON plus Origin-Prüfung zwei unabhängige Hürden gegen Fremdseiten sind; dass 16 KiB den schlimmsten Block (100 Höchstgrößen) in der Probe unter der Öffnen-Grenze L2 halten, ohne ADR-0025 anzufassen; dass der Wiederholschlüssel den im Designsystem vorgesehenen Wiederholweg ungefährlich macht. In Kauf genommen: Agenten kodieren JSON und entfernen Farbcodes; Texte über 16 KiB müssen bis I0010 geteilt werden; ein Neustart des Programms zwischen erstem Versuch und Wiederholung kann ein Doppel ergeben.

Regeln:

1. **Route** `POST /api/channels/{name}/nachrichten` unter `grenzschutz`, Extraktor `Teilnehmer` (Mensch und Agent gleich). Körper JSON `{"text": "<markdown>"}`; Kopf optional `Idempotency-Key: <schlüssel>`.
2. **Körpergrenze dieser Route 131.072 Bytes (128 KiB)** nach der Regel aus [ADR-0010](0010-eigene-annahmeschleife-hyper-http1.md), Ergänzung (R00010). Herleitung: größter gültiger Körper in der längsten JSON-Schreibweise — 16.384 Bytes Text aus ASCII-Zeichen, jedes als `\u0061` (6 Bytes; Zeichen mit 2–4 Bytes UTF-8 werden höchstens dreifach so lang) — `{"text":"` (9) + 98.304 + `"}` (2) = **98.315 Bytes**; mit 30 % Reserve 127.810, gerundet auf 128 KiB (≈ 33 % Reserve). Darüber 413 `{fehler}` mit „höchstens 131072 Bytes“, ohne Parse. Weil die Grenze über 16 KiB liegt, wird der Körper in `spawn_blocking` unter der gemeinsamen Semaphore geparst (ADR-0010, Ergänzung Regel 2).
3. **Prüfreihenfolge und Antworten** (abgewiesen, nie umgeformt):
   1. fremder `Origin`/Host 403 · ohne Sitzung 401 · **Vorab-Buchung** von 1 Einheit am Schreibkonto, sonst 429 ([ADR-0036](0036-schreibkonto-token-eimer-je-kennung-und-je-herkunft.md) Regel 3) · zu groß 413 · ohne JSON-`Content-Type` 415 · kein JSON 400 — sonst wie `POST /api/channels`;
   2. unbekannter oder ungültiger Channelname → 404 `{fehler}` („`# name` gibt es nicht.“);
   3. Körper kein Objekt, `text` fehlt oder ist keine Zeichenkette → 422 `{fehler, regel: "text"}`; ein weiteres Feld → 422 `{fehler, regel: "feld", feld}` (bis I0010/I0013 gibt es keine weiteren; ein Agent erfährt so, dass `antwort` noch nicht wirkt — Muster der Beschreibung in ADR-0021, Ergänzung);
   4. `text` nur aus Leerraum (`char::is_whitespace`, also auch Tab, Zeilenumbruch, geschütztes Leerzeichen) → 422 `{fehler, regel: "leer"}`;
   5. `text` länger als **16.384 Bytes UTF-8** (gemessen, wie er ankommt) → 422 `{fehler, regel: "laenge", hoechstens: 16384, bytes}`;
   6. verbotenes Zeichen → 422 `{fehler, regel: "zeichen", stelle, zeichen}` — `stelle` ist die Position in Zeichen ab 0, `zeichen` die Schreibweise `U+001B`; gemeldet wird das erste. **Verboten**: U+0000–U+0008, U+000B, U+000C, U+000E–U+001F, U+007F, U+0080–U+009F sowie U+000D, wenn nicht unmittelbar U+000A folgt. **Erlaubt**: Tab, Zeilenumbruch, `\r\n` (wird `\n`), alles übrige Unicode einschließlich Emoji, Verbindern und Richtungszeichen;
   7. `Idempotency-Key` vorhanden, aber nicht 1–64 Zeichen aus `[A-Za-z0-9_-]` → 422 `{fehler, regel: "schluessel"}`;
   8. Restbuchung (Kosten − 1) am Schreibkonto, erschöpft → 429 nach [ADR-0036](0036-schreibkonto-token-eimer-je-kennung-und-je-herkunft.md); eine Wiederholung bucht wie ein erster Versuch — bekommt sie 429, ist nichts geschehen, und sie darf nach `Retry-After` mit demselben Schlüssel wiederkommen;
   9. Senden (`spawn_blocking`): neu → **201** `{channel, nachricht}`; Wiederholung mit bekanntem Schlüssel → **200** `{channel, nachricht}` mit der ursprünglichen Nachricht, ohne Schreiben, ohne Ereignis; bekannter Schlüssel mit anderem Text oder anderem Channel → 422 `{fehler, regel: "schluessel"}`; Plattenfehler → 500 `{fehler}`, eine Zeile auf stderr mit Datei und Grund, nichts geschrieben (ADR-0011).
4. **Antwortform** `nachricht` = `{zeit, von, art, text}` — dieselbe Funktion wie Verlauf und Ereignis (A00004, Ergänzung #7). `text` ist der Text **so, wie die Verlauf-API ihn später liefert** (`\r\n` → `\n`, abschließende Leerzeilen entfallen) — der Sender sieht nie etwas anderes als die Leser.
5. **Art des Absenders**: `senden` schreibt nach der Kopfzeile die Metazeile `+ art: mensch` bzw. `+ art: agent` aus der Sitzung (Selbstauskunft, [ADR-0012](0012-identitaet-und-sitzung.md)). Verlauf, Antwort und Ereignis tragen `art`; ohne Metazeile (von Hand, ältere Sätze, Testdaten) `null`. Das ergänzt ADR-0025 Regel 3 um ein Feld; die Darstellung („Agent“ als Wort) gehört dem Designsystem.
6. **Wiederholschlüssel**: gemerkt unter der Schreibsperre ([ADR-0035](0035-nachrichtenzeit-eine-schreibsperre-ueber-alle-gespraeche.md)) als `(kennung, schluessel) → (gespraech, zeit, Prüfsumme des Texts)`; höchstens die **4.096** jüngsten Einträge und höchstens **10 Minuten** alt; nur im Speicher. Geltung je Kennung — ein fremder Absender kann einen Schlüssel weder treffen noch auslesen. Ein Schlüssel kommt nie in Log, Antwort oder Ereignis.
7. **Kein Senden in Direktgespräche** über diese Route; Direktgespräche bekommen mit I0012 ihre eigene Ressource gleicher Form (ADR-0025 Regel 7).
8. **Abgrenzung I0009 ↔ I0015**: der Browser und jeder Agent nutzen ab I0009 **dieselbe** Route; I0009 belegt das mit einem Integrationstest ohne Browser (kein `Origin`, nur Header). I0015 baut keine zweite Channel-Route, sondern bringt, was nur Agenten brauchen: Senden in Direktgespräche (nach I0012), die Beschreibung unter `/api/openapi.json` (A00002) und den Nachweis „Agent sendet, Browser zeigt live“ als Abnahme-Szenario.

## Konsequenzen

**Positiv:**
- Ein Agent sendet mit `curl -H 'X-LocalchatA-Sitzung: …' -H 'Content-Type: application/json' -d '{"text":"…"}' …/api/channels/allgemein/nachrichten` — und sieht in der Antwort Zeit, Kennung und Art, wie jeder Leser sie sieht.
- Der Browser darf nach Netzfehlern wiederholen, ohne Doppel zu erzeugen (Regel 6); die Prüfung ist ein Unit-Test unter der Sperre.
- Das Log bleibt mit `cat` lesbar und kann kein Terminal steuern.

**Negativ / Inkauf zu nehmen:**
- Texte über 16 KiB (lange Logs, große Diffs) müssen bis I0010 geteilt werden; die 422 nennt Grenze und Größe.
- Werkzeugausgaben mit Farbcodes werden abgewiesen (422 `zeichen` mit `U+001B`), bis der Agent sie entfernt — die Anleitung (I0019) nennt es.
- Neue 4xx-Antworten erscheinen im Browser als Konsolenzeile „Failed to load resource“, wenn ein Szenario sie bewusst herbeiführt (422, 429, 500) — sie gehören in die Liste vorgeschriebener Antworten des Konsolenkriteriums (AI-Team/planer.md, Gelerntes (5)).

**Pre-Mortem (ein Monat nach dem Start gescheitert, weil …):**
1. … nach jedem WLAN-Wackler Nachrichten doppelt im Channel stehen. Früh-Indikator: zwei Sätze mit gleichem Text und Absender im Abstand von Sekunden. Gegenmaßnahme: Regel 6; E2E mit verzögerter, dann abgebrochener Antwort und Wiederholung.
2. … ein Agent einen 40-KB-Bericht schickt, 422 bekommt und ihn in 30 Teilen nacheinander sendet. Früh-Indikator: viele Nachrichten derselben Kennung kurz hintereinander, alle nahe 16 KiB. Gegenmaßnahme: Schreibkonto mit Kosten nach Größe ([ADR-0036](0036-schreibkonto-token-eimer-je-kennung-und-je-herkunft.md)); Anleitung „lange Ausgaben als Anhang“ ab I0010.
3. … `text` und Darstellung auseinanderlaufen (Sender sieht `\r\n`, Leser `\n`). Früh-Indikator: Rundweg-Test POST → GET ungleich. Gegenmaßnahme: Regel 4, ein Test über drei Wege (Antwort, Ereignis, Verlauf).

**Folgeentscheidungen / Aufgaben:**
- [ ] Erster Unit-Test (rein, Textregel): `""`, `" \n\t"` → `leer`; 16.384 × `a` → ok, 16.385 → `laenge`; 5.461 × `€` (16.383 Bytes) + `a` → ok, + `€` statt `a` → `laenge`; je eine Vertreterin jeder verbotenen Klasse (`\0`, `\x08`, `\x0b`, `\x1b`, `\x7f`, `\u{85}`, `a\rb`) → `zeichen` mit richtiger `stelle`; `a\r\nb`, `\t`, `👩‍💻`, `\u{202e}` → ok. Mutanten „Grenze 16.383/16.385“ und „`\r\n` verboten“ werden rot.
- [ ] Erster Unit-Test (Gespräche, Speicherablage, aufzeichnender Rückruf): Schlüssel `k1` zweimal mit gleichem Text → einmal angehängt, zweite Antwort `Wiederholt` mit derselben Zeit, ein Rückruf; `k1` mit anderem Text → `SchluesselAnders`; derselbe `k1` von einer anderen Kennung → neue Nachricht; nach 10 min + 1 ms (gestellte Uhr) → neue Nachricht.
- [ ] Erster Integrationstest (echtes Programm, ohne `Origin`): ohne Header 401; fremder `Origin` 403; ohne JSON 415; 131.073 Bytes (ungültiges JSON) → 413, nicht 400, und 131.072 Bytes gültig → nicht 413 (Mutanten ± 1 Byte werden rot); `{"text":"Hallo","antwort":"x"}` → 422 `feld`; `allgemein` unbekannt → 404; `{"text":"Hallo\r\nWelt\n\n"}` → 201, `text` = `Hallo\nWelt`, `art` = `agent` bzw. `mensch` nach Beitritt, Tagesdatei enthält `+ art:` und den eingerückten Text, `GET …/nachrichten` liefert dieselbe Nachricht gleich; 20 gleichzeitige `POST` mit demselben Schlüssel → genau eine 201, 19 × 200 mit derselben `zeit`, ein Satz im Log.

## Quellen / Bezuege

- [A00004 HTTP-Schnittstelle](../Architektur/A00004-http-schnittstelle.md), [A00006 Gespräche](../Architektur/A00006-gespraeche.md), [A00007 Logformat](../Architektur/A00007-logformat.md); Aspekte [X00004 Fehlerbehandlung](../Architektur/Aspekte/X00004-fehlerbehandlung.md), [X00002 Benutzerrechte](../Architektur/Aspekte/X00002-benutzerrechte.md)
- ADR-0004, ADR-0010 (Ergänzung Körpergrenze), ADR-0011, ADR-0012, ADR-0013, ADR-0014, ADR-0021, ADR-0025, ADR-0026, ADR-0028, ADR-0035, ADR-0036; `src/gespraeche/mod.rs` (`senden`), `src/gespraeche/logformat.rs` (`satz`, `text`), `src/http/koerper.rs`; R00010 (Körpergrenze je Route)
- Vision R00000 („gleichberechtigt“, „Live überall“, Markdown, Anhänge); WBS I0009, I0015; Designsystem `components/Nachricht/README.md`, `components/VerlaufsMarken/README.md`, `3-offene-fragen.md` Frage 3 und 9
- IETF-Entwurf „The Idempotency-Key HTTP Header Field“ (httpapi-WG) als Vorbild für Name und 422 bei anderem Inhalt — aus dem Gedächtnis, in diesem Auftrag nicht geöffnet; CWE-150 (Escape-Folgen in Ausgaben), CWE-407, CWE-770
