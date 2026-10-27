---
id: X00006
name: Konfiguration und Geheimnisse
art: Aspekt
geltung: Makro
befund: Einheitlich
hausantwort:
betrifft: [A00003, A00006]
herkunft: dialog
quellen:
  - src/start/
---

# Konfiguration und Geheimnisse

## Hausantwort

Keine Vorlage zugeordnet; Vorbild kanbanD: nur Kommandozeile, Einstellungen im Ordner.

## Systemantwort

Programmweit nur Kommandozeile (`<ordner>`, `--port`, `--host`, `--kein-browser`, ab I0004 `--erreichbar-als <name>` mehrfach für weitere Host-Namen nach [ADR-0014](../../ADRs/0014-grenzschutz-host-origin-sicherheitsheader.md), später z. B. Anhanggrenze); je Gespräch `einstellungen.toml` im Ordner (Name, Frist). Umgebungsvariablen nur als Testknöpfe mit Präfix `LOCALCHATA_` (ADR-0009), je Knopf ein Test über den echten Weg. Geheimnisse gibt es keine (Sitzungs-Tokens leben nur im Speicher, [ADR-0012](../../ADRs/0012-identitaet-und-sitzung.md); Testknopf `LOCALCHATA_GNADENFRIST_MS`). Durchsetzung: `std::env::var` nur in einer Funktion je Knopf, Review-Punkt.

## Abweichungen

## Offene Fragen

## Notizen / Quellen

Entworfen am 2026-10-27.
