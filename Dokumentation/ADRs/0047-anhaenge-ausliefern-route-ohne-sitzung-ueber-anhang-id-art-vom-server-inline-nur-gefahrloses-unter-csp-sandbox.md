# ADR-0047: Anhänge ausliefern — eine Route ohne Sitzung über die Anhang-ID, die Art bestimmt der Server, inline nur Gefahrloses unter `CSP: sandbox`, alles andere als Download; Bildvorschau aus dem Original

**Status**: Vorgeschlagen
**Datum**: 2026-10-29
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #9 I0010) — wartet auf den Betreiber (Fragen B3, B4 unten) und auf die Angriffsmatrix des `security-reviewer` vor dem Bau
**Kontext-Anforderung**: — (WBS I0010, Dialog D0003, Karte #9; prägt I0012, I0015, I0016, I0017, I0022)
**Erprobt durch**: Wegwerf-Probe im Scratchpad, 2026-10-29 (Python-`http.server` auf freiem Port, Playwright 1.57 mit Chromium headless-shell und Chromium headless; jede Antwort mit `X-Content-Type-Options: nosniff`; App-Seite unter der CSP von ADR-0014 legt `localStorage.t = "geheim"` an): (1) `text/html` **ohne** `sandbox` im selben Ursprung → das eingebettete Skript liest `localStorage` (`skript-lief:geheim`) — die Bedrohung ist echt; (2) `text/html` mit `Content-Security-Policy: sandbox; default-src 'none'` → kein Skript läuft (Titel unverändert); (3) dasselbe mit `sandbox allow-scripts` → Skript läuft, `localStorage` wirft `SecurityError` (undurchsichtiger Ursprung); (4) `<img src>` in der App-Seite auf eine PNG-Antwort mit `sandbox`-CSP → geladen (`naturalWidth` 4); (5) PNG und `text/plain` mit `sandbox` öffnen als Dokument normal, ein `<script>` im Text erscheint als Text, HTML als `text/plain` erscheint als Quelltext; (6) `application/pdf` mit `sandbox` öffnet in Chromium headless denselben PDF-Betrachter wie ohne (`embed type="application/pdf"`, Bildschirmfoto gleich; Inhalt der Testdatei leer, Firefox nicht geprüft), headless-shell lädt PDFs stets herunter; (7) `application/octet-stream` + `Content-Disposition: attachment` (SVG mit Skript) → Download, nichts dargestellt. Probe danach gelöscht.

## Kontext

- **Vision**: „Anhänge. An eine Nachricht lassen sich eine oder mehrere Dateien hängen.“ WBS I0010: „… und lassen sich öffnen“. Leitplanken: keine Anmeldung, eine gemeinsame Instanz im LAN; „Ausdrücklich nicht Ziel“: Rechte, öffentlicher Internet-Dienst. WBS I0012: „nur Absender und Empfänger sehen das Gespräch“.
- **Ursprung der App**: `http://<ip-oder-name>:<port>` — Oberfläche, API und Token (`localStorage`, [ADR-0012](0012-identitaet-und-sitzung.md)) teilen ihn. Eine Datei, die dort als HTML, SVG oder XML-Dokument dargestellt wird, liest das Token und spricht die API im Namen des Opfers (Probe 1). Die CSP der App ([ADR-0014](0014-grenzschutz-host-origin-sicherheitsheader.md) Regel 5) gilt je Antwort — eine Anhang-Antwort braucht ihre eigene.
- **Sitzung nur im Header** ([ADR-0012](0012-identitaet-und-sitzung.md), [ADR-0015](0015-ein-ereignisstrom-je-browser-sharedworker-mit-sitzungs-header.md)): `<img src>`, `<a href>`, ein neuer Tab senden keinen eigenen Header; ein Token im Query-Parameter ist ausgeschlossen. Cookies sind verworfen (Cookie-Tossing über andere Programme auf `127.0.0.1`).
- **Sichtbarkeit** ([X00002 Benutzerrechte](../Architektur/Aspekte/X00002-benutzerrechte.md)): keine Rechte; jeder mit Sitzung liest jeden Channel, jeder im LAN kann beitreten; einzige Regel: ein Direktgespräch sehen nur seine Beteiligten (I0012, noch nicht gebaut).
- **Große Antworten** ([ADR-0039](0039-grosse-antworten-byte-budget-je-block-werkbank-und-abrissgrenze-als-sendefrist.md) Regel 7c, e): Dateien gestreamt in 64-KiB-Stücken mit Gegendruck; Sendefrist ist die Abrissgrenze.
- **Designsystem** (`components/Anhang/README.md`): Datei-Zeile mit Typkürzel, Name, Größe, „Öffnen“ und „Laden“; Bilder als Vorschau höchstens 360 × 240 px, Platz aus Breite und Höhe der Metadaten reserviert, Klick öffnet das Original im Dialog. README „Billig je Nachricht … Bilder reservieren ihren Platz aus den Metadaten“.
- Ablage, Anhang-ID (128 Bit Zufall) und Metazeile: [ADR-0046](0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md).

Szenarien mit Maß:

- **QS1 Kein aktiver Inhalt im Ursprung**: ein Teilnehmer lädt HTML, SVG, XHTML, eine Polyglot-Datei (PNG-Kopf + HTML) oder Text mit `<script>` hoch, ein anderer klickt „Öffnen“ → in keinem Fall läuft ein Skript mit Zugriff auf `localStorage` oder die API der Instanz; geprüft im echten Programm mit Chromium.
- **QS2 Öffnen**: Klick auf „Öffnen“ bei Bild, PDF oder Text → die Datei erscheint im neuen Tab, ohne Download-Dialog; bei jeder anderen Art → Download unter dem richtigen Namen.
- **QS3 Vorschau billig**: ein Channel mit 100 Bildnachrichten öffnet so schnell wie ohne (Q1b ≤ 500 ms, Platz reserviert, kein Springen L4 ≤ 1 px); Bilder laden erst in Sichtnähe.
- **QS4 Langsame Leser**: 160 Verbindungen einer Herkunft holen je einen 25-MiB-Anhang und lesen nicht → RSS-Zuwachs ≤ 160 × 64 KiB + 50 MB (≈ 60 MB), jede Verbindung nach der Abrissgrenze zu (Q8 sinngemäß).

## Optionen

### Frage 1: In welchem Ursprung wird ein Anhang dargestellt?

#### Option A1: Im Ursprung der App, aber jede Anhang-Antwort mit eigener CSP `sandbox; default-src 'none'`, `nosniff`, Art vom Server, inline nur eine Erlaubnisliste gefahrloser Arten, alles andere `application/octet-stream` + `attachment` (gewählt)
- Vorteile: zwei unabhängige Linien — was nicht auf der Liste steht, wird nie dargestellt (Download), und selbst ein dargestelltes Dokument läuft in einem undurchsichtigen Ursprung ohne Skript (Probe 2, 3); ein Port, ein Grenzschutz, ein Start; `<img>`-Vorschau ohne CSP-Änderung der App (Probe 4); PDF-Betrachter funktioniert (Probe 6, Chromium).
- Nachteile: hängt an der korrekten Umsetzung von `sandbox` im Browser; wer später eine Art auf die Liste setzt (SVG „für die Vorschau“), umgeht die erste Linie — die zweite hält dann allein.

#### Option A2: Eigener Ursprung für Anhänge — zweiter Listener auf einem weiteren Port, der nur `/anhaenge/` bedient
- Vorteile: die stärkste Trennung (der Ursprung umfasst den Port — fremder `localStorage`, fremde API); hält auch bei einem Fehler im `sandbox` des Browsers; Muster großer Dienste („usercontent“-Domains).
- Nachteile: zweiter Port je Instanz (finden, ausweichen, in der Startzeile nennen, in der Firewall öffnen), zweiter Grenzschutz mit eigener Host-Liste (ADR-0045), Verbindungsgrenzen doppelt; `<img>` der App braucht `img-src` mit dem zweiten Ursprung; die Leitplanke „ein Programm“ hält, aber das Bedienkonzept wie kanbanD (ein Port) nicht mehr.

#### Option A3: Nie darstellen — alles `application/octet-stream` + `attachment`, Vorschau nur über `<img>`
- Vorteile: am einfachsten; nichts außer `<img>` erreicht je den Renderer.
- Nachteile: „Öffnen“ des Designsystems und QS2 entfallen — jedes PDF, jeder Log-Text, jedes Bild im Original landet im Download-Ordner; widerspricht dem Fertig-Kriterium „lassen sich öffnen“ nur dann nicht, wenn man Download als Öffnen liest.

#### Option A4: Abruf mit Sitzungs-Header im Skript, Darstellung über `blob:`-URLs
- Vorteile: der Nachweis bleibt der Header — keine Route ohne Sitzung.
- Nachteile: ein `blob:`-Dokument gehört dem Ursprung der App (dort hätte ein HTML-Anhang das Token, Probe 1) — die Sicherheit hinge allein an der Art, die das eigene Skript dem Blob gibt; die ganze Datei liegt im Speicher des Tabs; die App-CSP braucht `img-src blob:`; Öffnen im neuen Tab, mittlere Maustaste und „Link kopieren“ funktionieren nicht wie gewohnt.

### Frage 2: Woran erkennt die Instanz, dass jemand einen Anhang öffnen darf?

- **Z1 (gewählt): die Anhang-ID ist die Berechtigung** — 128 Bit aus `getrandom`, Base32 klein (26 Zeichen); `GET /anhaenge/<id>` ohne Sitzung. Stärke: funktioniert für `<img>`, Links, neue Tabs und Agenten gleich; streamt im Browser ohne Skript; die ID erfährt nur, wer die Nachricht lesen darf (Verlauf, Strom, Warten — für Direktgespräche über den Filter von X00002). Schwäche: **wer den Link hat, öffnet ihn** — auch weitergegeben, auch ohne Beitritt, bis der Tag entfällt; für Direktgespräche wird „nur die Beteiligten sehen es“ zu „nur wer den Link kennt“ (Frage B4 an den Betreiber).
- Z2: Sitzungs-Header Pflicht, Öffnen nur über A4. Stärke: ein Nachweis für alles. Schwäche: die Nachteile von A4.
- Z3: Kurzzeit-Ticket — die Oberfläche holt mit Header `POST /api/anhaenge/<id>/ticket` und öffnet `/anhaenge/<id>?ticket=…` (60 s, einmalig). Stärke: ein weitergegebener Link verfällt. Schwäche: je sichtbarem Vorschaubild ein zusätzlicher Aufruf (100 Bilder im Fenster = 100 Tickets); ein Ticket in der URL ist ein kurzlebiges Token im Query (der Gedanke, den ADR-0015 ausschließt); Zustand je Ticket.
- Z4: Lesen ohne jeden Nachweis über Gespräch, Tag und Dateiname (`/anhaenge/<gespräch>/<tag>/<datei>`). Stärke: keine Zuordnung im Speicher. Schwäche: Namen sind erratbar (Zeit in ms ist in der Nachricht sichtbar) — für Direktgespräche offen für jeden im LAN.

### Frage 3: Welche Arten werden inline dargestellt?

Die Art bestimmt die Instanz beim Hochladen aus dem Inhalt ([A00018 Anhangprüfung](../Architektur/A00018-anhangpruefung.md)); der `Content-Type` des Senders und die Endung des Namens zählen nie.

| Art (Erkennung) | ausgeliefert als | `Content-Disposition` | Vorschau im Verlauf |
|---|---|---|---|
| PNG (`89 50 4E 47 0D 0A 1A 0A`, Maße aus `IHDR`) | `image/png` | `inline` | ja (Regel 5) |
| JPEG (`FF D8 FF`, Maße aus dem ersten `SOFn`) | `image/jpeg` | `inline` | ja |
| GIF (`GIF87a`/`GIF89a`, Maße aus dem Kopf) | `image/gif` | `inline` | ja |
| WebP (`RIFF….WEBP`, Maße aus `VP8 `/`VP8L`/`VP8X`) | `image/webp` | `inline` | ja |
| PDF (`%PDF-`) | `application/pdf` | `inline` | nein — Datei-Zeile |
| Text (gültiges UTF-8 über die ganze Datei, kein NUL) | `text/plain; charset=utf-8` | `inline` | nein — Datei-Zeile |
| alles andere — auch SVG, HTML, XHTML, XML, JavaScript, ausführbare Dateien, Archive | `application/octet-stream` | `attachment` | nein — Datei-Zeile, nur „Laden“ |

- Weil SVG, HTML und XML gültiger Text sind, erscheinen sie als **Quelltext** (`text/plain`, Probe 5) — nie als Dokument.
- **Gewählt** als Mechanik; ob die Liste kürzer sein soll (z. B. kein PDF inline) oder Arten gar nicht angenommen werden, ist Frage B1 an den Betreiber (ADR-0046).
- Verworfen: die Art aus der Endung (eine `bild.png`, die HTML enthält, würde als `image/png` mit `nosniff` zwar nicht ausgeführt, aber als kaputtes Bild gezeigt — und `.html` mit Bildinhalt umgekehrt); die Art vom Sender (Angreifer setzt `text/html`); SVG als Bild in der Vorschau (in `<img>` ohne Skript gefahrlos, beim „Öffnen“ aber ein aktives Dokument — die erste Linie bräche für eine Art, die die Liste erst trägt).

### Frage 4: Vorschau — Richtung beim Betreiber (B3), Mechanik hier

- **V1 (Empfehlung): Original in `<img>` mit reserviertem Platz** — nur Bildarten der Tabelle, höchstens 10 MiB und höchstens 50 Megapixel (jede Seite ≤ 16.384 px), `loading="lazy"`, Breite und Höhe aus der Metazeile (Designsystem: bis 360 × 240 px im Seitenverhältnis); größere Bilder als Datei-Zeile mit „Öffnen“. Stärke: kein Bildcode auf dem Server; der Platz steht, bevor ein Byte lädt (QS3); die Grenze in Pixeln hält Dekompressionsbomben aus dem Tab. Schwäche: ein 10-MiB-Foto wird für 360 px voll geladen — in einem Channel voller Fotos kostet Scrollen Bandbreite (im LAN hingenommen).
- V2: keine Vorschau, jedes Bild als Datei-Zeile. Stärke: am billigsten. Schwäche: Bildschirmfotos — der häufigste Anhang unter Menschen und Agenten — brauchen einen Klick.
- V3: Vorschaubilder auf dem Server (Bibliothek `image`, verkleinert beim Hochladen, abgelegt neben dem Original). Stärke: kleine Übertragung je Vorschau. Schwäche: Bilddekodierung fremder Dateien im Server-Prozess (Dekompressionsbomben, Parser-Fehler, CPU je Hochladung); neue große Abhängigkeit; zweite Datei je Anhang an Quote und Entfallen.

### Frage 5: Was sieht der Agent?

- **Gewählt: Metadaten überall, Bytes nur über die eine Route.** `nachricht` in Verlauf ([ADR-0025](0025-verlauf-ueber-die-api-bloecke-mit-zeit-als-cursor.md)), Senden-Antwort, Ereignis ([ADR-0028](0028-nachrichten-live-als-drittes-thema-ordnung-ueber-die-nachrichtenzeit.md)) und künftig Warten (I0016) trägt `anhaenge: [{id, name, bytes, art, breite, hoehe, url}]` (immer eine Liste, leer ohne Anhang; `breite`/`hoehe` `null` außer bei Bildern; `url` = `/anhaenge/<id>`, relativ). Stärke: ein Agent entscheidet ohne Herunterladen (Größe, Art) und lädt mit `curl -O` wie jeder Browser; Listen bleiben klein (ADR-0039 Regel 7c). Schwäche: keine Suche in Anhängen (I0011 sucht nur Text).
- Verworfen: Inhalt kleiner Textanhänge im Verlauf mitliefern (sprengt das Byte-Budget je Block, ADR-0039); eine eigene Liste „alle Anhänge eines Channels“ (kein Szenario).

## Entscheidung

**Gewählt (vorgeschlagen): A1, Z1, Arten-Tabelle aus Frage 3, V1 als Empfehlung an den Betreiber, Metadaten wie in Frage 5.**

Es überwiegt, dass A1 mit zwei unabhängigen Linien (Download für alles Unbekannte, `sandbox` für alles Dargestellte) im geprobten Browser kein Skript im Ursprung der App zulässt, ohne einen zweiten Port, einen zweiten Grenzschutz und eine zweite Ausnahme im Bedienkonzept zu verlangen; dass Z1 als einziger Nachweis für `<img>`, Links und Agenten gleich funktioniert, ohne Token in einer URL; und dass V1 die Vorschau ohne Bildcode auf dem Server liefert. In Kauf genommen: die Sicherheit dargestellter Anhänge hängt an `sandbox` im Browser (A2 bleibt die Ausweichoption, falls die Angriffsmatrix eine Lücke findet); **ein Link öffnet ohne Sitzung**, auch weitergegeben, bis sein Tag entfällt; Vorschaubilder werden in voller Größe geladen. **Vorgeschlagen**, weil Vorschau (B3) und die Bedeutung eines Links für Direktgespräche (B4) Richtungsfragen des Betreibers sind und die Angriffsmatrix des `security-reviewer` vor dem Bau steht.

Regeln:

1. **Route** `GET /anhaenge/{id}` (und `HEAD`) unter `grenzschutz` (Host-Prüfung gilt, also kein Lesen über DNS-Rebinding; ADR-0014, ADR-0045), **ohne** Extraktor `Teilnehmer` — eine mitgeschickte Sitzung wird nicht geprüft und verändert nichts (kein Lebenszeichen). Außerhalb von `/api/`, weil Browser sie ohne Header aufrufen. `id` nicht 26 Zeichen `[a-z2-7]` oder unbekannt → 404 `text/plain` „Diesen Anhang gibt es nicht (mehr).“; Datei fehlt auf der Platte → derselbe 404 und `WARNING anhang` mit Gespräch, Tag, Datei (nie der ID). Gleiche Antwortzeit-Klasse für „falsches Format“ und „unbekannt“ ist nicht nötig — 128 Bit lassen sich nicht durch Raten finden.
2. **Sichtbarkeit**: die ID erscheint nur dort, wo die Nachricht erscheint (Verlauf, Strom, Warten, Senden-Antwort). Für Direktgespräche (I0012) filtert X00002 diese Wege; die Route selbst prüft nichts (Z1). Ob ein weitergegebener Link aus einem Direktgespräch öffnen darf, entscheidet B4; lautet die Antwort nein, kommt für Direktgespräche Z3 dazu (eigene Entscheidung in I0012).
3. **Kopfzeilen jeder Antwort dieser Route** (auch 404, 304, `HEAD`), zusätzlich zu ADR-0014 Regel 5 und diese ersetzend, wo sie sich überschneiden:
   - `Content-Security-Policy: sandbox; default-src 'none'` — genau die geprobte Richtlinie (Bild-, Text- und PDF-Ansicht des Browsers erscheinen damit unverändert, Probe 5 und 6);
   - `X-Content-Type-Options: nosniff`, `Cross-Origin-Resource-Policy: same-origin`, `Referrer-Policy: no-referrer`, `X-Frame-Options: DENY` (bleiben aus ADR-0014);
   - `Content-Type` und `Content-Disposition` nach der Tabelle in Frage 3; `Content-Disposition: <inline|attachment>; filename="<ascii-ersatz>"; filename*=UTF-8''<prozentkodierter Anzeigename>` (RFC 6266/8187; der ASCII-Ersatz ist der gespeicherte Name ohne Präfix; `"` und `\` kommen in keinem der beiden vor, ADR-0046 Regel 3);
   - `Content-Length`; `Cache-Control: private, no-cache` und `ETag: "<id>"` — `If-None-Match` mit derselben ID → 304 ohne Körper (der Inhalt einer ID ändert sich nie; nach dem Entfallen antwortet die Prüfung 404, eine zwischengespeicherte Kopie im Browser bleibt bis zu dessen Aufräumen — hingenommen).
4. **Streaming** nach ADR-0039 Regel 7c: Datei öffnen (über den Port der [Logablage](../Architektur/A00008-logablage.md), `O_NOFOLLOW` auf der letzten Pfadstelle), in Stücken zu 64 KiB mit Gegendruck senden; Sendefrist = Abrissgrenze (Regel 6 dort). Keine `Range`-Anfragen (antwortet ganz mit 200; `Accept-Ranges: none`) — kein Szenario braucht Teilabrufe. Entfällt der Tag, während gesendet wird, hält der offene Dateideskriptor die Bytes bis zum Ende dieser Antwort (Unix-Semantik).
5. **Vorschau im Browser** (V1, falls B3 ja): `<img src="/anhaenge/<id>" width height loading="lazy" decoding="async" alt="<anzeigename>">` für Bildarten mit Maßen, ≤ 10 MiB, ≤ 50 Megapixel, jede Seite ≤ 16.384 px; Klick öffnet dasselbe Bild im Dialog (Designsystem); sonst Datei-Zeile. Die App-CSP bleibt unverändert (`img-src 'self' data:`, Probe 4). Kein `blob:`, kein `innerHTML` mit Anhang-Daten; Name und Größe nur über `textContent` (A00005).
6. **Öffnen und Laden** in der Oberfläche: „Öffnen“ = `<a href="/anhaenge/<id>" target="_blank" rel="noopener">` nur für Arten mit `inline`; „Laden“ = `<a href="/anhaenge/<id>" download="<anzeigename>">` für alle. Kein Skript zwischen Klick und Datei.
7. **Protokoll** ([ADR-0042](0042-stufen-ereigniskatalog-episoden-und-tagesgrenze-des-betriebsprotokolls.md)): die Anhang-ID ist eine Berechtigung und steht wie ein Token **nie** im Betriebsprotokoll — das Anfrageprotokoll (`src/http/anfrageprotokoll.rs`) schreibt für `/anhaenge/…` den Pfad als `/anhaenge/<6 Zeichen>…`. Durchsetzung: Integrationstest „Protokoll auf Debug, ein Abruf, `grep <id> logs/` = 0“.
8. **Agent** (Frage 5): Felder wie dort; `POST /api/anhaenge` und `GET /anhaenge/{id}` stehen in `/api/openapi.json` (I0015).

## Konsequenzen

**Positiv:**
- Kein Anhang erreicht im Ursprung der App einen Skript-Kontext: Unbekanntes wird heruntergeladen, Dargestelltes läuft ohne Skript in einem undurchsichtigen Ursprung (geprobt in Chromium).
- Öffnen, Laden, Vorschau, neuer Tab, `curl -O` — alles über eine Route ohne Skript dazwischen; die App-CSP bleibt, wie sie ist.
- Speicher je Abruf ≤ ein Stück, langsame Leser enden an der Abrissgrenze (QS4).

**Negativ / Inkauf zu nehmen:**
- Ein Link öffnet ohne Sitzung, auch weitergegeben, bis sein Tag entfällt (B4).
- Die zweite Linie hängt am Browser; Firefox ist nicht geprobt (Messwert unten).
- Vorschaubilder laden das Original; ein Channel voller großer Fotos kostet Bandbreite beim Scrollen.
- SVG, HTML und Markdown-Dateien öffnen als Quelltext, nicht gerendert.
- Eine Kopie im Browser-Cache überlebt das Entfallen des Tags.

**Pre-Mortem (ein Monat nach dem Bau gescheitert, weil …):**
1. … jemand SVG „für die Vorschau“ auf die Inline-Liste setzt oder die `sandbox`-CSP für eine Art weglässt, und ein SVG mit Skript das Token eines Lesers stiehlt. Früh-Indikator: Unit-Tabelle der Arten ändert sich; Integrationstest „jede Antwort unter `/anhaenge/` trägt `sandbox`“ wird angefasst. Gegenmaßnahme: Durchsetzung unten; jede Änderung der Tabelle ist eine Ergänzung dieser ADR.
2. … der Erkenner einer Bildart auf einer kaputten Datei in einer Schleife hängt oder abstürzt (JPEG-Segmente). Früh-Indikator: Hochladen hängt bis zur Frist; `EXCEPTION`. Gegenmaßnahme: Erkenner liest höchstens die ersten 64 KiB, jede Schleife schreitet fort oder endet; Fuzz-Probe des `security-reviewer`.
3. … ein Agent Links aus einem Direktgespräch in einen Channel kopiert und Vertrauliches so öffentlich wird. Früh-Indikator: keiner (gewollter Weg). Gegenmaßnahme: B4; bei „nein“ Z3 für Direktgespräche.

**Durchsetzung (Makro, Aspekte [X00002](../Architektur/Aspekte/X00002-benutzerrechte.md), [X00003](../Architektur/Aspekte/X00003-authentifizierung-und-sitzung.md)):**
- Unit (rein, A00018): Tabelle Inhalt → Art, Disposition, Vorschau; Mutanten „SVG inline“, „Endung entscheidet“, „Text ohne UTF-8-Prüfung“ rot.
- Integration (echtes Programm): je Art der Tabelle eine Datei hochladen, senden, abrufen → `Content-Type`, `Content-Disposition`, `sandbox`-CSP, `nosniff`; auch auf 404 und 304. Mutationsprobe je Kopfzeile.
- E2E (Chromium): HTML-, SVG- und Polyglot-Anhang über „Öffnen“ und als direkte Navigation → `localStorage` der App bleibt unerreicht (Titel oder Konsole der Probe wie oben), Download für Nicht-Inline-Arten.
- Review-Punkt `code-reviewer`/`security-reviewer`: keine Antwort unter `/anhaenge/` ohne die Kopfzeilen aus Regel 3; keine Anhang-ID im Protokoll.

**Folgeentscheidungen / Aufgaben:**
- [ ] **Betreiber** (über den Director): B3 Vorschau, B4 Link-Bedeutung — Fragen im Bericht Karte #9.
- [ ] **Angriffsmatrix** vom `security-reviewer` vor dem Bau (Bericht Karte #9 nennt die Zeilen); fällt dort eine Zeile von QS1, gilt A2 als nächste Entscheidung.
- [ ] **Erster Unit-Test** (A00018, rein): 8 Bytes PNG-Kopf + `IHDR` 1280 × 800 → `image/png`, inline, Maße 1280 × 800, Vorschau ja; derselbe Kopf mit `IHDR` 20.000 × 20.000 → inline, Vorschau nein; `<svg …><script>` → Text, inline als `text/plain`; dasselbe mit NUL-Byte → `application/octet-stream`, `attachment`; `%PDF-1.7` → `application/pdf`, inline; JPEG ohne `SOF` in den ersten 64 KiB → `image/jpeg` ohne Maße, Vorschau nein; abgeschnittene Köpfe jeder Art → nie Panik.
- [ ] **Erster Integrationstest** (echtes Programm): Hochladen `x.html` mit `<script>`, senden, `GET /anhaenge/<id>` → `text/plain; charset=utf-8`, `inline`, CSP beginnt mit `sandbox`; unbekannte ID → 404 mit derselben CSP; `If-None-Match: "<id>"` → 304; `Host: evil.example` → 403; nach Frist 1 und Tageswechsel (`LOCALCHATA_JETZT`) → 404.
- [ ] **Messwert** (Entwicklertest): PDF mit Inhalt in Chromium und Firefox über „Öffnen“ → Betrachter zeigt die Seite; bei Firefox ohne Darstellung unter `sandbox` → PDF auf `attachment` (Ergänzung).
- [ ] **Ergänzungen bei Annahme** (architekt): ADR-0014 (Regel 5 für `/anhaenge/`), ADR-0025 und ADR-0028 (Feld `anhaenge`), ADR-0042 (Regel 7 hier), X00002 (Z1).

## Quellen / Bezuege

- [A00004 HTTP-Schnittstelle](../Architektur/A00004-http-schnittstelle.md), [A00005 Weboberfläche](../Architektur/A00005-weboberflaeche.md), [A00016 Anhangverzeichnis](../Architektur/A00016-anhangverzeichnis.md), [A00018 Anhangprüfung](../Architektur/A00018-anhangpruefung.md), [A00008 Logablage](../Architektur/A00008-logablage.md); Aspekte [X00002](../Architektur/Aspekte/X00002-benutzerrechte.md), [X00003](../Architektur/Aspekte/X00003-authentifizierung-und-sitzung.md)
- ADR-0012, ADR-0014, ADR-0015, ADR-0025, ADR-0028, ADR-0039, ADR-0042, ADR-0045, ADR-0046
- Vision R00000; WBS I0010, I0012; Designsystem `components/Anhang/README.md`, `README.md` („Billig je Nachricht“)
- CSP-Direktive `sandbox` (CSP Level 2/3), `Content-Disposition` RFC 6266, `filename*` RFC 8187, `X-Content-Type-Options` (Fetch Standard) — aus dem Gedächtnis, in diesem Auftrag nicht geöffnet; das Verhalten in Chromium ist geprobt (oben)
