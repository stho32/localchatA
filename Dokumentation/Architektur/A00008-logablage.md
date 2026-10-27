---
id: A00008
name: Logablage
code: logablage
ebene: Komponente
eltern: A00006
repo: localchatA
status: Geplant
herkunft: dialog
quellen:
  - src/gespraeche/logablage.rs
verwandt: [A00007]
domaene: []
blackbox: Provider zum Dateisystem ohne Fachlogik; Layout vollständig in ADR-0003
---

# Logablage

## Verantwortung

Bildet Gespräche, Tage, Anhänge und Einstellungen auf Dateien im Ordner ab.

## Schnittstellen

Port (Trait) mit zwei Implementierungen — Dateisystem (Produktion) und Speicher (Tests der Eltern):

- `gespraeche() -> Vec<GespraechsOrt>`, `tage(gespraech) -> Vec<Datum>`, `lesen(gespraech, tag) -> String`.
- `anhaengen(gespraech, tag, satz: &str)` — ein `write_all` auf eine im Anhängemodus offene Datei; Tageswechsel öffnet die nächste.
- `anhang_ablegen(gespraech, tag, name, bytes) -> Pfad` — erst temporär, dann `rename`; erst danach schreibt Gespräche den Satz, der darauf zeigt.
- `einstellungen_lesen/schreiben(gespraech)` (`einstellungen.toml`, schreiben über temporäre Datei + `rename`).
- `tag_entfernen(gespraech, tag)` — Logdatei und Anhang-Ordner des Tages.

## Abhaengigkeiten

Dateisystem. Kennt das [Logformat](A00007-logformat.md) nicht — liest und schreibt Sätze als fertigen Text.

## Entscheidungen

- [ADR-0003: Ablage-Layout im Ordner](../ADRs/0003-ablage-layout-im-ordner.md) — Akzeptiert, 2026-10-27
- [ADR-0011: Anhängen mit Rückbau bei Fehler und Zeilenende-Wächter](../ADRs/0011-anhaengen-mit-rueckbau-und-zeilenende-waechter.md) — Akzeptiert, 2026-10-27

## Offene Fragen

## Notizen / Quellen

- Erster Test (Integration gegen `tempfile`): zwei Sätze an `channels/allgemein/2026-10-27.log` anhängen, `lesen` liefert beide in Reihenfolge; `tag_entfernen` löscht Log und `anhaenge/2026-10-27/`, lässt andere Tage stehen.
- Kein `fsync` je Nachricht: ein Stromausfall kann die letzten Sekunden kosten; ein Absturz des Programms nicht (Daten liegen im Seitencache des Systems).
- **Ergänzung 2026-10-27 (Karte #2, I0002):** `anhaengen` merkt die Länge vor dem Schreiben und schneidet bei Fehler darauf zurück; beim Öffnen einer Tagesdatei prüft es das letzte Byte und beginnt mit `\n`, wenn die Datei nicht auf `\n` endet ([ADR-0011](../ADRs/0011-anhaengen-mit-rueckbau-und-zeilenende-waechter.md)). Das ist das einzige Stück Format, das die Logablage kennt. Die Speicher-Attrappe des Ports kann „Fehler nach n Bytes“ simulieren, damit Gespräche den Fehlerpfad ohne Platte testet.
- Tagesdatei gewählt nach dem lokalen Datum der **Nachrichtenzeit**, nicht nach „jetzt“ — rotiert wird beim ersten Satz eines neuen Tages, ein eigener Mitternachts-Takt ist für I0002 nicht nötig (Entfallen alter Tage gehört zu I0018).
- Ein Gespräch ist ein Ordner unter `channels/` bzw. `direkt/`; fehlt `einstellungen.toml`, gelten Standardwerte (Anzeigename = Slug, Frist 7 Tage). Laden legt keine Einstellungen an — geschrieben werden sie erst mit I0007/I0018.
