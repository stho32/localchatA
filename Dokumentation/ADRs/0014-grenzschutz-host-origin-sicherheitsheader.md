# ADR-0014: Grenzschutz im LAN — Host-Erlaubnisliste, Origin-Prüfung für Schreibendes, Sicherheitsheader auf jeder Antwort

**Status**: Akzeptiert
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
   - `Cross-Origin-Opener-Policy: same-origin`, `Cross-Origin-Resource-Policy: same-origin`
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
