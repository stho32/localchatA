---
id: A00005
name: Weboberfläche
code: web
ebene: Komponente
eltern: A00002
repo: localchatA
status: Aktiv
herkunft: dialog
quellen:
  - web/
verwandt: [A00004]
domaene: []
blackbox: rahmengetrieben (Browser); Gestaltung gehört dem Designsystem, Verfeinerung mit den Dialogs
---

# Weboberfläche

## Verantwortung

Zeigt dem Menschen im Browser Verlauf, Eingabe und Teilnehmer nach dem Designsystem.

## Schnittstellen

- Eingehend: Browser lädt `/` und `/web/*`.
- Ausgehend: `fetch` auf `/api/*` mit Header `X-LocalchatA-Sitzung` (Token aus `localStorage`, [ADR-0012](../ADRs/0012-identitaet-und-sitzung.md)), ab I0005 Live-Strom auf `/api/ereignisse` — ein Strom je Browser in einem `SharedWorker` (`web/strom.js`), `fetch` mit Header, Wächter für stille Abrisse wie kanbanD `web/verbindung.js`; ohne `SharedWorker` je Tab ([ADR-0015](../ADRs/0015-ein-ereignisstrom-je-browser-sharedworker-mit-sitzungs-header.md)).

## Abhaengigkeiten

[HTTP-Schnittstelle](A00004-http-schnittstelle.md); Designsystem `Dokumentation/Wireframes/designsystem/` (Tokens, Bausteine).

## Entscheidungen

- [ADR-0002: Oberfläche ohne Build-Schritt in das Programm eingebettet](../ADRs/0002-oberflaeche-ohne-build-schritt-eingebettet.md) — Akzeptiert, 2026-10-27
- [ADR-0012: Identität und Sitzung — Herkunft aus der Gegenstelle, Sitzungs-Token im Speicher, eine Sitzung je Browser](../ADRs/0012-identitaet-und-sitzung.md) — Akzeptiert, 2026-10-27
- [ADR-0014: Grenzschutz im LAN — Host-Erlaubnisliste, Origin-Prüfung für Schreibendes, Sicherheitsheader auf jeder Antwort](../ADRs/0014-grenzschutz-host-origin-sicherheitsheader.md) — Akzeptiert, 2026-10-27
- [ADR-0015: Ein Ereignisstrom je Browser — SharedWorker mit `fetch`-Strom und Sitzungs-Header](../ADRs/0015-ein-ereignisstrom-je-browser-sharedworker-mit-sitzungs-header.md) — Akzeptiert, 2026-10-28
- [ADR-0016: Anwesenheitsstatus aus Haltern mit Nachlauf, Teilnehmerliste als Stand plus Wechsel](../ADRs/0016-anwesenheitsstatus-aus-haltern-mit-nachlauf-liste-als-stand-plus-wechsel.md) — Akzeptiert, 2026-10-28

## Offene Fragen

- Markdown wird im Browser gerendert. Welcher Renderer? Empfehlung: eigener kleiner Renderer wie kanbanD `web/markdown.js` mit Escape-zuerst (kein `innerHTML` aus Rohtext) — Prüfpunkt für den `security-reviewer` (Agenten schreiben beliebigen Text).

## Notizen / Quellen

- Vanilla-JavaScript-Dateien, je eine Verantwortung, reine Funktionen auch in Node testbar (Unit-Tests der Oberfläche unter `e2e/einheit/` wie kanbanD).
- Schriften IBM Plex lokal eingebettet (keine Netzabhängigkeit im LAN).
- **Ist 2026-10-27 (nach I0001):** `web/index.html` (Titel `localchatA`, deutsch) und `web/lc-tokens.css`, eingebettet per `include_bytes!` (`src/http/eingebettet.rs`); noch kein JavaScript, kein `EventSource`.
- **Ergänzung 2026-10-27 (Karte #3, I0004 — erstes JavaScript):** ADR-0002 trägt auch den Beitreten-Bildschirm; eine eigene ADR für die Oberfläche braucht es nicht. Festlegungen für den Bau:
  - **ES-Module ohne Bundler**: `index.html` lädt genau ein `<script type="module" src="/web/start.js">`; je Verantwortung eine Datei (Vorschlag: `api.js` — `fetch` mit Sitzungs-Header und Fehlerform; `sitzung.js` — Token und gemerkter Name in `localStorage`, Ablauf Laden → `GET /api/ich` → Wiederbeitritt; `namensregel.js` — Hinweis beim Tippen, rein; `beitreten.js` — Bildschirm 4.1 mit den Zuständen „Name vergeben“ und „Server weg“; `kopfzeile.js` — eigene Kennung). Jede Datei steht in `src/http/eingebettet.rs` mit Content-Type `text/javascript; charset=utf-8`; unbekannte Namen bleiben 404.
  - **CSP-tauglich** ([ADR-0014](../ADRs/0014-grenzschutz-host-origin-sicherheitsheader.md)): kein Inline-Skript, kein `<style>`-Block, keine `style=`- oder `on…=`-Attribute; das heutige `<style>` in `index.html` zieht nach `web/lc-seite.css`. Text aus dem Server (Kennungen, Fehlertexte) nur über `textContent`, nie `innerHTML`.
  - **Tests der Oberfläche**: reine Funktionen (`namensregel.js`, Zustandswahl in `sitzung.js` mit einem `localStorage`-Ersatz) unter `e2e/einheit/` mit `node --test` ([ADR-0009](../ADRs/0009-teststrategie-gate-und-health-check.md)); Bildschirm und Abläufe per Playwright gegen das echte Programm, darunter: zweiter Browser-Kontext mit demselben Namen → Zustand „Name vergeben“ mit Vorschlag; Neuladen → derselbe Teilnehmer ohne Beitreten-Bildschirm; Programm neu gestartet → Neuladen tritt still mit dem gemerkten Namen bei; keine Konsolenfehler (CSP). Die Namensregel gilt am Server; die Oberfläche spiegelt sie nur als Hinweis (Quelle der Wahrheit ist die 422-Antwort).
  - Erster Test (Node, rein): `namensregel.pruefen("Jörg")` → ungültig mit Vorschlag `joerg`; `pruefen("claude-2")` → gültig.
- **Ergänzung 2026-10-28 (Karte #4, I0005 — Live-Strom und Teilnehmerliste):**
  - **Dateien (Vorschlag):** `strom.js` — `SharedWorker`: ein `fetch`-Strom mit Header, Wiederverbinden mit Rückzug, letzter Stand für neue Tabs, 401 an die Tabs; `sse.js` — reine SSE-Zerlegung (Teilstücke, `\r\n`, Kommentare); `verbindung.js` — reine Wächter-Entscheidung (40 s Stille, Rückzug 1/2/5/10 s); `teilnehmerliste.js` — reine Zusammenführung Stand + Wechsel nach `stand` und Gruppierung nach Designsystem, dazu das Zeichnen der Spalte; `live.js` — im Tab: Worker verbinden oder Rückfall je Tab. Der Worker wird mit der Programmversion in der URL angelegt (`/web/strom.js?v=<version>`), damit nach einem Update kein alter Worker weiterläuft; `eingebettet.rs` liefert die Datei unabhängig vom Query aus.
  - **CSP**: der Worker kommt aus `/web/` (`script-src 'self'`, `worker-src` fällt darauf zurück — geprobt in ADR-0015); kein `blob:`-Worker.
  - **Darstellung** (Designsystem `components/Teilnehmerliste`, keine Architekturfrage): Menschen zuerst, Agenten danach, ab 8 Agenten nach Herkunft gruppiert, alphabetisch; Status nie nur über Farbe; Wechsel nicht angesagt. Die eigene Zeile erkennt die Oberfläche an der eigenen Kennung. Text aus dem Server nur über `textContent`.
  - **Tests**: Node (`e2e/einheit/`) für `sse.js`, `verbindung.js`, Zusammenführung und Gruppierung; Playwright gegen das echte Programm: zwei Browser-Kontexte sehen sich gegenseitig online, Kontext schließen → beim anderen offline in < 2 s; ein Kontext mit 7 Tabs → alle laden und zeigen die Liste; keine Konsolenfehler. Erste Tests im Einzelnen: ADR-0015 und ADR-0016.
  - **Nachzug aus #3 (nur genannt):** der Fokusrahmen auf gefüllten Schaltern liegt außen statt „nach innen versetzt“ wie im Designsystem-README (UX-Review #3, B0064) — eine Frage des Designsystems, keine der Architektur; das README sollte die Ausnahme nennen.
