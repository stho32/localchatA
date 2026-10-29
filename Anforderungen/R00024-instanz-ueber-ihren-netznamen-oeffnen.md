---
id: R00024
status: Neu
datum: 2026-10-29
---

# R00024: Instanz über ihren Netznamen öffnen

## Beschreibung

Ein Teilnehmer öffnet die Instanz über den Namen, den das eigene Netz ihr gibt, ohne dass der Betreiber etwas einstellt. Gemeint ist der Name, den der Router dem Rechner gibt (`http://nuc5.fritz.box:7430/`), oder ein Name unter `.local`, `.home.arpa` oder `.internal` bzw. ein einteiliger Name, der auf eine eigene Adresse zeigt. Jeder andere Name, vor allem jeder öffentliche, bleibt ohne `--erreichbar-als` abgewiesen und löst nicht einmal eine Namensauflösung aus. Die Abweisung ist für einen Browser eine deutsche Seite mit einem klickbaren Link auf die IP-Adresse, die sofort funktioniert; ein Agent bekommt Text mit derselben Adresse. Für bewusste Ausnahmen gibt es den Schalter `--alle-namen`: er nimmt jeden Namen an, warnt beim Start und lässt die Origin-Regeln aktiv.

Zahlt ein auf: [Vision](R00000-vision.md) — „Eine gemeinsame Instanz im LAN, die alle Teilnehmer ansprechen — auch über Rechnergrenzen“ und „Keine Anmeldung“ (der Schutz gegen fremde Websites bleibt im Grenzschutz, nicht in einer Anmeldung). Kein Widerspruch zur Vision: öffentliche Namen bleiben eine bewusste Entscheidung des Betreibers, die Instanz wird kein öffentlicher Internet-Dienst („Ausdrücklich nicht Ziel“).

Auslöser ist der Befund des Betreibers vom 2026-10-29 (Karte #35, Vorrang): „Wenn ich über DNS zugreife, bekomme ich die Meldung, die Instanz wäre unter dem Namen xy nicht freigegeben. Das ist aber etwas, das viele Browser nicht verstehen. Daher soll es möglich sein, den DNS-Namen zu verwenden, solange dieser auf die richtige IP aufgelöst wird.“ Die Richtung hat der Betreiber nach der Erklärung des Rebinding-Risikos entschieden: Namen des eigenen Netzes automatisch, öffentliche Namen nur mit `--erreichbar-als`, eine verständliche Seite statt einer nackten 403, ein bewusster Schalter `--alle-namen`.

Entschieden vom architekt: [ADR-0045](../Dokumentation/ADRs/0045-host-namen-des-eigenen-netzes-automatisch-oeffentliche-nur-ausdruecklich.md) (Regeln 1–9, Schnitt; ersetzt [ADR-0014](../Dokumentation/ADRs/0014-grenzschutz-host-origin-sicherheitsheader.md) Regel 1, die Regeln 2–6 gelten unverändert). Bausteine: [A00004 HTTP-Schnittstelle](../Dokumentation/Architektur/A00004-http-schnittstelle.md) (Namensauskunft, Grenzschutz) und [A00003 Start](../Dokumentation/Architektur/A00003-start.md) (lokale Adresse je Verbindung, `--alle-namen`), je Ergänzung Karte #35.

Slices (WBS `Dokumentation/Planung/localchata.md`, alle unter I0024 Instanz über ihren Netznamen öffnen, Dialog D0001 Start und Ablage): F0077 Über den Rückwärtsnamen öffnen, F0078 Name im lokalen Namensraum, F0079 Abweisung als verständliche Seite, F0080 Schalter `--alle-namen`.

## Geschäftlicher Nutzen

Im Heimnetz und im kleinen Büro ist der Name, den der Router vergibt, der normale Weg zu einem Rechner. Heute weist die Instanz genau diesen Namen ab, und zwar mit einem `text/plain`, das der Teilnehmer im Browser nicht einordnen kann und dessen Ausweg (`--erreichbar-als`) nur der Betreiber hat. Mit R00024 funktioniert `http://nuc5.fritz.box:7430/` ohne Startoption, ohne Neustart und auch nach einem DHCP-Wechsel. Wo ein Name trotzdem abgewiesen wird, zeigt die Seite den Link, der sofort funktioniert, und sagt dem Betreiber, was er tun kann. Der Schutz gegen DNS-Rebinding bleibt dabei bestehen: eine fremde Website, die ein Teilnehmer öffnet, bekommt weiterhin keinen Zugriff auf Channels, Verläufe und Fristen.

## Funktionale Anforderungen

### Über den Rückwärtsnamen öffnen (F0077; ADR-0045 Regeln 1–3, 5, 6, 8)

- Ein Host-Name, den die Rückwärtsauflösung der lokalen Adresse der Verbindung liefert, ist erlaubt, ohne Groß-/Kleinschreibung und mit oder ohne abschließenden Punkt.
- Vor jeder Prüfung wird der Host-Anteil in die Normalform gebracht; was sie verletzt, ist abgewiesen, ohne dass etwas aufgelöst wird.
- IP-Literale, `localhost`, der Rechnername samt `<rechnername>.local` und die Namen aus `--erreichbar-als` bleiben ohne Auflösung erlaubt.
- Ein Name außerhalb des lokalen Namensraums (also jeder öffentliche Name) löst nie eine Vorwärtsauflösung aus und ist nur mit `--erreichbar-als` erlaubt.
- Auflösungen laufen begrenzt: höchstens 2 gleichzeitig, höchstens 1 s Wartezeit je Anfrage, Ergebnisse im Cache (erlaubt 5 min, nicht erlaubt und Zeitgrenze 60 s, höchstens 256 Einträge).
- Das Betriebsprotokoll nennt je neuem Cache-Eintrag „erlaubt“ einmal den Namen und die lokale Adresse; eine zu langsame Auflösung steht als Warning-Episode darin.

### Name im lokalen Namensraum (F0078; ADR-0045 Regeln 4, 6, 8)

- Ein einteiliger Name oder ein Name unter `.local`, `.home.arpa`, `.internal` ist erlaubt, wenn seine Vorwärtsauflösung mindestens eine eigene Adresse liefert (Loopback zählt).
- Suffixe werden nur an Label-Grenzen erkannt; es gibt kein abgeleitetes Suffix (nicht aus der Suchdomäne, nicht aus dem Rückwärtsnamen).

### Abweisung als verständliche Seite (F0079; ADR-0045 Regel 7)

- Ein abgewiesener Host bekommt weiter 403. Fragt ein Browser nach einem Dokument, ist der Körper eine deutsche HTML-Seite ohne Stil und Skript mit einem klickbaren Link auf `http://<lokale Adresse>:<port>/`.
- Alle anderen Clients bekommen Text mit dem Namen, der Adresse und dem Ausweg; es gibt keine Umleitung.

### Schalter `--alle-namen` (F0080; ADR-0045 Regel 9)

- `--alle-namen` oder `LOCALCHATA_ALLE_NAMEN=1` nimmt jeden Host in Normalform an, ohne die Namensauskunft zu fragen.
- Origin- und `Sec-Fetch-Site`-Regel, CORS und Sicherheitsheader aus ADR-0014 bleiben aktiv.
- Der Zustand ist sichtbar: als WARNING beim Start, in `--hilfe` und im Health-Check.

## Nicht-funktionale Anforderungen

- **Sicherheit** (DNS-Rebinding, CWE-350; Flut, CWE-400): kein Name, den ein Angreifer im öffentlichen DNS besitzt, wird automatisch erlaubt, und keiner löst eine Auflösung aus. Mit `--alle-namen` ist der Rebinding-Weg bewusst offen, CSRF bleibt durch ADR-0014 Regel 2 geschlossen. Die Angriffsmatrix schreibt der `security-reviewer` vor dem Bau in ADR-0045; sie ist Teil dieser Anforderung (Kriterium „Angriffsmatrix“).
- **Leistung**: der Laufzeit-Takt löst nie auf; eine Anfrage wartet höchstens 1 s auf eine Auflösung. Zweck der Grenze: ein frei wählbarer `Host` darf die Instanz nicht blockieren, ein nicht vorhandener `.local`-Name kostet sonst 5 s (Probe in ADR-0045). Die Uhr startet beim Eingang der Anfrage im Grenzschutz. Erwartet sind im Heimnetz 4–141 ms (gemessen in der ADR-Probe); die Grenze liegt also weit über 130 % des erwarteten Werts. Geprüft wird die Regel exakt mit gestellter Uhr; im echten Programm gibt es kein Zeitkriterium.
- **Plattform**: Rückwärtsauflösung über `dns-lookup` unter Linux, macOS und Windows; „eigene Adresse“ über `UdpSocket::bind((ip, 0))`.
- **Zeit- und Konsolenkriterien** nach `AI-Team/planer.md`.

## Akzeptanzkriterien

Gemeinsam für alle Integrationskriterien: Start im Debug-Build mit `localchatA <ordner> --port 0 --host 127.0.0.1 --kein-browser` auf einem frischen Wegwerf-Ordner, `TZ=Europe/Berlin`. `<p>` ist der Port aus der Startzeile, `heute.log` die Datei `logs/<lokales Datum>.log`. „Testnamen“ heißt `LOCALCHATA_TESTNAMEN='127.0.0.1>nuc5.fritz.box;chat.local=127.0.0.1;drucker.local=192.0.2.1;boese.example=127.0.0.1;chat=127.0.0.1;nas.home.arpa=127.0.0.1;dienst.internal=127.0.0.1;chat.fritz.box=127.0.0.1'`. Angefragt wird immer `http://127.0.0.1:<p>/api/gesundheit` mit dem genannten `Host:`-Kopf. „Erlaubt“ heißt 200 mit `{"status":"bereit",…}`, „abgewiesen“ heißt 403.

### Sicherheit

- [ ] **Angriffsmatrix ADR-0045 vollständig grün**: jede Zeile der Angriffsmatrix, die der `security-reviewer` vor dem Bau in [ADR-0045](../Dokumentation/ADRs/0045-host-namen-des-eigenen-netzes-automatisch-oeffentliche-nur-ausdruecklich.md) ergänzt (Abschnitt „Angriffsmatrix“), ist ein Test über das echte Programm und grün; jede Zeile, die einen Wächter betrifft, wird mit dessen Mutante rot.
- [ ] **Mutationsprobe je Wächter** (ADR-0045, Durchsetzung): abgeschaltet werden einzeln Normalform, Label-Grenze, „nur der lokale Namensraum löst auf“, „eigene Adresse“, Zeitgrenze, Cache-Ablauf und „Ursprungsregel bei `--alle-namen`“; jede Mutante macht mindestens einen Test rot. Das Ergebnis steht im Bericht des Entwicklers.

### F0077 Über den Rückwärtsnamen öffnen

- [ ] **Abhängigkeitsprobe zuerst** (Review-Punkt; Bedingung des architekt in ADR-0045): der erste Commit dieser Interaction ist die Probe `dns-lookup` nach Skill `dependency-probe`. Sie enthält mindestens einen Happy Path (`127.0.0.1` → `localhost`) und eine Fault-Injection (nicht vergebene Adresse oder Resolver ohne Antwort → leeres Ergebnis bzw. Fehler ohne Panik, der Faden endet). Messwerte, Lizenz und Urteil stehen im Bericht. Trägt die Probe nicht, endet der Bau mit `blockiert`, und der architekt schreibt eine neue ADR.
- [ ] **Host-Urteil** (Unit, reine Operation; erster Test aus A00004): Namensauskunft als Tabelle `192.168.178.65 → nuc5.fritz.box` (rückwärts) und `boese.example → 192.168.178.65` (vorwärts, eigen).
  - `nuc5.fritz.box:7430` über die lokale Adresse `192.168.178.65` → erlaubt mit Grund „Rückwärtsname“.
  - `nuc5.fritz.box.` und `NUC5.Fritz.Box` → erlaubt.
  - `boese.example` → abgewiesen, und die Tabelle zählt für `boese.example` 0 Fragen.
  - `nuc5.fritz.box` über die lokale Adresse `127.0.0.1` (Tabelle ohne Eintrag dafür) → abgewiesen.
- [ ] **Normalform** (Unit, rein):
  - `Chat.LAN.` → `chat.lan`; `xn--bcher-kva.example` bleibt unverändert.
  - Abgewiesen ohne Auflösung: `a_b.example` (Unterstrich), `a..b` (leeres Label), ein Label aus 64 Zeichen, ein Name aus 254 Zeichen. Erlaubt als Form: ein Label aus 63 Zeichen, ein Name aus 253 Zeichen.
  - `evil-local` und `local.evil.example` enden nicht auf dem Label `.local`.
- [ ] **Rückwärtsname über das echte Programm** (Integration, Testnamen):
  - Erlaubt: `Host: nuc5.fritz.box:<p>`, `Host: NUC5.Fritz.Box:<p>`, `Host: nuc5.fritz.box.:<p>`.
  - Abgewiesen: `Host: boese.example:<p>` (die Testnamen zeigen ihn auf `127.0.0.1`, eine eigene Adresse, trotzdem 403); `Host: a_b.example:<p>`.
  - Bleibt erlaubt: `Host: 127.0.0.1:<p>`, `Host: localhost:<p>`; mit `--erreichbar-als chat.firma.de` ist `Host: chat.firma.de:<p>` erlaubt.
- [ ] **Rückwärtsname im echten Netz** (Integration, echtes DNS; nur wenn der Rechner eine Nicht-Loopback-IPv4 `<lan-ip>` hat und `getent hosts <lan-ip>` einen Namen `<name>` liefert, sonst als übersprungen gemeldet): Start mit `--host 0.0.0.0` ohne Testnamen; `GET http://<lan-ip>:<p>/api/gesundheit` mit `Host: <name>:<p>` → erlaubt.
- [ ] **Cache, Gleichzeitigkeit und Zeitgrenze** (Unit, gestellte Uhr, blockierende Testauskunft):
  - Gleichzeitigkeit: 4 verschiedene Namen, je Auflösung 3 s → zu keinem Zeitpunkt laufen mehr als 2 Auflösungen.
  - Zeitgrenze, Rechenbeispiel: die Auflösung von `nuc5.fritz.box` dauert 3 s und ergibt „erlaubt“. Anfrage A bei t = 0 → bei t = 1 s abgewiesen. Die Auflösung endet bei t = 3 s. Anfrage B bei t = 3,1 s → erlaubt aus dem Cache, ohne neue Auflösung (die Testauskunft zählt 1 Frage).
  - Geteilte Auflösung: zwei Anfragen für denselben Schlüssel bei t = 0 und t = 0,2 s → 1 Frage.
  - Ablauf „erlaubt“: bei t = 4:59 min keine neue Frage, nach 5 min eine neue Frage.
  - Ablauf „nicht erlaubt“, Rechenbeispiel: `drucker.local` abgewiesen bei t = 0 (1 Frage), wieder bei t = 59 s (weiter 1 Frage), wieder bei t = 61 s (2 Fragen).
  - Größe: nach 257 verschiedenen Schlüsseln ist der zuerst eingetragene nicht mehr im Cache (seine nächste Anfrage fragt neu), der zuletzt eingetragene schon.
  - Die Zeitgrenze ergibt genau eine `WARNING grenzschutz`-Episode „Namensauflösung zu langsam“.
- [ ] **Protokoll** (Integration, Testnamen): zwei Anfragen mit `Host: nuc5.fritz.box:<p>` → `heute.log` enthält genau eine Zeile `INFORMATION grenzschutz Host-Name über Rückwärtsauflösung erlaubt` mit `name=nuc5.fritz.box` und `adresse=127.0.0.1`.
- [ ] **Testknopf nur im Debug-Build** (Integration):
  - Debug-Build: `LOCALCHATA_TESTNAMEN=kaputt` → Exit 2, eine Zeile auf stderr nennt `LOCALCHATA_TESTNAMEN`.
  - Release-Build mit denselben Testnamen: `Host: nuc5.fritz.box:<p>` → abgewiesen (die echte Rückwärtsauflösung von `127.0.0.1` liefert `localhost`); `LOCALCHATA_TESTNAMEN=kaputt` → die Instanz startet.

### F0078 Name im lokalen Namensraum

- [ ] **Urteil** (Unit, reine Operation; Rest des ersten Tests aus A00004): Tabelle `chat.local → 192.168.178.65` (eigen) und `drucker.local → 192.168.178.99` (fremd).
  - `chat.local` → erlaubt mit Grund „lokaler Namensraum“; `drucker.local` → abgewiesen.
  - `local.boese.example` → abgewiesen, und die Tabelle zählt dafür 0 Fragen.
- [ ] **Über das echte Programm** (Integration, Testnamen):
  - Erlaubt: `chat.local`, `chat`, `nas.home.arpa`, `dienst.internal` (alle auf `127.0.0.1`).
  - Abgewiesen: `drucker.local` (auf `192.0.2.1`, nicht eigen); `local.boese.example`; `evil-local`.
  - Abgewiesen trotz Eintrag auf `127.0.0.1`: `chat.fritz.box` (kein abgeleitetes Suffix); mit `--erreichbar-als chat.fritz.box` erlaubt.
- [ ] **Protokoll** (Integration, Testnamen): zwei Anfragen mit `Host: chat.local:<p>` → genau eine Zeile `INFORMATION grenzschutz Host-Name über lokalen Namensraum erlaubt` mit `name=chat.local` und `adresse=127.0.0.1`.

### F0079 Abweisung als verständliche Seite

- [ ] **Seite** (Unit, rein): Name `boese.example`, lokale Adresse `192.168.178.65`, Port 7430.
  - Die Seite enthält `<html lang="de">`, die Überschrift „Diese Adresse ist für localchatA nicht freigegeben“ und einen Link `<a href="http://192.168.178.65:7430/">http://192.168.178.65:7430/</a>`.
  - Sie enthält den Satz „Der Betreiber kann den Namen boese.example mit `--erreichbar-als boese.example` beim Start freigeben.“ und einen Satz, der „fremde Websites“ als Grund nennt.
  - Sie enthält kein `<style`, kein `style=`, kein `<script` und kein `on…=`-Attribut.
  - Lokale Adresse `::1` → Link `http://[::1]:7430/`.
  - Name `<script>x` → in der Seite nur als `&lt;script&gt;x`.
- [ ] **Browser bekommt die Seite** (Integration):
  - `GET /` mit `Host: boese.example:<p>` und `Accept: text/html` → 403, `Content-Type: text/html; charset=utf-8`, die Seite aus dem Unit-Kriterium mit dem Link `http://127.0.0.1:<p>/`.
  - Ebenso mit `Sec-Fetch-Dest: document` und `Accept: */*`.
  - Die Antwort trägt den Header-Satz aus ADR-0014 Regel 5 mit unveränderter `Content-Security-Policy` und keinen `Location`-Kopf.
- [ ] **Agent bekommt Text** (Integration): `GET /api/gesundheit` mit `Host: boese.example:<p>` und `Accept: application/json` → 403 mit `Content-Type: text/plain; charset=utf-8`. Der Text enthält `boese.example`, `http://127.0.0.1:<p>/`, `--erreichbar-als boese.example` und „Namen des eigenen Netzes gelten automatisch“. Ohne `Accept`-Kopf gilt dasselbe. Ohne `Host`-Kopf bleibt die Antwort wie heute (`text/plain`).
- [ ] **Im Browser** (E2E, Playwright, Chromium mit `--host-resolver-rules=MAP boese.example 127.0.0.1`):
  - `http://boese.example:<p>/` öffnen → die Überschrift „Diese Adresse ist für localchatA nicht freigegeben“ ist sichtbar.
  - Ein Klick auf den Link öffnet `http://127.0.0.1:<p>/` mit dem Titel `localchatA`.
  - Konsole: keine unbehandelte Ausnahme und kein CSP-Verstoß. Erlaubt sind nur die Meldungen „Failed to load resource“ zu den 403 auf das Dokument unter `boese.example` und auf `/favicon.ico` unter `boese.example`.

### F0080 Schalter `--alle-namen`

- [ ] **Aufruf** (Unit, rein; erster Test aus A00003):
  - `["/tmp/x", "--alle-namen"]` → `alle_namen: true`; ohne die Option → `false`.
  - Umgebung `LOCALCHATA_ALLE_NAMEN=1` → `true`; `0` oder leer → `false`; `ja` → Aufruffehler (Exit 2), der die Variable nennt.
  - `--alle-namen` mit `LOCALCHATA_ALLE_NAMEN=0` → `true` (die Option geht vor); `--alle-namen` mit `LOCALCHATA_ALLE_NAMEN=ja` → Exit 2 (Muster `--protokoll-stufe`).
  - `--hilfe` nennt `--alle-namen` und `LOCALCHATA_ALLE_NAMEN`, auf eigener Zeile den Risikosatz aus dem Kriterium „Sichtbar“, und dass Namen des eigenen Netzes automatisch gelten.
- [ ] **Jeder Name** (Integration, `--alle-namen`, Testnamen gesetzt):
  - `Host: boese.example:<p>` → erlaubt; die Testauskunft wird nie gefragt (Unit mit zählender Tabelle: 0 Fragen).
  - Abgewiesen bleibt: `Host: a_b.example:<p>`, eine Anfrage ohne `Host`-Kopf und `POST /api/beitreten` mit `Host: boese.example:<p>` und `Origin: http://andere.example:<p>`.
  - Bewusst offen (Rebinding-Weg, ADR-0045 Regel 9): `POST /api/beitreten {"name":"beta"}` mit `Host: boese.example:<p>` und `Origin: http://boese.example:<p>` → 201.
- [ ] **Sichtbar** (Integration):
  - Mit `--alle-namen`, ebenso mit `LOCALCHATA_ALLE_NAMEN=1`, steht vor der Startzeile in `heute.log` und auf stderr eine `WARNING grenzschutz`-Zeile mit dem Text „--alle-namen ist aktiv: jeder Host-Name wird angenommen — eine fremde Website, die ein Teilnehmer öffnet, kann über DNS-Rebinding Channels mitlesen, Nachrichten schreiben und über die Frist Verläufe löschen.“ (escaped nach ADR-0040).
  - `/api/gesundheit` meldet `alle_namen: true`; ohne den Schalter `alle_namen: false` und keine solche Zeile.

## Betroffene Verzeichnisstruktur

- `src/http/`: neu `namensauskunft.rs` (Port, echte Implementierung, Tabelle, Testknopf-Parser), `grenzschutz.rs` (Normalform, Urteil, Cache mit Semaphore und Zeitgrenze, Abweisungsseite, `alle_namen`), `gesundheit.rs` (Feld `alle_namen`); Unit-Tests im Modul.
- `src/start/`: `ausliefern.rs` (lokale Adresse je Verbindung), `aufruf.rs` (`--alle-namen`, Hilfe), `mod.rs` (Namensauskunft erzeugen, Startwarnung), ggf. ein Modul für `LOCALCHATA_ALLE_NAMEN` nach dem Muster `protokollstufe.rs`.
- `tests/`: Integration über das echte Programm (Muster `tests/gemeinsam/mod.rs`), Probe `dns-lookup`; `e2e/`: Abweisungsseite im Browser.
- `Cargo.toml`: neue Abhängigkeit `dns-lookup`.

## Technische Überlegungen

### Ablauf

1. **Start**: Aufruf lesen (`--alle-namen`, Umgebung) → … → Protokoll öffnen → bei `alle_namen` WARNING → Namensauskunft erzeugen (echt; im Debug-Build mit `LOCALCHATA_TESTNAMEN` als Tabelle) → Grenzschutz mit Rechnername, `--erreichbar-als`, Namensauskunft, `alle_namen` → Binden → Startzeile.
2. **Verbindung**: Annahmeschleife setzt `ConnectInfo` (Gegenstelle) und die lokale Adresse als Extensions.
3. **Anfrage im Grenzschutz**:
   - 3.1 Host-Anteil lesen; fehlt er oder ist er unlesbar → 403.
   - 3.2 Normalform; verletzt → 403 ohne Auflösung.
   - 3.3 Wenn `alle_namen`: weiter zu 3.6.
   - 3.4 Feste Liste (Regel 2) → erlaubt.
   - 3.5 Auskunft mit Cache: rückwärts für die lokale Adresse (Regel 3); nur im lokalen Namensraum vorwärts für den Namen und je Adresse „eigen?“ (Regel 4); Wartezeit ≤ 1 s → Urteil.
     - 3.5.1 Wenn abgewiesen: Episode `grenzschutz`, Antwort 403 als Seite (Dokument) oder Text.
   - 3.6 Ursprungsregel (ADR-0014 Regel 2) → Route; Header-Satz auf jede Antwort.

### Grobentwurf (Vorplanung)

Die Bubbles der WBS sind Vorplanung; welche Bubbles es am Ende werden, entscheidet der Entwickler.

- F0077: B0364 Abhängigkeitsprobe `dns-lookup` · B0365 Host in Normalform · B0366 Lokale Adresse je Verbindung · B0367 Namensauskunft als Tabelle und Testknopf · B0368 Echte Namensauskunft · B0369 Host-Urteil · B0370 Auskunft mit Cache, Gleichzeitigkeit und Zeitgrenze · B0371 Rückwärtsname im Grenzschutz
- F0078: B0372 Urteil im lokalen Namensraum · B0373 Vorwärtsauflösung im Grenzschutz
- F0079: B0374 Abweisungsseite bilden · B0375 Seite oder Text ausliefern
- F0080: B0376 Aufruf um `--alle-namen` · B0377 Alle Namen annehmen · B0378 Alle Namen sichtbar machen

Elemente nach ADR-0045, „Schnitt“:

- `Namensauskunft` (Port, Provider) — Rückwärtsnamen, Adressen eines Namens, eigene Adresse; echte Implementierung und Tabelle.
  - `namen_der_adresse(IpAddr) -> Vec<String>`
  - `adressen_des_namens(&str) -> Vec<IpAddr>`
  - `ist_eigene_adresse(IpAddr) -> bool`
- `host_urteil(normalisierter Name, feste Liste, Rückwärtsnamen, Vorwärtsadressen mit „eigen?“) -> Erlaubt(Grund) | Abgewiesen` (Operation, rein).
- `Grund` (Aufzählung: fest, Rückwärtsname, lokaler Namensraum, alle Namen).
- Auskunfts-Cache (Operation mit Uhr; einziger Schreiber ist der Grenzschutz).
- `LokaleAdresse` (Request-Extension, immutable).

### Änderungen an bestehenden Klassen

- `Grenzschutz::neu` bekommt Namensauskunft und `alle_namen`; `host_erlaubt` wird asynchron (Auflösung) oder teilt sich in Urteil und Auskunft.
- `abweisung` in `grenzschutz.rs`: Seite oder Text je `Sec-Fetch-Dest`/`Accept`, mit lokaler Adresse und Port.
- `Startauftrag` + `alle_namen`; `HILFE` + Zeilen.
- Gesundheitsantwort + `alle_namen`.
- Annahmeschleife: zweite Extension.

## Tests

Nach Skill `test-pyramide` und ADR-0009; die Szenarien der User Story werden Tests.

- **Unit (rein)**: Normalform, Host-Urteil (Matrix aus A00004), Urteil im lokalen Namensraum, Cache/Semaphore/Zeitgrenze mit gestellter Uhr und blockierender Testauskunft, Testknopf-Parser, Abweisungsseite und Text, Aufruf `--alle-namen`.
- **Integration (echtes Programm)**: Testnamen über `LOCALCHATA_TESTNAMEN`, Rückwärtsname im echten Netz (bedingt), Release-Probe, Seite und Text, `--alle-namen`, Protokollzeilen, Health-Check, Angriffsmatrix ADR-0045.
- **E2E**: Abweisungsseite unter `boese.example` mit Link zur Oberfläche; die bestehenden Playwright-Tests bleiben grün.
- **Probe**: `dns-lookup` als erster Commit (Skill `dependency-probe`), danach als Regressionsschutz im Repo, soweit sie ohne Netz läuft.

## Abhängigkeiten

- Abhängig von: I0004 Namen wählen und beitreten (gruen, R00003; darin F0006 Grenzschutz); F0051 Protokoll schreibt und F0055 Grenzen, Episoden und Tagesbudget (beide gruen, R00014).
- Blockiert: keine.
- Vor dem Bau: die Angriffsmatrix des `security-reviewer` in ADR-0045.

## Offene Fragen

- Keine für den Bau. Für R00024 entschieden (den Wortlaut der ADR gleicht der architekt an):
  - (1) `LOCALCHATA_TESTNAMEN` ersetzt Rückwärts- und Vorwärtsauflösung; `ist_eigene_adresse` bleibt echt. So ist „eigen“ über `127.0.0.1` und „fremd“ über `192.0.2.1` deterministisch.
  - (2) Der Text für Nicht-Browser ist nur in seinen Bestandteilen festgelegt (Name, Adresse, `--erreichbar-als <name>`, „Namen des eigenen Netzes gelten automatisch“), nicht im Wortlaut.
  - (3) Ohne `Host`-Kopf bleibt die Abweisung `text/plain` wie heute, auch für Browser (Browser senden immer einen `Host`).
  - (4) `--alle-namen` mit einem falschen Wert in `LOCALCHATA_ALLE_NAMEN` ergibt Exit 2, wie `--protokoll-stufe`.
- Vor dem Bau, nicht von dieser Anforderung zu beantworten: die Angriffsmatrix (`security-reviewer`).

## Manuelle Vorbereitungstätigkeiten

- Der `security-reviewer` ergänzt ADR-0045 um den Abschnitt „Angriffsmatrix“, bevor gebaut wird (Director).

## Manuelle Nachbereitungstätigkeiten

- `AI-Team/umgebung.md` (Director): Testknopf `LOCALCHATA_TESTNAMEN` (nur Debug-Build) und Schalter `--alle-namen`/`LOCALCHATA_ALLE_NAMEN` eintragen (ADR-0045, Folgeaufgaben).
- Abnahme auf dem Rechner des Betreibers über den echten Namen (`http://nuc5.fritz.box:<port>/`, User Story US-1).

## Umfang

Gezählt aus der WBS (Bubbles, 2026-10-29). 0,4 h ist die Untergrenze je Bubble nach dem Skill `work-breakdown-structure`; die Ist-Zeiten des Projekts liegen darunter. Der Wert ist deshalb ein Richtwert, für diese Bubble-Typen ungemessen.

- F0077 Über den Rückwärtsnamen öffnen: 8 Bubbles · 5 klar (2,0 h) · 3 unklar (6–12 h)
- F0078 Name im lokalen Namensraum: 2 Bubbles · 2 klar (0,8 h)
- F0079 Abweisung als verständliche Seite: 2 Bubbles · 1 klar (0,4 h) · 1 unklar (2–4 h)
- F0080 Schalter `--alle-namen`: 3 Bubbles · 3 klar (1,2 h)
- Zusammen: 15 Bubbles · 4,4 h klar + 8–16 h unklar; Wellen F0077 → F0078 ‖ F0079 ‖ F0080

## Warum löst diese Anforderung das Problem? (Pflicht)

Auslöser ist die Meldung „unter dem Namen … nicht freigegeben“, wenn der Betreiber die Instanz über den Namen öffnet, den die Fritz!Box seinem Rechner gibt. Der Wunsch heißt: jeder Name, der auf die richtige IP zeigt. Genau das ist aber der Weg des DNS-Rebinding. Ein Name ist nicht sicher, weil er auf die Instanz zeigt, sondern weil die Antwort aus dem eigenen Netz kommt.

Die Wirkungskette:

- Der Grenzschutz erlaubt den Rückwärtsnamen der angesprochenen Adresse und Namen im lokalen Namensraum. Deshalb funktioniert der Router-Name ohne Startoption, und ein Angreifer mit eigener Domain bekommt trotzdem keinen Zugriff.
- Öffentliche Namen lösen nie eine Auflösung aus, und Auflösungen sind begrenzt. Deshalb kann ein frei wählbarer `Host` die Instanz nicht blockieren.
- Die Abweisung wird für den Browser eine Seite mit Link. Deshalb kommt der Teilnehmer ohne Wissen über Startoptionen sofort weiter, und der Betreiber erfährt seinen Ausweg.
- `--alle-namen` ist sichtbar riskant. Deshalb bleibt die Ausnahme eine bewusste Entscheidung.

Der Hebel liegt im Grenzschutz, weil dort die Frage „welcher Name?“ schon heute entschieden wird. Eine Freigabeseite für den Betreiber (Option C der ADR) wäre die erste Verwaltungsfunktion ohne Rechte, und ein Redirect (Option E) nähme den Namen aus der Adresszeile.

## Missing-Docs

- Glossar fehlt (`Dokumentation/Domaenenwissen/` existiert nicht). Begriffe wie Namensauskunft, lokaler Namensraum, Rückwärtsname und Normalform stammen aus ADR-0045.
- Verhalten von `getnameinfo` mit `nss-mdns` und bei hängendem Resolver ist nur aus der Wegwerf-Probe des architekt bekannt; die Probe B0364 schließt die Lücke.

## Notizen

### Verworfene Alternativen

In ADR-0045 entschieden, hier nur genannt:

- Jeden Namen zur Laufzeit auflösen und annehmen, wenn er auf eine eigene Adresse zeigt (Option A, Wortlaut des Befunds): verworfen, weil damit DNS-Rebinding wieder vollständig offen wäre. Als Ausnahme lebt sie im Schalter `--alle-namen` weiter, ohne Auflösung.
- Nur der Rückwärtsname ohne lokalen Namensraum und ohne Seite (Option B): verworfen, weil `.local`-Aliase und der ratlose Teilnehmer bleiben.
- Freigabeseite für den Betreiber (Option C): verworfen, weil sie eine Verwaltungsfunktion wäre und gerade den Angreifernamen am leichtesten freigibt.
- Umleitung auf das IP-Literal (Option E): verworfen, weil der Name aus der Adresszeile verschwindet und Sitzung und Entwürfe am IP-Ursprung hängen.

### Bewusst außerhalb

- Öffentliche Namen und Aliase unter der Router-Domäne, die nicht der Rückwärtsname sind (`chat.fritz.box`, Split-DNS) — weiter `--erreichbar-als`.
- Umschalten von `--alle-namen` zur Laufzeit.
- Ein Artboard für die Abweisungsseite: sie ist ohne Stil (ADR-0045 Regel 7) und gehört zu keinem Dialog.
