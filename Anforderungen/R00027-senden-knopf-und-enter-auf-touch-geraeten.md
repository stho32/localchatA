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
- **Die Eingabe ist mit dem Daumen bedienbar**: beim Tippen ins Feld zoomt die Seite nicht, der Knopf hat einen sichtbaren Fokusring, und „Erneut senden“ und „Verwerfen“ an einer fehlgeschlagenen Nachricht sind auf Touch-Geräten so groß wie der Knopf (UX-Vorprüfung #41).

Zahlt ein auf: [Vision](R00000-vision.md) — „Bedienkonzept wie kanbanD“, „Live überall“ und die Leitplanke „Zielgeräte“ (2026-10-29): „Bedienung im Browser auf Desktop, Tablet und Telefon, mit Maus, Tastatur und Touch — jede Aktion auch ohne Tastenkürzel; keine eigene App.“ Senden ist heute eine Aktion nur mit Tastenkürzel; R00027 macht sie auf jedem Gerät ohne Kürzel möglich.

Herkunft: Entscheidung des Betreibers vom 2026-10-29 (Vorschlag angenommen), Anlass ist die Geräte-Regel des UX-Reviews („auf Telefon und Tablet jede Aktion einmal nur mit Touch erledigen“).

Slices (WBS `Dokumentation/Planung/localchata.md`, beide unter I0009 Nachricht senden, Ausbaukette „Eingabe“): F0091 Senden-Knopf neben dem Feld (Eingabe 2), F0092 Auf Touch-Geräten macht Enter eine neue Zeile (Eingabe 3).

### Domänenbezug

Kein Glossar im Projekt (`Dokumentation/Domaenenwissen/` fehlt). Begriffe wie in R00011, R00012 und R00020: Feld (Eingabe unter dem Verlauf), Textregel (Länge und Steuerzeichen, F0048), Leerraum (Unicode `White_Space`, R00020 F0071), vorläufig, fehlgeschlagen, Warteschlange (ADR-0037), Bildschirmleser. Neu: **Touch-Gerät** — ein Gerät, dessen primärer Zeiger grob ist (CSS `(pointer: coarse)`, je Tastendruck abgefragt; ADR-0038 Ergänzung Karte #41, Frage 1); ein feiner oder fehlender Zeiger gilt wie am Desktop; **Bildschirmtastatur** — die eingeblendete Tastatur eines Touch-Geräts; **Wortvorschlag** — das Wort, das die Bildschirmtastatur gerade bildet (Komposition), unterstrichen im Feld sichtbar.

## Geschäftlicher Nutzen

Wer auf dem Telefon mitliest, kann heute nicht verlässlich antworten. Mit Android-Tastaturen kommt `Enter` als `keyCode 229` an und wird verworfen, die Nachricht bleibt im Feld. Unter iOS sendet `Enter`, dafür lässt sich keine zweite Zeile tippen. Damit fällt das Telefon als Gerät für einen schnellen Zwischenruf aus, etwa für die Antwort an einen Agenten oder die Freigabe vom Flur. Nach R00027 sendet ein Tippen auf „Senden“ auf jedem Gerät, und auf Touch-Geräten wird `Enter` zur Zeilentaste, wie die Bildschirmtastatur sie anbietet.

## Funktionale Anforderungen

### Senden-Knopf neben dem Feld (F0091)

- **Ort**: rechts neben dem Feld, innerhalb des Rahmens der Eingabe, bündig zur Unterkante des Felds; wächst das Feld (bis 200 px), bleibt der Knopf unten neben ihm. Er erscheint mit dem Feld, also sobald ein Channel gewählt ist. Er steht in einer Gruppe von Schaltern rechts vom Feld, als letzter und damit ganz rechts; zwischen Schaltern der Gruppe liegen mindestens 8 px. Mit I0010 kommt der Schalter „Anhang“ links neben ihn in dieselbe Gruppe. Bis dahin gibt es keinen leeren Schalter und keinen reservierten leeren Raum (ADR-0038 Regel 1; Ergänzung Karte #41, Frage 4).
- **Name und Beschriftung**: sichtbarer Text „Senden“, daraus der zugängliche Name genau „Senden“, Rolle `button`, kein abweichendes `aria-label` (ADR-0038 Ergänzung Karte #41, Frage 5).
- **Zielfläche**: auf Touch-Geräten mindestens 44 × 44 px, sonst mindestens 24 × 24 px (WCAG 2.5.8; 44 px Empfehlung für Telefone). Ein Doppeltipp auf den Knopf zoomt nicht (`touch-action: manipulation`).
- **Bereit oder nicht verfügbar**: der Knopf ist bereit, wenn `Enter` im Feld senden würde — beide fragen **dieselbe** Bedingung `sendbar` (ADR-0038 Ergänzung Karte #41 K2): der Text besteht nicht nur aus Zeichen der Unicode-Eigenschaft `White_Space` (R00020 F0071), und die Textregel meldet keinen Fehler (Länge über 16.384 Byte, Steuerzeichen; R00012 F0048). Sonst ist er **als nicht verfügbar gekennzeichnet**: `aria-disabled="true"`, fokussierbar, sichtbar gedämpft (Frage 2). Ein Tippen, Klicken, `Enter` oder `Leertaste` auf ihm hat dann keine Wirkung: keine Anfrage, kein Fehler, der Text bleibt im Feld.
- **Darstellung**: bereit im Stil des Primärschalters (Fläche `kd-accent`, Text `kd-on-accent`); nicht verfügbar gedämpft (Text `kd-ink-muted` auf `kd-surface`, Rahmen `kd-line`). In beiden Zuständen und beiden Farbschemata hat der Text gegen seine Fläche mindestens 4,5 : 1, gerechnet mit den Farben, die der Browser tatsächlich zeichnet, also auch nach einer `opacity` (UX-Vorprüfung #41 S4).
- **Fokusring**: hat der Knopf den Fokus (`:focus-visible`), ist ein eigener Ring sichtbar — mindestens 2 px breit, außen um den Knopf (Versatz ≥ 0), in einer Farbe mit mindestens 3 : 1 gegen `kd-surface`, ganz innerhalb des Rahmens der Eingabe; im bereiten wie im nicht verfügbaren Zustand (UX-Vorprüfung #41 M1). Heute läge der allgemeine Ring innen auf `kd-accent` und wäre mit 1,14 : 1 (hell) bzw. 1,02 : 1 (dunkel) unsichtbar.
- **Grund am Knopf**: der Knopf verweist mit `aria-describedby` auf die Hinweiszeile unter dem Feld. Steht dort ein Fehler der Textregel, hört man ihn am Knopf mit; bei leerem Feld ist die Zeile leer und es kommt nichts hinzu (UX-Vorprüfung #41 S3).
- **Senden**: Tippen, Klick, `Enter` oder `Leertaste` am bereiten Knopf senden den Text **auf demselben Weg wie `Enter` im Feld** — eine Funktion `absenden`, die Taste und Knopf rufen (ADR-0038 Regel 3, Ergänzung Karte #41 K1; ADR-0037). Das Feld wird leer und wieder eine Zeile hoch, der Entwurf des Channels ist gelöscht, die Nachricht steht vorläufig „wird gesendet …“ am Ende des Verlaufs und springt ans Ende. Sie wird bestätigt oder als fehlgeschlagen mit Grund gezeigt, mit „Erneut senden“ und „Verwerfen“. Ohne Verbindung wartet sie wie bei `Enter`. Weitere Nachrichten reihen sich in die Warteschlange des Tabs ein, ein Sendeauftrag zugleich. Der Knopf wird während des Sendens nie gesperrt; ob er bereit ist, hängt nur am Text im Feld.
- **Wortvorschlag**: tippt man auf den Knopf, während die Bildschirmtastatur ein Wort vorschlägt, wird genau der Text gesendet, den das Feld in diesem Moment zeigt, samt dem unterstrichenen Wort — nie weniger; ein Vorschlag, den die Tastatur erst danach einsetzen würde, wird nicht abgewartet. Die Komposition wird vor dem Leeren beendet, sodass keine Tastatur das Wort danach ins leere Feld zurückschreibt (ADR-0038 Ergänzung Karte #41, Frage 3, K4).
- **Fokus**: nach dem Senden über den Knopf liegt der Fokus im Feld, auch wenn mit der Tastatur über `Tab` auf den Knopf gegangen wurde. Auf Touch-Geräten verlässt der Fokus das Feld beim Tippen auf den Knopf nicht, die Bildschirmtastatur bleibt offen (K4).
- **Kein Zoom beim Fokus**: auf Touch-Geräten ist die Schrift im Feld mindestens 16 px groß, damit Safari unter iOS beim Tippen ins Feld nicht hineinzoomt und den Knopf aus dem Bild schiebt; am Desktop bleibt sie 13 px. Gelöst über die Schriftgröße, nicht über ein Zoomverbot im Viewport (`maximum-scale`, `user-scalable`) (UX-Vorprüfung #41 M2).
- **Schalter an einer fehlgeschlagenen Nachricht**: auf Touch-Geräten sind „Erneut senden“ und „Verwerfen“ mindestens 44 × 44 px groß und mindestens 8 px voneinander entfernt; am Desktop wie bisher (UX-Vorprüfung #41 S7).
- **Tab-Reihenfolge**: Kopfzeile → Seitenleiste → Verlauf → Feld → (Anhang, ab I0010) → **Knopf** → Teilnehmerliste, allein aus der Reihenfolge im Dokument, ohne positiven `tabindex` (ändert ADR-0038 Regel 8; Ergänzung Karte #41 K7).
- **Bildschirmleser**: der Knopf meldet sich als „Senden, Schalter“, im nicht bereiten Zustand zusätzlich als nicht verfügbar. Für den Knopf gibt es keine eigene Ansage und keine neue Live-Region. Ein Fehlschlag wird wie bisher sofort über die eine Ansage-Region gemeldet (ADR-0038 Regel 10, Ergänzung Karte #36; Ergänzung Karte #41 K8).

### Auf Touch-Geräten macht Enter eine neue Zeile (F0092)

- Ist der primäre Zeiger des Geräts grob (Touch-Gerät), macht `Enter` ohne Umschalttaste im Feld **eine neue Zeile** und sendet nicht. Die Zeile schreibt der Browser bzw. die Bildschirmtastatur selbst, auch wenn sie `Enter` als `keyCode 229` liefert. `Shift Enter` macht weiter eine neue Zeile. `Strg Enter` und `⌘ Enter` senden weiter, unter denselben Bedingungen wie bisher (nicht während einer Komposition, nicht bei Leerraum oder verletzter Textregel).
- Mit feinem oder ohne Zeiger (Maus, Trackpad) gilt R00011 F0043 unverändert: `Enter` sendet.
- Ausgewertet wird bei jedem Tastendruck, nicht einmal beim Laden. Wechselt die Zeigerart ohne Neuladen, gilt beim nächsten Tastendruck die neue Regel, etwa wenn an ein Tablet eine Maus angeschlossen wird (ADR-0038 Ergänzung Karte #41, Frage 1 Option A, K5). Mischgeräte folgen dem, was der Browser als primären Zeiger meldet: ein Laptop mit Touchscreen hat einen feinen Zeiger, dort sendet `Enter`.
- Das Feld gibt der Bildschirmtastatur keinen Sende-Hinweis (`enterkeyhint="send"`): ihre Eingabetaste macht eine Zeile (K6).
- Ändert die Zusage „`Enter` sendet“ aus R00011 F0043 für Touch-Geräte; R00011 bleibt unverändert, diese Anforderung geht für Touch-Geräte vor.

## Nicht-funktionale Anforderungen

- **Geräte**: geprüft als echtes Gerät emuliert (Playwright-Kontext mit `hasTouch`, `isMobile`, `deviceScaleFactor`), nicht als verkleinertes Desktop-Fenster: Telefon 375 × 812 (Faktor 3), Tablet 768 × 1024 (Faktor 2), Tablet quer 1024 × 768 (Faktor 2), Desktop 1280 × 800 ohne Touch. Geprobt am 2026-10-29 mit dem Chromium des Projekts (`e2e/node_modules/playwright`): `(pointer: coarse)` trifft mit `hasTouch` zu (auch ohne `isMobile`), ohne `hasTouch` trifft `(pointer: fine)` zu.
- **Am Gerät**: was Chromium ohne echte Bildschirmtastatur nicht zeigt (Tastatur bleibt offen, Wortvorschlag, Zoom unter iOS, Vorlesen mit VoiceOver und TalkBack), prüft der Entwicklertest an einem echten Android-Telefon (Gboard, Chrome) und einem iPhone (Safari) — Kriterium „Am Gerät“, US-13 (UX-Vorprüfung #41 S8; ADR-0038 Ergänzung Karte #41, Früh-Indikator zu Frage 3).
- **Schmal**: bei 320 px Fensterbreite kein Querscrollen; mit eingeblendeter Bildschirmtastatur (genähert als Telefon 375 × 500 und Tablet quer 1024 × 400) sind Feld, Knopf, Hinweiszeile und die eben gesendete vorläufige Nachricht ganz sichtbar (UX-Vorprüfung #41 S1, S2).
- **Zeitkriterien** nach `AI-Team/planer.md`: neue Zeitgrenzen gibt es nicht; „sofort vorläufig“ gilt wie Z3 in R00011.
- **Konsole**: keine unbehandelte Ausnahme, kein CSP-Verstoß, keine Konsolenzeile außer „Failed to load resource“ zu den Fehlerantworten, die ein Kriterium selbst vorschreibt (hier: 500 im Kriterium „Fehlschlag“).

## Akzeptanzkriterien

Gemeinsam für alle Kriterien, wenn nicht anders genannt: Start, `S(<name>)`, `POST`, Browser A und B und der Testdaten-Erzeuger wie in R00011. **Telefon** ist ein Browser-Kontext 375 × 812 mit `hasTouch`, `isMobile`, `deviceScaleFactor` 3; **Tablet** 768 × 1024 mit `hasTouch`, `isMobile`, `deviceScaleFactor` 2; **Tablet quer** 1024 × 768 mit `hasTouch`, `isMobile`, `deviceScaleFactor` 2; **Desktop** 1280 × 800 ohne Touch. „Tippen“ ist `tap()` des Kontexts, „Klick“ `click()`. „Der Knopf“ ist `getByRole('button', { name: 'Senden', exact: true })` in der Eingabe. „Nicht verfügbar“ heißt: `aria-disabled="true"` (ADR-0038 Ergänzung Karte #41, Frage 2). „Je Thema“ heißt: einmal mit `colorScheme: 'light'`, einmal mit `'dark'`.

### F0091 Senden-Knopf neben dem Feld

- [ ] **Bereit-Regel** (Node, rein; `sendbar`, ADR-0038 Ergänzung Karte #41 K2): `sendbar` ist wahr bei `"a"`, `"Hallo\nWelt"`, `"﻿"` (kein `White_Space`, wie beim Server) und bei genau 16.384 Byte (5.461 × `€` + `a`: 5.461 × 3 + 1 = 16.384); falsch bei `""`, `"   "`, `" \n\t"`, `"\u0085"`, `"　"`, bei 16.385 × `a` und bei `"rot \u001b[31m"`. Die Mutanten „Leer-Prüfung mit `trim()`“ (rot bei `"\u0085"` und `"﻿"`) und „Textregel nicht geprüft“ (rot bei 16.385 Byte) werden rot. Der `keydown`-Handler fragt dieselbe Funktion (Review-Punkt `code-reviewer`: keine zweite Prüfung `gezeigt.stufe === 'fehler'`, heute `web/eingabe.js:67`).
- [ ] **Tippen sendet** (E2E, Telefon und Tablet): A wählt `# allgemein`, tippt `Hallo` ins Feld, tippt auf den Knopf → genau ein `POST` mit `{"text":"Hallo"}`; im Verlauf von A steht `Hallo` von `shoff@127.0.0.1`, erst „wird gesendet …“, dann bestätigt, ohne Doppel; das Feld ist leer und eine Zeile hoch; `document.activeElement` ist das Feld; `sessionStorage` hat keinen Eintrag `localchatA.entwurf.allgemein`.
- [ ] **Klick sendet** (E2E, Desktop): wie „Tippen sendet“ mit Klick statt Tippen → dasselbe Ergebnis, der Fokus liegt danach im Feld, nicht auf dem Knopf.
- [ ] **Tastatur** (E2E, Desktop): A tippt `eins`, drückt `Tab` → der Fokus liegt auf dem Knopf; `Enter` → `eins` wird gesendet, der Fokus liegt im Feld. A tippt `zwei`, `Tab`, `Leertaste` → `zwei` wird gesendet, der Fokus liegt im Feld. Von dort führt `Tab` zum Knopf und ein weiteres `Tab` in die Teilnehmerliste.
- [ ] **Leer und Leerraum** (E2E, Telefon und Desktop): Feld leer → der Knopf ist nicht verfügbar; Tippen bzw. Klick → kein `POST`, kein Fehler, keine vorläufige Nachricht. A tippt drei Leerzeichen → weiter nicht verfügbar, Tippen bzw. Klick → kein `POST`, das Feld behält die drei Leerzeichen. A tippt danach `x` → der Knopf ist bereit. A löscht alles → er ist wieder nicht verfügbar.
- [ ] **Textregel** (E2E, Desktop): A fügt 16.385 × `a` ein → die Hinweiszeile zeigt den Fehler wie in R00012, der Knopf ist nicht verfügbar, Klick → kein `POST`; A löscht ein Zeichen → der Knopf ist bereit. Ebenso mit `rot ` + `\u001b` + `[31m` (Steuerzeichen).
- [ ] **Doppeltipp** (E2E, Telefon): A tippt `einmal` und tippt zweimal schnell hintereinander auf den Knopf (`tap()` zweimal ohne Warten) → genau ein `POST`, `einmal` steht einmal im Verlauf. Grund: nach dem ersten Tippen ist das Feld leer und der Knopf nicht verfügbar.
- [ ] **Komposition** (E2E, Telefon; erster Test aus ADR-0038 Ergänzung Karte #41, K4): A tippt `Hallo ` ins Feld; per CDP `Input.imeSetComposition({text: 'Wel', selectionStart: 3, selectionEnd: 3})` läuft ein Wortvorschlag; A tippt auf den Knopf → genau ein `POST` mit `{"text":"Hallo Wel"}` — das sichtbare Wort wird gesendet, nie ein halbes; das Feld ist leer, `document.activeElement` ist das Feld. Danach CDP `Input.insertText('x')` → das Feld enthält genau `"x"` — keine Tastatur schreibt das Wort zurück. Die Mutante „ohne `blur`/`focus`“ bleibt in Chromium grün (ADR, Probe P3); sie wird im Kriterium „Am Gerät“ geprüft.
- [ ] **Warteschlange** (E2E, Desktop, Antwort der Route um 2 s verzögert): A sendet `1` per Knopf, `2` per `Enter`, `3` per Knopf, direkt nacheinander → der Verlauf von A zeigt `1`, `2`, `3` in dieser Folge, je einmal; laut Netzprotokoll beginnt kein `POST`, bevor der vorige beantwortet ist; der Knopf ist während des Wartens bereit, sobald Text im Feld steht.
- [ ] **Fehlschlag** (E2E, Telefon): die Route antwortet auf den nächsten `POST` mit 500 → A tippt `kaputt` und tippt auf den Knopf → `kaputt` steht als fehlgeschlagen mit ⚠ und dem Grund aus ADR-0037 Regel 4 am Ende des Verlaufs, mit „Erneut senden“ und „Verwerfen“, genau wie nach `Enter` mit derselben Antwort; die Ansage-Region meldet den Fehlschlag einmal.
- [ ] **Ohne Verbindung** (E2E, Desktop; Gegeben wie die Trennung in R00012): der Strom ist getrennt → A sendet `später` per Knopf → `später` steht „wird gesendet …“ und wird nach dem Wiederverbinden bestätigt, wie nach `Enter`.
- [ ] **Zielfläche** (E2E): die Box des Knopfs (`getBoundingClientRect`) misst auf Telefon und Tablet mindestens 44 × 44 px, auf Desktop mindestens 24 × 24 px. Die Mutante „44 px nur bei 375 px Breite statt bei grobem Zeiger“ wird auf dem Tablet rot.
- [ ] **Ort und Schmal** (E2E): auf Telefon, Tablet, Tablet quer und Desktop liegt der Knopf rechts neben dem Feld. Seine linke Kante liegt rechts der rechten Kante des Felds, seine Unterkante höchstens 1 px von der Unterkante des Felds entfernt, auch nach 20 Zeilen per `Shift Enter` (Feld 200 px hoch). Bei 320 px Fensterbreite (Telefon-Kontext) gilt `scrollWidth <= innerWidth` mit Feld, Knopf und einer fehlgeschlagenen Nachricht. Mit genäherter Bildschirmtastatur — Telefon 375 × 500 und Tablet quer 1024 × 400 — liegen Feld, Knopf und Hinweiszeile ganz innerhalb von `visualViewport`, auch mit dem Fehler der Textregel in der Hinweiszeile; nach dem Senden per Knopf liegt die vorläufige Nachricht „wird gesendet …“ ganz innerhalb von `visualViewport` (UX-Vorprüfung #41 S1, S2).
- [ ] **Gruppe** (E2E, Telefon und Desktop): der Knopf ist das letzte Kind der Gruppe der Schalter rechts vom Feld (`.lc-comp-aktionen`); `getComputedStyle(gruppe).columnGap` ist mindestens 8 px; es gibt dort keinen weiteren Schalter und kein leeres Element mit Rolle `button` (UX-Vorprüfung #41 S6; ADR-0038 Regel 1).
- [ ] **Fokus sichtbar** (E2E je Thema; UX-Vorprüfung #41 M1, wörtlich): „Knopf nach Tab: outline nicht none, ≥ 2 px, offset ≥ 0, Abstand zur Innenkante der Box ≥ offset+width, Ringfarbe gegen kd-surface ≥ 3:1, auch „nicht verfügbar“; Mutante „ohne eigene Fokusregel“ rot.“ Lesart: Desktop, A tippt `x`, `Tab` → am Knopf `outlineStyle !== 'none'`, `outlineWidth` ≥ 2 px, `outlineOffset` ≥ 0 px; zwischen Knopf und Innenkante von `.lc-comp-box` liegen an jeder Seite mindestens `outlineOffset` + `outlineWidth` (bei 2 px + 2 px also 4 px); `outlineColor` gegen `kd-surface` mindestens 3 : 1 (gerechnet: `kd-focus` 5,9 : 1 hell, 7,5 : 1 dunkel). Dasselbe bei leerem Feld (nicht verfügbar). Gerätezeile: Tablet mit Tastatur, `Tab` vom Feld → derselbe Ring. Die Mutante „ohne eigene Fokusregel“ (es gilt nur `.lc :focus-visible` mit `outline-offset: -2px`, Ring auf `kd-accent`, 1,14 : 1 bzw. 1,02 : 1) wird rot.
- [ ] **Kein Zoom beim Fokus** (E2E; UX-Vorprüfung #41 M2, wörtlich): „bei (pointer: coarse) Feld font-size ≥ 16 px, Desktop 13 px; Mutante „nur bei 375 px“ auf Tablet rot; Gerät: iPhone Safari scale = 1, Knopf ganz sichtbar; nicht über maximum-scale=1.“ Lesart: auf Telefon, Tablet und Tablet quer ist `getComputedStyle(feld).fontSize` ≥ 16 px, am Desktop genau 13 px; das `content` von `<meta name="viewport">` enthält weder `maximum-scale` noch `user-scalable`. Die Mutante „16 px nur bei höchstens 375 px Breite statt bei grobem Zeiger“ wird auf dem Tablet rot. Gerätezeile im Kriterium „Am Gerät“.
- [ ] **Kontrast** (E2E je Thema, Desktop und Telefon; UX-Vorprüfung #41 S4): der Text des Knopfs hat gegen seine Fläche mindestens 4,5 : 1 — bereit und nicht verfügbar —, gerechnet aus den berechneten Farben einschließlich `opacity` des Knopfs und seiner Vorfahren (gerechnet mit den Tokens: bereit 6,72 : 1 hell, 8,87 : 1 dunkel; nicht verfügbar 6,75 : 1 hell, 6,68 : 1 dunkel). Die Mutante „`opacity: .5` auf nicht verfügbar“ wird rot.
- [ ] **Grund am Knopf** (E2E, Desktop; UX-Vorprüfung #41 S3): `aria-describedby` des Knopfs verweist auf die Hinweiszeile unter dem Feld. A fügt 16.385 × `a` ein → die zugängliche Beschreibung des Knopfs (Barrierefreiheitsbaum) enthält den Text der Hinweiszeile „⚠ Zu lang: …“; bei leerem Feld ist sie leer. Die Mutante „ohne `aria-describedby`“ wird rot.
- [ ] **Bildschirmleser** (E2E, Desktop): genau ein Element mit Rolle `button` und Namen „Senden“ in der Eingabe, ohne `aria-label`, ohne Attribut `disabled`; im nicht bereiten Zustand trägt es `aria-disabled="true"` und bleibt mit `Tab` erreichbar. Die Seite hat danach weiter genau eine Live-Region (`[aria-live], [role="status"], [role="alert"]` zählt 1, ADR-0038 Ergänzung Karte #36).
- [ ] **Schalter der fehlgeschlagenen Nachricht** (E2E, Telefon und Tablet; UX-Vorprüfung #41 S7): Gegeben wie „Fehlschlag“ → „Erneut senden“ und „Verwerfen“ messen je mindestens 44 × 44 px (`getBoundingClientRect`), zwischen ihnen liegen mindestens 8 px; am Desktop unverändert (Höhe `kd-row`, 28 px). Tippen auf „Erneut senden“ → `kaputt` wird gesendet und bestätigt, genau einmal. Die Mutante „44 px nur bei 375 px Breite statt bei grobem Zeiger“ wird auf dem Tablet rot.
- [ ] **Bestand bleibt grün** (E2E und Node): die Kriterien „Tasten“, „Senden“, „Reihenfolge“, „Leerraum“, „Höhe und Schmal“ aus R00011 und „Leerraum wie beim Server“ aus R00020 laufen auf Desktop unverändert.
- [ ] **Designsystem** (Review-Punkt `ux-reviewer` und `rechtschreib-reviewer`): `Dokumentation/Wireframes/designsystem/components/Eingabe/README.md` beschreibt den Knopf: Ort in der Gruppe (ganz rechts, ≥ 8 px Abstand), Beschriftung, Zielfläche, „nicht verfügbar“, Fokusring außen, Fokus nach dem Senden, Schriftgröße des Felds auf Touch-Geräten (16 px), kein Sende-Hinweis an die Bildschirmtastatur und Platz für „Anhang“. Zeile 6 nennt die Ausnahme für Touch-Geräte (F0092). `designsystem/1-tastatur.md` nennt in den Zeilen `Enter` und `Strg Enter` die Ausnahme für Touch-Geräte (UX-Vorprüfung #41 S5). `grep -n 'Senden' …/Eingabe/README.md` findet den Knopf, `grep -n 'Touch' …/1-tastatur.md` die Ausnahme.
- [ ] **Am Gerät** (Entwicklertest, nicht automatisierbar; UX-Vorprüfung #41 S8, M2-Gerätezeile; ADR-0038 Ergänzung Karte #41, Früh-Indikator zu Frage 3): an einem Android-Telefon mit Gboard (Chrome) und an einem iPhone (Safari), je in `# allgemein`:
  - ins Feld tippen → die Seite zoomt nicht (iPhone: `visualViewport.scale` bleibt 1, oder sichtbar: Schrift und Knopf behalten ihre Größe), Feld und Knopf sind ganz sichtbar;
  - ein Wort tippen und **ohne** Leerzeichen auf „Senden“ tippen → das ganze Wort geht hinaus, die Bildschirmtastatur bleibt offen (ein kurzes Flackern wird notiert), das nächste getippte Zeichen steht allein im Feld;
  - nach dem Senden sind die vorläufige und dann die bestätigte Nachricht über der Tastatur sichtbar;
  - mit VoiceOver bzw. TalkBack wird der Knopf als „Senden, Taste“ bzw. „Senden, Schaltfläche“ vorgelesen, bei leerem Feld zusätzlich als nicht verfügbar (gedimmt).
  Flackert die Tastatur störend oder steht das Wort nach dem Senden wieder im Feld, ist das ein Befund an F0091 (ADR-0038: Wechsel auf Option B über eine neue Ergänzung).
- [ ] Konsole bei A: keine unbehandelte Ausnahme, kein CSP-Verstoß, keine Konsolenzeile außer „Failed to load resource“ zur 500 im Kriterium „Fehlschlag“.

### F0092 Auf Touch-Geräten macht Enter eine neue Zeile

- [ ] **Taste mit Zeigerart** (Node, rein; Erweiterung des Tasten-Tests aus R00011): mit grobem Zeiger: `Enter` → zeile; `Shift Enter` → zeile; `Strg Enter`, `Meta Enter` → senden; `Strg Enter` mit `isComposing: true` oder `keyCode: 229` → nichts; `Strg Enter` bei `" \n "` → nichts; `Enter` mit `keyCode: 229` → nichts (Android: die Zeile schreibt die Tastatur, der Handler verhindert sie nicht). Mit feinem Zeiger: alle Fälle aus R00011 „Tasten“ unverändert (`Enter` → senden). Die Mutante „Zeigerart ignoriert“ wird rot (ADR-0038 Ergänzung Karte #41 K5, erster Test).
- [ ] **Telefon** (E2E): A tippt `Hallo`, drückt `Enter`, tippt `Welt` → kein `POST`; das Feld enthält `Hallo\nWelt`; A tippt auf den Knopf → eine Nachricht mit den Zeilen „Hallo“ und „Welt“.
- [ ] **Tablet mit Tastatur** (E2E): wie „Telefon“; danach tippt A `kurz` und drückt `Strg Enter` → `kurz` wird gesendet, der Fokus liegt im Feld.
- [ ] **Desktop unverändert** (E2E): A tippt `Hallo`, drückt `Enter` → `Hallo` wird gesendet (R00011 „Senden“).
- [ ] **Wechsel der Zeigerart** (E2E, Desktop-Kontext; ADR-0038 Ergänzung Karte #41, Frage 1, Probe P2): A tippt `eins`; CDP `Emulation.setTouchEmulationEnabled({enabled: true})`, ohne die Seite neu zu laden; A drückt `Enter` und tippt `zwei` → kein `POST`, das Feld enthält `eins\nzwei`. CDP `{enabled: false}`; A drückt `Enter` → genau ein `POST` mit `{"text":"eins\nzwei"}`. Geprüft wird das Verhalten, nicht die Medienabfrage (nach dem Ausschalten ist auch `(pointer: fine)` falsch, Zeiger „keiner“ — er gilt wie am Desktop). Die Mutante „Zeigerart einmal beim Laden bestimmt“ wird rot.
- [ ] **Kein Sende-Hinweis** (E2E, Telefon): das Feld hat kein Attribut `enterkeyhint` mit dem Wert `send` (K6).
- [ ] Konsole bei A: keine unbehandelte Ausnahme, kein CSP-Verstoß, keine Konsolenzeile.

## Betroffene Verzeichnisstruktur

- Oberfläche: `web/eingabe.js` (`sendbar`, `taste`, `eingabeAnlegen` mit `absenden`), `web/lc-seite.css` (`.lc-comp-box` als Zeile, Gruppe `.lc-comp-aktionen`, Knopf, Fokusring, Schriftgröße des Felds und `.lc-send-schalter button` unter `@media (pointer: coarse)`), `web/textregel.js` (`textPruefen`, unverändert genutzt).
- Tests: `e2e/einheit/` (`sendbar`, Tasten mit Zeigerart), `e2e/tests/r00027-*.spec.js` (Geräte-Kontexte Telefon, Tablet, Tablet quer, Desktop; CDP für Komposition und Wechsel der Zeigerart).
- Designsystem: `Dokumentation/Wireframes/designsystem/components/Eingabe/README.md` (ggf. `preview.html`), `Dokumentation/Wireframes/designsystem/1-tastatur.md`.
- ADR: `Dokumentation/ADRs/0038-eingabe-mehrzeilig-enter-sendet-entwurf-je-channel-gebuendelte-ansage.md`, Ergänzung Karte #41 (entschieden 2026-10-29).

## Technische Überlegungen

### Ablauf

1. **Bereit**: Eingabe im Feld, Senden, Channelwechsel → `sendbar(text)` (`textPruefen(text).art === 'ok'`) → `aria-disabled` am Knopf.
2. **Senden über den Knopf**: `pointerdown` am Knopf → `preventDefault()`, wenn das Feld den Fokus hat → `click` (Tippen, Klick, `Enter`/`Leertaste` am Knopf) → `aria-disabled`? ja: nichts | nein: läuft eine Komposition, `feld.blur()`, `feld.focus()` → `absenden()`.
3. **`absenden()`** (der eine Absendeweg, K1): Text lesen → `sendbar`? nein: nichts | ja: Feld leer, Höhe, Hinweis, Entwurf gelöscht, `senden(text)` → Zustellung (ADR-0037) → Fokus ins Feld.
4. **Taste im Feld** (F0092): `keydown` → `matchMedia('(pointer: coarse)')` → `taste(ereignis, text, 'grob' | 'fein')` → senden: `preventDefault()`, `absenden()` | zeile oder Komposition: nichts tun (die Zeile schreibt der Browser bzw. die Bildschirmtastatur) | `Enter` ohne Wirkung mit feinem Zeiger: `preventDefault()`.

### Optionen

Entschieden in ADR-0038 Ergänzung Karte #41 (2026-10-29), je Option A — Abwägung dort:

- Zustand bei leerem Feld: **A `aria-disabled="true"`**, fokussierbar, ohne Wirkung (gewählt); B `disabled`; C immer bereit.
- Erkennung des Touch-Geräts: **A `(pointer: coarse)` je Tastendruck** (gewählt); B `(any-pointer: coarse)`; C Art des letzten Zeigers; D einmal beim Laden.
- Fokus und Komposition: **A `preventDefault` auf `pointerdown`, Komposition mit `blur()`/`focus()` beenden** (gewählt); B ohne Beenden; C Knopf während Komposition wirkungslos; D Fokus wandert.
- Ort: **A Gruppe rechts im Rahmen** (gewählt); B eigene Zeile darunter; C außerhalb des Rahmens. Beschriftung: **A Text „Senden“** (gewählt); B Symbol; C beides.

Schriftgröße auf Touch-Geräten (M2): **16 px im Feld bei `(pointer: coarse)`** statt Zoomverbot im Viewport (`maximum-scale=1`) — ein Zoomverbot nähme Sehbehinderten das Vergrößern (WCAG 1.4.4); Vorgabe der UX-Vorprüfung #41, keine weitere Option.

### Grobentwurf (Vorplanung)

Bubbles aus der WBS, als Vorplanung; welche es am Ende werden, entscheidet der Entwickler.

- F0091: B0413 Bereit-Regel `sendbar` (rein; ersetzt die Prüfung im `keydown`-Handler) · B0414 Knopf zeichnen (Gruppe, Zustände, Zielfläche, `aria-describedby`, Kontrast; K3, K7) · B0415 Knopf verdrahten (`absenden`, `pointerdown`, Komposition, Fokus; K1, K4; E2E „Komposition“) · B0416 Designsystem Eingabe und Tastatur ergänzt · B0480 Fokusring des Knopfs sichtbar (M1) · B0481 Feld ohne Zoom auf Touch-Geräten (M2) · B0482 Schalter an fehlgeschlagener Nachricht auf Touch-Geräten (S7).
- F0092: B0483 Taste mit Zeigerart (K5, rein) · B0484 Zeigerart je Tastendruck (Handler, kein `preventDefault` bei Zeile, kein `enterkeyhint`; E2E Telefon, Tablet mit `Strg Enter`, Desktop, Wechsel über CDP).

Wireframe: kein Artboard `D0003.dc.html` (Hinweis `/wireframe verfeinern D0003`). Gestaltungsquelle: Designsystem `components/Eingabe/README.md`; Vorgabe für die Gestaltung, nicht Akzeptanzkriterium.

### Änderungen an bestehenden Klassen

- `web/eingabe.js`: neu `sendbar(text)` (rein, exportiert); `taste(ereignis, text, zeiger)` bekommt die Zeigerart; in `eingabeAnlegen` wird der Absendeweg aus dem `keydown`-Handler die Funktion `absenden()`, die Taste und Knopf rufen; der Knopf entsteht dort mit der Gruppe.
- `web/lc-seite.css`: `.lc-comp-box` wird eine Zeile aus Feld und Gruppe (Feld füllt den Rest); Knopf mit eigener `:focus-visible`-Regel (außen); unter `@media (pointer: coarse)`: Zielfläche des Knopfs, Schriftgröße des Felds 16 px, `.lc-send-schalter button` 44 × 44 px.

## Tests

Nach Skill `test-pyramide`; je Kriterium die genannte Mutante, das Ergebnis steht im Bericht des Entwicklers.

- **Node (rein)**: `sendbar`; Tasten mit Zeigerart.
- **E2E**: Geräte-Kontexte Telefon, Tablet, Tablet quer, Desktop (`browser.newContext` mit `hasTouch`, `isMobile`, `deviceScaleFactor`, `colorScheme` je Thema; Touch-Kontexte mit `tap()`); Senden, Leerraum, Textregel, Doppeltipp, Komposition (CDP `Input.imeSetComposition`, `Input.insertText`), Warteschlange, Fehlschlag, ohne Verbindung, Zielfläche, Ort und Schmal (mit `visualViewport`), Gruppe, Fokus sichtbar, Kein Zoom beim Fokus, Kontrast, Grund am Knopf, Bildschirmleser, Schalter der fehlgeschlagenen Nachricht, Tastatur; F0092 Telefon, Tablet mit Tastatur, Desktop, Wechsel der Zeigerart (CDP `Emulation.setTouchEmulationEnabled`), kein Sende-Hinweis; Bestand aus R00011 und R00020.
- **Am Gerät** (Entwicklertest): Android mit Gboard, iPhone mit Safari — Kriterium „Am Gerät“, US-13.
- **Review-Punkte**: Designsystem `Eingabe` und `1-tastatur.md`; `code-reviewer`: niemand außer `absenden` ruft `senden` aus `eingabe.js`, keine zweite Bereit-Prüfung, kein `disabled` am Knopf, kein `enterkeyhint="send"`.

## Abhängigkeiten

- Abhängig von: F0043 Eingabe und eigene Nachricht im Browser (gruen, R00011; Eingabe 1), F0071 Zusagen aus R00011 mit Tests belegt (gruen, R00020; Leer-Funktion `White_Space`). Mitgenutzt, ohne `Braucht` (I0020 braucht I0009): F0048 Entwurf, Längenanzeige und Textprüfung (gruen, R00012), F0046 Fehler und Wiederholung (gruen, R00012). Architektur: ADR-0038 Ergänzung Karte #41 (entschieden).
- Innerhalb: F0092 braucht F0091 (ohne Knopf sendet auf Touch-Geräten nichts mehr).
- Berührt dieselben Dateien (keine Vorbedingung, Reihenfolge beim Mergen beachten): R00025 F0085 Verwerfen rückgängig (rot; `web/verlauf.js`, `.lc-send-schalter` — „Rückgängig“ erbt die Touch-Größe aus B0482, wenn es dort steht), F0086 Hinweiszeile hilft beim Bereinigen (rot; `web/eingabe.js`, Hinweiszeile — ein Schalter „Steuerzeichen entfernen“ dort braucht auf Touch-Geräten ebenfalls 44 px); I0010 Anhänge (rot; Schalter „Anhang“ in der Gruppe links neben dem Knopf), I0017 Markdown (rot; Codeblock-Regel für `Enter` — auf Touch-Geräten ohne Wirkung, weil `Enter` dort ohnehin eine Zeile ist).
- Blockiert: nichts.

## Offene Fragen

Beantwortet (ADR-0038 Ergänzung Karte #41, 2026-10-29; Vision 2026-10-29):

1. **Erkennung des Touch-Geräts**: `(pointer: coarse)`, je Tastendruck abgefragt; fein und keiner wie Desktop; `Strg Enter`/`⌘ Enter` senden immer (Option A, K5).
2. **Zustand „nicht verfügbar“**: `aria-disabled="true"`, fokussierbar, gedämpft, ohne Wirkung; bereit genau dann, wenn `sendbar` (Option A, K2, K3).
3. **Fokus, Bildschirmtastatur, Komposition**: `preventDefault` auf `pointerdown`, laufende Komposition mit `blur()`/`focus()` beenden, gesendet wird der sichtbare Text; Früh-Indikator am Gerät (Option A, K4).
4. **Ort und Platz für „Anhang“**: Gruppe rechts im Rahmen, „Senden“ ganz rechts, Tab-Reihenfolge aus dem DOM (Option A, K3, K7).
5. **Sichtbare Beschriftung**: Text „Senden“ (Option A).
6. **Vision und Geräte**: Leitplanke „Zielgeräte“ in der Vision fortgeschrieben (Betreiber 2026-10-29).

Offen, nicht blockierend:

7. **Fokus nach „Erneut senden“ auf Touch-Geräten** (an den architekt; Optional der UX-Vorprüfung #41): R00012 legt den Fokus danach ins Feld; auf Telefon und Tablet öffnet das die Bildschirmtastatur, auch wenn man nur erneut senden wollte. Soll der Fokus dort auf dem Schalter bzw. der Nachricht bleiben? Empfehlung: R00012 unverändert lassen, bis der Entwicklertest am Gerät (Kriterium „Am Gerät“) zeigt, dass es stört — dann als Befund an R00012/R00025 F0085 (das den Fokus nach „Rückgängig“ schon festlegt). Kein Kriterium hier.

## Manuelle Vorbereitungstätigkeiten

- Keine mehr. Die Ergänzung zu ADR-0038 liegt vor (Karte #41, 2026-10-29).
- Für den Entwicklertest: ein Android-Telefon mit Gboard und ein iPhone im selben LAN wie die Instanz (Kriterium „Am Gerät“); fehlt eines, nennt der Durchlauf das, statt „Am Gerät“ zu bestehen.

## Manuelle Nachbereitungstätigkeiten

- Keine. Das Designsystem ergänzt B0416 im Bau.

## Umfang

Gezählt aus der WBS (2026-10-29, `/planung zaehlen` nach den Regeln des Skills `work-breakdown-structure`):

- F0091 Senden-Knopf neben dem Feld: 7 Bubbles · 7 klar (je 0,4 h Untergrenze, Richtwert, ungemessen) — zusammen 2,8 h zur Untergrenze. Vorher 4 Bubbles, davon 2 unklar (2–4 h); B0414 und B0415 sind nach der ADR-Ergänzung klar, B0480–B0482 kommen aus der UX-Vorprüfung #41 hinzu.
- F0092 Auf Touch-Geräten macht Enter eine neue Zeile: 2 Bubbles · 2 klar (je 0,4 h, Richtwert, ungemessen) — 0,8 h; die Hypothese „2 Bubbles“ hat gehalten.
- Zusammen 9 Bubbles, 3,6 h zur Untergrenze; kein Puffer eingerechnet. Nicht gezählt: die Prüfung am Gerät (Phase Entwicklertest, kein Aufwand in der WBS).

## Warum löst diese Anforderung das Problem? (Pflicht)

Auslöser: die Entscheidung des Betreibers vom 2026-10-29. Auf Telefon und Tablet lässt sich heute nicht verlässlich senden: die Eingabetaste der Bildschirmtastatur liefert `keyCode 229`, und `taste` wertet das als Komposition (`web/eingabe.js:10`), oder sie sendet, und dann fehlt die zweite Zeile.

Die Wirkungskette:

- Ein Knopf „Senden“ neben dem Feld nutzt denselben Absendeweg wie `Enter`. Deshalb sendet ein Tippen auf jedem Gerät, mit denselben Zusagen zu Bestätigung, Fehlschlag und Warteschlange. Es gibt keinen zweiten Weg, der anders scheitert.
- Der Knopf ist nur bereit, wenn `Enter` senden würde, und ist sonst als nicht verfügbar gekennzeichnet. Deshalb sieht man vor dem Tippen, warum nichts geschieht, und ein Text nur aus Leerraum geht nie als Anfrage hinaus (R00020).
- Die Zielfläche ist auf Touch-Geräten mindestens 44 px groß. Deshalb trifft der Daumen den Knopf, ohne in das Feld oder den Verlauf zu tippen.
- Auf Touch-Geräten macht `Enter` eine Zeile. Deshalb verhält sich die Eingabetaste der Bildschirmtastatur wie erwartet, und mehrzeilige Antworten sind möglich. Wer eine echte Tastatur hat, sendet mit `Strg Enter` oder, mit feinem Zeiger, weiter mit `Enter`.
- Ein Tippen auf den Knopf mit offenem Wortvorschlag sendet das sichtbare Wort und beendet die Komposition, bevor das Feld leer wird. Deshalb geht unter Android das letzte Wort nicht verloren und kehrt nicht ins leere Feld zurück.
- Auf Touch-Geräten hat das Feld 16 px Schrift. Deshalb zoomt Safari unter iOS beim Tippen ins Feld nicht, und der Knopf bleibt neben dem Feld im Bild, statt aus ihm geschoben zu werden.
- Der Knopf hat einen eigenen, außen liegenden Fokusring. Deshalb sieht, wer mit der Tastatur (auch am Tablet) mit `Tab` auf den Knopf geht, wo der Fokus ist — vorher läge er unsichtbar auf der Akzentfarbe.

## Missing-Docs

- Glossar fehlt (`Dokumentation/Domaenenwissen/` existiert nicht); „Touch-Gerät“, „Bildschirmtastatur“ und „Wortvorschlag“ hier eingeführt.

## Notizen

- **Weiche** (`rollen/planer/befunde-einplanen.md`): R00011 sagt kein Touch-Gerät zu, also kein aus-bug. Request und Response an der Systemgrenze bleiben das Senden aus I0009, also keine neue Interaction, sondern eine Ausbaukette „Eingabe“ unter I0009 (F0043 = Eingabe 1). Geschnitten in zwei Slices (Muster „ein Teil ist unklar, der Rest nicht“). Der Knopf allein hilft schon auf Android, wo `Enter` heute nichts tut. Die Touch-Regel für `Enter` hängt an der Erkennung und braucht den Knopf.
- Nicht hier: der Schalter „Anhang“ (I0010, Karte #9), die Codeblock-Regel für `Enter` (I0017), ein Hinweis im Platzhalter, wie man auf Touch-Geräten sendet (nicht verlangt; der Knopf ist sichtbar).
- **Review 2026-10-29 (Karte #41, planer, `/anforderung review` still)**: eingearbeitet die ADR-0038 Ergänzung Karte #41 (Fragen 1–5 je Option A, K1–K8; `sendbar` als eine Bedingung für Taste und Knopf; Kriterium „Komposition“ neu; „Wechsel der Zeigerart“ verhaltensbasiert, die Rückfallklausel auf einen Node-Test entfällt) und die UX-Vorprüfung #41 (Kommentar auf der Karte, 2026-10-29). Zuordnung der Vorprüfung:
  - **M1** Fokus sichtbar → Kriterium „Fokus sichtbar“ (wörtlich, mit Lesart, Mutante und Gerätezeile Tablet mit Tastatur), Funktionale Anforderung „Fokusring“, WBS B0480.
  - **M2** Kein Zoom beim Fokus → Kriterium „Kein Zoom beim Fokus“ (wörtlich, mit Lesart und Mutante), Gerätezeile iPhone Safari im Kriterium „Am Gerät“, Funktionale Anforderung, WBS B0481.
  - **S1** Tablet quer 1024 × 768 und 1024 × 400 → Gerät „Tablet quer“, Kriterium „Ort und Schmal“.
  - **S2** Hinweiszeile und vorläufige Nachricht bei offener Tastatur → Kriterium „Ort und Schmal“, US-1.
  - **S3** Grund am Knopf → Kriterium „Grund am Knopf“, WBS B0414.
  - **S4** Kontrast je Thema inkl. `opacity` → Kriterium „Kontrast“ (mit gerechneten Werten), Funktionale Anforderung „Darstellung“, WBS B0414.
  - **S5** `designsystem/1-tastatur.md` → Kriterium „Designsystem“, WBS B0416.
  - **S6** Gruppe ≥ 8 px, „Senden“ ganz rechts → Kriterium „Gruppe“, Funktionale Anforderung „Ort“, WBS B0414. Die Zielfläche je Schalter ≥ 44 px gilt für „Anhang“ mit I0010; heute gibt es nur „Senden“.
  - **S7** „Erneut senden“/„Verwerfen“ auf Touch ≥ 44 px → **hier** (Kriterium „Schalter der fehlgeschlagenen Nachricht“, WBS B0482), nicht als Anmerkung zu R00025 F0085: R00027 macht Telefon und Tablet zu geprüften Geräten des Sendens, US-5 tippt auf dem Telefon schon auf „Erneut senden“, und die Änderung ist eine CSS-Regel an `.lc-send-schalter`; F0085 ist ungebaut und erbt sie für „Rückgängig“, wenn es dort steht.
  - **S8** Gerätezeilen → Kriterium „Am Gerät“, US-13.
  - **Optional** Telefon quer 812 × 200: nicht eingeplant — bei 200 px Höhe bleibt neben Kopfzeile, Feld und Tastatur kein Verlauf; was dort sichtbar sein soll, wäre eine Gestaltungsfrage über die Eingabe hinaus (Hinweis an den Director, eigener Slice, falls gewünscht). **Optional** Fokus nach „Erneut senden“ auf Touch: „Offene Fragen“ 7, nicht blockierend.
