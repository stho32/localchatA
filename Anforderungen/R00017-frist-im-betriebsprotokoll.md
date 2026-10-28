---
id: R00017
status: Neu
datum: 2026-10-28
---

# R00017: Frist im Betriebsprotokoll

## Beschreibung

Die Meldungen der Aufbewahrungsfrist (R00008) stehen als Einträge im Betriebsprotokoll (R00014), nicht mehr als eigene stderr-Zeilen:

- „Frist gesetzt“ und „Tage entfallen“ sind `INFORMATION frist` und stehen nur in `logs/<lokales Datum>.log`;
- „Tag nicht entfallen“ und „Frist nicht gesetzt“ sind `ERROR frist`, „Nichts entfällt — Einstellungen defekt“ ist `WARNING frist`; diese drei stehen in der Datei und weiter auf stderr.

Wer wann welche Frist gesetzt hat und was dabei gelöscht wurde, findet die Betreiberin damit im Protokoll, auch wenn niemand das Terminal gelesen hat. Probleme melden sich weiter laut auf stderr.

Zahlt ein auf: [Vision](R00000-vision.md) — „Das Log ist das Gedächtnis“ (Tage entfallen nach der Frist) und „eine gemeinsame LAN-Instanz“: die einzige endgültige Handlung des Programms, das Löschen, ist im Betrieb nachvollziehbar.

Entschieden vom architekt: [ADR-0030](../Dokumentation/ADRs/0030-entfallen-beim-start-beim-tageswechsel-und-beim-verkuerzen-loeschen-unter-der-gespraechssperre.md), Ergänzung „wie I0018 an das Betriebsprotokoll anbindet“ (Option A, Abbildung der Zusagen); [ADR-0040](../Dokumentation/ADRs/0040-betriebsprotokoll-datei-je-lokalem-tag-unter-logs-eine-zeile-je-eintrag.md) Regel 6 mit Ergänzung „keine Ausnahme für ältere stderr-Zusagen“; [ADR-0042](../Dokumentation/ADRs/0042-stufen-ereigniskatalog-episoden-und-tagesgrenze-des-betriebsprotokolls.md), Ergänzung „I0018“ (zwei Katalogzeilen); ADR-0031 und ADR-0032, Ergänzungen „I0018 im Betriebsprotokoll“; X00002.

Slice (WBS `Dokumentation/Planung/localchata.md`): F0058 Frist im Betriebsprotokoll unter I0018 Aufbewahrungsfrist je Channel festlegen; er braucht F0051 Protokoll schreibt und F0052 Bereinigung nach 14 Tagen (R00014) sowie F0038 Löschschutz und Fehlerwege (R00008, bringt F0033–F0035 mit).

## Geschäftlicher Nutzen

R00008 sagt jede Friständerung und jedes Entfallen als Zeile auf stderr zu, mit der Kennung dessen, der die Frist geändert hat. Das ist die Spur einer endgültigen Löschung (X00002: jeder mit Sitzung darf verkürzen). Läuft die Instanz als Dienst, liest niemand das Terminal, und die Spur ist weg. Seit R00014 gehört so eine Spur ins Betriebsprotokoll; ADR-0040 Regel 6 hält stderr für das frei, was Aufmerksamkeit braucht. R00017 legt die Spur dorthin, wo sie bleibt, ohne stderr mit Routine zu füllen.

## Funktionale Anforderungen

- **Abbildung** der Zusagen aus R00008 (Quelle: ADR-0030, Ergänzung; Text ohne Kennung, Kennung und Namen im Kontext nach ADR-0040 Regel 3):

  | R00008 heute (stderr) | Eintrag im Betriebsprotokoll | stderr |
  |---|---|---|
  | `channels/allgemein: Frist 7 → 30 Tage, gesetzt von beta@127.0.0.1` | `INFORMATION frist Frist 7 → 30 Tage gesetzt \| channel=allgemein kennung=beta@127.0.0.1` | nein |
  | `channels/allgemein: 2 Tage entfallen (2026-10-20 bis 2026-10-21; Frist 7 Tage; Start)` | `INFORMATION frist 2 Tage entfallen (2026-10-20 bis 2026-10-21; Frist 7 Tage) \| channel=allgemein anzahl=2 grund=start` | nein |
  | dieselbe Zeile mit `Tageswechsel` bzw. `Frist gesetzt von <kennung>` | dieselbe Zeile mit `grund=tageswechsel` bzw. `grund=frist_gesetzt kennung=<kennung>` | nein |
  | `channels/allgemein/2026-10-21.log: nicht entfallen (<Grund>)` | `ERROR frist Tag nicht entfallen — erneuter Versuch beim nächsten Anlass \| channel=allgemein datei=channels/allgemein/2026-10-21.log grund="…"` | ja |
  | `channels/kaputt: nichts entfällt — Einstellungen defekt (channels/kaputt/einstellungen.toml:1)` | `WARNING frist Nichts entfällt — Einstellungen defekt \| channel=kaputt datei=channels/kaputt/einstellungen.toml zeile=1` | ja |
  | `PUT …/frist` 500: „eine Zeile auf stderr“ | `ERROR frist Frist nicht gesetzt — Einstellungen ließen sich nicht schreiben \| channel=… kennung=… datei=… grund="…"` | ja |

- Jeder Eintrag steht in der Datei des lokalen Datums seiner Zeit (Rotation aus R00014): ein Entfallen nach Mitternacht steht in der Datei des neuen Tages.
- stderr-Zeilen haben die Zeilenform des Betriebsprotokolls (mit Zeit, Stufe, Bereich, Kontext), wie R00014 für alles ab `WARNING` festlegt.
- **Unverändert aus R00008**: eine Zeile je Channel und Pflege, nur wenn etwas entfallen ist oder scheiterte; ein Defekt einmal je Channel und Pflege; Wiederholung erst beim nächsten Anlass, nie je Takt (Z4); derselbe Wert schreibt nichts; eine 409 oder 422 schreibt nichts.
- Die Pflege beim Start schreibt vor der Startzeile; ihr Eintrag steht in der Datei, sobald die Startzeile gelesen ist.
- Die Frist schreibt nichts mehr selbst auf stderr; alle ihre Meldungen gehen über das Protokoll.
- Unverändert und nicht Teil dieses Slice: die Warnung beim Laden defekter Einstellungen (`WARNING laden`, R00014 F0051).

## Nicht-funktionale Anforderungen

- **Sicherheit / Rechenschaft** (X00002): jede wirksame Friständerung und jedes Entfallen steht mit Kennung bzw. Anlass im Protokoll, solange die Protokollstufe `information` ist (Standard). Kennungen nie im Freitext, nur als `kennung=` (ADR-0040 Regel 3).
- **Zeiten**: keine neuen. Die Grenzen Z3 und Z4 stehen in R00008 und werden wörtlich übernommen; sie gelten jetzt für die Datei **und** stderr. Reihenfolgen werden exakt über die Lage der Zeilen in der Datei geprüft, nicht über Wartezeiten.

## Akzeptanzkriterien

Gemeinsam für alle Kriterien, wenn nicht anders genannt: Start und Gegeben wie in R00008, also `TZ=Europe/Berlin LOCALCHATA_JETZT=2026-10-28T12:00:00.000+01:00 localchatA <ordner> --port 0 --host 127.0.0.1 --kein-browser` auf einem Wegwerf-Ordner, Schreibweise `20.(2)` und `S(beta)` wie dort. `heute.log` ist `logs/2026-10-28.log`. Eine **Frist-Zeile** ist eine Zeile mit dem Bereich `frist`. „Die Zeile `X`“ meint die Zeile ab der Stufe, ohne den Zeitstempel davor; `grund="…"` heißt: ein nicht leerer Grund in Anführungszeichen. stderr ist der gesamte Fehlerkanal des Prozesses.

### F0058 Frist im Betriebsprotokoll

- [ ] **Einträge bilden** (Unit, rein): je Zeile der Abbildungstabelle ergibt die Eingabe genau den Eintrag der Tabelle (Stufe, Bereich `frist`, Text, Kontext); der Text enthält nie eine Kennung (kein `@` im Text). Für `grund` gelten genau die Werte `start`, `tageswechsel`, `frist_gesetzt`; nur bei `frist_gesetzt` steht `kennung=` im Kontext.
- [ ] **Setzen** (Integration, R00008 F0033 „Setzen“): `PUT {"tage":30}` → `heute.log` enthält genau eine Frist-Zeile `INFORMATION frist Frist 7 → 30 Tage gesetzt | channel=allgemein kennung=beta@127.0.0.1`; stderr enthält weder `Frist 7 → 30` noch `INFORMATION`.
- [ ] **Derselbe Wert und abgewiesene Werte** (Integration, im Anschluss): `PUT {"tage":30}` (200, derselbe Wert), `PUT {"tage":0}` (422) → `heute.log` enthält weiter genau eine Frist-Zeile.
- [ ] **Start** (Integration, R00008 F0034 „Start“): `allgemein` ohne `einstellungen.toml` mit `20.(1)`, `21.(1)`, `22.(1)`, `28.(1)` → sobald die Startzeile gelesen ist, enthält `heute.log` die Zeile `INFORMATION frist 2 Tage entfallen (2026-10-20 bis 2026-10-21; Frist 7 Tage) | channel=allgemein anzahl=2 grund=start`, und zwar **vor** der Zeile `INFORMATION start` (die Pflege läuft vor der Startzeile); stderr enthält nicht `entfallen`.
- [ ] **Bestätigt verkürzen** (Integration, R00008 F0034 „Verkürzen braucht Bestätigung“ und „Bestätigt verkürzen“): `allgemein` mit `frist_tage = 30` und `20.(2)`, `21.(3)`, `22.(1)`, `28.(1)`. `PUT {"tage":7}` → 409; danach enthält `heute.log` keine Frist-Zeile. `PUT {"tage":7,"bestaetigt":true}` → 200; `heute.log` enthält in dieser Reihenfolge genau die Frist-Zeilen `INFORMATION frist Frist 30 → 7 Tage gesetzt | channel=allgemein kennung=beta@127.0.0.1` und `INFORMATION frist 2 Tage entfallen (2026-10-20 bis 2026-10-21; Frist 7 Tage) | channel=allgemein anzahl=2 grund=frist_gesetzt kennung=beta@127.0.0.1`; stderr enthält keine der beiden.
- [ ] **Mitternacht** (Integration, R00008 F0035 „Mitternacht im echten Programm“): `allgemein` ohne Einstellungen mit `20.(1)`, `21.(1)`, `22.(1)`, `28.(1)`; Start mit `LOCALCHATA_JETZT=2026-10-28T23:59:58+01:00`. Sobald die Startzeile gelesen ist, enthält `logs/2026-10-28.log` die Frist-Zeile mit `anzahl=2 grund=start`. Innerhalb **Z3** (R00008) nach der Startzeile enthält `logs/2026-10-29.log` genau eine Frist-Zeile `INFORMATION frist …` mit `2026-10-22` im Text und `channel=allgemein anzahl=1 grund=tageswechsel` im Kontext; `logs/2026-10-28.log` enthält keine Zeile mit `grund=tageswechsel`; stderr enthält nicht `entfallen`. Rechnung: am 29.10. ist der älteste behaltene Tag der 23.10., es entfällt der 22. (1 Nachricht).
- [ ] **Einstellungen defekt bleibt laut** (Integration, R00008 F0034 „Defekt heißt nichts“): `channels/kaputt/` mit `frist_tage = "sieben"` (Zeile 1) und `2026-01-01.log`, `channels/zu-klein/` mit `frist_tage = 0` (Zeile 1) und `2026-01-01.log` → nach dem Start enthalten `heute.log` **und** stderr je genau eine Zeile `WARNING frist Nichts entfällt — Einstellungen defekt | channel=kaputt datei=channels/kaputt/einstellungen.toml zeile=1` und dieselbe mit `channel=zu-klein datei=channels/zu-klein/einstellungen.toml`. Die Laden-Warnung (`WARNING laden`) darf daneben stehen; sie zählt hier nicht.
- [ ] **Nicht löschbar bleibt laut, die Ruhe gilt für beide** (Integration, nicht als root, R00008 F0038 „Nicht löschbar“): `allgemein` mit `2026-10-20.log`, Ordner ohne Schreibrecht (`chmod a-w`); `zweiter` mit `2026-10-20.log`; Start mit `LOCALCHATA_JETZT=2026-10-28T23:59:58+01:00`.
  - Nach der Startzeile enthalten `logs/2026-10-28.log` und stderr je genau eine Zeile, die mit `ERROR frist Tag nicht entfallen — erneuter Versuch beim nächsten Anlass | channel=allgemein datei=channels/allgemein/2026-10-20.log grund="` beginnt; für `zweiter` steht in `logs/2026-10-28.log` eine `INFORMATION frist`-Zeile mit `channel=zweiter anzahl=1 grund=start` und auf stderr keine.
  - Innerhalb **Z3** nach der Startzeile enthalten `logs/2026-10-29.log` und stderr je genau eine weitere solche `ERROR frist`-Zeile (stderr damit zwei).
  - In **Z4** danach (3 s) kommt weder in `logs/2026-10-29.log` noch auf stderr eine weitere Frist-Zeile.
- [ ] **Schreibfehler bleibt laut** (Integration, nicht als root, R00008 F0038 „Schreibfehler“): `allgemein` ohne Einstellungen, Ordner ohne Schreibrecht; `PUT {"tage":30}` → 500; `heute.log` und stderr enthalten je genau eine Zeile, die mit `ERROR frist Frist nicht gesetzt — Einstellungen ließen sich nicht schreiben | channel=allgemein kennung=beta@127.0.0.1 datei=channels/allgemein/einstellungen.toml` beginnt (der Dateiname darf auf `.neu` enden) und `grund="` enthält; `heute.log` enthält keine `INFORMATION frist`-Zeile.
- [ ] **Keine eigene stderr-Ausgabe mehr** (Review-Punkt): der Code der Frist (`src/gespraeche/` für Pflege und Setzen, die Fristpflege im Start, `src/http/frist.rs`) enthält kein `eprintln!`; die Zeilenform aus B0184 („channels/x: n Tage entfallen …“) gibt es nicht mehr.
- [ ] **Mutationsprobe** (der Entwickler nennt die Ergebnisse): je einzeln eingeführt wird mindestens ein Test oben rot —
  - M1 „INFORMATION der Frist zusätzlich auf stderr“ → „Setzen“, „Start“;
  - M2 „Frist-Meldung weiter per `eprintln!` statt Eintrag“ → „Setzen“ (`heute.log` ohne Zeile);
  - M3 „Defekt-Meldung als INFORMATION“ → „Einstellungen defekt bleibt laut“ (stderr leer);
  - M4 „`grund` immer `start`“ → „Mitternacht“, „Bestätigt verkürzen“;
  - M5 „`kennung` fehlt bei `frist_gesetzt`“ → „Bestätigt verkürzen“;
  - M6 „Kennung im Text statt im Kontext“ → „Einträge bilden“;
  - M7 „Pflege beim Start nach der Startzeile“ → „Start“ (Zeile fehlt beim Lesen der Startzeile oder steht nach `INFORMATION start`);
  - M8 „Pflege in jedem Takt“ → „Nicht löschbar bleibt laut“ (Z4);
  - M9 „derselbe Wert schreibt eine Zeile“ → „Derselbe Wert und abgewiesene Werte“.

## Betroffene Verzeichnisstruktur

- Einträge bilden: neben dem Pflegebericht in `src/gespraeche/` (rein, ersetzt die Zeilenform aus B0184).
- Eintragen: `src/start/` (Pflege beim Start, Fristpflege am Tageswechsel), `src/http/frist.rs` (Setzen mit Kennung, Schreibfehler); das Protokoll kommt als Provider aus `src/protokoll/` (A00015).
- Tests: die Integrationstests aus R00008 (`tests/frist.rs`, `tests/frist_entfallen.rs`, `tests/frist_tageswechsel.rs`, `tests/frist_schutz.rs`) lesen die Frist-Zeilen aus `logs/`; Unit im Modul.

## Technische Überlegungen

### Ablauf

1. **Pflege** (Start, Tageswechsel, Setzen): `pflegen(heute, anlass)` → `Vec<Pflegebericht>` → je Bericht `frist_eintrag(bericht)` → `protokoll.eintragen(eintrag)`.
2. **Setzen**: `frist_setzen` wirksam → `eintrag(Friständerung { channel, alt, neu, kennung })` → `protokoll.eintragen`; Schreibfehler → `eintrag(Schreibfehler { … })` → `protokoll.eintragen` → 500.
3. **Protokoll** (R00014): Zeile in die Datei des lokalen Datums; ab `WARNING` zusätzlich auf stderr.

### Grobentwurf (Vorplanung)

Die Bubbles aus der WBS als Datenfluss — **Vorplanung**: welche es am Ende werden, entscheidet der Entwickler beim Bauen.

- B0289 Frist-Meldungen als Einträge (rein): Pflegebericht | Friständerung | Schreibfehler → `Eintrag` nach der Tabelle in ADR-0030.
- B0290 Frist-Einträge eintragen (Integration): Pflege beim Start, Fristpflege am Tageswechsel, `frist_setzen` → `protokoll.eintragen` statt `eprintln!`.

**Elemente** (nach A00015 und R00008):

- `fn frist_eintrag(bericht: &Pflegebericht) -> Option<Eintrag>` (rein; `None`, wenn nichts entfiel und nichts scheiterte).
- `fn frist_gesetzt_eintrag(channel: &str, alt: u32, neu: u32, von: &Kennung) -> Eintrag`, `fn frist_nicht_gesetzt_eintrag(channel: &str, von: &Kennung, datei: &str, grund: &str) -> Eintrag` (rein).
- `Anlass` (R00008) bekommt eine Abbildung auf `grund`: `Start` → `start`, `Tageswechsel` → `tageswechsel`, `FristGesetzt { von }` → `frist_gesetzt` mit `kennung`.

### Änderungen an bestehenden Klassen

- B0184 „Pflegebericht als Zeilen“ (R00008, Vorplanung) wird im selben Lauf gleich als Eintrag gebaut; eine Zeilenform für stderr entsteht nicht.
- `frist_setzen` (R00008) gibt Friständerung und Schreibfehler als Werte zurück, statt selbst zu schreiben; der Aufrufer trägt ein.

## Tests

Nach Skill `test-pyramide`; die Szenarien der User Story werden Tests.

- **Unit (rein)**: `frist_eintrag` und die beiden Bildner je Zeile der Abbildungstabelle; Kennung nie im Text.
- **Integration (echtes Programm)**: Setzen, Derselbe Wert, Start, Bestätigt verkürzen, Mitternacht, Einstellungen defekt, Nicht löschbar, Schreibfehler. Sie sind die Tests aus R00008 mit dem Ort der Zeile aus der Abbildung; ein Test, der eine INFORMATION-Zeile der Frist auf stderr erwartet, entsteht nicht.
- **E2E**: keine.

## Abhängigkeiten

- Abhängig von: F0051 Protokoll schreibt (rot, R00014), F0052 Bereinigung nach 14 Tagen (rot, R00014; Tagestakt), F0038 Löschschutz und Fehlerwege (rot, R00008; bringt F0033, F0034, F0035 mit).
- Blockiert: —
- **Bau**: im selben Lauf wie F0033–F0038 (I0018). I0018 braucht bereits I0021.

## Offene Fragen

- Keine für den Bau dieses Slice.
- An den architekt über den Director, blockiert R00017 nicht: wie sich die Fristpflege am Tagestakt aus F0052 anmeldet (Startwert des Takts, Pflege beim Start um Mitternacht, Uhr rückwärts, eigener Takt in F0035). Stand und Empfehlungen: WBS, „Offene Fragen“, Punkt „I0018 gegen den Tagestakt aus F0052“.

## Manuelle Vorbereitungstätigkeiten

- Für die Kriterien mit `chmod a-w`: Testlauf als normaler Benutzer, nicht als root.

## Manuelle Nachbereitungstätigkeiten

- Keine.

## Umfang

- F0058 Frist im Betriebsprotokoll: 2 Bubbles · 2 klar (0,8 h; Untergrenze, ungemessen für diesen Typ)

## Warum löst diese Anforderung das Problem? (Pflicht)

Auslöser ist ein Widerspruch: R00008 sagt gelingende Meldungen der Frist auf stderr zu, ADR-0040 Regel 6 schickt `INFORMATION` nie auf stderr, und I0018 wird nach I0021 gebaut.

Die Wirkungskette:

- Die Frist schreibt ihre Meldungen als Einträge ins Betriebsprotokoll statt auf stderr.
- Deshalb steht jede Friständerung mit Kennung und jedes Entfallen mit Anlass in `logs/<tag>.log`, 14 Tage lang, auch wenn die Instanz als Dienst ohne Terminal läuft.
- Gleichzeitig bleibt stderr für das, was Eingreifen braucht: nicht löschbare Dateien, defekte Einstellungen, ein Schreibfehler.
- Damit kann die Betreiberin eine Löschung nachträglich einer Kennung zuordnen, und der Kanal für Alarme verwässert nicht durch tägliche Routine.

Der Hebel liegt an der Meldung, nicht an der Regel: ADR-0040 bleibt ohne Ausnahme, die Zusage von R00008 ändert sich an genau einer Stelle, und der Bau im selben Lauf verhindert, dass stderr-Zeilen erst entstehen und dann umgebaut werden.

## Missing-Docs

- Glossar fehlt (`Dokumentation/Domaenenwissen/` existiert nicht). Begriffe wie Frist, Entfallen, Pflege, Anlass, Betriebsprotokoll und Eintrag sind aus ADR-0029 bis ADR-0032 und ADR-0040 bis ADR-0042 übernommen.

## Notizen

- **Änderung von Zusagen aus R00008** (R00008 bleibt unverändert):
  - „Protokoll: jede wirksame Änderung schreibt auf stderr `channels/allgemein: Frist 7 → 30 Tage, gesetzt von …`“ (F0033) wird `INFORMATION frist … | channel= kennung=`, nur in der Datei.
  - „Protokoll auf stderr, eine Zeile je Channel und Pflege“ (F0034, F0035) wird `INFORMATION frist … | channel= anzahl= grund=`, nur in der Datei; der Anlass steht in `grund`, nicht mehr im Text.
  - „nicht entfallen“, „Einstellungen defekt“ und der Schreibfehler bleiben auf stderr, in der Zeilenform des Betriebsprotokolls.
  - Kriterien und Stories aus R00008, die eine dieser Zeilen auf stderr prüfen — F0033 „Setzen“, „Derselbe Wert“; F0034 „Start“, „Bestätigt verkürzen“, „Defekt heißt nichts“; F0035 „Mitternacht im echten Programm“; F0038 „Nicht löschbar“, „Schreibfehler“; US-1, US-7, US-9, US-11, US-12, US-22, US-23 —, gelten mit dem Ort und der Form aus der Abbildung oben.
  - Nicht-funktionale Anforderung „jede wirksame Änderung und jedes Löschen steht mit Kennung auf stderr“ wird „… im Betriebsprotokoll“; sie hängt an der Protokollstufe (mit `--protokoll-stufe warning` fehlt die Spur; Standard ist `information`).
- Die Abnahme-Idee des architekt (Tests „Setzen“, „Bestätigt verkürzen“, „Start“, „Mitternacht“ lesen die Datei; stderr behält „nicht entfallen“, „Einstellungen defekt“ und den Schreibfehler) ist wörtlich in die Kriterien eingegangen; für „Mitternacht“ ist die Datei die des neuen Tages (`logs/2026-10-29.log`), weil jeder Eintrag in der Datei des Datums seiner Zeit steht.

### Verworfene Alternativen

Optionen und Wahl: ADR-0030, Ergänzung „wie I0018 an das Betriebsprotokoll anbindet“.

- **Ausnahme in ADR-0040 Regel 6 für `frist`**: verworfen — zwei Regeln für stderr, und jede ältere Zusage könnte dieselbe Ausnahme verlangen (R00011 wurde über F0057 ohne Ausnahme angebunden).
- **Frist-Ereignisse auf `WARNING` heben**: verworfen — Entfallen ist tägliche Routine; `warnings` stiege jeden Tag und das Signal verwässert.

### Bewusst außerhalb

- Der Rückfall auf stderr bei nicht schreibbarem `logs/` (R00014 F0051).
- Die Stufe einstellen und die Spur bei `--protokoll-stufe warning` (R00014 F0054).
- Die Anmeldung der Fristpflege am Tagestakt (R00008 F0035, offene Frage oben).
- Die Warnung beim Laden defekter Einstellungen (`WARNING laden`, R00014 F0051).
