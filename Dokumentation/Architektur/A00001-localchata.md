---
id: A00001
name: localchatA
code: localchatA
ebene: System
eltern:
repo: localchatA
status: Geplant
herkunft: dialog
quellen:
  - Cargo.toml
  - src/
  - web/
verwandt: []
domaene: []
---

# localchatA

## Verantwortung

Bringt Menschen und KI-Agenten im LAN in Channels und Direktgesprächen zusammen, deren Verlauf als lesbarer Ordner aus täglich rotierenden Logs besteht.

## Schnittstellen

- **Eingehend — Mensch im Browser:** HTTP (Seite, JSON-API, Server-Sent Events) auf einem Port im LAN, ohne Anmeldung; Identität `name@herkunft` über ein Sitzungs-Token ([ADR-0008](../ADRs/0008-identitaet-peer-adresse-und-sitzungs-token.md)).
- **Eingehend — Agent:** dieselbe JSON-API; Warten auf Neues als Long-Poll `GET /api/warten` ([ADR-0007](../ADRs/0007-live-kanal-sse-fuer-browser-long-poll-fuer-agenten.md)).
- **Ausgehend — Dateisystem:** ein Ordner, Aufruf `localchatA <ordner>`; Layout und Format in [ADR-0003](../ADRs/0003-ablage-layout-im-ordner.md) und [ADR-0004](../ADRs/0004-logformat-kopfzeile-und-eingerueckter-text.md). Lesen darf jeder, schreiben nur das Programm ([ADR-0005](../ADRs/0005-programm-als-einziger-schreiber-und-reihenfolge.md)).
- **Ausgehend — Browser des Startenden:** öffnet beim Start die Seite (abschaltbar), wie kanbanD.

## Abhaengigkeiten

- [Programm localchatA](A00002-programm-localchata.md) (geplant) — einziger Container.
- Extern: Rust-Toolchain (Bau), Node.js mit Playwright (nur E2E-Tests).
- Keine Kopplung an kanbanD (Vision, „Ausdrücklich nicht Ziel“); kanbanD ist nur Vorbild (Quelle Q04).

## Entscheidungen

- [ADR-0001: Ein Programm in Rust mit axum und tokio, Bibliothek plus Binärprogramm](../ADRs/0001-ein-programm-rust-axum-tokio.md) — Akzeptiert, 2026-10-27
- [ADR-0002: Oberfläche ohne Build-Schritt in das Programm eingebettet](../ADRs/0002-oberflaeche-ohne-build-schritt-eingebettet.md) — Akzeptiert, 2026-10-27
- [ADR-0003: Ablage-Layout im Ordner](../ADRs/0003-ablage-layout-im-ordner.md) — Akzeptiert, 2026-10-27
- [ADR-0004: Logformat — Kopfzeile und eingerückter Text](../ADRs/0004-logformat-kopfzeile-und-eingerueckter-text.md) — Akzeptiert, 2026-10-27
- [ADR-0005: Das Programm ist einziger Schreiber; Reihenfolge über Sperre je Gespräch und monotone Zeit](../ADRs/0005-programm-als-einziger-schreiber-und-reihenfolge.md) — Vorgeschlagen, 2026-10-27
- [ADR-0006: Verlaufsindex im Speicher über dem Rohtext der Tagesdateien](../ADRs/0006-verlaufsindex-im-speicher.md) — Akzeptiert, 2026-10-27
- [ADR-0007: Live-Kanal — SSE für Browser, Long-Poll für Agenten, ein Ereignisbus](../ADRs/0007-live-kanal-sse-fuer-browser-long-poll-fuer-agenten.md) — Vorgeschlagen, 2026-10-27
- [ADR-0008: Identität — Herkunft aus der Peer-Adresse, Nachweis über Sitzungs-Token](../ADRs/0008-identitaet-peer-adresse-und-sitzungs-token.md) — Vorgeschlagen, 2026-10-27
- [ADR-0009: Teststrategie, Gate und Health-Check](../ADRs/0009-teststrategie-gate-und-health-check.md) — Akzeptiert, 2026-10-27
- [ADR-0010: Eigene Annahmeschleife mit hyper http1 statt `axum::serve`](../ADRs/0010-eigene-annahmeschleife-hyper-http1.md) — Akzeptiert, 2026-10-27
- [ADR-0011: Anhängen mit Rückbau bei Fehler und Zeilenende-Wächter](../ADRs/0011-anhaengen-mit-rueckbau-und-zeilenende-waechter.md) — Akzeptiert, 2026-10-27

## Offene Fragen

- Die Vision sagt, der Agent schreibe „über die API oder an den Dateien“. Ist Schreiben an den Dateien (außerhalb des Programms) ein unterstützter Weg, oder sind die Dateien für Dritte nur lesbar? Empfehlung: nur lesbar ([ADR-0005](../ADRs/0005-programm-als-einziger-schreiber-und-reihenfolge.md)) — Richtungsfrage an den Betreiber.

## Notizen / Quellen

Entworfen am 2026-10-27 per `/architektur brainstorming --still` durch die Rolle `architekt` (Karte #1 I0001 Instanz starten). Wurzel ist das `System` — es gibt nur ein Repository, eine `Loesung`-Ebene entfällt (`_modell.md`, „Ebenen“). `herkunft: dialog` heißt hier: aus Vision R00000 und WBS abgeleitete, im Auftrag bestätigte Absicht; Ungeprüftes steht als offene Frage an den Bausteinen.

### Qualitätsziele mit Szenario

| # | Ziel | Szenario (Auslöser → erwartete Antwort, Maß) | Quelle |
|---|---|---|---|
| Q1 | Antwortzeit Verlauf | Ein Mensch wählt einen Channel, während 100.000 Nachrichten in der Frist liegen → die neuesten 100 stehen da; Server-Antwort < 50 ms, jeder ältere Block zu 100 ebenso | Vision „Schnell, auch bei großem Verlauf“; Designsystem „Leistung als Gestaltung“ |
| Q2 | Antwortzeit Suche | Suche über alle sichtbaren Nachrichten bei 100.000 → die ersten 50 Treffer (neueste zuerst) in < 100 ms Server-Zeit, auch wenn es keinen Treffer gibt | Vision; WBS I0011 |
| Q3 | Live | Eine Nachricht wird über API oder Browser gesendet → sie erscheint in jeder offenen Sicht und bei jedem wartenden Agenten, für den sie bestimmt ist, in < 1 s im LAN | Vision „Live überall“, „Agenten können warten“; WBS I0009, I0016 |
| Q4 | Anwesenheit | Ein Tab wird geschlossen → der Teilnehmer steht in < 2 s als offline; ohne Rückkehr nach ~60 s ist er aus der Liste und der Name frei | Vision „Wer da ist, ist sichtbar“; WBS I0006 |
| Q5 | Lesbarkeit ohne Programm | Jemand öffnet ein Tageslog mit `less` oder `grep` → Absender, Zeit und Markdown-Text jeder Nachricht sind ohne Werkzeug lesbar; keine Nachricht kann eine andere zerreißen | Vision „Ein Chat, der ein Ordner ist“ |
| Q6 | Start | `localchatA <ordner>` mit 100.000 Nachrichten in der Frist → bereit (Health-Check grün) in < 2 s | abgeleitet aus Q1; Probe in ADR-0006 |
| Q7 | Testbarkeit | Jede Komponente ist ohne die anderen prüfbar; das ganze Programm startet in Tests auf Port 0 mit einem Wegwerf-Ordner | Auftrag; kanbanD `tests/programm.rs`, `e2e/tests/hilfen.js` |

### Architekturbedeutsame Anforderungen

Ablage und Format (schwer umkehrbar: Bestand auf Platte), Schreibreihenfolge, Index und Startzeit (Leistungsziel prägt I0008, I0011), Live-Kanal und Anwesenheit (prägt I0005, I0006, I0009, I0015, I0016), Identität (prägt alle Dialogs), Teststrategie. Alles Weitere ist Mikroarchitektur im Baustein.

### Optionen-Tableau des Schnitts

| # | Variante | Testbarkeit (Q7) | Leistung (Q1, Q2) | Betrieb | Reversibilität | Schwäche |
|---|---|---|---|---|---|---|
| A | **Ein Programm, eine Bibliothek mit fachlichen Modulen** (Start, HTTP, Gespräche mit Logformat/Logablage/Verlaufsindex, Anwesenheit, Live-Kanal) — gewählt | jedes Modul einzeln (`cargo test`), Ganzes über das Binärprogramm | Index im selben Prozess wie der Schreiber | ein Artefakt, ein Start | Module lassen sich später herauslösen | Disziplin an den Modulgrenzen hängt an Review, nicht am Compiler |
| B | Cargo-Workspace mit je einer Crate je Komponente | Grenzen vom Compiler erzwungen | gleich | ein Artefakt | gut | mehr Bauzeit, Schnittstellen erstarren, bevor die Fachsprache steht |
| C | Zwei Prozesse: Schreib-/Index-Dienst und Web-Front | getrennt startbar | zusätzlicher Netzsprung je Anfrage | zwei Starts, Leitplanke „ein Programm“ verletzt | teuer | widerspricht der Vision |
| D | Ein Modul je technischer Schicht (handler, service, storage) | mittel | gleich | ein Artefakt | mittel | eine Interaction fällt quer durch alle Schichten (Schnitt nach Schicht) |

Verworfen: B (Grenzen zu früh hart, der Gewinn kommt erst mit stabiler Sprache — später möglich), C (Leitplanke), D (Schnitt nach Schicht).

### Pre-Mortem (Betrieb, ein Monat nach dem Start)

1. **Der Speicher läuft voll**, weil Agenten viel mehr schreiben als erwartet (Million statt hunderttausend). Früh-Indikator: Startzeile und `/api/gesundheit` nennen Nachrichtenzahl, Ladezeit und Index-Größe. Gegenmaßnahme im Entwurf: Index hinter einer schmalen Schnittstelle ([A00009](A00009-verlaufsindex.md)), Ausweg Offsets statt Rohtext ohne Änderung der Aufrufer.
2. **Zwei Instanzen laufen auf demselben Ordner** und verschränken ihre Logs. Früh-Indikator: zweiter Start. Gegenmaßnahme: Sperrdatei mit `File::try_lock`, zweiter Start endet mit Fehler ([ADR-0005](../ADRs/0005-programm-als-einziger-schreiber-und-reihenfolge.md)).
3. **Teilnehmer „hängen“ online oder verlieren ihren Namen mitten in der Arbeit**, weil stille Verbindungsabrisse (WLAN) oder lange Agenten-Arbeit falsch gedeutet werden. Früh-Indikator: Anwesenheitsliste weicht vom Bild der Menschen ab. Gegenmaßnahme: Herzschlag im SSE-Strom, Gnadenfrist je Sitzung ([ADR-0008](../ADRs/0008-identitaet-peer-adresse-und-sitzungs-token.md)).
