---
id: X00001
name: Logging und Diagnose
art: Aspekt
geltung: Makro
befund: Ungeklaert
hausantwort:
betrifft: [A00003, A00006]
herkunft: dialog
quellen:
  - src/
---

# Logging und Diagnose

## Hausantwort

Keine App-Architektur-Vorlage zugeordnet (`AI-Team/architekt.md` nennt keine); Vorbild kanbanD schreibt Startzeile auf stdout und Fehler mit `eprintln!` auf stderr.

## Systemantwort

Geplant: genau eine Startzeile auf stdout (`localchatA läuft für <ordner> unter <adresse>`, maschinenlesbar für Tests und Director) plus eine Zeile mit Kennzahlen (Nachrichten, Loggröße, Ladezeit, defekte Zeilen); Fehler und Warnungen einzeilig auf stderr mit Ort (Datei:Zeile im Log bei defekten Sätzen). Keine Nachrichteninhalte in Diagnosezeilen. Durchsetzung: Review-Punkt — `println!` nur in `start`, sonst `eprintln!` über eine Funktion in `fehler`.

## Abweichungen

## Offene Fragen

- Reicht stderr, oder soll das Programm eine eigene Diagnosedatei schreiben? Empfehlung: stderr — der Ordner gehört den Gesprächen.

## Notizen / Quellen

Entworfen am 2026-10-27; `Ungeklaert`, bis Code existiert.
