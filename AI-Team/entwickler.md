# entwickler — Projektschicht

Ergänzt die Rolle `entwickler` der Factory für dieses Projekt. Gilt für Projektgebundenes (Umgebung, Stack, Pfade, Befehle, Stil); Grenzen und Kernregeln der Rolle überschreibt sie nie (`gemeinsam/regeln.md`, „Vorrang“).

## Projektregeln

- Stack laut Vision: Rust, ein Programm für Weboberfläche und WebAPI; noch kein Code — Technik entscheidet der `architekt` (ADRs unter `Dokumentation/ADRs/`).
- Datenhaltung: täglich rotierende Logdateien je Channel, Frist je Channel (Standard 7 Tage); kein Git, keine Datenbank.
- Leistungsziel: 100.000 Nachrichten — Channel öffnet sofort, Verlauf scrollt flüssig, Suche ohne spürbare Wartezeit.
- Oberfläche deutsch; Designsystem `Dokumentation/Wireframes/designsystem/` (README, Tokens, Bausteine, offene Fragen).
- Gate und Testbefehle: siehe `AI-Team/umgebung.md` (ADR-0009); vor dem ersten `npm --prefix e2e test` einmal `npm --prefix e2e ci`. Prüfung und Commit getrennt fahren — eine Pipe verdeckt den Exit-Code.
- Außenwirkung per Testknopf `LOCALCHATA_*` prüfbar machen (z. B. `LOCALCHATA_BROWSER`); Standard-Ports in Tests selbst belegen.
- Preview nie auf Standard-Port (Skill `freier-port`); Port 7421 gehört dem kanbanD-Board.

## Spezialisierungen

Aus der Factory (`rollen/entwickler/spezialisierungen/`), gewählt bei der Bestückung; fremde mit `entwickler/<name>`.

- rust — Server, Gate, Tests
- javascript — Oberfläche im Browser

## Quellen

IDs aus `AI-Team/quellen.md`, die diese Rolle besonders braucht, mit Anlass.

- Q01 — Zielbild
- Q02 — Arbeitspakete
- Q03 — Gestaltung umsetzen
- Q04 — Vorbild für Aufbau und Live-Kanal

## Gelerntes
- 2026-10-28 (#3): Oberfläche vor dem Bericht selbst gegen das Designsystem prüfen — Fokus auf gefüllten Schaltern außen (Kontrast ≥ 3:1 gegen die Fläche daneben), kein Querscrollen bei 320 px, Fehler in kd-danger mit ⚠ und aria-invalid, Kennungen in Mono, Konsole über die LAN-Adresse ohne Fehler; mehrere Tabs desselben Browsers gleichzeitig laden.
- 2026-10-28 (#4, realist-consultant): Zeittests. (1) Die Grenze im Test ist die Grenze aus dem Kriterium, nie schärfer (`tests/leistung.rs` prüfte 1 s, R00002 verlangt 2 s); fehlt eine Zeit im Kriterium, wird die Grenze aus der Mechanik hergeleitet (Rückzugsplan, Takt) und im Test begründet. (2) Im E2E wird auf das Merkmal gewartet und ereignisgenau gemessen: `expect.poll(…, {intervals:[25]})` oder `page.waitForFunction(…, {polling:'raf'})`; `toHaveText` im Standardraster (0/100/350/850/1.850 ms, dann je 1 s) nie für eine Dauer; das `timeout` des Wartens liegt deutlich über der Grenze (10 s), die Grenze prüft eine eigene Zeile. (3) Die Uhr startet beim Ereignis des Kriteriums (nach `await close()`, ab der 201). (4) Untergrenzen über `seit` vom Server, Obergrenzen über die Ankunft. (5) Jeder neue Zeittest: 20 Wiederholungen unter `cargo build -j8`, min/Median/max gegen die Grenze im Bericht; liegt der Median näher als 10 % an der Grenze, Rückfrage an den Director statt Test oder Code zu verbiegen. (6) Konsolenfänger: der erlaubende Modus mit der Liste der vorgeschriebenen Antworten ist Standard; `streng` nur im Normalbetrieb ohne Neustart.
