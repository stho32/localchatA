---
id: R00003
status: Neu
datum: 2026-10-27
---

# R00003: Namen wählen und beitreten

## Beschreibung

Wer mitschreiben will, gibt sich einen Namen; die Instanz ergänzt ihn um die Herkunft der Verbindung zur Kennung `name@herkunft` und weist einen Namen ab, den schon jemand trägt — egal von wo. Ein Mensch tut das im Browser auf dem Beitreten-Bildschirm und sieht danach seine Kennung in der Kopfzeile; Neuladen, neue Tabs und ein Neustart der Instanz führen ihn ohne erneutes Tippen zurück. Dieselbe Route steht Skripten und später Agenten offen (`POST /api/beitreten`, `GET /api/ich`).

Weil `beitreten` die erste Route mit Wirkung ist, kommt mit R00003 der Grenzschutz für alle Routen: freigegebene Host-Namen gegen DNS-Rebinding, Ursprungsprüfung gegen fremde Websites (CSRF) und Sicherheitsheader auf jeder Antwort. Und weil der Name ab jetzt in Kennung, Log und später in Ordnernamen und Erwähnungen steht, gilt die Namensregel aus ADR-0013 auch beim Lesen der Logs.

Zahlt ein auf: [Vision](R00000-vision.md) — „Mensch und KI sind gleichberechtigt“ (Name plus Herkunft, jeder Name nur einmal, keine Anmeldung) und „Über Rechnergrenzen“ (eine Instanz im LAN, die fremde Websites nicht fernsteuern können).

Abweichung von der Vision, angesprochen statt überschrieben: die Vision nennt als Herkunft „IP-Adresse oder Rechnername“ mit Beispielen wie `claude@localhost` und `claude-entwickler@server.de`. R00003 baut nach ADR-0012/ADR-0013 nur die Textform der IP-Adresse (`claude@127.0.0.1`); Rechnernamen wären eine spätere Erweiterung (offene Frage an den Betreiber).

Slices (WBS `Dokumentation/Planung/localchata.md`, alle unter I0004 Namen wählen und beitreten): F0006 Grenzschutz an der Systemgrenze, F0007 Namensregel in der Kennung, F0008 Beitreten mit Namen über die API, F0009 Sitzung wiedererkennen und nach der Gnadenfrist freigeben, F0010 Beitreten-Bildschirm.

## Geschäftlicher Nutzen

Ohne Namen gibt es keinen Absender: keine Nachricht (I0009), keine Teilnehmerliste (I0005), keine Direktnachricht (I0012) und kein Agent (I0014) kann entstehen. R00003 legt die Identität so, dass sie im LAN ohne Anmeldung trägt: die Herkunft kommt aus dem Socket und nicht vom Client, ein Name ist nur einmal vergeben, und Verwechslungen über Groß-/Kleinschreibung, Umlaute oder unsichtbare Zeichen sind ausgeschlossen. Der Grenzschutz verhindert, dass eine beliebige Website, die ein Teilnehmer gerade offen hat, über seinen Browser Namen besetzt oder später Verlauf liest. Für den Menschen ist der Beitritt ein Feld und eine Taste; danach merkt sich der Browser die Sitzung.

## Funktionale Anforderungen

### Grenzschutz (F0006, ADR-0014)

- **Host**: Jede Anfrage, deren `Host`-Header (ohne Port, IPv6 in eckigen Klammern) kein IP-Literal, nicht `localhost`, nicht der Rechnername oder `<rechnername>.local` (ohne Beachtung der Groß-/Kleinschreibung) und nicht mit `--erreichbar-als <name>` freigegeben ist, oder die keinen `Host`-Header trägt, wird mit 403 abgewiesen; der Text lautet „Diese Instanz ist unter dem Namen <name> nicht freigegeben — über die IP-Adresse öffnen oder mit --erreichbar-als <name> starten.“
- **`--erreichbar-als <name>`**: neue Option des Aufrufs, mehrfach erlaubt; ohne Wert ist der Aufruf falsch (Exit 2); `--hilfe` nennt sie. Rechnername = `uname -n`; ohne Unix gelten nur IP-Literale, `localhost` und `--erreichbar-als`.
- **Ursprung**: jede Methode außer `GET`, `HEAD`, `OPTIONS` wird mit 403 `{fehler}` abgewiesen, wenn `Origin` gesetzt ist und nicht genau `http://<Host-Header>` lautet (`Origin: null` ist nie gleich), oder wenn `Sec-Fetch-Site` gesetzt ist und nicht `same-origin` oder `none` lautet. Fehlen beide, gilt die Anfrage als nicht vom Browser und wird nicht abgewiesen.
- **Lesende Methoden** `GET` und `HEAD` ändern nie Zustand; JSON-Routen verlangen `Content-Type: application/json` (sonst 415).
- **CORS**: die Instanz beantwortet keine Vorabfrage zustimmend und setzt nie `Access-Control-Allow-*`.
- **Sicherheitsheader auf jeder Antwort** (auch 403, 404, 415): `X-Content-Type-Options: nosniff`; `Content-Security-Policy: default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self' data:; font-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'`; `X-Frame-Options: DENY`; `Referrer-Policy: no-referrer`; `Cross-Origin-Opener-Policy: same-origin`; `Cross-Origin-Resource-Policy: same-origin`; unter `/api/` zusätzlich `Cache-Control: no-store`.
- **Oberfläche ohne Inline-Code**: das ausgelieferte HTML enthält kein `<style>`, kein `style=`, kein `on…=`-Attribut und kein `<script>` ohne `src`; der heutige Seitenstil zieht in eine Datei unter `/web/`.

### Namensregel (F0007, ADR-0013)

- **Name**: 1 bis 32 Zeichen aus `a–z`, `0–9` und `-`, nicht mit `-` beginnend oder endend. Zwei Namen sind gleich, wenn sie Zeichen für Zeichen gleich sind.
- **Herkunft**: Textform der kanonischen IP-Adresse der Gegenstelle, Zeichen `[0-9a-f.:]`, 1 bis 45 Zeichen; IPv4-gemappte IPv6-Adressen erscheinen als IPv4 (`127.0.0.1`, nie `::ffff:127.0.0.1`); kein `localhost`, keine Namensauflösung.
- **Kennung** = Name `@` Herkunft. Eine Kopfzeile im Log, deren Kennung eine der beiden Regeln verletzt, ist beim Laden eine defekte Zeile (wie in R00002).
- **Reserviert**: `alle` und `hier` (für spätere Sammel-Erwähnungen, I0013) sind beim Beitreten nicht wählbar.

### Beitreten (F0008, ADR-0012)

- `POST /api/beitreten` mit JSON `{"name": …, "art": …}`; `art` ist `mensch` oder `agent`, fehlt sie, gilt `agent` (die Oberfläche sendet `mensch`). Antwort 201 `{token, kennung, name, herkunft, art, gnadenfrist_s}`; `token` sind 32 Hex-Zeichen aus dem Zufall des Betriebssystems, `gnadenfrist_s` ist 60.
- **Name vergeben** (lebende Sitzung mit diesem Namen, egal welche Herkunft) → 409 `{fehler, belegt_von, seit, vorschlag}`; `belegt_von` ist die volle Kennung des Trägers, `seit` der Zeitpunkt seines Beitritts (RFC 3339), `vorschlag` der erste freie Name aus `<name>-2`, `<name>-3`, …; wird er länger als 32 Zeichen, wird der Stamm gekürzt.
- **Regelverstoß** → 422 `{fehler, regel, vorschlag?}`; `vorschlag` ist die Umformung (klein, Umlaute zu `ae/oe/ue/ss`, sonst nicht Erlaubtes zu `-`, Mehrfach-`-` zusammengefasst, Ränder entfernt, auf 32 gekürzt), wenn sie nicht leer ist. Der Server formt nie still um. Reservierter Name oder unbekannte `art` → 422.
- **Grenzen gegen Besetzen**: höchstens 64 lebende Sitzungen je Herkunft und 512 insgesamt; darüber 429 `{fehler}`.
- Prüfen und Belegen eines Namens geschehen atomar: gleichzeitige Beitritte mit demselben Namen ergeben genau einen Erfolg.
- Das Token steht nur in der Antwort auf `beitreten` — nie im Log, auf stdout, auf stderr oder in einer Kennung. Ein Neustart beendet alle Sitzungen; Namen werden über den Neustart nicht reserviert.

### Sitzung (F0009, ADR-0012)

- Nachweis bei jedem Aufruf mit Sitzung: Header `X-LocalchatA-Sitzung: <token>`, gültig nur von derselben Herkunft wie beim Beitritt.
- `GET /api/ich` → 200 `{kennung, name, herkunft, art}`; fehlt das Token, ist es unbekannt, abgelaufen oder von anderer Herkunft → 401 `{fehler}`.
- **Wiederkehr**: `POST /api/beitreten` mit gültigem Token derselben Herkunft im Header → 200 mit derselben Sitzung (gleiches Token, gleiche Kennung); ein anderer Name im Body → 422, die Sitzung bleibt unverändert; ein unbekanntes oder abgelaufenes Token zählt nicht — dann gilt der Aufruf als neuer Beitritt mit dem Namen aus dem Body.
- **Lebensdauer**: jede Anfrage mit gültigem Token ist ein Lebenszeichen. Liegt das letzte Lebenszeichen länger als die Gnadenfrist zurück, ist die Sitzung weg: Token ungültig, Name frei. Geprüft wird beim Zugriff; Halter (offene Ströme) kommen erst mit I0005.
- **Gnadenfrist**: Standard 60 s; Testknopf `LOCALCHATA_GNADENFRIST_MS` (Millisekunden). Ein unlesbarer Wert beendet den Start mit Exit 2 und einer Zeile auf stderr, die `LOCALCHATA_GNADENFRIST_MS` nennt.

### Beitreten-Bildschirm (F0010, Designsystem Bildschirm 4.1)

- Ohne gemerkte Sitzung zeigt die Seite den Beitreten-Bildschirm: Namensfeld mit Fokus und Schalter „Beitreten“; `Enter` im Feld tritt bei.
- Beim Tippen zeigt die Seite einen Hinweis, wenn der Name die Regel verletzt, samt Vorschlag; maßgeblich ist die 422-Antwort des Servers.
- Nach dem Beitritt zeigt die Seite die Kopfzeile mit der eigenen Kennung `name@herkunft`; mehr Hauptansicht gibt es in I0004 nicht.
- **Name vergeben**: der Bildschirm nennt die Kennung des Trägers und die Uhrzeit seines Beitritts (`HH:MM`) und füllt das Feld mit dem Vorschlag vor.
- **Server nicht erreichbar**: der Bildschirm sagt es mit ⚠ und Wort und bietet „Erneut versuchen“.
- **Wiederfinden**: Token und zuletzt gewählter Name liegen im `localStorage` des Ursprungs (`localchatA.sitzung`, `localchatA.name`); alle Tabs eines Browsers teilen die Sitzung. Beim Laden: Token vorhanden → `GET /api/ich`; 200 → Kopfzeile; 401 → still `beitreten` mit dem gemerkten Namen; 201 → Kopfzeile; 409 → Bildschirm im Zustand „Name vergeben“. Kein Token und kein Name → Beitreten-Bildschirm.
- Bedienbar ohne Maus, Fokus immer sichtbar; Text vom Server nur als Text, nie als HTML.

## Nicht-funktionale Anforderungen

- Sicherheit: Grenzschutz als äußerste Schicht vor jeder Route (auch 404); je Wächter (Host, Ursprung, `Sec-Fetch-Site`, jeder Sicherheitsheader) mindestens ein Test, der rot wird, wenn der Wächter fehlt (Mutationsprobe, `AI-Team/gelerntes.md`, `Dokumentation/Gelerntes.md`).
- Sicherheit: Token mit 128 Bit aus dem Zufall des Betriebssystems, nur im Speicher.
- Benutzerfreundlichkeit: jede Abweisung nennt Grund und Ausweg (403-Text, 409 mit Vorschlag, 422 mit Vorschlag); Oberfläche deutsch nach Designsystem.
- Testbarkeit: Uhr, Zufall und Gnadenfrist sind ersetzbar (Uhr-Attrappe, feste Zufallsfolge, `LOCALCHATA_GNADENFRIST_MS`); Tests mit `--port 0`, Wegwerf-Ordnern und zweiter Absenderadresse `127.0.0.2` (Linux-Loopback).

## Akzeptanzkriterien

Gemeinsam für alle Kriterien: Start mit `localchatA <ordner> --port 0 --host 127.0.0.1 --kein-browser`, `<p>` ist der Port aus der Startzeile, `<rn>` die Ausgabe von `uname -n`. „Header-Satz“ heißt alle sechs Sicherheitsheader aus den funktionalen Anforderungen mit genau diesen Werten (unter `/api/` zusätzlich `Cache-Control: no-store`).

### F0006 Grenzschutz an der Systemgrenze — Angriffsmatrix

Jede Zeile ist ein Integrationstest über das echte Programm.

| Nr | Start | Anfrage | Erwartet |
|---|---|---|---|
| G1 | Standard | `GET /` mit `Host: evil.example` | 403, Text nennt `evil.example` und `--erreichbar-als`, Header-Satz |
| G2 | Standard | `GET /web/lc-tokens.css` mit `Host: evil.example` | 403 |
| G3 | Standard | `GET /api/gesundheit` mit `Host: evil.example` | 403, Header-Satz mit `no-store` |
| G4 | Standard | `GET /api/gesundheit` ohne `Host`-Header | 403 |
| G5 | Standard | `GET /api/gesundheit` mit `Host: 127.0.0.1.evil.example:<p>` | 403 (kein IP-Literal) |
| G6 | Standard | `GET /api/gesundheit` mit `Host: 127.0.0.1:<p>` | 200 |
| G7 | Standard | `GET /api/gesundheit` mit `Host: localhost:<p>` | 200 |
| G8 | Standard | `GET /api/gesundheit` mit `Host: <rn>:<p>`, `Host: <rn>.local:<p>` und `<rn>` in Großbuchstaben | je 200 |
| G9 | `--host ::` | `GET /api/gesundheit` über `[::1]` mit `Host: [::1]:<p>` | 200 |
| G10 | Standard | `GET /` mit `Host: chat.lan:<p>` | 403 |
| G11 | `--erreichbar-als chat.lan --erreichbar-als nuc` | `GET /` mit `Host: chat.lan:<p>`, dann mit `Host: nuc:<p>` | je 200 |
| G12 | Standard | `POST /api/gesundheit` mit `Origin: http://evil.example` | 403 `{fehler}` |
| G13 | Standard | `POST /api/gesundheit` mit `Origin: null` | 403 |
| G14 | Standard | `POST /api/gesundheit` mit `Origin: http://127.0.0.1:7421` (anderer Port, z. B. kanbanD) | 403 |
| G15 | Standard | `POST /api/gesundheit` ohne `Origin`, mit `Sec-Fetch-Site: cross-site` | 403 |
| G16 | Standard | `POST /api/gesundheit` ohne `Origin`, mit `Sec-Fetch-Site: same-site` | 403 |
| G17 | Standard | `POST /api/gesundheit` mit `Host: 127.0.0.1:<p>`, `Origin: http://127.0.0.1:<p>`, `Sec-Fetch-Site: same-origin` | nicht 403 (405) |
| G18 | Standard | `POST /api/gesundheit` ohne `Origin` und ohne `Sec-Fetch-Site` | nicht 403 (405) |
| G19 | Standard | `OPTIONS /api/gesundheit` mit `Origin: http://evil.example` und `Access-Control-Request-Method: POST` | kein Header `Access-Control-Allow-*` |
| G20 | Standard | `GET /`, `GET /web/gibtsnicht.js`, G1 | 200, 404 und 403 tragen je den Header-Satz; `GET /` zusätzlich ohne `no-store` |

- [ ] G1 bis G20 sind grün.
- [ ] Mutationsprobe: wird ein einzelner Wächter abgeschaltet (Host-Prüfung, Ursprungsprüfung, `Sec-Fetch-Site`-Prüfung, je einer der sechs Header, `Cache-Control: no-store` unter `/api/`), wird mindestens ein Test rot; der Entwickler nennt das Ergebnis je Wächter in seinem Bericht.
- [ ] `GET /` liefert HTML ohne `<style`, ohne ` style=`, ohne ` on…=`-Attribut und ohne `<script>` ohne `src`; der Seitenstil kommt als `text/css` aus einer Datei unter `/web/`.
- [ ] Im Browser (Playwright, Chromium) lädt `http://127.0.0.1:<p>/` ohne einen einzigen Konsolenfehler (ein CSP-Verstoß wäre einer).
- [ ] `localchatA <ordner> --erreichbar-als` (ohne Wert) → Exit 2; `localchatA --hilfe` nennt `--erreichbar-als`.

### F0007 Namensregel in der Kennung

- [ ] Ordner mit `channels/allgemein/2026-10-27.log` mit 17 Sätzen, je eine Textzeile, in dieser Reihenfolge mit den Kennungen: gültig `claude-2@127.0.0.1`, `aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa@10.0.0.5` (32 × `a`), `shoff@fe80::1`; ungültig `Claude@127.0.0.1`, `jörg@127.0.0.1`, `chef`+U+200B+`@127.0.0.1`, U+202E+`abc@127.0.0.1`, U+FEFF+`abc@127.0.0.1`, `a_b@127.0.0.1`, `a.b@127.0.0.1`, `a/b@127.0.0.1`, `a\b@127.0.0.1`, `a+b@127.0.0.1`, `-a@127.0.0.1`, `a-@127.0.0.1`, 33 × `a` + `@127.0.0.1`, `claude@localhost` → Start mit `LOCALCHATA_JETZT=2026-10-27T12:00:00+01:00`: `/api/gesundheit` meldet `"nachrichten":3` und `"defekte_zeilen":14` (Rechnung: 17 Sätze − 3 gültige = 14 Defekte), stderr nennt 14 Zeilen `channels/allgemein/2026-10-27.log:<zeile>`.
- [ ] Die Ordner aus R00002 (Kennungen `shoff@127.0.0.1`, `claude@10.0.0.5`) laden unverändert; die Tests von R00002 bleiben grün.

### F0008 Beitreten mit Namen über die API

Absender `127.0.0.1` und `127.0.0.2` (`curl --interface 127.0.0.2`); jeder `POST` mit `Content-Type: application/json`.

- [ ] Von `127.0.0.2`: `{"name":"claude"}` → 201 mit `"kennung":"claude@127.0.0.2"`, `"name":"claude"`, `"herkunft":"127.0.0.2"`, `"art":"agent"`, `"gnadenfrist_s":60` und einem `token` aus genau 32 Zeichen `[0-9a-f]`; Header-Satz mit `Cache-Control: no-store`.
- [ ] Von `127.0.0.1`: `{"name":"shoff","art":"mensch"}` → 201 mit `"art":"mensch"`.
- [ ] Von `127.0.0.1`: `{"name":"claude"}` → 409 mit `"belegt_von":"claude@127.0.0.2"`, `seit` als RFC-3339-Zeitpunkt und `"vorschlag":"claude-2"`; nach einem Beitritt von `claude-2` lautet der Vorschlag für `claude` `claude-3`.
- [ ] Vorschlag an der Längengrenze: `aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa` (32 × `a`) belegt → `vorschlag` ist 30 × `a` + `-2` (Rechnung: 32 − 2 Zeichen für `-2` = 30 Zeichen Stamm, Länge 32).
- [ ] Namensregel über die Route: `a`, `claude-x` und 32 × `b` → 201; 422 mit Vorschlag: `Jörg` → `joerg`, `Claude` → `claude`, `a_b` → `a-b`, `a.b` → `a-b`, `a b` → `a-b`, `a@b` → `a-b`, `-a` → `a`, `a-` → `a`, `chef`+U+200B → `chef`, U+202E+`abc` → `abc`, 33 × `c` → 32 × `c`; 422 ohne `vorschlag`: `""`, `___`.
- [ ] Reserviert und unbekannte Art: `{"name":"alle"}`, `{"name":"hier"}` und `{"name":"x1","art":"roboter"}` → je 422; danach ist keiner dieser Namen belegt (`x1` → 201).
- [ ] Herkunft kanonisch: Start mit `--host ::`, Beitritt `{"name":"v4"}` über `127.0.0.1` → `"kennung":"v4@127.0.0.1"`; Beitritt `{"name":"v6"}` über `[::1]` → `"kennung":"v6@::1"`.
- [ ] Grenze je Herkunft: 64 Beitritte `n1` … `n64` von `127.0.0.1` → je 201, `n65` von `127.0.0.1` → 429 `{fehler}`, `n65` von `127.0.0.2` → 201.
- [ ] Grenze gesamt: je 64 Beitritte von `127.0.0.1` bis `127.0.0.8` (8 × 64 = 512) → alle 201; ein weiterer von `127.0.0.9` → 429.
- [ ] Gleichzeitig: 20 parallele Beitritte `{"name":"wettlauf"}` → genau eine 201 und 19 × 409.
- [ ] Grenzschutz an der Route: `{"name":"opfer"}` mit `Origin: http://evil.example` → 403, danach `{"name":"opfer"}` ohne Origin → 201 (der Name war nicht belegt); mit `Content-Type: text/plain` → 415 mit Header-Satz, kein Name belegt.
- [ ] Token bleibt geheim: nach drei Beitritten enthalten weder stdout noch stderr noch eine Datei im Ordner eines der drei Tokens.
- [ ] Kein Name über den Neustart: `claude` von `127.0.0.2` beigetreten, Programm beendet und auf demselben Ordner neu gestartet, `{"name":"claude"}` von `127.0.0.1` → 201 `claude@127.0.0.1`.

### F0009 Sitzung wiedererkennen und nach der Gnadenfrist freigeben

- [ ] `GET /api/ich` mit dem Token von `claude@127.0.0.2` von `127.0.0.2` → 200 `{"kennung":"claude@127.0.0.2","name":"claude","herkunft":"127.0.0.2","art":"agent"}`; dasselbe Token von `127.0.0.1` → 401 `{fehler}`; ohne Header → 401; mit `X-LocalchatA-Sitzung: 00000000000000000000000000000000` → 401.
- [ ] Wiederkehr: `POST /api/beitreten {"name":"claude"}` mit dem gültigen Token im Header von `127.0.0.2` → 200 mit demselben `token` und derselben `kennung`; mit `{"name":"claude-neu"}` → 422, danach liefert `GET /api/ich` weiter `claude@127.0.0.2`, und `claude-neu` ist nicht belegt.
- [ ] Unbekanntes Token beim Beitreten: `POST /api/beitreten {"name":"frisch"}` mit `X-LocalchatA-Sitzung: 00000000000000000000000000000000` → 201 `frisch@<herkunft>`.
- [ ] Gnadenfrist (Rechenbeispiel, `LOCALCHATA_GNADENFRIST_MS=3000`): t = 0 s `claude` von `127.0.0.2` beigetreten; t = 2 s `GET /api/ich` → 200 (Lebenszeichen bei 2 s); t = 4 s `claude` von `127.0.0.1` → 409 (2 s seit dem Lebenszeichen ≤ 3 s); ohne weitere Anfrage t = 6,5 s `claude` von `127.0.0.1` → 201 (4,5 s > 3 s), und das alte Token liefert bei `GET /api/ich` 401.
- [ ] Standard ohne Testknopf (Unit, Uhr-Attrappe): Beitritt bei 0 s; bei +59 s ist der Name vergeben; `wiederkehren` bei +59 s erneuert das Lebenszeichen; 61 s danach ist der Name frei und das Token ungültig.
- [ ] `LOCALCHATA_GNADENFRIST_MS=bald localchatA <ordner> --port 0 --kein-browser` → Exit 2, stderr nennt `LOCALCHATA_GNADENFRIST_MS`.

### F0010 Beitreten-Bildschirm

Playwright mit Chromium gegen das echte Programm; Start auf einem freien, festen Port, damit ein Neustart denselben Ursprung behält.

- [ ] Leerer Browser-Kontext öffnet `/` → Beitreten-Bildschirm, Namensfeld hat den Fokus; `shoff` tippen, `Enter` → Kopfzeile zeigt `shoff@127.0.0.1`; `GET /api/ich` mit dem Token aus `localStorage` (`localchatA.sitzung`) liefert `"art":"mensch"`; `localchatA.name` ist `shoff`.
- [ ] Ungültiger Name: `Jörg` tippen → vor dem Senden steht ein Hinweis mit dem Vorschlag `joerg`; `Enter` → kein Beitritt (Bildschirm bleibt, Hinweis mit `joerg`), `Jörg` ist nirgends belegt.
- [ ] Name vergeben: Kontext A ist `shoff`; Kontext B tippt `shoff`, `Enter` → Bildschirm nennt `shoff@127.0.0.1` und die Uhrzeit des Beitritts von A als `HH:MM`, das Feld enthält `shoff-2`; `Enter` → Kopfzeile `shoff-2@127.0.0.1`.
- [ ] Neuladen: Kontext A lädt neu → Kopfzeile `shoff@127.0.0.1` ohne Beitreten-Bildschirm und ohne `POST /api/beitreten`; ein neuer Tab in Kontext A zeigt dieselbe Kennung; danach ist `shoff` weiterhin genau einmal belegt (Beitritt `shoff` per API → 409).
- [ ] Neustart: Programm beenden und auf demselben Ordner und Port neu starten, Kontext A lädt neu → Kopfzeile `shoff@127.0.0.1` ohne Beitreten-Bildschirm (still mit dem gemerkten Namen wieder beigetreten, neues Token im `localStorage`).
- [ ] Gnadenfrist ohne Halter (`LOCALCHATA_GNADENFRIST_MS=2000`): Kontext A beigetreten, 3 s ohne Anfrage, Neuladen → Kopfzeile `shoff@127.0.0.1` ohne Beitreten-Bildschirm; hat in der Zwischenzeit Kontext B `shoff` genommen, zeigt A beim Neuladen den Zustand „Name vergeben“ mit Vorschlag `shoff-2`.
- [ ] Server weg: Programm beenden, in einem leeren Kontext (Seite vorher geladen) `shoff` tippen, `Enter` → ⚠ „Server nicht erreichbar“ mit Schalter „Erneut versuchen“; Programm auf demselben Port neu starten, „Erneut versuchen“ → Kopfzeile `shoff@127.0.0.1`.
- [ ] Ohne Maus: alle Szenarien dieses Abschnitts gehen mit Tastatur (`Tab`, `Enter`); der fokussierte Schalter oder das Feld ist sichtbar umrandet.
- [ ] In keinem der Szenarien dieses Abschnitts erscheint ein Konsolenfehler.

## Betroffene Verzeichnisstruktur

- Grenzschutz: neu `src/http/grenzschutz.rs` (Schicht, Host- und Ursprungsprüfung, Header); geändert `src/http/mod.rs` (Router unter der Schicht, Testserver über `ausliefern`), `src/start/aufruf.rs` (`--erreichbar-als`), `src/start/mod.rs` (Rechnername, Anwesenheit, Gnadenfrist), `Cargo.toml` (`rustix` Feature `system`).
- Namensregel: `src/gespraeche/kennung.rs` (Kennung nach Regel); der Typ `Name` dort oder in einem eigenen Modul (Entscheidung des Entwicklers).
- Anwesenheit: neu `src/anwesenheit/` (A00010: Sitzungen, Namen belegen, Lebensdauer, Wiederkehr, Vorschlag), `src/zufall.rs` (Provider), `src/http/beitreten.rs`, `src/http/ich.rs`, Extraktor `Teilnehmer` unter `src/http/`; geändert `src/start/ausliefern.rs` (Herkunft je Verbindung).
- Oberfläche: `web/index.html`, neu `web/lc-seite.css`, `web/start.js`, `web/api.js`, `web/sitzung.js`, `web/namensregel.js`, `web/beitreten.js`, `web/kopfzeile.js`; `src/http/eingebettet.rs` (neue Dateien mit Content-Type).
- Tests: Unit im jeweiligen Modul; Integration `tests/grenzschutz.rs`, `tests/beitreten.rs`, `tests/sitzung.rs` mit `tests/gemeinsam/mod.rs` (zweite Absenderadresse, `LOCALCHATA_GNADENFRIST_MS`); Oberfläche rein `e2e/einheit/*.test.js` (`node --test`), Abläufe `e2e/tests/r00003-beitreten.spec.js`.

## Technische Überlegungen

### Ablauf

1. **Start** (A00003): Aufruf lesen (mit `--erreichbar-als`) → Uhr → Gnadenfrist aus `LOCALCHATA_GNADENFRIST_MS` (unlesbar → Exit 2) → Ordner → Sperre → Laden → Rechnername → Anwesenheit (Uhr, Zufall, Gnadenfrist) → binden → Signale → Startzeile → ausliefern.
2. **Jede Anfrage** (A00004): Annahmeschleife setzt die kanonische Gegenstelle als `ConnectInfo` → `grenzschutz`: Host prüfen (403) → Ursprung prüfen bei nicht lesender Methode (403) → Route → Sicherheitsheader auf die Antwort.
3. **Beitreten** (A00010): JSON lesen (415/422) → Beitrittswunsch prüfen (Namensregel, reserviert, `art`; 422) → Header mit gültigem Token derselben Herkunft? → Wiederkehr (200 | 422 anderer Name) → sonst unter der Sperre: abgelaufene Sitzungen gelten als weg → Name lebend belegt? (409 mit Vorschlag) → Grenzen (429) → Sitzung mit Token anlegen (201).
4. **Ich**: Extraktor `Teilnehmer` (Header, Herkunft → `wiederkehren`, Lebenszeichen | 401) → 200.
5. **Oberfläche beim Laden**: Token? → `GET /api/ich` (200 → Kopfzeile | 401 → `beitreten` mit gemerktem Namen) | kein Token, Name gemerkt → `beitreten` | nichts → Bildschirm; Netzfehler → „Server nicht erreichbar“.

### Grobentwurf (Vorplanung)

Die Bubbles aus der WBS als Datenfluss — **Vorplanung**: welche es am Ende werden, entscheidet der Entwickler beim Bauen.

- F0006: B0034 Aufruf um `--erreichbar-als` erweitern (Argumente → `erreichbar_als` | Aufruffehler) · B0035 Rechnername lesen (→ `Option<Rechnername>`) · B0036 Host prüfen (`Host`, freigegebene Namen → erlaubt | abgewiesen) · B0037 Ursprung prüfen (Methode, `Origin`, `Sec-Fetch-Site`, `Host` → erlaubt | abgewiesen) · B0038 Sicherheitsheader setzen (Antwort, Pfad → Antwort mit Header-Satz) · B0039 Grenzschutz verdrahten (Router → äußerste Schicht; Angriffsmatrix, Mutationsprobe) · B0040 Seitenstil in eine Datei auslagern
- F0007: B0041 Name prüfen und Umformung vorschlagen (Text → `Name` | `Namensfehler {regel, vorschlag?}`) · B0042 Kennung nach der Namensregel bilden (Text → `Kennung` | ungültig; Test `ä@fe80::1` umstellen)
- F0008: B0043 Herkunft je Verbindung setzen · B0044 Testserver über `ausliefern` · B0045 Zufall für Tokens · B0046 Beitrittswunsch prüfen (JSON → `Beitrittswunsch` | 422) · B0047 Freien Namen vorschlagen · B0048 Namen belegen (→ `Sitzung` | `NameVergeben` | `ZuVieleSitzungen`) · B0049 Beitreten beantworten (Route)
- F0009: B0050 Gnadenfrist aus der Umgebung · B0051 Lebensdauer beim Zugriff · B0052 Wiederkehren · B0053 Teilnehmer an der Grenze erkennen · B0054 Ich beantworten · B0055 Wiederkehr beim Beitreten
- F0010: B0056 Namensregel als Hinweis · B0057 API aufrufen · B0058 Zustand beim Laden wählen · B0059 Beitreten-Bildschirm zeichnen · B0060 Eigene Kennung in der Kopfzeile · B0061 Oberfläche verdrahten (E2E)

**Elemente** (nur öffentliche Signaturen, Rust und JavaScript, nach A00004, A00005, A00010):

- `Grenzschutz { rechnername: Option<String>, erreichbar_als: Vec<String> }` (unveränderlich) mit `fn host_erlaubt(&self, host: Option<&str>) -> Result<(), HostAbgewiesen>` und `fn ursprung_erlaubt(methode, origin, sec_fetch_site, host) -> Result<(), UrsprungAbgewiesen>` (rein); Schicht `grenzschutz` als `axum::middleware`.
- `Name` (geprüfter Text) mit `fn bilden(&str) -> Result<Name, Namensfehler>`; `Namensfehler { regel, vorschlag: Option<Name> }`.
- `Kennung::bilden(&str)` nutzt `Name` und die Herkunftsregel.
- `Zufall` (Trait) mit `fn token(&mut self) -> Token`; Produktion `getrandom`, Test feste Folge.
- `Anwesenheit` (einziger Schreiber der Sitzungen, ein `Mutex`) mit `fn beitreten(&self, wunsch: Beitrittswunsch, herkunft: IpAddr) -> Result<Sitzung, Beitrittsfehler>` und `fn wiederkehren(&self, token: &Token, herkunft: IpAddr) -> Option<Sitzung>`; `Beitrittsfehler::{NameVergeben { belegt_von, seit, vorschlag }, ZuVieleSitzungen}`.
- `Teilnehmer` (Extraktor) `{ kennung, name, herkunft, art }` oder 401.
- Oberfläche: `namensregel.pruefen(text) -> {gueltig, hinweis?, vorschlag?}`; `sitzung.waehlen(speicher, antworten) -> Zustand`; `api.rufen(pfad, body, token) -> {status, daten} | ServerWeg`.

Wireframe: die WBS führt `wireframes: Dokumentation/Wireframes/`, ein Artboard `D0002.dc.html` fehlt (Hinweis `/wireframe verfeinern D0002`). Gestaltungsquelle bis dahin: `design-prompt.md` Bildschirm 4.1 und `Dokumentation/Wireframes/designsystem/` (README, `1-tastatur.md`, Bausteine `components/Kennung`, `components/Kopfzeile`, `components/ScreenFlow` — Zustände „Name vergeben“ und „Server weg“) — als Vorgabe für die Gestaltung, nicht als Akzeptanzkriterium.

### Änderungen an bestehenden Klassen

- `start::aufruf`: `Startauftrag` erhält `erreichbar_als`; Hilfe-Text.
- `start::ausliefern`: Gegenstelle bei `accept` kanonisch, `ConnectInfo` je Request, `Belegung::zulassen` mit derselben Adresse.
- `http::testserver`: über `ausliefern` statt `axum::serve`.
- `http::Zustand`: trägt Anwesenheit und Grenzschutz-Einstellung.
- `gespraeche::kennung::Kennung::bilden`: prüft Namens- und Herkunftsregel; der Test mit `ä@fe80::1` wird umgestellt.
- `web/index.html`: Seitenstil nach `web/lc-seite.css`, genau ein Modul-Skript `web/start.js`.

## Tests

Nach Skill `test-pyramide` und ADR-0009; die Szenarien der User Story werden Tests, die Angriffsmatrix G1–G20 je Zeile ein Integrationstest.

**Unit (rein):** Host- und Ursprungsprüfung (Matrix aus G1–G19 ohne Netz); Header-Satz; `Name` mit der Testmatrix aus ADR-0013 und den Vorschlägen; `Kennung` (gültig/ungültig aus F0007); Vorschlag freier Name (`claude-2`, `-3`, Längengrenze); Anwesenheit mit Uhr-Attrappe und fester Zufallsfolge (erster Test aus A00010: vergeben, +59 s, Wiederkehr, +61 s, andere Herkunft, Grenzen 64/512, gleichzeitige Beitritte); Aufruf `--erreichbar-als`; Gnadenfrist aus der Umgebung.

**Integration (`tests/`, echtes Programm):** Angriffsmatrix G1–G20; Log mit ungültigen Kennungen; Beitreten von `127.0.0.1`/`127.0.0.2`, 409/422/429/415, `--host ::`; `GET /api/ich`, Wiederkehr, Gnadenfrist mit `LOCALCHATA_GNADENFRIST_MS`; Token nicht auf stdout/stderr/Platte; Neustart. Mutationsprobe je Wächter.

**Oberfläche rein (`e2e/einheit/`, `node --test`):** `namensregel.js` (`Jörg` → `joerg`, `claude-2` gültig), `sitzung.js` (Zustandswahl mit `localStorage`-Ersatz für 200/401/201/409/Netzfehler).

**E2E (Playwright):** die Szenarien aus F0010 und F0006 (keine Konsolenfehler); mindestens ein Test mit verzögert antwortendem Server für „Beitreten“ (`gemeinsam/gelerntes.md`, UI-Tests mit verzögertem Fake).

## Abhängigkeiten

- Abhängig von: I0001 Instanz starten (R00001, grün) — Annahmeschleife, Aufruf, Oberfläche ausliefern, E2E-Gerüst; I0002 Nachricht landet im Log (R00002, grün) — `Kennung`, Laden mit Defekten, Uhr.
- Innerhalb (WBS `Braucht`): F0006 und F0007 frei und parallel; F0008 braucht F0006 (keine Route mit Wirkung ohne Grenzschutz, A00004) und F0007 (Typ `Name`); F0009 braucht F0008; F0010 braucht F0006 und F0009.
- Blockiert: I0005 Teilnehmer mit Online-Status sehen, I0007 Channel anlegen, I0014 Als Agent beitreten (je `Braucht` I0004) und damit alles Schreibende danach.

## Offene Fragen

Keine, die R00003 blockieren. Entschieden mit den Empfehlungen des architekt (Karte #3):

- Feld `art` beim Beitreten, Standard `agent`, die Oberfläche sendet `mensch` (Designsystem Frage 3) — Selbstauskunft, nur für die Darstellung.
- Name vergeben → 409 mit Vorschlag `<name>-2`, … ; die Oberfläche füllt ihn vor (Designsystem Frage 8).
- Länge des Namens 1 bis 32 Zeichen (ADR-0013).
- Reservierte Namen: `alle` und `hier`, sonst keine.
- Keine Reservierung eines Namens über einen Neustart; wer zuerst wieder beitritt, bekommt ihn.
- „Gelesen“ gilt je Browser, nicht je Name (Designsystem Frage 7) — gebaut mit I0008/I0013, in R00003 ohne Wirkung.
- Direktgespräche hängen an der vollen Kennung, nicht am Namen allein — gebaut mit I0012, in R00003 ohne Wirkung.

Weiter offen, ohne R00003 zu blockieren:

- **Betreiber**: Die Vision nennt als Herkunft auch den Rechnernamen (`claude@localhost`, `claude-entwickler@server.de`); R00003 baut nur IP-Adressen. Empfehlung: Vision-Beispiele bei der nächsten Fortschreibung auf IP-Adressen angleichen und Rechnernamen als eigene, spätere Interaction führen.
- **architekt**: Bildschirm 4.1 zeigt die Herkunft schon vor dem Beitreten; ADR-0012 bietet dafür keine Route (`/api/ich` verlangt die Sitzung). R00003 zeigt die Kennung erst nach dem Beitritt. Empfehlung: bei Bedarf eine wirkungsfreie Route für die eigene Herkunft per ADR-Ergänzung, als Ausbaustufe von F0010.

## Manuelle Vorbereitungstätigkeiten

- Keine über `AI-Team/umgebung.md` hinaus. Die Tests mit zweiter Absenderadresse brauchen Linux-Loopback (`127.0.0.0/8`), G9 und der IPv6-Fall brauchen `::1`.

## Manuelle Nachbereitungstätigkeiten

- Gate in `AI-Team/umgebung.md` um `node --test e2e/einheit` ergänzen, sobald es den Ordner gibt (Director).
- Erwartung für die Abnahme, bis I0005 Halter bringt: ein Browser, der 60 s nichts tut, verliert seinen Namen am Server und tritt beim nächsten Neuladen still wieder bei; die Kopfzeile zeigt bis zum Neuladen weiter den alten Namen.

## Umfang

Aus der WBS gezählt (`/planung zaehlen I0004`); belegte Werte aus `Schaetzungen/_ist-zeiten.md` (R00001 und R00002, Typ Standard, `bestaetigt` 0,01–0,05h) auf die Untergrenze 0,4h gehoben:

- F0006 Grenzschutz an der Systemgrenze = 7 Bubbles: 7 Standard (0,4h). Rest: 2,8h klar · 7 von 7 Werten belegt.
- F0007 Namensregel in der Kennung = 2 Bubbles: 2 Standard (0,4h). Rest: 0,8h klar · 2 von 2 Werten belegt.
- F0008 Beitreten mit Namen über die API = 7 Bubbles: 7 Standard (0,4h). Rest: 2,8h klar · 7 von 7 Werten belegt.
- F0009 Sitzung wiedererkennen und nach der Gnadenfrist freigeben = 6 Bubbles: 6 Standard (0,4h). Rest: 2,4h klar · 6 von 6 Werten belegt.
- F0010 Beitreten-Bildschirm = 6 Bubbles: 4 Standard (0,4h), 2 unklar (2–4h). Rest: 1,6h klar + 4–8h unklar · 4 von 6 Werten belegt, Rest Richtwerte (ungemessen).
- I0004 gesamt = 28 Bubbles. Rest: 10,4h klar + 4–8h unklar · 26 von 28 Werten belegt, Rest Richtwerte (ungemessen).

## Warum löst diese Anforderung das Problem? (Pflicht)

Auslöser: Nach I0002 kann die Instanz Nachrichten ins Log schreiben, aber niemand kann sie senden, weil es keinen Absender gibt — und die erste Route mit Wirkung würde ohne Schutz jede Website im Browser eines Teilnehmers zum Mitspieler machen (Security-Review #1: DNS-Rebinding, CSRF). Wenn zuerst der Grenzschutz vor allen Routen steht, dann entsteht `beitreten` gar nicht erst als Angriffsfläche; wenn die Namensregel Verwechslungen ausschließt und die Herkunft aus dem Socket kommt, dann ist `name@herkunft` im Log, in der Liste und später in Ordnernamen ein eindeutiges, nicht fälschbares Wort; und wenn der Browser die Sitzung im `localStorage` hält und still wieder beitritt, dann merkt der Mensch vom Sitzungsmodell nichts außer seiner Kennung in der Kopfzeile. Der Hebel liegt hier und nicht bei I0009 oder I0014, weil Name und Herkunft in jedes Log, jede Erwähnung und jeden Ordnernamen eingehen und dort schwer umkehrbar sind (ADR-0013), und weil Browser und Agent dieselbe Route und dasselbe Token-Modell nutzen — einmal richtig gebaut, erben I0005, I0012 und I0014 es.

## Missing-Docs

- Glossar fehlt (`Dokumentation/Domaenenwissen/` existiert nicht); Begriffe wie Teilnehmer, Kennung, Herkunft, Sitzung, Gnadenfrist, Lebenszeichen, Halter sind aus ADR-0012/0013 und A00010 übernommen.
- Verhalten von `Sec-Fetch-Site` und `Origin` in anderen Browsern als Chromium ist im Projekt nicht erprobt (ADR-0014 prüfte nur Chromium).

## Notizen

### Verworfene Alternativen

- **Grenzschutz als eigene Interaction vor I0004** (ADR-0014 Option B, Karte #18): verworfen — eine Interaction ohne Nutzerverhalten ist schlecht abnehmbar; als Slice F0006 bleibt er von außen prüfbar (Angriffsmatrix) und parallel zur Namensregel baubar.
- **Herkunft je Request als eigener Slice** (Zuschnitt-Vorschlag des architekt): verworfen — für sich nicht von außen beobachtbar; als Bubble B0043 in F0008 wird er über die Kennung `claude@127.0.0.2` geprüft.
- **Beitreten und Sitzung in einem Slice**: verworfen — 13 Bubbles; nach dem Muster Regeln ist das Belegen ohne Ablauf (F0008) für sich prüfbar, Wiederkehr und Gnadenfrist (F0009) kommen als eigener Slice.
- **Je Slice eine eigene Anforderung**: verworfen — alle fünf haben ein Ziel (Namen wählen und beitreten), durchlaufen als Arbeitspaket I0004 gemeinsam Entwicklertest und Abnahme, und F0008 ist ohne F0006 nicht auslieferbar.

### Bewusst außerhalb

Halter und Teilnehmerliste mit online/offline (I0005); Takt, der „weg“ meldet, `POST /api/verlassen`, stiller Abriss (I0006); längere Gnadenfrist für Agenten (`gnadenfrist_s` bis 3600) und Anleitung unter `/api` (I0014); Rechnernamen als Herkunft; Rate-Begrenzung je Herkunft und TLS (ADR-0014); Hauptansicht über die Kopfzeile hinaus (I0007 ff.).
