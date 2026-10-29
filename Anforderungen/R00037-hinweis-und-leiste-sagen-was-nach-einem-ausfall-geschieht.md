---
id: R00037
status: Neu
datum: 2026-10-29
---

# R00037: Hinweis und Leiste sagen, was nach einem Ausfall geschieht

## Beschreibung

Wer einen Channel öffnet, während die Verbindung wackelt, liest zwei Meldungen: in der Mitte „⚠ Verlauf nicht geladen — …“ und oben die Verbindungsleiste. Beide sagen nach dieser Anforderung, **was von selbst geschieht** und **was der Mensch tun kann**:

- **Hinweis nach Zustand**: ist der Strom getrennt oder läuft ein Laden-Lauf, lädt der Tab von selbst — der Hinweis sagt „⚠ Verlauf nicht geladen — wird geladen, sobald die Verbindung zurück ist.“ Steht der Strom und scheitert nur der Abruf (Fehlerantwort, Zeitgrenze), hilft nur die neue Wahl des Channels — der Hinweis sagt „⚠ Verlauf nicht geladen — Channel erneut wählen, um es noch einmal zu versuchen.“ Ändert sich der Zustand, tauscht derselbe Knoten nur den Text.
- **Leiste mit Stand während des Versuchs**: solange ein Versuch des Laden- oder Nachhol-Laufs unterwegs ist (bis zu 10 s), zeigt die Leiste „⚠ Verbindung verloren · wird verbunden …“ statt des Anlasses allein; danach wieder den Countdown oder „✓ Wieder verbunden · …“. Die Höhe bleibt gleich.

Zahlt ein auf: [Vision](R00000-vision.md) — „Mensch und KI sind gleichberechtigt“ und die eine gemeinsame LAN-Instanz: wer in einer Störung liest, erfährt, ob er warten oder handeln soll, statt zu raten. Kein Widerspruch zur Vision.

Herkunft: Karte #65 — ux-reviewer #61 (O1, O2), code-reviewer #61 (A1), Entscheidungen des product-owner 2026-10-29 (`AI-Team/po-entscheidungen.md` Zeilen 114 und 115). Entschieden in [ADR-0037](../Dokumentation/ADRs/0037-zustellung-im-tab-vorlaeufig-bestaetigt-zeichnen-je-frame.md) Ergänzung „Karte #65“ (Regel 1 Option A, Regel 2 Option A, Pre-Mortem, Bausteine, Mutanten).

Weiche (`rollen/planer/befunde-einplanen.md`): **reifer** — der alte Wortlaut „… — Verbindung prüfen.“ ist wie zugesagt gebaut (R00007, R00035 7k); es fehlt kein Verhalten, und keine Zusage bricht. Die Zusagen werden ehrlicher und genauer. Deshalb Feature F0128 mit Ausbaustufe „Verlauf nach Ausfall 2“ unter I0020, kein `aus-bug`.

Slice (WBS `Dokumentation/Planung/localchata.md`, unter I0020 Nachricht senden bei Störungen und mit Komfort): F0128 Hinweis und Leiste sagen, was nach einem Ausfall geschieht.

### Ersetzt (Verweis „ersetzt durch R00037“)

R00007 und R00035 bleiben unverändert stehen; ihr Wortlaut des Hinweises gilt nicht mehr. An diesen Stellen gilt statt „Verlauf nicht geladen — Verbindung prüfen.“ der Wortlaut und die Unterscheidung dieser Anforderung:

| Stelle | alter Text | ersetzt durch R00037 |
|---|---|---|
| R00007, Funktionale Anforderungen (Zeile 51) | „Verlauf nicht geladen — Verbindung prüfen.“ | beide Fassungen nach Zustand (Funktionale Anforderungen unten) |
| R00007, Akzeptanzkriterium „Fehler“ (Zeile 139) und User Story US-13 (`user-stories/R00007.md` Zeile 127) | „Verlauf nicht geladen — Verbindung prüfen.“ (Server beendet) | „⚠ Verlauf nicht geladen — wird geladen, sobald die Verbindung zurück ist.“ (Strom getrennt) |
| R00035, Beschreibung (Zeile 13) und 7k (Zeile 87) | „⚠ Verlauf nicht geladen — Verbindung prüfen.“ | beide Fassungen nach Zustand |
| R00035, Abnahme-Probe P1 (Zeile 98) und `user-stories/R00035.md` US-1 und US-3 (Zeilen 13 und 31) | „⚠ Verlauf nicht geladen — Verbindung prüfen.“ (Strom getrennt) | „⚠ Verlauf nicht geladen — wird geladen, sobald die Verbindung zurück ist.“ |

Alle übrigen Zusagen von R00007 („eine neue Wahl versucht es erneut“, Fehlergestalt ⚠ und `kd-danger`, kein „Noch keine Nachrichten“) und von R00035 bleiben unverändert.

### Domänenbezug

Kein Glossar im Projekt (`Dokumentation/Domaenenwissen/` fehlt). Begriffe wie in R00007, R00025 und R00035: Verlauf, Channel, Strom, Nachhol-Lauf, Laden-Lauf, Verbindungsleiste, Countdown, Rückzug, Hinweis.

## Geschäftlicher Nutzen

Der Hinweis „Verbindung prüfen“ verlangt eine Handlung, die seit R00035 nicht mehr nötig ist: der Tab lädt von selbst. Der Mensch prüft eine Verbindung, die längst wieder aufgebaut wird, oder sitzt bis zu 10 s vor „⚠ Verbindung verloren“ ohne Stand und rät, ob noch etwas geschieht. Steht der Strom und scheitert nur der Abruf, wäre ein pauschales „wird geladen“ falsch — dort hilft nur die neue Wahl.

## Funktionale Anforderungen

Die Regeln sind die der ADR-0037 Ergänzung „Karte #65“ und gelten als Teil dieser Anforderung:

- **Regel 1 — Hinweistext je Zustand**: maßgeblich ist, ob etwas **von selbst** lädt. *Selbstversuchend* ⇔ der Strom ist getrennt **oder** ein Laden-Lauf läuft.
  - selbstversuchend → „⚠ Verlauf nicht geladen — wird geladen, sobald die Verbindung zurück ist.“
  - sonst (Strom steht, Abruf mit Fehlerantwort oder Zeitgrenze, kein Lauf) → „⚠ Verlauf nicht geladen — Channel erneut wählen, um es noch einmal zu versuchen.“ (wahr nach R00007: eine neue Wahl versucht es erneut).
  - Gestalt unverändert: ein Knoten, `role="alert"`, ⚠ und Fehlerklasse, „kein zweiter Alarm“. Ändert sich der Zustand, während der Hinweis steht, wird **nur der Text desselben Knotens** getauscht, wenn er sich unterscheidet — kein neuer Knoten, kein Neuaufbau der Mitte. Der Wechsel „getrennt → Laden-Lauf läuft“ ändert den Text nicht.
- **Regel 2 — Stand der Leiste während eines Versuchs**: solange ein Versuch des Laden- oder des Nachhol-Laufs unterwegs ist (von seinem Beginn bis zu seinem Ergebnis), zeigt die Leiste „⚠ Verbindung verloren · wird verbunden …“; danach wieder Countdown („· neuer Versuch in n s“) oder „✓ Wieder verbunden · …“. Auch der **erste** Versuch zählt, der sofort mit `wiederverbunden` beginnt. Nur bei stehendem Strom: ist der Strom getrennt, gilt sein Zeitpunkt, und die Trennung beendet jeden Versuch. Zwischen den Versuchen gilt unverändert „⚠ Verbindung verloren“ mit Countdown (ADR-0037 7h).
- **Höhe (R00025 V6)**: die Leiste wechselt zwischen Anlass allein, Anlass + „neuer Versuch in n s“ und Anlass + „wird verbunden …“ ohne Sprung (± 1 px).
- **Ende des Versuchs** wird an einer Stelle gemeldet, die jeder Ausgang durchläuft (`geladen`, `nachgeholt`, `fehler`, `abgebrochen`, Trennung, Channelwechsel, Ende des Laufs): genau einmal `versuch(false)` je `versuch(true)`.

## Nicht-funktionale Anforderungen

- Keine Änderung an Routen, Antworten, Health-Check oder Protokoll; kein Server-Code. Rückzugstabelle, Zeitgrenze 10 s und Reihenfolge der Meldungen aus ADR-0037 7c/7h bleiben.
- Ansage: ein Textwechsel im Hinweis sagt nichts Neues an, wenn der Text gleich bleibt (gleicher Text → kein Schreibzugriff, Node-Test); der Sekundentakt der Leiste schreibt den Hinweis nie neu.
- Zeit (Startpunkt der Uhr: Beginn des Versuchs, das ist der Augenblick, in dem der gehaltene Abruf den Browser verlässt): „wird verbunden …“ steht spätestens 1 s nach dem Beginn (obere Grenze, wie die Zustellung) und noch 5 s nach dem Beginn; der Versuch endet an der Zeitgrenze von 10 s.
- Konsole: keine unbehandelte Ausnahme, kein CSP-Verstoß; „Failed to load resource“ nur zu den Antworten, die der Fall selbst vorschreibt — dem gehaltenen und abgebrochenen Abruf, der 500 der Probe, dem getrennten Strom und der 401 zum Wiederbeitritt nach einem Neustart.

## Akzeptanzkriterien

Die Abnahme-Probe steht wörtlich aus der ADR-0037 Ergänzung „Karte #65“ (E2E, Playwright, echtes Programm auf festem Port mit `freierPort()`; jeder Fall einmal mit und einmal ohne SharedWorker, „ohne“ wie `e2e/tests/r00004-ein-strom-je-browser.spec.js:31`):

### Hinweis und Leiste sagen, was geschieht (E2E, Ausnahmeweg)
- [ ] (a) Der Strom von A ist getrennt (Instanz beendet), der erste Abruf des Verlaufs ist gescheitert → die Mitte zeigt „⚠ Verlauf nicht geladen — wird geladen, sobald die Verbindung zurück ist.“
- [ ] (b) Der Strom steht, der Abruf des Verlaufs antwortet 500 (`page.route`, kein Lauf) → die Mitte zeigt „⚠ Verlauf nicht geladen — Channel erneut wählen, um es noch einmal zu versuchen.“
- [ ] (c) Ein Lade- oder Nachhol-Versuch wird 11 s gehalten (`page.route`) → die Leiste zeigt während des Versuchs „⚠ Verbindung verloren · wird verbunden …“, danach (die Zeitgrenze bricht bei 10 s ab) den Countdown „· neuer Versuch in n s“ bzw. nach Erfolg „✓ Wieder verbunden · …“; die Höhe der Leiste ist in allen drei Zuständen gleich (± 1 px) bei 320, 375 und 414 px Breite (R00025 V6).
- [ ] (d) Die Mutanten „alter Wortlaut“ und „kein Stand während des Versuchs“ sind rot.
- [ ] (e) R00007, R00025 und R00035 bleiben grün (soweit nicht der alte Wortlaut des Hinweises geprüft wurde, der nun der aus Regel 1 ist).

### Zustandswechsel bei stehendem Hinweis (E2E und Node)
- [ ] Steht der Hinweis aus (b) (Strom steht, 500), und wird danach die Instanz beendet, zeigt derselbe Knoten (`role="alert"`, kein neuer Knoten) den Text aus (a). Node: `hinweisNachziehen()` tauscht den Text, wenn er sich unterscheidet; gleicher Text → keine Änderung. Mutante „Hinweis nicht nachgezogen“ → rot.

### Erste Tests (Node, aus der ADR wörtlich)
- [ ] `e2e/einheit/verbindungsleiste.test.mjs`: `warnungText(null, jetzt, true)` → „⚠ Verbindung verloren · wird verbunden …“; `(jetzt + 7000, jetzt, true)` → derselbe Text (Vorrang); `(jetzt + 7000, jetzt, false)` → „⚠ Verbindung verloren · neuer Versuch in 7 s“. Mutante „`versuchLaeuft` ignoriert“ → rot.
- [ ] `e2e/einheit/nachhollauf.test.mjs`: Stand `{ geladen: false, gescheitert: true }`, `wiederverbunden` → `versuch(true)` sofort; `laden` liefert `fehler` → `versuch(false)` vor `gescheitert`; zweiter Versuch bei 1.000 → `versuch(true)`, `geladen` → `versuch(false)`, `fertig`. Dasselbe für die Art `nachholen`. `abgebrochen`, Trennung, Channelwechsel → `versuch(false)` genau einmal. Mutante „kein `versuch(false)` bei `abgebrochen`“ → rot.
- [ ] **Wächter (N1)**, `e2e/einheit/nachhollauf.test.mjs`: Antworten `['fehler', hängend, 'geladen']`, getrennt, `wiederverbunden`, `laufenBis(1200)`, `jetztVersuchen()`, auflösen, `laufenBis(5000)` → Versuche `[0, 1000]`, genau ein `fertig {um:1200, art:'geladen', mitLeiste:true}`; Mutante „Wächter `dieser.neuversuch = null` entfernt“ → rot.
- [ ] `e2e/einheit/live-verlauf.test.mjs`: `nichtGeladenText(true)` und `nichtGeladenText(false)` wörtlich wie Regel 1; bei stehendem Hinweis tauscht `hinweisNachziehen()` den Text, derselbe Knoten, `role="alert"` bleibt; gleicher Text → keine Änderung. Mutante „alter Wortlaut“ → rot.

### Mutanten, die rot werden müssen (alle sechs aus der ADR)
- [ ] „alter Wortlaut“ (Hinweis sagt wieder „Verbindung prüfen“) — Node `live-verlauf`, E2E (a) und (b).
- [ ] „kein Stand während des Versuchs“ (`versuchLaeuft` ignoriert) — Node `verbindungsleiste`, E2E (c).
- [ ] „Hinweis ohne Unterscheidung“ (immer der Text für den Strom getrennt) — E2E (b).
- [ ] „Hinweis nicht nachgezogen“ (kein `hinweisNachziehen`) — Node `live-verlauf` und der Zustandswechsel oben.
- [ ] „Ende des Versuchs nicht gemeldet bei `abgebrochen`“ — Node `nachhollauf`.
- [ ] N1 „Wächter `neuversuch = null` entfernt“ — Node `nachhollauf` (Wächter-Test oben).

### Keine Rückschritte
- [ ] Alle Tests aller Ebenen grün. Die Tests, die den alten Wortlaut festschreiben, sind auf den Wortlaut dieser Anforderung geändert: `e2e/tests/r00007-verlauf.spec.js:192` (Strom getrennt → „wird geladen …“) und `:235` (Strom steht → „Channel erneut wählen …“), `e2e/tests/r00025-verbindung.spec.js:387` und `:394`, `e2e/tests/r00035-verlauf-laden.spec.js:47` und `:189` (je nach Zustand des Stroms im Test).

## Betroffene Verzeichnisstruktur

- Web (Baustein A00005 Weboberfläche, kein neuer Baustein): geändert `web/nachhollauf.js`, `web/verbindungsleiste.js`, `web/verlauf.js`, `web/hauptansicht.js`; kein Server, keine API.
- Tests: `e2e/einheit/nachhollauf.test.mjs`, `e2e/einheit/verbindungsleiste.test.mjs`, `e2e/einheit/live-verlauf.test.mjs`, `e2e/einheit/hauptansicht.test.mjs`; E2E neu `e2e/tests/r00037-hinweis-und-leiste.spec.js`, geändert `r00007-verlauf.spec.js`, `r00025-verbindung.spec.js`, `r00035-verlauf-laden.spec.js`.

## Technische Überlegungen

### Ablauf
1. **Versuch beginnt** (Laden-Lauf oder Nachhol-Lauf, auch `jetztVersuchen` und der erste mit `wiederverbunden`)
   - 1.1 `versuch(true)` → `anzeige.versuchLaeuft = true` → Leiste „… · wird verbunden …“ (nur bei stehendem Strom)
   - 1.2 `hinweisNachziehen()` → Text des Hinweises nach Regel 1
2. **Versuch endet** (`geladen`, `nachgeholt`, `fehler`, `abgebrochen`, Trennung, Channelwechsel, Ende des Laufs): `versuchEnde` → `versuch(false)` genau einmal
   - 2.1 `fehler` → Countdown („· neuer Versuch in n s“)
   - 2.2 `geladen` / `nachgeholt` → „✓ Wieder verbunden · …“
3. **Strom trennt oder kommt zurück** → `melden` → `hinweisNachziehen()` (nur Text, nur bei Unterschied)

### Grobentwurf (Vorplanung)

Die Bubbles der WBS sind Vorplanung; welche es am Ende werden, entscheidet der Entwickler. Die vier Bausteine folgen ADR-0037 Karte #65:

- F0128: B0572 Nachhol-Lauf meldet jeden Versuch (`web/nachhollauf.js`, Rückruf `versuch(laeuft)`, Test für den Wächter N1) · B0573 Leiste zeigt den Stand während eines Versuchs (`web/verbindungsleiste.js`, `versuchLaeuft`) · B0574 Hinweis sagt, was geschieht (`web/verlauf.js`, `nichtGeladenText`, `selbstversuchend()`, `hinweisNachziehen()`) · B0575 Hauptansicht verdrahtet Hinweis und Stand (`web/hauptansicht.js`) · B0576 Abnahme-Probe (E2E)
- Reihenfolge: B0572, B0573 und B0574 berühren verschiedene Dateien; B0575 braucht alle drei, B0576 braucht B0575. Die Tests mit altem Wortlaut gehören in die Bubbles: Zustand „Strom steht“ (`r00007:235`, `r00035:47`/`:189`) in B0574, Zustand „Strom getrennt“ (`r00007:192`, `r00025:387`/`:394`) in B0575 — vor B0575 steht überall der Standardtext „Channel erneut wählen …“, deshalb dürfen B0574 und B0575 in einem Zug gebaut werden.

### Änderungen an bestehenden Klassen
Keine Klassen; Funktionen in `web/nachhollauf.js` (Rückruf `versuch`, `versuchEnde`), `web/verbindungsleiste.js` (`warnungText`/`leisteInhalt` kennen `versuchLaeuft`), `web/verlauf.js` (`nichtGeladenText`, `selbstversuchend`, `hinweisNachziehen`), `web/hauptansicht.js` (Verdrahtung).

## Tests

Nach Test-Pyramide: Unit (Node, gestellte Zeit, reine Funktionen und Fakes, jeder Ausgang des Versuchs), E2E (Playwright, Abnahme-Probe oben); kein Integrationstest am Server, weil dieser unverändert bleibt. Test zuerst je Bubble. Stil: Skill `javascript-stil` vor dem Schreiben. Fakes antworten verzögert (`test-pyramide`, UI-Tests mit verzögert antwortendem Fake).

## Abhängigkeiten

- Abhängig von: F0126 Verlauf lädt nach dem Wiederverbinden (R00035, grün), F0084 Eine Verbindungswarnung mit Countdown (R00025, grün).
- Blockiert: nichts.
- Berührt dieselbe Datei wie: `web/verlauf.js` — F0089 (rot, R00026); Reihenfolge entscheidet der Director.

## Offene Fragen

Keine an den Betreiber. Entschieden vom planer:

- **Weiche reifer statt aus-bug**: der alte Wortlaut hält, was R00007 und R00035 zusagen; das Ziel der Entscheidungen 114 und 115 ist Ehrlichkeit, kein gebrochenes Verhalten. Verworfen: falsch gebaut.
- **Option D bleibt außerhalb** (eigener Neuversuch bei stehendem Strom, R00035 „Offene Fragen“): bis dahin gilt der Text „Channel erneut wählen …“; wird D gebaut, ersetzt es diesen Fall.
- **Abnahme-Probe als eigene Bubble** (B0576) neben den vier Bausteinen der ADR, wie bei F0126 und F0127.
- **Art des Versuchs in (c)**: die ADR sagt „Lade- oder Nachhol-Versuch“; der E2E fährt einen der beiden (der Laden-Lauf nach (a) liegt nahe), die andere Art deckt der Node-Test der ADR („Dasselbe für die Art `nachholen`“).

## Warum löst diese Anforderung das Problem? (Pflicht)

Auslöser: Entwicklertest und Reviews #61 — der Hinweis „Verbindung prüfen“ verlangt eine Handlung, die seit R00035 nicht mehr nötig ist (ux-reviewer O1), und die Leiste zeigt bis zu 10 s nur „⚠ Verbindung verloren“, während ein Versuch läuft (ux-reviewer O2).

- Der Hinweistext folgt dem Zustand des Stroms und des Laden-Laufs. Deshalb liest der Mensch „wird geladen“, wenn der Tab von selbst lädt, und „Channel erneut wählen“ nur dort, wo ihm tatsächlich nur das hilft.
- Ein ausdrückliches `versuch(laeuft)` meldet Beginn und jedes Ende eines Versuchs, unabhängig von der Uhr. Deshalb zeigt die Leiste „wird verbunden …“ genau so lange, wie ein Versuch läuft, und bleibt nach keinem Ausgang hängen.
- Der Hinweis wird nach jeder Zustandsänderung nachgezogen, nur im Text und nur bei Unterschied. Deshalb steht nach einer Trennung nie „Channel erneut wählen“ bei einem Tab, der von selbst lädt, und der Bildschirmleser hört nichts doppelt.
- Sechs Mutanten halten Wortlaut, Stand, Unterscheidung, Nachziehen, Ende und Wächter fest. Deshalb fällt ein Rückfall in „Verbindung prüfen“ oder eine Leiste ohne Stand im Test auf.

## Missing-Docs

- Glossar fehlt (`Dokumentation/Domaenenwissen/` existiert nicht).

## Notizen

- Optionen (ADR-0037 Karte #65, nicht wiederholt): Regel 1 Option A (Zustand des Stroms **und** des Laden-Laufs) gewählt, B (nur Strom), C (pauschaler Text), D (so lassen) verworfen; Regel 2 Option A (ausdrückliche Größe `versuchLaeuft`) gewählt, B (Zeitpunkt = jetzt setzen), C (so lassen) verworfen.
- In Kauf genommen (ADR): ein Rückruf mehr zwischen Nachhol-Lauf, Hauptansicht und Verlauf; „wird verbunden …“ steht auch bei einem Versuch, der nur einen Abruf bedeutet (kein Verbindungsaufbau) — wie beim Countdown, der 0 erreicht.
- Pre-Mortem (ADR): (1) ein Ende des Versuchs wird nicht gemeldet → Test auf jedem Ausgang, ein `versuchEnde`; (2) Hinweis nicht nachgezogen → `hinweisNachziehen()` nach jedem `melden` und bei `versuch`; (3) erneute Ansage bei jedem Sekundentakt → nur bei Unterschied.
- Priorität: Mittel (Klasse Befund aus dem Review, kein Datenverlust). Betroffene Nutzer: jeder, dessen Tab während eines Ausfalls einen Channel öffnet.
- Umfang (Zählung der WBS, Aufwand Hypothese): 1 Slice, 5 Bubbles, 8,4–16,4 h (0,4 + 4 × 2–4); nicht bis Bubble geplant: nichts. Die Probe im Entwicklertest: (a) und (c) lassen sich von Hand herstellen (Instanz beenden), (b) mit einem Debug-Build und `LOCALCHATA_TESTPANIK=verlauf` (lässt den nächsten Verlaufsblock einmal in Panik geraten; die Antwort 500 ist aus der Beschreibung des Testknopfs gelesen, nicht ausgeführt), sonst nur über die Abnahme-Probe.
