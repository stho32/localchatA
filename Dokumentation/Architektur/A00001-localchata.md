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

- **Eingehend — Mensch im Browser:** HTTP (Seite, JSON-API, Server-Sent Events) auf einem Port im LAN, ohne Anmeldung; Identität `name@herkunft` über ein Sitzungs-Token ([ADR-0012](../ADRs/0012-identitaet-und-sitzung.md)); Host-, Origin-Prüfung und Sicherheitsheader an der Grenze ([ADR-0014](../ADRs/0014-grenzschutz-host-origin-sicherheitsheader.md)).
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
- [ADR-0008: Identität — Herkunft aus der Peer-Adresse, Nachweis über Sitzungs-Token](../ADRs/0008-identitaet-peer-adresse-und-sitzungs-token.md) — Abgelöst durch ADR-0012, 2026-10-27
- [ADR-0009: Teststrategie, Gate und Health-Check](../ADRs/0009-teststrategie-gate-und-health-check.md) — Akzeptiert, 2026-10-27
- [ADR-0010: Eigene Annahmeschleife mit hyper http1 statt `axum::serve`](../ADRs/0010-eigene-annahmeschleife-hyper-http1.md) — Akzeptiert, 2026-10-27
- [ADR-0011: Anhängen mit Rückbau bei Fehler und Zeilenende-Wächter](../ADRs/0011-anhaengen-mit-rueckbau-und-zeilenende-waechter.md) — Akzeptiert, 2026-10-27
- [ADR-0012: Identität und Sitzung — Herkunft aus der Gegenstelle, Sitzungs-Token im Speicher, eine Sitzung je Browser](../ADRs/0012-identitaet-und-sitzung.md) — Akzeptiert, 2026-10-27
- [ADR-0013: Namensregel als Erlaubnisliste — `[a-z0-9-]`, 1 bis 32 Zeichen, abweisen statt umformen](../ADRs/0013-namensregel-als-erlaubnisliste.md) — Akzeptiert, 2026-10-27
- [ADR-0014: Grenzschutz im LAN — Host-Erlaubnisliste, Origin-Prüfung für Schreibendes, Sicherheitsheader auf jeder Antwort](../ADRs/0014-grenzschutz-host-origin-sicherheitsheader.md) — Akzeptiert, 2026-10-27
- [ADR-0015: Ein Ereignisstrom je Browser — SharedWorker mit `fetch`-Strom und Sitzungs-Header](../ADRs/0015-ein-ereignisstrom-je-browser-sharedworker-mit-sitzungs-header.md) — Akzeptiert, 2026-10-28
- [ADR-0016: Anwesenheitsstatus aus Haltern mit Nachlauf, Teilnehmerliste als Stand plus Wechsel](../ADRs/0016-anwesenheitsstatus-aus-haltern-mit-nachlauf-liste-als-stand-plus-wechsel.md) — Akzeptiert, 2026-10-28
- [ADR-0017: Stiller Abriss — der Kernel erkennt tote Verbindungen (Abrissgrenze und Keepalive je Verbindung), Herzschlag 5 s](../ADRs/0017-stiller-abriss-kernel-erkennt-tote-verbindungen.md) — Vorgeschlagen, 2026-10-28
- [ADR-0018: `POST /api/verlassen` — sofort `weg`, Name frei; die Oberfläche ruft es nie beim Schließen](../ADRs/0018-verlassen-sofort-weg-nie-beim-schliessen.md) — Akzeptiert, 2026-10-28
- [ADR-0019: Restzeit `weg_um` in Serverzeit mit Uhrenabgleich über `jetzt`; gestrichelt heißt „gleich weg“](../ADRs/0019-restzeit-weg-um-in-serverzeit-und-statusformen.md) — Akzeptiert, 2026-10-28
- [ADR-0026: Verlauf im Browser als Fenster aus höchstens fünf Blöcken — Nachladen an beiden Enden, Anker von Hand; was „sofort“ und „flüssig“ messbar heißt](../ADRs/0026-verlauf-im-browser-als-fenster-aus-hoechstens-fuenf-bloecken.md) — Akzeptiert, 2026-10-28
- [ADR-0040: Betriebsprotokoll — eine Datei je lokalem Tag unter `logs/`, eine Zeile je Eintrag; stderr spiegelt ab Warning](../ADRs/0040-betriebsprotokoll-datei-je-lokalem-tag-unter-logs-eine-zeile-je-eintrag.md) — Akzeptiert, 2026-10-28
- [ADR-0042: Stufen und Ereigniskatalog des Betriebsprotokolls — Error gegen Exception, 4xx nur als Debug oder Episode, Tagesbudget je Stufenklasse](../ADRs/0042-stufen-ereigniskatalog-episoden-und-tagesgrenze-des-betriebsprotokolls.md) — Akzeptiert, 2026-10-28
- [ADR-0039: Große Antworten — Byte-Budget je Block, schwere Arbeit nur auf der Werkbank, die Abrissgrenze ist die Sendefrist](../ADRs/0039-grosse-antworten-byte-budget-je-block-werkbank-und-abrissgrenze-als-sendefrist.md) — Akzeptiert, 2026-10-28

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

- **Ergänzung 2026-10-28 (Karte #4, I0005):** Live-Kanal und Anwesenheit sind für den Browser festgelegt: ein Ereignisstrom je Browser über einen `SharedWorker` mit Sitzungs-Header ([ADR-0015](../ADRs/0015-ein-ereignisstrom-je-browser-sharedworker-mit-sitzungs-header.md)) — Grund ist die geprobte 6-Verbindungen-Grenze von HTTP/1.1 je Browser, die bei einem Strom je Tab ab 6 Tabs die Instanz blockiert; Status aus Haltern mit 1 s Nachlauf, Liste als Stand plus Wechsel ([ADR-0016](../ADRs/0016-anwesenheitsstatus-aus-haltern-mit-nachlauf-liste-als-stand-plus-wechsel.md)). Q4 gilt weiter (< 2 s); erwartet ≤ 1,25 s plus Ende des Workers. Pre-Mortem 3 oben stützt sich seither auf ADR-0012 (statt der abgelösten ADR-0008) und ADR-0016; der stille Abriss bleibt offen bis I0006 ([ADR-0007](../ADRs/0007-live-kanal-sse-fuer-browser-long-poll-fuer-agenten.md), Ergänzung 2026-10-28).
- **Ergänzung 2026-10-28 (Karte #5, I0006):** Q4 bekommt ein zweites Szenario. **Q4b Stiller Abriss**: ein Teilnehmer verliert das Netz ohne Verbindungsende (WLAN weg, Deckel zu) → die anderen sehen ihn offline; erwartet ≈ 13,6 s, höchstens 16,25 s nach dem Abriss (Herzschlag 5 s + Abrissgrenze 10 s + Nachlauf 1 s + Takt), Grenze 22 s — **Messwert**, kein Gate, weil ein echter Abriss Rechte braucht ([ADR-0017](../ADRs/0017-stiller-abriss-kernel-erkennt-tote-verbindungen.md)); „weg“ 60 s nach dem Erkennen. **Q4c Verlassen**: `POST /api/verlassen` → Name sofort frei, die anderen sehen `weg` in < 1 s (wie Q3, [ADR-0018](../ADRs/0018-verlassen-sofort-weg-nie-beim-schliessen.md)). Pre-Mortem 3 stützt sich nun auf ADR-0017 statt auf den Herzschlag allein.
- **Ergänzung 2026-10-28 (Karte #7, I0008):** Q1 wird in drei Szenarien geteilt ([ADR-0026](../ADRs/0026-verlauf-im-browser-als-fenster-aus-hoechstens-fuenf-bloecken.md), Probe mit 100.000 Nachrichten). **Q1a Server** wie bisher: Block zu 100 < 50 ms (Gate L1; erwartet wenige ms). **Q1b Öffnen im Browser**: Klick auf `# name` → neueste Nachricht mit Absender und Zeit sichtbar; erwartet ≈ 45 ms, Gate < 500 ms (L2), Messwert Median ≤ 100 ms (M2). **Q1c Scrollen**: Hochscrollen über beliebig viele Nachrichten hält höchstens 500 im DOM (L3), nichts springt (L4, ≤ 1 px), nichts fehlt oder doppelt (L5); flüssig als Messwert: höchstens 2 % Frames über 50 ms, keine Aufgabe über 250 ms (M1; Probe 0 %). Der Verlauf ist ein Fenster aus höchstens fünf Blöcken, nachgeladen über die Nachrichtenzeit als Cursor ([ADR-0025](../ADRs/0025-verlauf-ueber-die-api-bloecke-mit-zeit-als-cursor.md)).
- **Ergänzung 2026-10-28 (Befundkarte #23, vor I0009):** Neues Qualitätsszenario **Q8 Langsame Leser**: ein Rechner im LAN öffnet 64 Verbindungen, holt je den größten Verlaufsblock und liest nicht → der Speicher wächst um höchstens 64 × 2 × Budget + 50 MB (≈ 84 MB je Herkunft), jede dieser Verbindungen ist server-seitig nach der Abrissgrenze (2H) zu, andere Routen antworten weiter. Quelle: Security-Review #7 (gemessen + 746 MB je Herkunft bei 200 × 16 KiB). Makroregel für jede große Antwort — Budget mit Herleitung, Listen nach Budget mit Fortsetzungs-Flag, Dateien gestreamt, schwere Arbeit auf der Werkbank, Sendefrist = Abrissgrenze: [ADR-0039](../ADRs/0039-grosse-antworten-byte-budget-je-block-werkbank-und-abrissgrenze-als-sendefrist.md) Regel 7; Durchsetzung über Review-Punkt und die Gates K1–K7.
- **Ergänzung 2026-10-28 (Vorlauf „Betriebsprotokoll“):** Neues Qualitätsszenario **Q9 Nachvollziehbarkeit**: im Betrieb scheitert etwas (Platte voll, Panik, Flut) → am nächsten Morgen findet der Betreiber in `logs/<tag>.log` mit `grep ' ERROR \| EXCEPTION '` was, wann (Zeit mit Versatz), bei wem (Kennung, Herkunft) und warum (Grund, Ort), ohne Code zu lesen; Programmfehler sind von Betriebsfehlern getrennt zählbar (`exceptions`, `errors` im Health-Check); kein Client kann das Protokoll über 32 MiB Information+Warning je Tag treiben, kein Token und kein Nachrichtentext steht darin; Protokolle älter als 14 Kalendertage sind weg. Quelle: Vorgabe des Betreibers (`AI-Team/umgebung.md`, „Logging-Konzept“); Entscheidungen [ADR-0040](../ADRs/0040-betriebsprotokoll-datei-je-lokalem-tag-unter-logs-eine-zeile-je-eintrag.md) bis [ADR-0043](../ADRs/0043-protokollstufe-einstellen-und-kennzahlen-im-health-check.md), Baustein [A00015](A00015-betriebsprotokoll.md); Prüfung durch den `observability-reviewer`.
- **Ergänzung 2026-10-28 (Lastprofil „Team-LAN“, Betreiber, `AI-Team/umgebung.md`; Auftrag des Directors):** Abgleich der gebauten bzw. entschiedenen Grenzen mit dem Profil — bis 20 Menschen und 50 Agenten gleichzeitig, Spitze 30 Nachrichten/s gesamt, dauerhaft 5/s, bis 100.000 Nachrichten je Channel, 50 Channels aktiv, Zustellung < 1 s, Öffnen < 500 ms, Suche < 1 s, normaler Rechner als Server. Annahme für den ungünstigen Fall: alle 50 Agenten laufen auf **einem** Rechner (eine Herkunft), dort sitzt auch ein Mensch mit Browser. Gerechnet, nicht gemessen; die Entscheidung über jede Änderung liegt beim Director (kein Code geändert).

  | Grenze (Quelle) | Bedarf im Profil | Urteil | Vorschlag |
  |---|---|---|---|
  | Verbindungen je Herkunft 64, gesamt `min(512, Deskriptoren/2)` (`src/start/verbindungen.rs:11–13`, ADR-0010) | 50 wartende Agenten (Long-Poll, I0016) + 50 kurze Sendeverbindungen in Spitzen + Browser (1 Strom + bis 6) ≈ 57 dauerhaft, ≈ 107 in Spitzen; gesamt 20 × 7 + 50 × 2 ≈ 240 | **unterschreitet** je Herkunft (≈ 107 > 64; abgewiesen wird heute **still**, ADR-0042 macht es zur Warning-Episode); gesamt passt (240 ≤ 512) | je Herkunft **160** (≈ 1,5 × Spitze); Folge für Q8: RSS-Zuwachs langsamer Leser je Herkunft ≤ 160 × 0,5 MiB + 50 MB ≈ 134 MB statt 84 MB (ADR-0039 Rechnung Q8) — neu messen (K4) |
  | Sitzungen je Herkunft 64, gesamt 512; verlassene zählen bis zum Ende der Gnadenfrist (60 s) mit (`src/anwesenheit/bestand.rs:19–20, 241`, ADR-0012, ADR-0018 Regel 2a) | 50 Agenten + 1 Mensch = 51; startet jeder Agent einmal neu mit neuem Namen, hält die alte Sitzung 60 s → bis 101 | **knapp / unterschreitet bei Neustarts** (51 ≤ 64, 101 > 64); gesamt passt (70 bzw. 140 ≤ 512) | je Herkunft **128**; zusätzlich für I0019 (Werkzeug `lc`): eine Sitzung je Agent wiederverwenden (Token-Datei), nie ein Beitreten je Aufruf — sonst reicht keine Grenze |
  | Schreibkonto je Kennung Vorrat 30, 1/s (ADR-0036) | dauerhaft 5/s über 70 Teilnehmer ≈ 0,07/s je Kennung; ein Agent im Zug 20–30 Nachrichten | **passt** | — |
  | Schreibkonto je Herkunft Vorrat 120, 8/s (ADR-0036) | dauerhaft 5/s aus einer Herkunft (Reserve 1,6); Spitze 30/s: der Vorrat trägt 120 / (30 − 8) ≈ 5,5 s, danach 8/s | **passt dauerhaft, Grenzfall in der Spitze** — dauert eine Spitze länger als ≈ 5 s und kommt sie aus einer Herkunft, wird gebremst, und der Mensch an diesem Rechner wartet mit (ADR-0036, in Kauf genommen) | Spitzendauer beim Betreiber erfragen; trägt sie 10 s: Nachschub je Herkunft **16/s**, Vorrat **300** (10 × (30 − 16) = 140 ≤ 300; Dauerlast-Obergrenze je Herkunft dann ≈ 64 KiB/s, ≈ 5,5 GB/Tag im schlimmsten Fall) |
  | Channels gesamt 256, je Herkunft 32, dauerhaft gezählt, kein Löschen (`src/gespraeche/verzeichnis.rs:58–60`, ADR-0021) | 50 aktive Channels; legen Agenten eines Rechners sie an, zählen alle auf dieselbe Herkunft | gesamt **passt** (Faktor 5, wächst aber ohne Löschen über Monate); je Herkunft **unterschreitet** (50 > 32) | je Herkunft **128**; gesamt 256 lassen; Channel löschen bzw. archivieren als künftige Interaction erwägen (planer) |
  | Verlauf: Q1, Q2, Q6 und ADR-0006 sind auf **100.000 Nachrichten insgesamt** ausgelegt (Probe 1.000.000: Laden 3,5 s, Speicher ≈ 1,1 GB, Vollscan 220–380 ms) | 100.000 je Channel × 50 aktiv = bis 5.000.000; aus der Rate: 5/s × 86.400 × 7 Tage Frist ≈ 3.000.000 in der Frist | Block und Öffnen je Channel **passen** (Index je Gespräch, Block unabhängig von der Gesamtmenge); **unterschreitet**: Start (linear ≈ 10 s bei 3 Mio, ≈ 17 s bei 5 Mio gegen Q6 < 2 s), Speicher (≈ 3,3 bzw. 5,5 GB auf einem normalen Rechner), Suche über alle sichtbaren Nachrichten (Vollscan ≈ 0,7–1,1 s bzw. 1,1–1,9 s gegen Profil < 1 s und Q2 < 100 ms) | vor I0011: Probe mit 3 und 5 Mio im Release-Build; dann ADR zu Suche (z. B. Treffergrenze mit Abbruch nach Zeitbudget, Suche je Channel zuerst) und Index (ADR-0006 Option D „Offsets“ für Speicher); Q2 und Q6 auf das Profil neu fassen |
  | Live-Bus `KAPAZITAET` 1.024 Ereignisse (`src/live/mod.rs:8`), Zustellung < 1 s (Q3) | Spitze 30 Ereignisse/s an ≈ 70 Leser | **passt** (Puffer ≈ 34 s Spitze für einen langsamen Leser) | — |
  | Betriebsprotokoll (ADR-0042 Regel 6) | Information ≈ 1.500 Einträge/Tag; Debug bei 50 wartenden Agenten ≈ 250 Anfragen/s | **ausgelegt**: Budgets 64/32/32 MiB je Tag | Messwert nach ADR-0042, Folgeaufgaben |

  Zusammengefasst unterschreiten vier Grenzen das Profil: Verbindungen je Herkunft (64 → 160), Sitzungen je Herkunft (64 → 128), Channels je Herkunft (32 → 128) und die Auslegung des Verlaufs auf 100.000 insgesamt (Probe 3–5 Mio vor I0011); das Schreibkonto je Herkunft ist ein Grenzfall, der an der Dauer einer Spitze hängt (Frage an den Betreiber).
