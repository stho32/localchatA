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
- [ADR-0017: Stiller Abriss — der Kernel erkennt tote Verbindungen (Abrissgrenze und Keepalive je Verbindung), Herzschlag 5 s](../ADRs/0017-stiller-abriss-kernel-erkennt-tote-verbindungen.md) — Vorgeschlagen, 2026-10-28
- [ADR-0018: `POST /api/verlassen` — sofort `weg`, Name frei; die Oberfläche ruft es nie beim Schließen](../ADRs/0018-verlassen-sofort-weg-nie-beim-schliessen.md) — Akzeptiert, 2026-10-28
- [ADR-0023: Channels als zweites Thema im vorhandenen Ereignisstrom](../ADRs/0023-channels-als-zweites-thema-im-ereignisstrom.md) — Akzeptiert, 2026-10-28
- [ADR-0024: Oberfläche für Channels — Seitenleiste links, Anlegen im modalen `<dialog>`, Mitte unverändert bis I0008](../ADRs/0024-seitenleiste-und-anlegen-dialog-mitte-bleibt-bis-i0008.md) — Akzeptiert, 2026-10-28
- [ADR-0019: Restzeit `weg_um` in Serverzeit mit Uhrenabgleich über `jetzt`; gestrichelt heißt „gleich weg“](../ADRs/0019-restzeit-weg-um-in-serverzeit-und-statusformen.md) — Akzeptiert, 2026-10-28
- [ADR-0026: Verlauf im Browser als Fenster aus höchstens fünf Blöcken — Nachladen an beiden Enden, Anker von Hand; was „sofort“ und „flüssig“ messbar heißt](../ADRs/0026-verlauf-im-browser-als-fenster-aus-hoechstens-fuenf-bloecken.md) — Akzeptiert, 2026-10-28
- [ADR-0027: Channel wählen — Adresse `#/c/<name>`, Seitenleiste aus Links, zuletzt gewählter Channel je Tab; Name und Beschreibung in der Kopfzeile](../ADRs/0027-channel-waehlen-adresse-als-hash-zuletzt-gewaehlt-je-tab.md) — Akzeptiert, 2026-10-28
- [ADR-0028: Nachrichten live als drittes Thema im Ereignisstrom — Ordnung über die Nachrichtenzeit je Channel, kein Stand im Strom](../ADRs/0028-nachrichten-live-als-drittes-thema-ordnung-ueber-die-nachrichtenzeit.md) — Akzeptiert, 2026-10-28 (gebaut mit I0009)
- [ADR-0033: Frist in der Oberfläche — `# name` in der Kopfzeile öffnet „Channel-Einstellungen“, Verkürzen als zweiter Schritt mit Zahlen](../ADRs/0033-frist-in-der-kopfzeile-dialog-mit-bestaetigung-beim-verkuerzen.md) — Akzeptiert, 2026-10-28
- [ADR-0037: Zustellung im Tab — vorläufig, bestätigt, je Frame gebündelt](../ADRs/0037-zustellung-im-tab-vorlaeufig-bestaetigt-zeichnen-je-frame.md) — Akzeptiert, 2026-10-28
- [ADR-0038: Eingabe — mehrzeilig, `Enter` sendet, Entwurf je Channel, gebündelte Ansage](../ADRs/0038-eingabe-mehrzeilig-enter-sendet-entwurf-je-channel-gebuendelte-ansage.md) — Akzeptiert, 2026-10-28
- [ADR-0039: Große Antworten — Byte-Budget je Block, schwere Arbeit nur auf der Werkbank, die Abrissgrenze ist die Sendefrist](../ADRs/0039-grosse-antworten-byte-budget-je-block-werkbank-und-abrissgrenze-als-sendefrist.md) — Akzeptiert, 2026-10-28

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
- **Ergänzung 2026-10-28 (Karte #5, I0006):**
  - **Wächter** 15 s statt 40 s (3 × Herzschlag, [ADR-0017](../ADRs/0017-stiller-abriss-kernel-erkennt-tote-verbindungen.md)).
  - **Restzeit** ([ADR-0019](../ADRs/0019-restzeit-weg-um-in-serverzeit-und-statusformen.md)): wer Ereignisse liest (Worker, im Rückfall der Tab), bildet aus `jetzt` den Uhrenversatz und reicht ihn mit; der Worker gibt auch den Herzschlag weiter und jedem neuen Tab zum letzten Stand den aktuellen Versatz. Eine reine Funktion rechnet `weg_um`, Versatz, Uhr → „weg in m:ss“ (≤ 60 s) | „offline“; ein Sekundentakt zeichnet nur Zeilen „gleich weg“ neu. Gestrichelt heißt nur noch „gleich weg“; „zuletzt online“ ist ein gefüllter Punkt in `lc-offline`. Statusspalte fest so breit, dass „weg in 1:00“ passt (statt 52 px), für alle Zeilen gleich.
  - **Verlassen** ([ADR-0018](../ADRs/0018-verlassen-sofort-weg-nie-beim-schliessen.md)): nie bei `pagehide`/`beforeunload`, nie aus dem Worker (`sendBeacon` scheidet aus: kein Header). Eine Bedienung „Verlassen“ gibt es erst, wenn das Designsystem sie hat; dann: Token aus dem `localStorage` entfernen, `verlassen` mit Header, kein stiller Wiederbeitritt ohne Token, kein Neuverbinden nach eigenem `weg`.
  - **Erste Tests (Node):** `restzeit` mit gestellter Uhr und Versatz (ADR-0019); Verteiler gibt neuem Tab Stand plus Versatz aus dem letzten Herzschlag; Wächter 15 000 ms → weiter, 15 001 ms → neu.
- **Ergänzung 2026-10-28 (Karte #6, I0007):** Neue Teile: Seitenleiste links (`<nav aria-label="Channels">`, Liste `# name`, Schalter `+`), Dialog „Channel anlegen“ (`<dialog>`, `showModal()`, `Strg Alt N` ohne `AltGraph`), reine Zusammenführung `channelsZusammenfuehren` (Stand ersetzt, Wechsel mit größerem `stand` fügt nach Name ein), Worker merkt sich beide Listen ([ADR-0023](../ADRs/0023-channels-als-zweites-thema-im-ereignisstrom.md), [ADR-0024](../ADRs/0024-seitenleiste-und-anlegen-dialog-mitte-bleibt-bis-i0008.md)). Die Mitte bleibt bis I0008 leer. Erste Tests: ADR-0023 (Node), ADR-0024 (Node, E2E).
- **Ergänzung 2026-10-28 (Karte #7, I0008 — Channel wählen und Verlauf):**
  - **Dateien (Vorschlag):** `adresse.js` — rein: `auswahl(hash, liste)`, Merker je Tab (`sessionStorage`), `Alt ↑/↓` ([ADR-0027](../ADRs/0027-channel-waehlen-adresse-als-hash-zuletzt-gewaehlt-je-tab.md)); `fenster.js` — rein: Nachlade-Entscheidung und Einfügen/Abwerfen von Blöcken ([ADR-0026](../ADRs/0026-verlauf-im-browser-als-fenster-aus-hoechstens-fuenf-bloecken.md) Regeln 1–2); `nachricht.js` — rein: Kopf oder gruppiert, Tagestrenner, Uhrzeit `HH:MM` aus `zeit` (Versatz des Servers, wie die Logdatei), plus Zeichnen per `textContent` (Markdown erst I0017); `verlauf.js` — DOM: Fenster zeichnen, Anker in einem Zug, `ResizeObserver` am Ende, Pille „Zum Neuesten“, `AbortController` je Channel; `seitenleiste.js` macht aus den Einträgen Links mit `aria-current`; `kopfzeile.js` bekommt die Zone „Ort“ (`# name`, später Beschreibung).
  - **Zeit im Verlauf:** Uhrzeit und Tagestrenner aus dem Text von `zeit` mit dem Versatz des Servers — so fällt ein Tagestrenner auf den Wechsel der Logdatei (ADR-0003), und Browser und `less` zeigen dieselbe Uhrzeit; das volle Datum steht im Tooltip.
  - **Grenzen:** höchstens 500 Nachrichten im DOM; kein Zwischenspeicher je Channel; Verlauf `role="log"` mit `aria-live="off"`, fokussierbar.
  - **Erste Tests:** Node für `auswahl`, Fenster-Entscheidung (Spanne um 2 × Sicht), Gruppierung über eine Blockgrenze; E2E L2–L5 nach ADR-0026 Regel 7 und die Adress-Fälle aus ADR-0027.
- **Ergänzung 2026-10-28 (Karte #12, I0018 — Frist):** `# name` in der Kopfzeile wird `<button aria-haspopup="dialog">` und öffnet den Dialog „Channel-Einstellungen“ mit dem Feld „Aufbewahrungsfrist“; ab 720 px steht `· 7 Tage` (bzw. `· ⚠ Frist gesperrt`) vor der Beschreibung; Verkürzen ist ein zweiter Schritt im selben Dialog mit den Zahlen aus der 409, Fokus auf „Zurück“; nach dem Verkürzen öffnet der eigene Tab den Channel neu ([ADR-0033](../ADRs/0033-frist-in-der-kopfzeile-dialog-mit-bestaetigung-beim-verkuerzen.md)). **Dateien (Vorschlag):** `frist.js` — rein: `fristPruefen(text)`, `fristText(tage)` (Singular/Plural, `null`), `bestaetigungText(entfaellt)` (deutsche Zahlen und Datum); `einstellungen.js` — DOM: Dialog mit zwei Schritten; `kopfzeile.js` — Schalter und Frist-Text, aktualisiert aus `channelsZusammenfuehren`. Erste Tests: ADR-0033, „Folgeentscheidungen“.
- **Ergänzung 2026-10-28 (Karte #8, I0009 — Senden):** Neue Teile: Eingabe unter dem Verlauf (`<textarea>`, `Enter`/`Strg Enter` senden, `Shift Enter` Zeile, nie während IME, Entwurf je Channel in `sessionStorage`, Längen- und Zeichenregel wie der Server, [ADR-0038](../ADRs/0038-eingabe-mehrzeilig-enter-sendet-entwurf-je-channel-gebuendelte-ansage.md)); Zustellung als reine Funktionen — vorläufige Nachrichten mit Warteschlange (ein Sendeauftrag je Tab, Wiederholschlüssel), Bestätigung über Antwort oder eigenes Ereignis, fremde Nachrichten je Frame angefügt oder gezählt (Pille), Lücken über `nach=` vor der Warteschlange ([ADR-0037](../ADRs/0037-zustellung-im-tab-vorlaeufig-bestaetigt-zeichnen-je-frame.md)); gebündelte Ansage in eigener Statusregion, höchstens alle 10 s. Tab-Reihenfolge Kopfzeile → Seitenleiste → Verlauf → Eingabe → Teilnehmerliste. Erste Tests: ADR-0037 und ADR-0038, „Folgeentscheidungen“.
