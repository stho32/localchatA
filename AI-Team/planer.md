# planer — Projektschicht

Ergänzt die Rolle `planer` der Factory für dieses Projekt. Gilt für Projektgebundenes (Umgebung, Stack, Pfade, Befehle, Stil); Grenzen und Kernregeln der Rolle überschreibt sie nie (`gemeinsam/regeln.md`, „Vorrang“).

## Projektregeln

- Stack laut Vision: Rust, ein Programm für Weboberfläche und WebAPI; noch kein Code — Technik entscheidet der `architekt` (ADRs unter `Dokumentation/ADRs/`).
- Datenhaltung: täglich rotierende Logdateien je Channel, Frist je Channel (Standard 7 Tage); kein Git, keine Datenbank.
- Leistungsziel: 100.000 Nachrichten — Channel öffnet sofort, Verlauf scrollt flüssig, Suche ohne spürbare Wartezeit.
- Oberfläche deutsch; Designsystem `Dokumentation/Wireframes/designsystem/` (README, Tokens, Bausteine, offene Fragen).
- WBS `Dokumentation/Planung/localchata.md`, Phasen-Datei daneben; Interactions sind aus der Vision, noch ohne Anforderung.
- Offene Fragen des Designsystems (`3-offene-fragen.md`) beim Schreiben der passenden Anforderung klären lassen, nicht raten.

## Spezialisierungen

Aus der Factory (`rollen/planer/spezialisierungen/`), gewählt bei der Bestückung; fremde mit `planer/<name>`.


## Quellen

IDs aus `AI-Team/quellen.md`, die diese Rolle besonders braucht, mit Anlass.

- Q01 — Zielbild
- Q02 — WBS
- Q03 — offene Fragen des Designsystems
- Q05 — Briefing

## Gelerntes
