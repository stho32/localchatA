---
id: A00013
name: Einstellungsformat
code: einstellungen
ebene: Komponente
eltern: A00006
repo: localchatA
status: Geplant
herkunft: dialog
quellen:
  - src/gespraeche/
verwandt: [A00008, A00012]
domaene: []
blackbox: reine Funktionen über der Crate `toml`; Schlüssel und Regeln vollständig in ADR-0022
---

# Einstellungsformat

## Verantwortung

Übersetzt die Einstellungen eines Channels zwischen `einstellungen.toml`-Text und geprüften Werten.

## Schnittstellen

- `lesen(bytes) -> Result<Einstellungen, Defekt { zeile, grund }>` — prüft Typen, Kennung (`angelegt_von`, ADR-0013) und Beschreibung (ADR-0022 Regel 5); unbekannte Schlüssel sind kein Defekt.
- `schreiben(&Einstellungen, vorher: Option<bytes>) -> String` — bekannte Schlüssel gesetzt, unbekannte aus `vorher` erhalten.
- `beschreibung_pruefen(text) -> Result<Option<Beschreibung>, Beschreibungsfehler>` — auch für die HTTP-Schnittstelle über [Gespräche](A00006-gespraeche.md).

## Abhaengigkeiten

Crate `toml` 1.x mit `serde` ([ADR-0022](../ADRs/0022-channel-einstellungen-je-channel-als-toml.md)); Typ `Kennung` (`src/gespraeche/kennung.rs`).

## Entscheidungen

- [ADR-0022: Channel-Einstellungen als `einstellungen.toml` im Ordner des Channels](../ADRs/0022-channel-einstellungen-je-channel-als-toml.md) — Akzeptiert, 2026-10-28
- [ADR-0029: Frist in ganzen Kalendertagen nach lokalem Datum, heute zählt mit, 1 bis 3650, Standard 7](../ADRs/0029-frist-in-kalendertagen-heute-zaehlt-mit-1-bis-3650.md) — Akzeptiert, 2026-10-28
- [ADR-0032: Löschen nur nach Erlaubnisliste — reguläre Tagesdateien gültiger Channels, keine Symlinks; defekte Einstellungen heißen „nichts entfällt“](../ADRs/0032-loeschen-nur-tagesdateien-nach-erlaubnisliste-keine-symlinks-defekt-heisst-nichts.md) — Akzeptiert, 2026-10-28

## Offene Fragen

## Notizen / Quellen

- Gegenstück zum [Logformat](A00007-logformat.md): wie dieses kennt es keine Datei, nur Text; die [Logablage](A00008-logablage.md) kennt nur Bytes.
- Erster Test: siehe ADR-0022, „Folgeentscheidungen“ — die Fälle der Dependency-Probe werden Unit-Tests dieses Bausteins (Skill `dependency-probe`, Probe behalten).
- **Ergänzung 2026-10-28 (Karte #12, I0018):** `Einstellungen` wächst um `frist_tage: Option<u32>`; `lesen` prüft Typ und Spanne 1–3650 (sonst `Defekt` mit Zeile — nach ADR-0022 Regel 6 und [ADR-0032](../ADRs/0032-loeschen-nur-tagesdateien-nach-erlaubnisliste-keine-symlinks-defekt-heisst-nichts.md) Regel 5 sperrt das das Entfallen); fehlt der Schlüssel, `None` (= 7, kein Defekt); `schreiben` setzt `frist_tage`, wenn `Some`, und erhält unbekannte Schlüssel. Die Spanne steht als Konstante neben der Prüfung, die auch die HTTP-Schnittstelle über Gespräche nutzt ([ADR-0029](../ADRs/0029-frist-in-kalendertagen-heute-zaehlt-mit-1-bis-3650.md)). Erster Test: `frist_tage = 1` und `= 3650` gelesen; `0`, `3651`, `-3`, `"7"`, `7.5` → je `Defekt` mit Zeile; Rundweg mit `frist_tage = 30` und unbekanntem Schlüssel `farbe`.
