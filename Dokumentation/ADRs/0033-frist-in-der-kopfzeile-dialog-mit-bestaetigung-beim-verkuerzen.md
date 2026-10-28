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
