# ADR-0054: Warten — Long-Poll `GET /api/warten` über alle sichtbaren Gespräche mit **einem** Zeiger `seit`, erst abonnieren, dann bis zur Hochwassermarke nachholen, dann den Bus filtern; Filter als Vereinigung; geweckt wird nur von Nachrichten (Channel, Direkt, Erwähnung, Antwort); Ende mit Grund

**Status**: Akzeptiert — drei Bestätigungen beim `product-owner` offen (W1–W3 unter „Offene Fragen“), keine davon ist eine Richtungsfrage der Vision; kippt eine, kommt eine Ergänzung
**Datum**: 2026-10-30
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #17 I0016)
**Kontext-Anforderung**: — (WBS I0016, noch ohne Anforderung; prägt F0158/B0712 aus R00047, B0698 P10 aus R00045, I0013, I0019)
**Erprobt durch**: — (gerechnet und aus dem Bestand abgeleitet, nicht gemessen; überträgt Gebautes: Schreibsperre über alle Gespräche mit streng steigender Zeit und Veröffentlichen unter der Sperre `src/gespraeche/mod.rs` `senden_mit`, `letzte_zeit` (ADR-0035); Ereignisbus mit `Lagged` → `NeuerStand` `src/live/bus.rs`; Halter der Anwesenheit `src/anwesenheit/bestand.rs` `verbunden`; Ende-Signal `src/start/ende.rs` `abwarten`, schon vorbereitet für `/warten` in `src/start/ausliefern.rs:323`; Werkbank und Byte-Budget (ADR-0039); Abrissgrenze und Keepalive je Verbindung `src/start/abrisspflege.rs` (ADR-0017). Verlustfreiheit und Ende-Zeit werden mit den Abnahme-Proben W5 und W7 gemessen.)

## Kontext

- **Auftrag** (WBS I0016, Karte #17): „Agent wartet blockierend auf Neues für sich (Channel, Direktnachricht, Erwähnung) und erhält es ohne Polling-Schleife“; Vision „Agenten können warten und reagieren — ein Agent holt neue Nachrichten ab oder wartet auf sie, verarbeitet sie und antwortet“, „Mensch und KI sind gleichberechtigt“, „Direktnachrichten und Erwähnungen … damit ein Agent auf das reagieren kann, was ihn betrifft“.
- **Was schon festliegt**:
  - [ADR-0007](0007-live-kanal-sse-fuer-browser-long-poll-fuer-agenten.md) Option A: Long-Poll `/api/warten?seit=&bis=` mit Filtern `erwaehnung`, `direkt`, `channel`; erst aus dem Index nachholen, dann auf den Bus warten; `bis` höchstens 3600 s; Empfehlung „alle aufgelaufenen, höchstens 100“. Ergänzung Karte #2: `warten` hört das Ende-Signal und antwortet leer; jeder Warte-Aufruf belegt einen Platz seiner Gegenstelle (160 je Gegenstelle seit R00015, 512 gesamt).
  - [ADR-0035](0035-nachrichtenzeit-eine-schreibsperre-ueber-alle-gespraeche.md): Zeit vergeben, anhängen, indexieren und veröffentlichen geschehen unter **einer** Schreibsperre über alle Gespräche — die Busfolge ist über alle Gespräche die Zeitfolge; „`warten` nimmt **einen** Zeiger `seit`“; Pre-Mortem 1 verlangt den Test „Speicherablage, die für Channel A beim Anhängen wartet“.
  - [ADR-0012](0012-identitaet-und-sitzung.md)/[ADR-0016](0016-anwesenheitsstatus-aus-haltern-mit-nachlauf-liste-als-stand-plus-wechsel.md): ein wartender Aufruf ist ein **Halter** der Sitzung (online, solange er läuft, 1 s Nachlauf); mehrere Halter je Sitzung sind erlaubt; nach einem Neustart des Programms ist das Token unbekannt (Wiederkehr über `beitreten`).
  - [ADR-0025](0025-verlauf-ueber-die-api-bloecke-mit-zeit-als-cursor.md): die Nachrichtenzeit ist ID und exklusiver Cursor, verglichen als Zeitpunkt, `+` muss kodiert werden (422 `cursor`); jedes Feld einer Nachricht ist immer da.
  - [ADR-0039](0039-grosse-antworten-byte-budget-je-block-werkbank-und-abrissgrenze-als-sendefrist.md): Listen schneiden nach Byte-Budget (256 KiB) mit Cursor und Fortsetzung, mindestens ein Element; schwere Arbeit auf der Werkbank; Regel 6: ein Warte-Aufruf schreibt während des Wartens nichts, die Abrissgrenze gilt trotzdem.
  - [ADR-0017](0017-stiller-abriss-kernel-erkennt-tote-verbindungen.md): Keepalive je Verbindung (Leerlauf 5 s, Abstand 5 s, 2 Proben) — ein still abgerissener Warte-Aufruf (ohne laufende Daten) fällt nach ≈ 15 s.
  - **Nicht wecken**: Reaktionen (PO Zeile 88), Aktivitätssignale (PO Zeile 140; [ADR-0051](0051-aktivitaet-tippt-und-denkt-nach-fluechtig-im-speicher-eigener-signalbus-eine-route-je-gespraech.md) Regel 12: `warten` abonniert den Signalbus nie), Änderungen (Empfehlung [ADR-0048](0048-eigene-nachrichten-bearbeiten-und-loeschen-aenderungssatz-im-tageslog-der-aenderung-fassung-je-nachricht-im-index.md) Regel 11, „dort zu entscheiden“).
  - **Wecken**: Antwort auf eine eigene Nachricht wie eine Erwähnung, über die Kennung (PO Zeile 150; [ADR-0053](0053-antworten-mit-zitat-bezug-als-metazeile-zeit-und-kennung-aufgeloest-beim-lesen-feld-antwort-auf-zitat-im-thema-aenderung.md) Regel 9, F7) — beim Nachholen aus der Metazeile, ohne Nachschlagen.
- **Bestand im Code**: kein `warten`; Bus (`Ereignis::Nachricht{channel, nachricht}`, `Aenderung`, `Anhaenge`, `Anwesenheit`, `Channel`; Kapazität 1.024); Direktgespräche veröffentlichen noch nichts (`src/live/bus.rs` `nachrichten_veroeffentlichen`: „eigener Weg (I0012)“); keine Erwähnungsregel (I0013 rot). Der Index liegt je Gespräch (`src/gespraeche/verlaufsindex.rs`), Blöcke `nach=<zeit>` je Gespräch.
- **Vorgänger in der WBS**: I0016 braucht I0015 (→ I0012 Direktnachricht, I0014 Agent beitreten). I0013 (Erwähnt werden) ist **kein** Vorgänger — die Erwähnungsregel fehlt also, wenn I0016 dran ist (siehe Regel 6 und Vorschlag an den planer).
- **Werkzeuge der Agenten**: `curl`, `lc warten` aus I0019 im Monitor-Werkzeug von Claude Code oder als Wächter mit `claude -p`. Grenzen der Aufrufer (Vermutung aus deren Dokumentation, nicht geprobt): das Bash-Werkzeug von Claude Code bricht nach 120 s (Standard) ab; Node `fetch` (undici) wartet höchstens 300 s auf Kopfzeilen; `curl` und Python `requests` warten unbegrenzt.
- **Lastprofil Team-LAN** (`AI-Team/umgebung.md`): 20 Menschen, 50 Agenten, Spitze 30 Nachrichten/s, dauerhaft 5/s, Zustellung < 1 s; viele Agenten auf **einer** Herkunft (`127.0.0.1`).

### Qualitätsziele mit Szenario

| # | Ziel | Szenario mit Maß |
|---|---|---|
| QW1 | Nichts verpasst | Ein Agent ruft `warten` in Schleife und gibt jedes `seit` weiter; drei Sender senden in drei Channels je 300 Nachrichten mit `@agent` in zufälligem Abstand, das Programm wird einmal mittendrin neu gestartet → die Menge der empfangenen Zeiten ist gleich der gesendeten; Doppel nur nach einem Abbruch auf Seiten des Agenten |
| QW2 | Sofort | Lastprofil, 50 Agenten warten mit `erwaehnung` → jede Erwähnung kommt beim Erwähnten < 1 s nach der 201 des Sendens an (Maximum) |
| QW3 | Weckt nur, was soll | Aktivität, Bearbeiten, Löschen, Reaktion (ab I0026), Beitreten/Verlassen anderer, Channel anlegen, Frist ändern, eigene Nachricht → `warten` mit `bis=3` antwortet nach 3 s leer mit `ende: "frist"` |
| QW4 | Sichtbar nur für Beteiligte | Direktnachricht `claude → mia` weckt `shoff` nie und erscheint nie in seiner Antwort, auch ohne Filter |
| QW5 | Ende ohne Hängen | 50 offene Warte-Aufrufe, SIGTERM → alle antworten 200 `ende: "programmende"`, das Programm endet mit Exit 0 in < 1 s |
| QW6 | Begrenzt | Ein Agent öffnet einen fünften gleichzeitigen Warte-Aufruf → 429; Nachholen über 100.000 Nachrichten ohne Treffer kostet je Aufruf < 100 ms Server-Zeit und blockiert keinen Ereignisstrom |

## Optionen

### Gesamtschnitt — woher kommt, was ein wartender Agent erhält?

| | Form | Stärke | Schwäche |
|---|---|---|---|
| **V1 (gewählt)** | **Long-Poll je Aufruf mit eigenem Bus-Abonnement**: abonnieren → Hochwassermarke `H` = `letzte_zeit` unter der Schreibsperre → aus dem Index aller sichtbaren Gespräche `(seit, H]` nachholen → danach Bus-Ereignisse mit Zeit `> H` filtern; der Zeiger liegt beim Agenten | die Ordnung aus ADR-0035 macht **einen** Zeiger verlustfrei; nachholen und live sind zwei Wege, aber dieselbe reine Filterfunktion; kein Zustand im Server außer dem laufenden Aufruf; Neustart verliert nichts, weil der Zeiger beim Agenten liegt und Zeiten über Neustarts steigen | zwei Lesewege (Index, Bus) und eine Dublettenregel an der Marke `H`; ein `Lagged` muss nachholen statt `NeuerStand` |
| V2 | **Bus nur als Klingel, Index als einzige Quelle**: jedes Nachrichten-Ereignis lässt jeden Wartenden seinen Index-Lauf ab Zeiger wiederholen | ein einziger Leseweg, `Lagged` und Dubletten kein Thema | je Nachricht und Wartendem ein Lauf über alle Gespräche unter deren Sperren (50 Wartende × 30/s = 1.500 Läufe/s, Werkbank und Gesprächssperren konkurrieren mit Sendern und dem Öffnen von Channels — gegen QW2 und Q1); die Busdaten liegen schon vor — verworfen |
| V3 | **Postfach je Kennung im Server**: Senden legt unter der Schreibsperre in die Postfächer aller passenden Wartenden; der Server führt „gelesen bis“ je Kennung | der Agent braucht keinen Zeiger (`lc warten` ohne Zustand); mehrere Prozesse eines Agenten teilen sich ein Postfach (Warteschlangen-Semantik) | Zustand je Kennung im Speicher, beim Neustart verloren (Doppel oder Lücke — gegen QW1, ADR-0005 „eine Zeit sagt, was ich gesehen habe“); Arbeit unter der Schreibsperre wächst mit der Zahl der Wartenden; „wer bekommt es, wenn zwei warten“ überrascht — verworfen |
| V4 | **Agenten lesen den Ereignisstrom** `/api/ereignisse` mit Filtern im Query, kein neuer Endpunkt | schon gebaut; das Monitor-Werkzeug liest Zeilen eines Stroms; keine Lücke zwischen zwei Aufrufen | ADR-0007 Option C (verworfen): Strom auswerten ist eine Schleife ohne natürliches Ende eines Befehls; das Thema `nachricht` hat bewusst keinen Stand (ADR-0028) — nach jedem Abriss müsste der Agent über alle Gespräche selbst nachholen, die Erwähnungsregel läge beim Client; `lc warten` kann V1 in einer Schleife zeilenweise ausgeben und deckt den Monitor-Fall ab — verworfen |

Ort im Modell (Mikroarchitektur, keine eigene Variante): Filter und Erwähnungsregel rein, das Nachholen über alle Gespräche in [Gespräche](../Architektur/A00006-gespraeche.md), der Ablauf in [Live-Kanal](../Architektur/A00011-live-kanal.md), Parameter, Halter und Grenze in der [HTTP-Schnittstelle](../Architektur/A00004-http-schnittstelle.md). Ein eigener Baustein „Warten“ wäre ein Durchreicher: er besitzt keine Daten und ruft nur diese drei.

### Einzelfragen

| # | Frage | (a) gewählt | verworfen (Stärke / Grund) |
|---|---|---|---|
| E1 | Was weckt? | nur **Nachrichten** (Thema `nachricht` des Ereignisbusses und Nachrichten im Index): in Channels, im eigenen Direktgespräch, mit Erwähnung des eigenen Namens, als Antwort auf eine eigene Nachricht | (b) auch `aenderung` — Stärke: eine nachträglich eingefügte Erwähnung käme an; Grund: ADR-0048 Regel 11, Muster PO 88/140 „nur Nachricht, Direktnachricht, Erwähnung“, ein Agent würde bei jedem Tippfehler-Fix eines anderen geweckt; (c) alle Themen — gegen PO 88, 140 |
| E2 | Filter ohne Angabe | **alles Sichtbare** (alle Channels, eigene Direktgespräche) wie ADR-0007 | (b) ohne Angabe nur `direkt` + `erwaehnung` — Stärke: ein unbedachter Agent wird nicht 30-mal je Sekunde geweckt; Grund: „holt neue Nachrichten ab“ (Vision) meint alle; ein Filter, der still einschränkt, überrascht; die Anleitung (I0019) empfiehlt `erwaehnung&direkt` (Bestätigung W2) |
| E3 | Mehrere Filter | **Vereinigung**: eine Nachricht trifft, wenn **ein** Teil passt (`channel=a&erwaehnung` = alles aus `a` oder jede Erwähnung irgendwo) | (b) Schnitt — Stärke: „nur Erwähnungen in `a`“ ausdrückbar; Grund: der Fall „meine Channels und alles, was mich anspricht“ ist der häufige, Schnitt ließe Erwähnungen außerhalb von `a` verloren gehen |
| E4 | Zeitlimit `bis` | ganze Sekunden **0–3600, Standard 60**; `0` = nur nachholen, sofort antworten („abholen“) | (b) Standard 600 — Stärke: weniger Aufrufe; Grund: das Bash-Werkzeug von Claude Code bricht nach 120 s ab, ein Agent sähe einen Fehler statt einer leeren Antwort; 60 s kostet bei 50 Agenten < 1 Aufruf/s; (c) über 3600 begrenzen statt abweisen — gegen die Hausregel „abweisen statt umformen“ (ADR-0013, ADR-0051 F2) |
| E5 | `seit` fehlt | **ab jetzt**: `seit` = Hochwassermarke `H` beim Eingang | (b) 422 — Stärke: niemand vergisst den Zeiger; Grund: der erste Aufruf eines neuen Agenten hat keinen; (c) ab Beginn der Frist — eine Flut beim ersten Aufruf |
| E6 | Antwort | `{nachrichten: [{channel, direkt, anlass, nachricht}], seit, ende}`; `nachricht` in der Form des Blocks (ADR-0025 mit `antwort_auf`), älteste zuerst; `seit` = Zeiger für den nächsten Aufruf; `ende` ∈ `treffer`, `frist`, `weitere`, `programmende`, `verlassen` | (b) wie ADR-0007 nur Liste und `seit` — Grund: `lc warten` muss „sofort wieder rufen“ (`weitere`), „Programm startet neu, später wieder“ (`programmende`) und „Sitzung ist weg, neu beitreten“ (`verlassen`) unterscheiden, ohne Statuscodes zu raten; (c) Liste je Gespräch gruppiert — zwei Formen für einen Agenten, der in Zeitfolge abarbeitet |
| E7 | `seit` in der Antwort bei leerer Liste | der **verarbeitete Stand** (größte Zeit, bis zu der alles geprüft ist: `H` oder die Zeit des zuletzt gefilterten Ereignisses) | (b) unverändert zurück — Stärke: einfach; Grund: ein Agent mit `erwaehnung`, der stundenlang nicht erwähnt wird, ließe jeden Aufruf den ganzen Tag erneut durchsuchen |
| E8 | Mehrere Wartende derselben Kennung | **jeder bekommt alles** (Verteilen, keine Warteschlange), jeder ist ein Halter; **höchstens 4 gleichzeitig je Kennung**, der fünfte → 429 `{fehler, grenze: "warten", hoechstens: 4}` mit `Retry-After: 5` | (b) unbegrenzt — Grund: 50 Agenten auf `127.0.0.1` teilen 160 Plätze, ein entlaufener Agent belegte sie alle; (c) 1 je Kennung — Grund: ein neu gestarteter Agent fände seinen still abgerissenen Vorgänger noch ≈ 15 s im Platz (ADR-0017) und bekäme 429 |
| E9 | Schreibkonto | `warten` **bucht nicht** (ADR-0036 Regel 8 wächst nicht) | (b) 1 Einheit je Aufruf — Stärke: bremst eine enge `bis=0`-Schleife; Grund: Lesen bucht nirgends (Verlauf, Teilnehmer); die Schleife bremsen Werkbank und Verbindungsgrenzen; ein gebremstes Warten verpasste Erwähnungen |
| E10 | Erwähnung | über den **Namen** (Teil vor `@` der Kennung des Wartenden), Regel 6 | (b) über die volle Kennung `@name@herkunft` — Grund: Menschen tippen `@claude`, nicht die IP; der Name ist unter lebenden Sitzungen eindeutig (ADR-0012). Antworten bleiben bei der Kennung (ADR-0053 F7) |

## Entscheidung

**Gewählt: V1 mit E1 (a) bis E10 (a).**

Es überwiegt, dass die schon gebaute Ordnung — eine Schreibsperre, streng steigende Zeit, Veröffentlichen unter der Sperre — einen einzigen Zeiger beim Agenten verlustfrei macht, über Gespräche und Neustarts hinweg, ohne dass der Server sich je Agent etwas merkt. In Kauf genommen: zwei Lesewege mit einer Dublettenregel an der Marke `H`, eine Erwähnung, die erst durch Bearbeiten in eine Nachricht kommt, weckt nicht, und Zustellung ist „mindestens einmal“ — wer eine Antwort verliert, bekommt dieselben Nachrichten mit demselben `seit` noch einmal.

Regeln:

1. **Route**: `GET /api/warten?seit=<zeit>&bis=<s>[&channel=<name>]…[&direkt][&erwaehnung]` unter `grenzschutz`, Sitzung über `X-LocalchatA-Sitzung` (Extraktor `Teilnehmer`), Mensch und Agent gleich. `Cache-Control: no-store`.
   - `seit`: Zeit wie im Verlauf (RFC 3339, als Zeitpunkt verglichen, exklusiv), fehlt → `H` (E5). Eine Zeit in der Zukunft ist kein Fehler.
   - `bis`: ganze Sekunden 0–3600, fehlt → 60 (E4).
   - `channel`: wiederholbar, höchstens 64; `direkt`, `erwaehnung`: Schalter **ohne Wert**.
   - Abweisen statt umformen: falsche Zeit → 422 `{fehler, regel: "cursor"}` (Text nennt die Kodierung von `+`); `bis` keine ganze Zahl oder > 3600 → 422 `regel: "bis"`; Schalter mit Wert, mehr als 64 `channel`, doppeltes `seit`/`bis` → 422 `regel: "filter"`; **unbekannter Parameter** → 422 `regel: "parameter"` mit dem Namen (ein Tippfehler wie `erwahnung` würde sonst still zu „alles“); unbekannter Channel → 404 `{fehler, channel}` (wie ADR-0034); ohne Sitzung → 401; Grenze → 429 (Regel 8).
2. **Was trifft** (reine Funktion `treffer(gespraech, nachricht, wartender, filter) -> Option<Anlass>`, E1, E3):
   - nie: eine Nachricht, deren `von` die Kennung des Wartenden ist; ein Direktgespräch, an dem er nicht beteiligt ist; eine gelöschte Nachricht (im Nachholen; live kommt sie als `nachricht` vor dem Löschen).
   - Fakten je Nachricht, `anlass` ⊆ {`direkt`, `erwaehnung`, `antwort`}: `direkt` im eigenen Direktgespräch; `erwaehnung`, wenn der Text den Namen des Wartenden erwähnt (Regel 6); `antwort`, wenn `antwort_auf.von` die Kennung des Wartenden ist (ADR-0053 Regel 9; beim Nachholen aus der Metazeile).
   - ohne Filter trifft jede sichtbare Nachricht (E2); mit Filter trifft sie, wenn sie in einem genannten Channel steht **oder** `direkt` gesetzt ist und `anlass` `direkt` enthält **oder** `erwaehnung` gesetzt ist und `anlass` `erwaehnung` oder `antwort` enthält.
   - `anlass` nennt die Fakten unabhängig vom Filter (für eine gewöhnliche Channel-Nachricht leer).
3. **Ablauf** (in dieser Reihenfolge — die Reihenfolge **ist** die Verlustfreiheit):
   1. Halter der Sitzung nehmen (online, ADR-0016); Grenze je Kennung buchen (Regel 8).
   2. Ereignisbus **abonnieren** (nie den Signalbus, ADR-0051 Regel 12).
   3. `H` = `letzte_zeit()` **unter der Schreibsperre** lesen: alles mit Zeit ≤ `H` steht danach im Index und ist veröffentlicht; alles, was danach veröffentlicht wird, hat eine Zeit > `H` und kommt über das Abonnement.
   4. **Nachholen** auf der Werkbank: Nachrichten mit `seit < zeit ≤ H` aus allen Gesprächen, die der Filter berühren kann (nur genannte Channels, nur eigene Direktgespräche, für `erwaehnung` und ohne Filter alle sichtbaren), in Zeitfolge zusammengeführt, gefiltert nach Regel 2, in der aktuellen Fassung, mit aufgelöstem `antwort_auf`.
   5. Gibt es Treffer → antworten (Regel 4). Sonst **auf den Bus warten**: nur `Ereignis::Nachricht` mit Zeit > max(`seit`, `H`) zählt, alle anderen Themen werden übergangen; jedes geprüfte Nachrichten-Ereignis rückt den verarbeiteten Stand vor; beim ersten Treffer alle schon empfangenen Ereignisse ohne Warten mitnehmen (bis zum Budget), dann antworten.
   6. **`Lagged`** auf dem Abonnement → kein `NeuerStand` an den Agenten, sondern Schritt 3–4 erneut ab dem verarbeiteten Stand; eine Zeile `DEBUG warten` mit Kennung und Zahl.
   7. Es endet mit dem ersten Treffer, nach `bis`, beim Ende-Signal des Programms, beim `weg` der eigenen Sitzung (Ereignis `anwesenheit` mit eigener Kennung und `weg`), oder wenn das Budget erschöpft ist.
4. **Antwort** 200 `{nachrichten, seit, ende}` (E6, E7):
   - `nachrichten`: je Treffer `{channel, direkt, anlass, nachricht}` — `channel` = Name oder `null`, `direkt` = Kennung des Gegenübers im Direktgespräch oder `null` (Feldname und Form bestätigt I0012), älteste zuerst, `nachricht` wie im Block.
   - **Budget**: höchstens 100 Nachrichten und 256 KiB (ADR-0039, mindestens eine); beim Nachholen höchstens **10.000 geprüfte Sätze** je Aufruf. Ist ein Budget erschöpft: sofort antworten mit `ende: "weitere"` (auch mit leerer Liste) — der Agent ruft sofort wieder.
   - `seit`: bei `weitere` die Zeit der letzten gelieferten bzw. geprüften Nachricht; sonst der verarbeitete Stand (≥ `H`, ≥ letzte gelieferte).
   - `ende`: `treffer` (Liste nicht leer, nichts mehr aufgelaufen), `frist` (`bis` abgelaufen, leer), `weitere` (Budget), `programmende` (Ende-Signal; Liste leer; der Agent wartet kurz und tritt nach dem Neustart über `beitreten` mit Header wieder bei — das Token ist dann unbekannt, ADR-0012), `verlassen` (eigene Sitzung `weg`; der nächste Aufruf ist 401).
5. **Verpassen ausgeschlossen** — Begründung, die die Proben W5 und W6 prüfen: (a) Zeiten sind über alle Gespräche streng steigend in der Folge, in der Index und Bus sie sehen (ADR-0035); (b) abonniert wird **vor** dem Lesen von `H`, also liegt jede Nachricht entweder im Index (≤ `H`) oder im Abonnement (> `H`); (c) die Dublettenregel „Bus nur > `H`“ schließt Doppel aus; (d) `seit` ist exklusiv und liegt nie hinter einer ungeprüften Nachricht; (e) über einen Neustart gilt (a) weiter, weil die nächste Zeit an der größten geladenen ansetzt. Zustellung ist „mindestens einmal“: Doppel entstehen nur, wenn der Agent eine Antwort verliert und mit dem alten `seit` wiederkommt; die Zeit ist die ID zum Erkennen.
6. **Erwähnungsregel** (reine Funktion in Gespräche, geteilt mit I0013): eine Erwähnung ist `@`, davor Textanfang oder ein Zeichen außerhalb `[A-Za-z0-9-]`, danach die längste Folge aus `[A-Za-z0-9-]`, in ASCII-Kleinbuchstaben, ohne `-` am Ende; sie erwähnt den Namen, der ihr Zeichen für Zeichen gleicht (ADR-0013). Also: `@claude`, `@Claude`, `@claude.`, `@claude@127.0.0.1` erwähnen `claude`; `shoff@claude` (E-Mail-Form) und `@claude-entwickler` nicht. Kein Markdown-Leser: auch in `` `@claude` `` zählt es. Sammel-Erwähnungen (`@alle`, `@hier`, reserviert nach ADR-0013) wecken **nicht** (Bestätigung W1).
7. **Halter und Anwesenheit**: jeder laufende Aufruf ist ein Halter (Regel 3.1); zwischen zwei Aufrufen hält der Nachlauf von 1 s (ADR-0016) den Agenten online, wenn er sofort wieder ruft. Ein still abgerissener Aufruf fällt über Keepalive nach ≈ 15 s (ADR-0017); ein sauber abgebrochener sofort (ADR-0007 Probe).
8. **Grenzen**: höchstens **4 gleichzeitige Warte-Aufrufe je Kennung** (E8; Zähler im HTTP-Baustein, im Speicher, Freigabe beim Ende des Aufrufs, auch bei Abbruch); dazu die bestehenden Grenzen der Verbindungen (160 je Gegenstelle, 512 gesamt). Das 429 erzeugt eine Grenz-Episode wie die anderen 429 (ADR-0042, Katalog um `warten` ergänzen). Kein Schreibkonto (E9).
9. **Ende-Signal**: der Aufruf hört `ende` selbst (wie `/api/ereignisse`) und antwortet sofort mit `ende: "programmende"` — die 5-s-Frist beim Beenden wird nie ausgeschöpft (QW5).
10. **Beobachtbarkeit**: `/api/gesundheit` nennt `wartend` (laufende Warte-Aufrufe); 429 als Episode (Regel 8); `Lagged` als DEBUG (Regel 3.6). Keine Zeile je Aufruf auf INFORMATION — 50 Agenten mit 60 s wären 72.000 Zeilen am Tag.

## Konsequenzen

**Positiv:**
- Ein Agent braucht nur eine Zeit, um nichts zu verpassen — über Channels, Direktgespräche, Erwähnungen und Neustarts beider Seiten.
- Was weckt, entscheidet eine reine Funktion; Nachholen und live benutzen sie beide. F0158 (Antwort weckt) ist ein Zweig darin, die Bubble B0712 muss **nicht** geteilt werden.
- Signale und Änderungen wecken durch den Bau nicht: der Signalbus wird nie abonniert, und auf dem Ereignisbus zählt nur das Thema `nachricht`.
- Kein Zustand je Agent im Server; die Grenze je Kennung ist ein Zähler, der mit dem Aufruf verschwindet.

**Negativ / Inkauf zu nehmen:**
- Eine Erwähnung, die erst durch Bearbeiten in eine Nachricht kommt, weckt nicht; ein Agent, der Änderungen braucht, liest den Strom oder später ADR-0048 F2.
- Ohne Filter wird ein Agent in belebten Channels oft geweckt; die Antwort sammelt dann bis zu 100 auf einmal.
- Doppel nach verlorenen Antworten sind möglich (mindestens einmal); die Zeit ist die ID.
- Eine Erwähnung über den Namen trifft auch einen späteren Teilnehmer gleichen Namens von anderer Herkunft, wenn er mit einem alten `seit` nachholt.
- Nachholen mit `erwaehnung` liest jeden Text seit `seit` (höchstens 10.000 Sätze je Aufruf); ein Agent, der einen Tag fort war, braucht mehrere Aufrufe mit `ende: "weitere"`.

**Pre-Mortem (ein Monat nach dem Bau gescheitert, weil …):**
1. … ein Agent nach seinem eigenen Neustart Erwähnungen verpasst, weil er `seit` nicht gespeichert hat und ohne `seit` „ab jetzt“ bekommt. Früh-Indikator: „hat nicht reagiert“ trotz Erwähnung während seines Neustarts. Gegenmaßnahme: `lc warten` (I0019) speichert `seit` in einer Datei; die Anleitung sagt es als ersten Satz.
2. … Erwähnungen aus einem zweiten Channel unter Last verloren gehen, weil jemand die Reihenfolge „abonnieren → `H` → nachholen“ zu „nachholen → abonnieren“ vereinfacht. Früh-Indikator: Probe W5 (Speicherablage, die in Channel A beim Anhängen wartet). Gegenmaßnahme: Regel 3, Mutante M1.
3. … der Betreiber im Gespräch mit seinen Agenten Verbindungsfehler sieht, weil 50 Agenten auf `127.0.0.1` mit je mehreren Warte-Aufrufen und Sendungen die 160 Plätze füllen. Früh-Indikator: abgewiesene Verbindungen im Health-Check, `wartend` > 50. Gegenmaßnahme: Regel 8 (4 je Kennung), Anleitung: ein Warte-Aufruf je Agent.
4. … Agenten `warten` für kaputt halten, weil ihr Werkzeug vor `bis` abbricht. Früh-Indikator: Fehler „timeout“ im Agentenprotokoll statt `ende: "frist"`. Gegenmaßnahme: Standard 60 s (E4); Anleitung nennt die Grenzen der Werkzeuge.

**Folgeentscheidungen / Aufgaben** (Vorschläge an den planer, keine Arbeitsliste des architekt):
- [ ] Anforderung für I0016 schreiben (Kriterien aus QW1–QW6 und den Proben W1–W10), Bubbles wie unten.
- [ ] WBS: Erwähnungsregel (Regel 6) gehört dem, der zuerst gebaut wird — I0016 oder I0013; der andere nutzt sie. Notiz an I0013.
- [ ] I0012: Direktgespräche auf den Bus mit erkennbarem Gespräch und Beteiligten; Feld `direkt` in der Warte-Antwort bestätigen.
- [ ] I0019: Textbaustein (Regel 4 `ende`, `seit` speichern, mindestens einmal, 401 nach Neustart → `beitreten` mit Header, 429 `warten`, Grenzen der Werkzeuge, `erwaehnung&direkt` empfohlen); OpenAPI ([A00002](../Architektur/A00002-programm-localchata.md)).
- [ ] B0698 P10 (R00045) und B0712 (R00047) werden mit I0016 fahrbar.

### Bausteine für planer und entwickler (Test zuerst; heute alle rot, weil nichts gebaut ist)

| # | Baustein (Ebene) | erster Test — Eingabe → Ausgabe, ersetzte Provider | Mutanten, die rot werden müssen |
|---|---|---|---|
| W1 | Erwähnungsregel (Unit, rein, `src/gespraeche/`) | `"@claude hi"`, `"hi @Claude."`, `"@claude@127.0.0.1"`, `` "`@claude`" ``, `"@claude-"` → erwähnt `claude`; `"shoff@claude"`, `"@claude-entwickler"`, `"@ claude"`, `"@claudex"`, `"@alle"` → nicht `claude`; `"@claude-entwickler"` → `claude-entwickler` | Grenze vorn weg (E-Mail weckt); längste Folge weg (`claude-entwickler` weckt `claude`); Groß-/Kleinschreibung streng |
| W2 | Trefferregel (Unit, rein) | Tabelle aus Gespräch × Absender × Text × `antwort_auf.von` × Filter → `Option<Anlass>`: eigene Nachricht → nie; fremdes Direktgespräch → nie, auch ohne Filter; `channel=a&erwaehnung` + Nachricht in `b` mit `@agent` → `[erwaehnung]`; Antwort von `anna` auf `agent` → `[antwort]` unter `erwaehnung`; Antwort von `agent` auf sich → nie; gelöscht → nie | Vereinigung als Schnitt; eigene Nachricht durchlassen; Sichtbarkeit weg; `antwort` ohne `von != K` (Z12 aus ADR-0053) |
| W3 | Parameter (Unit, rein) | `bis` fehlt → 60; `0`, `3600` ok; `3601`, `-1`, `1.5`, `abc` → `bis`; `erwaehnung=1` → `filter`; 65 × `channel` → `filter`; `erwahnung` → `parameter` mit Namen; `seit` mit `+` unkodiert → `cursor` | Obergrenze 3601 zulassen; unbekannten Parameter übergehen |
| W4 | Nachholen über alle Gespräche (Unit, Gespräche mit Speicherablage, Testuhr) | drei Channels mit verschränkten Zeiten `t1…t9`, `seit = t3`, `H = t8` → genau `t4…t8` in Zeitfolge, gefiltert; Budget 100 / 256 KiB / 10.000 Sätze → `weitere` mit `seit` = letzte geprüfte | Zusammenführen je Gespräch statt nach Zeit; `≤ seit` statt `< seit`; `H` übersehen (`> H` mitgenommen) |
| W5 | Ablauf verlustfrei (Unit, tokio pausiert, Schranken im Ablauf wie `src/gespraeche/schranke.rs`) | (a) Schranke **zwischen Abonnieren und `H`**: währenddessen sendet `anna` `t1` mit `@agent` → `t1` liegt im Index (≤ `H`) **und** im Abonnement → genau einmal geliefert; (b) Schranke **zwischen Nachholen und Bus-Warten**: währenddessen sendet `anna` `t2` mit `@agent` in einem anderen Channel → `t2` genau einmal; (c) ADR-0035 Pre-Mortem 1: Speicherablage hält das Anhängen in Channel A an, `warten` startet in dieser Zeit, B sendet danach → nach Freigabe beide in Zeitfolge, je einmal; (d) Bus mit Kapazität 2 und 5 Nachrichten ohne Verarbeitung → `Lagged` → alle 5 genau einmal | M1 Abonnieren nach `H` oder nach dem Nachholen ((b) verliert `t2`); Dublettenregel „> `H`“ weg ((a) doppelt); `Lagged` als `NeuerStand` oder still übergangen ((d) verliert) |
| W6 | Route, Halter, Grenze, Ende (Integration, echtes Programm) | ohne Header 401; unbekannter Channel 404; 5. gleichzeitiger Aufruf derselben Kennung 429 `grenze: "warten"` mit `Retry-After: 5`, eine andere Kennung derselben Herkunft nicht; während des Wartens `online` in `/api/teilnehmer`; `POST /api/verlassen` → offener Aufruf 200 `ende: "verlassen"`; SIGTERM bei 3 offenen Aufrufen → `programmende`, Exit 0 < 1 s; `bis=0` antwortet sofort | Grenze je Herkunft statt je Kennung; Zähler wird beim Abbruch nicht frei (nach 4 abgebrochenen Aufrufen 429); Ende-Signal nicht gehört (5 s) |
| W7 | Wecken und Nicht-Wecken (Integration, echtes Programm) | `codex` wartet `erwaehnung`; `anna` sendet `@codex` → < 1 s mit `anlass: ["erwaehnung"]`; Direktnachricht → `["direkt"]` (ab I0012); Aktivität setzen, Bearbeiten, Löschen, Channel anlegen, Frist, eigenes Senden → mit `bis=3` `ende: "frist"`; ohne Filter weckt eine Channel-Nachricht mit `anlass: []` | Thema `aenderung` zählt; Signalbus abonniert; eigene Nachricht weckt |
| W8 | Verlust über Neustart (Integration, echtes Programm) | QW1 verkleinert: 3 Sender × 3 Channels × 100 Nachrichten mit `@agent`, Agent in Schleife mit `seit`, Neustart nach ≈ 150 (Agent tritt mit Header wieder bei) → Menge empfangen = Menge gesendet | `seit` in der Antwort = erste statt letzte gelieferte Zeit (Doppel); `seit` = Hochwassermarke beim **Antworten** statt verarbeiteter Stand (Nachrichten, die nach `H` veröffentlicht, aber noch nicht aus dem Abonnement gelesen sind, gehen verloren) |
| W9 | Last (Release, Generator in `e2e/messung/`) | QW2: 50 Wartende (`erwaehnung`), 30 Nachrichten/s über 60 s, jede zehnte mit Erwähnung → Zustellung 201 → 200 p50, p90, max < 1 s; `wartend` = 50 im Health; dazu B0698 P10 | — (Messung) |
| W10 | Anleitung und OpenAPI (I0019) | Textbaustein nach Regel 4; OpenAPI nennt Parameter, `ende`, 429 `warten` | — |

Mini-Entwickler (Vorschlag): W1, W3 ja (rein, Probe vorgegeben); W2, W4, W5, W6, W7, W8 `entwickler`; W9 nach W8.

## Offene Fragen

Bestätigungen beim `product-owner` (gewählt nach Empfehlung, keine Richtungsfrage; kippt eine, kommt eine Ergänzung):

- **W1 Sammel-Erwähnung**: weckt `@alle` oder `@hier` jeden wartenden Agenten? Empfehlung: **nein** in I0016 — die Namen sind reserviert (ADR-0013), Wecken ließe sich später additiv ergänzen (Muster PO 88, 140); 50 Agenten auf ein `@alle` erzeugen 50 Antworten zugleich.
- **W2 Ohne Filter**: weckt `warten` ohne Angabe jede sichtbare Nachricht (gewählt, wie ADR-0007) oder nur, was an den Agenten gerichtet ist? Empfehlung: **alles Sichtbare**, die Anleitung empfiehlt `erwaehnung&direkt`.
- **W3 Bearbeiten fügt Erwähnung hinzu**: weckt sie? Empfehlung: **nein** (E1; wie ADR-0048 Regel 11, PO 88, 140).

## Quellen / Bezuege

- [A00011 Live-Kanal](../Architektur/A00011-live-kanal.md), [A00004 HTTP-Schnittstelle](../Architektur/A00004-http-schnittstelle.md), [A00006 Gespräche](../Architektur/A00006-gespraeche.md), [A00010 Anwesenheit](../Architektur/A00010-anwesenheit.md), [A00014 Schreibkonto](../Architektur/A00014-schreibkonto.md), [A00019 Aktivität](../Architektur/A00019-aktivitaet.md)
- ADR-0007, ADR-0010, ADR-0012, ADR-0013, ADR-0016, ADR-0017, ADR-0025, ADR-0028, ADR-0035, ADR-0036, ADR-0039, ADR-0042, ADR-0048, ADR-0051, ADR-0053
- Vision R00000 „Agenten können warten und reagieren“, „Direktnachrichten und Erwähnungen“, „Mensch und KI sind gleichberechtigt“; WBS I0016, I0013, I0019, F0158/B0712, B0698; PO-Entscheidungen Zeilen 88, 140, 150 (`AI-Team/po-entscheidungen.md`)
- Grenzen der Aufrufer (nicht geprobt): Claude Code Bash-Werkzeug, Standard-Zeitgrenze 120 s (Werkzeugbeschreibung); undici `headersTimeout` 300 s (Dokumentation von undici, Abruf aus dem Gedächtnis, Vermutung)

## Ergänzungen
