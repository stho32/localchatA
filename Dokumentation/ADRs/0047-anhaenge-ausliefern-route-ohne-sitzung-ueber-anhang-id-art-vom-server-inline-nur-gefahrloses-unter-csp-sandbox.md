# ADR-0047: Anhänge ausliefern — eine Route ohne Sitzung über die Anhang-ID, die Art bestimmt der Server, inline nur Gefahrloses unter `CSP: sandbox`, alles andere als Download; Bildvorschau aus dem Original, Video und Audio im Player des Browsers mit Range

**Status**: Akzeptiert
**Datum**: 2026-10-29 (Entwurf), 2026-10-29 (akzeptiert nach den Antworten des Betreibers B1, B3, B4)
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #9 I0010); Richtung vom Betreiber (Kommentar auf Karte #9, 2026-10-29 07:40)
**Kontext-Anforderung**: — (WBS I0010, Dialog D0003, Karte #9; prägt I0012, I0015, I0016, I0017, I0022)
**Erprobt durch**: zwei Wegwerf-Proben im Scratchpad, beide 2026-10-29, danach gelöscht (Python-`http.server` auf freiem Port, Playwright 1.57 mit Chromium headless-shell und Chromium headless; jede Anhang-Antwort mit `X-Content-Type-Options: nosniff`, `Content-Security-Policy: sandbox; default-src 'none'`; die App-Seite legt `localStorage.t = "geheim"` an).
Probe 1 (Entwurf): (1) `text/html` **ohne** `sandbox` im selben Ursprung → das eingebettete Skript liest `localStorage` — die Bedrohung ist echt; (2) `text/html` mit `sandbox; default-src 'none'` → kein Skript läuft; (3) mit `sandbox allow-scripts` → Skript läuft, `localStorage` wirft `SecurityError`; (4) `<img>` in der App-Seite auf eine PNG-Antwort mit `sandbox`-CSP → geladen; (5) PNG und `text/plain` mit `sandbox` öffnen als Dokument normal, `<script>` im Text erscheint als Text; (6) `application/pdf` mit `sandbox` öffnet in Chromium headless den PDF-Betrachter wie ohne (Firefox nicht geprüft); (7) `application/octet-stream` + `attachment` → Download.
Probe 2 (nach B1/B3; Testdateien aus `ffmpeg`: WebM VP8/Opus, MP4 VP9/Opus, MP4 H.264/AAC, MP3, Ogg Vorbis, je 6 s): (8) **App-CSP von heute** (`default-src 'none'` ohne `media-src`): `<video>`/`<audio>` → `play()` scheitert mit `NotSupportedError`, keine einzige Anfrage — die App-CSP braucht `media-src 'self'`; (9) mit `media-src 'self'` spielen WebM, MP4 (VP9 und H.264), Ogg und MP3 in beiden Chromium-Varianten, obwohl jede Medienantwort die `sandbox`-CSP trägt; der Browser fragt `Range: bytes=0-` an; (10) **mit Range** (206) springt Spulen ans Ende (`currentTime` 6,01 von 6,01, `seekable` bis 6,0); (11) **ohne Range** (`Accept-Ranges: none`, 200 ganz) bleibt `seekable` leer und der Sprung ans Ende landet bei 1,98 s — Spulen geht ohne Range nicht; (12) **direkter Aufruf** der Medien-URL (wie „Öffnen“ im neuen Tab oder ein weitergegebener Link): Chromium baut sein Medien-Dokument mit `<video controls autoplay>` — **der Browser setzt `autoplay` selbst**; `localStorage` wirft dort `SecurityError` (undurchsichtiger Ursprung); (13) **Polyglot** WebM + angehängtes `<script>`-HTML und HTML-Text als `video/mp4` ausgeliefert → Medien-Dokument, kein Skript (Titel leer), `localStorage` unerreichbar.

## Kontext

- **Vision**: „Anhänge. An eine Nachricht lassen sich eine oder mehrere Dateien hängen.“ WBS I0010: „… und lassen sich öffnen“. Leitplanken: keine Anmeldung, eine gemeinsame Instanz im LAN; „Ausdrücklich nicht Ziel“: Rechte, öffentlicher Internet-Dienst. WBS I0012: „nur Absender und Empfänger sehen das Gespräch“.
- **Antworten des Betreibers** (Karte #9, 2026-10-29):
  - **B1** „jede Datei annehmen; im Browser öffnen Bilder, PDF, Text und Video/Audio (MP4, WebM, MP3, Ogg — Player des Browsers); alles andere nur Laden.“
  - **B3** „Bilder als Vorschau aus dem Original mit reserviertem Platz; Videos mit dem Player des Browsers ohne Autoplay.“
  - **B4** „wer den Link hat, öffnet (auch Direktgespräche), bis der Tag entfällt.“
  - B2 (Größe fast unbegrenzt, non-blocking): Folgen für das Ablegen in [ADR-0046](0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md); hier: Auslieferung von Gigabytes gestreamt und mit Range.
- **Ursprung der App**: `http://<ip-oder-name>:<port>` — Oberfläche, API und Token (`localStorage`, [ADR-0012](0012-identitaet-und-sitzung.md)) teilen ihn. Eine Datei, die dort als HTML, SVG oder XML-Dokument dargestellt wird, liest das Token (Probe 1). Die CSP der App ([ADR-0014](0014-grenzschutz-host-origin-sicherheitsheader.md) Regel 5, `src/http/grenzschutz.rs:368`) gilt je Antwort — eine Anhang-Antwort braucht ihre eigene; die App-CSP selbst erlaubt heute keine Medien (Probe 8).
- **Sitzung nur im Header** ([ADR-0012](0012-identitaet-und-sitzung.md), [ADR-0015](0015-ein-ereignisstrom-je-browser-sharedworker-mit-sitzungs-header.md)): `<img src>`, `<video src>`, `<a href>`, ein neuer Tab senden keinen eigenen Header; ein Token im Query-Parameter ist ausgeschlossen; Cookies sind verworfen.
- **Sichtbarkeit** ([X00002 Benutzerrechte](../Architektur/Aspekte/X00002-benutzerrechte.md)): keine Rechte; jeder mit Sitzung liest jeden Channel; einzige Regel: ein Direktgespräch sehen nur seine Beteiligten (I0012).
- **Große Antworten** ([ADR-0039](0039-grosse-antworten-byte-budget-je-block-werkbank-und-abrissgrenze-als-sendefrist.md) Regel 7c, e): Dateien gestreamt in 64-KiB-Stücken mit Gegendruck; Sendefrist ist die Abrissgrenze.
- **Designsystem** (`components/Anhang/README.md`): Datei-Zeile mit Typkürzel, Name, Größe, „Öffnen“ und „Laden“; Bilder als Vorschau höchstens 360 × 240 px, Platz aus Breite und Höhe der Metadaten reserviert, Klick öffnet das Original im Dialog. README „Billig je Nachricht … Bilder reservieren ihren Platz aus den Metadaten“.
- Ablage, Anhang-ID (128 Bit Zufall) und Metazeile: [ADR-0046](0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md).

Szenarien mit Maß:

- **QS1 Kein aktiver Inhalt im Ursprung**: ein Teilnehmer lädt HTML, SVG, XHTML, eine Polyglot-Datei (PNG-Kopf + HTML, WebM/MP4 + HTML) oder Text mit `<script>` hoch, ein anderer klickt „Öffnen“, spielt ab oder ruft den Link direkt auf → in keinem Fall läuft ein Skript mit Zugriff auf `localStorage` oder die API der Instanz; geprüft im echten Programm mit Chromium.
- **QS2 Öffnen**: Klick auf „Öffnen“ bei Bild, PDF oder Text → die Datei erscheint im neuen Tab, ohne Download-Dialog; Video und Audio spielen im Player im Verlauf; jede andere Art → Download unter dem richtigen Namen.
- **QS3 Vorschau billig**: ein Channel mit 100 Bildnachrichten öffnet so schnell wie ohne (Q1b ≤ 500 ms, Platz reserviert, kein Springen L4 ≤ 1 px); Bilder laden erst in Sichtnähe.
- **QS4 Langsame Leser**: 160 Verbindungen einer Herkunft holen je einen großen Anhang und lesen nicht → RSS-Zuwachs ≤ 160 × 64 KiB + 50 MB (≈ 60 MB), jede Verbindung nach der Abrissgrenze zu (Q8 sinngemäß) — unabhängig von der Dateigröße.
- **QS5 Video und Audio**: ein Channel mit 20 Video- und Audionachrichten öffnet ohne eine einzige Medienanfrage, bevor jemand „Abspielen“ drückt; nichts spielt von selbst; jeder Player hat seinen Platz, bevor ein Byte lädt (L4 ≤ 1 px).
- **QS6 Spulen**: in einem 2-GiB-Video ans Ende springen → die Wiedergabe setzt im LAN < 1 s später dort ein, ohne die Datei bis dahin zu laden.

## Optionen

### Frage 1: In welchem Ursprung wird ein Anhang dargestellt?

#### Option A1: Im Ursprung der App, aber jede Anhang-Antwort mit eigener CSP `sandbox; default-src 'none'`, `nosniff`, Art vom Server, inline nur eine Erlaubnisliste gefahrloser Arten, alles andere `application/octet-stream` + `attachment` (gewählt)
- Vorteile: zwei unabhängige Linien — was nicht auf der Liste steht, wird nie dargestellt (Download), und selbst ein dargestelltes Dokument läuft in einem undurchsichtigen Ursprung ohne Skript (Proben 2, 3, 12, 13); ein Port, ein Grenzschutz, ein Start; `<img>`, `<video>`, `<audio>` laden Antworten mit `sandbox`-CSP (Proben 4, 9); PDF-Betrachter und Medien-Dokument funktionieren (Proben 6, 12).
- Nachteile: hängt an der korrekten Umsetzung von `sandbox` im Browser; wer später eine Art auf die Liste setzt (SVG „für die Vorschau“), umgeht die erste Linie — die zweite hält dann allein.

#### Option A2: Eigener Ursprung für Anhänge — zweiter Listener auf einem weiteren Port
- Vorteile: die stärkste Trennung; hält auch bei einem Fehler im `sandbox` des Browsers.
- Nachteile: zweiter Port je Instanz (finden, ausweichen, nennen, in der Firewall öffnen), zweiter Grenzschutz mit eigener Host-Liste (ADR-0045), Verbindungsgrenzen doppelt; `img-src`/`media-src` der App bräuchten den zweiten Ursprung. Bleibt die Ausweichoption, falls die Angriffsmatrix eine Lücke in A1 findet.

#### Option A3: Nie darstellen — alles `attachment`, Vorschau nur über `<img>`
- Vorteile: am einfachsten.
- Nachteile: widerspricht B1 („im Browser öffnen … im Player des Browsers“). Verworfen.

#### Option A4: Abruf mit Sitzungs-Header im Skript, Darstellung über `blob:`-URLs
- Vorteile: der Nachweis bleibt der Header.
- Nachteile: ein `blob:`-Dokument gehört dem Ursprung der App (dort hätte ein HTML-Anhang das Token, Probe 1); die ganze Datei liegt im Speicher des Tabs — bei B2 Gigabytes; kein Spulen ohne eigene Range-Logik. Verworfen.

### Frage 2: Woran erkennt die Instanz, dass jemand einen Anhang öffnen darf? — entschieden vom Betreiber (B4)

- **Z1 (gewählt, B4): die Anhang-ID ist die Berechtigung** — 128 Bit aus `getrandom`, Base32 klein (26 Zeichen); `GET /anhaenge/<id>` ohne Sitzung. Funktioniert für `<img>`, `<video>`, Links, neue Tabs und Agenten gleich; die ID erfährt nur, wer die Nachricht lesen darf (für Direktgespräche über den Filter von X00002). **Wer den Link hat, öffnet ihn** — auch weitergegeben, auch aus Direktgesprächen, bis der Tag entfällt (B4 wörtlich).
- Z2 Sitzungs-Header Pflicht (nur über A4), Z3 Kurzzeit-Ticket je Abruf, Z4 erratbare Pfade — mit B4 nicht mehr nötig bzw. verworfen (Gründe im Entwurf: A4 siehe oben; Z3 ein Aufruf je Vorschaubild und ein Token in der URL; Z4 erratbar).

### Frage 3: Welche Arten werden inline dargestellt? (B1)

Die Art bestimmt die Instanz beim Hochladen aus dem Inhalt ([A00018 Anhangprüfung](../Architektur/A00018-anhangpruefung.md)); der `Content-Type` des Senders und die Endung des Namens zählen nie. Reihenfolge der Prüfung: Signaturen der Tabelle von oben nach unten, dann Text, sonst `application/octet-stream`. Gelesen werden für Signaturen höchstens die ersten 64 KiB; die Text-Prüfung läuft über die ganze Datei.

| Art (Erkennung) | ausgeliefert als | `Content-Disposition` | im Verlauf |
|---|---|---|---|
| PNG (`89 50 4E 47 0D 0A 1A 0A`, Maße aus `IHDR`) | `image/png` | `inline` | Vorschau (Regel 5) |
| JPEG (`FF D8 FF`, Maße aus dem ersten `SOFn`) | `image/jpeg` | `inline` | Vorschau |
| GIF (`GIF87a`/`GIF89a`, Maße aus dem Kopf) | `image/gif` | `inline` | Vorschau |
| WebP (`RIFF….WEBP`, Maße aus `VP8 `/`VP8L`/`VP8X`) | `image/webp` | `inline` | Vorschau |
| PDF (`%PDF-`) | `application/pdf` | `inline` | Datei-Zeile mit „Öffnen“ |
| MP4 (Box `ftyp` ab Byte 4, Boxgröße 16–4.096; Hauptmarke `M4A `/`M4B ` → Audio, `qt  ` → nicht MP4) | `video/mp4` bzw. `audio/mp4` | `inline` | Player (Regel 5) |
| WebM (EBML `1A 45 DF A3`, `DocType` = `webm` im EBML-Kopf; `matroska` → nicht WebM) | `video/webm` | `inline` | Player |
| Ogg (`OggS`, Version 0; erstes Paket `\x01vorbis` oder `OpusHead` → Audio, `\x80theora` → Video; sonst nicht Ogg) | `audio/ogg` bzw. `video/ogg` | `inline` | Player |
| MP3 (`ID3` mit Version 2–4, oder MPEG-Audio-Rahmenkopf Layer III mit gültiger Bitrate und Abtastrate **und** ein zweiter gültiger Rahmenkopf an der berechneten Stelle) | `audio/mpeg` | `inline` | Player |
| Text (gültiges UTF-8 über die ganze Datei, kein NUL), **≤ 16 MiB** | `text/plain; charset=utf-8` | `inline` | Datei-Zeile mit „Öffnen“ |
| Text über 16 MiB | `text/plain; charset=utf-8` | `attachment` | Datei-Zeile, nur „Laden“ |
| alles andere — auch Text mit NUL oder ungültigem UTF-8, ausführbare Dateien, Archive, MOV, MKV, WAV, FLAC | `application/octet-stream` | `attachment` | Datei-Zeile, nur „Laden“ |

- Weil SVG, HTML und XML gültiger Text sind, erscheinen sie als **Quelltext** (`text/plain`, Probe 5) — nie als Dokument.
- Text über 16 MiB wird nicht inline geöffnet: ein Tab, der Hunderte Megabytes Text setzt, friert ein (B2 macht solche Dateien möglich); 16 MiB decken Logs und Berichte. Bestätigung als offene Frage F3.
- Medien inline auch dann, wenn der Browser den Codec nicht kann: der Player zeigt einen Fehler, „Laden“ bleibt — gefahrlos, weil ein Medien-Dokument kein Skript ausführt (Proben 12, 13).
- Verworfen: die Art aus der Endung, die Art vom Sender, SVG als Bild (Gründe im Entwurf: aktive Inhalte beim „Öffnen“; die erste Linie bräche); Medien-Erkennung über tiefes Parsen der Container (zweite Parser-Fläche ohne Szenario — die Signatur genügt, weil der Browser die Datei ohnehin selbst dekodiert und ein Fehlurteil nur zu einem Player mit Fehlermeldung führt).

### Frage 4: Vorschau und Player — entschieden vom Betreiber (B3), Mechanik hier

**Bilder** — V1 (gewählt, B3): Original in `<img>` mit reserviertem Platz — nur Bildarten der Tabelle, höchstens 10 MiB und höchstens 50 Megapixel (jede Seite ≤ 16.384 px), `loading="lazy"`, Breite und Höhe aus der Metazeile (bis 360 × 240 px im Seitenverhältnis); größere Bilder als Datei-Zeile mit „Öffnen“. Stärke: kein Bildcode auf dem Server; der Platz steht, bevor ein Byte lädt (QS3); die Grenze in Pixeln hält Dekompressionsbomben aus dem Verlauf. Schwäche: ein 10-MiB-Foto wird für 360 px voll geladen (im LAN hingenommen). V2 (keine Vorschau) und V3 (Vorschaubilder auf dem Server) widersprechen B3 bzw. bringen Bilddekodierung fremder Dateien in den Server-Prozess — verworfen.

**Video und Audio** — Platz ohne Dekodierung auf dem Server:

- **P1 (gewählt): feste Box** — Video in einem Rahmen von 360 × 202 px (16 : 9), `<video controls preload="none" playsinline>` füllt ihn mit `object-fit: contain` (andere Seitenverhältnisse bekommen Ränder); Audio als `<audio controls preload="none">` in einer Zeile fester Höhe, 360 px breit; nie `autoplay`, nie `muted`-Autostart. Stärke: der Platz steht ohne ein Byte (QS5, L4); `preload="none"` lädt nichts, bis jemand abspielt — 20 Videos im Fenster kosten keine Anfrage; keine Medienanalyse auf dem Server. Schwäche: vor dem Abspielen weder erstes Bild noch Dauer (schwarzer Rahmen mit Bedienleiste); Hochformat-Videos erscheinen klein mit breiten Rändern.
- P2: Maße auf dem Server aus dem Container lesen (MP4 `tkhd`, WebM `PixelWidth`/`PixelHeight`) und wie bei Bildern reservieren. Stärke: richtige Form ohne Ränder. Schwäche: zweiter, tieferer Parser fremder Container im Server; bei MP4 liegt `moov` oft am Dateiende — Lesen jenseits der ersten 64 KiB; Rotation und Seitenverhältnis-Boxen verkomplizieren. Verworfen ohne Szenario, das Ränder verbietet.
- P3: `preload="metadata"` und Größe vom Browser. Stärke: Dauer und Form sichtbar. Schwäche: der Player springt nach dem Laden der Metadaten auf seine Größe (L4 verletzt), und jedes Video im Fenster stellt eine Range-Anfrage beim Öffnen des Channels (QS5 verletzt). Verworfen.

### Frage 5: Was sieht der Agent?

- **Gewählt: Metadaten überall, Bytes nur über die eine Route.** `nachricht` in Verlauf ([ADR-0025](0025-verlauf-ueber-die-api-bloecke-mit-zeit-als-cursor.md)), Senden-Antwort, Ereignis ([ADR-0028](0028-nachrichten-live-als-drittes-thema-ordnung-ueber-die-nachrichtenzeit.md)) und künftig Warten (I0016) trägt `anhaenge: [{id, name, bytes, art, breite, hoehe, url}]` (immer eine Liste, leer ohne Anhang; `breite`/`hoehe` `null` außer bei Bildern; `url` = `/anhaenge/<id>`, relativ). Ein Agent entscheidet ohne Herunterladen (Größe, Art) und lädt mit `curl -O` — bei großen Dateien mit `curl -C -` fortsetzbar (Range). Keine Suche in Anhängen (I0011 sucht nur Text).

### Frage 6: Teilabrufe (Range) — neu, B1/B3

- **R1 (gewählt): genau ein Bereich je Anfrage** nach RFC 9110 — `Accept-Ranges: bytes`; `Range: bytes=a-b`, `bytes=a-` und `bytes=-n` → **206** mit `Content-Range`; mehrere Bereiche oder eine unlesbare Angabe → die Angabe wird übergangen, **200** ganz; nicht erfüllbar (`a` ≥ Länge, `-0`) → **416** mit `Content-Range: bytes */<länge>`; `If-Range` mit dem ETag → Bereich, sonst 200 ganz. Stärke: Spulen in Video und Audio (Proben 10, 11), PDF-Betrachter laden seitenweise, abgerissene Downloads setzen fort; Speicher je Antwort bleibt ein Stück. Schwäche: ein Bereich je Anfrage verlangt vom Browser mehrere Anfragen beim Springen (geprobt: Chromium fragt ohnehin einzeln).
- R2: keine Teilabrufe (`Accept-Ranges: none`, Entwurf). Stärke: kein Code. Schwäche: Spulen geht nicht (Probe 11) — widerspricht „im Player des Browsers“ bei großen Videos; ein Abriss nach 1,9 GiB lädt alles neu. Verworfen.
- R3: auch mehrere Bereiche (`multipart/byteranges`). Stärke: vollständig nach Norm. Schwäche: kein Browser braucht es für Medien; überlappende Bereiche sind eine bekannte Verstärkungsfläche (eine kleine Anfrage, viele Kopien derselben Bytes). Verworfen.

### Frage 7: „Öffnen“ bei Video und Audio — neu, B1/B3

- **M1 (gewählt): Player im Verlauf, kein „Öffnen“-Link, „Laden“ bleibt.** Der Player im Verlauf **ist** das Öffnen im Player des Browsers (B1), mit Vollbild aus seiner Bedienleiste; nichts spielt ungefragt (B3). Ein direkter Aufruf des Links (weitergegeben, B4, oder in die Adresszeile kopiert) öffnet das Medien-Dokument des Browsers, das selbst `autoplay` setzt (Probe 12) — hingenommen, weil der Aufruf eine ausdrückliche Handlung ist; Bestätigung als offene Frage F2.
- M2: zusätzlich „Öffnen“ im neuen Tab. Stärke: gewohnt wie bei PDF. Schwäche: das Medien-Dokument spielt sofort ab (Probe 12) — liest sich gegen B3.
- M3: Medien beim direkten Aufruf als `attachment` (nur der Player im Verlauf spielt). Stärke: nie Autoplay. Schwäche: der Server unterscheidet `<video>`-Abrufe von Navigationen nur über `Sec-Fetch-Dest` — ein Header, auf den kein Agent und kein älterer Browser achtet; ein weitergegebener Link (B4) lädt herunter statt zu spielen. Verworfen.

## Entscheidung

**Gewählt: A1, Z1 (B4), Arten-Tabelle aus Frage 3 (B1), V1 und P1 (B3), Metadaten wie in Frage 5, R1, M1.**

Es überwiegt, dass A1 mit zwei unabhängigen Linien (Download für alles Unbekannte, `sandbox` für alles Dargestellte, auch für Medien-Dokumente und Polyglot-Container geprobt) kein Skript im Ursprung der App zulässt, ohne einen zweiten Port zu verlangen; dass Z1 als einziger Nachweis für `<img>`, `<video>`, Links und Agenten gleich funktioniert und genau die Bedeutung hat, die der Betreiber für einen Link festgelegt hat; dass V1 und P1 Vorschau und Player ohne Bild- oder Mediencode auf dem Server liefern und ihren Platz vor dem ersten Byte reservieren; und dass R1 Spulen und Fortsetzen trägt, ohne die Mehrfachbereiche, die niemand braucht. In Kauf genommen: die Sicherheit dargestellter Anhänge hängt an `sandbox` im Browser (A2 bleibt die Ausweichoption); ein Link öffnet ohne Sitzung, auch weitergegeben, bis sein Tag entfällt; Vorschaubilder laden das Original; Videos zeigen vor dem Abspielen weder Bild noch Dauer und erscheinen in einem 16 : 9-Rahmen; ein direkt aufgerufener Medienlink spielt sofort.

**Akzeptiert**, weil die Richtungsfragen beantwortet sind (B1, B3, B4) und die tragenden Annahmen in Chromium geprobt sind (Proben 1–13). Die **Angriffsmatrix** des `security-reviewer` (unten) läuft vor dem Bau bzw. als Gate am Arbeitspaket; fällt eine Zeile von QS1, gilt A2 als nächste Entscheidung (neue ADR, diese wird abgelöst).

Regeln:

1. **Route** `GET /anhaenge/{id}` (und `HEAD`) unter `grenzschutz` (Host-Prüfung gilt, also kein Lesen über DNS-Rebinding; ADR-0014, ADR-0045), **ohne** Extraktor `Teilnehmer` — eine mitgeschickte Sitzung wird nicht geprüft und verändert nichts (kein Lebenszeichen). Außerhalb von `/api/`, weil Browser sie ohne Header aufrufen. `id` nicht 26 Zeichen `[a-z2-7]` oder unbekannt → 404 `text/plain` „Diesen Anhang gibt es nicht (mehr).“; Datei fehlt auf der Platte → derselbe 404 und `WARNING anhang` mit Gespräch, Tag, Datei (nie der ID).
2. **Sichtbarkeit** (B4): die ID erscheint nur dort, wo die Nachricht erscheint (Verlauf, Strom, Warten, Senden-Antwort); für Direktgespräche (I0012) filtert X00002 diese Wege. Die Route selbst prüft nichts: wer den Link hat, öffnet ihn, auch aus Direktgesprächen, bis der Tag entfällt.
3. **Kopfzeilen jeder Antwort dieser Route** (auch 206, 304, 404, 416, `HEAD`), zusätzlich zu ADR-0014 Regel 5 und diese ersetzend, wo sie sich überschneiden:
   - `Content-Security-Policy: sandbox; default-src 'none'` — genau die geprobte Richtlinie;
   - `X-Content-Type-Options: nosniff`, `Cross-Origin-Resource-Policy: same-origin`, `Referrer-Policy: no-referrer`, `X-Frame-Options: DENY` (bleiben aus ADR-0014);
   - `Content-Type` und `Content-Disposition` nach der Tabelle in Frage 3; `Content-Disposition: <inline|attachment>; filename="<ascii-ersatz>"; filename*=UTF-8''<prozentkodierter Anzeigename>` (RFC 6266/8187; ASCII-Ersatz = gespeicherter Name ohne Präfix; `"`, `\`, Steuerzeichen und Zeilenumbrüche kommen in keinem der beiden vor, ADR-0046 Regel 4);
   - `Accept-Ranges: bytes`; `Content-Length` (bei 206 die Länge des Bereichs); `Content-Range` bei 206 und 416;
   - `Cache-Control: private, no-cache` und `ETag: "<id>"` (stark) — `If-None-Match` mit derselben ID → 304 ohne Körper, geprüft vor `Range`.
4. **Streaming und Range** nach ADR-0039 Regel 7c und Frage 6 (R1): Datei öffnen (über den Port der [Logablage](../Architektur/A00008-logablage.md), `O_NOFOLLOW` auf der letzten Pfadstelle), bei 206 auf den Anfang des Bereichs setzen, in Stücken zu 64 KiB mit Gegendruck senden; Sendefrist = Abrissgrenze. Zahlen der `Range`-Angabe als u64 mit Überlaufprüfung; eine Angabe mit Komma, Leerraum innerhalb der Zahlen oder einer anderen Einheit als `bytes` wird übergangen (200 ganz). Entfällt der Tag, während gesendet wird, hält der offene Dateideskriptor die Bytes bis zum Ende dieser Antwort.
5. **Im Verlauf** (V1, P1): Bilder als `<img src="/anhaenge/<id>" width height loading="lazy" decoding="async" alt="<anzeigename>">` für Bildarten mit Maßen, ≤ 10 MiB, ≤ 50 Megapixel, jede Seite ≤ 16.384 px; Klick öffnet dasselbe Bild im Dialog (Designsystem); sonst Datei-Zeile. Video als `<video controls preload="none" playsinline src="/anhaenge/<id>">` in einem Rahmen von 360 × 202 px mit `object-fit: contain`; Audio als `<audio controls preload="none" src="/anhaenge/<id>">` in einer Zeile fester Höhe; nie `autoplay`. Unter dem Player die Datei-Zeile mit Name, Größe und „Laden“. Kein `blob:`, kein `innerHTML` mit Anhang-Daten; Name und Größe nur über `textContent` (A00005).
6. **Öffnen und Laden** in der Oberfläche: „Öffnen“ = `<a href="/anhaenge/<id>" target="_blank" rel="noopener">` nur für Bilder, PDF und Text bis 16 MiB; Video und Audio öffnen im Player (M1); „Laden“ = `<a href="/anhaenge/<id>" download="<anzeigename>">` für alle. Kein Skript zwischen Klick und Datei.
7. **App-CSP** (ergänzt [ADR-0014](0014-grenzschutz-host-origin-sicherheitsheader.md) Regel 5): `media-src 'self'` kommt dazu (Probe 8, 9) — ohne `data:` und `blob:`; alle anderen Direktiven bleiben (`img-src 'self' data:` trägt die Vorschau, Probe 4).
8. **Protokoll** ([ADR-0042](0042-stufen-ereigniskatalog-episoden-und-tagesgrenze-des-betriebsprotokolls.md)): die Anhang-ID ist eine Berechtigung und steht wie ein Token **nie** im Betriebsprotokoll — das Anfrageprotokoll (`src/http/anfrageprotokoll.rs`) schreibt für `/anhaenge/…` und `/api/anhaenge/…` den Pfad als `…/<6 Zeichen>…`; `Range` steht höchstens als Länge, nie mit der ID. Durchsetzung: Integrationstest „Protokoll auf Debug, Anmelden, Hochladen, Senden, Abruf mit und ohne Range, `grep <id> logs/` = 0“.
9. **Agent** (Frage 5): Felder wie dort; `POST /api/anhaenge`, `PUT|DELETE /api/anhaenge/{id}` und `GET /anhaenge/{id}` stehen in `/api/openapi.json` (I0015).

## Konsequenzen

**Positiv:**
- Kein Anhang erreicht im Ursprung der App einen Skript-Kontext: Unbekanntes wird heruntergeladen, Dargestelltes — auch Medien-Dokumente und Polyglot-Container — läuft ohne Skript in einem undurchsichtigen Ursprung (geprobt in Chromium).
- Öffnen, Laden, Vorschau, Player, Spulen, neuer Tab, `curl -O`/`curl -C -` — alles über eine Route ohne Skript dazwischen.
- Speicher je Abruf ≤ ein Stück, auch bei Gigabyte-Dateien und bei Range; langsame Leser enden an der Abrissgrenze (QS4).
- Ein Channel voller Videos kostet beim Öffnen keine Medienanfrage (QS5).

**Negativ / Inkauf zu nehmen:**
- Ein Link öffnet ohne Sitzung, auch weitergegeben, auch aus Direktgesprächen, bis sein Tag entfällt (B4, gewollt).
- Die zweite Linie hängt am Browser; Firefox ist nicht geprobt (Messwert unten).
- Vorschaubilder laden das Original; Videos zeigen vor dem Abspielen weder Bild noch Dauer, Hochformat erscheint mit Rändern.
- Ein direkt aufgerufener Medienlink spielt sofort (Medien-Dokument des Browsers, F2).
- SVG, HTML und Markdown-Dateien öffnen als Quelltext; Text über 16 MiB nur als Download (F3).
- Eine Kopie im Browser-Cache überlebt das Entfallen des Tags.

**Pre-Mortem (ein Monat nach dem Bau gescheitert, weil …):**
1. … jemand SVG „für die Vorschau“ auf die Inline-Liste setzt oder die `sandbox`-CSP für eine Art oder für 206/416 weglässt, und ein SVG mit Skript das Token eines Lesers stiehlt. Früh-Indikator: Unit-Tabelle der Arten ändert sich; Integrationstest „jede Antwort unter `/anhaenge/` trägt `sandbox`“ wird angefasst. Gegenmaßnahme: Durchsetzung unten; jede Änderung der Tabelle ist eine Ergänzung dieser ADR.
2. … der Erkenner einer Art auf einer kaputten Datei in einer Schleife hängt oder abstürzt (JPEG-Segmente, EBML-Längen, MP3-Rahmenlängen). Früh-Indikator: Hochladen hängt bis zur Frist; `EXCEPTION`. Gegenmaßnahme: Erkenner liest höchstens die ersten 64 KiB, jede Schleife schreitet fort oder endet, Längen mit Überlaufprüfung; Fuzz-Probe des `security-reviewer`.
3. … die Range-Auswertung bei `bytes=-0`, `bytes=5-2` oder Zahlen nahe 2⁶⁴ falsch rechnet und Bytes außerhalb der Datei oder eine falsche Länge sendet. Früh-Indikator: 206 mit `Content-Length` ≠ gesendeter Länge; abgebrochene Videos. Gegenmaßnahme: reine Funktion `bereich(angabe, laenge) -> Ganz | Teil(a, b) | Unerfuellbar` mit Tabellentest; Matrix-Zeile.
4. … Nutzer über Videos klagen, die sofort losspielen, wenn sie einen Link aus dem Chat in einem neuen Tab öffnen. Früh-Indikator: Rückmeldung im Entwicklertest. Gegenmaßnahme: F2; bei „nein“ M3 mit `Sec-Fetch-Dest` als Ergänzung.

**Durchsetzung (Makro, Aspekte [X00002](../Architektur/Aspekte/X00002-benutzerrechte.md), [X00003](../Architektur/Aspekte/X00003-authentifizierung-und-sitzung.md)):**
- Unit (rein, A00018): Tabelle Inhalt → Art, Disposition, Anzeige; Mutanten „SVG inline“, „Endung entscheidet“, „Text ohne UTF-8-Prüfung“, „Text über 16 MiB inline“, „MP3 ohne zweiten Rahmenkopf“ rot.
- Unit (rein): `bereich` mit `bytes=0-0`, `0-`, `-1`, `-0`, `5-2`, `len-`, `0-len+10` (gekappt), `18446744073709551615-`, `0-1,2-3`, `items=0-1`, ` bytes=0-1` → Ganz/Teil/Unerfüllbar wie in Frage 6.
- Integration (echtes Programm): je Art der Tabelle eine Datei hochladen, senden, abrufen → `Content-Type`, `Content-Disposition`, `sandbox`-CSP, `nosniff`, `Accept-Ranges`; auch auf 206, 304, 404, 416. Mutationsprobe je Kopfzeile.
- E2E (Chromium): HTML-, SVG- und Polyglot-Anhang (PNG + HTML, WebM + HTML) über „Öffnen“, über den Player und als direkte Navigation → `localStorage` der App bleibt unerreicht; Video im Verlauf: vor „Abspielen“ 0 Medienanfragen, Platz steht, nach Sprung ans Ende 206 und `currentTime` am Ende; Download für Nicht-Inline-Arten.
- Review-Punkt `code-reviewer`/`security-reviewer`: keine Antwort unter `/anhaenge/` ohne die Kopfzeilen aus Regel 3; keine Anhang-ID im Protokoll; kein `autoplay` im Code der Oberfläche.

**Angriffsmatrix (Zeilen für den `security-reviewer`, vor dem Bau bzw. als Gate am Arbeitspaket):**

| # | Fläche | Angriff | erwartet |
|---|---|---|---|
| M1 | Aktiver Inhalt | HTML, SVG, XHTML, XML mit XSLT, Text mit `<script>`, PDF mit JavaScript; über „Öffnen“, `<img>`, `<video>`, direkte Navigation; Chromium **und** Firefox | kein Skript mit Zugriff auf `localStorage`/API; HTML/SVG als Quelltext |
| M2 | Polyglot | PNG-Kopf + HTML; GIF + JS; WebM/MP4/Ogg/MP3 + angehängtes HTML; HTML mit `ID3`- oder `OggS`-Präfix (wird Medienart) | Medien-Dokument bzw. Bild, kein Skript (Probe 13 nachfahren im echten Programm) |
| M3 | Art-Erkennung | Fuzz der Köpfe PNG/JPEG/GIF/WebP/`ftyp`/EBML/`OggS`/MPEG-Rahmen: abgeschnitten, Längen 0 und 2⁶⁴−1, verschachtelte Boxen, Rahmenlänge 0; UTF-8 mit Mehrbytezeichen über einer Stückgrenze, ungültiges UTF-8 genau an der Grenze, NUL nach 2 GiB | keine Panik, keine Schleife, ≤ 64 KiB gelesen für Signaturen; Text-Urteil unabhängig von der Stückelung |
| M4 | Range | `bytes=-0`, `5-2`, `a` ≥ Länge, Zahlen über u64, `0-1,1-2,…` (1.000 Bereiche), `If-Range` falsch und als Datum, Range auf 404/304/HEAD, viele kleine Bereiche in schneller Folge (Deskriptoren) | 416 bzw. 200 ganz wie Frage 6; nie `multipart/byteranges`; Länge stimmt; Deskriptoren werden je Antwort geschlossen |
| M5 | Große Dateien, Hochladen | `Content-Length` > angemeldet, < angemeldet (Stückfrist), zwei `Content-Length`, `Transfer-Encoding: chunked` plus `Content-Length`, Bytes nach dem Körper (Request Smuggling über keep-alive), führende Nullen, negative Zahl | 422/411/400 ohne Lesen bzw. 408; keine zweite Anfrage aus Körperbytes |
| M6 | Langsam | Anmelden, `PUT` mit 1 Byte je 9 s; mit 0,9 MiB/s; 16 solche je Herkunft; 32 gesamt aus zwei Adressen | 408 nach Stückfrist bzw. Gesamtfrist; Reservierung frei nach Anlauffrist; Senden und Gesundheit der anderen unverändert (QA3-Werte) |
| M7 | Quote-Erschöpfung durch einen Teilnehmer | eine Kennung meldet den ganzen Anhangplatz an, ohne hochzuladen, wiederholt im Takt; eine Herkunft füllt den Anhangplatz mit echten Dateien; Flut winziger Dateien (1 Byte); Platte voll bei `--plattenquote aus` | 429 nach Schreibkonto; Reservierung ≤ 60 s; danach 507 `quote` für Anhänge, **Text 201** bis zur Quote; Blockmaß zählt 4 KiB; 507 `platte` vor dem Schreiben |
| M8 | Abbruch und Aufräumen | Abbruch vor dem ersten Byte, mitten drin, nach dem letzten Byte vor `rename`, während des Bindens; `kill -9` während des Hochladens; Symlink in `.eingang/` und in `anhaenge/<tag>/` | keine `.teil` bleibt, Reservierung frei, Invariante hält; Start leert den Eingang ohne Symlinks zu folgen |
| M9 | Fremde IDs | `PUT`/`DELETE`/Senden mit der ID eines anderen | 404/204 ohne Wirkung/422 `fremd`; kein Orakel über den Zustand fremder IDs |
| M10 | Name | Pfad (`../`), Richtungszeichen, CR/LF und `"` für Header-Einschleusen in `Content-Disposition`, 255-Byte-Grenze, Normalisierung | 422 `name`; Header immer wohlgeformt |
| M11 | ID-Abfluss | Betriebsprotokoll (alle Stufen), `Referer`, Fehlermeldungen, Anfrageprotokoll mit Range | ID nie sichtbar (Regel 8) |
| M12 | Ursprung | DNS-Rebinding auf `/anhaenge/`, CSRF auf `POST`/`PUT`/`DELETE /api/anhaenge` von fremdem Ursprung (auch `text/plain`-Formular) | 403 bzw. nie beantwortete Vorabfrage; keine Wirkung |
| M13 | Direktgespräche (B4) | Dritter ohne Beteiligung liest Verlauf, Strom, Warten | keine Anhang-ID aus dem Direktgespräch; mit Link öffnet er (gewollt) |

**Folgeentscheidungen / Aufgaben:**
- [x] **Betreiber**: B1, B3, B4 beantwortet (Karte #9, 2026-10-29).
- [ ] **Offene Fragen** an den Betreiber (Bericht Karte #9): F2 — darf ein direkt aufgerufener Medienlink sofort abspielen (Verhalten des Browsers), oder gilt „ohne Autoplay“ auch dort? F3 — Text über 16 MiB nur als Download, einverstanden?
- [ ] **Angriffsmatrix** vom `security-reviewer` (Zeilen M1–M13); fällt eine Zeile von QS1, gilt A2 als nächste Entscheidung.
- [ ] **Erster Unit-Test** (A00018, rein): 8 Bytes PNG-Kopf + `IHDR` 1280 × 800 → `image/png`, inline, Maße 1280 × 800, Vorschau ja; derselbe Kopf mit `IHDR` 20.000 × 20.000 → inline, Vorschau nein; `<svg …><script>` → Text, inline als `text/plain`; dasselbe mit NUL-Byte → `application/octet-stream`, `attachment`; `%PDF-1.7` → `application/pdf`, inline; `….ftypisom` → `video/mp4`; `….ftypM4A ` → `audio/mp4`; `….ftypqt  ` → `application/octet-stream`; EBML mit `DocType` `webm` → `video/webm`, mit `matroska` → `application/octet-stream`; `OggS` + `OpusHead` → `audio/ogg`; `ID3\x04` → `audio/mpeg`; ein einzelner MPEG-Rahmenkopf ohne zweiten → kein MP3; abgeschnittene Köpfe jeder Art → nie Panik.
- [ ] **Erster Integrationstest** (echtes Programm): `x.html` mit `<script>` hochladen, senden, `GET /anhaenge/<id>` → `text/plain; charset=utf-8`, `inline`, CSP beginnt mit `sandbox`; WebM hochladen → `video/webm`, `Accept-Ranges: bytes`; `Range: bytes=100-199` → 206, `Content-Range: bytes 100-199/<n>`, 100 Bytes gleich der Datei; `bytes=<n>-` → 416 `bytes */<n>`; `bytes=0-1,5-6` → 200 ganz; unbekannte ID → 404 mit derselben CSP; `If-None-Match: "<id>"` → 304; `Host: evil.example` → 403; nach Frist 1 und Tageswechsel (`LOCALCHATA_JETZT`) → 404.
- [ ] **Messwert** (Entwicklertest): PDF und WebM in Chromium und Firefox über „Öffnen“ bzw. Player → Darstellung; bei Firefox ohne Darstellung unter `sandbox` → Art auf `attachment` (Ergänzung). 2-GiB-Video im LAN: Sprung ans Ende spielt < 1 s (QS6).
- [x] **Ergänzungen bei Annahme** (architekt, Karte #9): ADR-0014 (Regel 5 für `/anhaenge/`, `media-src`), ADR-0025 und ADR-0028 (Feld `anhaenge`), ADR-0042 (Regel 8 hier), X00002 (Z1, B4) — geschrieben.

## Quellen / Bezuege

- [A00004 HTTP-Schnittstelle](../Architektur/A00004-http-schnittstelle.md), [A00005 Weboberfläche](../Architektur/A00005-weboberflaeche.md), [A00016 Anhangverzeichnis](../Architektur/A00016-anhangverzeichnis.md), [A00018 Anhangprüfung](../Architektur/A00018-anhangpruefung.md), [A00008 Logablage](../Architektur/A00008-logablage.md); Aspekte [X00002](../Architektur/Aspekte/X00002-benutzerrechte.md), [X00003](../Architektur/Aspekte/X00003-authentifizierung-und-sitzung.md)
- ADR-0012, ADR-0014, ADR-0015, ADR-0025, ADR-0028, ADR-0039, ADR-0042, ADR-0045, ADR-0046
- Vision R00000; WBS I0010, I0012; Betreiber-Antworten B1, B3, B4 (Karte #9, Kommentar 2026-10-29 07:40); Designsystem `components/Anhang/README.md`, `README.md` („Billig je Nachricht“); `src/http/grenzschutz.rs:368` (App-CSP heute)
- CSP-Direktiven `sandbox`, `media-src` (CSP Level 2/3), `Content-Disposition` RFC 6266, `filename*` RFC 8187, Range/206/416/`If-Range` RFC 9110 Abschnitt 14, `X-Content-Type-Options` (Fetch Standard), Signaturen von MP4 (`ftyp`, ISO/IEC 14496-12), EBML/WebM, Ogg (RFC 3533), MPEG-Audio-Rahmenkopf — aus dem Gedächtnis, in diesem Auftrag nicht geöffnet; das Verhalten in Chromium ist geprobt (oben)
