# ADR-0045: Host-Namen — Namen des eigenen Netzes gelten automatisch, öffentliche Namen nur ausdrücklich; Abweisung als Seite mit Ausweg

**Status**: Vorgeschlagen
**Datum**: 2026-10-29
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #35)
**Kontext-Anforderung**: — (Befund des Betreibers 2026-10-29, Karte #35; ändert ADR-0014 Regel 1, alle übrigen Regeln von ADR-0014 gelten unverändert)
**Erprobt durch**: Wegwerf-Probe im Scratchpad, 2026-10-29, Rechner `nuc5` im Netz einer Fritz!Box (Rust, `dns-lookup` 4.0.2 für die Rückwärtsauflösung, `std::net::ToSocketAddrs` vorwärts, „eigene Adresse“ = `UdpSocket::bind((ip, 0))` gelingt): Rückwärtsauflösung `192.168.178.65` → `nuc5.fritz.box` (3,7 ms), ULA-IPv6 → `nuc5.fritz.box`, `127.0.0.1` → `localhost`, fremde LAN-Adresse `192.168.178.1` → `fritz.box`, nicht vergebene `10.255.255.1` → `NoName` (24 ms); Vorwärts `nuc5.fritz.box` → 5 Adressen, darunter eigene (5 ms), `nuc5.local` → eigene (141 ms), **`gibtsnicht.local` → keine Adresse nach 5,0 s** (mDNS-Zeitgrenze), `gibtsnicht.fritz.box` → keine (7 ms); Bind-Probe: `127.0.0.2`, LAN-, ULA- und globale IPv6-Adressen → eigen, `8.8.8.8` und `192.0.2.1` → fremd (`EADDRNOTAVAIL`). Außerdem: der Router filtert öffentliche Namen, die auf private oder Loopback-Adressen zeigen (`localtest.me` → `127.0.0.1` bei `1.1.1.1`, lokal ohne Antwort; `example.com` löst auf) — der Rebind-Schutz der Fritz!Box. Die Probe lief außerhalb des Projekts und ist gelöscht.

## Kontext

- **Befund des Betreibers (2026-10-29), wörtlich:** „Wenn ich über DNS zugreife, bekomme ich die Meldung die Instanz wäre unter dem Namen xy nicht freigegeben. Das ist aber etwas, das viele Browser nicht verstehen. Daher soll es möglich sein, den DNS Namen zu verwenden, so lange dieser auf die richtige IP aufgelöst wird.“
- **Heute** ([ADR-0014](0014-grenzschutz-host-origin-sicherheitsheader.md) Regel 1, `src/http/grenzschutz.rs`, `Grenzschutz::name_freigegeben`): erlaubt sind IP-Literale, `localhost`, der Rechnername aus `uname().nodename()` (nur Unix, `src/start/rechnername.rs`), `<rechnername>.local` und Namen aus `--erreichbar-als`. Alles andere → 403 `text/plain` mit „Diese Instanz ist unter dem Namen <name> nicht freigegeben — über die IP-Adresse öffnen oder mit --erreichbar-als <name> starten.“ Der Name, den der Router dem Rechner gibt (`nuc5.fritz.box`, Probe oben), ist damit abgewiesen — das ist der Normalfall im Heimnetz und in kleinen Büros. ADR-0014 hat das in Kauf genommen („`<rechnername>.fritz.box` … nicht automatisch erlaubt“).
- **Die Abweisung erreicht den Falschen.** Wer sie sieht, ist ein Teilnehmer im Browser; der Ausweg (`--erreichbar-als`) ist eine Startoption des Betreibers. Ein nacktes `text/plain` ohne klickbare Adresse lässt den Teilnehmer ratlos.
- **Die Spannung:** DNS-Rebinding ist genau „ein Name zeigt auf die IP der Instanz“. Der Angriffsweg: ein Teilnehmer öffnet irgendeine Website `boese.example`; deren DNS antwortet erst mit der eigenen Server-Adresse (Seite mit Skript wird geladen), dann mit der LAN-Adresse der Instanz. Für den Browser bleibt der Ursprung `http://boese.example:7430`; das Skript spricht jetzt die Instanz an, mit `Host: boese.example:7430` und `Origin: http://boese.example:7430`. Was dann gilt:
  - **Origin-Prüfung (Regel 2) hilft nicht** — Origin und Host stimmen überein, `Sec-Fetch-Site` ist `same-origin`.
  - **Cookie-freie Sitzung hilft halb** — das Token des Teilnehmers liegt im `localStorage` des echten Ursprungs ([ADR-0012](0012-identitaet-und-sitzung.md)), das Skript kann es nicht lesen und die bestehende Sitzung nicht übernehmen.
  - **Aber** das Skript tritt selbst bei (`POST /api/beitreten`, neuer Name, Herkunft = Adresse des Opfers, bis 64 Sitzungen je Herkunft), liest danach jeden Channel und jeden Verlauf und schickt ihn an `boese.example` zurück (die Antwort ist für das Skript lesbar, die CSP der Instanz greift nur in Dokumenten der Instanz), schreibt Nachrichten unter der Herkunft des Opfers, legt Channels an und **verkürzt Fristen** — Verkürzen löscht Tagesdateien ([ADR-0031](0031-frist-setzen-put-api-channels-name-frist-jeder-mit-sitzung-bestaetigung-beim-verkuerzen.md)). Kurz: eine fremde Website bekäme die Rechte eines Teilnehmers im LAN, und das Lesen trägt den Verlauf aus dem LAN hinaus.
  - Eine Auflösung **auf dem Server** („zeigt der Name auf mich?“) entscheidet nichts: der Server fragt dieselbe Namenskette, die der Angreifer beantwortet, und nach dem Umbiegen ist die Antwort ehrlich „ja“. Eine kurze TTL, ein Cache oder ein Zeitfenster verschieben nur, wann der Angreifer umbiegt.
- **Was einen Namen sicher macht, ist nicht seine Antwort, sondern wer sie gibt.** Ein Rebinding-Angreifer aus dem Internet kontrolliert nur Namen, die er im öffentlichen DNS besitzt. Namen, deren Antwort aus dem eigenen Netz kommt — die Rückwärtsauflösung der eigenen Adresse (Zone des Netzbetreibers), mDNS unter `.local` (nur Geräte im selben Segment), IANA-reservierte lokale Suffixe (`.home.arpa`, `.internal`), einteilige Namen (Suchdomäne bzw. `hosts`) —, kann er weder vergeben noch mit eigenen Inhalten belegen. Wer im LAN mDNS fälschen kann, spricht die Instanz ohnehin direkt an (Restrisiko (a) in der Ergänzung #4 zu ADR-0014, unverändert).
- **Plattformen:** die Rust-Standardbibliothek löst nur vorwärts auf und zählt keine Netzschnittstellen auf (ADR-0014, Kontext). Die Annahmeschleife ([ADR-0010](0010-eigene-annahmeschleife-hyper-http1.md)) kennt aber je Verbindung die **lokale Adresse**, über die der Client kam — das ist eine eigene Adresse ohne Aufzählung. Ob eine aufgelöste Adresse eine eigene ist, beantwortet `UdpSocket::bind((ip, 0))` auf Linux, macOS und Windows gleich (Probe).
- **Auflösen blockiert und kann lange dauern:** `getaddrinfo`/`getnameinfo` sind blockierend und nicht abbrechbar; ein nicht vorhandener `.local`-Name kostet 5 s (Probe). Abgewiesene Namen sind frei wählbar (Kopfzeile `Host`) — jede Auflösung je Anfrage wäre ein Hebel für Flut.
- Leitplanken: „Keine Anmeldung“, „Eine gemeinsame Instanz im LAN, auch über Rechnergrenzen“, „Lokal bzw. LAN als Einsatzraum“; „Ausdrücklich nicht Ziel: kein öffentlicher Internet-Dienst, keine Benutzerverwaltung und keine Rechte“ (Vision R00000).

## Optionen

### Option A: Jeden Namen zur Laufzeit auflösen, annehmen, wenn er auf eine eigene Adresse zeigt (Wortlaut des Betreibers)
- Vorteile: erfüllt den Wortlaut vollständig, auch für öffentliche Namen wie `chat.firma.de`; keine Liste, keine Startoption; ein Satz Code mit injizierbarem Resolver.
- Nachteile: **öffnet DNS-Rebinding wieder vollständig** (siehe Kontext — der Angreifer beantwortet die Frage des Servers); Cache und TTL ändern daran nichts. Jeder unbekannte `Host` löst eine blockierende Auflösung aus (Flut, 5 s je `.local`-Name). Im Netz des Betreibers liefe der Gewinn für öffentliche Namen sogar ins Leere: der Rebind-Schutz des Routers verwirft öffentliche Namen mit privater Antwort (Probe `localtest.me`).

### Option B: Eigene Namen des Rechners automatisch — Rückwärtsauflösung der angesprochenen Adresse, dazu Rechnername wie heute
- Vorteile: trifft den Fall des Betreibers genau (`nuc5.fritz.box`, auch der Name unter der ULA- und der globalen IPv6-Adresse, Probe); die Antwort kommt aus der Zone des Netzbetreibers, nicht vom Angreifer; keine Aufzählung der Schnittstellen nötig (Schlüssel ist die lokale Adresse der Verbindung); folgt DHCP-Wechseln von selbst; auch unter Windows (dort fehlt heute der Rechnername).
- Nachteile: neue Abhängigkeit für `getnameinfo` (`dns-lookup`, `unsafe_code = "forbid"` gilt nur für das eigene Crate); Aliase, die nicht der Rückwärtsname sind (`chat.fritz.box`, mDNS-Alias `chat.local`), bleiben abgewiesen; ein Router ohne PTR-Einträge liefert nichts.

### Option C: Verständliche Abweisung als HTML-Seite, Betreiber bestätigt neue Namen auf dem Server-Rechner (Seite unter `127.0.0.1`), Freigabe dauerhaft in einer Datei im Instanzordner
- Vorteile: jeder Name lässt sich ohne Neustart freigeben, auch öffentliche; der Teilnehmer sieht eine Seite statt eines Fehlers.
- Nachteile: die erste **Verwaltungsfunktion** in einem System ohne Rechte (Vision „keine Rechte“), gebunden an „sitzt am Server-Rechner“ — neue Angriffsfläche (CSRF und Rebinding gegen die Bestätigungsseite selbst; sie müsste nur unter IP-Literal und mit Origin-Prüfung erreichbar sein); **Sozialtechnik**: die Abweisungsseite eines Angreifers führt den Betreiber genau zu dem Klick, der den Angreifernamen freigibt; neuer Dialog (Artboard, Wireframe), neue Datei mit Pflege und Löschweg.

### Option D (gewählt): B, dazu A eingeschränkt auf den lokalen Namensraum, dazu der erste Teil von C (Abweisung als Seite mit klickbarem Ausweg) — ohne Bestätigungsseite
- Vorteile: erfüllt den Wortlaut des Betreibers für jeden Namen, dessen Antwort aus dem eigenen Netz kommt (Rückwärtsname, `.local`, `.home.arpa`, `.internal`, einteilige Namen), und hält DNS-Rebinding aus dem Internet geschlossen; der Teilnehmer sieht eine deutsche Seite mit der Adresse, die sicher funktioniert; öffentliche Namen bleiben eine bewusste Entscheidung des Betreibers (`--erreichbar-als`) — ohne neue Verwaltungsfunktion.
- Nachteile: ein öffentlicher Name, der auf die Instanz zeigt, funktioniert nicht von selbst — der Wortlaut ist dort nicht erfüllt (Frage an den Betreiber, siehe „Entscheidung“); zwei Auflösungswege mit Cache und Zeitgrenze statt einer Liste; neue Abhängigkeit.

### Option E: Unbekannten Namen bei Navigation auf das IP-Literal umleiten (307 auf `http://<lokale Adresse>:<port><pfad>`)
- Vorteile: für den Teilnehmer „funktioniert“ jeder Name, ohne jede Freigabe; für ein Rebinding-Skript harmlos (die Umleitung führt auf einen fremden Ursprung, CORS sperrt die Antwort).
- Nachteile: der Name verschwindet aus der Adresszeile — genau das, was der Betreiber nicht will; Lesezeichen und `localStorage` liegen dann am IP-Ursprung, ein Wechsel der Adresse (DHCP) verliert Sitzung und Entwürfe; verdeckt die Ursache.

## Entscheidung

**Gewählt: Option D.**

Es überwiegt, dass D den Normalfall des Betreibers — den Namen, den der Router dem Rechner gibt — ohne Startoption und ohne Bedienung löst, während die Grenze dort bleibt, wo die Sicherheit tatsächlich entsteht: bei der Frage, **wer** die Namensauflösung beantwortet, nicht **was** sie antwortet. In Kauf genommen: ein öffentlicher DNS-Name, der auf die Instanz zeigt, bleibt ohne `--erreichbar-als` abgewiesen; die Abweisung sagt es in Worten, die ein Teilnehmer versteht, und nennt die Adresse, die sofort funktioniert. A ist verworfen, weil sie das Schutzziel von ADR-0014 vollständig aufgibt; C ohne Bestätigungsseite übernommen, weil eine Verwaltungsfunktion der Vision widerspricht und gerade den Angreifernamen am leichtesten freigibt; E verworfen, weil sie den Namen aus der Adresszeile nimmt.

Die Entscheidung bleibt **Vorgeschlagen**, bis der Betreiber die Frage zu öffentlichen Namen beantwortet hat (Bericht an den Director, Karte #35) und die Abhängigkeitsprobe (Folgeaufgaben) im Projekt grün ist.

Regeln — **ersetzen ADR-0014 Regel 1**; Regeln 2 bis 6 von ADR-0014 und ihre Ergänzungen gelten unverändert:

1. **Normalform.** Der Host-Anteil (ohne Port, IPv6 in Klammern) wird für jede Prüfung normalisiert: ASCII-Kleinschreibung, ein abschließender Punkt entfernt, nur `[a-z0-9.-]`, Labels 1–63 Zeichen, gesamt höchstens 253 — sonst abgewiesen ohne Auflösung. Internationalisierte Namen kommen als `xn--…` an (Browser), werden nicht umgewandelt. Suffixe werden nur an Label-Grenzen verglichen (`evil-local` und `local.evil.example` sind kein `.local`).
2. **Erlaubt ohne Auflösung** (wie bisher): IP-Literal; `localhost`; der Rechnername und `<rechnername>.local` (Unix); jeder Name aus `--erreichbar-als`.
3. **Erlaubt: Rückwärtsname der angesprochenen Adresse.** Die Annahmeschleife setzt je Verbindung zusätzlich die **lokale Adresse** (kanonisch, wie die Gegenstelle in ADR-0012) als Request-Extension. Ist der Host-Name gleich einem Namen, den die Rückwärtsauflösung dieser lokalen Adresse liefert, ist er erlaubt. Keine Vorwärtsbestätigung: die Antwort stammt aus der Zone des Betreibers der eigenen Adresse.
4. **Erlaubt: Name im lokalen Namensraum, der auf eine eigene Adresse zeigt.** Lokaler Namensraum = einteiliger Name, oder Suffix `.local`, `.home.arpa`, `.internal`. Ist der Name darin und liefert die Vorwärtsauflösung mindestens eine Adresse, die eine eigene ist (`UdpSocket::bind((ip, 0))` gelingt; Loopback `127.0.0.0/8`, `::1` zählt), ist er erlaubt. Kein weiteres Suffix — insbesondere keine Suffixe aus der Suchdomäne oder aus dem Rückwärtsnamen abgeleitet (`fritz.box` → `*.fritz.box`): ein abgeleitetes Suffix könnte ein öffentliches sein (`host.com` → `*.com`), und ohne Public-Suffix-Liste ist das nicht entscheidbar.
5. **Alles andere** — insbesondere jeder öffentliche Name, gleich wohin er zeigt — ist nur mit `--erreichbar-als` erlaubt.
6. **Auflösung nie auf dem Laufzeit-Takt und nie unbegrenzt** (Baustein Namensauskunft, siehe „Schnitt“):
   - nur für Namen, die an Regel 2 scheitern und die Normalform erfüllen; Regel 4 nur für Namen im lokalen Namensraum — ein öffentlicher Name löst nie eine Auflösung aus;
   - blockierend nur in `spawn_blocking`, höchstens **2 gleichzeitig** (eigene Semaphore, nicht die Werkbank aus ADR-0039; die Erlaubnis hält der blockierende Faden bis zu seinem Ende, auch wenn niemand mehr wartet), Wartezeit der Anfrage höchstens **1 s** — danach gilt der Name für diese Anfrage als nicht erlaubt (Seite aus Regel 7), die Auflösung läuft zu Ende und füllt den Cache;
   - Cache je Schlüssel (Rückwärts: lokale Adresse; Vorwärts: Name): Ergebnis „erlaubt“ **5 min**, „nicht erlaubt“ oder Zeitgrenze **60 s**, höchstens **256 Einträge** (der älteste fällt); läuft für einen Schlüssel schon eine Auflösung, wartet die Anfrage auf diese, statt eine zweite zu starten;
   - Zeit über den Uhr-Provider ([X00007](../Architektur/Aspekte/X00007-zeit.md)), Auflösung über den Provider `Namensauskunft` — beide im Test ersetzt.
7. **Abweisung als Seite.** Ein abgewiesener Host bekommt weiter **403**. Fragt ein Browser nach einem Dokument (`Sec-Fetch-Dest: document` oder `Accept` enthält `text/html`), ist der Körper eine **HTML-Seite** (`text/html; charset=utf-8`, `lang="de"`, ohne Stil, ohne Skript — die CSP aus ADR-0014 Regel 5 bleibt unverändert): Überschrift „Diese Adresse ist für localchatA nicht freigegeben“, ein Satz für den Teilnehmer mit **klickbarem Link auf `http://<lokale Adresse>:<port>/`** (IPv6 in Klammern), ein Satz für den Betreiber („Der Betreiber kann den Namen <name> mit `--erreichbar-als <name>` beim Start freigeben.“), ein Satz zum Warum („Schutz davor, dass fremde Websites über einen umgeleiteten Namen mitlesen“). Der abgewiesene Name erscheint nur HTML-maskiert. Alle anderen (API, Agent, curl) bekommen den Text wie bisher, ergänzt um die Adresse. Kein Redirect (Option E).
8. **Protokoll** ([ADR-0042](0042-stufen-ereigniskatalog-episoden-und-tagesgrenze-des-betriebsprotokolls.md)): Abweisung wie bisher als Episode `grenzschutz` je Herkunft; neu je Cache-Eintrag einmal **Information** „Host-Name über Rückwärtsauflösung erlaubt“ bzw. „… über lokalen Namensraum erlaubt“ mit Name und lokaler Adresse; Zeitgrenze der Auflösung als **Warning**-Episode „Namensauflösung zu langsam“ (Bereich `grenzschutz`).

### Schnitt

- **Namensauskunft** (Provider, neu, Teil von [A00004](../Architektur/A00004-http-schnittstelle.md), Modul `src/http/namensauskunft.rs`): Port mit drei Operationen — `namen_der_adresse(IpAddr) -> Vec<String>`, `adressen_des_namens(&str) -> Vec<IpAddr>`, `ist_eigene_adresse(IpAddr) -> bool`. Echte Implementierung über `dns-lookup::lookup_addr`, `ToSocketAddrs`, `UdpSocket::bind`; Testimplementierung als feste Tabelle. Hoheit über den Cache hat der Grenzschutz (einziger Schreiber).
- **Grenzschutz** bleibt die äußerste Schicht; das Urteil über einen Namen ist eine reine Operation `host_urteil(normalisierter Name, feste Liste, Rückwärtsnamen, Vorwärtsadressen + eigen?) -> Erlaubt(Grund) | Abgewiesen` — die Auflösung davor ist Integration.
- **Annahmeschleife** ([A00003](../Architektur/A00003-start.md), `src/start/ausliefern.rs`): setzt neben `ConnectInfo` die lokale Adresse der Verbindung.
- **Testknopf** wie `LOCALCHATA_TESTPANIK` (nur Debug-Build, im Release nicht gelesen): `LOCALCHATA_TESTNAMEN=<name>=<ip>[,<ip>];…` und `<ip>><name>` für rückwärts ersetzt die Namensauskunft im echten Programm, damit Integration und E2E die Regeln ohne echtes DNS fahren; falscher Wert Exit 2.

## Konsequenzen

**Positiv:**
- Der Name, den der Router dem Rechner gibt, funktioniert ohne Startoption und ohne Neustart, auch über IPv6 und unter Windows; ein mDNS- oder `hosts`-Alias im lokalen Namensraum ebenso.
- DNS-Rebinding aus dem Internet bleibt geschlossen: ein Name, den ein Angreifer besitzt, wird nie automatisch erlaubt, und er löst nicht einmal eine Auflösung aus.
- Der Teilnehmer im Browser bekommt eine verständliche Seite mit einem Link, der sofort funktioniert.

**Negativ / Inkauf zu nehmen:**
- Öffentliche Namen (`chat.firma.de` → LAN-Adresse, auch Split-DNS im Firmennetz) und Aliase unter der Router-Domäne, die nicht der Rückwärtsname sind (`chat.fritz.box`), brauchen weiter `--erreichbar-als`.
- Der erste Zugriff über einen neuen Namen kann bis zu 1 s warten; ein nicht vorhandener `.local`-Name wird bis zu 60 s lang ohne neue Auflösung abgewiesen, ein gerade neu vergebener Rückwärtsname erst nach Ablauf des Negativ-Caches erlaubt.
- Wer im LAN mDNS, LLMNR oder die Rückwärtszone fälschen kann, kann einen Namen erlauben lassen — er spricht die Instanz aber ohnehin direkt an (Restrisiko (a) aus ADR-0014, unverändert).
- Neue Abhängigkeit `dns-lookup` (getnameinfo hinter einer sicheren Schnittstelle).

**Durchsetzung (Makro, Aspekte [X00002](../Architektur/Aspekte/X00002-benutzerrechte.md), [X00003](../Architektur/Aspekte/X00003-authentifizierung-und-sitzung.md)):**
- Unit-Tests der reinen Operation `host_urteil` als Matrix (Regeln 1–5) mit der Testimplementierung der Namensauskunft; die Angriffsmatrix schreibt der `security-reviewer` vor dem Bau.
- Integrationstest über das echte Programm mit `LOCALCHATA_TESTNAMEN`: erlaubter Rückwärtsname → 200; öffentlicher Name auf die eigene Adresse → 403, und die Namensauskunft wurde **nicht** gefragt; HTML-Seite bei `Accept: text/html`, Text bei `Accept: application/json`; Sicherheitsheader auf der Seite.
- Mutationsprobe je Wächter (Normalform, Label-Grenze, „nur lokaler Namensraum löst auf“, „eigene Adresse“, Zeitgrenze, Cache-Ablauf).
- Release-Probe: `LOCALCHATA_TESTNAMEN` wird im Release-Build nicht gelesen (wie `LOCALCHATA_TESTPANIK`).
- Review-Punkt: keine Auflösung außerhalb der Namensauskunft, keine Auflösung für Namen außerhalb des lokalen Namensraums.

**Folgeentscheidungen / Aufgaben:**
- [ ] Abhängigkeitsprobe `dns-lookup` im Projekt (Skill `dependency-probe`): Rückwärtsauflösung unter Linux mit `nss-mdns`, Verhalten bei hängendem Resolver (Zeitgrenze, Faden läuft aus), Lizenz, Build unter Windows im CI, falls vorhanden.
- [ ] Annahmeschleife setzt die lokale Adresse je Verbindung (A00003).
- [ ] Namensauskunft als Provider mit Testimplementierung und Testknopf; Grenzschutz mit Cache, Semaphore, Zeitgrenze; Abweisungsseite.
- [ ] `--hilfe` und Abweisungstext nennen, dass Namen des eigenen Netzes automatisch gelten.
- [ ] `AI-Team/umgebung.md` (Director): Testknopf `LOCALCHATA_TESTNAMEN` eintragen, sobald gebaut.

## Quellen / Bezuege

- [ADR-0014](0014-grenzschutz-host-origin-sicherheitsheader.md) (Regel 1 hier ersetzt, Restrisiko (a) der Ergänzung Karte #4), [ADR-0010](0010-eigene-annahmeschleife-hyper-http1.md), [ADR-0012](0012-identitaet-und-sitzung.md), [ADR-0031](0031-frist-setzen-put-api-channels-name-frist-jeder-mit-sitzung-bestaetigung-beim-verkuerzen.md), [ADR-0039](0039-grosse-antworten-byte-budget-je-block-werkbank-und-abrissgrenze-als-sendefrist.md), [ADR-0042](0042-stufen-ereigniskatalog-episoden-und-tagesgrenze-des-betriebsprotokolls.md)
- [A00004 HTTP-Schnittstelle](../Architektur/A00004-http-schnittstelle.md) (primär), [A00003 Start](../Architektur/A00003-start.md); Aspekte X00002, X00003, X00007
- Code: `src/http/grenzschutz.rs` (`abweisung`, `Grenzschutz::name_freigegeben`, `host_erlaubt`), `src/start/rechnername.rs`, `src/start/ausliefern.rs`
- Vision R00000, Leitplanken und „Ausdrücklich nicht Ziel“; Befund des Betreibers 2026-10-29 über den Auftrag des Directors (Karte #35)
- RFC 6762 (Multicast DNS, `.local` nur im Segment), RFC 8375 (`home.arpa`), ICANN-Beschluss 2024 zu `.internal` (reserviert für private Nutzung); W3C Fetch Metadata (`Sec-Fetch-Dest`)
