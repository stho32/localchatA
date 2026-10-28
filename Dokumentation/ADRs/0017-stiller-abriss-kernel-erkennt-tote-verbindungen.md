# ADR-0017: Stiller Abriss — der Kernel erkennt tote Verbindungen (Abrissgrenze und Keepalive je Verbindung), Herzschlag 5 s

**Status**: Vorgeschlagen — wartet auf den Messwert „echter Abriss“ (siehe Folgeaufgaben); der Bau hängt nicht daran, weil das Gate deterministisch ist
**Datum**: 2026-10-28
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #5 I0006)
**Kontext-Anforderung**: — (WBS I0006; prägt I0014, I0016)
**Erprobt durch**: Wegwerf-Probe im Scratchpad gegen das echte Programm (Stand afcd286, `target/debug/localchatA`), danach gelöscht. (1) Offener `GET /api/ereignisse` über einen rohen Socket, dann Abbruch: sauber (FIN) und hart (RST, `SO_LINGER 0`) je 20 Läufe → `/api/gesundheit` meldet `halter: 0` nach min 0,5 / Median 0,6 / max 0,9 ms, in beiden Fällen gleich — hyper liest auch während eines laufenden Stroms und bemerkt einen Socket-Fehler sofort. (2) Auf einem angenommenen Loopback-Socket lassen sich `TCP_USER_TIMEOUT` (10 000 ms), `SO_KEEPALIVE`, `TCP_KEEPIDLE`/`TCP_KEEPINTVL` (5 s) und `TCP_KEEPCNT` (2) setzen und genauso zurücklesen (Kernel 6.14, `tcp_retries2 = 15`). **Nicht geprobt**: ein echter stiller Abriss — unprivilegierte Netz-Namespaces sind auf dem Rechner gesperrt (`apparmor_restrict_unprivileged_userns = 1`), ohne sie lässt sich kein Paketverlust erzeugen.

## Kontext

- Vision „Wer da ist, ist sichtbar“: bricht die Verbindung ab, steht der Teilnehmer **sofort** als offline; ohne Rückkehr nach etwa einer Minute ist er weg. WBS I0006: hier bleibt der **stille** Abriss (WLAN weg, Rechner schläft) — der saubere Abbruch (Tab zu) ist mit I0005 gebaut (< 2 s, [ADR-0016](0016-anwesenheitsstatus-aus-haltern-mit-nachlauf-liste-als-stand-plus-wechsel.md)).
- [ADR-0007](0007-live-kanal-sse-fuer-browser-long-poll-fuer-agenten.md), Ergänzung 2026-10-28: ein Herzschlag in eine tote TCP-Verbindung gelingt in den Sendepuffer des Kernels; scheitern würde er erst nach der Wiederholungsgrenze von TCP — unter Linux mit `tcp_retries2 = 15` rund 15 Minuten. Bis dahin hält der Strom seinen Halter, und der Teilnehmer steht bei allen anderen als `online`. R00004 nennt das als Erwartung für die Abnahme bis I0006.
- Der Halter fällt, sobald die Antwort des Stroms abbricht (Probe 1: FIN wie RST in < 1 ms). Was fehlt, ist allein, dass der Server **merkt**, dass die Gegenstelle nicht mehr antwortet.
- Herzschlag heute 15 s (ADR-0007, Testknopf `LOCALCHATA_HERZSCHLAG_MS`); Wächter im Browser 40 s ohne Byte → neu verbinden (`web/verbindung.js`). Warte-Aufrufe der Agenten (I0016) senden während des Wartens nichts.
- Die Annahmeschleife ist Projektcode ([ADR-0010](0010-eigene-annahmeschleife-hyper-http1.md), `src/start/ausliefern.rs`): jede Verbindung geht durch eine Stelle, an der der Socket noch greifbar ist.
- Projektregel für Zeitkriterien (`AI-Team/planer.md`, `AI-Team/entwickler.md`, Gelerntes #4): Zweck, Startpunkt, erwarteter Wert, Grenze mit ≥ 30 % Reserve; die Regel exakt prüfen, die Zustellung als obere Grenze; deterministisch statt gegen Wanduhr oder Pollraster.

## Optionen

### Option A: Der Kernel erkennt es — je angenommener Verbindung `TCP_USER_TIMEOUT` und TCP-Keepalive, abgeleitet aus dem Herzschlag; Herzschlag 5 s
- Vorteile: eine Stelle (Annahmeschleife), kein Protokoll, keine neue Route; gilt für jeden Strom und jeden Warte-Aufruf gleich, auch für `curl` ohne Mitwirkung; der Abbruch kommt beim Server als Socket-Fehler an, also über denselben Weg wie ein RST, der nachweislich in < 1 ms den Halter fallen lässt (Probe 1); `TCP_USER_TIMEOUT` greift bei unbestätigten Daten (der Herzschlag liefert sie alle 5 s), Keepalive bei ruhenden Verbindungen (Warte-Aufruf) — beide zusammen decken beide Fälle.
- Nachteile: `TCP_USER_TIMEOUT` gibt es nur unter Linux (macOS und Windows haben eigene Optionen); die Zeit hängt am Kernel und lässt sich ohne Paketverlust nicht echt messen; ein Netzaussetzer länger als die Abrissgrenze kostet einen Strom (der Wächter im Browser verbindet neu, die Sitzung lebt weiter).

### Option B: Nur TCP-Keepalive
- Vorteile: auf allen Plattformen gleich (`SO_KEEPALIVE` mit Leerlauf, Abstand, Anzahl); bewährtes Mittel.
- Nachteile: Keepalive-Proben gehen nur bei einer **ruhenden** Verbindung hinaus; der Herzschlag hält den Strom nie lange ruhig, nach einem stillen Abriss liegen unbestätigte Daten an — dann gilt die Wiederholungsgrenze von TCP (Minuten), nicht Keepalive. Für den Hauptfall (Browser-Strom) wirkungslos.

### Option C: Quittung der Gegenstelle — Worker bzw. Agent bestätigt den Herzschlag (`POST /api/lebenszeichen`), ohne Quittung binnen 2 Herzschlägen beendet der Server den Strom
- Vorteile: unabhängig von Plattform und Kernel; die Regel „letzte Quittung älter als 2 H → Strom zu“ ist rein und mit gestellter Uhr vollständig prüfbar; erkennt auch einen hängenden Worker bei lebender Verbindung.
- Nachteile: eine neue Route mit Wirkung (Grenzschutz, Sitzung), ein Aufruf je Browser alle 5–15 s; Agenten, die den Strom mit `curl -N` lesen, können nicht quittieren und verlören ihren Halter — Sonderregel nötig; Halter wäre nicht mehr „die Verbindung lebt“, sondern „die Quittung ist frisch“ — zwei Wahrheiten, die auseinanderlaufen können (Verbindung offen, Halter tot).

### Option D: Der Server fragt selbst — bei jedem Herzschlag `TCP_INFO` des Sockets lesen (seit wann unbestätigt), reine Regel entscheidet, der Strom endet
- Vorteile: die Entscheidung ist eine reine Funktion über eine Naht (`unbestaetigt_seit`), mit Attrappe und gestellter Uhr prüfbar; Grenze frei wählbar.
- Nachteile: dieselbe Kernel-Information wie A, aber mit Linux-eigenem Strukturlesen, Deskriptor-Durchreichen von der Annahmeschleife in den Handler und mehr Code; keine Wirkung auf ruhende Warte-Aufrufe ohne Herzschlag.

### Option E: Nichts tun — der Browser-Wächter (40 s) verbindet von sich aus neu, der Server wartet auf TCP
- Vorteile: kein Code.
- Nachteile: der Abgerissene steht bei allen anderen rund 15 Minuten als `online` (Geisterzeile); Q4 und Vision verfehlt; der Name bleibt so lange belegt.

| # | Erkennt Strom | Erkennt Warte-Aufruf | Plattform | Prüfbar im Gate | Neuer Code | Schwäche |
|---|---|---|---|---|---|---|
| A | ≤ H + 2H | ≈ H + 2H | Linux voll, sonst nur Keepalive | Ableitung + Socket-Werte + RST-Kette | klein | echte Zeit nur als Messwert |
| B | nein (Minuten) | ja | alle | Socket-Werte | klein | Hauptfall offen |
| C | ≤ 2H | nein ohne Sonderweg | alle | voll (gestellte Uhr) | mittel, neue Route | zwei Wahrheiten, `curl`-Leser |
| D | ≤ H + Grenze | nein | Linux | voll (Attrappe) | mittel | Deskriptor-Durchreichen |
| E | ~15 min | ~15 min | alle | — | keiner | Vision verfehlt |

## Entscheidung

**Gewählt (vorgeschlagen): Option A**

Es überwiegt, dass eine einzige Stelle — die Annahmeschleife, durch die jede Verbindung geht — den stillen Abriss für Ströme und Warte-Aufrufe zugleich erkennbar macht, ohne Protokoll, Route oder Mitwirkung der Gegenstelle, und dass der Rest der Kette (Socket-Fehler → Halter fällt → `offline` nach dem Nachlauf) schon gebaut und geprobt ist. In Kauf genommen: unter macOS/Windows bleibt es bei Keepalive (Hauptfall dort nicht schneller als TCP), und die echte Erkennungszeit ist nur ein Messwert, kein Gate.

Regeln:

1. **Herzschlag `H` = 5 s** (statt 15 s; Testknopf `LOCALCHATA_HERZSCHLAG_MS` bleibt). Er erzeugt alle `H` unbestätigte Daten — erst damit läuft die Abrissgrenze. Kosten: rund 30 Byte je Browser alle 5 s.
2. **Abrissgrenze = 2 × H** (10 s), abgeleitet, kein eigener Knopf: `TCP_USER_TIMEOUT` = 2H; `SO_KEEPALIVE` an, `TCP_KEEPIDLE` = H, `TCP_KEEPINTVL` = H, `TCP_KEEPCNT` = 2 (Sekunden, aufgerundet, mindestens 1). Eine reine Funktion `pflege_aus(herzschlag) -> Verbindungspflege` bildet die Werte.
3. **Naht**: `ausliefern` bekommt die Pflege als Parameter (Funktion oder kleiner Trait mit der Kernel-Fassung und einer aufzeichnenden Testfassung) und ruft sie **je zugelassener Verbindung einmal vor dem Bedienen**. Die Kernel-Fassung setzt die Optionen über `rustix` (Feature `net`, `rustix::net::sockopt`; `rustix` ist schon direkte Abhängigkeit); `TCP_USER_TIMEOUT` nur unter Linux, Keepalive überall.
4. **Fehlerpfad**: scheitert das Setzen, wird die Verbindung trotzdem bedient (ohne Pflege wie heute); der erste Fehler je Programmlauf erscheint einmal auf stderr mit Option und Fehler — nie je Verbindung (stderr-Flut, ADR-0010).
5. **Kette danach unverändert**: Kernel bricht ab → hyper beendet die Antwort → `Halter` fällt (Lebenszeichen = Erkennungszeitpunkt) → `offline` nach dem Nachlauf (1 s) beim nächsten Takt (≤ 250 ms) → `weg` nach der Gnadenfrist ab dem Erkennungszeitpunkt ([ADR-0016](0016-anwesenheitsstatus-aus-haltern-mit-nachlauf-liste-als-stand-plus-wechsel.md)).
6. **Wächter im Browser = 3 × H** (15 s ohne Byte → neu verbinden, statt 40 s): nach dem Aufwachen eines Rechners steht der eigene Strom nach ≤ 15 s + Prüfabstand wieder, solange die Sitzung lebt.

**Rechnung „sofort offline“ beim stillen Abriss** (Zweck: die anderen sehen deutlich vor dem „weg“, dass der Teilnehmer nicht mehr antwortet — „sofort“ heißt hier: ein Bruchteil der Minute bis „weg“; Startpunkt: der Moment, ab dem die Gegenstelle keine Pakete mehr erreicht):

| Glied | Mittel | schlechtester Fall |
|---|---|---|
| bis zum nächsten Herzschlag (unbestätigte Daten liegen an) | H/2 = 2,5 s | H = 5 s |
| Abrissgrenze (`TCP_USER_TIMEOUT`) | 2H = 10 s | 10 s |
| Socket-Fehler → Halter fällt | < 1 ms (Probe 1) | < 1 ms |
| Nachlauf | 1 s | 1 s |
| Takt | 0,125 s | 0,25 s |
| **Summe** | **≈ 13,6 s** | **16,25 s** |

Grenze für den Messwert: **22 s** (≥ 30 % über 16,25 s). „Weg“ folgt 60 s nach dem Erkennen, also spätestens etwa 76 s nach dem Abriss — „etwa eine Minute“ der Vision, gemessen vom letzten Lebenszeichen, das der Server kennt.

## Konsequenzen

**Positiv:**
- Ein schlafender Laptop steht nach gut 15 s als offline statt nach Minuten; sein Name wird nach etwa einer Minute frei.
- Dieselbe Pflege deckt Warte-Aufrufe der Agenten (I0016) ab, ohne dass `warten` etwas senden muss.
- Kein neues Protokoll; Browser und Agenten bleiben unverändert (außer der Wächter-Konstante).

**Negativ / Inkauf zu nehmen:**
- Ein Netzaussetzer über 10 s bei laufendem Herzschlag beendet den Strom; der Wächter verbindet neu, die Sitzung lebt (Gnadenfrist), für andere erscheint ein kurzer Wechsel offline → online.
- Unter macOS/Windows erkennt nur Keepalive ruhende Verbindungen; ein Strom mit Herzschlag fällt dort erst nach der Wiederholungsgrenze des Systems. Projektumgebung ist Linux (`AI-Team/umgebung.md`); eine andere Plattform braucht eine neue Entscheidung.
- Fünfmal so viele Herzschläge (vernachlässigbar bei ≤ 512 Sitzungen).

**Pre-Mortem (ein Monat nach dem Start gescheitert, weil …):**
1. … Menschen im WLAN ständig kurz offline erscheinen, weil Aussetzer länger als 10 s häufig sind. Früh-Indikator: Wechsel offline → online derselben Sitzung binnen 20 s. Gegenmaßnahme: Abrissgrenze hängt an einer Stelle (2H) — anheben ohne Umbau; Messwert aus dem Entwicklertest vorher kennen.
2. … die Pflege still nicht greift (Option falsch gesetzt, Plattform), und niemand merkt es, weil kein Gate echten Paketverlust erzeugt. Früh-Indikator: Messwert im Entwicklertest über 22 s. Gegenmaßnahme: Unit-Test liest die Optionen am echten Socket zurück; Mutationsprobe: Aufruf der Pflege in `ausliefern` entfernen → der aufzeichnende Test wird rot.
3. … Agenten mit langen Warte-Aufrufen hinter einem NAT oder einer Firewall abbrechen, weil Keepalive dort anders wirkt. Früh-Indikator: Warte-Aufrufe enden vor `bis` mit Abbruch. Gegenmaßnahme: bei I0016 eigens messen; der Agent ruft ohnehin erneut auf.

**Folgeentscheidungen / Aufgaben:**
- [ ] Erster Test (Unit, rein): `pflege_aus(5 s)` → `{user_timeout: 10 000 ms, keepidle: 5 s, keepintvl: 5 s, keepcnt: 2}`; `pflege_aus(500 ms)` → `{1 000 ms, 1 s, 1 s, 2}` (aufgerundet, mindestens 1 s).
- [ ] Erster Test (Unit, Linux, echter Loopback-Socket, kein Netzabriss): Kernel-Pflege auf einem angenommenen Socket → `getsockopt` liest genau diese Werte zurück.
- [ ] Erster Test (Unit, `ausliefern` mit aufzeichnender Pflege): zwei Anfragen auf zwei Verbindungen → genau zwei gepflegte Verbindungen; eine über der Verbindungsgrenze abgewiesene → nicht gepflegt.
- [ ] Erster Test (Integration, echtes Programm): Beobachter `beta` hält einen Strom; `alpha` öffnet einen Strom über einen rohen Socket und bricht hart ab (RST, `SO_LINGER 0`) → `beta` erhält `alpha` `offline`; Startpunkt: Rückkehr von `close()`; erwartet ≈ 1,25 s (Nachlauf + Takt), Grenze 2 s (Q4). Dazu exakt über die Serverzeiten: `weg_um − seit = Gnadenfrist − 1 s` ([ADR-0019](0019-restzeit-weg-um-in-serverzeit-und-statusformen.md)).
- [ ] Gesundheit nennt `herzschlag_ms` und `abrissgrenze_ms` (Betrieb sieht die wirksamen Werte).
- [ ] **Messwert, kein Gate**: echter stiller Abriss — entweder im Netz-Namespace mit Rechten (`sudo unshare -n`, Programm und Client darin, nach dem ersten Ereignis `nft`-Regel „drop“ für den Client-Port) oder im Entwicklertest mit zweitem Gerät (WLAN aus bzw. Deckel zu, Stoppuhr am Beobachter). Ergebnis (min/Median/max, Zahl der Läufe) in diese ADR unter „Erprobt durch“; ≤ 22 s → Status `Akzeptiert`, ADR-0007 ebenso; darüber → neue Entscheidung (Option C oder D).
- [ ] ADR-0007 und ADR-0015 nennen Herzschlag 15 s und Wächter 40 s — Ergänzungen verweisen hierher.

## Quellen / Bezuege

- [A00003 Start](../Architektur/A00003-start.md) (Annahmeschleife, primär), [A00011 Live-Kanal](../Architektur/A00011-live-kanal.md), [A00010 Anwesenheit](../Architektur/A00010-anwesenheit.md), [A00005 Weboberfläche](../Architektur/A00005-weboberflaeche.md); Aspekte [X00007](../Architektur/Aspekte/X00007-zeit.md), [X00009](../Architektur/Aspekte/X00009-testbarkeit.md)
- ADR-0007, ADR-0010, ADR-0015, ADR-0016
- Linux `tcp(7)` (Abschnitte `TCP_USER_TIMEOUT`, `TCP_KEEPIDLE`, `TCP_KEEPINTVL`, `TCP_KEEPCNT`): unbestätigte Daten länger als die Grenze → Verbindung wird geschlossen, die Anwendung erhält `ETIMEDOUT`; RFC 5482 (TCP User Timeout Option); `/proc/sys/net/ipv4/tcp_retries2`
- `AI-Team/planer.md`, `AI-Team/entwickler.md` (Gelerntes #4, Zeitkriterien); `AI-Team/logbuch.md` (#4 Retrospektive)

## Ergänzungen

- **2026-10-28 (Befundkarte #23) — die Abrissgrenze ist auch die Sendefrist ([ADR-0039](0039-grosse-antworten-byte-budget-je-block-werkbank-und-abrissgrenze-als-sendefrist.md)):** Probe (Loopback, Kernel 6.14): ein Server-Socket mit `TCP_USER_TIMEOUT`, dessen Gegenstelle nie liest (Nullfenster, Timer `persist`), meldet `ETIMEDOUT` nach Grenze + ≈ 0,3 s (2 s → 2,29–2,32 s; 10 s → 10,27–10,30 s); gegen das echte Programm (Release, 9d7d4bb) schließen 64 nicht lesende Verbindungen server-seitig nach 10–12 s. Damit deckt Option A neben dem stillen Abriss auch langsame Leser ab, ohne weitere Mechanik (ADR-0039 Regel 6). Der Status bleibt `Vorgeschlagen`: der Messwert „echter stiller Abriss“ fehlt weiter; der Nullfenster-Fall ist geprobt und bekommt ein eigenes Gate (ADR-0039, K5 mit `LOCALCHATA_HERZSCHLAG_MS=2000`, Spanne 3,6–5,6 s).
