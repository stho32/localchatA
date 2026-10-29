---
id: R00030
status: Neu
datum: 2026-10-29
---

# R00030: Anhänge im Browser hochladen und anzeigen

## Beschreibung

R00029 legt Anhänge über die API ab und liefert sie aus. Im Browser gibt es dafür noch nichts. Danach gilt:

- **Dateien wählen.** In der Eingabe steht vor „Senden“ ein Schalter „Anhang“. Ein Tippen, ein Klick, `Enter`/`Leertaste` auf ihm oder `Strg Shift U` im Feld öffnet die Dateiauswahl des Systems — auf dem Telefon also auch Kamera und Fotos, wie das System sie anbietet. Gewählte Dateien (höchstens 10) stehen als Zeilen über dem Feld mit Name, Größe und „Entfernen“.
- **Senden mit Dateien**, mit „Senden“ oder per Taste wie bisher, auch ohne Text. Die Nachricht erscheint sofort im eigenen Verlauf als „wird hochgeladen …“; jede Datei wird angemeldet (ist der Platz für Anhänge voll oder die Datei zu groß, steht das an der Datei, bevor etwas lädt) und dann mit Balken und Prozent hochgeladen, mit „Abbrechen“ und bei Fehlschlag „Erneut hochladen“. Danach wird die Nachricht gesendet wie jede andere. Was in der Zwischenzeit als Text gesendet wird, geht sofort hinaus.
- **Nichts geht still verloren.** Wer während des Hochladens neu lädt, wird gefragt und findet danach seinen Text und einen Hinweis, welche Dateien neu zu wählen sind; bricht die Verbindung ab, wartet die Datei und lädt danach selbst weiter.
- **Anhänge sehen.** Jede Nachricht zeigt ihre Anhänge unter dem Text: Bilder als Vorschau (Hochformatfotos richtig herum), groß im Dialog; Video und Audio im Player des Browsers, der erst lädt, wenn jemand abspielt; jeder Anhang mit Datei-Zeile, „Öffnen“ (wo der Browser die Art darstellt) und „Laden“.
- **Gelöschte Anhänge.** Löscht jemand einen Anhang (über die API oder später im Laufwerk), zeigt der Verlauf an seiner Stelle eine Zeile „Gelöscht“ — sofort in allen offenen Tabs. Gelöscht wird nicht im Verlauf, sondern im Laufwerk (Karte #45).

Alles mit Touch auf Telefon und Tablet, mit Maus und Tastatur am Desktop und mit Bildschirmleser.

Zahlt ein auf: [Vision](R00000-vision.md) — „Anhänge“, „Live überall“ (auch Löschungen erscheinen sofort), „Laufwerk“ (Löschen dort, nicht im Verlauf) und die Leitplanke „Zielgeräte“ (Desktop, Tablet, Telefon; jede Aktion auch ohne Tastenkürzel). Kein Widerspruch zur Vision.

Herkunft: WBS I0010 „Anhänge senden und öffnen“ (Karte #9); [ADR-0046](../Dokumentation/ADRs/0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) Regel 13 und Frage 7, [ADR-0047](../Dokumentation/ADRs/0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md) Regeln 5–7, [ADR-0038](../Dokumentation/ADRs/0038-eingabe-mehrzeilig-enter-sendet-entwurf-je-channel-gebuendelte-ansage.md) Ergänzung Karte #41 (Gruppe `.lc-comp-aktionen`: „Anhang“ vor „Senden“; bei 320 px eher Symbolschalter mit zugänglichem Namen), Betreiber B1, B3, B5. Angeglichen am 2026-10-29 an die UX-Vorprüfung #43 (`ux-reviewer`, Muss M1–M4, Sollte, Antworten auf die Offenen Fragen 3–6) und an Betreiber F1′ und F4 ([ADR-0047](../Dokumentation/ADRs/0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md) Ergänzungen „Karte #9 Betreiber F1′“ Regeln 3, 4 und „F4“; [ADR-0046](../Dokumentation/ADRs/0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) Ergänzung „F1′“ Regel 2).

Slices (WBS `Dokumentation/Planung/localchata.md`, unter I0010): F0101 Hochladen im Browser (Ausbaustufe „Hochladen 1“), F0102 Anhänge im Verlauf zeigen, F0118 Gelöschte Anhänge im Verlauf, F0119 Hochladen übersteht Neuladen und Verbindungsabbruch. Nicht beauftragt: F0104 Dateien ablegen und einfügen (`ausbau`, „Hochladen 2“).

### Domänenbezug

Kein Glossar im Projekt. Begriffe wie in R00011, R00027 und [R00029](R00029-anhaenge-ueber-die-api-ablegen-senden-und-ausliefern.md) (Anhang, Anmeldung, Platz für Anhänge, Art; Feld, Knopf „Senden“, vorläufig, fehlgeschlagen, Warteschlange, Ansage-Region, Touch-Gerät, Bildschirmtastatur). Neu: **Schalter „Anhang“**; **Datei-Zeile** (Typkürzel, Name, Größe, „Öffnen“, „Laden“); **Vorschau** (verkleinertes Bild im Verlauf); **Player** (Wiedergabe des Browsers); **gelöscht** (Zustand eines Anhangs nach R00029 F0116, im Verlauf als Zeile „Gelöscht“).

## Geschäftlicher Nutzen

Die Betreiberin schickt vom Telefon ein Foto vom Whiteboard oder einen Bildschirmausschnitt an die Agenten, ohne an den Rechner zu gehen; am Desktop zieht ein Entwickler ein Log heran. Wer mitliest, sieht Bilder gleich im Verlauf und spielt eine Bildschirmaufnahme ab, ohne sie herunterzuladen. Weil der Chat während des Hochladens weiterläuft, hält eine große Datei niemanden auf.

## Funktionale Anforderungen

### Hochladen im Browser (F0101)

- **Schalter „Anhang“**: `<button type="button">` in der Gruppe `.lc-comp-aktionen` rechts vom Feld, in DOM- und Tab-Reihenfolge vor „Senden“: Feld → **Anhang** → Senden → Teilnehmerliste (ADR-0038 Ergänzung Karte #41, K7). Zugänglicher Name genau „Anhang“, `aria-keyshortcuts="Control+Shift+U"`. Sichtbar unter 480 px Fensterbreite (`max-width: 479px`) das Symbol ⎘, sonst der Text „Anhang“ (UX-Antwort auf Frage 3; im Designsystem als benannte Ausnahme, B0454). Das Feld behält bei 375 px mindestens 160 px und bei 320 px mindestens 100 px Breite, „Anhang“ und „Senden“ stehen rechts davon in einer Zeile, kein Querscrollen (UX M1). Zielfläche auf Touch-Geräten (`(pointer: coarse)`) mindestens 44 × 44 px, sonst mindestens 24 × 24 px. Erscheint mit dem Feld, sobald ein Channel gewählt ist.
- **Dateiauswahl**: Tippen, Klick, `Enter` oder `Leertaste` auf dem Schalter oder `Strg Shift U` im Feld öffnen die Dateiauswahl des Systems mit Mehrfachauswahl (`<input type="file" multiple>`, ohne `accept` — jede Datei, B1). Der Fokus kehrt danach ins Feld zurück.
- **Gewählte Dateien** stehen als Zeilen über dem Feld in einer Liste `role="list"` mit dem Namen „Angehängte Dateien“ (Typkürzel aus der Endung, nur zur Anzeige; Name mit Ellipse; Größe in Worten; „Entfernen“ mit zugänglichem Namen „<name> entfernen“); die Liste ist höchstens 3½ Zeilen hoch und scrollt in sich. Höchstens 10 je Nachricht; wer mehr wählt, behält die ersten bis zur Zehn, und die Hinweiszeile sagt „Höchstens 10 Dateien je Nachricht — <k> nicht übernommen.“ Die Liste gehört zum Channel und bleibt im Speicher des Tabs beim Channelwechsel erhalten; beim Neuladen bleiben nur Text und Dateinamen für den Hinweis (F0119), nie der Inhalt.
- **Auswahl angesagt** (UX M2): nach jeder Auswahl meldet die eine Ansage-Region einmal „<n> Dateien angehängt: <namen>“ bzw. „Datei angehängt: <name>“, bei mehr als zehn mit dem Zusatz „ — <k> nicht übernommen, höchstens 10 je Nachricht“.
- **Bereit**: „Senden“ und die Taste senden, wenn der Text sendbar ist **oder** mindestens eine Datei gewählt ist und der Text die Textregel nicht verletzt (Leerraum allein ist mit Datei erlaubt, B5). `sendbar` aus ADR-0038 Ergänzung Karte #41 K2 wächst um die Dateien; ein Absendeweg (K1); `Enter` am Desktop bzw. `Strg Enter` auf dem Tablet sendet auch nur mit Dateien.
- **Senden mit Dateien**: Feld und Liste werden leer, am Ende des eigenen Verlaufs steht ein lokaler Eintrag „wird hochgeladen …“ mit Text und je Datei einer Zeile. Zuerst werden **alle** Dateien angemeldet (`POST /api/anhaenge`). Eine Abweisung erscheint an ihrer Datei mit ⚠ und dem Text der Instanz — 507 `voll`: „Nicht hochgeladen: der Platz für Anhänge ist voll — frei sind <frei>, die Datei braucht <braucht>. Platz entsteht, wenn Anhänge gelöscht werden oder ältere Tage entfallen; Text kann weiter gesendet werden.“; 422 `zu_gross`; 422 `name` — und „Entfernen“; 429 und 507 `hochladungen` sind keine Abweisung: die Datei zeigt „wartet · neuer Versuch in <s> s“ und meldet nach `Retry-After` selbst neu an. Die übrigen Dateien laden weiter, nichts wird gesendet, solange eine Datei abgewiesen und nicht entfernt ist.
- **Hochladen**: je Datei ein `PUT` über `XMLHttpRequest` mit Sitzungs-Header; Balken 4 px in `kd-accent` mit Prozent, `progressbar` mit Namen „<name> wird hochgeladen“ und `aria-valuenow` 0–100; höchstens 2 Hochladungen zugleich je Browser über alle Tabs (Web Locks), weitere zeigen sichtbar „wartet“. 422 `metadaten` nach dem letzten Byte (R00029 F0115) → ⚠ mit dem Text der Instanz und „Entfernen“. „Abbrechen“ (Name „<name>: Hochladen abbrechen“) bricht ab, gibt die Anmeldung frei (`DELETE`) und nimmt die Datei aus dem Eintrag; der Fokus geht auf das nächste „Abbrechen“, ohne eines ins Feld, nie auf `body`. War es die letzte Datei, verschwindet der Eintrag ohne Text; mit Text wird der Text gesendet. Fehlschlag bei bestehender Verbindung (408, 5xx, Abbruch) → ⚠ mit Grund und „Erneut hochladen“ (Name „<name> erneut hochladen“); es lädt dieselbe Anmeldung neu, nach `abgelaufen` meldet der Browser neu an, der Fokus liegt danach auf „Abbrechen“ derselben Datei. Ohne Verbindung gilt F0119.
- **Verwerfen** (nach Betreiber U2, Empfehlung — Offene Frage 3): der Eintrag „wird hochgeladen …“ trägt „Verwerfen“; es bricht alle Hochladungen ab (`DELETE` je Anmeldung) und legt den Text zurück ins Feld, wenn es leer ist, sonst nennt die Hinweiszeile „Verworfen: <Anfang des Texts>“.
- **Danach senden** (N1): sind alle Dateien oben, tritt die Nachricht mit `{text, anhaenge}` in die Sendeschlange des Tabs (ADR-0037) und wird wie jede Nachricht vorläufig, bestätigt oder fehlgeschlagen gezeigt. Text, der während des Hochladens gesendet wird, wartet nicht und erscheint vorher; die Nachricht mit Anhängen steht danach an der Stelle ihrer Serverzeit. Ein Channelwechsel unterbricht das Hochladen nicht; zurück im Channel steht der Eintrag mit aktuellem Prozent bzw. die gesendete Nachricht.
- **Ansage** über die eine Ansage-Region (ADR-0038 Regel 10, Ergänzung Karte #36), nicht über ein zusätzliches `role="alert"` — Abweichung vom Designsystem, B0454 zieht es nach (UX-Antwort auf Frage 4): die Auswahl einmal (oben); ein Fehler sofort, außerhalb der Bündelung, als „Nicht hochgeladen: <name> — <Grund>“; das Ende einmal je Nachricht, nicht je Datei: „<n> Dateien hochgeladen, Nachricht wird gesendet.“. Fortschritt wird nicht angesagt.
- **Fokus sichtbar** (UX M3, wie R00027 M1): an „Anhang“, „<name> entfernen“, „Abbrechen“, „Erneut hochladen“, „Verwerfen“, Vorschau (Umriss mindestens 2 px außerhalb), „Öffnen“, „Laden“ und „Schließen“: `outline` nicht `none`, mindestens 2 px, Abstand ≥ 0, Kontrast ≥ 3 : 1 gegen `kd-surface`.
- **Zielflächen** bei `(pointer: coarse)`: „<name> entfernen“, „Abbrechen“, „Erneut hochladen“, „Verwerfen“, „Öffnen“, „Laden“ und „Schließen“ je mindestens 44 × 44 px, zwischen zwei Schaltern mindestens 8 px; am Desktop mindestens 24 × 24 px.
- **Kontrast**: Name, Größe, „wartet“, „wird hochgeladen …“, Prozent, Fehlertext und der Platzhalter der Vorschau mindestens 4,5 : 1 einschließlich `opacity` der Vorfahren; der Balken mindestens 3 : 1 gegen seine Spur.
- **Abgelegte Datei**: bis F0104 gebaut ist, lässt eine auf die Seite gezogene Datei die Seite unverändert (`dragover`/`drop` mit `preventDefault`, keine Navigation); angehängt wird über „Anhang“.

### Anhänge im Verlauf zeigen (F0102)

- Jede Nachricht — im Verlauf, live aus dem Strom, die eigene nach der Bestätigung — zeigt ihre Anhänge in Sendefolge unter dem Text, gestapelt mit `kd-space-4`; ohne Text nur die Anhänge.
- **Bilder** (Art `image/*` mit Maßen, höchstens 10 MiB, höchstens 50 Megapixel, jede Seite höchstens 16.384 px): Vorschau bis 360 × 240 px im Seitenverhältnis als `<img loading="lazy" decoding="async" width height alt="<anzeigename>">`; der Platz steht, bevor ein Byte lädt (bis dahin `kd-surface-2` mit Name und Maßen). Die Vorschau zeichnet mit `object-fit: contain`: ein Foto mit EXIF-`Orientation` 5–8 (die R00029 F0115 stehen lässt) erscheint gedreht und unverzerrt im reservierten Platz. Scheitert die Bildanfrage (`error`), bleibt der Platz und zeigt „Bild nicht verfügbar.“. Tippen, Klick oder `Enter` auf die Vorschau (Schalter „<anzeigename> groß anzeigen“) öffnet das Bild groß in einem `<dialog>` über `showModal()` mit dem zugänglichen Namen = Anzeigename (in den Bildschirm eingepasst, `alt` = Anzeigename, „Schließen“); beim Öffnen liegt der Fokus auf „Schließen“, `Tab` bleibt im Dialog; `Esc`, „Schließen“ oder Tippen daneben schließt ihn, der Fokus kehrt auf die Vorschau zurück. Größere Bilder erscheinen nur als Datei-Zeile — mit „Öffnen“.
- **Video** (`video/*`): `<video controls preload="none" playsinline>` in einem Rahmen 360 × 202 px (16 : 9, `object-fit: contain`); schmaler als 360 px: volle Breite im selben Verhältnis. **Audio** (`audio/*`): `<audio controls preload="none">` in einer Zeile fester Höhe, 360 px breit bzw. volle Breite, nie breiter als die Nachricht. Beide tragen `aria-label` = Anzeigename. Nie `autoplay`; vor „Abspielen“ keine Anfrage; Spulen über Teilabruf (R00029 F0100). Meldet der Player `error`, steht darunter „Kann im Browser nicht abgespielt werden — „Laden“ nutzen.“, die Höhe bleibt. Solange ein Player spielt, folgt der Verlauf neuen Nachrichten nicht (nach Betreiber U1, Empfehlung — Offene Frage 2); die Lesestelle bleibt.
- **Datei-Zeile** unter jedem Anhang (auch unter Vorschau und Player): Typkürzel, Name mit Ellipse, Größe in Worten, „Öffnen“ und „Laden“; höchstens 420 px breit. „Öffnen“ = `<a target="_blank" rel="noopener">` nur für Bilder (jeder Größe), PDF und Text bis 16 MiB; Video und Audio öffnen im Player (M1), alles andere nur „Laden“. „Laden“ = `<a download="<anzeigename>">` für alle. Zugängliche Namen „<name> öffnen“ und „<name> laden“. Name und Größe nur über `textContent`.
- **Größe in Worten**: unter 1.024 Bytes „<n> Byte“; unter 1 MiB „<n> KiB“ (ganz, kaufmännisch gerundet, mindestens 1; ergibt die Rundung 1.024 KiB, gilt „1,0 MiB“ — UX-Antwort auf Frage 5); ab 1 MiB wie ADR-0044 Regel 4 (MiB bzw. ab 1 GiB GiB, eine Nachkommastelle, deutsches Komma). Rechenbeispiel: 812 → „812 Byte“; 307.200 → „300 KiB“; 2.457.600 → „2,3 MiB“; 1.610.612.736 → „1,5 GiB“.
- **App-CSP** wächst um `media-src 'self'` (ADR-0014 Ergänzung Karte #9) — ohne sie spielt kein Player.
- **Nichts springt**: Vorschau und Player haben ihre Höhe, bevor geladen wird; der Verlauf verschiebt sich durch das Laden um höchstens 1 px (L4).
- **Kein Lösch-Knopf**: im Verlauf gibt es keinen Weg, einen Anhang zu löschen — gelöscht wird im Laufwerk (D0006, Karte #45; ADR-0047 Ergänzung „F1′“ Regel 4).

### Gelöschte Anhänge im Verlauf (F0118)

- Ein Anhang mit `zustand: "geloescht"` (R00029 F0117) erscheint als Datei-Zeile mit Typkürzel, Name, Größe und „Gelöscht“ (Wortlaut beim `ux-reviewer`), ohne „Öffnen“, „Laden“, Vorschau, Player und ohne reservierten Bildplatz — im Verlauf, live und nach dem Neuladen gleich.
- Trifft ein Ereignis `anhaenge` eine Nachricht im Fenster, ersetzt diese Zeile Bild oder Player ohne Neuladen; die Höhe ändert sich dabei einmal (L4 für diesen Fall hingenommen, ADR-0047 Ergänzung „F1′“ Regel 4), die Lesestelle bleibt. Nach `NeuerStand` oder einem Wiederverbinden übernimmt das Nachholen den `zustand` schon gezeigter Nachrichten (Ausnahme von ADR-0028 Regel 5, nur dieses Feld).
- Ein `error` auf `<img>`, `<video>` oder `<audio>` ohne bekannten Zustand zeigt die Fehlerzustände aus F0102, nicht „Gelöscht“ — ein Ladefehler belegt kein Löschen (Netz, Codec). Abweichung vom letzten Halbsatz der ADR-Regel 4, Offene Frage 5.
- Wer gelöscht hat, zeigt die Zeile nicht (bis zur Antwort auf F7, R00029).

### Hochladen übersteht Neuladen und Verbindungsabbruch (F0119)

- **Neuladen** (UX M4): solange ein Eintrag „wird hochgeladen …“ offen ist, fragt der Browser vor dem Neuladen oder Schließen nach (`beforeunload`). Wird trotzdem verlassen, merkt sich der Tab Text und Dateinamen je Channel in `sessionStorage` (nie den Inhalt); nach dem Neuladen steht der Text wieder im Feld, wenn es leer ist, und die eine Ansage-Region und die Hinweiszeile sagen „Hochladen unterbrochen — <namen> bitte erneut wählen.“ (Wortlaut beim `ux-reviewer`). Die Anmeldungen verfallen am Server von selbst. Ohne offenen Eintrag gibt es keine Rückfrage.
- **Ohne Verbindung**: bricht die Verbindung während Anmelden oder Hochladen ab, zeigt die Datei „wartet auf Verbindung“ — kein ⚠, kein „Erneut hochladen“. Nach „Wieder verbunden“ meldet der Browser selbst neu an bzw. lädt dieselbe Anmeldung neu (nach `abgelaufen` neue Anmeldung) und sendet danach.

## Nicht-funktionale Anforderungen

- **Geräte** wie R00027: geprüft als emuliertes Gerät (Playwright-Kontext mit `hasTouch`, `isMobile`, `deviceScaleFactor`): Telefon 375 × 812 (Faktor 3), Tablet 768 × 1024 (Faktor 2), Tablet quer 1024 × 768 (Faktor 2), Desktop 1280 × 800 ohne Touch. Telefon und Tablet werden nur durch Tippen bedient (`tap()`), die Dateiauswahl über das Ereignis `filechooser`.
- **Schmal**: bei 320 px Fensterbreite (Telefon-Kontext) kein Querscrollen mit Feld, beiden Schaltern, drei gewählten Dateien, einem Eintrag „wird hochgeladen …“, einer Vorschau, einem Video, einem Audio-Player und einer Datei-Zeile mit langem Namen.
- **Leistung** (Zweck: „Channel öffnet sofort“, Lastprofil Q1b ≤ 500 ms; QS3, QS5): ein Channel mit 100 Bildnachrichten öffnet in derselben Zeit wie ohne Bilder (≤ 500 ms bis zur gezeichneten letzten Nachricht, gemessen wie R00007); ein Channel mit 20 Video- und Audionachrichten stellt beim Öffnen keine einzige Anfrage an `/anhaenge/`.
- **Text während des Hochladens** (Zweck: B2 „non-blocking“, Zustellung < 1 s): während eine Datei hochlädt, steht ein per `Enter` gesendeter Text spätestens 1 s nach dem Tastendruck bestätigt im eigenen Verlauf und im Verlauf von B (erwartet unter 100 ms im LAN; obere Grenze aus dem Lastprofil, Uhr ab dem Tastendruck).
- **Konsole**: keine unbehandelte Ausnahme, kein CSP-Verstoß, keine Konsolenzeile außer „Failed to load resource“ zu den Antworten, die ein Kriterium vorschreibt: 507 („Kein Platz“, „Wartet“), 422 („Zu groß und beschädigt“), 404 („Abgelaufen“, „Fehlerzustände“), der im Test abgebrochene `PUT` („Abbrechen“, „Fehlschlag“, „Ohne Verbindung“) und die Anfragen der Seite, solange sie offline ist („Ohne Verbindung“).

## Akzeptanzkriterien

Gemeinsam für alle Kriterien, wenn nicht anders genannt: Start, Beitritte per `curl`, `S(<name>)` und Browser A (`shoff`) und B (`anna`) wie in R00011; Quoten wie R00029 (`--plattenquote 10MiB --anhangquote 10MiB`, Anhang-Quote 10.485.760 Bytes). **Telefon**, **Tablet**, **Desktop** wie R00027, **Tablet quer** 1024 × 768 mit `hasTouch`, `isMobile`, `deviceScaleFactor` 2. „Der Schalter“ ist `getByRole('button', { name: 'Anhang', exact: true })` in der Eingabe, „der Knopf“ der Knopf „Senden“ aus R00027. „Wählen“ heißt: Schalter auslösen, das Ereignis `filechooser` abwarten, `setFiles(...)`. Testdateien entstehen im Test: `foto.png` (PNG 1280 × 800, ≈ 300 KiB), `bericht.pdf` (2.457.600 Bytes, beginnt mit `%PDF-1.7`), `log.txt` (Text, 40 KiB), `film.webm` und `ton.ogg` (6 s, mit `ffmpeg` im Setup), `archiv.zip` (Zufallsbytes, 1 MiB), `hochkant.jpg` (JPEG 4000 × 3000 mit EXIF `Orientation` 6 und GPS-Ort, ≈ 1,2 MiB). „Gegeben über die API“ heißt: `beta` lädt per `curl` hoch und sendet (R00029).

### F0101 Hochladen im Browser

- [ ] **Bereit-Regel** (Node, rein): `sendbar("", 1 Datei)` → wahr; `sendbar("   ", 1 Datei)` → wahr (Text wird `""`); `sendbar("", 0)` → falsch; `sendbar(16.385 × "a", 1 Datei)` → falsch (Textregel); `sendbar("a", 0)` → wahr wie R00027. Mutante „Dateien ignoriert“ wird rot.
- [ ] **Wählen und senden auf dem Telefon** (E2E, Telefon): A wählt `# allgemein`, tippt auf den Schalter, wählt `foto.png`, tippt auf den Knopf → über dem Feld stand bis dahin eine Zeile `foto.png` mit „300 KiB“ und „Entfernen“; danach genau ein `POST /api/anhaenge`, ein `PUT`, ein `POST …/nachrichten` mit `{"text":"","anhaenge":["<id>"]}`; im Verlauf von A und B steht die Nachricht von `shoff@127.0.0.1` mit dem Bild (F0102), genau einmal; Feld und Liste sind leer, der Fokus liegt im Feld.
- [ ] **Tablet** (E2E, Tablet): wie Telefon mit `bericht.pdf` und dem Text `Bericht Q3` → die Nachricht trägt Text und Datei-Zeile.
- [ ] **Desktop mit Maus und Tastatur** (E2E, Desktop): Klick auf den Schalter → `filechooser`; `Strg Shift U` im Feld → `filechooser`; `Tab` vom Feld → Schalter, `Enter` → `filechooser`, `Leertaste` → `filechooser`; ein weiteres `Tab` → Knopf „Senden“, danach Teilnehmerliste. Gewählt `log.txt` und `archiv.zip`, Text `zwei Dateien`, `Enter` im Feld → gesendet mit beiden Anhängen in Wahlreihenfolge.
- [ ] **Zehn Dateien** (E2E, Desktop): 12 Dateien wählen → 10 Zeilen, Hinweis „Höchstens 10 Dateien je Nachricht — 2 nicht übernommen.“; „Entfernen“ an der dritten → 9 Zeilen, Fokus auf dem nächsten „Entfernen“ bzw. im Feld, wenn keine Zeile mehr da ist.
- [ ] **Channelwechsel** (E2E, Desktop): `foto.png` in `# allgemein` gewählt, Wechsel nach `# technik` → keine Zeile; zurück nach `# allgemein` → die Zeile ist wieder da.
- [ ] **Kein Platz** (E2E, Desktop; Betreiber F1′): Gegeben über die API, dass `beta` neun Anhänge zu 1.048.576 Bytes und einen zu 843.776 Bytes gesendet hat (G = 10.280.960, frei 204.800) → A wählt `foto.png` (307.200 Bytes) und sendet → an der Datei ⚠ „Nicht hochgeladen: der Platz für Anhänge ist voll — frei sind 0,2 MiB, die Datei braucht 0,3 MiB. Platz entsteht, wenn Anhänge gelöscht werden oder ältere Tage entfallen; Text kann weiter gesendet werden.“ und „Entfernen“; kein `PUT`, kein `POST …/nachrichten`; die Ansage-Region meldet einmal „Nicht hochgeladen: foto.png — …“. A entfernt die Datei → der Eintrag verschwindet (ohne Text) bzw. wird als Text gesendet (mit Text `trotzdem`).
- [ ] **Wartet** (E2E, Desktop): Gegeben über die API, dass `beta` 10.485.760 Bytes angemeldet hält (507 `hochladungen` für jede weitere Anmeldung) → A sendet `foto.png` → an der Datei „wartet · neuer Versuch in 30 s“, kein ⚠; `beta` verwirft seine Anmeldung (`DELETE`) → spätestens 40 s nach der ersten Abweisung (30 s `Retry-After` + 30 %) meldet der Browser neu an, lädt hoch und sendet. In „Höchstens zwei zugleich“ steht an der dritten Datei sichtbar „wartet“, bis ihr `PUT` beginnt.
- [ ] **Zu groß und beschädigt** (E2E, Desktop): eine Datei von 10.485.761 Bytes → an der Datei ⚠ mit dem Text der 422 `zu_gross` („… größer als der ganze Platz für Anhänge dieser Instanz (10,0 MiB).“), kein `PUT`; ein JPEG, dessen EXIF-Block über das Dateiende zeigt → nach dem Hochladen ⚠ mit dem Text der 422 `metadaten`, „Entfernen“, nichts gesendet.
- [ ] **Fortschritt** (E2E, Desktop, Hochladen über CDP `Network.emulateNetworkConditions` auf 1 MiB/s gedrosselt): `bericht.pdf` → der Balken (`role="progressbar"`) steigt in mindestens zwei Schritten zwischen 0 und 100 und zeigt danach keinen Balken mehr; der Eintrag hieß bis dahin „wird hochgeladen …“. Greift die Drosselung beim Hochladen im Chromium des Projekts nicht, gilt das Kriterium als Node-Test der Fortschrittsrechnung, und der Bericht nennt das.
- [ ] **Höchstens zwei zugleich** (E2E, Desktop): zwei Tabs desselben Browsers, je 3 Dateien gedrosselt → laut Netzprotokoll laufen nie mehr als 2 `PUT` gleichzeitig; alle 6 kommen an.
- [ ] **Abbrechen** (E2E, Telefon, gedrosselt): A sendet `foto.png` und `bericht.pdf`, tippt während des Hochladens auf „Abbrechen“ an `bericht.pdf` → `DELETE` für dessen ID, `/api/gesundheit` zeigt danach `reserviert` 0 für A; die Nachricht wird mit nur `foto.png` gesendet; der Fokus lag nach dem Tippen auf dem nächsten „Abbrechen“ (`document.activeElement` nie `body`). Mit Text `nur Text` und nur `bericht.pdf`: „Abbrechen“ → `nur Text` wird als Text gesendet.
- [ ] **Verwerfen** (E2E, Desktop, gedrosselt; nach Betreiber U2, Empfehlung): A sendet `bericht.pdf` mit `Hallo` und wählt am Eintrag „Verwerfen“ → `DELETE` für die Anmeldung, der Eintrag ist weg, `Hallo` steht wieder im Feld; mit `Neu` im Feld steht stattdessen „Verworfen: Hallo“ in der Hinweiszeile.
- [ ] **Fehlschlag und Erneut** (E2E, Desktop): der erste `PUT` wird im Test abgebrochen (`route.abort()`) → ⚠ mit Grund „Keine Antwort vom Server.“ und „Erneut hochladen“ an der Datei; „Erneut hochladen“ → ein neuer `PUT` auf dieselbe ID, der Fokus liegt danach auf „Abbrechen“ derselben Datei, danach wird gesendet. Ist die Anmeldung inzwischen abgelaufen (Antwort 404 `abgelaufen`, im Test vorgegeben), meldet der Browser neu an und lädt hoch.
- [ ] **Text geht vor** (E2E, Desktop, gedrosselt; N1): A sendet `film.webm` (vergrößert auf 8 MiB) mit Text `Video`, tippt während des Hochladens `zwischendurch` und drückt `Enter` → `zwischendurch` steht spätestens 1 s nach dem Tastendruck bestätigt bei A und B; danach erscheint `Video` mit dem Player, in beiden Verläufen nach `zwischendurch`.
- [ ] **Bildschirmleser** (E2E, Desktop): genau ein Element mit Rolle `button` und Namen „Anhang“ in der Eingabe, mit `aria-keyshortcuts="Control+Shift+U"`; die gewählten Dateien stehen in einer Liste „Angehängte Dateien“, jede mit einem Schalter „<name> entfernen“; der Balken ist `progressbar` „<name> wird hochgeladen“ mit `aria-valuenow`; „Abbrechen“ heißt „<name>: Hochladen abbrechen“, „Erneut hochladen“ „<name> erneut hochladen“; ein Fehler steht genau einmal in der einen Ansage-Region; die Seite hat weiter genau eine Live-Region.
- [ ] **Auswahl angesagt** (E2E, Desktop; UX M2): A wählt `foto.png` und `log.txt` → die Ansage-Region meldet genau einmal „2 Dateien angehängt: foto.png, log.txt“; nur `log.txt` → „Datei angehängt: log.txt“; 12 Dateien → „10 Dateien angehängt: … — 2 nicht übernommen, höchstens 10 je Nachricht“. Nach dem Senden von zwei Dateien meldet sie einmal „2 Dateien hochgeladen, Nachricht wird gesendet.“. Mutante „keine Ansage“ wird rot.
- [ ] **Fokus sichtbar** (E2E, Desktop; UX M3): an „Anhang“, „<name> entfernen“, „Abbrechen“, „Erneut hochladen“, „Verwerfen“, Vorschau, „Öffnen“, „Laden“ und „Schließen“ ist nach `Tab` der berechnete `outline-style` nicht `none`, `outline-width` ≥ 2 px, `outline-offset` ≥ 0 (Vorschau ≥ 2 px), Kontrast der Umrissfarbe ≥ 3 : 1 gegen `kd-surface`. Mutante „nur allgemeine Regel“ wird rot.
- [ ] **Liste mit Tastatur** (E2E; Sollte): mit 10 gewählten Dateien und genäherter Bildschirmtastatur (Telefon 375 × 500, Tablet quer 1024 × 400) liegen Feld, „Anhang“, „Senden“ und die Hinweiszeile ganz innerhalb von `visualViewport`; die Liste ist höchstens 3½ Zeilen hoch und scrollt in sich.
- [ ] **Taste ohne Text** (E2E; Node): Desktop, Feld leer, `log.txt` gewählt, `Enter` im Feld → genau ein Absenden mit `{"text":"","anhaenge":[…]}`; Tablet: dasselbe mit `Strg Enter`. Node: `taste` erhält die Zahl der Dateien; Mutante „Leer-Prüfung ohne Dateien“ wird rot.
- [ ] **Channelwechsel während des Hochladens** (E2E, Desktop, gedrosselt): A sendet `bericht.pdf` in `# allgemein` und wechselt nach `# technik` → das Hochladen läuft weiter, die Nachricht kommt in `# allgemein` an; zurück steht der Eintrag mit aktuellem Prozent bzw. die gesendete Nachricht.
- [ ] **Abgelegte Datei** (E2E, Desktop): eine auf den Verlauf gezogene Datei (`dispatchEvent` von `dragover` und `drop` mit `DataTransfer`) → die Adresse ist unverändert, keine Navigation, keine Datei in der Liste.
- [ ] **Kontrast** (E2E): Name, Größe, „wartet“, „wird hochgeladen …“, Prozent, Fehlertext und Platzhalter der Vorschau ≥ 4,5 : 1 gegen ihren Hintergrund einschließlich `opacity` der Vorfahren; der Balken ≥ 3 : 1 gegen seine Spur. Mutante „`opacity: .6` am Eintrag“ wird rot.
- [ ] **Zielfläche und Ort** (E2E, Telefon, Tablet, Tablet quer, Desktop; UX M1): der Schalter misst auf Touch-Geräten mindestens 44 × 44 px, auf Desktop mindestens 24 × 24 px; „<name> entfernen“, „Abbrechen“, „Erneut hochladen“, „Verwerfen“, „Öffnen“, „Laden“ und „Schließen“ auf Touch-Geräten ebenso mit mindestens 8 px Abstand zueinander; der Schalter liegt links von „Senden“ und rechts vom Feld, beide in einer Zeile. Mit drei gewählten Dateien und einem Eintrag „wird hochgeladen …“: bei 375 px ist das Feld mindestens 160 px breit, bei 320 px mindestens 100 px, und `scrollWidth <= innerWidth`. Bei 479 px zeigt der Schalter ⎘, bei 480 px den Text „Anhang“; der zugängliche Name ist immer „Anhang“. Mutante „44 px nur bei 375 px Breite“ wird auf dem Tablet rot. „Wählen und senden“ läuft auch auf Tablet quer.
- [ ] **Bestand bleibt grün**: die Kriterien aus R00011 „Tasten“, „Senden“, „Reihenfolge“ und aus R00027 laufen unverändert — außer R00027 „Gruppe“, das mit R00030 so gilt: „Senden“ ist das letzte Kind von `.lc-comp-aktionen`, „Anhang“ das einzige weitere, `columnGap` mindestens 8 px.
- [ ] **Designsystem** (Review-Punkt `ux-reviewer`, `rechtschreib-reviewer`): `components/Anhang/README.md` und `components/Eingabe/README.md` beschreiben Schalter (⎘ unter 480 px als benannte Ausnahme), „wird hochgeladen …“, „wartet“, „wartet auf Verbindung“, „Gelöscht“, „Erneut hochladen“, Ansage über die eine Region; „Zu groß: sofort beim Auswählen geprüft“ ist ersetzt durch die Abweisung bei der Anmeldung; Größen in KiB/MiB statt „2,4 MB“; Ablegen und Einfügen stehen als Ausbaustufe.
- [ ] Konsole bei A nach der Regel unter „Nicht-funktionale Anforderungen“.

### F0102 Anhänge im Verlauf zeigen

- [ ] **Darstellung wählen** (Node, rein): `image/png` 1280 × 800, 300 KiB → Vorschau 360 × 225; `image/png` 800 × 1200 → Vorschau 160 × 240; `image/jpeg` 20.000 × 20.000 → Zeile mit „Öffnen“; `image/png` 10 MiB + 1 Byte → Zeile mit „Öffnen“; `image/jpeg` 4000 × 3000, 12 MiB → Zeile mit „Öffnen“; jeder Anhang mit `zustand: "geloescht"` → `Geloescht` (F0118); `video/webm` → Video; `audio/ogg`, `audio/mpeg` → Audio; `application/pdf` und `text/plain` bis 16 MiB → Zeile mit „Öffnen“; `text/plain` über 16 MiB (`attachment`) und `application/octet-stream` → Zeile ohne „Öffnen“ (Betreiber F3 bestätigt).
- [ ] **Größe in Worten** (Node, rein): 0 → „0 Byte“, 812 → „812 Byte“, 1.023 → „1.023 Byte“, 1.024 → „1 KiB“, 307.200 → „300 KiB“, 1.048.063 → „1.023 KiB“, 1.048.064 → „1,0 MiB“, 1.048.575 → „1,0 MiB“, 1.048.576 → „1,0 MiB“, 2.457.600 → „2,3 MiB“, 1.610.612.736 → „1,5 GiB“.
- [ ] **Bild** (E2E, Desktop; Gegeben über die API): Nachricht mit `foto.png` → `<img>` mit `width="360" height="225"`, `loading="lazy"`, `alt="foto.png"`; die Unterkante der Nachricht verschiebt sich zwischen „vor dem Laden“ und „Bild geladen“ um höchstens 1 px (Antwort der Bildanfrage im Test um 1 s verzögert). Klick → `<dialog>` (modal, zugänglicher Name „foto.png“) mit dem Bild (`alt="foto.png"`), der Fokus liegt auf „Schließen“, `Tab` bleibt im Dialog; `Esc` → zu, Fokus auf der Vorschau.
- [ ] **Hochformatfoto** (E2E, Desktop; Betreiber F4, ADR-0047 Ergänzung „F4“): `hochkant.jpg` (4000 × 3000, `Orientation` 6) über die API gesendet → die Vorschau zeigt das Bild hochkant und unverzerrt (gezeichnetes Seitenverhältnis 3 : 4 ± 1 %, `object-fit: contain`) im reservierten Platz, die Unterkante verschiebt sich beim Laden um höchstens 1 px. Der reservierte Platz folgt den gelieferten `breite`/`hoehe` (360 × 270 bis zur Antwort auf Offene Frage 4; liefert R00029 sie getauscht, 180 × 240).
- [ ] **Fehlerzustände** (E2E, Desktop): die Bildanfrage antwortet im Test 404 → der reservierte Platz bleibt, darin „Bild nicht verfügbar.“; ein Video, das der Browser nicht abspielen kann (im Test `error` ausgelöst), → darunter „Kann im Browser nicht abgespielt werden — „Laden“ nutzen.“; die Höhe der Nachricht ändert sich in beiden Fällen nicht.
- [ ] **Bild auf dem Telefon** (E2E, Telefon): Tippen auf die Vorschau → Dialog, das Bild passt in 375 × 812 ohne Querscrollen; Tippen auf „Schließen“ → zu.
- [ ] **Video und Audio** (E2E, Desktop): Nachrichten mit `film.webm` und `ton.ogg` → `<video controls preload="none" playsinline aria-label="film.webm">` im Rahmen 360 × 202 px bzw. `<audio controls preload="none" aria-label="ton.ogg">`, ohne `autoplay`; bis zum Abspielen keine Anfrage an `/anhaenge/`; `play()` → die Wiedergabe läuft; Sprung ans Ende (`currentTime = duration − 0,1`) → eine Anfrage mit `Range`, Antwort 206, `currentTime` am Ende.
- [ ] **Datei-Zeile, Öffnen und Laden** (E2E, Desktop): `bericht.pdf` → „PDF“, `bericht.pdf`, „2,3 MiB“, „Öffnen“ (`target="_blank"`, `rel="noopener"`, `href="/anhaenge/<id>"`), „Laden“ (`download="bericht.pdf"`); Klick auf „Öffnen“ → neue Seite mit `Content-Type: application/pdf`; `archiv.zip` → nur „Laden“, Klick → Ereignis `download` mit `suggestedFilename` `archiv.zip`.
- [ ] **Nur Anhänge, mehrere** (E2E, Desktop): Nachricht ohne Text mit Bild, PDF und Video → keine leere Textzeile, drei Anhänge in Sendefolge untereinander; live bei B genauso wie nach dem Neuladen.
- [ ] **Kein Skript beim Öffnen** (E2E, Desktop; QS1 im Browser): `boese.html` und `boese.svg` mit Skript → Datei-Zeile mit „Öffnen“ (Text); „Öffnen“ zeigt Quelltext, kein Skript läuft, `localStorage` der App unverändert; ein Bild-Polyglot (PNG-Kopf + HTML) als Vorschau läuft kein Skript.
- [ ] **Billig beim Öffnen** (E2E, Desktop; QS3, QS5): Channel mit 100 Bildnachrichten (über die API erzeugt) öffnet ≤ 500 ms (wie R00007), höchstens die Bilder in Sichtnähe werden angefragt; Channel mit 20 Video- und Audionachrichten → 0 Anfragen an `/anhaenge/` beim Öffnen und Scrollen.
- [ ] **Schmal** (E2E, Telefon 320 px): Vorschau, Video (volle Breite, 16 : 9), Audio-Player (höchstens so breit wie die Nachricht) und Datei-Zeile mit 120 Zeichen Namen → `scrollWidth <= innerWidth`; „Öffnen“ und „Laden“ je mindestens 44 × 44 px Zielfläche.
- [ ] **Verlauf folgt nicht, solange ein Player spielt** (E2E, Desktop; nach Betreiber U1, Empfehlung): B spielt `film.webm` ab, unten im Verlauf; A sendet drei Textnachrichten → bei B bleibt der Player an seiner Stelle sichtbar, der Verlauf scrollt nicht, der Hinweis auf neue Nachrichten erscheint wie sonst beim Lesen weiter oben; nach dem Ende der Wiedergabe gilt das gewohnte Folgen.
- [ ] **Kein Lösch-Knopf** (E2E, Desktop): eine Nachricht mit Anhängen enthält kein Element mit einem Namen, der „lösch“ enthält.
- [ ] **Tastatur und Bildschirmleser** (E2E, Desktop): mit `Tab` sind Vorschau (als Schalter „<anzeigename> groß anzeigen“), Player-Bedienung, „Öffnen“ und „Laden“ erreichbar; „Öffnen“ und „Laden“ haben zugängliche Namen mit dem Dateinamen („bericht.pdf öffnen“, „bericht.pdf laden“).
- [ ] **App-CSP** (Unit, Konstante): der Wert enthält `media-src 'self'` und weder `data:` noch `blob:` für Medien.
- [ ] **Direkter Medienlink** (vom Betreiber bestätigt, F2: hingenommen): kein Kriterium hier — der Player im Verlauf spielt nie von selbst; der direkte Aufruf ist R00029 F0100.
- [ ] Konsole bei A und B nach der Regel unter „Nicht-funktionale Anforderungen“.

### F0118 Gelöschte Anhänge im Verlauf

- [ ] **Darstellung** (Node, rein): Anhang `{art: "image/png", breite: 1280, hoehe: 800, zustand: "geloescht"}` → Zeile mit „Gelöscht“, ohne „Öffnen“, ohne „Laden“, ohne Vorschau; ebenso Video, Audio und `application/pdf`.
- [ ] **Live gelöscht** (E2E, Desktop): Gegeben über die API eine Nachricht mit `foto.png`, `film.webm` und `bericht.pdf`; A und B haben den Channel offen, das Bild ist geladen; `gamma` löscht `foto.png` und `film.webm` bestätigt über die API → bei A und B steht weniger als 1 s nach der 200 an Stelle von Vorschau und Player je eine Zeile „foto.png · 300 KiB · Gelöscht“ bzw. für `film.webm`, ohne Links; `bericht.pdf` bleibt mit „Öffnen“ und „Laden“; die Höhe der Nachricht ändert sich genau einmal, die Lesestelle bleibt; danach keine Anfrage an `/anhaenge/` für die beiden. Nach dem Neuladen dieselbe Darstellung.
- [ ] **Beim Nachholen** (E2E, Desktop): B verliert die Verbindung (`context.setOffline(true)`), `gamma` löscht `foto.png`, B ist wieder online → nach „Wieder verbunden“ zeigt B die Zeile „Gelöscht“ statt der Vorschau, ohne Neuladen. Mutante „Nachholen verwirft bekannte Nachrichten ganz“ wird rot.
- [ ] **Ladefehler ist nicht gelöscht** (E2E, Desktop): eine Bildanfrage antwortet im Test mit Netzfehler → „Bild nicht verfügbar.“ (F0102), nicht „Gelöscht“.

### F0119 Hochladen übersteht Neuladen und Verbindungsabbruch

- [ ] **Neuladen während des Hochladens** (E2E, Desktop, gedrosselt; UX M4): A tippt `Bericht anbei`, wählt `bericht.pdf` und sendet; während „wird hochgeladen …“ lädt A neu → das Ereignis `dialog` vom Typ `beforeunload` erscheint; nach dem Bestätigen und Neuladen steht `Bericht anbei` im Feld, die Hinweiszeile und die Ansage-Region sagen „Hochladen unterbrochen — bericht.pdf bitte erneut wählen.“; `sessionStorage` enthält den Dateinamen und keinen Dateiinhalt (Größe des Eintrags < 1 KiB). Ohne offenen Eintrag erscheint beim Neuladen kein `dialog`. Mutante „Text nicht gesichert“ wird rot.
- [ ] **Ohne Verbindung** (E2E, Desktop, gedrosselt): während `bericht.pdf` hochlädt, geht A offline (`context.setOffline(true)`) → an der Datei „wartet auf Verbindung“, kein ⚠, kein „Erneut hochladen“; A ist wieder online → nach „Wieder verbunden“ lädt der Browser selbst weiter (neuer `PUT` auf dieselbe ID oder neue Anmeldung nach `abgelaufen`) und sendet die Nachricht, genau einmal bei A und B.

## Betroffene Verzeichnisstruktur

- Oberfläche: `web/eingabe.js` (Schalter, Liste, `sendbar`, `absenden`), neu `web/anhaenge.js` (Anmelden, Hochladen, Abbrechen, Erneut, Verwerfen, Warten, Neuladen, lokaler Eintrag), `web/zustellung.js` und `web/senden.js` (Körper mit `anhaenge`), `web/nachricht.js` (Anhänge zeichnen, „Gelöscht“), `web/live-verlauf.js` und `web/nachholen.js` (Ereignis `anhaenge`, `zustand` beim Nachholen), neu `web/anhang-anzeige.js` (Darstellung wählen, Größe in Worten, Dialog), `web/lc-seite.css`.
- Server: `src/http/grenzschutz.rs` (App-CSP `media-src 'self'`).
- Tests: `e2e/einheit/` (Bereit-Regel, Darstellung, Größe), `e2e/tests/r00030-*.spec.js` (Geräte-Kontexte).
- Designsystem: `Dokumentation/Wireframes/designsystem/components/Anhang/README.md`, `components/Eingabe/README.md`.

## Technische Überlegungen

### Ablauf

1. **Wählen**: Schalter \| `Strg Shift U` → Dateiauswahl → Liste je Channel → Zeilen über dem Feld → `sendbar`.
2. **Absenden**: `absenden()` → lokaler Eintrag „wird hochgeladen …“ → alle anmelden (Abweisung an der Datei \| warten nach `Retry-After`) → je Datei unter Web Lock `PUT` mit Fortschritt (offline: warten) → alle fertig → `senden(text, ids)` in die Sendeschlange; `beforeunload` und `sessionStorage` (Namen), solange der Eintrag offen ist.
3. **Zeigen**: `nachricht.anhaenge` → Darstellung wählen → Vorschau \| Player \| Zeile \| Gelöscht + Datei-Zeile.
4. **Löschen folgen**: Ereignis `anhaenge` → Nachricht über (channel, zeit) finden → Vorschau bzw. Player durch „Gelöscht“ ersetzen; Nachholen übernimmt `zustand`.

### Optionen

- Form des Schalters bei 320 px (die Wahl trifft das Designsystem, B0454; ADR-0038 Ergänzung Karte #41 Frage 4): **A Symbol ⎘ mit zugänglichem Namen „Anhang“ auf Touch-Geräten und unter 480 px, Text „Anhang“ sonst** (Empfehlung: „Senden“ behält seinen Text, das Feld genug Breite); B immer Text (zu eng bei 320 px mit „Senden“); C immer Symbol (am Desktop muss es gedeutet werden).
- Gewählte Dateien beim Channelwechsel: **A je Channel im Speicher des Tabs** (gewählt, wie der Text-Entwurf, ADR-0038); B verwerfen beim Wechsel (Überraschung); C im `sessionStorage` (hält keinen Dateiinhalt, ADR-0046 Frage 5).
- Fehleransage: **A die eine Ansage-Region** (gewählt, ADR-0038 Ergänzung Karte #36, vom `ux-reviewer` bestätigt); B `role="alert"` wie im Designsystem (zweite Live-Region, widerspricht der Ergänzung).
- Neuladen während des Hochladens (UX M4): **A `beforeunload` plus Text und Namen in `sessionStorage`, Hinweis nach dem Neuladen** (gewählt, UX-Vorschlag); B Dateien in IndexedDB halten und weiterladen (Inhalt im Browser, Quote des Browsers, wiederaufnehmbares Hochladen fehlt am Server — ADR-0046 U7); C nur Hinweis ohne Rückfrage (verliert den Text still).
- `error` auf Bild oder Player: **A Fehlerzustand mit Text, Platz bleibt** (gewählt, UX-Sollte); B Zeile „Gelöscht“ wie ADR-0047 Ergänzung „F1′“ Regel 4 (behauptet ein Löschen, das ein Netz- oder Codecfehler nicht belegt) — Offene Frage 5.
- Schnitt: **gelöschte Anhänge (F0118) und Neuladen/Verbindung (F0119) je als eigener Slice** (gewählt; F0101 und F0102 bleiben bei sieben Bubbles, F0118 hängt an R00029 F0117, F0119 ist der Sonderweg nach dem Hauptweg); verworfen: beides in F0101/F0102 (je neun Bubbles).

### Grobentwurf (Vorplanung)

- F0101: B0448 Schalter „Anhang“ und Dateiauswahl (unklar) · B0449 Gewählte Dateien · B0450 Anmelden beim Senden (unklar) · B0451 Hochladen mit Fortschritt (unklar) · B0452 Abbrechen und Erneut · B0453 Nach dem Hochladen senden · B0454 Designsystem Anhang und Eingabe nachgezogen.
- F0102: B0455 Darstellung wählen · B0456 Größe in Worten · B0457 Datei-Zeile · B0458 Bildvorschau · B0459 Bild im Dialog (unklar) · B0460 Player · B0461 Anhänge an der Nachricht.
- F0118: B0517 Gelöschter Anhang als Zeile · B0518 Löschen im Verlauf folgen (unklar).
- F0119: B0519 Neuladen während des Hochladens · B0520 Ohne Verbindung warten (unklar).

Wireframe: kein Artboard `D0003.dc.html` (Hinweis `/wireframe verfeinern D0003`). Gestaltungsquelle: Designsystem `components/Anhang`, `components/Eingabe`; Vorgabe für die Gestaltung, nicht Akzeptanzkriterium.

### Änderungen an bestehenden Klassen

- `web/eingabe.js`: Gruppe `.lc-comp-aktionen` bekommt den Schalter vor „Senden“; `sendbar(text, dateien)`; `absenden` reicht die Dateien an `web/anhaenge.js`.
- `web/zustellung.js`: Sendeauftrag mit `anhaenge`; lokaler Eintrag „wird hochgeladen …“ vor dem Eintritt in die Schlange.
- `web/nachricht.js`: Anhänge unter dem Text, Zeile „Gelöscht“.
- `web/live-verlauf.js`, `web/nachholen.js`: Thema `anhaenge`; beim Nachholen das Feld `zustand` bekannter Nachrichten übernehmen.
- `src/http/grenzschutz.rs`: App-CSP um `media-src 'self'`.

## Tests

Nach Skill `test-pyramide`; je Kriterium die genannte Mutante, das Ergebnis steht im Bericht des Entwicklers.

- **Node (rein)**: Bereit-Regel mit Dateien, Taste mit Dateien, Darstellung wählen (mit `zustand`), Größe in Worten, Fortschrittsrechnung.
- **E2E**: Geräte-Kontexte Telefon, Tablet, Tablet quer, Desktop; Dateiauswahl über `filechooser`; Drosselung über CDP; `context.setOffline`; Ereignis `dialog` für `beforeunload`; Netzprotokoll für Anfragen; verzögerter Fake für Bildanfragen (Springen) und für den `PUT` (Abbrechen, Text geht vor); Löschen über die API von R00029 F0116 als Gegeben.
- **Unit (Rust)**: Konstante der App-CSP.
- **Entwicklertest am Gerät** (UX-Sollte „Am Gerät“): (1) iPhone: „Anhang“ bietet Fotomediathek, Foto aufnehmen und Dateien; ein Foto aus der Mediathek kommt als JPEG mit Vorschau — HEIC ist Befund. (2) Android: Kamera und Dateien; nach einem Kamerafoto keine Neuladung, die Datei steht in der Liste. (3) Ein Hochkant-Foto erscheint bei B unverzerrt und richtig herum. (4) ⎘ wird als Zeichen dargestellt. (5) Video Vollbild und zurück — Lesestelle unverändert. (6) „Laden“ legt die Datei unter ihrem Namen ab. (7) Bildschirm sperren während des Hochladens → läuft weiter, wartet oder zeigt „Erneut hochladen“, nie still weg. (8) VoiceOver/TalkBack lesen „Anhang, Taste“, „<name> entfernen“ und den Fortschritt. (9) Linux/IBus: `Strg Shift U` öffnet die Auswahl, sonst notieren.

## Abhängigkeiten

- Abhängig von: [R00029](R00029-anhaenge-ueber-die-api-ablegen-senden-und-ausliefern.md) — F0101 braucht F0099 Senden mit Anhängen, F0102 braucht F0100 Ausliefern mit Art und Teilabruf, F0118 braucht F0117 Gelöschte Anhänge in offenen Sichten (alle rot); R00027 F0091 Senden-Knopf neben dem Feld (rot, Karte #41; Gruppe `.lc-comp-aktionen`, `absenden`, `sendbar`). Mitgenutzt: R00011 (Eingabe, Zustellung, Verlauf), R00012 (Fehler und Wiederholung).
- Innerhalb: F0101 und F0102 sind parallel baubar (F0102 nimmt das Gegeben über die API); F0119 braucht F0101; F0118 braucht F0102 und R00029 F0117.
- Blockiert: F0104 Dateien ablegen und einfügen (`ausbau`); D0006 Laufwerk (Karte #45, direkt nach #43) nutzt Datei-Zeile, Öffnen und „Gelöscht“.

## Offene Fragen

Beantwortet und eingearbeitet (2026-10-29): Betreiber **F2** (direkter Medienlink hingenommen; hier nichts Prüfbares) und **F3** (Text über 16 MiB nur „Laden“, Kriterium „Darstellung wählen“); Betreiber **F1′** (keine Verdrängung, 507 `voll`/`hochladungen` an der Datei, gelöschte Anhänge → F0118) und **F4** (Orientierung bleibt → Kriterium „Hochformatfoto“). UX-Vorprüfung #43 zu den früheren Fragen 3–6: (3) ⎘ nur unter 480 px, sonst Text „Anhang“, Name immer „Anhang“; (4) eine Ansage-Region: ja; (5) Größe in Worten wie oben, nur dass 1.048.575 nicht „1.024 KiB“, sondern „1,0 MiB“ heißt (Regel ergänzt); das Designsystem stellt „2,4 MB“ auf KiB/MiB um (B0454); (6) Hinweis nach dem Neuladen nötig → F0119.

Offen an den Betreiber (stellt der Director; bis zur Antwort gilt die Empfehlung, die Kriterien sind markiert):

1. **F7 Wer gelöscht hat** (R00029): bis zur Antwort nennt die Zeile „Gelöscht“ niemanden (Kriterium F0118).
2. **U1 Verlauf und Player** (aus der UX-Vorprüfung): folgt der Verlauf neuen Nachrichten nicht, solange ein Video oder Audio spielt? Empfehlung: nicht folgen — wer zusieht, verliert sonst das Bild (Kriterium „Verlauf folgt nicht, solange ein Player spielt“).
3. **U2 Verwerfen während des Hochladens** (aus der UX-Vorprüfung): soll der Eintrag „wird hochgeladen …“ ein „Verwerfen“ tragen, das alles abbricht und den Text zurück ins Feld legt? Empfehlung: ja — sonst bleibt bei einer großen Datei nur Abbrechen Datei für Datei (Kriterium „Verwerfen“).

An den `architekt` (über den Director):

4. **Maße bei EXIF-Orientierung 5–8** (R00029 Offene Frage 4): getauschte `breite`/`hoehe` würden den Platz hochkant reservieren (180 × 240 statt 360 × 270). Bis zur Antwort zeigt die Vorschau das Bild unverzerrt im querformatigen Platz.
5. **`error` ist nicht „Gelöscht“**: R00030 weicht vom letzten Halbsatz der ADR-0047 Ergänzung „F1′“ Regel 4 ab (Optionen oben) — Bestätigung als Ergänzung der ADR erbeten.

Nicht übernommen aus der UX-Vorprüfung: der Wortlaut „frei 200 KiB, nötig 300 KiB“ bzw. „kein Platz mehr“ für „Kein Platz“ — die Oberfläche zeigt den Text der Instanz, und den legt ADR-0046 Ergänzung „F1′“ Regel 2 fest (frei und nötig in MiB nach ADR-0044 Regel 4); eine Angabe in KiB unter 1 MiB wäre eine Ergänzung dieser ADR.

## Manuelle Vorbereitungstätigkeiten

- `ux-reviewer`: Vorprüfung erledigt (Kommentar auf Karte #43, 2026-10-29) und eingearbeitet; eine Wiederholung prüft nur M1–M4 gegen die Kriterien „Zielfläche und Ort“, „Auswahl angesagt“, „Fokus sichtbar“ und „Neuladen während des Hochladens“ sowie den Wortlaut „Gelöscht“ und den Hinweis nach dem Neuladen.
- `security-reviewer`: Matrix-Zeile M1 im Browser („Öffnen“, `<img>`, `<video>`) gegen B0457, B0458, B0460; `sessionStorage` hält nur Namen (F0119).

## Manuelle Nachbereitungstätigkeiten

- Entwicklertest am Gerät (Android, iPhone, Linux/IBus) wie unter „Tests“, Punkte 1–9; Ergebnis im Zettel.
- `/wireframe verfeinern D0003`, falls der Betreiber ein Artboard für Anhänge will.

## Umfang

Gezählt aus der WBS (2026-10-29, nach UX-Vorprüfung #43 und Betreiber F1′/F4 angeglichen, nach den Regeln des Skills `work-breakdown-structure`):

- F0101 Hochladen im Browser: 7 Bubbles · 4 klar (je 0,4 h Untergrenze, Richtwert, ungemessen) · 3 unklar (je 2–4 h) — 7,6–13,6 h.
- F0102 Anhänge im Verlauf zeigen: 7 Bubbles · 6 klar · 1 unklar — 4,4–6,4 h.
- F0118 Gelöschte Anhänge im Verlauf: 2 Bubbles · 1 klar · 1 unklar — 2,4–4,4 h.
- F0119 Hochladen übersteht Neuladen und Verbindungsabbruch: 2 Bubbles · 1 klar · 1 unklar — 2,4–4,4 h.
- F0104 Dateien ablegen und einfügen: `ausbau`, nicht gezählt; Hypothese 2 Bubbles.
- Zusammen 18 Bubbles · 12 klar · 6 unklar — 16,8–28,8 h (vorher 14 Bubbles, 12–20 h). Die UX-Sollte liegen in den bestehenden Bubbles von F0101 und F0102 (Klassen unverändert); ihr Mehrinhalt liegt in den Spannen von B0448, B0450, B0451 — Hypothese, nicht gemessen.

## Warum löst diese Anforderung das Problem? (Pflicht)

Auslöser: Menschen sollen im Browser Dateien teilen und sehen, auf jedem Zielgerät (Vision, Leitplanke Zielgeräte); R00029 liefert dafür nur die API.

- Der Schalter „Anhang“ sitzt in derselben Gruppe wie „Senden“ und öffnet die Dateiauswahl des Systems. Deshalb geht das Anhängen auf dem Telefon mit einem Tippen, ohne Tastenkürzel, und am Desktop zusätzlich mit `Strg Shift U`.
- Alle Dateien werden vor dem ersten Byte angemeldet. Deshalb sieht der Mensch „kein Platz“ an der Datei, statt Minuten zu warten und dann einen Abbruch zu erleben.
- Die Nachricht wartet außerhalb der Sendeschlange auf ihre Dateien. Deshalb hält eine große Datei keinen Text auf, und der Chat bleibt live.
- Vorschau und Player reservieren ihren Platz und laden erst bei Bedarf. Deshalb öffnet ein Channel voller Bilder und Videos so schnell wie ohne, und nichts springt.
- „Öffnen“ und „Laden“ sind einfache Links auf die eine Route mit `sandbox`. Deshalb läuft zwischen Klick und Datei kein eigener Code, und eine präparierte Datei erreicht nichts.
- Ein gelöschter Anhang wird live zur Zeile „Gelöscht“. Deshalb klickt niemand auf einen toten Link, und das Bild verschwindet aus jedem offenen Tab, sobald jemand im Laufwerk Platz schafft.
- Neuladen fragt nach und gibt den Text zurück, ein Verbindungsabbruch wartet. Deshalb verliert niemand still seine Nachricht, wenn das Telefon die Seite neu lädt oder das WLAN kurz weg ist.

## Missing-Docs

- Glossar fehlt (`Dokumentation/Domaenenwissen/` existiert nicht).
- Drosselung des Hochladens über CDP im Chromium des Projekts ist nicht geprobt; Web Locks über Tabs in Playwright nicht geprobt; `beforeunload` im Headless-Chromium (Ereignis `dialog`) und `setOffline` während eines laufenden `XMLHttpRequest` nicht geprobt.

## Notizen

- Verworfene Alternativen: „Zu groß beim Auswählen“ (Designsystem) — es gibt keine feste Grenze (B2), die Antwort gibt die Anmeldung; Hochladen schon beim Auswählen — Reservierungen von Dateien, die nie gesendet werden, hielten die Quote; Nachricht zuerst, Dateien danach (ADR-0046 U4).
- Nicht hier: Ablegen und Einfügen (F0104), Anhänge in Direktgesprächen (I0012), Markdown-Leiste „Code“ (I0017), Löschen in der Oberfläche (Laufwerk, I0030).
- 2026-10-29 angeglichen (planer, `/anforderung review` ohne Dialog, Karte #43): UX-Vorprüfung #43 — Muss M1 (Feldbreite bei 375/320 px statt 160 px bei 320 px), M2 (Auswahl angesagt), M3 (Fokus sichtbar), M4 (Neuladen → F0119); Sollte (Liste mit Tastatur, Tablet quer, Zielflächen, Namen, Fokus nach Aktionen, Taste ohne Text, ohne Verbindung, „wartet“, Abbrechen/Verwerfen, EXIF-Ausrichtung, Fehlerzustände, Audio schmal, Bild-Dialog, Öffnen für große Bilder, Player-Name, Channelwechsel, abgelegte Datei, Wortlaut, Kontrast, am Gerät); Antworten 3–6. Betreiber F1′ (507 `voll` mit frei/braucht, `hochladungen` als Warten, 422 `zu_gross`, gelöschte Anhänge → F0118, kein Lösch-Knopf im Verlauf) und F4 (422 `metadaten`, Hochformatfoto). Nicht übernommen: KiB-Wortlaut für „Kein Platz“ (Offene Fragen).
