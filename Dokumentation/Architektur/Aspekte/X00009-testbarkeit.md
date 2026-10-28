---
id: X00009
name: Testbarkeit
art: Aspekt
geltung: Makro
befund: Einheitlich
hausantwort:
betrifft: [A00006, A00008, A00009, A00010, A00011, A00004, A00003, A00005]
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

- **Ergänzung 2026-10-27 (Karte #2, Nachzug aus #1):** Ist-Stand und zwei Regeln aus dem Bau, Einzelheiten im Nachtrag zu [ADR-0009](../../ADRs/0009-teststrategie-gate-und-health-check.md). (1) **Zwei Server-Stacks:** Programm über die eigene Annahmeschleife ([ADR-0010](../../ADRs/0010-eigene-annahmeschleife-hyper-http1.md)), In-Process-`testserver` über `axum::serve` — In-Process nur für Routing und Serialisierung; Herkunft, Ströme, Zeit- und Verbindungsgrenzen über das echte Programm, bis der Testserver über `ausliefern` läuft. (2) **Baustein-Tests im Modul:** die Bibliothek zeigt nur `start::ausfuehren`; Tests einzelner Bausteine gegen `tempfile` stehen als `#[cfg(test)]` im Modul (wie `src/http/gesundheit.rs`), `tests/` fährt immer das Binärprogramm. Abweichung von der Systemantwort oben: die Logablage-Integration „gegen `tempfile` in `tests/*.rs`“ (ADR-0009) liegt damit im Modul — kein Befund, weil die Hilfe dieselbe ist.

- **Ergänzung 2026-10-28 (Karte #4, I0005):** Ströme und Mehr-Tab-Verhalten sind nur über das echte Programm und einen echten Browser beobachtbar: die 6-Verbindungen-Grenze je Browser und das Ende des `SharedWorker` mit dem letzten Tab zeigt keine Attrappe ([ADR-0015](../../ADRs/0015-ein-ereignisstrom-je-browser-sharedworker-mit-sitzungs-header.md)). Darum: SSE-Zerlegung, Wächter und Zusammenführung rein in Node; Halter, Stand und Wechsel über das echte Programm (`reqwest`-Strom); Mehr-Tab und „Tab zu → offline < 2 s“ per Playwright. Der In-Process-`testserver` läuft seit I0004 über `ausliefern` und taugt damit auch für Ströme.
- **Ergänzung 2026-10-28 (Karte #5, I0006) — Gate gegen Messwert:** Was ohne Rechte nicht herstellbar ist, wird zerlegt statt geraten: der stille Abriss braucht Paketverlust (Netz-Namespace, Firewall), den der Testrechner unprivilegiert nicht erlaubt. Das **Gate** prüft deshalb die Glieder einzeln und deterministisch — Ableitung der Socket-Werte (rein), Werte am echten Socket (`getsockopt`), Aufruf je Verbindung (Naht „Pflege“ mit aufzeichnender Fassung in `ausliefern`), Socket-Fehler → Halter fällt (RST über `SO_LINGER 0` wie ein Abbruch durch den Kernel), Status und Fristen über die gestellte Uhr; die **Gesamtzeit** ist ein Messwert (Entwicklertest mit zweitem Gerät oder Probe mit Rechten), der in die ADR geht und kein Test rot färbt ([ADR-0017](../../ADRs/0017-stiller-abriss-kernel-erkennt-tote-verbindungen.md)). Restzeiten im Browser sind reine Funktionen mit gestellter Uhr und gestelltem Versatz ([ADR-0019](../../ADRs/0019-restzeit-weg-um-in-serverzeit-und-statusformen.md)).
