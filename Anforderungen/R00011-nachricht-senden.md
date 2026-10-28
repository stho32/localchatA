---
id: R00011
status: Neu
datum: 2026-10-28
---

# R00011: Nachricht senden

## Beschreibung

Wer beigetreten ist, schreibt in einen Channel — der Mensch im Browser in ein mehrzeiliges Feld unter dem Verlauf, der Agent mit einem `POST` an dieselbe Ressource, aus der er den Verlauf liest. Die Nachricht steht mit Zeit, Absender und Art des Absenders (Mensch oder Agent) im Log ihres Channels und erscheint sofort in allen offenen Sichten: beim Absender im selben Augenblick als „wird gesendet …“ und dann bestätigt, bei allen anderen Browsern und Tabs im offenen Channel, bei jedem Agenten im Ereignisstrom. Die Zeiten sind über alle Channels eine Folge, damit ein Agent später mit einem einzigen Zeiger nichts verpasst. Ein Schreibkonto je Absender und je Adresse lässt Menschen und Agenten normal arbeiten, bremst aber eine Flut, bevor sie die Sichten der anderen einfriert.

Zahlt ein auf: [Vision](R00000-vision.md) — „Live überall“ („Eine neue Nachricht erscheint sofort in allen offenen Sichten, egal ob sie aus dem Browser oder über die API kommt.“), „Mensch und KI sind gleichberechtigt“ („Der Mensch schreibt im Browser, der Agent über die API“ — über dieselbe Route), „Das Log ist das Gedächtnis“ (jede Nachricht mit Absender und Zeit im Log), „Agenten können warten und reagieren“ (eine Zeitfolge über alle Gespräche als Grundlage für I0016).

Entschieden vom architekt (ADR-0034 bis ADR-0038, ADR-0010 Ergänzung, Berichtigung zu ADR-0028 Regel 8, Baustein A00014; Vision nicht berührt): `POST /api/channels/{name}/nachrichten` mit `{text}` für Mensch und Agent, Text höchstens 16 384 Bytes, Steuerzeichen abgewiesen, Art des Absenders als Metazeile `+ art:`, Körper bis 131 072 Bytes außerhalb des Workers geparst; eine Schreibsperre über alle Gespräche; Schreibkonto als Token-Eimer je Kennung und je Herkunft; im Tab vorläufig, dann bestätigt, ein Sendeauftrag zugleich, fremde Nachrichten je Frame gebündelt; `<textarea>`, `Enter` sendet.

Slices (WBS `Dokumentation/Planung/localchata.md`, alle unter I0009 Nachricht senden): F0041 Senden über die API, F0042 Nachricht live im Strom, F0043 Eingabe und eigene Nachricht im Browser, F0044 Fremde Nachrichten im offenen Verlauf, F0045 Schreibkonto. Die übrigen Slices von I0009 — F0046 Fehler und Wiederholung, F0047 Lücken nach Wiederverbinden, F0048 Entwurf, Längenanzeige und Textprüfung im Browser, F0049 Gebündelte Ansage — sind bis Bubble geplant, gehören aber **nicht** zu dieser Anforderung (Offene Fragen: Teilung von I0009).

## Geschäftlicher Nutzen

Bis R00010 kann man in localchatA lesen, aber nicht schreiben: Nachrichten entstehen nur, wenn jemand Tagesdateien von Hand in den Ordner legt. Der Anlass der Vision — Agenten auf verschiedenen Rechnern helfen einander, zwei Factories sprechen sich ab — braucht genau diesen einen Schritt. Mit R00011 schickt ein Agent einen Befund mit einem `curl`, der Mensch antwortet im Browser, und beide sehen das Gespräch ohne Neuladen wachsen. Weil Menschen und Agenten dieselbe Route nutzen, ist „gleichberechtigt“ geprüft statt behauptet; weil jede Nachricht eine Zeit in **einer** Folge über alle Channels bekommt, kann I0016 („auf Nachrichten warten“) später ohne Verlust darauf aufsetzen; und weil das Schreibkonto eine Flut bremst, bleibt die gemeinsame LAN-Instanz für alle bedienbar, auch wenn ein Agent in eine Schleife gerät.

## Funktionale Anforderungen

### Senden über die API (F0041; ADR-0034 Regeln 1–5, ADR-0010 Ergänzung Regeln 1–2)

- `POST /api/channels/{name}/nachrichten` mit Sitzung (`X-LocalchatA-Sitzung`) und JSON-Körper `{"text": "<markdown>"}` hängt eine Nachricht an den Channel an. Browser und Agent nutzen dieselbe Route; ohne `Origin`-Kopf (Agent mit `curl`) gilt sie genauso.
- **Prüfreihenfolge** (abgewiesen, nie umgeformt): fremder `Origin` oder Host → 403 · ohne Sitzung → 401 · [ab F0045: Vorab-Buchung, sonst 429] · Körper über **131 072 Bytes** → 413 `{fehler}` mit „höchstens 131072 Bytes“, ohne Parse (auch wenn der Körper kein JSON ist) · kein JSON-`Content-Type` → 415 · kein JSON → 400 · unbekannter oder ungültiger Channelname → 404 `{fehler}` „# name gibt es nicht.“ · Körper kein Objekt, `text` fehlt oder ist keine Zeichenkette → 422 `{fehler, regel:"text"}` · ein weiteres Feld → 422 `{fehler, regel:"feld", feld}` · Text nur aus Leerraum (`char::is_whitespace`, auch Tab, Zeilenumbruch, geschütztes Leerzeichen) → 422 `{fehler, regel:"leer"}` · Text über **16 384 Bytes UTF-8** (gemessen am Text, wie er ankommt) → 422 `{fehler, regel:"laenge", hoechstens:16384, bytes}` · verbotenes Zeichen → 422 `{fehler, regel:"zeichen", stelle, zeichen}` · [ab F0045: Restbuchung, sonst 429] · Plattenfehler → 500 `{fehler}`, eine Zeile auf stderr mit Datei und Grund, nichts geschrieben.
- **Verbotene Zeichen**: U+0000–U+0008, U+000B, U+000C, U+000E–U+001F, U+007F, U+0080–U+009F sowie U+000D, wenn nicht unmittelbar U+000A folgt. Erlaubt sind Tab, Zeilenumbruch, `\r\n` und alles übrige Unicode (Emoji, Verbinder, Richtungszeichen). `stelle` ist die Position des **ersten** verbotenen Zeichens in Zeichen ab 0, `zeichen` die Schreibweise `U+001B`.
- **Texte der Abweisung** (der Browser zeigt sie wörtlich, F0043): `leer` „Die Nachricht ist leer.“; `laenge` „Zu lang: 17.020 von höchstens 16.384 Byte — bitte teilen.“ (Zahlen mit Tausenderpunkt); `zeichen` „Steuerzeichen U+001B an Stelle 5 — bitte entfernen.“ (Stelle für Menschen ab 1, also `stelle + 1`); `feld` „Unbekanntes Feld antwort — erlaubt ist nur text.“; `text` „Der Körper braucht text als Zeichenkette.“
- **Antwort** 201 `{channel, nachricht}` mit `nachricht = {zeit, von, art, text}` — dieselbe Form wie im Verlauf und im Ereignis. `text` ist der Text, wie ihn der Verlauf später liefert: `\r\n` wird `\n`, abschließende Leerzeilen entfallen.
- **Art des Absenders**: der Satz bekommt nach der Kopfzeile die Metazeile `+ art: mensch` bzw. `+ art: agent` aus der Sitzung (Beitritt mit `"art":"mensch"` aus der Oberfläche, ohne Angabe `agent`). Verlauf, Antwort und Ereignis tragen `art`; ein Satz ohne Metazeile (von Hand, ältere Sätze, Testdaten) hat `art: null`.
- Große Körper (Grenze über 16 KiB) werden außerhalb des async-Workers in `spawn_blocking` unter einer gemeinsamen Semaphore geparst (Erlaubnisse = Zahl der Kerne, mindestens 2).
- Kein Senden in Direktgespräche über diese Route (I0012); kein Zeitstempel vom Client.

### Nachricht live im Strom (F0042; ADR-0035 Regeln 1–4, ADR-0028 Regeln 1–4 mit Berichtigung)

- Jede 201 erzeugt genau ein Ereignis `{"art":"nachricht","channel":"<name>","nachricht":{zeit, von, art, text}}` an **jeden** offenen Strom (alle sehen alle Channels); Fehlerantworten (4xx, 500) erzeugen keines. Beim Öffnen des Stroms kommt für Nachrichten kein Stand.
- **Eine Schreibsperre über alle Gespräche**: Zeit vergeben, anhängen, in den Index eintragen und veröffentlichen geschehen unter derselben Sperre; die Zeit ist `max(jetzt auf ms, letzte + 1 ms)`. Damit sind die Zeiten über alle Channels eindeutig und streng steigend in genau der Folge, in der Log, Index und Strom sie sehen. Sperrordnung: Gespräch → Schreibsperre; die Pflege der Frist nimmt die Schreibsperre nie.
- **Vorlauf sichtbar**: liegt die vergebene Zeit mehr als 60 s vor der Uhr, schreibt das Senden **einmal je Episode** eine Zeile auf stderr, z. B. „Nachrichtenzeit läuft der Systemuhr 2 h 03 min voraus (Uhr zurückgestellt?) — Zeiten bleiben streng steigend, bis die Uhr aufholt.“ (Stunden und Minuten abgerundet); die Episode endet, sobald die Uhr die letzte Zeit einholt. `/api/gesundheit` nennt `zeit_vorlauf_ms` = max(0, letzte vergebene Zeit − jetzt), gelesen ohne die Schreibsperre.

### Eingabe und eigene Nachricht im Browser (F0043; ADR-0037 Regeln 1–3 und 4 für Fehlerantworten, ADR-0038 Regeln 1–3, 9)

- **Feld**: sobald ein Channel gewählt ist, steht unter dem Verlauf über die volle Breite der Mitte ein `<textarea rows="1">` mit zugänglichem Namen und Platzhalter „Nachricht an # name“; Rahmen `kd-line-strong`, Fokus 2 px `kd-focus`; es wächst mit dem Inhalt bis 200 px (`lc-composer-max`) und scrollt dann in sich. Ohne gewählten Channel keine Eingabe. Tab-Reihenfolge: Kopfzeile → Seitenleiste → Verlauf → Eingabe → Teilnehmerliste.
- **Tasten**: `Enter` ohne Umschalt-, Strg-, Alt- und Meta-Taste, `Strg Enter` und `⌘ Enter` senden; `Shift Enter` macht eine neue Zeile; `Alt Enter` tut nichts; während einer IME-Komposition (`isComposing` oder `keyCode 229`) wird nie gesendet; Text nur aus Leerraum → nichts (keine Anfrage, kein Fehler).
- **Senden**: die Nachricht steht sofort als vorläufige „wird gesendet …“ (Text in `kd-ink-muted`, darunter ◌) **unter** allen bestätigten Nachrichten; das Feld wird leer und wieder eine Zeile hoch, der Fokus bleibt im Feld, das Feld wird nie gesperrt. Steht der Blick nicht am Ende oder ist der neueste Block nicht im Fenster, springt der Verlauf ans Ende (wie „Zum Neuesten“, aber der Fokus bleibt im Feld).
- **Ein Sendeauftrag zugleich je Tab**: vorläufige Nachrichten eines Tabs bilden eine Warteschlange über alle Channels; gesendet wird nur die älteste, die nächste erst nach deren Antwort; jeder Auftrag trägt einen eigenen `Idempotency-Key` (`crypto.randomUUID()`), der bei Wiederholungen gleich bleibt.
- **Bestätigen**: die erste von zwei Quellen gewinnt — die Antwort 201/200 oder das Ereignis `nachricht` mit der eigenen Kennung, dem Channel und dem Text der gerade gesendeten vorläufigen. Die bestätigte Nachricht ersetzt die vorläufige an ihrer Stelle; die zweite Quelle findet nichts mehr zu tun. Jede Zeit steht genau einmal im Verlauf.
- **Fehlerantwort**: 413, 422 und 429 machen die Nachricht fehlgeschlagen mit ⚠ und dem `fehler` der Antwort in `kd-danger` unter dem Text; 500 mit „Nicht gespeichert — der Server konnte nicht schreiben.“; ein Netzfehler bei verbundenem Strom mit „Keine Antwort vom Server.“ (bei getrenntem Strom gilt ab F0046 „ruhen“) Die fehlgeschlagene Nachricht bleibt an ihrer Stelle und hält die Warteschlange nicht an. (Schalter „Erneut senden“ und „Verwerfen“, Zeitgrenze, 401 und Puffern bei getrenntem Strom: F0046, nicht hier.)
- Vorläufige und fehlgeschlagene Nachrichten gehören zu ihrem Channel und stehen nach einem Channel-Wechsel wieder unter dessen Verlauf; beim Neuladen sind sie verloren.
- **Leerer Channel**: unter „Noch keine Nachrichten in # name.“ steht in `kd-ink-muted` „Agenten schreiben hier mit `POST /api/channels/name/nachrichten`.“ (ohne Link bis I0015).
- Bei 320 px Fensterbreite kein Querscrollen.

### Fremde Nachrichten im offenen Verlauf (F0044; ADR-0037 Regel 6, ADR-0028 Regeln 5, 7)

- Der Tab hört Nachrichten schon, bevor ein Channel geöffnet ist, und puffert die des gewählten Channels; nach dem Block übernimmt er nur Nachrichten mit einer Zeit **größer** als die jüngste bekannte Zeit des Channels, kleinere und gleiche verwirft er.
- Ereignisse werden gesammelt und **einmal je `requestAnimationFrame`** eingefügt — eine Anker-Korrektur und ein Abgleich des Fensters je Frame.
- **Blick am Ende** (Abstand zum Ende höchstens eine Zeile): anfügen, der Blick folgt. **Sonst**: anfügen, wenn der neueste Block im Fenster steht, sonst nur zählen; nichts scrollt, die Pille zeigt „n neue Nachrichten“ (Singular „1 neue Nachricht“). Klick auf die Pille oder `Ende`: steht die erste neue im Fenster, steht sie danach am oberen Rand mit einer Zeile Kontext darüber, sonst öffnet der Tab neu wie „Zum Neuesten“; die Zahl gilt, bis der Blick das Ende erreicht. Eigene bestätigte Nachrichten zählen nie zur Pille.
- Nachrichten anderer Channels werden weder gezeichnet noch gezählt.
- Ein zweiter Tab desselben Browsers zeigt Nachrichten des ersten wie fremde (es gibt dort keine passende vorläufige).
- **Art des Absenders** im Verlauf (geöffnet und live): Kachel und Name in `kd-agent` auf `kd-agent-hell` mit der Marke „Agent“ für `art: agent`, in `kd-mensch` ohne Marke für `mensch` und `null`.

### Schreibkonto (F0045; ADR-0036 Regeln 1–7, ADR-0034 Regel 3, ADR-0010 Ergänzung Regel 3)

- Zwei Token-Eimer, nur im Speicher: je Kennung (`name@herkunft`) Vorrat 30 Einheiten, Nachschub 1 je Sekunde; je Herkunft (IP) Vorrat 120, Nachschub 8 je Sekunde. Nachschub = vergangene Zeit × Rate, höchstens bis zum Vorrat; springt die Uhr zurück, gibt es weder Nachschub noch Abzug.
- **Kosten** einer Nachricht: ⌈Bytes des Texts / 4 096⌉, mindestens 1 (also 1 bis 4). Gebucht wird in zwei Schritten, jeweils an **beiden** Eimern, alles oder nichts: **1 Einheit vorab** nach der Sitzungsprüfung und vor dem Lesen des Körpers (auch bei späterem 4xx nicht erstattet), der **Rest** (Kosten − 1) nach allen 4xx-Prüfungen und vor dem Schreiben; ein 500 erstattet nicht.
- Reicht ein Eimer nicht → 429 `{fehler, grenze:"kennung"|"herkunft", wieder_in_ms}` mit Kopf `Retry-After: ⌈wieder_in_ms / 1000⌉` (mindestens 1); kein Satz, kein Ereignis. `wieder_in_ms` ist die Zeit, bis **beide** Eimer die **vollen** Kosten der Anfrage tragen (Vorab-Einheit plus Rest) — eine Wiederholung nach `Retry-After` gelingt dann, wenn nichts anderes bucht. `grenze` nennt den Eimer, der länger braucht. Text: „Zu viele Nachrichten von beta@127.0.0.1 — wieder möglich in 3 s.“ bzw. „Zu viele Nachrichten von 127.0.0.1 (alle Teilnehmer dieser Adresse zusammen) — wieder möglich in 3 s.“ (die Zahl ist `Retry-After`).
- **Kein Gegenzug**: der Eimer hängt an der Kennung, nicht an der Sitzung — wer verlässt und mit demselben Namen neu beitritt, findet seinen Eimer wieder; ein neuer Name bekommt einen vollen Eimer je Kennung, aber nie einen vollen Eimer je Herkunft.
- **Speicher begrenzt**: volle Eimer werden beim Zugriff entfernt, zusätzlich in einem Durchgang, sobald mehr als 1 024 Einträge bestehen.
- **Betrieb**: `/api/gesundheit` nennt `gebremst` (Zahl der 429 seit dem Start); jede gebremste Kennung bekommt **eine** Zeile auf stderr je Episode — `beta@127.0.0.1 gebremst (kennung) — Flut?` —; eine Episode endet, wenn die Kennung 30 s lang kein 429 bekommen hat.
- Gebucht wird nur das Senden in Channels; `beitreten`, `verlassen`, `POST /api/channels` und `PUT …/frist` buchen nicht (Frist: offene Frage).

## Nicht-funktionale Anforderungen

- **Zeiten** — die Grenzen stehen nur hier; Kriterien und User Story nennen sie über ihre Nummer, Tests übernehmen sie wörtlich und sind nie schärfer. Die Regeln selbst (Reihenfolge, kein Doppel, Eimer, Vorlauf) prüfen die Tests exakt — über Zeiten, Zählung und gestellte Uhr; die Zeiten unten sind nur Zustellung. Nummern Z1–Z5 und Z9 wie in ADR-0037; Z6–Z8 gehören zu F0046/F0047 und nicht hierher.

  | # | Was | Art | Startpunkt der Uhr / Messweg | erwartet | Grenze | Zweck |
  |---|---|---|---|---|---|---|
  | Z10 | Nachricht im Strom eines Agenten | Gate (Integration) | Erhalt der 201 im Testprozess → Ereignis `nachricht` beim Strom-Leser gelesen | wenige ms (Bus ohne Takt, wie R00008 Z1) | 2 s | „Live überall“ auch für Agenten |
  | Z1 | Fremde Nachricht erscheint im offenen Channel einer zweiten Sicht | Gate (E2E) — Zustellung | Erhalt der 201 im Testprozess (`Date.now()`) → Nachricht mit Absender in B sichtbar (`page.waitForFunction` mit `polling: 'raf'`, `Date.now()` im Fenster von B; gleiche Maschine) | ≈ 20–60 ms (Bus < 1 ms, Weg über den `SharedWorker`, ein bis zwei Frames; Probe zu ADR-0034: 33 ms) | < 1 s | „erscheint sofort“ (Q3 in A00001: < 1 s im LAN) |
  | Z2 | Reihenfolge und Eindeutigkeit | Gate (E2E) — Regel | alpha und beta senden abwechselnd je 5 über die API | B zeigt dieselben 10 Zeiten wie der Verlauf der API, streng steigend, je einmal | exakt | gleiche Ordnung überall |
  | Z3 | Eigene Nachricht sofort vorläufig | Gate (E2E, Antwort der Route um 2 s verzögert) | `Enter` → Element „wird gesendet …“ mit dem Text sichtbar | im nächsten Frame | sichtbar, bevor die Antwort eintrifft | „sofort“ auch für den Absender |
  | Z4 | Kein Doppel, gleich wer zuerst kommt | Gate (Node, rein) | Ereignis vor Antwort; Antwort vor Ereignis; nicht passender Text | jede Zeit genau einmal, keine vorläufige übrig | exakt | Bestätigung über zwei Wege |
  | Z5 | Nicht am Ende: nichts scrollt | Gate (E2E) | B steht 2 Bildschirmhöhen über dem Ende; beta sendet 3 | `scrollTop` von B unverändert (± 1 px), Pille „3 neue Nachrichten“ | exakt 3 | Designsystem `VerlaufsMarken` |
  | Z9 | Ausbruch zeichnen | **Messwert im Entwicklertest, kein Gate** | B am Ende von `# allgemein`; 4 Agenten einer Herkunft senden je 30 kleine Nachrichten so schnell wie möglich (120 = Vorrat der Herkunft) | wenige ms je Frame (ungemessen) | keine Aufgabe über 250 ms, höchstens 2 % der Frames über 50 ms (wie M1 in R00007) | Lehre #5: 2.456 Ereignisse/s froren einen Tab 28,7 s ein |
  | M2 | Flut großer Körper mit Sitzung | **Messwert im Entwicklertest, kein Gate** (Release-Build) | 24 Schleifen von 4 Herkünften (`127.0.0.1` bis `127.0.0.4`, je 6 Agenten), jede wiederholt `POST …/allgemein/nachrichten` mit einem Körper von genau 131 072 Bytes: `{"text":"` + 16 384 Leerzeichen als Text + `"}`, aufgefüllt mit Leerraum (Antwort 422 `leer` oder 429), 10 s lang; von `127.0.0.5` alle 50 ms `GET /api/gesundheit`, je Anfrage vom Öffnen der Verbindung bis zum letzten Byte, ab 1 s nach Beginn bis zum Ende | ungemessen; R00010 maß auf dem 413-Weg p90 0,018–0,038 s | p90 ≤ 0,15 s (wie R00010) | die Instanz bleibt für alle bedienbar, während ein Rechner mit gültigen Sitzungen flutet |

  Reserve: Z10 und Z1 liegen mehr als das 15-Fache über dem erwarteten Wert; Z3 und Z5 sind keine Dauern. Für M2 gilt die Kontrolle aus R00010: 3 s Leerlauf vorher, Leerlauf-p90 über 0,05 s macht den Lauf ungültig (wiederholen, nicht rot); im Bericht stehen Leerlauf-p90, p50, p90, max und die Antworten je Status (erwartet: überwiegend 429, weil das Konto die Vorab-Einheit vor dem Lesen bucht). Über der Grenze von Z9 oder M2 ist ein Befund an ADR-0037 bzw. ADR-0036, kein roter Test.
- **Sicherheit**: Senden nur mit Sitzung und unter Grenzschutz (Host, Origin, Content-Type, Körpergrenze 131 072 Bytes); Text wird abgewiesen, nie umgeformt; Steuerzeichen kommen nie ins Log (kein Terminal-Steuern, CWE-150) und blähen deshalb keine Verlaufsantwort auf (JSON eines angenommenen Texts ≤ 2 × seiner Bytes, Annahme des Budgets aus R00013; Security-Review zu #23); Text erscheint im Browser nur über `textContent`; das Schreibkonto bremst Fluten je Kennung und je Herkunft (CWE-770), auch Fluten ungültiger großer Körper (Vorab-Buchung vor dem Lesen); jede Bremsung ist im Health-Check zählbar.
- **Benutzerfreundlichkeit**: die eigene Nachricht erscheint vor der Antwort; Fehler stehen an der Nachricht mit ⚠ und Wort, nie nur über Farbe; der Fokus bleibt nach dem Senden im Feld; Lesende werden nie aus ihrer Stelle im Verlauf gerissen; bei 320 px kein Querscrollen.
- **Testbarkeit**: Eimer, Vorlauf und Bündeln über gestellte Uhr bzw. gestellten `requestAnimationFrame`; Zustellung und Tasten als reine Funktionen mit Node-Tests; Herkünfte im Integrationstest über Quelladressen `127.0.0.2` bis `127.0.0.11`; Gegeben mit vielen Nachrichten über den Testdaten-Erzeuger aus R00007.

## Akzeptanzkriterien

Gemeinsam für alle Kriterien, wenn nicht anders genannt: Start mit `TZ=Europe/Berlin localchatA <ordner> --port 0 --host 127.0.0.1 --kein-browser` auf einem Wegwerf-Ordner mit den leeren Ordnern `channels/allgemein/` und `channels/technik/`; `<p>` ist der Port aus der Startzeile. `S(<name>)` ist der Header `X-LocalchatA-Sitzung` mit dem Token aus dem Beitritt von `<name>` per `curl` (ohne `art`: Agent, Herkunft `127.0.0.1`, wenn nicht anders genannt). `POST` meint `POST http://127.0.0.1:<p>/api/channels/<name>/nachrichten` mit `Content-Type: application/json`, `S(beta)` und ohne `Origin`. `alpha` liest den Strom `GET /api/ereignisse` mit `S(alpha)`. Browser A und B sind getrennte Browser-Kontexte (Chromium, 1280 × 800), beigetreten als `shoff` und `anna` (Menschen). Jede Kennung sendet je Szenario höchstens 30 Anfragen in einem Zug (Vorrat ihres Eimers, F0045; abgewiesene zählen mit), außer das Kriterium sagt es anders.

### F0041 Senden über die API

- [ ] Textregel (Unit, rein): `""`, `" \n\t"`, `"\u{a0}"` → `leer`; 16 384 × `a` → ok, 16 385 × `a` → `laenge` mit `bytes: 16385`; 5 461 × `€` + `a` → ok (5 461 × 3 + 1 = 16 384 Bytes), 5 462 × `€` → `laenge` mit `bytes: 16386` (5 462 × 3); 16 385 Leerzeichen → `leer` (Leer vor Länge); je eine Vertreterin jeder verbotenen Klasse — `\0`, `\x08`, `\x0b`, `\x0c`, `\x1b`, `\x7f`, `\u{85}`, `\u{9f}` — hinter `"ab"` → `zeichen` mit `stelle: 2` und der Schreibweise (`U+001B` usw.); `"a\rb"` → `stelle: 1`, `U+000D`; `"ab\r"` → `stelle: 2`; `"a\r\nb"`, `"\t"`, `"👩‍💻"`, `"\u{202e}abc"` → ok. Die Mutanten „Grenze 16 383“, „Grenze 16 385“, „Zeichen statt Bytes gezählt“ und „`\r\n` verboten“ werden je rot (der Entwickler nennt die vier Ergebnisse).
- [ ] Sendekörper (Unit, rein): `{"text":"Hallo"}` → `Hallo`; `{}`, `{"text":5}`, `{"text":null}`, `[1]`, `"Hallo"` → je `regel:"text"`; `{"text":"Hallo","antwort":"x"}` → `regel:"feld"` mit `feld:"antwort"`.
- [ ] Senden und Rundweg (Integration, echtes Programm, ohne `Origin`): `beta` sendet `{"text":"Hallo\r\nWelt\n\n"}` an `allgemein` → 201 `{"channel":"allgemein","nachricht":{"zeit":"<zeit>","von":"beta@127.0.0.1","art":"agent","text":"Hallo\nWelt"}}`; die Tagesdatei des Datums von `<zeit>` endet mit dem Satz `@ <zeit> beta@127.0.0.1`, `+ art: agent`, `  Hallo`, `  Welt` und einer Leerzeile; `GET …/allgemein/nachrichten` mit `S(beta)` liefert als neueste Nachricht genau das Objekt aus der 201; nach einem Neustart liefert es dasselbe.
- [ ] Mensch (Integration): `anna` tritt mit `{"name":"anna","art":"mensch"}` bei und sendet `Hallo` → `"art":"mensch"`, die Tagesdatei enthält `+ art: mensch`.
- [ ] Satz ohne Metazeile (Integration): vor dem Start liegt in `channels/allgemein/<heute>.log` ein Satz `@ <zeit> claude@10.0.0.1` mit `  von Hand` → der Verlauf liefert ihn mit `"art":null`.
- [ ] Körpergrenze als Spanne (Integration): ein gültiger Körper von genau 131 072 Bytes (`{"text":"` + 16 384 × `a` + `"}` + Leerzeichen) → 201 mit 16 384 × `a`; ein Körper von 131 073 Bytes (`{` und 131 072 × `a`, kein JSON) → 413 mit „höchstens 131072 Bytes“, nicht 400; derselbe mit `Content-Type: text/plain` → 413 (Größe vor Typ). Die Mutanten „Grenze − 1“ und „Grenze + 1“ werden je rot.
- [ ] Längste Schreibweise (Integration): Körper `{"text":"` + 16 384 × `a` + `"}` (98 315 Bytes) → 201 mit 16 384 × `a`; Körper mit 5 462 × `€` → 422 `regel:"laenge"`, `bytes:16386`.
- [ ] Abweisungen (Integration): `{"text":"  "}` → 422 `regel:"leer"`, `fehler` „Die Nachricht ist leer.“; 16 385 × `a` → 422 `{regel:"laenge", hoechstens:16384, bytes:16385}`, `fehler` „Zu lang: 16.385 von höchstens 16.384 Byte — bitte teilen.“; `"rot \u001b[31m"` → 422 `{regel:"zeichen", stelle:4, zeichen:"U+001B"}`, `fehler` „Steuerzeichen U+001B an Stelle 5 — bitte entfernen.“; `{"text":"Hallo","antwort":"x"}` → 422 `{regel:"feld", feld:"antwort"}`; `{"text":5}` → 422 `regel:"text"`. Danach ist die Tagesdatei byte-gleich wie vorher und der Verlauf unverändert.
- [ ] Steuerzeichen und JSON-Ausdehnung (Zweck: ein Text, den die Instanz annimmt, wird im Verlauf als JSON höchstens doppelt so lang — die Annahme „JSON ≤ 2 × Satzbytes + 60 Bytes je Nachricht“ hinter dem Budget aus R00013; ein Steuerzeichen ohne Kurzform würde als `\u0001` 6 Bytes, ein Text aus `a` und 16 383 × U+0001 also 1 + 16 383 × 6 = 98 299 Bytes JSON statt höchstens 32 768):
  - Klassengrenzen als Spanne (Unit, rein, je hinter `"ab"`): U+0008 → `zeichen`, U+0009 → ok; U+000A → ok, U+000B → `zeichen`; U+001F → `zeichen`, U+0020 → ok; U+007E → ok, U+007F → `zeichen`; U+009F → `zeichen`, U+00A0 → ok. Die Mutanten „Klasse endet bei U+001E“, „Tab verboten“, „Zeilenumbruch verboten“ und „Zeichenregel abgeschaltet“ werden je rot (der Entwickler nennt die vier Ergebnisse).
  - Ausdehnung (Unit, rein): für `a` + 16 383 × `\t`, `a` + 16 383 × `"`, `a` + 16 383 × `\\` und `a` + 16 383 × `\n` sagt die Textregel ok, und die JSON-Zeichenkette des Texts (`serde_json::to_vec`) ist höchstens 2 × 16 384 + 2 = 32 770 Bytes lang (gerechnet: 1 + 16 383 × 2 + 2 Anführungszeichen = 32 769).
  - Über HTTP (Integration): `{"text":"a\u0001b"}` → 422 `{regel:"zeichen", stelle:1, zeichen:"U+0001"}`, `fehler` „Steuerzeichen U+0001 an Stelle 2 — bitte entfernen.“, Tagesdatei byte-gleich; Kontrolle `{"text":"a\tb\nc"}` → 201, der Verlauf liefert `"text":"a\tb\nc"` (Tab und Zeilenumbruch bleiben erlaubt).
- [ ] Grenzschutz und Reihenfolge (Integration): ohne Header → 401; `Origin: http://fremd.example` ohne Header → 403; `Content-Type: text/plain` → 415; Körper `{` → 400; an `fehlt` und an `Allgemein` → je 404 mit `fehler` „# fehlt gibt es nicht.“ bzw. „# Allgemein gibt es nicht.“; an `fehlt` mit `{"text":""}` → 404 (Channel vor Text).
- [ ] Plattenfehler (Integration; Testlauf nicht als root): `channels/allgemein/` ohne Schreibrecht (`chmod a-w`) → `POST` `Hallo` → 500 `{fehler}`; stderr nennt Datei und Grund; keine Tagesdatei entstanden, der Verlauf ist leer; nach `chmod u+w` gelingt `Hallo` mit 201.

### F0042 Nachricht live im Strom

- [ ] Schranke über zwei Channels (Unit, Gespräche mit Speicherablage, aufzeichnender Rückruf): Thread 1 sendet in `allgemein`, die Ablage hält dessen Anhängen an einer Schranke fest; Thread 2 sendet in `technik`; Schranke frei → der Rückruf sieht `allgemein` vor `technik`, und `zeit(allgemein) < zeit(technik)`. Der Test ist vor dem Umbau auf die Schreibsperre rot (der Entwickler nennt das Ergebnis vor dem Umbau).
- [ ] Sperrordnung (Unit): Pflege der Frist (I0018) und Senden laufen 200-mal gleichzeitig über denselben Channel → beide enden, der Test endet innerhalb von 10 s (kein Hängen).
- [ ] Vorlauf (Unit, gestellte Uhr): letzte Zeit 2026-10-28T12:00:00.000+01:00, Uhr 11:58:59.000 → vergebene Zeit 12:00:00.001, eine Zeile, Vorlauf 61 001 ms; zweite Nachricht → keine zweite Zeile; Uhr 12:00:01.000 → Vorlauf 0, Episode zu; Uhr wieder 11:58:00 und Senden → wieder eine Zeile. Bei 60 000 ms Vorlauf keine Zeile, bei 60 001 ms eine (Mutanten „Schwelle 59 s“ und „61 s“ werden rot).
- [ ] Ereignis (Integration): `alpha` liest den Strom; `beta` sendet `Hallo` an `allgemein` → `alpha` liest `{"art":"nachricht","channel":"allgemein","nachricht":{"zeit":<zeit aus der 201>,"von":"beta@127.0.0.1","art":"agent","text":"Hallo"}}` innerhalb **Z10** nach Erhalt der 201.
- [ ] Kein Ereignis bei Fehlern (Integration, exakt über das nächste Ereignis): `beta` sendet `{"text":" "}` (422), an `fehlt` (404), einen Körper von 131 073 Bytes (413), dann `zwei` (201) → das nächste Ereignis bei `alpha` ist `zwei`. Plattenfehler (nicht als root): Ordner ohne Schreibrecht, `drei` → 500; Schreibrecht zurück, `vier` → das nächste Ereignis bei `alpha` ist `vier`.
- [ ] Ordnung über Channels (Integration): 40 Agenten — je 4 von den Herkünften `127.0.0.2` bis `127.0.0.11` — senden gleichzeitig je 25 Nachrichten, abwechselnd an `allgemein` und `technik`; `alpha` liest mit → alle 1 000 Anfragen enden mit 201; `alpha` liest 1 000 Ereignisse, ihre Zeiten sind in Lesefolge streng steigend und paarweise verschieden; je Channel ist die Folge der Zeiten im Strom gleich der Folge der Sätze in den Tagesdateien. Rechnung: je Kennung 25 ≤ 30, je Herkunft 4 × 25 = 100 ≤ 120 — kein 429; Kapazität des Busses 1 024 ≥ 1 000 — kein `NeuerStand`.
- [ ] Vorlauf im echten Programm (Integration): Start mit `LOCALCHATA_JETZT=2026-10-28T12:00:00.000+01:00`; vor dem Start liegt in `channels/allgemein/2026-10-28.log` ein Satz mit der Zeit `2026-10-28T14:03:30.000+01:00` → `beta` sendet zweimal → die Zeiten sind `14:03:30.001` und `14:03:30.002`; stderr enthält genau eine Zeile mit „Nachrichtenzeit läuft der Systemuhr 2 h 03 min voraus“; `/api/gesundheit` meldet `zeit_vorlauf_ms` zwischen 7 410 002 − (ms seit der Startzeile + 1 000) und 7 410 002. Rechnung: 14:03:30.002 − 12:00:00.000 = 2 h 3 min 30,002 s = 7 410 002 ms; nach weniger als 30 s Laufzeit bleiben es abgerundet 2 h 03 min. Ohne Satz aus der Zukunft meldet die Gesundheit `zeit_vorlauf_ms: 0`.

### F0043 Eingabe und eigene Nachricht im Browser

- [ ] Tasten (Node-Test, rein): `Enter` → senden; `Shift Enter` → zeile; `Strg Enter`, `Meta Enter` → senden; `Alt Enter` → nichts; `Enter` mit `isComposing: true` oder `keyCode: 229` → nichts; `Enter` bei Text `" \n "` → nichts; `a` → nichts.
- [ ] Zustellung (Node-Test, rein; **Z4**): (a) senden `x` → eine vorläufige, Sendeauftrag `x`; Antwort 201 mit Zeit 5 → keine vorläufige, Zeit 5 bestätigt. (b) senden `x`; Ereignis eigene Kennung `x` Zeit 5 vor der Antwort → ersetzt; danach Antwort Zeit 5 → unverändert, Zeit 5 genau einmal. (c) senden `x` und `y` → nur `x` im Auftrag; Antwort für `x` → Auftrag `y`. (d) senden `x`; Ereignis eigene Kennung mit Text `z` → keine Bestätigung, die Nachricht geht als fremde weiter, `x` bleibt vorläufig. (e) Antwort 422 mit `fehler` „Die Nachricht ist leer.“ → fehlgeschlagen mit diesem Grund, der nächste Auftrag läuft; Antwort 500 → „Nicht gespeichert — der Server konnte nicht schreiben.“; Netzfehler → „Keine Antwort vom Server.“ Der Schlüssel jedes Auftrags ist bei jedem Abruf derselbe.
- [ ] Senden (E2E): A wählt `# allgemein` → das Feld hat den zugänglichen Namen und Platzhalter „Nachricht an # allgemein“; A tippt `Hallo`, `Shift Enter`, `Welt`, `Enter` → das Feld ist leer und eine Zeile hoch, `document.activeElement` ist das Feld; der Verlauf von A zeigt eine Nachricht von `shoff@127.0.0.1` mit den Zeilen „Hallo“ und „Welt“; `GET …/allgemein/nachrichten` liefert sie mit `"text":"Hallo\nWelt"` und `"art":"mensch"`.
- [ ] Vorläufig (E2E, **Z3**): die Antwort der Route `POST …/nachrichten` wird um 2 s verzögert (`page.route`, Anfrage weitergeleitet); A sendet `eins` → „wird gesendet …“ mit dem Text `eins` ist sichtbar, solange die Antwort noch aussteht; nach der Antwort steht `eins` genau einmal im Verlauf und kein Element „wird gesendet …“ mehr.
- [ ] Reihenfolge (E2E): A sendet `1`, `2`, `3` direkt nacheinander (je `Enter`) → der Verlauf von A zeigt `1`, `2`, `3` in dieser Folge, je einmal; der Verlauf der API zeigt dieselbe Folge mit streng steigenden Zeiten; laut Netzprotokoll der Seite beginnt kein `POST`, bevor der vorige beantwortet ist.
- [ ] Leerraum (E2E): A tippt drei Leerzeichen und `Enter` → kein `POST`, kein Fehler, das Feld behält den Text.
- [ ] Senden springt ans Ende (E2E): `allgemein` mit 1 000 Nachrichten aus dem Testdaten-Erzeuger (R00007); A scrollt nach oben, bis der neueste Block abgeworfen ist (`neuer`), setzt den Fokus ins Feld und sendet `hier` → der Verlauf steht am Ende, `hier` ist sichtbar, der Fokus liegt im Feld.
- [ ] Fehler an der Nachricht (E2E, Antworten von der Route gestellt): die Route antwortet nacheinander 422 `{"fehler":"Die Nachricht ist leer.","regel":"leer"}`, 429 `{"fehler":"Zu viele Nachrichten von shoff@127.0.0.1 — wieder möglich in 3 s.","grenze":"kennung","wieder_in_ms":2500}` mit `Retry-After: 3`, 500 `{"fehler":"…"}` und bricht danach eine Anfrage ab (`route.abort`); A sendet vier Nachrichten → sie stehen in Sendereihenfolge unter dem Verlauf, je mit ⚠ und „Die Nachricht ist leer.“, „Zu viele Nachrichten von shoff@127.0.0.1 — wieder möglich in 3 s.“, „Nicht gespeichert — der Server konnte nicht schreiben.“ bzw. „Keine Antwort vom Server.“ in der Farbe von `kd-danger`; danach gibt die Route frei, A sendet `fünf` → 201, `fünf` steht bestätigt im Verlauf (die Fehlschläge halten die Warteschlange nicht an).
- [ ] Channel-Wechsel (E2E): eine fehlgeschlagene Nachricht in `# allgemein`; A wechselt nach `# technik` und zurück → sie steht wieder unter dem Verlauf von `# allgemein`, in `# technik` nicht.
- [ ] Leerer Channel (E2E): A wählt `# technik` (leer) → „Noch keine Nachrichten in # technik.“ und darunter „Agenten schreiben hier mit `POST /api/channels/technik/nachrichten`.“
- [ ] Höhe und Schmal (E2E): 20 Zeilen per `Shift Enter` → das Feld ist höchstens 200 px hoch und scrollt in sich; bei 320 px Fensterbreite mit Feld und einer fehlgeschlagenen Nachricht gilt `scrollWidth <= innerWidth`.
- [ ] Konsole bei A und B: keine unbehandelte Ausnahme und kein CSP-Verstoß. Konsolenfehler: in allen Szenarien außer „Fehler an der Nachricht“ keine; dort höchstens „Failed to load resource“ zu den gestellten Antworten 422, 429 und 500 und zur abgebrochenen Anfrage.

### F0044 Fremde Nachrichten im offenen Verlauf

- [ ] Einordnen (Node-Test, rein): Block mit den Zeiten 8, 9, 10; vor dem Block gepuffert 9 und 11 → nach dem Block 8, 9, 10, 11; danach Ereignisse 10 und 12 → 8, 9, 10, 11, 12 (10 verworfen); Ereignis anderer Channel → unverändert.
- [ ] Bündeln (Node-Test, gestellter `requestAnimationFrame`): 120 Ereignisse in einem Task → genau ein Einfügen mit 120 Nachrichten beim nächsten Frame; ein Frame ohne Ereignisse → kein Einfügen.
- [ ] Folgen oder zählen (Node-Test, rein; Zeilenhöhe 20 px): Abstand zum Ende 0 → folgen; 20 px → folgen; 21 px → anfügen und zählen; neuester Block nicht im Fenster → nur zählen; Zählung 1 → „1 neue Nachricht“, 3 → „3 neue Nachrichten“.
- [ ] Zweite Sicht (E2E, **Z1**): A, B und ein zweiter Tab A2 von Browser A haben `# allgemein` gewählt und stehen am Ende; `beta` sendet `Hallo B` über die API → in B erscheint `Hallo B` mit `beta@127.0.0.1` innerhalb **Z1** nach Erhalt der 201, ebenso in A und A2; der Blick von B steht am Ende; keine Pille.
- [ ] Reihenfolge (E2E, **Z2**): `alpha` und `beta` senden über die API abwechselnd je 5 Nachrichten → B zeigt 10 Nachrichten in der Folge der Zeiten aus `GET …/nachrichten`, jede genau einmal.
- [ ] Anderer Channel (E2E): B hat `# allgemein` gewählt; `beta` sendet an `technik` → in B ändert sich der Verlauf nicht, keine Pille; B wählt `# technik` → die Nachricht steht da.
- [ ] Nicht am Ende (E2E, **Z5**): `allgemein` mit 300 Nachrichten aus dem Testdaten-Erzeuger; B scrollt 2 Bildschirmhöhen über das Ende; `beta` sendet 3 → `scrollTop` von B ändert sich um höchstens 1 px, die Pille zeigt „3 neue Nachrichten“; Klick auf die Pille → die erste der drei ist sichtbar, ihr oberer Rand liegt höchstens eine Zeilenhöhe + 1 px unter dem oberen Rand des Verlaufs; scrollt B ans Ende, verschwindet die Pille.
- [ ] Neuester Block abgeworfen (E2E): `allgemein` mit 1 000 Nachrichten; B scrollt nach oben, bis der neueste Block abgeworfen ist; `beta` sendet 2 → keine neue Nachricht im DOM, Pille „2 neue Nachrichten“; `Ende` im Verlauf → B öffnet neu, beide stehen am Ende.
- [ ] Art des Absenders (E2E): `beta` (Agent) und `carla` (per `curl` mit `"art":"mensch"` von `127.0.0.1`) senden je eine Nachricht; vor dem Start liegt ein Satz von `claude@10.0.0.1` ohne Metazeile → beim Öffnen und live trägt die Nachricht von `beta` die Marke „Agent“ und Kachel und Name in der Farbe von `kd-agent`; `carla` und `claude@10.0.0.1` ohne Marke in der Farbe von `kd-mensch`.
- [ ] **Z9** — Prüfpunkt im Entwicklertest, kein Gate: Ausbruch nach der Tabelle; längste Aufgabe und Anteil Frames über 50 ms in B stehen im Bericht des Entwicklertests und in ADR-0037 unter „Ergänzungen“.
- [ ] Konsole bei A, A2 und B: keine unbehandelte Ausnahme, kein CSP-Verstoß, keine Konsolenfehler.

### F0045 Schreibkonto

- [ ] Eimer je Kennung (Unit, rein, gestellte Uhr): `a@10.0.0.1` bucht 30 × 1 → ok; die 31. → `Gebremst{kennung, 1000 ms}`; vom Zustand nach der 30. aus: nach 999 ms → gebremst, nach 1 000 ms → ok. Die Mutanten „Vorrat 29“, „Vorrat 31“, „Nachschub 0,9/s“ und „Nachschub 1,1/s“ werden je rot.
- [ ] Eimer je Herkunft (Unit): vier Kennungen von `10.0.0.1` buchen je 30 → ok; die 121. Einheit (fünfte Kennung) → `Gebremst{herkunft, 125 ms}` (1 Einheit bei 8/s); eine Kennung von `10.0.0.2` → ok. Die Mutanten „Vorrat 119“, „Vorrat 121“, „Nachschub 7/s“ und „Nachschub 9/s“ werden je rot.
- [ ] Alles oder nichts, Uhr zurück, Kosten (Unit): Kennung voll, Herkunft leer → gebremst, der Eimer der Kennung bleibt voll; Uhr 1 h zurück → kein Nachschub, kein Fehler; Kosten: 1 Byte → 1, 4 096 → 1, 4 097 → 2, 16 384 → 4.
- [ ] Zwei Schritte und volle Kosten (Unit): Kennung mit 2 Einheiten; vorab 1 → ok (1 übrig); Rest 3 → `Gebremst{kennung, 3000 ms}`, die Vorab-Einheit bleibt verbraucht. Vom Zustand danach: nach 2 999 ms reicht der Eimer nicht für 1 + 3, nach 3 000 ms reicht er. Rechnung: 1 Einheit + 3 s × 1/s = 4 = volle Kosten (Abweichung vom Rechenbeispiel in ADR-0036, siehe Offene Fragen).
- [ ] Speicher (Unit): 1 025 Kennungen buchen je 1; 30 s später (alle wieder voll) bucht eine weitere → die Tabelle hat danach genau 1 Eintrag.
- [ ] Episode (Unit, gestellte Uhr): Bremsung bei t = 0 → eine Zeile; weitere bei 1 s und 29,999 s → keine; nächste Bremsung 30 s nach der letzten (bei 59,999 s) → eine neue Zeile. Die Mutanten „eine Zeile je 429“ und „nie eine zweite Zeile“ werden je rot.
- [ ] Kennung im echten Programm als Spanne (Integration): `beta` sendet 60 kleine Nachrichten so schnell wie möglich nacheinander → die Zahl der 201 liegt in [30, 30 + ⌈Dauer in s⌉ + 1] (Dauer von der ersten bis zur letzten Antwort); jede 429 trägt `grenze:"kennung"`, `wieder_in_ms` > 0 und `Retry-After` ≥ 1; die Tagesdatei enthält genau so viele Sätze wie 201, und `alpha` hat genau so viele Ereignisse gelesen.
- [ ] Herkunft im echten Programm (Integration): fünf Agenten von `127.0.0.2` senden gleichzeitig je 30 kleine Nachrichten → die Zahl der 201 liegt in [120, 120 + 8 × ⌈Dauer in s⌉ + 1], mindestens eine 429 trägt `grenze:"herkunft"`; danach sendet ein Agent von `127.0.0.3` → 201.
- [ ] Kosten nach Größe (Integration): `beta` sendet 10 Texte zu je 16 384 Bytes nacheinander → die ersten 7 enden mit 201 (7 × 4 = 28 ≤ 30 Einheiten), die Zahl der 201 insgesamt ist höchstens 7 + ⌈Dauer in s / 4⌉ + 1; die erste 429 trägt `grenze:"kennung"` und `Retry-After: 3`. Rechnung: vor der 8. liegen 2 Einheiten, vorab −1 → 1, der Rest 3 fehlt; volle Kosten 4 − 1 = 3 Einheiten zu je 1 s.
- [ ] Vorab vor dem Körper (Integration): `beta` sendet 30-mal `{}` (je 422, die Vorab-Einheit wird nicht erstattet) und danach gültige kleine Texte bis zur ersten 429 → vor ihr enden höchstens ⌈Dauer in s⌉ + 1 mit 201 (Dauer ab dem ersten `{}`; gültig nur unter 5 s, sonst wiederholen) — ohne Vorab-Buchung wären es 30; unmittelbar danach ein Körper von 131 073 Bytes → 429, nicht 413 (gebucht wird vor dem Lesen). Mit leerem Eimer von `beta`: fremder `Origin` mit `S(beta)` → 403 und ohne Header → 401 (Grenzschutz und Sitzung vor dem Konto), `gebremst` steigt dabei nicht.
- [ ] Kein Gegenzug (Integration): `beta` sendet 30 kleine Nachrichten, die 31. → 429; `beta` verlässt (`POST /api/verlassen`), tritt mit dem Namen `beta` neu bei und sendet sofort 5 kleine Nachrichten → die Zahl der 201 darunter ist höchstens ⌈Sekunden seit der ersten 429⌉ + 1. Gültig nur, wenn alles in unter 3 s läuft (sonst wiederholen) — ein frischer Eimer gäbe 5.
- [ ] Betrieb (Integration, im Anschluss an „Kennung“): `/api/gesundheit` meldet `gebremst` gleich der Zahl der 429; stderr enthält für die Schleife genau eine Zeile `beta@127.0.0.1 gebremst (kennung) — Flut?`; ohne Bremsung meldet ein frischer Start `gebremst: 0`.
- [ ] Text (Integration): eine 429 mit `Retry-After: 3` und `grenze:"kennung"` trägt `fehler` „Zu viele Nachrichten von beta@127.0.0.1 — wieder möglich in 3 s.“; mit `grenze:"herkunft"` „Zu viele Nachrichten von 127.0.0.2 (alle Teilnehmer dieser Adresse zusammen) — wieder möglich in <n> s.“
- [ ] Mutationsprobe der Wächter: je einzeln abgeschaltet — Vorab-Buchung („Vorab vor dem Körper“), Restbuchung („Kosten nach Größe“), Eimer je Kennung, Eimer je Herkunft — wird je mindestens ein Integrationstest rot; der Entwickler nennt die vier Ergebnisse.
- [ ] **M2** — Prüfpunkt im Entwicklertest, kein Gate: Flut nach der Tabelle im Release-Build (`cargo test --release --test verfuegbarkeit -- --ignored`); Werte im Bericht des Entwicklertests.

## Betroffene Verzeichnisstruktur

- Senden: `src/http/nachrichten.rs` (`POST` neben `GET`; `NachrichtAntwort` öffentlich und mit `art`), `src/http/koerper.rs` (Parse außerhalb des Workers mit Semaphore), `src/http/mod.rs` (Route), neu `src/gespraeche/textregel.rs` (rein); `src/gespraeche/logformat.rs` (Metazeile `art`), `src/gespraeche/mod.rs` (`senden` mit Art unter der Schreibsperre, `Nachricht.art`), `src/gespraeche/verlaufsindex.rs` (Art beim Lesen).
- Strom: `src/live/bus.rs` (`Ereignis::Nachricht`), `src/http/ereignisse.rs` (Form im Strom), `src/http/gesundheit.rs` (`zeit_vorlauf_ms`, `gebremst`), `src/uhr.rs` (Vorlauf).
- Schreibkonto: neu `src/schreibkonto/` (A00014: Eimer, Abbuchen, Kosten, Episode); `src/start/mod.rs` (Baustein erzeugen).
- Oberfläche: neu `web/eingabe.js` (Feld, Tasten), `web/zustellung.js` (rein), `web/live-verlauf.js` oder Erweiterung von `web/verlauf.js` (Einordnen, Bündeln, Pille); geändert `web/api.js` (`nachrichtSenden`), `web/hauptansicht.js`, `web/lc-seite.css`.
- Tests: Unit im jeweiligen Modul; Integration neu `tests/senden.rs`, `tests/senden_strom.rs`, `tests/schreibkonto.rs`, Messwert in `tests/verfuegbarkeit.rs`; Oberfläche rein `e2e/einheit/eingabe.test.mjs`, `zustellung.test.mjs`, `live-verlauf.test.mjs`; Abläufe `e2e/tests/r00011-*.spec.js`.

## Technische Überlegungen

### Ablauf

1. **Senden** `POST …/nachrichten`: Grenzschutz → Teilnehmer (401) → Vorab-Buchung (429) → Körper begrenzt lesen (413), Content-Type (415), Parse in `spawn_blocking` unter der Semaphore (400) → Channelname (404) → Sendekörper (422 `text`/`feld`) → Textregel (422 `leer`/`laenge`/`zeichen`) → Restbuchung (429) → `spawn_blocking(senden)` → 201 | 500.
2. **`senden`** (Gespräche): Gesprächssperre → Schreibsperre → Zeit `max(jetzt, letzte + 1 ms)` → Satz mit `+ art:` anhängen → Index → Rückruf `veroeffentlichen` (Bus) → Vorlauf-Atom setzen, ggf. stderr → beide frei.
3. **Strom**: Bus → je Abonnent `{"art":"nachricht",…}`.
4. **Tab, eigene Nachricht**: `Enter` → `taste` → Zustellung `senden` → vorläufige zeichnen → ggf. ans Ende → ältester Auftrag `nachrichtSenden` → Antwort oder eigenes Ereignis bestätigt → zeichnen.
5. **Tab, fremde Nachricht**: Ereignis → Einordnen (Zeit > jüngste) → Sammler → rAF → einfügen | zählen → Pille.

### Grobentwurf (Vorplanung)

Die Bubbles aus der WBS als Datenfluss — **Vorplanung**: welche es am Ende werden, entscheidet der Entwickler beim Bauen.

- F0041: B0206 Textregel prüfen (rein) · B0207 Sendekörper lesen (rein) · B0208 Art als Metazeile (rein) · B0209 Nachricht mit Art senden und lesen · B0210 Nachricht als JSON mit Art · B0211 Großen Körper außerhalb des Workers lesen (unklar) · B0212 Senden beantworten
- F0042: B0213 Eine Schreibsperre über alle Gespräche (unklar) · B0214 Nachricht veröffentlichen · B0215 Vorlauf bestimmen (rein) · B0216 Vorlauf melden
- F0043: B0217 Taste auswerten (rein) · B0218 Zustellung im Tab (rein, unklar) · B0219 Nachricht senden abrufen · B0220 Eingabe zeichnen (unklar) · B0221 Vorläufige und fehlgeschlagene zeichnen · B0222 Senden verdrahten (unklar)
- F0044: B0223 Einordnen über die Zeit (rein) · B0224 Je Frame bündeln · B0225 Folgen oder zählen (rein) · B0226 Pille „n neue Nachrichten“ (unklar) · B0227 Absender-Art zeichnen · B0228 Live-Nachrichten verdrahten (unklar) · B0229 Ausbruch messen (Testhilfe Z9)
- F0045: B0230 Eimer und Kosten (rein) · B0231 Abbuchen über beide Eimer · B0232 Volle Eimer entfernen (rein) · B0233 Senden bucht in zwei Schritten · B0234 Gebremst beantworten · B0235 Bremsen sichtbar · B0236 Flut mit Sitzung messen (Testhilfe M2)

**Elemente** (nur öffentliche Signaturen, nach A00004, A00006, A00007, A00011, A00014, A00005):

- `Art` (Wert) — `Mensch` | `Agent`; `Nachricht { zeit, von, art: Option<Art>, text }`.
- `fn text_pruefen(text: &str) -> Result<(), Textfehler>` mit `Textfehler = Leer | Laenge { bytes } | Zeichen { stelle, zeichen }`; Konstante `TEXT_HOECHSTENS_BYTES = 16_384`.
- `fn sendekoerper_lesen(wert: &Value) -> Result<&str, Koerperfehler>` mit `Koerperfehler = Text | Feld { feld }`.
- `Gespraeche::senden(&self, gespraech, von: &Kennung, art: Art, text: &str) -> Result<Nachricht, Sendefehler>`; Sperre `schreiben` hält die letzte Zeit; `fn zeit_vorlauf_ms(&self) -> u64` (Atom).
- `Ereignis::Nachricht { channel, nachricht }`.
- `Schreibkonto` — `fn abbuchen(&self, kennung: &Kennung, herkunft: IpAddr, kosten: u32) -> Result<(), Gebremst>`, `Gebremst { grenze: Kennung | Herkunft, wieder_in: Duration }`, `fn kosten_senden(bytes: usize) -> u32`, `fn gebremst(&self) -> u64`; Konstanten Vorrat/Nachschub je Eimer; Uhr über den Provider.
- Oberfläche (rein): `taste(ereignis, text) → "senden" | "zeile" | null`; Zustellung `zustellen(zustand, ereignis) → {zustand, auftrag?}`; `einordnen(bekannt, nachrichten) → neue`; `folgenOderZaehlen(abstand, zeilenhoehe, neuesterImFenster) → "folgen" | "zaehlen" | "nurZaehlen"`; am Port (`web/api.js`): `nachrichtSenden(channel, text, schluessel)`.

Wireframe: die WBS führt `wireframes: Dokumentation/Wireframes/`, ein Artboard `D0003.dc.html` fehlt (Hinweis `/wireframe verfeinern D0003`). Gestaltungsquelle: Designsystem `components/Eingabe/README.md` (Feld, Tasten), `components/Nachricht/README.md` (Zustände „wird gesendet …“, fehlgeschlagen), `components/VerlaufsMarken/README.md` (Pille, Folgen), `components/Kennung/README.md` (Marke „Agent“) — Vorgabe für die Gestaltung, nicht Akzeptanzkriterium.

### Änderungen an bestehenden Klassen

- `gespraeche::Gespraeche::senden`: Art, Schreibsperre über alle Gespräche statt `letzte`, Rückruf `veroeffentlichen` unter der Sperre; `Nachricht` um `art`.
- `gespraeche::logformat`: Metazeile `+ art:` schreiben und lesen.
- `http::nachrichten::NachrichtAntwort` (seit R00013, B0254, die eine Form einer Nachricht an der Grenze; heute privat, `{zeit, von, text}`): öffentlich gemacht, damit die 201 und das Ereignis im Strom (F0042, `src/http/ereignisse.rs`) dieselbe Struktur serialisieren; um `art` ergänzt (`null` ohne Metazeile). Kein zweiter Weg zu JSON — `nachricht_als_json` gibt es seit R00013 nicht mehr.
- `http::koerper`: Parse großer Körper außerhalb des Workers.
- `http::gesundheit`: `zeit_vorlauf_ms`, `gebremst`.
- `live::bus`: `Ereignis::Nachricht`.
- `web/verlauf.js` (aus R00007): Thema `nachricht`, Bündeln, Pille, Absender-Art; `web/hauptansicht.js`: Eingabe unter dem Verlauf.

## Tests

Nach Skill `test-pyramide` und ADR-0009; die Szenarien der User Story werden Tests. Tests sind nie schärfer als diese Anforderung: sie übernehmen Z1, Z10 und die exakten Regeln wörtlich; Z9 und M2 sind Messwerte, keine Tests im Gate.

**Unit (rein, gestellte Uhr):** Textregel (Spanne in Bytes, Klassengrenzen der Steuerzeichen, JSON-Ausdehnung ≤ 2 ×, Mutanten), Sendekörper, Metazeile, Schranken-Test der Schreibsperre (vorher rot), Sperrordnung mit der Pflege, Vorlauf (Schwelle 60 000/60 001 ms), Eimer (30/31, 120/121, 999/1 000 ms, Mutanten), Kosten, zwei Schritte, Speicher, Episode.

**Integration (`tests/`, echtes Programm, ohne `Origin`):** Rundweg POST → Datei → GET → Neustart; Art Mensch/Agent/null; U+0001 abgewiesen, Tab und Zeilenumbruch angenommen; Körpergrenze 131 072/131 073; Abweisungen und Reihenfolge; Plattenfehler; Ereignis und kein Ereignis bei Fehlern; 40 × 25 über zwei Channels; Vorlauf; Konto je Kennung und je Herkunft als Spanne, Vorab vor dem Körper, kein Gegenzug, Betrieb, Mutationsprobe.

**Oberfläche rein (`e2e/einheit/`):** Tasten, Zustellung (Z4), Einordnen, Bündeln, Folgen oder zählen.

**E2E (Playwright):** die Szenarien aus F0043 und F0044 mit zwei Kontexten; Z1 ereignisgenau (`waitForFunction` mit `polling: 'raf'`), nie im Standardraster von `expect`; Z3 mit um 2 s verzögerter Route (Gelerntes der Factory: UI-Tests mit verzögert antwortendem Fake); Fehler mit gestellten Antworten der Route.

**Entwicklertest, Messwerte (kein Gate):** Z9 mit B0229, M2 mit B0236.

## Abhängigkeiten

- Abhängig von: I0002 Nachricht landet im Log (R00002, grün) — Anhängen, Index, Zeitregel; I0004 bis I0007 (grün) — Sitzung, Grenzschutz, Strom, Channels; I0008 Channel wechseln und Verlauf lesen (R00007, rot, im Bau) — F0041 braucht F0027 (Verlauf-API), F0043 und F0044 brauchen F0029 (Verlauf öffnen) und F0031 (Fenster, „Zum Neuesten“); F0040 Körpergrenze je Route (R00010, rot) — F0041 braucht die Grenze je Route.
- Innerhalb (WBS `Braucht`): F0041 Welle 1; F0042 und F0045 brauchen F0041 — Welle 2 (parallel); F0043 und F0044 brauchen F0042 — Welle 3 (parallel).
- Blockiert: F0046–F0049 (Rest von I0009), I0010 Anhänge, I0011 Suche, I0012 Direktnachricht, I0013 Erwähnungen, I0015 Nachricht über die API senden, I0016 Auf Nachrichten warten (ein Zeiger `seit`, erst mit der Schreibsperre verlustfrei), I0017 Markdown, I0019 Claude Code einbinden (`lc senden` hält `Retry-After` ein).

## Offene Fragen

- **An den Director/Betreiber: Teilung von I0009** (blockiert R00011 nicht). I0009 hat 47 Bubbles (gezählt), über der Schwelle von etwa 35 aus dem Auftrag. Vorschlag: I0009 behält F0041–F0045 (31 Bubbles, Gegenstand dieser Anforderung; erfüllt das Fertig-Kriterium „erscheint bei allen offenen Sichten sofort“ für Agent, Absender und alle anderen Sichten, mit Schutz gegen Flut); eine neue Interaction in D0003 nimmt F0046–F0049 (16 Bubbles: Wiederholung, Nachholen, Entwurf, Ansage; Braucht I0009) und bekommt R00012. Ohne Teilung bekommt I0009 eine zweite Anforderung R00012 über F0046–F0049.
- **An den Betreiber (Richtungsfrage): Bucht die Frist am Schreibkonto?** ADR-0036 empfiehlt für `PUT /api/channels/{name}/frist` pauschal 10 Einheiten (Wechsel-Flut M3 aus R00008); nicht Teil von R00011.
- **An den architekt (Wortlaut, entschieden für R00011):** (1) `wieder_in_ms` reicht bis zu den **vollen** Kosten (Vorab plus Rest); das Rechenbeispiel in ADR-0036 („die 8. wartet 2 s“) wird damit 3 s. (2) Eine stderr-Episode des Kontos endet nach 30 s ohne 429 der Kennung. (3) Die Texte der Abweisung nennen keine Kennungen der Planung (ADR-0038 Regel 4 „oder ab I0010 als Anhang“ entfällt) und zählen die Stelle für Menschen ab 1. (4) Die Tests aus ADR-0035 (20 × 50) und ADR-0037 Z9 (ein Agent, 120) sind auf 40 × 25 bzw. 4 × 30 umgestellt, weil sie sonst am Konto scheitern.

Entschieden vom planer für R00011 (Rückfragen mit Empfehlung beantwortet):

- **Fehlgeschlagen mit Grund gehört zu F0043**, nicht erst zu F0046 — sonst stünde im Hauptweg eine abgewiesene Nachricht für immer „wird gesendet …“; F0046 bringt die Schalter, die Zeitgrenze, 401 und das Puffern.
- **Textprüfung vor dem Senden gehört zu F0048** (nicht zu R00011); bis dahin weist der Server ab, und F0043 zeigt den Grund an der Nachricht.
- **Marke „Agent“ in F0044**: I0008 hat die Art des Absenders ausdrücklich nach I0009 verwiesen; der Zuschnitt des architekt nannte sie nicht.
- **Ordnung über Channels mit 40 × 25 auf 10 Herkünften** und **Z9 mit 4 × 30**: derselbe Zweck wie in den ADRs, gültig vor und nach dem Konto.
- **Flut M2 an F0045** statt an F0041: gemessen wird der Endzustand, in dem das Konto die Vorab-Einheit vor dem Lesen bucht; der Parse außerhalb des Workers (F0041) bleibt Teil desselben Laufs.
- **Kombination „Antwort verloren und Strom getrennt“** (Z7/Z8) gehört zu F0046, nicht hierher.
- **Z1 mit Absender über die API**: so ist F0044 ohne F0043 prüfbar (parallel); die eigene Nachricht im Browser prüft Z3.

## Manuelle Vorbereitungstätigkeiten

- Für Z9 und M2: ein ruhiger Rechner (`uptime` unter 2), Chromium aus `e2e/node_modules`, Fenster 1280 × 800, Release-Build.
- Für die Kriterien mit `chmod a-w`: Testlauf als normaler Benutzer, nicht als root.
- Quelladressen `127.0.0.2` bis `127.0.0.11` sind unter Linux ohne Einrichtung über Loopback erreichbar.

## Manuelle Nachbereitungstätigkeiten

- ADR-0037 und ADR-0036: Z9 und M2 unter „Ergänzungen“ eintragen; den Wortlaut aus „Offene Fragen“ (an den architekt) angleichen — architekt über den Director.
- `/wireframe verfeinern D0003`, sobald der Betreiber die Gestaltung der Mitte sehen will.

## Umfang

Aus der WBS gezählt (`/planung zaehlen` je Slice); Standard-Bubbles mit der Untergrenze 0,4h (Ist-Zeiten aus `Schaetzungen/_ist-zeiten.md`, Typ Standard, liegen darunter; die Untergrenze gilt nach Skill), unklare als Richtwert 2–4h (ungemessen):

- F0041 Senden über die API = 7 Bubbles: 6 Standard (0,4h), 1 unklar (2–4h). Rest: 2,4h klar + 2–4h unklar.
- F0042 Nachricht live im Strom = 4 Bubbles: 3 Standard (0,4h), 1 unklar (2–4h). Rest: 1,2h klar + 2–4h unklar.
- F0043 Eingabe und eigene Nachricht im Browser = 6 Bubbles: 3 Standard (0,4h), 3 unklar (2–4h). Rest: 1,2h klar + 6–12h unklar.
- F0044 Fremde Nachrichten im offenen Verlauf = 7 Bubbles: 5 Standard (0,4h), 2 unklar (2–4h). Rest: 2,0h klar + 4–8h unklar.
- F0045 Schreibkonto = 7 Bubbles: 7 Standard (0,4h). Rest: 2,8h klar.
- R00011 gesamt = 31 Bubbles: 24 Standard, 7 unklar. Rest: 9,6h klar + 14–28h unklar (Richtwerte ungemessen).
- Nicht enthalten: F0046–F0049 (16 Bubbles, ohne Anforderung bis zur Entscheidung über die Teilung); Senden in Direktgespräche (I0012/I0015), Markdown (I0017), Anhänge (I0010), Buchung der Frist (offene Frage).

## Warum löst diese Anforderung das Problem? (Pflicht)

Auslöser: Die Vision verspricht ein Gespräch zwischen Menschen und Agenten, das „sofort in allen offenen Sichten“ erscheint, aber bis R00010 gibt es keinen Weg, eine Nachricht zu schreiben. Wenn Browser und Agent dieselbe Route mit derselben Antwortform nutzen, dann sieht der Sender genau das, was jeder Leser später sieht, und „gleichberechtigt“ ist eine geprüfte Eigenschaft (Integrationstest ohne `Origin`). Wenn Zeit, Log, Index und Strom unter **einer** Sperre in **einer** Folge entstehen, dann kann jede Sicht — Tab, Strom, später `warten` — mit einem Vergleich über die Zeit entscheiden, was neu ist, und nichts geht verloren oder doppelt; deshalb steht die eigene Nachricht vorläufig sofort da und wird ohne Doppel bestätigt, gleich welcher Weg zuerst ankommt. Wenn fremde Nachrichten je Frame gebündelt und, wer nicht am Ende liest, nur gezählt werden, dann bleibt „sofort“ auch bei einem Agenten im Takt bedienbar, statt den Leser aus seiner Stelle zu reißen. Und wenn jedes Senden vorab am Konto bucht, dann kann ein entlaufener Agent die Instanz nicht fluten — der Hebel ist die Buchung **vor** dem Lesen des Körpers, weil sie gültige wie ungültige Fluten mit demselben Zähler begrenzt.

## Missing-Docs

- Glossar fehlt (`Dokumentation/Domaenenwissen/` existiert nicht); Begriffe wie Sendeauftrag, vorläufig, Schreibkonto, Eimer, Episode, Vorlauf sind aus ADR-0034 bis ADR-0038 übernommen.
- Kein Artboard für D0003.
- Die Gestaltung der Pille „n neue Nachrichten“ und der Zeile für den leeren Channel steht nur im Designsystem-Text (`VerlaufsMarken`, `Hauptansicht`), ohne Bild.

## Notizen

### Verworfene Alternativen

- **Eine Anforderung über alle neun Slices von I0009**: verworfen — eine Anforderung wird nach dem Schreiben nie geändert; wird I0009 wie vorgeschlagen geteilt, wäre sie für I0009 falsch und für die neue Interaction nicht da.
- **Je Slice eine Anforderung**: verworfen — F0041–F0045 haben ein Ziel (das Fertig-Kriterium von I0009) und durchlaufen gemeinsam Entwicklertest und Abnahme.
- **Z1 mit Absender im Browser A**: verworfen — F0044 hinge dann an F0043; der Absender über die API misst dieselbe Zustellung.
- **20 × 50 aus einer Kennung je Task** (ADR-0035): verworfen — scheitert ab F0045 am Konto; 40 × 25 auf 10 Herkünften bleibt unter beiden Eimern.
- **Texte mit Kennungen der Planung** („ab I0010 als Anhang“, ADR-0038): verworfen — Oberflächentexte nennen keine Planungskennungen.

### Review 2026-10-28 (planer, `/anforderung review` im Auftrag des Directors)

- Grobentwurf und „Änderungen an bestehenden Klassen“: `nachricht_als_json` wurde mit R00013 (Commit 80d1e89) entfernt; die eine Form an der Grenze ist `NachrichtAntwort` — B0210 macht sie öffentlich und ergänzt `art`.
- F0041: Kriterium „Steuerzeichen und JSON-Ausdehnung“ ergänzt (Hinweis aus dem Security-Review zu #23: Steuerzeichen blähen JSON auf das 6-Fache). Die Regel selbst stand schon da (verbotene Zeichen, ADR-0034); neu sind Klassengrenzen als Spanne mit Mutanten, die Ausdehnungsgrenze und die Probe über HTTP mit U+0001. Tab und Zeilenumbruch bleiben erlaubt (nicht strenger als die Regel). Die Alternative „Budget zählt 6 Bytes je Steuerzeichen“ entfällt, weil Senden sie abweist.

### Bewusst außerhalb

Schalter „Erneut senden“ und „Verwerfen“, Zeitgrenze 10 s, 401 mit Wiederbeitritt, Puffern bei getrenntem Strom, Wiederholschlüssel auf dem Server (F0046); Lücken nach Wiederverbinden (F0047); Entwurf je Channel, Längenanzeige, Textprüfung vor dem Senden, `/` und `Esc` in die Eingabe (F0048); gebündelte Ansage (F0049); Codeblock-Regel und Leiste (I0017, I0010); `↑` in leerer Eingabe; Senden in Direktgespräche (I0012, I0015); `/api/openapi.json` (I0015); Buchung der Frist (offene Frage).
