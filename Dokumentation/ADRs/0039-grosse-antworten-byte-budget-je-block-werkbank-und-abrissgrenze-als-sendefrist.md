# ADR-0039: Große Antworten — Byte-Budget je Block, schwere Arbeit nur auf der Werkbank, die Abrissgrenze ist die Sendefrist

**Status**: Akzeptiert
**Datum**: 2026-10-28
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Befundkarte #23 „langsame Leser halten große Verlaufsblöcke“, vor I0009)
**Kontext-Anforderung**: — (Befund aus dem Security-Review #7 zu I0008/R00007; prägt I0009, I0010, I0011, I0012, I0016, I0019)
**Erprobt durch**: Wegwerf-Proben im Scratchpad (2026-10-28, Release-Build von `vorlauf/23` = 9d7d4bb, 8 Kerne, Last ≈ 3,5, Kernel 6.14), danach gelöscht:
(1) **Langsame Leser gegen das echte Programm** — Fixture 200 × 16.384 Bytes Text (`x`, eine Zeile) in einem Tag; je Herkunft 64 Rohverbindungen mit `SO_RCVBUF` 4 KiB, je `GET …/nachrichten?n=…` ohne Lesen; RSS aus `/proc/<pid>/status`, offene Verbindungen getrennt nach Server-Seite (`ss sport`) und Client-Seite (`ss dport`):

| Lauf | RSS vorher | RSS nach 1–10 s | Threads | Server-Seite offen | Client-Seite offen | RSS nach dem Schließen |
|---|---|---|---|---|---|---|
| `n=200`, 1 Herkunft | 8 MB | 754 MB | 72 | 64 bis 10 s, **0 ab 12 s** | 64 bis zum Ende | 169 MB |
| `n=200` (`nach`), 2 Herkünfte | 7 MB | 1.027 MB | 134 | 128 bis 10 s, **0 ab 12 s** | 128 | 490 MB |
| `n=100`, 1 Herkunft | 7 MB | 281 MB | 72 | 0 ab 12 s | 64 | 224 MB |
| `n=15` (≈ 256 KiB, **Stellvertreter für das Budget**) | 8 MB | 40 MB | 64 | 0 ab 12 s | 64 | 30 MB |
| `n=1` | 8 MB | 10 MB | 14 | 0 ab 12 s | 64 | 10 MB |

(2) **Kernel allein** (Python, Loopback): Server-Socket mit `TCP_USER_TIMEOUT`, Client `SO_RCVBUF` 4 KiB liest nie, Server schreibt, bis der Sendepuffer voll ist → `SO_ERROR = ETIMEDOUT` nach 2,29–2,32 s (Grenze 2 s, 5 Läufe), 3,25–3,27 s (3 s), 10,27–10,30 s (10 s); `ss` zeigt solange `timer:(persist,…)`. Ein Tropfleser (4 KiB alle 2 s bzw. 8 s) wurde auf Loopback ebenso nach ≈ 10 s getrennt.
(3) **Realistischer Verlauf** — 100.000 Nachrichten über 7 Tage aus `examples/verlauf-erzeugen.rs`: größter Satz 1.346 Bytes, Mittel 410 Bytes; größte Summe der Satzbytes über 100 aufeinanderfolgende Sätze 41.769 Bytes (40,8 KiB), über 200 Sätze **82.678 Bytes (80,7 KiB)**; als JSON je Nachricht `{zeit, von, text}` fast gleich (82.516 Bytes).

## Kontext

- **Befund** (Security-Review #7, Karte #23): ein Channel mit 200 Nachrichten zu 16 KiB ergibt eine Verlaufsantwort von 3,27 MB. 64 Verbindungen je Herkunft, die `?n=200` holen und nicht lesen, treiben den Speicher von 68 auf 834 MB (1 Herkunft) bzw. 2,25 GB (7 Herkünfte); nach dem Schließen bleiben 1,46 GB. Probe (1) bestätigt die Größenordnung (754 MB bzw. 1.027 MB). Erzeugbar wird der Fall erst mit Senden (I0009, [ADR-0034](0034-nachricht-senden-post-nachrichten-text-hoechstens-16-kib-steuerzeichen-abweisen-wiederholschluessel.md): 16 KiB je Nachricht; [ADR-0036](0036-schreibkonto-token-eimer-je-kennung-und-je-herkunft.md): 32 KiB/s je Herkunft füllen 200 Höchstgrößen in rund 100 s).
- **Wo der Speicher entsteht** (Vermutung, mit den Zahlen von Probe (1) verträglich): je Abruf läuft `spawn_blocking` mit Kopie der Texte (3,2 MB), einem `serde_json::Value`-Baum und dem wachsenden Ausgabepuffer — gut 10 MB Spitze je Abruf; 64 Abrufe laufen gleichzeitig auf 64 Blocking-Threads (72 Threads in der Probe), jeder mit eigener glibc-Arena. Danach hält hyper je Verbindung die ungesendete Antwort. Die Arenen geben den Speicher nach dem Schließen nicht ganz zurück (169 MB bzw. 490 MB). `Json(Value).into_response()` serialisiert zudem auf dem async-Worker (`src/http/nachrichten.rs`).
- **Die Behauptung „nach 120 s noch offen“ trifft nur die Client-Seite**: Probe (1) zeigt, dass das Programm jede nicht lesende Verbindung nach 10–12 s selbst schließt — die Abrisspflege aus [ADR-0017](0017-stiller-abriss-kernel-erkennt-tote-verbindungen.md) (`TCP_USER_TIMEOUT` = 2H = 10 s, `src/start/abrisspflege.rs`) greift auch bei einem Nullfenster (Probe 2; Linux ≥ 5.11 zählt unbeantwortete Fenster-Proben gegen den User-Timeout). Der Kernel sendet dabei kein RST; der Client, der nie liest, sieht seinen Socket weiter `ESTABLISHED`. Die Annahmeschleife hat also bereits eine Sendefrist — eine, die niemand als solche geplant, getestet oder benannt hat.
- **Grenzen, die heute gelten**: 64 Verbindungen je Herkunft, gesamt `min(512, Deskriptoren/2)` ([ADR-0010](0010-eigene-annahmeschleife-hyper-http1.md)); Blöcke `n` 1–200, Standard 100 ([ADR-0025](0025-verlauf-ueber-die-api-bloecke-mit-zeit-als-cursor.md)); Browser-Fenster 5 Blöcke ([ADR-0026](0026-verlauf-im-browser-als-fenster-aus-hoechstens-fuenf-bloecken.md)); gemeinsame Semaphore für große Körper (ADR-0010, Ergänzung R00010, noch nicht gebaut); tokio-Standard `max_blocking_threads` = 512.
- **Qualitätsziele**: Q1 (Block < 50 ms, neueste 100 bei 100.000 Nachrichten), Q4b (stiller Abriss ≈ 13,6 s), Vision „Schnell, auch bei großem Verlauf“; der Listener steht ohne Anmeldung im LAN (`AI-Team/gelerntes.md` #1). Neu als Szenario **Q8 Langsame Leser** (in [A00001](../Architektur/A00001-localchata.md), Ergänzung): 64 Verbindungen je Herkunft holen den größten Block und lesen nicht → RSS-Zuwachs ≤ 64 × 2 × Budget + 50 MB, jede dieser Verbindungen ist server-seitig nach der Abrissgrenze zu, andere Routen antworten weiter.
- Kommende große Antworten: Anhänge (I0010, Dateien bis in den MB-Bereich), Suche (I0011, Q2: die ersten 50 Treffer < 100 ms), Direktgespräche (I0012, gleiche Abfrage wie ADR-0025), Warte-Aufrufe der Agenten (I0016, Long-Poll).

## Optionen

Vier Teilfragen, je mit eigenen Optionen; gewählt wird je Teilfrage.

### Frage 1 — Größe einer Verlaufsantwort

**1A: Byte-Budget auf die Satzbytes im Index** — der Index sammelt in Cursor-Richtung, solange die Summe der Satzlängen (`ende − anfang` des Satzes im Rohtext, inklusive Kopf- und Metazeilen) das Budget nicht übersteigt; mindestens ein Satz.
- Vorteile: die Länge ist im Index schon bekannt (Satzanfänge, ADR-0006) — das Sammeln hört auf, **bevor** kopiert oder serialisiert wird; rein und mit gebauten Sätzen exakt testbar (Budget ± 1 Byte); unabhängig von der JSON-Form, also auch für Direktgespräche (I0012) und den Warte-Aufruf (I0016) dieselbe Grenze.
- Nachteile: JSON ist nicht exakt begrenzt — Anführungszeichen, Backslash, `\n` und Tab werden zu zwei Bytes, die Einrückung des Logs fällt weg; JSON eines Textes ≤ 2 × Satzbytes. Die Antwort ist also ≤ 2 × Budget + Feldnamen (≈ 60 Bytes je Nachricht).

**1B: Byte-Budget auf die erzeugte JSON-Antwort** — Nachricht für Nachricht serialisieren, aufhören, bevor die nächste das Budget überschreitet.
- Vorteile: die Antwort ist exakt begrenzt; was gemessen wird, ist, was über die Leitung geht.
- Nachteile: kopiert und serialisiert, bevor entschieden ist; koppelt die Grenze an die HTTP-Form (eine neue Metazeile wie `art` verschiebt sie); der Index liefert dann mehr, als gebraucht wird, oder braucht die Serialisierung als Rückruf.

**1C: Kleineres `n` für alle** (z. B. höchstens 20) ohne Budget.
- Vorteile: keine neue Regel, die API bleibt „genau `n`, wenn vorhanden“.
- Nachteile: begrenzt nicht — 20 × 32 KiB Satz sind immer noch 640 KiB; bricht Designsystem (Blöcke zu 100) und Q1-Messungen; Agenten brauchen fünfmal so viele Abrufe.

**1D: Antwort als Strom mit Gegendruck** (Body wird satzweise aus dem Index erzeugt, nur so viel wie die Verbindung abnimmt).
- Vorteile: Speicher je Verbindung unabhängig von der Blockgröße; `n` bleibt wörtlich.
- Nachteile: der Strom muss den Index über viele Schritte wieder sperren (Gesprächssperre, ADR-0005) oder eine Kopie halten (dann wieder Speicher); Entfallen (ADR-0030) mitten im Strom; deutlich mehr Code; der langsame Leser hält die Verbindung trotzdem bis zur Sendefrist.

### Frage 2 — Sendefrist

**2A: Die Abrissgrenze aus ADR-0017 ist die Sendefrist** — keine neue Mechanik; `TCP_USER_TIMEOUT` = 2H gilt auch, wenn die Gegenstelle ein Nullfenster meldet; benannt, getestet, im Health-Check sichtbar.
- Vorteile: schon gebaut und je Verbindung gesetzt; Probe (1) und (2): server-seitig zu nach Grenze + ≈ 0,3 s, für jede Route; misst **Fortschritt**, nicht Dauer — ein gesunder Ereignisstrom (Herzschlag 30 Bytes alle 5 s, der Browser liest immer) erreicht nie ein Nullfenster und ist nicht betroffen; ein Strom, dessen Leser 2H nichts abnimmt, ist nach ADR-0017 ohnehin tot.
- Nachteile: nur Linux ≥ 5.11 (Projektumgebung Linux, ADR-0017 nimmt das schon in Kauf); ein geschickter Tropfleser über ein echtes LAN (MSS 1.460, kleine Fenster-Öffnungen) könnte die Uhr immer wieder zurücksetzen — auf Loopback nicht gelungen, über LAN nicht geprobt (Vermutung).

**2B: Fortschrittsfrist im Programm** — ein Wrapper um den `TcpStream` merkt, seit wann ein Schreiben ansteht (`poll_write` → `Pending`); ohne angenommenes Byte in T s → Fehler → hyper schließt.
- Vorteile: plattformunabhängig; mit gestellter Uhr und Attrappe vollständig prüfbar; trifft keinen gesunden Strom (der schreibt nie gegen ein volles Fenster).
- Nachteile: doppelt zur Kernel-Grenze unter Linux; jede Verbindung durch einen weiteren Adapter; dieselbe Lücke beim Tropfleser.

**2C: Mindestrate, solange geschrieben werden muss** (wie 2B, aber ≥ k Bytes je T s, z. B. 16 KiB je 10 s).
- Vorteile: schließt auch den Tropfleser; trifft gesunde Ströme nicht.
- Nachteile: zwei Parameter, deren falsche Wahl langsame, aber echte Verbindungen trifft; der Tropfleser hält mit Budget höchstens ≈ 512 KiB — der Gewinn ist klein gegenüber einem Angreifer, der einfach 64 Ereignisströme offen hält (ADR-0010 nimmt das in Kauf).

**2D: Gesamtfrist je endlicher Antwort** (z. B. 30 s) über eine Liste der Strom-Routen, die ausgenommen sind.
- Vorteile: einfach zu erklären; deckt den Tropfleser ab.
- Nachteile: eine Ausnahmeliste, die jede neue Strom-Route (Long-Poll I0016) kennen muss — wer sie vergisst, schneidet Ströme ab; die Antwort ist zu dem Zeitpunkt schon an hyper übergeben, die Frist muss also doch an der Verbindung sitzen (wie 2B) und die Route kennen.

### Frage 3 — Gleichzeitige Arbeit

**3A: Werkbank** — eine gemeinsame Semaphore (Erlaubnisse = Zahl der Kerne, mindestens 2), die vor jedem `spawn_blocking` mit großer Arbeit erworben wird: Verlaufsblöcke, große Körper (ADR-0010 Ergänzung Regel 2), später Suche und Anhang-Metadaten. Die Antwort entsteht dort als fertige Bytes. Dazu `max_blocking_threads` = 64 als Rückhalt.
- Vorteile: Spitze an Kopien und Arenen ≈ Kerne × Arbeit je Abruf statt Verbindungen × Arbeit; eine Stelle für alle schweren Arbeiten; FIFO-fair; der Rückhalt begrenzt Threads und Stapel, falls eine leichte Blocking-Arbeit (Platte) hängt.
- Nachteile: unter einer Flut wartet jeder in derselben Schlange (mit Budget ≈ 1 ms je Abruf: 64 Abrufe / 8 Kerne ≈ 8 ms); hängt die Platte, warten nach 64 Threads alle.

**3B: Nur `max_blocking_threads` kleiner** (z. B. 16).
- Vorteile: eine Zeile im Runtime-Builder.
- Nachteile: begrenzt auch leichte, kurze Blocking-Arbeit (Channel-Stand, Plattenschreiben); keine Unterscheidung schwer/leicht; Arenen bleiben bis zu 8 × Kerne.

**3C: Höchstens k gleichzeitige Verlaufsabrufe je Herkunft** (429 mit `Retry-After`).
- Vorteile: Fairness zwischen Herkünften; ein Rechner kann die Werkbank nicht allein füllen.
- Nachteile: `127.0.0.1` teilen Betreiber und alle lokalen Agenten (ADR-0036) — ein Schwarm bekäme 429 beim Lesen; ändert nicht die Obergrenze über alle Herkünfte (ein Angreifer nimmt acht Adressen); eine weitere Zählstelle neben Verbindungen, Sitzungen und Schreibkonto.

**3D: Anderer Allokator** (jemalloc/mimalloc) gegen die nicht zurückgegebenen Arenen.
- Vorteile: gibt Speicher nach Spitzen besser zurück.
- Nachteile: neue Abhängigkeit mit C-Bau; behandelt die Folge, nicht die Spitze; mit 3A und Budget ist die Spitze klein (Probe `n=15`: 30 MB nach dem Schließen).

### Frage 4 — künftige große Antworten

**4A: Allgemeine Regel**: jede Antwort, die über 64 KiB wachsen kann, nennt ihre Obergrenze mit Herleitung (wie die Körpergrenze, ADR-0010 Ergänzung Regel 1); Listen schneiden nach Budget mit Cursor und Fortsetzungs-Flag; Dateien werden gestreamt; schwere Arbeit auf der Werkbank; die Abrissgrenze ist die Sendefrist.
- Vorteile: I0010/I0011 erben die Lösung, statt den Befund zu wiederholen; Review-Punkt prüfbar.
- Nachteile: eine Regel mehr, die jede neue Route kennen muss.

**4B: Je Route entscheiden, wenn sie kommt.**
- Vorteile: keine Vorwegnahme.
- Nachteile: derselbe Befund kommt mit Anhängen und Suche wieder (Suche: Treffer mit Text, Anhänge: ganze Dateien).

| Frage | Gewählt | Verworfen, weil |
|---|---|---|
| 1 Größe | **1A** Satzbytes-Budget im Index | 1B kopiert vor der Entscheidung; 1C begrenzt nicht; 1D viel Code und Sperren über Zeit |
| 2 Sendefrist | **2A** Abrissgrenze, benannt und getestet | 2B doppelt unter Linux; 2C/2D Parameter bzw. Ausnahmeliste gegen kleinen Gewinn |
| 3 Arbeit | **3A** Werkbank + Rückhalt 64 Threads | 3B unscharf; 3C trifft den geteilten Loopback; 3D behandelt die Folge |
| 4 Allgemein | **4A** | 4B wiederholt den Befund |

## Entscheidung

**Gewählt: 1A, 2A, 3A, 4A.**

Es überwiegt, dass das Budget den Speicher an der Quelle begrenzt — vor jeder Kopie, rein testbar — und dass die Sendefrist schon wirkt und nur benannt und abgesichert werden muss; zusammen mit der Werkbank wird aus „Verbindungen × Antwortgröße × Arbeitsfaktor“ eine Obergrenze aus festen Zahlen (Probe `n=15` als Stellvertreter: 40 statt 754 MB). In Kauf genommen: eine Antwort hat weniger als `n` Nachrichten, obwohl es mehr gibt — Browser und Agenten müssen `aelter`/`neuer` lesen, nicht zählen; ein Tropfleser über ein echtes LAN kann eine Verbindung länger als die Abrissgrenze halten, belegt dann aber höchstens ≈ 512 KiB und einen der 64 Plätze seiner Herkunft; die Sendefrist hängt am Linux-Kernel.

Regeln:

1. **Budget `VERLAUF_BUDGET` = 262.144 Satzbytes (256 KiB)** je Verlaufsblock — Konstante neben dem Handler mit dieser Herleitung:
   - **Kontrollpaar**: der größte realistische Block zu 200 hat 80,7 KiB (Probe 3) — das Budget liegt 3,2-fach darüber; jeder Block bis 200 aus realistischen Nachrichten bleibt vollständig, Q1-Messungen und L3 aus ADR-0026 ändern sich nicht.
   - **Größte Nachricht**: 16.384 Bytes Text (ADR-0034); der Satz ist am größten, wenn jede Zeile ein Zeichen hat (`a\n` → `  a\n`): ≈ 32.768 Bytes + Kopf (≈ 70) — also mindestens **7** Höchstgrößen je Block, bei einzeiligen 16-KiB-Texten 15.
   - **Speicher**: JSON ≤ 2 × Budget + 60 Bytes je Nachricht ≈ 0,5 MiB je Antwort; 64 Verbindungen einer Herkunft ≤ 32 MiB, alle 512 Verbindungen ≤ 256 MiB im schlechtesten Zeichenmix, bei üblichem Text die Hälfte.
   - **Browser**: ein Block aus Höchstgrößen kostet im Tab ≈ 170 ms je MB (ADR-0034, Probe) — 256 KiB ≈ 45 ms statt ≈ 280 ms für 100 Höchstgrößen.
2. **Verhalten an der Grenze**: der Index nimmt in Cursor-Richtung Satz für Satz, solange `Summe + Satzlänge ≤ VERLAUF_BUDGET` **und** weniger als `n` genommen sind; der **erste** Satz wird immer genommen, auch wenn er allein größer ist (nur durch Handarbeit am Log möglich, ADR-0004). Die Antwort bleibt älteste zuerst. Richtung „neueste“ und `vor`: abgeschnitten wird am **älteren** Ende → `aelter: true`; `nach`: am **neueren** Ende → `neuer: true`. Der Block ist immer lückenlos vom Cursor aus — der nächste Abruf mit der Randzeit setzt genau an. Kopiert werden nur die genommenen Sätze.
3. **Die Antwort sagt nicht mehr „genau `n`“**: `n` ist eine Obergrenze. Fortsetzung allein über `aelter`/`neuer`, nie über `nachrichten.length < n` (Browser tut das schon: `web/fenster.js`, `web/verlauf.js`). Die Anleitung für Agenten (I0019) nennt es; die 200-Antwort bekommt **kein** neues Feld (der Grund des Abschneidens ist für Leser ohne Belang).
4. **Antwort als Bytes auf der Werkbank**: der Verlaufs-Handler erwirbt eine Erlaubnis der Werkbank **vor** `spawn_blocking`; im Blocking-Teil entstehen Auswahl, Kopie und JSON-Bytes (`serde_json::to_vec` über serialisierbare Strukturen, kein `Value`-Baum); der Handler gibt die fertigen Bytes mit `Content-Type: application/json` zurück. Die Werkbank ist **dieselbe** Semaphore wie für große Körper (ADR-0010, Ergänzung Regel 2): ein Typ im Zustand (`Werkbank`), Erlaubnisse = `available_parallelism`, mindestens 2.
5. **Rückhalt Blocking-Pool**: die Laufzeit wird mit `max_blocking_threads(64)` gebaut (statt 512) — `#[tokio::main]` wird zum Runtime-Builder in `main.rs`. 64 = so viele Plätze wie eine Herkunft Verbindungen hat; leichte Blocking-Arbeit (Channel-Stand, Plattenschreiben) bleibt ohne Werkbank.
6. **Sendefrist = Abrissgrenze (2H)** für **jede** Route: wer 2H lang nichts abnimmt (Nullfenster) oder nichts bestätigt (stiller Abriss), dessen Verbindung schließt der Kernel; hyper beendet die Antwort, der Platz wird frei. Kein eigener Knopf, keine Ausnahmeliste: ein gesunder Ereignisstrom schreibt nie gegen ein volles Fenster, ein Warte-Aufruf (I0016) schreibt während des Wartens nichts. Die Kopf-Zeitgrenze (10 s, ADR-0010) bleibt davon getrennt (sie läuft nur, wenn ein Kopf erwartet wird). Der Health-Check nennt die Abrissgrenze schon (`abrissgrenze_ms`); er nennt sie ab jetzt auch als Sendefrist in der Beschreibung des Betriebs (README/Hilfe), nicht als neues Feld.
7. **Allgemein für große Antworten** (gilt ab jetzt für jede Route, Durchsetzung unten):
   - a. Jede Route, deren Antwort über **64 KiB** wachsen kann, nennt ihre Obergrenze mit Herleitung als Konstante neben dem Handler (größter Inhalt × Schreibweise, Kontrollpaar aus realistischen Daten mit ≥ 30 % Reserve).
   - b. **Listen** (Verlauf, Direktgespräche I0012, Suchtreffer I0011, Warte-Aufruf I0016) schneiden nach Budget mit Cursor und Fortsetzungs-Flag, mindestens ein Element; Suchtreffer tragen Ausschnitte, nicht ganze Texte, und das Budget gilt für die Trefferliste wie für den Block.
   - c. **Dateien** (Anhänge I0010) werden nie ganz in den Speicher gelesen: aus der Datei in Stücken (64 KiB) mit Gegendruck gestreamt; Speicher je Verbindung ≈ ein Stück plus Puffer von hyper. Ein Anhang ist nie Teil einer Liste — die Liste trägt Metadaten.
   - d. **Schwere Arbeit** (über ≈ 1 ms oder über 64 KiB Kopie) nur auf der Werkbank; die Antwort entsteht dort als Bytes.
   - e. **Sendefrist** ist die Abrissgrenze (Regel 6); eine Route, die bewusst länger als 2H nichts abnehmen lassen will, gibt es nicht.

**Rechnung Q8** (Zweck: ein Rechner im LAN kann den Speicher nicht durch Nicht-Lesen treiben; Startpunkt: die 64 Anfragen sind gesendet): je Verbindung höchstens eine ungesendete Antwort ≤ 2 × 256 KiB + 15 × 60 Bytes ≈ 0,5 MiB → 64 × 0,5 MiB = 32 MiB; dazu höchstens Kerne × Arbeitsspitze je Block (≈ 3 × 0,5 MiB) auf der Werkbank; Grundlast und Puffer von hyper/Kernel als 50 MB Zuschlag. **Grenze RSS-Zuwachs ≤ 64 × 2 × Budget + 50 MB = 32 MiB + 50 MB ≈ 84 MB** (1 Herkunft), 2 Herkünfte ≤ 64 MiB + 50 MB ≈ 117 MB. Erwartet nach Probe `n=15` (ohne Werkbank, einzeilige Texte = 1 × Budget): ≈ 32 MB — 2,6-fache Reserve. Nach dem Schließen: RSS ≤ Start + 50 MB (Probe `n=15`: + 22 MB).

## Konsequenzen

**Positiv:**
- Der Speicher je Verlaufsabruf ist eine feste Zahl, unabhängig von Nachrichtengröße und `n`; Probe `n=15` als Stellvertreter: 40 MB statt 754 MB, nach dem Schließen 30 statt 169 MB.
- Die Sendefrist gibt es schon — sie wird ein benannter, getesteter Vertrag statt eines Nebeneffekts.
- Ein Block aus großen Nachrichten öffnet im Browser in ≈ 45 ms statt ≈ 280 ms (ADR-0034, Probe).
- I0010 und I0011 erben Budget, Werkbank und Sendefrist (Regel 7).

**Negativ / Inkauf zu nehmen:**
- `n` ist nur noch Obergrenze; ein Agent, der „weniger als `n` heißt Ende“ annimmt, hört zu früh auf (Anleitung I0019; 422-Fehlertexte bleiben unverändert).
- Kanäle aus großen Nachrichten brauchen mehr Abrufe beim Zurückscrollen (15 statt 100 Nachrichten je Abruf) — je ≈ 12 ms, im Vorlauf von 2 Sichthöhen unsichtbar.
- Die Fenster-Regel im Browser braucht einen Schutz gegen Hin-und-her-Laden, wenn Blöcke niedrig sind ([ADR-0026](0026-verlauf-im-browser-als-fenster-aus-hoechstens-fuenf-bloecken.md), Ergänzung).
- Sendefrist nur unter Linux ≥ 5.11; ein Tropfleser über echtes LAN ist nicht geprobt.
- Unter einer Flut warten echte Leser in der Schlange der Werkbank (≈ 8 ms bei 64 Abrufen).

**Durchsetzung (Makro, Regel 7):** Review-Punkt für `code-reviewer` und `security-reviewer` — jede neue Route mit Antwort über 64 KiB nennt ihre Obergrenze mit Herleitung, jede Liste hat Budget und Fortsetzungs-Flag, jedes `spawn_blocking` mit großer Arbeit läuft unter der Werkbank; die Integrationstests K4/K5 (unten) laufen gegen das echte Programm und werden rot, wenn jemand Budget, Werkbank oder Pflege entfernt.

**Pre-Mortem (ein Monat nach I0009 gescheitert, weil …):**
1. … ein Agent nach dem ersten kurzen Block aufhört zu lesen, weil er Nachrichten zählt. Früh-Indikator: Agent meldet „Ende“ mitten im Verlauf eines Channels mit großen Nachrichten. Gegenmaßnahme: Regel 3, Anleitung I0019, Integrationstest K2 (Weiterblättern nur über `aelter`).
2. … der Browser an einem 4K-Schirm in einem Channel aus großen Nachrichten endlos oben und unten nachlädt. Früh-Indikator: Abrufe ohne Scrollen. Gegenmaßnahme: ADR-0026, Ergänzung (Abwurfschutz), Test K8.
3. … die Kernel-Grenze in einer anderen Umgebung (Container mit älterem Kernel, andere Plattform) nicht greift und der Befund stumm wiederkommt. Früh-Indikator: K5 rot in dieser Umgebung. Gegenmaßnahme: K5 läuft im Gate; eine andere Plattform braucht eine neue Entscheidung (Option 2B liegt bereit).
4. … die Werkbank unter Last Q1 reißt, weil Suche (I0011) sie lange belegt. Früh-Indikator: L1 im Gate mit gleichzeitiger Suche über 50 ms. Gegenmaßnahme: Suche bekommt ein eigenes Arbeitsbudget je Aufruf; falls nötig 3C (k je Herkunft) für die Suche.

**Folgeentscheidungen / Aufgaben** (Kriterien für den `planer`, Bug-Zuschnitt „aus-bug“; Messwerte mit Reserve, Wertprüfungen als Spanne mit Mutanten):

| # | Was | Art | Eingabe / Messweg | erwartet | Grenze / Spanne | Mutanten, die rot werden |
|---|---|---|---|---|---|---|
| K1 | Budget im Index | Unit (rein, Verlaufsindex) | gebaute Sätze mit bekannter Satzlänge, Budget als Parameter `B`: (a) 4 Sätze, Summe genau `B`, `n=200` → alle 4; (b) Summe `B + 1` → 3, Richtung neueste `aelter: true`, `nach` gespiegelt `neuer: true`; (c) ein Satz `B + 100` allein → 1; (d) kleine Sätze, `n=2` → 2 | wie links | genau | Budget `B − 1` (a rot), `B + 1` (b rot), „mindestens 1“ entfernt (c rot), `n` übergangen (d rot); Konstante `VERLAUF_BUDGET` = 262.144 in einem eigenen Test genau |
| K2 | Budget über HTTP | Integration (echtes Programm, Debug) | 200 × 16.384 `x` an einem Tag; Satzlänge `s` aus der Datei | `GET ?n=200` → ⌊262.144 / s⌋ Nachrichten (15), die neuesten, `aelter: true`; weiter mit `vor=<erste Zeit>` nur solange `aelter` → genau 200 Zeiten lückenlos, streng steigend; ebenso vorwärts über `nach` bis `neuer: false`; `n=1` → 1 | genau | Budget ± 1 über K1; „Flag aus Anzahl“ → Abbruch zu früh |
| K3 | Kontrollpaar | Integration (bestehendes L1-Fixture, 100.000 realistisch) | `n=200` an 20 Stellen inkl. des größten Fensters (Probe 3: 80,7 KiB); `n=100` neueste | 200 bzw. 100 | genau | Budget auf 64 KiB → rot |
| K4 | Speicher bei langsamen Lesern (Q8) | **Messwert**, Abnahme-Probe des `security-reviewer`, Release-Build | Fixture wie K2; je Herkunft 64 Rohverbindungen, `SO_RCVBUF` 4 KiB, `GET ?n=200` ohne Lesen; RSS aus `/proc` vor und 5 s danach; Varianten `n=1/100/200`, neueste/`vor`/`nach`, 1 und 2 Herkünfte (`127.0.0.2`) | ≈ + 32 MB je Herkunft; Threads ≤ Basis + 64 | ≤ + 84 MB (1 Herkunft), ≤ + 117 MB (2); 15 s nach dem Schließen ≤ Start + 50 MB | heute rot: + 746 MB / + 1.020 MB, nach dem Schließen + 161 / + 483 MB |
| K5 | Sendefrist = Abrissgrenze | Gate (Integration, echtes Programm, Linux), `LOCALCHATA_HERZSCHLAG_MS=2000` (2H = 4 s) | 4 Rohverbindungen wie K4; Startpunkt: Anfrage gesendet; Ende: server-seitig zu (Rohsocket bekommt beim nächsten Senden `EPIPE`/RST oder `ss` ohne Eintrag — der Entwickler wählt einen beobachtbaren Weg, nicht `sleep`) | ≈ 4,3 s (Probe 2: Grenze + 0,3 s) | in **[3,6 s; 5,6 s]** | Faktor 1 (≈ 2,3 s) und 3 (≈ 6,3 s) rot, Pflege nicht angewandt → offen nach 5,6 s rot; Frist ± 1 ms über den vorhandenen Unit-Test `pflege_aus` (ADR-0017) |
| K6 | Strom unberührt | Gate (Integration), `LOCALCHATA_HERZSCHLAG_MS=2000` | lesender Client hält `GET /api/ereignisse` | Herzschläge kommen, Strom offen | nach 3 × 2H = 12 s noch offen, ≥ 5 Herzschläge | eine Gesamtfrist je Antwort (Option 2D ohne Ausnahme) → rot |
| K7 | Werkbank | Unit/Integration (In-Process mit `ausliefern`) | Werkbank mit 1 Erlaubnis, die der Test hält; `GET …/nachrichten` | keine Antwort, solange gehalten; Antwort < 1 s nach Freigabe | genau | Erwerb entfernt → antwortet sofort → rot |
| K8 | Browser ohne Hin-und-her | Node-Unit (`e2e/einheit/`) + E2E | ADR-0026, Ergänzung | siehe dort | siehe dort | „immer abwerfen“ → rot |

- [ ] `max_blocking_threads(64)` (Regel 5): Review-Punkt; K4 nennt die Thread-Zahl.
- [ ] I0019 (Anleitung für Agenten): „`n` ist eine Obergrenze — weiter, solange `aelter`/`neuer`“.
- [ ] Messwert **Tropfleser über echtes LAN** (zweites Gerät oder Netz-Namespace mit Rechten wie ADR-0017): liest alle 2 s 4 KiB eines 512-KiB-Blocks. Hält er die Verbindung über 60 s, ist das kein Befund (≤ 0,5 MiB je Platz), aber Anlass, Option 2C für Anhänge (I0010) neu zu prüfen.

## Quellen / Bezuege

- [A00004 HTTP-Schnittstelle](../Architektur/A00004-http-schnittstelle.md) (primär), [A00009 Verlaufsindex](../Architektur/A00009-verlaufsindex.md), [A00003 Start](../Architektur/A00003-start.md), [A00005 Weboberfläche](../Architektur/A00005-weboberflaeche.md), [A00001 localchatA](../Architektur/A00001-localchata.md) (Q8); Aspekt [X00008](../Architektur/Aspekte/X00008-transaktionen-und-nebenlaeufigkeit.md)
- ADR-0004, ADR-0005, ADR-0006, ADR-0010 (Ergänzung R00010, Werkbank), ADR-0017 (Abrisspflege), ADR-0025, ADR-0026, ADR-0030, ADR-0034, ADR-0036
- `src/http/nachrichten.rs`, `src/start/ausliefern.rs`, `src/start/abrisspflege.rs`, `src/main.rs`, `src/gespraeche/logformat.rs`, `examples/verlauf-erzeugen.rs`, `web/fenster.js`, `web/verlauf.js`
- Linux `tcp(7)` (`TCP_USER_TIMEOUT`); Kernel-Änderung „tcp: fix TCP_USER_TIMEOUT with zero window“ (Linux 5.11) — aus dem Gedächtnis, hier durch Probe (2) auf 6.14 belegt; tokio `runtime::Builder::max_blocking_threads` (Standard 512)
- Security-Review #7 (Befund Karte #23)

## Ergänzungen

- **2026-10-28 (R00015, F0056; Lastprofil „Team-LAN“) — Rechnung Q8 mit 160 Plätzen je Herkunft:** Die Grenze je Gegenstelle steigt auf 160 ([ADR-0010](0010-eigene-annahmeschleife-hyper-http1.md), Ergänzung R00015). Die Rechnung Q8 bleibt dieselbe, nur die Zahl der Plätze wächst: je Verbindung ≤ 2 × 256 KiB ≈ 0,5 MiB → **1 Herkunft: 160 × 0,5 MiB = 80 MiB ≈ 83,9 MB + 50 MB ≈ 134 MB** (statt 84 MB); **2 Herkünfte: 320 × 0,5 MiB = 160 MiB ≈ 167,8 MB + 50 MB ≈ 218 MB** (statt 117 MB). Erwartet linear zur Probe `n=15`: ≈ + 80 MB bzw. + 160 MB. Unverändert: Threads ≤ Basis + 64 (`max_blocking_threads(64)`, Regel 5), Sendefrist = Abrissgrenze, Rückgabe nach dem Schließen ≤ Start + 50 MB. Gate K4 wird mit 160 nicht lesenden Abrufen je Herkunft wiederholt (R00015 G5); ein Wert über der Grenze ist ein Befund an diese ADR bzw. an die Anhebung, kein Grund, die Grenze zu ändern. Die Grenzen in der Tabelle der Gates (K4) lesen sich ab R00015 „≤ + 134 MB (1 Herkunft), ≤ + 218 MB (2)“.
- **2026-10-28 (Observability-Review #23, Katalog-Lücke) — die Sendefrist hat eine Spur:** Eine Verbindung, die der Kernel an der Abrissgrenze schließt (Regel 6), ist ab jetzt eine `WARNING verbindung`-Episode je Herkunft und zählt in `abgerissen` im Health-Check; Gate K5 prüft beides mit. Entscheidung und Optionen: [ADR-0042](0042-stufen-ereigniskatalog-episoden-und-tagesgrenze-des-betriebsprotokolls.md), Ergänzung #24 Punkt 7; [ADR-0043](0043-protokollstufe-einstellen-und-kennzahlen-im-health-check.md), Ergänzung #24. Ebenso: ein Abbruch, weil die Werkbank geschlossen ist, trägt `grund=werkbank_geschlossen` (ADR-0042, Ergänzung #24 Punkt 6).
