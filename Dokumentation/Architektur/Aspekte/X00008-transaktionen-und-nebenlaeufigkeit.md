---
id: X00008
name: Transaktionen und Nebenläufigkeit
art: Aspekt
geltung: Makro
befund: Einheitlich
hausantwort:
betrifft: [A00006, A00011, A00010, A00003, A00014]
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
- **Ergänzung 2026-10-28 (Karte #5, Nachzug aus #4) — Anwesenheit veröffentlicht unter der Sperre:** Die Systemantwort oben („Anwesenheit: … Ereignisse nach dem Freigeben“) und die Ergänzung vom 2026-10-28 (I0005) sind überholt: seit Nacharbeit 1 und 3 von #4 (Commits b3645cf, f6e446f) ruft die Anwesenheit eine eingesetzte, synchrone Veröffentlichungsfunktion (`broadcast::send`, kein `await`, keine Ein-/Ausgabe) **unter** ihrer Sperre; „jetzt“ wird unter der Sperre gelesen und ist monoton. Damit ist die Reihenfolge auf dem Bus die von `stand`; der Empfänger-Schutz über `stand` bleibt zweite Linie. Abonnieren und Stand geschehen unter einer Sperre (`stand_mit`). Gilt ab I0006 auch für `verlassen` und verschobenes `weg_um` ([ADR-0018](../../ADRs/0018-verlassen-sofort-weg-nie-beim-schliessen.md), [ADR-0019](../../ADRs/0019-restzeit-weg-um-in-serverzeit-und-statusformen.md)). Durchsetzung: Unit-Test mit aufzeichnender Veröffentlichung, die beim Aufruf prüft, dass die Sperre gehalten wird (`try_lock` scheitert); `clippy::await_holding_lock` im Gate.
- **Ergänzung 2026-10-28 (Karte #8, I0009) — Schreibsperre über alle Gespräche:** `senden` hält Gesprächssperre und die globale Sperre `schreiben` von der Zeitvergabe bis nach der Veröffentlichung; Sperrordnung Verzeichnis → Gespräch → `schreiben`, nichts nimmt umgekehrt ([ADR-0035](../../ADRs/0035-nachrichtenzeit-eine-schreibsperre-ueber-alle-gespraeche.md)). Schreiben ist damit über alle Channels seriell (Mikrosekunden je Nachricht); Leser (Block) nehmen nur die Gesprächssperre. Der Health-Check liest die letzte Zeit aus einem Atomwert, nie über `schreiben`. Durchsetzung: Unit-Test mit Schranke in der Speicherablage (zwei Channels, Busfolge = Zeitfolge); Review-Punkt „keine Gesprächssperre unter `schreiben` nehmen“. Das Schreibkonto ([A00014](../A00014-schreibkonto.md)) hat eine eigene Sperre, nie zusammen mit `schreiben` gehalten (gebucht wird vor `senden`).
