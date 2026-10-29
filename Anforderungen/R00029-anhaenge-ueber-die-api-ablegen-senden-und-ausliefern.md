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

- `POST /api/anhaenge` mit Sitzung und JSON `{name, bytes}` (Körper höchstens 2.048 Bytes) meldet eine Datei an. Prüfreihenfolge und Antworten nach ADR-0046 Regel 1: 403 · 401 · 429 (1 Einheit vorab am Schreibkonto) · Körperfehler 415/400/413/408 · 422 `feld` · 422 `bytes` (keine ganze Zahl in u64) · 422 `leer` (0) · 422 `name` mit `grund` · 429 `offen_kennung` \| `offen_herkunft` \| `offen_gesamt` mit `hoechstens` (4, 16, 32) und `Retry-After: 5` · 507 `platte` mit `frei` · 507 `quote` mit `belegt`, `hoechstens`, `frei_fuer_anhaenge` · 201 `{anhang: {id, name, bytes, hochladen, anlauf_bis}}`.
- Namensregel nach ADR-0046 Regel 4: Anzeigename 1–255 Bytes; abgewiesen werden die verbotenen Zeichen aus ADR-0034 Regel 3.6, Tab, CR, LF, Richtungszeichen U+200E, U+200F, U+202A–U+202E, U+2066–U+2069, `/`, `\`, `"` und die Namen `.` und `..`. Gespeichert wird `<hhmmssmmm>-<n>-<ascii>`; der Anzeigename bleibt unverändert in Metazeile, API und `Content-Disposition`.
- Es gibt **keine Grenze je Datei** außer dem Platz für Anhänge und der Platte (Betreiber B2). Platte: `bytes + reserviert + 1 GiB > frei` → 507 `platte`, auch bei `--plattenquote aus`. Quote: `gebunden + reserviert + blockmass(bytes) > hoechstens − hoechstens / 10` → 507 `quote` mit dem Text „Nicht hochgeladen: der Platz für Anhänge ist voll — noch <n> frei; Text kann weiter gesendet werden.“ (Größen wie ADR-0044 Regel 4). Sonst wird `blockmass(bytes)` reserviert, bevor ein Byte geschrieben ist.
- `PUT /api/anhaenge/{id}` mit Sitzung, `Content-Type: application/octet-stream` und `Content-Length` = angemeldete Länge lädt hoch. Vor dem ersten gelesenen Byte: 404 `unbekannt` \| `abgelaufen` (eine fremde ID erscheint als `unbekannt`), 409 `laeuft` \| `fertig`, 415, 411 (ohne `Content-Length`, auch `chunked`), 422 `laenge` mit `erwartet` — jeweils mit `Connection: close`. Danach gestreamt in `.eingang/<id>.teil`, dann `rename`; Erfolg 200 `{anhang: {id, name, bytes, art, breite, hoehe, gueltig_bis}}`.
- **Fristen**: kommen 10 s lang keine weiteren 64 KiB (bzw. der Rest) an → 408; Gesamtfrist 60 s + Länge / 1 MiB/s → 408 (Text in der Form der Ergänzung Karte #26 zu ADR-0010). Jeder Abbruch (Frist, Lesefehler, Client weg, zu wenig Bytes, Ende der Instanz) löscht die Teildatei; die Anmeldung gilt wieder 60 s, ein neuer `PUT` auf dieselbe ID ist „Erneut“.
- `DELETE /api/anhaenge/{id}` → immer 204; für die eigene ID: Teil- oder fertige Datei weg, Reservierung sofort frei, ein laufender `PUT` endet als Abbruch; fremde und unbekannte IDs bleiben unberührt.
- Lebensdauer: eine Anmeldung ohne laufendes Hochladen verfällt 60 s nach der Anmeldung bzw. nach dem letzten Abbruch, eine fertige Datei 15 min nach dem Ende des Hochladens; der Start leert `.eingang/` vor dem Laden (nur reguläre Dateien; Symlinks und Unterordner bleiben und werden als `WARNING laden` gemeldet).
- **Nicht blockierend** (B2): Hochladen hält keine Gesprächs- oder Schreibsperre, belegt nie die Werkbank, höchstens 8 Blocking-Threads (Hochladebank) und je Hochladung höchstens ein Stück (≤ 1 MiB) im Speicher.
- `/api/gesundheit` trägt `belegt` (gebunden + reserviert), `reserviert`, `hochladungen` (offen) und `hochladebank_belegt`. Das Anfrageprotokoll schreibt Pfade unter `/api/anhaenge/` und `/anhaenge/` nur mit den ersten 6 Zeichen der ID.

### Art der Datei erkennen (F0098)

- Die Art kommt aus dem Inhalt nach der Tabelle in ADR-0047 Frage 3 (Signaturen aus den ersten 64 KiB, dann die Text-Prüfung über die ganze Datei): PNG, JPEG, GIF, WebP (mit Breite und Höhe) → `image/*`; `%PDF-` → `application/pdf`; MP4 (`ftyp`; `M4A `/`M4B ` → `audio/mp4`, `qt  ` → keine) → `video/mp4`; WebM (`DocType` `webm`; `matroska` → keine) → `video/webm`; Ogg (Opus, Vorbis → `audio/ogg`; Theora → `video/ogg`); MP3 (`ID3` v2–4 oder zwei gültige Rahmenköpfe) → `audio/mpeg`; gültiges UTF-8 ohne NUL → `text/plain; charset=utf-8`; alles andere → `application/octet-stream`.
- Disposition: `inline` für alle Arten der Tabelle außer Text über 16 MiB und `application/octet-stream` — diese `attachment` (nach Betreiber F3).
- SVG, HTML und XML sind Text und werden nie als Dokument dargestellt.
- Bis F0098 gebaut ist, ist jede Datei `application/octet-stream` mit `attachment`.

### Senden mit Anhängen (F0099)

- `POST /api/channels/{name}/nachrichten` nimmt `{text, anhaenge?: [id, …]}`: 1–10 IDs, keine doppelt. Text leer oder nur Leerraum ist erlaubt, wenn mindestens ein Anhang dabei ist (B5), und wird als `""` gespeichert. Abweisungen nach ADR-0034 Regel 3.6: 422 `{regel: "anhang", id, grund}` mit `unbekannt` \| `abgelaufen` \| `fremd` \| `doppelt` \| `unvollstaendig`; 422 `{regel: "anhaenge", hoechstens: 10}`; ohne Text und ohne Anhang 422 `leer` wie bisher.
- Beim Senden wandert jede Datei per `rename` nach `channels/<name>/anhaenge/<tag der nachricht>/<hhmmssmmm>-<n>-<ascii>`; erst danach wird der Satz mit je einer Metazeile `+ anhang: <id> <bytes> <art> <b>x<h>|- <datei> <anzeigename>` angehängt (ADR-0046 Regel 7). Scheitert das, wandern die Dateien zurück in den Eingang, der Satz ist zurückgebaut, die Antwort 500, nichts erscheint im Verlauf oder Strom, und die Anhänge bleiben bis zu ihrer Frist sendbar.
- Der Wiederholschlüssel gilt für Text **und** Anhang-IDs; Kosten am Schreibkonto bleiben ⌈Textbytes / 4.096⌉, mindestens 1.
- Jede `nachricht` — Senden-Antwort, Verlauf, Ereignis im Strom — trägt `anhaenge: [{id, name, bytes, art, breite, hoehe, url}]`: immer eine Liste, in Sendefolge, `breite`/`hoehe` nur bei Bildern (sonst `null`), `url` = `/anhaenge/<id>`.
- Quote: der Satz prüft nur noch sich selbst gegen die volle Quote (die Anhänge sind seit der Anmeldung reserviert); beim Senden wechseln die Anhangbytes von `reserviert` nach `gebunden`. Beim Laden zählt jede reguläre Datei in `anhaenge/<tag>/` der Tage im Index im Blockmaß an `gebunden`, auch eine Waise. Entfällt ein Tag (Start, Tageswechsel, Verkürzen der Frist), entfällt `anhaenge/<tag>/` nach der Tagesdatei — Symlinks werden nie verfolgt — samt seinen Bytes.

### Ausliefern mit Art und Teilabruf (F0100)

- `GET` und `HEAD /anhaenge/{id}` ohne Sitzung, unter dem Grenzschutz (fremder Host 403). ID nicht in der Form oder unbekannt oder ihr Tag entfallen → 404 `text/plain` „Diesen Anhang gibt es nicht (mehr).“; fehlt die Datei auf der Platte → derselbe 404 und `WARNING anhang` mit Gespräch, Tag und Datei, nie der ID.
- Jede Antwort der Route (200, 206, 304, 404, 416, `HEAD`) trägt `Content-Security-Policy: sandbox; default-src 'none'`, `X-Content-Type-Options: nosniff`, `Cross-Origin-Resource-Policy: same-origin`, `Referrer-Policy: no-referrer`, `X-Frame-Options: DENY`; Dateiantworten zusätzlich `Content-Type` und `Content-Disposition: <inline|attachment>; filename="<ascii-ersatz>"; filename*=UTF-8''<anzeigename prozentkodiert>`, `Accept-Ranges: bytes`, `Content-Length`, `Cache-Control: private, no-cache`, `ETag: "<id>"`.
- Teilabruf nach ADR-0047 Regel 4 und Frage 6: genau ein Bereich `bytes=a-b`, `a-` oder `-n` → 206 mit `Content-Range`; nicht erfüllbar (`a` ≥ Länge, `-0`) → 416 mit `Content-Range: bytes */<länge>`; mehrere Bereiche oder unlesbare Angabe → 200 ganz; `If-Range` mit dem ETag → Bereich, sonst 200 ganz; `If-None-Match: "<id>"` → 304, geprüft vor `Range`.
- Gestreamt in Stücken zu 64 KiB mit Gegendruck; die Sendefrist ist die Abrissgrenze; eine Datei, deren Tag während des Sendens entfällt, wird zu Ende gesendet.
- Eine mitgeschickte Sitzung wird nicht geprüft und verlängert nichts.

### Anhänge im Betrieb (F0103)

- Ereignisse nach ADR-0042 Ergänzung Karte #9: `DEBUG anhang` je Anmeldung, Hochladen-Ende, Abbruch, Ablauf und Abruf; `INFORMATION anhang Großes Hochladen abgeschlossen` ab 64 MiB mit `bytes`, `dauer_ms`, `mib_s`; Episoden `WARNING anhang Hochladen zu langsam abgebrochen` je Herkunft, `WARNING quote Anhänge abgewiesen: Platz für Anhänge voll` je Kennung, `WARNING quote Anhang abgewiesen: Platte fast voll` je Herkunft; `abgewiesen.zu_langsam` zählt die 408 des `PUT`.
- Nie im Protokoll: Anhang-ID (auch nicht im Anfrageprotokoll), Anzeigename, Inhalt.
- Messwerte QA3, QS6, QA5 werden gemessen und mit Rechner und Plattenart berichtet.

## Nicht-funktionale Anforderungen

- **Nicht blockierend (QA3, nach ADR-0046; Zweck: Lastprofil „Team-LAN“, Zustellung < 1 s)**: während zwei Teilnehmer gleichzeitig je 1 GiB hochladen, antwortet `GET /api/gesundheit` mit p90 ≤ 0,15 s, Senden einer Textnachricht mit p90 ≤ 0,25 s, und die Nachricht erscheint im Strom < 1 s nach der 201; ein dritter Teilnehmer lädt 1 MiB in der Zeit hoch, die er ohne die beiden braucht (± 50 %); RSS wächst höchstens um 2 × 1 MiB + 50 MB. Gemessen im Release-Build auf dem Zielrechner (B0465), nicht im Gate; Vergleichslauf ohne Hochladungen. Uhr der Zustellung: ab der 201 des Sendens.
- **Langsame Leser (QS4)**: 160 Verbindungen einer Herkunft, die je einen großen Anhang anfordern und nicht lesen, lassen RSS um höchstens 160 × 64 KiB + 50 MB wachsen (≈ 60 MB), unabhängig von der Dateigröße.
- **Spulen (QS6)**: Sprung ans Ende eines 2-GiB-Videos → Wiedergabe setzt im LAN < 1 s später dort ein, ohne die Datei bis dahin zu laden (B0466).
- **Laden (QA5)**: ein Ordner mit 1.000 Anhängen startet ≤ 50 ms langsamer als ohne — gerechnet in ADR-0046, hier gemessen (B0466).
- **Sicherheit**: kein Skript aus einem Anhang erreicht `localStorage` oder die API der App (QS1); Angriffsmatrix ADR-0047 M1–M12 als Gate (siehe „Manuelle Vorbereitungstätigkeiten“).
- **Zeitkriterien** nach `AI-Team/planer.md`: Regeln (60 s, 15 min, Stück- und Gesamtfrist) exakt im Unit-Test mit gestellter Uhr; an der Grenze des echten Programms nur obere Grenzen mit Startpunkt und 30 % Spiel.

## Akzeptanzkriterien

Gemeinsam für alle Kriterien, wenn nicht anders genannt: Start mit `TZ=Europe/Berlin localchatA <ordner> --port 0 --host 127.0.0.1 --kein-browser --plattenquote 10MiB` auf einem Wegwerf-Ordner mit dem leeren Ordner `channels/allgemein/`; `<p>` ist der Port aus der Startzeile. Quote 10.485.760 Bytes, Textreserve 1.048.576, **Platz für Anhänge 9.437.184 Bytes** minus `belegt`. `S(<name>)` ist der Sitzungs-Header aus dem Beitritt per `curl` (Agent, Herkunft `127.0.0.1`). „Anmelden n“ heißt `POST /api/anhaenge` mit `{"name":"bericht.pdf","bytes":n}` und `S(beta)`; „Hochladen“ heißt `curl -X PUT --data-binary @<datei> -H 'Content-Type: application/octet-stream'` mit `S(beta)` auf `hochladen` aus der 201; „Senden“ heißt `POST /api/channels/allgemein/nachrichten` mit `S(beta)`. Testdateien entstehen im Test (Bytes im Code oder `ffmpeg` für Medien), nicht im Repository.

### F0097 Anmelden und Hochladen über die API

- [ ] **Hauptweg** (Integration): Anmelden 307.200 → 201 mit 26-Zeichen-`id` aus `[a-z2-7]`, `hochladen` = `/api/anhaenge/<id>`, `anlauf_bis`; `/api/gesundheit` zeigt `reserviert` 307.200, `hochladungen` 1, `belegt` 307.200. Hochladen von 307.200 Bytes → 200 mit `gueltig_bis` = Zeitpunkt der Antwort + 15 min (± 1 s); `.eingang/<id>` hat 307.200 Bytes, kein `.teil` liegt im Eingang.
- [ ] **Quote, Rechenbeispiel** (Integration): Anmelden 9.437.184 → 201 (0 + 0 + 9.437.184 ≤ 9.437.184). Anmelden 1 → 507 `{regel: "quote", belegt: 9437184, hoechstens: 10485760, frei_fuer_anhaenge: 0}` mit dem Text „Nicht hochgeladen: der Platz für Anhänge ist voll — noch 0,0 MiB frei; Text kann weiter gesendet werden.“ (ADR-0046 Regel 1.6; Blockmaß von 1 Byte ist 4.096); `reserviert` bleibt 9.437.184. **Text geht weiter**: Senden `{"text":"Hallo"}` → 201. `DELETE` der ersten ID → 204, `reserviert` 0; Anmelden 1 → 201, `reserviert` 4.096. Mutanten „gegen `hoechstens` statt `hoechstens − reserve`“, „ohne Blockmaß“ und „`>=` statt `>`“ werden rot.
- [ ] **Platte** (Integration, auch mit `--plattenquote aus`): Anmelden 1.125.899.906.842.624 (1 PiB) → 507 `{regel: "platte", frei}` mit `frei` > 0, nichts reserviert. Die Regel `bytes + reserviert + 1 GiB > frei` prüft der Unit-Test mit gestellter Plattenauskunft (frei = Länge + 1 GiB − 1 → `platte`, frei = Länge + 1 GiB → ok).
- [ ] **Nur eine Herkunft füllt den Platz** (nach Betreiber F1): mit dem Standard der ADR darf eine Herkunft den ganzen Platz für Anhänge reservieren; es gibt keine Grenze je Herkunft außer den offenen Hochladungen. Ändert der Betreiber das (Option K7), wird dieses Kriterium ersetzt.
- [ ] **Abweisungen der Anmeldung** (Integration): ohne Sitzung 401; `Origin: http://evil.example` 403; `{"name":"a","bytes":1,"x":1}` → 422 `feld`; `bytes` `-1`, `1.5`, `"1"`, 18.446.744.073.709.551.616 → 422 `bytes`; `bytes` 0 → 422 `leer`; Körper über 2.048 Bytes → 413; jede Antwort JSON, keine Reservierung.
- [ ] **Namensregel** (Unit, rein): `Bericht März.pdf` → gespeichert `…-Bericht_M_rz.pdf`, Anzeigename unverändert; `../x`, `a/b`, `a\b`, `a"b`, `.`, `..`, leer, 256 Bytes, `rechnung` + U+202E + `fdp.exe`, `a\tb`, `a\nb`, `\u001b[31m` → 422 `name` mit `grund`; 255 Bytes → ok; `x.tar.gz` behält `.gz`, eine Endung über 16 Zeichen entfällt; ein Name nur aus Umlauten ergibt nie einen leeren gespeicherten Namen (`datei`). Über das Programm: Anmelden mit `"name":"../x"` → 422 `name`.
- [ ] **Offene Hochladungen** (Integration): `beta` meldet viermal 1 Byte an, die fünfte → 429 `{grenze: "offen_kennung", hoechstens: 4}` mit `Retry-After: 5`. Vier Kennungen derselben Herkunft mit je vier → die erste Anmeldung einer fünften Kennung → 429 `offen_herkunft`, `hoechstens: 16`. `offen_gesamt` (32) prüft der Unit-Test.
- [ ] **Hochladen abgewiesen ohne Lesen** (Integration): `Content-Length` 307.199 bei angemeldeten 307.200 → 422 `{regel: "laenge", erwartet: 307200}` mit `Connection: close`; ohne `Content-Length` (`Transfer-Encoding: chunked`) → 411; `Content-Type: text/plain` → 415; zweiter `PUT` nach Erfolg → 409 `fertig`; `PUT` einer ID von `gamma` mit `S(beta)` → 404 `unbekannt`; ausgedachte ID → 404 `unbekannt`. In keinem Fall liegt danach eine `.teil`-Datei im Eingang.
- [ ] **Abbruch und Erneut** (Integration): `PUT` mit `Content-Length` 307.200, nach 100 KiB Verbindung geschlossen → kein `.teil`, `reserviert` unverändert 307.200; ein neuer `PUT` auf dieselbe ID mit allen Bytes → 200.
- [ ] **Fristen** (Unit, rein, `tokio::time::pause`): 64 KiB, dann 9,999 s Stille → läuft; 10,000 s → `stueckfrist`; 1 Byte je 9 s → `stueckfrist` nach 10 s ohne 64 KiB; Gesamtfrist für 10 MiB = 70 s; 10 MiB gleichmäßig mit 1 MiB/s → gelesen; mit 0,9 MiB/s → `gesamtfrist` bei 70 s. Über das Programm: Anmelden 1 MiB, `PUT` mit 64 KiB, dann Stille → 408 spätestens 13 s nach dem letzten Byte (10 s + 30 %), kein `.teil`.
- [ ] **Verfall** (Unit mit gestellter Uhr; Integration als obere Grenze): Anmeldung ohne `PUT`, Uhr + 60 s → noch reserviert, + 60 s + 1 ms und ein Takt → frei; fertige Datei, Uhr + 15 min + 1 s → Eingang leer, frei. Über das Programm: Anmelden 1 → `reserviert` 4.096 bei 30 s nach der 201 und 0 spätestens 80 s nach der 201 (60 s Frist, 1 s Takt, 30 % Spiel).
- [ ] **Verwerfen** (Integration): `DELETE` der eigenen ID während eines laufenden `PUT` → 204, der `PUT` endet ohne 200, kein `.teil`, `reserviert` 0 sofort (nächste Abfrage der Gesundheit); `DELETE` einer ID von `gamma` mit `S(beta)` → 204, `gamma` kann danach hochladen; `DELETE` einer unbekannten ID → 204.
- [ ] **Start leert den Eingang** (Integration): Instanz beenden, während eine Datei fertig und eine angemeldet ist; Neustart → `.eingang/` enthält keine reguläre Datei, `reserviert` 0, `hochladungen` 0. Ein Symlink und ein Unterordner in `.eingang/` bleiben, je eine `WARNING laden` steht im Protokoll; das Ziel des Symlinks bleibt unberührt.
- [ ] **Nicht blockierend, schnell geprüft** (Integration, Debug-Build): während ein `PUT` über 64 MiB mit 4 MiB/s läuft, antwortet Senden einer Textnachricht 201, und ein zweiter Teilnehmer meldet an und lädt 1 KiB hoch (200). Die Leistungswerte misst F0103.
- [ ] **Kennzahlen und ID gekürzt** (Integration): `/api/gesundheit` hat die Felder `belegt`, `reserviert`, `hochladungen`, `hochladebank_belegt` (0 im Ruhezustand). Mit `--protokoll-stufe debug`: nach Anmelden und Hochladen enthält `logs/<heute>.log` die ID nicht (`grep <id>` = 0), wohl aber ihre ersten 6 Zeichen im gekürzten Pfad.

### F0098 Art der Datei erkennen

- [ ] **Tabelle** (Unit, rein; Tabelle in ADR-0047 Frage 3): PNG-Kopf mit `IHDR` 1280 × 800 → `image/png`, inline, 1280 × 800; JPEG, GIF, WebP (`VP8 `, `VP8L`, `VP8X`) je mit ihren Maßen; `%PDF-1.7` → `application/pdf`; `….ftypisom` → `video/mp4`; `….ftypM4A ` → `audio/mp4`; `….ftypqt  ` → `application/octet-stream`; EBML mit `DocType` `webm` → `video/webm`, `matroska` → `application/octet-stream`; `OggS` + `OpusHead` → `audio/ogg`, + `\x80theora` → `video/ogg`; `ID3\x04` → `audio/mpeg`; ein einzelner MPEG-Rahmenkopf ohne zweiten → kein MP3; `<svg …><script>` → `text/plain; charset=utf-8`, inline; dasselbe mit NUL-Byte → `application/octet-stream`, `attachment`.
- [ ] **Text über 16 MiB** (Unit; nach Betreiber F3): 16.777.216 Bytes `a` → Text, inline; 16.777.217 → Text, `attachment`.
- [ ] **Stückelung** (Unit): dieselbe Datei in Stücken zu 1, 3 und 65.536 Bytes → gleiches Urteil, auch mit `ä` (2 Bytes) und `😀` (4 Bytes) über einer Stückgrenze und mit ungültigem UTF-8 genau an der Grenze.
- [ ] **Kaputte Köpfe** (Unit): jede Signatur der Tabelle abgeschnitten nach 1, 4, 8, 16 und 32 Bytes, Längenfelder 0 und 2⁶⁴ − 1, MPEG-Rahmenlänge 0 → nie Panik, keine Schleife, höchstens 64 KiB für Signaturen gelesen; Ergebnis Text oder `application/octet-stream`.
- [ ] **Name und Sender zählen nicht** (Unit): HTML-Inhalt mit Anzeigename `bild.png` → Text; PNG-Inhalt mit Name `x.txt` → `image/png`. Mutanten „Endung entscheidet“, „SVG inline als Bild“, „Text ohne UTF-8-Prüfung“, „Text über 16 MiB inline“, „MP3 ohne zweiten Rahmenkopf“ werden rot.
- [ ] **Über das Programm** (Integration): je eine Datei jeder Zeile der Tabelle anmelden und hochladen → die 200 nennt `art`, `breite`, `hoehe` wie die Tabelle (`breite`/`hoehe` `null` außer bei Bildern).

### F0099 Senden mit Anhängen

- [ ] **Nur Anhang, ohne Text** (Integration; B5): 307.200 Bytes anmelden und hochladen; Senden `{"text":"","anhaenge":["<id>"]}` → 201 mit `text: ""` und `anhaenge: [{id, name: "bericht.pdf", bytes: 307200, art, breite: null, hoehe: null, url: "/anhaenge/<id>"}]`; `channels/allgemein/anhaenge/<heute>/` enthält genau eine Datei `<hhmmssmmm>-1-bericht.pdf` mit 307.200 Bytes, `.eingang/` ist leer; der Satz in `channels/allgemein/<heute>.log` trägt `+ anhang: <id> 307200 <art> - <datei> bericht.pdf`; `alpha` liest im Strom dieselbe Liste; `GET /api/channels/allgemein/nachrichten` zeigt sie ebenso.
- [ ] **Zähler beim Senden** (Integration): vor dem Senden `belegt` = `reserviert` = 307.200 (Blockmaß 75 × 4.096); danach `reserviert` 0 und `belegt` = 307.200 + Bytes des Satzes. Ein Text mit drei Anhängen zu 1, 4.096 und 4.097 Bytes zählt 4.096 + 4.096 + 8.192 = 16.384 Anhangbytes.
- [ ] **Abweisungen** (Integration): dieselbe ID ein zweites Mal senden → 422 `{regel: "anhang", grund: "unbekannt"}`; dieselbe ID zweimal in einer Liste → 422 `doppelt`; eine angemeldete, nicht hochgeladene ID → 422 `unvollstaendig`; eine ID von `gamma` → 422 `fremd`; 11 IDs → 422 `{regel: "anhaenge", hoechstens: 10}`; `{"text":""}` und `{"text":"  ","anhaenge":[]}` → 422 `leer`; `{"text":"x","anhaenge":["…"],"y":1}` → 422 `feld`. In keinem Fall ändert sich die Tagesdatei.
- [ ] **Text mit zehn Anhängen** (Integration): Text `Hier die Belege` mit 10 Anhängen → 201; die Liste hat 10 Einträge in der Reihenfolge des Körpers, die Tagesdatei 10 Metazeilen, `anhaenge/<heute>/` 10 Dateien mit `-1-` bis `-10-` im Namen.
- [ ] **Wiederholschlüssel** (Integration): zweimal derselbe Körper mit demselben Schlüssel → zweimal 201 mit derselben Zeit, ein Satz; derselbe Schlüssel mit anderem Text oder anderer ID → wie ein neuer Text (ADR-0034 Ergänzung Karte #9).
- [ ] **Nichts halb** (Unit mit Attrappe): scheitert das zweite `rename` oder das Anhängen des Satzes → beide Dateien wieder im Eingang, kein Satz, nichts im Index, kein Ereignis, `reserviert` unverändert, die IDs sind danach sendbar; scheitert das Zurücklegen → die Datei bleibt als Waise im Tag, `ERROR senden` mit Datei und Grund. Invariante `gebunden = Σ Verlauf + Σ Anhangbytes je Tag`, `reserviert = Σ Eingang` nach jedem Schritt (erster Unit-Test aus ADR-0046 mit Quote 100.000).
- [ ] **Neustart** (Integration): nach dem Kriterium „Nur Anhang“ Instanz neu starten → der Verlauf liefert dieselbe `anhaenge`-Liste, `belegt` = Bytes der Tagesdatei + 307.200; eine Datei in `anhaenge/<heute>/` ohne Metazeile (Waise) zählt mit, ein Symlink dort zählt nicht. Eine unlesbare Metazeile (`+ anhang: zu kurz`) macht den Satz nicht defekt, der Anhang fehlt in der Liste, `WARNING laden` nennt Datei und Zeile.
- [ ] **Entfallen** (Integration mit `LOCALCHATA_JETZT`): eine Nachricht mit Anhang von vor 7 Tagen (Frist 7) → beim Start entfallen Tagesdatei und `anhaenge/<tag>/`, `belegt` zählt beides nicht mehr; ein Symlink in `anhaenge/<tag>/` auf eine Datei außerhalb → der Symlink ist weg, sein Ziel bleibt. Gleiches beim Tageswechsel und beim Verkürzen der Frist (`PUT /api/channels/allgemein/frist`). Mutante „Entfallen vergisst Anhangbytes“ wird rot.

### F0100 Ausliefern mit Art und Teilabruf

- [ ] **Öffnen ohne Sitzung** (Integration): `x.html` mit `<script>localStorage.t</script>` hochladen, senden, `GET /anhaenge/<id>` ohne Sitzung → 200, `Content-Type: text/plain; charset=utf-8`, `Content-Disposition: inline; filename="x.html"; filename*=UTF-8''x.html`, CSP genau `sandbox; default-src 'none'`, `nosniff`, Körper byte-gleich mit der Datei.
- [ ] **Download unter dem Anzeigenamen** (Integration): eine Datei mit NUL-Bytes, Anzeigename `Bericht März.bin` → `application/octet-stream`, `Content-Disposition: attachment; filename="Bericht_M_rz.bin"; filename*=UTF-8''Bericht%20M%C3%A4rz.bin`.
- [ ] **Kopfzeilen überall** (Integration): Antworten 200, 206, 304, 404 und 416 sowie `HEAD` tragen alle die CSP `sandbox; default-src 'none'`, `nosniff`, `Cross-Origin-Resource-Policy: same-origin`, `Referrer-Policy: no-referrer`, `X-Frame-Options: DENY`; 200 und 206 dazu `Accept-Ranges: bytes`, `ETag: "<id>"`, `Cache-Control: private, no-cache`. Mutationsprobe je Kopfzeile: jede einzeln entfernt → ein Test rot.
- [ ] **Teilabruf** (Unit, rein; ADR-0047 „Durchsetzung“): Länge 1.000: `bytes=0-0` → Teil(0, 0); `0-` → Teil(0, 999); `-1` → Teil(999, 999); `-0` → unerfüllbar; `5-2` → Ganz; `1000-` → unerfüllbar; `0-1010` → Teil(0, 999); `18446744073709551615-` → unerfüllbar; `18446744073709551616-` → Ganz; `0-1,2-3` → Ganz; `items=0-1` → Ganz; ` bytes=0-1` → Ganz.
- [ ] **Teilabruf über das Programm** (Integration): WebM mit n Bytes: `Range: bytes=100-199` → 206, `Content-Range: bytes 100-199/<n>`, `Content-Length: 100`, die 100 Bytes gleich der Datei; `bytes=<n>-` → 416, `Content-Range: bytes */<n>`; `bytes=0-1,5-6` → 200 ganz; `If-Range: "<id>"` mit `bytes=0-9` → 206, `If-Range: "anders"` → 200 ganz; `If-None-Match: "<id>"` → 304 ohne Körper, auch mit `Range`.
- [ ] **Nicht gefunden** (Integration): `GET /anhaenge/abc` → 404 „Diesen Anhang gibt es nicht (mehr).“; ausgedachte gültige ID → derselbe 404; Datei auf der Platte gelöscht → derselbe 404 und `WARNING anhang` mit Gespräch, Tag und Datei, ohne die ID; nach dem Entfallen des Tags (`LOCALCHATA_JETZT` + 7 Tage, Neustart) → 404.
- [ ] **Grenzschutz** (Integration): `Host: evil.example` → 403; eine mitgeschickte Sitzung ändert die Antwort nicht und verlängert die Sitzung nicht (Restzeit unverändert).
- [ ] **Kein Skript im Ursprung** (E2E, Chromium; QS1): im Browser A, beigetreten und mit `localStorage`-Token, `page.goto('/anhaenge/<id>')` je für HTML mit `<script>`, SVG mit `<script>`, PNG-Kopf + HTML (Polyglot) und WebM + angehängtes HTML (Polyglot) → kein Skript läuft (ein Skript, das `document.title` setzen würde, setzt ihn nicht), HTML und SVG erscheinen als Quelltext, das Token in `localStorage` der App bleibt unverändert.
- [ ] **Direkter Medienlink** (nach Betreiber F2): `page.goto('/anhaenge/<id>')` auf eine WebM-Datei öffnet das Medien-Dokument des Browsers, das selbst abspielt — mit dem Standard der ADR hingenommen. Verlangt der Betreiber „nie Autoplay“, gilt ADR-0047 Pre-Mortem 4 (M3 mit `Sec-Fetch-Dest`) und dieses Kriterium wird ersetzt.
- [ ] **Langsame Leser** (Integration; QS4): 160 Verbindungen einer Herkunft fordern je eine Datei von 64 MiB an und lesen nicht → RSS der Instanz wächst um höchstens 160 × 64 KiB + 50 MB; jede Verbindung ist nach der Abrissgrenze zu.

### F0103 Anhänge im Betrieb

- [ ] **Großes Hochladen** (Integration): eine Datei von 64 MiB hochladen → genau eine Zeile `INFORMATION anhang Großes Hochladen abgeschlossen` mit `bytes=67108864`, `dauer_ms`, `mib_s`; bei 64 MiB − 1 Byte keine solche Zeile.
- [ ] **Episoden** (Integration): drei 408 des `PUT` aus derselben Herkunft → eine Episode `WARNING anhang Hochladen zu langsam abgebrochen` (Beginn und Ende mit Zahl) und `abgewiesen.zu_langsam` + 3; drei 507 `quote` an Anmeldungen von `beta` → eine Episode `WARNING quote Anhänge abgewiesen: Platz für Anhänge voll` mit `kennung=beta@127.0.0.1`; 507 `platte` → `WARNING quote Anhang abgewiesen: Platte fast voll`.
- [ ] **Debug** (Integration, `--protokoll-stufe debug`): je eine `DEBUG anhang`-Zeile für Anmeldung, Hochladen-Ende, Abbruch, Ablauf und Abruf mit `kennung`, `herkunft`, `bytes`, `art`, `dauer_ms`, beim Abruf mit Range `bereich` als Länge.
- [ ] **Keine ID, kein Name** (Integration; Matrix M11): auf Debug Anmelden, Hochladen, Senden, Abruf mit und ohne Range, 404, Abbruch, Ablauf mit Anzeigename `Geheimplan Q4.pdf` → `grep -r <id> logs/` = 0 und `grep -r 'Geheimplan' logs/` = 0.
- [ ] **Messwert QA3** (Entwicklertest, Release-Build, Zielrechner): zwei `PUT` à 1 GiB gleichzeitig; Gesundheit alle 100 ms → p90 ≤ 0,15 s; ein dritter Client sendet 5 Textnachrichten/s → Antwort p90 ≤ 0,25 s, Zustellung im Strom < 1 s ab der 201; ein vierter lädt 1 MiB hoch in der Zeit des Vergleichslaufs ± 50 %; RSS-Zuwachs ≤ 2 × 1 MiB + 50 MB. Der Bericht nennt Rechner, RAM und Plattenart (SSD/HDD) und die Werte des Vergleichslaufs ohne Hochladungen.
- [ ] **Messwert QS6** (Entwicklertest): 2-GiB-WebM, im Player ans Ende springen → die Wiedergabe setzt < 1 s später ein; die Summe der übertragenen Bytes bis dahin liegt unter 64 MiB.
- [ ] **Messwert QA5** (Entwicklertest, Release-Build): Ordner mit 1.000 Anhängen (je 1 KiB, über 7 Tage) → `ladezeit_ms` höchstens 50 ms über dem Ordner ohne Anhänge; gemessen dreimal, Median.

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

- F0097: B0424 Namensregel prüfen · B0425 Eingang führen (unklar) · B0426 Platz reservieren (unklar) · B0427 Anmelden beantworten · B0428 Hochladefristen · B0429 Stücke schreiben (unklar) · B0430 Verwerfen · B0431 Eingang beim Start und im Takt · B0432 Anhang-Kennzahlen und gekürzte ID.
- F0098: B0433 Bildart und Maße (unklar) · B0434 Medienart (unklar) · B0435 Text über Stückgrenzen · B0436 Art beim Hochladen.
- F0099: B0437 Metazeile schreiben und lesen · B0438 Anhangverzeichnis · B0439 Sendekörper mit Anhängen prüfen · B0440 Binden (unklar) · B0441 Anhänge in der Nachricht · B0442 Anhänge laden · B0443 Anhänge entfallen mit dem Tag.
- F0100: B0444 Bereich lesen · B0445 Kopfzeilen der Anhang-Antwort · B0446 Anhang ausliefern · B0447 Gestreamt senden (unklar).
- F0103: B0462 Anhang-Ereignisse · B0463 Anhang-Episoden · B0464 Keine Anhang-ID im Protokoll belegt · B0465 Messwert QA3 (unklar) · B0466 Messwerte QS6 und QA5 (unklar).

Elemente (Schnittstellen in A00016–A00018): `Anhangeingang` (Operation, Zustände und Fristen), `Erkenner` und `name_pruefen` (rein), `Anhangverzeichnis` (Operation), `bereich(angabe, laenge) -> Ganz | Teil(a, b) | Unerfuellbar` (rein), Plattenauskunft als Port der Logablage.

### Änderungen an bestehenden Klassen

- `Gespraeche`: Zähler `gebunden` und `reserviert`, `reservieren`, `senden` mit Anhängen, Laden und Entfallen mit Anhangbytes.
- Senderoute: Körper `{text, anhaenge?}`, Leer-Regel mit Anhang, Wiederholschlüssel über Text und IDs.
- `grenzschutz.rs`: Kopfzeilen für `/anhaenge/` (die App-CSP mit `media-src` gehört zu R00030 F0102).
- `anfrageprotokoll.rs`: Pfade mit Anhang-ID kürzen.

## Tests

Nach Skill `test-pyramide`; je Kriterium die genannte Mutante, das Ergebnis steht im Bericht des Entwicklers.

- **Unit (rein)**: Namensregel, Erkenner (Tabelle, Stückelung, kaputte Köpfe), Fristen, Bereich, Kopfzeilen, Metazeile, Blockmaß; Eingang und Gespräche mit Speicherablage, gestellter Uhr, festem Zufall und gestellter Plattenauskunft (erster Unit-Test aus ADR-0046 samt Mutanten).
- **Integration** (echtes Programm, Wegwerf-Ordner): alle Kriterien „Integration“ oben; Probe gegen den echten Einstieg für Grenzschutz, Kopfzeilen und Protokoll.
- **E2E**: „Kein Skript im Ursprung“ (Chromium).
- **Messwerte**: QA3, QS6, QA5 im Entwicklertest, nicht im Gate.

## Abhängigkeiten

- Abhängig von: I0009 Nachricht senden (gelb; Senderoute, Strom, Verlauf — R00011 grün), I0023 Plattenquote (rot, Karte #34): F0075 Senden an der Quote abgewiesen (Zähler `belegt`, Quote-Prüfung) für F0097, F0076 Quote im Betriebsprotokoll (Episoden-Bereich `quote`) für F0103. I0021 Betriebsprotokoll (gelb; Protokoll, Episoden, Tagestakt — benutzt).
- Innerhalb: F0098 und F0099 brauchen F0097; F0100 braucht F0098 und F0099; F0103 braucht F0100.
- Blockiert: R00030 (F0101 braucht F0099, F0102 braucht F0100); I0012, I0015, I0016, I0019, I0022 nutzen Anhänge (ADR-0046, ADR-0047 „prägt“).

## Offene Fragen

An den Betreiber (stellt der Director; bis zur Antwort gilt der Standard der ADR, die Kriterien sind markiert):

1. **F1 Anteil je Herkunft**: darf ein Teilnehmer bzw. eine Herkunft den ganzen Platz für Anhänge belegen, oder höchstens einen Anteil (ADR-0046 Option K7)? Standard: ganzer Platz, nur offene Hochladungen begrenzt. Betrifft F0097 „Nur eine Herkunft füllt den Platz“.
2. **F2 Direkter Medienlink**: darf ein direkt aufgerufener Medienlink sofort abspielen (Verhalten des Browsers), oder gilt „ohne Autoplay“ auch dort? Standard: hingenommen. Betrifft F0100 „Direkter Medienlink“.
3. **F3 Großer Text**: Text über 16 MiB nur als Download? Standard: ja. Betrifft F0098 „Text über 16 MiB“.

## Manuelle Vorbereitungstätigkeiten

- **Gate vor dem Bau** — `security-reviewer`: Angriffsmatrix ADR-0047 M1–M12 gegen die Bubbles prüfen, bevor F0097, F0098, F0099 und F0100 gebaut werden; Zuordnung: M5, M6, M9, M10, M11, M12 → F0097 (B0424, B0427–B0430, B0432); M2, M3 → F0098 (B0433–B0436); M7, M8, M9 → F0097 und F0099 (B0426, B0431, B0440, B0442, B0443); M1, M2, M4, M10, M11, M12 → F0100 (B0444–B0447); M13 erst mit I0012. Fällt eine Zeile von QS1, gilt ADR-0047 Option A2 (eigener Ursprung) als nächste Entscheidung.
- Director: offene Fragen F1–F3 an den Betreiber.

## Manuelle Nachbereitungstätigkeiten

- Entwicklertest: Messwerte QA3, QS6, QA5 auf dem Zielrechner; PDF und WebM zusätzlich in Firefox (ADR-0047 „Messwert“; ohne Darstellung unter `sandbox` → Art auf `attachment` als Ergänzung der ADR).
- I0019 (Anleitung `lc`): drei Schritte, `anlauf_bis`, `gueltig_bis`, 507 nicht wiederholen, 429 nach `Retry-After`, 404 `abgelaufen` → neu anmelden.

## Umfang

Gezählt aus der WBS (2026-10-29, nach den Regeln des Skills `work-breakdown-structure`):

- F0097 Anmelden und Hochladen über die API: 9 Bubbles · 6 klar (je 0,4 h Untergrenze, Richtwert, ungemessen) · 3 unklar (je 2–4 h) — zusammen 8,4–14,4 h.
- F0098 Art der Datei erkennen: 4 Bubbles · 2 klar · 2 unklar — 4,8–8,8 h.
- F0099 Senden mit Anhängen: 7 Bubbles · 6 klar · 1 unklar — 4,4–6,4 h.
- F0100 Ausliefern mit Art und Teilabruf: 4 Bubbles · 3 klar · 1 unklar — 3,2–5,2 h.
- F0103 Anhänge im Betrieb: 5 Bubbles · 3 klar · 2 unklar — 5,2–9,2 h.
- Zusammen 29 Bubbles · 20 klar · 9 unklar — 26–44 h.

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
