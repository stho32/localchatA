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

Programmweit nur Kommandozeile (`<ordner>`, `--port`, `--host`, `--kein-browser`, später z. B. Anhanggrenze); je Gespräch `einstellungen.toml` im Ordner (Name, Frist). Umgebungsvariablen nur als Testknöpfe mit Präfix `LOCALCHATA_` (ADR-0009), je Knopf ein Test über den echten Weg. Geheimnisse gibt es keine (Sitzungs-Tokens leben nur im Speicher). Durchsetzung: `std::env::var` nur in einer Funktion je Knopf, Review-Punkt.

## Abweichungen

## Offene Fragen

## Notizen / Quellen

Entworfen am 2026-10-27.
