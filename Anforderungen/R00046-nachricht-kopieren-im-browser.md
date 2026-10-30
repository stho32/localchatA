---
id: R00046
status: Neu
datum: 2026-10-30
---

# R00046: Nachricht kopieren im Browser

## Beschreibung

Ein Teilnehmer kopiert im Browser den Text einer Nachricht mit einem Griff in die Zwischenablage — an jeder bestätigten, nicht gelöschten Nachricht, auch an fremden und an Nachrichten von Agenten. Kopiert wird der Text so, wie er geschrieben wurde (Markdown-Quelltext), ohne Absender, Zeit, Marken und Anhänge. Die Aktion steht in der Aktionsleiste (Maus), im Schalter „⋯“ (Touch) und im Tab-Weg; die Taste `C` auf der fokussierten Nachricht ist nur Abkürzung.

Zahlt ein auf: [Vision](R00000-vision.md) — „Nachrichten kopieren und mit Zitat beantworten“ (neuer Zielbild-Punkt, 2026-10-30), dazu „Markdown in Nachrichten“ (der Quelltext ist das Geschriebene) und die Leitplanke „Zielgeräte“ (Maus, Tastatur, Touch; jede Aktion auch ohne Tastenkürzel).

Herkunft: Karte #78 — Auftraggeber 2026-10-30: „Ok, sag ihm bitte auch, dass ich gerne eine einfache Kopieren-Button-Funktion an den Nachrichten hätte. Es gibt in üblichen Chat-Anwendungen die Mit KI chatten übliche Funktionen: Kopieren, Darauf antworten / mit Zitat antworten. Es wäre schön wenn sowas ermöglicht werden könnte. In Microsoft Teams geht sowas.“; entschieden vom product-owner in `AI-Team/po-entscheidungen.md` Zeilen 142, 144, 145.

Umfang (WBS `Dokumentation/Planung/localchata.md`): **I0042** Nachricht kopieren (D0003 Channels). Stand nach ADR-0052: I0042 ist Blatt der Steuerungsebene (kein Feature) mit den Bubbles B0699–B0703. Das Gegenstück „Antworten“ steht in [R00047](R00047-mit-zitat-antworten-api-und-browser.md).

### Domänenbezug

Kein Glossar im Projekt (`Dokumentation/Domaenenwissen/` fehlt). Begriffe wie in R00041: Nachricht, bestätigt (nicht „wird gesendet …“, nicht fehlgeschlagen), gelöscht, Aktion, Aktionsleiste, Schalter „⋯“ (Name „Aktionen“), Ansage-Region, eigene Nachricht, Anhang. Neu und hier festgelegt: **Quelltext** (der Text der Nachricht, wie ihn der Absender geschrieben hat, vor der Darstellung als Markdown).

## Geschäftlicher Nutzen

Wirkungskette: Ein Agent antwortet im Channel mit einem Befehl oder einem Codeblock → der Mensch will ihn in sein Terminal oder in den Prompt eines anderen Agenten übernehmen → ohne Aktion markiert er den Text von Hand, trifft dabei den Rand des Blocks schlecht und kopiert Formatierung statt Quelltext (auf dem Telefon fast nicht möglich) → mit „Kopieren“ landet genau der geschriebene Text in der Zwischenablage, auf jedem Zielgerät und aus fremden wie eigenen Nachrichten.

## Funktionale Anforderungen

Mechanik laut ADR-0052: der Text kommt aus dem Fenster (nicht aus dem DOM), kopiert wird synchron in der Geste über ein verstecktes Textfeld und `document.execCommand('copy')`, die Clipboard-API ist nur der zweite Weg. Es gibt keinen Server-Teil. [B0699–B0703]

### I0042 Nachricht kopieren

- **Wo die Aktion steht**: an jeder bestätigten, nicht gelöschten Nachricht mit Text — eigene, fremde, von Agenten. Nicht angeboten wird sie an einer gelöschten Nachricht, an einer Nachricht nur aus Anhängen (Text leer), an einer Nachricht „wird gesendet …“ und an einer fehlgeschlagenen.
- **Erreichen**: Maus — „Kopieren“ in der Aktionsleiste bei Hover und Fokus (Ort wie R00041); Tastatur — `Tab` von der fokussierten Nachricht zu ihren Aktionen, `Enter` oder Leertaste löst aus; Abkürzung `C` ohne Strg, Alt, Meta und Umschalt auf der fokussierten Nachricht, **nie** im Eingabefeld, nie im Bearbeiten-Feld, nie bei markiertem Text (dann gilt das gewohnte Kopieren des Browsers), `Strg+C` bleibt unberührt; Touch — Schalter „⋯“ (Name „Aktionen“, ≥ 44 × 44 px) mit Menüeintrag „Kopieren“ (≥ 44 × 44 px). Die Taste ist nie der einzige Weg.
- **Was kopiert wird**: genau der Quelltext der Nachricht — ein- und mehrzeilig, mit Markdown-Zeichen, Codeblöcken, Emoji-Kurznamen so, wie gespeichert —, ohne Absender, Zeit, Marken („bearbeitet“) und Anhänge. Eine Nachricht mit Text und Anhang kopiert den Text. Nichts wird ergänzt oder abgeschnitten, auch nicht am Ende (kein angehängter Zeilenumbruch).
- **Auch im LAN**: die Seite läuft dort über `http` ohne sicheren Kontext, in dem `navigator.clipboard` fehlt; „Kopieren“ gelingt trotzdem, weil `execCommand` der Hauptweg ist (ADR-0052: im LAN der getestete Normalfall, nicht ein Rückfall). Die Kriterien prüfen die Wirkung unter `localhost` **und** unter `http://<LAN-IP>`; auf einem echten Telefon (iOS Safari, WebKit) und in Firefox ist der Weg im Gate **nicht** belegt (Playwright nur Chromium) — dafür gilt der Entwicklertest (siehe Akzeptanzkriterien).
- **Rückmeldung**: gelingt es, steht am Knopf kurz „Kopiert.“ und die vorhandene Ansage-Region sagt genau einmal „Kopiert.“; gelingt es nicht, steht am Knopf und in der Region „Kopieren nicht möglich“. Das Kopieren ändert weder Fokus noch Lesestelle.
- **Bestehendes bleibt**: „Bearbeiten“ und „Löschen“ stehen weiter nur an eigenen Nachrichten, `E` und `Entf` wirken nur auf der fokussierten eigenen (R00041); die Leiste und der Schalter erscheinen jetzt an jeder bestätigten, nicht gelöschten Nachricht, weil dort „Kopieren“ gilt.

## Nicht-funktionale Anforderungen

- **Bedienbarkeit** (`ux-reviewer` legt fest): Ort und Wortlaut des Knopfes, Rückmeldung am Knopf (Dauer, Kontrast), Kollision der Taste `C` mit Strg+C, mit Schreibprogrammen und Bildschirmlesern, Touch-Ziele ≥ 44 × 44 px (Leitplanke Zielgeräte, 8 px Abstand im Menü wie R00041), Trefferfläche an gruppierten einzeiligen Nachrichten ohne Verschieben des Textes.
- **Zeit**: die Rückmeldung erscheint binnen 200 ms nach dem Auslösen (Hypothese des planer: das Kopieren ist lokal; der `ux-reviewer` bestätigt oder ändert; erwartet wenige ms).
- **Sicherheit**: es wird nichts an den Server gesendet und nichts gespeichert; die Zwischenablage wird nur beschrieben, nie gelesen; Text einer fremden Nachricht darf nur in die Zwischenablage, nie als HTML (kein Einschleusen); Gate `security-reviewer`.
- **Last**: kein Einfluss auf Lastprofil, Plattenquote oder Betriebsprotokoll (keine Anfrage, keine Zeile).

## Akzeptanzkriterien

Gemeinsam: Start mit `localchatA <ordner> --port 0 --host 0.0.0.0 --kein-browser` auf einem Wegwerf-Ordner mit dem Channel `allgemein`; `shoff` und `anna` in je einem Browser-Fenster (Desktop 1280 × 800, Telefon 375 × 667 mit Touch), Agent `claude` über die API; der Test liest die Zwischenablage über Playwright (`navigator.clipboard.readText()` unter localhost mit Berechtigung; unter `http://<LAN-IP>` über ein eingefügtes Textfeld und `Strg+V`). Die Szenarien stehen in [R00046](user-stories/R00046.md).

- [ ] **Aktion an eigener und fremder Nachricht** (E2E): `shoff` sendet `Hallo`, `claude` sendet `Ergebnis` → bei `shoff` steht an beiden Nachrichten „Kopieren“ (Hover); an der eigenen zusätzlich „Bearbeiten“ und „Löschen“, an der von `claude` nicht.
- [ ] **Quelltext genau** (E2E): `claude` sendet einen mehrzeiligen Text aus vier Zeilen — `Ein **fett** und ein Wort`, eine Leerzeile, ein Codeblock (Zaun mit `sh`, Zeile `ls -la`, Zaun) und `Ende` — → nach „Kopieren“ ist die Zwischenablage zeichengleich dem gesendeten Text (gleiche Länge, `===`), ohne `claude`, ohne Uhrzeit, ohne „bearbeitet“, ohne angehängten Zeilenumbruch.
- [ ] **Über http im LAN** (E2E): dasselbe Szenario über `http://<LAN-IP>:<port>` (nicht localhost; dort ist `window.isSecureContext` falsch) → Zwischenablage zeichengleich, „Kopiert.“ erscheint.
- [ ] **Tastatur** (E2E): fokussierte fremde Nachricht, `Tab` → Fokus auf „Kopieren“, `Enter` → kopiert; Leertaste ebenso; zurück auf die Nachricht, `C` → kopiert; `Strg+C`, `Alt+C`, `Meta+C`, `Umschalt+C` auf der Nachricht kopieren nichts über die Aktion.
- [ ] **`C` nie im Eingabefeld und nie bei Markierung** (E2E): im Eingabefeld getippt, ergibt `C` den Buchstaben `c` und lässt die Zwischenablage unverändert; im Bearbeiten-Feld ebenso; markiert der Test mit der Maus ein Wort der fokussierten Nachricht und drückt `C`, bleibt die Zwischenablage unverändert; `Strg+C` danach kopiert wie im Browser üblich genau die Markierung.
- [ ] **Touch** (E2E, Telefon): Tippen auf „Aktionen“ („⋯“, `aria-expanded` danach `true`) an der fremden Nachricht → Menü mit „Kopieren“; Tippen darauf → kopiert; Schalter und Eintrag je ≥ 44 × 44 px, `document.elementFromPoint` an Mitte und vier Ecken (1 px nach innen) liefert sie selbst oder ein Kind, auch bei drei gruppierten einzeiligen Nachrichten; höchstens ein Menü offen.
- [ ] **Rückmeldung** (E2E): nach dem Kopieren steht am Knopf „Kopiert.“ und die Ansage-Region wechselt genau einmal auf „Kopiert.“; wiederholtes Kopieren derselben Nachricht innerhalb 2 s ändert die Region nicht erneut; ein erzwungener Fehlschlag (Test verbietet beide Wege) zeigt „Kopieren nicht möglich“ am Knopf und in der Region, die Zwischenablage ist unverändert.
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
3. **Ort der Rückmeldung am Knopf** (Beschriftung wechselt kurz auf „Kopiert.“ oder Häkchen; Dauer, Ort, Form): `ux-reviewer`, **offen**.
4. **Nachricht mit Emoji-Kurznamen** (`:smile:`): bestätigt — kopiert wird, was in `text` steht, also der Kurzname.
5. **Rückfall unter http** — entschieden: `execCommand('copy')` ist der Hauptweg (in Chromium über die LAN-IP geprobt: `isSecureContext` falsch, `navigator.clipboard` undefiniert, Textfeld und `execCommand` liefern den Text zeichengleich), `navigator.clipboard.writeText` nur der zweite Weg im sicheren Kontext; ein dritter Weg ist nicht nötig. Offen bleibt WebKit/Firefox: Entwicklertest am Telefon. Fällt `execCommand` in einem Browser weg, gibt es im LAN über `http` keinen Weg mehr — dann wäre HTTPS im LAN eine eigene Entscheidung (heute nicht verlangt).

## Warum löst diese Anforderung das Problem?

Das Problem ist der Weg vom Gelesenen zum Weiterverwendeten: der Text eines Agenten oder Kollegen soll woanders hin, und das Markieren ist unzuverlässig und auf dem Telefon kaum machbar. Die Aktion beantwortet genau das, ohne etwas zu speichern oder zu senden: Griff an der Nachricht → Quelltext in die Zwischenablage → Rückmeldung → Einfügen im Zielfenster. Dass sie auch im LAN über `http` und auf Touch geht, belegen die Kriterien „Über http im LAN“ und „Touch“.

## Notizen

- Optionen: Richtung vom product-owner entschieden (Zeile 144). Verworfen: Kopieren mit Absender und Uhrzeit als zweite Aktion (später additiv möglich); Kopieren der gerenderten Sicht (das Ziel ist oft eine andere Chat-Eingabe oder ein Prompt, der Quelltext braucht).
- Referenzen aus Kenntnis der Produkte, nicht abgerufen (Lücke): ChatGPT- und Claude-Oberflächen kopieren den Markdown-Quelltext, Teams und Slack den Text ohne Kopfzeile.
- Karte #78: die Karte nennt den Rückfall unter `http` ausdrücklich als Technikfrage; die Kriterien prüfen die Wirkung, nicht den Mechanismus.
