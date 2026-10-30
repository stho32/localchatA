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
- [ADR-0039: Große Antworten — Byte-Budget je Block, schwere Arbeit nur auf der Werkbank, die Abrissgrenze ist die Sendefrist](../ADRs/0039-grosse-antworten-byte-budget-je-block-werkbank-und-abrissgrenze-als-sendefrist.md) — Akzeptiert, 2026-10-28
- [ADR-0048: Eigene Nachrichten bearbeiten und löschen — Änderungssatz im Tageslog der Änderung, Fassung je Nachricht im Index, zwei Routen, Thema `aenderung`, der Löschsatz ist der Commit-Punkt für die Anhänge](../ADRs/0048-eigene-nachrichten-bearbeiten-und-loeschen-aenderungssatz-im-tageslog-der-aenderung-fassung-je-nachricht-im-index.md)
- [ADR-0053: Antworten mit Zitat — Bezug als Metazeile `+ antwort_auf: <zeit> <kennung>`, aufgelöst beim Lesen, Feld `antwort_auf`, `zitat` im Thema `aenderung`, Sprung über den Verlauf, Wecken über die Kennung](../ADRs/0053-antworten-mit-zitat-bezug-als-metazeile-zeit-und-kennung-aufgeloest-beim-lesen-feld-antwort-auf-zitat-im-thema-aenderung.md) — Akzeptiert, 2026-10-30

## Offene Fragen

- Was genau ist ein Treffer — Teilwort ohne Groß-/Kleinschreibung (Empfehlung, so geprobt), ganze Wörter, auch Absender und Anhangnamen? Fachlich, für den `planer` bei I0011.

## Notizen / Quellen

- Je Tagesdatei: Rohtext (`String`), Satzanfänge (`Vec<u32>`), Suchkopie in Kleinschreibung je Satz mit eigener Offset-Tabelle (Kleinschreibung ändert Bytelängen, z. B. `İ`).
- Suche läuft Tag für Tag von neu nach alt über alle sichtbaren Gespräche, sortiert die Treffer eines Tages nach Zeit und hört bei 50 auf.
- Erster Test (Unit, ohne Platte): drei Tage zu je 250 Sätzen aufnehmen; `block(g, None, 100)` liefert die letzten 100 des jüngsten Tages, `block(g, Some(zeit_des_ersten), 100)` greift über die Tagesgrenze; `suchen("GRÖSSE")` findet `größe`.
- **Ergänzung 2026-10-27 (Karte #2, I0002):** `anhaengen` wird nur nach erfolgreichem Schreiben aufgerufen ([ADR-0011](../ADRs/0011-anhaengen-mit-rueckbau-und-zeilenende-waechter.md)); damit ist der Rohtext im Speicher byte-gleich mit der Datei, und `log_bytes` im Health-Check ist die Summe der Rohtexte. Für I0002 genügt `tag_aufnehmen`, `anhaengen` und die Zählung; `block` und `suchen` gehören zu I0008/I0011, sind aber der natürliche Prüfweg der Unit-Tests von Gespräche.
- **Ergänzung 2026-10-28 (Karte #7, I0008):** `block` nach [ADR-0025](../ADRs/0025-verlauf-ueber-die-api-bloecke-mit-zeit-als-cursor.md) Regeln 2–4: exklusive Grenzen in beide Richtungen, Weg über die `BTreeMap` der Tage, binäre Suche über die Kopfzeiten der Satzanfänge eines Tages (Zeiten steigen im Tag, einziger Schreiber), Ergebnis als Bytebereiche bzw. Kopien der Sätze — das Deuten (`logformat::text`, Kennung, Zeit) liegt bei [Gespräche](A00006-gespraeche.md) bzw. im Logformat, nicht im Index. `aelter`/`neuer` ergeben sich aus der Lage der Grenze, ohne zu zählen. Kosten je Block unabhängig von der Länge des Verlaufs (ADR-0006: 0,06 ms im Release). Erster Test: ADR-0025, „Folgeentscheidungen“ (drei Tage zu 250 Sätzen, Grenze am Tageswechsel, Cursor zwischen zwei Sätzen).
- **Ergänzung 2026-10-28 (Karte #12, I0018):** `tag_entfernen(tag) -> Option<(nachrichten, bytes)>` nimmt einen Tag aus der `BTreeMap` — nur nach erfolgreichem Löschen der Datei ([ADR-0030](../ADRs/0030-entfallen-beim-start-beim-tageswechsel-und-beim-verkuerzen-loeschen-unter-der-gespraechssperre.md) Regel 4); `saetze_vor(tag)` zählt die Sätze der Tage vor einer Grenze für die 409-Vorschau. `block` braucht nichts Neues: ein Cursor in einem entfallenen Tag läuft über die `BTreeMap` zum nächsten vorhandenen, `aelter` wird `false`, sobald der älteste Tag fehlt. Erster Test: Index mit drei Tagen, den ältesten entfernen → `block(Vor(erste Zeit des mittleren Tags))` leer mit `aelter: false`; `block(Nach(Zeit im entfernten Tag))` beginnt beim mittleren Tag.
- **Ergänzung 2026-10-29 (Karte #48, I0032/I0033), Soll:** Nachrichten und Änderungssätze je Tag getrennt geordnet (Spanne endet vor jedem Satz, Block zählt nur Nachrichten); Fassung je Nachricht `zeit → {geaendert, bearbeitet(Ort) | geloescht}`, beim Laden nach allen Nachrichten in Zeitfolge gebildet, `tag_entfernen` nimmt den Bereich heraus (ADR-0048 Regeln 2, 3).
- **Ergänzung 2026-10-30 (Karte #79, I0043), Soll:** keine neue Struktur. Ein Bezug wird beim Kopieren des Blocks über `nachricht(zeit)` samt Fassung aufgelöst; der Textanfang (≤ 120 Zeichen) wird aus dem Satz gelesen, ohne das Original zu kopieren; nicht gefunden oder andere Kennung → `entfallen`. Das Byte-Budget zählt `antwort_auf` mit. Regeln in ADR-0053 Regel 3, Lastnachweis dort.
