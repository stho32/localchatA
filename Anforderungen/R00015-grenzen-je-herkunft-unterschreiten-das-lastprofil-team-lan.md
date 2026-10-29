---
id: R00015
status: Neu
datum: 2026-10-28
ursprung: Bug-Report
ursprungslauf: R00003
---

# R00015: Behebung zu niedriger Grenzen je Herkunft für das Lastprofil „Team-LAN“ und der fehlenden Lesefrist für den Körper

## Beschreibung

Der Betreiber hat am 2026-10-28 das Lastprofil „Team-LAN“ festgelegt (`AI-Team/umgebung.md`): bis 20 Menschen und 50 Agenten gleichzeitig, Spitze 30 Nachrichten/s, 50 aktive Channels, normaler Rechner im LAN. Der architekt hat die gebauten Grenzen daran gemessen (A00001, Ergänzung „Lastprofil Team-LAN“; gerechnet, nicht gemessen) und den ungünstigen Fall angenommen: alle 50 Agenten laufen auf **einem** Rechner, dort sitzt auch ein Mensch mit Browser. Drei Grenzen je Herkunft reichen dafür nicht:

| Grenze | heute | Bedarf im Profil (A00001) | neu |
|---|---|---|---|
| Verbindungen je Herkunft | 64 | 50 wartende Agenten + 50 kurze Sendeverbindungen in Spitzen + Browser (1 Strom + bis 6) ≈ 57 dauerhaft, ≈ 107 in Spitzen | **160** (≈ 1,5 × Spitze) |
| Lebende Sitzungen je Herkunft | 64 | 50 Agenten + 1 Mensch = 51; startet jeder Agent einmal mit neuem Namen neu, hält die alte Sitzung 60 s → bis 101 | **128** |
| Angelegte Channels je Herkunft | 32 | 50 aktive Channels, von Agenten eines Rechners angelegt | **128** |

Reproduktion (Verbindungen, heute): 65 offene Verbindungen von `127.0.0.1` → die 65. wird ohne Antwort abgewiesen. In der Spitze des Profils betrifft das jeden 2. Agenten dieses Rechners, und zwar still: die Abweisung hinterlässt bis R00014 keine Spur. Sitzungen: 65 Beitritte von `127.0.0.1` → der 65. ergibt 429. Channels: `c01` … `c32` von einer Herkunft → `c33` ergibt 429 `{grenze:"herkunft", hoechstens:32}`.

**Dazu, vorbestehend, aus dem Security-Review zu #21:** der Körper einer Anfrage hat unterhalb der Grenze **keine Lesefrist**. Ein Client schickt einen Kopf mit `Content-Length: 4096`, dann 10 Bytes, und schweigt. Die Verbindung bleibt länger als 120 s offen (geprobt im Security-Review). 64 solche Verbindungen je Herkunft belegen heute alle Plätze dieser Herkunft, ein Rechner mit 8 LAN-Adressen alle 512. Mit 160 Plätzen je Herkunft genügen dafür 4 Adressen — die Anhebung macht die Lücke wichtiger, deshalb gehört die Frist in dieselbe Anforderung.

Erwartet: ein Rechner mit 50 Agenten und einem Menschen arbeitet im Profil, ohne an eine Grenze je Herkunft zu stoßen, und kein Client belegt einen Platz länger als eine feste Frist, nur weil er seinen Körper nicht schickt. Tatsächlich: er stößt an drei Grenzen, und ein schweigender Körper hält seinen Platz unbegrenzt.

Zahlt ein auf: [Vision](R00000-vision.md) — „Mensch und KI sind gleichberechtigt“ und „eine gemeinsame LAN-Instanz“: viele Agenten auf einem Rechner sind der Anlass der Vision („Agenten auf verschiedenen Rechnern helfen einander“), und die Instanz bleibt für alle bedienbar.

## Ursachenanalyse

### Root Cause

- `src/start/verbindungen.rs:11` — `JE_GEGENSTELLE: usize = 64` (ADR-0010). Der Wert war „genug für Browser-Tabs, Live-Kanal und Agenten“, aber für das Profil nicht hergeleitet; das Profil gab es damals nicht.
- `src/anwesenheit/bestand.rs:19` — `JE_HERKUNFT: usize = 64`, laut Kommentar „gleich den Verbindungsgrenzen (ADR-0012)“. Verlassene Sitzungen zählen bis zum Ende der Gnadenfrist mit (`:241`, ADR-0018 Regel 2a).
- `src/gespraeche/verzeichnis.rs:60` — `JE_HERKUNFT: usize = 32` (ADR-0021 Regel 4), dauerhaft gezählt, ohne Löschen.
- `src/http/koerper.rs` `begrenzt_lesen` (`:40–51`) liest den Strom ohne Frist; nur `rest_verwerfen` über der Grenze hat eine (`VERWERFEN_FRIST` 2 s). `KOPF_ZEITGRENZE` (`src/start/ausliefern.rs:21`, 10 s) gilt laut Kommentar nur „vom ersten Byte bis zur Leerzeile“ des Kopfes.

Ausgeschlossen:

- Die Gesamtgrenzen passen zum Profil: Verbindungen gesamt `min(512, Deskriptoren/2)` bei ≈ 240 Bedarf, Sitzungen gesamt 512 bei 70 bzw. 140, Channels gesamt 256 bei 50 aktiven.
- Das Schreibkonto je Kennung passt.
- Das Schreibkonto je Herkunft (Vorrat 120, 8/s) ist ein Grenzfall, der an der Dauer einer Spitze hängt. Der Betreiber kennt sie nicht; nach der Entscheidung des Directors (2026-10-28) bleibt es wie gebaut, und die Auslegung folgt der Messung im Betrieb (F0057, R00016).
- Die Auslegung des Verlaufs auf 3–5 Mio. Nachrichten ist eine Probe vor I0011, keine Anforderung (Notiz an I0011 in der WBS).

### Betroffene Komponenten

- Start: Verbindungsgrenzen der Annahmeschleife (`src/start/verbindungen.rs`, A00003, ADR-0010)
- Anwesenheit: Bestand (`src/anwesenheit/bestand.rs`, A00010, ADR-0012)
- Gespräche: Channelverzeichnis (`src/gespraeche/verzeichnis.rs`, A00012, ADR-0021)
- HTTP: Körper lesen (`src/http/koerper.rs`, A00004, ADR-0010)
- Folgen: Q8 „Langsame Leser“ (A00001; R00013 K4) — der Speicher je Herkunft wächst mit der Zahl der Plätze.

## Lösungsvorschlag

### Langfristige Lösung

Die drei Konstanten werden je mit ihrer Herleitung aus dem Lastprofil im Kommentar angehoben. Dazu kommt eine **Lesefrist für den Körper** von 10 s (`KOERPER_LESEFRIST`, gleich der Kopf-Zeitgrenze): gesamt ab dem Beginn des Körpers, nicht je Stück. Ist der Körper bis dahin nicht vollständig, antwortet die Instanz 408 `{fehler}` („Körper nicht vollständig innerhalb von 10 s — Anfrage abgebrochen.“) und schließt die Verbindung.

**Folgen, die diese Anforderung mitnimmt:**

- **Q8 / R00013 K4** neu gemessen. Die Rechnung folgt ADR-0039 „Rechnung Q8“ (Plätze × 2 × Budget 256 KiB + 50 MB):
  - 1 Herkunft: 160 × 0,5 MiB = 80 MiB ≈ 83,9 MB, plus 50 MB ≈ **134 MB** statt 84 MB;
  - 2 Herkünfte: 320 × 0,5 MiB = 160 MiB ≈ 167,8 MB, plus 50 MB ≈ **218 MB** statt 117 MB.
- **ADR-0012** „Sitzungen gleich den Verbindungsgrenzen“ gilt nicht mehr wörtlich (160 gegen 128).
- **Gesamtgrenze der Sitzungen** bleibt 512: 4 Herkünfte mit je 128 erreichen sie.
- **Gesamtgrenze der Channels** bleibt 256: 2 Herkünfte mit je 128 erreichen sie.

### Alternative Ansätze

| Option | Komplexität | Testbarkeit | Reversibilität | Risiko |
|---|---|---|---|---|
| **A Konstanten nach Profil anheben, Lesefrist dazu** (gewählt) | gering | hoch (Spanne ±1, Frist mit gestellter Zeit) | hoch | Q8 je Herkunft größer (134 MB) |
| B Grenzen je Herkunft einstellbar (Option/Umgebung) | mittel | mittel | hoch | ein Betreiber setzt sie zu hoch, ohne Q8 zu kennen; neue Angriffsfläche ohne Anmeldung nein, aber eine weitere Konfiguration |
| C Grenzen je Kennung statt je Herkunft | hoch | mittel | gering | Kennungen kosten nichts (beitreten), Schutz gegen Flut fiele weg |

- B verworfen: das Profil ist bekannt, und eine Zahl mit Herleitung ist prüfbar. Einstellbarkeit kann später kommen, wenn ein zweites Profil auftritt.
- C verworfen: es schwächt den Schutz, der die Grenzen begründet (CWE-770).
- Lesefrist, Variante „Leerlauf je Stück“ verworfen: ein Tröpfler hält dann mit einem Byte je 9 s beliebig lange.
- Lesefrist, Variante „im Kernel über `TCP_USER_TIMEOUT`“ verworfen: das gilt nur für unbestätigte Daten, ein schweigender Sender hat keine.

## Test-Strategie

### Unit Test zur Bug-Reproduktion

- `src/start/verbindungen.rs`: Grenze je Gegenstelle = 160 bei ausreichenden Deskriptoren. Heute rot (64).
- `src/anwesenheit/bestand.rs`: 128 Beitritte einer Herkunft ergeben Erfolg, der 129. `ZuVieleSitzungen`. Heute rot beim 65.
- `src/gespraeche/verzeichnis.rs`: 128 Anlagen von `10.0.0.1` ergeben Erfolg, die 129. `Grenze {herkunft, 128}`. Heute rot bei der 33.
- `src/http/koerper.rs` mit `tokio::time::pause`: ein Strom liefert 10 Bytes von angekündigten 4.096 und dann nichts → nach 9,999 s wird noch gelesen, nach 10,000 s kommt `ZuLangsam`. Heute rot: liest endlos.

### Code-Extraktion für isoliertes Testing

- Die Frist ist ein Parameter der Lesefunktion (gebaut: `roh_lesen`, über `lesen_mit_umstaenden` aus `json_lesen`; ADR-0010 Nachzug Karte #28), die Konstante setzt der Aufrufer — so ist sie mit pausierter Zeit testbar.
- Die Grenzen bleiben Konstanten neben ihrer Prüfung; die bestehenden reinen Tests übernehmen die neue Spanne.

### Regressionstests

- Bestehende Tests mit den alten Zahlen werden auf die neue Spanne umgestellt, nicht gelöscht:
  - `tests/beitreten.rs:226`, `:254` (64 → 128);
  - `src/start/verbindungen.rs:172–200` (64 → 160);
  - R00006-Tests „32. → 201, 33. → 429“ (→ 128/129) und „8 Herkünfte × 32 = 256“ (→ 2 Herkünfte × 128 = 256).
- R00010 (Körpergrenze 413) und R00013 (Budget, Sendefrist) bleiben grün.
- Mutationsprobe je Grenze ±1 und für die Frist (siehe Kriterien).

## Akzeptanzkriterien

- [ ] Der Reproduktionstest ist ohne Fix rot und mit Fix grün
- [ ] Der ursprüngliche Fehler tritt im Szenario der User Story nicht mehr auf
- [ ] Keine neuen Fehler: alle Tests aller Ebenen grün (Gate nach `AI-Team/umgebung.md`)

Gemeinsam: Start mit `localchatA <ordner> --port 0 --host 127.0.0.1 --kein-browser` auf einem Wegwerf-Ordner. Vor jedem Integrationstest mit vielen Verbindungen prüft der Test die weiche Deskriptorgrenze (`ulimit -n` ≥ 1.024) und meldet sonst einen Abbruch statt eines Befunds. Damit ist die Gesamtgrenze 512.

### G1 Verbindungen je Herkunft (Unit und Integration)

- [ ] Unit: `fuer_deskriptoren(Some(u64::MAX))` → `je_gegenstelle: 160`, `gesamt: 512`; mit `Some(200)` → je Gegenstelle `min(160, 100)` = 100.
- [ ] Integration: 160 offene Verbindungen von `127.0.0.1` werden bedient (je `GET /api/gesundheit` → 200 auf derselben Verbindung). Die 161. wird abgewiesen. Eine Verbindung von `127.0.0.2` wird danach bedient.
- [ ] Mutanten 159 und 161 werden rot.

### G2 Sitzungen je Herkunft (Unit und Integration)

- [ ] Integration: 128 Beitritte `a001` … `a128` von `127.0.0.1` → 128 × 201. `a129` → 429. Von `127.0.0.2` → 201.
- [ ] Gesamt (Unit, Bestand): 4 Herkünfte × 128 = 512 → Erfolg; die 513. von einer fünften Herkunft → `ZuVieleSitzungen` gesamt.
- [ ] Mutanten 127 und 129 werden rot.

### G3 Channels je Herkunft (Unit und Integration)

- [ ] Integration: `alpha` legt von `127.0.0.1` `c001` … `c128` an → 128 × 201. `c129` → 429 `{fehler, grenze:"herkunft", hoechstens:128}`, auch nach einem Neustart. `beta` von `127.0.0.2` legt einen an → 201.
- [ ] Gesamt (Integration): 2 Herkünfte × 128 = 256 → 256 × 201. Ein weiterer von einer dritten Herkunft → 429 `{grenze:"gesamt", hoechstens:256}`.
- [ ] Mutanten 127 und 129 werden rot.

### G4 Lesefrist für den Körper (Unit und Integration; Abnahme-Probe des Security-Reviews zu #21)

- [ ] **Unit** (`tokio::time::pause`): 10 Bytes von angekündigten 4.096, danach Stille → nach 9,999 s keine Entscheidung, nach 10,000 s `ZuLangsam`.
  - Ein Strom, der alle 1 s ein Byte liefert, endet ebenso nach 10,000 s ab dem ersten Körperbyte (gesamt, nicht je Stück).
  - 248 Bytes, vollständig nach 9,9 s → gelesen.
- [ ] **Integration, Abnahme-Probe**. Zweck: ein Client, der seinen Körper nicht schickt, hält keinen Platz länger als die Frist; mit G parallel schweigenden Körpern einer Herkunft ist danach wieder Platz.
  - Gegeben: von `127.0.0.1` öffnen G = 160 Rohverbindungen (mindestens die 64 der Probe) je `POST /api/beitreten` mit `Content-Type: application/json` und `Content-Length: 4096`. Jede sendet nach dem Kopf 10 Bytes und schweigt.
  - Startpunkt der Uhr: die 10 Bytes der jeweiligen Verbindung sind gesendet.
  - Erwartet: jede Verbindung bekommt 408 mit `{fehler}` oder wird geschlossen, nach ≈ 10,1 s (Frist 10 s plus Lauf der Laufzeit). **Grenze 13,5 s** (34 % über 10,1 s), geprüft durch Lesen mit Zeitgrenze, nicht mit `sleep`.
  - Danach, aber vor 15 s nach dem Start der Probe: eine neue Verbindung von `127.0.0.1` ergibt `GET /api/gesundheit` → 200.
- [ ] **Tröpfler** (Integration): eine Verbindung sendet den Körper eines Beitritts zu 4.096 Bytes mit einem Byte je Sekunde → 408 oder geschlossen spätestens 13,5 s nach dem ersten Körperbyte.
- [ ] **Kontrolle** (Integration): ein gültiger Beitritt mit 248 Bytes Körper, getröpfelt in 8 Stücken zu 31 Bytes, je 1 s Abstand (fertig nach ≈ 7 s, unter der Frist), ergibt 201.
- [ ] Mutanten, die rot werden müssen:
  - Frist fehlt (heutiger Stand): Verbindungen nach 13,5 s noch offen;
  - Frist nur für den Kopf: gleiches Ergebnis;
  - Frist je Stück statt gesamt: der Tröpfler hält;
  - Frist 5 s: die Kontrolle wird 408.

### G5 Langsame Leser bei 160 (Messwert im Release-Build; Folge für Q8 und R00013 K4)

- [ ] K4 aus R00013 wird mit G = 160 nicht lesenden Abrufen je Herkunft wiederholt, derselbe Messweg (`cargo test --release -- --ignored langsame_leser`), Startpunkt `t0` wie dort:

  | Wert | erwartet (Herleitung) | Grenze |
  |---|---|---|
  | RSS-Zuwachs, 1 Herkunft × 160 | ≈ + 80 MB (R00013 erwartet ≈ + 32 MB je 64, linear × 2,5) | **≤ 134 MB** (160 × 2 × 256 KiB + 50 MB) |
  | RSS-Zuwachs, 2 Herkünfte × 160 | ≈ + 160 MB | **≤ 218 MB** |
  | Threads | ≤ `t0`-Threads + 64 | **≤ `t0`-Threads + 64** (Rückhalt aus R00013) |
  | Sendefrist | ≈ 10,3 s | **zu bei `t0` + 15 s** |
  | andere Routen | 200 in ms | **200 innerhalb 1 s** von `127.0.0.9` |
  | Rückgabe nach dem Schließen | ≈ + 22 MB | **≤ 50 MB** |

- [ ] Ein Wert über der Grenze ist ein Befund an ADR-0039 bzw. an diese Anhebung, kein Grund, die Grenze zu ändern. Die Grenzen stehen nur hier; der Test übernimmt sie und verschärft sie nie.

## Implementierungshinweise

- Konstanten mit Herleitung im Kommentar: Profil, Rechnung, Datum, Verweis auf A00001.
- `roh_lesen(kopf, koerper, grenze, frist)` (gebaut; geplant hieß sie `begrenzt_lesen`) mit `tokio::time::timeout_at` über die ganze Schleife. Die Antwort 408 entsteht wie 413 im Aufrufer.
- Für Integrationstests mit vielen Verbindungen: Rohverbindungen mit `bind` auf `127.0.0.x` vor `connect`, wie in R00013.
- Ist R00014 schon gebaut, steht die 408 als DEBUG-Zeile im Protokoll (route, herkunft, art); die Zählung in `abgewiesen` siehe Offene Fragen.

## Offene Fragen

- Keine für den Bau.
- An den architekt, nicht blockierend: Nachzug ADR-0010 (Grenze je Gegenstelle 160, Lesefrist des Körpers), ADR-0012 (Sitzungen 128, nicht mehr gleich den Verbindungen) und ADR-0021 (Channels je Herkunft 128). Zählt eine 408 in `abgewiesen` (R00014)? Empfehlung: eine fünfte Art `zu_langsam`, mit Ergänzung in ADR-0043.
- An den Betreiber, offen (Entscheidung Director 2026-10-28: Auslegung nach Messung im Betrieb): die Dauer einer Spitze von 30 Nachrichten/s aus **einer** Herkunft. Das Schreibkonto je Herkunft bleibt bis dahin wie gebaut (Vorrat 120, 8/s). Die Messung liefern Schreibkonto-Episode und `gebremst` (F0057, R00016). Trägt eine Spitze länger als ≈ 5 s, liegt der Vorschlag aus A00001 bereit: 16/s und Vorrat 300.
- Nicht blockierend: Channel löschen bzw. archivieren als künftige Interaction erwägen, weil die Gesamtgrenze 256 ohne Löschen über Monate wächst (A00001).

## Warum löst diese Anforderung das Problem? (Pflicht)

Auslöser ist das neue Lastprofil des Betreibers: 50 Agenten und ein Mensch auf einem Rechner sind der ungünstige, aber erwartete Fall. Die Grenzen je Herkunft wurden ohne Profil gesetzt und schneiden genau diesen Fall ab.

Die Wirkungskette:

- 160 Plätze, 128 Sitzungen und 128 Channels je Herkunft tragen die gerechnete Spitze mit Reserve (≈ 1,5 × 107 Verbindungen; 101 Sitzungen bei Neustarts; 50 Channels). Deshalb arbeitet der Rechner ohne Abweisung.
- Die Gesamtgrenzen bleiben. Deshalb kann eine einzelne Herkunft die Instanz nicht für alle anderen belegen.
- Mehr Plätze je Herkunft heißen auch mehr Plätze, die ein schweigender Client halten kann. Die Lesefrist für den Körper macht jeden Platz nach höchstens 10 s wieder frei, wie es der Kopf schon tut.

Der Hebel liegt an den Konstanten selbst, nicht an einer Einstellung oder an Kennungen: so bleibt jede Zahl hergeleitet und prüfbar. Die Folge für den Speicher (Q8) ist berechnet und wird gemessen, statt übersehen zu werden.

## Notizen

- Priorität: Hoch, vor I0014–I0016 (Agenten an der API) und vor dem Lastbetrieb.
- Betroffene Nutzer/Systeme: alle Agenten eines Rechners; die Betreiberin (Abweisung bis R00014 still).
- Workaround: Agenten auf mehrere Rechner verteilen.
- Weiche (`rollen/planer/befunde-einplanen.md`):
  - Grenzen: **reifer**. Keine Anforderung sagte das Profil zu; es entstand nach dem Bau. Die Lesefrist ist ein Sicherheitsbefund, vorbestehend seit R00003/R00010. Beides steht nach Auftrag des Directors im Bug-Sonderformat, wie R00010 und R00013.
- **Änderung von Zusagen**:
  - R00003: „mehr als 64 lebende Sitzungen je Herkunft → 429“ gilt ab R00015 mit 128.
  - R00006: „je Herkunft höchstens 32 Channels“ gilt ab R00015 mit 128.
  - Die Grenze von 64 Verbindungen je Gegenstelle (R00001, B0017) gilt ab R00015 mit 160.
  - R00013 K4: die Grenze 84 MB gilt für 64 Verbindungen; ab R00015 gilt G5 mit 160.
  - R00003, R00006 und R00013 bleiben unverändert; Anforderungen werden nicht geändert.
- Abnahme-Probe der Lesefrist (Security-Review zu #21) steht in G4. Die Zeitgrenze ist **13,5 s** statt der vorgeschlagenen „Frist + 2 s“ = 12 s, weil 12 s nur 19 % über dem erwarteten Wert lägen (`AI-Team/planer.md`: mindestens 30 %). Die Regel selbst (10,000 s) prüft der Unit-Test exakt.
- Herkunft: A00001, Ergänzung „Lastprofil Team-LAN“ (Commit 4883270), Entscheidung Director 2026-10-28; Security-Review zu #21 (Lesefrist).
- WBS: F0056 unter I0004 mit B0283–B0285 und B0288.
