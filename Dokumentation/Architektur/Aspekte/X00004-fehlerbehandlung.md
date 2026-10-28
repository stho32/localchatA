---
id: X00004
name: Fehlerbehandlung
art: Aspekt
geltung: Makro
befund: Ungeklaert
hausantwort:
betrifft: [A00004, A00006, A00007, A00005, A00014]
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

- **Ergänzung 2026-10-27 (Karte #2, I0002):** Schreibfehler beim Anhängen: zusätzlich zu „Index und Live unberührt“ wird die Datei auf die Länge vor dem Schreiben zurückgebaut; scheitert der Rückbau, steht eine Zeile auf stderr, und der nächste Satz beginnt nach einem `\n` ([ADR-0011](../../ADRs/0011-anhaengen-mit-rueckbau-und-zeilenende-waechter.md)).
- **Ergänzung 2026-10-28 (Karte #8, I0009):** (1) 429 mit `Retry-After` und `{fehler, grenze, wieder_in_ms}` ist die Antwort auf „gerade zu viel“ — nicht verarbeitet, später erneut erlaubt ([ADR-0036](../../ADRs/0036-schreibkonto-token-eimer-je-kennung-und-je-herkunft.md)). (2) Wiederholung mit demselben `Idempotency-Key` ist gefahrlos: 200 mit der ursprünglichen Nachricht ([ADR-0034](../../ADRs/0034-nachricht-senden-post-nachrichten-text-hoechstens-16-kib-steuerzeichen-abweisen-wiederholschluessel.md)). (3) Im Browser steht ein Fehlschlag an der Nachricht (⚠, Grund, „Erneut senden“, „Verwerfen“), nie als Dialog; bei getrenntem Strom wird gepuffert statt gemeldet ([ADR-0037](../../ADRs/0037-zustellung-im-tab-vorlaeufig-bestaetigt-zeichnen-je-frame.md)).
