---
id: R00048
status: Neu
datum: 2026-10-30
ursprung: Bug-Report
ursprungslauf: R00011
---

# R00048: Behebung von „Wachsende Eingabe verdeckt die letzte Nachricht“

## Beschreibung

Wer am Telefon (375 × 667, 320 × 568) mitliest und eine mehrzeilige Nachricht tippt, verliert die letzte Nachricht aus dem Blick: das Feld wächst von einer auf drei Zeilen, der Verlauf wird oben und unten kürzer — gemessen schrumpft seine Unterkante von 585 auf 549 px —, `scrollTop` bleibt, und die letzte Nachricht (Unterkante 573 px) liegt danach unter der Eingabe. Erwartet (R00011: das Feld „wächst mit dem Inhalt“, Lesende werden nie aus ihrer Stelle gerissen): die Nachricht, die vorher ganz sichtbar war, bleibt es; beim Schrumpfen des Felds springt nichts.

Abnahme-Probe wörtlich (Karte #80, ux-reviewer-Vorprüfung #78/#79, S8): **Eingabe wächst von 1 auf 3 (und 5) Zeilen bei 375 × 667 und 320 × 568 → letzte Nachricht bleibt vollständig sichtbar, wenn sie vorher sichtbar war (`scrollTop` folgt), kein Springen beim Schrumpfen.**

Zahlt ein auf: [Vision](R00000-vision.md) — „Zielgeräte“ (Telefon) und „Live überall“: wer mitliest und antwortet, behält den Gesprächsstand vor Augen. Kein Widerspruch zur Vision. WCAG 1.4.13 / Heuristik 1 (Sichtbarkeit des Systemzustands).

Weiche (`rollen/planer/befunde-einplanen.md`): **falsch gebaut** — R00011 sagt zu, dass das Feld mit dem Inhalt wächst und Lesende nicht aus ihrer Stelle gerissen werden; der Slice F0043 hält das beim Wachsen der Eingabe nicht. Kein neuer Umfang.

Slice (WBS `Dokumentation/Planung/localchata.md`, unter I0009 Nachricht senden): F0164 Wachsende Eingabe verdeckt die letzte Nachricht nicht.

### Domänenbezug

Kein Glossar im Projekt (`Dokumentation/Domaenenwissen/` fehlt). Begriffe wie in R00011 und R00020: Verlauf, Feld (Eingabe unter dem Verlauf), Ende des Inhalts, Auslauf, Lesestelle.

## Ursachenanalyse

### Root Cause

**Hypothese** (gelesen, nicht ausgeführt; die Messung des ux-reviewers belegt die Wirkung, nicht die Stelle):

- Das Feld ändert seine Höhe in `hoeheAnpassen` (`web/eingabe.js:207`–`213`) über `style.height`; der Verlauf (`.lc-verlauf`, `flex: 1`, `web/lc-seite.css:104`) wird dadurch kürzer, `log.clientHeight` schrumpft.
- Der `ResizeObserver` des Verlaufs (`web/verlauf.js:90`–`96`) führt nur nach, wenn `amEnde` gilt (`web/verlauf.js:67`, `lageMerken` `web/verlauf.js:70`–`81`: Abstand zum Ende ≤ 1 px und kein neues Fenster). Wer bei der Messung knapp über dem Ende stand (letzte Nachricht sichtbar, aber nicht auf 1 px am Ende, etwa durch den Auslauf, R00020 F0067, oder eine Zeilenhöhe Abstand, wie R00011 es für „folgt neuen Nachrichten“ erlaubt), wird nicht nachgeführt: `scrollTop` bleibt, die Unterkante rückt um die Höhendifferenz nach oben, die letzte Nachricht liegt unter dem Feld.
- Das Zurückschrumpfen ist der Gegenfall: ohne Nachführung rückt der Inhalt um dieselbe Differenz nach unten; mit einer Nachführung nur beim Wachsen würde er springen.
- Ausgeschlossen (nicht geprüft am Lauf): ein Fehler der Höhenberechnung des Felds; die Messung zeigt `scrollTop` unverändert bei kleinerer Sichthöhe.
- Latent: dieselbe Lücke trifft jede andere Änderung der Sichthöhe des Verlaufs — Tippanzeige (F0162, eine Zeile mehr an der Eingabe), Antwortzeile (I0044), Hinweiszeile unter dem Feld, Bildschirmtastatur, Fensterhöhe. Die Behebung gehört deshalb in den Verlauf, nicht in das Feld.

Vor dem Bau: Der Reproduktionstest (E2E) zeigt die Ursache; stimmt die Hypothese nicht, gilt der Ausführungspfad, den er zeigt.

### Betroffene Komponenten

`web/verlauf.js` (ResizeObserver, `lageMerken`, `ankerHalten`), `web/live-verlauf.js` (reine Funktionen), Baustein A00005 Weboberfläche; `web/eingabe.js` und `web/lc-seite.css` nur lesend; kein Server, keine API.

### Testlücke

R00011 prüft „Höhe und Schmal“ mit 20 Zeilen (Feld höchstens 200 px, `scrollWidth <= innerWidth`), aber nie die Lage des Verlaufs beim Wachsen; die Folgen-Tests (Z5, „Lesestelle nach dem Sprung“) ändern die Sichthöhe nie.

## Lösungsvorschlag

### Langfristige Lösung

Option A (Empfehlung): **Die Unterkante des Verlaufs halten, unabhängig von `amEnde`.** Ändert sich `clientHeight` des Verlaufs, ohne dass sich der Inhalt ändert, wird `scrollTop` um die Differenz nachgeführt (Abstand der Lesestelle zur Unterkante bleibt gleich); die Rechnung ist eine reine Funktion in `web/live-verlauf.js`. Wachsen und Schrumpfen laufen über dieselbe Rechnung und sind damit symmetrisch (kein Springen beim Schrumpfen). Am Ende (`amEnde`) bleibt es beim bisherigen Weg `ansEnde`.

### Alternative Ansätze

- B: `amEnde` mit Toleranz von einer Zeilenhöhe (wie `folgenOderZaehlen`) — behebt nur den Fall knapp über dem Ende; wer weiter oben liest, wird weiter verschoben (nicht nur sichtbar-letzte-Nachricht, aber Lesestelle).
- C: Feld als Überlagerung über dem Verlauf (Verlauf behält seine Höhe, bekommt unten Platz in Höhe des Felds) — ändert die Layout-Grundlage von ADR-0026/0038, Risiko für Auslauf und Scrollbalken; Aufwand deutlich höher.
- D: Verlauf nach jedem `input` zum Ende springen — reißt Lesende aus ihrer Stelle, verletzt R00011 „Nicht am Ende: nichts scrollt“.

## Test-Strategie

### Unit Test zur Bug-Reproduktion

Reine Funktion `scrollTopNachHoehe({ scrollTop, hoeheVorher, hoeheNachher, inhaltsHoehe })` (Node-Test, Mutanten): 667-px-Fenster, Höhe 584 → 548 (Abstand Lesestelle zur Unterkante bleibt, `scrollTop` +36); zurück → −36; Grenzen: Ergebnis nie < 0 und nie > `inhaltsHoehe − hoeheNachher`; unveränderte Höhe → unverändert. Ohne die Funktion rot (fehlt), ohne Nachführung im Observer der E2E rot.

### Code-Extraktion für isoliertes Testing

Die Rechnung steht in `web/live-verlauf.js` (rein), der Observer in `web/verlauf.js` ruft sie nur.

### Regressionstests

- E2E (echtes Programm, Playwright): Telefon 375 × 667 und 320 × 568 mit `hasTouch`, `isMobile`; `allgemein` mit 30 Nachrichten; (a) Lesestelle am Ende, (b) Lesestelle eine Zeilenhöhe über dem Ende, (c) Lesestelle in der Mitte; Feld wächst über `Shift Enter` von 1 auf 3 und 5 Zeilen: die letzte Nachricht, die vorher ganz sichtbar war, hat danach `bottom <= Oberkante der Eingabe`, `scrollTop` folgt; Feld schrumpft (Text löschen, Senden): die Lesestelle steht wieder auf ± 1 px dort wie vorher.
- Bestehende Folgen-, Z5-, Sprung- und Auslauf-Tests (R00011, R00020, R00026) bleiben grün.
- Beide Farbschemata sind nicht Gegenstand (reines Layout).

## Akzeptanzkriterien

- [ ] Der Reproduktionstest (Unit der reinen Funktion und E2E) ist ohne Fix rot und mit Fix grün
- [ ] **Abnahme-Probe wörtlich**: Eingabe wächst von 1 auf 3 und auf 5 Zeilen bei 375 × 667 und bei 320 × 568 → die letzte Nachricht bleibt vollständig sichtbar (`bottom` ≤ Oberkante der Eingabe), wenn sie vorher sichtbar war; `scrollTop` folgt der Höhenänderung (± 1 px)
- [ ] Kein Springen beim Schrumpfen: nach dem Zurück auf eine Zeile (Text gelöscht oder gesendet) steht die zuvor sichtbare Nachricht auf ± 1 px an ihrer Ausgangsstelle, solange der Inhalt dieselbe Länge hat
- [ ] Nicht am Ende (Z5): ändert sich die Höhe der Eingabe, bleibt die Lesestelle (Abstand zur Unterkante des Verlaufs) auf ± 1 px; die Pille zeigt dasselbe wie vorher
- [ ] Keine neuen Fehler: alle Tests aller Ebenen grün, Konsole ohne Ausnahme

## Implementierungshinweise

Beobachten von `log` hat der `ResizeObserver` schon (`web/verlauf.js:96`); `clientHeight` des Vorgängerstands wird dort gemerkt. Änderungen des Inhalts (`inhalt`, `ausstehend`) behalten ihren Weg. Reihenfolge im Callback: erst Nachführen, dann `lageMerken`. Offen: wirkt das Klemmen an `0` bzw. am Maximum, geht die Rechnung beim Zurückschrumpfen nicht auf — der E2E zeigt, ob eine Ausgangslage gemerkt werden muss.

## Offene Fragen

- Reicht die Nachführung der Unterkante (Option A), oder soll bei Lesestelle weit oberhalb des Endes bewusst nichts passieren? Empfehlung: Option A überall, weil die Lesestelle sich sonst unter dem Feld bewegt; Entscheidung bei Bau, kein Richtungsthema.

## Warum löst diese Anforderung das Problem? (Pflicht)

Ursache: Der Verlauf führt `scrollTop` nur nach, wenn er auf 1 px am Ende steht; verkürzt das wachsende Feld seine Sichthöhe, bleibt der Inhalt stehen und rutscht unter das Feld. Fix: dieselbe Rechnung für jede Änderung der Sichthöhe, die den Abstand der Lesestelle zur Unterkante hält. Wirkung: die Nachricht, die man las, bleibt sichtbar — beim Wachsen und beim Schrumpfen, auf jedem Gerät und auch bei den kommenden Zeilen an der Eingabe (Tippanzeige, Antwortzeile).

## Notizen

- Priorität: Hoch (Telefon ist Zielgerät; Antworten auf eine Nachricht ist der Hauptweg)
- Betroffene Nutzer/Systeme: Mitleser und Schreiber am Telefon, 375 × 667 und 320 × 568; auch Desktop bei niedrigem Fenster
- Workaround: Fenster höher machen oder mit `Ende` ans Ende springen
- Mini-Entwickler: ja (zwei Bubbles, ein Baustein, Probe vorgegeben)
