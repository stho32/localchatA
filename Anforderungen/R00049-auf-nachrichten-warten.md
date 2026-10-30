---
id: R00049
status: Neu
datum: 2026-10-30
---

# R00049: Auf Nachrichten warten

## Beschreibung

Ein Agent wartet **blockierend** auf Neues für sich — Nachrichten in Channels, Direktnachrichten, Erwähnungen — und erhält es, sobald es da ist, ohne eine Polling-Schleife: ein einziger Aufruf `GET /api/warten`, der offen bleibt, bis etwas trifft, die gewünschte Zeit abläuft oder die Instanz endet. Der Agent hält **einen** Zeiger `seit` (die Zeit der zuletzt verarbeiteten Nachricht), gibt ihn mit jedem Aufruf weiter und verpasst damit nichts — über alle Gespräche, über sein eigenes Neustarten und über den Neustart der Instanz hinweg. Mensch und Agent rufen dieselbe Route; geschrieben wird nichts.

Geweckt wird **nur von Nachrichten**: neue Nachrichten in Channels, in einem Direktgespräch des Wartenden und solche, die ihn erwähnen. Aktivitätssignale, Bearbeiten, Löschen, Beitreten und Verlassen anderer, neue Channels und Fristen wecken nie. Wer eine Antwort auf die eigene Nachricht erhält, wird in R00047 (F0158) geweckt; dieses Dokument legt den Platz dafür fest, baut den Zweig aber nicht.

Zahlt ein auf: [Vision](R00000-vision.md) — „Agenten können warten und reagieren“, dazu „Direktnachrichten und Erwähnungen“, „Mensch und KI sind gleichberechtigt“, „Live überall“ und die Leitplanke „Lastprofil Team-LAN“ (Zustellung live unter 1 s).

Herkunft: Karte #17 (WBS I0016; Ergänzung F0170 aus dem Security-Befund 1 zu F0166 mit der ADR-0054-Ergänzung „Nachholen von `warten` hat eine eigene Bank, ein Konto in Sätzen je Kennung und Herkunft und ein kleineres Budget“; Ergänzung F0171 aus der Nachmessung von F0170 mit der ADR-0054-Ergänzung „Kosten-Einheit des Nachholens“ und den Rückmeldungen des `entw2`); Entscheidung des `architekt` in [ADR-0054](../Dokumentation/ADRs/0054-warten-long-poll-ueber-alle-gespraeche-ein-zeiger-seit-filter-als-vereinigung-nur-nachrichten-wecken.md) (Qualitätsziele QW1–QW6, Regeln 1–10, Bausteine W1–W10); die drei Bestätigungen W1–W3 sind vom `product-owner` entschieden (`AI-Team/po-entscheidungen.md` Zeilen 159–161, Stand `main`); Weckkreis aus Zeilen 88 und 140, Antwort weckt aus Zeile 150.

Umfang (WBS `Dokumentation/Planung/localchata.md`, Dialog D0005 Agent an der API): **I0016** mit sechs Slices — **F0166** Warten auf Neues in Channels (Ausbaustufe Warten 1), **F0167** Erwähnung weckt, **F0168** Direktnachricht weckt (Ausbaustufe Warten 2, nach I0012), **F0169** Warten endet mit Grund und bleibt begrenzt, **F0170** Nachholen bleibt begrenzt (Ergänzung 2026-10-30 aus dem Security-Befund 1 zu F0166, Ausbaustufe Nachholen 1), **F0171** Nachholen zählt nach Aufwand (Ergänzung 2026-10-30 nach F0170, Ausbaustufe Nachholen 2); 26 Bubbles B0735–B0751, B0755–B0759 und B0760–B0763. Der Zweig „Antwort weckt“ gehört F0158 in [R00047](R00047-mit-zitat-antworten-api-und-browser.md).

### Domänenbezug

Kein Glossar im Projekt (`Dokumentation/Domaenenwissen/` fehlt). Begriffe wie in R00004, R00011, R00039, R00045, R00047: Nachricht (angegeben über ihre Zeit), Kennung (`name@herkunft`), Name (der Teil vor `@`), Gespräch (Channel oder Direktgespräch), Verlauf, Halter, Gnadenfrist, Plattenquote. Neu und hier festgelegt: **Wartekommando** (`warten`, der Aufruf), **Zeiger** (`seit`, exklusiv: es kommen nur Nachrichten mit späterer Zeit), **Treffer** (eine Nachricht, die den Wartenden weckt), **Anlass** (warum sie trifft: `direkt`, `erwaehnung`; `antwort` mit F0158), **verarbeiteter Stand** (die Zeit, bis zu der alles geprüft ist — der `seit` für den nächsten Aufruf), **Erwähnung** (`@name` nach der Regel unten).

## Geschäftlicher Nutzen

Wirkungskette: Ein Agent soll auf `@claude` in `# technik` antworten → ohne `warten` fragt er in einer Schleife alle paar Sekunden den Verlauf ab, belegt Verbindungen, verbraucht Werkzeugaufrufe und reagiert verspätet oder gar nicht, wenn ein Aufruf scheitert → mit `warten` liegt ein Aufruf offen, die Erwähnung kommt binnen 1 s, und der Zeiger `seit` stellt sicher, dass auch eine Nachricht, die während seines eigenen Neustarts ankam, beim nächsten Aufruf nachgeholt wird → der Agent ist ein Teilnehmer, der erreichbar ist (Vision: „Agenten können warten und reagieren“), und Claude Code kann sich in I0019 mit `lc warten` im Monitor-Werkzeug wecken lassen.

## Funktionale Anforderungen

Festgelegt in ADR-0054: Route `GET /api/warten?seit=<zeit>&bis=<s>[&channel=<name>]…[&direkt][&erwaehnung]` unter dem Grenzschutz, Sitzung über `X-LocalchatA-Sitzung`, `Cache-Control: no-store`; Antwort 200 `{nachrichten, seit, ende}` mit `nachrichten` als Liste von `{channel, direkt, anlass, nachricht}`, älteste zuerst, `nachricht` in der Form des Verlaufsblocks (jedes Feld immer da). Die Wortlaute der deutschen Fehlertexte legt der Bau fest (sie nennen den Parameter und, bei `seit`, die Kodierung von `+`).

### I0016 Auf Nachrichten warten

**F0166 Warten auf Neues in Channels (Warten 1)** — B0735–B0740

- **Aufruf und Parameter**: `seit` ist die Zeit einer Nachricht wie im Verlauf (RFC 3339, als Zeitpunkt verglichen, exklusiv; `+` muss kodiert werden); fehlt es, gilt „ab jetzt“ (die Zeit der jüngsten Nachricht beim Eingang des Aufrufs), eine Zeit in der Zukunft ist kein Fehler. `bis` sind ganze Sekunden 0–3600, Standard 60; `bis=0` holt nur nach und antwortet sofort. `channel` ist wiederholbar (höchstens 64). Abgewiesen wird statt umgeformt: falsche Zeit 422 `regel: "cursor"`, `bis` keine ganze Zahl oder über 3600 422 `regel: "bis"`, Schalter mit Wert, mehr als 64 `channel`, doppeltes `seit` oder `bis` 422 `regel: "filter"`, **unbekannter Parameter** 422 `regel: "parameter"` mit dem Namen (ein Tippfehler wie `erwahnung` darf nicht still zu „alles“ werden), unbekannter Channel 404 mit `channel`, ohne Sitzung 401.
- **Was trifft**: ohne Filter jede neue Nachricht in jedem Channel (Annahme A2 unten); mit `channel=a` nur Nachrichten aus `a`; mehrere Filter sind eine **Vereinigung** (eine Nachricht trifft, wenn **ein** Teil passt). Nie trifft eine eigene Nachricht des Wartenden und nie eine gelöschte Nachricht (beim Nachholen); eine bearbeitete kommt in der aktuellen Fassung. `anlass` ist für eine gewöhnliche Channel-Nachricht die leere Liste.
- **Nachholen, dann warten**: der Aufruf liefert zuerst alle Treffer mit Zeit nach `seit` bis zur Marke `H` (der jüngsten bereits veröffentlichten Zeit beim Eingang), über **alle** Gespräche in Zeitfolge zusammengeführt; gibt es keine, wartet er auf neue. Reihenfolge der Schritte (die Reihenfolge **ist** die Verlustfreiheit): abonnieren, dann `H` unter der Schreibsperre lesen, dann nachholen, dann den Bus filtern — Nachrichten mit Zeit nach `H` kommen nur aus dem Bus, nie doppelt. Verliert das Abonnement Ereignisse (zu langsam gelesen), holt der Aufruf aus dem Index nach statt dem Agenten einen „Neuer Stand“ zu melden.
- **Antwort und Zeiger**: `ende` ist `treffer` (Liste nicht leer, nichts mehr aufgelaufen), `frist` (`bis` abgelaufen, Liste leer) oder `weitere` (ein Budget ist erschöpft: sofort wieder rufen). `seit` ist der **verarbeitete Stand** — bei `weitere` die Zeit der letzten gelieferten bzw. geprüften Nachricht, sonst mindestens `H` und mindestens die Zeit der letzten gelieferten. Auch bei leerer Liste rückt `seit` vor, damit ein Agent mit engem Filter nicht täglich dieselbe Strecke neu durchsucht. Zustellung ist „mindestens einmal“: verliert der Agent eine Antwort und ruft mit dem alten `seit` wieder, bekommt er dieselben Nachrichten noch einmal; die Zeit ist die ID zum Erkennen.
- **Budget**: höchstens 100 Nachrichten und 256 KiB je Antwort (mindestens eine); beim Nachholen höchstens **2.000** Kosten je Aufruf **[K1]** (Budget nach F0170; zuvor 10.000 Sätze; seit F0171 in Kosten, ein Satz unter 1 KiB kostet 1, siehe [K6]).
- **Wecken nur durch Nachrichten**: Aktivität (`tippt`, `denkt_nach`), Bearbeiten und Löschen (ohne neue Erwähnung des Wartenden), Beitreten und Verlassen anderer, Channel anlegen, Frist ändern und die eigene Nachricht wecken nicht; `warten` abonniert den Signalbus nie. Reaktionen (ab I0026) wecken ebenfalls nicht (PO Zeile 88); das Kriterium dazu trägt I0026 nach.
- **Halter und Sichtbarkeit**: jeder laufende Aufruf ist ein Halter der Sitzung — sie gilt als online, solange er läuft, und zwischen zwei Aufrufen hält der Nachlauf von 1 s sie online; mehrere Aufrufe derselben Kennung bekommen jeder alles (Verteilen, keine Warteschlange). `warten` bucht nichts am Schreibkonto. `/api/gesundheit` nennt `wartend` (laufende Aufrufe).

**F0167 Erwähnung weckt** — B0741–B0746

- **Erwähnungsregel** (eine reine Funktion, geteilt mit I0013): eine Erwähnung ist `@`, davor Textanfang oder ein Zeichen außerhalb `[A-Za-z0-9-]`, danach die längste Folge aus `[A-Za-z0-9-]`, in ASCII-Kleinbuchstaben verglichen, ohne `-` am Ende; sie erwähnt den Namen, der ihr Zeichen für Zeichen gleicht. Also erwähnen `@claude`, `@Claude`, `@claude.`, `@claude@127.0.0.1` und `` `@claude` `` den Namen `claude`; nicht erwähnt wird er von `shoff@claude`, `@claude-entwickler`, `@ claude`, `@claudex`, `@alle`; `@claude-entwickler` erwähnt `claude-entwickler`. Es gibt keinen Markdown-Leser: auch im Code zählt es.
- **Filter `erwaehnung`**: trifft jede Nachricht, die den Namen des Wartenden (Teil vor `@` seiner Kennung) erwähnt, in jedem Channel; `anlass` enthält `erwaehnung`. In einer Vereinigung (`channel=a&erwaehnung`) trifft alles aus `a` **oder** jede Erwähnung irgendwo. Eine Nachricht des Wartenden, die seinen eigenen Namen enthält, weckt ihn nicht.
- **Nachholen**: eine Erwähnung, die ankam, während der Agent nicht wartete, kommt mit seinem alten `seit` beim nächsten Aufruf; das Nachholen mit `erwaehnung` liest jeden Text seit `seit` (höchstens 2.000 Kosten je Aufruf [K1, K6], dann `ende: "weitere"` auch mit leerer Liste).
- **Nichts verpasst, auch über einen Neustart** (QW1): ein Agent, der in Schleife wartet und jedes `seit` weitergibt, empfängt die Menge aller an ihn gerichteten Nachrichten genau einmal — auch wenn die Instanz mittendrin neu gestartet wird (der Agent tritt danach mit seinem Sitzungs-Header wieder bei); Doppel nur nach einem Abbruch auf Seiten des Agenten.
- **Nachholkosten**: das Nachholen mit `erwaehnung` liest jeden Text; über 100.000 Nachrichten ohne Treffer kostet es je Aufruf unter 100 ms Server-Zeit und blockiert keinen Ereignisstrom (QW6, zweiter Teil).
- **Sofort** (QW2): im Lastprofil Team-LAN kommt jede Erwähnung beim Erwähnten binnen 1 s nach der 201 des Sendens an.

**F0168 Direktnachricht weckt (Warten 2)** — B0747–B0749 (setzt I0012 voraus)

- Der Filter `direkt` trifft jede Nachricht in einem Direktgespräch, an dem der Wartende beteiligt ist (nicht seine eigenen); `anlass` enthält `direkt`, `direkt` im Eintrag ist die Kennung des Gegenübers, `channel` ist dann `null` (umgekehrt: in einem Channel ist `direkt` `null` und `channel` der Name). Ohne Filter treffen die eigenen Direktgespräche mit; Nachholen und Wecken gelten gleichermaßen.
- **Sichtbar nur für Beteiligte** (QW4): eine Direktnachricht `claude → mia` weckt `shoff` nie und erscheint nie in seiner Antwort — auch ohne Filter, auch beim Nachholen mit einem `seit` vor der Nachricht.

**F0169 Warten endet mit Grund und bleibt begrenzt** — B0750–B0751

- **Ende**: beim Ende der Instanz (SIGTERM, Strg+C) antwortet jeder offene Aufruf sofort 200 mit leerer Liste und `ende: "programmende"` — die Frist von 5 s beim Beenden wird nie ausgeschöpft; verlässt der Wartende selbst (`POST /api/verlassen`), antwortet sein offener Aufruf 200 `ende: "verlassen"`, der nächste Aufruf mit diesem Token ist 401. Nach einem Neustart der Instanz ist das Token unbekannt: der Agent tritt wieder bei (R00003) und ruft mit seinem `seit` weiter.
- **Begrenzt**: höchstens 4 gleichzeitige Aufrufe je Kennung; der fünfte wird mit 429 `{fehler, grenze: "warten", hoechstens: 4}` und `Retry-After: 5` abgewiesen. Eine andere Kennung derselben Herkunft ist davon nicht betroffen; endet oder bricht ein Aufruf ab, ist sein Platz frei. Das 429 erzeugt eine Grenz-Episode im Betriebsprotokoll wie die übrigen 429 (Katalog ADR-0042 um `warten` ergänzt). Keine Zeile je Aufruf auf der Stufe INFORMATION.

**F0170 Nachholen bleibt begrenzt** — B0755–B0759 (Ergänzung 2026-10-30 aus dem Security-Befund 1 zu F0166; ADR-0054 Ergänzung „Nachholen von `warten` hat eine eigene Bank, ein Konto in Sätzen je Kennung und Herkunft und ein kleineres Budget“)

*Seit F0171 zählen Budget, Konto und Reservierung in Kosten statt in Sätzen (bei Sätzen unter 1 KiB ist beides gleich); der Text dieses Abschnitts beschreibt die Stufe Nachholen 1.*

Der Befund: ein Warte-Aufruf mit altem `seit` und `erwaehnung` prüft Sätze auf der gemeinsamen Werkbank (gemessen ≈ 5 µs je Satz); wer mit beliebig vielen Namen in Schleifen `bis=0` ruft, hält die Werkbank so besetzt, dass der Verlauf von ≈ 1 ms auf über 900 ms steigt (64 Schleifen, p90 946 ms, max 1.338 ms). Die Grenze von 4 gleichzeitigen Aufrufen je Kennung (F0169) hilft nicht: Namen sind frei, und sie zählt Wartezeit, nicht Arbeit. Gewählt ist eine Kombination (V7): eigene Bank, Konto in Sätzen und kleineres Budget — jede Konstante daraus steht in genau einem Kriterium (K1–K5) und ist einzeln kippbar.

- **Nachholbank [K2]**: das Nachholen von `warten` (auch die Wiederholung nach `Lagged`) läuft nur auf einer eigenen Bank, nie auf der Werkbank; Verlauf, Suche und Anhang-Metadaten berühren sie nie. Plätze: Kerne / 2, mindestens 1, fest seit dem Start.
- **Budget [K1]**: höchstens 2.000 geprüfte Sätze je Aufruf (zuvor 10.000), 100 Nachrichten und 256 KiB bleiben; erschöpft: `ende: "weitere"`.
- **Nachholkonto**: zwei Eimer in geprüften Sätzen im Speicher — **je Kennung [K3]** (Vorrat 100.000, Nachschub 20.000 je Sekunde) und **je Herkunft [K4]** (Vorrat 500.000, Nachschub 60.000 je Sekunde; Herkunft ist die IP der Gegenstelle). Jeder Aufruf **reserviert [K5]** das Budget in beiden Eimern (alles oder nichts) und erstattet nach dem Nachholen, was er nicht geprüft hat; ein leichter Aufruf kostet fast nichts. Das Nachholen nach `Lagged` bucht nichts.
- **429 `nachholen`**: reicht ein Eimer nicht, antwortet der Aufruf **vor** Halter, Abonnement und dem Lesen von `H` mit 429 `{fehler, grenze: "nachholen", von: "kennung" | "herkunft", wieder_in_ms}` und `Retry-After` (⌈`wieder_in_ms` / 1000⌉, mindestens 1); nichts geht verloren, der Zeiger bleibt gültig, mit demselben `seit` nach der Wartezeit kommt dasselbe an.
- **Betrieb**: `/api/gesundheit` nennt `nachholen {saetze, gebremst, bank_plaetze, bank_belegt}`; eine Grenz-Episode je Kennung (nicht je 429; Katalog ADR-0042 um `nachholen` ergänzt). Die Grenze von 4 gleichzeitigen Aufrufen (F0169) bleibt unverändert und prüft unabhängig.

**F0171 Nachholen zählt nach Aufwand** — B0760–B0763 (Ausbaustufe Nachholen 2 von F0170; Ergänzung 2026-10-30; ADR-0054 Ergänzung „Kosten-Einheit des Nachholens“)

Der Befund (Lastprobe B0759 des `entw2`, Kommentar „entw2 #17 F0170“): die Einheit „geprüfter Satz“ trägt bei langen Sätzen nicht — ein Channel mit 2.000 Sätzen zu je 16 KiB braucht im Median 111–118 ms je Aufruf statt der erwarteten ≈ 10 ms (≈ 55–59 µs je Satz statt 5 µs; Modell ≈ 4 µs je Satz + ≈ 3,5 µs je KiB). Ein Satz zu 16 KiB kostet damit das ≈ 11-fache eines kurzen; das Konto je Herkunft (60.000 Sätze je Sekunde) liefe bei langen Sätzen auf ≈ 3,3 Kerne. Gewählt ist Variante B von vier (verworfen: Grenze der Probe anheben, zweites Budget in Bytes, zwei Konten): die **Einheit wird Kosten**, alles andere bleibt.

- **Kosten eines Satzes [K6]**: 1 + ⌊Satzbytes / 1.024⌋ (Satzbytes = Länge des Satzes in der Tagesdatei; ist sie nicht billig verfügbar, genügt `text.len()`); Sätze unter 1 KiB kosten 1 — für sie ändert sich nichts.
- **Budget [K1] in Kosten**: höchstens 2.000 Kosten je Aufruf (100 Nachrichten und 256 KiB bleiben). Ein Satz, der das Rest-Budget überschreitet, wird nicht mehr geprüft: `ende: "weitere"` mit `seit` gleich der Zeit des letzten geprüften Satzes; der **erste** Satz eines Aufrufs wird immer geprüft (Fortschritt). Genau 2.000 Kosten verbraucht und nichts ungeprüft → `frist`/`treffer` wie in F0170.
- **Konto und Reservierung [K3, K4, K5] in Kosten**: Vorräte, Nachschub und Reservierung 2.000 unverändert; die Erstattung ist `2.000 − Kosten des Aufrufs`. Das Nachholen nach `Lagged` bucht weiter nichts.
- **Betrieb**: `/api/gesundheit` nennt unter `nachholen` zusätzlich `kosten` (seit dem Start geprüfte Kosten); `saetze` bleibt die Zahl der Sätze. Z2 prüft `kosten`.
- **Bank bleibt [K2]**: ein Konto begrenzt *eine* Herkunft auf ≈ 30 % eines Kerns, die Bank deckt *viele* Herkünfte. Dafür kommt die Probe Z6 (64 Herkünfte) und die Variante Z6b des `security-reviewer` (6 Herkünfte × 16 Sitzungen × 4 Schleifen, gemessen: Mutante „Bank allein aus“ → Verlauf p90 867 ms, max 1.365 ms); Rückfall, wenn die Mutante beide Proben nicht rot macht: Zusicherung nur auf der Probe N2.
- **Aufholzeit (P1)**: ein Tag mit 430.000 Sätzen kostet bei Ø unter 1 KiB ≈ 17 s (unverändert), bei Ø 2 KiB ≈ 60 s, bei Ø 8 KiB ≈ 3 min je Kennung (gerechnet, Rechenarbeit statt Satzzahl als Maßstab). Das ist dieselbe Aussage wie P1 (Verlustfreiheit, nicht Aufholgeschwindigkeit); eine neue Frage an den `product-owner` entsteht nur, wenn die Lastprobe im Lastprofil (kurze Sätze) über 20 s misst.

### Entschiedene Annahmen (PO Zeilen 159–161) — Kippen ändert genau eine Zeile

Drei Festlegungen, die der `product-owner` bestätigt hat und die jederzeit umkehrbar sind. Jede steht **einmal** als eigenes Akzeptanzkriterium, markiert mit **[A1]**, **[A2]** bzw. **[A3]**, und trägt eine eigene Story-Zeile; kippt eine, ändert sich dieses Kriterium (und ADR-0054 bekommt eine Ergänzung), die übrigen bleiben.

- **A1** (PO 159): `@alle` und `@hier` wecken nicht; die Namen sind reserviert (ADR-0013).
- **A2** (PO 160): ohne Filter weckt jede neue Nachricht in jedem sichtbaren Gespräch (Channels, die eigenen Direktgespräche); die Anleitung (I0019) empfiehlt `erwaehnung&direkt`.
- **A3** (PO 161): eine Bearbeitung, die eine Erwähnung hinzufügt, weckt nicht.

### Konstanten des Nachholens (F0170, F0171) — jede einzeln kippbar

Die Werte sind aus dem Messwert des `security-reviewer` gerechnet (≈ 5 µs je Satz, Last 10–14) und werden mit der Lastprobe B0759 bestätigt oder korrigiert. Jede Konstante hat ein Kriterium; ein Kippen ändert genau diese Zeile (und die Story, die sie trägt), nie die übrigen. Zwei Fragen an den `product-owner` laufen parallel (Empfehlung gilt als Auslegung bis zur Antwort, keine Richtungsfrage der Vision):

| Kürzel | Konstante | Wert | Kriterium | Kippt, wenn |
|---|---|---|---|---|
| K1 | Budget je Aufruf | 2.000 Kosten (bis F0171: geprüfte Sätze) | F0170 „Budget“, F0167 „Budget der Prüfung“, F0171 „Budget in Kosten“ | Rechenzeit je Kosteneinheit über 10 µs (Z5 neu) |
| K2 | Plätze der Nachholbank | Kerne / 2, mindestens 1 | F0170 „Bank“ | die Bank zum Engpass wird |
| K3 | Konto je Kennung | Vorrat 100.000, Nachschub 20.000 je Sekunde | F0170 „Konto je Kennung“, „Aufholen“ | **P1**: Aufholzeit nach einem Tag ≈ 17 s statt 2 s nicht hinnehmbar |
| K4 | Konto je Herkunft | Vorrat 500.000, Nachschub 60.000 je Sekunde | F0170 „Konto je Herkunft“, „Normalbetrieb“ | **P2**: mehr als 50 Agenten je Adresse |
| K5 | Reservierung je Aufruf | gleich dem Budget (K1) | F0170 „Reservierung“ | K1 ändert sich |
| K6 | Byte je Kosteneinheit | 1.024 (Kosten = 1 + ⌊Satzbytes / 1.024⌋) | F0171 „Kosten eines Satzes“ | Z5 neu rot, weil die Namenssuche teurer ist als die Reserve des Modells (≈ 1,4 ×) deckt → 512 Byte |

- **P1 Aufholzeit** (offen beim PO, Empfehlung hinnehmbar): ein Agent, der einen Tag fortblieb (≈ 430.000 Sätze), holt in ≈ 17 s nach statt in ≈ 2 s, nach einer Woche ≈ 145 s (gerechnet). Wird P1 verneint, ändert sich K3 (und die Zeile „Aufholen dauert, aber geht“).
- **P2 Agenten je Adresse** (offen beim PO, Auslegung 50 = Lastprofil Team-LAN): sind es mehr, ändert sich K4 (Vorrat und Nachschub der Herkunft anheben, additiv).

## Nicht-funktionale Anforderungen

- **Zeit** (Zweck: Leitplanke „Zustellung live unter 1 s“, Vision „Live überall“): Zustellung eines Treffers an einen offenen Aufruf binnen 1 s, Startpunkt der Uhr ist die 201 des Sendens; erwartet wenige ms (Weg im Prozess: Veröffentlichen unter der Schreibsperre, Filter, Antwort; gerechnet, ungemessen), die Grenze liegt weit darüber. Die **Regel** `bis` ist davon getrennt: ein Aufruf mit `bis=3`, der nichts trifft, antwortet nicht früher als 3,0 s und nicht später als 4,0 s nach dem Eingang der Anfrage (erwartet ≈ 3,0 s; die obere Grenze deckt Last auf dem Rechner ab). Beim Ende der Instanz antworten alle offenen Aufrufe binnen 1 s nach dem Signal und die Instanz endet mit Exit 0 (erwartet wenige ms; ADR-0054 QW5).
- **Last** (Lastprofil Team-LAN): 50 Agenten warten gleichzeitig (je eine Kennung, 50 Ströme von einer Herkunft `127.0.0.1` neben den Sendern), 30 Nachrichten/s Spitze, jede zehnte mit Erwähnung — Zustellung p50, p90 und Maximum unter 1 s; `wartend` zeigt 50. Gemessen im Release mit dem Lastgenerator in `e2e/messung/` (Baustein W9). Die Messung läuft nicht nachts (Nachtmodus: höchstens ein Entwickler und ein Prüfer, Lüfter).
- **Kosten des Nachholens**: Nachholen über 100.000 Nachrichten ohne Treffer unter 100 ms Server-Zeit je Aufruf (erwartet ≈ 10 ms bei 2.000 kurzen Sätzen, gerechnet aus 5 µs je Satz, dem Messwert des `security-reviewer`, und 7–13 ms bei 2.000 Sätzen zu 16 KiB, gerechnet aus dem Modell ≈ 4 µs + 3,5 µs je KiB; Nachholen auf der Nachholbank, Budget [K1] 2.000 Kosten, [K6]).
- **Sicherheit**: die Sitzung kommt aus dem Header, nie aus dem Parameter; `channel`, `seit`, `bis` werden gegen die Form geprüft (Abweisen statt Umformen), unbekannter Channel antwortet 404 wie bei den übrigen Channel-Routen; Direktgespräche sind für Dritte unsichtbar (US-F0168, auch im Nachholen); ein Aufruf schreibt während des Wartens nichts (Abrissgrenze und Keepalive je Verbindung gelten unverändert, ADR-0017). Gate `security-reviewer` (neue Route mit Eingaben, Sichtbarkeitsregel) und `observability-reviewer` (neues Health-Feld `wartend`, neue Grenz-Episode); `rechtschreib-reviewer` für die deutschen Fehlertexte.
- **Datenhaltung**: `warten` schreibt nichts (kein Schreibkonto, keine Plattenquote, kein Log je Aufruf); der Zeiger liegt beim Agenten, der Server merkt sich keinen Stand je Agent.

## Akzeptanzkriterien

Die Zahlen und Zeiten gelten für die Starts der User Stories (`R00049` in `user-stories/`); `t1…tn` sind Zeiten von Nachrichten in der Zeitfolge der Instanz.

### F0166 Warten auf Neues in Channels

- [ ] Rechenbeispiel Nachholen: `anna` sendet `t1` und `t2` in `# allgemein` und `t3` in `# technik`, `ben` danach `t4` in `# allgemein`; `claude` ruft `warten?seit=<t0>&bis=0` mit `t0` vor `t1` → 200 mit genau den vier Nachrichten in der Reihenfolge `t1, t2, t3, t4`, jede als `{channel, direkt: null, anlass: [], nachricht}` mit `channel` `allgemein` bzw. `technik`, `seit` gleich `t4`, `ende: "treffer"`; derselbe Aufruf mit `seit=<t4>` → 200 mit leerer Liste, `seit` gleich `t4`, `ende: "frist"`
- [ ] Blockieren ohne Polling: `claude` ruft `warten?seit=<t4>&bis=30`, nach 2 s sendet `anna` `t5` → die Antwort mit genau `t5` kommt binnen 1 s nach der 201 (Startpunkt: die 201), `ende: "treffer"`, `seit` gleich `t5`; es war ein Aufruf
- [ ] Ohne `seit` gilt „ab jetzt“: eine Nachricht, die vor dem Aufruf gesendet wurde, ist nicht in der Antwort; die nach dem Aufruf gesendete ist es
- [ ] `bis` fehlt → der Aufruf wartet bis zu 60 s (Probe mit `bis=3`, 3,0 bis 4,0 s nach dem Eingang, Antwort 200 leer, `ende: "frist"`); `bis=0` antwortet sofort; `bis=3600` wird angenommen
- [ ] Abweisungen ohne Nebenwirkung: `bis=3601`, `bis=-1`, `bis=1.5`, `bis=abc` → 422 `regel: "bis"`; `seit=gestern` und ein `seit` mit unkodiertem `+` → 422 `regel: "cursor"` (der Text nennt die Kodierung); `erwaehnung=1`, 65 × `channel`, zweimal `seit` → 422 `regel: "filter"`; `erwahnung` → 422 `regel: "parameter"` mit dem Namen `erwahnung`; `channel=gibtesnicht` → 404 mit `channel`; ohne Header und mit unbekanntem Token → 401
- [ ] Filter `channel`: mit `channel=technik` kommen nur Nachrichten aus `# technik`; mit `channel=technik&channel=allgemein` die aus beiden (Vereinigung); eine Nachricht aus einem dritten Channel weckt nicht
- [ ] **[A2]** Ohne Filter weckt jede neue Nachricht in jedem Channel (alle Channels der Instanz), `anlass` ist `[]`
- [ ] Eigene und gelöschte Nachrichten: `claude` sendet selbst `t6` → sein offener Aufruf kehrt nicht zurück; `anna` sendet `t7` und löscht sie, bevor `claude` zum ersten Mal ruft → `warten?seit=<t5>` liefert `t7` nicht; ein bearbeiteter Treffer kommt mit `zustand: "bearbeitet"` und dem neuen Text
- [ ] Verlustfrei in der Schleife (QW1 ohne Neustart): 3 Sender senden je in einem von 3 Channels 300 Nachrichten in zufälligem Abstand, `claude` ruft in Schleife und gibt jedes `seit` weiter → die Menge der empfangenen Zeiten ist gleich der der gesendeten (900), jede genau einmal
- [ ] Budget, Rechenbeispiel: 150 Nachrichten stehen nach `t0` → der erste Aufruf mit `seit=<t0>` liefert die ersten 100 mit `ende: "weitere"` und `seit` gleich der Zeit der hundertsten; der zweite mit diesem `seit` liefert 50 mit `ende: "treffer"`. Sind die Nachrichten so groß, dass 100 über 256 KiB kämen, wird nach Byte-Budget geschnitten (mindestens eine), ebenfalls `weitere`
- [ ] Nichts anderes weckt: mit `bis=3` und Filter `channel=allgemein` antworten die Aufrufe 3,0 bis 4,0 s nach dem Eingang leer mit `ende: "frist"`, wenn in dieser Zeit (je einzeln, an `claude` vorbei) geschieht: `anna` setzt `tippt` und erneuert es nach 2 s; ein anderer Agent setzt `denkt_nach`; `anna` bearbeitet eine ihrer Nachrichten aus `# allgemein` (ohne Erwähnung von `claude`); sie löscht eine; `ben` tritt bei und verlässt; ein Channel wird angelegt; eine Frist wird geändert; `claude` sendet selbst (die bearbeitete und die gelöschte Nachricht standen schon vor dem Aufruf im Channel)
- [ ] Halter und Gesundheit: solange der Aufruf läuft, zeigt `GET /api/teilnehmer` `claude` als `online`; `GET /api/gesundheit` nennt `wartend: 1`, unmittelbar nach dem Ende `wartend: 0`, `claude` ist wegen des Nachlaufs von 1 s noch `online` und 2 s nach dem Ende `offline`
- [ ] Zwei Aufrufe derselben Kennung bekommen beide dieselbe Nachricht (Verteilen); `warten` ändert nichts an `/api/gesundheit.schreibkonto` und nichts an den Dateien unter `channels/` (`sha256sum` vorher und nachher)

### F0167 Erwähnung weckt

- [ ] Erwähnungsregel als Tabelle (Eingabe → erwähnter Name): `"@claude hi"`, `"hi @Claude."`, `"@claude@127.0.0.1"`, `` "`@claude`" `` und `"@claude-"` → `claude`; `"@claude-entwickler"` → `claude-entwickler`; `"shoff@claude"`, `"@ claude"`, `"@claudex"` (für den Namen `claude`) und `"@alle"` → nicht `claude`; die Tabelle gilt als Unit-Probe der reinen Funktion und ebenso über die Route (je ein Senden und ein Wecken-oder-nicht)
- [ ] `codex` wartet mit `erwaehnung`; `anna` sendet `@codex bitte prüfen` in `# technik` → die Antwort kommt binnen 1 s nach der 201 mit genau dieser Nachricht und `anlass: ["erwaehnung"]`; eine Nachricht ohne `@codex` weckt `codex` nicht (`bis=3`: 3,0 bis 4,0 s, leer, `ende: "frist"`)
- [ ] Vereinigung: `codex` wartet mit `channel=technik&erwaehnung`; eine Nachricht ohne Erwähnung in `# allgemein` weckt nicht, eine mit `@codex` in `# allgemein` weckt (Anlass `erwaehnung`), jede Nachricht in `# technik` weckt (Anlass `[]` oder `erwaehnung`)
- [ ] Nachholen: `codex` ist 10 s lang nicht am Warten; in dieser Zeit sendet `anna` `@codex` (`t1`) und `@claude` (`t2`) → `codex` ruft mit `seit` vor `t1` und `erwaehnung` und erhält genau `t1`; `seit` steht danach auf einem Wert ≥ `t2`
- [ ] Eigener Name: schreibt `codex` selbst „@codex“ in eine Nachricht, weckt das den eigenen Aufruf nicht
- [ ] **[A1]** `@alle` und `@hier` wecken einen Wartenden mit `erwaehnung` nicht (`bis=3`, leer, `ende: "frist"`)
- [ ] **[A3]** `anna` sendet `t1` ohne Erwähnung und bearbeitet sie danach zu `Hallo @codex`, während `codex` mit `erwaehnung` wartet → das Warten kehrt nicht zurück (`bis=3`, leer, `ende: "frist"`)
- [ ] Beim Nachholen gilt die aktuelle Fassung: ruft `codex` danach mit einem `seit` vor `t1` und `erwaehnung`, kommt `t1` (jetzt mit `@codex`) als Treffer, weil die Fassung zählt, die der Index hat (ADR-0054 Regel 3.4; die Folge steht unter „Offene Fragen“)
- [ ] Budget der Prüfung, Rechenbeispiel **[K1]**: 5.000 Nachrichten ohne Erwähnung nach `t0`, Aufruf mit `erwaehnung` und `bis=0` → 200 leer mit `ende: "weitere"` und `seit` gleich der Zeit der 2.000. geprüften; der zweite Aufruf `weitere` bei der 4.000.; der dritte `frist` mit `seit` ≥ der Zeit der 5.000.; Grenzfall: genau 4.000 Sätze → der zweite Aufruf `frist` (Auslegung des planer, beim Bau zu bestätigen); die Folge ist lückenlos und ohne Doppel
- [ ] Nachholkosten, Rechenbeispiel: in `# allgemein` stehen 100.000 Nachrichten ohne `@claude`; `claude` ruft mit `erwaehnung`, `bis=0` und `seit` vor der ersten, fünfzigmal hintereinander mit dem jeweils zurückgegebenen `seit` → 49 Aufrufe antworten `ende: "weitere"` (je 2.000 geprüfte Sätze, K1), der fünfzigste `ende: "frist"` (kein ungeprüfter Satz bleibt) mit `seit` ≥ der Zeit der letzten Nachricht; jeder Aufruf kostet unter 100 ms Server-Zeit (erwartet ≈ 10 ms; gerechnet aus 5 µs je Satz, die Grenze ist die Zusage QW6), und ein offener `GET /api/ereignisse` liefert währenddessen jedes Tippsignal eines dritten Teilnehmers (einmal je Sekunde, `PUT …/aktivitaet`) in unter 1 s; währenddessen wird keine Nachricht gesendet, die Satzzahl bleibt genau 100.000 (mit einer weiteren Nachricht bliebe nach 50 × 2.000 ein Satz ungeprüft, und der fünfzigste Aufruf endete mit `weitere`)
- [ ] Verlust über Neustart (QW1, verkleinert): 3 Sender senden in jedem von 3 Channels je 100 Nachrichten (zusammen 900) mit `@agent` in zufälligem Abstand, `agent` ruft in Schleife mit `seit`; nach etwa 150 empfangenen wird die Instanz mit SIGTERM beendet und neu gestartet (gleicher Ordner), `agent` tritt mit seinem Sitzungs-Header wieder bei und setzt mit seinem `seit` fort; Sender wiederholen, was während des Neustarts scheitert → die Menge der empfangenen Zeiten ist gleich der Menge der Nachrichten, die mit 201 bestätigt wurden, jede genau einmal
- [ ] Zustellung unter Last (Release): 50 Agenten warten mit `erwaehnung`, 30 Nachrichten/s über 60 s, jede zehnte erwähnt einen zufälligen Agenten → Zustellung von der 201 bis zur Antwort p50, p90 und Maximum unter 1 s; `wartend` ist 50; Bericht mit den drei Werten (außerdem B0698, P10 aus R00045)

### F0168 Direktnachricht weckt

- [ ] `mia` wartet mit `direkt`; `claude` sendet ihr eine Direktnachricht (Route aus I0012) → die Antwort kommt binnen 1 s nach der 201 mit `anlass: ["direkt"]`, `direkt: "claude@127.0.0.1"`, `channel: null`; eine Channel-Nachricht in derselben Zeit weckt `mia` nicht
- [ ] Ohne Filter treffen die eigenen Direktgespräche mit (Anlass `direkt`) und auch Channel-Nachrichten (Anlass `[]`)
- [ ] Nachholen: eine Direktnachricht an `mia`, die ankam, während sie nicht wartete, kommt mit ihrem alten `seit`
- [ ] Eigene Direktnachrichten wecken den Absender nicht
- [ ] QW4: `shoff` wartet ohne Filter, `claude` sendet `mia` eine Direktnachricht → `shoff` wird nicht geweckt (`bis=3` leer), und ein Nachholen mit einem `seit` vor der Nachricht liefert sie nicht — auch nicht ihre Anhang-ID; `mia` und `claude` erhalten sie

### F0169 Warten endet mit Grund und bleibt begrenzt

- [ ] Ende der Instanz (QW5): 50 offene Aufrufe von 50 Kennungen, SIGTERM → alle 50 antworten 200 mit leerer Liste und `ende: "programmende"`, die Instanz endet mit Exit 0 binnen 1 s nach dem Signal (Startpunkt: das Signal); das Betriebsprotokoll enthält keine Zeile der Stufe EXCEPTION
- [ ] Verlassen: `claude` ruft `warten?bis=60`, `POST /api/verlassen` mit seinem Token → der offene Aufruf antwortet binnen 1 s mit 200 und `ende: "verlassen"`; der nächste Aufruf mit diesem Token ist 401
- [ ] Grenze (QW6, erster Teil): `claude` öffnet 4 Aufrufe (`bis=60`) → alle laufen; der fünfte → 429 mit `grenze: "warten"`, `hoechstens: 4`, Kopf `Retry-After: 5`; `mia` (andere Kennung, selbe Herkunft) öffnet zugleich einen → 200 läuft
- [ ] Matrix A2 des `security-reviewer` (Grenze je Kennung): Aufrufe 1 bis 4 einer Kennung → 200, der 5. → 429 `warten`, Freigabe beim Abbruch (nach einem abgebrochenen Aufruf ein neuer → 200), eine fremde Kennung ist nicht beeinflusst, und Kennungen derselben Herkunft (`claude@127.0.0.1`, `mia@127.0.0.1`) zählen getrennt
- [ ] Platz wird frei: bricht `claude` einen der 4 ab (curl beendet), nimmt ein neuer Aufruf sofort den Platz; nach 4 abgebrochenen und einem neuen Aufruf entsteht kein 429 (der Zähler wächst durch Abbrüche nicht)
- [ ] Das 429 erzeugt genau eine Grenz-Episode im Betriebsprotokoll (Katalog ADR-0042: Bereich `warten`), keine Zeile je Aufruf auf INFORMATION; im `/api/gesundheit` steht `wartend` entsprechend den laufenden Aufrufen

### F0170 Nachholen bleibt begrenzt

Gemessen wird mit dem Lastgenerator in `e2e/messung/` (Release, nicht nachts); die Unit- und Integrationsproben der Bubbles laufen im Gate. Die Zahlen sind die der ADR-0054 Ergänzung; K1–K5 siehe „Konstanten des Nachholens“.

- [ ] **Abnahme-Probe des `security-reviewer` (#17 F0166, wörtlich; heute rot):** Instanz mit 100.000 Nachrichten, 10 Channels; 16 Sitzungen × 4 Schleifen `GET /api/warten?bis=0&seit=2000-01-01T00:00:00Z&erwaehnung` (64 Verbindungen), 8 s; Beobachter mit eigener Kennung 40 × `GET /api/channels/allgemein/nachrichten?n=50`, 50 ms Abstand → Verlauf p90 < 100 ms, max < 500 ms; Schleifen oberhalb der Grenze 429 warten oder gedrosselt. Kontrolle ohne Schleifen median < 5 ms; gleiche Last als Verlauf-Schleife n=200 p90 < 100 ms. Mutationsprobe: Bank oder Grenze abschalten → rot (Bank abschalten: Verlauf p90 ≫ 100 ms; Konto je Herkunft abschalten: `nachholen.saetze` ≫ 1 Mio). Gedrosselt heißt hier: 429 `nachholen` (das 429 `warten` greift nicht, jede Sitzung hat genau 4 sequentielle Schleifen)
- [ ] **Z1** schlimmster Fall der Probe: die Schleifen ignorieren `Retry-After` und rufen sofort wieder; die 429 selbst heben den Verlauf nicht über die Grenzen der Probe
- [ ] **Z2** (folgt aus K4 und K1) nach den 8 s der Probe nennt `/api/gesundheit` `nachholen.kosten` ≤ ≈ 1,0 Mio (Rechnung: 500.000 + 8 × 60.000 + 4 × 2.000 ≈ 0,98 Mio; bei kurzen Sätzen gleich `nachholen.saetze`, bis F0171 gebaut ist gilt `saetze`) und `nachholen.gebremst` > 0; ohne das Konto je Herkunft wären es über 5 Mio
- [ ] **Z3** andere Herkunft bleibt flink: im selben Lauf wartet ein Agent von `127.0.0.2` mit `warten?erwaehnung` in Schleife, ein Sender von dort sendet 20 Nachrichten mit Erwähnung in 20 s → jede kommt weniger als 2 s nach der 201 an
- [ ] **Z4** Senden bleibt flink: im selben Lauf senden 8 Sender (eigene Kennungen, ab F0171 reihum, je 5 Sendungen) 40 Nachrichten im Abstand von 200 ms → alle 201, p90 < 100 ms (Herleitung: das Schreibkonto einer Kennung — Vorrat 30, Nachschub 1 je Sekunde, A00014 — lässt in 8 s nur 30 + 8 = 38 Sendungen durch, gemessen 37 von 40; je Kennung 5 liegen weit darunter; alternativ ein Sender im Abstand von 1 s, dann dauert die Probe 40 s)
- ~~**Z5** (Hypothese zur Kosten-Einheit „Satz“)~~ — entfällt hier: gemessen Median 111–118 ms bei 2.000 Sätzen zu 16 KiB, die Hypothese „zählt die Einheit Bytes mit“ ist eingetreten; ersetzt durch **Z5 neu** in F0171
- [ ] **[K1] Budget**: ein Channel mit 5.000 Sätzen ohne Treffer → erster Aufruf `weitere` mit `seit` gleich der Zeit des 2.000. Satzes, zweiter `weitere` bei der 4.000., dritter `frist` oder `treffer`; lückenlos, ohne Doppel (Unit, Fortsetzung von W4; dieselbe Folge wie im Kriterium „Budget der Prüfung“ in F0167, hier als Konstante mit eigenem Test `NACHHOLBUDGET.saetze == 2_000`)
- [ ] **[K2] Bank**: Plätze bei 1 Kern → 1, 2 → 1, 8 → 4, 9 → 4; hält der Test die Werkbank (ein Platz), antwortet `GET /api/warten?bis=0&seit=<alt>` trotzdem; hält der Test die Bank (ein Platz), antwortet `GET /api/channels/allgemein/nachrichten?n=50` sofort und `warten` erst nach der Freigabe
- [ ] **[K3] Konto je Kennung**: *Unit mit gestellter Uhr* (Provider X00007; `LOCALCHATA_JETZT` setzt nur den Start der Uhr, sie läuft weiter, und ist darum für „stehend“ ungeeignet): 50 × Reservierung 2.000 → alle ok; die 51. → `Gebremst{kennung}` mit `wieder_in` gleich 100 ms (50 × 2.000 = 100.000 = Vorrat; der Nachschub von 20.000 je Sekunde gibt 2.000 in 100 ms zurück). *Integration mit laufender Uhr*: schwere Aufrufe einer Kennung ohne Pause (Instanz mit > 100.000 Sätzen) → die ersten 50 sind 200, spätestens beim 80. kommt 429 `nachholen` mit `von: "kennung"`, `wieder_in_ms` größer 0 und höchstens 100 (Rechnung: ≈ 10 ms je Aufruf, Nachschub in dieser Zeit ≈ 200, Bremse bei ≈ 56; Hypothese, ungemessen — darum „spätestens“, nie „genau der 51.“)
- [ ] **[K4] Konto je Herkunft**: 16 Kennungen von `127.0.0.1` buchen zusammen 250 schwere Aufrufe (250 × 2.000 = 500.000 = Vorrat; jede Kennung bleibt unter ihrem Vorrat) → *Unit mit gestellter Uhr*: alle ok, der 251. `Gebremst{herkunft}`; *Integration mit laufender Uhr*: kein 429 `kennung`, spätestens beim 600. Aufruf 429 `nachholen` mit `von: "herkunft"` (Hypothese, ungemessen); reicht die Herkunft, aber nicht die Kennung, bleibt der Eimer der Herkunft unverändert (alles oder nichts)
- [ ] **[K5] Reservierung mit Erstattung**: 200 leichte Aufrufe in Folge (`seit` gleich der Marke, wenige neue Nachrichten) → kein einziges 429; 1.000 × Reservieren und Erstatten bei stehender Uhr gelingen alle (Unit); die Erstattung gilt für jeden Ausgang, nicht nur für einen Treffer
- [ ] **429-Antwort**: der Körper nennt `fehler`, `grenze: "nachholen"`, `von` und `wieder_in_ms`, der Kopf `Retry-After` gleich ⌈`wieder_in_ms` / 1000⌉ (mindestens 1); der Text nennt die Kennung bzw. die Herkunft („alle Teilnehmer dieser Adresse zusammen“) und sagt, dass `seit` gültig bleibt; derselbe Aufruf mit demselben `seit` nach der Wartezeit liefert dieselbe Antwort wie ohne Bremse
- [ ] **Reihenfolge und Nebenwirkungen**: die Prüfungen laufen 401 → 422 → 404 → Grenze 4 → Reservieren → Halter → Abonnieren → `H` → Bank → Nachholen → Erstatten; ein 429 `nachholen` hinterlässt keinen Halter (`wartend` bleibt 0, `/api/teilnehmer` unverändert) und liest `H` nicht
- [ ] **`Lagged` bucht nicht**: der Bus mit Kapazität 2 und 5 Nachrichten ohne Verarbeitung bei leerem Konto → alle 5 genau einmal geliefert, kein 429, `nachholen.gebremst` unverändert (das Nachholen nach `Lagged` läuft über die Bank, aber ohne Buchung)
- [ ] **Betrieb**: `/api/gesundheit` nennt `nachholen` mit `saetze` (seit dem Start tatsächlich geprüfte Sätze), `gebremst` (Zahl der 429 `nachholen`), `bank_plaetze` und `bank_belegt`; das Betriebsprotokoll enthält genau zwei Zeilen je Kennung und Episode — ein `WARNING` „… Episode beginnt“ beim ersten 429 und ein `INFORMATION` „… Episode endet: n in d s“ 60 s nach dem letzten (ADR-0042 Regel 5) — nicht je 429, ohne Nachrichtentext und ohne Token
- [ ] **Normalbetrieb ungebremst (QW6c, hängt an K3 und K4)**: 50 Agenten einer Herkunft starten nach einem Neustart neu und holen je 3.000 Sätze nach (≈ 10 Minuten bei 5 Nachrichten/s) → kein einziges 429 `nachholen`
- [ ] **Aufholen dauert, aber geht (QW6d, hängt an K3 — [P1])**: ein Agent, der einen Tag fortblieb (≈ 430.000 Sätze), holt in höchstens 20 s vollständig und ohne Doppel nach (Rechnung: Vorrat 100.000 + 20.000 je Sekunde ≈ 17 s)
- [ ] Mutanten, je einzeln rot: Nachholen auf der Werkbank; Bank und Werkbank teilen eine Semaphore; Erlaubnis zu früh frei; Erstattung fehlt; nur ein Eimer; nicht alles oder nichts; `erstatten` über den Vorrat; Herkunft gleich Kennung; Budget 10.000 oder `<` statt `≤`; Reservierung nach dem Halter; `Lagged` bucht; Erstattung nur bei Treffer; `Retry-After` fehlt; eine Zeile je 429 statt je Episode

### F0171 Nachholen zählt nach Aufwand

Unit- und Integrationsproben laufen im Gate; Z5 neu, Z6 und Z6b misst der Lastgenerator in `e2e/messung/` (Release, nicht nachts). Sätze unter 1 KiB kosten 1: alle Tests von F0167 und F0170 mit kurzen Sätzen bleiben unverändert grün. „16 KiB“ heißt hier ein Text aus Buchstaben ohne Zeilenumbruch von 16.384 Byte (Satzbytes in der Tagesdatei dann ≈ 16,4 KiB, Kosten 17).

- [ ] **[K6] Kosten eines Satzes** (Unit, rein): Satzbytes 0 / 1.023 / 1.024 / 16.384 / 32.768 → Kosten 1 / 1 / 2 / 17 / 33
- [ ] **[K1] Budget in Kosten, Rechenbeispiel**: ein Channel mit 300 Sätzen zu 16 KiB, ohne Treffer → der 1. Aufruf `weitere` nach 117 Sätzen (117 × 17 = 1.989 ≤ 2.000 < 118 × 17 = 2.006), der 2. `weitere` nach weiteren 117, der 3. `frist` nach den restlichen 66; die Folge ist lückenlos und ohne Doppel; 5.000 kurze Sätze ergeben wie in F0170 `weitere` bei 2.000 und 4.000
- [ ] **[K1] Fortschritt** (Unit): bei einem Budget von 5 wird ein Satz zu 16 KiB trotzdem geprüft — der Aufruf kommt voran, die Wiederholung endet
- [ ] **[K3, K5] Erstattung und Konto in Kosten**: Reservierung 2.000, geprüft 117 Sätze zu 16 KiB (Kosten 1.989) → Erstattung 11 (nicht 2.000 − 117); 50 solcher Aufrufe einer Kennung bei gestellter Uhr (Unit) → alle ok, der 51. `Gebremst{kennung}` (verbraucht 50 × 1.989 = 99.450, Rest 550 < 2.000; `wieder_in` = 1.450 / 20.000 s = 72,5 ms, aufgerundet 73 ms — Auslegung des planer, beim Bau zu bestätigen)
- [ ] **Betrieb**: `/api/gesundheit` nennt `nachholen` mit `saetze`, `kosten`, `gebremst`, `bank_plaetze`, `bank_belegt`; nach dem Nachholen der 300 Sätze zu 16 KiB steht `saetze` um 300 und `kosten` um 5.100 (300 × 17) höher; `kosten` ≥ `saetze`, bei kurzen Sätzen gleich
- [ ] **Z2 auf Kosten**: nach den 8 s der Abnahme-Probe aus F0170 nennt `nachholen.kosten` höchstens ≈ 1,0 Mio und `nachholen.gebremst` ist größer als 0
- [ ] **Z4 reihum**: 40 Sendungen im Abstand von 200 ms, reihum von 8 Kennungen (je 5), im selben Lauf wie die Abnahme-Probe → alle 201, p90 < 100 ms
- [ ] **Z5 neu** (nach B0742/B0743, mit Namenssuche): ein Channel mit 2.000 Sätzen zu je 16 KiB, die Erwähnung von `claude` im letzten; `claude` ruft mit `erwaehnung`, `bis=0` und dem jeweils zurückgegebenen `seit` → jeder der ≈ 18 Aufrufe bis zum Treffer braucht höchstens 20 ms Server-Zeit im Median (gerechnet 7–13 ms), die Folge ist lückenlos ohne Doppel, der letzte Aufruf liefert den Treffer; dieselbe Probe mit 2.000 Sätzen zu je ≈ 32 KiB (Zeile aus Zeilenumbrüchen) — gleiche Grenze. Vor B0742/B0743 gilt nur die Zeile „jeder Aufruf ≤ 20 ms“ als Vorlauf
- [ ] **Z6 viele Herkünfte**: wie die Abnahme-Probe aus F0170, aber 64 Herkünfte (`127.0.2.1`–`127.0.2.64`, Quelladresse gebunden wie bei Z3) mit je 1 Sitzung und je 4 Schleifen (256 gleichzeitige Nachhol-Aufrufe), `Retry-After` ignoriert, 8 s; Beobachter von `127.0.0.3` → Verlauf p90 < 100 ms, max < 500 ms
- [ ] **Z6b Variante des security-reviewer**: 6 Herkünfte × 16 Sitzungen × 4 Schleifen (384 Verbindungen), `Retry-After` beachtet → Verlauf p90 < 100 ms, max < 500 ms; gemessen mit der Mutante „Bank allein aus“: p90 867 ms, max 1.365 ms (rot)
- [ ] Mutanten, je einzeln rot: Kosten gleich 1 (Z5 neu über 20 ms, die Reihe braucht mehr als 18 Aufrufe); ⌈ statt ⌊; Budget in Sätzen; Budget prüft nach dem Satz statt davor; kein Fortschritt (Budget 5, ein Satz zu 16 KiB muss geprüft werden); Erstattung nach Sätzen statt Kosten; Health `kosten` zählt Sätze; Bank allein aus (Z6, Z6b und die Probe der Bank aus F0170 rot — bleibt sie unter Z6 und Z6b grün, entfällt Z6 als Gate und die Zusicherung steht nur auf der Probe der Bank)

### Rahmen

- [ ] `/api/gesundheit` nennt `wartend`; der Signalbus wird von `warten` nie abonniert (Probe: Mutant „Signalbus abonniert“ färbt den Test „Aktivität weckt nicht“ rot)
- [ ] Die Mutanten der ADR färben die zugehörigen Tests rot (Abonnieren nach `H`, Dublettenregel „nur nach `H`“ weg, `Lagged` als „Neuer Stand“ oder still, Zähler bei Abbruch nicht frei, eigene Nachricht weckt, Thema `aenderung` zählt, Obergrenze `3601` zugelassen, unbekannter Parameter übergangen, Grenze je Herkunft statt je Kennung; für F0170 die Mutanten des Kriteriums dort)
- [ ] Gate grün (fmt, clippy, `cargo test`, `npm --prefix e2e test`); Release-Lastprobe (F0167, letzte Bubble) einmal gefahren und im Bericht

## Betroffene Verzeichnisstruktur

Folgt ADR-0054 und dem Modell: Erwähnungsregel, Trefferregel und Nachholen über alle Gespräche in `src/gespraeche/` (Gespräche, A00006), Route, Parameter, Halter und Grenze in `src/http/` (Datei `warten.rs`, HTTP-Schnittstelle A00004; `src/http/mod.rs` für die Route, `src/http/gesundheit.rs` für `wartend`), Ablauf am Ereignisbus in `src/live/` (Live-Kanal, A00011), Ende-Signal über `src/start/ende.rs`, Episode `warten` im Katalog (`src/episodenkatalog.rs`), Lastgenerator in `e2e/messung/`; F0170 ergänzt das Nachholkonto (zweite Instanz des Eimer-Typs aus `src/schreibkonto/eimer.rs`), die Nachholbank und den Port `Nachholplatz` (löst die Abhängigkeit `live` → `http`), die Episode `nachholen` und das Health-Feld `nachholen`; F0171 ändert die Einheit in `src/gespraeche/nachholen.rs`, `src/schreibkonto/nachholkonto.rs`, `src/http/nachholplatz.rs` und `src/http/gesundheit.rs` (Feld `kosten`). Ein eigener Baustein „Warten“ entsteht nicht (er wäre ein Durchreicher). Keine Oberfläche, kein Artboard.

## Technische Überlegungen

### Ablauf (Vorplanung, Stand nach ADR-0054)

1. Parameter prüfen (vor jeder Arbeit): Form, Grenzen, unbekannte Parameter, Channel, Sitzung → 401, 404, 422.
2. Grenze je Kennung buchen (4) → 429 `warten`; **Nachholkonto reservieren (F0170) → 429 `nachholen`**; erst dann Halter der Sitzung nehmen (online).
3. Ereignisbus **abonnieren**, dann `H` = jüngste Zeit unter der Schreibsperre lesen.
4. Nachholen auf der **Nachholbank** (F0170; zuvor Werkbank): Nachrichten mit Zeit in `(seit, H]` aus allen Gesprächen, die der Filter berühren kann, in Zeitfolge zusammengeführt, mit der Trefferregel gefiltert (Budget 100 / 256 KiB / 2.000 Kosten), danach Erstattung der nicht verbrauchten Kosten.
5. Treffer → antworten; sonst auf den Bus warten (nur `Ereignis::Nachricht` mit Zeit nach max(`seit`, `H`)); jedes geprüfte Ereignis rückt den verarbeiteten Stand vor; beim ersten Treffer alles schon Empfangene mitnehmen.
6. `Lagged` → Schritte 3–4 ab dem verarbeiteten Stand; Ende bei Treffer, `bis`, Ende-Signal, eigenem `weg`, Budget.

### Grobentwurf

Siehe ADR-0054 (reine Funktion `treffer(gespraech, nachricht, wartender, filter) -> Option<Anlass>`, Zeitfolge aus ADR-0035, Antwortform E6/E7) und die Bubbles B0735–B0751 und B0755–B0759 der WBS (F0170: Ergänzung „Nachholen von `warten` …“, Bausteine N1–N5; F0171: Bausteine K1–K4 der Ergänzung „Kosten-Einheit des Nachholens“, B0760–B0763). Die Bubbles sind Vorplanung, nicht Vereinbarung: welche es am Ende werden, entscheidet der Entwickler.

## Tests

Test zuerst je Bubble; die erste Probe und die Mutanten stehen in den Bubbles der WBS (Bausteine W1–W9 der ADR; für F0170 N1–N5 der ADR-Ergänzung, für F0171 K1–K4 der Ergänzung „Kosten-Einheit des Nachholens“).

**Unit (rein)**: Erwähnungsregel (W1), Trefferregel (W2), Parameter (W3), Nachholen mit Speicherablage und Testuhr (W4), Ablauf mit pausiertem tokio und Schranken wie `src/gespraeche/schranke.rs` (W5; Schranke zwischen Abonnieren und `H`, zwischen Nachholen und Warten, Speicherablage, die das Anhängen in Channel A anhält, Bus mit Kapazität 2).
**Integration (echtes Programm)**: Route, Halter, Grenze, Ende (W6); Wecken und Nicht-Wecken (W7); Verlust über Neustart (W8).
**Release-Messung**: Last (W9) mit dem Lastgenerator; Nachholkosten über 100.000 Nachrichten; Abnahme-Probe und Zusätze Z1–Z5 des Nachholens (F0170, B0759).
**F0170**: Nachholkonto (N1, Unit mit Testuhr), Nachholbank (N2), Budget (N3, Unit), Verkabelung (N4, Integration, laufende Uhr), Last (N5, Release).
**F0171**: Kosten eines Satzes (K1, Unit, rein), Budget in Kosten (K2, Unit), Konto in Kosten mit Health (K3, Unit und Integration), Z5 neu, Z6 und Z6b (K4, Release).
**E2E (Browser)**: keine — es gibt keine Oberfläche; die Wirkung in der Oberfläche (Nachricht des Agenten erscheint live) bleibt bei I0015.

## Abhängigkeiten

- F0171 braucht F0170 (Konto, Bank, Budget); sein Baustein K4 braucht zusätzlich B0742 und B0743 (Namenssuche für Z5 neu) und damit F0167 — nur dieser eine Baustein, K1 bis K3 sind sofort baubar.
- F0170 braucht F0166 (Nachholen, Route) und B0751 (Grenze je Kennung, Matrix A2); B0757 (Budget) wird zuerst gebaut und ändert die Werte der Proben von B0737, B0743 und B0744. Schreibkonto (A00014, ADR-0036): F0170 baut eine zweite Instanz desselben Eimer-Typs, bucht aber nie am Schreibkonto.
- Abhängig von: I0015 (Nachricht über die API senden; selbst abhängig von I0014 Als Agent beitreten, I0009 Nachricht senden, I0012 Direktnachricht senden); R00011 (Senden, Verlauf, Strom), R00004 (Anwesenheit, Halter), R00003 (Beitreten, Sitzung), R00023 (Plattenquote; `warten` bucht nichts), ADR-0035 (Schreibsperre über alle Gespräche, Zeitfolge).
- F0168 braucht zusätzlich I0012 ausdrücklich (Direktgespräche veröffentlichen auf dem Bus, erkennbar mit Beteiligten; Feld `direkt` der Antwort) — I0015 hängt schon von I0012 ab, die Verknüpfung steht der Deutlichkeit halber an der Ausbaustufe.
- Nicht abhängig von I0013 (Erwähnt werden): die Erwähnungsregel baut, wer zuerst gebaut wird (I0013 oder F0167 B0741); der andere nutzt sie. Wird I0013 zuerst gebaut, entfällt B0741 (Status `bestehend`), seine Probe übernimmt I0013.
- Blockiert: F0158 (R00047 „Antwort weckt“) braucht F0167 (Trefferregel mit Anlass und Filter `erwaehnung`) und fügt den Zweig `antwort` hinzu; B0698 P10 (R00045, Kein Wecken durch Signale) wird mit B0740 fahrbar. I0019 (Claude Code einbinden) braucht I0016 für `lc warten`.
- Nachgezogen, sobald gebaut: I0022 Channel archivieren (PO Zeile 160: archivierte Channels weder nachholen noch wecken), I0026 Reaktionen (wecken nicht, Zeile 88), I0019 Textbaustein (siehe WBS-Notiz von I0019).

## Offene Fragen

Alle Fragen der ADR sind beantwortet: A1–A3 (PO 159–161). Zur Klärung für den `architekt`, nicht blockierend:

1. **Nachholen und Bearbeitung (zu A3)**: A3 sagt „eine Bearbeitung weckt nicht“. Beim **Nachholen** prüft die Trefferregel die aktuelle Fassung aus dem Index (ADR-0054 Regel 3.4), also kann eine nachträglich eingefügte Erwähnung dort als Treffer ankommen, wenn der Agent nicht wartete. Empfehlung (übernommen, als Kriterium in F0167 festgehalten): so lassen — „mindestens einmal“ und „frühere Fassungen sieht niemand“; kippt der `architekt` es auf „nur die ursprüngliche Fassung“, fiele die Zeile „Beim Nachholen gilt die aktuelle Fassung“ weg, A3 bliebe unberührt.
2. **OpenAPI-Eintrag**: ADR-0054 legt ihn in W10 (I0019). I0015 legt `/api/openapi.json` schon an; bis I0019 fehlt `warten` darin. Empfehlung: den Eintrag in B0739 (Route) mitnehmen — Entscheidung beim `architekt`; solange keine, trägt ihn I0019 nach, und dieses Dokument verlangt ihn nicht.
3. **Archivierte Channels (PO Zeile 160)**: „archivierte nie“ ist erst prüfbar, wenn I0022 gebaut ist; das Kriterium trägt I0022 (Hinweis in der WBS an I0022).
4. **Kosten des Nachholens (QW6, zweiter Teil)** steht in der ADR nicht als Baustein; B0744 misst es (Planer-Zusatz).
5. **P1 und P2 (F0170)** liegen beim `product-owner` (Aufholzeit, Agenten je Adresse); bis zur Antwort gelten die Empfehlungen K3 und K4 als Auslegung, die Konstanten sind einzeln kippbar. Kein Kriterium blockiert.
6. **Grenzfall des Budgets**: bleibt nach genau 2.000 geprüften Sätzen nichts ungeprüft, antwortet der Aufruf `frist` (nicht `weitere`) — Auslegung des planer, die ADR nennt den Fall nicht; Bestätigung beim Bau von B0757 oder beim `architekt`.

7. **Kosten-Einheit (F0171)**: K6 ist an zwei Messpunkten kalibriert (kurzer Satz, 16 KiB); die Namenssuche ist noch nicht gemessen. Bestätigt oder korrigiert (512 Byte) wird K6 mit Z5 neu nach B0742/B0743; keine Richtungsfrage.
8. **Nachholbank unter vielen Adressen (Anmerkung N1 des `security-reviewer`, Optional)**: wer viele LAN-Adressen hat, belegt die Bank dauerhaft; ein ehrlicher Agent mit `warten?bis=0` hat dann Median 209–737 ms, p90 1,3–1,5 s. Empfehlung des Reviewers: kleines Nachholen (`seit` nahe der Marke) ohne Warteschlange der Bank. Entscheidung beim `architekt`; **nicht** Teil von F0171 und kein Kriterium hier.

## Warum löst diese Anforderung das Problem?

Das Problem ist, dass ein Agent ohne Eingabe von außen nur reagieren kann, wenn er fragt — und in einer Schleife zu fragen belegt Verbindungen, kostet Werkzeugaufrufe und verpasst, was während eines Fehlers ankommt. Die Lösung: ein blockierender Aufruf mit **einem** Zeiger → die Instanz holt aus dem Verlauf nach, was seit dem Zeiger passiert ist, und wartet sonst auf den Strom → die Reihenfolge „abonnieren, Marke lesen, nachholen, filtern“ und die Zeitfolge der Schreibsperre machen den Zeiger verlustfrei → nur Nachrichten wecken, so wird der Agent nicht von Signalen, Änderungen oder Beitritten geweckt → binnen 1 s ist er bei dem, was ihn betrifft, und kann antworten.

## Notizen

- Optionen: Richtung entschieden im ADR-0054 (Gesamtschnitt V1–V4, Einzelfragen E1–E10); verworfen dort: Bus nur als Klingel mit Index-Lauf je Nachricht und Wartendem (V2, Last), Postfach je Kennung im Server (V3, Zustand verloren beim Neustart), Ereignisstrom für Agenten (V4, Erwähnungsregel läge beim Client).
- Schnitt (Begründung in der WBS-Notiz „Karte #17 Detailplanung“): nach Verhalten, nicht nach Schicht — Kern (Channels), Regel (Erwähnung), Weg (Direkt), Pfad (Ende und Grenzen), Schutz (Begrenzung der Arbeit des Nachholens, F0170 — von außen prüfbar als Verlauf unter Last und als 429, kein Qualitäts-Slice ohne Nutzen, sondern die Antwort auf einen Befund). Verworfen: ein einziger Slice mit 17 Bubbles (Arbeitspaket zu groß, ein Urteil für Kern und Nebenwege), Last und Neustart als eigener Slice (Qualitäts-Slice ohne eigenen Nutzen — die Proben stehen als Bubbles an den Slices, die sie betreffen).
- Grenzen der Aufrufer (aus ADR-0054, nicht geprobt): das Bash-Werkzeug von Claude Code bricht nach 120 s ab, deshalb Standard `bis=60`; undici wartet höchstens 300 s auf Kopfzeilen; `curl` und `requests` unbegrenzt.
- Zustellung „mindestens einmal“: Doppel entstehen nur, wenn der Agent eine Antwort verliert und mit dem alten `seit` wiederkommt.
- F0171 (Nachholen zählt nach Aufwand): ein Slice statt Kosten, Budget und Konto einzeln — jede Hälfte allein ließe das Modell widersprüchlich (Variante C der ADR, verworfen); vier Bubbles nach dem Datenfluss. Stories US-44 bis US-50, zusammen mit US-1 bis US-43 sind es 50.
- Mini-Entwickler (Vorschlag der ADR): W1 (B0741) und W3 (B0735) ja, die übrigen nein; W9 (B0746) nach W8 (B0745).
