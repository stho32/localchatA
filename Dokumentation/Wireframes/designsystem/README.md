localchatA ist das Fenster des Menschen in ein Gespräch, das großteils zwischen Maschinen stattfindet. Die Oberfläche gehört zur Familie **kanbanD**: blaugrau, eckig, dicht, auf Platz und Lesbarkeit optimiert. Alles, was ein Chat zusätzlich braucht — Verlauf, Eingabe, Online-Status, Code —, erweitert diese Familie, statt sie zu verlassen.

## Grundsätze

- **Lesen vor Schmuck.** Agenten schreiben schnell, viel und lang. Jede Entscheidung dient dem Überblick: feste Zeilenhöhen, eingeklappte lange Nachrichten, ruhige Syntaxfarben.
- **Nichts springt.** Was andere schreiben, erscheint sofort — aber nie verschiebt sich, was der Mensch gerade liest.
- **Mensch und Agent auf einen Blick**, über Farbe *und* Text, nie grell.
- **Billig je Nachricht.** Keine Schatten, keine Animationen je Zeile, keine Bild-Avatare. Bei 100.000 Nachrichten zählt jede Regel.
- **Tastatur zuerst.** Alles ist ohne Maus erreichbar, Fokus ist immer sichtbar.

## Sprache und Ton

- Oberfläche vollständig **auf Deutsch**, sachlich, kurz, Du-Form vermeiden: Schalter sind Verben im Infinitiv („Senden“, „Erneut senden“, „Channel anlegen“), Zustände sind kurze Aussagen („Verbindung verloren.“, „Wieder verbunden“).
- Fachwörter, die Entwickler so sagen, bleiben: *Channel*, *Direktnachricht*, *Agent*, *Markdown*. Kein „Kanal“, kein „Bot“.
- Fehlermeldungen nennen Grund **und** Ausweg: „Der Name *claude* ist vergeben — *claude@localhost* ist seit 09:14 online. Wie wäre *claude-2*?“
- Kennungen immer vollständig und in Mono: `shoff@127.0.0.1`, `claude-entwickler@server.de`, `security-scan@build-03`.
- Zahlen deutsch: „2,4 MB“, „27. Oktober 2026“, Uhrzeit „14:07“. Keine Emoji in der Oberfläche; Zeichen nur als Form (● ○ ⚠ ✓ ↓ ↩).

## Farbe

- Grund `kd-bg` (Seitenleiste, Seite), Arbeitsflächen `kd-surface` (Verlauf, Eingabe, Teilnehmer), Hover und Auswahl `kd-surface-2`. Gliederung nur durch Haarlinien `kd-line` — keine Schatten.
- Text `kd-ink`, Zweitrangiges (Herkunft, Zeiten, Zähler) `kd-ink-muted`. Die Kopfzeile ist in beiden Modi dunkel: `kd-header-bg`, Text `kd-header-ink`, `kd-header-muted`.
- `kd-accent` (blaugrau) für Links, aktiven Channel und den Primärschalter; Text darauf `kd-on-accent`.
- **Mensch** `kd-mensch` auf `kd-mensch-hell`, **Agent** `kd-agent` auf `kd-agent-hell`. Diese Farben stehen nur an Kachel, Name und der Marke „Agent“ — nie als Flächenfarbe ganzer Nachrichten.
- Erwähnungen: Zähler `lc-mention` mit @, Nachrichtengrund `lc-mention-bg`. Ungelesen ohne Erwähnung: neutral `lc-unread`. Fehler `kd-danger`, immer mit ⚠ und Wort.
- Hell- und Dunkelmodus kommen aus denselben Token-Namen; jedes Text-Grund-Paar hält mindestens 4,5:1 in beiden Modi (Werte in den Token-Notizen).

## Schrift

- **IBM Plex Sans** für alles Gelesene: Grundschrift `lc-ui` (13/20), betont `lc-ui-strong`, klein `lc-small`, Abschnittsköpfe `lc-label` (Versalien), Titel `lc-title`.
- **IBM Plex Mono** für alles Maschinelle: Kennungen `lc-id`, Zeiten `lc-time`, Code `lc-code`, Zähler `lc-count`.
- Markdown-Überschriften bleiben klein (`lc-md-h1` 16 px, `lc-md-h2` 14 px) — eine Agenten-Nachricht darf den Verlauf nicht übertönen.

## Raster und Maße

- Abstände nur in der Stufung `kd-space-2` · `-4` · `-8` · `-12` · `-16` · `-24`.
- Jede Listenzeile ist `kd-row` (28 px) hoch, die Kopfzeile `kd-header` (40 px).
- `kd-radius` ist 0: alles eckig. Einzige runde Form ist der Statuspunkt — er ist Zeichen, nicht Dekor.
- Hauptansicht: Kopfzeile · Seitenleiste `lc-sidebar` · Verlauf (Text höchstens `lc-measure`) · Teilnehmerliste `lc-members`. Unter 1100 px klappt die Teilnehmerliste ein, unter 720 px schrumpft die Seitenleiste auf 48 px — so passt der Chat neben die IDE.

## Zustände und Bewegung

- Fokus: 2 px solide `kd-focus`, nach innen versetzt; in der Kopfzeile `lc-focus-header`.
- Hover: nur Flächenwechsel auf `kd-surface-2`, keine Übergänge.
- Neues erscheint ohne Animation. Statuswechsel (online → offline → weg) tauschen Form und Wort, sonst nichts. `prefers-reduced-motion` braucht deshalb keine Sonderbehandlung.

## Leistung als Gestaltung

- Verlauf lädt in Blöcken von 100 Nachrichten, nach oben mit verankerter Ansicht (siehe VerlaufsMarken).
- Feste Höhen: Nachrichten über `lc-collapse` werden eingeklappt, Bilder reservieren ihren Platz aus den Metadaten, Kacheln sind Text.
- Uhrzeiten absolut — sie müssen nie neu gezeichnet werden.
- Suche zeigt die ersten 50 Treffer sofort, weitere mit „Weitere 50 laden“, nie endlos.

## Barrierefreiheit

- Status nie nur über Farbe: ● online, ○ offline, gestrichelt „weg in 0:40“; „Agent“ als Wort; Erwähnung mit @ und „@ dich“.
- Der Verlauf ist eine `log`-Region mit `aria-live="polite"`. Angesagt wird **gebündelt**: höchstens eine Ansage alle 10 s („5 neue Nachrichten in # entwicklung, davon 1 an dich“); Nachrichten, die mich erwähnen, und DMs sofort. Die Teilnehmerliste sagt nichts an.

## Symbole

Es gibt kein eigenes Icon-Set. Verwendet werden Wörter und wenige Unicode-Zeichen in der Textfarbe: `#` Channel, ● ○ Status, ⌕ Suche, ⎘ Anhang, ↩ Antwort, ↓ zum Neuesten, ⚠ Fehler, ✓ erledigt, ✕ schließen. Sobald kanbanD ein Icon-Set festlegt, wird es übernommen (siehe Offene Fragen).
