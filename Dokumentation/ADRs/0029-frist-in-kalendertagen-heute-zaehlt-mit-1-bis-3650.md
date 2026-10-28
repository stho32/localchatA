# ADR-0029: Frist in ganzen Kalendertagen nach lokalem Datum, heute zählt mit, 1 bis 3650, Standard 7

**Status**: Akzeptiert
**Datum**: 2026-10-28
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #12 I0018)
**Kontext-Anforderung**: — (WBS I0018; baut auf R00002 „heute zählt mit“ und B0022)
**Erprobt durch**: gebaute Regel `src/gespraeche/frist.rs` `tage_in_frist` mit Tests (Frist 7: der 21. bleibt am 27. drin, der 20. nicht; Frist 1; Grenze über den Monat) und R00002, Kriterium „Frist-Grenze, heute zählt mit“ (grün seit Karte #2)

## Kontext

- Vision, Zielbild „Das Log ist das Gedächtnis“: „Die Logs rotieren täglich; Tage, die älter sind als die Aufbewahrungsfrist des Channels, entfallen. Die Frist ist je Channel wählbar, Standard sind 7 Tage.“ WBS I0018: „Frist je Channel wählbar, Standard 7 Tage; Logtage älter als die Frist entfallen“.
- [ADR-0003](0003-ablage-layout-im-ordner.md): eine Tagesdatei `JJJJ-MM-TT.log` je Tag nach dem **lokalen** Datum des Servers bei der Nachrichtenzeit; Frist-Pflege ist Löschen ganzer Dateien. Folgeaufgabe dort: „Genaue Grenze der Frist (zählt heute mit?)“.
- Für das Laden schon entschieden (WBS-Notiz I0002, R00002): **heute zählt mit** — Frist 7 am 2026-10-27 lädt 2026-10-21 bis 2026-10-27; Tage nach heute werden mitgeladen. Gebaut in `tage_in_frist` (`src/gespraeche/frist.rs:10`).
- [ADR-0022](0022-channel-einstellungen-je-channel-als-toml.md): `frist_tage` ist ein optionaler Schlüssel in `einstellungen.toml`, fehlt = 7; eine defekte Datei verhindert jedes Entfallen in diesem Channel (Regel 6).
- [ADR-0006](0006-verlaufsindex-im-speicher.md): der Index hält alle Tage in der Frist im Speicher (≈ 2 × Loggröße); die Frist begrenzt das Wachstum.
- Aspekt [Zeit](../Architektur/Aspekte/X00007-zeit.md): „jetzt“ nur über den Uhr-Provider, Datum = lokales Datum der Uhr samt Versatz.

## Optionen

### Option A: Ganze Kalendertage nach lokalem Datum, heute zählt mit
Frist `n` hält heute und die `n − 1` Vortage. Tag `D` entfällt genau dann, wenn `D < heute − (n − 1)`, also `D ≤ heute − n`.
- Vorteile: dieselbe Regel, die Laden schon benutzt (eine Funktion für Laden und Entfallen, nie zwei Grenzen); die Grenze ist ein Datum, kein Zeitpunkt — Mitternacht und Zeitumstellung ändern nichts an der Rechnung; „Frist 1“ heißt verständlich „nur heute“.
- Nachteile: kurz nach Mitternacht sind bei Frist 7 nur sechs volle Tage plus wenige Minuten da.

### Option B: Ganze Kalendertage, heute zählt nicht (Frist `n` = heute plus `n` volle Vortage)
- Vorteile: nie weniger als `n` volle Tage Verlauf; wörtlichste Lesart von „älter als 7 Tage“.
- Nachteile: widerspricht dem gebauten und abgenommenen Laden (R00002) — dann laden und löschen zwei verschiedene Grenzen, oder R00002 muss neu; „Frist 1“ hält zwei Tage.

### Option C: Frist als Dauer (`n × 24 h`) über die Nachrichtenzeit
- Vorteile: genau `n × 24 h` Verlauf zu jeder Uhrzeit.
- Nachteile: eine Tagesdatei liegt dann teils in, teils außerhalb der Frist — Entfallen hieße Dateien umschreiben, nicht löschen (gegen ADR-0003); die Zeitumstellung macht einen Tag 23 oder 25 Stunden lang.

## Entscheidung

**Gewählt: Option A**

Es überwiegt, dass Laden und Entfallen dieselbe gebaute und getestete Grenze teilen und die Regel an Dateinamen, nicht an Uhrzeiten hängt. In Kauf genommen: direkt nach Mitternacht ist der älteste behaltene Tag ein voller, der jüngste erst Minuten alt — bei Frist 7 also gut sechs Tage Verlauf.

Regeln:

1. **Einheit und Wert**: ganze Kalendertage, Ganzzahl **1 bis 3650**, Standard **7**. 3650 (≈ 10 Jahre) steht für „praktisch unbegrenzt“; ein echtes „nie“ gibt es nicht, weil der Index alles in der Frist im Speicher hält (ADR-0006). Außerhalb der Spanne, keine Ganzzahl oder ein anderer Typ: an der API 422 `{fehler, regel: "frist"}`, in `einstellungen.toml` ein Defekt (ADR-0022 Regel 3 und 6 — dann entfällt nichts).
2. **Grenze**: `aeltester_behaltener(heute, n) = heute − (n − 1)`; Tag `D` entfällt ⇔ `D < aeltester_behaltener`. **Heute** ist das lokale Datum von `uhr.jetzt()` (Zone des Servers, dieselbe, nach der die Tagesdatei gewählt wird). Heute und Tage nach heute (zurückgestellte Uhr) entfallen nie.
3. **Eine Funktion**: `tage_in_frist` (Laden) und die Auswahl der entfallenden Tage sind Komplement derselben reinen Operation in `src/gespraeche/frist.rs` — `teilen(tage, heute, n) → (behalten, entfallen)`.
4. **Gilt je Channel**; Direktgespräche (`direkt/`) bleiben bei 7 Tagen und entfallen in I0018 nicht — ihre Frist entscheidet I0012 (offene Frage in [A00006](../Architektur/A00006-gespraeche.md)).
5. **Rechenbeispiele** (Tagesdatei = lokales Datum der Nachrichtenzeit):

   | Frist | heute | ältester behaltener Tag | entfällt |
   |---|---|---|---|
   | 7 | 2026-10-28 | 2026-10-22 | 2026-10-21 und älter |
   | 1 | 2026-10-28 | 2026-10-28 | 2026-10-27 und älter |
   | 30 | 2026-11-02 | 2026-10-04 | 2026-10-03 und älter (über den Monat) |
   | 7 | 2026-10-26 | 2026-10-20 | 2026-10-19 und älter (der 25. hatte 25 Stunden — zählt als ein Tag) |
   | 7 | 2027-01-03 | 2026-12-28 | 2026-12-27 und älter (über das Jahr) |
   | 3650 | 2026-10-28 | 2016-10-31 | 2016-10-30 und älter |

   Eine Nachricht von 2026-10-21 23:59:59.999+02:00 steht in `2026-10-21.log` und entfällt bei Frist 7 um 2026-10-29 00:00:00.000 lokal, also nach gut sieben Tagen; eine von 2026-10-22 00:00:00.000 bleibt bis 2026-10-30 00:00 — der Abstand von genau einer Millisekunde ergibt einen Tag Unterschied, weil die Datei die Einheit ist.

## Konsequenzen

**Positiv:**
- Keine zweite Grenze im Code: Laden und Entfallen können nicht auseinanderlaufen (ein Mutant „heute zählt nicht“ wird an beiden Stellen zugleich rot).
- Die Prüfung ist eine reine Funktion über Datumswerte, deterministisch ohne Uhr testbar.

**Negativ / Inkauf zu nehmen:**
- 3650 Tage in einem lebhaften Channel können den Speicher füllen (ADR-0006: ≈ 1,1 GB je 1.000.000 Nachrichten). Die Frist ist kein Speicherschutz; der Health-Check zeigt `log_bytes`.

**Pre-Mortem (ein Monat nach dem Start gescheitert, weil …):**
1. … jemand „Frist 7“ als „7 volle Tage“ versteht und am Morgen Nachrichten von vor sieben Tagen vermisst. Früh-Indikator: Befund im Entwicklertest. Gegenmaßnahme: der Hinweis im Dialog sagt „Heute zählt mit“ ([ADR-0033](0033-frist-in-der-kopfzeile-dialog-mit-bestaetigung-beim-verkuerzen.md)); die Anleitung für Agenten (I0019) nennt die Regel.
2. … Laden und Entfallen doch verschiedene Grenzen rechnen, weil jemand die Formel ein zweites Mal schreibt. Früh-Indikator: Review. Gegenmaßnahme: Regel 3, ein Unit-Test, der für alle Tage einer Liste prüft, dass jeder genau in einer der beiden Mengen liegt.
3. … die Testuhr `LOCALCHATA_JETZT` in UTC gesetzt wird und das Datum kippt. Früh-Indikator: Test um 23:30 UTC rot. Gegenmaßnahme: „heute“ ist das Datum der lokalen Uhr (`TZ=Europe/Berlin` in Tests, AI-Team/umgebung.md); die Uhr wandelt jede Startzeit in die lokale Zone (`src/uhr.rs`).

**Folgeentscheidungen / Aufgaben:**
- [ ] Erster Unit-Test (rein, `frist.rs`): `teilen([2026-10-21, 2026-10-22, 2026-10-28, 2026-10-29], heute 2026-10-28, 7)` → behalten `[22, 28, 29]`, entfallen `[21]`; `n = 1` → entfallen `[21, 22]`; `n = 3650` → entfallen `[]`; jeder Tag in genau einer Menge. Mutanten „heute zählt nicht“ (behält den 21.) und „`n − 2`“ (wirft den 22.) werden rot.
- [ ] Spanne (Unit, Einstellungsformat und Prüfung an der Grenze): 1 und 3650 gültig, 0 und 3651 ungültig; Mutanten 0/2 und 3649/3651 als Grenze werden rot (Wertprüfung als Spanne, Lehre #5).

## Quellen / Bezuege

- [A00006 Gespräche](../Architektur/A00006-gespraeche.md), [A00012 Channelverzeichnis](../Architektur/A00012-channelverzeichnis.md), [A00013 Einstellungsformat](../Architektur/A00013-einstellungsformat.md); Aspekt [X00007 Zeit](../Architektur/Aspekte/X00007-zeit.md)
- ADR-0003 (Tagesdatei, Folgeaufgabe), ADR-0006 (Speicher), ADR-0022 (Regel 6); R00002 (heute zählt mit); `src/gespraeche/frist.rs`
- Vision R00000, Zielbild „Das Log ist das Gedächtnis“; WBS I0018
