# ADR-0024: Oberfläche für Channels — Seitenleiste links, Anlegen im modalen `<dialog>` über `+` und `Strg Alt N`, Mitte unverändert bis I0008

**Status**: Akzeptiert
**Datum**: 2026-10-28
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #6 I0007)
**Kontext-Anforderung**: — (WBS I0007; Grenze zu I0008 „Channel wechseln und Verlauf lesen“)
**Erprobt durch**: — (folgt dem Designsystem und den gebauten Mustern `web/hauptansicht.js`, `web/beitreten.js`, `web/namensregel.js`)

## Kontext

- Designsystem (Export 2026-10-27): `components/Seitenleiste` — linke Spalte, Abschnittskopf in `lc-label` mit Schalter `+` „Channel anlegen“, Einträge `# name` in 28-px-Zeilen, aktiver Eintrag mit `aria-current="page"`, Zähler für Ungelesen/Erwähnt, unter 720 px nur 48 px breit mit `#` und Tooltip. `components/Hauptansicht` — Tab-Reihenfolge Kopfzeile → Seitenleiste → Verlauf → Eingabe → Teilnehmerliste; leerer Channel „Noch keine Nachrichten in # name“ plus Hinweis, wie Agenten schreiben. `1-tastatur.md` — `Strg Alt N` Channel anlegen (überall), `Esc` schließt Dialoge, `Alt ↑/↓` wechselt Channels. `design-prompt.md` 4.3 — Zustände „Name schon vorhanden“, „ungültiger Name“. Kein Artboard für 4.3 und keins für D0003.
- Gebaut: Hauptansicht mit leerer Mitte (`<main class="lc-center">`) und Teilnehmerliste rechts (`web/hauptansicht.js`); Live-Prüfung der Namensregel beim Tippen (`web/namensregel.js`, Beitreten); Fehlergestalt `kd-danger` mit ⚠ und `aria-invalid` (Gelerntes entwickler #3); CSP ohne Inline-Stil (ADR-0014 Regel 6).
- Grenze zu I0008: Channel **wechseln**, Verlauf zeigen, Leerzustand „Noch keine Nachrichten“. Der Hinweis „wie Agenten hier schreiben“ setzt eine Senderoute voraus (I0009/I0015).
- Lehre #3: die erste Oberfläche eines Bereichs braucht den UX-Maßstab im Auftrag (Fokus auf Akzentflächen, 320 px ohne Querscrollen, Fehlergestalt).

## Optionen (Anlegen)

### Option A: Modaler `<dialog>` (`showModal()`), geöffnet über `+` und `Strg Alt N`
- Vorteile: der Browser setzt den Fokus in den Dialog, sperrt den Rest (inert) und schließt mit `Esc` — ohne eigene Fokusfalle; derselbe Ort für beide Auslöser; Platz für Fehlerzeile und Vorschlag; CSP-verträglich.
- Nachteile: ein Overlay für ein einziges Feld wirkt schwer; `::backdrop` und Fokus-Rückgabe müssen gestaltet werden.

### Option B: Eingabezeile direkt in der Seitenleiste unter dem Abschnittskopf (Inline-Anlegen)
- Vorteile: leicht, kein Overlay; der neue Eintrag erscheint dort, wo getippt wurde.
- Nachteile: unter 720 px ist die Leiste 48 px breit — dort passt kein Feld; Fehlertext und Vorschlag haben in 28-px-Zeilen keinen Platz; eigene Fokusführung nötig.

### Option C: Anlegen über die Suche/Schnellwechsel (`Strg K`/`Strg P`: „# neuer-name anlegen“)
- Vorteile: Tastatur-Fluss wie Slack; ein Feld für Finden und Anlegen.
- Nachteile: Suche und Schnellwechsel gibt es noch nicht (I0011); koppelt I0007 an eine spätere Interaction.

## Optionen (Mitte, solange I0008 fehlt)

| # | Mitte | Stärke | Schwäche |
|---|---|---|---|
| M1 | unverändert leer (wie seit I0004); Einträge der Seitenleiste sind noch keine Links | baut nichts, was I0008 wegwirft; keine falsche Aussage | nach dem Anlegen passiert in der Mitte nichts |
| M2 | Kopf des gewählten Channels (`# name`, Beschreibung, „angelegt von … am …“) | gibt dem Anlegen ein sichtbares Ziel | nimmt „Channel wählen“ aus I0008 vorweg; Auswahl ohne Verlauf ist ein Zwischenstand, der umgebaut wird |
| M3 | Leerzustand des Designsystems „Noch keine Nachrichten in # name“ | Text aus dem Designsystem | falsch für Channels, die schon Logs im Ordner haben; der Agenten-Hinweis verweist auf eine Route, die es noch nicht gibt |

## Entscheidung

**Gewählt: Option A für das Anlegen, M1 für die Mitte.**

Es überwiegt, dass der native Dialog Fokus, Sperre und `Esc` ohne eigenen Code richtig macht und auch bei 48 px Seitenleiste funktioniert; für die Mitte, dass I0007 nichts zeigt, was falsch ist oder in I0008 weggeworfen wird. In Kauf genommen: ein Overlay für ein Feld (plus optionale Beschreibung) und eine Mitte, in der sich beim Anlegen nichts tut — die Rückmeldung liegt in Seitenleiste und Statuszeile.

Regeln:

1. **Seitenleiste** `<nav aria-label="Channels">` links in `.lc-main` (Breite `lc-sidebar`), vor der Mitte in der Tab-Reihenfolge: Kopf `<h2>` „Channels“ in `lc-label`, rechts `<button>` `+` mit `aria-label="Channel anlegen"` und `aria-keyshortcuts="Control+Alt+N"`; darunter `<ul>` mit je `<li>` `# name` (Name in der Schrift der Liste, `#` gedämpft), nach Name sortiert. Kein Zähler, kein `aria-current`, kein Link — das bringen I0008 (Wechseln) und spätere Interactions (Ungelesen, Erwähnt).
2. **Leere Liste**: eine Zeile „Noch keine Channels.“ in `kd-ink-muted` unter dem Kopf; der Schalter `+` ist der Weg.
3. **Unter 720 px** (48 px): Einträge zeigen `#` mit dem Namen als `title` und als zugänglichen Namen; `+` bleibt sichtbar. Bei 320 px kein Querscrollen.
4. **Dialog** `<dialog aria-labelledby>` mit Titel „Channel anlegen“, Feld „Name“ (Hinweis mit der Regel, Live-Prüfung über dieselbe Funktion wie Beitreten, `web/namensregel.js`), optional „Beschreibung“ (einzeilig, `maxlength` 200), Schalter „Channel anlegen“ (Primär, `kd-accent`) und „Abbrechen“. `Enter` im Feld sendet, `Esc` und „Abbrechen“ schließen, der Fokus kehrt zum Auslöser zurück (`+` oder das zuvor fokussierte Element).
5. **Antworten im Dialog**: 201 → Dialog zu, Statuszeile (`role="status"`) „Channel # name angelegt.“, der neue Eintrag ist in der Liste (über das eigene Ereignis oder sofort aus der Antwort — einmal, nicht doppelt). 409 vorhanden → Fehlerzeile „# name gibt es schon.“; 422 → Fehlerzeile mit Regel und, wenn vorhanden, Schalter „`vorschlag` übernehmen“; 429 → „Zu viele Channels: höchstens n …“ mit der Grenze aus der Antwort; Netzfehler → „Nicht angelegt — Verbindung prüfen.“ Fehler in `kd-danger` mit ⚠, Feld `aria-invalid="true"`, Fehlertext über `aria-describedby`; der Dialog bleibt offen, die Eingabe bleibt stehen.
6. **Live bei anderen**: ein fremd angelegter Channel erscheint an seiner Sortierstelle, ohne den Fokus zu verschieben und ohne Ansage (kein Lärm bei Agenten, die anlegen).
7. **Tastatur** `Strg Alt N` öffnet den Dialog überall, außer wenn `AltGraph` gedrückt ist (`getModifierState('AltGraph')`): auf deutschen Tastaturen ist `AltGr` = `Strg Alt`, und `AltGr N` tippt in Textfeldern ein Zeichen.
8. **Beschreibung** erscheint in I0007 nur als `title` des Eintrags (isoliert, ADR-0022 Regel 5); ihr sichtbarer Ort ist der Kopf des Channels in I0008.
9. **Mitte** bleibt die leere Fläche aus I0004 (M1). Die Anforderung nennt das unter „nicht Ziel“, damit Entwicklertest und Abnahme es nicht als Lücke werten.

## Konsequenzen

**Positiv:**
- I0008 macht aus den `<li>` Links mit `aria-current`, füllt die Mitte und übernimmt den Leerzustand — ohne Rückbau.
- Fokus, Sperre und `Esc` des Dialogs liefert der Browser; Tests prüfen das Ergebnis, nicht eigene Mechanik.

**Negativ / Inkauf zu nehmen:**
- Nach dem Anlegen „passiert in der Mitte nichts“ — Rückmeldung nur über Statuszeile und Liste.
- Ohne Artboard für 4.3 entscheidet die Gestaltung des Dialogs der Entwickler aus Tokens und Bausteinen; der `ux-reviewer` prüft gegen das Designsystem.

**Pre-Mortem (ein Monat nach dem Start gescheitert, weil …):**
1. … `AltGr N` in der Eingabe den Dialog öffnet statt ein Zeichen zu tippen. Früh-Indikator: Meldung von Nutzern mit deutscher Tastatur. Gegenmaßnahme: Regel 7, Node-Test der Tastenauswertung (`ctrlKey && altKey && key === 'n'` mit und ohne `AltGraph`).
2. … der neue Channel beim Anleger doppelt erscheint (Antwort und Ereignis). Früh-Indikator: E2E zählt zwei gleiche Zeilen. Gegenmaßnahme: Zusammenführung nach Name (ADR-0023 Regel 6) — Einfügen ist Ersetzen.
3. … der Fokus nach dem Schließen im Nichts landet. Früh-Indikator: `document.activeElement === body` nach `Esc`. Gegenmaßnahme: Regel 4, E2E-Prüfung.

**Folgeentscheidungen / Aufgaben:**
- [ ] Erster Node-Test: Tastenauswertung (Regel 7); Einsortieren eines Channels in die Liste ohne Duplikat.
- [ ] Erster E2E-Test: zwei Browser-Kontexte; A öffnet mit `Strg Alt N`, legt `allgemein` an → Dialog zu, Fokus auf `+`, Statuszeile; B sieht `# allgemein` ohne Neuladen; A versucht `Allgemein` → Fehlerzeile mit Schalter „allgemein übernehmen“, nach Übernahme 409 „gibt es schon“; Breite 320 px ohne Querscrollen; keine Konsolenzeile außer den vorgeschriebenen Antworten (409, 422 — Kriterium nach `AI-Team/planer.md` (5)).

## Quellen / Bezuege

- [A00005 Weboberfläche](../Architektur/A00005-weboberflaeche.md), [A00004 HTTP-Schnittstelle](../Architektur/A00004-http-schnittstelle.md)
- Designsystem `components/Seitenleiste/README.md`, `components/Hauptansicht/README.md`, `1-tastatur.md`, `README.md`, `3-offene-fragen.md` (Frage 10); `design-prompt.md` 4.2, 4.3
- ADR-0014 (CSP), ADR-0020, ADR-0021, ADR-0022, ADR-0023; `AI-Team/entwickler.md` Gelerntes #3; `AI-Team/planer.md` Gelerntes #4
