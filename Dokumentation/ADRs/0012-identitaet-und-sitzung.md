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
- **2026-10-28 (Karte #5 I0006, Security-Review #5):** Die Grenzen „64 je Herkunft, 512 gesamt“ zählen außer den lebenden Sitzungen auch die durch `verlassen` entfernten, bis deren Gnadenfrist ab dem `verlassen` abgelaufen ist; der Name ist trotzdem sofort frei. Wortlaut und Konsequenzen: [ADR-0018](0018-verlassen-sofort-weg-nie-beim-schliessen.md), Ergänzungen, Regel 2a.
- **2026-10-28 (R00015, F0056; Lastprofil „Team-LAN“) — 128 Sitzungen je Herkunft, nicht mehr gleich der Verbindungsgrenze:** Die Regel „höchstens 64 lebende Sitzungen je Herkunft, 512 gesamt (gleich den Verbindungsgrenzen)“ unter „Lebensdauer“ gilt ab R00015 mit **128 je Herkunft**, gesamt unverändert 512; mitgezählt werden weiter die durch `verlassen` entfernten bis zum Ende ihrer Gnadenfrist (ADR-0018 Regel 2a).
  - **Herleitung** (Kommentar der Konstante `JE_HERKUNFT`, `src/anwesenheit/bestand.rs`): im ungünstigen Fall des Profils sitzen 50 Agenten und ein Mensch an **einem** Rechner = 51 Sitzungen; startet jeder Agent einmal mit neuem Namen neu, hält die alte Sitzung noch die Gnadenfrist (60 s) → bis 101. 128 trägt das mit ≈ 27 % Reserve ([A00001](../Architektur/A00001-localchata.md), Ergänzung „Lastprofil Team-LAN“).
  - **Warum nicht gleich den Verbindungen (160)**: die beiden Grenzen schützen Verschiedenes. Verbindungen sind Plätze der Annahmeschleife und müssen die **Spitze** gleichzeitiger Anfragen tragen (kurze Sendeverbindungen zusätzlich zu den wartenden); Sitzungen sind **Namen** und wachsen nur mit Teilnehmern und Neustarts. Die Grenze gegen Namensbesetzung (Pre-Mortem 2) soll so klein sein, wie das Profil erlaubt. Der Satz „gleich den Verbindungsgrenzen“ und „die 64 Plätze je Gegenstelle begrenzen Verbindungen, die 64 Sitzungen je Herkunft Namen“ (Zustand) lesen sich ab hier „160 Plätze … 128 Sitzungen“; die Unabhängigkeit beider bleibt.
  - **Gesamt 512 bleibt**: das Profil braucht 70 bzw. mit Neustarts 140; vier Herkünfte mit je 128 erreichen die Gesamtgrenze.
  - **Annahme, an der 128 hängt**: Agenten nutzen beim eigenen Neustart die **Wiederkehr** (Token gemerkt, gleicher Name → 200 mit derselben Sitzung) statt eines neuen Namens. Ein Agent, der je Aufruf neu beitritt oder eine lange Gnadenfrist verlangt (`gnadenfrist_s` bis 3600, I0014) und dann mit neuem Namen neu startet, belegt je Neustart einen Platz bis zu einer Stunde — dagegen hilft keine Grenze. Deshalb gehört „eine Sitzung je Agent wiederverwenden (Token-Datei)“ in das Werkzeug für Agenten (I0019) und in die Agenten-Anleitung; Früh-Indikator ist die `WARNING sitzung`-Episode der Sitzungsgrenze je Herkunft ([ADR-0042](0042-stufen-ereigniskatalog-episoden-und-tagesgrenze-des-betriebsprotokolls.md) Regel 2).
  - Erster Test: R00015 G2 (128 × 201 von `127.0.0.1`, die 129. → 429, `127.0.0.2` → 201; 4 × 128 = 512 gesamt, die 513. → `ZuVieleSitzungen`; Mutanten 127 und 129).
- **2026-10-29 (Karte #36, R00025 F0088, B0410; Code-Review #22 A2; WBS Offene Fragen #22 Punkt 8) — ein Weg zum Wiederbeitritt für Strom und Senden, auch beim Scheitern:** Präzisiert den stillen Wiederbeitritt im Browser, auf den [ADR-0037](0037-zustellung-im-tab-vorlaeufig-bestaetigt-zeichnen-je-frame.md) Regel 4 (401 beim Senden) verweist; ersetzt wird nichts. Heute gibt es zwei Wege (`web/start.js:104–121` `stillWiederBeitreten` für den Strom über `sitzung.nachAblehnung` mit Geschwister-Tabs und Beitreten-Bildschirm beim Scheitern; `:123–140` `nachSendeAblehnungBeitreten` für das Senden über `sitzung.beitreten` direkt, ohne Geschwister, Scheitern still — die Nachricht wird mit dem Fehler der zweiten 401 fehlgeschlagen). Eine 401 beim Senden heißt aber: das Token ist tot, der Strom stößt beim nächsten Verbinden auf dieselbe Ablehnung. Beide Wege enden also ohnehin im selben Zustand; offen war nur, was beim Scheitern mit Warteschlange, ungesendeten Nachrichten und Entwurf geschieht.
  - Option A: **ein Weg; Scheitern → Beitreten-Bildschirm wie im Strom; die ungesendeten Nachrichten überstehen den Bildschirm und stehen nach dem nächsten Beitritt als fehlgeschlagen da; nichts wird automatisch gesendet; der Entwurf bleibt** (**gewählt**). Stärke: ein Ergebnis gleich welcher Weg die Ablehnung zuerst sieht; das System wirft keinen Text weg, den der Mensch nicht verworfen hat — gerade die Zusage von R00012 und R00025 („geht nicht verloren“); der Mensch entscheidet nach dem Beitritt, ob er erneut sendet (unter Umständen unter neuem Namen). Schwäche: die Zustellung muss den Wechsel der Ansicht überleben — sie gehört dann dem Tab, nicht der Hauptansicht (ein Umbau in `web/start.js` und `web/hauptansicht.js`).
  - Option B: ein Weg; Scheitern → Beitreten-Bildschirm; ungesendete Nachrichten verfallen wie beim Neuladen ([ADR-0037](0037-zustellung-im-tab-vorlaeufig-bestaetigt-zeichnen-je-frame.md) Regel 5). Stärke: kleinster Bau, kein Zustand über den Wechsel. Schwäche: das Neuladen ist eine Handlung des Menschen, der Wechsel hier eine des Systems — es verlöre Text, ohne dass jemand „Verwerfen“ sagte.
  - Option C: das Senden bleibt im Verlauf — die Nachricht wird fehlgeschlagen mit „Sitzung abgelaufen“ und einem Schalter „Neu beitreten“; nur der Strom zeigt den Bildschirm. Stärke: kein Kontextverlust. Schwäche: zwei Wege bleiben (gegen F0088); der Strom zeigt den Bildschirm ohnehin wenige Sekunden später.
  - Option D: wie A, aber nach dem Beitritt automatisch senden. Stärke: kein Klick. Schwäche: sendet unter einer womöglich neuen Identität ohne Zustimmung; ein neuer Name ist eine neue Kennung, für die der alte Wiederholschlüssel nicht gilt.
  - **Gewählt: A.** In Kauf genommen: der Umbau der Zuständigkeit für die Zustellung; wer auf dem Beitreten-Bildschirm neu lädt, verliert die ungesendeten Nachrichten (Regel 5 von ADR-0037 bleibt).
  - **Regeln** (Browser):
    1. **Ein Weg**: eine Funktion im Tab für jede Ablehnung — `wiederBeitreten(abgelehntesToken)`: unter dem Web Lock `localchatA.sitzung` (ohne Web Locks: Warten auf das `storage`-Ereignis des Nachbar-Tabs wie beim Laden) `sitzung.nachAblehnung` → hat ein anderer Tab schon ein neues Token gemerkt, gilt es (kein `POST /api/beitreten`); sonst stiller Beitritt mit dem gemerkten Namen. Strom (`beitreten: true` aus dem Verteiler) und Senden (401) rufen dieselbe Funktion; `nachSendeAblehnungBeitreten` entfällt.
    2. **Gleichzeitig**: melden Strom und Senden dieselbe Ablehnung, gibt es einen Lauf — der zweite Aufrufer wartet auf das Ergebnis des ersten (ein laufendes Versprechen je abgelehntem Token).
    3. **Gelingt es**: der Strom bekommt das Token (`tokenSetzen`), der eine neue Versuch des Sendens (ADR-0037 Regel 4) trägt es.
    4. **Scheitert es** (Zustand `beitreten`, `vergeben`, `ungueltig`, `fehler`, `serverWeg`), gleich welcher Weg es auslöste:
       - die Warteschlange hält sofort an — kein weiterer Sendeversuch mit dem toten Token;
       - jede vorläufige Nachricht wird fehlgeschlagen mit dem Grund „Nicht gesendet — der Beitritt war abgelaufen.“ (Wortlaut der UX-Vorprüfung #36 V11, R00025 F0088; ersetzt den Vorschlag „… die Sitzung war beendet.“ — Ergänzung „Karte #36 Wortlaut“ unten), fehlgeschlagene und verworfene bleiben, wie sie sind (verworfene mit ihrer Uhr, ADR-0037, Ergänzung „Verworfen auf Zeit“);
       - der Strom wird beendet, der Beitreten-Bildschirm zeigt den Zustand — wie heute im Strom;
       - die Zustellung lebt im Tab weiter (nicht in der Hauptansicht). Nach dem nächsten Beitritt — gleich unter welchem Namen — stehen ihre Nachrichten in ihren Channels unter dem Verlauf, mit „Erneut senden“ und „Verwerfen“; gesendet wird erst auf „Erneut senden“, mit dem gespeicherten Schlüssel (unter einer neuen Kennung ist er dort neu; unter derselben schützt er weiter vor dem Doppel);
       - der Entwurf bleibt im `sessionStorage` und liegt nach dem Beitritt wieder im Feld (ADR-0038 Regel 7, unverändert).
    5. **Neuladen** auf dem Beitreten-Bildschirm: wie ADR-0037 Regel 5 — ungesendete Nachrichten sind weg, der Entwurf bleibt.
  - **Erste Tests** (für den `planer`; ergänzen B0410):
    - Node mit gestellter API: Strom und Senden melden dasselbe abgelehnte Token im selben Task → genau ein `POST /api/beitreten`. Mutante „zwei Läufe“ → rot.
    - Node: stiller Beitritt scheitert (409 `vergeben`) nach einer 401 beim Senden, zwei vorläufige in der Warteschlange → kein weiterer `POST …/nachrichten`, beide fehlgeschlagen mit dem Grund aus Regel 4, der Zustand `vergeben` geht an `zeigen`; dasselbe, wenn der Strom die Ablehnung zuerst meldet. Mutante „Senden scheitert still wie heute“ → rot.
    - E2E: wie R00025 „Scheitern“ — 401 auf das Senden, Beitritt scheitert → Beitreten-Bildschirm wie bei einer 401 im Strom; danach Beitritt unter anderem Namen → die Nachricht steht fehlgeschlagen mit „Erneut senden“; Klick → gesendet unter dem neuen Namen, genau ein Satz. Entwurf im Feld wie vorher.
- **2026-10-29 (Karte #36 Wortlaut; R00025 F0088, B0412; UX-Vorprüfung #36 V11; R00025 Offene Frage 5) — Grund beim gescheiterten Wiederbeitritt:** Regel 4 der Ergänzung Karte #36 nennt als Grund „Nicht gesendet — der Beitritt war abgelaufen.“ statt des Vorschlags „Nicht gesendet — die Sitzung war beendet.“ Grund: die Oberfläche spricht vom Beitreten, nicht von Sitzungen (Beitreten-Bildschirm, R00025 „Wartende Nachrichten genannt“); „Sitzung“ ist ein Wort der API. Sonst unverändert; der erste Test (Node, „Scheitern im Tab“) prüft den Text wörtlich.
