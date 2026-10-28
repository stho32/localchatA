---
id: R00013
status: Neu
datum: 2026-10-28
ursprung: Bug-Report
ursprungslauf: R00007
---

# R00013: Behebung des Speicherwachstums, wenn langsame Leser große Verlaufsblöcke holen

## Beschreibung

Jeder Rechner im LAN mit einer Sitzung kann den Speicher der Instanz um Hunderte MB treiben, ohne etwas zu senden: er öffnet 64 Verbindungen (die Grenze je Herkunft, ADR-0010), holt auf jeder `GET /api/channels/<name>/nachrichten?n=200` aus einem Channel mit großen Nachrichten und liest die Antwort nie. Ein Block ist heute nur durch die Anzahl `n` begrenzt, nicht durch seine Größe: 200 Nachrichten zu 16 KiB ergeben eine Antwort von 3,27 MB, und jeder Abruf erzeugt Kopie, `serde_json::Value`-Baum und Ausgabepuffer auf einem eigenen Blocking-Thread — 64 Abrufe laufen gleichzeitig. Beobachtet im Security-Review von Karte #7 (I0008, 2026-10-28): + 766 MB bei einer Herkunft, 2,25 GB bei sieben; nach dem Schließen bleiben 1,46 GB. Erzeugbar wird der Fall mit „Nachricht senden“ (I0009: 16 KiB je Nachricht, ADR-0034); heute nur über Dateien im Ordner.

Reproduktion (Probe des `architekt`, ADR-0039 Probe 1, Release-Build 9d7d4bb, 8 Kerne, Last ≈ 3,5): Channel mit 200 Nachrichten zu 16 384 Bytes `x` (eine Zeile) an einem Tag; je Herkunft 64 Rohverbindungen mit `SO_RCVBUF` 4 KiB, je `GET …/nachrichten?n=…` ohne Lesen; RSS aus `/proc/<pid>/status`, offene Verbindungen getrennt nach Server-Seite (`ss sport`) und Client-Seite (`ss dport`):

| Lauf | RSS vorher | RSS nach 1–10 s | Threads | Server-Seite offen | RSS nach dem Schließen |
|---|---|---|---|---|---|
| `n=200`, 1 Herkunft | 8 MB | 754 MB | 72 | 64 bis 10 s, 0 ab 12 s | 169 MB |
| `n=200` (`nach`), 2 Herkünfte | 7 MB | 1 027 MB | 134 | 128 bis 10 s, 0 ab 12 s | 490 MB |
| `n=100`, 1 Herkunft | 7 MB | 281 MB | 72 | 0 ab 12 s | 224 MB |
| `n=15` (≈ 256 KiB, Stellvertreter für das Budget) | 8 MB | 40 MB | 64 | 0 ab 12 s | 30 MB |
| `n=1` | 8 MB | 10 MB | 14 | 0 ab 12 s | 10 MB |

Erwartet: der Speicher je Abruf ist eine feste, kleine Zahl, unabhängig von Nachrichtengröße und `n`. Tatsächlich: er wächst mit `n` × Nachrichtengröße × Zahl der Verbindungen und kommt nach dem Schließen nicht ganz zurück. Die Behauptung des Befunds „nach 120 s noch offen“ trifft nur die Client-Seite: das Programm schließt jede nicht lesende Verbindung nach 10–12 s selbst (siehe Ursachenanalyse) — diese Sendefrist hat bisher niemand benannt oder getestet.

Zahlt ein auf: [Vision](R00000-vision.md) — eine gemeinsame LAN-Instanz ohne Anmeldung, „schnell, auch bei großem Verlauf“ (Leistungsziel 100.000 Nachrichten).

## Ursachenanalyse

### Root Cause

- `src/gespraeche/verlaufsindex.rs:96` (rückwärts) und `:130` (vorwärts) — der Block nimmt `n` Sätze, gleich wie groß sie sind; es gibt keine Grenze in Bytes. Bei 200 Höchstgrößen sind das 3,2 MB Kopie (`Verlauf::satz`, `:167`) unter der Gesprächssperre.
- `src/gespraeche/mod.rs:324–331` — `verlauf` kopiert alle Sätze und deutet sie zu `Nachricht` (zweite Kopie des Textes).
- `src/http/nachrichten.rs:31–36` — jeder Abruf startet `spawn_blocking` ohne Begrenzung der gleichzeitigen schweren Arbeit; 64 Abrufe einer Herkunft laufen auf 64 Blocking-Threads, jeder mit eigener glibc-Arena (Probe: 72 Threads).
- `src/http/nachrichten.rs:49–57` — `block_als_json` baut einen `serde_json::Value`-Baum (dritte Kopie), und `:38` `Json(antwort).into_response()` serialisiert ihn auf dem async-Worker (vierte Kopie als Ausgabepuffer). Danach hält hyper je Verbindung die ungesendete Antwort.
- `src/main.rs:5` — `#[tokio::main]` mit dem Standard `max_blocking_threads` = 512: nichts begrenzt die Zahl der Arenen über die Verbindungsgrenze hinaus.

Ausgeschlossen: Die Verbindungen bleiben **nicht** beliebig offen. `src/start/abrisspflege.rs:29` (`pflege_aus`) setzt je Verbindung `TCP_USER_TIMEOUT` = 2 × Herzschlag (Standard 10 s, ADR-0017); unter Linux ≥ 5.11 zählt der Kernel auch unbeantwortete Fenster-Proben (Nullfenster) dagegen und schließt die Verbindung server-seitig nach Grenze + ≈ 0,3 s (ADR-0039 Probe 2: 2,29–2,32 s bei 2 s, 10,27–10,30 s bei 10 s). Der Client sieht seinen Socket weiter `ESTABLISHED`, weil kein RST kommt. Ein latenter Defekt bleibt im Browser: `web/fenster.js:26–48` wirft über 5 Blöcke immer am anderen Ende ab — mit Blöcken unter 1,25 Sichthöhen (z. B. 15 eingeklappte große Nachrichten an einem 4K-Schirm) lädt das Fenster ohne Scrollen hin und her (ADR-0026, Ergänzung).

### Betroffene Komponenten

- Verlaufsindex (`src/gespraeche/verlaufsindex.rs`, A00009), Gespräche (`src/gespraeche/mod.rs`)
- HTTP-Schnittstelle Verlauf (`src/http/nachrichten.rs`, A00004), Zustand (`src/http/mod.rs`)
- Start und Laufzeit (`src/main.rs`, `src/start/`, A00003); Abrisspflege (`src/start/abrisspflege.rs`) bleibt unverändert, wird aber als Sendefrist getestet; Hilfe (`src/start/aufruf.rs`, `HILFE`)
- Weboberfläche Fenster (`web/fenster.js`, `web/verlauf.js`, A00005)
- künftig: Senden (I0009, B0211 „Großen Körper außerhalb des Workers lesen“ nutzt dieselbe Werkbank), Anhänge (I0010), Suche (I0011), Direktgespräche (I0012), Warte-Aufruf (I0016), Anleitung für Agenten (I0019)

## Lösungsvorschlag

### Langfristige Lösung

[ADR-0039](../Dokumentation/ADRs/0039-grosse-antworten-byte-budget-je-block-werkbank-und-abrissgrenze-als-sendefrist.md), Optionen 1A, 2A, 3A, 4A:

1. **Byte-Budget im Index** — `VERLAUF_BUDGET` = 262 144 Satzbytes (256 KiB) je Block. Der Index nimmt in Cursor-Richtung Satz für Satz, solange `Summe + Satzlänge ≤ VERLAUF_BUDGET` **und** weniger als `n` genommen sind; der erste Satz wird immer genommen. Abgeschnitten wird am cursorfernen Ende (neueste und `vor` → `aelter: true`; `nach` → `neuer: true`), der Block bleibt lückenlos vom Cursor aus; kopiert werden nur die genommenen Sätze. `n` ist ab jetzt eine **Obergrenze**; Fortsetzung allein über `aelter`/`neuer`.
2. **Sendefrist = Abrissgrenze** (2 × Herzschlag) für jede Route — keine neue Mechanik, aber benannt, im Gate getestet und in der Hilfe beschrieben.
3. **Werkbank** — eine gemeinsame Semaphore mit `available_parallelism` Erlaubnissen (mindestens 2); der Verlaufs-Handler erwirbt eine Erlaubnis **vor** `spawn_blocking`, dort entstehen Auswahl, Kopie und die fertigen JSON-Bytes (`serde_json::to_vec` über serialisierbare Strukturen). Dazu `max_blocking_threads(64)` als Rückhalt.
4. **Allgemeine Regel für große Antworten** (ADR-0039 Regel 7) gilt ab jetzt für jede neue Route; hier nur als Review-Punkt, gebaut wird sie mit den Routen I0010, I0011, I0012, I0016.
5. **Abwurfschutz im Browser** (ADR-0026, Ergänzung): über 5 Blöcke wird am anderen Ende nur abgeworfen, wenn der Abstand vom Blick zu diesem Ende danach noch mindestens 2 Sichthöhen beträgt; sonst wächst das Fenster bis höchstens 8 Blöcke.

Herleitung des Budgets (ADR-0039 Regel 1): der größte realistische Block zu 200 hat 80,7 KiB (100.000 Nachrichten aus dem Testdaten-Erzeuger, gemessen) — das Budget liegt 3,2-fach darüber, realistische Blöcke bleiben vollständig. JSON ≤ 2 × Satzbytes + 60 Bytes je Nachricht ≈ 0,5 MiB je Antwort; 64 Verbindungen einer Herkunft ≤ 32 MiB.

### Alternative Ansätze

| Teilfrage | Option | Komplexität | Testbarkeit | Reversibilität | Risiko |
|---|---|---|---|---|---|
| Größe | **1A Satzbytes-Budget im Index** (gewählt) | gering: eine Bedingung im Sammeln | hoch: rein, Spanne ± 1 Byte | hoch | `n` ist nur Obergrenze; Leser, die zählen, hören zu früh auf |
| | 1B Budget auf die JSON-Antwort | mittel | hoch | hoch | kopiert und serialisiert, bevor entschieden ist |
| | 1C kleineres `n` (≤ 20) | gering | hoch | hoch | begrenzt nicht (20 × 32 KiB = 640 KiB), bricht Blöcke zu 100 |
| | 1D Antwort als Strom mit Gegendruck | hoch | mittel | gering | Sperren über Zeit, Entfallen mitten im Strom |
| Sendefrist | **2A Abrissgrenze benennen und testen** (gewählt) | keine neue Mechanik | Gate mit 2H = 4 s | hoch | nur Linux ≥ 5.11; Tropfleser über echtes LAN nicht geprobt |
| | 2B Fortschrittsfrist im Programm | mittel | hoch | hoch | doppelt zur Kernel-Grenze |
| | 2C Mindestrate / 2D Gesamtfrist | mittel | mittel | mittel | Parameter bzw. Ausnahmeliste für Ströme |
| Arbeit | **3A Werkbank + Rückhalt 64** (gewählt) | mittel | In-Process mit gehaltener Erlaubnis | hoch | unter Flut ≈ 8 ms Wartezeit |
| | 3B nur `max_blocking_threads` kleiner | gering | gering | hoch | begrenzt auch leichte Arbeit |
| | 3C k Abrufe je Herkunft (429) | mittel | hoch | hoch | trifft den geteilten Loopback der Agenten |
| | 3D anderer Allokator | mittel | gering | mittel | behandelt die Folge, nicht die Spitze |

- 1B verworfen: die Arbeit, die begrenzt werden soll, ist dann schon getan. 1C verworfen: begrenzt nicht. 1D verworfen: viel Code für einen Gewinn, den 1A mit einer Bedingung bringt.
- 2B, 2C, 2D verworfen: unter Linux doppelt bzw. mit Parametern oder Ausnahmelisten gegen einen kleinen Gewinn (ein Tropfleser hält mit Budget ≤ 0,5 MiB).
- 3B verworfen: unscharf. 3C verworfen: Betreiber und alle lokalen Agenten teilen `127.0.0.1`. 3D verworfen: mit Budget und Werkbank ist die Spitze klein.

## Test-Strategie

### Unit Test zur Bug-Reproduktion

- `src/gespraeche/verlaufsindex.rs`, Test `budget_schneidet_den_block_als_spanne` (K1): gebaute Sätze mit bekannter Satzlänge, Budget `B` als Parameter — vier Sätze mit Summe genau `B`, `n=200` → alle vier; Summe `B + 1` → drei, neueste `aelter: true`. Heute rot: `block` kennt kein Budget (kompiliert nicht bzw. liefert vier).
- Integration im Gate (K2, echtes Programm): Channel `riesig` mit 200 Nachrichten zu 16 384 Bytes `x` an einem Tag, `GET …?n=200` → 15 Nachrichten, `aelter: true`. Heute rot: 200 Nachrichten (3,27 MB).

### Code-Extraktion für isoliertes Testing

- Das Sammeln mit Budget bleibt eine reine Operation des Verlaufsindex mit dem Budget als Parameter (die Konstante setzt der Aufrufer) — so ist die Spanne mit kleinen Zahlen testbar.
- `Werkbank` als eigener Typ im Zustand mit einer reinen Regel `Erlaubnisse aus Kernen` und einem Erzeuger mit fester Zahl für Tests.
- Abwurfschutz als reine Funktion in `web/fenster.js` (Abstände und Höhen als Eingabe, wie `nachladen` heute).

### Regressionstests

- Bestehende Tests aus R00007 bleiben unverändert grün, insbesondere „100.000 lückenlos“, L1 (< 50 ms), L3 (genau 500 im DOM), L4, L5 — das Kontrollpaar für „realistische Blöcke bleiben vollständig“.
- Bestehender Unit-Test `pflege_aus` (Abrissgrenze aus dem Herzschlag, ± 1 ms) bleibt.
- Mutationsproben je Kriterium unten; Ergebnis im Bericht des Entwicklers (Mutanten im eigenen Worktree mit eigenem `CARGO_TARGET_DIR`).

## Akzeptanzkriterien

- [ ] Der Reproduktionstest ist ohne Fix rot und mit Fix grün
- [ ] Der ursprüngliche Fehler tritt im Szenario der User Story nicht mehr auf
- [ ] Keine neuen Fehler: alle Tests aller Ebenen grün (Gate nach `AI-Team/umgebung.md`)

Gemeinsam für alle Kriterien: Start mit `localchatA <ordner> --port 0 --host 127.0.0.1 --kein-browser` auf einem Wegwerf-Ordner. `riesig` ist ein Channel mit 200 Sätzen zu je 16 384 Bytes `x` in einer Zeile an einem Tag (`channels/riesig/2026-10-28.log`, Zeiten im Sekundentakt ab 08:00, Absender `alpha@127.0.0.2`); seine Satzlänge `s` = Kopfzeile `@ 2026-10-28T08:00:00.000+01:00 alpha@127.0.0.2\n` (48 Bytes) + Textzeile `  x…x\n` (16 387 Bytes) + Leerzeile (1 Byte) = **16 436 Bytes** — der Test liest `s` aus der Datei, statt ihn fest einzutragen. `gross` ist der Channel des Testdaten-Erzeugers mit 100.000 Nachrichten über 7 Tage (R00007). Rohverbindung = TCP-Socket mit `SO_RCVBUF` 4 KiB vor dem Verbinden, gebunden an die Adresse seiner Herkunft, der die Anfrage mit `X-LocalchatA-Sitzung` sendet und danach nie liest; die Sitzung je Herkunft entsteht vorher mit `POST /api/beitreten` von dieser Adresse auf einer eigenen, danach geschlossenen Verbindung (sonst belegt sie einen der 64 Plätze).

### K1 Budget im Index (Unit, rein)

- [ ] Mit Budget `B` als Parameter und gebauten Sätzen bekannter Länge, je für Richtung neueste, `vor` und `nach`:
  - (a) vier Sätze, Summe genau `B`, `n=200` → alle vier, Flag zum cursorfernen Ende `false`;
  - (b) vier Sätze, Summe `B + 1` → drei, und zwar die drei cursornahen (neueste/`vor`: die jüngsten, `aelter: true`; `nach`: die ältesten, `neuer: true`);
  - (c) ein einzelner Satz der Länge `B + 100` → genau dieser eine;
  - (d) zehn kleine Sätze weit unter `B`, `n=2` → zwei;
  - (e) Sätze über zwei Tage, das Budget endet auf dem älteren Tag (neueste) bzw. dem jüngeren (`nach`) → die Summe zählt über die Tagesgrenze hinweg weiter, nicht je Tag neu.
- [ ] Die Antwort bleibt älteste zuerst und lückenlos vom Cursor aus; kopiert werden nur die genommenen Sätze.
- [ ] `VERLAUF_BUDGET` ist genau 262 144 (eigener Test).
- [ ] Mutanten, die rot werden müssen (Ergebnis im Bericht): Vergleich `<` statt `≤` (wirkt wie Budget `B − 1`: (a) rot), Budget `B + 1` ((b) rot), „mindestens ein Satz“ entfernt ((c) rot), `n` übergangen ((d) rot), Summe je Tag neu ((e) rot).

### K2 Budget über HTTP (Gate, Integration, echtes Programm)

- [ ] `riesig`: `GET …/riesig/nachrichten?n=200` → 200 mit genau ⌊262 144 / s⌋ Nachrichten — Rechnung: 15 × 16 436 = 246 540 ≤ 262 144 < 16 × 16 436 = 262 976, also **15** —, die 15 neuesten (08:03:05 bis 08:03:19), `aelter: true`, `neuer: false`.
- [ ] Rückwärts weiter mit `vor=<erste Zeit des vorigen Blocks>`, **solange `aelter`** → 14 Abrufe (13 × 15 + 1 × 5), zusammen genau 200 Zeiten, streng steigend, ohne Doppel; der letzte meldet `aelter: false`.
- [ ] Vorwärts mit `nach=2026-10-28T07:59:59.999%2B01:00`, weiter mit `nach=<letzte Zeit>` **solange `neuer`** → ebenso genau 200 Zeiten in 14 Abrufen, der letzte meldet `neuer: false`.
- [ ] `n=1` → genau 1 Nachricht; `n=10` → 10.
- [ ] Die Antwort hat weiter genau die Felder `channel`, `nachrichten`, `aelter`, `neuer` (kein neues Feld) und `Content-Type: application/json`; 401, 404 und 422 mit Texten wie in R00007 unverändert.
- [ ] Mutant „Fortsetzung aus `nachrichten.length < n`“ in der Testhilfe bricht nach dem ersten Block ab → der Test stellt fest, dass weniger als 200 Zeiten gelesen sind (rot).

### K3 Kontrollpaar realistisch (Gate, Integration, `gross`)

- [ ] `n=200` an 20 Stellen des Verlaufs, darunter das Fenster aus 200 aufeinanderfolgenden Sätzen mit der größten Summe der Satzlängen (der Test findet es aus den Tagesdateien; ADR-0039 Probe 3: 82 678 Bytes) → je genau 200 Nachrichten; ohne Cursor und ohne `n` → genau 100.
- [ ] Mutant „Budget 65 536“ (64 KiB) → mindestens die Abfrage am größten Fenster liefert weniger als 200 (rot).

### K4 Langsame Leser (Messwert im Release-Build; Abnahme der Befundkarte #23)

**Zweck**: ein Rechner im LAN kann den Speicher der Instanz durch Nicht-Lesen nicht treiben und hält keinen Platz länger als die Sendefrist (Q8 in A00001). **Messweg** (`#[ignore]`-Test, `cargo test --release -- --ignored langsame_leser`, nicht im Gate): frische Instanz je Lauf mit `riesig`, Standard-Herzschlag (Sendefrist 10 s); `r0` = RSS aus `/proc/<pid>/status` (`VmRSS`) und `t0`-Threads (`Threads`) nach den Beitritten und einem ersten gelesenen Abruf `GET …/riesig/nachrichten?n=1` von `127.0.0.9`; **Startpunkt der Uhr `t0`**: alle Anfragen der Rohverbindungen sind gesendet.

| Wert | Messung | erwartet (Herleitung) | Grenze | Reserve |
|---|---|---|---|---|
| RSS-Zuwachs, 1 Herkunft | 64 Rohverbindungen von `127.0.0.1`, je `GET …/riesig/nachrichten?n=200`; Höchstwert von `VmRSS − r0`, abgetastet alle 250 ms von `t0` bis `t0 + 5 s` | ≈ + 32 MB (ADR-0039 Probe `n=15` ohne Werkbank, einzeilige Texte) | **≤ 84 MB** (64 × 2 × 256 KiB + 50 MB, ADR-0039 „Rechnung Q8“) | 2,6-fach |
| RSS-Zuwachs, 2 Herkünfte | je 64 von `127.0.0.1` und `127.0.0.2` | ≈ + 64 MB (doppelt) | **≤ 117 MB** | 1,8-fach |
| Threads | `Threads` aus `/proc` zur selben Zeit | ≤ `t0`-Threads + 64 | **≤ `t0`-Threads + 64** | — (Rückhalt `max_blocking_threads(64)`, deterministisch) |
| Sendefrist | keine dieser Verbindungen ist server-seitig offen (`ss -tn state all sport = :<p>` ohne Eintrag zu den Rohverbindungen), geprüft bei `t0 + 15 s` | ≈ 10,3 s (Abrissgrenze 10 s + 0,3 s, ADR-0039 Probe 2; Probe 1: „0 ab 12 s“) | **zu bei `t0 + 15 s`** | 1,46-fach |
| andere Routen | `GET /api/gesundheit` von `127.0.0.9` bei `t0 + 2 s` und `t0 + 5 s` | 200 in ≈ 2 ms | **200 innerhalb 1 s** | > 100-fach |
| Rückgabe | Rohverbindungen bei `t0 + 15 s` schließen; `VmRSS − r0` 15 s danach | ≈ + 22 MB (Probe `n=15`) | **≤ 50 MB** | 2,3-fach |

- [ ] **Abnahme der Befundkarte #23** (Abnahme-Probe des `security-reviewer`): bei 64 nicht lesenden Verbindungen einer Herkunft, die `?n=200` aus `riesig` holen, ist der RSS-Zuwachs ≤ 84 MB (2 Herkünfte: ≤ 117 MB) **und** jede dieser Verbindungen ist server-seitig nach der Sendefrist zu (bei `t0 + 15 s`). Beide Teile gelten zugleich im selben Lauf; der zweite Teil ist schon heute erfüllt und darf durch den Fix nicht verloren gehen.
- [ ] Alle Zeilen der Tabelle gelten im Hauptlauf (`n=200`, neueste, 1 Herkunft) und im Lauf mit 2 Herkünften; die Varianten `n=1`, `n=100`, `vor=2026-10-28T09:00:00.000%2B01:00` (alle 200 sind älter, also wie neueste) und `nach=2026-10-28T07:59:59.999%2B01:00` (1 Herkunft) halten dieselben Grenzen. Die Reihenfolge der Läufe ist zufällig; `n=1` ist das Kontrollpaar (erwartet ≈ + 2 MB).
- [ ] Heute (vor dem Fix) ist der Test rot: + 746 MB bzw. + 1 020 MB, nach dem Schließen + 161 MB bzw. + 483 MB (ADR-0039 Probe 1).
- [ ] Der Bericht des Entwicklers nennt je Lauf `r0`, Höchstwert, Threads, Zeitpunkt „server-seitig zu“ (erster Abtastzeitpunkt ohne Eintrag, Abtastung alle 250 ms) und RSS nach dem Schließen. Ein Wert über der Grenze ist ein Befund an ADR-0039, kein Grund, die Grenze im Test zu ändern; die Grenzen stehen nur hier und werden im Test übernommen, nie verschärft.

### K5 Sendefrist = Abrissgrenze (Gate, Integration, echtes Programm, Linux)

- [ ] **Zweck**: die Sendefrist ist ein benannter Vertrag, der rot wird, wenn jemand die Abrisspflege entfernt oder ihren Faktor ändert. Start mit `LOCALCHATA_HERZSCHLAG_MS=2000` (Abrissgrenze 2H = 4 s); 4 Rohverbindungen holen je `?n=200` aus `riesig`. **Startpunkt**: die Anfrage der Verbindung ist vollständig gesendet. **Ende**: die Verbindung ist server-seitig zu — beobachtet über `ss` bzw. `/proc/net/tcp` (Server-Port ohne Eintrag zur Gegenstelle), abgefragt höchstens alle 100 ms; kein festes `sleep` als Urteil, keine Beobachtung, die selbst Daten an den Server sendet.
- [ ] Je Verbindung liegt die Dauer in **[3,6 s; 5,8 s]**. Erwartet ≈ 4,3 s (Grenze 4 s + ≈ 0,3 s Kernel, ADR-0039 Probe 2) plus höchstens 0,1 s Abtastung = 4,4 s; die Obergrenze liegt 32 % darüber.
- [ ] Mutanten, die rot werden müssen: Faktor 1 statt 2 (≈ 2,3 s < 3,6 s), Faktor 3 (≈ 6,3 s > 5,8 s), Pflege nicht angewandt (nach 5,8 s noch offen). Die Frist selbst (± 1 ms) prüft weiter der Unit-Test `pflege_aus`.

### K6 Ein lesender Ereignisstrom bleibt offen (Gate, Integration)

- [ ] **Zweck**: die Sendefrist trifft keinen gesunden Strom. Start mit `LOCALCHATA_HERZSCHLAG_MS=2000`; ein Client mit Sitzung hält `GET /api/ereignisse` und liest fortlaufend. **Startpunkt**: Antwortkopf des Stroms empfangen. Nach **12 s** (= 3 × Abrissgrenze) ist der Strom offen — er liefert danach noch ein Ereignis — und in diesen 12 s kamen **mindestens 4** Herzschläge (erwartet 6, alle 2 s; 6 / 1,3 = 4,6 → 4).
- [ ] Mutant „Gesamtfrist je Antwort“ (Option 2D ohne Ausnahme, z. B. jede Antwort nach 2H geschlossen) → der Strom ist vor 12 s zu (rot).

### K7 Werkbank (Unit und In-Process-Integration mit `ausliefern`)

- [ ] Erlaubnisse aus den Kernen: 1 → 2, 2 → 2, 8 → 8 (Unit; Mutant „mindestens 2“ entfernt → 1 → 1, rot).
- [ ] Werkbank mit genau 1 Erlaubnis, die der Test hält; `GET …/nachrichten` mit Sitzung → **keine Antwort innerhalb 1 s**, solange die Erlaubnis gehalten ist (ohne Werkbank erwartet ≈ wenige ms, der Mutant „Erwerb entfernt“ antwortet also sofort und wird rot); nach der Freigabe kommt die Antwort 200 **innerhalb 1 s** (Startpunkt: Freigabe; erwartet wenige ms).
- [ ] Die Antwort entsteht als fertige JSON-Bytes im Blocking-Teil; der async-Worker serialisiert nicht (Review-Punkt, nicht Test).

### K8 Fenster im Browser ohne Hin-und-her (ADR-0026, Ergänzung)

- [ ] **L3b** (Node-Unit, `e2e/einheit/fenster.test.mjs`): `einfuegen` mit Abstand und Höhe am anderen Ende — beträgt der Abstand vom Blick zum anderen Ende nach dem Abwerfen genau 2 × Sicht → abgeworfen; 2 × Sicht − 1 px → behalten, Fenster wächst. Bei 8 Blöcken und nicht abwerfbarem anderem Ende liefert `nachladen` `null`.
- [ ] **L3b Simulation**: Sicht 2 160 px, Blöcke zu 1 680 px, `aelter` und `neuer` an beiden Enden immer `true`; Start mit 1 Block, Blick mittig in diesem Block (Oberkante des Blicks 240 px über dem Block); je Schritt `nachladen` → bei Richtung ein Block an diesem Ende einfügen (Blick bleibt an seiner Stelle im Inhalt), sonst nichts; 20 Schritte. Ergebnis: **6 Abrufe, danach keiner**, das Fenster hat 7 Blöcke. Rechnung: nach oben und unten je mindestens 2 × 2 160 = 4 320 px; über dem Blick fehlen 4 320 + 240 = 4 560 px → ⌈4 560 / 1 680⌉ = 3 Blöcke je Seite, zusammen 1 + 3 + 3 = 7 ≤ 8.
- [ ] Mutanten, die rot werden müssen: „immer abwerfen über 5“ (alte Regel: die Simulation ruft in allen 20 Schritten ab), „Obergrenze 9“ (der Test „8 Blöcke, nicht abwerfbar → `null`“). Das Kontrollpaar ist L3 aus R00007 (realistisch bei 1280 × 800 genau 500 im DOM) — der Mutant „nie abwerfen“ wird dort rot.
- [ ] **L3c** (E2E, Fenster 1280 × 2160, `riesig`): A öffnet `# riesig` und wartet 2 s → in den folgenden 2 s **kein** Abruf von `/nachrichten` (Zweck: kein Hin-und-her ohne Scrollen; erwartet 0, die Kaskade beim Öffnen braucht ≈ 3 Abrufe zu je ≈ 60 ms, also unter 0,5 s). Danach hoch bis `aelter: false` und zurück bis `neuer: false` → jede Zeit genau einmal, streng steigend (wie L5); zu keinem Zeitpunkt mehr als 8 Blöcke im Fenster (höchstens 8 × 15 = 120 Nachrichten im DOM); jedes Nachladen verschiebt die oberste sichtbare Nachricht um höchstens 1 px (wie L4).

### Betrieb

- [ ] `localchatA --hilfe` nennt die Sendefrist in einem Satz: eine Verbindung, die 2 × Herzschlag (Standard 10 s) lang nichts abnimmt oder bestätigt, schließt das System; kein neues Feld in `/api/gesundheit` (`abrissgrenze_ms` bleibt, wie es ist).
- [ ] Die Laufzeit hat `max_blocking_threads` 64 (Review-Punkt; K4 belegt es über die Thread-Zahl).

## Implementierungshinweise

- `Verlauf::block(abfrage, budget)` bzw. ein Budget im Abfrage-Typ; die Konstante `VERLAUF_BUDGET` mit Herleitung als Kommentar neben dem Handler (`src/http/nachrichten.rs`), ADR-0039 Regel 1.
- `Werkbank` als Typ im Zustand (`src/http/mod.rs`), `tokio::sync::Semaphore` in `Arc`; `erwerben` vor `spawn_blocking`, die Erlaubnis lebt bis zum Ende des Blocking-Teils. Dieselbe Werkbank nutzt später B0211 (I0009, großer Körper) statt einer eigenen Semaphore.
- JSON-Bytes über `#[derive(Serialize)]`-Strukturen und `serde_json::to_vec`; Antwort `([(CONTENT_TYPE, "application/json")], bytes)`. `nachricht_als_json` bleibt für den Strom (I0009), solange er sie braucht.
- `main.rs`: `tokio::runtime::Builder::new_multi_thread().enable_all().max_blocking_threads(64).build()` statt `#[tokio::main]`.
- `web/fenster.js`: `einfuegen` bekommt die Höhen der Blöcke und den Abstand des Blicks (aus `web/verlauf.js`); `HOECHSTENS_BLOECKE` 5 bleibt die Regel, 8 die Obergrenze.
- Leistungstests nach `gemeinsam/gelerntes.md` (zufällige Reihenfolge, Kontrollpaar) und `AI-Team/entwickler.md` (Grenze nie schärfer als hier). Rohverbindungen mit `SO_RCVBUF` vor `connect` (z. B. über `socket2`, falls schon im Baum, sonst über `rustix`); Absender `127.0.0.2` per `bind` vor `connect`.

## Offene Fragen

- Keine für den Bau.
- Nicht blockierend, an den Director: ADR-0039 nennt für K5 die Obergrenze 5,6 s und für K6 „≥ 5 Herzschläge“; nach `AI-Team/planer.md` (≥ 30 % Reserve über dem erwarteten Wert) stehen hier 5,8 s und ≥ 4 — der `architekt` gleicht den Wortlaut an oder widerspricht.
- Nicht blockierend: Messwert „Tropfleser über echtes LAN“ (ADR-0039, Aufgaben) ist nicht Teil dieser Anforderung; er gehört vor I0010 (Anhänge).

## Warum löst diese Anforderung das Problem? (Pflicht)

Auslöser ist der Security-Befund: 64 Verbindungen einer Herkunft holen je 200 Nachrichten zu 16 KiB und lesen nicht — der Speicher steigt um 746 MB und kommt nach dem Schließen nicht zurück. Ursache ist, dass die Größe einer Antwort an `n` × Nachrichtengröße hängt und jede Antwort mehrfach kopiert auf einem eigenen Thread entsteht. Wenn der Index nach 256 KiB Satzbytes aufhört zu sammeln, ist jede Antwort ≤ 0,5 MiB, bevor überhaupt kopiert wird; wenn die schwere Arbeit nur auf so vielen Plätzen läuft, wie es Kerne gibt, entstehen höchstens so viele Spitzen und Arenen; und weil die Abrissgrenze jede nicht abnehmende Verbindung nach 2H schließt, hält ein langsamer Leser höchstens 0,5 MiB für höchstens 10 s. Zusammen wird aus „Verbindungen × Antwortgröße × Arbeitsfaktor“ die feste Obergrenze ≤ 84 MB je Herkunft. Der Hebel liegt im Index, nicht an der Leitung (Strom mit Gegendruck) oder am Allokator, weil Arbeit und Speicher, die gar nicht anfallen, keinen Schutz brauchen; der Preis — `n` ist nur Obergrenze — trifft realistische Daten nicht (größter Block zu 200: 80,7 KiB) und ist im Browser durch den Abwurfschutz und für Agenten durch „weiter, solange `aelter`/`neuer`“ (I0019) abgefangen.

## Notizen

- Priorität: Hoch (Verfügbarkeit ohne Anmeldung im LAN, CWE-400/CWE-770); muss vor I0009 gebaut sein, weil Senden den Fall erzeugbar macht.
- Betroffene Nutzer/Systeme: alle Teilnehmer einer Instanz im LAN; Agenten, die den Verlauf blockweise lesen.
- Workaround: keiner im Programm; heute nur über Dateien im Ordner erzeugbar.
- Weiche (`rollen/planer/befunde-einplanen.md`): R00007 sagt unter „Sicherheit“ zu, `n` sei auf 200 begrenzt und eine Antwort „höchstens ≈ 130 KB bei den Testdaten“ — das gilt nur für realistische Texte; Speicher unter langsamen Lesern sagt R00007 nicht zu, nach der Weiche also „reifer“. Als Sicherheitsbefund nach Auftrag des Directors im Bug-Sonderformat, wie R00010.
- **Änderung einer Zusage aus R00007**: dort heißt es „ohne Cursor die neuesten `n`“; ab R00013 ist `n` eine Obergrenze, ein Block endet zusätzlich am Budget. R00007 bleibt unverändert (Anforderungen werden nicht geändert); für realistische Daten bleiben alle Kriterien von R00007 wörtlich wahr (K3).
- Abnahme-Probe: die Befundkarte selbst wurde nicht gelesen (Board nicht anfassen, Auftrag); K4 übernimmt die Probe aus ADR-0039 (K4) mit der Vorgabe des Directors „RSS-Zuwachs begrenzt **und** Verbindungen nach der Sendefrist zu“.
- Herkunft: Befundkarte #23, Security-Review Karte #7 (I0008), 2026-10-28; Ursprung R00007 (F0027 Verlauf über die API, F0031 Fenster beim Scrollen); Architektur ADR-0039 mit Ergänzungen in ADR-0010, 0017, 0025, 0026, 0034 und A00001 (Q8).
- Anleitung für Agenten (I0019): „`n` ist eine Obergrenze — weiterlesen, solange `aelter`/`neuer`“ — als Notiz an I0019 in der WBS.
- WBS: F0050 unter I0008 mit B0253–B0258.

### Bubbles

- F0050: B0253 Budget im Block · B0254 Werkbank und JSON-Bytes · B0255 Laufzeit mit Rückhalt · B0256 Sendefrist als Vertrag · B0257 Abwurfschutz im Fenster · B0258 Langsame Leser messen
