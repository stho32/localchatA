# ADR-0041: Betriebsprotokoll als eigenes Modul ohne neue Crate — Rotation beim Schreiben nach Eintragsdatum, Bereinigung nach 14 Kalendertagen beim Start und beim Tageswechsel

**Status**: Akzeptiert
**Datum**: 2026-10-28
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Vorlauf „Betriebsprotokoll“)
**Kontext-Anforderung**: — (neue Interaction „Betriebsprotokoll“)
**Erprobt durch**: Probe nach Skill `dependency-probe` im Scratchpad (nicht im Repository): `tracing-appender` 0.2.5 mit `tracing` 0.1.44, `tracing-subscriber` 0.3.23, Rust-Edition 2024 — Ergebnisse unter Option B; dazu Lesen der Quelle `tracing-appender-0.2.5/src/rolling.rs` (Zeilen 198–223, 655–722)

## Kontext

- [ADR-0040](0040-betriebsprotokoll-datei-je-lokalem-tag-unter-logs-eine-zeile-je-eintrag.md): Datei je **lokalem** Tag, Zeile mit Escape und Erlaubnisliste der Schlüssel, Rückfall auf stderr.
- Vorgabe des Betreibers: täglich rotierend, Aufbewahrung 14 Tage, danach bereinigt (`AI-Team/umgebung.md`).
- Projektregeln: „jetzt“ nur über den Uhr-Provider (X00007); Regeln mit gestellter Uhr prüfbar, der Takt bestimmt nur das Wann (ADR-0016, ADR-0030); Löschen nur nach Erlaubnisliste, keine Symlinks ([ADR-0032](0032-loeschen-nur-tagesdateien-nach-erlaubnisliste-keine-symlinks-defekt-heisst-nichts.md)); Frist in Kalendertagen, heute zählt mit ([ADR-0029](0029-frist-in-kalendertagen-heute-zaehlt-mit-1-bis-3650.md)).
- Abhängigkeiten heute (`Cargo.toml`): axum, tokio, hyper, chrono (`clock`), serde, toml, getrandom, rustix — `tracing` nur transitiv über axum (`Cargo.lock`), kein Subscriber, kein Appender.
- Die Fristpflege der Gespräche (I0018, ADR-0030) ist entschieden, aber nicht gebaut; sie braucht denselben Tageswechsel.

## Optionen

### Option A: Eigenes Modul `protokoll` (std + chrono + Uhr-Provider)
- Ein `Protokoll` im Zustand (`Arc`), `std::sync::Mutex` um die offene Datei und ihr Datum; `eintragen(stufe, bereich, text, kontext)`; Formatierer rein; Bereinigung als Operation `bereinigen(heute)`.
- Vorteile: Datum, Uhr, Escape, Erlaubnisliste, Rückfall und Bereinigung genau wie ADR-0040 verlangt; mit `LOCALCHATA_JETZT` und gestellter Uhr deterministisch prüfbar; keine neue Crate; gleiches Muster wie die Logablage.
- Nachteile: Rotation, Bereinigung und Escape sind eigener Code mit eigenen Tests (geschätzt 300–400 Zeilen); keine Spans und kein Anschluss an Bibliotheken, die selbst über `tracing` melden.

### Option B: `tracing` + `tracing-subscriber` + `tracing-appender` (tägliche Rotation)
- Vorteile: verbreitet; strukturierte Felder und Spans je Anfrage (Kontext ohne Durchreichen); nicht blockierender Schreiber verfügbar.
- Probe und Quelle zeigen (Annahme → Ergebnis):
  - Datum der Datei: lokal? → **nein**, `OffsetDateTime::now_utc()` (rolling.rs:198, 222); zwischen 00:00 und 01:00/02:00 Ortszeit landen Einträge in der Datei des Vortags — widerspricht ADR-0040 Regel 2.
  - Uhr ersetzbar? → **nein**, die Uhr ist nur unter `#[cfg(test)]` der Crate selbst austauschbar (rolling.rs:210–223); `LOCALCHATA_JETZT` wirkt nicht, Rotation und Bereinigung wären nur mit echter Mitternacht prüfbar.
  - Aufbewahrung 14 **Tage**? → **nein**, `max_log_files(n)` zählt **Dateien** und sortiert nach Erstellungszeit des Dateisystems (rolling.rs:655–722). Probe: drei Dateien `2025-01-01.log` bis `2025-01-03.log` (fast zwei Jahre alt) plus die neue — mit `max_log_files(14)` blieben **alle** liegen. Läuft die Instanz nicht jeden Tag, bleiben ältere Tage über 14 Tage hinaus.
  - Fehler sichtbar? → Löschfehler und Lesefehler gehen per `eprintln!` an stderr (rolling.rs:708, 717), nicht ins Protokoll; ein nicht anlegbarer Ordner ergibt beim Bauen einen Fehler (Probe: `failed to create log directory`).
  - Escape: der Appender schreibt Bytes wie übergeben (Probe: ESC `\x1b[31m` stand roh in der Datei); ob der Formatierer von `tracing-subscriber` Steuerzeichen in Feldern neutralisiert, ist **nicht geprobt** — ADR-0040 Regel 4 bräuchte ohnehin einen eigenen Formatierer.
- Nachteile: `tracing` selbst ist schon transitiv da (über axum), neu wären `tracing-subscriber` und `tracing-appender` samt `time`, `sharded-slab`, `thread_local`, `crossbeam-channel` — für eine Funktion, die wir in den drei tragenden Punkten (Datum, Uhr, Aufbewahrung) selbst nachbauen müssten.

### Option C: `tracing` als Schnittstelle (Makros, Spans) mit eigenem Schreiber (`MakeWriter`) für Datei, Datum, Bereinigung
- Vorteile: Spans liefern Kontext je Anfrage; Bibliotheken, die über `tracing` melden, landen mit im Protokoll; Datum, Uhr und Bereinigung bleiben unsere.
- Nachteile: zwei Crates (`tracing`, `tracing-subscriber`) und ihre Begriffe (Layer, Subscriber, Filter) für das Team; der Formatierer muss trotzdem eigen sein (Escape, Erlaubnisliste); Spans reichen nicht über `spawn_blocking` hinweg ohne eigenes Weiterreichen — ausgerechnet dort, wo Panik und Plattenfehler entstehen.

### Option D: Fassade `log` mit eigenem Hintergrund
- Vorteile: kleinste Fassade; Bibliotheken mit `log`-Ausgabe landen mit im Protokoll.
- Nachteile: Stufen `Error/Warn/Info/Debug/Trace` passen nicht auf die fünf Stufen der Vorgabe (keine Exception); Kontext nur als Text; eine globale Instanz statt eines Providers im Zustand erschwert parallele Tests im selben Prozess.

### Wann bereinigen?
- **Beim Start und beim Tageswechsel (Takt 1 s über den Uhr-Provider)** — wie ADR-0030; deterministisch, auch an Tagen ohne Eintrag.
- **Nur beim Rotieren (erster Eintrag eines neuen Tages)**: kein Takt; aber an einem stillen Tag ohne Eintrag bleibt die Bereinigung aus — die Grenze „14 Tage“ hängt dann an der Last.
- **Nur beim Start**: am einfachsten; eine Instanz, die vier Wochen läuft, hält vier Wochen Protokoll.

## Entscheidung

**Gewählt: Option A, Bereinigung beim Start und beim Tageswechsel**

Es überwiegt, dass nur das eigene Modul die drei tragenden Punkte der Vorgabe — lokales Datum, ersetzbare Uhr, Aufbewahrung in Kalendertagen — erfüllt und wie die Logablage mit gestellter Uhr prüfbar bleibt; die Probe hat B in genau diesen drei Punkten widerlegt. In Kauf genommen: eigener Code für Rotation, Bereinigung und Escape (mit Tests) und kein Anschluss an Bibliotheken, die über `tracing` melden — `tracing` ist zwar schon transitiv im Bau (`cargo tree -i tracing`: über axum 0.8.9, axum-core, tower — Standard-Feature `tracing` von axum), dort aber nur für Ablehnungsdetails auf niedriger Stufe; ohne Subscriber gehen sie verloren wie heute, und jede Ablehnung, die der Betreiber braucht, protokolliert unsere Grenze selbst (ADR-0042). Kommt später ein Bedarf an Spans, ist C eine Ergänzung (eigener `MakeWriter` hinter derselben Datei), kein Umbau.

Regeln:

1. **Modul** `src/protokoll/` (Baustein [A00015](../Architektur/A00015-betriebsprotokoll.md)): `Protokoll::oeffnen(ordner, uhr, stufe, grenzen) -> Protokoll` (scheitert nie — Rückfall nach ADR-0040 Regel 8); `eintragen(Eintrag)`; `bereinigen(heute: NaiveDate) -> Bereinigung`; Kennzahlen für den Health-Check. Der Formatierer ist eine reine Funktion `zeile(&Eintrag, Zeit) -> String`.
2. **Rotation beim Schreiben**: `eintragen` nimmt `jetzt` vom Uhr-Provider, bildet daraus Zeitstempel **und** Datei; weicht das Datum von dem der offenen Datei ab, wird die offene geschlossen und `logs/<datum>.log` geöffnet. Kein Umbenennen, kein Kopieren. Datum rückwärts (zurückgestellte Uhr) öffnet die ältere Datei wieder und hängt dort an.
3. **Schreiben**: unter dem `Mutex` ein `write_all` der fertigen Zeile auf die Datei (ohne `BufWriter`); die Sperre wird nie über ein `.await` gehalten. Gerechnet: ≈ 200 Byte je Zeile, im Lastprofil ≈ 0,1 Einträge/s auf Information und ≈ 30/s auf Debug (ADR-0042) — ein Systemaufruf je Eintrag ist tragbar. Ein Panik-Haken (ADR-0042 Regel 4) nimmt die Sperre nur mit `try_lock`; ist sie belegt oder vergiftet, schreibt er auf stderr — nie Verklemmung im Panikpfad.
4. **Aufbewahrung**: `AUFBEWAHRUNG_TAGE = 14`, heute zählt mit wie ADR-0029: Datei D entfällt ⇔ `D < heute − 13`. Am 2026-10-28 bleiben `2026-10-15.log` bis `2026-10-28.log` (14 Dateien), `2026-10-14.log` entfällt.
5. **Was gelöscht werden darf** (wie ADR-0032): nur **reguläre Dateien** direkt in `logs/`, deren Name genau `JJJJ-MM-TT.log` mit gültigem Datum ist (`symlink_metadata`, kein Folgen von Links); Unterordner, Symlinks, fremde Dateien (`notizen.txt`, `2026-10-14.log.gz`) bleiben. Der Pfad entsteht aus dem Datum, nie aus dem gelesenen Namen.
6. **Anlässe**:
   - **Start**: einmal synchron direkt nach dem Öffnen des Protokolls (nach der Sperre, vor dem Laden) — damit die Bereinigung selbst schon im Protokoll steht.
   - **Tageswechsel**: ein **Tagestakt** `tagestakt::takten(uhr, ende, PERIODE = 1 s, aufgaben)` fragt jede Sekunde `uhr.jetzt().date_naive()`; nur wenn sich das Datum seit dem letzten Lauf geändert hat, ruft er die angemeldeten Tagesaufgaben über `spawn_blocking` — zuerst `protokoll.bereinigen(heute)`. Die Fristpflege der Gespräche (ADR-0030 Regel 2, I0018) meldet sich als zweite Aufgabe **an denselben Takt** an, statt einen eigenen zu bauen; ADR-0030 bleibt in der Regel unverändert, nur der Task ist geteilt.
7. **Protokoll der Bereinigung**: `INFORMATION protokoll <n> Protokolldateien entfallen (<älteste> bis <jüngste>; Aufbewahrung 14 Tage; Start|Tageswechsel)` nur, wenn etwas entfiel; je gescheiterter Datei `ERROR protokoll Protokolldatei nicht entfallen | datei=… grund=…`; Wiederholung erst beim nächsten Anlass (nicht im nächsten Takt).

## Konsequenzen

**Positiv:**
- Rotation und Bereinigung sind Unit-testbar mit gestellter Uhr; der Integrationstest braucht nur `LOCALCHATA_JETZT` kurz vor Mitternacht.
- Keine neue Crate, keine neue Lizenzprüfung, kein Wechsel der Begriffe.
- Ein Tagestakt für Protokoll und Fristpflege — ein Muster, ein Test.

**Negativ / Inkauf zu nehmen:**
- Eigener Code für etwas, das Bibliotheken anbieten; er muss Zeitumstellung, Rückstellung und Erlaubnisliste selbst richtig machen.
- Kein Kontext über Spans: der Kontext wird an der Grenze als Wert übergeben (Kennung, Route).
- Schreiben ist synchron: eine hängende Platte hält den Aufrufer so lange auf wie das Anhängen an ein Tageslog — derselbe Datenträger, dieselbe Lage.

**Pre-Mortem (ein Monat nach dem Start gescheitert, weil …):**
1. … der Betreiber `logs/` auf ein Netzlaufwerk verlinkt und die Bereinigung über den Link hinweg Fremdes löscht. Früh-Indikator: Test mit `logs/` als Symlink. Gegenmaßnahme: Regel 5 prüft jede Datei mit `symlink_metadata`; ist `logs/` selbst ein Symlink, wird hineingeschrieben (Betreiberwunsch), aber nur nach Regel 5 gelöscht.
2. … die Uhr Jahre vorgeht und die Bereinigung alle Protokolle löscht, die den Fehler belegen. Früh-Indikator: Information-Zeile mit großem `n`. Gegenmaßnahme: hingenommen wie ADR-0030 Pre-Mortem 1 — die Zeile nennt Anzahl und Spanne; die Datei des (falschen) Tages bleibt und zeigt die Startzeile mit Zeitstempel.
3. … der Panik-Haken sich an der eigenen Sperre verklemmt, weil die Panik im Protokoll selbst entsteht. Früh-Indikator: Unit-Test „Panik, während die Sperre gehalten wird“. Gegenmaßnahme: Regel 3, `try_lock` im Haken.

**Folgeentscheidungen / Aufgaben:**
- [ ] Erster Unit-Test Bereinigung (Wegwerf-Ordner, ohne Uhr): `logs/` mit `2026-10-13.log`, `2026-10-14.log`, `2026-10-15.log`, `2026-10-28.log`, `notizen.txt`, `2026-10-01.log` als Symlink auf eine Datei außerhalb, Ordner `2026-10-02.log/` → `bereinigen(2026-10-28)` entfernt genau -13 und -14; -15, -28, `notizen.txt`, der Symlink, sein Ziel und der Ordner bleiben. Mutanten: Aufbewahrung 13 → -15 fehlt (rot), 15 → -14 bleibt (rot); `metadata` statt `symlink_metadata` → Ziel des Symlinks fehlt (rot).
- [ ] Unit Tagestakt (`tokio::time::pause`, gestellte Uhr): 23:59:59.500 → kein Aufruf; 00:00:00.000 des Folgetags, ein Takt → genau ein Aufruf mit dem neuen Datum; weitere Takte am selben Tag → keiner; zwei angemeldete Aufgaben → beide in Anmeldereihenfolge.
- [ ] Integrationstest (echtes Programm): `logs/2026-10-15.log` vorhanden, Start mit `LOCALCHATA_JETZT=2026-10-28T23:59:58+01:00`, `TZ=Europe/Berlin` → nach der Startzeile ist -15 noch da (Grenze: 15 ≥ 28 − 13); danach fehlt -15 spätestens 4 s nach der Startzeile (erwartet ≈ 2–3 s: 2 s bis Mitternacht plus höchstens ein Takt; Grenze 30 % darüber), und `logs/2026-10-29.log` enthält die `INFORMATION protokoll 1 Protokolldateien entfallen …`-Zeile. Die Mitternachtsregel selbst prüft der Takt-Unit-Test, nicht dieser Test.
- [ ] Probe-Projekt nicht ins Repository übernehmen (keine Crate gewählt); die Probe-Ergebnisse stehen hier.

## Quellen / Bezuege

- [A00015 Betriebsprotokoll](../Architektur/A00015-betriebsprotokoll.md), [A00003 Start](../Architektur/A00003-start.md); Aspekte [X00001](../Architektur/Aspekte/X00001-logging-und-diagnose.md), [X00007 Zeit](../Architektur/Aspekte/X00007-zeit.md)
- ADR-0016 (Takt-Form), ADR-0029, ADR-0030, ADR-0032, ADR-0040, ADR-0042
- `tracing-appender` 0.2.5, Quelle `src/rolling.rs` (gelesen 2026-10-28 aus dem lokalen Cargo-Register); Skill `dependency-probe`
