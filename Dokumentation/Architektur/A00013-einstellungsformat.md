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

## Offene Fragen

## Notizen / Quellen

- Gegenstück zum [Logformat](A00007-logformat.md): wie dieses kennt es keine Datei, nur Text; die [Logablage](A00008-logablage.md) kennt nur Bytes.
- Erster Test: siehe ADR-0022, „Folgeentscheidungen“ — die Fälle der Dependency-Probe werden Unit-Tests dieses Bausteins (Skill `dependency-probe`, Probe behalten).
