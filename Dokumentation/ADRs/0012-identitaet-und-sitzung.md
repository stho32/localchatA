# ADR-0012: Identität und Sitzung — Herkunft aus der Gegenstelle, Sitzungs-Token im Speicher, eine Sitzung je Browser

**Status**: Akzeptiert
**Datum**: 2026-10-27
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #3 I0004)
**Kontext-Anforderung**: — (WBS I0004; prägt I0005, I0006, I0012, I0014, I0016)
**Erprobt durch**: Wegwerf-Probe im Scratchpad (axum 0.8.9, hyper 1.12, getrandom 0.4.3 aus dem Lockfile des Projekts): eigene Annahmeschleife setzt je Verbindung `ConnectInfo<SocketAddr>` mit `IpAddr::to_canonical()`; Listener auf `[::]`, Anfrage über `127.0.0.1` → Handler sieht `127.0.0.1` (nicht `::ffff:127.0.0.1`), über `[::1]` → `::1`; Listener auf `0.0.0.0`, Absender `127.0.0.2` → `127.0.0.2`; `getrandom::fill` liefert 16 Byte. Probe danach gelöscht.

Löst [ADR-0008](0008-identitaet-peer-adresse-und-sitzungs-token.md) ab. Was dort richtig war (Herkunft aus der Peer-Adresse, Token als Träger der Sitzung, Header `X-LocalchatA-Sitzung`), bleibt; geändert sind die Ablage des Tokens im Browser (eine Sitzung je Browser statt je Tab), die Lebensdauer (Lebenszeichen statt nur Halter), Grenzen gegen Namensbesetzung und die Stelle, an der die Herkunft entsteht. Die Namensregel steht in [ADR-0013](0013-namensregel-als-erlaubnisliste.md), der Schutz der Systemgrenze in [ADR-0014](0014-grenzschutz-host-origin-sicherheitsheader.md).

## Kontext

- Vision, Zielbild „Mensch und KI sind gleichberechtigt“: kein Login; wer schreiben will, gibt sich einen Namen, die Herkunft „(IP-Adresse oder Rechnername)“ ergänzt ihn zu `name@herkunft`; ein Name vor dem `@` ist nur einmal vergeben, egal von wo.
- Vision, „Wer da ist, ist sichtbar“: Abbruch → sofort offline; ohne Rückkehr nach etwa einer Minute aus der Liste, Name wieder frei (WBS I0006).
- Vision, „Ausdrücklich nicht Ziel“: keine Benutzerverwaltung, keine Rechte, kein öffentlicher Dienst; Einsatzraum LAN.
- Code: das Programm liefert über eine eigene Annahmeschleife aus ([ADR-0010](0010-eigene-annahmeschleife-hyper-http1.md)); `ConnectInfo` setzt axum dort nicht (`src/start/ausliefern.rs`, `bedienen`). Der In-Process-Testserver (`src/http/mod.rs`, `testserver`) fährt `axum::serve` ohne Connect-Info — ein Handler mit `ConnectInfo`-Extraktor scheitert dort.
- Designsystem `3-offene-fragen.md`: Frage 3 (Agent erkennen), 7 („gelesen“ je Tab oder je Name), 8 (Name zurückgewinnen); ScreenFlow: „Name gemerkt → direkt 4.2 Hauptansicht“, Zustände „Name vergeben — Hinweis + freier Vorschlag“ und „Server weg — Erneut versuchen“.
- Verbindungsgrenze 64 je Gegenstelle, 512 gesamt ([ADR-0010](0010-eigene-annahmeschleife-hyper-http1.md)); Live-Kanal mit Haltern ([ADR-0007](0007-live-kanal-sse-fuer-browser-long-poll-fuer-agenten.md), Vorgeschlagen).

## Optionen

Erwogen wurden der Träger der Sitzung und — davon abhängig — wo der Browser ihn hält. Die Herkunft aus der Gegenstelle war in allen Varianten gesetzt (Rückwärtsauflösung als Rechnername bleibt eine spätere Erweiterung, Begründung in ADR-0008 Option B).

### Option A: Token im Speicher des Servers, im Browser je Tab (`sessionStorage`) — Stand von ADR-0008
- Vorteile: zwei Tabs eines Browsers können zwei verschiedene Teilnehmer sein; ein geschlossener Tab nimmt seine Sitzung mit.
- Nachteile: ein neuer Tab desselben Menschen kennt das Token nicht; mit dem gemerkten Namen (ScreenFlow „Name gemerkt → direkt“) trifft er auf seinen eigenen, noch belegten Namen und wird mit 409 abgewiesen — der Mensch kann nicht zwei Tabs als er selbst offen haben. „Tab duplizieren“ kopiert `sessionStorage` und erzeugt doch geteilte Sitzungen, also kein klares Modell.

### Option B: Token im Speicher des Servers, im Browser je Ursprung (`localStorage`); alle Tabs eines Browsers teilen eine Sitzung
- Vorteile: ein Mensch = ein Browser = eine Sitzung; neue Tabs und Neuladen sind sofort derselbe Teilnehmer; „gelesen“ ist je Browser eindeutig (Frage 7); Agenten halten ihr Token selbst — dasselbe Modell, nur ohne Browser.
- Nachteile: zwei Menschen am selben Browserprofil sind ein Teilnehmer; jedes Skript des Ursprungs liest das Token (wie bei A — der Schutz dagegen ist die CSP aus ADR-0014, nicht der Speicherort).

### Option C: Sitzungs-Cookie (`HttpOnly`, `SameSite=Strict`) für den Browser, Header nur für Agenten
- Vorteile: Skripte kommen nicht an das Token (`HttpOnly`); `EventSource` schickt Cookies von selbst, kein Token in der URL.
- Nachteile: Cookies trennen nicht nach Port — jedes andere Programm auf `127.0.0.1` (etwa kanbanD auf 7421) kann Cookies für diesen Host setzen oder überschreiben (Cookie-Tossing); jeder Browser-Aufruf trägt die Sitzung automatisch, also hängt jeder Schreibweg an der Origin-Prüfung (CSRF); zwei Nachweiswege an der Grenze.

### Option D: Sitzung auf Platte (`<ordner>/.localchatA.sitzungen`), übersteht den Neustart
- Vorteile: nach einem Neustart behält jeder seinen Namen; kein Wettlauf um Namen.
- Nachteile: der Ordner ist zum Lesen für Dritte gedacht (Vision „lesbar ohne localchatA“, ADR-0005) — Tokens lägen offen; ein zweiter Datenbestand mit eigener Schreibregel; die Gnadenfrist müsste über den Neustart hinweg gerechnet werden.

## Entscheidung

**Gewählt: Option B**

Es überwiegt, dass „ein Mensch, ein Name“ im Browser ohne Sonderfall gilt und Frage 7 eine eindeutige Antwort bekommt; die Herkunft bleibt unfälschbar im Sinne der Vision, weil sie aus der Gegenstelle des Sockets kommt und nie vom Client. In Kauf genommen: zwei Menschen an einem Browserprofil teilen sich einen Namen, und nach einem Neustart des Programms gewinnt den Namen, wer zuerst wieder beitritt.

### Herkunft

- Die Annahmeschleife (`src/start/ausliefern.rs`) normalisiert die Gegenstelle **einmal** bei `accept` mit `IpAddr::to_canonical()` (IPv4-gemappte IPv6 → IPv4) und setzt je Verbindung `ConnectInfo<SocketAddr>` als Request-Extension. Dieselbe kanonische Adresse zählt in `Belegung::zulassen` — Platzzählung und Herkunft sehen dieselbe Gegenstelle.
- Herkunft im Text = `IpAddr`-Darstellung (`127.0.0.1`, `10.0.0.5`, `::1`, `fe80::1`); kein `localhost`, keine Rückwärtsauflösung.
- Der In-Process-Testserver läuft ab I0004 über `ausliefern` statt `axum::serve` (Folgeaufgabe aus ADR-0010 wird fällig, weil `beitreten` die Herkunft braucht).

### Sitzung

- `POST /api/beitreten` mit JSON `{name, art?}` → 201 `{token, kennung, name, herkunft, art, gnadenfrist_s}`. `art` ist `mensch` | `agent`; fehlt sie, gilt `agent` (die Oberfläche sendet `mensch`) — Selbstauskunft, ungeprüft, nur für die Darstellung (Frage 3).
- Trägt `beitreten` einen Header `X-LocalchatA-Sitzung` mit gültigem Token derselben Herkunft, ist es eine **Wiederkehr**: 200 mit derselben Sitzung — so tritt ein Agent nach seinem eigenen Neustart wieder bei, ohne an seinem noch belegten Namen zu scheitern. Ein anderer Name im Body → 422 (einen Namenswechsel gibt es nicht, auch nicht still). Unbekanntes Token (z. B. nach Neustart) → behandelt wie ein neuer Beitritt mit dem Namen aus dem Body.
- Name vergeben → 409 `{fehler, belegt_von, seit, vorschlag}`; `vorschlag` ist der erste freie Name `<name>-2`, `-3`, … innerhalb der Längengrenze (Frage 8). Name verletzt die Regel → 422 nach [ADR-0013](0013-namensregel-als-erlaubnisliste.md).
- Token: 128 Bit aus dem Zufall des Betriebssystems (`getrandom`, bereits im Lockfile), hexadezimal 32 Zeichen; erzeugt über einen Provider `Zufall`, damit Unit-Tests feste Tokens setzen. Nur im Speicher; nie im Log, nie in der Startzeile, nie in einer Kennung. Antworten mit Token tragen `Cache-Control: no-store`.
- Nachweis bei jedem Aufruf mit Sitzung: Header `X-LocalchatA-Sitzung`; gültig nur, wenn die Herkunft der Anfrage der Herkunft der Sitzung gleicht (ein Laptop mit neuer Adresse tritt neu bei). Fehlt oder passt er nicht → 401 `{fehler}`. Ein Extraktor `Teilnehmer` an der Grenze liefert Handlern die geprüfte Kennung (Aspekt [X00003](../Architektur/Aspekte/X00003-authentifizierung-und-sitzung.md)). Für `EventSource` (ab I0005) entscheidet die dortige Anforderung zwischen Query-Parameter (mit `Referrer-Policy: no-referrer`, ADR-0014) und `fetch`-Strom mit Header.
- `GET /api/ich` → 200 `{kennung, name, herkunft, art}` | 401 — die erste Route, die die Sitzung verlangt; die Oberfläche prüft damit beim Laden ihr gemerktes Token.

### Lebensdauer

- Eine Sitzung lebt, solange sie **Halter** hat (offener SSE-Strom oder wartender Aufruf, ab I0005/I0016 nach ADR-0007) oder ihr letztes **Lebenszeichen** nicht länger als die Gnadenfrist zurückliegt. Lebenszeichen = jede Anfrage mit gültigem Token und das Ende des letzten Halters.
- Danach ist sie **weg**: Token ungültig, Name frei. Geprüft wird beim Zugriff (Beitreten, Token-Prüfung) gegen den Uhr-Provider ([X00007](../Architektur/Aspekte/X00007-zeit.md)); ein Takt, der „weg“ als Ereignis meldet, kommt mit I0006.
- Gnadenfrist Standard 60 s (Testknopf `LOCALCHATA_GNADENFRIST_MS`, ADR-0009). Ein Agent darf beim Beitreten eine längere verlangen (`gnadenfrist_s`, höchstens 3600) — Parameter von I0014, nicht von I0004.
- Grenzen gegen Besetzen: höchstens 64 lebende Sitzungen je Herkunft, 512 gesamt (gleich den Verbindungsgrenzen); darüber 429 `{fehler}`.

### Zustand

- Alle Sitzungen leben nur im Speicher des Bausteins [Anwesenheit](../Architektur/A00010-anwesenheit.md) — er ist der **einzige Schreiber** dieses Bestands (Datenhoheit). Ein Neustart beendet alle Sitzungen; die Oberfläche tritt mit dem gemerkten Namen neu bei.
- Sitzungen sind von Verbindungen unabhängig: eine Sitzung kann über viele Verbindungen sprechen, eine Verbindung über keine oder mehrere Sitzungen. Die 64 Plätze je Gegenstelle begrenzen Verbindungen, die 64 Sitzungen je Herkunft Namen.

### Browser

- `localStorage` des Ursprungs: `localchatA.sitzung` (Token) und `localchatA.name` (zuletzt gewählter Name). Alle Tabs teilen die Sitzung.
- Beim Laden: Token vorhanden → `GET /api/ich`; 200 → Hauptansicht, 401 → `beitreten` mit gemerktem Namen; 201 → Hauptansicht, 409 → Beitreten-Bildschirm im Zustand „Name vergeben“ mit Vorschlag. Kein Token, kein Name → Beitreten-Bildschirm.
- „Gelesen“ (Frage 7) lebt, wenn es kommt, ebenfalls im `localStorage` je Kennung; der Server führt keinen Lesestand.

## Konsequenzen

**Positiv:**
- Jede Anfrage trifft an der Systemgrenze zuerst Grenzschutz ([ADR-0014](0014-grenzschutz-host-origin-sicherheitsheader.md)) und Sitzungsprüfung; Fachbausteine bekommen nur eine geprüfte Kennung.
- Agenten und Browser folgen demselben Modell (Token selbst halten, Header senden); Agenten, die nur senden und nie warten, verlieren ihren Namen nicht, solange sie innerhalb der Gnadenfrist etwas tun.
- Die Herkunft ist an einer Stelle definiert und mit der Verbindungszählung deckungsgleich.

**Negativ / Inkauf zu nehmen:**
- Nach einem Neustart kann ein anderer einen Namen übernehmen, bevor sein bisheriger Träger zurückkehrt.
- Wer Token und Adresse eines anderen kennt, kann für ihn schreiben — ohne Rechte hingenommen (Vision).
- Bis I0005 gibt es keine Halter: ein Browser, der nur den Beitreten-Bildschirm und die Kopfzeile hat, verliert seinen Namen nach 60 s ohne Anfrage und tritt beim nächsten Laden still wieder bei.
- `sessionStorage` (ADR-0008) wird nicht genutzt; zwei Personen an einem Browserprofil brauchen zwei Profile.

**Pre-Mortem (ein Monat nach dem Start gescheitert, weil …):**
1. … Agenten mitten in langer Arbeit ihren Namen verlieren (über 60 s ohne Anfrage, noch kein Warte-Aufruf). Früh-Indikator: 401 mit anschließendem Wiederbeitritt unter `<name>-2`. Gegenmaßnahme: Wiederkehr per Token in `beitreten`, `gnadenfrist_s` bis 3600 mit I0014.
2. … ein Skript im LAN alle naheliegenden Namen besetzt. Früh-Indikator: viele Sitzungen einer Herkunft. Gegenmaßnahme: 64 Sitzungen je Herkunft, 512 gesamt (429); Kennung zeigt die Herkunft.
3. … nach einem Neustart Namen bei Fremden landen und Direktnachrichten falsch zugeordnet werden. Früh-Indikator: 409 beim stillen Wiederbeitritt. Gegenmaßnahme: Vorschlag statt stillem Ausweichen; Direktgespräche hängen an der vollen Kennung (Empfehlung A00006, I0012).

**Folgeentscheidungen / Aufgaben:**
- [ ] Annahmeschleife setzt `ConnectInfo` kanonisch und zählt Plätze mit derselben Adresse; erster Test über das echte Programm: `beitreten` von `127.0.0.2` → Kennung `<name>@127.0.0.2`; Listener auf `[::]` (`--host ::`), Anfrage über `127.0.0.1` → Herkunft `127.0.0.1`.
- [ ] Testserver über `ausliefern` (ein Server-Stack, ADR-0010).
- [ ] Baustein Anwesenheit (A00010) mit Provider `Zufall` und Uhr; erster Unit-Test siehe A00010.
- [ ] I0014: `gnadenfrist_s` für Agenten; I0006: explizites `POST /api/verlassen` (Empfehlung) und Takt für „weg“.

## Quellen / Bezuege

- [A00010 Anwesenheit](../Architektur/A00010-anwesenheit.md) (primär), [A00004 HTTP-Schnittstelle](../Architektur/A00004-http-schnittstelle.md), [A00003 Start](../Architektur/A00003-start.md), [A00005 Weboberfläche](../Architektur/A00005-weboberflaeche.md); Aspekte [X00003](../Architektur/Aspekte/X00003-authentifizierung-und-sitzung.md), [X00002](../Architektur/Aspekte/X00002-benutzerrechte.md)
- ADR-0003 (Ordnernamen der Direktgespräche), ADR-0007, ADR-0008 (abgelöst), ADR-0009, ADR-0010, ADR-0013, ADR-0014
- Vision R00000, Zielbild „Mensch und KI sind gleichberechtigt“, „Wer da ist, ist sichtbar“; Designsystem `3-offene-fragen.md` Fragen 3, 7, 8, `components/ScreenFlow`; `AI-Team/gelerntes.md` (#1, Annahmeschleife)

## Ergänzungen

- **2026-10-28 (Karte #5 I0006):** Folgeaufgaben „explizites `POST /api/verlassen`“ und „Takt für weg“ sind entschieden bzw. gebaut: `verlassen` in [ADR-0018](0018-verlassen-sofort-weg-nie-beim-schliessen.md) (sofort `weg`, idempotent 204, die Oberfläche ruft es nie beim Schließen); `weg` meldet der Takt seit I0005 ([ADR-0016](0016-anwesenheitsstatus-aus-haltern-mit-nachlauf-liste-als-stand-plus-wechsel.md)). Das Ende eines Halters durch einen stillen Abriss ([ADR-0017](0017-stiller-abriss-kernel-erkennt-tote-verbindungen.md)) ist ein Lebenszeichen zum Zeitpunkt des Erkennens; die Gnadenfrist läuft ab dort.
