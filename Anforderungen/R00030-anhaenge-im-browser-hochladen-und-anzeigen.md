---
id: R00030
status: Neu
datum: 2026-10-29
---

# R00030: Anhänge im Browser hochladen und anzeigen

## Beschreibung

R00029 legt Anhänge über die API ab und liefert sie aus. Im Browser gibt es dafür noch nichts. Danach gilt:

- **Dateien wählen.** In der Eingabe steht vor „Senden“ ein Schalter „Anhang“. Ein Tippen, ein Klick, `Enter`/`Leertaste` auf ihm oder `Strg Shift U` im Feld öffnet die Dateiauswahl des Systems — auf dem Telefon also auch Kamera und Fotos, wie das System sie anbietet. Gewählte Dateien (höchstens 10) stehen als Zeilen über dem Feld mit Name, Größe und „Entfernen“.
- **Senden mit Dateien**, mit „Senden“ oder per Taste wie bisher, auch ohne Text. Die Nachricht erscheint sofort im eigenen Verlauf als „wird hochgeladen“; jede Datei wird angemeldet (ist kein Platz, steht das an der Datei, bevor etwas lädt) und dann mit Balken und Prozent hochgeladen, mit „Abbrechen“ und bei Fehlschlag „Erneut“. Danach wird die Nachricht gesendet wie jede andere. Was in der Zwischenzeit als Text gesendet wird, geht sofort hinaus.
- **Anhänge sehen.** Jede Nachricht zeigt ihre Anhänge unter dem Text: Bilder als Vorschau, groß im Dialog; Video und Audio im Player des Browsers, der erst lädt, wenn jemand abspielt; jeder Anhang mit Datei-Zeile, „Öffnen“ (wo der Browser die Art darstellt) und „Laden“.

Alles mit Touch auf Telefon und Tablet, mit Maus und Tastatur am Desktop und mit Bildschirmleser.

Zahlt ein auf: [Vision](R00000-vision.md) — „Anhänge“, „Live überall“ und die Leitplanke „Zielgeräte“ (Desktop, Tablet, Telefon; jede Aktion auch ohne Tastenkürzel). Kein Widerspruch zur Vision.

Herkunft: WBS I0010 „Anhänge senden und öffnen“ (Karte #9); [ADR-0046](../Dokumentation/ADRs/0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) Regel 13 und Frage 7, [ADR-0047](../Dokumentation/ADRs/0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md) Regeln 5–7, [ADR-0038](../Dokumentation/ADRs/0038-eingabe-mehrzeilig-enter-sendet-entwurf-je-channel-gebuendelte-ansage.md) Ergänzung Karte #41 (Gruppe `.lc-comp-aktionen`: „Anhang“ vor „Senden“; bei 320 px eher Symbolschalter mit zugänglichem Namen), Betreiber B1, B3, B5.

Slices (WBS `Dokumentation/Planung/localchata.md`, unter I0010): F0101 Hochladen im Browser (Ausbaustufe „Hochladen 1“), F0102 Anhänge im Verlauf zeigen. Nicht beauftragt: F0104 Dateien ablegen und einfügen (`ausbau`, „Hochladen 2“).

### Domänenbezug

Kein Glossar im Projekt. Begriffe wie in R00011, R00027 und [R00029](R00029-anhaenge-ueber-die-api-ablegen-senden-und-ausliefern.md) (Anhang, Anmeldung, Platz für Anhänge, Art; Feld, Knopf „Senden“, vorläufig, fehlgeschlagen, Warteschlange, Ansage-Region, Touch-Gerät, Bildschirmtastatur). Neu: **Schalter „Anhang“**; **Datei-Zeile** (Typkürzel, Name, Größe, „Öffnen“, „Laden“); **Vorschau** (verkleinertes Bild im Verlauf); **Player** (Wiedergabe des Browsers).

## Geschäftlicher Nutzen

Die Betreiberin schickt vom Telefon ein Foto vom Whiteboard oder einen Bildschirmausschnitt an die Agenten, ohne an den Rechner zu gehen; am Desktop zieht ein Entwickler ein Log heran. Wer mitliest, sieht Bilder gleich im Verlauf und spielt eine Bildschirmaufnahme ab, ohne sie herunterzuladen. Weil der Chat während des Hochladens weiterläuft, hält eine große Datei niemanden auf.

## Funktionale Anforderungen

### Hochladen im Browser (F0101)

- **Schalter „Anhang“**: `<button type="button">` in der Gruppe `.lc-comp-aktionen` rechts vom Feld, in DOM- und Tab-Reihenfolge vor „Senden“: Feld → **Anhang** → Senden → Teilnehmerliste (ADR-0038 Ergänzung Karte #41, K7). Zugänglicher Name genau „Anhang“. Sichtbar: Text „Anhang“ oder Symbol ⎘ mit zugänglichem Namen — die Form legt das Designsystem fest (B0454); bei 320 px kein Querscrollen, das Feld behält mindestens 160 px Breite. Zielfläche auf Touch-Geräten (`(pointer: coarse)`) mindestens 44 × 44 px, sonst mindestens 24 × 24 px. Erscheint mit dem Feld, sobald ein Channel gewählt ist.
- **Dateiauswahl**: Tippen, Klick, `Enter` oder `Leertaste` auf dem Schalter oder `Strg Shift U` im Feld öffnen die Dateiauswahl des Systems mit Mehrfachauswahl (`<input type="file" multiple>`, ohne `accept` — jede Datei, B1). Der Fokus kehrt danach ins Feld zurück.
- **Gewählte Dateien** stehen als Zeilen über dem Feld (Typkürzel aus der Endung, nur zur Anzeige; Name mit Ellipse; Größe in Worten; „Entfernen“ mit zugänglichem Namen „<name> entfernen“). Höchstens 10 je Nachricht; wer mehr wählt, behält die ersten bis zur Zehn, und die Hinweiszeile sagt „Höchstens 10 Dateien je Nachricht — <k> nicht übernommen.“ Die Liste gehört zum Channel und bleibt im Speicher des Tabs beim Channelwechsel erhalten; beim Neuladen geht sie verloren (der Entwurf hält keinen Dateiinhalt).
- **Bereit**: „Senden“ und die Taste senden, wenn der Text sendbar ist **oder** mindestens eine Datei gewählt ist und der Text die Textregel nicht verletzt (Leerraum allein ist mit Datei erlaubt, B5). `sendbar` aus ADR-0038 Ergänzung Karte #41 K2 wächst um die Dateien; ein Absendeweg (K1).
- **Senden mit Dateien**: Feld und Liste werden leer, am Ende des eigenen Verlaufs steht ein lokaler Eintrag „wird hochgeladen“ mit Text und je Datei einer Zeile. Zuerst werden **alle** Dateien angemeldet (`POST /api/anhaenge`). Eine Abweisung erscheint an ihrer Datei mit ⚠ und dem Text der Instanz (507 `quote`: „Nicht hochgeladen: der Platz für Anhänge ist voll — noch <n> frei; Text kann weiter gesendet werden.“; 422 `name`; 429 → nach `Retry-After` selbst erneut) und „Entfernen“; die übrigen Dateien laden weiter, nichts wird gesendet, solange eine Datei abgewiesen und nicht entfernt ist.
- **Hochladen**: je Datei ein `PUT` über `XMLHttpRequest` mit Sitzungs-Header; Balken 4 px in `kd-accent` mit Prozent; höchstens 2 Hochladungen zugleich je Browser über alle Tabs (Web Locks), weitere warten mit „wartet“. „Abbrechen“ an einer Datei bricht ab, gibt die Anmeldung frei (`DELETE`) und nimmt die Datei aus dem Eintrag; ohne Text und ohne weitere Datei verschwindet der Eintrag. Fehlschlag (Netz, 408, 5xx) → ⚠ mit Grund und „Erneut“; „Erneut“ lädt dieselbe Anmeldung neu, nach `abgelaufen` meldet der Browser neu an.
- **Danach senden** (N1): sind alle Dateien oben, tritt die Nachricht mit `{text, anhaenge}` in die Sendeschlange des Tabs (ADR-0037) und wird wie jede Nachricht vorläufig, bestätigt oder fehlgeschlagen gezeigt. Text, der während des Hochladens gesendet wird, wartet nicht und erscheint vorher; die Nachricht mit Anhängen steht danach an der Stelle ihrer Serverzeit.
- **Ansage**: Fehler an Dateien und das Ende des Hochladens werden über die eine Ansage-Region gemeldet (ADR-0038 Regel 10, Ergänzung Karte #36), nicht über ein zusätzliches `role="alert"` — Abweichung vom Designsystem, B0454 zieht es nach. Fortschritt wird nicht angesagt; der Balken ist ein `progressbar` mit Wert.

### Anhänge im Verlauf zeigen (F0102)

- Jede Nachricht — im Verlauf, live aus dem Strom, die eigene nach der Bestätigung — zeigt ihre Anhänge in Sendefolge unter dem Text, gestapelt mit `kd-space-4`; ohne Text nur die Anhänge.
- **Bilder** (Art `image/*` mit Maßen, höchstens 10 MiB, höchstens 50 Megapixel, jede Seite höchstens 16.384 px): Vorschau bis 360 × 240 px im Seitenverhältnis als `<img loading="lazy" decoding="async" width height alt="<anzeigename>">`; der Platz steht, bevor ein Byte lädt (bis dahin `kd-surface-2` mit Name und Maßen). Tippen, Klick oder `Enter` auf die Vorschau öffnet das Bild groß in einem modalen Dialog (in den Bildschirm eingepasst, Name, „Schließen“); `Esc`, „Schließen“ oder Tippen daneben schließt ihn, der Fokus kehrt auf die Vorschau zurück. Größere Bilder erscheinen nur als Datei-Zeile.
- **Video** (`video/*`): `<video controls preload="none" playsinline>` in einem Rahmen 360 × 202 px (16 : 9, `object-fit: contain`); schmaler als 360 px: volle Breite im selben Verhältnis. **Audio** (`audio/*`): `<audio controls preload="none">` in einer Zeile fester Höhe, 360 px breit bzw. volle Breite. Nie `autoplay`; vor „Abspielen“ keine Anfrage; Spulen über Teilabruf (R00029 F0100).
- **Datei-Zeile** unter jedem Anhang (auch unter Vorschau und Player): Typkürzel, Name mit Ellipse, Größe in Worten, „Öffnen“ und „Laden“; höchstens 420 px breit. „Öffnen“ = `<a target="_blank" rel="noopener">` nur für Bilder, PDF und Text bis 16 MiB; Video und Audio öffnen im Player (M1), alles andere nur „Laden“. „Laden“ = `<a download="<anzeigename>">` für alle. Name und Größe nur über `textContent`.
- **Größe in Worten**: unter 1.024 Bytes „<n> Byte“; unter 1 MiB „<n> KiB“ (ganz, kaufmännisch gerundet, mindestens 1); ab 1 MiB wie ADR-0044 Regel 4 (MiB bzw. ab 1 GiB GiB, eine Nachkommastelle, deutsches Komma). Rechenbeispiel: 812 → „812 Byte“; 307.200 → „300 KiB“; 2.457.600 → „2,3 MiB“; 1.610.612.736 → „1,5 GiB“.
- **App-CSP** wächst um `media-src 'self'` (ADR-0014 Ergänzung Karte #9) — ohne sie spielt kein Player.
- **Nichts springt**: Vorschau und Player haben ihre Höhe, bevor geladen wird; der Verlauf verschiebt sich durch das Laden um höchstens 1 px (L4).

## Nicht-funktionale Anforderungen

- **Geräte** wie R00027: geprüft als emuliertes Gerät (Playwright-Kontext mit `hasTouch`, `isMobile`, `deviceScaleFactor`): Telefon 375 × 812 (Faktor 3), Tablet 768 × 1024 (Faktor 2), Desktop 1280 × 800 ohne Touch. Telefon und Tablet werden nur durch Tippen bedient (`tap()`), die Dateiauswahl über das Ereignis `filechooser`.
- **Schmal**: bei 320 px Fensterbreite (Telefon-Kontext) kein Querscrollen mit Feld, beiden Schaltern, drei gewählten Dateien, einem Eintrag „wird hochgeladen“, einer Vorschau, einem Video und einer Datei-Zeile mit langem Namen.
- **Leistung** (Zweck: „Channel öffnet sofort“, Lastprofil Q1b ≤ 500 ms; QS3, QS5): ein Channel mit 100 Bildnachrichten öffnet in derselben Zeit wie ohne Bilder (≤ 500 ms bis zur gezeichneten letzten Nachricht, gemessen wie R00007); ein Channel mit 20 Video- und Audionachrichten stellt beim Öffnen keine einzige Anfrage an `/anhaenge/`.
- **Text während des Hochladens** (Zweck: B2 „non-blocking“, Zustellung < 1 s): während eine Datei hochlädt, steht ein per `Enter` gesendeter Text spätestens 1 s nach dem Tastendruck bestätigt im eigenen Verlauf und im Verlauf von B (erwartet unter 100 ms im LAN; obere Grenze aus dem Lastprofil, Uhr ab dem Tastendruck).
- **Konsole**: keine unbehandelte Ausnahme, kein CSP-Verstoß, keine Konsolenzeile außer „Failed to load resource“ zu den Antworten, die ein Kriterium vorschreibt: 507 („Kein Platz“), 404 („Abgelaufen“), der im Test abgebrochene `PUT` („Abbrechen“, „Fehlschlag“).

## Akzeptanzkriterien

Gemeinsam für alle Kriterien, wenn nicht anders genannt: Start, Beitritte per `curl`, `S(<name>)` und Browser A (`shoff`) und B (`anna`) wie in R00011; Quote wie R00029 (`--plattenquote 10MiB`, Platz für Anhänge 9 MiB). **Telefon**, **Tablet**, **Desktop** wie R00027. „Der Schalter“ ist `getByRole('button', { name: 'Anhang', exact: true })` in der Eingabe, „der Knopf“ der Knopf „Senden“ aus R00027. „Wählen“ heißt: Schalter auslösen, das Ereignis `filechooser` abwarten, `setFiles(...)`. Testdateien entstehen im Test: `foto.png` (PNG 1280 × 800, ≈ 300 KiB), `bericht.pdf` (2.457.600 Bytes, beginnt mit `%PDF-1.7`), `log.txt` (Text, 40 KiB), `film.webm` und `ton.ogg` (6 s, mit `ffmpeg` im Setup), `archiv.zip` (Zufallsbytes, 1 MiB). „Gegeben über die API“ heißt: `beta` lädt per `curl` hoch und sendet (R00029).

### F0101 Hochladen im Browser

- [ ] **Bereit-Regel** (Node, rein): `sendbar("", 1 Datei)` → wahr; `sendbar("   ", 1 Datei)` → wahr (Text wird `""`); `sendbar("", 0)` → falsch; `sendbar(16.385 × "a", 1 Datei)` → falsch (Textregel); `sendbar("a", 0)` → wahr wie R00027. Mutante „Dateien ignoriert“ wird rot.
- [ ] **Wählen und senden auf dem Telefon** (E2E, Telefon): A wählt `# allgemein`, tippt auf den Schalter, wählt `foto.png`, tippt auf den Knopf → über dem Feld stand bis dahin eine Zeile `foto.png` mit „300 KiB“ und „Entfernen“; danach genau ein `POST /api/anhaenge`, ein `PUT`, ein `POST …/nachrichten` mit `{"text":"","anhaenge":["<id>"]}`; im Verlauf von A und B steht die Nachricht von `shoff@127.0.0.1` mit dem Bild (F0102), genau einmal; Feld und Liste sind leer, der Fokus liegt im Feld.
- [ ] **Tablet** (E2E, Tablet): wie Telefon mit `bericht.pdf` und dem Text `Bericht Q3` → die Nachricht trägt Text und Datei-Zeile.
- [ ] **Desktop mit Maus und Tastatur** (E2E, Desktop): Klick auf den Schalter → `filechooser`; `Strg Shift U` im Feld → `filechooser`; `Tab` vom Feld → Schalter, `Enter` → `filechooser`, `Leertaste` → `filechooser`; ein weiteres `Tab` → Knopf „Senden“, danach Teilnehmerliste. Gewählt `log.txt` und `archiv.zip`, Text `zwei Dateien`, `Enter` im Feld → gesendet mit beiden Anhängen in Wahlreihenfolge.
- [ ] **Zehn Dateien** (E2E, Desktop): 12 Dateien wählen → 10 Zeilen, Hinweis „Höchstens 10 Dateien je Nachricht — 2 nicht übernommen.“; „Entfernen“ an der dritten → 9 Zeilen, Fokus auf dem nächsten „Entfernen“ bzw. im Feld, wenn keine Zeile mehr da ist.
- [ ] **Channelwechsel** (E2E, Desktop): `foto.png` in `# allgemein` gewählt, Wechsel nach `# technik` → keine Zeile; zurück nach `# allgemein` → die Zeile ist wieder da.
- [ ] **Kein Platz** (E2E, Desktop): Gegeben über die API, dass `beta` 9.437.184 Bytes angemeldet hält → A wählt `foto.png` und sendet → an der Datei ⚠ „Nicht hochgeladen: der Platz für Anhänge ist voll — noch 0,0 MiB frei; Text kann weiter gesendet werden.“ und „Entfernen“; kein `PUT`, kein `POST …/nachrichten`; die Ansage-Region meldet den Fehler einmal. A entfernt die Datei → der Eintrag verschwindet (ohne Text) bzw. wird als Text gesendet (mit Text `trotzdem`).
- [ ] **Fortschritt** (E2E, Desktop, Hochladen über CDP `Network.emulateNetworkConditions` auf 1 MiB/s gedrosselt): `bericht.pdf` → der Balken (`role="progressbar"`) steigt in mindestens zwei Schritten zwischen 0 und 100 und zeigt danach keinen Balken mehr; der Eintrag hieß bis dahin „wird hochgeladen“. Greift die Drosselung beim Hochladen im Chromium des Projekts nicht, gilt das Kriterium als Node-Test der Fortschrittsrechnung, und der Bericht nennt das.
- [ ] **Höchstens zwei zugleich** (E2E, Desktop): zwei Tabs desselben Browsers, je 3 Dateien gedrosselt → laut Netzprotokoll laufen nie mehr als 2 `PUT` gleichzeitig; alle 6 kommen an.
- [ ] **Abbrechen** (E2E, Telefon, gedrosselt): A sendet `foto.png` und `bericht.pdf`, tippt während des Hochladens auf „Abbrechen“ an `bericht.pdf` → `DELETE` für dessen ID, `/api/gesundheit` zeigt danach `reserviert` 0 für A; die Nachricht wird mit nur `foto.png` gesendet.
- [ ] **Fehlschlag und Erneut** (E2E, Desktop): der erste `PUT` wird im Test abgebrochen (`route.abort()`) → ⚠ mit Grund „Keine Antwort vom Server.“ und „Erneut“ an der Datei; „Erneut“ → ein neuer `PUT` auf dieselbe ID, danach wird gesendet. Ist die Anmeldung inzwischen abgelaufen (Antwort 404 `abgelaufen`, im Test vorgegeben), meldet der Browser neu an und lädt hoch.
- [ ] **Text geht vor** (E2E, Desktop, gedrosselt; N1): A sendet `film.webm` (vergrößert auf 8 MiB) mit Text `Video`, tippt während des Hochladens `zwischendurch` und drückt `Enter` → `zwischendurch` steht spätestens 1 s nach dem Tastendruck bestätigt bei A und B; danach erscheint `Video` mit dem Player, in beiden Verläufen nach `zwischendurch`.
- [ ] **Bildschirmleser** (E2E, Desktop): genau ein Element mit Rolle `button` und Namen „Anhang“ in der Eingabe; jede gewählte Datei hat einen Schalter „<name> entfernen“; der Balken ist `progressbar` mit `aria-valuenow`; ein Fehler steht genau einmal in der einen Ansage-Region; die Seite hat weiter genau eine Live-Region.
- [ ] **Zielfläche und Ort** (E2E): der Schalter misst auf Telefon und Tablet mindestens 44 × 44 px, auf Desktop mindestens 24 × 24 px; er liegt links von „Senden“ und rechts vom Feld; bei 320 px (Telefon-Kontext) gilt `scrollWidth <= innerWidth` mit drei gewählten Dateien und einem Eintrag „wird hochgeladen“, das Feld ist mindestens 160 px breit.
- [ ] **Bestand bleibt grün**: die Kriterien aus R00011 „Tasten“, „Senden“, „Reihenfolge“ und aus R00027 laufen unverändert — außer R00027 „Gruppe“, das mit R00030 so gilt: „Senden“ ist das letzte Kind von `.lc-comp-aktionen`, „Anhang“ das einzige weitere, `columnGap` mindestens 8 px.
- [ ] **Designsystem** (Review-Punkt `ux-reviewer`, `rechtschreib-reviewer`): `components/Anhang/README.md` und `components/Eingabe/README.md` beschreiben Schalter, „wird hochgeladen“, Ansage über die eine Region; „Zu groß: sofort beim Auswählen geprüft“ ist ersetzt durch die Abweisung bei der Anmeldung; Ablegen und Einfügen stehen als Ausbaustufe.
- [ ] Konsole bei A nach der Regel unter „Nicht-funktionale Anforderungen“.

### F0102 Anhänge im Verlauf zeigen

- [ ] **Darstellung wählen** (Node, rein): `image/png` 1280 × 800, 300 KiB → Vorschau 360 × 225; `image/png` 800 × 1200 → Vorschau 160 × 240; `image/jpeg` 20.000 × 20.000 → Zeile; `image/png` 10 MiB + 1 Byte → Zeile; `video/webm` → Video; `audio/ogg`, `audio/mpeg` → Audio; `application/pdf` und `text/plain` bis 16 MiB → Zeile mit „Öffnen“; `text/plain` über 16 MiB (`attachment`) und `application/octet-stream` → Zeile ohne „Öffnen“ (nach Betreiber F3).
- [ ] **Größe in Worten** (Node, rein): 0 → „0 Byte“, 812 → „812 Byte“, 1.023 → „1.023 Byte“, 1.024 → „1 KiB“, 307.200 → „300 KiB“, 1.048.575 → „1.024 KiB“, 1.048.576 → „1,0 MiB“, 2.457.600 → „2,3 MiB“, 1.610.612.736 → „1,5 GiB“.
- [ ] **Bild** (E2E, Desktop; Gegeben über die API): Nachricht mit `foto.png` → `<img>` mit `width="360" height="225"`, `loading="lazy"`, `alt="foto.png"`; die Unterkante der Nachricht verschiebt sich zwischen „vor dem Laden“ und „Bild geladen“ um höchstens 1 px (Antwort der Bildanfrage im Test um 1 s verzögert). Klick → Dialog mit dem Bild und „Schließen“, `Esc` → zu, Fokus auf der Vorschau.
- [ ] **Bild auf dem Telefon** (E2E, Telefon): Tippen auf die Vorschau → Dialog, das Bild passt in 375 × 812 ohne Querscrollen; Tippen auf „Schließen“ → zu.
- [ ] **Video und Audio** (E2E, Desktop): Nachrichten mit `film.webm` und `ton.ogg` → `<video controls preload="none" playsinline>` im Rahmen 360 × 202 px bzw. `<audio controls preload="none">`, ohne `autoplay`; bis zum Abspielen keine Anfrage an `/anhaenge/`; `play()` → die Wiedergabe läuft; Sprung ans Ende (`currentTime = duration − 0,1`) → eine Anfrage mit `Range`, Antwort 206, `currentTime` am Ende.
- [ ] **Datei-Zeile, Öffnen und Laden** (E2E, Desktop): `bericht.pdf` → „PDF“, `bericht.pdf`, „2,3 MiB“, „Öffnen“ (`target="_blank"`, `rel="noopener"`, `href="/anhaenge/<id>"`), „Laden“ (`download="bericht.pdf"`); Klick auf „Öffnen“ → neue Seite mit `Content-Type: application/pdf`; `archiv.zip` → nur „Laden“, Klick → Ereignis `download` mit `suggestedFilename` `archiv.zip`.
- [ ] **Nur Anhänge, mehrere** (E2E, Desktop): Nachricht ohne Text mit Bild, PDF und Video → keine leere Textzeile, drei Anhänge in Sendefolge untereinander; live bei B genauso wie nach dem Neuladen.
- [ ] **Kein Skript beim Öffnen** (E2E, Desktop; QS1 im Browser): `boese.html` und `boese.svg` mit Skript → Datei-Zeile mit „Öffnen“ (Text); „Öffnen“ zeigt Quelltext, kein Skript läuft, `localStorage` der App unverändert; ein Bild-Polyglot (PNG-Kopf + HTML) als Vorschau läuft kein Skript.
- [ ] **Billig beim Öffnen** (E2E, Desktop; QS3, QS5): Channel mit 100 Bildnachrichten (über die API erzeugt) öffnet ≤ 500 ms (wie R00007), höchstens die Bilder in Sichtnähe werden angefragt; Channel mit 20 Video- und Audionachrichten → 0 Anfragen an `/anhaenge/` beim Öffnen und Scrollen.
- [ ] **Schmal** (E2E, Telefon 320 px): Vorschau, Video (volle Breite, 16 : 9) und Datei-Zeile mit 120 Zeichen Namen → `scrollWidth <= innerWidth`; „Öffnen“ und „Laden“ je mindestens 44 × 44 px Zielfläche.
- [ ] **Tastatur und Bildschirmleser** (E2E, Desktop): mit `Tab` sind Vorschau (als Schalter „<anzeigename> groß anzeigen“), Player-Bedienung, „Öffnen“ und „Laden“ erreichbar; „Öffnen“ und „Laden“ haben zugängliche Namen mit dem Dateinamen („bericht.pdf öffnen“, „bericht.pdf laden“).
- [ ] **App-CSP** (Unit, Konstante): der Wert enthält `media-src 'self'` und weder `data:` noch `blob:` für Medien.
- [ ] **Direkter Medienlink** (nach Betreiber F2): kein Kriterium hier — der Player im Verlauf spielt nie von selbst; der direkte Aufruf ist R00029 F0100.
- [ ] Konsole bei A und B nach der Regel unter „Nicht-funktionale Anforderungen“.

## Betroffene Verzeichnisstruktur

- Oberfläche: `web/eingabe.js` (Schalter, Liste, `sendbar`, `absenden`), neu `web/anhaenge.js` (Anmelden, Hochladen, Abbrechen, Erneut, lokaler Eintrag), `web/zustellung.js` und `web/senden.js` (Körper mit `anhaenge`), `web/nachricht.js` (Anhänge zeichnen), neu `web/anhang-anzeige.js` (Darstellung wählen, Größe in Worten, Dialog), `web/lc-seite.css`.
- Server: `src/http/grenzschutz.rs` (App-CSP `media-src 'self'`).
- Tests: `e2e/einheit/` (Bereit-Regel, Darstellung, Größe), `e2e/tests/r00030-*.spec.js` (Geräte-Kontexte).
- Designsystem: `Dokumentation/Wireframes/designsystem/components/Anhang/README.md`, `components/Eingabe/README.md`.

## Technische Überlegungen

### Ablauf

1. **Wählen**: Schalter \| `Strg Shift U` → Dateiauswahl → Liste je Channel → Zeilen über dem Feld → `sendbar`.
2. **Absenden**: `absenden()` → lokaler Eintrag „wird hochgeladen“ → alle anmelden → je Datei unter Web Lock `PUT` mit Fortschritt → alle fertig → `senden(text, ids)` in die Sendeschlange.
3. **Zeigen**: `nachricht.anhaenge` → Darstellung wählen → Vorschau \| Player \| Zeile + Datei-Zeile.

### Optionen

- Form des Schalters bei 320 px (die Wahl trifft das Designsystem, B0454; ADR-0038 Ergänzung Karte #41 Frage 4): **A Symbol ⎘ mit zugänglichem Namen „Anhang“ auf Touch-Geräten und unter 480 px, Text „Anhang“ sonst** (Empfehlung: „Senden“ behält seinen Text, das Feld genug Breite); B immer Text (zu eng bei 320 px mit „Senden“); C immer Symbol (am Desktop muss es gedeutet werden).
- Gewählte Dateien beim Channelwechsel: **A je Channel im Speicher des Tabs** (gewählt, wie der Text-Entwurf, ADR-0038); B verwerfen beim Wechsel (Überraschung); C im `sessionStorage` (hält keinen Dateiinhalt, ADR-0046 Frage 5).
- Fehleransage: **A die eine Ansage-Region** (gewählt, ADR-0038 Ergänzung Karte #36); B `role="alert"` wie im Designsystem (zweite Live-Region, widerspricht der Ergänzung).

### Grobentwurf (Vorplanung)

- F0101: B0448 Schalter „Anhang“ und Dateiauswahl (unklar) · B0449 Gewählte Dateien · B0450 Anmelden beim Senden (unklar) · B0451 Hochladen mit Fortschritt (unklar) · B0452 Abbrechen und Erneut · B0453 Nach dem Hochladen senden · B0454 Designsystem Anhang und Eingabe nachgezogen.
- F0102: B0455 Darstellung wählen · B0456 Größe in Worten · B0457 Datei-Zeile · B0458 Bildvorschau · B0459 Bild im Dialog (unklar) · B0460 Player · B0461 Anhänge an der Nachricht.

Wireframe: kein Artboard `D0003.dc.html` (Hinweis `/wireframe verfeinern D0003`). Gestaltungsquelle: Designsystem `components/Anhang`, `components/Eingabe`; Vorgabe für die Gestaltung, nicht Akzeptanzkriterium.

### Änderungen an bestehenden Klassen

- `web/eingabe.js`: Gruppe `.lc-comp-aktionen` bekommt den Schalter vor „Senden“; `sendbar(text, dateien)`; `absenden` reicht die Dateien an `web/anhaenge.js`.
- `web/zustellung.js`: Sendeauftrag mit `anhaenge`; lokaler Eintrag „wird hochgeladen“ vor dem Eintritt in die Schlange.
- `web/nachricht.js`: Anhänge unter dem Text.
- `src/http/grenzschutz.rs`: App-CSP um `media-src 'self'`.

## Tests

Nach Skill `test-pyramide`; je Kriterium die genannte Mutante, das Ergebnis steht im Bericht des Entwicklers.

- **Node (rein)**: Bereit-Regel mit Dateien, Darstellung wählen, Größe in Worten, Fortschrittsrechnung.
- **E2E**: Geräte-Kontexte Telefon, Tablet, Desktop; Dateiauswahl über `filechooser`; Drosselung über CDP; Netzprotokoll für Anfragen; verzögerter Fake für Bildanfragen (Springen) und für den `PUT` (Abbrechen, Text geht vor).
- **Unit (Rust)**: Konstante der App-CSP.
- **Entwicklertest am Gerät**: Android und iPhone — Dateiauswahl mit Kamera und Fotos, Hochladen eines Fotos, Bild im Dialog, Video abspielen.

## Abhängigkeiten

- Abhängig von: [R00029](R00029-anhaenge-ueber-die-api-ablegen-senden-und-ausliefern.md) — F0101 braucht F0099 Senden mit Anhängen, F0102 braucht F0100 Ausliefern mit Art und Teilabruf (beide rot); R00027 F0091 Senden-Knopf neben dem Feld (rot, Karte #41; Gruppe `.lc-comp-aktionen`, `absenden`, `sendbar`). Mitgenutzt: R00011 (Eingabe, Zustellung, Verlauf), R00012 (Fehler und Wiederholung).
- Innerhalb: F0101 und F0102 sind parallel baubar (F0102 nimmt das Gegeben über die API).
- Blockiert: F0104 Dateien ablegen und einfügen (`ausbau`).

## Offene Fragen

An den Betreiber (stellt der Director; bis zur Antwort gilt der Standard der ADR):

1. **F2 Direkter Medienlink** — betrifft hier nichts Prüfbares; „Öffnen“ gibt es für Video und Audio nicht (M1).
2. **F3 Großer Text** — Text über 16 MiB bekommt nur „Laden“ (Kriterium „Darstellung wählen“).

An die UX-Vorprüfung (`ux-reviewer`), Empfehlungen des planer in den Kriterien:

3. Form des Schalters bei 320 px und am Desktop (Optionen oben; Empfehlung A).
4. Fehleransage über die eine Region statt `role="alert"` (Abweichung vom Designsystem).
5. Größe in Worten (Regel und Rechenbeispiel oben).
6. Liste der Dateien geht beim Neuladen verloren — Hinweis nötig? Empfehlung: nein, wie der Text-Entwurf, der beim Neuladen nur den Text hält.

## Manuelle Vorbereitungstätigkeiten

- `ux-reviewer`: Vorprüfung dieser Anforderung (Detailplanung, Betreiber 2026-10-29); der `planer` arbeitet die Kriterien ein.
- `security-reviewer`: Matrix-Zeile M1 im Browser („Öffnen“, `<img>`, `<video>`) gegen B0457, B0458, B0460.

## Manuelle Nachbereitungstätigkeiten

- Entwicklertest am Gerät (Android, iPhone) wie unter „Tests“; Ergebnis im Zettel.
- `/wireframe verfeinern D0003`, falls der Betreiber ein Artboard für Anhänge will.

## Umfang

Gezählt aus der WBS (2026-10-29, nach den Regeln des Skills `work-breakdown-structure`):

- F0101 Hochladen im Browser: 7 Bubbles · 4 klar (je 0,4 h Untergrenze, Richtwert, ungemessen) · 3 unklar (je 2–4 h) — zusammen 7,6–13,6 h.
- F0102 Anhänge im Verlauf zeigen: 7 Bubbles · 6 klar · 1 unklar — 4,4–6,4 h.
- F0104 Dateien ablegen und einfügen: `ausbau`, nicht gezählt; Hypothese 2 Bubbles.
- Zusammen 14 Bubbles · 10 klar · 4 unklar — 12–20 h.

## Warum löst diese Anforderung das Problem? (Pflicht)

Auslöser: Menschen sollen im Browser Dateien teilen und sehen, auf jedem Zielgerät (Vision, Leitplanke Zielgeräte); R00029 liefert dafür nur die API.

- Der Schalter „Anhang“ sitzt in derselben Gruppe wie „Senden“ und öffnet die Dateiauswahl des Systems. Deshalb geht das Anhängen auf dem Telefon mit einem Tippen, ohne Tastenkürzel, und am Desktop zusätzlich mit `Strg Shift U`.
- Alle Dateien werden vor dem ersten Byte angemeldet. Deshalb sieht der Mensch „kein Platz“ an der Datei, statt Minuten zu warten und dann einen Abbruch zu erleben.
- Die Nachricht wartet außerhalb der Sendeschlange auf ihre Dateien. Deshalb hält eine große Datei keinen Text auf, und der Chat bleibt live.
- Vorschau und Player reservieren ihren Platz und laden erst bei Bedarf. Deshalb öffnet ein Channel voller Bilder und Videos so schnell wie ohne, und nichts springt.
- „Öffnen“ und „Laden“ sind einfache Links auf die eine Route mit `sandbox`. Deshalb läuft zwischen Klick und Datei kein eigener Code, und eine präparierte Datei erreicht nichts.

## Missing-Docs

- Glossar fehlt (`Dokumentation/Domaenenwissen/` existiert nicht).
- Drosselung des Hochladens über CDP im Chromium des Projekts ist nicht geprobt; Web Locks über Tabs in Playwright nicht geprobt.

## Notizen

- Verworfene Alternativen: „Zu groß beim Auswählen“ (Designsystem) — es gibt keine feste Grenze (B2), die Antwort gibt die Anmeldung; Hochladen schon beim Auswählen — Reservierungen von Dateien, die nie gesendet werden, hielten die Quote; Nachricht zuerst, Dateien danach (ADR-0046 U4).
- Nicht hier: Ablegen und Einfügen (F0104), Anhänge in Direktgesprächen (I0012), Markdown-Leiste „Code“ (I0017).
