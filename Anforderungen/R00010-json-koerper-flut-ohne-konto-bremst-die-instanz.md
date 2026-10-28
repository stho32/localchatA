---
id: R00010
status: Neu
datum: 2026-10-28
ursprung: Bug-Report
ursprungslauf: R00003
---

# R00010: Behebung der Verlangsamung durch eine Flut großer JSON-Körper ohne Konto

## Beschreibung

Jeder Rechner im LAN kann die Instanz ohne Sitzung spürbar verlangsamen: `POST /api/beitreten` nimmt JSON-Körper bis 2 MiB an und baut daraus auf dem async-Worker einen vollständigen `serde_json::Value`-Baum, bevor überhaupt geprüft wird, ob ein Name darin steht. Beobachtet im Security-Review von Karte #6 (2026-10-28): 24 Schleifen solcher Körper ohne Konto heben die Antwortzeit von `GET /api/gesundheit` (p90) auf 0,229 s, 48 Schleifen auf 0,272 s bei einem Speicher (RSS) von etwa 460 MB. Ein Beitritt braucht dagegen nur wenige hundert Bytes (`name` bis 32 Zeichen, `art`); auch `POST /api/channels` braucht höchstens etwa 2,6 KiB (Name und Beschreibung bis 200 Zeichen).

Reproduktion (nachgefahren vom planer, 2026-10-28, Release-Programm aus `cargo install`, eigener Ordner und Port, Rechner mit 8 Kernen bei Last ≈ 4): je Schleife wiederholt `POST /api/beitreten` mit `Content-Type: application/json` und dem Körper `[0,0,…,0]` aus 1 048 575 Nullen (2 097 151 Bytes, 1 Byte unter 2 MiB; Antwort 422), Schleifen reihum von `127.0.0.1` bis `127.0.0.4`; von `127.0.0.5` alle 50 ms `GET /api/gesundheit`, 10 s lang:

| Schleifen | Körper | Gesundheit p50 | p90 | Antworten |
|---|---|---|---|---|
| 0 (Leerlauf) | — | 0,0015 s | 0,0017 s | — |
| 24 | 2 097 151 Bytes (Parse) | 0,072 s | 0,175 s | 603 × 422 |
| 48 | 2 097 151 Bytes (Parse) | 0,060 s | 0,279 s | 735 × 422 |
| 24 | 2 097 153 Bytes (413, ohne Parse) | 0,008 s | 0,018 s | 9 608 × 413 |
| 48 | 2 097 153 Bytes (413, ohne Parse) | 0,018 s | 0,038 s | 10 072 × 413 |

Die letzten beiden Zeilen zeigen den Weg nach dem Fix: derselbe Körper, aber über der Grenze — gelesen und verworfen, nie geparst.

Zahlt ein auf: [Vision](R00000-vision.md) — eine gemeinsame LAN-Instanz ohne Anmeldung, in der ein Channel sofort öffnet (Leistungsziel 100.000 Nachrichten).

## Ursachenanalyse

### Root Cause

- `src/http/koerper.rs:14` — `GRENZE = 2 * 1024 * 1024` gilt für jede JSON-Route gleich, ohne Bezug auf das, was die Route braucht.
- `src/http/koerper.rs:30–36` — `json_lesen` liest bis `GRENZE` und ruft dann `serde_json::from_slice::<Value>` direkt im async-Handler auf. Für `[0,0,…]` mit 1 048 575 Elementen entstehen ebenso viele `Value`-Knoten (je 32 Bytes und mehr) — CPU-Arbeit von vielen Millisekunden je Anfrage auf einem Tokio-Worker, der in dieser Zeit keine andere Verbindung bedient, und Speicher in der Größenordnung zehn MB je laufender Anfrage.
- `src/http/beitreten.rs:29` — `beitreten` liest den Körper vor jeder Prüfung und ohne Sitzung (die Route ist der Weg zur Sitzung). Begrenzt ist nur die Zahl der Verbindungen (64 je Herkunft, 512 gesamt, ADR-0010), nicht die Arbeit je Anfrage.
- `src/http/channels.rs:29` — derselbe Leseweg mit Sitzung (Extraktor `Teilnehmer` vorher, also 401 ohne Sitzung); eine Sitzung kostet einen Beitritt.

Ausgeschlossen: der Grenzschutz (Host, Origin) lässt `curl` ohne `Origin` durch, wie gewollt (ADR-0014); die Annahmeschleife hält ihre Grenzen (gemessen: keine abgewiesene Verbindung, alle Anfragen beantwortet). Der Engpass ist das Parsen, nicht das Lesen: derselbe Körper 2 Bytes größer (413, gelesen und verworfen, kein Parse) lässt p90 bei 0,018 s bzw. 0,038 s.

### Betroffene Komponenten

- `src/http/koerper.rs` (`json_lesen`, `GRENZE`)
- `src/http/beitreten.rs`, `src/http/channels.rs` (Aufrufer)
- künftig `PUT /api/channels/<name>/frist` (R00008, noch nicht gebaut) und jede weitere JSON-Route
- ADR-0010 (Grenzen der Annahmeschleife) — Nachzug durch den `architekt`, siehe Notizen

## Lösungsvorschlag

### Langfristige Lösung

**Option A — Körpergrenze je Route.** `json_lesen` bekommt die Grenze als Argument; jede Route nennt ihre eigene Grenze mit Herleitung. `beitreten` und `channels` (anlegen) bekommen je **4096 Bytes**. Über der Grenze antwortet die Route 413, bevor ein Byte geparst wird (der Rest wird wie heute begrenzt gelesen und verworfen, damit der Client die 413 sieht statt eines Verbindungsabbruchs).

Herleitung der 4096 (gerechnet): der größte gültige Körper ist ein Channel mit 32 Zeichen Name und 200 Zeichen Beschreibung, jedes Zeichen in der längsten JSON-Schreibweise — Name `a` je Zeichen (6 Bytes), Beschreibung ein Ersatzpaar `😀` je Zeichen (12 Bytes): `{"name":"` (9) + 32 × 6 (192) + `","beschreibung":"` (18) + 200 × 12 (2 400) + `"}` (2) = **2 621 Bytes**. Mit 30 % Reserve 3 408 Bytes, auf 4 KiB gerundet 4 096 (≈ 56 % Reserve). In gewöhnlicher Schreibweise ist derselbe Körper 9 + 32 + 18 + 800 + 2 = 861 Bytes; ein Beitritt höchstens `{"name":"` (9) + 192 + `","art":"` (9) + 6 × 6 (36) + 2 = 248 Bytes.

### Alternative Ansätze

| Option | Komplexität | Testbarkeit | Reversibilität | Risiko |
|---|---|---|---|---|
| **A Körpergrenze je Route** (Empfehlung) | gering: ein Argument, zwei Konstanten | hoch: Spanne 4096/4097 je Route, Messung | hoch | eine künftige Route mit großen Körpern (Nachricht senden, I0009) braucht eine eigene, begründete Grenze |
| B Typisierte Struktur statt `Value` | gering je Route | hoch | hoch | parst weiter bis 2 MiB auf dem Worker (O(n) CPU, ein 2-MiB-String wird trotzdem angelegt) — dämpft, begrenzt nicht |
| C Parse in `spawn_blocking` mit Semaphore | mittel: Pool, Erlaubnisse, neue Einstellung | mittel: Wettlauf und Wartezeit messen, eigene Spanne für die Zahl der Erlaubnisse | mittel | Speicher bleibt (2 MiB × Ausdehnung × Erlaubnisse); unter Flut warten echte Beitritte in der Schlange der Semaphore |

- B verworfen als Fix: ohne Grenze bleibt die Arbeit je Anfrage proportional zu 2 MiB; als spätere Härtung sinnvoll (Sollte, gehört nach der Regel „Nacharbeit nur Muss“ auf eine Härtungskarte).
- C verworfen für diese Routen: sie brauchen keine großen Körper, also ist das Problem durch Nichtannehmen gelöst statt durch Verlagern; C bleibt die Antwort für eine künftige Route, die große Körper wirklich braucht.

## Test-Strategie

### Unit Test zur Bug-Reproduktion

- `src/http/koerper.rs`, Test `grenze_je_aufruf_als_spanne`: `json_lesen(kopf, koerper, 4096)` mit einem Körper aus genau 4 096 Bytes gültigem JSON (`{"name":"spanne"}` und Leerzeichen bis 4 096) → `Ok`; mit 4 097 Bytes → `Err(ZuGross)`; mit 4 097 Bytes **ungültigem** JSON (`{` und 4 096 × `x`) → `Err(ZuGross)`, nicht `Err(Kaputt)` — die Grenze greift vor dem Parse. Heute rot, weil `json_lesen` keine Grenze annimmt (kompiliert nicht) und 4 097 Bytes unter 2 MiB geparst würden.
- Integration (`tests/beitreten.rs`, echtes Programm): `POST /api/beitreten` mit den 4 097 ungültigen Bytes → **413** `{fehler}` mit „zu groß“; heute 400. Das ist der Reproduktionstest im Gate.

### Code-Extraktion für isoliertes Testing

Keine neue nötig: `json_lesen` ist bereits eine Operation mit eigenen Unit-Tests; die Grenze wird Argument statt Konstante.

### Regressionstests

- Integration je Route (`beitreten`, `channels` anlegen) über das echte Programm: Spanne 4 096 → nicht 413 (Antwort nach Inhalt: 201, 409 oder 422) · 4 097 → 413; ohne `Content-Type` und über der Grenze → 413 (wie heute: Größe vor Typ); bestehende 413-Tests (`zu_grosser_koerper_ergibt_413_auch_bei_langsamem_senden`, `ein_riesiger_tropfender_koerper_bekommt_die_413_nach_kurzer_frist`) bleiben grün.
- Mutationsprobe (Spanne, Lehre #5): Grenze 4 095 → Test „4 096 nicht 413“ rot; Grenze 4 097 → Test „4 097 ist 413“ rot; Grenze wieder 2 MiB bzw. ignoriert → beide Routen-Tests „4 097 ist 413“ rot; Grenze nur an einer Route gesetzt → der Test der anderen Route rot. Mutationsproben im eigenen Worktree mit eigenem `CARGO_TARGET_DIR`, Original mit frischem Änderungsdatum zurück.
- Kontrollpaar (Integration): gültige Beitritte `{"name":"spanne"}` → 201 und Channel-Anlagen `{"name":"security-entwicklung"}` → 201 unverändert; der längste gültige Körper (2 621 Bytes, Schreibweise wie in der Herleitung) ist nie 413.
- Leistungstest Flut (`tests/verfuegbarkeit.rs`, `#[ignore]`, nur im Release-Build): siehe Akzeptanzkriterien „Flut“.

## Akzeptanzkriterien

- [ ] Der Reproduktionstest ist ohne Fix rot und mit Fix grün
- [ ] Der ursprüngliche Fehler tritt im Szenario der User Story nicht mehr auf
- [ ] Keine neuen Fehler: alle Tests aller Ebenen grün (Gate nach `AI-Team/umgebung.md`)

### Grenze (Gate)

- [ ] `POST /api/beitreten` und `POST /api/channels` (mit Sitzung) nehmen Körper bis einschließlich 4 096 Bytes an und antworten bei 4 097 Bytes und mehr **413** `{fehler}` mit „zu groß“, ohne den Körper zu parsen: 4 097 Bytes ungültiges JSON → 413, nicht 400.
- [ ] Spanne mit Mutanten (Grenze ± 1 Byte): je Route ein Test für 4 096 (nicht 413) und einer für 4 097 (413); die Mutanten „Grenze 4 095“, „Grenze 4 097“, „Grenze 2 MiB/ignoriert“ und „Grenze nur an einer Route“ werden je von mindestens einem Test rot. Ergebnis der Mutationsprobe steht im Bericht des Entwicklers.
- [ ] Kontrollpaar: gültige Beitritte und Anlagen bleiben 201 — `{"name":"spanne"}` → 201; `{"name":"security-entwicklung"}` mit Sitzung → 201; der längste gültige Channel-Körper (2 621 Bytes: Name `a` × 32, Beschreibung `😀` × 200) ist nie 413 (201, sobald F0026 Beschreibung gebaut ist, vorher 422 `regel:"beschreibung"`).

### Flut (Messung im Release-Build, Entwicklertest und Abnahme)

- [ ] **Abnahme der Befundkarte #21** (wörtlich): 24 und 48 Schleifen dieses Körpers von 4 Herkünften für 10 s → Gesundheit von einer fünften Herkunft p90 ≤ 0,15 s.
- [ ] „Dieser Körper“ ist der Körper der Reproduktion: `[0,0,…,0]` mit 1 048 575 Nullen (2 097 151 Bytes), `Content-Type: application/json`, an `POST /api/beitreten`, ohne Sitzung; Schleifen reihum von `127.0.0.1` bis `127.0.0.4`, jede Anfrage auf eigener Verbindung; erwartete Antwort jetzt 413.
- [ ] **Zweck**: die Instanz bleibt für alle anderen bedienbar, während ein Rechner flutet — `GET /api/gesundheit` ist die billigste Route und steht für jede andere („Channel öffnet sofort“, Vision). **Startpunkt der Uhr**: je Gesundheitsanfrage vom Öffnen der Verbindung bis zum letzten Byte der Antwort; gemessen von `127.0.0.5` alle 50 ms ab 1 s nach dem Start der Schleifen bis zu ihrem Ende (≈ 180 Messungen), p90 = 90-%-Quantil dieser Messungen. **Erwarteter Wert** (gemessen, siehe Tabelle, 413-Weg): p90 0,018 s bei 24, 0,038 s bei 48 Schleifen; die Grenze 0,15 s liegt das 3,9-Fache darüber (≥ 30 % Reserve), und vor dem Fix liegt der Wert bei 48 Schleifen mit 0,272–0,279 s deutlich darüber (bei 24 Schleifen 0,175–0,229 s, knapp — der Test vor dem Fix gilt mit 48 Schleifen als rot).
- [ ] **Kontrolle in derselben Messung**: vor den Schleifen 3 s Leerlauf von `127.0.0.5`; liegt der Leerlauf-p90 über 0,05 s (Rechner zu ausgelastet; erwartet ≈ 0,002 s), ist der Lauf ungültig und wird wiederholt, nicht als rot gewertet. Die Reihenfolge 24/48 ist zufällig; Leerlauf-p90, p50, p90, max, Zahl der Antworten je Status und RSS nach dem Lauf stehen im Bericht (RSS ist Messwert, keine Grenze).
- [ ] Die Messung läuft gegen das echte Programm im Release-Build (`cargo test --release --test verfuegbarkeit -- --ignored koerperflut`), nicht im Gate; die Grenze 0,15 s steht nur hier und wird im Test übernommen, nie verschärft.

## Implementierungshinweise

- `json_lesen(kopf, koerper, grenze: usize)`; Konstanten je Route neben dem Handler mit einem Satz Herleitung (`BEITRETEN_GRENZE`, `CHANNEL_ANLEGEN_GRENZE`, je 4 096); `GRENZE` (2 MiB) entfällt als Vorgabe. Das Verwerfen des Rests (`VERWERFEN_HOECHSTENS`, `VERWERFEN_FRIST`) bleibt — sonst sieht der Client einen Abbruch statt der 413 (bestehende Tests).
- Die Fehlermeldung der 413 nennt die Grenze der Route („höchstens 4096 Bytes“), damit ein Agent den Ausweg kennt.
- `PUT /api/channels/<name>/frist` (R00008, F0035 ff.) verwendet bei seinem Bau dieselbe Form mit eigener Grenze; wird R00010 zuerst gebaut, hat R00008 das Argument schon.
- Für Leistungstests gilt `gemeinsam/gelerntes.md` (Timing-Proben bei hoher Systemlast nur mit zufälliger Reihenfolge und Kontrollpaar) und `AI-Team/entwickler.md` (#4: Grenze nie schärfer als hier, Median-Abstand zur Grenze im Bericht).

## Offene Fragen

- Keine für den Bau. Nachzug an den `architekt` (nicht blockierend): Ergänzung zu ADR-0010 „jede JSON-Route nennt ihre Körpergrenze mit Herleitung; Parse großer Körper nur außerhalb des Workers (Option C)“ — gehört vor I0009 (Nachricht senden), dessen Körper größer sein wird.

## Warum löst diese Anforderung das Problem? (Pflicht)

Auslöser ist der Security-Befund: ein Rechner ohne Sitzung schickt in Schleifen 2-MiB-Körper an `beitreten`, und jede Anfrage baut auf einem Tokio-Worker einen Baum aus über einer Million JSON-Werten — die Worker sind mit Parsen belegt, Gesundheit und alle anderen Routen warten (p90 0,23–0,28 s, RSS ≈ 460 MB). Ursache ist die eine 2-MiB-Grenze für Routen, die höchstens 2,6 KiB brauchen. Wenn jede Route nur annimmt, was sie braucht (4 KiB), dann wird ein großer Körper mit 413 abgewiesen, bevor geparst wird; die Arbeit je Anfrage schrumpft auf Lesen und Verwerfen, und die gemessene Antwortzeit der Gesundheit fällt unter Flut auf 0,018–0,038 s. Der Hebel liegt an der Grenze und nicht an der Ausführung (Option C), weil Arbeit, die gar nicht anfällt, weder Worker noch Speicher bindet; die Spanne 4 096/4 097 mit Mutanten stellt sicher, dass die Grenze wirklich dort sitzt und echte Beitritte und Anlagen nicht trifft.

## Notizen

- Priorität: Hoch (Verfügbarkeit ohne Anmeldung, CWE-400/CWE-770)
- Betroffene Nutzer/Systeme: alle Teilnehmer einer Instanz im LAN, Agenten mit Gesundheitsprüfung
- Workaround: keiner im Programm; Netz auf vertrauenswürdige Rechner beschränken
- Weiche (`rollen/planer/befunde-einplanen.md`): R00003 sagt keine Verfügbarkeit unter Körperflut zu, nach der Weiche also „reifer“; als Sicherheitsbefund nach Auftrag des Directors im Bug-Sonderformat (Ursache, Optionen, Test-Strategie). Die Abnahme der Befundkarte ist als Angriffsmatrix übernommen; eine erweiterte Matrix (z. B. `Transfer-Encoding: chunked` ohne Länge) schreibt bei Bedarf der `security-reviewer`.
- Herkunft: Befundkarte #21, Security-Review Karte #6 (I0007), 2026-10-28; Ursprung R00003 (F0008 Beitreten über die API) und ADR-0010
- Messung der Reproduktion: Skript im Scratchpad des planers (entfernt), Release-Programm `~/.cargo/bin/localchatA` vom 2026-10-28 09:31 auf eigenem Port und Wegwerf-Ordner; Systemlast beim Messen ≈ 4 (8 Kerne)
- WBS: F0040 unter I0004 mit B0204, B0205

### Bubbles

- F0040: B0204 Körpergrenze je Route · B0205 Flut im Release-Build messen
