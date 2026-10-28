# ADR-0010: Eigene Annahmeschleife mit hyper http1 statt `axum::serve`

**Status**: Akzeptiert
**Datum**: 2026-10-27
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #2 I0002 — Nachzug aus Karte #1); gebaut vom `entwickler` als Nacharbeit des Security-Reviews
**Kontext-Anforderung**: R00001 (B0017 Verbindungen begrenzen)
**Erprobt durch**: Commit 6812820 (`src/start/ausliefern.rs`, `src/start/verbindungen.rs`), Angriffsmatrix Zeilen 1–6 in `tests/verfuegbarkeit.rs` über das echte Programm, Mutationsprobe je Wächter (`Dokumentation/Gelerntes.md`, Zeile 2026-10-27 Slowloris)

## Kontext

ADR-0001 wählte axum 0.8 auf tokio; stillschweigend mitgedacht war `axum::serve` als Auslieferung. Das Security-Review zu I0001 fand eine Slowloris-Lücke (CWE-400): `axum::serve` setzt keinen hyper-Timer, kennt also keine Kopf-Zeitgrenze, keine Verbindungsgrenze und kein Ende nach Frist (`Dokumentation/Gelerntes.md`, 2026-10-27). Die Instanz lauscht standardmäßig auf `0.0.0.0` ohne Anmeldung (Vision, Leitplanken) — jeder Rechner im LAN kann Verbindungen offen halten. Zugleich braucht der Live-Kanal lange Antworten (SSE, Long-Poll, ADR-0007), die keine Zeitgrenze abschneiden darf, und die Identität braucht die Peer-Adresse (ADR-0008).

## Optionen

### Option A: `axum::serve` mit `with_graceful_shutdown`, Grenzen über tower-Schichten
- Vorteile: kürzester Code; `ConnectInfo` über `into_make_service_with_connect_info` frei Haus; Testserver und Programm gleich.
- Nachteile: tower-`TimeoutLayer` begrenzt die Bearbeitung, nicht das Lesen des Kopfes — Slowloris bleibt offen; keine Verbindungsgrenze je Gegenstelle; das geordnete Ende wartet auf jeden offenen Strom ohne Frist.

### Option B: Eigene Annahmeschleife: `TcpListener::accept` → Platz zählen → je Verbindung `hyper::server::conn::http1` mit `TokioTimer` und `header_read_timeout`, Ende über `watch` mit Frist
- Vorteile: Kopf-Zeitgrenze, die zugleich Leerlaufgrenze zwischen Requests ist, aber laufende Antworten nicht abschneidet; Grenze je Gegenstelle und gesamt vor jeder Bearbeitung; Ende nach fester Frist; alles im echten Programm geprüft.
- Nachteile: Mechanik, die axum sonst mitbringt, liegt im Projekt (`ConnectInfo`, geordnetes Ende); nur HTTP/1.1; zwei neue direkte Abhängigkeiten (`hyper`, `hyper-util`) plus `rustix` für die Deskriptorgrenze.

### Option C: Wie B, aber `hyper_util::server::conn::auto::Builder` (HTTP/1.1 und HTTP/2) mit `GracefulShutdown`
- Vorteile: HTTP/2 für Browser (mehr als sechs Verbindungen je Rechner ohne Mehrfach-Verbindung); fertiges geordnetes Ende.
- Nachteile: größere Angriffsfläche (HTTP/2-Rapid-Reset-Klasse, Strom-Grenzen zusätzlich zu begrenzen); HTTP/2 ohne TLS (h2c) nutzt kein Browser — der Gewinn fiele im LAN ohne TLS weg.

### Option D: Vorgeschalteter Reverse-Proxy (nginx, Caddy) übernimmt die Grenzen
- Vorteile: bewährte Grenzen ohne eigenen Code.
- Nachteile: zweiter Prozess — verletzt die Leitplanke „ein einziges Programm“; die Peer-Adresse käme nur noch über Header (ADR-0008 setzt „kein Proxy“ voraus).

## Entscheidung

**Gewählt: Option B**

Nur B schließt die Lücke im Programm selbst, ohne die langen Antworten des Live-Kanals zu gefährden; der Beleg ist die grüne Angriffsmatrix über das echte Programm samt Mutationsprobe. Festgelegt (Stand Commit 6812820):

- Kopf-Zeitgrenze 10 s (`KOPF_ZEITGRENZE`), gilt als Gesamtfrist je Kopf und läuft nach jeder Antwort neu an (Leerlaufgrenze); während einer laufenden Antwort läuft sie nicht.
- Höchstens 64 offene Verbindungen je Gegenstelle (IP-Adresse), gesamt `min(512, weiche Deskriptorgrenze / 2)`; über der Grenze wird die Verbindung ohne Antwort geschlossen.
- Beim Signal: Listener schließen, jeder Verbindung das Ende melden (`graceful_shutdown`), nach 5 s (`ENDE_FRIST`) abbrechen; Exit 0.
- Nur HTTP/1.1, keine Upgrades (`serve_connection` ohne `with_upgrades`) — WebSocket ist damit ausgeschlossen, im Einklang mit ADR-0007 Option A.

In Kauf genommen: die Annahmeschleife ist Projektcode, den jede neue Anforderung an die Verbindung (Herkunft, Ströme, Ende) kennen muss — und `ConnectInfo` fehlt, bis die Schleife es selbst setzt.

## Konsequenzen

**Positiv:**
- Ein einzelner Rechner kann die Instanz nicht mehr durch offene oder halbe Verbindungen lahmlegen; das ist über das echte Programm belegt.
- Das geordnete Ende ist begrenzt: spätestens 5 s nach dem Signal ist der Prozess fort, auch bei offenen Strömen.

**Negativ / Inkauf zu nehmen:**
- **`ConnectInfo<SocketAddr>` gibt es nicht**: axum setzt es nur in `axum::serve`. Die Herkunft (ADR-0008) muss die Schleife selbst je Request als Extension setzen; die Gegenstelle ist bei `accept` bekannt.
- **Ströme belegen Plätze**: jeder offene SSE-Strom und jeder wartende Long-Poll hält einen der 64 Plätze seiner Gegenstelle. Alle Agenten und Tabs eines Rechners teilen sich diese 64 — bei vielen Agenten auf einem Rechner (z. B. mehrere Factories auf `127.0.0.1`) eine echte Grenze.
- **Ströme enden beim Signal erst durch Abbruch**: `graceful_shutdown` wartet, bis die laufende Antwort fertig ist; ein SSE-Strom endet nie von selbst, ein Long-Poll erst nach `bis`. Ohne Gegenmaßnahme dauert jedes Ende mit offenem Tab volle 5 s.
- **Zwei Server-Stacks**: der In-Process-Testserver (`src/http/mod.rs`, `testserver`) fährt noch `axum::serve`; Tests darüber sehen weder Grenzen noch Zeitgrenze noch (nach dem Umbau) dieselbe Herkunft.
- Nur HTTP/1.1: ein Browser hält höchstens sechs Verbindungen je Rechner; ein SSE-Strom je Tab bleibt darunter.

**Folgeentscheidungen / Aufgaben:**
- [ ] Vor I0004: die Schleife setzt je Request `ConnectInfo<SocketAddr>` (IPv4-gemappte IPv6 einmal bei `accept` zu IPv4 normalisiert — dieselbe Adresse für Platzzählung und Herkunft); erster Test über das echte Programm von `127.0.0.2` aus.
- [ ] Testserver auf `ausliefern` umstellen (ein Stack), spätestens wenn ein In-Process-Test Herkunft, Ströme oder Grenzen berührt.
- [ ] Mit dem Live-Kanal (I0009/I0016): Ströme und Warte-Aufrufe hören das Ende-Signal und schließen selbst (SSE: Strom endet; Long-Poll: leere Antwort) — Ende ohne 5-s-Abbruch.
- [ ] Früh-Indikator: abgewiesene Verbindungen je Gegenstelle zählen und im Health-Check nennen, damit die 64er-Grenze auffällt, bevor Agenten scheitern.

## Quellen / Bezuege

- [A00002 Programm localchatA](../Architektur/A00002-programm-localchata.md) (primär), [A00003 Start](../Architektur/A00003-start.md), [A00004 HTTP-Schnittstelle](../Architektur/A00004-http-schnittstelle.md), [A00011 Live-Kanal](../Architektur/A00011-live-kanal.md); Aspekte [X00008](../Architektur/Aspekte/X00008-transaktionen-und-nebenlaeufigkeit.md), [X00009](../Architektur/Aspekte/X00009-testbarkeit.md)
- ADR-0001 (ergänzt), ADR-0007, ADR-0008, ADR-0009
- `src/start/ausliefern.rs`, `src/start/verbindungen.rs`, `tests/verfuegbarkeit.rs`; `Dokumentation/Gelerntes.md` (2026-10-27); `AI-Team/gelerntes.md` (#1)

## Ergänzungen

- **2026-10-28 (Karte #8 I0009, Nachzug aus Befundkarte #21 / R00010) — Körpergrenze je Route, große Körper nie auf dem Worker parsen:** Anlass: die eine Grenze von 2 MiB für alle JSON-Routen ließ ohne Sitzung eine Flut großer Körper zu, deren Parse auf dem async-Worker alle anderen Routen bremste (R00010: Gesundheit p90 0,23–0,28 s, RSS ≈ 460 MB; derselbe Körper als 413 ohne Parse: p90 0,018–0,038 s). Die Grenzen dieser ADR (Verbindungen, Kopfzeit) begrenzen Verbindungen, nicht die Arbeit je Anfrage. Regeln:
  1. **Jede JSON-Route nennt ihre Körpergrenze mit Herleitung** — neben dem Handler als Konstante mit einem Satz: größter gültiger Körper in der längsten JSON-Schreibweise (jedes Zeichen als `\uXXXX`, Zeichen außerhalb der BMP als Ersatzpaar) plus mindestens 30 %, gerundet. Eine allgemeine Vorgabe gibt es nicht mehr; eine Route ohne genannte Grenze ist ein Review-Befund (Durchsetzung: Review-Punkt für `code-reviewer`/`security-reviewer`, `json_lesen` verlangt die Grenze als Argument). Über der Grenze 413 `{fehler}` mit „höchstens n Bytes“, ohne Parse; der Rest wird wie in `koerper.rs` begrenzt gelesen und verworfen.
  2. **Große Körper nur außerhalb des Workers**: liegt die Grenze einer Route über **16 KiB**, wird der Körper in `spawn_blocking` geparst, und zwar unter einer gemeinsamen Semaphore (`tokio::sync::Semaphore`, Erlaubnisse = Zahl der Kerne, mindestens 2), die vor `spawn_blocking` erworben wird. Unter 16 KiB parst der Handler direkt (Kosten unter einer Millisekunde, R00010: 4 096 Bytes für `beitreten` und `channels`).
  3. **Arbeit vor dem Parse kostet**: eine Route mit großer Grenze, die eine Sitzung verlangt, bucht vor dem Lesen des Körpers am Schreibkonto ([ADR-0036](0036-schreibkonto-token-eimer-je-kennung-und-je-herkunft.md) Regel 3) — so begrenzt das Konto auch Fluten ungültiger großer Körper, nicht nur gültige Nachrichten.
  - Stand der Grenzen: `beitreten`, `channels` 4 096 Bytes (R00010); `PUT …/frist` nennt seine beim Bau (R00008); `POST /api/channels/{name}/nachrichten` **131 072 Bytes** ([ADR-0034](0034-nachricht-senden-post-nachrichten-text-hoechstens-16-kib-steuerzeichen-abweisen-wiederholschluessel.md) Regel 2) — die erste Route über 16 KiB, also mit Parse außerhalb des Workers.
  - In Kauf genommen: unter einer Flut großer Körper warten echte Sender in der Schlange der Semaphore (Option C aus R00010); die Vorab-Buchung am Konto hält diese Schlange je Herkunft kurz.
  - Erster Test (Integration, echtes Programm): Senderoute 131 072 Bytes gültig (Text in Leerzeichen-Auffüllung) → nicht 413; 131 073 → 413 ohne Parse (ungültiges JSON → 413, nicht 400). Messwert im Release-Build wie R00010: 24 Schleifen à 131 072 Bytes gültig formatiertes, aber leeres `text` mit Sitzung → Gesundheit p90 ≤ 0,15 s.
