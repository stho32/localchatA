---
id: R00047
status: Neu
datum: 2026-10-30
---

# R00047: Mit Zitat antworten (API und Browser)

## Beschreibung

Ein Teilnehmer antwortet auf eine bestimmte frühere Nachricht seines Gesprächs. Die Antwort ist eine **gewöhnliche neue Nachricht** am Ende des Verlaufs in Zeitfolge; sie trägt einen Bezug auf genau eine frühere Nachricht desselben Gesprächs und zeigt über ihrem Text ein Zitat des Originals. Ein Klick auf das Zitat springt zum Original. Das Zitat ist ein lebender Verweis, keine Kopie: wird das Original bearbeitet, gelöscht oder fällt es durch die Aufbewahrungsfrist weg, ist es dem Zitat anzusehen. Der Agent antwortet über die API, der Mensch im Browser; beide gleich. Eine Antwort auf die Nachricht eines Teilnehmers kann dessen wartenden Agenten wecken.

**Keine Threads**: kein Unterverlauf, keine Verschachtelung, keine zweite Ansicht; eine Antwort auf eine Antwort zeigt nur die direkt beantwortete Nachricht, keine Kette.

Zahlt ein auf: [Vision](R00000-vision.md) — „Nachrichten kopieren und mit Zitat beantworten“ (neuer Zielbild-Punkt, 2026-10-30), dazu „Mensch und KI sind gleichberechtigt“, „Agenten können warten und reagieren“, „Live überall“, „Eigene Nachrichten bearbeiten und löschen“ (frühere Fassungen sieht niemand) und die Leitplanken „Datenhaltung in täglich rotierenden Logdateien“, „Plattenquote“ und „Zielgeräte“.

Herkunft: Karte #79 — Auftraggeber 2026-10-30 (Wortlaut in [R00046](R00046-nachricht-kopieren-im-browser.md)); entschieden vom product-owner in `AI-Team/po-entscheidungen.md` Zeilen 142, 143, 145–151.

Umfang (WBS `Dokumentation/Planung/localchata.md`): **I0043** Auf eine Nachricht mit Bezug antworten (API; D0005 Agent an der API) mit den Ausbaustufen **F0157** Bezug senden und lesen und **F0158** Antwort weckt den Beantworteten; **I0044** Im Browser mit Zitat antworten (D0003 Channels) mit **F0159** Antworten und Zitat zeigen und **F0160** Zitat lebt mit dem Original. Bubbles gibt es noch nicht (Rolling Wave); sie kommen nach der ADR des `architekt`. Das Gegenstück „Kopieren“ steht in R00046.

### Domänenbezug

Kein Glossar im Projekt (`Dokumentation/Domaenenwissen/` fehlt). Begriffe wie in R00039, R00041, R00045: Nachricht (angegeben über ihre Zeit), Kennung, Gespräch, Verlauf, Fenster, Nachholen, Fassung, Marke, Aktion, Ansage-Region, Wartekommando (`warten`), Erwähnung, Tageslog, Plattenquote. Neu und hier festgelegt: **Antwort** (Nachricht mit Bezug), **Bezug** (Verweis auf das Original: Zeit und Kennung), **Zitat** (die Darstellung des Bezugs über dem Text) und **Zustand des Originals** (`da`, `bearbeitet`, `gelöscht`, `entfallen`).

## Geschäftlicher Nutzen

Wirkungskette: In einem Channel mit 70 Teilnehmern schreiben drei Agenten und zehn Menschen durcheinander → auf die Frage von `claude` von vor zwanzig Nachrichten kommt eine Antwort, die niemand mehr zuordnen kann → mit Zitat steht über der Antwort, worauf sie sich bezieht, ein Klick führt zur Frage, und der fragende Agent wird geweckt, ohne dass der Antwortende `@claude` schreiben muss → das Gespräch bleibt ein einziger Strom und trotzdem lesbar. Ohne Threads, weil ein zweiter Verlauf je Nachricht Last, Platz und Aufmerksamkeit kostet.

## Funktionale Anforderungen

Route, Feldnamen und Ereignisart legt die ADR fest; die Kriterien nennen die Wirkung. „Bezug“ bezeichnet die Angabe beim Senden, „Zitat“ die Darstellung im Browser.

### I0043 Auf eine Nachricht mit Bezug antworten (API)

**Ausbaustufe 1 — F0157 Bezug senden und lesen**

- **Senden mit Bezug**: dieselbe Sende-Route wie ohne Bezug, mit einem optionalen Bezug auf eine Nachricht des Gesprächs, angegeben über ihre Zeit (wie beim Bearbeiten und Löschen). Mensch (Browser) und Agent (API) verhalten sich gleich; eine Antwort auf die Nachricht eines Agenten braucht keinen Sonderweg. Ohne Bezug verhält sich das Senden wie bisher (R00011), mit allen Grenzen (Länge, Schreibkonto, Plattenquote).
- **Der Bezug in der Antwort**: die Antwort des Sendens, der Verlauf, der Strom und `warten` (ab I0016) liefern den Bezug als Kennung des Originals, seine Zeit, einen Textanfang und seinen **Zustand**: `da` (unverändert vorhanden), `bearbeitet` (mit dem neuen Textanfang), `gelöscht` (ohne Text), `entfallen` (das Original ist durch die Aufbewahrungsfrist weg, ohne Text). Der Textanfang ist höchstens so lang, wie die ADR festlegt (Empfehlung planer: 120 Zeichen; Hypothese, reicht für zwei Zeilen bei 375 px; mit „…“ gekürzt); bei einem Original nur aus Anhängen steht kein Textanfang, sondern der Dateiname des ersten Anhangs.
- **Abweisung**: ein **unbekannter** Bezug (keine Nachricht mit dieser Zeit im Gespräch), ein Bezug auf eine Nachricht eines **anderen Gesprächs** und ein Bezug auf eine **gelöschte** Nachricht werden mit einem deutschen Text abgewiesen (Status und Feld wie die übrigen Abweisungen, laut ADR; Empfehlung planer: 422 mit `regel`, wie R00039 und R00040); nichts wird gespeichert, kein Anhang abgelegt, das Schreibkonto bleibt unberührt. Ein Bezug auf eine **bearbeitete** Nachricht ist gültig.
- **Im Log nur der Bezug**: der Tageslog trägt am Satz der Antwort ausschließlich Zeit und Kennung des Originals, nie den zitierten Text und nicht den Zustand; der Text des Originals steht nur an seiner eigenen Stelle. Folge: die Plattenquote wächst durch eine Antwort nur um den Bezug (Zeit und Kennung des Originals), nie um den zitierten Text.
- **Keine Verschachtelung**: antwortet jemand auf eine Antwort, trägt die neue Antwort einen Bezug auf diese Antwort; der Zustand und der Textanfang gelten dieser Antwort — nie wird eine Kette aufgelöst oder mitgeliefert.
- **Zeitfolge, kein Thread**: die Antwort steht in Zeitfolge am Ende des Verlaufs, im Index wie jede Nachricht; es gibt keinen zweiten Verlauf, keine Abfrage „alle Antworten auf …“ und keinen Zähler am Original.
- **Zustand folgt dem Original**: wird das Original bearbeitet oder gelöscht, liefert jedes spätere Lesen den neuen Zustand; das Log der Antwort ändert sich dabei nicht.

**Ausbaustufe 2 — F0158 Antwort weckt den Beantworteten**

- Eine Antwort auf die Nachricht eines Teilnehmers gilt für dessen **Wartekommando** (`warten`, I0016) wie eine **Erwähnung** dieses Teilnehmers: sein `warten` kehrt zurück, ohne dass der Antwortende `@name` schreibt. Antwortet jemand auf die eigene Nachricht, weckt das niemanden (eigene Nachrichten wecken nie). Ist das am vorhandenen Weckmechanismus teuer, bleibt der Bezug trotzdem lesbar, nur ohne Wecken, und der `architekt` meldet es als Kommentar ohne neue Richtungsfrage (PO Zeile 150).

### I0044 Im Browser mit Zitat antworten

**Ausbaustufe 1 — F0159 Antworten und Zitat zeigen**

- **Antworten beginnen**: die Aktion „Antworten“ steht an jeder bestätigten, nicht gelöschten Nachricht (eigene, fremde, von Agenten) in der Aktionsleiste (Maus), im Schalter „⋯“ (Touch) und im Tab-Weg; die Taste `R` auf der fokussierten Nachricht ist nur Abkürzung — ohne Strg, Alt, Meta und Umschalt, nie im Eingabefeld, nie im Bearbeiten-Feld, nie bei markiertem Text; sie ist nie der einzige Weg. Nicht angeboten wird sie an einer gelöschten, einer noch nicht bestätigten und einer fehlgeschlagenen Nachricht, und in archivierten Channels (nur lesbar).
- **Die Zeile über dem Eingabefeld**: „Antwort an <Kennung>: <Textanfang>“ mit „Abbrechen“ (auch `Esc`, solange der Fokus im Feld oder in der Zeile steht); der Fokus geht ins Eingabefeld. Die Zeile bleibt, bis die Antwort gesendet, abgebrochen oder der Channel gewechselt wird; ein Channelwechsel **verwirft** den Bezug (anders als den Entwurf — ein Bezug ohne den Verlauf sagt nichts). Es gibt höchstens einen Bezug je Eingabe; „Antworten“ auf eine andere Nachricht ersetzt ihn.
- **Senden**: die Antwort braucht Text oder Anhang wie jede Nachricht; sie geht über die Route aus I0043 mit der Zeit des Originals; Fehlschlag, Wiederholen und Verwerfen verhalten sich wie beim Senden ohne Bezug (R00012), der Bezug bleibt dabei erhalten. Weist die Instanz den Bezug ab (Original inzwischen gelöscht), erscheint der Text der Instanz am Eingabefeld, die Zeile wird entfernt, das Getippte bleibt.
- **Das Zitat in der gesendeten Antwort**: über dem Text der Antwort steht das Zitat — Kennung des Originals, Uhrzeit (HH:MM), Textanfang auf höchstens zwei Zeilen gekürzt, bei einem Original nur aus Anhängen „Anhang: <Dateiname>“. Das Zitat zeigt den Stand, den die Instanz beim Laden oder Senden liefert: Textanfang, bei gelöschtem Original „Nachricht gelöscht“ ohne Text, bei entfallenem Original „ältere Nachricht, nicht mehr vorhanden“. Eine Antwort auf eine Antwort zeigt nur diese Nachricht, keine Kette.
- **Sprung zum Original**: Klick, Antippen oder `Enter` auf dem Zitat springt zum Original, hebt es kurz hervor (nicht nur durch Farbe) und lädt nach, wenn es außerhalb des geladenen Fensters liegt — wie der Sprung aus der Suche; ist es entfallen, steht ein Hinweis, und der Verlauf bleibt, wo er ist. Auch aus einem Verlauf mit 100.000 Nachrichten hängt der Tab dabei nicht. Das Zitat ist ein bedienbares Element mit zugänglichem Namen (Kennung, Uhrzeit, Textanfang).
- **Ansage**: das Beginnen einer Antwort wird genau einmal angesagt („Antwort an <Kennung>“), das Abbrechen genau einmal („Antwort abgebrochen“); Wortlaute legt der `ux-reviewer` fest.

**Ausbaustufe 2 — F0160 Zitat lebt mit dem Original**

- Wird das Original **bearbeitet**, zeigt jedes Zitat darauf in allen offenen Tabs, die es zeigen, binnen 1 s den neuen Textanfang. Wird es **gelöscht**, steht dort binnen 1 s „Nachricht gelöscht“ ohne Text (der gelöschte Text bleibt aus der Sicht). Entfällt sein Tag durch die Aufbewahrungsfrist, steht dort beim nächsten Laden „ältere Nachricht, nicht mehr vorhanden“. Eine offene Zeile „Antwort an <Kennung>: <Textanfang>“ über dem Eingabefeld folgt ebenso: wird das Original gelöscht, während man antwortet, ändert sich die Zeile zu „Nachricht gelöscht“ und „Senden“ der Antwort wird beim Abweisen wie oben behandelt — nichts Getipptes geht still verloren.

## Nicht-funktionale Anforderungen

- **Zeit** (Zweck: Vision „Live überall“): Zustellung einer Antwort und Nachführen eines Zitats binnen 1 s; Regel und Zustellung sind getrennt gemessen (von der Antwort der Instanz bis zum Zustand im Tab); erwartet wenige ms, die Grenze liegt weit über dem erwarteten Wert.
- **Sprung im großen Verlauf**: in einem Channel mit 100.000 Nachrichten steht das Original binnen 1 s nach dem Klick sichtbar und hervorgehoben im Fenster (Grenze der Leitplanke „Suche“; erwartet deutlich darunter, ungemessen), und der Hauptthread blockiert nie länger als 200 ms am Stück (Hypothese des planer, vom `ux-reviewer` zu bestätigen).
- **Last**: Lastprofil „Team-LAN“ unverändert: 30 Nachrichten/s, darunter Antworten; ein Bezug kostet beim Senden eine Nachschlagung der Zeit im Verlaufsindex des Gesprächs (Zeit O(log n) oder besser — der `architekt` belegt es bei 100.000 Nachrichten); Zustellung unter 1 s bleibt.
- **Datenhaltung**: im Tageslog nur der Bezug, nie der zitierte Text; keine Änderung der Aufbewahrung (ein Bezug hält das Original nicht am Leben).
- **Sicherheit**: der Bezug kommt vom Client und wird nie geglaubt: Zeit und Gespräch werden gegen den Index geprüft; ein Bezug aus einem Gespräch, das der Absender nicht sehen darf, antwortet wie ein unbekannter (kein Orakel über fremde Nachrichten, auch nicht über den Zustand); der Textanfang eines Zitats wird wie jeder Nachrichtentext dargestellt (kein HTML, kein Markdown-Zugriff auf fremde Ressourcen); Gate `security-reviewer`.
- **Bedienbarkeit** (`ux-reviewer` legt fest): Ort und Wortlaut von „Antworten“, Zitatzeile und Zitat (Kontrast, Platz, Umbruch bei 320 px und 375 px, kein Springen des Verlaufs beim Erscheinen der Zeile), Taste `R` gegen Kollision mit Schreibprogrammen und Bildschirmlesern, Touch ≥ 44 × 44 px, Ansagen, Hervorhebung des Originals.

## Akzeptanzkriterien

Gemeinsam: Start mit `localchatA <ordner> --port 0 --host 127.0.0.1 --kein-browser` auf einem Wegwerf-Ordner mit den Channels `allgemein` und `technik`; Menschen `anna` und `ben` in je einem Browser-Fenster (Desktop 1280 × 800, Telefon 375 × 667 mit Touch), Agenten `claude` und `codex` über die API (Kennungen `claude@127.0.0.1`, `codex@127.0.0.1`). Die Szenarien stehen in [R00047](user-stories/R00047.md).

### I0043 Auf eine Nachricht mit Bezug antworten (API)

- [ ] **Antwort mit Bezug** (Integration, F0157): `claude` sendet `Frage?` (Zeit `t1`), `codex` sendet `Antwort` mit Bezug auf `t1` → 201, die Antwort nennt den Bezug mit Kennung `claude@127.0.0.1`, Zeit `t1`, Textanfang `Frage?` und Zustand `da`; der Verlauf liefert dieselben Angaben.
- [ ] **Lesen, Strom und Warten** (Integration, F0157): dieselbe Antwort erscheint im Verlauf, im Ereignisstrom (`GET /api/ereignisse`) und — sobald I0016 grün — in `warten` mit demselben Bezug und Zustand; ohne Bezug fehlt das Feld (kein `null`-Zwang, laut ADR).
- [ ] **Vier Fälle** (Integration, F0157): (a) gültiger Bezug → 201; (b) Zeit, die es im Gespräch nicht gibt → Abweisung; (c) Zeit einer Nachricht aus `# technik`, gesendet in `# allgemein` → Abweisung; (d) Bezug auf eine gelöschte Nachricht → Abweisung; in (b) bis (d) deutscher Text, `sha256sum` über alle Dateien unter `channels/` und `belegt` in `/api/gesundheit` unverändert, kein Anhang abgelegt.
- [ ] **Zustände des Originals** (Integration, F0157): nach dem Bearbeiten von `Frage?` zu `Frage!` liefert Lesen der Antwort Zustand `bearbeitet` und Textanfang `Frage!`; nach dem Löschen `gelöscht` ohne Text; nach Wegfall des Tags durch die Frist (Testdaten-Erzeuger wie R00008) `entfallen` ohne Text; der Satz der Antwort im Tageslog ist dabei unverändert (`sha256sum` der Datei des Tags der Antwort).
- [ ] **Log nur Bezug** (Integration, F0157): Original mit der Marke `ZITATMARKE-4711` und 500 Zeichen Text → in der Datei des Tags der Antwort kommt die Marke genau einmal vor (am Original), nicht am Satz der Antwort; die Bytes des Satzes übersteigen die einer gleichen Antwort ohne Bezug höchstens um Zeit und Kennung des Originals plus Feldkennung (Rechenbeispiel: Kennung `claude@127.0.0.1` 16 Zeichen, Zeit 29 Zeichen → höchstens 45 Zeichen plus Feldkennung, nie um 500).
- [ ] **Keine Verschachtelung, Zeitfolge** (Integration, F0157): `anna` antwortet auf die Antwort von `codex` → der Bezug nennt `codex` und deren Textanfang, nie `claude`; die Antworten stehen im Verlauf in Zeitfolge am Ende; es gibt keine Route und kein Feld „Antworten auf …“.
- [ ] **Kein Rückschritt ohne Bezug** (Integration, F0157): Senden ohne Bezug liefert die gleiche Antwort wie vor der Änderung (R00011), alle Grenzen gelten unverändert (Länge, Schreibkonto, Plattenquote 507).
- [ ] **Kein Orakel** (Integration, F0157): Bezug auf eine Zeit in einem Direktgespräch, das der Absender nicht sehen darf, antwortet wie ein unbekannter Bezug (gilt, sobald I0012 grün).
- [ ] **Wecken** (Integration, F0158): `codex` sendet `N` und wartet über `warten`; `anna` antwortet auf `N` ohne `@codex` → `warten` kehrt binnen 1 s mit der Antwort zurück; eine Antwort auf eine Nachricht von `claude` weckt `codex` nicht; antwortet `codex` auf die eigene Nachricht, kehrt sein `warten` nicht zurück. Ist das am Weckmechanismus teuer, steht der Kommentar des `architekt` und der Bezug bleibt lesbar (dann entfällt nur dieses Kriterium, nicht die Anforderung).

### I0044 Im Browser mit Zitat antworten

- [ ] **Antworten per Maus** (E2E, F0159): `anna` sendet `Frage?`, bei `ben` fährt die Maus über die Nachricht, Klick auf „Antworten“ → über dem Eingabefeld steht „Antwort an anna: Frage?“ mit „Abbrechen“, der Fokus steht im Eingabefeld; `ben` tippt `Ja` und sendet → die Antwort steht am Ende des Verlaufs, über `Ja` das Zitat mit `anna`, Uhrzeit und `Frage?`; die Zeile über dem Feld ist weg.
- [ ] **Tastatur** (E2E, F0159): fokussierte Nachricht, `R` → Zeile und Fokus im Feld; `Esc` → Zeile weg, kein `POST`, Fokus im Feld; `Strg R`, `Alt R`, `Meta R`, `Umschalt R` auf der Nachricht tun nichts; `R` im Eingabefeld tippt den Buchstaben; `R` bei markiertem Text tut nichts; `Tab` von der Nachricht erreicht „Antworten“ (und an eigenen die übrigen Aktionen), `Enter` löst aus.
- [ ] **Touch** (E2E, Telefon, F0159): „Aktionen“ („⋯“) → Menü mit „Kopieren“ und „Antworten“ (je ≥ 44 × 44 px, 8 px Abstand); Tippen auf „Antworten“ → Zeile über dem Feld, Bildschirmtastatur geht auf (Fokus im Klick synchron gesetzt); Senden → Zitat.
- [ ] **Zeile und Channelwechsel** (E2E, F0159): Zeile steht, Wechsel nach `# technik` und zurück → Zeile weg, der Entwurf des Feldes bleibt; „Antworten“ auf eine andere Nachricht ersetzt die Zeile (genau eine); die Position der letzten Nachricht im Verlauf ist vor und nach dem Erscheinen der Zeile gleich (Toleranz 0 px, auch bei 375 px).
- [ ] **Zitat im Detail** (E2E, F0159): Original mit 300 Zeichen → Zitat auf höchstens zwei Zeilen gekürzt (berechnete Höhe ≤ 2 × `line-height`), auch bei 320 px ohne Querscrollen; Original nur aus einem Anhang `bericht.pdf` → „Anhang: bericht.pdf“; Antwort auf eine Antwort → Zitat zeigt nur diese Nachricht.
- [ ] **Sprung** (E2E, F0159): Klick, Antippen und `Enter` auf dem Zitat → das Original ist sichtbar und hervorgehoben (nicht nur durch Farbe, nach etwa 2 s wieder normal); liegt es außerhalb des geladenen Fensters, steht es nach dem Nachladen im Bild; in einem Channel mit 100.000 Nachrichten (Testdaten-Erzeuger) und dem Original als ältester Nachricht: Original sichtbar binnen 1 s, kein Hauptthread-Block über 200 ms (Long-Task-Messung); ist das Original entfallen, steht ein Hinweis und der Verlauf bleibt stehen.
- [ ] **Nicht angeboten** (E2E, F0159): gelöschte Nachricht, „wird gesendet …“ und fehlgeschlagene Nachricht bieten „Antworten“ nicht an, `R` tut dort nichts; im Archiv (gilt, sobald I0038 grün) gibt es kein „Antworten“, aber „Kopieren“ (R00046).
- [ ] **Abweisung** (E2E, F0159): `ben` beginnt die Antwort auf eine Nachricht von `anna`, `anna` löscht sie, `ben` sendet → die Instanz weist ab, der Text der Instanz steht am Feld, die Zeile ist weg, das Getippte bleibt, die Nachricht ist nicht gespeichert.
- [ ] **Fehlschlag und Wiederholen** (E2E, F0159): Senden schlägt fehl (Test unterbricht die Verbindung) → die Nachricht steht als fehlgeschlagen mit ihrem Zitat da; „Erneut senden“ sendet mit demselben Bezug.
- [ ] **Ansage** (E2E, F0159): Beginnen und Abbrechen werden je genau einmal angesagt; das Senden ändert die Region wie bisher.
- [ ] **Zitat lebt** (E2E, F0160): `anna` bearbeitet `Frage?` zu `Frage!` → bei `ben` und in einem zweiten Tab von `ben` zeigt das Zitat binnen 1 s `Frage!`; `anna` löscht → binnen 1 s steht „Nachricht gelöscht“ ohne Text; Original durch die Frist entfallen (Testdaten-Erzeuger) → nach dem Neuladen „ältere Nachricht, nicht mehr vorhanden“.
- [ ] **Offene Zeile lebt** (E2E, F0160): `ben` hat die Zeile „Antwort an anna: Frage?“ offen, `anna` bearbeitet → die Zeile zeigt `Frage!`; `anna` löscht → die Zeile zeigt „Nachricht gelöscht“, der Text im Feld bleibt.
- [ ] **Kein Thread** (E2E, F0159): es gibt keine Ansicht, kein Feld und keinen Zähler „Antworten“ am Original; die Antwort steht in Zeitfolge am Ende, nach 10 weiteren Nachrichten dazwischen weiterhin dort, wo sie gesendet wurde.
- [ ] **Beide Wege gleich** (Integration, E2E): dieselbe Antwort, im Browser gesendet und per `curl` gelesen, trägt denselben Bezug und Zustand wie eine per `curl` gesendete im Browser.
- [ ] **Reviews**: UX-Vorprüfung, Code- und Security-Review und Entwicklertest bestanden.

## Betroffene Verzeichnisstruktur

Folgt der ADR (`architekt`): Bezug am Nachrichtensatz im Tageslog und im Verlaufsindex, Prüfung und Zustand in `src/gespraeche/`, Route in der HTTP-Schnittstelle (A00004, OpenAPI A00002 und Betreiber-Anleitung I0019 nachziehen), Ereignisart im Live-Kanal (A00011), Weckregel am Wartekommando (I0016), Oberfläche in `web/` (`web/verlauf.js`, `web/nachricht.js`, Eingabe, Stil in `web/lc-seite.css`; Designsystem `components/Nachricht/README.md` nachziehen), Tests spiegeln die Themenordner.

## Technische Überlegungen

### Ablauf (Vorplanung — Bubbles entstehen nach der ADR)

1. Bezug beim Senden prüfen: Zeit im Index des Gesprächs nachschlagen, Zustand lesen; unbekannt, fremd oder gelöscht → Abweisung vor jedem Schreiben.
2. Bezug am Satz im Tageslog festhalten (Zeit und Kennung des Originals), nie den Text.
3. Beim Lesen den Bezug auflösen: aktueller Textanfang und Zustand des Originals aus dem Index (bearbeitet, gelöscht, entfallen).
4. Ereignis `aenderung` (R00039, R00040) führt offene Zitate nach; die Anzeige in den Tabs folgt der Übernahme-Regel von `web/live-verlauf.js`.
5. Aktionsleiste um „Antworten“ erweitern; Zeile über dem Eingabefeld; Sprung über den Mechanismus des Sprungs aus der Suche.
6. Ausbaustufe 2: Weckregel für Antworten am Wartekommando.

### Grobentwurf

Entfällt bis zur ADR (Rolling Wave).

## Abhängigkeiten

- Abhängig von: R00011 (Senden, Verlauf, Strom), R00039 und R00040 (Zeit als Adresse einer Nachricht, `aenderung`), [R00041](R00041-eigene-nachrichten-im-browser-bearbeiten-und-loeschen.md) F0134, F0138 (Änderungen live, für F0160), F0135 und F0141 (Aktionsleiste, Schalter, Fokus), [R00046](R00046-nachricht-kopieren-im-browser.md) (#78 vor #79; die Leiste kennt dann auch „Kopieren“). I0043 für I0044. F0158 zusätzlich I0016 Auf Nachrichten warten.
- Nachgezogen, sobald gebaut: I0016 (`warten` liefert den Bezug), I0011 (Sprung aus der Suche als gemeinsamer Mechanismus), I0012 (Sichtbarkeit im Direktgespräch), I0038 (Archiv nur lesbar), I0019 (Betreiber-Anleitung nennt den Bezug).
- Blockiert: nichts.

## Offene Fragen

Aus den Entscheidungen des product-owner nicht beantwortet, vom planer mit Empfehlung gesetzt (der `architekt` bzw. `ux-reviewer` bestätigt oder ändert):

1. **Länge des Textanfangs** in API und Zitat — Empfehlung 120 Zeichen in der API (Hypothese, reicht für zwei Zeilen bei 375 px), das Zitat kürzt zusätzlich per Darstellung auf zwei Zeilen: `architekt` und `ux-reviewer`.
2. **Route und Feldnamen** des Bezugs (Empfehlung: ein Feld `antwort_auf` mit der Zeit im Sende-Körper; Antwort und Lesen liefern ein Objekt mit Kennung, Zeit, Textanfang und `zustand`): `architekt` (PO Zeile 149).
3. **Status der Abweisung** — Empfehlung 422 mit `regel`, wie R00039/R00040: `architekt`.
4. **Zustand `entfallen` erkennen**, wenn der Tag des Originals fehlt: Original nicht im Index, aber Bezug mit Zeit älter als der älteste vorhandene Tag → `entfallen`; ist die Zeit im vorhandenen Bereich und fehlt, gilt der Bezug schon beim Senden als unbekannt. Annahme des planer, `architekt` bestätigt.
5. **Antworten in archivierten Channels**: nicht angeboten (nur lesbar), Kopieren schon (R00046). Annahme des planer.
6. **Antwort auf die Nachricht eines Teilnehmers, der abwesend ist**: kein Sonderfall, der Bezug steht, geweckt wird nur, wer wartet. Annahme des planer.
7. **Löschen der Antwort selbst**: ein Bezug lebt nur in der Antwort; wird sie gelöscht, verschwindet mit ihrem Text auch das Zitat (R00040). Annahme des planer.
8. **Wecken und Kosten** (F0158): der `architekt` meldet am Kommentar der Karte, falls es teuer ist (PO Zeile 150).
9. **Sprungziel außerhalb des Fensters bei 100.000 Nachrichten**: Mechanismus des Sprungs aus der Suche (I0011, noch nicht gebaut) — wenn bis dahin nicht gebaut, baut I0044 den Nachladeteil selbst: `architekt`.

## Warum löst diese Anforderung das Problem?

Das Problem ist die verlorene Zuordnung in einem einzigen Strom: eine Antwort sagt nicht, worauf sie antwortet, und der Beantwortete merkt es nicht. Der Bezug beantwortet beides, ohne einen zweiten Verlauf zu eröffnen: Antworten wählen → Bezug am Satz (nur Zeit und Kennung) → Zitat über dem Text, lebend mit dem Original → Klick zum Original → Wecken des beantworteten Agenten. Dass es dabei kein Thread wird und keinen zitierten Text im Log hinterlässt, belegen die Kriterien „Kein Thread“, „Log nur Bezug“ und „Keine Verschachtelung“.

## Notizen

- Optionen: Richtung vom product-owner entschieden (Zeilen 143, 146–151). Verworfen: Threads mit Unterverlauf (Zeile 143), mehrere Zitate in einer Nachricht (Zeile 146, später ergänzbar), das Zitat als Kopie des Textes im Log (Zeile 148, Plattenquote und „frühere Fassungen sieht niemand“).
- Referenz Teams-Chat: „Zitierte Antwort“ als normale Nachricht mit Vorschau des Originals, ohne Thread, Klick springt zum Original (https://support.microsoft.com/office/reply-to-a-specific-message-in-chat-940c614c-4f07-41d4-850b-90f11da8329b, abgerufen 2026-10-30 laut PO Zeile 146); Slack und Teams benachrichtigen den Autor des Originals bei einer Antwort (Zeile 150).
- Schnitt (Begründung in der WBS-Notiz „Karten #78 und #79“): zwei Interactions nach Zugang (API, Browser), Ausbaustufen nach „dasselbe Verhalten reifer“.
- Karte #79 Abnahmepunkt (7): ux-Vorprüfung, Code- und Security-Review und Entwicklertest sind Teil der Fertig-Kriterien.
