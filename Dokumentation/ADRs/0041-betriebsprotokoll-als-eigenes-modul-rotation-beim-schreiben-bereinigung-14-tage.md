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

## Ergänzungen

- **2026-10-28 (Abgleich I0018 ↔ Tagestakt, Fragen a–e des planer vor Karte #24) — Vertrag des Tagestakts:** Präzisiert Regel 6; ersetzt wird keine Regel. Anlass: R00008 (F0035) plant einen eigenen Takt der Fristpflege, R00014 (F0052) den gemeinsamen Tagestakt; beide sagen Verschiedenes zu Start, Startwert, Rückwärtsgang und Fehlerfall (WBS, „Offene Fragen“, Punkt „I0018 gegen den Tagestakt aus F0052“). Belege: `src/start/mod.rs:114–121` (der Anwesenheits-Takt startet nach dem Ende-Signal und **vor** der Startzeile), `src/live/takt.rs` (Takt-Form, `MissedTickBehavior::Delay`), [ADR-0030](0030-entfallen-beim-start-beim-tageswechsel-und-beim-verkuerzen-loeschen-unter-der-gespraechssperre.md) Regel 2 und Pre-Mortem 1, [ADR-0042](0042-stufen-ereigniskatalog-episoden-und-tagesgrenze-des-betriebsprotokolls.md) Regel 4 und Ergänzung 6, [A00003](../Architektur/A00003-start.md) Ergänzung „Vorlauf Betriebsprotokoll“ (Tagestakt **vor** der Startzeile).

  **(a) Wer taktet die Fristpflege?**
  - Option A1: **Die Fristpflege ist eine Tagesaufgabe am Tagestakt.** Stärke: ein Datumsvergleich, ein Test, eine feste Reihenfolge um Mitternacht (erst Bereinigung, dann Fristpflege). Schwäche: I0018 hängt in der Reihenfolge an F0052 (I0018 braucht I0021 ohnehin).
  - Option A2: Eigener Takt `fristpflege::takten` wie in R00008 geplant. Stärke: F0035 baut unabhängig von I0021. Schwäche: zwei Sekundentakte mit derselben Datumslogik und zwei Tests dafür; die Reihenfolge von Bereinigung und Fristpflege um Mitternacht ist zufällig; widerspricht Regel 6.
  - Option A3: Fristpflege als Sekundenaufgabe des Tagestakts mit eigenem Datumsvergleich. Stärke: der Takt bleibt einfach (nur Sekundenaufgaben). Schwäche: die Datumslogik steht wieder doppelt — genau das, was Regel 6 vermeiden wollte.
  - **Gewählt: A1.** In Kauf genommen: F0035 kann erst nach F0052 gebaut werden. Die reine Funktion „Datum gewechselt“ (R00008 B0188) gehört damit zum Tagestakt, nicht zur Fristpflege.

  **(b) Startet der Tagestakt vor oder nach der Startzeile?**
  - Option B1: **Vor der Startzeile**, direkt nach dem Ende-Signal und nach allen Start-Läufen, wie der Anwesenheits-Takt. Stärke: „bereit“ (Startzeile) heißt „alle Hintergrundaufgaben laufen“; deckt sich mit R00008, ADR-0030 Regel 2, A00003 und dem Code. Schwäche: ein Tageswechsel in den Millisekunden zwischen Takt und Startzeile schreibt seine Einträge **vor** `INFORMATION start` in die Datei.
  - Option B2: Nach `INFORMATION start` (R00014, Ablauf 1). Stärke: der Start-Eintrag steht vor jedem Tageswechsel-Eintrag. Schwäche: widerspricht R00008, ADR-0030 und A00003; zwischen Startzeile und Takt gibt es eine Spanne ohne Takt, in der die Instanz schon als bereit gilt.
  - Option B3: Erster Lauf des Takts synchron vor der Startzeile, danach Hintergrund. Stärke: kein Fenster. Schwäche: zusätzlicher Startschritt ohne Gewinn, sobald (c) gilt.
  - **Gewählt: B1.** Mit (c) hängt die Richtigkeit nicht mehr am Startpunkt des Takts (jede Aufgabe kennt ihr Datum), die Wahl ist also nur noch Einheitlichkeit. In Kauf genommen: `INFORMATION start` ist nicht garantiert der erste Eintrag nach den Start-Läufen. Reihenfolge im Start: … → Protokoll öffnen → Panik-Haken → **Bereinigung (Start)** → Laden → **Fristpflege (Start)** → Binden → Ende-Signal → Anwesenheits-Takt und **Tagestakt** (beide bewacht) → Startzeile → `INFORMATION start`.

  **(c) Womit beginnt „seit dem letzten Lauf“?**
  - Option C1: Ein gemeinsames `zuletzt`, vorbelegt mit dem Datum beim Start des Takts. Schwäche: liegt Mitternacht zwischen Start-Lauf und Takt, läuft der neue Tag nie (übersprungener Tag bis zum nächsten Wechsel).
  - Option C2: Ein gemeinsames `zuletzt`, vorbelegt mit dem Datum der Start-Bereinigung. Schwäche: sah die Start-Fristpflege (nach dem Laden) schon den neuen Tag, pflegt der Takt ihn ein zweites Mal — eine zweite `ERROR frist Tag nicht entfallen` binnen 1 s, gegen R00008 „am selben Tag nie ein zweites Mal“.
  - Option C3: **`zuletzt` je Tagesaufgabe**, vorbelegt mit dem `heute`, das ihr Start-Lauf als Argument bekam. Stärke: jede Aufgabe läuft genau einmal je Datum, gleich wo Mitternacht in den Start fällt; kein Zwang für den Start, ein einziges `heute` zu bilden. Schwäche: Zustand je Aufgabe statt eines Werts; der Start muss das Datum, das er übergab, bei der Anmeldung wiederholen.
  - Option C4: Der Takt macht auch die Start-Läufe (`zuletzt = None`). Stärke: eine Datumsquelle. Schwäche: Bereinigung läuft vor dem Laden, Fristpflege nach dem Laden und beide vor der Startzeile — das ist kein einzelner Takt; die Anlässe `Start` und `Tageswechsel` (`grund=`) wären nicht mehr zu unterscheiden.
  - **Gewählt: C3.** In Kauf genommen: ein Datum je Aufgabe im Takt. Ein Start, der ein einziges `heute` für beide Start-Läufe bildet, erfüllt den Vertrag ebenfalls.

  **(d) Was gilt, wenn die Uhr zurückgestellt wird?**
  - Option D1: **Vergleich auf Ungleichheit** (`heute ≠ zuletzt`). Stärke: nach einer Korrektur der Uhr (auch einer, die Jahre vorging — ADR-0030 Pre-Mortem 1) läuft jede Aufgabe am nächsten Wechsel wieder; Bereinigung und Fristpflege löschen mit einem früheren `heute` nie mehr als vorher (Regel 4 hier, ADR-0029). Schwäche: eine Uhr, die um Mitternacht hin- und herspringt, löst je Sprung einen Lauf aus — und je Sprung wieder die ERROR-Zeile einer nicht löschbaren Datei.
  - Option D2: Nur vorwärts (`heute > zuletzt`, `zuletzt` als Höchstwert). Stärke: kein Lauf bei Hin- und Hersprüngen. Schwäche: ging die Uhr einmal Jahre vor und wird korrigiert, läuft keine Tagesaufgabe mehr, bis das falsche Datum wieder erreicht ist — die Fristen gelten dann stillschweigend nicht.
  - Option D3: D1 plus eigene `WARNING zeit Datum zurückgesprungen`. Stärke: der Rückwärtsgang ist im Protokoll benannt. Schwäche: neuer Katalogeintrag für einen Fall, den die Einträge der Läufe (Datei des älteren Tages, Rotation Regel 2) schon zeigen.
  - **Gewählt: D1.** In Kauf genommen: Läufe je Sprung bei springender Uhr; der Rückwärtsgang hat keine eigene Zeile. Der Anlass bleibt `Tageswechsel` (`grund=tageswechsel`), auch rückwärts.

  **(e) Beendet eine Panik in einer Tagesaufgabe den Tagestakt?**
  - Option E1: Ja — die Wache meldet `Hintergrundaufgabe tagestakt beendet`. Stärke: einfach, laut. Schwäche: ein Fehler in der Fristpflege beendet auch Bereinigung und Episodenende; die 14 Tage und die Episoden gelten nicht mehr bis zum Neustart.
  - Option E2: **Nein — je Aufgabe abgefangen, der Takt läuft weiter, erneut beim nächsten Wechsel.** Stärke: Aufgaben fallen einzeln aus, nicht gemeinsam; eine Panik, die an einer Datei hängt, kommt höchstens einmal je Datum wieder (zwei Zeilen am Tag), nie je Sekunde. Schwäche: ein Programmfehler, der jeden Tag auftritt, steht jeden Tag neu im Protokoll, statt einmal als „ausgefallen“.
  - Option E3: Abfangen und die Aufgabe bis zum Neustart abmelden (Name in `aufgaben_ausgefallen`). Stärke: der Health-Check zeigt den Ausfall dauerhaft. Schwäche: eine einmalige Panik (eine seltsame Datei) setzt die Frist bis zum Neustart aus.
  - **Gewählt: E2.** In Kauf genommen: täglich wiederkehrende Programmfehler kommen täglich wieder; erkennbar über `exceptions` im Health-Check (ADR-0043), nicht über `aufgaben_ausgefallen`.

  **Regeln (Vertrag des Tagestakts, ergänzen Regel 6):**
  1. **Ein Takt, zwei Arten von Aufgaben.** `tagestakt::takten(uhr, ende, PERIODE = 1 s, tagesaufgaben, sekundenaufgaben)`. Eine Tagesaufgabe hat Name, Folge-Text, `zuletzt: NaiveDate` und `ausfuehren(heute)`; angemeldet sind in dieser Reihenfolge `bereinigen` (I0021) und `fristpflege` (I0018). Sekundenaufgabe ist `episoden_schliessen(jetzt)` (F0055). Die Fristpflege baut keinen eigenen Takt.
  2. **Fällig** (rein, `datum_gewechselt(zuletzt, jetzt) -> Option<NaiveDate>`): `heute = jetzt.date_naive()` (lokal, Uhr-Provider); fällig ⇔ `heute ≠ zuletzt`. Ungleichheit, nicht „größer“; Datum, nicht Uhrzeit.
  3. **Startwert**: `zuletzt` jeder Tagesaufgabe ist das `heute`, das ihr Start-Lauf bekam (Bereinigung: das Datum aus „Start“ nach dem Öffnen des Protokolls; Fristpflege: das Datum der Pflege beim Start nach dem Laden).
  4. **Ablauf je Takt**: zuerst die Sekundenaufgaben; dann je Tagesaufgabe in Anmeldereihenfolge: fällig? → `zuletzt = heute` **vor** dem Aufruf → `ausfuehren(heute)` über `spawn_blocking`, abgewartet, bevor die nächste Aufgabe dran ist. Weil `zuletzt` vor dem Aufruf steht, wird eine gescheiterte Aufgabe erst beim nächsten Wechsel wieder versucht (wie ADR-0030 Regel 4).
  5. **Gescheiterte Tagesaufgabe** (Panik oder Abbruch in ihrem `spawn_blocking`): der Takt läuft weiter, die übrigen Aufgaben dieses Takts laufen. Bei Panik schreibt der Haken `EXCEPTION programmfehler` mit `ort` (ADR-0042 Regel 4) und der Takt `ERROR programmfehler Tagesaufgabe <name> gescheitert — <folge>; erneuter Versuch beim nächsten Tageswechsel | operation=<name> grund=panik`; bei Abbruch ohne Panik schreibt der Takt dieselbe Zeile als `EXCEPTION` mit `grund=abgebrochen` (wie ADR-0042 Ergänzung 6). Folge-Texte: `bereinigen` „alte Protokolldateien bleiben bis dahin liegen“, `fristpflege` „Tage jenseits der Frist bleiben bis dahin liegen“. Kein Eintrag in `aufgaben_ausgefallen` — die Aufgabe ist nicht beendet.
  6. **Gescheiterte Sekundenaufgabe** (Panik, abgefangen mit `catch_unwind` um den synchronen Aufruf): sie wird abgemeldet — sie liefe sonst jede Sekunde in dieselbe Panik —, der Takt schreibt `ERROR programmfehler Hintergrundaufgabe episoden beendet — Episoden enden nicht mehr` und trägt `episoden` in `aufgaben_ausgefallen` ein (ADR-0042 Regel 4, ADR-0043). Die Tagesaufgaben laufen weiter.
  7. **Der Takt selbst** läuft bewacht (ADR-0042 Regel 4, Name `tagestakt`); endet er vor dem Ende-Signal, gilt die Zeile der Wache.

  **Erste Tests (Vorschlag an den planer, Unit mit `tokio::time::pause`, gestellter Uhr und aufzeichnenden Aufgaben, Protokoll in eine Speicher-Senke):**
  - *Grenze*: `zuletzt` 2026-10-28; Uhr 2026-10-28T23:59:59.999 → kein Aufruf; 2026-10-29T00:00:00.000, ein Takt → je Aufgabe genau ein Aufruf mit 2026-10-29, in Anmeldereihenfolge; zehn weitere Takte am 29. → keiner. Mutanten „jeder Takt“, „Uhrzeit statt Datum“ → rot.
  - *Rückwärts*: danach Uhr 2026-10-28T23:00 → je Aufgabe genau ein Aufruf mit 2026-10-28; weitere Takte → keiner; wieder 2026-10-29T00:00:00.000 → je einer mit 2026-10-29. Mutante „`>` statt `≠`“ → rot.
  - *Startwert*: Aufgabe A mit `zuletzt` 2026-10-28, B mit `zuletzt` 2026-10-29 (Mitternacht fiel zwischen die Start-Läufe); Uhr 2026-10-29T00:00:00.500, ein Takt → nur A mit 2026-10-29; B nicht. Mutanten „ein gemeinsames `zuletzt` aus der ersten Aufgabe“ (B läuft) und „`zuletzt` = Datum beim Start des Takts“ (A läuft nicht) → je rot.
  - *Panik*: A gerät bei jedem Aufruf in Panik, B zeichnet auf; Wechsel auf den 29. → B läuft mit 2026-10-29, die Senke hat genau eine `ERROR programmfehler Tagesaufgabe A gescheitert …`-Zeile mit `operation=A grund=panik`; zehn weitere Takte am 29. → kein neuer Aufruf von A, keine weitere Zeile; Wechsel auf den 30. → A und B laufen wieder. Mutanten „Takt endet bei Panik“ (30. fehlt), „`zuletzt` erst nach Erfolg“ (A je Takt, Zeilen je Sekunde), „alle Aufgaben in einem `spawn_blocking`“ (B fehlt am 29.) → je rot.
  - *Sekundenaufgabe*: sie gerät beim dritten Takt in Panik → genau eine `ERROR programmfehler Hintergrundaufgabe episoden beendet`-Zeile, `episoden` in den ausgefallenen, kein weiterer Aufruf; ein folgender Tageswechsel ruft die Tagesaufgaben weiter.
  Die Integration über Mitternacht (Regel 6 „Folgeentscheidungen“, Z3 aus R00008) bleibt unverändert; kein neuer Testknopf — die Panik einer Tagesaufgabe prüft die Unit, den echten Panikweg über `spawn_blocking` prüft schon `LOCALCHATA_TESTPANIK=verlauf` (ADR-0042).

  **Pre-Mortem (Ergänzung):** … die Uhr springt nach einem NTP-Abgleich um Mitternacht mehrmals hin und her, und eine nicht löschbare Datei füllt das Protokoll. Früh-Indikator: mehrere `grund=tageswechsel`-Zeilen desselben Channels binnen Minuten. Gegenmaßnahme: hingenommen — je Sprung eine Zeile, begrenzt durch die Zahl der Sprünge und das Tagesbudget (ADR-0042 Regel 6).
- **2026-10-28 (Observability-Review #24, Befund M1) — Regel 3 und Regel 2 nach dem Öffnen je Eintrag:** Mit [ADR-0040](0040-betriebsprotokoll-datei-je-lokalem-tag-unter-logs-eine-zeile-je-eintrag.md), Ergänzung „gelöschte Tagesdatei“ (Option C), liest sich Regel 3: unter dem `Mutex` je Eintrag `open` (append, create) → ein `write_all` der fertigen Zeile → Größe aus den Metadaten → `close`; vier Systemaufrufe statt einem, gerechnet ≈ 0,4/s auf Information und ≈ 1.000/s auf Debug unter Volllast — tragbar. Die Sperre wird weiter nie über ein `.await` gehalten, der Panik-Haken nimmt sie weiter nur mit `try_lock`. Regel 2 („die offene wird geschlossen, `logs/<datum>.log` geöffnet“) entfällt als Mechanik: das Datum des Eintrags bestimmt den Pfad, sonst nichts; Datum rückwärts hängt an die ältere Datei an wie bisher. Die Tests aus „Folgeentscheidungen“ (Mitternacht, Zeitumstellung) gelten unverändert.
- **2026-10-28 (Observability-Review #24, Befund O7) — `INFORMATION start` steht vor der Startzeile:** Ändert die Reihenfolge aus Ergänzung (b) an ihrem Ende; B1 (Tagestakt vor der Startzeile) bleibt. Gebaut ist `INFORMATION start` **vor** `startzeile_schreiben` (`src/start/mod.rs:272–286`); R00014 und ADR-0042 Ergänzung 7 sagen „nach“.
  - Option A: **vor der Startzeile** (Code). Stärke: die Startzeile auf stdout ist das Bereit-Signal, auf das Tests, Director und Dienstverwaltung warten; wer sie gesehen hat, findet `INFORMATION start` schon in der Datei — ohne Abfragen in einer Schleife, ohne Zeitfenster. Schwäche: hängt das Schreiben (Platte), erscheint die Startzeile entsprechend später.
  - Option B: nach der Startzeile (Anforderung). Stärke: die Zeile beschreibt eine Instanz, die schon als bereit gilt. Schwäche: ein Test, der nach der Startzeile die Datei liest, kann die Zeile verpassen und muss abfragen, bis sie da ist — genau die Art Flake, die `gemeinsam/gelerntes.md` („am beobachtbaren Urteil festmachen, nicht an Wartezeiten“) vermeiden will.
  - **Gewählt: A.** In Kauf genommen: eine hängende Platte verzögert das Bereit-Signal (im Rückfall geht die Zeile auf stderr und verzögert nichts). Reihenfolge im Start ab jetzt: … → Ende-Signal → Anwesenheits-Takt und Tagestakt → **`INFORMATION start`** → Startzeile. Der planer zieht R00014 („Start und Ende“) nach.
- **2026-10-28 (Nachzug, gesammelte Doku-Punkte zu Karte #28) — Regeln 2, 3 und 7 so gelesen, wie gebaut:** Nur Angleichung, keine neue Entscheidung; die Regeln oben bleiben als Stand ihres Tages stehen.
  1. **Regel 2 und 3** gelten in der Fassung der Ergänzung „Regel 3 und Regel 2 nach dem Öffnen je Eintrag“ (Observability-Review #24, M1): es gibt **keine** offen gehaltene Tagesdatei mehr; je Eintrag `open` (append, create) → ein `write_all` → Größe aus den Metadaten → `close` (`src/protokoll/senke.rs`). Wo Regel 2 „die offene wird geschlossen“ und Regel 3 „ein `write_all` … auf die Datei“ sagt, gilt diese Fassung; „Rotation beim Schreiben“ im Titel heißt seither nur: das Datum des Eintrags bestimmt den Pfad.
  2. **Regel 7 und der Integrationstest unter „Folgeentscheidungen“**: Einzahl wie gebaut (Rechtschreib-Review #24, `src/protokoll/mod.rs:479–486`) — `INFORMATION protokoll 1 Protokolldatei entfällt (…)`, ab zwei `<n> Protokolldateien entfallen (…)`. Der Integrationstest erwartet für die eine Datei `2026-10-15.log` also `INFORMATION protokoll 1 Protokolldatei entfällt …`.
  3. **Spanne bei genau einer Datei** (Beobachtung): gebaut ist `(2026-10-14 bis 2026-10-14; …)` (`src/protokoll/mod.rs:849`), die Frist-Zeile derselben Art nennt einen einzelnen Tag allein (`spanne`, `src/gespraeche/fristeintrag.rs:124–131`). Entscheidung: wie die Frist — bei einer Datei nur das Datum, `(2026-10-14; Aufbewahrung 14 Tage; Start)`. Option „lassen“ verworfen: zwei Zeilen gleicher Bauart, zwei Formen; Kosten eine Funktion und ein Test. Vorschlag an den planer (Konsolidierung, keine eigene Karte nötig).
