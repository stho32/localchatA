---
id: X00002
name: Benutzerrechte
art: Aspekt
geltung: Makro
befund: Einheitlich
hausantwort:
betrifft: [A00004, A00011]
herkunft: dialog
quellen:
  - src/http/
  - src/live/
---

# Benutzerrechte

## Hausantwort

Keine Vorlage zugeordnet.

## Systemantwort

Es gibt keine Rechte (Vision: „keine Benutzerverwaltung und keine Rechte“). Einzige Sichtbarkeitsregel: ein Direktgespräch sehen nur seine beiden Beteiligten — in Abruf, Suche, SSE und Warten. Geprüft wird an der Systemgrenze ([A00004](../A00004-http-schnittstelle.md)) bzw. im Filter des [Live-Kanals](../A00011-live-kanal.md), nie in Index oder Ablage. Durchsetzung: Integrationstest je Weg (Abruf, Suche, SSE, Warten), dass ein Dritter eine Direktnachricht nicht erhält.

## Abweichungen

## Offene Fragen

- Designsystem-Frage 10 (wer sieht welche Channels) — fachlich, `planer`.

## Notizen / Quellen

Sichtbarkeit ist kein Schutz gegen Absicht: Dateien sind lesbar, Tokens sind nicht an Personen gebunden (ADR-0008).
