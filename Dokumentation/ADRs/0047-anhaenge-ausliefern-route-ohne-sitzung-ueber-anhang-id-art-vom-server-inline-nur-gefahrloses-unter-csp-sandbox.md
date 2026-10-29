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
- [x] **Offene Fragen** an den Betreiber (beantwortet 2026-10-29: F2 hingenommen, F3 bestätigt, F4 tilgen — Ergänzungen unten): F2 — darf ein direkt aufgerufener Medienlink sofort abspielen (Verhalten des Browsers), oder gilt „ohne Autoplay“ auch dort? F3 — Text über 16 MiB nur als Download, einverstanden?
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

## Ergänzungen

- **2026-10-29 (Karte #9 Gate, `security-reviewer`) — Angriffsmatrix mit Linie und Bubble, M14–M39, und die Regeln, die daraus folgen:** Entscheider: Claude (autonom, architekt, Auftrag des Directors). Status der Ergänzung: Akzeptiert; zwei Lücken sind am heutigen Code geprobt (Regeln 1 und 2). Die Matrix oben bleibt stehen; diese Ergänzung ordnet ihr Linien und Bubbles zu, präzisiert M6, M7 und M9 und fügt Zeilen an. Die Matrix ist das Kriterium „Angriffsmatrix“ von R00029: jede Zeile außer L2r ein Test über die genannte Linie, jede Zeile mit Wächter rot unter dessen Mutante.

  **Linien (Prüfebene):** L1 im Prozess — Unit und Integration mit Speicherablage, Testuhr, festem Zufall und Attrappen · L2 echtes Programm über die Annahmeschleife (Debug-Build, Wegwerf-Ordner, `LOCALCHATA_*`, `--port 0`), Browserzeilen mit Playwright/Chromium gegen dieses Programm · L2r von Hand im Entwicklertest (Firefox, Zielrechner, LAN).

  **Kontrollpaar K:** K1 gutartiger Ablauf `alpha@127.0.0.1` — Anmelden 300 KiB PNG → 201, `PUT` → 200 `image/png`, Senden → 201, `GET /anhaenge/<id>` → 200 mit `sandbox`-CSP, `Range: bytes=0-99` → 206; K2 eine volle Herkunft — 127.0.0.2 mit 16 offenen Anmeldungen, die 17. → 429 `offen_herkunft`. Färbt eine Probe K1 rot, misst sie nichts; bleibt K2 ohne 429, misst M21 nichts. Beide laufen in jeder Linie mit, in der eine Probe läuft.

  **M1–M13 ergänzt** (Linie, Bubble, Präzisierung, Lücke aus dem Gate):

  | # | Linie | Bubble | Präzisierung / Lücke |
  |---|---|---|---|
  | M1 | L1, L2 (Chromium, direkte Navigation), L2r (Firefox) | B0433–B0436, B0446, Weiche (Regel 1) | `<img>`, `<video>` und „Öffnen“ aus der Oberfläche gehören zu R00030 (Karte #43, F0102); Firefox ist der Messwert oben (L2r) |
  | M2 | L1, L2 (Chromium) | B0434, B0436, B0446 | Probe 13 im echten Programm |
  | M3 | L1 | B0433, B0434, B0435 | Tabellentest plus Zufallsköpfe mit fester Saat; Maße nach Regel 4 (M27) |
  | M4 | L1, L2 | B0444, B0446, B0447 | Erweiterungen in M31 |
  | M5 | L2 | B0429 | Duplikate, `+`/`-`, TE+CL, Smuggling ausführlich in M32 |
  | M6 | L1, L2 | B0428, B0429, B0425, B0427 | „32 gesamt aus zwei Adressen“ wird: 16 laufende je Herkunft aus vier Adressen (127.0.0.2–.5) → die 65. laufende → 429 `offen_gesamt` (ADR-0046 Ergänzung Karte #9 Gate Regel 3); Senden und Gesundheit von 127.0.0.1 unverändert |
  | M7 | L1, L2 | B0426, B0427, B0425 | „Reservierung ≤ 60 s“ wird: eine Anmeldung ohne `PUT` ≤ 60 s, mit abgebrochenen `PUT`s ≤ Lebensende (ADR-0046 Ergänzung Regel 1, M20); wiederholtes Neu-Anmelden kostet je 1 Einheit und erscheint als Episode (M22) |
  | M8 | L1, L2 | B0425, B0429, B0440, B0431 | `kill -9` in M25, Symlink in M26 |
  | M9 | L2 | B0429, B0430, B0440 | **präzisiert**: kein Orakel für **ratbare** Werte. `PUT` auf eine fremde ID → 404 `unbekannt` (wie nie vergeben); `DELETE` fremd → 204 ohne Wirkung; Senden mit fremder ID → 422 `anhang`/`fremd` **bleibt** — die ID ist 128 Bit Zufall, wer sie nennt, kennt sie schon; die Antwort verrät nichts, was er nicht hat |
  | M10 | L1, L2 | B0424, B0445, B0446 | Sonderzeichen in M28 |
  | M11 | L2 | B0432, B0464 | verfehlte Pfade in M18, Warnungen in M19 |
  | M12 | L2 | B0427, B0429, B0430, B0446 | `PUT`/`DELETE` und Vorabfrage in M34 |
  | M13 | L2 | — (nicht in R00029) | gehört zum Arbeitspaket I0012 (Direktgespräche); Hinweis an den `planer` für die WBS-Zeile I0012 |

  **Neue Zeilen** („heute“ nur, wo am Code von `20cea97` geprobt; die Routen selbst gibt es noch nicht):

  | # | Fläche | Angriff | erwartet | Linie | Bubble | heute |
  |---|---|---|---|---|---|---|
  | M14 | Kopfzeilen | jede Antwort unter `/anhaenge/`: 200, 206, 304, 404 (Handler und Router, `/anhaenge/x/y`), 416, `HEAD`, 403 (fremder Host), 405 (`POST`) | `Content-Security-Policy: sandbox; default-src 'none'` genau so, `nosniff` | L2 | Weiche (neu, Regel 1), B0446 | ✗ `sicherheitsheader_setzen` (`src/http/grenzschutz.rs:341–361`) ersetzt jede CSP der Route durch die App-CSP |
  | M15 | Skript-Gadget | Anhang (Text mit JS, Polyglot) in der App-Seite eingebunden als `<script src>`, `<script type=module>`/`import()`, `new Worker`, `new SharedWorker`, `navigator.serviceWorker.register`, `<link rel=stylesheet>` | nichts ausgeführt bzw. angewandt (MIME-Prüfung unter `nosniff`) | L2 (Chromium) | B0445, B0446 | — |
  | M16 | Metazeile eingeschleust | Text mit Zeilen `+ anhang: <id> …` bzw. eine erfundene ID im Text | bleibt Text (ADR-0004: Textzeilen eingerückt), kein Eintrag im Anhangverzeichnis | L1, L2 | B0437, B0439, B0441 | — |
  | M17 | Metazeile beim Laden | von Hand: ID falscher Form, `bytes` ≠ Datei, `<datei>` mit `/`, `..` oder ohne Präfix, zwei Zeilen mit derselben ID, Art außerhalb der Tabelle | kein Eintrag, `WARNING laden` (Datei nur als Präfix), nie ein Pfad außerhalb `anhaenge/<tag>/` | L1 | B0437, B0442 | — |
  | M18 | ID im Anfrageprotokoll | verfehlte Route mit ID: `/anhaenge/<id>/x`, `/api/anhaenge/<id>/`, prozentkodiert, in Großbuchstaben | im Protokoll nur 6 Zeichen der ID (Regel 2) | L2 | B0432, B0464 | ✗ `src/http/anfrageprotokoll.rs:63` schreibt ohne Routenmuster den ganzen dekodierten Pfad |
  | M19 | ID und Name in Meldungen | `WARNING anhang` (Datei fehlt), `ERROR senden`, `WARNING laden`, `ERROR anhang` | Datei nur als `<hhmmssmmm>-<n>`, nie ASCII-Teil, Anzeigename oder ID (Regel 3) | L1, L2 | B0440, B0442, B0446, B0464 | — |
  | M20 | Reservierung halten | „Anmelden 9.437.184, 20 min alle 50 s PUT nach 1 Byte geschlossen“ | „spätestens nach der absoluten Lebensdauer 404 `abgelaufen`, reserviert 0“ (78 s) | L1, L2 | B0425, B0428, B0429 | — |
  | M21 | Eingang sperren | „127.0.0.2 und .3 je 4 Kennungen × 4 Anmeldungen à 1 Byte erneuert“ | „beta@127.0.0.1 Anmelden 201, PUT 200; Kontrolle mit einer vollen Herkunft“ (K2) | L2 | B0425, B0427, B0429 | — |
  | M22 | Halten sichtbar | wie M7/M20 | 507 `quote` nennt `reserviert`; Episode `Reservierung verfallen ohne Hochladen`; Health `reserviert` | L2 | B0427, B0432, B0463 | — |
  | M23 | Wettläufe im Eingang | dieselbe ID in zwei Channels zugleich; `DELETE` gegen Senden; `DELETE` vor `rename`; Ablauf während Senden; zwei `PUT` | genau ein Gewinner, Verlierer 422/204/409 nach ADR-0046 Ergänzung Regel 5; Invariante hält; keine Waise im Eingang | L1, L2 | B0425, B0429, B0430, B0440 | — |
  | M24 | Schreibfehler | `ENOSPC`/`EIO` beim Stück (Attrappe) | Abbruch, `.teil` weg, 507 `platte` bzw. 500 (ADR-0046 Ergänzung Regel 8) | L1 | B0429 | — |
  | M25 | `kill -9` | während `PUT`, zwischen `rename` und Satz | nach dem Start `.eingang/` leer, kein Satz ohne Datei, Waise zählt an der Quote (QA2) | L2 | B0431, B0440, B0442 | — |
  | M26 | `.eingang` | Symlink auf einen fremden Ordner, reguläre Datei | Exit 2, nichts gelöscht (ADR-0046 Ergänzung Regel 7) | L2 | B0431 | — |
  | M27 | Maße-Täuschung, Dekompressionsbombe | JPEG mit `FF C0` in einem APPn-Segment, `SOFn` erst hinter falscher Segmentlänge, PNG ohne `IHDR` als ersten Chunk, Breite oder Höhe 0, 65.535 × 65.535 | Maße nur aus dem über die Längen erreichten Segment (Regel 4); 0 → keine Maße, keine Vorschau; über 50 MP → keine Vorschau | L1 | B0433 | — |
  | M28 | `Content-Disposition` | Anzeigename mit `;`, `'`, `%`, `*`, Leerzeichen, Umlauten, Zeichen außerhalb der BMP | `filename*` nach RFC 8187 (alles außer `attr-char` prozentkodiert), `filename` ASCII-Ersatz in Anführungszeichen, Kopfzeile parsebar | L1, L2 | B0445, B0446 | — |
  | M29 | Namens-Täuschung (optional) | Doppelendung, viele Leerzeichen vor `.exe`, Homoglyphen | Anzeigename bleibt (erlaubt); die Art entscheidet der Inhalt, nie inline außer Tabelle | L1 | B0424, B0436 | — |
  | M30 | Entfallen und Cache | `If-None-Match: "<id>"` nach Entfallen des Tags | 404, nicht 304 (Regel 5) | L2 | B0446 | — |
  | M31 | Range erweitert | `bytes=-n` mit n > Länge; Einheit `Bytes`/`items`; `If-Range` mit `W/"<id>"` und als Datum; zwei `Range`-Kopfzeilen; `Range` auf `HEAD` und 404; 1.000 Bereichsanfragen nacheinander auf einer Verbindung | 206 `0–(Länge−1)` bzw. 200 ganz bzw. 404 nach Regel 5; offene Deskriptoren danach wie davor | L1, L2 | B0444, B0446, B0447 | — |
  | M32 | Rohkörper | `Content-Length` größer als gesendet; doppelt (gleich, ungleich); `+5`, `-5`; `Transfer-Encoding: chunked` + `Content-Length`; `Transfer-Encoding: identity`; HTTP/1.0; `Expect: 100-continue` bei Abweisung; Bytes nach dem Körper | Abweisung 400/411/422 **ohne** `100 Continue`, mit `Connection: close`; nie eine zweite Anfrage aus Körperbytes; zu wenig Bytes → Stückfrist | L2 | B0429 | — (hyper-Verhalten zu proben) |
  | M33 | Methode und Pfad | `X-HTTP-Method-Override`, Methode klein geschrieben, `..%2f`, `%00`, ID mit Großbuchstaben | 405 bzw. 404; die ID ist nie ein Dateiname (nur Schlüssel des Verzeichnisses) | L2 | B0429, B0446 | — |
  | M34 | CSRF und Rebinding | `POST`/`PUT`/`DELETE /api/anhaenge` mit fremdem `Origin`, `Sec-Fetch-Site: cross-site`, `text/plain`-Formular, fremdem Host; `OPTIONS`-Vorabfrage | 403 ohne Wirkung; `OPTIONS` ohne `Access-Control-Allow-*` | L2 | B0427, B0429, B0430, B0446 | — |
  | M35–M37 | — | im Gate-Bericht auf der Karte nicht enthalten — Nummern frei | — | — | — | — |
  | M38 | Gebundene ID | ein Dritter liest die gebundene ID im Channel und sendet, `PUT`et oder `DELETE`t sie | 422 `anhang`/`unbekannt`, 404 `unbekannt`, 204 ohne Wirkung; gebundene Datei unverändert | L2 | B0429, B0430, B0440 | — |
  | M39 | Überschreiben | Ziel des `rename` existiert schon (Waise, untergeschobene Datei) in `anhaenge/<tag>/` oder `.eingang/` | nie überschrieben; Rückbau, 500 (ADR-0046 Ergänzung Regel 6) | L1, L2 | B0429, B0440 | — |

  **Regeln, die daraus folgen:**

  1. **CSP-Weiche nach Pfad in `sicherheitsheader_setzen`** (ersetzt in Regel 3 „zusätzlich zu ADR-0014 Regel 5“; M14): die eine Stelle, die heute jede Antwort mit der App-CSP überschreibt (`src/http/grenzschutz.rs:341–361`, `kopf.insert`), wählt die CSP nach dem Pfad — ist er, roh oder prozent-dekodiert, `/anhaenge` oder beginnt er mit `/anhaenge/`, gilt `sandbox; default-src 'none'`, sonst die App-CSP. Alle anderen Header des Satzes bleiben gleich. Der Handler der Route setzt keine CSP; die Weiche greift damit für jeden Status und jede Methode, auch für Antworten des Grenzschutzes (403) und des Routers (404, 405). Durchsetzung: M14 je Status; Mutante „Weiche fehlt“ rot (heute rot).
  2. **ID kürzen nach dem Dekodieren, auch ohne Routenmuster** (ergänzt Regel 8; M18): das Anfrageprotokoll kürzt nach `prozent_dekodiert` in jedem Pfad jedes Segment, das mit 26 Zeichen aus `[A-Za-z2-7]` beginnt (Groß-/Kleinschreibung egal — eine groß geschriebene ID ist die ID), auf seine ersten 6 Zeichen und `…`. Gilt für jeden Pfad, nicht nur für erkannte Routen; ein langer Channelname aus diesen Zeichen erscheint auf Debug ebenfalls gekürzt — hingenommen.
  3. **Datei im Protokoll nur als Präfix** (ergänzt Regel 1, Regel 8 und [ADR-0046](0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) Regel 6; M19): wo eine Meldung eine gespeicherte Anhangdatei nennt, steht nur `<hhmmssmmm>-<n>` mit Gespräch und Tag — der ASCII-Teil trägt den Anzeigenamen und bleibt draußen; Dateien des Eingangs (`.eingang/<id>`) erscheinen als ID-Präfix nach Regel 2.
  4. **Maße aus dem dekodierten Segment** (ergänzt Frage 3 und A00018; M27): Maße stammen nur aus der Struktur, die der Erkenner über die Längenkette tatsächlich erreicht — PNG aus `IHDR` als erstem Chunk, JPEG aus dem ersten `SOFn`, das über die Segmentlängen ab `SOI` erreicht wird (nie eine Suche nach Markerbytes), GIF aus dem Kopf, WebP aus dem Chunk, den der Kopf nennt. Breite oder Höhe 0 → keine Maße; die Art bleibt, die Anzeige ist eine Datei-Zeile.
  5. **Ausliefern: Existenz vor Cache, Range-Ränder** (ergänzt Regeln 3 und 4; M30, M31): erst `finden`, dann `If-None-Match` — eine entfallene ID antwortet 404, auch mit passendem ETag. `bytes=-n` mit n größer als die Länge → 206 über die ganze Datei (RFC 9110, Suffix länger als die Darstellung); `If-Range` mit schwachem ETag oder als Datum → 200 ganz; mehr als eine `Range`-Kopfzeile → übergangen, 200 ganz; `HEAD` übergeht `Range` → 200-Kopf; 404 bleibt 404.
  6. **Härtung `script-src` — geprüft, nicht übernommen:** `script-src` auf die Pfade der App-Skripte zu begrenzen, bräuchte eine Quelle mit Host und Pfad je Anfrage statt `'self'`; die Schutzwirkung trägt heute `nosniff` mit Anhang-Arten, die nie Skript- oder Stilarten sind — M15 ist der Test dafür. Wird M15 rot, ist die Begrenzung (oder Hashes) die nächste Ergänzung.

  **Offen, nicht entschieden — Fragen an den Betreiber** (Stellen markiert, es gilt bis zur Antwort diese ADR): **F2** direkt aufgerufener Medienlink spielt sofort (Frage 7, M1); **F3** Text über 16 MiB nur als Download (Frage 3); **F4 (neu, aus dem Gate)** Bilder und Videos werden unverändert ausgeliefert — EXIF-Daten wie GPS-Ort, Kameramodell und Aufnahmezeit gehen an jeden mit dem Link (B4); sollen sie beim Hochladen entfernt werden, oder genügt ein Hinweis in der Oberfläche (R00030)? Bis zur Antwort: unverändert, wie B3 („Vorschau aus dem Original“).

- **2026-10-29 (Karte #9 Betreiber F1; [ADR-0046](0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) Ergänzung „Karte #9 Betreiber F1“) — ein verdrängter Anhang: 410 statt 404, „nicht mehr vorhanden“ im Verlauf, die Nachricht bleibt; Matrixzeilen M40–M46:** Entscheider: Claude (autonom, architekt, Auftrag des Directors). Status: Akzeptiert.

  **Antwort der Route** — Varianten:

  | | Antwort auf eine verdrängte ID | Stärke | Schwäche |
  |---|---|---|---|
  | **W1 (gewählt)** | **410 Gone** `text/plain` „Dieser Anhang wurde verdrängt: der Platz für Anhänge wurde für neuere Dateien gebraucht. Die Nachricht bleibt.“ | Agent und Mensch erfahren den Grund (nicht „kaputt“, nicht „abgelaufen“); 410 heißt „dauerhaft weg, nicht wiederholen“; kein Orakel: nur wer die 128-Bit-ID kennt, sieht den Unterschied | ein Status mehr; der Unterschied verrät dem Linkbesitzer, dass die Nachricht noch in der Frist liegt (wer den Link hat, durfte sie lesen — B4) |
  | W2 | derselbe 404 wie entfallen | nichts Neues | „verdrängt“ und „Datei fehlt“ (Defekt, `WARNING`) sind nicht zu unterscheiden; ein Agent kann nicht zwischen Fehler und Regel unterscheiden — verworfen |
  | W3 | 200 mit Platzhalterbild | `<img>` zeigt etwas | lügt über die Art; ein Download lädt den Platzhalter unter dem echten Namen — verworfen |

  **Regeln:**
  1. `GET`/`HEAD /anhaenge/{id}`: Eintrag `verdraengt` → **410**, Kopfzeilen wie jede Antwort der Route (CSP-Weiche, `nosniff`), `Cache-Control: no-store`; **vor** `If-None-Match` und `Range` (Existenz vor Cache, Ergänzung „Karte #9 Gate“ Regel 5) — ein ETag aus der Zeit vor dem Verdrängen ergibt 410, nie 304. Entfallen bleibt 404, Datei fehlt bleibt 404 mit `WARNING anhang`. Ein Abruf, der vor dem Kürzen begann, liefert seine Antwort mit dem offenen Deskriptor zu Ende oder bricht mit dem Kürzen ab (Länge stimmt dann nicht) — hingenommen, der Client sieht einen Abbruch.
  2. **Metadaten** (Frage 5): jeder Eintrag in `anhaenge` trägt zusätzlich `zustand: "vorhanden" | "verdraengt"`; bei `verdraengt` bleiben `name`, `bytes`, `art` (was gesendet wurde), `url` bleibt (antwortet 410). Gilt für Verlauf, Strom, Warten, Senden-Antwort.
  3. **Kein Live-Ereignis** beim Verdrängen (wie ADR-0044 Regel 10): offene Tabs erfahren es beim nächsten Abruf. Die Oberfläche ([A00005](../Architektur/A00005-weboberflaeche.md), R00030): ein Anhang mit `zustand: "verdraengt"` erscheint als **Datei-Zeile** mit Name, Größe und „Nicht mehr vorhanden — der Platz wurde für neuere Anhänge gebraucht“, **ohne** „Öffnen“, „Laden“, Vorschau oder Player (kein reservierter Bildplatz); ein Bild oder Player, dessen Abruf scheitert (`error` auf `<img>`, `<video>`, `<audio>`), wird durch dieselbe Zeile ersetzt — die Höhe ändert sich dabei einmal (L4 für diesen Fall hingenommen, Hinweis an den `ux-reviewer`). Text und Absender der Nachricht bleiben unverändert.
  4. **Direktgespräche**: verdrängt wird über alle Gespräche; die Antwort 410 verrät nichts über das Gespräch.

  **Neue Matrixzeilen** (für den `security-reviewer`; Gate an R00029):

  | # | Fläche | Angriff | erwartet | Linie |
  |---|---|---|---|---|
  | M40 | Verdrängungsflut | eine Herkunft lädt `Q` Bytes in vielen Dateien hoch und sendet sie | nur gebundene Anhänge, ältester zuerst; nie ein Satz, nie ein Eintrag des Eingangs; je Hochladung nie mehr verdrängt als geschrieben + 64 MiB; Episode `Anhänge verdrängt` mit dieser Herkunft; Kontrollpaar K1 | L1, L2 |
  | M41 | Abbruch nach Verdrängen | Anmelden `Q`, 3 Abschnitte schreiben, Verbindung schließen | Verdrängtes bleibt verdrängt (≤ 3 × 64 MiB), `.teil` weg, `beansprucht` 0, Invarianten halten | L1, L2 |
  | M42 | Anspruch hält Platz | Anmelden `Q`, alle 9 s 64 KiB, nach der Gesamtfrist neu `PUT` | andere Anmeldungen 507 `hochladungen` mit `Retry-After`, **Text 201**; spätestens am Lebensende frei; sichtbar in Health `reserviert` — bewertet der `security-reviewer`: Restrisiko annehmbar oder Folgeoption E2 (ADR-0046) | L2 |
  | M43 | Wettläufe | Verdrängen gegen Entfallen desselben Tages, gegen `GET` mit Range, gegen Binden in einem anderen Gespräch, zwei Hochladungen wählen Opfer zugleich | jeder Anhang genau einmal abgezogen; zwei Hochladungen nie dasselbe Opfer; keine Verklemmung (Verdrängen nimmt keine Gesprächssperre) | L1 |
  | M44 | Orakel | ausgedachte ID, ID eines verdrängten Anhangs aus einem Direktgespräch | 404 bzw. 410 ohne Gespräch, Tag oder Namen | L2 |
  | M45 | Kürzen und Symlink | Opferdatei vor dem Kürzen durch einen Symlink auf eine fremde Datei ersetzt; Tag-Ordner durch Symlink ersetzt | fremde Datei unverändert (`O_NOFOLLOW`), `ERROR anhang`, Bytes als Waise gezählt | L1, L2 |
  | M46 | Quote gesenkt | Neustart mit `--anhangquote` unter der Belegung | Start verdrängt nichts, `WARNING`; die nächste Hochladung verdrängt den Überhang plus Bedarf | L2 |

  **Einordnung des Risikos** (ehrlich): neu ist eine zweite Löschhandlung für Anhänge, die jeder Teilnehmer mit Sitzung auslösen kann — gewollt oder versehentlich, über alle Channels und Direktgespräche, ohne Bestätigung (bis F5). Gemessen am Rahmen der Vision (LAN, keine Anmeldung, keine Rechte; Lastprofil „Team-LAN“ mit 20 Menschen und 50 Agenten, die einander vertrauen) ist die Fähigkeit nicht grundsätzlich neu: jeder kann schon heute die Frist eines Channels auf 1 Tag verkürzen und damit dessen ältere Nachrichten samt Anhängen löschen ([ADR-0031](0031-frist-setzen-put-api-channels-name-frist-jeder-mit-sitzung-bestaetigung-beim-verkuerzen.md), mit Bestätigung). Anders sind drei Dinge: es trifft auch Direktgespräche anderer, es braucht keine Bestätigung, und der wahrscheinliche Fall ist ein Versehen (ein großes Abbild). Daher die Empfehlung in F5 (Bestätigung) und als **Folgerichtung an den `security-reviewer`**: M40–M46 als Gate an R00029 fahren; M42 mit Zahlen bewerten (Kosten ≈ 6,4 KiB/s halten 20 GiB bis ≈ 11,4 h) und entscheiden, ob E2 (Anspruch je Herkunft ≤ 50 %) nötig ist; prüfen, ob die Kopplung „Verdrängen nur für geschriebene Bytes“ (ADR-0046 Regel 4) im echten Programm hält (Mutante „Verdrängen bei der Anmeldung“).

- **2026-10-29 (Karte #9 Betreiber F2, F3) — beantwortet:** F2 „hingenommen“ — ein direkt aufgerufener Medienlink darf abspielen (M1 bleibt, M3 bleibt verworfen; Pre-Mortem 4 entfällt). F3 — Text über 16 MiB nur als Download, wie Frage 3. Beide Punkte unter „Offene Fragen“ sind damit erledigt.

- **2026-10-29 (Karte #9 Betreiber F4) — Ortsangaben und Metadaten werden beim Fertigstellen getilgt, an Ort und Stelle bei gleicher Länge; Orientierung bleibt; ein Format im Geltungsbereich, das sich nicht sicher tilgen lässt, wird abgewiesen:** Entscheider: Claude (autonom, architekt, Auftrag des Directors); Richtung vom Betreiber. Status: **Akzeptiert** für die Richtung und die Regeln; die Format-Leser sind nicht geprobt — Erste Tests unten sind ihr Beleg (fällt ein Format im Bau durch, ist das eine Ergänzung dieser Regel, nicht eine neue Richtung).

  **Anlass.** Betreiber auf F4: „beim Hochladen entfernen, mindestens den GPS-Ort“; das Original muss nicht byte-gleich bleiben. Ersetzt „Bis zur Antwort: unverändert“ aus der Ergänzung „Karte #9 Gate“.

  **Szenario mit Maß — QS7 Kein Ort im Anhang:** ein Handyfoto (JPEG oder HEIC) und ein Handyvideo (MP4/MOV) mit GPS-Ort werden hochgeladen und gesendet → in den ausgelieferten Bytes findet `exiftool` keine GPS-, XMP- oder Ortsangabe; das Foto steht im Browser richtig herum; Länge, `Content-Length` und Range-Antworten stimmen mit `bytes` der Anmeldung überein; ein 2-GiB-Video braucht für das Tilgen ≤ 2 s zusätzlich (Lesen der Kopf-Boxen, nicht der Daten) — gerechnet, im Messwert QA3 zu prüfen.

  **Frage 1 — wie wird getilgt?**

  | | Verfahren | Stärke | Schwäche |
  |---|---|---|---|
  | **T1 (gewählt)** | **an Ort und Stelle unkenntlich machen, gleiche Länge**: Metadatenblöcke werden mit Nullen überschrieben und als unbekannter Block markiert, wo das Format das verlangt (Regel 3); kein Byte verschoben | keine Kopie (kein zweiter Platz gegen die Anhangquote, keine Kopierzeit für große Dateien); Länge bleibt = `bytes` der Anmeldung → Metazeile, `Content-Length`, Zählmaß, Range, ETag (`"<id>"`) unverändert; Versatztabellen (MP4 `stco`/`co64`, HEIC `iloc`) bleiben gültig | die Datei ist nicht minimal; wer genau hinsieht, sieht, dass dort Metadaten waren (Größe des genullten Blocks) |
  | T2 | Blöcke herausschneiden, in eine neue Datei kopieren | saubere, kleinere Datei | Länge ändert sich nach der Anmeldung; doppelter Platz während der Kopie (verletzt die Anhangquote, B2); bei MP4 müssten alle Versatztabellen umgerechnet werden — verworfen |
  | T3 | Bild neu kodieren | entfernt alles sicher | Bild- und Videodecoder fremder Dateien im Server (ADR-0047 V3 verworfen aus demselben Grund), Qualitätsverlust, Dekompressionsbomben — verworfen |
  | T4 | im Browser vor dem Hochladen | Server bleibt einfach | Agenten und `curl` umgehen es; der Server muss es ohnehin — verworfen |

  **Frage 2 — was wird getilgt?**

  | | Umfang | Stärke | Schwäche |
  |---|---|---|---|
  | M1 | nur GPS (GPS-IFD im EXIF) | Kamera, Zeit bleiben | GPS steckt auch in XMP, IPTC, Herstellerblöcken (MakerNote) und in Video-Boxen; chirurgisches Ändern der IFD-Verweise ist fehleranfällig — verworfen für Bilder |
  | **M2 (gewählt)** | **Bilder: alle beschreibenden Metadaten** (EXIF samt GPS, MakerNote, Kamera, Seriennummer, Aufnahmezeit; XMP; IPTC; Kommentare; Textblöcke) — **die Orientierung bleibt** als minimales EXIF mit dem einen Eintrag `Orientation`; **Videos: nur Ortsangaben** | Betreiber „mindestens den GPS-Ort“ sicher erfüllt, ohne IFD-Chirurgie; Fotos stehen richtig herum; Videos bleiben unangetastet bis auf eng begrenzte Boxen | Aufnahmezeit und Kamera gehen bei Bildern verloren (auch dem, der sie wollte) |
  | M3 | alles, auch Orientierung und Farbprofil | einfachste Regel | Hochformatfotos liegen quer; Farben falsch ohne ICC — verworfen |

  **Frage 3 — Datei im Geltungsbereich, die sich nicht sicher tilgen lässt** (Längen widersprechen sich, Verschachtelung zu tief, Exif-Item nicht als Bytebereich der Datei abgelegt):
  - **D1 (gewählt): abweisen** — Antwort des `PUT` nach dem letzten Byte **422** `{fehler, regel: "metadaten", grund}`, Text „Nicht hochgeladen: die Datei ist beschädigt — Ortsangaben ließen sich nicht sicher entfernen.“; Teildatei gelöscht, Eintrag verworfen, Anspruch frei (wie `DELETE`). Stärke: kein Ort geht durch, nur weil die Datei kaputt ist; der Körper ist ganz gelesen, die JSON-Antwort kommt an. Schwäche: der Fehler zeigt sich erst nach dem Hochladen (selten: Kameras schreiben gültige Dateien).
  - D2: unverändert speichern — Ort kann durchgehen; widerspricht F4 — verworfen. D3: als `application/octet-stream` unverändert — der Download trägt den Ort weiter — verworfen.

  **Frage 4 — eigene Leser oder Crate?** **Eigene Leser ohne neue Abhängigkeit (gewählt)** in [A00018 Anhangprüfung](../Architektur/A00018-anhangpruefung.md): sie lesen nur Blockköpfe (Typ, Länge) und schreiben Nullen an Positionen — ein paar hundert Zeilen je Familie, mit derselben Disziplin wie der Erkenner (Längen mit Überlaufprüfung, jede Schleife schreitet fort, Tiefe begrenzt). Verworfen: Crates zum Umschreiben von Bildmetadaten (sie kopieren statt an Ort und Stelle zu arbeiten → T2; decken HEIC und MP4-Ortsboxen nicht ab; eine zweite Parser-Fläche fremder Herkunft bräuchte eine `dependency-probe`).

  **Regeln:**
  1. **Wo im Ablauf** (ergänzt [ADR-0046](0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) Regel 2.6): nach dem letzten Byte, **vor** `rename` von `.teil` nach `.eingang/<id>`, im Blocking-Thread der Hochladung unter ihrer Erlaubnis der Hochladebank — keine Sperre, nicht die Werkbank (B2 „non-blocking“). Gelesen und geschrieben wird per Position (`pread`/`pwrite`) über den Port der Logablage; die Länge ändert sich nie, also ändert sich am Zählmaß, an Anspruch und Belegung nichts. Ohne `fsync` wie alles andere. Dauer zählt nicht in die Gesamtfrist des `PUT`; begrenzt durch Regel 4.
  2. **Geltungsbereich nach Struktur, nicht nach der Art der Tabelle** — auch Formate, die nur als Download ausgeliefert werden, tragen Orte:
     - **JPEG** (`FF D8 FF`): Segmente ab `SOI` bis `SOS` über die Längenkette; `APP1` mit `Exif\0\0` → ersetzt durch ein minimales EXIF (TIFF-Kopf, ein IFD mit nur `Orientation`, wenn das Original sie trug und ≠ 1), Rest des Segments genullt; `APP1` mit XMP-Kennung (auch Extended XMP) und `APP13` (Photoshop/IPTC) → Kennung und Inhalt genullt (ein `APPn` mit unbekannter Kennung wird übergangen); `COM` → Inhalt genullt. Bleiben: `APP0` (JFIF), `APP2` `ICC_PROFILE`, `APP14` Adobe, alles ab `SOS`.
     - **PNG**: Chunks bis `IEND`; `eXIf` → minimales EXIF wie oben, Rest genullt; `tEXt`, `zTXt`, `iTXt` (auch XMP), `tIME` → Typ auf den privaten, nicht kritischen Typ `lcAx` umgestellt, Inhalt genullt, CRC neu. Bleiben: kritische Chunks, Farbe (`iCCP`, `sRGB`, `gAMA`, `cHRM`, `sBIT`), `pHYs`, `tRNS`, `bKGD`, APNG (`acTL`, `fcTL`, `fdAT`).
     - **WebP** (`RIFF….WEBP`): Chunk `EXIF` → minimales EXIF, Rest genullt; `XMP ` → Typ `JUNK`, Inhalt genullt, XMP-Bit in `VP8X` gelöscht.
     - **GIF**: Kommentar-Erweiterung (`21 FE`) und Anwendungs-Erweiterung `XMP DataXMP` → Unterblöcke genullt (Längenbytes bleiben); `NETSCAPE2.0` bleibt.
     - **ISOBMFF** (gültige `ftyp`-Box am Anfang, jede Marke — MP4, MOV, M4A, 3GP, HEIC, HEIF, AVIF): Boxen der obersten Ebene über ihre Längen (`size` 1 → 64-Bit, `size` 0 → bis Dateiende); in `moov` (ganz gelesen, Regel 4): `udta/©xyz`, `udta/loci` und in `meta` (`keys`/`ilst`) die Werte zu Schlüsseln, die `location` enthalten (`com.apple.quicktime.location.ISO6709` u. a.) → Inhalt genullt; im `meta` der obersten Ebene (HEIC/AVIF): Items vom Typ `Exif` → minimales EXIF an der Stelle nach dem `exif_tiff_header_offset`, Rest genullt; Items `mime` mit XMP-Inhaltstyp → genullt — nur über `iloc`-Ausdehnungen mit `construction_method` 0 innerhalb der Datei.
     - **Alles andere unverändert** (WebM/Matroska, Ogg, MP3, PDF, Text, Archive, Office): keine Kameraorte in den Formaten, die Telefone und Kameras schreiben; ein Foto **in** einem ZIP, PDF oder Dokument behält seinen Ort — benannte Grenze, Hinweis in der Anleitung (I0019) und an den `ux-reviewer`.
  3. **Kennzeichnung**: wo das Format einen Block nach seinem Typ deutet (PNG, WebP, JPEG-Kennung), wird der Typ bzw. die Kennung mit unkenntlich gemacht, damit kein Leser genullte Daten als EXIF oder XMP deutet; sonst genügen Nullen.
  4. **Grenzen der Leser** (Robustheit, M47–M50): Längen als u64 mit Überlaufprüfung, jeder Block muss ganz in der Datei liegen; Verschachtelung ≤ 16; `moov` bzw. `meta` ≤ 64 MiB (sonst D1); höchstens 100.000 Blöcke je Datei; jede Schleife schreitet um mindestens einen Blockkopf fort. Verletzung → D1 (422 `metadaten`). Die Art (Erkenner) und die Maße bleiben unberührt — die Leser ändern nie `SOFn`, `IHDR`, `VP8*`, Bilddaten oder Medien-Boxen außerhalb der genannten.
  5. **Länge und ETag**: Länge = `bytes` der Anmeldung (Prüfung nach dem Tilgen, Mutante „Länge ändert sich“); ETag bleibt `"<id>"` — die Datei wird nur einmal und vor ihrer ersten Auslieferung verändert.
  6. **Protokoll**: `DEBUG anhang Metadaten getilgt` mit `art`, `bloecke` (Zahl) und `ort` (`ja`/`nein`, ob eine Ortsangabe dabei war) — nie Inhalte, Koordinaten, Kameradaten (wie M19). Kein Health-Feld (nichts zu entscheiden). 422 `metadaten` zählt in die Episode `WARNING anhang Hochladen abgewiesen: Metadaten nicht tilgbar` je Herkunft ([ADR-0042](0042-stufen-ereigniskatalog-episoden-und-tagesgrenze-des-betriebsprotokolls.md) Ergänzung „Karte #9 Betreiber F1/F4“).
  7. **Hinweis in der Oberfläche** (R00030): keiner nötig — getilgt wird immer; die Anleitung (I0019) nennt, was getilgt wird und was nicht (Regel 2, letzter Punkt).

  **Neue Matrixzeilen** (für den `security-reviewer`):

  | # | Fläche | Angriff | erwartet | Linie |
  |---|---|---|---|---|
  | M47 | Präparierte Metadaten | GPS nur in XMP; nur in MakerNote; nur in IPTC; zwei `APP1`-Exif; Extended XMP über mehrere Segmente; GPS in PNG `iTXt` komprimiert; HEIC mit zwei Exif-Items; MOV mit `©xyz` in `udta` und in `meta/keys` | nach dem Tilgen findet ein Leser (`exiftool`) keine Ortsangabe; Bild- und Mediendaten byte-gleich | L1, L2 |
  | M48 | Parser-Robustheit | Fuzz der Längen: Segmentlänge 0/1/über das Dateiende, Box `size` 0, 1 mit 2⁶⁴−1, 8 mit Kindern, Tiefe 10.000, `iloc` mit Versatz außerhalb der Datei oder `construction_method` 1/2, 10⁶ leere Chunks, CRC falsch | keine Panik, keine Schleife, kein Schreiben außerhalb der Datei; 422 `metadaten` oder getilgt; Dauer ≤ 2 s | L1 |
  | M49 | Täuschung der Grenzen | Metadaten **nach** `SOS` (JPEG) bzw. nach `IEND` (PNG), Polyglot mit zweitem `ftyp` | was hinter dem Ende liegt, wird nicht als Block gedeutet; die Art bleibt laut Erkenner; ein Ort hinter dem Ende ist Daten, kein Metadatum — benannt, nicht getilgt | L1 |
  | M50 | Gleichzeitigkeit und Länge | Tilgen einer 2-GiB-MP4 mit `moov` am Ende, während andere senden; Range-Abruf nach dem Binden | Senden p90 wie QA3; Länge = Anmeldung; `Content-Range` stimmt | L2 |

  **Erste Tests:**
  - Unit (A00018, rein auf Byte-Puffern): JPEG mit `APP1` Exif (GPS-IFD, `Model`, `DateTimeOriginal`, `Orientation` 6), `APP1` XMP mit `exif:GPSLatitude`, `APP13`, `COM`, `APP2` ICC → gleiche Länge; das neue `APP1` enthält genau `Orientation` 6; XMP-, IPTC- und `COM`-Inhalte sind Nullen; ICC, `SOF0`, Scan-Daten byte-gleich; derselbe Puffer ohne `Orientation` → kein IFD-Eintrag. PNG mit `eXIf` und `iTXt` XMP → `iTXt` als `lcAx` mit Nullen und gültiger CRC; `IHDR`/`IDAT` gleich. WebP mit `EXIF`/`XMP ` → `JUNK`, XMP-Bit aus. MP4 mit `moov/udta/©xyz` „+52.5200+013.4050/“ → genullt, `stco` und `mdat` gleich. HEIC mit Exif-Item über `iloc` → minimales EXIF an der Stelle. Abgeschnittenes JPEG (Segmentlänge über das Ende) → `metadaten`. Mutanten „nur GPS-IFD“, „Orientierung verworfen“, „ICC getilgt“, „Länge geändert“, „Box `size` 1 nicht gelesen“ werden rot.
  - Integration (echtes Programm): Handy-JPEG mit GPS hochladen, senden, `GET` → kein `GPS`/`xmp`-Text in den Bytes, Länge = Anmeldung, `image/jpeg` mit Maßen wie vorher; beschädigtes JPEG → `PUT` 422 `metadaten`, kein `.teil`, `reserviert` 0.
  - E2E (Chromium, R00030): das Hochformatfoto mit `Orientation` 6 erscheint in der Vorschau hochkant.

  **Folgen:** R00029-Kriterium „Bilder unverändert“ (nach F4) **entfällt** und wird ersetzt durch QS7 mit den Tests oben; „Teilabruf über das Programm“ vergleicht mit der getilgten Datei, wo das Format im Geltungsbereich liegt (WebM nicht). Die Frage F4 unter „Offen“ ist beantwortet.

- **2026-10-29 (Karte #9 Betreiber F1′; Karte #45 D0006; [ADR-0046](0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) Ergänzung „Karte #9 Betreiber F1′“) — ein gelöschter Anhang: 410, `zustand: "geloescht"`, Ereignis `anhaenge` an offene Sichten; Lesesicht des Laufwerks als Verzeichnis je Gespräch mit Sortierung und Paging; Matrixzeilen M40′–M46′, M51–M53:** Entscheider: Claude (autonom, architekt, Auftrag des Directors); Richtung vom Betreiber. Status: **Akzeptiert** für die Regeln 1–4 (Auslieferung, Metadaten, Ereignis, Oberfläche — Bau mit Karte #9 bzw. #43); **Vorgeschlagen** für Regel 5 (Lesesicht, Skizze für Karte #45; offen F8–F14).

  **Anlass.** Betreiber zu F1 neu (keine Verdrängung, Hochladen bei voller Quote blockiert, Sammellöschung im Laufwerk), zu den Laufwerk-Richtungsfragen 1 („Jeder darf jeden Anhang löschen“), 2 (eine Sicht über alle Channels, „nach Channels geordnet … was zu welcher Nachricht gehört hat“), 3 („zuerst nur Verzeichnisse mit Sortierung nach Name, Größe und Alter; Suche und Filter später als Ausbaustufe“) und 4, wörtlich: „Die Chat-Struktur ist praktisch über Verzeichnisse abgebildet, ansonsten verhält es sich wie ein Web-Laufwerk“ (gewählt war: Dateien auch direkt ins Laufwerk eines Channels ablegen, ohne Nachricht). **Zurückgenommen, bevor gebaut**: aus der Ergänzung „Karte #9 Betreiber F1“ die Antwort W1 (410 „verdrängt“), die Regeln 1–4 dort im Wortlaut „verdrängt“, Regel 3 „Kein Live-Ereignis“ und die Matrixzeilen M40–M46 samt „Einordnung des Risikos“.

  **Regeln** (Auslieferung und Verlauf):

  1. **Abruf** `GET`/`HEAD /anhaenge/{id}` eines Eintrags `geloescht` → **410** `text/plain` „Dieser Anhang wurde gelöscht. Die Nachricht bleibt.“, Kopfzeilen wie jede Antwort der Route (CSP-Weiche, `nosniff`), `Cache-Control: no-store`; geprüft **vor** `If-None-Match` und `Range` — ein ETag aus der Zeit davor ergibt 410, nie 304. Entfallen bleibt 404; Datei fehlt bleibt 404 mit `WARNING anhang`. Eine Antwort, die vor dem Kürzen begann, bricht mit ihm ab (Länge stimmt nicht) — hingenommen. Verworfen wie in F1: derselbe 404 (Regel und Defekt nicht unterscheidbar), 200 mit Platzhalter (lügt über die Art).
  2. **Metadaten** (Frage 5): jeder Eintrag in `anhaenge` trägt `zustand: "vorhanden" | "geloescht"`; bei `geloescht` bleiben `name`, `bytes`, `art`, `url` (antwortet 410). Gilt für Verlauf, Strom, Warten, Senden-Antwort.
  3. **Ereignis `anhaenge`** (ersetzt F1 Regel 3; neues Thema im Strom nach [ADR-0023](0023-channels-als-zweites-thema-im-ereignisstrom.md), Bus `Ereignis::Anhaenge`): nach Schritt (c) des Löschens ([ADR-0046](0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) Ergänzung „F1′“ Regel 5) je betroffenem Gespräch ein Ereignis `{"art":"anhaenge","channel":"<name>","geloescht":[{"id":"…","zeit":"<Satzzeit>"}]}` — `zeit` ist die Zeit des Satzes, an dem der Anhang hängt, damit der Tab die Nachricht findet. **Ohne Ordnungszahl und ohne Stand**: `geloescht` ist endgültig und idempotent, die Reihenfolge der Ereignisse ändert nichts. Direktgespräche nur an die Beteiligten (Filter „sichtbar für“, [A00011](../Architektur/A00011-live-kanal.md)). Nach `NeuerStand` oder Wiederverbinden holt der Tab den offenen Block nach ([ADR-0028](0028-nachrichten-live-als-drittes-thema-ordnung-ueber-die-nachrichtenzeit.md) Regel 6) und **übernimmt dabei `zustand` schon bekannter Nachrichten** (Ausnahme von „kleiner oder gleicher Zeit wird verworfen“, Regel 5 dort — nur das Feld `zustand` der Anhänge). Varianten: E-a eigenes Thema (gewählt — Stärke: „Live überall“, Laufwerk und Verlauf folgen ohne Neuladen; Schwäche: ein Thema mehr im Strom); E-b die Nachricht als „geändert“ neu senden (bricht die Regel, dass Nachrichten unveränderlich sind und nach Zeit zusammengeführt werden — verworfen); E-c kein Ereignis wie bei F1 (Laufwerk zeigte Gelöschtes weiter an, ein Klick ergäbe 410 — verworfen).
  4. **Oberfläche** (R00030, [A00005](../Architektur/A00005-weboberflaeche.md)): ein Anhang mit `zustand: "geloescht"` erscheint als Datei-Zeile mit Name, Größe und „Gelöscht“ (Wortlaut beim `ux-reviewer`), ohne „Öffnen“, „Laden“, Vorschau oder Player und ohne reservierten Bildplatz; trifft ein Ereignis `anhaenge` eine sichtbare Nachricht, wird Bild oder Player durch diese Zeile ersetzt (Höhe ändert sich einmal, L4 für diesen Fall hingenommen); ein `error` auf `<img>`, `<video>`, `<audio>` führt zur selben Zeile. Ein Lösch-Knopf im Verlauf ist nicht beauftragt — gelöscht wird im Laufwerk (Karte #45). Die 507 `voll` erscheint an der Datei mit dem Text aus ADR-0046 Ergänzung „F1′“ Regel 2.

  **Regel 5 — Lesesicht des Laufwerks (Vorgeschlagen, Skizze für Karte #45):**

  Lage: ein Verzeichnis je Channel bzw. Direktgespräch; darin die Anhänge von Nachrichten (mit Bezug zur Nachricht) und **Dateien ohne Nachricht** (Betreiber, Frage 4); sortierbar nach Name, Größe und Alter, auf- und absteigend, mit Paging; keine Suche, kein Filter (Ausbaustufe). „Wie ein Web-Laufwerk“ ist darüber hinaus nicht bestimmt — F9–F14.

  *Varianten der Liste:*

  | | Form | Stärke | Schwäche |
  |---|---|---|---|
  | L-a | eine flache Liste über alle Gespräche, nach Zeit geblättert | eine Route | widerspricht „nach Channels geordnet“ — verworfen |
  | L-b | ganzer Baum in einer Antwort | ein Aufruf | bis 327.680 Einträge — verworfen |
  | **L-c (vorgeschlagen)** | **zwei Ebenen**: Liste der Verzeichnisse, dann ein Verzeichnis sortiert und geblättert | wie ein Laufwerk; je Antwort beschränkt; Sortierung je Verzeichnis | drei geordnete Indizes je Gespräch im Speicher (A00016) |

  *Routen* (alle unter `grenzschutz` mit Sitzung, lesend, ohne Konto-Buchung wie der Verlauf):
  - `GET /api/anhaenge/platz` → `{hoechstens, belegt, gebunden, reserviert, frei, anhaenge}` — O(1) aus den Zählern von Gespräche (ersetzt die Form `verdraengbar`, `verdraengt_seit_start` aus ADR-0043 Ergänzung „F1“).
  - `GET /api/anhaenge/verzeichnisse` → `{verzeichnisse: [{channel, anhaenge, bytes, juengster}]}` für jedes **sichtbare** Gespräch, auch ohne Anhänge (ein leeres Verzeichnis ist ein Ziel für Dateien ohne Nachricht), sortiert nach Name; Summen je Gespräch hält das Verzeichnis (O(Gespräche)).
  - `GET /api/channels/{name}/anhaenge?sortierung=name|groesse|alter&richtung=auf|ab&n=1..200&nach=<cursor>` → `{anhaenge: [{id, name, bytes, art, url, zeit, nachricht: {zeit, von} | null}], weiter: <cursor> | null}` — nur `vorhanden`; Standard `alter`, `ab` (neueste zuerst). `zeit` ist die Zeit des Satzes (Alter); `nachricht` trägt den Bezug für den Sprung in den Verlauf, `null` bei Dateien ohne Nachricht. Sortierschlüssel: Name nach Unicode-Kleinschreibung, dann ID; Größe nach `bytes`, dann ID; Alter nach (Satzzeit, Sendefolge). Der Cursor ist undurchsichtig (Base32 aus Schlüssel und ID); ein Eintrag, der zwischen zwei Seiten gelöscht wird, verschiebt nichts. Unbekannte oder ungültige Parameter → 422 (abweisen, nicht umformen). Direktgespräche unter dem Pfad, den I0012 festlegt, nur für Beteiligte; ein Nichtbeteiligter erhält 404 wie für ein unbekanntes Gespräch.
  - **Sprung in den Verlauf**: über `nachricht.zeit`; die Adresse (Hash nach [ADR-0027](0027-channel-waehlen-adresse-als-hash-zuletzt-gewaehlt-je-tab.md) plus Zeit) und ein Block „um eine Zeit“ entscheidet Karte #45 mit R00030.
  - **Live im Laufwerk**: neue Anhänge aus dem Ereignis `nachricht` (trägt `anhaenge`), gelöschte aus `anhaenge`; Platz und Dateien ohne Nachricht — Entscheidung in Karte #45 (Vorschlag: `anhaenge` trägt `platz` mit, zusätzlich höchstens 1 Ereignis/s bei Änderung von G oder A).

  *Dateien ohne Nachricht — Grundlage, damit Karte #9 nicht wartet:* sie werden über dieselben Routen angemeldet und hochgeladen (Eingang, Anspruch, Tilgen nach F4) und an einen **Ablage-Satz** gebunden — ein Satz im Tageslog des Gesprächs mit `+ art: ablage`, ohne Text, mit derselben Metazeile `+ anhang:` ([ADR-0004](0004-logformat-kopfzeile-und-eingerueckter-text.md): unbekannte Werte bleiben erhalten). Er erscheint nie im Verlauf, nie in der Suche und nie im Ereignis `nachricht`. Stärke: Ablage, Quote, Zählmaß, Laden, Löschen (`POST /api/anhaenge/loeschen`), Abruf, Sichtbarkeit und Entfallen gelten unverändert — eine Datei ohne Nachricht ist ein Anhang ohne sichtbaren Satz. Schwäche: sie entfällt mit ihrem Tag (Frage F8). Verworfen: ein eigener Ordner `laufwerk/` je Gespräch ohne Satz (zweite Ablage, zweite Löschlogik, zweites Laden — nur nötig, wenn F8 Option 2 gewählt wird). **Für Karte #9 folgt daraus nur**: kein Feld, keine Route und kein Ereignis darf voraussetzen, dass hinter jedem Anhang eine sichtbare Nachricht steht — Löschen, Abruf und Ereignis arbeiten über die ID und die Satzzeit; das Verzeichnis (A00016) trägt je Eintrag die Satzzeit und den Absender. Die Route zum Binden ohne Nachricht (Vorschlag `POST /api/channels/{name}/ablage` mit `{anhaenge: [ids]}`) gehört zu Karte #45.

  **Offen — Fragen an den Betreiber** (je eine Frage; bis zur Antwort gilt nichts davon als gebaut — Karte #45):

  **F8 — Wie lange bleiben Dateien ohne Nachricht?**
  1. Wie Nachrichten: sie entfallen mit der Aufbewahrungsfrist ihres Channels, gerechnet ab dem Ablegen **(Empfehlung: eine Regel für alles im Verzeichnis, kein zweiter Ablageort, das Laufwerk füllt sich nicht unbemerkt über Monate)**
  2. Bis jemand sie löscht — nur die Anhang-Quote begrenzt sie (braucht eigene Ablage außerhalb der Tagesordner)
  3. Eigene Frist für Dateien ohne Nachricht je Channel, einstellbar wie die Frist

  **F9 — Unterordner in einem Channel-Verzeichnis?**
  1. Nein: je Channel bzw. Direktgespräch genau ein Verzeichnis, flach **(Empfehlung: die Chat-Struktur ist die Verzeichnisstruktur; Unterordner brauchen Namen, Verschieben und eigene Löschregeln)**
  2. Ja, Unterordner nur für Dateien ohne Nachricht
  3. Ja, beliebige Unterordner für alle Dateien

  **F10 — Umbenennen von Dateien?**
  1. Nein: der Name ist der beim Hochladen **(Empfehlung: die Nachricht nennt den Namen, unter dem sie gesendet wurde; Umbenennen bräuchte einen dauerhaften Vermerk neben dem Log)**
  2. Nur Dateien ohne Nachricht
  3. Alle, die Nachricht zeigt dann den neuen Namen

  **F11 — Verschieben zwischen Verzeichnissen?**
  1. Nein **(Empfehlung: ein Anhang gehört zum Gespräch seiner Nachricht; Verschieben in ein anderes Gespräch änderte, wer ihn sieht)**
  2. Nur Dateien ohne Nachricht zwischen Channels
  3. Kopieren statt Verschieben (zählt doppelt an der Quote)

  **F12 — Ersetzen und Versionen?**
  1. Nein: eine neue Fassung ist eine neue Datei **(Empfehlung: jede Datei ist unveränderlich, ETag `"<id>"` bleibt wahr, kein Versionsspeicher an der Quote)**
  2. Ersetzen einer Datei ohne Nachricht unter gleichem Namen, alte Fassung gelöscht
  3. Versionen mit Verlauf je Datei

  **F13 — Mehrere Dateien und ganze Ordner hochladen?**
  1. Mehrere Dateien auf einmal ja, Ordner nein **(Empfehlung: mehrere Dateien sind dieselben Routen wie heute; Ordner hängen an F9)**
  2. Auch ganze Ordner, als flache Liste ihrer Dateien
  3. Ganze Ordner mit Struktur (setzt F9 Option 3 voraus)

  **F14 — Herunterladen mehrerer Dateien als Archiv?**
  1. Nein: jede Datei einzeln über „Laden“ **(Empfehlung: kein Archivcode und kein zweiter großer Strom im Server; ein Agent lädt mit `curl` je ID)**
  2. Ja, Auswahl als ZIP, gestreamt ohne Kompression
  3. Ja, ein ganzes Verzeichnis als ZIP

  **Matrixzeilen** (ersetzen M40–M46 der Ergänzung „F1“; für den `security-reviewer`, Gate an R00029; M47–M50 bleiben):

  | # | Fläche | Angriff | erwartet | Linie |
  |---|---|---|---|---|
  | M40′ | Quote füllen | eine Herkunft lädt `Q` Bytes in vielen Dateien hoch und sendet sie | danach jede Anmeldung 507 `voll` ohne `Retry-After`, **Text 201**, kein gebundener Anhang verändert; Episode `Platz für Anhänge voll` je Kennung; nach bestätigtem Löschen von n Bytes nimmt die nächste Anmeldung ≤ n an; Kontrollpaar K1 vor dem Füllen | L1, L2 |
  | M41′ | Abbruch | Anmelden `Q`, 1 GiB schreiben, Verbindung schließen; Lebensende abwarten | Anspruch frei, `.teil` weg, kein Anhang anderer verändert, Invarianten halten | L1, L2 |
  | M42′ | Anspruch hält Platz | Anmelden `Q`, alle 9 s 64 KiB, nach der Gesamtfrist neu `PUT` | andere 507 `hochladungen` mit `Retry-After`, Text 201; spätestens am Lebensende frei; Health `reserviert` ≈ `Q`; **Löschen hilft nicht** (nur gebundene) — Restrisiko bewerten (≈ 6,4 KiB/s halten 20 GiB ≈ 11,4 h), Folgeoption E2 (Anspruch je Herkunft ≤ 50 %) | L2 |
  | M43′ | Wettläufe | Löschen gegen Entfallen desselben Tages, gegen `GET` mit Range, gegen zweites Löschen derselben ID, gegen eine Anmeldung, die auf den Platz wartet; Kürzen scheitert | jeder Anhang genau einmal abgezogen; Platz erst nach dem Kürzen frei; `fehler` stellt `vorhanden` her; keine Verklemmung (Löschen nimmt keine Gesprächssperre) | L1 |
  | M44′ | Orakel und Sichtbarkeit | Löschen mit ausgedachter ID, mit ID eines Direktgesprächs als Nichtbeteiligter, mit ID des Eingangs; Ereignis `anhaenge` und Laufwerk-Liste eines Direktgesprächs für Dritte | `unbekannt` in allen drei Fällen, gleich lange Antwort; kein Ereignis, kein Verzeichnis für Dritte; 410 ohne Gespräch, Tag, Name | L2 |
  | M45′ | Kürzen und Symlink | Datei vor dem Kürzen durch einen Symlink auf eine fremde Datei ersetzt; Tag-Ordner durch Symlink ersetzt | fremde Datei unverändert (`O_NOFOLLOW`), Ergebnis `fehler`, `ERROR anhang`, Anhang wieder `vorhanden` | L1, L2 |
  | M46′ | Quote gesenkt | Neustart mit `--anhangquote` unter der Belegung | Start löscht nichts, `WARNING`; jede Anmeldung 507 `voll`, bis Löschen sie unter die Quote bringt | L2 |
  | M51 | Löschflut | ein Agent löscht alle IDs aus Verlauf bzw. Laufwerk mit `bestaetigt: true` in Anfragen zu 500 | gelingt (gewollt, keine Rechte); je Anfrage eine `INFORMATION`-Zeile mit Kennung, Zahl, `fremde`; 429 nach Schreibkonto (6 Einheiten je 500); Health `anhaenge_geloescht`; nie ID oder Name im Protokoll | L2 |
  | M52 | CSRF und Rebinding | `POST /api/anhaenge/loeschen` mit fremdem `Origin`, `Sec-Fetch-Site: cross-site`, `text/plain`-Formular, fremdem Host | 403 bzw. 415 ohne Wirkung | L2 |
  | M53 | Löschkörper | leere Liste, 501 IDs, doppelte ID, ID in Großbuchstaben, 25/27 Zeichen, `bestaetigt: "ja"`, weiteres Feld, Körper 16.385 Bytes | 422 `leer`/`anzahl`/`doppelt`/`id`/`bestaetigt`/`feld`, 413; nichts gelöscht, keine 409-Vorschau | L1, L2 |

  **Einordnung des Risikos** (ehrlich): keine Daten gehen mehr ohne menschliche Handlung verloren — das Verdrängen ist weg. Dafür zwei Wege, auf denen ein einzelner Teilnehmer allen schadet: (1) **die Quote füllen** — 20 GiB im Gigabit-LAN in ≈ 3–4 min, danach lädt niemand hoch, bis jemand löscht; Text geht weiter (eigene Quote). Gemessen an der Leitplanke „Lastprofil Team-LAN“ (20 Menschen, 50 Agenten auf einem normalen Rechner, Maßstab für Grenzen gegen Flut) bleibt der Chat selbst unberührt, die Wirkung ist sichtbar (Episode, 90 %, Health) und durch eine Sammellöschung in Sekunden behoben — hingenommen, weil der Betreiber Blockieren statt Verdrängen gewählt hat und ein Anteil je Herkunft „für alle zusammen“ und B2 widerspräche. (2) **alles löschen** — gewollt möglich („Jeder darf jeden Anhang löschen“); Gegenmittel ohne Rechte: Bestätigung mit `fremde`, Protokoll mit Kennung, Konto; Rückgängig und Sichtbarkeit des Löschenden sind F6 und F7 ([ADR-0046](0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) Ergänzung „F1′“). Folgerichtung an den `security-reviewer`: M40′–M53 als Gate an R00029; M42′ mit Zahlen bewerten.

  **Zuschnitt (Empfehlung an Director und `planer`):** Karte #9 (R00029) baut die blockierende Quote (507 `voll`/`hochladungen`, 422 `zu_gross`), die Route Löschen mit Bestätigung, Abruf 410, `zustand`, das Ereignis `anhaenge` auf dem Server, Laden „Länge 0 = gelöscht“, Protokoll und Health — ohne Löschen gäbe es zwischen #9 und #45 keinen Ausweg aus einer vollen Quote außer der Frist. Karte #43 (R00030) zeigt `zustand` und folgt dem Ereignis im Verlauf, zeigt 507 `voll` an der Datei. Karte #45 (D0006) baut die Lesesicht (Regel 5), die Oberfläche mit Auswahl, Bestätigungsdialog und Sammellöschung, Dateien ohne Nachricht und — je nach F8–F14 — mehr.
- **2026-10-29 (Karte #9 Nachfragen planer; R00029 Offene Frage 4, R00030 Offene Fragen 4 und 5) — angezeigte Maße bei EXIF-Orientierung; Ladefehler ist nicht „Gelöscht“:** Entscheider: Claude (autonom, architekt, Auftrag des Directors).
  1. **`breite`/`hoehe` sind die angezeigten Maße** (Status: Akzeptiert). Trägt ein Bild `Orientation` 5, 6, 7 oder 8, liefert der Server Breite und Höhe **getauscht** — überall gleich: 200 des `PUT`, `nachricht.anhaenge` in Verlauf, Senden-Antwort und Ereignis, und in der Satzzeile `+ anhang: … <breite>x<hoehe>` ([ADR-0046](0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) Regel 6; noch nicht gebaut, keine Umstellung alter Daten).
     - **Woher**: Tag `0x0112` nur aus IFD0 der EXIF-Struktur, die der Tilger nach Ergänzung „F4“ Regel 2 erreicht (JPEG `APP1` `Exif\0\0` vor `SOS` über die Längenkette, PNG `eXIf`, WebP `EXIF`); beide Byte-Ordnungen des TIFF-Kopfs. Jeder andere Wert (0, 1–4, 9 und mehr, Typ falsch, Eintrag außerhalb des Segments, nur in IFD1) → nicht getauscht. GIF und die übrigen Formate: nie getauscht. Weil das Tilgen `Orientation` unverändert stehen lässt, sind die Maße vor und nach dem Tilgen gleich.
     - **Ein Leser**: das Lesen von IFD0/`Orientation` gehört einmal in [A00018](../Architektur/A00018-anhangpruefung.md) und dient Erkenner (F0098) und Tilger (F0115, minimales EXIF); wer ihn baut, entscheidet der `planer`.
     - **Grenzen unberührt**: 50 Megapixel und 16.384 px je Seite sind symmetrisch.
     - Optionen: **B1 (gewählt) getauscht liefern** — Stärke: `width`/`height` am `<img>` entsprechen dem, was der Browser zeichnet (`image-orientation: from-image` ist Standard), der Platz steht hochkant, bevor ein Byte lädt (4000 × 3000, `Orientation` 6 → 3000 × 4000 → Vorschau 180 × 240 statt Bild klein im querformatigen 360 × 270), und ein Agent an der API bekommt die Maße, die ein Mensch sieht. Schwäche: `breite`/`hoehe` sind nicht mehr die Rohmaße aus `SOFn`/`IHDR` — wer die Pixelmatrix braucht, muss selbst drehen; hingenommen, niemand im System braucht sie. **B2 Rohmaße plus Feld `orientierung`** (verworfen): Stärke: nichts geht verloren; Schwäche: die Regel „5–8 tauscht“ müsste jede Oberfläche und jeder Agent kennen — zwei Orte für eine Regel, ein vergessener Ort reserviert falsch. **B3 Rohmaße wie bisher** (verworfen): Stärke: kein Leser für IFD0 im Erkenner; Schwäche: jedes Hochformatfoto vom Telefon (der Normalfall) erscheint klein im Querformat-Platz.
     - **Vermutung, nicht geprobt**: Chromium und Firefox drehen JPEG nach EXIF; ob jeder Browser auch PNG `eXIf` und WebP `EXIF` dreht, ist offen. Dreht einer nicht, zeigt `object-fit: contain` das ungedrehte Bild mit Rand im hochkanten Platz — unverzerrt, ohne Sprung. Geprüft wird JPEG (E2E „Hochformatfoto“).
     - **Erste Tests** (für den `planer`): Unit — JPEG `SOF0` 4000 × 3000 mit `Orientation` 6 → 3000 × 4000; 8 → 3000 × 4000; 3 → 4000 × 3000; 9 → 4000 × 3000; `Orientation` 6 nur in IFD1 → 4000 × 3000; `Orientation` 6 im Motorola- und im Intel-Kopf → je getauscht; IFD0-Offset hinter dem Segmentende → nicht getauscht, Maße bleiben; PNG mit `eXIf` `Orientation` 5 und `IHDR` 1280 × 800 → 800 × 1280. Mutante „Tausch weg“ rot.
     - **Folgen**: R00029 F0098 (Kriterium „Tabelle“ bzw. „Maße aus dem Segment“ um die Fälle oben, „Über das Programm“ mit `hochkant.jpg` → 3000 × 4000), Offene Frage 4 erledigt. R00030 Kriterium „Hochformatfoto“: der reservierte Platz ist 180 × 240 (Seitenverhältnis 3 : 4 vor dem ersten Byte), `width="3000" height="4000"`, die Höhe der Nachricht ändert sich beim Laden nicht; Offene Frage 4 erledigt.
  2. **Ein `error` auf `<img>`, `<video>`, `<audio>` zeigt den Fehlerzustand, nicht „Gelöscht“** (Status: Akzeptiert) — bestätigt R00030 und **ersetzt den letzten Halbsatz** von Ergänzung „F1′“ Regel 4 („ein `error` … führt zur selben Zeile“). „Gelöscht“ zeigt die Oberfläche nur, wenn der Server es sagt: `zustand: "geloescht"` in Verlauf, Senden-Antwort oder Ereignis `anhaenge`. Bei `error` bleibt der reservierte Platz, darin der Fehlertext aus R00030 F0102; die Höhe ändert sich nicht.
     - Optionen: **E1 (gewählt) Fehlerzustand** — Stärke: die Oberfläche behauptet nur, was belegt ist; ein Netz- oder Codecfehler (Video, das der Browser nicht abspielt; WLAN weg) sieht nicht aus wie ein Löschen durch einen anderen. Schwäche: ist ein Ereignis `anhaenge` verloren gegangen, zeigt die Nachricht „Bild nicht verfügbar.“ statt „Gelöscht“ bis zum nächsten Laden des Verlaufs — hingenommen, F0119 lädt nach Neuladen und Verbindungsabbruch den Verlauf mit `zustand` neu. **E2 „Gelöscht“ wie Regel 4** (verworfen): Stärke: ein Pfad weniger; Schwäche: behauptet ein Löschen, das nicht stattfand — gegen „keine Statusaussage ohne Beleg“, und ein Codecfehler hieße „Gelöscht“, obwohl „Laden“ die Datei liefert. **E3 bei `error` nachfragen** (`HEAD /anhaenge/<id>`, 410 → „Gelöscht“, sonst Fehlerzustand; verworfen): Stärke: richtig auch ohne Ereignis; Schwäche: bei Netzausfall eine Anfrage je sichtbarem Bild und Player, ein weiterer Pfad unter `/anhaenge/` (CSP-Weiche, M14) für einen Fall, den F0119 schon abdeckt.
     - **Folgen**: R00030 bleibt wie geschrieben (Kriterien „Fehlerzustände“, „Ladefehler ist nicht gelöscht“); der Verweis „Abweichung vom letzten Halbsatz der ADR-Regel 4“ und Offene Frage 5 entfallen, Quelle ist diese Ergänzung. R00029 unberührt.
