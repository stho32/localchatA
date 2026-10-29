---
id: R00036
status: Neu
datum: 2026-10-29
---

# R00036: Tabs auf dem Beitreten-Bildschirm folgen dem Beitritt eines Nachbar-Tabs

## Beschreibung

Steht ein Tab auf dem Beitreten-Bildschirm (Erstbeitritt, „Name vergeben“ oder „ungültig“) und tritt ein anderer Tab desselben Browsers bei, folgt er automatisch: binnen 1 s zeigt er Kopfzeile, Kennung und Teilnehmerliste des Nachbarn, mit und ohne SharedWorker, ohne eigenen `POST /api/beitreten` und mit derselben Kennung wie nach einem Neuladen. Was im Namensfeld getippt war, entfällt; eine wartende ungesendete Nachricht steht danach wie in R00025 US-11 fehlgeschlagen mit „Erneut senden“ und „Verwerfen“, gesendet wird nichts von selbst.

Zahlt ein auf: [Vision](R00000-vision.md) — „Mensch und KI sind gleichberechtigt“ und die eine gemeinsame LAN-Instanz: ein Mensch ist ein Browser ist eine Sitzung (ADR-0012 Option B); wer in einem Tab beitritt, hat damit für alle Tabs seines Browsers beigetreten. Kein Widerspruch zur Vision.

Herkunft: Karte #62 — Ausbaustufe von I0004 Namen wählen und beitreten, aus dem Entwicklertest #58, Anmerkung A2; Entscheidung des product-owner 2026-10-29 (`AI-Team/po-entscheidungen.md` Zeile 110, Option 2). Entschieden und gebaut-vorgeplant in [ADR-0012](../Dokumentation/ADRs/0012-identitaet-und-sitzung.md) Ergänzung „Karte #62“ (Regeln 1–8, alle Fragen Option A).

Weiche (`rollen/planer/befunde-einplanen.md`): **reifer** — R00003 sagt „alle Tabs eines Browsers teilen die Sitzung“ (Neuladen, neue Tabs); der Fall „Tab steht schon auf dem Bildschirm“ ist dieselbe Zusage mit offenem Bildschirm, kein neues Verhalten an der Systemgrenze. Deshalb Feature F0127 mit Ausbaustufe unter I0004, kein `aus-bug`. R00003 und R00025 bleiben unverändert.

Slice (WBS `Dokumentation/Planung/localchata.md`, unter I0004 Namen wählen und beitreten): F0127 Tabs auf dem Beitreten-Bildschirm folgen dem Beitritt eines Nachbar-Tabs.

### Domänenbezug

Kein Glossar im Projekt (`Dokumentation/Domaenenwissen/` fehlt). Begriffe wie in R00003, R00004 und R00025: Teilnehmer, Kennung, Herkunft, Sitzung, Token, Beitreten-Bildschirm, Hauptansicht, SharedWorker, Zustellung, ungesendete Nachricht.

## Geschäftlicher Nutzen

Wer in einem Browser mehrere Tabs offen hat, muss nach dem Beitritt in einem Tab nicht in jedem weiteren Tab erneut beitreten oder neu laden. Heute bleibt ein Tab auf dem Bildschirm stehen und fordert einen zweiten Beitritt, obwohl der Browser längst Teilnehmer ist; tritt der Mensch dort mit einem anderen Namen bei, entstehen zwei Teilnehmer für einen Browser.

## Funktionale Anforderungen

Die Regeln sind die der ADR-0012 Ergänzung „Karte #62“ und gelten als Teil dieser Anforderung:

- **Signal (Regel 1)**: ein `storage`-Ereignis mit `key === 'localchatA.sitzung'` und nicht leerem `newValue`, das nicht das `abgelehntesToken` des Tabs ist (das Token, dessen Ablehnung ihn auf den Bildschirm brachte; sonst `null`). Ein leerer Wert und das eigene abgelehnte Token werden ignoriert. Gilt mit und ohne SharedWorker; der Verteiler bleibt unverändert.
- **Lebensdauer des Lauschers (Regel 2)**: angemeldet, wenn der Bildschirm gezeichnet wird; abgemeldet, bevor er neu gezeichnet wird oder die Hauptansicht kommt; höchstens ein Lauscher je Tab. Das 2-s-Warten beim Laden ohne Web Locks bleibt ein eigener Einmal-Lauscher.
- **Nachlese (Regel 3)**: beim Zeichnen in den Zuständen `beitreten`, `vergeben`, `ungueltig` gilt ein schon im Speicher liegendes Token, das nicht das `abgelehntesToken` ist, als Signal. In den Zuständen `serverWeg` und `fehler` keine Nachlese.
- **Folgen (Regel 4)**: Formular sofort gesperrt (`disabled`, `aria-busy`), `GET /api/ich` mit dem Token aus dem Signal. 200 → Kopfzeile ohne Fokus auf die Kennung, der Strom beginnt mit genau diesem Token. 401, `serverWeg` oder andere Antwort → Bildschirm bleibt mit Zustand und Eingabe, Formular wieder frei, **nichts wird aus dem Speicher gelöscht** (das Token gehört dem Nachbarn). Folgen schreibt nie in den Speicher.
- **Ein Lauf zugleich (Regel 5)**: kommt ein Signal während eines Folgens, wird es gemerkt; scheitert der Lauf, wird das zuletzt gemerkte Token geprüft, wenn es ein anderes ist. Läuft der eigene Beitritt per Klick, wird kein Folgen gestartet.
- **Eigener Klick (Regel 6)**: unter der Sitzungssperre (`navigator.locks`, Name `localchatA.sitzung`; ohne Web Locks ohne Sperre) zuerst nachsehen — liegt ein Token im Speicher, das nicht das `abgelehntesToken` ist, wird gefolgt wie in Regel 4 und **kein** `POST /api/beitreten` gesendet; sonst beitreten wie bisher. Endet der eigene `POST` nicht mit 200/201, vor dem Zeigen der Meldung nachsehen; liegt ein Nachbar-Token vor, wird gefolgt.
- **Namensfeld und Zustellung (Regel 7)**: Getipptes entfällt; Zustellung und Entwurf gehen unverändert in die Hauptansicht wie nach eigenem Beitritt (R00025 US-11); nichts wird automatisch gesendet.
- **Hauptansicht unverändert (Regel 8)**: ein Tab in der Hauptansicht folgt keinem `storage`-Ereignis; dort gilt der Wiederbeitritt aus ADR-0012 Ergänzung Karte #36.

## Nicht-funktionale Anforderungen

- Zeit: „binnen 1 s“ zählt ab der Antwort 201 auf den Beitritt in Tab A (Startpunkt der Uhr). Herleitung: erwartet ≈ 0,1–0,2 s (`storage`-Ereignis in wenigen ms, `GET /api/ich` im LAN, Zeichnen); die Grenze 1 s liegt weit über dem Erwartungswert und ist die obere Grenze der Zustellung, keine Regel.
- Keine Änderung an Routen, Antworten, Health-Check oder Protokoll; kein Server-Code.
- Konsole: keine unbehandelte Ausnahme und kein CSP-Verstoß; Konsolenzeilen „Failed to load resource“ sind nur zu Antworten erlaubt, die der Fall selbst vorschreibt (im Szenario „Name vergeben“ die 409 des gescheiterten Wiederbeitritts und die 401 zuvor), und werden ab dem Beitritt in Tab A gezählt.

## Akzeptanzkriterien

Die Abnahme-Probe steht wörtlich aus der ADR-0012 Ergänzung „Karte #62“ (E2E, Playwright, echtes Programm auf freiem Port; Läufe: SharedWorker vorhanden · SharedWorker per `addInitScript` entfernt — je mit 2 und mit 3 Tabs eines Browser-Kontexts; zusätzlich ein Lauf ohne Web Locks wie `e2e/tests/r00003-tabs.spec.js` B1; `POST /api/beitreten` gezählt über `context.on('request')`):

### Erstbeitritt
- [ ] Alle Tabs stehen auf dem Beitreten-Bildschirm (kein Token, kein Name). Tab A tritt als `shoff` bei → binnen 1 s zeigen alle Tabs Kopfzeile, Kennung `shoff@127.0.0.1` und die Teilnehmerliste; genau **ein** `POST /api/beitreten` im Kontext; `shoff` steht in jeder Liste genau einmal; ein weiterer Beitritt als `shoff` über die API von einer anderen Herkunft (`127.0.0.2`) → 409 (einmal belegt). In Tab B war `abc` im Namensfeld getippt → nach dem Folgen kein Namensfeld mehr, keine Konsolenfehler (Auslegung siehe Nicht-funktionale Anforderungen).

### Name vergeben (wie R00025 US-11)
- [ ] Tabs A und B als `shoff` in der Hauptansicht, Neustart des Programms, danach per API von derselben Herkunft `shoff` belegt; in B Entwurf `noch offen`, B sendet `fünf` → Wiederbeitritt scheitert mit 409 → beide Tabs auf dem Bildschirm „Name vergeben“, B zeigt „1 ungesendete Nachricht wartet auf den Beitritt — Neuladen verwirft sie.“. Tab A tritt als `shoff2` bei → binnen 1 s zeigt B Kopfzeile und Kennung `shoff2@127.0.0.1`; `fünf` steht in `# allgemein` fehlgeschlagen mit „Nicht gesendet — der Beitritt war abgelaufen.“, „Erneut senden“ und „Verwerfen“; seit dem Klick in A genau **ein** `POST /api/beitreten` und kein `POST …/nachrichten`; im Feld von B steht `noch offen`. „Erneut senden“ in B → genau ein Satz unter `shoff2`.

### Klick im Wettlauf (mit Web Locks)
- [ ] A und B stehen auf dem Bildschirm, beide klicken im selben Task „Beitreten“ (A `shoff`, B `bert`) → beide zeigen dieselbe Kennung, genau ein `POST /api/beitreten`, eine Sitzung am Server.

### Mutanten, die rot werden müssen
- [ ] Lauscher nicht angemeldet → B bleibt auf dem Bildschirm.
- [ ] Folgen per `beitreten` statt `ich` → zwei POSTs.
- [ ] Folgen löscht bei 401 das Token → ein dritter Tab, der danach lädt, tritt neu bei (zweiter POST).

### Erste Tests (Node, `e2e/einheit/sitzung.test.mjs`, gestellter Speicher und gestellte API)
- [ ] `nachbarFolgen(speicher, api, abgelehntesToken)`: kein Token oder das abgelehnte → `null`, keine Anfrage; `ich` 200 → `kopfzeile` mit diesem Token; 401 und `serverWeg` → `null`, `removeItem` nie gerufen.
- [ ] `vomBildschirmBeitreten(name, speicher, api, abgelehntesToken)`: Nachbar-Token im Speicher → kein `api.beitreten`, Ergebnis `kopfzeile`; kein Token → genau ein `api.beitreten`; `beitreten` 422 und danach Nachbar-Token im Speicher → `kopfzeile` statt `ungueltig`.

### Keine Rückschritte
- [ ] Alle Tests aller Ebenen grün; `e2e/tests/r00003-tabs.spec.js`, `r00003-beitreten.spec.js` und die Wiederbeitritt-Spezifikationen zu R00025 bleiben unverändert grün.

## Betroffene Verzeichnisstruktur

- Web: geändert `web/sitzung.js`, `web/beitreten.js`, `web/start.js` (Baustein A00005 Weboberfläche); kein Server, keine API.
- Tests: `e2e/einheit/sitzung.test.mjs`, `e2e/einheit/beitreten.test.mjs`, neu `e2e/tests/r00003-tabs-folgen.spec.js`.

## Technische Überlegungen

### Ablauf
1. **Bildschirm wird gezeichnet** (`web/start.js` `zeigen`)
   - 1.1 Lauscher auf `storage` anmelden (vorher den alten abmelden)
   - 1.2 in `beitreten`, `vergeben`, `ungueltig`: Nachlese des Speichers
2. **Signal** → `nachbarFolgen(speicher, api, abgelehntesToken)`; Formular sperren
   - 2.1 `GET /api/ich` 200 → Kopfzeile mit Zustellung und Entwurf
   - 2.2 sonst Formular frei, Speicher unberührt
3. **Klick „Beitreten“** unter der Sitzungssperre → `vomBildschirmBeitreten(...)`; Nachbar-Token im Speicher → wie 2, sonst `POST`

### Grobentwurf (Vorplanung)

Die Bubbles der WBS sind Vorplanung; welche es am Ende werden, entscheidet der Entwickler. Die vier Bausteine folgen ADR-0012 Karte #62:

- F0127: B0568 `nachbarFolgen` und `vomBildschirmBeitreten` als reine Funktionen (`web/sitzung.js`) · B0569 `zeichnen` gibt `{ sperren, freigeben }` zurück (`web/beitreten.js`) · B0570 Lauscher, Nachlese, ein Lauf zugleich, Klick unter der Sitzungssperre (`web/start.js`) · B0571 Abnahme-Probe (`e2e/tests/r00003-tabs-folgen.spec.js`)

### Änderungen an bestehenden Klassen
Keine Klassen; Funktionen in `web/sitzung.js`, `web/beitreten.js`, `web/start.js` (ES-Module ohne Bundler).

## Tests

Nach Test-Pyramide: Unit (Node, reine Funktionen, gestellter Speicher und gestellte API), E2E (Playwright, Abnahme-Probe oben); kein Integrationstest am Server, weil dieser unverändert bleibt. Stil: Skill `javascript-stil` vor dem Schreiben.

## Abhängigkeiten

- Abhängig von: F0010 Beitreten-Bildschirm (grün), F0088 Wiederbeitritt beim Senden (R00025, grün).
- Blockiert: nichts.
- Berührt dieselbe Datei wie: `web/start.js` — Karte #61 (F0126, R00035) berührt laut WBS andere Dateien (nicht gegengeprüft); Reihenfolge entscheidet der Director.

## Offene Fragen

Keine an den Betreiber. Entschieden vom planer:

- **Weiche reifer statt aus-bug**: R00003 sagt zu, dass Neuladen und neue Tabs derselbe Teilnehmer sind; der Fall „Tab steht schon auf dem Bildschirm“ ist dort nicht ausgeschlossen, aber nicht zugesagt. Verworfen: falsch gebaut — der Befund verlangt eine Reife der Zusage, keine Reparatur.
- **Konsolenkriterium**: die ADR sagt „keine Konsolenfehler“ ohne Allquantor-Ausnahmen; ausgelegt nach `AI-Team/planer.md` Gelerntes (5), siehe Nicht-funktionale Anforderungen. Weicht der Auftrag davon ab, bitte nennen.

## Warum löst diese Anforderung das Problem? (Pflicht)

Auslöser: Entwicklertest #58, Anmerkung A2 — zwei Tabs eines Browsers stehen auf dem Beitreten-Bildschirm, der Mensch tritt im ersten bei, der zweite bleibt stehen.

- Jeder Beitritt merkt das Token im `localStorage`, deshalb erreicht das `storage`-Ereignis jeden anderen Tab desselben Browsers, mit und ohne SharedWorker.
- Der wartende Tab prüft das Token mit `GET /api/ich` und übernimmt die Kennung, deshalb zeigt er dieselbe Kopfzeile wie nach einem Neuladen und ruft nie selbst `beitreten`.
- Der eigene Klick sieht unter der Sitzungssperre zuerst im Speicher nach, deshalb entsteht bei gleichzeitigen Klicks mit Web Locks genau eine Sitzung, und der Name bleibt einmal belegt.
- Wartende Nachrichten laufen über den schon gebauten Weg der Zustellung, deshalb geht keine verloren und keine wird ungefragt gesendet.

## Missing-Docs

- Glossar fehlt (`Dokumentation/Domaenenwissen/` existiert nicht).

## Notizen

- Optionen: je Frage 3–4 Varianten mit Tradeoffs in der ADR-0012 Ergänzung „Karte #62“ (Signal: `storage` gewählt; Verteiler-Meldung, `BroadcastChannel` und Abfrage verworfen; Prüfung per `GET /api/ich`; Server-seitiger Abweis, Sperrmarke und automatisches Senden verworfen); hier nicht wiederholt.
- In Kauf genommen (ADR): kein Folgen, wenn der Browser nichts speichern lässt (wie Neuladen); zwei Sitzungen bei gleichzeitigen Klicks ohne Web Locks (die zweite endet nach der Gnadenfrist); ein halb getippter Name geht verloren.
- Umfang (Zählung der WBS, Hypothese für den Aufwand): 1 Slice, 4 Bubbles, Aufwand 6,4–10,4 (2 + 0,4 + 2–4 + 2–4); nicht bis Bubble geplant: nichts.
