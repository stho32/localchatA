# ADR-0031: Frist setzen über `PUT /api/channels/{name}/frist` — jeder mit Sitzung, Verkürzen nur mit Bestätigung, verteilt als Channel-Wechsel

**Status**: Akzeptiert
**Datum**: 2026-10-28
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #12 I0018)
**Kontext-Anforderung**: — (WBS I0018)
**Erprobt durch**: — (überträgt gebaute Mechanik: Route unter `grenzschutz` mit Extraktor `Teilnehmer` und begrenztem JSON-Körper — `src/http/channels.rs`, `src/http/koerper.rs`; Schreiben über `einstellungen.toml.neu` + `rename` mit Erhalt unbekannter Schlüssel — `src/gespraeche/einstellungen.rs` `schreiben`; Channel-Wechsel unter der Sperre des Verzeichnisses — ADR-0023)

## Kontext

- Vision: „Die Frist ist je Channel wählbar“; „Mensch und KI sind gleichberechtigt“; ausdrücklich nicht Ziel: „keine Benutzerverwaltung und keine Rechte“. Einsatzraum LAN, eine gemeinsame Instanz.
- [ADR-0021](0021-channel-anlegen-jeder-mit-sitzung-grenzen-je-herkunft-und-gesamt.md): Channels legt jeder mit Sitzung an; Grenzen statt Rechte. [ADR-0014](0014-grenzschutz-host-origin-sicherheitsheader.md): jede nicht lesende Methode prüft Host und Origin.
- [ADR-0022](0022-channel-einstellungen-je-channel-als-toml.md): `frist_tage` in `einstellungen.toml`, geschrieben nur vom Programm über `rename`; unbekannte Schlüssel bleiben erhalten; eine defekte Datei bleibt unangetastet, und bei ihr entfällt nichts (Regel 6).
- [ADR-0023](0023-channels-als-zweites-thema-im-ereignisstrom.md) Regel 1/3: `stand` zählt mit jeder Änderung (ab I0018) hoch; ein Wechsel `{"art":"channel","stand","channel"}` ersetzt den Channel nach `name`.
- [ADR-0029](0029-frist-in-kalendertagen-heute-zaehlt-mit-1-bis-3650.md) (Wert 1–3650), [ADR-0030](0030-entfallen-beim-start-beim-tageswechsel-und-beim-verkuerzen-loeschen-unter-der-gespraechssperre.md) (Verkürzen löscht sofort, endgültig).
- Gelerntes des Projekts (AI-Team/gelerntes.md, #1): jede schreibende Route ist LAN-Angriffsfläche; Grenzschutz vor der Route.

## Optionen

### Option A: Eigene Unterressource `PUT /api/channels/{name}/frist` mit `{"tage": n, "bestaetigt"?: true}`
- Vorteile: genau ein Wert, idempotent; die Prüfung ist eindeutig (ein Feld); keine Verwechslung mit Feldern, die niemand ändern darf (`angelegt`, `angelegt_von`); passt zu `…/nachrichten` als Unterressource (ADR-0025).
- Nachteile: jede weitere änderbare Einstellung bekäme eine eigene Unterressource.

### Option B: `PATCH /api/channels/{name}` mit `{"frist_tage": n}` (Teil-Änderung des Channels)
- Vorteile: eine Route für alle künftigen Änderungen am Channel (Beschreibung ändern o. Ä.).
- Nachteile: Merge-Regeln müssen festgelegt werden (`null` = löschen oder ignorieren? unbekannte Felder?); nicht änderbare Felder im selben Objekt verlangen eigene 422-Fälle; in Vision und WBS ist keine weitere Änderung geplant.

### Option C: `PUT /api/channels/{name}` mit dem ganzen Channel
- Vorteile: klassisches REST, die Darstellung ist die aus Liste und Ereignis.
- Nachteile: der Aufrufer muss alle Felder mitschicken — ein Agent mit veraltetem Stand überschreibt eine inzwischen geänderte Beschreibung; Felder wie `angelegt_von` dürfen sich nicht ändern und müssten doch gesendet werden.

### Wer darf ändern
- **Jeder mit Sitzung** — Stärke: Vision „keine Rechte“, wie Anlegen; Schwäche: jeder im LAN kann Verlauf löschen.
- **Nur der Anleger** (`angelegt_von`) — Stärke: wer den Channel gemacht hat, bestimmt. Schwäche: ein Recht, das die Vision ausschließt; Kennungen werden frei, sobald jemand geht (ADR-0012), ein Späterer gleichen Namens erbt es; Channels ohne bekannten Anleger wären nie änderbar.
- **Nur von der Loopback-Herkunft** (der Rechner der Instanz) — Stärke: der Betreiber hat die Hand drauf. Schwäche: ein Recht nach Herkunft; Agenten auf anderen Rechnern sind nicht mehr gleichberechtigt.

## Entscheidung

**Gewählt: Option A; ändern darf jeder mit Sitzung; Verkürzen, das Tage löscht, verlangt `bestaetigt: true`.**

Es überwiegt, dass ein einzelner Wert eine schmale, eindeutige Route bekommt und die Vision Rechte ausschließt. Den Schaden eines versehentlichen Verkürzens fängt die Bestätigung ab: ohne sie antwortet der Server, was entfallen würde, und tut nichts — dieselbe Antwort liefert der Oberfläche die Zahlen für ihre Rückfrage. In Kauf genommen: jeder Teilnehmer im LAN kann nach Bestätigung Verlauf eines Channels endgültig löschen; sichtbar bleibt es auf stderr mit Kennung.

Regeln:

1. **Route** `PUT /api/channels/{name}/frist` unter `grenzschutz`, Extraktor `Teilnehmer`, Körper begrenzt wie `POST /api/channels`. Körper `{"tage": <Ganzzahl>, "bestaetigt": <bool, optional, Standard false>}`.
2. **Prüfreihenfolge** und Antworten:
   1. ohne Sitzung 401 · fremder `Origin` 403 · ohne JSON-`Content-Type` 415 · zu groß 413 · kein JSON 400 — wie bei `POST /api/channels`;
   2. unbekannter oder ungültiger Channelname → 404 `{fehler}` („`# name` gibt es nicht.“);
   3. `tage` fehlt, ist keine Ganzzahl oder liegt außerhalb 1–3650; `bestaetigt` vorhanden, aber kein Wahrheitswert → 422 `{fehler, regel: "frist"}` bzw. `regel: "bestaetigt"` — abgewiesen, nicht umgeformt (Hausregel ADR-0013); `"7"` und `7.0` sind keine Ganzzahl;
   4. Einstellungen des Channels defekt oder nicht lesbar (beim Start erkannt, ADR-0022 Regel 3) → 409 `{fehler, regel: "einstellungen-defekt", datei}` — „`channels/name/einstellungen.toml` ist defekt; von Hand reparieren und neu starten.“ Die Datei wird nie überschrieben;
   5. `tage` gleich der geltenden Frist → 200 wie unten mit `entfallen: {tage: 0, nachrichten: 0}`, **ohne** Schreiben, ohne neuen `stand`, ohne Ereignis;
   6. würden Tage entfallen (es gibt auf der Platte Tage vor `heute − (tage − 1)`, ADR-0029) und `bestaetigt` ist nicht `true` → 409 `{fehler, regel: "bestaetigen", entfaellt: {tage, nachrichten, bis}}` — `tage` = Zahl der Tagesdateien, `nachrichten` = Sätze dieser Tage im Index, `bis` = jüngster entfallender Tag (`JJJJ-MM-TT`); nichts geändert;
   7. sonst: `einstellungen.toml` schreiben (gelesene Bytes als `vorher`, unbekannte Schlüssel bleiben, `.neu` + `rename`); scheitert das → 500 `{fehler}`, Frist, Datei und `stand` unverändert, eine Zeile auf stderr; bei Erfolg Frist im Verzeichnis setzen, `stand + 1`, Wechsel veröffentlichen — alles unter der Sperre des Verzeichnisses;
   8. danach (Sperre des Verzeichnisses frei) `pflegen(heute, FristGesetzt { von })` für diesen Channel (ADR-0030) → 200 `{channel, entfallen: {tage, nachrichten}}`.
   Verlängern löscht nichts und braucht keine Bestätigung; es lädt keine schon entfallenen Tage zurück.
3. **Channel-Darstellung** in Antwort, Liste, Stand und Wechsel wächst um `frist_tage`: die geltende Frist (aus der Datei, fehlt sie: 7) oder `null`, wenn die Einstellungsdatei defekt bzw. nicht lesbar ist — `null` heißt „nichts entfällt“. Neue Felder kommen hinzu, keines wird umgedeutet (ADR-0025 Regel 3).
4. **Live**: eine geänderte Frist ist ein Channel-Wechsel nach ADR-0023 (`{"art":"channel","stand","channel"}`, ersetzt nach `name`). Entfallene Tage sind kein Ereignis (ADR-0030 Regel 5).
5. **Protokoll**: jede wirksame Änderung schreibt eine Zeile auf stderr — `channels/allgemein: Frist 30 → 7 Tage, gesetzt von shoff@127.0.0.1` —, gefolgt von der Pflege-Zeile aus ADR-0030 Regel 6.
6. **Anlegen bleibt**: `POST /api/channels` nimmt keine Frist (Designsystem 4.3: Name, optional Beschreibung); `einstellungen.toml` bekommt `frist_tage` erst mit der ersten Änderung — fehlt der Schlüssel, gilt 7.
7. **Blockierende Arbeit** (Datei lesen und schreiben, Tage auflisten, Löschen) über `spawn_blocking` (A00004). Sperrordnung: Verzeichnis (schreibend, Schritte 2.2–2.7) vor Gespräch (Schritt 2.8, ohne Verzeichnis-Schreibsperre).

## Konsequenzen

**Positiv:**
- Ein Agent setzt eine Frist mit einem Aufruf; ein versehentliches Verkürzen kostet ihn eine 409 mit Zahlen, keine Daten.
- Die Oberfläche braucht keine zweite Route für die Vorschau: der erste `PUT` ohne Bestätigung liefert sie.
- Anlegen, Liste und Strom bleiben unverändert in der Form; nur ein Feld kommt hinzu.

**Negativ / Inkauf zu nehmen:**
- Zwischen 409-Vorschau und bestätigtem `PUT` kann ein Tageswechsel liegen; dann entfällt ein Tag mehr, als die Vorschau nannte. Die 200 nennt die tatsächlichen Zahlen.
- Die 409 `bestaetigen` erscheint im Browser als Konsolenzeile „Failed to load resource … 409“ — sie gehört in die Liste vorgeschriebener Antworten des Konsolenkriteriums (AI-Team/planer.md, Gelerntes (5)).
- Eine reparierte Einstellungsdatei wirkt erst nach einem Neustart (ADR-0022 Regel 4).

**Pre-Mortem (ein Monat nach dem Start gescheitert, weil …):**
1. … ein Agent in einer Schleife alle Channels auf Frist 1 setzt und dabei `bestaetigt: true` gleich mitschickt. Früh-Indikator: stderr-Zeilen „Frist … → 1 Tage, gesetzt von <agent>“. Gegenmaßnahme: Protokoll mit Kennung (Regel 5); die Anleitung für Agenten (I0019) sagt „Frist nur auf ausdrücklichen Wunsch eines Menschen, nie `bestaetigt` ohne vorherige 409“. Rechte gibt es nach Vision nicht — hingenommen.
2. … ein Agent im Takt zwischen zwei Werten wechselt und jeder Wechsel ein Ereignis an alle Browser schickt. Früh-Indikator: viele `channel`-Ereignisse mit derselben Kennung auf stderr. Gegenmaßnahme: gleiche Werte erzeugen kein Ereignis (Regel 2.5); eine Rate je Herkunft wäre eine eigene Interaction, wie für Senden — hingenommen.
3. … eine Hand-Reparatur der Einstellungsdatei während des Laufs vom nächsten `PUT` überschrieben wird. Früh-Indikator: Betreiber-Meldung. Gegenmaßnahme: ADR-0022 Regel 4 (Änderungen von Hand nur bei gestopptem Programm); die 409 `einstellungen-defekt` verhindert wenigstens das Überschreiben einer beim Start defekten Datei.

**Folgeentscheidungen / Aufgaben:**
- [ ] Erster Unit-Test (Gespräche, Speicherablage, gestellte Uhr 2026-10-28, aufzeichnender Rückruf): Channel mit Tagen -20 bis -28; `frist_setzen(allgemein, 30, nicht bestätigt)` → gesetzt ohne Bestätigung (nichts entfällt), Rückruf `stand` +1, Datei enthält `frist_tage = 30` und den unbekannten Schlüssel `farbe`; `frist_setzen(allgemein, 7, nicht bestätigt)` → `Bestaetigen { tage: 2, nachrichten, bis: 2026-10-21 }`, nichts geändert, kein Rückruf; bestätigt → Frist 7, zwei Tage entfallen; `frist_setzen(allgemein, 7, …)` noch einmal → kein Schreiben, kein Rückruf. Ersetzt: Logablage (Speicher), Uhr, Rückruf.
- [ ] Erster Integrationstest (echtes Programm): ohne Header 401; fremder `Origin` 403; ohne JSON 415; `tage` 0, 3651, -1, `"7"`, `7.5`, fehlt → je 422 `regel: "frist"`, `tage` 1 und 3650 → 200 (Mutanten 0/2 und 3649/3651 werden rot); `bestaetigt: "ja"` → 422 `regel: "bestaetigt"`; unbekannter Channel → 404; Neustart → `GET /api/channels` nennt die gesetzte Frist.
- [ ] Integrationstest Live: A liest den Strom, B setzt die Frist → A erhält `channel` mit `frist_tage` und `stand` +1 spätestens 2 s nach Erhalt der 200 (erwartet wenige ms); derselbe Wert noch einmal → kein Ereignis innerhalb 1 s.

## Quellen / Bezuege

- [A00004 HTTP-Schnittstelle](../Architektur/A00004-http-schnittstelle.md), [A00012 Channelverzeichnis](../Architektur/A00012-channelverzeichnis.md), [A00013 Einstellungsformat](../Architektur/A00013-einstellungsformat.md), [A00011 Live-Kanal](../Architektur/A00011-live-kanal.md); Aspekte [X00002 Benutzerrechte](../Architektur/Aspekte/X00002-benutzerrechte.md), [X00004 Fehlerbehandlung](../Architektur/Aspekte/X00004-fehlerbehandlung.md)
- ADR-0012, ADR-0013, ADR-0014, ADR-0021, ADR-0022, ADR-0023, ADR-0025, ADR-0029, ADR-0030; `src/http/channels.rs`, `src/http/koerper.rs`, `src/gespraeche/einstellungen.rs`
- Vision R00000 (Frist wählbar, gleichberechtigt, keine Rechte); WBS I0018; Geschwister kanbanD `PUT /api/einstellungen` (`web/app.js:1043`) als Vorbild für Einstellungen per `PUT`
