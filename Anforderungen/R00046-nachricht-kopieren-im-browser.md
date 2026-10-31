---
id: R00046
status: Neu
datum: 2026-10-30
---

# R00046: Nachricht kopieren im Browser

## Beschreibung

Ein Teilnehmer kopiert im Browser den Text einer Nachricht mit einem Griff in die Zwischenablage — an jeder bestätigten, nicht gelöschten Nachricht, auch an fremden und an Nachrichten von Agenten. Kopiert wird der Text so, wie er geschrieben wurde (Markdown-Quelltext), ohne Absender, Zeit, Marken und Anhänge. Die Aktion steht in der Aktionsleiste (Maus), im Schalter „⋯“ (Touch) und im Tab-Weg; die Taste `C` auf der fokussierten Nachricht ist nur Abkürzung.

Zahlt ein auf: [Vision](R00000-vision.md) — „Nachrichten kopieren und mit Zitat beantworten“ (neuer Zielbild-Punkt, 2026-10-30), dazu „Markdown in Nachrichten“ (der Quelltext ist das Geschriebene) und die Leitplanke „Zielgeräte“ (Maus, Tastatur, Touch; jede Aktion auch ohne Tastenkürzel).

Herkunft: Karte #78 — Auftraggeber 2026-10-30: „Ok, sag ihm bitte auch, dass ich gerne eine einfache Kopieren-Button-Funktion an den Nachrichten hätte. Es gibt in üblichen Chat-Anwendungen die Mit KI chatten übliche Funktionen: Kopieren, Darauf antworten / mit Zitat antworten. Es wäre schön wenn sowas ermöglicht werden könnte. In Microsoft Teams geht sowas.“; entschieden vom product-owner in `AI-Team/po-entscheidungen.md` Zeilen 142, 144, 145 und — nach der UX-Vorprüfung — 152–154; die Vorprüfung des `ux-reviewer` (Kommentar Karte #78/#79, 2026-10-30) ist eingearbeitet.

Umfang (WBS `Dokumentation/Planung/localchata.md`): **I0042** Nachricht kopieren (D0003 Channels). Stand nach ADR-0052: I0042 ist Blatt der Steuerungsebene (kein Feature) mit den Bubbles B0699–B0703. Das Gegenstück „Antworten“ steht in [R00047](R00047-mit-zitat-antworten-api-und-browser.md).

### Domänenbezug

Kein Glossar im Projekt (`Dokumentation/Domaenenwissen/` fehlt). Begriffe wie in R00041: Nachricht, bestätigt (nicht „wird gesendet …“, nicht fehlgeschlagen), gelöscht, Aktion, Aktionsleiste, Schalter „⋯“ (Name „Aktionen“), Ansage-Region, eigene Nachricht, Anhang. Neu und hier festgelegt: **Quelltext** (der Text der Nachricht, wie ihn der Absender geschrieben hat, vor der Darstellung als Markdown).

## Geschäftlicher Nutzen

Wirkungskette: Ein Agent antwortet im Channel mit einem Befehl oder einem Codeblock → der Mensch will ihn in sein Terminal oder in den Prompt eines anderen Agenten übernehmen → ohne Aktion markiert er den Text von Hand, trifft dabei den Rand des Blocks schlecht und kopiert Formatierung statt Quelltext (auf dem Telefon fast nicht möglich) → mit „Kopieren“ landet genau der geschriebene Text in der Zwischenablage, auf jedem Zielgerät und aus fremden wie eigenen Nachrichten.

## Funktionale Anforderungen

Mechanik laut ADR-0052: der Text kommt aus dem Fenster (nicht aus dem DOM), kopiert wird synchron in der Geste über ein verstecktes Textfeld und `document.execCommand('copy')`, die Clipboard-API ist nur der zweite Weg. Es gibt keinen Server-Teil. [B0699–B0703]

### I0042 Nachricht kopieren

- **Wo die Aktion steht**: an jeder bestätigten, nicht gelöschten Nachricht mit Text — eigene, fremde, von Agenten. Nicht angeboten wird sie an einer gelöschten Nachricht, an einer Nachricht nur aus Anhängen (Text leer), an einer Nachricht „wird gesendet …“ und an einer fehlgeschlagenen.
- **Erreichen**: Maus — „Kopieren“ in der Aktionsleiste bei Hover **und** `:focus-within` (Ort wie R00041); Reihenfolge in Leiste und Menü **Antworten · Kopieren · Bearbeiten · Löschen** (Antworten kommt mit R00047, nicht anwendbare Einträge entfallen und die übrigen rücken auf); Tastatur — `Tab` von der fokussierten Nachricht zu ihren Aktionen, `Enter` oder Leertaste löst aus; Abkürzung `C`, ausgewertet über `event.key` (nicht `code`; `c` und `C` bei Feststelltaste ohne Umschalt kopieren), ohne Strg, Alt, Meta und Umschalt, auf der fokussierten Nachricht und **wirkungslos** bei offenem `dialog[open]`, bei Fokus auf einem Knopf oder einem Zitat, im Eingabefeld, im Bearbeiten-Feld und bei markiertem Text (dann gilt das gewohnte Kopieren des Browsers); `Strg+C` bleibt unberührt; Touch — Schalter „⋯“ (Name „Aktionen“, ≥ 44 × 44 px) mit Menüeintrag „Kopieren“ (≥ 44 × 44 px). Die Taste ist nie der einzige Weg. Der Codeblock-Knopf aus dem Designsystem heißt „Code kopieren“ und hat keine Taste (PO 153). Optional: `title` „Kopieren (C)“.
- **Was kopiert wird**: genau der Quelltext der Nachricht — ein- und mehrzeilig, mit Markdown-Zeichen, Codeblöcken, Emoji-Kurznamen so, wie gespeichert —, ohne Absender, Zeit, Marken („bearbeitet“) und Anhänge. Eine Nachricht mit Text und Anhang kopiert den Text. Nichts wird ergänzt oder abgeschnitten, auch nicht am Ende (kein angehängter Zeilenumbruch).
- **Auch im LAN**: die Seite läuft dort über `http` ohne sicheren Kontext, in dem `navigator.clipboard` fehlt; „Kopieren“ gelingt trotzdem, weil `execCommand` der Hauptweg ist (ADR-0052: im LAN der getestete Normalfall, nicht ein Rückfall). Die Kriterien prüfen die Wirkung unter `localhost` **und** unter `http://<LAN-IP>`; auf einem echten Telefon (iOS Safari, WebKit) und in Firefox ist der Weg im Gate **nicht** belegt (Playwright nur Chromium) — dafür gilt der Entwicklertest (siehe Akzeptanzkriterien).
- **Rückmeldung** (festgelegt in der Vorprüfung): gelingt es, wechselt die Beschriftung des Knopfes für **2000 ms** auf „✓ Kopiert.“; die Breite des Knopfes ist dafür vorab reserviert (die Nachbarknöpfe verschieben sich um 0 px); der sichtbare Wechsel ist `aria-hidden`, der zugängliche Name bleibt „Kopieren“, und die vorhandene Ansage-Region sagt genau einmal „Kopiert.“. Gelingt es nicht, steht für **5000 ms** „⚠ Kopieren nicht möglich“ in `kd-danger` am Knopf, und die Region sagt „Kopieren nicht möglich. Text markieren und Strg+C drücken.“. Das Kopieren ändert weder Fokus noch Lesestelle.
- **Touch-Menü** (K1, K2): das Menü des Schalters „⋯“ liegt bei der **letzten** und der **ersten sichtbaren** Nachricht vollständig im `visualViewport` — bei 375 × 667 und bei 375 × 367 (Höhe mit Bildschirmtastatur); das Öffnen scrollt den Verlauf nicht. `Esc`, ein Tipp daneben und ein zweites Tippen auf „Aktionen“ schließen es; der Fokus geht auf „Aktionen“ zurück, `aria-expanded` wird `false`. Auf Touch wechselt der Menüeintrag auf „✓ Kopiert.“ bzw. „⚠ Kopieren nicht möglich“, und das Menü bleibt dabei **1200 ms** offen (danach schließt es wie oben).
- **Kosten auf Touch** (PO 152, bewusst): „⋯“ steht an jeder bestätigten, nicht gelöschten Nachricht, daher ist jeder Einzeiler auf Touch 44 px hoch — bei 375 × 667 sind etwa 10 statt 19 Zeilen sichtbar; Maus und Tastatur behalten die Dichte. Verworfen: Aktionen nur an der angetippten Nachricht.
- **Leiste** (Sollte): Leistenknöpfe mindestens 24 × 24 px und innerhalb der Zeile der Nachricht (die Zeile wächst nicht); Text in `kd-ink` mit Kontrast ≥ 4,5 : 1 auf `kd-surface-2`; die Breite von `.lc-body` ändert sich durch die Leiste (Maus und Tastatur) höchstens um 4 px. Auf Touch gilt das nicht: dort reserviert der Schalter „⋯“ rechts bewusst 48 px (≥ 44 px Zielfläche), das ist ein gewollter Preis (product-owner Zeile 157, Karte #85).
- **Bestehendes bleibt**: „Bearbeiten“ und „Löschen“ stehen weiter nur an eigenen Nachrichten, `E` und `Entf` wirken nur auf der fokussierten eigenen (R00041); die Leiste und der Schalter erscheinen jetzt an jeder bestätigten, nicht gelöschten Nachricht, weil dort „Kopieren“ gilt.

## Nicht-funktionale Anforderungen

- **Bedienbarkeit** (festgelegt in der Vorprüfung des `ux-reviewer`; Ort, Wortlaut, Dauer und Kontrast stehen in den funktionalen Anforderungen und im Designsystem `components/Nachricht/README.md`): Touch-Ziele ≥ 44 × 44 px (Leitplanke Zielgeräte, 8 px Abstand im Menü wie R00041), Leistenknöpfe mit Maus ≥ 24 × 24 px, Kontrast ≥ 4,5 : 1, Trefferfläche an gruppierten einzeiligen Nachrichten ohne Verschieben des Textes, Taste `C` ohne Kollision mit Strg+C, Eingabe, Markierung und Dialogen.
- **Zeit**: die Rückmeldung erscheint binnen 200 ms nach dem Auslösen (Hypothese des planer: das Kopieren ist lokal; der `ux-reviewer` hat nichts anderes gefordert; erwartet wenige ms).
- **Sicherheit**: es wird nichts an den Server gesendet und nichts gespeichert; die Zwischenablage wird nur beschrieben, nie gelesen; Text einer fremden Nachricht darf nur in die Zwischenablage, nie als HTML (kein Einschleusen); Gate `security-reviewer`.
- **Last**: kein Einfluss auf Lastprofil, Plattenquote oder Betriebsprotokoll (keine Anfrage, keine Zeile).

## Akzeptanzkriterien

Gemeinsam: Start mit `localchatA <ordner> --port 0 --host 0.0.0.0 --kein-browser` auf einem Wegwerf-Ordner mit dem Channel `allgemein`; `shoff` und `anna` in je einem Browser-Fenster (Desktop 1280 × 800, Telefon 375 × 667 mit Touch), Agent `claude` über die API; der Test liest die Zwischenablage über Playwright (`navigator.clipboard.readText()` unter localhost mit Berechtigung; unter `http://<LAN-IP>` über ein eingefügtes Textfeld und `Strg+V`). Die Szenarien stehen in [R00046](user-stories/R00046.md).

- [ ] **Aktion an eigener und fremder Nachricht** (E2E): `shoff` sendet `Hallo`, `claude` sendet `Ergebnis` → bei `shoff` steht an beiden Nachrichten „Kopieren“ (Hover); an der eigenen zusätzlich „Bearbeiten“ und „Löschen“, an der von `claude` nicht.
- [ ] **Quelltext genau** (E2E): `claude` sendet einen mehrzeiligen Text aus vier Zeilen — `Ein **fett** und ein Wort`, eine Leerzeile, ein Codeblock (Zaun mit `sh`, Zeile `ls -la`, Zaun) und `Ende` — → nach „Kopieren“ ist die Zwischenablage zeichengleich dem gesendeten Text (gleiche Länge, `===`), ohne `claude`, ohne Uhrzeit, ohne „bearbeitet“, ohne angehängten Zeilenumbruch.
- [ ] **Über http im LAN** (E2E): dasselbe Szenario über `http://<LAN-IP>:<port>` (nicht localhost; dort ist `window.isSecureContext` falsch) → Zwischenablage zeichengleich, „Kopiert.“ erscheint.
- [ ] **Tastatur** (E2E): fokussierte fremde Nachricht, `Tab` → Fokus auf „Kopieren“, `Enter` → kopiert; Leertaste ebenso; zurück auf die Nachricht, `C` → kopiert, mit Feststelltaste (`key === "C"`, Umschalt nicht gedrückt) ebenso; `Strg+C`, `Alt+C`, `Meta+C`, `Umschalt+C` auf der Nachricht kopieren nichts über die Aktion; die Taste wertet `event.key`, nicht `code` (Test mit Tastaturbelegung, in der die Taste an anderer Stelle sitzt: Ereignis mit `key: "c"` und anderem `code` kopiert).
- [ ] **`C` nie im Eingabefeld, im Dialog, am Knopf oder bei Markierung** (E2E): im Eingabefeld getippt, ergibt `C` den Buchstaben `c` und lässt die Zwischenablage unverändert; im Bearbeiten-Feld ebenso; bei offenem `dialog[open]` (Test öffnet einen) ist `C` wirkungslos; steht der Fokus auf einem Knopf (z. B. „Kopieren“ oder „Aktionen“) oder auf einem Zitat, ist `C` wirkungslos; markiert der Test mit der Maus ein Wort der fokussierten Nachricht und drückt `C`, bleibt die Zwischenablage unverändert; `Strg+C` danach kopiert wie im Browser üblich genau die Markierung.
- [ ] **Touch** (E2E, Telefon): Tippen auf „Aktionen“ („⋯“, `aria-expanded` danach `true`) an der fremden Nachricht → Menü mit „Kopieren“; Tippen darauf → kopiert; Schalter und Eintrag je ≥ 44 × 44 px, `document.elementFromPoint` an Mitte und vier Ecken (1 px nach innen) liefert sie selbst oder ein Kind, auch bei drei gruppierten einzeiligen Nachrichten; höchstens ein Menü offen.
- [ ] **Touch-Menü vollständig sichtbar** (E2E, Telefon, K1): bei 375 × 667 **und** 375 × 367 liegen Menü und Einträge der **letzten** und der **ersten sichtbaren** Nachricht vollständig im `visualViewport` (`elementFromPoint` an Mitte und vier Ecken liefert den Eintrag selbst oder ein Kind); das Öffnen ändert `scrollTop` des Verlaufs nicht (Toleranz 0 px); `Esc`, ein Tipp daneben und ein zweites Tippen auf „Aktionen“ schließen das Menü, der Fokus steht danach auf „Aktionen“, `aria-expanded` ist `false`.
- [ ] **Rückmeldung auf Touch** (E2E, Telefon, K2): nach dem Tippen auf „Kopieren“ steht im Menüeintrag „✓ Kopiert.“ (bei erzwungenem Fehlschlag „⚠ Kopieren nicht möglich“), das Menü ist nach 1000 ms noch offen und nach 1500 ms geschlossen.
- [ ] **Rückmeldung** (E2E): nach dem Kopieren steht am Knopf für 2000 ms (± 300 ms Messtoleranz) „✓ Kopiert.“, der zugängliche Name des Knopfes ist weiter „Kopieren“, der sichtbare Wechsel ist `aria-hidden`, die Breite des Knopfes und die Lage seiner Nachbarknöpfe ändern sich um 0 px, und die Ansage-Region wechselt genau einmal auf „Kopiert.“; wiederholtes Kopieren derselben Nachricht innerhalb 2 s ändert die Region nicht erneut; ein erzwungener Fehlschlag (Test verbietet beide Wege) zeigt 5000 ms lang „⚠ Kopieren nicht möglich“ am Knopf in `kd-danger` und sagt in der Region „Kopieren nicht möglich. Text markieren und Strg+C drücken.“, die Zwischenablage ist unverändert.
- [ ] **Leiste** (E2E, Sollte): die Reihenfolge der Einträge in Leiste und Menü ist Antworten · Kopieren · Bearbeiten · Löschen (ohne R00047 beginnt sie mit Kopieren); Leistenknöpfe sind mit Maus mindestens 24 × 24 px und liegen innerhalb der Zeilenhöhe (die Höhe der Zeile ändert sich nicht); der Text der Knöpfe hat gegen `kd-surface-2` ein Kontrastverhältnis ≥ 4,5 : 1 (berechnet aus den Farbwerten); die Leiste steht bei Hover und bei `:focus-within`; die Breite von `.lc-body` ändert sich durch die Leiste bei Hover und `:focus-within` um höchstens 4 px; auf Touch reserviert der Schalter „⋯“ bewusst 48 px rechts (product-owner Zeile 157), geprüft wird dort nur, dass der Schalter ≥ 44 × 44 px groß ist und keinen Text verdeckt.
- [ ] **Nicht angeboten** (E2E): gelöschte Nachricht (`shoff` löscht eine eigene) → weder Leiste noch „Aktionen“ noch „Kopieren“, `C` kopiert nichts; Nachricht nur aus Anhängen → kein „Kopieren“, `C` kopiert nichts; Nachricht mit Text `Bild` und Anhang → „Kopieren“ kopiert `Bild`; Nachricht „wird gesendet …“ (Senden im Test verzögert) und fehlgeschlagene → kein „Kopieren“.
- [ ] **Bearbeiten und Löschen unverändert** (E2E): an Nachrichten von `anna` und `claude` gibt es bei `shoff` kein Element, dessen zugänglicher Name „Bearbeiten“ oder „lösch“ enthält; `E` und `Entf` auf ihnen tun nichts; an eigenen Nachrichten bleiben beide wie in R00041.
- [ ] **Nichts geht zum Server** (E2E, Netzbeobachtung): beim Kopieren entsteht kein Netzaufruf; `sha256sum` über alle Dateien unter `channels/` ist davor und danach gleich.
- [ ] **Kein Rückschritt für Fokus und Lesestelle** (E2E): nach dem Kopieren steht der Fokus unverändert (auf der Nachricht bzw. auf dem Knopf, von dem aus ausgelöst wurde) und der Verlauf hat nicht gescrollt (Position der Nachricht, Toleranz 0 px).
- [ ] **Playwright-Test** mit Zwischenablage-Prüfung ist Teil der E2E-Suite und grün. Die Zwischenablage gehört dem Browser-Prozess, nicht dem Kontext: die Kopieren-Tests laufen **seriell** (eigene Playwright-Datei mit `test.describe.configure({ mode: 'serial' })` oder eigenem Worker) und schreiben vor jeder Prüfung einen Wächtertext (z. B. `WAECHTER-78`) in die Ablage, damit ein Rest aus einem früheren Test nicht als Erfolg zählt; die Prüfungen „Quelltext genau“ und „Über http im LAN“ lesen die Ablage per `Strg+V` in ein Prüffeld.
- [ ] **Entwicklertest auf einem echten Telefon** (iOS Safari, Phase `entwicklertest`): die Seite wird über `http://<LAN-IP>:<port>` geöffnet, „Kopieren“ am Schalter „⋯“ und der Text im Notizen-Programm eingefügt — zeichengleich, „Kopiert.“ erscheint, der Bildschirm zoomt beim Kopieren nicht. Das ist die Lücke der Gate-Tests (WebKit und Firefox ungeprüft); ein Fehlschlag ist ein Befund an I0042 (ein Weg im LAN gäbe es dann nur über HTTPS, eine eigene Entscheidung).
- [ ] **UX-Vorprüfung und Entwicklertest bestanden.**

## Betroffene Verzeichnisstruktur

Folgt der ADR (`architekt`): Oberfläche in `web/` (`web/verlauf.js`, `web/nachricht.js`, Stil in `web/lc-seite.css`; Designsystem `components/Nachricht/README.md` nachziehen), E2E unter `e2e/`. Kein Rust-Code.

## Technische Überlegungen

### Ablauf (Stand nach ADR-0052)

1. Welche Aktionen für eine Nachricht gelten, wird je Aktion bestimmt (R00041 Nachtrag): „Kopieren“ gilt für bestätigte, nicht gelöschte Nachrichten mit Text.
2. Der Quelltext kommt aus der Nachricht, die der Verlauf hält; nichts wird neu geladen.
3. Zwischenablage über die moderne Schnittstelle, wo es sie gibt, sonst über den Rückfall; beide Wege werden im Browser belegt.
4. Rückmeldung über die vorhandene Ansage-Region und am Knopf.

### Grobentwurf

Siehe ADR-0052 und die Bubbles B0699–B0703 der WBS: `kopierbar` mit Text im Fenster (B0699), `kopieren` (B0700), Aktion in Leiste, Schalter und Tab-Weg (B0701), Taste `C` (B0702), Rückmeldung (B0703).

## Abhängigkeiten

- Abhängig von: [R00041](R00041-eigene-nachrichten-im-browser-bearbeiten-und-loeschen.md) F0135 (Aktionsleiste B0609, Schalter B0628) und F0141 (Nachrichten fokussieren), weil alle `web/verlauf.js` und `web/nachricht.js` teilen und die Leiste das gemeinsame Gefäß ist; nicht gleichzeitig bauen. R00011 (Verlauf, Ansage-Region), R00027 (Touch-Regeln).
- Blockiert: R00047 (Antworten) nutzt dieselbe Leiste und kommt danach.
- Berührt: [R00007](R00007-channel-wechseln-und-verlauf-lesen.md) hatte die Nachrichtentasten `R` und `C` auf später geschoben; sie sind mit PO Zeile 145 freigegeben, R00007 bleibt unverändert.

## Offene Fragen

Stand nach ADR-0052: Fragen 1, 2, 4 und 5 sind vom `architekt` **bestätigt bzw. entschieden**; Frage 3 liegt beim `ux-reviewer`.

1. **Archivierte Channels** (nur lesbar): bestätigt — „Kopieren“ ist Lesen und schreibt nichts, angeboten sobald I0038 gebaut ist; Antworten dort nicht (R00047).
2. **Direktgespräche** (I0012): bestätigt — wie Channels, nur für die Beteiligten sichtbar, angeboten sobald I0012 gebaut ist.
3. **Ort der Rückmeldung am Knopf** — **entschieden** vom `ux-reviewer` (Vorprüfung #78/#79): die Beschriftung des Knopfes wechselt für 2000 ms auf „✓ Kopiert.“ (Fehler 5000 ms, „⚠ Kopieren nicht möglich“), Breite reserviert, zugänglicher Name unverändert; auf Touch im Menüeintrag, 1200 ms. Die Doppelbelegung von `C` mit dem Codeblock (K3) löste PO 153: `C` = Nachricht, der Codeblock behält nur den Knopf „Code kopieren“.
4. **Nachricht mit Emoji-Kurznamen** (`:smile:`): bestätigt — kopiert wird, was in `text` steht, also der Kurzname.
5. **Rückfall unter http** — entschieden: `execCommand('copy')` ist der Hauptweg (in Chromium über die LAN-IP geprobt: `isSecureContext` falsch, `navigator.clipboard` undefiniert, Textfeld und `execCommand` liefern den Text zeichengleich), `navigator.clipboard.writeText` nur der zweite Weg im sicheren Kontext; ein dritter Weg ist nicht nötig. Offen bleibt WebKit/Firefox: Entwicklertest am Telefon. Fällt `execCommand` in einem Browser weg, gibt es im LAN über `http` keinen Weg mehr — dann wäre HTTPS im LAN eine eigene Entscheidung (heute nicht verlangt).

## Warum löst diese Anforderung das Problem?

Das Problem ist der Weg vom Gelesenen zum Weiterverwendeten: der Text eines Agenten oder Kollegen soll woanders hin, und das Markieren ist unzuverlässig und auf dem Telefon kaum machbar. Die Aktion beantwortet genau das, ohne etwas zu speichern oder zu senden: Griff an der Nachricht → Quelltext in die Zwischenablage → Rückmeldung → Einfügen im Zielfenster. Dass sie auch im LAN über `http` und auf Touch geht, belegen die Kriterien „Über http im LAN“ und „Touch“.

## Notizen

- Optionen: Richtung vom product-owner entschieden (Zeile 144). Verworfen: Kopieren mit Absender und Uhrzeit als zweite Aktion (später additiv möglich); Kopieren der gerenderten Sicht (das Ziel ist oft eine andere Chat-Eingabe oder ein Prompt, der Quelltext braucht).
- Referenzen aus Kenntnis der Produkte, nicht abgerufen (Lücke): ChatGPT- und Claude-Oberflächen kopieren den Markdown-Quelltext, Teams und Slack den Text ohne Kopfzeile.
- Karte #78: die Karte nennt den Rückfall unter `http` ausdrücklich als Technikfrage; die Kriterien prüfen die Wirkung, nicht den Mechanismus.
- Vorprüfung `ux-reviewer` (2026-10-30): vier Muss (Rückmeldung am Knopf, K1 Menü sichtbar, K2 Rückmeldung auf Touch, K4 Taste `C`), K3 durch PO 153 aufgelöst; die Sollte-Punkte (Leiste, Kontrast, Reihenfolge, Fehler mit Ausweg) stehen oben als Anforderung, `title` „Kopieren (C)“ ist optional. PO 154: „Kennung kopieren“ entfällt im Designsystem, PO 152: Touch-Dichte bewusst.
- **Nachtrag 2026-10-31 (planer, Karte #85, product-owner Zeile 157)**: die Grenze „höchstens 4 px“ gilt nur für die Hover-Leiste (Maus, Tastatur); auf Touch sind die 48 px Reserve für „⋯“ eine bewusste Kostenentscheidung, kein Umbau. Das Kriterium „Leiste“ ist entsprechend eingegrenzt; ein E2E-Test, der auf Touch ≤ 4 px prüft, ist anzupassen. Das Designsystem (`components/Nachricht/README.md`) trägt dieselbe Eingrenzung.
