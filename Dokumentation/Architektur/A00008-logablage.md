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
- [ADR-0022: Channel-Einstellungen als `einstellungen.toml` im Ordner des Channels](../ADRs/0022-channel-einstellungen-je-channel-als-toml.md) — Akzeptiert, 2026-10-28
- [ADR-0030: Entfallen beim Start, beim Tageswechsel im Sekundentakt und beim Verkürzen — Löschen der Tagesdatei unter der Gesprächssperre, dann Index](../ADRs/0030-entfallen-beim-start-beim-tageswechsel-und-beim-verkuerzen-loeschen-unter-der-gespraechssperre.md) — Akzeptiert, 2026-10-28
- [ADR-0032: Löschen nur nach Erlaubnisliste — reguläre Tagesdateien gültiger Channels, keine Symlinks; defekte Einstellungen heißen „nichts entfällt“](../ADRs/0032-loeschen-nur-tagesdateien-nach-erlaubnisliste-keine-symlinks-defekt-heisst-nichts.md) — Akzeptiert, 2026-10-28

## Offene Fragen

## Notizen / Quellen

- Erster Test (Integration gegen `tempfile`): zwei Sätze an `channels/allgemein/2026-10-27.log` anhängen, `lesen` liefert beide in Reihenfolge; `tag_entfernen` löscht Log und `anhaenge/2026-10-27/`, lässt andere Tage stehen.
- Kein `fsync` je Nachricht: ein Stromausfall kann die letzten Sekunden kosten; ein Absturz des Programms nicht (Daten liegen im Seitencache des Systems).
- **Ergänzung 2026-10-27 (Karte #2, I0002):** `anhaengen` merkt die Länge vor dem Schreiben und schneidet bei Fehler darauf zurück; beim Öffnen einer Tagesdatei prüft es das letzte Byte und beginnt mit `\n`, wenn die Datei nicht auf `\n` endet ([ADR-0011](../ADRs/0011-anhaengen-mit-rueckbau-und-zeilenende-waechter.md)). Das ist das einzige Stück Format, das die Logablage kennt. Die Speicher-Attrappe des Ports kann „Fehler nach n Bytes“ simulieren, damit Gespräche den Fehlerpfad ohne Platte testet.
- Tagesdatei gewählt nach dem lokalen Datum der **Nachrichtenzeit**, nicht nach „jetzt“ — rotiert wird beim ersten Satz eines neuen Tages, ein eigener Mitternachts-Takt ist für I0002 nicht nötig (Entfallen alter Tage gehört zu I0018).
- Ein Gespräch ist ein Ordner unter `channels/` bzw. `direkt/`; fehlt `einstellungen.toml`, gelten Standardwerte (Anzeigename = Slug, Frist 7 Tage). Laden legt keine Einstellungen an — geschrieben werden sie erst mit I0007/I0018.
- **Ergänzung 2026-10-28 (Karte #6, I0007):** Der Port wächst um `channel_ordner_anlegen(name)` (`create_dir`, scheitert mit „vorhanden“, wenn der Ordner existiert — kein Überschreiben, [ADR-0021](../ADRs/0021-channel-anlegen-jeder-mit-sitzung-grenzen-je-herkunft-und-gesamt.md)), `einstellungen_lesen(gespraech) -> Option<Vec<u8>>`, `einstellungen_schreiben(gespraech, bytes)` (`einstellungen.toml.neu` + `rename`) und `ordner_entfernen(gespraech)` (nur für den Rückbau eines eben angelegten, sonst leeren Ordners). Das TOML kennt sie nicht ([Einstellungsformat](A00013-einstellungsformat.md)). `gespraeche()` liefert weiter alle Unterordner; welche davon Channels sind, entscheidet [Gespräche](A00006-gespraeche.md) nach [ADR-0020](../ADRs/0020-channelname-ist-ordnername-erlaubnisliste.md). Die Speicher-Attrappe bildet `create_dir` (vorhanden → Fehler) und „Einstellungen schreiben scheitert“ nach. Erster Test (Integration gegen `tempfile`): `channel_ordner_anlegen` zweimal → zweites Mal „vorhanden“; `einstellungen_schreiben` hinterlässt genau `einstellungen.toml`, keine `.neu`-Datei, und ersetzt eine vorhandene.
- **Ergänzung 2026-10-28 (Karte #12, I0018):** `tag_entfernen(gespraech, tag)` bildet den Pfad aus `Gespraech` und Datum, prüft mit `symlink_metadata`, dass dort eine reguläre Datei liegt, und ruft `remove_file`; fehlt die Datei, ist das Erfolg; liegt dort etwas anderes, ein Fehler ohne Löschen ([ADR-0032](../ADRs/0032-loeschen-nur-tagesdateien-nach-erlaubnisliste-keine-symlinks-defekt-heisst-nichts.md)). Kandidaten liefert das unveränderte `tage` (nur reguläre Dateien `JJJJ-MM-TT.log`, gebaut und geprobt: Symlinks und Ordner mit Tagesnamen sind nicht dabei). Ab I0010 danach `anhaenge/<tag>/` mit `remove_dir_all` (folgt keinen Symlinks, geprobt). Die Speicherablage bildet „Löschen scheitert“ nach. Erster Test (Integration gegen `tempfile`): ADR-0032, „Folgeentscheidungen“.
