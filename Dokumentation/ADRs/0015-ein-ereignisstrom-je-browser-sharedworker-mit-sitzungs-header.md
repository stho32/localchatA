# ADR-0015: Ein Ereignisstrom je Browser — SharedWorker mit `fetch`-Strom und Sitzungs-Header

**Status**: Akzeptiert
**Datum**: 2026-10-28
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #4 I0005)
**Kontext-Anforderung**: — (WBS I0005; prägt I0006, I0009, I0012, I0013)
**Erprobt durch**: zwei Wegwerf-Proben im Scratchpad, danach gelöscht. (1) Chromium (Playwright aus `e2e/`), Seite mit dem CSP-Satz aus ADR-0014, HTTP/1.1, **ein `fetch`-Strom je Tab** mit Header `X-LocalchatA-Sitzung`: Header kommt an, Ereignisse werden gelesen, keine Konsolenfehler — aber mit 6 offenen Tabs lädt der 7. Tab nicht mehr (Zeitüberschreitung) und jedes `fetch` in den übrigen Tabs hängt: die 6 Verbindungen je Host teilt sich der ganze Browser. (2) Dieselbe Seite unter einem **unsicheren Ursprung** (Nicht-Loopback-Adresse, `isSecureContext` falsch, keine Web Locks), **ein `SharedWorker`** (`type: 'module'`) hält einen `fetch`-Strom mit Header und verteilt die Ereignisse an alle Tabs: 8 Tabs → 1 Strom am Server, ein `fetch` aus Tab 8 antwortet sofort; 7 Tabs geschlossen → der Strom bleibt; letzter Tab geschlossen → der Strom ist binnen 1,5 s am Server zu; keine Konsolenfehler (CSP lässt den Worker über `script-src 'self'` zu).

Ergänzt [ADR-0007](0007-live-kanal-sse-fuer-browser-long-poll-fuer-agenten.md) für den Browser-Teil (dort: `EventSource`) und beantwortet die in [ADR-0012](0012-identitaet-und-sitzung.md) offen gelassene Frage, wie sich der Strom ausweist.

## Kontext

- Vision „Wer da ist, ist sichtbar“ und „Live überall“; Qualitätsziele Q3 (Live < 1 s) und Q4 (Tab zu → offline < 2 s) in [A00001](../Architektur/A00001-localchata.md).
- ADR-0007 (Vorgeschlagen): Browser über SSE, ein offener Strom ist ein **Halter** der Anwesenheit. ADR-0012: eine Sitzung je Browser, Token im `localStorage`, Nachweis nur über den Header `X-LocalchatA-Sitzung`; Cookie verworfen (Cookie-Tossing über andere Programme auf `127.0.0.1`, CSRF). `EventSource` kann keine eigenen Header senden.
- ADR-0010: nur HTTP/1.1, keine Upgrades; ein Browser hält höchstens 6 Verbindungen je Host — die Probe zeigt, dass alle Tabs des Profils sie sich teilen.
- ADR-0014: `Referrer-Policy: no-referrer`, CSP mit `connect-src 'self'`, `script-src 'self'`; Web Locks gibt es nur in sicheren Kontexten — über die LAN-Adresse, den Normalfall der Vision, nicht (`web/start.js`).
- Das Programm schreibt heute keine Zugriffszeilen (stderr nur für Start und Fehler, [X00001](../Architektur/Aspekte/X00001-logging-und-diagnose.md)).

## Optionen

### Option A: `EventSource` je Tab, Token als Query-Parameter (`/api/ereignisse?sitzung=<token>`)
- Vorteile: kürzester Code; `EventSource` verbindet selbst neu; Vorbild kanbanD.
- Nachteile: Token in der URL (Werkzeuge, Fehlermeldungen, künftige Zugriffszeilen, Entwicklerwerkzeuge des Browsers zeigen es; OWASP rät ab); zweiter Nachweisweg an der Grenze neben dem Header; ein Strom je Tab — ab 6 Tabs steht der Browser für diese Instanz (Probe 1).

### Option B: `fetch`-Strom je Tab mit Header, eigener kleiner SSE-Leser
- Vorteile: ein einziger Nachweisweg (Extraktor `Teilnehmer` unverändert); Token nie in einer URL; Leser und Wiederverbinden sind reine Funktionen, in Node testbar.
- Nachteile: Wiederverbinden selbst gebaut (der Wächter für stille Abrisse ist ohnehin eigener Code, kanbanD `web/verbindung.js`); ein Strom je Tab — dieselbe 6-Tab-Grenze wie A (Probe 1).

### Option C: Ein Strom je Browser — ein `SharedWorker` hält den `fetch`-Strom mit Header und verteilt die Ereignisse über `MessagePort` an alle Tabs; ohne `SharedWorker` Rückfall auf B
- Vorteile: ein Halter je Sitzung, wie ADR-0012 sie meint (ein Mensch = ein Browser); Tabs belegen keine Verbindung dauerhaft — die 6er-Grenze und die 64 Plätze je Gegenstelle bleiben frei; funktioniert im unsicheren Kontext; der Strom endet von selbst, wenn der letzte Tab geht (Probe 2); Nachweis wie B.
- Nachteile: ein zweiter Ausführungsort im Browser (Worker), schwerer zu beobachten (eigene Entwicklerwerkzeuge); ein neuer Tab bekommt den letzten Stand nur, wenn der Worker ihn hält; nach einem Programm-Update kann ein alter Worker weiterlaufen; Browser ohne `SharedWorker` (Chrome auf Android) fallen auf B mit der 6-Tab-Grenze zurück.

### Option D: Ein Strom je Browser über einen gewählten Leit-Tab, Verteilung per `BroadcastChannel`
- Vorteile: kein Worker; jeder Tab ist gleich gebaut.
- Nachteile: die Wahl des Leit-Tabs braucht ohne Web Locks (unsicherer Kontext) eine eigene Absprache über `localStorage`-Ereignisse — mit Wettläufen; schließt der Leit-Tab, reißt der Strom ab, bis ein anderer übernimmt (Anwesenheit flackert).

### Option E: Kurzlebiges Einmal-Ticket (`POST /api/strom-ticket` mit Header → `EventSource` mit `?ticket=`)
- Vorteile: `EventSource` bleibt; das Token selbst steht nie in der URL.
- Nachteile: eine zusätzliche Route mit Wirkung; nach jedem Abriss braucht `EventSource` ein neues Ticket, verbindet also doch nicht selbst neu; ein Strom je Tab wie A.

| # | Nachweis | Verbindungen je Browser | Testbarkeit | Code im Browser | Schwäche |
|---|---|---|---|---|---|
| A | Query (Token in URL) | 1 je Tab | gut | klein | Token in URL, 6-Tab-Grenze |
| B | Header | 1 je Tab | sehr gut (rein) | mittel | 6-Tab-Grenze |
| C | Header | 1 | gut (rein + Playwright) | mittel | Worker-Lebenszyklus, Rückfall ohne `SharedWorker` |
| D | Header | 1 | mittel | groß | Wahl ohne Sperre, Übergabe-Lücke |
| E | Ticket | 1 je Tab | mittel | mittel | Zusatzroute, 6-Tab-Grenze |

## Entscheidung

**Gewählt: Option C**

Es überwiegt, dass nur ein Strom je Browser die Instanz mit vielen Tabs bedienbar hält und zugleich genau einen Halter je Sitzung ergibt; der Nachweis bleibt der eine Header aller Routen. In Kauf genommen: ein Worker als zweiter Ausführungsort mit eigenem Lebenszyklus, und der Rückfall auf einen Strom je Tab in Browsern ohne `SharedWorker`.

Regeln:

1. **Server**: `GET /api/ereignisse` verlangt die Sitzung über den Header `X-LocalchatA-Sitzung` (Extraktor `Teilnehmer`, ADR-0012); es gibt **keinen** Query-Parameter für das Token, auch nicht als Ausnahme. Antwort `text/event-stream`, Format nach ADR-0007 (`data: {art, …}`, `event: herzschlag` alle 15 s, Testknopf `LOCALCHATA_HERZSCHLAG_MS`). Agenten dürfen denselben Strom mit `curl -N -H 'X-LocalchatA-Sitzung: …'` lesen.
2. **Ein Strom je Browser**: `web/strom.js` ist ein Modul-`SharedWorker`; jeder Tab verbindet sich mit ihm, gibt ihm das Token und erhält alle Ereignisse. Der Worker hält höchstens einen Strom, merkt sich den jüngsten Stand der Teilnehmerliste und gibt ihn jedem neuen Tab sofort (ADR-0016).
3. **Version im Worker-Namen**: der Worker wird mit der Programmversion im Namen oder in der URL angelegt (`/web/strom.js?v=<version>`), damit nach einem Update kein alter Worker mit altem Protokoll weiterläuft.
4. **Wiederverbinden im Worker**: Wächter wie kanbanD `web/verbindung.js` (40 s ohne Byte → neu verbinden; Rückzug 1 s, 2 s, 5 s, dann 10 s). Antwortet der Strom 401, meldet der Worker das allen Tabs; der Tab, der es zuerst sieht, tritt nach ADR-0012 („Browser“) still wieder bei und gibt dem Worker das neue Token.
5. **Rückfall**: fehlt `SharedWorker`, öffnet jeder Tab denselben Strom selbst (Option B, gleicher Leser-Code). Die 6-Tab-Grenze gilt dann und ist hingenommen.
6. **Leser rein**: SSE-Zerlegung (Teilstücke, `\r\n`, Kommentarzeilen, mehrere Ereignisse je Stück) und Wächter-Entscheidung sind reine Funktionen, unter `e2e/einheit/` mit `node --test` geprüft.

## Konsequenzen

**Positiv:**
- Viele Tabs eines Menschen kosten eine Verbindung; die Instanz bleibt in jedem Tab bedienbar (Probe 2: `fetch` aus Tab 8 sofort beantwortet).
- Ein Browser = eine Sitzung = höchstens ein Halter; „online“ heißt: dieser Browser hat localchatA offen.
- Das Token verlässt nie den Header; die Grenze kennt genau einen Nachweisweg.

**Negativ / Inkauf zu nehmen:**
- Der Worker ist eigener Code mit eigenem Lebenszyklus (Start beim ersten Tab, Ende mit dem letzten); Fehler darin sieht man nur in den Worker-Werkzeugen des Browsers — darum meldet der Worker Fehler an die Tabs, die sie in der Konsole zeigen.
- Lädt der einzige Tab neu, endet der Worker und mit ihm der Strom; ein neuer entsteht nach dem Laden. Die Anwesenheit glättet diese Lücke (Nachlauf, ADR-0016).
- Browser ohne `SharedWorker` haben je Tab einen Strom und ab 6 Tabs eine hängende Instanz.

**Pre-Mortem (ein Monat nach dem Start gescheitert, weil …):**
1. … nach einem Update ein alter Worker mit altem Ereignisformat weiterläuft und neue Tabs nichts mehr zeigen. Früh-Indikator: Fehler „unbekannte Ereignisart“ in der Konsole nach Programmwechsel. Gegenmaßnahme: Regel 3 (Version im Worker-Namen); Durchsetzung über einen Integrationstest, dass die ausgelieferte Seite die Worker-URL mit der Programmversion nennt.
2. … ein neuer Tab eine leere Teilnehmerliste zeigt, weil der Strom schon lief, als er dazukam. Früh-Indikator: Liste erst nach der nächsten Änderung gefüllt. Gegenmaßnahme: Regel 2 (Worker gibt den letzten Stand weiter), E2E: zweiter Tab sieht die Liste ohne neue Änderung.
3. … ein Mensch nach dem Schließen aller Tabs „online“ bleibt, weil der Browser den Worker länger am Leben hält. Früh-Indikator: E2E-Messung Tab zu → offline über 2 s. Gegenmaßnahme: erster E2E-Test misst genau das (Q4); Probe 2 zeigte unter 1,5 s.

**Folgeentscheidungen / Aufgaben:**
- [ ] I0005: `GET /api/ereignisse` mit Header, Herzschlag, Ende-Signal (siehe ADR-0016 und ADR-0010); `web/strom.js` als Worker mit Rückfall; reiner Leser und Wächter mit Node-Tests.
- [ ] Erster Test (Integration, echtes Programm): Strom ohne Header → 401; mit `?sitzung=<gültiges token>` ohne Header → 401; mit Header → 200 `text/event-stream`, erstes Ereignis ist der Stand der Teilnehmerliste.
- [ ] Erster E2E-Test: ein Browser-Kontext mit 7 Tabs → alle laden, alle zeigen die Liste, am Server genau ein offener Strom dieser Sitzung (Gesundheit oder Testknopf); keine Konsolenfehler.

## Quellen / Bezuege

- [A00005 Weboberfläche](../Architektur/A00005-weboberflaeche.md) (primär), [A00004 HTTP-Schnittstelle](../Architektur/A00004-http-schnittstelle.md), [A00011 Live-Kanal](../Architektur/A00011-live-kanal.md), [A00010 Anwesenheit](../Architektur/A00010-anwesenheit.md); Aspekte [X00003](../Architektur/Aspekte/X00003-authentifizierung-und-sitzung.md), [X00009](../Architektur/Aspekte/X00009-testbarkeit.md)
- ADR-0007, ADR-0010, ADR-0012, ADR-0014, ADR-0016
- kanbanD `web/verbindung.js` (Wächter, Rückzug; Q04, nur gelesen); `web/start.js` (Web Locks nur im sicheren Kontext)
- OWASP Session Management Cheat Sheet (Sitzungskennung nicht in der URL); WHATWG HTML, Abschnitt „Shared workers“; Fetch Standard (Antwortkörper als `ReadableStream`)

## Ergänzungen

- **2026-10-28 (Karte #5 I0006):** Regel 1 und 4 — Herzschlag 5 s statt 15 s, Wächter 15 s statt 40 s ([ADR-0017](0017-stiller-abriss-kernel-erkennt-tote-verbindungen.md)); Rückzug 1 s, 2 s, 5 s, 10 s unverändert. Regel 2 — der Worker reicht neben den Datenereignissen den Herzschlag (`jetzt`) weiter und gibt jedem neuen Tab zum letzten Stand den aktuellen Uhrenversatz mit ([ADR-0019](0019-restzeit-weg-um-in-serverzeit-und-statusformen.md), Regel 4). Nach dem eigenen `weg` (`POST /api/verlassen`) verbindet der Worker nicht neu ([ADR-0018](0018-verlassen-sofort-weg-nie-beim-schliessen.md), Regel 4 und 6).
- **2026-10-29 (Karte #36, R00025 F0084, B0397; WBS Offene Fragen #22 Punkt 3) — Zeitpunkt des nächsten Versuchs im Zustand:** Ergänzt Regel 2 (Worker → Tab) und Regel 4 (Rückzug); ersetzt wird nichts. Anlass: die Warnung zählt bis zum nächsten Versuch des Stroms herunter (Designsystem `VerlaufsMarken` „Countdown bis zum nächsten Versuch“), der Rückzug läuft aber im Halter (`web/live.js` `laufen`, im `SharedWorker` oder im Rückfall im Tab); der Tab kennt den Zeitpunkt nicht.
  - Option A: **absoluter Zeitpunkt** `naechsterVersuchUm` (ms seit der Epoche, `Date.now()` des Halters) im Zustand `unterbrochen` (**gewählt**). Stärke: ein Wert, der für jeden Empfänger gleich richtig ist — auch für einen Tab, der erst während der Wartezeit dazukommt und den Zustand vom Verteiler nachgereicht bekommt; derselbe Feldtyp trägt den Nachhol-Versuch des Tabs ([ADR-0037](0037-zustellung-im-tab-vorlaeufig-bestaetigt-zeichnen-je-frame.md), Ergänzung Karte #22/#36, 7c), die Anzeige wählt nur die Quelle. Schwäche: Halter und Tab müssen dieselbe Wanduhr lesen — beide lesen `Date.now()` desselben Browsers auf demselben Rechner; wird die Systemuhr verstellt, irrt die Anzeige eines einzelnen Versuchs (begrenzt, s. Regel 4 unten), nie der Versuch selbst.
  - Option B: relative Wartezeit `wartenMs` im Zustand. Stärke: unabhängig von der Wanduhr. Schwäche: falsch für jeden, der sie später liest — der Verteiler müsste beim Nachreichen an einen neuen Tab die verstrichene Zeit abziehen, also doch eine Uhr führen; die Laufzeit der Nachricht geht unbemerkt ein.
  - Option C: der Tab rechnet den Rückzug selbst aus der Zahl der `unterbrochen`-Meldungen. Stärke: kein neues Feld. Schwäche: eine zweite Kopie der Regel, die schon heute abweicht (`fehlversuche = 1` nach einem gelesenen Strom, eigener Zähler für Ablehnungen); ein später dazugekommener Tab kennt die Zahl nicht.
  - Option D: der Worker schickt je Sekunde den Rest. Stärke: der Tab rechnet nichts. Schwäche: eine Nachricht je Sekunde und Tab für eine Anzeige; die Anzeige hängt an der Zustellung über den Port; verpasst ein Tab einen Takt, steht die Zahl.
  - **Gewählt: A.** In Kauf genommen: eine verstellte Systemuhr verfälscht die Anzeige für höchstens einen Versuch.
  - **Regeln**:
    1. **Halter** (`halten` in `web/live.js`): meldet `unterbrochen` zusammen mit `naechsterVersuchUm = jetzt() + Wartezeit des kommenden Schlafs` — dieselbe Wartezeit, mit der er danach schläft (eine Rechnung, zwei Verwendungen). `verbunden` und `abgelehnt` tragen keinen Zeitpunkt. Einen Zustand „Versuch läuft“ gibt es nicht: ist der Zeitpunkt erreicht und kein neues `unterbrochen` gekommen, läuft der Versuch; scheitert er, kommt ein neues `unterbrochen` mit dem nächsten Zeitpunkt.
    2. **Verteiler** (`web/verteiler.js`): die Zustandsnachricht wird `{ art: 'zustand', zustand, beitreten, token, naechsterVersuchUm }`; er merkt sich den Zeitpunkt mit dem Zustand und gibt ihn einem neuen Tab mit, wie den Versatz. Der Rückfall ohne `SharedWorker` liefert dasselbe Feld über `beiZustand`.
    3. **Uhr**: `Date.now()` auf beiden Seiten. `performance.now()` ist ausgeschlossen — Worker und Tab haben je einen eigenen Zeitursprung.
    4. **Anzeige im Tab**: Rest = `naechsterVersuchUm − Date.now()`, begrenzt auf 0 bis 10.000 ms (der längste Rückzug), in ganzen Sekunden aufgerundet (B0396); 0 heißt „verbinde …“. Weitergezählt wird im vorhandenen Sekundentakt des Tabs (`web/sekundentakt.js`), nur solange die Warnung steht — kein eigener Zeitgeber je Anzeige.
    5. **Welcher Zeitpunkt gilt**: Strom `unterbrochen` → der des Stroms; Strom verbunden, Nachholen gescheitert → der des nächsten Nachhol-Versuchs (Tab); Netz des Tabs aus (`navigator.onLine === false`), Strom noch ohne `unterbrochen` → Warnung ohne Countdown „⚠ Verbindung verloren“ (R00025 nennt diesen Fall nicht; Folge für den `planer`).
  - **Erste Tests** (Node, rein, gestellte Uhr und gestellter Schlaf): `halten` mit `holen`, das immer scheitert, `jetzt` bei 0 → `unterbrochen` mit `naechsterVersuchUm` 1.000, dann 2.000 nach dem zweiten Versuch (relativ zu dessen Ende), 5.000, 10.000, 10.000; nach einem gelesenen Strom wieder 1.000 — jeweils gleich der Zeit, die `schlafen` bekommt. Verteiler: Zustand `unterbrochen` mit Zeitpunkt 7.000, danach verbindet ein neuer Tab → er bekommt denselben Zeitpunkt. Mutante „Wartezeit statt Zeitpunkt weitergereicht“ → rot (der neue Tab sähe die volle Wartezeit).
