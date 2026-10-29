---
id: R00035
status: Neu
datum: 2026-10-29
ursprung: Bug-Report
ursprungslauf: R00025
---

# R00035: Behebung von „Verlauf bleibt nach Ausfall beim ersten Laden leer“

## Beschreibung

Fällt das erste `GET /api/channels/<c>/nachrichten` eines frisch geöffneten Tabs in einen Ausfall, zeigt der Tab „⚠ Verlauf nicht geladen — Verbindung prüfen.“ (R00007) und oben „⚠ Verbindung verloren“. Kommt die Instanz zurück, verschwindet die Leiste und die Teilnehmerliste zeigt ●, aber der Verlauf bleibt leer, und neue Nachrichten — auch die eigenen gesendeten — erscheinen in diesem Tab nie. Mit und ohne SharedWorker. Erwartet (R00025, F0047): nach dem Wiederverbinden steht der Verlauf da, und „✓ Wieder verbunden“ steht nie über einem leeren Verlauf. Danach gilt:

- **Der Nachhol-Lauf bekommt eine zweite Art „laden“**: ist der offene Channel bei der Rückkehr nie geladen (erster Abruf gescheitert **oder** noch unterwegs), öffnet der Lauf ihn vollständig neu — wie eine Wahl des Channels —, mit Zeitgrenze 10 s, Rückzug und Leiste wie beim Nachholen (ADR-0037 7a–7e).
- **Meldung erst nach dem Verlauf**: bei Erfolg ersetzt der Verlauf den Hinweis „Verlauf nicht geladen“, und die Leiste meldet 4 s „✓ Wieder verbunden · Verlauf geladen“; angesagt wird derselbe Text ohne „✓“.
- **Scheitert das Laden erneut**, bleibt der Hinweis in der Mitte, die Leiste „⚠ Verbindung verloren“ mit Countdown, und die Warteschlange ruht (eigene Nachrichten bleiben „wird gesendet …“, gehen nicht verloren).

Zahlt ein auf: [Vision](R00000-vision.md) — „eine gemeinsame LAN-Instanz“ und „Mensch und KI sind gleichberechtigt“: wer einen Tab öffnet, während die Instanz neu startet, liest danach den Channel und kann in ihm mitschreiben, ohne neu zu laden. Kein Widerspruch zur Vision.

Herkunft: Karte #61 — Vorrang-Befund aus dem Entwicklertest #58 (2026-10-29), Anmerkung A1; schon vor #58 so. Ursache und Entscheidung: [ADR-0037](../Dokumentation/ADRs/0037-zustellung-im-tab-vorlaeufig-bestaetigt-zeichnen-je-frame.md) Ergänzung „Karte #61“ (Regeln 7g–7k, Option A).

Weiche (`rollen/planer/befunde-einplanen.md`): **falsch gebaut** — R00025/R00012 F0047 sagen zu, dass nach dem Wiederverbinden die Nachrichten des offenen Channels dastehen und „✓ Wieder verbunden“ nur bei stimmendem Stand erscheint; für den nie geladenen Channel hält der Slice das nicht (R00007 verlangt bei der Wahl nur „eine neue Wahl versucht es erneut“ — das bleibt unverändert). Deshalb `aus-bug` und ein Feature F0126 unter I0020 als Slice der Behebung.

Slice (WBS `Dokumentation/Planung/localchata.md`, unter I0020 Nachricht senden bei Störungen und mit Komfort): F0126 Verlauf lädt nach dem Wiederverbinden.

### Domänenbezug

Kein Glossar im Projekt (`Dokumentation/Domaenenwissen/` fehlt). Begriffe wie in R00007, R00012 und R00025: Verlauf, Channel, Nachhol-Lauf, Verbindungsleiste (Warnung), Countdown, Rückzug, Puffer, SharedWorker, Strom.

## Ursachenanalyse

### Root Cause

Herkunft `code`, gelesen, nicht ausgeführt (architekt, ADR-0037 Karte #61):

- Nach dem Fehlschlag des ersten Abrufs gilt `gescheitert = true`, `geladen = false` (`web/verlauf.js:239`–`242`).
- `wiederverbunden` findet den Channel dann nicht nachholbar (`nachholbar` verlangt `geladen`, `web/nachhollauf.js:73`) und meldet sofort `fertig({ art: 'nachgeholt', zahl: 0 })` (`web/nachhollauf.js:99`–`101`): „✓ Wieder verbunden“, die Warteschlange läuft, aber nichts lädt.
- Jede live angekommene Nachricht — fremd oder eigen bestätigt — landet im Puffer vor dem ersten Block (`web/verlauf.js:278`–`280`), der nie kommt; gezeichnet wird ohnehin nicht (`!gescheitert`, `web/verlauf.js:323`).
- Neu geöffnet wird nur durch eine neue Wahl desselben Channels (`web/verlauf.js:628`, `web/hauptansicht.js:366`).
- Ausgeschlossen: der Weg über den SharedWorker — der Fehler liegt im Tab (mit und ohne SharedWorker gleich).
- Latent: wird die Rückkehr gemeldet, solange der erste Abruf noch unterwegs ist (er hat beim ersten Laden keine Zeitgrenze), gilt `gescheitert` noch nicht und nichts geschieht (ADR-0037 7g).

### Betroffene Komponenten

`web/verlauf.js`, `web/nachhollauf.js`, `web/hauptansicht.js`, `web/verbindungsleiste.js` (alle im Baustein A00005 Weboberfläche); kein Server, keine API.

### Testlücke

Die Tests zu F0047 und F0084 decken das Nachholen eines **geladenen** Channels und die Leiste; kein Test öffnet einen Tab, dessen erster Abruf in den Ausfall fällt, und lässt die Instanz zurückkehren. Der E2E von R00007 („Fehler“) endet beim Hinweis „Verlauf nicht geladen“ und prüft nie die Rückkehr.

## Lösungsvorschlag

### Langfristige Lösung

Option A (ADR-0037 Karte #61, gewählt): **Laden-Lauf** im Nachhol-Lauf. `stand()` meldet zusätzlich `gescheitert`; `wiederverbunden` startet für einen offenen Channel mit `geladen = false` einen Lauf der Art `laden` mit sofortigem ersten Versuch; ein Versuch ist `laden(channel, signal)` → `verlauf.oeffnen(channel)` mit Ergebnis `{ art: 'geladen' | 'fehler' | 'abgebrochen' }`, Zeitgrenze 10 s. Regeln 7g–7k der ADR sind Teil dieser Anforderung (Auslöser, Versuch, Ende und Wahl, Puffer und Abgleich, Hinweis und Ansage).

### Alternative Ansätze

- B: einmal neu öffnen beim Wiederverbinden — kein Neuversuch, wenn auch dieser Abruf scheitert; „✓ Wieder verbunden“ über leerem Verlauf; Warteschlange läuft vor dem Verlauf.
- C: Live-Ereignisse ohne Verlauf zeichnen — ohne Block kein Cursor, Fenster und Anker (ADR-0026) bekämen einen Zustand „Fenster ohne Block“.
- D: eigener Neuversuch des ersten Ladens unabhängig vom Strom — ein zweiter Neuversuchsweg neben dem Nachhol-Lauf; **nicht Teil dieser Anforderung** (siehe Offene Fragen).

## Test-Strategie

### Unit Test zur Bug-Reproduktion

Node `e2e/einheit/nachhollauf.test.mjs` (gestellte Zeit): Stand `{ channel: 'a', geladen: false, gescheitert: true }`; `wiederverbunden` → `laden('a')` sofort, kein `abrufen`, kein `fertig`. Heute rot: der Lauf meldet sofort `fertig({ art: 'nachgeholt', zahl: 0 })`.

### Code-Extraktion für isoliertes Testing

Keine nötig: der Nachhol-Lauf nimmt `laden`, Zeitgeber und Uhr schon als Abhängigkeiten; `laden` ist neu und im Test ein Fake (verzögert antwortend, nach `test-pyramide`).

### Regressionstests

- Node `nachhollauf`: „unterwegs“, „abgebrochen beendet“, Rückzug mit Channelwechsel und `jetztVersuchen`, `NeuerStand` (Zahlen siehe Akzeptanzkriterien).
- Node `verbindungsleiste`: Text für `{ art: 'geladen' }`.
- E2E Abnahme-Probe P1, zweimal (mit und ohne SharedWorker).
- Bestehende Tests zu R00007, R00012 (F0047) und R00025 bleiben grün.

## Funktionale Anforderungen

- **7g Auslöser**: `stand()` meldet `gescheitert`. Bei `wiederverbunden` gilt ein offener Channel mit `geladen = false` (gescheitert **oder** erster Abruf noch unterwegs) als „laden“: Lauf der Art `laden`, sofortiger erster Versuch; ein noch laufender erster Abruf wird abgebrochen. `NeuerStand` bei verbundenem Strom startet einen Laden-Lauf nur bei `gescheitert`, ohne Leiste und ohne Ruhen der Warteschlange. Ein Laden-Lauf braucht keinen Cursor.
- **7h Versuch**: `laden(channel, signal)` → `verlauf.oeffnen(channel)` mit `{ art }`; Zeitgrenze 10 s. `fehler` (Netz, Zeitgrenze, Status außer 200): keine Meldung, Leiste „⚠ Verbindung verloren“ mit Countdown, Warteschlange ruht, Rückzug wie das Nachholen. `geladen`: `fertig({ art: 'geladen' }, mitLeiste)`, dann läuft die Warteschlange. `abgebrochen` (ein anderes `oeffnen` hat übernommen): Lauf endet ohne Neuversuch und ohne Meldung.
- **7i Ende und Wahl**: Trennung und Channelwechsel beenden den Laden-Lauf; nach dem nächsten Wiederverbinden gilt 7g erneut. Eine neue Wahl desselben Channels während eines Laden-Laufs zieht dessen nächsten Versuch vor (`jetztVersuchen`), ohne zweites `oeffnen`; ohne Laden-Lauf bleibt es bei `erneutVersuchen` (R00007).
- **7j Puffer und Abgleich**: scheitert das Laden, wird der Puffer verworfen, und bis zum nächsten `oeffnen` wird nichts mehr gepuffert; das Laden ist ein Neu-Öffnen, jede Zeit steht genau einmal da (eigene bestätigte Nachrichten eingeschlossen); vorläufige und fehlgeschlagene eigene bleiben in der Zustellung und stehen nach dem Laden unter dem Verlauf.
- **7k Hinweis und Ansage**: während der Störung bleibt „⚠ Verlauf nicht geladen — Verbindung prüfen.“ (`role="alert"`) in der Mitte und oben „⚠ Verbindung verloren“ mit Countdown, auch über Fehlschläge des Laden-Laufs hinweg; bei Erfolg ersetzt der Verlauf (oder „Noch keine Nachrichten in # …“) den Hinweis, die Leiste meldet 4 s „✓ Wieder verbunden · Verlauf geladen“, angesagt wird derselbe Text ohne „✓“ (`ansagen({ art: 'wiederverbunden' })`).

## Nicht-funktionale Anforderungen

- Keine Änderung an Routen, Antworten, Health-Check oder Protokoll; kein Server-Code.
- Zeitgrenze des Ladens 10 s wie ADR-0037 7a; Rückzug wie 7c (erster Versuch ≤ 1 s nach der Rückkehr).

## Akzeptanzkriterien

- [ ] Der Reproduktionstest („Stand gescheitert, `wiederverbunden` → `laden('a')` sofort“) ist ohne Fix rot und mit Fix grün
- [ ] **Abnahme-Probe P1, einmal mit und einmal ohne SharedWorker** (wörtlich aus ADR-0037 Karte #61; „ohne“ wie `e2e/tests/r00004-ein-strom-je-browser.spec.js:31`, `delete window.SharedWorker` im Init-Skript): Erstes Laden in einer Störung (E2E, Ausnahmeweg, zwei Kontexte, Instanz auf festem Port): `# allgemein` enthält 3 Nachrichten (über die API angelegt). A ist beigetreten; der Abruf `GET /api/channels/allgemein/nachrichten` von A wird beim Öffnen 2,5 s gehalten (`page.route`), 0,5 s nach seinem Beginn wird die Instanz beendet; der gehaltene Abruf scheitert, A zeigt „⚠ Verlauf nicht geladen — Verbindung prüfen.“ und „⚠ Verbindung verloren“. Dann wird die Instanz neu gestartet (Route aufgehoben). → Binnen **5 s** ab dem Neustart zeigt A die 3 Nachrichten je genau einmal, keinen Hinweis „Verlauf nicht geladen“ mehr, und die Leiste „✓ Wieder verbunden · Verlauf geladen“ erscheint erst, wenn die Nachrichten sichtbar sind. Danach sendet B (anderer Kontext) eine Nachricht → sie erscheint in A binnen **1 s** (Z1) genau einmal; dann sendet A eine Nachricht → sie steht in A genau einmal bestätigt (kein „wird gesendet …“ übrig). Konsole: erlaubt sind „Failed to load resource“ zum gehaltenen Abruf, zum getrennten Strom und 401 zum Wiederbeitritt nach dem Neustart, sonst nichts. Mutante „`wiederverbunden` meldet ohne Laden fertig“ → rot (heute: der Verlauf bleibt leer, die Nachricht von B erscheint nie). Die Grenze 5 s deckt den Rückzug des Stroms nach dem Neustart (erster Versuch ≤ 1 s, ADR-0037 7c) plus einen Abruf; erwartet ≈ 1–2 s.
- [ ] **Auslöser** (Node `nachhollauf`, gestellte Zeit): Stand `{ channel: 'a', geladen: false, gescheitert: true }`; `wiederverbunden` → `laden('a')` sofort, kein `abrufen`, kein `fertig`; `laden` liefert `fehler` → `gescheitert(jetzt + 1000)`; bei 1.000 zweiter Versuch → `geladen` → `fertig({ art: 'geladen' }, true)` genau einmal. Mutante „`wiederverbunden` meldet ohne Laden fertig“ → rot.
- [ ] **Unterwegs** (Node): `{ geladen: false, gescheitert: false }`, `wiederverbunden` → `laden` sofort. Mutante „Auslöser nur bei `gescheitert`“ → rot.
- [ ] **Abgebrochen beendet** (Node): `laden` liefert `abgebrochen` → kein Zeitgeber, kein `fertig`, nach 10 s kein weiterer Versuch. Mutante „abgebrochen zählt als Fehlschlag“ → rot.
- [ ] **Rückzug und Wahl** (Node): Fehlschlag bei 0, Channelwechsel bei 500 → kein Versuch bei 1.000; `jetztVersuchen` bei 300 → Versuch bei 300, keiner bei 1.000.
- [ ] **NeuerStand** (Node): bei `gescheitert` → Laden-Lauf, `fertig` mit `mitLeiste = false`; bei `geladen = false, gescheitert = false` → nichts.
- [ ] **Text der Leiste** (Node `verbindungsleiste`): `wiederVerbundenText({ art: 'geladen' })` → „✓ Wieder verbunden · Verlauf geladen“.
- [ ] Keine neuen Fehler: alle Tests aller Ebenen grün (R00007, R00012, R00025 eingeschlossen)

## Betroffene Verzeichnisstruktur

- Web: geändert `web/verlauf.js`, `web/nachhollauf.js`, `web/hauptansicht.js`, `web/verbindungsleiste.js`.
- Tests: `e2e/einheit/nachhollauf.test.mjs`, `e2e/einheit/verbindungsleiste.test.mjs`, `e2e/tests/r00025-verbindung.spec.js` (P1 zweimal; Neustart auf festem Port mit `freierPort()`, `instanz.beenden()`/`starten()` aus `e2e/tests/hilfen.js`).

## Implementierungshinweise

### Grobentwurf (Vorplanung)

Die Bubbles der WBS sind Vorplanung; welche es am Ende werden, entscheidet der Entwickler. Die fünf Bausteine folgen ADR-0037 Karte #61:

- F0126: B0560 Verlauf meldet sein Laden · B0561 Laden-Lauf im Nachhol-Lauf · B0562 Hauptansicht verdrahtet das Laden · B0563 Meldung „Verlauf geladen“ · B0564 Abnahme-Probe P1 zweimal

## Offene Fragen

Keine an den Betreiber. Entschieden vom planer:

- **Weiche falsch gebaut statt reifer**: R00007 sagt nur „eine neue Wahl versucht es erneut“, F0047 aber „nach jedem Wiederverbinden … stehen die Nachrichten da“ und R00025 macht die Rückkehr zur Zusage. Verworfen: „reifer“ — der Befund verlangt keinen neuen Umfang, sondern dass eine bestehende Zusage hält.
- **Option D bleibt außerhalb**: bleibt der Strom verbunden und scheitert nur der erste Abruf (500, hängender Abruf), hilft weiter nur die neue Wahl des Channels. Vorschlag an den Director: eigene Karte, falls ein Entwicklertest das als Befund meldet; dann auch eine Zeitgrenze von 10 s für das erste Laden.

## Warum löst diese Anforderung das Problem? (Pflicht)

Auslöser: Entwicklertest #58, Anmerkung A1 — Tab geöffnet, während die Instanz neu startet; nach der Rückkehr bleibt der Verlauf leer, auch eigene Nachrichten erscheinen nicht.

- Der Nachhol-Lauf kennt die Art „laden“ und löst sie bei jedem nie geladenen Channel aus. Deshalb öffnet die Rückkehr den Verlauf neu, statt „nachgeholt, 0“ zu melden.
- Der Versuch wiederholt sich mit Zeitgrenze und Rückzug. Deshalb genügt ein halb hochgefahrener Server nicht, um den Tab dauerhaft leer zu lassen.
- Der Puffer der Störung wird verworfen und das Laden gleicht gegen alle Zeiten ab. Deshalb steht jede Nachricht — auch die eigene — genau einmal da, und der Puffer wächst nicht über eine lange Störung.
- „✓ Wieder verbunden · Verlauf geladen“ kommt erst nach dem Verlauf. Deshalb sagt die Leiste nie „wieder da“ über einem leeren Channel.

## Missing-Docs

- Glossar fehlt (`Dokumentation/Domaenenwissen/` existiert nicht).

## Notizen

- Priorität: Hoch (Klasse Vorrang auf Karte #61). Betroffene Nutzer: jeder, dessen Tab öffnet, während die Instanz ausfällt oder neu startet (Mensch und Agent in der Weboberfläche). Workaround: den Channel erneut wählen.
- Konflikte: offene Knoten, die dieselben Dateien berühren (aus der WBS gelesen, Dateiliste nicht gegengeprüft): F0089, F0090 (I0009), F0102 (I0010), F0118. Reihenfolge entscheidet der Director.
- Verworfen: B, C (siehe Alternative Ansätze); D als eigene Karte vorgeschlagen.
