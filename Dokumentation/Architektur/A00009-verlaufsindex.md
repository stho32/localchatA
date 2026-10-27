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
- `block(gespraech, vor, n)`, `suchen(begriff, gespraeche, nach, n)`, `tag_entfernen(gespraech, tag)`.
- Kennzahlen für den Health-Check: Nachrichtenzahl, Bytes Rohtext, Bytes Suchkopie.

## Abhaengigkeiten

[Logformat](A00007-logformat.md) (Satzgrenzen, Text ohne Einrückung).

## Entscheidungen

- [ADR-0006: Verlaufsindex im Speicher über dem Rohtext der Tagesdateien](../ADRs/0006-verlaufsindex-im-speicher.md) — Akzeptiert, 2026-10-27

## Offene Fragen

- Was genau ist ein Treffer — Teilwort ohne Groß-/Kleinschreibung (Empfehlung, so geprobt), ganze Wörter, auch Absender und Anhangnamen? Fachlich, für den `planer` bei I0011.

## Notizen / Quellen

- Je Tagesdatei: Rohtext (`String`), Satzanfänge (`Vec<u32>`), Suchkopie in Kleinschreibung je Satz mit eigener Offset-Tabelle (Kleinschreibung ändert Bytelängen, z. B. `İ`).
- Suche läuft Tag für Tag von neu nach alt über alle sichtbaren Gespräche, sortiert die Treffer eines Tages nach Zeit und hört bei 50 auf.
- Erster Test (Unit, ohne Platte): drei Tage zu je 250 Sätzen aufnehmen; `block(g, None, 100)` liefert die letzten 100 des jüngsten Tages, `block(g, Some(zeit_des_ersten), 100)` greift über die Tagesgrenze; `suchen("GRÖSSE")` findet `größe`.
