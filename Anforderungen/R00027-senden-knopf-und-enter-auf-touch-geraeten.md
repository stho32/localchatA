---
id: R00027
status: Neu
datum: 2026-10-29
---

# R00027: Senden-Knopf und Enter auf Touch-Geräten

## Beschreibung

Bis heute sendet nur die Taste `Enter` (R00011 F0043; Designsystem `components/Eingabe/README.md:6`). Auf Telefon und Tablet kennt die Bildschirmtastatur kein `Shift Enter`. Ihre Eingabetaste macht meist eine neue Zeile oder liefert während der Wortvorschläge `keyCode 229`, das die Eingabe als Komposition verwirft (`web/eingabe.js:10`). Senden ist dort nicht verlässlich möglich. Danach gilt:

- **Ein Knopf „Senden“ neben dem Feld** sendet den Text auf jedem Gerät, mit Touch, Maus und Tastatur. Er sendet genau dann, wenn `Enter` im Feld senden würde, und auf demselben Weg: dieselbe Zustellung, dieselben Fehlschläge, dieselbe Warteschlange. Bei leerem Feld, Text nur aus Leerraum oder verletzter Textregel ist er als nicht verfügbar gekennzeichnet und ohne Wirkung.
- **Auf Touch-Geräten macht `Enter` eine neue Zeile.** Gesendet wird dort über den Knopf; `Strg Enter` und `⌘ Enter` senden weiter, etwa mit einer angesteckten Tastatur am Tablet. Mit Maus und Tastatur bleibt `Enter` Senden wie bisher.

Zahlt ein auf: [Vision](R00000-vision.md) — „Bedienkonzept wie kanbanD“ und „Live überall“: ein Mensch schreibt im Browser mit, auch vom Telefon oder Tablet. Kein Widerspruch zur Vision: „keine Mobil-App“ schließt eine eigene App aus, nicht den Browser auf dem Telefon (Hinweis an den Betreiber unter „Offene Fragen“ 6).

Herkunft: Entscheidung des Betreibers vom 2026-10-29 (Vorschlag angenommen), Anlass ist die Geräte-Regel des UX-Reviews („auf Telefon und Tablet jede Aktion einmal nur mit Touch erledigen“).

Slices (WBS `Dokumentation/Planung/localchata.md`, beide unter I0009 Nachricht senden, Ausbaukette „Eingabe“): F0091 Senden-Knopf neben dem Feld (Eingabe 2), F0092 Auf Touch-Geräten macht Enter eine neue Zeile (Eingabe 3).

### Domänenbezug

Kein Glossar im Projekt (`Dokumentation/Domaenenwissen/` fehlt). Begriffe wie in R00011, R00012 und R00020: Feld (Eingabe unter dem Verlauf), Textregel (Länge und Steuerzeichen, F0048), Leerraum (Unicode `White_Space`, R00020 F0071), vorläufig, fehlgeschlagen, Warteschlange (ADR-0037), Bildschirmleser. Neu: **Touch-Gerät** — ein Gerät, dessen primärer Zeiger grob ist (CSS `(pointer: coarse)`; die Erkennung bestätigt der architekt, „Offene Fragen“ 1); **Bildschirmtastatur** — die eingeblendete Tastatur eines Touch-Geräts.

## Geschäftlicher Nutzen

Wer auf dem Telefon mitliest, kann heute nicht verlässlich antworten. Mit Android-Tastaturen kommt `Enter` als `keyCode 229` an und wird verworfen, die Nachricht bleibt im Feld. Unter iOS sendet `Enter`, dafür lässt sich keine zweite Zeile tippen. Damit fällt das Telefon als Gerät für einen schnellen Zwischenruf aus, etwa für die Antwort an einen Agenten oder die Freigabe vom Flur. Nach R00027 sendet ein Tippen auf „Senden“ auf jedem Gerät, und auf Touch-Geräten wird `Enter` zur Zeilentaste, wie die Bildschirmtastatur sie anbietet.

## Funktionale Anforderungen

### Senden-Knopf neben dem Feld (F0091)

- **Ort**: rechts neben dem Feld, innerhalb des Rahmens der Eingabe, bündig zur Unterkante des Felds; wächst das Feld (bis 200 px), bleibt der Knopf unten neben ihm. Er erscheint mit dem Feld, also sobald ein Channel gewählt ist. Links neben ihm kommt mit I0010 der Schalter „Anhang“. Dafür ist Platz vorzusehen, ein leerer Schalter wird nicht gezeichnet (ADR-0038 Regel 1).
- **Name und Beschriftung**: zugänglicher Name genau „Senden“, Rolle `button`. Die sichtbare Beschriftung legt die Ergänzung zum Designsystem fest. Ist Text sichtbar, ist er Teil des Namens (Empfehlung: sichtbarer Text „Senden“, „Offene Fragen“ 5).
- **Zielfläche**: auf Touch-Geräten mindestens 44 × 44 px, sonst mindestens 24 × 24 px (WCAG 2.5.8; 44 px Empfehlung für Telefone).
- **Bereit oder nicht verfügbar**: der Knopf ist bereit, wenn `Enter` im Feld senden würde. Dazu darf der Text nicht nur aus Zeichen der Unicode-Eigenschaft `White_Space` bestehen (dieselbe Leer-Funktion wie `Enter`, R00020 F0071), und die Textregel darf keinen Fehler melden (Länge über 16.384 Byte, Steuerzeichen; R00012 F0048). Sonst ist er **als nicht verfügbar gekennzeichnet** (Empfehlung: `aria-disabled="true"`, fokussierbar, sichtbar gedämpft; „Offene Fragen“ 2). Ein Tippen, Klicken, `Enter` oder `Leertaste` auf ihm hat dann keine Wirkung: keine Anfrage, kein Fehler, der Text bleibt im Feld.
- **Senden**: Tippen, Klick, `Enter` oder `Leertaste` am bereiten Knopf senden den Text **auf demselben Weg wie `Enter` im Feld** (ADR-0038 Regel 3, ADR-0037). Das Feld wird leer und wieder eine Zeile hoch, der Entwurf des Channels ist gelöscht, die Nachricht steht vorläufig „wird gesendet …“ am Ende des Verlaufs und springt ans Ende. Sie wird bestätigt oder als fehlgeschlagen mit Grund gezeigt, mit „Erneut senden“ und „Verwerfen“. Ohne Verbindung wartet sie wie bei `Enter`. Weitere Nachrichten reihen sich in die Warteschlange des Tabs ein, ein Sendeauftrag zugleich. Der Knopf wird während des Sendens nie gesperrt; ob er bereit ist, hängt nur am Text im Feld.
- **Fokus**: nach dem Senden über den Knopf liegt der Fokus im Feld, auch wenn mit der Tastatur über `Tab` auf den Knopf gegangen wurde. Auf Touch-Geräten verlässt der Fokus das Feld beim Tippen auf den Knopf nicht, die Bildschirmtastatur bleibt offen („Offene Fragen“ 3).
- **Tab-Reihenfolge**: Kopfzeile → Seitenleiste → Verlauf → Feld → **Knopf** → Teilnehmerliste (ändert ADR-0038 Regel 8 um den Knopf).
- **Bildschirmleser**: der Knopf meldet sich als „Senden, Schalter“, im nicht bereiten Zustand zusätzlich als nicht verfügbar. Für den Knopf gibt es keine eigene Ansage und keine neue Live-Region. Ein Fehlschlag wird wie bisher sofort über die eine Ansage-Region gemeldet (ADR-0038 Regel 10, Ergänzung Karte #36).

### Auf Touch-Geräten macht Enter eine neue Zeile (F0092)

- Ist der Zeiger des Geräts grob (Touch-Gerät), macht `Enter` ohne Umschalttaste im Feld **eine neue Zeile** und sendet nicht. `Shift Enter` macht weiter eine neue Zeile. `Strg Enter` und `⌘ Enter` senden weiter, unter denselben Bedingungen wie bisher (nicht während einer Komposition, nicht bei Leerraum oder verletzter Textregel).
- Mit feinem Zeiger (Maus, Trackpad) gilt R00011 F0043 unverändert: `Enter` sendet.
- Ausgewertet wird bei jedem Tastendruck, nicht einmal beim Laden. Wechselt die Zeigerart, gilt beim nächsten Tastendruck die neue Regel, etwa wenn an ein Tablet eine Maus angeschlossen wird. Ob das so bleibt, entscheidet der architekt („Offene Fragen“ 1).
- Ändert die Zusage „`Enter` sendet“ aus R00011 F0043 für Touch-Geräte; R00011 bleibt unverändert, diese Anforderung geht für Touch-Geräte vor.

## Nicht-funktionale Anforderungen

- **Geräte**: geprüft als echtes Gerät emuliert (Playwright-Kontext mit `hasTouch`, `isMobile`, `deviceScaleFactor`), nicht als verkleinertes Desktop-Fenster: Telefon 375 × 812 (Faktor 3), Tablet 768 × 1024 (Faktor 2), Desktop 1280 × 800 ohne Touch. Geprobt am 2026-10-29 mit dem Chromium des Projekts (`e2e/node_modules/playwright`): `(pointer: coarse)` trifft mit `hasTouch` zu (auch ohne `isMobile`), ohne `hasTouch` trifft `(pointer: fine)` zu.
- **Schmal**: bei 320 px Fensterbreite kein Querscrollen; mit eingeblendeter Bildschirmtastatur (genähert als Telefon 375 × 500) sind Feld und Knopf ganz sichtbar.
- **Zeitkriterien** nach `AI-Team/planer.md`: neue Zeitgrenzen gibt es nicht; „sofort vorläufig“ gilt wie Z3 in R00011.
- **Konsole**: keine unbehandelte Ausnahme, kein CSP-Verstoß, keine Konsolenzeile außer „Failed to load resource“ zu den Fehlerantworten, die ein Kriterium selbst vorschreibt (hier: 500 im Kriterium „Fehlschlag“).

## Akzeptanzkriterien

Gemeinsam für alle Kriterien, wenn nicht anders genannt: Start, `S(<name>)`, `POST`, Browser A und B und der Testdaten-Erzeuger wie in R00011. **Telefon** ist ein Browser-Kontext 375 × 812 mit `hasTouch`, `isMobile`, `deviceScaleFactor` 3; **Tablet** 768 × 1024 mit `hasTouch`, `isMobile`, `deviceScaleFactor` 2; **Desktop** 1280 × 800 ohne Touch. „Tippen“ ist `tap()` des Kontexts, „Klick“ `click()`. „Der Knopf“ ist `getByRole('button', { name: 'Senden', exact: true })` in der Eingabe. „Nicht verfügbar“ heißt im Folgenden: so gekennzeichnet, wie die Ergänzung zu ADR-0038 festlegt (Empfehlung `aria-disabled="true"`).

### F0091 Senden-Knopf neben dem Feld

- [ ] **Bereit-Regel** (Node, rein): bereit ist der Knopf bei `"a"`, `"Hallo\nWelt"`, `"﻿"` (kein `White_Space`, wie beim Server) und bei genau 16.384 Byte (5.461 × `€` + `a`); nicht bereit bei `""`, `"   "`, `" \n\t"`, `"\u0085"`, `"　"`, bei 16.385 × `a` und bei `"rot \u001b[31m"`. Die Mutanten „Leer-Prüfung mit `trim()`“ (rot bei `"\u0085"` und `"﻿"`) und „Textregel nicht geprüft“ (rot bei 16.385 Byte) werden rot.
- [ ] **Tippen sendet** (E2E, Telefon und Tablet): A wählt `# allgemein`, tippt `Hallo` ins Feld, tippt auf den Knopf → genau ein `POST` mit `{"text":"Hallo"}`; im Verlauf von A steht `Hallo` von `shoff@127.0.0.1`, erst „wird gesendet …“, dann bestätigt, ohne Doppel; das Feld ist leer und eine Zeile hoch; `document.activeElement` ist das Feld; `sessionStorage` hat keinen Eintrag `localchatA.entwurf.allgemein`.
- [ ] **Klick sendet** (E2E, Desktop): wie „Tippen sendet“ mit Klick statt Tippen → dasselbe Ergebnis, der Fokus liegt danach im Feld, nicht auf dem Knopf.
- [ ] **Tastatur** (E2E, Desktop): A tippt `eins`, drückt `Tab` → der Fokus liegt auf dem Knopf; `Enter` → `eins` wird gesendet, der Fokus liegt im Feld. A tippt `zwei`, `Tab`, `Leertaste` → `zwei` wird gesendet, der Fokus liegt im Feld. Von dort führt `Tab` zum Knopf und ein weiteres `Tab` in die Teilnehmerliste.
- [ ] **Leer und Leerraum** (E2E, Telefon und Desktop): Feld leer → der Knopf ist nicht verfügbar; Tippen bzw. Klick → kein `POST`, kein Fehler, keine vorläufige Nachricht. A tippt drei Leerzeichen → weiter nicht verfügbar, Tippen bzw. Klick → kein `POST`, das Feld behält die drei Leerzeichen. A tippt danach `x` → der Knopf ist bereit. A löscht alles → er ist wieder nicht verfügbar.
- [ ] **Textregel** (E2E, Desktop): A fügt 16.385 × `a` ein → die Hinweiszeile zeigt den Fehler wie in R00012, der Knopf ist nicht verfügbar, Klick → kein `POST`; A löscht ein Zeichen → der Knopf ist bereit. Ebenso mit `rot ` + `\u001b` + `[31m` (Steuerzeichen).
- [ ] **Doppeltipp** (E2E, Telefon): A tippt `einmal` und tippt zweimal schnell hintereinander auf den Knopf (`tap()` zweimal ohne Warten) → genau ein `POST`, `einmal` steht einmal im Verlauf. Grund: nach dem ersten Tippen ist das Feld leer und der Knopf nicht verfügbar.
- [ ] **Warteschlange** (E2E, Desktop, Antwort der Route um 2 s verzögert): A sendet `1` per Knopf, `2` per `Enter`, `3` per Knopf, direkt nacheinander → der Verlauf von A zeigt `1`, `2`, `3` in dieser Folge, je einmal; laut Netzprotokoll beginnt kein `POST`, bevor der vorige beantwortet ist; der Knopf ist während des Wartens bereit, sobald Text im Feld steht.
- [ ] **Fehlschlag** (E2E, Telefon): die Route antwortet auf den nächsten `POST` mit 500 → A tippt `kaputt` und tippt auf den Knopf → `kaputt` steht als fehlgeschlagen mit ⚠ und dem Grund aus ADR-0037 Regel 4 am Ende des Verlaufs, mit „Erneut senden“ und „Verwerfen“, genau wie nach `Enter` mit derselben Antwort; die Ansage-Region meldet den Fehlschlag einmal.
- [ ] **Ohne Verbindung** (E2E, Desktop; Gegeben wie die Trennung in R00012): der Strom ist getrennt → A sendet `später` per Knopf → `später` steht „wird gesendet …“ und wird nach dem Wiederverbinden bestätigt, wie nach `Enter`.
- [ ] **Zielfläche** (E2E): die Box des Knopfs (`getBoundingClientRect`) misst auf Telefon und Tablet mindestens 44 × 44 px, auf Desktop mindestens 24 × 24 px. Die Mutante „44 px nur bei 375 px Breite statt bei grobem Zeiger“ wird auf dem Tablet rot.
- [ ] **Ort und Schmal** (E2E): auf Telefon, Tablet und Desktop liegt der Knopf rechts neben dem Feld. Seine linke Kante liegt rechts der rechten Kante des Felds, seine Unterkante höchstens 1 px von der Unterkante des Felds entfernt, auch nach 20 Zeilen per `Shift Enter` (Feld 200 px hoch). Bei 320 px Fensterbreite (Telefon-Kontext) gilt `scrollWidth <= innerWidth` mit Feld, Knopf und einer fehlgeschlagenen Nachricht. Telefon 375 × 500: Feld und Knopf liegen ganz innerhalb von `visualViewport`.
- [ ] **Bildschirmleser** (E2E, Desktop): genau ein Element mit Rolle `button` und Namen „Senden“ in der Eingabe; im nicht bereiten Zustand trägt es die Kennzeichnung „nicht verfügbar“ und bleibt mit `Tab` erreichbar (Empfehlung; gilt in der Form, die ADR-0038 festlegt). Die Seite hat danach weiter genau eine Live-Region (`[aria-live], [role="status"], [role="alert"]` zählt 1, ADR-0038 Ergänzung Karte #36).
- [ ] **Bestand bleibt grün** (E2E und Node): die Kriterien „Tasten“, „Senden“, „Reihenfolge“, „Leerraum“, „Höhe und Schmal“ aus R00011 und „Leerraum wie beim Server“ aus R00020 laufen auf Desktop unverändert.
- [ ] **Designsystem** (Review-Punkt `ux-reviewer` und `rechtschreib-reviewer`): `Dokumentation/Wireframes/designsystem/components/Eingabe/README.md` beschreibt den Knopf: Ort, Beschriftung, Zielfläche, „nicht verfügbar“, Fokus nach dem Senden und Platz für „Anhang“. Zeile 6 nennt die Ausnahme für Touch-Geräte (F0092). `grep -n 'Senden' …/Eingabe/README.md` findet den Knopf.
- [ ] Konsole bei A: keine unbehandelte Ausnahme, kein CSP-Verstoß, keine Konsolenzeile außer „Failed to load resource“ zur 500 im Kriterium „Fehlschlag“.

### F0092 Auf Touch-Geräten macht Enter eine neue Zeile

- [ ] **Taste mit Zeigerart** (Node, rein; Erweiterung des Tasten-Tests aus R00011): mit grobem Zeiger: `Enter` → zeile; `Shift Enter` → zeile; `Strg Enter`, `Meta Enter` → senden; `Strg Enter` mit `isComposing: true` oder `keyCode: 229` → nichts; `Strg Enter` bei `" \n "` → nichts. Mit feinem Zeiger: alle Fälle aus R00011 „Tasten“ unverändert (`Enter` → senden). Die Mutante „Zeigerart ignoriert“ wird rot.
- [ ] **Telefon** (E2E): A tippt `Hallo`, drückt `Enter`, tippt `Welt` → kein `POST`; das Feld enthält `Hallo\nWelt`; A tippt auf den Knopf → eine Nachricht mit den Zeilen „Hallo“ und „Welt“.
- [ ] **Tablet mit Tastatur** (E2E): wie „Telefon“; danach tippt A `kurz` und drückt `Strg Enter` → `kurz` wird gesendet, der Fokus liegt im Feld.
- [ ] **Desktop unverändert** (E2E): A tippt `Hallo`, drückt `Enter` → `Hallo` wird gesendet (R00011 „Senden“).
- [ ] **Wechsel der Zeigerart** (E2E, nach der Entscheidung des architekt, „Offene Fragen“ 1; Empfehlung): in einem Kontext ohne Touch wird die Medienabfrage `(pointer: coarse)` per CDP-Emulation (`Emulation.setEmitTouchEventsForMouse` bzw. `setTouchEmulationEnabled`) eingeschaltet, ohne die Seite neu zu laden → der nächste `Enter` macht eine neue Zeile; ausgeschaltet → der nächste `Enter` sendet. Lässt sich die Umschaltung im Chromium des Projekts nicht herstellen, gilt das Kriterium als Node-Test mit gestellter Zeigerart, und der Bericht nennt das.
- [ ] Konsole bei A: keine unbehandelte Ausnahme, kein CSP-Verstoß, keine Konsolenzeile.

## Betroffene Verzeichnisstruktur

- Oberfläche: `web/eingabe.js` (`taste`, `eingabeAnlegen`), `web/lc-seite.css` (`.lc-comp`, `.lc-comp-box`, neue Regeln für den Knopf), ggf. `web/textregel.js` (Leer-Funktion, unverändert genutzt).
- Tests: `e2e/einheit/` (Bereit-Regel, Tasten mit Zeigerart), `e2e/tests/r00027-*.spec.js` (Geräte-Kontexte Telefon, Tablet, Desktop).
- Designsystem: `Dokumentation/Wireframes/designsystem/components/Eingabe/README.md` (ggf. `preview.html`).
- ADR: Ergänzung zu `Dokumentation/ADRs/0038-eingabe-mehrzeilig-enter-sendet-entwurf-je-channel-gebuendelte-ansage.md` (architekt, vor dem Bau).

## Technische Überlegungen

### Ablauf

1. **Bereit**: Eingabe im Feld → Textregel und Leer-Funktion → bereit | nicht bereit → Kennzeichnung am Knopf.
2. **Senden über den Knopf**: Tippen | Klick | `Enter`/`Leertaste` am Knopf → bereit? → nein: nichts | ja: derselbe Absendeweg wie `Enter` (Feld leeren, Höhe, Hinweis, Entwurf gelöscht, `senden(text)` → Zustellung ADR-0037) → Fokus im Feld.
3. **Taste im Feld** (F0092): Tastenereignis, Text, Zeigerart → `taste` → senden | zeile | nichts; mit grobem Zeiger ist `Enter` ohne Strg/⌘ eine Zeile (Verhalten des Browsers, kein `preventDefault`).

### Optionen

Zum Zustand bei leerem Feld (die Wahl bestätigt der architekt, „Offene Fragen“ 2):

- **A: `aria-disabled="true"`, fokussierbar, ohne Wirkung** (Empfehlung). Stärke: der Knopf bleibt mit `Tab` erreichbar und wird vorgelesen („Senden, nicht verfügbar“); wer ihn findet, erfährt, dass er da ist. Schwäche: die Wirkungslosigkeit muss der Code sichern, nicht der Browser.
- **B: `disabled`**. Stärke: der Browser verhindert Klick und Fokus. Schwäche: der Knopf fällt aus der Tab-Reihenfolge, ein Bildschirmleser findet ihn beim Tabben nicht; die Tab-Reihenfolge ändert sich mit dem Text.
- **C: immer bereit, ohne Wirkung bei Leerraum** (wie `Enter` heute). Stärke: einfachster Code, gleich wie die Taste. Schwäche: man sieht nicht, warum nichts passiert. Auf dem Telefon wirkt ein Tippen ohne Antwort wie ein Defekt.

Zur Erkennung des Touch-Geräts (F0092; Wahl beim architekt, „Offene Fragen“ 1):

- **A: `(pointer: coarse)`, je Tastendruck abgefragt** (Empfehlung). Stärke: eine Medienabfrage, folgt dem primären Zeiger; in Playwright mit `hasTouch` herstellbar (geprobt). Schwäche: ein Laptop mit Touchscreen hat einen feinen primären Zeiger, also sendet `Enter` dort, auch beim Tippen auf dem Bildschirm (gewollt: er hat eine echte Tastatur).
- **B: `(any-pointer: coarse)`**. Stärke: erkennt jedes Gerät mit Touch. Schwäche: macht `Enter` auch auf Laptops mit Touchscreen und echter Tastatur zur Zeile, gegen die Gewohnheit.
- **C: nach dem letzten Eingabeereignis** (`pointerType` des letzten `pointerdown`). Stärke: folgt dem, womit gerade bedient wird. Schwäche: mehr Zustand, schwer zu testen; die Bildschirmtastatur selbst liefert keine Pointer-Ereignisse.

### Grobentwurf (Vorplanung)

Bubbles aus der WBS, als Vorplanung. Welche es am Ende werden, entscheidet der Entwickler; sie sind vor der Ergänzung zu ADR-0038 geplant, ihr Inhalt folgt der Entscheidung.

- F0091: B0413 Knopf bereit (Text, Prüfergebnis → bereit | nicht bereit; rein) · B0414 Knopf zeichnen (Channel, bereit → `<button type="button">` neben dem Feld, Zielfläche nach Zeigerart; unklar: Layout, Platz für „Anhang“) · B0415 Knopf verdrahten (Tippen | Klick | Taste → Absendeweg wie `Enter` → Fokus im Feld; unklar: Bildschirmtastatur, Komposition) · B0416 Designsystem Eingabe um den Knopf ergänzt.
- F0092: noch keine Bubbles (Rolling Wave, nach der ADR-Ergänzung). Elemente: `taste` bekommt die Zeigerart als Eingabe (bleibt rein), die Abfrage der Zeigerart sitzt im Ereignis-Handler.

Wireframe: kein Artboard `D0003.dc.html` (Hinweis `/wireframe verfeinern D0003`). Gestaltungsquelle: Designsystem `components/Eingabe/README.md`; Vorgabe für die Gestaltung, nicht Akzeptanzkriterium.

### Änderungen an bestehenden Klassen

- `web/eingabe.js`: der Absendeweg aus dem `keydown`-Handler wird eine Funktion, die Taste und Knopf rufen; `taste(ereignis, text)` bekommt die Zeigerart (F0092).
- `web/lc-seite.css`: `.lc-comp-box` wird eine Zeile aus Feld und Knopf (Feld füllt den Rest), Zielfläche über `@media (pointer: coarse)`.

## Tests

Nach Skill `test-pyramide`; je Kriterium die genannte Mutante, das Ergebnis steht im Bericht des Entwicklers.

- **Node (rein)**: Bereit-Regel; Tasten mit Zeigerart.
- **E2E**: Geräte-Kontexte Telefon, Tablet, Desktop (`browser.newContext` mit `hasTouch`, `isMobile`, `deviceScaleFactor`; Telefon und Tablet mit `tap()`); Senden, Leerraum, Textregel, Doppeltipp, Warteschlange, Fehlschlag, ohne Verbindung, Zielfläche, Ort, Tastatur, Bildschirmleser; Bestand aus R00011 und R00020.
- **Review-Punkt**: Designsystem `Eingabe`.

## Abhängigkeiten

- Abhängig von: F0043 Eingabe und eigene Nachricht im Browser (gruen, R00011; Eingabe 1), F0071 Zusagen aus R00011 mit Tests belegt (gruen, R00020; Leer-Funktion `White_Space`). Mitgenutzt, ohne `Braucht` (I0020 braucht I0009): F0048 Entwurf, Längenanzeige und Textprüfung (gruen, R00012), F0046 Fehler und Wiederholung (gruen, R00012). Vor dem Bau: Ergänzung des architekt zu ADR-0038 („Offene Fragen“ 1–5).
- Innerhalb: F0092 braucht F0091 (ohne Knopf sendet auf Touch-Geräten nichts mehr).
- Berührt dieselben Dateien (keine Vorbedingung, Reihenfolge beim Mergen beachten): R00025 F0085 Verwerfen rückgängig, F0086 Hinweiszeile hilft beim Bereinigen (rot; `web/eingabe.js`, Hinweiszeile); I0010 Anhänge (rot; Schalter „Anhang“ links neben dem Knopf), I0017 Markdown (rot; Codeblock-Regel für `Enter` — auf Touch-Geräten ohne Wirkung, weil `Enter` dort ohnehin eine Zeile ist).
- Blockiert: nichts.

## Offene Fragen

An den architekt, vor dem Bau (Ergänzung zu ADR-0038); die Empfehlungen des planer stehen in den Kriterien, die Kriterien folgen der Entscheidung:

1. **Erkennung des Touch-Geräts** (F0092): `(pointer: coarse)` je Tastendruck, `(any-pointer: coarse)` oder das letzte Eingabeereignis? Was gilt für Mischgeräte (Laptop mit Touchscreen: primär fein; Tablet mit Tastatur und Trackpad)? Empfehlung: `(pointer: coarse)`, je Tastendruck abgefragt; `Strg Enter`/`⌘ Enter` senden immer. Probe (2026-10-29, Chromium des Projekts): mit `hasTouch` trifft `(pointer: coarse)` zu, ohne `(pointer: fine)`.
2. **Zustand „nicht verfügbar“** (F0091): `aria-disabled` (fokussierbar), `disabled` oder immer bereit? Empfehlung: `aria-disabled="true"`, fokussierbar, sichtbar gedämpft, ohne Wirkung; bereit genau dann, wenn `Enter` senden würde (Leer-Funktion `White_Space`, Textregel).
3. **Fokus, Bildschirmtastatur und Komposition beim Tippen auf den Knopf** (F0091): Der Fokus soll das Feld nicht verlassen, damit die Bildschirmtastatur offen bleibt, etwa über `pointerdown` mit `preventDefault`. Was geschieht dabei mit einer laufenden Komposition (Wortvorschlag unter Android)? Wird sie übernommen, bevor gesendet wird? Empfehlung: Fokus bleibt im Feld; gesendet wird der Text, der nach Abschluss der Komposition im Feld steht, nie ein halbes Wort; nach dem Senden per Tastatur geht der Fokus ins Feld zurück.
4. **Ort und Platz für „Anhang“** (F0091, I0010): Knopf rechts im Rahmen der Eingabe, bündig unten neben dem wachsenden Feld; „Anhang“ kommt mit I0010 links daneben. Bis dahin ändert sich ADR-0038 Regel 1 („bis dahin keine leeren Schalter“) nur insofern, als der Knopf kein leerer Schalter ist. Empfehlung: so; Tab-Reihenfolge Feld → (Anhang) → Senden → Teilnehmerliste.
5. **Sichtbare Beschriftung** (F0091): Text „Senden“, Symbol (etwa ➤) oder beides? Empfehlung: sichtbarer Text „Senden“, denn der Name enthält den sichtbaren Text (WCAG 2.5.3), es gibt keine Deutung eines Symbols, und bei 320 px bleibt dem Feld genug Breite. Das legt der architekt mit der Ergänzung des Designsystems fest (B0416).

An den Betreiber, nicht blockierend:

6. **Vision und Geräte**: die Vision nennt „keine Mobil-App“, aber kein Zielgerät für den Browser. Mit R00027 werden Telefon und Tablet im Browser zu geprüften Geräten. Empfehlung: in der Vision unter den Leitplanken fortschreiben („Bedienung im Browser auf Desktop, Tablet und Telefon; keine eigene App“), über `/vision fortschreiben`. Der planer schreibt die Vision nicht.

## Manuelle Vorbereitungstätigkeiten

- architekt: Ergänzung zu ADR-0038 („Offene Fragen“ 1–5).

## Manuelle Nachbereitungstätigkeiten

- Keine. Das Designsystem ergänzt B0416 im Bau.

## Umfang

Gezählt aus der WBS (2026-10-29, `/planung zaehlen` nach den Regeln des Skills `work-breakdown-structure`):

- F0091 Senden-Knopf neben dem Feld: 4 Bubbles · 2 klar (je 0,4 h Untergrenze, Richtwert, ungemessen) · 2 unklar (Bandbreite je 2–4 h) — zusammen 4,8–8,8 h.
- F0092 Auf Touch-Geräten macht Enter eine neue Zeile: nicht bis Bubble geplant. Hypothese, nicht gezählt: 2 Bubbles (Taste mit Zeigerart, Zeigerart erkennen), etwa 0,8 h zur Untergrenze.

## Warum löst diese Anforderung das Problem? (Pflicht)

Auslöser: die Entscheidung des Betreibers vom 2026-10-29. Auf Telefon und Tablet lässt sich heute nicht verlässlich senden: die Eingabetaste der Bildschirmtastatur liefert `keyCode 229`, und `taste` wertet das als Komposition (`web/eingabe.js:10`), oder sie sendet, und dann fehlt die zweite Zeile.

Die Wirkungskette:

- Ein Knopf „Senden“ neben dem Feld nutzt denselben Absendeweg wie `Enter`. Deshalb sendet ein Tippen auf jedem Gerät, mit denselben Zusagen zu Bestätigung, Fehlschlag und Warteschlange. Es gibt keinen zweiten Weg, der anders scheitert.
- Der Knopf ist nur bereit, wenn `Enter` senden würde, und ist sonst als nicht verfügbar gekennzeichnet. Deshalb sieht man vor dem Tippen, warum nichts geschieht, und ein Text nur aus Leerraum geht nie als Anfrage hinaus (R00020).
- Die Zielfläche ist auf Touch-Geräten mindestens 44 px groß. Deshalb trifft der Daumen den Knopf, ohne in das Feld oder den Verlauf zu tippen.
- Auf Touch-Geräten macht `Enter` eine Zeile. Deshalb verhält sich die Eingabetaste der Bildschirmtastatur wie erwartet, und mehrzeilige Antworten sind möglich. Wer eine echte Tastatur hat, sendet mit `Strg Enter` oder, mit feinem Zeiger, weiter mit `Enter`.

## Missing-Docs

- Glossar fehlt (`Dokumentation/Domaenenwissen/` existiert nicht); „Touch-Gerät“ und „Bildschirmtastatur“ hier eingeführt.

## Notizen

- **Weiche** (`rollen/planer/befunde-einplanen.md`): R00011 sagt kein Touch-Gerät zu, also kein aus-bug. Request und Response an der Systemgrenze bleiben das Senden aus I0009, also keine neue Interaction, sondern eine Ausbaukette „Eingabe“ unter I0009 (F0043 = Eingabe 1). Geschnitten in zwei Slices (Muster „ein Teil ist unklar, der Rest nicht“). Der Knopf allein hilft schon auf Android, wo `Enter` heute nichts tut. Die Touch-Regel für `Enter` hängt an der Erkennung und braucht den Knopf.
- Nicht hier: der Schalter „Anhang“ (I0010, Karte #9), die Codeblock-Regel für `Enter` (I0017), ein Hinweis im Platzhalter, wie man auf Touch-Geräten sendet (nicht verlangt; der Knopf ist sichtbar).
