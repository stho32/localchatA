---
id: R00029
status: Neu
datum: 2026-10-29
---

# R00029: Anhänge über die API ablegen, senden und ausliefern

## Beschreibung

Bis heute trägt eine Nachricht nur Text. Danach gilt:

- **Anmelden, dann hochladen.** Ein Teilnehmer, Mensch oder Agent, meldet eine Datei mit Name und Größe an. Jede Abweisung (Name, zu viele offene Hochladungen, Schreibkonto, Platte, Quote) kommt als kleine JSON-Antwort, bevor ein Byte der Datei unterwegs ist. Dann lädt er genau so viele Bytes hoch, gestreamt, ohne den Chat zu bremsen. Er kann jederzeit abbrechen.
- **Die Instanz bestimmt die Art** aus dem Inhalt: Bilder, PDF, Video, Audio, Text — alles andere gilt als Datei zum Laden. Name und Angabe des Senders zählen nie.
- **Senden mit Anhängen.** Eine Nachricht trägt bis zu zehn fertig hochgeladene Dateien, auch ohne Text. Die Dateien liegen danach im Tag der Nachricht unter `channels/<name>/anhaenge/<tag>/`, zählen an der Quote und entfallen mit ihrem Tag.
- **Öffnen über einen Link.** `GET /anhaenge/<id>` liefert die Datei ohne Sitzung — wer den Link hat, öffnet ihn, bis der Tag entfällt (Betreiber B4). Dargestellt wird nur Gefahrloses, und jede Antwort läuft in einer Sandbox ohne Skript. Video und Audio lassen sich spulen.
- **Betrieb.** Der Betreiber sieht große Übertragungen und Abweisungen im Betriebsprotokoll, nie eine Anhang-ID; die Leistungszusage „blockiert nichts“ wird gemessen.

Zahlt ein auf: [Vision](R00000-vision.md) — „Anhänge. An eine Nachricht lassen sich eine oder mehrere Dateien hängen.“, „Mensch und KI sind gleichberechtigt“ (dieselben drei Aufrufe für beide) und die Leitplanken „Plattenquote“ und „Lastprofil Team-LAN“. Kein Widerspruch zur Vision.

Herkunft: WBS I0010 „Anhänge senden und öffnen“ (Karte #9), Entscheidungen [ADR-0046](../Dokumentation/ADRs/0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) und [ADR-0047](../Dokumentation/ADRs/0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md) (beide Akzeptiert nach den Antworten des Betreibers B1–B5 vom 2026-10-29).

Slices (WBS `Dokumentation/Planung/localchata.md`, unter I0010): F0097 Anmelden und Hochladen über die API, F0098 Art der Datei erkennen, F0099 Senden mit Anhängen, F0100 Ausliefern mit Art und Teilabruf, F0103 Anhänge im Betrieb. Die Oberfläche (F0101, F0102) beschreibt [R00030](R00030-anhaenge-im-browser-hochladen-und-anzeigen.md).

### Domänenbezug

Kein Glossar im Projekt (`Dokumentation/Domaenenwissen/` fehlt). Begriffe wie in R00011 und R00023 (Kennung, Herkunft, Sitzung, Schreibkonto, Quote, `belegt`, Tag, Frist, Strom). Neu:

- **Anhang** — eine an eine Nachricht gebundene Datei; **Anhang-ID** — 26 Zeichen `[a-z2-7]` (128 Bit Zufall), zugleich die Berechtigung zum Öffnen.
- **Anmeldung** — Ankündigung einer Datei mit Name und Größe, reserviert ihren Platz an der Quote.
- **Eingang** — `<ordner>/.eingang/`, angemeldete und hochgeladene, noch nicht gesendete Dateien.
- **Blockmaß** — ⌈Bytes / 4.096⌉ × 4.096; so zählt ein Anhang an der Quote.
- **Platz für Anhänge** — Quote minus Textreserve (10 % der Quote) minus `belegt`.
- **Art** — der vom Server bestimmte Inhaltstyp (`image/png`, `video/webm`, …, `application/octet-stream`).

## Geschäftlicher Nutzen

Agenten tauschen heute Berichte, Logs und Bildschirmfotos nur als Pfad auf ihrem eigenen Rechner — ein Agent auf einem anderen Rechner kommt nicht heran. Mit R00029 legt ein Agent einen 5-MB-Bericht mit drei Aufrufen in den Channel, und jeder Teilnehmer im LAN lädt ihn mit `curl -O` oder öffnet ihn im Browser. Ein Mensch teilt ein 2-GiB-Abbild, ohne dass der Chat währenddessen stockt, und die Quote schützt die Platte: ist der Platz für Anhänge voll, geht Text weiter.

## Funktionale Anforderungen

### Anmelden und Hochladen über die API (F0097)

- `POST /api/anhaenge` mit Sitzung und JSON `{name, bytes}` (Körper höchstens 2.048 Bytes) meldet eine Datei an. Prüfreihenfolge und Antworten nach ADR-0046 Regel 1: 403 · 401 · 429 (1 Einheit vorab am Schreibkonto) · Körperfehler 415/400/413/408 · 422 `feld` · 422 `bytes` (keine ganze Zahl in u64) · 422 `leer` (0) · 422 `name` mit `grund` · 429 `offen_kennung` \| `offen_herkunft` \| `offen_gesamt` mit `hoechstens` (4, 16, 64) und `Retry-After: 5` · 507 `platte` mit `frei` · 507 `quote` mit `belegt`, `hoechstens`, `frei_fuer_anhaenge`, `reserviert` · 201 `{anhang: {id, name, bytes, hochladen, anlauf_bis, hochladen_bis}}` — `hochladen_bis` ist das Lebensende der Anmeldung.
- **Offene Hochladungen** (ADR-0046 Ergänzung Karte #9 Gate Regel 3): je Kennung höchstens 4 und je Herkunft höchstens 16, gezählt über `angemeldet`, `laeuft` und `fertig`; gesamt höchstens **64 laufende** Hochladungen (`offen_gesamt` zählt nur `laeuft`), geprüft bei der Anmeldung und hart beim Beginn eines `PUT`.
- Namensregel nach ADR-0046 Regel 4: Anzeigename 1–255 Bytes; abgewiesen werden die verbotenen Zeichen aus ADR-0034 Regel 3.6, Tab, CR, LF, Richtungszeichen U+200E, U+200F, U+202A–U+202E, U+2066–U+2069, `/`, `\`, `"` und die Namen `.` und `..`. Gespeichert wird `<hhmmssmmm>-<n>-<ascii>`; der Anzeigename bleibt unverändert in Metazeile, API und `Content-Disposition`.
- Es gibt **keine Grenze je Datei** außer dem Platz für Anhänge und der Platte (Betreiber B2). Platte: `bytes + reserviert + 1 GiB > frei` → 507 `platte`, auch bei `--plattenquote aus`. Quote: `gebunden + reserviert + blockmass(bytes) > hoechstens − hoechstens / 10` → 507 `quote` mit dem Text „Nicht hochgeladen: der Platz für Anhänge ist voll — noch <n> frei; Text kann weiter gesendet werden.“ (Größen wie ADR-0044 Regel 4). Sonst wird `blockmass(bytes)` reserviert, bevor ein Byte geschrieben ist.
- `PUT /api/anhaenge/{id}` mit Sitzung, `Content-Type: application/octet-stream` und `Content-Length` = angemeldete Länge lädt hoch. Vor dem ersten gelesenen Byte: 404 `unbekannt` \| `abgelaufen` (eine fremde ID erscheint als `unbekannt`), 409 `laeuft` \| `fertig`, 429 `offen_gesamt` (die Anmeldung bleibt bestehen), 415, 411 (ohne `Content-Length`, auch `chunked`), 422 `laenge` mit `erwartet` — jeweils mit `Connection: close` und nie mit `100 Continue`. Danach gestreamt in `.eingang/<id>.teil` (angelegt mit `O_CREAT|O_EXCL|O_NOFOLLOW`), dann `rename` mit `RENAME_NOREPLACE`; Erfolg 200 `{anhang: {id, name, bytes, art, breite, hoehe, gueltig_bis}}`.
- **Lebensende** (ADR-0046 Ergänzung Karte #9 Gate Regel 1): bei der Anmeldung wird `lebensende = jetzt + 60 s + 2 × bytes / 1 MiB/s` (in ms aufgerundet) festgelegt; kein Abbruch, kein Erneut und kein Zurücklegen verlängert es. Am Lebensende verfällt eine Anmeldung, ob angemeldet oder laufend: die Reservierung ist sofort frei, ein laufender `PUT` antwortet 408, jeder spätere `PUT` 404 `abgelaufen`, Senden 422 `anhang`/`abgelaufen`. Rechenbeispiel: 9.437.184 Bytes → 60 s + 2 × 9 s = 78 s; 2 GiB → 60 s + 4.096 s ≈ 69 min. Eine fertige Datei ist davon frei und gilt 15 min ab Abschluss.
- **Fristen**: kommen 10 s lang keine weiteren 64 KiB (bzw. der Rest) an → 408; Gesamtfrist `min(beginn + 60 s + Länge / 1 MiB/s, lebensende)` → 408 (Text in der Form der Ergänzung Karte #26 zu ADR-0010). Jeder Abbruch (Frist, Lesefehler, Client weg, zu wenig Bytes, Ende der Instanz, Schreibfehler) löscht die Teildatei; die Anmeldung gilt danach bis `min(jetzt + 60 s, lebensende)`, ein neuer `PUT` auf dieselbe ID ist „Erneut“. Schreibfehler beim Stück: 507 `{regel: "platte", frei}` bei `ENOSPC`, sonst 500.
- **Eine Sperre, ein Gewinner** (Regel 5): Anmelden, Beginnen, Abschließen, Abbruch, Verwerfen, Ablaufen, Nehmen und Zurücklegen sind Übergänge unter der Schreibsperre. Wer einen Übergang verliert, räumt seine Datei: zwei `PUT` auf dieselbe ID → genau einer läuft, der andere 409 `laeuft`; `DELETE` nach dem Nehmen beim Senden → 204 ohne Wirkung; dieselbe ID in zwei gleichzeitigen Sendungen → genau eine bindet, die andere 422 `anhang`/`unbekannt`.
- **Nie überschreiben** (Regel 6): kein `rename` des Eingangs und der Bindung überschreibt eine vorhandene Datei; ein vorhandenes Ziel ist ein Fehler des Schritts.
- `DELETE /api/anhaenge/{id}` → immer 204; für die eigene ID: Teil- oder fertige Datei weg, Reservierung sofort frei, ein laufender `PUT` endet als Abbruch; fremde und unbekannte IDs bleiben unberührt.
- Lebensdauer: eine Anmeldung ohne laufendes Hochladen verfällt 60 s nach der Anmeldung bzw. nach dem letzten Abbruch, spätestens an ihrem Lebensende; eine fertige Datei 15 min nach dem Ende des Hochladens. Der Start prüft `<ordner>/.eingang`: fehlt er, wird er angelegt; ist er ein Symlink oder kein Ordner, endet der Start mit Exit 2 und einer Zeile auf stderr, die den Pfad nennt, und nichts wird gelöscht (Regel 7). Sonst leert er `.eingang/` vor dem Laden (nur reguläre Dateien; Symlinks und Unterordner darin bleiben und werden als `WARNING laden` gemeldet).
- **Nicht blockierend** (B2): Hochladen hält keine Gesprächs- oder Schreibsperre, belegt nie die Werkbank, höchstens 8 Blocking-Threads (Hochladebank) und je Hochladung höchstens ein Stück (≤ 1 MiB) im Speicher.
- `/api/gesundheit` trägt `belegt` (gebunden + reserviert), `reserviert`, `hochladungen` (offen) und `hochladebank_belegt`. Das Anfrageprotokoll kürzt nach dem Prozent-Dekodieren in **jedem** Pfad — auch bei verfehlten Routen ohne Routenmuster — jedes Segment, das mit 26 Zeichen aus `[A-Za-z2-7]` beginnt, auf seine ersten 6 Zeichen und `…` (ADR-0047 Ergänzung Karte #9 Gate Regel 2).

### Art der Datei erkennen (F0098)

- Die Art kommt aus dem Inhalt nach der Tabelle in ADR-0047 Frage 3 (Signaturen aus den ersten 64 KiB, dann die Text-Prüfung über die ganze Datei): PNG, JPEG, GIF, WebP (mit Breite und Höhe) → `image/*`; `%PDF-` → `application/pdf`; MP4 (`ftyp`; `M4A `/`M4B ` → `audio/mp4`, `qt  ` → keine) → `video/mp4`; WebM (`DocType` `webm`; `matroska` → keine) → `video/webm`; Ogg (Opus, Vorbis → `audio/ogg`; Theora → `video/ogg`); MP3 (`ID3` v2–4 oder zwei gültige Rahmenköpfe) → `audio/mpeg`; gültiges UTF-8 ohne NUL → `text/plain; charset=utf-8`; alles andere → `application/octet-stream`.
- Disposition: `inline` für alle Arten der Tabelle außer Text über 16 MiB und `application/octet-stream` — diese `attachment` (nach Betreiber F3).
- **Maße aus dem dekodierten Segment** (ADR-0047 Ergänzung Karte #9 Gate Regel 4): Breite und Höhe stammen nur aus der Struktur, die der Erkenner über die Längenkette erreicht — PNG aus `IHDR` als erstem Chunk, JPEG aus dem ersten über die Segmentlängen ab `SOI` erreichten `SOFn` (nie aus einer Suche nach Markerbytes), GIF aus dem Kopf, WebP aus dem Chunk, den der Kopf nennt. Breite oder Höhe 0 → keine Maße; die Art bleibt.
- SVG, HTML und XML sind Text und werden nie als Dokument dargestellt.
- Bis F0098 gebaut ist, ist jede Datei `application/octet-stream` mit `attachment`.

### Senden mit Anhängen (F0099)

- `POST /api/channels/{name}/nachrichten` nimmt `{text, anhaenge?: [id, …]}`: 1–10 IDs, keine doppelt. Text leer oder nur Leerraum ist erlaubt, wenn mindestens ein Anhang dabei ist (B5), und wird als `""` gespeichert. Abweisungen nach ADR-0034 Regel 3.6: 422 `{regel: "anhang", id, grund}` mit `unbekannt` \| `abgelaufen` \| `fremd` \| `doppelt` \| `unvollstaendig`; 422 `{regel: "anhaenge", hoechstens: 10}`; ohne Text und ohne Anhang 422 `leer` wie bisher.
- Beim Senden wandert jede Datei per `rename` nach `channels/<name>/anhaenge/<tag der nachricht>/<hhmmssmmm>-<n>-<ascii>`; erst danach wird der Satz mit je einer Metazeile `+ anhang: <id> <bytes> <art> <b>x<h>|- <datei> <anzeigename>` angehängt (ADR-0046 Regel 7). Scheitert das — auch weil das Ziel schon existiert (`RENAME_NOREPLACE`, nie überschrieben) —, wandern die Dateien zurück in den Eingang, der Satz ist zurückgebaut, die Antwort 500, nichts erscheint im Verlauf oder Strom, und die Anhänge bleiben bis zu ihrer Frist sendbar. `ERROR senden` nennt eine Datei nur als `<hhmmssmmm>-<n>` mit Gespräch und Tag.
- Eine gebundene ID ist zum Senden, Hochladen und Verwerfen verbraucht — für den Absender wie für einen Dritten, der sie im Channel liest (Öffnen über den Link bleibt): Senden → 422 `anhang`/`unbekannt`, `PUT` → 404 `unbekannt`, `DELETE` → 204 ohne Wirkung; die gebundene Datei bleibt unverändert (Matrix M38).
- Beim Laden ergibt eine unplausible Metazeile (ID falscher Form, `bytes` ≠ Datei, `<datei>` mit `/`, `..` oder ohne Präfix, zwei Zeilen mit derselben ID, Art außerhalb der Tabelle) keinen Eintrag und eine `WARNING laden` (Datei nur als Präfix); nie wird ein Pfad außerhalb `anhaenge/<tag>/` geöffnet (M17). Eine eingeschleuste Zeile `+ anhang: …` im Text bleibt Text (M16).
- Der Wiederholschlüssel gilt für Text **und** Anhang-IDs; Kosten am Schreibkonto bleiben ⌈Textbytes / 4.096⌉, mindestens 1.
- Jede `nachricht` — Senden-Antwort, Verlauf, Ereignis im Strom — trägt `anhaenge: [{id, name, bytes, art, breite, hoehe, url}]`: immer eine Liste, in Sendefolge, `breite`/`hoehe` nur bei Bildern (sonst `null`), `url` = `/anhaenge/<id>`.
- Quote: der Satz prüft nur noch sich selbst gegen die volle Quote (die Anhänge sind seit der Anmeldung reserviert); beim Senden wechseln die Anhangbytes von `reserviert` nach `gebunden`. Beim Laden zählt jede reguläre Datei in `anhaenge/<tag>/` der Tage im Index im Blockmaß an `gebunden`, auch eine Waise. Entfällt ein Tag (Start, Tageswechsel, Verkürzen der Frist), entfällt `anhaenge/<tag>/` nach der Tagesdatei — Symlinks werden nie verfolgt — samt seinen Bytes.

### Ausliefern mit Art und Teilabruf (F0100)

- `GET` und `HEAD /anhaenge/{id}` ohne Sitzung, unter dem Grenzschutz (fremder Host 403). ID nicht in der Form oder unbekannt oder ihr Tag entfallen → 404 `text/plain` „Diesen Anhang gibt es nicht (mehr).“; fehlt die Datei auf der Platte → derselbe 404 und `WARNING anhang` mit Gespräch, Tag und Datei nur als `<hhmmssmmm>-<n>`, nie der ID. Die ID ist nur Schlüssel des Verzeichnisses, nie ein Dateiname.
- **Jede** Antwort unter `/anhaenge` und `/anhaenge/…` — jeder Status und jede Methode, auch 403 des Grenzschutzes und 404/405 des Routers — trägt `Content-Security-Policy: sandbox; default-src 'none'` und `X-Content-Type-Options: nosniff`. Die CSP wählt die eine Stelle `sicherheitsheader_setzen` nach dem Pfad (roh oder prozent-dekodiert), alle anderen Pfade behalten die App-CSP; der Handler der Route setzt keine CSP (ADR-0047 Ergänzung Karte #9 Gate Regel 1). Antworten der Route tragen dazu `Cross-Origin-Resource-Policy: same-origin`, `Referrer-Policy: no-referrer`, `X-Frame-Options: DENY`; Dateiantworten zusätzlich `Content-Type` und `Content-Disposition: <inline|attachment>; filename="<ascii-ersatz>"; filename*=UTF-8''<anzeigename prozentkodiert nach RFC 8187>` (alles außer `attr-char` kodiert), `Accept-Ranges: bytes`, `Content-Length`, `Cache-Control: private, no-cache`, `ETag: "<id>"`.
- Teilabruf nach ADR-0047 Regel 4, Frage 6 und Ergänzung Karte #9 Gate Regel 5: genau ein Bereich `bytes=a-b`, `a-` oder `-n` → 206 mit `Content-Range`; `-n` mit n größer als die Länge → 206 über die ganze Datei; nicht erfüllbar (`a` ≥ Länge, `-0`) → 416 mit `Content-Range: bytes */<länge>`; mehrere Bereiche, mehr als eine `Range`-Kopfzeile, andere Einheit oder unlesbare Angabe → 200 ganz; `If-Range` mit dem ETag → Bereich, schwach (`W/…`), als Datum oder anders → 200 ganz; `HEAD` übergeht `Range` (200-Kopf). **Existenz vor Cache**: erst wird die ID gefunden, dann `If-None-Match: "<id>"` → 304, geprüft vor `Range`; eine entfallene ID antwortet 404, auch mit passendem ETag.
- Gestreamt in Stücken zu 64 KiB mit Gegendruck; die Sendefrist ist die Abrissgrenze; eine Datei, deren Tag während des Sendens entfällt, wird zu Ende gesendet.
- Eine mitgeschickte Sitzung wird nicht geprüft und verlängert nichts.

### Anhänge im Betrieb (F0103)

- Ereignisse nach ADR-0042 Ergänzung Karte #9: `DEBUG anhang` je Anmeldung, Hochladen-Ende, Abbruch, Ablauf und Abruf; `INFORMATION anhang Großes Hochladen abgeschlossen` ab 64 MiB mit `bytes`, `dauer_ms`, `mib_s`; Episoden `WARNING anhang Hochladen zu langsam abgebrochen` je Herkunft, `WARNING quote Anhänge abgewiesen: Platz für Anhänge voll` je Kennung, `WARNING quote Anhang abgewiesen: Platte fast voll` je Herkunft, `WARNING anhang Reservierung verfallen ohne Hochladen` je Herkunft (eine Anmeldung, die verfällt, ohne je fertig geworden zu sein; erste Zeile mit `kennung` und `bytes`; ADR-0046 Ergänzung Karte #9 Gate Regel 4); `abgewiesen.zu_langsam` zählt die 408 des `PUT`; Schreibfehler beim Stück → `ERROR anhang` mit Herkunft und Grund.
- Nie im Protokoll: Anhang-ID (auch nicht im Anfrageprotokoll, auch nicht bei verfehlten Routen), Anzeigename, der ASCII-Teil des gespeicherten Namens, Inhalt. Eine gespeicherte Datei erscheint nur als `<hhmmssmmm>-<n>` mit Gespräch und Tag, eine Datei des Eingangs als ID-Präfix von 6 Zeichen (ADR-0047 Ergänzung Karte #9 Gate Regel 3).
- Messwerte QA3, QS6, QA5 werden gemessen und mit Rechner und Plattenart berichtet.

## Nicht-funktionale Anforderungen

- **Nicht blockierend (QA3, nach ADR-0046; Zweck: Lastprofil „Team-LAN“, Zustellung < 1 s)**: während zwei Teilnehmer gleichzeitig je 1 GiB hochladen, antwortet `GET /api/gesundheit` mit p90 ≤ 0,15 s, Senden einer Textnachricht mit p90 ≤ 0,25 s, und die Nachricht erscheint im Strom < 1 s nach der 201; ein dritter Teilnehmer lädt 1 MiB in der Zeit hoch, die er ohne die beiden braucht (± 50 %); RSS wächst höchstens um 2 × 1 MiB + 50 MB. Gemessen im Release-Build auf dem Zielrechner (B0465), nicht im Gate; Vergleichslauf ohne Hochladungen. Uhr der Zustellung: ab der 201 des Sendens.
- **Langsame Leser (QS4)**: 160 Verbindungen einer Herkunft, die je einen großen Anhang anfordern und nicht lesen, lassen RSS um höchstens 160 × 64 KiB + 50 MB wachsen (≈ 60 MB), unabhängig von der Dateigröße.
- **Spulen (QS6)**: Sprung ans Ende eines 2-GiB-Videos → Wiedergabe setzt im LAN < 1 s später dort ein, ohne die Datei bis dahin zu laden (B0466).
- **Laden (QA5)**: ein Ordner mit 1.000 Anhängen startet ≤ 50 ms langsamer als ohne — gerechnet in ADR-0046, hier gemessen (B0466).
- **Sicherheit**: kein Skript aus einem Anhang erreicht `localStorage` oder die API der App (QS1); Angriffsmatrix nach ADR-0047 Ergänzung Karte #9 Gate als Gate und als Kriterium „Angriffsmatrix“ (Zuordnung Zeile → Linie → Bubble unter „Tests“).
- **Zeitkriterien** nach `AI-Team/planer.md`: Regeln (60 s, 15 min, Stück- und Gesamtfrist) exakt im Unit-Test mit gestellter Uhr; an der Grenze des echten Programms nur obere Grenzen mit Startpunkt und 30 % Spiel.

## Akzeptanzkriterien

Gemeinsam für alle Kriterien, wenn nicht anders genannt: Start mit `TZ=Europe/Berlin localchatA <ordner> --port 0 --host 127.0.0.1 --kein-browser --plattenquote 10MiB` auf einem Wegwerf-Ordner mit dem leeren Ordner `channels/allgemein/`; `<p>` ist der Port aus der Startzeile. Quote 10.485.760 Bytes, Textreserve 1.048.576, **Platz für Anhänge 9.437.184 Bytes** minus `belegt`. `S(<name>)` ist der Sitzungs-Header aus dem Beitritt per `curl` (Agent, Herkunft `127.0.0.1`). „Anmelden n“ heißt `POST /api/anhaenge` mit `{"name":"bericht.pdf","bytes":n}` und `S(beta)`; „Hochladen“ heißt `curl -X PUT --data-binary @<datei> -H 'Content-Type: application/octet-stream'` mit `S(beta)` auf `hochladen` aus der 201; „Senden“ heißt `POST /api/channels/allgemein/nachrichten` mit `S(beta)`. Testdateien entstehen im Test (Bytes im Code oder `ffmpeg` für Medien), nicht im Repository.

### F0097 Anmelden und Hochladen über die API

- [ ] **Hauptweg** (Integration): Anmelden 307.200 → 201 mit 26-Zeichen-`id` aus `[a-z2-7]`, `hochladen` = `/api/anhaenge/<id>`, `anlauf_bis`, `hochladen_bis` = Anmeldung + 60,586 s (60 s + 2 × 307.200 / 1.048.576 s = 60,5859… s, aufgerundet; ± 1 s gegen die Uhr des Prüfers); `/api/gesundheit` zeigt `reserviert` 307.200, `hochladungen` 1, `belegt` 307.200. Hochladen von 307.200 Bytes → 200 mit `gueltig_bis` = Zeitpunkt der Antwort + 15 min (± 1 s); `.eingang/<id>` hat 307.200 Bytes, kein `.teil` liegt im Eingang.
- [ ] **Quote, Rechenbeispiel** (Integration): Anmelden 9.437.184 → 201 (0 + 0 + 9.437.184 ≤ 9.437.184). Anmelden 1 → 507 `{regel: "quote", belegt: 9437184, hoechstens: 10485760, frei_fuer_anhaenge: 0}` mit dem Text „Nicht hochgeladen: der Platz für Anhänge ist voll — noch 0,0 MiB frei; Text kann weiter gesendet werden.“ (ADR-0046 Regel 1.6; Blockmaß von 1 Byte ist 4.096); `reserviert` bleibt 9.437.184. **Text geht weiter**: Senden `{"text":"Hallo"}` → 201. `DELETE` der ersten ID → 204, `reserviert` 0; Anmelden 1 → 201, `reserviert` 4.096. Mutanten „gegen `hoechstens` statt `hoechstens − reserve`“, „ohne Blockmaß“ und „`>=` statt `>`“ werden rot.
- [ ] **Platte** (Integration, auch mit `--plattenquote aus`): Anmelden 1.125.899.906.842.624 (1 PiB) → 507 `{regel: "platte", frei}` mit `frei` > 0, nichts reserviert. Die Regel `bytes + reserviert + 1 GiB > frei` prüft der Unit-Test mit gestellter Plattenauskunft (frei = Länge + 1 GiB − 1 → `platte`, frei = Länge + 1 GiB → ok).
- [ ] **Nur eine Herkunft füllt den Platz** (nach Betreiber F1): mit dem Standard der ADR darf eine Herkunft den ganzen Platz für Anhänge reservieren und binden — `beta` meldet 9.437.184 an → 201; es gibt keine Grenze je Herkunft außer den offenen Hochladungen. Empfehlung des Directors, nicht entschieden: keine Grenze für gebundene Anhänge, Reservierungen je Herkunft höchstens 50 % des Platzes für Anhänge (dann: 4.718.592 → 201, weitere 4.096 aus derselben Herkunft → abgewiesen). Ändert der Betreiber das (Option K7), wird dieses Kriterium ersetzt.
- [ ] **Lebensende** (Unit mit gestellter Uhr; ADR-0046 Ergänzung Karte #9 Gate „Erste Tests“): Anmelden 9.437.184 bei t0 → `hochladen_bis` t0 + 78 s, `anlauf_bis` t0 + 60 s; bei t0 + 50 s `PUT`, 1 Byte, Abbruch → `anlauf_bis` t0 + 78 s (nicht t0 + 110 s); bei t0 + 77,999 s beginnt ein `PUT`; bei t0 + 78,001 s → `reserviert` 0, laufender `PUT` 408, nächster `PUT` 404 `abgelaufen`; eine bei t0 + 70 s fertige Datei gilt bis t0 + 70 s + 15 min und bleibt bei t0 + 79 s stehen. Mutanten „Abbruch setzt Lebensende neu“, „Anlauffrist nach Abbruch ohne `min`“, „Gesamtfrist ohne `min`“ werden rot. **Über das Programm** (Probe M20 wörtlich): „Anmelden 9.437.184, 20 min alle 50 s PUT nach 1 Byte geschlossen → spätestens nach der absoluten Lebensdauer 404 `abgelaufen`, reserviert 0.“ — der erste 404 `abgelaufen` kommt spätestens 103 s nach der 201 (78 s Lebensende + 1 s Takt, + 30 %), danach `reserviert` 0; der Lauf endet mit diesem 404.
- [ ] **Eingang nicht sperrbar** (Integration; Probe M21 wörtlich): „127.0.0.2 und .3 je 4 Kennungen × 4 Anmeldungen à 1 Byte erneuert → beta@127.0.0.1 Anmelden 201, PUT 200; Kontrolle mit einer vollen Herkunft.“ Kontrolle: die 17. Anmeldung von 127.0.0.2 → 429 `offen_herkunft`. Mutante „`offen_gesamt` zählt `angemeldet`“ wird rot.
- [ ] **Abweisungen der Anmeldung** (Integration): ohne Sitzung 401; `Origin: http://evil.example` 403; `{"name":"a","bytes":1,"x":1}` → 422 `feld`; `bytes` `-1`, `1.5`, `"1"`, 18.446.744.073.709.551.616 → 422 `bytes`; `bytes` 0 → 422 `leer`; Körper über 2.048 Bytes → 413; jede Antwort JSON, keine Reservierung.
- [ ] **Namensregel** (Unit, rein): `Bericht März.pdf` → gespeichert `…-Bericht_M_rz.pdf`, Anzeigename unverändert; `../x`, `a/b`, `a\b`, `a"b`, `.`, `..`, leer, 256 Bytes, `rechnung` + U+202E + `fdp.exe`, `a\tb`, `a\nb`, `\u001b[31m` → 422 `name` mit `grund`; 255 Bytes → ok; `x.tar.gz` behält `.gz`, eine Endung über 16 Zeichen entfällt; ein Name nur aus Umlauten ergibt nie einen leeren gespeicherten Namen (`datei`). Über das Programm: Anmelden mit `"name":"../x"` → 422 `name`.
- [ ] **Offene Hochladungen** (Integration): `beta` meldet viermal 1 Byte an, die fünfte → 429 `{grenze: "offen_kennung", hoechstens: 4}` mit `Retry-After: 5`. Vier Kennungen derselben Herkunft mit je vier → die erste Anmeldung einer fünften Kennung → 429 `offen_herkunft`, `hoechstens: 16`. **64 laufende gesamt** (ADR-0046 Ergänzung Karte #9 Gate Regel 3; Matrix M6): 127.0.0.2 bis 127.0.0.5 halten je 16 laufende `PUT` (4 Kennungen × 4, je 131.072 Bytes angemeldet, die ersten 65.536 gesendet, der Rest zurückgehalten, solange die Stückfrist von 10 s nicht greift) — 4 × 16 = 64 laufen, reserviert 64 × 131.072 = 8.388.608 ≤ 9.437.184; ein weiterer `PUT` einer schon vorher angemeldeten ID von 127.0.0.1 → 429 `{grenze: "offen_gesamt", hoechstens: 64}` mit `Retry-After: 5` und `Connection: close`, die Anmeldung bleibt bestehen; währenddessen antworten Senden und `/api/gesundheit` für 127.0.0.1 wie ohne die Hochladungen (201 bzw. 200). Angemeldete und fertige Einträge zählen nie gegen die 64 (Unit: 127.0.0.2 und .3 je 16 `angemeldet` → Anmeldung von 127.0.0.1 → 201).
- [ ] **Hochladen abgewiesen ohne Lesen** (Integration): `Content-Length` 307.199 bei angemeldeten 307.200 → 422 `{regel: "laenge", erwartet: 307200}` mit `Connection: close`; ohne `Content-Length` (`Transfer-Encoding: chunked`) → 411; `Content-Type: text/plain` → 415; zweiter `PUT` nach Erfolg → 409 `fertig`; `PUT` einer ID von `gamma` mit `S(beta)` → 404 `unbekannt`; ausgedachte ID → 404 `unbekannt`. In keinem Fall liegt danach eine `.teil`-Datei im Eingang.
- [ ] **Abbruch und Erneut** (Integration): `PUT` mit `Content-Length` 307.200, nach 100 KiB Verbindung geschlossen → kein `.teil`, `reserviert` unverändert 307.200; ein neuer `PUT` auf dieselbe ID mit allen Bytes → 200.
- [ ] **Fristen** (Unit, rein, `tokio::time::pause`): 64 KiB, dann 9,999 s Stille → läuft; 10,000 s → `stueckfrist`; 1 Byte je 9 s → `stueckfrist` nach 10 s ohne 64 KiB; Gesamtfrist für 10 MiB = 70 s; 10 MiB gleichmäßig mit 1 MiB/s → gelesen; mit 0,9 MiB/s → `gesamtfrist` bei 70 s. Über das Programm: Anmelden 1 MiB, `PUT` mit 64 KiB, dann Stille → 408 spätestens 13 s nach dem letzten Byte (10 s + 30 %), kein `.teil`.
- [ ] **Verfall** (Unit mit gestellter Uhr; Integration als obere Grenze): Anmeldung von 1 Byte ohne `PUT` (Lebensende 60,001 s: 60 s + 2 × 1 / 1.048.576 s, aufgerundet auf ms) — Uhr + 60 s → noch reserviert, + 60,002 s und ein Takt → frei; fertige Datei, Uhr + 15 min + 1 s → Eingang leer, frei. Über das Programm: Anmelden 1 → `reserviert` 4.096 bei 30 s nach der 201 und 0 spätestens 80 s nach der 201 (60 s Frist, 1 s Takt, 30 % Spiel); im Protokoll steht die Episode `WARNING anhang Reservierung verfallen ohne Hochladen` mit `kennung=beta@127.0.0.1` und `bytes=1`.
- [ ] **Verwerfen** (Integration): `DELETE` der eigenen ID während eines laufenden `PUT` → 204, der `PUT` endet ohne 200, kein `.teil`, `reserviert` 0 sofort (nächste Abfrage der Gesundheit); `DELETE` einer ID von `gamma` mit `S(beta)` → 204, `gamma` kann danach hochladen; `DELETE` einer unbekannten ID → 204.
- [ ] **Start leert den Eingang** (Integration): Instanz beenden, während eine Datei fertig und eine angemeldet ist; Neustart → `.eingang/` enthält keine reguläre Datei, `reserviert` 0, `hochladungen` 0. Ein Symlink und ein Unterordner in `.eingang/` bleiben, je eine `WARNING laden` steht im Protokoll; das Ziel des Symlinks bleibt unberührt. Dasselbe nach `kill -9` während eines `PUT` (M25).
- [ ] **Eingang als Symlink** (Integration; M26): ist `<ordner>/.eingang` ein Symlink auf einen fremden Ordner mit einer Datei, endet der Start mit Exit 2 und einer stderr-Zeile mit dem Pfad; der fremde Ordner und seine Datei sind unverändert. Ist `.eingang` eine reguläre Datei → ebenfalls Exit 2, die Datei bleibt. Fehlt `.eingang` → der Start legt ihn an.
- [ ] **Wettläufe** (Unit mit Barriere in der Attrappe; M23): zwei gleichzeitige `PUT` auf dieselbe ID → genau einer läuft, der andere 409 `laeuft`; `DELETE` zwischen Nehmen und `rename` beim Senden → die Bindung gelingt, `DELETE` 204 ohne Wirkung; `DELETE` zwischen letztem Stück und `rename` des `PUT` → der `PUT` löscht seine `.teil`, 404; Ablauf während des Sendens wartet auf die Sperre; dieselbe ID in zwei gleichzeitigen Sendungen in zwei Channels → genau eine 201, die andere 422 `anhang`/`unbekannt`. Nach jedem Schritt gilt die Invariante aus ADR-0046 Regel 8, im Eingang bleibt keine Waise. Über das Programm: zwei gleichzeitige `PUT` → 200 und 409.
- [ ] **Schreibfehler** (Unit mit Attrappe; M24): `ENOSPC` beim zweiten Stück → 507 `{regel: "platte", frei}`, kein `.teil`, Anmeldung wieder `angemeldet` bis zu ihrem Lebensende, `ERROR anhang` mit Herkunft und Grund; `EIO` → 500, sonst gleich.
- [ ] **Nie überschreiben** (Unit mit Attrappe; Integration; M39): liegt `.eingang/<id>` beim Abschluss schon vor (untergeschobene Datei) → sie bleibt byte-gleich, der `PUT` antwortet 500, kein `.teil`. Liegt das Ziel `anhaenge/<tag>/<datei>` beim Senden schon vor → Rückbau, 500, das vorhandene Ziel byte-gleich, `ERROR senden`. Mutante „`rename` ohne `NOREPLACE`“ wird rot.
- [ ] **Angriffsmatrix Hochladen** (Integration über das echte Programm, Rohanfragen; M5, M32, M33, M12, M34; Kontrollpaar K1 und K2 grün): `Content-Length` größer als gesendet → Stückfrist 408; doppelt gleich oder ungleich, `+5`, `-5`, `Transfer-Encoding: chunked` mit `Content-Length`, `Transfer-Encoding: identity` → 400, 411 oder 422, nie `100 Continue`, mit `Connection: close`, keine zweite Anfrage aus den Körperbytes; `Expect: 100-continue` bei einer Abweisung → die Abweisung ohne `100 Continue`; `X-HTTP-Method-Override: DELETE` auf `POST` → wirkt nicht; Methode `put` klein, Pfad mit `..%2f` oder `%00`, ID in Großbuchstaben → 405 bzw. 404, nie eine Datei außerhalb des Eingangs berührt; `POST`, `PUT`, `DELETE /api/anhaenge…` mit `Origin: http://evil.example`, `Sec-Fetch-Site: cross-site`, als `text/plain`-Formular oder mit fremdem Host → 403 ohne Wirkung (`reserviert` unverändert); `OPTIONS` → ohne `Access-Control-Allow-*`.
- [ ] **Nicht blockierend, schnell geprüft** (Integration, Debug-Build): während ein `PUT` über 64 MiB mit 4 MiB/s läuft, antwortet Senden einer Textnachricht 201, und ein zweiter Teilnehmer meldet an und lädt 1 KiB hoch (200). Die Leistungswerte misst F0103.
- [ ] **Kennzahlen und ID gekürzt** (Integration): `/api/gesundheit` hat die Felder `belegt`, `reserviert`, `hochladungen`, `hochladebank_belegt` (0 im Ruhezustand). Mit `--protokoll-stufe debug`: nach Anmelden und Hochladen enthält `logs/<heute>.log` die ID nicht (`grep -i <id>` = 0), wohl aber ihre ersten 6 Zeichen im gekürzten Pfad. **Verfehlte Routen** (M18): `GET /anhaenge/<id>/x`, `GET /api/anhaenge/<id>/`, die ID prozentkodiert (`%61…`) und in Großbuchstaben → danach `grep -i <id> logs/` = 0. Mutanten „kürzen vor dem Dekodieren“ und „kürzen nur mit Routenmuster“ werden rot.

### F0098 Art der Datei erkennen

- [ ] **Tabelle** (Unit, rein; Tabelle in ADR-0047 Frage 3): PNG-Kopf mit `IHDR` 1280 × 800 → `image/png`, inline, 1280 × 800; JPEG, GIF, WebP (`VP8 `, `VP8L`, `VP8X`) je mit ihren Maßen; `%PDF-1.7` → `application/pdf`; `….ftypisom` → `video/mp4`; `….ftypM4A ` → `audio/mp4`; `….ftypqt  ` → `application/octet-stream`; EBML mit `DocType` `webm` → `video/webm`, `matroska` → `application/octet-stream`; `OggS` + `OpusHead` → `audio/ogg`, + `\x80theora` → `video/ogg`; `ID3\x04` → `audio/mpeg`; ein einzelner MPEG-Rahmenkopf ohne zweiten → kein MP3; `<svg …><script>` → `text/plain; charset=utf-8`, inline; dasselbe mit NUL-Byte → `application/octet-stream`, `attachment`.
- [ ] **Maße aus dem Segment** (Unit; M27): JPEG mit den Bytes `FF C0` und falschen Maßen in einem APP1-Segment vor dem echten `SOF0` 640 × 480 → 640 × 480; `SOF0` erst hinter einer falschen Segmentlänge → keine Maße; PNG, dessen erster Chunk nicht `IHDR` ist → keine Maße; Breite oder Höhe 0 → Art bleibt, `breite`/`hoehe` `null`; 65.535 × 65.535 → Maße wie angegeben (die Vorschau-Grenze gehört zu R00030). Mutante „Markersuche statt Längenkette“ wird rot.
- [ ] **Text über 16 MiB** (Unit; nach Betreiber F3): 16.777.216 Bytes `a` → Text, inline; 16.777.217 → Text, `attachment`.
- [ ] **Stückelung** (Unit): dieselbe Datei in Stücken zu 1, 3 und 65.536 Bytes → gleiches Urteil, auch mit `ä` (2 Bytes) und `😀` (4 Bytes) über einer Stückgrenze und mit ungültigem UTF-8 genau an der Grenze.
- [ ] **Kaputte Köpfe** (Unit): jede Signatur der Tabelle abgeschnitten nach 1, 4, 8, 16 und 32 Bytes, Längenfelder 0 und 2⁶⁴ − 1, MPEG-Rahmenlänge 0 → nie Panik, keine Schleife, höchstens 64 KiB für Signaturen gelesen; Ergebnis Text oder `application/octet-stream`.
- [ ] **Name und Sender zählen nicht** (Unit): HTML-Inhalt mit Anzeigename `bild.png` → Text; PNG-Inhalt mit Name `x.txt` → `image/png`. Mutanten „Endung entscheidet“, „SVG inline als Bild“, „Text ohne UTF-8-Prüfung“, „Text über 16 MiB inline“, „MP3 ohne zweiten Rahmenkopf“ werden rot.
- [ ] **Über das Programm** (Integration): je eine Datei jeder Zeile der Tabelle anmelden und hochladen → die 200 nennt `art`, `breite`, `hoehe` wie die Tabelle (`breite`/`hoehe` `null` außer bei Bildern).

### F0099 Senden mit Anhängen

- [ ] **Nur Anhang, ohne Text** (Integration; B5): 307.200 Bytes anmelden und hochladen; Senden `{"text":"","anhaenge":["<id>"]}` → 201 mit `text: ""` und `anhaenge: [{id, name: "bericht.pdf", bytes: 307200, art, breite: null, hoehe: null, url: "/anhaenge/<id>"}]`; `channels/allgemein/anhaenge/<heute>/` enthält genau eine Datei `<hhmmssmmm>-1-bericht.pdf` mit 307.200 Bytes, `.eingang/` ist leer; der Satz in `channels/allgemein/<heute>.log` trägt `+ anhang: <id> 307200 <art> - <datei> bericht.pdf`; `alpha` liest im Strom dieselbe Liste; `GET /api/channels/allgemein/nachrichten` zeigt sie ebenso.
- [ ] **Zähler beim Senden** (Integration): vor dem Senden `belegt` = `reserviert` = 307.200 (Blockmaß 75 × 4.096); danach `reserviert` 0 und `belegt` = 307.200 + Bytes des Satzes. Ein Text mit drei Anhängen zu 1, 4.096 und 4.097 Bytes zählt 4.096 + 4.096 + 8.192 = 16.384 Anhangbytes.
- [ ] **Abweisungen** (Integration): dieselbe ID ein zweites Mal senden → 422 `{regel: "anhang", grund: "unbekannt"}`; dieselbe ID zweimal in einer Liste → 422 `doppelt`; eine angemeldete, nicht hochgeladene ID → 422 `unvollstaendig`; eine ID von `gamma` → 422 `fremd`; 11 IDs → 422 `{regel: "anhaenge", hoechstens: 10}`; `{"text":""}` und `{"text":"  ","anhaenge":[]}` → 422 `leer`; `{"text":"x","anhaenge":["…"],"y":1}` → 422 `feld`. In keinem Fall ändert sich die Tagesdatei.
- [ ] **Gebundene ID eines Dritten** (Integration; M38): `gamma` liest die gebundene ID von `beta` im Verlauf und sendet sie → 422 `{regel: "anhang", grund: "unbekannt"}`, `PUT` → 404 `unbekannt`, `DELETE` → 204; die Datei in `anhaenge/<heute>/` bleibt byte-gleich, `GET /anhaenge/<id>` → 200.
- [ ] **Metazeile eingeschleust** (Unit und Integration; M16): Senden `{"text":"+ anhang: aaaaaaaaaaaaaaaaaaaaaaaaaa 1 text/plain - x x"}` → 201 mit `anhaenge: []`; nach dem Neustart steht die Zeile als Text im Verlauf, `GET /anhaenge/aaaaaaaaaaaaaaaaaaaaaaaaaa` → 404.
- [ ] **Text mit zehn Anhängen** (Integration): Text `Hier die Belege` mit 10 Anhängen → 201; die Liste hat 10 Einträge in der Reihenfolge des Körpers, die Tagesdatei 10 Metazeilen, `anhaenge/<heute>/` 10 Dateien mit `-1-` bis `-10-` im Namen.
- [ ] **Wiederholschlüssel** (Integration): zweimal derselbe Körper mit demselben Schlüssel → zweimal 201 mit derselben Zeit, ein Satz; derselbe Schlüssel mit anderem Text oder anderer ID → wie ein neuer Text (ADR-0034 Ergänzung Karte #9).
- [ ] **Nichts halb** (Unit mit Attrappe): scheitert das zweite `rename` oder das Anhängen des Satzes → beide Dateien wieder im Eingang, kein Satz, nichts im Index, kein Ereignis, `reserviert` unverändert, die IDs sind danach sendbar; scheitert das Zurücklegen → die Datei bleibt als Waise im Tag, `ERROR senden` mit Datei und Grund. Invariante `gebunden = Σ Verlauf + Σ Anhangbytes je Tag`, `reserviert = Σ Eingang` nach jedem Schritt (erster Unit-Test aus ADR-0046 mit Quote 100.000).
- [ ] **Neustart** (Integration): nach dem Kriterium „Nur Anhang“ Instanz neu starten → der Verlauf liefert dieselbe `anhaenge`-Liste, `belegt` = Bytes der Tagesdatei + 307.200; eine Datei in `anhaenge/<heute>/` ohne Metazeile (Waise) zählt mit, ein Symlink dort zählt nicht. Eine unlesbare Metazeile (`+ anhang: zu kurz`) macht den Satz nicht defekt, der Anhang fehlt in der Liste, `WARNING laden` nennt Tagesdatei und Zeile. **Unplausible Metazeilen** (Unit; M17), von Hand in die Tagesdatei geschrieben: ID falscher Form, `bytes` ≠ Größe der Datei, `<datei>` mit `/`, mit `..` oder ohne Präfix `<hhmmssmmm>-<n>`, zwei Zeilen mit derselben ID, Art außerhalb der Tabelle → je kein Eintrag, eine `WARNING laden` (eine Anhangdatei nur als Präfix), keine Datei außerhalb `anhaenge/<tag>/` geöffnet.
- [ ] **Entfallen** (Integration mit `LOCALCHATA_JETZT`): eine Nachricht mit Anhang von vor 7 Tagen (Frist 7) → beim Start entfallen Tagesdatei und `anhaenge/<tag>/`, `belegt` zählt beides nicht mehr; ein Symlink in `anhaenge/<tag>/` auf eine Datei außerhalb → der Symlink ist weg, sein Ziel bleibt. Gleiches beim Tageswechsel und beim Verkürzen der Frist (`PUT /api/channels/allgemein/frist`). Mutante „Entfallen vergisst Anhangbytes“ wird rot.

### F0100 Ausliefern mit Art und Teilabruf

- [ ] **Öffnen ohne Sitzung** (Integration): `x.html` mit `<script>localStorage.t</script>` hochladen, senden, `GET /anhaenge/<id>` ohne Sitzung → 200, `Content-Type: text/plain; charset=utf-8`, `Content-Disposition: inline; filename="x.html"; filename*=UTF-8''x.html`, CSP genau `sandbox; default-src 'none'`, `nosniff`, Körper byte-gleich mit der Datei.
- [ ] **Download unter dem Anzeigenamen** (Integration): eine Datei mit NUL-Bytes, Anzeigename `Bericht März.bin` → `application/octet-stream`, `Content-Disposition: attachment; filename="Bericht_M_rz.bin"; filename*=UTF-8''Bericht%20M%C3%A4rz.bin`. **Sonderzeichen** (Unit und Integration; M28): Anzeigenamen mit `;`, `'`, `%`, `*`, Leerzeichen, Umlauten und `😀` → `filename*` kodiert alles außer `attr-char` (`a;b'c%d*e f.txt` → `filename*=UTF-8''a%3Bb%27c%25d%2Ae%20f.txt`), `filename` ist ASCII in Anführungszeichen, die Kopfzeile ist parsebar, und `filename*` dekodiert ergibt den Anzeigenamen.
- [ ] **Kopfzeilen überall** (Integration; M14): Antworten 200, 206, 304, 404 aus dem Handler, 404 aus dem Router (`/anhaenge/x/y`), 416, `HEAD`, 403 (`Host: evil.example`) und 405 (`POST /anhaenge/<id>`) tragen alle die CSP genau `sandbox; default-src 'none'` und `nosniff`; die Antworten der Route dazu `Cross-Origin-Resource-Policy: same-origin`, `Referrer-Policy: no-referrer`, `X-Frame-Options: DENY`; 200 und 206 dazu `Accept-Ranges: bytes`, `ETag: "<id>"`, `Cache-Control: private, no-cache`. `/%61nhaenge/x` → `sandbox`; `/anhaengex`, `/api/anhaenge` und `/` → App-CSP. Mutante „Weiche fehlt“ (App-CSP auf jeder Antwort, Stand heute) wird rot; Mutationsprobe je Kopfzeile: jede einzeln entfernt → ein Test rot.
- [ ] **Teilabruf** (Unit, rein; ADR-0047 „Durchsetzung“): Länge 1.000: `bytes=0-0` → Teil(0, 0); `0-` → Teil(0, 999); `-1` → Teil(999, 999); `-0` → unerfüllbar; `5-2` → Ganz; `1000-` → unerfüllbar; `0-1010` → Teil(0, 999); `18446744073709551615-` → unerfüllbar; `18446744073709551616-` → Ganz; `0-1,2-3` → Ganz; `items=0-1` → Ganz; `Bytes=0-1` → Ganz; ` bytes=0-1` → Ganz; `-1010` → Teil(0, 999); zwei `Range`-Kopfzeilen → Ganz; `If-Range: W/"<id>"` → Ganz; `If-Range` als Datum → Ganz (M31).
- [ ] **Teilabruf über das Programm** (Integration): WebM mit n Bytes: `Range: bytes=100-199` → 206, `Content-Range: bytes 100-199/<n>`, `Content-Length: 100`, die 100 Bytes gleich der Datei; `bytes=<n>-` → 416, `Content-Range: bytes */<n>`; `bytes=0-1,5-6` → 200 ganz; `If-Range: "<id>"` mit `bytes=0-9` → 206, `If-Range: "anders"` → 200 ganz; `If-None-Match: "<id>"` → 304 ohne Körper, auch mit `Range`; `bytes=-<n + 10>` → 206 `bytes 0-<n−1>/<n>`; `HEAD` mit `Range: bytes=0-9` → 200-Kopf mit `Content-Length: <n>`; 1.000 Bereichsanfragen nacheinander auf einer Verbindung → die Zahl offener Deskriptoren der Instanz (`/proc/<pid>/fd`) ist danach dieselbe wie davor (M31).
- [ ] **Nicht gefunden** (Integration): `GET /anhaenge/abc` → 404 „Diesen Anhang gibt es nicht (mehr).“; ausgedachte gültige ID → derselbe 404; Datei auf der Platte gelöscht → derselbe 404 und `WARNING anhang` mit Gespräch, Tag und Datei nur als `<hhmmssmmm>-<n>`, ohne die ID und ohne den ASCII-Teil; nach dem Entfallen des Tags (`LOCALCHATA_JETZT` + 7 Tage, Neustart) → 404, **auch mit `If-None-Match: "<id>"`** (Existenz vor Cache, M30; Mutante „ETag vor `finden`“ wird rot). Pfade `/anhaenge/..%2fanhaenge`, `/anhaenge/<id>%00`, die ID in Großbuchstaben → 404 (M33).
- [ ] **Grenzschutz** (Integration): `Host: evil.example` → 403; eine mitgeschickte Sitzung ändert die Antwort nicht und verlängert die Sitzung nicht (Restzeit unverändert).
- [ ] **Kein Skript im Ursprung** (E2E, Chromium; QS1): im Browser A, beigetreten und mit `localStorage`-Token, `page.goto('/anhaenge/<id>')` je für HTML mit `<script>`, SVG mit `<script>`, PNG-Kopf + HTML (Polyglot) und WebM + angehängtes HTML (Polyglot) → kein Skript läuft (ein Skript, das `document.title` setzen würde, setzt ihn nicht), HTML und SVG erscheinen als Quelltext, das Token in `localStorage` der App bleibt unverändert. **Skript-Gadget** (M15): eine App-Seite bindet einen Text-Anhang mit JavaScript ein als `<script src>`, `<script type=module>` bzw. `import()`, `new Worker`, `new SharedWorker`, `navigator.serviceWorker.register` und einen Text-Anhang mit CSS als `<link rel=stylesheet>` → nichts wird ausgeführt oder angewandt (die Konsole meldet die MIME-Prüfung unter `nosniff`). Firefox prüft der Entwicklertest von Hand (Linie L2r).
- [ ] **Bilder unverändert** (nach Betreiber F4; Integration): ein JPEG mit EXIF-Block (GPS-Ort, Kameramodell, Aufnahmezeit) hochladen, senden, `GET /anhaenge/<id>` → Körper byte-gleich mit der Datei, EXIF inklusive — Standard der ADR bis zur Antwort. Verlangt der Betreiber das Entfernen beim Hochladen oder einen Hinweis in der Oberfläche (R00030), wird dieses Kriterium ersetzt.
- [ ] **Direkter Medienlink** (nach Betreiber F2): `page.goto('/anhaenge/<id>')` auf eine WebM-Datei öffnet das Medien-Dokument des Browsers, das selbst abspielt — mit dem Standard der ADR hingenommen. Verlangt der Betreiber „nie Autoplay“, gilt ADR-0047 Pre-Mortem 4 (M3 mit `Sec-Fetch-Dest`) und dieses Kriterium wird ersetzt.
- [ ] **Langsame Leser** (Integration; QS4): 160 Verbindungen einer Herkunft fordern je eine Datei von 64 MiB an und lesen nicht → RSS der Instanz wächst um höchstens 160 × 64 KiB + 50 MB; jede Verbindung ist nach der Abrissgrenze zu.

### F0103 Anhänge im Betrieb

- [ ] **Großes Hochladen** (Integration): eine Datei von 64 MiB hochladen → genau eine Zeile `INFORMATION anhang Großes Hochladen abgeschlossen` mit `bytes=67108864`, `dauer_ms`, `mib_s`; bei 64 MiB − 1 Byte keine solche Zeile.
- [ ] **Episoden** (Integration): drei 408 des `PUT` aus derselben Herkunft → eine Episode `WARNING anhang Hochladen zu langsam abgebrochen` (Beginn und Ende mit Zahl) und `abgewiesen.zu_langsam` + 3; drei 507 `quote` an Anmeldungen von `beta` → eine Episode `WARNING quote Anhänge abgewiesen: Platz für Anhänge voll` mit `kennung=beta@127.0.0.1`; 507 `platte` → `WARNING quote Anhang abgewiesen: Platte fast voll`.
- [ ] **Debug** (Integration, `--protokoll-stufe debug`): je eine `DEBUG anhang`-Zeile für Anmeldung, Hochladen-Ende, Abbruch, Ablauf und Abruf mit `kennung`, `herkunft`, `bytes`, `art`, `dauer_ms`, beim Abruf mit Range `bereich` als Länge.
- [ ] **Keine ID, kein Name** (Integration; Matrix M11, M18, M19; ADR-0047 Ergänzung Karte #9 Gate Regel 3): auf Debug Anmelden, Hochladen, Senden, Abruf mit und ohne Range, 404, Abbruch, Ablauf, verfehlte Routen mit der ID, eine auf der Platte gelöschte Anhangdatei (`WARNING anhang`) und eine unplausible Metazeile (`WARNING laden`) mit Anzeigename `Geheimplan Q4.pdf` → `grep -ri <id> logs/` = 0, `grep -r 'Geheimplan' logs/` = 0 (Anzeigename **und** ASCII-Teil `Geheimplan_Q4.pdf` des gespeicherten Namens); jede Meldung, die die Datei nennt, nennt sie als `<hhmmssmmm>-<n>` mit Gespräch und Tag.
- [ ] **Episode Reservierung verfallen** (Integration; M22): `beta` meldet dreimal 1 Byte an und lädt nichts hoch → spätestens 80 s nach der letzten 201 eine Episode `WARNING anhang Reservierung verfallen ohne Hochladen` mit `kennung=beta@127.0.0.1` und `bytes=1` in der ersten Zeile; eine Anmeldung, die fertig hochgeladen wurde und nach 15 min verfällt, zählt nicht mit. 507 `quote` nennt `reserviert` (Kriterium „Quote“).
- [ ] **Messwert QA3** (Entwicklertest, Release-Build, Zielrechner): zwei `PUT` à 1 GiB gleichzeitig; Gesundheit alle 100 ms → p90 ≤ 0,15 s; ein dritter Client sendet 5 Textnachrichten/s → Antwort p90 ≤ 0,25 s, Zustellung im Strom < 1 s ab der 201; ein vierter lädt 1 MiB hoch in der Zeit des Vergleichslaufs ± 50 %; RSS-Zuwachs ≤ 2 × 1 MiB + 50 MB. Der Bericht nennt Rechner, RAM und Plattenart (SSD/HDD) und die Werte des Vergleichslaufs ohne Hochladungen.
- [ ] **Messwert QS6** (Entwicklertest): 2-GiB-WebM, im Player ans Ende springen → die Wiedergabe setzt < 1 s später ein; die Summe der übertragenen Bytes bis dahin liegt unter 64 MiB.
- [ ] **Messwert QA5** (Entwicklertest, Release-Build): Ordner mit 1.000 Anhängen (je 1 KiB, über 7 Tage) → `ladezeit_ms` höchstens 50 ms über dem Ordner ohne Anhänge; gemessen dreimal, Median.

### Angriffsmatrix (alle Slices)

- [ ] **Matrix nach ADR-0047 Ergänzung Karte #9 Gate**: jede Zeile M1–M34, M38, M39 außer M13 (gehört zu I0012) und dem Firefox-Teil (Linie L2r, von Hand im Entwicklertest) hat einen Test auf der Linie und in der Bubble, die die Tabelle „Angriffsmatrix: Zuordnung“ (unter „Tests“) nennt; jede Zeile mit Wächter wird unter dessen Mutante rot. Das Kontrollpaar K1 (gutartiger Ablauf `alpha@127.0.0.1`: Anmelden 300 KiB PNG → 201, `PUT` → 200 `image/png`, Senden → 201, `GET /anhaenge/<id>` → 200 mit `sandbox`-CSP, `Range: bytes=0-99` → 206) und K2 (127.0.0.2 mit 16 offenen Anmeldungen, die 17. → 429 `offen_herkunft`) läuft in jeder Linie mit, in der eine Probe läuft, und ist grün. Der Bericht des Entwicklers nennt je Zeile Test und Mutante.

## Betroffene Verzeichnisstruktur

- HTTP: `src/http/` — neu `anhaenge.rs` (Anmelden, Hochladen, Verwerfen) und `anhang_ausliefern.rs` (`GET`/`HEAD`, Bereich, Kopfzeilen); Änderungen in `senden.rs`, `nachrichten.rs`, `grenzschutz.rs`, `gesundheit.rs`, `anfrageprotokoll.rs`, `mod.rs` (Routen).
- Gespräche: `src/gespraeche/` — neu `anhangeingang.rs` (A00017), `anhangpruefung.rs` (A00018), `anhangverzeichnis.rs` (A00016); Änderungen in `mod.rs` (Reservieren, Binden, Zähler), `logformat.rs` (Metazeile), `logablage.rs` (Eingang, `platz_frei`, Datei öffnen), `pflege.rs` (Entfallen der Anhänge), `wiederholung.rs`.
- Start: `src/start/` (Eingang leeren, Sekundenaufgabe).
- Tests: `tests/anhaenge_*.rs` (Integration), Unit-Tests in den Modulen, `e2e/tests/r00029-*.spec.js` (Kein Skript im Ursprung); Messwerkzeuge unter `examples/`.

## Technische Überlegungen

### Ablauf

1. **Anmelden** `POST /api/anhaenge` → Grenzschutz, Sitzung, Schreibkonto → Körper → Name → offene Hochladungen → Platte → Quote (Schreibsperre, reservieren) → 201.
2. **Hochladen** `PUT /api/anhaenge/{id}` → Eingangszustand, Kopf prüfen → Stücke unter der Hochladebank in `.teil`, Art erkennen, Fristen → `rename` → 200 \| Abbruch.
3. **Senden** `POST …/nachrichten` mit `anhaenge` → Körper → Wiederholschlüssel → unter Gesprächs- und Schreibsperre: nehmen, Zeit, Quote für den Satz, `rename` je Anhang, Satz anhängen, Zähler, Verzeichnis, Ereignis.
4. **Ausliefern** `GET /anhaenge/{id}` → Grenzschutz → finden → 304 \| 404 \| 416 \| 200/206 gestreamt.
5. **Pflege**: Sekundentakt lässt Eingangseinträge verfallen; Tagestakt und Start lassen Tage samt Anhängen entfallen.

### Optionen

Die Optionen sind in ADR-0046 (Fragen 1–7) und ADR-0047 (Fragen 1–7) entschieden; die Anforderung übernimmt sie. Offen ist nur der Schnitt:

- **A: fünf Slices, Art erkennen getrennt** (gewählt). Stärke: F0097 und F0098 bleiben unter neun Bubbles; bis F0098 ist jede Datei ein Download — der sichere Ausgang; F0098 und F0099 parallel. Schwäche: ein Slice mehr, F0100 braucht beide.
- B: Art erkennen in F0097 (Zuschnitt a des architekt). Stärke: ein Slice weniger. Schwäche: 13 Bubbles in einem Slice.
- C: Medien (Erkennung, Teilabruf, Player) als eigener Slice quer durch Server und Browser. Stärke: fachlich geschlossen. Schwäche: Teilabruf dient auch PDF und Downloads, und der Player-Teil gehört in die Oberfläche (R00030).

### Grobentwurf (Vorplanung)

Bubbles aus der WBS, als Vorplanung; welche es am Ende werden, entscheidet der Entwickler.

- F0097: B0424 Namensregel prüfen · B0425 Eingang führen (unklar) · B0426 Platz reservieren (unklar) · B0427 Anmelden beantworten · B0428 Hochladefristen · B0429 Stücke schreiben (unklar) · B0430 Verwerfen · B0431 Eingang beim Start und im Takt · B0432 Anhang-Kennzahlen und gekürzte ID · B0468 Angriffsmatrix Hochladen über das Programm (unklar; Testbubble, Linie L2).
- F0098: B0433 Bildart und Maße (unklar) · B0434 Medienart (unklar) · B0435 Text über Stückgrenzen · B0436 Art beim Hochladen.
- F0099: B0437 Metazeile schreiben und lesen · B0438 Anhangverzeichnis · B0439 Sendekörper mit Anhängen prüfen · B0440 Binden (unklar) · B0441 Anhänge in der Nachricht · B0442 Anhänge laden · B0443 Anhänge entfallen mit dem Tag.
- F0100: B0444 Bereich lesen · B0445 Kopfzeilen der Anhang-Antwort · B0446 Anhang ausliefern · B0447 Gestreamt senden (unklar) · B0467 Sicherheitsheader je Pfad.
- F0103: B0462 Anhang-Ereignisse · B0463 Anhang-Episoden · B0464 Keine Anhang-ID im Protokoll belegt · B0465 Messwert QA3 (unklar) · B0466 Messwerte QS6 und QA5 (unklar).

Elemente (Schnittstellen in A00016–A00018): `Anhangeingang` (Operation, Zustände und Fristen), `Erkenner` und `name_pruefen` (rein), `Anhangverzeichnis` (Operation), `bereich(angabe, laenge) -> Ganz | Teil(a, b) | Unerfuellbar` (rein), Plattenauskunft als Port der Logablage.

### Änderungen an bestehenden Klassen

- `Gespraeche`: Zähler `gebunden` und `reserviert`, `reservieren`, `senden` mit Anhängen, Laden und Entfallen mit Anhangbytes.
- Senderoute: Körper `{text, anhaenge?}`, Leer-Regel mit Anhang, Wiederholschlüssel über Text und IDs.
- `grenzschutz.rs`: `sicherheitsheader_setzen` (heute Zeilen 341–361, `kopf.insert` mit der App-CSP für jede Antwort) wählt die CSP nach dem Pfad — die Weiche, B0467 (die App-CSP mit `media-src` gehört zu R00030 F0102).
- `anfrageprotokoll.rs`: nach dem Dekodieren jedes ID-förmige Segment kürzen, auch ohne Routenmuster (heute Zeile 63 schreibt den ganzen dekodierten Pfad).
- `Anhangeingang`: Feld `lebensende` ohne Setter außer in `anmelden`; Übergänge unter der Schreibsperre.

## Tests

Nach Skill `test-pyramide`; je Kriterium die genannte Mutante, das Ergebnis steht im Bericht des Entwicklers.

- **Unit (rein)**: Namensregel, Erkenner (Tabelle, Stückelung, kaputte Köpfe), Fristen, Bereich, Kopfzeilen, Metazeile, Blockmaß; Eingang und Gespräche mit Speicherablage, gestellter Uhr, festem Zufall und gestellter Plattenauskunft (erster Unit-Test aus ADR-0046 samt Mutanten).
- **Integration** (echtes Programm, Wegwerf-Ordner): alle Kriterien „Integration“ oben; Probe gegen den echten Einstieg für Grenzschutz, Kopfzeilen und Protokoll.
- **E2E**: „Kein Skript im Ursprung“ samt Skript-Gadget (Chromium).
- **Messwerte**: QA3, QS6, QA5 im Entwicklertest, nicht im Gate.

### Angriffsmatrix: Zuordnung

Matrix nach ADR-0047 Ergänzung Karte #9 Gate (Zeilen M1–M13 der ADR, M14–M39 der Ergänzung). Linien: **L1** im Prozess (Unit, Integration mit Speicherablage, Testuhr, festem Zufall, Attrappen) · **L2** echtes Programm über die Annahmeschleife (Debug-Build, Wegwerf-Ordner, `--port 0`), Browserzeilen mit Playwright/Chromium · **L2r** von Hand im Entwicklertest (Firefox, Zielrechner). Der Test steht in der erstgenannten Bubble, die übrigen tragen den Wächter.

| Zeile | Angriff (kurz) | Linie | Bubble | Kriterium |
|---|---|---|---|---|
| K1, K2 | Kontrollpaar | L1, L2 | B0468, in jeder Probe mit | Angriffsmatrix (alle Slices) |
| M1 | aktiver Inhalt | L1, L2 (Chromium), L2r (Firefox) | B0446, B0433–B0436, B0467 | Kein Skript im Ursprung; Firefox im Entwicklertest; `<img>`, `<video>`, „Öffnen“ in R00030 |
| M2 | Polyglot | L1, L2 (Chromium) | B0446, B0434, B0436 | Kein Skript im Ursprung |
| M3 | Art-Erkennung, Fuzz | L1 | B0433, B0434, B0435 | Kaputte Köpfe, Stückelung |
| M4 | Range | L1, L2 | B0444, B0446, B0447 | Teilabruf, Teilabruf über das Programm |
| M5 | Hochladen, Längen | L2 | B0468, B0429 | Angriffsmatrix Hochladen, Hochladen abgewiesen ohne Lesen |
| M6 | Langsam, 64 laufende | L1, L2 | B0428, B0468, B0425, B0427, B0429 | Fristen, Offene Hochladungen |
| M7 | Quote-Erschöpfung | L1, L2 | B0426, B0425, B0427 | Quote, Lebensende, Verfall |
| M8 | Abbruch und Aufräumen | L1, L2 | B0429, B0425, B0431, B0440 | Abbruch und Erneut, Start leert den Eingang, Nichts halb |
| M9 | fremde IDs | L2 | B0429, B0430, B0440 | Hochladen abgewiesen ohne Lesen, Verwerfen, Abweisungen |
| M10 | Name | L1, L2 | B0424, B0445, B0446 | Namensregel, Download unter dem Anzeigenamen |
| M11 | ID-Abfluss | L2 | B0464, B0432 | Keine ID, kein Name; Kennzahlen und ID gekürzt |
| M12 | Ursprung, CSRF | L2 | B0468, B0427, B0429, B0430, B0446 | Angriffsmatrix Hochladen, Grenzschutz |
| M13 | Direktgespräche | L2 | — | nicht in R00029; I0012 (WBS-Notiz) |
| M14 | CSP je Status | L2 | B0467, B0446 | Kopfzeilen überall |
| M15 | Skript-Gadget | L2 (Chromium) | B0446, B0445 | Kein Skript im Ursprung (Skript-Gadget) |
| M16 | Metazeile eingeschleust | L1, L2 | B0437, B0439, B0441 | Metazeile eingeschleust |
| M17 | Metazeile beim Laden | L1 | B0442, B0437 | Neustart (unplausible Metazeilen) |
| M18 | ID im Anfrageprotokoll | L2 | B0432, B0464 | Kennzahlen und ID gekürzt (verfehlte Routen) |
| M19 | ID und Name in Meldungen | L1, L2 | B0464, B0440, B0442, B0446, B0462 | Keine ID, kein Name |
| M20 | Reservierung halten | L1, L2 | B0425 (L1), B0468 (L2), B0428, B0429 | Lebensende |
| M21 | Eingang sperren | L2 | B0468, B0425, B0427 | Eingang nicht sperrbar |
| M22 | Halten sichtbar | L2 | B0463, B0426, B0427, B0432 | Episode Reservierung verfallen, Quote, Verfall |
| M23 | Wettläufe | L1, L2 | B0425, B0468, B0429, B0430, B0440 | Wettläufe |
| M24 | Schreibfehler | L1 | B0429, B0462 | Schreibfehler |
| M25 | `kill -9` | L2 | B0431, B0440, B0442 | Start leert den Eingang |
| M26 | `.eingang` als Symlink | L2 | B0431 | Eingang als Symlink |
| M27 | Maße-Täuschung | L1 | B0433 | Maße aus dem Segment |
| M28 | `Content-Disposition` | L1, L2 | B0445, B0446 | Download unter dem Anzeigenamen (Sonderzeichen) |
| M29 | Namens-Täuschung (optional) | L1 | B0436, B0424 | Name und Sender zählen nicht |
| M30 | Entfallen und Cache | L2 | B0446 | Nicht gefunden |
| M31 | Range erweitert | L1, L2 | B0444, B0446, B0447 | Teilabruf, Teilabruf über das Programm |
| M32 | Rohkörper | L2 | B0468, B0429 | Angriffsmatrix Hochladen |
| M33 | Methode und Pfad | L2 | B0468, B0446, B0429 | Angriffsmatrix Hochladen, Nicht gefunden |
| M34 | CSRF und Rebinding | L2 | B0468, B0427, B0429, B0430, B0446 | Angriffsmatrix Hochladen |
| M35–M37 | — | — | — | im Gate-Bericht nicht enthalten, Nummern frei |
| M38 | gebundene ID wiederverwenden | L2 | B0440, B0429, B0430 | Gebundene ID eines Dritten |
| M39 | `rename` überschreibt | L1, L2 | B0429, B0440 | Nie überschreiben |

## Abhängigkeiten

- Abhängig von: I0009 Nachricht senden (gelb; Senderoute, Strom, Verlauf — R00011 grün), I0023 Plattenquote (rot, Karte #34): F0075 Senden an der Quote abgewiesen (Zähler `belegt`, Quote-Prüfung) für F0097, F0076 Quote im Betriebsprotokoll (Episoden-Bereich `quote`) für F0103. I0021 Betriebsprotokoll (gelb; Protokoll, Episoden, Tagestakt — benutzt).
- Innerhalb: F0098 und F0099 brauchen F0097; F0100 braucht F0098 und F0099; F0103 braucht F0100.
- Blockiert: R00030 (F0101 braucht F0099, F0102 braucht F0100); I0012, I0015, I0016, I0019, I0022 nutzen Anhänge (ADR-0046, ADR-0047 „prägt“).

## Offene Fragen

An den Betreiber (stellt der Director; bis zur Antwort gilt der Standard der ADR, die Kriterien sind markiert):

1. **F1 Anteil je Herkunft** (noch unbeantwortet): darf ein Teilnehmer bzw. eine Herkunft den ganzen Platz für Anhänge belegen, oder höchstens einen Anteil (ADR-0046 Option K7)? Standard: ganzer Platz, nur offene Hochladungen und das Lebensende je Anmeldung begrenzt. Empfehlung des Directors aus dem Gate: keine Grenze für gebundene Anhänge (eine zweite IP genügt zum Umgehen, und B2 verlangt Dateien bis zur Größe des Platzes), Reservierungen je Herkunft höchstens 50 % des Platzes für Anhänge, nur im Speicher. Betrifft F0097 „Nur eine Herkunft füllt den Platz“; bei Annahme der Empfehlung kommt ein Wächter in B0426 und eine Zeile in die Matrix.
2. **F2 Direkter Medienlink**: darf ein direkt aufgerufener Medienlink sofort abspielen (Verhalten des Browsers), oder gilt „ohne Autoplay“ auch dort? Standard: hingenommen. Betrifft F0100 „Direkter Medienlink“.
3. **F3 Großer Text**: Text über 16 MiB nur als Download? Standard: ja. Betrifft F0098 „Text über 16 MiB“.
4. **F4 EXIF/GPS** (neu aus dem Gate, ADR-0047 Ergänzung Karte #9 Gate): Bilder und Videos werden unverändert ausgeliefert — EXIF-Daten wie GPS-Ort, Kameramodell und Aufnahmezeit gehen an jeden mit dem Link (B4). Sollen sie beim Hochladen entfernt werden, oder genügt ein Hinweis in der Oberfläche (R00030)? Standard: unverändert, wie B3 („Vorschau aus dem Original“). Betrifft F0100 „Bilder unverändert“; Entfernen hieße eine neue Bubble in F0098 (Umschreiben beim Hochladen) und ändert `bytes` nach der Anmeldung — eine Entscheidung für den architekt.

## Manuelle Vorbereitungstätigkeiten

- **Gate vor dem Bau** — `security-reviewer`: Matrix nach ADR-0047 Ergänzung Karte #9 Gate gegen die Bubbles prüfen, bevor F0097, F0098, F0099 und F0100 gebaut werden; Zuordnung Zeile → Linie → Bubble in der Tabelle „Angriffsmatrix: Zuordnung“ unter „Tests“; M13 erst mit I0012. Befund B1 des Gates (Lebensende, `offen_gesamt`) ist in ADR-0046 Ergänzung Karte #9 Gate entschieden und hier eingearbeitet; vor dem Bau von B0425, B0427 und B0429 bestätigt der `security-reviewer` die Angleichung (Proben M20, M21). Fällt eine Zeile von QS1, gilt ADR-0047 Option A2 (eigener Ursprung) als nächste Entscheidung; wird M15 rot, die Begrenzung von `script-src` (ADR-0047 Ergänzung Regel 6).
- Director: offene Fragen F1–F4 an den Betreiber (F2 und F3 bestehen seit der Planung, F1 ist unbeantwortet, F4 kam aus dem Gate).

## Manuelle Nachbereitungstätigkeiten

- Entwicklertest: Messwerte QA3, QS6, QA5 auf dem Zielrechner; PDF und WebM zusätzlich in Firefox (ADR-0047 „Messwert“; ohne Darstellung unter `sandbox` → Art auf `attachment` als Ergänzung der ADR).
- I0019 (Anleitung `lc`): drei Schritte, `anlauf_bis`, `gueltig_bis`, 507 nicht wiederholen, 429 nach `Retry-After`, 404 `abgelaufen` → neu anmelden.

## Umfang

Gezählt aus der WBS (2026-10-29, nach dem Gate angeglichen, nach den Regeln des Skills `work-breakdown-structure`):

- F0097 Anmelden und Hochladen über die API: 10 Bubbles · 6 klar (je 0,4 h Untergrenze, Richtwert, ungemessen) · 4 unklar (je 2–4 h) — zusammen 10,4–18,4 h.
- F0098 Art der Datei erkennen: 4 Bubbles · 2 klar · 2 unklar — 4,8–8,8 h.
- F0099 Senden mit Anhängen: 7 Bubbles · 6 klar · 1 unklar — 4,4–6,4 h.
- F0100 Ausliefern mit Art und Teilabruf: 5 Bubbles · 4 klar · 1 unklar — 3,6–5,6 h.
- F0103 Anhänge im Betrieb: 5 Bubbles · 3 klar · 2 unklar — 5,2–9,2 h.
- Zusammen 31 Bubbles · 21 klar · 10 unklar — 28,4–48,4 h (vor dem Gate 29 Bubbles, 26–44 h; neu B0467, B0468). Die Angleichung der 17 Bubbles hat keine Klasse geändert; ihr Mehrinhalt liegt in den Spannen der unklaren (B0425, B0429, B0440) — Hypothese, nicht gemessen.

## Warum löst diese Anforderung das Problem? (Pflicht)

Auslöser: die Vision verspricht Anhänge, und Agenten auf verschiedenen Rechnern haben heute keinen Weg, eine Datei zu teilen. Der Betreiber hat dafür „fast unbegrenzt, aber non-blocking und ohne die Quote zu verletzen“ verlangt (B2).

- Die Anmeldung reserviert den Platz, bevor ein Byte unterwegs ist. Deshalb bekommt der Sender „kein Platz“ als lesbare Antwort statt eines Verbindungsabbruchs, und die Quote kann durch Hochladen nie überschritten werden.
- Die Textreserve von 10 % gehört nur Sätzen. Deshalb bleibt der Chat bedienbar, auch wenn jemand den Platz für Anhänge füllt.
- Hochladen läuft ohne Sperre, in einer eigenen Bank von 8 Threads und stückweise. Deshalb wartet keine Textnachricht auf eine Gigabyte-Datei.
- Die Datei wandert erst beim Senden per `rename` in den Tag ihrer Nachricht. Deshalb gibt es nie einen Satz ohne Datei, und die Anhänge entfallen mit derselben Frist wie der Text.
- Die Art bestimmt der Server, und jede Auslieferung läuft unter `sandbox`. Deshalb kann eine hochgeladene HTML- oder SVG-Datei das Token eines Lesers nicht stehlen, obwohl der Link ohne Sitzung öffnet.

## Missing-Docs

- Glossar fehlt (`Dokumentation/Domaenenwissen/` existiert nicht); Anhang, Anmeldung, Eingang, Blockmaß, Platz für Anhänge hier eingeführt.
- Verhalten von Firefox bei `sandbox`-CSP auf PDF und Medien ist nicht geprobt (ADR-0047).

## Notizen

- Verworfene Alternativen zum Schnitt: B (Art erkennen in F0097) — 13 Bubbles in einem Slice; C (Medien quer) — Teilabruf und Player gehören zu verschiedenen Slices und Anforderungen.
- Nicht hier: Oberfläche (R00030), Direktgespräche (I0012), Archiv (I0022), Anleitung für Agenten (I0019), wiederaufnehmbares Hochladen (ADR-0046 Option U7, Folgeoption bei beobachteten Abbrüchen).
- 2026-10-29 nach dem Gate des `security-reviewer` (Kommentar Karte #9, Befund B1 und Matrix M14–M39) angeglichen (planer, `/anforderung review` ohne Dialog): Lebensende je Anmeldung, `offen_gesamt` 64 laufende, eine Sperre, `RENAME_NOREPLACE`, `.eingang` Exit 2, Schreibfehler, ID-Kürzung nach dem Dekodieren, CSP-Weiche nach Pfad, Maße aus dem Segment, Datei nur als Präfix, Existenz vor ETag, Range-Ränder; Kriterien „Lebensende“, „Eingang nicht sperrbar“, „Eingang als Symlink“, „Wettläufe“, „Schreibfehler“, „Nie überschreiben“, „Angriffsmatrix Hochladen“, „Maße aus dem Segment“, „Gebundene ID eines Dritten“, „Metazeile eingeschleust“, „Bilder unverändert“, „Episode Reservierung verfallen“, „Angriffsmatrix (alle Slices)“ neu; Zuordnungstabelle unter „Tests“. Verworfen: die L2-Proben des Hochladens auf B0427, B0429, B0430 zu verteilen — sie teilen ein Geschirr (mehrere Loopback-Herkünfte, Rohclient, Kontrollpaar), das sonst dreimal entstünde, und B0429 trüge allein zehn Zeilen; der Preis ist eine zehnte Bubble in F0097.
