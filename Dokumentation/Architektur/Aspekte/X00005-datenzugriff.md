---
id: X00005
name: Datenzugriff
art: Aspekt
geltung: Makro
befund: Einheitlich
hausantwort:
betrifft: [A00006, A00008, A00009]
herkunft: dialog
quellen:
  - src/gespraeche/
---

# Datenzugriff

## Hausantwort

Keine Vorlage zugeordnet.

## Systemantwort

Einziger Weg zu den Daten ist [Gespräche](../A00006-gespraeche.md): schreibend über die Logablage (Port mit Dateisystem- und Speicher-Implementierung), lesend über den Verlaufsindex im Speicher. Kein anderer Baustein öffnet Dateien im Ordner. Durchsetzung: Review-Punkt und später Architekturtest — `std::fs` kommt nur in `src/gespraeche/logablage.rs` und `src/start/` (Ordner anlegen, Sperre) vor.

## Abweichungen

## Offene Fragen

## Notizen / Quellen

ADR-0003, ADR-0005, ADR-0006.
