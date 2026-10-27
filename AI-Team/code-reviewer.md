# code-reviewer — Projektschicht

Ergänzt die Rolle `code-reviewer` der Factory für dieses Projekt. Gilt für Projektgebundenes (Umgebung, Stack, Pfade, Befehle, Stil); Grenzen und Kernregeln der Rolle überschreibt sie nie (`gemeinsam/regeln.md`, „Vorrang“).

## Projektregeln

- Stack laut Vision: Rust, ein Programm für Weboberfläche und WebAPI; noch kein Code — Technik entscheidet der `architekt` (ADRs unter `Dokumentation/ADRs/`).
- Datenhaltung: täglich rotierende Logdateien je Channel, Frist je Channel (Standard 7 Tage); kein Git, keine Datenbank.
- Leistungsziel: 100.000 Nachrichten — Channel öffnet sofort, Verlauf scrollt flüssig, Suche ohne spürbare Wartezeit.
- Oberfläche deutsch; Designsystem `Dokumentation/Wireframes/designsystem/` (README, Tokens, Bausteine, offene Fragen).
- Leistungsziel als Prüfpunkt: keine Operation, die je Nachricht den ganzen Verlauf liest.

## Spezialisierungen

Aus der Factory (`rollen/code-reviewer/spezialisierungen/`), gewählt bei der Bestückung; fremde mit `code-reviewer/<name>`.

- entwickler/rust — Maßstab Server
- entwickler/javascript — Maßstab Oberfläche

## Quellen

IDs aus `AI-Team/quellen.md`, die diese Rolle besonders braucht, mit Anlass.

- Q01 — Zielbild
- Q02 — Fertig-Kriterien
- Q04 — Vorbild

## Gelerntes
