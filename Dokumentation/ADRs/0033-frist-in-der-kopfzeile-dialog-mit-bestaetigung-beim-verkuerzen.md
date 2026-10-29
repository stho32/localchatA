# ADR-0033: Frist in der Oberfläche — `# name` in der Kopfzeile öffnet den Dialog „Channel-Einstellungen“, Verkürzen als zweiter Schritt mit Zahlen

**Status**: Akzeptiert
**Datum**: 2026-10-28
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #12 I0018)
**Kontext-Anforderung**: — (WBS I0018, Dialog D0003 Channels)
**Erprobt durch**: — (überträgt gebaute Mechanik: nativer `<dialog>` mit `showModal()`, Fehlerzeile in `kd-danger` mit ⚠ und `aria-invalid`, Fokus zurück zum Auslöser, Statuszeile `role="status"` — ADR-0024, gebaut in I0007; Kopfzeile Zone „Ort“ — ADR-0027 Regel 5)

## Kontext

- Vision: Frist je Channel wählbar; Leitplanke „Gleiches Bedienkonzept wie kanbanD“; Oberfläche deutsch.
- Das Designsystem (`Dokumentation/Wireframes/designsystem/`) und der Design-Auftrag (`design-prompt.md` 4.2, 4.3) sehen **keinen** Ort für die Frist vor; 4.3 „Channel anlegen“ nennt nur Name und Beschreibung. Es gibt kein Artboard für D0003.
- Kopfzeile, Zone „Ort“ (Designsystem `components/Kopfzeile`, ADR-0027 Regel 5): Instanzname, `# name` in `lc-title`, Beschreibung einzeilig mit Ellipse; unter 720 px entfällt die Beschreibung, bei 320 px kein Querscrollen. Fokus in der Kopfzeile `lc-focus-header`.
- Designsystem `1-tastatur.md`: „Gewohnte Konventionen (Slack/Discord, Browser)“. Dort öffnet ein Klick auf den Channelnamen im Kopf die Details des Channels.
- [ADR-0024](0024-seitenleiste-und-anlegen-dialog-mitte-bleibt-bis-i0008.md): Dialog-Muster (Titel, Felder, Primär „… anlegen“ in `kd-accent`, „Abbrechen“, `Enter`/`Esc`, Fehler im offenen Dialog, Eingabe bleibt).
- [ADR-0031](0031-frist-setzen-put-api-channels-name-frist-jeder-mit-sitzung-bestaetigung-beim-verkuerzen.md): `PUT …/frist` antwortet ohne Bestätigung mit 409 `bestaetigen` samt `entfaellt {tage, nachrichten, bis}`; `frist_tage: null` heißt Einstellungen defekt.
- Projektregel (AI-Team/entwickler.md, Gelerntes #3): Fokus auf gefüllten Schaltern ≥ 3:1, kein Querscrollen bei 320 px, Fehler in `kd-danger` mit ⚠ und `aria-invalid`.

## Optionen

### Option A: `# name` in der Kopfzeile wird ein Schalter → Dialog „Channel-Einstellungen“; daneben die Frist als Text
- Vorteile: kostet bei 320 px keine Breite (der Name steht ohnehin da, dazu ein `▾`); Konvention aus Slack/Discord; der Dialog ist der Ort für spätere Einstellungen des Channels; die Frist ist ab 720 px sichtbar (`· 7 Tage`), also auch eine Erklärung, warum der Verlauf endet.
- Nachteile: wer die Konvention nicht kennt, findet die Einstellung erst über den Pfeil; unter 720 px ist die Frist nur im Dialog sichtbar.

### Option B: Eigener Schalter „7 Tage“ in der Kopfzeile → Dialog
- Vorteile: Wert und Weg sind auf einen Blick da.
- Nachteile: ein weiteres Element in der dichten Kopfzeile; bei 320 px konkurriert es mit Kennung, Teilnehmer-Schalter und `?` um Platz — eine Kurzform („7 T“) wäre schlechtes Deutsch.

### Option C: Kontextmenü am Eintrag der Seitenleiste „Frist ändern …“ → Dialog
- Vorteile: kanbanD arbeitet mit Kontextmenüs an Karten; kein Platz in der Kopfzeile; auch für nicht geöffnete Channels.
- Nachteile: unsichtbar ohne Rechtsklick oder `Kontextmenü`-Taste; localchatA hat noch keine Menü-Mechanik (neuer Baustein mit eigener Tastaturführung); der aktuelle Wert ist nirgends zu sehen.

### Option D: Feld „Frist“ im Dialog „Channel anlegen“
- Vorteile: kein neuer Dialog.
- Nachteile: ändert nur beim Anlegen — für bestehende Channels ist die Frist nicht wählbar (Fertig-Kriterium verfehlt); widerspricht 4.3 (Name, optional Beschreibung).

## Entscheidung

**Gewählt: Option A**

Es überwiegt, dass der Weg ohne zusätzliche Breite in der Kopfzeile liegt, einer bekannten Konvention folgt und den Wert ab 720 px als Text zeigt; Bestätigung und Fehler folgen dem gebauten Dialog-Muster. In Kauf genommen: unter 720 px sieht man die Frist erst im Dialog, und die Einstellung ist nur für den geöffneten Channel erreichbar.

Regeln:

1. **Auslöser**: in der Zone „Ort“ ist `# name` ein `<button>` mit `aria-haspopup="dialog"`, zugänglicher Name „# name — Channel-Einstellungen“, sichtbar `# name ▾` (`▾` in `kd-header-muted`); Hover `kd-header-surface`, Fokus `lc-focus-header`. Ohne gewählten Channel gibt es ihn nicht.
2. **Frist als Text** (ab 720 px): nach dem Schalter `· 7 Tage` in `lc-small`, `kd-header-muted`, dann die Beschreibung (ADR-0027 Regel 5) — die Frist steht vor der Beschreibung, damit die Ellipse nur die Beschreibung kürzt. `1 Tag` im Singular. Bei `frist_tage: null`: `· ⚠ Frist gesperrt`. Unter 720 px entfallen Frist-Text und Beschreibung gemeinsam. Der Text aktualisiert sich still mit jedem Channel-Wechsel aus dem Strom (keine Ansage).
3. **Dialog** `<dialog aria-labelledby>` „# name — Einstellungen“ mit einem Feld „Aufbewahrungsfrist“: `<input type="number" min="1" max="3650" step="1" inputmode="numeric">`, Einheit „Tage“ dahinter, vorbelegt mit der geltenden Frist; Hinweis „Ältere Tage entfallen. Heute zählt mit. Standard: 7 Tage.“ (über `aria-describedby`). Schalter „Speichern“ (Primär, `kd-accent`) und „Abbrechen“. Live-Prüfung über eine reine Funktion (`fristPruefen`): nur Ziffern, Wert 1–3650, sonst Fehlerzeile „Frist: ganze Zahl von 1 bis 3650 Tagen.“ und `aria-invalid="true"`. `Enter` speichert, `Esc` und „Abbrechen“ schließen, Fokus zurück auf `# name`.
4. **Ablauf**:
   - „Speichern“ sendet `PUT …/frist {tage}` ohne Bestätigung. 200 → Dialog zu, Statuszeile „Frist für # name: 30 Tage.“.
   - 409 `bestaetigen` → **zweiter Schritt im selben Dialog**: Feld schreibgeschützt, Text „Bei 7 Tagen entfallen sofort 23 Tage bis einschließlich 21. Oktober 2026 mit 4.210 Nachrichten. Das lässt sich nicht rückgängig machen.“ (Zahlen und Datum aus `entfaellt`, deutsch formatiert); Schalter „Zurück“ (Fokus darauf, sicherer Standard) und „Frist verkürzen“ (Primär). „Frist verkürzen“ sendet dasselbe mit `bestaetigt: true`; 200 → Dialog zu, Statuszeile „Frist für # name: 7 Tage — 23 Tage mit 4.210 Nachrichten entfallen.“, und der Tab öffnet den Channel neu wie „Zum Neuesten“ ([ADR-0026](0026-verlauf-im-browser-als-fenster-aus-hoechstens-fuenf-bloecken.md) Regel 5), damit er keine gelöschten Nachrichten mehr zeigt. „Zurück“ kehrt zum ersten Schritt mit der Eingabe zurück.
   - 409 `einstellungen-defekt`, 404, 422, Netzfehler → Fehlerzeile im offenen Dialog wie ADR-0024 Regel 5 („Nicht gespeichert — Verbindung prüfen.“); die Eingabe bleibt.
   - Ist `frist_tage` schon beim Öffnen `null`, ist das Feld gesperrt und der Dialog sagt: „⚠ Die Einstellungsdatei `channels/name/einstellungen.toml` ist defekt — in diesem Channel entfällt nichts. Von Hand reparieren und localchatA neu starten.“
5. **Live**: ändert jemand anders die Frist, während der Dialog offen ist, bleibt die Eingabe stehen; der Hinweis zeigt zusätzlich „Inzwischen geändert: 30 Tage.“ Andere Tabs öffnen nichts neu (ADR-0030 Regel 5).
6. **Kein Tastenkürzel** — der Dialog ist selten nötig; der Weg ist Fokus auf `# name` (in der Tab-Reihenfolge der Kopfzeile) und `Enter`.
7. **Designsystem**: die Kopfzeile ergänzt den Schalter und den Frist-Text, der Dialog ist ein neuer Baustein `ChannelEinstellungen` (`components/Kopfzeile/README.md`, `components/ChannelEinstellungen/README.md`); ein Artboard entsteht nicht — der `ux-reviewer` prüft gegen Tokens und diese Regeln.

## Konsequenzen

**Positiv:**
- Kein neues Element, das bei 320 px Platz braucht; der Kopf des Channels wird der eine Ort für Einstellungen, ohne dass I0018 eine Menü-Mechanik baut.
- Die Bestätigung nennt konkrete Zahlen, nicht „Sind Sie sicher?“; ein Agent bekommt dieselbe Sicherung über die API.

**Negativ / Inkauf zu nehmen:**
- Die Frist eines nicht geöffneten Channels lässt sich nur nach dem Öffnen ändern.
- Die 409 `bestaetigen` erzeugt eine Konsolenzeile „Failed to load resource … 409“ — vorgeschriebene Antwort im Sinne des Konsolenkriteriums.

**Pre-Mortem (ein Monat nach dem Start gescheitert, weil …):**
1. … niemand den Pfeil an `# name` als Weg erkennt und die Abnahme „Frist nicht wählbar“ meldet. Früh-Indikator: Entwicklertest ohne Hinweis. Gegenmaßnahme: Frist-Text `· 7 Tage` direkt daneben (Regel 2), Tooltip „Channel-Einstellungen“; Übersicht `?` nennt „Channel-Einstellungen: Klick auf # name“.
2. … `Enter` im ersten Schritt sofort verkürzt, weil der Fokus im zweiten Schritt auf „Frist verkürzen“ landet. Früh-Indikator: E2E „Enter, Enter“ löscht. Gegenmaßnahme: Fokus im zweiten Schritt auf „Zurück“ (Regel 4), E2E prüft, dass zweimal `Enter` nichts löscht.
3. … ein `<input type="number">` im Browser `7,5` oder `1e3` annimmt und der Server 422 liefert, die der Mensch nicht versteht. Früh-Indikator: Node-Test von `fristPruefen` mit `"7,5"`, `"1e3"`, `""`, `"0"`, `"3651"`. Gegenmaßnahme: Live-Prüfung mit derselben Spanne wie der Server, Fehlerzeile vor dem Senden.

**Folgeentscheidungen / Aufgaben:**
- [ ] Erster Node-Test (rein): `fristPruefen` — `"1"`, `"3650"`, `"7"` gültig; `"0"`, `"3651"`, `"-1"`, `"7.5"`, `"7,5"`, `"1e3"`, `""`, `" 7"`, `"+7"` ungültig (Regel: nur Ziffern, Wert 1–3650; führende Nullen wie `"007"` gelten als 7); Mutanten 0/2 und 3649/3651 werden rot. Text der Bestätigung aus `{tage: 23, nachrichten: 4210, bis: "2026-10-21"}` → „… 23 Tage bis einschließlich 21. Oktober 2026 mit 4.210 Nachrichten …“; `{tage: 1, nachrichten: 1}` im Singular.
- [ ] Erster E2E-Test: zwei Kontexte; A öffnet `# allgemein`, Kopfzeile zeigt `· 7 Tage`; Klick auf `# allgemein` → Dialog, Feld 7; 30 + `Enter` → Dialog zu, Statuszeile, Kopfzeile `· 30 Tage`; B zeigt `· 30 Tage` ohne Neuladen spätestens 2 s nach der 200; A setzt 7 bei vorhandenen alten Tagen → zweiter Schritt mit Zahlen, Fokus auf „Zurück“, `Enter` → wieder Schritt 1, nichts gelöscht; „Frist verkürzen“ → Statuszeile mit Zahlen, Verlauf beginnt beim ältesten Tag in der Frist; `Esc` → Fokus auf `# allgemein`; Breite 320 px ohne Querscrollen; Konsole nur mit den vorgeschriebenen Antworten (409 `bestaetigen`).

## Quellen / Bezuege

- [A00005 Weboberfläche](../Architektur/A00005-weboberflaeche.md), [A00004 HTTP-Schnittstelle](../Architektur/A00004-http-schnittstelle.md)
- ADR-0024 (Dialog-Muster), ADR-0026 (Öffnen), ADR-0027 (Kopfzeile, Zone „Ort“), ADR-0029, ADR-0031
- Designsystem `README.md`, `1-tastatur.md`, `components/Kopfzeile/README.md`; `design-prompt.md` 4.2, 4.3; Vision R00000; WBS I0018, D0003

## Ergänzungen

- **2026-10-29 (Karte #27, R00019 F0065, Frage A2 des planer) — Schalter `# name ▾` als Touch-Ziel:** Ergänzt Regel 1. Beleg Ist: `web/lc-seite.css:43` — der Schalter ist 28 px hoch, die Kopfzeile mindestens 40 px (`--kd-header`); 44 px passen ab 480 px nicht in die Leiste.
  1. **Mit grobem Zeiger** (`@media (pointer: coarse)`, dieselbe Unterscheidung wie [ADR-0038](0038-eingabe-mehrzeilig-enter-sendet-entwurf-je-channel-gebuendelte-ansage.md) K3/K5 und R00025): der Schalter ist mindestens 44 × 44 px (`min-height`, `min-width`), Inhalt senkrecht mittig; ab 480 px ist die Kopfzeile dann mindestens 44 px hoch (eigene Mindesthöhe der `.lc-head` in dieser Media-Query — das Token `--kd-header` bleibt 40 px). Unter 480 px liegt der Schalter in Reihe 1 von 44 px ([ADR-0027](0027-channel-waehlen-adresse-als-hash-zuletzt-gewaehlt-je-tab.md), Ergänzung Karte #27); die 80 px teilen sich 44 (Zone „Ort“) + 36 (Bedienung).
  2. **Mit feinem Zeiger** bleibt alles wie heute: Schalter 28 px, Kopfzeile ab 480 px 40 px.
  3. **Nicht hier**: „Teilnehmer n“ (`.lc-hbtn`) bleibt 28 px — unter 480 px fasst Reihe 2 (36 px) kein Ziel von 44 px, ohne die 80 px zu sprengen; 28 px erfüllen WCAG 2.5.8 (24 px), nicht 2.5.5 (44 px). Das ist eine benannte Lücke, kein Versehen.
  - Option A: **44 px über `pointer: coarse`, Kopfzeile wächst auf Touch-Geräten ab 480 px um 4 px** (Empfehlung des planer, **gewählt**). Stärke: dasselbe Muster wie Senden-Knopf und Schalter an Nachrichten (`web/lc-seite.css:132`, `:148`); im Test herstellbar (ADR-0038, Proben P1/P2); Desktop unverändert. Schwäche: auf Touch-Geräten ist die Leiste 44 statt 40 px; ein Touch-Laptop meldet einen feinen Zeiger und bekommt 28 px (wie in ADR-0038 in Kauf genommen).
  - Option B: **unsichtbare Trefferfläche** (Pseudo-Element mit negativem Rand) um den 28-px-Schalter, Leiste bleibt 40 px. Stärke: kein Pixel Layout-Änderung. Verworfen: die Fläche ragt über die Leiste in die Hauptansicht und überdeckt deren oberste Zeile — das Kriterium „Schalter ganz in `.lc-head`, `elementFromPoint` an den vier Ecken“ wäre nicht prüfbar, Überdeckung ist genau der Befund aus ux-reviewer #32.
  - Option C: **Kopfzeile immer 44 px**, auch am Desktop. Stärke: eine Höhe für alle. Verworfen: weicht vom 40-px-Maß des Geschwisters kanbanD ab (Leitplanke „gleiches Bedienkonzept“) und vom Kontrollpaar 1280 × 800 in R00019.
  - Option D: **`any-pointer: coarse`**. Verworfen wie in ADR-0038 K5 Option B: Touch-Laptops mit echter Maus bekämen die Touch-Gestalt; zwei Regeln für „Touch“ im selben Programm.
  - **Erster Test** (E2E, Kriterium „Touch-Ziel“ in R00019): 375 × 667 mit `hasTouch`/`isMobile` → Schalter ≥ 44 × 44, ganz in `.lc-head`, `elementFromPoint` 2 px innerhalb der vier Ecken trifft ihn; 1024 × 768 mit Touch → Kopfzeile ≥ 44 px, einzeilig; 1280 × 800 ohne Touch → Schalter 28 px, Kopfzeile 40 px. Mutanten „44 px ohne Media-Query“ und „Media-Query entfernt“ → rot.

- **2026-10-29 (Karte #27, R00019 F0066/F0083, Frage A3 des planer) — Fehlerzeile des Dialogs: `aria-invalid` nur bei Prüffehlern, wiederholter Fehlversuch, 429 mit Uhrzeit:** Ändert Regel 3 (letzter Satz) und Regel 4, dritter Spiegelstrich. Beleg Ist: Regel 4 behandelt 409 `einstellungen-defekt`, 404, 422 und Netzfehler gleich; der Baustein `ChannelEinstellungen` sagt „Fehler (ungültige Zahl, Netz, defekte Datei) … Feld mit `aria-invalid`“. Das sagt einem Bildschirmleser „Eingabe falsch“, wo der Ordner schreibgeschützt ist (UX-Review #12 Sollte 3).
  1. **Prüffehler** sind genau: die Prüfung vor dem Senden (`fristPruefen` schlägt fehl, auch „ 30“, R00019 F0083) und eine Antwort 422 mit `regel: "frist"`. Nur sie setzen `aria-invalid="true"` am Feld. Jede andere Antwort — 5xx (auch ohne Körper), 429, 404, 409 `einstellungen-defekt`, Netzfehler, Abbruch — zeigt nur die Fehlerzeile mit ⚠; das Feld bleibt ohne `aria-invalid`, die Eingabe steht, der Fokus liegt im Feld. Text: der deutsche `fehler` der Antwort; ohne ihn „Nicht gespeichert (Status <n>) — später erneut versuchen.“; ohne Antwort „Nicht gespeichert — Verbindung prüfen.“
  2. **Wiederholter Fehlversuch**: die Fehlerzeile zählt Versuche mit **gleichem Fehlerschlüssel**, solange sie nicht weggeräumt wurde; ab dem zweiten endet sie auf „(2. Versuch)“, „(3. Versuch)“ … Der Schlüssel ist der angezeigte Text **ohne** Zusatz und, bei 429, ohne die Uhrzeit (Teil vor „ — “) — so zählt eine zweite 429 als zweiter Versuch, auch wenn die Uhrzeit eine Sekunde weiter steht. Neu bei 1 beginnt der Zähler, wenn der Schlüssel wechselt, die Zeile weggeräumt wird (gültige Eingabe, zweiter Schritt, Erfolg) oder der Dialog schließt. Weil sich der Text ändert, sagt die vorhandene Region `role="alert"` ihn neu an — kein zweiter Ansagemechanismus.
  3. **429 mit Uhrzeit** (ändert die Anzeige aus R00021; die Antwort an Agenten bleibt): Fehlerzeile = Teil des `fehler` vor „ — “ + „ — wieder möglich ab HH:MM:SS.“; Zeitpunkt = Eintreffen der Antwort (Uhr des Browsers) + `wieder_in_ms`, auf die volle Sekunde **aufgerundet**, Ortszeit, 24 h, zweistellig, über Mitternacht ohne Datum. Die Rechnung ist eine reine Funktion (Eingabe Eintreffen in ms und `wieder_in_ms`, Ausgabe „HH:MM:SS“), ohne `toLocaleTimeString` — damit hängt sie nicht an der Sprache des Browsers.
  - Zu 1, Option „`aria-invalid` bei jedem Fehler“ (Regel 4 bisher, Baustein bisher) verworfen: markiert die richtige Eingabe als falsch und schickt den Menschen ins Feld statt zum Betreiber. Option „nur clientseitige Prüfung zählt, 422 nicht“ verworfen: die 422 `frist` ist derselbe Prüffehler, nur vom Server gefunden (anderer Browser, umgangene Prüfung).
  - Zu 2, Option „Fehlerzeile kurz leeren und neu füllen“ (Ansage durch Wiederbefüllen) verworfen: ob und wann ein Bildschirmleser das neu ansagt, hängt am Zeitabstand und am Leser, und Sehende merken nichts. Option „Uhrzeit des Versuchs anhängen“ verworfen: jede Zeile wäre neu, der Zähler sagt mit weniger Text dasselbe.
  - Zu 3, Option „Restzeit, die herunterzählt“ („wieder möglich in 9 s … 8 s“) verworfen: eine laufend geänderte Fehlerzeile in `role="alert"` wird jede Sekunde angesagt oder müsste aus der Region heraus — und eine stehende „in 10 s“ veraltet, während der Dialog offen ist (ux-reviewer #32). Schwäche der Uhrzeit, in Kauf genommen: sie gilt für die Uhr des Browsers; weicht die von der Serveruhr ab, ist das egal, weil beide Seiten nur Abstände rechnen.
  - **Erster Test** (Node, rein; ersetzt: nichts): Uhrzeit — 14:03:12,400 + 9.600 ms → „14:03:22“; + 10.000 ms → „14:03:23“; 23:59:58,000 + 3.000 ms → „00:00:01“ (Rechenbeispiele aus R00019 „429 im Dialog“). Zähler — Schlüssel `S`, `S`, `S` → Zusatz —, „(2. Versuch)“, „(3. Versuch)“; `S`, weggeräumt, `S` → beide ohne Zusatz; 429 mit 14:03:22, danach 14:03:23 → „(2. Versuch)“. Prüffehler — `fristPruefen(" 30")` und 422 `frist` → `aria-invalid`; 500, 502 ohne Körper, 404, 409 `einstellungen-defekt`, 429, Abbruch → kein `aria-invalid`. Mutanten „Zusatz fehlt“, „Uhrzeit im Schlüssel“, „abgerundet statt aufgerundet“, „`aria-invalid` bei jedem Fehler“ → rot.
