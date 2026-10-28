---
id: A00009
name: Verlaufsindex
code: verlaufsindex
ebene: Komponente
eltern: A00006
repo: localchatA
status: Geplant
herkunft: dialog
quellen:
  - src/gespraeche/verlaufsindex.rs
verwandt: [A00007]
domaene: []
blackbox: reine Datenstruktur im Speicher; Messwerte und Grenzen in ADR-0006
---

# Verlaufsindex

## Verantwortung

Beantwortet Blockabrufe und Suchanfragen über alle Nachrichten in der Frist aus dem Speicher.

## Schnittstellen

- `tag_aufnehmen(gespraech, tag, rohtext)` — beim Laden und beim Tageswechsel.
- `anhaengen(gespraech, tag, satz)` — nach jedem Schreiben, unter der Sperre des Gesprächs.
- `block(abschnitt, n) -> Block { saetze, aelter, neuer }` mit `abschnitt` = `Neueste` | `Vor(zeit)` | `Nach(zeit)`, Sätze älteste zuerst ([ADR-0025](../ADRs/0025-verlauf-ueber-die-api-bloecke-mit-zeit-als-cursor.md)); `suchen(begriff, gespraeche, nach, n)`, `tag_entfernen(gespraech, tag)`.
- Kennzahlen für den Health-Check: Nachrichtenzahl, Bytes Rohtext, Bytes Suchkopie.

## Abhaengigkeiten

[Logformat](A00007-logformat.md) (Satzgrenzen, Text ohne Einrückung).

## Entscheidungen

- [ADR-0006: Verlaufsindex im Speicher über dem Rohtext der Tagesdateien](../ADRs/0006-verlaufsindex-im-speicher.md) — Akzeptiert, 2026-10-27
- [ADR-0025: Verlauf über die API — `GET /api/channels/<name>/nachrichten` in Blöcken, Nachrichtenzeit als Cursor in beide Richtungen](../ADRs/0025-verlauf-ueber-die-api-bloecke-mit-zeit-als-cursor.md) — Akzeptiert, 2026-10-28
- [ADR-0026: Verlauf im Browser als Fenster aus höchstens fünf Blöcken — Nachladen an beiden Enden, Anker von Hand; was „sofort“ und „flüssig“ messbar heißt](../ADRs/0026-verlauf-im-browser-als-fenster-aus-hoechstens-fuenf-bloecken.md) — Akzeptiert, 2026-10-28
- [ADR-0030: Entfallen beim Start, beim Tageswechsel im Sekundentakt und beim Verkürzen — Löschen der Tagesdatei unter der Gesprächssperre, dann Index](../ADRs/0030-entfallen-beim-start-beim-tageswechsel-und-beim-verkuerzen-loeschen-unter-der-gespraechssperre.md) — Akzeptiert, 2026-10-28

## Offene Fragen

- Was genau ist ein Treffer — Teilwort ohne Groß-/Kleinschreibung (Empfehlung, so geprobt), ganze Wörter, auch Absender und Anhangnamen? Fachlich, für den `planer` bei I0011.

## Notizen / Quellen

- Je Tagesdatei: Rohtext (`String`), Satzanfänge (`Vec<u32>`), Suchkopie in Kleinschreibung je Satz mit eigener Offset-Tabelle (Kleinschreibung ändert Bytelängen, z. B. `İ`).
- Suche läuft Tag für Tag von neu nach alt über alle sichtbaren Gespräche, sortiert die Treffer eines Tages nach Zeit und hört bei 50 auf.
- Erster Test (Unit, ohne Platte): drei Tage zu je 250 Sätzen aufnehmen; `block(g, None, 100)` liefert die letzten 100 des jüngsten Tages, `block(g, Some(zeit_des_ersten), 100)` greift über die Tagesgrenze; `suchen("GRÖSSE")` findet `größe`.
- **Ergänzung 2026-10-27 (Karte #2, I0002):** `anhaengen` wird nur nach erfolgreichem Schreiben aufgerufen ([ADR-0011](../ADRs/0011-anhaengen-mit-rueckbau-und-zeilenende-waechter.md)); damit ist der Rohtext im Speicher byte-gleich mit der Datei, und `log_bytes` im Health-Check ist die Summe der Rohtexte. Für I0002 genügt `tag_aufnehmen`, `anhaengen` und die Zählung; `block` und `suchen` gehören zu I0008/I0011, sind aber der natürliche Prüfweg der Unit-Tests von Gespräche.
- **Ergänzung 2026-10-28 (Karte #7, I0008):** `block` nach [ADR-0025](../ADRs/0025-verlauf-ueber-die-api-bloecke-mit-zeit-als-cursor.md) Regeln 2–4: exklusive Grenzen in beide Richtungen, Weg über die `BTreeMap` der Tage, binäre Suche über die Kopfzeiten der Satzanfänge eines Tages (Zeiten steigen im Tag, einziger Schreiber), Ergebnis als Bytebereiche bzw. Kopien der Sätze — das Deuten (`logformat::text`, Kennung, Zeit) liegt bei [Gespräche](A00006-gespraeche.md) bzw. im Logformat, nicht im Index. `aelter`/`neuer` ergeben sich aus der Lage der Grenze, ohne zu zählen. Kosten je Block unabhängig von der Länge des Verlaufs (ADR-0006: 0,06 ms im Release). Erster Test: ADR-0025, „Folgeentscheidungen“ (drei Tage zu 250 Sätzen, Grenze am Tageswechsel, Cursor zwischen zwei Sätzen).
- **Ergänzung 2026-10-28 (Karte #12, I0018):** `tag_entfernen(tag) -> Option<(nachrichten, bytes)>` nimmt einen Tag aus der `BTreeMap` — nur nach erfolgreichem Löschen der Datei ([ADR-0030](../ADRs/0030-entfallen-beim-start-beim-tageswechsel-und-beim-verkuerzen-loeschen-unter-der-gespraechssperre.md) Regel 4); `saetze_vor(tag)` zählt die Sätze der Tage vor einer Grenze für die 409-Vorschau. `block` braucht nichts Neues: ein Cursor in einem entfallenen Tag läuft über die `BTreeMap` zum nächsten vorhandenen, `aelter` wird `false`, sobald der älteste Tag fehlt. Erster Test: Index mit drei Tagen, den ältesten entfernen → `block(Vor(erste Zeit des mittleren Tags))` leer mit `aelter: false`; `block(Nach(Zeit im entfernten Tag))` beginnt beim mittleren Tag.
