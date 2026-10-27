---
id: X00009
name: Testbarkeit
art: Aspekt
geltung: Makro
befund: Einheitlich
hausantwort:
betrifft: [A00006, A00008, A00009, A00010, A00011, A00004]
herkunft: dialog
quellen:
  - tests/
  - e2e/
---

# Testbarkeit

## Hausantwort

Skill `test-pyramide` (Unit, Integration, E2E; Test zuerst je Bubble).

## Systemantwort

Drei Ebenen nach [ADR-0009](../../ADRs/0009-teststrategie-gate-und-health-check.md). Fachlogik (Logformat, Verlaufsindex, Anwesenheit, Live-Filter) ist rein bzw. bekommt Uhr und Logablage als Provider mit Zweitimplementierung; Integration und E2E fahren das echte Binärprogramm auf Port 0 mit Wegwerf-Ordner. Je Baustein steht der erste Test unter „Notizen / Quellen“. Durchsetzung: Gate (`cargo test`, `npm --prefix e2e test`), Review-Punkt „neue Logik ohne Unit-Test“.

## Abweichungen

## Offene Fragen

## Notizen / Quellen

Entworfen am 2026-10-27.
