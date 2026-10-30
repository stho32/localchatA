# ADR-0014: Grenzschutz im LAN — Host-Erlaubnisliste, Origin-Prüfung für Schreibendes, Sicherheitsheader auf jeder Antwort

**Status**: Akzeptiert — Regel 1 abgelöst durch ADR-0045
**Datum**: 2026-10-27
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #3 I0004)
**Kontext-Anforderung**: — (WBS I0004 als erste schreibende Route; gilt für alle Routen)
**Erprobt durch**: Wegwerf-Probe im Scratchpad (axum 0.8.9, `middleware::from_fn` vor allen Routen, eigene Annahmeschleife): `Host: evil.example` → 403; `Host: <eigener Rechnername>:<port>`, `localhost`, `127.0.0.1` → 200; `POST` mit `Origin: http://evil.example` → 403, mit eigenem Origin → 200, ohne Origin (curl, Agent) → 200; `X-Content-Type-Options: nosniff` an der Antwort. Chromium (Playwright aus `e2e/`): Same-Origin-`fetch` `POST` sendet `Origin: http://127.0.0.1:<port>` und `Sec-Fetch-Site: same-origin`; ein Cross-Origin-`POST` im Modus `no-cors` **erreicht** den Server (Antwort `opaque`) — ohne Origin-Prüfung wäre er ausgeführt worden. Rechnername über `rustix::system::uname().nodename()`. Probe danach gelöscht.

## Kontext

- Die Instanz lauscht standardmäßig auf `0.0.0.0` ohne Anmeldung (Vision, Leitplanken „Keine Anmeldung“, „Eine gemeinsame Instanz im LAN“; [A00002](../Architektur/A00002-programm-localchata.md)).
- Security-Review #1 (Karte #1): Host und Origin sind ungeprüft — **DNS-Rebinding** (eine fremde Website lässt ihren Namen auf `127.0.0.1` oder eine LAN-Adresse zeigen und spricht die Instanz dann als „eigener Ursprung“ an) und **CSRF** (eine fremde Website schickt dem Browser eines Teilnehmers einen `POST` an die Instanz) sind möglich; Sicherheitsheader (`nosniff`, CSP, `frame-ancestors`) fehlen.
- Grenzregel in [A00004](../Architektur/A00004-http-schnittstelle.md): keine Schreibroute vor Sitzungs- und Host-/Origin-Prüfung samt Sicherheitsheadern; `AI-Team/gelerntes.md` (#1): jede neue Route ist LAN-Angriffsfläche.
- I0004 bringt die erste Route mit Wirkung (`POST /api/beitreten` belegt einen Namen) — sie trägt **kein** Token, weil sie es erst ausstellt; ohne Origin-Prüfung könnte jede Website im Browser eines Besuchers Namen besetzen, mit dessen Herkunft.
- Markdown in Nachrichten (Vision) und Agenten mit beliebigem Text machen eine Skript-Einschleusung später wahrscheinlich; die CSP ist die zweite Linie hinter dem Escape-zuerst-Renderer ([A00005](../Architektur/A00005-weboberflaeche.md)).
- Die Rust-Standardbibliothek zählt keine Netzschnittstellen auf; Adressen ändern sich im LAN (DHCP, WLAN).

## Optionen

### Option A: Eine Schicht vor allen Routen in I0004 — Host-Erlaubnisliste (IP-Literale, `localhost`, Rechnername, `--erreichbar-als`), Origin-Prüfung für jede nicht lesende Methode, feste Sicherheitsheader auf jeder Antwort
- Vorteile: schließt Rebinding und CSRF, bevor die erste wirksame Route existiert; eine Stelle, jede Route erbt sie, auch 404 und 403; IP-Literale brauchen keine Liste der eigenen Adressen (ein Rebinding-Angriff braucht einen Namen, ein IP-Literal ist bereits der echte Ursprung); kein neuer Prozess, eine kleine Abhängigkeitserweiterung (`rustix` Feature `system`).
- Nachteile: Zugriff über einen Namen, den die Instanz nicht kennt (`build-03.fritz.box`), wird abgewiesen, bis er mit `--erreichbar-als` freigegeben ist; I0004 wird größer.

### Option B: Wie A, aber als eigene Interaction vor I0004 (Karte #18), I0004 wartet darauf
- Vorteile: I0004 bleibt fachlich schmal; der Schutz hat eine eigene Abnahme.
- Nachteile: eine Interaction ohne Nutzerverhalten (sichtbar nur als Abweisung) — schlecht abnehmbar; zwei Karten müssen in fester Reihenfolge laufen, obwohl keine ohne die andere einen Sinn hat; die Gefahr, I0004 doch zuerst zu bauen.

### Option C: Nur CSRF über das Token — jede wirksame Route verlangt einen eigenen Header (`X-LocalchatA-Sitzung` bzw. für `beitreten` einen festen `X-LocalchatA: 1`), keine Host-Prüfung
- Vorteile: ein Browser schickt eigene Header nur nach CORS-Vorabfrage, die die Instanz nicht beantwortet — CSRF ist damit geschlossen; keine Namensliste.
- Nachteile: DNS-Rebinding bleibt offen: nach dem Umbiegen ist die fremde Seite „gleicher Ursprung“, darf eigene Header setzen und Antworten lesen — inklusive Verlauf, sobald er lesbar ist; Sicherheitsheader fehlen weiter.

### Option D: Standard-Bindung auf `127.0.0.1`, LAN nur mit `--host 0.0.0.0`
- Vorteile: ohne ausdrückliche Wahl keine LAN-Angriffsfläche.
- Nachteile: widerspricht der Leitplanke „eine gemeinsame Instanz im LAN, über Rechnergrenzen“ als Normalfall; Rebinding gegen `127.0.0.1` bleibt trotzdem offen (der Browser des Startenden ist lokal).

## Entscheidung

**Gewählt: Option A**, als erster Slice von I0004 vor `beitreten`.

Es überwiegt, dass nur A beide Angriffe schließt, ohne die LAN-Leitplanke anzutasten, und dass die erste wirksame Route ohne sie gar nicht entstehen darf (Grenzregel A00004). In Kauf genommen: wer die Instanz über einen DNS-Namen anspricht, der nicht der Rechnername ist, muss ihn beim Start freigeben; eine Abweisung nennt den Weg.

Regeln (Schicht `grenzschutz` in `src/http/`, als äußerste Schicht des Routers — gilt damit auch im Testserver):

1. **Host** (jede Methode, jede Route): der Host-Anteil des `Host`-Headers (ohne Port; IPv6 in eckigen Klammern) ist erlaubt, wenn er
   - ein IP-Literal ist (IPv4 oder IPv6), oder
   - `localhost` ist, oder
   - (ohne Beachtung der Groß-/Kleinschreibung) der Rechnername (`uname().nodename()`) oder `<rechnername>.local` ist, oder
   - mit `--erreichbar-als <name>` beim Start freigegeben ist (mehrfach erlaubt; Aufruf, [A00003](../Architektur/A00003-start.md), [X00006](../Architektur/Aspekte/X00006-konfiguration-und-geheimnisse.md)).
   Fehlt der Header oder ist er nicht erlaubt → 403, Text: „Diese Instanz ist unter dem Namen <name> nicht freigegeben — über die IP-Adresse öffnen oder mit --erreichbar-als <name> starten.“
2. **Origin** (jede Methode außer `GET`, `HEAD`, `OPTIONS`): ist `Origin` gesetzt, muss er genau `http://<Host-Header>` sein (Groß-/Kleinschreibung des Namens egal); `Origin: null` ist nie gleich. Ist `Sec-Fetch-Site` gesetzt, muss er `same-origin` oder `none` sein. Fehlen beide (curl, Agent), gilt die Anfrage als nicht vom Browser. Verletzung → 403 `{fehler}`.
3. **Lesende Methoden ändern nie Zustand** (`GET`, `HEAD` sind wirkungsfrei) — nur dann deckt Regel 2 alle Wirkungen ab. JSON-Routen verlangen `Content-Type: application/json` (axum `Json` antwortet sonst 415), was jede Cross-Origin-Anfrage aus dem Browser in eine unbeantwortete Vorabfrage zwingt.
4. **CORS**: die Instanz beantwortet keine Vorabfrage und setzt nie `Access-Control-Allow-*`.
5. **Sicherheitsheader auf jeder Antwort** (auch 403, 404, Fehler):
   - `X-Content-Type-Options: nosniff`
   - `Content-Security-Policy: default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self' data:; font-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'`
   - `X-Frame-Options: DENY` (für Browser ohne `frame-ancestors`)
   - `Referrer-Policy: no-referrer` (kein Token und kein Pfad verlassen den Ursprung, auch nicht über Links in Nachrichten)
   - `Cross-Origin-Opener-Policy: same-origin`, `Cross-Origin-Resource-Policy: same-origin` — COOP seit 2026-10-28 nur unter Loopback bzw. `localhost` (siehe Ergänzungen)
   - unter `/api/`: `Cache-Control: no-store`
6. **Oberfläche ohne Inline-Code**: kein `<script>` ohne `src`, kein `<style>`-Block, keine `style=`-Attribute im ausgelieferten HTML, keine `on…=`-Attribute; Stil über Dateien unter `/web/`, dynamische Werte über CSSOM (`element.style.…`, von `style-src 'self'` nicht erfasst). Das heutige `<style>` in `web/index.html` zieht in eine Datei um.

Nicht in I0004: Rate-Begrenzung je Herkunft (die Sitzungsgrenze aus ADR-0012 und die Verbindungsgrenze aus ADR-0010 genügen für den Beitritt), TLS (Vision: LAN, keine Verschlüsselung als Ziel).

## Konsequenzen

**Positiv:**
- DNS-Rebinding und CSRF sind für alle heutigen und künftigen Routen geschlossen, ohne dass eine Route daran denken muss.
- Die CSP macht eine spätere Lücke im Markdown-Renderer deutlich weniger ausnutzbar (kein fremdes Skript, kein Abfluss an fremde Adressen über `connect-src`).
- Die Abweisung erklärt sich selbst; der Betreiber braucht keine Dokumentation, um einen weiteren Namen freizugeben.

**Negativ / Inkauf zu nehmen:**
- `<rechnername>.fritz.box` und ähnliche Namen aus dem Router-DNS sind nicht automatisch erlaubt.
- Jede Oberflächendatei muss CSP-tauglich bleiben; ein Verstoß zeigt sich als Konsolenfehler im Browser.
- Ein Angreifer im selben LAN, der direkt per IP spricht, wird nicht abgehalten — dagegen schützt nichts ohne Anmeldung (Vision).

**Durchsetzung (Makro, Aspekte [X00002](../Architektur/Aspekte/X00002-benutzerrechte.md), [X00003](../Architektur/Aspekte/X00003-authentifizierung-und-sitzung.md)):**
- Integrationstest über das echte Programm, Matrix je Regel: `GET /`, `GET /web/<datei>`, `GET /api/gesundheit`, `POST /api/beitreten` mit `Host: evil.example` → 403; mit `Host: 127.0.0.1:<port>`, `localhost:<port>`, `[::1]:<port>` (bei `--host ::`) und einem `--erreichbar-als`-Namen → nicht 403; `POST` mit fremdem Origin, `Origin: null`, `Sec-Fetch-Site: cross-site` → 403; ohne Origin → nicht 403. Header-Satz auf 200, 403, 404 und 415.
- Mutationsprobe je Wächter (Host, Origin, Sec-Fetch-Site, jeder Header einzeln), wie für die Verbindungsgrenzen (`Dokumentation/Gelerntes.md`, 2026-10-27 Slowloris).
- E2E: jede Seite wird ohne Konsolenfehler geladen (CSP-Verstoß = roter Test).
- Review-Punkt: jede neue Route hängt unter dem Router mit `grenzschutz`; kein `GET`-Handler mit Wirkung.

**Folgeentscheidungen / Aufgaben:**
- [ ] Aufruf um `--erreichbar-als <name>` (mehrfach) erweitern, Hilfe-Text, Unit-Test im Aufruf.
- [ ] `rustix` um das Feature `system` erweitern (Rechnername); unter Nicht-Unix nur `localhost`, IP-Literale und `--erreichbar-als`.
- [ ] `<style>` aus `web/index.html` in eine Datei unter `web/` verlagern.

## Quellen / Bezuege

- [A00004 HTTP-Schnittstelle](../Architektur/A00004-http-schnittstelle.md) (primär), [A00003 Start](../Architektur/A00003-start.md), [A00005 Weboberfläche](../Architektur/A00005-weboberflaeche.md); Aspekte X00002, X00003, X00006
- ADR-0002 (Oberfläche eingebettet), ADR-0010 (Annahmeschleife), ADR-0012 (Sitzung)
- Vision R00000, Leitplanken und „Ausdrücklich nicht Ziel“; Security-Review Karte #1 (über den Auftrag des Directors); `AI-Team/gelerntes.md` (#1, LAN-Angriffsfläche)
- OWASP Cheat Sheets „Cross-Site Request Forgery Prevention“ (Abschnitt Verifying Origin with Standard Headers, Fetch Metadata) und „HTTP Headers“; W3C Fetch Metadata Request Headers (`Sec-Fetch-Site`)

## Ergänzungen

- **2026-10-28 (Karte #4, Nachzug aus #3) — Regel 5, COOP:** `Cross-Origin-Opener-Policy: same-origin` wird nur noch gesendet, wenn der `Host` einen vertrauenswürdigen Ursprung nennt (`127.0.0.0/8`, `[::1]`, `localhost`, `*.localhost`); sonst entfernt der Grenzschutz den Header (`src/http/grenzschutz.rs`, `sicherheitsheader_setzen`, `ursprung_vertrauenswuerdig`; Commit e00cdf3, Bubble B0070). Grund: ein Browser wendet COOP nur in einem sicheren Kontext an (HTML, „obtain an opener policy“) und Chromium meldet den ignorierten Header über die LAN-Adresse bei jedem Laden als Konsolenfehler — über den Normalfall der Vision wäre er nur Lärm. Security-Review #3: sicherheitlich neutral, weil der Browser COOP außerhalb sicherer Kontexte ohnehin nicht anwendet. Alle anderen Header aus Regel 5 gelten unverändert auf jeder Antwort. Restrisiken aus dem Security-Review #3, hingenommen und hier festgehalten: (a) wer die Instanz über einen **Hostnamen** anspricht (Rechnername, `--erreichbar-als`), vertraut der Namensauflösung im LAN (mDNS/LLMNR lassen sich im selben Netz fälschen) — empfohlen ist der Zugang über das IP-Literal; die Startzeile nennt die Adresse; (b) COOP hängt am `Host`-Header, nicht am tatsächlichen Kontext des Browsers — stünde ein TLS-Terminator davor, wäre die Seite für den Browser ein sicherer Kontext (`https://<name>`), der Header aber fehlte still, weil der `Host` kein Loopback nennt; die Vision sieht weder TLS noch Proxy vor (ADR-0010 Option D verworfen) — wer das ändert, braucht eine neue Entscheidung zu Regel 5. Durchsetzung: Unit-Test `coop_nur_unter_vertrauenswuerdigen_ursprungen`.
- **2026-10-28 (Karte #4 I0005) — Regel 3 und der Ereignisstrom:** Ein Halter (offener `GET /api/ereignisse`) und das Lebenszeichen jeder Anfrage mit Token sind Wirkungen eines `GET` auf die Anwesenheit. Sie verletzen die Absicht von Regel 3 nicht, weil sie nur mit dem Header `X-LocalchatA-Sitzung` entstehen: einen eigenen Header darf eine fremde Seite nur nach einer CORS-Vorabfrage senden, die die Instanz nie beantwortet (Regel 4) — ein fremder Ursprung kann also weder einen Halter erzeugen noch ein Lebenszeichen setzen. Darum gibt es für das Token **keinen** Query-Parameter ([ADR-0015](0015-ein-ereignisstrom-je-browser-sharedworker-mit-sitzungs-header.md)). Regel 3 heißt damit genauer: ein `GET` ohne Sitzungs-Header ist wirkungsfrei; mit Header wirkt er nur auf die eigene Sitzung. Durchsetzung: Integrationstest „`GET /api/ereignisse?sitzung=<gültiges token>` ohne Header → 401“. Die CSP lässt den `SharedWorker` aus `/web/` über `script-src 'self'` zu (`worker-src` fällt darauf zurück; Probe in ADR-0015).
- **2026-10-28 (Karte #5, Nachzug):** Der Wortlaut von Regel 5 verweist jetzt auf die COOP-Ergänzung oben (Abweichung aus #3, Bubble B0070, Commit e00cdf3: gesendet nur, wenn der `Host` `127.0.0.0/8`, `[::1]`, `localhost` oder `*.localhost` nennt). Neue Schreibroute `POST /api/verlassen` ([ADR-0018](0018-verlassen-sofort-weg-nie-beim-schliessen.md)) steht wie jede andere unter Regel 2; Durchsetzung dort: fremder Origin → 403, Sitzung lebt.
- **2026-10-29 (Karte #35, Befund des Betreibers zum Zugriff über DNS-Namen) — Regel 1:** [ADR-0045](0045-host-namen-des-eigenen-netzes-automatisch-oeffentliche-nur-ausdruecklich.md) (Akzeptiert, Entscheidung des Betreibers 2026-10-29) ersetzt Regel 1: zusätzlich erlaubt sind der Rückwärtsname der angesprochenen Adresse und Namen im lokalen Namensraum (einteilig, `.local`, `.home.arpa`, `.internal`), die auf eine eigene Adresse zeigen; öffentliche Namen weiter nur mit `--erreichbar-als`; die Abweisung wird für Browser eine HTML-Seite mit Link auf das IP-Literal. dazu der Schalter `--alle-namen` (jeder Host-Name, WARNING beim Start), unter dem Regeln 2 bis 6 aktiv bleiben. Regeln 2 bis 6 und die Ergänzungen oben bleiben; geändert ist nur die Statuszeile.
- **2026-10-29 (Karte #9 I0010, Anhänge; [ADR-0047](0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md) akzeptiert) — Regel 5 für Anhänge, `media-src`, neue Methoden:** (1) Antworten unter `/anhaenge/` tragen zusätzlich bzw. anstelle der App-CSP `Content-Security-Policy: sandbox; default-src 'none'`, dazu `Accept-Ranges`, `Content-Disposition`, `ETag` ([ADR-0047](0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md) Regel 3) — auf **jeder** Antwort der Route, auch 206, 304, 404, 416 und `HEAD`. (2) Die **App-CSP** wächst um `media-src 'self'`: ohne sie scheitern `<video>`/`<audio>` im Verlauf mit `NotSupportedError`, ohne eine einzige Anfrage (Probe 8 in [ADR-0047](0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md)); `data:` und `blob:` bleiben für Medien verboten. Neuer Wert: `default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self' data:; media-src 'self'; font-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'`; Durchsetzung: der Unit-Test der Konstante (`src/http/grenzschutz.rs:731`) prüft den neuen Wert. (3) `PUT` und `DELETE /api/anhaenge/{id}` stehen wie jede Schreibroute unter Regel 2 (Origin) und brauchen den Sitzungs-Header; als nicht „einfache“ Methoden kämen sie von einem fremden Ursprung nur nach einer CORS-Vorabfrage, die die Instanz nie beantwortet (Regel 4). (4) `GET /anhaenge/{id}` ist die erste Route mit Inhalt **ohne** Sitzung; Regel 3 hält, weil sie keine Wirkung hat (kein Lebenszeichen, kein Zähler außer Protokoll). Die übrigen Regeln gelten unverändert.
- **2026-10-30 (Karte #9; code-reviewer #9 Wiederholung, Sollte S1; security-reviewer #9: kein Sicherheitsbefund) — jede Abweisung des Grenzschutzes schließt die Verbindung:** Entscheider: Claude (autonom, architekt, Auftrag des Directors). Status: **Akzeptiert**.

  **Anlass.** [ADR-0046](0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) Ergänzung „`Connection: close` auf jeder PUT-Antwort“ verlangt die Regel „gleich welcher Status“. Der `code-reviewer` hat geprobt: ein `PUT /api/anhaenge/{id}` mit fremdem `Host` endet im Grenzschutz mit 403 **ohne** `Connection: close`; ein auf derselben Verbindung angehängtes `GET` wird beantwortet. Der Grenzschutz (`src/http/grenzschutz.rs`, Schicht vor jeder Route) erzeugt die Antwort, bevor der Handler läuft — die Regel des Handlers erreicht ihn nicht. Sicherheit: kein Befund (die angehängte Anfrage durchläuft selbst den Grenzschutz); aber eine Regel mit unbemerkter Ausnahme, und bei einem abgewiesenen Rohkörper (bis 2 GiB angekündigt) bliebe die Verbindung offen, solange hyper den ungelesenen Körper verwirft.

  | | Regel | Stärke | Schwäche |
  |---|---|---|---|
  | S-a | ADR-0046 einengen: „jede Antwort **des Handlers**“; Grenzschutz-403 hält die Verbindung | keine Änderung am Code | die Regel bekommt eine Ausnahme, die man im Review kennen muss; ein abgewiesener `PUT` mit großem `Content-Length` hält die Verbindung, bis der Körper verworfen ist — verworfen |
  | S-b | im Grenzschutz eine Weiche nach Pfad: `Connection: close` nur für `PUT /api/anhaenge/…` | trifft genau den Befund | zweite Pfadweiche im Grenzschutz (wie die CSP für `/anhaenge/`); jede neue Rohkörper-Route muss daran denken — verworfen |
  | **S-c (gewählt)** | **jede Abweisung des Grenzschutzes trägt `Connection: close`** — 403 wegen `Host`, 403 wegen `Origin`/`Sec-Fetch-Site`, für jede Route und jede Methode | eine Regel ohne Pfad; kein Körper einer abgewiesenen Anfrage wird gelesen oder verworfen; der Platz der Verbindung (160 je Gegenstelle) ist sofort frei; die Regel „gleich welcher Status“ in ADR-0046 gilt damit ohne Ausnahme | ein gutartiger Browser mit falschem Namen baut je Anfrage neu auf — er sieht ohnehin nur die Abweisung |

  **Regel** (ergänzt Regeln 1 und 3): Antwortet der Grenzschutz selbst (Abweisung wegen `Host` oder Ursprung), setzt er **`Connection: close`**; die Verbindung endet nach der Antwort, ein angehängter oder folgender Körper wird nicht gelesen. Antworten, die die Route erzeugt, behalten ihre eigenen Regeln (für `PUT /api/anhaenge/{id}`: ADR-0046, immer `close`).

  **Abnahme-Probe** (Kriteriumstext für den `planer`; Rohsocket wie `tests/anhaenge_hochladen.rs:69`): (1) Anmelden 10 Bytes; auf **einer** Verbindung in einem `write`: `PUT /api/anhaenge/<id>` mit `Host: fremd.example`, `Content-Length: 10`, 10 Bytes, unmittelbar dahinter `GET /api/gesundheit` mit gültigem `Host` → genau **eine** Antwort, 403 mit `connection: close`, danach EOF; heute zwei Antworten (rot). (2) Dasselbe für `POST /api/nachrichten` mit fremdem `Origin` → eine Antwort 403 `close`, EOF. (3) `PUT /api/anhaenge/<id>` mit fremdem `Host` und `Content-Length: 2147483648`, nur der Kopf gesendet → 403 sofort, danach EOF, ohne dass der Client Körper sendet. (4) Kontrolle: zwei `GET /api/gesundheit` mit gültigem `Host` hintereinander auf einer Verbindung → zwei Antworten 200 (Keep-Alive für Gutes bleibt). Mutante „Grenzschutz ohne `close`“ → (1) rot.

  **Baustein für den `planer`/`entwickler`:** eine Bubble in `src/http/grenzschutz.rs` (an der einen Stelle, die die Abweisung baut: `abweisung`/`host_abweisung`), Test zuerst; R00029 Kriterium zum `PUT` nennt „auch die 403 des Grenzschutzes“.
- **2026-10-30 (Karte #71, Befund entw2 #9 Teil C Offen 1 und security-reviewer #9 Wiederholung Teil C) — Regel 5 für Anhänge: Medien-Dokument bekommt `media-src 'self'`, sonst bleibt die Anhang-CSP; App-CSP unverändert bis B0460:** Entscheider: Claude (autonom, architekt, Auftrag des Directors). Status: **Vorgeschlagen**, bis die Proben M-1 und M-2 unten in Chromium grün sind (heute nur aus Spezifikation und Konsolenmeldung belegt; in dieser Nacht lief kein Browser). Ort: hier, weil die Entscheidung die CSP-Weiche in `src/http/grenzschutz.rs` ändert (Ergänzung Karte #9, Punkt 1); [ADR-0047](0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md) trägt nur einen Verweis (Regel 3, Probe 12).

  **Anlass.** Ein direkter Aufruf von `/anhaenge/<id>` eines Videos oder Tons spielt nicht; Konsole: „violates … `default-src 'none'` … `media-src` was not explicitly set“. Das widerspricht der Entscheidung des Betreibers F2 („Direktlink darf abspielen“). Ursache: der Browser baut für eine Navigation auf Medien sein Medien-Dokument (`<video src=<eigene URL>>`), und dessen Abruf unterliegt der CSP der Antwort — `default-src 'none'` verbietet ihn. ADR-0047 Probe 12 hat nur belegt, dass das Dokument `autoplay` setzt und kein Skript läuft, nicht, dass es spielt; die Probe 9 (Player im Verlauf) lief unter der App-CSP mit `media-src 'self'`, wo die Medien-Antwort selbst nur als Quelle diente.

  **Belege (Spezifikation, nicht Browserlauf).** (a) CSP3 §2.2/§2.1 („Parse a response’s Content Security Policies“): bei einer per Kopfzeile gelieferten Richtlinie ist die `self-origin` der CSP-Liste **der Ursprung der Antwort-URL**, nicht der des Dokuments; `'self'` trifft damit unter `sandbox` (undurchsichtiger Dokumentursprung) weiter den Ursprung der Instanz — Abruf 2026-10-30, `https://www.w3.org/TR/CSP3/`. (b) CSP3 §6.1.8: `media-src` regelt Video, Audio und Text-Spuren; ohne sie fällt der Abruf auf `default-src` zurück (§6.1 Fallback). (c) `sandbox` ist eine Dokument-Direktive und unabhängig von den Abruf-Direktiven; `media-src` weitet sie nicht auf. (d) **Zweites mögliches Hindernis:** Fetch §3.7 (Cross-Origin Resource Policy, Abruf 2026-10-30, `https://fetch.spec.whatwg.org/`): bei `same-origin` wird ein `no-cors`-Abruf blockiert, wenn der Ursprung des Anfragenden nicht derselbe ist wie der der Antwort — ein sandboxed Dokument hat einen undurchsichtigen Ursprung, ist also nie „same origin“. Die Antworten der Route tragen `Cross-Origin-Resource-Policy: same-origin` (Regel 5). Der Befund zeigt die CSP-Meldung zuerst; die CORP-Sperre käme erst danach und wäre bisher verdeckt. Firefox ist weder geprobt noch aus der Browser-Doku belegt.

  | Variante | Wirkung | Stärke | Schwäche |
  |---|---|---|---|
  | **C1 (gewählt): CSP je Art — Medienarten `sandbox; default-src 'none'; media-src 'self'`, alles andere unverändert `sandbox; default-src 'none'`** | Medien-Dokument darf genau die eigene URL laden | `sandbox` und `default-src 'none'` bleiben auf jeder Antwort; `media-src` liefert kein Skript, keinen Stil, kein Bild, kein Netz; nur die sechs vom Server bestimmten Medien-Content-Types | eine zweite Konstante und eine Entscheidung nach Art an der Weiche (Marker); CORP kann zusätzlich nötig sein (Regel 3) |
  | C2: `media-src 'self'` auf **jede** Anhang-Antwort | ein Fall weniger in der Weiche | kein Marker nötig | PDF, Text, Bild, unbekannte Arten erlauben Medien-Abrufe ohne Anlass — mehr Fläche als gebraucht; die Weiche bräuchte keine Art, aber „nur Gefahrloses“ (Auftrag, M14) wird zu „alles“ — verworfen |
  | C3: `media-src *` oder `media-src 'self' data: blob:` | spielt auch bei Eigenheiten des Browsers | robust gegen Auslegung von `'self'` unter `sandbox` | erlaubt Abrufe an fremde Adressen aus einem Dokument, dessen Inhalt Fremde bestimmen; `data:`/`blob:` ADR-0047 Regel 7 verboten — verworfen (Mutante unten) |
  | C4: `sandbox` weglassen oder `allow-same-origin` für Medienarten | spielt sicher | einfachste CSP | das Medien-Dokument liefe im Ursprung der App; ein Polyglot (ADR-0047 Probe 13, M2) oder ein Fehlurteil der Art-Erkennung stünde dann einen Schritt vor `localStorage` — bricht M14, verworfen |
  | C5: Medien beim direkten Aufruf als `attachment` | nie Abspielen, keine CSP-Änderung | kein Risiko | widerspricht F2 und ADR-0047 M3 (verworfen aus denselben Gründen) |
  | C6: `img-src 'self'` zusätzlich | — | — | das Medien-Dokument braucht kein Bild (kein `poster`); ohne Verbraucher kein Recht — verworfen |

  **Entscheidung: C1.** Es überwiegt, dass die Aufweichung auf das Mindeste fällt, was der Befund verlangt (eine Abruf-Direktive, nur für die eigene Antwort-URL, nur für Arten, die der Server aus dem Inhalt bestimmt hat), während die Linie gegen Skript (`sandbox`, `default-src 'none'`, `nosniff`) wortgleich bleibt. In Kauf genommen: die Gültigkeit von `'self'` unter `sandbox` hängt an der Auslegung des Browsers (Spezifikation eindeutig, Chromium ungeprobt, Firefox unbekannt); die Art-Entscheidung rückt in die Weiche.

  **Regeln** (ändern Punkt 1 der Ergänzung Karte #9 und ADR-0047 Regel 3, erster Spiegelstrich, nur für Medienarten):

  1. **Konstanten.** Neben `ANHANG_CSP` (`sandbox; default-src 'none'`) steht `ANHANG_CSP_MEDIUM` = genau `sandbox; default-src 'none'; media-src 'self'`. Es gibt keine dritte Anhang-CSP.
  2. **Marker, nicht Kopfzeile.** Der Handler von `GET|HEAD /anhaenge/{id}` setzt keine CSP (Ergänzung Karte #9 Punkt 1 bleibt), sondern hängt der Antwort einen internen Marker (Typ in den Antwort-Erweiterungen, nicht eine Kopfzeile, von außen nicht setzbar) an, **wenn die vom Server aus dem Inhalt bestimmte Art** (ADR-0047 Frage 3) eine der sechs Medienarten ist: `video/mp4`, `audio/mp4`, `video/webm`, `video/ogg`, `audio/ogg`, `audio/mpeg`. Nie nach dem `Content-Type` der Anfrage, dem Namen, einem Header des Anfragenden oder dem Pfad.
  3. **Weiche.** `sicherheitsheader_setzen` wählt unter `/anhaenge` (roh oder dekodiert): Marker da → `ANHANG_CSP_MEDIUM`, sonst `ANHANG_CSP`; alle anderen Kopfzeilen des Satzes unverändert. Ohne Marker (404, 403, 405, 410, Router-Antworten, Nicht-Medienarten) gilt `ANHANG_CSP`. Auf **allen** Antworten einer Medien-Datei (200, 206, 304, 416, `HEAD`) gilt der Marker gleich, damit die Kopfzeile nicht je nach Status wechselt.
  4. **CORP.** Zeigt Probe M-2, dass `Cross-Origin-Resource-Policy: same-origin` das Medien-Dokument sperrt, tragen **nur die Antworten mit Marker** `Cross-Origin-Resource-Policy: cross-origin`; alle anderen Antworten der Route behalten `same-origin`. Begründung: das Recht, die Bytes zu holen, ist die unratbare ID; eine fremde Seite liest `no-cors`-Medien nicht (undurchsichtige Antwort), `Host`- und Origin-Prüfung (Regeln 1, 2) bleiben. Die Weiche entscheidet das mit demselben Marker; dieser Fall braucht eine Bewertung des `security-reviewer` vor dem Zusammenführen.
  5. **App-CSP: nein, nicht jetzt.** `media-src 'self'` in der App-CSP bleibt bei B0460 (Player im Verlauf, R00030 F0102; ADR-0047 Regel 7) — der Direktlink ist ein eigenes Dokument mit eigener CSP und braucht sie nicht, und ohne Player im Verlauf gäbe es keinen Verbraucher. Die App-CSP ändert sich in dieser Karte nicht (Unit-Test der Konstante unverändert).
  6. **Nicht verändert:** `script-src`, `style-src`, `img-src`, `connect-src` und jede andere Direktive der Anhang-CSP — es gibt keine; `X-Content-Type-Options: nosniff`, `X-Frame-Options`, `Referrer-Policy` und `Content-Disposition` nach ADR-0047 Frage 3.

  **Abnahme-Proben** (Kriteriumstexte für den `planer`; Integration gegen das echte Programm, E2E in Chromium auf freiem Port, Dateien aus `ffmpeg` wie in ADR-0047 Probe 2):

  - **M-1 (Spike zuerst, heute rot).** Je eine Datei WebM (VP8/Opus), MP4 (VP9/Opus; H.264 nur, wo das Chromium es kann) und MP3 hochladen, senden, `/anhaenge/<id>` als Navigation öffnen → Medien-Dokument: `canplay` feuert, nach `play()` rückt `currentTime` vor (> 0,5 s), `error` ist `null`, Konsole ohne CSP-Meldung (`securitypolicyviolation` bleibt aus), Antwortkopf `Content-Security-Policy` genau `ANHANG_CSP_MEDIUM`; ein Sprung ans Ende (`currentTime = duration − 0,1`) ergibt 206 und `ended`.
  - **M-2 (CORP).** Dieselbe Probe mit `Cross-Origin-Resource-Policy: same-origin` auf der Antwort: spielt sie → Regel 4 entfällt; spielt sie nicht (Konsole/Netz: blockiert durch CORP) → Regel 4 gilt, danach M-1 grün und eine **fremde Seite** (anderer Port) mit `<video src=…/anhaenge/<id>>` darf per `fetch(…, {mode:"cors"})` nichts lesen (Fehler).
  - **M-3 (kein Skript, weiter).** Anhänge `text/html`, SVG mit `<script>`, XHTML, ein Bild-Name über HTML-Inhalt (`.png`), ein WebM und ein MP3 mit angehängtem `<script>`-HTML (Polyglot), HTML mit `ID3`- und `OggS`-Präfix (wird Medienart): über direkte Navigation → in keinem Fall läuft ein Skript (Sentinel `window.__x` bleibt ungesetzt), `localStorage` der App unerreichbar (`SecurityError`/Titel leer); HTML, SVG und XHTML erscheinen als Quelltext; ihre Antwort trägt `ANHANG_CSP` (ohne `media-src`).
  - **M-4 (Kopfzeilen je Art und Status, Integration).** Je Art der Tabelle (ADR-0047 Frage 3) und je Status 200, 206, 304, 416, `HEAD`, 404 (unbekannte ID), 410 (gelöscht): genau `ANHANG_CSP_MEDIUM` nur bei den sechs Medienarten mit Datei, sonst genau `ANHANG_CSP`; `nosniff`, `sandbox` in jedem Fall.
  - **M-5 (Marker nicht fälschbar).** Anfrage mit `Content-Type: video/mp4`, mit Name `x.mp4`, mit Header `X-Medium: 1` auf eine PDF-, Text- und Bild-Datei → `ANHANG_CSP`.
  - **Mutanten** (je eine Probe rot): „`media-src *`“, „`media-src 'self' data: blob:`“, „`sandbox` fehlt“ (nur Medienart), „`allow-same-origin` dazu“, „`default-src` fehlt“, „`script-src 'self'` dazu“, „`img-src` dazu“, „Medien-CSP auf jeder Anhang-Antwort (C2)“, „Medien-CSP auf 404/410“, „Marker aus dem `Content-Type` der Anfrage“, „Marker nur auf 200, nicht auf 206/HEAD“, „CORP `cross-origin` auf allen Anhang-Antworten“ (nur falls Regel 4 gilt).
  - Firefox: Probe M-1 und M-3 einmal von Hand, Ergebnis als Anmerkung; kein Gate (Spezifikation trägt, Browser ungeprobt).

  **Pre-Mortem** (ein Monat danach gescheitert, weil …): (1) `'self'` unter `sandbox` in einem Browser anders gilt → Medien spielen dort per Direktlink nicht; Folge: Hinweis „Laden“ statt Abspielen, kein Sicherheitsverlust — C3 bleibt verworfen. (2) Jemand ergänzt eine siebte Medienart (z. B. WAV) in die Tabelle, ohne den Marker → spielt nicht (fail-closed); die Zuordnung steht als eine Tabelle mit Test neben der Art-Erkennung. (3) Die Weiche gibt dem Handler Macht über die CSP → der Marker ist ein Typ mit nur zwei Wirkungen (Konstante wählen), keine freie Zeichenkette.

  **Bausteine** (klein, je Bubble Test zuerst; Reihenfolge = Ausführung):

  1. **Spike M-1/M-2** im Scratchpad (Python-`http.server` oder das Programm, Playwright-Chromium): drei Dateien, vier Kopf-Varianten (`ANHANG_CSP`, `ANHANG_CSP_MEDIUM`, mit/ohne CORP `same-origin`) — Ergebnis als Zeilen in dieser Ergänzung unter „Erprobt“; entscheidet Regel 4. Kein Produktivcode.
  2. `src/http/grenzschutz.rs`: Konstante `ANHANG_CSP_MEDIUM` plus Marker-Typ `AnhangMedium` (pub(crate)); Unit-Test der Konstante auf den Wortlaut.
  3. `src/http/grenzschutz.rs`: Weiche wählt nach Marker (und CORP nach Regel 4, falls Spike es verlangt); Unit-Test erweitert `die_csp_weiche_nach_dem_pfad` um Marker ja/nein, Mutanten „Weiche ignoriert Marker“, „Marker auf Nicht-Anhangpfad bleibt wirkungslos“ (außerhalb `/anhaenge` gilt die App-CSP).
  4. Route der Anhänge (`GET|HEAD`): Marker anhängen, wenn die Art eine der sechs Medienarten ist; Tabelle `Art → Marker ja/nein` als benannte Konstante neben der Art-Erkennung (A00018); Unit-Test über alle Arten der Tabelle.
  5. Integrationstest M-4 und M-5 (echtes Programm, Rohantwort), Mutationsprobe je Mutant der Liste.
  6. E2E M-1 und M-3 in Chromium (`tests/`-Playwright wie die bestehende Anhang-Suite), zuerst rot gegen den Stand `main`.

  **Folgen:** ADR-0047 Regel 3 (Verweis), Probe 12 (Korrektur); [A00005](../Architektur/A00005-weboberflaeche.md) und die Anhang-Bausteine nennen die zweite CSP; `R00029` Kriterium „Kopfzeilen“ und `R00030` (Direktlink) erhalten M-1 bis M-5 — beides Sache des `planer`.

  **Erprobt (2026-10-30, entwickler, B0660 Spike; Playwright-Chromium 143.0.7499.4 headless, Python-Server im Scratchpad mit Range-Unterstützung, `nosniff` auf jeder Antwort; Dateien aus `ffmpeg`, je 4 s).** Direkte Navigation auf die Datei, danach `play()` (stumm), 1,5 s gewartet, Sprung auf `duration − 0,1`:

  | Datei | `ANHANG_CSP` | `ANHANG_CSP` + CORP `same-origin` | `ANHANG_CSP_MEDIUM` | `ANHANG_CSP_MEDIUM` + CORP `same-origin` | `ANHANG_CSP_MEDIUM` + CORP `cross-origin` |
  |---|---|---|---|---|---|
  | WebM VP8/Opus | blockiert durch CSP | blockiert durch CSP | **spielt** | **blockiert durch CORP** | spielt |
  | MP4 VP9/Opus | blockiert durch CSP | blockiert durch CSP | **spielt** | **blockiert durch CORP** | spielt |
  | MP3 | blockiert durch CSP | blockiert durch CSP | **spielt** | **blockiert durch CORP** | spielt |
  | MP4 H.264/AAC (zusätzlich; dieses Chromium kann es) | blockiert durch CSP | blockiert durch CSP | **spielt** | **blockiert durch CORP** | spielt |

  - *blockiert durch CSP*: Konsole „Loading media from '…' violates the following Content Security Policy directive: "default-src 'none'". Note that 'media-src' was not explicitly set …“, `readyState` 0, `networkState` 3 (keine Quelle), `currentTime` bleibt 0, nur die Navigationsantwort 200, kein Medienabruf.
  - *blockiert durch CORP*: Konsole „Failed to load resource: net::ERR_BLOCKED_BY_RESPONSE.NotSameOrigin“, der Medienabruf (`Range: bytes=0-` → 206) wird vom Browser verworfen, `readyState` 0. Bestätigt Beleg (d): das sandboxed Medien-Dokument hat einen undurchsichtigen Ursprung und ist für CORP `same-origin` nie derselbe Ursprung.
  - *spielt*: `readyState` 4, `error` `null`, Konsole leer, `currentTime` nach 1,5 s ≈ 1,46 s, nach dem Sprung `ended` = `true`; Abrufe 200 (Navigation) und 206 (`Range: bytes=0-`). Bei Dateien dieser Größe (33–82 KiB) lädt der erste Medienabruf die ganze Datei, der Sprung löst **keinen** weiteren Abruf aus — für „Sprung ans Ende ergibt 206“ (M-1) braucht der E2E eine Datei, die größer ist als der erste Abruf, oder er wertet die 206 des ersten Medienabrufs.
  - Fremde Seite (anderer Port, `127.0.0.1`) mit `<video src=…>` und `fetch`: bei CORP `same-origin` lädt das Video nicht (`ERR_BLOCKED_BY_RESPONSE.NotSameOrigin`), `fetch(…, {mode: "cors"})` → `TypeError`. Bei CORP `cross-origin` **spielt das eingebettete Video auf der fremden Seite** (`readyState` 4, `duration` 4,008 lesbar), `fetch(…, {mode: "cors"})` → `TypeError` (kein `Access-Control-Allow-Origin`), `fetch(…, {mode: "no-cors"})` → undurchsichtige Antwort (`type` `opaque`, Status 0, 0 Bytes).
  - Werkzeughinweis für B0665: im sandboxed Medien-Dokument laufen keine Seiten-Timer (`setTimeout` in `page.evaluate` kehrt nie zurück) und `page.goto` erreicht `load` nicht, solange das Medium blockiert ist — Navigation mit `waitUntil: 'commit'`, im Dokument nur synchron abfragen, gewartet wird in Node.

  **Ergebnis des Spikes: Regel 4 gilt.** Mit der Medien-CSP allein spielt der Direktlink in Chromium nicht, weil CORP `same-origin` (heute auf jeder Antwort der Route, `kopfzeilen::route` und Header-Satz) das Medien-Dokument sperrt; erst CORP `cross-origin` auf den Medienantworten lässt ihn spielen. Preis (geprobt): jede fremde Seite, die die ID kennt, kann das Medium einbetten und abspielen und dabei Dauer und Maße lesen; die Bytes liest sie weder per `cors` noch per `no-cors`. Der Bau von B0662–B0665 ruht, bis der `security-reviewer` diesen Fall bewertet hat (Auftrag des Directors: „STOPP und melden“). Status bleibt **Vorgeschlagen**. Firefox ungeprobt.
