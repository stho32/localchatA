---
id: R00007
status: Neu
datum: 2026-10-28
---

# R00007: Channel wechseln und Verlauf lesen

## Beschreibung

Ein Mensch wählt links einen Channel und liest seinen Verlauf: jede Nachricht mit Absender und Uhrzeit, Folgenachrichten gruppiert, Tage getrennt, lange Agenten-Nachrichten eingeklappt. Der Channel öffnet sofort am Neuesten, auch wenn er 100.000 Nachrichten hat, und beim Hochscrollen lädt der Verlauf leise nach, ohne dass etwas springt oder der Tab mit dem Lesen wächst. Ein Agent liest denselben Verlauf über die API in Blöcken nach — rückwärts oder ab der Zeit seiner letzten Nachricht. Die Adresse `#/c/<name>` macht jeden Channel verlinkbar; Neuladen, Zurück und zwei Tabs mit zwei Channels funktionieren. Dazu bekommt jeder Channel optional einen Satz Beschreibung, der in der Kopfzeile steht.

Zahlt ein auf: [Vision](R00000-vision.md) — „Schnell, auch bei großem Verlauf“ (mit 100.000 Nachrichten öffnen Channels sofort, der Verlauf scrollt leichtgängig), „Mehrere Channels“, „Das Log ist das Gedächtnis“ (jede Nachricht mit Absender und Zeit), „Mensch und KI sind gleichberechtigt“ (derselbe Verlauf über Oberfläche und API).

Entschieden vom Director nach Empfehlung (Vision nicht berührt): Gruppierungsfenster **5 Minuten** (Designsystem-Frage 4); **F0026 Beschreibung** kommt in I0008 als letzter, abtrennbarer Slice.

Slices (WBS `Dokumentation/Planung/localchata.md`, alle unter I0008 Channel wechseln und Verlauf lesen): F0027 Verlauf über die API, F0028 Channel wählen und Kopfzeile, F0029 Verlauf öffnen, F0030 Verlauf gegliedert, F0031 Fenster beim Scrollen, F0032 Channel per Tastatur wechseln, F0026 Beschreibung.

## Geschäftlicher Nutzen

Bis R00006 gibt es Channels nur als Namen in der Seitenleiste; was darin steht, liest man mit `less` im Ordner. Zwei Factories, die sich im Channel `security-entwicklung` abstimmen, brauchen einen Menschen, der mitliest, und Agenten, die nachlesen, was seit ihrer letzten Nachricht geschah. R00007 macht den Verlauf zur Hauptsache der Oberfläche und zu einer Abfrage der API — mit derselben Zeit als Kennzeichen für beide. Weil Öffnen und Nachladen nur je 100 Nachrichten kosten, bleibt der Chat auch nach Tagen voller Agenten-Befunde so schnell wie am ersten Tag. Damit ist der Weg frei für I0009 (Nachricht senden, live in allen Sichten) und I0011 (Suche mit Sprung an den Treffer).

## Funktionale Anforderungen

### Verlauf über die API (F0027; ADR-0025)

- `GET /api/channels/<name>/nachrichten` mit Sitzung (`X-LocalchatA-Sitzung`) liefert einen Block des Verlaufs; jeder mit Sitzung liest jeden Channel.
- **Abfrage**, höchstens ein Cursor: ohne Cursor die neuesten `n`; `vor=<zeit>` die `n` jüngsten mit Zeit **echt kleiner** als `zeit`; `nach=<zeit>` die `n` ältesten mit Zeit **echt größer** als `zeit`. `n` ist eine ganze Zahl von 1 bis 200, Standard 100.
- Der Cursor ist RFC 3339 mit Versatz, verglichen als Zeitpunkt (nie als Zeichenkette); er muss zu keiner Nachricht gehören. Im Query ist `+` als `%2B` zu kodieren.
- **Antwort** 200 `{channel, nachrichten, aelter, neuer}`: `nachrichten` in Zeitfolge, **älteste zuerst** (in beiden Richtungen); je Nachricht `{zeit, von, text}` — `zeit` wie im Log (Millisekunden, Versatz des Servers) und zugleich die Kennung der Nachricht, `von` die Kennung `name@herkunft`, `text` ohne die Einrückung des Logs; `aelter`/`neuer` sagen, ob es davor bzw. danach im Channel noch Nachrichten gibt. Leerer Channel: `nachrichten: []`, beide `false`.
- Ein Block reicht über beliebig viele Tage; defekte Zeilen erscheinen nie (sie sind beim Laden übergangen).
- **Abweisen statt umformen**: zwei Cursor, ein nicht lesbarer Cursor (auch durch unkodiertes `+`) → 422 `{fehler, regel:"cursor"}`; `n` außerhalb 1–200 oder keine ganze Zahl → 422 `{fehler, regel:"n"}`.
- Unbekannter oder ungültiger Channelname → 404 `{fehler}` („# name gibt es nicht.“); ohne gültige Sitzung → 401.
- **Testdaten-Erzeuger**: `cargo run --release --example verlauf-erzeugen -- <ordner> <channel> <anzahl> <tage>` schreibt `<anzahl>` Nachrichten gleichmäßig über `<tage>` Tagesdateien bis zum heutigen Datum (nach `LOCALCHATA_JETZT` bzw. Systemuhr und `TZ`) in `<ordner>/channels/<channel>/` — Texte mit 10–90 Wörtern, jede zwanzigste mit einem 45-zeiligen Codeblock, Zeiten streng steigend mit dem Versatz ihres Tages, Absender aus `claude@10.0.0.1` … `claude@10.0.0.5`; die letzte Zeit liegt vor der Startzeit. Er dient Gate, Entwicklertest und Abnahme.

### Channel wählen und Kopfzeile (F0028; ADR-0027 Regeln 1–3, 5, 7)

- Jeder Eintrag der Seitenleiste ist ein Link `#/c/<name>`; Klick, `Enter` und Mittelklick wirken wie bei jedem Link. Der gewählte Eintrag trägt `aria-current="page"` und die Fläche `kd-accent`.
- Die Auswahl folgt allein der Adresse und der Channel-Liste: `#/c/<name>` mit bekanntem Namen wählt den Channel; ein unbekannter oder ungültiger Name zeigt in der Mitte „# name gibt es nicht.“ — erscheint der Channel später in der Liste, öffnet er sich ohne Neuladen.
- Jede Auswahl wird **je Tab** gemerkt (`sessionStorage`). Lädt ein Tab ohne Adresse, setzt er den gemerkten Channel als Adresse (ohne neuen Eintrag im Browser-Verlauf); ohne Merker ist kein Channel gewählt und die Mitte sagt „Links einen Channel wählen.“ Es wird nie automatisch ein Channel gewählt.
- Ein Wechsel per Link ist ein Eintrag im Browser-Verlauf: Zurück wählt den vorigen Channel.
- **Kopfzeile**, Zone „Ort“: nach dem Instanznamen `# name` in `lc-title`; der Tab heißt „# name — localchatA“. Ohne bekannte Auswahl steht dort kein Channel und der Tab heißt „localchatA“.
- Tab-Reihenfolge: Kopfzeile → Seitenleiste → Verlauf → Teilnehmerliste.

### Verlauf öffnen (F0029; ADR-0026 Regeln 5, 8, 9)

- Die Wahl eines bekannten Channels holt seine neuesten 100 Nachrichten (ein Abruf), zeigt sie älteste oben und steht am Ende — die neueste Nachricht ist ganz sichtbar. Ändern sich danach Höhen (Schrift geladen, Fenster verkleinert), bleibt der Blick am Ende, solange er dort stand.
- Jede Nachricht zeigt den Absender als Kennung `name@herkunft` in der Mono-Schrift (`lc-id`) und die Uhrzeit `HH:MM` in `lc-time` — die Ortszeit **des Servers** aus dem Versatz der Nachrichtenzeit, unabhängig von der Zeitzone des Browsers; Datum und Uhrzeit mit Sekunden stehen im `title`. Der Text erscheint als Text (nie als HTML), Zeilenumbrüche bleiben. Die Kachel links zeigt das Kürzel ohne Art (Mensch/Agent kommt mit I0009).
- Leerer Channel: „Noch keine Nachrichten in # name.“
- Ein Wechsel bricht den laufenden Abruf des vorigen Channels ab; eine Antwort für einen nicht mehr gewählten Channel wird verworfen. Daten des vorigen Channels fallen weg (kein Zwischenspeicher je Channel).
- Scheitert der Abruf (Server weg, Fehlerantwort), steht in der Mitte „Verlauf nicht geladen — Verbindung prüfen.“ in Fehlergestalt (⚠, `kd-danger`) statt eines leeren Channels; eine neue Wahl versucht es erneut.
- Der Verlauf ist die einzige scrollende Fläche der Mitte, `role="log"`, `aria-live="off"`, mit der Tastatur fokussierbar (`tabindex="0"`; `Bild ↑`/`Bild ↓` scrollen).

### Verlauf gegliedert (F0030; ADR-0026 Regel 4; Designsystem `Nachricht`, `VerlaufsMarken`, `LangeNachricht`)

- **Gruppierung**: eine Nachricht mit demselben `von` wie die unmittelbar vorige, **höchstens 5 Minuten** nach ihr und ohne Tagestrenner dazwischen, steht ohne Kopf; ihre Uhrzeit steht in der linken Spalte und ist bei Hover sichtbar, für Screenreader immer lesbar. Die erste Nachricht des geladenen Bereichs trägt immer einen Kopf.
- **Tagestrenner** vor der ersten Nachricht jedes Tages (Datum der Nachrichtenzeit in ihrem Versatz): „Heute“ und „Gestern“ nach dem heutigen Datum des Servers, sonst „Montag, 26. Oktober 2026“; zentriert zwischen zwei Haarlinien; der Trenner des obersten sichtbaren Tages bleibt beim Scrollen oben stehen.
- **Lange Nachrichten**: ist der Inhalt höher als `lc-collapse` (240 px), wird er auf 240 px abgeschnitten; darunter die Leiste „Ganze Nachricht anzeigen · n Zeilen“ (Schalter mit `aria-expanded`). Aufklappen erweitert an Ort und Stelle, der Kopf der Nachricht bleibt an seiner Bildschirmstelle, die Leiste heißt dann „Einklappen“. Der Zustand gilt je Nachricht und Tab und wird nicht gespeichert. Bis I0009 die Art des Absenders liefert, gilt die Höhe für alle Nachrichten gleich.

### Fenster beim Scrollen (F0031; ADR-0026 Regeln 1–3, 6)

- Fällt der Abstand zum oberen Rand unter **2 Bildschirmhöhen** und gibt es ältere Nachrichten, lädt der Verlauf die nächsten 100 davor (`vor=<erste Zeit>`); ebenso unten mit `nach=<letzte Zeit>`, wenn es neuere gibt. Höchstens ein Abruf zugleich; danach wird erneut geprüft — auch ein Sprung an den Rand des Scrollbalkens holt Block für Block nach.
- Im DOM stehen höchstens **5 Blöcke** (500 Nachrichten); lädt ein sechster, fällt am anderen Ende ein Block weg.
- **Nichts springt**: die oberste sichtbare Nachricht bleibt beim Einfügen, Abwerfen, beim Wegfall eines Kopfs durch die Gruppierung, bei einem Tagestrenner an der Blockgrenze und beim Ein- und Ausblenden der Ladezeile an ihrer Stelle.
- Solange ein Abruf nach oben läuft, steht oben die Zeile „Ältere Nachrichten werden geladen …“.
- **„Zum Neuesten“**: steht der Blick mehr als 1 Bildschirmhöhe über dem Ende oder fehlen unten Nachrichten, erscheint die Pille „Zum Neuesten“ unten mittig; Klick oder `Ende` im Verlauf öffnet den Channel neu wie beim Wählen, der Fokus geht in den Verlauf.
- Nachladen wird nie angesagt.

### Channel per Tastatur wechseln (F0032; ADR-0027 Regel 4)

- `Alt ↓` wählt den nächsten, `Alt ↑` den vorigen Channel in der Reihenfolge der Seitenleiste — über die Adresse wie ein Klick; am Rand geschieht nichts; ohne Auswahl wählt `Alt ↓` den ersten, `Alt ↑` den letzten.
- Die Kürzel wirken überall in der Hauptansicht, auch im Verlauf, nicht bei offenem Dialog und nicht mit zusätzlich `Strg`, `Umschalt` oder `AltGraph`. Der Fokus bleibt, wo er ist.

### Beschreibung (F0026; ADR-0021 Regel 1, ADR-0022 Regeln 3, 5, ADR-0024 Regel 4, ADR-0027 Regel 5)

- `POST /api/channels {"name", "beschreibung"}`: `beschreibung` fehlt, `null` oder `""` = keine; sonst Text aus 1–200 Unicode-Zeichen ohne Steuerzeichen (Kategorie Cc, also auch kein Zeilenumbruch und kein Tab). Verstoß oder kein Text → 422 `{fehler, regel:"beschreibung"}`, nichts angelegt. Kein Umformen, kein Kürzen.
- Die Beschreibung steht in `einstellungen.toml` und in Antwort, Liste, Stand und Ereignis als `beschreibung`. Die Abweisung jeder gesetzten Beschreibung aus R00006 entfällt.
- **Laden**: eine `einstellungen.toml`, deren `beschreibung` die Regel verletzt, ist defekt (ADR-0022 Regel 3): `angelegt`, `angelegt_von` und `beschreibung` `null`, eine Zeile auf stderr mit Datei und Zeile, mitgezählt in `defekte_zeilen`; die Datei bleibt unangetastet.
- **Dialog** „Channel anlegen“: zusätzliches optionales Feld „Beschreibung“ (einzeilig, `maxlength` 200) unter „Name“; eine 422 `beschreibung` erscheint als Fehlerzeile wie die anderen Fehler, die Eingaben bleiben stehen.
- **Kopfzeile**: nach `# name` die Beschreibung in `lc-head-desc`, einzeilig mit Ellipse, isoliert in `<bdi>`, vollständig als `title`; unter 720 px Fensterbreite ausgeblendet; ohne Beschreibung nichts.

## Nicht-funktionale Anforderungen

- **Performance** — die Leistungsgrenzen stehen nur hier (Kriterien und User Story nennen sie über ihre Nummer); Tests übernehmen sie wörtlich und sind nie schärfer. Testdaten für alle: ein Channel `gross` mit 100.000 Nachrichten über 7 Tage aus dem Testdaten-Erzeuger, Fenster 1280 × 800, Chromium.

  | # | Was | Art | Startpunkt der Uhr / Messweg | erwartet | Grenze | Zweck |
  |---|---|---|---|---|---|---|
  | L1 | ein Block über HTTP: die neuesten 100 und ein Block mitten im Verlauf (`vor` = Zeit der 50.000. Nachricht) | Gate (Integration, echtes Programm, Debug-Build) | Anfrage gesendet → Antwort vollständig gelesen; je Art 20 Anfragen, größter Wert | ≈ 2–10 ms (gerechnet: Block 0,06 ms laut ADR-0006, dazu ≈ 63 KB JSON) | < 50 ms | der Server hängt nicht an der Länge des Verlaufs (Q1 in A00001) |
  | L2 | Channel öffnen im Browser | Gate (E2E) | Zeitstempel des `click` auf `# gross` → neueste Nachricht mit Absender und Zeit sichtbar, ereignisgenau (`requestAnimationFrame`), Warten höchstens 10 s | ≈ 45 ms (Probe ADR-0026), CPU 4 × gedrosselt ≈ 190 ms | < 500 ms | „öffnet sofort“: Öffnen hängt nicht an der Länge des Verlaufs — wer alles holt (≈ 63 MB), liegt um Größenordnungen darüber |
  | L3 | Fenster begrenzt | Gate (E2E, deterministisch) | nach Hochscrollen über 1.000 Nachrichten (10 × Nachladen) | 500 Nachrichten im DOM | genau 500 | Speicher und Arbeit je Bild bleiben begrenzt |
  | L4 | nichts springt | Gate (E2E, deterministisch) | je Nachladen oben und unten und je Sprung an den oberen Rand: Lage der zuvor obersten sichtbaren Nachricht | 0 px | höchstens 1 px | „Nichts springt“ (Designsystem) |
  | L5 | lückenlos | Gate (E2E, deterministisch) | Hochscrollen über 1.000 Nachrichten, dann zurück bis zum Neuesten | jede Zeit genau einmal | — | Abwerfen und Neuladen verlieren nichts |
  | M1 | scrollt flüssig | **Messwert im Entwicklertest, kein Gate** (Frame-Zeiten unter Bauast unzuverlässig) | Mausrad über 2.000 Nachrichten nach oben; Abstände der `requestAnimationFrame`-Takte, `longtask`-Einträge | 0 % Frames über 50 ms, keine lange Aufgabe (4 × gedrosselt: 0,8 %, längste 100 ms) | höchstens 2 % der Frames über 50 ms, keine Aufgabe über 250 ms | „leichtgängig“ (Vision) als Zahl |
  | M2 | öffnet sofort, wie der Mensch es sieht | **Messwert im Entwicklertest, kein Gate** | wie L2, Median aus 20 Öffnungen ohne Drosselung | ≈ 45 ms | Median ≤ 100 ms | unter 0,1 s gilt eine Antwort als unmittelbar |

  Reserve: L1 > 5 ×, L2 > 2,5 × über dem gedrosselten Wert, M2 > 2 ×; L3–L5 sind deterministisch. Über einer Messwert-Grenze ist ein Befund an ADR-0026, kein roter Test.
- **Zustellung** „unbekannter Channel öffnet sich, sobald er angelegt ist“ (Zweck: ein Link auf einen Channel, der gerade entsteht, bleibt nicht auf „gibt es nicht“ stehen; Start: Erhalt der 201 beim Anleger im Testprozess; Beobachter: die Kopfzeile des wartenden Tabs, Prüfintervall ≤ 25 ms): Grenze **2 s**, erwartet unter 0,1 s — gerechnet wie in R00006 (Strom → Worker → Tab ohne Takt).
- **Sicherheit**: lesen nur mit Sitzung; der Channelname ist eine Erlaubnisliste und verlässt keinen Ordner; Text und Beschreibung erscheinen nur als Text (`textContent`), die Beschreibung schreibrichtungs-isoliert; `n` ist auf 200 begrenzt (eine Antwort höchstens ≈ 130 KB bei den Testdaten).
- **Benutzerfreundlichkeit**: der Verlauf ist mit der Tastatur erreichbar und scrollbar; Fehler nie nur über Farbe (⚠ und Wort); Nachladen ohne Ansage; bei 320 px kein Querscrollen.
- **Testbarkeit**: Uhrzeiten und Tagestrenner über die gestellte Uhr (`LOCALCHATA_JETZT`, `TZ=Europe/Berlin`) und eine abweichende Zeitzone des Browsers (Playwright `timezoneId`); Reihenfolge und Vollständigkeit exakt über die Zeiten; Zeitgrenzen nur als obere Grenze zwischen zwei Augenblicken beim selben Beobachter (Projektregel #4).

## Akzeptanzkriterien

Gemeinsam für alle Kriterien: Start mit `TZ=Europe/Berlin LOCALCHATA_JETZT=2026-10-28T15:00:00.000+01:00 localchatA <ordner> --port 0 --host 127.0.0.1 --kein-browser` auf einem Wegwerf-Ordner, `<p>` ist der Port aus der Startzeile; die Frist umfasst den 22. bis 28. Oktober 2026 (Zeitumstellung am 25. Oktober um 03:00, davor `+02:00`). Nachrichten entstehen vor dem Start als Tagesdateien `channels/<name>/<datum>.log` im Logformat (`@ <zeit> <kennung>`, Text mit zwei Leerzeichen eingerückt, Leerzeile) oder mit dem Testdaten-Erzeuger; `gross` = 100.000 Nachrichten über 7 Tage. `S(<name>)` ist der Header `X-LocalchatA-Sitzung` mit dem Token aus dem Beitritt von `<name>`; Beitritte per `curl` sind Agenten. Browser A und B sind getrennte Browser-Kontexte. Nummern von Nachrichten zählen vom ältesten (1) zum neuesten (100.000).

### F0027 Verlauf über die API

- [ ] Block über Tage (Unit, Verlaufsindex rein): drei Tage zu je 250 Sätzen; `Neueste, 100` → die letzten 100 des jüngsten Tages, älteste zuerst, `aelter: true`, `neuer: false`; `Vor(Zeit des 50. Satzes des jüngsten Tages), 100` → 51 vom mittleren und 49 vom jüngsten Tag, lückenlos; `Nach(Zeit des letzten Satzes des ältesten Tages), 100` → die ersten 100 des mittleren Tages; `Nach(vor dem ältesten Tag), 7` → die ersten 7 des Channels, `aelter: false`.
- [ ] Exklusive Grenze (Unit): Sätze um 10:00:00.000 und 10:00:02.000; `Vor(10:00:02.000)` → nur der erste; `Nach(10:00:01.000)` (kein Satz mit dieser Zeit) → nur der zweite; `Vor(10:00:00.000)` → leer, `aelter: false`, `neuer: true`.
- [ ] Zeitpunkt statt Zeichenkette (Unit): Satz `2026-10-25T02:45:00.000+02:00` (= 00:45 UTC) und Cursor `vor=2026-10-25T02:30:00.000+01:00` (= 01:30 UTC) → der Satz gehört zum Block.
- [ ] Antwort (Integration, echtes Programm): `channels/allgemein/2026-10-28.log` mit drei Sätzen von `alpha@127.0.0.2` um 14:00, 14:01, 14:02 (Text „eins“, zweizeilig „zwei/drei“, „vier“) → `GET /api/channels/allgemein/nachrichten` mit `S(beta)` → 200 `{"channel":"allgemein","nachrichten":[{zeit:"2026-10-28T14:00:00.000+01:00", von:"alpha@127.0.0.2", text:"eins"}, {…, text:"zwei\ndrei"}, {…, text:"vier"}], "aelter":false, "neuer":false}`; leerer Channel → `nachrichten: []`, beide `false`.
- [ ] 100.000 lückenlos (Integration, `gross`): ohne Cursor → 100 Nachrichten, die letzte ist die jüngste des Channels, `aelter: true`; 1.000 Abrufe rückwärts mit `vor=<erste Zeit des vorigen Blocks>` ergeben zusammen genau 100.000 Zeiten, streng steigend, ohne Doppel; der letzte Abruf meldet `aelter: false`.
- [ ] Wertprüfung als Spanne: `n=1` → 1 Nachricht, `n=200` → 200; `n=0`, `n=201`, `n=abc`, `n=1.5`, `n=-1` → 422 `regel:"n"`; die Mutanten „untere Grenze 0“, „untere Grenze 2“, „obere Grenze 199“, „obere Grenze 201“ werden je rot (der Entwickler nennt die vier Ergebnisse).
- [ ] Cursor: `vor=2026-10-28T14:00:00.000%2B01:00` → 200; dieselbe Zeit mit unkodiertem `+` → 422 `regel:"cursor"`; `vor` und `nach` zugleich → 422 `regel:"cursor"`; `vor=gestern` → 422 `regel:"cursor"`.
- [ ] Unbekannt und ohne Sitzung: `GET /api/channels/fehlt/nachrichten` → 404 `{fehler}`; `…/Allgemein/nachrichten` → 404; ohne Header → 401.
- [ ] **L1** (Integration, `gross`, Debug-Build): je 20 Abrufe der neuesten 100 und von `vor=<Zeit der 50.000. Nachricht>`; der größte Wert je Art (gesendet → vollständig gelesen) liegt **unter der Grenze L1**.
- [ ] Erzeuger: `cargo run --release --example verlauf-erzeugen -- <ordner> gross 100000 7` mit `TZ=Europe/Berlin LOCALCHATA_JETZT=2026-10-28T15:00:00.000+01:00` → sieben Dateien `2026-10-22.log` … `2026-10-28.log`; ein Start auf dem Ordner meldet in `/api/gesundheit` `"nachrichten":100000` und `"defekte_zeilen":0`; Sätze am 24. Oktober tragen `+02:00`, am 26. Oktober `+01:00`.

### F0028 Channel wählen und Kopfzeile

- [ ] Ableitung (Node-Test, rein): `auswahl('#/c/allgemein', [allgemein, test])` → `{name:'allgemein', bekannt:true}`; `'#/c/neu'` → `{name:'neu', bekannt:false}`; `'#/c/Neu'`, `'#/x'`, `''` → keine; Start ohne Hash mit Merker `test` → Adresse `#/c/test` per `replaceState`; Start ohne Hash ohne Merker → keine.
- [ ] Wählen (E2E): `allgemein` und `test` existieren; A ist beigetreten und klickt `# test` → Adresse endet auf `#/c/test`, der Eintrag `# test` trägt `aria-current="page"` und `# allgemein` nicht, die Kopfzeile zeigt `# test`, `document.title` ist „# test — localchatA“.
- [ ] Ohne Auswahl (E2E): A tritt auf `http://127.0.0.1:<p>/` bei → die Mitte zeigt „Links einen Channel wählen.“, kein Eintrag trägt `aria-current`, `document.title` ist „localchatA“.
- [ ] Neuladen und Merker (E2E): A wählt `test`, lädt neu → `# test` gewählt; A navigiert im selben Tab auf `http://127.0.0.1:<p>/` (ohne Adresse) → die Adresse wird `#/c/test`, `# test` ist gewählt.
- [ ] Zurück (E2E): A wählt `allgemein`, dann `test`, dann Zurück → `# allgemein` gewählt, Kopfzeile `# allgemein`.
- [ ] Zwei Tabs (E2E): Tab 1 wählt `allgemein`, Tab 2 (neuer Tab im selben Kontext) öffnet `#/c/test` → Tab 1 zeigt `# allgemein`, Tab 2 `# test`; nach Neuladen beider Tabs ebenso.
- [ ] Unbekannt, dann angelegt (E2E): A öffnet `#/c/spaeter` → die Mitte zeigt „# spaeter gibt es nicht.“, die Kopfzeile keinen Channel; der Agent `alpha` legt `spaeter` über die API an → A zeigt `# spaeter` in der Kopfzeile und `aria-current` am Eintrag innerhalb der **Zustellgrenze** (Nicht-funktionale Anforderungen), ohne Neuladen.
- [ ] Schmal (E2E): bei 320 px mit gewähltem Channel, dessen Name 32 Zeichen hat → `scrollWidth <= innerWidth`; der gewählte Eintrag trägt `aria-current`.
- [ ] Konsole der beobachtenden Seite: keine unbehandelte Ausnahme und kein CSP-Verstoß; Konsolenfehler in diesen Szenarien keine — diese Slice schreibt keine Fehlerantwort vor.

### F0029 Verlauf öffnen

- [ ] Uhrzeit (Node-Test, rein): `2026-10-24T23:30:00.000+02:00` → `23:30`; `2026-10-28T09:05:59.999+01:00` → `09:05`; das Ergebnis ist mit `TZ=Asia/Tokyo` und `TZ=America/New_York` im Testprozess dasselbe.
- [ ] Anzeige (E2E): `allgemein` enthält drei Nachrichten von `alpha@127.0.0.2` (14:00, „eins“), `beta@127.0.0.3` (14:20, zweizeilig „zwei“/„drei“), `alpha@127.0.0.2` (14:40, `<b>vier</b>`) am 28. Oktober; Browser mit `timezoneId: 'Asia/Tokyo'`; A wählt `# allgemein` → drei Nachrichten in dieser Reihenfolge, je mit Kennung in `lc-id` (Mono) und Zeit `14:00`, `14:20`, `14:40`; „zwei“ und „drei“ stehen in zwei Zeilen; `<b>vier</b>` steht als Text da, ohne fettes Element; die letzte Nachricht ist ganz sichtbar.
- [ ] Leer (E2E): `leer` hat keine Tagesdatei → die Mitte zeigt „Noch keine Nachrichten in # leer.“
- [ ] **L2** (E2E, `gross`): A klickt `# gross` → die neueste Nachricht mit Absender und Zeit ist nach dem Zeitstempel des `click` **innerhalb der Grenze L2** sichtbar; genau ein Abruf von `/nachrichten` ohne Cursor.
- [ ] Wechsel mit verzögerter Antwort (E2E, Pflichttest): `gross` und `allgemein` existieren; die Route `/api/channels/gross/nachrichten` antwortet 1 s verzögert; A klickt `# gross` und sofort `# allgemein` → nachdem der Abruf für `gross` beendet oder abgebrochen ist (`requestfinished` bzw. `requestfailed`), zeigt der Verlauf genau die drei Nachrichten von `allgemein` und keine aus `gross`, die Kopfzeile `# allgemein`.
- [ ] Am Ende trotz später Schrift (E2E): die Schriftdatei antwortet 1 s verzögert; A öffnet `gross` → nachdem die Schrift geladen ist (`document.fonts.ready`), ist die neueste Nachricht weiter ganz sichtbar.
- [ ] Fehler (E2E, Ausnahmeweg, Kriterium nach Nutzen): A ist beigetreten, das Programm wird beendet, A klickt `# allgemein` → die Mitte zeigt „Verlauf nicht geladen — Verbindung prüfen.“ mit ⚠, nicht „Noch keine Nachrichten“.
- [ ] Fokus (E2E): `Tab` von der Seitenleiste erreicht den Verlauf (`role="log"`); `Bild ↑` scrollt ihn.
- [ ] Konsole bei A: keine unbehandelte Ausnahme und kein CSP-Verstoß. Konsolenfehler: in „Anzeige“, „Leer“, „L2“, „Am Ende trotz später Schrift“ und „Fokus“ keine; in „Wechsel mit verzögerter Antwort“ höchstens „Failed to load resource“ zum abgebrochenen Abruf von `/api/channels/gross/nachrichten`. Für „Fehler“ gilt das Kriterium nach Nutzen, keine weitere Vorgabe zur Konsole.
- [ ] **M2** — Prüfpunkt im Entwicklertest, kein Gate: Median aus 20 Öffnungen von `gross` ohne Drosselung innerhalb der Grenze M2; min/Median/max stehen im Bericht des Entwicklertests und in ADR-0026 unter „Ergänzungen“.

### F0030 Verlauf gegliedert

- [ ] Gruppierung als Spanne (Node-Test, rein): vorige `alpha` 14:00:00.000, Nachricht `alpha` 14:05:00.000 → ohne Kopf; `alpha` 14:05:00.001 → mit Kopf; `beta` 14:01 → mit Kopf; `alpha` 23:59 → `alpha` 00:01 am nächsten Tag → mit Kopf und Tagestrenner; ohne vorige → mit Kopf. Die Mutanten „Fenster 4 min“ und „Fenster 6 min“ werden je rot.
- [ ] Datum (Node-Test, rein, heutiges Serverdatum 28.10.2026): 28.10. → „Heute“, 27.10. → „Gestern“, 26.10. → „Montag, 26. Oktober 2026“, 24.10. → „Samstag, 24. Oktober 2026“.
- [ ] Ansicht (E2E, Browser mit `timezoneId: 'Asia/Tokyo'`): `allgemein` enthält am 24.10. `alpha` 23:30 (`+02:00`), am 27.10. `alpha` 10:00, `alpha` 10:04, `alpha` 10:09:00.001, `beta` 10:10, am 28.10. `beta` 09:00 → Tagestrenner „Samstag, 24. Oktober 2026“, „Gestern“, „Heute“ in dieser Reihenfolge; Köpfe genau bei 23:30, 10:00, 10:09, 10:10 und 09:00 (fünf Köpfe, 10:04 ohne Kopf); beim Zeigen auf die Nachricht von 10:04 erscheint `10:04` in der linken Spalte; die Zeit 23:30 steht unter dem Trenner vom 24. Oktober (Ortszeit des Servers, nicht Tokio).
- [ ] Kleben (E2E): in `gross` hochgescrollt bis über einen Tageswechsel → der Trenner des obersten sichtbaren Tages steht am oberen Rand des Verlaufs.
- [ ] Lange Nachricht (E2E): `allgemein` enthält eine Nachricht mit 30 Zeilen „Zeile 1“ … „Zeile 30“ und eine mit 3 Zeilen → die lange zeigt einen 240 px hohen Inhalt (± 1 px) und den Schalter „Ganze Nachricht anzeigen · 30 Zeilen“ mit `aria-expanded="false"`, die kurze keinen; Klick → „Zeile 30“ ist sichtbar, der Schalter heißt „Einklappen“ mit `aria-expanded="true"`, der Kopf der Nachricht steht höchstens 1 px von seiner vorigen Bildschirmlage; Klick auf „Einklappen“ → wieder 240 px.
- [ ] Konsole bei A: keine unbehandelte Ausnahme, kein CSP-Verstoß, keine Konsolenfehler — diese Slice schreibt keine Fehlerantwort vor.

### F0031 Fenster beim Scrollen

- [ ] Nachladen entscheiden (Node-Test, rein, Sichthöhe 800 px): Abstand oben 1.599 px mit `aelter` → `aelter`; 1.600 px → nichts; 1.599 px ohne `aelter` → nichts; unten entsprechend mit `neuer`; läuft ein Abruf, → nichts.
- [ ] Block ins Fenster (Node-Test, rein): fünf Blöcke + einer oben → der unterste fällt weg, `neuer: true`, fünf Blöcke; fünf + einer unten → der oberste fällt weg, `aelter: true`. Die Mutanten „4 Blöcke“ und „6 Blöcke“ werden je rot.
- [ ] **L3** (E2E, `gross`): A öffnet `gross` und scrollt nach oben, bis zehnmal nachgeladen ist → im DOM stehen genau 500 Nachrichten, die Nummern 98.901 bis 99.400 (Rechnung: geöffnet 99.901–100.000; nach 10 Blöcken ist 98.901 die älteste geladene, behalten werden 5 × 100 = 98.901–99.400); die Pille „Zum Neuesten“ ist sichtbar.
- [ ] **L4** (E2E, `gross` und `grenzen`; `grenzen` enthält am 27.10. 150 Nachrichten von `alpha@127.0.0.2` im Minutentakt ab 08:00 und am 28.10. 100 ebensolche ab 08:00 — die erste Blockgrenze liegt damit genau am Tageswechsel, die zweite zwischen der 50. und 51. Nachricht des 27.10. mitten in einer Gruppe): bei jedem Nachladen oben und unten, bei jedem Ein- und Ausblenden der Ladezeile und bei 10 Sprüngen an den oberen Rand (`scrollTop = 0`) hält die Lage der zuvor obersten sichtbaren Nachricht die **Grenze L4**.
- [ ] **L5** (E2E, `gross`): hoch über 1.000 Nachrichten, dann nach unten bis `neuer: false` → jede gesehene Zeit genau einmal, streng steigend; die neueste ist die jüngste des Channels.
- [ ] Ladezeile (E2E): die Route `vor=` antwortet 1 s verzögert → während des Wartens steht oben „Ältere Nachrichten werden geladen …“, danach nicht mehr; in der Zeit läuft kein zweiter Abruf.
- [ ] Zum Neuesten (E2E): in `gross` 1 Bildschirmhöhe minus 10 px über dem Ende → keine Pille; 1 Bildschirmhöhe plus 10 px → Pille „Zum Neuesten“; Klick → die neueste Nachricht ist ganz sichtbar, der Fokus liegt im Verlauf; dasselbe mit `Ende` bei fokussiertem Verlauf.
- [ ] Konsole bei A: keine unbehandelte Ausnahme, kein CSP-Verstoß, keine Konsolenfehler — diese Slice schreibt keine Fehlerantwort vor.
- [ ] **M1** — Prüfpunkt im Entwicklertest, kein Gate: Mausrad über 2.000 Nachrichten in `gross` nach oben → innerhalb der Grenze M1; Werte im Bericht des Entwicklertests und in ADR-0026 unter „Ergänzungen“.

### F0032 Channel per Tastatur wechseln

- [ ] Nachbar (Node-Test, rein): Liste `[a, b, c]`; `b` ↓ → `c`; `c` ↓ → nichts; `a` ↑ → nichts; keine Auswahl ↓ → `a`, ↑ → `c`; leere Liste → nichts.
- [ ] Kürzel (Node-Test, rein): `{altKey, key:'ArrowDown'}` → wechseln; dasselbe mit `ctrlKey`, `shiftKey` oder `getModifierState('AltGraph')` → nichts.
- [ ] Wechseln (E2E): `a`, `b`, `c` existieren, A hat `a` gewählt, der Fokus liegt im Verlauf → `Alt ↓` → Adresse `#/c/b`, Kopfzeile `# b`, der Verlauf zeigt `b`, der Fokus liegt weiter im Verlauf; `Alt ↓` → `c`; `Alt ↓` → weiter `c`; Zurück → `b`.
- [ ] Dialog offen (E2E): A hat den Dialog „Channel anlegen“ geöffnet → `Alt ↓` ändert die Auswahl nicht.
- [ ] Konsole bei A: keine unbehandelte Ausnahme, kein CSP-Verstoß, keine Konsolenfehler.

### F0026 Beschreibung

- [ ] Regel als Spanne (Unit): `""`, `null`, fehlend → keine; 200 × `ä` → gültig, 201 × `ä` → Verstoß; 200 × `😀` (200 Zeichen, 800 Bytes) → gültig; `a\nb`, `a\tb`, `a\u0007` → Verstoß; `"‮abc"` → gültig; `5` → Verstoß. Die Mutanten „höchstens 199“ und „höchstens 201“ werden je rot.
- [ ] Anlegen (Integration): `alpha` sendet `{"name":"security-entwicklung","beschreibung":"Absprachen zwischen Security- und Entwicklungs-Factory"}` → 201 mit dieser `beschreibung`; `einstellungen.toml` enthält sie; `GET /api/channels`, der Stand eines neu geöffneten Stroms und das Ereignis `channel` eines laufenden Stroms tragen sie; nach einem Neustart ebenso.
- [ ] Abweisen (Integration): 201 × `ä` → 422 `regel:"beschreibung"`, kein Ordner; `"a\nb"` → 422; `5` → 422; `null` und `""` → 201 mit `beschreibung: null`.
- [ ] R00006 zieht mit, ohne Schwächung: der Test `us6_…` in `tests/channels.rs` prüft danach `{"name":"x1","beschreibung":"Absprachen"}` → 201 mit `beschreibung:"Absprachen"`, einen Verstoß → 422 `regel:"beschreibung"` ohne Ordner, `null` und `""` → 201 mit `null`; keine bisherige Prüfung entfällt ohne Ersatz.
- [ ] Defekt beim Laden (Integration): `channels/kaputt/einstellungen.toml` mit `angelegt_von = "alpha@127.0.0.2"` und `beschreibung = "a\nb"` → `kaputt` mit `angelegt`, `angelegt_von` und `beschreibung` `null`; stderr nennt die Datei mit Zeile; `defekte_zeilen` 1; die Datei ist byte-gleich wie vorher.
- [ ] Dialog (E2E): A öffnet „Channel anlegen“, gibt `doku` und als Beschreibung `Notizen zur Doku` ein, `Enter` → 201, Statuszeile wie in R00006; A gibt `test2` und als Beschreibung `a<Tab>b` (eingefügt) ein → Fehlerzeile mit ⚠ in `kd-danger`, Dialog offen, beide Eingaben stehen noch.
- [ ] Kopfzeile (E2E, 1280 × 800): A wählt `security-entwicklung` → nach `# security-entwicklung` steht „Absprachen zwischen Security- und Entwicklungs-Factory“ in einem `<bdi>`; ein Channel mit 200 Zeichen Beschreibung → die Beschreibung ist einzeilig (Höhe einer Zeile), endet sichtbar mit Ellipse (`scrollWidth > clientWidth`, `text-overflow: ellipsis`), ihr `title` ist der ganze Text; ein Channel ohne Beschreibung → kein `lc-head-desc`.
- [ ] Schmal (E2E): bei 719 px Breite ist die Beschreibung nicht sichtbar, bei 720 px sichtbar; bei 320 px kein Querscrollen.
- [ ] Konsole bei A: keine unbehandelte Ausnahme und kein CSP-Verstoß; Konsolenfehler in „Kopfzeile“ und „Schmal“ keine, in „Dialog“ höchstens „Failed to load resource“ zu `POST /api/channels` mit 422.

## Betroffene Verzeichnisstruktur

- Verlauf: `src/gespraeche/verlaufsindex.rs` (`block`, Grenze im Tag), `src/gespraeche/logformat.rs` (Satz als Nachricht), `src/gespraeche/mod.rs` (Abfrage im Verzeichnis).
- HTTP: neu `src/http/nachrichten.rs` (`GET`, Abfrage lesen), `src/http/mod.rs` (Route), `src/http/channels.rs` (Beschreibung statt Abweisung).
- Einstellungen: `src/gespraeche/einstellungen.rs` (`beschreibung`, Regel, Defekt).
- Werkzeug: neu `examples/verlauf-erzeugen.rs`; `tests/leistung.rs` darf ihn mitnutzen.
- Oberfläche: neu `web/verlauf.js` (Öffnen, Fenster, Anker, Zum Neuesten), `web/auswahl.js` (Ableitung, Merker, Nachbar-Channel), `web/gliederung.js` (Gruppierung, Datum, Uhrzeit); geändert `web/seitenleiste.js` (Links, `aria-current`), `web/kopfzeile.js` (Channel, Beschreibung), `web/hauptansicht.js` (Mitte, Kürzel), `web/channel-anlegen.js` (Feld Beschreibung), `web/api.js` (Verlauf abrufen), `web/lc-seite.css`.
- Tests: Unit im jeweiligen Modul; Integration neu `tests/nachrichten.rs`, `tests/nachrichten_leistung.rs`, geändert `tests/channels.rs`, `tests/channels_ordner.rs`; Oberfläche rein `e2e/einheit/*.test.mjs`; Abläufe `e2e/tests/r00007-*.spec.js`.

## Technische Überlegungen

### Ablauf

1. **API**: `GET /api/channels/<name>/nachrichten?…` → Grenzschutz → Teilnehmer → Abfrage lesen (422) → `spawn_blocking`:
   - 1.1 Verzeichnis lesend: Name → Gespräch (404);
   - 1.2 Gesprächssperre: Block über Tage → Sätze kopieren;
   - 1.3 ohne Sperre: Sätze → Nachrichten → JSON.
2. **Wählen**: Start und `hashchange` und jedes Channel-Ereignis → `auswahl(hash, liste)` → Seitenleiste, Kopfzeile, Mitte; bekannte Auswahl → 3.
3. **Öffnen**: laufenden Abruf abbrechen → neueste 100 → Gliederung → zeichnen → ans Ende; `ResizeObserver`, solange am Ende.
4. **Scrollen**: `scroll` → Nachladen entscheiden → höchstens ein Abruf → Anker merken → Block einfügen, am anderen Ende abwerfen, Köpfe und Trenner an der Grenze neu → `scrollTop` korrigieren → erneut entscheiden.
5. **Beschreibung**: Anlegen prüft sie wie den Namen; Einstellungen schreiben sie; Liste, Stand und Ereignis tragen sie; die Kopfzeile zeigt sie.

### Grobentwurf (Vorplanung)

Die Bubbles aus der WBS als Datenfluss — **Vorplanung**: welche es am Ende werden, entscheidet der Entwickler beim Bauen.

- F0027: B0141 Grenze im Tag finden (rein) · B0142 Block über Tage sammeln (rein) · B0143 Satz als Nachricht (rein) · B0144 Abfrage lesen (rein) · B0145 Verlauf im Verzeichnis abfragen · B0146 Verlauf beantworten · B0147 Testdaten-Erzeuger
- F0028: B0148 Auswahl ableiten (rein) · B0149 Auswahl merken und wiederherstellen · B0150 Seitenleiste mit Links · B0151 Kopfzeile mit Channel · B0152 Auswahl in der Hauptansicht (unklar)
- F0029: B0153 Verlauf abrufen · B0154 Uhrzeit aus der Nachrichtenzeit (rein) · B0155 Nachricht zeichnen · B0156 Channel öffnen (unklar) · B0157 Öffnungszeit messen
- F0030: B0158 Gliederung bestimmen (rein) · B0159 Datum benennen (rein) · B0160 Gegliedert zeichnen (unklar) · B0161 Lange Nachricht einklappen (unklar)
- F0031: B0162 Nachladen entscheiden (rein) · B0163 Block ins Fenster (rein) · B0164 Anker halten (unklar) · B0165 Nachladen beim Scrollen (unklar) · B0166 Zum Neuesten · B0167 Frames beim Scrollen messen
- F0032: B0168 Nachbar-Channel (rein) · B0169 Kürzel Alt ↑ und Alt ↓
- F0026: B0170 Beschreibung prüfen (rein) · B0171 Beschreibung in den Einstellungen (rein) · B0172 Anlegen mit Beschreibung · B0173 Feld Beschreibung im Dialog · B0174 Beschreibung in der Kopfzeile

**Elemente** (nur öffentliche Signaturen, nach A00009, A00006, A00004, A00013, A00005):

- `Abfrage { richtung: Neueste | Vor(Zeit) | Nach(Zeit), n: Blockgroesse }` (Wert); `Blockgroesse` 1–200; `Abfragefehler { regel: Cursor | N }`; `fn abfrage_lesen(query: &str) -> Result<Abfrage, Abfragefehler>`.
- `Verlauf` (A00009) — zusätzlich `fn block(&self, abfrage: &Abfrage) -> Block`; `Block { saetze, aelter: bool, neuer: bool }`.
- `Gespraeche` — zusätzlich `fn verlauf(&self, name: &str, abfrage: &Abfrage) -> Result<Nachrichtenblock, Unbekannt>`; `Nachricht { zeit, von, text }` (DTO, vorhanden in `src/gespraeche/mod.rs`, wird ausgegeben).
- `Beschreibung` (Wert, 1–200 Zeichen, ohne Cc) — `fn bilden(wert: Option<&Value>) -> Result<Option<Beschreibung>, Regelfehler>`; `Einstellungen` um `beschreibung` erweitert; `Anlegefehler::Beschreibung` bedeutet künftig „Regel verletzt“.
- Oberfläche (rein): `auswahl(hash, liste)`, `startadresse(hash, merker)`, `nachbarChannel(liste, auswahl, richtung)`, `uhrzeit(zeit)`, `datumBenennen(datum, heute)`, `gliederung(vorige, nachricht)`, `nachladen({bloecke, aelter, neuer, abstandOben, abstandUnten, sicht, laeuft})`, `einfuegen(fenster, block, richtung)`.

Wireframe: die WBS führt `wireframes: Dokumentation/Wireframes/`, ein Artboard `D0003.dc.html` fehlt (Hinweis `/wireframe verfeinern D0003`). Gestaltungsquelle bis dahin: Designsystem `components/Nachricht`, `VerlaufsMarken`, `LangeNachricht`, `Kopfzeile`, `Seitenleiste`, `Hauptansicht`, `1-tastatur.md` — Vorgabe für die Gestaltung, nicht Akzeptanzkriterium.

### Änderungen an bestehenden Klassen

- `gespraeche::verlaufsindex::Verlauf`: `block` (geplant seit B0023).
- `gespraeche::Gespraeche`: Abfrage des Verlaufs je Channel; `channel_anlegen` nimmt die Beschreibung.
- `gespraeche::einstellungen`: `beschreibung` lesen, prüfen, schreiben; bisher unbekannter Schlüssel.
- `http::channels`: Beschreibung prüfen statt abweisen; Antwort mit `beschreibung`.
- `web/seitenleiste.js` (Einträge werden Links), `web/kopfzeile.js` (Zone „Ort“), `web/hauptansicht.js` (Mitte, `Alt ↑`/`Alt ↓`), `web/channel-anlegen.js` (Feld), `web/api.js`.

## Tests

Nach Skill `test-pyramide` und ADR-0009; die Szenarien der User Story werden Tests. Tests sind nie schärfer als diese Anforderung: sie übernehmen die Grenzen L1–L5 wörtlich; M1 und M2 sind Messwerte, keine Tests im Gate.

**Unit (rein, gestellte Uhr):** Grenze im Tag und Block über Tage (drei Tage zu 250, exklusive Grenzen, Zeitumstellung), Abfrage lesen (Spanne 1/200, Cursor), Beschreibung (Spanne 200/201, Steuerzeichen), Einstellungen mit Beschreibung.

**Integration (`tests/`, echtes Programm):** Antwortform, 100.000 lückenlos, 404/401/422, L1; Beschreibung über Anlegen, Liste, Strom und Neustart; Defekt beim Laden; angepasster R00006-Test `us6_`. Mutationsproben: Grenzen von `n` (0/2, 199/201), Beschreibung (199/201).

**Oberfläche rein (`e2e/einheit/`):** Auswahl und Merker, Nachbar-Channel und Kürzel, Uhrzeit (mehrere `TZ`), Gruppierung (Spanne, Mutanten 4/6 min), Datum, Nachladen entscheiden (Spanne), Block ins Fenster (Mutanten 4/6 Blöcke).

**E2E (Playwright):** die Szenarien aus F0028–F0032 und F0026; Dauern ereignisgenau (`waitForFunction` mit `polling: 'raf'`), nie im Standardraster von `expect`. Pflicht: Wechsel mit verzögert antwortender Route (F0029) und Anker über eine Gruppen- und eine Tagesgrenze (F0031). Die Testdaten `gross` entstehen einmal je Lauf mit dem Testdaten-Erzeuger.

**Entwicklertest, Messwerte (kein Gate):** M1 und M2 mit B0157 und B0167.

## Abhängigkeiten

- Abhängig von: I0007 Channel anlegen (R00006, grün) — Seitenleiste, Channel-Liste im Strom, Dialog, Einstellungsdatei; I0002 Nachricht landet im Log (R00002, grün) — Verlaufsindex, Logformat; I0005/I0006 (R00004, R00005, grün) — Strom, Worker, Serverzeit (`jetzt`, Versatz).
- Innerhalb (WBS `Braucht`): F0027 und F0028 Welle 1 (parallel); F0029 braucht F0027 und F0028, F0032 und F0026 brauchen F0028 — Welle 2 (parallel); F0030 braucht F0029 (Welle 3); F0031 braucht F0030 (Welle 4).
- Blockiert: I0009 Nachricht senden (Thema `nachricht` nach ADR-0028 setzt auf Verlauf und Fenster auf), I0011 Nachrichten durchsuchen (Sprung an eine Zeit), I0017 Markdown (M1 wird dort neu gemessen), I0010 Anhänge (Höhen aus Metadaten).

## Offene Fragen

Keine, die R00007 blockieren.

Entschieden vom planer für R00007 (Rückfragen mit Empfehlung beantwortet):

- **Gruppierung zur vorigen Nachricht**, nicht zum Kopf der Gruppe: so liest sich das Designsystem („folgt … innerhalb von 5 Minuten“), und beim Nachladen ändert sich nur der Kopf der Grenznachricht (ADR-0026 Regel 3). Folge: ein Agent, der im Minutentakt schreibt, bekommt lange keinen neuen Kopf — die Zeit steht dann bei Hover in der linken Spalte.
- **Lange Nachrichten für alle bei 240 px**, bis I0009 die Art des Absenders liefert; „eigene nie, Menschen ab doppelter Höhe“ (Designsystem) kommt dann.
- **Zusammenfassung der Leiste nur mit der Zeilenzahl**; Listeneinträge und Codeblöcke zählt erst das Markdown aus I0017.
- **Fehler beim Öffnen**: eine Zeile in der Mitte statt eines leeren Channels; Wiederholen über eine neue Wahl — die Verbindungsleiste mit Countdown gehört zu I0009 (Senden puffern).
- **Testdaten-Erzeuger als `cargo`-Beispiel**: Gate, Entwicklertest und Abnahme brauchen 100.000 Nachrichten, und bis I0009 gibt es keine Senderoute.
- **Beschreibung nur in der Kopfzeile**: ADR-0024 Regel 8 (Beschreibung als `title` des Seitenleisten-Eintrags) widerspricht unter 720 px Regel 3 (dort ist der `title` der Name); der `title` des Eintrags bleibt wie in R00006.
- **Uhrzeit-`title` ohne relative Angabe** („vor 2 min“ aus dem Designsystem): sie veraltet ohne Neuzeichnen; absolute Zeiten ändern sich nie (Designsystem „Billig je Nachricht“).

## Manuelle Vorbereitungstätigkeiten

- Für die Messwerte M1 und M2: ein ruhiger Rechner (`uptime` unter 2), Chromium aus `e2e/node_modules`, Fenster 1280 × 800.

## Manuelle Nachbereitungstätigkeiten

- ADR-0026: M1 und M2 unter „Ergänzungen“ eintragen — architekt über den Director.
- ADR-0024 Regel 8: auf ADR-0027 Regel 5 verweisen (Beschreibung nur in der Kopfzeile) — architekt über den Director.
- Designsystem `3-offene-fragen.md` Frage 4: Antwort „5 Minuten zur vorigen Nachricht desselben Absenders“ — über den Director.
- R00006 US-6 und das Kriterium „Beschreibung noch nicht“ sind durch F0026 überholt; R00006 bleibt unverändert, R00007 nennt den Ersatz.

## Umfang

Aus der WBS gezählt (`/planung zaehlen I0008`); Standard-Bubbles mit der Untergrenze 0,4h (belegt aus `Schaetzungen/_ist-zeiten.md`, R00001–R00006, Typ Standard), unklare als Richtwert 2–4h:

- F0027 Verlauf über die API = 7 Bubbles: 7 Standard (0,4h). Rest: 2,8h klar · 7 von 7 Werten belegt.
- F0028 Channel wählen und Kopfzeile = 5 Bubbles: 4 Standard (0,4h), 1 unklar (2–4h). Rest: 1,6h klar + 2–4h unklar · 4 von 5 Werten belegt, Rest Richtwert (ungemessen).
- F0029 Verlauf öffnen = 5 Bubbles: 4 Standard (0,4h), 1 unklar (2–4h). Rest: 1,6h klar + 2–4h unklar · 4 von 5 Werten belegt, Rest Richtwert (ungemessen).
- F0030 Verlauf gegliedert = 4 Bubbles: 2 Standard (0,4h), 2 unklar (2–4h). Rest: 0,8h klar + 4–8h unklar · 2 von 4 Werten belegt, Rest Richtwerte (ungemessen).
- F0031 Fenster beim Scrollen = 6 Bubbles: 4 Standard (0,4h), 2 unklar (2–4h). Rest: 1,6h klar + 4–8h unklar · 4 von 6 Werten belegt, Rest Richtwerte (ungemessen).
- F0032 Channel per Tastatur wechseln = 2 Bubbles: 2 Standard (0,4h). Rest: 0,8h klar · 2 von 2 Werten belegt.
- F0026 Beschreibung = 5 Bubbles: 5 Standard (0,4h). Rest: 2,0h klar · 5 von 5 Werten belegt.
- I0008 gesamt = 34 Bubbles. Rest: 11,2h klar + 12–24h unklar · 28 von 34 Werten belegt, Rest Richtwerte (ungemessen).
- Nicht enthalten: Senden und Nachrichten live (I0009), Ungelesen-Linie, `Pos1`, `Strg G`, `Strg P`, Markdown (I0017).

## Warum löst diese Anforderung das Problem? (Pflicht)

Auslöser: Die Security-Factory und die Entwicklungs-Factory haben seit R00006 einen gemeinsamen Channel, aber niemand sieht darin etwas — der Mensch liest mit `less` im Ordner, ein Agent hat keinen Weg nachzulesen, was seit seiner letzten Nachricht geschah. Wenn die API den Verlauf in Blöcken zu höchstens 200 mit der Nachrichtenzeit als Cursor liefert, dann kostet jeder Abruf gleich wenig, egal ob der Channel 100 oder 100.000 Nachrichten hat, und ein Agent liest mit `nach=<zeit>` genau das Fehlende nach. Wenn der Browser beim Öffnen nur den neuesten Block holt und beim Scrollen höchstens fünf Blöcke hält, dann öffnet der Channel in einem Bruchteil der halben Sekunde, und Speicher und Arbeit je Bild bleiben begrenzt, auch nach Stunden Lesen — „sofort“ und „leichtgängig“ aus der Vision werden zu Messgrößen, die das Gate hält. Wenn die Adresse den Channel trägt, dann kann ein Agent einem Menschen einen Link schicken, und Neuladen, Zurück und zwei Tabs verhalten sich wie gewohnt. Gruppierung, Tagestrenner und eingeklappte lange Befunde machen aus dem Strom von Agenten-Nachrichten einen Verlauf, den ein Mensch überfliegen kann.

## Missing-Docs

- Glossar fehlt (`Dokumentation/Domaenenwissen/` existiert nicht); Begriffe wie Verlauf, Block, Cursor, Fenster, Gruppe, Tagestrenner, Auswahl sind aus ADR-0025 bis ADR-0027 und dem Designsystem übernommen.
- Kein Artboard für D0003 (Hauptansicht mit Verlauf).
- Schwelle 0,1 s für „unmittelbar“ (M2) stammt laut ADR-0026 aus Nielsen, *Usability Engineering* (1993), Kap. 5 — dort aus dem Gedächtnis zitiert, nicht geöffnet.

## Notizen

### Verworfene Alternativen

- **Je Slice eine eigene Anforderung**: verworfen — alle sieben haben ein Ziel (ein Channel wird gewählt und sein Verlauf gelesen) und durchlaufen als Arbeitspaket I0008 gemeinsam Entwicklertest und Abnahme.
- **F0026 als eigene Anforderung**: verworfen — der Director hat die Beschreibung in I0008 geholt; ihr sichtbarer Ort (Kopfzeile) entsteht hier, und als eigene Gruppe von Kriterien bleibt sie abtrennbar.
- **„Verlauf zeigen“ als ein Slice** (Zuschnitt des architekt, Punkt 3): geteilt in F0029 Verlauf öffnen und F0030 Verlauf gegliedert — zusammen neun Bubbles, und „öffnet sofort“ (L2, M2) ist ohne Gliederung von außen prüfbar.
- **L2 und M2 am Fenster** (Zuschnitt des architekt, Punkt 4): an F0029 gehängt — das Öffnen entsteht dort; das Fenster trägt „scrollt flüssig“ (L3–L5, M1).
- **Eigener Tastatur-Slice mit „Verlauf fokussierbar“**: Fokus und `Bild ↑` gehören zum Verlauf (F0029), `Ende` zur Pille (F0031); F0032 behält nur den Channel-Wechsel.
- **Gruppierung ab dem Kopf der Gruppe** (alle 5 Minuten ein neuer Kopf): verworfen — beim Nachladen und Abwerfen änderten sich Köpfe tief im Fenster statt nur an der Blockgrenze.

### Bewusst außerhalb

Senden, Nachrichten live und die Pille „n neue Nachrichten“ (I0009, ADR-0028); Ungelesen-Linie und Öffnen an ihr; `Pos1`, `Strg G`, `Strg P`; `↑`/`↓` zum Fokussieren einzelner Nachrichten, `R`, `C`, Leertaste; Markdown und Syntaxfarben (I0017); Hinweis „wie Agenten hier schreiben“ im leeren Channel (I0009/I0019); Permalink auf eine Nachricht; Zähler für Ungelesen und Erwähnt; Ausblenden von Channels.
