---
id: A00011
name: Live-Kanal
code: live
ebene: Komponente
eltern: A00002
repo: localchatA
status: Geplant
herkunft: dialog
quellen:
  - src/live/
verwandt: [A00006, A00010]
domaene: []
blackbox: Ereignisbus mit Filter; Mechanik in ADR-0007, Vorbild kanbanD ereignisstrom
---

# Live-Kanal

## Verantwortung

Verteilt jedes Ereignis an genau die offenen Sichten und wartenden Agenten, für die es sichtbar ist.

## Schnittstellen

- `veroeffentlichen(Ereignis)` — `tokio::sync::broadcast`; Ereignisarten `Nachricht{gespraech, zeit, von, erwaehnt}`, `Channel`, `Frist`, `Anwesenheit(Wechsel)` (ab I0005, mit `stand`, [ADR-0016](../ADRs/0016-anwesenheitsstatus-aus-haltern-mit-nachlauf-liste-als-stand-plus-wechsel.md)).
- `abonnieren(fuer: Kennung, filter) -> Strom` — Filter: sichtbar für (Direktgespräche nur für Beteiligte), optional nur `erwaehnung`, `direkt`, `channel=<…>`. Ab I0005: erst abonnieren, dann den `Stand` der Teilnehmerliste als erstes Ereignis, danach die Wechsel; der Strom endet mit dem Ende-Signal des Programms.
- `warten(fuer, filter, seit: Zeit, bis: Dauer) -> Vec<Nachricht>` — erst nachholen (Index, alles nach `seit`), dann auf den Bus warten; antwortet beim ersten Treffer oder nach `bis` leer.
- Wer zu langsam liest (`Lagged`), bekommt ein Ereignis „neu laden“ statt stiller Lücke (wie kanbanD); für die Anwesenheit ist das ein neuer `Stand`.

## Abhaengigkeiten

[Gespräche](A00006-gespraeche.md) (Nachholen über den Index); Uhr-Provider für Herzschlag und Zeitüberschreitung.

## Entscheidungen

- [ADR-0007: Live-Kanal — SSE für Browser, Long-Poll für Agenten, ein Ereignisbus](../ADRs/0007-live-kanal-sse-fuer-browser-long-poll-fuer-agenten.md) — Vorgeschlagen, 2026-10-27
- [ADR-0010: Eigene Annahmeschleife mit hyper http1 statt `axum::serve`](../ADRs/0010-eigene-annahmeschleife-hyper-http1.md) — Akzeptiert, 2026-10-27
- [ADR-0015: Ein Ereignisstrom je Browser — SharedWorker mit `fetch`-Strom und Sitzungs-Header](../ADRs/0015-ein-ereignisstrom-je-browser-sharedworker-mit-sitzungs-header.md) — Akzeptiert, 2026-10-28
- [ADR-0016: Anwesenheitsstatus aus Haltern mit Nachlauf, Teilnehmerliste als Stand plus Wechsel](../ADRs/0016-anwesenheitsstatus-aus-haltern-mit-nachlauf-liste-als-stand-plus-wechsel.md) — Akzeptiert, 2026-10-28

## Offene Fragen

## Notizen / Quellen

- `seit` ist eine Nachrichtenzeit; weil Zeiten global streng steigen ([ADR-0005](../ADRs/0005-programm-als-einziger-schreiber-und-reihenfolge.md)), verliert ein Agent zwischen zwei Warte-Aufrufen nichts — auch nicht über einen Neustart des Programms.
- Erster Test (Unit, tokio): Abonnent `shoff` mit Filter `erwaehnung`; Ereignis ohne `@shoff` → nichts, mit `@shoff` → genau eins; Direktnachricht `claude→mia` erreicht `shoff` nie.
- **Ergänzung 2026-10-27 (Karte #2, Nachzug aus #1):** Die Auslieferung ([ADR-0010](../ADRs/0010-eigene-annahmeschleife-hyper-http1.md)) bestimmt drei Randbedingungen: (1) laufende Antworten werden von der Kopf-Zeitgrenze (10 s) nicht abgeschnitten; (2) jeder Strom und jeder wartende Aufruf belegt einen der 64 Plätze seiner Gegenstelle — Tabs und Agenten eines Rechners teilen sie sich; (3) beim Ende werden offene Antworten nach 5 s abgebrochen. Deshalb hören `abonnieren` und `warten` ein Ende-Signal (wie `ende` in `src/start/ausliefern.rs`) und enden selbst: der Strom schließt, `warten` antwortet leer. Erster Test dazu (Integration): offener SSE-Strom, SIGTERM → Strom endet und Prozess endet mit Exit 0 in deutlich unter 5 s.
- **Ergänzung 2026-10-28 (Karte #4, I0005 — erster Strom):** Der Live-Kanal entsteht mit I0005, nicht erst mit I0009: Bus, `/api/ereignisse` und das Ende-Signal kommen jetzt, Nachrichten-Ereignisse und `warten` später. Der Strom ist ein Halter der [Anwesenheit](A00010-anwesenheit.md) (`verbunden` beim Start des Stroms, `Drop` beim Ende — geprobt über die eigene Annahmeschleife: 0,3 ms nach dem Schließen, auch ohne laufende Daten; die Kopf-Zeitgrenze schneidet ihn nicht ab). Im Browser hält ihn ein `SharedWorker` je Browser ([ADR-0015](../ADRs/0015-ein-ereignisstrom-je-browser-sharedworker-mit-sitzungs-header.md)), damit belegt ein Mensch mit vielen Tabs einen Platz statt einen je Tab. Das Ende-Signal (bisher nur intern in `ausliefern`) braucht der Strom als Empfänger aus dem Start (`watch::Receiver`, vgl. kanbanD `ende`), die Folgeaufgabe aus ADR-0010 wird mit I0005 fällig.
  - **Erster Test (Integration, echtes Programm):** siehe ADR-0016 — zwei Sitzungen, Stand als erstes Ereignis, Wechsel online/offline in < 2 s, SIGTERM bei offenem Strom → Exit 0 in < 1 s; dazu ADR-0015 — ohne Header 401, Token im Query ohne Header 401.
