---
id: X00008
name: Transaktionen und Nebenläufigkeit
art: Aspekt
geltung: Makro
befund: Einheitlich
hausantwort:
betrifft: [A00006, A00011, A00010, A00003]
herkunft: dialog
quellen:
  - src/gespraeche/
  - src/start/ausliefern.rs
  - src/start/verbindungen.rs
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

- **Ergänzung 2026-10-27 (Karte #2, Nachzug aus #1) — Verbindungen:** gebaut ist bisher nur die Nebenläufigkeit der Auslieferung ([ADR-0010](../../ADRs/0010-eigene-annahmeschleife-hyper-http1.md)): eine Annahmeschleife (`tokio::select!` über Signal, `accept`, `JoinSet::join_next`), je Verbindung ein Task im `JoinSet`; die Belegung je Gegenstelle ist ein `std::sync::Mutex` mit kurzer Haltezeit, nie über ein `await` gehalten, freigegeben im `Drop` des Platzes (vergiftete Sperre wird weiterbenutzt, `PoisonError::into_inner`); das Ende wird über einen `watch`-Kanal an alle Verbindungen gemeldet, danach `JoinSet` mit Frist 5 s, sonst `shutdown`. Lange Antworten (SSE, Long-Poll) belegen Plätze und enden beim Signal erst durch Abbruch, solange sie das Ende-Signal nicht selbst hören ([A00011](../A00011-live-kanal.md)). Durchsetzung wie oben plus `tests/verfuegbarkeit.rs` (Grenzen über das echte Programm) und Unit-Tests in `src/start/verbindungen.rs`.
- **Ergänzung 2026-10-28 (Karte #4, I0005) — Anwesenheit:** Statuswechsel entstehen unter der einen Sperre der Anwesenheit und bekommen dort eine laufende Nummer `stand`; veröffentlicht wird nach dem Freigeben durch die Integration. Weil zwei Veröffentlichungen sich überholen können, verwirft der Empfänger jeden Wechsel mit kleinerem `stand` als dem bekannten; ein Strom abonniert erst den Bus und holt dann den Stand, damit keine Lücke entsteht ([ADR-0016](../../ADRs/0016-anwesenheitsstatus-aus-haltern-mit-nachlauf-liste-als-stand-plus-wechsel.md)). Der `Drop` eines Halters nimmt die Sperre kurz (kein `await`, keine Ein-/Ausgabe). Ein Takt-Task ruft alle 250 ms `pflegen`. Durchsetzung: Unit-Test „vertauschte Veröffentlichung“ in der Zusammenführung der Oberfläche, Unit-Test „`stand` streng steigend“ in der Anwesenheit.
