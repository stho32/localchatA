# ADR-0044: Plattenquote — die Gesamtgröße der Gespräche ist begrenzt (`--plattenquote`, Standard 4 GiB), gezählt am Verlaufsindex; an der Quote wird Senden mit 507 abgewiesen, gelöscht wird deswegen nichts

**Status**: Akzeptiert
**Datum**: 2026-10-29
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #34); Richtung vom Betreiber 2026-10-29: „Ja, als Quote — die Gesamtgröße der Gespräche ist auf einen einstellbaren Wert begrenzt.“
**Kontext-Anforderung**: — (noch keine; Vorschlag für den planer im Bericht Karte #34: neue Interaction unter D0001)
**Erprobt durch**: — (überträgt gebaute Mechanik: der Verlaufsindex hält je Tag den Rohtext der Datei und kennt seine Bytes, `src/gespraeche/verlaufsindex.rs:54` `bytes`, `:60` `tag_entfernen` liefert die Bytes des entfernten Tags; `log_bytes` im Health-Check ist heute schon Σ Index, `src/gespraeche/mod.rs:342–355`; Senden prüft unter der Schreibsperre, `src/gespraeche/mod.rs` `senden_mit`; Optionen mit Umgebungswert und Exit 2 wie [ADR-0043](0043-protokollstufe-einstellen-und-kennzahlen-im-health-check.md), `src/start/aufruf.rs`; der Browser zeigt den `fehler` jeder Antwort außer 500 an der fehlgeschlagenen Nachricht, `web/zustellung.js:79–88`)

## Kontext

- **Betreiber 2026-10-29** auf die Frage aus [ADR-0036](0036-schreibkonto-token-eimer-je-kennung-und-je-herkunft.md), Ergänzung Karte #31 (a): die Platte wird **gedeckelt**, als Quote auf die Gesamtgröße der Gespräche, einstellbar.
- Heute nur **verlangsamt**: eine Herkunft schreibt im schlimmsten Fall ≈ 66 KB/s ≈ **5,7 GB/Tag** in die Tagesdateien (ADR-0036, Ergänzung Karte #31 (a)); mit der Standardfrist hält eine dauerhaft flutende Herkunft ≈ 40 GB, mit 3.650 Tagen Frist unbegrenzt.
- **Der Index ist der Speicher**: [ADR-0006](0006-verlaufsindex-im-speicher.md) hält den Rohtext jedes Tages in der Frist im Arbeitsspeicher (heute ≈ 1 × Loggröße, mit der geplanten Suchkopie ≈ 2 ×). 40 GB Log heißen 40–80 GB Speicher — die Instanz stirbt am Speicher, lange bevor die Platte voll ist. Eine Quote auf die Gespräche ist deshalb zugleich die Grenze des Speichers.
- **Lastprofil „Team-LAN“** (Vision, Leitplanke; `AI-Team/umgebung.md`): dauerhaft 5 Nachrichten/s × ≈ 530 B ≈ **230 MB/Tag** ([ADR-0042](0042-stufen-ereigniskatalog-episoden-und-tagesgrenze-des-betriebsprotokolls.md) Regel 6); bis 100.000 Nachrichten je Channel, 50 Channels aktiv.
- **Vision**: „Endgültig entfallen Nachrichten nur über die Aufbewahrungsfrist — es gibt keine zweite Löschhandlung.“ Archivierte Channels (I0022, WBS) wandern nach `archiv/<name>/`, „der Verlauf bleibt auf der Platte lesbar“.
- **Bestand der Ablage** ([ADR-0003](0003-ablage-layout-im-ordner.md)): `channels/<name>/JJJJ-MM-TT.log`, `channels/<name>/einstellungen.toml`, `direkt/…` (I0012), `anhaenge/<tag>/` (I0010), `archiv/<name>/` (I0022), dazu `logs/` des Betriebsprotokolls mit eigener Bereinigung (14 Tage) und eigenem Tagesbudget (höchstens 128 MiB/Tag, 1,75 GiB gesamt; ADR-0041, ADR-0042 Regel 6).
- **Sperrordnung** (A00006, Ergänzung Karte #8): Verzeichnis → Gespräch → Schreibsperre; die Pflege nimmt die Schreibsperre nie.
- **Health-Check** ([ADR-0043](0043-protokollstufe-einstellen-und-kennzahlen-im-health-check.md), Ergänzung Karte #31): neue Felder nur, wenn das Monitoring ohne Protokoll etwas **entscheiden** muss — ob und wie oft, keine Aufteilungen.

Szenarien mit Maß:

- **Q1 Normalbetrieb**: das Lastprofil über eine Woche (≈ 1,6 GB) und der Bestand des Profils (50 Channels × 100.000 Nachrichten × 530 B ≈ 2,65 GB) laufen ohne Warnung und ohne Abweisung.
- **Q2 Flut**: eine Herkunft flutet dauerhaft (≈ 5,7 GB/Tag) — Platte und Speicher der Gespräche überschreiten die Quote nie; der Betreiber erfährt es Stunden, bevor Senden scheitert.
- **Q3 Quote erreicht**: wer sendet, bekommt sofort eine Antwort, die den Grund und den Ausweg nennt; Lesen, Anlegen, Beitreten, Frist bleiben möglich; nichts wird wegen der Quote gelöscht.
- **Q4 Kosten**: die Prüfung kostet je Senden O(1) und keine Plattenzugriffe; der Start zählt ohne zusätzliches Lesen.

## Optionen

### Frage 1: Was geschieht an der Quote?

#### Option A: Senden abweisen, weiche Warnschwelle vorher (gewählt)
- Vorteile: die einzige Variante, die „begrenzt“ wörtlich erfüllt, ohne neben der Frist zu löschen; Bestehendes bleibt unangetastet; Platz entsteht nur über die Frist (Tageswechsel, Verkürzen) oder eine größere Quote.
- Nachteile: ist die Quote voll, ist die Instanz für **alle** stumm, bis Platz entsteht — im schlimmsten Fall bis zum nächsten Tageswechsel, bei langen Fristen länger.

#### Option B: Ältestes zuerst löschen (Ringpuffer)
- Vorteile: Senden scheitert nie.
- Nachteile: eine zweite endgültige Löschhandlung neben der Frist — von der Vision ausgeschlossen; ein Flutender löscht die Gespräche der anderen.

#### Option C: Frist an der Quote automatisch verkürzen
- Vorteile: nutzt den vorhandenen Löschweg.
- Nachteile: dieselbe zweite Löschhandlung, nur anders ausgelöst; die Frist, die ein Teilnehmer gewählt hat, gälte nicht mehr.

#### Option D: Nur warnen (weiche Quote)
- Vorteile: nie stumm.
- Nachteile: keine Begrenzung — widerspricht der Entscheidung des Betreibers; Q2 nicht erfüllt.

#### Option E: Quote je Herkunft (Anteil an der Gesamtquote)
- Vorteile: ein Flutender füllt nur seinen Anteil; die anderen senden weiter.
- Nachteile: der Bestand muss je Herkunft über Tage und Neustarts zugeordnet werden (beim Laden aus jeder Kopfzeile); viele IPv6-Adressen eines Geräts umgehen den Anteil (wie bei ADR-0021 und ADR-0036); eine Factory auf `127.0.0.1`, die legitim den meisten Verkehr erzeugt, stößt zuerst an. Mechanik ohne Bedarf im Profil — bleibt als Folgeoption, falls Pre-Mortem 1 eintritt.

| | Q1 Normal | Q2 Flut | Q3 an der Quote | Vision | Schwäche |
|---|---|---|---|---|---|
| A | frei | gedeckelt, Warnung vorher | abgewiesen mit Ausweg | hält | stumm für alle bis Platz entsteht |
| B | frei | gedeckelt | Senden geht, Altes weg | verletzt | zweite Löschhandlung |
| C | frei | gedeckelt | Senden geht, Frist gekürzt | verletzt | zweite Löschhandlung |
| D | frei | **nicht** gedeckelt | Senden geht | hält | keine Grenze |
| E | frei | gedeckelt je Herkunft | nur der Flutende stumm | hält | Zuordnung, IPv6, Factory zuerst |

### Frage 2: Wie wird gemessen?

#### Option M1: Ein Zähler neben dem Verlaufsindex (gewählt)
Belegt = Σ Bytes des Rohtexts aller Tage im Index; beim Laden aufsummiert (die Bytes werden ohnehin gelesen), je erfolgreichem Anhängen um die Bytes des Satzes erhöht, beim Entfallen um die Bytes des entfernten Tags vermindert (`Verlauf::tag_entfernen` liefert sie schon).
- Vorteile: O(1) je Senden; exakt das, was die Instanz hält — Platte **und** Speicher; mit dem Index unter denselben Sperren gepflegt; `log_bytes` im Health-Check ist heute schon dieselbe Größe und wird O(1).
- Nachteile: sieht nur, was im Index steht — von Hand gelöschte oder hinzugefügte Dateien zählen erst nach einem Neustart; alte Tage eines Channels mit defekten Einstellungen (nicht geladen, nicht gelöscht, [ADR-0032](0032-loeschen-nur-tagesdateien-nach-erlaubnisliste-keine-symlinks-defekt-heisst-nichts.md)) zählen nicht.

#### Option M2: Σ über den Index bei jedem Senden (wie `kennzahlen()` heute)
- Vorteile: kein zusätzlicher Zustand, kann nicht abdriften.
- Nachteile: O(Gespräche × Tage) je Senden; nimmt unter der Gesprächs- und Schreibsperre die Sperren **anderer** Gespräche — verletzt die Sperrordnung (Verklemmung mit einer Pflege).

#### Option M3: Plattenzähler als Hülle um die Logablage (Decorator des Ports)
- Vorteile: fängt jeden Schreibweg der Ablage ab, auch künftige (Anhänge); zählt die Platte, nicht den Speicher.
- Nachteile: die Entscheidung „zu groß“ wäre Fachlogik im Provider (A00008: „ohne Fachlogik“); der Start müsste alle Dateien messen statt der geladenen; Platte und Speicher können auseinanderlaufen, und der Speicher ist der engere Rand.

#### Option M4: Ordner zyklisch vermessen (`du` im Sekundentakt) oder freien Platz prüfen (`statvfs`)
- Vorteile: sieht Handarbeit im Ordner; freier Platz schützt auch vor fremden Dateien.
- Nachteile: Verzeichnislauf über bis zu 256 Channels je Takt; `statvfs` braucht `libc`/unsafe und misst nicht „die Gespräche“, sondern die ganze Platte — eine andere Frage als die des Betreibers.

### Frage 3: Einstellung

- **Option S1** (gewählt): Option `--plattenquote <größe>` und Umgebungswert `LOCALCHATA_PLATTENQUOTE`, gelesen beim Start, Option vor Umgebung vor Standard; Größe als ganze Zahl **mit Einheit** `MiB` oder `GiB`, oder das Wort `aus`. Stärke: wie [ADR-0043](0043-protokollstufe-einstellen-und-kennzahlen-im-health-check.md); kein Rätsel „Bytes oder Gigabyte?“. Schwäche: Ändern heißt Neustart.
- **Option S2**: Zahl ohne Einheit in GiB. Stärke: kürzer. Schwäche: kleine Quoten für Tests (1 MiB) unmöglich; `4` liest sich auch als Bytes.
- **Option S3**: Einstellung in einer Datei im Ordner (`localchatA.toml`). Stärke: wandert mit dem Ordner. Schwäche: eine zweite Konfigurationsquelle, die es bisher nicht gibt (X00006); jeder mit Platte-Zugriff könnte sie ändern, ohne dass die Instanz es merkt.
- **Option S4**: über eine Route zur Laufzeit. Schwäche: ohne Anmeldung kann jeder im LAN die Quote aufheben (Muster ADR-0043 Option B).
- **0 = aus?** verworfen: im Haus ist 0 ein Fehler (`LOCALCHATA_HERZSCHLAG_MS`, `LOCALCHATA_EPISODE_RUHE_MS`), und „Quote 0“ liest sich als „nichts erlaubt“. Ausgeschaltet wird mit dem Wort `aus`.

### Frage 4: Status der Abweisung

- **507 Insufficient Storage** (gewählt; RFC 4918 Abschnitt 11.5 — aus dem Gedächtnis, in diesem Auftrag nicht geöffnet): „der Server kann die Darstellung nicht speichern“ — genau der Fall; ein Agent wartet nicht darauf, dass es von selbst besser wird. Schwäche: ein 5xx, also zählt ein Monitoring, das nur 5xx zählt, sie als Serverfehler — dafür gibt es `quote_abgewiesen`.
- **429 mit `Retry-After` bis Mitternacht**: Stärke: Agenten warten schon auf 429 (`lc senden`, I0019). Schwäche: Warten hilft nur, wenn um Mitternacht etwas entfällt — bei langen Fristen nicht; 429 heißt im Haus „du bist zu schnell“ (ADR-0036), hier ist niemand zu schnell.
- **503 mit `Retry-After`**: Schwäche: Lesen, Anlegen, Frist laufen weiter — die Instanz ist nicht „nicht verfügbar“.
- **409/422**: Schwäche: die Anfrage ist richtig; ein 4xx sagte dem Sender, er solle sie ändern.

## Entscheidung

**Gewählt: A (abweisen, Warnschwelle), M1 (Zähler neben dem Index), S1 (`--plattenquote`, Standard 4 GiB, `aus`), 507.**

Es überwiegt, dass nur das Abweisen die Quote einhält, ohne eine zweite Löschhandlung neben der Frist einzuführen, und dass der Zähler am Index genau das misst, woran die Instanz zuerst stirbt (Speicher), mit Kosten O(1) und unter den Sperren, die das Senden ohnehin hält. In Kauf genommen: an der Quote ist die Instanz für alle stumm, bis eine Frist Platz schafft oder der Betreiber die Quote anhebt — auch wenn ein Einzelner sie gefüllt hat (Pre-Mortem 1); Handarbeit im Ordner wird erst nach einem Neustart gezählt; Dateien, die die Instanz nicht lädt, zählen nicht.

Regeln:

1. **Was zählt** — „belegt“ sind die Bytes, die die Instanz von den Gesprächen hält:
   - **zählt**: die Tagesdateien aller Channels und Direktgespräche, soweit im Verlaufsindex (also alle Tage in der Frist; ein Tag, dessen Löschen scheiterte, bleibt nach [ADR-0030](0030-entfallen-beim-start-beim-tageswechsel-und-beim-verkuerzen-loeschen-unter-der-gespraechssperre.md) Regel 4 im Index und zählt weiter);
   - **zählt, sobald gebaut**: Anhänge (I0010) mit den Bytes der gespeicherten Datei; archivierte Channels (I0022) mit den Bytes ihrer Tagesdateien und Anhänge — Archivieren verschiebt Bytes, es schafft keinen Platz (sonst wäre „voll → archivieren → weiter fluten“ ein Gegenzug, Lehre #5). Beide Interactions bringen ihren Anteil am Zähler mit (Folgeentscheidungen);
   - **zählt nicht**: `einstellungen.toml` (höchstens 256 Channels × weniger als 1 KB, durch ADR-0021 gedeckelt); `logs/` (eigenes Budget und eigene Bereinigung, ADR-0042 Regel 6, ADR-0041); Dateien, die die Instanz nicht lädt (ungültige Namen, Tage außerhalb der Frist in Channels mit defekten Einstellungen, ADR-0032), und `.localchatA.sperre`.
2. **Zähler** `belegt: AtomicU64` im Baustein [Gespräche](../Architektur/A00006-gespraeche.md), einziger Halter:
   - **Laden**: Σ der Rohtext-Bytes je geladenem Tag (die Bytes liegen beim Laden ohnehin vor).
   - **Senden**: unter Gesprächs- **und** Schreibsperre, nach der Prüfung des Wiederholschlüssels und nach dem Bilden des Satzes, **vor** dem Anhängen: `belegt + Bytes(Satz) > hoechstens` → `Sendefehler::Quote { belegt, hoechstens }`, nichts angehängt, nichts veröffentlicht, die vergebene Zeit verfällt (wie beim Plattenfehler). Sonst anhängen; erst nach Erfolg `belegt += Bytes(Satz)` (+ 1, wenn die Logablage ein Zeilenende ergänzt hat, ADR-0011 — dieses eine Byte kann die Quote nach Handarbeit überschreiten; hingenommen). Eine **Wiederholung mit bekanntem Schlüssel** antwortet vor der Prüfung mit der ursprünglichen Nachricht (200) — sie schreibt nichts.
   - **Entfallen**: je aus dem Index genommenem Tag `belegt −= Bytes` (aus `Verlauf::tag_entfernen`), atomar ohne Schreibsperre — die Pflege nimmt sie nie (A00006). Ein gleichzeitiges Abziehen macht eine laufende Prüfung höchstens strenger, nie lascher; jedes Hinzufügen geschieht unter der Schreibsperre, also überholen sich zwei Sendungen nie über die Quote.
   - **Invariante**: `belegt = Σ Verlauf::bytes()` über alle Gespräche, zu jedem Zeitpunkt ohne laufende Operation. `log_bytes` im Health-Check liest künftig den Zähler (O(1)) statt die Summe zu bilden.
3. **Einstellung** `--plattenquote <größe>` bzw. `LOCALCHATA_PLATTENQUOTE`:
   - Werte: ganze Zahl mit Einheit `MiB` oder `GiB` (Groß-/Kleinschreibung egal, ohne Leerzeichen: `4GiB`, `512mib`), Spanne **1 MiB bis 1.024 GiB**; oder `aus`. Ohne Einheit, mit `MB`/`GB`, `0…`, außerhalb der Spanne, leer oder keine Zahl → Meldung auf stderr mit Beispiel (`--plattenquote braucht eine Größe wie 4GiB oder 512MiB, oder aus`), **Exit 2**, nichts geschrieben, kein `logs/` angelegt (ADR-0043 Regel 2). Fehlt der Wert hinter der Option → `--plattenquote braucht einen Wert — Aufruf siehe localchatA --hilfe`.
   - Vorrang Option vor Umgebung vor Standard; gelesen nur beim Start.
   - `--hilfe`: `--plattenquote <größe>  Höchstgröße aller Gespräche (Tageslogs der Channels und Direktgespräche) wie 4GiB oder 512MiB, aus für keine Grenze; Standard 4GiB; auch über LOCALCHATA_PLATTENQUOTE`.
   - **Standard 4 GiB.** Herleitung: Normalwoche ≈ 1,6 GB (230 MB/Tag × 7 Tage Standardfrist) und Bestand des Profils ≈ 2,65 GB (50 × 100.000 × 530 B) liegen unter der Warnschwelle von 3,6 GiB (Q1); eine flutende Herkunft (≈ 66 KB/s) braucht vom Normalstand bis zur Quote ≈ 11 h, zwei ≈ 5,5 h — Zeit für den Betreiber zwischen Warnung und Stillstand (Q2); Speicher des Index ≤ 4 GiB heute, ≤ 8 GiB mit Suchkopie — auf einem normalen Rechner im LAN tragbar; mit `logs/` (≤ 1,75 GiB) belegt die Instanz höchstens ≈ 6 GiB Platte. Gerechnet, nicht gemessen.
4. **Antwort an der Quote** — `POST /api/channels/{name}/nachrichten` (und künftig jede Route, die Gesprächsbytes schreibt): **507** `{fehler, regel: "quote", belegt, hoechstens}` ohne `Retry-After`. Stelle in der Prüfreihenfolge von [ADR-0034](0034-nachricht-senden-post-nachrichten-text-hoechstens-16-kib-steuerzeichen-abweisen-wiederholschluessel.md) Regel 3: in Schritt 9 (Senden), also **nach** Vorab-Buchung, Körpergrenze (413), allen 4xx und der Restbuchung am Schreibkonto (429) — die gebuchten Einheiten werden nicht erstattet (wie beim 500); eine Schleife auf 507 wird darum vom Schreibkonto gebremst. Text (Zahlen in GiB mit einer Nachkommastelle, unter 1 GiB in MiB, deutsches Komma):
   „Nicht gesendet: die Gespräche belegen 4,0 GiB, die Quote dieser Instanz ist 4,0 GiB. Platz entsteht, wenn alte Tage nach ihrer Aufbewahrungsfrist entfallen (beim nächsten Tageswechsel) oder eine Frist verkürzt wird; die Quote stellt der Betreiber beim Start mit --plattenquote ein.“
   Der Browser zeigt ihn an der fehlgeschlagenen Nachricht wie jeden `fehler` außer 500 (`web/zustellung.js:79–88`), „Erneut senden“ bleibt (F0046).
5. **Was an der Quote weiter geht**: Lesen, Ereignisstrom, Beitreten und Verlassen (schreiben keine Gesprächsbytes); **Channel anlegen** (nur Ordner und Einstellungen, gedeckelt durch ADR-0021); **Frist setzen** — auch Verlängern: die Quote begrenzt Bytes, nicht Zeit; Verlängern bindet vorhandenen Platz länger, schreibt aber nichts. Verkürzen ist der eine Weg, auf dem Teilnehmer Platz schaffen, mit Bestätigung ([ADR-0031](0031-frist-setzen-put-api-channels-name-frist-jeder-mit-sitzung-bestaetigung-beim-verkuerzen.md)).
6. **Nichts wird wegen der Quote gelöscht.** Platz entsteht nur über die Frist (Tageswechsel, Verkürzen, Start) oder eine größere Quote beim Neustart. Kein „ältestes zuerst“, kein automatisches Verkürzen (Vision: keine zweite Löschhandlung).
7. **Warnschwelle 90 %** (Konstante, nicht einstellbar) als **Zustandswechsel**, nicht als zeitliche Episode — der Zustand hält Tage, eine Episode endete nach 60 s Ruhe und begänne mit jeder Nachricht neu:
   - steigt `belegt` beim Senden über 90 % der Quote → einmal `WARNING quote Gespräche belegen 90 % der Quote | belegt=<bytes> hoechstens=<bytes>`;
   - fällt `belegt` beim Entfallen wieder unter 90 % → einmal `INFORMATION quote Gespräche wieder unter 90 % der Quote | belegt=… hoechstens=… grund=<anlass>` (`start` | `tageswechsel` | `frist_gesetzt`).
   Über 90 % geht es nur durch Senden, darunter nur durch Entfallen — also höchstens ein Wechsel je Pflege; kein Flattern, keine Hysterese nötig.
   - **Start**: liegt `belegt` schon über 90 % bzw. über der Quote, schreibt der Start nach dem Laden einmal `WARNING quote Gespräche belegen 90 % der Quote` bzw. `WARNING quote Gespräche belegen mehr als die Quote — Senden wird abgewiesen` (mit `belegt`, `hoechstens`); geladen wird trotzdem alles (die Quote begrenzt das Wachsen, nicht das Laden).
   - Der Start-Eintrag (`INFORMATION start`) nennt zusätzlich `plattenquote=<bytes>|aus`, `belegt=<bytes>` und woher die Quote kommt (`option=--plattenquote|LOCALCHATA_PLATTENQUOTE|standard`, wie ADR-0043 Regel 4).
8. **Abweisungen im Protokoll**: `WARNING quote Senden abgewiesen: Quote erreicht` als **Episode je Kennung** (ADR-0042 Regel 5); Kontext `kennung`, `channel`, `belegt`, `hoechstens`; die Ende-Zeile zählt die Abweisungen. Einzelne 507 zusätzlich als `DEBUG` wie jede Abweisung.
9. **Health-Check** — zwei neue Felder, flach (Regel ADR-0043, Ergänzung Karte #31: das Monitoring muss ohne Protokoll entscheiden, **ob** die Ablage bald voll ist, und dazu braucht es die Grenze neben `log_bytes`):

   | Feld | Bedeutung |
   |---|---|
   | `plattenquote` | wirksame Quote in Bytes, `null` bei `aus` |
   | `quote_abgewiesen` | Zahl der 507 wegen der Quote seit dem Start — gezählt beim Entstehen, unabhängig von Stufe, Episode und Budget |

   `log_bytes` bleibt und ist der Zähler aus Regel 2. Keine Aufteilung je Channel oder Herkunft (steht im Protokoll). Monitoring-Empfehlung (Anleitung, kein Code): Hinweis bei `log_bytes ≥ 0,9 × plattenquote`, Alarm bei `quote_abgewiesen` steigend.
10. **Kein Live-Hinweis an die Tabs.** Wer sendet, erfährt es an seiner Nachricht (Regel 4); Lesende brauchen es nicht; ein neues Thema oder Feld im Strom wäre eine Zusage für alle Tabs ohne Szenario. Kommt der Wunsch auf (Betreiber oder UX-Review), eigene Ergänzung.
11. **Ausgeschaltet** (`aus`): kein Abweisen, keine Warnschwelle, `plattenquote: null`; der Zähler läuft weiter (`log_bytes`).

## Konsequenzen

**Positiv:**
- Platte **und** Speicher der Gespräche sind gedeckelt — auch bei 3.650 Tagen Frist und dauerhafter Flut; zusammen mit dem Budget von `logs/` belegt die Instanz im Standard höchstens ≈ 6 GiB.
- Keine zweite Löschhandlung: an der Quote geht nichts verloren, was schon geschrieben ist.
- Die Prüfung kostet je Senden eine Addition und einen Vergleich unter einer Sperre, die das Senden ohnehin hält; der Health-Check wird billiger (`log_bytes` ohne Summe über alle Gespräche).
- Exakt testbar ohne Platte (Speicherablage, gestellte Uhr) und mit kleiner Quote (1 MiB) im echten Programm.

**Negativ / Inkauf zu nehmen:**
- Ist die Quote voll, kann niemand senden, bis eine Frist Platz schafft oder der Betreiber die Quote anhebt (Neustart) — auch wenn ein einzelner Teilnehmer sie gefüllt hat.
- Wer zur Abhilfe Dateien von Hand löscht, sieht den Platz erst nach einem Neustart (der Index hält die Bytes weiter im Speicher — der Zähler sagt also die Wahrheit über den Speicher).
- Tage außerhalb der Frist in Channels mit defekten Einstellungen liegen ungezählt auf der Platte (ADR-0032 lässt sie liegen); `du` zeigt mehr als `log_bytes`.
- Ein Ändern der Quote heißt Neustart; Sitzungen gehen nach der Gnadenfrist verloren (ADR-0012).
- 507 ist ein 5xx — ein Monitoring, das 5xx als Programmfehler deutet, braucht `quote_abgewiesen` zur Unterscheidung.

**Pre-Mortem (ein Monat nach dem Start gescheitert, weil …):**
1. … ein entlaufener Agent die Quote über Nacht gefüllt hat und morgens niemand senden kann, bis zum nächsten Tageswechsel. Früh-Indikator: `WARNING quote … 90 %` Stunden vorher; die Episode „Senden gebremst“ des Schreibkontos mit derselben Kennung; `quote_abgewiesen` steigt. Gegenmaßnahme: Schreibkonto verlangsamt auf ≥ 11 h vom Normalstand bis zur Quote (Regel 3); der Text nennt die Auswege; der Betreiber verkürzt die Frist des gefluteten Channels oder startet mit größerer Quote. Tritt es wiederholt auf: Folgeoption E (Anteil je Herkunft) als eigene Entscheidung.
2. … der Zähler vom Index abdriftet, weil ein späterer Schreibweg (Anhänge, Direktgespräche, Archiv, Rückbau) ihn vergisst, und die Quote zu früh oder nie greift. Früh-Indikator: Unit-Invariante `belegt = Σ Verlauf::bytes()` nach jeder Operation; nach einem Neustart springt `log_bytes`. Gegenmaßnahme: Invariante als Test in Gespräche (erste Tests unten); jede Interaction, die Gesprächsbytes schreibt oder verschiebt (I0010, I0012, I0022), nennt in ihrer ADR ihren Anteil am Zähler — Review-Punkt des `architekt` und des `code-reviewer`.
3. … der Standard von 4 GiB für ein echtes Team zu klein ist und Menschen im Alltag 507 sehen. Früh-Indikator: `WARNING quote … 90 %` ohne Flut, `log_bytes` wächst gleichmäßig über Tage. Gegenmaßnahme: `--plattenquote` anheben ist ein Neustart, keine neue Entscheidung; der Standard ist eine Konstante mit Herleitung im Kommentar — anheben ist ein Commit.
4. … nach einem Neustart mit kleinerer Quote als dem Bestand sofort alles abgewiesen wird. Früh-Indikator: `WARNING quote Gespräche belegen mehr als die Quote` direkt nach dem Start. Gegenmaßnahme: Regel 7 (Start); hingenommen — der Betreiber hat die Quote so gewählt.

**Folgeentscheidungen / Aufgaben:**
- [ ] **Erster Unit-Test** (Gespräche, Speicherablage, gestellte Uhr; ersetzt: Logablage, Uhr): Quote 1.000 Bytes; geladen ein Tag mit 900 Bytes → `belegt = 900`. Senden mit Satz von 80 Bytes → ok, `belegt = 980 = Σ Index`. Satz von 21 Bytes → `Quote { belegt: 980, hoechstens: 1.000 }`, Ablage und Index unverändert, kein Rückruf. Satz von genau 20 Bytes → ok, `belegt = 1.000` (Grenze gehört dazu; Mutanten `>=` statt `>` und „Prüfung nach dem Anhängen“ werden rot). Wiederholung mit bekanntem Schlüssel bei voller Quote → `Wiederholt`, kein Fehler. `pflegen` nimmt den 900-Byte-Tag → `belegt = 100`; Mutante „Entfallen zieht nicht ab“ wird rot. Nach jedem Schritt `belegt = Σ Verlauf::bytes()`.
- [ ] **Unit Aufruf** (rein): `--plattenquote 4GiB` → 4.294.967.296; `512mib` → 536.870.912; `aus` → keine Quote; `1MiB` → 1.048.576; `0MiB`, `0`, `4`, `4GB`, `1025GiB`, `-1GiB`, leer → Aufruffehler mit „4GiB oder 512MiB, oder aus“; ohne Wert → `WertFehlt`; Option schlägt Umgebung; ohne beides → 4 GiB.
- [ ] **Erster Integrationstest** (echtes Programm, Wegwerf-Ordner, `TZ=Europe/Berlin`, `LOCALCHATA_JETZT`): `channels/allgemein/` mit `einstellungen.toml` (Frist 7) und einer gültigen Tagesdatei von **gestern** mit 1.048.576 − 300 Bytes; Start mit `--plattenquote 1MiB` → Protokoll hat `WARNING quote Gespräche belegen 90 % der Quote`; eine kurze Nachricht (Satz < 300 Bytes) → 201; Nachrichten, bis eine 507 `{regel: "quote", belegt, hoechstens: 1048576}` kommt; Tagesdatei von heute unverändert gegenüber vor der 507, kein `nachricht`-Ereignis an einen zweiten Strom, Health `plattenquote = 1048576`, `quote_abgewiesen = 1`, `log_bytes = belegt` aus der Antwort; genau eine `WARNING quote Senden abgewiesen: Quote erreicht — Episode beginnt`. Dann `PUT …/frist` auf 1 mit Bestätigung → gestern entfällt, `INFORMATION quote Gespräche wieder unter 90 % der Quote … grund=frist_gesetzt`, die nächste Nachricht → 201. `LOCALCHATA_PLATTENQUOTE=4` → Exit 2, kein `logs/`.
- [ ] **ADR-0042** Katalog um den Bereich `quote` ergänzt (Ergänzung Karte #34) und **ADR-0043** um die Felder (Ergänzung Karte #34); **ADR-0034** Regel 3 Schritt 9 um 507 (Ergänzung Karte #34).
- [ ] **I0010 Anhänge**: Bytes der Datei zählen; geprüft wie der Satz, **vor** dem Ablegen der Datei (Anhang + Satz zusammen gegen die Quote), abgezogen mit dem Tag. Entscheidet die ADR von I0010.
- [ ] **I0012 Direktgespräche**: gleiche Prüfung, gleiche Antwort; zählt über den gemeinsamen Index ohne Sonderweg.
- [ ] **I0022 Archivieren**: archivierte Bytes bleiben im Zähler (Regel 1), auch wenn sie den Index verlassen — der Zähler wird dann `Index + Archiv`, und `log_bytes` ändert seine Bedeutung oder ein Feld `belegt` kommt dazu; entscheidet die Ablösung von ADR-0021. Ob Tage im Archiv nach ihrer Frist weiter entfallen, ist Frage 6 an den Betreiber (WBS I0022, Offene Fragen) — mit dieser ADR hängt daran, ob Archiviertes die Quote je wieder freigibt (Empfehlung im Bericht Karte #34: ja, die Frist gilt im Archiv weiter).
- [ ] **I0019 Anleitung / `lc senden`**: 507 wird nicht wiederholt, sondern mit dem `fehler` gemeldet (anders als 429).
- [ ] `AI-Team/umgebung.md` (Director): `--plattenquote` / `LOCALCHATA_PLATTENQUOTE` nachtragen, sobald gebaut.

## Quellen / Bezuege

- [A00006 Gespräche](../Architektur/A00006-gespraeche.md), [A00009 Verlaufsindex](../Architektur/A00009-verlaufsindex.md), [A00003 Start](../Architektur/A00003-start.md), [A00004 HTTP-Schnittstelle](../Architektur/A00004-http-schnittstelle.md), [A00014 Schreibkonto](../Architektur/A00014-schreibkonto.md), [A00015 Betriebsprotokoll](../Architektur/A00015-betriebsprotokoll.md); Aspekte [X00006 Konfiguration und Geheimnisse](../Architektur/Aspekte/X00006-konfiguration-und-geheimnisse.md), [X00004 Fehlerbehandlung](../Architektur/Aspekte/X00004-fehlerbehandlung.md), [X00008 Transaktionen und Nebenläufigkeit](../Architektur/Aspekte/X00008-transaktionen-und-nebenlaeufigkeit.md)
- ADR-0003, ADR-0006, ADR-0011, ADR-0021, ADR-0030, ADR-0031, ADR-0032, ADR-0034, ADR-0035, ADR-0036 (Ergänzung Karte #31 (a)), ADR-0041, ADR-0042, ADR-0043
- Vision R00000 (Zielbild „Channels lassen sich archivieren“ — keine zweite Löschhandlung; Leitplanke Lastprofil „Team-LAN“); WBS I0022 (Offene Fragen 6); `src/gespraeche/verlaufsindex.rs:54–64`, `src/gespraeche/mod.rs:342–355`, `web/zustellung.js:79–88`
- 507: RFC 4918 Abschnitt 11.5 — aus dem Gedächtnis, in diesem Auftrag nicht geöffnet

## Ergänzungen
