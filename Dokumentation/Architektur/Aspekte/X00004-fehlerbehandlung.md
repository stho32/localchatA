---
id: X00004
name: Fehlerbehandlung
art: Aspekt
geltung: Makro
befund: Ungeklaert
hausantwort:
betrifft: [A00004, A00006, A00007]
herkunft: dialog
quellen:
  - src/fehler.rs
---

# Fehlerbehandlung

## Hausantwort

Keine Vorlage zugeordnet; Vorbild kanbanD `src/fehler.rs` (ein Fehlertyp, Abbildung auf HTTP-Status mit `{fehler: <text>}`).

## Systemantwort

Geplant: Fachbausteine liefern `Result<_, Fehler>`; ein `enum Fehler` (ungültige Eingabe 400, ohne Sitzung 401, unbekannt 404, Name vergeben 409, zu groß 413, Dateisystem 500) wird ausschließlich an der Grenze ([A00004](../A00004-http-schnittstelle.md)) in eine deutsche Meldung mit Grund und Ausweg übersetzt (Designsystem „Sprache und Ton“). Defekte Logzeilen werden beim Laden übersprungen und gemeldet, nie still ersetzt (ADR-0004). Schreibfehler beim Anhängen: die Nachricht gilt als nicht gesendet, Index und Live bleiben unberührt (Reihenfolge Log → Index → Live, ADR-0005). Durchsetzung: Review-Punkt — kein `unwrap`/`expect` außerhalb von Start und Tests (`clippy::unwrap_used` im Bibliotheksteil).

## Abweichungen

## Offene Fragen

## Notizen / Quellen

Entworfen am 2026-10-27.
