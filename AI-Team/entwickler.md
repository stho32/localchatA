# entwickler — Projektschicht

Ergänzt die Rolle `entwickler` der Factory für dieses Projekt. Gilt für Projektgebundenes (Umgebung, Stack, Pfade, Befehle, Stil); Grenzen und Kernregeln der Rolle überschreibt sie nie (`gemeinsam/regeln.md`, „Vorrang“).

## Projektregeln

- Stack laut Vision: Rust, ein Programm für Weboberfläche und WebAPI; noch kein Code — Technik entscheidet der `architekt` (ADRs unter `Dokumentation/ADRs/`).
- Datenhaltung: täglich rotierende Logdateien je Channel, Frist je Channel (Standard 7 Tage); kein Git, keine Datenbank.
- Leistungsziel: 100.000 Nachrichten — Channel öffnet sofort, Verlauf scrollt flüssig, Suche ohne spürbare Wartezeit.
- Oberfläche deutsch; Designsystem `Dokumentation/Wireframes/designsystem/` (README, Tokens, Bausteine, offene Fragen).
- Gate und Testbefehle: `cargo build`, `cargo test` (Unit, Integration), Playwright gegen das echte Programm (E2E) — sobald der architekt den Aufbau festgelegt hat, hier nachtragen.
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
