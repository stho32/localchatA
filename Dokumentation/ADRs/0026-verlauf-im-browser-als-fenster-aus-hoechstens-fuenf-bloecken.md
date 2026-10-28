# ADR-0026: Verlauf im Browser als Fenster aus höchstens fünf Blöcken — Nachladen an beiden Enden, Anker von Hand; was „sofort“ und „flüssig“ messbar heißt

**Status**: Akzeptiert
**Datum**: 2026-10-28
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #7 I0008)
**Kontext-Anforderung**: — (WBS I0008; prägt I0009, I0011, I0017, I0010)
**Erprobt durch**: Wegwerf-Probe im Scratchpad (2026-10-28, Chromium headless aus `e2e/node_modules` Playwright 1.57, 1280 × 800, 8 Kerne, Last ≈ 1,2): HTTP-Attrappe mit 100.000 Nachrichten wie `tests/leistung.rs` nach [ADR-0025](0025-verlauf-ueber-die-api-bloecke-mit-zeit-als-cursor.md); je Variante 20 × Öffnen, dann Mausrad (1.000 px je 16 ms) über 20.000 Nachrichten nach oben, dann 10 × an den oberen Rand ziehen (`scrollTop = 0`); gemessen im Fenster: Abstand der `requestAnimationFrame`-Takte, `longtask`-Einträge, Lage der obersten sichtbaren Nachricht vor/nach jedem Nachladen; Knoten und JS-Heap über CDP. Ergebnisse in der Tabelle unten. Probe danach gelöscht.

## Kontext

- Vision „Schnell, auch bei großem Verlauf“: mit 100.000 Nachrichten und mehr in der Frist öffnen Channels **sofort**, der Verlauf scrollt **leichtgängig**. Fertig-Kriterium I0008: „bei 100.000 Nachrichten öffnet der Channel sofort und scrollt flüssig“.
- Designsystem: Blöcke zu 100; Nachladen nach oben ab etwa zwei Bildschirmhöhen vor dem Rand, die Ansicht bleibt auf der gelesenen Nachricht verankert („nichts springt“); „Zum Neuesten“ ständig erreichbar; feste Höhen — lange Nachrichten ab `lc-collapse` (240 px) eingeklappt, Bilder reservieren ihren Platz; „Billig je Nachricht“ (keine Schatten, keine Animation je Zeile).
- `AI-Team/planer.md` und `entwickler.md`, Gelerntes #4/#5: jede Zeitgrenze nennt Zweck, Startpunkt und erwarteten Wert und liegt mindestens 30 % darüber; Gate und Messwert getrennt; Leistungsgrenzen einmal in der Anforderung, der Test nie schärfer; Zeittests ereignisgenau (`polling: 'raf'`); Wertprüfungen als Spanne mit Mutanten.
- Q1 in [A00001](../Architektur/A00001-localchata.md) misst nur den Server (< 50 ms je Block); was der Mensch sieht, entscheidet der Browser.
- Gemessene Höhe je Nachricht mit den Testdaten: 156 px bei 1280 px Breite, 223 px bei 320 px — ein Block zu 100 ist ≈ 15.600 px hoch, rund 20 Bildschirme zu 800 px. Kurze Agenten-Nachrichten (eine Zeile, 28 px) ergeben ≈ 2.800 px je Block.

## Optionen

### Option A: Fenster — höchstens fünf Blöcke im DOM; oben nachladen, unten abwerfen und umgekehrt; Anker von Hand
- Vorteile: DOM und Speicher haben eine feste Obergrenze (500 Nachrichten), egal wie weit man scrollt; jedes Nachladen kostet gleich viel; der Browser scrollt nativ (Mausrad, Touchpad, `Bild ↑`, Scrollbalken innerhalb des Fensters).
- Nachteile: der Scrollbalken zeigt nur das Fenster, nicht den ganzen Verlauf; Zurückscrollen lädt abgeworfene Blöcke neu; eigene Anker-Korrektur.

### Option B: Alles Geladene bleibt im DOM; Anker von Hand
- Vorteile: kein Abwerfen, kein Neuladen beim Zurückscrollen; einfachster Code.
- Nachteile: DOM wächst mit dem Lesen — bei 20.000 gelesenen Nachrichten 162.000 Knoten, auf 100.000 hochgerechnet rund 810.000; Layout-Kosten wachsen mit (lange Aufgaben bis 147 ms schon bei 20.000).

### Option C: Alles Geladene bleibt, `content-visibility: auto` je Nachricht, Anker durch den Browser (`overflow-anchor`)
- Vorteile: der Browser überspringt das Zeichnen außerhalb des Blicks; keine eigene Anker-Mechanik.
- Nachteile: in der Probe am langsamsten (Platzhalterhöhen werden beim Scrollen laufend korrigiert); am oberen Rand (`scrollTop = 0`) verankert der Browser nicht — jeder Block schiebt den Blick um seine Platzhalterhöhe (8.800 px) und löst sofort den nächsten Abruf aus (Kaskade über 214 Blöcke); Safari kennt `overflow-anchor` nicht.

### Option D: Vollständige Virtualisierung — nur sichtbare Zeilen im DOM, Scrollbalken über alle 100.000 mit geschätzten Höhen
- Vorteile: echter Scrollbalken über den ganzen Verlauf; DOM noch kleiner als A.
- Nachteile: braucht Positionen statt Zeiten in der API (Option B in ADR-0025, wandernde Positionen); Markdown-, Code- und Tabellenhöhen sind vorab nicht bekannt — jede falsche Schätzung verschiebt Scrollbalken und Blick; deutlich mehr eigener Code ohne Build-Schritt (ADR-0002). Nicht geprobt, weil schon die API-Voraussetzung verworfen ist.

| Messung (Probe) | A Fenster | B alles | C content-visibility | A, CPU 4 × gedrosselt¹ |
|---|---|---|---|---|
| Öffnen, Klick → neueste 100 gezeichnet, n = 20 (min / Median / max)² | 39,9 / 44,0 / 47,2 ms | 39,4 / 47,0 / 57,7 ms | 39,2 / 43,5 / 47,2 ms | 63,8 / 78,6 / 186,6 ms |
| Frames beim Hochscrollen (Median / p95 / max) | 16,7 / 16,8 / 33,4 ms | 16,7 / 16,8 / 283 ms | 33,4 / 150 / 567 ms | 16,7 / 16,8 / 117 ms |
| Frames über 50 ms | 0 von 6.341 | 139 von 6.372 | 2.435 von 6.894 | 13 von 1.600 (0,8 %) |
| lange Aufgaben (Anzahl, längste) | 0 | 125, 147 ms | 2.633, 630 ms | 7, 100 ms |
| Sprung beim Nachladen (199 bzw. 50 ×) | 0 px | 0 px | 0 px | 0 px |
| Sprung am oberen Rand (10 ×) | 0 px | 0 px | 8.800 px, Kaskade | 0 px |
| Abruf je Block (Median / max) | 12 / 22 ms | — | — | 7 / 19 ms |
| DOM nach 20.000 Nachrichten | 500 Nachrichten, 4.074 Knoten | 20.000, 162.219 Knoten | 21.400, — | 500, 4.074 Knoten |
| JS-Heap nach GC | 2,0 MB | 2,5 MB | 3,7 MB | 2,0 MB |
| Dauer 20.000 Nachrichten hoch | 106 s | 119 s | 374 s | — (5.000) |

¹ CDP `Emulation.setCPUThrottlingRate 4`, über 5.000 Nachrichten — Stellvertreter für einen schwächeren Rechner oder ein Gate unter Bauast. ² gemessen bis nach zwei `requestAnimationFrame`, enthält also bis zu 33 ms Wartezeit auf Frames; der sichtbare Zeitpunkt liegt früher. Der JS-Heap enthält die DOM-Knoten nicht; ihr Maß ist die Knotenzahl.

## Entscheidung

**Gewählt: Option A**

Es überwiegt, dass nur das Fenster Speicher und Arbeit je Bild unabhängig davon hält, wie weit jemand zurückliest — in der Probe ohne einen Frame über 50 ms, ohne lange Aufgabe und ohne Sprung, auch am oberen Rand und bei vierfach gedrosselter CPU noch mit 99,2 % ruhiger Frames. In Kauf genommen: der Scrollbalken zeigt nur das Fenster, Zurückscrollen lädt abgeworfene Blöcke erneut (≈ 12 ms je Block), und die Anker-Korrektur ist eigener Code mit eigenen Tests.

Regeln:

1. **Fenster**: höchstens **5 Blöcke zu 100** Nachrichten (500) im DOM und im Speicher des Tabs. Fünf Blöcke tragen auch kurze Nachrichten auf einem 4K-Schirm (5 × 2.800 px ≥ 2 Bildschirmhöhen darüber + Blick + 2 darunter bei 2.160 px). Lädt ein Block über die Grenze, wird am anderen Ende ein Block abgeworfen und `neuer` bzw. `aelter` gemerkt.
2. **Nachladen**: oben, wenn der Abstand zum oberen Rand unter **2 Bildschirmhöhen** fällt und `aelter` gilt (`vor=<erste Zeit>`); unten, wenn der Abstand zum Ende unter 2 Bildschirmhöhen fällt und `neuer` gilt (`nach=<letzte Zeit>`). Höchstens ein Abruf zugleich; nach jedem Abruf wird die Bedingung erneut geprüft (so holt auch ein Sprung an den Rand des Scrollbalkens Block für Block nach, ohne Kaskade). Ausgelöst vom `scroll`-Ereignis (`passive`), also auch von Tastatur und Scrollbalken.
3. **Anker von Hand, in einem Zug**: vor jeder Änderung oberhalb des Blicks (Block vorn einfügen, Block oben abwerfen, Kopf der Grenznachricht wegen Gruppierung entfernen, Tagestrenner an der Blockgrenze) die oberste sichtbare Nachricht und ihren Abstand zum oberen Rand merken; nach allen Änderungen `scrollTop` um die Verschiebung dieser Nachricht korrigieren — im selben Task, vor dem nächsten Frame. Der Verlauf trägt `overflow-anchor: none`, damit der Browser nicht zusätzlich korrigiert. (Die Probe korrigierte über die Differenz von `scrollHeight`; die Korrektur über die Lage der Ankernachricht deckt zusätzlich Höhenänderungen an der Blockgrenze ab.)
4. **Vorhersehbare Höhen**: lange Nachrichten werden mit CSS (`max-height: var(--lc-collapse)`, `overflow: hidden`) eingeklappt, ohne beim Laden zu messen; kein `content-visibility` (Option C); Bilder reservieren ab I0010 ihren Platz aus den Metadaten. Gruppierung und Tagestrenner entstehen beim Zeichnen eines Blocks aus `von` und `zeit`, auch über die Blockgrenze hinweg (Regel 3).
5. **Öffnen**: neueste 100 holen (ein Abruf ohne Cursor), zeichnen, ans Ende scrollen. Solange der Blick am Ende steht, bleibt er dort, auch wenn sich Höhen nachträglich ändern (Schriften geladen, Fenster verkleinert) — ein `ResizeObserver` auf dem Inhalt setzt dann `scrollTop` neu.
6. **Zum Neuesten**: sobald der Blick mehr als eine Bildschirmhöhe über dem Ende steht oder `neuer` gilt, steht die Pille „Zum Neuesten“ (Designsystem `VerlaufsMarken`); Klick oder `Ende` öffnet neu wie Regel 5. Der Fokus geht dabei in den Verlauf.
7. **Kriterien** (für den `planer`; Leistungsgrenzen stehen einmal in der Anforderung, Tests übernehmen sie):

   | # | Was | Art | Startpunkt der Uhr / Messweg | erwartet | Grenze | Zweck |
   |---|---|---|---|---|---|---|
   | L1 | Block über HTTP: neueste 100 und ein Block mitten im Verlauf (`vor`) bei 100.000 Nachrichten in **einem** Channel | Gate (Integration, echtes Programm, Debug) | Anfrage gesendet → Antwort vollständig gelesen, je 20 Anfragen, größter Wert | ≈ 2–10 ms (gerechnet: Block 0,06 ms im Release laut ADR-0006, dazu ≈ 63 KB JSON im Debug-Build) | < 50 ms (Q1) | der Server hängt nicht an der Länge des Verlaufs |
   | L2 | Channel öffnen im Browser bei 100.000 Nachrichten | Gate (E2E) | `click` auf `# name` (Zeitstempel des Ereignisses im Fenster) → neueste Nachricht mit Absender und Zeit sichtbar im Verlauf, ereignisgenau (`waitForFunction`, `polling: 'raf'`, Warten bis 10 s) | ≈ 45 ms (Probe), 4 × gedrosselt bis ≈ 190 ms | < 500 ms | „öffnet sofort“ heißt: Öffnen hängt nicht an der Länge des Verlaufs — wer den ganzen Verlauf (≈ 63 MB JSON) holt, liegt um Größenordnungen darüber |
   | L3 | Fenster begrenzt | Gate (E2E, deterministisch) | nach Hochscrollen über 1.000 Nachrichten (10 × Nachladen) | genau 500 Nachrichten im DOM | genau 500 — Mutanten „4 Blöcke“ und „6 Blöcke“ werden rot | Speicher und Arbeit je Bild bleiben begrenzt, egal wie weit man liest |
   | L4 | nichts springt | Gate (E2E, deterministisch) | je Nachladen (oben und unten) und je Sprung an den oberen Rand: Lage der zuvor obersten sichtbaren Nachricht | 0 px | höchstens 1 px Abweichung | „Nichts springt“ (Designsystem) |
   | L5 | lückenlos | Gate (E2E, deterministisch) | Hochscrollen über 1.000, dann zurück bis zum Neuesten | jede Zeit genau einmal, streng steigend, die neueste ist die jüngste des Channels | — | Abwerfen und Neuladen verlieren nichts |
   | M1 | scrollt flüssig | **Messwert**, kein Gate (Frame-Zeiten unter Bauast unzuverlässig) | Mausrad über 2.000 Nachrichten, `requestAnimationFrame`-Abstände und `longtask` | 0 % Frames über 50 ms, keine lange Aufgabe (4 × gedrosselt: 0,8 %, längste 100 ms) | höchstens 2 % der Frames über 50 ms, keine Aufgabe über 250 ms | „leichtgängig“ (Vision) als Zahl für Entwicklertest und Abnahme |
   | M2 | öffnet sofort, wie der Mensch es sieht | **Messwert** | wie L2, Median aus 20 Öffnungen ohne Drosselung | ≈ 45 ms | Median ≤ 100 ms | unter 0,1 s gilt eine Antwort als unmittelbar |

   Für L2–L5 und M1/M2 liegt ein Channel mit 100.000 Nachrichten über 7 Tage im Wegwerf-Ordner (Generator wie `tests/leistung.rs`, Start < 2 s laut R00002); Fenster 1280 × 800.
8. **Wechsel und Speicher**: je geöffnetem Channel ein `AbortController`; eine Antwort für einen nicht mehr gewählten Channel wird verworfen. Beim Wechsel fallen DOM und Daten des alten Channels weg — kein Zwischenspeicher je Channel (Öffnen kostet ≈ 45 ms).
9. **Zugänglichkeit**: der Verlauf ist `role="log"` mit `aria-live="off"` und `tabindex="0"` (Bild ↑/↓ scrollt und lädt nach); Nachladen wird nie angesagt. Die gebündelte Ansage neuer Nachrichten (Designsystem, Barrierefreiheit) kommt mit I0009 als eigene Statusregion. „Ältere Nachrichten werden geladen …“ steht als 28-px-Zeile oben, solange ein Abruf nach oben läuft.

## Konsequenzen

**Positiv:**
- Speicher im Tab bleibt bei ≈ 500 Nachrichten (≈ 4.100 Knoten), auch nach Stunden Lesen; Arbeit je Nachladen ist ein Block.
- Die Leistungskriterien L3–L5 sind deterministisch — sie wackeln nicht unter Bauast; nur L1/L2 sind Zeiten, mit mehr als dreifacher Reserve.
- Live-Nachrichten (I0009) und Suchtreffer (I0011) können mit demselben Fenster „an eine Zeit springen“ ([ADR-0025](0025-verlauf-ueber-die-api-bloecke-mit-zeit-als-cursor.md) `vor`/`nach`).

**Negativ / Inkauf zu nehmen:**
- Kein Scrollbalken über den ganzen Verlauf; der Weg an den Anfang ist `Pos1` (später), nicht der Balken.
- Wer schnell zurückscrollt, wartet je abgeworfenem Block einen Abruf (≈ 12 ms in der Probe) — bei 2 Bildschirmhöhen Vorlauf unsichtbar.
- Die Probe lief headless auf einem 8-Kern-Rechner mit einfacher Darstellung (Text und `<pre>`, ohne Markdown und Syntaxfarben); Markdown (I0017) kostet mehr je Block — M1 wird dort erneut gemessen.

**Pre-Mortem (ein Monat nach dem Start gescheitert, weil …):**
1. … der Blick beim Nachladen um eine Zeile springt, weil die Grenznachricht ihren Kopf verliert (Gruppierung) oder ein Tagestrenner dazukommt. Früh-Indikator: L4 mit Testdaten, in denen Block-Grenzen mitten in einer Gruppe und genau am Tageswechsel liegen. Gegenmaßnahme: Regel 3 (Korrektur über die Ankernachricht nach **allen** Änderungen).
2. … ein schneller Wechsel zwischen zwei Channels den Verlauf des ersten in den zweiten zeichnet. Früh-Indikator: E2E mit verzögert antwortender Route (Gelerntes „UI-Tests mit verzögert antwortendem Fake“). Gegenmaßnahme: Regel 8.
3. … nach dem Laden der Schrift der Channel nicht mehr am Ende steht und die neueste Nachricht halb verdeckt ist. Früh-Indikator: E2E mit erst später geladener Schrift (Route verzögert). Gegenmaßnahme: Regel 5 (`ResizeObserver`, solange am Ende).

**Folgeentscheidungen / Aufgaben:**
- [ ] Erster Node-Test (rein, `e2e/einheit/`): Fenster-Entscheidung als Funktion `(bloecke, aelter, neuer, abstandOben, abstandUnten, sicht) → 'aelter' | 'neuer' | nichts` und `einfuegen(fenster, block, richtung) → {fenster, abgeworfen}` — fünf Blöcke + einer oben → der unterste fällt, `neuer` wird wahr; Grenzen 2 × Sicht genau (Spanne: knapp darunter lädt, knapp darüber nicht).
- [ ] Erster E2E-Test: L3–L5 an einem Channel mit 100.000 Nachrichten; dazu Anker über eine Gruppen- und eine Tagesgrenze.
- [ ] M1/M2 als Messanleitung für den `entwicklertester` (Werte in diese ADR unter „Ergänzungen“).

## Quellen / Bezuege

- [A00005 Weboberfläche](../Architektur/A00005-weboberflaeche.md), [A00009 Verlaufsindex](../Architektur/A00009-verlaufsindex.md), [A00001 localchatA](../Architektur/A00001-localchata.md) (Q1)
- ADR-0002 (ohne Build-Schritt), ADR-0006 (Index-Kosten), ADR-0025
- Vision R00000 „Schnell, auch bei großem Verlauf“; WBS I0008; Designsystem `README.md` („Grundsätze“, „Leistung als Gestaltung“, „Barrierefreiheit“), `components/VerlaufsMarken/README.md`, `components/LangeNachricht/README.md`, `components/Nachricht/README.md`; `design-prompt.md` Abschnitt 5
- `AI-Team/planer.md` und `AI-Team/entwickler.md`, Gelerntes 2026-10-28 (#4); Logbuch Retrospektiven #4, #5; Factory `gemeinsam/gelerntes.md` („UI-Tests mit verzögert antwortendem Fake“)
- Schwelle 0,1 s für „unmittelbar“: Nielsen, *Usability Engineering* (1993), Kap. 5 „Response Times“ — aus dem Gedächtnis, in diesem Auftrag nicht geöffnet

## Ergänzungen

- **2026-10-28 (Befundkarte #23, vor I0009) — Blöcke unter 100 Nachrichten, Abwurfschutz ([ADR-0039](0039-grosse-antworten-byte-budget-je-block-werkbank-und-abrissgrenze-als-sendefrist.md)):** Ein Block hat ab jetzt höchstens 256 KiB Satzbytes; aus realistischen Nachrichten bleibt er bei 100 (größter Block zu 100: 40,8 KiB), aus 16-KiB-Nachrichten hat er 7–15. Was das im Browser heißt:
  1. **Nachladeschwelle unverändert** (Regel 2, 2 Sichthöhen): ein niedriger Block löst nach dem Einfügen die Prüfung erneut aus und holt den nächsten — Block für Block, ohne Kaskade. Fortsetzung allein über `aelter`/`neuer` (so gebaut in `web/fenster.js`, `web/verlauf.js`); ein Block mit weniger als `n` heißt nie „Ende“.
  2. **Abwurfschutz** (Regel 1 ergänzt): Hin-und-her-Laden entsteht, wenn das Fenster niedriger ist als 5 Sichthöhen + ein Block — dann fällt nach dem Laden oben unten ein Block weg, der Abstand nach unten unterschreitet 2 Sichthöhen, unten wird geladen, oben fällt einer weg, und so fort. Bei 5 Blöcken heißt das: Blockhöhe unter 1,25 Sichthöhen. Ein Block, den das Budget schneidet, ist mindestens 7 × 240 px (`lc-collapse`) = 1.680 px hoch — Annahme: jeder Text bricht um (`.lc-body` mit `overflow-wrap: anywhere`, `web/lc-seite.css`), eine Nachricht über ≈ 1,5 KB ist also eingeklappt 240 px hoch — bei 800 px Sicht sicher, bei 2.160 px (4K) nicht. Darum: über 5 Blöcke wird am anderen Ende **nur** abgeworfen, wenn der Abstand vom Blick zu diesem Ende danach noch **mindestens 2 Sichthöhen** beträgt; sonst bleibt der Block, und das Fenster wächst — höchstens bis **8 Blöcke** (8 × 1.680 = 13.440 px ≥ 5 × 2.160 + 1.680 = 12.480 px). Hat das Fenster 8 Blöcke und ist das andere Ende nicht abwerfbar, liefert `nachladen` nichts — am Rand selbst ist das andere Ende immer abwerfbar, solange 8 Blöcke höher sind als 3 Sichthöhen, man bleibt also nie oben hängen. Speicher im Tab höchstens 8 × 256 KiB Text.
  3. **Kriterien** (zu Regel 7): L3 bleibt „genau 500“ für das realistische Fixture bei 1280 × 800 (dort ist jedes Ende abwerfbar). Neu:
     - **L3b** (Node-Unit, `e2e/einheit/`): `einfuegen` mit Abstand und Höhe am anderen Ende: Abstand nach dem Abwerfen genau 2 × Sicht → abgeworfen, 2 × Sicht − 1 px → behalten (Spanne); 8 Blöcke und nicht abwerfbar → `nachladen` liefert `null`; Simulation ohne Scrollen mit Blöcken zu 1.680 px bei Sicht 2.160 px über 20 Schritte → höchstens so viele Abrufe, bis die Schwellen erfüllt sind, danach keiner. Mutanten „immer abwerfen über 5“ und „Obergrenze 9“ werden rot.
     - **L3c** (E2E): Channel aus 200 × 16 KiB, Fenster 1280 × 2160: öffnen, 2 s ruhen → danach kein Abruf mehr; hoch bis zum Anfang und zurück → jede Zeit genau einmal (wie L5), höchstens 8 Blöcke im DOM, Sprung je Nachladen ≤ 1 px (wie L4).
