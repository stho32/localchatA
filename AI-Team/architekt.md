# architekt — Projektschicht

Ergänzt die Rolle `architekt` der Factory für dieses Projekt. Gilt für Projektgebundenes (Umgebung, Stack, Pfade, Befehle, Stil); Grenzen und Kernregeln der Rolle überschreibt sie nie (`gemeinsam/regeln.md`, „Vorrang“).

## Projektregeln

- Stack laut Vision: Rust, ein Programm für Weboberfläche und WebAPI; noch kein Code — Technik entscheidet der `architekt` (ADRs unter `Dokumentation/ADRs/`).
- Datenhaltung: täglich rotierende Logdateien je Channel, Frist je Channel (Standard 7 Tage); kein Git, keine Datenbank.
- Leistungsziel: 100.000 Nachrichten — Channel öffnet sofort, Verlauf scrollt flüssig, Suche ohne spürbare Wartezeit.
- Oberfläche deutsch; Designsystem `Dokumentation/Wireframes/designsystem/` (README, Tokens, Bausteine, offene Fragen).
- Architektur-Modell unter `Dokumentation/Architektur/` (fehlt noch); Geschwister kanbanD (`~/Projekte/kanbanD`) als Vorbild für Aufbau und Live-Kanal lesen, nicht kopieren.

## Spezialisierungen

Aus der Factory (`rollen/architekt/spezialisierungen/`), gewählt bei der Bestückung; fremde mit `architekt/<name>`.


## Quellen

IDs aus `AI-Team/quellen.md`, die diese Rolle besonders braucht, mit Anlass.

- Q01 — Leitplanken
- Q02 — Umfang
- Q04 — Aufbau, Live-Kanal, Einbettung der Oberfläche als Vorbild

## Gelerntes
