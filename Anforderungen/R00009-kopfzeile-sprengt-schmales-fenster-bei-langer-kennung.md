---
id: R00009
status: Neu
datum: 2026-10-28
ursprung: Bug-Report
ursprungslauf: R00003
---

# R00009: Behebung von Querscrollen der Kopfzeile bei langer eigener Kennung im schmalen Fenster

## Beschreibung

Wer mit einem langen Namen beitritt, bekommt im schmalen Fenster eine Seite, die quer scrollt: die Kopfzeile mit der eigenen Kennung `name@herkunft` schrumpft nicht und bricht nicht um. Beobachtet im UX-Review von Karte #6 (2026-10-28): bei 320 px Fensterbreite ergibt eine eigene Kennung mit einem Namen von 13 Zeichen `scrollWidth` 330, mit 32 Zeichen 467 (erwartet je ≤ 320). Die Namensregel erlaubt 32 Zeichen (ADR-0013), die Eingabe ist also gültig. Verletzt ist WCAG 2.2, Erfolgskriterium 1.4.10 (Reflow) und die Zusage „kein Querscrollen bei 320 px“, die R00004 bis R00008 für die Hauptansicht geben.

Reproduktion: Instanz starten, Fenster 320 × 640 px, `/` öffnen, als `abcdefghijklmnopqrstuvwxyz012345` beitreten → `document.documentElement.scrollWidth` ist 467 statt ≤ 320; die Kopfzeile ragt rechts aus dem Fenster.

Zahlt ein auf: [Vision](R00000-vision.md) — Bedienkonzept wie kanbanD, Oberfläche für Menschen im Browser (auch im schmalen Fenster neben der Arbeit).

## Ursachenanalyse

### Root Cause

Drei Regeln in `web/lc-seite.css` legen die Kopfzeile auf eine Zeile in ihrer natürlichen Breite fest:

- `web/lc-seite.css:37` — `.lc-head-bed { white-space: nowrap; display: flex; … }`: der Bedienteil (Kachel, Kennung, Agent-Marke, Schalter „Teilnehmer“) bricht nie um, und als Flex-Kind ohne `min-width: 0` schrumpft `.lc-me` nicht unter seinen Inhalt.
- `web/lc-seite.css:33` und `:43` — die Spalte des Bedienteils ist `auto` (unter 480 px `minmax(0, 1fr) auto`), sie nimmt also die volle Breite der einzeiligen Kennung; dazu `height: var(--kd-header)` — eine feste Höhe von 40 px, in die keine zweite Zeile passt.
- `web/lc-seite.css:59` — `.lc-app { grid-template-rows: var(--kd-header) … }`: auch das Seitenraster hält die Kopfzeile fest auf 40 px.
- `web/lc-seite.css:38` — `.lc-me` hat keine Umbruchregel; eine Kennung ohne Leerzeichen (`[a-z0-9-]`, `@`, Ziffern und Punkte) bietet dem Browser keine Bruchstelle.

Ausgeschlossen: der Instanzname (`.lc-head-inst` kürzt sich mit Auslassung, Zeile 36) und die Navigation (entfällt unter 480 px, Zeile 44).

Warum die Tests es nicht fanden: alle Schmal-Tests (`e2e/tests/r00003-schmal.spec.js`, `r00004-*`, `r00005-restzeit`, `r00006-seitenleiste`) treten als `shoff` bei (Kennung `shoff@127.0.0.1`, 15 Zeichen) und prüfen lange Namen nur in der Teilnehmerliste und der Seitenleiste; B0065 (R00003) prüft 320 px nur vor dem Beitritt.

### Betroffene Komponenten

- `web/lc-seite.css` (Kopfzeile, Seitenraster)
- `web/kopfzeile.js` (Aufbau der Kopfzeile; voraussichtlich unverändert)
- Baustein Kopfzeile des Designsystems (`Dokumentation/Wireframes/designsystem/`, schmal unter 480 px)

## Lösungsvorschlag

### Langfristige Lösung

**Option A — Kennung bricht um, die Kopfzeile wächst mit.** `.lc-me` darf schrumpfen (`min-width: 0`) und bricht an beliebiger Stelle um (`overflow-wrap: anywhere`); der Bedienteil bricht nicht mehr als Ganzes (`white-space: nowrap` nur noch an Kachel, Marke und Schalter); die Kopfzeile ist mindestens 40 px hoch statt fest 40 px (`min-height`), die erste Zeile des Seitenrasters `minmax(var(--kd-header), auto)`. Bei breitem Fenster ändert sich nichts — die Kennung passt in eine Zeile, die Kopfzeile bleibt 40 px.

Rechenbeispiel (Hypothese aus der Schriftgröße, gemessen wird im Test): Mono 12 px ≈ 7,2 px je Zeichen; bei 320 px bleiben nach Innenabstand (20 px), Kachel (20 px), Lücken (2 × 8 px) und dem Schalter „Teilnehmer n“ (≈ 110 px) etwa 150 px ≈ 20 Zeichen je Zeile. Die Kennung `abcdefghijklmnopqrstuvwxyz012345@127.0.0.1` (42 Zeichen) braucht damit drei Zeilen à ≈ 16 px ≈ 48 px — mehr als 40 px, deshalb muss die Höhe nachgeben.

### Alternative Ansätze

| Option | Komplexität | Testbarkeit | Reversibilität | Risiko |
|---|---|---|---|---|
| **A Umbruch, Kopfzeile wächst** (Empfehlung) | gering, nur CSS | hoch: `scrollWidth`, Höhe, Text | hoch | Hauptansicht rückt unter 480 px um eine bis zwei Zeilen nach unten |
| B Kennung unter 480 px in eine zweite Zeile der Kopfzeile | mittel (Raster mit zwei Zeilen, Fokus- und Lesereihenfolge prüfen) | hoch | hoch | bricht trotzdem bei 78 Zeichen (IPv6-Herkunft), braucht also zusätzlich A |
| C Kennung mit Auslassung kürzen, ganze Kennung als `title` | gering | hoch | hoch | verletzt die Abnahme „vollständig lesbar“; `title` ist mit Tastatur und Touch nicht erreichbar |

- B verworfen: löst den langen Fall nicht allein und baut mehr um als nötig.
- C verworfen: die eigene Kennung ist die Identität im Chat — sie muss ganz lesbar sein (Abnahme der Befundkarte).

## Test-Strategie

### Unit Test zur Bug-Reproduktion

Das Verhalten ist Layout und nur im Browser messbar; die Reproduktion ist deshalb ein E2E-Test (Playwright), der vor dem Fix rot ist:

- `e2e/tests/r00009-kopfzeile-schmal.spec.js`, Test „lange eigene Kennung bei 320 px ohne Querscrollen“ — Arrange: frische Instanz, Fenster 320 × 640, Farbschema hell; Act: über den Beitreten-Bildschirm als `abcdefghijklmnopqrstuvwxyz012345` beitreten; Assert: `document.documentElement.scrollWidth <= window.innerWidth`. Heute rot mit 467 > 320.

### Code-Extraktion für isoliertes Testing

Keine — die Ursache liegt in CSS, nicht in Logik. Der Grenzfall „längste mögliche Kennung“ (Name 32 Zeichen, IPv6-Herkunft 45 Zeichen, zusammen 78) ist über einen Beitritt nicht herstellbar (die Instanz lauscht im Test auf `127.0.0.1`); er wird im selben E2E-Test hergestellt, indem die Seite `zeichnen()` aus `/web/kopfzeile.js` mit einem Teilnehmer `{name: "abcdefghijklmnopqrstuvwxyz012345", herkunft: "ffff:ffff:ffff:ffff:ffff:ffff:255.255.255.255", art: "agent"}` in einen eigenen Container aufruft.

### Regressionstests

- E2E: Matrix Fensterbreite 320, 360, 375 px × Farbschema hell, dunkel × Kennung mit Name 13 Zeichen (`abcdefghijklm`), 32 Zeichen (Beitritt) und 78 Zeichen (gezeichnet, Agent mit Marke) → je `scrollWidth <= innerWidth`, ganze Kennung im Text von `.lc-me`, `.lc-me` liegt ganz im Fenster (`getBoundingClientRect().right <= innerWidth`).
- E2E-Kontrollpaar 1280 × 800: dieselbe 32-Zeichen-Kennung → Kopfzeile 40 px hoch, Kennung einzeilig (Höhe von `.lc-me` = eine Zeilenhöhe), wie vor dem Fix.
- Bestehende Schmal- und Kopfzeilen-Tests aus R00003 bis R00006 bleiben grün (`r00003-schmal`, `r00003-fokus`, `r00004-*`, `r00006-seitenleiste`).

## Akzeptanzkriterien

- [ ] Der Reproduktionstest ist ohne Fix rot und mit Fix grün
- [ ] Der ursprüngliche Fehler tritt im Szenario der User Story nicht mehr auf
- [ ] Keine neuen Fehler: alle Tests aller Ebenen grün (Gate nach `AI-Team/umgebung.md`)
- [ ] **Abnahme der Befundkarte #20** (wörtlich): Beitritt als `abcdefghijklmnopqrstuvwxyz012345` bei 320 px → `scrollWidth ≤ innerWidth`, hell und dunkel; Kennung bleibt vollständig lesbar (Umbruch erlaubt). Dazu ein Kontrollpaar bei 1280 px unverändert.
- [ ] „Vollständig lesbar“ heißt: der Text von `.lc-me` ist genau `abcdefghijklmnopqrstuvwxyz012345@127.0.0.1` (42 Zeichen), kein Teil ist abgeschnitten oder verdeckt (`.lc-me` hat `scrollWidth <= clientWidth`, liegt ganz im Fenster, keine Auslassung), die Herkunft bleibt gedämpft.
- [ ] „Unverändert bei 1280 px“ heißt: Kopfzeile 40 px hoch, Kennung in einer Zeile, `scrollWidth ≤ innerWidth` — je hell und dunkel.
- [ ] Spanne der Kennungslänge bei 320, 360 und 375 px, hell und dunkel: 13, 32 und 78 Zeichen Name samt Herkunft wie in den Regressionstests → je kein Querscrollen; der Schalter „Teilnehmer“ und bei Agenten die Marke „Agent“ bleiben sichtbar und mit `Tab` erreichbar.
- [ ] Rechenbeispiel Querscrollen: `scrollWidth` 330 bei `innerWidth` 320 ist rot (330 > 320), 320 bei 320 grün.

## Implementierungshinweise

- Nur `web/lc-seite.css`; kein neues Token, die Kopfzeile behält `--kd-header` als Mindesthöhe.
- Die Abweichung „Kopfzeile wächst unter 480 px mit der Kennung“ ist eine Ergänzung zum Baustein Kopfzeile (schmal); im Kommentar über der Media-Query nennen.
- Die Teilnehmerliste unter 480 px und Dialoge, die sich an der Kopfzeilenhöhe ausrichten, gegen die gewachsene Kopfzeile prüfen (E2E bestehend).

## Offene Fragen

- Keine. Entschieden im Einsatz (Bericht an den Director): Option A, weil nur sie die Abnahme „vollständig lesbar“ bis zur längsten Kennung hält.

## Warum löst diese Anforderung das Problem? (Pflicht)

Auslöser ist der UX-Befund: eine Entwicklerin mit dem erlaubten 32-Zeichen-Namen `abcdefghijklmnopqrstuvwxyz012345` muss im schmalen Fenster quer scrollen, um ihre eigene Kennung und den Schalter „Teilnehmer“ zu sehen. Ursache ist, dass Kopfzeile, Bedienteil und Kennung auf eine Zeile in natürlicher Breite festgelegt sind (`nowrap`, Spalte `auto`, feste Höhe 40 px). Wenn die Kennung schrumpfen und umbrechen darf und die Kopfzeile in der Höhe nachgibt, dann passt der Bedienteil in jede Breite ab 320 px, die Seite scrollt nicht mehr quer, und die Kennung bleibt ganz lesbar. Der Hebel liegt genau an diesen drei CSS-Regeln, weil nur die Kopfzeile breiter als das Fenster wird — Seitenleiste, Liste und Dialoge halten 320 px bereits (R00004–R00008). Die Matrix mit 78 Zeichen und das Kontrollpaar bei 1280 px verhindern, dass der Fix nur das gemeldete Beispiel trifft oder die breite Ansicht verändert.

## Notizen

- Priorität: Mittel (Barrierefreiheit, WCAG 1.4.10; trifft jeden Namen ab etwa 12 Zeichen bei 320 px)
- Betroffene Nutzer/Systeme: Menschen im Browser mit langem Namen in schmalen Fenstern (Telefon, geteilter Bildschirm)
- Workaround: Fenster breiter ziehen oder kürzeren Namen wählen
- Herkunft: Befundkarte #20, UX-Review Karte #6 (I0007), 2026-10-28; Ursprung I0004 / R00003 (F0010, B0060 Eigene Kennung in der Kopfzeile)
- WBS: F0039 unter I0004 mit B0202, B0203

### Bubbles

- F0039: B0202 Schmal-Matrix der eigenen Kennung · B0203 Kennung in der Kopfzeile umbrechen
