---
id: X00008
name: Transaktionen und Nebenläufigkeit
art: Aspekt
geltung: Makro
befund: Einheitlich
hausantwort:
betrifft: [A00006, A00011, A00010]
herkunft: dialog
quellen:
  - src/gespraeche/
---

# Transaktionen und Nebenläufigkeit

## Hausantwort

keine — je System zu entscheiden (`_aspekte.md`).

## Systemantwort

Atomar ist das Senden einer Nachricht in ein Gespräch: unter einer Sperre je Gespräch Zeit vergeben → Satz anhängen → Index ergänzen → Ereignis veröffentlichen ([ADR-0005](../../ADRs/0005-programm-als-einziger-schreiber-und-reihenfolge.md)). Die globale Uhr der Nachrichtenzeiten ist ein `Mutex<Zeit>` mit kurzer Haltezeit. Dateizugriffe laufen über `spawn_blocking`; im async-Kontext wird keine Sperre über ein `await` gehalten. Anwesenheit: ein `Mutex` um das Register, Ereignisse nach dem Freigeben. Eine Instanz je Ordner über die Sperrdatei. Durchsetzung: Integrationstest „20 × 50 gleichzeitige Nachrichten, gleiche Folge in Log, Index, SSE“; `clippy::await_holding_lock` im Gate (durch `-D warnings` aktiv).

## Abweichungen

## Offene Fragen

## Notizen / Quellen

Entworfen am 2026-10-27.
